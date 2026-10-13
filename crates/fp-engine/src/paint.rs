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
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::Arc;

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
    /// Endpunkt weglassen (Pinsel: Ziehen 300 → 400 füllt 300–399, gemessen am Original).
    pub trim_end: bool,
}

impl std::fmt::Debug for PaintJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PaintJob").field("tool", &self.tool).field("points", &self.points.len()).field("diameter", &self.brush.diameter).field("mix", &self.mix).finish()
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
        path.file_stem().and_then(|s| crate::resources::texture_from_file(path, &s.to_string_lossy()))
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
    v.and_then(|v| v.as_str()).and_then(Rgba8::parse_hex).map(|c| c.to_argb())
}

/// Punkte `[[x, y], …]` bzw. `[{x, y}, …]`.
pub fn parse_points(v: Option<&Value>) -> Vec<Point> {
    let Some(Value::Array(a)) = v else { return Vec::new() };
    a.iter()
        .filter_map(|p| match p {
            Value::Array(xy) if xy.len() >= 2 => Some(Point::new(xy[0].as_f64()? as f32, xy[1].as_f64()? as f32)),
            Value::Object(o) => Some(Point::new(o.get("x")?.as_f64()? as f32, o.get("y")?.as_f64()? as f32)),
            _ => None,
        })
        .collect()
}

impl PaintJob {
    /// Auftrag aus den Parametern von `bitmap.paint` (`tool` = Pinsel/Bleistift) bzw. `bitmap.erase`
    /// (`tool` = Radiergummi). Fehlende Werte: Vorgaben aus `fallback_color` und `default_size`.
    pub fn from_params(tool: PaintTool, p: &Value, fallback_color: Argb, default_size: f32) -> PaintJob {
        let points = parse_points(p.get("points"));
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
                mix: PaintMix { opacity: 255, blend: 0, preserve: false },
                erase: Some(erase),
                trim_end: false,
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
        let texture = p.get("texture").and_then(|v| v.as_str()).and_then(texture_mask);
        let blend = p.get("blend").and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))).unwrap_or(0).clamp(0, 255) as u8;
        PaintJob {
            tool,
            points,
            brush,
            texture,
            color: parse_color(p.get("color")).unwrap_or(fallback_color),
            mix: PaintMix {
                opacity: (percent("opacity") * 2.55).round() as u8,
                blend: if blend == 55 { 0 } else { blend },
                preserve: p.get("preserveTransparency").and_then(|v| v.as_bool()).unwrap_or(false),
            },
            erase: None,
            trim_end: p.get("trimEnd").and_then(|v| v.as_bool()).unwrap_or(tool == PaintTool::Brush),
        }
    }

    /// Punkte des Strichs: Einzelklick = ein Abdruck, beim Pinsel ohne den Endpunkt.
    fn stroke_points(&self) -> Vec<Point> {
        let mut pts = self.points.clone();
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
        let scatter = if b.sensitivities.iter().any(|s| s.target == b'R' && s.value > 0) { 5.0 * d } else { 0.0 };
        let tips = if b.tip_count > 1 { b.tip_spacing.max(0) as f32 * d / 100.0 } else { 0.0 };
        (d / 2.0 + scatter + tips).ceil() as i32 + 3
    }

    /// Bereich, den der Strich höchstens berührt.
    pub fn cover(&self) -> IRect {
        points_cover(&self.points, self.margin())
    }

    /// Neues Bild für `region`: Ausgangspixel `base` (fehlt = transparent) mit dem Strich bzw. dem
    /// Radieren gemischt, beschnitten auf die Pixelauswahl `sel`.
    pub fn render(&self, base: Option<&RgbaImage>, sel: Option<&Mask>, region: IRect) -> RgbaImage {
        let mut dst = RgbaImage::new(region).unwrap_or_else(|_| RgbaImage::empty_at(region.left, region.top));
        if region.is_empty() {
            return dst;
        }
        if let Some(b) = base {
            dst.paste(&b.cropped(region));
        }
        if let Some(e) = &self.erase {
            let dab = crate::commands::bitmap::Dab::new(e.size, e.round, e.edge);
            let stamps: Vec<Point> = crate::commands::bitmap::dab_points(&self.points, crate::commands::bitmap::ERASER_STEP)
                .into_iter()
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
        let keep: Option<Vec<u8>> = self.mix.preserve.then(|| dst.data().iter().map(|p| p[3]).collect());
        let opt = CompositeOptions { mode: BlendMode(self.mix.blend), opacity: Opacity(self.mix.opacity), variant: fp_color::blend::OverVariant::Scalar, clip: None };
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
        let pts = self.stroke_points();
        let mut path = fv::VectorPath::new(vec![fv::PathContour::from_points(&pts, false)]);
        let mut stroke = fv::Stroke::new(fp_render::convert::brush_params(&self.brush), self.color);
        if self.brush.texture_blend > 0 || self.brush.texture_edge > 0 {
            stroke.texture = self.texture.clone().map(|mask| fv::StrokeTexture { mask });
        }
        path.stroke = Some(stroke.clone());
        let px = fv::stroke::stroke_layer(&path, &stroke, region);
        RgbaImage::from_raw(region, px).unwrap_or_else(|_| RgbaImage::empty_at(region.left, region.top))
    }

    /// Ohne fp-vector/fp-render: runde Abdrücke.
    #[cfg(not(all(feature = "vector", feature = "render")))]
    fn stroke_layer(&self, region: IRect) -> RgbaImage {
        let mut out = RgbaImage::new(region).unwrap_or_else(|_| RgbaImage::empty_at(region.left, region.top));
        if let Some(m) = crate::commands::bitmap::stroke_mask(&self.stroke_points(), self.brush.diameter as f32, self.brush.antialiased, self.brush.shape != 0) {
            let c = self.color.to_rgba8();
            let (l, t) = out.origin();
            let w = out.width().max(1);
            for (k, px) in out.data_mut().iter_mut().enumerate() {
                let v = m.get(l + (k % w) as i32, t + (k / w) as i32);
                if v > 0 {
                    *px = [c.r, c.g, c.b, ((v as u32 * c.a as u32 + 127) / 255) as u8];
                }
            }
        }
        out
    }
}

/// Umgebendes Rechteck der Punkte, um `margin` vergrößert.
pub fn points_cover(points: &[Point], margin: i32) -> IRect {
    let Some(first) = points.first() else { return IRect::default() };
    let (mut l, mut t, mut r, mut b) = (first.x, first.y, first.x, first.y);
    for p in points {
        l = l.min(p.x);
        t = t.min(p.y);
        r = r.max(p.x);
        b = b.max(p.y);
    }
    IRect::new(l.floor() as i32, t.floor() as i32, r.ceil() as i32 + 1, b.ceil() as i32 + 1).inflate(margin, margin)
}

/// Ergebnis eines Malauftrags für eine Bitmap: neues Bild (Grenzen ggf. erweitert) oder `None`,
/// wenn sich nichts ändert.
pub fn paint_into(job: &PaintJob, base: Option<&RgbaImage>, sel: Option<&Mask>) -> Option<RgbaImage> {
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
            b.get(region.left + (k % w) as i32, region.top + (k / w) as i32).unwrap_or([0; 4]) != *px
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
            let mut out = if bounds == b.bounds() { b.clone() } else { b.extended_to(bounds).ok()? };
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
        let v = if ((x.div_euclid(8)) + (y.div_euclid(8))) % 2 == 0 { 255 } else { 204 };
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
}

impl std::fmt::Debug for LivePaint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LivePaint").field("job", &self.job).field("target", &self.target).field("tiles", &self.tiles.len()).field("generation", &self.generation).finish()
    }
}

/// Zielbitmap eines Malwerkzeugs: die oberste ausgewählte Bitmap der Seite. Gesperrte Objekte
/// (Element oder Ebene) zählen wie in der Sitzung nicht als ausgewählt (`Session::execute` nimmt
/// sie vor jedem Befehl aus der Auswahl); Live-Vorschau und Befehl wählen so dasselbe Ziel.
pub fn target_bitmap(doc: &Document, page: usize, selection: &[ElementId]) -> Option<(ElementId, Arc<fp_model::Element>)> {
    docops::paths_sorted(doc, page, selection).into_iter().filter(|p| !docops::is_locked(doc, page, p)).find_map(|path| {
        let e = doc.element_at(&path)?.clone();
        matches!(e.kind, ElementKind::Bitmap(_)).then(|| (e.id, e.clone()))
    })
}

impl LivePaint {
    /// Beginnt einen Malvorgang (rechnet den ersten Abdruck).
    pub fn new(job: PaintJob, base: Option<RgbaImage>, sel: Option<Mask>, target: Option<ElementId>, backdrop: Argb) -> LivePaint {
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
        self.job.points.push(p);
        let n = self.job.points.len();
        let margin = self.job.margin();
        // das bisherige Endstück (ohne Endpunkt, Zählerstand der Stempel) und das neue Stück
        let tail = points_cover(&self.job.points[n.saturating_sub(3)..], margin);
        let mut rects = vec![tail];
        let new_cover = self.cover.union(&tail);
        // Wächst die Strichfläche, fallen am alten Rand evtl. abgeschnittene Randpixel weg (07 §6.6
        // „Grenzen des Strichpuffers“): schmale Streifen dort neu rechnen
        let old = self.cover;
        let band = 4;
        if !old.is_empty() {
            if new_cover.left < old.left {
                rects.push(IRect::new(old.left - band, new_cover.top, old.left + margin + band, new_cover.bottom));
            }
            if new_cover.right > old.right {
                rects.push(IRect::new(old.right - margin - band, new_cover.top, old.right + band, new_cover.bottom));
            }
            if new_cover.top < old.top {
                rects.push(IRect::new(new_cover.left, old.top - band, new_cover.right, old.top + margin + band));
            }
            if new_cover.bottom > old.bottom {
                rects.push(IRect::new(new_cover.left, old.bottom - margin - band, new_cover.right, old.bottom + band));
            }
        }
        self.cover = new_cover;
        self.render_rects(&rects);
    }

    /// Ersetzt die Punktfolge (Umschalt: gerade Linie) und rechnet alles neu.
    pub fn set_points(&mut self, pts: Vec<Point>) {
        self.job.points = pts;
        let cover = self.job.cover();
        let all = self.painted.union(&cover);
        self.cover = cover;
        self.render_rects(&[all]);
    }

    fn render_rects(&mut self, rects: &[IRect]) {
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
            let img = self.job.render(self.base.as_deref(), self.sel.as_deref(), r);
            self.store(&img);
            self.painted = self.painted.union(&r);
        }
    }

    /// Schreibt ein gerechnetes Stück in die Kacheln: geänderte Pixel, sonst transparent.
    fn store(&mut self, img: &RgbaImage) {
        let r = img.bounds();
        self.generation += 1;
        let (tx0, ty0) = (r.left.div_euclid(TILE), r.top.div_euclid(TILE));
        let (tx1, ty1) = ((r.right - 1).div_euclid(TILE), (r.bottom - 1).div_euclid(TILE));
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
                    image: Arc::new(RgbaImage::new(trect).unwrap_or_else(|_| RgbaImage::empty_at(trect.left, trect.top))),
                    version: 0,
                });
                let ti = Arc::make_mut(&mut tile.image);
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
                        ti.set_pixel(x, y, Rgba8::new(shown[0], shown[1], shown[2], shown[3]));
                    }
                }
                tile.version = self.generation;
            }
        }
    }

    /// Vorschau für die Leinwand.
    pub fn preview(&self) -> PaintPreview {
        PaintPreview {
            bounds: self.painted.to_rect(),
            generation: self.generation,
            stroke: self.stroke,
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
}

/// `src` über einem deckenden Hintergrund.
fn over(src: [u8; 4], bg: [u8; 4]) -> [u8; 4] {
    let a = src[3] as u32;
    let mix = |s: u8, d: u8| ((s as u32 * a + d as u32 * (255 - a) + 127) / 255) as u8;
    [mix(src[0], bg[0]), mix(src[1], bg[1]), mix(src[2], bg[2]), 255]
}

#[cfg(test)]
mod tests {
    use super::*;

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
            mix: PaintMix { opacity: 255, blend: 0, preserve: false },
            erase: None,
            trim_end: true,
        }
    }

    #[test]
    fn live_pieces_equal_whole_stroke() {
        let pts: Vec<Point> = (0..40).map(|i| Point::new(20.0 + i as f32 * 7.3, 40.0 + (i as f32 * 0.4).sin() * 30.0)).collect();
        let j = job(pts.clone(), 400, 15);
        let whole = paint_into(&j, None, None).expect("Strich");
        let mut lp = LivePaint::new(job(vec![pts[0]], 400, 15), None, None, None, Argb(0xffff_ffff));
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
