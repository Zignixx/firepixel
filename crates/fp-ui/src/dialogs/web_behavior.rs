//! Web-Verhalten mit eigenem Dialog (`re\31-webdialoge-innen-einfuegen.md` §1–§3):
//! „Bild austauschen“ (Vorlage 6013, 328 × 230 DLU) und „Navigationsleistenbild festlegen“
//! (Vorlage 6015). Einstiege: Kontextmenü Segment/Hotspot, „+“ und Doppelklick im
//! Verhalten-Bedienfeld. OK legt das Verhalten über `pi.behavior.add` in einem Verlaufsschritt an.
//!
//! Zustand und Regeln (Vorgaben, Grau-Regeln, OK-Übernahme) liegen in den Strukturen und sind ohne
//! Fenster prüfbar; `show` zeichnet nur.

use super::{
    DialogClose, button_row, dialog_frame, dlu_rect, group_box, push_button, t, text_block,
};
use crate::app_state::AppState;
use crate::file_dialog::{self, Request};
use crate::inspector::fields;
use crate::theme::Theme;
use crate::tips::HoverTip;
use egui::{Align2, Color32, Id, Rect, Sense, Stroke, StrokeKind, pos2, vec2};
use fp_model::{Behavior, ElementId, ElementKind};
use serde_json::{Value, json};

/// Ein Eintrag der Zielliste: Segment-Element, Anzeigename, Lage im Dokument.
#[derive(Clone, Debug, PartialEq)]
pub struct SliceEntry {
    pub id: ElementId,
    pub name: String,
    pub bounds: fp_geom::Rect,
}

/// „Bild austauschen“ (6013).
#[derive(Clone, Debug, PartialEq)]
pub struct SwapImageDialog {
    /// Segmente/Hotspots, an die das Verhalten kommt (`ids`).
    pub owners: Vec<ElementId>,
    /// Bearbeitetes Verhalten (Listenposition) beim Doppelklick im Bedienfeld.
    pub edit_index: Option<usize>,
    /// Zielliste (Segmente mit Segmentart ≠ 3, keine Hotspots), Reihenfolge der Web-Objekte.
    pub targets: Vec<SliceEntry>,
    /// Gewählte Zeile; `None` = keine Auswahl (kein Ziel).
    pub target: Option<usize>,
    /// Anzahl Einträge der Statusliste (mindestens 2).
    pub frame_count: usize,
    /// Gewählter Status, 1-basiert wie angezeigt.
    pub frame: usize,
    /// Quelle „Grafikdatei“ statt „Status-Nr.“.
    pub use_file: bool,
    /// Inhalt der Eingabe 5900 (Pfad mit `/`).
    pub file: String,
    pub preload: bool,
    pub restore: bool,
    /// Leinwand (für den Lageplan).
    pub canvas: fp_geom::Rect,
}

impl SwapImageDialog {
    /// Vorgaben (re 31 §1.3): `initial` = vorhandenes Verhalten (Bearbeiten) oder `None` (neu:
    /// kein Ziel, Status 2, Status-Nr., beide Kästchen an).
    pub fn from_doc(
        doc: &fp_model::Document,
        page: usize,
        owners: Vec<ElementId>,
        initial: Option<(usize, &Behavior)>,
    ) -> SwapImageDialog {
        let targets = slice_entries(doc, page);
        let frame_count = doc
            .pages
            .get(page)
            .map(|p| p.root.frame_count as usize)
            .unwrap_or(1)
            .max(2);
        let canvas = doc
            .pages
            .get(page)
            .map(|p| {
                fp_geom::Rect::new(
                    p.canvas.origin.x,
                    p.canvas.origin.y,
                    p.canvas.origin.x + p.canvas.width,
                    p.canvas.origin.y + p.canvas.height,
                )
            })
            .unwrap_or(fp_geom::Rect::new(0.0, 0.0, 1.0, 1.0));
        let b = initial.map(|(_, b)| b);
        let target_slice = b.and_then(|b| b.target_slice);
        let target = target_slice.and_then(|sid| {
            targets.iter().position(|t| {
                fp_engine::docops::element_on_page(doc, page, t.id).is_some_and(|e| matches!(&e.kind, ElementKind::WebObject(w) if w.slice.as_ref().and_then(|s| s.slice_id) == Some(sid)))
            })
        });
        // übergebener Status (0-basiert) + 1; < 1 → 2 (B, @006b1240)
        let frame = b
            .and_then(|b| b.frame)
            .map(|f| f + 1)
            .filter(|f| *f >= 1)
            .unwrap_or(2) as usize;
        let file = b.and_then(|b| b.file.clone()).unwrap_or_default();
        SwapImageDialog {
            owners,
            edit_index: initial.map(|(i, _)| i),
            targets,
            target,
            frame_count,
            frame,
            use_file: !file.is_empty(),
            file,
            preload: b.and_then(|b| b.preload).unwrap_or(true),
            restore: b.and_then(|b| b.restore).unwrap_or(true),
            canvas,
        }
    }

    /// Eingabe 5900 aktiv nur bei „Grafikdatei“ (B).
    pub fn file_enabled(&self) -> bool {
        self.use_file
    }
    /// Klappliste 5886 aktiv nur bei „Status-Nr.“ (B).
    pub fn frame_list_enabled(&self) -> bool {
        !self.use_file
    }
    /// Klick „Status-Nr.“: Datei verwerfen, Eingabe grau (re 31 §1.4).
    pub fn choose_frame(&mut self) {
        if self.use_file {
            self.use_file = false;
            self.file.clear();
        }
    }
    /// Klick „Grafikdatei“: Eingabe aktiv, Klappliste grau.
    pub fn choose_file(&mut self) {
        self.use_file = true;
    }
    /// Ergebnis von „Durchsuchen…“: `\` → `/`, Grafikdatei an.
    pub fn browsed(&mut self, path: &str) {
        self.file = path.replace('\\', "/");
        self.use_file = true;
    }
    /// Startpfad für „Durchsuchen…“ (`/` → `\`).
    pub fn browse_start(&self) -> String {
        self.file.replace('/', "\\")
    }

    /// Zeile der Zielliste unter einem Dokumentpunkt (Klick in den Lageplan, oberstes zuerst).
    pub fn pick_at(&self, p: fp_geom::Point) -> Option<usize> {
        self.targets
            .iter()
            .enumerate()
            .rev()
            .find(|(_, t)| {
                p.x >= t.bounds.left
                    && p.x < t.bounds.right
                    && p.y >= t.bounds.top
                    && p.y < t.bounds.bottom
            })
            .map(|(i, _)| i)
    }

    /// Parameter für `pi.behavior.add` (OK, re 31 §1.5/§1.6). Ohne Ziel wird das Verhalten trotzdem
    /// angelegt; der Export verwirft es still.
    pub fn params(&self) -> Value {
        let ids: Vec<u64> = self.owners.iter().map(|i| i.0).collect();
        let mut v = json!({
            "ids": ids,
            "action": 2,
            "event": 0,
            "frame": if self.use_file { -1 } else { self.frame as i64 - 1 },
            "preload": self.preload,
            "restore": self.restore,
        });
        if self.use_file {
            v["file"] = json!(self.file.replace('\\', "/"));
        }
        if let Some(t) = self.target.and_then(|i| self.targets.get(i)) {
            v["target"] = json!(t.id.0);
        }
        if let Some(i) = self.edit_index {
            v["index"] = json!(i);
        }
        v
    }
}

/// „Navigationsleistenbild festlegen“ (6015).
#[derive(Clone, Debug, PartialEq)]
pub struct NavBarDialog {
    pub owners: Vec<ElementId>,
    /// Bearbeitetes Verhalten (Doppelklick im Bedienfeld).
    pub edit_index: Option<usize>,
    /// Kontextmenü: vorhandenes Navigationsleistenbild an seiner Position ersetzen (re 31 §2.3).
    pub replace: bool,
    /// „OverDown-Status einschließen (Status 4)“.
    pub over_down: bool,
    /// „Beim Laden Down-Bild anzeigen“.
    pub down_on_load: bool,
    /// Eigenes Segment (Vorschau), Leinwand.
    pub own: Option<fp_geom::Rect>,
}

impl NavBarDialog {
    /// Kontextmenü (`replace` = true): Werte eines vorhandenen Navigationsleistenbilds, sonst beide aus.
    /// Bedienfeld „+“ (`replace` = false): beide aus, immer anhängen.
    pub fn new(
        owner: Option<&fp_model::Element>,
        owners: Vec<ElementId>,
        replace: bool,
        edit: Option<(usize, &Behavior)>,
    ) -> NavBarDialog {
        let existing = edit.map(|(_, b)| b.clone()).or_else(|| {
            if !replace {
                return None;
            }
            behaviors_of(owner?).into_iter().find(|b| b.action == 0)
        });
        let own = owner.and_then(|e| match &e.kind {
            ElementKind::WebObject(w) => Some(w.bounds),
            _ => None,
        });
        NavBarDialog {
            owners,
            edit_index: edit.map(|(i, _)| i),
            replace,
            over_down: existing
                .as_ref()
                .and_then(|b| b.flags)
                .is_some_and(|f| f & 1 != 0),
            down_on_load: existing.as_ref().and_then(|b| b.checked).unwrap_or(false),
            own,
        }
    }

    /// Parameter für `pi.behavior.add`: Art 0, `ROLi = 2 | OverDown`, `CHKb`.
    pub fn params(&self) -> Value {
        let ids: Vec<u64> = self.owners.iter().map(|i| i.0).collect();
        let mut v = json!({
            "ids": ids,
            "action": 0,
            "event": 0,
            "flags": 2 | i64::from(self.over_down),
            "downOnLoad": self.down_on_load,
            "replace": self.replace,
        });
        if let Some(i) = self.edit_index {
            v["index"] = json!(i);
        }
        v
    }
}

/// Offener Web-Verhaltensdialog.
#[derive(Clone, Debug, PartialEq)]
pub enum WebBehaviorDialog {
    Swap(SwapImageDialog),
    NavBar(NavBarDialog),
}

fn behaviors_of(e: &fp_model::Element) -> Vec<Behavior> {
    match &e.kind {
        ElementKind::WebObject(w) => w
            .behaviors
            .as_ref()
            .map(|b| b.items.clone())
            .unwrap_or_default(),
        ElementKind::Instance(i) => i
            .behaviors
            .as_ref()
            .map(|b| b.items.clone())
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// Zielliste: alle Segmente der Seite mit Segmentart ≠ 3 (leer/HTML), keine Hotspots (re 31 §1.3).
/// Reihenfolge der Web-Objekte (unterstes zuerst, A); Text = Exportname, sonst der automatische Name.
pub fn slice_entries(doc: &fp_model::Document, page: usize) -> Vec<SliceEntry> {
    let mut out = Vec::new();
    let Some(pg) = doc.pages.get(page) else {
        return out;
    };
    fp_model::traverse::walk_root(&pg.root, fp_model::RootRef::Page(page), &mut |_, _, e| {
        if let ElementKind::WebObject(w) = &e.kind
            && let Some(s) = &w.slice
            && s.slice_type != 3
        {
            let name = w
                .file_name
                .clone()
                .filter(|n| !n.is_empty())
                .or_else(|| {
                    fp_export::slices::auto_name_of(
                        doc,
                        page,
                        e.id,
                        "",
                        fp_export::slices::NameDate::today(),
                    )
                })
                .or_else(|| e.common.name.clone())
                .unwrap_or_default();
            out.push(SliceEntry {
                id: e.id,
                name,
                bounds: w.bounds,
            });
        }
    });
    // Durchlauf liefert das oberste zuerst; die Liste zeigt die Anlegereihenfolge (unterstes zuerst)
    out.reverse();
    out
}

fn selected_web(st: &AppState) -> Option<(Vec<ElementId>, std::sync::Arc<fp_model::Element>)> {
    let d = st.session.active()?;
    let ids: Vec<ElementId> = d
        .selection
        .elements
        .iter()
        .copied()
        .filter(|id| {
            fp_engine::docops::element_on_page(&d.doc, d.page, *id).is_some_and(|e| {
                matches!(e.kind, ElementKind::WebObject(_) | ElementKind::Instance(_))
            })
        })
        .collect();
    let first = fp_engine::docops::element_on_page(&d.doc, d.page, *ids.first()?)?;
    Some((ids, first))
}

/// „Bild austauschen“ öffnen: neues Verhalten (`edit` = None) oder das Verhalten an `edit` bearbeiten.
pub fn open_swap(st: &mut AppState, edit: Option<usize>) {
    let Some((ids, first)) = selected_web(st) else {
        return;
    };
    let Some(d) = st.session.active() else { return };
    let list = behaviors_of(&first);
    let initial = edit.and_then(|i| list.get(i).map(|b| (i, b)));
    st.dialogs.web_behavior = Some(WebBehaviorDialog::Swap(SwapImageDialog::from_doc(
        &d.doc, d.page, ids, initial,
    )));
}

/// „Navigationsleistenbild festlegen“ öffnen; `replace` = Einstieg aus dem Kontextmenü.
pub fn open_nav_bar(st: &mut AppState, replace: bool, edit: Option<usize>) {
    let Some((ids, first)) = selected_web(st) else {
        return;
    };
    let list = behaviors_of(&first);
    let initial = edit.and_then(|i| list.get(i).map(|b| (i, b)));
    st.dialogs.web_behavior = Some(WebBehaviorDialog::NavBar(NavBarDialog::new(
        Some(&first),
        ids,
        replace,
        initial,
    )));
}

/// Lageplan: Segmentrechtecke maßstäblich, gewähltes hervorgehoben, eigenes schraffiert.
fn draw_map(ui: &mut egui::Ui, r: Rect, d: &mut SwapImageDialog) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let resp = ui
        .interact(r, Id::new("fp-swap-map"), Sense::click())
        .hover_tip(t("dlg-swap-map-tip"));
    ui.painter().rect_filled(r, 0.0, c.list_bg);
    ui.painter()
        .rect_stroke(r, 0.0, Stroke::new(1.0, c.list_border), StrokeKind::Inside);
    let cv = d.canvas;
    let (cw, ch) = ((cv.right - cv.left).max(1.0), (cv.bottom - cv.top).max(1.0));
    let inner = r.shrink(4.0);
    let s = (inner.width() / cw).min(inner.height() / ch);
    let o = pos2(
        inner.center().x - cw * s / 2.0,
        inner.center().y - ch * s / 2.0,
    );
    let map = |b: &fp_geom::Rect| {
        Rect::from_min_max(
            pos2(o.x + (b.left - cv.left) * s, o.y + (b.top - cv.top) * s),
            pos2(o.x + (b.right - cv.left) * s, o.y + (b.bottom - cv.top) * s),
        )
    };
    ui.painter().rect_filled(
        Rect::from_min_size(o, vec2(cw * s, ch * s)),
        0.0,
        Color32::WHITE,
    );
    for (i, e) in d.targets.iter().enumerate() {
        let rr = map(&e.bounds);
        let sel = d.target == Some(i);
        let own = d.owners.contains(&e.id);
        let fill = if sel {
            Color32::from_rgba_unmultiplied(0, 120, 215, 140)
        } else if own {
            Color32::from_rgba_unmultiplied(0, 160, 0, 60)
        } else {
            Color32::from_rgba_unmultiplied(0, 160, 0, 30)
        };
        ui.painter().rect_filled(rr, 0.0, fill);
        ui.painter().rect_stroke(
            rr,
            0.0,
            Stroke::new(
                1.0,
                if sel {
                    c.focus
                } else {
                    Color32::from_rgb(0, 140, 0)
                },
            ),
            StrokeKind::Inside,
        );
    }
    if resp.clicked()
        && let Some(p) = resp.interact_pointer_pos()
        && s > 0.0
    {
        let dp = fp_geom::Point::new((p.x - o.x) / s + cv.left, (p.y - o.y) / s + cv.top);
        if let Some(i) = d.pick_at(dp) {
            d.target = Some(i);
        }
    }
}

/// Zielliste als ein Tabstopp (Pfeiltasten wechseln die Zeile).
fn draw_list(ui: &mut egui::Ui, r: Rect, d: &mut SwapImageDialog, autofocus: bool) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let id = Id::new("fp-swap-list");
    let resp = ui.interact(r, id, Sense::click());
    if autofocus {
        super::autofocus_widget(ui, id);
    }
    let inner = fields::list_box(ui, r);
    if resp.has_focus() {
        ui.painter()
            .rect_stroke(r, 0.0, Stroke::new(1.0, c.focus), StrokeKind::Inside);
        let n = d.targets.len();
        if n > 0 {
            if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                d.target = Some(d.target.map_or(0, |i| (i + 1).min(n - 1)));
            }
            if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                d.target = Some(d.target.map_or(0, |i| i.saturating_sub(1)));
            }
        }
    }
    let row_h = 16.0;
    for (i, e) in d.targets.iter().enumerate() {
        let rr = Rect::from_min_size(
            pos2(inner.left(), inner.top() + i as f32 * row_h),
            vec2(inner.width(), row_h),
        );
        if rr.bottom() > inner.bottom() {
            break;
        }
        if d.target == Some(i) {
            ui.painter().rect_filled(rr, 0.0, c.list_selected_bg);
        }
        crate::widgets::text_clipped(
            ui.painter(),
            pos2(rr.left() + 4.0, rr.center().y),
            Align2::LEFT_CENTER,
            &e.name,
            theme.ui_font(),
            c.text,
            rr.width() - 6.0,
        );
    }
    if resp.clicked()
        && let Some(p) = resp.interact_pointer_pos()
    {
        let i = ((p.y - inner.top()) / row_h).floor() as usize;
        if i < d.targets.len() {
            d.target = Some(i);
        }
    }
}

/// Zeigt den offenen Dialog.
pub fn show(ctx: &egui::Context, st: &mut AppState) {
    let Some(dlg) = st.dialogs.web_behavior.clone() else {
        return;
    };
    match dlg {
        WebBehaviorDialog::Swap(d) => show_swap(ctx, st, d),
        WebBehaviorDialog::NavBar(d) => show_nav(ctx, st, d),
    }
}

fn show_swap(ctx: &egui::Context, st: &mut AppState, mut d: SwapImageDialog) {
    let (ok_l, cancel_l) = (t("btn-ok"), t("btn-cancel"));
    let mut outcome = None;
    let mut browse = false;
    let size = super::dlu(328.0, 230.0);
    let r = dialog_frame(
        ctx,
        "fp-swap-image",
        &t("dlg-swap-title"),
        size,
        |ui, body| {
            let o = body.min;
            group_box(ui, dlu_rect(o, [5.0, 1.0, 256.0, 111.0]), "");
            text_block(
                ui,
                dlu_rect(o, [10.0, 9.0, 244.0, 16.0]),
                t("dlg-swap-intro").replace('&', ""),
            );
            // Tab-Folge wie die Vorlage: Liste → Status-Nr. → Klappliste → Grafikdatei → Eingabe →
            // Durchsuchen → Vorab laden → Wiederherstellen → OK → Abbrechen (re 31 §1.2)
            draw_list(ui, dlu_rect(o, [10.0, 30.0, 102.0, 76.0]), &mut d, true);
            draw_map(ui, dlu_rect(o, [117.0, 30.0, 137.0, 76.0]), &mut d);
            group_box(
                ui,
                dlu_rect(o, [5.0, 118.0, 256.0, 43.0]),
                &t("dlg-swap-source").replace('&', ""),
            );
            if fields::radio(
                ui,
                "fp-swap-frame",
                dlu_rect(o, [10.0, 131.0, 57.0, 11.0]),
                &t("dlg-swap-frame-no").replace('&', ""),
                !d.use_file,
                true,
            ) {
                d.choose_frame();
            }
            let items: Vec<String> = (1..=d.frame_count).map(|n| n.to_string()).collect();
            let refs: Vec<&str> = items.iter().map(String::as_str).collect();
            let fl = d.frame_list_enabled();
            if let Some(i) = fields::combo(
                ui,
                "fp-swap-frames",
                dlu_rect(o, [64.0, 130.0, 108.0, 12.0]),
                &refs,
                Some(d.frame.saturating_sub(1)),
                fl,
            ) {
                d.frame = i + 1;
            }
            if fields::radio(
                ui,
                "fp-swap-file",
                dlu_rect(o, [10.0, 144.0, 54.0, 11.0]),
                &t("dlg-swap-file").replace('&', ""),
                d.use_file,
                true,
            ) {
                d.choose_file();
            }
            let fr = dlu_rect(o, [64.0, 144.0, 128.0, 12.0]);
            let fe = d.file_enabled();
            let theme = Theme::current(ui.ctx());
            ui.painter().rect_filled(
                fr,
                0.0,
                if fe {
                    theme.colors.field_bg
                } else {
                    theme.colors.frame_bg
                },
            );
            ui.painter().rect_stroke(
                fr,
                0.0,
                Stroke::new(1.0, theme.colors.field_border),
                StrokeKind::Inside,
            );
            ui.add_enabled_ui(fe, |ui| {
                ui.put(
                    fr.shrink(2.0),
                    egui::TextEdit::singleline(&mut d.file)
                        .id(Id::new("fp-swap-path"))
                        .frame(egui::Frame::NONE)
                        .font(theme.ui_font())
                        .desired_width(fr.width() - 4.0),
                );
            });
            if push_button(
                ui,
                dlu_rect(o, [195.0, 144.0, 63.0, 12.0]),
                &t("dlg-swap-browse"),
                false,
                true,
            ) {
                browse = true;
            }
            if let Some(v) = fields::checkbox(
                ui,
                "fp-swap-preload",
                dlu_rect(o, [10.0, 164.0, 248.0, 9.0]),
                &t("dlg-swap-preload").replace('&', ""),
                d.preload,
                true,
            ) {
                d.preload = v;
            }
            if let Some(v) = fields::checkbox(
                ui,
                "fp-swap-restore",
                dlu_rect(o, [10.0, 176.0, 248.0, 9.0]),
                &t("dlg-swap-restore").replace('&', ""),
                d.restore,
                true,
            ) {
                d.restore = v;
            }
            text_block(
                ui,
                dlu_rect(o, [7.0, 190.0, 256.0, 34.0]),
                t("dlg-swap-hint"),
            );
            let b = button_row(
                ui.ctx(),
                body.right() - 8.0,
                body.top() + super::dlu(0.0, 7.0).y,
                &[&ok_l],
                75.0,
                23.0,
                8.0,
            );
            let b2 = Rect::from_min_size(pos2(b[0].left(), b[0].bottom() + 4.0), b[0].size());
            if push_button(ui, b[0], &ok_l, true, true) {
                outcome = Some(true);
            }
            if push_button(ui, b2, &cancel_l, false, true) {
                outcome = Some(false);
            }
        },
    );
    if browse {
        let start = d.browse_start();
        st.dialogs.web_behavior = Some(WebBehaviorDialog::Swap(d));
        let dir = std::path::Path::new(&start)
            .parent()
            .map(|p| p.to_path_buf());
        let req = Request::open(t("dlg-swap-browse").replace('&', ""))
            .filter(t("dlg-swap-filter-gif"), &["gif"])
            .filter(t("dlg-swap-filter-jpg"), &["jpg"])
            .filter(t("dlg-swap-filter-png"), &["png"])
            .filter(t("dlg-swap-filter-readable"), &["gif", "jpg", "png"])
            .filter(t("dlg-swap-filter-all"), &["*"])
            .dir(dir.as_deref());
        file_dialog::request(st, req, |st, paths| {
            if let (Some(WebBehaviorDialog::Swap(d)), Some(p)) =
                (st.dialogs.web_behavior.as_mut(), paths.first())
            {
                d.browsed(&p.to_string_lossy());
            }
        });
        return;
    }
    match outcome.or((r == DialogClose::Cancelled).then_some(false)) {
        Some(true) => {
            st.dialogs.web_behavior = None;
            st.run("pi.behavior.add", d.params());
        }
        Some(false) => st.dialogs.web_behavior = None,
        None => st.dialogs.web_behavior = Some(WebBehaviorDialog::Swap(d)),
    }
}

fn show_nav(ctx: &egui::Context, st: &mut AppState, mut d: NavBarDialog) {
    let (ok_l, cancel_l) = (t("btn-ok"), t("btn-cancel"));
    let mut outcome = None;
    let size = super::dlu(328.0, 230.0);
    let r = dialog_frame(
        ctx,
        "fp-nav-bar",
        &t("dlg-navbar-title"),
        size,
        |ui, body| {
            let o = body.min;
            group_box(
                ui,
                dlu_rect(o, [5.0, 4.0, 191.0, 86.0]),
                &t("dlg-navbar-preview"),
            );
            // Vorschau (A): Status Up/Over/Down (+ OverDown) des eigenen Segments nebeneinander, nur Anzeige
            let pr = dlu_rect(o, [11.0, 16.0, 177.0, 68.0]);
            let theme = Theme::current(ui.ctx());
            ui.painter().rect_filled(pr, 0.0, theme.colors.list_bg);
            ui.painter().rect_stroke(
                pr,
                0.0,
                Stroke::new(1.0, theme.colors.list_border),
                StrokeKind::Inside,
            );
            let states = [
                "dlg-navbar-up",
                "dlg-navbar-over",
                "dlg-navbar-down",
                "dlg-navbar-overdown-state",
            ];
            let n = if d.over_down { 4 } else { 3 };
            let cell = (pr.width() - 8.0) / n as f32;
            let aspect = d
                .own
                .map(|b| ((b.bottom - b.top) / (b.right - b.left).max(1.0)).clamp(0.2, 1.0))
                .unwrap_or(0.5);
            for (k, key) in states.iter().take(n).enumerate() {
                let w = cell - 6.0;
                let h = (w * aspect).min(pr.height() - 26.0);
                let cr = Rect::from_min_size(
                    pos2(pr.left() + 4.0 + k as f32 * cell + 3.0, pr.top() + 6.0),
                    vec2(w, h),
                );
                let down = k >= 2 || (k == 0 && d.down_on_load);
                ui.painter().rect_filled(
                    cr,
                    0.0,
                    if down {
                        Color32::from_gray(200)
                    } else {
                        Color32::from_gray(235)
                    },
                );
                ui.painter().rect_stroke(
                    cr,
                    0.0,
                    Stroke::new(
                        1.0,
                        Color32::from_gray(if k == 1 || k == 3 { 90 } else { 150 }),
                    ),
                    StrokeKind::Inside,
                );
                crate::widgets::text_clipped(
                    ui.painter(),
                    pos2(cr.center().x, cr.bottom() + 9.0),
                    Align2::CENTER_CENTER,
                    &t(key),
                    theme.small_font(),
                    theme.colors.text,
                    cell,
                );
            }
            group_box(
                ui,
                dlu_rect(o, [5.0, 95.0, 256.0, 40.0]),
                &t("dlg-navbar-options"),
            );
            // Tab-Folge: OverDown → Down beim Laden → OK → Abbrechen; Startfokus OverDown (re 31 §2.2)
            let first = Id::new("fp-nav-overdown");
            super::autofocus_widget(ui, first);
            if let Some(v) = fields::checkbox(
                ui,
                "fp-nav-overdown",
                dlu_rect(o, [10.0, 106.0, 245.0, 10.0]),
                &t("dlg-navbar-overdown").replace('&', ""),
                d.over_down,
                true,
            ) {
                d.over_down = v;
            }
            if let Some(v) = fields::checkbox(
                ui,
                "fp-nav-downonload",
                dlu_rect(o, [10.0, 118.0, 247.0, 10.0]),
                &t("dlg-navbar-down-on-load").replace('&', ""),
                d.down_on_load,
                true,
            ) {
                d.down_on_load = v;
            }
            group_box(ui, dlu_rect(o, [5.0, 139.0, 256.0, 86.0]), "");
            text_block(
                ui,
                dlu_rect(o, [10.0, 145.0, 246.0, 17.0]),
                t("dlg-navbar-hint1"),
            );
            text_block(
                ui,
                dlu_rect(o, [10.0, 166.0, 246.0, 26.0]),
                t("dlg-navbar-hint2"),
            );
            text_block(
                ui,
                dlu_rect(o, [10.0, 196.0, 246.0, 26.0]),
                t("dlg-navbar-hint3"),
            );
            let b = button_row(
                ui.ctx(),
                body.right() - 8.0,
                body.top() + super::dlu(0.0, 7.0).y,
                &[&ok_l],
                75.0,
                23.0,
                8.0,
            );
            let b2 = Rect::from_min_size(pos2(b[0].left(), b[0].bottom() + 4.0), b[0].size());
            if push_button(ui, b[0], &ok_l, true, true) {
                outcome = Some(true);
            }
            if push_button(ui, b2, &cancel_l, false, true) {
                outcome = Some(false);
            }
        },
    );
    match outcome.or((r == DialogClose::Cancelled).then_some(false)) {
        Some(true) => {
            st.dialogs.web_behavior = None;
            st.run("pi.behavior.add", d.params());
        }
        Some(false) => st.dialogs.web_behavior = None,
        None => st.dialogs.web_behavior = Some(WebBehaviorDialog::NavBar(d)),
    }
}
