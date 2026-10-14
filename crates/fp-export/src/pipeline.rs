//! Optimieren-Pipeline (08 §2.1, `@0074fbf0`): Palette bauen → Palettenflags → Umsetzen/Dithern →
//! Verlust → unbenutzte Farben → bearbeitete Farben → Sortierung. Reine Funktion, damit die Vorschau,
//! „Optimieren auf Größe“ und der Export sie beliebig oft aufrufen können.

use fp_raster::RgbaImage;

use crate::options::{ColorMode, Dither, ExportFormat, OptimizeSettings, PaletteKind, Transparency};
use crate::palettes::{palette_exact, palette_gray, palette_grid, palette_mac, palette_web216, palette_windows};
use crate::px::{self, a, dist2, from_rgba, matte_blend};
use crate::quant::{Quantizer, TRANSPARENCY_INDEX, TRANSPARENCY_INDEX_ALPHA, TRANSPARENCY_NONE, TRANSPARENCY_RGBA};
use crate::rgba::{apply_edited, quantize_rgba, remap_rgba, remove_unused, sort_palette, PaletteSlot};

/// Laufzeit-Umgebung des Exports (Voreinstellungen und Dokumentwerte).
#[derive(Clone, Debug, PartialEq)]
pub struct ExportContext {
    /// Leinwandfarbe der Seite (`0xAARRGGBB`, Alpha 0 = transparent).
    pub canvas: u32,
    /// Auflösung in Pixel je Zoll (Einheit wie `RSUi`: 0 Zoll, 1 cm; gespeichert wird immer je Zoll).
    pub resolution_dpi: f32,
    /// Auflösung in Pixel je cm, wenn die Seite in cm misst (`pHYs` = trunc(px/cm)·100, BA-124).
    pub resolution_per_cm: Option<f32>,
    /// Voreinstellung `LZWSmoothingAmount` (Vorgabe 7).
    pub lzw_smoothing: i32,
    /// Voreinstellung `BlackAndWhiteForDirector` (Vorgabe aus).
    pub black_and_white_for_director: bool,
    /// Voreinstellung `ExportPngWithAdam7Interlacing`.
    pub png_adam7: bool,
    /// Software-Text für PNG/TIFF (`None` = nicht schreiben).
    pub software: Option<String>,
    /// Erstellungszeit als Text (PNG „Creation Time“), `None` = nicht schreiben.
    pub creation_time: Option<String>,
}

impl Default for ExportContext {
    fn default() -> Self {
        ExportContext {
            canvas: 0xFFFF_FFFF,
            resolution_dpi: 72.0,
            resolution_per_cm: None,
            lzw_smoothing: 7,
            black_and_white_for_director: false,
            png_adam7: false,
            software: Some("Firepixel".to_string()),
            creation_time: None,
        }
    }
}

/// Indiziertes Ergebnis (GIF/PNG8/BMP8/TIFF8/WBMP).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct IndexedImage {
    pub width: usize,
    pub height: usize,
    /// Palette `0xAARRGGBB` (Alpha < 255 nur bei transparenten Einträgen bzw. RGBA-Palette).
    pub palette: Vec<u32>,
    /// Indizes zeilenweise.
    pub indices: Vec<u8>,
    /// Transparenzindex (Index-Alpha: hinter der Palette; Indextransparenz: erster Eintrag mit Alpha 0).
    pub transparent_index: Option<usize>,
    /// Pixelzähler je Eintrag.
    pub counts: Vec<u64>,
    /// Hat jeder Eintrag einen eigenen Alphawert (PNG `tRNS` vollständig)?
    pub alpha_palette: bool,
}

impl IndexedImage {
    /// Farbe eines Index (hinter der Palette: Transparenz).
    pub fn color(&self, i: usize) -> u32 {
        self.palette.get(i).copied().unwrap_or(0)
    }

    /// Palette als RGB-Tripel inkl. Transparenzeintrag (für GIF/PNG-PLTE).
    pub fn palette_rgb(&self, matte: u32) -> Vec<[u8; 3]> {
        let mut v: Vec<[u8; 3]> = self.palette.iter().map(|&c| [px::r(c) as u8, px::g(c) as u8, px::b(c) as u8]).collect();
        if let Some(t) = self.transparent_index
            && t >= v.len()
        {
            v.resize(t, [0, 0, 0]);
            v.push([px::r(matte) as u8, px::g(matte) as u8, px::b(matte) as u8]);
        }
        v
    }

    /// Anzahl Paletteneinträge in der Datei (inkl. Transparenzeintrag hinter der Palette).
    pub fn file_palette_len(&self) -> usize {
        match self.transparent_index {
            Some(t) if t >= self.palette.len() => t + 1,
            _ => self.palette.len(),
        }
    }

    /// Vorschau: Indexbild über die Palette aufgelöst (transparente Einträge Alpha 0).
    pub fn to_rgba(&self, origin: (i32, i32)) -> RgbaImage {
        let mut data = Vec::with_capacity(self.indices.len());
        for &i in &self.indices {
            let i = i as usize;
            let c = if Some(i) == self.transparent_index || i >= self.palette.len() { 0 } else { self.palette[i] };
            data.push(px::to_rgba(c));
        }
        let rect = fp_geom::IRect::new(origin.0, origin.1, origin.0 + self.width as i32, origin.1 + self.height as i32);
        RgbaImage::from_raw(rect, data).unwrap_or_else(|_| RgbaImage::empty_at(origin.0, origin.1))
    }
}

/// Ergebnis der Optimierung.
#[derive(Clone, Debug, PartialEq)]
pub enum Optimized {
    /// 8 Bit indiziert.
    Indexed(IndexedImage),
    /// 24/32 Bit (Pixel `0xAARRGGBB`; bei 24 Bit über die Matte gemischt, Alpha 255).
    Truecolor { width: usize, height: usize, pixels: Vec<u32>, alpha: bool },
}

impl Optimized {
    pub fn width(&self) -> usize {
        match self {
            Optimized::Indexed(i) => i.width,
            Optimized::Truecolor { width, .. } => *width,
        }
    }
    pub fn height(&self) -> usize {
        match self {
            Optimized::Indexed(i) => i.height,
            Optimized::Truecolor { height, .. } => *height,
        }
    }
}

/// Pixel eines Bildes als `0xAARRGGBB`-Vektor.
pub fn image_pixels(img: &RgbaImage) -> Vec<u32> {
    img.data().iter().map(|&p| from_rgba(p)).collect()
}

/// Wirksame Farbtiefe für ein Format (JPEG ⇒ 24 Bit, WBMP ⇒ indiziert s/w, 32 Bit mit deckender Matte ⇒ 24 Bit).
pub fn effective_color_mode(s: &OptimizeSettings) -> ColorMode {
    match s.format {
        ExportFormat::Jpeg => ColorMode::Rgb24,
        ExportFormat::Wbmp => ColorMode::Indexed,
        ExportFormat::Avif | ExportFormat::WebP | ExportFormat::WebPLossless | ExportFormat::AnimatedWebP | ExportFormat::Qoi | ExportFormat::Ico | ExportFormat::Apng => {
            if s.matte.is_some_and(|m| a(m) == 255) { ColorMode::Rgb24 } else { ColorMode::Rgba32 }
        }
        ExportFormat::Gif | ExportFormat::AnimatedGif => ColorMode::Indexed,
        _ => {
            if s.color_mode == ColorMode::Rgba32 && s.matte.is_some_and(|m| a(m) == 255) {
                ColorMode::Rgb24
            } else {
                s.color_mode
            }
        }
    }
}

/// Optimiert ein Bild nach den Einstellungen (ohne Kodierung).
pub fn optimize(img: &RgbaImage, s: &OptimizeSettings, ctx: &ExportContext) -> Optimized {
    let (w, h) = (img.width(), img.height());
    let pixels = image_pixels(img);
    optimize_pixels(&pixels, w, h, s, ctx)
}

/// Wie [`optimize`], aber auf `0xAARRGGBB`-Pixeln.
pub fn optimize_pixels(pixels: &[u32], w: usize, h: usize, s: &OptimizeSettings, ctx: &ExportContext) -> Optimized {
    let matte = s.effective_matte(ctx.canvas);
    match effective_color_mode(s) {
        ColorMode::Indexed => Optimized::Indexed(quantize_indexed(pixels, w, h, s, ctx)),
        ColorMode::Rgb24 => {
            let px = pixels.iter().map(|&c| if a(c) == 255 { c } else { matte_blend(c, matte, false) }).collect();
            Optimized::Truecolor { width: w, height: h, pixels: px, alpha: false }
        }
        ColorMode::Rgba32 => Optimized::Truecolor { width: w, height: h, pixels: pixels.to_vec(), alpha: true },
    }
}

/// Wirksamer Transparenzmodus für ein Format.
pub fn effective_transparency(s: &OptimizeSettings) -> i32 {
    let t = s.transparency.code();
    match s.format {
        // PNG8: „Index-Alpha“ und „RGBA“ sind gleichwertig (Laufzeit 08 T2: byte-identisch).
        ExportFormat::Png if t == TRANSPARENCY_INDEX_ALPHA => TRANSPARENCY_RGBA,
        // GIF kann keine Alphapalette: RGBA wie Index-Alpha (A).
        ExportFormat::Gif | ExportFormat::AnimatedGif if t == TRANSPARENCY_RGBA => TRANSPARENCY_INDEX_ALPHA,
        ExportFormat::Wbmp | ExportFormat::Bmp => TRANSPARENCY_NONE,
        _ => t,
    }
}

/// Ergebnis der Palettenwahl ([uild_quantizer], @0074da40).
#[derive(Clone, Debug)]
pub struct BuiltPalette {
    /// Quantisierer mit Palette und Suchstrukturen.
    pub q: Quantizer,
    /// Adaptive RGB-Palette (Zähler = Box-Gewichte; „unbenutzte Farben entfernen“ greift nicht).
    pub adaptive_counts: bool,
    /// RGBA-Palette (PNG8 mit Alpha).
    pub rgba: bool,
    /// Web-Snap der Quelle beim Umsetzen (adaptiv + Web-adaptiv).
    pub snap: bool,
    /// Wirksamer Dithermodus und -prozent.
    pub dither_mode: i32,
    pub dither_percent: i32,
    /// Wirksame Farbzahl (nach Sonderfällen).
    pub max_colors: i32,
}

/// Palettenwahl nach @0074da40 (wie w_palette.build): feste Paletten, Exakt, adaptiv bzw. RGBA.
pub fn build_quantizer(pixels: &[u32], w: usize, s: &OptimizeSettings, ctx: &ExportContext) -> BuiltPalette {
    let matte = s.effective_matte(ctx.canvas);
    let mut tm = effective_transparency(s);
    let (mut dmode, dpct) = s.dither_mode();
    if dpct == 0 {
        dmode = 0;
    }
    let mut kind = s.palette_kind;
    let mut max_colors = s.max_colors;
    if s.format == ExportFormat::Wbmp {
        kind = PaletteKind::Grayscale;
        max_colors = 2;
    }
    if kind == PaletteKind::Custom && s.palette.len() < 2 {
        kind = PaletteKind::Adaptive;
        max_colors = 128;
    }
    let snap = kind == PaletteKind::Adaptive && s.web_adaptive;
    let canvas_special = if s.matte.is_some() { None } else { Some(ctx.canvas) };
    let mut locked: Vec<u32> = s.palette.iter().filter(|p| p.locked).map(|p| p.color | 0xFF00_0000).collect();
    let png = s.format == ExportFormat::Png;
    if tm == TRANSPARENCY_RGBA && !png {
        tm = TRANSPARENCY_INDEX_ALPHA;
    }
    let mut q = Quantizer::new(tm, matte);
    let mut rgba_palette: Option<Vec<u32>> = None;
    let mut adaptive_counts = false;
    match kind {
        PaletteKind::Custom => q.set_palette(s.palette.iter().map(|p| p.color | 0xFF00_0000).collect(), 0, false, true),
        PaletteKind::Uniform => q.set_palette(palette_grid(s.grid_levels), s.grid_levels.clamp(2, 6) as u32, false, false),
        PaletteKind::Web216 => q.set_palette(palette_web216(), 6, false, false),
        PaletteKind::Grayscale => {
            let n = max_colors - if tm == TRANSPARENCY_INDEX_ALPHA { 1 } else { 0 };
            q.set_palette(palette_gray(n), 0, true, false);
        }
        PaletteKind::Mac | PaletteKind::Windows => {
            q.set_palette(if kind == PaletteKind::Mac { palette_mac() } else { palette_windows() }, 0, false, false);
            if q.tmode == TRANSPARENCY_INDEX_ALPHA {
                q.tmode = TRANSPARENCY_INDEX;
                q.transparent_index = None;
            }
        }
        PaletteKind::Adaptive | PaletteKind::Exact => {
            let ex = if kind == PaletteKind::Exact { palette_exact(pixels, matte) } else { None };
            if let Some(ex) = ex {
                q.set_palette(ex, 0, false, false);
            } else {
                let n = if kind == PaletteKind::Adaptive { max_colors } else { 256 };
                if tm == TRANSPARENCY_RGBA {
                    let (_rq, rpal) = quantize_rgba(pixels, n, 15, snap, s.websnap_tolerance, &locked, w);
                    q.set_palette(rpal.clone(), 0, false, false);
                    q.tmode = TRANSPARENCY_RGBA;
                    rgba_palette = Some(rpal);
                } else {
                    let temp_bw = (!s.web_adaptive && s.format.is_plugin_format()) || ctx.black_and_white_for_director;
                    let user_locked = locked.len();
                    if temp_bw {
                        locked.push(0xFFFF_FFFF);
                        locked.push(0xFF00_0000);
                    }
                    q.build_adaptive(pixels, n, snap, s.websnap_tolerance, &locked, canvas_special, w);
                    if temp_bw {
                        // Schwarz/Weiß nur vorübergehend gesperrt (08 §2.2).
                        let mut seen = 0;
                        for (i, l) in q.locked.iter_mut().enumerate() {
                            if *l {
                                seen += 1;
                                if seen > user_locked && (q.palette[i] == 0xFFFF_FFFF || q.palette[i] == 0xFF00_0000) {
                                    *l = false;
                                }
                            }
                        }
                    }
                    adaptive_counts = true;
                }
            }
        }
    }
    if q.custom && q.palette.iter().all(|&c| a(c) == 0 || (px::r(c) == px::g(c) && px::g(c) == px::b(c))) {
        q.gray = true;
    }
    BuiltPalette { q, adaptive_counts, rgba: rgba_palette.is_some(), snap, dither_mode: dmode, dither_percent: dpct, max_colors }
}

/// Umsetzen/Dithern nach @0074e280 (wie w_palette.remap, Glättung mit LZWSmoothingAmount).
pub fn remap_pixels(b: &mut BuiltPalette, pixels: &[u32], w: usize, h: usize, s: &OptimizeSettings, ctx: &ExportContext) -> Vec<u8> {
    let (dmode, dpct, snap) = (b.dither_mode, b.dither_percent, b.snap);
    let q = &mut b.q;
    if q.tmode == TRANSPARENCY_RGBA {
        return remap_rgba(&q.palette, pixels, w, h, dmode == 1, dpct, snap, s.websnap_tolerance);
    }
    match dmode {
        1 => q.remap_diffusion(pixels, w, h, dpct, snap, s.websnap_tolerance),
        2 => q.remap_2x2(pixels, w, h),
        _ if s.format == ExportFormat::AnimatedGif => q.remap_plain(pixels, w, h),
        _ => q.remap_none(pixels, w, h, s.optimized, ctx.lzw_smoothing, s.lossy),
    }
}

/// Indizierte Optimierung (Palette, Flags, Umsetzen, Nachbearbeitung).
pub fn quantize_indexed(pixels: &[u32], w: usize, h: usize, s: &OptimizeSettings, ctx: &ExportContext) -> IndexedImage {
    let matte = s.effective_matte(ctx.canvas);
    let mut built = build_quantizer(pixels, w, s, ctx);
    let tm = built.q.tmode;
    let max_colors = built.max_colors;
    let adaptive_counts = built.adaptive_counts;
    let rgba_palette = built.rgba;

    // Palettenflags in den Quantisierer (08 §2.6, `@00748690`).
    let q = &mut built.q;
    let mut edited: Vec<Option<u32>> = vec![None; q.palette.len()];
    if tm != TRANSPARENCY_RGBA {
        let mut appended = false;
        let mut any_marked = false;
        let mut matched = false;
        for p in &s.palette {
            if p.transparent && tm != TRANSPARENCY_NONE {
                any_marked = true;
                if let Some(k) = nearest_exhaustive(&q.palette, p.color)
                    && dist2(q.palette[k], p.color) <= 625
                {
                    q.palette[k] &= 0x00FF_FFFF;
                    matched = true;
                }
            }
            if let Some(nc) = p.edited
                && let Some(k) = q.palette.iter().position(|&c| (c | 0xFF00_0000) == (p.color | 0xFF00_0000))
            {
                edited[k] = Some(nc);
            }
        }
        if tm == TRANSPARENCY_INDEX && !matched {
            let target = if any_marked {
                s.palette.iter().find(|p| p.transparent).map(|p| p.color)
            } else {
                Some(matte)
            };
            if let Some(t) = target {
                match nearest_exhaustive(&q.palette, t) {
                    Some(k) if dist2(q.palette[k], t) <= 625 && !any_marked => q.palette[k] &= 0x00FF_FFFF,
                    _ if q.palette.len() < 256 => {
                        q.palette.push(t & 0x00FF_FFFF);
                        q.locked.push(false);
                        q.counts.push(0);
                        edited.push(None);
                        appended = true;
                    }
                    _ => {}
                }
            }
        }
        if q.palette.len() != edited.len() {
            edited.resize(q.palette.len(), None);
        }
        // Nur angehängte Einträge erzwingen neue Kandidatenlisten; Alpha-Markierungen ändern die
        // RGB-Suche nicht (die Zellen der adaptiven Palette bleiben die vor dem Feinabgleich).
        if appended {
            q.replace_palette_keep(q.palette.clone());
        }
    }

    // Umsetzen (08 §3, `@0074e280`).
    let animated = s.format == ExportFormat::AnimatedGif;
    let mut indices = remap_pixels(&mut built, pixels, w, h, s, ctx);
    let is_gif = matches!(s.format, ExportFormat::Gif | ExportFormat::AnimatedGif);
    if is_gif && s.lossy > 0 && !animated {
        let pal: Vec<[u8; 3]> = built.q.palette.iter().map(|&c| [px::r(c) as u8, px::g(c) as u8, px::b(c) as u8]).collect();
        indices = crate::gif::lossy_simulate(&indices, &pal, s.lossy);
    }

    // Nachbearbeitung: Zähler, unbenutzte Farben, bearbeitete Farben, Sortierung.
    let plen = built.q.palette.len();
    let mut counts = vec![0u64; plen];
    for &i in &indices {
        if let Some(c) = counts.get_mut(i as usize) {
            *c += 1;
        }
    }
    let mut slots: Vec<PaletteSlot> = (0..plen)
        .map(|i| PaletteSlot {
            color: built.q.palette[i],
            // A: Das Original kopiert bei adaptiven Paletten die Box-Gewichte des Quantisierers (08 §1.2 +4).
            count: if adaptive_counts { built.q.counts.get(i).copied().unwrap_or(0) } else { counts[i] },
            locked: built.q.locked.get(i).copied().unwrap_or(false),
            edited: edited.get(i).copied().flatten(),
            transparent: a(built.q.palette[i]) == 0,
            origin: i,
        })
        .collect();
    let remove = s.optimized && !animated && (!adaptive_counts || rgba_palette);
    if remove {
        let (kept, truncated) = remove_unused(&slots, max_colors, built.q.tmode, 0);
        if kept.len() != slots.len() || truncated {
            if truncated {
                built.q.replace_palette_keep(kept.iter().map(|e| e.color).collect());
                built.q.locked = kept.iter().map(|e| e.locked).collect();
                indices = remap_pixels(&mut built, pixels, w, h, s, ctx);
                slots = kept.iter().enumerate().map(|(i, e)| PaletteSlot { origin: i, ..e.clone() }).collect();
                for sl in slots.iter_mut() {
                    sl.count = 0;
                }
                for &i in &indices {
                    if let Some(sl) = slots.get_mut(i as usize) {
                        sl.count += 1;
                    }
                }
            } else {
                let mut map = vec![0u8; plen + 1];
                for (oi, m) in map.iter_mut().enumerate().take(plen) {
                    let c = built.q.palette[oi];
                    *m = kept
                        .iter()
                        .position(|e| e.origin == oi)
                        .or_else(|| kept.iter().position(|e| e.color == c))
                        .unwrap_or(0) as u8;
                }
                let tidx = built.q.transparent_index;
                for i in indices.iter_mut() {
                    let iu = *i as usize;
                    if Some(iu) == tidx && iu >= plen {
                        *i = kept.len().min(255) as u8;
                    } else if iu < map.len() {
                        *i = map[iu];
                    }
                }
                slots = kept.iter().enumerate().map(|(i, e)| PaletteSlot { origin: i, ..e.clone() }).collect();
                if built.q.tmode == TRANSPARENCY_INDEX_ALPHA {
                    built.q.transparent_index = Some(slots.len());
                }
            }
        }
    }
    apply_edited(&mut slots);
    let tmode = built.q.tmode;
    let sort_mode = if tmode == TRANSPARENCY_RGBA { 0 } else { s.sorting.code() };
    if sort_mode != 0 {
        let order: Vec<PaletteSlot> = slots.iter().enumerate().map(|(i, e)| PaletteSlot { origin: i, ..e.clone() }).collect();
        let sorted = sort_palette(&order, sort_mode, tmode);
        let mut map = vec![0u8; sorted.len().max(1)];
        for (ni, e) in sorted.iter().enumerate() {
            map[e.origin] = ni as u8;
        }
        for i in indices.iter_mut() {
            if (*i as usize) < map.len() && (*i as usize) < sorted.len() {
                *i = map[*i as usize];
            }
        }
        slots = sorted;
    }
    let palette: Vec<u32> = slots.iter().map(|e| e.color).collect();
    let transparent_index = match tmode {
        TRANSPARENCY_INDEX_ALPHA => Some(palette.len()),
        TRANSPARENCY_INDEX => palette.iter().position(|&c| a(c) == 0),
        _ => None,
    };
    IndexedImage {
        width: w,
        height: h,
        counts: slots.iter().map(|e| e.count).collect(),
        palette,
        indices,
        transparent_index,
        alpha_palette: tmode == TRANSPARENCY_RGBA,
    }
}
fn nearest_exhaustive(pal: &[u32], c: u32) -> Option<usize> {
    let mut best = u32::MAX;
    let mut bi = None;
    for (i, &p) in pal.iter().enumerate() {
        let d = dist2(p, c);
        if d < best {
            best = d;
            bi = Some(i);
        }
    }
    bi
}

impl Quantizer {
    /// Palette ersetzen und dabei die Schnellsuchen behalten (Flags geändert, Farben gleich).
    pub fn replace_palette_keep(&mut self, pal: Vec<u32>) {
        let (gl, gray, custom, tm, ti) = (self.grid_levels, self.gray, self.custom, self.tmode, self.transparent_index);
        let same_colors = pal.len() == self.palette.len() && pal.iter().zip(&self.palette).all(|(x, y)| (x | 0xFF00_0000) == (y | 0xFF00_0000));
        self.replace_palette(pal);
        if same_colors {
            self.grid_levels = gl;
            self.gray = gray;
        }
        self.custom = custom;
        self.tmode = tm;
        if tm != TRANSPARENCY_INDEX_ALPHA {
            self.transparent_index = ti.filter(|&t| t < self.palette.len());
        }
    }
}

/// Fertiges Ergebnis eines Bild-Exports.
#[derive(Clone, Debug, PartialEq)]
pub struct ExportResult {
    /// Dateiinhalt.
    pub bytes: Vec<u8>,
    /// Optimiertes Bild (Indexbild bzw. Pixel vor der Kodierung).
    pub optimized: Optimized,
    /// Anzahl Farben (indiziert) bzw. 0.
    pub colors: usize,
}

/// Prüft, ob ein Dither gesetzt ist (Hilfe für die Oberfläche).
pub fn has_dither(s: &OptimizeSettings) -> bool {
    !matches!(s.dither, Dither::None) && s.transparency != Transparency::Rgba
}
