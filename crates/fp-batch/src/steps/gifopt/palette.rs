//! Palette (SPEC-BATCH §5.6 Punkt 2): gewichtete Farbstatistik, Median-Cut mit varianzminimierender
//! Teilung (nach Wu 1991 / Heckbert 1982) und k-Means-Verfeinerung (Lloyd) – alles im Oklab-Raum,
//! eigene Umsetzung. Hat das Bild höchstens so viele Farben wie Plätze frei sind, werden sie exakt
//! übernommen (verlustfrei für Bildschirmaufnahmen, Pixelart, Zeichentrick mit flachen Farben).

use super::color::{dist2, lab_to_rgb, pack, rgb_to_lab, ColorMap, Lab};

/// Gewichtete Farbstatistik. Wird sie zu groß, werden Farben in gröbere Fächer zusammengelegt
/// (6, dann 5 Bit je Kanal; Farbe des Fachs = gewichteter Mittelwert in Oklab).
#[derive(Clone)]
pub struct Histogram {
    map: ColorMap<([f64; 3], f64)>,
    /// Bits je Kanal, die vom Schlüssel abgeschnitten werden (0 = exakt).
    shift: u32,
}

/// Höchstzahl der Einträge, bevor gröber zusammengelegt wird.
const MAX_ENTRIES: usize = 60_000;

impl Default for Histogram {
    fn default() -> Self {
        Histogram::new()
    }
}

impl Histogram {
    pub fn new() -> Histogram {
        Histogram { map: ColorMap::default(), shift: 0 }
    }

    fn key(&self, rgb: [u8; 3]) -> u32 {
        let m = !((1u8 << self.shift) - 1);
        pack([rgb[0] & m, rgb[1] & m, rgb[2] & m])
    }

    /// Farbe mit Gewicht `w` zählen.
    pub fn add(&mut self, rgb: [u8; 3], lab: &Lab, w: f32) {
        if w <= 0.0 {
            return;
        }
        let k = self.key(rgb);
        let e = self.map.entry(k).or_insert(([0.0; 3], 0.0));
        let w = w as f64;
        e.0[0] += lab[0] as f64 * w;
        e.0[1] += lab[1] as f64 * w;
        e.0[2] += lab[2] as f64 * w;
        e.1 += w;
        if self.map.len() > MAX_ENTRIES && self.shift < 3 {
            self.coarsen();
        }
    }

    fn coarsen(&mut self) {
        self.shift += if self.shift == 0 { 2 } else { 1 };
        let old = std::mem::take(&mut self.map);
        for (k, (s, w)) in old {
            let rgb = super::color::unpack(k);
            let nk = self.key(rgb);
            let e = self.map.entry(nk).or_insert(([0.0; 3], 0.0));
            e.0[0] += s[0];
            e.0[1] += s[1];
            e.0[2] += s[2];
            e.1 += w;
        }
    }

    /// Zahl der (ggf. zusammengelegten) Farben.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Exakt (nicht zusammengelegt)?
    pub fn exact(&self) -> bool {
        self.shift == 0
    }

    /// Einträge (Farbe, Gewicht), deterministisch nach Schlüssel sortiert.
    pub fn entries(&self) -> Vec<(u32, Lab, f32)> {
        let mut v: Vec<(u32, Lab, f32)> = self
            .map
            .iter()
            .map(|(k, (s, w))| (*k, [(s[0] / w) as f32, (s[1] / w) as f32, (s[2] / w) as f32], *w as f32))
            .collect();
        v.sort_unstable_by_key(|e| e.0);
        v
    }
}

/// Palette mit Zwischenspeicher für die Suche der nächsten Farbe.
#[derive(Clone)]
pub struct Palette {
    pub rgb: Vec<[u8; 3]>,
    pub lab: Vec<Lab>,
    cache: ColorMap<u8>,
}

impl Palette {
    pub fn new(rgb: Vec<[u8; 3]>) -> Palette {
        let lab = rgb.iter().map(|c| rgb_to_lab(*c)).collect();
        Palette { rgb, lab, cache: ColorMap::default() }
    }

    pub fn len(&self) -> usize {
        self.rgb.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rgb.is_empty()
    }

    /// Nächste Farbe zu einem Oklab-Wert (vollständige Suche).
    pub fn nearest_lab(&self, c: &Lab) -> u8 {
        let mut best = 0usize;
        let mut bd = f32::MAX;
        for (i, p) in self.lab.iter().enumerate() {
            let d = dist2(p, c);
            if d < bd {
                bd = d;
                best = i;
            }
        }
        best as u8
    }

    /// Nächste Farbe zu einer sRGB-Farbe (zwischengespeichert).
    pub fn nearest(&mut self, rgb: [u8; 3]) -> u8 {
        let k = pack(rgb);
        if let Some(i) = self.cache.get(&k) {
            return *i;
        }
        let i = self.nearest_lab(&rgb_to_lab(rgb));
        if self.cache.len() > 1 << 20 {
            self.cache.clear();
        }
        self.cache.insert(k, i);
        i
    }

    /// Index einer exakt enthaltenen Farbe.
    pub fn exact(&self, rgb: [u8; 3]) -> Option<u8> {
        self.rgb.iter().position(|c| *c == rgb).map(|i| i as u8)
    }

    /// Mittlerer Abstand der Palettenfarben zu ihrem nächsten Nachbarn (Maß für die Streuung beim
    /// geordneten Dithering).
    pub fn mean_spacing(&self) -> f32 {
        if self.lab.len() < 2 {
            return 0.05;
        }
        let mut sum = 0.0;
        for (i, a) in self.lab.iter().enumerate() {
            let mut bd = f32::MAX;
            for (j, b) in self.lab.iter().enumerate() {
                if i != j {
                    bd = bd.min(dist2(a, b));
                }
            }
            sum += bd.sqrt();
        }
        sum / self.lab.len() as f32
    }
}

/// Bildet höchstens `k` Farben aus der Statistik.
pub fn build(hist: &Histogram, k: usize) -> Vec<[u8; 3]> {
    let k = k.clamp(1, 256);
    let entries = hist.entries();
    if entries.is_empty() {
        return vec![[0, 0, 0]];
    }
    if entries.len() <= k && hist.exact() {
        return entries.iter().map(|e| super::color::unpack(e.0)).collect();
    }
    let pts: Vec<(Lab, f32)> = entries.iter().map(|e| (e.1, e.2)).collect();
    let mut centers = median_cut(&pts, k);
    kmeans(&pts, &mut centers, 6);
    let mut out: Vec<[u8; 3]> = Vec::with_capacity(centers.len());
    for c in centers {
        let rgb = lab_to_rgb(c);
        if !out.contains(&rgb) {
            out.push(rgb);
        }
    }
    out
}

struct CutBox {
    lo: usize,
    hi: usize,
    sse: f64,
}

fn box_stats(pts: &[(Lab, f32)]) -> ([f64; 3], f64, f64) {
    let (mut s, mut w, mut q) = ([0f64; 3], 0f64, 0f64);
    for (c, wt) in pts {
        let wt = *wt as f64;
        for k in 0..3 {
            s[k] += c[k] as f64 * wt;
            q += (c[k] as f64) * (c[k] as f64) * wt;
        }
        w += wt;
    }
    let sse = if w > 0.0 { q - (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]) / w } else { 0.0 };
    (s, w, sse.max(0.0))
}

/// Median-Cut: teilt immer die Schachtel mit dem größten gewichteten Fehler entlang der Achse mit
/// der größten Varianz an der Stelle, die die Summe der Fehler beider Hälften minimiert.
fn median_cut(pts: &[(Lab, f32)], k: usize) -> Vec<Lab> {
    let mut pts = pts.to_vec();
    let (_, _, sse) = box_stats(&pts);
    let mut boxes = vec![CutBox { lo: 0, hi: pts.len(), sse }];
    while boxes.len() < k {
        let Some((bi, _)) = boxes.iter().enumerate().filter(|(_, b)| b.hi - b.lo > 1 && b.sse > 1e-12).max_by(|a, b| a.1.sse.total_cmp(&b.1.sse)) else {
            break;
        };
        let (lo, hi) = (boxes[bi].lo, boxes[bi].hi);
        let slice = &mut pts[lo..hi];
        // Achse mit der größten Varianz
        let (s, w, _) = box_stats(slice);
        let mut var = [0f64; 3];
        for (c, wt) in slice.iter() {
            for k in 0..3 {
                let d = c[k] as f64 - s[k] / w.max(1e-12);
                var[k] += d * d * *wt as f64;
            }
        }
        let axis = (0..3).max_by(|a, b| var[*a].total_cmp(&var[*b])).unwrap_or(0);
        slice.sort_by(|a, b| a.0[axis].total_cmp(&b.0[axis]));
        // Präfixsummen → beste Teilung
        let n = slice.len();
        let (tot_s, tot_w, _) = (s, w, ());
        let tot_q: f64 = slice.iter().map(|(c, wt)| (c[0] as f64 * c[0] as f64 + c[1] as f64 * c[1] as f64 + c[2] as f64 * c[2] as f64) * *wt as f64).sum();
        let (mut ls, mut lw, mut lq) = ([0f64; 3], 0f64, 0f64);
        let mut best = (f64::MAX, n / 2);
        for (i, (c, wt)) in slice.iter().enumerate().take(n - 1) {
            let wt = *wt as f64;
            for k in 0..3 {
                ls[k] += c[k] as f64 * wt;
            }
            lq += (c[0] as f64 * c[0] as f64 + c[1] as f64 * c[1] as f64 + c[2] as f64 * c[2] as f64) * wt;
            lw += wt;
            let rw = tot_w - lw;
            if lw <= 0.0 || rw <= 0.0 {
                continue;
            }
            let rs = [tot_s[0] - ls[0], tot_s[1] - ls[1], tot_s[2] - ls[2]];
            let e = (lq - (ls[0] * ls[0] + ls[1] * ls[1] + ls[2] * ls[2]) / lw) + ((tot_q - lq) - (rs[0] * rs[0] + rs[1] * rs[1] + rs[2] * rs[2]) / rw);
            if e < best.0 {
                best = (e, i + 1);
            }
        }
        let mid = lo + best.1.clamp(1, n - 1);
        let (_, _, sa) = box_stats(&pts[lo..mid]);
        let (_, _, sb) = box_stats(&pts[mid..hi]);
        boxes[bi] = CutBox { lo, hi: mid, sse: sa };
        boxes.push(CutBox { lo: mid, hi, sse: sb });
    }
    boxes
        .iter()
        .map(|b| {
            let (s, w, _) = box_stats(&pts[b.lo..b.hi]);
            let w = w.max(1e-12);
            [(s[0] / w) as f32, (s[1] / w) as f32, (s[2] / w) as f32]
        })
        .collect()
}

/// Gewichtete k-Means-Verfeinerung (Lloyd), höchstens `iters` Runden.
fn kmeans(pts: &[(Lab, f32)], centers: &mut [Lab], iters: usize) {
    let k = centers.len();
    if k < 2 {
        return;
    }
    let mut assign = vec![0usize; pts.len()];
    for _ in 0..iters {
        let mut sums = vec![([0f64; 3], 0f64); k];
        for (i, (c, w)) in pts.iter().enumerate() {
            // zuerst die bisherige Zuordnung als Schranke
            let mut best = assign[i];
            let mut bd = dist2(&centers[best], c);
            for (j, cc) in centers.iter().enumerate() {
                let d = dist2(cc, c);
                if d < bd {
                    bd = d;
                    best = j;
                }
            }
            assign[i] = best;
            let s = &mut sums[best];
            let w = *w as f64;
            s.0[0] += c[0] as f64 * w;
            s.0[1] += c[1] as f64 * w;
            s.0[2] += c[2] as f64 * w;
            s.1 += w;
        }
        let mut moved = 0f32;
        for (c, (s, w)) in centers.iter_mut().zip(sums) {
            if w > 0.0 {
                let n = [(s[0] / w) as f32, (s[1] / w) as f32, (s[2] / w) as f32];
                moved = moved.max(dist2(c, &n));
                *c = n;
            }
        }
        if moved < 1e-8 {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn few_colors_are_kept_exactly() {
        let mut h = Histogram::new();
        for c in [[1, 2, 3], [200, 10, 10], [0, 0, 0], [255, 255, 255]] {
            h.add(c, &rgb_to_lab(c), 1.0);
        }
        let p = build(&h, 16);
        assert_eq!(p.len(), 4);
        assert!(p.contains(&[1, 2, 3]) && p.contains(&[200, 10, 10]));
    }

    #[test]
    fn gradient_is_spread_evenly() {
        let mut h = Histogram::new();
        for v in 0..=255u8 {
            let c = [v, v, v];
            h.add(c, &rgb_to_lab(c), 1.0);
        }
        let p = build(&h, 16);
        assert_eq!(p.len(), 16);
        let mut ls: Vec<f32> = p.iter().map(|c| rgb_to_lab(*c)[0]).collect();
        ls.sort_by(f32::total_cmp);
        // Gleichmäßig in L verteilt: größte Lücke höchstens doppelt so groß wie die mittlere
        let gaps: Vec<f32> = ls.windows(2).map(|w| w[1] - w[0]).collect();
        let mean = gaps.iter().sum::<f32>() / gaps.len() as f32;
        assert!(gaps.iter().all(|g| *g < mean * 2.0), "{gaps:?}");
    }

    #[test]
    fn histogram_coarsens_and_stays_bounded() {
        let mut h = Histogram::new();
        for r in 0..=255u8 {
            for g in 0..=255u8 {
                let c = [r, g, 77];
                h.add(c, &rgb_to_lab(c), 1.0);
            }
        }
        assert!(h.len() <= MAX_ENTRIES && !h.exact());
        let p = build(&h, 256);
        assert!(p.len() > 200 && p.len() <= 256);
    }
}
