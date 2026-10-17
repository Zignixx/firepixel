//! Daten für „Bildvorschau“ und die Vorschau-Register: Dateigröße, Ladezeit, dekodierte JPEG-Vorschau.
//!
//! Das Original zeigt unter dem Vorschaubild Format, Dateigröße und Ladezeit bei einer gewählten
//! Verbindungsgeschwindigkeit an (08 §7, A für die Formeln). Ladezeit hier: `Bytes · 8 / Bit je s`,
//! auf ganze Sekunden aufgerundet (mindestens 1 s, sobald die Datei nicht leer ist).

use zune_jpeg::JpegDecoder;
use zune_jpeg::zune_core::bytestream::ZCursor;
use zune_jpeg::zune_core::colorspace::ColorSpace;
use zune_jpeg::zune_core::options::DecoderOptions;

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
        PreviewInfo {
            format: format.to_string(),
            bytes,
            seconds: download_seconds(bytes, bps),
            speed: name,
            colors,
        }
    }

    /// Anzeigetext.
    pub fn text(&self) -> String {
        let mut s = format!(
            "{}  {}  {} s bei {}",
            self.format,
            format_size(self.bytes),
            self.seconds,
            self.speed
        );
        if self.colors > 0 {
            s.push_str(&format!("  {} Farben", self.colors));
        }
        s
    }
}

// ---------------------------------------------------------------------- Kopfzeile der Bildvorschau

/// Byte je Sekunde der festen Modemrate der Bildvorschau („56 KBit/s“, re\17-bildvorschau.md §5.4):
/// etwa 49,8 kbit/s nutzbare Rate einer 56k-Verbindung.
pub const MODEM_BYTES_PER_SECOND: usize = 6222;

/// Größenangabe der Kopfzeile (re\17 §5.4) vor dem Übersetzen der Einheit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SizeLabel {
    /// Unter 1000 Byte: Zahl für „… Byte“ (wie im Original `n + 10`).
    Bytes(usize),
    /// Sonst „x.yyK“ mit Punkt und K = 1000 Byte (abgeschnitten, nicht gerundet).
    Kilo(String),
}

/// Größe der Kopfzeile aus der Dateigröße `n` in Byte (re\17 §5.4):
/// `k = n + 5`; unter 1000 → „n + 10 Byte“, sonst `k / 1000` „.“ `(k % 1000) / 10` zweistellig „K“.
pub fn size_label(n: usize) -> SizeLabel {
    let k = n.saturating_add(5);
    if k < 1000 {
        SizeLabel::Bytes(n + 10)
    } else {
        SizeLabel::Kilo(format!("{}.{:02}K", k / 1000, (k % 1000) / 10))
    }
}

/// Ladezeit der Kopfzeile in ganzen Sekunden bei 56 KBit/s: `(n + 5) / 6222`, abgeschnitten.
pub fn modem_seconds(n: usize) -> usize {
    n.saturating_add(5) / MODEM_BYTES_PER_SECOND
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
    Some((
        w,
        h,
        px.as_chunks::<3>()
            .0
            .iter()
            .take(w * h)
            .map(|c| 0xFF00_0000 | (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32)
            .collect(),
    ))
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

    /// Alle Beispielwerte aus re\17-bildvorschau.md §5.4 und die Grenzen der Formel.
    #[test]
    fn header_size_and_modem_time_match_spec_examples() {
        let kilo = |s: &str| SizeLabel::Kilo(s.to_string());
        // BMP 24, 480 × 320 · 3 + 54 Byte: „460.85K  74 s“
        assert_eq!(480 * 320 * 3 + 54, 460_854);
        assert_eq!(size_label(460_854), kilo("460.85K"));
        assert_eq!(modem_seconds(460_854), 74);
        // eigenes GIF 8 × 8 mit 48 Byte: „58 Byte  0 s“ (sic: n + 10)
        assert_eq!(size_label(48), SizeLabel::Bytes(58));
        assert_eq!(modem_seconds(48), 0);
        // GIF Web 216 aus dem Nutzerbild: „612.08K  98 s“
        assert_eq!(size_label(612_080), kilo("612.08K"));
        assert_eq!(modem_seconds(612_080), 98);
        // PNG: „1459.04K  234 s“ (Byte-Zahl nicht notiert; jede aus 1 459 035 … 1 459 044 passt)
        for n in 1_459_035..=1_459_044 {
            assert_eq!(size_label(n), kilo("1459.04K"), "{n}");
            assert_eq!(modem_seconds(n), 234, "{n}");
        }
        // Grenzen: k = n + 5 < 1000 → Byte; zweistellige Hundertstel, abgeschnitten, Punkt fest
        assert_eq!(size_label(0), SizeLabel::Bytes(10));
        assert_eq!(size_label(994), SizeLabel::Bytes(1004));
        assert_eq!(size_label(995), kilo("1.00K"));
        assert_eq!(size_label(1_004), kilo("1.00K"));
        assert_eq!(size_label(1_005), kilo("1.01K"));
        assert_eq!(size_label(1_094), kilo("1.09K"));
        assert_eq!(size_label(1_099_994), kilo("1099.99K"));
        assert_eq!(modem_seconds(6_216), 0);
        assert_eq!(modem_seconds(6_217), 1);
        assert_eq!(modem_seconds(12_438), 1);
        assert_eq!(modem_seconds(12_439), 2);
        assert_eq!(size_label(usize::MAX), size_label(usize::MAX - 5));
    }

    #[test]
    fn jpeg_roundtrip_is_close() {
        let t = crate::jpeg_tables::FwJpegTables::ijg_fallback();
        let (w, h) = (37usize, 21usize);
        let px: Vec<u32> = (0..w * h)
            .map(|i| crate::px::argb(255, (i % w * 6) as u32, (i / w * 12) as u32, 100))
            .collect();
        for prog in [false, true] {
            for sub in [0, 1] {
                for smooth in [0, 3] {
                    let p = crate::jpeg::JpegParams {
                        quality: 90,
                        progressive: prog,
                        subsampling: sub,
                        smoothing: smooth,
                        ..Default::default()
                    };
                    let j = crate::jpeg::encode_jpeg(&px, w, h, &p, &t).unwrap();
                    let (dw, dh, d) = decode_jpeg(&j).expect("dekodierbar");
                    assert_eq!((dw, dh), (w, h));
                    let err: i64 = px
                        .iter()
                        .zip(&d)
                        .map(|(&a, &b)| crate::px::dist2(a, b) as i64)
                        .sum::<i64>()
                        / (w * h) as i64;
                    assert!(
                        err < 120,
                        "prog {prog} sub {sub} smooth {smooth}: mittlerer Fehler² {err}"
                    );
                }
            }
        }
    }
}
