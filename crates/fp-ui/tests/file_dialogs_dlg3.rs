//! DLG-3 (P-DLG3, P-HG1): Datei-Dialoge wie im Original gemessen (Startordner je Art, Titel,
//! Filter und deren Reihenfolge, Brücken-Dialog mit denselben Angaben) und der Schließen-Ablauf
//! (Schließen von außen mit und ohne ungespeicherte Dokumente, Strg+W ohne Verzögerung).

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fp_engine::Preferences;
use fp_engine::prefs::PrefValue;
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use fp_ui::file_dialog::Kind;
use fp_ui::files::{self, DialogKind};
use serde_json::json;
use std::path::{Path, PathBuf};

type H = Harness<'static, FirepixelApp>;

/// Die Sprache ist prozessweit: Tests, die sie setzen oder deutsche Texte prüfen, nacheinander.
static LANG: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn lang(code: &str) -> std::sync::MutexGuard<'static, ()> {
    let g = LANG
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    fp_i18n::set_language(code);
    g
}

/// Eigener Temp-Ordner je Test mit den Unterordnern `doc`, `open`, `import`, `save`.
fn dirs(name: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!("fp-dlg3-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    for d in ["doc", "open", "import", "save"] {
        std::fs::create_dir_all(base.join(d)).expect("Testordner");
    }
    base
}

fn prefs_with(pairs: &[(&str, &Path)]) -> Preferences {
    let mut p = Preferences::new();
    for (k, v) in pairs {
        p.set(k, PrefValue::Str(v.display().to_string()));
    }
    p
}

#[test]
fn start_folder_open_prefers_last_open_location_then_document_then_documents() {
    let b = dirs("open");
    let doc = b.join("doc").join("x.fw.png");
    // gemessen: LastOpenLocation gewinnt vor dem Ordner des Dokuments
    let p = prefs_with(&[("LastOpenLocation", &b.join("open"))]);
    assert_eq!(
        files::start_dir(&p, DialogKind::Open, Some(&doc)),
        Some(b.join("open"))
    );
    assert_eq!(
        files::start_dir(&p, DialogKind::UpdateHtml, Some(&doc)),
        Some(b.join("open"))
    );
    // ohne gemerkten Ordner: Ordner des Dokuments
    let none = Preferences::new();
    assert_eq!(
        files::start_dir(&none, DialogKind::Open, Some(&doc)),
        Some(b.join("doc"))
    );
    // gemerkter Ordner existiert nicht mehr → Rückfall
    let gone = prefs_with(&[("LastOpenLocation", &b.join("weg"))]);
    assert_eq!(
        files::start_dir(&gone, DialogKind::Open, Some(&doc)),
        Some(b.join("doc"))
    );
    // weder noch: „Dokumente“
    assert_eq!(
        files::start_dir(&none, DialogKind::Open, None),
        files::documents_dir()
    );
    assert!(files::documents_dir().is_some());
    let _ = std::fs::remove_dir_all(&b);
}

#[test]
fn start_folder_import_and_save_as_like_the_original() {
    let b = dirs("import");
    let doc = b.join("doc").join("x.fw.png");
    // Importieren: LastImportLocation, sonst LastOpenLocation, sonst Dokumentordner
    let p = prefs_with(&[
        ("LastImportLocation", &b.join("import")),
        ("LastOpenLocation", &b.join("open")),
    ]);
    assert_eq!(
        files::start_dir(&p, DialogKind::Import, Some(&doc)),
        Some(b.join("import"))
    );
    let p = prefs_with(&[("LastOpenLocation", &b.join("open"))]);
    assert_eq!(
        files::start_dir(&p, DialogKind::Import, Some(&doc)),
        Some(b.join("open"))
    );
    assert_eq!(
        files::start_dir(&Preferences::new(), DialogKind::Import, Some(&doc)),
        Some(b.join("doc"))
    );
    // Speichern unter: gespeichertes Dokument → dessen Ordner, auch wenn LastSaveLocation woanders ist
    let p = prefs_with(&[("LastSaveLocation", &b.join("save"))]);
    assert_eq!(
        files::start_dir(&p, DialogKind::SaveAs, Some(&doc)),
        Some(b.join("doc"))
    );
    // unbenanntes Dokument → LastSaveLocation
    assert_eq!(
        files::start_dir(&p, DialogKind::SaveAs, None),
        Some(b.join("save"))
    );
    let _ = std::fs::remove_dir_all(&b);
}

#[test]
fn remember_dir_writes_the_key_of_the_kind_only_once() {
    let b = dirs("remember");
    let mut p = Preferences::new();
    let f = b.join("open").join("bild.png");
    assert!(files::remember_dir(&mut p, DialogKind::Open, &f));
    assert_eq!(
        p.get_str("LastOpenLocation", ""),
        b.join("open").display().to_string()
    );
    // dieselbe Wahl noch einmal: keine Änderung (kein unnötiges Speichern)
    assert!(!files::remember_dir(&mut p, DialogKind::Open, &f));
    assert!(files::remember_dir(
        &mut p,
        DialogKind::SaveAs,
        &b.join("save").join("a.fw.png")
    ));
    assert!(files::remember_dir(
        &mut p,
        DialogKind::Import,
        &b.join("import").join("a.psd")
    ));
    assert_eq!(
        p.get_str("LastSaveLocation", ""),
        b.join("save").display().to_string()
    );
    assert_eq!(
        p.get_str("LastImportLocation", ""),
        b.join("import").display().to_string()
    );
    // Vorlagen und HTML merken nichts
    assert!(!files::remember_dir(
        &mut p,
        DialogKind::NewFromTemplate,
        &f
    ));
    assert!(!files::remember_dir(&mut p, DialogKind::UpdateHtml, &f));
    assert_eq!(
        p.get_str("LastOpenLocation", ""),
        b.join("open").display().to_string()
    );
    let _ = std::fs::remove_dir_all(&b);
}

fn names(r: &fp_ui::file_dialog::Request) -> Vec<String> {
    r.filters.iter().map(|(n, _)| n.clone()).collect()
}

#[test]
fn titles_filters_and_order_per_kind() {
    let _l = lang("de");
    // D03 Öffnen: Mehrfachauswahl, Einzelformate in der Reihenfolge des Originals, dann
    // „Alle lesbaren Dateien“ (vorgewählt) und „Alle Dateien“
    let r = DialogKind::Open.request("");
    assert_eq!(r.kind, Kind::OpenMany);
    assert_eq!(r.title, "Öffnen");
    let n = names(&r);
    let pos = |s: &str| {
        n.iter()
            .position(|x| x.starts_with(s))
            .unwrap_or_else(|| panic!("{s} fehlt in {n:?}"))
    };
    assert_eq!(n[0], "FW-PNG (*.png)");
    let order = [
        "FW-PNG", "PSD (", "GIF", "JPEG (", "WBMP", "TIFF", "Targa", "BMP", "AI (", "EPS",
    ];
    for w in order.windows(2) {
        assert!(
            pos(w[0]) < pos(w[1]),
            "{} vor {} erwartet: {n:?}",
            w[0],
            w[1]
        );
    }
    assert_eq!(n[n.len() - 2], "Alle lesbaren Dateien");
    assert_eq!(n[n.len() - 1], "Alle Dateien (*.*)");
    assert_eq!(r.default_filter, n.len() - 2);
    // nativer Dialog (rfd wählt den ersten Filter): der vorgewählte steht dort vorn
    let nf = r.native_filters();
    assert_eq!(nf[0].0, "Alle lesbaren Dateien");
    assert_eq!(nf[1].0, "FW-PNG (*.png)");
    assert_eq!(nf.len(), n.len());
    // D06 Importieren: dieselben Filter, eine Datei
    let i = DialogKind::Import.request("");
    assert_eq!((i.kind, i.title.as_str()), (Kind::Open, "Importieren"));
    assert_eq!(names(&i), n);
    // D02 Neu von Vorlage: eigener Titel, eine Datei, nur FW-PNG
    let t = DialogKind::NewFromTemplate.request("");
    assert_eq!((t.kind, t.title.as_str()), (Kind::Open, "Neu von Vorlage"));
    assert_eq!(names(&t), vec!["FW-PNG (*.png)"]);
    // D04 Speichern unter: FW-PNG voran, Reihenfolge des Originals
    let s = DialogKind::SaveAs.request("a.fw.png");
    assert_eq!(
        (s.kind, s.title.as_str(), s.file_name.as_deref()),
        (Kind::Save, "Speichern unter", Some("a.fw.png"))
    );
    let sn = names(&s);
    // EB-2: „Reduziertes PNG“ an seiner Stelle aus dem Original (nach JPEG, vor PSD)
    assert_eq!(
        &sn[..8],
        &[
            "FW-PNG (*.png)",
            "BMP (*.bmp)",
            "GIF (*.gif)",
            "JPEG (*.jpg; *.jpeg; *.jpe)",
            "Reduziertes PNG (*.png)",
            "PSD (*.psd)",
            "TIFF (*.tif; *.tiff)",
            "WBMP (*.wbmp)"
        ]
    );
    // D05 Als Vorlage speichern: im Original der Standardtitel „Speichern unter“, nur FW-PNG
    let v = DialogKind::SaveAsTemplate.request("a.fw.png");
    assert_eq!((v.kind, v.title.as_str()), (Kind::Save, "Speichern unter"));
    assert_eq!(names(&v), vec!["FW-PNG (*.png)"]);
    // D09 HTML aktualisieren
    let h = DialogKind::UpdateHtml.request("");
    assert_eq!(
        (h.kind, h.title.as_str()),
        (Kind::Open, "HTML-Datei suchen:")
    );
    assert_eq!(
        names(&h),
        vec!["HTML-Dateien (*.htm)", "Alle Dateien (*.*)"]
    );
}

#[test]
fn titles_in_english() {
    // Eigene Sprache nur für die Prüfung, danach wieder Deutsch
    let _l = lang("en");
    let open = DialogKind::Open.request("");
    let tpl = DialogKind::NewFromTemplate.request("");
    let html = DialogKind::UpdateHtml.request("");
    fp_i18n::set_language("de");
    assert_eq!(open.title, "Open");
    assert!(names(&open).contains(&"All readable files".to_string()));
    assert_eq!(tpl.title, "New from Template");
    assert_eq!(html.title, "Locate HTML File:");
}

fn harness(s: fp_engine::Session) -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(12)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(3);
    h
}

/// Dialog-Ablauf ohne Fenster (`FP_FILE_DIALOG=path:`): dieselbe Beschreibung wie für den nativen
/// und den Brücken-Dialog. Einziger Test dieser Datei mit der Umgebungsvariablen.
#[test]
fn dialog_request_carries_title_filters_start_folder_and_remembers_folder() {
    let _l = lang("de");
    let b = dirs("bridge");
    let src = b.join("doc").join("quelle.fw.png");
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 40, "height": 30}))
        .expect("doc");
    s.execute("file.saveAs", json!({"path": src.to_string_lossy()}))
        .expect("save");
    std::fs::copy(&src, b.join("open").join("zweite.fw.png")).expect("copy");
    s.prefs.set(
        "LastOpenLocation",
        PrefValue::Str(b.join("open").display().to_string()),
    );
    let mut h = harness(s);
    // nur dieser Test setzt den Testmodus
    set_mode(&format!("path:{}", b.join("open").display()));
    h.state_mut().state.run("file.open", json!({}));
    h.run_steps(3);
    let st = fp_ui::file_dialog::status_json();
    let ev = st["last_events"]
        .as_array()
        .and_then(|a| {
            a.iter()
                .find(|e| e["request"]["title"] == "Öffnen")
                .cloned()
        })
        .expect("Ereignis Öffnen");
    assert_eq!(
        ev["request"]["directory"],
        json!(b.join("open").display().to_string())
    );
    assert_eq!(ev["request"]["kind"], "open_many");
    let filters: Vec<String> = ev["request"]["filters"]
        .as_array()
        .expect("filters")
        .iter()
        .map(|f| f["name"].as_str().unwrap_or("").to_string())
        .collect();
    assert_eq!(filters[0], "FW-PNG (*.png)");
    assert_eq!(ev["request"]["default_filter"], json!(filters.len() - 2));
    // gewählt: zweite.fw.png ist offen, der Ordner bleibt gemerkt
    assert!(
        h.state()
            .state
            .session
            .documents()
            .iter()
            .any(|d| d.path.as_deref() == Some(b.join("open").join("zweite.fw.png").as_path()))
    );
    // Speichern unter des gespeicherten Dokuments: dessen Ordner, Titel „Speichern unter“
    let first = h
        .state()
        .state
        .session
        .documents()
        .iter()
        .find(|d| d.path.as_deref() == Some(src.as_path()))
        .map(|d| d.id)
        .expect("Quelle");
    let _ = h.state_mut().state.session.activate(first);
    set_mode("cancel");
    h.state_mut().state.run("file.saveAs", json!({}));
    h.run_steps(2);
    let st = fp_ui::file_dialog::status_json();
    let ev = st["last_events"]
        .as_array()
        .and_then(|a| a.iter().find(|e| e["request"]["kind"] == "save").cloned())
        .expect("Ereignis Speichern");
    assert_eq!(ev["request"]["title"], "Speichern unter");
    assert_eq!(
        ev["request"]["directory"],
        json!(b.join("doc").display().to_string())
    );
    assert_eq!(ev["request"]["file_name"], "quelle.fw.png");
    clear_env();
    let _ = std::fs::remove_dir_all(&b);
}

fn set_mode(v: &str) {
    fp_ui::file_dialog::set_test_mode(Some(v));
}

fn clear_env() {
    fp_ui::file_dialog::set_test_mode(None);
}

// ---------------------------------------------------------------- Schließen-Ablauf (P-HG1)

fn close_from_outside(h: &mut H) {
    h.input_mut()
        .viewports
        .entry(egui::ViewportId::ROOT)
        .or_default()
        .events
        .push(egui::ViewportEvent::Close);
}

#[test]
fn close_from_outside_without_documents_quits_at_once() {
    let mut h = harness(fp_engine::Session::default());
    close_from_outside(&mut h);
    h.run_steps(1);
    let st = &h.state().state;
    assert!(st.quit_confirmed, "ohne Dokumente sofort beenden");
    assert!(st.dialogs.unsaved.is_none());
}

#[test]
fn close_from_outside_with_clean_document_quits_and_with_dirty_document_asks() {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 40, "height": 30}))
        .expect("doc");
    let mut h = harness(s);
    close_from_outside(&mut h);
    h.run_steps(1);
    assert!(
        h.state().state.quit_confirmed,
        "unverändertes Dokument: ohne Rückfrage beenden"
    );

    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 40, "height": 30}))
        .expect("doc");
    s.execute("draw.rectangle", json!({"rect": [1, 1, 10, 10]}))
        .expect("rect");
    let mut h = harness(s);
    close_from_outside(&mut h);
    h.run_steps(1);
    assert!(
        !h.state().state.quit_confirmed,
        "ungespeichert: erst fragen"
    );
    assert!(h.state().state.dialogs.unsaved.is_some(), "Rückfrage offen");
    // zweites Schließen von außen, während die Rückfrage offen ist: keine zweite Rückfrage
    close_from_outside(&mut h);
    h.run_steps(1);
    assert_eq!(
        h.state()
            .state
            .dialogs
            .unsaved
            .as_ref()
            .map(|f| f.queue.len()),
        Some(1)
    );
}

#[test]
fn ctrl_w_shows_the_question_in_the_same_frame() {
    let _l = lang("de");
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 40, "height": 30}))
        .expect("doc");
    s.execute("draw.rectangle", json!({"rect": [1, 1, 10, 10]}))
        .expect("rect");
    let mut h = harness(s);
    let m = egui::Modifiers {
        ctrl: true,
        command: true,
        ..Default::default()
    };
    h.event(egui::Event::ModifiersChanged(m));
    h.event(egui::Event::Key {
        key: egui::Key::W,
        physical_key: Some(egui::Key::W),
        pressed: true,
        repeat: false,
        modifiers: m,
    });
    // ein Bild: die Taste wird verarbeitet und die Rückfrage im selben Bild gezeichnet (zweiter
    // Durchgang von egui), nicht erst im nächsten Bild
    h.step();
    assert!(
        h.state().state.dialogs.unsaved.is_some(),
        "Rückfrage angelegt"
    );
    assert!(
        h.query_by_label("Ja").is_some(),
        "Rückfrage mit Ja/Nein/Abbrechen im selben Bild sichtbar"
    );
    // Nein schließt ohne Speichern, wieder im selben Bild
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    h.query_by_label("Nein").expect("Nein").click();
    h.run_steps(2);
    assert!(h.state().state.session.documents().is_empty());
    assert!(h.state().state.dialogs.unsaved.is_none());
}

/// CT-3 (FIX-E): Schließen von außen bei **minimiertem** Fenster mit geändertem Dokument. eframe ruft
/// dann nur `logic` auf; das Fenster kommt zurück, und die Rückfrage muss im nächsten Bild stehen,
/// obwohl das Schließen-Ereignis dort nicht mehr ankommt. Wiederholt (§4b): eine Rückfrage.
#[test]
fn close_from_outside_while_minimized_restores_and_asks() {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 40, "height": 30}))
        .expect("doc");
    s.execute("draw.rectangle", json!({"rect": [1, 1, 10, 10]}))
        .expect("rect");
    let mut h = harness(s);
    let ctx = h.ctx.clone();
    assert!(
        h.state_mut().close_while_minimized(&ctx, true, true),
        "Schließwunsch übernommen"
    );
    // zweites WM_CLOSE, bevor das Fenster zurück ist
    assert!(h.state_mut().close_while_minimized(&ctx, true, true));
    h.run_steps(2);
    let st = &h.state().state;
    assert!(!st.quit_confirmed, "erst fragen");
    assert_eq!(
        st.dialogs.unsaved.as_ref().map(|f| f.queue.len()),
        Some(1),
        "genau eine Rückfrage"
    );
    // nicht minimiert bzw. ohne Änderungen: nichts zu tun (normaler Weg über root_ui)
    assert!(!h.state_mut().close_while_minimized(&ctx, false, true));
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 40, "height": 30}))
        .expect("doc");
    let mut h = harness(s);
    let ctx = h.ctx.clone();
    assert!(
        !h.state_mut().close_while_minimized(&ctx, true, true),
        "unverändert: eframe beendet direkt"
    );
}
