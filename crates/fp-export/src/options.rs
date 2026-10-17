//! Optimieren-Einstellungen als typisierte Sicht auf `fp_model::ExportSettings` (`EXPv`, 08 §1) und
//! Vorgaben (08 §1.6), erweitert um moderne Formate (WebP, AVIF, APNG, animiertes WebP).
//!
//! # Abbildung auf das Dateiformat
//!
//! Das Original kennt die Formate 0 GIF, 1 JPEG, 2 PNG, 3 „custom“ (Plug-in-Format, Typ über
//! `MCTi`) und 4 GIF-Animation. TIFF/BMP/WBMP/PICT stehen wie beim Original als `FMTi = 3` mit
//! `MCTi` = Mac-Typ und `MCCi = 'MKBY'`. Die neuen Formate werden ebenfalls als `FMTi = 3`
//! gespeichert, mit eigenem Typ (`WEBP`, `WEBL` verlustfrei, `WBPA` animiert, `AVIF`, `APNG`, `QOI `,
//! `ICO `) und Creator `'FPXL'`; Qualität steht in `JPQi`. Ein echtes Original liest solche
//! Dateien ohne Fehler, kennt das Plug-in aber nicht (A).
//!
//! `sorting` (Laufzeitfeld, nicht in der Datei) und die Matte (`EXPv/FCLi`, Seiteneinstellung) sind
//! eigene Felder von [`OptimizeSettings`].

use fp_model::{ExportSettings, FrameInfo, PaletteEntry};

/// Vierzeichen-Code als `u32` (Big Endian wie Mac-Typen).
pub const fn fourcc(s: &[u8; 4]) -> u32 {
    u32::from_be_bytes(*s)
}

/// Creator der Original-Plug-in-Formate.
pub const CREATOR_ORIGINAL: u32 = fourcc(b"MKBY");
/// Creator der Firepixel-Erweiterungen.
pub const CREATOR_FIREPIXEL: u32 = fourcc(b"FPXL");

/// Exportformat.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExportFormat {
    Gif,
    AnimatedGif,
    Jpeg,
    /// PNG (8/24/32 Bit über [`ColorMode`]).
    Png,
    Tiff,
    Bmp,
    Wbmp,
    /// PICT (nur als Einstellung erhalten; Schreiben nicht unterstützt).
    Pict,
    /// WebP verlustbehaftet.
    WebP,
    /// WebP verlustfrei.
    WebPLossless,
    /// Animiertes WebP (verlustbehaftet oder verlustfrei nach [`WebpSettings::lossless`]).
    AnimatedWebP,
    Avif,
    /// Animiertes PNG.
    Apng,
    Qoi,
    /// Windows-Symbol (mehrere Größen, siehe [`crate::formats::IcoOptions`]).
    Ico,
    /// Unbekanntes Plug-in-Format (Mac-Typ bleibt erhalten).
    Custom(u32),
}

impl ExportFormat {
    /// Übliche Dateiendung (ohne Punkt).
    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Gif | ExportFormat::AnimatedGif => "gif",
            ExportFormat::Jpeg => "jpg",
            ExportFormat::Png | ExportFormat::Apng => "png",
            ExportFormat::Tiff => "tif",
            ExportFormat::Bmp => "bmp",
            ExportFormat::Wbmp => "wbmp",
            ExportFormat::Pict => "pct",
            ExportFormat::WebP | ExportFormat::WebPLossless | ExportFormat::AnimatedWebP => "webp",
            ExportFormat::Avif => "avif",
            ExportFormat::Qoi => "qoi",
            ExportFormat::Ico => "ico",
            ExportFormat::Custom(_) => "bin",
        }
    }

    /// Anzeigename (deutsch, wie im Optimieren-Bedienfeld).
    pub fn display_name(self) -> &'static str {
        match self {
            ExportFormat::Gif => "GIF",
            ExportFormat::AnimatedGif => "Animiertes GIF",
            ExportFormat::Jpeg => "JPEG",
            ExportFormat::Png => "PNG",
            ExportFormat::Tiff => "TIFF",
            ExportFormat::Bmp => "BMP",
            ExportFormat::Wbmp => "WBMP",
            ExportFormat::Pict => "PICT",
            ExportFormat::WebP => "WebP",
            ExportFormat::WebPLossless => "WebP verlustfrei",
            ExportFormat::AnimatedWebP => "Animiertes WebP",
            ExportFormat::Avif => "AVIF",
            ExportFormat::Apng => "APNG",
            ExportFormat::Qoi => "QOI",
            ExportFormat::Ico => "ICO",
            ExportFormat::Custom(_) => "Benutzerdefiniert",
        }
    }

    /// Ein „custom“-Format im Sinne des Originals (`FMTi = 3`)?
    pub fn is_plugin_format(self) -> bool {
        !matches!(
            self,
            ExportFormat::Gif | ExportFormat::AnimatedGif | ExportFormat::Jpeg | ExportFormat::Png
        )
    }

    /// Animiertes Format?
    pub fn is_animated(self) -> bool {
        matches!(
            self,
            ExportFormat::AnimatedGif | ExportFormat::AnimatedWebP | ExportFormat::Apng
        )
    }

    /// Alle Formate in der Reihenfolge der Formatliste der Oberfläche.
    pub const ALL: [ExportFormat; 15] = [
        ExportFormat::Gif,
        ExportFormat::AnimatedGif,
        ExportFormat::Jpeg,
        ExportFormat::Png,
        ExportFormat::Tiff,
        ExportFormat::Bmp,
        ExportFormat::Wbmp,
        ExportFormat::Pict,
        ExportFormat::WebP,
        ExportFormat::WebPLossless,
        ExportFormat::AnimatedWebP,
        ExportFormat::Avif,
        ExportFormat::Apng,
        ExportFormat::Qoi,
        ExportFormat::Ico,
    ];

    fn mac(self) -> Option<(u32, u32)> {
        Some(match self {
            ExportFormat::Tiff => (fourcc(b"TIFF"), CREATOR_ORIGINAL),
            ExportFormat::Bmp => (fourcc(b"BMP "), CREATOR_ORIGINAL),
            ExportFormat::Wbmp => (fourcc(b"WBMP"), CREATOR_ORIGINAL),
            ExportFormat::Pict => (fourcc(b"PICT"), CREATOR_ORIGINAL),
            ExportFormat::WebP => (fourcc(b"WEBP"), CREATOR_FIREPIXEL),
            ExportFormat::WebPLossless => (fourcc(b"WEBL"), CREATOR_FIREPIXEL),
            ExportFormat::AnimatedWebP => (fourcc(b"WBPA"), CREATOR_FIREPIXEL),
            ExportFormat::Avif => (fourcc(b"AVIF"), CREATOR_FIREPIXEL),
            ExportFormat::Apng => (fourcc(b"APNG"), CREATOR_FIREPIXEL),
            ExportFormat::Qoi => (fourcc(b"QOI "), CREATOR_FIREPIXEL),
            ExportFormat::Ico => (fourcc(b"ICO "), CREATOR_FIREPIXEL),
            ExportFormat::Custom(t) => (t, 0),
            _ => return None,
        })
    }

    fn from_mac(t: u32) -> ExportFormat {
        match &t.to_be_bytes() {
            b"TIFF" => ExportFormat::Tiff,
            b"BMP " | b"BMPf" => ExportFormat::Bmp,
            b"WBMP" => ExportFormat::Wbmp,
            b"PICT" => ExportFormat::Pict,
            b"WEBP" => ExportFormat::WebP,
            b"WEBL" => ExportFormat::WebPLossless,
            b"WBPA" => ExportFormat::AnimatedWebP,
            b"AVIF" => ExportFormat::Avif,
            b"APNG" => ExportFormat::Apng,
            b"QOI " => ExportFormat::Qoi,
            b"ICO " => ExportFormat::Ico,
            _ => ExportFormat::Custom(t),
        }
    }
}

/// Farbtiefe (`colorMode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColorMode {
    /// 8 Bit indiziert.
    Indexed,
    /// 24 Bit.
    Rgb24,
    /// 32 Bit mit Alpha.
    Rgba32,
}

/// Palettenart (`paletteMode`, 08 §1.4/§2.2). „Web-adaptiv“ = [`PaletteKind::Adaptive`] mit
/// [`OptimizeSettings::web_adaptive`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PaletteKind {
    /// 0 Benutzerdefiniert (Farben aus [`OptimizeSettings::palette`], z. B. aus einer ACT-Datei).
    Custom,
    /// 1 Adaptiv.
    Adaptive,
    /// 2 Einheitlich (Raster mit [`OptimizeSettings::grid_levels`] Stufen).
    Uniform,
    /// 3 Graustufen mit [`OptimizeSettings::max_colors`] Stufen (2 = Schwarzweiß).
    Grayscale,
    /// 4 Macintosh.
    Mac,
    /// 5 Windows.
    Windows,
    /// 6 Exakt.
    Exact,
    /// 7 Web 216.
    Web216,
}

impl PaletteKind {
    pub fn from_code(v: i32) -> PaletteKind {
        match v {
            0 => PaletteKind::Custom,
            2 => PaletteKind::Uniform,
            3 => PaletteKind::Grayscale,
            4 => PaletteKind::Mac,
            5 => PaletteKind::Windows,
            6 => PaletteKind::Exact,
            7 => PaletteKind::Web216,
            _ => PaletteKind::Adaptive,
        }
    }
    pub fn code(self) -> i32 {
        match self {
            PaletteKind::Custom => 0,
            PaletteKind::Adaptive => 1,
            PaletteKind::Uniform => 2,
            PaletteKind::Grayscale => 3,
            PaletteKind::Mac => 4,
            PaletteKind::Windows => 5,
            PaletteKind::Exact => 6,
            PaletteKind::Web216 => 7,
        }
    }
    /// Anzeigename (deutsch, `resources_de.json` 10833–10842); `web` = Web-adaptiv.
    pub fn display_name(self, web: bool) -> &'static str {
        match self {
            PaletteKind::Custom => "Benutzerdefiniert",
            PaletteKind::Adaptive if web => "WebSnap adaptiv",
            PaletteKind::Adaptive => "Adaptiv",
            PaletteKind::Uniform => "Einheitlich",
            PaletteKind::Grayscale => "Graustufen",
            PaletteKind::Mac => "Macintosh",
            PaletteKind::Windows => "Windows",
            PaletteKind::Exact => "Exakt",
            PaletteKind::Web216 => "Web 216",
        }
    }
}

/// Dithering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Dither {
    None,
    /// Original-Fehlerdiffusion mit Prozentwert (10…100 wirksam).
    Diffusion(i32),
    /// 2×2-Muster.
    Pattern2x2,
}

/// Transparenz (`paletteTransparency`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Transparency {
    None,
    /// Indextransparenz.
    Index,
    /// Alpha-Transparenz bei indizierten Bildern (Index-Alpha; bei PNG8 = RGBA-Palette).
    IndexAlpha,
    /// RGBA-Palette (PNG8).
    Rgba,
}

impl Transparency {
    pub fn code(self) -> i32 {
        match self {
            Transparency::None => 0,
            Transparency::Index => 1,
            Transparency::IndexAlpha => 2,
            Transparency::Rgba => 3,
        }
    }
    pub fn from_code(v: i32) -> Transparency {
        match v {
            1 => Transparency::Index,
            2 => Transparency::IndexAlpha,
            3 => Transparency::Rgba,
            _ => Transparency::None,
        }
    }
}

/// Exportsortierung der Palette (Laufzeitfeld `sorting`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Sorting {
    #[default]
    None,
    Luminance,
    Popularity,
}

impl Sorting {
    pub fn code(self) -> i32 {
        match self {
            Sorting::None => 0,
            Sorting::Luminance => 1,
            Sorting::Popularity => 2,
        }
    }
}

/// Paletteneintrag mit Flags (Optimieren-Bedienfeld: sperren, transparent, bearbeiten).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PaletteColor {
    /// Farbe `0xAARRGGBB`.
    pub color: u32,
    pub locked: bool,
    pub transparent: bool,
    /// Bearbeitet: neue Farbe.
    pub edited: Option<u32>,
}

impl PaletteColor {
    pub fn new(color: u32) -> PaletteColor {
        PaletteColor {
            color,
            locked: false,
            transparent: false,
            edited: None,
        }
    }
}

/// Selektives JPEG (08 §4.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SelectiveJpeg {
    /// Aktiv (`JSEb`).
    pub enabled: bool,
    /// Qualität der Maske (`JSQi`).
    pub quality: i32,
    /// Text beibehalten (`JSTb`), Schaltflächen beibehalten (`JSBb`).
    pub preserve_text: bool,
    pub preserve_buttons: bool,
}

/// JPEG-Einstellungen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct JpegSettings {
    /// Qualität 1…100 (`JPQi`).
    pub quality: i32,
    /// Glättung 0…7 (`JPSi`, IJG `smoothing_factor = 16·s`).
    pub smoothing: i32,
    /// `jpegSubsampling` (`JPOi`): 0 = „Schärfere Farbkanten“ (4:4:4), sonst 4:2:0 bis Qualität 85.
    pub subsampling: i32,
    /// Progressiv (`JPPb`).
    pub progressive: bool,
    pub selective: SelectiveJpeg,
}

/// WebP-Einstellungen.
#[derive(Clone, Copy, Debug, PartialEq, Hash)]
pub struct WebpSettings {
    /// Verlustfrei (animiertes WebP).
    pub lossless: bool,
    /// Qualität 0…100 (verlustbehaftet) bzw. Aufwand (verlustfrei).
    pub quality: i32,
}

/// AVIF-Einstellungen.
#[derive(Clone, Copy, Debug, PartialEq, Hash)]
pub struct AvifSettings {
    /// Qualität 1…100.
    pub quality: i32,
    /// Geschwindigkeit 1 (langsam, klein) … 10 (schnell).
    pub speed: u8,
}

/// Animation (Status als Einzelbilder).
#[derive(Clone, Debug, PartialEq)]
pub struct AnimSettings {
    /// Schleife (`FLPi`): 0 endlos, −1 keine, n > 0 Wiederholungen.
    pub loop_count: i32,
    /// Auto-Crop (`AACb`), Auto-Differenz (`AADb`).
    pub auto_crop: bool,
    pub auto_difference: bool,
    /// Status-Eigenschaften (`FNLv`).
    pub frames: Vec<FrameInfo>,
}

/// Alle Optimieren-Einstellungen einer Seite, Slice oder Vorgabe.
#[derive(Clone, Debug, PartialEq)]
pub struct OptimizeSettings {
    pub format: ExportFormat,
    pub color_mode: ColorMode,
    pub palette_kind: PaletteKind,
    /// Web-adaptiv (`APSb`).
    pub web_adaptive: bool,
    /// Web-Snap-Toleranz (`ASTi`, Vorgabe 14).
    pub websnap_tolerance: i32,
    /// Maximale Farbzahl (`PLRi`, 2…256).
    pub max_colors: i32,
    /// Stufen der einheitlichen Palette (`PLNi` bei Raster, 2…6).
    pub grid_levels: i32,
    pub dither: Dither,
    pub transparency: Transparency,
    /// Verlust (`GLTi`).
    pub lossy: i32,
    /// „Optimiert“ (`PLOb`): unbenutzte Farben entfernen + LZW-Glättung.
    pub optimized: bool,
    /// Zeilensprung (`GFIb`, auch PNG-Adam7).
    pub interlaced: bool,
    pub sorting: Sorting,
    /// Palette mit Flags (`PALv`).
    pub palette: Vec<PaletteColor>,
    /// Matte (`useMatteColor` + `matteColor`): `Some` = gesetzt.
    pub matte: Option<u32>,
    pub jpeg: JpegSettings,
    pub webp: WebpSettings,
    pub avif: AvifSettings,
    pub anim: AnimSettings,
    /// Exportbereich (links, oben, rechts, unten) in Dokumentpixeln.
    pub crop: Option<[i32; 4]>,
    /// Skalierung in Prozent bzw. Zielgröße.
    pub scale: Option<fp_model::ExportScale>,
    /// BMP 8 Bit mit `BI_RLE8` (re E5, Bittiefe-Modus 1).
    pub bmp_rle8: bool,
}

impl Default for OptimizeSettings {
    /// Original-Vorgaben (`@007dba30`): GIF, adaptiv 256, ohne Dither, „Optimiert“.
    fn default() -> Self {
        OptimizeSettings::from_model(&ExportSettings::default(), None)
    }
}

impl OptimizeSettings {
    /// Typisierte Sicht auf Exporteinstellungen des Modells. `matte` = Seiteneinstellung
    /// (`ExportSettings::html.matte_color`), falls nicht im übergebenen Objekt enthalten.
    pub fn from_model(e: &ExportSettings, matte: Option<u32>) -> OptimizeSettings {
        let format = match e.format {
            1 => ExportFormat::Jpeg,
            2 => ExportFormat::Png,
            3 => ExportFormat::from_mac(e.mac_type.unwrap_or(0)),
            4 => ExportFormat::AnimatedGif,
            _ => ExportFormat::Gif,
        };
        let color_mode = match e.color_mode {
            1 => ColorMode::Rgb24,
            2 => ColorMode::Rgba32,
            _ => ColorMode::Indexed,
        };
        let (dm, dp) = e.dither_mode();
        let dither = match dm {
            1 => Dither::Diffusion(dp),
            2 => Dither::Pattern2x2,
            _ => Dither::None,
        };
        let palette = e
            .palette
            .iter()
            .map(|p| PaletteColor {
                color: p.color,
                locked: p.locked.unwrap_or(false),
                transparent: p.transparent.unwrap_or(false),
                edited: if p.modified.unwrap_or(false) {
                    p.new_color
                } else {
                    None
                },
            })
            .collect();
        let palette_kind = PaletteKind::from_code(e.palette_mode);
        let matte = e.html.as_ref().and_then(|h| h.matte_color).or(matte);
        OptimizeSettings {
            format,
            color_mode,
            palette_kind,
            web_adaptive: e.websnap_adaptive,
            websnap_tolerance: e.websnap_tolerance,
            max_colors: e.entries_requested,
            grid_levels: if palette_kind == PaletteKind::Uniform {
                e.palette_count
            } else {
                6
            },
            dither,
            transparency: Transparency::from_code(e.transparency),
            lossy: e.lossy,
            optimized: e.optimized,
            interlaced: e.interlaced,
            sorting: Sorting::None,
            palette,
            matte,
            jpeg: JpegSettings {
                quality: e.jpeg_quality,
                smoothing: e.jpeg_smoothness,
                subsampling: e.jpeg_subsampling,
                progressive: e.jpeg_progressive,
                selective: SelectiveJpeg {
                    enabled: e.jpeg_sel_enabled,
                    quality: e.jpeg_sel_quality,
                    preserve_text: e.jpeg_sel_preserve_text,
                    preserve_buttons: e.jpeg_sel_preserve_buttons,
                },
            },
            webp: WebpSettings {
                lossless: format == ExportFormat::WebPLossless
                    || color_mode == ColorMode::Rgba32 && format == ExportFormat::AnimatedWebP,
                quality: e.jpeg_quality,
            },
            avif: AvifSettings {
                quality: e.jpeg_quality,
                speed: 6,
            },
            anim: AnimSettings {
                loop_count: e.loop_count,
                auto_crop: e.anim_auto_crop,
                auto_difference: e.anim_auto_difference,
                frames: e.frames.clone(),
            },
            crop: e.crop,
            scale: e.scale,
            bmp_rle8: e.bmp_rle8,
        }
    }

    /// Schreibt die Einstellungen in ein Modellobjekt (unbekannte Felder, HTML-Teil und Rundreise-Daten
    /// von `base` bleiben erhalten). Die Matte landet nur im HTML-Teil, wenn `base` einen hat.
    pub fn to_model(&self, base: &ExportSettings) -> ExportSettings {
        let mut e = base.clone();
        e.format = match self.format {
            ExportFormat::Gif => 0,
            ExportFormat::Jpeg => 1,
            ExportFormat::Png => 2,
            ExportFormat::AnimatedGif => 4,
            _ => 3,
        };
        match self.format.mac() {
            Some((t, c)) => {
                e.mac_type = Some(t);
                e.mac_creator = Some(c);
            }
            None => {
                e.mac_type = None;
                e.mac_creator = None;
            }
        }
        e.color_mode = match self.color_mode {
            ColorMode::Indexed => 0,
            ColorMode::Rgb24 => 1,
            ColorMode::Rgba32 => 2,
        };
        e.palette_mode = self.palette_kind.code();
        e.websnap_adaptive = self.web_adaptive;
        e.websnap_tolerance = self.websnap_tolerance;
        e.entries_requested = self.max_colors;
        e.palette = self
            .palette
            .iter()
            .map(|p| PaletteEntry {
                color: p.color,
                locked: p.locked.then_some(true),
                transparent: p.transparent.then_some(true),
                modified: p.edited.map(|_| true),
                new_color: p.edited,
                extra: Default::default(),
            })
            .collect();
        e.palette_count = if self.palette_kind == PaletteKind::Uniform {
            self.grid_levels
        } else {
            e.palette.len() as i32
        };
        e.dither = match self.dither {
            Dither::None => 0,
            Dither::Pattern2x2 => 2,
            Dither::Diffusion(p) => {
                if p > 5 {
                    p
                } else {
                    0
                }
            }
        };
        e.transparency = self.transparency.code();
        e.lossy = self.lossy;
        e.optimized = self.optimized;
        e.interlaced = self.interlaced;
        e.jpeg_quality = match self.format {
            ExportFormat::WebP | ExportFormat::WebPLossless | ExportFormat::AnimatedWebP => {
                self.webp.quality
            }
            ExportFormat::Avif => self.avif.quality,
            _ => self.jpeg.quality,
        };
        e.jpeg_smoothness = self.jpeg.smoothing;
        e.jpeg_subsampling = self.jpeg.subsampling;
        e.jpeg_progressive = self.jpeg.progressive;
        e.jpeg_sel_enabled = self.jpeg.selective.enabled;
        e.jpeg_sel_quality = self.jpeg.selective.quality;
        e.jpeg_sel_preserve_text = self.jpeg.selective.preserve_text;
        e.jpeg_sel_preserve_buttons = self.jpeg.selective.preserve_buttons;
        e.loop_count = self.anim.loop_count;
        e.anim_auto_crop = self.anim.auto_crop;
        e.anim_auto_difference = self.anim.auto_difference;
        e.frames = self.anim.frames.clone();
        e.crop = self.crop;
        e.scale = self.scale;
        e.bmp_rle8 = self.bmp_rle8;
        if let Some(h) = e.html.as_mut() {
            h.matte_color = self.matte;
        }
        e
    }

    /// Dithering als (Modus, Prozent) wie `ditherMode`/`ditherPercent`.
    pub fn dither_mode(&self) -> (i32, i32) {
        match self.dither {
            Dither::None => (0, 100),
            Dither::Diffusion(p) => (if p == 0 { 0 } else { 1 }, p),
            Dither::Pattern2x2 => (2, 100),
        }
    }

    /// Wirksame Matte (08 §2.1): gesetzte Matte, sonst Leinwandfarbe; transparente Leinwand ⇒ opakes Weiß.
    pub fn effective_matte(&self, canvas: u32) -> u32 {
        match self.matte {
            Some(m) => m,
            None if canvas >> 24 == 0 => 0xFFFF_FFFF,
            None => canvas,
        }
    }
}

/// Eine eingebaute Vorgabe.
#[derive(Clone, Debug, PartialEq)]
pub struct Preset {
    /// Anzeigename (deutsch).
    pub name: &'static str,
    pub settings: OptimizeSettings,
}

/// Eingebaute Vorgaben (08 §1.6, `@007dd850`) plus Firepixel-Vorgaben für moderne Formate.
pub fn builtin_presets() -> Vec<Preset> {
    let base = OptimizeSettings::default();
    let mk = |name: &'static str, f: &dyn Fn(&mut OptimizeSettings)| {
        let mut s = base.clone();
        s.optimized = true;
        f(&mut s);
        Preset { name, settings: s }
    };
    vec![
        mk("GIF Web 216", &|s| {
            s.palette_kind = PaletteKind::Web216;
            s.max_colors = 216;
            s.dither = Dither::Diffusion(100);
        }),
        mk("GIF WebSnap 256", &|s| s.web_adaptive = true),
        mk("GIF WebSnap 128", &|s| {
            s.web_adaptive = true;
            s.max_colors = 128;
        }),
        mk("GIF Adaptiv 256", &|_| {}),
        mk("JPEG – Höhere Qualität", &|s| {
            s.format = ExportFormat::Jpeg;
            s.color_mode = ColorMode::Rgb24;
            s.jpeg.quality = 80;
            s.jpeg.smoothing = 0;
            s.jpeg.subsampling = 0;
        }),
        mk("JPEG – Kleinere Datei", &|s| {
            s.format = ExportFormat::Jpeg;
            s.color_mode = ColorMode::Rgb24;
            s.jpeg.quality = 60;
            s.jpeg.smoothing = 2;
            s.jpeg.subsampling = 1;
        }),
        mk("Animiertes GIF WebSnap 128", &|s| {
            s.format = ExportFormat::AnimatedGif;
            s.web_adaptive = true;
            s.max_colors = 128;
        }),
        mk("PNG8", &|s| s.format = ExportFormat::Png),
        mk("PNG24", &|s| {
            s.format = ExportFormat::Png;
            s.color_mode = ColorMode::Rgb24;
        }),
        mk("PNG32", &|s| {
            s.format = ExportFormat::Png;
            s.color_mode = ColorMode::Rgba32;
        }),
        mk("WebP 80", &|s| {
            s.format = ExportFormat::WebP;
            s.color_mode = ColorMode::Rgba32;
            s.webp.quality = 80;
        }),
        mk("WebP verlustfrei", &|s| {
            s.format = ExportFormat::WebPLossless;
            s.color_mode = ColorMode::Rgba32;
            s.webp.lossless = true;
        }),
        mk("AVIF 60", &|s| {
            s.format = ExportFormat::Avif;
            s.color_mode = ColorMode::Rgba32;
            s.avif.quality = 60;
        }),
        mk("APNG", &|s| {
            s.format = ExportFormat::Apng;
            s.color_mode = ColorMode::Rgba32;
        }),
        mk("Animiertes WebP", &|s| {
            s.format = ExportFormat::AnimatedWebP;
            s.color_mode = ColorMode::Rgba32;
            s.webp.lossless = true;
        }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_roundtrip() {
        let base = ExportSettings::default();
        let s = OptimizeSettings::from_model(&base, None);
        assert_eq!(s.format, ExportFormat::Gif);
        assert_eq!(s.max_colors, 256);
        assert_eq!(s.to_model(&base), base);
        let mut w = s.clone();
        w.format = ExportFormat::WebP;
        w.webp.quality = 77;
        let m = w.to_model(&base);
        assert_eq!(m.format, 3);
        assert_eq!(m.mac_type, Some(fourcc(b"WEBP")));
        let back = OptimizeSettings::from_model(&m, None);
        assert_eq!(back.format, ExportFormat::WebP);
        assert_eq!(back.webp.quality, 77);
        assert_eq!(s.effective_matte(0x00FF_FFFF), 0xFFFF_FFFF);
        assert_eq!(builtin_presets().len(), 15);
    }
}
