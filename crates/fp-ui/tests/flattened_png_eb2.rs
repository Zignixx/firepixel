//! EB-2: Dateityp „Reduziertes PNG“ in *Datei ▸ Speichern unter* (re\20 §5).
//! Formatabbildung aus den Optimierungseinstellungen, keine privaten Chunks, Pixel gleich dem
//! normalen PNG-Export, „Als Kopie speichern“ ja/nein mit Pfad und Art des Dokuments danach,
//! Typwahl über den Filter statt nur über die Endung, Zusatzfelder des Dialogs (de/en).
//! Wie alle Testläufe mit `FP_NO_SHELL_OPEN=1` starten (AGENTEN_REGELN §4a).

use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use fp_export::options::{ColorMode, ExportFormat, OptimizeSettings};
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::file_dialog::{save_extras_ui, CopyBox, SaveChoice, TypeExtras};
use fp_ui::files::{self, DialogKind, SaveKind};
use serde_json::json;
use std::path::{Path, PathBuf};

fn state() -> AppState {
    fp_engine::shell::disable();
    let session = fp_ui::ui_commands::new_session(fp_engine::Preferences::new());
    let mut st = AppState::new(session, UiSettings::default(), None);
    st.sync_render = true;
    st.os_clipboard = false;
    st
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fp-eb2-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Zwei Ebenen mit Rechteck und halbtransparenter Ellipse auf Weiß, Optimierung `format`.
fn document(st: &mut AppState, format: &str) {
    st.session.execute("file.new", json!({"width": 60, "height": 40, "background": "#ffffff"})).unwrap();
    st.session.execute("draw.rectangle", json!({"rect": [5, 5, 40, 30], "fillColor": "#3366cc", "strokeWidth": 0})).unwrap();
    st.session.execute("layer.add", json!({})).unwrap();
    st.session.execute("draw.ellipse", json!({"rect": [20, 10, 55, 38], "fillColor": "#ff000080", "strokeWidth": 0})).unwrap();
    st.session.execute("file.imagePreview", json!({"settings": {"format": format}, "apply": true})).unwrap();
}

fn flat_index() -> usize {
    files::save_type_index(SaveKind::FlatPng).unwrap()
}

/// Speichert über den Weg des Dialogs „Speichern unter“ (Typ „Reduziertes PNG“).
fn save_flat(st: &mut AppState, path: &Path, copy: bool) {
    assert!(files::save_chosen(st, SaveChoice { path: path.to_path_buf(), filter: Some(flat_index()), copy }));
}

fn chunks(bytes: &[u8]) -> Vec<fp_fwpng::png::Chunk> {
    fp_fwpng::png::read_chunks(bytes).unwrap()
}

fn names(bytes: &[u8]) -> Vec<String> {
    chunks(bytes).iter().map(|c| String::from_utf8_lossy(&c.kind).to_string()).collect()
}

fn rgba(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let i = image::load_from_memory(bytes).unwrap().to_rgba8();
    (i.width(), i.height(), i.into_raw())
}

fn layer_count(st: &AppState) -> usize {
    let d = st.session.active().unwrap();
    d.doc.pages[d.page].root.layers.len()
}

#[test]
fn format_mapping() {
    let s = |format, color_mode| OptimizeSettings { format, color_mode, ..OptimizeSettings::default() };
    let cases = [
        (s(ExportFormat::Png, ColorMode::Rgba32), "png32"),
        (s(ExportFormat::Png, ColorMode::Rgb24), "png24"),
        (s(ExportFormat::Png, ColorMode::Indexed), "png8"),
        (s(ExportFormat::Gif, ColorMode::Indexed), "png8"),
        (s(ExportFormat::AnimatedGif, ColorMode::Indexed), "png8"),
        (s(ExportFormat::Jpeg, ColorMode::Rgb24), "png32"),
        (s(ExportFormat::Tiff, ColorMode::Rgba32), "png32"),
        (s(ExportFormat::Bmp, ColorMode::Rgb24), "png32"),
        (s(ExportFormat::Wbmp, ColorMode::Indexed), "png32"),
        (s(ExportFormat::WebP, ColorMode::Rgba32), "png32"),
    ];
    for (set, want) in &cases {
        assert_eq!(fp_export::flat_png::flattened_png_kind(set), *want, "{:?}/{:?}", set.format, set.color_mode);
        assert_eq!(fp_export::flat_png::flattened_png_settings(set).format, ExportFormat::Png);
    }
    // GIF-Paletteneinstellungen bleiben (Farbanzahl, Dither)
    let mut g = s(ExportFormat::Gif, ColorMode::Indexed);
    g.max_colors = 17;
    assert_eq!(fp_export::flat_png::flattened_png_settings(&g).max_colors, 17);

    // in der Datei: Farbtyp nach der Abbildung (IHDR Byte 9), sBIT wie §5.2
    let dir = tmp("map");
    for (format, color_type, sbit) in [("png32", 6u8, vec![8u8, 8, 8, 8]), ("png24", 2, vec![8, 8, 8]), ("gif", 3, vec![8, 8, 8]), ("png8", 3, vec![8, 8, 8]), ("jpeg", 6, vec![8, 8, 8, 8])] {
        let mut st = state();
        document(&mut st, format);
        let p = dir.join(format!("{format}.png"));
        save_flat(&mut st, &p, true);
        let c = chunks(&std::fs::read(&p).unwrap());
        assert_eq!(c[0].data[9], color_type, "{format}: Farbtyp");
        let sb = c.iter().find(|c| &c.kind == b"sBIT").map(|c| c.data.clone());
        assert_eq!(sb, Some(sbit), "{format}: sBIT");
        assert_eq!(c.iter().any(|c| &c.kind == b"PLTE"), color_type == 3, "{format}: PLTE");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn no_private_chunks() {
    let dir = tmp("chunks");
    let mut st = state();
    document(&mut st, "png32");
    let p = dir.join("flach.png");
    save_flat(&mut st, &p, true);
    let bytes = std::fs::read(&p).unwrap();
    assert_eq!(names(&bytes), ["IHDR", "sBIT", "pHYs", "tEXt", "tEXt", "IDAT", "IEND"]);
    assert!(!fp_fwpng::is_fwpng(&bytes));
    let texts: Vec<(String, String)> = chunks(&bytes)
        .iter()
        .filter(|c| &c.kind == b"tEXt")
        .map(|c| {
            let i = c.data.iter().position(|b| *b == 0).unwrap();
            (String::from_utf8_lossy(&c.data[..i]).to_string(), String::from_utf8_lossy(&c.data[i + 1..]).to_string())
        })
        .collect();
    assert_eq!(texts[0].0, "Creation Time");
    assert_eq!(texts[0].1, fp_export::flat_png::creation_time_today(), "neues Dokument: heute (MM/DD/YY)");
    assert_eq!(texts[1], ("Software".to_string(), "Firepixel".to_string()));
    // pHYs wie beim PNG-Export (72 dpi, Einheit 1). Das Original schreibt 2834 Pixel/m (abgeschnitten),
    // der gemeinsame PNG-Schreiber rundet auf 2835 – Befund an RF, gilt für beide Wege gleich.
    let phys = chunks(&bytes).into_iter().find(|c| &c.kind == b"pHYs").unwrap();
    let ppm = u32::from_be_bytes([phys.data[0], phys.data[1], phys.data[2], phys.data[3]]);
    assert!((2834..=2835).contains(&ppm) && phys.data[8] == 1, "{:?}", phys.data);
    let exp = dir.join("export.png");
    st.session.execute("file.export", json!({"path": exp.to_string_lossy()})).unwrap();
    let phys_export = chunks(&std::fs::read(&exp).unwrap()).into_iter().find(|c| &c.kind == b"pHYs").unwrap();
    assert_eq!(phys.data, phys_export.data);

    // PNG 8 (GIF-Einstellung): PLTE, keine privaten Chunks
    let mut st = state();
    document(&mut st, "gif");
    let p8 = dir.join("flach8.png");
    save_flat(&mut st, &p8, true);
    let n8 = names(&std::fs::read(&p8).unwrap());
    let allowed = ["IHDR", "sBIT", "PLTE", "tRNS", "pHYs", "tEXt", "IDAT", "IEND"];
    assert!(n8.iter().all(|n| allowed.contains(&n.as_str())), "{n8:?}");
    assert_eq!(n8.iter().filter(|n| *n == "tEXt").count(), 2);

    // geöffnetes Dokument mit eigenem Erstellungsdatum: das wird übernommen
    let mut st = state();
    document(&mut st, "png32");
    st.session.doc_mut().unwrap().doc_mut().file_meta.text_chunks.push(("Creation Time".into(), "10/17/06".into()));
    let pc = dir.join("datum.png");
    save_flat(&mut st, &pc, true);
    let b = std::fs::read(&pc).unwrap();
    let t = chunks(&b).into_iter().find(|c| &c.kind == b"tEXt").unwrap();
    assert_eq!(t.data, b"Creation Time\x0010/17/06");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn pixels_equal_png_export() {
    let dir = tmp("pixel");
    for (format, export_settings) in [("png32", json!(null)), ("png24", json!(null)), ("png8", json!(null)), ("gif", json!({"format": "png8"}))] {
        let mut st = state();
        document(&mut st, format);
        let flat = dir.join(format!("{format}-flach.png"));
        save_flat(&mut st, &flat, true);
        let exp = dir.join(format!("{format}-export.png"));
        let mut args = json!({"path": exp.to_string_lossy()});
        if !export_settings.is_null() {
            args["settings"] = export_settings;
        }
        st.session.execute("file.export", args).unwrap();
        let a = rgba(&std::fs::read(&flat).unwrap());
        let b = rgba(&std::fs::read(&exp).unwrap());
        assert_eq!((a.0, a.1), (60, 40), "{format}");
        assert!(a == b, "{format}: Pixel weichen vom PNG-Export ab");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_as_copy_yes_no() {
    let dir = tmp("copy");
    let mut st = state();
    document(&mut st, "png32");
    let title = st.session.active().unwrap().title.clone();
    let layers = layer_count(&st);
    assert!(layers >= 2);

    // mit „Als Kopie speichern“: Datei da, Dokument unverändert (Name, Pfad, Art, geändert)
    let kopie = dir.join("kopie.png");
    save_flat(&mut st, &kopie, true);
    assert!(kopie.is_file());
    let d = st.session.active().unwrap();
    assert_eq!((d.path.clone(), d.title.clone(), d.flattened, d.is_dirty()), (None, title.clone(), false, true));
    assert!(st.status.as_ref().is_some_and(|(s, _)| s.contains("kopie.png")), "Statuszeile meldet die Kopie");

    // ohne: das Dokument gehört zur flachen PNG (Titel, Pfad, Art), nicht geändert, Ebenen bleiben
    let flach = dir.join("flach.png");
    save_flat(&mut st, &flach, false);
    let d = st.session.active().unwrap();
    assert_eq!((d.path.clone(), d.title.as_str(), d.flattened, d.is_dirty()), (Some(flach.clone()), "flach.png", true, false));
    assert_eq!(layer_count(&st), layers, "Ebenen bleiben im geöffneten Dokument");
    assert!(!flach.with_file_name("flach.fw.png").exists(), "kein .fw.png-Anhang");
    let before = rgba(&std::fs::read(&flach).unwrap());

    // Strg+S schreibt wieder flach (Art bleibt), mit dem neuen Inhalt
    st.session.execute("draw.rectangle", json!({"rect": [0, 0, 10, 10], "fillColor": "#00ff00", "strokeWidth": 0})).unwrap();
    assert!(st.session.active().unwrap().is_dirty());
    let saved = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let s2 = std::sync::Arc::clone(&saved);
    files::save_active(&mut st, false, move |_, ok| s2.store(ok, std::sync::atomic::Ordering::SeqCst));
    assert!(saved.load(std::sync::atomic::Ordering::SeqCst));
    let bytes = std::fs::read(&flach).unwrap();
    assert!(!fp_fwpng::is_fwpng(&bytes), "Strg+S nach Reduziertes PNG bleibt flach");
    let after = rgba(&bytes);
    assert_ne!(before, after);
    assert_eq!(&after.2[..4], &[0, 255, 0, 255], "neuer Inhalt in der Datei");
    assert!(!st.session.active().unwrap().is_dirty());

    // wieder als FW-PNG gespeichert: Art zurück, `.fw.png` angehängt
    assert!(files::save_chosen(&mut st, SaveChoice { path: dir.join("wieder.png"), filter: Some(0), copy: false }));
    let d = st.session.active().unwrap();
    assert_eq!((d.path.clone(), d.flattened), (Some(dir.join("wieder.fw.png")), false));
    assert!(fp_fwpng::is_fwpng(&std::fs::read(dir.join("wieder.fw.png")).unwrap()));

    // FW-PNG als Kopie: Dokument behält Namen
    assert!(files::save_chosen(&mut st, SaveChoice { path: dir.join("fwkopie.fw.png"), filter: Some(0), copy: true }));
    assert!(dir.join("fwkopie.fw.png").is_file());
    assert_eq!(st.session.active().unwrap().path.clone(), Some(dir.join("wieder.fw.png")));

    // Rückgängig nach dem flachen Speichern wirkt weiter auf das Dokument mit Ebenen
    st.session.execute("edit.undo", json!({})).unwrap();
    assert_eq!(layer_count(&st), layers);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn engine_save_as_flattened_param() {
    // derselbe Weg für Agenten: file.saveAs {flattened, copy}, danach file.save wieder flach
    let dir = tmp("engine");
    let mut st = state();
    document(&mut st, "gif");
    let p = dir.join("agent.png");
    let out = st.session.execute("file.saveAs", json!({"path": p.to_string_lossy(), "flattened": true})).unwrap();
    assert_eq!(out.value["flattened"], json!(true));
    assert_eq!(chunks(&std::fs::read(&p).unwrap())[0].data[9], 3);
    st.session.execute("file.save", json!({})).unwrap();
    assert!(!fp_fwpng::is_fwpng(&std::fs::read(&p).unwrap()));
    // ohne Parameter bleibt file.saveAs FW-PNG (rückwärtskompatibel)
    let q = dir.join("agent.fw.png");
    st.session.execute("file.saveAs", json!({"path": q.to_string_lossy()})).unwrap();
    assert!(fp_fwpng::is_fwpng(&std::fs::read(&q).unwrap()));
    assert!(!st.session.active().unwrap().flattened);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn type_by_filter_not_only_extension() {
    let f = Some(flat_index());
    let jpeg = files::save_type_index(SaveKind::Export("jpeg"));
    // ohne Typ (Testvorgabe, Windows-Dialog ohne Nachfrage): `.png` bleibt FW-PNG
    assert_eq!(files::save_kind_for(Path::new("a.png"), None), SaveKind::FwPng);
    assert_eq!(files::save_kind_for(Path::new("a.png"), Some(0)), SaveKind::FwPng);
    assert_eq!(files::save_kind_for(Path::new("a.png"), f), SaveKind::FlatPng);
    assert_eq!(files::save_kind_for(Path::new("a.fw.png"), f), SaveKind::FlatPng);
    assert_eq!(files::save_kind_for(Path::new("a"), f), SaveKind::FlatPng);
    // Endung passt nicht zum Typ: die Endung entscheidet
    assert_eq!(files::save_kind_for(Path::new("a.jpg"), f), SaveKind::Export("jpeg"));
    assert_eq!(files::save_kind_for(Path::new("a"), jpeg), SaveKind::Export("jpeg"));
    // Ziele: Reduziertes PNG ohne `.fw.png`, FW-PNG mit
    assert_eq!(files::save_target_for(PathBuf::from("a.fw.png"), SaveKind::FlatPng), PathBuf::from("a.png"));
    assert_eq!(files::save_target_for(PathBuf::from("a.PNG"), SaveKind::FlatPng), PathBuf::from("a.PNG"));
    assert_eq!(files::save_target_for(PathBuf::from("a"), SaveKind::FlatPng), PathBuf::from("a.png"));
    assert_eq!(files::save_target_for(PathBuf::from("a.png"), SaveKind::FwPng), PathBuf::from("a.fw.png"));
    assert_eq!(files::save_target_for(PathBuf::from("a"), SaveKind::Export("jpeg")), PathBuf::from("a.jpg"));
    // Speichern als `.png` ohne Typ (Testvorgabe) bleibt wie bisher FW-PNG mit `.fw.png`
    let dir = tmp("filter");
    let mut st = state();
    document(&mut st, "png32");
    assert!(files::save_chosen(&mut st, SaveChoice { path: dir.join("x.png"), filter: None, copy: false }));
    assert_eq!(st.session.active().unwrap().path.clone(), Some(dir.join("x.fw.png")));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn dialog_types_and_extras() {
    let r = DialogKind::SaveAs.request("a.fw.png");
    let x = r.save_extras.as_ref().expect("Zusatzfelder bei Speichern unter");
    assert_eq!(x.types.len(), r.filters.len());
    let i = flat_index();
    assert_eq!(r.filters[i].0, "Reduziertes PNG (*.png)");
    assert_eq!(r.filters[i].1, ["png"]);
    assert_eq!(i, 4, "nach JPEG wie im Original");
    assert_eq!(x.types[i], TypeExtras { copy: CopyBox::Off, options: true, hint: true });
    assert_eq!(x.types[0], TypeExtras { copy: CopyBox::Off, options: false, hint: false });
    let j = files::save_type_index(SaveKind::Export("jpeg")).unwrap();
    assert_eq!(x.types[j].copy, CopyBox::Locked);
    // Nachfrage nach dem Windows-Dialog nur bei mehrdeutigem `.png`
    assert_eq!((x.ambiguous)(Path::new("C:\\a\\b.png")), vec![0, i]);
    assert!((x.ambiguous)(Path::new("b.fw.png")).is_empty());
    assert!((x.ambiguous)(Path::new("b.jpg")).is_empty());
    // andere Dialoge ohne Zusatzfelder
    assert!(DialogKind::SaveAsTemplate.request("a.fw.png").save_extras.is_none());
    assert!(DialogKind::Open.request("").save_extras.is_none());

    fp_i18n::set_language("en");
    let r = DialogKind::SaveAs.request("a.fw.png");
    assert_eq!(r.filters[i].0, "Flattened PNG (*.png)");
    fp_i18n::set_language("de");
}

/// Zusatzfelder als Oberfläche: Kästchen schaltet, „Optionen…“ meldet den Klick, gesperrtes
/// Kästchen bleibt an; Texte de/en.
#[test]
fn extras_ui_kittest() {
    for lang in ["de", "en"] {
        fp_i18n::set_language(lang);
        let flat = TypeExtras { copy: CopyBox::Off, options: true, hint: true };
        let mut h = Harness::builder().with_size(egui::vec2(520.0, 160.0)).build_ui_state(
            move |ui, s: &mut (bool, usize)| {
                if save_extras_ui(ui, flat, &mut s.0) {
                    s.1 += 1;
                }
            },
            (false, 0usize),
        );
        h.run();
        let hint = fp_i18n::t("fdlg-flatten-hint");
        assert!(h.query_by_label(&hint).is_some(), "{lang}: Hinweis sichtbar");
        h.get_by_label(&fp_i18n::t("fdlg-save-as-copy")).click();
        h.run();
        assert!(h.state().0, "{lang}: Kästchen an");
        h.get_by_label(&fp_i18n::t("fdlg-save-as-copy")).click();
        h.run();
        assert!(!h.state().0, "{lang}: Kästchen wieder aus");
        h.get_by_label(&fp_i18n::t("fdlg-options")).click();
        h.run();
        assert_eq!(h.state().1, 1, "{lang}: Optionen… geklickt");

        // gesperrt (Exportformate): an, Klick ändert nichts; ohne Hinweis bei FW-PNG
        let locked = TypeExtras { copy: CopyBox::Locked, options: false, hint: false };
        let mut h = Harness::builder().with_size(egui::vec2(520.0, 160.0)).build_ui_state(
            move |ui, s: &mut (bool, usize)| {
                if save_extras_ui(ui, locked, &mut s.0) {
                    s.1 += 1;
                }
            },
            (false, 0usize),
        );
        h.run();
        assert!(h.query_by_label(&hint).is_none());
        h.get_by_label(&fp_i18n::t("fdlg-options")).click();
        h.run();
        assert_eq!(h.state().1, 0, "{lang}: Optionen… gesperrt");
    }
    fp_i18n::set_language("de");
}
