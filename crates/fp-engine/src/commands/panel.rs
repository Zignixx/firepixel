//! Bedienfeld- und Inspektorbefehle (Präfix `pi.`) sowie die Befehle der Bedienfeldmenüs
//! (`panel.*`, mit Befehls-ID des Originals, damit die ≡-Menüs der Bedienfelder über
//! [`crate::CommandRegistry::by_fw_id`] wirken).
//!
//! `pi.*`: Inspektor-, Bedienfeld- und Dialogänderungen, für die es keinen Menübefehl gibt –
//! Füllkategorie/Kante/Textur, Strichkategorie/Weichheit, Eckenrundung, Effektparameter,
//! Pfadoperationen (Compound Shapes), Web-Objekte, Schaltflächen, Status-Eigenschaften,
//! Exporteinstellungen, Stile und Symbole. Sie stehen im Register von
//! [`crate::CommandRegistry::with_defaults`] und sind damit auch für CLI, MCP und JSF sichtbar.
//! Jede Änderung ist ein Verlaufsschritt mit Beschriftung.
//!
//! Herkunft: Phase 4 in fp-ui (`ui_commands.rs`), Phase 5 hierher verschoben (Agent IBD).

use crate::command::{cmd, Command, CommandInfo, CommandOutput, CommandRegistry, Effect as CmdEffect, FnCommand, ParamKind, Params, Requirement};
use crate::session::{DocumentState, SessionEvent};
use crate::{docops, EngineError, PrefValue, Result, Session};
use fp_color::{Argb, Rgba8};
use fp_model::{LayerKind, LayerSlot};
use fp_geom::{Point, Rect};
use fp_model::effects::{Effect, EffectList};
use fp_model::{Brush, Element, ElementId, ElementKind, Fill, Gradient, GradientStop, GradientStops, StopList, Texture};
use serde_json::{json, Value};
use std::sync::Arc;

/// Präfix der Adapter-Befehle.
pub const PREFIX: &str = "pi.";

const DOC: &[Requirement] = &[Requirement::Document];

fn bad(p: &Params, msg: impl Into<String>) -> EngineError {
    EngineError::InvalidParams { name: p.command.clone(), message: msg.into() }
}
// ---------------------------------------------------------------------- Hilfen

/// Ziel-IDs: Parameter `ids` oder die Auswahl.
fn targets(s: &Session, p: &Params) -> Result<(usize, Vec<ElementId>)> {
    let d = s.doc()?;
    let ids = match p.opt_ids("ids")? {
        Some(v) => v,
        None => d.selection.elements.clone(),
    };
    if ids.is_empty() {
        return Err(EngineError::Selection(fp_i18n::t("err-sel-nothing")));
    }
    Ok((d.page, ids))
}

/// Ändert jedes Zielelement.
fn edit_each(s: &mut Session, p: &Params, f: impl Fn(&mut Element, Option<Rect>)) -> Result<CommandOutput> {
    let (page, ids) = targets(s, p)?;
    let d = s.doc_mut()?;
    let doc = Arc::make_mut(&mut d.doc);
    for id in ids {
        let bounds = docops::element_on_page(doc, page, id).and_then(|e| docops::element_bounds(doc, &e));
        docops::edit_by_id(doc, page, id, |e| f(e, bounds)).ok_or(EngineError::ElementNotFound(id.0))?;
    }
    Ok(CommandOutput::doc())
}

/// Wie [`edit_each`], aber `f` meldet, ob sich das Element geändert hat. Ändert sich keines, bleibt
/// das Dokument dasselbe (kein Verlaufsschritt, z. B. wiederholtes `pi.fill` mit derselben Art).
fn edit_each_changed(s: &mut Session, p: &Params, f: impl Fn(&mut Element, Option<Rect>) -> bool) -> Result<CommandOutput> {
    let (page, ids) = targets(s, p)?;
    let d = s.doc_mut()?;
    let orig = Arc::clone(&d.doc);
    let doc = Arc::make_mut(&mut d.doc);
    let mut changed = false;
    for id in ids {
        let bounds = docops::element_on_page(doc, page, id).and_then(|e| docops::element_bounds(doc, &e));
        changed |= docops::edit_by_id(doc, page, id, |e| f(e, bounds)).ok_or(EngineError::ElementNotFound(id.0))?;
    }
    if !changed {
        d.doc = orig;
    }
    Ok(CommandOutput::doc())
}

fn parse_color(v: &Value) -> Option<Argb> {
    match v {
        Value::String(s) => Rgba8::parse_hex(s).map(|c| c.to_argb()),
        Value::Number(n) => n.as_u64().map(|x| Argb(x as u32)),
        _ => None,
    }
}

/// `#rrggbbaa` einer Farbe.
pub fn hex(c: Argb) -> String {
    format!("#{:02x}{:02x}{:02x}{:02x}", c.r(), c.g(), c.b(), c.a())
}

// ---------------------------------------------------------------------- Füllung

/// Verlaufsformen in der Reihenfolge der Liste „Verlauf:“ im Verlaufs-Popup des Originals (12 Arten,
/// PAR 18/02): (Anzeigename, Formkennung, `CATs`, `INMs`). Die Formkennung ist `FSHi`; nur der
/// Konturverlauf hat die Kennung [`CONTOUR_SHAPE`] (gespeichert als `FSHi` 3 mit `FSXi` 17).
pub const GRADIENT_SHAPES: &[(&str, i32, &str, &str)] = &[
    ("Linear", 2, "fc_Linear", "Linear"),
    ("Radial", 3, "fc_Circular", "Radial"),
    ("Rechteck", 10, "fc_Rectangular", "Rectangular"),
    ("Kegel", 4, "fc_Conical", "Conical"),
    ("Kontur", CONTOUR_SHAPE, "fc_ContourGrad", "Contour"),
    ("Glanz", 5, "fc_Satin", "Satin"),
    ("Supernova", 7, "fc_Pinch", "Pinch"),
    ("Falten", 8, "fc_Folds", "Folds"),
    ("Ellipse", 9, "fc_Elliptical", "Elliptical"),
    ("Streifen", 12, "fc_Bars", "Bars"),
    ("Kleine Wellen", 13, "fc_Ripple", "Ripple"),
    ("Wellen", 14, "fc_Waves", "Waves"),
];

/// Formkennung des Konturverlaufs in [`GRADIENT_SHAPES`] und im Parameter `shape` von `pi.fill`.
pub const CONTOUR_SHAPE: i32 = 17;

/// Formkennung einer Füllung wie in [`GRADIENT_SHAPES`] (Konturverlauf: [`CONTOUR_SHAPE`]).
pub fn shape_id(f: &Fill) -> i32 {
    if f.is_contour_gradient() { CONTOUR_SHAPE } else { f.shape }
}

/// Setzt die Form einer Füllung aus der Formkennung (`FSHi`/`FSXi`).
fn set_shape_id(f: &mut Fill, id: i32) {
    if id == CONTOUR_SHAPE {
        f.shape = 3;
        f.shape_ext = 17;
    } else {
        f.shape = id;
        f.shape_ext = 0;
    }
}

/// Name des Vorgabemusters beim Umschalten auf „Muster“ (Original: „Baumwolle“, JSF „Cotton“).
pub const DEFAULT_PATTERN: &str = "Cotton";

/// Winkel des Verlaufsvektors in Grad wie im Verlaufs-Popup des Originals (0 = nach rechts,
/// 90 = nach oben, Vorgabe senkrecht nach unten = 270).
pub fn handles_angle(h: &fp_model::FillHandles) -> f64 {
    let (dx, dy) = ((h.end.x - h.start.x) as f64, (h.end.y - h.start.y) as f64);
    if dx.abs() < 1e-9 && dy.abs() < 1e-9 {
        return 0.0;
    }
    let a = (-dy).atan2(dx).to_degrees().rem_euclid(360.0);
    if (a - 360.0).abs() < 1e-6 { 0.0 } else { a }
}

/// Dreht die Griffe um den Startpunkt, sodass der Vektor den Winkel `deg` hat (Länge bleibt).
fn rotate_handles(h: &mut fp_model::FillHandles, deg: f64) {
    let cur = handles_angle(h);
    let d = (cur - deg).to_radians();
    let (s, c) = d.sin_cos();
    let rot = |p: Point| {
        let (x, y) = ((p.x - h.start.x) as f64, (p.y - h.start.y) as f64);
        // Bildschirmkoordinaten (y nach unten): Drehung um −d im mathematischen Sinn
        Point::new(h.start.x + (x * c - y * s) as f32, h.start.y + (x * s + y * c) as f32)
    };
    h.end = rot(h.end);
    h.second = rot(h.second);
}

/// Sind die Griffe entartet (Start = Ende oder keine zweite Achse)? Dann zeichnet ein Verlauf bzw.
/// ein Muster nur eine Farbe (PAR 18/03).
fn degenerate(h: &fp_model::FillHandles) -> bool {
    let (ax, ay) = (h.end.x - h.start.x, h.end.y - h.start.y);
    let (bx, by) = (h.second.x - h.start.x, h.second.y - h.start.y);
    !h.valid || (ax * by - ay * bx).abs() < 1e-6
}

/// Kategorie einer Füllung für die Füllart-Knöpfe: 0 keine, 1 einfarbig, 2 Verlauf, 3 Muster, 4 Web-Dither.
pub fn fill_kind(f: Option<&Fill>) -> u8 {
    match f {
        None => 0,
        Some(f) if f.shape == 15 => 3,
        Some(f) if f.shape == 16 => 4,
        Some(f) if f.gradient.is_some() && f.shape != 0 => 2,
        Some(_) => 1,
    }
}

fn stop_list(v: &[(f32, Argb)]) -> StopList {
    StopList { stops: v.iter().map(|&(p, c)| GradientStop { position: p.clamp(0.0, 1.0), color: c, extra: Default::default() }).collect(), extra: Default::default() }
}

/// Verlauf aus Farbstopps (Deckkraft aus dem Alpha der Farben).
pub fn make_gradient(shape: i32, colors: &[(f32, Argb)], opacities: Option<&[(f32, Argb)]>) -> Gradient {
    let (cat, name) = GRADIENT_SHAPES.iter().find(|g| g.1 == shape).map(|g| (g.2, g.3)).unwrap_or(("fc_Linear", "Linear"));
    let opac: Vec<(f32, Argb)> = match opacities {
        Some(o) => o.to_vec(),
        None => colors.iter().map(|&(p, c)| (p, Argb(((c.a() as u32) << 24) | 0x00ff_ffff))).collect(),
    };
    let solid: Vec<(f32, Argb)> = colors.iter().map(|&(p, c)| (p, Argb(c.0 | 0xff00_0000))).collect();
    Gradient {
        category: cat.into(),
        name: name.into(),
        user_name: None,
        legacy_stops: None,
        stops: Some(GradientStops { colors: stop_list(&solid), opacities: stop_list(&opac), dither: None, dither_value: None, extra: Default::default() }),
        extra: Default::default(),
    }
}

fn parse_stops(v: Option<&Value>) -> Option<Vec<(f32, Argb)>> {
    let a = v?.as_array()?;
    let mut out = Vec::new();
    for s in a {
        let pos = s.get(0).and_then(|x| x.as_f64())? as f32;
        let c = s.get(1).and_then(parse_color)?;
        out.push((pos, c));
    }
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    (out.len() >= 2).then_some(out)
}

/// Vorgabegriffe eines neuen Verlaufs je Art, so dass das Ergebnis den Exporten des Originals
/// gleicht (PAR 18/03, 18/03b; `re\_testdateien\par\px\inspector-fill\03_*.png`, Objekt 50,50–150,120):
/// - Linear: Ober- → Unterkante (Winkel 270°), zweite Achse senkrecht dazu
/// - Kontur: Mitte → halbe kleinere Seite nach unten
/// - Radial, Rechteck, Kegel, Ellipse, Streifen, Kleine Wellen: Mitte → Mitte rechts,
///   zweite Achse Mitte → Mitte oben (Kreis mit halber Breite, Rechteck/Ellipse füllen das Objekt)
/// - Glanz, Falten: Mitte → rechte untere Ecke; Wellen: Mitte → halber Weg dorthin
/// - Supernova: Mitte → rechte untere Ecke, zweite Achse Mitte → halber Weg zur linken unteren Ecke
pub fn default_handles(b: Option<Rect>, shape: i32) -> fp_model::FillHandles {
    let r = b.unwrap_or(Rect::new(0.0, 0.0, 100.0, 100.0));
    let cy = (r.top + r.bottom) / 2.0;
    let cx = (r.left + r.right) / 2.0;
    let (qw, qh) = ((r.right - r.left) / 4.0, (r.bottom - r.top) / 4.0);
    let c = Point::new(cx, cy);
    let rot = |s: Point, e: Point| Point::new(s.x - (e.y - s.y), s.y + (e.x - s.x));
    let (start, end, second) = match shape {
        2 => {
            let (s, e) = (Point::new(cx, r.top), Point::new(cx, r.bottom));
            (s, e, rot(s, e))
        }
        5 | 8 => {
            let e = Point::new(r.right, r.bottom);
            (c, e, rot(c, e))
        }
        14 => {
            let e = Point::new(cx + qw, cy + qh);
            (c, e, rot(c, e))
        }
        7 => (c, Point::new(r.right, r.bottom), Point::new(cx - qw, cy + qh)),
        // Kontur: Griffabstand = halbe kleinere Seite, damit die Stopps nicht umgerechnet werden
        // (Verlauf vom Rand bis zur Mittellinie wie im Export des Originals)
        CONTOUR_SHAPE => {
            let e = Point::new(cx, cy + 2.0 * qw.min(qh));
            (c, e, rot(c, e))
        }
        _ => (c, Point::new(r.right, cy), Point::new(cx, r.top)),
    };
    fp_model::FillHandles { valid: true, start, end, second }
}

/// Griffe setzen: `force` immer (Umschalten auf Verlauf), sonst nur ungültige bzw. entartete.
fn set_handles(e: &mut Element, h: fp_model::FillHandles, force: bool) {
    match &mut e.kind {
        ElementKind::Path(pe) => {
            if force || degenerate(&pe.fill_handles) {
                pe.fill_handles = h;
            }
        }
        ElementKind::Text(t) => {
            if force || degenerate(&t.fill_handles) {
                t.fill_handles = h;
            }
        }
        ElementKind::Group(g) => {
            for c in &mut g.children {
                set_handles(Element::edit(c), h, force);
            }
        }
        _ => {}
    }
}

/// Griffe eines Elements (erstes Kind einer Gruppe).
pub fn handles_of(e: &Element) -> Option<fp_model::FillHandles> {
    match &e.kind {
        ElementKind::Path(pe) => Some(pe.fill_handles),
        ElementKind::Text(t) => Some(t.fill_handles),
        ElementKind::Group(g) => g.children.iter().find_map(|c| handles_of(c)),
        _ => None,
    }
}

/// Dreht die Griffe eines Elements (und der Kinder einer Gruppe) auf den Winkel `deg`.
fn rotate_element_handles(e: &mut Element, deg: f64) {
    match &mut e.kind {
        ElementKind::Path(pe) => rotate_handles(&mut pe.fill_handles, deg),
        ElementKind::Text(t) => rotate_handles(&mut t.fill_handles, deg),
        ElementKind::Group(g) => {
            for c in &mut g.children {
                rotate_element_handles(Element::edit(c), deg);
            }
        }
        _ => {}
    }
}

/// Textur nach Namen: mit Maske aus den Texturordnern (`crate::resources`), sonst nur der Name
/// (ohne Datei bleibt die Textur ohne Wirkung, wie bisher).
/// Ein absoluter Pfad („Andere…“ mit einer Datei außerhalb der Texturordner) lädt diese Datei;
/// Name ist dann der Dateiname ohne Endung.
fn texture(name: &str) -> Texture {
    let path = std::path::Path::new(name.trim());
    if path.is_absolute()
        && let Some(stem) = path.file_stem().map(|s| s.to_string_lossy().to_string())
        && let Some(t) = crate::resources::texture_from_file(path, &stem)
    {
        return t;
    }
    crate::resources::load_texture(name).unwrap_or_else(|| Texture { name: name.to_string(), user_name: Some(name.to_string()), mask: None, extra: Default::default() })
}

/// Unbekannter Texturname → Fehlermeldung (AGENTS §3), sobald es auf diesem Rechner Texturdateien
/// gibt; ohne Texturordner (Rechner ohne Ressourcen) bleibt es beim Namen wie bisher. Leer = keine
/// Textur; volle Pfade müssen eine lesbare Bilddatei sein.
fn check_texture_name(p: &Params, name: Option<&str>) -> Result<()> {
    match name.map(str::trim).filter(|n| !texture_name_known(n)) {
        Some(n) => Err(EngineError::InvalidParams { name: p.command.clone(), message: fp_i18n::tr!("err-not-found", what = n) }),
        None => Ok(()),
    }
}

/// Texturname gültig für `pi.fill`/`pi.stroke` (siehe [`check_texture_name`]).
pub(crate) fn texture_name_known(name: &str) -> bool {
    let n = name.trim();
    n.is_empty()
        || crate::resources::texture_file(n).is_some()
        || (!std::path::Path::new(n).is_absolute() && crate::resources::files_in(&crate::resources::dirs(crate::resources::TEXTURES)).is_empty())
}

// ---------------------------------------------------------------------- Pinsel

/// Eine Strichvorgabe: vollständiges Pinselobjekt je Art, gemessen am Original (RE-PINSEL,
/// `re\24-pinsel.md`, Werte aus `re\_testdateien\par\px\tool-brush-stroke-options\presets.csv`;
/// Prozentwerte hier in Promille wie im Modell). Kategorien und Arten in der Reihenfolge des
/// Strichart-Menüs (Arten alphabetisch nach dem Anzeigenamen).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrushPreset {
    pub category: &'static str,
    pub category_de: &'static str,
    pub name: &'static str,
    pub name_de: &'static str,
    /// Natürliche Pinsel-Engine (sonst „simple“).
    pub natural: bool,
    /// Vorgabestärke (nur, wenn noch kein Strich da ist; ein Wechsel der Art behält die Größe).
    pub diameter: i32,
    pub softness: i32,
    pub round: bool,
    pub antialiased: bool,
    /// Weichheit als Glockenkurve (sonst linear).
    pub bell: bool,
    pub aspect: i32,
    pub angle: i32,
    pub concentration: i32,
    pub blackness: i32,
    pub spacing: i32,
    pub flow_rate: i32,
    pub feedback: i32,
    pub texture_blend: i32,
    pub texture_edge: i32,
    pub alpha_remap: i32,
    pub tip_count: i32,
    pub tip_spacing: i32,
    pub tip_spacing_mode: i32,
    pub tip_coloring_mode: i32,
    pub dash_count: i32,
    pub dash_on: [i32; 3],
    pub dash_off: [i32; 3],
    pub min_size: i32,
    pub max_count: i32,
    /// Empfindlichkeiten (Quelle `PSHVR`, Ziel `ZAOBRHLS`, Promille).
    pub sensitivities: &'static [(u8, u8, i32)],
}

/// Strichvorgaben in Menüreihenfolge (12 Kategorien, 54 Arten wie im Original).
pub const BRUSH_PRESETS: &[BrushPreset] = &[
    BrushPreset {
        category: "bc_Basic", category_de: "Einfach", name: "bn_Hard Rounded", name_de: "Präzise abger.",
        natural: false, diameter: 20, softness: 0, round: true, antialiased: false, bell: true,
        aspect: 1000, angle: 0, concentration: 1000, blackness: 0, spacing: 60, flow_rate: 0, feedback: 0,
        texture_blend: 0, texture_edge: 0, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [10, 1, 1], dash_off: [2, 2, 2], min_size: 10, max_count: 14,
        sensitivities: &[],
    },
    BrushPreset {
        category: "bc_Basic", category_de: "Einfach", name: "bn_Hard Line", name_de: "Präzise Linie",
        natural: false, diameter: 20, softness: 0, round: false, antialiased: false, bell: true,
        aspect: 1000, angle: 0, concentration: 1000, blackness: 0, spacing: 60, flow_rate: 0, feedback: 0,
        texture_blend: 0, texture_edge: 0, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [10, 1, 1], dash_off: [2, 2, 2], min_size: 10, max_count: 14,
        sensitivities: &[],
    },
    BrushPreset {
        category: "bc_Basic", category_de: "Einfach", name: "bn_Soft Rounded", name_de: "Weich abger.",
        natural: false, diameter: 20, softness: 500, round: true, antialiased: true, bell: true,
        aspect: 1000, angle: 0, concentration: 1000, blackness: 0, spacing: 60, flow_rate: 0, feedback: 1,
        texture_blend: 0, texture_edge: 0, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [10, 1, 1], dash_off: [2, 2, 2], min_size: 10, max_count: 14,
        sensitivities: &[],
    },
    BrushPreset {
        category: "bc_Basic", category_de: "Einfach", name: "bn_Soft Line", name_de: "Weiche Linie",
        natural: false, diameter: 20, softness: 500, round: false, antialiased: true, bell: true,
        aspect: 1000, angle: 0, concentration: 1000, blackness: 0, spacing: 60, flow_rate: 0, feedback: 1,
        texture_blend: 0, texture_edge: 0, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [10, 1, 1], dash_off: [2, 2, 2], min_size: 10, max_count: 14,
        sensitivities: &[],
    },
    BrushPreset {
        category: "bc_Air Brush", category_de: "Airbrush", name: "bn_Basic", name_de: "Einfach",
        natural: true, diameter: 20, softness: 1000, round: true, antialiased: false, bell: true,
        aspect: 1000, angle: 0, concentration: 900, blackness: 0, spacing: 220, flow_rate: 800, feedback: 1,
        texture_blend: 0, texture_edge: 0, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [10, 2, 2], dash_off: [4, 4, 4], min_size: 100, max_count: 14,
        sensitivities: &[(b'P', b'O', 1000), (b'P', b'Z', 500), (b'S', b'O', 203), (b'S', b'Z', 203)],
    },
    BrushPreset {
        category: "bc_Air Brush", category_de: "Airbrush", name: "bn_Textured", name_de: "Texturiert",
        natural: true, diameter: 20, softness: 1000, round: true, antialiased: false, bell: true,
        aspect: 1000, angle: 0, concentration: 700, blackness: 400, spacing: 200, flow_rate: 800, feedback: 1,
        texture_blend: 800, texture_edge: 0, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [10, 2, 2], dash_off: [4, 4, 4], min_size: 10, max_count: 14,
        sensitivities: &[(b'P', b'O', 1000), (b'P', b'Z', 297), (b'S', b'O', 203), (b'S', b'Z', 203)],
    },
    BrushPreset {
        category: "bc_Pencil", category_de: "Bleistift", name: "bn_1-Pixel", name_de: "1-Pixel hart",
        natural: false, diameter: 1, softness: 0, round: true, antialiased: false, bell: true,
        aspect: 1000, angle: 0, concentration: 1000, blackness: 0, spacing: 150, flow_rate: 0, feedback: 0,
        texture_blend: 0, texture_edge: 0, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [8, 1, 1], dash_off: [2, 2, 2], min_size: 10, max_count: 15,
        sensitivities: &[],
    },
    BrushPreset {
        category: "bc_Pencil", category_de: "Bleistift", name: "bn_1-Pixel Anti-Aliased", name_de: "1-Pixel weich",
        natural: false, diameter: 1, softness: 0, round: true, antialiased: true, bell: true,
        aspect: 1000, angle: 0, concentration: 1000, blackness: 0, spacing: 150, flow_rate: 0, feedback: 1,
        texture_blend: 0, texture_edge: 0, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [8, 1, 1], dash_off: [2, 2, 2], min_size: 10, max_count: 15,
        sensitivities: &[],
    },
    BrushPreset {
        category: "bc_Pencil", category_de: "Bleistift", name: "bn_Colored Pencil", name_de: "Buntstift",
        natural: true, diameter: 20, softness: 10, round: false, antialiased: false, bell: true,
        aspect: 600, angle: 90, concentration: 1000, blackness: 0, spacing: 150, flow_rate: 0, feedback: 1,
        texture_blend: 0, texture_edge: 10, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [8, 1, 1], dash_off: [2, 2, 2], min_size: 10, max_count: 14,
        sensitivities: &[(b'P', b'B', 102), (b'P', b'O', 602), (b'S', b'B', 102), (b'S', b'O', 602), (b'S', b'Z', 102)],
    },
    BrushPreset {
        category: "bc_Pencil", category_de: "Bleistift", name: "bn_Graphite", name_de: "Graphit",
        natural: true, diameter: 20, softness: 0, round: false, antialiased: true, bell: true,
        aspect: 1000, angle: 0, concentration: 972, blackness: 0, spacing: 200, flow_rate: 0, feedback: 1,
        texture_blend: 800, texture_edge: 0, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [20, 10, 10], dash_off: [8, 8, 8], min_size: 10, max_count: 14,
        sensitivities: &[(b'P', b'O', 602), (b'P', b'Z', 203), (b'S', b'O', 602), (b'S', b'Z', 203)],
    },
    BrushPreset {
        category: "bc_Felt Tip", category_de: "Filzstift", name: "bn_Dark Marker", name_de: "Dunkler Marker",
        natural: true, diameter: 20, softness: 0, round: true, antialiased: true, bell: true,
        aspect: 1000, angle: 0, concentration: 900, blackness: 350, spacing: 70, flow_rate: 0, feedback: 1,
        texture_blend: 0, texture_edge: 0, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [40, 2, 2], dash_off: [6, 8, 8], min_size: 700, max_count: 14,
        sensitivities: &[(b'P', b'B', 500), (b'S', b'B', 203)],
    },
    BrushPreset {
        category: "bc_Felt Tip", category_de: "Filzstift", name: "bn_Thin", name_de: "Dünn",
        natural: true, diameter: 20, softness: 0, round: true, antialiased: true, bell: true,
        aspect: 1000, angle: 0, concentration: 700, blackness: 400, spacing: 150, flow_rate: 0, feedback: 1,
        texture_blend: 0, texture_edge: 0, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [40, 2, 2], dash_off: [6, 8, 8], min_size: 10, max_count: 14,
        sensitivities: &[(b'P', b'B', 703), (b'S', b'B', 602)],
    },
    BrushPreset {
        category: "bc_Felt Tip", category_de: "Filzstift", name: "bn_Light Marker", name_de: "Heller Marker",
        natural: true, diameter: 20, softness: 50, round: false, antialiased: true, bell: true,
        aspect: 120, angle: 125, concentration: 1000, blackness: 300, spacing: 70, flow_rate: 0, feedback: 1,
        texture_blend: 0, texture_edge: 0, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [40, 2, 2], dash_off: [6, 8, 8], min_size: 10, max_count: 14,
        sensitivities: &[(b'P', b'B', 703), (b'P', b'O', 602), (b'S', b'B', 398), (b'S', b'O', 797)],
    },
    BrushPreset {
        category: "bc_Felt Tip", category_de: "Filzstift", name: "bn_Highlighter", name_de: "Hervorhebung",
        natural: true, diameter: 20, softness: 0, round: false, antialiased: true, bell: false,
        aspect: 450, angle: 90, concentration: 500, blackness: 0, spacing: 90, flow_rate: 0, feedback: 1,
        texture_blend: 0, texture_edge: 0, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [40, 2, 2], dash_off: [6, 8, 8], min_size: 600, max_count: 9,
        sensitivities: &[],
    },
    BrushPreset {
        category: "bc_Dashed", category_de: "Gestrichelt", name: "bn_Dashed_Heavy", name_de: "Dick gestrichelt",
        natural: false, diameter: 20, softness: 500, round: true, antialiased: true, bell: true,
        aspect: 1000, angle: 0, concentration: 1000, blackness: 0, spacing: 60, flow_rate: 0, feedback: 1,
        texture_blend: 0, texture_edge: 0, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 1, dash_on: [20, 5, 5], dash_off: [10, 10, 10], min_size: 10, max_count: 14,
        sensitivities: &[],
    },
    BrushPreset {
        category: "bc_Dashed", category_de: "Gestrichelt", name: "bn_Dashed_Double", name_de: "Doppelt gestr.",
        natural: false, diameter: 20, softness: 500, round: true, antialiased: true, bell: true,
        aspect: 1000, angle: 0, concentration: 1000, blackness: 0, spacing: 60, flow_rate: 0, feedback: 1,
        texture_blend: 0, texture_edge: 0, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 2, dash_on: [10, 1, 1], dash_off: [2, 2, 2], min_size: 10, max_count: 14,
        sensitivities: &[],
    },
    BrushPreset {
        category: "bc_Dashed", category_de: "Gestrichelt", name: "bn_Dashed_Triple", name_de: "Dreifach gestr.",
        natural: false, diameter: 20, softness: 500, round: true, antialiased: true, bell: true,
        aspect: 1000, angle: 0, concentration: 1000, blackness: 0, spacing: 60, flow_rate: 0, feedback: 1,
        texture_blend: 0, texture_edge: 0, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 3, dash_on: [10, 1, 1], dash_off: [2, 2, 2], min_size: 10, max_count: 14,
        sensitivities: &[],
    },
    BrushPreset {
        category: "bc_Dashed", category_de: "Gestrichelt", name: "bn_Dashed_Basic", name_de: "Einfach gestr.",
        natural: false, diameter: 20, softness: 500, round: true, antialiased: true, bell: true,
        aspect: 1000, angle: 0, concentration: 1000, blackness: 0, spacing: 60, flow_rate: 0, feedback: 1,
        texture_blend: 0, texture_edge: 0, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 1, dash_on: [10, 1, 1], dash_off: [2, 2, 2], min_size: 10, max_count: 14,
        sensitivities: &[],
    },
    BrushPreset {
        category: "bc_Dashed", category_de: "Gestrichelt", name: "bn_Dotted", name_de: "Gepunktet",
        natural: true, diameter: 20, softness: 500, round: true, antialiased: true, bell: true,
        aspect: 1000, angle: 0, concentration: 1000, blackness: 0, spacing: 2500, flow_rate: 0, feedback: 1,
        texture_blend: 0, texture_edge: 0, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [0, 0, 0], dash_off: [0, 0, 0], min_size: 10, max_count: 14,
        sensitivities: &[],
    },
    BrushPreset {
        category: "bc_Dashed", category_de: "Gestrichelt", name: "bn_Dashed_Hard", name_de: "Präzise gestr.",
        natural: false, diameter: 20, softness: 0, round: false, antialiased: false, bell: true,
        aspect: 1000, angle: 0, concentration: 1000, blackness: 0, spacing: 60, flow_rate: 0, feedback: 0,
        texture_blend: 0, texture_edge: 0, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 1, dash_on: [10, 1, 1], dash_off: [5, 5, 5], min_size: 10, max_count: 14,
        sensitivities: &[],
    },
    BrushPreset {
        category: "bc_Calligraphy", category_de: "Kalligrafie", name: "bn_Bamboo", name_de: "Bambus",
        natural: true, diameter: 20, softness: 0, round: true, antialiased: true, bell: true,
        aspect: 1000, angle: 0, concentration: 1000, blackness: 260, spacing: 80, flow_rate: 0, feedback: 1,
        texture_blend: 500, texture_edge: 250, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [40, 2, 2], dash_off: [4, 4, 4], min_size: 10, max_count: 19,
        sensitivities: &[(b'P', b'Z', 898), (b'S', b'Z', 797)],
    },
    BrushPreset {
        category: "bc_Calligraphy", category_de: "Kalligrafie", name: "bn_Ribbon", name_de: "Band",
        natural: true, diameter: 20, softness: 0, round: false, antialiased: true, bell: true,
        aspect: 140, angle: 125, concentration: 1000, blackness: 0, spacing: 60, flow_rate: 0, feedback: 1,
        texture_blend: 0, texture_edge: 0, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [40, 2, 2], dash_off: [4, 4, 4], min_size: 10, max_count: 14,
        sensitivities: &[],
    },
    BrushPreset {
        category: "bc_Calligraphy", category_de: "Kalligrafie", name: "bn_Basic", name_de: "Einfach",
        natural: true, diameter: 20, softness: 0, round: false, antialiased: true, bell: true,
        aspect: 140, angle: 125, concentration: 1000, blackness: 0, spacing: 50, flow_rate: 0, feedback: 0,
        texture_blend: 0, texture_edge: 0, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [40, 2, 2], dash_off: [4, 4, 4], min_size: 10, max_count: 14,
        sensitivities: &[],
    },
    BrushPreset {
        category: "bc_Calligraphy", category_de: "Kalligrafie", name: "bn_Quill", name_de: "Federkiel",
        natural: true, diameter: 20, softness: 200, round: true, antialiased: true, bell: true,
        aspect: 440, angle: 69, concentration: 1000, blackness: 250, spacing: 100, flow_rate: 900, feedback: 1,
        texture_blend: 250, texture_edge: 0, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [40, 2, 2], dash_off: [4, 4, 4], min_size: 160, max_count: 15,
        sensitivities: &[(b'P', b'O', 1000), (b'P', b'Z', 898), (b'S', b'Z', 797)],
    },
    BrushPreset {
        category: "bc_Calligraphy", category_de: "Kalligrafie", name: "bn_Wet", name_de: "Naß",
        natural: true, diameter: 20, softness: 20, round: false, antialiased: true, bell: true,
        aspect: 120, angle: 125, concentration: 1000, blackness: 120, spacing: 60, flow_rate: 0, feedback: 1,
        texture_blend: 0, texture_edge: 0, alpha_remap: 2, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [40, 2, 2], dash_off: [4, 4, 4], min_size: 10, max_count: 14,
        sensitivities: &[(b'P', b'B', 203), (b'S', b'B', 203)],
    },
    BrushPreset {
        category: "bc_Charcoal", category_de: "Kohlestift", name: "bn_Creamy", name_de: "Cremig",
        natural: true, diameter: 20, softness: 20, round: true, antialiased: false, bell: true,
        aspect: 1000, angle: 0, concentration: 1000, blackness: 380, spacing: 70, flow_rate: 0, feedback: 1,
        texture_blend: 160, texture_edge: 980, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [40, 2, 2], dash_off: [4, 4, 4], min_size: 300, max_count: 14,
        sensitivities: &[(b'P', b'B', 602), (b'P', b'Z', 898), (b'S', b'B', 500), (b'S', b'Z', 797)],
    },
    BrushPreset {
        category: "bc_Charcoal", category_de: "Kohlestift", name: "bn_Pastel", name_de: "Pastell",
        natural: true, diameter: 20, softness: 0, round: true, antialiased: false, bell: true,
        aspect: 1000, angle: 0, concentration: 800, blackness: 500, spacing: 100, flow_rate: 0, feedback: 1,
        texture_blend: 240, texture_edge: 200, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [40, 2, 2], dash_off: [4, 4, 4], min_size: 500, max_count: 14,
        sensitivities: &[(b'P', b'B', 1000), (b'P', b'O', 297), (b'P', b'Z', 797), (b'R', b'B', 500), (b'S', b'B', 1000), (b'S', b'O', 297), (b'S', b'Z', 297)],
    },
    BrushPreset {
        category: "bc_Charcoal", category_de: "Kohlestift", name: "bn_Textured", name_de: "Texturiert",
        natural: true, diameter: 20, softness: 600, round: true, antialiased: false, bell: true,
        aspect: 1000, angle: 0, concentration: 1000, blackness: 450, spacing: 200, flow_rate: 0, feedback: 1,
        texture_blend: 850, texture_edge: 300, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [30, 2, 2], dash_off: [4, 4, 4], min_size: 10, max_count: 14,
        sensitivities: &[(b'P', b'B', 1000), (b'S', b'B', 47)],
    },
    BrushPreset {
        category: "bc_Charcoal", category_de: "Kohlestift", name: "bn_Soft", name_de: "Weich",
        natural: true, diameter: 20, softness: 500, round: true, antialiased: false, bell: true,
        aspect: 1000, angle: 0, concentration: 800, blackness: 300, spacing: 110, flow_rate: 0, feedback: 1,
        texture_blend: 300, texture_edge: 150, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [40, 2, 2], dash_off: [4, 4, 4], min_size: 200, max_count: 14,
        sensitivities: &[(b'R', b'B', 1000), (b'R', b'Z', 750)],
    },
    BrushPreset {
        category: "bc_Crayon", category_de: "Pastellkreide", name: "bn_Thick", name_de: "Dick",
        natural: true, diameter: 20, softness: 0, round: true, antialiased: false, bell: true,
        aspect: 1000, angle: 0, concentration: 1000, blackness: 250, spacing: 100, flow_rate: 0, feedback: 1,
        texture_blend: 200, texture_edge: 950, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [40, 2, 2], dash_off: [4, 4, 4], min_size: 700, max_count: 14,
        sensitivities: &[(b'P', b'O', 797), (b'P', b'Z', 1000), (b'R', b'H', 47), (b'R', b'Z', 297), (b'S', b'O', 203), (b'S', b'Z', 703)],
    },
    BrushPreset {
        category: "bc_Crayon", category_de: "Pastellkreide", name: "bn_Basic", name_de: "Einfach",
        natural: true, diameter: 20, softness: 0, round: true, antialiased: false, bell: true,
        aspect: 1000, angle: 0, concentration: 1000, blackness: 0, spacing: 100, flow_rate: 0, feedback: 1,
        texture_blend: 650, texture_edge: 400, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [50, 2, 2], dash_off: [6, 6, 6], min_size: 700, max_count: 14,
        sensitivities: &[(b'P', b'O', 297), (b'P', b'Z', 602), (b'R', b'H', 47), (b'S', b'O', 203), (b'S', b'Z', 703)],
    },
    BrushPreset {
        category: "bc_Crayon", category_de: "Pastellkreide", name: "bn_Rake", name_de: "Harke",
        natural: true, diameter: 20, softness: 0, round: true, antialiased: false, bell: false,
        aspect: 1000, angle: 0, concentration: 1000, blackness: 0, spacing: 300, flow_rate: 0, feedback: 1,
        texture_blend: 650, texture_edge: 400, alpha_remap: 0, tip_count: 4, tip_spacing: 90, tip_spacing_mode: 2, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [80, 25, 25], dash_off: [100, 100, 100], min_size: 900, max_count: 9,
        sensitivities: &[(b'P', b'O', 602), (b'P', b'Z', 602), (b'R', b'H', 250), (b'S', b'O', 203), (b'S', b'Z', 703)],
    },
    BrushPreset {
        category: "bc_Watercolor", category_de: "Wasserfarbe", name: "bn_Thick", name_de: "Dick",
        natural: true, diameter: 20, softness: 700, round: true, antialiased: false, bell: true,
        aspect: 1000, angle: 0, concentration: 850, blackness: 0, spacing: 100, flow_rate: 0, feedback: 1,
        texture_blend: 50, texture_edge: 350, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [40, 2, 2], dash_off: [3, 4, 4], min_size: 10, max_count: 14,
        sensitivities: &[(b'P', b'O', 602), (b'P', b'Z', 102), (b'S', b'O', 602), (b'S', b'Z', 102)],
    },
    BrushPreset {
        category: "bc_Watercolor", category_de: "Wasserfarbe", name: "bn_Thin", name_de: "Dünn",
        natural: true, diameter: 20, softness: 250, round: true, antialiased: true, bell: true,
        aspect: 1000, angle: 0, concentration: 950, blackness: 0, spacing: 90, flow_rate: 0, feedback: 1,
        texture_blend: 0, texture_edge: 0, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [30, 2, 2], dash_off: [2, 4, 4], min_size: 100, max_count: 31,
        sensitivities: &[(b'P', b'O', 797), (b'P', b'Z', 703), (b'S', b'O', 797), (b'S', b'Z', 602)],
    },
    BrushPreset {
        category: "bc_Watercolor", category_de: "Wasserfarbe", name: "bn_Heavy", name_de: "Intensiv",
        natural: true, diameter: 20, softness: 500, round: true, antialiased: false, bell: true,
        aspect: 1000, angle: 0, concentration: 850, blackness: 0, spacing: 150, flow_rate: 0, feedback: 1,
        texture_blend: 300, texture_edge: 990, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [30, 2, 2], dash_off: [8, 8, 8], min_size: 600, max_count: 14,
        sensitivities: &[(b'P', b'O', 500), (b'P', b'Z', 797), (b'R', b'H', 102), (b'R', b'O', 102), (b'R', b'R', 16), (b'S', b'O', 500), (b'S', b'Z', 703)],
    },
    BrushPreset {
        category: "bc_Oil", category_de: "Öl", name: "bn_Bristle", name_de: "Borsten",
        natural: true, diameter: 20, softness: 0, round: true, antialiased: true, bell: true,
        aspect: 1000, angle: 0, concentration: 950, blackness: 0, spacing: 300, flow_rate: 0, feedback: 1,
        texture_blend: 200, texture_edge: 200, alpha_remap: 0, tip_count: 3, tip_spacing: 50, tip_spacing_mode: 2, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [30, 2, 2], dash_off: [30, 40, 40], min_size: 10, max_count: 14,
        sensitivities: &[(b'P', b'O', 102), (b'R', b'A', 250), (b'R', b'H', 70), (b'R', b'S', 31), (b'R', b'Z', 203)],
    },
    BrushPreset {
        category: "bc_Oil", category_de: "Öl", name: "bn_Broad Splatter", name_de: "Breiter Kleckser",
        natural: true, diameter: 20, softness: 0, round: true, antialiased: false, bell: true,
        aspect: 1000, angle: 0, concentration: 750, blackness: 0, spacing: 450, flow_rate: 970, feedback: 1,
        texture_blend: 300, texture_edge: 220, alpha_remap: 0, tip_count: 2, tip_spacing: 500, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [150, 40, 40], dash_off: [200, 200, 200], min_size: 200, max_count: 14,
        sensitivities: &[(b'P', b'O', 602), (b'P', b'R', 297), (b'P', b'Z', 602), (b'R', b'B', 1000), (b'R', b'O', 602), (b'R', b'S', 1000), (b'R', b'R', 1000), (b'R', b'Z', 602), (b'S', b'O', 602), (b'S', b'Z', 602)],
    },
    BrushPreset {
        category: "bc_Oil", category_de: "Öl", name: "bn_Strands", name_de: "Fasern",
        natural: true, diameter: 20, softness: 430, round: true, antialiased: false, bell: true,
        aspect: 1000, angle: 0, concentration: 900, blackness: 0, spacing: 300, flow_rate: 0, feedback: 1,
        texture_blend: 0, texture_edge: 0, alpha_remap: 0, tip_count: 5, tip_spacing: 150, tip_spacing_mode: 1, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [100, 20, 20], dash_off: [60, 60, 60], min_size: 100, max_count: 19,
        sensitivities: &[(b'R', b'H', 47), (b'R', b'S', 47), (b'R', b'R', 31), (b'R', b'Z', 102)],
    },
    BrushPreset {
        category: "bc_Oil", category_de: "Öl", name: "bn_Splatter", name_de: "Kleckser",
        natural: true, diameter: 20, softness: 0, round: true, antialiased: false, bell: true,
        aspect: 1000, angle: 0, concentration: 990, blackness: 0, spacing: 200, flow_rate: 970, feedback: 1,
        texture_blend: 300, texture_edge: 250, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [80, 20, 20], dash_off: [60, 60, 60], min_size: 200, max_count: 14,
        sensitivities: &[(b'P', b'O', 602), (b'P', b'R', 102), (b'R', b'O', 500), (b'R', b'S', 1000), (b'R', b'R', 250), (b'R', b'Z', 1000), (b'S', b'O', 602)],
    },
    BrushPreset {
        category: "bc_Oil", category_de: "Öl", name: "bn_Textured Bristle", name_de: "Txtur. Borsten",
        natural: true, diameter: 20, softness: 0, round: true, antialiased: true, bell: true,
        aspect: 1000, angle: 0, concentration: 700, blackness: 0, spacing: 250, flow_rate: 0, feedback: 1,
        texture_blend: 500, texture_edge: 0, alpha_remap: 0, tip_count: 4, tip_spacing: 75, tip_spacing_mode: 2, tip_coloring_mode: 4,
        dash_count: 0, dash_on: [100, 40, 40], dash_off: [80, 80, 80], min_size: 10, max_count: 14,
        sensitivities: &[(b'P', b'R', 47), (b'P', b'Z', 297), (b'R', b'H', 31), (b'R', b'S', 31), (b'R', b'R', 47), (b'S', b'R', 203), (b'S', b'Z', 297)],
    },
    BrushPreset {
        category: "bc_Unnatural", category_de: "Unnatürlich", name: "bn_3D", name_de: "3D",
        natural: true, diameter: 20, softness: 120, round: true, antialiased: false, bell: true,
        aspect: 1000, angle: 0, concentration: 1000, blackness: 0, spacing: 100, flow_rate: 0, feedback: 1,
        texture_blend: 0, texture_edge: 0, alpha_remap: 0, tip_count: 3, tip_spacing: 23, tip_spacing_mode: 2, tip_coloring_mode: 4,
        dash_count: 0, dash_on: [80, 20, 20], dash_off: [40, 40, 40], min_size: 10, max_count: 14,
        sensitivities: &[(b'R', b'H', 703)],
    },
    BrushPreset {
        category: "bc_Unnatural", category_de: "Unnatürlich", name: "bn_3D Glow", name_de: "3D Glühen",
        natural: true, diameter: 20, softness: 1000, round: true, antialiased: true, bell: true,
        aspect: 1000, angle: 0, concentration: 1000, blackness: 0, spacing: 200, flow_rate: 0, feedback: 1,
        texture_blend: 0, texture_edge: 0, alpha_remap: 1, tip_count: 4, tip_spacing: 30, tip_spacing_mode: 2, tip_coloring_mode: 3,
        dash_count: 0, dash_on: [80, 20, 20], dash_off: [40, 40, 40], min_size: 10, max_count: 14,
        sensitivities: &[(b'R', b'H', 703)],
    },
    BrushPreset {
        category: "bc_Unnatural", category_de: "Unnatürlich", name: "bn_Chameleon", name_de: "Chamäleon",
        natural: true, diameter: 20, softness: 0, round: true, antialiased: false, bell: true,
        aspect: 1000, angle: 0, concentration: 788, blackness: 0, spacing: 150, flow_rate: 0, feedback: 1,
        texture_blend: 310, texture_edge: 400, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [30, 10, 10], dash_off: [8, 8, 8], min_size: 10, max_count: 14,
        sensitivities: &[(b'P', b'H', 602), (b'S', b'H', 602)],
    },
    BrushPreset {
        category: "bc_Unnatural", category_de: "Unnatürlich", name: "bn_Paint Splatter", name_de: "Farbkleckser",
        natural: true, diameter: 20, softness: 1000, round: true, antialiased: false, bell: true,
        aspect: 1000, angle: 0, concentration: 1000, blackness: 250, spacing: 500, flow_rate: 920, feedback: 1,
        texture_blend: 0, texture_edge: 0, alpha_remap: 3, tip_count: 2, tip_spacing: 50, tip_spacing_mode: 2, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [60, 20, 20], dash_off: [150, 150, 150], min_size: 10, max_count: 14,
        sensitivities: &[(b'P', b'R', 203), (b'P', b'Z', 203), (b'R', b'H', 117), (b'R', b'S', 102), (b'R', b'R', 398), (b'R', b'Z', 602), (b'S', b'R', 203), (b'S', b'Z', 203)],
    },
    BrushPreset {
        category: "bc_Unnatural", category_de: "Unnatürlich", name: "bn_Fluid Splatter", name_de: "Flüssiger Kleckser",
        natural: true, diameter: 20, softness: 1000, round: true, antialiased: false, bell: true,
        aspect: 1000, angle: 0, concentration: 1000, blackness: 0, spacing: 170, flow_rate: 980, feedback: 1,
        texture_blend: 0, texture_edge: 0, alpha_remap: 1, tip_count: 2, tip_spacing: 30, tip_spacing_mode: 1, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [100, 40, 40], dash_off: [40, 40, 40], min_size: 10, max_count: 14,
        sensitivities: &[(b'R', b'H', 70), (b'R', b'R', 297), (b'R', b'Z', 398)],
    },
    BrushPreset {
        category: "bc_Unnatural", category_de: "Unnatürlich", name: "bn_Toxic Waste", name_de: "Giftmüll",
        natural: true, diameter: 20, softness: 1000, round: true, antialiased: false, bell: true,
        aspect: 1000, angle: 0, concentration: 1000, blackness: 400, spacing: 150, flow_rate: 0, feedback: 0,
        texture_blend: 0, texture_edge: 0, alpha_remap: 4, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [35, 10, 10], dash_off: [8, 8, 8], min_size: 800, max_count: 14,
        sensitivities: &[(b'P', b'B', 297), (b'P', b'H', 203), (b'P', b'O', 297), (b'R', b'H', 203), (b'R', b'S', 1000), (b'R', b'R', 47), (b'S', b'B', 297), (b'S', b'H', 1000), (b'S', b'O', 203), (b'S', b'Z', 711)],
    },
    BrushPreset {
        category: "bc_Unnatural", category_de: "Unnatürlich", name: "bn_Outline", name_de: "Kontur",
        natural: true, diameter: 20, softness: 1000, round: true, antialiased: true, bell: true,
        aspect: 1000, angle: 0, concentration: 1000, blackness: 0, spacing: 250, flow_rate: 0, feedback: 1,
        texture_blend: 0, texture_edge: 0, alpha_remap: 5, tip_count: 4, tip_spacing: 25, tip_spacing_mode: 2, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [50, 10, 10], dash_off: [40, 40, 40], min_size: 10, max_count: 14,
        sensitivities: &[(b'R', b'H', 703)],
    },
    BrushPreset {
        category: "bc_Unnatural", category_de: "Unnatürlich", name: "bn_Viscous Alien Paint", name_de: "SpaceAge-F.",
        natural: true, diameter: 20, softness: 300, round: true, antialiased: false, bell: true,
        aspect: 1000, angle: 0, concentration: 1000, blackness: 0, spacing: 200, flow_rate: 0, feedback: 1,
        texture_blend: 0, texture_edge: 50, alpha_remap: 3, tip_count: 3, tip_spacing: 50, tip_spacing_mode: 2, tip_coloring_mode: 4,
        dash_count: 0, dash_on: [80, 20, 20], dash_off: [50, 50, 50], min_size: 10, max_count: 14,
        sensitivities: &[(b'P', b'O', 203), (b'P', b'R', 47), (b'P', b'Z', 8), (b'R', b'H', 203), (b'R', b'O', 203), (b'R', b'R', 47), (b'R', b'Z', 398), (b'S', b'O', 500), (b'S', b'R', 8), (b'S', b'Z', 8)],
    },
    BrushPreset {
        category: "bc_Unnatural", category_de: "Unnatürlich", name: "bn_Toothpaste", name_de: "Zahnpasta",
        natural: true, diameter: 20, softness: 500, round: true, antialiased: false, bell: true,
        aspect: 1000, angle: 0, concentration: 1000, blackness: 0, spacing: 130, flow_rate: 0, feedback: 1,
        texture_blend: 0, texture_edge: 0, alpha_remap: 0, tip_count: 3, tip_spacing: 40, tip_spacing_mode: 2, tip_coloring_mode: 2,
        dash_count: 0, dash_on: [100, 20, 20], dash_off: [40, 40, 40], min_size: 10, max_count: 14,
        sensitivities: &[(b'R', b'H', 797), (b'R', b'S', 797)],
    },
    BrushPreset {
        category: "bc_Random", category_de: "Unregelmäßig", name: "bn_Yarn", name_de: "Garn",
        natural: true, diameter: 20, softness: 0, round: true, antialiased: true, bell: true,
        aspect: 1000, angle: 0, concentration: 972, blackness: 0, spacing: 180, flow_rate: 0, feedback: 1,
        texture_blend: 0, texture_edge: 0, alpha_remap: 0, tip_count: 4, tip_spacing: 75, tip_spacing_mode: 2, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [100, 20, 20], dash_off: [100, 100, 100], min_size: 500, max_count: 14,
        sensitivities: &[(b'P', b'Z', 898), (b'R', b'H', 203), (b'R', b'S', 203), (b'S', b'Z', 797)],
    },
    BrushPreset {
        category: "bc_Random", category_de: "Unregelmäßig", name: "bn_Confetti", name_de: "Konfetti",
        natural: true, diameter: 20, softness: 250, round: true, antialiased: false, bell: true,
        aspect: 340, angle: 0, concentration: 866, blackness: 0, spacing: 400, flow_rate: 0, feedback: 0,
        texture_blend: 0, texture_edge: 0, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [40, 10, 10], dash_off: [35, 35, 35], min_size: 10, max_count: 31,
        sensitivities: &[(b'R', b'H', 797), (b'R', b'R', 352)],
    },
    BrushPreset {
        category: "bc_Random", category_de: "Unregelmäßig", name: "bn_Fur", name_de: "Pelz",
        natural: true, diameter: 20, softness: 0, round: true, antialiased: false, bell: true,
        aspect: 180, angle: 0, concentration: 972, blackness: 0, spacing: 300, flow_rate: 600, feedback: 1,
        texture_blend: 0, texture_edge: 0, alpha_remap: 0, tip_count: 4, tip_spacing: 150, tip_spacing_mode: 2, tip_coloring_mode: 3,
        dash_count: 0, dash_on: [200, 40, 40], dash_off: [150, 150, 150], min_size: 200, max_count: 31,
        sensitivities: &[(b'R', b'A', 203), (b'R', b'B', 203), (b'R', b'H', 203), (b'R', b'O', 203), (b'R', b'R', 203)],
    },
    BrushPreset {
        category: "bc_Random", category_de: "Unregelmäßig", name: "bn_Dots", name_de: "Punkte",
        natural: true, diameter: 20, softness: 200, round: false, antialiased: false, bell: true,
        aspect: 1000, angle: 0, concentration: 780, blackness: 0, spacing: 600, flow_rate: 0, feedback: 1,
        texture_blend: 0, texture_edge: 0, alpha_remap: 0, tip_count: 4, tip_spacing: 180, tip_spacing_mode: 2, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [100, 25, 25], dash_off: [300, 300, 300], min_size: 10, max_count: 14,
        sensitivities: &[(b'P', b'R', 352), (b'P', b'Z', 398), (b'R', b'H', 70), (b'R', b'R', 500), (b'S', b'R', 250), (b'S', b'Z', 398)],
    },
    BrushPreset {
        category: "bc_Random", category_de: "Unregelmäßig", name: "bn_Squares", name_de: "Quadrate",
        natural: true, diameter: 20, softness: 0, round: false, antialiased: false, bell: true,
        aspect: 1000, angle: 0, concentration: 850, blackness: 0, spacing: 500, flow_rate: 0, feedback: 1,
        texture_blend: 200, texture_edge: 150, alpha_remap: 0, tip_count: 1, tip_spacing: 0, tip_spacing_mode: 0, tip_coloring_mode: 0,
        dash_count: 0, dash_on: [75, 25, 25], dash_off: [75, 25, 25], min_size: 10, max_count: 14,
        sensitivities: &[(b'P', b'O', 23), (b'P', b'R', 203), (b'R', b'A', 203), (b'R', b'H', 297), (b'R', b'R', 250), (b'S', b'O', 23)],
    },
];

/// Frühere Namen „Kategorie/Art“ bzw. interne Namen → heutige (gespeicherte Werkzeugvorgaben,
/// Skripte, ältere Dokumente).
const PRESET_ALIASES: &[(&str, &str)] = &[
    ("Bleistift/Farbstift", "Bleistift/Buntstift"),
    ("Bleistift/Grafit", "Bleistift/Graphit"),
    ("Einfach/Harte Linie", "Einfach/Präzise Linie"),
    ("Einfach/Hart, abgerundet", "Einfach/Präzise abger."),
    ("Einfach/Weich, abgerundet", "Einfach/Weich abger."),
    ("Airbrush/Strukturiert", "Airbrush/Texturiert"),
    ("Kalligraphie/Bambus", "Kalligrafie/Bambus"),
    ("Kalligraphie/Feder", "Kalligrafie/Federkiel"),
    ("Kohle/Weich", "Kohlestift/Weich"),
    ("Kohle/Pastell", "Kohlestift/Pastell"),
    ("Buntstift/Einfach", "Pastellkreide/Einfach"),
    ("Gestrichelt/Einfach gestrichelt", "Gestrichelt/Einfach gestr."),
    ("Gestrichelt/Doppelt gestrichelt", "Gestrichelt/Doppelt gestr."),
    ("Ölfarbe/Borste", "Öl/Borsten"),
    ("Aquarell/Hell", "Wasserfarbe/Dünn"),
    ("Aquarell/Dick", "Wasserfarbe/Dick"),
    ("bn_1-Pixel Hard", "bn_1-Pixel"),
    ("bn_1-Pixel Soft", "bn_1-Pixel Anti-Aliased"),
];

/// Vorgabe nach „Kategorie/Art“ (Anzeigenamen), internem Namen (`bn_…`) oder Artnamen; frühere
/// Namen über [`PRESET_ALIASES`].
pub fn find_preset(n: &str) -> Option<&'static BrushPreset> {
    let n = PRESET_ALIASES.iter().find(|(old, _)| *old == n).map(|(_, new)| *new).unwrap_or(n);
    BRUSH_PRESETS.iter().find(|b| format!("{}/{}", b.category_de, b.name_de) == n || b.name == n || b.name_de == n)
}

/// Index der Vorgabe zu Kategorie und internem Namen (auch frühere interne Namen).
pub fn preset_index_of(category: &str, name: &str) -> Option<usize> {
    let name = PRESET_ALIASES.iter().find(|(old, _)| *old == name).map(|(_, new)| *new).unwrap_or(name);
    BRUSH_PRESETS.iter().position(|p| p.category == category && p.name == name)
}

/// Kategorien der Strichvorgaben (Anzeigenamen, Reihenfolge des Popups).
pub fn brush_categories() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = Vec::new();
    for b in BRUSH_PRESETS {
        if !v.contains(&b.category_de) {
            v.push(b.category_de);
        }
    }
    v
}

/// Anzeigename eines Pinsels (Vorgabe oder interner Name ohne `bn_`).
pub fn brush_label(b: &Brush) -> String {
    if let Some(p) = preset_index_of(&b.category, &b.name).map(|i| &BRUSH_PRESETS[i]) {
        return p.name_de.to_string();
    }
    if let Some(u) = &b.user_name {
        return u.clone();
    }
    b.name.trim_start_matches("bn_").to_string()
}

fn apply_preset(b: &mut Brush, p: &BrushPreset) {
    // alle Werte der Art, außer der Größe (re\24: „Strichart-Wahl behält die Größe“)
    b.category = p.category.into();
    b.name = p.name.into();
    b.user_name = Some(p.name_de.into());
    b.brush_type = if p.natural { 0 } else { 1 };
    b.softness = p.softness;
    b.shape = i32::from(p.round);
    b.antialiased = p.antialiased;
    b.soften_mode = i32::from(p.bell);
    b.aspect = p.aspect;
    b.angle = p.angle;
    b.concentration = p.concentration;
    b.blackness = p.blackness;
    b.spacing = p.spacing;
    b.flow_rate = p.flow_rate;
    b.feedback = p.feedback;
    b.texture_blend = p.texture_blend;
    b.texture_edge = p.texture_edge;
    b.alpha_remap = p.alpha_remap;
    b.tip_count = p.tip_count;
    b.tip_spacing = p.tip_spacing;
    b.tip_spacing_mode = p.tip_spacing_mode;
    b.tip_coloring_mode = p.tip_coloring_mode;
    b.dash_count = p.dash_count;
    b.dash_on = p.dash_on;
    b.dash_off = p.dash_off;
    b.min_size = p.min_size;
    b.max_count = p.max_count;
    b.sensitivities = p.sensitivities.iter().map(|(s, t, v)| fp_model::attrs::Sensitivity { source: *s, target: *t, value: *v }).collect();
}

/// Felder von „Strich bearbeiten“ (`pi.brush`, `pi.tool.brush`), auf einen Pinsel anwendbar.
#[derive(Clone, Copy, Default)]
struct BrushEdit {
    diameter: Option<i32>,
    softness: Option<i32>,
    angle: Option<i32>,
    aspect: Option<i32>,
    shape: Option<i32>,
    spacing: Option<i32>,
    flow: Option<i32>,
    tips: Option<i32>,
    dashes: Option<i32>,
    on: Option<[i32; 3]>,
    off: Option<[i32; 3]>,
}

impl BrushEdit {
    fn parse(p: &Params) -> Result<Self> {
        let int = |k: &str| p.opt_i64(k).map(|v| v.map(|x| x as i32));
        let triple = |k: &str| -> Result<Option<[i32; 3]>> {
            let Some(v) = p.get(k) else { return Ok(None) };
            let a: Vec<i64> = serde_json::from_value(v.clone()).map_err(|e| bad(p, format!("{k}: {e}")))?;
            let mut out = [0; 3];
            for (o, x) in out.iter_mut().zip(a) {
                *o = x.clamp(0, 1000) as i32;
            }
            Ok(Some(out))
        };
        Ok(BrushEdit {
            diameter: int("diameter")?,
            softness: int("softness")?,
            angle: int("angle")?,
            aspect: int("aspect")?,
            shape: int("shape")?,
            spacing: int("spacing")?,
            flow: int("flowRate")?,
            tips: int("tipCount")?,
            dashes: int("dashCount")?,
            on: triple("dashOn")?,
            off: triple("dashOff")?,
        })
    }

    fn apply(&self, b: &mut Brush) {
        if let Some(v) = self.diameter {
            b.diameter = v.max(1);
        }
        if let Some(v) = self.softness {
            b.softness = v.clamp(0, 100) * 10;
        }
        if let Some(v) = self.angle {
            b.angle = v.rem_euclid(360);
        }
        if let Some(v) = self.aspect {
            b.aspect = v.clamp(1, 100) * 10;
        }
        if let Some(v) = self.shape {
            b.shape = v.clamp(0, 1);
        }
        if let Some(v) = self.spacing {
            b.spacing = v.clamp(0, 1000) * 10;
        }
        if let Some(v) = self.flow {
            b.flow_rate = v.clamp(0, 100) * 10;
        }
        if let Some(v) = self.tips {
            b.tip_count = v.clamp(1, 32);
        }
        if let Some(v) = self.dashes {
            b.dash_count = v.clamp(0, 3);
        }
        if let Some(v) = self.on {
            b.dash_on = v;
        }
        if let Some(v) = self.off {
            b.dash_off = v;
        }
    }
}

/// Vollständiger Pinsel aus dem Parameter `brush` (Feldnamen wie im Modell).
fn full_brush(p: &Params) -> Result<Option<Brush>> {
    match p.get("brush") {
        Some(v) => serde_json::from_value::<Brush>(v.clone()).map(|mut b| {
            b.diameter = b.diameter.clamp(1, 1000);
            Some(b)
        }).map_err(|e| bad(p, fp_i18n::tr!("err-brush-detail", detail = e.to_string()))),
        None => Ok(None),
    }
}

/// Strichart der Werksvorgabe für neue Linien, Stiftpfade und Formen wie im Original:
/// „Einfach/Präzise abger.“ (`bc_Basic`/`bn_Hard Rounded`, tool-line-pen Frage 1, tool-shapes 12).
pub const DEFAULT_TOOL_PRESET: (&str, &str) = ("bc_Basic", "bn_Hard Rounded");

/// Pinsel der Werksvorgabe ([`DEFAULT_TOOL_PRESET`]) in der Größe `width`.
pub fn default_tool_brush(width: i32) -> Brush {
    let mut b = Brush::basic(width.max(1));
    if let Some(p) = BRUSH_PRESETS.iter().find(|p| (p.category, p.name) == DEFAULT_TOOL_PRESET) {
        apply_preset(&mut b, p);
    }
    b
}

/// Pinsel der Werkzeugvorgabe: `new_brush`, sonst aus der Strichvorgabe `new_stroke`
/// (Vorgabe, Stärke, Weichheit) gebildet, sonst die Werksvorgabe „Präzise abger.“ 1 px.
pub fn tool_brush(o: &crate::tools::ToolOptions) -> Brush {
    tool_brush_from(o.new_stroke.as_ref(), o.new_brush.as_ref())
}

/// Wie [`tool_brush`] aus Strichvorgabe (`pi.stroke`-Parameter) und Vorgabepinsel.
pub fn tool_brush_from(new_stroke: Option<&Value>, new_brush: Option<&Value>) -> Brush {
    if let Some(b) = new_brush.and_then(|v| serde_json::from_value::<Brush>(v.clone()).ok()) {
        return b;
    }
    let Some(st) = new_stroke else {
        return default_tool_brush(1);
    };
    let st = Some(st);
    let get = |k: &str| st.and_then(|v| v.get(k));
    let preset = get("preset").and_then(|v| v.as_str()).and_then(find_preset);
    let width = get("width").and_then(|v| v.as_f64()).map(|w| w.round().clamp(1.0, 1000.0) as i32);
    let mut b = Brush::basic(width.or(preset.map(|p| p.diameter)).unwrap_or(1).max(1));
    if let Some(pr) = preset {
        apply_preset(&mut b, pr);
    }
    if let Some(sf) = get("softness").and_then(|v| v.as_i64()) {
        b.softness = (sf.clamp(0, 100) * 10) as i32;
    }
    b
}

/// Aktuelle Breite und Höhe einer Rechteck-Grundform in Dokumentpixeln (Rechteck durch die Matrix).
pub fn primitive_size(pr: &fp_model::RectPrimitive) -> (f32, f32) {
    let m = pr.transform;
    let r = pr.rect;
    let tl = m.apply(Point::new(r.left, r.top));
    let tr = m.apply(Point::new(r.right, r.top));
    let bl = m.apply(Point::new(r.left, r.bottom));
    let d = |a: Point, b: Point| ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt();
    (d(tl, tr), d(tl, bl))
}

/// Rundung einer Rechteck-Grundform in Pixeln (halbe kürzere Seite × Anteil).
pub fn primitive_roundness_px(pr: &fp_model::RectPrimitive) -> f32 {
    let (w, h) = primitive_size(pr);
    pr.roundness * w.min(h) / 2.0
}

// ---------------------------------------------------------------------- Effekte

/// Effektliste eines Elements setzen (Rechteck-Grundformen: Kopie in `PRIv/EFLv`, Befund fp-render).
pub fn set_effects(e: &mut Element, l: Option<EffectList>) {
    let l = l.filter(|x| !x.effects.is_empty());
    if let ElementKind::Path(p) = &mut e.kind
        && let Some(pr) = &mut p.primitive
    {
        pr.effects = l.clone();
    }
    e.common.effects = l;
}

/// Wert eines Effektparameters für `pi.effect.set`: Text wie im Dateiformat bzw. eine Punktliste
/// (Kurven-Effekt, KV-3), wenn ein Feld aus Punkten `[x, y]` oder `{x, y}` kommt.
pub(crate) fn effect_param_value(v: &Value) -> fp_model::DataValue {
    let point = |q: &Value| -> Option<fp_geom::Point> {
        let (x, y) = match q {
            Value::Array(a) if a.len() == 2 => (a[0].as_f64()?, a[1].as_f64()?),
            Value::Object(o) => (o.get("x")?.as_f64()?, o.get("y")?.as_f64()?),
            _ => return None,
        };
        Some(fp_geom::Point::new(x as f32, y as f32))
    };
    if let Value::Array(a) = v
        && let Some(points) = a.iter().map(point).collect::<Option<Vec<_>>>()
    {
        return fp_model::DataValue::Points(fp_model::PointList { points, ..Default::default() });
    }
    fp_model::DataValue::Scalar(v.as_str().map(|x| x.to_string()).unwrap_or_else(|| v.to_string()))
}

fn edit_effects(s: &mut Session, p: &Params, f: impl Fn(&mut EffectList) -> Result<()>) -> Result<CommandOutput> {
    let (page, ids) = targets(s, p)?;
    let d = s.doc_mut()?;
    let doc = Arc::make_mut(&mut d.doc);
    for id in ids {
        let mut err = None;
        docops::edit_by_id(doc, page, id, |e| {
            let mut l = e.common.effects.clone().unwrap_or_default();
            match f(&mut l) {
                Ok(()) => set_effects(e, Some(l)),
                Err(x) => err = Some(x),
            }
        })
        .ok_or(EngineError::ElementNotFound(id.0))?;
        if let Some(x) = err {
            return Err(x);
        }
    }
    Ok(CommandOutput::doc())
}

/// Effekt aus `pi.effects.replace`: MOA-ID, sichtbar, Parameter.
type EffectSpec = (String, bool, Vec<(String, String)>);

// ---------------------------------------------------------------------- Pfadoperationen

#[cfg(feature = "vector")]
fn to_v(c: &[fp_model::Contour]) -> Vec<fp_vector::pathops::Contour> {
    c.iter().map(|k| fp_vector::pathops::Contour::from_bezier(&crate::geometry::to_bezier(k))).collect()
}

#[cfg(feature = "vector")]
fn from_v(c: &[fp_vector::pathops::Contour]) -> Vec<fp_model::Contour> {
    c.iter().map(|k| crate::geometry::from_bezier(&k.to_bezier())).collect()
}

/// Konturen und Füllregel eines Elements (Pfad, Ergebnis einer Compound Shape).
#[cfg(feature = "vector")]
fn shape_of(e: &Element) -> Option<(Vec<fp_model::Contour>, bool)> {
    match &e.kind {
        ElementKind::Path(p) => Some((p.contours.clone(), p.even_odd)),
        ElementKind::CompoundShape(c) => c.result.as_ref().and_then(|r| shape_of(r)),
        _ => None,
    }
}

/// Ergebnis einer Compound Shape neu berechnen (16 §2.1).
pub fn recompute_compound(e: &mut Element) {
    let ElementKind::CompoundShape(c) = &mut e.kind else { return };
    #[cfg(not(feature = "vector"))]
    {
        let _ = c;
        e.cache = None;
    }
    #[cfg(feature = "vector")]
    {
    let ops: Vec<fp_vector::pathops::Operand> = c
        .operands
        .iter()
        .filter_map(|o| {
            let (cs, eo) = shape_of(o)?;
            let op = match &o.kind {
                ElementKind::Path(p) => p.path_op,
                _ => 1,
            };
            Some(fp_vector::pathops::Operand::new(to_v(&cs), eo, fp_vector::pathops::PathOp::from_u32(op)))
        })
        .collect();
    let res = from_v(&fp_vector::pathops::evaluate_compound(&ops));
    let attrs = c.operands.last().and_then(|o| docops::path_attrs_of(o)).map(|a| (*a).clone()).unwrap_or_default();
    match &mut c.result {
        Some(r) => {
            let r = Element::edit(r);
            if let ElementKind::Path(p) = &mut r.kind {
                // Füll-Handles/Texturursprung, die noch zur alten Form gehörten, mitführen
                // (sonst hängt das Ergebnis von der Vorgeschichte ab, §4b)
                let old_b = fp_model::new::contour_bounds(&p.contours);
                let new_b = fp_model::new::contour_bounds(&res);
                if p.fill_handles == fp_model::new::default_fill_handles(old_b) {
                    p.fill_handles = fp_model::new::default_fill_handles(new_b);
                }
                if p.texture_offset == fp_geom::Point::new(old_b.left, old_b.top) {
                    p.texture_offset = fp_geom::Point::new(new_b.left, new_b.top);
                }
                p.contours = res;
                p.polyline = None;
                p.even_odd = true;
            }
            r.cache = None;
        }
        None => {
            let mut ids = fp_model::IdGen::default();
            let mut r = Element::new_path(&mut ids, res, attrs);
            r.id = ElementId(0);
            if let ElementKind::Path(p) = &mut r.kind {
                p.even_odd = true;
            }
            c.result = Some(Arc::new(r));
        }
    }
    e.cache = None;
    }
}

/// Verknüpfungen der Operanden einer neuen Compound Shape (16 §2.1).
fn compound_ops(op: u32, n: usize) -> Vec<u32> {
    #[cfg(feature = "vector")]
    {
        fp_vector::pathops::create_compound_ops(fp_vector::pathops::PathOp::from_u32(op), n).into_iter().map(|x| x as u32).collect()
    }
    #[cfg(not(feature = "vector"))]
    {
        let mut v = vec![op; n];
        if let Some(l) = v.last_mut() {
            *l = 0;
        }
        v
    }
}

// ---------------------------------------------------------------------- Symbole umnummerieren

fn remap_element(e: &mut Element, map: &std::collections::HashMap<u32, u32>) {
    match &mut e.kind {
        ElementKind::Instance(i) => {
            if let Some(n) = map.get(&i.symbol_id.0) {
                i.symbol_id = fp_model::SymbolId(*n);
            }
        }
        ElementKind::Group(g) => {
            for c in &mut g.children {
                remap_element(Element::edit(c), map);
            }
            if let Some(m) = &mut g.mask {
                remap_element(Element::edit(m), map);
            }
        }
        ElementKind::CompoundShape(c) => {
            for o in &mut c.operands {
                remap_element(Element::edit(o), map);
            }
        }
        ElementKind::SubLayer(l) => remap_layer(Arc::make_mut(l), map),
        _ => {}
    }
}

fn remap_layer(l: &mut fp_model::Layer, map: &std::collections::HashMap<u32, u32>) {
    for c in &mut l.cels {
        for e in &mut Arc::make_mut(c).elements {
            remap_element(Element::edit(e), map);
        }
    }
}

// ---------------------------------------------------------------------- Registrierung

/// Trägt alle Adapter-Befehle ein.
pub fn register(r: &mut CommandRegistry) {
    let ids = |b: crate::command::CommandBuilder| b.opt("ids", ParamKind::Ids, "Elemente (Vorgabe: Auswahl)");

    // Füllung (Teilfeld 14011, Fülltextur 14013, Verlaufs-Popup 5980)
    r.add(
        ids(cmd("pi.fill").label("Füllen").requires(DOC))
            .opt("category", ParamKind::Enum { values: ["none", "solid", "gradient", "pattern", "webDither"].iter().map(|s| s.to_string()).collect() }, "Füllart (pattern: Vorgabemuster „Cotton“, wenn noch kein Muster)")
            .opt("color", ParamKind::Color, "Füllfarbe")
            .opt("shape", ParamKind::Integer { min: Some(2), max: Some(17) }, "Verlaufsform (FSHi; 17 = Konturverlauf)")
            .opt("angle", ParamKind::Number { min: Some(-360.0), max: Some(720.0) }, "Winkel des Verlaufs in Grad (0 = nach rechts, 270 = nach unten)")
            .opt("stops", ParamKind::Json, "Farbstopps [[Position 0..1, \"#rrggbbaa\"], …]")
            .opt("opacityStops", ParamKind::Json, "Deckkraftstopps [[Position, \"#ffffffaa\"], …]")
            .opt("reverse", ParamKind::Bool, "Verlauf umkehren")
            .opt("edge", ParamKind::Integer { min: Some(0), max: Some(2) }, "Kante: 0 hart, 1 Anti-Alias, 2 weiche Kante")
            .opt("feather", ParamKind::Integer { min: Some(0), max: Some(100) }, "Breite der weichen Kante")
            .opt("texture", ParamKind::String, "Texturname (leer = keine)")
            .opt("textureAmount", ParamKind::Integer { min: Some(0), max: Some(100) }, "Texturmenge in %")
            .opt("textureTransparent", ParamKind::Bool, "transparente Textur")
            .opt("dither", ParamKind::Bool, "Verlauf mit Dither")
            .opt("pattern", ParamKind::String, "Musterfüllung nach Name (Datei aus den Musterordnern)")
            .run(|s, p| {
                let cat = p.opt_str("category")?;
                // Muster nur per Name: Datei aus den Musterordnern laden (`crate::resources`);
                // „Muster“ ohne Namen = Vorgabemuster des Originals (nur, wenn noch kein Muster da ist)
                let pat_name = match p.opt_str("pattern")? {
                    Some(n) if !n.is_empty() => Some(n),
                    _ if cat.as_deref() == Some("pattern") => Some(DEFAULT_PATTERN.to_string()),
                    _ => None,
                };
                let pattern = match pat_name {
                    // „Andere…“ als Werkzeugvorgabe: voller Pfad einer Bilddatei
                    Some(n) if std::path::Path::new(&n).is_absolute() => {
                        let path = std::path::PathBuf::from(&n);
                        let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                        Some(crate::resources::pattern_from_file(&path, &stem).ok_or_else(|| EngineError::InvalidParams { name: p.command.clone(), message: fp_i18n::tr!("err-not-found", what = n.as_str()) })?)
                    }
                    Some(n) => Some(crate::resources::load_pattern(&n).ok_or_else(|| EngineError::InvalidParams { name: p.command.clone(), message: fp_i18n::tr!("err-not-found", what = n.as_str()) })?),
                    None => None,
                };
                let keep_pattern = cat.as_deref() == Some("pattern") && p.opt_str("pattern")?.is_none_or(|n| n.is_empty());
                let color = p.opt_color("color")?;
                let shape = p.opt_i64("shape")?.map(|v| v as i32);
                let angle = p.opt_f64("angle")?;
                let stops = parse_stops(p.get("stops"));
                let ostops = parse_stops(p.get("opacityStops"));
                let reverse = p.opt_bool("reverse")?.unwrap_or(false);
                let edge = p.opt_i64("edge")?.map(|v| v as i32);
                let feather = p.opt_i64("feather")?.map(|v| v as i32);
                let tex = p.opt_str("texture")?;
                check_texture_name(p, tex.as_deref())?;
                let tex_amount = p.opt_i64("textureAmount")?.map(|v| v as i32);
                let tex_tr = p.opt_bool("textureTransparent")?;
                let dither = p.opt_bool("dither")?;
                let fallback_fill = s.tools.fill_color;
                edit_each_changed(s, p, |e, bounds| {
                    let (attrs0, handles0) = (docops::path_attrs_of(e), handles_of(e));
                    let before = docops::path_attrs_of(e).map(|a| fill_kind(a.fill.as_ref())).unwrap_or(0);
                    let before_shape = docops::path_attrs_of(e).and_then(|a| a.fill.as_ref().map(shape_id));
                    let before_handles = handles_of(e);
                    // „Muster“ an einer Musterfüllung: Muster behalten (nur das Popup öffnet sich)
                    let pattern = if keep_pattern && before == 3 { None } else { pattern.clone() };
                    docops::edit_path_attrs(e, &|a| {
                        match cat.as_deref() {
                            Some("none") => {
                                a.fill = None;
                                return;
                            }
                            Some("solid") => {
                                let mut f = a.fill.clone().filter(|f| f.shape == 0).unwrap_or_else(Fill::solid);
                                f.shape = 0;
                                f.shape_ext = 0;
                                f.gradient = None;
                                f.pattern = None;
                                f.category = "fc_Solid".into();
                                f.name = "Solid".into();
                                f.user_name = Some("Solid".into());
                                a.fill = Some(f);
                            }
                            Some("webDither") => {
                                let mut f = a.fill.clone().unwrap_or_else(Fill::solid);
                                f.shape = 16;
                                f.gradient = None;
                                f.category = "fc_Web Dither".into();
                                f.name = "Web Dither".into();
                                let c = a.fill_color.0;
                                f.dither_colors = Some([c, c, c, c]);
                                a.fill = Some(f);
                            }
                            Some("gradient") => {
                                let is_grad = a.fill.as_ref().is_some_and(|f| f.gradient.is_some() && f.shape != 0);
                                if !is_grad {
                                    let mut f = a.fill.clone().unwrap_or_else(Fill::solid);
                                    let sh = shape.unwrap_or(2);
                                    let base = if a.fill.is_some() { a.fill_color } else { fallback_fill };
                                    // wie im Original: Weiß → Füllfarbe (bei weißer Füllfarbe Weiß → Schwarz)
                                    let base = Argb(base.0 | 0xff00_0000);
                                    let end = if base == Argb(0xffff_ffff) { Argb(0xff00_0000) } else { base };
                                    let g = make_gradient(sh, &[(0.0, Argb(0xffff_ffff)), (1.0, end)], None);
                                    f.category = g.category.clone();
                                    f.name = g.name.clone();
                                    f.user_name = Some(g.name.clone());
                                    set_shape_id(&mut f, sh);
                                    f.pattern = None;
                                    f.gradient = Some(g);
                                    a.fill = Some(f);
                                }
                            }
                            _ => {}
                        }
                        if let Some(pat) = &pattern {
                            let mut f = a.fill.clone().unwrap_or_else(Fill::solid);
                            f.category = "fc_Pattern".into();
                            f.name = pat.name.clone();
                            f.user_name = Some(pat.name.clone());
                            f.shape = 15;
                            f.shape_ext = 0;
                            f.gradient = None;
                            f.pattern = Some(pat.clone());
                            a.fill = Some(f);
                        }
                        if let Some(c) = color {
                            a.fill_color = c;
                            if a.fill.is_none() && cat.is_none() {
                                a.fill = Some(Fill::solid());
                            }
                        }
                        let Some(f) = a.fill.as_mut() else { return };
                        if let Some(sh) = shape
                            && f.gradient.is_some()
                        {
                            set_shape_id(f, sh);
                            if let Some((_, _, c, n)) = GRADIENT_SHAPES.iter().find(|g| g.1 == sh) {
                                f.category = (*c).into();
                                f.name = (*n).into();
                                f.user_name = Some((*n).into());
                                if let Some(g) = &mut f.gradient {
                                    g.category = (*c).into();
                                    g.name = (*n).into();
                                }
                            }
                        }
                        if let Some(g) = &mut f.gradient {
                            if let Some(st) = &stops {
                                let opac = ostops.clone();
                                let ng = make_gradient(f.shape, st, opac.as_deref());
                                g.stops = ng.stops;
                                g.legacy_stops = None;
                            } else if let Some(os) = &ostops {
                                let cs: Vec<(f32, Argb)> = g.color_stops().iter().map(|s| (s.position, s.color)).collect();
                                let ng = make_gradient(f.shape, &cs, Some(os));
                                g.stops = ng.stops;
                                g.legacy_stops = None;
                            }
                            if reverse {
                                let cs: Vec<(f32, Argb)> = g.color_stops().iter().map(|s| (1.0 - s.position, s.color)).collect();
                                let os: Vec<(f32, Argb)> = g.opacity_stops().iter().map(|s| (1.0 - s.position, s.color)).collect();
                                let mut cs = cs;
                                cs.sort_by(|a, b| a.0.total_cmp(&b.0));
                                let mut os = os;
                                os.sort_by(|a, b| a.0.total_cmp(&b.0));
                                let ng = make_gradient(f.shape, &cs, if os.is_empty() { None } else { Some(&os) });
                                g.stops = ng.stops;
                                g.legacy_stops = None;
                            }
                            if let Some(dt) = dither
                                && let Some(st) = &mut g.stops
                            {
                                st.dither = Some(dt);
                            }
                        }
                        if let Some(ed) = edge {
                            f.edge = ed;
                            // Hart/Anti-Alias: die Breite der weichen Kante gilt nicht mehr (PAR 18/11;
                            // das Original setzt sie beim Wechsel zurück)
                            if ed != 2 {
                                f.feather = 0;
                            }
                        }
                        if let Some(fe) = feather
                            && edge.is_none_or(|ed| ed == 2)
                        {
                            f.feather = fe;
                            if fe > 0 && edge.is_none() {
                                f.edge = 2;
                            }
                        }
                        if let Some(t) = &tex {
                            a.fill_texture = if t.is_empty() { None } else { Some(texture(t)) };
                        }
                        if let Some(tm) = tex_amount
                            && let Some(f) = a.fill.as_mut()
                        {
                            f.texture_amount = tm * 10;
                        }
                        if let Some(tt) = tex_tr
                            && let Some(f) = a.fill.as_mut()
                        {
                            f.transparent_dither = Some(tt);
                        }
                    });
                    let after = docops::path_attrs_of(e).map(|a| fill_kind(a.fill.as_ref())).unwrap_or(0);
                    if after == 2 || after == 3 {
                        // neu auf Verlauf/Muster umgestellt: Vorgabegriffe des Originals; sonst nur
                        // ungültige bzw. entartete Griffe ersetzen (Objekte aus Dateien, PAR 18/03)
                        let sh = docops::path_attrs_of(e).and_then(|a| a.fill.as_ref().map(shape_id)).unwrap_or(2);
                        // andere Verlaufsart: unveränderte Vorgabegriffe der alten Art durch die der
                        // neuen ersetzen (Radial & Co. um die Mitte, PAR 18/03b); vom Nutzer
                        // verschobene oder gedrehte Griffe bleiben
                        let untouched = after == 2
                            && before == 2
                            && before_shape.is_some_and(|o| o != sh && before_handles == Some(default_handles(bounds, o)));
                        let force = after != before || untouched;
                        let h = if after == 3 { bounds.map(fp_model::new::default_fill_handles).unwrap_or_else(|| default_handles(None, 2)) } else { default_handles(bounds, sh) };
                        set_handles(e, h, force);
                    }
                    if let Some(deg) = angle
                        && (after == 2 || after == 3)
                    {
                        rotate_element_handles(e, deg.rem_euclid(360.0));
                    }
                    // nur bei echter Änderung ein Verlaufsschritt (gleiche Art erneut gewählt: keiner)
                    docops::path_attrs_of(e).as_deref() != attrs0.as_deref() || handles_of(e) != handles0
                })
            }),
    );

    // Strich (Teilfeld 14010, Strichtextur 14103, Popups 4116/4118)
    r.add(
        ids(cmd("pi.stroke").label("Strich ändern").requires(DOC))
            .opt("none", ParamKind::Bool, "kein Strich")
            .opt("color", ParamKind::Color, "Strichfarbe")
            .opt("width", ParamKind::Number { min: Some(1.0), max: Some(1000.0) }, "Pinselstärke")
            .opt("preset", ParamKind::String, "Vorgabe „Kategorie/Name“ (Anzeigenamen) oder interner Name")
            .opt("softness", ParamKind::Integer { min: Some(0), max: Some(100) }, "Kantenweichheit")
            .opt("placement", ParamKind::Enum { values: vec!["inside".into(), "center".into(), "outside".into()] }, "Strichposition")
            .opt("texture", ParamKind::String, "Texturname (leer = keine)")
            .opt("textureAmount", ParamKind::Integer { min: Some(0), max: Some(100) }, "Texturmenge in %")
            .opt("fillOverStroke", ParamKind::Bool, "Füllung über Strich")
            .opt("antialiased", ParamKind::Bool, "Kantenglättung")
            .run(|s, p| {
                let none = p.opt_bool("none")?.unwrap_or(false);
                let color = p.opt_color("color")?;
                let width = p.opt_f64("width")?;
                let preset_name = p.opt_str("preset")?;
                // gespeicherter Pinselstrich des Dokuments („Pinselstrich speichern“)
                let saved: Option<Brush> = preset_name.as_ref().and_then(|n| {
                    let d = s.active()?;
                    d.doc.pages.get(d.page)?.presets.brushes.as_ref()?.iter().find(|b| b.user_name.as_deref() == Some(n.as_str()) || format!("Gespeichert/{}", b.user_name.clone().unwrap_or_default()) == *n).cloned()
                });
                let preset = match &preset_name {
                    Some(_) if saved.is_some() => None,
                    Some(n) => Some(*find_preset(n).ok_or_else(|| bad(p, fp_i18n::tr!("err-unknown-stroke-preset", name = n.as_str())))?),
                    None => None,
                };
                let soft = p.opt_i64("softness")?.map(|v| v as i32);
                let place = p.opt_str("placement")?.map(|v| match v.as_str() {
                    "inside" => 0,
                    "outside" => 2,
                    _ => 1,
                });
                let tex = p.opt_str("texture")?;
                check_texture_name(p, tex.as_deref())?;
                let tex_amount = p.opt_i64("textureAmount")?.map(|v| v as i32);
                let fos = p.opt_bool("fillOverStroke")?;
                let aa = p.opt_bool("antialiased")?;
                edit_each(s, p, |e, _| {
                    match &mut e.kind {
                        ElementKind::Path(pe) => {
                            if let Some(pl) = place {
                                pe.stroke_placement = pl;
                            }
                            if let Some(v) = fos {
                                pe.fill_on_top = v;
                            }
                        }
                        ElementKind::Text(t) => {
                            if let Some(pl) = place {
                                t.stroke_placement = pl;
                            }
                            if let Some(v) = fos {
                                t.fill_on_top = v;
                            }
                        }
                        _ => {}
                    }
                    docops::edit_path_attrs(e, &|a| {
                        if none {
                            a.brush = None;
                            return;
                        }
                        if let Some(c) = color {
                            a.stroke_color = c;
                        }
                        let creating = a.brush.is_none() && saved.is_none() && (preset.is_some() || width.is_some() || color.is_some());
                        if creating {
                            a.brush = Some(Brush::basic(width.map(|w| w.round() as i32).unwrap_or(1).max(1)));
                        }
                        if let Some(sb) = &saved {
                            let keep = a.brush.as_ref().map(|b| b.diameter);
                            let mut nb = sb.clone();
                            if let (Some(k), None) = (keep, width) {
                                nb.diameter = k;
                            }
                            a.brush = Some(nb);
                        }
                        let Some(b) = a.brush.as_mut() else { return };
                        if let Some(pr) = &preset {
                            apply_preset(b, pr);
                            if width.is_none() && creating {
                                b.diameter = pr.diameter;
                            }
                        }
                        if let Some(w) = width {
                            b.diameter = w.round().max(1.0) as i32;
                        }
                        if let Some(sf) = soft {
                            b.softness = sf.clamp(0, 100) * 10;
                        }
                        if let Some(v) = aa {
                            b.antialiased = v;
                        }
                        if let Some(t) = &tex {
                            a.stroke_texture = if t.is_empty() { None } else { Some(texture(t)) };
                        }
                        if let Some(tm) = tex_amount
                            && let Some(b) = a.brush.as_mut()
                        {
                            b.texture_blend = tm * 10;
                        }
                    });
                })
            }),
    );

    // Strich bearbeiten (5963–5965): Pinselspitze und Strichmuster des vorhandenen Pinsels
    r.add(
        ids(cmd("pi.brush").label("Strich bearbeiten").requires(DOC))
            .opt("diameter", ParamKind::Integer { min: Some(1), max: Some(1000) }, "Spitzengröße in px")
            .opt("softness", ParamKind::Integer { min: Some(0), max: Some(100) }, "Kantenweichheit in %")
            .opt("angle", ParamKind::Integer { min: Some(0), max: Some(359) }, "Winkel")
            .opt("aspect", ParamKind::Integer { min: Some(1), max: Some(100) }, "Seitenverhältnis in %")
            .opt("shape", ParamKind::Integer { min: Some(0), max: Some(1) }, "Spitzenform 0 eckig, 1 rund")
            .opt("spacing", ParamKind::Integer { min: Some(0), max: Some(1000) }, "Abstand in %")
            .opt("flowRate", ParamKind::Integer { min: Some(0), max: Some(100) }, "Fluss in %")
            .opt("tipCount", ParamKind::Integer { min: Some(1), max: Some(32) }, "Anzahl Spitzen")
            .opt("dashCount", ParamKind::Integer { min: Some(0), max: Some(3) }, "Anzahl Strich/Lücke-Paare")
            .opt("dashOn", ParamKind::Json, "Strichlängen [a, b, c]")
            .opt("dashOff", ParamKind::Json, "Lückenlängen [a, b, c]")
            .opt("brush", ParamKind::Json, "vollständiger Pinsel wie `element.brush` (Dialog „Pinselstrich bearbeiten“); die übrigen Felder wirken danach")
            .run(|s, p| {
                let ed = BrushEdit::parse(p)?;
                let full = full_brush(p)?;
                edit_each(s, p, |e, _| {
                    docops::edit_path_attrs(e, &|a| {
                        if let (Some(f), Some(_)) = (&full, &a.brush) {
                            a.brush = Some(f.clone());
                        }
                        if let Some(b) = a.brush.as_mut() {
                            ed.apply(b);
                        }
                    });
                })
            }),
    );

    // Pinsel der Werkzeugvorgabe (Zeichenwerkzeuge ohne Auswahl): „Strich bearbeiten“ und die
    // Strich-Felder des Inspektors schreiben den vollständigen Pinsel nach `ToolOptions::new_brush`,
    // den `tool.shape` auf neue Objekte legt
    r.add(
        cmd("pi.tool.brush")
            .label("Strich bearbeiten (Werkzeugvorgabe)")
            .opt("clear", ParamKind::Bool, "Vorgabepinsel entfernen (Strich wieder aus der Strichvorgabe)")
            .opt("preset", ParamKind::String, "Strichvorgabe „Kategorie/Name“ (ersetzt die Pinselart)")
            .opt("width", ParamKind::Number { min: Some(1.0), max: Some(1000.0) }, "Pinselstärke")
            .opt("diameter", ParamKind::Integer { min: Some(1), max: Some(1000) }, "Spitzengröße in px")
            .opt("softness", ParamKind::Integer { min: Some(0), max: Some(100) }, "Kantenweichheit in %")
            .opt("angle", ParamKind::Integer { min: Some(0), max: Some(359) }, "Winkel")
            .opt("aspect", ParamKind::Integer { min: Some(1), max: Some(100) }, "Seitenverhältnis in %")
            .opt("shape", ParamKind::Integer { min: Some(0), max: Some(1) }, "Spitzenform 0 eckig, 1 rund")
            .opt("spacing", ParamKind::Integer { min: Some(0), max: Some(1000) }, "Abstand in %")
            .opt("flowRate", ParamKind::Integer { min: Some(0), max: Some(100) }, "Fluss in %")
            .opt("tipCount", ParamKind::Integer { min: Some(1), max: Some(32) }, "Anzahl Spitzen")
            .opt("dashCount", ParamKind::Integer { min: Some(0), max: Some(3) }, "Anzahl Strich/Lücke-Paare")
            .opt("dashOn", ParamKind::Json, "Strichlängen [a, b, c]")
            .opt("dashOff", ParamKind::Json, "Lückenlängen [a, b, c]")
            .opt("sensitivity", ParamKind::Json, "Empfindlichkeiten {\"PZ\": %, …} wie element.setBrushSensitivity")
            .opt("brush", ParamKind::Json, "vollständiger Pinsel wie `element.brush` (Dialog „Pinselstrich bearbeiten“); die übrigen Felder wirken danach")
            .run(|s, p| {
                if p.opt_bool("clear")?.unwrap_or(false) {
                    s.tools.options.new_brush = None;
                    return Ok(CommandOutput::session_with(json!({ "brush": null })));
                }
                let mut b = full_brush(p)?.unwrap_or_else(|| tool_brush(&s.tools.options));
                if let Some(n) = p.opt_str("preset")? {
                    let pr = *find_preset(&n).ok_or_else(|| bad(p, fp_i18n::tr!("err-unknown-stroke-preset", name = n.as_str())))?;
                    let keep = b.diameter;
                    b = Brush::basic(keep.max(1));
                    apply_preset(&mut b, &pr);
                }
                if let Some(w) = p.opt_f64("width")? {
                    b.diameter = w.round().clamp(1.0, 1000.0) as i32;
                }
                BrushEdit::parse(p)?.apply(&mut b);
                if let Some(Value::Object(m)) = p.get("sensitivity") {
                    for (k, v) in m {
                        let kb = k.as_bytes();
                        if kb.len() != 2 || !fp_model::attrs::SENSITIVITY_SOURCES.contains(&kb[0]) || !fp_model::attrs::SENSITIVITY_TARGETS.contains(&kb[1]) {
                            return Err(bad(p, fp_i18n::tr!("err-invalid-key", name = k.as_str())));
                        }
                        let pct = v.as_f64().ok_or_else(|| bad(p, fp_i18n::tr!("err-param-number-expected", key = k.as_str())))?;
                        let val = (pct.clamp(0.0, 100.0) * 10.0).round() as i32;
                        b.sensitivities.retain(|x| !(x.source == kb[0] && x.target == kb[1]));
                        if val > 0 {
                            b.sensitivities.push(fp_model::attrs::Sensitivity { source: kb[0], target: kb[1], value: val });
                        }
                    }
                }
                let v = serde_json::to_value(&b).map_err(|e| EngineError::Failed(fp_i18n::tr!("err-brush-detail", detail = e.to_string())))?;
                s.tools.options.new_brush = Some(v.clone());
                Ok(CommandOutput::session_with(json!({ "brush": v })))
            }),
    );

    // Pinselstrich speichern/löschen (Zeile „Pinselstrich bearbeiten“): Vorgaben im Dokument
    r.add(cmd("pi.brush.save").label("Pinselstrich speichern").requires(DOC).param("name", ParamKind::String, "Name").opt("tool", ParamKind::Bool, "Pinsel der Werkzeugvorgabe speichern (Werkzeug-Inspektor) statt des Strichs der Auswahl").run(|s, p| {
        let name = p.str("name")?.trim().to_string();
        if name.is_empty() {
            return Err(bad(p, fp_i18n::t("err-param-name-missing")));
        }
        // Werkzeug-Inspektor (Pinsel, Zeichenwerkzeuge): der Symbolknopf ist im Original auch ohne
        // Auswahl aktiv und speichert die Werkzeugvorgabe (PAR 02/15)
        let tool = p.opt_bool("tool")?.unwrap_or(false).then(|| tool_brush(&s.tools.options));
        let d = s.doc_mut()?;
        let page = d.page;
        let b = match tool {
            Some(b) => b,
            None => d.selection.elements.iter().filter_map(|id| docops::element_on_page(&d.doc, page, *id)).find_map(|e| docops::path_attrs_of(&e).and_then(|a| a.brush.clone())).ok_or_else(|| EngineError::Selection(fp_i18n::t("err-sel-no-stroked-object")))?,
        };
        let mut nb = b;
        nb.user_name = Some(name.clone());
        let v = page_mut(d)?.presets.brushes.get_or_insert_with(Vec::new);
        match v.iter().position(|x| x.user_name.as_deref() == Some(name.as_str())) {
            Some(i) => v[i] = nb,
            None => v.push(nb),
        }
        Ok(CommandOutput::doc())
    }));
    r.add(cmd("pi.brush.delete").label("Pinselstrich löschen").requires(DOC).param("name", ParamKind::String, "Name").run(|s, p| {
        let name = p.str("name")?;
        let d = s.doc_mut()?;
        let v = page_mut(d)?.presets.brushes.get_or_insert_with(Vec::new);
        let n = v.len();
        v.retain(|x| x.user_name.as_deref() != Some(name.as_str()));
        if v.len() == n {
            return Err(EngineError::NotFound(fp_i18n::tr!("err-nf-brush", name = name.as_str())));
        }
        Ok(CommandOutput::doc())
    }));

    // Eckenrundung der Rechteck-Grundform (14131: Rundung + %/px)
    r.add(
        ids(cmd("pi.roundness").label("Eckrundung").requires(DOC))
            .param("value", ParamKind::Number { min: Some(0.0), max: None }, "Rundung (Prozent bzw. Pixel)")
            .opt_default("pixels", ParamKind::Bool, json!(false), "Wert in Pixeln")
            .run(|s, p| {
                let v = p.f64("value")?;
                let px = p.bool_or("pixels", false)?;
                edit_each(s, p, |e, _| {
                    let ElementKind::Path(pe) = &mut e.kind else { return };
                    let Some(pr) = pe.primitive.as_mut() else { return };
                    // Pixel beziehen sich auf die aktuelle Größe (nach B/H-Änderungen steckt die
                    // Skalierung in der Matrix, PAR 17/05)
                    let (w, h) = primitive_size(pr);
                    let half = (w.min(h) / 2.0).max(1e-6);
                    let frac = if px { (v as f32 / half).clamp(0.0, 1.0) } else { (v as f32 / 100.0).clamp(0.0, 1.0) };
                    pr.roundness = frac;
                    pr.roundness_pixels = Some(px);
                    let m = pr.transform;
                    let local = crate::geometry::rect_contour(pr.rect, frac as f64);
                    let map = |q: Point| m.apply(q);
                    let mut c = local;
                    for n in &mut c.nodes {
                        n.anchor = map(n.anchor);
                        n.pred = map(n.pred);
                        n.succ = map(n.succ);
                    }
                    pe.contours = vec![c];
                    pe.polyline = None;
                })
            }),
    );

    // Effekte (Live-Filter-Liste 4224)
    r.add(
        ids(cmd("pi.effect.set").label("Effekt bearbeiten").requires(DOC))
            .param("index", ParamKind::Integer { min: Some(0), max: None }, "Index in der Effektliste")
            .param("params", ParamKind::Json, "Parameter {Schlüssel: Wert} wie im Dateiformat")
            .run(|s, p| {
                let i = p.usize("index")?;
                // Text wie im Dateiformat; Punktlisten (Kurven, `[[x, y], …]` bzw. `[{x, y}, …]`) als `GPLv`
                let vals: Vec<(String, fp_model::DataValue)> = match p.get("params") {
                    Some(Value::Object(m)) => m.iter().map(|(k, v)| (k.clone(), effect_param_value(v))).collect(),
                    _ => return Err(bad(p, fp_i18n::t("err-param-params"))),
                };
                edit_effects(s, p, move |l| match l.effects.get_mut(i) {
                    Some(e) => {
                        for (k, v) in &vals {
                            match v {
                                fp_model::DataValue::Scalar(t) => e.set(k, t.clone()),
                                other => e.params.set_structured(k, other.clone()),
                            }
                        }
                        Ok(())
                    }
                    None => Err(EngineError::NotFound(fp_i18n::t("err-nf-effect"))),
                })
            }),
    );
    r.add(
        ids(cmd("pi.effect.move").label("Effekt verschieben").requires(DOC))
            .param("index", ParamKind::Integer { min: Some(0), max: None }, "Index")
            .param("to", ParamKind::Integer { min: Some(0), max: None }, "neuer Index")
            .run(|s, p| {
                let (i, to) = (p.usize("index")?, p.usize("to")?);
                edit_effects(s, p, move |l| {
                    if i >= l.effects.len() {
                        return Err(EngineError::NotFound(fp_i18n::t("err-nf-effect")));
                    }
                    let e = l.effects.remove(i);
                    let to = to.min(l.effects.len());
                    l.effects.insert(to, e);
                    Ok(())
                })
            }),
    );
    r.add(ids(cmd("pi.effects.visible").label("Alle Filter ein/aus").requires(DOC)).param("visible", ParamKind::Bool, "sichtbar").run(|s, p| {
        let v = p.bool_or("visible", true)?;
        edit_effects(s, p, move |l| {
            for e in &mut l.effects {
                e.set_visible(v);
            }
            Ok(())
        })
    }));
    r.add(
        ids(cmd("pi.effects.replace").label("Effekte").requires(DOC))
            .param("effects", ParamKind::Json, "Effekte [{moa, visible, params{…}}, …] (leer = keine)")
            .run(|s, p| {
                let list: Vec<EffectSpec> = match p.get("effects") {
                    Some(Value::Array(a)) => a
                        .iter()
                        .filter_map(|e| {
                            let moa = e.get("moa")?.as_str()?.to_string();
                            let vis = e.get("visible").and_then(|v| v.as_bool()).unwrap_or(true);
                            let params = e.get("params").and_then(|v| v.as_object()).map(|m| m.iter().map(|(k, v)| (k.clone(), v.as_str().map(|x| x.to_string()).unwrap_or_else(|| v.to_string()))).collect()).unwrap_or_default();
                            Some((moa, vis, params))
                        })
                        .collect(),
                    _ => return Err(bad(p, fp_i18n::t("err-param-effects"))),
                };
                edit_effects(s, p, move |l| {
                    l.effects.clear();
                    for (moa, vis, params) in &list {
                        let mut e = Effect::new(moa, *vis);
                        for (k, v) in params {
                            if k != "EffectIsVisible" && k != "EffectMoaID" {
                                e.set(k, v.clone());
                            }
                        }
                        l.effects.push(e);
                    }
                    Ok(())
                })
            }),
    );

    // Pfadoperationen (15174): Compound Shape anlegen, Verknüpfung ändern, kombinieren
    r.add(
        cmd("pi.compound.setOp")
            .label("Pfadoperation")
            .requires(&[Requirement::Document, Requirement::Selection])
            .param("op", ParamKind::Integer { min: Some(0), max: Some(4) }, "0 Normal, 1 Vereinigen, 2 Überschneiden, 3 Ausstanzen, 4 Zuschneiden")
            .run(|s, p| {
                let op = p.i64("op")?.clamp(0, 4) as u32;
                let (page, ids) = targets(s, p)?;
                let d = s.doc_mut()?;
                let doc = Arc::make_mut(&mut d.doc);
                let elems: Vec<Arc<Element>> = ids.iter().filter_map(|id| docops::element_on_page(doc, page, *id)).collect();
                let all_compound = elems.iter().all(|e| matches!(e.kind, ElementKind::CompoundShape(_)));
                if all_compound && !elems.is_empty() {
                    for e in &elems {
                        docops::edit_by_id(doc, page, e.id, |x| {
                            if let ElementKind::CompoundShape(c) = &mut x.kind {
                                // wie beim Anlegen: Verknüpfungen nur aus der gewählten Operation
                                // (unabhängig von der Vorgeschichte, §4b)
                                let ops = compound_ops(op.max(1), c.operands.len());
                                for (o, po) in c.operands.iter_mut().zip(ops) {
                                    if let ElementKind::Path(pe) = &mut Element::edit(o).kind {
                                        pe.path_op = if op == 0 { 0 } else { po };
                                    }
                                }
                            }
                            recompute_compound(x);
                        });
                    }
                    return Ok(CommandOutput::doc());
                }
                // Pfade zu einer neuen Compound Shape zusammenfassen
                let paths: Vec<fp_model::ElementPath> = docops::paths_sorted(doc, page, &ids);
                let mut operands = Vec::new();
                for path in &paths {
                    if let Some(e) = doc.element_at(path)
                        && matches!(e.kind, ElementKind::Path(_))
                    {
                        operands.push(Arc::clone(e));
                    }
                }
                if operands.len() < 2 {
                    return Err(EngineError::Selection(fp_i18n::t("err-sel-two-paths-needed")));
                }
                let pop = compound_ops(op.max(1), operands.len());
                for (o, po) in operands.iter_mut().zip(pop) {
                    if let ElementKind::Path(pe) = &mut Element::edit(o).kind {
                        pe.path_op = po;
                    }
                }
                let first = paths.first().cloned().ok_or_else(|| EngineError::Selection(fp_i18n::t("err-sel-no-paths")))?;
                let op_ids: Vec<ElementId> = operands.iter().map(|o| o.id).collect();
                docops::remove_ids(doc, page, &op_ids);
                let mut idg = std::mem::take(&mut doc.ids);
                let mut cs = Element::new_compound_shape(&mut idg, operands);
                doc.ids = idg;
                recompute_compound(&mut cs);
                let new_id = cs.id;
                let idx = match first.steps.last() {
                    Some(fp_model::PathStep::Child(i)) => *i,
                    _ => 0,
                };
                let parent = fp_model::ElementPath { steps: first.steps[..first.steps.len().saturating_sub(1)].to_vec(), ..first.clone() };
                let inserted = if parent.steps.is_empty() {
                    let frame = d.frame;
                    let layer = doc.pages.get(page).and_then(|pg| match pg.root.layers.get(first.slot) {
                        Some(fp_model::LayerSlot::Own(l)) => Some(l.id),
                        _ => None,
                    });
                    docops::insert_element(doc, page, frame, layer, idx, Arc::new(cs)).is_some()
                } else {
                    let probe = fp_model::ElementPath { steps: [parent.steps.clone(), vec![fp_model::PathStep::Child(idx)]].concat(), ..first.clone() };
                    match docops::parent_list_mut(doc, &probe) {
                        Some((list, i)) => {
                            list.insert(i.min(list.len()), Arc::new(cs));
                            true
                        }
                        None => false,
                    }
                };
                if !inserted {
                    return Err(EngineError::Failed(fp_i18n::t("err-compound-insert-failed")));
                }
                d.selection.set(vec![new_id]);
                Ok(CommandOutput::doc())
            }),
    );
    r.add(cmd("pi.compound.combine").label("Kombinieren").requires(&[Requirement::Document, Requirement::Selection]).run(|s, p| {
        let (page, ids) = targets(s, p)?;
        let d = s.doc_mut()?;
        let doc = Arc::make_mut(&mut d.doc);
        let mut new_sel = Vec::new();
        for id in ids {
            let Some(path) = docops::find_on_page(doc, page, id) else { continue };
            let Some(e) = doc.element_at(&path).cloned() else { continue };
            let ElementKind::CompoundShape(c) = &e.kind else {
                new_sel.push(id);
                continue;
            };
            let Some(res) = c.result.clone() else { continue };
            let mut r = (*res).clone();
            r.id = e.id;
            r.common = e.common.clone();
            if let ElementKind::Path(pe) = &mut r.kind {
                pe.path_op = 0;
            }
            if let Some((list, i)) = docops::parent_list_mut(doc, &path)
                && let Some(slot) = list.get_mut(i)
            {
                *slot = Arc::new(r);
                new_sel.push(id);
            }
        }
        d.selection.set(new_sel);
        Ok(CommandOutput::doc())
    }));

    // Text: Glättung und Textversatz (Teilfeld 14144)
    r.add(
        ids(cmd("pi.text.antialias").label("Glättung").requires(DOC))
            .param("mode", ParamKind::Integer { min: Some(0), max: Some(4) }, "0 kein, 1 scharf, 2 stark, 3 glatt, 4 benutzerdefiniert")
            .run(|s, p| {
                // Zuordnung wie `text.setProperties {antialias}` (re\09 §3.5): „kein“ ist die harte
                // Füllkante, kein `TAAi`-Wert; `TAAi` 0 ist glatt (BEFUNDE EB, FV-9)
                let code = match p.i64("mode")? {
                    0 => "none",
                    1 => "crisp",
                    2 => "strong",
                    3 => "smooth",
                    _ => "custom",
                };
                let v = super::text::antialias_code(code);
                edit_each(s, p, |e, _| {
                    if let ElementKind::Text(t) = &mut e.kind {
                        super::text::apply_antialias(t, v);
                        t.detach_story();
                    }
                })
            }),
    );
    r.add(ids(cmd("pi.text.autoKern").label("Autom. Unterschneidung").requires(DOC)).param("on", ParamKind::Bool, "ein").run(|s, p| {
        let v = p.bool_or("on", true)?;
        edit_each(s, p, |e, _| {
            if let ElementKind::Text(t) = &mut e.kind {
                t.auto_kern = v;
                t.detach_story();
            }
        })
    }));
    r.add(ids(cmd("pi.text.pathOffset").label("Textversatz").requires(DOC)).param("offset", ParamKind::Number { min: None, max: None }, "Versatz in px").run(|s, p| {
        let v = p.f64("offset")? as f32;
        edit_each(s, p, |e, _| {
            if let ElementKind::Text(t) = &mut e.kind
                && let Some(o) = &mut t.on_path
            {
                o.offset = Some(v);
            }
        })
    }));

    // Web-Objekte (Segment 14121/14104/14120/14152/14151, Hotspot 14125/14126/14127)
    r.add(
        ids(cmd("pi.web").label("Web-Objekt").requires(DOC))
            .opt("url", ParamKind::String, "Hyperlink")
            .opt("alt", ParamKind::String, "Alternativtext")
            .opt("target", ParamKind::String, "Zielfenster")
            .opt("fileName", ParamKind::String, "Dateiname")
            .opt("sliceType", ParamKind::Integer { min: Some(0), max: Some(2) }, "0 Vordergrundbild, 1 Hintergrundbild, 2 HTML")
            .opt("html", ParamKind::String, "HTML-Text")
            .opt("shape", ParamKind::Integer { min: Some(0), max: Some(2) }, "Hotspot-Form 0 Rechteck, 1 Ellipse, 2 Polygon")
            .opt("color", ParamKind::Color, "Farbe der Web-Ebene")
            .opt("bgRepeat", ParamKind::Integer { min: Some(0), max: Some(3) }, "Hintergrund wiederholen")
            .opt("bgAttachment", ParamKind::Integer { min: Some(0), max: Some(1) }, "Anhang")
            .opt("bgHAlign", ParamKind::Integer { min: Some(0), max: Some(3) }, "horizontale Position")
            .opt("bgVAlign", ParamKind::Integer { min: Some(0), max: Some(3) }, "vertikale Position")
            .opt("bgHValue", ParamKind::Integer { min: Some(0), max: Some(100) }, "Wert horizontal")
            .opt("bgVValue", ParamKind::Integer { min: Some(0), max: Some(100) }, "Wert vertikal")
            .run(|s, p| {
                let o = |k: &str| p.opt_str(k);
                let (url, alt, target, file, html) = (o("url")?, o("alt")?, o("target")?, o("fileName")?, o("html")?);
                let st = p.opt_i64("sliceType")?.map(|v| v as i32);
                let shape = p.opt_i64("shape")?;
                let color = p.opt_color("color")?;
                let bg: Vec<Option<i32>> = ["bgRepeat", "bgAttachment", "bgHAlign", "bgVAlign", "bgHValue", "bgVValue"].iter().map(|k| p.opt_i64(k).ok().flatten().map(|v| v as i32)).collect();
                let opt = |v: &Option<String>| v.as_ref().map(|x| if x.is_empty() { None } else { Some(x.clone()) });
                edit_each(s, p, |e, _| {
                    match &mut e.kind {
                        ElementKind::WebObject(w) => {
                            if let Some(v) = opt(&url) {
                                w.url = v;
                            }
                            if let Some(v) = opt(&alt) {
                                w.alt_text = v;
                            }
                            if let Some(v) = opt(&target) {
                                w.target = v;
                            }
                            if let Some(v) = opt(&file) {
                                w.file_name = v;
                            }
                            if let Some(c) = color {
                                w.color = c;
                            }
                            if let Some(sh) = shape {
                                w.shape = fp_model::WebShape::from_code(sh as i32);
                            }
                            if let Some(sp) = &mut w.slice {
                                if let Some(t) = st {
                                    sp.slice_type = t;
                                }
                                if let Some(v) = opt(&html) {
                                    sp.html = v;
                                }
                                if bg.iter().any(|x| x.is_some()) {
                                    let b = sp.background.get_or_insert_with(Default::default);
                                    if let Some(v) = bg[0] {
                                        b.repeat = v;
                                    }
                                    if let Some(v) = bg[1] {
                                        b.attachment = v;
                                    }
                                    if let Some(v) = bg[2] {
                                        b.h_align = v;
                                    }
                                    if let Some(v) = bg[3] {
                                        b.v_align = v;
                                    }
                                    if let Some(v) = bg[4] {
                                        b.h_value = v;
                                    }
                                    if let Some(v) = bg[5] {
                                        b.v_value = v;
                                    }
                                }
                            }
                        }
                        ElementKind::Instance(i) => {
                            if let Some(v) = opt(&url) {
                                i.button.url = v;
                            }
                            if let Some(v) = opt(&alt) {
                                i.button.alt = v;
                            }
                            if let Some(v) = opt(&target) {
                                i.button.target = v;
                            }
                            if let Some(v) = opt(&file) {
                                i.button.file_name = v;
                            }
                        }
                        _ => {}
                    }
                })
            }),
    );
    // Verhalten (Verhalten-Bedienfeld 4128, Menüs 6927/6928)
    r.add(
        ids(cmd("pi.behavior.add").label("Verhalten hinzufügen").requires(DOC))
            .param("action", ParamKind::Integer { min: Some(0), max: Some(11) }, "BETi")
            .opt_default("event", ParamKind::Integer { min: Some(0), max: Some(3) }, json!(0), "0 onMouseOver, 1 onClick, 2 onMouseOut, 3 onLoad")
            .opt("statusText", ParamKind::String, "Statusleistentext (Art 1)")
            .opt("frame", ParamKind::Integer { min: Some(0), max: None }, "Status (Bild austauschen)")
            .run(|s, p| {
                let a = p.i64("action")? as i32;
                let ev = p.i64_or("event", 0)? as i32;
                let txt = p.opt_str("statusText")?;
                let fr = p.opt_i64("frame")?.map(|v| v as i32);
                edit_each(s, p, |e, _| {
                    let mut b = fp_model::Behavior::new(fp_model::BehaviorAction::from_code(a), ev);
                    b.status_text = txt.clone();
                    b.frame = fr;
                    match &mut e.kind {
                        ElementKind::WebObject(w) => w.behaviors.get_or_insert_with(Default::default).items.push(b),
                        ElementKind::Instance(i) => i.behaviors.get_or_insert_with(Default::default).items.push(b),
                        _ => {}
                    }
                })
            }),
    );
    r.add(ids(cmd("pi.behavior.remove").label("Verhalten löschen").requires(DOC)).param("index", ParamKind::Integer { min: Some(0), max: None }, "Index").run(|s, p| {
        let i = p.usize("index")?;
        edit_each(s, p, |e, _| {
            let list = match &mut e.kind {
                ElementKind::WebObject(w) => w.behaviors.as_mut(),
                ElementKind::Instance(x) => x.behaviors.as_mut(),
                _ => None,
            };
            if let Some(l) = list
                && i < l.items.len()
            {
                l.items.remove(i);
            }
        })
    }));
    r.add(
        ids(cmd("pi.behavior.event").label("Ereignis ändern").requires(DOC))
            .param("index", ParamKind::Integer { min: Some(0), max: None }, "Index")
            .param("event", ParamKind::Integer { min: Some(0), max: Some(3) }, "Ereignis")
            .run(|s, p| {
                let (i, ev) = (p.usize("index")?, p.i64("event")? as i32);
                edit_each(s, p, |e, _| {
                    let list = match &mut e.kind {
                        ElementKind::WebObject(w) => w.behaviors.as_mut(),
                        ElementKind::Instance(x) => x.behaviors.as_mut(),
                        _ => None,
                    };
                    if let Some(b) = list.and_then(|l| l.items.get_mut(i)) {
                        b.event = ev;
                    }
                })
            }),
    );
    r.add(
        ids(cmd("pi.button").label("Schaltfläche").requires(DOC))
            .opt("text", ParamKind::String, "Schaltflächentext")
            .opt("showDownOnLoad", ParamKind::Bool, "Down-Status beim Laden anzeigen")
            .run(|s, p| {
                let text = p.opt_str("text")?;
                let down = p.opt_bool("showDownOnLoad")?;
                edit_each(s, p, |e, _| {
                    if let ElementKind::Instance(i) = &mut e.kind {
                        if let Some(t) = &text {
                            i.button.text = if t.is_empty() { None } else { Some(t.clone()) };
                        }
                        if let Some(v) = down {
                            i.show_down_on_load = Some(v);
                        }
                    }
                })
            }),
    );
    r.add(
        ids(cmd("pi.symbol.params").label("Symboleigenschaften").requires(DOC))
            .param("values", ParamKind::Json, "{Name: Wert} der Rich-Symbol-Parameter")
            .run(|s, p| {
                let vals: Vec<(String, String)> = match p.get("values") {
                    Some(Value::Object(m)) => m.iter().map(|(k, v)| (k.clone(), v.as_str().map(|x| x.to_string()).unwrap_or_else(|| v.to_string()))).collect(),
                    _ => return Err(bad(p, fp_i18n::t("err-param-values"))),
                };
                edit_each(s, p, |e, _| {
                    if let ElementKind::Instance(i) = &mut e.kind {
                        let mut w = i.widget_values();
                        for (k, v) in &vals {
                            match w.iter_mut().find(|x| &x.name == k) {
                                Some(x) => x.value = v.clone(),
                                None => w.push(fp_model::element::WidgetParam { name: k.clone(), value: v.clone(), kind: "string".into() }),
                            }
                        }
                        i.widget_params = Some(fp_model::element::format_widget_params(&w));
                    }
                })
            }),
    );

    // Status-Eigenschaften (Windoid 6041, Status-Bedienfeld)
    r.add(
        cmd("pi.state.props")
            .label("Statuseigenschaften")
            .requires(DOC)
            .opt("indices", ParamKind::Json, "Status-Indizes (Vorgabe: aktueller)")
            .opt("delay", ParamKind::Integer { min: Some(0), max: Some(9999) }, "Verzögerung in 1/100 s")
            .opt("visible", ParamKind::Bool, "Beim Export einschließen")
            .opt("name", ParamKind::String, "Statusname")
            .opt("disposal", ParamKind::Integer { min: Some(0), max: Some(3) }, "Löschmethode")
            .run(|s, p| {
                let delay = p.opt_i64("delay")?.map(|v| v as i32);
                let vis = p.opt_bool("visible")?;
                let name = p.opt_str("name")?;
                let disp = p.opt_i64("disposal")?.map(|v| v as i32);
                let d = s.doc_mut()?;
                let n = d.frame_count();
                let idx: Vec<usize> = match p.get("indices").and_then(|v| v.as_array()) {
                    Some(a) => a.iter().filter_map(|v| v.as_u64().map(|x| x as usize)).filter(|&i| i < n).collect(),
                    None => vec![d.frame],
                };
                let page = d.page;
                let pg = Arc::make_mut(&mut d.doc).page_mut(page).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?;
                let ex = pg.root.export.get_or_insert_with(Default::default);
                while ex.frames.len() < n {
                    ex.frames.push(fp_model::FrameInfo::default());
                }
                for i in idx {
                    if let Some(f) = ex.frames.get_mut(i) {
                        if let Some(v) = delay {
                            f.delay = v;
                        }
                        if let Some(v) = vis {
                            f.visible = v;
                        }
                        if let Some(v) = &name {
                            f.name = if v.is_empty() { None } else { Some(v.clone()) };
                        }
                        if let Some(v) = disp {
                            f.disposal = v;
                        }
                    }
                }
                Ok(CommandOutput::doc())
            }),
    );
    r.add(
        cmd("pi.state.loop")
            .label("GIF-Animationsschleife")
            .requires(DOC)
            .param("loop", ParamKind::Integer { min: Some(-1), max: Some(10000) }, "−1 keine, 0 endlos, n Wiederholungen")
            .run(|s, p| {
                let v = p.i64("loop")? as i32;
                let d = s.doc_mut()?;
                let page = d.page;
                let pg = Arc::make_mut(&mut d.doc).page_mut(page).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?;
                pg.root.export.get_or_insert_with(Default::default).loop_count = v;
                Ok(CommandOutput::doc())
            }),
    );
    r.add(
        cmd("pi.state.onionSkin")
            .label("Zwiebelschichteneffekt")
            .requires(DOC)
            .param("before", ParamKind::Integer { min: Some(0), max: Some(1000) }, "Status davor")
            .param("after", ParamKind::Integer { min: Some(0), max: Some(1000) }, "Status danach")
            .run(|s, p| {
                let (b, a) = (p.i64("before")? as i32, p.i64("after")? as i32);
                let d = s.doc_mut()?;
                let page = d.page;
                let pg = Arc::make_mut(&mut d.doc).page_mut(page).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?;
                pg.root.onion_before = b;
                pg.root.onion_after = a;
                Ok(CommandOutput::doc())
            }),
    );

    // wie im Original schon mit einem Objekt aktiv (MN-3, PARITAET_B §3)
    r.add(cmd("pi.state.distribute").label("Auf Status verteilen").requires(&[Requirement::Document, Requirement::Selection]).run(|s, p| {
        let (page, ids) = targets(s, p)?;
        let d = s.doc_mut()?;
        let start = d.frame;
        let layer = d.layer;
        let doc = Arc::make_mut(&mut d.doc);
        let paths = docops::paths_sorted(doc, page, &ids);
        // unten zuerst (das Original verteilt von unten nach oben)
        let mut order: Vec<ElementId> = paths.iter().rev().filter_map(|pth| doc.element_at(pth).map(|e| e.id)).collect();
        order.dedup();
        let removed = docops::remove_ids(doc, page, &order);
        let need = start + removed.len();
        let have = doc.pages.get(page).map(|pg| pg.root.frame_count as usize).unwrap_or(1);
        if need > have {
            docops::insert_frames(doc, page, have, need - have).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?;
        }
        let by_id: std::collections::HashMap<ElementId, Arc<Element>> = removed.into_iter().map(|(_, e)| (e.id, e)).collect();
        for (k, id) in order.iter().enumerate() {
            if let Some(e) = by_id.get(id) {
                docops::insert_element(doc, page, start + k, layer, 0, Arc::clone(e)).ok_or_else(|| EngineError::Failed(fp_i18n::t("err-distribute-failed")))?;
            }
        }
        d.selection.clear();
        Ok(CommandOutput::doc())
    }));
    r.add(
        cmd("pi.state.copyTo")
            .label("In Status kopieren")
            .requires(&[Requirement::Document, Requirement::Selection])
            .param("from", ParamKind::Integer { min: Some(0), max: None }, "erster Zielstatus")
            .param("to", ParamKind::Integer { min: Some(0), max: None }, "letzter Zielstatus")
            .run(|s, p| {
                let (from, to) = (p.usize("from")?, p.usize("to")?);
                let (page, ids) = targets(s, p)?;
                let d = s.doc_mut()?;
                let (cur, layer) = (d.frame, d.layer);
                let n = d.frame_count();
                let doc = Arc::make_mut(&mut d.doc);
                let elems: Vec<Arc<Element>> = ids.iter().filter_map(|id| docops::element_on_page(doc, page, *id)).collect();
                for f in from.min(to)..=to.max(from).min(n.saturating_sub(1)) {
                    if f == cur {
                        continue;
                    }
                    for e in elems.iter().rev() {
                        let copy = docops::reassign(doc, e);
                        docops::insert_element(doc, page, f, layer, 0, Arc::new(copy));
                    }
                }
                Ok(CommandOutput::doc())
            }),
    );

    // Exporteinstellungen der Seite (Optimieren-Bedienfeld)
    r.add(
        cmd("pi.export.settings")
            .label("Optimieren")
            .requires(DOC)
            .param("settings", ParamKind::Json, "fp_model::ExportSettings als JSON")
            .run(|s, p| {
                let es: fp_model::ExportSettings = p.parse("settings")?.ok_or_else(|| bad(p, fp_i18n::t("err-param-settings-missing")))?;
                let d = s.doc_mut()?;
                let page = d.page;
                let pg = Arc::make_mut(&mut d.doc).page_mut(page).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?;
                let frames = pg.root.export.as_ref().map(|e| e.frames.clone()).unwrap_or_default();
                let mut es = es;
                if es.frames.is_empty() {
                    es.frames = frames;
                }
                pg.root.export = Some(Box::new(es));
                Ok(CommandOutput::doc())
            }),
    );

    // Ebenen/Seiten: Masterseite, Web-Ebene-Farbe, Unterebene
    r.add(cmd("pi.page.setMaster").label("Als Masterseite festlegen").requires(DOC).opt("index", ParamKind::Integer { min: Some(0), max: None }, "Seite").opt_default("master", ParamKind::Bool, json!(true), "Masterseite").run(|s, p| {
        let m = p.bool_or("master", true)?;
        let d = s.doc_mut()?;
        let i = p.opt_i64("index")?.map(|v| v as usize).unwrap_or(d.page);
        let doc = Arc::make_mut(&mut d.doc);
        if i >= doc.pages.len() {
            return Err(EngineError::NotFound(fp_i18n::t("err-nf-page")));
        }
        if m {
            for (k, pg) in doc.pages.iter_mut().enumerate() {
                if k != i && pg.is_master == Some(true) {
                    Arc::make_mut(pg).is_master = Some(false);
                }
            }
            let pg = Arc::make_mut(&mut doc.pages[i]);
            pg.is_master = Some(true);
            if i != 0 {
                let pg = doc.pages.remove(i);
                doc.pages.insert(0, pg);
                d.page = 0;
            }
        } else if let Some(pg) = doc.pages.get_mut(i) {
            Arc::make_mut(pg).is_master = Some(false);
        }
        doc.invalidate_root_copy();
        Ok(CommandOutput::doc())
    }));
    r.add(
        cmd("pi.layer.addSub")
            .label("Neue Unterebene")
            .requires(DOC)
            .opt("name", ParamKind::String, "Name")
            .run(|s, p| {
                let name = p.opt_str("name")?;
                let d = s.doc_mut()?;
                let (page, frame, layer) = (d.page, d.frame, d.layer);
                let doc = Arc::make_mut(&mut d.doc);
                let frames = doc.pages.get(page).map(|pg| pg.root.frame_count as usize).unwrap_or(1);
                let mut idg = std::mem::take(&mut doc.ids);
                let n = name.unwrap_or_else(|| fp_i18n::t("msg-default-sublayer"));
                let sub = fp_model::Layer::new(&mut idg, &n, fp_model::LayerKind::Normal, frames.max(1));
                let sub_id = sub.id;
                let e = Element::new(idg.element(), ElementKind::SubLayer(Arc::new(sub)));
                doc.ids = idg;
                let id = e.id;
                // in der aktiven Ebene, auch einer aktiven Unterebene (verschachtelt); die neue
                // Unterebene wird aktiv wie im Original (B-2 BA-030)
                docops::insert_element(doc, page, frame, layer, 0, Arc::new(e)).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-layer")))?;
                d.layer = Some(sub_id);
                d.selection.set(vec![id]);
                Ok(CommandOutput::doc())
            }),
    );

    // Ebenen-Bedienfeld: über Seiten geteilte Ebene als aktive Ebene wählen (für „Ausschließen“/„Lösen“)
    r.add(cmd("pi.layer.setActiveShared").label("Aktive Ebene").requires(DOC).param("layer", ParamKind::Integer { min: Some(0), max: None }, "Ebenen-ID der geteilten Ebene").run(|s, p| {
        let id = fp_model::LayerId(p.i64("layer")? as u64);
        let d = s.doc_mut()?;
        let page = d.page;
        let on_page = d.doc.pages.get(page).is_some_and(|pg| {
            pg.root.layers.iter().any(|sl| matches!(sl, LayerSlot::Shared(r) if d.doc.shared_layer(&r.id).and_then(|x| x.layer.as_ref()).is_some_and(|l| l.id == id)))
        });
        if !on_page {
            return Err(EngineError::NotFound(fp_i18n::t("err-nf-shared-layer")));
        }
        d.layer = Some(id);
        d.selection.clear();
        Ok(CommandOutput::doc())
    }));

    // Ausrichten-Bedienfeld (Align.swf): Ausrichten, Proportionieren, Größe, Abstand
    r.add(
        cmd("pi.align")
            .label("Ausrichten")
            .requires(&[Requirement::Document, Requirement::Selection])
            .param(
                "mode",
                ParamKind::Enum {
                    values: [
                        "left", "hcenter", "right", "top", "vcenter", "bottom", "distLeft", "distHCenter", "distRight", "distTop", "distVCenter", "distBottom", "matchWidth", "matchHeight", "matchBoth", "spaceH", "spaceV",
                    ]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                },
                "Art",
            )
            .opt_default("toCanvas", ParamKind::Bool, json!(false), "relativ zur Leinwand statt zur Auswahl")
            .run(|s, p| {
                let mode = p.str("mode")?;
                let to_canvas = p.bool_or("toCanvas", false)?;
                let sampler = s.prefs.edit_sampler();
                let (page, ids) = targets(s, p)?;
                let d = s.doc_mut()?;
                let doc = Arc::make_mut(&mut d.doc);
                let items: Vec<(ElementId, Rect)> = ids.iter().filter_map(|id| docops::element_on_page(doc, page, *id).and_then(|e| docops::element_bounds(doc, &e)).map(|b| (*id, b))).collect();
                if items.is_empty() {
                    return Err(EngineError::Selection(fp_i18n::t("err-sel-no-bounds-short")));
                }
                let union = items.iter().skip(1).fold(items[0].1, |a, (_, b)| a.union(b));
                let canvas = doc.pages.get(page).map(|pg| Rect::new(pg.canvas.origin.x, pg.canvas.origin.y, pg.canvas.origin.x + pg.canvas.width, pg.canvas.origin.y + pg.canvas.height)).unwrap_or(union);
                let refr = if to_canvas || items.len() == 1 { canvas } else { union };
                let mut moves: Vec<(ElementId, f32, f32)> = Vec::new();
                let mut scales: Vec<(ElementId, Rect, Rect)> = Vec::new();
                let sorted = |key: &dyn Fn(&Rect) -> f32| {
                    let mut v = items.clone();
                    v.sort_by(|a, b| key(&a.1).total_cmp(&key(&b.1)));
                    v
                };
                match mode.as_str() {
                    "left" => moves = items.iter().map(|(i, b)| (*i, refr.left - b.left, 0.0)).collect(),
                    "right" => moves = items.iter().map(|(i, b)| (*i, refr.right - b.right, 0.0)).collect(),
                    "hcenter" => moves = items.iter().map(|(i, b)| (*i, (refr.left + refr.right - b.left - b.right) / 2.0, 0.0)).collect(),
                    "top" => moves = items.iter().map(|(i, b)| (*i, 0.0, refr.top - b.top)).collect(),
                    "bottom" => moves = items.iter().map(|(i, b)| (*i, 0.0, refr.bottom - b.bottom)).collect(),
                    "vcenter" => moves = items.iter().map(|(i, b)| (*i, 0.0, (refr.top + refr.bottom - b.top - b.bottom) / 2.0)).collect(),
                    "distLeft" | "distHCenter" | "distRight" | "distTop" | "distVCenter" | "distBottom" => {
                        let key: Box<dyn Fn(&Rect) -> f32> = match mode.as_str() {
                            "distLeft" => Box::new(|r: &Rect| r.left),
                            "distHCenter" => Box::new(|r: &Rect| (r.left + r.right) / 2.0),
                            "distRight" => Box::new(|r: &Rect| r.right),
                            "distTop" => Box::new(|r: &Rect| r.top),
                            "distVCenter" => Box::new(|r: &Rect| (r.top + r.bottom) / 2.0),
                            _ => Box::new(|r: &Rect| r.bottom),
                        };
                        let horizontal = mode.contains("Left") || mode.contains("HCenter") || mode.contains("Right");
                        let v = sorted(&*key);
                        if v.len() >= 2 {
                            let (a, b) = if to_canvas { if horizontal { (canvas.left, canvas.right) } else { (canvas.top, canvas.bottom) } } else { (key(&v[0].1), key(&v[v.len() - 1].1)) };
                            let n = (v.len() - 1).max(1) as f32;
                            for (k, (i, r)) in v.iter().enumerate() {
                                let target = a + (b - a) * k as f32 / n;
                                let delta = target - key(r);
                                moves.push(if horizontal { (*i, delta, 0.0) } else { (*i, 0.0, delta) });
                            }
                        }
                    }
                    "spaceH" | "spaceV" => {
                        let horizontal = mode == "spaceH";
                        let v = sorted(&|r: &Rect| if horizontal { r.left } else { r.top });
                        if v.len() >= 3 || (to_canvas && !v.is_empty()) {
                            let total: f32 = v.iter().map(|(_, r)| if horizontal { r.width() } else { r.height() }).sum();
                            let (a, b) = if to_canvas { if horizontal { (canvas.left, canvas.right) } else { (canvas.top, canvas.bottom) } } else if horizontal { (union.left, union.right) } else { (union.top, union.bottom) };
                            let gaps = if to_canvas { v.len() as f32 + 1.0 } else { (v.len() - 1) as f32 };
                            let gap = ((b - a) - total) / gaps.max(1.0);
                            let mut cur = if to_canvas { a + gap } else { a };
                            for (i, r) in &v {
                                let (start, size) = if horizontal { (r.left, r.width()) } else { (r.top, r.height()) };
                                moves.push(if horizontal { (*i, cur - start, 0.0) } else { (*i, 0.0, cur - start) });
                                cur += size + gap;
                            }
                        }
                    }
                    _ => {
                        let (tw, th) = if to_canvas || items.len() == 1 {
                            (canvas.width(), canvas.height())
                        } else {
                            (items.iter().map(|(_, r)| r.width()).fold(0.0, f32::max), items.iter().map(|(_, r)| r.height()).fold(0.0, f32::max))
                        };
                        for (i, r) in &items {
                            let w = if mode != "matchHeight" { tw } else { r.width() };
                            let h = if mode != "matchWidth" { th } else { r.height() };
                            scales.push((*i, *r, Rect::new(r.left, r.top, r.left + w, r.top + h)));
                        }
                    }
                }
                for (id, dx, dy) in moves {
                    if dx != 0.0 || dy != 0.0 {
                        docops::edit_by_id(doc, page, id, |e| docops::translate_element(e, dx, dy));
                    }
                }
                for (id, from, to) in scales {
                    if from.width() > 0.0 && from.height() > 0.0 {
                        let m = docops::rect_to_rect(from, to);
                        docops::edit_by_id(doc, page, id, |e| docops::transform_element(e, &m, sampler));
                    }
                }
                Ok(CommandOutput::doc())
            }),
    );

    // Ebenen-Bedienfeld: Element per Ziehen in eine Ebene/Position verschieben
    r.add(
        cmd("pi.element.moveTo")
            .label("Objekt verschieben")
            .requires(DOC)
            .param("id", ParamKind::Integer { min: Some(0), max: None }, "Element-ID")
            .param("layer", ParamKind::Integer { min: Some(0), max: None }, "Ziel-Ebenen-ID")
            .param("index", ParamKind::Integer { min: Some(0), max: None }, "Position in der Ebene (0 = oben)")
            .run(|s, p| {
                let id = ElementId(p.i64("id")? as u64);
                let layer = fp_model::LayerId(p.i64("layer")? as u64);
                let mut index = p.usize("index")?;
                let d = s.doc_mut()?;
                let (page, frame) = (d.page, d.frame);
                let doc = Arc::make_mut(&mut d.doc);
                let path = docops::find_on_page(doc, page, id).ok_or(EngineError::ElementNotFound(id.0))?;
                let target_slot = docops::layer_slot(doc, page, layer).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-layer")))?;
                if path.steps.len() == 1
                    && path.slot == target_slot
                    && let Some(fp_model::PathStep::Child(old)) = path.steps.first()
                    && *old < index
                {
                    index -= 1;
                }
                let e = docops::remove_at(doc, &path).ok_or(EngineError::ElementNotFound(id.0))?;
                docops::insert_element(doc, page, frame, Some(layer), index, e).ok_or_else(|| EngineError::Failed(fp_i18n::t("err-object-move-failed")))?;
                d.layer = Some(layer);
                d.selection.set(vec![id]);
                Ok(CommandOutput::doc())
            }),
    );

    // Stile (Teilfeld 14153, Stile-Bedienfeld)
    r.add(cmd("pi.style.new").label("Neuer Stil").requires(&[Requirement::Document, Requirement::Selection]).param("name", ParamKind::String, "Name").opt("flags", ParamKind::Integer { min: Some(0), max: Some(0x1ff) }, "Attributgruppen ZSFi").opt("otherFlags", ParamKind::Integer { min: Some(0), max: Some(0x1f) }, "ZSOi").run(|s, p| {
        let name = p.str("name")?;
        if name.trim().is_empty() {
            return Err(bad(p, fp_i18n::t("err-param-name-missing")));
        }
        let flags = p.opt_i64("flags")?.map(|v| v as u32);
        let other = p.opt_i64("otherFlags")?.map(|v| v as u32);
        let d = s.doc_mut()?;
        let (page, ids) = (d.page, d.selection.elements.clone());
        let first = ids.first().and_then(|id| docops::element_on_page(&d.doc, page, *id)).ok_or_else(|| EngineError::Selection(fp_i18n::t("err-sel-nothing")))?;
        let mut st = fp_model::Style::new(&name);
        if let Some(f) = flags {
            st.flags = f;
        }
        if let Some(o) = other {
            st.other_flags = o;
        }
        st.attrs = docops::path_attrs_of(&first);
        st.effects = first.common.effects.clone();
        if let ElementKind::Text(t) = &first.kind {
            st.text = t.resolved_runs().first().map(|(a, _)| a.clone()).unwrap_or_default();
            st.antialias = Some(t.antialias);
        }
        let doc = Arc::make_mut(&mut d.doc);
        let styles = doc.styles.get_or_insert_with(Vec::new);
        // „Stil neu definieren“ ersetzt an derselben Stelle
        match styles.iter().position(|x| x.name == name) {
            Some(i) => styles[i] = Arc::new(st),
            None => styles.push(Arc::new(st)),
        }
        for id in ids {
            docops::edit_by_id(doc, page, id, |e| e.common.style = Some(name.clone()));
        }
        Ok(CommandOutput::doc())
    }));
    // ohne Auswahl über `ids` (Stil aus dem Bedienfeld auf ein Objekt ziehen)
    r.add(cmd("pi.style.apply").label("Stil anwenden").requires(DOC).opt("ids", ParamKind::Ids, "Elemente (Vorgabe: Auswahl)").param("name", ParamKind::String, "Stilname (leer = Verknüpfung lösen)").run(|s, p| {
        let name = p.str("name")?;
        let (page, ids) = targets(s, p)?;
        let d = s.doc_mut()?;
        let st = d.doc.styles.as_ref().and_then(|v| v.iter().find(|x| x.name == name).cloned());
        let doc = Arc::make_mut(&mut d.doc);
        for id in ids {
            docops::edit_by_id(doc, page, id, |e| {
                if name.is_empty() {
                    e.common.style = None;
                    return;
                }
                let Some(st) = &st else { return };
                e.common.style = Some(name.clone());
                if let Some(a) = &st.attrs {
                    docops::edit_path_attrs(e, &|x| {
                        if st.use_fill() {
                            x.fill = a.fill.clone();
                            x.fill_texture = a.fill_texture.clone();
                        }
                        if st.use_fill_color() {
                            x.fill_color = a.fill_color;
                        }
                        if st.use_brush() {
                            x.brush = a.brush.clone();
                            x.stroke_texture = a.stroke_texture.clone();
                        }
                        if st.use_brush_color() {
                            x.stroke_color = a.stroke_color;
                        }
                    });
                }
                if st.use_effects() {
                    set_effects(e, st.effects.clone());
                }
                if let ElementKind::Text(t) = &mut e.kind {
                    let ta = st.text.clone();
                    let (uf, us, ust) = (st.use_text_font(), st.use_text_size(), st.use_text_styles());
                    for r in &mut t.runs {
                        if uf && ta.font.is_some() {
                            r.attrs.font = ta.font.clone();
                        }
                        if us && ta.size.is_some() {
                            r.attrs.size = ta.size;
                        }
                        if ust {
                            r.attrs.bold = ta.bold.or(r.attrs.bold);
                            r.attrs.italic = ta.italic.or(r.attrs.italic);
                            r.attrs.underline = ta.underline.or(r.attrs.underline);
                        }
                    }
                    t.detach_story();
                }
            });
        }
        Ok(CommandOutput::doc())
    }));
    r.add(cmd("pi.style.delete").label("Stil löschen").requires(DOC).param("name", ParamKind::String, "Name").run(|s, p| {
        let name = p.str("name")?;
        let d = s.doc_mut()?;
        let doc = Arc::make_mut(&mut d.doc);
        if let Some(v) = doc.styles.as_mut() {
            let before = v.len();
            v.retain(|x| x.name != name);
            if v.len() == before {
                return Err(EngineError::NotFound(fp_i18n::tr!("err-nf-style", name = name.as_str())));
            }
        }
        Ok(CommandOutput::doc())
    }));
    r.add(cmd("pi.style.rename").label("Stil umbenennen").requires(DOC).param("name", ParamKind::String, "alter Name").param("newName", ParamKind::String, "neuer Name").run(|s, p| {
        let (old, new) = (p.str("name")?, p.str("newName")?);
        let d = s.doc_mut()?;
        let doc = Arc::make_mut(&mut d.doc);
        let st = doc.styles.as_mut().and_then(|v| v.iter_mut().find(|x| x.name == old)).ok_or_else(|| EngineError::NotFound(fp_i18n::tr!("err-nf-style", name = old.as_str())))?;
        Arc::make_mut(st).name = new;
        Ok(CommandOutput::doc())
    }));

    // Symbole (Dokumentbibliothek)
    r.add(cmd("pi.symbol.rename").label("Symbol umbenennen").requires(DOC).param("index", ParamKind::Integer { min: Some(0), max: None }, "Symbolindex").param("name", ParamKind::String, "Name").run(|s, p| {
        let (i, n) = (p.usize("index")?, p.str("name")?);
        let d = s.doc_mut()?;
        let doc = Arc::make_mut(&mut d.doc);
        let sym = doc.symbols.get_mut(i).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-symbol")))?;
        Arc::make_mut(sym).name = n;
        Ok(CommandOutput::doc())
    }));
    r.add(cmd("pi.symbol.delete").label("Symbol löschen").requires(DOC).param("index", ParamKind::Integer { min: Some(0), max: None }, "Symbolindex").run(|s, p| {
        let i = p.usize("index")?;
        let d = s.doc_mut()?;
        let doc = Arc::make_mut(&mut d.doc);
        if i >= doc.symbols.len() {
            return Err(EngineError::NotFound(fp_i18n::t("err-nf-symbol")));
        }
        let sym_id = doc.symbols[i].id;
        doc.symbols.remove(i);
        // Instanzen des Symbols entfernen (Rückfrage 6003 macht die Oberfläche)
        let page = d.page;
        let inst: Vec<ElementId> = {
            let mut v = Vec::new();
            if let Some(pg) = doc.pages.get(page) {
                fp_model::traverse::walk_root(&pg.root, fp_model::RootRef::Page(page), &mut |_, _, e| {
                    if let ElementKind::Instance(x) = &e.kind
                        && x.symbol_id == sym_id
                    {
                        v.push(e.id);
                    }
                });
            }
            v
        };
        docops::remove_ids(doc, page, &inst);
        d.selection.clear();
        Ok(CommandOutput::doc())
    }));
    r.add(
        cmd("pi.symbol.place")
            .label("Symbol einfügen")
            .requires(DOC)
            .param("index", ParamKind::Integer { min: Some(0), max: None }, "Symbolindex")
            .opt("at", ParamKind::Point, "Mittelpunkt (Vorgabe: Leinwandmitte)")
            .run(|s, p| {
                let i = p.usize("index")?;
                let at = p.opt_point("at")?;
                let d = s.doc_mut()?;
                let (page, frame, layer) = (d.page, d.frame, d.layer);
                let doc = Arc::make_mut(&mut d.doc);
                let sym = doc.symbols.get(i).cloned().ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-symbol")))?;
                let c = doc.pages.get(page).map(|pg| pg.canvas);
                let center = at.unwrap_or_else(|| c.map(|c| Point::new(c.origin.x + c.width / 2.0, c.origin.y + c.height / 2.0)).unwrap_or(Point::new(0.0, 0.0)));
                let mut idg = std::mem::take(&mut doc.ids);
                // Mitte des Symbolinhalts auf den Zielpunkt legen (Inhalt liegt in eigenen Koordinaten)
                let probe = Element::new_instance(&mut idg.clone(), sym.id, fp_geom::Matrix3::IDENTITY);
                let (cx, cy) = docops::element_bounds(doc, &probe).map(|b| ((b.left + b.right) / 2.0, (b.top + b.bottom) / 2.0)).unwrap_or((0.0, 0.0));
                let mut e = Element::new_instance(&mut idg, sym.id, fp_geom::Matrix3::translation(center.x - cx, center.y - cy));
                doc.ids = idg;
                e.common.name = Some(sym.name.clone());
                let id = e.id;
                docops::insert_element(doc, page, frame, layer, 0, Arc::new(e)).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-layer")))?;
                d.selection.set(vec![id]);
                Ok(CommandOutput::doc())
            }),
    );

    r.add(
        cmd("pi.symbol.import")
            .label("Symbol importieren")
            .requires(DOC)
            .param("path", ParamKind::Path, "FW-PNG mit Symbolen (Bibliothek)")
            .opt_default("place", ParamKind::Bool, json!(true), "Instanz des ersten Symbols einfügen")
            .run(|s, p| {
                let path = p.str("path")?;
                let place = p.bool_or("place", true)?;
                let bytes = std::fs::read(&path).map_err(|e| EngineError::Io(e.to_string()))?;
                let lib = fp_fwpng::read(&bytes).map_err(|e| EngineError::Format(e.to_string()))?;
                if lib.symbols.is_empty() {
                    return Err(EngineError::Failed(fp_i18n::t("err-file-no-symbols")));
                }
                let d = s.doc_mut()?;
                let (page, frame, layer) = (d.page, d.frame, d.layer);
                let doc = Arc::make_mut(&mut d.doc);
                let base = doc.symbols.iter().map(|x| x.id.0).max().unwrap_or(0) + 1;
                let map: std::collections::HashMap<u32, u32> = lib.symbols.iter().enumerate().map(|(i, x)| (x.id.0, base + i as u32)).collect();
                let mut idg = std::mem::take(&mut doc.ids);
                let mut first = None;
                for sym in &lib.symbols {
                    let mut ns = (**sym).clone();
                    ns.id = fp_model::SymbolId(*map.get(&sym.id.0).unwrap_or(&base));
                    ns.library_file = Some(path.clone());
                    for slot in &mut ns.root.layers {
                        if let fp_model::LayerSlot::Own(l) = slot {
                            let l = Arc::make_mut(l);
                            l.reassign_ids(&mut idg);
                            remap_layer(l, &map);
                        }
                    }
                    if first.is_none() {
                        first = Some(ns.id);
                    }
                    doc.symbols.push(Arc::new(ns));
                }
                let mut out = json!({"symbols": lib.symbols.len()});
                if place && let Some(sid) = first {
                    let c = doc.pages.get(page).map(|pg| pg.canvas);
                    let center = c.map(|c| Point::new(c.origin.x + c.width / 2.0, c.origin.y + c.height / 2.0)).unwrap_or(Point::new(0.0, 0.0));
                    let mut e = Element::new_instance(&mut idg, sid, fp_geom::Matrix3::translation(center.x, center.y));
                    e.common.name = doc.symbols.last().map(|x| x.name.clone());
                    let id = e.id;
                    doc.ids = idg;
                    docops::insert_element(doc, page, frame, layer, 0, Arc::new(e)).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-layer")))?;
                    d.selection.set(vec![id]);
                    out["id"] = json!(id.0);
                } else {
                    doc.ids = idg;
                }
                Ok(CommandOutput::doc_with(out))
            }),
    );

    // Hilfslinien-Dialog (5975): Farbe, anzeigen, ausrichten, sperren, alle löschen – ein Schritt
    r.add(
        cmd("pi.guides")
            .label("Hilfslinien")
            .requires(DOC)
            .opt("color", ParamKind::Color, "Hilfslinienfarbe")
            .opt("show", ParamKind::Bool, "Hilfslinien anzeigen")
            .opt("snap", ParamKind::Bool, "An Hilfslinien ausrichten")
            .opt("locked", ParamKind::Bool, "Hilfslinien sperren")
            .opt_default("clear", ParamKind::Bool, json!(false), "alle Hilfslinien löschen")
            .run(|s, p| {
                let c = p.opt_color("color")?;
                let l = p.opt_bool("locked")?;
                let (show, snap) = (p.opt_bool("show")?, p.opt_bool("snap")?);
                let clear = p.bool_or("clear", false)?;
                let d = s.doc_mut()?;
                if let Some(v) = show {
                    d.view.show_guides = v;
                }
                if let Some(v) = snap {
                    d.view.snap_to_guides = v;
                }
                if let Some(v) = l {
                    d.view.lock_guides = v;
                }
                let page = d.page;
                let pg = Arc::make_mut(&mut d.doc).page_mut(page).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?;
                let g = pg.root.guides.get_or_insert_with(Default::default);
                if let Some(c) = c {
                    g.color = Some(c);
                }
                if let Some(l) = l {
                    g.locked = Some(l);
                }
                if clear {
                    g.horizontal.clear();
                    g.vertical.clear();
                }
                Ok(CommandOutput::doc())
            }),
    );

    register_menus(r);
}

// ====================================================================== Bedienfeldmenüs

/// Befehl mit eigener Befehls-ID des Originals. Einträge der Bedienfeldmenüs stehen nicht im
/// Hauptmenü-Katalog, `CommandBuilder` setzt die ID aber nur aus dem Katalog.
struct FwCommand {
    info: CommandInfo,
    inner: FnCommand,
}

impl Command for FwCommand {
    fn info(&self) -> &CommandInfo {
        &self.info
    }
    fn apply(&self, s: &mut Session, p: &Params) -> Result<CommandOutput> {
        self.inner.apply(s, p)
    }
    fn checked(&self, s: &Session) -> Option<bool> {
        self.inner.checked(s)
    }
    fn menu_available(&self, s: &Session) -> bool {
        self.inner.menu_available(s)
    }
    fn extra_available(&self, s: &Session) -> bool {
        self.inner.extra_available(s)
    }
}

/// Registriert einen Bedienfeldmenü-Befehl mit Befehls-ID des Originals.
fn add_fw(r: &mut CommandRegistry, id: u32, c: FnCommand) {
    let mut info = c.info().clone();
    info.fw_id = Some(id);
    if info.label.is_empty() {
        info.label = info.name.clone();
    }
    if info.menu_label.is_empty() {
        info.menu_label = info.label.clone();
    }
    r.register(Arc::new(FwCommand { info, inner: c }));
}

/// Ergebnis eines weitergeleiteten Befehls (der innere Befehl hat den Verlaufsschritt angelegt).
fn passthrough(o: CommandOutput) -> CommandOutput {
    CommandOutput { effect: CmdEffect::None, ..o }
}

fn no_effect(value: Value) -> CommandOutput {
    CommandOutput { effect: CmdEffect::None, label: None, value }
}

/// Feste Parameter, ergänzt um die übergebenen.
fn merged(fixed: &Value, p: &Params) -> Value {
    let mut m = fixed.as_object().cloned().unwrap_or_default();
    if let Some(o) = p.value.as_object() {
        for (k, v) in o {
            m.insert(k.clone(), v.clone());
        }
    }
    Value::Object(m)
}

/// Bedienfeldmenü-Eintrag als Verweis auf einen vorhandenen Befehl (gleiche Wirkung, eigene ID).
/// Löschbefehl des Bedienfeldmenüs, der wie im Original immer aktiv ist: mit mehr als einem
/// Status/einer Seite (`many`) wie `target`, sonst ohne Wirkung (der letzte bleibt stehen).
fn last_kept(r: &mut CommandRegistry, id: u32, name: &str, label: &str, target: &'static str, many: fn(&DocumentState) -> bool) {
    let c = cmd(name).label(label).describe(&fp_i18n::tr!("msg-panel-alias", target = target)).requires(DOC).opt("index", ParamKind::Integer { min: Some(0), max: None }, "Index (Vorgabe: aktueller)").run(move |s, p| {
        if s.active().is_some_and(many) {
            s.execute(target, merged(&json!({}), p)).map(passthrough)
        } else {
            Ok(CommandOutput::value(json!({ "deleted": false })))
        }
    });
    add_fw(r, id, c);
}

fn alias(r: &mut CommandRegistry, id: u32, name: &str, label: &str, target: &'static str, fixed: Value) {
    let c = cmd(name).label(label).describe(&fp_i18n::tr!("msg-panel-alias", target = target)).available(move |s| s.command_state(target).0).run(move |s, p| s.execute(target, merged(&fixed, p)).map(passthrough));
    add_fw(r, id, c);
}

/// Setzt einen Wahrheitswert in den Voreinstellungen (Ganzzahl-Schlüssel als 0/1).
fn set_pref_bool(s: &mut Session, key: &str, v: bool) {
    let int = crate::prefs::definition(key).is_some_and(|d| matches!(d.default, PrefValue::Int(_)));
    s.prefs.set(key, if int { PrefValue::Int(v as i64) } else { PrefValue::Bool(v) });
    s.prefs_changed();
}

/// Umschalter in den Voreinstellungen (Parameter `on`, sonst umschalten).
fn pref_toggle(r: &mut CommandRegistry, id: u32, name: &str, label: &str, key: &'static str, default: bool) {
    let c = cmd(name).label(label).opt("on", ParamKind::Bool, "ein/aus (Vorgabe: umschalten)").checked(move |s| s.prefs.get_bool(key, default)).run(move |s, p| {
        let v = match p.opt_bool("on")? {
            Some(v) => v,
            None => !s.prefs.get_bool(key, default),
        };
        set_pref_bool(s, key, v);
        Ok(CommandOutput::session_with(json!({ "on": v })))
    });
    add_fw(r, id, c);
}

/// Auswahl eines Werts in den Voreinstellungen (Optionsgruppe im Menü).
fn pref_choice(r: &mut CommandRegistry, id: u32, name: &str, label: &str, key: &'static str, value: i64, default: i64) {
    let c = cmd(name).label(label).checked(move |s| s.prefs.get_int(key, default) == value).run(move |s, _| {
        s.prefs.set(key, PrefValue::Int(value));
        s.prefs_changed();
        Ok(CommandOutput::session())
    });
    add_fw(r, id, c);
}

fn page_mut(d: &mut DocumentState) -> Result<&mut fp_model::Page> {
    let page = d.page;
    Arc::make_mut(&mut d.doc).page_mut(page).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))
}

/// Exporteinstellungen der aktuellen Seite (Copy-on-Write, angelegt mit Vorgaben).
fn export_mut(d: &mut DocumentState) -> Result<&mut fp_model::ExportSettings> {
    Ok(page_mut(d)?.root.export.get_or_insert_with(Default::default))
}

/// Exportformat JPEG (`FMTi` 1).
fn is_jpeg(e: &fp_model::ExportSettings) -> bool {
    e.format == 1
}

/// Exportformat mit Farbtabelle: GIF, GIF-Animation, PNG 8 (`CLRi` 0).
fn has_palette(e: &fp_model::ExportSettings) -> bool {
    matches!(e.format, 0 | 4) || (e.format == 2 && e.color_mode == 0)
}

/// Erstes gewähltes Objekt ist eine Gruppe mit Maske („Maske bearbeiten“).
fn selection_has_mask(s: &Session) -> bool {
    s.active().is_some_and(|d| {
        d.selection.elements.first().and_then(|id| docops::element_on_page(&d.doc, d.page, *id)).is_some_and(|e| matches!(&e.kind, ElementKind::Group(g) if g.mask.is_some()))
    })
}

fn export_of(s: &Session) -> fp_model::ExportSettings {
    s.active().and_then(|d| d.doc.pages.get(d.page)).and_then(|p| p.root.export.as_deref().cloned()).unwrap_or_default()
}

/// Slot der aktiven Ebene (eigene oder geteilte).
fn active_slot(d: &DocumentState) -> Option<usize> {
    let id = d.layer?;
    let pg = d.doc.pages.get(d.page)?;
    pg.root.layers.iter().position(|sl| match sl {
        LayerSlot::Own(l) => l.id == id,
        LayerSlot::Shared(r) => d.doc.shared_layer(&r.id).and_then(|x| x.layer.as_ref()).is_some_and(|l| l.id == id),
        _ => false,
    })
}

/// Freigeben für Seiten nur für Ebenen der obersten Stufe, nicht bei aktiver Unterebene (B-9).
fn no_active_sublayer(s: &Session) -> bool {
    s.active().is_some_and(|d| !d.layer.is_some_and(|l| docops::as_sublayer(&d.doc, d.page, l).is_some()))
}

/// Einfügeposition für neue Ebenen „oben“: unter der Web-Ebene.
fn top_insert_index(pg: &fp_model::Page) -> usize {
    pg.root.layers.iter().position(|sl| matches!(sl, LayerSlot::Own(l) if l.kind == LayerKind::Web)).unwrap_or(pg.root.layers.len())
}

fn master_page(doc: &fp_model::Document) -> Option<usize> {
    doc.pages.iter().position(|p| p.is_master == Some(true))
}

fn color_list(p: &Params, key: &str) -> Result<Vec<u32>> {
    let Some(v) = p.get(key) else { return Ok(Vec::new()) };
    let items: Vec<Value> = match v {
        Value::Array(a) => a.clone(),
        x => vec![x.clone()],
    };
    items.iter().map(|x| parse_color(x).map(|c| c.0 | 0xff00_0000).ok_or_else(|| bad(p, fp_i18n::tr!("err-param-color-expected", key = key)))).collect()
}

fn read_file(path: &str) -> Result<Vec<u8>> {
    std::fs::read(path).map_err(|e| EngineError::Io(format!("{path}: {e}")))
}

fn write_file(path: &str, bytes: &[u8]) -> Result<()> {
    fp_i18n::profile::guard_write(std::path::Path::new(path));
    std::fs::write(path, bytes).map_err(|e| EngineError::Io(format!("{path}: {e}")))
}

fn ext_of(path: &str) -> String {
    std::path::Path::new(path).extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

// ---------------------------------------------------------------------- Farbfelder

/// Voreinstellung des Farbfeldsatzes: 0 Farbwürfel, 1 benutzerdefiniert, 2 Graustufen, 3 Mac,
/// 4 Windows, 5 Kontinuierlicher Farbton.
pub const SWATCH_SET_KEY: &str = "SwatchesPanelColorPaletteChoice";
/// Eigene Farbfelder (Liste `#rrggbb`, Firepixel).
pub const CUSTOM_SWATCHES_KEY: &str = "Firepixel_CustomSwatches";

/// Eingebauter Farbfeldsatz (Nummern wie [`SWATCH_SET_KEY`]).
pub fn swatch_set(choice: i64) -> Vec<Argb> {
    let opaque = |v: Vec<u32>| v.into_iter().map(|c| Argb(c | 0xff00_0000)).collect::<Vec<_>>();
    match choice {
        0 => opaque(fp_export::palettes::palette_web216()),
        2 => opaque(fp_export::palettes::palette_gray(256)),
        3 => opaque(fp_export::palettes::palette_mac()),
        4 => opaque(fp_export::palettes::palette_windows()),
        5 => {
            let mut v = opaque(fp_export::palettes::palette_web216());
            v.sort_by_key(|c| {
                let (h, s, b) = fp_color::models::rgb_to_hsb8(c.r(), c.g(), c.b());
                (if s == 0 { 1 } else { 0 }, h, -b, s)
            });
            v
        }
        _ => Vec::new(),
    }
}

/// Eigene Farbfelder aus den Voreinstellungen.
pub fn custom_swatches(prefs: &crate::Preferences) -> Vec<Argb> {
    match prefs.get(CUSTOM_SWATCHES_KEY) {
        Some(PrefValue::List(v)) => v.iter().filter_map(|x| x.as_str()).filter_map(|h| Rgba8::parse_hex(&h)).map(|c| Argb(c.to_argb().0 | 0xff00_0000)).collect(),
        _ => Vec::new(),
    }
}

/// Aktuelle Farbfelder des Farbfelder-Bedienfelds.
pub fn current_swatches(prefs: &crate::Preferences) -> Vec<Argb> {
    match prefs.get_int(SWATCH_SET_KEY, 0) {
        1 => custom_swatches(prefs),
        n => swatch_set(n),
    }
}

fn set_custom_swatches(s: &mut Session, v: &[Argb]) {
    let list = v.iter().map(|c| PrefValue::Str(format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b()))).collect();
    s.prefs.set(CUSTOM_SWATCHES_KEY, PrefValue::List(list));
    s.prefs.set(SWATCH_SET_KEY, PrefValue::Int(1));
    s.prefs_changed();
}

/// Farbfelder aus einer Datei (ACT, ASE, ACO, GIF, Farbtabellen des Originals).
pub fn read_swatch_file(path: &str) -> Result<Vec<Argb>> {
    let bytes = read_file(path)?;
    let cols: Vec<Rgba8> = match ext_of(path).as_str() {
        "aco" => fp_color::swatches::read_aco(&bytes).map_err(|e| EngineError::Format(e.to_string()))?.iter().filter_map(|c| c.to_rgba8(None)).collect(),
        _ => fp_color::swatches::read_swatches_original(&bytes, None).map_err(|e| EngineError::Format(e.to_string()))?,
    };
    Ok(cols.into_iter().map(|c| Argb(c.to_argb().0 | 0xff00_0000)).collect())
}

/// Farbfelder als Datei (Endung `act`, `ase` oder `aco`).
pub fn swatch_file_bytes(ext: &str, colors: &[Argb]) -> Vec<u8> {
    let cols: Vec<Rgba8> = colors.iter().map(|c| c.to_rgba8()).collect();
    match ext {
        "ase" => fp_color::swatches::write_ase_original(&cols),
        "aco" => fp_color::swatches::write_aco(&cols.iter().map(|c| fp_color::swatches::AcoColor::from_rgb(*c, None)).collect::<Vec<_>>()),
        _ => fp_color::swatches::write_act_original(&cols),
    }
}

/// Sortieren nach Farbe (14 §2.4): Auswahlsortierung nach kleinstem H8, bei Gleichstand der letzte.
pub fn sort_by_color(colors: &[Argb]) -> Vec<Argb> {
    let mut rest: Vec<Argb> = colors.to_vec();
    let mut out = Vec::with_capacity(rest.len());
    while !rest.is_empty() {
        let mut best = 0;
        let mut best_h = i32::MAX;
        for (i, c) in rest.iter().enumerate() {
            let (h, _, _) = fp_color::models::rgb_to_hsb8(c.r(), c.g(), c.b());
            if h <= best_h {
                best_h = h;
                best = i;
            }
        }
        out.push(rest.remove(best));
    }
    out
}

// ---------------------------------------------------------------------- Export-Hilfen

/// Rendert die aktuelle Seite und optimiert sie mit den Seiteneinstellungen.
fn export_current(s: &Session, settings: Option<&fp_export::OptimizeSettings>) -> Result<(fp_raster::RgbaImage, fp_export::OptimizeSettings, fp_export::export::ExportedImage)> {
    use fp_export::render::PageRenderer as _;
    let d = s.doc()?;
    let st = settings.cloned().unwrap_or_else(|| fp_export::doc_export::page_settings(&d.doc, d.page));
    let req = fp_export::render::RenderRequest {
        page: d.page,
        frame: d.frame,
        background: if st.matte.is_some() { fp_export::render::Background::Transparent } else { fp_export::render::Background::Canvas },
        ..Default::default()
    };
    let img = crate::export_backend::ExportRenderer::new(s.renderer()).render(&d.doc, &req).map_err(EngineError::Failed)?;
    let ctx = fp_export::doc_export::page_context(&d.doc, d.page);
    let ex = fp_export::export::export_image(&img, &st, &ctx, &fp_export::export::EncodeExtras::standalone()).map_err(|e| EngineError::Failed(e.to_string()))?;
    Ok((img, st, ex))
}

/// Palette des aktuellen Exports (indizierte Formate), ARGB.
pub fn current_export_palette(s: &Session) -> Result<Vec<u32>> {
    let (_, _, ex) = export_current(s, None)?;
    match &ex.optimized {
        fp_export::Optimized::Indexed(ix) => Ok(ix.palette.clone()),
        _ => Err(EngineError::Failed(fp_i18n::t("err-format-no-palette"))),
    }
}

/// Exporteinstellungen der Seite aus `OptimizeSettings` schreiben.
fn store_settings(d: &mut DocumentState, st: &fp_export::OptimizeSettings) -> Result<()> {
    let base = d.doc.pages.get(d.page).and_then(|p| p.root.export.as_deref().cloned()).unwrap_or_default();
    let model = st.to_model(&base);
    page_mut(d)?.root.export = Some(Box::new(model));
    Ok(())
}

/// Gespeicherte Exportvorgaben (Name → Exporteinstellungen als JSON).
pub const EXPORT_PRESETS_KEY: &str = "Firepixel_ExportPresets";

/// Eigene Exportvorgaben aus den Voreinstellungen.
pub fn export_presets(prefs: &crate::Preferences) -> Vec<(String, fp_model::ExportSettings)> {
    match prefs.get(EXPORT_PRESETS_KEY) {
        Some(PrefValue::Dict(m)) => m.iter().filter_map(|(k, v)| v.as_str().and_then(|j| serde_json::from_str(&j).ok()).map(|e| (k.clone(), e))).collect(),
        _ => Vec::new(),
    }
}

// ---------------------------------------------------------------------- Stile/Symbole: Dateien

fn read_styles(path: &str) -> Result<Vec<Arc<fp_model::Style>>> {
    let bytes = read_file(path)?;
    if bytes.starts_with(b"\x89PNG") {
        let doc = fp_fwpng::read(&bytes).map_err(|e| EngineError::Format(e.to_string()))?;
        return Ok(doc.styles.unwrap_or_default());
    }
    let v: Vec<fp_model::Style> = serde_json::from_slice(&bytes).map_err(|e| EngineError::Format(fp_i18n::tr!("err-style-library", detail = e.to_string())))?;
    Ok(v.into_iter().map(Arc::new).collect())
}

/// Dokument, das nur Symbole enthält (Bibliotheksdatei wie „Symbole exportieren“).
fn library_document(symbols: Vec<Arc<fp_model::Symbol>>) -> fp_model::Document {
    let mut doc = fp_model::Document::new(100.0, 100.0, Argb(0x00ff_ffff));
    doc.symbols = symbols;
    doc
}

fn write_library(s: &Session, path: &str, symbols: Vec<Arc<fp_model::Symbol>>) -> Result<()> {
    let doc = library_document(symbols);
    let bytes = fp_fwpng::write(&doc, &fp_fwpng::WriteOptions::default(), &fp_fwpng::NoFlattener).map_err(|e| EngineError::Format(e.to_string()))?;
    let _ = s;
    write_file(path, &bytes)
}

fn symbol_index(s: &Session, p: &Params) -> Result<usize> {
    let d = s.doc()?;
    let i = p.usize("index")?;
    if i >= d.doc.symbols.len() {
        return Err(EngineError::NotFound(fp_i18n::t("err-nf-symbol")));
    }
    Ok(i)
}

fn unused_symbols(doc: &fp_model::Document) -> Vec<usize> {
    let mut used = std::collections::HashSet::new();
    doc.walk_elements(&mut |_: &fp_model::WalkCtx, _: &fp_model::ElementPath, e: &Arc<Element>| {
        if let ElementKind::Instance(i) = &e.kind {
            used.insert(i.symbol_id);
        }
    });
    doc.symbols.iter().enumerate().filter(|(_, x)| !used.contains(&x.id)).map(|(i, _)| i).collect()
}

/// Ausgewählte Verlaufsschritte als Befehle: Parameter `steps` (Indizes) oder `from`/`to`.
fn history_steps(s: &Session, p: &Params) -> Result<Vec<crate::history::RecordedCommand>> {
    let d = s.doc()?;
    let n = d.history.undo_count();
    let idx: Vec<usize> = match p.parse::<Vec<usize>>("steps")? {
        Some(v) => v.into_iter().filter(|i| *i < n).collect(),
        None => {
            let from = p.i64_or("from", 0)?.max(0) as usize;
            let to = p.opt_i64("to")?.map(|v| v.max(0) as usize).unwrap_or(n).min(n);
            (from..to).collect()
        }
    };
    if idx.is_empty() {
        return Err(EngineError::Failed(fp_i18n::t("err-no-steps")));
    }
    Ok(idx.into_iter().flat_map(|i| d.history.export(i..i + 1)).collect())
}

/// Alle Bedienfeldmenü-Befehle (11 §2.1, Kontextmenü-Ressourcen 6919–6941, 7026).
fn register_menus(r: &mut CommandRegistry) {
    register_layer_menu(r);
    register_state_menu(r);
    register_page_menu(r);
    register_history_menu(r);
    register_style_menu(r);
    register_library_menu(r);
    register_swatch_menu(r);
    register_optimize_menu(r);
    register_context_menus(r);
}

// ---------------------------------------------------------------------- Weitere Kontextmenüs

fn cur_frame_info(s: &Session) -> fp_model::FrameInfo {
    s.active().map(|d| export_of(s).frame(d.frame)).unwrap_or_default()
}

fn onion(s: &Session) -> (i32, i32) {
    s.active().and_then(|d| d.doc.pages.get(d.page)).map(|p| (p.root.onion_before, p.root.onion_after)).unwrap_or((0, 0))
}

/// Inspektor-Effektmenü (6921), Löschmethode (6922), Zwiebelschicht (6933), Schleife (6940),
/// Palettenmenü des Optimieren-Bedienfelds (6942).
fn register_context_menus(r: &mut CommandRegistry) {
    // 6921
    add_fw(
        r,
        35148,
        cmd("panel.effects.saveAsStyle").label("Als Stil speichern").requires(&[Requirement::Document, Requirement::Selection]).param("name", ParamKind::String, "Stilname").run(|s, p| {
            // nur die Effekte (ZSOi Bit 3)
            s.execute("pi.style.new", json!({"name": p.str("name")?, "flags": 0x1f0, "otherFlags": 8})).map(passthrough)
        }),
    );
    alias(r, 35350, "panel.effects.allOn", "Alle ein", "pi.effects.visible", json!({"visible": true}));
    alias(r, 35351, "panel.effects.allOff", "Alle aus", "pi.effects.visible", json!({"visible": false}));
    add_fw(
        r,
        35352,
        cmd("panel.effects.locatePlugins").label("Zusatzmodule suchen").param("path", ParamKind::Path, "Ordner mit Zusatzmodulen").run(|s, p| {
            let path = p.str("path")?;
            if !std::path::Path::new(&path).is_dir() {
                return Err(EngineError::NotFound(fp_i18n::tr!("err-nf-folder", path = path.as_str())));
            }
            s.prefs.set("Firepixel_PluginFolder", PrefValue::Str(path));
            s.prefs_changed();
            Ok(CommandOutput::session())
        }),
    );
    // 6922 Löschmethode des aktuellen Status
    for (id, name, label, v) in [(35155, "panel.states.disposalUnspecified", "Nicht angegeben", 0), (35156, "panel.states.disposalNone", "Ohne", 1), (35157, "panel.states.disposalBackground", "Hintergrund wiederherstellen", 2), (35158, "panel.states.disposalPrevious", "Vorheriges wiederherstellen", 3)] {
        add_fw(r, id, cmd(name).label(label).requires(DOC).checked(move |s| s.active().is_some() && cur_frame_info(s).disposal == v).run(move |s, _| s.execute("pi.state.props", json!({"disposal": v})).map(passthrough)));
    }
    // 6933 Zwiebelschicht
    for (id, name, label, b, a) in [(35282, "panel.states.onionNone", "Kein Zwiebelschicht-Effekt", 0, 0), (35283, "panel.states.onionNext", "Nächsten Status anzeigen", 0, 1), (35284, "panel.states.onionBeforeAfter", "Vorher und Nachher", 1, 1), (35322, "panel.states.onionAll", "Alle Status anzeigen", 999, 999)] {
        add_fw(r, id, cmd(name).label(label).requires(DOC).checked(move |s| s.active().is_some() && onion(s) == (b, a)).run(move |s, _| s.execute("pi.state.onionSkin", json!({"before": b, "after": a})).map(passthrough)));
    }
    alias(r, 35281, "panel.states.onionCustom", "Zwiebelschicht: Benutzerdefiniert", "pi.state.onionSkin", json!({}));
    pref_toggle(r, 35285, "panel.states.multiFrameEditing", "Mehrfach-Status-Bearbeitung", "MultiFrameEditing", true);
    // 6940 Schleife
    for (id, n) in [(35387u32, -1i64), (35389, 1), (35390, 2), (35391, 3), (35392, 4), (35393, 5), (35394, 10), (35395, 20), (35388, 0)] {
        let name = match n {
            -1 => "panel.states.loopNone".to_string(),
            0 => "panel.states.loopForever".to_string(),
            k => format!("panel.states.loop{k}"),
        };
        let label = match n {
            -1 => "Keine Schleife".to_string(),
            0 => "Endlos".to_string(),
            k => format!("{k} Wiederholungen"),
        };
        add_fw(r, id, cmd(&name).label(&label).requires(DOC).checked(move |s| s.active().is_some() && export_of(s).loop_count as i64 == n).run(move |s, _| s.execute("pi.state.loop", json!({"loop": n})).map(passthrough)));
    }
    // 6942 Palette
    add_fw(r, 35409, cmd("panel.optimize.rebuildPalette").label("Farbtabelle neu aufbauen").requires(DOC).run(|s, _| edit_opt(s, fp_export::palette_edit::rebuild)));
    alias(r, 35208, "panel.optimize.addColorMenu", "Farbe hinzufügen", "panel.optimize.addColor", json!({}));
    alias(r, 35202, "panel.optimize.editColorMenu", "Farbe bearbeiten", "panel.optimize.editColor", json!({}));
    alias(r, 35192, "panel.optimize.deleteColorMenu", "Farbe löschen", "panel.optimize.deleteColors", json!({}));
    alias(r, 35224, "panel.optimize.snapToWebMenu", "Websicher machen", "panel.optimize.snapToWeb", json!({}));
    alias(r, 35218, "panel.optimize.lockColorMenu", "Farbe sperren", "panel.optimize.lockColors", json!({}));
    for (id, name, label, v) in [(35219, "panel.optimize.sortLuminance", "Nach Leuchtkraft sortieren", 1), (35220, "panel.optimize.sortPopularity", "Nach Häufigkeit sortieren", 2), (35221, "panel.optimize.sortNone", "Nicht sortiert", 0)] {
        pref_choice(r, id, name, label, "Firepixel_OptimizeSort", v, 0);
    }
}

// ---------------------------------------------------------------------- Ebenen (6923)

fn edit_all_layers(s: &mut Session, f: impl Fn(&mut fp_model::Layer)) -> Result<CommandOutput> {
    let d = s.doc_mut()?;
    let pg = page_mut(d)?;
    for sl in &mut pg.root.layers {
        if let LayerSlot::Own(l) = sl {
            f(Arc::make_mut(l));
        }
    }
    Ok(CommandOutput::doc())
}

fn register_layer_menu(r: &mut CommandRegistry) {
    alias(r, 35439, "panel.layers.newBitmap", "Neues Bild", "edit.insert.emptyBitmap", json!({}));
    add_fw(
        r,
        35159,
        cmd("panel.layers.newLayer")
            .label("Neue Ebene")
            .requires(DOC)
            .opt("name", ParamKind::String, "Name")
            .opt_default("shared", ParamKind::Bool, json!(false), "Statusübergreifend freigeben")
            .run(|s, p| {
                let name = p.opt_str("name")?;
                let shared = p.bool_or("shared", false)?;
                super::with_doc(s, |d| {
                    let id = super::edit::add_layer(d, name.as_deref(), LayerKind::Normal)?;
                    if shared {
                        let page = d.page;
                        if let Some(slot) = docops::layer_slot(&d.doc, page, id)
                            && let Some(l) = docops::layer_mut(Arc::make_mut(&mut d.doc), page, slot)
                        {
                            l.shared_across_frames = true;
                        }
                    }
                    d.layer = Some(id);
                    Ok(CommandOutput::doc_with(json!({"layer": id.0})).labeled(fp_i18n::t("msg-step-new-layer")))
                })
            }),
    );
    alias(r, 35794, "panel.layers.newSubLayer", "Neue Unterebene", "pi.layer.addSub", json!({}));
    add_fw(
        r,
        35160,
        cmd("panel.layers.duplicate")
            .label("Ebene duplizieren")
            .requires(DOC)
            .opt_default("count", ParamKind::Integer { min: Some(1), max: Some(100) }, json!(1), "Anzahl")
            .opt_default("position", ParamKind::Enum { values: ["top", "above", "below", "bottom"].iter().map(|s| s.to_string()).collect() }, json!("above"), "Einfügen: oben, vor (über) der aktuellen, nach (unter) der aktuellen, unten")
            .run(|s, p| {
                let n = p.i64_or("count", 1)?.clamp(1, 100) as usize;
                let pos = p.opt_str("position")?.unwrap_or_else(|| "above".into());
                super::with_doc(s, |d| {
                    let page = d.page;
                    // aktive Unterebene: Kopien in ihrer Elternebene (Liste oben → unten), B-9
                    if let Some(el) = d.layer.and_then(|l| docops::as_sublayer(&d.doc, page, l)) {
                        let nid = docops::duplicate_sublayer(Arc::make_mut(&mut d.doc), page, el, n, |i, len| match pos.as_str() {
                            "top" => 0,
                            "below" => i + 1,
                            "bottom" => len,
                            _ => i,
                        })
                        .ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-layer")))?;
                        d.layer = Some(nid);
                        return Ok(CommandOutput::doc().labeled(fp_i18n::t("msg-step-duplicate-layer")));
                    }
                    let slot = active_slot(d).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-layer")))?;
                    let doc = Arc::make_mut(&mut d.doc);
                    let Some(LayerSlot::Own(src)) = doc.pages.get(page).and_then(|pg| pg.root.layers.get(slot)).cloned() else {
                        return Err(EngineError::Failed(fp_i18n::t("err-only-own-layers")));
                    };
                    if src.kind == LayerKind::Web {
                        return Err(EngineError::Failed(fp_i18n::t("err-web-layer-no-duplicate")));
                    }
                    let mut idg = super::take_ids(doc);
                    let copies: Vec<fp_model::Layer> = (0..n)
                        .map(|_| {
                            let mut l = (*src).clone();
                            l.reassign_ids(&mut idg);
                            l.name = fp_i18n::tr!("msg-default-copy", name = src.name.as_str());
                            l
                        })
                        .collect();
                    doc.ids = idg;
                    let pg = doc.page_mut(page).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?;
                    let at = match pos.as_str() {
                        "top" => top_insert_index(pg),
                        "below" => slot,
                        "bottom" => 0,
                        _ => slot + 1,
                    };
                    let first = copies.last().map(|l| l.id);
                    for (k, l) in copies.into_iter().enumerate() {
                        pg.root.layers.insert(at + k, LayerSlot::Own(Arc::new(l)));
                    }
                    d.layer = first.or(d.layer);
                    Ok(CommandOutput::doc().labeled(fp_i18n::t("msg-step-duplicate-layer")))
                })
            }),
    );
    add_fw(
        r,
        35205,
        cmd("panel.layers.shareAcrossStates")
            .label("Ebene statusübergreifend freigeben")
            .requires(DOC)
            // auch eine aktive Unterebene (B-9)
            .checked(|s| s.active().and_then(|d| d.layer.and_then(|l| docops::layer_by_id(&d.doc, d.page, l))).is_some_and(|l| l.shared_across_frames))
            .run(|s, _| {
                let d = s.doc()?;
                let cur = d.layer.and_then(|l| docops::layer_by_id(&d.doc, d.page, l)).map(|l| l.shared_across_frames).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-layer")))?;
                s.execute("layer.setShared", json!({"shared": !cur})).map(passthrough)
            }),
    );
    pref_toggle(r, 35167, "panel.layers.singleLayerEditing", "Einzelebene bearbeiten", "SingleLayerEditing", false);
    alias(r, 35162, "panel.layers.delete", "Ebene löschen", "layer.delete", json!({}));
    add_fw(
        r,
        35816,
        cmd("panel.layers.addMasterPageLayer")
            .label("Masterseitenebene hinzufügen")
            .requires(DOC)
            .available(|s| s.active().is_some_and(|d| master_page(&d.doc).is_some_and(|m| m != d.page) && !d.doc.pages.get(d.page).is_some_and(|p| p.root.layers.iter().any(|l| matches!(l, LayerSlot::MasterPage(_))))))
            .run(|s, _| {
                super::with_doc(s, |d| {
                    let m = master_page(&d.doc).ok_or_else(|| EngineError::Failed(fp_i18n::t("err-no-master-page")))?;
                    let page = d.page;
                    let doc = Arc::make_mut(&mut d.doc);
                    let guid = match doc.pages[m].id.clone() {
                        Some(g) => g,
                        None => {
                            let g = fp_model::Guid::new_random();
                            Arc::make_mut(&mut doc.pages[m]).id = Some(g.clone());
                            g
                        }
                    };
                    let pg = doc.page_mut(page).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?;
                    pg.root.layers.insert(0, LayerSlot::MasterPage(fp_model::MasterPageLayer { master: guid, expanded: true, name: "Masterseite".into(), visibility: vec![true], has_visibility_node: true, visibility_extra: Default::default(), extra: Default::default() }));
                    pg.linked_to_master = Some(true);
                    doc.invalidate_root_copy();
                    Ok(CommandOutput::doc())
                })
            }),
    );
    let share = |s: &mut Session, pages: Option<Vec<usize>>| -> Result<CommandOutput> {
        super::with_doc(s, |d| {
            let page = d.page;
            if d.layer.is_some_and(|l| docops::as_sublayer(&d.doc, page, l).is_some()) {
                return Err(EngineError::Failed(fp_i18n::t("err-sublayer-no-share")));
            }
            let slot = active_slot(d).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-layer")))?;
            let all = pages.is_none();
            let doc = Arc::make_mut(&mut d.doc);
            let n = doc.pages.len();
            let targets: Vec<usize> = pages.unwrap_or_else(|| (0..n).collect()).into_iter().filter(|&i| i < n && i != page).collect();
            let guid = match doc.pages[page].root.layers.get(slot).cloned() {
                Some(LayerSlot::Own(l)) => {
                    if l.kind == LayerKind::Web {
                        return Err(EngineError::Failed(fp_i18n::t("err-web-layer-no-share")));
                    }
                    let g = fp_model::Guid::new_random();
                    doc.shared_layers.push(fp_model::SharedLayerDef { id: g.clone(), to_all_pages: all, layer: Some(l), extra: Default::default() });
                    doc.has_shared_layer_list = true;
                    let pg = doc.page_mut(page).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?;
                    pg.root.layers[slot] = LayerSlot::Shared(fp_model::SharedLayerRef { id: g.clone(), to_all_pages: false, extra: Default::default() });
                    g
                }
                Some(LayerSlot::Shared(r)) => {
                    if all && let Some(def) = doc.shared_layers.iter_mut().find(|x| x.id.same_as(&r.id)) {
                        def.to_all_pages = true;
                    }
                    r.id
                }
                _ => return Err(EngineError::NotFound(fp_i18n::t("err-nf-layer"))),
            };
            for i in targets {
                let pg = Arc::make_mut(&mut doc.pages[i]);
                if pg.root.layers.iter().any(|sl| matches!(sl, LayerSlot::Shared(r) if r.id.same_as(&guid))) {
                    continue;
                }
                let at = top_insert_index(pg);
                pg.root.layers.insert(at, LayerSlot::Shared(fp_model::SharedLayerRef { id: guid.clone(), to_all_pages: false, extra: Default::default() }));
            }
            doc.invalidate_root_copy();
            Ok(CommandOutput::doc())
        })
    };
    add_fw(
        r,
        35799,
        cmd("panel.layers.shareToPages").label("Ebene für Seiten freigeben").requires(DOC).menu_when(no_active_sublayer).param("pages", ParamKind::Json, "Seitenindizes [0, 2, …]").run(move |s, p| {
            let pages: Vec<usize> = p.parse("pages")?.unwrap_or_default();
            share(s, Some(pages))
        }),
    );
    add_fw(r, 35800, cmd("panel.layers.shareToAllPages").label("Ebene für alle Seiten freigeben").requires(DOC).menu_when(no_active_sublayer).run(move |s, _| share(s, None)));
    let shared_ref = |d: &DocumentState| -> Option<(usize, fp_model::Guid)> {
        let slot = active_slot(d)?;
        match d.doc.pages.get(d.page)?.root.layers.get(slot)? {
            LayerSlot::Shared(r) => Some((slot, r.id.clone())),
            _ => None,
        }
    };
    add_fw(
        r,
        35805,
        cmd("panel.layers.excludeShared")
            .label("Freigegebene Ebene ausschließen")
            .requires(DOC)
            .available(move |s| s.active().is_some_and(|d| shared_ref(d).is_some()))
            .run(move |s, _| {
                super::with_doc(s, |d| {
                    let (slot, _) = shared_ref(d).ok_or_else(|| EngineError::Failed(fp_i18n::t("err-no-shared-layer")))?;
                    page_mut(d)?.root.layers.remove(slot);
                    let page = d.page;
                    d.layer = docops::top_normal_layer(&d.doc, page);
                    Arc::make_mut(&mut d.doc).invalidate_root_copy();
                    Ok(CommandOutput::doc())
                })
            }),
    );
    add_fw(
        r,
        35796,
        cmd("panel.layers.detachShared")
            .label("Freigegebene Ebene lösen")
            .requires(DOC)
            .available(move |s| s.active().is_some_and(|d| shared_ref(d).is_some()))
            .run(move |s, _| {
                super::with_doc(s, |d| {
                    let (_, guid) = shared_ref(d).ok_or_else(|| EngineError::Failed(fp_i18n::t("err-no-shared-layer")))?;
                    let doc = Arc::make_mut(&mut d.doc);
                    let def = doc.shared_layers.iter().position(|x| x.id.same_as(&guid)).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-shared-layer")))?;
                    let layer = doc.shared_layers[def].layer.clone().ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-shared-layer")))?;
                    let mut first = true;
                    let mut idg = super::take_ids(doc);
                    let mut keep_id = None;
                    for pg in doc.pages.iter_mut() {
                        let pg = Arc::make_mut(pg);
                        for sl in pg.root.layers.iter_mut() {
                            if matches!(sl, LayerSlot::Shared(r) if r.id.same_as(&guid)) {
                                let mut l = (*layer).clone();
                                if first {
                                    keep_id = Some(l.id);
                                    first = false;
                                } else {
                                    l.reassign_ids(&mut idg);
                                }
                                *sl = LayerSlot::Own(Arc::new(l));
                            }
                        }
                    }
                    doc.ids = idg;
                    doc.shared_layers.remove(def);
                    doc.invalidate_root_copy();
                    let page = d.page;
                    let mine = docops::own_layers(&d.doc, page).into_iter().find(|(_, l)| l.name == layer.name).map(|(_, l)| l.id);
                    d.layer = mine.or(keep_id).or(d.layer);
                    Ok(CommandOutput::doc())
                })
            }),
    );
    alias(r, 35584, "panel.layers.flattenSelection", "Auswahl auf Hintergrund reduzieren", "modify.flattenSelection", json!({}));
    alias(r, 35585, "panel.layers.mergeDown", "Mit darunter liegender auf eine Ebene reduzieren", "modify.mergeDown", json!({}));
    let set_vis = |v: bool| {
        move |l: &mut fp_model::Layer| {
            if l.visibility.is_empty() {
                l.visibility.push(v);
            } else {
                l.visibility.fill(v);
            }
            l.has_visibility_node = true;
        }
    };
    add_fw(r, 35163, cmd("panel.layers.hideAll").label("Alles ausblenden").requires(DOC).run(move |s, _| edit_all_layers(s, set_vis(false))));
    add_fw(r, 35164, cmd("panel.layers.showAll").label("Alles anzeigen").requires(DOC).run(move |s, _| edit_all_layers(s, set_vis(true))));
    let set_lock = |v: bool| {
        move |l: &mut fp_model::Layer| {
            for c in &mut l.cels {
                Arc::make_mut(c).locked = v;
            }
        }
    };
    add_fw(r, 35165, cmd("panel.layers.lockAll").label("Alles sperren").requires(DOC).run(move |s, _| edit_all_layers(s, set_lock(true))));
    add_fw(r, 35166, cmd("panel.layers.unlockAll").label("Alle Sperrungen aufheben").requires(DOC).run(move |s, _| edit_all_layers(s, set_lock(false))));
    alias(r, 35441, "panel.layers.addMask", "Maske hinzufügen", "modify.mask.revealAll", json!({}));
    add_fw(
        r,
        35442,
        cmd("panel.layers.editMask").label("Maske bearbeiten").requires(&[Requirement::Document, Requirement::Selection]).menu_when(selection_has_mask).run(|s, _| {
            super::with_doc(s, |d| {
                let page = d.page;
                let first = d.selection.elements.first().copied().ok_or_else(|| super::sel_err("err-sel-nothing"))?;
                let e = docops::element_on_page(&d.doc, page, first).ok_or(EngineError::ElementNotFound(first.0))?;
                let ElementKind::Group(g) = &e.kind else { return Err(super::sel_err("err-sel-no-mask-group")) };
                let m = g.mask.as_ref().ok_or_else(|| super::sel_err("err-sel-group-no-mask"))?;
                let is_bitmap = matches!(m.kind, ElementKind::Bitmap(_));
                d.selection.set(vec![m.id]);
                if is_bitmap {
                    d.bitmap_mode = true;
                }
                Ok(CommandOutput::doc())
            })
        }),
    );
    alias(r, 35443, "panel.layers.disableMask", "Maske deaktivieren", "modify.mask.disableMask", json!({}));
    alias(r, 35444, "panel.layers.deleteMask", "Maske löschen", "modify.mask.deleteMask", json!({}));
    add_fw(
        r,
        35440,
        cmd("panel.layers.thumbnailOptions").label("Miniaturoptionen").param("size", ParamKind::Integer { min: Some(0), max: Some(128) }, "Miniaturgröße in px (0 = keine)").run(|s, p| {
            let v = p.i64("size")?.clamp(0, 128);
            s.prefs.set("LayersList_ThumbnailSize", PrefValue::Int(v));
            s.prefs_changed();
            Ok(CommandOutput::session())
        }),
    );
}

// ---------------------------------------------------------------------- Status (6924)

/// Einfügeposition aus „Am Anfang / Vor / Nach dem aktuellen / Am Ende“.
fn frame_position(d: &DocumentState, p: &Params) -> Result<usize> {
    Ok(match p.opt_str("where")?.as_deref() {
        Some("start") => 0,
        Some("before") => d.frame,
        Some("end") => d.frame_count(),
        _ => d.frame + 1,
    })
}

fn where_param() -> ParamKind {
    ParamKind::Enum { values: ["start", "before", "after", "end"].iter().map(|s| s.to_string()).collect() }
}

fn register_state_menu(r: &mut CommandRegistry) {
    add_fw(
        r,
        35168,
        cmd("panel.states.add")
            .label("Status hinzufügen")
            .requires(DOC)
            .opt_default("count", ParamKind::Integer { min: Some(1), max: Some(1000) }, json!(1), "Anzahl")
            .opt_default("where", where_param(), json!("after"), "Einfügen: am Anfang, vor/nach dem aktuellen Status, am Ende")
            .run(|s, p| {
                let n = p.i64_or("count", 1)?.max(1);
                let at = frame_position(s.doc()?, p)?;
                s.execute("state.add", json!({"count": n, "at": at})).map(passthrough)
            }),
    );
    add_fw(
        r,
        35169,
        cmd("panel.states.duplicate")
            .label("Status duplizieren")
            .requires(DOC)
            .opt_default("count", ParamKind::Integer { min: Some(1), max: Some(1000) }, json!(1), "Anzahl")
            .opt_default("where", where_param(), json!("after"), "Einfügen")
            .run(|s, p| {
                let n = p.i64_or("count", 1)?.clamp(1, 1000) as usize;
                super::with_doc(s, |d| {
                    let at = frame_position(d, p)?;
                    let (page, src) = (d.page, d.frame);
                    let doc = Arc::make_mut(&mut d.doc);
                    docops::insert_frames(doc, page, at, n).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?;
                    let src = if at <= src { src + n } else { src };
                    let mut idg = super::take_ids(doc);
                    if let Some(pg) = doc.page_mut(page) {
                        for sl in &mut pg.root.layers {
                            let LayerSlot::Own(l) = sl else { continue };
                            let l = Arc::make_mut(l);
                            if l.shared_across_frames {
                                continue;
                            }
                            let Some(c) = l.cels.get(src).cloned() else { continue };
                            for k in 0..n {
                                let mut nc = (*c).clone();
                                for e in &mut nc.elements {
                                    Arc::make_mut(e).reassign_ids(&mut idg);
                                }
                                if let Some(dst) = l.cels.get_mut(at + k) {
                                    *dst = Arc::new(nc);
                                }
                            }
                        }
                    }
                    doc.ids = idg;
                    d.frame = at.min(d.frame_count().saturating_sub(1));
                    d.selection.clear();
                    Ok(CommandOutput::doc().labeled(fp_i18n::t("msg-step-duplicate-state")))
                })
            }),
    );
    // wie im Original auch beim letzten Status aktiv; der letzte Status bleibt dann stehen (MN-3)
    last_kept(r, 35170, "panel.states.delete", "Status löschen", "state.delete", |d| d.frame_count() > 1);
    // mit nur einem Status gibt es kein Ziel: grau wie im Original (MN-3)
    add_fw(
        r,
        35171,
        cmd("panel.states.copyTo")
            .label("In Status kopieren")
            .describe(&fp_i18n::tr!("msg-panel-alias", target = "pi.state.copyTo"))
            .available(|s| s.command_state("pi.state.copyTo").0 && s.active().is_some_and(|d| d.frame_count() > 1))
            .run(|s, p| s.execute("pi.state.copyTo", merged(&json!({}), p)).map(passthrough)),
    );
    alias(r, 35172, "panel.states.distribute", "Auf Status verteilen", "pi.state.distribute", json!({}));
    let anim_toggle = |r: &mut CommandRegistry, id: u32, name: &str, label: &str, crop: bool| {
        let c = cmd(name)
            .label(label)
            .requires(DOC)
            .opt("on", ParamKind::Bool, "ein/aus (Vorgabe: umschalten)")
            .checked(move |s| {
                let e = export_of(s);
                if crop { e.anim_auto_crop } else { e.anim_auto_difference }
            })
            .run(move |s, p| {
                let cur = export_of(s);
                let cur = if crop { cur.anim_auto_crop } else { cur.anim_auto_difference };
                let v = p.opt_bool("on")?.unwrap_or(!cur);
                let d = s.doc_mut()?;
                let e = export_mut(d)?;
                if crop {
                    e.anim_auto_crop = v;
                } else {
                    e.anim_auto_difference = v;
                }
                Ok(CommandOutput::doc())
            });
        add_fw(r, id, c);
    };
    anim_toggle(r, 35383, "panel.states.autoCrop", "Autom. zuschneiden", true);
    anim_toggle(r, 35384, "panel.states.autoDifference", "Auto-Differenz", false);
    pref_toggle(r, 35852, "panel.states.numbering", "Nummerierung", "FrameNumbering", true);
    alias(r, 35385, "panel.states.properties", "Statuseigenschaften", "pi.state.props", json!({}));
}

// ---------------------------------------------------------------------- Seiten (7026)

fn register_page_menu(r: &mut CommandRegistry) {
    alias(r, 35801, "panel.pages.new", "Neue Seite", "edit.insert.page", json!({}));
    alias(r, 35806, "panel.pages.duplicate", "Seite duplizieren", "page.duplicate", json!({}));
    last_kept(r, 35802, "panel.pages.delete", "Seite löschen", "page.delete", |d| d.doc.pages.len() > 1);
    alias(r, 35803, "panel.pages.rename", "Seite umbenennen", "page.rename", json!({}));
    add_fw(
        r,
        35818,
        cmd("panel.pages.linkToMaster")
            .label("Mit Masterseite verknüpfen")
            .requires(DOC)
            .available(|s| s.active().is_some_and(|d| master_page(&d.doc).is_some_and(|m| m != d.page)))
            .checked(|s| s.active().and_then(|d| d.doc.pages.get(d.page)).is_some_and(|p| p.root.layers.iter().any(|l| matches!(l, LayerSlot::MasterPage(m) if m.master_id().is_some()))))
            .run(|s, _| {
                let linked = s.active().and_then(|d| d.doc.pages.get(d.page)).is_some_and(|p| p.root.layers.iter().any(|l| matches!(l, LayerSlot::MasterPage(m) if m.master_id().is_some())));
                if !linked {
                    let has_slot = s.active().and_then(|d| d.doc.pages.get(d.page)).is_some_and(|p| p.root.layers.iter().any(|l| matches!(l, LayerSlot::MasterPage(_))));
                    if !has_slot {
                        return s.execute("panel.layers.addMasterPageLayer", json!({})).map(passthrough);
                    }
                }
                super::with_doc(s, |d| {
                    let m = master_page(&d.doc).ok_or_else(|| EngineError::Failed(fp_i18n::t("err-no-master-page")))?;
                    let guid = d.doc.pages[m].id.clone().unwrap_or_else(fp_model::Guid::new_random);
                    let doc = Arc::make_mut(&mut d.doc);
                    Arc::make_mut(&mut doc.pages[m]).id = Some(guid.clone());
                    let page = d.page;
                    let pg = doc.page_mut(page).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?;
                    for sl in pg.root.layers.iter_mut() {
                        if let LayerSlot::MasterPage(mp) = sl {
                            mp.master = if linked { fp_model::Guid::nil() } else { guid.clone() };
                        }
                    }
                    pg.linked_to_master = Some(!linked);
                    doc.invalidate_root_copy();
                    Ok(CommandOutput::doc())
                })
            }),
    );
    alias(r, 35813, "panel.pages.setMaster", "Als Masterseite festlegen", "pi.page.setMaster", json!({}));
    add_fw(
        r,
        35814,
        cmd("panel.pages.resetMaster")
            .label("Masterseite zurücksetzen")
            .requires(DOC)
            .available(|s| s.active().is_some_and(|d| master_page(&d.doc).is_some()))
            .run(|s, _| {
                let m = s.active().and_then(|d| master_page(&d.doc)).ok_or_else(|| EngineError::Failed(fp_i18n::t("err-no-master-page")))?;
                s.execute("pi.page.setMaster", json!({"index": m, "master": false})).map(passthrough)
            }),
    );
    add_fw(
        r,
        35823,
        cmd("panel.pages.export")
            .label("Ausgewählte Seite(n) exportieren")
            .requires(DOC)
            .param("path", ParamKind::Path, "Zieldatei (Format nach den Exporteinstellungen der Seite)")
            .opt("pages", ParamKind::Json, "Seitenindizes (Vorgabe: aktuelle Seite); mehrere Seiten erhalten _Seitenname im Dateinamen")
            .run(|s, p| {
                let path = p.str("path")?;
                let d = s.doc()?;
                let pages: Vec<usize> = p.parse("pages")?.unwrap_or_else(|| vec![d.page]);
                let er = crate::export_backend::ExportRenderer::new(s.renderer());
                let mut written = Vec::new();
                for &pi in &pages {
                    if pi >= d.doc.pages.len() {
                        continue;
                    }
                    let st = fp_export::doc_export::page_settings(&d.doc, pi);
                    let bytes = fp_export::doc_export::export_whole(&er, &d.doc, pi, &st, &fp_export::export::EncodeExtras::standalone()).map_err(|e| EngineError::Failed(e.to_string()))?;
                    let target = if pages.len() == 1 {
                        path.clone()
                    } else {
                        let pp = std::path::Path::new(&path);
                        let stem = pp.file_stem().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
                        let ext = pp.extension().map(|x| format!(".{}", x.to_string_lossy())).unwrap_or_default();
                        let name = d.doc.pages[pi].name.clone().unwrap_or_else(|| fp_i18n::tr!("msg-default-page", n = (pi + 1).to_string()));
                        pp.with_file_name(format!("{stem}_{name}{ext}")).to_string_lossy().to_string()
                    };
                    write_file(&target, &bytes)?;
                    written.push(target);
                }
                Ok(CommandOutput::value(json!({"files": written})))
            }),
    );
    pref_toggle(r, 35853, "panel.pages.numbering", "Nummerierung", "PageNumbering", true);
    add_fw(
        r,
        35804,
        cmd("panel.pages.thumbnailOptions").label("Optionen für Miniaturansicht").param("size", ParamKind::Integer { min: Some(0), max: Some(128) }, "Miniaturgröße in px (0 = keine)").run(|s, p| {
            let v = p.i64("size")?.clamp(0, 128);
            s.prefs.set("PagesList_ThumbnailSize", PrefValue::Int(v));
            s.prefs_changed();
            Ok(CommandOutput::session())
        }),
    );
}

// ---------------------------------------------------------------------- Verlauf (6939)

fn register_history_menu(r: &mut CommandRegistry) {
    let range_opts = |b: crate::command::CommandBuilder| b.opt("steps", ParamKind::Json, "Schrittindizes [0, 2, …] (statt from/to)").opt("from", ParamKind::Integer { min: Some(0), max: None }, "erster Schritt (0 = ältester)").opt("to", ParamKind::Integer { min: Some(0), max: None }, "Ende (exklusiv, Vorgabe: alle)");
    add_fw(
        r,
        35378,
        range_opts(cmd("panel.history.replaySelected").label("Ausgewählte Schritte wiederholen").requires(&[Requirement::Document, Requirement::CanUndo])).run(|s, p| {
            let cmds = history_steps(s, p)?;
            if cmds.is_empty() {
                return Err(EngineError::Failed(fp_i18n::t("err-steps-not-replayable")));
            }
            s.replay(&cmds, "Schritte wiederholen")?;
            Ok(no_effect(json!({"steps": cmds.len()})))
        }),
    );
    add_fw(
        r,
        35379,
        range_opts(cmd("panel.history.copySteps").label("Schritte kopieren").requires(&[Requirement::Document, Requirement::CanUndo])).run(|s, p| {
            let cmds = history_steps(s, p)?;
            Ok(CommandOutput::value(json!({"commands": cmds, "script": crate::history::to_script(&cmds)})))
        }),
    );
    add_fw(
        r,
        35380,
        range_opts(cmd("panel.history.saveAsCommand").label("Als Befehl speichern").requires(&[Requirement::Document, Requirement::CanUndo]))
            .param("path", ParamKind::Path, "Zieldatei (.jsf)")
            .run(|s, p| {
                let path = p.str("path")?;
                let cmds = history_steps(s, p)?;
                write_file(&path, crate::history::to_script(&cmds).as_bytes())?;
                Ok(CommandOutput::value(json!({"path": path, "steps": cmds.len()})))
            }),
    );
    add_fw(
        r,
        35381,
        cmd("panel.history.clear").label("Verlauf löschen").requires(&[Requirement::Document]).run(|s, _| {
            let d = s.doc_mut()?;
            d.history.clear();
            let id = d.id;
            s.emit(SessionEvent::HistoryChanged(id));
            Ok(CommandOutput::session())
        }),
    );
}

// ---------------------------------------------------------------------- Stile (6935)

fn style_names(p: &Params) -> Result<Vec<String>> {
    Ok(match p.get("names") {
        Some(Value::Array(a)) => a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect(),
        _ => match p.opt_str("name")? {
            Some(n) => vec![n],
            None => Vec::new(),
        },
    })
}

fn register_style_menu(r: &mut CommandRegistry) {
    alias(r, 35294, "panel.styles.new", "Neuer Stil", "pi.style.new", json!({}));
    add_fw(
        r,
        35829,
        cmd("panel.styles.duplicate").label("Stil duplizieren").requires(DOC).param("name", ParamKind::String, "Stilname").opt("newName", ParamKind::String, "Name der Kopie").run(|s, p| {
            let name = p.str("name")?;
            let nn = p.opt_str("newName")?;
            super::with_doc(s, |d| {
                let doc = Arc::make_mut(&mut d.doc);
                let v = doc.styles.get_or_insert_with(Vec::new);
                let src = v.iter().find(|x| x.name == name).cloned().ok_or_else(|| EngineError::NotFound(fp_i18n::tr!("err-nf-style", name = name.as_str())))?;
                let mut copy = (*src).clone();
                let mut n = nn.unwrap_or_else(|| fp_i18n::tr!("msg-default-copy", name = name.as_str()));
                let mut k = 2;
                while v.iter().any(|x| x.name == n) {
                    n = fp_i18n::tr!("msg-default-copy-n", name = name.as_str(), n = k.to_string());
                    k += 1;
                }
                copy.name = n.clone();
                v.push(Arc::new(copy));
                Ok(CommandOutput::doc_with(json!({"name": n})))
            })
        }),
    );
    add_fw(
        r,
        35296,
        cmd("panel.styles.delete").label("Stile löschen").requires(DOC).opt("names", ParamKind::Json, "Stilnamen").opt("name", ParamKind::String, "Stilname").run(|s, p| {
            let names = style_names(p)?;
            if names.is_empty() {
                return Err(bad(p, fp_i18n::t("err-param-no-style")));
            }
            super::with_doc(s, |d| {
                let doc = Arc::make_mut(&mut d.doc);
                if let Some(v) = doc.styles.as_mut() {
                    v.retain(|x| !names.contains(&x.name));
                }
                Ok(CommandOutput::doc())
            })
        }),
    );
    alias(r, 35830, "panel.styles.rename", "Stil umbenennen", "pi.style.rename", json!({}));
    add_fw(
        r,
        35338,
        cmd("panel.styles.redefine").label("Stil neu definieren").requires(&[Requirement::Document, Requirement::Selection]).param("name", ParamKind::String, "Stilname").run(|s, p| {
            let name = p.str("name")?;
            let (flags, other) = s
                .doc()?
                .doc
                .styles
                .as_ref()
                .and_then(|v| v.iter().find(|x| x.name == name))
                .map(|x| (x.flags, x.other_flags))
                .ok_or_else(|| EngineError::NotFound(fp_i18n::tr!("err-nf-style", name = name.as_str())))?;
            s.execute("pi.style.new", json!({"name": name, "flags": flags, "otherFlags": other})).map(passthrough)
        }),
    );
    add_fw(
        r,
        35832,
        cmd("panel.styles.clearOverrides").label("Abweichungen löschen").requires(&[Requirement::Document, Requirement::Selection]).run(|s, _| {
            let d = s.doc()?;
            let items: Vec<(ElementId, String)> = d.selection.elements.iter().filter_map(|id| docops::element_on_page(&d.doc, d.page, *id).and_then(|e| e.common.style.clone().map(|n| (*id, n)))).collect();
            if items.is_empty() {
                return Err(super::sel_err("err-sel-no-styled-object"));
            }
            s.run_group(&fp_i18n::t("msg-step-clear-overrides"), |s| {
                for (id, n) in &items {
                    s.execute("pi.style.apply", json!({"name": n, "ids": [id.0]}))?;
                }
                Ok(())
            })?;
            Ok(no_effect(Value::Null))
        }),
    );
    alias(r, 35831, "panel.styles.breakLink", "Verknüpfung mit Stil aufheben", "pi.style.apply", json!({"name": ""}));
    add_fw(
        r,
        35833,
        cmd("panel.styles.selectUnused").label("Nicht verwendete Stile auswählen").requires(DOC).run(|s, _| {
            let d = s.doc()?;
            let mut used = std::collections::HashSet::new();
            d.doc.walk_elements(&mut |_: &fp_model::WalkCtx, _: &fp_model::ElementPath, e: &Arc<Element>| {
                if let Some(n) = &e.common.style {
                    used.insert(n.clone());
                }
            });
            let unused: Vec<String> = d.doc.styles.iter().flatten().filter(|x| !used.contains(&x.name)).map(|x| x.name.clone()).collect();
            Ok(CommandOutput::value(json!({"names": unused})))
        }),
    );
    add_fw(
        r,
        35295,
        cmd("panel.styles.options")
            .label("Stiloptionen")
            .requires(DOC)
            .param("name", ParamKind::String, "Stilname")
            .opt("newName", ParamKind::String, "neuer Name")
            .opt("flags", ParamKind::Integer { min: Some(0), max: Some(0x1ff) }, "Attributgruppen ZSFi")
            .opt("otherFlags", ParamKind::Integer { min: Some(0), max: Some(0x1f) }, "ZSOi")
            .run(|s, p| {
                let name = p.str("name")?;
                let nn = p.opt_str("newName")?;
                let (f, o) = (p.opt_i64("flags")?, p.opt_i64("otherFlags")?);
                super::with_doc(s, |d| {
                    let doc = Arc::make_mut(&mut d.doc);
                    let st = doc.styles.as_mut().and_then(|v| v.iter_mut().find(|x| x.name == name)).ok_or_else(|| EngineError::NotFound(fp_i18n::tr!("err-nf-style", name = name.as_str())))?;
                    let st = Arc::make_mut(st);
                    if let Some(f) = f {
                        st.flags = f as u32;
                    }
                    if let Some(o) = o {
                        st.other_flags = o as u32;
                    }
                    if let Some(n) = nn.filter(|n| !n.trim().is_empty()) {
                        st.name = n;
                    }
                    Ok(CommandOutput::doc())
                })
            }),
    );
    let load = |replace: bool| {
        move |s: &mut Session, p: &Params| -> Result<CommandOutput> {
            let path = p.str("path")?;
            let styles = read_styles(&path)?;
            let n = styles.len();
            super::with_doc(s, |d| {
                let doc = Arc::make_mut(&mut d.doc);
                let v = doc.styles.get_or_insert_with(Vec::new);
                if replace {
                    v.clear();
                }
                for st in styles {
                    v.retain(|x| x.name != st.name);
                    v.push(st);
                }
                Ok(CommandOutput::doc_with(json!({"styles": n})))
            })
        }
    };
    add_fw(r, 35834, cmd("panel.styles.load").label("Stil laden").requires(DOC).param("path", ParamKind::Path, "Stilbibliothek (.json) oder FW-PNG").run(load(true)));
    add_fw(r, 35836, cmd("panel.styles.importLibrary").label("Stilbibliothek importieren").requires(DOC).param("path", ParamKind::Path, "Stilbibliothek (.json) oder FW-PNG").run(load(false)));
    add_fw(
        r,
        35835,
        cmd("panel.styles.saveLibrary").label("Stilbibliothek speichern").requires(DOC).param("path", ParamKind::Path, "Zieldatei (.json)").run(|s, p| {
            let path = p.str("path")?;
            let v: Vec<fp_model::Style> = s.doc()?.doc.styles.iter().flatten().map(|x| (**x).clone()).collect();
            let json = serde_json::to_vec_pretty(&v).map_err(|e| EngineError::Format(e.to_string()))?;
            write_file(&path, &json)?;
            Ok(CommandOutput::value(json!({"styles": v.len()})))
        }),
    );
    pref_toggle(r, 35297, "panel.styles.largeIcons", "Große Symbole", "StylesOptions_LargePreview", false);
}

// ---------------------------------------------------------------------- Dokumentbibliothek (6937)

fn register_library_menu(r: &mut CommandRegistry) {
    let idx = |b: crate::command::CommandBuilder| b.param("index", ParamKind::Integer { min: Some(0), max: None }, "Symbolindex");
    add_fw(
        r,
        35353,
        cmd("panel.library.newSymbol")
            .label("Neues Symbol")
            .requires(DOC)
            .opt("name", ParamKind::String, "Name")
            .opt_default("kind", ParamKind::Enum { values: vec!["graphic".into(), "animation".into(), "button".into()] }, json!("graphic"), "Typ")
            .opt_default("nineSlice", ParamKind::Bool, json!(false), "Hilfslinien für 9-Slice-Skalierung aktivieren")
            .run(|s, p| {
                let name = p.opt_str("name")?.unwrap_or_else(|| "Symbol".into());
                let kind = match p.opt_str("kind")?.as_deref() {
                    Some("animation") => fp_model::symbol::SymbolKind::Animation,
                    Some("button") => fp_model::symbol::SymbolKind::Button,
                    _ => fp_model::symbol::SymbolKind::Graphic,
                };
                let nine = p.bool_or("nineSlice", false)?;
                super::with_doc(s, |d| {
                    let doc = Arc::make_mut(&mut d.doc);
                    let sid = doc.new_symbol_id();
                    let mut idg = super::take_ids(doc);
                    let mut sym = fp_model::Symbol::new(&mut idg, &name, sid, kind);
                    if nine {
                        set_nine_slice(&mut sym, true);
                    }
                    doc.ids = idg;
                    doc.symbols.push(Arc::new(sym));
                    Ok(CommandOutput::doc_with(json!({"index": doc.symbols.len() - 1})))
                })
            }),
    );
    add_fw(
        r,
        35355,
        idx(cmd("panel.library.duplicate").label("Symbol duplizieren").requires(DOC)).run(|s, p| {
            let i = symbol_index(s, p)?;
            super::with_doc(s, |d| {
                let doc = Arc::make_mut(&mut d.doc);
                let mut sym = (*doc.symbols[i]).clone();
                sym.id = doc.new_symbol_id();
                let mut idg = super::take_ids(doc);
                for sl in &mut sym.root.layers {
                    if let LayerSlot::Own(l) = sl {
                        Arc::make_mut(l).reassign_ids(&mut idg);
                    }
                }
                doc.ids = idg;
                sym.name = fp_i18n::tr!("msg-default-copy", name = sym.name.as_str());
                sym.library_file = None;
                doc.symbols.insert(i + 1, Arc::new(sym));
                Ok(CommandOutput::doc_with(json!({"index": i + 1})))
            })
        }),
    );
    alias(r, 35356, "panel.library.delete", "Symbol löschen", "pi.symbol.delete", json!({}));
    add_fw(
        r,
        35357,
        idx(cmd("panel.library.edit").label("Symbol bearbeiten").requires(DOC)).run(|s, p| {
            let i = symbol_index(s, p)?;
            let sid = s.doc()?.doc.symbols[i].id;
            let d = s.doc()?;
            let inst = docops::selectable_ids(&d.doc, d.page, d.frame).into_iter().find(|id| docops::element_on_page(&d.doc, d.page, *id).is_some_and(|e| matches!(&e.kind, ElementKind::Instance(x) if x.symbol_id == sid)));
            s.run_group(&fp_i18n::t("msg-step-edit-symbol"), |s| {
                match inst {
                    Some(id) => {
                        s.execute("select.set", json!({"ids": [id.0]}))?;
                    }
                    None => {
                        s.execute("pi.symbol.place", json!({"index": i}))?;
                    }
                }
                if s.is_available("modify.symbol.editSymbol") {
                    s.execute("modify.symbol.editSymbol", json!({}))?;
                }
                Ok(())
            })?;
            Ok(no_effect(Value::Null))
        }),
    );
    add_fw(
        r,
        35358,
        idx(cmd("panel.library.properties").label("Symboleigenschaften").requires(DOC))
            .opt("name", ParamKind::String, "Name")
            .opt("kind", ParamKind::Enum { values: vec!["graphic".into(), "animation".into(), "button".into()] }, "Typ")
            .opt("nineSlice", ParamKind::Bool, "Hilfslinien für 9-Slice-Skalierung")
            .run(|s, p| {
                let i = symbol_index(s, p)?;
                let name = p.opt_str("name")?;
                let kind = p.opt_str("kind")?;
                let nine = p.opt_bool("nineSlice")?;
                super::with_doc(s, |d| {
                    let doc = Arc::make_mut(&mut d.doc);
                    let sym = Arc::make_mut(&mut doc.symbols[i]);
                    if let Some(n) = name.filter(|n| !n.trim().is_empty()) {
                        sym.name = n;
                    }
                    if let Some(k) = kind {
                        let k = match k.as_str() {
                            "animation" => fp_model::symbol::SymbolKind::Animation,
                            "button" => fp_model::symbol::SymbolKind::Button,
                            _ => fp_model::symbol::SymbolKind::Graphic,
                        };
                        sym.kind_code = Some(k.code());
                    }
                    if let Some(v) = nine {
                        set_nine_slice(sym, v);
                    }
                    Ok(CommandOutput::doc())
                })
            }),
    );
    add_fw(
        r,
        35839,
        idx(cmd("panel.library.nineSlice").label("Hilfslinien für 9-Slice-Skalierung").requires(DOC)).opt("on", ParamKind::Bool, "ein/aus").run(|s, p| {
            let i = symbol_index(s, p)?;
            let cur = s.doc()?.doc.symbols[i].nine_slice.is_some();
            let v = p.opt_bool("on")?.unwrap_or(!cur);
            super::with_doc(s, |d| {
                set_nine_slice(Arc::make_mut(&mut Arc::make_mut(&mut d.doc).symbols[i]), v);
                Ok(CommandOutput::doc())
            })
        }),
    );
    add_fw(
        r,
        35840,
        idx(cmd("panel.library.lockNineSlice").label("Hilfslinien für 9-Slice-Skalierung sperren").requires(DOC)).opt("on", ParamKind::Bool, "ein/aus").run(|s, p| {
            let i = symbol_index(s, p)?;
            let cur = s.doc()?.doc.symbols[i].nine_slice.as_ref().and_then(|n| n.locked).unwrap_or(false);
            let v = p.opt_bool("on")?.unwrap_or(!cur);
            super::with_doc(s, |d| {
                let sym = Arc::make_mut(&mut Arc::make_mut(&mut d.doc).symbols[i]);
                if sym.nine_slice.is_none() {
                    set_nine_slice(sym, true);
                }
                if let Some(n) = sym.nine_slice.as_mut() {
                    n.locked = Some(v);
                }
                Ok(CommandOutput::doc())
            })
        }),
    );
    add_fw(
        r,
        35820,
        idx(cmd("panel.library.saveToCommon").label("In allgemeiner Bibliothek speichern").requires(DOC)).param("path", ParamKind::Path, "Zieldatei (.png)").run(|s, p| {
            let i = symbol_index(s, p)?;
            let path = p.str("path")?;
            let sym = Arc::clone(&s.doc()?.doc.symbols[i]);
            write_library(s, &path, vec![sym])?;
            Ok(CommandOutput::value(json!({"path": path})))
        }),
    );
    add_fw(
        r,
        35359,
        cmd("panel.library.selectUnused").label("Nicht verwendete Elemente auswählen").requires(DOC).run(|s, _| Ok(CommandOutput::value(json!({"indices": unused_symbols(&s.doc()?.doc)})))),
    );
    add_fw(
        r,
        35360,
        cmd("panel.library.update").label("Aktualisieren").requires(DOC).opt("index", ParamKind::Integer { min: Some(0), max: None }, "Symbolindex (Vorgabe: alle aus Bibliotheken)").run(|s, p| {
            let only = p.opt_i64("index")?.map(|v| v as usize);
            let syms: Vec<(usize, Arc<fp_model::Symbol>)> = s.doc()?.doc.symbols.iter().cloned().enumerate().filter(|(i, x)| x.library_file.is_some() && only.is_none_or(|o| o == *i)).collect();
            if syms.is_empty() {
                return Err(EngineError::Failed(fp_i18n::t("err-not-library-symbol")));
            }
            let mut updated = 0;
            let mut fresh: Vec<(usize, fp_model::Symbol)> = Vec::new();
            for (i, sym) in syms {
                let path = sym.library_file.clone().unwrap_or_default();
                let Ok(bytes) = std::fs::read(&path) else { continue };
                let Ok(lib) = fp_fwpng::read(&bytes) else { continue };
                if let Some(src) = lib.symbols.iter().find(|x| x.name == sym.name) {
                    let mut ns = (**src).clone();
                    ns.id = sym.id;
                    ns.library_file = sym.library_file.clone();
                    fresh.push((i, ns));
                    updated += 1;
                }
            }
            super::with_doc(s, |d| {
                let doc = Arc::make_mut(&mut d.doc);
                let mut idg = super::take_ids(doc);
                for (i, mut ns) in fresh {
                    for sl in &mut ns.root.layers {
                        if let LayerSlot::Own(l) = sl {
                            Arc::make_mut(l).reassign_ids(&mut idg);
                        }
                    }
                    doc.symbols[i] = Arc::new(ns);
                }
                doc.ids = idg;
                Ok(CommandOutput::doc_with(json!({"updated": updated})))
            })
        }),
    );
    pref_toggle(r, 35361, "panel.library.play", "Abspielen", "Firepixel_LibraryPlay", false);
    alias(r, 35362, "panel.library.import", "Symbole importieren", "pi.symbol.import", json!({"place": false}));
    add_fw(
        r,
        35363,
        cmd("panel.library.export").label("Symbole exportieren").requires(DOC).param("path", ParamKind::Path, "Zieldatei (.png)").opt("indices", ParamKind::Json, "Symbolindizes (Vorgabe: alle)").run(|s, p| {
            let path = p.str("path")?;
            let only: Option<Vec<usize>> = p.parse("indices")?;
            let syms: Vec<Arc<fp_model::Symbol>> = s.doc()?.doc.symbols.iter().enumerate().filter(|(i, _)| only.as_ref().is_none_or(|o| o.contains(i))).map(|(_, x)| Arc::clone(x)).collect();
            if syms.is_empty() {
                return Err(EngineError::Failed(fp_i18n::t("err-no-symbols")));
            }
            let n = syms.len();
            write_library(s, &path, syms)?;
            Ok(CommandOutput::value(json!({"path": path, "symbols": n})))
        }),
    );
}

pub(super) fn set_nine_slice(sym: &mut fp_model::Symbol, on: bool) {
    if !on {
        sym.nine_slice = None;
        return;
    }
    if sym.nine_slice.is_some() {
        return;
    }
    // Vorgabe: Hilfslinien bei einem Drittel der Symbolgrenzen (Original: 1/3, 2/3)
    let scratch = fp_model::Document::new(1.0, 1.0, Argb(0));
    let b = sym
        .root
        .layers
        .iter()
        .filter_map(|sl| match sl {
            LayerSlot::Own(l) => l.cels.first().cloned(),
            _ => None,
        })
        .flat_map(|c| c.elements.clone())
        .filter_map(|e| docops::element_bounds(&scratch, &e))
        .reduce(|a, b| a.union(&b));
    let r = b.unwrap_or(Rect::new(0.0, 0.0, 90.0, 90.0));
    let (w, h) = (r.width() as f64, r.height() as f64);
    let guides = vec![r.left as f64 + w / 3.0, r.left as f64 + 2.0 * w / 3.0, r.top as f64 + h / 3.0, r.top as f64 + 2.0 * h / 3.0];
    sym.nine_slice = Some(fp_model::symbol::NineSlice { locked: Some(false), color: None, guides, bounds: Some(r), bounds_extra: Default::default(), view: None, view_extra: Default::default(), extra: Default::default() });
}

// ---------------------------------------------------------------------- Farbfelder (6919), Mischer (6920), Info (6932)

fn register_swatch_menu(r: &mut CommandRegistry) {
    let file_cmd = |replace: bool| {
        move |s: &mut Session, p: &Params| -> Result<CommandOutput> {
            let path = p.str("path")?;
            let mut more = read_swatch_file(&path)?;
            let mut v = if replace { Vec::new() } else { current_swatches(&s.prefs) };
            let n = more.len();
            v.append(&mut more);
            set_custom_swatches(s, &v);
            Ok(CommandOutput::session_with(json!({"colors": n})))
        }
    };
    add_fw(r, 35096, cmd("panel.swatches.add").label("Farbfelder hinzufügen").param("path", ParamKind::Path, "Farbtabelle (ACT, ASE, ACO, GIF)").run(file_cmd(false)));
    add_fw(r, 35097, cmd("panel.swatches.replace").label("Farbfelder ersetzen").param("path", ParamKind::Path, "Farbtabelle (ACT, ASE, ACO, GIF)").run(file_cmd(true)));
    add_fw(
        r,
        35098,
        cmd("panel.swatches.save").label("Farbfelder speichern").param("path", ParamKind::Path, "Zieldatei (.act, .ase, .aco)").run(|s, p| {
            let path = p.str("path")?;
            let v = current_swatches(&s.prefs);
            write_file(&path, &swatch_file_bytes(&ext_of(&path), &v))?;
            Ok(CommandOutput::value(json!({"colors": v.len()})))
        }),
    );
    add_fw(
        r,
        35225,
        cmd("panel.swatches.clear").label("Farbfelder löschen").run(|s, _| {
            set_custom_swatches(s, &[]);
            Ok(CommandOutput::session())
        }),
    );
    pref_choice(r, 35460, "panel.swatches.colorCubes", "Farbwürfel", SWATCH_SET_KEY, 0, 0);
    pref_choice(r, 35461, "panel.swatches.continuousTone", "Kontinuierlicher Farbton", SWATCH_SET_KEY, 5, 0);
    pref_choice(r, 35102, "panel.swatches.windows", "Windows-System", SWATCH_SET_KEY, 4, 0);
    pref_choice(r, 35100, "panel.swatches.macintosh", "Macintosh-System", SWATCH_SET_KEY, 3, 0);
    pref_choice(r, 35099, "panel.swatches.grayscale", "Graustufen", SWATCH_SET_KEY, 2, 0);
    add_fw(
        r,
        35182,
        cmd("panel.swatches.exportPalette").label("Aktuelle Exportpalette").requires(DOC).run(|s, _| {
            let pal = current_export_palette(s)?;
            let v: Vec<Argb> = pal.into_iter().map(|c| Argb(c | 0xff00_0000)).collect();
            set_custom_swatches(s, &v);
            Ok(CommandOutput::session_with(json!({"colors": v.len()})))
        }),
    );
    add_fw(
        r,
        35103,
        cmd("panel.swatches.sortByColor").label("Nach Farbe sortieren").run(|s, _| {
            let v = sort_by_color(&current_swatches(&s.prefs));
            set_custom_swatches(s, &v);
            Ok(CommandOutput::session())
        }),
    );
    r.add(cmd("panel.swatches.addColor").label("Farbfeld hinzufügen").param("color", ParamKind::Color, "Farbe").run(|s, p| {
        let c = p.color("color")?;
        let mut v = current_swatches(&s.prefs);
        v.push(Argb(c.0 | 0xff00_0000));
        set_custom_swatches(s, &v);
        Ok(CommandOutput::session())
    }));
    r.add(cmd("panel.swatches.removeColor").label("Farbfeld löschen").param("index", ParamKind::Integer { min: Some(0), max: None }, "Index").run(|s, p| {
        let i = p.usize("index")?;
        let mut v = current_swatches(&s.prefs);
        if i >= v.len() {
            return Err(EngineError::NotFound(fp_i18n::t("err-nf-swatch")));
        }
        v.remove(i);
        set_custom_swatches(s, &v);
        Ok(CommandOutput::session())
    }));
    r.add(cmd("panel.swatches.setChoice").label("Farbfeldsatz").param("set", ParamKind::Integer { min: Some(0), max: Some(5) }, "0 Farbwürfel, 1 benutzerdefiniert, 2 Graustufen, 3 Mac, 4 Windows, 5 Kontinuierlicher Farbton").run(|s, p| {
        s.prefs.set(SWATCH_SET_KEY, PrefValue::Int(p.i64("set")?.clamp(0, 5)));
        s.prefs_changed();
        Ok(CommandOutput::session())
    }));
    for (id, name, label, v) in [(35105, "panel.mixer.rgb", "RGB", 0), (35106, "panel.mixer.hex", "Hexadezimal", 1), (35108, "panel.mixer.cmy", "CMY", 2), (35107, "panel.mixer.hsb", "HSB", 3), (35109, "panel.mixer.grayscale", "Graustufen", 4)] {
        pref_choice(r, id, name, label, "ColorMixerColorModel", v, 1);
    }
    for (id, name, label, v) in [(35258, "panel.info.rgb", "RGB", 0), (35259, "panel.info.hex", "Hexadezimal", 1), (35260, "panel.info.cmy", "CMY", 2), (35261, "panel.info.hsb", "HSB", 3)] {
        pref_choice(r, id, name, label, "InfoPanelColorModel", v, 1);
    }
    for (id, name, label, v) in [(35262, "panel.info.pixels", "Pixel", 0), (35263, "panel.info.inches", "Zoll", 1), (35264, "panel.info.centimeters", "Zentimeter", 2)] {
        pref_choice(r, id, name, label, "InfoPanelRulerUnits", v, 0);
    }
    pref_toggle(r, 35280, "panel.info.scaleAttributes", "Attribute skalieren", "Firepixel_InfoScaleAttributes", true);
}

// ---------------------------------------------------------------------- Optimieren (6941)

/// Exporteinstellungen der Seite als `OptimizeSettings` ändern (Palettenbearbeitung über
/// `fp_export::palette_edit`) und zurückschreiben – ein Verlaufsschritt.
fn edit_opt(s: &mut Session, f: impl FnOnce(&mut fp_export::OptimizeSettings)) -> Result<CommandOutput> {
    let d = s.doc()?;
    let mut o = fp_export::doc_export::page_settings(&d.doc, d.page);
    f(&mut o);
    super::with_doc(s, |d| {
        store_settings(d, &o)?;
        Ok(CommandOutput::doc())
    })
}

fn opt_entry(o: &fp_export::OptimizeSettings, c: u32) -> Option<&fp_export::options::PaletteColor> {
    o.palette.iter().find(|e| e.color | 0xff00_0000 == c | 0xff00_0000)
}

fn register_optimize_menu(r: &mut CommandRegistry) {
    add_fw(
        r,
        35417,
        cmd("panel.optimize.saveSettings").label("Einstellungen speichern").requires(DOC).param("name", ParamKind::String, "Name der Vorgabe").run(|s, p| {
            let name = p.str("name")?;
            if name.trim().is_empty() {
                return Err(bad(p, fp_i18n::t("err-param-name-missing")));
            }
            let e = export_of(s);
            let mut m = match s.prefs.get(EXPORT_PRESETS_KEY) {
                Some(PrefValue::Dict(m)) => m,
                _ => Default::default(),
            };
            m.insert(name.trim().to_string(), PrefValue::Str(serde_json::to_string(&e).map_err(|x| EngineError::Format(x.to_string()))?));
            s.prefs.set(EXPORT_PRESETS_KEY, PrefValue::Dict(m));
            s.prefs_changed();
            Ok(CommandOutput::session())
        }),
    );
    add_fw(
        r,
        35418,
        cmd("panel.optimize.deleteSettings").label("Einstellungen löschen").param("name", ParamKind::String, "Name der Vorgabe").run(|s, p| {
            let name = p.str("name")?;
            let mut m = match s.prefs.get(EXPORT_PRESETS_KEY) {
                Some(PrefValue::Dict(m)) => m,
                _ => Default::default(),
            };
            if m.remove(&name).is_none() {
                return Err(EngineError::NotFound(fp_i18n::tr!("err-nf-preset", name = name.as_str())));
            }
            s.prefs.set(EXPORT_PRESETS_KEY, PrefValue::Dict(m));
            s.prefs_changed();
            Ok(CommandOutput::session())
        }),
    );
    r.add(cmd("panel.optimize.applySettings").label("Gespeicherte Einstellungen").requires(DOC).param("name", ParamKind::String, "Name der Vorgabe").run(|s, p| {
        let name = p.str("name")?;
        let e = export_presets(&s.prefs).into_iter().find(|(n, _)| *n == name).map(|(_, e)| e).ok_or_else(|| EngineError::NotFound(fp_i18n::tr!("err-nf-preset", name = name.as_str())))?;
        let d = s.doc_mut()?;
        let frames = d.doc.pages.get(d.page).and_then(|p| p.root.export.as_ref()).map(|x| x.frames.clone()).unwrap_or_default();
        let mut e = e;
        e.frames = frames;
        page_mut(d)?.root.export = Some(Box::new(e));
        Ok(CommandOutput::doc())
    }));
    add_fw(
        r,
        35419,
        cmd("panel.optimize.optimizeToSize").label("Optimieren für Dateigröße").requires(DOC).param("kb", ParamKind::Number { min: Some(0.1), max: Some(100000.0) }, "Zielgröße in KB").run(|s, p| {
            let target = (p.f64("kb")? * 1024.0).round().max(1.0) as usize;
            let (img, st, _) = export_current(s, None)?;
            let d = s.doc()?;
            let ctx = fp_export::doc_export::page_context(&d.doc, d.page);
            let res = fp_export::size::optimize_to_size(&img, &st, target, &ctx, &fp_export::export::EncodeExtras::standalone()).map_err(|e| EngineError::Failed(e.to_string()))?;
            if !res.changed {
                return Err(EngineError::Failed(fp_i18n::t("err-no-size-optimize")));
            }
            let bytes = res.bytes;
            super::with_doc(s, |d| {
                store_settings(d, &res.settings)?;
                Ok(CommandOutput::doc_with(json!({"bytes": bytes})))
            })
        }),
    );
    add_fw(
        r,
        35421,
        cmd("panel.optimize.exportWizard")
            .label("Exportassistent")
            .requires(DOC)
            .opt_default("target", ParamKind::Enum { values: vec!["web".into(), "imageEditing".into(), "dreamweaver".into()] }, json!("web"), "Ziel")
            .opt("kb", ParamKind::Number { min: Some(0.1), max: Some(100000.0) }, "Zielgröße in KB")
            .run(|s, p| {
                // Analyse (4720): Fotos (viele Farben) → JPEG, sonst GIF adaptiv; Bildbearbeitung → PNG 32
                let target = p.opt_str("target")?.unwrap_or_else(|| "web".into());
                let (img, cur, _) = export_current(s, None)?;
                let mut st = cur.clone();
                if target == "imageEditing" {
                    st.format = fp_export::ExportFormat::Png;
                    st.color_mode = fp_export::ColorMode::Rgba32;
                } else {
                    let mut colors = std::collections::HashSet::new();
                    'outer: for y in img.bounds().top..img.bounds().bottom {
                        for x in img.bounds().left..img.bounds().right {
                            let px = img.pixel(x, y);
                            colors.insert((px.r, px.g, px.b));
                            if colors.len() > 256 {
                                break 'outer;
                            }
                        }
                    }
                    if colors.len() > 256 {
                        st.format = fp_export::ExportFormat::Jpeg;
                        st.color_mode = fp_export::ColorMode::Rgb24;
                        st.jpeg.quality = 80;
                    } else {
                        st.format = fp_export::ExportFormat::Gif;
                        st.color_mode = fp_export::ColorMode::Indexed;
                        st.palette_kind = fp_export::PaletteKind::Adaptive;
                        st.max_colors = 256;
                    }
                }
                let mut bytes = None;
                if let Some(kb) = p.opt_f64("kb")? {
                    let d = s.doc()?;
                    let ctx = fp_export::doc_export::page_context(&d.doc, d.page);
                    let res = fp_export::size::optimize_to_size(&img, &st, (kb * 1024.0) as usize, &ctx, &fp_export::export::EncodeExtras::standalone()).map_err(|e| EngineError::Failed(e.to_string()))?;
                    st = res.settings;
                    bytes = Some(res.bytes);
                }
                let fmt = format!("{:?}", st.format);
                super::with_doc(s, |d| {
                    store_settings(d, &st)?;
                    Ok(CommandOutput::doc_with(json!({"format": fmt, "bytes": bytes})).labeled(fp_i18n::t("msg-step-export-wizard")))
                })
            }),
    );
    let fmt_toggle = |r: &mut CommandRegistry, id: u32, name: &str, label: &str, get: fn(&fp_model::ExportSettings) -> bool, set: fn(&mut fp_model::ExportSettings, bool)| {
        let c = cmd(name).label(label).requires(DOC).opt("on", ParamKind::Bool, "ein/aus (Vorgabe: umschalten)").checked(move |s| s.active().is_some() && get(&export_of(s))).run(move |s, p| {
            let v = p.opt_bool("on")?.unwrap_or(!get(&export_of(s)));
            let d = s.doc_mut()?;
            set(export_mut(d)?, v);
            Ok(CommandOutput::doc())
        });
        add_fw(r, id, c);
    };
    fmt_toggle(r, 35397, "panel.optimize.interlaced", "Mit Zeilensprung", |e| e.interlaced, |e, v| e.interlaced = v);
    // JPEG-Optionen nur bei JPEG, Palettenbefehle nur bei Formaten mit Farbtabelle: sonst grau
    // und ohne Häkchen wie im Original (MN-3, PARITAET_B §3); Skripte dürfen sie weiter setzen
    let only_when = |r: &mut CommandRegistry, id: u32, name: &str, label: &str, when: fn(&fp_model::ExportSettings) -> bool, get: fn(&fp_model::ExportSettings) -> bool, set: fn(&mut fp_model::ExportSettings, bool)| {
        let c = cmd(name)
            .label(label)
            .requires(DOC)
            .opt("on", ParamKind::Bool, "ein/aus (Vorgabe: umschalten)")
            .menu_when(move |s| when(&export_of(s)))
            .checked(move |s| s.active().is_some() && when(&export_of(s)) && get(&export_of(s)))
            .run(move |s, p| {
                let v = p.opt_bool("on")?.unwrap_or(!get(&export_of(s)));
                let d = s.doc_mut()?;
                set(export_mut(d)?, v);
                Ok(CommandOutput::doc())
            });
        add_fw(r, id, c);
    };
    only_when(r, 35398, "panel.optimize.progressive", "Progressives JPEG", is_jpeg, |e| e.jpeg_progressive, |e, v| e.jpeg_progressive = v);
    // „JPEG-Ränder scharfzeichnen“ = keine Farbunterabtastung (4:4:4, `jpegSubsampling = 0`)
    only_when(r, 35399, "panel.optimize.sharpenJpegEdges", "JPEG-Ränder scharfzeichnen", is_jpeg, |e| e.jpeg_subsampling == 0, |e, v| e.jpeg_subsampling = if v { 0 } else { 1 });
    only_when(r, 35396, "panel.optimize.removeUnusedColors", "Nicht verwendete Farben entfernen", has_palette, |e| e.optimized, |e, v| e.optimized = v);
    add_fw(r, 35595, cmd("panel.optimize.unlockAll").label("Sperrung aller Farben aufheben").requires(DOC).menu_when(|s| has_palette(&export_of(s))).run(|s, _| edit_opt(s, fp_export::palette_edit::unlock_all)));
    let colors_cmd = |name: &str, label: &str| cmd(name).label(label).requires(DOC).menu_when(|s| has_palette(&export_of(s))).param("colors", ParamKind::Json, "Palettenfarben [\"#rrggbb\", …]");
    add_fw(
        r,
        35209,
        cmd("panel.optimize.editColor").label("Paletteneintrag ersetzen").requires(DOC).menu_when(|s| has_palette(&export_of(s))).param("color", ParamKind::Color, "Palettenfarbe").param("newColor", ParamKind::Color, "neue Farbe").run(|s, p| {
            let (c, n) = (p.color("color")?.0 | 0xff00_0000, p.color("newColor")?.0 | 0xff00_0000);
            edit_opt(s, |o| fp_export::palette_edit::edit_color(o, c, Some(n)))
        }),
    );
    add_fw(
        r,
        35235,
        colors_cmd("panel.optimize.makeTransparent", "Transparent machen").opt("on", ParamKind::Bool, "transparent (Vorgabe: umschalten)").run(|s, p| {
            let cols = color_list(p, "colors")?;
            let on = p.opt_bool("on")?;
            edit_opt(s, |o| {
                for c in cols {
                    let v = on.unwrap_or(!opt_entry(o, c).is_some_and(|e| e.transparent));
                    fp_export::palette_edit::set_transparent(o, c, v);
                }
            })
        }),
    );
    add_fw(
        r,
        35234,
        colors_cmd("panel.optimize.removeEdit", "Bearbeitung entfernen").run(|s, p| {
            let cols = color_list(p, "colors")?;
            edit_opt(s, |o| {
                for c in cols {
                    fp_export::palette_edit::edit_color(o, c, None);
                }
            })
        }),
    );
    add_fw(
        r,
        35223,
        cmd("panel.optimize.removeAllEdits").label("Alle Bearbeitungen entfernen").requires(DOC).menu_when(|s| has_palette(&export_of(s))).run(|s, _| {
            edit_opt(s, |o| {
                let edited: Vec<u32> = o.palette.iter().filter(|e| e.edited.is_some()).map(|e| e.color).collect();
                for c in edited {
                    fp_export::palette_edit::edit_color(o, c, None);
                }
            })
        }),
    );
    r.add(colors_cmd("panel.optimize.lockColors", "Farbe sperren").opt("on", ParamKind::Bool, "sperren (Vorgabe: umschalten)").run(|s, p| {
        let cols = color_list(p, "colors")?;
        let on = p.opt_bool("on")?;
        edit_opt(s, |o| {
            for c in cols {
                let v = on.unwrap_or(!opt_entry(o, c).is_some_and(|e| e.locked));
                fp_export::palette_edit::set_locked(o, c, v);
            }
        })
    }));
    r.add(colors_cmd("panel.optimize.snapToWeb", "Websicher machen").run(|s, p| {
        let cols = color_list(p, "colors")?;
        edit_opt(s, |o| {
            for c in cols {
                fp_export::palette_edit::snap_to_web(o, c);
            }
        })
    }));
    r.add(cmd("panel.optimize.addColor").label("Farbe hinzufügen").requires(DOC).param("color", ParamKind::Color, "Farbe").run(|s, p| {
        let c = p.color("color")?.0 | 0xff00_0000;
        edit_opt(s, |o| fp_export::palette_edit::add_color(o, c))
    }));
    r.add(colors_cmd("panel.optimize.deleteColors", "Farbe löschen").run(|s, p| {
        let cols = color_list(p, "colors")?;
        // Löschen wandelt in eine benutzerdefinierte Palette um (braucht die aktuelle Palette)
        let (img, o0, _) = export_current(s, None)?;
        let ctx = { let d = s.doc()?; fp_export::doc_export::page_context(&d.doc, d.page) };
        let current = fp_export::palette_edit::current_palette(&img, &o0, &ctx);
        edit_opt(s, |o| {
            for c in cols {
                fp_export::palette_edit::delete_color(o, &current, c);
            }
        })
    }));
    add_fw(
        r,
        35203,
        cmd("panel.optimize.loadPalette").label("Palette laden").requires(DOC).param("path", ParamKind::Path, "Farbtabelle (ACT, GIF, ASE)").run(|s, p| {
            let path = p.str("path")?;
            let cols: Vec<[u8; 3]> = read_swatch_file(&path)?.iter().take(256).map(|c| [c.r(), c.g(), c.b()]).collect();
            edit_opt(s, |o| fp_export::palette_edit::load_palette(o, &cols))
        }),
    );
    add_fw(
        r,
        35204,
        cmd("panel.optimize.savePalette").label("Palette speichern").requires(DOC).param("path", ParamKind::Path, "Zieldatei (.act)").run(|s, p| {
            let path = p.str("path")?;
            let pal: Vec<Argb> = current_export_palette(s)?.into_iter().map(Argb).collect();
            write_file(&path, &swatch_file_bytes(&ext_of(&path), &pal))?;
            Ok(CommandOutput::value(json!({"colors": pal.len()})))
        }),
    );
    pref_toggle(r, 35236, "panel.optimize.swatchFeedback", "Farbfeld-Rückmeldung anzeigen", "Firepixel_OptimizeSwatchFeedback", true);
}
#[cfg(test)]
mod tests {
    use super::*;

    fn doc_with_rect() -> Session {
        let mut s = Session::default();
        let _ = s.execute("file.new", json!({"width": 200, "height": 100}));
        let _ = s.execute("draw.rectangle", json!({"rect": [10, 10, 110, 60]}));
        s
    }

    fn attrs(s: &Session) -> Arc<fp_model::PathAttrs> {
        let d = s.active().expect("Dokument");
        let e = docops::element_on_page(&d.doc, d.page, d.selection.elements[0]).expect("Element");
        docops::path_attrs_of(&e).expect("Attribute")
    }

    #[test]
    fn fill_gradient_and_undo() {
        let mut s = doc_with_rect();
        let before = s.active().map(|d| d.history.undo_count()).unwrap_or(0);
        s.execute("pi.fill", json!({"category": "gradient", "shape": 3})).expect("pi.fill");
        let a = attrs(&s);
        assert_eq!(fill_kind(a.fill.as_ref()), 2);
        assert_eq!(a.fill.as_ref().map(|f| f.shape), Some(3));
        assert_eq!(s.active().map(|d| d.history.undo_count()), Some(before + 1));
        s.undo().expect("undo");
        assert_ne!(fill_kind(attrs(&s).fill.as_ref()), 2);
    }

    #[test]
    fn stroke_preset_width_and_roundness() {
        let mut s = Session::default();
        let _ = s.execute("file.new", json!({"width": 200, "height": 100}));
        let _ = s.execute("draw.rectangle", json!({"rect": [10, 10, 110, 60]}));
        s.execute("pi.stroke", json!({"preset": "Bleistift/1-Pixel weich", "width": 3, "softness": 20})).expect("stroke");
        let a = attrs(&s);
        let b = a.brush.as_ref().expect("Pinsel");
        assert_eq!(b.diameter, 3);
        assert_eq!(b.softness, 200);
        assert_eq!(brush_label(b), "1-Pixel weich");
        s.execute("pi.roundness", json!({"value": 50})).expect("roundness");
        let d = s.active().expect("doc");
        let e = docops::element_on_page(&d.doc, d.page, d.selection.elements[0]).expect("e");
        if let ElementKind::Path(p) = &e.kind {
            assert!((p.primitive.as_ref().map(|x| x.roundness).unwrap_or(0.0) - 0.5).abs() < 1e-6);
            assert!(p.contours[0].nodes.len() == 8);
        }
    }

    #[test]
    fn compound_shape_from_two_rects() {
        let mut s = doc_with_rect();
        let first = s.active().map(|d| d.selection.elements[0]).expect("id");
        let _ = s.execute("draw.rectangle", json!({"rect": [60, 30, 150, 90]}));
        let second = s.active().map(|d| d.selection.elements[0]).expect("id");
        let _ = s.execute("select.set", json!({"ids": [first.0, second.0]}));
        s.execute("pi.compound.setOp", json!({"op": 3})).expect("compound");
        let d = s.active().expect("doc");
        assert_eq!(d.selection.elements.len(), 1);
        let e = docops::element_on_page(&d.doc, d.page, d.selection.elements[0]).expect("e");
        assert!(matches!(e.kind, ElementKind::CompoundShape(_)));
        s.execute("pi.compound.combine", json!({})).expect("combine");
        let d = s.active().expect("doc");
        let e = docops::element_on_page(&d.doc, d.page, d.selection.elements[0]).expect("e");
        assert!(matches!(e.kind, ElementKind::Path(_)));
    }

    // ------------------------------------------------------------------ Bedienfeldmenüs

    fn doc_arc(s: &Session) -> Arc<fp_model::Document> {
        Arc::clone(&s.active().expect("doc").doc)
    }

    /// Befehl ändert das Dokument, Rückgängig stellt exakt her.
    fn changes_and_undoes(s: &mut Session, name: &str, params: Value) -> Value {
        let before = doc_arc(s);
        let out = s.execute(name, params).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(!Arc::ptr_eq(&before, &doc_arc(s)), "{name} hat das Dokument nicht geändert");
        s.undo().unwrap_or_else(|e| panic!("{name} undo: {e}"));
        assert!(Arc::ptr_eq(&before, &doc_arc(s)) || *before == *doc_arc(s), "{name}: Rückgängig stellt nicht her");
        s.redo().expect("redo");
        out.value
    }

    fn tmp(name: &str) -> String {
        let d = std::env::temp_dir().join("fp-ibd-panel-tests");
        let _ = std::fs::create_dir_all(&d);
        d.join(name).to_string_lossy().to_string()
    }

    fn own_layer_count(s: &Session) -> usize {
        let d = s.active().expect("doc");
        docops::own_layers(&d.doc, d.page).len()
    }

    #[test]
    fn every_panel_menu_entry_has_a_command() {
        let r = CommandRegistry::with_defaults();
        let ids: &[u32] = &[
            // Ebenen 6923
            35439, 35159, 35794, 35160, 35205, 35167, 35162, 35816, 35799, 35800, 35805, 35796, 35584, 35585, 35163, 35164, 35165, 35166, 35441, 35442, 35443, 35444, 35440,
            // Status 6924
            35168, 35169, 35170, 35171, 35172, 35383, 35852, 35384, 35385, // Seiten 7026
            35801, 35806, 35802, 35803, 35818, 35813, 35814, 35823, 35853, 35804, // Verlauf 6939
            35378, 35379, 35380, 35381, // Stile 6935
            35294, 35829, 35296, 35830, 35338, 35832, 35831, 35833, 35295, 35834, 35836, 35835, 35297, // Farbfelder 6919
            35096, 35097, 35098, 35225, 35460, 35461, 35102, 35100, 35099, 35182, 35103, // Mischer 6920, Info 6932
            35105, 35106, 35108, 35107, 35109, 35259, 35258, 35260, 35261, 35262, 35263, 35264, 35280, // Optimieren 6941
            35417, 35418, 35419, 35421, 35397, 35398, 35399, 35595, 35396, 35209, 35235, 35236, 35234, 35223, 35203, 35204, // Bibliothek 6937
            35353, 35355, 35356, 35357, 35358, 35839, 35840, 35820, 35359, 35360, 35361, 35362, 35363,
            // Inspektor-Effektmenü 6921, Löschmethode 6922, Zwiebelschicht 6933, Schleife 6940, Palette 6942
            35148, 35350, 35351, 35352, 35155, 35156, 35157, 35158, 35282, 35283, 35284, 35322, 35281, 35285, 35387, 35389, 35390, 35391, 35392, 35393, 35394, 35395, 35388, 35409, 35208, 35202, 35192, 35224, 35218, 35219, 35220, 35221,
        ];
        let missing: Vec<u32> = ids.iter().copied().filter(|id| r.by_fw_id(*id).is_none()).collect();
        assert!(missing.is_empty(), "ohne Befehl: {missing:?}");
        for id in ids {
            let c = r.by_fw_id(*id).expect("Befehl");
            assert!(c.info().implemented, "{} nicht umgesetzt", c.info().name);
        }
    }

    #[test]
    fn layer_menu_commands() {
        let mut s = doc_with_rect();
        let n0 = own_layer_count(&s);
        changes_and_undoes(&mut s, "panel.layers.newLayer", json!({"name": "Oben", "shared": true}));
        assert_eq!(own_layer_count(&s), n0 + 1);
        let d = s.active().expect("doc");
        let (_, l) = docops::own_layers(&d.doc, d.page).into_iter().find(|(_, l)| l.name == "Oben").expect("neue Ebene");
        assert!(l.shared_across_frames);
        assert_eq!(d.layer, Some(l.id));
        // Ebene duplizieren: 3 Kopien ganz unten
        changes_and_undoes(&mut s, "panel.layers.duplicate", json!({"count": 3, "position": "bottom"}));
        assert_eq!(own_layer_count(&s), n0 + 4);
        let d = s.active().expect("doc");
        let names: Vec<String> = docops::own_layers(&d.doc, d.page).into_iter().map(|(_, l)| l.name.clone()).collect();
        assert_eq!(&names[..3], &["Oben Kopie", "Oben Kopie", "Oben Kopie"]);
        // Statusübergreifend umschalten
        assert_eq!(s.command_state("panel.layers.shareAcrossStates").1, Some(true));
        // Alles aus-/einblenden, sperren
        changes_and_undoes(&mut s, "panel.layers.hideAll", json!({}));
        let d = s.active().expect("doc");
        assert!(docops::own_layers(&d.doc, d.page).iter().all(|(_, l)| !l.is_visible()));
        s.execute("panel.layers.showAll", json!({})).expect("showAll");
        changes_and_undoes(&mut s, "panel.layers.lockAll", json!({}));
        let d = s.active().expect("doc");
        assert!(docops::own_layers(&d.doc, d.page).iter().all(|(_, l)| l.cels.iter().all(|c| c.locked)));
        s.execute("panel.layers.unlockAll", json!({})).expect("unlockAll");
        // Einzelebene bearbeiten (Voreinstellung)
        s.execute("panel.layers.singleLayerEditing", json!({})).expect("single");
        assert!(s.prefs.get_bool("SingleLayerEditing", false));
        assert_eq!(s.command_state("panel.layers.singleLayerEditing").1, Some(true));
        s.execute("panel.layers.thumbnailOptions", json!({"size": 64})).expect("thumbs");
        assert_eq!(s.prefs.get_int("LayersList_ThumbnailSize", 0), 64);
        // Neues Bild über das Bedienfeldmenü
        changes_and_undoes(&mut s, "panel.layers.newBitmap", json!({}));
    }

    #[test]
    fn layers_shared_across_pages() {
        let mut s = doc_with_rect();
        s.execute("edit.insert.page", json!({})).expect("Seite");
        s.execute("page.setCurrent", json!({"index": 0})).expect("Seite 1");
        changes_and_undoes(&mut s, "panel.layers.shareToAllPages", json!({}));
        let d = s.active().expect("doc");
        assert_eq!(d.doc.shared_layers.len(), 1);
        for p in &d.doc.pages {
            assert!(p.root.layers.iter().any(|l| matches!(l, LayerSlot::Shared(_))), "Verweis auf jeder Seite");
        }
        assert!(s.is_available("panel.layers.detachShared"));
        changes_and_undoes(&mut s, "panel.layers.detachShared", json!({}));
        let d = s.active().expect("doc");
        assert!(d.doc.shared_layers.is_empty());
        assert!(d.doc.pages.iter().all(|p| !p.root.layers.iter().any(|l| matches!(l, LayerSlot::Shared(_)))));
        // nur für Seite 2, dann auf Seite 2 ausschließen
        s.execute("panel.layers.shareToPages", json!({"pages": [1]})).expect("freigeben");
        s.execute("page.setCurrent", json!({"index": 1})).expect("Seite 2");
        let d = s.active().expect("doc");
        let shared_id = d.doc.shared_layers[0].layer.as_ref().map(|l| l.id).expect("Ebene");
        s.doc_mut().expect("doc").layer = Some(shared_id);
        changes_and_undoes(&mut s, "panel.layers.excludeShared", json!({}));
        let d = s.active().expect("doc");
        assert!(!d.doc.pages[1].root.layers.iter().any(|l| matches!(l, LayerSlot::Shared(_))));
    }

    #[test]
    fn state_and_page_menu_commands() {
        let mut s = doc_with_rect();
        changes_and_undoes(&mut s, "panel.states.add", json!({"count": 2, "where": "start"}));
        assert_eq!(s.active().map(|d| d.frame_count()), Some(3));
        assert_eq!(s.active().map(|d| d.frame), Some(0));
        s.execute("state.setCurrent", json!({"index": 2})).expect("Status 3");
        let elems = |s: &Session, f: usize| {
            let d = s.active().expect("doc");
            docops::own_layers(&d.doc, d.page).iter().map(|(_, l)| l.cels.get(f).map(|c| c.elements.len()).unwrap_or(0)).sum::<usize>()
        };
        assert_eq!(elems(&s, 2), 1);
        changes_and_undoes(&mut s, "panel.states.duplicate", json!({"count": 2, "where": "end"}));
        assert_eq!(s.active().map(|d| d.frame_count()), Some(5));
        assert_eq!(elems(&s, 3), 1);
        assert_eq!(elems(&s, 4), 1);
        let before = s.command_state("panel.states.autoCrop").1;
        changes_and_undoes(&mut s, "panel.states.autoCrop", json!({}));
        assert_ne!(s.command_state("panel.states.autoCrop").1, before);
        changes_and_undoes(&mut s, "panel.states.properties", json!({"delay": 50, "name": "Anfang"}));
        // Seiten
        s.execute("edit.insert.page", json!({})).expect("Seite");
        changes_and_undoes(&mut s, "panel.pages.setMaster", json!({}));
        assert_eq!(s.active().map(|d| d.doc.pages[0].is_master), Some(Some(true)));
        s.execute("page.setCurrent", json!({"index": 1})).expect("Seite 2");
        assert!(s.is_available("panel.layers.addMasterPageLayer"));
        changes_and_undoes(&mut s, "panel.pages.linkToMaster", json!({}));
        assert_eq!(s.command_state("panel.pages.linkToMaster").1, Some(true));
        changes_and_undoes(&mut s, "panel.pages.linkToMaster", json!({}));
        assert_eq!(s.command_state("panel.pages.linkToMaster").1, Some(false));
        changes_and_undoes(&mut s, "panel.pages.resetMaster", json!({}));
        assert!(s.active().is_some_and(|d| d.doc.pages.iter().all(|p| p.is_master != Some(true))));
        changes_and_undoes(&mut s, "panel.pages.rename", json!({"name": "Zweite"}));
        assert_eq!(s.active().and_then(|d| d.doc.pages[1].name.clone()).as_deref(), Some("Zweite"));
        let path = tmp("seite.png");
        let out = s.execute("panel.pages.export", json!({"path": path})).expect("export");
        assert_eq!(out.value["files"].as_array().map(|a| a.len()), Some(1));
        assert!(std::fs::metadata(&path).map(|m| m.len() > 0).unwrap_or(false));
    }

    #[test]
    fn history_menu_commands() {
        let mut s = doc_with_rect();
        s.execute("edit.move", json!({"dx": 5, "dy": 0})).expect("move");
        let n = s.active().map(|d| d.history.undo_count()).unwrap_or(0);
        let x0 = docops::ids_bounds(&doc_arc(&s), 0, &s.active().expect("d").selection.elements).map(|b| b.left).unwrap_or(0.0);
        s.execute("panel.history.replaySelected", json!({"steps": [n - 1]})).expect("replay");
        let x1 = docops::ids_bounds(&doc_arc(&s), 0, &s.active().expect("d").selection.elements).map(|b| b.left).unwrap_or(0.0);
        assert!((x1 - x0 - 5.0).abs() < 1e-3, "{x0} → {x1}");
        assert_eq!(s.active().map(|d| d.history.undo_count()), Some(n + 1));
        let v = s.execute("panel.history.copySteps", json!({"steps": [n - 1]})).expect("copy").value;
        assert!(v["script"].as_str().is_some_and(|t| t.contains("edit.move")));
        let path = tmp("schritte.jsf");
        s.execute("panel.history.saveAsCommand", json!({"from": 0, "path": path})).expect("save");
        assert!(std::fs::read_to_string(&path).is_ok_and(|t| t.contains("edit.move")));
        s.execute("panel.history.clear", json!({})).expect("clear");
        assert_eq!(s.active().map(|d| d.history.undo_count()), Some(0));
    }

    #[test]
    fn style_menu_commands() {
        let mut s = doc_with_rect();
        s.execute("pi.fill", json!({"color": "#ff0000"})).expect("rot");
        changes_and_undoes(&mut s, "panel.styles.new", json!({"name": "Rot"}));
        changes_and_undoes(&mut s, "panel.styles.duplicate", json!({"name": "Rot"}));
        let names = |s: &Session| s.active().and_then(|d| d.doc.styles.clone()).unwrap_or_default().iter().map(|x| x.name.clone()).collect::<Vec<_>>();
        assert_eq!(names(&s), vec!["Rot", "Rot Kopie"]);
        changes_and_undoes(&mut s, "panel.styles.options", json!({"name": "Rot Kopie", "newName": "Kopie", "flags": 4}));
        assert_eq!(s.active().and_then(|d| d.doc.styles.as_ref().and_then(|v| v.iter().find(|x| x.name == "Kopie").map(|x| x.flags))), Some(4));
        let unused = s.execute("panel.styles.selectUnused", json!({})).expect("unbenutzt").value;
        assert_eq!(unused["names"], json!(["Kopie"]));
        // Abweichung erzeugen, löschen
        s.execute("pi.fill", json!({"color": "#00ff00"})).expect("grün");
        changes_and_undoes(&mut s, "panel.styles.clearOverrides", json!({}));
        let col = attrs(&s).fill_color;
        assert_eq!((col.r(), col.g()), (255, 0));
        changes_and_undoes(&mut s, "panel.styles.breakLink", json!({}));
        // neu definieren
        s.execute("pi.style.apply", json!({"name": "Rot"})).expect("apply");
        s.execute("pi.fill", json!({"color": "#0000ff"})).expect("blau");
        changes_and_undoes(&mut s, "panel.styles.redefine", json!({"name": "Rot"}));
        // Bibliothek speichern und laden
        let path = tmp("stile.json");
        s.execute("panel.styles.saveLibrary", json!({"path": path})).expect("speichern");
        changes_and_undoes(&mut s, "panel.styles.delete", json!({"names": ["Rot", "Kopie"]}));
        assert!(names(&s).is_empty());
        changes_and_undoes(&mut s, "panel.styles.load", json!({"path": path}));
        assert_eq!(names(&s), vec!["Rot", "Kopie"]);
        s.execute("panel.styles.largeIcons", json!({"on": true})).expect("groß");
        assert_eq!(s.prefs.get_int("StylesOptions_LargePreview", 0), 1);
    }

    #[test]
    fn library_menu_commands() {
        let mut s = doc_with_rect();
        let v = changes_and_undoes(&mut s, "panel.library.newSymbol", json!({"name": "Leer", "kind": "button"}));
        assert_eq!(v["index"], json!(0));
        s.execute("modify.symbol.convertToSymbol", json!({"name": "Kasten"})).expect("Symbol");
        changes_and_undoes(&mut s, "panel.library.duplicate", json!({"index": 1}));
        let syms = |s: &Session| s.active().map(|d| d.doc.symbols.iter().map(|x| x.name.clone()).collect::<Vec<_>>()).unwrap_or_default();
        assert_eq!(syms(&s), vec!["Leer", "Kasten", "Kasten Kopie"]);
        changes_and_undoes(&mut s, "panel.library.properties", json!({"index": 2, "name": "Zweit", "kind": "animation", "nineSlice": true}));
        let d = s.active().expect("doc");
        assert_eq!(d.doc.symbols[2].kind(), fp_model::SymbolKind::Animation);
        assert_eq!(d.doc.symbols[2].nine_slice.as_ref().map(|n| n.guides.len()), Some(4));
        changes_and_undoes(&mut s, "panel.library.lockNineSlice", json!({"index": 2}));
        changes_and_undoes(&mut s, "panel.library.nineSlice", json!({"index": 2}));
        assert!(s.active().is_some_and(|d| d.doc.symbols[2].nine_slice.is_none()));
        let unused = s.execute("panel.library.selectUnused", json!({})).expect("unbenutzt").value;
        assert_eq!(unused["indices"], json!([0, 2]));
        let path = tmp("symbole.png");
        s.execute("panel.library.export", json!({"path": path, "indices": [1]})).expect("export");
        changes_and_undoes(&mut s, "panel.library.import", json!({"path": path}));
        assert_eq!(syms(&s).last().map(|x| x.as_str()), Some("Kasten"));
        changes_and_undoes(&mut s, "panel.library.delete", json!({"index": 0}));
        let common = tmp("gemeinsam.png");
        s.execute("panel.library.saveToCommon", json!({"index": 0, "path": common})).expect("allgemein");
        assert!(std::fs::metadata(&common).is_ok());
    }

    #[test]
    fn swatch_mixer_info_commands() {
        let mut s = Session::default();
        s.execute("panel.swatches.windows", json!({})).expect("Windows");
        assert_eq!(current_swatches(&s.prefs).len(), 256);
        assert_eq!(s.command_state("panel.swatches.windows").1, Some(true));
        s.execute("panel.swatches.addColor", json!({"color": "#123456"})).expect("add");
        assert_eq!(s.prefs.get_int(SWATCH_SET_KEY, 0), 1);
        let v = current_swatches(&s.prefs);
        assert_eq!(v.len(), 257);
        assert_eq!(v.last().copied(), Some(Argb(0xff12_3456)));
        s.execute("panel.swatches.removeColor", json!({"index": 256})).expect("remove");
        assert_eq!(current_swatches(&s.prefs).len(), 256);
        let path = tmp("farben.aco");
        s.execute("panel.swatches.save", json!({"path": path})).expect("save");
        s.execute("panel.swatches.clear", json!({})).expect("clear");
        assert!(current_swatches(&s.prefs).is_empty());
        s.execute("panel.swatches.replace", json!({"path": path})).expect("replace");
        assert_eq!(current_swatches(&s.prefs).len(), 256);
        s.execute("panel.swatches.sortByColor", json!({})).expect("sort");
        for (cmd, key, v) in [("panel.mixer.hsb", "ColorMixerColorModel", 3), ("panel.info.cmy", "InfoPanelColorModel", 2), ("panel.info.centimeters", "InfoPanelRulerUnits", 2)] {
            s.execute(cmd, json!({})).expect(cmd);
            assert_eq!(s.prefs.get_int(key, -1), v, "{cmd}");
            assert_eq!(s.command_state(cmd).1, Some(true));
        }
    }

    #[test]
    fn optimize_menu_commands() {
        let mut s = doc_with_rect();
        s.execute("pi.fill", json!({"color": "#ff0000"})).expect("rot");
        // neue Seiten exportieren als PNG 32 (BA-033); die Palettenbefehle brauchen GIF
        s.execute("pi.export.settings", json!({"settings": serde_json::to_value(fp_model::ExportSettings::default()).expect("json")})).expect("gif");
        let ex = |s: &Session| export_of(s);
        changes_and_undoes(&mut s, "panel.optimize.interlaced", json!({}));
        assert!(ex(&s).interlaced);
        changes_and_undoes(&mut s, "panel.optimize.sharpenJpegEdges", json!({"on": false}));
        assert_eq!(ex(&s).jpeg_subsampling, 1);
        changes_and_undoes(&mut s, "panel.optimize.editColor", json!({"color": "#ff0000", "newColor": "#00ff00"}));
        assert_eq!(ex(&s).palette.iter().find(|p| p.color == 0xffff_0000).and_then(|p| p.new_color), Some(0xff00_ff00));
        changes_and_undoes(&mut s, "panel.optimize.makeTransparent", json!({"colors": ["#ffffff"]}));
        assert!(ex(&s).palette.iter().any(|p| p.color == 0xffff_ffff && p.transparent == Some(true)));
        changes_and_undoes(&mut s, "panel.optimize.removeEdit", json!({"colors": ["#ff0000"]}));
        changes_and_undoes(&mut s, "panel.optimize.lockColors", json!({"colors": ["#ff0000"]}));
        changes_and_undoes(&mut s, "panel.optimize.unlockAll", json!({}));
        assert!(ex(&s).palette.iter().all(|p| p.locked.is_none()));
        // Exportpalette des Bildes enthält Rot (und durch die Bearbeitung kein Grün mehr)
        let pal = current_export_palette(&s).expect("Palette");
        assert!(pal.iter().any(|c| c & 0x00ff_ffff == 0xff_0000), "{pal:x?}");
        let path = tmp("palette.act");
        s.execute("panel.optimize.savePalette", json!({"path": path})).expect("save");
        changes_and_undoes(&mut s, "panel.optimize.loadPalette", json!({"path": path}));
        assert_eq!(ex(&s).palette_mode, 0);
        // Vorgaben
        s.execute("panel.optimize.saveSettings", json!({"name": "Meine"})).expect("save settings");
        assert_eq!(export_presets(&s.prefs).len(), 1);
        changes_and_undoes(&mut s, "panel.optimize.exportWizard", json!({"target": "imageEditing"}));
        assert_eq!(ex(&s).format, 2);
        changes_and_undoes(&mut s, "panel.optimize.applySettings", json!({"name": "Meine"}));
        assert_eq!(ex(&s).palette_mode, 0);
        s.execute("panel.optimize.deleteSettings", json!({"name": "Meine"})).expect("delete");
        assert!(export_presets(&s.prefs).is_empty());
        // Optimieren auf Größe (JPEG)
        s.execute("pi.export.settings", json!({"settings": serde_json::to_value(fp_model::ExportSettings { format: 1, color_mode: 1, jpeg_quality: 100, ..Default::default() }).expect("json")})).expect("jpeg");
        let v = changes_and_undoes(&mut s, "panel.optimize.optimizeToSize", json!({"kb": 0.5}));
        assert!(v["bytes"].as_u64().is_some());
        assert!(ex(&s).jpeg_quality < 100);
    }

    /// §4b: dieselbe Änderung 50× (wie Tippen/Schieben) ergibt dasselbe wie einmal mit dem
    /// Endwert – keine Duplikate in Effekt-, Palette-, Stil-, Farbfeld- oder Hilfslinienlisten.
    #[test]
    fn repeated_changes_do_not_duplicate() {
        let mut s = doc_with_rect();
        s.execute("element.addEffect", json!({"effect": "glow", "params": {"BevelType": "4"}})).expect("glow");
        let fx_len = |s: &Session| {
            let d = s.active().expect("d");
            docops::element_on_page(&d.doc, d.page, d.selection.elements[0]).and_then(|e| e.common.effects.clone()).map(|l| l.effects.len()).unwrap_or(0)
        };
        let n0 = s.active().map(|d| d.history.undo_count()).unwrap_or(0);
        for k in 0..50 {
            s.execute("pi.effect.set", json!({"index": 0, "params": {"GlowWidth": (k % 30).to_string()}})).expect("set");
            s.execute("pi.fill", json!({"color": format!("#{:02x}0000", k * 5)})).expect("fill");
            s.execute("pi.effects.visible", json!({"visible": k % 2 == 0})).expect("visible");
            s.execute("panel.optimize.lockColors", json!({"colors": ["#ff0000"], "on": true})).expect("lock");
            s.execute("panel.optimize.editColor", json!({"color": "#00ff00", "newColor": format!("#0000{:02x}", k)})).expect("edit");
            s.execute("pi.guides", json!({"color": "#ff00ff", "locked": k % 2 == 0})).expect("guides");
            s.execute("pi.state.props", json!({"delay": k})).expect("delay");
            s.execute("panel.swatches.setChoice", json!({"set": 2})).expect("swatches");
            s.execute("panel.layers.thumbnailOptions", json!({"size": 16})).expect("thumbs");
        }
        assert_eq!(fx_len(&s), 1, "genau ein Glühen");
        let e = export_of(&s);
        assert_eq!(e.palette.iter().filter(|p| p.color == 0xffff_0000).count(), 1);
        assert_eq!(e.palette.iter().filter(|p| p.color == 0xff00_ff00).count(), 1);
        assert_eq!(e.palette.len(), 2);
        assert_eq!(e.frames.first().map(|f| f.delay), Some(49));
        assert_eq!(current_swatches(&s.prefs).len(), 256);
        let d = s.active().expect("d");
        let g = d.doc.pages[0].root.guides.clone().unwrap_or_default();
        assert!(g.horizontal.is_empty() && g.vertical.is_empty());
        // Stil 50× anwenden → ein Stil, Element verweist darauf
        s.execute("pi.style.new", json!({"name": "S"})).expect("style");
        for _ in 0..50 {
            s.execute("pi.style.apply", json!({"name": "S"})).expect("apply");
            s.execute("pi.style.new", json!({"name": "S"})).expect("redefine");
        }
        assert_eq!(s.active().and_then(|d| d.doc.styles.as_ref().map(|v| v.len())), Some(1));
        // Verlauf: jeder Befehl ein Schritt, vollständig rückgängig machbar
        let steps = s.active().map(|d| d.history.undo_count()).unwrap_or(0);
        assert!(steps > n0);
        let _ = s;
    }

    #[test]
    fn saved_brush_stroke_roundtrip() {
        let mut s = doc_with_rect();
        s.execute("pi.stroke", json!({"preset": "Kohle/Pastell", "width": 7})).expect("stroke");
        let a = s.active().map(|d| d.selection.elements[0]).expect("a");
        changes_and_undoes(&mut s, "pi.brush.save", json!({"name": "Mein Strich"}));
        s.execute("draw.rectangle", json!({"rect": [120, 10, 180, 60]})).expect("r2");
        s.execute("pi.stroke", json!({"width": 2})).expect("w");
        changes_and_undoes(&mut s, "pi.stroke", json!({"preset": "Mein Strich"}));
        let b = attrs(&s).brush.clone().expect("Pinsel");
        assert_eq!(b.category, "bc_Charcoal");
        assert_eq!(b.diameter, 2, "Breite bleibt");
        let _ = a;
        changes_and_undoes(&mut s, "pi.brush.delete", json!({"name": "Mein Strich"}));
        assert!(s.active().and_then(|d| d.doc.pages[0].presets.brushes.as_ref().map(|v| v.is_empty())).unwrap_or(true));
    }

    /// Pinsel der Werkzeugvorgabe: „Strich bearbeiten“ ohne Auswahl landet in `new_brush` und
    /// auf neu gezeichneten Rechtecken und Stiftpfaden; §4b mehrfach ändern, Löschen, Undo.
    #[test]
    fn tool_brush_default_goes_to_new_shapes() {
        use crate::tools::ToolId;
        use crate::ToolEvent;
        let mut s = doc_with_rect();
        s.execute("select.none", json!({})).expect("none");
        let doc0 = s.active().map(|d| d.doc.clone()).expect("d");
        // 50× ändern = einmal mit dem letzten Wert; das Dokument bleibt unberührt
        for k in 0..50 {
            s.execute("pi.tool.brush", json!({"diameter": 3 + k % 5, "softness": 40, "angle": 30, "sensitivity": {"PZ": k % 3 * 10}})).expect("tool");
        }
        assert!(Arc::ptr_eq(&doc0, &s.active().expect("d").doc), "Werkzeugvorgabe ändert das Dokument nicht");
        let b = tool_brush(&s.tools.options);
        assert_eq!((b.diameter, b.softness, b.angle, b.sensitivity(b'P', b'Z')), (7, 400, 30, 100), "Promille");
        s.execute("pi.tool.brush", json!({"sensitivity": {"PZ": 25}})).expect("sens");
        let mut once = Session::default();
        once.execute("pi.tool.brush", json!({"diameter": 7, "softness": 40, "angle": 30, "sensitivity": {"PZ": 25}})).expect("once");
        assert_eq!(s.tools.options.new_brush, once.tools.options.new_brush, "Ergebnis hängt nur von den letzten Werten ab");
        let check = |s: &Session, what: &str| {
            let b = attrs(s).brush.clone().unwrap_or_else(|| panic!("{what}: kein Pinsel"));
            assert_eq!((b.diameter, b.softness, b.angle, b.sensitivity(b'P', b'Z')), (7, 400, 30, 250), "{what}");
        };
        // Rechteck aufziehen
        let ev = |s: &mut Session, list: &[ToolEvent]| {
            for e in list {
                s.tool_event(e).expect("Ereignis");
            }
        };
        s.set_tool(ToolId::Rectangle).expect("rect");
        let before = s.active().map(|d| d.doc.clone()).expect("d");
        ev(&mut s, &[ToolEvent::down(120.0, 10.0), ToolEvent::drag(150.0, 40.0), ToolEvent::up(180.0, 70.0)]);
        check(&s, "Rechteck");
        s.execute("edit.undo", json!({})).expect("undo");
        assert_eq!(*before, *s.active().expect("d").doc, "Rückgängig in einem Schritt");
        s.execute("edit.redo", json!({})).expect("redo");
        check(&s, "Rechteck nach Wiederholen");
        // Stiftpfad
        s.set_tool(ToolId::Pen).expect("pen");
        ev(&mut s, &[ToolEvent::down(10.0, 80.0), ToolEvent::up(10.0, 80.0), ToolEvent::down(90.0, 90.0), ToolEvent::up(90.0, 90.0), ToolEvent::key(crate::ToolKey::Enter)]);
        check(&s, "Stift");
        // Strichvorgabe wechseln ersetzt die Pinselart, Stärke bleibt
        s.execute("pi.tool.brush", json!({"preset": "Kohle/Pastell"})).expect("preset");
        let b = tool_brush(&s.tools.options);
        assert_eq!((b.category.as_str(), b.diameter), ("bc_Charcoal", 7));
        s.execute("pi.tool.brush", json!({"width": 12})).expect("width");
        assert_eq!(tool_brush(&s.tools.options).diameter, 12);
        // Löschen: wieder die Strichvorgabe
        s.execute("pi.tool.brush", json!({"clear": true})).expect("clear");
        assert!(s.tools.options.new_brush.is_none());
        assert_eq!(tool_brush(&s.tools.options).diameter, 1);
        s.set_tool(ToolId::Rectangle).expect("rect");
        ev(&mut s, &[ToolEvent::down(20.0, 20.0), ToolEvent::drag(40.0, 30.0), ToolEvent::up(60.0, 50.0)]);
        assert_ne!(attrs(&s).brush.as_ref().map(|b| b.angle), Some(30), "ohne Vorgabepinsel kein alter Pinsel");
        // Fehler: ungültiger Empfindlichkeitsschlüssel, unbekannte Vorgabe
        assert!(s.execute("pi.tool.brush", json!({"sensitivity": {"XX": 5}})).is_err());
        assert!(s.execute("pi.tool.brush", json!({"preset": "gibt es nicht"})).is_err());
        assert!(s.tools.options.new_brush.is_none(), "Fehler lassen die Vorgabe unverändert");
    }

    /// Inhalte einer über Seiten geteilten Ebene sind auf der anderen Seite auswähl- und
    /// bearbeitbar (Inspektor-Befehle), Rückgängig stellt exakt her.
    #[test]
    fn shared_layer_contents_editable_on_other_page() {
        let mut s = doc_with_rect();
        let a = s.active().map(|d| d.selection.elements[0]).expect("a");
        s.execute("page.add", json!({})).expect("Seite");
        s.execute("page.setCurrent", json!({"index": 0})).expect("Seite 1");
        s.execute("panel.layers.shareToAllPages", json!({})).expect("teilen");
        s.execute("page.setCurrent", json!({"index": 1})).expect("Seite 2");
        s.execute("select.set", json!({"ids": [a.0]})).expect("auswählbar");
        let before = s.active().map(|d| (*d.doc).clone()).expect("d");
        s.execute("pi.fill", json!({"color": "#00ff00"})).expect("Füllung");
        s.execute("pi.stroke", json!({"width": 4, "color": "#0000ff"})).expect("Strich");
        assert_eq!(attrs(&s).brush.as_ref().map(|b| b.diameter), Some(4));
        s.execute("edit.undo", json!({})).expect("u1");
        s.execute("edit.undo", json!({})).expect("u2");
        assert_eq!(before, *s.active().expect("d").doc);
    }

    /// §4b: Pfadoperation 50× wechseln = einmal mit der letzten Operation.
    #[test]
    fn compound_op_depends_only_on_final_value() {
        let make = || {
            let mut s = doc_with_rect();
            let a = s.active().map(|d| d.selection.elements[0]).expect("a");
            s.execute("draw.rectangle", json!({"rect": [60, 30, 150, 90]})).expect("b");
            let b = s.active().map(|d| d.selection.elements[0]).expect("b");
            s.execute("select.set", json!({"ids": [a.0, b.0]})).expect("sel");
            s
        };
        let contours = |s: &Session| {
            let d = s.active().expect("d");
            let e = docops::element_on_page(&d.doc, d.page, d.selection.elements[0]).expect("e");
            match &e.kind {
                ElementKind::CompoundShape(c) => (c.operands.iter().map(|o| if let ElementKind::Path(p) = &o.kind { p.path_op } else { 99 }).collect::<Vec<_>>(), format!("{:?}", c.result.as_ref().map(|r| shape_of(r).map(|x| x.0)))),
                _ => (Vec::new(), String::new()),
            }
        };
        let mut s1 = make();
        s1.execute("pi.compound.setOp", json!({"op": 2})).expect("einmal");
        let mut s2 = make();
        for k in 0..50 {
            s2.execute("pi.compound.setOp", json!({"op": (k % 4) + 1})).expect("wechseln");
        }
        s2.execute("pi.compound.setOp", json!({"op": 2})).expect("ende");
        assert_eq!(contours(&s1), contours(&s2));
    }

    /// §4b: Folgen aus Ausführen, Rückgängig und Wiederholen stellen exakt her.
    #[test]
    fn undo_redo_sequences() {
        let mut s = doc_with_rect();
        let d0 = doc_arc(&s);
        s.execute("pi.fill", json!({"color": "#ff0000"})).expect("A");
        let da = doc_arc(&s);
        s.execute("pi.stroke", json!({"width": 4, "color": "#0000ff"})).expect("B");
        s.undo().expect("undo B");
        assert!(Arc::ptr_eq(&da, &doc_arc(&s)));
        s.execute("pi.roundness", json!({"value": 30})).expect("C");
        let dc = doc_arc(&s);
        assert!(s.redo().is_err() || Arc::ptr_eq(&dc, &doc_arc(&s)), "nach neuem Schritt kein Wiederholen von B");
        s.undo().expect("undo C");
        assert!(Arc::ptr_eq(&da, &doc_arc(&s)));
        s.redo().expect("redo C");
        assert!(Arc::ptr_eq(&dc, &doc_arc(&s)));
        s.undo().expect("undo C");
        s.undo().expect("undo A");
        assert!(Arc::ptr_eq(&d0, &doc_arc(&s)));
    }

    /// §4b: leere Auswahl, gesperrte/ausgeblendete Objekte, Gruppen, mehrere Seiten.
    #[test]
    fn edge_cases_selection_groups_pages() {
        let mut s = doc_with_rect();
        s.execute("select.set", json!({"ids": []})).expect("leer");
        assert!(s.execute("pi.fill", json!({"color": "#ff0000"})).is_err(), "ohne Auswahl Fehler statt Wirkung");
        assert!(s.execute("pi.effect.set", json!({"index": 0, "params": {}})).is_err());
        // Gruppe: Füllung wirkt auf die Kinder
        s.execute("draw.rectangle", json!({"rect": [130, 10, 180, 60]})).expect("r2");
        s.execute("select.all", json!({})).expect("alle");
        s.execute("modify.group", json!({})).expect("group");
        changes_and_undoes(&mut s, "pi.fill", json!({"color": "#00ff00"}));
        // ausgeblendetes Objekt bleibt änderbar (wie das Original über den Inspektor)
        s.execute("element.setVisible", json!({"visible": false})).expect("hide");
        changes_and_undoes(&mut s, "pi.fill", json!({"color": "#0000ff"}));
        // zweite Seite: Befehle wirken auf die aktuelle Seite
        s.execute("edit.insert.page", json!({})).expect("Seite");
        s.execute("draw.rectangle", json!({"rect": [10, 10, 50, 50]})).expect("r3");
        let p0 = Arc::clone(&s.active().expect("d").doc.pages[0]);
        changes_and_undoes(&mut s, "pi.fill", json!({"color": "#123456"}));
        assert!(Arc::ptr_eq(&p0, &s.active().expect("d").doc.pages[0]), "Seite 1 unverändert");
    }

    #[test]
    fn guides_dialog_command() {
        let mut s = doc_with_rect();
        s.execute("guide.add", json!({"orientation": "vertical", "position": 20})).expect("Hilfslinie");
        changes_and_undoes(&mut s, "pi.guides", json!({"color": "#ff00ff", "show": false, "snap": true, "locked": true, "clear": true}));
        let d = s.active().expect("doc");
        assert!(!d.view.show_guides && d.view.snap_to_guides && d.view.lock_guides);
        let g = d.doc.pages[0].root.guides.clone().expect("Hilfslinien");
        assert!(g.vertical.is_empty());
        assert_eq!(g.color, Some(Argb(0xffff_00ff)));
    }
}
