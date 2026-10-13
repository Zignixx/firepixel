//! `firepixel-cli sprache …` (I18N-KERN): Vorlage anlegen, prüfen (sauber und mit Fehlern), Liste.

use assert_cmd::Command;
use predicates::prelude::*;

fn cli() -> Command {
    let mut c = Command::cargo_bin("firepixel-cli").expect("exe");
    c.env("FP_NO_SHELL_OPEN", "1").env("FIREPIXEL_LANG", "de");
    c.env(fp_i18n::profile::ENV_TEST_PROFILE, fp_i18n::profile::use_test_profile()); // nie ins echte Nutzerprofil (EB-3)
    c
}

#[test]
fn vorlage_pruefen_liste() {
    let dir = std::env::temp_dir().join(format!("fp-cli-sprache-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    cli().args(["sprache", "vorlage", "fr"]).arg(&dir).assert().success().stdout(predicate::str::contains("Vorlage für „fr“"));
    let fr = dir.join("fr");
    assert!(fr.join("common.ftl").exists());
    // zweites Mal ohne --ueberschreiben: nichts überschrieben
    cli().args(["sprache", "vorlage", "fr"]).arg(&dir).assert().success().stdout(predicate::str::contains("übersprungen"));
    // sauber
    cli().args(["sprache", "pruefen", "--kurz"]).arg(&fr).assert().success().stdout(predicate::str::contains("0 Fehler"));
    // Fehler einbauen
    let f = fr.join("common.ftl");
    let text = std::fs::read_to_string(&f).unwrap_or_default();
    let text = text.replace("test-unsaved = Save changes to “{ $name }”?", "test-unsaved = Enregistrer { $nom } ?") + "\nbtn-ok = OK\n";
    let _ = std::fs::write(&f, text);
    let out = cli().args(["--json", "sprache", "pruefen"]).arg(&fr).assert().failure().get_output().stdout.clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap_or_default();
    assert_eq!(v["ok"], false, "{v}");
    let kinds: Vec<&str> = v["problems"].as_array().map(|a| a.iter().filter_map(|p| p["kind"].as_str()).collect()).unwrap_or_default();
    assert!(kinds.contains(&"platzhalter-unbekannt") && kinds.contains(&"doppelt"), "{kinds:?}");
    // ungültiger Code
    cli().args(["sprache", "vorlage", "x_1"]).arg(&dir).assert().failure();
    cli().args(["sprache", "liste"]).assert().success().stdout(predicate::str::contains("Deutsch").and(predicate::str::contains("English")));
    cli().args(["sprache", "pruefen", "--eingebaut"]).assert().success();
    let _ = std::fs::remove_dir_all(&dir);
}
