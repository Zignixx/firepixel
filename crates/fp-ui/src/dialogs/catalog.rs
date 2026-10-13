//! Formular-Dialoge je Menübefehl (Felder, Bereiche und Kombifeld-Inhalte nach `dialoge.json`,
//! 11 §3.2/§3.5.5): Filter (5971, 5972, 6074, 6525, 19110, 19157, 19159, 19160) mit Live-Vorschau,
//! Auswahl (5976, 19105–19108, 6540–6542), Pfad (6006–6008), Text (6073), Symbol (19104, 6005),
//! Animation (19146), Leinwandfarbe (5977), Raster (5984), Hilfslinien (5975), Ebenen/Status
//! (5987, 19128, 5990, 5991, 6021), Statusleistentext (6012), HTML-Segment (14139),
//! Strichoptionen (5963–5965), Neuer Stil (19123).

use super::form::{number_input, CustomLayout, Field, FormDialog};
use crate::inspector::scale;
use super::{dlu, dlu_rect, t, text_w};
use crate::app_state::AppState;
use crate::inspector::fields;
use crate::theme::Theme;
use egui::{pos2, vec2, Align2, Rect, Stroke, Ui};
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
            ("5971", t("dlg-title-gaussian-blur"), vec![Field::slider("radius", t("dlg-lbl-blur-radius").as_str(), dnum(&d, "radius", 4.1), 0.1, 250.0).decimals(1).dot_decimal().scale(scale::BLUR_RADIUS)])
        }
        "filter.sharpen.unsharpMask" => {
            let d = defaults::<UnsharpMaskParams>();
            (
                "5972",
                t("dlg-title-unsharp-mask"),
                vec![
                    Field::slider("amount", t("dlg-lbl-sharpen-amount").as_str(), dnum(&d, "amount", 50.0), 1.0, 500.0).scale(scale::SHARPEN_AMOUNT),
                    Field::slider("radius", t("dlg-lbl-pixel-radius").as_str(), dnum(&d, "radius", 4.1), 0.1, 250.0).decimals(1).dot_decimal().scale(scale::BLUR_RADIUS),
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
            ("6525", t("dlg-title-motion-blur"), vec![Field::slider("angle", t("dlg-lbl-angle").as_str(), dnum(&d, "angle", 90.0), 0.0, 360.0).popup(), Field::slider("distance", t("dlg-lbl-motion-distance").as_str(), dnum(&d, "distance", 10.0), 1.0, 100.0).popup()])
        }
        "filter.adjustColor.hueSaturation" => (
            "19110",
            t("dlg-title-hue-saturation"),
            vec![
                Field::slider("hue", t("dlg-lbl-hue").as_str(), 0.0, -180.0, 180.0).scale(scale::HUE),
                Field::slider("saturation", t("dlg-lbl-saturation").as_str(), 0.0, -100.0, 100.0).scale(scale::SATURATION),
                Field::slider("lightness", t("dlg-lbl-lightness").as_str(), 0.0, -100.0, 100.0),
                Field::check("colorize", t("dlg-lbl-colorize").as_str(), false).beside_buttons(),
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
            ("19160", t("dlg-title-zoom-blur"), vec![Field::slider("amount", t("dlg-lbl-zoom-amount").as_str(), dnum(&d, "amount", 30.0), 1.0, 100.0).popup(), Field::slider("quality", t("dlg-lbl-quality").as_str(), dnum(&d, "quality", 20.0), 1.0, 100.0).popup()])
        }
        _ => return None,
    };
    let mut f = FormDialog::new(id, &title, cmd, fields).with_preview();
    // Anordnung des Originals (PM-6): Felder links, Knöpfe rechts; Rauschen, Bewegung, Radial und
    // Zentriert mit Startfokus OK (D45/D47/D49/D50)
    if matches!(id, "5971" | "5972" | "6074" | "19110" | "19157" | "6525" | "19159" | "19160") {
        f = f.with_side_buttons();
    }
    if matches!(id, "19157" | "6525" | "19159" | "19160") {
        f = f.with_focus_ok();
    }
    // Tab-Folge wie im Original (PM-6): D42 Schieber vor dem Feld und Vorschau vor OK, D46 erst die
    // Felder, dann Vorschau und Schieber, D41 Schieber keine Tab-Stopps
    match id {
        "6074" => f = f.with_side_order(super::form::SideOrder::SlidersFirst),
        "5972" => f = f.with_side_order(super::form::SideOrder::FieldsPreviewSliders),
        "19110" => f = f.without_slider_tab_stops(),
        _ => {}
    }
    // zuletzt per OK bestätigte Werte, app-weit wie im Original (PM-6 D41/D42/D45–D50); Abbrechen
    // merkt nichts. Eigene Schlüssel (das Original legt sie nicht in seiner Voreinstellungsdatei ab).
    let remembered: &[(&str, &str)] = match id {
        "5971" => &[("radius", "GaussianBlurRadius")],
        "5972" => &[("amount", "UnsharpMaskAmount"), ("radius", "UnsharpMaskRadius"), ("threshold", "UnsharpMaskThreshold")],
        "6074" => &[("brightness", "BrightnessContrastBrightness"), ("contrast", "BrightnessContrastContrast")],
        "6525" => &[("angle", "MotionBlurAngle"), ("distance", "MotionBlurDistance")],
        "19110" => &[("hue", "HueSaturationHue"), ("saturation", "HueSaturationSaturation"), ("lightness", "HueSaturationLightness"), ("colorize", "HueSaturationColorize")],
        "19157" => &[("amount", "AddNoiseAmount"), ("use_color", "AddNoiseColor")],
        "19159" => &[("amount", "RadialBlurAmount"), ("quality", "RadialBlurQuality")],
        "19160" => &[("amount", "ZoomBlurAmount"), ("quality", "ZoomBlurQuality")],
        _ => &[],
    };
    // Live-Effekt: ein neuer Effekt beginnt mit den Vorgaben des Effekts (Rauschen 50, CT-1a FX-4)
    // und merkt nichts; nur der Menüfilter nimmt die zuletzt bestätigten Werte
    if !live {
        f = f.with_prefs(remembered);
    }
    // Kästchen „Vorschau“ gemerkt wie im Original (nur diese drei Filter haben eine Voreinstellung)
    let preview_pref = match id {
        "6525" => Some("MotionBlurShowPreview"),
        "19159" => Some("RadialBlurShowPreview"),
        "19160" => Some("ZoomBlurShowPreview"),
        _ => None,
    };
    if let Some(k) = preview_pref {
        f = f.with_prefs(&[(super::form::PREVIEW_FIELD, k)]);
    }
    if live {
        f = f.with_base(json!({"live": true}));
    }
    Some(f)
}

fn hex_of(c: fp_color::Argb) -> String {
    crate::ui_commands::hex(c)
}

/// Filterdialog mit den gemerkten Werten aus den Voreinstellungen.
pub fn filter_form_with_prefs(st: &AppState, cmd: &str, live: bool) -> Option<FormDialog> {
    let mut f = filter_form(cmd, live)?;
    f.load_prefs(&st.session.prefs);
    Some(f)
}

/// Formular zu einem Menübefehl (falls der Befehl ohne Parameter aufgerufen wurde), mit den
/// gemerkten Werten aus den Voreinstellungen (wie im Original).
pub fn form_for(st: &AppState, cmd: &str) -> Option<FormDialog> {
    if let Some(f) = filter_form_with_prefs(st, cmd, false) {
        return Some(f);
    }
    let mut f = plain_form(st, cmd)?;
    f.load_prefs(&st.session.prefs);
    Some(f)
}

fn plain_form(st: &AppState, cmd: &str) -> Option<FormDialog> {
    let d = st.session.active();
    let f = match cmd {
        "select.feather" => FormDialog::new("5976", t("dlg-title-feather").as_str(), cmd, vec![Field::number("radius", t("dlg-lbl-radius").as_str(), 10.0, 1.0, 1000.0).unit("dlg-unit-pixels")]).with_prefs(&[("radius", "FeatherSelectionAmount")]).with_layout(super::marquee::LAYOUT),
        "select.expandMarquee" => FormDialog::new("19105", t("dlg-title-expand-marquee").as_str(), cmd, vec![Field::number("amount", t("dlg-lbl-expand-by").as_str(), 10.0, 1.0, 100.0).unit("dlg-unit-pixels").clamp_on_ok()]).with_prefs(&[("amount", "ExpandSelectionAmount")]).with_layout(super::marquee::LAYOUT),
        "select.contractMarquee" => FormDialog::new("19106", t("dlg-title-contract-marquee").as_str(), cmd, vec![Field::number("amount", t("dlg-lbl-contract-by").as_str(), 10.0, 1.0, 100.0).unit("dlg-unit-pixels").clamp_on_ok()]).with_prefs(&[("amount", "ContractSelectionAmount")]).with_layout(super::marquee::LAYOUT),
        "select.borderMarquee" => FormDialog::new("19107", t("dlg-title-border-marquee").as_str(), cmd, vec![Field::number("amount", t("dlg-lbl-border-width").as_str(), 10.0, 1.0, 100.0).unit("dlg-unit-pixels").clamp_on_ok()]).with_prefs(&[("amount", "BorderSelectionAmount")]).with_layout(super::marquee::LAYOUT),
        "select.smoothMarquee" => FormDialog::new("19108", t("dlg-title-smooth-marquee").as_str(), cmd, vec![Field::number("amount", t("dlg-lbl-selection-radius").as_str(), 10.0, 1.0, 100.0).unit("dlg-unit-pixels").clamp_on_ok()]).with_prefs(&[("amount", "SmoothSelectionAmount")]).with_layout(super::marquee::LAYOUT),
        // Bitmap-Auswahl speichern/wiederherstellen/löschen: eigener Dialog in `bitmap_selection.rs` (KV-3)
        // Pfad-Dialoge mit Anordnung des Originals (DLG-7, PM-6 D29–D32): eigene Datei
        "modify.alterPath.simplify" => super::path_dialogs::simplify(cmd),
        "modify.alterPath.insetPath" => super::path_dialogs::inset(cmd),
        "modify.alterPath.expandStroke" => super::path_dialogs::expand_stroke(cmd),
        "modify.convertPathToMarquee" => super::path_dialogs::to_marquee(cmd),
        // Vorgabe wie im Original: Größe des gewählten Texts, sonst der zuletzt bestätigte Wert;
        // Feld mit Popup-Schieber 8..96 (11 §3.5.5), ohne Einheit, Knöpfe rechts untereinander
        "text.size.other" => FormDialog::new("6073", t("dlg-title-text-size").as_str(), cmd, vec![Field::number("size", t("dlg-lbl-size").as_str(), text_size_default(st), 8.0, 96.0).popup()]).with_layout(TEXT_SIZE_LAYOUT).with_store_hook(remember_text_size),
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
        .with_layout(super::symbol_anim::CONVERT_LAYOUT)
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
            .with_layout(super::symbol_anim::ANIMATE_LAYOUT)
            .with_focus_ok()
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
                    // untereinander, Farbfeld eingerückt unter „Benutzerdefiniert:“ (Vorlage 5977)
                    Field::radio_of("_mode", "", &[t("dlg-canvas-white").as_str(), t("dlg-canvas-transparent").as_str(), t("dlg-canvas-custom").as_str()], vec![json!(0), json!(1), json!(2)], idx).vertical(),
                    // benutzerdefinierte Farbe: die der Seite, sonst die gemerkte (`CustomCanvasColor`, Vorgabe schwarz)
                    Field::color("_color", "", &hex_of(if idx == 2 { bg } else { st.session.prefs.get_color("CustomCanvasColor", fp_color::Argb(0xff00_0000)) })),
                ],
            )
            .with_width(230.0)
            .with_transform(canvas_color_params)
            .with_store_hook(store_custom_canvas_color)
        }
        "view.grid.editGrid" => {
            let g = d.and_then(|d| d.doc.pages.get(d.page)).and_then(|p| p.root.grid.clone());
            let (gw, gh, col) = g.map(|g| (g.size.x as f64, g.size.y as f64, g.color)).unwrap_or((36.0, 36.0, fp_color::Argb(0xff00_ffff)));
            // wie im Original nur Farbe und die beiden Abstände (↔/↕-Symbole statt Beschriftung);
            // „Raster anzeigen“/„Am Raster ausrichten“ stehen nur im Menü Ansicht ▸ Raster
            FormDialog::new(
                "5984",
                t("dlg-title-edit-grid").as_str(),
                cmd,
                vec![Field::color("color", t("dlg-lbl-color").as_str(), &hex_of(col)), Field::number("width", t("dlg-lbl-grid-h").as_str(), gw, 1.0, 10000.0).unit("dlg-unit-pixels").popup().scale(GRID_SPACING), Field::number("height", t("dlg-lbl-grid-v").as_str(), gh, 1.0, 10000.0).unit("dlg-unit-pixels").popup().scale(GRID_SPACING)],
            )
            .with_layout(GRID_LAYOUT)
            .with_skip_unchanged()
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

/// Leinwandfarbe: die benutzerdefinierte Farbe nur merken, wenn sie gewählt ist.
fn store_custom_canvas_color(d: &FormDialog, p: &mut fp_engine::Preferences) {
    if d.get("_mode").as_i64() == Some(2)
        && let Some(c) = d.get("_color").as_str().and_then(fp_color::Rgba8::parse_hex)
    {
        p.set("CustomCanvasColor", fp_engine::PrefValue::Str(format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)));
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

// ------------------------------------------------- eigene Anordnungen nach der Vorlage (P-DLG2)
//
// Lage und Größe in Dialogeinheiten aus `dialoge.json` (Vorlagen 5984, 6073, 19147); längere Texte
// anderer Sprachen schieben die Spalte rechts davon und verbreitern den Dialog.

fn field_at(d: &FormDialog, key: &str) -> usize {
    d.fields.iter().position(|f| f.key == key).unwrap_or(usize::MAX)
}

fn label_of(d: &FormDialog, key: &str) -> String {
    d.fields.get(field_at(d, key)).map(|f| f.label.clone()).unwrap_or_default()
}

/// OK und Abbrechen: gemeinsame Breite (mindestens 50 DLU wie in den Vorlagen).
fn ok_cancel_w(ctx: &egui::Context) -> f32 {
    let (ok, cancel) = (t("btn-ok"), t("btn-cancel"));
    super::buttons_w(ctx, &[&ok, &cancel], dlu(50.0, 0.0).x)
}

/// Farbfeld `key` (Farbknopf mit Farbwahl wie im Inspektor).
fn color_input(ui: &mut Ui, st: &AppState, d: &mut FormDialog, key: &str, r: Rect, enabled: bool) -> bool {
    let i = field_at(d, key);
    let id = d.id.clone();
    let Some(f) = d.fields.get_mut(i) else { return false };
    let cur = f.value.as_str().and_then(fp_color::Rgba8::parse_hex).map(|c| c.to_argb());
    if let Some(crate::inspector::popups::ColorChoice::Color(col)) = crate::inspector::popups::color_button(ui, (id.as_str(), i), r, st, cur, enabled, false, false) {
        let a = cur.map(|c| c.a()).unwrap_or(255) as u32;
        f.value = json!(crate::ui_commands::hex(fp_color::Argb((col.0 & 0x00ff_ffff) | a << 24)));
        return true;
    }
    false
}

/// Zahlenfeld `key` (siehe [`number_input`]).
fn number_at(ui: &mut Ui, d: &mut FormDialog, key: &str, r: Rect, enabled: bool) -> bool {
    let i = field_at(d, key);
    let id = d.id.clone();
    d.fields.get_mut(i).is_some_and(|f| number_input(ui, &id, i, f, r, enabled))
}

/// Kontrollkästchen `key`.
fn check_at(ui: &mut Ui, d: &mut FormDialog, key: &str, r: Rect, enabled: bool) -> bool {
    let i = field_at(d, key);
    let id = d.id.clone();
    let Some(f) = d.fields.get_mut(i) else { return false };
    if let Some(b) = fields::checkbox(ui, (id.as_str(), i), r, &f.label, f.value.as_bool().unwrap_or(false), enabled) {
        f.value = json!(b);
        return true;
    }
    false
}

/// Beschriftung rechtsbündig im Rechteck (wie `SS_RIGHT` der Vorlage), gesperrt grau.
fn right_label(ui: &Ui, r: Rect, text: &str, enabled: bool) {
    if text_w(ui.ctx(), text) > r.width() + 0.5 {
        super::report_overflow(ui.ctx(), format!("Beschriftung „{text}“ breiter als {:.0} px", r.width()));
    }
    let theme = Theme::current(ui.ctx());
    let col = if enabled { theme.colors.text } else { theme.colors.text_disabled };
    ui.painter().text(pos2(r.right(), r.center().y), Align2::RIGHT_CENTER, text.replace('&', ""), theme.ui_font(), col);
}

/// Doppelpfeil ↔ bzw. ↕ (Bildchen 5948/5949 der Vorlage 5984).
fn paint_double_arrow(ui: &Ui, r: Rect, horizontal: bool, col: egui::Color32) {
    let p = ui.painter();
    let s = Stroke::new(1.0, col);
    let c = r.center();
    if horizontal {
        let (a, b) = (pos2(r.left(), c.y), pos2(r.right(), c.y));
        p.line_segment([a, b], s);
        for (tip, dir) in [(a, 1.0), (b, -1.0)] {
            p.line_segment([tip, pos2(tip.x + 3.0 * dir, tip.y - 3.0)], s);
            p.line_segment([tip, pos2(tip.x + 3.0 * dir, tip.y + 3.0)], s);
        }
    } else {
        let (a, b) = (pos2(c.x, r.top()), pos2(c.x, r.bottom()));
        p.line_segment([a, b], s);
        for (tip, dir) in [(a, 1.0), (b, -1.0)] {
            p.line_segment([tip, pos2(tip.x - 3.0, tip.y + 3.0 * dir)], s);
            p.line_segment([tip, pos2(tip.x + 3.0, tip.y + 3.0 * dir)], s);
        }
    }
}

// Raster bearbeiten (5984): 151 × 89 DLU. „Farbe:“ [9,9], Farbfeld [38,8,13,11]; ↔ [24,29,10,4],
// Feld [38,25,30,13], „Pixel“ [82,27]; ↕ [29,43,5,9], Feld [38,41,30,13], „Pixel“ [82,43];
// OK [37,68,50,14], Abbrechen [92,68,50,14]. Neben den Feldern die Popup-Schieber ▾ (Rahmen
// 5769/5814, TW-10); die Einheit steht rechts davon.

/// Kennlinie der Popup-Schieber der Rasterabstände: 1 … 100 linear (Annahme FIX-G1, der Bereich
/// im Original ist noch nicht gemessen, BEFUNDE DLG-2); getippt bleibt 1 … 10000 erlaubt.
const GRID_SPACING: crate::inspector::scale::Scale = crate::inspector::scale::Scale::Piecewise(&[(0.0, 1.0), (99.0, 100.0)]);

/// Verschiebung der Feldspalte, wenn „Farbe:“ nicht in die 29 DLU vor den Feldern passt.
fn grid_dx(ctx: &egui::Context) -> f32 {
    (text_w(ctx, &t("dlg-lbl-color")) + 6.0 - dlu(29.0, 0.0).x).max(0.0).ceil()
}

fn grid_size(ctx: &egui::Context, _d: &FormDialog) -> egui::Vec2 {
    let dx = grid_dx(ctx);
    let unit = dlu(72.0, 0.0).x + dx + 15.0 + text_w(ctx, &super::form::unit_text("dlg-unit-pixels")) + dlu(9.0, 0.0).x;
    let buttons = dlu(37.0, 0.0).x + 2.0 * ok_cancel_w(ctx) + dlu(5.0 + 9.0, 0.0).x;
    vec2(dlu(151.0, 0.0).x.max(unit).max(buttons).ceil(), dlu(0.0, 89.0).y.ceil())
}

fn grid_draw(ui: &mut Ui, st: &AppState, d: &mut FormDialog, body: Rect) -> (bool, [Rect; 2]) {
    let o = body.min;
    let dx = grid_dx(ui.ctx());
    let theme = Theme::current(ui.ctx());
    let sh = |r: Rect| r.translate(vec2(dx, 0.0));
    let mut changed = false;
    let sw = sh(dlu_rect(o, [38.0, 7.0, 16.0, 12.0]));
    let lx = o.x + dlu(9.0, 0.0).x;
    super::fit_label(ui, Rect::from_min_max(pos2(lx, sw.center().y - 10.0), pos2(sw.left() - 2.0, sw.center().y + 10.0)), &t("dlg-lbl-color"), true);
    // das Farbfeld ist im Original kein Tab-Stopp (Tab: Feld › Feld › OK › Abbrechen)
    let skip = egui::Id::new((d.id.as_str(), "swatch-skip"));
    super::skip_tab_stop_before(ui, skip);
    changed |= color_input(ui, st, d, "color", sw, true);
    let swatch_id = egui::Id::new(egui::Id::new((d.id.as_str(), field_at(d, "color"))));
    if let Some(resp) = ui.ctx().read_response(swatch_id) {
        super::skip_tab_stop_after(ui, skip, &resp);
    }
    let unit = super::form::unit_text("dlg-unit-pixels");
    for (key, arrow, y, horizontal) in [("width", [24.0, 29.0, 10.0, 4.0], 25.0, true), ("height", [29.0, 43.0, 5.0, 9.0], 41.0, false)] {
        let er = sh(dlu_rect(o, [38.0, y, 30.0, 13.0]));
        // Symbol statt Beschriftung; der bisherige Text („Abstand ↔:“) bleibt Tooltip und Name
        let ar = sh(dlu_rect(o, arrow));
        paint_double_arrow(ui, ar, horizontal, theme.colors.text);
        let label = label_of(d, key);
        let hover = ui.interact(ar.expand(3.0), egui::Id::new((d.id.as_str(), key, "symbol")), egui::Sense::hover());
        hover.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, label.as_str()));
        let _ = hover.on_hover_text(label.as_str());
        changed |= number_at(ui, d, key, er, true);
        // Einheit rechts vom Popup-Schieber ▾ (der liegt direkt am Feld, 14 px breit)
        ui.painter().text(pos2(er.right() + 15.0 + dlu(3.0, 0.0).x, er.center().y), Align2::LEFT_CENTER, &unit, theme.ui_font(), theme.colors.text);
    }
    let bw = ok_cancel_w(ui.ctx());
    let top = o.y + dlu(0.0, 68.0).y;
    let h = dlu(0.0, 14.0).y;
    let cancel = Rect::from_min_size(pos2(body.right() - dlu(9.0, 0.0).x - bw, top), vec2(bw, h));
    let ok = Rect::from_min_size(pos2(cancel.left() - dlu(5.0, 0.0).x - bw, top), vec2(bw, h));
    (changed, [ok, cancel])
}

const GRID_LAYOUT: CustomLayout = CustomLayout { size: grid_size, draw: grid_draw };

// Textgröße (6073): 183 × 50 DLU. „Größe:“ rechtsbündig [3,17,55,8], Feld [62,15,30,13] mit
// Popup-Schieber [93,15,10,13] (Bereich 8..96), OK [121,5,50,14] und Abbrechen [121,23,50,14]
// rechts untereinander.

fn text_size_dx(ctx: &egui::Context) -> f32 {
    (text_w(ctx, &t("dlg-lbl-size")) - dlu(55.0, 0.0).x).max(0.0).ceil()
}

fn text_size_size(ctx: &egui::Context, _d: &FormDialog) -> egui::Vec2 {
    let bx = dlu(121.0, 0.0).x + text_size_dx(ctx) + ok_cancel_w(ctx) + dlu(12.0, 0.0).x;
    vec2(dlu(183.0, 0.0).x.max(bx).ceil(), dlu(0.0, 50.0).y.ceil())
}

fn text_size_draw(ui: &mut Ui, _st: &AppState, d: &mut FormDialog, body: Rect) -> (bool, [Rect; 2]) {
    let o = body.min;
    let er = dlu_rect(o, [62.0, 15.0, 30.0, 13.0]).translate(vec2(text_size_dx(ui.ctx()), 0.0));
    right_label(ui, Rect::from_min_max(pos2(o.x + dlu(3.0, 0.0).x, er.top()), pos2(er.left() - dlu(4.0, 0.0).x, er.bottom())), &label_of(d, "size"), true);
    let changed = number_at(ui, d, "size", er, true);
    let bw = ok_cancel_w(ui.ctx());
    let x = body.right() - dlu(12.0, 0.0).x - bw;
    let h = dlu(0.0, 14.0).y;
    (changed, [Rect::from_min_size(pos2(x, o.y + dlu(0.0, 5.0).y), vec2(bw, h)), Rect::from_min_size(pos2(x, o.y + dlu(0.0, 23.0).y), vec2(bw, h))])
}

const TEXT_SIZE_LAYOUT: CustomLayout = CustomLayout { size: text_size_size, draw: text_size_draw };

thread_local! {
    /// Textgröße: zuletzt bestätigter Wert und die Vorgabe des Textwerkzeugs danach. Ohne
    /// Textauswahl zeigt der Dialog diesen Wert, solange die Vorgabe seitdem gleich blieb.
    static LAST_TEXT_SIZE: std::cell::Cell<(Option<f64>, f64)> = const { std::cell::Cell::new((None, 0.0)) };
    /// Beim Öffnen: Vorgabe des Textwerkzeugs und ob Text gewählt war.
    static OPEN_TEXT_SIZE: std::cell::Cell<(f64, bool)> = const { std::cell::Cell::new((0.0, false)) };
}

/// Vorgabe des Dialogs „Textgröße“ wie im Original: Größe des ersten gewählten Texts (erster
/// Lauf), sonst der zuletzt bestätigte Wert, sonst die Vorgabe des Textwerkzeugs.
fn text_size_default(st: &AppState) -> f64 {
    let tool = st.session.tools.options.text_size as f64;
    let sel = crate::inspector::obj_quads::selection(st).iter().find_map(|e| match &e.kind {
        fp_model::ElementKind::Text(t) => Some(t.resolved_runs().first().and_then(|(a, _)| a.size).or(t.initial.size).unwrap_or(12.0) as f64),
        _ => None,
    });
    OPEN_TEXT_SIZE.with(|c| c.set((tool, sel.is_some())));
    let (last, tool_then) = LAST_TEXT_SIZE.with(|c| c.get());
    match (sel, last) {
        (Some(v), _) => v,
        (None, Some(v)) if tool_then == tool => v,
        _ => tool,
    }
}

/// OK in „Textgröße“: Wert merken. Ohne Textauswahl setzt der Befehl die Vorgabe des Werkzeugs
/// auf denselben Wert, mit Auswahl bleibt sie.
fn remember_text_size(d: &FormDialog, _p: &mut fp_engine::Preferences) {
    if let Some(v) = d.get("size").as_f64() {
        let (tool, had_text) = OPEN_TEXT_SIZE.with(|c| c.get());
        LAST_TEXT_SIZE.with(|c| c.set((Some(v), if had_text { tool } else { v as f32 as f64 })));
    }
}

// Selektive JPEG-Einstellungen (19147): 196 × 94 DLU. „Selektive Qualität aktivieren“ [9,8,114,10]
// mit Qualität [137,5,40,14] daneben; „Überlagerungsfarbe:“ rechtsbündig [18,26,116,8] mit Farbfeld
// [137,23,16,13]; Trennlinie [6,40,184,1]; Text-/Schaltflächenqualität [9,44] und [9,54];
// OK [85,75,50,14], Abbrechen [141,75,50,14]. Ohne Haken sind Qualität, Überlagerungsfarbe und die
// beiden Kästchen gesperrt (Original gemessen, PM-5).

fn jpeg_dx(ctx: &egui::Context, d: &FormDialog) -> f32 {
    let check = dlu(9.0, 0.0).x + 18.0 + text_w(ctx, &label_of(d, "enabled")) + 8.0 - dlu(137.0, 0.0).x;
    let overlay = text_w(ctx, &label_of(d, "_overlay")) - dlu(116.0, 0.0).x;
    check.max(overlay).max(0.0).ceil()
}

fn jpeg_size(ctx: &egui::Context, d: &FormDialog) -> egui::Vec2 {
    let dx = jpeg_dx(ctx, d);
    let checks = dlu(9.0 + 6.0, 0.0).x + 18.0 + text_w(ctx, &label_of(d, "preserveText")).max(text_w(ctx, &label_of(d, "preserveButtons"))) + 4.0;
    let buttons = dlu(85.0, 0.0).x + 2.0 * ok_cancel_w(ctx) + dlu(6.0 + 5.0, 0.0).x;
    vec2((dlu(196.0, 0.0).x + dx).max(checks).max(buttons).ceil(), dlu(0.0, 94.0).y.ceil())
}

fn jpeg_draw(ui: &mut Ui, st: &AppState, d: &mut FormDialog, body: Rect) -> (bool, [Rect; 2]) {
    let o = body.min;
    let dx = jpeg_dx(ui.ctx(), d);
    let theme = Theme::current(ui.ctx());
    let sh = |r: Rect| r.translate(vec2(dx, 0.0));
    let mut changed = false;
    let er = sh(dlu_rect(o, [137.0, 5.0, 40.0, 14.0]));
    let cr = dlu_rect(o, [9.0, 8.0, 114.0, 10.0]);
    changed |= check_at(ui, d, "enabled", Rect::from_min_max(pos2(cr.left(), er.center().y - 8.0), pos2(er.left() - 8.0, er.center().y + 8.0)), true);
    let on = d.get("enabled").as_bool().unwrap_or(false);
    changed |= number_at(ui, d, "quality", er, on);
    let sw = sh(dlu_rect(o, [137.0, 23.0, 16.0, 13.0]));
    right_label(ui, Rect::from_min_max(pos2(o.x + dlu(18.0, 0.0).x, sw.top()), pos2(sw.left() - dlu(3.0, 0.0).x, sw.bottom())), &label_of(d, "_overlay"), on);
    changed |= color_input(ui, st, d, "_overlay", sw, on);
    let y = (o.y + dlu(0.0, 40.0).y).round() + 0.5;
    ui.painter().hline((o.x + dlu(6.0, 0.0).x)..=(body.right() - dlu(6.0, 0.0).x), y, Stroke::new(1.0, theme.colors.line_mid));
    let w = body.right() - dlu(6.0, 0.0).x;
    changed |= check_at(ui, d, "preserveText", dlu_rect(o, [9.0, 44.0, 181.0, 10.0]).with_max_x(w), on);
    changed |= check_at(ui, d, "preserveButtons", dlu_rect(o, [9.0, 54.0, 180.0, 10.0]).with_max_x(w), on);
    let bw = ok_cancel_w(ui.ctx());
    let top = o.y + dlu(0.0, 75.0).y;
    let h = dlu(0.0, 14.0).y;
    let cancel = Rect::from_min_size(pos2(body.right() - dlu(5.0, 0.0).x - bw, top), vec2(bw, h));
    let ok = Rect::from_min_size(pos2(cancel.left() - dlu(6.0, 0.0).x - bw, top), vec2(bw, h));
    (changed, [ok, cancel])
}

/// Anordnung von „Selektive JPEG-Einstellungen“ (19147).
pub const SELECTIVE_JPEG_LAYOUT: CustomLayout = CustomLayout { size: jpeg_size, draw: jpeg_draw };

/// Überlagerungsfarbe der JPEG-Maske (`SelectiveJPEGColor`, Vorgabe #ff0080) als Farbfeld `_overlay`.
pub fn selective_jpeg_overlay_field(st: &AppState) -> Field {
    Field::color("_overlay", t("dlg-md-overlay-color").as_str(), &st.session.prefs.get_str("SelectiveJPEGColor", "#ff0080"))
}

/// OK in „Selektive JPEG-Einstellungen“: Überlagerungsfarbe merken (`#rrggbb` wie die Vorgabe).
pub fn store_selective_jpeg_overlay(d: &FormDialog, p: &mut fp_engine::Preferences) {
    if let Some(c) = d.get("_overlay").as_str().and_then(fp_color::Rgba8::parse_hex) {
        p.set("SelectiveJPEGColor", fp_engine::PrefValue::Str(format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)));
    }
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
        // Live-Effekt: Vorgabe 50 auch wenn der Menüfilter zuletzt 20 bestätigt hat (CT-1a FX-4)
        let mut p = fp_engine::Preferences::new();
        p.set("AddNoiseAmount", fp_engine::PrefValue::Real(20.0));
        let mut live = filter_form("filter.noise.addNoise", true).expect("Rauschen");
        live.load_prefs(&p);
        assert_eq!(live.get("amount").as_f64(), Some(50.0), "Live-Effekt mit Vorgabe");
        let mut menu = filter_form("filter.noise.addNoise", false).expect("Rauschen");
        menu.load_prefs(&p);
        assert_eq!(menu.get("amount").as_f64(), Some(20.0), "Menüfilter mit gemerktem Wert");
        assert!(filter_form("filter.unknown", false).is_none());
    }
}
