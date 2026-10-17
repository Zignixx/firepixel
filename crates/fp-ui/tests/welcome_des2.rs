//! DES-2 (Nutzerbefund N11, Teil Startbildschirm): neu gestalteter Startbildschirm
//! (`src\welcome.rs`) – drei Karten, Vorschaubilder der zuletzt geöffneten Dateien, Hover/Fokus,
//! Tastaturbedienung. Je Design und Sprache: alle Einträge vorhanden und auslösbar, nichts
//! abgeschnitten oder überlappend, keine Rohschlüssel. Bilder (`--ignored n11_start_bilder`):
//! `docs\screenshots\nutzerbefunde\N11_start_<design>_<sprache>.png`.

use egui::epaint::Shape;
use egui::{vec2, Event, Key, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::Harness;
use fp_i18n::t;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::theme::ThemeKind;
use std::path::{Path, PathBuf};

struct W {
    st: AppState,
    fonts: bool,
}

fn state(recent: Vec<PathBuf>, kind: ThemeKind) -> AppState {
    fp_engine::shell::disable();
    let session = fp_ui::ui_commands::new_session(fp_engine::Preferences::new());
    let mut st = AppState::new(session, UiSettings::default(), None);
    st.sync_render = true;
    st.os_clipboard = false;
    st.session.prefs.recent_files = recent;
    st.set_theme(kind);
    st
}

fn frame(ui: &mut egui::Ui, w: &mut W) {
    if !w.fonts {
        fp_ui::theme::install_fonts(ui.ctx());
        w.fonts = true;
        ui.ctx().request_repaint();
        return;
    }
    if w.st.theme_dirty {
        w.st.theme.apply(ui.ctx());
        w.st.theme_dirty = false;
    }
    let r = ui.max_rect();
    fp_ui::welcome::show(ui, &mut w.st, r);
}

fn harness_ex(st: AppState, w: f32, h: f32, ppp: f32, wgpu: bool) -> Harness<'static, W> {
    let b = Harness::builder().renderer(crate::gpu_common::renderer()).with_size(vec2(w / ppp, h / ppp)).with_pixels_per_point(ppp).with_max_steps(20);
    let b = if wgpu { b.renderer(crate::gpu_common::renderer()) } else { b };
    let mut hn = b.build_ui_state(frame, W { st, fonts: false });
    hn.run_steps(4);
    hn
}

fn harness(st: AppState, w: f32, h: f32) -> Harness<'static, W> {
    harness_ex(st, w, h, 1.0, false)
}

/// Wartet, bis die Vorschaubilder gelesen sind (Hintergrund-Threads).
fn settle_thumbs(h: &mut Harness<'_, W>) {
    for _ in 0..200 {
        h.run_steps(1);
        if !fp_ui::welcome::thumbnails_pending(&h.ctx) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(15));
    }
    assert!(!fp_ui::welcome::thumbnails_pending(&h.ctx), "Vorschaubilder nicht fertig");
    h.run_steps(2);
}

#[derive(Debug, Clone)]
struct T {
    text: String,
    rect: Rect,
    clip: Rect,
}

fn texts(h: &Harness<'_, W>) -> Vec<T> {
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

/// Anzahl der Flächen mit Textur (Vorschaubilder, Schachbrett, Logo – ohne Schrift).
fn textured_meshes(h: &Harness<'_, W>) -> usize {
    fn walk(s: &Shape, n: &mut usize) {
        match s {
            Shape::Mesh(m) if m.texture_id != egui::TextureId::default() => *n += 1,
            Shape::Rect(r) if r.fill_texture_id() != egui::TextureId::default() => *n += 1,
            Shape::Vec(v) => v.iter().for_each(|x| walk(x, n)),
            _ => {}
        }
    }
    let mut n = 0;
    for cs in &h.output().shapes {
        walk(&cs.shape, &mut n);
    }
    n
}

fn find(h: &Harness<'_, W>, text: &str) -> Option<T> {
    texts(h).into_iter().find(|t| t.text == text)
}

fn click(h: &mut Harness<'_, W>, p: Pos2) {
    h.event(Event::PointerMoved(p));
    h.run_steps(1);
    h.event(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.run_steps(1);
    h.event(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

fn key(h: &mut Harness<'_, W>, k: Key) {
    for pressed in [true, false] {
        h.event(Event::Key { key: k, physical_key: None, pressed, repeat: false, modifiers: Modifiers::NONE });
        h.run_steps(1);
    }
    h.run_steps(1);
}

fn lang(code: &str) {
    assert!(fp_i18n::set_language(code), "Sprache {code}");
}

fn dir() -> PathBuf {
    let d = std::env::temp_dir().join("firepixel-des2-welcome").join(std::process::id().to_string()).join("Projekte");
    let _ = std::fs::create_dir_all(&d);
    d
}

/// Kleine bunte PNG-Datei (für Vorschaubilder).
fn png_file(name: &str, seed: u32) -> PathBuf {
    let p = dir().join(name);
    let (w, h) = (96u32, 64u32);
    let mut px = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let fx = x as f32 / w as f32;
            let fy = y as f32 / h as f32;
            let (cx, cy) = (x as i32 - (30 + (seed as i32 * 17) % 40), y as i32 - 32);
            let disc = cx * cx + cy * cy < 18 * 18;
            let base = match seed % 3 {
                0 => [(255.0 * fx) as u8, (120.0 + 100.0 * fy) as u8, 220],
                1 => [250, (180.0 * (1.0 - fy)) as u8 + 40, (60.0 + 120.0 * fx) as u8],
                _ => [(40.0 + 80.0 * fy) as u8, (160.0 + 90.0 * fx) as u8, (120.0 + 60.0 * fy) as u8],
            };
            let c = if disc { [255, 255, 255 - (seed * 50 % 200) as u8] } else { base };
            px.extend_from_slice(&[c[0], c[1], c[2], 255]);
        }
    }
    let f = std::fs::File::create(&p).expect("Datei");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().and_then(|mut wr| wr.write_image_data(&px)).expect("PNG");
    p
}

fn text_file(name: &str) -> PathBuf {
    let p = dir().join(name);
    let _ = std::fs::write(&p, b"// Skript\n");
    p
}

fn sample_recent() -> Vec<PathBuf> {
    vec![
        png_file("plakat_sommerfest.fw.png", 0),
        png_file("logo_entwurf.png", 1),
        png_file("titelbild_web.png", 2),
        text_file("aufraeumen.jsf"),
        dir().join("alte_skizze_gibt_es_nicht_mehr.fw.png"),
    ]
}

/// Layout-Prüfung: kein Text abgeschnitten (ohne „…“), keine Überlappungen.
fn layout_findings(h: &Harness<'_, W>) -> Vec<String> {
    let screen = h.ctx.content_rect();
    let ts = texts(h);
    let mut out = Vec::new();
    for t in &ts {
        let vis = t.rect.intersect(t.clip).intersect(screen);
        if !vis.is_positive() {
            continue;
        }
        let ell = t.text.ends_with('…');
        if (t.rect.min.x < t.clip.min.x - 1.0 || t.rect.max.x > t.clip.max.x + 1.0) && !ell && t.rect.width() - vis.width() > 2.0 {
            out.push(format!("abgeschnitten: {} {:?} clip {:?}", t.text, t.rect, t.clip));
        }
    }
    let s = |r: Rect| Rect::from_min_max(r.min + vec2(1.0, 2.0), r.max - vec2(1.0, 2.0));
    for i in 0..ts.len() {
        for j in (i + 1)..ts.len() {
            let (a, b) = (&ts[i], &ts[j]);
            let x = s(a.rect.intersect(a.clip)).intersect(s(b.rect.intersect(b.clip)));
            if x.is_positive() && x.width() > 2.0 && x.height() > 3.0 {
                out.push(format!("Überlappung: „{}“ × „{}“", a.text, b.text));
            }
        }
    }
    out
}

/// Einträge, die auslösbar sein müssen: (Textschlüssel, erwarteter Befehl).
fn entries(st: &AppState) -> Vec<(&'static str, &'static str)> {
    let mut v = vec![
        ("welcome-new-document", "file.new"),
        ("welcome-new-template", "file.newFromTemplate"),
        ("welcome-open", "file.open"),
        ("welcome-help-guide", "help.firepixelHelp"),
        ("welcome-help-shortcuts", "edit.keyboardShortcuts"),
        ("welcome-help-models", "ai.manageModels"),
        ("welcome-help-automation", "help.firepixelHelp"),
    ];
    for (k, c) in [("welcome-extend-commands", "commands.manageSavedCommands"), ("welcome-extend-extensions", "commands.manageExtensions")] {
        if st.session.registry().get(c).is_some() {
            v.push((k, c));
        }
    }
    v
}

#[test]
fn cards_complete_in_all_themes_and_languages() {
    let recent = sample_recent();
    let mut all = Vec::new();
    for code in ["de", "en"] {
        lang(code);
        for kind in ThemeKind::ALL {
            for (w, hh, ppp) in [(1280.0, 860.0, 1.0), (1024.0, 768.0, 1.0), (1366.0, 768.0, 1.5), (700.0, 700.0, 1.0), (420.0, 900.0, 1.0)] {
                let h = harness_ex(state(recent.clone(), kind), w, hh, ppp, false);
                let tag = format!("{code} {kind:?} {w}x{hh}@{ppp}");
                let fits = ppp == 1.0 && w >= 1000.0;
                for key in ["welcome-new-title", "welcome-recent-title", "welcome-help-title", "welcome-new-document", "welcome-new-template", "welcome-open", "welcome-dont-show"] {
                    if key == "welcome-dont-show" && !fits {
                        continue; // unten, erst nach Bildlauf sichtbar
                    }
                    let s = t(key);
                    if !texts(&h).iter().any(|x| x.text == s || (x.text.ends_with('…') && s.starts_with(x.text.trim_end_matches('…')))) {
                        all.push(format!("{tag}: „{s}“ fehlt"));
                    }
                }
                for name in ["plakat_sommerfest.fw.png", "logo_entwurf.png", "aufraeumen.jsf"] {
                    if find(&h, name).is_none() && w > 600.0 {
                        all.push(format!("{tag}: Datei „{name}“ fehlt"));
                    }
                }
                for f in layout_findings(&h) {
                    all.push(format!("{tag}: {f}"));
                }
                for x in texts(&h) {
                    if x.text.contains("welcome-") {
                        all.push(format!("{tag}: Rohschlüssel {}", x.text));
                    }
                }
            }
        }
    }
    assert!(all.is_empty(), "{} Befunde:\n{}", all.len(), all.join("\n"));
}

#[test]
fn every_entry_triggers_in_all_themes_and_languages() {
    let recent = sample_recent();
    for code in ["de", "en"] {
        lang(code);
        for kind in ThemeKind::ALL {
            let mut h = harness(state(recent.clone(), kind), 1280.0, 900.0);
            let list = entries(&h.state().st);
            for (key, cmd) in list {
                let r = find(&h, &t(key)).unwrap_or_else(|| panic!("{code} {kind:?}: „{}“ nicht gezeichnet", t(key)));
                click(&mut h, r.rect.center());
                assert_eq!(fp_ui::welcome::last_triggered(&h.ctx).as_deref(), Some(cmd), "{code} {kind:?}: „{}“ löst nicht {cmd} aus", t(key));
            }
            // zuletzt geöffnete Datei: Klick auf Namen bzw. Ordnerzeile öffnet sie
            let r = find(&h, "logo_entwurf.png").expect("Eintrag");
            click(&mut h, r.rect.center() + vec2(0.0, 14.0));
            let want = format!("open:{}", recent[1].display());
            assert_eq!(fp_ui::welcome::last_triggered(&h.ctx), Some(want), "{code} {kind:?}: Datei nicht geöffnet");
        }
    }
    lang("de");
}

#[test]
fn new_document_really_runs() {
    lang("de");
    let mut h = harness(state(vec![], ThemeKind::Oled), 1280.0, 860.0);
    let r = find(&h, &t("welcome-new-document")).expect("Eintrag");
    click(&mut h, r.rect.center());
    let ctx = h.ctx.clone();
    let st = &mut h.state_mut().st;
    st.process_actions(&ctx);
    assert!(st.session.active().is_some() || st.dialogs.new_document.is_some(), "Neu erstellen ohne Wirkung");
}

#[test]
fn keyboard_tab_arrows_enter_space() {
    for code in ["de", "en"] {
        lang(code);
        for kind in ThemeKind::ALL {
            let mut h = harness(state(sample_recent(), kind), 1280.0, 860.0);
            assert_eq!(fp_ui::welcome::last_triggered(&h.ctx), None);
            // Tab fokussiert den ersten Eintrag (Neues Dokument), Eingabe löst ihn aus
            key(&mut h, Key::Tab);
            key(&mut h, Key::Enter);
            assert_eq!(fp_ui::welcome::last_triggered(&h.ctx).as_deref(), Some("file.new"), "{code} {kind:?}: Tab + Eingabe");
            // Pfeil nach unten: Aus Vorlage, Leertaste löst aus
            key(&mut h, Key::ArrowDown);
            key(&mut h, Key::Space);
            assert_eq!(fp_ui::welcome::last_triggered(&h.ctx).as_deref(), Some("file.newFromTemplate"), "{code} {kind:?}: Pfeil + Leertaste");
            // Pfeil nach rechts: in die Karte „Zuletzt geöffnet“
            key(&mut h, Key::ArrowRight);
            key(&mut h, Key::Enter);
            let last = fp_ui::welcome::last_triggered(&h.ctx).unwrap_or_default();
            assert!(last.starts_with("open:"), "{code} {kind:?}: Pfeil rechts landet nicht in der Dateiliste ({last})");
            // Umschalt+Tab zurück, nichts hängt
            h.event(Event::Key { key: Key::Tab, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::SHIFT });
            h.run_steps(2);
        }
    }
    lang("de");
}

#[test]
fn thumbnail_for_images_icon_otherwise() {
    lang("de");
    let real = png_file("vorschau_echt.png", 1);
    let fake = dir().join("vorschau_kaputt.png");
    let _ = std::fs::write(&fake, b"kein png");
    let mut a = harness(state(vec![real], ThemeKind::Normal), 1280.0, 860.0);
    settle_thumbs(&mut a);
    let mut b = harness(state(vec![fake], ThemeKind::Normal), 1280.0, 860.0);
    settle_thumbs(&mut b);
    let (na, nb) = (textured_meshes(&a), textured_meshes(&b));
    assert!(na >= nb + 2, "keine Vorschau gezeichnet ({na} vs. {nb} Texturflächen)");
    // Ordner gedämpft unter dem Namen
    let name = find(&a, "vorschau_echt.png").expect("Name");
    let folder = texts(&a).into_iter().find(|x| x.text.ends_with("Projekte")).expect("Ordnerzeile");
    assert!(folder.rect.top() > name.rect.bottom() - 1.0, "Ordner nicht unter dem Namen");
}

#[test]
fn empty_state_and_long_names() {
    for code in ["de", "en"] {
        lang(code);
        let h = harness(state(vec![], ThemeKind::Dark), 1280.0, 860.0);
        assert!(find(&h, &t("welcome-recent-empty")).is_some(), "{code}: leerer Zustand fehlt");
        let long = text_file("ein_sehr_sehr_langer_dateiname_fuer_den_startbildschirm_mit_noch_mehr_text_2026_endfassung_wirklich.jsf");
        let h = harness(state(vec![long], ThemeKind::Oled), 1024.0, 768.0);
        let x = texts(&h).into_iter().find(|x| x.text.starts_with("ein_sehr")).expect("Name");
        assert!(x.text.ends_with('…'), "langer Name nicht gekürzt: {}", x.text);
        assert!(layout_findings(&h).is_empty(), "{:?}", layout_findings(&h));
    }
    lang("de");
}

fn save_png(path: &Path, img: &image::RgbaImage) {
    let f = std::fs::File::create(path).expect("Datei");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), img.width(), img.height());
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.set_compression(png::Compression::High);
    enc.write_header().and_then(|mut wr| wr.write_image_data(img.as_raw())).expect("PNG");
}

/// Bilder für den Nutzerbefund N11: 3 Designs × de/en, 1280 × 860, mit Vorschaubildern.
#[test]
#[ignore]
fn n11_start_bilder() {
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("docs").join("screenshots").join("nutzerbefunde");
    let _ = std::fs::create_dir_all(&out);
    let recent = sample_recent();
    for code in ["de", "en"] {
        lang(code);
        for (kind, name) in [(ThemeKind::Normal, "normal"), (ThemeKind::Dark, "dunkel"), (ThemeKind::Oled, "oled")] {
            let mut h = harness_ex(state(recent.clone(), kind), 1280.0, 860.0, 1.0, true);
            settle_thumbs(&mut h);
            // Maus über „Logo-Entwurf“: Hover-Zustand sichtbar
            if let Some(r) = find(&h, "logo_entwurf.png") {
                h.event(Event::PointerMoved(r.rect.center()));
                h.run_steps(3);
            }
            for x in texts(&h) {
                assert!(fp_neutral::find_in_bytes(x.text.as_bytes()).is_empty(), "gesperrter Name im Bild: {}", x.text);
            }
            let Ok(img) = h.render() else {
                eprintln!("kein Grafikadapter");
                return;
            };
            let p = out.join(format!("N11_start_{name}_{code}.png"));
            save_png(&p, &img);
            let bytes = std::fs::read(&p).expect("PNG lesen");
            assert!(fp_neutral::find_in_bytes(&bytes).is_empty(), "gesperrter Name in {}", p.display());
        }
    }
    lang("de");
}
