//! Live-Filter (Live-Effekte): Menü „+“ des Inspektors (Kategorie → Filter), Namen der Effekte in
//! der Filterliste und die Einstellungsdialoge (Popups 4121/4123/4124/4158/4229 mit den
//! DDV-Bereichen aus 11 §2.3.3: Schatten Abstand 0..100, Deckkraft 0..100, Weichheit 0..30,
//! Winkel 0..360; Abschrägung Breite 0..35, Kontrast 0..100, Weichheit 0..10; Glühen Breite 0..35,
//! Deckkraft 0..100, Weichheit 0..30). Dialoge mit Live-Vorschau über den Verlauf.

use super::form::{Field, FormDialog};
use super::t;
use crate::app_state::AppState;
use crate::inspector::fields;
use crate::theme::Theme;
use egui::{vec2, Response, Ui};
use fp_model::effects::{Effect, EffectKind};
use serde_json::{json, Map, Value};

/// Art eines Menüeintrags.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FxKind {
    /// Live-Effekt mit Dialog (Name für `element.addEffect`, Abschrägungstyp).
    Effect { name: &'static str, bevel: Option<i32> },
    /// Bitmap-Filter mit Dialog (Befehl `filter.*` mit `live: true`).
    Filter(&'static str),
    /// Effekt ohne Einstellungen.
    Simple(&'static str),
}

/// Ein Eintrag des Live-Filter-Menüs (`group`, `label`: Textschlüssel aus `filter.ftl`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fx {
    pub group: &'static str,
    pub label: &'static str,
    pub kind: FxKind,
}

impl Fx {
    /// Name in der aktuellen Sprache (ohne „...“).
    pub fn name(&self) -> String {
        t(self.label)
    }
    /// Menütext: Filter mit eigenem Dialog tragen denselben Namen wie im Menü Filter (das Original
    /// registriert je Filter einen Namen, z. B. „Gaußscher Weichz. ...“, `re\06-bitmap-filter.md` §2.2).
    pub fn menu_text(&self) -> String {
        match self.kind {
            FxKind::Filter(cmd) => crate::menu::clean_label(&t(&fp_i18n::command_key(cmd))).0,
            _ => t(self.label),
        }
    }
}

const fn fx(group: &'static str, label: &'static str, kind: FxKind) -> Fx {
    Fx { group, label, kind }
}

/// Menü in Original-Reihenfolge.
pub const FX: &[Fx] = &[
    fx("effect-group-bevel", "effect-bevel-inner", FxKind::Effect { name: "bevel", bevel: Some(0) }),
    fx("effect-group-bevel", "effect-bevel-outer", FxKind::Effect { name: "bevel", bevel: Some(1) }),
    fx("effect-group-bevel", "effect-emboss-inset", FxKind::Effect { name: "bevel", bevel: Some(3) }),
    fx("effect-group-bevel", "effect-emboss-raised", FxKind::Effect { name: "bevel", bevel: Some(2) }),
    fx("effect-group-adjust-color", "filter-adjustColor-autoLevels", FxKind::Simple("autoLevels")),
    fx("effect-group-adjust-color", "effect-color-fill", FxKind::Effect { name: "colorFill", bevel: None }),
    fx("effect-group-adjust-color", "filter-adjustColor-hueSaturation", FxKind::Filter("filter.adjustColor.hueSaturation")),
    fx("effect-group-adjust-color", "filter-adjustColor-curves", FxKind::Filter("filter.adjustColor.curves")),
    fx("effect-group-adjust-color", "filter-adjustColor-brightnessContrast", FxKind::Filter("filter.adjustColor.brightnessContrast")),
    fx("effect-group-adjust-color", "filter-adjustColor-levels", FxKind::Filter("filter.adjustColor.levels")),
    fx("effect-group-adjust-color", "filter-adjustColor-invert", FxKind::Simple("invert")),
    fx("effect-group-other", "filter-other-convertToAlpha", FxKind::Simple("convertToAlpha")),
    fx("effect-group-other", "filter-other-findEdges", FxKind::Simple("findEdges")),
    fx("effect-group-noise", "filter-noise-addNoise", FxKind::Filter("filter.noise.addNoise")),
    fx("effect-group-sharpen", "filter-sharpen-sharpen", FxKind::Simple("sharpen")),
    fx("effect-group-sharpen", "filter-sharpen-sharpenMore", FxKind::Simple("sharpenMore")),
    fx("effect-group-sharpen", "filter-sharpen-unsharpMask", FxKind::Filter("filter.sharpen.unsharpMask")),
    fx("effect-group-shadow-glow", "effect-solid-shadow", FxKind::Effect { name: "solidShadow", bevel: None }),
    fx("effect-group-shadow-glow", "effect-glow", FxKind::Effect { name: "glow", bevel: Some(4) }),
    fx("effect-group-shadow-glow", "effect-inner-shadow", FxKind::Effect { name: "innerShadow", bevel: None }),
    fx("effect-group-shadow-glow", "effect-inner-glow", FxKind::Effect { name: "innerGlow", bevel: None }),
    fx("effect-group-shadow-glow", "effect-drop-shadow", FxKind::Effect { name: "dropShadow", bevel: None }),
    fx("effect-group-blur", "filter-blur-motion", FxKind::Filter("filter.blur.motion")),
    fx("effect-group-blur", "filter-blur-gaussian", FxKind::Filter("filter.blur.gaussian")),
    fx("effect-group-blur", "filter-blur-radial", FxKind::Filter("filter.blur.radial")),
    fx("effect-group-blur", "filter-blur-blurMore", FxKind::Simple("blurMore")),
    fx("effect-group-blur", "filter-blur-blur", FxKind::Simple("blur")),
    fx("effect-group-blur", "filter-blur-zoom", FxKind::Filter("filter.blur.zoom")),
];

/// Anzeigename eines Effekts in der Filterliste (Namen aus `filter.ftl` über
/// `Effect::label_key`; unbekannte Effekte: Name aus der Datei bzw. „Effekt“).
pub fn effect_label(e: &Effect) -> String {
    match e.label_key() {
        Some(key) => t(key),
        None => e.ui_name().map(|s| s.to_string()).unwrap_or_else(|| t("effect-generic")),
    }
}
/// Hat der Effekt einen Einstellungsdialog?
pub fn editable(e: &Effect) -> bool {
    matches!(
        e.kind(),
        EffectKind::BevelFamily
            | EffectKind::DropShadow
            | EffectKind::InnerShadow
            | EffectKind::InnerGlow
            | EffectKind::SolidShadow
            | EffectKind::ColorFill
            | EffectKind::GaussianBlur
            | EffectKind::MotionBlur
            | EffectKind::RadialBlur
            | EffectKind::ZoomBlur
            | EffectKind::UnsharpMask
            | EffectKind::AddNoise
            | EffectKind::BrightnessContrast
            | EffectKind::HueSaturation
            | EffectKind::Curves
            | EffectKind::Levels
    )
}

/// Ziel beim Bearbeiten eines vorhandenen Kurven- bzw. Tonwert-Effekts über ⓘ
/// (re\19-kurven-tonwert.md §1.1/§6.1): Index in der Effektliste und die beim Öffnen gewählten
/// Objekte (Auswahlwechsel während des Dialogs, AGENTEN_REGELN §4b). OK ersetzt die Werte
/// (`pi.effect.set`), Abbrechen lässt sie unverändert.
#[derive(Clone, Debug, PartialEq)]
pub struct LiveEdit {
    pub index: usize,
    pub ids: Vec<u64>,
}

impl LiveEdit {
    /// Parameter für `pi.effect.set`.
    pub fn invocation(&self, params: Map<String, Value>) -> Value {
        let mut v = json!({"index": self.index, "params": params});
        if !self.ids.is_empty() {
            v["ids"] = json!(self.ids);
        }
        v
    }
}

/// Effektparameter eines Kurven-Effekts wie im Dateiformat (alle vier Punktlisten, §6.1).
pub fn curves_effect_params(p: &fp_raster::filters::CurvesParams) -> Map<String, Value> {
    let pts = |v: &Vec<fp_raster::filters::CurvePoint>| json!(v.iter().map(|q| json!([q.x, q.y])).collect::<Vec<_>>());
    let mut m = Map::new();
    m.insert("rgb_points".into(), pts(&p.rgb_points));
    m.insert("red_points".into(), pts(&p.red_points));
    m.insert("green_points".into(), pts(&p.green_points));
    m.insert("blue_points".into(), pts(&p.blue_points));
    m
}

/// Effektparameter eines Tonwert-Effekts wie im Dateiformat (`source_low_rgb` … `gamma_blue`).
pub fn levels_effect_params(p: &fp_raster::filters::LevelsParams) -> Map<String, Value> {
    let mut m = Map::new();
    for (n, c) in [("rgb", &p.rgb), ("red", &p.red), ("green", &p.green), ("blue", &p.blue)] {
        m.insert(format!("source_low_{n}"), json!(c.source_low.to_string()));
        m.insert(format!("source_high_{n}"), json!(c.source_high.to_string()));
        m.insert(format!("dest_low_{n}"), json!(c.dest_low.to_string()));
        m.insert(format!("dest_high_{n}"), json!(c.dest_high.to_string()));
        m.insert(format!("gamma_{n}"), json!(c.gamma.to_string()));
    }
    m
}

/// Gespeicherte Kurven eines Effekts (fehlende bzw. unvollständige Listen: Identität).
pub fn curves_of(e: &Effect) -> fp_raster::filters::CurvesParams {
    let mut p = fp_raster::filters::CurvesParams::default();
    for (k, dst) in [("rgb_points", &mut p.rgb_points), ("red_points", &mut p.red_points), ("green_points", &mut p.green_points), ("blue_points", &mut p.blue_points)] {
        if let Some(v) = e.get_points(k).filter(|v| v.len() >= 2) {
            *dst = v.iter().map(|q| fp_raster::filters::CurvePoint { x: q.x, y: q.y }).collect();
        }
    }
    p
}

/// Gespeicherte Tonwerte eines Effekts (fehlende Schlüssel: Grundwerte 0/1/255 und 0/255).
pub fn levels_of(e: &Effect) -> fp_raster::filters::LevelsParams {
    let ch = |n: &str| fp_raster::filters::LevelsChannel {
        source_low: e.get_f64(&format!("source_low_{n}")).map(|v| v as i32).unwrap_or(0),
        source_high: e.get_f64(&format!("source_high_{n}")).map(|v| v as i32).unwrap_or(255),
        dest_low: e.get_f64(&format!("dest_low_{n}")).map(|v| v as i32).unwrap_or(0),
        dest_high: e.get_f64(&format!("dest_high_{n}")).map(|v| v as i32).unwrap_or(255),
        gamma: e.get_f64(&format!("gamma_{n}")).unwrap_or(1.0),
    };
    fp_raster::filters::LevelsParams { rgb: ch("rgb"), red: ch("red"), green: ch("green"), blue: ch("blue") }
}

fn selected_ids(st: &AppState) -> Vec<u64> {
    st.session.active().map(|d| d.selection.elements.iter().map(|i| i.0).collect()).unwrap_or_default()
}

/// Kurven- bzw. Tonwertdialog als Live-Effekt öffnen: ohne `edit` zum Anlegen (Grundwerte, OK hängt
/// den Effekt an, Abbrechen legt nichts an), mit `edit` für einen vorhandenen Effekt.
fn open_adjust_dialog(st: &mut AppState, cmd: &str, edit: Option<(usize, &Effect)>) -> bool {
    let ids = selected_ids(st);
    match cmd {
        "filter.adjustColor.curves" => {
            let mut d = super::curves::CurvesDialog::new(st, true);
            if let Some((index, e)) = edit {
                d.params = curves_of(e);
                d.set_channel(0);
                d.edit = Some(LiveEdit { index, ids });
                d.preview.command = "pi.effect.set".into();
            }
            st.dialogs.curves = Some(d);
        }
        "filter.adjustColor.levels" => {
            let mut d = super::levels::LevelsDialog::new(st, true);
            if let Some((index, e)) = edit {
                d.params = levels_of(e);
                d.set_channel(0);
                d.edit = Some(LiveEdit { index, ids });
                d.preview.command = "pi.effect.set".into();
            }
            st.dialogs.levels = Some(d);
        }
        _ => return false,
    }
    true
}

/// Menü „+“ an einer Antwort (Kategorie → Filter, Optionen „Alle ein/aus“).
pub fn add_menu(ui: &mut Ui, resp: &Response, st: &mut AppState) {
    let theme = Theme::current(ui.ctx());
    let mut chosen: Option<usize> = None;
    // „Optionen“ wie im Original aus der Ressource 6921: Als Stil speichern │ Alle ein, Alle aus │
    // Zusatzmodule suchen (vorher im ≡ des Inspektors, MN-3)
    let options = crate::panels::common::resource_menu_nodes(st, "6921");
    let mut acts = Vec::new();
    egui::Popup::menu(resp).frame(crate::menu::menu_frame(&theme)).show(|ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        ui.menu_button(t("dlg-fx-options"), |ui| crate::menu::show_nodes(ui, &options, &mut acts));
        let mut groups: Vec<&str> = Vec::new();
        for f in FX {
            if !groups.contains(&f.group) {
                groups.push(f.group);
            }
        }
        for g in groups {
            ui.menu_button(t(g), |ui| {
                for (i, f) in FX.iter().enumerate().filter(|(_, f)| f.group == g) {
                    if fields::list_item(ui, &f.menu_text(), false).clicked() {
                        chosen = Some(i);
                        ui.close();
                    }
                }
            });
        }
        let _ = fields::list_item(ui, &t("dlg-fx-photoshop-unavailable"), false);
    });
    for a in acts {
        st.push(a);
    }
    if let Some(i) = chosen {
        begin_add(st, i);
    }
}

/// Effekt aus dem Menü hinzufügen (mit Dialog, falls einstellbar).
pub fn begin_add(st: &mut AppState, index: usize) {
    let Some(f) = FX.get(index) else { return };
    match f.kind {
        FxKind::Simple(name) => st.run("element.addEffect", json!({"effect": name})),
        FxKind::Filter(cmd) => {
            if open_adjust_dialog(st, cmd, None) {
                return;
            }
            if let Some(d) = super::catalog::filter_form_with_prefs(st, cmd, true) {
                st.dialogs.form = Some(d);
            } else {
                st.run(cmd, json!({"live": true}));
            }
        }
        FxKind::Effect { name, bevel } => {
            let fill = st.session.tools.fill_color;
            st.dialogs.form = Some(with_target(st, effect_form(name, bevel, None, None, fill)));
        }
    }
}

/// Einstellungen eines vorhandenen Effekts bearbeiten.
pub fn begin_edit(st: &mut AppState, index: usize, e: &Effect) {
    if !editable(e) {
        return;
    }
    match e.kind() {
        EffectKind::Curves => {
            open_adjust_dialog(st, "filter.adjustColor.curves", Some((index, e)));
            return;
        }
        EffectKind::Levels => {
            open_adjust_dialog(st, "filter.adjustColor.levels", Some((index, e)));
            return;
        }
        _ => {}
    }
    let fill = st.session.tools.fill_color;
    let d = match e.kind() {
        EffectKind::BevelFamily => {
            let bt = e.get_f64("BevelType").unwrap_or(0.0) as i32;
            Some(effect_form(if bt == 4 { "glow" } else { "bevel" }, Some(bt), Some(e), Some(index), fill))
        }
        EffectKind::DropShadow => Some(effect_form("dropShadow", None, Some(e), Some(index), fill)),
        EffectKind::InnerShadow => Some(effect_form("innerShadow", None, Some(e), Some(index), fill)),
        EffectKind::InnerGlow => Some(effect_form("innerGlow", None, Some(e), Some(index), fill)),
        EffectKind::SolidShadow => Some(effect_form("solidShadow", None, Some(e), Some(index), fill)),
        EffectKind::ColorFill => Some(effect_form("colorFill", None, Some(e), Some(index), fill)),
        _ => filter_effect_form(e, index),
    };
    if let Some(d) = d {
        st.dialogs.form = Some(with_target(st, d));
    }
}

fn hex_alpha(hex: &str) -> (String, f64) {
    let c = fp_color::Rgba8::parse_hex(hex).unwrap_or(fp_color::Rgba8::new(0, 0, 0, 255));
    (format!("#{:02x}{:02x}{:02x}ff", c.r, c.g, c.b), (c.a as f64 * 100.0 / 255.0).round())
}


/// Farbe mit Deckkraft in Prozent. Entspricht `pct` noch dem ursprünglichen Alphawert `orig`
/// (gerundet), bleibt dieser exakt erhalten – das %-Feld ist sonst verlustbehaftet (0x80 → 0x7f).
fn with_alpha_keep(hex: &str, pct: f64, orig: Option<u8>) -> String {
    let c = fp_color::Rgba8::parse_hex(hex).unwrap_or(fp_color::Rgba8::new(0, 0, 0, 255));
    let a = match orig {
        Some(o) if (o as f64 * 100.0 / 255.0).round() == pct.round() => o,
        _ => (pct.clamp(0.0, 100.0) * 2.55).round() as u8,
    };
    format!("#{:02x}{:02x}{:02x}{:02x}", c.r, c.g, c.b, a)
}

fn orig_alpha(d: &FormDialog) -> Option<u8> {
    d.base.get("_alpha").and_then(|v| v.as_u64()).map(|v| v as u8)
}

fn num(e: Option<&Effect>, key: &str, d: f64) -> f64 {
    e.and_then(|x| x.get_f64(key)).unwrap_or(d)
}

fn txt(e: Option<&Effect>, key: &str, d: &str) -> String {
    e.and_then(|x| x.get(key)).map(|s| s.to_string()).unwrap_or_else(|| d.to_string())
}

/// Vollständige Vorgabeliste der Abschrägungsfamilie (`EFDv`, 05 §3; Rücklesung re Gruppe 2):
/// Abschrägung innen/außen Breite 10, Weichheit (AngleSoftness) 3; Prägung Breite 2, MaskSoftness 2,
/// vertieft mit Fläche `#0000001e`. Werte als Text wie im Dateiformat.
pub fn bevel_defaults(bevel_type: i64) -> Vec<(&'static str, String)> {
    let emboss = bevel_type == 2 || bevel_type == 3;
    let kv: [(&'static str, &str); 20] = [
        ("BevelType", ""),
        ("BevelWidth", if emboss { "2" } else { "10" }),
        ("BevelContrast", "75"),
        ("EdgeThreshold", "0"),
        ("AngleSoftness", "3"),
        ("MaskSoftness", if emboss { "2" } else { "0" }),
        ("LightDistance", "100"),
        ("LightAngle", "135"),
        ("SlopeType", "0"),
        ("SlopeMultiplier", "1"),
        ("OuterBevelColor", "#df0000"),
        ("DownBlendColor", "#0000003f"),
        ("HitBlendColor", "#ffffff3f"),
        ("HiliteColor", "#ffffff"),
        ("ShadowColor", "#000000"),
        ("EmbossFaceColor", if bevel_type == 3 { "#0000001e" } else { "#00000000" }),
        ("ShowObject", "false"),
        ("ButtonState", "0"),
        ("GlowStartDistance", "0"),
        ("GlowWidth", "0"),
    ];
    kv.iter().map(|(k, v)| (*k, if *k == "BevelType" { bevel_type.to_string() } else { v.to_string() })).collect()
}

/// Parameter eines Effektdialogs für `element.addEffect` bzw. `pi.effect.set`.
fn effect_params(d: &FormDialog) -> Value {
    let v = d.raw_values();
    let mut params = Map::new();
    let s = |k: &str| v.get(k).cloned().unwrap_or(Value::Null);
    let n = |k: &str| v.get(k).and_then(|x| x.as_f64()).unwrap_or(0.0);
    let effect = d.base.get("effect").and_then(|x| x.as_str()).unwrap_or("dropShadow").to_string();
    match effect.as_str() {
        "dropShadow" | "innerShadow" => {
            params.insert("ShadowDistance".into(), json!(n("dist").to_string()));
            params.insert("ShadowAngle".into(), json!(n("angle").to_string()));
            params.insert("ShadowBlur".into(), json!(n("soft").to_string()));
            params.insert("ShadowColor".into(), json!(with_alpha_keep(s("color").as_str().unwrap_or("#000000"), n("opacity"), orig_alpha(d))));
            params.insert("ShadowType".into(), json!(if s("knockout").as_bool() == Some(true) { "1" } else { "0" }));
        }
        "bevel" | "glow" => {
            let bt = v.get("type").and_then(|x| x.as_i64()).unwrap_or_else(|| d.base.get("bevel").and_then(|x| x.as_i64()).unwrap_or(0).max(0));
            params.insert("BevelType".into(), json!(bt.to_string()));
            if bt == 4 {
                params.insert("GlowWidth".into(), json!(n("width").to_string()));
                params.insert("MaskSoftness".into(), json!(n("soft").to_string()));
                params.insert("GlowStartDistance".into(), json!(n("offset").to_string()));
                params.insert("OuterBevelColor".into(), json!(with_alpha_keep(s("color").as_str().unwrap_or("#ffff00"), n("opacity"), orig_alpha(d))));
            } else {
                let emboss = bt == 2 || bt == 3;
                let base_bt = d.base.get("bevel").and_then(|x| x.as_i64()).unwrap_or(-1);
                let is_new = d.base.get("index").is_none();
                // Neuer Effekt: vollständige Vorgabeliste wie das Original (re: „alle Schlüssel werden immer
                // gespeichert“), sonst greifen die Lese-Vorgaben des Renderers (AngleSoftness 25, Fläche transparent).
                if is_new {
                    for (k, val) in bevel_defaults(bt) {
                        params.insert(k.into(), json!(val));
                    }
                }
                // Art gewechselt (erhaben ↔ vertieft): Fläche der neuen Art (vertieft `#0000001e`).
                if emboss && (is_new || base_bt != bt) {
                    params.insert("EmbossFaceColor".into(), json!(if bt == 3 { "#0000001e" } else { "#00000000" }));
                }
                params.insert("BevelWidth".into(), json!(n("width").to_string()));
                params.insert("BevelContrast".into(), json!(n("contrast").to_string()));
                // Weichheit: Abschrägung ↔ AngleSoftness, Prägung ↔ MaskSoftness (re Dialogtabelle).
                params.insert(if emboss { "MaskSoftness" } else { "AngleSoftness" }.into(), json!(n("soft").to_string()));
                params.insert("LightAngle".into(), json!(n("angle").to_string()));
                if let Some(sl) = v.get("slope").and_then(|x| x.as_i64()) {
                    params.insert("SlopeType".into(), json!(sl.to_string()));
                }
                if let Some(b) = v.get("button").and_then(|x| x.as_i64()) {
                    params.insert("ButtonState".into(), json!(b.to_string()));
                }
                if bt == 1
                    && let Some(c) = s("color").as_str()
                {
                    params.insert("OuterBevelColor".into(), json!(c));
                }
                if bt >= 2 {
                    params.insert("ShowObject".into(), json!(if s("show").as_bool() == Some(true) { "true" } else { "false" }));
                }
            }
        }
        "innerGlow" => {
            params.insert("GlowWidth".into(), json!(n("width").to_string()));
            params.insert("MaskSoftness".into(), json!(n("soft").to_string()));
            params.insert("GlowStartDistance".into(), json!(n("offset").to_string()));
            params.insert("OuterBevelColor".into(), json!(with_alpha_keep(s("color").as_str().unwrap_or("#ffff00"), n("opacity"), orig_alpha(d))));
        }
        "solidShadow" => {
            params.insert("angle".into(), json!(n("angle").to_string()));
            params.insert("distance".into(), json!(n("dist").to_string()));
            let orig = s("original").as_bool() == Some(true);
            params.insert("color".into(), json!(if orig { "#ffffff00".to_string() } else { s("color").as_str().unwrap_or("#000000ff").to_string() }));
        }
        "colorFill" => {
            params.insert("Color".into(), json!(s("color").as_str().unwrap_or("#ff0000ff")));
            params.insert("Opacity".into(), json!(n("opacity").to_string()));
            let bl = v.get("blend").and_then(|x| x.as_i64()).unwrap_or(0);
            params.insert("Blendmode".into(), json!(bl.to_string()));
        }
        _ => {}
    }
    let mut out = match d.base.get("index").and_then(|x| x.as_u64()) {
        Some(i) => json!({"index": i, "params": params}),
        None => json!({"effect": effect, "params": params}),
    };
    // Ziel beim Öffnen festhalten (Auswahlwechsel während des Dialogs, §4b)
    if let Some(ids) = d.base.get("ids") {
        out["ids"] = ids.clone();
    }
    out
}

/// Ausgewählte Elemente als Ziel des Dialogs.
fn with_target(st: &AppState, mut f: FormDialog) -> FormDialog {
    let ids: Vec<u64> = st.session.active().map(|d| d.selection.elements.iter().map(|i| i.0).collect()).unwrap_or_default();
    if !ids.is_empty() {
        f.base["ids"] = json!(ids);
    }
    f
}

/// Dialog eines Live-Effekts (neu bzw. vorhandener Effekt `index`).
pub fn effect_form(name: &str, bevel: Option<i32>, e: Option<&Effect>, index: Option<usize>, fill: fp_color::Argb) -> FormDialog {
    let (title, fields): (String, Vec<Field>) = match name {
        "dropShadow" | "innerShadow" => {
            let (col, op) = hex_alpha(&txt(e, "ShadowColor", "#000000a6"));
            (
                if name == "dropShadow" { t("effect-drop-shadow") } else { t("effect-inner-shadow") },
                vec![
                    Field::slider("dist", t("dlg-lbl-distance").as_str(), num(e, "ShadowDistance", 7.0), 0.0, 100.0),
                    Field::slider("opacity", t("dlg-lbl-opacity").as_str(), op, 0.0, 100.0).unit("%"),
                    Field::slider("soft", t("dlg-lbl-softness").as_str(), num(e, "ShadowBlur", 4.0), 0.0, 30.0),
                    Field::slider("angle", t("dlg-lbl-angle").as_str(), num(e, "ShadowAngle", 315.0), 0.0, 360.0).unit("°"),
                    Field::color("color", t("dlg-lbl-color").as_str(), &col),
                    Field::check("knockout", t("dlg-fx-knockout").as_str(), txt(e, "ShadowType", "0") == "1"),
                ],
            )
        }
        "glow" => {
            let (col, op) = hex_alpha(&txt(e, "OuterBevelColor", &format!("#{:02x}{:02x}{:02x}ff", fill.r(), fill.g(), fill.b())));
            (
                t("effect-glow"),
                vec![
                    Field::slider("width", t("dlg-lbl-width").as_str(), num(e, "GlowWidth", 4.0), 0.0, 35.0),
                    Field::slider("opacity", t("dlg-lbl-opacity").as_str(), if e.is_some() { op } else { 65.0 }, 0.0, 100.0).unit("%"),
                    Field::slider("soft", t("dlg-lbl-softness").as_str(), num(e, "MaskSoftness", 4.0), 0.0, 30.0),
                    Field::slider("offset", t("dlg-fx-glow-offset").as_str(), num(e, "GlowStartDistance", 0.0), 0.0, 100.0),
                    Field::color("color", t("dlg-lbl-color").as_str(), &col),
                ],
            )
        }
        "innerGlow" => {
            let (col, op) = hex_alpha(&txt(e, "OuterBevelColor", &format!("#{:02x}{:02x}{:02x}ff", fill.r(), fill.g(), fill.b())));
            (
                t("effect-inner-glow"),
                vec![
                    Field::slider("width", t("dlg-lbl-width").as_str(), num(e, "GlowWidth", 4.0), 0.0, 35.0),
                    Field::slider("opacity", t("dlg-lbl-opacity").as_str(), if e.is_some() { op } else { 65.0 }, 0.0, 100.0).unit("%"),
                    Field::slider("soft", t("dlg-lbl-softness").as_str(), num(e, "MaskSoftness", 4.0), 0.0, 30.0),
                    Field::slider("offset", t("dlg-fx-glow-offset").as_str(), num(e, "GlowStartDistance", 0.0), 0.0, 100.0),
                    Field::color("color", t("dlg-lbl-color").as_str(), &col),
                ],
            )
        }
        "solidShadow" => {
            let c = txt(e, "color", "#000000ff");
            (
                t("effect-solid-shadow"),
                vec![
                    Field::slider("angle", t("dlg-lbl-angle").as_str(), num(e, "angle", 315.0), 0.0, 360.0).unit("°"),
                    Field::slider("dist", t("dlg-lbl-distance").as_str(), num(e, "distance", 7.0), 0.0, 100.0),
                    Field::color("color", t("dlg-lbl-color").as_str(), if c == "#ffffff00" { "#000000ff" } else { &c }),
                    Field::check("original", t("dlg-fx-use-original-colors").as_str(), c == "#ffffff00"),
                ],
            )
        }
        "colorFill" => {
            let bl = num(e, "Blendmode", 0.0) as i64;
            let labels = crate::inspector::texts::blend_labels();
            let names: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
            let vals: Vec<Value> = crate::inspector::obj_quads::BLEND_ITEMS.iter().map(|b| json!(b.1.max(0))).collect();
            let idx = crate::inspector::obj_quads::BLEND_ITEMS.iter().position(|b| b.1 as i64 == bl).unwrap_or(0);
            (
                t("effect-color-fill"),
                vec![
                    Field::choice_of("blend", t("dlg-lbl-blend-mode").as_str(), &names, vals, idx),
                    Field::color("color", t("dlg-lbl-color").as_str(), &txt(e, "Color", "#ff0000ff")),
                    Field::slider("opacity", t("dlg-lbl-opacity").as_str(), num(e, "Opacity", 100.0), 0.0, 100.0).unit("%"),
                ],
            )
        }
        _ => {
            let bt = bevel.unwrap_or(0);
            let emboss = bt == 2 || bt == 3;
            let mut v = Vec::new();
            if emboss {
                v.push(Field::choice_of("type", t("dlg-lbl-kind").as_str(), &[t("effect-emboss-raised").as_str(), t("effect-emboss-inset").as_str()], vec![json!(2), json!(3)], if bt == 3 { 1 } else { 0 }));
            } else {
                v.push(Field::choice_of("type", t("dlg-lbl-kind").as_str(), &[t("effect-bevel-inner").as_str(), t("effect-bevel-outer").as_str()], vec![json!(0), json!(1)], if bt == 1 { 1 } else { 0 }));
                v.push(Field::choice("slope", t("dlg-fx-edge-shape").as_str(), &[t("dlg-fx-slope-flat").as_str(), t("dlg-fx-slope-smooth").as_str(), t("dlg-fx-slope-sloped").as_str(), t("dlg-fx-slope-frame1").as_str(), t("dlg-fx-slope-frame2").as_str(), t("dlg-fx-slope-ring").as_str(), t("dlg-fx-slope-ruffle").as_str()], num(e, "SlopeType", 0.0).clamp(0.0, 6.0) as usize));
            }
            // Reihenfolge wie im Original (re Dialogtabelle): [Kantenform], Breite, [Farbe nur außen], Kontrast,
            // Weichheit, Winkel, Tastenstatus bzw. „Objekt anzeigen“. Vorgaben nach `EFDv` (05 §3).
            v.push(Field::slider("width", t("dlg-lbl-width").as_str(), num(e, "BevelWidth", if emboss { 2.0 } else { 10.0 }), 0.0, 35.0));
            if bt == 1 {
                v.push(Field::color("color", t("dlg-lbl-color").as_str(), &txt(e, "OuterBevelColor", &format!("#{:02x}{:02x}{:02x}ff", fill.r(), fill.g(), fill.b()))));
            }
            v.push(Field::slider("contrast", t("dlg-lbl-contrast").as_str(), num(e, "BevelContrast", 75.0), 0.0, 100.0).unit("%"));
            // Weichheit: Prägung ↔ MaskSoftness (Vorgabe 2), Abschrägung ↔ AngleSoftness (Vorgabe 3), je 0–10.
            let soft = if emboss { num(e, "MaskSoftness", 2.0) } else { num(e, "AngleSoftness", 3.0) };
            v.push(Field::slider("soft", t("dlg-lbl-softness").as_str(), soft, 0.0, 10.0));
            v.push(Field::slider("angle", t("dlg-lbl-angle").as_str(), num(e, "LightAngle", 135.0), 0.0, 360.0).unit("°"));
            if emboss {
                v.push(Field::check("show", t("dlg-fx-show-object").as_str(), txt(e, "ShowObject", "false").eq_ignore_ascii_case("true")));
            } else {
                v.push(Field::choice("button", t("dlg-fx-button-preset").as_str(), &[t("dlg-fx-button-raised").as_str(), t("dlg-fx-button-highlighted").as_str(), t("dlg-fx-button-inset").as_str(), t("dlg-fx-button-inverted").as_str()], num(e, "ButtonState", 0.0).clamp(0.0, 3.0) as usize));
            }
            (if emboss { t("dlg-fx-emboss") } else { t("dlg-fx-bevel") }, v)
        }
    };
    let mut base = json!({"effect": name, "bevel": bevel.unwrap_or(-1)});
    // ursprünglicher Alphawert der Effektfarbe (verlustfreie Deckkraft)
    if let Some(e) = e {
        let key = match name {
            "dropShadow" | "innerShadow" => "ShadowColor",
            _ => "OuterBevelColor",
        };
        if let Some(c) = e.get(key).and_then(fp_color::Rgba8::parse_hex) {
            base["_alpha"] = json!(c.a);
        }
    }
    if let Some(i) = index {
        base["index"] = json!(i);
    }
    let command = if index.is_some() { "pi.effect.set" } else { "element.addEffect" };
    FormDialog::new(&format!("fx-{name}"), &title, command, fields).with_base(base).with_transform(effect_params).with_preview()
}

/// Dialog eines vorhandenen Bitmap-Filter-Effekts (Parameter wie im Dateiformat, 06 §2.2).
fn filter_effect_form(e: &Effect, index: usize) -> Option<FormDialog> {
    let n = |k: &str, d: f64| e.get_f64(k).unwrap_or(d);
    let (title, keys, fields): (String, Vec<(&str, &str)>, Vec<Field>) = match e.kind() {
        EffectKind::GaussianBlur => (t("dlg-title-gaussian-blur"), vec![("radius", "gaussian_blur_radius")], vec![Field::slider("radius", t("dlg-lbl-blur-radius").as_str(), n("gaussian_blur_radius", 1.0), 0.1, 250.0).decimals(1).dot_decimal().scale(crate::inspector::scale::BLUR_RADIUS)]),
        EffectKind::MotionBlur => (
t("dlg-title-motion-blur"),
            vec![("angle", "motion_blur_angle"), ("distance", "motion_blur_distance")],
            vec![Field::slider("angle", t("dlg-lbl-angle").as_str(), n("motion_blur_angle", 90.0), 0.0, 360.0).popup(), Field::slider("distance", t("dlg-lbl-motion-distance").as_str(), n("motion_blur_distance", 10.0), 1.0, 100.0).popup()],
        ),
        EffectKind::RadialBlur => (
t("dlg-title-radial-blur"),
            vec![("amount", "radial_blur_amount"), ("quality", "radial_blur_quality")],
            vec![Field::slider("amount", t("dlg-lbl-amount").as_str(), n("radial_blur_amount", 30.0), 1.0, 100.0).popup(), Field::slider("quality", t("dlg-lbl-quality").as_str(), n("radial_blur_quality", 20.0), 1.0, 100.0).popup()],
        ),
        EffectKind::ZoomBlur => (
t("dlg-title-zoom-blur"),
            vec![("amount", "zoom_blur_amount"), ("quality", "zoom_blur_quality")],
            vec![Field::slider("amount", t("dlg-lbl-zoom-amount").as_str(), n("zoom_blur_amount", 30.0), 1.0, 100.0).popup(), Field::slider("quality", t("dlg-lbl-quality").as_str(), n("zoom_blur_quality", 20.0), 1.0, 100.0).popup()],
        ),
        EffectKind::UnsharpMask => (
t("dlg-title-unsharp-mask"),
            vec![("amount", "unsharp_mask_amount"), ("radius", "unsharp_mask_radius"), ("threshold", "unsharp_mask_threshold")],
            vec![
                Field::slider("amount", t("dlg-lbl-sharpen-amount").as_str(), n("unsharp_mask_amount", 50.0), 1.0, 500.0).scale(crate::inspector::scale::SHARPEN_AMOUNT),
                Field::slider("radius", t("dlg-lbl-pixel-radius").as_str(), n("unsharp_mask_radius", 1.0), 0.1, 250.0).decimals(1).dot_decimal().scale(crate::inspector::scale::BLUR_RADIUS),
                Field::slider("threshold", t("dlg-lbl-threshold").as_str(), n("unsharp_mask_threshold", 0.0), 0.0, 255.0).scale(crate::inspector::scale::THRESHOLD),
            ],
        ),
        EffectKind::AddNoise => (
t("dlg-title-add-noise"),
            vec![("amount", "add_noise_amount"), ("use_color", "add_noise_use_color")],
            vec![Field::slider("amount", t("dlg-lbl-amount").as_str(), n("add_noise_amount", 50.0), 1.0, 400.0).popup(), Field::check("use_color", t("dlg-lbl-color-check").as_str(), e.get_bool("add_noise_use_color").unwrap_or(true))],
        ),
        EffectKind::BrightnessContrast => (
t("dlg-title-brightness-contrast"),
            vec![("brightness", "brightness_amount"), ("contrast", "contrast_amount")],
            vec![Field::slider("brightness", t("dlg-lbl-brightness").as_str(), n("brightness_amount", 0.0), -100.0, 100.0), Field::slider("contrast", t("dlg-lbl-contrast").as_str(), n("contrast_amount", 0.0), -100.0, 100.0)],
        ),
        EffectKind::HueSaturation => (
t("dlg-title-hue-saturation"),
            vec![("hue", "hue_amount"), ("saturation", "saturation_amount"), ("lightness", "lightness_amount"), ("colorize", "hls_colorize")],
            vec![
                Field::slider("hue", t("dlg-lbl-hue").as_str(), n("hue_amount", 0.0), -180.0, 180.0).scale(crate::inspector::scale::HUE),
                Field::slider("saturation", t("dlg-lbl-saturation").as_str(), n("saturation_amount", 0.0), -100.0, 100.0).scale(crate::inspector::scale::SATURATION),
                Field::slider("lightness", t("dlg-lbl-lightness").as_str(), n("lightness_amount", 0.0), -100.0, 100.0),
                Field::check("colorize", t("dlg-lbl-colorize").as_str(), e.get_bool("hls_colorize").unwrap_or(false)).beside_buttons(),
            ],
        ),
        _ => return None,
    };
    let mapping: Vec<Value> = keys.iter().map(|(a, b)| json!([a, b])).collect();
    let f = FormDialog::new("fx-filter", &title, "pi.effect.set", fields).with_base(json!({"index": index, "map": mapping})).with_transform(filter_effect_params).with_preview();
    // dieselbe Anordnung wie der Filterdialog aus dem Menü (PM-6)
    let side = matches!(e.kind(), EffectKind::GaussianBlur | EffectKind::UnsharpMask | EffectKind::AddNoise | EffectKind::BrightnessContrast | EffectKind::HueSaturation);
    Some(if side { f.with_side_buttons() } else { f })
}

fn filter_effect_params(d: &FormDialog) -> Value {
    let v = d.raw_values();
    let mut params = Map::new();
    if let Some(Value::Array(map)) = d.base.get("map") {
        for m in map {
            let (Some(a), Some(b)) = (m.get(0).and_then(|x| x.as_str()), m.get(1).and_then(|x| x.as_str())) else { continue };
            let val = match v.get(a) {
                Some(Value::Bool(x)) => x.to_string(),
                Some(Value::Number(x)) => x.to_string(),
                Some(other) => other.to_string(),
                None => continue,
            };
            params.insert(b.to_string(), json!(val));
        }
    }
    let mut out = json!({"index": d.base.get("index").cloned().unwrap_or(json!(0)), "params": params});
    if let Some(ids) = d.base.get("ids") {
        out["ids"] = ids.clone();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shadow_dialog_params() {
        let d = effect_form("dropShadow", None, None, None, fp_color::Argb(0xff00_00ff));
        let p = d.params();
        assert_eq!(p["effect"], json!("dropShadow"));
        assert_eq!(p["params"]["ShadowDistance"], json!("7"));
        assert_eq!(p["params"]["ShadowColor"], json!("#000000a6"));
        let mut e = Effect::new("{a7944db8-6ce2-11d1-8c76000502701850}", true);
        e.set("ShadowDistance", "12");
        e.set("ShadowColor", "#ff000080");
        let d = effect_form("dropShadow", None, Some(&e), Some(0), fp_color::Argb(0));
        let p = d.params();
        assert_eq!(p["index"], json!(0));
        assert_eq!(p["params"]["ShadowDistance"], json!("12"));
        // Deckkraft über das %-Feld bleibt verlustfrei, solange sie nicht geändert wird (0x80)
        assert_eq!(p["params"]["ShadowColor"], json!("#ff000080"));
        assert_eq!(effect_label(&e), "Schlagschatten");
    }
}
