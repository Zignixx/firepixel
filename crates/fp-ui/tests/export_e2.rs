//! E2: Optionen-Dialoge des Exports (re\18-export-dialoge.md §4): „Optionen…“ öffnet je Art den
//! richtigen Dialog; die Werte wirken auf die geschriebenen Dateien und werden gemerkt
//! (PDF/Photoshop/Ebenen-Status-Seiten in den Voreinstellungen, CSS-Sprites im Dokument).

use fp_engine::Session;
use fp_export::doc_export::FilesStyle;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::export::{begin_export, panel_rules, style_index, ExportDialog, ExportStep, STYLES};
use fp_ui::dialogs::export_options::{FilesOpts, OptionsDialog, OptionsTarget, PdfOpts, PsdOpts};
use serde_json::json;
use std::path::{Path, PathBuf};

/// 240 × 160, zwei Segmente, zwei Status, zwei Ebenen „Ebene Zwei“.
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
    s.execute("state.setCurrent", json!({"index": 0})).expect("state 0");
    s.execute("select.set", json!({"ids": []})).ok();
    AppState::new(s, UiSettings::default(), None)
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fp-e2-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn listing(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
    v.sort();
    v
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

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

#[test]
fn optionen_knopf_je_art() {
    let st = fixture();
    for (i, (_, kind)) in STYLES.iter().enumerate() {
        let mut d = ExportDialog::new(&st).unwrap();
        d.set_style(i);
        let target = d.open_options();
        let want = match kind {
            FilesStyle::Pdf => OptionsTarget::Pdf,
            FilesStyle::CssSprites => OptionsTarget::Sprites,
            FilesStyle::LayersToFiles | FilesStyle::FramesToFiles | FilesStyle::PagesToFiles => OptionsTarget::Files,
            FilesStyle::Psd => OptionsTarget::Psd,
            FilesStyle::HtmlAndImages | FilesStyle::CssAndImages => OptionsTarget::ImagePreview,
            FilesStyle::ImagesOnly | FilesStyle::Svg => OptionsTarget::None,
        };
        assert_eq!(target, want, "{kind:?}");
        // Dialog offen genau bei den eigenen Optionen; grau bei Nur Bilder und SVG
        let own = matches!(want, OptionsTarget::Pdf | OptionsTarget::Sprites | OptionsTarget::Files | OptionsTarget::Psd);
        assert_eq!(d.options.is_some(), own, "{kind:?}");
        assert_eq!(panel_rules(&d).options, want != OptionsTarget::None, "{kind:?}");
        if let Some(OptionsDialog::Files(k, _)) = &d.options {
            assert_eq!(k, kind);
        }
    }
}

#[test]
fn pdf_optionen_wirken_und_werden_gemerkt() {
    let mut st = fixture();
    // Vorgabe: PDF 1.6, JPEG, Hoch
    let d0 = ExportDialog::new(&st).unwrap();
    assert_eq!((d0.pdf.compatibility, d0.pdf.compression, d0.pdf.quality), (3, 2, 0));
    let dir = tmp("pdf-default");
    let mut d = dialog(&st, FilesStyle::Pdf, &dir, "doc");
    run(&mut st, &mut d);
    let pdf = std::fs::read(dir.join("doc.pdf")).unwrap();
    assert!(pdf.starts_with(b"%PDF-1.6") && contains(&pdf, b"/DCTDecode"), "Vorgabe JPEG/1.6");
    // OK im Dialog: PDF 1.3, ohne Komprimierung, Graustufen, Anschnitt 10 px
    let mut d = dialog(&st, FilesStyle::Pdf, &tmp("pdf-a"), "doc");
    d.open_options();
    let Some(OptionsDialog::Pdf(o)) = d.options.as_mut() else { panic!() };
    o.compatibility = 0;
    o.compression = 0;
    o.grayscale = true;
    o.bleed = 10;
    d.close_options(&mut st, true);
    let dir = PathBuf::from(&d.folder);
    run(&mut st, &mut d);
    let pdf = std::fs::read(dir.join("doc.pdf")).unwrap();
    assert!(pdf.starts_with(b"%PDF-1.3"));
    assert!(contains(&pdf, b"/ColorSpace /DeviceGray") && !contains(&pdf, b"/DCTDecode") && !contains(&pdf, b"/FlateDecode"));
    assert!(contains(&pdf, b"/TrimBox"), "Anschnitt");
    // gemerkt in den Voreinstellungen (1-basiert), neuer Dialog übernimmt sie
    assert_eq!(st.session.prefs.get_int("PDFExport_Compatibility", 0), 1);
    assert_eq!(st.session.prefs.get_int("PDFExport_Compression", 0), 1);
    let again = ExportDialog::new(&st).unwrap().pdf;
    assert_eq!((again.compatibility, again.compression, again.grayscale, again.bleed), (0, 0, true, 10));
    // Kennwörter wirken, werden aber nicht gemerkt
    let dir = tmp("pdf-pw");
    let mut d = dialog(&st, FilesStyle::Pdf, &dir, "doc");
    d.open_options();
    let Some(OptionsDialog::Pdf(o)) = d.options.as_mut() else { panic!() };
    o.compatibility = 3;
    o.compression = 1;
    o.use_open_password = true;
    o.open_password = "auf".into();
    o.use_permissions_password = true;
    o.permissions_password = "chef".into();
    o.allow_print = true;
    d.close_options(&mut st, true);
    run(&mut st, &mut d);
    let pdf = std::fs::read(dir.join("doc.pdf")).unwrap();
    assert!(contains(&pdf, b"/Encrypt") && contains(&pdf, b"/R 3") && contains(&pdf, b"/ID ["), "Kennwortschutz");
    assert!(contains(&pdf, b"/FlateDecode"), "ZIP");
    // Drucken erlaubt (Bit 3), Kopieren nicht (Bit 5)
    let text = String::from_utf8_lossy(&pdf);
    let p: i64 = text.split("/P ").nth(1).unwrap().split_whitespace().next().unwrap().parse().unwrap();
    assert!(p & 4 != 0 && p & 16 == 0, "P = {p}");
    let again = ExportDialog::new(&st).unwrap().pdf;
    assert!(!again.use_open_password && again.open_password.is_empty() && again.permissions_password.is_empty());
    // Abbrechen ändert nichts
    let before = PdfOpts::from_prefs(&st);
    let mut d = ExportDialog::new(&st).unwrap();
    d.set_style(style_index(FilesStyle::Pdf));
    d.open_options();
    if let Some(OptionsDialog::Pdf(o)) = d.options.as_mut() {
        o.quality = 4;
    }
    d.close_options(&mut st, false);
    assert_eq!(d.pdf, before);
    assert_eq!(PdfOpts::from_prefs(&st), before);
    // Seitenwahl und „PDF nach Export anzeigen“ werden beim Export gemerkt (§2.10)
    let mut d = dialog(&st, FilesStyle::Pdf, &tmp("pdf-pages"), "doc");
    d.pdf_pages = 1;
    run(&mut st, &mut d);
    let again = ExportDialog::new(&st).unwrap();
    assert_eq!((again.pdf_pages, again.open_pdf), (1, false));
}

#[test]
fn sprite_optionen_wirken_und_werden_im_dokument_gemerkt() {
    let mut st = fixture();
    let dir = tmp("sprite");
    let mut d = dialog(&st, FilesStyle::CssSprites, &dir, "sprite");
    assert_eq!(d.file_name, "sprite.png");
    d.open_options();
    let Some(OptionsDialog::Sprites(o)) = d.options.as_mut() else { panic!() };
    // Werte aus den Voreinstellungen (re 28 E6): class, leer, Raster, PNG32; Auffüllung fest 10 px
    // (M10, re 43 §1: `SpritePadding` 11 wird nicht gelesen)
    assert_eq!((o.selector_is_class, o.prefix.as_str(), o.padding, o.layout, o.preset), (true, "", 10, 2, 0));
    o.selector_is_class = false;
    o.prefix = "p-".into();
    o.suffix = "-s".into();
    o.padding = 4;
    o.layout = 0;
    o.preset = 1; // JPEG – Höhere Qualität
    assert!(o.example().starts_with("#p-") && o.example().ends_with("-s"), "{}", o.example());
    d.close_options(&mut st, true);
    assert_eq!(d.file_name, "sprite.jpg", "Endung folgt der Vorgabe");
    run(&mut st, &mut d);
    assert_eq!(listing(&dir), vec!["sprite.css".to_string(), "sprite.jpg".to_string()]);
    assert!(std::fs::read(dir.join("sprite.jpg")).unwrap().starts_with(&[0xFF, 0xD8]));
    let css = std::fs::read_to_string(dir.join("sprite.css")).unwrap();
    assert!(css.starts_with("#sprite{ background:url(\"sprite.jpg\")"), "{css}");
    assert_eq!(css.matches("#p-").count(), 4, "{css}");
    assert_eq!(css.matches("-s{").count() + css.matches("-s_").count(), 4, "{css}");
    // waagerecht: alle Teile auf y = 4, x wächst
    assert_eq!(css.matches(" -4px; }").count(), 4, "{css}");
    assert!(css.contains("background-position:-4px -4px;"), "{css}");
    assert!(d.notice.is_none(), "gültige Namen ⇒ keine Meldung");
    // im Dokument gemerkt, neuer Dialog übernimmt die Werte
    let a = st.session.active().unwrap();
    let h = a.doc.pages[a.page].root.export.as_ref().and_then(|e| e.html.as_deref()).unwrap();
    assert_eq!((h.sprite_padding, h.sprite_orientation, h.sprite_selector_is_class), (Some(4), Some(1), Some(false)));
    let again = ExportDialog::new(&st).unwrap();
    // übrige Werte aus den Voreinstellungen, die Auffüllung beginnt wieder bei 10 (M10)
    let mut want = d.sprite.clone();
    want.padding = 10;
    assert_eq!(again.sprite, want);
    // ungültige Selektornamen ⇒ Meldung (§4.2)
    let dir = tmp("sprite-bad");
    let mut d = dialog(&st, FilesStyle::CssSprites, &dir, "sprite");
    d.sprite.prefix = "1 x".into();
    run(&mut st, &mut d);
    assert_eq!(d.notice.as_deref(), Some(fp_i18n::t("dlg-expopt-sprite-modified").as_str()));
    // Abbrechen ändert nichts
    let mut d = ExportDialog::new(&st).unwrap();
    d.set_style(style_index(FilesStyle::CssSprites));
    let before = d.sprite.clone();
    d.open_options();
    if let Some(OptionsDialog::Sprites(o)) = d.options.as_mut() {
        o.padding = 77;
    }
    d.close_options(&mut st, false);
    assert_eq!(d.sprite, before);
}

#[test]
fn dateien_optionen_praefix_suffix_dateityp() {
    let mut st = fixture();
    let doc_name = fp_ui::dialogs::export::file_stem(&st.session.active().unwrap().title);
    let doc_name: String = doc_name.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' }).collect();
    // Ebenen: Präfix Dokumentname, Suffix # (01…), Dateityp GIF Web 216
    let dir = tmp("files-layers");
    let mut d = dialog(&st, FilesStyle::LayersToFiles, &dir, "eb");
    d.open_options();
    let Some(OptionsDialog::Files(_, o)) = d.options.as_mut() else { panic!() };
    o.add_prefix = true;
    o.prefix = 1;
    o.add_suffix = true;
    o.suffix = 3;
    o.file_type = 4; // GIF Web 216
    d.close_options(&mut st, true);
    run(&mut st, &mut d);
    let files = listing(&dir);
    assert_eq!(files.len(), 3, "{files:?}");
    assert!(files.iter().all(|f| f.starts_with(&format!("{doc_name}_")) && f.ends_with(".gif")), "{files:?}");
    assert!(files.contains(&format!("{doc_name}_Ebene_Zwei_02.gif")), "{files:?}");
    assert!(std::fs::read(dir.join(&files[0])).unwrap().starts_with(b"GIF8"));
    // gemerkt in den Voreinstellungen, je Art
    assert_eq!(FilesOpts::from_prefs(&st, FilesStyle::LayersToFiles), d.files_layers);
    assert_eq!(ExportDialog::new(&st).unwrap().files_layers, d.files_layers);
    assert_eq!(FilesOpts::from_prefs(&st, FilesStyle::FramesToFiles), FilesOpts::default());
    // Status: Suffix # (1…), Dokumenteinstellungen ⇒ s01_1.<ext>; Status 2 der Vorlage ist leer
    // (`state.add` ohne Inhalt, nur die Webebene) und wird mit „Bilder zuschneiden“ (Vorgabe)
    // übersprungen – keine Datei (CT-C-18, re 29 §3)
    let dir = tmp("files-frames");
    let mut d = dialog(&st, FilesStyle::FramesToFiles, &dir, "st");
    let ext = d.page_ext.clone();
    d.open_options();
    let Some(OptionsDialog::Files(_, o)) = d.options.as_mut() else { panic!() };
    o.add_suffix = true;
    o.suffix = 2;
    // Liste grau ⇒ ohne Kästchen kein Präfix
    o.prefix = 1;
    d.close_options(&mut st, true);
    run(&mut st, &mut d);
    assert_eq!(listing(&dir), vec![format!("s01_1.{ext}")]);
    // Seiten: aktuelle Seite mit Präfix # (01…)
    let dir = tmp("files-pages");
    let mut d = dialog(&st, FilesStyle::PagesToFiles, &dir, "se");
    d.open_options();
    let Some(OptionsDialog::Files(_, o)) = d.options.as_mut() else { panic!() };
    o.add_prefix = true;
    o.prefix = 3;
    d.close_options(&mut st, true);
    run(&mut st, &mut d);
    assert_eq!(listing(&dir), vec![format!("01_se.{ext}")]);
}

/// Anzahl Ebenen einer PSD-Datei (Abschnitt „Ebenen- und Maskeninfo“).
fn psd_layer_count(b: &[u8]) -> i16 {
    let be = |i: usize| u32::from_be_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]) as usize;
    let mut i = 26;
    i += 4 + be(i); // Farbmodusdaten
    i += 4 + be(i); // Bildressourcen
    i += 4 + 4; // Länge Ebenen-/Maskeninfo, Länge Ebeneninfo
    i16::from_be_bytes([b[i], b[i + 1]]).abs()
}

#[test]
fn psd_optionen_reduzieren_und_merken() {
    let mut st = fixture();
    let dir = tmp("psd-layers");
    let mut d = dialog(&st, FilesStyle::Psd, &dir, "ps");
    run(&mut st, &mut d);
    let layered = psd_layer_count(&std::fs::read(dir.join("ps.psd")).unwrap());
    assert!(layered >= 3, "{layered}");
    // „Kleinere Photoshop-Datei“ setzt „auf Hintergrundebene reduzieren“
    let dir = tmp("psd-flat");
    let mut d = dialog(&st, FilesStyle::Psd, &dir, "ps");
    d.open_options();
    let Some(OptionsDialog::Psd(o)) = d.options.as_mut() else { panic!() };
    o.set_settings(2);
    assert_eq!((o.objects, o.effects, o.vector, o.text), (1, 1, 1, 1));
    d.close_options(&mut st, true);
    run(&mut st, &mut d);
    assert_eq!(psd_layer_count(&std::fs::read(dir.join("ps.psd")).unwrap()), 1);
    // gemerkt (1-basiert)
    assert_eq!(st.session.prefs.get_int("PsdExport_Settings", 0), 3);
    assert_eq!(st.session.prefs.get_int("PsdExport_Layers", 0), 2);
    assert_eq!(ExportDialog::new(&st).unwrap().psd, d.psd);
    // „Benutzerdefiniert“ behält die Einzelwerte, eine Vorgabe setzt sie
    let mut p = PsdOpts::default();
    p.objects = 1;
    p.set_settings(3);
    assert_eq!(p.objects, 1);
    p.set_settings(0);
    assert_eq!((p.objects, p.effects), (0, 0));
}

/// Jeder Optionen-Dialog zeichnet in de/en ohne Panik; Esc schließt nur den Optionen-Dialog.
#[test]
fn optionen_dialoge_zeichnen() {
    for lang in ["de", "en", "qps"] {
        fp_i18n::set_language(lang);
        for kind in [FilesStyle::Pdf, FilesStyle::CssSprites, FilesStyle::FramesToFiles, FilesStyle::PagesToFiles, FilesStyle::Psd] {
            let st0 = fixture();
            let mut harness = egui_kittest::Harness::builder().with_size(egui::vec2(1100.0, 800.0)).build_ui_state(
                |ui, st: &mut AppState| {
                    fp_ui::dialogs::export::show_export(ui.ctx(), st);
                },
                st0,
            );
            let mut d = ExportDialog::new(harness.state()).unwrap();
            d.set_style(style_index(kind));
            d.open_options();
            harness.state_mut().dialogs.export = Some(d);
            harness.run_steps(3);
            harness.key_press(egui::Key::Escape);
            harness.run_steps(3);
            let d = harness.state().dialogs.export.clone().expect("Exportdialog bleibt offen");
            assert!(d.options.is_none(), "{kind:?} {lang}");
        }
    }
    fp_i18n::set_language("de");
}
