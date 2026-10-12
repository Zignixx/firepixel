//! NZ-4: Kleinkram in Engine, Dateien und Export.
//! pHYs 2834 Pixel/m, „Neue Schaltfläche“ öffnet die Symbolbearbeitung, Fußknopf „Auf Status
//! verteilen“ wie das Menü, Sprite-Einstellungen im FW-PNG, Zusatzfelder von Öffnen/Speichern unter,
//! Startordner „Bilder“ über `fp_i18n::profile`.
//! Wie alle Testläufe mit `FP_NO_SHELL_OPEN=1` starten (AGENTEN_REGELN §4a).

use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::file_dialog::{OpenChoice, SaveChoice};
use fp_ui::files::{self, SaveKind};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn state() -> AppState {
    fp_engine::shell::disable();
    let session = fp_ui::ui_commands::new_session(fp_engine::Preferences::new());
    let mut st = AppState::new(session, UiSettings::default(), None);
    st.sync_render = true;
    st.os_clipboard = false;
    st
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fp-nz4-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn phys(bytes: &[u8]) -> Vec<u8> {
    let chunks = fp_fwpng::png::read_chunks(bytes).unwrap();
    chunks.into_iter().find(|c| &c.kind == b"pHYs").expect("pHYs").data
}

// ------------------------------------------------------------------ 1. pHYs

#[test]
fn png_export_writes_2834_pixels_per_meter_at_72_dpi() {
    let mut st = state();
    let dir = tmp("phys");
    st.session.execute("file.new", json!({"width": 20, "height": 10, "background": "#ffffff", "resolution": 72})).unwrap();
    st.session.execute("draw.rectangle", json!({"rect": [2, 2, 12, 8], "fillColor": "#3366cc", "strokeWidth": 0})).unwrap();
    let p = dir.join("a.png");
    st.session.execute("file.export", json!({"path": p.to_string_lossy(), "settings": {"format": "png32"}})).unwrap();
    let d = phys(&std::fs::read(&p).unwrap());
    let x = u32::from_be_bytes([d[0], d[1], d[2], d[3]]);
    let y = u32::from_be_bytes([d[4], d[5], d[6], d[7]]);
    // Original: abgeschnitten (72 · 10000 / 254 = 2834,6 → 2834), Einheit Meter
    assert_eq!((x, y, d[8]), (2834, 2834, 1));
    let _ = std::fs::remove_dir_all(&dir);
}

// ------------------------------------------------------------------ 2. Neue Schaltfläche

#[test]
fn new_button_opens_symbol_editing_like_new_symbol() {
    let mut st = state();
    st.session.execute("file.new", json!({"width": 100, "height": 100})).unwrap();
    let parent = st.session.active_id().unwrap();
    // Menü: Formular mit Name; OK ruft den Befehl mit `edit: true`
    assert!(fp_ui::dialogs::open_for_command(&mut st, "edit.insert.newButton"));
    let f = st.dialogs.form.clone().expect("Dialog offen");
    assert_eq!(f.command, "edit.insert.newButton");
    assert_eq!(f.base["edit"], json!(true));
    st.dialogs.form = None;
    let v = st.session.execute("edit.insert.newButton", json!({"name": "Knopf", "edit": true})).unwrap().value;
    assert!(v["doc"].as_u64().is_some());
    let view = st.session.active().unwrap();
    assert_ne!(view.id, parent, "Symbolbearbeitung aktiv");
    assert_eq!(view.symbol_edit.as_ref().map(|l| l.parent), Some(parent));
    let doc = &st.session.document(parent).unwrap().doc;
    assert_eq!(doc.symbols.len(), 1);
    assert_eq!(doc.symbols[0].name, "Knopf");
    assert_eq!(doc.symbols[0].kind(), fp_model::SymbolKind::Button);
    let elems: usize = fp_engine::docops::own_layers(doc, 0).iter().map(|(_, l)| l.cels.iter().map(|c| c.elements.len()).sum::<usize>()).sum();
    assert_eq!(elems, 0, "keine Instanz im Dokument");
    // ohne `edit` (Skript `addNewSymbol`) weiter wie bisher: Instanz im Dokument
    st.session.execute("modify.symbol.exitEdit", json!({})).unwrap();
    st.session.execute("edit.insert.newButton", json!({"name": "Alt"})).unwrap();
    assert!(st.session.active().is_some_and(|d| d.symbol_edit.is_none() && d.doc.symbols.len() == 2));
}

// ------------------------------------------------------------------ 3. Fußknopf „Auf Status verteilen“

#[test]
fn distribute_footer_button_enabled_from_one_object_like_menu() {
    let mut st = state();
    st.session.execute("file.new", json!({"width": 100, "height": 100})).unwrap();
    st.session.execute("draw.rectangle", json!({"rect": [5, 5, 40, 30], "fillColor": "#3366cc", "strokeWidth": 0})).unwrap();
    st.session.execute("select.none", json!({})).unwrap();
    assert!(!fp_ui::panels::states::distribute_enabled(&st), "nichts gewählt: grau");
    st.session.execute("select.all", json!({})).unwrap();
    assert_eq!(st.session.active().unwrap().selection.elements.len(), 1);
    assert!(fp_ui::panels::states::distribute_enabled(&st), "ein Objekt: aktiv wie das Menü");
    assert_eq!(fp_ui::panels::states::distribute_enabled(&st), st.session.is_available("panel.states.distribute"));
}

// ------------------------------------------------------------------ 5. Sprite-Einstellungen im FW-PNG

#[test]
fn sprite_settings_survive_save_and_open() {
    let mut st = state();
    let dir = tmp("sprite");
    st.session.execute("file.new", json!({"width": 40, "height": 30})).unwrap();
    {
        let d = st.session.active_mut().unwrap();
        let doc = Arc::make_mut(&mut d.doc);
        let e = Arc::make_mut(&mut doc.pages[0]).root.export.get_or_insert_with(Default::default);
        let h = e.html.get_or_insert_with(|| Box::new(fp_model::HtmlSettings::cs6_default()));
        h.sprite_selector_is_class = Some(false);
        h.sprite_css_prefix = Some("pre-".into());
        h.sprite_css_suffix = Some("-suf".into());
        h.sprite_padding = Some(7);
        h.sprite_orientation = Some(2);
        h.sprite_preset = Some("GIF Web 216".into());
    }
    let p = dir.join("sprites.fw.png");
    st.session.execute("file.saveAs", json!({"path": p.to_string_lossy()})).unwrap();
    // Datei direkt lesen und über die Sitzung öffnen
    let doc = fp_fwpng::read(&std::fs::read(&p).unwrap()).unwrap();
    let h = doc.pages[0].root.export.as_ref().and_then(|e| e.html.as_deref()).expect("HTML-Einstellungen");
    assert_eq!(h.sprite_selector_is_class, Some(false));
    assert_eq!(h.sprite_css_prefix.as_deref(), Some("pre-"));
    assert_eq!(h.sprite_css_suffix.as_deref(), Some("-suf"));
    assert_eq!(h.sprite_padding, Some(7));
    assert_eq!(h.sprite_orientation, Some(2));
    assert_eq!(h.sprite_preset.as_deref(), Some("GIF Web 216"));
    assert!(!doc.pages[0].root.export.as_ref().unwrap().extra.has_raw(fp_model::TagKey::new(b"fpSv")), "Erweiterung nicht doppelt als unbekannter Tag");
    // nochmals speichern: kein zweiter Container
    st.session.execute("file.open", json!({"path": p.to_string_lossy()})).unwrap();
    let p2 = dir.join("wieder.fw.png");
    st.session.execute("file.saveAs", json!({"path": p2.to_string_lossy()})).unwrap();
    let raw = std::fs::read(&p2).unwrap();
    let again = fp_fwpng::read(&raw).unwrap();
    assert_eq!(again.pages[0].root.export.as_ref().and_then(|e| e.html.as_ref()).and_then(|h| h.sprite_padding), Some(7));
    // ohne Sprite-Werte wird nichts Eigenes geschrieben
    let plain = fp_model::HtmlSettings::cs6_default();
    assert!(plain.sprite_padding.is_none() && plain.sprite_preset.is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

// ------------------------------------------------------------------ 6. Zusatzfelder der Datei-Dialoge

fn png_file(dir: &Path, name: &str, color: &str) -> PathBuf {
    let mut st = state();
    st.session.execute("file.new", json!({"width": 16, "height": 16, "background": color})).unwrap();
    let p = dir.join(name);
    st.session.execute("file.saveAs", json!({"path": p.to_string_lossy()})).unwrap();
    p
}

#[test]
fn open_as_untitled_and_as_animation() {
    let dir = tmp("open");
    let a = png_file(&dir, "a.fw.png", "#ff0000");
    let b = png_file(&dir, "b.fw.png", "#00ff00");
    let c = png_file(&dir, "c.fw.png", "#0000ff");
    let mut st = state();
    // normal: mit Dateinamen
    files::open_chosen(&mut st, std::slice::from_ref(&a), OpenChoice::default());
    assert_eq!(st.session.active().and_then(|d| d.path.clone()), Some(a.clone()));
    // als „Unbenannt“: ohne Dateinamen
    files::open_chosen(&mut st, std::slice::from_ref(&b), OpenChoice { untitled: true, animation: false });
    assert!(st.session.active().is_some_and(|d| d.path.is_none()), "unbenannt");
    assert!(st.dialogs.message.is_none(), "{:?}", st.dialogs.message);
    // als Animation: ein unbenanntes Dokument, ein Status je Datei
    let n = st.session.documents().len();
    files::open_chosen(&mut st, &[a.clone(), b.clone(), c.clone()], OpenChoice { untitled: false, animation: true });
    assert_eq!(st.session.documents().len(), n + 1);
    let d = st.session.active().unwrap();
    assert!(d.path.is_none());
    assert_eq!(d.doc.pages[0].root.frame_count, 3, "drei Status");
    // eine Datei: „Als Animation öffnen“ wirkt nicht (im Original grau)
    files::open_chosen(&mut st, std::slice::from_ref(&c), OpenChoice { untitled: false, animation: true });
    assert_eq!(st.session.active().and_then(|d| d.path.clone()), Some(c.clone()));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_as_without_append_fw_png_keeps_the_name() {
    let dir = tmp("append");
    let mut st = state();
    st.session.execute("file.new", json!({"width": 16, "height": 16})).unwrap();
    let fw = files::save_type_index(SaveKind::FwPng);
    assert!(files::save_chosen(&mut st, SaveChoice { path: dir.join("ohne.png"), filter: fw, copy: false, append_fw_png: false }));
    assert!(dir.join("ohne.png").is_file() && !dir.join("ohne.fw.png").exists());
    assert!(fp_fwpng::is_fwpng(&std::fs::read(dir.join("ohne.png")).unwrap()), "trotzdem FW-PNG");
    assert!(files::save_chosen(&mut st, SaveChoice { path: dir.join("mit.png"), filter: fw, copy: false, append_fw_png: true }));
    assert!(dir.join("mit.fw.png").is_file());
    assert_eq!(files::save_target_for_ex(PathBuf::from("x"), SaveKind::FwPng, false), PathBuf::from("x.png"));
    assert_eq!(files::save_target_for_ex(PathBuf::from("x"), SaveKind::FwPng, true), PathBuf::from("x.fw.png"));
    // Kästchen nur bei FW-PNG
    assert!(files::save_type_extras(SaveKind::FwPng).append_fw);
    assert!(!files::save_type_extras(SaveKind::FlatPng).append_fw);
    assert!(!files::save_type_extras(SaveKind::Export("gif")).append_fw);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn extra_field_labels_in_both_languages() {
    assert_eq!(fp_i18n::t("fdlg-append-fw-png"), ".fw.png anhängen");
    assert_eq!(fp_i18n::t("fdlg-open-untitled"), "Als „Unbenannt“ öffnen");
    assert_eq!(fp_i18n::t("fdlg-open-as-animation"), "Als Animation öffnen");
    fp_i18n::set_language("en");
    assert_eq!(fp_i18n::t("fdlg-open-as-animation"), "Open as Animation");
    fp_i18n::set_language("de");
}

// ------------------------------------------------------------------ 7. Startordner „Bilder“

#[test]
fn known_folders_stay_in_the_test_profile() {
    use fp_i18n::profile::{known_folder, KnownFolder};
    assert!(fp_i18n::profile::is_test_run());
    let root = fp_i18n::profile::redirected_root().unwrap_or_else(fp_i18n::profile::use_test_profile);
    for f in [KnownFolder::Pictures, KnownFolder::Documents] {
        let p = known_folder(f).expect("Ordner");
        assert!(p.starts_with(&root), "{p:?} liegt im Testprofil {root:?}");
        assert!(!fp_i18n::profile::is_real(&p) || p.starts_with(&root));
    }
}
