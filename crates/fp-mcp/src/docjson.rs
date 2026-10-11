//! Dokumentbaum als JSON (MCP `get_document`, CLI `info --json`).
//!
//! Aufbau (Ebenen in Dateireihenfolge **unten → oben** wie `dom.layers` im Original; die
//! Web-Ebene ist die letzte; Elemente einer Ebene **oben → unten**):
//!
//! ```json
//! { "doc": 1, "title": "x.fw.png", "path": "C:\\…", "dirty": false,
//!   "currentPage": 0, "currentState": 0, "activeLayer": 3, "selection": [12],
//!   "pages": [ { "index": 0, "name": "Seite 1", "width": 400, "height": 300,
//!     "background": "#ffffffff", "resolution": 72, "states": 1, "isMaster": false,
//!     "layers": [ { "id": 3, "name": "Ebene 1", "kind": "normal", "visible": true,
//!       "locked": false, "sharedAcrossStates": false, "origin": "own",
//!       "elements": [ { "id": 12, "type": "rectangle", "jsType": "RectanglePrimitive",
//!         "name": null, "visible": true, "locked": false, "opacity": 100, "blendMode": "normal",
//!         "bounds": {"left": 10, "top": 10, "right": 60, "bottom": 40, "width": 50, "height": 30},
//!         "fill": "#ff0000ff", "stroke": null, "selected": true } ] } ] } ] }
//! ```

use fp_engine::{DocumentState, Session};
use fp_model::element::ElementKind;
use fp_model::{Argb, Document, Element, LayerKind, LayerOrigin};
use serde_json::{json, Map, Value};
use std::sync::Arc;

/// Optionen für [`document_json`].
#[derive(Clone, Debug)]
pub struct DocJsonOptions {
    /// Nur diese Seite ausführlich (Vorgabe: aktuelle Seite); andere Seiten als Kurzfassung.
    pub page: Option<usize>,
    /// Alle Seiten ausführlich.
    pub all_pages: bool,
    /// Status, dessen Inhalt gezeigt wird (Vorgabe: aktueller Status).
    pub state: Option<usize>,
    /// Verschachtelungstiefe: 0 = nur Ebenen, 1 = Elemente direkt in der Ebene, 2 = auch
    /// Gruppenkinder usw.
    pub depth: usize,
    /// Höchstzahl ausgegebener Elemente (danach `"truncated": true`).
    pub max_elements: usize,
    /// Nur diese Typen (`path`, `rectangle`, `text`, `bitmap`, `group`, `instance`, …); leer = alle.
    pub types: Vec<String>,
    /// Nur Elemente, deren Name diesen Text enthält (Groß-/Kleinschreibung egal).
    pub name_contains: Option<String>,
    /// Nur ausgewählte Elemente.
    pub selected_only: bool,
    /// Attribute (Füllung, Strich, Text, Effekte, Symbol …) ausgeben.
    pub attributes: bool,
    /// customData ausgeben (als Quelltext wie `toSource`).
    pub custom_data: bool,
}

impl Default for DocJsonOptions {
    fn default() -> Self {
        DocJsonOptions {
            page: None,
            all_pages: false,
            state: None,
            depth: 8,
            max_elements: 500,
            types: Vec::new(),
            name_contains: None,
            selected_only: false,
            attributes: true,
            custom_data: false,
        }
    }
}

/// Farbe als `#rrggbbaa`.
pub fn color_hex(c: Argb) -> String {
    format!("#{:02x}{:02x}{:02x}{:02x}", c.r(), c.g(), c.b(), c.a())
}

fn r2(v: f32) -> f64 {
    (v as f64 * 100.0).round() / 100.0
}

/// Kurzer Typname eines Elements (stabil, für Agenten).
pub fn element_type(e: &Element) -> &'static str {
    match &e.kind {
        ElementKind::Bitmap(_) => "bitmap",
        ElementKind::Path(p) if p.primitive.is_some() => "rectangle",
        ElementKind::Path(_) => "path",
        ElementKind::Text(_) => "text",
        ElementKind::Group(g) if g.smart_shape.is_some() => "autoShape",
        ElementKind::Group(g) if g.kind.is_mask() => "maskGroup",
        ElementKind::Group(_) => "group",
        ElementKind::CompoundShape(_) => "compoundShape",
        ElementKind::Instance(_) => "instance",
        ElementKind::WebObject(w) if w.slice.is_some() => "slice",
        ElementKind::WebObject(_) => "hotspot",
        ElementKind::SubLayer(_) => "subLayer",
        ElementKind::Raw(_) => "unknown",
    }
}

struct Ctx<'a> {
    doc: &'a Document,
    opts: &'a DocJsonOptions,
    selection: &'a [fp_model::ElementId],
    count: usize,
    truncated: bool,
}

/// Dokument des Sitzungszustands als JSON.
pub fn document_json(d: &DocumentState, opts: &DocJsonOptions) -> Value {
    let doc = &d.doc;
    let detail_page = opts.page.unwrap_or(d.page);
    let state = opts.state.unwrap_or(d.frame);
    let mut ctx = Ctx { doc, opts, selection: &d.selection.elements, count: 0, truncated: false };
    let mut pages = Vec::new();
    for (i, p) in doc.pages.iter().enumerate() {
        let mut o = Map::new();
        o.insert("index".into(), json!(i));
        o.insert("name".into(), json!(p.display_name()));
        o.insert("width".into(), json!(r2(p.canvas.width)));
        o.insert("height".into(), json!(r2(p.canvas.height)));
        if p.canvas.origin.x != 0.0 || p.canvas.origin.y != 0.0 {
            o.insert("origin".into(), json!({"x": r2(p.canvas.origin.x), "y": r2(p.canvas.origin.y)}));
        }
        o.insert("background".into(), json!(color_hex(p.root.background)));
        o.insert("resolution".into(), json!(r2(p.canvas.resolution)));
        o.insert("states".into(), json!(p.root.frame_count.max(1)));
        o.insert("isMaster".into(), json!(p.master()));
        if opts.all_pages || i == detail_page {
            o.insert("layers".into(), Value::Array(layers_json(&mut ctx, i, state)));
        } else {
            o.insert("layerCount".into(), json!(p.root.layers.len()));
        }
        pages.push(Value::Object(o));
    }
    let mut out = json!({
        "doc": d.id.0,
        "title": d.title,
        "path": d.path.as_ref().map(|p| p.to_string_lossy().to_string()),
        "dirty": d.is_dirty(),
        "generation": d.generation,
        "currentPage": d.page,
        "currentState": d.frame,
        "shownState": state,
        "activeLayer": d.layer.map(|l| l.0),
        "selection": d.selection.elements.iter().map(|e| e.0).collect::<Vec<_>>(),
        "pixelSelection": d.selection.pixels.as_ref().map(|m| {
            let b = m.tight_bounds();
            json!({"left": b.left, "top": b.top, "right": b.right, "bottom": b.bottom})
        }),
        "bitmapMode": d.bitmap_mode,
        "history": {"undo": d.history.undo_count(), "redo": d.history.redo_count()},
        "symbols": doc.symbols.len(),
        "pages": pages,
    });
    if ctx.truncated {
        out["truncated"] = json!(true);
        out["note"] = json!(format!("nur die ersten {} Elemente; mit depth/types/name/page einschränken oder maxElements erhöhen", opts.max_elements));
    }
    out
}

/// Kurzliste der offenen Dokumente.
pub fn documents_json(s: &Session) -> Value {
    let active = s.active_id();
    Value::Array(
        s.documents()
            .iter()
            .map(|d| {
                let (w, h) = d.doc.pages.get(d.page).map(|p| (r2(p.canvas.width), r2(p.canvas.height))).unwrap_or((0.0, 0.0));
                json!({"doc": d.id.0, "title": d.title, "path": d.path.as_ref().map(|p| p.to_string_lossy().to_string()),
                       "active": Some(d.id) == active, "dirty": d.is_dirty(), "pages": d.doc.pages.len(), "width": w, "height": h})
            })
            .collect(),
    )
}

fn layers_json(ctx: &mut Ctx, page: usize, state: usize) -> Vec<Value> {
    let doc = ctx.doc;
    let mut out = Vec::new();
    for rl in doc.resolved_layers(page) {
        let l = &rl.layer;
        let origin = match rl.origin {
            LayerOrigin::Own => "own",
            LayerOrigin::Shared => "sharedAcrossPages",
            LayerOrigin::MasterPage => "masterPage",
        };
        let cel = l.cel(state);
        let mut o = Map::new();
        o.insert("id".into(), json!(l.id.0));
        o.insert("name".into(), json!(l.name));
        o.insert("kind".into(), json!(if l.kind == LayerKind::Web { "web" } else { "normal" }));
        o.insert("visible".into(), json!(l.is_visible()));
        o.insert("locked".into(), json!(cel.is_some_and(|c| c.locked)));
        o.insert("sharedAcrossStates".into(), json!(l.shared_across_frames));
        o.insert("level".into(), json!(l.level));
        o.insert("origin".into(), json!(origin));
        if ctx.opts.depth > 0 {
            let els: Vec<Value> = cel.map(|c| elements_json(ctx, &c.elements, 1)).unwrap_or_default();
            o.insert("elements".into(), Value::Array(els));
        } else {
            o.insert("elementCount".into(), json!(cel.map(|c| c.elements.len()).unwrap_or(0)));
        }
        out.push(Value::Object(o));
    }
    out
}

fn matches(ctx: &Ctx, e: &Element) -> bool {
    let o = ctx.opts;
    if !o.types.is_empty() && !o.types.iter().any(|t| t.eq_ignore_ascii_case(element_type(e)) || t.eq_ignore_ascii_case(e.js_type())) {
        return false;
    }
    if let Some(n) = &o.name_contains {
        let name = e.common.name.as_deref().unwrap_or("");
        if !name.to_lowercase().contains(&n.to_lowercase()) {
            return false;
        }
    }
    if o.selected_only && !ctx.selection.contains(&e.id) {
        return false;
    }
    true
}

fn elements_json(ctx: &mut Ctx, list: &[Arc<Element>], level: usize) -> Vec<Value> {
    let filtering = !ctx.opts.types.is_empty() || ctx.opts.name_contains.is_some() || ctx.opts.selected_only;
    let mut out = Vec::new();
    for e in list {
        if ctx.count >= ctx.opts.max_elements {
            ctx.truncated = true;
            break;
        }
        if matches(ctx, e) {
            ctx.count += 1;
            out.push(element_json(ctx, e, level));
        } else if filtering && level < ctx.opts.depth {
            // gefilterte Suche steigt in Container ab und hebt Treffer nach oben
            out.extend(elements_json(ctx, e.children(), level + 1));
        }
    }
    out
}

/// Ein Element als JSON (ohne Rekursionsgrenze der Optionen: `level` = aktuelle Tiefe).
fn element_json(ctx: &mut Ctx, e: &Arc<Element>, level: usize) -> Value {
    let doc = ctx.doc;
    let mut o = Map::new();
    o.insert("id".into(), json!(e.id.0));
    o.insert("type".into(), json!(element_type(e)));
    o.insert("jsType".into(), json!(e.js_type()));
    o.insert("name".into(), json!(e.common.name));
    o.insert("visible".into(), json!(e.common.visible));
    o.insert("locked".into(), json!(e.common.locked));
    o.insert("opacity".into(), json!((e.common.opacity.to_percent() * 10.0).round() / 10.0));
    o.insert("blendMode".into(), json!(e.common.blend.name().unwrap_or("normal")));
    if let Some(b) = fp_engine::docops::element_bounds(doc, e) {
        o.insert(
            "bounds".into(),
            json!({"left": r2(b.left), "top": r2(b.top), "right": r2(b.right), "bottom": r2(b.bottom), "width": r2(b.right - b.left), "height": r2(b.bottom - b.top)}),
        );
    }
    if ctx.selection.contains(&e.id) {
        o.insert("selected".into(), json!(true));
    }
    if ctx.opts.attributes {
        attributes(doc, e, &mut o);
    }
    if ctx.opts.custom_data && !e.common.custom_data.is_empty() {
        o.insert("customData".into(), Value::Object(e.common.custom_data.entries.iter().map(|en| (en.key.clone(), match &en.value { fp_model::DataValue::Dict(_) | fp_model::DataValue::List(_) | fp_model::DataValue::Points(_) => json!(en.value.to_source()), _ => json!(en.text_value()) })).collect()));
    }
    let filtering = !ctx.opts.types.is_empty() || ctx.opts.name_contains.is_some() || ctx.opts.selected_only;
    match &e.kind {
        ElementKind::Group(g) => {
            if level < ctx.opts.depth && !filtering {
                if let Some(m) = &g.mask {
                    let mj = element_json(ctx, m, level + 1);
                    o.insert("mask".into(), mj);
                }
                let kids = elements_json(ctx, &g.children, level + 1);
                o.insert("children".into(), Value::Array(kids));
            } else {
                o.insert("childCount".into(), json!(g.children.len()));
            }
        }
        ElementKind::CompoundShape(c) => {
            if level < ctx.opts.depth && !filtering {
                let kids = elements_json(ctx, &c.operands, level + 1);
                o.insert("children".into(), Value::Array(kids));
            } else {
                o.insert("childCount".into(), json!(c.operands.len()));
            }
        }
        _ => {}
    }
    Value::Object(o)
}

fn attributes(doc: &Document, e: &Element, o: &mut Map<String, Value>) {
    let pa = fp_engine::docops::path_attrs_of(e);
    if let Some(a) = &pa {
        let fill = match &a.fill {
            None => Value::Null,
            Some(f) if f.gradient.is_some() => {
                let stops: Vec<Value> = f
                    .gradient
                    .as_ref()
                    .map(|g| g.color_stops().iter().map(|s| json!({"position": r2(s.position), "color": color_hex(s.color)})).collect())
                    .unwrap_or_default();
                json!({"type": "gradient", "name": f.name, "stops": stops})
            }
            Some(f) if f.pattern.is_some() => json!({"type": "pattern", "name": f.name}),
            Some(_) => json!(color_hex(a.fill_color)),
        };
        o.insert("fill".into(), fill);
        let stroke = match &a.brush {
            None => Value::Null,
            Some(b) => json!({"color": color_hex(a.stroke_color), "width": b.diameter, "name": b.name}),
        };
        o.insert("stroke".into(), stroke);
    }
    match &e.kind {
        ElementKind::Path(p) => {
            o.insert("contours".into(), json!(p.contours.len()));
            o.insert("closed".into(), json!(p.contours.iter().any(|c| c.closed)));
            if let Some(pr) = &p.primitive {
                o.insert("roundness".into(), json!(r2(pr.roundness)));
            }
        }
        ElementKind::Text(t) => {
            let mut text = t.plain_text();
            if text.chars().count() > 500 {
                text = text.chars().take(500).collect::<String>() + "…";
            }
            o.insert("text".into(), json!(text));
            let first = t.resolved_runs().first().map(|(a, _)| a.clone());
            if let Some(a) = first {
                o.insert("font".into(), json!(a.font));
                o.insert("fontSize".into(), json!(a.size.map(r2)));
                if let Some(c) = a.color {
                    o.insert("color".into(), json!(color_hex(Argb(c))));
                }
            }
        }
        ElementKind::Bitmap(b) => {
            o.insert("pixelWidth".into(), json!(b.image.grid.width));
            o.insert("pixelHeight".into(), json!(b.image.grid.height));
        }
        ElementKind::Instance(i) => {
            if let Some((_, s)) = doc.symbol_by_id(i.symbol_id) {
                o.insert("symbol".into(), json!({"id": i.symbol_id.0, "name": s.name, "kind": s.kind().js_name()}));
            } else {
                o.insert("symbol".into(), json!({"id": i.symbol_id.0, "missing": true}));
            }
        }
        ElementKind::WebObject(w) => {
            o.insert("url".into(), json!(w.url));
            o.insert("alt".into(), json!(w.alt_text));
            o.insert("target".into(), json!(w.target));
        }
        _ => {}
    }
    if let Some(fx) = &e.common.effects {
        let list: Vec<Value> = fx
            .effects
            .iter()
            .map(|f| json!({"name": f.ui_name(), "kind": format!("{:?}", f.kind()), "moaId": f.moa_id(), "visible": f.is_visible()}))
            .collect();
        if !list.is_empty() {
            o.insert("effects".into(), Value::Array(list));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_contains_new_rectangle() {
        let mut s = Session::default();
        assert!(s.execute("file.new", json!({"width": 100, "height": 80})).is_ok());
        assert!(s.execute("draw.rectangle", json!({"rect": [10, 10, 50, 40]})).is_ok());
        let Some(d) = s.active() else { panic!("kein Dokument") };
        let v = document_json(d, &DocJsonOptions::default());
        let layers = v["pages"][0]["layers"].as_array().cloned().unwrap_or_default();
        let els: Vec<Value> = layers.iter().flat_map(|l| l["elements"].as_array().cloned().unwrap_or_default()).collect();
        assert_eq!(els.len(), 1);
        assert_eq!(els[0]["type"], "rectangle");
        assert_eq!(els[0]["bounds"]["width"], 40.0);
        assert_eq!(els[0]["selected"], true);
        let only_text = document_json(d, &DocJsonOptions { types: vec!["text".into()], ..DocJsonOptions::default() });
        let n: usize = only_text["pages"][0]["layers"].as_array().map(|ls| ls.iter().map(|l| l["elements"].as_array().map_or(0, |a| a.len())).sum()).unwrap_or(0);
        assert_eq!(n, 0);
    }
}
