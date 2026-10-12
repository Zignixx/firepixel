//! Warnungen bei immer aktiven Menübefehlen ohne passendes Ziel (FV-5, BEFUNDE EB, Rohdaten
//! `re\_testdateien\ux\paritaet\dialoge\o_dialoge_bericht.txt` Z01–Z18): Lage „Dokument ohne
//! Auswahl“ → Warnung mit dem Wortlaut des Originals, kein Verlaufsschritt; „Nicht mehr anzeigen“
//! schaltet sie ab, „Warnmeldungen zurücksetzen“ wieder ein.

mod common;

use common::*;
use fp_engine::{EngineError, Session, SessionEvent};
use serde_json::json;

fn history_len(s: &Session) -> usize {
    s.active().map(|d| d.history.len()).unwrap_or(0)
}

fn warning_events(s: &mut Session) -> Vec<fp_engine::TargetWarning> {
    s.drain_events()
        .into_iter()
        .filter_map(|e| match e {
            SessionEvent::Warning(w) => Some(w),
            _ => None,
        })
        .collect()
}

/// Befehl in der Lage „Dokument ohne Auswahl“: Warnung mit Schlüssel und Wortlaut, kein Schritt;
/// abgeschaltet keine Warnung (Befehl bleibt ohne Wirkung); zurückgesetzt wieder da.
fn check(command: &str, key: &str, text: &str) {
    fp_i18n::with_language("de", || {
        let mut s = doc_session(300.0, 200.0);
        // Lage S1: ein Objekt im Dokument, nichts ausgewählt
        rect(&mut s, 10.0, 10.0, 50.0, 50.0);
        ex(&mut s, "select.none", json!({}));
        assert!(s.is_available(command), "{command}: im Menü immer aktiv");
        let steps = history_len(&s);
        let model = s.active().map(|d| d.doc.clone());
        s.drain_events();

        let w = s.target_warning(command).unwrap_or_else(|| panic!("{command}: keine Warnung"));
        assert_eq!(w.key, key, "{command}");
        assert_eq!(w.text, text, "{command}");
        assert_eq!(w.command, command);
        assert!(!w.suppressed);
        assert!(w.pref.is_some(), "{command}: Kästchen „Nicht mehr anzeigen“");
        assert!(w.choices.is_empty());

        match s.execute(command, json!({})) {
            Err(EngineError::Selection(m)) => assert_eq!(m, text, "{command}"),
            other => panic!("{command}: {other:?}"),
        }
        let ev = warning_events(&mut s);
        assert_eq!(ev.len(), 1, "{command}: genau eine Warnung");
        assert_eq!(ev[0].key, key);
        assert_eq!(history_len(&s), steps, "{command}: kein Verlaufsschritt");
        assert!(s.active().map(|d| d.doc.clone()).zip(model).is_some_and(|(a, b)| std::sync::Arc::ptr_eq(&a, &b)), "{command}: Dokument unverändert");

        // „Nicht mehr anzeigen“
        s.set_warning_disabled(&w, true);
        let again = s.target_warning(command).expect("Warnung bleibt bekannt");
        assert!(again.suppressed, "{command}: abgeschaltet");
        assert!(s.execute(command, json!({})).is_err());
        assert!(warning_events(&mut s).is_empty(), "{command}: abgeschaltet keine Warnung");
        assert_eq!(history_len(&s), steps);
        // Warnungen mit demselben Schlüssel (andere Befehle) sind mit abgeschaltet
        for (k, cmds) in fp_engine::warnings::all() {
            if k == key {
                for c in cmds {
                    assert!(s.target_warning(c).is_some_and(|w| w.suppressed), "{c}");
                }
            }
        }

        // „Warnmeldungen zurücksetzen“ (Befehle-Menü bzw. Voreinstellungen)
        s.prefs.reset_warnings();
        assert!(s.target_warning(command).is_some_and(|w| !w.suppressed), "{command}: nach Zurücksetzen wieder da");
        assert!(s.execute(command, json!({})).is_err());
        assert_eq!(warning_events(&mut s).len(), 1);
    });
}

macro_rules! warn_test {
    ($($name:ident: $cmd:literal, $key:literal, $text:literal;)*) => {
        $(
            #[test]
            fn $name() {
                check($cmd, $key, $text);
            }
        )*
    };
}

const Z01: &str = "Zum Öffnen, Vereinigen, Überschneiden und Zuschneiden müssen zwei oder mehr Pfade ausgewählt werden.";
const Z05: &str = "Zum Vereinfachen, Erweitern und Einlassen ist mindestens ein Pfad erforderlich.";

#[test]
fn z01_bis_z07_wortlaut_gleich_den_konstanten() {
    // Die Makro-Fälle unten wiederholen den Wortlaut; hier gegen die gemeinsamen Texte geprüft.
    fp_i18n::with_language("de", || {
        assert_eq!(fp_i18n::t("msg-warn-combine-paths"), Z01);
        assert_eq!(fp_i18n::t("msg-warn-alter-path"), Z05);
    });
}

warn_test! {
    z01_vereinigen: "modify.combinePaths.union", "combinePaths", "Zum Öffnen, Vereinigen, Überschneiden und Zuschneiden müssen zwei oder mehr Pfade ausgewählt werden.";
    z02_ueberschneiden: "modify.combinePaths.intersect", "combinePaths", "Zum Öffnen, Vereinigen, Überschneiden und Zuschneiden müssen zwei oder mehr Pfade ausgewählt werden.";
    z03_oeffnen: "modify.combinePaths.punch", "combinePaths", "Zum Öffnen, Vereinigen, Überschneiden und Zuschneiden müssen zwei oder mehr Pfade ausgewählt werden.";
    z04_zuschneiden: "modify.combinePaths.crop", "combinePaths", "Zum Öffnen, Vereinigen, Überschneiden und Zuschneiden müssen zwei oder mehr Pfade ausgewählt werden.";
    z05_vereinfachen: "modify.alterPath.simplify", "alterPath", "Zum Vereinfachen, Erweitern und Einlassen ist mindestens ein Pfad erforderlich.";
    z06_strich_erweitern: "modify.alterPath.expandStroke", "alterPath", "Zum Vereinfachen, Erweitern und Einlassen ist mindestens ein Pfad erforderlich.";
    z07_pfad_einlassen: "modify.alterPath.insetPath", "alterPath", "Zum Vereinfachen, Erweitern und Einlassen ist mindestens ein Pfad erforderlich.";
    z08_instanzen_tweenen: "modify.symbol.tweenInstances", "tweenInstances", "Wählen Sie zwei oder mehr Instanzen desselben Bibliothekssymbols aus, bevor Sie den Tween-Befehl verwenden.";
    z09_symbol_teilen: "modify.symbol.breakApart", "breakApart", "Um die Verknüpfung zwischen einem Bibliothekssymbol und den Instanzen zu entfernen, müssen Sie zuerst ein Symbol bzw. eine Instanz auswählen.";
    z14_hotspot: "edit.insert.hotspot", "insertHotspot", "Wählen Sie mindestens ein Objekt aus, bevor Sie den Befehl zum Einfügen von Hotspots verwenden.";
    z15_rechteckiges_segment: "edit.insert.slice", "insertSlice", "Wählen Sie mindestens ein Objekt aus, bevor Sie den Befehl zum Einfügen eines Segments verwenden.";
    z16_polygonsegment: "edit.insert.polygonSlice", "insertSlice", "Wählen Sie mindestens ein Objekt aus, bevor Sie den Befehl zum Einfügen eines Segments verwenden.";
    z17_aehnliches_auswaehlen: "select.similar", "selectSimilar", "Nehmen Sie mit dem Auswahlrahmen, dem Lasso oder dem Zauberstab eine Pixelauswahl vor, bevor Sie den Befehl \"Ähnliches auswählen\" verwenden.";
    z18_weiche_kante: "select.feather", "feather", "Bevor Sie den Befehl \"Weiche Kante\" verwenden, nehmen Sie mit dem Auswahlrahmen, dem Lasso oder dem Zauberstab eine Pixelauswahl vor.";
}

/// Z10: „Popupmenü hinzufügen…“ ohne Hotspot/Segment fragt (ohne Kästchen) nach Hotspot oder Segment.
#[test]
fn z10_popupmenue_fragt_hotspot_oder_segment() {
    fp_i18n::with_language("de", || {
        let mut s = doc_session(300.0, 200.0);
        rect(&mut s, 10.0, 10.0, 50.0, 50.0);
        ex(&mut s, "select.none", json!({}));
        let steps = history_len(&s);
        let w = s.target_warning("modify.popUpMenu.addPopUpMenu").expect("Rückfrage");
        assert_eq!(w.key, "addPopUpMenu");
        assert_eq!(w.text, "Verhalten werden über das Verhalten-Bedienfeld auf Hotspot- oder Segmentobjekte angewendet. Hotspot oder Segment erstellen?");
        assert_eq!(w.pref, None, "Rückfrage ohne „Nicht mehr anzeigen“");
        let choices: Vec<(&str, &str)> = w.choices.iter().map(|c| (c.label.as_str(), c.command.as_str())).collect();
        assert_eq!(choices, [("&Hotspot", "edit.insert.hotspot"), ("&Segment", "edit.insert.slice")]);
        assert!(s.execute("modify.popUpMenu.addPopUpMenu", json!({"items": []})).is_err());
        assert_eq!(history_len(&s), steps);
        // abschalten geht nicht (kein Kästchen)
        s.set_warning_disabled(&w, true);
        assert!(s.target_warning("modify.popUpMenu.addPopUpMenu").is_some_and(|w| !w.suppressed));
        // mit Objekt: „Hotspot“ legt einen Hotspot an, danach keine Rückfrage mehr
        let id = rect(&mut s, 60.0, 60.0, 90.0, 90.0);
        select(&mut s, &[id]);
        assert!(s.target_warning("modify.popUpMenu.addPopUpMenu").is_some(), "kein Web-Objekt ausgewählt");
        assert!(s.target_warning("edit.insert.hotspot").is_none());
        ex(&mut s, "edit.insert.hotspot", json!({}));
        assert!(s.target_warning("modify.popUpMenu.addPopUpMenu").is_none(), "Hotspot ausgewählt");
    });
}

/// Mit passendem Ziel keine Warnung.
#[test]
fn mit_ziel_keine_warnung() {
    let mut s = doc_session(300.0, 200.0);
    let a = rect(&mut s, 10.0, 10.0, 50.0, 50.0);
    let b = ellipse(&mut s, 30.0, 30.0, 80.0, 80.0);
    select(&mut s, &[a]);
    // ein Pfad: Vereinfachen ja, Kombinieren nein
    assert!(s.target_warning("modify.alterPath.simplify").is_none());
    assert!(s.target_warning("modify.combinePaths.union").is_some());
    assert!(s.target_warning("edit.insert.hotspot").is_none());
    assert!(s.target_warning("select.similar").is_none());
    assert!(s.target_warning("select.feather").is_some(), "keine Pixelauswahl");
    select(&mut s, &[a, b]);
    assert!(s.target_warning("modify.combinePaths.union").is_none());
    ex(&mut s, "modify.combinePaths.union", json!({}));
    assert!(warning_events(&mut s).is_empty());
    ex(&mut s, "select.pixels", json!({"shape": "rect", "rect": [10, 10, 30, 30]}));
    assert!(s.target_warning("select.feather").is_none());
    // Befehle ohne Warnung des Originals
    assert!(s.target_warning("edit.copy").is_none());
    assert!(s.target_warning("unbekannt").is_none());
}

/// Ohne Dokument gibt es keine Warnung (die Befehle sind dann gesperrt).
#[test]
fn ohne_dokument_keine_warnung() {
    let s = Session::default();
    for (_, cmds) in fp_engine::warnings::all() {
        for c in cmds {
            assert!(s.target_warning(c).is_none(), "{c}");
            assert!(!s.is_available(c), "{c}");
        }
    }
}

/// Die Abschaltung landet in den Voreinstellungen (gespeichert und wieder gelesen).
#[test]
fn nicht_mehr_anzeigen_in_den_voreinstellungen_gespeichert() {
    let dir = std::env::temp_dir().join(format!("fp-fv5-warn-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("prefs.json");
    let mut s = doc_session(100.0, 100.0);
    s.prefs_path = Some(path.clone());
    let w = s.target_warning("select.feather").expect("Warnung");
    s.set_warning_disabled(&w, true);
    let p = fp_engine::Preferences::load(&path).expect("Voreinstellungen lesen");
    assert!(p.get_bool("DisableNoticeFeather", false));
    let mut s2 = Session::new(p);
    ex(&mut s2, "file.new", json!({"width": 100, "height": 100}));
    assert!(s2.target_warning("select.feather").is_some_and(|w| w.suppressed));
    s2.prefs.reset_warnings();
    assert!(s2.target_warning("select.feather").is_some_and(|w| !w.suppressed));
    let _ = std::fs::remove_dir_all(&dir);
}

/// Englische Texte vorhanden und verschieden vom Deutschen.
#[test]
fn englische_warnungen() {
    fp_i18n::with_language("en", || {
        let s = {
            let mut s = doc_session(100.0, 100.0);
            ex(&mut s, "select.none", json!({}));
            s
        };
        let w = s.target_warning("modify.combinePaths.union").expect("Warnung");
        assert_eq!(w.text, "Two or more paths must be selected to punch, union, intersect or crop.");
        assert_eq!(fp_i18n::t("msg-warn-dont-show"), "Don't s&how again");
    });
}

/// Z11–Z13: Selektives JPEG ohne Pixelauswahl bzw. Maske wie im Original.
#[test]
fn z11_bis_z13_selektives_jpeg() {
    fp_i18n::with_language("de", || {
        let mut s = doc_session(300.0, 200.0);
        rect(&mut s, 10.0, 10.0, 50.0, 50.0);
        ex(&mut s, "select.none", json!({}));
        let steps = history_len(&s);
        // Z11 still, kein Schritt
        ex(&mut s, "modify.selectiveJpeg.saveSelectionAsJpegMask", json!({}));
        assert_eq!(history_len(&s), steps);
        assert!(s.active().and_then(|d| d.doc.pages[d.page].jpeg_mask.as_ref()).is_none());
        // Z12 Schritt „JPEG-Maske laden“, auch ohne Maske
        ex(&mut s, "modify.selectiveJpeg.restoreJpegMaskAsSelection", json!({}));
        assert_eq!(history_len(&s), steps + 1);
        assert_eq!(s.undo_redo_labels().0.as_deref(), Some("JPEG-Maske laden"));
        // Z13 Schritt „JPEG-Maske löschen“
        ex(&mut s, "modify.selectiveJpeg.removeJpegMask", json!({}));
        assert_eq!(history_len(&s), steps + 2);
        assert_eq!(s.undo_redo_labels().0.as_deref(), Some("JPEG-Maske löschen"));
        // Rückgängig nimmt die leeren Schritte zurück
        ex(&mut s, "edit.undo", json!({}));
        ex(&mut s, "edit.undo", json!({}));
        assert_eq!(s.undo_redo_labels().1.as_deref(), Some("JPEG-Maske laden"));
        assert!(warning_events(&mut s).is_empty());

        // mit Maske: laden stellt die Auswahl her, löschen entfernt die Maske
        ex(&mut s, "select.pixels", json!({"shape": "rect", "rect": [10, 10, 30, 30]}));
        ex(&mut s, "modify.selectiveJpeg.saveSelectionAsJpegMask", json!({}));
        assert!(s.active().and_then(|d| d.doc.pages[d.page].jpeg_mask.as_ref()).is_some());
        ex(&mut s, "select.none", json!({}));
        ex(&mut s, "modify.selectiveJpeg.restoreJpegMaskAsSelection", json!({}));
        assert!(s.active().is_some_and(|d| d.selection.has_pixels()));
        assert_eq!(s.undo_redo_labels().0.as_deref(), Some("JPEG-Maske laden"));
        ex(&mut s, "modify.selectiveJpeg.removeJpegMask", json!({}));
        assert!(s.active().and_then(|d| d.doc.pages[d.page].jpeg_mask.as_ref()).is_none());
        ex(&mut s, "edit.undo", json!({}));
        assert!(s.active().and_then(|d| d.doc.pages[d.page].jpeg_mask.as_ref()).is_some(), "Rückgängig stellt die Maske her");
    });
}

/// Wiederholen ohne zurückgenommenen Schritt: „Wiederholen: <letzter Schritt>“ (PARITAET_B).
#[test]
fn wiederholen_zeigt_den_letzten_schritt() {
    fp_i18n::with_language("de", || {
        let mut s = doc_session(300.0, 200.0);
        rect(&mut s, 10.0, 10.0, 50.0, 50.0);
        ex(&mut s, "select.none", json!({}));
        assert!(s.is_available("edit.redo"));
        assert_eq!(s.undo_redo_labels().1.as_deref(), Some("Auswahl aufheben"));
    });
}
