//! „HTML-Einrichtung“ (Vorlage 4766, re\18 §6): drei Registerkarten Allgemein / Tabelle /
//! Dokumentspezifisch mit den Feldern, die der HTML-Export auswertet (Paket E3, nur funktional).
//!
//! Speicherort nach §6.5: Endung, Kommentare, Kleinbuchstaben und leere Zellen in den
//! Voreinstellungen, alles andere im Dokument (`HtmlSettings` der Seite, Befehl `file.htmlSetup`).
//! OK übernimmt alle Karten auf einmal; ohne Änderung wird nichts geschrieben (kein
//! Verlaufsschritt). Abbrechen und Esc verwerfen alles.

use super::{dialog_frame, push_button, refs, t, DialogClose};
use crate::app_state::AppState;
use crate::inspector::fields;
use egui::{pos2, vec2, Rect};
use fp_engine::prefs::PrefValue;
use serde_json::{json, Map, Value};

/// Namensbausteine der Segmente (Liste N ohne Trennstriche) → Enum `NM4i` (re\18 §6.4).
pub const SLICE_TOKENS: [i32; 12] = [0, 6, 7, 4, 5, 2, 3, 1, 15, 16, 17, 18];
/// Statusliste S1 → Enum `NMFi` (Index = Wert).
pub const FRAME1_TOKENS: [i32; 10] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
/// Statusliste S2 → Enum `NMFi` (Index 6 „Benutzerdefiniert“ = 9).
pub const FRAME2_TOKENS: [i32; 7] = [0, 1, 2, 3, 4, 5, 9];
/// „Abstand mit:“ → `shimGeneration` (verschachtelt 3, ohne 0, transparent 1).
pub const SHIM_CODES: [i32; 3] = [3, 0, 1];
/// Endungen der Liste „Erweiterung:“ (editierbar).
const EXTENSIONS: [&str; 11] = [".htm", ".HTM", ".html", ".HTML", ".asp", ".jsp", ".php", ".shtml", ".xhtml", ".cfm", ".XHTML"];

/// Breite der linken Spalte (Feld „HTML-Stil:“) und Abstand zur rechten Spalte (Karte „Allgemein“).
const COL1_W: f32 = 190.0;
const COL_GAP: f32 = 18.0;

/// Zustand des Dialogs (eine Arbeitskopie aller drei Karten).
#[derive(Clone, Debug, PartialEq)]
pub struct HtmlSetupDialog {
    pub tab: usize,
    // Allgemein
    pub xhtml: bool,
    pub extension: String,
    pub comments: bool,
    pub lowercase: bool,
    pub css_popups: bool,
    /// „CSS in externe Datei schreiben“ (`externalCSS`, grau solange `css_popups` aus ist).
    pub external_css: bool,
    pub background_url: String,
    pub background_repeat: usize,
    /// Liste „Anhang:“: 0 fixed, 1 scroll (= Wert von `CBAi`).
    pub attachment: usize,
    pub page_alignment: usize,
    // Tabelle
    pub shims: usize,
    pub cell_color: String,
    pub use_canvas: bool,
    pub contents: usize,
    // Dokumentspezifisch
    pub slice_naming: [usize; 6],
    pub frame1: usize,
    pub frame2: usize,
    pub state_names: [String; 4],
    pub alt: String,
    pub include_unsliced: bool,
    pub utf8: bool,
    /// Ausgangswerte (für „ohne Änderung nichts schreiben“).
    initial: Option<Box<HtmlSetupDialog>>,
}

fn idx_of(list: &[i32], v: i32, fallback: usize) -> usize {
    list.iter().position(|x| *x == v).unwrap_or(fallback)
}

impl HtmlSetupDialog {
    /// Werte aus Dokument (aktuelle Seite) und Voreinstellungen.
    pub fn new(st: &AppState) -> Option<HtmlSetupDialog> {
        let d = st.session.active()?;
        let h = d.doc.pages.get(d.page).and_then(|p| p.root.export.as_ref()).and_then(|e| e.html.as_deref()).cloned().unwrap_or_else(fp_model::HtmlSettings::cs6_default);
        let prefs = &st.session.prefs;
        let hp = fp_engine::commands::export::html_prefs(prefs);
        let names = h.slice_naming.clone().unwrap_or_else(|| vec![6, 15, 1, 0, 0, 0]);
        let mut slice_naming = [0usize; 6];
        for (i, n) in slice_naming.iter_mut().enumerate() {
            *n = idx_of(&SLICE_TOKENS, names.get(i).copied().unwrap_or(0), 0);
        }
        let fr = h.frame_naming.clone().unwrap_or_else(|| vec![5, 1]);
        let ext = fp_export::doc_export::html_extension(Some(&h), &hp);
        let mut dlg = HtmlSetupDialog {
            tab: 0,
            xhtml: h.xhtml.unwrap_or(1) != 0,
            extension: format!(".{ext}"),
            comments: hp.comments,
            lowercase: hp.lowercase,
            css_popups: h.css_popups.unwrap_or(true),
            external_css: h.external_css.unwrap_or(1) != 0,
            background_url: h.background_url.clone().unwrap_or_default(),
            background_repeat: h.background_repeat.unwrap_or(0).clamp(0, 3) as usize,
            attachment: if h.background_attachment == Some(0) { 0 } else { 1 },
            page_alignment: h.page_alignment.unwrap_or(0).clamp(0, 2) as usize,
            shims: idx_of(&SHIM_CODES, h.shim_generation.unwrap_or(1), 2),
            cell_color: format!("#{:06x}", hp.empty_cell_color),
            use_canvas: hp.empty_cell_canvas,
            contents: hp.empty_cell_contents.clamp(0, 2) as usize,
            slice_naming,
            frame1: idx_of(&FRAME1_TOKENS, fr.first().copied().unwrap_or(5), 5),
            frame2: idx_of(&FRAME2_TOKENS, fr.get(1).copied().unwrap_or(1), 1),
            state_names: h.state_names.clone().unwrap_or_else(|| [String::new(), "_o".into(), "_d".into(), "_od".into()]),
            alt: h.default_alt.clone().unwrap_or_default(),
            include_unsliced: !h.discard_unspecified_slices.unwrap_or(false),
            utf8: h.encoding.unwrap_or(1) == 1,
            initial: None,
        };
        dlg.initial = Some(Box::new(dlg.clone()));
        Some(dlg)
    }

    fn base(&self) -> HtmlSetupDialog {
        self.initial.as_deref().cloned().unwrap_or_else(|| self.clone())
    }

    /// Gleiche Werte wie beim Öffnen (die Karte zählt nicht)?
    pub fn unchanged(&self) -> bool {
        let mut a = self.clone();
        a.tab = 0;
        a.initial = None;
        let mut b = self.base();
        b.tab = 0;
        b.initial = None;
        a == b
    }

    /// Endung mit Punkt (leer ⇒ „.htm“).
    pub fn ext(&self) -> String {
        let e = self.extension.trim().trim_start_matches('.');
        if e.is_empty() { ".htm".into() } else { format!(".{e}") }
    }

    /// Beispiel-Dateiname (Dokumentspezifisch, „Beispiel:“): Bausteine mit Zeile 1, Spalte 2,
    /// Status 2, Endung „.gif“.
    pub fn example(&self, doc_name: &str) -> String {
        let tokens: Vec<i32> = self.slice_naming.iter().map(|i| SLICE_TOKENS[*i]).collect();
        let date = fp_export::slices::NameDate { year: 2026, month: 1, day: 1 };
        let name = fp_export::slices::auto_name(&tokens, 1, 2, doc_name, 2, 2, date);
        let frames = [FRAME1_TOKENS[self.frame1], FRAME2_TOKENS[self.frame2]];
        format!("{name}{}.gif", fp_export::slices::frame_suffix(&frames, 2, &self.state_names))
    }

    /// Felder des Dokuments, die sich gegenüber dem Öffnen geändert haben (für `file.htmlSetup`).
    pub fn doc_changes(&self) -> Map<String, Value> {
        let b = self.base();
        let mut m = Map::new();
        let mut put = |k: &str, changed: bool, v: Value| {
            if changed {
                m.insert(k.into(), v);
            }
        };
        put("xhtml", self.xhtml != b.xhtml, json!(i32::from(self.xhtml)));
        put("css_popups", self.css_popups != b.css_popups, json!(self.css_popups));
        put("external_css", self.external_css != b.external_css, json!(i32::from(self.external_css)));
        put("background_url", self.background_url != b.background_url, json!(self.background_url.trim()));
        put("background_repeat", self.background_repeat != b.background_repeat, json!(self.background_repeat));
        put("background_attachment", self.attachment != b.attachment, json!(self.attachment));
        put("page_alignment", self.page_alignment != b.page_alignment, json!(self.page_alignment));
        put("shim_generation", self.shims != b.shims, json!(SHIM_CODES[self.shims]));
        put("slice_naming", self.slice_naming != b.slice_naming, json!(self.slice_naming.iter().map(|i| SLICE_TOKENS[*i]).collect::<Vec<_>>()));
        put("frame_naming", self.frame1 != b.frame1 || self.frame2 != b.frame2, json!([FRAME1_TOKENS[self.frame1], FRAME2_TOKENS[self.frame2]]));
        put("state_names", self.state_names != b.state_names, json!(self.state_names));
        put("default_alt", self.alt != b.alt, json!(self.alt));
        put("discard_unspecified_slices", self.include_unsliced != b.include_unsliced, json!(!self.include_unsliced));
        put("encoding", self.utf8 != b.utf8, json!(if self.utf8 { 1 } else { 0 }));
        m
    }

    /// Statusnamen bearbeitbar (eine Statusliste auf „Benutzerdefiniert“, re\18 §6.4)?
    pub fn custom_names(&self) -> bool {
        FRAME1_TOKENS[self.frame1] == 9 || FRAME2_TOKENS[self.frame2] == 9
    }
}

/// Öffnet den Dialog (Datei ▸ HTML-Einrichtung… bzw. „Optionen…“ der HTML-Arten im Export).
pub fn open(st: &mut AppState) -> bool {
    st.dialogs.html_setup = HtmlSetupDialog::new(st);
    true
}

/// OK: Voreinstellungen und Dokument schreiben (nur Geändertes; ohne Änderung nichts).
pub fn apply(st: &mut AppState, d: &HtmlSetupDialog) {
    if d.unchanged() {
        return;
    }
    let b = d.base();
    let prefs = &mut st.session.prefs;
    if d.ext() != b.ext() {
        prefs.set("HTMLExtension", PrefValue::Str(d.ext()));
    }
    if d.comments != b.comments {
        prefs.set("HTMLCommentsOn", PrefValue::Bool(d.comments));
    }
    if d.lowercase != b.lowercase {
        prefs.set("ForceLowercaseFilenames", PrefValue::Bool(d.lowercase));
    }
    if d.cell_color.trim() != b.cell_color.trim() && let Some(c) = parse_hex(&d.cell_color) {
        prefs.set("HTMLEmptyCellColor", PrefValue::Str(format!("#{c:06x}")));
    }
    if d.use_canvas != b.use_canvas {
        prefs.set("HTMLEmptyCellUsesCanvasColor", PrefValue::Bool(d.use_canvas));
    }
    if d.contents != b.contents {
        prefs.set("HTMLEmptyCellContents", PrefValue::Int(d.contents as i64 + 1));
    }
    let mut changes = d.doc_changes();
    // eine eigene Endung im Dokument ginge der Voreinstellung vor: mitziehen
    if d.ext() != b.ext() {
        let doc_ext = st.session.active().and_then(|x| x.doc.pages.get(x.page)).and_then(|p| p.root.export.as_ref()).and_then(|e| e.html.as_ref()).and_then(|h| h.file_extension.clone()).filter(|e| !e.trim().is_empty());
        if doc_ext.is_some() {
            changes.insert("file_extension".into(), json!(d.ext()));
        }
        if let Some(x) = st.dialogs.export.as_mut() {
            x.html_ext = d.ext().trim_start_matches('.').to_string();
            if !x.file_name.trim().is_empty() {
                x.file_name = super::export::with_ext(&x.file_name, &x.ext());
            }
        }
    }
    if !changes.is_empty()
        && let Err(e) = crate::ui_commands::execute(&mut st.session, "file.htmlSetup", json!({ "settings": Value::Object(changes) }))
    {
        st.dialogs.message = Some((t("dlg-md-html-setup"), e.to_string()));
    }
}

fn parse_hex(s: &str) -> Option<u32> {
    let h = s.trim().trim_start_matches('#');
    (h.len() == 6).then(|| u32::from_str_radix(h, 16).ok()).flatten()
}

/// Zeigt den Dialog.
pub fn show(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.dialogs.html_setup.take() else { return };
    let doc_name = st.session.active().map(|x| super::export::file_stem(&x.path.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_default())).unwrap_or_default();
    let ans = show_dialog(ctx, &mut d, &doc_name);
    match ans {
        Some(true) => apply(st, &d),
        Some(false) => {}
        None => st.dialogs.html_setup = Some(d),
    }
}

/// Zeichnet den Dialog; `Some(true)` OK, `Some(false)` Abbrechen/Esc.
pub fn show_dialog(ctx: &egui::Context, d: &mut HtmlSetupDialog, doc_name: &str) -> Option<bool> {
    let l = |k: &str| t(k);
    let tabs = [l("dlg-hs-tab-general"), l("dlg-hs-tab-table"), l("dlg-hs-tab-document")];
    let g_labels = [l("dlg-hs-style"), l("dlg-hs-extension"), l("dlg-hs-bg-image"), l("dlg-hs-repeat"), l("dlg-hs-attachment"), l("dlg-hs-alignment")];
    let t_labels = [l("dlg-hs-space-with"), l("dlg-hs-cell-color"), l("dlg-hs-contents")];
    let d_labels = [l("dlg-hs-slice-names"), l("dlg-hs-frame-names"), l("dlg-hs-alt")];
    let all: Vec<&str> = g_labels.iter().chain(t_labels.iter()).chain(d_labels.iter()).map(|s| s.as_str()).collect();
    let lw = super::max_text_w(ctx, &all).max(110.0).ceil();
    // rechte Spalte der Karte „Allgemein“: Beschriftungen und Kontrollkästchen
    let lw2 = super::max_text_w(ctx, &[g_labels[1].as_str(), g_labels[4].as_str()]).ceil();
    let cw2 = (super::max_text_w(ctx, &[l("dlg-hs-css-popups").as_str(), l("dlg-hs-external-css").as_str()]) + 26.0).max(lw2 + 8.0 + 100.0).ceil();
    let slice_items: Vec<String> = ["none", "doc-name", "slice", "num", "num2", "upper", "lower", "row-col", "underscore", "period", "space", "hyphen"].iter().map(|k| l(&format!("dlg-hs-n-{k}"))).collect();
    let frame1_items: Vec<String> = ["none", "frame-s", "frame-n", "rollover", "abbrev", "underscore", "period", "space", "hyphen", "custom"].iter().map(|k| l(&format!("dlg-hs-f-{k}"))).collect();
    let frame2_items: Vec<String> = ["none", "frame-s", "frame-n", "rollover", "abbrev", "underscore", "custom"].iter().map(|k| l(&format!("dlg-hs-f-{k}"))).collect();
    let nw = super::combo_fit_w(ctx, &refs(&slice_items), 150.0).max(super::combo_fit_w(ctx, &refs(&frame1_items), 150.0));
    let width = (32.0 + (nw + 8.0) * 3.0).max(32.0 + lw + 8.0 + 330.0).max(32.0 + lw + 8.0 + COL1_W + COL_GAP + cw2).max(560.0).ceil();
    let mut ans = None;
    let c = dialog_frame(ctx, "fp-html-setup", &t("dlg-md-html-setup"), vec2(width, 430.0), |ui, body| {
        // Startfokus auf OK wie im Original (Enter = OK)
        super::autofocus_default_button(ui);
        // Registerreiter
        let mut x = body.left() + 12.0;
        for (i, name) in tabs.iter().enumerate() {
            let w = super::button_w(ui.ctx(), name, 70.0);
            if ui.put(Rect::from_min_size(pos2(x, body.top() + 10.0), vec2(w, 23.0)), egui::Button::selectable(d.tab == i, name.as_str())).clicked() {
                d.tab = i;
            }
            x += w + 4.0;
        }
        let o = body.min + vec2(16.0, 48.0);
        let label = |y: f32| Rect::from_min_size(o + vec2(0.0, y + 1.0), vec2(lw, 20.0));
        let field = |y: f32, w: f32| Rect::from_min_size(o + vec2(lw + 8.0, y), vec2(w, 22.0));
        let fx = o.x + lw + 8.0;
        match d.tab {
            0 => {
                // zweispaltig wie das Original: links Stil + Kommentare/Kleinbuchstaben,
                // rechts Endung + die beiden CSS-Schalter
                let style_items = [l("dlg-hs-style-html"), l("dlg-hs-style-xhtml")];
                super::fit_label(ui, label(0.0), &g_labels[0], true);
                if let Some(i) = fields::combo(ui, "hs-style", field(0.0, COL1_W), &refs(&style_items), Some(usize::from(d.xhtml)), true) {
                    d.xhtml = i == 1;
                }
                let x2 = fx + COL1_W + COL_GAP;
                let label2 = |y: f32| Rect::from_min_size(pos2(x2, o.y + y + 1.0), vec2(lw2, 20.0));
                let field2 = |y: f32, w: f32| Rect::from_min_size(pos2(x2 + lw2 + 8.0, o.y + y), vec2(w, 22.0));
                super::fit_label(ui, label2(0.0), &g_labels[1], true);
                let exts: Vec<String> = EXTENSIONS.iter().map(|s| s.to_string()).collect();
                if let Some(v) = fields::combo_edit(ui, "hs-ext", field2(0.0, 100.0), &d.extension, &exts, true) {
                    d.extension = v;
                }
                let w1 = COL1_W + COL_GAP - 8.0;
                let w2 = body.right() - 16.0 - x2;
                let cb = |ui: &mut egui::Ui, id: &str, x: f32, w: f32, y: f32, text: &str, v: &mut bool, on: bool| {
                    if let Some(b) = fields::checkbox(ui, id, Rect::from_min_size(pos2(x, o.y + y), vec2(w, 18.0)), text, *v, on) {
                        *v = b;
                    }
                };
                cb(ui, "hs-comments", fx, w1, 30.0, &l("dlg-hs-comments"), &mut d.comments, true);
                cb(ui, "hs-lower", fx, w1, 52.0, &l("dlg-hs-lowercase"), &mut d.lowercase, true);
                cb(ui, "hs-css-popups", x2, w2, 30.0, &l("dlg-hs-css-popups"), &mut d.css_popups, true);
                // grau, solange „CSS für Popupmenüs“ aus ist; das Häkchen bleibt stehen (re8 §6.2)
                let css_on = d.css_popups;
                cb(ui, "hs-external-css", x2, w2, 52.0, &l("dlg-hs-external-css"), &mut d.external_css, css_on);
                // Dokumenteigenschaften (CSS der Seite)
                super::fit_label(ui, Rect::from_min_size(o + vec2(0.0, 88.0), vec2(body.width() - 32.0, 20.0)), &l("dlg-hs-doc-props"), true);
                super::fit_label(ui, label(114.0), &g_labels[2], true);
                if let Some(v) = fields::text(ui, "hs-bg-url", field(114.0, (body.right() - 16.0 - fx).min(COL1_W + COL_GAP + lw2 + 8.0 + 100.0)), &d.background_url, true) {
                    d.background_url = v;
                }
                super::fit_label(ui, label(142.0), &g_labels[3], true);
                if let Some(i) = fields::combo(ui, "hs-repeat", field(142.0, 110.0), &["no-repeat", "repeat", "repeat-x", "repeat-y"], Some(d.background_repeat), true) {
                    d.background_repeat = i;
                }
                super::fit_label(ui, label2(142.0), &g_labels[4], true);
                if let Some(i) = fields::combo(ui, "hs-attach", field2(142.0, 100.0), &["fixed", "scroll"], Some(d.attachment), true) {
                    d.attachment = i;
                }
                super::fit_label(ui, label(176.0), &g_labels[5], true);
                if let Some(i) = fields::combo(ui, "hs-align", field(176.0, 110.0), &["left", "center", "right"], Some(d.page_alignment), true) {
                    d.page_alignment = i;
                }
            }
            1 => {
                let shim_items = [l("dlg-hs-shim-nested"), l("dlg-hs-shim-none"), l("dlg-hs-shim-spacer")];
                super::fit_label(ui, label(0.0), &t_labels[0], true);
                if let Some(i) = fields::combo(ui, "hs-shims", field(0.0, super::combo_fit_w(ui.ctx(), &refs(&shim_items), 250.0)), &refs(&shim_items), Some(d.shims), true) {
                    d.shims = i;
                }
                super::fit_label(ui, Rect::from_min_size(o + vec2(0.0, 40.0), vec2(body.width() - 32.0, 20.0)), &l("dlg-hs-empty-cells"), true);
                let color_on = !d.use_canvas;
                super::fit_label(ui, label(66.0), &t_labels[1], color_on);
                let sw = Rect::from_min_size(pos2(fx, o.y + 67.0), vec2(20.0, 18.0));
                fields::swatch(ui, "hs-cell-swatch", sw, parse_hex(&d.cell_color).map(|c| fp_color::Argb(0xFF00_0000 | c)), color_on);
                if let Some(v) = fields::text(ui, "hs-cell-color", Rect::from_min_size(pos2(sw.right() + 6.0, o.y + 66.0), vec2(80.0, 22.0)), &d.cell_color, color_on) {
                    d.cell_color = v;
                }
                if let Some(b) = fields::checkbox(ui, "hs-canvas", Rect::from_min_size(pos2(sw.right() + 96.0, o.y + 68.0), vec2(body.right() - 16.0 - sw.right() - 96.0, 18.0)), &l("dlg-hs-use-canvas"), d.use_canvas, true) {
                    d.use_canvas = b;
                }
                let content_items = [l("dlg-hs-content-none"), l("dlg-hs-content-spacer"), l("dlg-hs-content-nbsp")];
                super::fit_label(ui, label(98.0), &t_labels[2], true);
                if let Some(i) = fields::combo(ui, "hs-contents", field(98.0, super::combo_fit_w(ui.ctx(), &refs(&content_items), 170.0)), &refs(&content_items), Some(d.contents), true) {
                    d.contents = i;
                }
            }
            _ => {
                super::fit_label(ui, Rect::from_min_size(o, vec2(body.width() - 32.0, 20.0)), &d_labels[0], true);
                for k in 0..6 {
                    let r = Rect::from_min_size(o + vec2((k % 3) as f32 * (nw + 8.0), 24.0 + (k / 3) as f32 * 28.0), vec2(nw, 22.0));
                    if let Some(i) = fields::combo(ui, ("hs-name", k), r, &refs(&slice_items), Some(d.slice_naming[k]), true) {
                        d.slice_naming[k] = i;
                    }
                }
                super::fit_label(ui, label(92.0), &d_labels[1], true);
                if let Some(i) = fields::combo(ui, "hs-frame1", field(92.0, nw), &refs(&frame1_items), Some(d.frame1), true) {
                    d.frame1 = i;
                }
                if let Some(i) = fields::combo(ui, "hs-frame2", Rect::from_min_size(pos2(fx + nw + 8.0, o.y + 92.0), vec2(nw, 22.0)), &refs(&frame2_items), Some(d.frame2), true) {
                    d.frame2 = i;
                }
                // eigene Statusnamen (Up, Over, Down, OverDown), nur bei „Benutzerdefiniert“
                let on = d.custom_names();
                let sw = ((body.width() - 32.0 - 3.0 * 8.0) / 4.0).floor();
                for k in 0..4 {
                    let r = Rect::from_min_size(o + vec2(k as f32 * (sw + 8.0), 122.0), vec2(sw, 22.0));
                    if let Some(v) = fields::text(ui, ("hs-state", k), r, &d.state_names[k], on) {
                        d.state_names[k] = v;
                    }
                }
                super::fit_label(ui, Rect::from_min_size(o + vec2(0.0, 150.0), vec2(body.width() - 32.0, 20.0)), &fp_i18n::tr!("dlg-hs-example", name = d.example(doc_name)), true);
                super::fit_label(ui, label(180.0), &d_labels[2], true);
                if let Some(v) = fields::text(ui, "hs-alt", field(180.0, (body.width() - 32.0 - lw - 8.0).min(374.0)), &d.alt, true) {
                    d.alt = v;
                }
                if let Some(b) = fields::checkbox(ui, "hs-unsliced", Rect::from_min_size(o + vec2(0.0, 216.0), vec2(body.width() - 32.0, 18.0)), &l("dlg-hs-include-unsliced"), d.include_unsliced, true) {
                    d.include_unsliced = b;
                }
                if let Some(b) = fields::checkbox(ui, "hs-utf8", Rect::from_min_size(o + vec2(0.0, 240.0), vec2(body.width() - 32.0, 18.0)), &l("dlg-hs-utf8"), d.utf8, true) {
                    d.utf8 = b;
                }
            }
        }
        let (ok_l, cancel_l) = (t("btn-ok"), t("btn-cancel"));
        let rects = super::button_row(ui.ctx(), body.right() - 12.0, body.bottom() - 35.0, &[&ok_l, &cancel_l], 75.0, 23.0, 8.0);
        if push_button(ui, rects[0], &ok_l, true, true) {
            ans = Some(true);
        }
        if push_button(ui, rects[1], &cancel_l, false, true) {
            ans = Some(false);
        }
    });
    if c == DialogClose::Cancelled { Some(false) } else { ans }
}
