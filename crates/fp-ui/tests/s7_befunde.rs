//! S-7: „Speichern unter“ ▸ BMP ▸ „Optionen…“ wie im Original (re 33 §5): Bildvorschau mit
//! „BMP 8“/„BMP 24“, keine RLE-Wahl; das Speichern als BMP schreibt danach mit dieser Einstellung.
//! Wie alle Testläufe mit `FP_NO_SHELL_OPEN=1` starten (AGENTEN_REGELN §4a).

use fp_export::options::{ColorMode, ExportFormat};
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::file_dialog::SaveChoice;
use fp_ui::files::{self, SaveKind};
use serde_json::json;
use std::path::PathBuf;

fn state() -> AppState {
    fp_engine::shell::disable();
    let session = fp_ui::ui_commands::new_session(fp_engine::Preferences::new());
    let mut st = AppState::new(session, UiSettings::default(), None);
    st.sync_render = true;
    st.os_clipboard = false;
    st
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fp-s7-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Bits je Pixel aus dem BMP-Kopf (`biBitCount`).
fn bmp_bpp(p: &std::path::Path) -> u16 {
    let b = std::fs::read(p).expect("BMP");
    assert_eq!(&b[0..2], b"BM");
    u16::from_le_bytes([b[28], b[29]])
}

#[test]
fn bmp_options_open_the_image_preview_with_bmp_8() {
    let mut st = state();
    st.session.execute("file.new", json!({"width": 16, "height": 16})).unwrap();
    let bmp = files::save_type_index(SaveKind::Export("bmp"));
    let i = bmp.expect("BMP-Typ");
    let extras = files::save_as_extras();
    assert!(extras.types[i].options, "„Optionen…“ bei BMP aktiv");
    assert!(!extras.types[files::save_type_index(SaveKind::FwPng).unwrap()].options, "bei FW-PNG grau");
    (extras.on_options)(&mut st, i);
    assert!((extras.options_open)(&st), "Bildvorschau offen");
    let s = st.dialogs.preview.as_ref().expect("Bildvorschau").active_settings().clone();
    assert_eq!((s.format, s.color_mode), (ExportFormat::Bmp, ColorMode::Indexed), "BMP 8");
    assert!(!s.bmp_rle8, "keine RLE-Wahl");
    // die Formatliste bietet BMP 8 und BMP 24
    let f = fp_ui::dialogs::image_preview::options::FORMATS;
    assert!(f.iter().any(|(n, x)| *n == "BMP 8" && *x == Some((ExportFormat::Bmp, ColorMode::Indexed))));
    assert!(f.iter().any(|(n, x)| *n == "BMP 24" && *x == Some((ExportFormat::Bmp, ColorMode::Rgb24))));
    // schon BMP: bleibt (BMP 24 bleibt BMP 24)
    let mut b24 = s.clone();
    b24.color_mode = ColorMode::Rgb24;
    assert_eq!(files::bmp_options_settings(b24.clone()).color_mode, ColorMode::Rgb24);
}

#[test]
fn save_as_bmp_writes_with_the_options_setting() {
    let dir = tmp("bmp");
    let mut st = state();
    st.session.execute("file.new", json!({"width": 16, "height": 16})).unwrap();
    let bmp = files::save_type_index(SaveKind::Export("bmp"));
    let save = |st: &mut AppState, name: &str| {
        let p = dir.join(name);
        assert!(files::save_chosen(st, SaveChoice { path: p.clone(), filter: bmp, copy: true, append_fw_png: false }));
        p
    };
    // ohne Optionen wie bisher BMP 24
    assert_eq!(bmp_bpp(&save(&mut st, "vorher.bmp")), 24);
    // „Optionen…“ ▸ OK mit BMP 8
    (files::save_as_extras().on_options)(&mut st, bmp.unwrap());
    let s = st.dialogs.preview.take().expect("Bildvorschau").active_settings().clone();
    assert!(fp_ui::dialogs::image_preview::commit_settings(&mut st, &s));
    st.process_actions(&egui::Context::default());
    assert_eq!(bmp_bpp(&save(&mut st, "bmp8.bmp")), 8);
    // BMP 24 gewählt
    let mut s24 = s.clone();
    s24.color_mode = ColorMode::Rgb24;
    assert!(fp_ui::dialogs::image_preview::commit_settings(&mut st, &s24));
    st.process_actions(&egui::Context::default());
    assert_eq!(bmp_bpp(&save(&mut st, "bmp24.bmp")), 24);
    let _ = std::fs::remove_dir_all(&dir);
}
