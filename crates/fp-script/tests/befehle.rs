//! Mitgelieferte Befehle (Menü „Befehle“): jeder läuft fehlerfrei auf einem Dokument mit Text,
//! Pfad und Rechteck, als ein Verlaufsschritt über `commands.runScript`.

use fp_engine::Session;
use serde_json::json;

fn setup() -> Session {
    let mut s = Session::default();
    fp_script::install_host(&mut s);
    s.execute("file.new", json!({"width": 300, "height": 200}))
        .unwrap_or_else(|e| panic!("{e}"));
    s.execute(
        "draw.rectangle",
        json!({"rect": [10, 10, 100, 80], "fillColor": "#3366cc"}),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    s.execute("draw.line", json!({"from": [20, 150], "to": [200, 150]}))
        .unwrap_or_else(|e| panic!("{e}"));
    s.execute(
        "draw.text",
        json!({"text": "hallo welt", "anchor": [120, 40], "size": 18}),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    s.execute("select.all", json!({}))
        .unwrap_or_else(|e| panic!("{e}"));
    s
}

#[test]
fn builtin_commands_run() {
    let dir = std::env::temp_dir().join(format!("fp_befehle_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let n = fp_script::builtin_commands::install_into(&dir).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(n, fp_script::builtin_commands::BUILTIN.len());
    // zweiter Aufruf überschreibt nichts
    assert_eq!(
        fp_script::builtin_commands::install_into(&dir).unwrap_or(99),
        0
    );
    let mut failed = Vec::new();
    for (rel, _) in fp_script::builtin_commands::BUILTIN {
        let path = rel.split('/').fold(dir.clone(), |a, p| a.join(p));
        let mut s = setup();
        let before = s.active().map(|d| d.history.undo_count()).unwrap_or(0);
        match s.execute(
            "commands.runScript",
            json!({"path": path.to_string_lossy()}),
        ) {
            Ok(_) => {
                let after = s.active().map(|d| d.history.undo_count()).unwrap_or(0);
                assert!(
                    after <= before + 1,
                    "{rel}: mehr als ein Verlaufsschritt ({before} → {after})"
                );
            }
            Err(e) => failed.push(format!("{rel}: {e}")),
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(failed.is_empty(), "{failed:#?}");
}

#[test]
fn text_case_and_shadow_commands_change_document() {
    let dir = std::env::temp_dir().join(format!("fp_befehle2_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    fp_script::builtin_commands::install_into(&dir).unwrap_or_else(|e| panic!("{e}"));
    let mut s = setup();
    let run = |s: &mut Session, rel: &str| {
        let p = rel.split('/').fold(dir.clone(), |a, x| a.join(x));
        s.execute("commands.runScript", json!({"path": p.to_string_lossy()}))
            .unwrap_or_else(|e| panic!("{rel}: {e}"));
    };
    run(&mut s, "Text/Als Großbuchstaben.jsf");
    let out = fp_script::run_source(
        &mut s,
        "var t = ''; for (var i = 0; i < fw.selection.length; i++) if (fw.selection[i].textChars !== undefined) t = fw.selection[i].textChars; t",
        &fp_script::ScriptOptions::default(),
    );
    assert_eq!(out.result.as_deref(), Some("HALLO WELT"), "{out:?}");
    run(&mut s, "Kreativ/Schatten hinzufügen.jsf");
    let out = fp_script::run_source(
        &mut s,
        "fw.selection[0].effectList.effects.length",
        &fp_script::ScriptOptions::default(),
    );
    assert_eq!(out.result.as_deref(), Some("1"), "{out:?}");
    // 3× Schatten: kein Aufschaukeln – jeweils ein Effekt mehr, nicht exponentiell
    run(&mut s, "Kreativ/Schatten hinzufügen.jsf");
    let out = fp_script::run_source(
        &mut s,
        "fw.selection[0].effectList.effects.length",
        &fp_script::ScriptOptions::default(),
    );
    assert_eq!(out.result.as_deref(), Some("2"), "{out:?}");
    s.undo().unwrap_or_else(|e| panic!("{e}"));
    let out = fp_script::run_source(
        &mut s,
        "fw.selection[0].effectList.effects.length",
        &fp_script::ScriptOptions::default(),
    );
    assert_eq!(out.result.as_deref(), Some("1"), "{out:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Englische Namen: gleiche Zahl, eindeutig, alle Dateien landen im Ordner.
#[test]
fn englische_namen() {
    assert_eq!(
        fp_script::builtin_commands::BUILTIN_EN.len(),
        fp_script::builtin_commands::BUILTIN.len()
    );
    let dir = std::env::temp_dir().join(format!("fp-befehle-en-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let n = fp_script::builtin_commands::install_into_lang(&dir, "en")
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(n, fp_script::builtin_commands::BUILTIN.len());
    assert!(dir.join("Creative").join("Add Drop Shadow.jsf").exists());
    assert!(dir.join("Reset Warning Dialogs.jsf").exists());
    let _ = std::fs::remove_dir_all(&dir);
}
