//! Kern des GIF-Optimierers: Palette(n), Dithering, Planung gegen das sichtbare Bild, Wahl der
//! Entsorgung (Disposal) je Frame nach kleinerer Größe, verlustbehaftetes LZW, GIF89a-Datei.

use super::color::{rgb_to_lab, unpack, Lab};
use super::dither::{index_frame, DitherMode, FrameInput, OPAQUE, TRANSPARENT};
use super::frames::{needs_clear, plan, PFrame, PlanInput, Rect};
use super::lzw_lossy::{self, LzwParams, PlanPx};
use super::palette::{build, Histogram, Palette};

/// Palettenart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteMode {
    Global,
    PerFrame,
    Auto,
}

/// Aufgelöste Einstellungen eines Kodiervorgangs.
#[derive(Clone, Debug)]
pub struct GifSettings {
    /// Höchste Palettengröße (2…256, inklusive Transparenz-Index).
    pub colors: usize,
    pub palette: PaletteMode,
    pub dither: DitherMode,
    /// Dithering-Stärke 0…1.
    pub dither_amount: f32,
    /// Verlust 0…200.
    pub lossy: u32,
    pub diff_transparency: bool,
    /// Oklab-Abstand „unverändert“.
    pub diff_threshold: f32,
    pub crop_changed: bool,
    pub merge_similar: bool,
    /// Entsorgung je Frame nach kleinerer Größe wählen (sonst immer „nicht entsorgen“, außer nötig).
    pub disposal_auto: bool,
    /// Transparenz behalten.
    pub transparency: bool,
    pub matte: [u8; 3],
    /// Wiederholungen (0 = endlos, < 0 = einmal abspielen).
    pub loop_count: i32,
    /// Größenfaktor (Zielgröße), 1 = unverändert.
    pub scale: f32,
    /// Jedes n-te Frame (Zielgröße), 1 = alle.
    pub frame_step: usize,
}

impl Default for GifSettings {
    fn default() -> Self {
        GifSettings {
            colors: 256,
            palette: PaletteMode::Auto,
            dither: DitherMode::Temporal,
            dither_amount: 0.7,
            lossy: 40,
            diff_transparency: true,
            diff_threshold: 0.03,
            crop_changed: true,
            merge_similar: true,
            disposal_auto: true,
            transparency: true,
            matte: [255, 255, 255],
            loop_count: 0,
            scale: 1.0,
            frame_step: 1,
        }
    }
}

impl GifSettings {
    /// Oklab-Schwelle des verlustbehafteten LZW.
    pub fn lossy_threshold(&self) -> f32 {
        self.lossy.min(200) as f32 / 200.0 * 0.09
    }
}

/// Kennzahlen eines Frames.
#[derive(Clone, Debug, PartialEq)]
pub struct FrameStat {
    pub rect: Rect,
    /// 1 = nicht entsorgen, 2 = Hintergrund.
    pub disposal: u8,
    pub local_palette: bool,
    pub delay_cs: u16,
    /// Bytes des Frames in der Datei.
    pub bytes: usize,
}

/// Kennzahlen eines Kodiervorgangs.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GifStats {
    pub width: usize,
    pub height: usize,
    pub frames: Vec<FrameStat>,
    /// Farben der globalen Palette (ohne Transparenz-Index).
    pub global_colors: usize,
    pub bytes: usize,
}

struct Encoded {
    rect: Rect,
    disposal: u8,
    delay_cs: u16,
    pal_id: usize,
    transparent: Option<u8>,
    chosen: Vec<u8>,
    data: Vec<u8>,
}

/// Kodiert vorbereitete Frames zu einer GIF-Datei.
pub fn encode(w: usize, h: usize, frames: &[PFrame], s: &GifSettings) -> (Vec<u8>, GifStats) {
    let n = w * h;
    let has_alpha = frames.iter().any(|f| f.px.iter().any(|p| *p & OPAQUE == 0));
    let need_transp = has_alpha || (frames.len() > 1 && s.diff_transparency);
    let k = s.colors.clamp(2, 256) - usize::from(need_transp);
    let k = k.max(1);
    // Oklab und Kanten je Frame
    let labs: Vec<Vec<Lab>> = frames.iter().map(|f| frame_lab(&f.px)).collect();
    let edges: Vec<Vec<f32>> = labs.iter().map(|l| edge_map(w, h, l)).collect();

    // --- Paletten
    let total: usize = n * frames.len();
    let sample = total.div_ceil(4_000_000).max(1);
    let hist_of = |range: std::ops::Range<usize>, all_pixels: bool| -> Histogram {
        let mut hist = Histogram::new();
        for fi in range {
            let f = &frames[fi];
            let off = fi % sample;
            for i in (off..n).step_by(sample) {
                let p = f.px[i];
                if p & OPAQUE == 0 {
                    continue;
                }
                // statische Pixel zählen weniger, Kanten mehr (SPEC §5.6 Punkt 2)
                let base = if fi == 0 || all_pixels || !f.static_mask[i] { 1.0 } else { 0.1 };
                let wgt = base * (1.0 + (edges[fi][i] * 5.0).min(1.0));
                hist.add(unpack(p), &labs[fi][i], wgt);
            }
        }
        hist
    };
    let mut palettes: Vec<Palette> = Vec::new();
    let global_hist = match s.palette {
        PaletteMode::PerFrame => hist_of(0..frames.len().min(1), true),
        _ => hist_of(0..frames.len(), false),
    };
    palettes.push(Palette::new(build(&global_hist, k)));
    let mut pal_of = vec![0usize; frames.len()];
    match s.palette {
        PaletteMode::Global => {}
        PaletteMode::PerFrame => {
            for (fi, slot) in pal_of.iter_mut().enumerate().skip(1) {
                palettes.push(Palette::new(build(&hist_of(fi..fi + 1, true), k)));
                *slot = palettes.len() - 1;
            }
        }
        PaletteMode::Auto => {
            // Szenen: ändert sich die grobe Farbverteilung (512 Fächer) um mehr als ein Drittel der
            // Fläche, beginnt eine neue (Kamerafahrten und Bewegung behalten ihre Verteilung).
            let mut starts = vec![0usize];
            let mut prev_h = coarse_hist(&frames[0].px, sample);
            for (fi, f) in frames.iter().enumerate().skip(1) {
                let h = coarse_hist(&f.px, sample);
                let moved: f32 = h.iter().zip(&prev_h).map(|(a, b)| (a - b).abs()).sum::<f32>() / 2.0;
                if moved > 0.33 {
                    starts.push(fi);
                }
                prev_h = h;
            }
            if starts.len() > 1 {
                let seg = |si: usize| starts[si]..starts.get(si + 1).copied().unwrap_or(frames.len());
                let mut seg_pal: Vec<Palette> = (0..starts.len()).map(|si| Palette::new(build(&hist_of(seg(si), false), k))).collect();
                let err = |pal: &mut Palette, si: usize| -> f32 {
                    let (mut s, mut c) = (0f32, 0f32);
                    for fi in seg(si) {
                        let (e, cnt) = frame_error(pal, &frames[fi], &labs[fi], fi == starts[si], sample);
                        s += e * cnt;
                        c += cnt;
                    }
                    if c > 0.0 { s / c } else { 0.0 }
                };
                // lohnt eine eigene Palette? (deutlich kleinerer Fehler)
                let better = |eg: f32, es: f32| eg > 1.2 * es + 0.0005;
                let e0 = (err(&mut palettes[0], 0), err(&mut seg_pal[0], 0));
                if better(e0.0, e0.1) {
                    // die erste Szene bestimmt die globale Palette
                    palettes[0] = seg_pal[0].clone();
                }
                for (si, sp) in seg_pal.iter_mut().enumerate().skip(1) {
                    let eg = err(&mut palettes[0], si);
                    let es = err(sp, si);
                    if better(eg, es) {
                        palettes.push(sp.clone());
                        let id = palettes.len() - 1;
                        for fi in seg(si) {
                            pal_of[fi] = id;
                        }
                    }
                }
            }
        }
    }
    let transp_of = |pid: usize, pals: &[Palette]| -> Option<u8> { need_transp.then(|| pals[pid].len() as u8) };

    // --- Indizieren (Dithering)
    let mut idxs: Vec<Vec<u16>> = Vec::with_capacity(frames.len());
    for (fi, f) in frames.iter().enumerate() {
        let prev = (fi > 0 && pal_of[fi - 1] == pal_of[fi]).then(|| idxs[fi - 1].as_slice());
        let inp = FrameInput { w, h, px: &f.px, lab: &labs[fi], static_mask: &f.static_mask, prev_idx: prev, edge: &edges[fi] };
        let pal = &mut palettes[pal_of[fi]];
        let v = index_frame(&inp, pal, s.dither, s.dither_amount);
        idxs.push(v);
    }

    // --- Planen und Kodieren
    let thr = s.lossy_threshold();
    let mut canvas = vec![0u32; n];
    let mut done: Vec<Encoded> = Vec::with_capacity(frames.len());
    let mut pending: Option<Encoded> = None;
    let mut acc_ms = 0u64;
    let mut emitted_cs = 0u64;
    for (fi, f) in frames.iter().enumerate() {
        let pid = pal_of[fi];
        let pal = &palettes[pid];
        let transparent = transp_of(pid, &palettes);
        let desired: Vec<u32> = idxs[fi].iter().map(|i| if *i == TRANSPARENT { 0 } else { OPAQUE | super::color::pack(pal.rgb[*i as usize]) }).collect();
        let inp = PlanInput {
            w,
            h,
            desired: &desired,
            idx: &idxs[fi],
            src_lab: &labs[fi],
            static_mask: &f.static_mask,
            transparent,
            diff_transparency: s.diff_transparency,
            crop: s.crop_changed,
            keep_tolerance: 0.0,
            palette: &pal.rgb,
        };
        let lzw = |planv: &[PlanPx], rect: Rect| -> (Vec<u8>, Vec<u8>) {
            let table = pal.len() + usize::from(transparent.is_some());
            let p = LzwParams { min_code_size: lzw_lossy::min_code_size(table), palette: &pal.lab, transparent, threshold: thr, width: rect.w };
            lzw_lossy::encode(planv, &p)
        };
        // Option K: Vorframe nicht entsorgen
        let clear_k = needs_clear(&inp, &canvas);
        let mut options: Vec<(u8, Vec<u32>, Option<Rect>)> = Vec::new();
        if clear_k.is_none() {
            options.push((1, canvas.clone(), None));
        }
        if let Some(pd) = &pending
            && (clear_k.is_some() || (s.disposal_auto && has_alpha))
        {
            // Option B: Rechteck des Vorframes auf Hintergrund (transparent) setzen
            let mut prect = pd.rect;
            let mut base = canvas.clone();
            clear_rect(&mut base, w, prect);
            if let Some(extra) = needs_clear(&inp, &base) {
                prect = prect.union(extra);
                base = canvas.clone();
                clear_rect(&mut base, w, prect);
            }
            options.push((2, base, Some(prect)));
        }
        if options.is_empty() {
            // erstes Frame mit Löschbedarf kann es nicht geben (Leinwand beginnt transparent)
            options.push((1, canvas.clone(), None));
        }
        let mut best: Option<(usize, Rect, Vec<u8>, Vec<u8>)> = None;
        for (oi, (_, base, _)) in options.iter().enumerate() {
            let (rect, planv) = plan(&inp, base);
            let (data, chosen) = lzw(&planv, rect);
            if best.as_ref().is_none_or(|b| data.len() < b.3.len()) {
                best = Some((oi, rect, chosen, data));
            }
        }
        let Some((oi, rect, chosen, data)) = best else { continue };
        let (disp, base, expand) = options.swap_remove(oi);
        if let Some(mut pd) = pending.take() {
            pd.disposal = disp;
            if let Some(er) = expand
                && er != pd.rect
            {
                reencode_expanded(&mut pd, er, &palettes);
            }
            done.push(pd);
        }
        canvas = base;
        for (k, c) in chosen.iter().enumerate() {
            let (x, y) = (rect.x + k % rect.w, rect.y + k / rect.w);
            if Some(*c) != transparent {
                canvas[y * w + x] = OPAQUE | super::color::pack(pal.rgb[*c as usize]);
            }
        }
        acc_ms += f.delay_ms as u64;
        let target_cs = (acc_ms + 5) / 10;
        let cs = if f.delay_ms == 0 { 0 } else { target_cs.saturating_sub(emitted_cs).max(2) };
        emitted_cs += cs;
        pending = Some(Encoded { rect, disposal: 1, delay_cs: cs.min(65535) as u16, pal_id: pid, transparent, chosen, data });
    }
    if let Some(mut pd) = pending.take() {
        pd.disposal = if has_alpha && frames.len() > 1 { 2 } else { 1 };
        done.push(pd);
    }
    write_gif(w, h, &palettes, need_transp, s.loop_count, &done, frames.len() > 1)
}

fn clear_rect(buf: &mut [u32], w: usize, r: Rect) {
    for y in r.y..r.y + r.h {
        buf[y * w + r.x..y * w + r.x + r.w].iter_mut().for_each(|p| *p = 0);
    }
}

/// Vergrößert das Rechteck eines schon kodierten Frames (für die Entsorgung „Hintergrund“) und kodiert
/// es mit genau denselben Pixeln neu (neue Fläche transparent).
fn reencode_expanded(pd: &mut Encoded, er: Rect, palettes: &[Palette]) {
    let t = pd.transparent.unwrap_or(0);
    let mut planv = vec![PlanPx::fixed(t); er.w * er.h];
    for (k, c) in pd.chosen.iter().enumerate() {
        let (x, y) = (pd.rect.x + k % pd.rect.w, pd.rect.y + k / pd.rect.w);
        planv[(y - er.y) * er.w + (x - er.x)] = PlanPx::fixed(*c);
    }
    let pal = &palettes[pd.pal_id];
    let table = pal.len() + usize::from(pd.transparent.is_some());
    let p = LzwParams { min_code_size: lzw_lossy::min_code_size(table), palette: &pal.lab, transparent: pd.transparent, threshold: 0.0, width: er.w };
    let (data, chosen) = lzw_lossy::encode(&planv, &p);
    pd.rect = er;
    pd.data = data;
    pd.chosen = chosen;
}

fn frame_lab(px: &[u32]) -> Vec<Lab> {
    let mut cache: super::color::ColorMap<Lab> = Default::default();
    px.iter()
        .map(|p| {
            if *p & OPAQUE == 0 {
                [0.0; 3]
            } else {
                *cache.entry(*p).or_insert_with(|| rgb_to_lab(unpack(*p)))
            }
        })
        .collect()
}

/// Kantenstärke: größter Oklab-Abstand zum rechten bzw. unteren Nachbarn.
fn edge_map(w: usize, h: usize, lab: &[Lab]) -> Vec<f32> {
    let mut e = vec![0f32; w * h];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let mut m = 0f32;
            if x + 1 < w {
                m = m.max(super::color::dist2(&lab[i], &lab[i + 1]));
            }
            if y + 1 < h {
                m = m.max(super::color::dist2(&lab[i], &lab[i + w]));
            }
            e[i] = m.sqrt();
        }
    }
    e
}

/// Grobe, normierte Farbverteilung (3 Bit je Kanal; transparent zählt nicht).
fn coarse_hist(px: &[u32], sample: usize) -> Vec<f32> {
    let mut h = vec![0f32; 512];
    let mut n = 0f32;
    for p in px.iter().step_by(sample.max(1)) {
        if *p & OPAQUE == 0 {
            continue;
        }
        let [r, g, b] = unpack(*p);
        h[((r >> 5) as usize) << 6 | ((g >> 5) as usize) << 3 | (b >> 5) as usize] += 1.0;
        n += 1.0;
    }
    if n > 0.0 {
        h.iter_mut().for_each(|v| *v /= n);
    }
    h
}

/// Mittlerer Fehler (Oklab) der geänderten Pixel eines Frames mit einer Palette, und deren Zahl.
fn frame_error(pal: &mut Palette, f: &PFrame, lab: &[Lab], all: bool, sample: usize) -> (f32, f32) {
    let (mut sum, mut cnt) = (0f32, 0f32);
    for i in (0..f.px.len()).step_by(sample) {
        let p = f.px[i];
        if p & OPAQUE == 0 || (!all && f.static_mask[i]) {
            continue;
        }
        let q = pal.nearest(unpack(p));
        sum += super::color::dist(&pal.lab[q as usize], &lab[i]);
        cnt += 1.0;
    }
    (if cnt > 0.0 { sum / cnt } else { 0.0 }, cnt)
}

/// Farbtabelle auf eine Zweierpotenz (≥ 2) auffüllen; liefert Tabelle und Größenfeld (n mit 2^(n+1)).
fn padded_table(pal: &Palette, transp: bool) -> (Vec<u8>, u8) {
    let entries = pal.len() + usize::from(transp);
    let mut size = 2usize;
    let mut field = 0u8;
    while size < entries {
        size <<= 1;
        field += 1;
    }
    let mut t = Vec::with_capacity(size * 3);
    for c in &pal.rgb {
        t.extend_from_slice(c);
    }
    t.resize(size * 3, 0);
    (t, field)
}

fn write_gif(w: usize, h: usize, palettes: &[Palette], transp: bool, loop_count: i32, frames: &[Encoded], animated: bool) -> (Vec<u8>, GifStats) {
    let mut out = Vec::with_capacity(1024 + frames.iter().map(|f| f.data.len() + 32).sum::<usize>());
    out.extend_from_slice(b"GIF89a");
    out.extend_from_slice(&(w.min(65535) as u16).to_le_bytes());
    out.extend_from_slice(&(h.min(65535) as u16).to_le_bytes());
    let (gct, gfield) = padded_table(&palettes[0], transp);
    out.push(0x80 | 0x70 | gfield);
    out.push(0); // Hintergrundfarbe
    out.push(0); // Seitenverhältnis
    out.extend_from_slice(&gct);
    if animated && loop_count >= 0 {
        out.extend_from_slice(&[0x21, 0xFF, 0x0B]);
        out.extend_from_slice(b"NETSCAPE2.0");
        out.extend_from_slice(&[0x03, 0x01]);
        out.extend_from_slice(&(loop_count.min(65535) as u16).to_le_bytes());
        out.push(0);
    }
    let mut stats = GifStats { width: w, height: h, frames: Vec::new(), global_colors: palettes[0].len(), bytes: 0 };
    for f in frames {
        let start = out.len();
        // Grafiksteuerung
        out.extend_from_slice(&[0x21, 0xF9, 0x04]);
        let tflag = u8::from(f.transparent.is_some());
        out.push((f.disposal & 7) << 2 | tflag);
        out.extend_from_slice(&f.delay_cs.to_le_bytes());
        out.push(f.transparent.unwrap_or(0));
        out.push(0);
        // Bildbeschreibung
        out.push(0x2C);
        for v in [f.rect.x, f.rect.y, f.rect.w, f.rect.h] {
            out.extend_from_slice(&(v.min(65535) as u16).to_le_bytes());
        }
        let local = f.pal_id != 0;
        let table_len = palettes[f.pal_id].len() + usize::from(f.transparent.is_some());
        if local {
            let (lct, lfield) = padded_table(&palettes[f.pal_id], f.transparent.is_some());
            out.push(0x80 | lfield);
            out.extend_from_slice(&lct);
        } else {
            out.push(0);
        }
        out.push(lzw_lossy::min_code_size(table_len));
        for chunk in f.data.chunks(255) {
            out.push(chunk.len() as u8);
            out.extend_from_slice(chunk);
        }
        out.push(0);
        stats.frames.push(FrameStat { rect: f.rect, disposal: f.disposal, local_palette: local, delay_cs: f.delay_cs, bytes: out.len() - start });
    }
    out.push(0x3B);
    stats.bytes = out.len();
    (out, stats)
}
