//! Formaterkennung und Leser-Registry (12 §1.1–§1.4, §6; Referenz `re\import\detect.py`).
//!
//! Das Original wählt den Leser über die Endung (unter Windows gibt es keinen Mac-Typ). Firepixel prüft
//! zuerst die Signatur und fällt auf die Endung zurück (12 §9 „robuster als das Original, aber kompatibel“).

/// Erkannte Dateiformate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FileFormat {
    /// FW-PNG: PNG mit Dokumentdaten (`mkTS`/`mkBS` …).
    FwPng,
    Png,
    /// Animiertes PNG (`acTL` vor `IDAT`).
    Apng,
    Psd,
    Gif,
    Jpeg,
    Bmp,
    Tiff,
    Tga,
    Wbmp,
    /// Illustrator ≤ 8 bzw. PostScript-AI.
    Illustrator,
    /// Illustrator ≥ 9 bzw. PDF.
    Pdf,
    Eps,
    /// DOS-EPS mit Binärkopf (`C5D0D3C6`).
    EpsBinary,
    Svg,
    WebP,
    Avif,
    Heif,
    JpegXl,
    Qoi,
    Ico,
    /// ACT-Palette (kein Bild-Leser, nur „Palette laden“).
    Act,
    Unknown,
}

impl FileFormat {
    /// Anzeigename (deutsch).
    pub fn display_name(self) -> &'static str {
        match self {
            FileFormat::FwPng => "FW-PNG",
            FileFormat::Png => "PNG",
            FileFormat::Apng => "APNG",
            FileFormat::Psd => "PSD",
            FileFormat::Gif => "GIF",
            FileFormat::Jpeg => "JPEG",
            FileFormat::Bmp => "BMP",
            FileFormat::Tiff => "TIFF",
            FileFormat::Tga => "Targa",
            FileFormat::Wbmp => "WBMP",
            FileFormat::Illustrator => "AI",
            FileFormat::Pdf => "PDF",
            FileFormat::Eps | FileFormat::EpsBinary => "EPS",
            FileFormat::Svg => "SVG",
            FileFormat::WebP => "WebP",
            FileFormat::Avif => "AVIF",
            FileFormat::Heif => "HEIC/HEIF",
            FileFormat::JpegXl => "JPEG XL",
            FileFormat::Qoi => "QOI",
            FileFormat::Ico => "ICO",
            FileFormat::Act => "ACT-Palette",
            FileFormat::Unknown => "Unbekannt",
        }
    }
}

/// Eintrag der Leser-Registry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReaderInfo {
    /// Format-ID-Name (Windows-Zwischenablageformat bzw. Firepixel-Name).
    pub id: &'static str,
    /// Anzeigename (deutsch).
    pub name: &'static str,
    /// Dateimuster.
    pub pattern: &'static str,
    /// Endungen (klein, mit Punkt).
    pub extensions: &'static [&'static str],
    /// Steht im Öffnen-/Import-Filter (Dateiliste).
    pub in_file_dialog: bool,
    /// Format.
    pub format: FileFormat,
}

/// Registry in der Reihenfolge des Originals (`@0055f140`) plus Firepixel-Formate am Ende.
pub const REGISTRY: &[ReaderInfo] = &[
    ReaderInfo { id: "PNG", name: "FW-PNG", pattern: "*.png", extensions: &[".png"], in_file_dialog: true, format: FileFormat::Png },
    ReaderInfo { id: "PS30", name: "PSD", pattern: "*.psd", extensions: &[".psd"], in_file_dialog: true, format: FileFormat::Psd },
    ReaderInfo { id: "kMoaCfFormat_GIF", name: "GIF", pattern: "*.gif", extensions: &[".gif"], in_file_dialog: true, format: FileFormat::Gif },
    ReaderInfo { id: "kMoaCfFormat_JPEG", name: "JPEG", pattern: "*.jpg;*.jpe;*.jpeg", extensions: &[".jpg", ".jpe", ".jpeg"], in_file_dialog: true, format: FileFormat::Jpeg },
    ReaderInfo { id: "kMoaCfFormat_BMP", name: "BMP", pattern: "*.bmp;*.dib;*.rle", extensions: &[".bmp", ".dib", ".rle"], in_file_dialog: true, format: FileFormat::Bmp },
    ReaderInfo { id: "kMoaCfFormat_TIFF", name: "TIFF", pattern: "*.tif;*.tiff", extensions: &[".tif", ".tiff"], in_file_dialog: true, format: FileFormat::Tiff },
    ReaderInfo { id: "kMoaCfFormat_TGA", name: "Targa", pattern: "*.tga", extensions: &[".tga"], in_file_dialog: true, format: FileFormat::Tga },
    ReaderInfo { id: "kMoaCfFormat_WBMP", name: "WBMP", pattern: "*.wbmp;*.wbm", extensions: &[".wbmp", ".wbm"], in_file_dialog: true, format: FileFormat::Wbmp },
    ReaderInfo { id: "AI3", name: "AI", pattern: "*.ai;*.art", extensions: &[".ai", ".art"], in_file_dialog: true, format: FileFormat::Illustrator },
    ReaderInfo { id: "EPSF", name: "EPS", pattern: "*.eps", extensions: &[".eps"], in_file_dialog: true, format: FileFormat::Eps },
    ReaderInfo { id: "image/webp", name: "WebP", pattern: "*.webp", extensions: &[".webp"], in_file_dialog: true, format: FileFormat::WebP },
    ReaderInfo { id: "image/jxl", name: "JPEG XL", pattern: "*.jxl", extensions: &[".jxl"], in_file_dialog: true, format: FileFormat::JpegXl },
    ReaderInfo { id: "image/avif", name: "AVIF", pattern: "*.avif", extensions: &[".avif"], in_file_dialog: true, format: FileFormat::Avif },
    ReaderInfo { id: "image/heic", name: "HEIC/HEIF", pattern: "*.heic;*.heif", extensions: &[".heic", ".heif"], in_file_dialog: true, format: FileFormat::Heif },
    ReaderInfo { id: "image/qoi", name: "QOI", pattern: "*.qoi", extensions: &[".qoi"], in_file_dialog: true, format: FileFormat::Qoi },
    ReaderInfo { id: "image/x-icon", name: "ICO", pattern: "*.ico;*.cur", extensions: &[".ico", ".cur"], in_file_dialog: true, format: FileFormat::Ico },
    ReaderInfo { id: "image/svg+xml", name: "SVG", pattern: "*.svg;*.svgz", extensions: &[".svg", ".svgz"], in_file_dialog: true, format: FileFormat::Svg },
    ReaderInfo { id: "kMoaCfFormat_PhotoshopPalette", name: "ACT-Palette", pattern: "*.act", extensions: &[".act"], in_file_dialog: false, format: FileFormat::Act },
];

/// Format über die Endung (erster Leser mit passender Endung, Registrierungsreihenfolge).
pub fn by_extension(name: &str) -> FileFormat {
    let lower = name.to_ascii_lowercase();
    REGISTRY.iter().find(|r| r.extensions.iter().any(|e| lower.ends_with(e))).map(|r| r.format).unwrap_or(FileFormat::Unknown)
}

fn has_chunk(png: &[u8], kind: &[u8; 4], stop_at_idat: bool) -> bool {
    let mut o = 8usize;
    while o + 8 <= png.len() {
        let len = u32::from_be_bytes([png[o], png[o + 1], png[o + 2], png[o + 3]]) as usize;
        let k = &png[o + 4..o + 8];
        if k == kind {
            return true;
        }
        if stop_at_idat && k == b"IDAT" {
            return false;
        }
        o = o.saturating_add(12).saturating_add(len);
    }
    false
}

/// Signaturerkennung (Rückfall Endung).
pub fn detect_format(data: &[u8], name: &str) -> FileFormat {
    let s = |n: usize| data.get(..n).unwrap_or(&[]);
    if s(8) == b"\x89PNG\r\n\x1a\n" {
        if has_chunk(data, b"mkTS", false) || has_chunk(data, b"mkBS", false) {
            return FileFormat::FwPng;
        }
        return if has_chunk(data, b"acTL", true) { FileFormat::Apng } else { FileFormat::Png };
    }
    if s(4) == b"8BPS" {
        return FileFormat::Psd;
    }
    if s(6) == b"GIF87a" || s(6) == b"GIF89a" {
        return FileFormat::Gif;
    }
    if s(3) == b"\xff\xd8\xff" {
        return FileFormat::Jpeg;
    }
    if s(2) == b"BM" && data.len() > 26 {
        return FileFormat::Bmp;
    }
    if s(4) == b"II*\0" || s(4) == b"MM\0*" {
        return FileFormat::Tiff;
    }
    if s(4) == b"%PDF" {
        return FileFormat::Pdf;
    }
    if s(4) == b"\xc5\xd0\xd3\xc6" {
        return FileFormat::EpsBinary;
    }
    // DSC-Kopf `%!PS-<Hersteller>-<Version>`: EPS, wenn die erste Zeile `EPSF` nennt, sonst PostScript-AI
    if s(4) == b"%!PS" {
        let first = data.split(|&b| b == b'\n' || b == b'\r').next().unwrap_or(&[]);
        return if first.windows(4).any(|w| w == b"EPSF") { FileFormat::Eps } else { FileFormat::Illustrator };
    }
    if s(4) == b"RIFF" && data.get(8..12) == Some(b"WEBP") {
        return FileFormat::WebP;
    }
    if data.get(4..8) == Some(b"ftyp") {
        let brand = data.get(8..12).unwrap_or(&[]);
        let compat = data.get(16..data.len().min(64)).unwrap_or(&[]);
        let any = |b: &[u8]| brand == b || compat.chunks(4).any(|c| c == b);
        if any(b"avif") || any(b"avis") {
            return FileFormat::Avif;
        }
        if any(b"heic") || any(b"heix") || any(b"mif1") || any(b"heim") || any(b"hevc") {
            return FileFormat::Heif;
        }
    }
    if s(2) == [0xFF, 0x0A] || s(12) == b"\0\0\0\x0cJXL \r\n\x87\n" {
        return FileFormat::JpegXl;
    }
    if s(4) == b"qoif" {
        return FileFormat::Qoi;
    }
    if (s(4) == [0, 0, 1, 0] || s(4) == [0, 0, 2, 0]) && data.len() >= 22 && data[4] | data[5] != 0 {
        return FileFormat::Ico;
    }
    let head = std::str::from_utf8(&data[..data.len().min(1024)]).unwrap_or("");
    if head.trim_start_matches('\u{feff}').trim_start().starts_with("<?xml") && head.contains("<svg") || head.trim_start().starts_with("<svg") {
        return FileFormat::Svg;
    }
    if s(2) == [0x1f, 0x8b] && name.to_ascii_lowercase().ends_with(".svgz") {
        return FileFormat::Svg;
    }
    let ext = by_extension(name);
    match ext {
        FileFormat::Act if data.len() == 768 || data.len() == 772 => FileFormat::Act,
        FileFormat::Tga => FileFormat::Tga,
        FileFormat::Wbmp if s(1) == [0] => FileFormat::Wbmp,
        FileFormat::Act | FileFormat::Wbmp => FileFormat::Unknown,
        other => other,
    }
}

/// Filter des Öffnen-/Import-Dialogs wie `@010f0c30`: `Name (Muster)|Muster|` je Leser der Dateiliste,
/// dann „Alle lesbaren Dateien“, optional „Alle Dateien (*.*)“, abschließendes `|`.
pub fn open_filter(native_only: bool, with_all_files: bool) -> String {
    let mut parts = String::new();
    let mut pats = Vec::new();
    for r in REGISTRY.iter().filter(|r| r.in_file_dialog) {
        if native_only && r.id.starts_with("image/") {
            continue;
        }
        parts.push_str(&format!("{} ({})|{}|", r.name, r.pattern, r.pattern));
        pats.push(r.pattern);
    }
    if pats.len() > 1 {
        parts.push_str(&format!("Alle lesbaren Dateien|{}|", pats.join(";")));
    }
    if with_all_files {
        parts.push_str("Alle Dateien (*.*)|*.*|");
    }
    parts.push('|');
    parts
}

/// Reihenfolge der Formate beim Einfügen (`PasteImportOrder`, 12 §6): Inhalte runder Klammern
/// (verschachtelt) sind Tokens (`#ID` oder Formatname), Text außerhalb ist Kommentar; danach folgen alle
/// `registry` in Registrierungsreihenfolge.
pub fn paste_order(pref: &str, registry: &[&str]) -> Vec<String> {
    let mut s = pref.to_string();
    for id in registry {
        s.push_str(&format!(" (#{id})"));
    }
    let mut toks = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    for ch in s.chars() {
        if ch == ')' {
            depth -= 1;
            if depth < 1 {
                let t = cur.trim().to_string();
                if !t.is_empty() {
                    toks.push(t);
                }
                cur.clear();
                depth = 0;
                continue;
            }
        }
        if depth > 0 {
            cur.push(ch);
        }
        if ch == '(' {
            depth += 1;
        }
    }
    toks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signatures() {
        assert_eq!(detect_format(b"GIF89a....", "x.png"), FileFormat::Gif);
        assert_eq!(detect_format(b"8BPS\0\x01", "x"), FileFormat::Psd);
        assert_eq!(detect_format(b"\xff\xd8\xff\xe0", ""), FileFormat::Jpeg);
        assert_eq!(detect_format(b"RIFF\0\0\0\0WEBPVP8 ", ""), FileFormat::WebP);
        assert_eq!(detect_format(b"qoif\0\0\0\x01", ""), FileFormat::Qoi);
        assert_eq!(detect_format(b"%!PS-3.0 EPSF-3.0\n", ""), FileFormat::Eps);
        assert_eq!(detect_format(b"%!PS-2.0\n%%Creator: Illustrator", ""), FileFormat::Illustrator);
        assert_eq!(detect_format(b"\0\0\0\x1cftypavif\0\0\0\0avifmif1", ""), FileFormat::Avif);
        assert_eq!(detect_format(b"<svg xmlns='http://www.w3.org/2000/svg'/>", "a"), FileFormat::Svg);
        assert_eq!(detect_format(b"\0\x01\x02", "bild.tga"), FileFormat::Tga);
        assert_eq!(detect_format(&[0u8; 768], "p.act"), FileFormat::Act);
        assert_eq!(by_extension("A.JPE"), FileFormat::Jpeg);
    }

    #[test]
    fn filter_and_paste_order() {
        let f = open_filter(true, true);
        assert!(f.starts_with("FW-PNG (*.png)|*.png|PSD (*.psd)|*.psd|"));
        assert!(f.ends_with("Alle Dateien (*.*)|*.*||"));
        assert!(!f.contains("*.act"));
        let o = paste_order("Illustrator (AI3) PNG (kMoaCfFormat_PNG)", &["PNG", "PS30"]);
        assert_eq!(o, vec!["AI3", "kMoaCfFormat_PNG", "#PNG", "#PS30"]);
    }
}
