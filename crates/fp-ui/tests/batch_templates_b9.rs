//! BATCH-9: eingebaute Vorlagen im Menü „Rezepte“ des Batch-Studios und Start mit
//! `firepixel --batch <rezept> <dateien>` (`batch::launch`).

use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::batch;
use std::path::PathBuf;

fn state() -> AppState {
    fp_engine::shell::disable();
    let session = fp_ui::ui_commands::new_session(fp_engine::Preferences::new());
    let mut st = AppState::new(session, UiSettings::default(), None);
    st.sync_render = true;
    st.os_clipboard = false;
    st
}

fn png(dir: &std::path::Path, name: &str) -> PathBuf {
    let p = dir.join(name);
    let px: Vec<u8> = (0..12 * 8).flat_map(|i| [(i * 3) as u8, 90, 200, 255]).collect();
    let item = fp_batch::item::still_from_rgba(12, 8, &px).expect("item");
    let (bytes, _) = fp_batch::output::encode(&item, fp_batch::OutputFormat::Png, &Default::default()).expect("png");
    std::fs::write(&p, bytes).expect("write");
    p
}

#[test]
fn templates_registered_with_the_app_and_translated() {
    let ctx = egui::Context::default();
    let _app = fp_ui::FirepixelApp::with_session(&ctx, fp_engine::Session::default(), UiSettings::default(), false);
    let titles: Vec<String> = batch::templates().into_iter().map(|t| t.title).collect();
    assert!(titles.iter().any(|t| t == "Logos nachzeichnen → SVG"), "{titles:?}");
    assert!(titles.iter().any(|t| t == "Für das Web verkleinern + WebP"));
    // zweite App im selben Prozess meldet nicht doppelt an
    let _app2 = fp_ui::FirepixelApp::with_session(&ctx, fp_engine::Session::default(), UiSettings::default(), false);
    let n = batch::templates().iter().filter(|t| t.title == "Logos nachzeichnen → SVG").count();
    assert_eq!(n, 1);
    let en: Vec<String> = fp_i18n::with_language("en", || batch::templates().into_iter().map(|t| t.title).collect());
    assert!(en.iter().any(|t| t == "Trace logos → SVG"), "{en:?}");
    // Vorlage wählen = Rezept setzen (wie der Menüeintrag): Name, Schritte, Format
    let t = batch::templates().into_iter().find(|t| t.title == "Logos nachzeichnen → SVG").expect("Vorlage");
    assert_eq!(t.recipe.steps[0].id, "vector.trace");
    assert_eq!(t.recipe.output.format, "svg");
}

#[test]
fn launch_with_template_name_or_file() {
    let dir = std::env::temp_dir().join(format!("firepixel-batch9-ui-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("dir");
    let a = png(&dir, "a.png");
    let b = png(&dir, "b.png");
    let mut st = state();
    assert!(batch::launch(&mut st, Some("logos-to-svg"), &[a.clone(), b.clone()]));
    let s = batch::studio(&st).expect("Studio offen");
    assert!(s.front);
    assert_eq!(s.files.len(), 2);
    assert_eq!(s.recipe.steps.len(), 1);
    assert_eq!(s.recipe.output.format, "svg");
    // gleiche Dateien noch einmal: keine Doppelten, Rezept ersetzt (nicht addiert)
    assert!(batch::launch(&mut st, Some("web-webp"), &[a.clone()]));
    let s = batch::studio(&st).expect("Studio");
    assert_eq!(s.files.len(), 2);
    assert_eq!(s.recipe.output.format, "webp");
    // Rezeptdatei hat Vorrang und wird als Pfad gemerkt
    let f = dir.join("mein.fpbatch.json");
    let mut r = fp_batch::Recipe::named("Mein Rezept");
    r.output.format = "png".into();
    std::fs::write(&f, r.to_json()).expect("write");
    assert!(batch::launch(&mut st, Some(&f.to_string_lossy()), &[]));
    let s = batch::studio(&st).expect("Studio");
    assert_eq!((s.recipe.name.as_str(), s.recipe_path.as_deref()), ("Mein Rezept", Some(f.as_path())));
    // unbekannt: Studio offen, Rezept bleibt
    assert!(!batch::launch(&mut st, Some("gibt-es-nicht"), &[]));
    assert_eq!(batch::studio(&st).expect("Studio").recipe.name, "Mein Rezept");
    let _ = std::fs::remove_dir_all(&dir);
}
