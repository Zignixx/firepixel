//! Elemente eines Auftrags (SPEC-BATCH §5.1, §5.2 Punkt 1): Beschreibung aus dem Dateikopf
//! ([`ItemInfo`], [`probe`]), Sammeln aus Dateien/Ordnern ([`collect`]) und das dekodierte
//! Element ([`Item`]: Einzelbild, Animation, Dokument oder „nur Datei“).

use crate::error::{BatchError, Result};
use fp_io::FileFormat;
use fp_raster::{IRect, RgbaImage};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::SystemTime;

/// Art eines Elements.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ItemKind {
    Image,
    Animation,
    Video,
    /// Dokument mit Ebenen (FW-PNG, PSD, SVG, PDF/AI/EPS).
    Document,
    /// Keine Bild-/Video-Datei (nur bei reinen Dateischritten erlaubt).
    Other,
}

impl ItemKind {
    /// Alle Arten.
    pub const ALL: [ItemKind; 5] = [ItemKind::Image, ItemKind::Animation, ItemKind::Video, ItemKind::Document, ItemKind::Other];
    /// Bildarten (ohne Video und „nur Datei“).
    pub const PIXELS: [ItemKind; 3] = [ItemKind::Image, ItemKind::Animation, ItemKind::Document];
}

/// EXIF-Zusammenfassung für Namensbausteine (`{exif-date}`, Kamera …). Gelesen über einen Haken,
/// den das Metadaten-Paket (BATCH-5) mit [`set_exif_probe`] setzt; ohne Haken bleibt sie leer.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExifSummary {
    /// Aufnahmedatum (Ortszeit der Kamera, ohne Zone).
    pub date_taken: Option<chrono::NaiveDateTime>,
    pub make: Option<String>,
    pub model: Option<String>,
    pub lens: Option<String>,
    pub iso: Option<u32>,
    /// Blende als Text (`f/2.8`).
    pub aperture: Option<String>,
    /// EXIF-Ausrichtung 1…8.
    pub orientation: Option<u16>,
}

/// Leser für EXIF-Zusammenfassungen (Pfad, Dateiinhalt).
pub type ExifProbe = fn(&Path, &[u8]) -> Option<ExifSummary>;
static EXIF_PROBE: OnceLock<ExifProbe> = OnceLock::new();

/// Setzt den EXIF-Leser (einmal je Prozess; BATCH-5). Liefert `false`, wenn schon einer gesetzt war.
pub fn set_exif_probe(f: ExifProbe) -> bool {
    EXIF_PROBE.set(f).is_ok()
}

/// EXIF-Zusammenfassung einer Datei (leer ohne Leser).
pub fn read_exif(path: &Path) -> Option<ExifSummary> {
    let f = EXIF_PROBE.get()?;
    let data = std::fs::read(path).ok()?;
    f(path, &data)
}

/// Beschreibung eines Elements (nur Kopf gelesen).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemInfo {
    pub path: PathBuf,
    /// Ordner, über den die Datei hinzugefügt wurde (rekursiv); für „Struktur beibehalten“ und den
    /// neuen Ausgabeordner neben den Quellen.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root: Option<PathBuf>,
    pub kind: ItemKind,
    /// Format (technisch, klein: `png`, `jpeg`, `gif`, `fwpng` …; `""` = unbekannt).
    pub format: String,
    pub bytes: u64,
    /// Pixelmaße aus dem Kopf (0 = unbekannt).
    pub width: u32,
    pub height: u32,
    /// Zahl der Einzelbilder (1 bei Standbildern, 0 = unbekannt).
    pub frames: u32,
    #[serde(skip)]
    pub modified: Option<SystemTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exif: Option<ExifSummary>,
}

impl ItemInfo {
    /// Ordner relativ zur Wurzel (leer ohne Wurzel).
    pub fn rel_dir(&self) -> PathBuf {
        match (&self.root, self.path.parent()) {
            (Some(r), Some(p)) => p.strip_prefix(r).map(Path::to_path_buf).unwrap_or_default(),
            _ => PathBuf::new(),
        }
    }

    /// Dateiname.
    pub fn file_name(&self) -> String {
        self.path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
    }

    /// Ordner der Quelle.
    pub fn dir(&self) -> PathBuf {
        self.path.parent().map(Path::to_path_buf).unwrap_or_default()
    }

    /// Geschätzter Speicherbedarf beim Bearbeiten (§5.2 Punkt 4): B × H × 4 × (2 + Schritte mit Kopie),
    /// bei Animationen × Frames (höchstens 32 im Fenster).
    pub fn memory_estimate(&self, steps: usize) -> u64 {
        let (w, h) = if self.width == 0 || self.height == 0 { (2048u64, 2048u64) } else { (self.width as u64, self.height as u64) };
        let frames = (self.frames.max(1) as u64).min(32);
        w * h * 4 * (2 + steps as u64) * frames
    }
}

/// Format-Kennung (technisch) eines fp-io-Formats.
pub fn format_name(f: FileFormat) -> &'static str {
    match f {
        FileFormat::FwPng => "fwpng",
        FileFormat::Png => "png",
        FileFormat::Apng => "apng",
        FileFormat::Psd => "psd",
        FileFormat::Gif => "gif",
        FileFormat::Jpeg => "jpeg",
        FileFormat::Bmp => "bmp",
        FileFormat::Tiff => "tiff",
        FileFormat::Tga => "tga",
        FileFormat::Wbmp => "wbmp",
        FileFormat::Illustrator => "ai",
        FileFormat::Pdf => "pdf",
        FileFormat::Eps | FileFormat::EpsBinary => "eps",
        FileFormat::Svg => "svg",
        FileFormat::WebP => "webp",
        FileFormat::Avif => "avif",
        FileFormat::Heif => "heif",
        FileFormat::JpegXl => "jxl",
        FileFormat::Qoi => "qoi",
        FileFormat::Ico => "ico",
        FileFormat::Act | FileFormat::Unknown => "",
    }
}

/// Videoendungen (Dekodierung über BATCH-7/10).
pub const VIDEO_EXTENSIONS: &[&str] = &["mp4", "m4v", "mov", "webm", "mkv", "avi"];

fn lower_ext(path: &Path) -> String {
    path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

/// Liest den Kopf einer Datei: Format, Art, Pixelmaße (für PNG/APNG, GIF, JPEG, BMP, WebP, QOI aus
/// dem Kopf, sonst 0), Frames, Größe, Änderungsdatum. Liest nicht die ganze Datei.
pub fn probe(path: &Path) -> Result<ItemInfo> {
    let meta = std::fs::metadata(path).map_err(|e| BatchError::Io(format!("{}: {e}", path.display())))?;
    let mut head = vec![0u8; 64 * 1024];
    let n = {
        use std::io::Read;
        let mut f = std::fs::File::open(path).map_err(|e| BatchError::Io(format!("{}: {e}", path.display())))?;
        let mut total = 0;
        while total < head.len() {
            match f.read(&mut head[total..]) {
                Ok(0) => break,
                Ok(k) => total += k,
                Err(e) => return Err(BatchError::Io(format!("{}: {e}", path.display()))),
            }
        }
        total
    };
    head.truncate(n);
    let name = path.to_string_lossy();
    let ext = lower_ext(path);
    let fmt = if n == 0 { FileFormat::Unknown } else { fp_io::detect_format(&head, &name) };
    let mut info = ItemInfo {
        path: path.to_path_buf(),
        root: None,
        kind: ItemKind::Other,
        format: format_name(fmt).to_string(),
        bytes: meta.len(),
        width: 0,
        height: 0,
        frames: 0,
        modified: meta.modified().ok(),
        exif: None,
    };
    if VIDEO_EXTENSIONS.contains(&ext.as_str()) && fmt == FileFormat::Unknown {
        info.kind = ItemKind::Video;
        info.format = ext;
        return Ok(info);
    }
    info.kind = match fmt {
        FileFormat::Unknown | FileFormat::Act => ItemKind::Other,
        FileFormat::FwPng | FileFormat::Psd | FileFormat::Svg | FileFormat::Pdf | FileFormat::Illustrator | FileFormat::Eps | FileFormat::EpsBinary => ItemKind::Document,
        FileFormat::Apng => ItemKind::Animation,
        _ => ItemKind::Image,
    };
    if let Some((w, h, frames)) = header_dims(fmt, &head) {
        info.width = w;
        info.height = h;
        info.frames = frames;
        if frames > 1 && info.kind == ItemKind::Image {
            info.kind = ItemKind::Animation;
        }
    } else if info.kind == ItemKind::Image {
        info.frames = 1;
    }
    Ok(info)
}

fn be32(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(i..i + 4)?.try_into().ok()?))
}
fn le16(b: &[u8], i: usize) -> Option<u32> {
    Some(u16::from_le_bytes(b.get(i..i + 2)?.try_into().ok()?) as u32)
}
fn le32(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?))
}

/// Pixelmaße und Frames aus dem Dateikopf (None = nicht aus dem Kopf bestimmbar).
fn header_dims(fmt: FileFormat, b: &[u8]) -> Option<(u32, u32, u32)> {
    match fmt {
        FileFormat::Png | FileFormat::Apng | FileFormat::FwPng => {
            let (w, h) = (be32(b, 16)?, be32(b, 20)?);
            // acTL vor IDAT → Zahl der Frames
            let mut i = 8;
            let mut frames = 1;
            while i + 8 <= b.len() {
                let len = be32(b, i)? as usize;
                let ty = b.get(i + 4..i + 8)?;
                if ty == b"acTL" {
                    frames = be32(b, i + 8).unwrap_or(1);
                    break;
                }
                if ty == b"IDAT" {
                    break;
                }
                i += 12 + len;
            }
            Some((w, h, frames))
        }
        FileFormat::Gif => {
            let (w, h) = (le16(b, 6)?, le16(b, 8)?);
            Some((w, h, 0))
        }
        FileFormat::Bmp => {
            let w = le32(b, 18)? as i32;
            let h = le32(b, 22)? as i32;
            Some((w.unsigned_abs(), h.unsigned_abs(), 1))
        }
        FileFormat::Qoi => Some((be32(b, 4)?, be32(b, 8)?, 1)),
        FileFormat::Jpeg => {
            let mut i = 2;
            while i + 9 < b.len() {
                if b[i] != 0xff {
                    i += 1;
                    continue;
                }
                let m = b[i + 1];
                if m == 0xff {
                    i += 1;
                    continue;
                }
                let len = ((b[i + 2] as usize) << 8) | b[i + 3] as usize;
                if matches!(m, 0xc0..=0xcf) && !matches!(m, 0xc4 | 0xc8 | 0xcc) {
                    let h = ((b[i + 5] as u32) << 8) | b[i + 6] as u32;
                    let w = ((b[i + 7] as u32) << 8) | b[i + 8] as u32;
                    return Some((w, h, 1));
                }
                i += 2 + len;
            }
            None
        }
        FileFormat::WebP => {
            let chunk = b.get(12..16)?;
            match chunk {
                b"VP8X" => {
                    let flags = *b.get(20)?;
                    let w = 1 + (le32(b, 24)? & 0xff_ffff);
                    let h = 1 + (le32(b, 27)? & 0xff_ffff);
                    Some((w, h, if flags & 0x02 != 0 { 0 } else { 1 }))
                }
                b"VP8 " => Some((le16(b, 26)? & 0x3fff, le16(b, 28)? & 0x3fff, 1)),
                b"VP8L" => {
                    let v = le32(b, 21)?;
                    Some(((v & 0x3fff) + 1, ((v >> 14) & 0x3fff) + 1, 1))
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// Ergebnis von [`collect`].
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Collected {
    pub items: Vec<ItemInfo>,
    /// Schon in der Liste (gleicher kanonischer Pfad).
    pub duplicates: Vec<PathBuf>,
    /// Nicht aufgenommen (unbekannt/nicht lesbar).
    pub skipped: Vec<PathBuf>,
}

/// Sammelt Dateien und Ordner (Ordner rekursiv, wenn `recursive`). Doppelte (gleicher kanonischer
/// Pfad, auch gegen `existing`) werden nicht erneut aufgenommen; unbekannte Dateien nur, wenn
/// `allow_other` (Rezept mit reinen Dateischritten). Reihenfolge: wie angegeben, Ordnerinhalt
/// natürlich sortiert.
pub fn collect(paths: &[PathBuf], recursive: bool, allow_other: bool, existing: &[ItemInfo]) -> Collected {
    let mut seen: HashSet<PathBuf> = existing.iter().map(|i| canonical(&i.path)).collect();
    let mut out = Collected::default();
    let mut add = |p: &Path, root: Option<&Path>, out: &mut Collected| {
        let key = canonical(p);
        if !seen.insert(key) {
            out.duplicates.push(p.to_path_buf());
            return;
        }
        match probe(p) {
            Ok(mut info) if info.kind != ItemKind::Other || allow_other => {
                info.root = root.map(Path::to_path_buf);
                out.items.push(info);
            }
            _ => out.skipped.push(p.to_path_buf()),
        }
    };
    for p in paths {
        if p.is_dir() {
            let walker = walkdir::WalkDir::new(p).max_depth(if recursive { usize::MAX } else { 1 }).sort_by(|a, b| natural_cmp(&a.file_name().to_string_lossy(), &b.file_name().to_string_lossy()));
            for e in walker.into_iter().flatten() {
                if e.file_type().is_file() {
                    add(e.path(), Some(p), &mut out);
                }
            }
        } else {
            add(p, None, &mut out);
        }
    }
    out
}

/// Kanonischer Pfad zum Vergleichen (Rückfall: Pfad selbst; unter Windows ohne Groß/klein).
pub fn canonical(p: &Path) -> PathBuf {
    let c = std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    if cfg!(windows) { PathBuf::from(c.to_string_lossy().to_lowercase()) } else { c }
}

/// Natürlicher Vergleich (`bild2` vor `bild10`, ohne Groß/klein).
pub fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let (a, b) = (a.to_lowercase(), b.to_lowercase());
    let (mut ia, mut ib) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (ia.peek().copied(), ib.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let mut na = String::new();
                while let Some(c) = ia.peek().copied().filter(char::is_ascii_digit) {
                    na.push(c);
                    ia.next();
                }
                let mut nb = String::new();
                while let Some(c) = ib.peek().copied().filter(char::is_ascii_digit) {
                    nb.push(c);
                    ib.next();
                }
                let (ta, tb) = (na.trim_start_matches('0'), nb.trim_start_matches('0'));
                let o = ta.len().cmp(&tb.len()).then_with(|| ta.cmp(tb)).then_with(|| na.len().cmp(&nb.len()));
                if o != Ordering::Equal {
                    return o;
                }
            }
            (Some(x), Some(y)) => {
                if x != y {
                    return x.cmp(&y);
                }
                ia.next();
                ib.next();
            }
        }
    }
}

// ------------------------------------------------------------------ dekodierte Elemente

/// Begleitdaten eines Elements.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Meta {
    pub dpi: Option<f32>,
    pub has_transparency: bool,
    /// Format der Quelle (technisch).
    pub source_format: String,
    /// Roh-EXIF (TIFF-Block ohne `Exif\0\0`), ICC-Profil, XMP-Paket – von BATCH-5 gelesen/geschrieben.
    pub exif: Option<Vec<u8>>,
    pub icc: Option<Vec<u8>>,
    pub xmp: Option<Vec<u8>>,
    /// Weitere Daten der Schritte (z. B. zwischengespeicherte KI-Maske).
    pub extra: BTreeMap<String, Value>,
}

/// Einzelbild.
#[derive(Clone, Debug, PartialEq)]
pub struct Still {
    /// Pixel (nicht vormultipliziert), Ursprung (0, 0).
    pub image: RgbaImage,
    pub meta: Meta,
}

/// Einzelbild einer Animation (volle Leinwand, schon zusammengesetzt).
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub image: RgbaImage,
    /// Dauer in ms.
    pub delay_ms: u32,
}

/// Animation als Frame-Folge (BATCH-7 ergänzt gestreamte Videos).
#[derive(Clone, Debug, PartialEq)]
pub struct Animation {
    pub width: u32,
    pub height: u32,
    pub frames: Vec<Frame>,
    /// 0 = endlos, n > 0 Wiederholungen, −1 keine Schleife.
    pub loop_count: i32,
    pub meta: Meta,
}

/// Dokument (FW-PNG, PSD, SVG …) – bleibt bearbeitbar, solange kein Pixelschritt es flach rechnet.
#[derive(Clone, Debug)]
pub struct DocItem {
    pub doc: Arc<fp_model::Document>,
    pub meta: Meta,
}

/// Ein Element in Bearbeitung.
#[derive(Clone, Debug)]
pub enum Item {
    Still(Still),
    Animation(Animation),
    Document(DocItem),
    /// Nur Datei (kein Dekodieren, §5.3).
    FileOnly(PathBuf),
}

impl Item {
    /// Art des Elements.
    pub fn kind(&self) -> ItemKind {
        match self {
            Item::Still(_) => ItemKind::Image,
            Item::Animation(_) => ItemKind::Animation,
            Item::Document(_) => ItemKind::Document,
            Item::FileOnly(_) => ItemKind::Other,
        }
    }

    /// Pixelmaße (Dokument: Leinwand der ersten Seite; „nur Datei“: 0 × 0).
    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            Item::Still(s) => (s.image.width() as u32, s.image.height() as u32),
            Item::Animation(a) => (a.width, a.height),
            Item::Document(d) => d.doc.pages.first().map(|p| (p.canvas.width.max(0.0) as u32, p.canvas.height.max(0.0) as u32)).unwrap_or((0, 0)),
            Item::FileOnly(_) => (0, 0),
        }
    }

    /// Begleitdaten (None bei „nur Datei“).
    pub fn meta(&self) -> Option<&Meta> {
        match self {
            Item::Still(s) => Some(&s.meta),
            Item::Animation(a) => Some(&a.meta),
            Item::Document(d) => Some(&d.meta),
            Item::FileOnly(_) => None,
        }
    }

    pub fn meta_mut(&mut self) -> Option<&mut Meta> {
        match self {
            Item::Still(s) => Some(&mut s.meta),
            Item::Animation(a) => Some(&mut a.meta),
            Item::Document(d) => Some(&mut d.meta),
            Item::FileOnly(_) => None,
        }
    }

    /// Rechnet ein Dokument flach zu einem Einzelbild (erste Seite, erster Status, transparenter
    /// Hintergrund außerhalb der Leinwandfarbe). Einzelbild und Animation bleiben.
    pub fn flatten_document(&mut self) -> Result<()> {
        if let Item::Document(d) = self {
            let img = render_document(&d.doc)?;
            let has_transparency = img.data().iter().any(|p| p[3] < 255);
            let meta = Meta { has_transparency, ..d.meta.clone() };
            *self = Item::Still(Still { image: img, meta });
        }
        Ok(())
    }

    /// Wendet `f` auf jedes Pixelbild an (Einzelbild bzw. jedes Frame; Dokumente werden vorher
    /// flach gerechnet). Für Geometrie-/Farbschritte (BATCH-4 ff.). Ändert `f` die Größe der Frames,
    /// übernimmt die Animation die Größe des ersten Frames.
    pub fn map_images(&mut self, mut f: impl FnMut(&mut RgbaImage) -> Result<()>) -> Result<()> {
        self.flatten_document()?;
        match self {
            Item::Still(s) => {
                f(&mut s.image)?;
                s.image.set_origin(0, 0);
            }
            Item::Animation(a) => {
                for fr in &mut a.frames {
                    f(&mut fr.image)?;
                    fr.image.set_origin(0, 0);
                }
                if let Some(first) = a.frames.first() {
                    a.width = first.image.width() as u32;
                    a.height = first.image.height() as u32;
                }
            }
            Item::Document(_) | Item::FileOnly(_) => {}
        }
        Ok(())
    }

    /// Erstes Bild (Vorschau): Einzelbild, erstes Frame bzw. flach gerechnetes Dokument.
    pub fn preview_image(&self) -> Result<RgbaImage> {
        match self {
            Item::Still(s) => Ok(s.image.clone()),
            Item::Animation(a) => a.frames.first().map(|f| f.image.clone()).ok_or_else(|| BatchError::Decode(fp_i18n::t("batch-err-no-frames"))),
            Item::Document(d) => render_document(&d.doc),
            Item::FileOnly(p) => Err(BatchError::Unsupported(fp_i18n::tr!("batch-err-not-image", path = p.display().to_string()))),
        }
    }
}

/// Erste Seite eines Dokuments flach (wie der Export).
pub fn render_document(doc: &fp_model::Document) -> Result<RgbaImage> {
    let r = fp_render::Renderer::default();
    let img = r.render_page(doc, 0, 0).image;
    if img.is_empty() {
        return Err(BatchError::Decode(fp_i18n::t("batch-err-empty-page")));
    }
    let mut img = img;
    img.set_origin(0, 0);
    Ok(img)
}

fn still(r: fp_io::RasterImage, fmt: &str) -> Item {
    let mut image = r.image;
    image.set_origin(0, 0);
    Item::Still(Still { image, meta: Meta { dpi: r.dpi, has_transparency: r.has_transparency, source_format: fmt.to_string(), ..Meta::default() } })
}

fn animation(a: fp_io::Animation, fmt: &str) -> Item {
    let composed = a.composed_frames();
    if composed.len() <= 1 {
        let image = composed.into_iter().next().unwrap_or_else(|| RgbaImage::empty_at(0, 0));
        return Item::Still(Still { image, meta: Meta { has_transparency: a.has_transparency, source_format: fmt.to_string(), ..Meta::default() } });
    }
    let frames = composed
        .into_iter()
        .zip(a.frames.iter())
        .map(|(mut image, f)| {
            image.set_origin(0, 0);
            Frame { image, delay_ms: f.delay_cs.saturating_mul(10) }
        })
        .collect();
    Item::Animation(Animation { width: a.width, height: a.height, frames, loop_count: a.loop_count, meta: Meta { has_transparency: a.has_transparency, source_format: fmt.to_string(), ..Meta::default() } })
}

/// Dekodiert eine Datei. `file_only` = nur Dateischritte (dann wird nichts gelesen).
pub fn load(info: &ItemInfo, file_only: bool) -> Result<Item> {
    if file_only {
        return Ok(Item::FileOnly(info.path.clone()));
    }
    let data = std::fs::read(&info.path).map_err(|e| BatchError::Io(format!("{}: {e}", info.path.display())))?;
    decode_bytes(&info.path.to_string_lossy(), &data)
}

/// Dekodiert Dateiinhalt (Format am Inhalt erkannt, Rückfall Endung).
pub fn decode_bytes(name: &str, data: &[u8]) -> Result<Item> {
    if data.is_empty() {
        return Err(BatchError::Decode(fp_i18n::t("batch-err-empty-file")));
    }
    let fmt = fp_io::detect_format(data, name);
    let fname = format_name(fmt);
    let err = |e: fp_io::ImportError| BatchError::Decode(e.to_string());
    let item = match fmt {
        FileFormat::Png => still(fp_io::png_in::decode_png(data).map_err(err)?, fname),
        FileFormat::Apng => animation(fp_io::png_in::decode_apng(data).map_err(err)?, fname),
        FileFormat::Gif => animation(fp_io::gif::decode(data).map_err(err)?.into_animation(), fname),
        FileFormat::Jpeg => still(fp_io::jpeg_in::decode(data).map_err(err)?, fname),
        FileFormat::Bmp => still(fp_io::bmp::decode(data).map_err(err)?, fname),
        FileFormat::Tga => still(fp_io::tga::decode(data).map_err(err)?, fname),
        FileFormat::Wbmp => still(fp_io::wbmp::decode(data).map_err(err)?, fname),
        FileFormat::Tiff => still(fp_io::tiff_in::decode(data, None).map_err(err)?, fname),
        FileFormat::Qoi => still(fp_io::modern::decode_qoi(data).map_err(err)?, fname),
        FileFormat::WebP => animation(fp_io::modern::decode_webp(data).map_err(err)?, fname),
        FileFormat::JpegXl => animation(fp_io::modern::decode_jxl(data).map_err(err)?, fname),
        FileFormat::Avif => animation(fp_io::avif_in::decode_avif(data).map_err(err)?, fname),
        FileFormat::Heif if fp_io::avif_in::is_av1_heif(data) => animation(fp_io::avif_in::decode_avif(data).map_err(err)?, "avif"),
        FileFormat::Heif => still(fp_io::heic_in::decode_heic(data).map_err(err)?, fname),
        FileFormat::Unknown | FileFormat::Act => return Err(BatchError::Decode(fp_i18n::tr!("batch-err-unknown-format", path = name.to_string()))),
        // Dokumente und ICO über das Öffnen von fp-io
        _ => {
            let r = fp_io::open_bytes(name, data, &fp_io::ImportOptions::default()).map_err(err)?;
            Item::Document(DocItem { doc: Arc::new(r.document), meta: Meta { source_format: fname.to_string(), ..Meta::default() } })
        }
    };
    if let Item::Still(s) = &item
        && s.image.is_empty()
    {
        return Err(BatchError::Decode(fp_i18n::t("batch-err-empty-page")));
    }
    Ok(item)
}

/// Neues Einzelbild aus RGBA-Bytes (Tests, Schritte).
pub fn still_from_rgba(w: u32, h: u32, rgba: &[u8]) -> Result<Item> {
    let image = RgbaImage::from_rgba_bytes(IRect::new(0, 0, w as i32, h as i32), rgba).map_err(|e| BatchError::Decode(e.to_string()))?;
    let has_transparency = rgba.chunks_exact(4).any(|p| p[3] < 255);
    Ok(Item::Still(Still { image, meta: Meta { has_transparency, source_format: "png".into(), ..Meta::default() } }))
}
