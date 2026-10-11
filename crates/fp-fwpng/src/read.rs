//! Abbildung Tag-Baum (`mkTS`) → Dokumentmodell.
//!
//! Eine Funktion je Container, wie die Leser im Original (04 §1.4). Unbekannte Tags bleiben als
//! Rohknoten erhalten; jeder Container bekommt seine Rundreise-Daten ([`fp_model::Extra`]).
//! Fehlende Pflicht-Tags führen nicht zum Fehler: Der Container bleibt dann als Rohknoten erhalten
//! (z. B. [`ElementKind::Raw`]).

use crate::k;
use crate::schema::{Interner, Rd};
use fp_color::Argb;
use fp_geom::{Point, Rect};
use fp_model::*;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Lesekontext: Kacheln, ID-Vergabe, Interner, Warnungen.
pub struct ReadCtx<'t> {
    pub ids: IdGen,
    pub tiles: &'t HashMap<u32, Arc<Tile>>,
    pub interner: Interner,
    pub warnings: Vec<String>,
    /// Kachel-IDs, die der Baum referenziert.
    pub referenced: HashSet<u32>,
}

impl<'t> ReadCtx<'t> {
    pub fn new(tiles: &'t HashMap<u32, Arc<Tile>>) -> Self {
        ReadCtx { ids: IdGen::new(), tiles, interner: Interner::default(), warnings: Vec::new(), referenced: HashSet::new() }
    }
    fn finish(&mut self, rd: Rd) -> Extra {
        rd.finish(&mut self.interner)
    }
}

const LTRB: [TagKey; 4] = [k!(b"LFTf"), k!(b"TOPf"), k!(b"RITf"), k!(b"BOTf")];

/// Liest die Wurzeln von `mkTS` in ein Dokument.
pub fn read_document(roots: &[RawNode], ctx: &mut ReadCtx, ate: Option<AteData>) -> crate::error::Result<Document> {
    let mut doc = Document {
        pages: Vec::new(),
        current_page: 0,
        root_copy: None,
        root_id: None,
        single_root: false,
        symbols: Vec::new(),
        symbols_extra: Extra::default(),
        styles: None,
        styles_extra: Extra::default(),
        shared_layers: Vec::new(),
        shared_layers_extra: Extra::default(),
        has_shared_layer_list: false,
        ate,
        extra_roots: Vec::new(),
        ids: IdGen::new(),
        file_meta: FileMeta::default(),
    };
    let mut mkb: Option<&RawNode> = None;
    let mut pdcs: Vec<&RawNode> = Vec::new();
    for (i, r) in roots.iter().enumerate() {
        if i == 0 && r.key == k!(b"MKBv") {
            mkb = Some(r);
        } else if r.key == k!(b"PDCv") && doc.extra_roots.is_empty() {
            pdcs.push(r);
        } else {
            doc.extra_roots.push(r.clone());
        }
    }
    let Some(mkb) = mkb else {
        return Err(crate::error::FwError::Document("mkTS beginnt nicht mit MKBv".into()));
    };
    let root_page = read_page(mkb.children(), ctx, Some(&mut doc));
    if pdcs.is_empty() {
        doc.single_root = true;
        doc.root_id = root_page.id.clone();
        doc.pages.push(Arc::new(root_page));
    } else {
        doc.root_id = root_page.id.clone();
        for p in pdcs {
            let page = read_page(p.children(), ctx, None);
            doc.pages.push(Arc::new(page));
        }
        doc.current_page = guess_active_page(&root_page, &doc.pages);
        doc.root_copy = Some(Arc::new(root_page));
    }
    doc.ids = std::mem::take(&mut ctx.ids);
    Ok(doc)
}

/// Strukturmerkmal einer Seite für den Abgleich mit der `MKBv`-Kopie: Elementarten (und
/// Symbol-IDs von Instanzen) der eigenen Nicht-Web-Ebenen in Dateireihenfolge, über alle Status.
/// Elemente der Nicht-Web-Ebenen in Dateireihenfolge über alle Status; ein
/// Masterseiten-Platzhalter wird durch die Ebenen der Masterseite ersetzt (wie in der Kopie).
fn page_elements<'a>(p: &'a Page, master: Option<&'a Page>, out: &mut Vec<&'a Arc<fp_model::Element>>) {
    use fp_model::{LayerKind, LayerSlot};
    for s in &p.root.layers {
        match s {
            LayerSlot::Own(l) if l.kind != LayerKind::Web => {
                for c in &l.cels {
                    out.extend(c.elements.iter());
                }
            }
            LayerSlot::MasterPage(_) => {
                if let Some(m) = master.filter(|m| !std::ptr::eq(*m, p)) {
                    page_elements(m, None, out);
                }
            }
            _ => {}
        }
    }
}

fn page_signature(elems: &[&Arc<fp_model::Element>]) -> Vec<(u8, u64)> {
    use fp_model::ElementKind;
    let mut out = Vec::new();
    {
        {
            for e in elems {
                let tag = match &e.kind {
                    ElementKind::Bitmap(_) => 1,
                    ElementKind::Path(_) => 2,
                    ElementKind::Text(_) => 3,
                    ElementKind::Group(_) => 4,
                    ElementKind::CompoundShape(_) => 5,
                    ElementKind::Instance(_) => 6,
                    ElementKind::WebObject(_) => 7,
                    ElementKind::SubLayer(_) => 8,
                    ElementKind::Raw(_) => 9,
                };
                let sym = match &e.kind {
                    ElementKind::Instance(i) => {
                        use std::hash::{Hash, Hasher};
                        let mut h = std::collections::hash_map::DefaultHasher::new();
                        format!("{:?}", i.symbol_id).hash(&mut h);
                        h.finish()
                    }
                    _ => 0,
                };
                out.push((tag, sym));
            }
        }
    }
    out
}

/// Welche Seite kopiert `MKBv`? Vorrang hat die Struktur (Elementarten/Symbole der eigenen Ebenen
/// wie `symrender.active_pdc`): genau gleich ⇒ +100, sonst Anteil gleicher Stellen (bis +40);
/// danach Name, Größe, Ursprung, Leinwandfarbe und Ebenenanzahl.
fn guess_active_page(copy: &Page, pages: &[Arc<Page>]) -> usize {
    let master = pages.iter().find(|p| p.master()).map(|p| &**p);
    let mut want_e = Vec::new();
    page_elements(copy, None, &mut want_e);
    let want = page_signature(&want_e);
    let score = |p: &Page| -> i32 {
        let mut s = 0;
        let mut el = Vec::new();
        page_elements(p, master, &mut el);
        let sig = page_signature(&el);
        let n = sig.len().max(want.len()).max(1);
        if sig == want {
            s += 100;
        } else {
            // vorn und hinten ausgerichtet (geteilte Ebenen verschieben die Folge)
            let front = sig.iter().zip(&want).filter(|(a, b)| a == b).count();
            let back = sig.iter().rev().zip(want.iter().rev()).filter(|(a, b)| a == b).count();
            let same = front.max(back);
            s += (40 * same / n) as i32;
        }
        // gleicher Inhalt (Elemente selbst gleich) entscheidet zwischen gleich gebauten Seiten
        let eq_front = el.iter().zip(&want_e).filter(|(a, b)| a.common == b.common && a.kind == b.kind).count();
        let eq_back = el.iter().rev().zip(want_e.iter().rev()).filter(|(a, b)| a.common == b.common && a.kind == b.kind).count();
        s += (30 * eq_front.max(eq_back) / n) as i32;
        if p.name.is_some() && p.name == copy.name {
            s += 4;
        }
        if p.canvas.width == copy.canvas.width && p.canvas.height == copy.canvas.height {
            s += 2;
        }
        if p.canvas.origin == copy.canvas.origin {
            s += 1;
        }
        if p.root.background == copy.root.background {
            s += 1;
        }
        let own = |r: &LayerRoot| r.layers.len();
        if own(&p.root) == own(&copy.root) {
            s += 1;
        }
        s
    };
    let mut best = 0usize;
    let mut best_score = i32::MIN;
    for (i, p) in pages.iter().enumerate() {
        let sc = score(p);
        if sc > best_score {
            best_score = sc;
            best = i;
        }
    }
    best
}

/// Seite (`MKBv`/`PDCv`). Mit `globals` werden die dokumentweiten Listen von `MKBv` mitgelesen.
pub fn read_page(kids: &[RawNode], ctx: &mut ReadCtx, mut globals: Option<&mut Document>) -> Page {
    let mut rd = Rd::new(kids);
    let format_version = rd.int(k!(b"VRSi"));
    let format_minor = rd.int(k!(b"VRMi"));
    let doc_random = rd.int(k!(b"RQNi"));
    let canvas = Canvas {
        origin: Point::new(rd.f32(k!(b"XLCf")).unwrap_or(0.0), rd.f32(k!(b"YLCf")).unwrap_or(0.0)),
        width: rd.f32(k!(b"WIDf")).unwrap_or(0.0),
        height: rd.f32(k!(b"HITf")).unwrap_or(0.0),
        resolution: rd.f32(k!(b"RESf")).unwrap_or(72.0),
        resolution_unit: rd.i32(k!(b"RSUi")).unwrap_or(0),
    };
    let root = read_layer_root(&mut rd, ctx, 2);
    let mut presets = Presets::default();
    if let Some(v) = rd.strict_list(k!(b"PLLv"), k!(b"BPLv")) {
        let (items, ex) = read_items(v, ctx, read_brush);
        presets.brushes = Some(items);
        presets.brushes_extra = ex;
    }
    if let Some(v) = rd.strict_list(k!(b"FLDv"), k!(b"FPLv")) {
        let (items, ex) = read_items(v, ctx, read_fill);
        presets.fills = Some(items);
        presets.fills_extra = ex;
    }
    if let Some(v) = rd.strict_list(k!(b"GLDv"), k!(b"FGLv")) {
        let (items, ex) = read_items(v, ctx, read_gradient);
        presets.gradients = Some(items);
        presets.gradients_extra = ex;
    }
    if let Some(v) = rd.strict_list(k!(b"EFDv"), k!(b"EFLv")) {
        let (items, ex) = read_items(v, ctx, read_effect_list);
        presets.effects = Some(items);
        presets.effects_extra = ex;
    }
    if let Some(g) = globals.as_deref_mut() {
        if let Some(v) = rd.strict_list(k!(b"MSLv"), k!(b"MSTv")) {
            let mut sub = Rd::new(v);
            for n in sub.take_all(k!(b"MSTv")) {
                g.symbols.push(Arc::new(read_symbol(n.children(), ctx)));
            }
            g.symbols_extra = ctx.finish(sub);
        }
        if let Some(v) = rd.strict_list(k!(b"ZSLv"), k!(b"ZSTv")) {
            let mut sub = Rd::new(v);
            let styles: Vec<Arc<Style>> = sub.take_all(k!(b"ZSTv")).into_iter().map(|n| Arc::new(read_style(n.children(), ctx))).collect();
            g.styles = Some(styles);
            g.styles_extra = ctx.finish(sub);
        }
    }
    let tool_attrs = rd.list(k!(b"PATv")).map(|v| Arc::new(read_attrs(v, ctx)));
    let image_map = rd.list(k!(b"MNFv")).map(|v| {
        let mut s = Rd::new(v);
        let name = s.str(k!(b"MNDs"));
        ImageMap { name, extra: ctx.finish(s) }
    });
    let name = rd.str(k!(b"PGNs"));
    let is_master = rd.bool(k!(b"IMPb"));
    let linked_to_master = rd.bool(k!(b"PILb"));
    let id = rd.str(k!(b"PIDs")).map(Guid);
    let mut local_shared_layers = None;
    let mut local_shared_layers_extra = Extra::default();
    if let Some(v) = rd.strict_list(k!(b"LSMv"), k!(b"LSEv")) {
        let mut sub = Rd::new(v);
        let defs: Vec<SharedLayerDef> = sub.take_all(k!(b"LSEv")).into_iter().map(|n| read_shared_def(n.children(), ctx)).collect();
        let ex = ctx.finish(sub);
        match globals {
            Some(g) => {
                g.shared_layers = defs;
                g.shared_layers_extra = ex;
                g.has_shared_layer_list = true;
            }
            None => {
                local_shared_layers = Some(defs);
                local_shared_layers_extra = ex;
            }
        }
    }
    let compat_layer_copy = rd.bool(k!(b"ILLb"));
    let last_export_dir = rd.str(k!(b"LXDs"));
    let last_export_file = rd.str(k!(b"LXFs"));
    let selection = rd.list(k!(b"MSVv")).map(|v| read_selection(v, ctx));
    let mut saved_selections = None;
    let mut saved_selections_extra = Extra::default();
    if let Some(v) = rd.strict_list(k!(b"MSSv"), k!(b"OSSv")) {
        let mut sub = Rd::new(v);
        saved_selections = Some(sub.take_all(k!(b"OSSv")).into_iter().map(|n| read_selection(n.children(), ctx)).collect());
        saved_selections_extra = ctx.finish(sub);
    }
    let jpeg_mask = rd.list(k!(b"MJPv")).map(|v| read_selection(v, ctx));
    let printer_data = rd.bytes(k!(b"PTRy"));
    let legacy_loop = rd.i32(k!(b"FLPi"));
    let legacy_export = rd.list(k!(b"EXSv")).map(|v| Box::new(read_export(v, ctx)));
    let extra = ctx.finish(rd);
    Page {
        format_version,
        format_minor,
        doc_random,
        canvas,
        root,
        presets,
        tool_attrs,
        image_map,
        name,
        is_master,
        linked_to_master,
        id,
        local_shared_layers,
        local_shared_layers_extra,
        compat_layer_copy,
        last_export_dir,
        last_export_file,
        selection,
        saved_selections,
        saved_selections_extra,
        jpeg_mask,
        printer_data,
        legacy_loop,
        legacy_export,
        extra,
    }
}

fn read_items<T>(v: &[RawNode], ctx: &mut ReadCtx, f: fn(&[RawNode], &mut ReadCtx) -> T) -> (Vec<T>, Extra) {
    let mut sub = Rd::new(v);
    let mut items = Vec::with_capacity(v.len());
    for (i, n) in v.iter().enumerate() {
        sub.mark(i);
        items.push(f(n.children(), ctx));
    }
    (items, ctx.finish(sub))
}

/// Gemeinsamer Teil von Seite und Symbol. `frc_default` = übliche Anzahl der `FRCi`-Tags.
fn read_layer_root(rd: &mut Rd, ctx: &mut ReadCtx, frc_default: u32) -> LayerRoot {
    let frc_nodes: Vec<u32> = rd.all().iter().filter(|n| n.key == k!(b"FRCi")).filter_map(|n| n.as_int()).collect();
    let mut frame_count = 1;
    let mut frame_count_repeat = None;
    if !frc_nodes.is_empty() && frc_nodes.iter().all(|v| *v == frc_nodes[0]) {
        rd.take_all(k!(b"FRCi"));
        frame_count = frc_nodes[0];
        if frc_nodes.len() as u32 != frc_default {
            frame_count_repeat = Some(frc_nodes.len() as u32);
        }
    } else if let Some(v) = frc_nodes.first() {
        frame_count = *v;
    }
    let mut layers = Vec::new();
    let mut layers_extra = Extra::default();
    let mut has_layer_list = false;
    if let Some(v) = rd.list(k!(b"LYLv")) {
        has_layer_list = true;
        let (l, e) = read_layer_list(v, ctx);
        layers = l;
        layers_extra = e;
    }
    let grid = rd.list(k!(b"GRDv")).map(|v| {
        let mut s = Rd::new(v);
        let origin = Point::new(s.f32(k!(b"GOXf")).unwrap_or(0.0), s.f32(k!(b"GOYf")).unwrap_or(0.0));
        let size = Point::new(s.f32(k!(b"GSXf")).unwrap_or(36.0), s.f32(k!(b"GSYf")).unwrap_or(36.0));
        let color = Argb(s.int(k!(b"GCLi")).unwrap_or(0xff66_6666));
        Grid { origin, size, color, extra: ctx.finish(s) }
    });
    let guides = rd.list(k!(b"GDSv")).map(|v| read_guides(v, ctx));
    let onion_before = rd.i32(k!(b"OSBi")).unwrap_or(0);
    let onion_after = rd.i32(k!(b"OSAi")).unwrap_or(0);
    let background = Argb(rd.int(k!(b"BGCi")).unwrap_or(0xffff_ffff));
    let export = rd.list(k!(b"EXPv")).map(|v| Box::new(read_export(v, ctx)));
    LayerRoot {
        frame_count,
        frame_count_repeat,
        layers,
        has_layer_list,
        layers_extra,
        grid,
        guides,
        onion_before,
        onion_after,
        background,
        export,
    }
}

fn read_guides(v: &[RawNode], ctx: &mut ReadCtx) -> Guides {
    let mut s = Rd::new(v);
    let locked = s.bool(k!(b"GDLb"));
    let color = s.int(k!(b"GDKi")).map(Argb);
    let mut horizontal = Vec::new();
    let mut horizontal_extra = Extra::default();
    if let Some(h) = s.strict_list(k!(b"GDHv"), k!(b"GDCf")) {
        let (vals, ex) = read_float_list(h, k!(b"GDCf"), ctx);
        horizontal = vals;
        horizontal_extra = ex;
    }
    let mut vertical = Vec::new();
    let mut vertical_extra = Extra::default();
    if let Some(h) = s.strict_list(k!(b"GDVv"), k!(b"GDCf")) {
        let (vals, ex) = read_float_list(h, k!(b"GDCf"), ctx);
        vertical = vals;
        vertical_extra = ex;
    }
    Guides { locked, color, horizontal, horizontal_extra, vertical, vertical_extra, extra: ctx.finish(s) }
}

fn read_float_list(v: &[RawNode], key: TagKey, ctx: &mut ReadCtx) -> (Vec<f32>, Extra) {
    let mut s = Rd::new(v);
    let mut vals = Vec::new();
    let mut shadow = Vec::new();
    for n in s.take_all(key) {
        let x = n.as_num().unwrap_or(0.0);
        let f = x as f32;
        if crate::tree::format_float(f as f64) != crate::tree::format_float(x) {
            shadow.push(n.clone());
        }
        vals.push(f);
    }
    let mut ex = ctx.finish(s);
    ex.shadow = shadow;
    (vals, ex)
}

fn read_selection(v: &[RawNode], ctx: &mut ReadCtx) -> SavedSelection {
    let mut s = Rd::new(v);
    let mask = s.list(k!(b"MSKv")).map(|m| read_mask(m, ctx));
    let left = s.f32(k!(b"LFTf")).unwrap_or(0.0);
    let top = s.f32(k!(b"TOPf")).unwrap_or(0.0);
    let name = s.str(k!(b"NMSs"));
    SavedSelection { mask, left, top, name, extra: ctx.finish(s) }
}

fn read_shared_def(v: &[RawNode], ctx: &mut ReadCtx) -> SharedLayerDef {
    let mut s = Rd::new(v);
    let id = Guid(s.str(k!(b"UIDs")).unwrap_or_default());
    let to_all_pages = s.bool(k!(b"ISPb")).unwrap_or(false);
    let layer = s.list(k!(b"LAYv")).map(|l| Arc::new(read_layer(l, ctx)));
    SharedLayerDef { id, to_all_pages, layer, extra: ctx.finish(s) }
}

/// Ebenenliste `LYLv`.
pub fn read_layer_list(v: &[RawNode], ctx: &mut ReadCtx) -> (Vec<LayerSlot>, Extra) {
    let mut s = Rd::new(v);
    let mut out = Vec::new();
    for (i, n) in v.iter().enumerate() {
        let slot = match &n.key.0 {
            b"LAYv" => LayerSlot::Own(Arc::new(read_layer(n.children(), ctx))),
            b"LSAv" => {
                let mut r = Rd::new(n.children());
                let id = Guid(r.str(k!(b"UIDs")).unwrap_or_default());
                let to_all_pages = r.bool(k!(b"ISPb")).unwrap_or(false);
                LayerSlot::Shared(SharedLayerRef { id, to_all_pages, extra: ctx.finish(r) })
            }
            b"MPLv" => {
                let mut r = Rd::new(n.children());
                let master = Guid(r.str(k!(b"PIDs")).unwrap_or_default());
                let expanded = r.bool(k!(b"DISb")).unwrap_or(true);
                let name = r.str(k!(b"LNMs")).unwrap_or_default();
                let (visibility, has_visibility_node, visibility_extra) = read_visibility(&mut r, ctx);
                LayerSlot::MasterPage(MasterPageLayer {
                    master,
                    expanded,
                    name,
                    visibility,
                    has_visibility_node,
                    visibility_extra,
                    extra: ctx.finish(r),
                })
            }
            _ => LayerSlot::Raw(n.clone()),
        };
        s.mark(i);
        out.push(slot);
    }
    (out, ctx.finish(s))
}

fn read_visibility(r: &mut Rd, ctx: &mut ReadCtx) -> (Vec<bool>, bool, Extra) {
    match r.strict_list(k!(b"VIFv"), k!(b"VISb")) {
        Some(v) => {
            let mut s = Rd::new(v);
            let vis: Vec<bool> = s.take_all(k!(b"VISb")).into_iter().map(|n| n.as_bool().unwrap_or(true)).collect();
            (vis, true, ctx.finish(s))
        }
        None => (Vec::new(), false, Extra::default()),
    }
}

/// Ebene `LAYv` bzw. Unterebene `LSLv`.
pub fn read_layer(v: &[RawNode], ctx: &mut ReadCtx) -> Layer {
    let mut r = Rd::new(v);
    let background = r.bool(k!(b"BKGb")).unwrap_or(false);
    let web = r.bool(k!(b"URLb")).unwrap_or(false);
    let name = r.str(k!(b"LNMs")).unwrap_or_default();
    let level = r.i32(k!(b"LLLi")).unwrap_or(0);
    let msl = r.bool(k!(b"MSLb"));
    let mcl = r.bool(k!(b"MCLb"));
    let expanded = r.bool(k!(b"DISb")).unwrap_or(true);
    let opacity = r.opacity(k!(b"OPAi")).unwrap_or(fp_color::Opacity(255));
    let blend = r.blend(k!(b"BLDi")).unwrap_or_default();
    // Teilung über Status (02 §4.2, `@00a758c0`).
    let shared_across_frames = if background && !web {
        true
    } else {
        match mcl {
            Some(m) => !m,
            None => msl.unwrap_or(true),
        }
    };
    let mut cels = Vec::new();
    let mut cels_extra = Extra::default();
    if let Some(cl) = r.strict_list(k!(b"CLLv"), k!(b"CELv")) {
        let mut s = Rd::new(cl);
        for n in s.take_all(k!(b"CELv")) {
            cels.push(Arc::new(read_cel(n.children(), ctx)));
        }
        cels_extra = ctx.finish(s);
    }
    let (visibility, has_visibility_node, visibility_extra) = read_visibility(&mut r, ctx);
    Layer {
        id: ctx.ids.layer(),
        name,
        kind: if web { LayerKind::Web } else { LayerKind::Normal },
        background,
        level,
        legacy_shared: msl.unwrap_or(false),
        shared_across_frames,
        expanded,
        visibility,
        has_visibility_node,
        visibility_extra,
        opacity,
        blend,
        cels,
        cels_extra,
        extra: ctx.finish(r),
    }
}

fn read_cel(v: &[RawNode], ctx: &mut ReadCtx) -> Cel {
    let mut r = Rd::new(v);
    let visible = r.bool(k!(b"VISb")).unwrap_or(true);
    let locked = r.bool(k!(b"LCKb")).unwrap_or(false);
    let (elements, elements_extra) = match r.list(k!(b"ELMv")) {
        Some(e) => read_elements(e, ctx),
        None => (Vec::new(), Extra::default()),
    };
    Cel { locked, visible, elements, elements_extra, extra: ctx.finish(r) }
}

/// Elementliste `ELMv` (oben → unten). Ein `PTHv` mit `PRPb=1` direkt hinter einem `CSGv` ist dessen
/// Stellvertreter-Pfad und wird an das Compound Shape gehängt.
pub fn read_elements(v: &[RawNode], ctx: &mut ReadCtx) -> (Vec<Arc<Element>>, Extra) {
    let mut s = Rd::new(v);
    let mut out: Vec<Arc<Element>> = Vec::new();
    for (i, n) in v.iter().enumerate() {
        let e = read_element(n, ctx);
        s.mark(i);
        if let ElementKind::Path(p) = &e.kind
            && p.compat_proxy
                && let Some(last) = out.last_mut()
                    && let ElementKind::CompoundShape(c) = &last.kind
                        && c.proxy.is_none() {
                            let mut proxy = e.clone();
                            if let ElementKind::Path(pp) = &mut proxy.kind {
                                pp.compat_proxy = false;
                            }
                            if let ElementKind::CompoundShape(c) = &mut Arc::make_mut(last).kind {
                                c.proxy = Some(Arc::new(proxy));
                            }
                            continue;
                        }
        out.push(Arc::new(e));
    }
    (out, ctx.finish(s))
}

/// Gemeinsame Attribute eines Elements.
fn read_common(r: &mut Rd, ctx: &mut ReadCtx) -> CommonAttrs {
    CommonAttrs {
        name: r.str(k!(b"OBNs")),
        visible: r.bool(k!(b"VISb")).unwrap_or(true),
        locked: r.bool(k!(b"LCKb")).unwrap_or(false),
        opacity: r.opacity(k!(b"OPAi")).unwrap_or(fp_color::Opacity(255)),
        blend: r.blend(k!(b"BLDi")).unwrap_or_default(),
        effects: r.list(k!(b"EFLv")).map(|v| read_effect_list(v, ctx)),
        style: r.str(k!(b"ZSNs")),
        custom_data: r.list(k!(b"DTAv")).map(|v| read_dict(v, ctx)).unwrap_or_default(),
    }
}

/// Ein Element; unvollständige Container bleiben roh.
pub fn read_element(n: &RawNode, ctx: &mut ReadCtx) -> Element {
    let kids = n.children();
    let id = ctx.ids.element();
    let raw = |id| Element::new(id, ElementKind::Raw(n.clone()));
    match &n.key.0 {
        b"IMGv" => match read_bitmap_node(kids, ctx) {
            Some(b) => Element { id, common: b.common, kind: ElementKind::Bitmap(b.bitmap), cache: None, extra: b.extra },
            None => raw(id),
        },
        b"PTHv" => {
            let mut r = Rd::new(kids);
            let common = read_common(&mut r, ctx);
            let path = read_path(&mut r, ctx);
            let cache = take_bitmap(&mut r, ctx);
            Element { id, common, kind: ElementKind::Path(path), cache, extra: ctx.finish(r) }
        }
        b"TXTv" => {
            let mut r = Rd::new(kids);
            let common = read_common(&mut r, ctx);
            let text = read_text(&mut r, ctx);
            let cache = take_bitmap(&mut r, ctx);
            Element { id, common, kind: ElementKind::Text(Box::new(text)), cache, extra: ctx.finish(r) }
        }
        b"GRPv" => {
            let mut r = Rd::new(kids);
            let common = read_common(&mut r, ctx);
            let group = read_group(&mut r, ctx);
            let cache = take_bitmap(&mut r, ctx);
            Element { id, common, kind: ElementKind::Group(group), cache, extra: ctx.finish(r) }
        }
        b"CSGv" => {
            let mut r = Rd::new(kids);
            let common = read_common(&mut r, ctx);
            let group_type = r.i32(k!(b"GRTi")).unwrap_or(0);
            let result = r.list(k!(b"PTHv")).map(|v| Arc::new(read_element(&RawNode::list(k!(b"PTHv"), v.to_vec()), ctx)));
            let (operands, operands_extra) = match r.list(k!(b"ELMv")) {
                Some(v) => read_elements(v, ctx),
                None => (Vec::new(), Extra::default()),
            };
            let cache = take_bitmap(&mut r, ctx);
            Element {
                id,
                common,
                kind: ElementKind::CompoundShape(CompoundShapeElem { group_type, result, operands, operands_extra, proxy: None }),
                cache,
                extra: ctx.finish(r),
            }
        }
        b"ALSv" => {
            let mut r = Rd::new(kids);
            if r.peek_int(k!(b"MSNi")).is_none() || !r.has(k!(b"MTXv")) {
                return raw(id);
            }
            let common = read_common(&mut r, ctx);
            let Some(inst) = read_instance(&mut r, ctx) else { return raw(id) };
            let cache = take_bitmap(&mut r, ctx);
            Element { id, common, kind: ElementKind::Instance(inst), cache, extra: ctx.finish(r) }
        }
        b"URLv" => {
            let mut r = Rd::new(kids);
            let common = read_common(&mut r, ctx);
            let web = read_web(&mut r, ctx);
            Element { id, common, kind: ElementKind::WebObject(Box::new(web)), cache: None, extra: ctx.finish(r) }
        }
        b"LSLv" => {
            let layer = read_layer(kids, ctx);
            Element::new(id, ElementKind::SubLayer(Arc::new(layer)))
        }
        _ => raw(id),
    }
}

impl Rd<'_> {
    /// Ganzzahl lesen, ohne sie zu verbrauchen.
    pub fn peek_int(&self, key: TagKey) -> Option<u32> {
        let mut v = None;
        for n in self.all() {
            if n.key == key {
                if v.is_some() {
                    return None;
                }
                v = n.as_int();
            }
        }
        v
    }
}

/// Liest ein Kind-`IMGv` (Cache, Effekt-Ergebnis, Musterbild); unvollständige bleiben roh.
fn take_bitmap(r: &mut Rd, ctx: &mut ReadCtx) -> Option<Arc<BitmapNode>> {
    let v = r.peek_list(k!(b"IMGv"))?;
    let b = read_bitmap_node(v, ctx)?;
    r.list(k!(b"IMGv"));
    Some(Arc::new(b))
}

/// `IMGv` als Ganzes (Bitmap, Cache, Effekt-Ergebnis, Musterbild).
pub fn read_bitmap_node(v: &[RawNode], ctx: &mut ReadCtx) -> Option<BitmapNode> {
    let mut r = Rd::new(v);
    for key in [k!(b"WPXi"), k!(b"HPXi")] {
        r.peek_int(key)?;
    }
    let common = read_common(&mut r, ctx);
    let origin = Point::new(r.f32(k!(b"XLCf")).unwrap_or(0.0), r.f32(k!(b"YLCf")).unwrap_or(0.0));
    let offset = Point::new(r.f32(k!(b"XOFf")).unwrap_or(0.0), r.f32(k!(b"YOFf")).unwrap_or(0.0));
    let grid = read_tile_grid(&mut r, ctx, 128);
    let effect_result = take_bitmap(&mut r, ctx);
    let extra = ctx.finish(r);
    Some(BitmapNode { common, bitmap: BitmapElem { image: ImageData { origin, offset, grid }, effect_result }, extra })
}

fn read_tile_grid(r: &mut Rd, ctx: &mut ReadCtx, default_ts: u32) -> TileGrid {
    let width = r.int(k!(b"WPXi")).unwrap_or(0);
    let height = r.int(k!(b"HPXi")).unwrap_or(0);
    let tile_size = r.int(k!(b"TSZi")).unwrap_or(default_ts);
    let mut tiles = Vec::new();
    let mut tiles_extra = Extra::default();
    if let Some(tl) = r.list(k!(b"TILv")) {
        let mut s = Rd::new(tl);
        for (i, n) in tl.iter().enumerate() {
            let e = match (&n.key.0, n.as_int()) {
                (b"TIDi", Some(id)) => {
                    ctx.referenced.insert(id);
                    match ctx.tiles.get(&id) {
                        Some(t) => TileEntry::Tile(Arc::clone(t)),
                        None => TileEntry::Missing(id),
                    }
                }
                (b"TMCi", Some(c)) => TileEntry::Solid(Argb(c)),
                (b"TMMi", Some(c)) => TileEntry::SolidMask(c),
                _ => continue,
            };
            s.mark(i);
            tiles.push(e);
        }
        tiles_extra = ctx.finish(s);
    }
    TileGrid { width, height, tile_size, tiles, tiles_extra }
}

fn read_mask(v: &[RawNode], ctx: &mut ReadCtx) -> MaskData {
    let mut r = Rd::new(v);
    let grid = read_tile_grid(&mut r, ctx, 256);
    MaskData { grid, extra: ctx.finish(r) }
}

/// Füll-Handles (`PCFb` optional).
fn read_fill_handles(r: &mut Rd, with_pcf: bool) -> FillHandles {
    let valid = if with_pcf { r.bool(k!(b"PCFb")).unwrap_or(false) } else { false };
    FillHandles {
        valid,
        start: Point::new(r.f32(k!(b"PSXf")).unwrap_or(0.0), r.f32(k!(b"PSYf")).unwrap_or(0.0)),
        end: Point::new(r.f32(k!(b"PEXf")).unwrap_or(0.0), r.f32(k!(b"PEYf")).unwrap_or(0.0)),
        second: Point::new(r.f32(k!(b"PFXf")).unwrap_or(0.0), r.f32(k!(b"PFYf")).unwrap_or(0.0)),
    }
}

/// Pfad-Inhalt (ohne gemeinsame Attribute und Cache).
fn read_path(r: &mut Rd, ctx: &mut ReadCtx) -> PathElem {
    let attrs = Arc::new(r.list(k!(b"PATv")).map(|v| read_attrs(v, ctx)).unwrap_or_default());
    let stroke_placement = r.i32(k!(b"BRPi")).unwrap_or(1);
    let ptt = r.i32(k!(b"PTTi")).unwrap_or(0);
    let fill_on_top = r.bool(k!(b"FOTb")).unwrap_or(false);
    let random_seed = r.int(k!(b"RNDi")).unwrap_or(0);
    let compat_proxy = r.bool(k!(b"PRPb")).unwrap_or(false);
    let texture_offset = Point::new(r.f32(k!(b"TOXf")).unwrap_or(0.0), r.f32(k!(b"TOYf")).unwrap_or(0.0));
    let fill_handles = read_fill_handles(r, true);
    let mut contours = Vec::new();
    let mut contours_extra = Extra::default();
    if let Some(v) = r.strict_list(k!(b"PBLv"), k!(b"PBPv")) {
        let mut s = Rd::new(v);
        for n in s.take_all(k!(b"PBPv")) {
            contours.push(read_contour(n.children(), ctx));
        }
        contours_extra = ctx.finish(s);
    }
    let mut polyline = None;
    let mut polyline_extra = Extra::default();
    if let Some(v) = r.strict_list(k!(b"PCLv"), k!(b"PPLv")) {
        let mut s = Rd::new(v);
        polyline = Some(s.take_all(k!(b"PPLv")).into_iter().map(|n| read_polyline(n.children(), ctx)).collect());
        polyline_extra = ctx.finish(s);
    }
    let primitive = r.list(k!(b"PRIv")).map(|v| read_primitive(v, ctx));
    let path_op = r.int(k!(b"POPi")).unwrap_or(0);
    let even_odd = r.bool(k!(b"EOFb")).unwrap_or(false);
    let agm = r.bool(k!(b"AGMb")).unwrap_or(false);
    PathElem {
        attrs,
        stroke_placement,
        ptt,
        fill_on_top,
        random_seed,
        texture_offset,
        fill_handles,
        contours,
        contours_extra,
        polyline,
        polyline_extra,
        even_odd,
        agm,
        path_op,
        compat_proxy,
        primitive,
    }
}

/// Kontur `PBPv` (auch Polygon von Web-Objekten).
pub fn read_contour(v: &[RawNode], ctx: &mut ReadCtx) -> Contour {
    let mut r = Rd::new(v);
    let closed = r.bool(k!(b"ISCb")).unwrap_or(false);
    let bsl = r.f32(k!(b"BSLf"));
    let nodes_raw = r.take_all(k!(b"PBTv"));
    let nodes: Vec<ContourNode> = nodes_raw.iter().map(|n| read_contour_node(n.children(), ctx)).collect();
    // PPCi ist abgeleitet; passt es nicht zur Knotenzahl, bleibt es roh.
    if r.peek_int(k!(b"PPCi")) == Some(nodes.len() as u32) {
        r.int(k!(b"PPCi"));
    }
    Contour { closed, bsl, nodes, extra: ctx.finish(r) }
}

fn read_contour_node(v: &[RawNode], ctx: &mut ReadCtx) -> ContourNode {
    let mut r = Rd::new(v);
    let ax = r.f32(k!(b"XLCf")).unwrap_or(0.0);
    let ay = r.f32(k!(b"YLCf")).unwrap_or(0.0);
    let pred = Point::new(r.f32(k!(b"XPCf")).unwrap_or(ax), r.f32(k!(b"YPCf")).unwrap_or(ay));
    let succ = Point::new(r.f32(k!(b"XSCf")).unwrap_or(ax), r.f32(k!(b"YSCf")).unwrap_or(ay));
    let curve = r.bool(k!(b"CRVb")).unwrap_or(false);
    let random = r.int(k!(b"RNDi"));
    let segment_length = r.f32(k!(b"NSDf"));
    let pressure = r.list(k!(b"BZLv")).map(|b| {
        let mut s = Rd::new(b);
        let count = s.int(k!(b"BZCi"));
        let samples = s
            .take_all(k!(b"BAZv"))
            .into_iter()
            .map(|z| {
                let mut q = Rd::new(z.children());
                let pressure = q.f32(k!(b"PRSf"));
                let velocity = q.f32(k!(b"VELf"));
                PressureSample { pressure, velocity, extra: ctx.finish(q) }
            })
            .collect();
        PressureProfile { count, samples, extra: ctx.finish(s) }
    });
    ContourNode { anchor: Point::new(ax, ay), pred, succ, curve, random, segment_length, pressure, extra: ctx.finish(r) }
}

fn read_polyline(v: &[RawNode], ctx: &mut ReadCtx) -> Polyline {
    let mut r = Rd::new(v);
    let closed = r.bool(k!(b"ISCb")).unwrap_or(false);
    let pts = r.take_all(k!(b"PPTv"));
    let points: Vec<PolyPoint> = pts
        .iter()
        .map(|p| {
            let mut q = Rd::new(p.children());
            let pos = Point::new(q.f32(k!(b"XLCf")).unwrap_or(0.0), q.f32(k!(b"YLCf")).unwrap_or(0.0));
            let pressure = q.f32(k!(b"PRSf")).unwrap_or(0.0);
            let velocity = q.f32(k!(b"VELf")).unwrap_or(0.0);
            let duration = q.int(k!(b"DURi")).unwrap_or(0);
            let random = q.int(k!(b"RNDi")).unwrap_or(0);
            let on_node = q.bool(k!(b"ONCb"));
            PolyPoint { pos, pressure, velocity, duration, random, on_node, extra: ctx.finish(q) }
        })
        .collect();
    if r.peek_int(k!(b"PPCi")) == Some(points.len() as u32) {
        r.int(k!(b"PPCi"));
    }
    Polyline { closed, points, extra: ctx.finish(r) }
}

fn read_primitive(v: &[RawNode], ctx: &mut ReadCtx) -> RectPrimitive {
    let mut r = Rd::new(v);
    let fill_handles = read_fill_handles(&mut r, true);
    let transform = r.matrix(k!(b"MTXv")).unwrap_or_default();
    let effects = r.list(k!(b"EFLv")).map(|e| read_effect_list(e, ctx));
    let (rect, roundness, rect_extra) = match r.list(k!(b"RCTv")) {
        Some(rc) => {
            let mut s = Rd::new(rc);
            let roundness = s.f32(k!(b"RDSf")).unwrap_or(0.0);
            let rect = Rect::new(
                s.f32(LTRB[0]).unwrap_or(0.0),
                s.f32(LTRB[1]).unwrap_or(0.0),
                s.f32(LTRB[2]).unwrap_or(0.0),
                s.f32(LTRB[3]).unwrap_or(0.0),
            );
            (rect, roundness, ctx.finish(s))
        }
        None => (Rect::default(), 0.0, Extra::default()),
    };
    let roundness_pixels = r.bool(k!(b"RDMb"));
    RectPrimitive { fill_handles, transform, effects, rect, roundness, rect_extra, roundness_pixels, extra: ctx.finish(r) }
}

/// Attribute `PATv`.
pub fn read_attrs(v: &[RawNode], ctx: &mut ReadCtx) -> PathAttrs {
    let mut r = Rd::new(v);
    let stroke_color = Argb(r.int(k!(b"BCLi")).unwrap_or(0xff00_0000));
    let fill_color = Argb(r.int(k!(b"FCLi")).unwrap_or(0xff00_00ff));
    let brush = r.list(k!(b"BPLv")).map(|b| read_brush(b, ctx));
    let fill = r.list(k!(b"FPLv")).map(|f| read_fill(f, ctx));
    let stroke_texture = r.list(k!(b"TXBv")).map(|t| read_texture(t, ctx));
    let fill_texture = r.list(k!(b"TXFv")).map(|t| read_texture(t, ctx));
    PathAttrs { stroke_color, fill_color, brush, fill, stroke_texture, fill_texture, extra: ctx.finish(r) }
}

fn read_texture(v: &[RawNode], ctx: &mut ReadCtx) -> Texture {
    let mut r = Rd::new(v);
    let name = r.str(k!(b"INMs")).unwrap_or_default();
    let user_name = r.str(k!(b"UNMs"));
    let mask = r.list(k!(b"MSKv")).map(|m| read_mask(m, ctx));
    Texture { name, user_name, mask, extra: ctx.finish(r) }
}

/// Füllung `FPLv`.
pub fn read_fill(v: &[RawNode], ctx: &mut ReadCtx) -> Fill {
    let mut r = Rd::new(v);
    let category = r.str(k!(b"CATs")).unwrap_or_default();
    let name = r.str(k!(b"INMs")).unwrap_or_default();
    let user_name = r.str(k!(b"UNMs"));
    let read_only = r.bool(k!(b"RDOb")).unwrap_or(false);
    let texture_amount = r.i32(k!(b"FTBi")).unwrap_or(0);
    let feather = r.i32(k!(b"FEFi")).unwrap_or(0);
    let stamping_mode = r.i32(k!(b"FSMi")).unwrap_or(1);
    let edge = r.i32(k!(b"FETi")).unwrap_or(1);
    let dither = r.bool(k!(b"FRDb")).unwrap_or(false);
    let dither_amount = r.i32(k!(b"FRRi")).unwrap_or(0);
    let shape = r.i32(k!(b"FSHi")).unwrap_or(0);
    let shape_ext = r.i32(k!(b"FSXi")).unwrap_or(0);
    let keys = [k!(b"FD1i"), k!(b"FD2i"), k!(b"FD3i"), k!(b"FD4i")];
    let dither_colors = if keys.iter().all(|k| r.peek_int(*k).is_some()) {
        let mut a = [0u32; 4];
        for (i, k) in keys.iter().enumerate() {
            a[i] = r.int(*k).unwrap_or(0);
        }
        Some(a)
    } else {
        None
    };
    let transparent_dither = r.bool(k!(b"FDTb"));
    let gradient = r.list(k!(b"FGLv")).map(|g| read_gradient(g, ctx));
    let pattern = r.list(k!(b"FPTv")).map(|p| {
        let mut s = Rd::new(p);
        let name = s.str(k!(b"INMs")).unwrap_or_default();
        let user_name = s.str(k!(b"UNMs"));
        let image = take_bitmap(&mut s, ctx);
        Pattern { name, user_name, image, extra: ctx.finish(s) }
    });
    Fill {
        category,
        name,
        user_name,
        read_only,
        texture_amount,
        feather,
        stamping_mode,
        edge,
        dither,
        dither_amount,
        shape,
        shape_ext,
        dither_colors,
        transparent_dither,
        gradient,
        pattern,
        extra: ctx.finish(r),
    }
}

/// Verlauf `FGLv`.
pub fn read_gradient(v: &[RawNode], ctx: &mut ReadCtx) -> Gradient {
    let mut r = Rd::new(v);
    let category = r.str(k!(b"CATs")).unwrap_or_default();
    let name = r.str(k!(b"INMs")).unwrap_or_default();
    let user_name = r.str(k!(b"UNMs"));
    let legacy_stops = r.list(k!(b"FGVv")).map(|s| read_stops(s, ctx));
    let stops = r.list(k!(b"FGYv")).map(|y| {
        let mut s = Rd::new(y);
        let colors = s.list(k!(b"FG0v")).map(|c| read_stops(c, ctx)).unwrap_or_default();
        let opacities = s.list(k!(b"FG1v")).map(|c| read_stops(c, ctx)).unwrap_or_default();
        let dither = s.bool(k!(b"FGDb"));
        let dither_value = s.i32(k!(b"FGDi"));
        GradientStops { colors, opacities, dither, dither_value, extra: ctx.finish(s) }
    });
    Gradient { category, name, user_name, legacy_stops, stops, extra: ctx.finish(r) }
}

fn read_stops(v: &[RawNode], ctx: &mut ReadCtx) -> StopList {
    let mut r = Rd::new(v);
    let stops: Vec<GradientStop> = r
        .take_all(k!(b"FGIv"))
        .into_iter()
        .map(|s| {
            let mut q = Rd::new(s.children());
            let position = q.f32(k!(b"FGPf")).unwrap_or(0.0);
            let color = Argb(q.int(k!(b"FGCi")).unwrap_or(0xff00_0000));
            GradientStop { position, color, extra: ctx.finish(q) }
        })
        .collect();
    if r.peek_int(k!(b"FNCi")) == Some(stops.len() as u32) {
        r.int(k!(b"FNCi"));
    }
    StopList { stops, extra: ctx.finish(r) }
}

/// Pinsel `BPLv`.
pub fn read_brush(v: &[RawNode], ctx: &mut ReadCtx) -> Brush {
    let mut r = Rd::new(v);
    let category = r.str(k!(b"CATs")).unwrap_or_default();
    let name = r.str(k!(b"INMs")).unwrap_or_default();
    let user_name = r.str(k!(b"UNMs"));
    let read_only = r.bool(k!(b"RDOb")).unwrap_or(false);
    let mut g = |key: TagKey| r.i32(key).unwrap_or(0);
    let angle = g(k!(b"BANi"));
    let aspect = g(k!(b"BASi"));
    let diameter = g(k!(b"BDIi"));
    let max_count = g(k!(b"BMMi"));
    let min_size = g(k!(b"BMSi"));
    let softness = g(k!(b"BSEi"));
    let soften_mode = g(k!(b"BSFi"));
    let shape = g(k!(b"BSHi"));
    let bbl = g(k!(b"BBLi"));
    let blackness = g(k!(b"BBKi"));
    let concentration = g(k!(b"BCNi"));
    let alpha_remap = g(k!(b"BEFi"));
    let brush_type = g(k!(b"BRTi"));
    let feedback = g(k!(b"BFBi"));
    let flow_rate = g(k!(b"BFRi"));
    let tip_count = g(k!(b"BNTi"));
    let spacing = g(k!(b"BSPi"));
    let texture_blend = g(k!(b"BTBi"));
    let texture_edge = g(k!(b"BTEi"));
    let tip_spacing = g(k!(b"BTSi"));
    let tip_spacing_mode = g(k!(b"BSMi"));
    let tip_coloring_mode = g(k!(b"BCMi"));
    let antialiased = r.bool(k!(b"BIAb")).unwrap_or(false);
    let mut g = |key: TagKey| r.i32(key).unwrap_or(0);
    let dash_count = g(k!(b"NDIi"));
    let dash_on = [g(k!(b"DO1i")), g(k!(b"DO2i")), g(k!(b"DO3i"))];
    let dash_off = [g(k!(b"DF1i")), g(k!(b"DF2i")), g(k!(b"DF3i"))];
    let mut sensitivities = Vec::new();
    for (i, n) in v.iter().enumerate() {
        let t = n.key.0;
        if t[0] == b'S'
            && t[3] == b'i'
            && attrs::SENSITIVITY_SOURCES.contains(&t[1])
            && attrs::SENSITIVITY_TARGETS.contains(&t[2])
            && !r.is_used(i)
            && v.iter().filter(|m| m.key == n.key).count() == 1
            && let Some(val) = n.as_int() {
                r.mark(i);
                sensitivities.push(Sensitivity { source: t[1], target: t[2], value: val as i32 });
            }
    }
    Brush {
        category,
        name,
        user_name,
        read_only,
        angle,
        aspect,
        diameter,
        max_count,
        min_size,
        softness,
        soften_mode,
        shape,
        bbl,
        blackness,
        concentration,
        alpha_remap,
        brush_type,
        feedback,
        flow_rate,
        tip_count,
        spacing,
        texture_blend,
        texture_edge,
        tip_spacing,
        tip_spacing_mode,
        tip_coloring_mode,
        antialiased,
        dash_count,
        dash_on,
        dash_off,
        sensitivities,
        extra: ctx.finish(r),
    }
}

/// Effektliste `EFLv`.
pub fn read_effect_list(v: &[RawNode], ctx: &mut ReadCtx) -> EffectList {
    let mut r = Rd::new(v);
    let category = r.str(k!(b"CATs")).unwrap_or_default();
    let name = r.str(k!(b"INMs")).unwrap_or_default();
    let user_name = r.str(k!(b"UNMs"));
    let mut effects = Vec::new();
    for (i, n) in v.iter().enumerate() {
        let tag = match &n.key.0 {
            b"EPSv" => EffectTag::Eps,
            b"EPDv" => EffectTag::Epd,
            _ => continue,
        };
        r.mark(i);
        effects.push(Effect { tag, params: read_dict(n.children(), ctx) });
    }
    EffectList { category, name, user_name, effects, extra: ctx.finish(r) }
}

/// Wörterbuch (`DTAv`, `EPSv`, `GDTv`).
pub fn read_dict(v: &[RawNode], ctx: &mut ReadCtx) -> DataDict {
    let mut r = Rd::new(v);
    let mut entries = Vec::new();
    for (i, n) in v.iter().enumerate() {
        let tag = match &n.key.0 {
            b"DCEv" => EntryTag::Dce,
            b"DCFv" => EntryTag::Dcf,
            _ => continue,
        };
        if let Some(e) = read_entry(n.children(), tag, ctx) {
            r.mark(i);
            entries.push(e);
        }
    }
    DataDict { entries, extra: ctx.finish(r) }
}

fn read_entry(v: &[RawNode], tag: EntryTag, ctx: &mut ReadCtx) -> Option<DataEntry> {
    let mut r = Rd::new(v);
    let key = r.str(k!(b"DCKs"))?;
    let structured = if let Some(p) = r.list(k!(b"GPLv")) {
        Some(DataValue::Points(read_points(p, ctx)))
    } else if let Some(l) = r.list(k!(b"GVRv")) {
        Some(DataValue::List(read_data_list(l, ctx)))
    } else {
        r.list(k!(b"GDTv")).map(|d| DataValue::Dict(read_dict(d, ctx)))
    };
    let text = r.str(k!(b"DCVs"));
    let (value, text) = match structured {
        Some(s) => (s, text),
        None => match text {
            Some(t) => (DataValue::Scalar(t), None),
            None => (DataValue::None, None),
        },
    };
    Some(DataEntry { tag, key, value, text, extra: ctx.finish(r) })
}

fn read_data_list(v: &[RawNode], ctx: &mut ReadCtx) -> DataList {
    let mut r = Rd::new(v);
    let mut items = Vec::new();
    for (i, n) in v.iter().enumerate() {
        let item = match (&n.key.0, &n.value) {
            (b"VVLs", RawValue::Str(s)) => DataValue::Scalar(s.clone()),
            (b"GDTv", RawValue::List(k)) => DataValue::Dict(read_dict(k, ctx)),
            (b"GVRv", RawValue::List(k)) => DataValue::List(read_data_list(k, ctx)),
            (b"GPLv", RawValue::List(k)) => DataValue::Points(read_points(k, ctx)),
            _ => continue,
        };
        r.mark(i);
        items.push(item);
    }
    DataList { items, extra: ctx.finish(r) }
}

fn read_points(v: &[RawNode], ctx: &mut ReadCtx) -> PointList {
    let mut r = Rd::new(v);
    let mut points = Vec::new();
    let mut point_extra = Vec::new();
    for n in r.take_all(k!(b"GPTv")) {
        let mut q = Rd::new(n.children());
        points.push(Point::new(q.f32(k!(b"XLCf")).unwrap_or(0.0), q.f32(k!(b"YLCf")).unwrap_or(0.0)));
        point_extra.push(ctx.finish(q));
    }
    PointList { points, point_extra, extra: ctx.finish(r) }
}

/// Textattribute (Kopf bzw. Attributblock).
fn read_text_attrs(r: &mut Rd) -> TextAttrs {
    TextAttrs {
        font: r.str(k!(b"FONs")),
        ps_font: r.str(k!(b"CFTs")),
        size: r.f32(k!(b"PTSf")),
        bold: r.bool(k!(b"BOLb")),
        italic: r.bool(k!(b"ITLb")),
        underline: r.bool(k!(b"UNDb")),
        horizontal_scale: r.f32(k!(b"HSCf")),
        kerning: r.f32(k!(b"KRNf")),
        tracking: r.f32(k!(b"RKNf")),
        baseline_shift: r.f32(k!(b"BLSf")),
        leading: r.f32(k!(b"LEDf")),
        leading_mode: r.i32(k!(b"LDMi")),
        alignment: r.i32(k!(b"JSTi")),
        color: r.int(k!(b"FCLi")),
        indent: r.f32(k!(b"PINf")),
        space_before: r.f32(k!(b"PSBf")),
        space_after: r.f32(k!(b"PSAf")),
    }
}

fn read_text(r: &mut Rd, ctx: &mut ReadCtx) -> TextElem {
    let bounds = Rect::new(
        r.f32(LTRB[0]).unwrap_or(0.0),
        r.f32(LTRB[1]).unwrap_or(0.0),
        r.f32(LTRB[2]).unwrap_or(250.0),
        r.f32(LTRB[3]).unwrap_or(250.0),
    );
    let anchor_rect = r.rect4([k!(b"LFAf"), k!(b"TOAf"), k!(b"RIAf"), k!(b"BOAf")]);
    let frame_type = r.i32(k!(b"TFTi"));
    let story_index = r.int(k!(b"AIDi"));
    let from_story = r.bool(k!(b"FOAb"));
    let attrs = Arc::new(r.list(k!(b"PATv")).map(|v| read_attrs(v, ctx)).unwrap_or_default());
    let stroke_placement = r.i32(k!(b"BRPi")).unwrap_or(1);
    let ptt = r.i32(k!(b"PTTi")).unwrap_or(0);
    let fill_on_top = r.bool(k!(b"FOTb")).unwrap_or(false);
    let random_seed = r.int(k!(b"RNDi")).unwrap_or(0);
    let texture_offset = Point::new(r.f32(k!(b"TOXf")).unwrap_or(0.0), r.f32(k!(b"TOYf")).unwrap_or(0.0));
    let orientation = r.i32(k!(b"ORIi")).unwrap_or(0);
    let fill_handles = read_fill_handles(r, true);
    let transform = r.matrix(k!(b"MTXv"));
    let transform_mode = r.i32(k!(b"XFMi")).unwrap_or(1);
    let antialias = r.i32(k!(b"TAAi")).unwrap_or(0);
    let aa_sharpness = r.i32(k!(b"TA1i")).unwrap_or(192);
    let aa_strength = r.i32(k!(b"TA2i")).unwrap_or(64);
    let aa_oversample = r.i32(k!(b"TOSi")).unwrap_or(8);
    let ate = r.bool(k!(b"ATEb"));
    let auto_kern = r.bool(k!(b"ATKb")).unwrap_or(true);
    let initial = read_text_attrs(r);
    let on_path = r.list(k!(b"TAPv")).map(|t| {
        let mut s = Rd::new(t);
        let mode = s.i32(k!(b"TAMi"));
        let offset = s.f32(k!(b"CHOf"));
        let path = s.list(k!(b"PTHv")).map(|p| Arc::new(read_element(&RawNode::list(k!(b"PTHv"), p.to_vec()), ctx)));
        TextOnPath { mode, offset, path, extra: ctx.finish(s) }
    });
    let mut runs = Vec::new();
    let mut trailing_attrs = None;
    let mut runs_extra = Extra::default();
    let mut has_runs_node = false;
    if let Some(t) = r.list(k!(b"TFSv")) {
        has_runs_node = true;
        let (rs, tr, ex) = read_runs(t, ctx);
        runs = rs;
        trailing_attrs = tr;
        runs_extra = ex;
    }
    TextElem {
        bounds,
        anchor_rect,
        frame_type,
        story_index,
        from_story,
        attrs,
        stroke_placement,
        ptt,
        fill_on_top,
        random_seed,
        texture_offset,
        orientation,
        fill_handles,
        transform,
        transform_mode,
        antialias,
        aa_sharpness,
        aa_strength,
        aa_oversample,
        ate,
        auto_kern,
        initial,
        runs,
        trailing_attrs,
        has_runs_node,
        runs_extra,
        on_path,
    }
}

/// Läufe in `TFSv`: `[Attributblock] TRNs …`; jeder Block wird mit eigener Reihenfolge gelesen.
fn read_runs(v: &[RawNode], ctx: &mut ReadCtx) -> (Vec<TextRun>, Option<TextRun>, Extra) {
    let mut outer = Rd::new(v);
    let mut runs = Vec::new();
    let mut start = 0usize;
    for (i, n) in v.iter().enumerate() {
        if n.key == k!(b"TRNs") {
            let RawValue::Str(text) = &n.value else { continue };
            let block = &v[start..i];
            let mut br = Rd::new(block);
            let attrs = read_text_attrs(&mut br);
            let extra = ctx.finish(br);
            for j in start..=i {
                outer.mark(j);
            }
            runs.push(TextRun { attrs, text: text.clone(), extra });
            start = i + 1;
        }
    }
    let trailing = if start < v.len() {
        let block = &v[start..];
        let mut br = Rd::new(block);
        let attrs = read_text_attrs(&mut br);
        let extra = ctx.finish(br);
        for j in start..v.len() {
            outer.mark(j);
        }
        Some(TextRun { attrs, text: String::new(), extra })
    } else {
        None
    };
    // Die Blöcke tragen ihre Reihenfolge selbst; der Container braucht nur die Markierung „gelesen“.
    let mut ex = ctx.finish(outer);
    ex.order = Some(ctx.interner.get(Vec::new()));
    (runs, trailing, ex)
}

fn read_group(r: &mut Rd, ctx: &mut ReadCtx) -> GroupElem {
    let kind = GroupKind::from_code(r.i32(k!(b"GRTi")).unwrap_or(0));
    let mask_flags = MaskFlags {
        linked: r.bool(k!(b"LNKb")).unwrap_or(true),
        enabled: r.bool(k!(b"MENb")).unwrap_or(true),
        hide_attrs: r.bool(k!(b"MNAb")).unwrap_or(false),
        auto_expand: r.bool(k!(b"MRXb")).unwrap_or(false),
    };
    let (mut children, children_extra) = match r.list(k!(b"ELMv")) {
        Some(v) => read_elements(v, ctx),
        None => (Vec::new(), Extra::default()),
    };
    let mask = if kind.is_mask() && !children.is_empty() { Some(children.remove(0)) } else { None };
    let mut replacement = None;
    let mut replacement_extra = Extra::default();
    if let Some(m) = r.list(k!(b"MRMv")) {
        let (els, ex) = read_elements(m, ctx);
        replacement = Some(els);
        replacement_extra = ex;
    }
    let smart_shape = r.list(k!(b"ISSv")).map(|s| read_smart_shape(s, ctx));
    GroupElem { kind, mask, children, children_extra, mask_flags, replacement, replacement_extra, smart_shape }
}

fn read_smart_shape(v: &[RawNode], ctx: &mut ReadCtx) -> SmartShape {
    let mut r = Rd::new(v);
    let name = r.str(k!(b"FILs")).unwrap_or_default();
    let mut script_chunks = Vec::new();
    let mut script_extra = Extra::default();
    if let Some(j) = r.strict_list(k!(b"JSVv"), k!(b"JSSs")) {
        let mut s = Rd::new(j);
        let mut ok = true;
        for n in j {
            match n.as_str() {
                Some(t) => script_chunks.push(t.to_string()),
                None => ok = false,
            }
        }
        if ok {
            s.take_all(k!(b"JSSs"));
        } else {
            script_chunks.clear();
        }
        script_extra = ctx.finish(s);
    }
    let transform = r.matrix(k!(b"MTXv"));
    let mut control_points = None;
    let mut control_points_extra = Extra::default();
    if let Some(c) = r.strict_list(k!(b"CPSv"), k!(b"CPTv")) {
        let mut s = Rd::new(c);
        control_points = Some(
            s.take_all(k!(b"CPTv"))
                .into_iter()
                .map(|p| {
                    let mut q = Rd::new(p.children());
                    let position = Point::new(q.f32(k!(b"XLCf")).unwrap_or(0.0), q.f32(k!(b"YLCf")).unwrap_or(0.0));
                    let kind = q.i32(k!(b"CTYi")).unwrap_or(0);
                    let name = q.str(k!(b"OBNs"));
                    let tooltip = q.str(k!(b"TTPs"));
                    let tooltip_tracks_drag = q.bool(k!(b"TTDb")).unwrap_or(false);
                    let visible = q.bool(k!(b"VISb")).unwrap_or(true);
                    ControlPoint { position, kind, name, tooltip, tooltip_tracks_drag, visible, extra: ctx.finish(q) }
                })
                .collect(),
        );
        control_points_extra = ctx.finish(s);
    }
    let mut events = None;
    let mut events_extra = Extra::default();
    if let Some(e) = r.strict_list(k!(b"EVVv"), k!(b"EVTs"))
        && e.iter().all(|n| n.as_str().is_some()) {
            let mut s = Rd::new(e);
            events = Some(s.take_all(k!(b"EVTs")).into_iter().filter_map(|n| n.as_str().map(String::from)).collect());
            events_extra = ctx.finish(s);
        }
    SmartShape {
        name,
        script_chunks,
        script_extra,
        transform,
        control_points,
        control_points_extra,
        events,
        events_extra,
        extra: ctx.finish(r),
    }
}

fn read_button_strings(r: &mut Rd) -> ButtonStrings {
    ButtonStrings {
        url: r.str(k!(b"INMs")),
        alt: r.str(k!(b"A2Ts")),
        target: r.str(k!(b"ALTs")),
        file_name: r.str(k!(b"FILs")),
        text: r.str(k!(b"BUTs")),
    }
}

fn read_instance(r: &mut Rd, ctx: &mut ReadCtx) -> Option<InstanceElem> {
    let symbol_id = SymbolId(r.int(k!(b"MSNi"))?);
    let instance_id = r.int(k!(b"ALNi"));
    let transform = r.matrix(k!(b"MTXv"))?;
    let alt_transform = r.matrix(k!(b"ATMv"));
    let transform_mode = r.i32(k!(b"XFMi")).unwrap_or(1);
    let anim = AnimParams {
        frames: r.int(k!(b"ANNi")).unwrap_or(5),
        offset: Point::new(r.f32(k!(b"ANXf")).unwrap_or(0.0), r.f32(k!(b"ANYf")).unwrap_or(0.0)),
        rotation: r.i32(k!(b"ANRi")).unwrap_or(0),
        scale: r.f32(k!(b"ASCf")).unwrap_or(100.0),
        start_opacity: r.int(k!(b"ASOi")).unwrap_or(1000),
        end_opacity: r.int(k!(b"AEOi")).unwrap_or(1000),
    };
    let widget_params = r.str(k!(b"WPVs"));
    let button = read_button_strings(r);
    let show_down_on_load = r.bool(k!(b"DVLb"));
    let behaviors = r.list(k!(b"BEVv")).map(|b| read_behaviors(b, ctx));
    Some(InstanceElem {
        symbol_id,
        instance_id,
        transform,
        alt_transform,
        transform_mode,
        anim,
        widget_params,
        button,
        show_down_on_load,
        behaviors,
    })
}

fn read_web(r: &mut Rd, ctx: &mut ReadCtx) -> WebObjectElem {
    let url = r.str(k!(b"INMs"));
    let alt_text = r.str(k!(b"A2Ts"));
    let target = r.str(k!(b"ALTs"));
    let color = Argb(r.int(k!(b"FCLi")).unwrap_or(0xff00_0000));
    let shape = WebShape::from_code(r.i32(k!(b"URSi")).unwrap_or(0));
    let bounds = Rect::new(
        r.f32(LTRB[0]).unwrap_or(0.0),
        r.f32(LTRB[1]).unwrap_or(0.0),
        r.f32(LTRB[2]).unwrap_or(0.0),
        r.f32(LTRB[3]).unwrap_or(0.0),
    );
    let polygon = r.list(k!(b"PBPv")).map(|p| read_contour(p, ctx));
    let behaviors = r.list(k!(b"BEVv")).map(|b| read_behaviors(b, ctx));
    let file_name = r.str(k!(b"FILs"));
    let slice = r.i32(k!(b"TSLi")).map(|slice_type| {
        let html = r.str(k!(b"HTMs"));
        let td_text = r.str(k!(b"TDTs"));
        let slice_id = r.int(k!(b"MSNi"));
        let export = r.list(k!(b"EXPv")).map(|e| Box::new(read_export(e, ctx)));
        let bk = [k!(b"CBTb"), k!(b"CBRi"), k!(b"CBAi"), k!(b"CBHi"), k!(b"CHVi"), k!(b"CBVi"), k!(b"CVVi")];
        let background = if bk.iter().all(|k| r.has(*k)) {
            let is_background = r.bool(bk[0]);
            let repeat = r.i32(bk[1]);
            let attachment = r.i32(bk[2]);
            let h_align = r.i32(bk[3]);
            let h_value = r.i32(bk[4]);
            let v_align = r.i32(bk[5]);
            let v_value = r.i32(bk[6]);
            match (is_background, repeat, attachment, h_align, h_value, v_align, v_value) {
                (Some(a), Some(b), Some(c), Some(d), Some(e), Some(f), Some(g)) => Some(SliceBackground {
                    is_background: a,
                    repeat: b,
                    attachment: c,
                    h_align: d,
                    h_value: e,
                    v_align: f,
                    v_value: g,
                }),
                _ => None,
            }
        } else {
            None
        };
        SliceProps { slice_type, html, td_text, slice_id, export, background }
    });
    let legacy_status = r.str(k!(b"UNMs"));
    WebObjectElem { url, alt_text, target, color, shape, bounds, polygon, file_name, slice, behaviors, legacy_status }
}

fn read_behaviors(v: &[RawNode], ctx: &mut ReadCtx) -> Behaviors {
    let mut r = Rd::new(v);
    let mut items = Vec::new();
    for (i, n) in v.iter().enumerate() {
        if n.key != k!(b"BEHv") {
            continue;
        }
        if let Some(b) = read_behavior(n.children(), ctx) {
            r.mark(i);
            items.push(b);
        }
    }
    Behaviors { items, extra: ctx.finish(r) }
}

fn read_behavior(v: &[RawNode], ctx: &mut ReadCtx) -> Option<Behavior> {
    let mut r = Rd::new(v);
    let action = r.i32(k!(b"BETi"))?;
    let event = r.i32(k!(b"BEEi"))?;
    let mut b = Behavior::new(BehaviorAction::from_code(action), event);
    b.flags = r.i32(k!(b"ROLi"));
    b.checked = r.bool(k!(b"CHKb"));
    b.status_text = r.str(k!(b"UNMs"));
    b.target_slice = r.int(k!(b"MSNi"));
    b.frame = r.i32(k!(b"FRCi"));
    b.restore = r.bool(k!(b"RMOb"));
    b.preload = r.bool(k!(b"PREb"));
    b.down_frame = r.i32(k!(b"FRTi"));
    b.file = r.str(k!(b"FILs"));
    b.end = r.bool(k!(b"ENDb"));
    if action == 9 {
        let mut p = PopupMenu {
            creation_date: r.str(k!(b"CDTs")),
            horz_offset: r.f32(k!(b"XLCf")),
            vert_offset: r.f32(k!(b"YLCf")),
            items_text: r.str(k!(b"TRNs")),
            text_up_color: r.int(k!(b"FCLi")),
            text_over_color: r.int(k!(b"FETi")),
            text_alignment: r.i32(k!(b"JSTi")),
            vert_alignment: r.i32(k!(b"VERi")),
            hide_timeout: r.i32(k!(b"TMOi")),
            horz_submenu_offset: r.f32(k!(b"XSLf")),
            vert_submenu_offset: r.f32(k!(b"YSLf")),
            submenu_relative: r.bool(k!(b"RELb")),
            item_spacing: r.i32(k!(b"CSPi")),
            item_padding: r.i32(k!(b"CPDi")),
            opaque: r.bool(k!(b"OPQb")),
            hilite_color: r.int(k!(b"HICi")),
            shadow_color: r.int(k!(b"CSHi")),
            border_color: r.int(k!(b"BOCi")),
            border_size: r.i32(k!(b"BOSi")),
            override_width: r.bool(k!(b"OVWb")),
            override_height: r.bool(k!(b"OVHb")),
            text_indent: r.i32(k!(b"MITi")),
            vertical: r.bool(k!(b"MIVb")),
            menu_id: r.i32(k!(b"MIDi")),
            menu_width: r.i32(k!(b"MIWi")),
            menu_height: r.i32(k!(b"MIHi")),
            hide_on_mouseout: r.bool(k!(b"HIMb")),
            bold: r.bool(k!(b"BOLb")),
            italic: r.bool(k!(b"ITLb")),
            font: r.str(k!(b"FONs")),
            size: r.f32(k!(b"PTSf")),
            text_only: r.bool(k!(b"ATEb")),
            cell_up_color: r.int(k!(b"BCLi")),
            cell_over_color: r.int(k!(b"BCMi")),
            styles: Vec::new(),
        };
        p.styles = r.take_all(k!(b"ZSTv")).into_iter().map(|s| read_style(s.children(), ctx)).collect();
        b.popup = Some(Box::new(p));
    }
    b.down_file = r.str(k!(b"INMs"));
    b.extra = ctx.finish(r);
    Some(b)
}

/// Exporteinstellungen `EXPv`.
pub fn read_export(v: &[RawNode], ctx: &mut ReadCtx) -> ExportSettings {
    let mut r = Rd::new(v);
    let d = ExportSettings::options_only();
    let format = r.i32(k!(b"FMTi")).unwrap_or(d.format);
    let mac_type = r.int(k!(b"MCTi"));
    let mac_creator = r.int(k!(b"MCCi"));
    let color_mode = r.i32(k!(b"CLRi")).unwrap_or(d.color_mode);
    let palette_mode = r.i32(k!(b"PLTi")).unwrap_or(d.palette_mode);
    let mut palette = Vec::new();
    let mut palette_extra = Extra::default();
    if let Some(p) = r.peek_list(k!(b"PALv"))
        && let Some(entries) = read_palette(p, ctx) {
            r.list(k!(b"PALv"));
            palette = entries;
            let s = Rd::new(&[]);
            palette_extra = ctx.finish(s);
        }
    let palette_count = r.i32(k!(b"PLNi")).unwrap_or(0);
    let entries_requested = r.i32(k!(b"PLRi")).unwrap_or(d.entries_requested);
    let transparency = r.i32(k!(b"PTNi")).unwrap_or(0);
    let optimized = r.bool(k!(b"PLOb")).unwrap_or(d.optimized);
    let local_adaptive = r.bool(k!(b"APLb")).unwrap_or(d.local_adaptive);
    let websnap_adaptive = r.bool(k!(b"APSb")).unwrap_or(d.websnap_adaptive);
    let anim_auto_crop = r.bool(k!(b"AACb")).unwrap_or(d.anim_auto_crop);
    let anim_auto_difference = r.bool(k!(b"AADb")).unwrap_or(d.anim_auto_difference);
    let websnap_tolerance = r.i32(k!(b"ASTi")).unwrap_or(d.websnap_tolerance);
    let dither = r.i32(k!(b"PLDi")).unwrap_or(0);
    let lossy = r.i32(k!(b"GLTi")).unwrap_or(0);
    let jpeg_quality = r.i32(k!(b"JPQi")).unwrap_or(d.jpeg_quality);
    let jpeg_smoothness = r.i32(k!(b"JPSi")).unwrap_or(0);
    let jpeg_subsampling = r.i32(k!(b"JPOi")).unwrap_or(0);
    let jpeg_progressive = r.bool(k!(b"JPPb")).unwrap_or(false);
    let interlaced = r.bool(k!(b"GFIb")).unwrap_or(false);
    let jpeg_sel_quality = r.i32(k!(b"JSQi")).unwrap_or(d.jpeg_sel_quality);
    let jpeg_sel_enabled = r.bool(k!(b"JSEb")).unwrap_or(false);
    let jpeg_sel_preserve_text = r.bool(k!(b"JSTb")).unwrap_or(true);
    let jpeg_sel_preserve_buttons = r.bool(k!(b"JSBb")).unwrap_or(false);
    let loop_count = r.i32(k!(b"FLPi")).unwrap_or(0);
    let mut frames = Vec::new();
    let mut frames_extra = Extra::default();
    if let Some(f) = r.strict_list(k!(b"FNLv"), k!(b"FNMv")) {
        let mut s = Rd::new(f);
        for n in s.take_all(k!(b"FNMv")) {
            let mut q = Rd::new(n.children());
            let disposal = q.i32(k!(b"FNPi")).unwrap_or(0);
            let delay = q.i32(k!(b"FNDi")).unwrap_or(7);
            let visible = q.bool(k!(b"VISb")).unwrap_or(true);
            let name = q.str(k!(b"OBNs"));
            frames.push(FrameInfo { disposal, delay, visible, name, extra: ctx.finish(q) });
        }
        frames_extra = ctx.finish(s);
    }
    let crop_keys = [k!(b"LRLi"), k!(b"LRTi"), k!(b"LRRi"), k!(b"LRBi")];
    let crop = if crop_keys.iter().all(|k| r.peek_int(*k).is_some()) {
        Some([
            r.i32(crop_keys[0]).unwrap_or(0),
            r.i32(crop_keys[1]).unwrap_or(0),
            r.i32(crop_keys[2]).unwrap_or(0),
            r.i32(crop_keys[3]).unwrap_or(0),
        ])
    } else {
        None
    };
    let scale = if [k!(b"CLSb"), k!(b"CNSb")].iter().all(|k| r.has(*k))
        && [k!(b"CLPi"), k!(b"XSZi"), k!(b"YSZi")].iter().all(|k| r.peek_int(*k).is_some())
    {
        match (r.bool(k!(b"CLSb")), r.bool(k!(b"CNSb"))) {
            (Some(use_scale), Some(constrain)) => Some(ExportScale {
                use_scale,
                constrain,
                percent: r.i32(k!(b"CLPi")).unwrap_or(100),
                width: r.i32(k!(b"XSZi")).unwrap_or(0),
                height: r.i32(k!(b"YSZi")).unwrap_or(0),
            }),
            _ => None,
        }
    } else {
        None
    };
    let html = read_html(&mut r, ctx);
    ExportSettings {
        format,
        mac_type,
        mac_creator,
        color_mode,
        palette_mode,
        palette,
        palette_extra,
        palette_count,
        entries_requested,
        transparency,
        optimized,
        local_adaptive,
        websnap_adaptive,
        anim_auto_crop,
        anim_auto_difference,
        websnap_tolerance,
        dither,
        lossy,
        jpeg_quality,
        jpeg_smoothness,
        jpeg_subsampling,
        jpeg_progressive,
        interlaced,
        jpeg_sel_quality,
        jpeg_sel_enabled,
        jpeg_sel_preserve_text,
        jpeg_sel_preserve_buttons,
        loop_count,
        frames,
        frames_extra,
        crop,
        scale,
        html,
        extra: ctx.finish(r),
    }
}

impl<'a> Rd<'a> {
    /// Liste lesen, ohne sie zu verbrauchen.
    pub fn peek_list(&self, key: TagKey) -> Option<&'a [RawNode]> {
        let mut v = None;
        for n in self.all() {
            if n.key == key {
                if v.is_some() {
                    return None;
                }
                if let RawValue::List(k) = &n.value {
                    v = Some(k.as_slice());
                }
            }
        }
        v
    }
}

/// `PALv{FCLi [CLLb] [CLTb] [CLMb BCLi] …}`: nur wenn die Folge genau dem Schreibmuster entspricht.
fn read_palette(v: &[RawNode], _ctx: &mut ReadCtx) -> Option<Vec<PaletteEntry>> {
    let mut out: Vec<PaletteEntry> = Vec::new();
    for n in v {
        match (&n.key.0, &n.value) {
            (b"FCLi", RawValue::Int(c)) => out.push(PaletteEntry {
                color: *c,
                locked: None,
                transparent: None,
                modified: None,
                new_color: None,
                extra: Extra::default(),
            }),
            (b"CLLb", RawValue::Bool(b)) => out.last_mut()?.locked = Some(*b),
            (b"CLTb", RawValue::Bool(b)) => out.last_mut()?.transparent = Some(*b),
            (b"CLMb", RawValue::Bool(b)) => out.last_mut()?.modified = Some(*b),
            (b"BCLi", RawValue::Int(c)) => out.last_mut()?.new_color = Some(*c),
            _ => return None,
        }
    }
    // Nur annehmen, wenn das Zurückschreiben exakt dieselbe Folge ergibt.
    if crate::write::encode_palette(&out) != v {
        return None;
    }
    Some(out)
}

fn read_html(r: &mut Rd, ctx: &mut ReadCtx) -> Option<Box<HtmlSettings>> {
    const HTML_KEYS: [&[u8; 4]; 38] = [
        b"GSLb", b"LSUb", b"DSUb", b"LSDb", b"CSMb", b"ECSi", b"NYSb", b"LSSi", b"HTMi", b"HTTs", b"IMDs", b"XFSi", b"LXDs",
        b"XMDi", b"TRMb", b"HTMb", b"XDAb", b"EFBs", b"EFXs", b"NM4v", b"NMFv", b"NS0s", b"NS1s", b"NS2s", b"NS3s", b"ENCi",
        b"TALi", b"TPWi", b"XTMi", b"CBUs", b"CBRi", b"CBAi", b"CDAi", b"CDMv", b"CXBb", b"CXFs", b"CBDs", b"FCLi",
    ];
    if !HTML_KEYS.iter().any(|k| r.has(TagKey::new(k))) {
        return None;
    }
    let mut h = HtmlSettings {
        slice_along_guides: r.bool(k!(b"GSLb")),
        slice_using_urls: r.bool(k!(b"LSUb")),
        discard_unspecified_slices: r.bool(k!(b"DSUb")),
        generate_demo_html: r.bool(k!(b"LSDb")),
        css_popups: r.bool(k!(b"CSMb")),
        external_css: r.i32(k!(b"ECSi")),
        not_yet_set: r.bool(k!(b"NYSb")),
        shim_generation: r.i32(k!(b"LSSi")),
        html_destination: r.i32(k!(b"HTMi")),
        template_name: r.str(k!(b"HTTs")),
        images_dir: r.str(k!(b"IMDs")),
        export_files_style: r.i32(k!(b"XFSi")),
        last_export_dir: r.str(k!(b"LXDs")),
        multiple_files_mode: r.i32(k!(b"XMDi")),
        trm: r.bool(k!(b"TRMb")),
        htm: r.bool(k!(b"HTMb")),
        xda: r.bool(k!(b"XDAb")),
        base_name: r.str(k!(b"EFBs")),
        file_extension: r.str(k!(b"EFXs")),
        ..HtmlSettings::default()
    };
    if let Some(n) = r.strict_list(k!(b"NM4v"), k!(b"NM4i")) {
        let mut s = Rd::new(n);
        h.slice_naming = Some(s.take_all(k!(b"NM4i")).into_iter().map(|x| x.as_int().unwrap_or(0) as i32).collect());
        h.slice_naming_extra = ctx.finish(s);
    }
    if let Some(n) = r.strict_list(k!(b"NMFv"), k!(b"NMFi")) {
        let mut s = Rd::new(n);
        h.frame_naming = Some(s.take_all(k!(b"NMFi")).into_iter().map(|x| x.as_int().unwrap_or(0) as i32).collect());
        h.frame_naming_extra = ctx.finish(s);
    }
    let ns = [k!(b"NS0s"), k!(b"NS1s"), k!(b"NS2s"), k!(b"NS3s")];
    if ns.iter().all(|k| r.has(*k)) {
        let a = [r.str(ns[0]), r.str(ns[1]), r.str(ns[2]), r.str(ns[3])];
        if let [Some(a0), Some(a1), Some(a2), Some(a3)] = a {
            h.state_names = Some([a0, a1, a2, a3]);
        }
    }
    h.encoding = r.i32(k!(b"ENCi"));
    h.table_alignment = r.i32(k!(b"TALi"));
    h.table_pct_width = r.i32(k!(b"TPWi"));
    h.xhtml = r.i32(k!(b"XTMi"));
    h.background_url = r.str(k!(b"CBUs"));
    h.background_repeat = r.i32(k!(b"CBRi"));
    h.background_attachment = r.i32(k!(b"CBAi"));
    h.page_alignment = r.i32(k!(b"CDAi"));
    if let Some(m) = r.list(k!(b"CDMv")) {
        let mut s = Rd::new(m);
        let vals = [s.f32(LTRB[0]), s.f32(LTRB[1]), s.f32(LTRB[2]), s.f32(LTRB[3])];
        if let [Some(a), Some(b), Some(c), Some(d)] = vals {
            h.margins = Some([a, b, c, d]);
        } else {
            h.margins = Some([vals[0].unwrap_or(0.0), vals[1].unwrap_or(0.0), vals[2].unwrap_or(0.0), vals[3].unwrap_or(0.0)]);
        }
        h.margins_extra = ctx.finish(s);
    }
    h.external_css_for_doc = r.bool(k!(b"CXBb"));
    h.external_css_filename = r.str(k!(b"CXFs"));
    h.css_dir = r.str(k!(b"CBDs"));
    h.matte_color = r.int(k!(b"FCLi"));
    Some(Box::new(h))
}

/// Symbol `MSTv`.
pub fn read_symbol(v: &[RawNode], ctx: &mut ReadCtx) -> Symbol {
    let mut r = Rd::new(v);
    let name = r.str(k!(b"UNMs")).unwrap_or_default();
    let id = SymbolId(r.int(k!(b"MSNi")).unwrap_or(0));
    let kind_code = r.i32(k!(b"MSYi"));
    let origin_x = r.f32(k!(b"MOXf")).unwrap_or(0.0);
    let origin_y = r.f32(k!(b"MOYf")).unwrap_or(0.0);
    let library_file = r.str(k!(b"LFNs"));
    let library_symbol = r.int(k!(b"LMNi"));
    let modified = r.str(k!(b"LUTs"));
    let root = read_layer_root(&mut r, ctx, 1);
    let nine_slice = r.list(k!(b"NGSv")).map(|n| {
        let mut s = Rd::new(n);
        let locked = s.bool(k!(b"NGLb"));
        let color = s.int(k!(b"NGKi")).map(Argb);
        let guides: Vec<f64> = s.take_all(k!(b"NGCf")).into_iter().map(|g| g.as_num().unwrap_or(0.0)).collect();
        let (bounds, bounds_extra) = match s.list(k!(b"NGMv")) {
            Some(b) => {
                let mut q = Rd::new(b);
                let rect = q.rect4(LTRB);
                (rect, ctx.finish(q))
            }
            None => (None, Extra::default()),
        };
        let (view, view_extra) = match s.list(k!(b"NGVv")) {
            Some(b) => {
                let mut q = Rd::new(b);
                let rect = q.rect4(LTRB);
                (rect, ctx.finish(q))
            }
            None => (None, Extra::default()),
        };
        NineSlice { locked, color, guides, bounds, bounds_extra, view, view_extra, extra: ctx.finish(s) }
    });
    let widget_script = r.str(k!(b"WJSs"));
    let bo = ButtonOptions::default();
    let button_options = ButtonOptions {
        auto_slice: r.bool(k!(b"ATSb")).unwrap_or(bo.auto_slice),
        include_down: r.bool(k!(b"IDSb")).unwrap_or(false),
        show_down_on_load: r.bool(k!(b"DVLb")).unwrap_or(false),
        include_over_down: r.bool(k!(b"IODb")).unwrap_or(false),
    };
    let button = read_button_strings(&mut r);
    let status_message = r.str(k!(b"MSGs"));
    let behaviors = r.list(k!(b"BEVv")).map(|b| read_behaviors(b, ctx));
    Symbol {
        name,
        id,
        kind_code,
        origin_x,
        origin_y,
        library_file,
        library_symbol,
        modified,
        root,
        nine_slice,
        widget_script,
        button_options,
        button,
        status_message,
        behaviors,
        extra: ctx.finish(r),
    }
}

/// Stil `ZSTv`.
pub fn read_style(v: &[RawNode], ctx: &mut ReadCtx) -> Style {
    let mut r = Rd::new(v);
    let name = r.str(k!(b"ZSNs")).unwrap_or_default();
    let flags = r.int(k!(b"ZSFi")).unwrap_or(0);
    let other_flags = r.int(k!(b"ZSOi")).unwrap_or(0);
    let attrs = r.list(k!(b"PATv")).map(|p| Arc::new(read_attrs(p, ctx)));
    let hk = [k!(b"PSXf"), k!(b"PSYf"), k!(b"PEXf"), k!(b"PEYf"), k!(b"PFXf"), k!(b"PFYf")];
    let fill_handles = if hk.iter().all(|k| r.has(*k)) { Some(read_fill_handles(&mut r, false)) } else { None };
    let stroke_placement = r.i32(k!(b"BRPi"));
    let fill_on_top = r.bool(k!(b"FOTb"));
    let pk = [k!(b"FALi"), k!(b"FATi"), k!(b"FARi"), k!(b"FABi")];
    let preview_rect = if pk.iter().all(|k| r.peek_int(*k).is_some()) {
        Some([r.i32(pk[0]).unwrap_or(0), r.i32(pk[1]).unwrap_or(0), r.i32(pk[2]).unwrap_or(0), r.i32(pk[3]).unwrap_or(0)])
    } else {
        None
    };
    let effects = r.list(k!(b"EFLv")).map(|e| read_effect_list(e, ctx));
    let text = read_text_attrs(&mut r);
    let auto_kern = r.bool(k!(b"ATKb"));
    let tia = r.bool(k!(b"TIAb"));
    let antialias = r.i32(k!(b"TAAi"));
    let aa_sharpness = r.i32(k!(b"TA1i"));
    let aa_strength = r.i32(k!(b"TA2i"));
    let aa_oversample = r.i32(k!(b"TOSi"));
    let mut previews = Vec::new();
    for (i, n) in v.iter().enumerate() {
        if n.key == k!(b"IMGv") && !r.is_used(i)
            && let Some(b) = read_bitmap_node(n.children(), ctx) {
                r.mark(i);
                previews.push(Arc::new(b));
            }
    }
    Style {
        name,
        flags,
        other_flags,
        attrs,
        fill_handles,
        stroke_placement,
        fill_on_top,
        preview_rect,
        effects,
        text,
        auto_kern,
        tia,
        antialias,
        aa_sharpness,
        aa_strength,
        aa_oversample,
        previews,
        extra: ctx.finish(r),
    }
}
