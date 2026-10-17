//! Built-in swatch sets, "Sort by Color" and the swatch grid geometry of the swatches panel.
//!
//! Colours are plain `0x00RRGGBB` values. The tables are generated from their construction
//! rules (hue ramp, web cube, system palettes, grey ramp) instead of being stored as dumps.
//!
//! Preference `SwatchesPanelColorPaletteChoice`: 0 = "Color Cubes" (315-colour hue ramp, the
//! default), 1 = custom, 2 = Grayscale, 3 = Macintosh System, 4 = Windows System,
//! 5 = "Continuous Tone" (web-216 cube laid out in rows of 21).

/// Grid pitch of the swatches panel in pixels: 8 px colour plus a 1 px line.
pub const GRID_PITCH: u32 = 9;

/// Number of grid columns for a client width and the vertical scrollbar width:
/// `(client_w − scrollbar_w) / 9` (at least 1).
pub fn grid_columns(client_w: u32, scrollbar_w: u32) -> usize {
    (client_w.saturating_sub(scrollbar_w) / GRID_PITCH).max(1) as usize
}

const fn rgb(r: u32, g: u32, b: u32) -> u32 {
    (r << 16) | (g << 8) | b
}

/// Channel value of a hue ramp at quarter step `q` (0..=4) between `lo` and `hi`; the
/// distance from the nearer end point is rounded half up.
fn ramp_step(lo: u32, hi: u32, q: u32) -> u32 {
    let d = hi - lo;
    if q <= 2 {
        lo + (d * q + 2) / 4
    } else {
        hi - (d * (4 - q) + 2) / 4
    }
}

/// Colour at hue position `p` (quarter sextants, 0 = red, 4 = yellow, 8 = green, 12 = cyan,
/// 16 = blue, 20 = magenta) with channel range `lo..=hi`.
fn hue_quarter(p: u32, lo: u32, hi: u32) -> u32 {
    let (s, q) = (p / 4 % 6, p % 4);
    let up = ramp_step(lo, hi, q);
    let down = ramp_step(lo, hi, 4 - q);
    match s {
        0 => rgb(hi, up, lo),
        1 => rgb(down, hi, lo),
        2 => rgb(lo, hi, up),
        3 => rgb(lo, down, hi),
        4 => rgb(up, lo, hi),
        _ => rgb(hi, lo, down),
    }
}

/// Fully saturated colour at half-sextant `h` (0..12); the half step is 0x7F.
fn hue_half(h: u32) -> u32 {
    let (s, half) = (h / 2 % 6, h % 2 == 1);
    let (up, down) = if half { (0x7F, 0x7F) } else { (0, 0xFF) };
    match s {
        0 => rgb(0xFF, up, 0),
        1 => rgb(down, 0xFF, 0),
        2 => rgb(0, 0xFF, up),
        3 => rgb(0, down, 0xFF),
        4 => rgb(up, 0, 0xFF),
        _ => rgb(0xFF, 0, down),
    }
}

/// Brightness levels of the hue ramp: `round(255 · f)` for f = 0.25, 0.40, 0.55, 0.70, 0.85.
/// (0.70 · 255 lies just below 178.5 in binary floating point, hence 178.)
const RAMP_LEVELS: [u32; 5] = [64, 102, 140, 178, 217];

/// "Color Cubes" (preference 0, default): 15 rows of 21.
///
/// Rows 0–10: the 21 hues red → yellow → green → cyan → blue → magenta in quarter-sextant
/// steps, from light to dark: five tints (minimum channel `255 − level`), the pure hues, five
/// shades (maximum channel `level`). Row 11: 21 × black. Rows 12–14: grey ramps of 15 steps
/// (neutral, yellowish, bluish) plus black, black and four saturated hues in half-sextant
/// steps (red…chartreuse, azure…green, blue…rose).
pub fn color_cubes() -> Vec<u32> {
    let mut out = Vec::with_capacity(315);
    let mut row = |lo: u32, hi: u32| (0..21).for_each(|p| out.push(hue_quarter(p, lo, hi)));
    for l in RAMP_LEVELS {
        row(255 - l, 255);
    }
    row(0, 255);
    for l in RAMP_LEVELS.iter().rev() {
        row(0, *l);
    }
    out.extend(std::iter::repeat_n(0, 21));
    // Grey ramps: k = 0 white, k = 1..=14 tinted grey, k = 15 and 16 black.
    // (value at k = 0, blue offset, half-sextant hues): grey k = base − 17·k.
    let ramps: [(u32, i32, [u32; 4]); 3] = [
        (255, 0, [0, 1, 2, 3]),
        (257, -15, [7, 6, 5, 4]),
        (253, 15, [8, 9, 10, 11]),
    ];
    for (base, blue, hues) in ramps {
        out.push(0xFF_FFFF);
        out.extend((1..15).map(|k| {
            let v = base - 17 * k;
            rgb(v, v, v.saturating_add_signed(blue))
        }));
        out.extend([0, 0]);
        out.extend(hues.iter().map(|&h| hue_half(h)));
    }
    out
}

/// "Continuous Tone" (preference 5): 12 rows of 21 plus one black entry (253).
///
/// Each row starts with three edge columns: black, a marker colour (greys 00…FF, then red,
/// green, blue, yellow, cyan, magenta) and black. Then 18 web colours: rows 0–5 use red
/// CC/66/00 with green FF − 33·row, rows 6–11 red FF/99/33 with green 33·(row − 6); blue runs
/// FF→00, 00→FF, FF→00 in the three groups of six.
pub fn continuous_tone() -> Vec<u32> {
    const MARKERS: [u32; 12] = [
        0x00_0000, 0x33_3333, 0x66_6666, 0x99_9999, 0xCC_CCCC, 0xFF_FFFF, 0xFF_0000, 0x00_FF00,
        0x00_00FF, 0xFF_FF00, 0x00_FFFF, 0xFF_00FF,
    ];
    let mut out = Vec::with_capacity(253);
    for (row, marker) in MARKERS.iter().enumerate() {
        let row = row as u32;
        out.extend([0, *marker, 0]);
        let (reds, g) = if row < 6 {
            ([0xCC, 0x66, 0x00], 0xFF - 0x33 * row)
        } else {
            ([0xFF, 0x99, 0x33], 0x33 * (row - 6))
        };
        for (i, r) in reds.into_iter().enumerate() {
            for j in 0..6 {
                let b = if i == 1 { 0x33 * j } else { 0xFF - 0x33 * j };
                out.push(rgb(r, g, b));
            }
        }
    }
    out.push(0);
    out
}

/// "Grayscale" (preference 2): 256 greys from white to black.
pub fn grayscale() -> Vec<u32> {
    (0..=255u32).rev().map(|v| rgb(v, v, v)).collect()
}

/// "Macintosh System" (preference 3): the Macintosh system palette in reverse order: black,
/// grey ramp 11…EE, blue, green and red ramps 11…EE (the steps not on the 0x33 grid), then the
/// 6×6×6 cube without black from 000033 to FFFFFF.
pub fn macintosh_system() -> Vec<u32> {
    let ramp: Vec<u32> = (1..16u32)
        .map(|k| k * 0x11)
        .filter(|v| v % 0x33 != 0)
        .collect();
    let mut out = vec![0];
    out.extend(ramp.iter().map(|&v| rgb(v, v, v)));
    out.extend(ramp.iter().map(|&v| rgb(0, 0, v)));
    out.extend(ramp.iter().map(|&v| rgb(0, v, 0)));
    out.extend(ramp.iter().map(|&v| rgb(v, 0, 0)));
    for r in 0..6 {
        for g in 0..6 {
            for b in 0..6 {
                if r + g + b > 0 {
                    out.push(rgb(r * 0x33, g * 0x33, b * 0x33));
                }
            }
        }
    }
    out
}

/// "Windows System" (preference 4): the Windows halftone system palette in reverse order
/// (256 entries, `#808080` twice).
///
/// Forward order: 10 low static colours, a 7×9×4 colour grid (red 00/2A/55/7F/AA/D4/FF, green
/// in steps of 0x20 from 1F, blue 00/55/AA/FF) rotated to start at red 2A / green 3F / blue AA,
/// six extra pastel colours at the wrap point, then 10 high static colours. Grid entries with
/// red 0 and green below 7F and the primaries/white of the static colours are left out.
pub fn windows_system() -> Vec<u32> {
    const LOW: [u32; 10] = [
        0x00_0000, 0x80_0000, 0x00_8000, 0x80_8000, 0x00_0080, 0x80_0080, 0x00_8080, 0x80_8080,
        0xC0_DCC0, 0xA6_CAF0,
    ];
    const HIGH: [u32; 10] = [
        0xFF_FBF0, 0xA0_A0A4, 0x80_8080, 0xFF_0000, 0x00_FF00, 0xFF_FF00, 0x00_00FF, 0xFF_00FF,
        0x00_FFFF, 0xFF_FFFF,
    ];
    const EXTRA: [u32; 6] = [
        0xCC_CCFF, 0xFF_CCFF, 0x33_FFFF, 0x66_FFFF, 0x99_FFFF, 0xCC_FFFF,
    ];
    let reds = [0x00, 0x2A, 0x55, 0x7F, 0xAA, 0xD4, 0xFF];
    let greens = [0x00, 0x1F, 0x3F, 0x5F, 0x7F, 0x9F, 0xBF, 0xDF, 0xFF];
    let blues = [0x00, 0x55, 0xAA, 0xFF];
    let mut grid = Vec::with_capacity(252);
    for r in reds {
        for g in greens {
            grid.extend(blues.map(|b| rgb(r, g, b)));
        }
    }
    // Start of the rotation: red 2A, green 3F, blue AA (grid index 46).
    let start = 46;
    let mut fwd: Vec<u32> = LOW.to_vec();
    let keep = |c: &&u32| !HIGH[3..].contains(c);
    fwd.extend(grid[start..].iter().filter(keep));
    fwd.extend(EXTRA);
    fwd.extend(grid[16..start].iter().filter(keep));
    fwd.extend(HIGH);
    fwd.reverse();
    fwd
}

/// Built-in set for a value of `SwatchesPanelColorPaletteChoice` (1 = custom → empty).
pub fn builtin_set(choice: i64) -> Vec<u32> {
    match choice {
        0 => color_cubes(),
        2 => grayscale(),
        3 => macintosh_system(),
        4 => windows_system(),
        5 => continuous_tone(),
        _ => Vec::new(),
    }
}

// ------------------------------------------------------------------ Sort by Color

fn channels(c: u32) -> (i32, i32, i32) {
    (
        (c >> 16 & 0xFF) as i32,
        (c >> 8 & 0xFF) as i32,
        (c & 0xFF) as i32,
    )
}

/// Brightness key: `(R·109 + G·366 + B·37) >> 9` (0..255).
pub fn sort_luminance(c: u32) -> u8 {
    let (r, g, b) = channels(c);
    ((r * 109 + g * 366 + b * 37) >> 9) as u8
}

/// Hue key on a 0..255 scale (HSV with sextant width 42; ties of the maximum prefer R, then G).
pub fn sort_hue(c: u32) -> u8 {
    let (r, g, b) = channels(c);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    if max == 0 || d == 0 {
        return 0;
    }
    let mut h = if max == r {
        (g - b) as f64 * 42.0 / d as f64
    } else if max == g {
        (b - r) as f64 * 42.0 / d as f64 + 85.0
    } else {
        (r - g) as f64 * 42.0 / d as f64 + 170.0
    };
    if h < 0.0 {
        h += 255.0;
    }
    crate::round::round_even(h).clamp(0.0, 255.0) as u8
}

/// Selection with a window: repeatedly takes the entry with the smallest (`take_max = false`)
/// or largest key among the first `window` remaining entries; on ties the last one wins. The
/// window restarts after `window` picks.
fn window_select(mut src: Vec<u32>, window: usize, key: fn(u32) -> u8, take_max: bool) -> Vec<u32> {
    let mut out = Vec::with_capacity(src.len());
    let mut w = window.min(src.len());
    while !src.is_empty() {
        let mut best_i = 0;
        let mut best = if take_max { 0 } else { 255 };
        for (i, &c) in src.iter().take(w.max(1)).enumerate() {
            let k = key(c);
            if (take_max && k >= best) || (!take_max && k <= best) {
                best = k;
                best_i = i;
            }
        }
        out.push(src.remove(best_i));
        w = w.saturating_sub(1);
        if w == 0 {
            w = window.min(src.len());
        }
    }
    out
}

/// "Sort by Color": greys (R = G = B) move to the end, light to dark; the other colours are
/// ordered by hue ascending and then every run of `columns` entries (one grid row) by
/// brightness descending.
pub fn sort_by_color(colors: &[u32], columns: usize) -> Vec<u32> {
    let mut list: Vec<u32> = colors.iter().map(|c| c & 0xFF_FFFF).collect();
    let mut greys = Vec::new();
    for i in (0..list.len()).rev() {
        let (r, g, b) = channels(list[i]);
        if r == g && g == b {
            greys.push(list.remove(i));
        }
    }
    let n = greys.len();
    let greys = window_select(greys, n, sort_luminance, true);
    let n = list.len();
    let chrom = window_select(list, n, sort_hue, false);
    let mut out = window_select(chrom, columns.max(1), sort_luminance, true);
    out.extend(greys);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(v: &[u32]) -> Vec<String> {
        v.iter().map(|c| format!("{c:06X}")).collect()
    }

    #[test]
    fn color_cubes_table() {
        let v = color_cubes();
        assert_eq!(v.len(), 315);
        assert_eq!(hex(&v[..3]), ["FFBFBF", "FFCFBF", "FFDFBF"]);
        assert_eq!(v[5], 0xEF_FFBF);
        assert_eq!(hex(&v[312..]), ["7F00FF", "FF00FF", "FF007F"]);
        // Row 5 holds the pure hues, row 10 the darkest shade, row 11 black.
        assert_eq!(v[5 * 21 + 20], 0xFF_00FF);
        assert_eq!(v[10 * 21], 0x40_0000);
        assert_eq!(v[10 * 21 + 20], 0x40_0040);
        assert!(v[11 * 21..12 * 21].iter().all(|&c| c == 0));
        // Rounding of the intermediate hue steps.
        assert_eq!(hex(&v[21..25]), ["FF9999", "FFB399", "FFCC99", "FFE599"]);
        assert_eq!(hex(&v[63..67]), ["FF4D4D", "FF7A4D", "FFA64D", "FFD24D"]);
        assert_eq!(v[7 * 21], 0xB2_0000);
        assert_eq!(hex(&v[13 * 21..13 * 21 + 2]), ["FFFFFF", "F0F0E1"]);
        assert_eq!(v[14 * 21 + 14], 0x0F_0F1E);
    }

    #[test]
    fn continuous_tone_table() {
        let v = continuous_tone();
        assert_eq!(v.len(), 253);
        assert_eq!(hex(&v[..4]), ["000000", "000000", "000000", "CCFFFF"]);
        assert_eq!(hex(&v[250..]), ["33FF33", "33FF00", "000000"]);
        let mut web: Vec<u32> = v
            .chunks(21)
            .filter(|r| r.len() == 21)
            .flat_map(|r| r[3..].to_vec())
            .collect();
        web.sort_unstable();
        web.dedup();
        assert_eq!(web.len(), 216);
        assert_eq!(v.iter().filter(|&&c| c == 0).count(), 27);
    }

    #[test]
    fn system_and_grey_tables() {
        let w = windows_system();
        assert_eq!(w.len(), 256);
        assert_eq!(hex(&w[..3]), ["FFFFFF", "00FFFF", "FF00FF"]);
        assert_eq!(hex(&w[253..]), ["008000", "800000", "000000"]);
        assert_eq!(w.iter().filter(|&&c| c == 0x80_8080).count(), 2);
        let mut d = w.clone();
        d.sort_unstable();
        d.dedup();
        assert_eq!(d.len(), 255);
        let m = macintosh_system();
        assert_eq!(m.len(), 256);
        assert_eq!(hex(&m[..3]), ["000000", "111111", "222222"]);
        assert_eq!(hex(&m[253..]), ["FFFF99", "FFFFCC", "FFFFFF"]);
        let g = grayscale();
        assert_eq!(g.len(), 256);
        assert_eq!(hex(&g[..2]), ["FFFFFF", "FEFEFE"]);
        assert_eq!(hex(&g[254..]), ["010101", "000000"]);
        assert_eq!(builtin_set(1), Vec::<u32>::new());
        assert_eq!(builtin_set(0).len(), 315);
        assert_eq!(builtin_set(5).len(), 253);
    }

    #[test]
    fn grid_geometry() {
        assert_eq!(GRID_PITCH, 9);
        assert_eq!(grid_columns(206, 17), 21);
        assert_eq!(grid_columns(10, 17), 1);
    }

    #[test]
    fn keys() {
        assert_eq!(sort_luminance(0xFF_FFFF), 255);
        assert_eq!(sort_luminance(0xF0_DCB4), 221);
        assert_eq!(sort_hue(0xFF_0000), 0);
        assert_eq!(sort_hue(0x00_FF00), 85);
        assert_eq!(sort_hue(0x00_00FF), 170);
        assert_eq!(sort_hue(0xFF_00FF), 213);
        assert_eq!(sort_hue(0x80_8080), 0);
    }

    /// Replace with the 772-byte test palette, then sort (21 columns).
    #[test]
    fn sort_reference_example() {
        let mut data: Vec<u8> = [
            0x000000u32,
            0xFFFFFF,
            0xFF0000,
            0x00FF00,
            0x0000FF,
            0xFFFF00,
            0xFF00FF,
            0x00FFFF,
            0x808080,
            0xC04040,
            0x4080C0,
            0xFF8000,
            0x800080,
            0x008040,
            0xF0DCB4,
            0x282850,
        ]
        .iter()
        .flat_map(|c| [(c >> 16) as u8, (c >> 8) as u8, *c as u8])
        .collect();
        data.resize(768, 0);
        data.extend_from_slice(&[0, 0x10, 0xFF, 0xFF]);
        let cols: Vec<u32> = crate::swatches::read_swatches_original(&data, None)
            .unwrap()
            .iter()
            .map(|c| rgb(c.r as u32, c.g as u32, c.b as u32))
            .collect();
        assert_eq!(cols.len(), 257);
        let s = sort_by_color(&cols, 21);
        assert_eq!(s.len(), 257);
        assert_eq!(
            hex(&s[..17]),
            [
                "FFFF00", "F0DCB4", "00FFFF", "00FF00", "FF8000", "4080C0", "008040", "C04040",
                "FF00FF", "FF0000", "282850", "800080", "0010FF", "0000FF", "FFFFFF", "808080",
                "000000"
            ]
        );
        assert_eq!(s[5], 0x40_80C0);
        assert!(s[16..].iter().all(|&c| c == 0));
        let lums: Vec<u8> = s[..14].iter().map(|&c| sort_luminance(c)).collect();
        assert_eq!(
            lums,
            [236, 221, 200, 182, 145, 119, 96, 91, 72, 54, 42, 36, 29, 18]
        );
    }

    /// With more colours than columns the hue order decides the rows.
    #[test]
    fn sort_windows_by_row() {
        let v = [0x00_00FF, 0xFF_0000, 0x00_FF00, 0x80_0000];
        // Hue order: 800000 (hue 0, the last of the tie wins), FF0000, 00FF00, 0000FF.
        assert_eq!(
            sort_by_color(&v, 2),
            vec![0xFF_0000, 0x80_0000, 0x00_FF00, 0x00_00FF]
        );
        assert_eq!(
            sort_by_color(&v, 4),
            vec![0x00_FF00, 0xFF_0000, 0x80_0000, 0x00_00FF]
        );
    }
}
