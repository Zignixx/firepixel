//! PAR-X-INSP-2: Verlaufsarten wie die Exporte des Originals (PAR 18/03b,
//! `re\_testdateien\par\px\inspector-fill\03_*.png`, Objekt 50,50–150,120 #808080, Standardverlauf
//! Weiß → Grau) und Verlaufsschritt des Formwerkzeugs nach der Form (18/20b).
//! Logikfälle nach §4b: Wiederholung, Rückgängig, verschobene Griffe bleiben.

use fp_engine::commands::panel::{self, CONTOUR_SHAPE, GRADIENT_SHAPES};
use fp_engine::Session;
use fp_model::ElementId;
use serde_json::json;

fn session() -> (Session, u64) {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("Dokument");
    let id = s.execute("draw.rectangle", json!({"rect": [50, 50, 150, 120], "fillColor": "#808080", "strokeWidth": 0})).expect("Rechteck").value["id"].as_u64().expect("id");
    (s, id)
}

fn handles(s: &Session, id: u64) -> fp_model::FillHandles {
    let d = s.active().expect("Dokument");
    panel::handles_of(&fp_engine::docops::element_on_page(&d.doc, d.page, ElementId(id)).expect("Element")).expect("Griffe")
}

fn red_at(s: &Session, pts: &[(i32, i32)]) -> Vec<u8> {
    let d = s.active().expect("Dokument");
    let img = fp_engine::render::default_renderer().expect("Renderer").render_page(&d.doc, d.page, d.frame).expect("Bild");
    pts.iter().map(|&(x, y)| img.pixel(x, y).r).collect()
}

/// Stichproben (Rotkanal) aus den Exporten des Originals an
/// (50,85) (100,50) (100,85) (75,67) (140,110) (60,115).
const SAMPLES: [(i32, i32); 6] = [(50, 85), (100, 50), (100, 85), (75, 67), (140, 110), (60, 115)];
const ORIGINAL: [(&str, [u8; 6]); 10] = [
    ("Linear", [192, 255, 192, 225, 146, 137]),
    ("Radial", [128, 166, 255, 177, 135, 128]),
    ("Rectangular", [128, 128, 255, 190, 153, 146]),
    ("Conical", [128, 192, 128, 153, 233, 154]),
    ("Satin", [192, 138, 192, 251, 205, 223]),
    ("Folds", [255, 255, 255, 254, 184, 252]),
    ("Elliptical", [128, 128, 255, 164, 128, 128]),
    ("Bars", [128, 255, 255, 192, 153, 153]),
    ("Ripple", [128, 128, 255, 164, 137, 150]),
    ("Waves", [133, 128, 255, 254, 255, 128]),
];

/// Verlaufsfläche, dann jede Art aus der Liste wählen: Ergebnis wie im Original (Radial & Co. um
/// die Mitte, Streifen ≠ Linear, Wellen mit Farben); je Art nur ±2 Stufen Abweichung.
#[test]
fn gradient_kinds_like_original_exports() {
    for (name, want) in ORIGINAL {
        let (mut s, id) = session();
        s.execute("pi.fill", json!({"category": "gradient"})).expect("Verlauf");
        let shape = GRADIENT_SHAPES.iter().find(|g| g.3 == name).expect("Art").1;
        s.execute("pi.fill", json!({"shape": shape})).expect("Art");
        let got = red_at(&s, &SAMPLES);
        for (k, (g, w)) in got.iter().zip(want).enumerate() {
            assert!((*g as i32 - w as i32).abs() <= 2, "{name}: Punkt {:?} Firepixel {g}, Original {w} (alle: {got:?})", SAMPLES[k]);
        }
        let _ = id;
    }
}

/// Supernova und Kontur: die Exporte des Originals tragen eine Textur, darum nur die Form prüfen –
/// Supernova mit hellen Diagonalen durch die Ecken, Kontur hell auf der Mittellinie, dunkel am Rand.
#[test]
fn supernova_and_contour_centered() {
    let (mut s, _) = session();
    s.execute("pi.fill", json!({"category": "gradient", "shape": 7})).expect("Supernova");
    let v = red_at(&s, &[(100, 85), (60, 57), (140, 113), (100, 60), (75, 85)]);
    assert!(v[0] >= 250 && v[1] >= 240 && v[2] >= 240, "Diagonalen hell: {v:?}");
    assert!(v[3] < 235 && v[4] < v[0], "zwischen den Diagonalen dunkler: {v:?}");
    let (mut s, id) = session();
    s.execute("pi.fill", json!({"category": "gradient"})).expect("Verlauf");
    s.execute("pi.fill", json!({"shape": CONTOUR_SHAPE})).expect("Kontur");
    let h = handles(&s, id);
    assert_eq!((h.start.x, h.start.y, h.end.x, h.end.y), (100.0, 85.0, 100.0, 120.0));
    // Rand → Mittellinie über die halbe Höhe (35 px), Mittellinie von x 85 bis 115 weiß
    let v = red_at(&s, &[(50, 85), (85, 85), (115, 85), (100, 50), (100, 85), (67, 85)]);
    assert!(v[0] <= 132 && v[3] <= 132, "Rand grau: {v:?}");
    assert!(v[1] >= 250 && v[2] >= 250 && v[4] >= 250, "Mittellinie weiß: {v:?}");
    assert!((180..=200).contains(&v[5]), "halber Weg ≈ #c0: {v:?}");
}

/// Art wechseln setzt nur unveränderte Vorgabegriffe neu; einmal und mehrfach dieselbe Art sind gleich,
/// Rückgängig stellt Art und Griffe wieder her, verschobene/gedrehte Griffe bleiben.
#[test]
fn switching_kind_resets_only_default_handles() {
    let (mut s, id) = session();
    s.execute("pi.fill", json!({"category": "gradient"})).expect("Verlauf");
    let lin = handles(&s, id);
    let n_lin = s.active().unwrap().history.undo_count();
    s.execute("pi.fill", json!({"shape": 3})).expect("Radial");
    let rad = handles(&s, id);
    assert_eq!((rad.start.x, rad.start.y, rad.end.x, rad.end.y, rad.second.x, rad.second.y), (100.0, 85.0, 150.0, 85.0, 100.0, 50.0));
    // (Verlauf hält standardmäßig 20 Schritte; 5 Wiederholungen reichen)
    for _ in 0..5 {
        s.execute("pi.fill", json!({"shape": 3})).expect("Radial");
    }
    assert_eq!(handles(&s, id), rad, "dieselbe Art wiederholt ändert nichts");
    s.execute("pi.fill", json!({"shape": 12})).expect("Streifen");
    assert_eq!(handles(&s, id), rad, "Streifen hat dieselben Vorgabegriffe wie Radial");
    s.execute("pi.fill", json!({"shape": 14})).expect("Wellen");
    let w = handles(&s, id);
    assert_eq!((w.end.x, w.end.y), (125.0, 102.5), "Wellen: halber Weg zur rechten unteren Ecke");
    s.execute("edit.undo", json!({})).expect("undo");
    assert_eq!(handles(&s, id), rad);
    // zurück bis Linear (die gleichen Wahlen zählen höchstens als Schritte ohne Änderung)
    while s.active().unwrap().history.undo_count() > n_lin {
        s.execute("edit.undo", json!({})).expect("undo");
    }
    assert_eq!(handles(&s, id), lin, "Rückgängig bis Linear: Griffe oben → unten");
    // gedrehte Griffe (Winkel 0°) bleiben beim Artwechsel
    s.execute("pi.fill", json!({"angle": 0})).expect("Winkel");
    let turned = handles(&s, id);
    s.execute("pi.fill", json!({"shape": 3})).expect("Radial");
    assert_eq!(handles(&s, id), turned, "vom Nutzer gedrehte Griffe bleiben");
}

/// Formwerkzeug: Verlaufsschritt heißt nach der Form („Rechteck“, „Ellipse“), in Englisch übersetzt.
#[test]
fn shape_tool_step_named_after_shape() {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200})).expect("Dokument");
    let label = |s: &Session| s.active().and_then(|d| d.history.undo_label().map(|l| l.to_string())).unwrap_or_default();
    s.execute("tool.shape", json!({"draw": "draw.rectangle", "params": {"rect": [10, 10, 60, 40]}, "fill": {"category": "solid", "color": "#ff0000"}})).expect("Rechteck");
    assert_eq!(label(&s), "Rechteck");
    s.execute("tool.shape", json!({"draw": "draw.ellipse", "params": {"rect": [70, 10, 120, 40]}})).expect("Ellipse");
    assert_eq!(label(&s), "Ellipse");
    // englische Oberfläche: „Rectangle“. (Über die Brücke mit englischer Antwortsprache bleibt der
    // Verlaufsname in der Sprache der Oberfläche, np1 Nr. 5 / tw_f4::history_names_are_german.)
    fp_i18n::set_language("en");
    s.execute("tool.shape", json!({"draw": "draw.rectangle", "params": {"rect": [10, 50, 60, 90]}})).expect("Rectangle");
    assert_eq!(label(&s), "Rectangle");
    fp_i18n::set_language("de");
    // ein Schritt je Form; Rückgängig entfernt genau die letzte
    let n = s.active().unwrap().history.undo_count();
    s.execute("edit.undo", json!({})).expect("undo");
    assert_eq!(s.active().unwrap().history.undo_count(), n - 1);
    assert_eq!(label(&s), "Ellipse");
}
