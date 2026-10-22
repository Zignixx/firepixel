//! FB-20 (TW-198): the pattern fill finds patterns on every OS. Without the reference application's
//! pattern folders (macOS bundle, Linux, fresh Windows) the built-in patterns drawn in code are
//! available; names are matched case-insensitively (with or without `.png`, or by the shown name);
//! an unknown pattern name makes `pi.fill` use the default pattern instead of failing.

use fp_engine::resources::{self, BUILTIN_PATTERNS};
use fp_engine::Session;
use fp_model::{ElementId, Fill};
use serde_json::json;
use std::path::PathBuf;

fn session_with_rect() -> (Session, u64) {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("document");
    let id = s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 100], "fillColor": "#808080", "strokeWidth": 0})).expect("rectangle").value["id"].as_u64().expect("id");
    (s, id)
}

fn fill(s: &Session, id: u64) -> Fill {
    let d = s.active().expect("document");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, ElementId(id)).expect("element");
    fp_engine::docops::path_attrs_of(&e).and_then(|a| a.fill.clone()).expect("fill")
}

#[test]
fn builtin_patterns_without_any_folder() {
    let entries = resources::pattern_entries_in(&[]);
    assert_eq!(entries.len(), BUILTIN_PATTERNS.len(), "{entries:?}");
    for (name, path) in &entries {
        let b = resources::builtin_of(path).unwrap_or_else(|| panic!("{name}: built-in path {path:?}"));
        assert_eq!(b.name, name);
        let img = resources::pattern_image(path).expect("drawn tile");
        assert!(img.width() >= 8 && img.height() >= 8, "{name}");
        assert!(img.as_bytes().chunks(4).all(|p| p[3] == 255), "{name}: opaque tile");
        // a pattern, not a flat colour
        let first = &img.as_bytes()[..4];
        assert!(img.as_bytes().chunks(4).any(|p| p != first), "{name}: more than one colour");
    }
    let names: Vec<&str> = entries.iter().map(|(n, _)| n.as_str()).collect();
    let mut sorted = names.clone();
    sorted.sort_by_key(|n| n.to_lowercase());
    assert_eq!(names, sorted, "ordered by name");
    // a real file never resolves as built-in
    assert!(resources::builtin_of(std::path::Path::new("C:/Patterns/Grid.png")).is_none());
}

#[test]
fn builtin_lookup_is_case_insensitive() {
    for n in ["checkerboard", "CHECKERBOARD", "Checkerboard.png", "checkerboard.PNG", " Checkerboard "] {
        let p = resources::pattern_file_in(&[], n).unwrap_or_else(|| panic!("{n:?} not found"));
        assert_eq!(resources::builtin_of(&p).map(|b| b.name), Some("Checkerboard"), "{n:?}");
    }
    // shown (translated) name
    let p = fp_i18n::with_language("de", || resources::pattern_file_in(&[], "schachbrett")).expect("German label");
    assert_eq!(resources::builtin_of(&p).map(|b| b.name), Some("Checkerboard"));
    assert!(resources::pattern_file_in(&[], "does-not-exist").is_none());
    assert!(resources::pattern_file_in(&[], "").is_none());
}

#[test]
fn default_pattern_falls_back_to_first_available() {
    // no folders: the reference application's default is missing, the first pattern stands in
    let first = resources::pattern_entries_in(&[]).first().map(|(n, _)| n.clone());
    assert_eq!(resources::default_pattern_name_in(&[]), first);
    assert!(first.is_some());
    // a folder with a file of the default's name (any case): that file is the default
    let dir: PathBuf = std::env::temp_dir().join(format!("fp-fb20-patterns-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp folder");
    std::fs::write(dir.join("cotton.PNG"), b"x").expect("file");
    std::fs::write(dir.join("Aaa.png"), b"x").expect("file");
    let dirs = vec![dir.clone()];
    assert_eq!(resources::default_pattern_name_in(&dirs).as_deref(), Some("cotton"));
    let entries = resources::pattern_entries_in(&dirs);
    assert_eq!(entries.len(), BUILTIN_PATTERNS.len() + 2);
    assert_eq!(entries.first().map(|(n, _)| n.as_str()), Some("Aaa"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn pi_fill_pattern_never_fails_for_a_missing_name() {
    let (mut s, id) = session_with_rect();
    // the "Pattern" button: category without a name
    s.execute("pi.fill", json!({"category": "pattern"})).expect("pattern button");
    let f = fill(&s, id);
    assert_eq!(f.shape, 15);
    let p = f.pattern.expect("pattern");
    assert_eq!(Some(p.name.clone()), resources::default_pattern_name());
    assert!(p.image.is_some(), "pattern with image");
    s.execute("pi.fill", json!({"category": "solid"})).expect("solid");
    // unknown name (e.g. the reference application's default on a machine without its folders)
    let out = s.execute("pi.fill", json!({"pattern": "No Such Pattern"})).expect("stand-in instead of an error");
    assert_eq!(out.value["patternFallback"]["requested"], "No Such Pattern");
    let f = fill(&s, id);
    assert_eq!(f.shape, 15);
    assert_eq!(f.pattern.as_ref().map(|p| p.name.clone()), resources::default_pattern_name());
    assert_eq!(out.value["patternFallback"]["used"].as_str().map(str::to_string), resources::default_pattern_name());
    // built-in by name, any case
    s.execute("pi.fill", json!({"pattern": "POLKA DOTS"})).expect("built-in pattern");
    let f = fill(&s, id);
    assert!(f.pattern.as_ref().is_some_and(|p| p.name.eq_ignore_ascii_case("polka dots") && p.image.is_some()));
    // an explicit file path that does not exist is still an error (nothing to stand in for a file)
    let missing = std::env::temp_dir().join("fp-fb20-missing").join("none.png");
    assert!(s.execute("pi.fill", json!({"pattern": missing.to_string_lossy()})).is_err());
}
