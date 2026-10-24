//! Konstruktoren für neue Dokumente und Elemente mit Original-Vorgaben.
//!
//! Neu erzeugte Objekte haben leere Rundreise-Daten ([`crate::raw::Extra`] ohne `order`); ein Schreiber
//! benutzt dann die Original-Reihenfolge und schreibt die Felder, die das Original immer schreibt.

use crate::attrs::{Brush, Fill, FillHandles, PathAttrs};
use crate::doc::{Canvas, Document, FileMeta, Grid, Guides, ImageMap, LayerRoot, Page, Presets};
use crate::element::{
    AnimParams, BitmapElem, BitmapNode, ButtonStrings, CommonAttrs, CompoundShapeElem, Contour,
    ContourNode, Element, ElementKind, GroupElem, GroupKind, InstanceElem, MaskFlags, PathElem,
    RectPrimitive,
};
use crate::export::ExportSettings;
use crate::ids::{Guid, IdGen, SymbolId};
use crate::layer::{Layer, LayerKind, LayerSlot};
use crate::raw::Extra;
use crate::symbol::{ButtonOptions, Symbol};
use crate::text::{TextAttrs, TextElem, TextRun};
use crate::web::{SliceBackground, SliceProps, WebObjectElem, WebShape};
use fp_color::Argb;
use fp_geom::{Matrix3, Point, Rect};
use std::sync::Arc;

impl PathAttrs {
    /// Vollfarbige Füllung ohne Strich.
    pub fn solid_fill(color: Argb) -> PathAttrs {
        PathAttrs {
            fill_color: color,
            fill: Some(Fill::solid()),
            ..PathAttrs::default()
        }
    }

    /// Nur Strich (harter Stift mit Durchmesser `width`).
    pub fn stroke_only(color: Argb, width: i32) -> PathAttrs {
        PathAttrs {
            stroke_color: color,
            brush: Some(Brush::basic(width)),
            ..PathAttrs::default()
        }
    }
}

impl Page {
    /// Neue Seite mit Leinwand, einer normalen Ebene („Ebene 1“ in der Sprache der Oberfläche) und der Web-Ebene (zuletzt, ganz oben).
    pub fn new(ids: &mut IdGen, name: &str, width: f32, height: f32, background: Argb) -> Page {
        let mut root = LayerRoot::new(1);
        root.background = background;
        root.layers.push(LayerSlot::Own(Arc::new(Layer::new(
            ids,
            &fp_i18n::tr!("msg-default-layer", n = "1"),
            LayerKind::Normal,
            1,
        ))));
        root.layers.push(LayerSlot::Own(Arc::new(Layer::new(
            ids,
            &fp_i18n::t("msg-default-web-layer"),
            LayerKind::Web,
            1,
        ))));
        root.grid = Some(Grid::default());
        root.guides = Some(Guides::default());
        root.export = Some(Box::new(ExportSettings::new_page_default()));
        Page {
            format_version: Some(10),
            format_minor: Some(1),
            doc_random: Some(random_u32()),
            canvas: Canvas {
                origin: Point::default(),
                width,
                height,
                resolution: 72.0,
                resolution_unit: 0,
            },
            root,
            presets: Presets {
                brushes: Some(Vec::new()),
                fills: Some(Vec::new()),
                gradients: Some(Vec::new()),
                effects: Some(Vec::new()),
                ..Presets::default()
            },
            tool_attrs: Some(Arc::new(PathAttrs::solid_fill(Argb(0xff00_00ff)))),
            image_map: Some(ImageMap {
                name: Some(String::new()),
                extra: Extra::default(),
            }),
            name: Some(name.to_string()),
            is_master: Some(false),
            linked_to_master: Some(false),
            id: Some(Guid::new_random()),
            local_shared_layers: Some(Vec::new()),
            local_shared_layers_extra: Extra::default(),
            compat_layer_copy: None,
            last_export_dir: None,
            last_export_file: None,
            selection: None,
            saved_selections: None,
            saved_selections_extra: Extra::default(),
            jpeg_mask: None,
            printer_data: None,
            legacy_loop: None,
            legacy_export: None,
            extra: Extra::default(),
        }
    }
}

fn random_u32() -> u32 {
    let g = Guid::new_random();
    u32::from_str_radix(&g.0[..8], 16).unwrap_or(0x1234_5678)
}

impl Document {
    /// Neues einseitiges Dokument (Seite „Seite 1“ bzw. `msg-default-page` in der Sprache der Oberfläche, 72 ppi, Formatversion 10 wie CS6).
    pub fn new(width: f32, height: f32, background: Argb) -> Document {
        let mut ids = IdGen::new();
        let page = Page::new(
            &mut ids,
            &fp_i18n::tr!("msg-default-page", n = "1"),
            width,
            height,
            background,
        );
        Document {
            pages: vec![Arc::new(page)],
            current_page: 0,
            root_copy: None,
            root_id: Some(Guid::new_random()),
            single_root: false,
            symbols: Vec::new(),
            symbols_extra: Extra::default(),
            styles: Some(Vec::new()),
            styles_extra: Extra::default(),
            shared_layers: Vec::new(),
            shared_layers_extra: Extra::default(),
            has_shared_layer_list: true,
            ate: None,
            extra_roots: Vec::new(),
            ids,
            file_meta: FileMeta::default(),
        }
    }

    /// Fügt eine neue Seite hinter der aktiven ein (wie `addNewPage`) und liefert ihren Index.
    /// Für geteilte Ebenen mit „auf alle Seiten verteilt“ wird automatisch ein Verweis angelegt (04 §6.1 T2).
    pub fn add_page(&mut self, name: &str) -> usize {
        self.add_page_after(self.current_page, name)
    }

    /// Inserts a new page directly after page `after` (RE spec 70 §2.3: index `after + 1`, the end
    /// when `after` is the last page) with that page's canvas size and background, and returns its
    /// index. `current_page` keeps pointing at the same page (the stored root copy belongs to it).
    pub fn add_page_after(&mut self, after: usize, name: &str) -> usize {
        let (w, h, bg) = self
            .pages
            .get(after)
            .or_else(|| self.active_page())
            .map(|p| (p.canvas.width, p.canvas.height, p.root.background))
            .unwrap_or((500.0, 500.0, Argb(0xffff_ffff)));
        let mut page = Page::new(&mut self.ids, name, w, h, bg);
        for d in &self.shared_layers {
            if d.to_all_pages {
                let pos = page.root.layers.len().saturating_sub(1);
                page.root.layers.insert(
                    pos,
                    LayerSlot::Shared(crate::layer::SharedLayerRef {
                        id: d.id.clone(),
                        to_all_pages: false,
                        extra: Extra::default(),
                    }),
                );
            }
        }
        let at = after.saturating_add(1).min(self.pages.len());
        self.pages.insert(at, Arc::new(page));
        if at <= self.current_page {
            self.current_page += 1;
        }
        at
    }

    /// Neue eindeutige Symbol-ID.
    pub fn new_symbol_id(&self) -> SymbolId {
        let mut v = random_u32() | 1;
        while self.symbols.iter().any(|s| s.id.0 == v) {
            v = v.wrapping_mul(48271) | 1;
        }
        SymbolId(v)
    }
}

impl Symbol {
    /// Neues Symbol mit einer normalen Ebene und der Web-Ebene.
    pub fn new(
        ids: &mut IdGen,
        name: &str,
        id: SymbolId,
        kind: crate::symbol::SymbolKind,
    ) -> Symbol {
        let frames = if kind == crate::symbol::SymbolKind::Button {
            4
        } else {
            1
        };
        let mut root = LayerRoot::new(frames);
        root.layers.push(LayerSlot::Own(Arc::new(Layer::new(
            ids,
            &fp_i18n::tr!("msg-default-layer", n = "1"),
            LayerKind::Normal,
            frames as usize,
        ))));
        root.layers.push(LayerSlot::Own(Arc::new(Layer::new(
            ids,
            &fp_i18n::t("msg-default-web-layer"),
            LayerKind::Web,
            frames as usize,
        ))));
        root.grid = Some(Grid::default());
        root.guides = Some(Guides::default());
        root.frame_count_repeat = Some(1);
        Symbol {
            name: name.to_string(),
            id,
            kind_code: if kind == crate::symbol::SymbolKind::Graphic {
                None
            } else {
                Some(kind.code())
            },
            origin_x: 0.0,
            origin_y: 0.0,
            library_file: None,
            library_symbol: None,
            modified: Some(String::new()),
            root,
            nine_slice: None,
            widget_script: None,
            button_options: ButtonOptions::default(),
            button: ButtonStrings::default(),
            status_message: None,
            behaviors: None,
            extra: Extra::default(),
        }
    }
}

impl Element {
    /// Neuer Pfad aus Konturen.
    pub fn new_path(ids: &mut IdGen, contours: Vec<Contour>, attrs: PathAttrs) -> Element {
        let bounds = contour_bounds(&contours);
        let handles = default_fill_handles(bounds);
        Element::new(
            ids.element(),
            ElementKind::Path(PathElem {
                attrs: Arc::new(attrs),
                stroke_placement: 1,
                ptt: 0,
                fill_on_top: false,
                random_seed: 0,
                texture_offset: Point::new(bounds.left, bounds.top),
                fill_handles: handles,
                contours,
                contours_extra: Extra::default(),
                polyline: None,
                polyline_extra: Extra::default(),
                even_odd: true,
                agm: false,
                path_op: 0,
                compat_proxy: false,
                primitive: None,
            }),
        )
    }

    /// Neues Rechteck als Pfad mit Rechteck-Primitive (wie das Rechteck-Werkzeug), Rundung 0..1.
    pub fn new_rectangle(ids: &mut IdGen, r: Rect, roundness: f32, attrs: PathAttrs) -> Element {
        let pts = [
            Point::new(r.left, r.top),
            Point::new(r.right, r.top),
            Point::new(r.right, r.bottom),
            Point::new(r.left, r.bottom),
        ];
        // Gerundet wie das Original (Laufzeit 11.10.2026, rt_a): Radius = Rundung · min(B, H)/2,
        // 8 Knoten im Uhrzeigersinn ab „oben links hinter der Ecke“, Griffe κ·r nach außen.
        let rad = roundness.clamp(0.0, 1.0) * r.width().abs().min(r.height().abs()) / 2.0;
        let nodes = if rad > 0.0 {
            let k = rad * fp_geom::ELLIPSE_KAPPA;
            let n = |ax: f32, ay: f32, px: f32, py: f32, sx: f32, sy: f32| {
                let mut c = ContourNode::corner(Point::new(ax, ay));
                c.pred = Point::new(px, py);
                c.succ = Point::new(sx, sy);
                c.curve = true;
                c
            };
            let (l, t, rr, b) = (r.left, r.top, r.right, r.bottom);
            vec![
                n(l + rad, t, l + rad - k, t, l + rad, t),
                n(rr - rad, t, rr - rad, t, rr - rad + k, t),
                n(rr, t + rad, rr, t + rad - k, rr, t + rad),
                n(rr, b - rad, rr, b - rad, rr, b - rad + k),
                n(rr - rad, b, rr - rad + k, b, rr - rad, b),
                n(l + rad, b, l + rad, b, l + rad - k, b),
                n(l, b - rad, l, b - rad + k, l, b - rad),
                n(l, t + rad, l, t + rad, l, t + rad - k),
            ]
        } else {
            pts.iter().map(|&p| ContourNode::corner(p)).collect()
        };
        let contour = Contour {
            closed: true,
            bsl: None,
            nodes,
            extra: Extra::default(),
        };
        let mut e = Element::new_path(ids, vec![contour], attrs);
        if let ElementKind::Path(p) = &mut e.kind {
            // Rechteck-Grundformen des Originals: AGM-Kantenglättung und `PTTi 1` (Laufzeit rt_a:
            // ohne diese Werte setzt das Original sie beim Öffnen und rendert die Ecken anders)
            p.agm = true;
            p.ptt = 1;
            let (w, h) = (r.width() / 2.0, r.height() / 2.0);
            let (cx, cy) = (r.left + w, r.top + h);
            p.primitive = Some(RectPrimitive {
                fill_handles: p.fill_handles,
                transform: Matrix3 {
                    m: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [cx, cy, 1.0]],
                },
                effects: None,
                rect: Rect::new(-w, -h, w, h),
                roundness,
                rect_extra: Extra::default(),
                roundness_pixels: Some(false),
                extra: Extra::default(),
            });
        }
        e
    }

    /// Neue Bitmap aus RGBA8-Pixeln.
    pub fn new_bitmap(
        ids: &mut IdGen,
        origin: Point,
        width: u32,
        height: u32,
        rgba: &[u8],
        next_tile_id: &mut u32,
    ) -> Element {
        let node = BitmapNode::from_rgba(origin, width, height, rgba, next_tile_id);
        Element::new(
            ids.element(),
            ElementKind::Bitmap(BitmapElem {
                image: node.bitmap.image,
                effect_result: None,
            }),
        )
    }

    /// Neue Gruppe (Kinder oben → unten).
    pub fn new_group(ids: &mut IdGen, children: Vec<Arc<Element>>) -> Element {
        Element::new(
            ids.element(),
            ElementKind::Group(GroupElem {
                kind: GroupKind::Normal,
                mask: None,
                children,
                children_extra: Extra::default(),
                mask_flags: MaskFlags::default(),
                replacement: None,
                replacement_extra: Extra::default(),
                smart_shape: None,
            }),
        )
    }

    /// Neue Maskengruppe; `vector` = „mask to path“. Das Original setzt nach `group` `MNAb = 1`.
    pub fn new_mask_group(
        ids: &mut IdGen,
        mask: Arc<Element>,
        children: Vec<Arc<Element>>,
        vector: bool,
    ) -> Element {
        let mut e = Element::new_group(ids, children);
        if let ElementKind::Group(g) = &mut e.kind {
            g.kind = if vector {
                GroupKind::VectorMask
            } else {
                GroupKind::BitmapMask
            };
            g.mask = Some(mask);
            g.mask_flags.hide_attrs = true;
        }
        e
    }

    /// Neue Symbolinstanz.
    pub fn new_instance(ids: &mut IdGen, symbol: SymbolId, transform: Matrix3) -> Element {
        Element::new(
            ids.element(),
            ElementKind::Instance(InstanceElem {
                symbol_id: symbol,
                instance_id: None,
                transform,
                alt_transform: Some(transform),
                transform_mode: 0,
                anim: AnimParams {
                    frames: 1,
                    ..AnimParams::default()
                },
                widget_params: None,
                button: ButtonStrings::default(),
                show_down_on_load: None,
                behaviors: None,
            }),
        )
    }

    /// Neuer Punkttext ohne ATE-Story. `ATEb 1` wie die Punkttexte des Originals (09 T7, Korpus: 4.217
    /// Punkttexte alle mit `ATEb 1`, davon ~3.270 ohne Story); mit `ATEb 0` behandelt das Original den
    /// Text als Alt-Text vor ATE und öffnet ihn als Rahmentext (Laufzeit phase5 `rt_a`).
    pub fn new_text(
        ids: &mut IdGen,
        text: &str,
        font: &str,
        size: f32,
        color: Argb,
        anchor: Point,
    ) -> Element {
        let attrs = TextAttrs {
            font: Some(font.to_string()),
            ps_font: Some(font.to_string()),
            size: Some(size),
            bold: Some(false),
            italic: Some(false),
            underline: Some(false),
            horizontal_scale: Some(1.0),
            kerning: Some(0.0),
            tracking: Some(0.0),
            baseline_shift: Some(0.0),
            leading: Some(1.0),
            leading_mode: Some(0),
            alignment: Some(0),
            color: None,
            indent: Some(0.0),
            space_before: Some(0.0),
            space_after: Some(0.0),
        };
        let mut run_attrs = attrs.clone();
        run_attrs.ps_font = None;
        run_attrs.kerning = None;
        run_attrs.color = Some(color.0);
        let height = size * 1.2;
        let width = size * 0.6 * text.chars().count() as f32;
        let bounds = Rect::new(
            anchor.x,
            anchor.y - size,
            anchor.x + width,
            anchor.y - size + height,
        );
        Element::new(
            ids.element(),
            ElementKind::Text(Box::new(TextElem {
                bounds,
                anchor_rect: Some(Rect::new(anchor.x, anchor.y, 0.0, 0.0)),
                frame_type: Some(0),
                story_index: None,
                from_story: Some(false),
                attrs: Arc::new(PathAttrs::solid_fill(color)),
                stroke_placement: 1,
                ptt: 0,
                fill_on_top: false,
                random_seed: 0,
                texture_offset: anchor,
                orientation: 0,
                fill_handles: default_fill_handles(bounds),
                transform: None,
                transform_mode: 0,
                antialias: 0,
                aa_sharpness: 192,
                aa_strength: 64,
                aa_oversample: 8,
                // neu angelegter Text ist frisch gesetzt (M10, re 43 §4: `initialAttrs` 18, Zeilenabstand f32)
                relaid: true,
                ate: Some(true),
                auto_kern: true,
                initial: attrs,
                runs: vec![TextRun {
                    attrs: run_attrs,
                    text: text.to_string(),
                    extra: Extra::default(),
                }],
                trailing_attrs: None,
                has_runs_node: true,
                runs_extra: Extra::default(),
                on_path: None,
            })),
        )
    }

    /// Neuer rechteckiger Slice (grüne Überlagerung, Bild-Slice).
    pub fn new_slice(ids: &mut IdGen, bounds: Rect) -> Element {
        let mut e = Element::new_hotspot(ids, bounds);
        if let ElementKind::WebObject(w) = &mut e.kind {
            w.color = Argb(0xff00_ff00);
            w.slice = Some(SliceProps {
                slice_type: 0,
                html: None,
                td_text: Some(String::new()),
                slice_id: Some(random_u32() | 1),
                export: None,
                background: Some(SliceBackground::default()),
            });
        }
        e
    }

    /// Neuer rechteckiger Hotspot.
    pub fn new_hotspot(ids: &mut IdGen, bounds: Rect) -> Element {
        let common = CommonAttrs::default();
        Element {
            id: ids.element(),
            common,
            kind: ElementKind::WebObject(Box::new(WebObjectElem {
                url: Some(String::new()),
                alt_text: Some(String::new()),
                target: None,
                color: Argb(0xff00_ffff),
                shape: WebShape::Rect,
                bounds,
                polygon: None,
                file_name: None,
                slice: None,
                behaviors: None,
                legacy_status: None,
            })),
            cache: None,
            extra: Extra::default(),
        }
    }

    /// Neues leeres Compound Shape aus Operanden.
    pub fn new_compound_shape(ids: &mut IdGen, operands: Vec<Arc<Element>>) -> Element {
        Element::new(
            ids.element(),
            ElementKind::CompoundShape(CompoundShapeElem {
                group_type: 0,
                result: None,
                operands,
                operands_extra: Extra::default(),
                proxy: None,
            }),
        )
    }
}

/// Umgebendes Rechteck der Anker aller Konturen.
pub fn contour_bounds(contours: &[Contour]) -> Rect {
    let mut r = Rect::new(
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    );
    for c in contours {
        for n in &c.nodes {
            for p in [n.anchor, n.pred, n.succ] {
                r.left = r.left.min(p.x);
                r.top = r.top.min(p.y);
                r.right = r.right.max(p.x);
                r.bottom = r.bottom.max(p.y);
            }
        }
    }
    if !r.left.is_finite() {
        Rect::default()
    } else {
        r
    }
}

/// Standard-Füll-Handles wie das Original für neue Objekte: Start links Mitte, Ende rechts Mitte, zweite
/// Achse oben links (A, aus Korpusbeispielen).
pub fn default_fill_handles(b: Rect) -> FillHandles {
    let cy = (b.top + b.bottom) / 2.0;
    FillHandles {
        valid: true,
        start: Point::new(b.left, cy),
        end: Point::new(b.right, cy),
        second: Point::new(b.left, b.top),
    }
}

/// Fill handles of a **new pattern fill**, measured (RE spec 61 §1.2/§1.6): handle 1 at the centre of
/// the bounds, handle 2 one tile width to the right, handle 3 one tile height up, so the pattern is
/// shown at 100 % with tile pixel (0,0) on the centre. A missing or empty tile size falls back to
/// [`default_fill_handles`].
pub fn default_pattern_handles(b: Rect, tile_w: u32, tile_h: u32) -> FillHandles {
    if tile_w == 0 || tile_h == 0 {
        return default_fill_handles(b);
    }
    let c = Point::new((b.left + b.right) / 2.0, (b.top + b.bottom) / 2.0);
    FillHandles {
        valid: true,
        start: c,
        end: Point::new(c.x + tile_w as f32, c.y),
        second: Point::new(c.x, c.y - tile_h as f32),
    }
}

/// Tile size `(width, height)` of a pattern fill's image, if it has one.
pub fn pattern_tile_size(f: &Fill) -> Option<(u32, u32)> {
    let img = f.pattern.as_ref()?.image.as_ref()?;
    let g = &img.bitmap.image.grid;
    (g.width > 0 && g.height > 0).then_some((g.width, g.height))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_pattern_handles_centre_and_tile_size() {
        // RE spec 61 §1.2: rectangle 100,100–260,220 with a 94 × 88 tile → (180,160) (274,160) (180,72)
        let h = default_pattern_handles(Rect::new(100.0, 100.0, 260.0, 220.0), 94, 88);
        assert!(h.valid);
        assert_eq!(h.start, Point::new(180.0, 160.0));
        assert_eq!(h.end, Point::new(274.0, 160.0));
        assert_eq!(h.second, Point::new(180.0, 72.0));
        let b = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert_eq!(default_pattern_handles(b, 0, 5), default_fill_handles(b));
    }

    #[test]
    fn new_document_structure() {
        let mut d = Document::new(200.0, 100.0, Argb(0xffffffff));
        assert_eq!(d.pages.len(), 1);
        let p = &d.pages[0];
        assert_eq!(p.root.layers.len(), 2);
        assert!(
            matches!(&p.root.layers[1], LayerSlot::Own(l) if l.kind == LayerKind::Web && l.shared_across_frames)
        );
        assert!(
            d.check_invariants().is_empty(),
            "{:?}",
            d.check_invariants()
        );
        let i = d.add_page("Seite 2");
        assert_eq!(i, 1);
        assert_eq!(d.pages.len(), 2);
    }

    #[test]
    fn add_page_after_inserts_behind_the_given_page() {
        // RE spec 70 §2.3: directly after the given page, the end after the last one
        let mut d = Document::new(200.0, 100.0, Argb(0xffffffff));
        d.add_page("B");
        d.add_page_after(1, "C");
        let names = |d: &Document| d.pages.iter().map(|p| p.display_name().to_string()).collect::<Vec<_>>();
        assert_eq!(names(&d)[1..], ["B", "C"]);
        // in front of the model's current page: `current_page` follows its page
        d.current_page = 2;
        assert_eq!(d.add_page_after(0, "X"), 1);
        assert_eq!(names(&d)[1..], ["X", "B", "C"]);
        assert_eq!(d.current_page, 3, "still page C");
        assert_eq!(d.add_page_after(3, "Z"), 4, "after the last page: the end");
        assert_eq!(d.current_page, 3);
        // the new page takes the canvas size of the page it follows
        Arc::make_mut(&mut d.pages[1]).canvas.width = 333.0;
        let i = d.add_page_after(1, "Y");
        assert_eq!(d.pages[i].canvas.width, 333.0);
    }

    #[test]
    fn json_roundtrip() {
        let mut d = Document::new(64.0, 32.0, Argb(0x00ff_ffff));
        let mut ids = std::mem::take(&mut d.ids);
        let mut r = Element::new_rectangle(
            &mut ids,
            Rect::new(1.0, 2.0, 30.0, 20.0),
            0.25,
            PathAttrs::solid_fill(Argb(0xff112233)),
        );
        r.common.custom_data.set_number("width", 29.0);
        let mut t = Element::new_text(
            &mut ids,
            "Ä\r",
            "Arial",
            10.0,
            Argb(0xff000000),
            Point::new(2.0, 28.0),
        );
        // T-11: neuer Text beginnt mit `relaid`; der Merker ist Laufzeit (nicht gespeichert),
        // nach dem Einlesen gilt die Datei-Form – für den Vergleich daher zurücksetzen.
        if let ElementKind::Text(x) = &mut t.kind {
            assert!(x.relaid);
            x.relaid = false;
        }
        d.ids = ids;
        if let Some(LayerSlot::Own(l)) = d.page_mut(0).map(|p| &mut p.root.layers[0]) {
            let cel = Arc::make_mut(&mut Arc::make_mut(l).cels[0]);
            cel.elements.push(Arc::new(r));
            cel.elements.push(Arc::new(t));
        }
        let json = serde_json::to_string(&d).unwrap();
        let back: Document = serde_json::from_str(&json).unwrap();
        assert_eq!(back, d);
    }

    #[test]
    fn new_elements() {
        let mut ids = IdGen::new();
        let r = Element::new_rectangle(
            &mut ids,
            Rect::new(0.0, 0.0, 10.0, 20.0),
            0.0,
            PathAttrs::solid_fill(Argb(0xffff0000)),
        );
        assert_eq!(r.type_code(), 0x3304);
        let t = Element::new_text(
            &mut ids,
            "Hallo",
            "Arial",
            12.0,
            Argb(0xff000000),
            Point::new(5.0, 20.0),
        );
        if let ElementKind::Text(t) = &t.kind {
            assert_eq!(t.plain_text(), "Hallo");
        }
        let g = Element::new_group(&mut ids, vec![Arc::new(r)]);
        let d = g.duplicate(&mut ids);
        assert_ne!(d.id, g.id);
        assert_ne!(d.children()[0].id, g.children()[0].id);
    }
}
