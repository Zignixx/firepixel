//! Farbtabelle der Bildvorschau (re\17-bildvorschau.md §2.5–§2.7): Farbfelder mit Markierungen
//! (ausgewählt, gesperrt, bearbeitet, websicher, transparent), Klick/Mehrfachwahl, Doppelklick =
//! Farbwahl, die sechs Schalterknöpfe darunter, die Statuszeile, die drei Pipetten und das
//! Palettenmenü (Optionsmenü ≡ = Kontextmenü der Tabelle und der Vorschau).
//!
//! Die Tabelle zeigt die Palette des **aktiven** Feldes aus dessen fertigem Ergebnis
//! (`PaneResult::entries`, derselbe Lauf wie Bild und Größe); alle Änderungen schreiben nur die
//! Einstellung dieses Feldes (`fp_export::palette_edit`), das Feld rechnet danach neu.

use super::super::{DialogClose, dialog_frame, push_button};
use crate::menu;
use crate::theme::Theme;
use crate::widgets::{ButtonLook, button_frame};
use egui::{
    Color32, Id, Painter, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, pos2, vec2,
};
use fp_export::options::{OptimizeSettings, Sorting};
use fp_export::palette_edit::{self, PaletteEntry};
use fp_i18n::t;

/// Farbfelder je Zeile (§2.5.1).
pub const COLUMNS: usize = 16;
/// Höhe eines Farbfelds ohne Gitterlinie (Original 17 × 13 px).
pub const CELL_H: f32 = 13.0;
/// Kennung des Dialogs „Farbe“ (Farbwahl beim Doppelklick, „Farbe bearbeiten“, „hinzufügen“).
pub const COLOR_DIALOG_ID: &str = "fp-imgpreview-color";

fn rgb(c: u32) -> u32 {
    c & 0x00FF_FFFF
}

/// Zustand der Farbtabelle im Dialog.
#[derive(Clone, Debug)]
pub struct TableState {
    /// Gewählte Farben (Originalfarbe des Eintrags, wie `PaletteEntry::color`), erste = Statuszeile.
    pub selected: Vec<u32>,
    /// „Farbfeld-Rückmeldung anzeigen“ (Standard an, §2.7).
    pub feedback: bool,
    /// Für wen gilt die Auswahl (Feld, Format, Farbtiefe)? Wechsel = Auswahl aufheben.
    pub owner: Option<(usize, fp_export::ExportFormat, fp_export::ColorMode)>,
}

impl Default for TableState {
    fn default() -> Self {
        TableState {
            selected: Vec::new(),
            feedback: true,
            owner: None,
        }
    }
}

impl TableState {
    pub fn is_selected(&self, color: u32) -> bool {
        self.selected.iter().any(|&c| rgb(c) == rgb(color))
    }

    /// Klick auf ein Feld: ohne Umschalttasten genau diese Farbe, mit Strg/Umschalt dazu bzw. weg.
    pub fn click(&mut self, color: u32, toggle: bool) {
        if toggle {
            if let Some(k) = self.selected.iter().position(|&c| rgb(c) == rgb(color)) {
                self.selected.remove(k);
            } else {
                self.selected.push(color);
            }
        } else {
            self.selected = vec![color];
        }
    }

    /// Auswahl auf vorhandene Einträge beschränken (nach Neuberechnung, Löschen …).
    pub fn retain(&mut self, entries: &[PaletteEntry]) {
        self.selected
            .retain(|&c| entries.iter().any(|e| rgb(e.color) == rgb(c)));
    }

    /// Gewählte Einträge (in Auswahlreihenfolge).
    pub fn selected_entries<'a>(
        &self,
        entries: &'a [PaletteEntry],
    ) -> Vec<(usize, &'a PaletteEntry)> {
        self.selected
            .iter()
            .filter_map(|&c| {
                entries
                    .iter()
                    .enumerate()
                    .find(|(_, e)| rgb(e.color) == rgb(c))
            })
            .collect()
    }
}

// ---------------------------------------------------------------------- Befehle

/// Ein Befehl des Palettenmenüs bzw. der Knöpfe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteCmd {
    AddColor,
    EditColor,
    DeleteColor,
    ReplaceColor,
    SnapWeb,
    Transparent,
    Lock,
    UnlockAll,
    SortLuminance,
    SortPopularity,
    Unsorted,
    SwatchFeedback,
    RemoveEdit,
    RemoveAllEdits,
    LoadPalette,
    SavePalette,
}

/// Was nach einem Befehl noch zu tun ist (Dialog bzw. Datei).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FollowUp {
    None,
    /// Farbwahl öffnen: neue Farbe für die gewählte Farbe (Bearbeiten/Ersetzen) bzw. hinzufügen.
    ColorDialog(ColorMode),
    LoadPalette,
    SavePalette,
}

/// Wofür die Farbwahl geöffnet ist.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorMode {
    Add,
    /// Neue Farbe für den Eintrag mit dieser Originalfarbe.
    Edit(u32),
}

/// Führt einen Befehl auf der Einstellung `s` aus (rein, ohne Oberfläche). `entries` = angezeigte
/// Palette des Feldes. Schaltet Sperre/Transparenz wie die Schalterknöpfe: sind alle gewählten
/// Farben schon gesperrt (transparent), wird aufgehoben, sonst gesetzt.
pub fn apply(
    cmd: PaletteCmd,
    s: &mut OptimizeSettings,
    entries: &[PaletteEntry],
    table: &mut TableState,
) -> FollowUp {
    let sel: Vec<PaletteEntry> = table
        .selected_entries(entries)
        .into_iter()
        .map(|(_, e)| *e)
        .collect();
    match cmd {
        PaletteCmd::AddColor => return FollowUp::ColorDialog(ColorMode::Add),
        PaletteCmd::EditColor | PaletteCmd::ReplaceColor => {
            if let Some(e) = sel.first() {
                return FollowUp::ColorDialog(ColorMode::Edit(e.color));
            }
        }
        PaletteCmd::DeleteColor => {
            if !sel.is_empty() {
                let colors: Vec<u32> = sel.iter().map(|e| e.color).collect();
                palette_edit::delete_colors(s, entries, &colors);
                table.selected.clear();
            }
        }
        PaletteCmd::SnapWeb => {
            // Schalter: alle gewählten schon websicher gemacht → Bearbeitung zurücknehmen
            let undo = !sel.is_empty()
                && sel.iter().all(|e| {
                    e.edited && e.web_safe && rgb(palette_edit::web_snap(e.color)) == rgb(e.shown)
                });
            for e in &sel {
                if undo {
                    palette_edit::remove_edit(s, e.color);
                } else {
                    palette_edit::snap_to_web(s, e.color);
                }
            }
        }
        PaletteCmd::Transparent => {
            let on = !sel.iter().all(|e| e.transparent);
            for e in &sel {
                palette_edit::set_transparent(s, e.color, on);
            }
        }
        PaletteCmd::Lock => {
            let on = !sel.iter().all(|e| e.locked);
            for e in &sel {
                palette_edit::set_locked(s, e.color, on);
            }
        }
        PaletteCmd::UnlockAll => palette_edit::unlock_all(s),
        PaletteCmd::SortLuminance => s.sorting = Sorting::Luminance,
        PaletteCmd::SortPopularity => s.sorting = Sorting::Popularity,
        PaletteCmd::Unsorted => s.sorting = Sorting::None,
        PaletteCmd::SwatchFeedback => table.feedback = !table.feedback,
        PaletteCmd::RemoveEdit => {
            for e in &sel {
                palette_edit::remove_edit(s, e.color);
            }
        }
        PaletteCmd::RemoveAllEdits => palette_edit::remove_all_edits(s),
        PaletteCmd::LoadPalette => return FollowUp::LoadPalette,
        PaletteCmd::SavePalette => return FollowUp::SavePalette,
    }
    FollowUp::None
}

/// Pipetten (§2.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Pipette {
    /// „Farbe für Transparenz wählen“: nur diese Farbe transparent, Liste auf Indextransparenz.
    Set,
    /// „Farbe zu Transparenz hinzufügen“.
    Add,
    /// „Farbe aus Transparenz entfernen“.
    Remove,
}

/// Wendet eine Pipette mit der Palettenfarbe des angeklickten Bildpunkts an.
pub fn apply_pipette(p: Pipette, s: &mut OptimizeSettings, color: u32) {
    let color = color | 0xFF00_0000;
    match p {
        Pipette::Set => palette_edit::set_only_transparent(s, color),
        Pipette::Add => palette_edit::set_transparent(s, color, true),
        Pipette::Remove => palette_edit::set_transparent(s, color, false),
    }
}

// ---------------------------------------------------------------------- Statuszeile

/// Statuszeile einer gewählten Farbe (§2.5.4): `R:051 G:051 B:204 Hex:3333cc Index:000` und
/// angehängt „Gesperrt“, „Modifiziert“, „Websicher“, „Transparent“. Bei Alphapalette mit `A:`.
pub fn status_text(e: &PaletteEntry, index: usize, alpha_palette: bool) -> String {
    let c = e.shown;
    let (r, g, b, a) = ((c >> 16) & 255, (c >> 8) & 255, c & 255, c >> 24);
    let hex = format!("{r:02x}{g:02x}{b:02x}");
    let (rs, gs, bs, is) = (
        format!("{r:03}"),
        format!("{g:03}"),
        format!("{b:03}"),
        format!("{index:03}"),
    );
    let mut s = if alpha_palette {
        fp_i18n::tr!(
            "image-preview-color-status-alpha",
            r = rs,
            g = gs,
            b = bs,
            a = format!("{a:03}"),
            hex = hex,
            index = is
        )
    } else {
        fp_i18n::tr!(
            "image-preview-color-status",
            r = rs,
            g = gs,
            b = bs,
            hex = hex,
            index = is
        )
    };
    for (on, key) in [
        (e.locked, "image-preview-flag-locked"),
        (e.edited, "image-preview-flag-edited"),
        (e.web_safe, "image-preview-flag-websafe"),
        (e.transparent, "image-preview-flag-transparent"),
    ] {
        if on {
            s.push(' ');
            s.push_str(&t(key));
        }
    }
    s
}

// ---------------------------------------------------------------------- Zeichnen

/// Lage der Felder in der Tabelle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grid {
    pub origin: egui::Pos2,
    pub cell: egui::Vec2,
}

impl Grid {
    /// Felder 16 je Zeile; Höhe 13 px, bei wenig Platz kleiner, damit alle Zeilen hineinpassen.
    pub fn new(area: Rect, count: usize) -> Grid {
        let rows = count.div_ceil(COLUMNS).max(1) as f32;
        let w = ((area.width() - 1.0) / COLUMNS as f32).floor().max(4.0);
        let h = ((area.height() - 1.0) / rows).floor().clamp(4.0, CELL_H);
        Grid {
            origin: area.min,
            cell: vec2(w, h),
        }
    }

    /// Rechteck des Feldes `i` (ohne Gitterlinie).
    pub fn cell_rect(&self, i: usize) -> Rect {
        let (col, row) = ((i % COLUMNS) as f32, (i / COLUMNS) as f32);
        Rect::from_min_size(
            self.origin + vec2(1.0 + col * self.cell.x, 1.0 + row * self.cell.y),
            self.cell - vec2(1.0, 1.0),
        )
    }

    /// Feld unter einem Punkt.
    pub fn hit(&self, p: egui::Pos2, count: usize) -> Option<usize> {
        let d = p - self.origin - vec2(1.0, 1.0);
        if d.x < 0.0 || d.y < 0.0 {
            return None;
        }
        let (col, row) = ((d.x / self.cell.x) as usize, (d.y / self.cell.y) as usize);
        let i = row * COLUMNS + col;
        (col < COLUMNS && i < count).then_some(i)
    }
}

fn c32(c: u32) -> Color32 {
    Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

/// Kleines weißes Quadrat mit schwarzem Punkt (gesperrt rechts unten, bearbeitet links unten).
fn corner_mark(p: &Painter, r: Rect, right: bool) {
    let s = (r.height() * 0.42).clamp(3.0, 5.0).round();
    let m = if right {
        Rect::from_min_size(r.right_bottom() - vec2(s, s), vec2(s, s))
    } else {
        Rect::from_min_size(r.left_bottom() - vec2(0.0, s), vec2(s, s))
    };
    p.rect_filled(m, 0.0, Color32::WHITE);
    p.rect_stroke(m, 0.0, Stroke::new(0.5, Color32::BLACK), StrokeKind::Inside);
    p.rect_filled(
        Rect::from_center_size(m.center(), vec2(1.0, 1.0)),
        0.0,
        Color32::BLACK,
    );
}

/// Kleine Raute aus Schwarz-Weiß-Karos in der Mitte (websicher).
fn web_mark(p: &Painter, r: Rect) {
    let c = r.center();
    let k = (r.height() * 0.3).clamp(2.5, 4.0);
    let (t, rr, b, l) = (
        c + vec2(0.0, -k),
        c + vec2(k, 0.0),
        c + vec2(0.0, k),
        c + vec2(-k, 0.0),
    );
    for (i, (a, z)) in [(t, rr), (rr, b), (b, l), (l, t)].into_iter().enumerate() {
        p.add(Shape::convex_polygon(
            vec![c, a, z],
            if i % 2 == 0 {
                Color32::WHITE
            } else {
                Color32::BLACK
            },
            Stroke::NONE,
        ));
    }
}

/// Grau-weißes Karomuster (transparenter Eintrag).
fn checker_cell(p: &Painter, r: Rect) {
    p.rect_filled(r, 0.0, Color32::WHITE);
    let q = 3.0;
    let (nx, ny) = (
        (r.width() / q).ceil() as i32,
        (r.height() / q).ceil() as i32,
    );
    for j in 0..ny {
        for i in 0..nx {
            if (i + j) % 2 == 1 {
                let cr = Rect::from_min_size(
                    pos2(r.left() + i as f32 * q, r.top() + j as f32 * q),
                    vec2(q, q),
                )
                .intersect(r);
                p.rect_filled(cr, 0.0, Color32::from_gray(0xC0));
            }
        }
    }
}

/// Zeichnet ein Farbfeld mit allen Markierungen (§2.5.2); gemeinsamer Baustein für Tabellen mit
/// Palettenansicht (`fp_export::palette_edit::PaletteEntry`).
pub fn paint_cell(p: &Painter, r: Rect, e: &PaletteEntry, selected: bool) {
    if e.transparent || e.shown >> 24 == 0 {
        checker_cell(p, r);
    } else {
        p.rect_filled(r, 0.0, c32(e.shown));
    }
    if e.web_safe {
        web_mark(p, r);
    }
    if e.locked {
        corner_mark(p, r, true);
    }
    if e.edited {
        corner_mark(p, r, false);
    }
    if selected {
        // außen grün, dann weiß, dann schwarz
        let o = r.expand(1.0);
        p.rect_stroke(
            o,
            0.0,
            Stroke::new(2.0, Color32::from_rgb(0x1E, 0xC8, 0x2A)),
            StrokeKind::Inside,
        );
        p.rect_stroke(
            o.shrink(2.0),
            0.0,
            Stroke::new(1.0, Color32::WHITE),
            StrokeKind::Inside,
        );
        p.rect_stroke(
            o.shrink(3.0),
            0.0,
            Stroke::new(1.0, Color32::BLACK),
            StrokeKind::Inside,
        );
    }
}

/// Was in der Tabelle geschehen ist.
#[derive(Default)]
pub struct TableAction {
    /// Doppelklick: Farbwahl für diese Farbe.
    pub edit: Option<u32>,
    /// Antwort der Tabelle (für das Kontextmenü).
    pub response: Option<Response>,
}

/// Zeichnet die Tabelle in `area` und verarbeitet Klick, Strg/Umschalt+Klick, Doppelklick und den
/// Klick ins Leere (Auswahl aufheben, die Pipetten kehren zurück).
pub fn table(
    ui: &mut Ui,
    area: Rect,
    entries: &[PaletteEntry],
    table: &mut TableState,
) -> TableAction {
    let theme = Theme::current(ui.ctx());
    let p = ui.painter().with_clip_rect(area.intersect(ui.clip_rect()));
    p.rect_filled(area, 0.0, theme.colors.list_bg);
    ui.painter().rect_stroke(
        area,
        0.0,
        Stroke::new(1.0, theme.colors.list_border),
        StrokeKind::Inside,
    );
    let inner = area.shrink(1.0);
    let grid = Grid::new(inner, entries.len());
    let resp = ui.interact(inner, Id::new("fp-ipv-color-table"), Sense::CLICK);
    resp.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Other,
            true,
            t("image-preview-color-table"),
        )
    });
    // Gitter: Hintergrund der belegten Felder schwarz, Felder darüber
    let n = entries.len();
    for i in 0..n {
        let r = grid.cell_rect(i);
        p.rect_filled(r.expand2(vec2(1.0, 1.0)), 0.0, Color32::BLACK);
    }
    for (i, e) in entries.iter().enumerate() {
        paint_cell(&p, grid.cell_rect(i), e, table.is_selected(e.color));
    }
    let mut act = TableAction::default();
    let hit = resp.interact_pointer_pos().and_then(|pp| grid.hit(pp, n));
    // egui zählt einen Doppelklick kurz nach einem Klick an anderer Stelle als Dreifachklick
    if resp.double_clicked() || resp.triple_clicked() {
        if let Some(i) = hit {
            table.selected = vec![entries[i].color];
            act.edit = Some(entries[i].color);
        }
    } else if resp.clicked() || resp.secondary_clicked() {
        // Umschalttasten aus dem Zustand oder aus dem Klick selbst (Taste und Klick im selben Bild)
        let m = ui.input(|i| {
            let ev = i.events.iter().find_map(|e| match e {
                egui::Event::PointerButton { modifiers, .. } if modifiers.any() => Some(*modifiers),
                _ => None,
            });
            ev.unwrap_or(i.modifiers)
        });
        match hit {
            Some(i) => {
                let c = entries[i].color;
                if resp.clicked() {
                    table.click(c, m.command || m.shift);
                } else if !table.is_selected(c) {
                    table.selected = vec![c];
                }
            }
            None if resp.clicked() => table.selected.clear(),
            None => {}
        }
    }
    act.response = Some(resp);
    act
}

/// Schalterknopf (Kontrollkästchen in Knopfoptik, §2.5.3) mit Tooltip und Barrierefreiheits-Namen;
/// `paint` zeichnet das Symbol.
pub fn toggle_button(
    ui: &mut Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    r: Rect,
    pressed: bool,
    enabled: bool,
    tip: &str,
    paint: impl FnOnce(&Painter, Rect, bool),
) -> Response {
    let theme = Theme::current(ui.ctx());
    let resp = ui.interact(
        r,
        Id::new(("fp-ipv-toggle", id)),
        if enabled {
            Sense::CLICK
        } else {
            Sense::hover()
        },
    );
    resp.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, enabled, pressed, tip)
    });
    button_frame(
        ui.painter(),
        r,
        &resp,
        ButtonLook {
            selected: pressed,
            enabled,
            group: None,
        },
        &theme,
    );
    if !pressed && enabled && !resp.hovered() {
        ui.painter().rect_stroke(
            r,
            2.0,
            Stroke::new(
                1.0,
                crate::theme::with_alpha(theme.colors.button_border, 110),
            ),
            StrokeKind::Inside,
        );
    }
    paint(
        ui.painter(),
        Rect::from_center_size(r.center(), vec2(16.0, 16.0)),
        enabled,
    );
    if tip.is_empty() {
        resp
    } else {
        resp.on_hover_text(tip)
    }
}

/// Symbol aus dem Satz der Oberfläche.
pub fn paint_icon(theme: &Theme, p: &Painter, r: Rect, enabled: bool, icon: crate::icons::Icon) {
    crate::icons::paint(
        p,
        r,
        icon,
        &crate::icons::IconStyle::new(theme, None, enabled),
    );
}

/// Pipettensymbol mit „+“ bzw. „−“.
pub fn paint_pipette(theme: &Theme, p: &Painter, r: Rect, enabled: bool, kind: Pipette) {
    crate::icons::paint(
        p,
        r,
        crate::icons::Icon::Eyedropper,
        &crate::icons::IconStyle::new(theme, None, enabled),
    );
    let col = if enabled {
        theme.colors.icon_fg
    } else {
        theme.colors.icon_disabled
    };
    let c = r.right_top() + vec2(-2.0, 3.0);
    match kind {
        Pipette::Set => {}
        Pipette::Add => {
            p.hline((c.x - 2.5)..=(c.x + 2.5), c.y, Stroke::new(1.2, col));
            p.vline(c.x, (c.y - 2.5)..=(c.y + 2.5), Stroke::new(1.2, col));
        }
        Pipette::Remove => {
            p.hline((c.x - 2.5)..=(c.x + 2.5), c.y, Stroke::new(1.2, col));
        }
    }
}

/// Symbole der sechs Knöpfe: Farbe bearbeiten, Transparent, Websicher, Sperren, Hinzufügen, Löschen.
pub fn paint_button_glyph(theme: &Theme, p: &Painter, r: Rect, enabled: bool, k: usize) {
    use crate::icons::Icon;
    let st = crate::icons::IconStyle::new(theme, None, enabled);
    let fg = if enabled {
        theme.colors.icon_fg
    } else {
        theme.colors.icon_disabled
    };
    match k {
        0 => crate::icons::paint(p, r, Icon::Pencil, &st),
        1 => {
            let b = Rect::from_center_size(r.center(), vec2(11.0, 11.0));
            if enabled {
                checker_cell(p, b);
            }
            p.rect_stroke(b, 0.0, Stroke::new(1.0, fg), StrokeKind::Inside);
        }
        2 => {
            let b = Rect::from_center_size(r.center(), vec2(11.0, 11.0));
            p.rect_stroke(b, 0.0, Stroke::new(1.0, fg), StrokeKind::Inside);
            if enabled {
                web_mark(p, b.expand(2.0));
            }
        }
        3 => crate::icons::paint(p, r, Icon::Lock, &st),
        4 => crate::icons::paint(p, r, Icon::Plus, &st),
        _ => crate::icons::paint(p, r, Icon::Trash, &st),
    }
}

/// Menüeintrag mit Barrierefreiheits-Namen.
fn item(ui: &mut Ui, label: &str, checked: Option<bool>, enabled: bool) -> bool {
    let resp = menu::paint_item_pub(ui, label, None, checked, enabled, false, false);
    resp.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::Button,
            enabled,
            checked == Some(true),
            label,
        )
    });
    let clicked = enabled && resp.clicked();
    if clicked {
        ui.close();
    }
    clicked
}

/// Zustand für das Palettenmenü.
#[derive(Clone, Copy, Debug)]
pub struct MenuState {
    pub has_selection: bool,
    pub any_locked: bool,
    pub selection_edited: bool,
    pub any_edited: bool,
    pub sorting: Sorting,
    pub feedback: bool,
}

/// Inhalt des Palettenmenüs (Optionsmenü ≡ und Kontextmenü, §2.7).
pub fn menu_body(ui: &mut Ui, m: MenuState) -> Option<PaletteCmd> {
    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
    let mut out = None;
    let mut it = |ui: &mut Ui, key: &str, cmd: PaletteCmd, checked: Option<bool>, enabled: bool| {
        if item(ui, &t(key), checked, enabled) {
            out = Some(cmd);
        }
    };
    let sel = m.has_selection;
    it(
        ui,
        "image-preview-menu-add-color",
        PaletteCmd::AddColor,
        None,
        true,
    );
    it(
        ui,
        "image-preview-menu-edit-color",
        PaletteCmd::EditColor,
        None,
        sel,
    );
    it(
        ui,
        "image-preview-menu-delete-color",
        PaletteCmd::DeleteColor,
        None,
        sel,
    );
    it(
        ui,
        "image-preview-menu-replace-color",
        PaletteCmd::ReplaceColor,
        None,
        sel,
    );
    menu::paint_separator_pub(ui);
    it(
        ui,
        "image-preview-menu-snap-web",
        PaletteCmd::SnapWeb,
        None,
        sel,
    );
    menu::paint_separator_pub(ui);
    it(
        ui,
        "image-preview-menu-transparent",
        PaletteCmd::Transparent,
        None,
        sel,
    );
    it(ui, "image-preview-menu-lock", PaletteCmd::Lock, None, sel);
    it(
        ui,
        "image-preview-menu-unlock-all",
        PaletteCmd::UnlockAll,
        None,
        m.any_locked,
    );
    menu::paint_separator_pub(ui);
    it(
        ui,
        "image-preview-menu-sort-luminance",
        PaletteCmd::SortLuminance,
        Some(m.sorting == Sorting::Luminance),
        true,
    );
    it(
        ui,
        "image-preview-menu-sort-popularity",
        PaletteCmd::SortPopularity,
        Some(m.sorting == Sorting::Popularity),
        true,
    );
    it(
        ui,
        "image-preview-menu-unsorted",
        PaletteCmd::Unsorted,
        Some(m.sorting == Sorting::None),
        true,
    );
    menu::paint_separator_pub(ui);
    it(
        ui,
        "image-preview-menu-swatch-feedback",
        PaletteCmd::SwatchFeedback,
        Some(m.feedback),
        true,
    );
    menu::paint_separator_pub(ui);
    it(
        ui,
        "image-preview-menu-remove-edit",
        PaletteCmd::RemoveEdit,
        None,
        m.selection_edited,
    );
    it(
        ui,
        "image-preview-menu-remove-all-edits",
        PaletteCmd::RemoveAllEdits,
        None,
        m.any_edited,
    );
    menu::paint_separator_pub(ui);
    it(
        ui,
        "image-preview-menu-load-palette",
        PaletteCmd::LoadPalette,
        None,
        true,
    );
    it(
        ui,
        "image-preview-menu-save-palette",
        PaletteCmd::SavePalette,
        None,
        true,
    );
    out
}

/// Palettenmenü als Popup an `resp` (Knopf ≡).
pub fn popup_menu(ui: &Ui, resp: &Response, m: MenuState) -> Option<PaletteCmd> {
    let theme = Theme::current(ui.ctx());
    let mut out = None;
    egui::Popup::menu(resp)
        .frame(menu::menu_frame(&theme))
        .show(|ui| out = menu_body(ui, m));
    out
}

/// Palettenmenü als Kontextmenü (Rechtsklick auf Tabelle oder Vorschau).
pub fn context_menu(resp: &Response, m: MenuState) -> Option<PaletteCmd> {
    let mut out = None;
    resp.context_menu(|ui| out = menu_body(ui, m));
    out
}

// ---------------------------------------------------------------------- Farbwahl

/// Offener Dialog „Farbe“ (Systemfarbwahl beim Doppelklick, §2.5.4).
#[derive(Clone, Debug)]
pub struct ColorDialog {
    pub pane: usize,
    pub mode: ColorMode,
    pub color: Color32,
    pub hex: String,
}

impl ColorDialog {
    pub fn new(pane: usize, mode: ColorMode, start: u32) -> ColorDialog {
        let color = c32(start);
        ColorDialog {
            pane,
            mode,
            color,
            hex: hex(color),
        }
    }
}

fn hex(c: Color32) -> String {
    format!("{:02x}{:02x}{:02x}", c.r(), c.g(), c.b())
}

/// Ergebnis des Dialogs „Farbe“.
pub enum ColorOutcome {
    Open(ColorDialog),
    Ok(ColorDialog),
    Cancel,
}

/// Dialog „Farbe“: Farbfläche mit Farbton, Hex-Feld, OK/Abbrechen.
pub fn color_dialog(ctx: &egui::Context, mut d: ColorDialog) -> ColorOutcome {
    let title = t(if d.mode == ColorMode::Add {
        "image-preview-color-add-title"
    } else {
        "image-preview-color-title"
    });
    let (hex_l, ok_l, cancel_l) = (t("image-preview-color-hex"), t("btn-ok"), t("btn-cancel"));
    let dx = super::super::extra_w(ctx, &[&hex_l], 40.0);
    let mut result = None;
    let r = dialog_frame(
        ctx,
        COLOR_DIALOG_ID,
        &title,
        vec2(300.0 + dx, 330.0),
        |ui, body| {
            let pick = Rect::from_min_size(
                body.min + vec2(12.0, 10.0),
                vec2(body.width() - 24.0, 236.0),
            );
            ui.scope_builder(egui::UiBuilder::new().max_rect(pick), |ui| {
                ui.spacing_mut().slider_width = (pick.width() - 60.0).max(120.0);
                if egui::color_picker::color_picker_color32(
                    ui,
                    &mut d.color,
                    egui::color_picker::Alpha::Opaque,
                ) {
                    d.hex = hex(d.color);
                }
            });
            let y = body.top() + 254.0;
            super::super::fit_label(
                ui,
                Rect::from_min_size(pos2(body.left() + 12.0, y), vec2(40.0 + dx, 20.0)),
                &hex_l,
                true,
            );
            let sw = Rect::from_min_size(pos2(body.right() - 12.0 - 40.0, y), vec2(40.0, 20.0));
            ui.painter().rect_filled(sw, 0.0, d.color);
            ui.painter().rect_stroke(
                sw,
                0.0,
                Stroke::new(1.0, Theme::current(ui.ctx()).colors.field_border),
                StrokeKind::Inside,
            );
            if let Some(s) = crate::inspector::fields::text(
                ui,
                "fp-ipv-color-hex",
                Rect::from_min_size(pos2(body.left() + 56.0 + dx, y), vec2(80.0, 20.0)),
                &d.hex,
                true,
            ) {
                let s = s.trim().trim_start_matches('#').to_string();
                if s.len() == 6
                    && let Ok(v) = u32::from_str_radix(&s, 16)
                {
                    d.color = c32(v);
                }
                d.hex = hex(d.color);
            }
            let b = super::super::button_row(
                ui.ctx(),
                body.right() - 12.0,
                body.bottom() - 32.0,
                &[&ok_l, &cancel_l],
                76.0,
                23.0,
                8.0,
            );
            if push_button(ui, b[0], &ok_l, true, true) {
                result = Some(true);
            }
            if push_button(ui, b[1], &cancel_l, false, true) {
                result = Some(false);
            }
        },
    );
    match (result, r) {
        (Some(true), _) => ColorOutcome::Ok(d),
        (Some(false), _) | (_, DialogClose::Cancelled) => ColorOutcome::Cancel,
        _ => ColorOutcome::Open(d),
    }
}

/// Farbe des Dialogs als `0xFFRRGGBB`.
pub fn dialog_color(d: &ColorDialog) -> u32 {
    0xFF00_0000 | (d.color.r() as u32) << 16 | (d.color.g() as u32) << 8 | d.color.b() as u32
}

/// Übernimmt das Ergebnis des Dialogs „Farbe“ in die Einstellung: hinzufügen bzw. bearbeiten
/// (danach „Modifiziert“; eine Sperre bleibt).
pub fn apply_color_dialog(d: &ColorDialog, s: &mut OptimizeSettings) {
    let c = dialog_color(d);
    match d.mode {
        ColorMode::Add => palette_edit::add_color(s, c),
        ColorMode::Edit(orig) => palette_edit::edit_color(s, orig | 0xFF00_0000, Some(c)),
    }
}

// ---------------------------------------------------------------------- Farbfeld-Rückmeldung

/// Überlagerung für die Vorschau: Pixel der gewählten Einträge hervorgehoben, sonst durchsichtig.
pub fn feedback_image(
    indices: &[u8],
    w: usize,
    h: usize,
    selected: &[usize],
    col: Color32,
) -> Option<egui::ColorImage> {
    if indices.len() != w * h || w == 0 || h == 0 || selected.is_empty() {
        return None;
    }
    let mut mark = [false; 256];
    for &i in selected {
        if i < 256 {
            mark[i] = true;
        }
    }
    let px: Vec<Color32> = indices
        .iter()
        .map(|&i| {
            if mark[i as usize] {
                col
            } else {
                Color32::TRANSPARENT
            }
        })
        .collect();
    Some(egui::ColorImage::new([w, h], px))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(color: u32) -> PaletteEntry {
        PaletteEntry {
            color,
            shown: color,
            count: 1,
            locked: false,
            transparent: false,
            edited: false,
            web_safe: palette_edit::is_web_safe(color),
        }
    }

    #[test]
    fn grid_hits_and_selection() {
        let g = Grid::new(Rect::from_min_size(pos2(0.0, 0.0), vec2(257.0, 209.0)), 256);
        assert_eq!(g.cell, vec2(16.0, 13.0));
        assert_eq!(g.hit(g.cell_rect(17).center(), 256), Some(17));
        assert_eq!(g.hit(pos2(5.0, 300.0), 256), None);
        let mut t = TableState::default();
        t.click(0xFF11_2233, false);
        t.click(0xFF44_5566, true);
        assert_eq!(t.selected.len(), 2);
        t.click(0xFF11_2233, true);
        assert_eq!(t.selected, vec![0xFF44_5566]);
        t.retain(&[e(0xFF11_2233)]);
        assert!(t.selected.is_empty());
    }
}
