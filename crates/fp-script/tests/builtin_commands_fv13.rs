//! FV-13 (P-MENU1): mitgelieferte Befehle „Andere Ebenen ausblenden/sperren“ wirken bei mehreren
//! Ebenen (vorher brach das Skript ab: `setLayerVisible`/`setLayerLocked` haben wie im Original vier
//! Pflichtargumente), `allLayers` wirkt auf alle Ebenen, alte fehlerhafte Fassungen im
//! Befehlsordner werden ersetzt.

use fp_engine::Session;
use fp_script::builtin_commands::{BUILTIN, BUILTIN_EN, update_outdated};
use serde_json::json;
use std::path::PathBuf;

fn setup() -> Session {
    let mut s = Session::default();
    fp_script::install_host(&mut s);
    s.execute("file.new", json!({"width": 300, "height": 200}))
        .unwrap();
    s.execute("draw.rectangle", json!({"rect": [10, 10, 100, 80]}))
        .unwrap();
    s.execute("layer.add", json!({"name": "Oben"})).unwrap();
    s
}

/// (Name, sichtbar, gesperrt, Web-Ebene) aller Ebenen der Seite.
fn layers(s: &Session) -> Vec<(String, bool, bool, bool)> {
    let d = s.active().unwrap();
    fp_engine::docops::own_layers(&d.doc, d.page)
        .into_iter()
        .map(|(_, l)| {
            (
                l.name.clone(),
                l.visibility.first().copied().unwrap_or(true),
                l.cels.iter().any(|c| c.locked),
                l.kind == fp_model::LayerKind::Web,
            )
        })
        .collect()
}

fn source(rel: &str) -> &'static str {
    BUILTIN
        .iter()
        .find(|(r, _)| *r == rel)
        .map(|(_, s)| *s)
        .unwrap_or_else(|| panic!("{rel}"))
}

#[test]
fn hide_other_layers_hides_only_other_normal_layers() {
    let mut s = setup();
    let n = s.active().unwrap().history.undo_count();
    let before = serde_json::to_value(&*s.active().unwrap().doc).unwrap();
    s.execute(
        "commands.runScript",
        json!({"code": source("Dokument/Andere Ebenen ausblenden.jsf")}),
    )
    .expect("Skript läuft");
    let l = layers(&s);
    assert_eq!(
        l.iter().map(|x| (x.0.as_str(), x.1)).collect::<Vec<_>>(),
        vec![("Ebene 1", false), ("Oben", true), ("Webebene", true)],
        "{l:?}"
    );
    assert_eq!(
        s.active().unwrap().history.undo_count(),
        n + 1,
        "ein Verlaufsschritt"
    );
    s.undo().unwrap();
    assert_eq!(
        serde_json::to_value(&*s.active().unwrap().doc).unwrap(),
        before
    );
}

#[test]
fn lock_other_layers_locks_only_other_normal_layers() {
    let mut s = setup();
    s.execute(
        "commands.runScript",
        json!({"code": source("Dokument/Andere Ebenen sperren.jsf")}),
    )
    .expect("Skript läuft");
    let l = layers(&s);
    assert_eq!(
        l.iter().map(|x| (x.0.as_str(), x.2)).collect::<Vec<_>>(),
        vec![("Ebene 1", true), ("Oben", false), ("Webebene", false)],
        "{l:?}"
    );
}

#[test]
fn set_layer_visible_and_locked_follow_original_signature() {
    // drei Argumente: wie im Original ein Fehler (vier Pflichtargumente)
    let mut s = setup();
    assert!(
        s.execute(
            "commands.runScript",
            json!({"code": "fw.getDocumentDOM().setLayerVisible(0, -1, false);"})
        )
        .is_err()
    );
    // -1 = aktive Ebene
    s.execute(
        "commands.runScript",
        json!({"code": "fw.getDocumentDOM().setLayerLocked(-1, -1, true, false);"}),
    )
    .unwrap();
    assert_eq!(
        layers(&s).iter().map(|x| x.2).collect::<Vec<_>>(),
        vec![false, true, false]
    );
    // allLayers = true: alle Ebenen, layerIndex wird ignoriert
    s.execute(
        "commands.runScript",
        json!({"code": "fw.getDocumentDOM().setLayerVisible(0, -1, false, true);"}),
    )
    .unwrap();
    assert!(layers(&s).iter().all(|x| !x.1), "{:?}", layers(&s));
    s.execute(
        "commands.runScript",
        json!({"code": "fw.getDocumentDOM().setLayerLocked(0, -1, false, true);"}),
    )
    .unwrap();
    assert!(layers(&s).iter().all(|x| !x.2), "{:?}", layers(&s));
}

#[test]
fn outdated_builtin_commands_are_replaced_but_user_edits_kept() {
    assert_eq!(BUILTIN[18].0, "Dokument/Andere Ebenen ausblenden.jsf");
    assert_eq!(BUILTIN[19].0, "Dokument/Andere Ebenen sperren.jsf");
    assert_eq!(BUILTIN_EN[18], "Document/Hide Other Layers.jsf");
    assert_eq!(BUILTIN_EN[19], "Document/Lock Other Layers.jsf");
    let dir: PathBuf =
        std::env::temp_dir().join(format!("fp_fv13_outdated_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let file = |rel: &str| rel.split('/').fold(dir.clone(), |a, p| a.join(p));
    let old_hide = "var d = fw.getDocumentDOM();\nvar cur = d.currentLayerNum;\nfor (var i = 0; i < d.layers.length; i++) if (i != cur && d.layers[i].layerType != \"web\") d.setLayerVisible(i, -1, false);\n";
    let old_lock = "var d = fw.getDocumentDOM();\nvar cur = d.currentLayerNum;\nfor (var i = 0; i < d.layers.length; i++) if (i != cur && d.layers[i].layerType != \"web\") d.setLayerLocked(i, -1, true);\n";
    for (rel, src) in [
        (BUILTIN[18].0, old_hide),
        (BUILTIN_EN[19], old_lock),
        (BUILTIN[19].0, "// vom Nutzer geändert\n"),
    ] {
        std::fs::create_dir_all(file(rel).parent().unwrap()).unwrap();
        std::fs::write(file(rel), src).unwrap();
    }
    assert_eq!(update_outdated(&dir).unwrap(), 2);
    assert_eq!(
        std::fs::read_to_string(file(BUILTIN[18].0)).unwrap(),
        BUILTIN[18].1
    );
    assert_eq!(
        std::fs::read_to_string(file(BUILTIN_EN[19])).unwrap(),
        BUILTIN[19].1
    );
    assert_eq!(
        std::fs::read_to_string(file(BUILTIN[19].0)).unwrap(),
        "// vom Nutzer geändert\n",
        "Nutzeränderung bleibt"
    );
    assert_eq!(
        update_outdated(&dir).unwrap(),
        0,
        "zweiter Lauf ändert nichts"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn reset_warning_dialogs_command_clears_disabled_notices() {
    let mut s = setup();
    s.execute(
        "edit.preferences",
        json!({"key": "DisableNoticeCombinePaths", "value": true}),
    )
    .unwrap();
    s.execute(
        "commands.runScript",
        json!({"code": source("Warnmeldungen zurücksetzen.jsf")}),
    )
    .unwrap();
    assert!(!s.prefs.get_bool("DisableNoticeCombinePaths", false));
}

/// Alle mitgelieferten Befehle laufen auch bei mehreren Ebenen und Auswahl ohne Ausnahme (der
/// Fehler oben fiel nur mit einer zweiten Ebene auf).
#[test]
fn all_builtin_commands_run_with_two_layers() {
    let mut failed = Vec::new();
    for (rel, src) in BUILTIN {
        let mut s = setup();
        s.execute(
            "draw.text",
            json!({"text": "hallo welt", "anchor": [120, 40], "size": 18}),
        )
        .unwrap();
        s.execute("select.all", json!({})).unwrap();
        if let Err(e) = s.execute("commands.runScript", json!({"code": src})) {
            failed.push(format!("{rel}: {e}"));
        }
    }
    assert!(failed.is_empty(), "{failed:#?}");
}
