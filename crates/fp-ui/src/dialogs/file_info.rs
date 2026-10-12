//! Datei ▸ Datei-Info… (D10): Metadaten-Fenster wie das XMP-Fenster des Originals (eigenes Fenster,
//! kein kleiner Meldungsdialog). Bearbeitbar sind Dokumenttitel, Autor, Beschreibung,
//! Schlüsselwörter und Copyright-Vermerk; darunter stehen Datei, Seiten, Symbole, Formatversion,
//! Erstellt/Geändert und Anwendung. OK schreibt die Felder über `file.fileInfo` als XMP-Paket ins
//! Dokument (ein Verlaufsschritt, nur wenn sich etwas geändert hat); Abbrechen/Esc ändert nichts.
//! Gemessen und umgesetzt (DLG-11): Fenster außen 680 × 620, Titel = Dateiname des Dokuments.
//! Register und Felder je Register des Originals sind nicht gemessen (docs\berichte\dlg11.md).

use super::{button_row, dialog_frame, fit_label, group_box, push_button, t, DialogClose};
use crate::app_state::AppState;
use crate::theme::Theme;
use egui::{vec2, Id, Pos2, Rect, Stroke, StrokeKind};
use serde_json::{json, Value};

/// Zustand des Datei-Info-Fensters.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FileInfoDialog {
    pub title: String,
    pub author: String,
    pub description: String,
    /// Schlüsselwörter, durch Komma bzw. Semikolon getrennt.
    pub keywords: String,
    pub copyright: String,
    /// Werte beim Öffnen (Titel, Autor, Beschreibung, Schlüsselwörter, Copyright).
    pub initial: [String; 5],
    /// Nur angezeigte Angaben aus `file.fileInfo`.
    pub info: Value,
}

/// Größe des Fensters (Client-Bereich). Gemessen ist das Fenster des Originals außen 680 × 620
/// (PARITAET_B D10); abzüglich Rahmen und Titelleiste wie bei der Bildvorschau
/// (außen 714 × 621 → Client ≈ 698 × 582) ergibt das ≈ 664 × 581.
pub const SIZE: [f32; 2] = [664.0, 581.0];

/// Fenstertitel: wie im Original der Dateiname des Dokuments (gemessen „Unbenannt-77.fw.png“),
/// ohne Namen der allgemeine Titel „Datei-Info“.
pub fn window_title(info: &Value) -> String {
    match info["title"].as_str().map(str::trim) {
        Some(n) if !n.is_empty() => n.to_string(),
        _ => t("dlg-fileinfo-window-title"),
    }
}

impl FileInfoDialog {
    /// Liest die Angaben des aktiven Dokuments.
    pub fn new(st: &mut AppState) -> Option<FileInfoDialog> {
        let v = crate::ui_commands::execute(&mut st.session, "file.fileInfo", json!({})).ok()?.value;
        let m = &v["metadata"];
        let s = |k: &str| m[k].as_str().unwrap_or("").to_string();
        let kw: Vec<&str> = m["keywords"].as_array().map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
        let mut d = FileInfoDialog { title: s("title"), author: s("author"), description: s("description"), keywords: kw.join(", "), copyright: s("copyright"), initial: Default::default(), info: v.clone() };
        d.initial = d.values();
        Some(d)
    }

    fn values(&self) -> [String; 5] {
        [self.title.clone(), self.author.clone(), self.description.clone(), self.keywords.clone(), self.copyright.clone()]
    }

    /// Wurde etwas geändert?
    pub fn changed(&self) -> bool {
        self.values() != self.initial
    }

    /// Parameter für `file.fileInfo`.
    pub fn params(&self) -> Value {
        json!({
            "title": self.title, "author": self.author, "description": self.description,
            "keywords": self.keywords, "copyright": self.copyright,
        })
    }
}

/// Öffnet das Fenster (`false` ohne Dokument).
pub fn open(st: &mut AppState) -> bool {
    match FileInfoDialog::new(st) {
        Some(d) => {
            st.dialogs.file_info = Some(d);
            true
        }
        None => false,
    }
}

fn rr(o: Pos2, x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_min_size(o + vec2(x, y), vec2(w, h))
}

/// Eingabefeld mit Rahmen wie die übrigen Dialogfelder.
fn edit(ui: &mut egui::Ui, id: &str, r: Rect, value: &mut String, multiline: bool, first: bool) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    ui.painter().rect_filled(r, 0.0, c.field_bg);
    ui.painter().rect_stroke(r, 0.0, Stroke::new(1.0, c.field_border), StrokeKind::Inside);
    let te = if multiline { egui::TextEdit::multiline(value) } else { egui::TextEdit::singleline(value) };
    let resp = ui.put(r.shrink(2.0), te.id(Id::new(("fp-file-info", id))).frame(egui::Frame::NONE).font(theme.ui_font()).desired_width(r.width() - 4.0));
    if first {
        super::autofocus_field(ui, &resp, value.chars().count());
    } else {
        super::select_on_tab(ui, &resp, value.chars().count());
    }
}

/// Zeigt das Fenster.
pub fn show(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.dialogs.file_info.clone() else { return };
    let (ok_l, cancel_l) = (t("btn-ok"), t("btn-cancel"));
    let labels = super::tl(&["dlg-fileinfo-doc-title", "dlg-fileinfo-author", "dlg-fileinfo-description", "dlg-fileinfo-keywords", "dlg-fileinfo-copyright"]);
    let info_l = super::tl(&["dlg-fileinfo-path", "dlg-fileinfo-pages", "dlg-fileinfo-symbols", "dlg-fileinfo-version", "dlg-fileinfo-created", "dlg-fileinfo-modified", "dlg-fileinfo-application"]);
    let lw = (super::max_text_w(ctx, &super::refs(&labels)).max(super::max_text_w(ctx, &super::refs(&info_l))) + 8.0).max(130.0).ceil();
    let width = SIZE[0].max(16.0 + lw + 320.0 + 16.0);
    let mut outcome: Option<bool> = None;
    let title = window_title(&d.info);
    let r = dialog_frame(ctx, "fp-file-info", &title, vec2(width, SIZE[1]), |ui, body| {
        let o = body.min;
        let fx = 16.0 + lw;
        let fw = body.width() - fx - 16.0;
        group_box(ui, rr(o, 8.0, 8.0, body.width() - 16.0, 336.0), &t("dlg-fileinfo-group-description"));
        let rows: [(f32, f32); 5] = [(28.0, 22.0), (56.0, 22.0), (84.0, 130.0), (222.0, 50.0), (280.0, 50.0)];
        let vals = [&mut d.title, &mut d.author, &mut d.description, &mut d.keywords, &mut d.copyright];
        let keys = ["title", "author", "description", "keywords", "copyright"];
        for (k, (y, h)) in rows.iter().enumerate() {
            fit_label(ui, rr(o, 16.0, *y + 2.0, lw - 4.0, 18.0), &labels[k], true);
            edit(ui, keys[k], rr(o, fx, *y, fw, *h), vals[k], *h > 22.0, k == 0);
        }
        // nur angezeigte Angaben
        group_box(ui, rr(o, 8.0, 354.0, body.width() - 16.0, 146.0), &t("dlg-fileinfo-group-file"));
        let v = &d.info;
        let m = &v["metadata"];
        let txt = |x: &Value| x.as_str().map(str::to_string).unwrap_or_else(|| if x.is_null() { "—".to_string() } else { x.to_string() });
        let path = v["path"].as_str().map(str::to_string).unwrap_or_else(|| t("dlg-md-not-saved"));
        let vals_ro = [path, txt(&v["pages"]), txt(&v["symbols"]), txt(&v["formatVersion"]), txt(&m["created"]), txt(&m["modified"]), txt(&m["application"])];
        let theme = Theme::current(ui.ctx());
        for (k, val) in vals_ro.iter().enumerate() {
            let y = 374.0 + k as f32 * 17.0;
            fit_label(ui, rr(o, 16.0, y, lw - 4.0, 16.0), &info_l[k], true);
            let vr = rr(o, fx, y, fw, 16.0);
            super::put_left(ui, vr, egui::Label::new(egui::RichText::new(val).font(theme.ui_font()).color(theme.colors.text)).truncate());
        }
        let b = button_row(ui.ctx(), body.right() - 12.0, body.bottom() - 36.0, &[&ok_l, &cancel_l], 75.0, 23.0, 8.0);
        if push_button(ui, b[0], &ok_l, true, true) {
            outcome = Some(true);
        }
        if push_button(ui, b[1], &cancel_l, false, true) {
            outcome = Some(false);
        }
    });
    let outcome = outcome.or((r == DialogClose::Cancelled).then_some(false));
    match outcome {
        Some(true) => {
            st.dialogs.file_info = None;
            if d.changed() {
                st.run("file.fileInfo", d.params());
            }
        }
        Some(false) => st.dialogs.file_info = None,
        None => st.dialogs.file_info = Some(d),
    }
}
