//! Quadranten: Leervorlagen, Dokument (14100/14031/6564), Werkzeug (14006), Auswahltyp
//! (14101/14102) und B/H/X/Y (14007). Lage in DLU wie in `dialoge.json`.

use super::fields::{self, Dl};
use super::texts::t;
use super::QuadrantRegistry;
use crate::app_state::{Action, AppState};
use crate::icons::{self, IconStyle};
use crate::theme::{Theme, ToolGroup};
use crate::widgets;
use egui::{pos2, vec2, Align2, Color32, Rect, Stroke, StrokeKind, Ui};
use serde_json::json;

pub(super) fn register(r: &mut QuadrantRegistry) {
    for empty in [14079, 14080, 14081, 14082, 14083, 14084] {
        r.register(empty, |_, _, _| {});
    }
    r.register(14100, document_q1);
    r.register(14031, document_q3);
    r.register(6564, document_q4);
    r.register(14006, tool_q1);
    r.register(14101, selection_q1);
    r.register(14102, multi_q1);
    r.register(14007, box_q2);
}

fn thumb_frame(ui: &Ui, r: Rect) {
    let theme = Theme::current(ui.ctx());
    widgets::checker(ui.painter(), r, 5.0, &theme);
    ui.painter().rect_stroke(r, 0.0, Stroke::new(1.0, theme.colors.list_border), StrokeKind::Inside);
}

/// Dokument: Miniatur, „Dokument“, Dokumentname.
fn document_q1(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let d = Dl::at(r.min + vec2(6.0, 0.0));
    let thumb = d.r(0.0, 3.0, 27.0, 23.0);
    thumb_frame(ui, thumb);
    let Some(doc) = st.session.active() else { return };
    let bg = doc.doc.pages.get(doc.page).map(|p| p.root.background).unwrap_or(fp_color::Argb(0xffff_ffff));
    if (bg.0 >> 24) > 0 {
        ui.painter().rect_filled(thumb.shrink(1.0), 0.0, Color32::from_rgb((bg.0 >> 16) as u8, (bg.0 >> 8) as u8, bg.0 as u8));
    }
    if let Some(tex) = st.canvases.get(&doc.id).and_then(|cc| cc.texture.clone()) {
        let ts = tex.size_vec2();
        let s = (thumb.width() / ts.x).min(thumb.height() / ts.y);
        let ir = Rect::from_center_size(thumb.center(), ts * s);
        ui.painter().image(tex.id(), ir, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
    }
    let row = d.r(0.0, 3.0, 1.0, 8.0);
    fields::label(ui, Rect::from_min_max(pos2(thumb.right() + 6.0, row.top()), pos2(r.right() - 3.0, row.bottom())), &t("pi-doc-title"), true);
    let name = doc.title.trim_end_matches(".png").trim_end_matches(".fw").to_string();
    widgets::text_clipped(ui.painter(), pos2(thumb.right() + 6.0, d.r(0.0, 16.0, 1.0, 8.0).center().y), Align2::LEFT_CENTER, &name, theme.ui_font(), c.text, r.right() - thumb.right() - 10.0);
}

/// Dokument Zeile 1 (14031): Leinwandfarbe, Leinwandgröße…, Bildgröße…, Leinwand anpassen;
/// Standardoptionen für Export.
fn document_q3(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let theme = Theme::current(ui.ctx());
    let d = Dl::at(r.min + vec2(6.0, 2.0));
    let en = st.session.active().is_some();
    fields::label(ui, d.r(0.0, 2.0, 41.0, 8.0), &t("pi-doc-canvas"), en);
    let bg = st.session.active().and_then(|d| d.doc.pages.get(d.page).map(|p| p.root.background)).unwrap_or(fp_color::Argb(0xffff_ffff));
    // Farbwähler ohne Deckkraftfeld wie im Original (PAR 16/02)
    match super::popups::color_button_with(ui, "pi-canvas-color", d.r(45.0, 0.0, 16.0, 13.0), st, (bg.a() > 0).then_some(bg), en, super::color_picker::Footer::Canvas) {
        Some(super::popups::ColorChoice::Color(c)) => st.run("modify.canvas.canvasColor", json!({"color": crate::ui_commands::hex(c)})),
        Some(super::popups::ColorChoice::None) => st.run("modify.canvas.canvasColor", json!({"color": "#ffffff00"})),
        _ => {}
    }
    for (label, cmd, rr) in [("pi-doc-canvas-size", "modify.canvas.canvasSize", d.r(66.0, 0.0, 75.0, 12.0)), ("pi-doc-image-size", "modify.canvas.imageSize", d.r(146.0, 0.0, 75.0, 12.0)), ("pi-doc-fit-canvas", "modify.canvas.fitCanvas", d.r(230.0, 0.0, 84.0, 12.0))] {
        let available = st.session.is_available(cmd);
        if fields::button(ui, ("pi-doc", cmd), rr, &t(label), available) {
            st.push(Action::Run(cmd.into(), json!({})));
        }
    }
    // Standardoptionen für Export (14755) mit Symbol (14754)
    icons::paint(ui.painter(), d.r(28.0, 17.0, 12.0, 11.0), icons::Icon::Export, &IconStyle::new(&theme, Some(ToolGroup::Web), en));
    // Reihenfolge wie im Original (PAR 16/07): dessen acht Vorgaben zuerst, die Firepixel-Formate dahinter
    let all = fp_export::options::builtin_presets();
    let order = export_preset_order(&all);
    let names: Vec<String> = order.iter().map(|&i| crate::panels::optimize::preset_label(all[i].name)).collect();
    let cur = st.session.active().map(|d| fp_export::doc_export::page_settings(&d.doc, d.page));
    let sel = cur.as_ref().and_then(crate::panels::optimize::preset_index).and_then(|i| order.iter().position(|&k| k == i));
    let text = match (sel, cur.as_ref()) {
        (Some(i), _) => names[i].clone(),
        (None, Some(c)) => crate::panels::optimize::settings_label(c),
        _ => String::new(),
    };
    let resp = fields::combo_box_frame(ui, egui::Id::new("pi-export-preset"), d.r(45.0, 16.0, 178.0, 13.0), &text, en);
    let mut chosen = None;
    egui::Popup::menu(&resp).frame(crate::menu::menu_frame(&theme)).width(d.r(0.0, 0.0, 178.0, 1.0).width()).show(|ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        for (i, n) in names.iter().enumerate() {
            if fields::list_item(ui, n, Some(i) == sel).clicked() {
                chosen = Some(i);
                ui.close();
            }
        }
    });
    if let Some(i) = chosen {
        crate::panels::optimize::apply_settings(st, &all[order[i]].settings);
    }
    // Kein Knopf „Bearbeiten…“ neben der Liste: das Original hat dort keinen (PARITAET_A W29);
    // die Exporteinstellungen bearbeitet man im Optimieren-Bedienfeld bzw. in der Bildvorschau.
}

/// Vorgaben der Exportformat-Liste im Dokument-Inspektor in der Reihenfolge des Originals: PNG32,
/// JPEG - Höhere Qualität, JPEG - Kleinere Datei, GIF Web 216, GIF WebSnap 256, GIF WebSnap 128,
/// GIF Adaptiv 256, Animiertes GIF WebSnap 128; danach die übrigen (Firepixel-Formate) in ihrer
/// Reihenfolge. Liefert Indizes in `presets`.
pub fn export_preset_order(presets: &[fp_export::options::Preset]) -> Vec<usize> {
    const ORIGINAL: [&str; 8] = [
        "panel-opt-preset-png32",
        "panel-opt-preset-jpeg-better",
        "panel-opt-preset-jpeg-smaller",
        "panel-opt-preset-gif-web216",
        "panel-opt-preset-gif-websnap256",
        "panel-opt-preset-gif-websnap128",
        "panel-opt-preset-gif-adaptive256",
        "panel-opt-preset-anim-gif-websnap128",
    ];
    let mut v: Vec<usize> = ORIGINAL.iter().filter_map(|k| presets.iter().position(|p| crate::panels::optimize::preset_key(p.name) == Some(*k))).collect();
    let rest: Vec<usize> = (0..presets.len()).filter(|i| !v.contains(i)).collect();
    v.extend(rest);
    v
}

/// Dokument Zeile 2 (6564): Status.
fn document_q4(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let Some(doc) = st.session.active() else { return };
    let (n, frame) = (doc.frame_count(), doc.frame);
    let d = Dl::at(r.min + vec2(6.0, 4.0));
    fields::label(ui, d.r(9.0, 4.0, 24.0, 8.0), &t("pi-doc-state"), true);
    let names: Vec<String> = (0..n).map(|i| crate::panels::state_name(st, i)).collect();
    let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
    if let Some(i) = fields::combo(ui, "pi-state", d.r(37.0, 2.0, 119.0, 13.0), &refs, Some(frame), true)
        && i != frame
    {
        st.push(Action::SetFrame(i));
    }
}

/// Werkzeug: Symbol und Name (14006).
fn tool_q1(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let theme = Theme::current(ui.ctx());
    let id = st.session.tools.active;
    let info = fp_engine::tools::info(id);
    let group = info.map(|i| ToolGroup::from_section(i.section));
    let d = Dl::at(r.min + vec2(6.0, 0.0));
    icons::paint(ui.painter(), d.r(0.0, 5.0, 14.0, 13.0), icons::tool_icon(id), &IconStyle::new(&theme, group, true));
    let label = tool_q1_label(st);
    let lr = d.r(21.0, 1.0, 54.0, 26.0);
    let font = theme.small_font();
    let col = theme.colors.text;
    let lines: Vec<String> = if widgets::text_width(ui, &label, &font) <= lr.width() {
        widgets::text_clipped(ui.painter(), pos2(lr.left(), lr.top() + 8.0), Align2::LEFT_CENTER, &label, font, col, lr.width());
        vec![label.clone()]
    } else {
        // lange Werkzeugnamen bricht das Original in zwei Zeilen um („Rote Augen / entfernen“,
        // „Pfadausbesserung / - additiv“, PARITAET_A W10/W28), und zwar nur an Leerzeichen bzw.
        // nach einem Bindestrich im Wort, nie mitten im Wort; erst ab drei Zeilen „…“. Ein Wort,
        // das allein breiter als das Feld ist, darf bis an den Rand des Quadranten reichen, sonst
        // wird die Schrift für diese Zeile kleiner.
        let max_w = (r.right() - 2.0 - lr.left()).max(lr.width());
        let rows = word_wrap(ui, &label, &font, lr.width());
        if rows.len() <= 2 {
            let row_h = ui.ctx().fonts_mut(|f| f.row_height(&font));
            let top = lr.top() + 8.0 - row_h / 2.0;
            for (k, row) in rows.iter().enumerate() {
                let mut f = font.clone();
                while widgets::text_width(ui, row, &f) > max_w && f.size > 6.0 {
                    f.size -= 0.5;
                }
                let y = top + k as f32 * row_h + (row_h - ui.ctx().fonts_mut(|x| x.row_height(&f))) / 2.0;
                widgets::text_clipped(ui.painter(), pos2(lr.left(), y), Align2::LEFT_TOP, row, f, col, max_w);
            }
            rows
        } else {
            widgets::text_clipped(ui.painter(), pos2(lr.left(), lr.top() + 8.0), Align2::LEFT_CENTER, &label, font, col, lr.width());
            vec![format!("{label}…")]
        }
    };
    ui.ctx().data_mut(|dd| dd.insert_temp(egui::Id::new(TOOL_Q1_LINES), lines));
}

/// Bricht `text` wie ein Windows-Textfeld mit Wortumbruch um: nur nach Leerzeichen und nach einem
/// Bindestrich zwischen zwei Buchstaben, nie mitten im Wort (PARITAET_A W28). Ein einzelnes Wort,
/// das breiter als `width` ist, steht allein in seiner Zeile (ungekürzt).
fn word_wrap(ui: &Ui, text: &str, font: &egui::FontId, width: f32) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut segs: Vec<String> = Vec::new();
    let mut cur = String::new();
    for (i, &ch) in chars.iter().enumerate() {
        cur.push(ch);
        let after_hyphen = ch == '-' && i > 0 && chars[i - 1].is_alphabetic() && chars.get(i + 1).is_some_and(|c| c.is_alphabetic());
        if ch == ' ' || after_hyphen {
            segs.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        segs.push(cur);
    }
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for seg in segs {
        let cand = format!("{line}{seg}");
        if line.trim().is_empty() || widgets::text_width(ui, cand.trim_end(), font) <= width {
            line = cand;
        } else {
            lines.push(line.trim_end().to_string());
            line = seg.trim_start().to_string();
        }
    }
    if !line.trim().is_empty() {
        lines.push(line.trim_end().to_string());
    }
    lines
}

/// Schlüssel der zuletzt im Werkzeugkopf gezeichneten Zeilen (für Tests: [`tool_header_lines`]).
const TOOL_Q1_LINES: &str = "pi-tool-q1-lines";

/// Zeilen des Werkzeugnamens im Inspektorkopf, wie zuletzt gezeichnet (Tests).
pub fn tool_header_lines(ctx: &egui::Context) -> Vec<String> {
    ctx.data(|d| d.get_temp::<Vec<String>>(egui::Id::new(TOOL_Q1_LINES))).unwrap_or_default()
}

/// Name im Werkzeugkopf: Werkzeugname; bei der automatischen Form wie im Original der Name der
/// gewählten Form („Abgefastes Rechteck“, PARITAET_A W27).
fn tool_q1_label(st: &AppState) -> String {
    let id = st.session.tools.active;
    if id == fp_engine::ToolId::AutoShape {
        let shape = st.session.tools.options.auto_shape.as_str();
        if let Some((de, en)) = crate::tools_panel::AUTO_SHAPES.iter().find(|(de, _)| *de == shape) {
            return crate::tools_panel::shape_label(en, de);
        }
        if !shape.is_empty() {
            return shape.to_string();
        }
    }
    fp_engine::tools::info(id).map(super::texts::tool_label).unwrap_or_default()
}

/// Auswahl (14101): Miniatur, Typtext, Objektname (editierbar).
fn selection_q1(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    ui.ctx().data_mut(|dd| dd.insert_temp(egui::Id::new(THUMB_RECT), None::<Rect>));
    let Some(doc) = st.session.active() else { return };
    let (doc_id, page) = (doc.id, doc.page);
    let Some(e) = doc.selection.elements.first().and_then(|id| fp_engine::docops::element_on_page(&doc.doc, doc.page, *id)) else { return };
    let docref = std::sync::Arc::clone(&doc.doc);
    let d = Dl::at(r.min + vec2(6.0, 0.0));
    let thumb = d.r(0.0, 3.0, 27.0, 23.0);
    thumb_frame(ui, thumb);
    // wie im Original: das Objekt an seiner Lage im verkleinerten Dokument auf Schachbrett, nicht
    // formatfüllend (PARITAET_A O2)
    if let Some(tex) = crate::panels::thumbs::element_on_page_thumb(ui.ctx(), st, doc_id, &docref, page, &e, 96) {
        let inner = thumb.shrink(1.0);
        let ts = tex.size_vec2();
        let s = (inner.width() / ts.x).min(inner.height() / ts.y);
        let ir = Rect::from_center_size(inner.center(), ts * s);
        ui.painter().image(tex.id(), ir, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        ui.ctx().data_mut(|dd| dd.insert_temp(egui::Id::new(THUMB_RECT), Some(ir)));
    }
    let label = super::obj_quads::type_label(&e);
    let label = match &e.kind {
        fp_model::ElementKind::Instance(i) => match st.session.active().and_then(|d| d.doc.symbol_by_id(i.symbol_id)).map(|(_, s)| s.kind()) {
            Some(fp_model::SymbolKind::Button) => t("pi-type-button"),
            Some(fp_model::SymbolKind::Animation) => t("pi-type-animation"),
            _ => label,
        },
        _ => label,
    };
    let _ = c;
    let row = d.r(0.0, 3.0, 1.0, 10.0);
    fields::label(ui, Rect::from_min_max(pos2(thumb.right() + 5.0, row.top()), pos2(r.right() - 3.0, row.bottom())), &label, true);
    // Rechteck-Grundform ohne Namen: das Feld zeigt wie im Original „Rechteck“ (PAR 10/07)
    let unnamed_rect = || matches!(&e.kind, fp_model::ElementKind::Path(p) if p.primitive.is_some()).then(|| t("tool-rectangle"));
    // maskiertes Einzelelement: Name des Inhalts wie JS `name` (fixg5 Q-02)
    let name = e.common.name.clone().or_else(|| e.masked_single().and_then(|(c, _)| c.common.name.clone())).or_else(unnamed_rect).unwrap_or_default();
    if let Some(n) = fields::text(ui, "pi-objname", d.r(30.0, 14.0, 46.0, 12.0), &name, true) {
        st.run("element.setName", json!({"name": n}));
    }
}

/// Mehrfachauswahl (14102): „n Objekte“.
fn multi_q1(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let theme = Theme::current(ui.ctx());
    let n = st.session.active().map(|d| d.selection.elements.len()).unwrap_or(0);
    let d = Dl::at(r.min + vec2(6.0, 0.0));
    let thumb = d.r(0.0, 3.0, 27.0, 23.0);
    // wie im Original ohne Vorschaubild, „n Objekte“ mittig im Quadranten (PARITAET_A O2)
    ui.ctx().data_mut(|dd| dd.insert_temp(egui::Id::new(THUMB_RECT), None::<Rect>));
    let _ = &theme;
    let text = fp_i18n::tr!("pi-multi-objects", n = n);
    let row = Rect::from_min_max(pos2(r.left() + 3.0, thumb.center().y - 7.0), pos2(r.right() - 3.0, thumb.center().y + 7.0));
    let tw = widgets::text_width(ui, &text, &theme.ui_font()).min(row.width());
    let x = row.center().x - tw / 2.0;
    fields::label(ui, Rect::from_min_max(pos2(x, row.top()), pos2(row.right(), row.bottom())), &text, true);
}

/// Schlüssel: Lage des zuletzt gezeichneten Vorschaubilds im Auswahl-Quadranten (Tests).
const THUMB_RECT: &str = "pi-sel-thumb-rect";

/// Lage des zuletzt gezeichneten Objekt-Vorschaubilds im Inspektor (`None`: keins, z. B. bei
/// Mehrfachauswahl). Für Tests.
pub fn selection_thumb_rect(ctx: &egui::Context) -> Option<Rect> {
    ctx.data(|d| d.get_temp::<Option<Rect>>(egui::Id::new(THUMB_RECT))).flatten()
}

/// Obergrenze der B/H-Felder des Inspektors (Original: 6000).
pub const BOX_MAX: f64 = 6000.0;

/// B/H/X/Y der Auswahl mit „Proportionen beibehalten“ (14007).
fn box_q2(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let theme = Theme::current(ui.ctx());
    // während der Textbearbeitung der Rahmen des Texts im Editor (mit allen ungespeicherten
    // Änderungen) – wie außerhalb der Bearbeitung mit 2-px-Rand (Textgrenzen des Originals)
    let editing = if st.session.tool_is_editing_text() { st.session.text_edit_bounds() } else { None };
    // mit Auswahlrahmen dessen Maße (Original: nach dem Aufziehen B/H/X/Y des Rahmens, PAR 06/02);
    // Eingaben wirken dann nicht auf das Objekt
    let marq = if editing.is_some() { None } else { st.session.active().filter(|d| d.selection.has_pixels()).and_then(|d| d.selection.pixels.as_ref().map(|m| m.tight_bounds().to_rect())).filter(|r| r.width() > 0.0) };
    // Rahmen, den das Werkzeug gerade aufzieht (Zuschneiden, Exportbereich, Rechteck/Ellipse): wie
    // im Original live in B/H/X/Y (PAR 08/17, 08/20, 10/15); Eingaben wirken dann nicht
    let marq = if editing.is_some() { marq } else { st.session.tool_live_rect().map(|r| r.normalized()).or(marq) };
    let b = editing.or(marq).or_else(|| st.session.active().and_then(|d| fp_engine::docops::ids_bounds(&d.doc, d.page, &d.selection.elements)));
    // ohne Auswahl (Rechteck, Ellipse, Auto-Form, Auswahlrahmen, Lasso, Zuschneiden, Exportbereich)
    // sind die Felder wie im Original weiß, leer und bedienbar (PARITAET_A W22); eine Eingabe ohne
    // Auswahl hat kein Ziel und wird verworfen
    let has = b.is_some();
    let en = true;
    let (w, h, x, y) = b.map(|b| (b.width() as f64, b.height() as f64, b.left as f64, b.top as f64)).unwrap_or((0.0, 0.0, 0.0, 0.0));
    let d = Dl::at(r.min + vec2(6.0, 2.0));
    let key = egui::Id::new("pi-box-prop");
    let prop: bool = ui.data(|dd| dd.get_temp(key)).unwrap_or(false);
    let link = d.r(-2.0, 0.0, 7.0, 27.0);
    let lresp = ui.interact(link, key.with("btn"), egui::Sense::click()).on_hover_text(t("pi-box-keep-proportions"));
    crate::ui_kit::icon_button_info(&lresp, en, &t("pi-box-keep-proportions"), Some(prop));
    icons::paint(ui.painter(), link, icons::Icon::LinkProportions, &IconStyle::new(&theme, None, en));
    if prop {
        ui.painter().rect_stroke(link, 1.0, Stroke::new(1.0, theme.colors.focus), StrokeKind::Inside);
    }
    if lresp.clicked() {
        ui.data_mut(|dd| dd.insert_temp(key, !prop));
    }
    fields::label(ui, d.r(7.0, 3.0, 10.0, 8.0), &t("pi-box-w"), en);
    fields::label(ui, d.r(9.0, 16.0, 8.0, 8.0), &t("pi-box-h"), en);
    fields::label(ui, d.r(43.0, 3.0, 8.0, 8.0), &t("pi-box-x"), en);
    fields::label(ui, d.r(43.0, 16.0, 8.0, 8.0), &t("pi-box-y"), en);
    let val = |v: f64| has.then_some(v.round());
    // B/H wie im Original 1 … 6000: Werte unter 1 („-5“, „0“, „x“) werden verworfen, zu große
    // begrenzt und gemeldet (nicht still); PARITAET_A F12
    let limit = |st: &mut AppState, v: f64| -> Option<f64> {
        if v < 1.0 {
            return None;
        }
        let c = v.min(BOX_MAX);
        if c != v {
            st.push(crate::app_state::Action::Status(fp_i18n::tr!("pi-box-limit", max = format!("{BOX_MAX}"), value = format!("{c}"))));
        }
        Some(c)
    };
    if let Some(v) = fields::number_box(ui, "pi-box-w", d.r(17.0, 1.0, 24.0, 12.0), val(w), en).filter(|_| has && marq.is_none()).and_then(|v| limit(st, v)) {
        let p = if prop && w > 0.0 { json!({"width": v, "height": (h * v / w).round()}) } else { json!({"width": v}) };
        st.run("element.setBounds", p);
    }
    if let Some(v) = fields::number_box(ui, "pi-box-h", d.r(17.0, 15.0, 24.0, 12.0), val(h), en).filter(|_| has && marq.is_none()).and_then(|v| limit(st, v)) {
        let p = if prop && h > 0.0 { json!({"height": v, "width": (w * v / h).round()}) } else { json!({"height": v}) };
        st.run("element.setBounds", p);
    }
    // X/Y ohne Grenze im Original (9999 und -5 erlaubt, „x“ → 0); F13
    if let Some(v) = fields::number_box(ui, "pi-box-x", d.r(52.0, 1.0, 24.0, 12.0), val(x), en).filter(|_| has && marq.is_none()).map(|v| fields::clamp_range(v, (-30000.0, 30000.0))) {
        st.run("element.setBounds", json!({"x": v}));
    }
    if let Some(v) = fields::number_box(ui, "pi-box-y", d.r(52.0, 15.0, 24.0, 12.0), val(y), en).filter(|_| has && marq.is_none()).map(|v| fields::clamp_range(v, (-30000.0, 30000.0))) {
        st.run("element.setBounds", json!({"y": v}));
    }
}
