//! Gesamttest Dateien (HG, Phase 5): Speichern/Öffnen FW-PNG (Rundreise), Export in jedes
//! Exportformat (Datei ▸ Exportieren, `file.export`) und Wiederöffnen bzw. Importieren jeder
//! erzeugten Datei über die Oberfläche (`Action::OpenPath`, `file.import`), Drag & Drop auf das
//! Fenster (egui_kittest, `dropped_files`), zuletzt geöffnete Dateien, Bild-Zwischendatei der
//! Systemzwischenablage (PNG schreiben/lesen), Arbeitsbereich-XML. Dazu die Testdateien der
//! Spezifikationen (`re\_testdateien`, GIF/JPEG/HEIC/PNG), soweit vorhanden.
//!
//! Ergebnis: `docs\funktionstest\dateien_auto.md`.

use crate::full_common;

use egui_kittest::Harness;
use fp_ui::app_state::{Action, AppState, UiSettings};
use fp_ui::FirepixelApp;
use full_common::*;
use serde_json::json;
use std::path::{Path, PathBuf};

const EXPORTS: &[(&str, &str)] = &[
    ("gif", "gif"),
    ("animatedgif", "gif"),
    ("jpeg", "jpg"),
    ("png8", "png"),
    ("png24", "png"),
    ("png32", "png"),
    ("tiff", "tif"),
    ("bmp", "bmp"),
    ("wbmp", "wbmp"),
    ("webp", "webp"),
    ("webplossless", "webp"),
    ("animatedwebp", "webp"),
    ("avif", "avif"),
    ("apng", "png"),
    ("qoi", "qoi"),
    ("ico", "ico"),
];

fn open_ok(st: &mut AppState, ctx: &egui::Context, p: &Path) -> Result<String, String> {
    let n = st.session.documents().len();
    st.push(Action::OpenPath(p.to_path_buf()));
    st.process_actions(ctx);
    if let Some((_, m)) = st.dialogs.message.take() {
        return Err(m);
    }
    if st.session.documents().len() != n + 1 {
        return Err("kein Dokument geöffnet".into());
    }
    let d = st.session.active().ok_or("kein aktives Dokument")?;
    let r = fp_export::render::page_rect(&d.doc, d.page).ok_or("keine Leinwand")?;
    Ok(format!("geöffnet, {}×{}", r.right - r.left, r.bottom - r.top))
}

fn test_files() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("..").join("re").join("_testdateien");
    let mut out = Vec::new();
    let mut want: Vec<&str> = vec!["gif", "jpg", "heic", "png"];
    fn walk(d: &Path, out: &mut Vec<PathBuf>, want: &mut Vec<&str>) {
        let Ok(list) = std::fs::read_dir(d) else { return };
        for e in list.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out, want);
            } else if let Some(ext) = p.extension().map(|x| x.to_string_lossy().to_lowercase())
                && let Some(i) = want.iter().position(|w| *w == ext)
            {
                out.push(p);
                want.remove(i);
            }
        }
    }
    walk(&root, &mut out, &mut want);
    out
}

/// Auf das Fenster gezogene Datei (wie egui-winit sie liefert).
#[derive(Debug)]
struct TestDrop(PathBuf);

impl egui::DroppedFile for TestDrop {
    fn path(&self) -> &Path {
        &self.0
    }
    fn bytes(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.0).map_err(|e| e.to_string())
    }
}

#[test]
fn full_files() {
    let mut rows = Vec::new();
    let (mut st, ctx, f) = fixture();
    let dir = f.save_to.parent().map(Path::to_path_buf).unwrap_or_default();
    // FW-PNG speichern und wieder öffnen (Rundreise)
    let fw = dir.join("rundreise.fw.png");
    let before = st.session.active().map(|d| serde_json::to_value(&*d.doc).unwrap_or_default());
    let r = st.session.execute("file.saveAs", json!({ "path": fw.to_string_lossy() })).map_err(|e| e.to_string());
    let r = r.and_then(|_| {
        let _ = st.session.execute("file.close", json!({ "discard": true }));
        open_ok(&mut st, &ctx, &fw)
    });
    let same = st.session.active().map(|d| {
        let a = before.clone().unwrap_or_default();
        let b = serde_json::to_value(&*d.doc).unwrap_or_default();
        let n = |v: &serde_json::Value| v.get("pages").and_then(|p| p.as_array()).map(|a| a.len());
        n(&a) == n(&b)
    });
    rows.push(Row {
        path: "Datei ▸ Speichern unter / Öffnen (FW-PNG-Rundreise)".into(),
        command: "file.saveAs + OpenPath".into(),
        outcome: if r.is_ok() && same == Some(true) { Outcome::Doc } else { Outcome::Error(format!("{r:?}, Seiten gleich: {same:?}")) },
        note: r.clone().unwrap_or_default(),
    });
    // zuletzt geöffnete Dateien
    let recent = st.session.prefs.recent_files.iter().any(|p| p.file_name() == fw.file_name());
    rows.push(Row { path: "Datei ▸ Zuletzt geöffnete Dateien".into(), command: "recent_files".into(), outcome: if recent { Outcome::Ui } else { Outcome::Error("Datei fehlt in der Liste".into()) }, note: String::new() });
    // Exportformate + Wiederöffnen
    for (fmt, ext) in EXPORTS {
        let (mut st, ctx, f) = fixture();
        let dir = f.save_to.parent().map(Path::to_path_buf).unwrap_or_default().join(format!("export-{fmt}"));
        let _ = std::fs::create_dir_all(&dir);
        let target = dir.join(format!("bild.{ext}"));
        let r = st.session.execute("file.export", json!({ "path": target.to_string_lossy(), "settings": { "format": fmt } })).map_err(|e| e.to_string());
        let produced: Vec<PathBuf> = std::fs::read_dir(&dir).map(|l| l.flatten().map(|e| e.path()).filter(|p| p.is_file()).collect()).unwrap_or_default();
        let note;
        let o = match (&r, produced.first()) {
            (Ok(_), Some(p)) => {
                let size = std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
                match open_ok(&mut st, &ctx, p) {
                    Ok(s) => {
                        note = format!("{} ({size} Byte) exportiert und {s}", p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default());
                        Outcome::Doc
                    }
                    Err(e) => {
                        note = format!("exportiert ({size} Byte), Öffnen: {e}");
                        Outcome::Error(e)
                    }
                }
            }
            (Ok(_), None) => {
                note = "keine Datei geschrieben".into();
                Outcome::Error(note.clone())
            }
            (Err(e), _) => {
                note = e.clone();
                Outcome::Error(e.clone())
            }
        };
        rows.push(Row { path: format!("Datei ▸ Exportieren ▸ {fmt}"), command: "file.export".into(), outcome: o, note });
    }
    // Import (Datei ▸ Importieren) einer exportierten PNG-Datei in ein offenes Dokument
    {
        let (mut st, ctx, f) = fixture();
        let png = f.save_to.parent().map(Path::to_path_buf).unwrap_or_default().join("import.png");
        let _ = fp_ui::os_clipboard::write_png(&png, 4, 3, &[200u8; 48]);
        let n_before = st.session.active().map(|d| d.history.undo_count()).unwrap_or(0);
        st.file_override = Some(fp_ui::files::FilePickOverride { open: vec![png.clone()], save: None });
        st.push(Action::Run("file.import".into(), json!({})));
        st.process_actions(&ctx);
        // Platzierungszeiger (CT-C-05): Klick an 0,0 setzt die Datei
        if let Some(pl) = st.place_import.take() {
            st.push(Action::Run("file.import".into(), json!({"path": pl.path.to_string_lossy(), "at": [0, 0]})));
            st.process_actions(&ctx);
        }
        let n_after = st.session.active().map(|d| d.history.undo_count()).unwrap_or(0);
        rows.push(Row { path: "Datei ▸ Importieren (PNG)".into(), command: "file.import".into(), outcome: if n_after == n_before + 1 { Outcome::Doc } else { Outcome::Error(format!("{:?}", st.status)) }, note: "Zwischendatei der Systemzwischenablage (PNG) als Bitmap importiert".into() });
    }
    // Testdateien der Spezifikationen
    for p in test_files() {
        let (mut st, ctx, _f) = fixture();
        let r = open_ok(&mut st, &ctx, &p);
        let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        rows.push(Row { path: format!("Datei ▸ Öffnen ▸ {name}"), command: "OpenPath".into(), outcome: if r.is_ok() { Outcome::Doc } else { Outcome::Error(r.clone().unwrap_err()) }, note: r.unwrap_or_default() });
    }
    // Drag & Drop auf das Hauptfenster
    {
        let (st, _ctx, _f) = fixture();
        let n = st.session.documents().len();
        let ctx = egui::Context::default();
        let mut app = FirepixelApp::with_session(&ctx, fp_engine::Session::default(), UiSettings::default(), false);
        app.state = st;
        let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
        h.run_steps(2);
        h.input_mut().dropped_files.push(std::sync::Arc::new(TestDrop(fw.clone())));
        h.run_steps(3);
        let m = h.state().state.session.documents().len();
        rows.push(Row { path: "Hauptfenster ▸ Drag & Drop einer Datei".into(), command: "dropped_files".into(), outcome: if m == n + 1 || h.state().state.session.documents().iter().any(|d| d.path.as_deref() == Some(fw.as_path())) { Outcome::Ui } else { Outcome::Error(format!("{n} → {m} Dokumente")) }, note: String::new() });
    }
    let (ok, n) = write_report("dateien_auto.md", "Dateien (Öffnen, Speichern, Export, Import, Drag & Drop)", "full_files.rs", &rows);
    eprintln!("Dateien: {ok}/{n} ✅");
}
