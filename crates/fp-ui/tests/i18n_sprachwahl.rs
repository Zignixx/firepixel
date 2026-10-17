//! Sprachwahl (Phase 6, I18N-KERN): Untermenü „Ansicht ▸ Sprache“, Wechsel ohne Neustart,
//! Speichern in den Voreinstellungen, Start mit gespeicherter bzw. Systemsprache, Sprachordner,
//! Schriftabdeckung (Kyrillisch, Griechisch, CJK). Mit `-- --ignored` zusätzlich das Bild
//! `docs\screenshots\phase6_sprachwahl.png` (braucht einen Grafikadapter).

use egui_kittest::Harness;
use fp_engine::{PrefValue, Preferences, Session};
use fp_ui::app_state::{Action, AppState, UiSettings};
use fp_ui::language;
use fp_ui::menu::{self, ItemAction, Node};
use fp_ui::FirepixelApp;
use serde_json::json;
use std::sync::Mutex;

/// Die Sprache ist prozessweit: Tests dieser Datei laufen nacheinander.
static LOCK: Mutex<()> = Mutex::new(());

fn lock() -> std::sync::MutexGuard<'static, ()> {
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn harness() -> Harness<'static, FirepixelApp> {
    fp_engine::shell::disable();
    let ctx = egui::Context::default();
    let app = FirepixelApp::with_session(&ctx, Session::default(), UiSettings::default(), false);
    Harness::builder().renderer(crate::gpu_common::renderer()).with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(8).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app)
}

/// Untermenü „Sprache“ aus dem Ansicht-Menü (über die Aktion der Einträge erkannt).
fn language_menu(st: &AppState) -> Option<(String, Vec<Node>)> {
    for (_, nodes) in menu::build_main_menu(st) {
        for n in nodes {
            if let Node::Sub { label, children, .. } = &n
                && children.iter().any(|c| matches!(c, Node::Item { action: ItemAction::Ui(Action::Run(name, _)), .. } if name == "ui.setLanguage"))
            {
                return Some((label.clone(), children.clone()));
            }
        }
    }
    None
}

fn items(nodes: &[Node]) -> Vec<(String, Option<bool>, String)> {
    nodes
        .iter()
        .filter_map(|n| match n {
            Node::Item { label, checked, action: ItemAction::Ui(Action::Run(name, p)), .. } => {
                Some((label.clone(), *checked, format!("{name}:{}", p.get("code").and_then(|c| c.as_str()).unwrap_or(""))))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn menu_lists_languages_and_switches_immediately() {
    let _g = lock();
    fp_i18n::set_language("de");
    let mut h = harness();
    h.run_steps(2);
    let (label, kids) = language_menu(&h.state().state).expect("Ansicht ▸ Sprache");
    assert_eq!(label, "Sprache");
    let it = items(&kids);
    assert!(it.contains(&("Deutsch".to_string(), Some(true), "ui.setLanguage:de".to_string())), "{it:?}");
    assert!(it.contains(&("English".to_string(), Some(false), "ui.setLanguage:en".to_string())), "{it:?}");
    assert!(it.iter().any(|(l, _, a)| l == "Sprachordner öffnen..." && a.starts_with("ui.openLanguageFolder")), "{it:?}");
    // Testsprache nur im Debug-Build bzw. mit Entwickleroptionen
    assert_eq!(it.iter().any(|(_, _, a)| a == "ui.setLanguage:qps"), cfg!(debug_assertions), "{it:?}");

    // Wechsel nach Englisch: sofort wirksam, gespeichert, kein Neustart
    h.state_mut().state.push(Action::Run("ui.setLanguage".into(), json!({"code": "en"})));
    h.run_steps(2);
    assert_eq!(fp_i18n::current_language(), "en");
    assert_eq!(language::stored(&h.state().state.session.prefs).as_deref(), Some("en"));
    let (label, kids) = language_menu(&h.state().state).expect("View ▸ Language");
    assert_eq!(label, "Language");
    let it = items(&kids);
    assert!(it.contains(&("English".to_string(), Some(true), "ui.setLanguage:en".to_string())), "{it:?}");
    assert!(it.iter().any(|(l, _, _)| l == "Open Language Folder..."));
    // 50× hin und her: keine doppelten Einträge, Zustand stimmt
    for i in 0..50 {
        let code = if i % 2 == 0 { "de" } else { "en" };
        h.state_mut().state.push(Action::Run("ui.setLanguage".into(), json!({"code": code})));
        h.run_steps(1);
    }
    assert_eq!(fp_i18n::current_language(), "en");
    let (_, kids) = language_menu(&h.state().state).expect("Menü");
    assert_eq!(items(&kids).iter().filter(|(_, _, a)| a == "ui.setLanguage:de").count(), 1);
    assert_eq!(items(&kids).iter().filter(|(_, c, _)| *c == Some(true)).count(), 1, "genau ein Häkchen");
    // unbekannte Sprache: bleibt, Statusmeldung
    h.state_mut().state.push(Action::Run("ui.setLanguage".into(), json!({"code": "xx-nichts"})));
    h.run_steps(1);
    assert_eq!(fp_i18n::current_language(), "en");
    // zurück
    h.state_mut().state.push(Action::Run("ui.setLanguage".into(), json!({"code": "de"})));
    h.run_steps(2);
    assert_eq!(fp_i18n::current_language(), "de");
    assert_eq!(language_menu(&h.state().state).map(|m| m.0).as_deref(), Some("Sprache"));
}

#[test]
fn start_uses_stored_then_system_language() {
    let _g = lock();
    let mut p = Preferences::new();
    p.set(language::PREF_KEY, PrefValue::Str("en".into()));
    assert_eq!(language::init_with(&mut p, false), "en");
    p.set(language::PREF_KEY, PrefValue::Str("EN".into()));
    assert_eq!(language::init_with(&mut p, false), "en", "Groß-/Kleinschreibung egal");
    assert_eq!(language::stored(&p).as_deref(), Some("en"), "Schreibweise vereinheitlicht");
    // gespeicherte Sprache gibt es nicht mehr (Ordner gelöscht) → Deutsch bzw. beim ersten Start Systemsprache
    p.set(language::PREF_KEY, PrefValue::Str("xx".into()));
    assert_eq!(language::init_with(&mut p, false), "de");
    // erster Start: Systemsprache, danach festgehalten
    let mut first = Preferences::new();
    let sys = fp_i18n::system_language();
    assert_eq!(language::init_with(&mut first, true), sys);
    assert_eq!(language::stored(&first), Some(sys));
    // Voreinstellungen von vor der Sprachwahl (Datei vorhanden, ohne Sprache): Deutsch wie bisher
    let mut old = Preferences::new();
    assert_eq!(language::init_with(&mut old, false), "de");
    assert_eq!(language::stored(&old).as_deref(), Some("de"));
    fp_i18n::set_language("de");
}

#[test]
fn external_language_folder_appears_in_menu() {
    let _g = lock();
    let dir = std::env::temp_dir().join(format!("fp-ui-sprachen-{}", std::process::id()));
    let fr = dir.join("fr");
    let _ = std::fs::create_dir_all(&fr);
    let _ = std::fs::write(fr.join("common.ftl"), "language-name = Français\nlang-menu = Langue\nbtn-cancel = Annuler\n");
    assert_eq!(fp_i18n::load_external_dir(&dir), vec!["fr".to_string()]);
    let mut h = harness();
    h.run_steps(1);
    let (_, kids) = language_menu(&h.state().state).expect("Menü");
    assert!(items(&kids).iter().any(|(l, c, a)| l == "Français" && *c == Some(false) && a == "ui.setLanguage:fr"));
    h.state_mut().state.push(Action::Run("ui.setLanguage".into(), json!({"code": "fr"})));
    h.run_steps(2);
    assert_eq!(language_menu(&h.state().state).map(|m| m.0).as_deref(), Some("Langue"));
    // fehlt im Französischen → Englisch
    assert_eq!(fp_i18n::t("lang-open-folder"), "Open Language Folder...");
    fp_i18n::set_language("de");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn rtl_languages_are_marked() {
    let _g = lock();
    let dir = std::env::temp_dir().join(format!("fp-ui-rtl-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join("ar.ftl"), "language-name = العربية\n");
    fp_i18n::load_external_dir(&dir);
    let c = language::choices(false);
    assert!(c.iter().any(|(code, name)| code == "ar" && name.ends_with("(RTL)")), "{c:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Segoe UI deckt Kyrillisch und Griechisch ab, die CJK-Rückfallschriften Chinesisch, Japanisch
/// und Koreanisch (nur wo die Systemschriften vorhanden sind).
#[test]
fn fonts_cover_cyrillic_greek_and_cjk() {
    let _g = lock();
    let dir = std::env::var_os("WINDIR").map(std::path::PathBuf::from).unwrap_or_default().join("Fonts");
    if !dir.join("segoeui.ttf").exists() {
        eprintln!("Segoe UI fehlt – Test übersprungen");
        return;
    }
    let ctx = egui::Context::default();
    fp_ui::theme::install_fonts(&ctx);
    let mut out = ctx.run_ui(Default::default(), |_| {});
    out.textures_delta.clear();
    let id = egui::FontId::proportional(12.0);
    let has = |s: &str| ctx.fonts_mut(|f| f.has_glyphs(&id, s));
    assert!(has("Съешь же ещё этих мягких французских булок"), "Kyrillisch");
    assert!(has("Ξεσκεπάζω την ψυχοφθόρα βδελυγμία"), "Griechisch");
    assert!(has("ÀÉÎÕÜçñøåœ"), "Westeuropäisch");
    assert!(has("ąćęłńóśźż ěščřžýáíé őű"), "Mitteleuropäisch");
    if dir.join("msyh.ttc").exists() {
        assert!(has("文件编辑视图"), "Chinesisch");
    }
    if dir.join("YuGothM.ttc").exists() || dir.join("YuGothR.ttc").exists() {
        assert!(has("ファイル編集表示"), "Japanisch");
    }
    if dir.join("malgun.ttf").exists() {
        assert!(has("파일 편집 보기"), "Koreanisch");
    }
}

/// Bild der Sprachwahl für das README (`-- --ignored`, braucht einen Grafikadapter).
#[test]
#[ignore]
fn screenshot_sprachwahl() {
    let _g = lock();
    fp_i18n::set_language("de");
    let dir = std::env::temp_dir().join(format!("fp-ui-shot-{}", std::process::id()));
    let _ = std::fs::create_dir_all(dir.join("fr"));
    let _ = std::fs::write(dir.join("fr").join("common.ftl"), "language-name = Français\n");
    fp_i18n::load_external_dir(&dir);
    let ctx = egui::Context::default();
    let app = FirepixelApp::with_session(&ctx, Session::default(), UiSettings::default(), false);
    let fonts = std::sync::atomic::AtomicBool::new(false);
    let mut h = Harness::builder().renderer(crate::gpu_common::renderer()).with_size(egui::vec2(600.0, 300.0)).with_pixels_per_point(1.5).renderer(crate::gpu_common::renderer()).build_ui_state(
        move |ui, app: &mut FirepixelApp| {
            if !fonts.swap(true, std::sync::atomic::Ordering::SeqCst) {
                fp_ui::theme::install_fonts(ui.ctx());
            }
            let st = &mut app.state;
            st.theme.apply(ui.ctx());
            let theme = st.theme.clone();
            let ctx = ui.ctx().clone();
            egui::Frame::NONE.fill(theme.colors.canvas_bg).show(ui, |ui| ui.set_min_size(ui.available_size()));
            let menus = menu::build_main_menu(st);
            let view = menus.iter().find(|(_, n)| n.iter().any(|x| matches!(x, Node::Sub { children, .. } if children.iter().any(|c| matches!(c, Node::Item { action: ItemAction::Ui(Action::Run(n, _)), .. } if n == "ui.setLanguage")))));
            let Some((_, nodes)) = view else { return };
            // die letzten Einträge des Ansicht-Menüs (… Design ▸, Sprache ▸) und daneben das Untermenü
            let tail: Vec<Node> = nodes.iter().rev().take(6).rev().cloned().collect();
            let lang = language_menu(st).map(|m| m.1).unwrap_or_default();
            let mut out = Vec::new();
            let r = egui::Area::new(egui::Id::new("shot-view")).fixed_pos(egui::pos2(12.0, 12.0)).show(&ctx, |ui| {
                menu::menu_frame(&theme).show(ui, |ui| {
                    ui.set_width(250.0);
                    ui.vertical(|ui| menu::show_nodes(ui, &tail, &mut out));
                });
            });
            // Untermenü rechts neben der letzten Zeile („Sprache ▸“)
            let rect = r.response.rect;
            egui::Area::new(egui::Id::new("shot-lang")).fixed_pos(egui::pos2(rect.right() - 3.0, rect.bottom() - 34.0)).show(&ctx, |ui| {
                menu::menu_frame(&theme).show(ui, |ui| {
                    ui.set_width(230.0);
                    ui.vertical(|ui| menu::show_nodes(ui, &lang, &mut out));
                });
            });
        },
        app,
    );
    h.run_steps(4);
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("docs").join("screenshots");
    match h.render() {
        Ok(i) => {
            let _ = i.save(out.join("phase6_sprachwahl.png"));
        }
        Err(e) => eprintln!("kein Grafikadapter: {e}"),
    }
    let _ = std::fs::remove_dir_all(&dir);
}
