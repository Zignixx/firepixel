//! F26 (T-9-Rest, re 39 §2.2 Punkt 2): Der simple-Läufer liest wie der natürliche alle
//! Empfindlichkeiten des gemeinsamen Stempels – Streuung (vom Läufer selbst), HLS-Farbjitter (eine
//! Spitze) und Schwärze –, nur Abstand und Fluss nicht.

use fp_color::Argb;
use fp_geom::{IRect, Vec2};
use fp_vector::stroke::stroke_layer;
use fp_vector::{
    BrushParams, BrushType, PathContour, PointDynamics, SenseSource, SenseTarget, Stroke,
    VectorPath,
};

const W: usize = 420;

fn simple() -> BrushParams {
    let mut b = BrushParams {
        diameter: 6,
        max_count: 14,
        kind: BrushType::Simple,
        antialiased: false,
        spacing: 150,
        ..Default::default()
    };
    b.sense = Default::default();
    b
}

fn layer(b: &BrushParams, color: Argb) -> Vec<[u8; 4]> {
    let mut c = PathContour::new(vec![Vec2::new(60.0, 100.0), Vec2::new(360.0, 100.0)], false);
    c.dynamics = vec![
        PointDynamics {
            pressure: 1.0,
            velocity: 0.5,
            seed: 7,
            ..Default::default()
        };
        2
    ];
    stroke_layer(
        &VectorPath::new(vec![c]),
        &Stroke::new(b.clone(), color),
        IRect::new(0, 0, W as i32, 200),
    )
}

/// Größter senkrechter Abstand gedeckter Pixel von der Linie y = 100.
fn spread(px: &[[u8; 4]]) -> usize {
    (0..200)
        .filter(|&y| (0..W).any(|x| px[y * W + x][3] > 0))
        .map(|y: usize| y.abs_diff(100))
        .max()
        .unwrap_or(0)
}

#[test]
fn scatter_random_spreads_simple_stamps() {
    let plain = layer(&simple(), Argb::new(255, 0, 0, 0));
    let mut b = simple();
    b.sense[SenseTarget::Scatter as usize][SenseSource::Random as usize] =
        BrushParams::sense_from_permille(1000);
    let scattered = layer(&b, Argb::new(255, 0, 0, 0));
    assert!(
        spread(&plain) <= 4,
        "ohne Streuung schmal: {}",
        spread(&plain)
    );
    // Streukonstante 10·D′: Versätze bis ±30 px bei D 6
    assert!(
        spread(&scattered) > 8,
        "Streuung wirkt im simple-Läufer: {}",
        spread(&scattered)
    );
    assert!(
        spread(&scattered) <= 40,
        "innerhalb ±10·D′/2 + Spitze: {}",
        spread(&scattered)
    );
}

#[test]
fn hue_jitter_colors_simple_stamps() {
    let mut b = simple();
    b.sense[SenseTarget::Hue as usize][SenseSource::Random as usize] =
        BrushParams::sense_from_permille(1000);
    let px = layer(&b, Argb::new(255, 0xc0, 0x40, 0x40));
    let cols: std::collections::HashSet<[u8; 3]> = px
        .iter()
        .filter(|p| p[3] == 255)
        .map(|p| [p[0], p[1], p[2]])
        .collect();
    assert!(
        cols.len() > 3,
        "Farbtonjitter: verschiedene Farben je Stempel, war {cols:?}"
    );
}

#[test]
fn blackness_darkens_simple_stroke() {
    let base = layer(&simple(), Argb::new(255, 0x40, 0x80, 0xff));
    let mut b = simple();
    b.blackness = 1000;
    let dark = layer(&b, Argb::new(255, 0x40, 0x80, 0xff));
    let blue = |px: &[[u8; 4]]| {
        px.iter()
            .filter(|p| p[3] > 0)
            .map(|p| p[2] as u64)
            .sum::<u64>()
            / px.iter().filter(|p| p[3] > 0).count().max(1) as u64
    };
    assert!(
        blue(&dark) < blue(&base),
        "Schwärze dunkelt Blau ab: {} gegen {}",
        blue(&dark),
        blue(&base)
    );
}

/// Ohne diese Empfindlichkeiten bleibt der Strich wie bisher (kein RGB-Puffer, gleiche Pixel bei
/// gleicher Stufe).
#[test]
fn without_extras_unchanged_and_deterministic() {
    let a = layer(&simple(), Argb::new(255, 0x10, 0x20, 0x30));
    let b = layer(&simple(), Argb::new(255, 0x10, 0x20, 0x30));
    assert_eq!(a, b);
    assert!(
        a.iter()
            .filter(|p| p[3] > 0)
            .all(|p| [p[0], p[1], p[2]] == [0x10, 0x20, 0x30])
    );
}

/// M10 (re 43 §6a): Beim einfachen Läufer wirkt die Richtung nicht – „Größe ← waagerechte bzw.
/// senkrechte Richtung 100 %“ lässt den Strich wie ohne Empfindlichkeit ((h, v) = 0 ist richtig).
#[test]
fn t11_direction_has_no_effect_on_simple_runner() {
    let plain = layer(&simple(), Argb::new(255, 0, 0, 0));
    for src in [SenseSource::Horizontal, SenseSource::Vertical] {
        let mut b = simple();
        b.sense[SenseTarget::Size as usize][src as usize] = BrushParams::sense_from_permille(1000);
        assert_eq!(layer(&b, Argb::new(255, 0, 0, 0)), plain, "{src:?}");
    }
}
