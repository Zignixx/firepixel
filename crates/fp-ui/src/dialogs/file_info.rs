//! Datei ▸ Datei-Info… (D10): Metadaten-Fenster wie das XMP-Fenster des Originals (eigenes Fenster,
//! kein kleiner Meldungsdialog). Gemessen (B-2b, BA-023): Register Beschreibung, IPTC,
//! IPTC Extension, Kameradaten, GPS-Daten, Videodaten, Audiodaten, ▸; Seite „Beschreibung“ mit
//! Dokumenttitel, Autor, Autorentitel, Beschreibung, Bewertung (5 Sterne), Verfasser der Beschreibung,
//! Stichwörter (mit Hinweis), Copyright-Status, Copyright-Informationen, URL für
//! Copyright-Informationen + „Gehe zu URL…“; darunter Erstellt/Geändert/Anwendung/Format; Knöpfe
//! Voreinstellungen · Importieren… ▾ · OK · Abbrechen. Bearbeitbar ist die Seite „Beschreibung“;
//! Voreinstellungen/Importieren sind wie vereinbart noch gesperrt (V). Die nicht umgesetzten
//! Register (IPTC, IPTC Extension, Kameradaten, GPS-Daten, Videodaten, Audiodaten, ▸) werden nach
//! Nutzerentscheidung vom 17.10. ausgeblendet statt gesperrt angezeigt (D10).
//! OK schreibt die Felder über `file.fileInfo` als XMP-Paket ins Dokument (ein Verlaufsschritt,
//! nur wenn sich etwas geändert hat); Abbrechen/Esc ändert nichts.
//! Fenster außen 680 × 620, Titel = Dateiname des Dokuments (DLG-11).

use super::{DialogClose, button_row, dialog_frame, fit_label, push_button, t};
use crate::app_state::AppState;
use crate::inspector::fields;
use crate::theme::Theme;
use egui::{Align2, Id, Pos2, Rect, Sense, Stroke, StrokeKind, vec2};
use serde_json::{Value, json};

/// Copyright-Status in der Reihenfolge der Liste (Wert für `file.fileInfo`).
pub const COPYRIGHT_STATUS: [&str; 3] = ["unknown", "copyrighted", "publicDomain"];

/// Zustand des Datei-Info-Fensters.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FileInfoDialog {
    pub title: String,
    pub author: String,
    pub author_title: String,
    pub description: String,
    /// Bewertung 0–5 Sterne.
    pub rating: u8,
    pub caption_writer: String,
    /// Stichwörter, durch Komma bzw. Semikolon getrennt.
    pub keywords: String,
    /// Index in [`COPYRIGHT_STATUS`].
    pub copyright_status: usize,
    /// Copyright-Informationen.
    pub copyright: String,
    pub copyright_url: String,
    /// Parameter beim Öffnen (OK ohne Änderung legt keinen Schritt an).
    pub initial: Value,
    /// Nur angezeigte Angaben aus `file.fileInfo`.
    pub info: Value,
}

/// Größe des Fensters (Client-Bereich). Gemessen ist das Fenster des Originals außen 680 × 620
/// (PARITAET_B D10); abzüglich Rahmen und Titelleiste wie bei der Bildvorschau
/// (außen 714 × 621 → Client ≈ 698 × 582) ergibt das ≈ 664 × 581.
pub const SIZE: [f32; 2] = [664.0, 581.0];

/// Angezeigte Register. Das Original hat zusätzlich IPTC, IPTC Extension, Kameradaten, GPS-Daten,
/// Videodaten, Audiodaten und ▸ (BA-023); diese sind nicht umgesetzt und nach Nutzerentscheidung
/// vom 17.10. ausgeblendet statt gesperrt (D10).
pub const REGISTERS: [&str; 1] = ["dlg-fileinfo-tab-description"];

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
        let v = crate::ui_commands::execute(&mut st.session, "file.fileInfo", json!({}))
            .ok()?
            .value;
        Some(FileInfoDialog::from_info(v))
    }

    /// Felder aus dem Ergebnis von `file.fileInfo`.
    pub fn from_info(v: Value) -> FileInfoDialog {
        let m = &v["metadata"];
        let s = |k: &str| m[k].as_str().unwrap_or("").to_string();
        let kw: Vec<&str> = m["keywords"]
            .as_array()
            .map(|a| a.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        let status = m["copyrightStatus"]
            .as_str()
            .and_then(|x| COPYRIGHT_STATUS.iter().position(|c| *c == x))
            .unwrap_or(0);
        let mut d = FileInfoDialog {
            title: s("title"),
            author: s("author"),
            author_title: s("authorTitle"),
            description: s("description"),
            rating: m["rating"].as_u64().unwrap_or(0).min(5) as u8,
            caption_writer: s("captionWriter"),
            keywords: kw.join(", "),
            copyright_status: status,
            copyright: s("copyright"),
            copyright_url: s("copyrightUrl"),
            initial: Value::Null,
            info: v.clone(),
        };
        d.initial = d.params();
        d
    }

    /// Wurde etwas geändert?
    pub fn changed(&self) -> bool {
        self.params() != self.initial
    }

    /// Parameter für `file.fileInfo`.
    pub fn params(&self) -> Value {
        json!({
            "title": self.title, "author": self.author, "authorTitle": self.author_title,
            "description": self.description, "rating": self.rating, "captionWriter": self.caption_writer,
            "keywords": self.keywords, "copyrightStatus": COPYRIGHT_STATUS[self.copyright_status.min(2)],
            "copyright": self.copyright, "copyrightUrl": self.copyright_url,
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
    ui.painter()
        .rect_stroke(r, 0.0, Stroke::new(1.0, c.field_border), StrokeKind::Inside);
    let te = if multiline {
        egui::TextEdit::multiline(value)
    } else {
        egui::TextEdit::singleline(value)
    };
    let resp = ui.put(
        r.shrink(2.0),
        te.id(Id::new(("fp-file-info", id)))
            .frame(egui::Frame::NONE)
            .font(theme.ui_font())
            .desired_width(r.width() - 4.0),
    );
    if first {
        super::autofocus_field(ui, &resp, value.chars().count());
    } else {
        super::select_on_tab(ui, &resp, value.chars().count());
    }
}

/// Fünf Sterne der Bewertung; Klick auf einen Stern setzt die Bewertung, Klick auf den aktuellen
/// höchsten Stern nimmt sie zurück (0).
fn stars(ui: &mut egui::Ui, o: Pos2, rating: &mut u8) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    for k in 1..=5u8 {
        let r = Rect::from_min_size(o + vec2((k - 1) as f32 * 18.0, 0.0), vec2(16.0, 16.0));
        let resp = ui.interact(r, Id::new(("fp-file-info-star", k)), Sense::click());
        resp.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Button,
                true,
                format!("{} {k}", t("dlg-fileinfo-rating")),
            )
        });
        if resp.clicked() {
            *rating = if *rating == k { 0 } else { k };
        }
        let on = k <= *rating;
        let col = if on {
            c.text
        } else if resp.hovered() {
            c.focus
        } else {
            c.text_disabled
        };
        // fünfzackiger Stern (gefüllt = vergeben)
        let ctr = r.center();
        let pts: Vec<Pos2> = (0..10)
            .map(|i| {
                let a = std::f32::consts::PI * (i as f32) / 5.0 - std::f32::consts::FRAC_PI_2;
                let rad = if i % 2 == 0 { 7.5 } else { 3.2 };
                ctr + vec2(a.cos(), a.sin()) * rad
            })
            .collect();
        if on {
            // gefüllt aus konvexen Teilen (Fünfeck innen + fünf Zacken)
            let inner: Vec<Pos2> = (0..5).map(|i| pts[2 * i + 1]).collect();
            ui.painter()
                .add(egui::Shape::convex_polygon(inner, col, Stroke::NONE));
            for i in 0..5 {
                let tip = vec![pts[(2 * i + 9) % 10], pts[2 * i], pts[2 * i + 1]];
                ui.painter()
                    .add(egui::Shape::convex_polygon(tip, col, Stroke::NONE));
            }
        }
        ui.painter()
            .add(egui::Shape::closed_line(pts, Stroke::new(1.0, col)));
    }
}

/// Zeigt das Fenster.
pub fn show(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.dialogs.file_info.clone() else {
        return;
    };
    let (ok_l, cancel_l, prefs_l, import_l) = (
        t("btn-ok"),
        t("btn-cancel"),
        t("dlg-fileinfo-preferences"),
        t("dlg-fileinfo-import"),
    );
    let labels = super::tl(&[
        "dlg-fileinfo-doc-title",
        "dlg-fileinfo-author",
        "dlg-fileinfo-author-title",
        "dlg-fileinfo-description",
        "dlg-fileinfo-rating",
        "dlg-fileinfo-caption-writer",
        "dlg-fileinfo-keywords",
        "dlg-fileinfo-copyright-status",
        "dlg-fileinfo-copyright",
        "dlg-fileinfo-copyright-url",
    ]);
    let info_l = super::tl(&[
        "dlg-fileinfo-created",
        "dlg-fileinfo-modified",
        "dlg-fileinfo-application",
        "dlg-fileinfo-format",
    ]);
    let status_l = super::tl(&[
        "dlg-fileinfo-status-unknown",
        "dlg-fileinfo-status-copyrighted",
        "dlg-fileinfo-status-public",
    ]);
    let regs = super::tl(&REGISTERS);
    let goto_l = t("dlg-fileinfo-goto-url");
    let lw = (super::max_text_w(ctx, &super::refs(&labels))
        .max(super::max_text_w(ctx, &super::refs(&info_l)))
        + 8.0)
        .max(130.0)
        .ceil();
    let goto_w = (super::text_w(ctx, &goto_l) + 20.0).max(90.0).ceil();
    let width = SIZE[0].max(16.0 + lw + 320.0 + goto_w + 16.0);
    let mut outcome: Option<bool> = None;
    let title = window_title(&d.info);
    let r = dialog_frame(
        ctx,
        "fp-file-info",
        &title,
        vec2(width, SIZE[1]),
        |ui, body| {
            let o = body.min;
            let theme = Theme::current(ui.ctx());
            let c = theme.colors.clone();
            // Register (nur die umgesetzten; nicht umgesetzte sind ausgeblendet)
            let mut x = 8.0;
            let tab_h = 22.0;
            for name in regs.iter() {
                let w = super::text_w(ui.ctx(), name) + 16.0;
                let r = rr(o, x, 6.0, w, tab_h);
                ui.painter().rect_filled(r, 0.0, c.frame_bg);
                ui.painter().rect_stroke(
                    r,
                    0.0,
                    Stroke::new(1.0, c.field_border),
                    StrokeKind::Inside,
                );
                ui.painter().text(
                    r.center(),
                    Align2::CENTER_CENTER,
                    name,
                    theme.ui_font(),
                    c.text,
                );
                x += w + 2.0;
            }
            let page = rr(o, 8.0, 6.0 + tab_h, body.width() - 16.0, 470.0);
            ui.painter().rect_stroke(
                page,
                0.0,
                Stroke::new(1.0, c.field_border),
                StrokeKind::Inside,
            );
            let fx = 16.0 + lw;
            let fw = body.width() - fx - 16.0;
            let lab = |ui: &mut egui::Ui, k: usize, y: f32| {
                fit_label(ui, rr(o, 16.0, y + 2.0, lw - 4.0, 18.0), &labels[k], true)
            };
            let mut y = 40.0;
            // Dokumenttitel, Autor, Autorentitel
            for (k, (key, val)) in [
                ("title", &mut d.title),
                ("author", &mut d.author),
                ("author-title", &mut d.author_title),
            ]
            .into_iter()
            .enumerate()
            {
                lab(ui, k, y);
                edit(ui, key, rr(o, fx, y, fw, 22.0), val, false, k == 0);
                y += 28.0;
            }
            lab(ui, 3, y);
            edit(
                ui,
                "description",
                rr(o, fx, y, fw, 66.0),
                &mut d.description,
                true,
                false,
            );
            y += 72.0;
            lab(ui, 4, y);
            stars(ui, o + vec2(fx, y + 3.0), &mut d.rating);
            y += 26.0;
            lab(ui, 5, y);
            edit(
                ui,
                "caption-writer",
                rr(o, fx, y, fw, 22.0),
                &mut d.caption_writer,
                false,
                false,
            );
            y += 28.0;
            lab(ui, 6, y);
            edit(
                ui,
                "keywords",
                rr(o, fx, y, fw, 44.0),
                &mut d.keywords,
                true,
                false,
            );
            y += 46.0;
            ui.painter().text(
                o + vec2(fx, y + 8.0),
                Align2::LEFT_CENTER,
                t("dlg-fileinfo-keywords-hint"),
                theme.ui_font(),
                c.text_disabled,
            );
            y += 20.0;
            lab(ui, 7, y);
            let items = super::refs(&status_l);
            let cw = super::combo_fit_w(ui.ctx(), &items, 160.0);
            if let Some(i) = fields::combo(
                ui,
                "fp-file-info-status",
                rr(o, fx, y, cw, 22.0),
                &items,
                Some(d.copyright_status.min(2)),
                true,
            ) {
                d.copyright_status = i;
            }
            y += 28.0;
            lab(ui, 8, y);
            edit(
                ui,
                "copyright",
                rr(o, fx, y, fw, 44.0),
                &mut d.copyright,
                true,
                false,
            );
            y += 50.0;
            lab(ui, 9, y);
            edit(
                ui,
                "copyright-url",
                rr(o, fx, y, fw - goto_w - 6.0, 22.0),
                &mut d.copyright_url,
                false,
                false,
            );
            let url = d.copyright_url.trim().to_string();
            let can_go = url.starts_with("http://") || url.starts_with("https://");
            if push_button(
                ui,
                rr(o, fx + fw - goto_w, y - 0.5, goto_w, 23.0),
                &goto_l,
                false,
                can_go,
            ) {
                let _ = fp_engine::shell::shell_open(std::path::Path::new(&url), None);
            }
            y += 34.0;
            // nur angezeigte Angaben (Fußzeile der Seite)
            let v = &d.info;
            let m = &v["metadata"];
            let txt = |x: &Value| {
                x.as_str().map(str::to_string).unwrap_or_else(|| {
                    if x.is_null() {
                        "—".to_string()
                    } else {
                        x.to_string()
                    }
                })
            };
            let vals_ro = [
                txt(&m["created"]),
                txt(&m["modified"]),
                txt(&m["application"]),
                txt(&m["format"]),
            ];
            for (k, val) in vals_ro.iter().enumerate() {
                let yy = y + k as f32 * 17.0;
                fit_label(ui, rr(o, 16.0, yy, lw - 4.0, 16.0), &info_l[k], true);
                super::put_left(
                    ui,
                    rr(o, fx, yy, fw, 16.0),
                    egui::Label::new(egui::RichText::new(val).font(theme.ui_font()).color(c.text))
                        .truncate(),
                );
            }
            // Voreinstellungen (links) · Importieren… ▾ · OK · Abbrechen
            let by = body.bottom() - 36.0;
            let pw = (super::text_w(ui.ctx(), &prefs_l) + 20.0).max(75.0).ceil();
            let _ = push_button(
                ui,
                Rect::from_min_size(egui::pos2(body.left() + 12.0, by), vec2(pw, 23.0)),
                &prefs_l,
                false,
                false,
            );
            let import_label = format!("{import_l} ▾");
            let b = button_row(
                ui.ctx(),
                body.right() - 12.0,
                by,
                &[&import_label, &ok_l, &cancel_l],
                75.0,
                23.0,
                8.0,
            );
            let _ = push_button(ui, b[0], &import_label, false, false);
            if push_button(ui, b[1], &ok_l, true, true) {
                outcome = Some(true);
            }
            if push_button(ui, b[2], &cancel_l, false, true) {
                outcome = Some(false);
            }
        },
    );
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
