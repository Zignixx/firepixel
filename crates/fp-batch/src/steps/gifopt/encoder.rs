//! Kern des GIF-Optimierers: Palette(n), Dithering, Planung gegen das sichtbare Bild, Wahl der
//! Entsorgung (Disposal) je Frame nach kleinerer Größe, verlustbehaftetes LZW, GIF89a-Datei.

use super::color::{Lab, rgb_to_lab, unpack};
use super::dither::{DitherMode, FrameInput, OPAQUE, TRANSPARENT, index_frame};
use super::frames::{PFrame, PlanInput, Rect, change_regions, needs_clear, plan, plan_in};
use super::lzw_lossy::{self, LzwParams, PlanPx};
use super::palette::{Histogram, Palette, build_weighted};

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
    /// Inhaltsabhängige Palettengröße `[Grafik, Foto]` (BATCH-8b), höchstens `colors`; `None` = genau
    /// `colors`. Zwischen Grafik (≤ 4096 verschiedene Farben) und Foto (≥ 32768) logarithmisch.
    pub colors_auto: Option<[usize; 2]>,
    pub palette: PaletteMode,
    pub dither: DitherMode,
    /// Dithering-Stärke 0…1.
    pub dither_amount: f32,
    /// Verlust 0…200.
    pub lossy: u32,
    /// Inhaltsabhängiger Verlust `[Grafik, Foto]` (statt `lossy`, BATCH-8b).
    pub lossy_auto: Option<[u32; 2]>,
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
    /// Getrennte Änderungen eines Frames als Teilbilder speichern (BATCH-8b): vordere Teile je 20 ms
    /// vor dem Bildwechsel, das letzte zur Zeit des Frames.
    pub split_frames: bool,
    /// B-5: flexible (non-greedy) LZW parsing (`lzw: flexible`); off by default (output unchanged).
    pub lzw_flexible: bool,
    /// B-5: perceptual palette (`perceptualPalette`): colour statistics weighted by visibility (flat
    /// areas count more than busy texture, where quantization errors are masked); off by default.
    pub perceptual_palette: bool,
}

impl Default for GifSettings {
    fn default() -> Self {
        GifSettings {
            colors: 256,
            colors_auto: None,
            palette: PaletteMode::Auto,
            dither: DitherMode::Temporal,
            dither_amount: 0.7,
            lossy: 40,
            lossy_auto: None,
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
            split_frames: true,
            lzw_flexible: false,
            perceptual_palette: false,
        }
    }
}

/// Gewicht der Helligkeit für Paletten von Grafiken (BATCH-8b): Verläufe bekommen mehr
/// Helligkeitsstufen (Stufen in L fallen in ruhigen Flächen zuerst auf). Fotos: 1 (farbtreu, keine
/// Farbflecken); dazwischen nach dem Foto-Anteil.
pub const GRAPHIC_L_WEIGHT: f32 = 1.5;

impl GifSettings {
    /// Oklab-Schwelle des verlustbehafteten LZW.
    pub fn lossy_threshold(&self) -> f32 {
        lossy_to_threshold(self.lossy)
    }

    /// Palettengröße (ohne Transparenz-Index) und Verlust für einen Inhalt mit Foto-Anteil `photo`.
    pub fn for_content(&self, photo: f32) -> (usize, u32) {
        let p = photo.clamp(0.0, 1.0);
        let colors = match self.colors_auto {
            Some([g, f]) => {
                let (lg, lf) = ((g.max(2) as f32).log2(), (f.max(2) as f32).log2());
                (2f32.powf(lg + (lf - lg) * p).round() as usize).min(self.colors)
            }
            None => self.colors,
        };
        let lossy = match self.lossy_auto {
            Some([g, f]) => (g as f32 + (f as f32 - g as f32) * p).round() as u32,
            None => self.lossy,
        };
        (colors.clamp(2, 256), lossy.min(200))
    }
}

/// Oklab-Schwelle des verlustbehafteten LZW für `lossy` 0…200.
pub fn lossy_to_threshold(lossy: u32) -> f32 {
    lossy.min(200) as f32 / 200.0 * 0.09
}

/// Foto-Anteil 0…1 eines Bildes aus seinen verschiedenen Farben (BATCH-8b): Grafiken (Bildschirm,
/// Zeichentrick, Pixelart) haben wenige Farben je Pixel (≤ 2 %), Fotos/Videos durch Verläufe und Korn
/// viele (≥ 10 %), dazwischen logarithmisch; unter 256 Farben ist es immer Grafik, ab 2048 zählt nur
/// der Anteil.
pub fn photo_share(px: &[u32]) -> f32 {
    let mut seen: super::color::ColorMap<()> = Default::default();
    let mut n = 0usize;
    for p in px {
        if *p & OPAQUE != 0 {
            seen.insert(*p, ());
            n += 1;
        }
    }
    let u = seen.len().max(1) as f32;
    let share = u / n.max(1) as f32;
    let by_share =
        ((share.log2() - 0.02f32.log2()) / (0.1f32.log2() - 0.02f32.log2())).clamp(0.0, 1.0);
    let by_count = ((u.log2() - 8.0) / 3.0).clamp(0.0, 1.0);
    by_share.min(by_count)
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
    /// LZW-Mindest-Codelänge des Teilbilds.
    pub min_code_size: u8,
}

/// Kennzahlen eines Kodiervorgangs.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GifStats {
    pub width: usize,
    pub height: usize,
    pub frames: Vec<FrameStat>,
    /// Farben der globalen Palette (ohne Transparenz-Index).
    pub global_colors: usize,
    /// Foto-Anteil des Inhalts (0 = Grafik, 1 = Foto), siehe [`photo_share`].
    pub photo: f32,
    /// Verwendeter Verlust (0…200).
    pub lossy: u32,
    pub bytes: usize,
}

struct Encoded {
    rect: Rect,
    disposal: u8,
    delay_cs: u16,
    pal_id: usize,
    /// Transparenz-Index (intern; in der Datei immer Index 0, siehe [`FileOrder`]).
    transparent: Option<u8>,
    /// Gewählte Indizes (intern).
    chosen: Vec<u8>,
    data: Vec<u8>,
    /// LZW-Mindest-Codelänge dieses Teilbilds.
    mcs: u8,
}

/// Ergebnis einer LZW-Kodierung: Daten, gewählte Indizes (intern), Mindest-Codelänge.
type LzwOut = (Vec<u8>, Vec<u8>, u8);
/// Kodiertes Teilbild: Rechteck, gewählte Indizes (intern), Daten, Mindest-Codelänge.
type Part = (Rect, Vec<u8>, Vec<u8>, u8);

/// Reihenfolge der Farben in der Datei (BATCH-8b): Transparenz auf Index 0, dann die Farben nach
/// Häufigkeit in den geänderten Bereichen. Teilbilder, die nur die vorderen Indizes brauchen,
/// kommen mit einer kleineren LZW-Mindest-Codelänge aus (kürzere Codes), ohne eigene Farbtabelle.
struct FileOrder {
    /// intern → Datei (Länge Palette + 1; der letzte Eintrag ist der interne Transparenz-Index)
    to_file: Vec<u8>,
    /// Datei → intern
    to_int: Vec<u8>,
    /// Oklab der Farben in Datei-Reihenfolge (Transparenz: beliebig)
    lab: Vec<Lab>,
    /// RGB in Datei-Reihenfolge (ohne Transparenz-Eintrag)
    rgb: Vec<[u8; 3]>,
    transp: bool,
    /// B-5: flexible LZW parsing ([`GifSettings::lzw_flexible`]).
    flexible: bool,
}

impl FileOrder {
    fn new(pal: &Palette, usage: &[u64], transp: bool, flexible: bool) -> FileOrder {
        let k = pal.len();
        let mut colors: Vec<usize> = (0..k).collect();
        colors.sort_by(|a, b| usage[*b].cmp(&usage[*a]).then(a.cmp(b)));
        let off = usize::from(transp);
        let mut to_file = vec![0u8; k + 1];
        let mut to_int = vec![k as u8; k + off];
        for (pos, ci) in colors.iter().enumerate() {
            to_file[*ci] = (pos + off) as u8;
            to_int[pos + off] = *ci as u8;
        }
        to_file[k] = 0;
        let mut lab = vec![[0.0; 3]; k + off];
        for (pos, ci) in colors.iter().enumerate() {
            lab[pos + off] = pal.lab[*ci];
        }
        FileOrder {
            to_file,
            to_int,
            lab,
            rgb: colors.iter().map(|ci| pal.rgb[*ci]).collect(),
            transp,
            flexible,
        }
    }

    /// Kodiert einen Plan (interne Indizes) in Datei-Indizes mit passender Mindest-Codelänge.
    fn encode(&self, planv: &[PlanPx], width: usize, threshold: f32) -> LzwOut {
        let mapped: Vec<PlanPx> = planv
            .iter()
            .map(|q| PlanPx {
                default: self.to_file[q.default as usize],
                forbid: if q.forbid < 256 {
                    self.to_file[q.forbid as usize] as u16
                } else {
                    256
                },
                ..*q
            })
            .collect();
        // gewählt werden nur Vorgabe-Indizes oder Transparenz (0) → größter Vorgabe-Index reicht
        let max = mapped.iter().map(|q| q.default as usize).max().unwrap_or(0);
        let mcs = lzw_lossy::min_code_size(max + 1);
        let p = LzwParams {
            min_code_size: mcs,
            palette: &self.lab,
            transparent: self.transp.then_some(0),
            threshold,
            width,
            flexible: false,
        };
        // B-5: with `lzw: flexible` both parsings run in parallel, the smaller result is kept
        // (flexible parsing alone loses on sub-images where the greedy lookahead over equivalent
        // indices is better)
        let (data, chosen) = if self.flexible {
            let pf = LzwParams {
                flexible: true,
                ..p
            };
            let (g, f) = rayon::join(
                || lzw_lossy::encode(&mapped, &p),
                || lzw_lossy::encode(&mapped, &pf),
            );
            if f.0.len() < g.0.len() { f } else { g }
        } else {
            lzw_lossy::encode(&mapped, &p)
        };
        (
            data,
            chosen.iter().map(|c| self.to_int[*c as usize]).collect(),
            mcs,
        )
    }
}

/// Kodiert vorbereitete Frames zu einer GIF-Datei.
pub fn encode(w: usize, h: usize, frames: &[PFrame], s: &GifSettings) -> (Vec<u8>, GifStats) {
    let n = w * h;
    let has_alpha = frames.iter().any(|f| f.px.iter().any(|p| *p & OPAQUE == 0));
    let need_transp = has_alpha || (frames.len() > 1 && s.diff_transparency);
    // Oklab und Kanten je Frame
    // per frame in parallel (B-5; deterministic, same result as sequential)
    use rayon::prelude::*;
    let labs: Vec<Vec<Lab>> = frames.par_iter().map(|f| frame_lab(&f.px)).collect();
    let edges: Vec<Vec<f32>> = labs.par_iter().map(|l| edge_map(w, h, l)).collect();

    // --- Paletten
    let total: usize = n * frames.len();
    let sample = total.div_ceil(4_000_000).max(1);
    let hist_of = |range: std::ops::Range<usize>| -> Histogram {
        let mut hist = Histogram::new();
        for fi in range {
            let f = &frames[fi];
            let off = fi % sample;
            for i in (off..n).step_by(sample) {
                let p = f.px[i];
                if p & OPAQUE == 0 {
                    continue;
                }
                // jedes Pixel jedes Frames zählt gleich (BATCH-8b: gemessen besser als „statisch 0,1,
                // Kanten bis 2×“ – große ruhige Flächen bekommen feinere Stufen, die Palette streut
                // weniger in Mischfarben von Kanten)
                let weight = if s.perceptual_palette {
                    perceptual_weight(edges[fi][i])
                } else {
                    1.0
                };
                hist.add(unpack(p), &labs[fi][i], weight);
            }
        }
        hist
    };
    let mut palettes: Vec<Palette> = Vec::new();
    let global_hist = match s.palette {
        PaletteMode::PerFrame => hist_of(0..frames.len().min(1)),
        _ => hist_of(0..frames.len()),
    };
    // Inhalt: Grafik oder Foto → Palettengröße, Verlust, Gewicht der Helligkeit
    let photo = frames.first().map_or(0.0, |f| photo_share(&f.px));
    let (colors, lossy) = s.for_content(photo);
    let k = (colors - usize::from(need_transp)).max(1);
    let wl = GRAPHIC_L_WEIGHT + (1.0 - GRAPHIC_L_WEIGHT) * photo;
    let build = |h: &Histogram, k: usize| Palette::weighted(build_weighted(h, k, wl), wl);
    palettes.push(build(&global_hist, k));
    let mut pal_of = vec![0usize; frames.len()];
    match s.palette {
        PaletteMode::Global => {}
        PaletteMode::PerFrame => {
            for (fi, slot) in pal_of.iter_mut().enumerate().skip(1) {
                palettes.push(build(&hist_of(fi..fi + 1), k));
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
                let moved: f32 = h
                    .iter()
                    .zip(&prev_h)
                    .map(|(a, b)| (a - b).abs())
                    .sum::<f32>()
                    / 2.0;
                if moved > 0.33 {
                    starts.push(fi);
                }
                prev_h = h;
            }
            if starts.len() > 1 {
                let seg =
                    |si: usize| starts[si]..starts.get(si + 1).copied().unwrap_or(frames.len());
                let mut seg_pal: Vec<Palette> = (0..starts.len())
                    .map(|si| build(&hist_of(seg(si)), k))
                    .collect();
                let err = |pal: &mut Palette, si: usize| -> f32 {
                    let (mut s, mut c) = (0f32, 0f32);
                    for fi in seg(si) {
                        let (e, cnt) =
                            frame_error(pal, &frames[fi], &labs[fi], fi == starts[si], sample);
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
    let transp_of =
        |pid: usize, pals: &[Palette]| -> Option<u8> { need_transp.then(|| pals[pid].len() as u8) };

    // --- Indizieren (Dithering)
    let mut idxs: Vec<Vec<u16>> = Vec::with_capacity(frames.len());
    for (fi, f) in frames.iter().enumerate() {
        let prev = (fi > 0 && pal_of[fi - 1] == pal_of[fi]).then(|| idxs[fi - 1].as_slice());
        let inp = FrameInput {
            w,
            h,
            px: &f.px,
            lab: &labs[fi],
            static_mask: &f.static_mask,
            prev_idx: prev,
            edge: &edges[fi],
        };
        let pal = &mut palettes[pal_of[fi]];
        let v = index_frame(&inp, pal, s.dither, s.dither_amount);
        idxs.push(v);
    }

    // --- Reihenfolge in der Datei (Häufigkeit in den geänderten Bereichen)
    // (geänderte Pixel ab Frame 1 zählen 1024-fach, das erste Frame nur zur Ordnung bei Gleichstand:
    // es braucht ohnehin alle Farben)
    let mut usage: Vec<Vec<u64>> = palettes.iter().map(|p| vec![0u64; p.len()]).collect();
    for (fi, f) in frames.iter().enumerate() {
        let u = &mut usage[pal_of[fi]];
        for (i, v) in idxs[fi].iter().enumerate() {
            if *v != TRANSPARENT && (*v as usize) < u.len() {
                if fi > 0 && !f.static_mask[i] {
                    u[*v as usize] += 1024;
                } else if fi == 0 {
                    u[*v as usize] += 1;
                }
            }
        }
    }
    let orders: Vec<FileOrder> = palettes
        .iter()
        .zip(&usage)
        .map(|(p, u)| FileOrder::new(p, u, need_transp, s.lzw_flexible))
        .collect();

    // --- Planen und Kodieren
    let thr = lossy_to_threshold(lossy);
    let mut canvas = vec![0u32; n];
    let mut done: Vec<Encoded> = Vec::with_capacity(frames.len());
    let mut pending: Option<Encoded> = None;
    let mut acc_ms = 0u64;
    let mut emitted_cs = 0u64;
    for (fi, f) in frames.iter().enumerate() {
        let pid = pal_of[fi];
        let pal = &palettes[pid];
        let transparent = transp_of(pid, &palettes);
        let desired: Vec<u32> = idxs[fi]
            .iter()
            .map(|i| {
                if *i == TRANSPARENT {
                    0
                } else {
                    OPAQUE | super::color::pack(pal.rgb[*i as usize])
                }
            })
            .collect();
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
        // mit Verlust zusätzlich ohne Verlust kodieren und das kleinere nehmen: In Flächen und Verläufen
        // kann eine Ersetzung neue, seltene Folgen erzeugen – der Verlust darf nie größer machen
        let lzw = |planv: &[PlanPx], rect: Rect| -> LzwOut {
            let a = orders[pid].encode(planv, rect.w, thr);
            if thr <= 0.0 {
                return a;
            }
            let b = orders[pid].encode(planv, rect.w, 0.0);
            if b.0.len() <= a.0.len() { b } else { a }
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
        let mut best: Option<(usize, Part)> = None;
        for (oi, (_, base, _)) in options.iter().enumerate() {
            let (rect, planv) = plan(&inp, base);
            let (data, chosen, mcs) = lzw(&planv, rect);
            if best.as_ref().is_none_or(|b| data.len() < b.1.2.len()) {
                best = Some((oi, (rect, chosen, data, mcs)));
            }
        }
        let Some((oi, (rect, chosen, data, mcs))) = best else {
            continue;
        };
        let (disp, base, expand) = options.swap_remove(oi);
        acc_ms += f.delay_ms as u64;
        let target_cs = (acc_ms + 5) / 10;
        let cs = if f.delay_ms == 0 {
            0
        } else {
            target_cs.saturating_sub(emitted_cs).max(2)
        };
        emitted_cs += cs;
        // Getrennte Änderungen (Bildschirmaufnahmen: Zeiger, Eingabezeile, Fortschrittsbalken) als
        // Teilbilder statt eines großen, fast leeren Rechtecks – nur ohne Transparenz im Bild, mit
        // „nicht entsorgen“ und wenn die Teilbilder zusammen deutlich kleiner sind. Die vorderen
        // Teilbilder erscheinen je 20 ms **vor** dem Bildwechsel (die Zeit kommt vom Vorframe), das
        // letzte genau zur Zeit des Frames: Zu jedem Frame-Zeitpunkt ist das Bild vollständig.
        let prev_cs = pending.as_ref().map_or(0, |p| p.delay_cs as usize);
        let max_parts = (prev_cs.saturating_sub(MIN_PART_CS) / MIN_PART_CS + 1).min(MAX_PARTS);
        let parts = if s.split_frames
            && s.crop_changed
            && fi > 0
            && disp == 1
            && !has_alpha
            && cs >= MIN_PART_CS as u64
            && max_parts >= 2
        {
            split_parts(&inp, &base, max_parts, data.len(), &lzw)
        } else {
            None
        };
        if let Some(mut pd) = pending.take() {
            pd.disposal = disp;
            if let Some(p) = &parts {
                pd.delay_cs -= (MIN_PART_CS * (p.len() - 1)) as u16;
            }
            if let Some(er) = expand
                && er != pd.rect
            {
                reencode_expanded(&mut pd, er, &orders);
            }
            done.push(pd);
        }
        canvas = base;
        let paint = |canvas: &mut Vec<u32>, rect: Rect, chosen: &[u8]| {
            for (k, c) in chosen.iter().enumerate() {
                let (x, y) = (rect.x + k % rect.w, rect.y + k / rect.w);
                if Some(*c) != transparent {
                    canvas[y * w + x] = OPAQUE | super::color::pack(pal.rgb[*c as usize]);
                }
            }
        };
        match parts {
            Some(parts) => {
                // vordere Teilbilder je die kürzeste sichere Dauer, das letzte die Dauer des Frames
                let last = parts.len() - 1;
                for (k, (prect, pchosen, pdata, pmcs)) in parts.into_iter().enumerate() {
                    paint(&mut canvas, prect, &pchosen);
                    let d = if k < last { MIN_PART_CS as u64 } else { cs };
                    let e = Encoded {
                        rect: prect,
                        disposal: 1,
                        delay_cs: d.min(65535) as u16,
                        pal_id: pid,
                        transparent,
                        chosen: pchosen,
                        data: pdata,
                        mcs: pmcs,
                    };
                    if k < last {
                        done.push(e);
                    } else {
                        pending = Some(e);
                    }
                }
            }
            None => {
                paint(&mut canvas, rect, &chosen);
                pending = Some(Encoded {
                    rect,
                    disposal: 1,
                    delay_cs: cs.min(65535) as u16,
                    pal_id: pid,
                    transparent,
                    chosen,
                    data,
                    mcs,
                });
            }
        }
    }
    if let Some(mut pd) = pending.take() {
        pd.disposal = if has_alpha && frames.len() > 1 { 2 } else { 1 };
        done.push(pd);
    }
    let (bytes, mut stats) = write_gif(w, h, &orders, s.loop_count, &done, frames.len() > 1);
    stats.photo = photo;
    stats.lossy = lossy;
    (bytes, stats)
}

/// Kürzeste Dauer eines Teilbilds in 1/100 s (kürzere Werte spielen Browser als 1/10 s ab).
pub const MIN_PART_CS: usize = 2;
/// Höchstzahl der Teilbilder je Frame.
pub const MAX_PARTS: usize = 4;
/// Abstand in Pixeln, unter dem Änderungen ein Bereich bleiben (alte und neue Lage eines Objekts).
pub const SPLIT_GAP: usize = 24;

/// Teilbilder eines Frames (Rechteck, gewählte Indizes, LZW-Daten) in Zeichenreihenfolge, wenn sie
/// zusammen mindestens 10 % kleiner sind als das eine Rechteck (`whole` Bytes).
fn split_parts(
    inp: &PlanInput,
    base: &[u32],
    max_parts: usize,
    whole: usize,
    lzw: &dyn Fn(&[PlanPx], Rect) -> LzwOut,
) -> Option<Vec<Part>> {
    let regions = change_regions(inp, base, SPLIT_GAP, max_parts);
    if regions.len() < 2 {
        return None;
    }
    let mut cur = base.to_vec();
    let mut out = Vec::with_capacity(regions.len());
    let mut total = 0usize;
    for r in regions {
        let (rect, planv) = plan_in(inp, &cur, Some(r));
        let (data, chosen, mcs) = lzw(&planv, rect);
        for (k, c) in chosen.iter().enumerate() {
            let (x, y) = (rect.x + k % rect.w, rect.y + k / rect.w);
            if Some(*c) != inp.transparent {
                cur[y * inp.w + x] = OPAQUE | super::color::pack(inp.palette[*c as usize]);
            }
        }
        // Kopf je weiteres Teilbild: Grafiksteuerung 8 + Bildbeschreibung 10 + Codegröße/Ende 2
        total += data.len() + 20 + data.len() / 255;
        out.push((rect, chosen, data, mcs));
    }
    (total * 10 < (whole + 20) * 9).then_some(out)
}

fn clear_rect(buf: &mut [u32], w: usize, r: Rect) {
    for y in r.y..r.y + r.h {
        buf[y * w + r.x..y * w + r.x + r.w]
            .iter_mut()
            .for_each(|p| *p = 0);
    }
}

/// Vergrößert das Rechteck eines schon kodierten Frames (für die Entsorgung „Hintergrund“) und kodiert
/// es mit genau denselben Pixeln neu (neue Fläche transparent).
fn reencode_expanded(pd: &mut Encoded, er: Rect, orders: &[FileOrder]) {
    let t = pd.transparent.unwrap_or(0);
    let mut planv = vec![PlanPx::fixed(t); er.w * er.h];
    for (k, c) in pd.chosen.iter().enumerate() {
        let (x, y) = (pd.rect.x + k % pd.rect.w, pd.rect.y + k / pd.rect.w);
        planv[(y - er.y) * er.w + (x - er.x)] = PlanPx::fixed(*c);
    }
    let (data, chosen, mcs) = orders[pd.pal_id].encode(&planv, er.w, 0.0);
    pd.rect = er;
    pd.data = data;
    pd.chosen = chosen;
    pd.mcs = mcs;
}

/// Visibility weight of a pixel for the perceptual palette (B-5): quantization errors are seen in
/// flat areas and masked by local contrast (edge strength in Oklab, see [`edge_map`]).
pub fn perceptual_weight(edge: f32) -> f32 {
    1.0 / (1.0 + 12.0 * edge)
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
fn padded_table(ord: &FileOrder) -> (Vec<u8>, u8) {
    let entries = ord.rgb.len() + usize::from(ord.transp);
    let mut size = 2usize;
    let mut field = 0u8;
    while size < entries {
        size <<= 1;
        field += 1;
    }
    let mut t = Vec::with_capacity(size * 3);
    if ord.transp {
        t.extend_from_slice(&[0, 0, 0]);
    }
    for c in &ord.rgb {
        t.extend_from_slice(c);
    }
    t.resize(size * 3, 0);
    (t, field)
}

fn write_gif(
    w: usize,
    h: usize,
    orders: &[FileOrder],
    loop_count: i32,
    frames: &[Encoded],
    animated: bool,
) -> (Vec<u8>, GifStats) {
    let mut out =
        Vec::with_capacity(1024 + frames.iter().map(|f| f.data.len() + 32).sum::<usize>());
    out.extend_from_slice(b"GIF89a");
    out.extend_from_slice(&(w.min(65535) as u16).to_le_bytes());
    out.extend_from_slice(&(h.min(65535) as u16).to_le_bytes());
    let (gct, gfield) = padded_table(&orders[0]);
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
    let mut stats = GifStats {
        width: w,
        height: h,
        frames: Vec::new(),
        global_colors: orders[0].rgb.len(),
        bytes: 0,
        ..GifStats::default()
    };
    for f in frames {
        let start = out.len();
        // Grafiksteuerung
        out.extend_from_slice(&[0x21, 0xF9, 0x04]);
        let tflag = u8::from(f.transparent.is_some());
        out.push((f.disposal & 7) << 2 | tflag);
        out.extend_from_slice(&f.delay_cs.to_le_bytes());
        // Transparenz steht in der Datei immer auf Index 0
        out.push(0);
        out.push(0);
        // Bildbeschreibung
        out.push(0x2C);
        for v in [f.rect.x, f.rect.y, f.rect.w, f.rect.h] {
            out.extend_from_slice(&(v.min(65535) as u16).to_le_bytes());
        }
        let local = f.pal_id != 0;
        if local {
            let (lct, lfield) = padded_table(&orders[f.pal_id]);
            out.push(0x80 | lfield);
            out.extend_from_slice(&lct);
        } else {
            out.push(0);
        }
        out.push(f.mcs);
        for chunk in f.data.chunks(255) {
            out.push(chunk.len() as u8);
            out.extend_from_slice(chunk);
        }
        out.push(0);
        stats.frames.push(FrameStat {
            rect: f.rect,
            disposal: f.disposal,
            local_palette: local,
            delay_cs: f.delay_cs,
            bytes: out.len() - start,
            min_code_size: f.mcs,
        });
    }
    out.push(0x3B);
    stats.bytes = out.len();
    (out, stats)
}
