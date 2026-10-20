//! KI-Auswahl (Agent KI): ohne Modelle gibt es verständliche Meldungen statt einer Panik.
//! Eigene Testdatei (eigener Prozess), weil der Test die Modellsuche umstellt.

mod common;

use common::*;
use fp_engine::commands::ai;
use serde_json::json;

const DISK: (f32, f32) = (120.0, 150.0);
#[test]
fn missing_models_give_clear_message() {
    // eigener leerer Modellordner nur für diesen Prozess-Test über den Parameter `model` geht nicht –
    // deshalb: Modellordner der Sitzung auf einen leeren Ordner und Suche darauf beschränken.
    let dir = std::env::temp_dir().join(format!("fp-ai-leer-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    ai::fp_ai::download::disable();
    let mut s = doc_session(400.0, 300.0);
    ai::fp_ai::models::set_search_override(vec![dir.clone()]);
    let r = s.execute("select.aiObject", json!({"point": [DISK.0, DISK.1]}));
    let r2 = s.execute("select.aiSubject", json!({}));
    let missing = ai::missing_model(&s, "select.aiSubject", &json!({}));
    ai::fp_ai::models::set_search_override(Vec::new());
    let _ = std::fs::remove_dir_all(&dir);
    let e = r.expect_err("ohne Modell");
    assert!(e.to_string().contains("KI-Modell"), "{e}");
    assert!(e.to_string().contains("fehlt"), "{e}");
    assert!(r2.is_err());
    assert!(matches!(missing, Ok(Some(_))));
    // Download ist in Tests gesperrt
    let p = ai::fp_ai::Progress::new();
    assert!(matches!(
        ai::fp_ai::download::download(ai::fp_ai::models::ModelId::Sam2Tiny, &p),
        Err(ai::fp_ai::AiError::DownloadDisabled)
    ));
    // ungültige Parameter: verständliche Fehler, keine Panik
    assert!(s.execute("select.aiObject", json!({})).is_err());
    assert!(
        s.execute("select.aiObject", json!({"points": "x"}))
            .is_err()
    );
    assert!(
        s.execute(
            "select.aiObject",
            json!({"point": [1, 2], "model": "gibtsnicht"})
        )
        .is_err()
    );
    assert!(
        s.execute("select.aiRefineEdge", json!({})).is_err(),
        "ohne Pixelauswahl nicht verfügbar"
    );
    let models = ex(&mut s, "ai.models", json!({}));
    assert_eq!(models["models"].as_array().map(|a| a.len()), Some(8));
}
