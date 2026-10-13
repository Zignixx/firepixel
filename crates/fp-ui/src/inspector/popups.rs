//! Popups des Inspektors (11 §2.3.3 „Strich-, Füllungs- und Effekt-Popups“):
//! - Farbfeld mit dem Farbwähler-Popup des Originals (Umsetzung in [`super::color_picker`], N27b),
//! - Verlaufs-Editor (5980): Form, Umkehren, Farb- und Deckkraftstopps,
//! - Strichkategorien (4116) mit Vorgaben je Kategorie,
//! - Pipette über die Leinwand (Werkzeug Pipette vorübergehend, danach Rückkehr).

use super::fields::{self, Dl};
use super::color_picker::Footer;
use super::texts::{self, refs};
use crate::app_state::{Action, AppState};
use crate::theme::Theme;
use crate::ui_commands::{self, GRADIENT_SHAPES};
use egui::{pos2, vec2, Align2, Color32, Id, Rect, Response, Sense, Stroke, StrokeKind, Ui};
use fp_color::{Argb, Rgba8};
use fp_engine::ToolId;
use serde_json::{json, Value};

/// Ergebnis des Farbwähler-Popups ([`super::color_picker`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColorChoice {
    Color(Argb),
    /// Keine Farbe (transparent, Knopf „Ohne“).
    None,
    /// Pipette starten (älterer Ablauf; das Popup des Originals ist selbst schon Pipette und liefert
    /// die aufgenommene Farbe direkt als [`ColorChoice::Color`]).
    Pipette,
    /// Werkzeugleiste Füllung: Knopf „Verlauf“.
    Gradient,
    /// Werkzeugleiste Füllung: Knopf „Muster“.
    Pattern,
    /// Kontur: Strichlage 0 innen, 1 mittig, 2 außen.
    Placement(u8),
}

fn c32(c: Argb) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), c.a())
}

/// Id-Basis des Farbwählers eines Farbfelds (für Tests und Aufräumen; Farbdialog: `.with("sys")`,
/// `.with("sysb")`, `.with("syscol")`).
pub fn picker_id(id: Id) -> Id {
    id.with("picker")
}

/// Farbfeld mit dem Farbwähler-Popup des Originals. `allow_none`: Fußleiste mit „Ohne“; die Pipette
/// ist immer aktiv (`allow_pipette` bleibt nur für ältere Aufrufer).
#[allow(clippy::too_many_arguments)]
pub fn color_button(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, r: Rect, st: &AppState, color: Option<Argb>, enabled: bool, allow_none: bool, allow_pipette: bool) -> Option<ColorChoice> {
    let _ = allow_pipette;
    color_button_with(ui, id, r, st, color, enabled, if allow_none { Footer::NoneOnly } else { Footer::Plain })
}

/// Farbfeld mit dem Farbwähler-Popup und einer bestimmten Fußleiste.
pub fn color_button_with(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, r: Rect, st: &AppState, color: Option<Argb>, enabled: bool, footer: Footer) -> Option<ColorChoice> {
    let id = Id::new(id);
    let resp = fields::swatch(ui, id, r, color, enabled);
    super::color_picker::popup(ui, &resp, st, color, footer)
}

/// Farbwähler-Popup an einer Antwort (öffnet beim Klick darauf).
pub fn color_popup(ui: &mut Ui, resp: &Response, id: Id, st: &AppState, color: Option<Argb>, allow_none: bool, allow_pipette: bool) -> Option<ColorChoice> {
    let _ = (id, allow_pipette);
    super::color_picker::popup(ui, resp, st, color, if allow_none { Footer::NoneOnly } else { Footer::Plain })
}
// ---------------------------------------------------------------------- Pipette über die Leinwand

#[derive(Clone, Debug)]
struct PickState {
    command: String,
    params: Value,
    key: String,
    prev_tool: ToolId,
    fill: Argb,
    stroke: Argb,
}

const PICK_ID: &str = "pi-pick";

/// Platzhalterfarbe während der Pipette: völlig transparent und daher nie das Ergebnis einer
/// Aufnahme (die Pipette liefert deckende Farben). So wird auch eine Aufnahme erkannt, die
/// dieselbe Farbe wie vorher ergibt.
const PICK_SENTINEL: Argb = Argb(0x0012_3457);

/// Startet die Pipette: das nächste Aufnehmen auf der Leinwand setzt `params[key]` und führt
/// `command` aus (z. B. `pi.fill` mit `color`).
pub fn start_pick(ctx: &egui::Context, st: &mut AppState, command: &str, params: Value, key: &str) {
    let ps = PickState { command: command.into(), params, key: key.into(), prev_tool: st.session.tools.active, fill: st.session.tools.fill_color, stroke: st.session.tools.stroke_color };
    ctx.data_mut(|d| d.insert_temp(Id::new(PICK_ID), ps));
    st.push(Action::SetColors { stroke: Some(PICK_SENTINEL), fill: Some(PICK_SENTINEL) });
    st.push(Action::SetTool(ToolId::Eyedropper));
}

/// Ist die Pipette aktiv?
pub fn picking(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<PickState>(Id::new(PICK_ID))).is_some()
}

/// Jeden Frame aufrufen: übernimmt die aufgenommene Farbe und stellt Werkzeug und Farben wieder her.
pub fn poll_pick(ctx: &egui::Context, st: &mut AppState) {
    let Some(ps) = ctx.data(|d| d.get_temp::<PickState>(Id::new(PICK_ID))) else { return };
    let tools = &st.session.tools;
    if tools.active != ToolId::Eyedropper {
        // erst nach dem Werkzeugwechsel prüfen; anderes Werkzeug gewählt = abbrechen
        if tools.active != ps.prev_tool {
            ctx.data_mut(|d| d.remove::<PickState>(Id::new(PICK_ID)));
            st.push(Action::SetColors { stroke: Some(ps.stroke), fill: Some(ps.fill) });
        }
        return;
    }
    let picked = if tools.fill_color != PICK_SENTINEL {
        Some(tools.fill_color)
    } else if tools.stroke_color != PICK_SENTINEL {
        Some(tools.stroke_color)
    } else {
        None
    };
    if let Some(c) = picked {
        ctx.data_mut(|d| d.remove::<PickState>(Id::new(PICK_ID)));
        st.push(Action::SetColors { stroke: Some(ps.stroke), fill: Some(ps.fill) });
        let mut p = ps.params.clone();
        if let Value::Object(m) = &mut p {
            m.insert(ps.key.clone(), json!(ui_commands::hex(Argb(c.0 | 0xff00_0000))));
        }
        st.push(Action::Run(ps.command.clone(), p));
        st.push(Action::SetTool(ps.prev_tool));
    }
}

// ---------------------------------------------------------------------- Verlaufs-Editor

#[derive(Clone, Debug, PartialEq)]
struct GradEdit {
    colors: Vec<(f32, Argb)>,
    opacities: Vec<(f32, Argb)>,
    sel: Option<(bool, usize)>,
    drag: Option<(bool, usize)>,
}

fn lerp(a: Argb, b: Argb, t: f32) -> Argb {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round().clamp(0.0, 255.0) as u32;
    Argb(m(a.a(), b.a()) << 24 | m(a.r(), b.r()) << 16 | m(a.g(), b.g()) << 8 | m(a.b(), b.b()))
}

/// Farbe eines Stoppverlaufs an Position `p`.
pub fn sample_stops(stops: &[(f32, Argb)], p: f32) -> Argb {
    match stops {
        [] => Argb(0xff00_0000),
        [one] => one.1,
        _ => {
            if p <= stops[0].0 {
                return stops[0].1;
            }
            for w in stops.windows(2) {
                if p <= w[1].0 {
                    let span = (w[1].0 - w[0].0).max(1e-6);
                    return lerp(w[0].1, w[1].1, (p - w[0].0) / span);
                }
            }
            stops[stops.len() - 1].1
        }
    }
}

/// Verlaufs-Editor (Popup 5980). Liefert Parameter für `pi.fill` bzw. das Werkzeug.
pub fn gradient_editor(ui: &mut Ui, id: Id, fill: &fp_model::Fill) -> Option<Value> {
    let t = Theme::current(ui.ctx());
    let c = &t.colors;
    let g = fill.gradient.as_ref();
    let base_c: Vec<(f32, Argb)> = g.map(|g| g.color_stops().iter().map(|s| (s.position, s.color)).collect()).unwrap_or_default();
    let base_o: Vec<(f32, Argb)> = g.map(|g| g.opacity_stops().iter().map(|s| (s.position, s.color)).collect()).unwrap_or_default();
    let key = id.with("edit");
    let mut ge: GradEdit = ui.data(|d| d.get_temp(key)).unwrap_or(GradEdit { colors: base_c.clone(), opacities: base_o.clone(), sel: None, drag: None });
    if ge.drag.is_none() && (ge.colors != base_c || ge.opacities != base_o) && ge.sel.is_none() {
        ge.colors = base_c.clone();
        ge.opacities = base_o.clone();
    }
    let mut out = None;
    // Kopf: Form + Umkehren
    let (head, _) = ui.allocate_exact_size(vec2(240.0, 24.0), Sense::hover());
    let names = texts::gradient_labels();
    let cur = GRADIENT_SHAPES.iter().position(|g| g.1 == fill.shape);
    if let Some(i) = fields::combo(ui, id.with("shape"), Rect::from_min_size(head.min + vec2(0.0, 2.0), vec2(150.0, 20.0)), &refs(&names), cur, true) {
        out = Some(json!({"shape": GRADIENT_SHAPES[i].1}));
    }
    if fields::button(ui, id.with("rev"), Rect::from_min_size(pos2(head.left() + 156.0, head.top() + 2.0), vec2(84.0, 20.0)), &texts::t("pi-grad-reverse"), true) {
        out = Some(json!({"reverse": true}));
    }
    ui.add_space(4.0);
    // Deckkraftstopps (oben), Vorschau, Farbstopps (unten)
    let (area, resp) = ui.allocate_exact_size(vec2(240.0, 56.0), Sense::click_and_drag());
    let bar = Rect::from_min_max(pos2(area.left() + 6.0, area.top() + 16.0), pos2(area.right() - 6.0, area.top() + 38.0));
    crate::widgets::checker(ui.painter(), bar, 5.0, &t);
    let n = 48;
    for i in 0..n {
        let p0 = i as f32 / n as f32;
        let col = sample_stops(&ge.colors, p0 + 0.5 / n as f32);
        let a = if ge.opacities.is_empty() { col.a() } else { sample_stops(&ge.opacities, p0 + 0.5 / n as f32).a() };
        let r = Rect::from_min_max(pos2(bar.left() + p0 * bar.width(), bar.top()), pos2(bar.left() + (i + 1) as f32 / n as f32 * bar.width() + 0.5, bar.bottom()));
        ui.painter().rect_filled(r, 0.0, Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), a));
    }
    ui.painter().rect_stroke(bar, 0.0, Stroke::new(1.0, c.field_border), StrokeKind::Outside);
    let stop_rect = |top: bool, p: f32| {
        let x = bar.left() + p.clamp(0.0, 1.0) * bar.width();
        if top { Rect::from_center_size(pos2(x, area.top() + 7.0), vec2(9.0, 11.0)) } else { Rect::from_center_size(pos2(x, area.bottom() - 7.0), vec2(9.0, 11.0)) }
    };
    for (top, list) in [(true, &ge.opacities), (false, &ge.colors)] {
        for (i, (p, col)) in list.iter().enumerate() {
            let r = stop_rect(top, *p);
            let fillc = if top { let a = col.a(); Color32::from_gray(255 - a) } else { Color32::from_rgb(col.r(), col.g(), col.b()) };
            ui.painter().rect_filled(r, 1.0, fillc);
            let selected = ge.sel == Some((top, i));
            ui.painter().rect_stroke(r, 1.0, Stroke::new(if selected { 2.0 } else { 1.0 }, if selected { c.focus } else { c.icon_fg }), StrokeKind::Inside);
        }
    }
    if (resp.drag_started() || resp.clicked())
        && let Some(pp) = resp.interact_pointer_pos()
    {
        {
            let hit = |top: bool, list: &[(f32, Argb)]| list.iter().position(|(p, _)| stop_rect(top, *p).expand(2.0).contains(pp)).map(|i| (top, i));
            let found = hit(true, &ge.opacities).or_else(|| hit(false, &ge.colors));
            match found {
                Some(s) => {
                    ge.sel = Some(s);
                    if resp.drag_started() {
                        ge.drag = Some(s);
                    }
                }
                None if resp.clicked() => {
                    let pos = ((pp.x - bar.left()) / bar.width()).clamp(0.0, 1.0);
                    let top = pp.y < bar.top();
                    if top {
                        let a = sample_stops(&ge.opacities, pos);
                        ge.opacities.push((pos, a));
                        ge.opacities.sort_by(|a, b| a.0.total_cmp(&b.0));
                    } else {
                        let col = sample_stops(&ge.colors, pos);
                        ge.colors.push((pos, col));
                        ge.colors.sort_by(|a, b| a.0.total_cmp(&b.0));
                    }
                    out = Some(stops_params(&ge));
                    ge.sel = None;
                }
                None => {}
            }
        }
    }
    if let (Some((top, i)), Some(pp)) = (ge.drag, resp.interact_pointer_pos())
        && resp.dragged()
    {
        let pos = ((pp.x - bar.left()) / bar.width()).clamp(0.0, 1.0);
        let list = if top { &mut ge.opacities } else { &mut ge.colors };
        if let Some(s) = list.get_mut(i) {
            s.0 = pos;
        }
    }
    if resp.drag_stopped() && ge.drag.is_some() {
        ge.drag = None;
        ge.colors.sort_by(|a, b| a.0.total_cmp(&b.0));
        ge.opacities.sort_by(|a, b| a.0.total_cmp(&b.0));
        out = Some(stops_params(&ge));
        ge.sel = None;
    }
    // Bearbeiten des gewählten Stopps
    if let Some((top, i)) = ge.sel {
        let (row, _) = ui.allocate_exact_size(vec2(240.0, 24.0), Sense::hover());
        let d = Dl::at(row.min);
        if top {
            let a = ge.opacities.get(i).map(|s| s.1.a() as f64 * 100.0 / 255.0).unwrap_or(100.0);
            fields::label(ui, d.r(0.0, 2.0, 40.0, 10.0), &texts::t("pi-opacity"), true);
            if let Some(v) = fields::number(ui, id.with("op"), d.r(42.0, 0.0, 24.0, 13.0), Some(d.r(66.0, 0.0, 9.0, 13.0)), Some(a), (0.0, 100.0), 0, true)
                && let Some(s) = ge.opacities.get_mut(i)
            {
                let a8 = (v * 2.55).round().clamp(0.0, 255.0) as u32;
                s.1 = Argb(a8 << 24 | 0x00ff_ffff);
                out = Some(stops_params(&ge));
            }
        } else {
            let cur = ge.colors.get(i).map(|s| s.1);
            let hex = cur.map(|x| format!("#{:02X}{:02X}{:02X}", x.r(), x.g(), x.b())).unwrap_or_default();
            fields::label(ui, d.r(0.0, 2.0, 30.0, 10.0), &texts::t("pi-grad-color"), true);
            if let Some(s) = fields::text(ui, id.with("hex"), d.r(32.0, 0.0, 44.0, 13.0), &hex, true) {
                let s = if s.starts_with('#') { s } else { format!("#{s}") };
                if let (Some(col), Some(st)) = (Rgba8::parse_hex(&s), ge.colors.get_mut(i)) {
                    st.1 = Argb(col.to_argb().0 | 0xff00_0000);
                    out = Some(stops_params(&ge));
                }
            }
        }
        let list_len = if top { ge.opacities.len() } else { ge.colors.len() };
        if list_len > 2 && fields::button(ui, id.with("del"), d.r(110.0, 0.0, 50.0, 13.0), &texts::t("pi-grad-delete"), true) {
            if top {
                ge.opacities.remove(i);
            } else {
                ge.colors.remove(i);
            }
            ge.sel = None;
            out = Some(stops_params(&ge));
        }
        if !top {
            // Farbfelder für den gewählten Stopp
            let sw = [0xffff_ffffu32, 0xff00_0000, 0xffff_0000, 0xffff_9900, 0xffff_ff00, 0xff00_cc00, 0xff00_ccff, 0xff00_33ff, 0xff99_00ff, 0xffff_00cc, 0xff99_9999, 0xff66_3300];
            let (row2, r2) = ui.allocate_exact_size(vec2(240.0, 14.0), Sense::click());
            for (k, col) in sw.iter().enumerate() {
                let rr = Rect::from_min_size(pos2(row2.left() + k as f32 * 14.0, row2.top()), vec2(13.0, 13.0));
                ui.painter().rect_filled(rr, 0.0, c32(Argb(*col)));
                ui.painter().rect_stroke(rr, 0.0, Stroke::new(1.0, c.field_border), StrokeKind::Inside);
                if r2.clicked()
                    && r2.interact_pointer_pos().is_some_and(|p| rr.contains(p))
                    && let Some(st) = ge.colors.get_mut(i)
                {
                    st.1 = Argb(*col);
                    out = Some(stops_params(&ge));
                }
            }
        }
    }
    ui.painter().text(pos2(area.left(), area.bottom() + 2.0), Align2::LEFT_TOP, "", t.small_font(), c.text_weak);
    ui.data_mut(|d| d.insert_temp(key, ge));
    out
}

fn stops_params(ge: &GradEdit) -> Value {
    let cs: Vec<Value> = ge.colors.iter().map(|(p, c)| json!([p, ui_commands::hex(*c)])).collect();
    let os: Vec<Value> = ge.opacities.iter().map(|(p, c)| json!([p, ui_commands::hex(*c)])).collect();
    if os.len() >= 2 { json!({"stops": cs, "opacityStops": os}) } else { json!({"stops": cs}) }
}

// ---------------------------------------------------------------------- Strichkategorien

/// Strichkategorie-Knopf (14606) mit Menü: „Keine“, Kategorien mit Vorgaben.
/// Liefert `Some(None)` für „Keine“ bzw. `Some(Some(index))` in [`ui_commands::BRUSH_PRESETS`].
/// Gespeicherte Pinselstriche des Dokuments stehen unter „Gespeichert“ mit Indizes ab
/// `BRUSH_PRESETS.len()`. `label` ist der angezeigte Name, `has` = es gibt einen Strich,
/// `preset` = Index der aktuellen Vorgabe.
#[allow(clippy::too_many_arguments)]
pub fn stroke_category(ui: &mut Ui, id: Id, r: Rect, label: &str, has: bool, preset: Option<usize>, enabled: bool, saved: &[String]) -> Option<Option<usize>> {
    let resp = fields::combo_box_frame(ui, id, r, label, enabled);
    let th = Theme::current(ui.ctx());
    let mut out = None;
    egui::Popup::menu(&resp).frame(crate::menu::menu_frame(&th)).show(|ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        if fields::list_item(ui, &texts::t("pi-stroke-cat-none"), !has).clicked() {
            out = Some(None);
            ui.close();
        }
        for cat in texts::brush_categories() {
            ui.menu_button(texts::brush_category_label(cat), |ui| {
                for (i, b) in ui_commands::BRUSH_PRESETS.iter().enumerate().filter(|(_, b)| b.category == cat) {
                    if fields::list_item(ui, &texts::brush_preset_label(b), has && preset == Some(i)).clicked() {
                        out = Some(Some(i));
                        ui.close();
                    }
                }
            });
        }
        if !saved.is_empty() {
            ui.menu_button(texts::t("pi-stroke-saved"), |ui| {
                for (k, n) in saved.iter().enumerate() {
                    if fields::list_item(ui, n, n == label).clicked() {
                        out = Some(Some(ui_commands::BRUSH_PRESETS.len() + k));
                        ui.close();
                    }
                }
            });
        }
        // wie im Original am Ende: „Pinselstrichoptionen…“ (öffnet „Pinselstrich bearbeiten“)
        ui.separator();
        if fields::list_item(ui, &texts::t("pi-stroke-options-item"), false).clicked() {
            out = Some(Some(STROKE_OPTIONS_ITEM));
            ui.close();
        }
    });
    out
}

/// Rückgabewert von [`stroke_category`] für „Pinselstrichoptionen…“.
pub const STROKE_OPTIONS_ITEM: usize = usize::MAX;

/// Name des eigenen Benutzer-Texturordners in `%APPDATA%\Firepixel` (technisch englisch, §1c).
pub const USER_TEXTURES_DIR: &str = fp_engine::resources::TEXTURES;
/// Name des eigenen Benutzer-Musterordners in `%APPDATA%\Firepixel`.
pub const USER_PATTERNS_DIR: &str = fp_engine::resources::PATTERNS;

/// Bildendungen, die als Textur bzw. Muster gelesen werden.
pub const RESOURCE_IMAGE_EXTS: [&str; 5] = fp_engine::resources::IMAGE_EXTS;

/// Ordner einer Ressourcenart (`kind` = `Textures` bzw. `Patterns`) in Leserichtung: der
/// Ressourcenordner des Originals (`Configuration\<kind>`), dessen Vorlage für den
/// Benutzerordner (`First Run\<kind>`; das Original kopiert sie beim ersten Start in seinen
/// Benutzerordner und listet beide zusammen, gemessen 52 + 74 → 77 Texturen bzw. 50 + 60 → 60
/// Muster) und der eigene Benutzerordner `%APPDATA%\Firepixel\<kind>` (`user_base`). Dieselben
/// Ordner nutzt die Engine, um Texturen und Muster nur per Name zu laden ([`fp_engine::resources`]).
pub fn resource_dirs(original: Option<&std::path::Path>, user_base: Option<&std::path::Path>, kind: &str) -> Vec<std::path::PathBuf> {
    fp_engine::resources::dirs_in(original, user_base, kind)
}

/// Bilddateien der Ordner (Name = Dateiname ohne Endung, Pfad), je Name nur die erste Fundstelle
/// (ohne Rücksicht auf Groß-/Kleinschreibung), nach Namen geordnet. Fehlende Ordner zählen nicht.
pub fn resource_files_in(dirs: &[std::path::PathBuf]) -> Vec<(String, std::path::PathBuf)> {
    fp_engine::resources::files_in(dirs)
}

fn resource_files(kind: &str) -> Vec<(String, std::path::PathBuf)> {
    resource_files_in(&fp_engine::resources::dirs(kind))
}

/// Texturen (Name, Datei) aus dem Ressourcenordner des Originals, dessen Benutzervorlage und dem
/// eigenen Benutzerordner `%APPDATA%\Firepixel\Textures` (nur zur Laufzeit gelesen, nichts
/// eingebettet; Ordner aus `fp_color::install`). Ohne Ordner: leere Liste.
pub fn texture_files() -> &'static [(String, std::path::PathBuf)] {
    static N: std::sync::OnceLock<Vec<(String, std::path::PathBuf)>> = std::sync::OnceLock::new();
    N.get_or_init(|| resource_files(USER_TEXTURES_DIR))
}

/// Datei einer Textur nach ihrem Namen (ohne Rücksicht auf Groß-/Kleinschreibung).
pub fn texture_file(name: &str) -> Option<&'static std::path::Path> {
    texture_files().iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, p)| p.as_path())
}

/// Texturnamen (Dateinamen ohne Endung) aus [`texture_files`].
pub fn texture_names() -> &'static [String] {
    static N: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    N.get_or_init(|| texture_files().iter().map(|(n, _)| n.clone()).collect())
}

/// Musterdateien (Name, Datei) wie [`texture_files`], aus `Patterns`.
pub fn pattern_files() -> &'static [(String, std::path::PathBuf)] {
    static N: std::sync::OnceLock<Vec<(String, std::path::PathBuf)>> = std::sync::OnceLock::new();
    N.get_or_init(|| resource_files(USER_PATTERNS_DIR))
}

/// Inhalt eines Popups als Antwort-Platzhalter (für Tests: Fläche reservieren).
pub fn reserve(ui: &mut Ui, size: egui::Vec2) -> Rect {
    ui.allocate_exact_size(size, Sense::hover()).0
}
