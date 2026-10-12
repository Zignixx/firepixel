//! Farbreduktion (Median-Cut auf einem 5-Bit-Histogramm) und Suche einer unbenutzten
//! Schlüsselfarbe für transparente Pixel.

use visioncortex::Color;

/// Histogrammfach (5 Bit je Kanal).
#[inline]
fn bin5(r: u8, g: u8, b: u8) -> usize {
    ((r as usize >> 3) << 10) | ((g as usize >> 3) << 5) | (b as usize >> 3)
}

#[derive(Clone, Copy)]
struct Entry {
    bin: usize,
    count: u64,
    sum: [u64; 3],
}

impl Entry {
    fn avg(&self, c: usize) -> u64 {
        self.sum[c] / self.count.max(1)
    }
}

/// Reduziert die deckenden Pixel (`skip[i] = false`) auf höchstens `n` Farben. Jedes Pixel wird
/// auf die nächste Palettenfarbe gesetzt (Zuordnung über das 5-Bit-Fach).
pub(crate) fn reduce(px: &mut [u8], skip: &[bool], n: usize) {
    let mut hist = vec![Entry { bin: 0, count: 0, sum: [0; 3] }; 1 << 15];
    for (p, &s) in px.as_chunks::<4>().0.iter().zip(skip) {
        if s {
            continue;
        }
        let e = &mut hist[bin5(p[0], p[1], p[2])];
        e.count += 1;
        for (s, &v) in e.sum.iter_mut().zip(p) {
            *s += v as u64;
        }
    }
    let mut entries: Vec<Entry> = hist.iter().enumerate().filter(|(_, e)| e.count > 0).map(|(i, e)| Entry { bin: i, ..*e }).collect();
    if entries.is_empty() {
        return;
    }
    // Weniger Farben als erlaubt: nichts zu tun (genaue Farben bleiben erhalten).
    if entries.len() <= n {
        return;
    }
    // Boxen als Bereiche in `entries`.
    let mut boxes: Vec<(usize, usize)> = vec![(0, entries.len())];
    while boxes.len() < n {
        // Box mit der größten (Spannweite × Pixelzahl) teilen.
        let mut best: Option<(usize, usize, u64)> = None; // (Box, Kanal, Wert)
        for (bi, &(a, b)) in boxes.iter().enumerate() {
            if b - a < 2 {
                continue;
            }
            let count: u64 = entries[a..b].iter().map(|e| e.count).sum();
            for c in 0..3 {
                let (lo, hi) = entries[a..b].iter().fold((u64::MAX, 0), |(lo, hi), e| (lo.min(e.avg(c)), hi.max(e.avg(c))));
                let score = (hi - lo) * count;
                if hi > lo && best.is_none_or(|x| score > x.2) {
                    best = Some((bi, c, score));
                }
            }
        }
        let Some((bi, c, _)) = best else { break };
        let (a, b) = boxes[bi];
        entries[a..b].sort_by_key(|e| e.avg(c));
        let total: u64 = entries[a..b].iter().map(|e| e.count).sum();
        let mut acc = 0;
        let mut cut = a + 1;
        for (k, e) in entries[a..b].iter().enumerate() {
            acc += e.count;
            if acc * 2 >= total {
                cut = (a + k + 1).clamp(a + 1, b - 1);
                break;
            }
        }
        boxes[bi] = (a, cut);
        boxes.push((cut, b));
    }
    let palette: Vec<[u8; 3]> = boxes
        .iter()
        .map(|&(a, b)| {
            let cnt: u64 = entries[a..b].iter().map(|e| e.count).sum::<u64>().max(1);
            let mut c = [0u8; 3];
            for (k, v) in c.iter_mut().enumerate() {
                *v = (entries[a..b].iter().map(|e| e.sum[k]).sum::<u64>() / cnt) as u8;
            }
            c
        })
        .collect();
    // Fach → nächste Palettenfarbe (gemessen an der mittleren Farbe des Fachs).
    let mut lut = vec![0u8; 1 << 15];
    for e in &entries {
        let col = [e.avg(0) as i32, e.avg(1) as i32, e.avg(2) as i32];
        let mut bestk = 0;
        let mut bestd = i32::MAX;
        for (k, p) in palette.iter().enumerate() {
            let d = (0..3).map(|i| (col[i] - p[i] as i32).pow(2)).sum::<i32>();
            if d < bestd {
                bestd = d;
                bestk = k;
            }
        }
        lut[e.bin] = bestk as u8;
    }
    for (p, &s) in px.as_chunks_mut::<4>().0.iter_mut().zip(skip) {
        if s {
            continue;
        }
        let c = palette[lut[bin5(p[0], p[1], p[2])] as usize];
        p[..3].copy_from_slice(&c);
    }
}

/// Eine im Bild (ohne `skip`-Pixel) nicht vorkommende, deckende Farbe, möglichst weit von allen
/// benutzten Farben entfernt (Fächer mit `max(shift, 4)` Bit Verschiebung je Kanal).
pub(crate) fn unused_color(px: &[u8], skip: &[bool], shift: i32) -> Color {
    let s = shift.clamp(4, 6) as u32;
    let levels = 256usize >> s;
    let idx = |r: usize, g: usize, b: usize| (r * levels + g) * levels + b;
    let mut used = vec![false; levels * levels * levels];
    for (i, p) in px.as_chunks::<4>().0.iter().enumerate() {
        if skip.get(i).copied().unwrap_or(false) {
            continue;
        }
        used[idx(p[0] as usize >> s, p[1] as usize >> s, p[2] as usize >> s)] = true;
    }
    let center = |v: usize| ((v << s) + (1 << (s - 1))).min(255) as u8;
    // Bevorzugt ein freies Fach, dessen Nachbarn ebenfalls frei sind.
    let mut any = None;
    for r in 0..levels {
        for g in 0..levels {
            for b in 0..levels {
                if used[idx(r, g, b)] {
                    continue;
                }
                any.get_or_insert((r, g, b));
                let mut free = true;
                'n: for dr in -1i32..=1 {
                    for dg in -1i32..=1 {
                        for db in -1i32..=1 {
                            let (rr, gg, bb) = (r as i32 + dr, g as i32 + dg, b as i32 + db);
                            if rr < 0 || gg < 0 || bb < 0 || rr >= levels as i32 || gg >= levels as i32 || bb >= levels as i32 {
                                continue;
                            }
                            if used[idx(rr as usize, gg as usize, bb as usize)] {
                                free = false;
                                break 'n;
                            }
                        }
                    }
                }
                if free {
                    return Color::new_rgba(center(r), center(g), center(b), 255);
                }
            }
        }
    }
    if let Some((r, g, b)) = any {
        return Color::new_rgba(center(r), center(g), center(b), 255);
    }
    // Alle Fächer belegt (Foto): genaue Suche nach einer unbenutzten 24-Bit-Farbe.
    let mut exact = vec![0u64; (1 << 24) / 64];
    for (i, p) in px.as_chunks::<4>().0.iter().enumerate() {
        if skip.get(i).copied().unwrap_or(false) {
            continue;
        }
        let v = ((p[0] as usize) << 16) | ((p[1] as usize) << 8) | p[2] as usize;
        exact[v / 64] |= 1 << (v % 64);
    }
    for (w, bits) in exact.iter().enumerate() {
        if *bits != u64::MAX {
            let v = w * 64 + (!bits).trailing_zeros() as usize;
            return Color::new_rgba((v >> 16) as u8, (v >> 8) as u8, v as u8, 255);
        }
    }
    Color::new_rgba(1, 2, 3, 255)
}
