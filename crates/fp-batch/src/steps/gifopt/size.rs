//! Zielgröße in KB (SPEC-BATCH §5.6 Punkt 6, §3.10 `maxKB`): Bisektion über `quality` (steuert
//! Farben, Verlust, Dithering-Stärke und Glättung gemeinsam), danach Bildrate halbieren, danach
//! Größe in 10-%-Schritten; höchstens 8 Kodierungen. Jede Kodierung läuft voll (keine Schätzung auf
//! einem Teil der Frames, weil die Bilddifferenz bei ausgedünnten Frames größer wird und die Schätzung
//! verfälscht).

use super::{encode_source, GifOptions, Optimized, Source};

/// Höchstzahl der Kodierungen je Datei.
pub const MAX_ENCODES: usize = 8;
/// Untere Qualitätsgrenze der Suche.
pub const MIN_QUALITY: u8 = 10;

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
        // Bisektion: `lo` passt, `hi` nicht
        let (mut lo, mut hi_q) = (lo_q, hi);
        let mut best = low;
        while encodes.get() < MAX_ENCODES && hi_q > lo + 1 {
            let mid = lo + (hi_q - lo) / 2;
            let r = run(mid, 1, 1.0);
            if r.bytes.len() <= max_bytes {
                lo = mid;
                best = r;
            } else {
                hi_q = mid;
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
