//! Agent DESIGN: Startbildschirm (`src\welcome.rs`) – Inhalt, Bedienung, „Nicht mehr anzeigen“,
//! Kürzen langer Namen, leerer Zustand und Layout in allen Größen, DPI-Stufen, Designs und Sprachen
//! (de, en, Testsprache qps). Bilder: `--ignored design_welcome_images`.

use egui::epaint::Shape;
use egui::{vec2, Event, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::Harness;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::theme::ThemeKind;
use std::path::PathBuf;
use std::sync::Mutex;

/// Sprache ist prozessweit: Tests nacheinander.
static LANG: Mutex<()> = Mutex::new(());

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

fn harness(st: AppState, w: f32, h: f32, ppp: f32) -> Harness<'static, W> {
    harness_ex(st, w, h, ppp, false)
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

fn find(h: &Harness<'_, W>, text: &str) -> Option<T> {
    texts(h).into_iter().find(|t| t.text == text)
}

fn click(h: &mut Harness<'_, W>, p: Pos2) {
    h.event(Event::PointerMoved(p));
    h.run_steps(1);
    h.event(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.run_steps(1);
    h.event(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(3);
}

fn lang(code: &str) {
    assert!(fp_i18n::set_language(code), "Sprache {code}");
}

fn tmp_file(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join("firepixel-design-welcome").join(std::process::id().to_string());
    let _ = std::fs::create_dir_all(&d);
    let p = d.join(name);
    let _ = std::fs::write(&p, b"x");
    p
}

#[test]
fn shows_all_areas_and_entries() {
    let _g = LANG.lock().unwrap_or_else(|e| e.into_inner());
    lang("de");
    let a = tmp_file("banner.fw.png");
    let b = tmp_file("foto.jpg");
    let h = harness(state(vec![a, b, PathBuf::from(r"Z:\gibt\es\nicht.fw.png")], ThemeKind::Normal), 1280.0, 860.0, 1.0);
    for s in ["Firepixel", "Zuletzt geöffnete Dateien", "Neu erstellen", "Firepixel-Dokument (FW-PNG)", "Aus Vorlage...", "Öffnen...", "Kurzanleitung", "Tastenkürzel", "KI-Modelle verwalten", "Nicht mehr anzeigen", "banner.fw.png", "foto.jpg", "nicht.fw.png"] {
        assert!(find(&h, s).is_some(), "„{s}“ fehlt; gezeichnet: {:?}", texts(&h).iter().map(|t| t.text.clone()).collect::<Vec<_>>());
    }
    // Erweitern nur mit vorhandenen Befehlen
    let has = h.state().st.session.registry().get("commands.manageSavedCommands").is_some();
    assert_eq!(find(&h, "Erweitern").is_some(), has);
}

#[test]
fn new_document_works() {
    let _g = LANG.lock().unwrap_or_else(|e| e.into_inner());
    lang("de");
    let mut h = harness(state(vec![], ThemeKind::Normal), 1280.0, 860.0, 1.0);
    let r = find(&h, "Firepixel-Dokument (FW-PNG)").expect("Eintrag");
    click(&mut h, r.rect.center());
    let ctx = h.ctx.clone();
    let st = &mut h.state_mut().st;
    st.process_actions(&ctx);
    assert!(st.session.active().is_some() || st.dialogs.new_document.is_some(), "Neu erstellen ohne Wirkung");
}

#[test]
fn recent_file_opens() {
    let _g = LANG.lock().unwrap_or_else(|e| e.into_inner());
    lang("de");
    let mut s = state(vec![], ThemeKind::Normal);
    let f = tmp_file("oeffnen.fw.png");
    let _ = s.session.execute("file.new", serde_json::json!({ "width": 40, "height": 30 }));
    let _ = s.session.execute("file.saveAs", serde_json::json!({ "path": f.to_string_lossy() }));
    let _ = s.session.execute("file.close", serde_json::json!({ "discard": true }));
    s.session.prefs.recent_files = vec![f.clone()];
    let mut h = harness(s, 1280.0, 860.0, 1.0);
    let r = find(&h, "oeffnen.fw.png").expect("Eintrag");
    click(&mut h, r.rect.center());
    let ctx = h.ctx.clone();
    let st = &mut h.state_mut().st;
    st.process_actions(&ctx);
    assert!(st.session.active().is_some(), "Datei nicht geöffnet");
}

#[test]
fn do_not_show_again_sets_preference() {
    let _g = LANG.lock().unwrap_or_else(|e| e.into_inner());
    lang("de");
    let mut h = harness(state(vec![], ThemeKind::Normal), 1280.0, 860.0, 1.0);
    assert!(h.state().st.session.prefs.get_bool("ShowStartPage", true));
    let r = find(&h, "Nicht mehr anzeigen").expect("Kästchen");
    click(&mut h, r.rect.center());
    assert!(!h.state().st.session.prefs.get_bool("ShowStartPage", true), "Voreinstellung nicht gesetzt");
    // in dieser Sitzung weiter sichtbar, Häkchen lässt sich zurücknehmen
    h.run_steps(2);
    let r = find(&h, "Nicht mehr anzeigen").expect("Kästchen bleibt sichtbar");
    click(&mut h, r.rect.center());
    assert!(h.state().st.session.prefs.get_bool("ShowStartPage", true));
    // neue Sitzung mit abgeschalteter Voreinstellung: nichts gezeichnet
    let mut s = state(vec![], ThemeKind::Normal);
    s.session.prefs.set("ShowStartPage", fp_engine::PrefValue::Bool(false));
    let h2 = harness(s, 1280.0, 860.0, 1.0);
    assert!(find(&h2, "Neu erstellen").is_none(), "abgeschaltet, aber sichtbar");
}

#[test]
fn empty_state_and_long_names() {
    let _g = LANG.lock().unwrap_or_else(|e| e.into_inner());
    lang("de");
    let h = harness(state(vec![], ThemeKind::Dark), 1280.0, 860.0, 1.0);
    assert!(find(&h, "Noch keine Dateien geöffnet").is_some());
    let long = tmp_file("ein_sehr_sehr_langer_dateiname_fuer_den_startbildschirm_mit_noch_mehr_text_2026_endfassung_wirklich.fw.png");
    let h = harness(state(vec![long], ThemeKind::Normal), 1024.0, 768.0, 1.0);
    let t = texts(&h).into_iter().find(|t| t.text.starts_with("ein_sehr")).expect("Name");
    assert!(t.text.ends_with('…'), "langer Name nicht gekürzt: {}", t.text);
}

/// Layout-Prüfung: kein Text abgeschnitten (ohne „…“), keine Überlappungen.
fn layout_findings(h: &Harness<'_, W>) -> Vec<String> {
    let screen = h.ctx.content_rect();
    let ts = texts(h);
    let mut out = Vec::new();
    for t in &ts {
        let vis = t.rect.intersect(t.clip).intersect(screen);
        if !vis.is_positive() {
            continue; // weggescrollt
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

#[test]
fn all_sizes_themes_and_languages_without_overflow() {
    let _g = LANG.lock().unwrap_or_else(|e| e.into_inner());
    let recent: Vec<PathBuf> = (0..8).map(|i| tmp_file(&format!("projekt_{i}_mit_einem_recht_langen_namen_fuer_die_liste.fw.png"))).collect();
    let mut all = Vec::new();
    for code in ["de", "en", fp_i18n::PSEUDO_LANG] {
        lang(code);
        for kind in ThemeKind::ALL {
            for (w, hh) in [(800.0, 600.0), (1024.0, 768.0), (1366.0, 768.0), (1920.0, 1080.0), (3840.0, 2160.0), (420.0, 900.0)] {
                for ppp in [1.0, 1.5, 2.0] {
                    let h = harness(state(recent.clone(), kind), w, hh, ppp);
                    for f in layout_findings(&h) {
                        all.push(format!("{code} {kind:?} {w}x{hh}@{ppp}: {f}"));
                    }
                    for t in texts(&h) {
                        if t.text.contains("welcome-") {
                            all.push(format!("{code}: Rohschlüssel {}", t.text));
                        }
                    }
                }
            }
        }
    }
    lang("de");
    assert!(all.is_empty(), "{} Befunde:\n{}", all.len(), all.join("\n"));
}

/// Bilder des neuen Startbildschirms (Normal | Dunkel | OLED) für `docs\screenshots`:
/// `design_after_welcome{,_en,_empty,_narrow,_150}.png` (technische Namen englisch, B-6).
#[test]
#[ignore]
fn design_welcome_images() {
    let _g = LANG.lock().unwrap_or_else(|e| e.into_inner());
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("docs").join("screenshots");
    let recent = vec![tmp_file("banner_herbst.fw.png"), tmp_file("produktfoto_2026.jpg"), tmp_file("icons_neu.fw.png"), tmp_file("aufraeumen.jsf")];
    type Case = (&'static str, &'static str, Vec<PathBuf>, f32, f32, f32);
    let cases: [Case; 5] = [
        ("de", "", recent.clone(), 1280.0, 860.0, 1.0),
        ("en", "_en", recent.clone(), 1280.0, 860.0, 1.0),
        ("de", "_empty", vec![], 1280.0, 860.0, 1.0),
        ("de", "_narrow", recent.clone(), 480.0, 760.0, 1.0),
        ("de", "_150", recent.clone(), 1600.0, 1200.0, 1.5),
    ];
    for (code, suffix, list, w, hh, ppp) in cases {
        lang(code);
        let mut parts: Vec<(u32, u32, Vec<u8>)> = Vec::new();
        for kind in ThemeKind::ALL {
            let mut h = harness_ex(state(list.clone(), kind), w, hh, ppp, true);
            h.run_steps(3);
            let Ok(img) = h.render() else {
                eprintln!("kein Grafikadapter");
                return;
            };
            parts.push((img.width(), img.height(), img.into_raw()));
        }
        let gap = 6u32;
        let tw: u32 = parts.iter().map(|p| p.0).sum::<u32>() + gap * 2;
        let th = parts.iter().map(|p| p.1).max().unwrap_or(1);
        let mut px = vec![128u8; (tw * th * 4) as usize];
        let mut x = 0;
        for (pw, ph, data) in &parts {
            for y in 0..*ph {
                let d = ((y * tw + x) * 4) as usize;
                let s = (y * pw * 4) as usize;
                px[d..d + (*pw * 4) as usize].copy_from_slice(&data[s..s + (*pw * 4) as usize]);
            }
            x += pw + gap;
        }
        let f = std::fs::File::create(dir.join(format!("design_after_welcome{suffix}.png"))).expect("Datei");
        let mut enc = png::Encoder::new(std::io::BufWriter::new(f), tw, th);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_compression(png::Compression::High);
        enc.write_header().and_then(|mut wr| wr.write_image_data(&px)).expect("PNG");
    }
    lang("de");
}
