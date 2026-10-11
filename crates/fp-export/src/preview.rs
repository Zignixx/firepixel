//! Daten für „Bildvorschau“ und die Vorschau-Register: Dateigröße, Ladezeit, dekodierte JPEG-Vorschau.
//!
//! Das Original zeigt unter dem Vorschaubild Format, Dateigröße und Ladezeit bei einer gewählten
//! Verbindungsgeschwindigkeit an (08 §7, A für die Formeln). Ladezeit hier: `Bytes · 8 / Bit je s`,
//! auf ganze Sekunden aufgerundet (mindestens 1 s, sobald die Datei nicht leer ist).

use zune_jpeg::zune_core::bytestream::ZCursor;
use zune_jpeg::zune_core::colorspace::ColorSpace;
use zune_jpeg::zune_core::options::DecoderOptions;
use zune_jpeg::JpegDecoder;

/// Übliche Verbindungsgeschwindigkeiten (Name, Bit je Sekunde).
pub const CONNECTION_SPEEDS: [(&str, u64); 8] = [
    ("14,4 kbit/s", 14_400),
    ("28,8 kbit/s", 28_800),
    ("56 kbit/s", 56_000),
    ("ISDN 128 kbit/s", 128_000),
    ("DSL 1 Mbit/s", 1_000_000),
    ("DSL 16 Mbit/s", 16_000_000),
    ("100 Mbit/s", 100_000_000),
    ("1 Gbit/s", 1_000_000_000),
];

/// Ladezeit in Sekunden (aufgerundet).
pub fn download_seconds(bytes: usize, bits_per_second: u64) -> u64 {
    if bytes == 0 || bits_per_second == 0 {
        return 0;
    }
    ((bytes as u64) * 8).div_ceil(bits_per_second)
}

/// Dateigröße wie in der Vorschau: Byte unter 1 KB, sonst „x,y K“.
pub fn format_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes} Byte")
    } else {
        let k = bytes as f64 / 1024.0;
        format!("{:.2} K", k).replace('.', ",")
    }
}

/// Zusammenfassung für die Vorschauzeile (z. B. „GIF (Dokument)  12,34 K  2 s bei 56 kbit/s  128 Farben“).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewInfo {
    pub format: String,
    pub bytes: usize,
    pub seconds: u64,
    pub speed: &'static str,
    /// Anzahl Farben (indiziert) bzw. 0.
    pub colors: usize,
}

impl PreviewInfo {
    /// Info für ein Exportergebnis bei Geschwindigkeitsindex `speed` aus [`CONNECTION_SPEEDS`].
    pub fn new(format: &str, bytes: usize, colors: usize, speed: usize) -> PreviewInfo {
        let (name, bps) = CONNECTION_SPEEDS[speed.min(CONNECTION_SPEEDS.len() - 1)];
        PreviewInfo { format: format.to_string(), bytes, seconds: download_seconds(bytes, bps), speed: name, colors }
    }

    /// Anzeigetext.
    pub fn text(&self) -> String {
        let mut s = format!("{}  {}  {} s bei {}", self.format, format_size(self.bytes), self.seconds, self.speed);
        if self.colors > 0 {
            s.push_str(&format!("  {} Farben", self.colors));
        }
        s
    }
}

/// JPEG dekodieren (RGB) → (Breite, Höhe, Pixel `0xFFRRGGBB`).
pub fn decode_jpeg(data: &[u8]) -> Option<(usize, usize, Vec<u32>)> {
    let opts = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGB);
    let mut d = JpegDecoder::new_with_options(ZCursor::new(data), opts);
    let px = d.decode().ok()?;
    let info = d.info()?;
    let (w, h) = (info.width as usize, info.height as usize);
    if px.len() < w * h * 3 {
        return None;
    }
    Some((w, h, px.as_chunks::<3>().0.iter().take(w * h).map(|c| 0xFF00_0000 | (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32).collect()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_and_times() {
        assert_eq!(download_seconds(7000, 56_000), 1);
        assert_eq!(download_seconds(7001, 56_000), 2);
        assert_eq!(format_size(500), "500 Byte");
        assert_eq!(format_size(2048), "2,00 K");
        let i = PreviewInfo::new("GIF", 10_000, 64, 2);
        assert_eq!(i.seconds, 2);
        assert!(i.text().contains("64 Farben"));
    }

    #[test]
    fn jpeg_roundtrip_is_close() {
        let t = crate::jpeg_tables::FwJpegTables::ijg_fallback();
        let (w, h) = (37usize, 21usize);
        let px: Vec<u32> = (0..w * h).map(|i| crate::px::argb(255, (i % w * 6) as u32, (i / w * 12) as u32, 100)).collect();
        for prog in [false, true] {
            for sub in [0, 1] {
                for smooth in [0, 3] {
                    let p = crate::jpeg::JpegParams { quality: 90, progressive: prog, subsampling: sub, smoothing: smooth, ..Default::default() };
                    let j = crate::jpeg::encode_jpeg(&px, w, h, &p, &t).unwrap();
                    let (dw, dh, d) = decode_jpeg(&j).expect("dekodierbar");
                    assert_eq!((dw, dh), (w, h));
                    let err: i64 = px.iter().zip(&d).map(|(&a, &b)| crate::px::dist2(a, b) as i64).sum::<i64>() / (w * h) as i64;
                    assert!(err < 120, "prog {prog} sub {sub} smooth {smooth}: mittlerer Fehler² {err}");
                }
            }
        }
    }
}
