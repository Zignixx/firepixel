//! Frames (SPEC-BATCH §5.6 Punkte 1 und 4): Vorverarbeitung (Alpha → 1 Bit, Bildrate ausdünnen,
//! fast gleiche Frames zusammenlegen, zeitliche Glättung) und Planung je Frame gegen das sichtbare
//! Bild (Bilddifferenz → Transparenz, Zuschnitt auf das Rechteck der Änderungen).

use super::color::{dist2, rgb_to_lab, unpack, Lab};
use super::dither::{pack_px, OPAQUE, TRANSPARENT};
use super::lzw_lossy::PlanPx;

/// Vorbereitetes Frame (Leinwandgröße).
#[derive(Clone, Debug)]
pub struct PFrame {
    /// Gepackte Pixel (`OPAQUE | rgb`, 0 = transparent).
    pub px: Vec<u32>,
    pub delay_ms: u32,
    /// Gegenüber dem Vorframe unverändert (nach der Glättung).
    pub static_mask: Vec<bool>,
}

/// Einstellungen der Vorverarbeitung.
#[derive(Clone, Copy, Debug)]
pub struct PrepSettings {
    /// Transparenz behalten (sonst auf `matte` setzen).
    pub transparency: bool,
    pub matte: [u8; 3],
    /// Jedes n-te Frame behalten (Dauer wird addiert).
    pub frame_step: usize,
    /// Fast gleiche Frames zusammenlegen.
    pub merge_similar: bool,
    /// Oklab-Abstand „unverändert“ (0 = nur exakt gleich).
    pub threshold: f32,
}

/// Bereitet Frames vor. `frames`: RGBA-Pixel der Leinwand (Zeilen, `w × h`) und Dauer in ms.
pub fn prepare(w: usize, h: usize, frames: Vec<(Vec<[u8; 4]>, u32)>, s: &PrepSettings) -> Vec<PFrame> {
    let n = w * h;
    let step = s.frame_step.max(1);
    // Bildrate ausdünnen
    let mut thinned: Vec<(Vec<[u8; 4]>, u32)> = Vec::new();
    for (i, (px, d)) in frames.into_iter().enumerate() {
        if i % step == 0 {
            thinned.push((px, d));
        } else if let Some(last) = thinned.last_mut() {
            last.1 = last.1.saturating_add(d);
        }
    }
    let thr2 = s.threshold * s.threshold;
    let mut out: Vec<PFrame> = Vec::with_capacity(thinned.len());
    let mut prev_lab: Vec<Lab> = Vec::new();
    for (px, delay) in thinned {
        let mut packed: Vec<u32> = px
            .iter()
            .map(|p| {
                if s.transparency {
                    pack_px(*p)
                } else {
                    let a = p[3] as u32;
                    let mix = |c: u8, m: u8| ((c as u32 * a + m as u32 * (255 - a) + 127) / 255) as u8;
                    OPAQUE | super::color::pack([mix(p[0], s.matte[0]), mix(p[1], s.matte[1]), mix(p[2], s.matte[2])])
                }
            })
            .collect();
        packed.resize(n, 0);
        let mut lab: Vec<Lab> = packed.iter().map(|p| if *p & OPAQUE != 0 { rgb_to_lab(unpack(*p)) } else { [0.0; 3] }).collect();
        let Some(prev) = out.last_mut() else {
            out.push(PFrame { px: packed, delay_ms: delay, static_mask: vec![false; n] });
            prev_lab = lab;
            continue;
        };
        // 0 = geändert, 1 = gleich, 2 = kleine Änderung (Kandidat für die Glättung)
        let mut kind = vec![0u8; n];
        for i in 0..n {
            let (a, b) = (packed[i], prev.px[i]);
            kind[i] = if a == b {
                1
            } else if a & OPAQUE != 0 && b & OPAQUE != 0 && thr2 > 0.0 && dist2(&lab[i], &prev_lab[i]) < thr2 {
                2
            } else {
                0
            };
        }
        // Gleichgerichtete kleine Änderungen (Überblendung, Helligkeitsverlauf) sind Inhalt, kein
        // Rauschen: dann in diesem Frame nicht glätten (sonst ginge eine langsame Blende verloren).
        let (mut sum_dl, mut sum_abs, mut cnt2) = (0f64, 0f64, 0usize);
        for i in 0..n {
            if kind[i] == 2 {
                let dl = (lab[i][0] - prev_lab[i][0]) as f64;
                sum_dl += dl;
                sum_abs += dl.abs();
                cnt2 += 1;
            }
        }
        if cnt2 * 4 > n && cnt2 >= 16 && sum_abs > 0.0 && sum_dl.abs() > 0.5 * sum_abs {
            kind.iter_mut().filter(|k| **k == 2).for_each(|k| *k = 0);
        }
        // Glätten nur, wo die ganze 3×3-Umgebung ruhig ist: Rauschen auf stehendem Hintergrund wird
        // eingefroren, bewegte Inhalte (Kamerafahrt, Kanten) verschmieren nicht.
        let mut st = vec![false; n];
        let mut changed = 0usize;
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                let calm = kind[i] == 1
                    || (kind[i] == 2 && {
                        let (x0, x1, y0, y1) = (x.saturating_sub(1), (x + 1).min(w - 1), y.saturating_sub(1), (y + 1).min(h - 1));
                        (y0..=y1).all(|yy| (x0..=x1).all(|xx| kind[yy * w + xx] != 0))
                    });
                if calm {
                    st[i] = true;
                } else {
                    changed += 1;
                }
            }
        }
        for i in 0..n {
            if st[i] && kind[i] == 2 {
                packed[i] = prev.px[i];
                lab[i] = prev_lab[i];
            }
        }
        if s.merge_similar && changed == 0 {
            prev.delay_ms = prev.delay_ms.saturating_add(delay);
            continue;
        }
        out.push(PFrame { px: packed, delay_ms: delay, static_mask: st });
        prev_lab = lab;
    }
    out
}

/// Zeitliche Glättung auf RGBA-Frames (Schritt `anim.stabilize`): deckende Pixel, die sich gegenüber
/// dem (geglätteten) Vorframe um weniger als `threshold` (Oklab) ändern, übernehmen den Vorwert.
/// Liefert die Zahl der eingefrorenen Pixel.
pub fn stabilize(frames: &mut [Vec<[u8; 4]>], threshold: f32) -> usize {
    let thr2 = threshold * threshold;
    let mut frozen = 0;
    for k in 1..frames.len() {
        let (head, tail) = frames.split_at_mut(k);
        let prev = &head[k - 1];
        for (i, p) in tail[0].iter_mut().enumerate() {
            let q = prev.get(i).copied().unwrap_or([0; 4]);
            if *p != q && p[3] >= 128 && q[3] >= 128 && (p[3] as i32 - q[3] as i32).abs() < 8 && dist2(&rgb_to_lab([p[0], p[1], p[2]]), &rgb_to_lab([q[0], q[1], q[2]])) < thr2 {
                *p = q;
                frozen += 1;
            }
        }
    }
    frozen
}

/// Rechteck (links, oben, Breite, Höhe).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

impl Rect {
    pub fn union(self, o: Rect) -> Rect {
        let x0 = self.x.min(o.x);
        let y0 = self.y.min(o.y);
        let x1 = (self.x + self.w).max(o.x + o.w);
        let y1 = (self.y + self.h).max(o.y + o.h);
        Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
    }
    pub fn contains(&self, x: usize, y: usize) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
}

/// Umgebendes Rechteck der Pixel mit `pred`.
pub fn bbox(w: usize, h: usize, pred: impl Fn(usize) -> bool) -> Option<Rect> {
    let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0, 0);
    for y in 0..h {
        for x in 0..w {
            if pred(y * w + x) {
                x0 = x0.min(x);
                x1 = x1.max(x);
                y0 = y0.min(y);
                y1 = y1.max(y);
            }
        }
    }
    (x0 != usize::MAX).then(|| Rect { x: x0, y: y0, w: x1 - x0 + 1, h: y1 - y0 + 1 })
}

/// Eingaben der Planung eines Frames.
pub struct PlanInput<'a> {
    pub w: usize,
    pub h: usize,
    /// Gewünschtes sichtbares Bild (Palettenfarben nach Dithering, gepackt).
    pub desired: &'a [u32],
    /// Indizes nach Dithering ([`TRANSPARENT`] für transparent).
    pub idx: &'a [u16],
    /// Oklab der Quelle (vor der Quantisierung).
    pub src_lab: &'a [Lab],
    pub static_mask: &'a [bool],
    /// Transparenzindex der Palette dieses Frames.
    pub transparent: Option<u8>,
    pub diff_transparency: bool,
    pub crop: bool,
    /// Oklab-Abstand, bis zu dem ein statisches Pixel trotz kleiner Abweichung des sichtbaren Bildes
    /// (z. B. durch eine verlustbehaftete Ersetzung im Vorframe) als unverändert gilt.
    pub keep_tolerance: f32,
    /// Farben der Palette dieses Frames.
    pub palette: &'a [[u8; 3]],
}

/// Prüft, ob das Frame auf `base` gezeichnet werden kann (kein Pixel muss transparent **werden**).
/// Liefert sonst das Rechteck der Pixel, die erst gelöscht werden müssten.
pub fn needs_clear(inp: &PlanInput, base: &[u32]) -> Option<Rect> {
    bbox(inp.w, inp.h, |i| inp.desired[i] == 0 && base[i] != 0)
}

/// Pixel `i` ist gegenüber dem sichtbaren Bild `base` unverändert (muss nicht gezeichnet werden).
fn unchanged_px(inp: &PlanInput, base: &[u32], i: usize) -> bool {
    let (d, b) = (inp.desired[i], base[i]);
    if d == b {
        return true;
    }
    // Quelle unverändert und sichtbar deckend: das Sichtbare bleibt (auch wenn der Vorframe es
    // verlustbehaftet oder mit anderer Palette gezeichnet hat) – kein Flimmern, kein Neuzeichnen
    let tol2 = inp.keep_tolerance * inp.keep_tolerance;
    d != 0 && b != 0 && (inp.static_mask[i] || (tol2 > 0.0 && dist2(&rgb_to_lab(unpack(d)), &rgb_to_lab(unpack(b))) <= tol2))
}

/// Plant ein Frame gegen das sichtbare Bild `base`: Rechteck und Pixel-Plan für den Kodierer.
pub fn plan(inp: &PlanInput, base: &[u32]) -> (Rect, Vec<PlanPx>) {
    plan_in(inp, base, None)
}

/// Wie [`plan`], aber nur die Änderungen innerhalb von `limit` (Teilbild eines aufgeteilten Frames).
pub fn plan_in(inp: &PlanInput, base: &[u32], limit: Option<Rect>) -> (Rect, Vec<PlanPx>) {
    let unchanged = |i: usize| unchanged_px(inp, base, i);
    let rect = if inp.crop || limit.is_some() {
        let lim = limit.unwrap_or(Rect { x: 0, y: 0, w: inp.w, h: inp.h });
        bbox(inp.w, inp.h, |i| lim.contains(i % inp.w, i / inp.w) && !unchanged(i)).unwrap_or(Rect { x: lim.x, y: lim.y, w: 1, h: 1 })
    } else {
        Rect { x: 0, y: 0, w: inp.w, h: inp.h }
    };
    let mut pal_index: super::color::ColorMap<u8> = Default::default();
    for (k, c) in inp.palette.iter().enumerate() {
        pal_index.entry(super::color::pack(*c)).or_insert(k as u8);
    }
    let mut out = Vec::with_capacity(rect.w * rect.h);
    for y in rect.y..rect.y + rect.h {
        for x in rect.x..rect.x + rect.w {
            let i = y * inp.w + x;
            let d = inp.desired[i];
            let idx = inp.idx[i];
            if d == 0 || idx == TRANSPARENT {
                // gewünscht transparent; base ist dann ebenfalls transparent (sonst ungültig)
                out.push(PlanPx::fixed(inp.transparent.unwrap_or(0)));
            } else if unchanged(i) {
                let reference = rgb_to_lab(unpack(base[i]));
                match inp.transparent {
                    Some(t) if inp.diff_transparency => out.push(PlanPx { default: t, flex: true, can_transp: true, reference, forbid: 256 }),
                    // ohne Differenz-Transparenz: unverändert neu zeichnen, ohne Verlust (zeitlich stabil)
                    _ => out.push(PlanPx { default: idx as u8, flex: false, can_transp: false, reference, forbid: 256 }),
                }
            } else {
                let forbid = if base[i] != 0 { pal_index.get(&(base[i] & 0x00ff_ffff)).map_or(256, |v| *v as u16) } else { 256 };
                out.push(PlanPx { default: idx as u8, flex: true, can_transp: false, reference: inp.src_lab[i], forbid });
            }
        }
    }
    (rect, out)
}

/// Zellengröße der Bereichssuche.
const CELL: usize = 8;

/// Getrennte Änderungsbereiche eines Frames (BATCH-8b, Bildschirmaufnahmen): Änderungen in 8×8-Zellen,
/// Zusammenhang über die 8er-Nachbarschaft, danach werden Bereiche zusammengelegt, die näher als `gap`
/// Pixel beieinander liegen (ein bewegtes Objekt mit alter und neuer Lage bleibt ein Bereich) oder
/// deren gemeinsames Rechteck geschätzt nicht teurer ist als zwei Teilbilder. Höchstens `max_parts`
/// Bereiche (die billigsten Paare werden zuerst vereint); Ergebnis nach Zahl der Änderungen absteigend,
/// jeweils das Rechteck genau um die Änderungen. Leer, wenn sich nichts ändert.
pub fn change_regions(inp: &PlanInput, base: &[u32], gap: usize, max_parts: usize) -> Vec<Rect> {
    let (w, h) = (inp.w, inp.h);
    let (gw, gh) = (w.div_ceil(CELL), h.div_ceil(CELL));
    let mut cell = vec![false; gw * gh];
    for y in 0..h {
        for x in 0..w {
            if !unchanged_px(inp, base, y * w + x) {
                cell[(y / CELL) * gw + x / CELL] = true;
            }
        }
    }
    // Zusammenhangskomponenten der Zellen → (Rechteck, Zahl der Änderungen)
    let mut comp = vec![usize::MAX; gw * gh];
    let mut regions: Vec<(Rect, usize)> = Vec::new();
    for start in 0..gw * gh {
        if !cell[start] || comp[start] != usize::MAX {
            continue;
        }
        let id = regions.len();
        let mut stack = vec![start];
        comp[start] = id;
        let mut cells = Vec::new();
        while let Some(c) = stack.pop() {
            cells.push(c);
            let (cx, cy) = (c % gw, c / gw);
            for ny in cy.saturating_sub(1)..=(cy + 1).min(gh - 1) {
                for nx in cx.saturating_sub(1)..=(cx + 1).min(gw - 1) {
                    let nc = ny * gw + nx;
                    if cell[nc] && comp[nc] == usize::MAX {
                        comp[nc] = id;
                        stack.push(nc);
                    }
                }
            }
        }
        let (mut x0, mut y0, mut x1, mut y1, mut cnt) = (usize::MAX, usize::MAX, 0, 0, 0usize);
        for c in cells {
            let (cx, cy) = (c % gw * CELL, c / gw * CELL);
            for y in cy..(cy + CELL).min(h) {
                for x in cx..(cx + CELL).min(w) {
                    if !unchanged_px(inp, base, y * w + x) {
                        x0 = x0.min(x);
                        y0 = y0.min(y);
                        x1 = x1.max(x);
                        y1 = y1.max(y);
                        cnt += 1;
                    }
                }
            }
        }
        regions.push((Rect { x: x0, y: y0, w: x1 - x0 + 1, h: y1 - y0 + 1 }, cnt));
    }
    // geschätzte Bytes eines Teilbilds: Kopf + LZW-Läufe über die Fläche (≈ √(2·Fläche) Codes)
    let cost = |r: &Rect| 24.0 + 1.2 * ((2 * r.w * r.h) as f32).sqrt();
    let dist = |a: &Rect, b: &Rect| {
        let dx = b.x.saturating_sub(a.x + a.w).max(a.x.saturating_sub(b.x + b.w));
        let dy = b.y.saturating_sub(a.y + a.h).max(a.y.saturating_sub(b.y + b.h));
        dx.max(dy)
    };
    loop {
        let mut best: Option<(f32, usize, usize)> = None;
        for i in 0..regions.len() {
            for j in i + 1..regions.len() {
                let (a, b) = (&regions[i].0, &regions[j].0);
                let extra = cost(&a.union(*b)) - cost(a) - cost(b);
                let must = dist(a, b) < gap || extra <= 0.0 || regions.len() > max_parts.max(1);
                if must && best.is_none_or(|bb| extra < bb.0) {
                    best = Some((extra, i, j));
                }
            }
        }
        let Some((_, i, j)) = best else { break };
        let b = regions.swap_remove(j);
        regions[i] = (regions[i].0.union(b.0), regions[i].1 + b.1);
    }
    regions.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.y.cmp(&b.0.y)).then(a.0.x.cmp(&b.0.x)));
    regions.into_iter().map(|r| r.0).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn st(transparency: bool, merge: bool, thr: f32) -> PrepSettings {
        PrepSettings { transparency, matte: [255, 255, 255], frame_step: 1, merge_similar: merge, threshold: thr }
    }

    #[test]
    fn merge_and_smoothing() {
        let a = vec![[10, 10, 10, 255]; 4];
        let mut b = a.clone();
        b[1] = [11, 10, 10, 255]; // winzige Änderung
        let mut c = a.clone();
        c[2] = [200, 0, 0, 255];
        let out = prepare(2, 2, vec![(a.clone(), 100), (b, 50), (c, 70)], &st(true, true, 0.03));
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].delay_ms, 150);
        assert_eq!(out[1].static_mask, vec![true, true, false, true]);
        // ohne Glättung bleibt die kleine Änderung ein eigenes Frame
        let mut b = a.clone();
        b[1] = [11, 10, 10, 255];
        assert_eq!(prepare(2, 2, vec![(a.clone(), 100), (b, 50)], &st(true, true, 0.0)).len(), 2);
        // Alpha → Matte
        let t = vec![[0, 0, 0, 0]; 4];
        let o = prepare(2, 2, vec![(t, 10)], &st(false, true, 0.0));
        assert_eq!(o[0].px[0], OPAQUE | 0xffffff);
        // Bildrate ausdünnen: Dauer bleibt erhalten
        let f: Vec<(Vec<[u8; 4]>, u32)> = (0..5).map(|k| (vec![[k * 40, 0, 0, 255]; 4], 40)).collect();
        let o = prepare(2, 2, f, &PrepSettings { frame_step: 2, ..st(true, false, 0.0) });
        assert_eq!(o.iter().map(|f| f.delay_ms).collect::<Vec<_>>(), vec![80, 80, 40]);
    }

    /// Getrennte Änderungen → eigene Bereiche (nach Größe), nahe beieinander → ein Bereich,
    /// höchstens `max_parts`; Rechtecke genau um die Änderungen.
    #[test]
    fn change_regions_split_merge_and_limit() {
        let (w, h) = (96usize, 64usize);
        let base = vec![OPAQUE | 0x00ff_ffff; w * h];
        let regions_for = |spots: &[(usize, usize, usize)], max_parts: usize| -> Vec<Rect> {
            let mut desired = base.clone();
            for &(x0, y0, s) in spots {
                for y in y0..y0 + s {
                    for x in x0..x0 + s {
                        desired[y * w + x] = OPAQUE;
                    }
                }
            }
            let idx = vec![0u16; w * h];
            let lab = vec![[0.0f32; 3]; w * h];
            let st = vec![false; w * h];
            let pal = [[0u8, 0, 0], [255, 255, 255]];
            let inp = PlanInput { w, h, desired: &desired, idx: &idx, src_lab: &lab, static_mask: &st, transparent: Some(2), diff_transparency: true, crop: true, keep_tolerance: 0.0, palette: &pal };
            change_regions(&inp, &base, 24, max_parts)
        };
        // zwei weit getrennte Flecken: zwei Bereiche, der größere zuerst
        let r = regions_for(&[(2, 2, 3), (80, 50, 5)], 4);
        assert_eq!(r, vec![Rect { x: 80, y: 50, w: 5, h: 5 }, Rect { x: 2, y: 2, w: 3, h: 3 }]);
        // nah beieinander (alte und neue Lage eines Objekts): ein Bereich
        let r = regions_for(&[(10, 10, 4), (22, 12, 4)], 4);
        assert_eq!(r, vec![Rect { x: 10, y: 10, w: 16, h: 6 }]);
        // höchstens ein Teil → alles in einem Rechteck
        let r = regions_for(&[(2, 2, 3), (80, 50, 5), (2, 55, 2)], 1);
        assert_eq!(r, vec![Rect { x: 2, y: 2, w: 83, h: 55 }]);
        // nichts geändert → keine Bereiche
        assert!(regions_for(&[], 4).is_empty());
    }

    #[test]
    fn stabilize_freezes_small_changes_only() {
        let mut f = vec![vec![[100, 100, 100, 255], [0, 0, 0, 255]], vec![[101, 100, 100, 255], [255, 0, 0, 255]]];
        assert_eq!(stabilize(&mut f, 0.03), 1);
        assert_eq!(f[1][0], [100, 100, 100, 255]);
        assert_eq!(f[1][1], [255, 0, 0, 255]);
        // 50× angewendet = einmal
        let once = f.clone();
        for _ in 0..50 {
            stabilize(&mut f, 0.03);
        }
        assert_eq!(f, once);
    }
}
