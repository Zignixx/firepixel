//! Hilfen für die Werkzeug-Rückmeldung der Leinwand: „laufende Ameisen“ am echten Umriss der
//! Pixelauswahl (Kanten aus `fp_engine::geometry::trace_mask`, gecacht), Werkzeugattribute des
//! Inspektors als Parameter für neue Objekte und der eigene Mauszeiger.

use egui::{Color32, Painter, Pos2, Rect, Stroke};
use fp_geom::Point;
use serde_json::{json, Value};

/// Cache des Pixelauswahl-Umrisses (laufende Ameisen).
#[derive(Default)]
pub struct AntsCache {
    /// Schlüssel (Grenzen und Prüfsumme der Maske).
    pub key: (usize, i32, i32, i32, i32),
    /// Umrisse in Dokumentkoordinaten (geschlossene Polygone).
    pub outlines: Vec<Vec<Point>>,
}

fn mask_key(m: &fp_raster::Mask) -> (usize, i32, i32, i32, i32) {
    let b = m.bounds();
    let data = m.data();
    // Prüfsumme über eine Stichprobe (große Masken) bzw. alle Bytes (kleine)
    let step = (data.len() / 65_536).max(1);
    let mut h: usize = data.len();
    for (i, v) in data.iter().enumerate().step_by(step) {
        h = h.wrapping_mul(31).wrapping_add(*v as usize ^ i);
    }
    (h, b.left, b.top, b.right, b.bottom)
}

/// Ab so vielen gewählten Objekten zeichnet die Leinwand Auswahlrahmen und Eckgriffe gebündelt
/// als ein Mesh ([`FrameBatch`], PERF-1). `usize::MAX` schaltet das ab (Bildvergleich im Test).
pub static FRAME_BATCH_MIN: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(32);

/// Auswahlrahmen (1 px, Mitte auf ,5) und Eckgriffe vieler Objekte als **ein** Mesh ohne
/// Kantenglättung (PERF-1: Tausende gewählte Objekte = Zehntausende Formen mit ~50 Eckpunkten je
/// Objekt, auf dem Software-Rasterer 200 ms je Bild). Bei ganzzahliger Skalierung liegen alle
/// Kanten auf Pixelgrenzen; egui tastet dort genau dieselben Pixel ab wie bei `rect_stroke` bzw.
/// `rect_filled` (Kantenglättung halb innen, halb außen). Die Reihenfolge (Rahmen, dann seine Griffe,
/// Objekt für Objekt) bleibt; die Rahmenstreifen überlappen sich nicht (auch halbtransparent gleich).
#[derive(Default)]
pub struct FrameBatch {
    mesh: egui::Mesh,
}

impl FrameBatch {
    fn quad(&mut self, r: Rect, c: Color32) {
        if r.width() > 0.0 && r.height() > 0.0 {
            self.mesh.add_colored_rect(r, c);
        }
    }

    /// Rahmen `r` (Ecken auf ,5) mit Griffen der Kantenlänge `hs`. `false`: zu klein für die
    /// gebündelte Form – dann wie bisher mit einzelnen Formen zeichnen (vorher [`Self::flush`]).
    pub fn frame(&mut self, r: Rect, stroke: Color32, hs: f32, handle: Color32) -> bool {
        if !(r.width() >= 1.0 && r.height() >= 1.0) {
            return false;
        }
        let (l, t, rr, b) = (r.left(), r.top(), r.right(), r.bottom());
        self.quad(Rect::from_min_max(Pos2::new(l - 0.5, t - 0.5), Pos2::new(rr + 0.5, t + 0.5)), stroke);
        self.quad(Rect::from_min_max(Pos2::new(l - 0.5, b - 0.5), Pos2::new(rr + 0.5, b + 0.5)), stroke);
        self.quad(Rect::from_min_max(Pos2::new(l - 0.5, t + 0.5), Pos2::new(l + 0.5, b - 0.5)), stroke);
        self.quad(Rect::from_min_max(Pos2::new(rr - 0.5, t + 0.5), Pos2::new(rr + 0.5, b - 0.5)), stroke);
        for p in [r.left_top(), r.right_top(), r.left_bottom(), r.right_bottom()] {
            self.quad(Rect::from_center_size(p, egui::vec2(hs, hs)), handle);
        }
        true
    }

    /// Bisher Gesammeltes als eine Form ausgeben.
    pub fn flush(&mut self, painter: &Painter) {
        if !self.mesh.is_empty() {
            painter.add(egui::Shape::mesh(std::mem::take(&mut self.mesh)));
        }
    }
}

/// Umriss der Maske (aus dem Cache bzw. neu berechnet).
pub fn ants_for<'a>(slot: &'a mut Option<AntsCache>, mask: &fp_raster::Mask) -> &'a [Vec<Point>] {
    let key = mask_key(mask);
    if slot.as_ref().is_none_or(|c| c.key != key) {
        let outlines = fp_engine::geometry::trace_mask(mask).iter().map(|c| c.nodes.iter().map(|n| n.anchor).collect::<Vec<Point>>()).filter(|v| v.len() >= 2).collect();
        *slot = Some(AntsCache { key, outlines });
    }
    slot.as_ref().map(|c| c.outlines.as_slice()).unwrap_or(&[])
}

/// Zeichnet die Umrisse gestrichelt (4 px schwarz/weiß), das Muster wandert mit der Zeit. Nur
/// Segmente, die `clip` berühren, werden gezeichnet.
pub fn paint_ants(painter: &Painter, outlines: &[Vec<Point>], to_screen: &dyn Fn(Point) -> Pos2, time: f64, a: Color32, b: Color32, clip: Rect) {
    let phase = ((time * 8.0) as i64 % 8) as f32;
    for poly in outlines {
        let pts: Vec<Pos2> = poly.iter().map(|p| to_screen(*p)).collect();
        let n = pts.len();
        let mut walked = 0.0f32;
        for i in 0..n {
            let (p, q) = (pts[i], pts[(i + 1) % n]);
            let len = p.distance(q);
            let seg = Rect::from_two_pos(p, q).expand(1.0);
            if len > 0.0 && seg.intersects(clip) {
                painter.line_segment([p, q], Stroke::new(1.0, b));
                let dir = (q - p) / len;
                // Strichanfang so, dass das Muster über die Ecken durchläuft
                let mut t = -((walked + phase) % 8.0);
                while t < len {
                    let (t0, t1) = (t.max(0.0), (t + 4.0).min(len));
                    if t1 > t0 {
                        painter.line_segment([p + dir * t0, p + dir * t1], Stroke::new(1.0, a));
                    }
                    t += 8.0;
                }
            }
            walked += len;
        }
    }
}

/// Hervorhebung der Farbfeld-Rückmeldung (zwischengespeichert je Dokumentstand und Auswahl).
pub struct SwatchCache {
    pub key: String,
    pub texture: Option<egui::TextureHandle>,
    /// Lage in Dokumentkoordinaten.
    pub rect: fp_geom::Rect,
    /// Anzahl hervorgehobener Pixel.
    pub count: usize,
}

/// Farbfeld-Rückmeldung (Optimieren ▸ „Farbfeld-Rückmeldung anzeigen“): die Pixel des exportierten
/// Bildes, die eine der im Bedienfeld gewählten Palettenfarben benutzen, werden auf der Leinwand
/// hervorgehoben. Liefert die Zahl der hervorgehobenen Pixel (0 = aus).
pub fn swatch_feedback(ctx: &egui::Context, st: &mut crate::app_state::AppState, painter: &Painter, to_screen: &dyn Fn(Point) -> Pos2) -> usize {
    let Some(id) = st.session.active_id() else { return 0 };
    let on = st.session.prefs.get_bool("Firepixel_OptimizeSwatchFeedback", true);
    let sel = st.dialogs.panel_sel.palette.clone();
    if !on || sel.is_empty() {
        if let Some(cc) = st.canvases.get_mut(&id) {
            cc.swatch = None;
        }
        return 0;
    }
    let Some(d) = st.session.active() else { return 0 };
    let key = format!("{}-{}-{}-{}-{:?}", d.id.0, d.generation, d.page, d.frame, sel);
    if st.canvases.get(&id).and_then(|c| c.swatch.as_ref()).is_none_or(|s| s.key != key) {
        let s = fp_export::doc_export::page_settings(&d.doc, d.page);
        let origin = fp_render::Renderer::page_rect(&d.doc, d.page).map(|r| (r.left as f32, r.top as f32)).unwrap_or((0.0, 0.0));
        let pv = crate::panels::optimize::preview(ctx, st, &s, true);
        let mut cache = SwatchCache { key, texture: None, rect: fp_geom::Rect::default(), count: 0 };
        if let Some(img) = pv.as_ref().and_then(|p| p.image.as_ref()) {
            // gewählte Einträge (Originalfarbe) → angezeigte Farbe im Exportbild
            let ents = pv.as_ref().map(|p| p.entries.clone()).unwrap_or_default();
            let shown: Vec<u32> = sel.iter().map(|k| ents.iter().find(|e| (e.color | 0xff00_0000) == *k).map(|e| e.shown & 0x00ff_ffff).unwrap_or(k & 0x00ff_ffff)).collect();
            let b = img.bounds();
            let (w, h) = (b.width() as usize, b.height() as usize);
            let mut pixels = vec![0u8; w * h * 4];
            for y in 0..h {
                for x in 0..w {
                    if let Some(p) = img.get(b.left + x as i32, b.top + y as i32)
                        && p[3] > 0
                        && shown.contains(&(((p[0] as u32) << 16) | ((p[1] as u32) << 8) | p[2] as u32))
                    {
                        pixels[(y * w + x) * 4..(y * w + x) * 4 + 4].copy_from_slice(&[255, 0, 255, 150]);
                        cache.count += 1;
                    }
                }
            }
            if w > 0 && h > 0 {
                let ci = egui::ColorImage::from_rgba_unmultiplied([w, h], &pixels);
                cache.texture = Some(ctx.load_texture("swatch-feedback", ci, egui::TextureOptions::NEAREST));
                cache.rect = fp_geom::Rect::new(origin.0 + b.left as f32, origin.1 + b.top as f32, origin.0 + b.right as f32, origin.1 + b.bottom as f32);
            }
        }
        st.canvases.entry(id).or_default().swatch = Some(cache);
    }
    let Some(s) = st.canvases.get(&id).and_then(|c| c.swatch.as_ref()) else { return 0 };
    if let Some(t) = &s.texture {
        let r = Rect::from_min_max(to_screen(Point::new(s.rect.left, s.rect.top)), to_screen(Point::new(s.rect.right, s.rect.bottom)));
        painter.image(t.id(), r, Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
    }
    s.count
}

/// Werkzeugattribute des Inspektors → Parameter für `pi.fill` / `pi.stroke` neuer Objekte
/// (`None` = Vorgabe: einfarbige Füllung, 1-Pixel-Strich).
pub fn attrs_params(ta: &crate::inspector::obj_quads::ToolAttrs) -> (Option<Value>, Option<Value>) {
    let d = crate::inspector::obj_quads::ToolAttrs::default();
    let fill_default = ta.fill_kind == d.fill_kind && ta.edge == d.edge && ta.feather == d.feather && ta.fill_texture.is_empty();
    let fill = if fill_default {
        None
    } else {
        let mut f = match ta.fill_kind {
            0 => json!({"category": "none"}),
            2 => json!({"category": "gradient", "shape": ta.grad_shape}),
            // Muster der Werkzeugvorgabe (leer = Vorgabemuster des Originals)
            3 if ta.pattern.is_empty() => json!({"category": "pattern"}),
            3 => json!({"category": "pattern", "pattern": ta.pattern}),
            _ => json!({"category": "solid"}),
        };
        if ta.fill_kind != 0 {
            f["edge"] = json!(ta.edge.clamp(0, 2));
            if ta.edge == 2 {
                f["feather"] = json!(ta.feather.clamp(0, 100));
            }
            if !ta.fill_texture.is_empty() {
                f["texture"] = json!(ta.fill_texture);
                f["textureAmount"] = json!(ta.fill_texture_amount.clamp(0, 100));
                f["textureTransparent"] = json!(ta.fill_texture_transparent);
            }
        }
        Some(f)
    };
    let stroke_default = ta.stroke_preset == d.stroke_preset && ta.stroke_width == d.stroke_width && ta.softness == d.softness && ta.placement == d.placement && ta.stroke_texture.is_empty();
    let stroke = if stroke_default {
        None
    } else {
        match ta.stroke_preset.and_then(|i| fp_engine::commands::panel::BRUSH_PRESETS.get(i)) {
            None => Some(json!({"none": true})),
            Some(b) => {
                let mut s = json!({
                    "preset": format!("{}/{}", b.category_de, b.name_de),
                    "width": ta.stroke_width.clamp(1, 1000),
                    "softness": ta.softness.clamp(0, 100),
                    "placement": match ta.placement { 0 => "inside", 2 => "outside", _ => "center" },
                });
                if !ta.stroke_texture.is_empty() {
                    s["texture"] = json!(ta.stroke_texture);
                    s["textureAmount"] = json!(ta.stroke_texture_amount.clamp(0, 100));
                }
                Some(s)
            }
        }
    };
    (fill, stroke)
}

/// Überträgt die Werkzeugattribute des Inspektors in die Werkzeugoptionen der Sitzung.
pub fn sync_tool_attrs(ctx: &egui::Context, st: &mut crate::app_state::AppState) {
    let ta = crate::inspector::obj_quads::tool_attrs(ctx);
    let (f, s) = attrs_params(&ta);
    let o = &mut st.session.tools.options;
    if o.new_fill != f {
        o.new_fill = f;
    }
    if o.new_stroke != s {
        o.new_stroke = s;
    }
    // Mischmodus, „Transparenz beibehalten“ und „Automatisch löschen“ der Malwerkzeuge (PINSEL-1)
    let blend = crate::inspector::obj_quads::BLEND_ITEMS.get(ta.blend).map(|b| b.1).filter(|m| *m >= 0).unwrap_or(0) as u8;
    if o.paint_blend != blend {
        o.paint_blend = blend;
    }
    if o.keep_transparency != ta.keep_transparency {
        o.keep_transparency = ta.keep_transparency;
    }
    if o.auto_erase != ta.auto_erase {
        o.auto_erase = ta.auto_erase;
    }
    // Retusche-Werkzeuge lesen ihre Werte aus den Voreinstellungen: Änderungen im Inspektor für das
    // aktive Werkzeug dorthin übertragen (nur bei Änderung, damit gespeicherte Werte gelten)
    let last_id = egui::Id::new("lw-last-tool-attrs");
    // Werkzeugwechsel: Werte des neuen Werkzeugs aus dessen Voreinstellungen laden (Werksvorgaben
    // des Originals bzw. zuletzt eingestellt) – jedes Werkzeug behält eigene Werte
    let tool = st.session.tools.active;
    let tool_key = egui::Id::new("lw-tool-attrs-tool");
    if ctx.data(|d| d.get_temp::<fp_engine::ToolId>(tool_key)) != Some(tool) {
        ctx.data_mut(|d| d.insert_temp(tool_key, tool));
        let mut ta = crate::inspector::obj_quads::tool_attrs(ctx);
        if attrs_from_prefs(tool, &st.session.prefs, &mut ta) {
            if tool == fp_engine::ToolId::Brush {
                st.session.tools.options.brush_size = ta.stroke_width.max(1) as f32;
            }
            let t2 = ta.clone();
            crate::inspector::obj_quads::edit_tool_attrs(ctx, |a| *a = t2);
        }
        ctx.data_mut(|d| d.insert_temp(last_id, ta));
        return;
    }
    let last: Option<crate::inspector::obj_quads::ToolAttrs> = ctx.data(|d| d.get_temp(last_id));
    if last.as_ref() != Some(&ta) {
        if last.is_some() {
            for (k, v) in retouch_prefs(st.session.tools.active, &ta) {
                st.session.prefs.set(&k, v);
            }
        }
        ctx.data_mut(|d| d.insert_temp(last_id, ta));
    }
}

/// Werkzeugattribute des Inspektors aus den Voreinstellungen des Werkzeugs (Umkehrung von
/// [`retouch_prefs`]; Werksvorgaben = PREF-Daten des Originals, UX-A W3–W13). `false` = das Werkzeug
/// hat keine solchen Voreinstellungen.
pub fn attrs_from_prefs(tool: fp_engine::ToolId, p: &fp_engine::prefs::Preferences, ta: &mut crate::inspector::obj_quads::ToolAttrs) -> bool {
    use fp_engine::ToolId as T;
    let int = |k: &str, d: i64| p.get_int(k, d) as i32;
    match tool {
        T::Freeform => {
            ta.freeform_size = int("FreeFormPushNPullLen", 72);
            ta.pressure = p.get_bool("FreeFormPushNPullLen_Sens", true);
            ta.preview = p.get_bool("FreeFormPushNPull_Preview", true);
            return true;
        }
        T::Reshape => {
            ta.freeform_size = int("FreeFormGravDiameter", 72);
            ta.strength = int("FreeFormGravPower", 80);
            ta.pressure = p.get_bool("FreeFormGravDiameter_Sens", true);
            ta.velocity = p.get_bool("FreeFormGravPower_Sens", false);
            ta.preview = p.get_bool("FreeFormGravity_Preview", true);
            return true;
        }
        T::PathScrubAdd | T::PathScrubSubtract => {
            ta.pressure = p.get_bool("ScrubPressure", true);
            ta.velocity = p.get_bool("ScrubVelocity", true);
            ta.rate = int("ScrubRate", 10);
            return true;
        }
        T::Brush => return brush_attrs_from_prefs(p, ta),
        _ => {}
    }
    let pre = match tool {
        T::Blur => "BlurTool",
        T::Sharpen => "SharpenTool",
        T::Dodge => "DodgeTool",
        T::Burn => "BurnTool",
        T::Smudge => "SmudgeTool",
        T::RubberStamp => "StampTool",
        T::ReplaceColor => "ReplaceColorTool",
        T::RedEyeRemoval => "RedEyeRemovalTool",
        _ => return false,
    };
    let k = |s: &str| format!("{pre}{s}");
    if tool == T::RubberStamp {
        ta.retouch_size = int("StampToolSize", 16);
        ta.retouch_edge = int("StampToolEdge", 500) / 10;
        ta.source_aligned = p.get_int("StampToolSource", 0) == 0;
    } else {
        ta.retouch_size = int(&k("Diameter"), 13);
        ta.retouch_edge = int(&k("SoftEdge"), 1000) / 10;
    }
    ta.retouch_round = p.get_bool(&k("IsCircle"), true);
    match tool {
        T::Blur | T::Sharpen => ta.intensity = int(&k("Intensity"), 50),
        T::Dodge | T::Burn => {
            ta.exposure = int(&k("Exposure"), 50);
            ta.range = p.get_int(&k("Range"), 1).clamp(0, 2) as usize;
        }
        T::Smudge => ta.strength = int(&k("Pressure"), 50),
        T::RedEyeRemoval => {
            ta.strength = int(&k("Strength"), 50);
            ta.tolerance = int(&k("Tolerance"), 50);
        }
        T::ReplaceColor => {
            // Werksvorgaben des Originals: Toleranz 32, Stärke 200 (PARITAET_A W8; die Ressource des
            // Originals führt beide mit falschem Typ, `data\prefs.json` als Ganzzahl)
            ta.tolerance = int(&k("Tolerance"), 32);
            ta.strength = int(&k("Strength"), 200);
            ta.colorize = p.get_bool(&k("Colorize"), false);
        }
        _ => {}
    }
    true
}

/// Pinsel-Werkzeugvorgabe in den Voreinstellungen (PAR 02/18: das Original merkt Strichart, Größe
/// und Rand über den Neustart). Schlüssel technisch englisch (§1c); Strichart als interner Name
/// „Kategorie/Art“ (`bc_Basic/bn_Soft Rounded`), Größe im Schlüssel des Originals `BrushTipSize`.
pub const BRUSH_PREF_STROKE: &str = "BrushToolStroke";
pub const BRUSH_PREF_SOFT: &str = "BrushToolSoftEdge";
pub const BRUSH_PREF_TEXTURE: &str = "BrushToolTexture";
pub const BRUSH_PREF_TEXTURE_AMOUNT: &str = "BrushToolTextureAmount";

/// Werkzeugattribute des Pinsels aus den Voreinstellungen; `false`, solange nichts gespeichert ist
/// (dann gelten die bisherigen Werte).
pub fn brush_attrs_from_prefs(p: &fp_engine::prefs::Preferences, ta: &mut crate::inspector::obj_quads::ToolAttrs) -> bool {
    let stroke = p.get_str(BRUSH_PREF_STROKE, "");
    let Some((cat, name)) = stroke.split_once('/') else { return false };
    let Some(i) = fp_engine::commands::panel::preset_index_of(cat, name) else { return false };
    ta.stroke_preset = Some(i);
    ta.stroke_width = p.get_int("BrushTipSize", 1).clamp(1, 1000) as i32;
    ta.softness = p.get_int(BRUSH_PREF_SOFT, 0).clamp(0, 100) as i32;
    ta.stroke_texture = p.get_str(BRUSH_PREF_TEXTURE, "");
    ta.stroke_texture_amount = p.get_int(BRUSH_PREF_TEXTURE_AMOUNT, 0).clamp(0, 100) as i32;
    true
}

/// Voreinstellungen der Pinsel-Werkzeugvorgabe (Umkehrung von [`brush_attrs_from_prefs`]).
pub fn brush_prefs(ta: &crate::inspector::obj_quads::ToolAttrs) -> Vec<(String, fp_engine::prefs::PrefValue)> {
    use fp_engine::prefs::PrefValue::{Int, Str};
    let Some(b) = ta.stroke_preset.and_then(|i| fp_engine::commands::panel::BRUSH_PRESETS.get(i)) else { return Vec::new() };
    vec![
        (BRUSH_PREF_STROKE.into(), Str(format!("{}/{}", b.category, b.name))),
        ("BrushTipSize".into(), Int(ta.stroke_width.clamp(1, 1000) as i64)),
        (BRUSH_PREF_SOFT.into(), Int(ta.softness.clamp(0, 100) as i64)),
        (BRUSH_PREF_TEXTURE.into(), Str(ta.stroke_texture.clone())),
        (BRUSH_PREF_TEXTURE_AMOUNT.into(), Int(ta.stroke_texture_amount.clamp(0, 100) as i64)),
    ]
}

/// Voreinstellungen eines Retusche-Werkzeugs aus den Werkzeugattributen des Inspektors.
pub fn retouch_prefs(tool: fp_engine::ToolId, ta: &crate::inspector::obj_quads::ToolAttrs) -> Vec<(String, fp_engine::prefs::PrefValue)> {
    use fp_engine::prefs::PrefValue::{Bool, Int};
    use fp_engine::ToolId as T;
    match tool {
        T::Freeform => {
            return vec![
                ("FreeFormPushNPullLen".into(), Int(ta.freeform_size.max(1) as i64)),
                ("FreeFormPushNPullLen_Sens".into(), Bool(ta.pressure)),
                ("FreeFormPushNPull_Preview".into(), Bool(ta.preview)),
            ];
        }
        T::Reshape => {
            return vec![
                ("FreeFormGravDiameter".into(), Int(ta.freeform_size.max(1) as i64)),
                ("FreeFormGravPower".into(), Int(ta.strength as i64)),
                ("FreeFormGravDiameter_Sens".into(), Bool(ta.pressure)),
                ("FreeFormGravPower_Sens".into(), Bool(ta.velocity)),
            ];
        }
        T::PathScrubAdd | T::PathScrubSubtract => {
            return vec![("ScrubPressure".into(), Bool(ta.pressure)), ("ScrubVelocity".into(), Bool(ta.velocity)), ("ScrubRate".into(), Int(ta.rate as i64))];
        }
        T::Brush => return brush_prefs(ta),
        _ => {}
    }
    let pre = match tool {
        T::Blur => "BlurTool",
        T::Sharpen => "SharpenTool",
        T::Dodge => "DodgeTool",
        T::Burn => "BurnTool",
        T::Smudge => "SmudgeTool",
        T::RubberStamp => "StampTool",
        T::ReplaceColor => "ReplaceColorTool",
        T::RedEyeRemoval => "RedEyeRemovalTool",
        _ => return Vec::new(),
    };
    let k = |s: &str| format!("{pre}{s}");
    let mut v = Vec::new();
    if tool == T::RubberStamp {
        v.push(("StampToolSize".to_string(), Int(ta.retouch_size.max(1) as i64)));
        v.push(("StampToolEdge".to_string(), Int((ta.retouch_edge.clamp(0, 100) * 10) as i64)));
        v.push(("StampToolSource".to_string(), Int(if ta.source_aligned { 0 } else { 1 })));
    } else {
        v.push((k("Diameter"), Int(ta.retouch_size.max(1) as i64)));
        v.push((k("SoftEdge"), Int((ta.retouch_edge.clamp(0, 100) * 10) as i64)));
    }
    v.push((k("IsCircle"), Bool(ta.retouch_round)));
    match tool {
        T::Blur | T::Sharpen => v.push((k("Intensity"), Int(ta.intensity as i64))),
        T::Dodge | T::Burn => {
            v.push((k("Exposure"), Int(ta.exposure as i64)));
            v.push((k("Range"), Int(ta.range as i64)));
        }
        T::Smudge => v.push((k("Pressure"), Int(ta.strength as i64))),
        T::RedEyeRemoval => {
            v.push((k("Strength"), Int(ta.strength as i64)));
            v.push((k("Tolerance"), Int(ta.tolerance as i64)));
        }
        T::ReplaceColor => {
            v.push((k("Strength"), Int(ta.strength as i64)));
            v.push((k("Colorize"), Bool(ta.colorize)));
            v.push((k("Tolerance"), Int(ta.tolerance as i64)));
        }
        _ => {}
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_attrs_give_plain_shapes() {
        let ta = crate::inspector::obj_quads::ToolAttrs::default();
        assert_eq!(attrs_params(&ta), (None, None));
        let ta = crate::inspector::obj_quads::ToolAttrs { fill_kind: 2, grad_shape: 3, stroke_width: 4, ..Default::default() };
        let (f, s) = attrs_params(&ta);
        assert_eq!(f.as_ref().and_then(|v| v.get("category")).and_then(|v| v.as_str()), Some("gradient"));
        assert_eq!(s.as_ref().and_then(|v| v.get("width")).and_then(|v| v.as_i64()), Some(4));
        let none = crate::inspector::obj_quads::ToolAttrs { stroke_preset: None, ..Default::default() };
        assert_eq!(attrs_params(&none).1, Some(json!({"none": true})));
    }

    /// UX-A W3–W13: Werkzeugwechsel lädt die Werksvorgaben (PREF) je Werkzeug; Änderungen bleiben
    /// je Werkzeug erhalten (keine gemeinsame Pinselgröße).
    #[test]
    fn tool_attrs_follow_prefs_per_tool() {
        use fp_engine::ToolId as T;
        let ctx = egui::Context::default();
        let mut st = crate::app_state::AppState::new(fp_engine::Session::default(), crate::app_state::UiSettings::default(), None);
        let switch =|st: &mut crate::app_state::AppState, t: T| {
            st.session.tools.active = t;
            sync_tool_attrs(&ctx, st);
            crate::inspector::obj_quads::tool_attrs(&ctx)
        };
        let a = switch(&mut st, T::Blur);
        assert_eq!((a.retouch_size, a.retouch_edge, a.intensity), (13, 100, 50));
        let a = switch(&mut st, T::RubberStamp);
        assert_eq!((a.retouch_size, a.retouch_edge), (16, 50));
        let a = switch(&mut st, T::Smudge);
        assert_eq!((a.retouch_size, a.retouch_edge, a.strength), (22, 50, 50));
        let a = switch(&mut st, T::ReplaceColor);
        assert_eq!((a.retouch_size, a.tolerance), (13, 32));
        let a = switch(&mut st, T::RedEyeRemoval);
        assert_eq!((a.tolerance, a.strength), (50, 50));
        let a = switch(&mut st, T::Freeform);
        assert_eq!((a.freeform_size, a.pressure, a.preview), (72, true, true));
        let a = switch(&mut st, T::Reshape);
        assert_eq!((a.freeform_size, a.strength, a.pressure), (72, 80, true));
        let a = switch(&mut st, T::PathScrubAdd);
        assert_eq!((a.pressure, a.velocity, a.rate), (true, true, 10));
        // Weichzeichner auf 30 stellen, zum Abwedler und zurück: jeder behält seinen Wert
        switch(&mut st, T::Blur);
        crate::inspector::obj_quads::edit_tool_attrs(&ctx, |a| a.retouch_size = 30);
        sync_tool_attrs(&ctx, &mut st);
        assert_eq!(st.session.prefs.get_int("BlurToolDiameter", 0), 30);
        assert_eq!(switch(&mut st, T::Dodge).retouch_size, 13);
        assert_eq!(switch(&mut st, T::Blur).retouch_size, 30);
    }

    #[test]
    fn ants_trace_square_mask() {
        let m = fp_raster::Mask::filled(fp_geom::IRect::new(2, 3, 12, 8), 255).expect("mask");
        let mut slot = None;
        let o = ants_for(&mut slot, &m);
        assert_eq!(o.len(), 1);
        let xs: Vec<f32> = o[0].iter().map(|p| p.x).collect();
        assert!(xs.contains(&2.0) && xs.contains(&12.0));
    }
}
