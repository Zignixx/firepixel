//! Mehrsprachigkeit von fp-engine (Phase 6, I18N-ENGINE): Fehlermeldungen, Verlaufsschritte,
//! Statushinweise und Standardnamen kommen aus fp-i18n (`messages.ftl`, `filter.ftl`).
//!
//! Die Sprache ist global; alle Tests dieser Datei laufen unter einer Sperre nacheinander und
//! stellen am Ende Deutsch wieder her (andere Testdateien laufen in eigenen Prozessen).

use fp_engine::{EngineError, Session};
use serde_json::json;
use std::collections::BTreeSet;
use std::sync::Mutex;

static LOCK: Mutex<()> = Mutex::new(());

fn lock() -> std::sync::MutexGuard<'static, ()> {
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Stellt beim Verlassen Deutsch wieder her (auch nach einer Panik).
struct German;
impl Drop for German {
    fn drop(&mut self) {
        fp_i18n::set_language("de");
    }
}

fn keys(lang: &str) -> BTreeSet<String> {
    fp_i18n::keys(lang)
}

/// Alle Schlüssel-Literale (`"err-…"`, `"msg-…"`, `"filter-…"`, `"effect-…"`, `"blend-…"`) in
/// einem Quellordner.
fn literal_keys(dir: &std::path::Path, out: &mut BTreeSet<String>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            literal_keys(&p, out);
        } else if p.extension().and_then(|x| x.to_str()) == Some("rs") {
            let text = std::fs::read_to_string(&p).unwrap_or_default();
            for (i, _) in text.match_indices('"') {
                let rest = &text[i + 1..];
                let end = rest.find('"').unwrap_or(0);
                let lit = &rest[..end];
                let is_key = ["err-", "msg-", "filter-", "effect-", "blend-", "ai-"].iter().any(|p| lit.starts_with(p))
                    && lit.len() > 5
                    && lit.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
                    && !lit.ends_with('-');
                if is_key {
                    out.insert(lit.to_string());
                }
            }
        }
    }
}

#[test]
fn every_error_variant_has_keys_in_de_and_en() {
    let _g = lock();
    let _de = German;
    let (de, en) = (keys("de"), keys("en"));
    for e in fp_engine::error::all_variants_for_tests() {
        let k = e.message_key();
        assert!(de.contains(k) && en.contains(k), "Schlüssel {k} fehlt (de: {}, en: {})", de.contains(k), en.contains(k));
        for lang in ["de", "en"] {
            assert!(fp_i18n::set_language(lang));
            let s = e.to_string();
            assert!(!s.contains("err-") && !s.is_empty(), "{lang}: Rohschlüssel in „{s}“");
        }
    }
    // Begründungen der Voraussetzungen
    use fp_engine::{ObjectKind, Requirement};
    let reqs = [
        Requirement::Document,
        Requirement::Selection,
        Requirement::MinSelected(2),
        Requirement::PixelSelection,
        Requirement::SelectionOrPixels,
        Requirement::AllOf(ObjectKind::Path),
        Requirement::AnyOf(ObjectKind::Text),
        Requirement::Clipboard,
        Requirement::CanUndo,
        Requirement::CanRedo,
        Requirement::HasPath,
        Requirement::MultiplePages,
        Requirement::MultipleFrames,
    ];
    for r in reqs {
        let k = r.reason_key();
        assert!(de.contains(k) && en.contains(k), "{k}");
    }
}

/// Jeder Schlüssel, den fp-engine (und die Hilfs-Crates) im Code nennen, gibt es in de und en.
#[test]
fn all_keys_used_in_code_exist() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut used = BTreeSet::new();
    for c in ["fp-engine/src", "fp-model/src", "fp-color/src", "fp-io/src", "fp-export/src", "fp-ai/src", "fp-script/src"] {
        literal_keys(&root.join(c), &mut used);
    }
    let (de, en) = (keys("de"), keys("en"));
    let missing: Vec<&String> = used.iter().filter(|k| !de.contains(*k) || !en.contains(*k)).collect();
    assert!(missing.is_empty(), "Schlüssel ohne Text: {missing:#?}");
    assert!(used.len() > 200, "nur {} Schlüssel gefunden", used.len());
}

#[test]
fn german_wording_unchanged() {
    let _g = lock();
    let _de = German;
    fp_i18n::set_language("de");
    assert_eq!(EngineError::NotFound(fp_i18n::t("err-nf-page")).to_string(), "Seite nicht gefunden");
    assert_eq!(EngineError::UnknownCommand("a.b".into()).to_string(), "unbekannter Befehl „a.b“");
    assert_eq!(EngineError::NoDocument.to_string(), "kein Dokument geöffnet");
    assert_eq!(EngineError::ElementNotFound(12345).to_string(), "Element 12345 nicht gefunden");
    assert_eq!(EngineError::BackendMissing("err-backend-pathops").to_string(), "für diese Operation fehlt Pfadoperationen (fp-vector)");
    assert_eq!(EngineError::History("err-req-can-undo").to_string(), "nichts rückgängig zu machen");
    let mut s = Session::default();
    let e = s.execute("modify.group", json!({})).unwrap_err();
    assert_eq!(e.to_string(), "Befehl „modify.group“ ist nicht verfügbar: kein Dokument geöffnet");
    s.execute("file.new", json!({"width": 100, "height": 80})).unwrap();
    let e = s.execute("draw.rectangle", json!({"rect": "x"})).unwrap_err();
    assert_eq!(e.to_string(), "ungültige Parameter für „draw.rectangle“: Parameter „rect“ hat einen ungültigen Wert");
    let e = s.execute("modify.group", json!({})).unwrap_err();
    assert_eq!(e.message_key(), "err-unavailable");
    assert!(e.to_string().ends_with("keine Auswahl"), "{e}");
    // Standardname der ersten Ebene, Verlaufsschritt
    let d = s.active().unwrap();
    assert!(d.doc.pages[0].root.layers.iter().any(|l| matches!(l, fp_model::LayerSlot::Own(l) if l.name == "Ebene 1")));
    s.execute("draw.rectangle", json!({"rect": [10, 10, 40, 40]})).unwrap();
    s.execute("edit.move", json!({"dx": 3, "dy": 0})).unwrap();
    assert_eq!(s.active().unwrap().history.undo_label(), Some("Verschieben"));
}

/// Im englischen Modus erzeugen Befehle englische Verlaufsnamen, Standardnamen und Fehler.
#[test]
fn english_history_names_errors_and_defaults() {
    let _g = lock();
    let _de = German;
    assert!(fp_i18n::set_language("en"));
    let mut s = Session::default();
    let e = s.execute("modify.group", json!({})).unwrap_err();
    assert_eq!(e.to_string(), "command “modify.group” is not available: no document open");
    s.execute("file.new", json!({"width": 120, "height": 90})).unwrap();
    {
        let d = s.active().unwrap();
        assert!(d.doc.pages[0].root.layers.iter().any(|l| matches!(l, fp_model::LayerSlot::Own(l) if l.name == "Layer 1")), "Standardebene");
    }
    s.execute("draw.rectangle", json!({"rect": [10, 10, 60, 50]})).unwrap();
    s.execute("edit.move", json!({"dx": 5, "dy": 0})).unwrap();
    assert_eq!(s.active().unwrap().history.undo_label(), Some("Move"));
    // Filter: Name aus filter.ftl
    s.execute("filter.blur.gaussian", json!({"radius": 2.0, "live": true})).unwrap();
    assert_eq!(s.active().unwrap().history.undo_label(), Some("Gaussian Blur"));
    // Fehler mit Auswahl-Detail
    s.execute("select.none", json!({})).ok();
    let e = s.execute("filter.blur.gaussian", json!({})).unwrap_err();
    assert!(e.to_string().contains("not available") || e.to_string().contains("nothing"), "{e}");
    let e = s.execute("draw.rectangle", json!({"rect": "x"})).unwrap_err();
    assert_eq!(e.to_string(), "invalid parameters for “draw.rectangle”: parameter “rect” has an invalid value");
    // Neue Ebene
    s.execute("layer.add", json!({})).unwrap();
    let names: Vec<String> = s.active().unwrap().doc.pages[0]
        .root
        .layers
        .iter()
        .filter_map(|l| if let fp_model::LayerSlot::Own(l) = l { Some(l.name.clone()) } else { None })
        .collect();
    assert!(names.iter().all(|n| !n.starts_with("Ebene")), "{names:?}");
    // Rückgängig ohne Schritte
    let mut t = Session::default();
    t.execute("file.new", json!({"width": 10, "height": 10})).unwrap();
    let e = t.undo().unwrap_err();
    assert_eq!(e.to_string(), "nothing to undo");
    // Testsprache: markierte Texte, keine Rohschlüssel
    assert!(fp_i18n::set_language(fp_i18n::PSEUDO_LANG));
    let e = t.undo().unwrap_err().to_string();
    assert!(e.starts_with("[!!") && !e.contains("err-"), "{e}");
}

/// Hilfsschlüssel aus fp-color und fp-model zeigen auf vorhandene Texte.
#[test]
fn blend_and_effect_label_keys_exist() {
    let (de, en) = (keys("de"), keys("en"));
    for (m, _) in fp_color::BlendMode::ALL {
        let k = m.label_key().expect("Name");
        assert!(de.contains(&k) && en.contains(&k), "{k}");
    }
    for (moa, _) in fp_model::effects::EFFECT_MOA_IDS {
        let e = fp_model::effects::Effect::new(moa, true);
        if let Some(k) = e.label_key() {
            assert!(de.contains(k) && en.contains(k), "{k}");
        }
    }
}
