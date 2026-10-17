//! Datei ▸ Importieren (fp-io): Inhalt einer Datei (FW-PNG, PSD, SVG, GIF, JPEG, PNG, TIFF, WebP …)
//! in das aktive Dokument übernehmen – als ein Verlaufsschritt, ausgewählt.
//!
//! Regeln: Elemente der ersten Seite (Status 1) des importierten Dokuments; mehrere Ebenen werden je
//! Ebene zu einer Gruppe mit dem Ebenennamen (Struktur bleibt erhalten), eine Ebene wird direkt
//! eingefügt. Symbole des Imports kommen in die Dokumentbibliothek; kollidierende Symbol-IDs werden
//! neu vergeben und die Instanzen umgehängt. Lage: Parameter `at` (linke obere Ecke des Inhalts)
//! bzw. die Lage in der Quelldatei.

use super::{DOC, insert_new, with_doc};
use crate::command::{CommandOutput, CommandRegistry, ParamKind, cmd};
use crate::docops;
use crate::error::EngineError;
use fp_model::element::{Element, ElementKind};
use fp_model::{Document, LayerKind, LayerSlot, SymbolId};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

/// Hängt Instanzen (rekursiv) auf neue Symbol-IDs um.
pub fn remap_instances(e: &mut Element, map: &HashMap<SymbolId, SymbolId>) {
    match &mut e.kind {
        ElementKind::Instance(i) => {
            if let Some(n) = map.get(&i.symbol_id) {
                i.symbol_id = *n;
            }
        }
        ElementKind::Group(g) => {
            if let Some(m) = g.mask.as_mut() {
                remap_instances(Element::edit(m), map);
            }
            for c in g.children.iter_mut() {
                remap_instances(Element::edit(c), map);
            }
        }
        ElementKind::CompoundShape(c) => {
            for o in c.operands.iter_mut() {
                remap_instances(Element::edit(o), map);
            }
        }
        ElementKind::SubLayer(l) => {
            let l = Arc::make_mut(l);
            for cel in l.cels.iter_mut() {
                for c in Arc::make_mut(cel).elements.iter_mut() {
                    remap_instances(Element::edit(c), map);
                }
            }
        }
        _ => {}
    }
}

fn remap_root(root: &mut fp_model::LayerRoot, map: &HashMap<SymbolId, SymbolId>) {
    for slot in root.layers.iter_mut() {
        if let LayerSlot::Own(l) = slot {
            let l = Arc::make_mut(l);
            for cel in l.cels.iter_mut() {
                for c in Arc::make_mut(cel).elements.iter_mut() {
                    remap_instances(Element::edit(c), map);
                }
            }
        }
    }
}

/// Übernimmt die Symbole von `src` in `dst`; liefert die Umbenennung kollidierender IDs.
pub fn merge_symbols(dst: &mut Document, src: &Document) -> HashMap<SymbolId, SymbolId> {
    let mut map = HashMap::new();
    let mut next = dst.new_symbol_id().0;
    for s in &src.symbols {
        if dst.symbol_by_id(s.id).is_some() || map.values().any(|v: &SymbolId| *v == s.id) {
            map.insert(s.id, SymbolId(next));
            next += 1;
        }
    }
    for s in &src.symbols {
        let mut sym = (**s).clone();
        if let Some(n) = map.get(&s.id) {
            sym.id = *n;
        }
        remap_root(&mut sym.root, &map);
        dst.symbols.push(Arc::new(sym));
    }
    map
}

/// Optionen des Dialogs „Vektordatei-Optionen“ (TW-43, Spezifikation re/12 §4.2), soweit Firepixel sie anwendet.
#[derive(Clone, Debug, PartialEq)]
pub struct VectorImport {
    /// Faktor auf die Größe in der Datei (Skalieren × Auflösung / 72).
    pub scale: f32,
    /// Ebenen: 0 ignorieren (alles in eine Ebene), 1 berücksichtigen (je Ebene der Datei eine neue,
    /// benannte Ebene nur im aktuellen Status, U-1/re §6), 2 in Status konvertieren (oberste Ebene in
    /// den aktuellen Status, jede weitere einen später).
    pub layers: i64,
    /// Seite bzw. Zeichenfläche der Datei (0-basiert), „Seite öffnen“.
    pub page: usize,
    /// „Seiten als Status öffnen“: jede Seite der Datei in einen eigenen Status.
    pub pages_as_states: bool,
    /// „Als Bilder wiedergeben ▸ Gruppierungen über N Objekte“: solche Gruppen werden zur Bitmap.
    pub groups_over: Option<usize>,
    /// Unsichtbare Ebenen einschließen.
    pub include_invisible: bool,
    /// Kantenglättung der Pfade.
    pub aa_paths: bool,
    /// Kantenglättung des Texts (`None` = aus), sonst Art wie `TAAi` (0 glatt, 1 scharf, 2 stark, 3 System).
    pub aa_text: Option<i32>,
    /// „Ebenen berücksichtigen“ als neue Ebenen (Vektordatei bzw. ausdrücklich gewählt); sonst wird
    /// jede Ebene der Datei zu einer benannten Gruppe in der aktuellen Ebene (z. B. FW-PNG ohne Optionen).
    /// Wird von `file.import` gesetzt, nicht aus JSON gelesen.
    pub new_layers: bool,
}

impl Default for VectorImport {
    fn default() -> Self {
        VectorImport {
            scale: 1.0,
            layers: 1,
            page: 0,
            pages_as_states: false,
            groups_over: None,
            include_invisible: true,
            aa_paths: true,
            aa_text: Some(0),
            new_layers: false,
        }
    }
}

impl VectorImport {
    /// Aus dem Parameter `vector` von `file.import`.
    pub fn from_json(v: &serde_json::Value) -> VectorImport {
        let d = VectorImport::default();
        let f = |k: &str, def: f64| v.get(k).and_then(|x| x.as_f64()).unwrap_or(def);
        let b = |k: &str, def: bool| v.get(k).and_then(|x| x.as_bool()).unwrap_or(def);
        let ppi = match v
            .get("resolutionUnit")
            .and_then(|x| x.as_i64())
            .unwrap_or(0)
        {
            1 => f("resolution", 72.0) * 2.54,
            _ => f("resolution", 72.0),
        };
        let scale = (f("scale", 100.0) / 100.0 * ppi / 72.0).clamp(0.0001, 1000.0) as f32;
        // Anti-Alias-Liste wie im Original: 0 System, 1 Scharf, 2 Stark, 3 Glatt → `TAAi`
        let kind = match v.get("antialiasKind").and_then(|x| x.as_i64()).unwrap_or(3) {
            0 => 3,
            1 => 1,
            2 => 2,
            _ => 0,
        };
        VectorImport {
            scale,
            layers: v.get("layers").and_then(|x| x.as_i64()).unwrap_or(d.layers),
            page: v.get("page").and_then(|x| x.as_u64()).unwrap_or(0) as usize,
            pages_as_states: b("pagesAsStates", false),
            groups_over: v
                .get("groupsOver")
                .and_then(|x| x.as_u64())
                .map(|n| n.max(1) as usize),
            include_invisible: b("includeInvisible", false),
            aa_paths: b("antialiasPaths", true),
            aa_text: b("antialiasText", true).then_some(kind),
            new_layers: false,
        }
    }
}

/// Kantenglättung auf importierte Pfade und Texte anwenden (auch in Gruppen).
fn apply_antialias(e: &mut Element, o: &VectorImport) {
    match &mut e.kind {
        ElementKind::Path(p) => {
            if !o.aa_paths {
                let a = Arc::make_mut(&mut p.attrs);
                if let Some(f) = a.fill.as_mut() {
                    f.edge = 0;
                }
                if let Some(b) = a.brush.as_mut() {
                    b.antialiased = false;
                }
            }
        }
        ElementKind::Text(t) => {
            if let Some(k) = o.aa_text {
                t.antialias = k;
            }
        }
        ElementKind::Group(g) => {
            for c in g.children.iter_mut() {
                apply_antialias(Arc::make_mut(c), o);
            }
        }
        _ => {}
    }
}

/// Ergebnis von [`import_frames_with`].
#[derive(Debug, Default)]
pub struct ImportedFrames {
    /// Elemente je Status (oben → unten).
    pub frames: Vec<Vec<Element>>,
    /// Gruppen, die nur für eine Ebene der Datei stehen („Ebenen berücksichtigen“); mit
    /// [`VectorImport::new_layers`] wird jede davon beim Einfügen zu einer neuen Ebene ihres Namens.
    pub layer_groups: Vec<fp_model::ElementId>,
}

/// Elemente je Status (Index 0 = aktueller Status, 1 = der nächste …): „Seiten als Status öffnen“
/// legt jede Seite, „Ebenen in Status konvertieren“ jede Ebene (oben zuerst) in einen eigenen Status
/// (TW-43, re/12 §4.2); sonst gibt es nur Status 0 mit der gewählten Seite.
///
/// ANNAHME (ungemessen, re §6): „Ebenen in Status konvertieren“ legt die oberste Ebene der Datei in
/// den aktuellen Status und jede weitere in den jeweils folgenden Status (fehlende werden angelegt).
/// Gemessen ist nur „Ebenen berücksichtigen“ (neue Ebenen nur im aktuellen Status).
pub fn import_frames_with(dst: &mut Document, src: &Document, o: &VectorImport) -> ImportedFrames {
    let map = merge_symbols(dst, src);
    let mut ids = std::mem::take(&mut dst.ids);
    let mut layer_groups = Vec::new();
    let frames: Vec<Vec<Element>> = if o.pages_as_states {
        src.pages
            .iter()
            .map(|p| {
                page_elements(p, o, &map, &mut ids, &mut layer_groups)
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
            })
            .filter(|v| !v.is_empty())
            .collect()
    } else {
        let page = src.pages.get(o.page).or_else(|| src.pages.first());
        let per_layer = page
            .map(|p| page_elements(p, o, &map, &mut ids, &mut layer_groups))
            .unwrap_or_default();
        if o.layers == 2 && per_layer.len() > 1 {
            per_layer
        } else {
            vec![per_layer.into_iter().flatten().collect()]
        }
    };
    dst.ids = ids;
    ImportedFrames {
        frames,
        layer_groups,
    }
}

/// Anzahl der Objekte in einem Element (Gruppen zählen ihre Objekte, rekursiv).
fn object_count(e: &Element) -> usize {
    match &e.kind {
        ElementKind::Group(g) => g.children.iter().map(|c| object_count(c)).sum(),
        _ => 1,
    }
}

/// „Als Bilder wiedergeben ▸ Gruppierungen über N Objekte“ (re/12 §4.2/§4.3): Gruppen aus der Datei
/// mit mehr als `n` Objekten werden im Importschritt zur Bitmap (In Bitmap konvertieren).
fn rasterize_big_groups(
    s: &mut crate::Session,
    placed: &[(usize, fp_model::ElementId)],
    n: usize,
    layer_groups: &[fp_model::ElementId],
) {
    let Ok(d) = s.doc() else { return };
    let (page, base, sel) = (d.page, d.frame, d.selection.elements.clone());
    let mut todo: Vec<(usize, fp_model::ElementId)> = Vec::new();
    for (f, id) in placed {
        let Some(e) = docops::element_on_page(&d.doc, page, *id) else {
            continue;
        };
        let cands: Vec<Arc<Element>> = match &e.kind {
            ElementKind::Group(g) if layer_groups.contains(id) => g.children.to_vec(),
            _ => vec![Arc::clone(&e)],
        };
        todo.extend(
            cands
                .iter()
                .filter(|c| matches!(c.kind, ElementKind::Group(_)) && object_count(c) > n)
                .map(|c| (*f, c.id)),
        );
    }
    let mut sel = sel;
    for (f, id) in todo {
        if let Ok(d) = s.doc_mut() {
            d.frame = f;
            d.selection.set(vec![id]);
        }
        if let Ok(o) = s.apply_inline("modify.flattenSelection", json!({}))
            && let Some(nid) = o.value.get("id").and_then(|v| v.as_u64())
            && let Some(i) = sel.iter().position(|x| *x == id)
        {
            sel[i] = fp_model::ElementId(nid);
        }
    }
    if let Ok(d) = s.doc_mut() {
        d.frame = base;
        d.selection.set(sel);
    }
}

/// Elemente einer Seite je Ebene (oben → unten); mit „Ebenen berücksichtigen“ und mehreren Ebenen
/// je Ebene eine Gruppe mit dem Ebenennamen (ihre IDs in `groups`).
fn page_elements(
    page: &fp_model::Page,
    o: &VectorImport,
    map: &HashMap<SymbolId, SymbolId>,
    ids: &mut fp_model::IdGen,
    groups: &mut Vec<fp_model::ElementId>,
) -> Vec<Vec<Element>> {
    // Ebenen oben → unten (Liste ist unten → oben)
    let layers: Vec<_> = page
        .root
        .layers
        .iter()
        .rev()
        .filter_map(|s| match s {
            LayerSlot::Own(l) if l.kind != LayerKind::Web => Some(Arc::clone(l)),
            _ => None,
        })
        .filter(|l| l.cels.first().is_some_and(|c| !c.elements.is_empty()))
        .filter(|l| o.include_invisible || l.cels.first().is_none_or(|c| c.visible))
        .collect();
    let mut out = Vec::new();
    for l in &layers {
        let Some(cel) = l.cels.first() else { continue };
        let mut els: Vec<Arc<Element>> = Vec::new();
        for e in &cel.elements {
            let mut d = e.duplicate(ids);
            remap_instances(&mut d, map);
            els.push(Arc::new(d));
        }
        if layers.len() == 1 || o.layers != 1 {
            out.push(els.into_iter().map(|e| (*e).clone()).collect());
        } else {
            let mut g = Element::new_group(ids, els);
            g.common.name = Some(l.name.clone());
            groups.push(g.id);
            out.push(vec![g]);
        }
    }
    out
}

/// Rundung halb weg von null (re8 §1, wie `f32::round`).
fn round_away(v: f64) -> f32 {
    v.round() as f32
}

/// Rechteck des Platzierungszeigers (re8 §3): seitentreu zu `src`, vom Druckpunkt `start` zum Zeiger
/// `p`. `cover` = die Achse mit dem größeren Verhältnis Zug/Quelle führt (das Rechteck umschließt den
/// Zeiger, Werkzeug), sonst die mit dem kleineren (einpassen, `maintainAspectRatio`). Bei Gleichstand
/// führt y. Jede Koordinate einzeln halb weg von null gerundet.
pub fn place_rect(
    start: fp_geom::Point,
    p: fp_geom::Point,
    src: &fp_geom::Rect,
    cover: bool,
) -> fp_geom::Rect {
    let (sx, sy) = (start.x as f64, start.y as f64);
    let (dx, dy) = (p.x as f64 - sx, p.y as f64 - sy);
    let (qw, qh) = ((src.width() as f64).abs(), (src.height() as f64).abs());
    let (mut ex, mut ey) = (p.x as f64, p.y as f64);
    if qw > 0.0 && qh > 0.0 {
        let (rx, ry) = (dx.abs() / qw, dy.abs() / qh);
        let x_leads = if cover { ry < rx } else { rx < ry };
        if x_leads {
            ey = sy + dy.signum() * qh * rx;
        } else {
            ex = sx + dx.signum() * qw * ry;
        }
    }
    fp_geom::Rect::new(
        round_away(sx.min(ex)),
        round_away(sy.min(ey)),
        round_away(sx.max(ex)),
        round_away(sy.max(ey)),
    )
}

/// Sichtbare Grenzen einer Importdatei vor dem Import (K-9, re 38 Modus 1): dieselbe Vereinigung
/// der Elementgrenzen wie `file.import` (Bitmaps: sichtbare Pixel), in Dateikoordinaten. Die
/// Vektor-Option `scale` bleibt unberücksichtigt, weil der Platzierungszeiger nur das
/// Seitenverhältnis braucht. `None`, wenn die Datei nicht lesbar ist oder nichts enthält.
pub fn import_visible_bounds(
    path: &std::path::Path,
    vector: Option<&serde_json::Value>,
) -> Option<fp_geom::Rect> {
    let data = std::fs::read(path).ok()?;
    let res = fp_io::import_bytes(
        &path.to_string_lossy(),
        &data,
        &fp_io::ImportOptions::with_installation(),
    )
    .ok()?;
    let vo = vector.map(VectorImport::from_json).unwrap_or_default();
    let mut doc = Document::new(1.0, 1.0, fp_color::Argb(0));
    let frames = import_frames_with(&mut doc, &res.document, &vo).frames;
    frames
        .iter()
        .flatten()
        .filter_map(|e| docops::element_bounds(&doc, e))
        .reduce(|a, b| a.union(&b))
}

/// Vorschaurechteck des Platzierungszeigers beim Aufziehen (K-9): mit bekannten sichtbaren
/// Grenzen `visible` das Rechteck, das `file.import {placeFrom, placeTo}` belegt ([`place_rect`],
/// Abdecken); sonst das rohe Zugrechteck.
pub fn place_preview_rect(
    start: fp_geom::Point,
    p: fp_geom::Point,
    visible: Option<&fp_geom::Rect>,
) -> fp_geom::Rect {
    match visible.filter(|v| v.width().abs() > 0.0 && v.height().abs() > 0.0) {
        Some(v) => {
            // F27: wie `file.import` mit inklusivem Zugende
            let (start, p) = inclusive_drag(start, p);
            place_rect(start, p, v, true)
        }
        None => fp_geom::Rect::new(
            start.x.min(p.x),
            start.y.min(p.y),
            start.x.max(p.x),
            start.y.max(p.y),
        ),
    }
}

/// F27 (TW-101, re 41 §2.3): Das Zugende zählt inklusiv – das Rechteck reicht bis zur Pixelkante
/// hinter dem End-Bildpunkt (+1 je Achse, bei Zug nach links/oben am Druckpunkt).
pub fn inclusive_drag(f: fp_geom::Point, t: fp_geom::Point) -> (fp_geom::Point, fp_geom::Point) {
    let ax = |f: f32, t: f32| if t >= f { (f, t + 1.0) } else { (f + 1.0, t) };
    let (fx, tx) = ax(f.x, t.x);
    let (fy, ty) = ax(f.y, t.y);
    (fp_geom::Point::new(fx, fy), fp_geom::Point::new(tx, ty))
}

/// Abgebildetes Rechteck `b` unter `m` (Skalierung + Versatz), jede Kante abgerundet.
fn floor_rect(m: &fp_geom::Matrix3, b: &fp_geom::Rect) -> fp_geom::Rect {
    let (fx, fy, tx, ty) = (m.m[0][0], m.m[1][1], m.m[2][0], m.m[2][1]);
    let f = |v: f32| (v + 1e-3).floor();
    let (l, r) = (f(fx * b.left + tx), f(fx * b.right + tx));
    let (t, bt) = (f(fy * b.top + ty), f(fy * b.bottom + ty));
    fp_geom::Rect::new(l.min(r), t.min(bt), l.max(r), t.max(bt))
}

/// Matrix, die `b` mit getrennten Faktoren auf `r` abbildet (Faktor 1,0, wenn eine Seite < 1 px).
pub fn map_bounds(b: &fp_geom::Rect, r: &fp_geom::Rect) -> fp_geom::Matrix3 {
    let f = |dst: f32, src: f32| {
        if dst < 1.0 || src < 1.0 {
            1.0
        } else {
            dst / src
        }
    };
    let (fx, fy) = (f(r.width(), b.width()), f(r.height(), b.height()));
    fp_geom::Matrix3 {
        m: [
            [fx, 0.0, 0.0],
            [0.0, fy, 0.0],
            [r.left - fx * b.left, r.top - fy * b.top, 1.0],
        ],
    }
}

pub(super) fn register(r: &mut CommandRegistry) {
    r.add(
        cmd("file.import")
            .requires(DOC)
            .param("path", ParamKind::Path, "Datei (FW-PNG, PSD, SVG, AI/EPS, GIF, JPEG, PNG, BMP, TIFF, TGA, WebP, JXL, QOI, ICO …)")
            .opt("at", ParamKind::Point, "linke obere Ecke des importierten Inhalts (Vorgabe: Lage in der Datei)")
            .opt("rect", ParamKind::Rect, "Zielrechteck: die vollen Grenzen des Imports mit getrennten Faktoren x/y darauf abbilden")
            .opt("maintainAspectRatio", ParamKind::Bool, "mit rect: seitentreu in das Rechteck einpassen")
            .opt("placeFrom", ParamKind::Point, "Platzierungszeiger: Druckpunkt (allein = Klick, Originalgröße an diesem Punkt; Auswahl bleibt leer)")
            .opt("placeTo", ParamKind::Point, "Platzierungszeiger: Endpunkt des Zugs (Rechteck seitentreu zu den sichtbaren Pixeln, umschließt den Zeiger)")
            .opt("vector", ParamKind::Json, "Vektordatei-Optionen (AI/EPS/PDF): scale (%), resolution, resolutionUnit (0 Pixel/Zoll, 1 Pixel/cm), antialiasPaths, antialiasText, antialiasKind (0 System, 1 Scharf, 2 Stark, 3 Glatt), layers (0 ignorieren, 1 berücksichtigen, 2 in Status konvertieren), includeInvisible, page (Seite/Zeichenfläche, 0-basiert), pagesAsStates, groupsOver (Gruppen mit mehr als N Objekten als Bitmap)")
            .js("importFile")
            .run(|s, p| {
                let path = std::path::PathBuf::from(p.str("path")?);
                let data = std::fs::read(&path)?;
                let res = fp_io::import_bytes(&path.to_string_lossy(), &data, &fp_io::ImportOptions::with_installation()).map_err(|e| EngineError::Format(e.to_string()))?;
                let at = p.opt_point("at")?;
                let rect = p.opt_rect("rect")?.filter(|r| r.width() >= 1.0 && r.height() >= 1.0);
                let keep_aspect = p.bool_or("maintainAspectRatio", false)?;
                let place_from = p.opt_point("placeFrom")?;
                let place_to = p.opt_point("placeTo")?;
                let vector: Option<serde_json::Value> = p.parse("vector")?;
                // „Ebenen berücksichtigen“ legt bei Vektordateien (auch im Skript-Import ohne Optionen,
                // der wie im Original keinen Dialog zeigt, re §6) neue Ebenen an
                let is_vector = matches!(res.format, fp_io::FileFormat::Illustrator | fp_io::FileFormat::Pdf | fp_io::FileFormat::Eps | fp_io::FileFormat::EpsBinary | fp_io::FileFormat::Svg);
                let vopts = vector.as_ref().map(VectorImport::from_json);
                let mut vo = vopts.clone().unwrap_or_default();
                vo.new_layers = vo.layers == 1 && (vopts.is_some() || is_vector);
                let sampler = s.prefs.edit_sampler();
                let groups_over = vopts.as_ref().and_then(|o| o.groups_over);
                let out = with_doc(s, |d| {
                    let doc = Arc::make_mut(&mut d.doc);
                    let imported = import_frames_with(doc, &res.document, &vo);
                    let (frames, layer_groups) = (imported.frames, imported.layer_groups);
                    // alle Status gemeinsam platzieren (eine Lage für den ganzen Inhalt), danach je Status einfügen
                    let mut frame_of: Vec<usize> = Vec::new();
                    let mut els: Vec<Element> = Vec::new();
                    for (k, f) in frames.into_iter().enumerate() {
                        frame_of.extend(std::iter::repeat_n(k, f.len()));
                        els.extend(f);
                    }
                    if els.is_empty() {
                        return Err(EngineError::Failed(fp_i18n::t("err-file-no-objects")));
                    }
                    // Vektordatei-Optionen (TW-43): Größe und Kantenglättung vor dem Platzieren
                    if let Some(o) = &vopts {
                        if (o.scale - 1.0).abs() > 1e-6 {
                            let k = o.scale;
                            let m = fp_geom::Matrix3 { m: [[k, 0.0, 0.0], [0.0, k, 0.0], [0.0, 0.0, 1.0]] };
                            for e in els.iter_mut() {
                                docops::transform_element(e, &m, sampler);
                            }
                        }
                        for e in els.iter_mut() {
                            apply_antialias(e, o);
                        }
                    }
                    // Grenzen des Imports: `full` = ganze Elementgrenzen (Einzelbild: die ganze Bitmap mit
                    // transparentem Rand), `visible` = Vereinigung der Elementgrenzen (sichtbare Pixel)
                    let visible = els.iter().filter_map(|e| docops::element_bounds(doc, e)).reduce(|a, b| a.union(&b));
                    let full = if res.content == fp_io::ImportedContent::Bitmap {
                        res.document.pages.first().map(|p| fp_geom::Rect::new(0.0, 0.0, p.canvas.width, p.canvas.height))
                    } else {
                        visible
                    };
                    // Platzierungszeiger (K-7, re8): Zug von `placeFrom` nach `placeTo`, Rechteck seitentreu
                    // zu den sichtbaren Pixeln (Abdecken), Klick (ohne `placeTo`) = Originalgröße am Klickpunkt
                    let rect = match place_from {
                        Some(f) => match (place_to, visible) {
                            (Some(t), Some(v)) => {
                                let (f, t) = inclusive_drag(f, t);
                                Some(place_rect(f, t, &v, true))
                            }
                            _ => full.map(|b| fp_geom::Rect::new(f.x.round(), f.y.round(), f.x.round() + b.width(), f.y.round() + b.height())),
                        },
                        None => match (rect, full) {
                            (Some(r), Some(b)) if keep_aspect => Some(place_rect(fp_geom::Point::new(r.left, r.top), fp_geom::Point::new(r.right, r.bottom), &b, false)),
                            (r, _) => r,
                        },
                    };
                    if let Some(r) = rect {
                        // re8 §3: die vollen Grenzen mit getrennten Faktoren x/y auf das Rechteck abbilden
                        // (Bitmaps einmal neu berechnet, Vektoren: Matrix auf die Pfade)
                        if let Some(b) = full {
                            let m = map_bounds(&b, &r);
                            let dragged = place_from.is_some() && place_to.is_some();
                            for e in els.iter_mut() {
                                // F27 (TW-101): beim Aufziehen liegen die Kanten einer Bitmap auf den
                                // abgerundeten Bildpunkten der abgebildeten Kanten (Original 121,120–417,322)
                                let m = match (&e.kind, dragged.then(|| docops::element_bounds(doc, e)).flatten()) {
                                    (ElementKind::Bitmap(_), Some(eb)) => map_bounds(&eb, &floor_rect(&m, &eb)),
                                    _ => m,
                                };
                                docops::transform_element(e, &m, sampler);
                            }
                        }
                    } else if let Some(at) = at {
                        // Einzelbild: `at` ist die linke obere Ecke des Bildes; das auf die sichtbaren
                        // Pixel beschnittene Objekt behält seinen Versatz darin (re\22 §1.2 Regel 1)
                        let b = if res.content == fp_io::ImportedContent::Bitmap {
                            Some(fp_geom::Rect::new(0.0, 0.0, 0.0, 0.0))
                        } else {
                            els.iter().filter_map(|e| docops::element_bounds(doc, e)).reduce(|a, b| a.union(&b))
                        };
                        if let Some(b) = b {
                            let (dx, dy) = (at.x - b.left, at.y - b.top);
                            for e in els.iter_mut() {
                                docops::translate_element(e, dx, dy);
                            }
                        }
                    }
                    // weitere Status (Seiten/Ebenen als Status): hinter dem aktuellen, fehlende anhängen
                    let extra = frame_of.iter().copied().max().unwrap_or(0);
                    let (page, base) = (d.page, d.frame);
                    let need = (base + extra + 1).saturating_sub(d.frame_count());
                    if need > 0 {
                        let n = d.frame_count();
                        docops::insert_frames(Arc::make_mut(&mut d.doc), page, n, need).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?;
                    }
                    let mut new_ids = Vec::new();
                    let mut placed: Vec<(usize, fp_model::ElementId)> = Vec::new();
                    // unterstes zuerst einfügen, damit die Reihenfolge erhalten bleibt
                    for (e, k) in els.into_iter().zip(frame_of).rev() {
                        if k == 0 && vo.new_layers && layer_groups.contains(&e.id) {
                            // „Ebenen berücksichtigen“ (re §6): je Ebene der Datei eine neue Ebene mit
                            // ihrem Namen über der aktiven, Inhalt nur im aktuellen Status
                            let ElementKind::Group(g) = &e.kind else { continue };
                            let name = e.common.name.clone().unwrap_or_default();
                            super::edit::add_layer(d, Some(&name), LayerKind::Normal)?;
                            for c in g.children.iter().rev() {
                                let id = insert_new(d, (**c).clone())?;
                                new_ids.push(id);
                                placed.push((base, id));
                            }
                        } else if k == 0 {
                            let id = insert_new(d, e)?;
                            new_ids.push(id);
                            placed.push((base, id));
                        } else {
                            let id = e.id;
                            let layer = d.layer;
                            docops::insert_element(Arc::make_mut(&mut d.doc), page, base + k, layer, 0, Arc::new(e)).ok_or_else(|| EngineError::Failed(fp_i18n::t("err-element-insert-failed")))?;
                            placed.push((base + k, id));
                        }
                    }
                    new_ids.reverse();
                    d.frame = base;
                    // Platzierungszeiger: danach ist nichts ausgewählt (re8 §2.4)
                    d.selection.set(if place_from.is_some() { Vec::new() } else { new_ids.clone() });
                    Ok((CommandOutput::doc_with(json!({"ids": super::ids_json(&new_ids), "format": format!("{:?}", res.format), "warnings": res.warnings})).labeled(fp_i18n::t("msg-step-import")), placed, layer_groups))
                });
                let (out, placed, layer_groups) = out?;
                // „Als Bilder wiedergeben ▸ Gruppierungen über N Objekte“: solche Gruppen im selben
                // Schritt in Bitmaps umwandeln (ohne Renderer bleiben sie Vektoren)
                if let Some(n) = groups_over {
                    rasterize_big_groups(s, &placed, n, &layer_groups);
                }
                Ok(out)
            }),
    );
}
