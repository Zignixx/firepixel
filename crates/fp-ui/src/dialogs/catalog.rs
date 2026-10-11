//! Formular-Dialoge je Menübefehl (Felder, Bereiche und Kombifeld-Inhalte nach `dialoge.json`,
//! 11 §3.2/§3.5.5): Filter (5971, 5972, 6074, 6525, 19110, 19157, 19159, 19160) mit Live-Vorschau,
//! Auswahl (5976, 19105–19108, 6540–6542), Pfad (6006–6008), Text (6073), Symbol (19104, 6005),
//! Animation (19146), Leinwandfarbe (5977), Raster (5984), Hilfslinien (5975), Ebenen/Status
//! (5987, 19128, 5990, 5991, 6021), Statusleistentext (6012), HTML-Segment (14139),
//! Strichoptionen (5963–5965), Neuer Stil (19123).

use super::form::{Field, FormDialog};
use crate::app_state::AppState;
use serde_json::{json, Map, Value};

fn defaults<T: serde::Serialize + Default>() -> Value {
    serde_json::to_value(T::default()).unwrap_or(json!({}))
}

fn dnum(v: &Value, key: &str, d: f64) -> f64 {
    v.get(key).and_then(|x| x.as_f64()).unwrap_or(d)
}

/// Filterdialog (mit Vorschau); `live` = als Live-Effekt anwenden.
pub fn filter_form(cmd: &str, live: bool) -> Option<FormDialog> {
    use fp_raster::filters::*;
    let (id, title, fields): (&str, &str, Vec<Field>) = match cmd {
        "filter.blur.gaussian" => {
            let d = defaults::<GaussianBlurParams>();
            ("5971", "Gaußscher Weichzeichner", vec![Field::slider("radius", "Radius:", dnum(&d, "radius", 1.0), 0.1, 250.0).decimals(1)])
        }
        "filter.sharpen.unsharpMask" => {
            let d = defaults::<UnsharpMaskParams>();
            (
                "5972",
                "Unscharf maskieren",
                vec![
                    Field::slider("amount", "Schärfe:", dnum(&d, "amount", 50.0), 1.0, 500.0).unit("%"),
                    Field::slider("radius", "Pixelradius:", dnum(&d, "radius", 1.0), 0.1, 250.0).decimals(1),
                    Field::slider("threshold", "Schwellenwert:", dnum(&d, "threshold", 0.0), 0.0, 255.0),
                ],
            )
        }
        "filter.adjustColor.brightnessContrast" => (
            "6074",
            "Helligkeit/Kontrast",
            vec![Field::slider("brightness", "Helligkeit:", 0.0, -100.0, 100.0), Field::slider("contrast", "Kontrast:", 0.0, -100.0, 100.0)],
        ),
        "filter.blur.motion" => {
            let d = defaults::<MotionBlurParams>();
            ("6525", "Bewegungsunschärfe", vec![Field::slider("angle", "Winkel:", dnum(&d, "angle", 90.0), 0.0, 360.0).unit("°"), Field::slider("distance", "Abstand:", dnum(&d, "distance", 10.0), 1.0, 100.0)])
        }
        "filter.adjustColor.hueSaturation" => (
            "19110",
            "Farbton/Sättigung",
            vec![
                Field::slider("hue", "Farbton:", 0.0, -180.0, 180.0),
                Field::slider("saturation", "Sättigung:", 0.0, -100.0, 100.0),
                Field::slider("lightness", "Helligkeit:", 0.0, -100.0, 100.0),
                Field::check("colorize", "Färben", false),
            ],
        ),
        "filter.noise.addNoise" => {
            let d = defaults::<AddNoiseParams>();
            ("19157", "Rauschen hinzufügen", vec![Field::slider("amount", "Stärke:", dnum(&d, "amount", 50.0), 1.0, 400.0), Field::check("use_color", "Farbe", d.get("use_color").and_then(|x| x.as_bool()).unwrap_or(true))])
        }
        "filter.blur.radial" => {
            let d = defaults::<RadialBlurParams>();
            ("19159", "Radiales Weichzeichnen", vec![Field::slider("amount", "Stärke:", dnum(&d, "amount", 30.0), 1.0, 100.0), Field::slider("quality", "Qualität:", dnum(&d, "quality", 20.0), 1.0, 100.0)])
        }
        "filter.blur.zoom" => {
            let d = defaults::<ZoomBlurParams>();
            ("19160", "Zentriertes Weichzeichnen", vec![Field::slider("amount", "Stärke:", dnum(&d, "amount", 30.0), 1.0, 100.0), Field::slider("quality", "Qualität:", dnum(&d, "quality", 20.0), 1.0, 100.0)])
        }
        _ => return None,
    };
    let mut f = FormDialog::new(id, title, cmd, fields).with_preview();
    if live {
        f = f.with_base(json!({"live": true}));
    }
    Some(f)
}

fn hex_of(c: fp_color::Argb) -> String {
    crate::ui_commands::hex(c)
}

/// Formular zu einem Menübefehl (falls der Befehl ohne Parameter aufgerufen wurde).
pub fn form_for(st: &AppState, cmd: &str) -> Option<FormDialog> {
    if let Some(f) = filter_form(cmd, false) {
        return Some(f);
    }
    let d = st.session.active();
    let f = match cmd {
        "select.feather" => FormDialog::new("5976", "Weiche Auswahlkante", cmd, vec![Field::number("radius", "Radius:", 10.0, 1.0, 1000.0).unit("Pixel")]),
        "select.expandMarquee" => FormDialog::new("19105", "Auswahlrahmen erweitern", cmd, vec![Field::number("amount", "Erweitern um:", 1.0, 1.0, 100.0).unit("Pixel")]),
        "select.contractMarquee" => FormDialog::new("19106", "Auswahlrahmen verkleinern", cmd, vec![Field::number("amount", "Verkleinern um:", 1.0, 1.0, 100.0).unit("Pixel")]),
        "select.borderMarquee" => FormDialog::new("19107", "Auswahlrahmen begrenzen", cmd, vec![Field::number("amount", "Rand:", 1.0, 1.0, 100.0).unit("Pixel")]),
        "select.smoothMarquee" => FormDialog::new("19108", "Auswahlrahmen glätten", cmd, vec![Field::number("amount", "Stichprobenradius:", 1.0, 1.0, 100.0).unit("Pixel")]),
        "select.saveBitmapSelection" => FormDialog::new("6541", "Auswahl speichern", cmd, vec![Field::text("name", "Name:", "Auswahl 1")]),
        "select.restoreBitmapSelection" | "select.deleteBitmapSelection" => {
            let names: Vec<String> = d.and_then(|d| d.doc.pages.get(d.page)).and_then(|p| p.saved_selections.as_ref()).map(|v| v.iter().enumerate().map(|(i, s)| s.name.clone().unwrap_or_else(|| format!("Auswahl {}", i + 1))).collect()).unwrap_or_default();
            if names.is_empty() {
                return Some(FormDialog::new("6540", "Auswahl", "", vec![Field::note("Es sind keine gespeicherten Auswahlen vorhanden.")]));
            }
            let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
            let idx: Vec<Value> = (0..names.len()).map(|i| json!(i)).collect();
            let mut v = vec![Field::choice_of("index", "Auswahl:", &refs, idx, 0)];
            if cmd == "select.restoreBitmapSelection" {
                v.push(Field::radio_of("mode", "Vorgang:", &["Ersetzen", "Hinzufügen", "Subtrahieren", "Schnittmenge"], vec![json!("replace"), json!("add"), json!("subtract"), json!("intersect")], 0));
                FormDialog::new("6540", "Auswahl wiederherstellen", cmd, v).with_width(420.0)
            } else {
                FormDialog::new("6542", "Auswahl löschen", cmd, v)
            }
        }
        "modify.alterPath.simplify" => FormDialog::new("6006", "Vereinfachen", cmd, vec![Field::number("amount", "Betrag:", 1.0, 0.0, 100.0).decimals(1)]),
        "modify.alterPath.insetPath" => FormDialog::new(
            "6007",
            "Pfad einlassen",
            cmd,
            vec![
                Field::radio_of("direction", "Richtung:", &["Innen", "Außen"], vec![json!("inside"), json!("outside")], 0),
                Field::number("distance", "Breite:", 1.0, 0.0, 1000.0),
                Field::radio_of("join", "Ecken:", &["Abgeschrägt", "Rund", "Eckig"], vec![json!("bevel"), json!("round"), json!("miter")], 2),
                Field::number("miter", "Gehrungsgrenze:", 10.0, 4.0, 10.0),
            ],
        )
        .with_width(400.0),
        "modify.alterPath.expandStroke" => FormDialog::new(
            "6008",
            "Strich erweitern",
            cmd,
            vec![
                Field::number("width", "Breite:", 10.0, 0.1, 1000.0).decimals(1),
                Field::radio_of("join", "Ecken:", &["Abgeschrägt", "Rund", "Eckig"], vec![json!("bevel"), json!("round"), json!("miter")], 2),
                Field::number("miter", "Gehrungsgrenze:", 10.0, 1.0, 10.0),
                Field::radio_of("cap", "Enden:", &["Stumpf", "Eckig", "Rund"], vec![json!("butt"), json!("square"), json!("round")], 0),
            ],
        )
        .with_width(400.0),
        "text.size.other" => FormDialog::new("6073", "Textgröße", cmd, vec![Field::number("size", "Größe:", 12.0, 8.0, 96.0).unit("Punkt")]),
        "modify.symbol.tweenInstances" => FormDialog::new("6005", "Instanzen tweenen", cmd, vec![Field::number("steps", "Schritte:", 1.0, 1.0, 1000.0), Field::check("_distribute", "Auf Status verteilen", false)]).with_transform(tween_params),
        "modify.symbol.convertToSymbol" => FormDialog::new(
            "19104",
            "In Symbol konvertieren",
            cmd,
            vec![
                Field::text("name", "Name:", "Symbol"),
                Field::radio_of("kind", "Typ:", &["Grafik", "Animation", "Schaltfläche"], vec![json!("graphic"), json!("animation"), json!("button")], 0),
                Field::check("_nineslice", "Hilfslinien für 9-Slice-Skalierung aktivieren", false),
                Field::check("_common", "Für allgemeine Bibliothek speichern", false),
            ],
        )
        .with_base(json!({
            "_n": d.map(|d| d.doc.symbols.len()).unwrap_or(0),
            "_dir": crate::panels::library::library_roots().into_iter().next().map(|(_, p)| p.to_string_lossy().to_string()).unwrap_or_default(),
        }))
        .with_width(420.0)
        .with_transform(convert_symbol_params),
        "modify.animation.animateSelection" | "modify.animation.settings" => {
            let a = crate::inspector::obj_quads::selection(st).first().and_then(|e| match &e.kind {
                fp_model::ElementKind::Instance(i) => Some(i.anim),
                _ => None,
            });
            let (frames, dist, dir, scale, op0, op1, rot) = match &a {
                Some(a) => {
                    let dist = (a.offset.x as f64).hypot(a.offset.y as f64);
                    let dir = (-(a.offset.y as f64)).atan2(a.offset.x as f64).to_degrees().rem_euclid(360.0);
                    (a.frames as f64, dist, dir, a.scale as f64, a.start_opacity as f64 / 10.0, a.end_opacity as f64 / 10.0, a.rotation as f64)
                }
                None => (5.0, 72.0, 45.0, 100.0, 100.0, 100.0, 0.0),
            };
            FormDialog::new(
                "19146",
                "Animieren",
                cmd,
                vec![
                    Field::slider("frames", "Status:", frames, 1.0, 60.0),
                    Field::slider("_dist", "Bewegung:", dist.round(), 0.0, 250.0),
                    Field::slider("_dir", "Richtung:", dir.round(), 0.0, 360.0).unit("°"),
                    Field::slider("scale", "Skalierung auf:", scale, 0.0, 250.0).unit("%"),
                    Field::slider("startOpacity", "Deckkraft von:", op0.round(), 0.0, 100.0),
                    Field::slider("endOpacity", "bis:", op1.round(), 0.0, 100.0),
                    Field::slider("_rot", "Drehung:", rot.abs(), 0.0, 360.0).unit("°"),
                    Field::radio_of("_cw", "", &["Im Uhrzeigersinn", "Entgegen dem Uhrzeigersinn"], vec![json!(true), json!(false)], if rot < 0.0 { 1 } else { 0 }),
                ],
            )
            .with_width(380.0)
            .with_transform(animate_params)
        }
        "modify.canvas.canvasColor" => {
            let bg = d.and_then(|d| d.doc.pages.get(d.page)).map(|p| p.root.background).unwrap_or(fp_color::Argb(0xffff_ffff));
            let idx = if bg.a() == 0 {
                1
            } else if bg.0 == 0xffff_ffff {
                0
            } else {
                2
            };
            FormDialog::new(
                "5977",
                "Leinwandfarbe",
                cmd,
                vec![
                    Field::radio_of("_mode", "", &["Weiß", "Transparent", "Benutzerdefiniert"], vec![json!(0), json!(1), json!(2)], idx),
                    Field::color("_color", "Farbe:", &hex_of(if bg.a() == 0 { fp_color::Argb(0xff00_0000) } else { bg })),
                ],
            )
            .with_width(380.0)
            .with_transform(canvas_color_params)
        }
        "view.grid.editGrid" => {
            let g = d.and_then(|d| d.doc.pages.get(d.page)).and_then(|p| p.root.grid.clone());
            let (gw, gh, col) = g.map(|g| (g.size.x as f64, g.size.y as f64, g.color)).unwrap_or((36.0, 36.0, fp_color::Argb(0xff00_ffff)));
            let show = d.is_some_and(|d| d.view.show_grid);
            let snap = d.is_some_and(|d| d.view.snap_to_grid);
            FormDialog::new(
                "5984",
                "Raster bearbeiten",
                cmd,
                vec![Field::color("color", "Farbe:", &hex_of(col)), Field::check("show", "Raster anzeigen", show), Field::check("snap", "Am Raster ausrichten", snap), Field::number("width", "Abstand ↔:", gw, 1.0, 10000.0).unit("Pixel"), Field::number("height", "Abstand ↕:", gh, 1.0, 10000.0).unit("Pixel")],
            )
        }
        "view.guides.editGuides" => {
            let g = d.and_then(|d| d.doc.pages.get(d.page)).and_then(|p| p.root.guides.clone());
            let col = g.as_ref().and_then(|g| g.color).unwrap_or(fp_color::Argb(0xff00_ffff));
            let locked = d.is_some_and(|d| d.view.lock_guides);
            let show = d.is_some_and(|d| d.view.show_guides);
            let snap = d.is_some_and(|d| d.view.snap_to_guides);
            FormDialog::new(
                "5975",
                "Hilfslinien",
                "pi.guides",
                vec![
                    Field::color("color", "Farbe:", &hex_of(col)),
                    Field::check("show", "Hilfslinien anzeigen", show),
                    Field::check("snap", "An Hilfslinien ausrichten", snap),
                    Field::check("locked", "Hilfslinien sperren", locked),
                    Field::check("clear", "Alle löschen", false),
                    Field::note("Segmenthilfslinien werden mit den Segmenten angezeigt (Ansicht ▸ Segmenthilfslinien)."),
                ],
            )
        }
        "edit.insert.layer" | "panel.layers.newLayer" => FormDialog::new("5987", "Neue Ebene", "panel.layers.newLayer", vec![Field::text("name", "Name:", &next_layer_name(st)), Field::check("shared", "Statusübergreifend freigeben", false)]).with_width(360.0),
        "panel.layers.duplicate" => FormDialog::new(
            "19128",
            "Ebene duplizieren",
            "layer.duplicate",
            vec![Field::number("_count", "Anzahl:", 1.0, 1.0, 100.0), Field::radio_of("_pos", "Einfügen:", &["Oben", "Vor aktueller Ebene"], vec![json!(0), json!(1)], 1)],
        )
        .with_width(380.0),
        "edit.insert.state" | "panel.states.add" => FormDialog::new(
            "5990",
            "Status hinzufügen",
            "state.add",
            vec![
                Field::number("count", "Anzahl:", 1.0, 1.0, 1000.0),
                Field::radio_of("_where", "Einfügen:", &["Am Anfang", "Vor aktuellem Status", "Nach aktuellem Status", "Am Ende"], vec![json!(0), json!(1), json!(2), json!(3)], 2),
            ],
        )
        .with_width(420.0)
        .with_transform(add_states_params),
        "panel.states.copyTo" => {
            let n = d.map(|d| d.frame_count()).unwrap_or(1) as f64;
            FormDialog::new(
                "5991",
                "In Status kopieren",
                "pi.state.copyTo",
                vec![Field::radio_of("_which", "", &["Alle Status", "Vorheriger Status", "Nächster Status", "Bereich"], vec![json!(0), json!(1), json!(2), json!(3)], 0), Field::number("_from", "Von:", 1.0, 1.0, n), Field::number("_to", "Bis:", n, 1.0, n)],
            )
            .with_width(420.0)
            .with_transform(copy_states_params)
        }
        "panel.states.onionCustom" => {
            let (b, a) = d.and_then(|d| d.doc.pages.get(d.page)).map(|p| (p.root.onion_before as f64, p.root.onion_after as f64)).unwrap_or((1.0, 1.0));
            FormDialog::new("6021", "Zwiebelschichteneffekt", "pi.state.onionSkin", vec![Field::number("before", "Status davor anzeigen:", b, 0.0, 1000.0), Field::number("after", "Status danach anzeigen:", a, 0.0, 1000.0), Field::check("_multi", "Mehrfach-Status-Bearbeitung", false)]).with_width(380.0)
        }
        "panel.behaviors.statusText" => FormDialog::new("6012", "Statusleistentext festlegen", "pi.behavior.add", vec![Field::text("statusText", "Meldung:", "")]).with_base(json!({"action": 1, "event": 0})).with_width(400.0),
        _ => return None,
    };
    Some(f)
}

/// „Instanzen tweenen“ mit „Auf Status verteilen“: beides als ein Verlaufsschritt.
fn tween_params(d: &FormDialog) -> Value {
    let steps = d.get("steps");
    if d.get("_distribute").as_bool() == Some(true) {
        json!({"commands": [{"command": "modify.symbol.tweenInstances", "params": {"steps": steps}}, {"command": "pi.state.distribute", "params": {}}], "label": "Instanzen tweenen"})
    } else {
        json!({"steps": steps})
    }
}

/// „In Symbol konvertieren“ mit 9-Slice-Hilfslinien und Speichern in der allgemeinen Bibliothek.
fn convert_symbol_params(d: &FormDialog) -> Value {
    let name = d.get("name");
    let kind = d.get("kind");
    let nine = d.get("_nineslice").as_bool() == Some(true);
    let common = d.get("_common").as_bool() == Some(true);
    if !nine && !common {
        return json!({"name": name, "kind": kind});
    }
    let n = d.base.get("_n").and_then(|v| v.as_u64()).unwrap_or(0);
    let mut cmds = vec![json!({"command": "modify.symbol.convertToSymbol", "params": {"name": name, "kind": kind}})];
    if nine {
        cmds.push(json!({"command": "panel.library.nineSlice", "params": {"index": n, "on": true}}));
    }
    if common && let Some(dir) = d.base.get("_dir").and_then(|v| v.as_str()) {
        let file = std::path::Path::new(dir).join(format!("{}.png", name.as_str().unwrap_or("Symbol")));
        cmds.push(json!({"command": "panel.library.saveToCommon", "params": {"index": n, "path": file.to_string_lossy()}}));
    }
    json!({"commands": cmds, "label": "In Symbol konvertieren"})
}

fn next_layer_name(st: &AppState) -> String {
    let n = st.session.active().and_then(|d| d.doc.pages.get(d.page)).map(|p| p.root.layers.len()).unwrap_or(1);
    format!("Ebene {n}")
}

fn animate_params(d: &FormDialog) -> Value {
    let v = d.raw_values();
    let n = |k: &str| v.get(k).and_then(|x| x.as_f64()).unwrap_or(0.0);
    let (dist, dir) = (n("_dist"), n("_dir").to_radians());
    let cw = v.get("_cw").and_then(|x| x.as_bool()).unwrap_or(true);
    let rot = if cw { n("_rot") } else { -n("_rot") };
    json!({
        "frames": n("frames") as i64,
        "offsetX": (dist * dir.cos()).round(),
        "offsetY": (-dist * dir.sin()).round(),
        "scale": n("scale"),
        "startOpacity": n("startOpacity"),
        "endOpacity": n("endOpacity"),
        "rotation": rot as i64,
    })
}

fn canvas_color_params(d: &FormDialog) -> Value {
    match d.get("_mode").as_i64().unwrap_or(0) {
        1 => json!({"color": "#ffffff00"}),
        2 => json!({"color": d.get("_color")}),
        _ => json!({"color": "#ffffffff"}),
    }
}

fn add_states_params(d: &FormDialog) -> Value {
    let count = d.get("count").as_f64().unwrap_or(1.0) as i64;
    let at = d.base.get("frame").and_then(|x| x.as_i64()).unwrap_or(0);
    let n = d.base.get("frames").and_then(|x| x.as_i64()).unwrap_or(1);
    let pos = match d.get("_where").as_i64().unwrap_or(2) {
        0 => 0,
        1 => at,
        3 => n,
        _ => at + 1,
    };
    json!({"count": count, "at": pos})
}

fn copy_states_params(d: &FormDialog) -> Value {
    let cur = d.base.get("frame").and_then(|x| x.as_i64()).unwrap_or(0);
    let n = d.base.get("frames").and_then(|x| x.as_i64()).unwrap_or(1);
    let (from, to) = match d.get("_which").as_i64().unwrap_or(0) {
        1 => ((cur - 1).max(0), (cur - 1).max(0)),
        2 => ((cur + 1).min(n - 1), (cur + 1).min(n - 1)),
        3 => (d.get("_from").as_f64().unwrap_or(1.0) as i64 - 1, d.get("_to").as_f64().unwrap_or(1.0) as i64 - 1),
        _ => (0, n - 1),
    };
    json!({"from": from.max(0), "to": to.max(0)})
}

/// Basisparameter abhängig vom Dokument (aktueller Status, Anzahl) nachtragen.
pub fn with_doc_base(st: &AppState, mut f: FormDialog) -> FormDialog {
    if let Some(d) = st.session.active() {
        let mut m = match &f.base {
            Value::Object(o) => o.clone(),
            _ => Map::new(),
        };
        if matches!(f.id.as_str(), "5990" | "5991") {
            m.insert("frame".into(), json!(d.frame));
            m.insert("frames".into(), json!(d.frame_count()));
        }
        f.base = Value::Object(m);
    }
    f
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_forms_have_ranges() {
        let f = filter_form("filter.blur.gaussian", false).expect("Gauß");
        assert!(f.preview);
        assert_eq!(f.command, "filter.blur.gaussian");
        let g = filter_form("filter.noise.addNoise", true).expect("Rauschen");
        assert_eq!(g.params()["live"], json!(true));
        assert!(filter_form("filter.unknown", false).is_none());
    }
}
