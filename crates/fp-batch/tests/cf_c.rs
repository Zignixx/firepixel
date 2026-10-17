//! CF-C: Befunde des Klicktests CT-C im Batch (ohne KI-Modell).

mod support;

use fp_batch::Recipe;
use std::path::PathBuf;
use support::*;

/// CT-C-04: Zweimal rekursiv über denselben Ordner – der zweite Lauf nimmt `batch-output` nicht als
/// Eingabe mit und schreibt dieselben Ergebnisse nach `batch-output-2`.
#[test]
fn ct_c_04_recursive_run_skips_own_output_folders() {
    let dir = temp("ct-c-04");
    write_png(&dir.join("a.png"), 6, 4);
    std::fs::create_dir_all(dir.join("unter")).expect("Unterordner");
    write_png(&dir.join("unter").join("b.png"), 4, 6);
    let mut r = Recipe::named("t");
    r.output.format = "png".into();
    let collect = || fp_batch::collect(std::slice::from_ref(&dir), true, false, &[]).items;
    let items = collect();
    assert_eq!(items.len(), 2);
    let rep = fp_batch::run(&r, &items, &fp_batch::RunOptions::default()).expect("Lauf 1");
    assert_eq!(rep.totals.done, 2);
    assert!(dir.join("batch-output").is_dir(), "{:?}", files_under(&dir));
    // Lauf 2: nur die beiden Quellen
    let items = collect();
    let names: Vec<PathBuf> = items.iter().map(|i| i.path.clone()).collect();
    assert_eq!(
        items.len(),
        2,
        "eigene Ausgabe nicht erneut gesammelt: {names:?}"
    );
    let rep = fp_batch::run(&r, &items, &fp_batch::RunOptions::default()).expect("Lauf 2");
    assert_eq!(rep.totals.done, 2);
    let out2 = dir.join("batch-output-2");
    assert!(out2.is_dir());
    assert!(
        !out2.join("batch-output").exists(),
        "keine verschachtelte Ausgabe"
    );
    // dritter Lauf: auch batch-output-2 bleibt draußen
    assert_eq!(collect().len(), 2);
    // Ordner mit Bericht unter eigenem Namen ebenfalls; gewöhnliche Ordner werden gesammelt
    assert!(fp_batch::item::is_batch_output_dir(
        &dir.join("batch-output-12")
    ));
    assert!(!fp_batch::item::is_batch_output_dir(
        &dir.join("batch-outputs")
    ));
    assert!(!fp_batch::item::is_batch_output_dir(&dir.join("unter")));
    // ausdrücklich gewählter Ausgabeordner als Wurzel: wird gesammelt (der Nutzer will genau ihn)
    assert_eq!(
        fp_batch::collect(&[dir.join("batch-output")], true, false, &[])
            .items
            .len(),
        2
    );
    let _ = std::fs::remove_dir_all(&dir);
}
