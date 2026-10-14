//! BATCH-2: Batch-Studio (`src\batch\`) – Reiter im Hauptfenster, Dateiliste, Rezept-Karten,
//! Galerie, Vorschau, Ausgabe-Leiste, Lauf. Logikfälle nach AGENTEN_REGELN §4b (Wiederholung,
//! Rückgängig-Folgen, Abbrechen der Galerie, Auswahlwechsel während der Vorschau), Aufbau in drei
//! Designs × de/en ohne abgeschnittene Texte. Bilder (`--ignored batch2_bilder`):
//! `docs\screenshots\batch2_<design>_<sprache>.png`.

use egui::epaint::Shape;
use egui::{vec2, Event, Key, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use fp_i18n::t;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::batch::{self, CardId, EntryStatus, Mode};
use fp_ui::theme::ThemeKind;
use fp_ui::FirepixelApp;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;

// ------------------------------------------------------------------ Hilfen

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join("firepixel-batch2").join(std::process::id().to_string()).join(name);
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
            let (cx, cy) = (x as i32 - (w as i32 / 3 + (seed as i32 * 17) % (w as i32 / 3).max(1)), y as i32 - h as i32 / 2);
            let r = (h as i32 / 4).max(2);
            let disc = cx * cx + cy * cy < r * r;
            let base = match seed % 4 {
                0 => [(255.0 * fx) as u8, (120.0 + 100.0 * fy) as u8, 220],
                1 => [250, (180.0 * (1.0 - fy)) as u8 + 40, (60.0 + 120.0 * fx) as u8],
                2 => [(40.0 + 80.0 * fy) as u8, (160.0 + 90.0 * fx) as u8, (120.0 + 60.0 * fy) as u8],
                _ => [(30.0 + 200.0 * fy) as u8, (60.0 + 60.0 * fx) as u8, (150.0 + 100.0 * fx) as u8],
            };
            let c = if disc { [255, 255, 255 - (seed * 50 % 200) as u8, 255] } else if seed % 5 == 4 && fx > 0.7 { [0, 0, 0, 0] } else { [base[0], base[1], base[2], 255] };
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
    let b = Harness::builder().with_size(vec2(w / ppp, h / ppp)).with_pixels_per_point(ppp).with_max_steps(30).with_step_dt(1.0 / 60.0);
    let b = if gpu { b.wgpu() } else { b };
    let mut hn = b.build_ui_state(|ui, a: &mut FirepixelApp| a.root_ui(ui), app(st));
    hn.run_steps(4);
    hn
}

fn harness(st: AppState) -> H {
    harness_ex(st, 1440.0, 900.0, 1.0, false)
}

fn st(h: &mut H) -> &mut AppState {
    &mut h.state_mut().state
}

fn studio(h: &mut H) -> &mut batch::Studio {
    batch::studio_mut(st(h)).expect("Studio offen")
}

/// Wartet, bis Vorschau und Miniaturen fertig sind (Hintergrund-Threads).
fn settle(h: &mut H) {
    for _ in 0..600 {
        h.run_steps(1);
        let busy = batch::studio(&h.state().state).is_some_and(|s| s.preview.busy() || s.running());
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

fn lang(code: &str) {
    assert!(fp_i18n::set_language(code), "Sprache {code}");
}

#[derive(Debug)]
struct TestDrop(PathBuf);

impl egui::DroppedFile for TestDrop {
    fn path(&self) -> &Path {
        &self.0
    }
    fn bytes(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.0).map_err(|e| e.to_string())
    }
}

fn drop_files(h: &mut H, paths: &[PathBuf], at: Pos2) {
    h.event(Event::PointerMoved(at));
    let files: Vec<egui::DroppedFileHandle> = paths.iter().map(|p| Arc::new(TestDrop(p.clone())) as egui::DroppedFileHandle).collect();
    h.input_mut().dropped_files = files;
    h.run_steps(2);
}

/// Prüft: keine Rohschlüssel, nichts waagerecht abgeschnitten (Text in seinem Clip-Bereich bzw. im
/// Fenster), außer am Rand eines Bildlaufbereichs (gewollt) oder mit „…“ gekürzt.
fn assert_clean(h: &H, label: &str) {
    let screen = h.ctx.content_rect();
    for x in texts(h) {
        assert!(!x.text.contains("batch-ui-") && !x.text.contains("batch-param-"), "{label}: Rohschlüssel {:?}", x.text);
        let r = x.rect;
        let clip = x.clip.intersect(screen);
        if r.width() <= 0.0 || !clip.intersects(r) {
            continue;
        }
        // senkrecht angeschnitten (Bildlauf) ist gewollt; waagerecht nicht
        let fully_v = r.top() >= clip.top() - 1.0 && r.bottom() <= clip.bottom() + 1.0;
        if fully_v {
            assert!(r.left() >= clip.left() - 1.5 && r.right() <= clip.right() + 1.5, "{label}: Text abgeschnitten {:?} {:?} Clip {:?}", x.text, r, x.clip);
        }
    }
}

fn open_with_files(n: usize, kind: ThemeKind) -> (H, PathBuf) {
    let d = dir(&format!("files-{n}-{}", kind.label()));
    let mut st = state(kind);
    st.run("file.batchProcess", json!({}));
    let paths: Vec<PathBuf> = (0..n as u32).map(|i| png_file(&d, &format!("urlaub_{:02}.png", i + 1), i, 96 + i * 8, 64 + i * 4)).collect();
    let mut h = harness(st);
    if n > 0 {
        let s = studio(&mut h);
        s.add_paths(&paths, false);
    }
    h.run_steps(3);
    (h, d)
}

// ------------------------------------------------------------------ Aufbau und Einstieg

#[test]
fn menue_oeffnet_studio_als_reiter() {
    let mut h = harness(state(ThemeKind::Normal));
    assert!(!batch::is_open(&h.state().state));
    st(&mut h).run("file.batchProcess", json!({}));
    h.run_steps(4);
    assert!(batch::is_front(&h.state().state), "Datei ▸ Batch-Verarbeitung öffnet das Studio");
    assert!(find(&h, &t("batch-ui-title")).is_some(), "Banner sichtbar");
    assert!(find(&h, &t("batch-ui-files-title")).is_some());
    assert!(find(&h, &t("batch-ui-recipe-title")).is_some());
    assert!(find(&h, &t("batch-ui-preview-title")).is_some());
    assert!(find(&h, &t("batch-ui-tab")).is_some(), "Reiter „Batch“");
    // nochmal aufrufen: kein zweites Studio, nur nach vorn
    st(&mut h).run("file.batchProcess", json!({}));
    h.run_steps(2);
    assert!(batch::is_front(&h.state().state));
    // Neues Dokument → Dokument vorn, Studio bleibt offen
    st(&mut h).run("file.new", json!({"width": 200, "height": 100}));
    h.run_steps(4);
    assert!(batch::is_open(&h.state().state) && !batch::is_front(&h.state().state), "neues Dokument kommt nach vorn");
    // Klick auf den Reiter „Batch“ holt das Studio zurück
    let tab = find(&h, &t("batch-ui-tab")).expect("Reiter");
    click(&mut h, tab.rect.center());
    assert!(batch::is_front(&h.state().state));
    // Strg+Z wirkt im Studio auf das Rezept (eigener Verlauf), nicht auf das Dokument
    studio(&mut h).edit("o", |r| {
        r.output.format = "png".into();
        true
    });
    studio(&mut h).end_gesture();
    let doc_dirty = h.state().state.session.active().map(|d| d.is_dirty());
    key(&mut h, Key::Z, Modifiers::COMMAND);
    assert_eq!(studio(&mut h).recipe.output.format, "same", "Rezept zurückgenommen");
    assert_eq!(h.state().state.session.active().map(|d| d.is_dirty()), doc_dirty);
    // Schließen über × am Reiter
    let x = h.query_by_label(&t("batch-ui-close")).map(|n| n.rect()).expect("Schließen");
    click(&mut h, x.center());
    assert!(!batch::is_open(&h.state().state));
}

#[test]
fn mit_rezept_ohne_oberflaeche_an_batch_run() {
    // file.batchProcess mit `recipe` läuft über batch.run (Bericht zurück)
    let d = dir("cmd-recipe");
    let a = png_file(&d, "a.png", 1, 40, 30);
    let mut st = state(ThemeKind::Normal);
    fp_batch::install();
    let out = st.session.execute("file.batchProcess", json!({"files": [a.to_string_lossy()], "recipe": {"output": {"format": "png"}}})).expect("batch.run");
    assert_eq!(out.value["totals"]["done"], json!(1), "{}", out.value);
    assert!(d.join("batch-output").join("a.png").exists());
}

#[test]
fn ablegen_fuegt_hinzu_doppelte_nicht() {
    let (mut h, d) = open_with_files(0, ThemeKind::Normal);
    let a = png_file(&d, "a.png", 1, 40, 30);
    let b = png_file(&d, "b.png", 2, 40, 30);
    let txt = d.join("notiz.txt");
    std::fs::write(&txt, b"x").unwrap();
    let center = h.ctx.content_rect().center();
    drop_files(&mut h, &[a.clone(), b.clone(), txt], center);
    assert_eq!(studio(&mut h).files.len(), 2, "Textdatei wird übersprungen");
    drop_files(&mut h, std::slice::from_ref(&a), center);
    assert_eq!(studio(&mut h).files.len(), 2, "Doppelte nicht erneut");
    let notice = studio(&mut h).notice.clone().expect("Hinweis");
    assert!(notice.text.contains('1'), "{}", notice.text);
    // ganzer Ordner (rekursiv)
    let sub = d.join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    png_file(&sub, "c.png", 3, 20, 20);
    drop_files(&mut h, std::slice::from_ref(&d), center);
    assert_eq!(studio(&mut h).files.len(), 3);
    // Entf entfernt die markierte Datei
    studio(&mut h).selected = Some(0);
    key(&mut h, Key::Delete, Modifiers::NONE);
    assert_eq!(studio(&mut h).files.len(), 2);
}

#[test]
fn leerer_zustand_und_knoepfe() {
    let (mut h, _d) = open_with_files(0, ThemeKind::Normal);
    assert!(find(&h, &t("batch-ui-files-empty")).is_some());
    assert!(find(&h, &t("batch-ui-recipe-empty")).is_some());
    // Start gesperrt bei leerer Liste
    assert!(batch::output_bar::blocked(studio(&mut h)).is_some());
    assert_clean(&h, "leer");
}

// ------------------------------------------------------------------ Rezept: Logikfälle §4b

#[test]
fn schritt_50x_hinzufuegen_entfernen_ohne_reste() {
    let mut s = batch::Studio::new(Mode::Pro);
    for _ in 0..50 {
        batch::recipe::add_step(&mut s, "file.copyOnly");
        assert_eq!(s.recipe.steps.len(), 1);
        assert_eq!(s.open_card, Some(CardId::Step(0)), "zuletzt hinzugefügte Karte offen");
        s.edit("remove", |r| r.remove_step(0).is_some());
        s.end_gesture();
        assert!(s.recipe.steps.is_empty());
    }
    assert!(s.recipe.extra.is_empty());
    // 50 Hinzufügen + 50 Entfernen = 100 Verlaufsschritte, alles rückgängig = leer
    assert_eq!(s.undo_len(), 100);
}

#[test]
fn parameter_50x_aendern_ein_verlaufsschritt() {
    let mut s = batch::Studio::new(Mode::Pro);
    batch::recipe::add_step(&mut s, "file.copyOnly");
    let before = s.undo_len();
    // ein Schieberzug / Tippen im selben Feld: 50 Ereignisse, eine Geste
    for k in 0..50 {
        let v = if k % 2 == 0 { "move" } else { "copy" };
        s.edit("step0", |r| r.set_param(0, "action", json!(v)));
    }
    s.end_gesture();
    assert_eq!(s.undo_len(), before + 1, "ein Rückgängig-Schritt je Zug");
    assert_eq!(s.recipe.steps.len(), 1, "Schritt nur einmal");
    assert_eq!(s.recipe.steps[0].params["action"], json!("copy"), "Endwert");
    // gleicher Wert erneut: keine Änderung, kein Schritt
    assert!(!s.edit("step0", |r| r.set_param(0, "action", json!("copy"))));
    assert_eq!(s.undo_len(), before + 1);
}

#[test]
fn rueckgaengig_wiederholen_folge() {
    // A, B, ↶, C, ↷ (C ersetzt den Redo-Zweig)
    let mut s = batch::Studio::new(Mode::Pro);
    s.edit("a", |r| {
        r.output.format = "png".into();
        true
    });
    s.end_gesture();
    s.edit("b", |r| {
        r.output.format = "webp".into();
        true
    });
    s.end_gesture();
    assert!(s.undo());
    assert_eq!(s.recipe.output.format, "png");
    s.edit("c", |r| {
        r.output.format = "jpeg".into();
        true
    });
    s.end_gesture();
    assert!(!s.redo(), "nach neuer Änderung kein Wiederholen");
    assert_eq!(s.recipe.output.format, "jpeg");
    assert!(s.undo());
    assert!(s.undo());
    assert_eq!(s.recipe.output.format, "same");
    assert!(!s.undo());
    assert!(s.redo() && s.redo());
    assert_eq!(s.recipe.output.format, "jpeg");
}

#[test]
fn galerie_per_klick_hinzufuegen_und_esc_abbrechen() {
    let (mut h, _d) = open_with_files(2, ThemeKind::Normal);
    let add = find(&h, &t("batch-ui-add-step")).expect("Schritt hinzufügen");
    click(&mut h, add.rect.center());
    assert!(studio(&mut h).gallery.is_some(), "Galerie offen");
    assert!(find(&h, &t("batch-ui-gallery-title")).is_some());
    // Esc: Galerie zu, Rezept unverändert
    let before = studio(&mut h).recipe.clone();
    key(&mut h, Key::Escape, Modifiers::NONE);
    assert!(studio(&mut h).gallery.is_none());
    assert_eq!(studio(&mut h).recipe, before);
    // Profi-Modus: Suche nach der ID, Eingabe fügt hinzu
    studio(&mut h).mode = Mode::Pro;
    let add2 = find(&h, &t("batch-ui-add-step")).expect("Knopf").rect.center();
    click(&mut h, add2);
    h.run_steps(2);
    for c in "copyOnly".chars() {
        h.event(Event::Text(c.to_string()));
    }
    h.run_steps(2);
    key(&mut h, Key::Enter, Modifiers::NONE);
    assert_eq!(studio(&mut h).recipe.steps.len(), 1, "{:?}", studio(&mut h).recipe.steps);
    assert_eq!(studio(&mut h).recipe.steps[0].id, "file.copyOnly");
    assert!(studio(&mut h).gallery.is_none());
    // Karte offen: Formular sichtbar; Schalter aus → Schritt aus, ein Verlaufsschritt
    h.run_steps(2);
    let label = t("batch-ui-step-enabled");
    let n0 = studio(&mut h).undo_len();
    let node = h.query_by_label(&label).map(|n| n.rect()).expect("Schalter");
    click(&mut h, node.center());
    assert!(!studio(&mut h).recipe.steps[0].enabled);
    assert_eq!(studio(&mut h).undo_len(), n0 + 1);
    // Strg+Z im Studio
    key(&mut h, Key::Z, Modifiers::COMMAND);
    assert!(studio(&mut h).recipe.steps[0].enabled);
    assert_clean(&h, "Galerie/Karte");
}

#[test]
fn verschieben_alt_pfeil_und_menue() {
    let mut s = batch::Studio::new(Mode::Pro);
    batch::recipe::add_step(&mut s, "file.copyOnly");
    s.edit("p", |r| r.set_param(0, "action", json!("move")));
    s.end_gesture();
    batch::recipe::add_step(&mut s, "file.copyOnly");
    assert_eq!(s.open_card, Some(CardId::Step(1)));
    // Einfügeposition beim Ziehen
    let rects = [Rect::from_min_size(Pos2::new(0.0, 0.0), vec2(100.0, 56.0)), Rect::from_min_size(Pos2::new(0.0, 64.0), vec2(100.0, 56.0)), Rect::from_min_size(Pos2::new(0.0, 128.0), vec2(100.0, 56.0))];
    assert_eq!(batch::recipe::insert_index(&rects, 10.0, 2), 0);
    assert_eq!(batch::recipe::insert_index(&rects, 170.0, 0), 2);
    assert_eq!(batch::recipe::insert_index(&rects, 60.0, 1), 1);
    let mut h = harness(state(ThemeKind::Normal));
    st(&mut h).batch = Some(Box::new(s));
    h.run_steps(3);
    key(&mut h, Key::ArrowUp, Modifiers::ALT);
    let s = studio(&mut h);
    assert_eq!(s.open_card, Some(CardId::Step(0)), "offene Karte folgt");
    assert_eq!(s.recipe.steps[1].params["action"], json!("move"), "Reihenfolge getauscht");
}

#[test]
fn modus_wechsel_verliert_keine_werte() {
    let mut s = batch::Studio::new(Mode::Pro);
    s.edit("o", |r| {
        r.output.format = "jpeg".into();
        r.output.options.insert("subsampling".into(), json!("4:4:4"));
        true
    });
    s.mode = Mode::Beginner;
    s.mode = Mode::Pro;
    assert_eq!(s.recipe.output.options["subsampling"], json!("4:4:4"));
    // Zusammenfassung im Anfänger-Modus nennt versteckte Werte
    let reg = fp_batch::StepRegistry::global();
    let defs = reg.get("output").map(|x| x.params()).unwrap_or_default();
    let mut v = serde_json::Value::Object(s.recipe.output.options.clone());
    v["format"] = json!("jpeg");
    let sum = batch::params::auto_summary("output", &defs, &v, Mode::Beginner);
    assert!(sum.contains('+'), "{sum}");
}

// ------------------------------------------------------------------ Vorschau

#[test]
fn vorschau_zeigt_neue_datei_nie_die_alte() {
    let (mut h, _d) = open_with_files(3, ThemeKind::Normal);
    studio(&mut h).selected = Some(0);
    settle(&mut h);
    let g0 = studio(&mut h).preview.generation();
    let w0 = studio(&mut h).preview.shown.as_ref().map(|x| x.before_size).expect("Vorschau 1");
    assert_eq!(w0, (96, 64));
    // Auswahl wechseln, während (bzw. kurz nachdem) gerechnet wird → zeigt Datei 3
    studio(&mut h).selected = Some(1);
    h.run_steps(1);
    studio(&mut h).selected = Some(2);
    settle(&mut h);
    let sh = studio(&mut h).preview.shown.as_ref().map(|x| (x.before_size, x.bytes_after.is_some())).expect("Vorschau 3");
    assert_eq!(sh.0, (112, 72), "Vorschau der zuletzt gewählten Datei");
    assert!(sh.1, "Dateigröße nachher aus echter Kodierung");
    assert!(studio(&mut h).preview.generation() > g0);
    // Rezept ändern → neu gerechnet, Ergebnis ersetzt (kein Aufsummieren)
    studio(&mut h).edit("o", |r| {
        r.output.format = "jpeg".into();
        true
    });
    settle(&mut h);
    let s = studio(&mut h);
    assert!(s.preview.shown.as_ref().is_some_and(|x| x.bytes_after.is_some()));
    assert!(texts(&h).iter().any(|x| x.text.contains("112×72")));
}

// ------------------------------------------------------------------ Trockenlauf und Lauf

#[test]
fn trockenlauf_und_lauf_zweimal() {
    let (mut h, d) = open_with_files(3, ThemeKind::Normal);
    studio(&mut h).edit("o", |r| {
        r.output.format = "png".into();
        true
    });
    key(&mut h, Key::F5, Modifiers::NONE);
    let plan = studio(&mut h).plan.clone().expect("Plan");
    assert_eq!(plan.entries.len(), 3);
    assert!(!d.join("batch-output").exists(), "Trockenlauf schreibt nichts");
    assert!(find(&h, &t("batch-ui-plan-source")).is_some(), "Reiter Plan");
    // Lauf über Strg+Eingabe
    key(&mut h, Key::Enter, Modifiers::COMMAND);
    assert!(studio(&mut h).running() || studio(&mut h).report.is_some());
    settle(&mut h);
    let s = studio(&mut h);
    let rep = s.report.clone().expect("Bericht");
    assert_eq!(rep.totals.done, 3, "{rep:?}");
    assert!(s.files.iter().all(|f| f.status == EntryStatus::Done || f.status == EntryStatus::Warning));
    assert!(d.join("batch-output").join("urlaub_01.png").exists());
    assert!(s.show_summary);
    assert!(find(&h, &t("batch-ui-done-title")).is_some(), "Ende-Karte");
    // zweiter Lauf: neuer Ordner batch-output-2, Effekt nie doppelt
    key(&mut h, Key::Enter, Modifiers::COMMAND);
    settle(&mut h);
    assert!(d.join("batch-output-2").join("urlaub_01.png").exists(), "zweiter Lauf in batch-output-2");
    // Originale unverändert
    assert!(d.join("urlaub_01.png").exists());
}

#[test]
fn rezept_speichern_und_laden() {
    let d = dir("recipe-io");
    let mut s = batch::Studio::new(Mode::Beginner);
    batch::recipe::add_step(&mut s, "file.copyOnly");
    s.edit("o", |r| {
        r.output.format = "webp".into();
        true
    });
    assert!(s.is_dirty());
    let p = d.join("mein web");
    batch::banner::save_recipe(&mut s, &p, 0.0);
    let saved = d.join("mein web.fpbatch.json");
    assert!(saved.exists());
    assert!(!s.is_dirty());
    assert_eq!(s.recipe_title(), "mein web");
    let mut s2 = batch::Studio::new(Mode::Beginner);
    batch::banner::load_recipe(&mut s2, &saved, 0.0);
    assert_eq!(s2.recipe.steps.len(), 1);
    assert_eq!(s2.recipe.output.format, "webp");
    assert!(!s2.can_undo(), "geladenes Rezept beginnt einen neuen Verlauf");
}

// ------------------------------------------------------------------ Größen, Designs, Sprachen

#[test]
fn nichts_abgeschnitten_drei_designs_de_en() {
    for code in ["de", "en"] {
        lang(code);
        for kind in ThemeKind::ALL {
            for (w, hgt, ppp) in [(1024.0, 768.0, 1.0), (1920.0, 1080.0, 1.0), (1024.0, 768.0, 1.5), (1920.0, 1080.0, 1.5)] {
                let d = dir(&format!("clip-{code}-{}-{w}-{ppp}", kind.label()));
                let mut st = state(kind);
                st.run("file.batchProcess", json!({}));
                let mut h = harness_ex(st, w, hgt, ppp, false);
                let paths: Vec<PathBuf> = (0..4u32).map(|i| png_file(&d, &format!("ein_recht_langer_dateiname_fuer_die_liste_{i}.png"), i, 60, 40)).collect();
                {
                    let s = studio(&mut h);
                    s.add_paths(&paths, false);
                    batch::recipe::add_step(s, "file.copyOnly");
                }
                settle(&mut h);
                let label = format!("{code}/{}/{w}x{hgt}@{ppp}", kind.label());
                assert_clean(&h, &label);
                // Galerie offen
                studio(&mut h).gallery = Some(Default::default());
                h.run_steps(3);
                assert_clean(&h, &format!("{label} Galerie"));
            }
        }
    }
    lang("de");
}

#[test]
fn schmal_ein_und_zweispaltig() {
    for w in [700.0, 900.0] {
        let d = dir(&format!("narrow-{w}"));
        let mut st = state(ThemeKind::Dark);
        st.run("file.batchProcess", json!({}));
        let mut h = harness_ex(st, w, 600.0, 1.0, false);
        let p = png_file(&d, "a.png", 1, 50, 50);
        studio(&mut h).add_paths(&[p], false);
        settle(&mut h);
        assert_clean(&h, &format!("{w}"));
        assert!(find(&h, &t("batch-ui-recipe-title")).is_some());
    }
}

fn save_png(path: &Path, img: &image::RgbaImage) {
    let f = std::fs::File::create(path).expect("Datei");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), img.width(), img.height());
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.set_compression(png::Compression::High);
    enc.write_header().and_then(|mut wr| wr.write_image_data(img.as_raw())).expect("PNG");
}

/// Bilder: 3 Designs × de/en, 1440 × 900, mit Dateien, Vorschau und offener Karte.
#[test]
#[ignore]
fn batch2_bilder() {
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("docs").join("screenshots");
    let _ = std::fs::create_dir_all(&out);
    let d = dir("bilder");
    let names = ["urlaub_strand.png", "urlaub_berge.png", "logo_entwurf.png", "team_foto.png", "banner_web.png", "icon_set.png"];
    let paths: Vec<PathBuf> = names.iter().enumerate().map(|(i, n)| png_file(&d, n, i as u32, 640 + 80 * i as u32, 420 + 40 * i as u32)).collect();
    for code in ["de", "en"] {
        lang(code);
        for (kind, name) in [(ThemeKind::Normal, "normal"), (ThemeKind::Dark, "dunkel"), (ThemeKind::Oled, "oled")] {
            let mut st = state(kind);
            st.run("file.batchProcess", json!({}));
            let (bw, bh) = if code == "de" && kind == ThemeKind::Normal && std::env::var_os("FP_BATCH2_SMALL").is_some() { (1024.0, 768.0) } else { (1440.0, 900.0) };
            let mut h = harness_ex(st, bw, bh, 1.0, true);
            {
                let s = studio(&mut h);
                s.add_paths(&paths, false);
                s.selected = Some(1);
                let reg = fp_batch::StepRegistry::global();
                for id in ["resize", "adjust.brightnessContrast", "metadata.edit"] {
                    if reg.contains(id) && s.recipe.steps.len() < 2 {
                        batch::recipe::add_step(s, id);
                    }
                }
                s.edit("o", |r| {
                    r.output.format = "webp".into();
                    r.output.options.insert("quality".into(), json!(82));
                    r.rename.suffix = "_web".into();
                    true
                });
                s.open_card = Some(CardId::Output);
            }
            settle(&mut h);
            for x in texts(&h) {
                assert!(fp_neutral::find_in_bytes(x.text.as_bytes()).is_empty(), "gesperrter Name im Bild: {}", x.text);
            }
            let Ok(img) = h.render() else {
                eprintln!("kein Grafikadapter");
                return;
            };
            let p = out.join(format!("batch2_{name}_{code}.png"));
            save_png(&p, &img);
            let bytes = std::fs::read(&p).expect("PNG lesen");
            assert!(fp_neutral::find_in_bytes(&bytes).is_empty(), "gesperrter Name in {}", p.display());
        }
    }
    lang("de");
}

// ------------------------------------------------------------------ N33: Dateien/Ordner-Knöpfe

/// N33: „+ Dateien…“ und „Ordner…“ (Knopfzeile und leere Ablagefläche) sowie Strg+O öffnen den
/// Datei-Dialog, und die gewählten Dateien landen in der Liste. Vorher holten die Knöpfe das
/// Studio aus `st.batch`, das während des Zeichnens herausgenommen ist – es passierte nichts.
#[test]
fn n33_dateien_und_ordner_knoepfe_fuellen_die_liste() {
    let d = dir("n33-knoepfe");
    let mut s0 = state(ThemeKind::Normal);
    s0.run("file.batchProcess", json!({}));
    let mut h = harness(s0);
    let a = png_file(&d, "a.png", 1, 40, 30);
    let b = png_file(&d, "b.png", 2, 40, 30);
    st(&mut h).file_override = Some(fp_ui::files::FilePickOverride { open: vec![a.clone(), b.clone()], save: None });
    for (label, lower) in [("batch-ui-files-add", false), ("batch-ui-files-add", true), ("batch-ui-files-add-folder", false), ("batch-ui-files-add-folder", true)] {
        studio(&mut h).clear_files();
        h.run_steps(3);
        let mut rects: Vec<Rect> = h.query_all_by_label(&t(label)).map(|n| n.rect()).collect();
        rects.sort_by(|x, y| x.top().total_cmp(&y.top()));
        assert!(rects.len() >= 2, "{label}: Knopf oben und in der leeren Fläche, gefunden {}", rects.len());
        // [0] Knopfzeile, [1] leere Ablagefläche (weiter unten ggf. gleichnamiger Knopf der Ausgabe)
        let r = rects[usize::from(lower)];
        click(&mut h, r.center());
        settle(&mut h);
        let n = studio(&mut h).files.len();
        assert_eq!(n, 2, "{label} (leere Fläche: {lower}): Liste gefüllt");
    }
    // Strg+O
    studio(&mut h).clear_files();
    h.run_steps(3);
    key(&mut h, Key::O, Modifiers::COMMAND);
    settle(&mut h);
    assert_eq!(studio(&mut h).files.len(), 2, "Strg+O");
}

/// BATCH-T: Ein Mausklick auf „▶ n Dateien verarbeiten“ startet den Auftrag. Vorher holte
/// `output_bar::start` das Studio aus `st.batch`, das während des Zeichnens herausgenommen ist –
/// der Knopf meldete „clicked“, aber nichts lief (nur Strg+Eingabe ging).
#[test]
fn batcht_knopf_verarbeiten_startet_den_lauf() {
    let d = dir("batcht-start");
    let out = d.join("out");
    std::fs::create_dir_all(&out).unwrap();
    let mut s0 = state(ThemeKind::Normal);
    s0.run("file.batchProcess", json!({}));
    let mut h = harness(s0);
    let a = png_file(&d, "a.png", 1, 40, 30);
    {
        let s = studio(&mut h);
        s.add_paths(&[a], false);
        s.edit("o", |r| {
            r.output.format = "png".into();
            r.output.target.kind = fp_batch::recipe::TargetKind::Folder;
            r.output.target.path = Some(out.clone());
            true
        });
    }
    h.run_steps(3);
    let label = fp_i18n::tr!("batch-ui-start", n = 1);
    let r = h.query_by_label(&label).expect("Knopf Verarbeiten").rect();
    click(&mut h, r.center());
    h.run_steps(1);
    assert!(studio(&mut h).running() || studio(&mut h).report.is_some(), "Klick startet den Lauf");
    for _ in 0..500 {
        h.run_steps(1);
        if studio(&mut h).report.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(studio(&mut h).report.is_some(), "Lauf fertig mit Bericht");
    assert!(out.join("a.png").exists(), "Ausgabe geschrieben");
    // „Bericht speichern…“ behält den gewählten Namen (vorher wurde `.fw.png` angehängt)
    let kopie = d.join("kopie.json");
    st(&mut h).file_override = Some(fp_ui::files::FilePickOverride { open: vec![], save: Some(kopie.clone()) });
    h.run_steps(3);
    let r = h.query_by_label(&t("batch-ui-done-save-report")).expect("Bericht speichern").rect();
    click(&mut h, r.center());
    settle(&mut h);
    assert!(kopie.exists(), "Bericht unter dem gewählten Namen");
    assert!(!d.join("kopie.json.fw.png").exists(), "kein .fw.png angehängt");
}

/// BATCH-T: Tab im Batch-Studio ist nur Fokusfolge – vorher blendete ein Tab ohne Fokus über das
/// Kürzel „Bedienfelder ausblenden“ die Werkzeugleiste des Hauptfensters aus.
#[test]
fn batcht_tab_blendet_nichts_aus() {
    let mut s0 = state(ThemeKind::Normal);
    s0.run("file.batchProcess", json!({}));
    let mut h = harness(s0);
    h.run_steps(3);
    assert!(batch::is_front(&h.state().state));
    assert!(!h.state().state.settings.workspace.panels_hidden);
    key(&mut h, Key::Tab, Modifiers::NONE);
    assert!(!h.state().state.settings.workspace.panels_hidden, "Tab blendet im Studio nichts aus");
}

/// BATCH-T: Die Auswahlfelder „Art“ und „Sortierung“ der Filterzeile bleiben in ihrem Platz – ein
/// langer Eintrag („FW-PNG/Dokumente“) schob sich vorher über „Hinzugefügt“, das selbst über den
/// Kartenrand ragte.
#[test]
fn batcht_filterzeile_ueberlappt_nicht() {
    let d = dir("batcht-filter");
    let mut s0 = state(ThemeKind::Normal);
    s0.run("file.batchProcess", json!({}));
    let mut h = harness(s0);
    let a = png_file(&d, "a.png", 1, 40, 30);
    studio(&mut h).filter.kind = Some(fp_batch::ItemKind::Document);
    studio(&mut h).add_paths(&[a], false);
    h.run_steps(3);
    let kind = h.query_all_by_role(egui::accesskit::Role::ComboBox).map(|n| n.rect()).collect::<Vec<_>>();
    let mut row: Vec<Rect> = kind.into_iter().filter(|r| r.top() < 400.0).collect();
    row.sort_by(|x, y| x.left().total_cmp(&y.left()));
    assert!(row.len() >= 2, "Art und Sortierung gefunden: {row:?}");
    assert!(row[0].right() <= row[1].left() + 0.5, "Art überlappt Sortierung: {row:?}");
    let clear = h.query_by_label(&t("batch-ui-files-clear")).expect("Leeren").rect();
    assert!(row[1].right() <= clear.right() + 0.5, "Sortierung ragt aus der Karte: {:?} (Leeren {clear:?})", row[1]);
}
