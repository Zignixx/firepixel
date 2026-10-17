//! K-6 / T-9: Größenstufe der Pinselspitze (re\37 §1 Messung M7, re\39 §2 Regel). Pinsel
//! „1-Pixel Hard“ (simple-Pinsel ohne Kantenglättung), D 20, M 14, „Größe ← Geschwindigkeit“
//! 100 % bzw. 50 %: `spd = round(VELf·256)`, Stufe = (Zeile·M >> 7)·min(spd, 256) >> 8.
//!
//! Die Stufe verkleinert nicht den Durchmesser, sondern wählt die Abtastmatrix der Spitze
//! `s = m + (1 − m)·((M − k)/M)^1,5` mit Mittel der vier Pixelecken. Der simple-Läufer ruft
//! dieselbe Stufen- und Maskenfunktion wie der natürliche. Gezählt wird wie M7: Alpha > 80.

use fp_color::Argb;
use fp_geom::{IRect, Vec2};
use fp_vector::stroke::stroke_layer;
use fp_vector::{
    BrushParams, BrushType, PathContour, PointDynamics, SenseSource, SenseTarget, Stroke,
    VectorPath,
};

fn pencil(sense_pct: u32) -> BrushParams {
    let mut b = BrushParams {
        diameter: 20,
        max_count: 14,
        min_size: 10,
        kind: BrushType::Simple,
        antialiased: false,
        spacing: 150,
        ..Default::default()
    };
    b.sense[SenseTarget::Size as usize][SenseSource::Speed as usize] =
        BrushParams::sense_from_permille(sense_pct * 10);
    b
}

/// Senkrechte Breite des Strichs (größte Spaltenzahl mit Alpha > 80) einer waagrechten
/// 300-px-Linie mit konstantem `VELf`.
fn width(velf: f32, sense_pct: u32) -> usize {
    let dynamics = PointDynamics {
        velocity: velf,
        ..Default::default()
    };
    let mut c = PathContour::new(vec![Vec2::new(60.0, 100.0), Vec2::new(360.0, 100.0)], false);
    c.dynamics = vec![dynamics; 2];
    let path = VectorPath::new(vec![c]);
    let stroke = Stroke::new(pencil(sense_pct), Argb::new(255, 0xc0, 0x40, 0x40));
    let rect = IRect::new(0, 0, 420, 200);
    let px = stroke_layer(&path, &stroke, rect);
    (100..320)
        .map(|x| (0..200).filter(|&y| px[y * 420 + x][3] > 80).count())
        .max()
        .unwrap()
}

/// Volle Tabelle D 20, M 14 (re 39 §2.3): Stufe k 0…14 → 20/19/17/16/14/12/10/8/6/6/4/4/2/2/2.
#[test]
fn stage_widths_full_table_d20_m14() {
    let want = [20usize, 19, 17, 16, 14, 12, 10, 8, 6, 6, 4, 4, 2, 2, 2];
    let got: Vec<usize> = (0..=14u32)
        .map(|k| {
            // kleinste Geschwindigkeit mit (14·spd) >> 8 = k
            let spd = (k * 256).div_ceil(14);
            width(spd as f32 / 256.0, 100)
        })
        .collect();
    assert_eq!(got, want);
}

#[test]
fn speed_widths_like_original_100_percent() {
    let got: Vec<(f32, usize)> = [0.125f32, 0.25, 0.39, 0.5, 0.58, 0.75, 1.0]
        .iter()
        .map(|&v| (v, width(v, 100)))
        .collect();
    let want = [19usize, 16, 12, 8, 6, 4, 2];
    for ((v, w), e) in got.iter().zip(want) {
        assert_eq!(*w, e, "VELf {v}: Breite {w} statt {e} (alle: {got:?})");
    }
    assert_eq!(width(0.0, 100), 20, "VELf 0 = volle Breite");
    assert_eq!(width(2.3, 100), 2, "nie unter 2 px");
}

#[test]
fn speed_widths_like_original_50_percent() {
    for (v, e) in [(0.25f32, 19usize), (0.5, 16), (0.75, 12), (1.0, 8)] {
        assert_eq!(width(v, 50), e, "VELf {v} bei 50 %");
    }
    assert_eq!(width(0.5, 0), 20, "0 %: immer volle Breite");
}

/// Ohne Empfindlichkeit bleibt der simple-Läufer bei der ungestuften Spitze (Druck egal).
#[test]
fn simple_runner_without_sensitivity_ignores_dynamics() {
    let mut b = pencil(0);
    b.sense = Default::default();
    let mk = |p: f32| {
        let mut c = PathContour::new(vec![Vec2::new(60.0, 100.0), Vec2::new(360.0, 100.0)], false);
        c.dynamics = vec![
            PointDynamics {
                pressure: p,
                velocity: 1.0,
                ..Default::default()
            };
            2
        ];
        stroke_layer(
            &VectorPath::new(vec![c]),
            &Stroke::new(b.clone(), Argb::new(255, 0, 0, 0)),
            IRect::new(0, 0, 420, 200),
        )
    };
    assert_eq!(mk(0.1), mk(1.0));
}
