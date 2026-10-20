//! Der Compositor: Seite × Status → Bild.
//!
//! Regeln (Belege in der Crate-Doku): Ebenen unten → oben (geteilte Ebenen und Masterseite über
//! [`fp_model::Document::resolved_layers`] eingesetzt), Elemente in `ELMv` oben → unten (also
//! rückwärts zeichnen), Web-Ebene und Web-Objekte nicht ins Bild, unsichtbare Ebenen/Cels/Elemente
//! überspringen, `PRPb`-Stellvertreterpfade verwerfen. Mischen mit der MMX-Variante von
//! Original-over (Modus Normal) bzw. den Mischmodi aus `fp_color` (skalar), Deckkraft als Byte.

use std::sync::{Arc, Mutex, OnceLock};

use fp_color::blend::OverVariant;
use fp_color::{BlendMode, Opacity, Rgba8};
use fp_effects::{EffectEntry, apply_chain};
use fp_geom::{IRect, Matrix3, Vec2};
use fp_model::{
    Document, Element, ElementKind, ElementState, GroupKind, Layer, LayerKind, LayerSlot, Symbol,
    SymbolKind,
};
use fp_raster::resample::{nine_slice_scale, transform};
use fp_raster::{CompositeOptions, RgbaImage, composite};

use crate::cache::{Look, RenderCache};
use crate::convert;
use crate::nine::{PointMap, Stage, nine_slice_stage};
use crate::options::{BackgroundMode, CachePolicy, RenderOptions, RenderReport};

/// Ergebnis eines Renderings.
#[derive(Debug, Clone)]
pub struct RenderOutput {
    /// Bild (RGBA8, nicht vormultipliziert). Bei Seiten: Grenzen = Leinwand in Dokumentkoordinaten.
    pub image: RgbaImage,
    /// Was fehlte bzw. genähert wurde.
    pub report: RenderReport,
}

/// Ziel-Leinwand: fest (Seite, Ausschnitt) oder wachsend (isolierte Gruppen, Symbole).
pub(crate) struct Canvas {
    pub img: RgbaImage,
    pub growable: bool,
    /// Aufzeichnen statt Mischen (Seitenkacheln, [`crate::tiles`]).
    pub rec: Option<Vec<crate::tiles::DrawOp>>,
}

impl Canvas {
    pub(crate) fn growable() -> Canvas {
        Canvas {
            img: RgbaImage::empty_at(0, 0),
            growable: true,
            rec: None,
        }
    }

    /// Wie [`Canvas::draw`], beim Aufzeichnen aber nur den Schritt merken.
    pub(crate) fn draw_look(&mut self, img: &Arc<RgbaImage>, opacity: Opacity, blend: BlendMode) {
        if let Some(rec) = &mut self.rec {
            if !img.is_empty() && opacity.0 != 0 {
                rec.push(crate::tiles::DrawOp {
                    img: Arc::clone(img),
                    opacity,
                    blend,
                });
            }
            return;
        }
        self.draw(img, opacity, blend);
    }

    /// Mischt ein Aussehen mit Deckkraft und Mischmodus auf die Leinwand.
    pub(crate) fn draw(&mut self, img: &RgbaImage, opacity: Opacity, blend: BlendMode) {
        self.draw_variant(img, opacity, blend, OverVariant::Mmx);
    }

    /// Wie [`Canvas::draw`] mit wählbarer Rundung von „over“ (Masken mit sichtbaren Attributen: Inhalt
    /// skalar über die Füllung, re 26 §3.2).
    pub(crate) fn draw_variant(
        &mut self,
        img: &RgbaImage,
        opacity: Opacity,
        blend: BlendMode,
        variant: OverVariant,
    ) {
        let o = CompositeOptions {
            variant,
            ..opts(opacity, blend)
        };
        if img.is_empty() || opacity.0 == 0 {
            return;
        }
        if self.growable {
            if self.img.is_empty() {
                // Leere Leinwand: Normal mit voller Deckkraft ist eine Kopie.
                let mut base =
                    RgbaImage::new(img.bounds()).unwrap_or_else(|_| RgbaImage::empty_at(0, 0));
                if base.is_empty() {
                    return;
                }
                composite(&mut base, img, None, &o);
                self.img = base;
                return;
            }
            if !contains(self.img.bounds(), img.bounds())
                && let Ok(ext) = self.img.extended_to(img.bounds())
            {
                self.img = ext;
            }
        }
        composite(&mut self.img, img, None, &o);
    }
}

fn contains(a: IRect, b: IRect) -> bool {
    a.left <= b.left && a.top <= b.top && a.right >= b.right && a.bottom >= b.bottom
}

fn opts(opacity: Opacity, blend: BlendMode) -> CompositeOptions {
    CompositeOptions {
        mode: blend,
        opacity,
        variant: OverVariant::Mmx,
        clip: None,
    }
}

/// Zustand beim Abstieg.
#[derive(Clone)]
pub(crate) struct State {
    pub map: PointMap,
    pub frame: usize,
    /// Symbole auf dem Weg (Schutz vor Rekursion).
    pub symbols: Vec<fp_model::SymbolId>,
    /// Replaced pixels of one bitmap (live paint preview, BA-039).
    pub over: Option<Arc<Override>>,
}

/// Replaced pixels of one bitmap element plus the containers that hold it (their caches are
/// bypassed while the override is active).
pub(crate) struct Override {
    pub id: u64,
    pub image: Arc<RgbaImage>,
    pub ancestors: std::collections::HashSet<u64>,
}

impl State {
    /// Is `e` the overridden bitmap or one of its containers (no cached look then)?
    fn overridden(&self, e: &Element) -> bool {
        self.over
            .as_ref()
            .is_some_and(|o| o.id == e.id.0 || o.ancestors.contains(&e.id.0))
    }
}

/// Pixels that replace the stored pixels of one bitmap element for a single render: the live
/// preview of the bitmap tools composites the stroke in the bitmap's stacking position, with its
/// opacity, blend mode, live effects and masks (BA-039). `image` is in document coordinates like
/// the stored bitmap (its bounds may be a crop of the bitmap when only that part is rendered).
#[derive(Clone, Debug)]
pub struct PixelOverride {
    pub id: fp_model::ElementId,
    pub image: Arc<RgbaImage>,
}

/// Der Renderer (thread-sicher; Element-Cache intern).
pub struct Renderer {
    /// Einstellungen.
    pub options: RenderOptions,
    fonts: OnceLock<Arc<fp_text::FontDb>>,
    cache: Mutex<RenderCache>,
    tiles: Mutex<crate::tiles::TileCache>,
}

impl std::fmt::Debug for Renderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Renderer")
            .field("options", &self.options)
            .finish_non_exhaustive()
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Renderer::new(RenderOptions::default())
    }
}

impl Renderer {
    /// Neuer Renderer; Systemschriften werden beim ersten Text geladen.
    pub fn new(options: RenderOptions) -> Renderer {
        Renderer {
            options,
            fonts: OnceLock::new(),
            cache: Mutex::new(RenderCache::default()),
            tiles: Mutex::new(crate::tiles::TileCache::default()),
        }
    }

    /// Neuer Renderer mit vorgegebener Schriftdatenbank (z. B. von der Anwendung geteilt).
    pub fn with_fonts(options: RenderOptions, fonts: Arc<fp_text::FontDb>) -> Renderer {
        let r = Renderer::new(options);
        let _ = r.fonts.set(fonts);
        r
    }

    /// Schriftdatenbank (beim ersten Aufruf aus den Systemschriften aufgebaut).
    pub fn fonts(&self) -> &Arc<fp_text::FontDb> {
        self.fonts
            .get_or_init(|| Arc::new(fp_text::FontDb::system()))
    }

    /// Zugriff auf den Element-Cache (Generation erhöhen, leeren, Kennzahlen).
    pub fn with_cache<R>(&self, f: impl FnOnce(&mut RenderCache) -> R) -> R {
        match self.cache.lock() {
            Ok(mut c) => f(&mut c),
            Err(p) => f(&mut p.into_inner()),
        }
    }

    /// Kennzahlen des Kachel-Caches.
    pub fn tile_stats(&self) -> crate::tiles::TileStats {
        match self.tiles.lock() {
            Ok(t) => t.stats(),
            Err(p) => p.into_inner().stats(),
        }
    }

    /// Leert den Kachel-Cache (z. B. nach dem Schließen eines Dokuments).
    pub fn clear_tiles(&self) {
        match self.tiles.lock() {
            Ok(mut t) => t.clear(),
            Err(p) => p.into_inner().clear(),
        }
    }

    /// Leinwand der Seite in Dokumentkoordinaten.
    pub fn page_rect(doc: &Document, page: usize) -> Option<IRect> {
        let p = doc.pages.get(page)?;
        let c = &p.canvas;
        let (x, y) = (
            convert::round_coord(c.origin.x as f64),
            convert::round_coord(c.origin.y as f64),
        );
        let w = (c.width as f64).round().clamp(0.0, 1.0e6) as i32;
        let h = (c.height as f64).round().clamp(0.0, 1.0e6) as i32;
        Some(IRect::from_xywh(x, y, w, h))
    }

    /// Ganze Seite `page` im Status `frame` (Leinwandfarbe nach [`BackgroundMode`]).
    pub fn render_page(&self, doc: &Document, page: usize, frame: usize) -> RenderOutput {
        match Self::page_rect(doc, page) {
            Some(r) => self.render_rect(doc, page, frame, r),
            None => {
                let mut report = RenderReport::default();
                report.miss("page");
                RenderOutput {
                    image: RgbaImage::empty_at(0, 0),
                    report,
                }
            }
        }
    }

    /// Ausschnitt `rect` (Dokumentkoordinaten, 1:1) einer Seite.
    pub fn render_rect(
        &self,
        doc: &Document,
        page: usize,
        frame: usize,
        rect: IRect,
    ) -> RenderOutput {
        let (grid, image, report) = self.compose_rect(doc, page, frame, rect, None);
        match grid {
            Some(g) => RenderOutput {
                image: g.assemble(rect),
                report,
            },
            None => RenderOutput { image, report },
        }
    }

    /// Seite im Rechteck mischen: als Kacheln (Kachel-Cache) oder als fertiges Bild.
    fn compose_rect(
        &self,
        doc: &Document,
        page: usize,
        frame: usize,
        rect: IRect,
        over: Option<Arc<Override>>,
    ) -> (Option<crate::tiles::TileGrid>, RgbaImage, RenderReport) {
        let mut report = RenderReport::default();
        let Some(p) = doc.pages.get(page) else {
            report.miss("page");
            return (None, RgbaImage::empty_at(rect.left, rect.top), report);
        };
        let bg = p.root.background.to_rgba8();
        let mode = match self.options.background {
            BackgroundMode::Auto => {
                let mut erase = false;
                fp_model::traverse::walk_root(
                    &p.root,
                    fp_model::traverse::RootRef::Page(page),
                    &mut |_, _, e| {
                        erase |= e.common.blend.0 == 25;
                    },
                );
                if erase {
                    BackgroundMode::UnderlayLast
                } else {
                    BackgroundMode::Fill
                }
            }
            m => m,
        };
        let fill = match mode {
            BackgroundMode::Fill => bg,
            _ => Rgba8::TRANSPARENT,
        };
        let st = State {
            map: PointMap::identity(),
            frame,
            symbols: Vec::new(),
            over,
        };
        let underlay = if mode == BackgroundMode::UnderlayLast && bg.a > 0 {
            Some(bg)
        } else {
            None
        };
        if self.options.tile_cache && self.options.cache_budget_pixels > 0 && st.over.is_none() {
            // Aufzeichnen (Aussehen aus dem Element-Cache), dann kachelweise mischen; unveränderte
            // Kacheln kommen aus dem Kachel-Cache (Ergebnis pixelgleich zum direkten Mischen).
            let mut cv = Canvas {
                img: RgbaImage::empty_at(rect.left, rect.top),
                growable: false,
                rec: Some(Vec::new()),
            };
            for rl in doc.resolved_layers(page) {
                self.draw_layer(doc, &mut cv, &rl.layer, 0, rl.state(), &st, &mut report);
            }
            let ops = cv.rec.take().unwrap_or_default();
            let budget = self.options.cache_budget_pixels / 4;
            let grid = match self.tiles.lock() {
                Ok(mut t) => t.compose_grid(rect, fill, underlay, &ops, budget),
                Err(p) => p
                    .into_inner()
                    .compose_grid(rect, fill, underlay, &ops, budget),
            };
            return match grid {
                Some(g) => (Some(g), RgbaImage::empty_at(rect.left, rect.top), report),
                None => (
                    None,
                    crate::tiles::mix_direct(rect, fill, underlay, &ops),
                    report,
                ),
            };
        }
        let img = RgbaImage::filled(rect, fill)
            .unwrap_or_else(|_| RgbaImage::empty_at(rect.left, rect.top));
        let mut cv = Canvas {
            img,
            growable: false,
            rec: None,
        };
        for rl in doc.resolved_layers(page) {
            self.draw_layer(doc, &mut cv, &rl.layer, 0, rl.state(), &st, &mut report);
        }
        let mut image = cv.img;
        if mode == BackgroundMode::UnderlayLast && bg.a > 0 {
            let mut base = RgbaImage::filled(rect, bg)
                .unwrap_or_else(|_| RgbaImage::empty_at(rect.left, rect.top));
            composite(&mut base, &image, None, &opts(Opacity(255), BlendMode(0)));
            image = base;
        }
        (None, image, report)
    }

    /// Ausschnitt `region` (Dokumentkoordinaten) mit Zoomfaktor `zoom` für die Leinwand der UI.
    /// Ergebnis in Gerätepixeln (`ceil(Breite·zoom)` × `ceil(Höhe·zoom)`, Grenzen ab (0, 0));
    /// abgetastet wird je Gerätepixelmitte (nächster Nachbar), so bleiben Pixel beim Vergrößern scharf.
    pub fn render_region(
        &self,
        doc: &Document,
        page: usize,
        frame: usize,
        region: IRect,
        zoom: f64,
    ) -> RenderOutput {
        self.region_of(doc, page, frame, region, zoom, None)
    }

    /// Device pixels `dev` of the canvas grid anchored at `anchor` (document point of device pixel
    /// (0, 0)) with zoom `zoom`: device pixel (i, j) shows the document pixel
    /// `anchor + ⌊(i + ½) / zoom⌋` (the sampling rule of [`Self::render_region`], whose grid starts at
    /// the region). Only document pixels inside `clip` are drawn, the rest stays transparent. The
    /// grid does not depend on the part rendered, so parts of one view fit together without seams
    /// and equal their pixels in a whole render (UI canvas tiles, BA-098).
    #[allow(clippy::too_many_arguments)] // Dokument, Lage, Gitter, Ausschnitt
    pub fn render_device(
        &self,
        doc: &Document,
        page: usize,
        frame: usize,
        clip: IRect,
        anchor: (i32, i32),
        zoom: f64,
        dev: IRect,
    ) -> RenderOutput {
        let zoom = if zoom.is_finite() && zoom > 0.0 {
            zoom.clamp(1.0 / 256.0, 256.0)
        } else {
            1.0
        };
        let w = dev.width().min(32768) as i32;
        let h = dev.height().min(32768) as i32;
        let sample = |a: i32, i: i32| a + (((i as f64) + 0.5) / zoom).floor() as i32;
        let region = if w > 0 && h > 0 {
            IRect::new(
                sample(anchor.0, dev.left),
                sample(anchor.1, dev.top),
                sample(anchor.0, dev.left + w - 1) + 1,
                sample(anchor.1, dev.top + h - 1) + 1,
            )
            .intersect(&clip)
        } else {
            IRect::default()
        };
        let mut out = RgbaImage::new(IRect::new(0, 0, w.max(0), h.max(0)))
            .unwrap_or_else(|_| RgbaImage::empty_at(0, 0));
        if region.is_empty() || out.width() == 0 {
            return RenderOutput {
                image: out,
                report: RenderReport::default(),
            };
        }
        let (grid, base_img, report) = self.compose_rect(doc, page, frame, region, None);
        let ow = out.width();
        let xs: Vec<i32> = (0..w).map(|i| sample(anchor.0, dev.left + i)).collect();
        let ys: Vec<i32> = (0..h).map(|j| sample(anchor.1, dev.top + j)).collect();
        use rayon::prelude::*;
        out.data_mut()
            .par_chunks_mut(ow)
            .enumerate()
            .for_each(|(j, row)| {
                let y = ys[j];
                let inside_y = y >= region.top && y < region.bottom;
                for (i, p) in row.iter_mut().enumerate() {
                    let x = xs[i];
                    *p = if inside_y && x >= region.left && x < region.right {
                        match &grid {
                            Some(g) => g.get(x, y),
                            None => base_img.get(x, y).unwrap_or([0, 0, 0, 0]),
                        }
                    } else {
                        [0, 0, 0, 0]
                    };
                }
            });
        RenderOutput { image: out, report }
    }

    /// Like [`Self::render_region`], with the pixels of one bitmap replaced (live paint preview,
    /// BA-039). `None` when the bitmap is not drawn as part of the page (e.g. it is not on this
    /// page or lies inside a symbol); the caller then shows its own preview.
    pub fn render_region_with(
        &self,
        doc: &Document,
        page: usize,
        frame: usize,
        region: IRect,
        zoom: f64,
        over: &PixelOverride,
    ) -> Option<RenderOutput> {
        let (_, ancestors) = find_bitmap(doc, page, over.id)?;
        let o = Override {
            id: over.id.0,
            image: Arc::clone(&over.image),
            ancestors: ancestors.into_iter().collect(),
        };
        Some(self.region_of(doc, page, frame, region, zoom, Some(Arc::new(o))))
    }

    fn region_of(
        &self,
        doc: &Document,
        page: usize,
        frame: usize,
        region: IRect,
        zoom: f64,
        over: Option<Arc<Override>>,
    ) -> RenderOutput {
        let zoom = if zoom.is_finite() && zoom > 0.0 {
            zoom.clamp(1.0 / 256.0, 256.0)
        } else {
            1.0
        };
        let (grid, base_img, report) = self.compose_rect(doc, page, frame, region, over);
        if (zoom - 1.0).abs() < 1e-12 {
            let mut img = match &grid {
                Some(g) => g.assemble(region),
                None => base_img,
            };
            img.set_origin(0, 0);
            return RenderOutput { image: img, report };
        }
        let w = ((region.width() as f64) * zoom).ceil().clamp(0.0, 32768.0) as i32;
        let h = ((region.height() as f64) * zoom).ceil().clamp(0.0, 32768.0) as i32;
        let mut out =
            RgbaImage::new(IRect::new(0, 0, w, h)).unwrap_or_else(|_| RgbaImage::empty_at(0, 0));
        let ow = out.width();
        let src = &base_img;
        let xs: Vec<i32> = (0..w)
            .map(|i| region.left + (((i as f64) + 0.5) / zoom).floor() as i32)
            .collect();
        let ys: Vec<i32> = (0..h)
            .map(|j| region.top + (((j as f64) + 0.5) / zoom).floor() as i32)
            .collect();
        use rayon::prelude::*;
        if ow > 0 {
            out.data_mut()
                .par_chunks_mut(ow)
                .enumerate()
                .for_each(|(j, row)| {
                    let y = ys[j];
                    match &grid {
                        Some(g) => {
                            let inside_y = y >= region.top && y < region.bottom;
                            for (i, p) in row.iter_mut().enumerate() {
                                let x = xs[i];
                                *p = if inside_y && x >= region.left && x < region.right {
                                    g.get(x, y)
                                } else {
                                    [0, 0, 0, 0]
                                };
                            }
                        }
                        None => {
                            for (i, p) in row.iter_mut().enumerate() {
                                *p = src.get(xs[i], y).unwrap_or([0, 0, 0, 0]);
                            }
                        }
                    }
                });
        }
        RenderOutput { image: out, report }
    }

    /// Ein Symbol im Status `frame` in Symbolkoordinaten (Grenzen = Inhalt).
    pub fn render_symbol(
        &self,
        doc: &Document,
        symbol: &Arc<Symbol>,
        frame: usize,
    ) -> RenderOutput {
        let mut report = RenderReport::default();
        let st = State {
            map: PointMap::identity(),
            frame,
            symbols: vec![symbol.id],
            over: None,
        };
        let mut cv = Canvas::growable();
        self.draw_symbol_layers(doc, &mut cv, symbol, &st, &mut report);
        RenderOutput {
            image: cv.img,
            report,
        }
    }

    /// Aussehen eines einzelnen Elements (nach Effekten, vor Deckkraft/Mischmodus) in
    /// Dokumentkoordinaten, abgebildet mit `map`.
    pub fn render_element(
        &self,
        doc: &Document,
        e: &Arc<Element>,
        map: &PointMap,
        frame: usize,
    ) -> RenderOutput {
        let mut report = RenderReport::default();
        let st = State {
            map: map.clone(),
            frame,
            symbols: Vec::new(),
            over: None,
        };
        let image = match self.look(doc, e, &st, &mut report) {
            Some(l) => (*l.image).clone(),
            None => RgbaImage::empty_at(0, 0),
        };
        RenderOutput { image, report }
    }

    // ------------------------------------------------------------------ Ebenen

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn draw_layer(
        &self,
        doc: &Document,
        cv: &mut Canvas,
        layer: &Layer,
        depth: u32,
        state: ElementState,
        st: &State,
        rep: &mut RenderReport,
    ) {
        if layer.kind == LayerKind::Web {
            return;
        }
        let vis = layer
            .visibility
            .get(st.frame)
            .or(layer.visibility.first())
            .copied()
            .unwrap_or(true);
        if !vis {
            return;
        }
        let per_frame = !layer.shared_across_frames
            && (depth == 0 || (depth == 1 && state == ElementState::MasterPageContent));
        if per_frame && st.frame > 0 && st.frame < layer.cels.len() {
            // Animationsinstanzen früherer Status laufen über ANNi Status weiter (13 §7.3, Laufzeit T6:
            // die Instanz steht nur im Cel ihres Startstatus). Gezeichnet unter dem Inhalt des Status (A).
            for c in 0..st.frame {
                let k = st.frame - c;
                for e in layer.cels[c].elements.iter().rev() {
                    if let ElementKind::Instance(i) = &e.kind
                        && i.anim.frames as usize > k
                        && doc
                            .symbol_by_id(i.symbol_id)
                            .is_some_and(|(_, s)| s.kind() == SymbolKind::Animation)
                    {
                        let st2 = State {
                            frame: k,
                            ..st.clone()
                        };
                        self.draw_element(doc, cv, e, depth, &st2, rep);
                    }
                }
            }
        }
        let cel = layer
            .cel_at(st.frame, depth, state)
            .or_else(|| layer.cels.first());
        let Some(cel) = cel else { return };
        if !cel.visible {
            return;
        }
        // Elemente eines Status-Cels beginnen in diesem Status (Animationsstatus k = 0).
        let local = if per_frame && st.frame < layer.cels.len() {
            State {
                frame: 0,
                ..st.clone()
            }
        } else {
            st.clone()
        };
        for e in cel.elements.iter().rev() {
            self.draw_element(doc, cv, e, depth, &local, rep);
        }
    }

    fn draw_symbol_layers(
        &self,
        doc: &Document,
        cv: &mut Canvas,
        sym: &Arc<Symbol>,
        st: &State,
        rep: &mut RenderReport,
    ) {
        for slot in &sym.root.layers {
            match slot {
                LayerSlot::Own(l) => self.draw_layer(doc, cv, l, 0, ElementState::Normal, st, rep),
                LayerSlot::Shared(r) => {
                    if let Some(l) = doc.shared_layer(&r.id).and_then(|d| d.layer.as_ref()) {
                        self.draw_layer(doc, cv, l, 0, ElementState::SharedLayer, st, rep);
                    }
                }
                LayerSlot::MasterPage(_) => rep.miss("MPLv"),
                LayerSlot::Raw(_) => rep.miss("LYLv:raw"),
            }
        }
    }

    // ------------------------------------------------------------------ Elemente

    pub(crate) fn draw_element(
        &self,
        doc: &Document,
        cv: &mut Canvas,
        e: &Arc<Element>,
        depth: u32,
        st: &State,
        rep: &mut RenderReport,
    ) {
        if !e.common.visible {
            return;
        }
        match &e.kind {
            ElementKind::WebObject(_) => return,
            ElementKind::Raw(_) => {
                rep.miss("raw-element");
                return;
            }
            ElementKind::SubLayer(l) => {
                self.draw_layer(doc, cv, l, depth + 1, ElementState::Normal, st, rep);
                return;
            }
            ElementKind::Path(p) if p.compat_proxy => return,
            ElementKind::Group(g) if self.draw_group_direct(doc, e, g) => {
                for c in g.children.iter().rev() {
                    self.draw_element(doc, cv, c, depth, st, rep);
                }
                return;
            }
            _ => {}
        }
        if let Some(look) = self.look(doc, e, st, rep) {
            let op = match look.extra_opacity {
                Some(x) => {
                    Opacity(fp_color::round::mul255(x as u32, e.common.opacity.0 as u32) as u8)
                }
                None => e.common.opacity,
            };
            // Mischmodi von Elementen in Symbolinstanzen wirken wie Normal (Korpus: Instanz-Caches
            // +185.000 exakte Pixel, Menu_Bar_07 15.984/15.984; A für die Ursache).
            let blend = if !st.symbols.is_empty() && !self.options.symbol_blend_modes {
                BlendMode(0)
            } else {
                e.common.blend
            };
            cv.draw_look(&look.image, op, blend);
            rep.drawn += 1;
        }
    }

    /// Kinder einer Gruppe direkt auf die Leinwand (keine Isolation)?
    fn draw_group_direct(&self, doc: &Document, e: &Element, g: &fp_model::GroupElem) -> bool {
        if g.kind.is_mask() && g.mask_flags.enabled && g.effective_mask().is_some() {
            return false;
        }
        if has_visible_effects(e) {
            return false;
        }
        if e.common.opacity.0 != 255 || e.common.blend.0 != 0 {
            return false;
        }
        if e.cache.is_some() && self.cache_allowed(doc, e) {
            return false;
        }
        !self.options.isolate_groups || g.children.iter().any(|c| c.common.blend.0 != 0)
    }

    fn cache_allowed(&self, doc: &Document, e: &Element) -> bool {
        let pol = self.options.cache_policy;
        match &e.kind {
            ElementKind::Path(_) => pol == CachePolicy::All,
            ElementKind::Text(_) => pol != CachePolicy::None,
            ElementKind::Group(_) | ElementKind::CompoundShape(_) => {
                matches!(pol, CachePolicy::All | CachePolicy::Original)
            }
            ElementKind::Instance(i) => match pol {
                CachePolicy::All => true,
                CachePolicy::Original => {
                    let anim = doc
                        .symbol_by_id(i.symbol_id)
                        .map(|(_, s)| s.kind() == SymbolKind::Animation)
                        .unwrap_or(false);
                    doc.format_version() > 9 && !anim
                }
                _ => false,
            },
            _ => false,
        }
    }

    /// Aussehen eines Elements (gecacht).
    pub(crate) fn look(
        &self,
        doc: &Document,
        e: &Arc<Element>,
        st: &State,
        rep: &mut RenderReport,
    ) -> Option<Look> {
        let container = matches!(e.kind, ElementKind::Group(_) | ElementKind::Instance(_));
        let frame = if container { st.frame } else { usize::MAX };
        let symbol = match &e.kind {
            ElementKind::Instance(i) => doc.symbol_by_id(i.symbol_id).map(|(_, s)| Arc::clone(s)),
            _ => None,
        };
        let budget = if st.overridden(e) {
            0
        } else {
            self.options.cache_budget_pixels
        };
        if budget > 0
            && let Some((look, r)) = self.with_cache(|c| c.get(e, frame, &st.map, symbol.as_ref()))
        {
            rep.merge(&r);
            return look;
        }
        let mut r = RenderReport::default();
        let look = self.compute_look(doc, e, st, symbol.as_ref(), &mut r);
        rep.merge(&r);
        if budget > 0 {
            self.with_cache(|c| c.put(e, frame, &st.map, symbol.as_ref(), look.clone(), r, budget));
        }
        look
    }

    fn compute_look(
        &self,
        doc: &Document,
        e: &Arc<Element>,
        st: &State,
        symbol: Option<&Arc<Symbol>>,
        rep: &mut RenderReport,
    ) -> Option<Look> {
        let effects: Vec<EffectEntry> = e
            .common
            .effects
            .as_ref()
            .map(convert::effect_entries)
            .unwrap_or_default();
        let has_fx = effects.iter().any(|x| x.visible);
        // Gespeicherter Render-Cache (enthält Effekte).
        if let Some(c) = &e.cache
            && !st.overridden(e)
            && self.cache_allowed(doc, e)
            && let Some(img) = convert::image_of_node(c)
            && let Some(placed) = self.place_cached(&img, &st.map, e, rep)
        {
            rep.from_file_cache += 1;
            return Some(Look {
                image: Arc::new(placed),
                extra_opacity: None,
            });
        }
        let mut extra_opacity = None;
        let base: Option<RgbaImage> = match &e.kind {
            ElementKind::Bitmap(_) if st.over.as_ref().is_some_and(|o| o.id == e.id.0) => {
                let o = st.over.as_ref()?;
                self.place_bitmap(&o.image, &st.map, rep)
            }
            ElementKind::Bitmap(b) => {
                if has_fx
                    && self.options.cache_policy != CachePolicy::None
                    && let Some(res) = &b.effect_result
                    && let Some(img) = convert::image_of_node(res)
                {
                    rep.from_file_cache += 1;
                    return self.place_cached(&img, &st.map, e, rep).map(|i| Look {
                        image: Arc::new(i),
                        extra_opacity: None,
                    });
                }
                match convert::image_of(&b.image) {
                    Some(img) => self.place_bitmap(&img, &st.map, rep),
                    None => {
                        rep.miss("IMGv:tiles");
                        None
                    }
                }
            }
            ElementKind::Path(p) => self.path_layer(p, &st.map, rep),
            ElementKind::Text(t) => self.text_layer(doc, t, &st.map, rep),
            ElementKind::CompoundShape(c) => match c.result.as_ref().map(|r| &r.kind) {
                Some(ElementKind::Path(p)) => self.path_layer(p, &st.map, rep),
                _ => {
                    rep.miss("CSGv");
                    None
                }
            },
            ElementKind::Group(g) => self.group_layer(doc, e, g, st, rep),
            ElementKind::Instance(i) => {
                let r = self.instance_layer(doc, i, symbol, st, rep);
                if let Some((img, op)) = r {
                    extra_opacity = op;
                    Some(img)
                } else {
                    None
                }
            }
            ElementKind::WebObject(_) | ElementKind::SubLayer(_) | ElementKind::Raw(_) => None,
        };
        let mut img = base?;
        if has_fx {
            if self.options.effects {
                // Farbe durchsichtiger Pixel im Objektpuffer (die 3×3-Filter lesen sie mit).
                // Gemessen (RE-FX, FX-4): Vektorpfade weiß (Ränder suchen, Weichzeichnen, Schärfen
                // am Vektor mit Rundung nur so exakt, schwarz: bis 255 Stufen), Bitmaps behalten
                // ihre gespeicherten Farben (schwarz: bis 20 Stufen), Text schwarz.
                let vector = matches!(e.kind, ElementKind::Path(_))
                    || matches!(&e.kind, ElementKind::CompoundShape(c) if matches!(c.result.as_ref().map(|r| &r.kind), Some(ElementKind::Path(_))));
                let clear = match e.kind {
                    ElementKind::Bitmap(_) => None,
                    _ if vector => Some([255, 255, 255, 0]),
                    _ => Some([0, 0, 0, 0]),
                };
                if let Some(clear) = clear {
                    for p in img.data_mut() {
                        if p[3] == 0 {
                            *p = clear;
                        }
                    }
                }
                // re 30 §4 (re 28 D3): text buffer from the content rectangle ± 15 %
                let text_buf = match &e.kind {
                    ElementKind::Text(t) => st.map.integer_translation().and_then(|(dx, dy)| {
                        text_buffer_rect(self.fonts(), doc, t).map(|r| {
                            IRect::new(r.left + dx, r.top + dy, r.right + dx, r.bottom + dy)
                        })
                    }),
                    _ => None,
                };
                img = effect_buffer(img, &e.kind, &effects, text_buf);
                let res = apply_chain(&img, &effects, &self.options.effect_context);
                for i in &res.issues {
                    rep.miss(format!("effect:{i}"));
                }
                rep.effect_chains += 1;
                img = res.image;
                // re 30 §4.2: the cache of a text is cut to alpha > 0 after the effects (pixelRect
                // follows from the cache alpha)
                if text_buf.is_some() {
                    let a = img.alpha_bounds();
                    if !a.is_empty() && a != img.bounds() {
                        img = img.cropped(a);
                    }
                }
            } else {
                rep.miss("effects");
            }
        }
        Some(Look {
            image: Arc::new(img),
            extra_opacity,
        })
    }
}

/// Puffergrenzen der Effektkette wie im Original (FX-3, `re` Gruppe 3; die Farbe durchsichtiger
/// Pixel setzt der Aufrufer, FX-4). Nur wenn die Kette einen Bitmap-Filter enthält (dort zählen die
/// Grenzen: Randwiederholung der 3×3-Faltung, Mitte von Radial/Zoom, Gauß > 200, Randnormierung
/// des Gauß in Unscharf maskieren; Schatten/Glühen/Abschrägung bleiben unberührt):
/// - **Bitmap**: Puffer = Pixelgrenzen (Alpha > 0), gemessen an Weichzeichnen (keine Ausdehnung),
///   Gauß > 200 (Mittelwert über das zugeschnittene Rechteck) und Radial (Mitte).
/// - **Text**: Puffer nach [`text_effect_buffer`] (re 30 §4.2, Inhaltsrechteck ± 15 %), mindestens
///   die Pixelgrenzen; ohne Satzangaben wie bisher Pixelgrenzen + 15 px (rechts 16 px).
/// - **Pfad**: Puffer = Pfadgrenzen (unverändert).
fn effect_buffer(
    img: RgbaImage,
    kind: &ElementKind,
    effects: &[EffectEntry],
    text_buf: Option<IRect>,
) -> RgbaImage {
    let has_filter = effects
        .iter()
        .any(|x| x.visible && matches!(x.effect, fp_effects::LiveEffect::Filter(_)));
    if !has_filter {
        return img;
    }
    let a = img.alpha_bounds();
    if a.is_empty() {
        return img;
    }
    match kind {
        ElementKind::Bitmap(_) if a != img.bounds() => img.cropped(a),
        ElementKind::Text(_) => {
            let want = match text_buf {
                Some(b) => b.union(&a),
                None => IRect::new(a.left - 15, a.top - 15, a.right + 16, a.bottom + 15),
            };
            img.cropped(a).extended_to(want).unwrap_or(img)
        }
        _ => img,
    }
}

/// Default metrics for [`text_effect_buffer`] (Arial, the measuring font of re 28 D3): ascent of
/// the text frame as a share of the size (re 09 §5.3) and `head.yMax / unitsPerEm` (2060/2048).
pub const TEXT_BUFFER_ARIAL: (f64, f64) = (0.857986, 2060.0 / 2048.0);

/// Content rectangle B of a single-line point text (re 30 §4.2), unrounded except where the rule
/// rounds: left = raw-left, right = raw-left + advance (sum of rounded advances); top =
/// floor(baseline − yMax·s) with baseline = raw-top + ceil(asc·s); bottom = ceil(raw-top + 1.041·s).
pub fn text_content_box(
    raw_left: f64,
    raw_top: f64,
    advance: f64,
    size: f64,
    ascent: f64,
    y_max: f64,
) -> [f64; 4] {
    let baseline = raw_top + (ascent * size - 1e-9).ceil();
    [
        raw_left,
        (baseline - y_max * size).floor(),
        raw_left + advance,
        (raw_top + 1.041 * size - 1e-9).ceil(),
    ]
}

/// Effect input buffer from the content rectangle B (re 30 §4.2): B rounded outward to whole
/// pixels, grown by 0.15·width left and right and 0.15·height top and bottom, rounded outward
/// again. Effect expansions (none for the filter chain) would apply before the last rounding.
pub fn text_buffer_from_box(b: [f64; 4]) -> IRect {
    let (l, t, r, bt) = (
        (b[0] + 1e-9).floor(),
        (b[1] + 1e-9).floor(),
        (b[2] - 1e-9).ceil(),
        (b[3] - 1e-9).ceil(),
    );
    let (w, h) = (r - l, bt - t);
    IRect::new(
        (l - 0.15 * w).floor() as i32,
        (t - 0.15 * h).floor() as i32,
        (r + 0.15 * w).ceil() as i32,
        (bt + 0.15 * h).ceil() as i32,
    )
}

/// Input buffer of the live effects of a text (re 30 §4.2) with the Arial metrics of the
/// measurement: [`text_content_box`] → [`text_buffer_from_box`]. Example 72 pt "Fx Ag" at 40,60,
/// advance 188: B = 40,49–228,135 → buffer 11,36–257,148.
pub fn text_effect_buffer(raw_left: f64, raw_top: f64, advance: f64, size: f64) -> IRect {
    let (asc, ymax) = TEXT_BUFFER_ARIAL;
    text_buffer_from_box(text_content_box(
        raw_left, raw_top, advance, size, asc, ymax,
    ))
}

/// Content rectangle B of a text with a fixed frame (re 30 §4.2, `fx_text`: B' = 40,54–150,104,
/// "bottom = frame bottom"): the bottom edge lies on the frame bottom. One line (A, single
/// measurement): the point-text box keeps its height and moves down onto the frame bottom; several
/// lines: only the bottom grows to the frame bottom. The horizontal extent stays the advance box –
/// the 6 px of `fx_text` over the regular 104 come from the bold advances (110), not from the frame.
pub fn fixed_frame_box(b: &mut [f64; 4], frame_bottom: f64, lines: usize) {
    let dy = frame_bottom - b[3];
    if dy <= 0.0 {
        return;
    }
    if lines <= 1 {
        b[1] += dy;
    }
    b[3] = frame_bottom;
}

/// [`text_effect_buffer`] for a text object: advance = widest line, size = largest glyph, metrics
/// of the first glyph's font. Several lines (A, re 30 §4.3): bottom moves down by the distance of
/// the last baseline from the first.
pub fn text_buffer_rect(
    fonts: &fp_text::FontDb,
    doc: &Document,
    t: &fp_model::TextElem,
) -> Option<IRect> {
    let block = convert::text_block(doc, t);
    let lay = fp_text::layout(&block, fonts, &fp_text::LayoutOptions::default());
    let advance = lay.lines.iter().map(|l| l.width).fold(0.0f64, f64::max);
    let size = lay.glyphs.iter().map(|g| g.size).fold(0.0f64, f64::max);
    if !(advance > 0.0 && size > 0.0) {
        return None;
    }
    let (asc, ymax) = lay
        .glyphs
        .first()
        .map(|g| {
            (
                g.font.ascent(),
                g.font.bbox()[3] as f64 / g.font.units_per_em().max(1) as f64,
            )
        })
        .unwrap_or(TEXT_BUFFER_ARIAL);
    let mut b = text_content_box(
        t.bounds.left as f64,
        t.bounds.top as f64,
        advance,
        size,
        asc,
        ymax,
    );
    let lines = lay.lines.len();
    if let (Some(first), Some(last)) = (lay.lines.first(), lay.lines.last()) {
        b[3] += (last.baseline - first.baseline).max(0.0);
    }
    if t.frame_type == Some(1) {
        fixed_frame_box(&mut b, t.bounds.bottom as f64, lines);
    }
    Some(text_buffer_from_box(b))
}

impl Renderer {
    /// Gespeicherten Cache mit der Abbildung `map` platzieren: ganzzahlige Verschiebung direkt;
    /// sonst neu rechnen lassen (`None`), wenn das Element selbst gerastert werden kann, oder
    /// resampeln (Näherung).
    fn place_cached(
        &self,
        img: &RgbaImage,
        map: &PointMap,
        e: &Element,
        rep: &mut RenderReport,
    ) -> Option<RgbaImage> {
        if let Some((dx, dy)) = map.integer_translation() {
            return Some(img.translated(dx, dy));
        }
        // 9-Slice-Text: nur verschieben (13 §7.8).
        if let ElementKind::Text(t) = &e.kind
            && map.is_nine_slice()
        {
            let a = text_anchor(t);
            if let Some((dx, dy)) = map.translate_only(a).integer_translation() {
                return Some(img.translated(dx, dy));
            }
        }
        if !self.options.rasterize {
            rep.miss(format!("{}:transform", kind_tag(e)));
            return None;
        }
        match &e.kind {
            ElementKind::Path(_)
            | ElementKind::Text(_)
            | ElementKind::Group(_)
            | ElementKind::CompoundShape(_)
            | ElementKind::Instance(_) => None,
            _ => self.place_bitmap(img, map, rep),
        }
    }

    /// Bitmap mit der Abbildung platzieren (Verschiebung, 9-Slice oder affine Abtastung).
    fn place_bitmap(
        &self,
        img: &RgbaImage,
        map: &PointMap,
        rep: &mut RenderReport,
    ) -> Option<RgbaImage> {
        if let Some((dx, dy)) = map.integer_translation() {
            return Some(img.translated(dx, dy));
        }
        if !self.options.rasterize {
            rep.miss("IMGv:transform");
            return None;
        }
        rep.approx("bitmap-resampled");
        if map.is_nine_slice() {
            return Some(self.nine_slice_bitmap(img, map));
        }
        let m = map.as_affine().unwrap_or(Matrix3::IDENTITY);
        Some(transform(img, &m, self.options.sampler, None))
    }

    fn nine_slice_bitmap(&self, img: &RgbaImage, map: &PointMap) -> RgbaImage {
        // Stufen bis zur (ersten) 9-Slice-Stufe vorher anwenden, danach die übrigen affin.
        let mut cur = img.clone();
        for s in &map.stages {
            cur = match s {
                Stage::Affine(m) => match (PointMap { stages: vec![*s] }).integer_translation() {
                    Some((dx, dy)) => cur.translated(dx, dy),
                    None => transform(&cur, m, self.options.sampler, None),
                },
                Stage::Nine { x, y } => {
                    let b = cur.bounds();
                    let (x0, x1) = (x.apply(b.left as f64), x.apply(b.right as f64));
                    let (y0, y1) = (y.apply(b.top as f64), y.apply(b.bottom as f64));
                    let nw = (x1 - x0).round().clamp(0.0, 1.0e5) as u32;
                    let nh = (y1 - y0).round().clamp(0.0, 1.0e5) as u32;
                    let inset = |ax: &crate::nine::AxisMap, a0: i32, a1: i32| -> (u32, u32) {
                        if !ax.nine {
                            return (0, 0);
                        }
                        let g1 = ax.lo + ax.m1;
                        let g2 = ax.hi - ax.m2;
                        let l = (g1 - a0 as f64).round().clamp(0.0, (a1 - a0) as f64) as u32;
                        let r = (a1 as f64 - g2).round().clamp(0.0, (a1 - a0) as f64) as u32;
                        (l, r)
                    };
                    let (il, ir) = inset(x, b.left, b.right);
                    let (it, ib) = inset(y, b.top, b.bottom);
                    let mut out =
                        nine_slice_scale(&cur, [il, it, ir, ib], nw, nh, self.options.sampler);
                    out.set_origin(convert::round_coord(x0), convert::round_coord(y0));
                    out
                }
            };
        }
        cur
    }

    fn path_layer(
        &self,
        p: &fp_model::PathElem,
        map: &PointMap,
        rep: &mut RenderReport,
    ) -> Option<RgbaImage> {
        if !self.options.rasterize {
            rep.miss("PTHv");
            return None;
        }
        let vp = convert::vector_path(p, map);
        let layer = fp_vector::render_path_layer(&vp)?;
        for m in &layer.missing {
            rep.miss(format!("PTHv:{m:?}"));
        }
        rep.rasterized_paths += 1;
        Some(layer.image)
    }

    fn text_layer(
        &self,
        doc: &Document,
        t: &fp_model::TextElem,
        map: &PointMap,
        rep: &mut RenderReport,
    ) -> Option<RgbaImage> {
        if !self.options.rasterize {
            rep.miss("TXTv");
            return None;
        }
        let block = convert::text_block(doc, t);
        let mut lay = fp_text::layout(&block, self.fonts(), &fp_text::LayoutOptions::default());
        if !lay.missing_fonts.is_empty() {
            rep.approx("text-font-substituted");
        }
        // Abbildung: erst Text-MTXv, dann die Elternabbildung (bei 9-Slice nur verschieben).
        let mut m = PointMap::identity();
        if let Some(tm) = &t.transform {
            m = m.then_affine(tm);
        }
        let parent = if map.is_nine_slice() {
            map.translate_only(text_anchor(t))
        } else {
            map.clone()
        };
        let full = PointMap {
            stages: m
                .stages
                .iter()
                .chain(parent.stages.iter())
                .copied()
                .collect(),
        };
        if let Some(a) = full.as_affine()
            && !a.is_identity()
        {
            let aff = fp_text::raster::Affine {
                a: a.m[0][0] as f64,
                b: a.m[0][1] as f64,
                c: a.m[1][0] as f64,
                d: a.m[1][1] as f64,
                e: a.m[2][0] as f64,
                f: a.m[2][1] as f64,
            };
            for g in &mut lay.glyphs {
                g.transform = g.transform.then(&aff);
            }
            for d in &mut lay.decorations {
                for q in &mut d.quad {
                    *q = aff.apply(*q);
                }
            }
        }
        rep.rasterized_texts += 1;
        Some(fp_text::render::render(&lay, None))
    }

    fn group_layer(
        &self,
        doc: &Document,
        e: &Element,
        g: &fp_model::GroupElem,
        st: &State,
        rep: &mut RenderReport,
    ) -> Option<RgbaImage> {
        let mut cv = Canvas::growable();
        for c in g.children.iter().rev() {
            self.draw_element(doc, &mut cv, c, 0, st, rep);
        }
        let mut img = cv.img;
        let masked = g.kind.is_mask() && g.mask_flags.enabled;
        if masked && let Some(me) = g.effective_mask() {
            // Pfadumriss mit „Füllung und Pinselstrich anzeigen“ (Flags 0x01 und 0x10, re 26 §3.1)
            let show = g.kind == GroupKind::VectorMask && !g.mask_flags.hide_attrs;
            let mask = self.mask_values(doc, e, g, me, show, st, rep);
            let b = img.bounds();
            let w = img.width();
            for (i, p) in img.data_mut().iter_mut().enumerate() {
                if p[3] == 0 {
                    continue;
                }
                let x = b.left + (i % w.max(1)) as i32;
                let y = b.top + (i / w.max(1)) as i32;
                let m = mask.as_ref().map(|(mb, mw, data)| {
                    if mb.contains(x, y) {
                        data[(y - mb.top) as usize * mw + (x - mb.left) as usize]
                    } else {
                        0
                    }
                });
                let m = m.unwrap_or(0);
                p[3] = ((p[3] as u32 * m as u32 + 127) / 255) as u8;
            }
            if show {
                // Füllung der Maske → Inhalt (skalar) → Strich der Maske (MMX); Deckkraft und
                // Mischmodus der Maske zählen nicht (re 26 §3.2/§3.3)
                let (under, over) = self.mask_attr_layers(doc, me, st, rep);
                let mut z = Canvas::growable();
                if let Some(u) = &under {
                    z.draw(u, Opacity(255), BlendMode(0));
                }
                z.draw_variant(&img, Opacity(255), BlendMode(0), OverVariant::Scalar);
                if let Some(o) = &over {
                    z.draw(o, Opacity(255), BlendMode(0));
                }
                img = z.img;
            }
        }
        if img.is_empty() {
            return None;
        }
        Some(img)
    }

    /// Sichtbare Attribute einer Maske mit „Füllung und Pinselstrich anzeigen“: (unten, oben). Pfad:
    /// Füllung (mit weicher Kante, Farbalpha, Verlauf, Textur) unten, Strich oben. Andere
    /// Maskenelemente (Bitmap, Gruppe): das ganze Element unten (re 26 §3.2).
    fn mask_attr_layers(
        &self,
        doc: &Document,
        me: &Arc<Element>,
        st: &State,
        rep: &mut RenderReport,
    ) -> (Option<RgbaImage>, Option<RgbaImage>) {
        match &me.kind {
            ElementKind::Path(p) => {
                if !self.options.rasterize {
                    rep.miss("GRPv:mask");
                    return (None, None);
                }
                let vp = convert::vector_path(p, &st.map);
                let fill = vp.fill.is_some().then(|| fp_vector::VectorPath {
                    stroke: None,
                    ..vp.clone()
                });
                let stroke = vp.stroke.is_some().then(|| fp_vector::VectorPath {
                    fill: None,
                    ..vp.clone()
                });
                let layer = |v: Option<fp_vector::VectorPath>| {
                    v.and_then(|v| fp_vector::render_path_layer(&v))
                        .map(|l| l.image)
                        .filter(|i| !i.is_empty())
                };
                (layer(fill), layer(stroke))
            }
            _ => (
                self.look(doc, me, st, rep).map(|l| (*l.image).clone()),
                None,
            ),
        }
    }

    /// Maskenwerte (Grenzen, Breite, Daten) einer Maskengruppe (07 §7.5, Laufzeit T3; re 26 §3.2).
    /// Pfad: Fülldeckung der Geometrie (Farbe, Farbalpha und Strich zählen nicht); die weiche Kante
    /// wirkt nur ohne „anzeigen“ (`show`), sonst beschneidet die Geometrie mit Kantenglättung.
    #[allow(clippy::type_complexity, clippy::too_many_arguments)]
    fn mask_values(
        &self,
        doc: &Document,
        _group: &Element,
        g: &fp_model::GroupElem,
        me: &Arc<Element>,
        show: bool,
        st: &State,
        rep: &mut RenderReport,
    ) -> Option<(IRect, usize, Vec<u8>)> {
        // Graustufen nur bei Bitmapmasken (`MNAb`); Pfadumriss nimmt das Alpha (re 26 §3.2)
        let gray = g.kind == GroupKind::BitmapMask && g.mask_flags.hide_attrs;
        let img: Option<RgbaImage> = match (&me.kind, g.kind) {
            (ElementKind::Path(p), GroupKind::VectorMask)
            | (ElementKind::Path(p), GroupKind::BitmapMask) => {
                if !self.options.rasterize {
                    rep.miss("GRPv:mask");
                    return None;
                }
                let mut vp = convert::fill_only(convert::vector_path(p, &st.map));
                if show
                    && let Some(f) = &mut vp.fill
                    && matches!(f.edge, fp_vector::FillEdge::Feather(_))
                {
                    f.edge = fp_vector::FillEdge::AntiAliased;
                }
                fp_vector::render_path_layer(&vp).map(|l| l.image)
            }
            (ElementKind::Bitmap(b), _) => {
                let src = b
                    .effect_result
                    .as_ref()
                    .and_then(|r| convert::image_of_node(r))
                    .or_else(|| convert::image_of(&b.image));
                let src = src.and_then(|i| self.place_bitmap(&i, &st.map, rep));
                src.map(|mut i| {
                    if gray {
                        for p in i.data_mut() {
                            let lum =
                                (p[0] as u32 * 299 + p[1] as u32 * 587 + p[2] as u32 * 114) / 1000;
                            p[3] = ((p[3] as u32 * (255 - lum) + 127) / 255) as u8;
                        }
                    }
                    i
                })
            }
            _ => {
                rep.approx("mask-generic");
                self.look(doc, me, st, rep).map(|l| (*l.image).clone())
            }
        };
        let img = img?;
        let b = img.bounds();
        Some((b, img.width(), img.data().iter().map(|p| p[3]).collect()))
    }

    fn instance_layer(
        &self,
        doc: &Document,
        i: &fp_model::InstanceElem,
        symbol: Option<&Arc<Symbol>>,
        st: &State,
        rep: &mut RenderReport,
    ) -> Option<(RgbaImage, Option<u8>)> {
        let Some(sym) = symbol else {
            rep.miss("ALSv:symbol-fehlt");
            return None;
        };
        if st.symbols.contains(&sym.id) || st.symbols.len() > 32 {
            rep.miss("ALSv:rekursion");
            return None;
        }
        let mut m = i.transform;
        let mut extra = None;
        let mut frame = sym.shown_frame(st.frame, 0);
        if sym.kind() == SymbolKind::Animation {
            let k = st.frame;
            let af = i.anim.frame(k, sym.root.frame_count)?;
            frame = af.symbol_frame;
            let c = self.symbol_center(doc, sym, rep);
            let local = Matrix3::translation(-c.0, -c.1)
                .then(&Matrix3::scaling(af.scale as f32, af.scale as f32))
                .then(&Matrix3::rotation_degrees(af.rotation))
                .then(&Matrix3::translation(c.0, c.1));
            m = local
                .then(&m)
                .then(&Matrix3::translation(af.offset.x, af.offset.y));
            extra = Some(af.opacity.0);
        }
        let stage = sym
            .nine_slice
            .as_ref()
            .and_then(|ns| nine_slice_stage(ns, &m))
            .unwrap_or(Stage::Affine(m));
        let inner = st.map.prepend(stage);
        let mut syms = st.symbols.clone();
        syms.push(sym.id);
        let st2 = State {
            map: inner,
            frame,
            symbols: syms,
            over: None,
        };
        let mut cv = Canvas::growable();
        self.draw_symbol_layers(doc, &mut cv, sym, &st2, rep);
        if cv.img.is_empty() {
            return None;
        }
        Some((cv.img, extra))
    }

    /// Mitte der Inhaltsgrenzen eines Symbols (Bezugspunkt der Animation, 13 §7.3/T6).
    fn symbol_center(
        &self,
        doc: &Document,
        sym: &Arc<Symbol>,
        rep: &mut RenderReport,
    ) -> (f32, f32) {
        let mut cv = Canvas::growable();
        let st = State {
            map: PointMap::identity(),
            frame: 0,
            symbols: vec![sym.id],
            over: None,
        };
        let mut r = RenderReport::default();
        self.draw_symbol_layers(doc, &mut cv, sym, &st, &mut r);
        if !r.is_complete() {
            rep.approx("anim-center");
        }
        let b = cv.img.alpha_bounds();
        if b.is_empty() {
            return (0.0, 0.0);
        }
        (
            (b.left + b.right) as f32 / 2.0,
            (b.top + b.bottom) as f32 / 2.0,
        )
    }
}

/// Hat das Element sichtbare Live-Effekte?
pub fn has_visible_effects(e: &Element) -> bool {
    e.common.effects.as_ref().is_some_and(|l| l.has_visible())
}

/// Anker eines Texts für 9-Slice (13 §7.8): links → links oben, zentriert → Mitte, rechts → rechts oben.
fn text_anchor(t: &fp_model::TextElem) -> Vec2 {
    let b = t.bounds;
    let align = t
        .runs
        .first()
        .and_then(|r| r.attrs.alignment)
        .or(t.initial.alignment)
        .unwrap_or(0);
    match align {
        1 => Vec2::new(
            ((b.left + b.right) / 2.0) as f64,
            ((b.top + b.bottom) / 2.0) as f64,
        ),
        2 => Vec2::new(b.right as f64, b.top as f64),
        _ => Vec2::new(b.left as f64, b.top as f64),
    }
}

/// Tag-Name einer Elementart (für Berichte).
pub fn kind_tag(e: &Element) -> &'static str {
    match &e.kind {
        ElementKind::Bitmap(_) => "IMGv",
        ElementKind::Path(_) => "PTHv",
        ElementKind::Text(_) => "TXTv",
        ElementKind::Group(_) => "GRPv",
        ElementKind::CompoundShape(_) => "CSGv",
        ElementKind::Instance(_) => "ALSv",
        ElementKind::WebObject(_) => "URLv",
        ElementKind::SubLayer(_) => "LSLv",
        ElementKind::Raw(_) => "RAW",
    }
}

/// The bitmap `id` drawn as part of `page` and the ids of the containers around it (groups,
/// sub-layers, mask groups), outermost first; `None` when no such bitmap is drawn as part of the
/// page (symbol contents are not searched).
pub fn find_bitmap(
    doc: &Document,
    page: usize,
    id: fp_model::ElementId,
) -> Option<(Arc<Element>, Vec<u64>)> {
    fn in_list(list: &[Arc<Element>], id: u64, path: &mut Vec<u64>) -> Option<Arc<Element>> {
        for e in list {
            if e.id.0 == id {
                return matches!(e.kind, ElementKind::Bitmap(_)).then(|| Arc::clone(e));
            }
            path.push(e.id.0);
            let found = match &e.kind {
                ElementKind::Group(g) => in_list(&g.children, id, path).or_else(|| {
                    g.mask
                        .as_ref()
                        .and_then(|m| in_list(std::slice::from_ref(m), id, path))
                }),
                ElementKind::SubLayer(l) => {
                    l.cels.iter().find_map(|c| in_list(&c.elements, id, path))
                }
                _ => None,
            };
            if found.is_some() {
                return found;
            }
            path.pop();
        }
        None
    }
    let mut path = Vec::new();
    doc.resolved_layers(page).iter().find_map(|rl| {
        rl.layer
            .cels
            .iter()
            .find_map(|c| in_list(&c.elements, id.0, &mut path))
            .map(|e| (e, path.clone()))
    })
}
