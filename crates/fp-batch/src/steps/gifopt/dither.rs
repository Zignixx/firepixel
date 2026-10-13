//! Dithering (SPEC-BATCH §5.6 Punkt 3), eigene Umsetzung:
//!
//! - `none`: nächste Palettenfarbe.
//! - `ordered`: 8×8-Bayer-Schwellen, Zwei-Farben-Mischung (siehe unten).
//! - `blueNoise`: dieselbe Mischung mit einer 64×64-Blue-Noise-Schwellwertmatrix (Void-and-Cluster
//!   nach Ulichney 1993, eigen erzeugt, als Datei `bluenoise64.bin` im Crate).
//! - `temporal`: Blue-Noise-Matrix, die zwischen den Frames **fest** bleibt (kein Flimmern, gute
//!   LZW-Läufe), Stärke adaptiv: in glatten Flächen voll, an Kanten schwächer, und mit Totzone (nur
//!   wo der Quantisierungsfehler sichtbar wäre, BATCH-8b).
//! - `floydSteinberg`: Fehlerverteilung mit Serpentine in Oklab, nur in geänderten Bereichen.
//!
//! In allen Arten übernehmen statische Pixel (gegenüber dem Vorframe unverändert) die Indizes des
//! Vorframes, wenn er dieselbe Palette hat → keine „kriechenden“ Muster.
//!
//! Zwei-Farben-Mischung (geordnetes Dithering mit Palette): `p1` = nächste Farbe zu `c`, `p2` = nächste
//! Farbe zu `c + (c − p1)` (die Farbe „auf der anderen Seite“); Anteil `r` = Projektion von `c − p1`
//! auf `p2 − p1`. Pixel nimmt `p2`, wenn die Schwelle der Matrix `< r · Stärke` ist – so stimmt die
//! mittlere Farbe einer Fläche, und das Muster hängt nur von Farbe und Ort ab.

use super::color::{lab_to_rgb, pack, rgb_to_lab, unpack, ColorMap, Lab};
use super::palette::Palette;
use std::sync::OnceLock;

/// Kennung „transparent“ im Indexbild.
pub const TRANSPARENT: u16 = u16::MAX;
/// Kennzeichen „deckend“ im gepackten Pixel (`0x01RRGGBB`); 0 = transparent.
pub const OPAQUE: u32 = 1 << 24;

/// Dithering-Art.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DitherMode {
    None,
    Ordered,
    BlueNoise,
    FloydSteinberg,
    Temporal,
}

impl DitherMode {
    pub fn parse(s: &str) -> Option<DitherMode> {
        Some(match s {
            "none" => DitherMode::None,
            "ordered" => DitherMode::Ordered,
            "blueNoise" => DitherMode::BlueNoise,
            "floydSteinberg" | "diffusion" => DitherMode::FloydSteinberg,
            "temporal" => DitherMode::Temporal,
            _ => return None,
        })
    }
}

/// Seitenlänge der Blue-Noise-Matrix.
pub const BN: usize = 64;

/// Blue-Noise-Matrix (Rang 0…4095 je Zelle), aus `bluenoise64.bin` (u16, little endian).
pub fn blue_noise() -> &'static [u16] {
    static M: OnceLock<Vec<u16>> = OnceLock::new();
    M.get_or_init(|| {
        let raw: &[u8] = include_bytes!("bluenoise64.bin");
        if raw.len() == BN * BN * 2 {
            raw.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c)).collect()
        } else {
            generate_blue_noise()
        }
    })
}

/// Erzeugt die Blue-Noise-Matrix nach dem Void-and-Cluster-Verfahren (Ulichney 1993): toroidaler
/// Gauß-Filter (σ = 1,5), Startmuster aus einem festen Zufallsgenerator, Phase 0 (Muster
/// entspannen), Phase 1 (Ränge absteigend), Phase 2/3 (Ränge aufsteigend). Deterministisch.
pub fn generate_blue_noise() -> Vec<u16> {
    let n = BN * BN;
    let sigma2 = 2.0 * 1.5f32 * 1.5;
    // Kern über toroidale Abstände
    let mut kern = vec![0f32; n];
    for dy in 0..BN {
        for dx in 0..BN {
            let x = dx.min(BN - dx) as f32;
            let y = dy.min(BN - dy) as f32;
            kern[dy * BN + dx] = (-(x * x + y * y) / sigma2).exp();
        }
    }
    let update = |energy: &mut [f32], p: usize, sign: f32| {
        let (px, py) = (p % BN, p / BN);
        for y in 0..BN {
            let ky = (y + BN - py) % BN;
            for x in 0..BN {
                let kx = (x + BN - px) % BN;
                energy[y * BN + x] += sign * kern[ky * BN + kx];
            }
        }
    };
    // Startmuster: ≈ 10 % Einsen, fester LCG
    let mut seed: u64 = 0x2545_F491_4F6C_DD1D;
    let mut bits = vec![false; n];
    let mut ones = 0usize;
    while ones < n / 10 {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        let p = (seed >> 33) as usize % n;
        if !bits[p] {
            bits[p] = true;
            ones += 1;
        }
    }
    let mut energy = vec![0f32; n];
    for (p, b) in bits.iter().enumerate() {
        if *b {
            update(&mut energy, p, 1.0);
        }
    }
    let tightest = |bits: &[bool], energy: &[f32]| -> usize {
        let mut best = (f32::MIN, 0);
        for (p, b) in bits.iter().enumerate() {
            if *b && energy[p] > best.0 {
                best = (energy[p], p);
            }
        }
        best.1
    };
    let largest_void = |bits: &[bool], energy: &[f32]| -> usize {
        let mut best = (f32::MAX, 0);
        for (p, b) in bits.iter().enumerate() {
            if !*b && energy[p] < best.0 {
                best = (energy[p], p);
            }
        }
        best.1
    };
    // Phase 0: dichteste Häufung in die größte Lücke verschieben, bis es sich nicht mehr ändert
    for _ in 0..n {
        let c = tightest(&bits, &energy);
        bits[c] = false;
        update(&mut energy, c, -1.0);
        let v = largest_void(&bits, &energy);
        bits[v] = true;
        update(&mut energy, v, 1.0);
        if v == c {
            break;
        }
    }
    let proto = bits.clone();
    let proto_energy = energy.clone();
    let mut rank = vec![0u16; n];
    // Phase 1: Ränge der Einsen absteigend
    let mut r = ones;
    while r > 0 {
        let c = tightest(&bits, &energy);
        bits[c] = false;
        update(&mut energy, c, -1.0);
        r -= 1;
        rank[c] = r as u16;
    }
    // Phase 2/3: Ränge der übrigen Zellen aufsteigend (größte Lücke zuerst)
    let mut bits = proto;
    let mut energy = proto_energy;
    for r in ones..n {
        let v = largest_void(&bits, &energy);
        bits[v] = true;
        update(&mut energy, v, 1.0);
        rank[v] = r as u16;
    }
    rank
}

const BAYER8: [[u8; 8]; 8] = [
    [0, 32, 8, 40, 2, 34, 10, 42],
    [48, 16, 56, 24, 50, 18, 58, 26],
    [12, 44, 4, 36, 14, 46, 6, 38],
    [60, 28, 52, 20, 62, 30, 54, 22],
    [3, 35, 11, 43, 1, 33, 9, 41],
    [51, 19, 59, 27, 49, 17, 57, 25],
    [15, 47, 7, 39, 13, 45, 5, 37],
    [63, 31, 55, 23, 61, 29, 53, 21],
];

/// Eingabe eines Frames für das Indizieren.
pub struct FrameInput<'a> {
    pub w: usize,
    pub h: usize,
    /// Gepackte Pixel (`OPAQUE | rgb` bzw. 0 = transparent).
    pub px: &'a [u32],
    /// Oklab der Pixel.
    pub lab: &'a [Lab],
    /// Pixel gegenüber dem Vorframe unverändert.
    pub static_mask: &'a [bool],
    /// Indizes des Vorframes, wenn er dieselbe Palette hatte.
    pub prev_idx: Option<&'a [u16]>,
    /// Kantenstärke je Pixel (Oklab-Gradient), für die adaptive Stärke.
    pub edge: &'a [f32],
}

/// Indiziert ein Frame (Ergebnis: Palettenindex bzw. [`TRANSPARENT`]).
pub fn index_frame(inp: &FrameInput, pal: &mut Palette, mode: DitherMode, amount: f32) -> Vec<u16> {
    let n = inp.w * inp.h;
    let mut out = vec![TRANSPARENT; n];
    match mode {
        DitherMode::FloydSteinberg if amount > 0.0 => floyd_steinberg(inp, pal, amount, &mut out),
        DitherMode::None => {
            for (i, o) in out.iter_mut().enumerate() {
                *o = pick_static(inp, i).unwrap_or_else(|| if inp.px[i] & OPAQUE == 0 { TRANSPARENT } else { pal.nearest(unpack(inp.px[i])) as u16 });
            }
        }
        _ => {
            let mut mix: ColorMap<(u8, u8, f32)> = ColorMap::default();
            let bn = blue_noise();
            for y in 0..inp.h {
                for x in 0..inp.w {
                    let i = y * inp.w + x;
                    if let Some(v) = pick_static(inp, i) {
                        out[i] = v;
                        continue;
                    }
                    let p = inp.px[i];
                    if p & OPAQUE == 0 {
                        continue;
                    }
                    let (p1, p2, ratio) = *mix.entry(p).or_insert_with(|| two_color_mix(pal, unpack(p), &inp.lab[i]));
                    if p1 == p2 || amount <= 0.0 {
                        out[i] = p1 as u16;
                        continue;
                    }
                    let (t, strength) = match mode {
                        DitherMode::Ordered => ((BAYER8[y % 8][x % 8] as f32 + 0.5) / 64.0, amount),
                        DitherMode::Temporal => {
                            let e = (inp.edge[i] * 15.0).clamp(0.0, 0.85);
                            ((bn[(y % BN) * BN + x % BN] as f32 + 0.5) / 4096.0, amount * (1.0 - e))
                        }
                        _ => ((bn[(y % BN) * BN + x % BN] as f32 + 0.5) / 4096.0, amount),
                    };
                    let strength = if mode == DitherMode::Temporal { strength * deadzone(&pal.lab[p1 as usize], &inp.lab[i]) } else { strength };
                    out[i] = if t < ratio * strength { p2 as u16 } else { p1 as u16 };
                }
            }
        }
    }
    out
}

/// Totzone von `temporal` (BATCH-8b), Oklab-Abstand zur nächsten Palettenfarbe: darunter wird nicht
/// gemischt (der Fehler liegt unter der Sichtbarkeit, die Fläche bleibt ruhig und packt gut), darüber
/// steigt die Stärke weich bis [`DEADZONE_FULL`] auf 100 % (sichtbare Stufen werden weiter aufgelöst).
pub const DEADZONE_START: f32 = 0.01;
/// Ende der weichen Totzone (volle Stärke).
pub const DEADZONE_FULL: f32 = 0.03;

fn deadzone(p: &Lab, c: &Lab) -> f32 {
    let e = super::color::dist(p, c);
    let t = ((e - DEADZONE_START) / (DEADZONE_FULL - DEADZONE_START)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn pick_static(inp: &FrameInput, i: usize) -> Option<u16> {
    if inp.static_mask[i] { inp.prev_idx.map(|p| p[i]) } else { None }
}

/// Zwei-Farben-Mischung für eine Farbe: (`p1`, `p2`, Anteil von `p2`).
fn two_color_mix(pal: &mut Palette, rgb: [u8; 3], lab: &Lab) -> (u8, u8, f32) {
    let p1 = pal.nearest(rgb);
    let l1 = pal.lab[p1 as usize];
    let e = [lab[0] - l1[0], lab[1] - l1[1], lab[2] - l1[2]];
    let e2 = e[0] * e[0] + e[1] * e[1] + e[2] * e[2];
    if e2 < 1e-8 {
        return (p1, p1, 0.0);
    }
    let mut p2 = p1;
    for f in [1.0f32, 2.0, 4.0] {
        let c2 = [lab[0] + e[0] * f, lab[1] + e[1] * f, lab[2] + e[2] * f];
        p2 = pal.nearest_lab(&c2);
        if p2 != p1 {
            break;
        }
    }
    if p2 == p1 {
        return (p1, p1, 0.0);
    }
    let l2 = pal.lab[p2 as usize];
    let d = [l2[0] - l1[0], l2[1] - l1[1], l2[2] - l1[2]];
    let dd = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
    if dd < 1e-10 {
        return (p1, p1, 0.0);
    }
    let r = ((e[0] * d[0] + e[1] * d[1] + e[2] * d[2]) / dd).clamp(0.0, 1.0);
    (p1, p2, r)
}

/// Floyd–Steinberg mit Serpentine in Oklab. Fehler werden nur zwischen geänderten Pixeln verteilt;
/// statische Pixel übernehmen die Vorframe-Indizes, transparente setzen den Fehler zurück.
fn floyd_steinberg(inp: &FrameInput, pal: &mut Palette, amount: f32, out: &mut [u16]) {
    let (w, h) = (inp.w, inp.h);
    let mut cur = vec![[0f32; 3]; w + 2];
    let mut next = vec![[0f32; 3]; w + 2];
    let lim = 0.12f32;
    for y in 0..h {
        let rtl = y % 2 == 1;
        for k in 0..w {
            let x = if rtl { w - 1 - k } else { k };
            let i = y * w + x;
            let e = cur[x + 1];
            if let Some(v) = pick_static(inp, i) {
                out[i] = v;
                continue;
            }
            let p = inp.px[i];
            if p & OPAQUE == 0 {
                continue;
            }
            let c = inp.lab[i];
            let c2 = [c[0] + e[0].clamp(-lim, lim) * amount, c[1] + e[1].clamp(-lim, lim) * amount, c[2] + e[2].clamp(-lim, lim) * amount];
            let idx = if e == [0.0; 3] { pal.nearest(unpack(p)) } else { pal.nearest(lab_to_rgb(c2)) };
            out[i] = idx as u16;
            let q = pal.lab[idx as usize];
            let err = [c2[0] - q[0], c2[1] - q[1], c2[2] - q[2]];
            let (fwd, back) = if rtl { (x, x + 2) } else { (x + 2, x) };
            for k in 0..3 {
                cur[fwd][k] += err[k] * 7.0 / 16.0;
                next[back][k] += err[k] * 3.0 / 16.0;
                next[x + 1][k] += err[k] * 5.0 / 16.0;
                next[fwd][k] += err[k] / 16.0;
            }
        }
        std::mem::swap(&mut cur, &mut next);
        next.iter_mut().for_each(|v| *v = [0.0; 3]);
    }
}

/// Packt ein RGBA-Pixel (Alpha < 128 → transparent).
#[inline]
pub fn pack_px(p: [u8; 4]) -> u32 {
    if p[3] < 128 { 0 } else { OPAQUE | pack([p[0], p[1], p[2]]) }
}

/// Oklab eines gepackten Pixels (transparent → Schwarz).
#[inline]
pub fn lab_of(p: u32) -> Lab {
    rgb_to_lab(unpack(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blue_noise_file_matches_generator_and_is_a_permutation() {
        let matrix = generate_blue_noise();
        if std::env::var_os("FP_BLESS").is_some() {
            let bytes: Vec<u8> = matrix.iter().flat_map(|v| v.to_le_bytes()).collect();
            let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/steps/gifopt/bluenoise64.bin");
            std::fs::write(p, bytes).expect("write");
        }
        let mut seen = vec![false; BN * BN];
        for v in &matrix {
            assert!(!seen[*v as usize]);
            seen[*v as usize] = true;
        }
        assert_eq!(blue_noise(), &matrix[..], "bluenoise64.bin veraltet (FP_BLESS=1 erneuert)");
        // Blue Noise: Schwelle 50 % → Nachbarn selten gleich (kaum Klumpen)
        let on = |x: usize, y: usize| matrix[(y % BN) * BN + x % BN] < 2048;
        let mut same = 0;
        for y in 0..BN {
            for x in 0..BN {
                if on(x, y) == on(x + 1, y) {
                    same += 1;
                }
            }
        }
        assert!(same < BN * BN * 45 / 100, "zu klumpig: {same}");
    }

    /// Totzone (BATCH-8b): Liegt die Quelle fast auf einer Palettenfarbe, bleibt die Fläche ruhig
    /// (jede Zeile eines senkrechten Verlaufs einfarbig); bei groben Stufen wird weiter gemischt.
    #[test]
    fn temporal_deadzone_keeps_fine_gradients_calm() {
        let (w, h) = (64usize, 64usize);
        let px: Vec<u32> = (0..w * h).map(|i| OPAQUE | pack([90, 140, (150 + i / w) as u8])).collect();
        let lab: Vec<Lab> = px.iter().map(|p| lab_of(*p)).collect();
        let st = vec![false; w * h];
        let edge = vec![0.0; w * h];
        let inp = FrameInput { w, h, px: &px, lab: &lab, static_mask: &st, prev_idx: None, edge: &edge };
        let rows_uniform = |idx: &[u16]| (0..h).filter(|y| idx[y * w..(y + 1) * w].iter().all(|v| *v == idx[y * w])).count();
        // fein: jede zweite Quellfarbe in der Palette (Fehler ≤ 1 Stufe) → keine Mischung
        let mut fine = Palette::new((0..32).map(|k| [90, 140, (150 + 2 * k) as u8]).collect());
        assert_eq!(rows_uniform(&index_frame(&inp, &mut fine, DitherMode::Temporal, 1.0)), h);
        // grob: zwei Farben für den ganzen Verlauf → gemischt (viele Zeilen nicht einfarbig)
        let mut coarse = Palette::new(vec![[90, 140, 150], [90, 140, 213]]);
        assert!(rows_uniform(&index_frame(&inp, &mut coarse, DitherMode::Temporal, 1.0)) < h / 2);
    }

    #[test]
    fn ordered_mix_keeps_mean_color() {
        let mut pal = Palette::new(vec![[0, 0, 0], [255, 255, 255]]);
        let (w, h) = (64, 64);
        let px = vec![OPAQUE | pack([128, 128, 128]); w * h];
        let lab: Vec<Lab> = px.iter().map(|p| lab_of(*p)).collect();
        let st = vec![false; w * h];
        let edge = vec![0.0; w * h];
        let inp = FrameInput { w, h, px: &px, lab: &lab, static_mask: &st, prev_idx: None, edge: &edge };
        for mode in [DitherMode::Ordered, DitherMode::BlueNoise, DitherMode::Temporal, DitherMode::FloydSteinberg] {
            let idx = index_frame(&inp, &mut pal, mode, 1.0);
            let white = idx.iter().filter(|i| **i == 1).count() as f32 / (w * h) as f32;
            // Oklab-Mitte von Grau 128 liegt bei L ≈ 0,6 → Anteil Weiß ≈ 0,6
            assert!((0.45..0.75).contains(&white), "{mode:?}: {white}");
        }
        let none = index_frame(&inp, &mut pal, DitherMode::None, 1.0);
        assert!(none.iter().all(|i| *i == none[0]));
    }
}
