//! Rechenteil des Tonwertdialogs (re\19-kurven-tonwert.md §3): Gamma ↔ Mittelgriff über die zwei
//! Binärsuchen des Originals, Histogramm (Kanäle, RGB-Summe, Maßstab), Auto, Pipetten und die
//! Regeln der Schieber. Die Tonwerttabelle selbst steht in [`super::table`].

pub use super::curve_edit::Pipette;
use super::table::{LevelsChannel, LevelsParams, histogram_bounds};
use crate::image::RgbaImage;

/// Wertebereich der Schieber und Felder.
pub const LEVEL_MAX: f64 = 255.0;

/// Mindestabstand benachbarter Griffe des Eingabeschiebers (`255 / 30 = 8,5`, §3.5).
pub const INPUT_HANDLE_GAP: f64 = 255.0 / 30.0;

/// Ganzzahl wie das Original beim Übernehmen eines Schieberwerts ins Feld: kaufmännisch gerundet
/// (`floor(v + 0,5)`, §3.5; Griff auf 62,95 → „63“).
pub fn round_half_up(v: f64) -> i32 {
    (v + 0.5).floor() as i32
}

/// `0,5^(1/g) · 255`, begrenzt auf 255 und abgeschnitten: der Vergleichswert beider Suchen.
fn probe(g: f64) -> i64 {
    (0.5f64.powf(1.0 / g) * 255.0).min(255.0).trunc() as i64
}

/// Gamma aus dem Anteil `f` (0…1) wie das Original: Binärsuche mit Start 5, Schritt 2,5, höchstens
/// 15 Runden, Ziel `trunc(f · 255)` (§3.6). Ergibt etwa `ln 0,5 / ln f`, quantisiert; der Anteil
/// 0,5 liefert 0,99609375, nicht genau 1.
pub fn gamma_from_fraction(f: f64) -> f64 {
    let target = (f * 255.0).trunc() as i64;
    let mut g = 5.0f64;
    let mut step = 2.5f64;
    for _ in 0..15 {
        let v = probe(g);
        if v == target {
            return g;
        }
        if v < target {
            g += step;
        } else {
            g -= step;
        }
        step /= 2.0;
    }
    g
}

/// Anteil aus Gamma (Umkehrung von [`gamma_from_fraction`]): Start 0,5, Schritt 0,25, höchstens 15
/// Runden, fertig bei `|g(f) − gamma| < 0,03` (§3.6).
pub fn fraction_from_gamma(gamma: f64) -> f64 {
    let mut f = 0.5f64;
    let mut step = 0.25f64;
    for _ in 0..15 {
        let g = gamma_from_fraction(f);
        if (g - gamma).abs() < 0.03 {
            return f;
        }
        if g <= gamma {
            f += step;
        } else {
            f -= step;
        }
        step /= 2.0;
    }
    f
}

/// Gamma aus der Lage des mittleren Griffs. Der Anteil wird **von rechts** gemessen:
/// `f = (weiß − pos) / (weiß − schwarz)` (§3.6).
pub fn gamma_from_handle(pos: f64, black: f64, white: f64) -> f64 {
    let span = white - black;
    if span <= 0.0 {
        return 1.0;
    }
    gamma_from_fraction(((white - pos) / span).clamp(0.0, 1.0))
}

/// Lage des mittleren Griffs aus Gamma: `pos = weiß − (weiß − schwarz) · f(gamma)` (§3.6).
pub fn handle_from_gamma(gamma: f64, black: f64, white: f64) -> f64 {
    white - (white - black) * fraction_from_gamma(gamma)
}

/// Die drei Griffe des Eingabeschiebers (schwarz, grau, weiß) eines Kanals.
pub fn input_handles(c: &LevelsChannel) -> [f64; 3] {
    let (b, w) = (c.source_low as f64, c.source_high as f64);
    [b, handle_from_gamma(c.gamma, b, w), w]
}

/// Neuer Wert eines gezogenen Griffs des Eingabeschiebers: 0…255 und mindestens
/// [`INPUT_HANDLE_GAP`] Abstand zu beiden Nachbarn in ihrer **aktuellen** Lage (die Griffe bleiben
/// geordnet). Weil der Mittelgriff danach aus dem Gamma neu gesetzt wird, wandert er beim Ziehen von
/// Schwarz oder Weiß mit (§3.5).
pub fn drag_input_handle(handles: [f64; 3], idx: usize, v: f64) -> f64 {
    let lo = if idx == 0 {
        0.0
    } else {
        handles[idx - 1] + INPUT_HANDLE_GAP
    };
    let hi = if idx == 2 {
        LEVEL_MAX
    } else {
        handles[idx + 1] - INPUT_HANDLE_GAP
    };
    let v = v.clamp(0.0, LEVEL_MAX);
    if lo > hi {
        handles[idx]
    } else {
        v.clamp(lo, hi)
    }
}

/// Neuer Wert eines gezogenen Griffs des Ausgabeschiebers: nur 0…255, die Griffe dürfen sich
/// kreuzen (unten > oben kehrt um, §3.5).
pub fn drag_output_handle(v: f64) -> f64 {
    v.clamp(0.0, LEVEL_MAX)
}

/// Wendet einen gezogenen Eingabegriff auf den Kanal an: Schwarz/Weiß als gerundete Ganzzahl,
/// danach gilt das Gamma unverändert (der Mittelgriff folgt); der Mittelgriff setzt Gamma (§3.5/§3.6).
pub fn apply_input_drag(c: &mut LevelsChannel, idx: usize, v: f64) {
    let h = input_handles(c);
    let v = drag_input_handle(h, idx, v);
    match idx {
        0 => c.source_low = round_half_up(v).clamp(0, c.source_high - 2),
        2 => c.source_high = round_half_up(v).clamp(c.source_low + 2, 255),
        _ => {
            c.gamma =
                gamma_from_handle(v, c.source_low as f64, c.source_high as f64).clamp(0.1, 10.0)
        }
    }
}

/// Wendet einen gezogenen Ausgabegriff an (0 = unten, 1 = oben).
pub fn apply_output_drag(c: &mut LevelsChannel, idx: usize, v: f64) {
    let v = round_half_up(drag_output_handle(v)).clamp(0, 255);
    if idx == 0 {
        c.dest_low = v;
    } else {
        c.dest_high = v;
    }
}

// ------------------------------------------------------------------ Histogramm

/// Zählt R, G, B aller Pixel mit Alpha > 0 zu `h` hinzu (§3.3).
pub fn add_channel_histograms(h: &mut [[u64; 256]; 3], img: &RgbaImage) {
    for p in img.data() {
        if p[3] == 0 {
            continue;
        }
        for c in 0..3 {
            h[c][p[c] as usize] += 1;
        }
    }
}

/// Kanalhistogramme R, G, B eines Bilds (nur Pixel mit Alpha > 0).
pub fn channel_histograms(img: &RgbaImage) -> [[u64; 256]; 3] {
    let mut h = [[0u64; 256]; 3];
    add_channel_histograms(&mut h, img);
    h
}

/// Histogramm, das der Dialog für `channel` zeigt (0 = RGB, 1…3 = R, G, B). RGB ist die **Summe**
/// der drei Kanalhistogramme, jeder Wert vorher durch die Tonwerttabelle seines Kanals geführt
/// (`Summe[T_rot[v]] += H_rot[v]` …); die RGB-Einstellung selbst geht nicht ein (§3.3).
pub fn display_histogram(h: &[[u64; 256]; 3], p: &LevelsParams, channel: usize) -> [u64; 256] {
    match channel {
        1..=3 => h[channel - 1],
        _ => {
            let mut sum = [0u64; 256];
            for (k, ch) in [&p.red, &p.green, &p.blue].into_iter().enumerate() {
                let t = ch.table();
                for v in 0..256 {
                    sum[t[v] as usize] += h[k][v];
                }
            }
            sum
        }
    }
}

/// Maßstab: Summe der fünf größten Zähler, ganzzahlig durch 5 (§3.3).
pub fn histogram_scale(h: &[u64; 256]) -> u64 {
    let mut v = h.to_vec();
    v.sort_unstable_by(|a, b| b.cmp(a));
    v.iter().take(5).sum::<u64>() / 5
}

/// Säulenhöhen je Pixelspalte (`w` Spalten, Höhe `hc`): `v = round(x/(w−1)·255)`,
/// `h = min(round(H[v]/m · hc), hc)`. Sichtbar ist wie im Original `h − 1` (GDI-Linie ohne
/// Endpunkt, §3.3).
pub fn histogram_heights(h: &[u64; 256], w: usize, hc: u32) -> Vec<u32> {
    let m = histogram_scale(h);
    (0..w)
        .map(|x| {
            if m == 0 || w < 2 {
                return 0;
            }
            let v = ((x as f64 / (w - 1) as f64) * 255.0).round() as usize;
            ((h[v.min(255)] as f64 / m as f64 * hc as f64).round() as u64).min(hc as u64) as u32
        })
        .collect()
}

// ------------------------------------------------------------------ Auto und Pipetten

/// „Auto“ im Dialog (§3.8, dieselbe Rechnung wie Auto-Tonwert): Rot, Grün, Blau bekommen die
/// 0,5-%-Grenzen ihres Histogramms, Gamma 1, Ausgabe 0/255. Der RGB-Satz bleibt unverändert.
pub fn auto_levels_from(p: &mut LevelsParams, h: &[[u64; 256]; 3]) {
    for (k, ch) in [&mut p.red, &mut p.green, &mut p.blue]
        .into_iter()
        .enumerate()
    {
        let (lo, hi) = histogram_bounds(&h[k]);
        *ch = LevelsChannel {
            source_low: lo,
            source_high: hi,
            dest_low: 0,
            dest_high: 255,
            gamma: 1.0,
        };
    }
}

/// Wirkung einer Pipette mit der aufgenommenen (angezeigten) Farbe `c` (§3.9). Es ändern sich nur
/// die Kanalsätze Rot, Grün, Blau; der RGB-Satz bleibt.
pub fn apply_pick(p: &mut LevelsParams, pick: Pipette, c: [u8; 3]) {
    for (k, ch) in [&mut p.red, &mut p.green, &mut p.blue]
        .into_iter()
        .enumerate()
    {
        let ck = c[k] as i32;
        let (lo, hi) = (ch.source_low, ch.source_high);
        match pick {
            Pipette::Shadow => ch.source_low = if ck == 0 { 0 } else { ck.min(hi - 2) },
            Pipette::Highlight => ch.source_high = ck.max(lo + 2).min(255),
            Pipette::Midtone => {
                // Eigenheit des Originals, bewusst nachgebaut: Hier wird der Anteil von LINKS
                // gemessen, `(cc − lo) / (hi − lo)`, beim Ziehen des Mittelgriffs dagegen von
                // rechts. Ein dunkles Grau wird dadurch weiter abgedunkelt statt auf Mittelgrau
                // gezogen: Grau 64 → Gamma 0,5029 (Anzeige „0.5“) → 64 wird 16, 192 wird 145
                // (re\19-kurven-tonwert.md §3.9).
                let cc = ck.clamp(lo + 1, hi - 1);
                let span = (hi - lo) as f64;
                if span > 0.0 {
                    ch.gamma = gamma_from_fraction((cc - lo) as f64 / span);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gamma_search_examples() {
        assert_eq!(gamma_from_fraction(0.5), 0.99609375, "Mitte");
        // Griff 99,23 bei Schwarz 63, Weiß 200 → 2,265625 (Anzeige „2.27“)
        assert_eq!(gamma_from_handle(99.23, 63.0, 200.0), 2.265625);
        assert_eq!(fraction_from_gamma(1.0), 0.5);
        let f2 = fraction_from_gamma(2.0);
        assert!((f2 - 0.703).abs() < 0.01, "Gamma 2 bei f ≈ 0,703: {f2}");
        // Gamma 2 liegt links der Mitte
        assert!(handle_from_gamma(2.0, 0.0, 255.0) < 127.5);
    }

    #[test]
    fn midtone_pick_quirk() {
        let mut p = LevelsParams::default();
        apply_pick(&mut p, Pipette::Midtone, [64, 64, 64]);
        assert_eq!(p.red.gamma, 0.5029296875);
        let t = p.red.table();
        assert_eq!((t[64], t[192]), (16, 145));
        assert_eq!(p.rgb, LevelsChannel::default());
    }
}
