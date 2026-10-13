//! Zielgröße in KB (SPEC-BATCH §5.6 Punkt 6, §3.10 `maxKB`): Suche über `quality` (steuert
//! Farben, Verlust, Dithering-Stärke und Glättung gemeinsam), danach Bildrate halbieren, danach
//! Größe in 10-%-Schritten; höchstens 8 Kodierungen. Jede Kodierung läuft voll (keine Schätzung auf
//! einem Teil der Frames, weil die Bilddifferenz bei ausgedünnten Frames größer wird und die Schätzung
//! verfälscht).
//!
//! Schneller am Ziel (BATCH-8b): Die Qualität wird nicht halbiert, sondern aus den beiden
//! einschließenden Kodierungen geschätzt (die Dateigröße wächst etwa exponentiell mit der Qualität →
//! lineare Interpolation von ln(Größe), Regula falsi mit Illinois-Schritt). Liegt ein passendes
//! Ergebnis höchstens 4 % unter dem Ziel, ist es gut genug.

use super::{encode_source, GifOptions, Optimized, Source};

/// Höchstzahl der Kodierungen je Datei.
pub const MAX_ENCODES: usize = 8;
/// Untere Qualitätsgrenze der Suche.
pub const MIN_QUALITY: u8 = 10;
/// Ein passendes Ergebnis mit mindestens so viel Prozent des Ziels beendet die Suche.
pub const GOOD_ENOUGH_PERCENT: usize = 96;

/// Kodiert so, dass die Datei höchstens `max_bytes` groß wird (wenn erreichbar).
pub fn encode_to_target(src: &Source, opts: &GifOptions, max_bytes: usize) -> Optimized {
    let hi = opts.base_quality();
    let encodes = std::cell::Cell::new(0usize);
    let run = |q: u8, step: usize, scale: f32| -> Optimized {
        encodes.set(encodes.get() + 1);
        let mut s = opts.resolve(q);
        s.frame_step = step;
        s.scale = scale;
        encode_source(src, &s, q)
    };
    let first = run(hi, 1, 1.0);
    if first.bytes.len() <= max_bytes {
        return Optimized { reached: true, encodes: 1, ..first };
    }
    let lo_q = MIN_QUALITY.min(hi);
    let low = run(lo_q, 1, 1.0);
    if low.bytes.len() <= max_bytes {
        // Suche: `lo` passt, `hi_q` nicht; nächste Qualität aus ln(Größe) geschätzt
        let (mut lo, mut hi_q) = (lo_q, hi);
        let (mut lo_size, mut hi_size) = ((low.bytes.len().max(1) as f64).ln(), (first.bytes.len().max(1) as f64).ln());
        let target = (max_bytes.max(1) as f64).ln();
        let mut best = low;
        let mut last_side = 0i8;
        while encodes.get() < MAX_ENCODES && hi_q > lo + 1 && best.bytes.len() * 100 < max_bytes * GOOD_ENOUGH_PERCENT {
            let t = if hi_size > lo_size { ((target - lo_size) / (hi_size - lo_size)).clamp(0.0, 1.0) } else { 0.5 };
            let q = (lo as f64 + t * (hi_q - lo) as f64).round() as u8;
            let q = q.clamp(lo + 1, hi_q - 1);
            let r = run(q, 1, 1.0);
            let size = (r.bytes.len().max(1) as f64).ln();
            if r.bytes.len() <= max_bytes {
                lo = q;
                lo_size = size;
                best = r;
                // Illinois: zweimal dieselbe Seite → die andere Grenze halb so stark gewichten
                if last_side == -1 {
                    hi_size = (hi_size + target) / 2.0;
                }
                last_side = -1;
            } else {
                hi_q = q;
                hi_size = size;
                if last_side == 1 {
                    lo_size = (lo_size + target) / 2.0;
                }
                last_side = 1;
            }
        }
        return Optimized { reached: true, encodes: encodes.get(), ..best };
    }
    // Auch die niedrigste Qualität ist zu groß: Bildrate, dann Größe
    let mut smallest = low;
    let step = if src.frames.len() > 2 { 2 } else { 1 };
    if step > 1 && encodes.get() < MAX_ENCODES {
        let r = run(lo_q, step, 1.0);
        if r.bytes.len() <= max_bytes {
            return Optimized { reached: true, encodes: encodes.get(), ..r };
        }
        if r.bytes.len() < smallest.bytes.len() {
            smallest = r;
        }
    }
    let mut scale = 1.0f32;
    while encodes.get() < MAX_ENCODES {
        // Größe grob aus dem Verhältnis schätzen, mindestens 10 % kleiner
        let ratio = (max_bytes as f32 / smallest.bytes.len().max(1) as f32).sqrt();
        scale = (scale * ratio.min(0.9)).max(0.1);
        let r = run(lo_q, step, scale);
        if r.bytes.len() <= max_bytes {
            return Optimized { reached: true, encodes: encodes.get(), ..r };
        }
        if r.bytes.len() < smallest.bytes.len() {
            smallest = r;
        }
        if scale <= 0.1 {
            break;
        }
    }
    Optimized { reached: false, encodes: encodes.get(), ..smallest }
}
