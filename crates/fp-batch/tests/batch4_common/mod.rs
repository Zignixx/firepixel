//! Hilfen der Schritt-Tests von BATCH-4 (Geometrie, Farbe/Effekte, Erweitert).
#![allow(dead_code)]

use fp_batch::item::{Animation, Frame, Item, ItemInfo, ItemKind, Meta, Still};
use fp_batch::output::OutputFormat;
use fp_batch::step::{Issue, Severity, StepContext, StepRegistry, with_defaults};
use fp_batch::{BatchError, CancelToken};
use fp_raster::{IRect, RgbaImage};
use serde_json::Value;
use std::path::PathBuf;

/// Kopf-Beschreibung für ein Element im Speicher.
pub fn info(w: u32, h: u32) -> ItemInfo {
    ItemInfo {
        path: PathBuf::from("memory-test-item.png"),
        root: None,
        kind: ItemKind::Image,
        format: "png".into(),
        bytes: 0,
        width: w,
        height: h,
        frames: 1,
        modified: None,
        exif: None,
    }
}

/// Bild aus einer Pixelfunktion.
pub fn image(w: u32, h: u32, f: impl Fn(u32, u32) -> [u8; 4]) -> RgbaImage {
    let mut v = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            v.extend_from_slice(&f(x, y));
        }
    }
    RgbaImage::from_rgba_bytes(IRect::new(0, 0, w as i32, h as i32), &v).expect("image")
}

/// Verlauf mit etwas Struktur (Golden-Bild, deterministisch).
pub fn gradient(w: u32, h: u32) -> RgbaImage {
    image(w, h, |x, y| {
        [
            (x * 255 / w.max(1)) as u8,
            (y * 255 / h.max(1)) as u8,
            ((x * 7 + y * 13) % 256) as u8,
            255,
        ]
    })
}

/// Einzelbild-Element.
pub fn still(img: RgbaImage) -> Item {
    let has_transparency = img.data().iter().any(|p| p[3] < 255);
    Item::Still(Still {
        image: img,
        meta: Meta {
            has_transparency,
            source_format: "png".into(),
            ..Meta::default()
        },
    })
}

/// Animation aus Frames mit Verzögerungen.
pub fn animation(frames: Vec<(RgbaImage, u32)>) -> Item {
    let (w, h) = frames
        .first()
        .map(|(i, _)| (i.width() as u32, i.height() as u32))
        .unwrap_or((0, 0));
    Item::Animation(Animation {
        width: w,
        height: h,
        frames: frames
            .into_iter()
            .map(|(image, delay_ms)| Frame { image, delay_ms })
            .collect(),
        loop_count: 0,
        meta: Meta {
            source_format: "gif".into(),
            ..Meta::default()
        },
    })
}

/// Wendet einen Schritt des globalen Registers an (Parameter mit Vorgaben). Liefert Warnungen.
pub fn apply(id: &str, params: Value, item: &mut Item) -> Result<Vec<String>, BatchError> {
    let (w, h) = item.dimensions();
    apply_with(id, params, item, &info(w, h), false)
}

/// Wie [`apply`], mit eigener Kopf-Beschreibung und Vorschau-Schalter.
pub fn apply_with(
    id: &str,
    params: Value,
    item: &mut Item,
    inf: &ItemInfo,
    preview: bool,
) -> Result<Vec<String>, BatchError> {
    let reg = StepRegistry::global();
    let step = reg
        .get(id)
        .unwrap_or_else(|| panic!("step {id} registered"));
    let p = with_defaults(&step.params(), &params);
    let cancel = CancelToken::new();
    let mut ctx = StepContext::new(0, 1, inf, &cancel, OutputFormat::Png);
    ctx.preview = preview;
    step.apply(item, &p, &mut ctx)?;
    Ok(ctx.warnings)
}

/// Fehler der Validierung eines Schritts.
pub fn errors(id: &str, params: Value) -> Vec<Issue> {
    let reg = StepRegistry::global();
    reg.get(id)
        .expect("step")
        .validate(&params)
        .into_iter()
        .filter(|i| i.severity == Severity::Error)
        .collect()
}

/// Erstes Bild eines Elements.
pub fn first(item: &Item) -> RgbaImage {
    item.preview_image().expect("image")
}

/// FNV-1a über Maße und Pixel (Golden-Vergleich).
pub fn hash(img: &RgbaImage) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |b: u8| {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    };
    for b in (img.width() as u32)
        .to_le_bytes()
        .into_iter()
        .chain((img.height() as u32).to_le_bytes())
    {
        eat(b);
    }
    for b in img.as_bytes() {
        eat(*b);
    }
    h
}

/// Pixel an (x, y) (lokal).
pub fn px(img: &RgbaImage, x: usize, y: usize) -> [u8; 4] {
    img.row_local(y)[x]
}

/// Größte Kanalabweichung zweier gleich großer Bilder.
pub fn max_diff(a: &RgbaImage, b: &RgbaImage) -> u8 {
    assert_eq!(
        (a.width(), a.height()),
        (b.width(), b.height()),
        "gleiche Größe"
    );
    a.data()
        .iter()
        .zip(b.data())
        .flat_map(|(p, q)| (0..4).map(move |c| p[c].abs_diff(q[c])))
        .max()
        .unwrap_or(0)
}

/// Rechnet `f` einmal mit einem Thread und einmal mit acht (Ergebnisse müssen gleich sein).
pub fn single_and_multi<T>(f: impl Fn() -> T + Send + Sync) -> (T, T)
where
    T: Send,
{
    let one = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .expect("pool")
        .install(&f);
    let many = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .expect("pool")
        .install(&f);
    (one, many)
}
