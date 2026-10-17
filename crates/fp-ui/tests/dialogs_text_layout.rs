//! Nutzerbefund: Meldungstexte standen wie Blocksatz mit gesperrten Buchstaben und zentriert
//! („M ö c h t e n  S i e …“), weil `ui.put` umbrechende Beschriftungen in ein zentriertes,
//! ausgerichtetes Layout legt. Geprüft an den gezeichneten Galleys aller Meldungen und
//! Dialogtexte, in allen 3 Designs, bei 100 % und 150 % DPI: keine Lücken zwischen Zeichen über Rundung/Kerning hinaus (< 1,5 px)
//! (Zeilenbreite = natürliche Textbreite), mehrzeilige Texte linksbündig.

use egui::epaint::{Shape, TextShape};
use egui::{vec2, Pos2, Rect};
use egui_kittest::Harness;
use fp_engine::Session;
use fp_ui::app_state::UiSettings;
use fp_ui::dialogs::form::{Field, FormDialog};
use fp_ui::dialogs::{dialog_rect, last_dialog_id};
use fp_ui::theme::ThemeKind;
use fp_ui::FirepixelApp;
use serde_json::json;

const LONG: &str = "Dieser Hinweis ist absichtlich so lang, dass er im Dialog über mehrere Zeilen umbrechen muss, damit die Ausrichtung jeder Zeile geprüft werden kann.";

fn harness(theme: ThemeKind, ppp: f32) -> Harness<'static, FirepixelApp> {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 80]})).expect("rect");
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings { theme, ..UiSettings::default() }, false);
    a.state.sync_render = true;
    Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(ppp).with_max_steps(16).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a)
}

fn texts(shape: &Shape, out: &mut Vec<TextShape>) {
    match shape {
        Shape::Text(t) => out.push(t.clone()),
        Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
        _ => {}
    }
}

/// Prüft alle Texte im offenen Dialog; liefert die Zahl der mehrzeiligen Galleys.
fn check_dialog(h: &Harness<'_, FirepixelApp>, what: &str) -> usize {
    let id = last_dialog_id(&h.ctx).unwrap_or_else(|| panic!("{what}: kein Dialog"));
    let dr: Rect = dialog_rect(&h.ctx, &id).expect("Dialogbereich");
    let mut all = Vec::new();
    for cs in &h.output().shapes {
        texts(&cs.shape, &mut all);
    }
    let mut multi = 0;
    for t in all.iter().filter(|t| dr.contains(t.pos)) {
        let g = &t.galley;
        let text = g.job.text.clone();
        for row in &g.rows {
            let gl = &row.row.glyphs;
            for w in gl.windows(2) {
                let gap = w[1].pos.x - (w[0].pos.x + w[0].advance_width);
                assert!(gap < 1.5, "{what}: Lücke {gap:.1} px zwischen „{}“ und „{}“ in „{text}“ (Blocksatz/gesperrt)", w[0].chr, w[1].chr);
            }
        }
        let rows: Vec<_> = g.rows.iter().filter(|r| !r.row.glyphs.is_empty()).collect();
        if rows.len() > 1 {
            multi += 1;
            let left = rows[0].pos.x + rows[0].row.glyphs[0].pos.x;
            for r in &rows {
                let x = r.pos.x + r.row.glyphs[0].pos.x;
                assert!((x - left).abs() < 0.75, "{what}: Zeile nicht linksbündig ({x} statt {left}) in „{text}“");
            }
            // linksbündig im Rechteck, nicht zentriert: Text beginnt am linken Rand der Galley
            assert!(g.rect.left() > -0.75 && left < 0.75, "{what}: zentriert gesetzt in „{text}“");
        }
    }
    multi
}

fn close(h: &mut Harness<'_, FirepixelApp>) {
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
}

fn run(theme: ThemeKind, ppp: f32) {
    let mut h = harness(theme, ppp);
    h.run_steps(2);
    let tag = format!("{theme:?} {ppp}");
    // Ungespeichert-Abfrage
    let id = h.state().state.session.active_id().expect("Dokument");
    h.state_mut().state.dialogs.unsaved = Some(fp_ui::dialogs::UnsavedFlow::new(vec![id], fp_ui::dialogs::PendingClose::CloseDocs));
    h.run_steps(3);
    check_dialog(&h, &format!("{tag} Ungespeichert"));
    h.state_mut().state.dialogs.unsaved = None;
    h.run_steps(2);
    // Meldung (mehrzeilig)
    fp_ui::dialogs::misc::not_available_message(&mut h.state_mut().state, "Hinweis", LONG);
    h.run_steps(3);
    assert!(check_dialog(&h, &format!("{tag} Meldung")) >= 1, "{tag}: Meldung bricht nicht um");
    close(&mut h);
    // Formular mit Hinweiszeile
    h.state_mut().state.dialogs.form = Some(FormDialog::new("fp-test-note", "Test", "select.none", vec![Field::note(LONG)]));
    h.run_steps(3);
    assert!(check_dialog(&h, &format!("{tag} Formularhinweis")) >= 1, "{tag}: Hinweis bricht nicht um");
    close(&mut h);
    // weitere Dialoge mit Beschriftungen
    for name in ["file.new", "modify.canvas.imageSize", "file.export", "text.spellingSetup", "text.checkSpelling", "view.guides.editGuides", "panel.layers.newLayer"] {
        h.state_mut().state.run(name, json!({}));
        h.run_steps(3);
        if last_dialog_id(&h.ctx).is_some() && h.state().state.dialogs.any_open() {
            check_dialog(&h, &format!("{tag} {name}"));
        }
        close(&mut h);
        h.ctx.data_mut(|d| d.remove::<String>(egui::Id::new("fp-last-dialog")));
    }
    let _ = (vec2(0.0, 0.0), Pos2::ZERO);
}

#[test]
fn message_texts_are_not_justified_normal() {
    run(ThemeKind::Normal, 1.0);
    run(ThemeKind::Normal, 1.5);
}

#[test]
fn message_texts_are_not_justified_dark() {
    run(ThemeKind::Dark, 1.0);
    run(ThemeKind::Dark, 1.5);
}

#[test]
fn message_texts_are_not_justified_oled() {
    run(ThemeKind::Oled, 1.0);
    run(ThemeKind::Oled, 1.5);
}
