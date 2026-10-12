//! E1: Datei ▸ Exportieren… als ein Ablauf (re\18-export-dialoge.md §1–§5, §7): Dateien je Art
//! (Namen, Anzahl) in einem Temp-Ordner, Grau-Regeln, Überschreiben-Rückfrage, Merken.

use fp_engine::Session;
use fp_export::doc_export::FilesStyle;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::export::{answer_confirm, begin_export, panel_rules, style_index, ExportDialog, ExportStep, Vis};
use serde_json::json;
use std::path::{Path, PathBuf};

/// 240 × 160, zwei Segmente, zwei Status, zwei Seiten, zwei gleichnamige Ebenen.
fn fixture() -> AppState {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 240, "height": 160})).expect("file.new");
    s.execute("layer.add", json!({"name": "Ebene Zwei"})).expect("layer");
    s.execute("draw.rectangle", json!({"rect": [10, 10, 100, 100], "fillColor": "#ff0000"})).expect("rect");
    s.execute("edit.insert.slice", json!({})).expect("slice 1");
    s.execute("layer.add", json!({"name": "Ebene Zwei"})).expect("layer");
    s.execute("draw.rectangle", json!({"rect": [120, 20, 200, 80], "fillColor": "#0000ff"})).expect("rect");
    s.execute("edit.insert.slice", json!({})).expect("slice 2");
    s.execute("state.add", json!({"count": 1})).expect("state");
    s.execute("page.add", json!({})).expect("page");
    s.execute("page.setCurrent", json!({"index": 0})).expect("page 0");
    s.execute("state.setCurrent", json!({"index": 0})).expect("state 0");
    s.execute("select.set", json!({"ids": []})).ok();
    AppState::new(s, UiSettings::default(), None)
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fp-e1-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn listing(dir: &Path) -> Vec<String> {
    let mut v = Vec::new();
    for e in walk(dir) {
        v.push(e.strip_prefix(dir).unwrap().to_string_lossy().replace('\\', "/"));
    }
    v.sort();
    v
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(walk(&p));
        } else {
            out.push(p);
        }
    }
    out
}

fn dialog(st: &AppState, kind: FilesStyle, dir: &Path, name: &str) -> ExportDialog {
    let mut d = ExportDialog::new(st).expect("Dialog");
    d.folder = dir.display().to_string();
    d.file_name = name.into();
    d.open_pdf = false;
    d.set_style(style_index(kind));
    d
}

fn run(st: &mut AppState, d: &mut ExportDialog) -> usize {
    match begin_export(st, d) {
        ExportStep::Done(n) => n,
        o => panic!("{o:?}"),
    }
}

#[test]
fn endung_folgt_der_art_name_bleibt() {
    let st = fixture();
    let mut d = ExportDialog::new(&st).unwrap();
    // neues Dokument: „HTML und Bilder“, Name ohne Endung (§1.3, §1.4)
    assert_eq!(d.kind(), FilesStyle::HtmlAndImages);
    assert!(!d.file_name.contains('.'), "{}", d.file_name);
    d.file_name = "bild".into();
    let ext = d.page_ext.clone();
    for (k, e) in [(FilesStyle::Pdf, "pdf".to_string()), (FilesStyle::HtmlAndImages, "htm".into()), (FilesStyle::ImagesOnly, ext.clone()), (FilesStyle::CssAndImages, "htm".into()), (FilesStyle::CssSprites, ext.clone()), (FilesStyle::LayersToFiles, ext.clone()), (FilesStyle::FramesToFiles, ext.clone()), (FilesStyle::PagesToFiles, ext.clone()), (FilesStyle::Psd, "psd".into()), (FilesStyle::Svg, "svg".into())] {
        d.set_style(style_index(k));
        assert_eq!(d.file_name, format!("bild.{e}"), "{k:?}");
    }
    // zehn Arten in der Reihenfolge der Liste, ohne markengebundene
    let kinds: Vec<FilesStyle> = fp_ui::dialogs::export::STYLES.iter().map(|s| s.1).collect();
    assert_eq!(kinds.len(), 10);
    assert_eq!(kinds[0], FilesStyle::Pdf);
    assert_eq!(kinds[9], FilesStyle::Svg);
}

#[test]
fn nur_bilder_segmente_mit_statusbildern() {
    let mut st = fixture();
    let dir = tmp("images");
    let mut d = dialog(&st, FilesStyle::ImagesOnly, &dir, "bild");
    assert!(d.has_slices);
    let n = run(&mut st, &mut d);
    let files = listing(&dir);
    assert_eq!(files.len(), n);
    let ext = d.page_ext.clone();
    let base: Vec<&String> = files.iter().filter(|f| !f.contains("_s")).collect();
    assert!(files.contains(&format!("bild_r1_c1.{ext}")), "{files:?}");
    assert!(files.contains(&format!("bild_r1_c1_s2.{ext}")), "gemessen: Status 2 mit „_s2“: {files:?}");
    // je Zelle genau ein Status-2-Bild
    assert_eq!(files.len(), base.len() * 2, "{files:?}");
    for f in &base {
        assert!(f.starts_with("bild_r") && f.contains("_c"), "{f}");
        assert!(files.contains(&f.replace(&format!(".{ext}"), &format!("_s2.{ext}"))), "{f}");
    }
    // „Nur aktueller Status“ ⇒ keine „_s2“-Bilder
    let dir2 = tmp("images-one");
    let mut d = dialog(&st, FilesStyle::ImagesOnly, &dir2, "bild");
    d.current_frame_only = true;
    run(&mut st, &mut d);
    let one = listing(&dir2);
    assert_eq!(one.len(), base.len());
    assert!(one.iter().all(|f| !f.contains("_s2")));
    // ohne Segmente: ein Bild
    let dir3 = tmp("images-whole");
    let mut d = dialog(&st, FilesStyle::ImagesOnly, &dir3, "bild");
    d.slices = 0;
    run(&mut st, &mut d);
    assert_eq!(listing(&dir3), vec![format!("bild.{ext}")]);
}

#[test]
fn html_und_css_mit_bildordner() {
    let mut st = fixture();
    let dir = tmp("html");
    let mut d = dialog(&st, FilesStyle::HtmlAndImages, &dir, "seite");
    d.subfolder = true;
    d.images_dir = "bilder/".into();
    run(&mut st, &mut d);
    let files = listing(&dir);
    assert!(files.contains(&"seite.htm".to_string()), "{files:?}");
    assert!(files.iter().any(|f| f.starts_with("bilder/seite_r1_c1.")), "{files:?}");
    let html = std::fs::read_to_string(dir.join("seite.htm")).unwrap();
    assert!(html.contains("bilder/seite_r1_c1."), "Bildpfade relativ");
    // In Zwischenablage kopieren: keine HTML-Datei, HTML in der Zwischenablage
    let dir2 = tmp("html-clip");
    let mut d = dialog(&st, FilesStyle::HtmlAndImages, &dir2, "seite");
    d.html_dest = 1;
    run(&mut st, &mut d);
    assert!(!listing(&dir2).iter().any(|f| f.ends_with(".htm")));
    assert!(st.dialogs.clipboard.as_deref().is_some_and(|h| h.contains("seite_r1_c1")));
    // CSS und Bilder
    let dir3 = tmp("css");
    let mut d = dialog(&st, FilesStyle::CssAndImages, &dir3, "lagen");
    run(&mut st, &mut d);
    let files = listing(&dir3);
    assert!(files.contains(&"lagen.htm".to_string()), "{files:?}");
    assert!(files.len() > 1);
}

#[test]
fn sprites_png_und_css() {
    let mut st = fixture();
    let dir = tmp("sprites");
    let mut d = dialog(&st, FilesStyle::CssSprites, &dir, "sprite");
    run(&mut st, &mut d);
    assert_eq!(listing(&dir), vec!["sprite.css".to_string(), "sprite.png".to_string()]);
    let css = std::fs::read_to_string(dir.join("sprite.css")).unwrap();
    assert!(css.starts_with(".sprite{ background:url(\"sprite.png\") top left no-repeat; }"), "{css}");
    // 2 Segmente × 2 Status = 4 Selektoren, Status 2 mit „_1“, Auffüllung 10
    assert_eq!(css.matches("background-position:-10px").count(), 4, "{css}");
    assert_eq!(css.matches("_1{").count(), 2, "{css}");
}

#[test]
fn ebenen_status_seiten_in_dateien() {
    let mut st = fixture();
    let ext = ExportDialog::new(&st).unwrap().page_ext;
    // Ebenen: Dateiname = Ebenenname, Leerzeichen ⇒ „_“, Doppelte „_1“, Web-Ebene übersprungen
    let dir = tmp("layers");
    let mut d = dialog(&st, FilesStyle::LayersToFiles, &dir, "eb");
    run(&mut st, &mut d);
    let files = listing(&dir);
    assert!(files.contains(&format!("Ebene_Zwei.{ext}")) && files.contains(&format!("Ebene_Zwei_1.{ext}")), "{files:?}");
    assert!(!files.iter().any(|f| f.starts_with("eb")), "eingegebener Name wird nicht benutzt: {files:?}");
    assert_eq!(files.len(), 3, "Ebene 1 + 2× Ebene Zwei: {files:?}");
    // zugeschnitten auf den Inhalt der Ebene (kleiner als die Seite 240 × 160)
    let (w, h) = image_size(&dir.join(format!("Ebene_Zwei_1.{ext}"))).expect("Bildgröße");
    assert!(w < 240 && h < 160 && w >= 80 && h >= 60, "Bilder zuschneiden: {w} × {h}");
    // ohne Zuschneiden: Seitengröße
    let dir2 = tmp("layers-full");
    let mut d = dialog(&st, FilesStyle::LayersToFiles, &dir2, "eb");
    d.trim = false;
    run(&mut st, &mut d);
    assert_eq!(image_size(&dir2.join(format!("Ebene_Zwei_1.{ext}"))), Some((240, 160)));
    // Status: s01, s02
    let dir = tmp("states");
    let mut d = dialog(&st, FilesStyle::FramesToFiles, &dir, "st");
    run(&mut st, &mut d);
    assert_eq!(listing(&dir), vec![format!("s01.{ext}"), format!("s02.{ext}")]);
    // Seiten: aktuelle Seite ⇒ eingegebener Name; alle ⇒ Seitennamen
    let dir = tmp("pages");
    let mut d = dialog(&st, FilesStyle::PagesToFiles, &dir, "se");
    run(&mut st, &mut d);
    assert_eq!(listing(&dir), vec![format!("se.{ext}")]);
    let dir = tmp("pages-all");
    let mut d = dialog(&st, FilesStyle::PagesToFiles, &dir, "se");
    d.pages = 0;
    run(&mut st, &mut d);
    assert_eq!(listing(&dir).len(), 2, "{:?}", listing(&dir));
    // als FW-PNG
    let dir = tmp("pages-fw");
    let mut d = dialog(&st, FilesStyle::PagesToFiles, &dir, "se");
    d.pages_as = 1;
    run(&mut st, &mut d);
    assert_eq!(listing(&dir), vec!["se.fw.png".to_string()]);
}

fn image_size(p: &Path) -> Option<(usize, usize)> {
    let b = std::fs::read(p).ok()?;
    if b.starts_with(b"GIF8") {
        return Some((u16::from_le_bytes([b[6], b[7]]) as usize, u16::from_le_bytes([b[8], b[9]]) as usize));
    }
    if b.starts_with(b"\x89PNG") {
        return Some((u32::from_be_bytes([b[16], b[17], b[18], b[19]]) as usize, u32::from_be_bytes([b[20], b[21], b[22], b[23]]) as usize));
    }
    None
}

#[test]
fn pdf_psd_svg() {
    let mut st = fixture();
    for (k, name, want) in [(FilesStyle::Pdf, "doc", "doc.pdf"), (FilesStyle::Psd, "ps", "ps.psd"), (FilesStyle::Svg, "vek", "vek.svg")] {
        let dir = tmp(name);
        let mut d = dialog(&st, k, &dir, name);
        run(&mut st, &mut d);
        assert_eq!(listing(&dir), vec![want.to_string()], "{k:?}");
    }
    // PDF: Vorgabe „Alle Seiten“ ⇒ eine Datei mit zwei Seiten
    let pdf = std::fs::read(tmp_existing("doc").join("doc.pdf")).unwrap();
    assert!(pdf.starts_with(b"%PDF"));
    // PSD alle Seiten ⇒ je Seite eine Datei
    let dir = tmp("psd-all");
    let mut d = dialog(&st, FilesStyle::Psd, &dir, "ps");
    d.pages = 0;
    run(&mut st, &mut d);
    assert_eq!(listing(&dir).len(), 2);
}

fn tmp_existing(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("fp-e1-{}-{name}", std::process::id()))
}

#[test]
fn grau_und_sichtbarkeitsregeln() {
    let mut st = fixture();
    let mut d = ExportDialog::new(&st).unwrap();
    // HTML und Bilder mit Segmenten, keins ausgewählt
    d.set_style(style_index(FilesStyle::HtmlAndImages));
    let r = panel_rules(&d);
    assert_eq!((r.html, r.slices, r.pages), (Vis::Free, Vis::Free, Vis::Free));
    assert_eq!(r.selected_slices, Vis::Grey, "kein Segment ausgewählt");
    assert_eq!((r.current_frame, r.include_unsliced, r.subfolder), (Vis::Free, Vis::Free, Vis::Free));
    assert_eq!(r.browse, Vis::Grey, "Durchsuchen grau, solange Unterordner aus");
    assert!(r.options);
    d.subfolder = true;
    assert_eq!(panel_rules(&d).browse, Vis::Free);
    // Segmente ≠ „Segmente exportieren“ ⇒ drei Kästchen grau
    d.slices = 0;
    let r = panel_rules(&d);
    assert_eq!((r.selected_slices, r.current_frame, r.include_unsliced), (Vis::Grey, Vis::Grey, Vis::Grey));
    d.slices = 1;
    // Nur Bilder: HTML grau, Optionen grau, Unterordner ausgeblendet
    d.set_style(style_index(FilesStyle::ImagesOnly));
    let r = panel_rules(&d);
    assert_eq!((r.html, r.subfolder, r.browse), (Vis::Grey, Vis::Hidden, Vis::Hidden));
    assert!(!r.options);
    // Segment ausgewählt ⇒ „Nur ausgewählte Segmente“ frei
    let slice_id = {
        let a = st.session.active().unwrap();
        let p = &a.doc.pages[a.page];
        p.root.layers.iter().find_map(|s| match s {
            fp_model::LayerSlot::Own(l) if l.kind == fp_model::LayerKind::Web => l.cels[0].elements.iter().find(|e| matches!(&e.kind, fp_model::ElementKind::WebObject(w) if w.slice.is_some())).map(|e| e.id),
            _ => None,
        })
    }
    .expect("Segment");
    st.session.active_mut().unwrap().selection.elements = vec![slice_id];
    let mut d2 = ExportDialog::new(&st).unwrap();
    d2.set_style(style_index(FilesStyle::ImagesOnly));
    assert!(d2.slice_selected);
    assert_eq!(panel_rules(&d2).selected_slices, Vis::Free);
    // nur das ausgewählte Segment exportieren (mit Status 2)
    let dir = tmp("selected");
    d2.folder = dir.display().to_string();
    d2.file_name = "sel".into();
    d2.selected_slices_only = true;
    run(&mut st, &mut d2);
    assert_eq!(listing(&dir).len(), 2, "{:?}", listing(&dir));
    // Dokument ohne Segmente: Segmente grau
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 50, "height": 50})).unwrap();
    let plain = AppState::new(s, UiSettings::default(), None);
    let mut d3 = ExportDialog::new(&plain).unwrap();
    d3.set_style(style_index(FilesStyle::HtmlAndImages));
    let r = panel_rules(&d3);
    assert_eq!((r.slices, r.current_frame, r.include_unsliced), (Vis::Grey, Vis::Grey, Vis::Grey));
    // übrige Arten
    let rules = |k: FilesStyle| {
        let mut x = d.clone();
        x.set_style(style_index(k));
        panel_rules(&x)
    };
    let r = rules(FilesStyle::CssAndImages);
    assert_eq!((r.pages, r.trim, r.current_frame, r.subfolder, r.html, r.slices), (Vis::Free, Vis::Grey, Vis::Grey, Vis::Free, Vis::Hidden, Vis::Hidden));
    let r = rules(FilesStyle::CssSprites);
    assert_eq!((r.pages, r.selected_slices, r.current_frame), (Vis::Free, Vis::Grey, Vis::Free));
    let r = rules(FilesStyle::LayersToFiles);
    assert_eq!((r.trim, r.current_page, r.current_frame, r.pages), (Vis::Free, Vis::Grey, Vis::Grey, Vis::Hidden));
    let r = rules(FilesStyle::FramesToFiles);
    assert_eq!((r.trim, r.current_page, r.current_frame), (Vis::Free, Vis::Grey, Vis::Hidden));
    let r = rules(FilesStyle::PagesToFiles);
    assert_eq!((r.export_as, r.pages, r.trim, r.current_frame), (Vis::Free, Vis::Free, Vis::Grey, Vis::Grey));
    let r = rules(FilesStyle::Pdf);
    assert_eq!((r.pages, r.open_pdf), (Vis::Free, Vis::Free));
    assert!(r.options);
    let r = rules(FilesStyle::Svg);
    assert_eq!(r.pages, Vis::Free);
    assert!(!r.options, "SVG: Optionen grau wie FXG");
    assert!(rules(FilesStyle::Psd).options);
}

#[test]
fn ueberschreiben_rueckfrage_ok_und_abbrechen() {
    let mut st = fixture();
    let dir = tmp("replace");
    let mut d = dialog(&st, FilesStyle::ImagesOnly, &dir, "bild");
    run(&mut st, &mut d);
    let ext = d.page_ext.clone();
    let first = dir.join(format!("bild_r1_c1.{ext}"));
    std::fs::write(&first, b"alt").unwrap();
    let before = listing(&dir);
    let mut d = dialog(&st, FilesStyle::ImagesOnly, &dir, "bild");
    match begin_export(&mut st, &mut d) {
        ExportStep::Confirm(q) => assert_eq!(q, format!("bild_r1_c1.{ext} ist bereits vorhanden. Fortfahren und Dateien ersetzen?")),
        o => panic!("{o:?}"),
    }
    // Abbrechen: nichts geschrieben, zurück im Dialog
    assert_eq!(answer_confirm(&mut st, &mut d, false), ExportStep::Error(String::new()));
    assert_eq!(std::fs::read(&first).unwrap(), b"alt");
    assert_eq!(listing(&dir), before);
    assert!(d.confirm.is_none());
    // OK: alle ersetzt
    assert!(matches!(begin_export(&mut st, &mut d), ExportStep::Confirm(_)));
    assert!(matches!(answer_confirm(&mut st, &mut d, true), ExportStep::Done(_)));
    assert_ne!(std::fs::read(&first).unwrap(), b"alt");
    // Englisch
    fp_i18n::set_language("en");
    let mut d = dialog(&st, FilesStyle::ImagesOnly, &dir, "bild");
    match begin_export(&mut st, &mut d) {
        ExportStep::Confirm(q) => assert_eq!(q, format!("bild_r1_c1.{ext} already exists. Continue and replace files?")),
        o => panic!("{o:?}"),
    }
    fp_i18n::set_language("de");
}

#[test]
fn merken_ordner_und_art_ohne_verlaufsschritt() {
    let mut st = fixture();
    let hist = st.session.active().unwrap().history.len();
    let gen0 = st.session.active().unwrap().generation;
    let dir = tmp("remember");
    // Abbrechen ändert nichts
    let d0 = ExportDialog::new(&st).unwrap();
    let mut d = d0.clone();
    d.set_style(style_index(FilesStyle::CssSprites));
    drop(d);
    assert_eq!(ExportDialog::new(&st).unwrap(), d0);
    // Export merkt Art (Dokument) und Ordner (Voreinstellungen + Dokument)
    let mut d = dialog(&st, FilesStyle::CssSprites, &dir, "sprite");
    run(&mut st, &mut d);
    let again = ExportDialog::new(&st).unwrap();
    assert_eq!(again.kind(), FilesStyle::CssSprites);
    assert_eq!(again.folder, dir.display().to_string());
    assert_eq!(st.session.prefs.get_str("LastExportDirectory", ""), dir.display().to_string());
    let a = st.session.active().unwrap();
    let p = &a.doc.pages[a.page];
    assert_eq!(p.last_export_dir.as_deref(), Some(dir.display().to_string().as_str()));
    let h = p.root.export.as_ref().and_then(|e| e.html.as_deref()).unwrap();
    assert_eq!((h.export_files_style, h.not_yet_set), (Some(6), Some(false)));
    // SVG (ohne Wert in `exportFilesStyle`) über den Namen der letzten Exportdatei
    let mut d = dialog(&st, FilesStyle::Svg, &dir, "vek");
    run(&mut st, &mut d);
    assert_eq!(ExportDialog::new(&st).unwrap().kind(), FilesStyle::Svg);
    // kein Verlaufsschritt, Inhalt unverändert
    assert_eq!(st.session.active().unwrap().history.len(), hist);
    assert_eq!(st.session.active().unwrap().generation, gen0);
}

#[test]
fn fehlender_ordner_meldet_und_schreibt_nichts() {
    let mut st = fixture();
    let mut d = dialog(&st, FilesStyle::Pdf, Path::new("Z:\\gibt\\es\\nicht\\fp-e1"), "x");
    match begin_export(&mut st, &mut d) {
        ExportStep::Error(e) => assert!(e.contains("existiert nicht"), "{e}"),
        o => panic!("{o:?}"),
    }
}

/// Der Dialog zeichnet jede Art ohne Panik (de/en), Esc schließt ohne Export.
#[test]
fn dialog_zeichnet_alle_arten() {
    for lang in ["de", "en"] {
        fp_i18n::set_language(lang);
        let st0 = fixture();
        let mut harness = egui_kittest::Harness::builder().with_size(egui::vec2(1100.0, 760.0)).build_ui_state(
            |ui, st: &mut AppState| {
                fp_ui::dialogs::export::show_export(ui.ctx(), st);
            },
            st0,
        );
        for i in 0..fp_ui::dialogs::export::STYLES.len() {
            let mut d = ExportDialog::new(harness.state()).unwrap();
            d.set_style(i);
            harness.state_mut().dialogs.export = Some(d);
            harness.run_steps(2);
            assert!(harness.state().dialogs.export.is_some(), "Art {i}");
        }
        harness.key_press(egui::Key::Escape);
        harness.run_steps(2);
    }
    fp_i18n::set_language("de");
}
