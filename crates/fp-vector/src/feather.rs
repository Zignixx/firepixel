//! Weiche Kante der Füllung (Spezifikation 07 §5.6, B `@00834ba0`/`@0082f6e0`/`@00830070`,
//! Referenz `pathrender.feather_mask`).
//!
//! Separabler Dreiecksfilter der Breite `2r + 1` mit Gewichten `1, 2, …, r+1, …, 1`, Normierung
//! `(r+1)²`; erst waagerecht, dann senkrecht, nach jedem Durchgang abschneidende Ganzzahldivision.
//! Gefiltert wird die Deckungsmaske **vor** der Farbanwendung. Eigenheit: die letzte Zeile des
//! senkrechten Durchgangs (= letzte Zeile mit Deckung + r) wird als `screen(v, v)` geschrieben
//! (Laufzeittest T6: 9.600/9.600, ohne die Eigenheit 9.519).
//!
//! Die Summen werden über zwei Kastenfilter der Breite `r + 1` gebildet (Dreieck = Kasten ∗
//! Kasten); das ist ganzzahlig exakt und damit bitgleich zur direkten Summe.

use crate::coverage::Coverage;
use fp_geom::IRect;

/// Größter Radius (`FEFi` wird auf 1024 begrenzt).
pub const MAX_FEATHER: u32 = 1024;

/// `screen(v, v) = 255 − mul(255−v, 255−v)`.
#[inline]
fn screen_self(v: u32) -> u8 {
    let t = (255 - v) * (255 - v) + 0x80;
    (255 - (((t >> 8) + t) >> 8)) as u8
}

/// Dreieckssumme einer Folge: `out[i] = Σ_k a[i+k]·w[k]`, k = 0..2r, für `i < n_out`
/// (fehlende Werte gelten als 0).
fn tri_sums(a: &[u32], r: usize, n_out: usize, out: &mut Vec<u64>) {
    let n = a.len();
    // Kasten 1: b[i] = Σ_{t=i}^{i+r} a[t], gebraucht für i < n_out + r
    let nb = n_out + r;
    let mut pre = vec![0u64; n + 1];
    for (i, &v) in a.iter().enumerate() {
        pre[i + 1] = pre[i] + v as u64;
    }
    let ps = |i: usize| -> u64 { pre[i.min(n)] };
    let mut b = vec![0u64; nb];
    for (i, bv) in b.iter_mut().enumerate() {
        *bv = ps(i + r + 1) - ps(i);
    }
    let mut preb = vec![0u64; nb + 1];
    for i in 0..nb {
        preb[i + 1] = preb[i] + b[i];
    }
    out.clear();
    for i in 0..n_out {
        out.push(preb[(i + r + 1).min(nb)] - preb[i.min(nb)]);
    }
}

/// Weiche Kante: `big` ist die Deckung über `window` erweitert um `r` je Seite; Ergebnis über
/// `window`. Außerhalb von `big` gilt 0.
pub fn feather(big: &Coverage, r: u32, window: IRect) -> Coverage {
    let r = r.min(MAX_FEATHER) as usize;
    let mut out = Coverage::new(window);
    let (w, h) = (out.width(), out.height());
    let (mw, mh) = (big.width(), big.height());
    if w == 0 || h == 0 || mw != w + 2 * r || mh != h + 2 * r {
        return out;
    }
    let den = ((r + 1) * (r + 1)) as u64;
    // waagerecht: tmp[j][i] = Σ_k big[j][i+k]·w[k] / den, j = 0..mh
    let mut tmp = vec![0u32; w * mh];
    let mut row = vec![0u32; mw];
    let mut sums = Vec::new();
    for j in 0..mh {
        let src = &big.data[j * mw..(j + 1) * mw];
        if src.iter().all(|&v| v == 0) {
            continue;
        }
        for (d, s) in row.iter_mut().zip(src) {
            *d = *s as u32;
        }
        tri_sums(&row, r, w, &mut sums);
        for i in 0..w {
            tmp[j * w + i] = (sums[i] / den) as u32;
        }
    }
    let last = (0..mh)
        .rev()
        .find(|&j| big.data[j * mw..(j + 1) * mw].iter().any(|&v| v != 0));
    // senkrecht je Spalte
    let mut col = vec![0u32; mh];
    for i in 0..w {
        for (j, c) in col.iter_mut().enumerate() {
            *c = tmp[j * w + i];
        }
        tri_sums(&col, r, h, &mut sums);
        for (j, s) in sums.iter().enumerate().take(h) {
            let v = (s / den) as u32;
            out.data[j * w + i] = if Some(j) == last {
                screen_self(v)
            } else {
                v as u8
            };
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_direct_sum() {
        let r = 3usize;
        let win = IRect::new(0, 0, 9, 7);
        let mut big = Coverage::new(win.inflate(r as i32, r as i32));
        let mw = big.width();
        for j in 4..9 {
            for i in 3..11 {
                big.data[j * mw + i] = ((i * 37 + j * 11) % 256) as u8;
            }
        }
        let got = feather(&big, r as u32, win);
        // direkte Summe wie die Referenz
        let wts: Vec<u64> = (0..2 * r + 1)
            .map(|k| (k + 1).min(2 * r + 1 - k) as u64)
            .collect();
        let den = ((r + 1) * (r + 1)) as u64;
        let (w, h, mh) = (9usize, 7usize, 7 + 2 * r);
        let mut tmp = vec![0u64; w * mh];
        for j in 0..mh {
            for i in 0..w {
                let s: u64 = (0..2 * r + 1)
                    .map(|k| big.data[j * mw + i + k] as u64 * wts[k])
                    .sum();
                tmp[j * w + i] = s / den;
            }
        }
        let last = (0..mh)
            .rev()
            .find(|&j| (0..mw).any(|i| big.data[j * mw + i] != 0))
            .unwrap();
        for j in 0..h {
            for i in 0..w {
                let s: u64 = (0..2 * r + 1).map(|k| tmp[(j + k) * w + i] * wts[k]).sum();
                let mut v = (s / den) as u32;
                if j == last {
                    v = screen_self(v) as u32;
                }
                assert_eq!(got.data[j * w + i] as u32, v, "({i},{j})");
            }
        }
    }
}
