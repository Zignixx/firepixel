//! Malen der Bitmapwerkzeuge (Pinsel, Bleistift, Radiergummi) mit Live-Vorschau (PINSEL-1, N31).
//!
//! Ein Malvorgang ist ein [`PaintJob`]: Punktfolge, Pinsel (`fp_model::Brush` mit Spitze, Weichheit,
//! Strichart, Textur), Strichfarbe, Deckkraft, Mischmodus und „Transparenz beibehalten“ bzw. beim
//! Radiergummi Größe, Form, Rand und Deckkraft. Der Strich entsteht mit der Stempel-Engine von
//! fp-vector (`stroke_layer`, Spezifikation 07 §6) und wird pixelweise in die Bitmap gemischt
//! (07 §7: Mischmodus, Deckkraft des ganzen Strichs; Überlappungen im selben Zug addieren sich nicht).
//!
//! Live: [`LivePaint`] hält den Ausgangszustand der Zielbitmap und rechnet beim Ziehen nur die
//! Bereiche neu, die sich durch die neuen Punkte ändern können (Kacheln zu [`TILE`] Pixeln). Das
//! Ergebnis jedes Pixels hängt nur von der Punktfolge ab, nicht davon, in welchen Stücken es
//! gerechnet wurde; der Befehl `bitmap.paint` beim Loslassen rechnet denselben Auftrag noch einmal
//! am Stück (ein Verlaufsschritt) und kommt so auf dasselbe Bild wie die Vorschau.

use crate::docops;
use fp_color::{Argb, BlendMode, Opacity, Rgba8};
use fp_geom::{IRect, Point, Rect};
use fp_model::{Brush, Document, ElementId, ElementKind};
use fp_raster::{CompositeOptions, Mask, RgbaImage};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Geschwindigkeit je Punkt beim Malen (VELf; Maus ohne Stift, wie erzeugte Pfade).
pub const PAINT_VELOCITY: f32 = 0.125;

/// Weglänge (Bildschirmpixel), über die die Zeigergeschwindigkeit gemessen wird (re §2.2:
/// `short` Geschwindigkeit aus Weg/Zeit über die letzten ≈ 55 px, B `@00d48da0`).
pub const VELOCITY_WINDOW_PX: f64 = 55.0;

/// Rasterung der gespeicherten Geschwindigkeit: `VELf` ist die Zeigergeschwindigkeit in
/// Bildschirmpixeln je Millisekunde, auf 1/256 gerundet und nach oben unbegrenzt (B, re\33 §2a:
/// 0,18 px/ms → 47/256, 2,39 px/ms → 585/256; erster Punkt 0).
pub const VELOCITY_STEP: f64 = 256.0;

/// Obergrenze für übergebene Geschwindigkeiten (Schutz vor Unsinn; die Empfindlichkeit sättigt bei
/// 256/256 bzw. `sensitivity_speed` bei 255,996 px/ms).
pub const MAX_VELOCITY: f64 = 256.0;

/// Geschwindigkeit (`VELf`, px/ms in 1/256-Schritten) des letzten Punkts aus Zeigerweg und
/// Zeitstempeln: rückwärts bis ≥ 55 Bildschirmpixel Weg (oder bis zum Zugbeginn), Weg/Zeit. `zoom`
/// rechnet Dokument- in Bildschirmpixel um. `None`, wenn keine Zeit vergangen ist.
pub fn pointer_velocity(points: &[Point], times: &[u64], zoom: f64) -> Option<f32> {
    let n = points.len().min(times.len());
    if n < 2 {
        return None;
    }
    let zoom = if zoom.is_finite() && zoom > 0.0 {
        zoom
    } else {
        1.0
    };
    let mut len = 0.0f64;
    let mut j = n - 1;
    while j > 0 && len < VELOCITY_WINDOW_PX {
        let (a, b) = (points[j - 1], points[j]);
        len += ((b.x - a.x) as f64).hypot((b.y - a.y) as f64) * zoom;
        j -= 1;
    }
    let dt = times[n - 1].saturating_sub(times[j]);
    (dt > 0)
        .then(|| ((len / dt as f64).min(MAX_VELOCITY) * VELOCITY_STEP).round() / VELOCITY_STEP)
        .map(|v| v as f32)
}

/// Kantenlänge der Vorschau-Kacheln in Dokumentpixeln.
pub const TILE: i32 = 256;

/// Malwerkzeug eines Auftrags.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaintTool {
    Brush,
    Pencil,
    Eraser,
}

/// Mischung des Strichs in die Bitmap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaintMix {
    /// Deckkraft-Byte des ganzen Strichs.
    pub opacity: u8,
    /// Mischmodus (`BLDi`).
    pub blend: u8,
    /// „Transparenz beibehalten“: nur auf deckende Pixel malen, Alpha der Bitmap bleibt.
    pub preserve: bool,
}

/// Radiergummi: Abdruck und Deckkraft.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EraseSpec {
    pub size: f32,
    pub round: bool,
    pub edge: f32,
    /// Deckkraft 0..1.
    pub opacity: f32,
}

/// Ein vollständig beschriebener Malauftrag.
#[derive(Clone)]
pub struct PaintJob {
    pub tool: PaintTool,
    pub points: Vec<Point>,
    pub brush: Brush,
    /// Strichtextur (Maske ab 0,0, gekachelt).
    pub texture: Option<Arc<Mask>>,
    pub color: Argb,
    pub mix: PaintMix,
    pub erase: Option<EraseSpec>,
    /// Endpunkt weglassen (früher Vorgabe des Pinsels; echte Züge des Originals malen bis
    /// einschließlich Endpunkt, `real_strokes.txt` r03: 60→500 ergibt 60–500, PAR-X).
    pub trim_end: bool,
    /// Zufalls-Startwert des Zugs (natürliche Pinsel: Streuung, Spitzenversätze, Farbjitter). Je
    /// Punkt wird daraus ein eigener Startwert (`RNDi`) abgeleitet; nie 0 (MINSTD bliebe stehen).
    pub seed: u32,
    /// Geschwindigkeit (`VELf`) je Punkt aus der Zeigerbewegung; fehlende Punkte nehmen den letzten
    /// Wert, leer = [`PAINT_VELOCITY`] (Skripte, Züge ohne Zeitstempel).
    pub velocities: Vec<f32>,
    /// FB-31 (Firepixel extension): indices into `points` where a paused stroke was resumed. Each
    /// index starts a new segment that is not joined to the previous one by a line, but all
    /// segments form ONE stroke: they are rendered into one stroke layer and composited once, so
    /// the overlap of a semi-transparent stroke does not get darker. Empty = one segment.
    pub breaks: Vec<usize>,
}

/// FB-31: the segments of a stroke whose points are split at `breaks` (indices where a new segment
/// starts; out-of-range, unsorted or repeated indices are ignored). Without breaks: one segment.
pub fn split_segments<'a>(points: &'a [Point], breaks: &[usize]) -> Vec<&'a [Point]> {
    let mut out = Vec::new();
    let mut start = 0;
    for &b in breaks {
        if b > start && b < points.len() {
            out.push(&points[start..b]);
            start = b;
        }
    }
    if start < points.len() || out.is_empty() {
        out.push(&points[start..]);
    }
    out
}

/// FB-31: `breaks` parameter of the paint commands (`[i, …]`, indices into `points`).
pub fn parse_breaks(v: Option<&Value>) -> Vec<usize> {
    let Some(Value::Array(a)) = v else {
        return Vec::new();
    };
    a.iter()
        .filter_map(|b| b.as_u64())
        .map(|b| b as usize)
        .collect()
}

/// Startwert aus den Punkten (Aufrufe ohne `seed`: gleiches Ergebnis bei gleichen Punkten).
pub fn seed_of_points(points: &[Point]) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for p in points {
        for v in [p.x.to_bits(), p.y.to_bits()] {
            h ^= v;
            h = h.wrapping_mul(0x0100_0193);
        }
    }
    nonzero_seed(h)
}

/// Startwert im Bereich der MINSTD-Folge (1..2³¹−2).
pub fn nonzero_seed(s: u32) -> u32 {
    let v = s & 0x7fff_ffff;
    if v == 0 || v == 0x7fff_ffff {
        0x2545_f491
    } else {
        v
    }
}

/// Startwert des Punkts `i` (je Punkt verschieden, wie `RNDi` eines echten Zugs im Original).
fn point_seed(seed: u32, i: usize) -> u32 {
    let mut x = seed ^ (i as u32).wrapping_mul(0x9e37_79b9);
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    nonzero_seed(x)
}

impl std::fmt::Debug for PaintJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PaintJob")
            .field("tool", &self.tool)
            .field("points", &self.points.len())
            .field("diameter", &self.brush.diameter)
            .field("mix", &self.mix)
            .finish()
    }
}

/// Strichtextur nach Name (Texturordner) oder vollem Pfad („Andere…“).
pub fn texture_mask(name: &str) -> Option<Arc<Mask>> {
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    let path = std::path::Path::new(name);
    let tex = if path.is_absolute() {
        path.file_stem()
            .and_then(|s| crate::resources::texture_from_file(path, &s.to_string_lossy()))
    } else {
        crate::resources::load_texture(name)
    }?;
    mask_of_texture(&tex)
}

#[cfg(feature = "render")]
fn mask_of_texture(t: &fp_model::attrs::Texture) -> Option<Arc<Mask>> {
    t.mask.as_ref().and_then(fp_render::convert::mask_of)
}

#[cfg(not(feature = "render"))]
fn mask_of_texture(_t: &fp_model::attrs::Texture) -> Option<Arc<Mask>> {
    None
}

fn parse_color(v: Option<&Value>) -> Option<Argb> {
    v.and_then(|v| v.as_str())
        .and_then(Rgba8::parse_hex)
        .map(|c| c.to_argb())
}

/// Punkte `[[x, y], …]` bzw. `[{x, y}, …]`.
pub fn parse_points(v: Option<&Value>) -> Vec<Point> {
    let Some(Value::Array(a)) = v else {
        return Vec::new();
    };
    a.iter()
        .filter_map(|p| match p {
            Value::Array(xy) if xy.len() >= 2 => {
                Some(Point::new(xy[0].as_f64()? as f32, xy[1].as_f64()? as f32))
            }
            Value::Object(o) => Some(Point::new(
                o.get("x")?.as_f64()? as f32,
                o.get("y")?.as_f64()? as f32,
            )),
            _ => None,
        })
        .collect()
}

impl PaintJob {
    /// Auftrag aus den Parametern von `bitmap.paint` (`tool` = Pinsel/Bleistift) bzw. `bitmap.erase`
    /// (`tool` = Radiergummi). Fehlende Werte: Vorgaben aus `fallback_color` und `default_size`.
    pub fn from_params(
        tool: PaintTool,
        p: &Value,
        fallback_color: Argb,
        default_size: f32,
    ) -> PaintJob {
        let points = parse_points(p.get("points"));
        let breaks = parse_breaks(p.get("breaks"));
        let f = |k: &str| p.get(k).and_then(|v| v.as_f64());
        let size = f("size").unwrap_or(default_size as f64).clamp(0.5, 5000.0) as f32;
        let percent = |k: &str| f(k).unwrap_or(100.0).clamp(0.0, 100.0);
        if tool == PaintTool::Eraser {
            let erase = EraseSpec {
                size,
                round: p.get("round").and_then(|v| v.as_bool()).unwrap_or(true),
                edge: f("edge").unwrap_or(0.0).clamp(0.0, 100.0) as f32,
                opacity: (percent("opacity") / 100.0) as f32,
            };
            return PaintJob {
                tool,
                points,
                brush: Brush::basic(size.round().max(1.0) as i32),
                texture: None,
                color: Argb(0),
                mix: PaintMix {
                    opacity: 255,
                    blend: 0,
                    preserve: false,
                },
                erase: Some(erase),
                trim_end: false,
                seed: 1,
                velocities: Vec::new(),
                breaks,
            };
        }
        let mut brush = p
            .get("brush")
            .filter(|v| v.is_object())
            .and_then(|v| serde_json::from_value::<Brush>(v.clone()).ok())
            .unwrap_or_else(|| {
                let mut b = Brush::basic(size.round().max(1.0) as i32);
                b.antialiased = p.get("antialias").and_then(|v| v.as_bool()).unwrap_or(true);
                b
            });
        if p.get("brush").is_none() && p.get("size").is_some() {
            brush.diameter = size.round().max(1.0) as i32;
        }
        brush.diameter = brush.diameter.clamp(1, 5000);
        let texture = p
            .get("texture")
            .and_then(|v| v.as_str())
            .and_then(texture_mask);
        let blend = p
            .get("blend")
            .and_then(|v| {
                v.as_i64()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            })
            .unwrap_or(0)
            .clamp(0, 255) as u8;
        PaintJob {
            tool,
            brush,
            texture,
            color: parse_color(p.get("color")).unwrap_or(fallback_color),
            mix: PaintMix {
                opacity: fp_color::Opacity::from_percent(percent("opacity")).0,
                blend: if blend == 55 { 0 } else { blend },
                preserve: p
                    .get("preserveTransparency")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
            },
            erase: None,
            trim_end: p.get("trimEnd").and_then(|v| v.as_bool()).unwrap_or(false),
            seed: p
                .get("seed")
                .and_then(|v| v.as_u64())
                .map(|s| nonzero_seed(s as u32))
                .unwrap_or_else(|| seed_of_points(&points)),
            velocities: match p.get("velocities") {
                // nach oben unbegrenzt wie die Live-Vorschau (K-8, FP1-06: die frühere Grenze 1 px/ms
                // machte schnelle Züge nach dem Loslassen dicker als in der Vorschau)
                Some(Value::Array(a)) => a
                    .iter()
                    .filter_map(|v| v.as_f64())
                    .map(|v| {
                        if v.is_finite() {
                            v.clamp(0.0, MAX_VELOCITY) as f32
                        } else {
                            0.0
                        }
                    })
                    .collect(),
                _ => Vec::new(),
            },
            points,
            breaks,
        }
    }

    /// FB-31: the segments of the stroke (one without breaks).
    pub fn segments(&self) -> Vec<&[Point]> {
        split_segments(&self.points, &self.breaks)
    }

    /// Stroke points of every segment with the index of its first point in `points`.
    fn stroke_segments(&self) -> Vec<(usize, Vec<Point>)> {
        let mut start = 0;
        self.segments()
            .into_iter()
            .map(|seg| {
                let s = start;
                start += seg.len();
                (s, self.stroke_points_of(seg))
            })
            .collect()
    }

    /// Punkte eines Strichsegments: Einzelklick = ein Abdruck, beim Pinsel ohne den Endpunkt.
    fn stroke_points_of(&self, seg: &[Point]) -> Vec<Point> {
        let mut pts = seg.to_vec();
        if self.trim_end && pts.len() >= 2 {
            let n = pts.len();
            let (a, b) = (pts[n - 2], pts[n - 1]);
            let l = (b.x - a.x).hypot(b.y - a.y);
            if l > 1.0 {
                pts[n - 1] = Point::new(b.x - (b.x - a.x) / l, b.y - (b.y - a.y) / l);
            } else {
                pts.pop();
            }
        }
        if pts.len() == 1 {
            pts.push(pts[0]);
        }
        if self.tool == PaintTool::Pencil && self.brush.antialiased {
            // Bleistift mit Kantenglättung: Die Linie läuft durch die Pixelmitten des Mauspunkts
            // (Original: Waagrechte bei y 150 bleibt eine volle Zeile, PAR-X 03/03)
            for p in &mut pts {
                *p = Point::new(p.x.floor() + 0.5, p.y.floor() + 0.5);
            }
        }
        pts
    }

    /// Wie weit ein Abdruck über seinen Mittelpunkt hinausreicht (Pixel, großzügig).
    pub fn margin(&self) -> i32 {
        if let Some(e) = &self.erase {
            return (e.size / 2.0).ceil() as i32 + 3;
        }
        let b = &self.brush;
        let d = b.diameter.max(1) as f32;
        // Streuung bis ±5 D′ (07 §6.4), Mehrfachspitzen um BTSi·D′/100
        let scatter = if b
            .sensitivities
            .iter()
            .any(|s| s.target == b'R' && s.value > 0)
        {
            5.0 * d
        } else {
            0.0
        };
        let tips = if b.tip_count > 1 {
            b.tip_spacing.max(0) as f32 * d / 100.0
        } else {
            0.0
        };
        (d / 2.0 + scatter + tips).ceil() as i32 + 3
    }

    /// Bereich, den der Strich höchstens berührt.
    pub fn cover(&self) -> IRect {
        let m = self.margin();
        self.segments()
            .into_iter()
            .fold(IRect::default(), |r, seg| r.union(&points_cover(seg, m)))
    }

    /// Neues Bild für `region`: Ausgangspixel `base` (fehlt = transparent) mit dem Strich bzw. dem
    /// Radieren gemischt, beschnitten auf die Pixelauswahl `sel`.
    pub fn render(&self, base: Option<&RgbaImage>, sel: Option<&Mask>, region: IRect) -> RgbaImage {
        let mut dst =
            RgbaImage::new(region).unwrap_or_else(|_| RgbaImage::empty_at(region.left, region.top));
        if region.is_empty() {
            return dst;
        }
        if let Some(b) = base {
            dst.paste(&b.cropped(region));
        }
        if let Some(e) = &self.erase {
            let dab = crate::commands::bitmap::Dab::stroke(e.size, e.round, e.edge);
            // every segment of a resumed stroke starts with its own first dab (FB-31); all dabs
            // form one stroke, so the opacity still applies once
            let stamps: Vec<Point> = self
                .segments()
                .into_iter()
                .flat_map(|seg| {
                    crate::commands::bitmap::dab_points(seg, crate::commands::bitmap::ERASER_STEP)
                })
                .filter(|s| !dab.bounds(*s).intersect(&region).is_empty())
                .collect();
            crate::commands::bitmap::erase_dabs(&mut dst, &dab, &stamps, e.opacity, sel);
            return dst;
        }
        if self.color.a() == 0 || self.mix.opacity == 0 {
            return dst;
        }
        let mut layer = self.stroke_layer(region);
        if let Some(m) = sel {
            let (l, t) = layer.origin();
            let w = layer.width().max(1);
            for (k, px) in layer.data_mut().iter_mut().enumerate() {
                if px[3] != 0 {
                    let mv = m.get(l + (k % w) as i32, t + (k / w) as i32) as u32;
                    px[3] = ((px[3] as u32 * mv + 127) / 255) as u8;
                }
            }
        }
        let keep: Option<Vec<u8>> = self
            .mix
            .preserve
            .then(|| dst.data().iter().map(|p| p[3]).collect());
        let opt = CompositeOptions {
            mode: BlendMode(self.mix.blend),
            opacity: Opacity(self.mix.opacity),
            variant: fp_color::blend::OverVariant::Scalar,
            clip: None,
        };
        fp_raster::composite(&mut dst, &layer, None, &opt);
        if let Some(a) = keep {
            for (px, a) in dst.data_mut().iter_mut().zip(a) {
                px[3] = a;
            }
        }
        dst
    }

    /// Strichebene (Farbe mit Deckung) über `region`.
    #[cfg(all(feature = "vector", feature = "render"))]
    fn stroke_layer(&self, region: IRect) -> RgbaImage {
        use fp_vector as fv;
        // Stiftdaten je Punkt wie ein echter Zug im Original: eigener Zufalls-Startwert je Punkt
        // (sonst wiederholt jedes Segment dieselbe Streuung), Druck 1, Geschwindigkeit wie bei
        // erzeugten Pfaden (`VELf` 0,125, 07 §6.3) bzw. aus der Zeigerbewegung (`velocities`)
        let vel = |i: usize| {
            self.velocities
                .get(i)
                .or(self.velocities.last())
                .copied()
                .unwrap_or(PAINT_VELOCITY)
        };
        // FB-31: one contour per segment of a resumed stroke, all in the one stroke layer
        let contours = self
            .stroke_segments()
            .into_iter()
            .map(|(start, pts)| {
                let mut contour = fv::PathContour::from_points(&pts, false);
                contour.dynamics = (0..pts.len())
                    .map(|i| fv::PointDynamics {
                        pressure: 1.0,
                        velocity: vel(start + i),
                        seed: point_seed(self.seed, start + i),
                    })
                    .collect();
                contour
            })
            .collect();
        let mut path = fv::VectorPath::new(contours);
        path.random_seed = self.seed;
        let mut stroke = fv::Stroke::new(fp_render::convert::brush_params(&self.brush), self.color);
        if self.brush.texture_blend > 0 || self.brush.texture_edge > 0 {
            stroke.texture = self.texture.clone().map(|mask| fv::StrokeTexture { mask });
        }
        path.stroke = Some(stroke.clone());
        let px = fv::stroke::stroke_layer(&path, &stroke, region);
        RgbaImage::from_raw(region, px)
            .unwrap_or_else(|_| RgbaImage::empty_at(region.left, region.top))
    }

    /// Ohne fp-vector/fp-render: runde Abdrücke.
    #[cfg(not(all(feature = "vector", feature = "render")))]
    fn stroke_layer(&self, region: IRect) -> RgbaImage {
        let mut out =
            RgbaImage::new(region).unwrap_or_else(|_| RgbaImage::empty_at(region.left, region.top));
        // FB-31: the segments of a resumed stroke share one layer (the higher coverage wins)
        let c = self.color.to_rgba8();
        for (_, pts) in self.stroke_segments() {
            let Some(m) = crate::commands::bitmap::stroke_mask(
                &pts,
                self.brush.diameter as f32,
                self.brush.antialiased,
                self.brush.shape != 0,
            ) else {
                continue;
            };
            let (l, t) = out.origin();
            let w = out.width().max(1);
            for (k, px) in out.data_mut().iter_mut().enumerate() {
                let v = m.get(l + (k % w) as i32, t + (k / w) as i32);
                let a = ((v as u32 * c.a as u32 + 127) / 255) as u8;
                if v > 0 && a > px[3] {
                    *px = [c.r, c.g, c.b, a];
                }
            }
        }
        out
    }
}

/// Umgebendes Rechteck der Punkte, um `margin` vergrößert.
pub fn points_cover(points: &[Point], margin: i32) -> IRect {
    let Some(first) = points.first() else {
        return IRect::default();
    };
    let (mut l, mut t, mut r, mut b) = (first.x, first.y, first.x, first.y);
    for p in points {
        l = l.min(p.x);
        t = t.min(p.y);
        r = r.max(p.x);
        b = b.max(p.y);
    }
    IRect::new(
        l.floor() as i32,
        t.floor() as i32,
        r.ceil() as i32 + 1,
        b.ceil() as i32 + 1,
    )
    .inflate(margin, margin)
}

/// Ergebnis eines Malauftrags für eine Bitmap: neues Bild (Grenzen ggf. erweitert) oder `None`,
/// wenn sich nichts ändert.
pub fn paint_into(
    job: &PaintJob,
    base: Option<&RgbaImage>,
    sel: Option<&Mask>,
) -> Option<RgbaImage> {
    let mut region = job.cover();
    if job.mix.preserve || job.tool == PaintTool::Eraser {
        region = region.intersect(&base.map(|b| b.bounds()).unwrap_or_default());
    }
    if let Some(m) = sel {
        region = region.intersect(&m.tight_bounds());
    }
    if region.is_empty() {
        return None;
    }
    let patch = job.render(base, sel, region);
    let changed = match base {
        Some(b) => patch.data().iter().enumerate().any(|(k, px)| {
            let w = patch.width().max(1);
            b.get(region.left + (k % w) as i32, region.top + (k / w) as i32)
                .unwrap_or([0; 4])
                != *px
        }),
        None => patch.data().iter().any(|px| px[3] != 0),
    };
    if !changed {
        return None;
    }
    match base {
        Some(b) if !b.is_empty() => {
            // Bitmap nur so weit erweitern, wie der Strich deckende Pixel hat
            let grow = patch.alpha_bounds();
            let bounds = b.bounds().union(&grow);
            let mut out = if bounds == b.bounds() {
                b.clone()
            } else {
                b.extended_to(bounds).ok()?
            };
            out.paste(&patch.cropped(bounds));
            Some(out)
        }
        _ => {
            let t = patch.trimmed();
            (!t.is_empty()).then_some(t)
        }
    }
}

// ------------------------------------------------------------------ Live-Vorschau

/// Eine Kachel der Vorschau: geänderte Pixel der Bitmap (sonst transparent).
#[derive(Clone, Debug)]
pub struct PaintTile {
    pub key: (i32, i32),
    pub rect: IRect,
    pub image: Arc<RgbaImage>,
    /// True new bitmap pixels where `image` is not transparent (`image` shows pixels whose
    /// coverage dropped over a backdrop); `None` = `image` already holds the true pixels. The
    /// canvas composites them in the bitmap's stacking position (BA-039).
    pub exact: Option<Arc<RgbaImage>>,
    /// Steigt bei jeder Änderung der Kachel.
    pub version: u64,
}

/// Vorschau eines laufenden Malvorgangs für die Leinwand ([`crate::Overlay::Paint`]).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PaintPreview {
    /// Bereich mit geänderten Pixeln (Dokumentkoordinaten).
    pub bounds: Rect,
    /// Stand (steigt mit jeder Änderung).
    pub generation: u64,
    /// Kennung des Vorgangs (neuer Zug = neue Kennung).
    pub stroke: u64,
    /// Bemalte Bitmap (`None` = neue Bitmap): Die Leinwand legt die Objekte darüber wieder über
    /// die Vorschau (Stapelfolge, BA-039).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<u64>,
    /// Kacheln (nur in der Oberfläche).
    #[serde(skip)]
    pub tiles: Arc<Vec<PaintTile>>,
}

impl PartialEq for PaintPreview {
    fn eq(&self, o: &Self) -> bool {
        self.generation == o.generation && self.stroke == o.stroke && self.bounds == o.bounds
    }
}

/// Fortlaufende Kennung der Malvorgänge.
fn next_stroke_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(1);
    N.fetch_add(1, Ordering::Relaxed)
}

/// Hintergrund für Pixel, deren Deckung beim Malen sinkt (Radieren, Mischmodus „Radieren“): Die
/// Vorschau liegt über dem alten Bild und kann es nicht durchsichtiger machen; dort zeigt sie das
/// Ergebnis über der Leinwandfarbe (bei transparenter Leinwand über einem Schachbrett).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Backdrop(pub Argb);

impl Backdrop {
    fn at(&self, x: i32, y: i32) -> [u8; 4] {
        if self.0.a() > 0 {
            let c = self.0.to_rgba8();
            return [c.r, c.g, c.b, 255];
        }
        let v = if ((x.div_euclid(8)) + (y.div_euclid(8))) % 2 == 0 {
            255
        } else {
            204
        };
        [v, v, v, 255]
    }
}

/// Laufender Malvorgang: Ausgangsbild, Auftrag, gerechnete Kacheln.
#[derive(Clone)]
pub struct LivePaint {
    pub job: PaintJob,
    base: Option<Arc<RgbaImage>>,
    sel: Option<Arc<Mask>>,
    /// Zielbitmap (`None` = neue Bitmap).
    pub target: Option<ElementId>,
    backdrop: Backdrop,
    tiles: BTreeMap<(i32, i32), PaintTile>,
    /// Vereinigung aller gerechneten Bereiche.
    painted: IRect,
    /// Bereich, den der Strich bisher höchstens berührt.
    cover: IRect,
    generation: u64,
    stroke: u64,
    /// Pixel, die der letzte Schritt neu gerechnet hat (Messung: nur betroffene Bereiche).
    last_area: u64,
}

impl std::fmt::Debug for LivePaint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LivePaint")
            .field("job", &self.job)
            .field("target", &self.target)
            .field("tiles", &self.tiles.len())
            .field("generation", &self.generation)
            .finish()
    }
}

/// Zielbitmap eines Malwerkzeugs: die oberste ausgewählte Bitmap der Seite. Gesperrte Objekte
/// (Element oder Ebene) zählen wie in der Sitzung nicht als ausgewählt (`Session::execute` nimmt
/// sie vor jedem Befehl aus der Auswahl); Live-Vorschau und Befehl wählen so dasselbe Ziel.
pub fn target_bitmap(
    doc: &Document,
    page: usize,
    selection: &[ElementId],
) -> Option<(ElementId, Arc<fp_model::Element>)> {
    docops::paths_sorted(doc, page, selection)
        .into_iter()
        .filter(|p| !docops::is_locked(doc, page, p))
        .find_map(|path| {
            let e = doc.element_at(&path)?.clone();
            matches!(e.kind, ElementKind::Bitmap(_)).then(|| (e.id, e.clone()))
        })
}

impl LivePaint {
    /// Beginnt einen Malvorgang (rechnet den ersten Abdruck).
    pub fn new(
        job: PaintJob,
        base: Option<RgbaImage>,
        sel: Option<Mask>,
        target: Option<ElementId>,
        backdrop: Argb,
    ) -> LivePaint {
        let cover = job.cover();
        let mut lp = LivePaint {
            job,
            base: base.filter(|b| !b.is_empty()).map(Arc::new),
            sel: sel.map(Arc::new),
            target,
            backdrop: Backdrop(backdrop),
            tiles: BTreeMap::new(),
            painted: IRect::default(),
            cover,
            generation: 0,
            stroke: next_stroke_id(),
            last_area: 0,
        };
        lp.render_rects(&[cover]);
        lp
    }

    /// Punkte des Vorgangs.
    pub fn points(&self) -> &[Point] {
        &self.job.points
    }

    /// Hängt einen Punkt an und rechnet die betroffenen Bereiche neu.
    pub fn push(&mut self, p: Point) {
        self.push_with_velocity(p, None);
    }

    /// Wie [`Self::push`] mit der Geschwindigkeit des neuen Punkts (`VELf`, aus der Zeigerbewegung).
    pub fn push_with_velocity(&mut self, p: Point, velocity: Option<f32>) {
        if let Some(v) = velocity {
            let fill = self
                .job
                .velocities
                .last()
                .copied()
                .unwrap_or(PAINT_VELOCITY);
            self.job.velocities.resize(self.job.points.len(), fill);
            self.job.velocities.push(v);
        }
        self.job.points.push(p);
        let n = self.job.points.len();
        let margin = self.job.margin();
        // das bisherige Endstück (ohne Endpunkt, Zählerstand der Stempel) und das neue Stück –
        // nur im laufenden Segment (FB-31: ein fortgesetzter Strich ist nicht verbunden)
        let seg = self
            .job
            .breaks
            .last()
            .copied()
            .filter(|b| *b < n)
            .unwrap_or(0);
        let tail = points_cover(&self.job.points[n.saturating_sub(3).max(seg)..], margin);
        let mut rects = vec![tail];
        let new_cover = self.cover.union(&tail);
        // Wächst die Strichfläche, fallen am alten Rand evtl. abgeschnittene Randpixel weg (07 §6.6
        // „Grenzen des Strichpuffers“): schmale Streifen dort neu rechnen
        let old = self.cover;
        let band = 4;
        if !old.is_empty() {
            if new_cover.left < old.left {
                rects.push(IRect::new(
                    old.left - band,
                    new_cover.top,
                    old.left + margin + band,
                    new_cover.bottom,
                ));
            }
            if new_cover.right > old.right {
                rects.push(IRect::new(
                    old.right - margin - band,
                    new_cover.top,
                    old.right + band,
                    new_cover.bottom,
                ));
            }
            if new_cover.top < old.top {
                rects.push(IRect::new(
                    new_cover.left,
                    old.top - band,
                    new_cover.right,
                    old.top + margin + band,
                ));
            }
            if new_cover.bottom > old.bottom {
                rects.push(IRect::new(
                    new_cover.left,
                    old.bottom - margin - band,
                    new_cover.right,
                    old.bottom + band,
                ));
            }
        }
        self.cover = new_cover;
        self.render_rects(&rects);
    }

    /// FB-31: resumes a paused stroke at `p`: a new segment of the same stroke starts there (not
    /// joined to the previous point by a line) and everything painted so far is computed again
    /// (the stroke area grows at once, see the edge rule in [`Self::push_with_velocity`]).
    pub fn start_segment(&mut self, p: Point, velocity: Option<f32>) {
        let n = self.job.points.len();
        if n == 0 {
            self.push_with_velocity(p, velocity);
            return;
        }
        if let Some(v) = velocity {
            let fill = self
                .job
                .velocities
                .last()
                .copied()
                .unwrap_or(PAINT_VELOCITY);
            self.job.velocities.resize(n, fill);
            self.job.velocities.push(v);
        }
        self.job.breaks.push(n);
        self.job.points.push(p);
        let cover = self.job.cover();
        let all = self.painted.union(&cover);
        self.cover = cover;
        self.render_rects(&[all]);
    }

    /// Indices where the segments of a resumed stroke start (FB-31; empty = one segment).
    pub fn breaks(&self) -> &[usize] {
        &self.job.breaks
    }

    /// Ersetzt die Punktfolge (Umschalt: gerade Linie) und rechnet alles neu.
    pub fn set_points(&mut self, pts: Vec<Point>) {
        self.job.velocities.truncate(pts.len());
        self.job.points = pts;
        let cover = self.job.cover();
        let all = self.painted.union(&cover);
        self.cover = cover;
        self.render_rects(&[all]);
    }

    fn render_rects(&mut self, rects: &[IRect]) {
        self.last_area = 0;
        // weit außerhalb liegende Bereiche nicht rechnen (Schutz vor riesigen Puffern)
        let sane = IRect::new(-1_000_000, -1_000_000, 1_000_000, 1_000_000);
        for r in rects {
            let mut r = r.intersect(&sane);
            if self.job.mix.preserve || self.job.tool == PaintTool::Eraser {
                r = r.intersect(&self.base.as_ref().map(|b| b.bounds()).unwrap_or_default());
            }
            if r.is_empty() {
                continue;
            }
            self.last_area += r.area();
            let img = self
                .job
                .render(self.base.as_deref(), self.sel.as_deref(), r);
            self.store(&img);
            self.painted = self.painted.union(&r);
        }
    }

    /// Schreibt ein gerechnetes Stück in die Kacheln: geänderte Pixel, sonst transparent.
    fn store(&mut self, img: &RgbaImage) {
        let r = img.bounds();
        self.generation += 1;
        let (tx0, ty0) = (r.left.div_euclid(TILE), r.top.div_euclid(TILE));
        let (tx1, ty1) = (
            (r.right - 1).div_euclid(TILE),
            (r.bottom - 1).div_euclid(TILE),
        );
        for ty in ty0..=ty1 {
            for tx in tx0..=tx1 {
                let trect = IRect::from_xywh(tx * TILE, ty * TILE, TILE, TILE);
                let part = r.intersect(&trect);
                if part.is_empty() {
                    continue;
                }
                let tile = self.tiles.entry((tx, ty)).or_insert_with(|| PaintTile {
                    key: (tx, ty),
                    rect: trect,
                    image: Arc::new(
                        RgbaImage::new(trect)
                            .unwrap_or_else(|_| RgbaImage::empty_at(trect.left, trect.top)),
                    ),
                    exact: None,
                    version: 0,
                });
                let ti = Arc::make_mut(&mut tile.image);
                let mut exact = tile.exact.take();
                let base = self.base.as_deref();
                for y in part.top..part.bottom {
                    for x in part.left..part.right {
                        let new = img.get(x, y).unwrap_or([0; 4]);
                        let old = base.and_then(|b| b.get(x, y)).unwrap_or([0; 4]);
                        let shown = if new == old {
                            [0; 4]
                        } else if new[3] >= old[3] {
                            new
                        } else {
                            over(new, self.backdrop.at(x, y))
                        };
                        if shown[3] != 0 && (shown != new || exact.is_some()) {
                            // first pixel whose shown value differs: start from the shown image,
                            // which holds the true pixels everywhere else so far
                            let ex =
                                Arc::make_mut(exact.get_or_insert_with(|| Arc::new(ti.clone())));
                            ex.set_pixel(x, y, Rgba8::new(new[0], new[1], new[2], new[3]));
                        }
                        ti.set_pixel(x, y, Rgba8::new(shown[0], shown[1], shown[2], shown[3]));
                    }
                }
                tile.exact = exact;
                tile.version = self.generation;
            }
        }
    }

    /// Fläche (Pixel), die der letzte Schritt neu gerechnet hat.
    pub fn last_area(&self) -> u64 {
        self.last_area
    }

    /// Vorschau für die Leinwand.
    pub fn preview(&self) -> PaintPreview {
        PaintPreview {
            bounds: self.painted.to_rect(),
            generation: self.generation,
            stroke: self.stroke,
            target: self.target.map(|t| t.0),
            tiles: Arc::new(self.tiles.values().cloned().collect()),
        }
    }

    /// Das Ergebnis, wie die Vorschau es zeigt, für `rect` (Tests: Vorschau = fertige Bitmap).
    pub fn shown_pixel(&self, x: i32, y: i32) -> Option<[u8; 4]> {
        let t = self.tiles.get(&(x.div_euclid(TILE), y.div_euclid(TILE)))?;
        t.image.get(x, y).filter(|p| p[3] != 0)
    }

    /// Parameter für den abschließenden Befehl (`bitmap.paint` bzw. `bitmap.erase`).
    pub fn points_json(&self) -> Value {
        Value::Array(self.job.points.iter().map(|p| json!([p.x, p.y])).collect())
    }

    /// Geschwindigkeiten je Punkt für den abschließenden Befehl (leer = Vorgabe).
    pub fn velocities(&self) -> &[f32] {
        &self.job.velocities
    }
}

/// Preview tiles of a computed bitmap part (retouch live preview, FB-14a): over `area`, the pixels
/// of `new` that differ from `base` (outside `base` = transparent, so a grown bitmap shows its new
/// pixels), stored like [`LivePaint`] stores them: pixels whose coverage dropped are shown over
/// the `backdrop` with the true value in `exact`, unchanged pixels stay transparent.
pub(crate) fn diff_tiles(
    new: &RgbaImage,
    base: &RgbaImage,
    area: IRect,
    backdrop: Argb,
    version: u64,
) -> Vec<PaintTile> {
    let backdrop = Backdrop(backdrop);
    let mut tiles = Vec::new();
    if area.is_empty() {
        return tiles;
    }
    let (tx0, ty0) = (area.left.div_euclid(TILE), area.top.div_euclid(TILE));
    let (tx1, ty1) = (
        (area.right - 1).div_euclid(TILE),
        (area.bottom - 1).div_euclid(TILE),
    );
    for ty in ty0..=ty1 {
        for tx in tx0..=tx1 {
            let trect = IRect::from_xywh(tx * TILE, ty * TILE, TILE, TILE);
            let part = area.intersect(&trect);
            if part.is_empty() {
                continue;
            }
            let Ok(mut ti) = RgbaImage::new(trect) else {
                continue;
            };
            let mut exact: Option<RgbaImage> = None;
            for y in part.top..part.bottom {
                for x in part.left..part.right {
                    let n = new.get(x, y).unwrap_or([0; 4]);
                    let o = base.get(x, y).unwrap_or([0; 4]);
                    if n == o {
                        continue;
                    }
                    let shown = if n[3] >= o[3] {
                        n
                    } else {
                        over(n, backdrop.at(x, y))
                    };
                    if shown[3] != 0 && (shown != n || exact.is_some()) {
                        let ex = exact.get_or_insert_with(|| ti.clone());
                        ex.set_pixel(x, y, Rgba8::new(n[0], n[1], n[2], n[3]));
                    }
                    ti.set_pixel(x, y, Rgba8::new(shown[0], shown[1], shown[2], shown[3]));
                }
            }
            tiles.push(PaintTile {
                key: (tx, ty),
                rect: trect,
                image: Arc::new(ti),
                exact: exact.map(Arc::new),
                version,
            });
        }
    }
    tiles
}

/// Incremental variant of [`diff_tiles`] (FB-39, retouch live preview): rewrites only `area` of
/// the kept `tiles` with the pixels `new(x, y)` that differ from `base` (fully transparent pixels
/// count as equal); tiles outside `area` keep their version, so the canvas re-uploads and
/// re-composites only what the last pointer move changed. A tile is created only where a pixel
/// differs.
pub(crate) fn update_diff_tiles(
    tiles: &mut BTreeMap<(i32, i32), PaintTile>,
    area: IRect,
    new: &dyn Fn(i32, i32) -> [u8; 4],
    base: &RgbaImage,
    backdrop: Argb,
    version: u64,
) {
    let backdrop = Backdrop(backdrop);
    let area = area.intersect(&IRect::new(-1_000_000, -1_000_000, 1_000_000, 1_000_000));
    if area.is_empty() {
        return;
    }
    let norm = |p: [u8; 4]| if p[3] == 0 { [0; 4] } else { p };
    let (tx0, ty0) = (area.left.div_euclid(TILE), area.top.div_euclid(TILE));
    let (tx1, ty1) = (
        (area.right - 1).div_euclid(TILE),
        (area.bottom - 1).div_euclid(TILE),
    );
    for ty in ty0..=ty1 {
        for tx in tx0..=tx1 {
            let trect = IRect::from_xywh(tx * TILE, ty * TILE, TILE, TILE);
            let part = area.intersect(&trect);
            if part.is_empty() {
                continue;
            }
            // pixels of the part: (true value, shown value)
            let mut px = Vec::with_capacity(part.area() as usize);
            let mut any = false;
            for y in part.top..part.bottom {
                for x in part.left..part.right {
                    let n = norm(new(x, y));
                    let o = norm(base.get(x, y).unwrap_or([0; 4]));
                    let shown = if n == o {
                        [0; 4]
                    } else if n[3] >= o[3] {
                        n
                    } else {
                        over(n, backdrop.at(x, y))
                    };
                    any |= shown[3] != 0;
                    px.push((n, shown));
                }
            }
            if !any && !tiles.contains_key(&(tx, ty)) {
                continue;
            }
            let tile = tiles.entry((tx, ty)).or_insert_with(|| PaintTile {
                key: (tx, ty),
                rect: trect,
                image: Arc::new(
                    RgbaImage::new(trect)
                        .unwrap_or_else(|_| RgbaImage::empty_at(trect.left, trect.top)),
                ),
                exact: None,
                version: 0,
            });
            let ti = Arc::make_mut(&mut tile.image);
            let mut exact = tile.exact.take();
            let mut it = px.into_iter();
            for y in part.top..part.bottom {
                for x in part.left..part.right {
                    let Some((n, shown)) = it.next() else { break };
                    if shown[3] != 0 && (shown != n || exact.is_some()) {
                        // first pixel whose shown value differs: start from the shown image,
                        // which holds the true pixels everywhere else so far
                        let ex = Arc::make_mut(exact.get_or_insert_with(|| Arc::new(ti.clone())));
                        ex.set_pixel(x, y, Rgba8::new(n[0], n[1], n[2], n[3]));
                    }
                    ti.set_pixel(x, y, Rgba8::new(shown[0], shown[1], shown[2], shown[3]));
                }
            }
            tile.exact = exact;
            tile.version = version;
        }
    }
}

/// `src` über einem deckenden Hintergrund.
fn over(src: [u8; 4], bg: [u8; 4]) -> [u8; 4] {
    let a = src[3] as u32;
    let mix = |s: u8, d: u8| ((s as u32 * a + d as u32 * (255 - a) + 127) / 255) as u8;
    [
        mix(src[0], bg[0]),
        mix(src[1], bg[1]),
        mix(src[2], bg[2]),
        255,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// T-9 (F25): Werkzeugdeckkraft 30 % = Byte 77 (round(p·2,55)), nicht 76.
    #[test]
    fn t9_tool_opacity_percent_rounds_like_original() {
        let j = PaintJob::from_params(
            PaintTool::Brush,
            &serde_json::json!({"opacity": 30}),
            Argb(0xff00_0000),
            5.0,
        );
        assert_eq!(j.mix.opacity, 77);
    }

    fn job(points: Vec<Point>, soft: i32, size: i32) -> PaintJob {
        let mut b = Brush::basic(size);
        b.antialiased = true;
        b.softness = soft;
        PaintJob {
            tool: PaintTool::Brush,
            points,
            brush: b,
            texture: None,
            color: Argb(0xff00_0000),
            mix: PaintMix {
                opacity: 255,
                blend: 0,
                preserve: false,
            },
            erase: None,
            trim_end: true,
            seed: 7,
            velocities: Vec::new(),
            breaks: Vec::new(),
        }
    }

    #[test]
    fn live_pieces_equal_whole_stroke() {
        let pts: Vec<Point> = (0..40)
            .map(|i| Point::new(20.0 + i as f32 * 7.3, 40.0 + (i as f32 * 0.4).sin() * 30.0))
            .collect();
        let j = job(pts.clone(), 400, 15);
        let whole = paint_into(&j, None, None).expect("Strich");
        let mut lp = LivePaint::new(
            job(vec![pts[0]], 400, 15),
            None,
            None,
            None,
            Argb(0xffff_ffff),
        );
        for p in &pts[1..] {
            lp.push(*p);
        }
        let b = whole.bounds();
        let mut diff = 0;
        for y in b.top..b.bottom {
            for x in b.left..b.right {
                let w = whole.get(x, y).unwrap_or([0; 4]);
                let s = lp.shown_pixel(x, y).unwrap_or([0; 4]);
                if w[3] != 0 && w != s {
                    diff += 1;
                }
            }
        }
        assert_eq!(diff, 0, "Vorschau weicht vom Ergebnis ab");
    }
}
