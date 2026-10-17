//! Wirkung der Zeigergeschwindigkeit auf die Pinselgröße (re\34 §2.2): `spd = round(VELf·256)`
//! geht unverändert in die Läuferformeln ein (`min(spd, 256)`). Die frühere Abbildung aus der
//! Messung M6 (`1,17 − 1,5·v`) ist zurückgenommen: die Code-Regel geht vor der ungeklärten Messung
//! (re\34 §2.4, offene Frage A in `docs\berichte\t7.md`).

use fp_vector::natural::{sensitivity_speed, speed_size_factor};

/// Spitzenfaktor bei 100 % „Größe ← Geschwindigkeit“ = 1 − min(round(v·256), 256)/256.
#[test]
fn speed_size_factor_follows_code_rule() {
    for (v, want) in [
        (0.0f32, 1.0),
        (0.125, 0.875),
        (0.18, 1.0 - 46.0 / 256.0),
        (0.39, 1.0 - 100.0 / 256.0),
        (0.58, 1.0 - 148.0 / 256.0),
        (1.0, 0.0),
        (2.39, 0.0),
    ] {
        let f = speed_size_factor(v);
        assert!((f - want).abs() <= 1e-9, "v {v}: {f} statt {want}");
    }
    assert_eq!(sensitivity_speed(0.0), 0.0);
    assert_eq!(sensitivity_speed(-1.0), 0.0);
    assert_eq!(sensitivity_speed(0.125), 32.0, "Vorgabe ohne Dynamik");
    // False positive: 2.28515625 = 2 + 73/256 is exact in f32; the literal documents that.
    #[allow(clippy::excessive_precision)]
    let v_exact: f32 = 2.28515625;
    assert_eq!(
        sensitivity_speed(v_exact),
        585.0,
        "nach oben offen, die Formeln begrenzen auf 256"
    );
    assert_eq!(sensitivity_speed(300.0), 255.0, "≥ 255,996 ⇒ 0xFF");
    assert_eq!(fp_vector::PointDynamics::default().velocity, 0.125);
}

/// Messpunkte M6 §2b (echte Mauszüge, Breite 20 px, 100 %): 0,18 → 0,90, 0,39 → 0,4–0,6,
/// 0,58 → 0,30, ≥ 1,9 → kein Strich. Bei mittleren Geschwindigkeiten dünner als die Code-Regel;
/// Ursache offen (Zeittakt der Eingabe, re\34 §2.4). Nicht umgesetzt, nur dokumentiert.
#[test]
#[ignore = "Messung M6 widerspricht der Code-Regel bei mittleren Geschwindigkeiten (offene Frage)"]
fn speed_size_factor_matches_measurement() {
    for (v, want, tol) in [
        (0.18f32, 0.90, 0.01),
        (0.39, 0.5, 0.1),
        (0.58, 0.30, 0.01),
        (1.9, 0.0, 0.0),
    ] {
        let f = speed_size_factor(v);
        assert!((f - want).abs() <= tol + 1e-9, "v {v}: {f} statt {want}");
    }
}
