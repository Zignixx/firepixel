//! Alltagsabläufe mit realistischen Eingaben (HG, AGENTEN_REGELN §4c „Lehre“).
//!
//! Die Ereignisse sind so gebaut, wie egui-winit sie unter Windows liefert:
//! * Strg+C/Strg+X → nur `Event::Copy`/`Event::Cut` (kein Tastenereignis beim Drücken), danach
//!   das Loslassen der Taste; Strg+V → `Event::Paste(text)` **nur**, wenn die Systemzwischenablage
//!   Text enthält – sonst gar kein Ereignis beim Drücken (nur das Loslassen),
//! * Mausrad: `MouseWheel { unit: Line, delta: ±1 }` je Rastung; Präzisions-Touchpad: viele kleine
//!   `Point`-Schritte; Strg hält die Modifikatoren im Ereignis,
//! * Ziehen über viele Bilder mit `PointerMoved`, Doppelklick als zwei Klicks kurz hintereinander,
//! * Tastenwiederholung als mehrere `Key { pressed: true }` ohne Loslassen.
//!
//! Geprüft wird, was ein Mensch erwartet (eine Rastung = eine Zoomstufe, Einfügen fügt ein …).
//! Ergebnis: `docs\funktionstest\alltag_auto.md`.

use crate::full_common;

use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::FirepixelApp;
use full_common::*;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness(st: AppState) -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, fp_engine::Session::default(), UiSettings::default(), false);
    app.state = st;
    let mut h = Harness::builder().with_size(egui::vec2(1600.0, 1000.0)).with_pixels_per_point(1.0).with_max_steps(20).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn ev(h: &mut H, e: Event) {
    // egui-winit hält den Modifikatorzustand in `RawInput.modifiers` (ModifiersChanged) – wie dort
    // gilt er für alle Ereignisse des Bildes
    let m = match &e {
        Event::Key { modifiers, .. } | Event::PointerButton { modifiers, .. } | Event::MouseWheel { modifiers, .. } => Some(*modifiers),
        _ => None,
    };
    if let Some(m) = m {
        h.event(Event::ModifiersChanged(m));
    }
    h.event(e);
    h.run_steps(1);
}

fn ctrl() -> Modifiers {
    Modifiers { ctrl: true, command: true, ..Default::default() }
}

fn key(h: &mut H, k: Key, m: Modifiers) {
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: m });
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: m });
    ev(h, Event::ModifiersChanged(Modifiers::NONE));
}

/// Strg+C / Strg+X wie egui-winit: nur das Befehlsereignis, dann das Loslassen der Taste.
fn copy_cut(h: &mut H, cut: bool) {
    ev(h, if cut { Event::Cut } else { Event::Copy });
    let k = if cut { Key::X } else { Key::C };
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: ctrl() });
}

/// Strg+V wie egui-winit: `Paste` nur mit Text in der Systemzwischenablage.
fn paste(h: &mut H, os_text: Option<&str>) {
    if let Some(t) = os_text.filter(|t| !t.is_empty()) {
        ev(h, Event::Paste(t.to_string()));
    }
    ev(h, Event::Key { key: Key::V, physical_key: Some(Key::V), pressed: false, repeat: false, modifiers: ctrl() });
}

/// Dokumentpunkt → Bildschirmpunkt (über den Sichtbereich der Leinwand).
fn screen(h: &H, x: f32, y: f32) -> Pos2 {
    let s = st(h);
    let d = s.session.active().expect("Dokument");
    let area = s.canvases.get(&d.id).and_then(|c| c.area).expect("Leinwand");
    let (sx, sy) = d.view.doc_to_screen(fp_geom::Point::new(x, y));
    Pos2::new(area.min.x + sx as f32, area.min.y + sy as f32)
}

fn press(h: &mut H, p: Pos2, down: bool, m: Modifiers) {
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: down, modifiers: m });
}

fn click(h: &mut H, p: Pos2, m: Modifiers) {
    ev(h, Event::PointerMoved(p));
    press(h, p, true, m);
    press(h, p, false, m);
    ev(h, Event::ModifiersChanged(Modifiers::NONE));
}

fn drag(h: &mut H, a: Pos2, b: Pos2, frames: usize, m: Modifiers) {
    ev(h, Event::PointerMoved(a));
    press(h, a, true, m);
    for i in 1..=frames {
        let t = i as f32 / frames as f32;
        ev(h, Event::PointerMoved(a + (b - a) * t));
    }
    press(h, b, false, m);
}

fn wheel(h: &mut H, at: Pos2, unit: egui::MouseWheelUnit, dy: f32, m: Modifiers) {
    ev(h, Event::PointerMoved(at));
    ev(h, Event::MouseWheel { unit, delta: egui::vec2(0.0, dy), phase: egui::TouchPhase::Move, modifiers: m });
    h.run_steps(3);
}

fn count_elements(s: &AppState) -> usize {
    let v = s.session.active().map(|d| serde_json::to_value(&*d.doc).unwrap_or_default()).unwrap_or_default();
    v.to_string().matches("\"common\"").count()
}

fn bounds(s: &AppState, id: u64) -> Option<fp_geom::Rect> {
    let d = s.session.active()?;
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(id))?;
    fp_engine::docops::element_bounds(&d.doc, &e)
}

fn zoom(s: &AppState) -> f64 {
    s.session.active().map(|d| d.view.zoom).unwrap_or(0.0)
}

/// Abgelegte Datei, wie egui-winit sie liefert.
#[derive(Debug)]
struct TestDrop(std::path::PathBuf);

impl egui::DroppedFile for TestDrop {
    fn path(&self) -> &std::path::Path {
        &self.0
    }
    fn bytes(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.0).map_err(|e| e.to_string())
    }
}

struct Case {
    name: &'static str,
    run: fn() -> Result<String, String>,
}

fn fresh() -> (H, Fix) {
    let (mut s, _ctx, f) = fixture();
    let _ = s.session.execute("view.setZoom", json!({ "zoom": 1.0 }));
    let _ = s.session.execute("select.set", json!({ "ids": [] }));
    s.os_clipboard = false;
    (harness(s), f)
}

fn cases() -> Vec<Case> {
    vec![
        Case {
            name: "Objekt mit dem Zeiger anklicken wählt es aus",
            run: || {
                let (mut h, f) = fresh();
                let p = screen(&h, 120.0, 100.0);
                click(&mut h, p, Modifiers::NONE);
                let sel = sel(st(&h));
                if sel == f.rect { Ok("Rechteck ausgewählt".into()) } else { Err(format!("Auswahl {sel:?}, erwartet {:?}", f.rect)) }
            },
        },
        Case {
            name: "Umschalt+Klick erweitert die Auswahl",
            run: || {
                let (mut h, f) = fresh();
                let a = screen(&h, 120.0, 100.0);
                let b = screen(&h, 320.0, 100.0);
                click(&mut h, a, Modifiers::NONE);
                click(&mut h, b, Modifiers::SHIFT);
                let s = sel(st(&h));
                if s.len() == 2 && s.contains(&f.rect[0]) && s.contains(&f.ellipse[0]) { Ok("2 Objekte".into()) } else { Err(format!("Auswahl {s:?}")) }
            },
        },
        Case {
            name: "Strg+C, Strg+V (Systemzwischenablage ohne Text) fügt eine Kopie ein",
            run: || {
                let (mut h, f) = fresh();
                let _ = h.state_mut().state.session.execute("select.set", json!({ "ids": f.rect }));
                h.run_steps(1);
                let n = count_elements(st(&h));
                copy_cut(&mut h, false);
                paste(&mut h, None);
                let m = count_elements(st(&h));
                if m == n + 1 { Ok("eingefügt".into()) } else { Err(format!("Elemente {n} → {m}")) }
            },
        },
        Case {
            name: "Strg+C, Strg+V (Systemzwischenablage mit fremdem Text) fügt das Objekt ein",
            run: || {
                let (mut h, f) = fresh();
                let _ = h.state_mut().state.session.execute("select.set", json!({ "ids": f.rect }));
                h.run_steps(1);
                let n = count_elements(st(&h));
                copy_cut(&mut h, false);
                paste(&mut h, Some("irgendein Text"));
                let m = count_elements(st(&h));
                if m == n + 1 { Ok("eingefügt".into()) } else { Err(format!("Elemente {n} → {m}")) }
            },
        },
        Case {
            name: "Strg+X entfernt, Strg+V fügt wieder ein",
            run: || {
                let (mut h, f) = fresh();
                let _ = h.state_mut().state.session.execute("select.set", json!({ "ids": f.rect }));
                h.run_steps(1);
                let n = count_elements(st(&h));
                copy_cut(&mut h, true);
                let after_cut = count_elements(st(&h));
                paste(&mut h, None);
                let m = count_elements(st(&h));
                if after_cut + 1 == n && m == n { Ok("ausgeschnitten und eingefügt".into()) } else { Err(format!("Elemente {n} → {after_cut} → {m}")) }
            },
        },
        Case {
            name: "Strg+Alt+D dupliziert (Strg+D hebt wie im Original die Auswahl auf)",
            run: || {
                let (mut h, f) = fresh();
                let _ = h.state_mut().state.session.execute("select.set", json!({ "ids": f.rect }));
                h.run_steps(1);
                let n = count_elements(st(&h));
                key(&mut h, Key::D, Modifiers { alt: true, ..ctrl() });
                let m = count_elements(st(&h));
                key(&mut h, Key::D, ctrl());
                let none = sel(st(&h)).is_empty();
                if m == n + 1 && none { Ok("dupliziert, Strg+D hebt Auswahl auf".into()) } else { Err(format!("Elemente {n} → {m}, Auswahl nach Strg+D leer: {none}")) }
            },
        },
        Case {
            name: "Pfeiltaste verschiebt um 1 px, Umschalt+Pfeil um 10 px, Tastenwiederholung 5× = 5 px",
            run: || {
                let (mut h, f) = fresh();
                let _ = h.state_mut().state.session.execute("select.set", json!({ "ids": f.rect }));
                h.run_steps(1);
                let x0 = bounds(st(&h), f.rect[0]).map(|b| b.left).unwrap_or(0.0);
                key(&mut h, Key::ArrowRight, Modifiers::NONE);
                let x1 = bounds(st(&h), f.rect[0]).map(|b| b.left).unwrap_or(0.0);
                key(&mut h, Key::ArrowRight, Modifiers::SHIFT);
                let x2 = bounds(st(&h), f.rect[0]).map(|b| b.left).unwrap_or(0.0);
                for i in 0..5 {
                    ev(&mut h, Event::Key { key: Key::ArrowRight, physical_key: Some(Key::ArrowRight), pressed: true, repeat: i > 0, modifiers: Modifiers::NONE });
                }
                ev(&mut h, Event::Key { key: Key::ArrowRight, physical_key: Some(Key::ArrowRight), pressed: false, repeat: false, modifiers: Modifiers::NONE });
                let x3 = bounds(st(&h), f.rect[0]).map(|b| b.left).unwrap_or(0.0);
                let ok = (x1 - x0 - 1.0).abs() < 0.01 && (x2 - x1 - 10.0).abs() < 0.01 && (x3 - x2 - 5.0).abs() < 0.01;
                let msg = format!("Δ {:.1} / {:.1} / {:.1}", x1 - x0, x2 - x1, x3 - x2);
                if ok { Ok(msg) } else { Err(msg) }
            },
        },
        Case {
            name: "Objekt mit der Maus ziehen (20 Bilder) verschiebt um die Mausstrecke",
            run: || {
                // Ohne Einrasten genau die Mausstrecke. Mit den intelligenten Hilfslinien (Vorgabe ein)
                // rastet die Oberkante 65 an der Kante 60 des gelben Rechtecks über der Bitmap ein
                // (FV-14: Kante an Kante, Fangabstand 5 px) – das ist richtig, kein Verlust.
                let mut msgs = Vec::new();
                for snap in [false, true] {
                    let (mut h, f) = fresh();
                    let on = st(&h).session.active().is_some_and(|d| d.view.snap_to_smart_guides);
                    if on != snap {
                        let _ = h.state_mut().state.session.execute("view.smartGuides.snapToSmartGuides", json!({}));
                        h.run_steps(1);
                    }
                    let b0 = bounds(st(&h), f.rect[0]).ok_or("keine Grenzen")?;
                    let a = screen(&h, 120.0, 100.0);
                    drag(&mut h, a, a + egui::vec2(40.0, 25.0), 20, Modifiers::NONE);
                    let b1 = bounds(st(&h), f.rect[0]).ok_or("keine Grenzen")?;
                    let z = zoom(st(&h)) as f32;
                    let (dx, dy) = (b1.left - b0.left, b1.top - b0.top);
                    let want_dy = if snap { 60.0 - b0.top } else { 25.0 / z };
                    let msg = format!("{} Δ ({dx:.1}, {dy:.1}), erwartet ({:.1}, {want_dy:.1})", if snap { "mit Einrasten" } else { "ohne Einrasten" }, 40.0 / z);
                    if (dx - 40.0 / z).abs() >= 1.5 || (dy - want_dy).abs() >= 1.5 {
                        return Err(msg);
                    }
                    msgs.push(msg);
                }
                Ok(msgs.join("; "))
            },
        },
        Case {
            name: "Strg+G gruppiert, Strg+Umschalt+G hebt auf",
            run: || {
                let (mut h, f) = fresh();
                let _ = h.state_mut().state.session.execute("select.set", json!({ "ids": f.two }));
                h.run_steps(1);
                let n = count_elements(st(&h));
                key(&mut h, Key::G, ctrl());
                let g = count_elements(st(&h));
                key(&mut h, Key::G, Modifiers { shift: true, ..ctrl() });
                let u = count_elements(st(&h));
                if g == n + 1 && u == n { Ok("gruppiert und aufgehoben".into()) } else { Err(format!("Elemente {n} → {g} → {u}")) }
            },
        },
        Case {
            name: "Text: T, Klick, „Hallo“ tippen, Esc → Textobjekt mit Hallo",
            run: || {
                let (mut h, _f) = fresh();
                key(&mut h, Key::T, Modifiers::NONE);
                let p = screen(&h, 500.0, 300.0);
                click(&mut h, p, Modifiers::NONE);
                for ch in ["H", "a", "l", "l", "o"] {
                    let k = Key::from_name(&ch.to_uppercase()).unwrap_or(Key::A);
                    ev(&mut h, Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: Modifiers::NONE });
                    ev(&mut h, Event::Text(ch.into()));
                    ev(&mut h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: Modifiers::NONE });
                }
                key(&mut h, Key::Escape, Modifiers::NONE);
                let v = st(&h).session.active().map(|d| serde_json::to_value(&*d.doc).unwrap_or_default()).unwrap_or_default().to_string();
                if v.contains("Hallo") { Ok("Text angelegt".into()) } else { Err("kein Text „Hallo“ im Dokument".into()) }
            },
        },
        Case {
            name: "Strg+Z macht den letzten Schritt rückgängig, Strg+Y wiederholt",
            run: || {
                let (mut h, f) = fresh();
                let _ = h.state_mut().state.session.execute("select.set", json!({ "ids": f.rect }));
                h.run_steps(1);
                let x0 = bounds(st(&h), f.rect[0]).map(|b| b.left).unwrap_or(0.0);
                key(&mut h, Key::ArrowRight, Modifiers::SHIFT);
                key(&mut h, Key::Z, ctrl());
                let x1 = bounds(st(&h), f.rect[0]).map(|b| b.left).unwrap_or(0.0);
                key(&mut h, Key::Y, ctrl());
                let x2 = bounds(st(&h), f.rect[0]).map(|b| b.left).unwrap_or(0.0);
                if (x1 - x0).abs() < 0.01 && (x2 - x0 - 10.0).abs() < 0.01 { Ok("rückgängig/wiederholt".into()) } else { Err(format!("{x0} → {x1} → {x2}")) }
            },
        },
        Case {
            name: "Strg+Mausrad: eine Rastung = eine Zoomstufe (100 % → 150 % bzw. 66 %)",
            run: || {
                let (mut h, _f) = fresh();
                let p = screen(&h, 300.0, 200.0);
                let z0 = zoom(st(&h));
                wheel(&mut h, p, egui::MouseWheelUnit::Line, 1.0, ctrl());
                let z1 = zoom(st(&h));
                wheel(&mut h, p, egui::MouseWheelUnit::Line, -1.0, ctrl());
                wheel(&mut h, p, egui::MouseWheelUnit::Line, -1.0, ctrl());
                let z2 = zoom(st(&h));
                let msg = format!("{:.0} % → {:.0} % → {:.0} %", z0 * 100.0, z1 * 100.0, z2 * 100.0);
                if (z1 - 1.5).abs() < 0.01 && (z2 - 0.66).abs() < 0.02 { Ok(msg) } else { Err(msg) }
            },
        },
        Case {
            name: "Strg+Touchpad (10 kleine Schritte à 12 Punkte) zoomt höchstens wenige Stufen",
            run: || {
                let (mut h, _f) = fresh();
                let p = screen(&h, 300.0, 200.0);
                for _ in 0..10 {
                    wheel(&mut h, p, egui::MouseWheelUnit::Point, 12.0, ctrl());
                }
                let z = zoom(st(&h));
                let msg = format!("100 % → {:.0} %", z * 100.0);
                if z > 1.0 && z <= 3.0 { Ok(msg) } else { Err(msg) }
            },
        },
        Case {
            name: "Mausrad ohne Strg rollt die Ansicht um ein Stück (kein Sprung)",
            run: || {
                let (mut h, _f) = fresh();
                let _ = h.state_mut().state.session.execute("view.setZoom", json!({ "zoom": 4.0 }));
                h.run_steps(3);
                // Zeiger in der Mitte der Leinwandfläche (nicht über Ansichtsleiste/Reitern)
                let p = {
                    let s = st(&h);
                    let d = s.session.active().ok_or("kein Dokument")?;
                    s.canvases.get(&d.id).and_then(|c| c.area).ok_or("keine Leinwand")?.center()
                };
                let o0 = st(&h).session.active().map(|d| d.view.origin).unwrap_or_default();
                wheel(&mut h, p, egui::MouseWheelUnit::Line, -1.0, Modifiers::NONE);
                // egui verteilt den Bildlauf über mehrere Bilder (Smooth Scroll)
                h.run_steps(15);
                let o1 = st(&h).session.active().map(|d| d.view.origin).unwrap_or_default();
                let dy = (o1.1 - o0.1) * 4.0;
                let msg = format!("Bildlauf {dy:.0} Bildschirmpunkte");
                if dy > 5.0 && dy < 200.0 { Ok(msg) } else { Err(msg) }
            },
        },
        Case {
            name: "Dialog (Leinwandgröße) an der Titelleiste verschiebbar, OK übernimmt",
            run: || {
                let (mut h, _f) = fresh();
                h.state_mut().state.run("modify.canvas.canvasSize", json!({}));
                h.run_steps(3);
                let id = fp_ui::dialogs::last_dialog_id(&h.ctx).ok_or("kein Dialog")?;
                let r0 = fp_ui::dialogs::dialog_rect(&h.ctx, &id).ok_or("kein Dialogrechteck")?;
                let grip = r0.min + egui::vec2(r0.width() * 0.4, 12.0);
                drag(&mut h, grip, grip + egui::vec2(-120.0, 60.0), 12, Modifiers::NONE);
                h.run_steps(2);
                let r1 = fp_ui::dialogs::dialog_rect(&h.ctx, &id).ok_or("Dialog weg")?;
                let moved = ((r1.min.x - r0.min.x) + 120.0).abs() < 3.0 && ((r1.min.y - r0.min.y) - 60.0).abs() < 3.0;
                key(&mut h, Key::Enter, Modifiers::NONE);
                let closed = !st(&h).dialogs.any_open();
                let msg = format!("verschoben um ({:.0}, {:.0}), mit Eingabe geschlossen: {closed}", r1.min.x - r0.min.x, r1.min.y - r0.min.y);
                if moved && closed { Ok(msg) } else { Err(msg) }
            },
        },
        Case {
            name: "Bilddatei auf die Leinwand ziehen importiert sie dort (linke obere Ecke), ein Verlaufsschritt, ausgewählt",
            run: || {
                let (mut h, f) = fresh();
                let png = f.save_to.parent().map(|d| d.join("drop.png")).ok_or("kein Ordner")?;
                fp_ui::os_clipboard::write_png(&png, 10, 8, &[90u8; 320])?;
                let docs = st(&h).session.documents().len();
                let undo = st(&h).session.active().map(|d| d.history.undo_count()).unwrap_or(0);
                let target = screen(&h, 300.0, 250.0);
                let start = screen(&h, 100.0, 100.0);
                ev(&mut h, Event::PointerMoved(start));
                h.input_mut().dropped_files.push(std::sync::Arc::new(TestDrop(png.clone())));
                h.run_steps(1);
                // erste Mausbewegung nach dem Ablegen liefert die Ablagestelle
                ev(&mut h, Event::PointerMoved(target));
                h.run_steps(2);
                let s = st(&h);
                let sel = sel(s);
                let b = sel.first().and_then(|id| bounds(s, *id));
                let steps = s.session.active().map(|d| d.history.undo_count()).unwrap_or(0) - undo;
                let msg = format!("Dokumente {docs} → {}, Schritte +{steps}, Grenzen {b:?}", s.session.documents().len());
                match b {
                    Some(b) if s.session.documents().len() == docs && steps == 1 && (b.left - 300.0).abs() < 1.5 && (b.top - 250.0).abs() < 1.5 => Ok(msg),
                    _ => Err(msg),
                }
            },
        },
        Case {
            name: "Datei außerhalb der Leinwand (Bedienfeld) ablegen öffnet sie als Dokument",
            run: || {
                let (mut h, f) = fresh();
                let png = f.save_to.parent().map(|d| d.join("drop2.png")).ok_or("kein Ordner")?;
                fp_ui::os_clipboard::write_png(&png, 10, 8, &[90u8; 320])?;
                let docs = st(&h).session.documents().len();
                h.input_mut().dropped_files.push(std::sync::Arc::new(TestDrop(png.clone())));
                h.run_steps(1);
                ev(&mut h, Event::PointerMoved(Pos2::new(1500.0, 500.0)));
                h.run_steps(2);
                let n = st(&h).session.documents().len();
                if n == docs + 1 { Ok("als Dokument geöffnet".into()) } else { Err(format!("Dokumente {docs} → {n}")) }
            },
        },
        Case {
            name: "Menü per Tastatur: Alt+D öffnet „Datei“, ↓ und Eingabe führen „Neu“ aus, Esc schließt",
            run: || {
                let (mut h, _f) = fresh();
                ev(&mut h, Event::ModifiersChanged(Modifiers::ALT));
                ev(&mut h, Event::Key { key: Key::D, physical_key: Some(Key::D), pressed: true, repeat: false, modifiers: Modifiers::ALT });
                ev(&mut h, Event::Key { key: Key::D, physical_key: Some(Key::D), pressed: false, repeat: false, modifiers: Modifiers::ALT });
                ev(&mut h, Event::ModifiersChanged(Modifiers::NONE));
                let k = fp_ui::menu_kbd::state(&h.ctx);
                if !(k.active && k.top == 0 && !k.path.is_empty()) {
                    return Err(format!("Alt+D: Zustand {k:?}"));
                }
                // „Neu“ ist der erste Eintrag → Eingabe
                key(&mut h, Key::Enter, Modifiers::NONE);
                h.run_steps(2);
                let opened = st(&h).dialogs.new_document.is_some();
                if !opened {
                    return Err("Eingabe auf „Neu“ öffnete den Dialog nicht".into());
                }
                Ok("Datei-Menü per Alt+D, „Neu“ per Eingabe".into())
            },
        },
        Case {
            name: "Menü per Tastatur: F10 aktiviert die Leiste, → wechselt, ↓ öffnet, Esc schließt stufenweise; Alt allein aktiviert",
            run: || {
                let (mut h, _f) = fresh();
                key(&mut h, Key::F10, Modifiers::NONE);
                let a = fp_ui::menu_kbd::state(&h.ctx);
                key(&mut h, Key::ArrowRight, Modifiers::NONE);
                let b = fp_ui::menu_kbd::state(&h.ctx);
                key(&mut h, Key::ArrowDown, Modifiers::NONE);
                let c = fp_ui::menu_kbd::state(&h.ctx);
                key(&mut h, Key::Escape, Modifiers::NONE);
                let d = fp_ui::menu_kbd::state(&h.ctx);
                key(&mut h, Key::Escape, Modifiers::NONE);
                let e = fp_ui::menu_kbd::state(&h.ctx);
                // Alt allein drücken und loslassen
                ev(&mut h, Event::ModifiersChanged(Modifiers::ALT));
                ev(&mut h, Event::ModifiersChanged(Modifiers::NONE));
                let f = fp_ui::menu_kbd::state(&h.ctx);
                let ok = a.active && a.top == 0 && a.path.is_empty() && b.top == 1 && !c.path.is_empty() && d.active && d.path.is_empty() && !e.active && f.active;
                let msg = format!("F10 {} · → Menü {} · ↓ offen {} · Esc {} · Esc aktiv {} · Alt allein aktiv {}", a.active, b.top, !c.path.is_empty(), d.path.is_empty(), e.active, f.active);
                if ok { Ok(msg) } else { Err(msg) }
            },
        },
        Case {
            name: "Strg+Tab / Strg+F6 wechseln zum nächsten, Strg+Umschalt+Tab zum vorigen Dokument",
            run: || {
                let (mut h, _f) = fresh();
                let _ = h.state_mut().state.session.execute("file.new", json!({ "width": 100, "height": 100 }));
                h.run_steps(2);
                let ids: Vec<u64> = st(&h).session.documents().iter().map(|d| d.id.0).collect();
                let a0 = st(&h).session.active_id().map(|d| d.0);
                key(&mut h, Key::Tab, ctrl());
                let a1 = st(&h).session.active_id().map(|d| d.0);
                key(&mut h, Key::F6, ctrl());
                let a2 = st(&h).session.active_id().map(|d| d.0);
                key(&mut h, Key::Tab, Modifiers { shift: true, ..ctrl() });
                let a3 = st(&h).session.active_id().map(|d| d.0);
                let msg = format!("Dokumente {ids:?}: {a0:?} → {a1:?} → {a2:?} → {a3:?}");
                if a1 != a0 && a2 == a0 && a3 == a1 { Ok(msg) } else { Err(msg) }
            },
        },
        Case {
            name: "Viele Dokumente: Reiter laufen nicht aus dem Bild, „»“ zeigt weitere, aktiver Reiter sichtbar",
            run: || {
                let (mut h, _f) = fresh();
                for i in 0..18 {
                    let _ = h.state_mut().state.session.execute("file.new", json!({ "width": 100 + i, "height": 100 }));
                }
                h.run_steps(3);
                let tabs: Vec<(fp_engine::DocId, egui::Rect)> = h.ctx.data(|d| d.get_temp(egui::Id::new("lw-doc-tabs"))).unwrap_or_default();
                let screen = h.ctx.content_rect();
                let active = st(&h).session.active_id();
                let inside = tabs.iter().all(|(_, r)| r.right() <= screen.right());
                let active_shown = tabs.iter().any(|(id, _)| Some(*id) == active);
                let more = h.ctx.viewport(|v| v.prev_pass.widgets.get(egui::Id::new("doc-tabs-more")).is_some());
                let msg = format!("{} von 19 Reitern sichtbar, im Fenster {inside}, aktiver sichtbar {active_shown}, „»“ vorhanden {more}", tabs.len());
                if inside && active_shown && more && tabs.len() < 19 { Ok(msg) } else { Err(msg) }
            },
        },
        Case {
            name: "Rechtsklick auf einen Dokumentreiter öffnet dessen Kontextmenü",
            run: || {
                let (mut h, _f) = fresh();
                let tabs: Vec<(fp_engine::DocId, egui::Rect)> = h.ctx.data(|d| d.get_temp(egui::Id::new("lw-doc-tabs"))).unwrap_or_default();
                let (_, r) = tabs.first().copied().ok_or("kein Reiter")?;
                let p = r.center();
                ev(&mut h, Event::PointerMoved(p));
                ev(&mut h, Event::PointerButton { pos: p, button: PointerButton::Secondary, pressed: true, modifiers: Modifiers::NONE });
                ev(&mut h, Event::PointerButton { pos: p, button: PointerButton::Secondary, pressed: false, modifiers: Modifiers::NONE });
                h.run_steps(2);
                let open = egui::Popup::is_any_open(&h.ctx);
                let labels: Vec<String> = fp_ui::canvas::tab_menu(fp_engine::DocId(1), 1)
                    .iter()
                    .filter_map(|n| match n {
                        fp_ui::menu::Node::Item { label, .. } => Some(label.clone()),
                        _ => None,
                    })
                    .collect();
                let want = ["Alles hier zusammenführen", "Schließen", "Alle schließen", "In neues Fenster verschieben", "Neues Dokument", "Dokument öffnen"];
                let msg = format!("Popup offen {open}, Einträge {labels:?}");
                if open && labels == want { Ok(msg) } else { Err(msg) }
            },
        },
        Case {
            name: "Fenstertitel: Anzeigename, dann der Dokumentname wie im Original („Firepixel Studio – *Name.fw.png“), Speichern/Wiederherstellen grau bei unverändertem Dokument, Wiederherstellen fragt nach",
            run: || {
                let (mut h, f) = fresh();
                key(&mut h, Key::S, ctrl());
                h.run_steps(2);
                let t0 = st(&h).window_title();
                let save_unchanged = fp_ui::actions::ui_command_state(st(&h), "file.save").map(|x| x.0);
                key(&mut h, Key::ArrowRight, Modifiers::NONE);
                let _ = h.state_mut().state.session.execute("select.set", json!({ "ids": f.rect }));
                key(&mut h, Key::ArrowRight, Modifiers::SHIFT);
                let t1 = st(&h).window_title();
                let save_changed = fp_ui::actions::ui_command_state(st(&h), "file.save").map(|x| x.0);
                h.state_mut().state.run("file.revert", json!({}));
                h.run_steps(2);
                let asks = st(&h).dialogs.confirm.as_ref().map(|c| c.text.clone()).unwrap_or_default();
                let msg = format!("Titel „{t0}“ → „{t1}“, Speichern unverändert {save_unchanged:?}, geändert {save_changed:?}, Rückfrage „{asks}“");
                if t0 == format!("{} – gespeichert.fw.png", fp_i18n::DISPLAY_NAME) && t1 == format!("{} – *gespeichert.fw.png", fp_i18n::DISPLAY_NAME) && save_unchanged == Some(false) && save_changed == Some(true) && asks.contains("wiederherstellen") { Ok(msg) } else { Err(msg) }
            },
        },
        Case {
            name: "Start: in [Seiten, Status, Ebenen] ist Ebenen vorn",
            run: || {
                let w = fp_ui::workspace::Workspace::expanded();
                let p = w.active_panel(&w.dock[1]);
                if p == Some(fp_ui::panels::PanelId::Layers) { Ok("Ebenen vorn".into()) } else { Err(format!("vorn: {p:?}")) }
            },
        },
        Case {
            name: "Fremden Text einfügen (Strg+V, Text in der Systemzwischenablage) legt ein Textobjekt 12 pt mittig im sichtbaren Teil an",
            run: || {
                let (mut h, _f) = fresh();
                h.state_mut().state.session.clipboard = Default::default();
                let _ = h.state_mut().state.session.execute("view.setZoom", json!({ "zoom": 2.0 }));
                h.run_steps(3);
                paste(&mut h, Some("Hallo aus dem Editor"));
                h.run_steps(2);
                let s = st(&h);
                let id = sel(s).first().copied().ok_or("nichts ausgewählt")?;
                let b = bounds(s, id).ok_or("keine Grenzen")?;
                let d = s.session.active().ok_or("kein Dokument")?;
                let (cx, cy) = (d.view.origin.0 + d.view.viewport.0 / d.view.zoom / 2.0, d.view.origin.1 + d.view.viewport.1 / d.view.zoom / 2.0);
                let json = serde_json::to_value(&*d.doc).unwrap_or_default().to_string();
                let msg = format!("Mitte Text ({:.0}, {:.0}), sichtbare Mitte ({cx:.0}, {cy:.0}), Text im Dokument {}", (b.left + b.right) / 2.0, (b.top + b.bottom) / 2.0, json.contains("Hallo aus dem Editor"));
                if json.contains("Hallo aus dem Editor") && (((b.left + b.right) / 2.0) as f64 - cx).abs() < 2.0 && (((b.top + b.bottom) / 2.0) as f64 - cy).abs() < 2.0 { Ok(msg) } else { Err(msg) }
            },
        },
        Case {
            name: "Doppelklick auf das Zoom-Werkzeug = 100 %, auf die Hand = alles einpassen",
            run: || {
                let (mut h, _f) = fresh();
                let _ = h.state_mut().state.session.execute("view.setZoom", json!({ "zoom": 3.0 }));
                h.run_steps(3);
                let rect_of = |h: &H, name: &str| h.ctx.viewport(|v| v.prev_pass.widgets.get(egui::Id::new(("tool-slot", name))).map(|w| w.rect));
                let zr = rect_of(&h, "Zoom").ok_or("Zoom-Knopf nicht gefunden")?;
                click(&mut h, zr.center(), Modifiers::NONE);
                click(&mut h, zr.center(), Modifiers::NONE);
                h.run_steps(2);
                let z1 = zoom(st(&h));
                let _ = h.state_mut().state.session.execute("view.setZoom", json!({ "zoom": 3.0 }));
                // Pause wie zwischen zwei getrennten Bedienungen (sonst zählt egui weiter Klicks)
                h.run_steps(40);
                let hr = rect_of(&h, "Hand").ok_or("Hand-Knopf nicht gefunden")?;
                click(&mut h, hr.center(), Modifiers::NONE);
                click(&mut h, hr.center(), Modifiers::NONE);
                h.run_steps(2);
                let z2 = zoom(st(&h));
                let msg = format!("300 % → Zoom-Doppelklick {:.0} %; 300 % → Hand-Doppelklick {:.0} % (eingepasst)", z1 * 100.0, z2 * 100.0);
                if (z1 - 1.0).abs() < 1e-6 && (z2 - 3.0).abs() > 1e-6 { Ok(msg) } else { Err(msg) }
            },
        },
        Case {
            name: "Strg+S speichert (Dateiname über den Dialog)",
            run: || {
                let (mut h, f) = fresh();
                key(&mut h, Key::S, ctrl());
                h.run_steps(2);
                if f.save_to.is_file() { Ok("gespeichert".into()) } else { Err(format!("{} nicht geschrieben", f.save_to.display())) }
            },
        },
        Case {
            name: "Doppelklick auf Text öffnet die Textbearbeitung",
            run: || {
                let (mut h, _f) = fresh();
                let p = screen(&h, 60.0, 322.0);
                click(&mut h, p, Modifiers::NONE);
                click(&mut h, p, Modifiers::NONE);
                h.run_steps(2);
                let editing = st(&h).session.tool_is_editing_text();
                if editing { Ok("Text-Editor aktiv".into()) } else { Err(format!("Werkzeug {:?}, kein Text-Editor", st(&h).session.tools.active)) }
            },
        },
    ]
}

#[test]
fn full_everyday() {
    let mut rows = Vec::new();
    for c in cases() {
        let r = std::panic::catch_unwind(c.run);
        let (o, note) = match r {
            Ok(Ok(n)) => (Outcome::Ui, n),
            Ok(Err(e)) => (Outcome::Error(e.clone()), e),
            Err(e) => (Outcome::Panic(panic_msg(&e)), String::new()),
        };
        rows.push(Row { path: c.name.to_string(), command: "realistische Eingabe".into(), outcome: o, note });
    }
    let (ok, n) = write_report("alltag_auto.md", "Alltagsabläufe mit realistischen Eingaben", "full_everyday.rs", &rows);
    eprintln!("Alltag: {ok}/{n} ✅");
}
