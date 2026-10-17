//! BATCH-11a: Formular der Schrittkarten aus dem Parameter-Schema (Schieber + Feld + Einheit,
//! Segment/Liste, Chips, Schalter, Farbwähler des Originals, Datei, „Automatisch“, ↺), Felder je
//! Modus des Schritts, ein Rückgängig-Schritt je Geste, entprellte Live-Vorschau, Batch-Einstellungen
//! (ffmpeg) und Rückfrage beim Schließen während eines Laufs. Logikfälle nach AGENTEN_REGELN §4b,
//! realistische Eingaben (Ziehen über viele Frames, Tippen Zeichen für Zeichen).
//! Bilder (`--ignored batch11a_bilder`): `docs\screenshots\batch11a_*.png`.

use egui::epaint::Shape;
use egui::{vec2, Event, Key, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::kittest::{By, Queryable};
use egui_kittest::Harness;
use fp_i18n::t;
use fp_ui::app_state::{Action, AppState, UiSettings};
use fp_ui::batch::{self, params, CardId, Mode};
use fp_ui::theme::ThemeKind;
use fp_ui::FirepixelApp;
use serde_json::json;
use std::path::{Path, PathBuf};

// ------------------------------------------------------------------ Hilfen

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join("firepixel-batch11a").join(std::process::id().to_string()).join(name);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("Ordner");
    d
}

/// Bunte PNG-Datei.
fn png_file(d: &Path, name: &str, seed: u32, w: u32, h: u32) -> PathBuf {
    let p = d.join(name);
    let mut px = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let fx = x as f32 / w as f32;
            let fy = y as f32 / h as f32;
            let disc = (x as i32 - w as i32 / 2).pow(2) + (y as i32 - h as i32 / 2).pow(2) < (h as i32 / 4).pow(2);
            let c = if disc { [255, 250, 200 - (seed * 40 % 150) as u8, 255] } else { [(240.0 * fx) as u8, (90.0 + 120.0 * fy) as u8, (200 - seed * 30 % 150) as u8, 255] };
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
    for _ in 0..600 {
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

fn find(h: &H, text: &str) -> Option<T> {
    texts(h).into_iter().find(|t| t.text == text)
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

/// Tippt Zeichen für Zeichen (jede Taste ein Ereignis und ein Bild).
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

/// Rechteck des Bedienelements mit Namen `label` und Rolle `role`.
fn node(h: &H, label: &str, role: egui::accesskit::Role) -> Option<Rect> {
    h.query_all(By::new().label(label).role(role)).next().map(|n| n.rect())
}

/// Prüft: keine Rohschlüssel, nichts waagerecht abgeschnitten.
fn assert_clean(h: &H, label: &str) {
    let screen = h.ctx.content_rect();
    for x in texts(h) {
        assert!(!x.text.contains("batch-form-") && !x.text.contains("batch-settings-") && !x.text.contains("batch-value-") && !x.text.contains("batch-param-") && !x.text.contains("batch-quit-"), "{label}: Rohschlüssel {:?}", x.text);
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

/// Studio mit `n` Dateien und einem offenen Schritt `id`.
fn studio_with_step(n: usize, id: &str, mode: Mode, kind: ThemeKind, w: f32, hgt: f32) -> (H, PathBuf) {
    let d = dir(&format!("{}-{n}-{}-{w}", id.replace('.', "_"), kind.label()));
    let mut st = state(kind);
    st.run("file.batchProcess", json!({}));
    let paths: Vec<PathBuf> = (0..n as u32).map(|i| png_file(&d, &format!("bild_{:02}.png", i + 1), i, 120 + i * 8, 80 + i * 4)).collect();
    let mut h = harness_ex(st, w, hgt, 1.0, false);
    {
        let s = studio(&mut h);
        s.mode = mode;
        s.add_paths(&paths, false);
        batch::recipe::add_step(s, id);
    }
    settle(&mut h);
    (h, d)
}

fn param(h: &mut H, name: &str) -> Option<serde_json::Value> {
    studio(h).recipe.steps[0].params.get(name).cloned()
}

// ------------------------------------------------------------------ Felder je Modus

#[test]
fn felder_passend_zum_modus_des_schritts() {
    let (mut h, _d) = studio_with_step(2, "resize", Mode::Beginner, ThemeKind::Normal, 1440.0, 900.0);
    let size_l = params::param_label("resize", "size");
    let width_l = params::param_label("resize", "width");
    assert!(find(&h, &size_l).is_some(), "Längste Seite sichtbar");
    assert!(find(&h, &width_l).is_none(), "Breite gehört nicht zu „Längste Seite“");
    // Modus „Breite“ → Breite sichtbar, Längste Seite nicht; der Wert von „size“ bleibt erhalten
    studio(&mut h).edit("t", |r| r.set_param(0, "size", json!(900)) && r.set_param(0, "mode", json!("width")));
    studio(&mut h).end_gesture();
    h.run_steps(3);
    assert!(find(&h, &width_l).is_some());
    assert!(find(&h, &size_l).is_none());
    assert_eq!(param(&mut h, "size"), Some(json!(900)), "versteckter Wert bleibt erhalten");
    // Zusammenfassung nennt nur die passenden Werte
    let reg = fp_batch::StepRegistry::global();
    let defs = reg.get("crop").unwrap().params();
    let sum = params::auto_summary("crop", &defs, &json!({"mode": "rect", "unit": "px"}), Mode::Pro);
    assert!(!sum.contains(&params::param_label("crop", "ratio")), "{sum}");
    assert!(sum.contains(&params::param_label("crop", "x")), "{sum}");
    assert_clean(&h, "resize");
}

// ------------------------------------------------------------------ Schieber, Feld, Verlauf, Vorschau

#[test]
fn schieber_ziehen_ein_verlaufsschritt_und_entprellte_vorschau() {
    let (mut h, _d) = studio_with_step(2, "resize", Mode::Beginner, ThemeKind::Normal, 1440.0, 900.0);
    let label = params::param_label("resize", "size");
    let slider = node(&h, &label, egui::accesskit::Role::Slider).expect("Schieber");
    let n0 = studio(&mut h).undo_len();
    let g0 = studio(&mut h).preview.generation();
    // 30 Bilder Ziehen (½ s): währenddessen keine neue Vorschau-Rechnung (entprellt)
    let y = slider.center().y;
    h.event(Event::PointerMoved(Pos2::new(slider.left() + 4.0, y)));
    h.run_steps(1);
    h.event(Event::PointerButton { pos: Pos2::new(slider.left() + 4.0, y), button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.run_steps(1);
    let mut seen = Vec::new();
    for k in 1..=30 {
        let x = slider.left() + 4.0 + (slider.width() * 0.6) * (k as f32 / 30.0);
        h.event(Event::PointerMoved(Pos2::new(x, y)));
        h.run_steps(1);
        seen.push(param(&mut h, "size"));
    }
    assert_eq!(studio(&mut h).preview.generation(), g0, "keine Rechnung während des Ziehens");
    h.event(Event::PointerButton { pos: Pos2::new(slider.left() + 4.0 + slider.width() * 0.6, y), button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
    let fin = param(&mut h, "size").expect("Wert gesetzt");
    assert!(seen.windows(2).any(|w| w[0] != w[1]), "Wert ändert sich beim Ziehen");
    assert_eq!(studio(&mut h).undo_len(), n0 + 1, "ein Rückgängig-Schritt je Zug");
    assert_eq!(studio(&mut h).recipe.steps.len(), 1, "Schritt nur einmal");
    settle(&mut h);
    assert_eq!(studio(&mut h).preview.generation(), g0 + 1, "genau eine Rechnung nach dem Loslassen");
    assert!(studio(&mut h).preview.shown.is_some());
    // Rückgängig stellt exakt her, Wiederholen den Endwert
    studio(&mut h).undo();
    assert_eq!(param(&mut h, "size"), None);
    studio(&mut h).redo();
    assert_eq!(param(&mut h, "size"), Some(fin));
}

#[test]
fn zahlenfeld_tippen_fehler_und_zuruecksetzen() {
    let (mut h, _d) = studio_with_step(1, "resize", Mode::Beginner, ThemeKind::Dark, 1440.0, 900.0);
    let label = params::param_label("resize", "size");
    let field = node(&h, &label, egui::accesskit::Role::TextInput).expect("Zahlenfeld");
    let n0 = studio(&mut h).undo_len();
    click(&mut h, field.center());
    key(&mut h, Key::A, Modifiers::COMMAND); // im Feld: alles markieren (nicht die Dateiliste)
    assert!(studio(&mut h).files.iter().all(|f| f.enabled), "Strg+A im Feld wirkt nicht auf die Liste");
    type_text(&mut h, "1200");
    assert_eq!(param(&mut h, "size"), Some(json!(1200)));
    // Tippfehler: rote Meldung, Wert bleibt
    key(&mut h, Key::A, Modifiers::COMMAND);
    type_text(&mut h, "12x");
    let msg = fp_i18n::tr!("batch-form-range", label = label.clone(), min = "1", max = "50000");
    let shown = texts(&h).into_iter().any(|x| x.text.starts_with(&format!("{label}:")));
    assert!(shown, "Meldung sichtbar (z. B. „{msg}“)");
    assert_eq!(param(&mut h, "size"), Some(json!(12)), "letzter gültiger Wert bleibt");
    key(&mut h, Key::A, Modifiers::COMMAND);
    type_text(&mut h, "1600,0");
    key(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(2);
    assert_eq!(param(&mut h, "size"), Some(json!(1600)), "Dezimalkomma wird gelesen");
    assert_eq!(studio(&mut h).undo_len(), n0 + 1, "Tippen im selben Feld = ein Schritt");
    // Vorgabe ist 1600 → kein ↺; anderer Wert → ↺ setzt zurück
    studio(&mut h).edit("x", |r| r.set_param(0, "size", json!(777)));
    studio(&mut h).end_gesture();
    h.run_steps(2);
    let reset = format!("{}: {label}", t("batch-form-reset"));
    let r = node(&h, &reset, egui::accesskit::Role::Button).expect("Zurücksetzen");
    click(&mut h, r.center());
    assert_eq!(param(&mut h, "size"), None, "Vorgabe gilt wieder");
    studio(&mut h).undo();
    assert_eq!(param(&mut h, "size"), Some(json!(777)));
}

#[test]
fn geste_je_parameter() {
    // Tippen in Feld A, dann (ohne Maus) Feld B: zwei Rückgängig-Schritte
    assert_eq!(batch::recipe::changed_names(&json!({"a": 1, "b": 2}), &json!({"a": 1, "b": 3, "c": 0})), vec!["b".to_string(), "c".to_string()]);
    assert!(batch::recipe::changed_names(&json!({"a": 1}), &json!({"a": 1})).is_empty());
    let (mut h, _d) = studio_with_step(1, "adjust.basic", Mode::Beginner, ThemeKind::Normal, 1440.0, 900.0);
    let n0 = studio(&mut h).undo_len();
    let a = node(&h, &params::param_label("adjust.basic", "brightness"), egui::accesskit::Role::TextInput).expect("Helligkeit");
    click(&mut h, a.center());
    key(&mut h, Key::A, Modifiers::COMMAND);
    type_text(&mut h, "10");
    let b = node(&h, &params::param_label("adjust.basic", "contrast"), egui::accesskit::Role::TextInput).expect("Kontrast");
    click(&mut h, b.center());
    key(&mut h, Key::A, Modifiers::COMMAND);
    type_text(&mut h, "20");
    key(&mut h, Key::Enter, Modifiers::NONE);
    assert_eq!(param(&mut h, "brightness"), Some(json!(10)));
    assert_eq!(param(&mut h, "contrast"), Some(json!(20)));
    assert_eq!(studio(&mut h).undo_len(), n0 + 2, "je Parameter ein Schritt");
    studio(&mut h).undo();
    assert_eq!(param(&mut h, "contrast"), None);
    assert_eq!(param(&mut h, "brightness"), Some(json!(10)));
}

// ------------------------------------------------------------------ Auswahl, Chips, Automatisch

#[test]
fn segment_und_chips() {
    let (mut h, _d) = studio_with_step(1, "metadata.edit", Mode::Beginner, ThemeKind::Oled, 1440.0, 900.0);
    let none_l = t("batch-value-metadata-edit-keep-none");
    let all_l = t("batch-value-metadata-edit-keep-all");
    let cam = format!("{}: {}", params::param_label("metadata.edit", "remove"), t("batch-value-metadata-edit-remove-camera"));
    assert!(node(&h, &cam, egui::accesskit::Role::CheckBox).is_some(), "Chips sichtbar");
    // lange Werte → Liste statt Segment
    let keep_l = params::param_label("metadata.edit", "keep");
    let combo = node(&h, &keep_l, egui::accesskit::Role::ComboBox).expect("Liste „Behalten“");
    click(&mut h, combo.center());
    let item = find(&h, &none_l).expect("Eintrag „Alle entfernen“");
    click(&mut h, item.rect.center());
    assert_eq!(param(&mut h, "keep"), Some(json!("none")));
    assert!(node(&h, &cam, egui::accesskit::Role::CheckBox).is_none(), "Chips nur bei „behalten“/„Auswahl“");
    let combo = node(&h, &keep_l, egui::accesskit::Role::ComboBox).expect("Liste");
    click(&mut h, combo.center());
    let item = texts(&h).into_iter().rfind(|x| x.text == all_l).expect("Eintrag „Alle behalten“");
    click(&mut h, item.rect.center());
    let chip = node(&h, &cam, egui::accesskit::Role::CheckBox).expect("Chip Kamera");
    click(&mut h, chip.center());
    assert_eq!(param(&mut h, "remove"), Some(json!(["gps", "camera"])), "Reihenfolge wie im Schema");
    // 50× an/aus: Endzustand wie einmal, keine Duplikate
    for _ in 0..50 {
        let chip = node(&h, &cam, egui::accesskit::Role::CheckBox).expect("Chip");
        click(&mut h, chip.center());
    }
    assert_eq!(param(&mut h, "remove"), Some(json!(["gps", "camera"])));
    assert_clean(&h, "metadata.edit OLED");
}

#[test]
fn automatisch_fuer_werte_ohne_vorgabe() {
    let (mut h, _d) = studio_with_step(1, "vector.trace", Mode::Pro, ThemeKind::Normal, 1440.0, 1000.0);
    let turd = params::param_label("vector.trace", "turdSize");
    assert!(find(&h, &turd).is_none(), "Konturwerte erst bei potrace");
    let potrace = node(&h, &t("batch-value-vector-trace-engine-potrace"), egui::accesskit::Role::RadioButton).expect("Segment potrace");
    click(&mut h, potrace.center());
    assert_eq!(param(&mut h, "engine"), Some(json!("potrace")));
    assert!(find(&h, &turd).is_some());
    assert!(texts(&h).iter().any(|x| x.text == t("batch-form-auto")), "leeres Feld zeigt „Automatisch“");
    let field = node(&h, &turd, egui::accesskit::Role::TextInput).expect("Feld");
    click(&mut h, field.center());
    type_text(&mut h, "5");
    key(&mut h, Key::Enter, Modifiers::NONE);
    assert_eq!(param(&mut h, "turdSize"), Some(json!(5)));
    let reset = format!("{}: {turd}", t("batch-form-reset-auto"));
    let r = node(&h, &reset, egui::accesskit::Role::Button).expect("Zurück auf Automatisch");
    click(&mut h, r.center());
    assert_eq!(param(&mut h, "turdSize"), None);
    // Kästchen ohne Vorgabe: Automatisch | An | Aus
    let off = node(&h, &t("batch-form-off"), egui::accesskit::Role::RadioButton).expect("Aus");
    click(&mut h, off.center());
    let offs = ["ignoreTransparent", "refine", "optCurve"].iter().filter(|n| param(&mut h, n) == Some(json!(false))).count();
    assert_eq!(offs, 1, "genau ein Kästchen auf „Aus“");
    assert_clean(&h, "vector.trace");
}

// ------------------------------------------------------------------ Farbe

#[test]
fn farbe_mit_dem_farbwaehler_des_originals() {
    use fp_ui::inspector::color_picker as cp;
    let (mut h, _d) = studio_with_step(1, "background.flatten", Mode::Beginner, ThemeKind::Normal, 1440.0, 900.0);
    let label = params::param_label("background.flatten", "color");
    let sw = node(&h, &label, egui::accesskit::Role::ColorWell).expect("Farbfeld");
    let n0 = studio(&mut h).undo_len();
    click(&mut h, sw.center());
    let swatch = egui::Id::new(("fp-batch-param", "background.flatten", Some(0usize), &String::from("color"))).with("swatch");
    let popup = cp::open_rect(&h.ctx, swatch).expect("Farbwähler-Popup offen");
    let cell = cp::cell_rect(popup.min, 0, 5).center();
    click(&mut h, cell);
    h.run_steps(2);
    assert!(cp::open_rect(&h.ctx, swatch).is_none(), "Klick auf Zelle schließt");
    let want = cp::grid_color(0, 5);
    let hex = format!("#{:02x}{:02x}{:02x}", want.r(), want.g(), want.b());
    assert_eq!(param(&mut h, "color"), Some(json!(hex)));
    assert_eq!(studio(&mut h).undo_len(), n0 + 1);
    // HEX-Feld tippen
    let field = node(&h, &label, egui::accesskit::Role::TextInput).expect("HEX-Feld");
    click(&mut h, field.center());
    key(&mut h, Key::A, Modifiers::COMMAND);
    type_text(&mut h, "#123456");
    key(&mut h, Key::Enter, Modifiers::NONE);
    assert_eq!(param(&mut h, "color"), Some(json!("#123456")));
}

// ------------------------------------------------------------------ Einstellungen (ffmpeg)

#[test]
fn einstellungen_mit_ffmpeg_zustand() {
    for code in ["de", "en"] {
        lang(code);
        for kind in ThemeKind::ALL {
            let (mut h, _d) = studio_with_step(1, "resize", Mode::Beginner, kind, 1024.0, 768.0);
            let more = h.query_by_label(&t("batch-ui-more")).map(|n| n.rect()).expect("⋯");
            click(&mut h, more.center());
            let item = find(&h, &t("batch-settings-open")).expect("Menüeintrag Einstellungen");
            click(&mut h, item.rect.center());
            assert!(studio(&mut h).settings.is_some(), "Einstellungen offen");
            assert!(find(&h, &t("batch-settings-title")).is_some());
            settle(&mut h);
            let stt = studio(&mut h).settings.as_ref().and_then(|x| x.ffmpeg.clone()).expect("geprüft");
            let head = if stt.is_ok() { fp_i18n::tr!("batch-settings-ffmpeg-ok", version = stt.version.clone().unwrap_or_default()) } else { t("batch-settings-ffmpeg-missing") };
            assert!(find(&h, &head).is_some(), "{code}: Zustand „{head}“");
            if !stt.is_ok() {
                assert!(texts(&h).iter().any(|x| x.text.starts_with(&t("batch-settings-ffmpeg-hint")[..20])), "Hinweis bei fehlendem ffmpeg");
            }
            assert_clean(&h, &format!("Einstellungen {code} {}", kind.label()));
            // Erneut prüfen → prüft, dann wieder Zustand
            let re = node(&h, &t("batch-settings-ffmpeg-recheck"), egui::accesskit::Role::Button).expect("Erneut prüfen");
            click(&mut h, re.center());
            settle(&mut h);
            assert!(studio(&mut h).settings.as_ref().is_some_and(|x| !x.checking()));
            // Klick neben die Karte schließt nichts und erreicht das Studio darunter nicht
            let rec_before = studio(&mut h).recipe.clone();
            click(&mut h, Pos2::new(20.0, 400.0));
            assert!(studio(&mut h).settings.is_some());
            assert_eq!(studio(&mut h).recipe, rec_before);
            key(&mut h, Key::Escape, Modifiers::NONE);
            assert!(studio(&mut h).settings.is_none(), "Esc schließt");
        }
    }
    lang("de");
}

// ------------------------------------------------------------------ Rückfrage beim Schließen

/// Studio mit laufendem Auftrag, der auf eine Konfliktfrage wartet (Regel „Jedes Mal fragen“).
fn running_studio(name: &str) -> (H, PathBuf) {
    let d = dir(&format!("close-ask-{name}"));
    let out = d.join("out");
    std::fs::create_dir_all(&out).unwrap();
    let mut st0 = state(ThemeKind::Normal);
    st0.run("file.batchProcess", json!({}));
    let paths: Vec<PathBuf> = (0..3u32).map(|i| png_file(&d, &format!("f{i}.png"), i, 40, 30)).collect();
    for p in &paths {
        std::fs::copy(p, out.join(p.file_name().unwrap())).unwrap();
    }
    let mut h = harness_ex(st0, 1440.0, 900.0, 1.0, false);
    {
        let s = studio(&mut h);
        s.add_paths(&paths, false);
        s.edit("o", |r| {
            r.output.format = "png".into();
            r.output.target.kind = fp_batch::recipe::TargetKind::Folder;
            r.output.target.path = Some(out.clone());
            r.output.conflict = fp_batch::recipe::Conflict::Ask;
            true
        });
    }
    h.run_steps(2);
    batch::output_bar::start(studio(&mut h), 0.0);
    for _ in 0..300 {
        h.run_steps(1);
        if studio(&mut h).run.as_ref().is_some_and(|r| r.ask.is_some()) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(studio(&mut h).running(), "Lauf wartet auf die Konfliktfrage");
    (h, d)
}

#[test]
fn fenster_schliessen_waehrend_des_laufs_fragt() {
    let (mut h, _d) = running_studio("window");
    // Dokument vorn: die Rückfrage holt das Studio nach vorn
    studio(&mut h).front = false;
    st(&mut h).push(Action::Quit);
    h.run_steps(3);
    assert!(!h.state().state.quit_confirmed, "nicht sofort beenden");
    assert!(batch::is_front(&h.state().state));
    assert!(find(&h, &t("batch-quit-text")).is_some(), "Rückfrage sichtbar");
    // Weiterlaufen lassen
    let keep = node(&h, &t("batch-quit-keep"), egui::accesskit::Role::Button).expect("Weiterlaufen");
    click(&mut h, keep.center());
    assert!(studio(&mut h).close_ask.is_none());
    assert!(studio(&mut h).running(), "Lauf läuft weiter");
    assert!(!h.state().state.quit_confirmed);
    // Esc = weiterlaufen lassen
    st(&mut h).push(Action::Quit);
    h.run_steps(3);
    key(&mut h, Key::Escape, Modifiers::NONE);
    assert!(studio(&mut h).close_ask.is_none() && studio(&mut h).running());
    // Abbrechen und schließen → Lauf abgebrochen, Programm beendet sich (keine ungespeicherten Dokumente)
    st(&mut h).push(Action::Quit);
    h.run_steps(3);
    let ok = node(&h, &t("batch-quit-confirm"), egui::accesskit::Role::Button).expect("Abbrechen und schließen");
    click(&mut h, ok.center());
    h.run_steps(3);
    assert!(h.state().state.quit_confirmed, "beendet");
}

#[test]
fn studio_schliessen_waehrend_des_laufs_fragt() {
    let (mut h, _d) = running_studio("studio");
    let x = h.query_by_label(&t("batch-ui-close")).map(|n| n.rect()).expect("× am Reiter");
    click(&mut h, x.center());
    assert!(batch::is_open(&h.state().state), "Studio bleibt offen");
    assert_eq!(studio(&mut h).close_ask, Some(batch::settings::CloseAsk::Studio));
    let ok = node(&h, &t("batch-quit-confirm"), egui::accesskit::Role::Button).expect("Abbrechen und schließen");
    click(&mut h, ok.center());
    h.run_steps(3);
    assert!(!batch::is_open(&h.state().state), "Studio geschlossen, Lauf abgebrochen");
    assert!(!h.state().state.quit_confirmed, "Programm läuft weiter");
}

// ------------------------------------------------------------------ Größen, Designs, Sprachen

#[test]
fn formulare_nichts_abgeschnitten() {
    for code in ["de", "en"] {
        lang(code);
        for kind in ThemeKind::ALL {
            for (w, hgt, ppp) in [(1024.0, 768.0, 1.0), (1920.0, 1080.0, 1.0), (1024.0, 768.0, 1.5)] {
                for (id, mode) in [("overlay.text", Mode::Beginner), ("background.replace", Mode::Pro), ("vector.trace", Mode::Pro), ("metadata.edit", Mode::Beginner)] {
                    let d = dir(&format!("clip-{code}-{}-{w}-{ppp}-{id}", kind.label()));
                    let mut st = state(kind);
                    st.run("file.batchProcess", json!({}));
                    let mut h = harness_ex(st, w, hgt, ppp, false);
                    {
                        let p = png_file(&d, "ein_recht_langer_dateiname.png", 1, 60, 40);
                        let s = studio(&mut h);
                        s.mode = mode;
                        s.add_paths(&[p], false);
                        batch::recipe::add_step(s, id);
                        if id == "vector.trace" {
                            s.edit("e", |r| r.set_param(0, "engine", json!("potrace")));
                        }
                    }
                    settle(&mut h);
                    assert_clean(&h, &format!("{code}/{}/{w}x{hgt}@{ppp} {id}", kind.label()));
                }
            }
        }
    }
    lang("de");
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

/// Bilder: 3 Designs × de/en, 1440 × 900: Formular (Text-Wasserzeichen offen) und Einstellungen.
#[test]
#[ignore]
fn batch11a_bilder() {
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("docs").join("screenshots");
    let _ = std::fs::create_dir_all(&out);
    let d = dir("bilder");
    let names = ["urlaub_strand.png", "urlaub_berge.png", "logo_entwurf.png", "team_foto.png", "banner_web.png"];
    let paths: Vec<PathBuf> = names.iter().enumerate().map(|(i, n)| png_file(&d, n, i as u32, 640 + 80 * i as u32, 420 + 40 * i as u32)).collect();
    for code in ["de", "en"] {
        lang(code);
        for (kind, name) in [(ThemeKind::Normal, "normal"), (ThemeKind::Dark, "dunkel"), (ThemeKind::Oled, "oled")] {
            for view in ["form", "settings"] {
                let mut st = state(kind);
                st.run("file.batchProcess", json!({}));
                let mut h = harness_ex(st, 1440.0, 900.0, 1.0, true);
                {
                    let s = studio(&mut h);
                    s.add_paths(&paths, false);
                    s.selected = Some(1);
                    batch::recipe::add_step(s, "resize");
                    batch::recipe::add_step(s, "overlay.text");
                    s.edit("p", |r| r.set_param(1, "text", json!("© 2026 Firepixel")) && r.set_param(1, "size", json!(6.5)) && r.set_param(1, "color", json!("#ffd166")));
                    s.end_gesture();
                    s.open_card = Some(CardId::Step(1));
                    s.edit("o", |r| {
                        r.output.format = "webp".into();
                        r.output.options.insert("quality".into(), json!(82));
                        true
                    });
                    s.end_gesture();
                }
                if view == "settings" {
                    let ctx = h.ctx.clone();
                    batch::settings::open(&ctx, studio(&mut h));
                }
                settle(&mut h);
                for x in texts(&h) {
                    assert!(fp_neutral::find_in_bytes(x.text.as_bytes()).is_empty(), "gesperrter Name im Bild: {}", x.text);
                }
                let Ok(img) = h.render() else {
                    eprintln!("kein Grafikadapter");
                    return;
                };
                let p = out.join(format!("batch11a_{view}_{name}_{code}.png"));
                save_png(&p, &img);
                let bytes = std::fs::read(&p).expect("PNG lesen");
                assert!(fp_neutral::find_in_bytes(&bytes).is_empty(), "gesperrter Name in {}", p.display());
            }
        }
    }
    lang("de");
}
