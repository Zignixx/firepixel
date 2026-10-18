//! E3: HTML-Einrichtung und Exportassistent (re\18-export-dialoge.md §6, §8), HTML-Code kopieren,
//! HTML aktualisieren und einheitliche Rollover-Dateinamen „_s2“ (§3).

use fp_engine::Session;
use fp_export::options::{ColorMode, ExportFormat};
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::export::ExportDialog;
use fp_ui::dialogs::export_wizard::{self, Destination, ExportWizard, FramesChoice, ResultPage, Step};
use fp_ui::dialogs::html_setup::{self, HtmlSetupDialog};
use fp_ui::dialogs::image_preview::ViewMode;
use serde_json::json;
use std::path::{Path, PathBuf};

/// 240 × 160, zwei Segmente (das erste mit einfachem Rollover), zwei Status.
fn fixture() -> AppState {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 240, "height": 160})).expect("file.new");
    s.execute("draw.rectangle", json!({"rect": [10, 10, 100, 100], "fillColor": "#ff0000"})).expect("rect");
    s.execute("edit.insert.slice", json!({})).expect("slice 1");
    s.execute("pi.behavior.add", json!({"action": 8})).expect("rollover");
    s.execute("draw.rectangle", json!({"rect": [120, 20, 200, 80], "fillColor": "#0000ff"})).expect("rect");
    s.execute("edit.insert.slice", json!({})).expect("slice 2");
    s.execute("state.add", json!({"count": 1})).expect("state");
    s.execute("state.setCurrent", json!({"index": 0})).expect("state 0");
    s.execute("select.set", json!({"ids": []})).ok();
    AppState::new(s, UiSettings::default(), None)
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fp-e3-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn listing(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
    v.sort();
    v
}

fn undo_count(st: &AppState) -> usize {
    st.session.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn export_html(st: &mut AppState, dir: &Path, base: &str) -> (Vec<String>, String) {
    st.session.execute("file.export", json!({"path": dir.display().to_string(), "style": "htmlAndImages", "baseName": base})).expect("export");
    let files = listing(dir);
    let html = files.iter().find(|f| f.ends_with(".htm") || f.ends_with(".html") || f.ends_with(".php")).expect("HTML-Datei");
    let text = std::fs::read_to_string(dir.join(html)).unwrap();
    (files, text)
}

#[test]
fn html_einrichtung_felder_wirken_auf_das_html() {
    let mut st = fixture();
    // Vorgabe: XHTML, .htm, Rollover-Bild mit „_s2“
    let dir = tmp("vorgabe");
    let (files, html) = export_html(&mut st, &dir, "Seite");
    assert!(files.iter().any(|f| f == "Seite.htm"), "{files:?}");
    assert!(files.iter().any(|f| f.ends_with("_s2.png") || f.ends_with("_s2.gif") || f.ends_with("_s2.jpg")), "Rollover _s2: {files:?}");
    assert!(!files.iter().any(|f| f.contains("_f2")), "kein _f2 mehr: {files:?}");
    assert!(html.contains("XHTML 1.0"));

    let mut d = HtmlSetupDialog::new(&st).expect("Dialog");
    assert_eq!(d.tab, 0, "Start-Reiter Allgemein");
    d.style = 0;
    d.extension = ".html".into();
    d.comments = true;
    d.lowercase = true;
    d.background_url = "bg.png".into();
    d.background_repeat = 2;
    d.page_alignment = 1;
    d.shims = 1; // Einzelne Tabelle ohne Platzhalter
    d.use_canvas = false;
    d.cell_color = "#123456".into();
    d.contents = 2; // Geschütztes Leerzeichen
    d.alt = "Bild".into();
    d.include_unsliced = false;
    d.utf8 = false;
    let before = undo_count(&st);
    html_setup::apply(&mut st, &d);
    assert_eq!(undo_count(&st), before, "HTML-Einrichtung ohne Verlaufsschritt");
    // Speicherort: Endung, Kommentare, Kleinbuchstaben, leere Zellen in den Voreinstellungen
    let p = &st.session.prefs;
    assert_eq!(p.get_str("HTMLExtension", ""), ".html");
    assert!(p.get_bool("HTMLCommentsOn", false));
    assert!(p.get_bool("ForceLowercaseFilenames", false));
    assert!(!p.get_bool("HTMLEmptyCellUsesCanvasColor", true));
    assert_eq!(p.get_int("HTMLEmptyCellContents", 0), 3);
    // … der Rest im Dokument
    let v = st.session.execute("file.htmlSetup", json!({})).unwrap().value;
    assert_eq!(v["xhtml"], json!(0));
    assert_eq!(v["shim_generation"], json!(0));
    assert_eq!(v["default_alt"], json!("Bild"));
    assert_eq!(v["discard_unspecified_slices"], json!(true));

    let dir = tmp("geaendert");
    let (files, html) = export_html(&mut st, &dir, "Seite");
    assert!(files.iter().any(|f| f == "seite.html"), "Endung + Kleinbuchstaben: {files:?}");
    assert!(files.iter().all(|f| *f == f.to_lowercase()), "{files:?}");
    assert!(!files.iter().any(|f| f == "spacer.gif"), "ohne Abstandhalter: {files:?}");
    assert!(html.contains("HTML 4.01") && !html.contains("XHTML"), "HTML-Stil");
    assert!(html.contains("charset=iso-8859-1"), "ohne UTF-8");
    assert!(html.contains("<!-- Firepixel: begin of the sliced table"), "Kommentare");
    assert!(html.contains("alt=\"Bild\""), "Standard-ALT-Tag");
    assert!(html.contains("background-image: url(\"bg.png\")") && html.contains("repeat-x") && html.contains("text-align: center"), "Dokumenteigenschaften");
    assert!(html.contains("bgcolor=\"#123456\">&nbsp;</td>"), "leere Zellen: {html}");
}

#[test]
fn ohne_aenderung_nichts_geschrieben() {
    let mut st = fixture();
    let d = HtmlSetupDialog::new(&st).unwrap();
    assert!(d.unchanged());
    let doc_before = st.session.active().unwrap().doc.pages[0].root.export.clone();
    let prefs_before = st.session.prefs.values.clone();
    let mut d2 = d.clone();
    d2.tab = 2; // nur den Reiter gewechselt
    html_setup::apply(&mut st, &d2);
    assert_eq!(st.session.active().unwrap().doc.pages[0].root.export, doc_before);
    assert_eq!(st.session.prefs.values, prefs_before);
    // Beispielzeile live aus den Listen
    let mut d3 = d.clone();
    assert_eq!(d3.example("bild"), "bild_r1_c2_s2.gif");
    d3.frame2 = 3; // Rollover
    assert_eq!(d3.example("bild"), "bild_r1_c2_over.gif");
}

#[test]
fn endung_wirkt_im_offenen_exportdialog() {
    let mut st = fixture();
    let mut x = ExportDialog::new(&st).unwrap();
    x.file_name = "seite.htm".into();
    st.dialogs.export = Some(x);
    html_setup::open(&mut st);
    let mut d = st.dialogs.html_setup.clone().expect("HTML-Einrichtung offen");
    d.extension = ".php".into();
    st.dialogs.html_setup = None;
    html_setup::apply(&mut st, &d);
    let x = st.dialogs.export.as_ref().unwrap();
    assert_eq!(x.html_ext, "php");
    assert_eq!(x.file_name, "seite.php");
}

#[test]
fn assistent_bis_zur_bildvorschau() {
    // Web: Start → Status → Ziel → Analyseergebnisse → Bildvorschau 2-fach GIF | JPEG
    let mut st = fixture();
    let mut w = ExportWizard::new(&st).unwrap();
    assert_eq!(w.frames, 2);
    w.analyze = false;
    assert!(!w.advance());
    assert_eq!(w.step, Step::Frames);
    w.frames_choice = FramesChoice::SingleImage;
    assert!(!w.advance());
    assert_eq!(w.step, Step::Destination);
    w.destination = Destination::Web;
    assert!(!w.advance());
    assert_eq!(w.step, Step::Result(ResultPage::WebAnalysis));
    // Zurück und anders entscheiden
    w.back();
    assert_eq!(w.step, Step::Destination);
    w.back();
    w.back();
    assert_eq!(w.step, Step::Start);
    assert!(w.history.is_empty());
    w.analyze = false;
    w.advance();
    w.frames_choice = FramesChoice::SingleImage;
    w.advance();
    w.destination = Destination::WebPage;
    w.advance();
    let before = st.session.active().unwrap().doc.pages[0].root.export.clone();
    assert!(w.advance(), "Beenden");
    export_wizard::finish(&mut st, &w);
    let p = st.dialogs.preview.as_ref().expect("Bildvorschau offen");
    assert_eq!(p.mode, ViewMode::Two);
    assert_eq!(p.panes[0].settings.format, ExportFormat::Gif);
    // BA-032: Vorschlag GIF Web 216 wie im Original
    assert_eq!(p.panes[0].settings.max_colors, 216);
    assert_eq!(p.panes[0].settings.palette_kind, fp_export::options::PaletteKind::Web216);
    assert_eq!(p.panes[1].settings.format, ExportFormat::Jpeg);
    assert_eq!(p.panes[1].settings.jpeg.quality, 80);
    assert_eq!(st.session.active().unwrap().doc.pages[0].root.export, before, "erst die Bildvorschau übernimmt");

    // Bildbearbeitung ⇒ TIFF 24 Bit, 1-fach
    let mut st = fixture();
    let mut w = ExportWizard::new(&st).unwrap();
    w.analyze = false;
    w.advance();
    w.frames_choice = FramesChoice::SingleImage;
    w.advance();
    w.destination = Destination::ImageEditor;
    w.advance();
    assert_eq!(w.step, Step::Result(ResultPage::Tiff));
    assert!(w.advance());
    export_wizard::finish(&mut st, &w);
    let p = st.dialogs.preview.as_ref().unwrap();
    assert_eq!(p.mode, ViewMode::One);
    assert_eq!(p.panes[0].settings.format, ExportFormat::Tiff);
    assert_eq!(p.panes[0].settings.color_mode, ColorMode::Rgb24);

    // Animiertes GIF ⇒ direkt in die Bildvorschau
    let mut st = fixture();
    let mut w = ExportWizard::new(&st).unwrap();
    w.analyze = false;
    w.advance();
    w.frames_choice = FramesChoice::AnimatedGif;
    assert!(w.advance());
    export_wizard::finish(&mut st, &w);
    assert_eq!(st.dialogs.preview.as_ref().unwrap().panes[0].settings.format, ExportFormat::AnimatedGif);

    // Rollover ⇒ Analyseergebnisse
    let st = fixture();
    let mut w = ExportWizard::new(&st).unwrap();
    w.analyze = false;
    w.advance();
    w.frames_choice = FramesChoice::Rollover;
    w.advance();
    assert_eq!(w.step, Step::Result(ResultPage::WebAnalysis));
}

#[test]
fn assistent_zielgroesse_und_auswerten() {
    let mut st = fixture();
    st.session.execute("file.imagePreview", json!({"settings": {"format": "gif", "palette": "adaptive", "colors": 64}, "apply": true})).unwrap();
    let mut w = ExportWizard::new(&st).unwrap();
    assert!(w.analyzable && w.analyze, "GIF ist auswertbar und vorgewählt");
    assert!(!w.advance());
    assert_eq!(w.step, Step::Result(ResultPage::AdaptivePalette));
    w.target_on = true;
    w.target_kb = 1.0;
    export_wizard::finish(&mut st, &w);
    let p = st.dialogs.preview.as_ref().unwrap();
    assert_eq!(p.panes[0].settings.format, ExportFormat::Gif);
    // PNG 32 ist nicht auswertbar
    let mut st = fixture();
    st.session.execute("file.imagePreview", json!({"settings": {"format": "png32"}, "apply": true})).unwrap();
    let w = ExportWizard::new(&st).unwrap();
    assert!(!w.analyzable && !w.analyze);
}

#[test]
fn html_kopieren_und_aktualisieren() {
    let mut st = fixture();
    // Kopieren mit Bildordner: Code in die Zwischenablage, Bilder geschrieben
    let dir = tmp("kopieren");
    fp_ui::dialogs::menu_dialogs::copy_html(&mut st, Some(&dir.display().to_string()));
    let html = st.dialogs.clipboard.clone().expect("HTML in der Zwischenablage");
    assert!(html.contains("<!-- fwtable"));
    let files = listing(&dir);
    assert!(!files.is_empty() && files.iter().all(|f| !f.ends_with(".htm")), "{files:?}");
    assert!(st.dialogs.message.is_none());

    // Aktualisieren: nur die Tabelle wird ersetzt, eigener Inhalt bleibt
    let dir = tmp("aktualisieren");
    let page = dir.join("seite.html");
    let (_, exported) = export_html(&mut st, &dir, "seite");
    std::fs::remove_file(dir.join("seite.htm")).unwrap();
    let own = exported.replace("<body", "<p>Eigener Text</p>\n<body").replace("</body>", "<p>Fuss</p>\n</body>");
    std::fs::write(&page, &own).unwrap();
    st.session.execute("draw.rectangle", json!({"rect": [0, 120, 240, 160], "fillColor": "#00ff00"})).unwrap();
    st.session.execute("edit.insert.slice", json!({})).unwrap();
    fp_ui::dialogs::menu_dialogs::update_html(&mut st, &page.display().to_string());
    assert!(st.dialogs.message.is_none(), "{:?}", st.dialogs.message);
    let new = std::fs::read_to_string(&page).unwrap();
    assert!(new.contains("Eigener Text") && new.contains("<p>Fuss</p>"), "eigener Inhalt bleibt");
    assert_ne!(new, own, "Tabelle neu");
    assert_eq!(new.matches("<!-- fwtable").count(), 1);

    // Seite ohne Firepixel-Tabelle ⇒ Tabelle am Ende des Body eingefügt
    let other = dir.join("fremd.html");
    std::fs::write(&other, "<html><body><p>nichts</p></body></html>").unwrap();
    fp_ui::dialogs::menu_dialogs::update_html(&mut st, &other.display().to_string());
    assert!(st.dialogs.message.is_none());
    let t = std::fs::read_to_string(&other).unwrap();
    assert!(t.starts_with("<html><body><p>nichts</p><table") && t.contains("<!-- fwtable") && t.ends_with("</body></html>"), "{t}");
    // keine HTML-Seite ⇒ klare Meldung, Datei unverändert
    let txt = dir.join("text.htm");
    std::fs::write(&txt, "nur Text").unwrap();
    fp_ui::dialogs::menu_dialogs::update_html(&mut st, &txt.display().to_string());
    let (_, msg) = st.dialogs.message.clone().expect("Meldung");
    assert!(msg.contains("keine HTML-Seite"), "{msg}");
    assert_eq!(std::fs::read_to_string(&txt).unwrap(), "nur Text");
    // fehlende Datei ⇒ Meldung, kein Absturz
    st.dialogs.message = None;
    fp_ui::dialogs::menu_dialogs::update_html(&mut st, &dir.join("gibtsnicht.htm").display().to_string());
    assert!(st.dialogs.message.is_some());
}

#[test]
fn dialoge_zeichnen_in_allen_sprachen() {
    for lang in ["de", "en", "qps"] {
        fp_i18n::set_language(lang);
        let st0 = fixture();
        let mut harness = egui_kittest::Harness::builder().with_size(egui::vec2(1100.0, 800.0)).build_ui_state(
            |ui, st: &mut AppState| {
                html_setup::show(ui.ctx(), st);
                export_wizard::show(ui.ctx(), st);
            },
            st0,
        );
        for tab in 0..3 {
            let mut d = HtmlSetupDialog::new(harness.state()).unwrap();
            d.tab = tab;
            harness.state_mut().dialogs.html_setup = Some(d);
            harness.run_steps(3);
            assert!(harness.state().dialogs.html_setup.is_some(), "{lang} Reiter {tab}");
        }
        harness.key_press(egui::Key::Escape);
        harness.run_steps(3);
        assert!(harness.state().dialogs.html_setup.is_none(), "Esc schließt");
        let mut w = ExportWizard::new(harness.state()).unwrap();
        w.analyze = false;
        for _ in 0..3 {
            harness.state_mut().dialogs.wizard = Some(w.clone());
            harness.run_steps(3);
            assert!(harness.state().dialogs.wizard.is_some(), "{lang} {:?}", w.step);
            w.advance();
        }
        harness.key_press(egui::Key::Escape);
        harness.run_steps(3);
        assert!(harness.state().dialogs.wizard.is_none());
    }
    fp_i18n::set_language("de");
}
