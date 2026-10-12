//! Formular-Dialoge je Menübefehl (Felder, Bereiche und Kombifeld-Inhalte nach `dialoge.json`,
//! 11 §3.2/§3.5.5): Filter (5971, 5972, 6074, 6525, 19110, 19157, 19159, 19160) mit Live-Vorschau,
//! Auswahl (5976, 19105–19108, 6540–6542), Pfad (6006–6008), Text (6073), Symbol (19104, 6005),
//! Animation (19146), Leinwandfarbe (5977), Raster (5984), Hilfslinien (5975), Ebenen/Status
//! (5987, 19128, 5990, 5991, 6021), Statusleistentext (6012), HTML-Segment (14139),
//! Strichoptionen (5963–5965), Neuer Stil (19123).

use super::form::{Field, FormDialog, Pict};
use crate::inspector::scale;
use super::t;
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
    let (id, title, fields): (&str, String, Vec<Field>) = match cmd {
        "filter.blur.gaussian" => {
            let d = defaults::<GaussianBlurParams>();
            ("5971", t("dlg-title-gaussian-blur"), vec![Field::slider("radius", t("dlg-lbl-radius").as_str(), dnum(&d, "radius", 1.0), 0.1, 250.0).decimals(1).scale(scale::BLUR_RADIUS)])
        }
        "filter.sharpen.unsharpMask" => {
            let d = defaults::<UnsharpMaskParams>();
            (
                "5972",
                t("dlg-title-unsharp-mask"),
                vec![
                    Field::slider("amount", t("dlg-lbl-sharpen-amount").as_str(), dnum(&d, "amount", 50.0), 1.0, 500.0).unit("%").scale(scale::SHARPEN_AMOUNT),
                    Field::slider("radius", t("dlg-lbl-pixel-radius").as_str(), dnum(&d, "radius", 1.0), 0.1, 250.0).decimals(1).scale(scale::BLUR_RADIUS),
                    Field::slider("threshold", t("dlg-lbl-threshold").as_str(), dnum(&d, "threshold", 0.0), 0.0, 255.0).scale(scale::THRESHOLD),
                ],
            )
        }
        "filter.adjustColor.brightnessContrast" => (
            "6074",
            t("dlg-title-brightness-contrast"),
            vec![Field::slider("brightness", t("dlg-lbl-brightness").as_str(), 0.0, -100.0, 100.0), Field::slider("contrast", t("dlg-lbl-contrast").as_str(), 0.0, -100.0, 100.0)],
        ),
        "filter.blur.motion" => {
            let d = defaults::<MotionBlurParams>();
            ("6525", t("dlg-title-motion-blur"), vec![Field::slider("angle", t("dlg-lbl-angle").as_str(), dnum(&d, "angle", 90.0), 0.0, 360.0).unit("°").popup(), Field::slider("distance", t("dlg-lbl-distance").as_str(), dnum(&d, "distance", 10.0), 1.0, 100.0).popup()])
        }
        "filter.adjustColor.hueSaturation" => (
            "19110",
            t("dlg-title-hue-saturation"),
            vec![
                Field::slider("hue", t("dlg-lbl-hue").as_str(), 0.0, -180.0, 180.0).scale(scale::HUE),
                Field::slider("saturation", t("dlg-lbl-saturation").as_str(), 0.0, -100.0, 100.0).scale(scale::SATURATION),
                Field::slider("lightness", t("dlg-lbl-brightness").as_str(), 0.0, -100.0, 100.0),
                Field::check("colorize", t("dlg-lbl-colorize").as_str(), false),
            ],
        ),
        "filter.noise.addNoise" => {
            let d = defaults::<AddNoiseParams>();
            ("19157", t("dlg-title-add-noise"), vec![Field::slider("amount", t("dlg-lbl-amount").as_str(), dnum(&d, "amount", 50.0), 1.0, 400.0).popup(), Field::check("use_color", t("dlg-lbl-color-check").as_str(), d.get("use_color").and_then(|x| x.as_bool()).unwrap_or(true))])
        }
        "filter.blur.radial" => {
            let d = defaults::<RadialBlurParams>();
            ("19159", t("dlg-title-radial-blur"), vec![Field::slider("amount", t("dlg-lbl-amount").as_str(), dnum(&d, "amount", 30.0), 1.0, 100.0).popup(), Field::slider("quality", t("dlg-lbl-quality").as_str(), dnum(&d, "quality", 20.0), 1.0, 100.0).popup()])
        }
        "filter.blur.zoom" => {
            let d = defaults::<ZoomBlurParams>();
            ("19160", t("dlg-title-zoom-blur"), vec![Field::slider("amount", t("dlg-lbl-amount").as_str(), dnum(&d, "amount", 30.0), 1.0, 100.0).popup(), Field::slider("quality", t("dlg-lbl-quality").as_str(), dnum(&d, "quality", 20.0), 1.0, 100.0).popup()])
        }
        _ => return None,
    };
    let mut f = FormDialog::new(id, &title, cmd, fields).with_preview();
    if live {
        f = f.with_base(json!({"live": true}));
    }
    Some(f)
}

/// Ecken als Symbolknöpfe in der Reihenfolge des Originals (Gehrung, abgeschrägt, rund); Codes der
/// Voreinstellungen `InsetJoinType`/`ExpandJoinType`: 0 = rund (Vorgabe), 1 = abgeschrägt, 2 = Gehrung.
fn join_field() -> Field {
    Field::radio_of("join", t("dlg-lbl-corners").as_str(), &[t("dlg-join-miter").as_str(), t("dlg-join-bevel").as_str(), t("dlg-join-round").as_str()], vec![json!("miter"), json!("bevel"), json!("round")], 2)
        .icons(&[Pict::JoinMiter, Pict::JoinBevel, Pict::JoinRound])
        .pref_codes(&[2, 1, 0])
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
        "select.feather" => FormDialog::new("5976", t("dlg-title-feather").as_str(), cmd, vec![Field::number("radius", t("dlg-lbl-radius").as_str(), 10.0, 1.0, 1000.0).unit("dlg-unit-pixels")]),
        "select.expandMarquee" => FormDialog::new("19105", t("dlg-title-expand-marquee").as_str(), cmd, vec![Field::number("amount", t("dlg-lbl-expand-by").as_str(), 10.0, 1.0, 100.0).unit("dlg-unit-pixels")]).with_prefs(&[("amount", "ExpandSelectionAmount")]),
        "select.contractMarquee" => FormDialog::new("19106", t("dlg-title-contract-marquee").as_str(), cmd, vec![Field::number("amount", t("dlg-lbl-contract-by").as_str(), 10.0, 1.0, 100.0).unit("dlg-unit-pixels")]).with_prefs(&[("amount", "ContractSelectionAmount")]),
        "select.borderMarquee" => FormDialog::new("19107", t("dlg-title-border-marquee").as_str(), cmd, vec![Field::number("amount", t("dlg-lbl-border").as_str(), 10.0, 1.0, 100.0).unit("dlg-unit-pixels")]).with_prefs(&[("amount", "BorderSelectionAmount")]),
        "select.smoothMarquee" => FormDialog::new("19108", t("dlg-title-smooth-marquee").as_str(), cmd, vec![Field::number("amount", t("dlg-lbl-sample-radius").as_str(), 10.0, 1.0, 100.0).unit("dlg-unit-pixels")]).with_prefs(&[("amount", "SmoothSelectionAmount")]),
        "select.saveBitmapSelection" => FormDialog::new("6541", t("dlg-title-save-selection").as_str(), cmd, vec![Field::text("name", t("dlg-lbl-name").as_str(), &fp_i18n::tr!("dlg-selection-default-name", n = 1))]),
        "select.restoreBitmapSelection" | "select.deleteBitmapSelection" => {
            let names: Vec<String> = d.and_then(|d| d.doc.pages.get(d.page)).and_then(|p| p.saved_selections.as_ref()).map(|v| v.iter().enumerate().map(|(i, s)| s.name.clone().unwrap_or_else(|| fp_i18n::tr!("dlg-selection-default-name", n = i + 1))).collect()).unwrap_or_default();
            if names.is_empty() {
                return Some(FormDialog::new("6540", t("dlg-title-selection").as_str(), "", vec![Field::note(t("dlg-no-saved-selections").as_str())]));
            }
            let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
            let idx: Vec<Value> = (0..names.len()).map(|i| json!(i)).collect();
            let mut v = vec![Field::choice_of("index", t("dlg-lbl-selection").as_str(), &refs, idx, 0)];
            if cmd == "select.restoreBitmapSelection" {
                v.push(Field::radio_of("mode", t("dlg-lbl-operation").as_str(), &[t("dlg-op-replace").as_str(), t("dlg-op-add").as_str(), t("dlg-op-subtract").as_str(), t("dlg-op-intersect").as_str()], vec![json!("replace"), json!("add"), json!("subtract"), json!("intersect")], 0));
                FormDialog::new("6540", t("dlg-title-restore-selection").as_str(), cmd, v).with_width(420.0)
            } else {
                FormDialog::new("6542", t("dlg-title-delete-selection").as_str(), cmd, v)
            }
        }
        "modify.alterPath.simplify" => FormDialog::new("6006", t("dlg-title-simplify").as_str(), cmd, vec![Field::number("amount", t("dlg-lbl-simplify-amount").as_str(), 10.0, 0.0, 100.0).popup()]).with_prefs(&[("amount", "SimplifyAmount")]),
        "modify.alterPath.insetPath" => FormDialog::new(
            "6007",
            t("dlg-title-inset-path").as_str(),
            cmd,
            vec![
                Field::radio_of("direction", t("dlg-lbl-direction").as_str(), &[t("dlg-dir-inside").as_str(), t("dlg-dir-outside").as_str()], vec![json!("inside"), json!("outside")], 0).pref_codes(&[0, 1]),
                Field::number("distance", t("dlg-lbl-width").as_str(), 10.0, 0.0, 1000.0).popup(),
                join_field(),
                Field::number("miter", t("dlg-lbl-miter-limit").as_str(), 10.0, 4.0, 10.0).decimals(2),
            ],
        )
        .with_width(400.0)
        .with_prefs(&[("direction", "InsetDirection"), ("distance", "InsetAmount"), ("join", "InsetJoinType"), ("miter", "InsetMiterLimit")]),
        "modify.alterPath.expandStroke" => FormDialog::new(
            "6008",
            t("dlg-title-expand-stroke").as_str(),
            cmd,
            vec![
                Field::number("width", t("dlg-lbl-width").as_str(), 10.0, 1.0, 1000.0).popup(),
                join_field(),
                Field::number("miter", t("dlg-lbl-miter-limit").as_str(), 10.0, 1.0, 10.0).decimals(2),
                Field::radio_of("cap", t("dlg-lbl-caps").as_str(), &[t("dlg-cap-butt").as_str(), t("dlg-cap-square").as_str(), t("dlg-cap-round").as_str()], vec![json!("butt"), json!("square"), json!("round")], 0)
                    .icons(&[Pict::CapButt, Pict::CapSquare, Pict::CapRound])
                    .pref_codes(&[0, 1, 2]),
            ],
        )
        .with_width(400.0)
        .with_prefs(&[("width", "ExpandWidth"), ("join", "ExpandJoinType"), ("miter", "ExpandMiterLimit"), ("cap", "ExpandCapType")]),
        "text.size.other" => FormDialog::new("6073", t("dlg-title-text-size").as_str(), cmd, vec![Field::number("size", t("dlg-lbl-size").as_str(), 12.0, 8.0, 96.0).unit("dlg-unit-points")]),
        "modify.symbol.tweenInstances" => FormDialog::new("6005", t("dlg-title-tween-instances").as_str(), cmd, vec![Field::number("steps", t("dlg-lbl-steps").as_str(), 1.0, 1.0, 1000.0), Field::check("_distribute", t("dlg-lbl-distribute-states").as_str(), false)]).with_transform(tween_params),
        "modify.symbol.convertToSymbol" => FormDialog::new(
            "19104",
            t("dlg-title-convert-to-symbol").as_str(),
            cmd,
            vec![
                Field::text("name", t("dlg-lbl-name").as_str(), t("dlg-symbol-default-name").as_str()),
                Field::radio_of("kind", t("dlg-lbl-type").as_str(), &[t("dlg-symbol-graphic").as_str(), t("dlg-symbol-animation").as_str(), t("dlg-symbol-button").as_str()], vec![json!("graphic"), json!("animation"), json!("button")], 0),
                Field::check("_nineslice", t("dlg-lbl-enable-nineslice").as_str(), false),
                Field::check("_common", t("dlg-lbl-save-common-library").as_str(), false),
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
                None => (5.0, 72.0, 0.0, 100.0, 100.0, 100.0, 0.0),
            };
            FormDialog::new(
                "19146",
                t("dlg-title-animate").as_str(),
                cmd,
                vec![
                    Field::slider("frames", t("dlg-lbl-states").as_str(), frames, 1.0, 60.0).popup(),
                    Field::slider("_dist", t("dlg-lbl-movement").as_str(), dist.round(), 0.0, 250.0).popup(),
                    Field::slider("_dir", t("dlg-lbl-direction").as_str(), dir.round(), 0.0, 360.0).unit("°").popup(),
                    Field::slider("scale", t("dlg-lbl-scale-to").as_str(), scale, 0.0, 250.0).unit("%").popup(),
                    Field::slider("startOpacity", t("dlg-lbl-opacity-from").as_str(), op0.round(), 0.0, 100.0).popup(),
                    Field::slider("endOpacity", t("dlg-lbl-opacity-to").as_str(), op1.round(), 0.0, 100.0).popup(),
                    Field::slider("_rot", t("dlg-lbl-rotation").as_str(), rot.abs(), 0.0, 360.0).unit("°").popup(),
                    Field::radio_of("_cw", "", &[t("dlg-cw").as_str(), t("dlg-ccw").as_str()], vec![json!(true), json!(false)], if rot < 0.0 { 1 } else { 0 }),
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
                t("dlg-canvas-color-group").as_str(),
                cmd,
                vec![
                    Field::radio_of("_mode", "", &[t("dlg-canvas-white").as_str(), t("dlg-canvas-transparent").as_str(), t("dlg-canvas-custom-plain").as_str()], vec![json!(0), json!(1), json!(2)], idx),
                    Field::color("_color", t("dlg-lbl-color").as_str(), &hex_of(if bg.a() == 0 { fp_color::Argb(0xff00_0000) } else { bg })),
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
                t("dlg-title-edit-grid").as_str(),
                cmd,
                vec![Field::color("color", t("dlg-lbl-color").as_str(), &hex_of(col)), Field::check("show", t("dlg-lbl-show-grid").as_str(), show), Field::check("snap", t("dlg-lbl-snap-grid").as_str(), snap), Field::number("width", t("dlg-lbl-grid-h").as_str(), gw, 1.0, 10000.0).unit("dlg-unit-pixels"), Field::number("height", t("dlg-lbl-grid-v").as_str(), gh, 1.0, 10000.0).unit("dlg-unit-pixels")],
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
                t("dlg-title-guides").as_str(),
                "pi.guides",
                vec![
                    Field::color("color", t("dlg-lbl-color").as_str(), &hex_of(col)),
                    Field::check("show", t("dlg-lbl-show-guides").as_str(), show),
                    Field::check("snap", t("dlg-lbl-snap-guides").as_str(), snap),
                    Field::check("locked", t("dlg-lbl-lock-guides").as_str(), locked),
                    Field::check("clear", t("dlg-lbl-clear-all").as_str(), false),
                    Field::note(t("dlg-guides-note").as_str()),
                ],
            )
        }
        "edit.insert.layer" | "panel.layers.newLayer" => FormDialog::new("5987", t("dlg-title-new-layer").as_str(), "panel.layers.newLayer", vec![Field::text("name", t("dlg-lbl-name").as_str(), &next_layer_name(st)), Field::check("shared", t("dlg-lbl-share-across-states").as_str(), false)]).with_width(360.0),
        "panel.layers.duplicate" => FormDialog::new(
            "19128",
            t("dlg-title-duplicate-layer").as_str(),
            "layer.duplicate",
            vec![Field::number("_count", t("dlg-lbl-count").as_str(), 1.0, 1.0, 100.0), Field::radio_of("_pos", t("dlg-lbl-insert").as_str(), &[t("dlg-insert-top").as_str(), t("dlg-insert-before-layer").as_str()], vec![json!(0), json!(1)], 1)],
        )
        .with_width(380.0),
        "edit.insert.state" | "panel.states.add" => FormDialog::new(
            "5990",
            t("dlg-title-add-states").as_str(),
            "state.add",
            vec![
                Field::number("count", t("dlg-lbl-count").as_str(), 1.0, 1.0, 1000.0),
                Field::radio_of("_where", t("dlg-lbl-insert").as_str(), &[t("dlg-insert-start").as_str(), t("dlg-insert-before-state").as_str(), t("dlg-insert-after-state").as_str(), t("dlg-insert-end").as_str()], vec![json!(0), json!(1), json!(2), json!(3)], 2),
            ],
        )
        .with_width(420.0)
        .with_transform(add_states_params),
        "panel.states.copyTo" => {
            let n = d.map(|d| d.frame_count()).unwrap_or(1) as f64;
            FormDialog::new(
                "5991",
                t("dlg-title-copy-to-states").as_str(),
                "pi.state.copyTo",
                vec![Field::radio_of("_which", "", &[t("dlg-states-all").as_str(), t("dlg-states-prev").as_str(), t("dlg-states-next").as_str(), t("dlg-states-range").as_str()], vec![json!(0), json!(1), json!(2), json!(3)], 0), Field::number("_from", t("dlg-lbl-from").as_str(), 1.0, 1.0, n), Field::number("_to", t("dlg-lbl-to").as_str(), n, 1.0, n)],
            )
            .with_width(420.0)
            .with_transform(copy_states_params)
        }
        "panel.states.onionCustom" => {
            let (b, a) = d.and_then(|d| d.doc.pages.get(d.page)).map(|p| (p.root.onion_before as f64, p.root.onion_after as f64)).unwrap_or((1.0, 1.0));
            FormDialog::new("6021", t("dlg-title-onion-skin").as_str(), "pi.state.onionSkin", vec![Field::number("before", t("dlg-lbl-onion-before").as_str(), b, 0.0, 1000.0), Field::number("after", t("dlg-lbl-onion-after").as_str(), a, 0.0, 1000.0), Field::check("_multi", t("dlg-lbl-multi-state-edit").as_str(), false)]).with_width(380.0)
        }
        "panel.behaviors.statusText" => FormDialog::new("6012", t("dlg-title-status-text").as_str(), "pi.behavior.add", vec![Field::text("statusText", t("dlg-lbl-message").as_str(), "")]).with_base(json!({"action": 1, "event": 0})).with_width(400.0),
        _ => return None,
    };
    // gemerkte Werte aus den Voreinstellungen (wie im Original)
    let mut f = f;
    f.load_prefs(&st.session.prefs);
    Some(f)
}

/// „Instanzen tweenen“ mit „Auf Status verteilen“: beides als ein Verlaufsschritt.
fn tween_params(d: &FormDialog) -> Value {
    let steps = d.get("steps");
    if d.get("_distribute").as_bool() == Some(true) {
        json!({"commands": [{"command": "modify.symbol.tweenInstances", "params": {"steps": steps}}, {"command": "pi.state.distribute", "params": {}}], "label": t("dlg-title-tween-instances")})
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
        let file = std::path::Path::new(dir).join(format!("{}.png", name.as_str().map(str::to_string).unwrap_or_else(|| t("dlg-symbol-default-name"))));
        cmds.push(json!({"command": "panel.library.saveToCommon", "params": {"index": n, "path": file.to_string_lossy()}}));
    }
    json!({"commands": cmds, "label": t("dlg-title-convert-to-symbol")})
}

fn next_layer_name(st: &AppState) -> String {
    let n = st.session.active().and_then(|d| d.doc.pages.get(d.page)).map(|p| p.root.layers.len()).unwrap_or(1);
    fp_i18n::tr!("msg-default-layer", n = n.to_string())
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
