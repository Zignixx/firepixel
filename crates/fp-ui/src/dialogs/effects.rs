//! Live-Filter (Live-Effekte): Menü „+“ des Inspektors (Kategorie → Filter), Namen der Effekte in
//! der Filterliste und die Einstellungsdialoge (Popups 4121/4123/4124/4158/4229 mit den
//! DDV-Bereichen aus 11 §2.3.3: Schatten Abstand 0..100, Deckkraft 0..100, Weichheit 0..30,
//! Winkel 0..360; Abschrägung Breite 0..35, Kontrast 0..100, Weichheit 0..10; Glühen Breite 0..35,
//! Deckkraft 0..100, Weichheit 0..30). Dialoge mit Live-Vorschau über den Verlauf.

use super::form::{Field, FormDialog};
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

/// Ein Eintrag des Live-Filter-Menüs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fx {
    pub group: &'static str,
    pub label: &'static str,
    pub kind: FxKind,
}

const fn fx(group: &'static str, label: &'static str, kind: FxKind) -> Fx {
    Fx { group, label, kind }
}

/// Menü in Original-Reihenfolge.
pub const FX: &[Fx] = &[
    fx("Abschrägung und Prägung", "Abschrägung innen", FxKind::Effect { name: "bevel", bevel: Some(0) }),
    fx("Abschrägung und Prägung", "Abschrägung außen", FxKind::Effect { name: "bevel", bevel: Some(1) }),
    fx("Abschrägung und Prägung", "Prägung eingelassen", FxKind::Effect { name: "bevel", bevel: Some(3) }),
    fx("Abschrägung und Prägung", "Prägung erhaben", FxKind::Effect { name: "bevel", bevel: Some(2) }),
    fx("Anpassen der Farbe", "Auto-Tonwertkorrektur", FxKind::Simple("autoLevels")),
    fx("Anpassen der Farbe", "Farbfüllung", FxKind::Effect { name: "colorFill", bevel: None }),
    fx("Anpassen der Farbe", "Farbton/Sättigung...", FxKind::Filter("filter.adjustColor.hueSaturation")),
    fx("Anpassen der Farbe", "Gradationskurven...", FxKind::Filter("filter.adjustColor.curves")),
    fx("Anpassen der Farbe", "Helligkeit/Kontrast...", FxKind::Filter("filter.adjustColor.brightnessContrast")),
    fx("Anpassen der Farbe", "Tonwertkorrektur...", FxKind::Filter("filter.adjustColor.levels")),
    fx("Anpassen der Farbe", "Umkehren", FxKind::Simple("invert")),
    fx("Andere", "In Alpha konvertieren", FxKind::Simple("convertToAlpha")),
    fx("Andere", "Kanten suchen", FxKind::Simple("findEdges")),
    fx("Rauschen", "Rauschen hinzufügen...", FxKind::Filter("filter.noise.addNoise")),
    fx("Schärfen", "Scharfzeichnen", FxKind::Simple("sharpen")),
    fx("Schärfen", "Stark scharfzeichnen", FxKind::Simple("sharpenMore")),
    fx("Schärfen", "Unscharf maskieren...", FxKind::Filter("filter.sharpen.unsharpMask")),
    fx("Schatten und Glühen", "Einfarbiger Schatten", FxKind::Effect { name: "solidShadow", bevel: None }),
    fx("Schatten und Glühen", "Glühen", FxKind::Effect { name: "glow", bevel: Some(4) }),
    fx("Schatten und Glühen", "Innerer Schatten", FxKind::Effect { name: "innerShadow", bevel: None }),
    fx("Schatten und Glühen", "Inneres Glühen", FxKind::Effect { name: "innerGlow", bevel: None }),
    fx("Schatten und Glühen", "Schlagschatten", FxKind::Effect { name: "dropShadow", bevel: None }),
    fx("Weichzeichnen", "Bewegungsunschärfe...", FxKind::Filter("filter.blur.motion")),
    fx("Weichzeichnen", "Gaußscher Weichzeichner...", FxKind::Filter("filter.blur.gaussian")),
    fx("Weichzeichnen", "Radialer Weichzeichner...", FxKind::Filter("filter.blur.radial")),
    fx("Weichzeichnen", "Stark weichzeichnen", FxKind::Simple("blurMore")),
    fx("Weichzeichnen", "Weichzeichnen", FxKind::Simple("blur")),
    fx("Weichzeichnen", "Zoom-Weichzeichner...", FxKind::Filter("filter.blur.zoom")),
];

/// Anzeigename eines Effekts in der Filterliste.
pub fn effect_label(e: &Effect) -> String {
    match e.kind() {
        EffectKind::BevelFamily => match e.get_f64("BevelType").unwrap_or(0.0) as i32 {
            1 => "Abschrägung außen",
            2 => "Prägung erhaben",
            3 => "Prägung eingelassen",
            4 => "Glühen",
            _ => "Abschrägung innen",
        }
        .into(),
        EffectKind::DropShadow => "Schlagschatten".into(),
        EffectKind::InnerShadow => "Innerer Schatten".into(),
        EffectKind::InnerGlow => "Inneres Glühen".into(),
        EffectKind::SolidShadow => "Einfarbiger Schatten".into(),
        EffectKind::ColorFill => "Farbfüllung".into(),
        EffectKind::PhotoshopLive => "Photoshop-Live-Effekte".into(),
        EffectKind::GaussianBlur => "Gaußscher Weichzeichner".into(),
        EffectKind::Blur => "Weichzeichnen".into(),
        EffectKind::BlurMore => "Stark weichzeichnen".into(),
        EffectKind::MotionBlur => "Bewegungsunschärfe".into(),
        EffectKind::RadialBlur => "Radialer Weichzeichner".into(),
        EffectKind::ZoomBlur => "Zoom-Weichzeichner".into(),
        EffectKind::Sharpen => "Scharfzeichnen".into(),
        EffectKind::SharpenMore => "Stark scharfzeichnen".into(),
        EffectKind::UnsharpMask => "Unscharf maskieren".into(),
        EffectKind::AddNoise => "Rauschen hinzufügen".into(),
        EffectKind::BrightnessContrast => "Helligkeit/Kontrast".into(),
        EffectKind::HueSaturation => "Farbton/Sättigung".into(),
        EffectKind::Curves => "Gradationskurven".into(),
        EffectKind::Levels => "Tonwertkorrektur".into(),
        EffectKind::AutoLevels => "Auto-Tonwertkorrektur".into(),
        EffectKind::Invert => "Umkehren".into(),
        EffectKind::FindEdges => "Kanten suchen".into(),
        EffectKind::ConvertToAlpha => "In Alpha konvertieren".into(),
        _ => e.ui_name().map(|s| s.to_string()).unwrap_or_else(|| "Effekt".into()),
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
    )
}

/// Menü „+“ an einer Antwort (Kategorie → Filter, Optionen „Alle ein/aus“).
pub fn add_menu(ui: &mut Ui, resp: &Response, st: &mut AppState) {
    let theme = Theme::current(ui.ctx());
    let mut chosen: Option<usize> = None;
    let mut special: Option<&str> = None;
    egui::Popup::menu(resp).frame(crate::menu::menu_frame(&theme)).show(|ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        ui.menu_button("Optionen", |ui| {
            if fields::list_item(ui, "Alle ein", false).clicked() {
                special = Some("on");
                ui.close();
            }
            if fields::list_item(ui, "Alle aus", false).clicked() {
                special = Some("off");
                ui.close();
            }
            if fields::list_item(ui, "Als Stil speichern...", false).clicked() {
                special = Some("style");
                ui.close();
            }
        });
        let mut groups: Vec<&str> = Vec::new();
        for f in FX {
            if !groups.contains(&f.group) {
                groups.push(f.group);
            }
        }
        for g in groups {
            ui.menu_button(g, |ui| {
                for (i, f) in FX.iter().enumerate().filter(|(_, f)| f.group == g) {
                    if fields::list_item(ui, f.label, false).clicked() {
                        chosen = Some(i);
                        ui.close();
                    }
                }
            });
        }
        let _ = fields::list_item(ui, "Photoshop-Live-Effekte (noch nicht verfügbar)", false);
    });
    match special {
        Some("on") => st.run("pi.effects.visible", json!({"visible": true})),
        Some("off") => st.run("pi.effects.visible", json!({"visible": false})),
        Some("style") => st.dialogs.open_new_style(),
        _ => {}
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
            if let Some(d) = super::catalog::filter_form(cmd, true) {
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
                params.insert("BevelWidth".into(), json!(n("width").to_string()));
                params.insert("BevelContrast".into(), json!(n("contrast").to_string()));
                params.insert("AngleSoftness".into(), json!(n("soft").to_string()));
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
    let (title, fields): (&str, Vec<Field>) = match name {
        "dropShadow" | "innerShadow" => {
            let (col, op) = hex_alpha(&txt(e, "ShadowColor", "#000000a6"));
            (
                if name == "dropShadow" { "Schlagschatten" } else { "Innerer Schatten" },
                vec![
                    Field::slider("dist", "Abstand:", num(e, "ShadowDistance", 7.0), 0.0, 100.0),
                    Field::slider("opacity", "Deckkraft:", op, 0.0, 100.0).unit("%"),
                    Field::slider("soft", "Weichheit:", num(e, "ShadowBlur", 4.0), 0.0, 30.0),
                    Field::slider("angle", "Winkel:", num(e, "ShadowAngle", 315.0), 0.0, 360.0).unit("°"),
                    Field::color("color", "Farbe:", &col),
                    Field::check("knockout", "Objekt aussparen", txt(e, "ShadowType", "0") == "1"),
                ],
            )
        }
        "glow" => {
            let (col, op) = hex_alpha(&txt(e, "OuterBevelColor", &format!("#{:02x}{:02x}{:02x}ff", fill.r(), fill.g(), fill.b())));
            (
                "Glühen",
                vec![
                    Field::slider("width", "Breite:", num(e, "GlowWidth", 4.0), 0.0, 35.0),
                    Field::slider("opacity", "Deckkraft:", if e.is_some() { op } else { 65.0 }, 0.0, 100.0).unit("%"),
                    Field::slider("soft", "Weichheit:", num(e, "MaskSoftness", 4.0), 0.0, 30.0),
                    Field::slider("offset", "Glühversatz:", num(e, "GlowStartDistance", 0.0), 0.0, 100.0),
                    Field::color("color", "Farbe:", &col),
                ],
            )
        }
        "innerGlow" => {
            let (col, op) = hex_alpha(&txt(e, "OuterBevelColor", &format!("#{:02x}{:02x}{:02x}ff", fill.r(), fill.g(), fill.b())));
            (
                "Inneres Glühen",
                vec![
                    Field::slider("width", "Breite:", num(e, "GlowWidth", 4.0), 0.0, 35.0),
                    Field::slider("opacity", "Deckkraft:", if e.is_some() { op } else { 65.0 }, 0.0, 100.0).unit("%"),
                    Field::slider("soft", "Weichheit:", num(e, "MaskSoftness", 4.0), 0.0, 30.0),
                    Field::slider("offset", "Glühversatz:", num(e, "GlowStartDistance", 0.0), 0.0, 100.0),
                    Field::color("color", "Farbe:", &col),
                ],
            )
        }
        "solidShadow" => {
            let c = txt(e, "color", "#000000ff");
            (
                "Einfarbiger Schatten",
                vec![
                    Field::slider("angle", "Winkel:", num(e, "angle", 315.0), 0.0, 360.0).unit("°"),
                    Field::slider("dist", "Abstand:", num(e, "distance", 7.0), 0.0, 100.0),
                    Field::color("color", "Farbe:", if c == "#ffffff00" { "#000000ff" } else { &c }),
                    Field::check("original", "Originalfarben verwenden", c == "#ffffff00"),
                ],
            )
        }
        "colorFill" => {
            let bl = num(e, "Blendmode", 0.0) as i64;
            let names: Vec<&str> = crate::inspector::obj_quads::BLEND_ITEMS.iter().map(|b| b.0).collect();
            let vals: Vec<Value> = crate::inspector::obj_quads::BLEND_ITEMS.iter().map(|b| json!(b.1.max(0))).collect();
            let idx = crate::inspector::obj_quads::BLEND_ITEMS.iter().position(|b| b.1 as i64 == bl).unwrap_or(0);
            (
                "Farbfüllung",
                vec![
                    Field::choice_of("blend", "Mischmodus:", &names, vals, idx),
                    Field::color("color", "Farbe:", &txt(e, "Color", "#ff0000ff")),
                    Field::slider("opacity", "Deckkraft:", num(e, "Opacity", 100.0), 0.0, 100.0).unit("%"),
                ],
            )
        }
        _ => {
            let bt = bevel.unwrap_or(0);
            let emboss = bt == 2 || bt == 3;
            let mut v = Vec::new();
            if emboss {
                v.push(Field::choice_of("type", "Art:", &["Prägung erhaben", "Prägung eingelassen"], vec![json!(2), json!(3)], if bt == 3 { 1 } else { 0 }));
            } else {
                v.push(Field::choice_of("type", "Art:", &["Abschrägung innen", "Abschrägung außen"], vec![json!(0), json!(1)], if bt == 1 { 1 } else { 0 }));
                v.push(Field::choice("slope", "Kantenform:", &["Flach", "Glatt", "Abfallend", "Rahmen 1", "Rahmen 2", "Ring", "Kräuseln"], num(e, "SlopeType", 0.0).clamp(0.0, 6.0) as usize));
            }
            v.push(Field::slider("width", "Breite:", num(e, "BevelWidth", 10.0), 0.0, 35.0));
            v.push(Field::slider("contrast", "Kontrast:", num(e, "BevelContrast", 75.0), 0.0, 100.0).unit("%"));
            v.push(Field::slider("soft", "Weichheit:", num(e, "AngleSoftness", 3.0), 0.0, 10.0));
            v.push(Field::slider("angle", "Winkel:", num(e, "LightAngle", 135.0), 0.0, 360.0).unit("°"));
            if emboss {
                v.push(Field::check("show", "Objekt anzeigen", txt(e, "ShowObject", "false").eq_ignore_ascii_case("true")));
            } else {
                v.push(Field::choice("button", "Schaltflächen-Vorgabe:", &["Erhaben", "Hervorgehoben", "Eingelassen", "Umgekehrt"], num(e, "ButtonState", 0.0).clamp(0.0, 3.0) as usize));
                if bt == 1 {
                    v.push(Field::color("color", "Farbe:", &txt(e, "OuterBevelColor", &format!("#{:02x}{:02x}{:02x}ff", fill.r(), fill.g(), fill.b()))));
                }
            }
            (if emboss { "Prägung" } else { "Abschrägung" }, v)
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
    FormDialog::new(&format!("fx-{name}"), title, command, fields).with_base(base).with_transform(effect_params).with_preview()
}

/// Dialog eines vorhandenen Bitmap-Filter-Effekts (Parameter wie im Dateiformat, 06 §2.2).
fn filter_effect_form(e: &Effect, index: usize) -> Option<FormDialog> {
    let n = |k: &str, d: f64| e.get_f64(k).unwrap_or(d);
    let (title, keys, fields): (&str, Vec<(&str, &str)>, Vec<Field>) = match e.kind() {
        EffectKind::GaussianBlur => ("Gaußscher Weichzeichner", vec![("radius", "gaussian_blur_radius")], vec![Field::slider("radius", "Radius:", n("gaussian_blur_radius", 1.0), 0.1, 250.0).decimals(1)]),
        EffectKind::MotionBlur => (
            "Bewegungsunschärfe",
            vec![("angle", "motion_blur_angle"), ("distance", "motion_blur_distance")],
            vec![Field::slider("angle", "Winkel:", n("motion_blur_angle", 90.0), 0.0, 360.0), Field::slider("distance", "Abstand:", n("motion_blur_distance", 10.0), 1.0, 100.0)],
        ),
        EffectKind::RadialBlur => (
            "Radiales Weichzeichnen",
            vec![("amount", "radial_blur_amount"), ("quality", "radial_blur_quality")],
            vec![Field::slider("amount", "Stärke:", n("radial_blur_amount", 30.0), 1.0, 100.0), Field::slider("quality", "Qualität:", n("radial_blur_quality", 20.0), 1.0, 100.0)],
        ),
        EffectKind::ZoomBlur => (
            "Zentriertes Weichzeichnen",
            vec![("amount", "zoom_blur_amount"), ("quality", "zoom_blur_quality")],
            vec![Field::slider("amount", "Stärke:", n("zoom_blur_amount", 30.0), 1.0, 100.0), Field::slider("quality", "Qualität:", n("zoom_blur_quality", 20.0), 1.0, 100.0)],
        ),
        EffectKind::UnsharpMask => (
            "Unscharf maskieren",
            vec![("amount", "unsharp_mask_amount"), ("radius", "unsharp_mask_radius"), ("threshold", "unsharp_mask_threshold")],
            vec![
                Field::slider("amount", "Schärfe:", n("unsharp_mask_amount", 50.0), 1.0, 500.0),
                Field::slider("radius", "Pixelradius:", n("unsharp_mask_radius", 1.0), 0.1, 250.0).decimals(1),
                Field::slider("threshold", "Schwellenwert:", n("unsharp_mask_threshold", 0.0), 0.0, 255.0),
            ],
        ),
        EffectKind::AddNoise => (
            "Rauschen hinzufügen",
            vec![("amount", "add_noise_amount"), ("use_color", "add_noise_use_color")],
            vec![Field::slider("amount", "Stärke:", n("add_noise_amount", 50.0), 1.0, 400.0), Field::check("use_color", "Farbe", e.get_bool("add_noise_use_color").unwrap_or(true))],
        ),
        EffectKind::BrightnessContrast => (
            "Helligkeit/Kontrast",
            vec![("brightness", "brightness_amount"), ("contrast", "contrast_amount")],
            vec![Field::slider("brightness", "Helligkeit:", n("brightness_amount", 0.0), -100.0, 100.0), Field::slider("contrast", "Kontrast:", n("contrast_amount", 0.0), -100.0, 100.0)],
        ),
        EffectKind::HueSaturation => (
            "Farbton/Sättigung",
            vec![("hue", "hue_amount"), ("saturation", "saturation_amount"), ("lightness", "lightness_amount"), ("colorize", "hls_colorize")],
            vec![
                Field::slider("hue", "Farbton:", n("hue_amount", 0.0), -180.0, 180.0),
                Field::slider("saturation", "Sättigung:", n("saturation_amount", 0.0), -100.0, 100.0),
                Field::slider("lightness", "Helligkeit:", n("lightness_amount", 0.0), -100.0, 100.0),
                Field::check("colorize", "Färben", e.get_bool("hls_colorize").unwrap_or(false)),
            ],
        ),
        _ => return None,
    };
    let mapping: Vec<Value> = keys.iter().map(|(a, b)| json!([a, b])).collect();
    Some(FormDialog::new("fx-filter", title, "pi.effect.set", fields).with_base(json!({"index": index, "map": mapping})).with_transform(filter_effect_params).with_preview())
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
