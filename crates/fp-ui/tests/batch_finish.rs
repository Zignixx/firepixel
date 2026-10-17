//! BATCH-11b: eigene Felder der Endkarte „Speichern als“ je Format (GIF-Optimierer mit Voreinstellung,
//! Qualität, Zielgröße, Teilbilder …), erreichte Größe bzw. ⚠ „Zielgröße nicht erreichbar“, Aufräumen
//! beim Formatwechsel, Hinweise (Nachzeichnen + Pixelformat, JPEG), ffmpeg-Hinweis in Videozeile und
//! Karte „Video lesen“, Filter nach Größe/Pixeln und Platzhaltern, Knopf „Bericht ansehen“.
//! Logikfälle nach AGENTEN_REGELN §4b mit realistischen Eingaben (Klicks, Tippen Zeichen für Zeichen).
//! Bilder (`--ignored batch11b_bilder`): `docs\screenshots\batch11b_*.png`.

use egui::epaint::Shape;
use egui::{vec2, Event, Key, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::kittest::{By, Queryable};
use egui_kittest::Harness;
use fp_i18n::t;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::batch::{self, params, CardId, Mode};
use fp_ui::theme::ThemeKind;
use fp_ui::FirepixelApp;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

// ------------------------------------------------------------------ Hilfen

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join("firepixel-batch11b").join(std::process::id().to_string()).join(name);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("Ordner");
    d
}

/// PNG-Datei: Verlauf mit Scheibe (`noise` = Rauschen, schwer zu komprimieren).
fn png_file(d: &Path, name: &str, seed: u32, w: u32, h: u32, noise: bool) -> PathBuf {
    let p = d.join(name);
    let mut px = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let fx = x as f32 / w as f32;
            let fy = y as f32 / h as f32;
            let disc = (x as i32 - w as i32 / 2).pow(2) + (y as i32 - h as i32 / 2).pow(2) < (h as i32 / 4).pow(2);
            let c = if noise {
                let v = (x.wrapping_mul(2654435761) ^ y.wrapping_mul(40503) ^ seed).wrapping_mul(2246822519);
                [(v >> 8) as u8, (v >> 16) as u8, (v >> 24) as u8, 255]
            } else if disc {
                [255, 250, 200 - (seed * 40 % 150) as u8, 255]
            } else {
                [(240.0 * fx) as u8, (90.0 + 120.0 * fy) as u8, (200 - seed * 30 % 150) as u8, 255]
            };
            px.extend_from_slice(&c);
        }
    }
    let f = std::fs::File::create(&p).expect("Datei");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().and_then(|mut wr| wr.write_image_data(&px)).expect("PNG");
    p
}

/// Animiertes GIF aus `n` Bildern Rauschen (für eine Zielgröße, die nicht erreichbar ist).
fn noisy_gif(d: &Path, name: &str, w: u32, h: u32, n: u32) -> PathBuf {
    use fp_batch::item::{Animation, Frame, Item, Meta};
    let frames = (0..n)
        .map(|k| {
            let mut v = Vec::with_capacity((w * h * 4) as usize);
            for y in 0..h {
                for x in 0..w {
                    let r = (x.wrapping_mul(2654435761) ^ y.wrapping_mul(40503) ^ k.wrapping_mul(97)).wrapping_mul(2246822519);
                    v.extend_from_slice(&[(r >> 8) as u8, (r >> 16) as u8, (r >> 24) as u8, 255]);
                }
            }
            let image = fp_raster::RgbaImage::from_rgba_bytes(fp_raster::IRect::new(0, 0, w as i32, h as i32), &v).expect("Bild");
            Frame { image, delay_ms: 80 }
        })
        .collect();
    let item = Item::Animation(Animation { width: w, height: h, frames, loop_count: 0, meta: Meta { source_format: "gif".into(), ..Meta::default() } });
    let (bytes, _) = fp_batch::output::encode(&item, fp_batch::OutputFormat::Gif, &serde_json::Map::new()).expect("GIF");
    let p = d.join(name);
    std::fs::write(&p, bytes).expect("GIF schreiben");
    p
}

fn state(kind: ThemeKind) -> AppState {
    fp_engine::shell::disable();
    let session = fp_ui::ui_commands::new_session(fp_engine::Preferences::new());
    let mut st = AppState::new(session, UiSettings::default(), None);
    st.sync_render = true;
    st.os_clipboard = false;
    st.set_theme(kind);
    st
}

fn app(st: AppState) -> FirepixelApp {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, fp_engine::Session::default(), UiSettings::default(), false);
    app.state = st;
    app
}

type H = Harness<'static, FirepixelApp>;

fn harness_ex(st: AppState, w: f32, h: f32, ppp: f32, gpu: bool) -> H {
    let b = Harness::builder().renderer(crate::gpu_common::renderer()).with_size(vec2(w / ppp, h / ppp)).with_pixels_per_point(ppp).with_max_steps(30).with_step_dt(1.0 / 60.0);
    let b = if gpu { b.renderer(crate::gpu_common::renderer()) } else { b };
    let mut hn = b.build_ui_state(|ui, a: &mut FirepixelApp| a.root_ui(ui), app(st));
    hn.run_steps(4);
    hn
}

fn st(h: &mut H) -> &mut AppState {
    &mut h.state_mut().state
}

fn studio(h: &mut H) -> &mut batch::Studio {
    batch::studio_mut(st(h)).expect("Studio offen")
}

fn settle(h: &mut H) {
    for _ in 0..900 {
        h.run_steps(1);
        let busy = batch::studio(&h.state().state).is_some_and(|s| s.preview.busy() || s.running() || s.settings.as_ref().is_some_and(|x| x.checking()));
        if !busy && !batch::thumbnails_pending(&h.ctx) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    h.run_steps(3);
}

#[derive(Debug, Clone)]
struct T {
    text: String,
    rect: Rect,
    clip: Rect,
}

fn texts(h: &H) -> Vec<T> {
    fn walk(s: &Shape, clip: Rect, out: &mut Vec<T>) {
        match s {
            Shape::Text(t) => {
                let text = t.galley.text().to_string();
                if !text.trim().is_empty() {
                    out.push(T { text, rect: t.visual_bounding_rect(), clip });
                }
            }
            Shape::Vec(v) => v.iter().for_each(|x| walk(x, clip, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    for cs in &h.output().shapes {
        walk(&cs.shape, cs.clip_rect, &mut out);
    }
    out
}

fn has_text(h: &H, needle: &str) -> bool {
    texts(h).iter().any(|t| t.text.contains(needle))
}

fn click(h: &mut H, p: Pos2) {
    h.event(Event::PointerMoved(p));
    h.run_steps(1);
    for pressed in [true, false] {
        h.event(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
    }
    h.run_steps(2);
}

fn key(h: &mut H, k: Key, m: Modifiers) {
    h.event(Event::Key { key: k, physical_key: None, pressed: true, repeat: false, modifiers: m });
    h.event(Event::Key { key: k, physical_key: None, pressed: false, repeat: false, modifiers: m });
    h.run_steps(2);
}

fn type_text(h: &mut H, s: &str) {
    for c in s.chars() {
        h.event(Event::Text(c.to_string()));
        h.run_steps(1);
    }
    h.run_steps(1);
}

fn lang(code: &str) {
    assert!(fp_i18n::set_language(code), "Sprache {code}");
}

fn node(h: &H, label: &str, role: egui::accesskit::Role) -> Option<Rect> {
    h.query_all(By::new().label(label).role(role)).next().map(|n| n.rect())
}

fn assert_clean(h: &H, label: &str) {
    let screen = h.ctx.content_rect();
    for x in texts(h) {
        for raw in ["batch-form-", "batch-ui-", "batch-value-", "batch-param-", "batch-html-", "batch-warn-"] {
            assert!(!x.text.contains(raw), "{label}: Rohschlüssel {:?}", x.text);
        }
        let r = x.rect;
        let clip = x.clip.intersect(screen);
        if r.width() <= 0.0 || !clip.intersects(r) {
            continue;
        }
        let fully_v = r.top() >= clip.top() - 1.0 && r.bottom() <= clip.bottom() + 1.0;
        if fully_v {
            assert!(r.left() >= clip.left() - 1.5 && r.right() <= clip.right() + 1.5, "{label}: Text abgeschnitten {:?} {:?} Clip {:?}", x.text, r, x.clip);
        }
    }
}

/// Studio mit Dateien, Format `fmt` und offener Endkarte „Speichern als“.
fn studio_output(paths: &[PathBuf], fmt: &str, options: Value, mode: Mode, kind: ThemeKind, w: f32, hgt: f32) -> H {
    let mut st = state(kind);
    st.run("file.batchProcess", json!({}));
    let mut h = harness_ex(st, w, hgt, 1.0, false);
    {
        let s = studio(&mut h);
        s.mode = mode;
        s.add_paths(paths, false);
        s.edit("fmt", |r| {
            r.output.format = fmt.into();
            if let Some(o) = options.as_object() {
                r.output.options = o.clone();
            }
            true
        });
        s.end_gesture();
        s.open_card = Some(CardId::Output);
    }
    settle(&mut h);
    h
}

fn opt(h: &mut H, name: &str) -> Option<Value> {
    studio(h).recipe.output.options.get(name).cloned()
}

// ------------------------------------------------------------------ GIF-Endkarte

#[test]
fn gif_endkarte_hat_eigene_felder_statt_json() {
    let d = dir("gif-felder");
    let p = png_file(&d, "a.png", 1, 160, 100, false);
    for mode in [Mode::Beginner, Mode::Pro] {
        let h = studio_output(std::slice::from_ref(&p), "gif", json!({}), mode, ThemeKind::Normal, 1440.0, 900.0);
        assert!(has_text(&h, &t("batch-ui-gif-section")), "Abschnitt GIF-Optimierer");
        let q = params::param_label("output.gif", "quality");
        let kb = params::param_label("output.gif", "maxKB");
        assert!(node(&h, &q, egui::accesskit::Role::Slider).is_some(), "Schieber Qualität");
        assert!(node(&h, &kb, egui::accesskit::Role::Slider).is_some(), "Schieber Zielgröße");
        // Segment, wenn es neben die Beschriftung passt, sonst Liste
        let seg = node(&h, &params::value_label("output.gif", "preset", "balanced"), egui::accesskit::Role::RadioButton).is_some();
        let list = node(&h, &params::param_label("output.gif", "preset"), egui::accesskit::Role::ComboBox).is_some();
        assert!(seg || list, "Voreinstellung als Segment oder Liste");
        if mode == Mode::Beginner {
            assert!(seg, "Anfänger: Segment");
        }
        // nirgends JSON (kein Code-Feld mit geschweiften Klammern)
        assert!(texts(&h).iter().all(|x| !x.text.trim_start().starts_with('{')), "kein JSON");
        let split = params::param_label("output.gif", "splitFrames");
        if mode == Mode::Pro {
            let head: String = split.chars().take(10).collect();
            assert!(texts(&h).iter().any(|x| x.text.starts_with(&head)), "Profi: Teilbilder");
            assert!(node(&h, &t("batch-form-auto"), egui::accesskit::Role::RadioButton).is_some(), "Automatisch | An | Aus");
            assert!(has_text(&h, &params::param_label("output.gif", "lossy")));
        } else {
            assert!(!has_text(&h, &split), "Anfänger: nur Voreinstellung, Qualität, Zielgröße");
        }
        // keine Felder anderer Formate
        assert!(!has_text(&h, &params::param_label("output", "progressive")));
        assert_clean(&h, "gif");
    }
}

#[test]
fn voreinstellung_setzt_qualitaet_in_einem_schritt() {
    let d = dir("gif-preset");
    let p = png_file(&d, "a.png", 1, 160, 100, false);
    let mut h = studio_output(&[p], "gif", json!({}), Mode::Beginner, ThemeKind::Dark, 1440.0, 900.0);
    let n0 = studio(&mut h).undo_len();
    let smallest = params::value_label("output.gif", "preset", "smallest");
    let best = params::value_label("output.gif", "preset", "best");
    let r = node(&h, &smallest, egui::accesskit::Role::RadioButton).expect("Am kleinsten");
    click(&mut h, r.center());
    studio(&mut h).end_gesture();
    assert_eq!(opt(&mut h, "preset"), Some(json!("smallest")));
    assert_eq!(opt(&mut h, "quality"), Some(json!(60)), "Qualität folgt der Voreinstellung");
    assert_eq!(studio(&mut h).undo_len(), n0 + 1, "ein Rückgängig-Schritt");
    // 50× hin und her: keine doppelten Schlüssel, Endwert wie einmal
    for k in 0..50 {
        let l = if k % 2 == 0 { &best } else { &smallest };
        let r = node(&h, l, egui::accesskit::Role::RadioButton).expect("Segment");
        click(&mut h, r.center());
        studio(&mut h).end_gesture();
    }
    assert_eq!(opt(&mut h, "preset"), Some(json!("smallest")));
    assert_eq!(opt(&mut h, "quality"), Some(json!(60)));
    let keys: Vec<String> = studio(&mut h).recipe.output.options.keys().cloned().collect();
    assert_eq!(keys.len(), 2, "{keys:?}");
    // Rückgängig stellt Voreinstellung und Qualität gemeinsam zurück
    for _ in 0..51 {
        studio(&mut h).undo();
    }
    assert_eq!((opt(&mut h, "preset"), opt(&mut h, "quality")), (None, None));
    studio(&mut h).redo();
    assert_eq!((opt(&mut h, "preset"), opt(&mut h, "quality")), (Some(json!("smallest")), Some(json!(60))));
}

#[test]
fn zielgroesse_tippen_und_erreichte_groesse() {
    let d = dir("gif-ziel");
    let p = noisy_gif(&d, "rauschen.gif", 220, 220, 8);
    let mut h = studio_output(&[p], "gif", json!({}), Mode::Beginner, ThemeKind::Normal, 1440.0, 900.0);
    let kb = params::param_label("output.gif", "maxKB");
    let field = node(&h, &kb, egui::accesskit::Role::TextInput).expect("Feld Zielgröße");
    let n0 = studio(&mut h).undo_len();
    click(&mut h, field.center());
    key(&mut h, Key::End, Modifiers::NONE);
    for _ in 0..6 {
        key(&mut h, Key::Backspace, Modifiers::NONE);
    }
    type_text(&mut h, "1");
    key(&mut h, Key::Enter, Modifiers::NONE);
    studio(&mut h).end_gesture();
    assert_eq!(opt(&mut h, "maxKB"), Some(json!(1)));
    assert_eq!(studio(&mut h).undo_len(), n0 + 1);
    settle(&mut h);
    // 1 KB ist für 8 Bilder 220 × 220 Rauschen nicht erreichbar → Warnung mit kleinster Größe
    let missed = t("batch-ui-gif-missed");
    let head = missed.split(':').next().unwrap_or(&missed).trim().to_string();
    assert!(has_text(&h, &head), "Hinweis „{head}“ fehlt: {:?}", texts(&h).iter().map(|x| x.text.clone()).filter(|x| x.len() < 120).collect::<Vec<_>>());
    // großzügige Zielgröße → erreicht
    studio(&mut h).edit("kb", |r| {
        r.output.options.insert("maxKB".into(), json!(50000));
        true
    });
    studio(&mut h).end_gesture();
    settle(&mut h);
    let reached = t("batch-ui-gif-reached");
    let head = reached.split(':').next().unwrap_or(&reached).trim().to_string();
    assert!(has_text(&h, &head), "Hinweis „{head}“ fehlt");
    assert_clean(&h, "zielgroesse");
}

#[test]
fn formatwechsel_raeumt_unpassende_optionen_auf() {
    let d = dir("formatwechsel");
    let p = png_file(&d, "a.png", 1, 160, 100, false);
    let mut h = studio_output(&[p], "tiff", json!({"compression": "lzw"}), Mode::Pro, ThemeKind::Oled, 1440.0, 900.0);
    let fmt_l = params::param_label("output", "format");
    let combo = node(&h, &fmt_l, egui::accesskit::Role::ComboBox).expect("Format-Liste");
    click(&mut h, combo.center());
    let png_t = h.query_all(By::new().label("PNG")).map(|n| n.rect()).find(|r| r.top() > combo.top()).expect("Eintrag PNG");
    click(&mut h, png_t.center());
    studio(&mut h).end_gesture();
    assert_eq!(studio(&mut h).recipe.output.format, "png");
    assert_eq!(opt(&mut h, "compression"), None, "LZW passt nicht zu PNG");
    assert!(has_text(&h, &params::param_label("output", "bitDepth")));
    // Rückgängig: TIFF mit LZW wieder da
    studio(&mut h).undo();
    assert_eq!(studio(&mut h).recipe.output.format, "tiff");
    assert_eq!(opt(&mut h, "compression"), Some(json!("lzw")));
    // gültige Werte bleiben (Qualität 70 von JPEG nach WebP)
    let mut v = json!({"format": "webp", "quality": 70, "compression": "max"});
    batch::recipe::clean_output_options(&mut v);
    assert_eq!(v, json!({"format": "webp", "quality": 70, "compression": "max"}), "ohne passendes Feld bleibt ein Wert erhalten");
    let mut v = json!({"format": "png", "compression": "packbits", "bitDepth": "8", "palette": 300});
    batch::recipe::clean_output_options(&mut v);
    assert_eq!(v, json!({"format": "png", "bitDepth": "8"}));
}

#[test]
fn zusaetzlich_fwpng_schalter_und_hinweise() {
    let d = dir("hinweise");
    let p = png_file(&d, "logo.png", 1, 160, 100, false);
    let mut h = studio_output(std::slice::from_ref(&p), "svg", json!({}), Mode::Pro, ThemeKind::Normal, 1440.0, 900.0);
    let also = params::param_label("output", "alsoFwpng");
    let sw = node(&h, &also, egui::accesskit::Role::CheckBox).expect("Schalter Zusätzlich als FW-PNG");
    click(&mut h, sw.center());
    studio(&mut h).end_gesture();
    assert_eq!(opt(&mut h, "alsoFwpng"), Some(json!(true)));
    // Nachzeichnen + Pixelformat → Hinweis; mit SVG nicht
    {
        let s = studio(&mut h);
        batch::recipe::add_step(s, "vector.trace");
        s.open_card = Some(CardId::Output);
    }
    h.run_steps(3);
    assert!(!has_text(&h, &t("batch-ui-trace-pixel-hint")));
    studio(&mut h).edit("f", |r| {
        r.output.format = "png".into();
        true
    });
    studio(&mut h).end_gesture();
    h.run_steps(3);
    assert!(has_text(&h, &t("batch-ui-trace-pixel-hint")), "Hinweis Nachzeichnen");
    studio(&mut h).edit("f", |r| {
        r.output.format = "jpeg".into();
        true
    });
    studio(&mut h).end_gesture();
    h.run_steps(3);
    assert!(has_text(&h, &t("batch-ui-jpeg-white-hint")), "Hinweis JPEG");
    assert_clean(&h, "hinweise");
}

// ------------------------------------------------------------------ ffmpeg-Hinweis

#[test]
fn ffmpeg_fehlt_hinweis_in_zeile_und_karte_oeffnet_einstellungen() {
    let d = dir("ffmpeg");
    let p = png_file(&d, "a.png", 1, 160, 100, false);
    let v = d.join("clip.mp4");
    std::fs::write(&v, b"kein echtes video").expect("mp4");
    let mut st = state(ThemeKind::Normal);
    st.run("file.batchProcess", json!({}));
    let mut h = harness_ex(st, 1440.0, 900.0, 1.0, false);
    {
        let s = studio(&mut h);
        s.add_paths(&[p, v], false);
        batch::recipe::add_step(s, "video.import");
    }
    settle(&mut h);
    let now = h.ctx.input(|i| i.time);
    studio(&mut h).set_ffmpeg_known(false, now);
    h.run_steps(2);
    // Videozeile: Hinweis statt Maße, als Verweis bedienbar
    let row_hint = t("batch-ui-ffmpeg-missing-row");
    assert!(texts(&h).iter().any(|x| x.text.starts_with("⚠") && row_hint.starts_with(x.text.trim_start_matches("⚠ ").trim_end_matches(['…', '.']))), "Hinweis in der Zeile");
    let choose = t("batch-ui-ffmpeg-choose");
    let link = node(&h, &choose, egui::accesskit::Role::Link).expect("Verweis in der Zeile");
    // Karte „Video lesen“: Meldung + Knopf
    assert!(has_text(&h, t("batch-ui-ffmpeg-missing-card").split(':').next().unwrap_or_default()), "Meldung in der Karte");
    let btn = node(&h, &choose, egui::accesskit::Role::Button).expect("Knopf in der Karte");
    assert_clean(&h, "ffmpeg");
    // Klick auf den Verweis öffnet die Einstellungen (Abschnitt ffmpeg), die markierte Datei bleibt
    let sel = studio(&mut h).selected;
    click(&mut h, link.center());
    assert!(studio(&mut h).settings.is_some(), "Einstellungen offen");
    assert_eq!(studio(&mut h).selected, sel, "Zeile wird dabei nicht ausgewählt");
    settle(&mut h);
    key(&mut h, Key::Escape, Modifiers::NONE);
    assert!(studio(&mut h).settings.is_none());
    // Knopf in der Karte ebenso
    let now = h.ctx.input(|i| i.time);
    studio(&mut h).set_ffmpeg_known(false, now);
    h.run_steps(2);
    let btn2 = node(&h, &choose, egui::accesskit::Role::Button).unwrap_or(btn);
    click(&mut h, btn2.center());
    assert!(studio(&mut h).settings.is_some());
    settle(&mut h);
    key(&mut h, Key::Escape, Modifiers::NONE);
    // mit ffmpeg: kein Hinweis
    let now = h.ctx.input(|i| i.time);
    studio(&mut h).set_ffmpeg_known(true, now);
    h.run_steps(2);
    assert!(node(&h, &choose, egui::accesskit::Role::Link).is_none());
    assert!(node(&h, &choose, egui::accesskit::Role::Button).is_none());
}

// ------------------------------------------------------------------ Filter

#[test]
fn filter_nach_groesse_pixeln_und_platzhaltern() {
    let d = dir("filter");
    let small = png_file(&d, "klein_01.png", 1, 64, 48, false);
    let big = png_file(&d, "gross_01.png", 2, 900, 600, true);
    let mid = png_file(&d, "mittel.jpg.png", 3, 300, 200, false);
    let mut st = state(ThemeKind::Normal);
    st.run("file.batchProcess", json!({}));
    let mut h = harness_ex(st, 1440.0, 900.0, 1.0, false);
    studio(&mut h).add_paths(&[small, big.clone(), mid], false);
    studio(&mut h).mode = Mode::Pro;
    settle(&mut h);
    let names = |h: &mut H| -> Vec<String> {
        let s = studio(h);
        s.visible().iter().map(|&i| s.files[i].info.file_name()).collect()
    };
    assert_eq!(names(&mut h).len(), 3);
    // Platzhalter im Suchtext
    studio(&mut h).filter.text = "*_01.png".into();
    assert_eq!(names(&mut h), ["klein_01.png", "gross_01.png"]);
    studio(&mut h).filter.text = "k?ein*".into();
    assert_eq!(names(&mut h), ["klein_01.png"]);
    studio(&mut h).filter.text.clear();
    // Profi: Felder in der Typ-Liste, Zeichen für Zeichen getippt
    let kind = node(&h, &t("batch-ui-filter-all"), egui::accesskit::Role::ComboBox)
        .or_else(|| h.query_all(By::new().role(egui::accesskit::Role::ComboBox)).map(|n| n.rect()).next())
        .expect("Typ-Liste");
    click(&mut h, kind.center());
    let from = t("batch-ui-filter-min");
    let px_label = format!("{} {from}", t("batch-ui-filter-pixels"));
    let f = node(&h, &px_label, egui::accesskit::Role::TextInput).expect("Feld längste Seite ab");
    click(&mut h, f.center());
    type_text(&mut h, "250");
    assert_eq!(studio(&mut h).filter.min_px, Some(250));
    assert_eq!(names(&mut h), ["gross_01.png", "mittel.jpg.png"]);
    let mb_label = format!("{} {}", t("batch-ui-filter-size"), t("batch-ui-filter-max"));
    let f2 = node(&h, &mb_label, egui::accesskit::Role::TextInput).expect("Feld Größe bis");
    click(&mut h, f2.center());
    type_text(&mut h, "0,1");
    assert_eq!(studio(&mut h).filter.max_mb, Some(0.1));
    assert_eq!(names(&mut h), ["mittel.jpg.png"], "das große Rauschbild ist > 0,1 MB");
    // Zähler „1 von 3 Dateien“, Anfänger ignoriert die Profi-Filter (Werte bleiben)
    assert!(has_text(&h, &fp_i18n::tr!("batch-ui-files-count-filtered", shown = 1, total = 3)));
    key(&mut h, Key::Escape, Modifiers::NONE);
    studio(&mut h).mode = Mode::Beginner;
    assert_eq!(names(&mut h).len(), 3);
    assert_eq!(studio(&mut h).filter.min_px, Some(250));
    // gefilterte Dateien werden nicht verarbeitet
    studio(&mut h).mode = Mode::Pro;
    let items = studio(&mut h).to_process();
    assert_eq!(items.len(), 1);
    assert!(fp_ui::batch::files::glob_match("*.PNG", "a.png") && !fp_ui::batch::files::glob_match("*.png", "a.png.bak"));
    let _ = big;
}

// ------------------------------------------------------------------ Bilder

fn save_png(path: &Path, img: &image::RgbaImage) {
    let f = std::fs::File::create(path).expect("Datei");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), img.width(), img.height());
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.set_compression(png::Compression::High);
    enc.write_header().and_then(|mut wr| wr.write_image_data(img.as_raw())).expect("PNG");
}

#[test]
#[ignore = "Bilder für docs/screenshots (wgpu)"]
fn batch11b_bilder() {
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("docs").join("screenshots");
    let _ = std::fs::create_dir_all(&out);
    let d = dir("bilder");
    let names = ["urlaub_strand.png", "urlaub_berge.png", "logo_entwurf.png", "team_foto.png", "banner_web.png"];
    let mut paths: Vec<PathBuf> = names.iter().enumerate().map(|(i, n)| png_file(&d, n, i as u32, 640 + 80 * i as u32, 420 + 40 * i as u32, false)).collect();
    let clip = d.join("intro_clip.mp4");
    std::fs::write(&clip, b"kein echtes video").expect("mp4");
    paths.push(clip);
    for code in ["de", "en"] {
        lang(code);
        for (kind, name) in [(ThemeKind::Normal, "normal"), (ThemeKind::Dark, "dunkel"), (ThemeKind::Oled, "oled")] {
            for view in ["gif", "video", "done"] {
                let mut st = state(kind);
                st.run("file.batchProcess", json!({}));
                let mut h = harness_ex(st, 1440.0, 900.0, 1.0, true);
                {
                    let s = studio(&mut h);
                    s.add_paths(&paths, false);
                    s.selected = Some(1);
                    if view == "gif" {
                        s.mode = Mode::Pro;
                        batch::recipe::add_step(s, "resize");
                        s.edit("o", |r| {
                            r.output.format = "gif".into();
                            r.output.options.insert("preset".into(), json!("balanced"));
                            r.output.options.insert("quality".into(), json!(80));
                            r.output.options.insert("maxKB".into(), json!(400));
                            true
                        });
                        s.end_gesture();
                        s.open_card = Some(CardId::Output);
                    } else if view == "video" {
                        batch::recipe::add_step(s, "video.import");
                    } else {
                        // Lauf ohne das Video, danach die Ende-Karte mit „Bericht ansehen“
                        let last = s.files.len() - 1;
                        s.files[last].enabled = false;
                        batch::recipe::add_step(s, "resize");
                        s.edit("o", |r| {
                            r.output.format = "webp".into();
                            r.output.target = fp_batch::recipe::TargetSpec { kind: fp_batch::recipe::TargetKind::Folder, path: Some(d.join(format!("out-{code}-{name}"))), ..Default::default() };
                            true
                        });
                        s.end_gesture();
                    }
                }
                settle(&mut h);
                if view == "done" {
                    key(&mut h, Key::Enter, Modifiers::COMMAND);
                    settle(&mut h);
                    assert!(studio(&mut h).report.as_ref().is_some_and(|r| r.report_html.is_some()), "HTML-Bericht geschrieben");
                }
                if view == "video" {
                    let now = h.ctx.input(|i| i.time);
                    studio(&mut h).set_ffmpeg_known(false, now);
                    h.run_steps(3);
                }
                assert_clean(&h, &format!("{view} {name} {code}"));
                for x in texts(&h) {
                    assert!(fp_neutral::find_in_bytes(x.text.as_bytes()).is_empty(), "gesperrter Name im Bild: {}", x.text);
                }
                let Ok(img) = h.render() else {
                    eprintln!("kein Grafikadapter");
                    return;
                };
                let p = out.join(format!("batch11b_{view}_{name}_{code}.png"));
                save_png(&p, &img);
                let bytes = std::fs::read(&p).expect("PNG lesen");
                assert!(fp_neutral::find_in_bytes(&bytes).is_empty(), "gesperrter Name in {}", p.display());
            }
        }
    }
    lang("de");
}
