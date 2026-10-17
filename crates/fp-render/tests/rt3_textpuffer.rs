//! RT-3 (re 28 D3): Eingabepuffer der Live-Effekte an Text – Golden-Werte aus der Messung des
//! Originals (Rauschmuster). Senkrecht exakt je gemessener Schriftgröße, waagerecht ±1 px (die
//! Messung selbst nennt ±1).

use fp_geom::IRect;
use fp_render::compose::text_effect_buffer;

/// (Größe, raw-top, gemessen oben, gemessen unten) – Zeilen der Tabelle in re 28 D3.
const VERTICAL: [(f64, f64, i32, i32); 8] = [
    (12.0, 60.0, 55, 76),
    (24.0, 60.0, 51, 90),
    (40.0, 60.0, 46, 110),
    (40.0, 80.0, 66, 130),
    (72.0, 60.0, 36, 148),
    (72.0, 80.0, 56, 168),
    (72.0, 97.0, 73, 185),
    (100.0, 80.0, 47, 203),
];

#[test]
fn d3_vertical_golden() {
    for (s, top, t, b) in VERTICAL {
        let r = text_effect_buffer(40.0, top, 50.0, s);
        assert_eq!((r.top, r.bottom), (t, b), "Größe {s}, raw-top {top}");
    }
}

/// Waagerecht: Vorschub aus der Messung (72 pt „Fx Ag“ 188,0 → 11–257; 40 pt „WWWW“ 151,0 →
/// 57–255; 72 pt „i“ 16 → 77–99).
#[test]
fn d3_horizontal_golden() {
    for (left, adv, s, l, r) in [
        (40.0, 188.0, 72.0, 11, 257),
        (80.0, 151.0, 40.0, 57, 255),
        (80.0, 16.0, 72.0, 77, 99),
    ] {
        let b: IRect = text_effect_buffer(left, 80.0, adv, s);
        assert!(
            (b.left - l).abs() <= 1 && (b.right - r).abs() <= 1,
            "Vorschub {adv}: {b:?} statt {l}–{r}"
        );
    }
    // Mitte = raw-left + Vorschub/2, Breite ≈ 1,3 × Vorschub
    let b = text_effect_buffer(40.0, 60.0, 188.0, 72.0);
    assert_eq!((b.left, b.right), (11, 257));
}
