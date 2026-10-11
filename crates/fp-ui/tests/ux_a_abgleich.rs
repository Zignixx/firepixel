//! UX-Abgleich A (Leinwand und Eingabe): misst das Verhalten von Firepixel mit realistischen
//! Eingaben (egui-winit-Ereignisse wie unter Windows) für die Szenarien in `docs\UX_ABGLEICH_A.md`.
//!
//! `cargo test -p fp-ui --test ux_a_abgleich -- --ignored --nocapture` gibt je Szenario eine
//! Zeile `UXA|<Nr>|<Beobachtung>` aus. Die Erwartung (Verhalten des Originals) steht im Katalog;
//! die Prüfungen mit `assert` stehen in `regressions` (laufen immer).

use egui::{Event, Key, Modifiers, MouseWheelUnit, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_engine::ToolId;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness_with(w: f64, h: f64) -> H {
    let mut session = fp_engine::Session::default();
    let _ = session.execute("file.new", json!({"width": w, "height": h, "background": "#ffffff"}));
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, session, UiSettings::default(), false);
    app.state.os_clipboard = false;
    let mut hh = Harness::builder()
        .with_size(egui::vec2(1600.0, 1000.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(20)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    hh.run_steps(4);
    hh
}

/// Testdokument wie im Original (`uxa_basis`): R1, R2, Gruppe G (GA, GB), Text T.
fn basis() -> H {
    let mut h = harness_with(600.0, 400.0);
    {
        let s = &mut h.state_mut().state.session;
        rect(s, "R1", [50, 50, 150, 120], "#ff0000");
        rect(s, "R2", [250, 60, 330, 140], "#0000ff");
        let a = rect(s, "GA", [400, 50, 450, 100], "#00aa00");
        let b = rect(s, "GB", [470, 50, 520, 100], "#00aa00");
        let _ = s.execute("select.set", json!({"ids": [a, b]}));
        let _ = s.execute("modify.group", json!({}));
        let _ = s.execute("element.setName", json!({"name": "G"}));
        let _ = s.execute("draw.text", json!({"text": "Hallo Welt", "anchor": [60, 270], "size": 24, "color": "#000000"}));
        let _ = s.execute("element.setName", json!({"name": "T"}));
        let _ = s.execute("select.set", json!({"ids": []}));
        let _ = s.execute("view.setZoom", json!({"zoom": 1.0}));
    }
    h.run_steps(3);
    h
}

fn rect(s: &mut fp_engine::Session, name: &str, r: [i32; 4], c: &str) -> u64 {
    let v = s.execute("draw.rectangle", json!({"rect": r, "fillColor": c, "strokeWidth": 0, "name": name})).expect("Rechteck");
    v.value.get("id").and_then(|x| x.as_u64()).unwrap_or(0)
}

/// Graue Bitmap 100×100 bei (200, 200) importieren (ausgewählt).
fn bitmap(h: &mut H) -> String {
    let dir = std::env::temp_dir().join("fp_uxa");
    let _ = std::fs::create_dir_all(&dir);
    let p = dir.join("grau.png");
    let img: Vec<u8> = [136u8, 136, 136, 255].repeat(100 * 100);
    let _ = fp_ui::os_clipboard::write_png(&p, 100, 100, &img);
    run(h, "file.import", json!({"path": p.to_string_lossy(), "at": [200, 200]}))
}

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

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn ev(h: &mut H, e: Event) {
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
fn shift() -> Modifiers {
    Modifiers { shift: true, ..Default::default() }
}
fn alt() -> Modifiers {
    Modifiers { alt: true, ..Default::default() }
}
fn plus(a: Modifiers, b: Modifiers) -> Modifiers {
    a | b
}

fn key(h: &mut H, k: Key, m: Modifiers) {
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: m });
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: m });
    ev(h, Event::ModifiersChanged(Modifiers::NONE));
}

fn key_down(h: &mut H, k: Key, m: Modifiers, repeat: bool) {
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: true, repeat, modifiers: m });
}
fn key_up(h: &mut H, k: Key, m: Modifiers) {
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: m });
}

fn text(h: &mut H, t: &str) {
    for c in t.chars() {
        ev(h, Event::Text(c.to_string()));
    }
}

fn screen(h: &H, x: f32, y: f32) -> Pos2 {
    let s = st(h);
    let d = s.session.active().expect("Dokument");
    let area = s.canvases.get(&d.id).and_then(|c| c.area).expect("Leinwand");
    let (sx, sy) = d.view.doc_to_screen(fp_geom::Point::new(x, y));
    Pos2::new(area.min.x + sx as f32, area.min.y + sy as f32)
}

fn area(h: &H) -> egui::Rect {
    let s = st(h);
    let d = s.session.active().expect("Dokument");
    s.canvases.get(&d.id).and_then(|c| c.area).expect("Leinwand")
}

fn press(h: &mut H, p: Pos2, b: PointerButton, down: bool, m: Modifiers) {
    ev(h, Event::PointerButton { pos: p, button: b, pressed: down, modifiers: m });
}

fn click(h: &mut H, p: Pos2, m: Modifiers) {
    ev(h, Event::PointerMoved(p));
    press(h, p, PointerButton::Primary, true, m);
    press(h, p, PointerButton::Primary, false, m);
    ev(h, Event::ModifiersChanged(Modifiers::NONE));
    h.run_steps(2);
}

fn dclick(h: &mut H, p: Pos2, m: Modifiers) {
    ev(h, Event::PointerMoved(p));
    press(h, p, PointerButton::Primary, true, m);
    press(h, p, PointerButton::Primary, false, m);
    press(h, p, PointerButton::Primary, true, m);
    press(h, p, PointerButton::Primary, false, m);
    ev(h, Event::ModifiersChanged(Modifiers::NONE));
    h.run_steps(2);
}

fn drag(h: &mut H, a: Pos2, b: Pos2, frames: usize, m: Modifiers) {
    ev(h, Event::PointerMoved(a));
    press(h, a, PointerButton::Primary, true, m);
    for i in 1..=frames {
        let t = i as f32 / frames as f32;
        ev(h, Event::PointerMoved(a + (b - a) * t));
    }
    press(h, b, PointerButton::Primary, false, m);
    ev(h, Event::ModifiersChanged(Modifiers::NONE));
    h.run_steps(2);
}

fn wheel(h: &mut H, at: Pos2, unit: MouseWheelUnit, dx: f32, dy: f32, m: Modifiers) {
    ev(h, Event::PointerMoved(at));
    ev(h, Event::MouseWheel { unit, delta: egui::vec2(dx, dy), phase: egui::TouchPhase::Move, modifiers: m });
    h.run_steps(3);
}

fn tool(h: &mut H, t: ToolId) {
    let _ = h.state_mut().state.session.set_tool(t);
    h.run_steps(1);
}

fn run(h: &mut H, c: &str, p: serde_json::Value) -> String {
    match h.state_mut().state.session.execute(c, p) {
        Ok(v) => v.value.to_string(),
        Err(e) => format!("FEHLER {e}"),
    }
}

/// Elemente der Seite (oben → unten) als „Name@l,t,r,b“.
fn elements(h: &H) -> String {
    let s = st(h);
    let d = s.session.active().expect("Dokument");
    let mut out = Vec::new();
    for (_, l) in fp_engine::docops::own_layers(&d.doc, d.page).into_iter().rev() {
        if let Some(c) = l.cels.get(d.frame) {
            for e in &c.elements {
                let n = e.common.name.clone().unwrap_or_else(|| "?".into());
                let b = fp_engine::docops::element_bounds(&d.doc, e).map(|r| format!("{},{},{},{}", r.left, r.top, r.right, r.bottom)).unwrap_or_default();
                out.push(format!("{n}@{b}"));
            }
        }
    }
    format!("{}: {}", out.len(), out.join(" | "))
}

fn selection(h: &H) -> String {
    let s = st(h);
    let d = s.session.active().expect("Dokument");
    let mut out = Vec::new();
    for id in &d.selection.elements {
        if let Some(e) = fp_engine::docops::element_on_page(&d.doc, d.page, *id) {
            let n = e.common.name.clone().unwrap_or_else(|| "?".into());
            let b = fp_engine::docops::element_bounds(&d.doc, &e).map(|r| format!("{},{},{},{}", r.left, r.top, r.right, r.bottom)).unwrap_or_default();
            out.push(format!("{n}@{b}"));
        } else {
            out.push(format!("#{}", id.0));
        }
    }
    format!("{}: {}", out.len(), out.join(" | "))
}

fn ids_named(h: &H, name: &str) -> Vec<u64> {
    let s = st(h);
    let d = s.session.active().expect("Dokument");
    let v = serde_json::to_value(&*d.doc).unwrap_or_default();
    let mut ids = Vec::new();
    fn walk(v: &serde_json::Value, name: &str, ids: &mut Vec<u64>) {
        match v {
            serde_json::Value::Object(m) => {
                if m.get("common").and_then(|c| c.get("name")).and_then(|n| n.as_str()) == Some(name)
                    && let Some(id) = m.get("id").and_then(|i| i.as_u64())
                {
                    ids.push(id);
                }
                for x in m.values() {
                    walk(x, name, ids);
                }
            }
            serde_json::Value::Array(a) => {
                for x in a {
                    walk(x, name, ids);
                }
            }
            _ => {}
        }
    }
    walk(&v, name, &mut ids);
    ids
}

fn select(h: &mut H, names: &[&str]) {
    let mut ids = Vec::new();
    for n in names {
        ids.extend(ids_named(h, n));
    }
    let _ = h.state_mut().state.session.execute("select.set", json!({"ids": ids}));
    h.run_steps(1);
}

fn history(h: &H) -> String {
    let d = st(h).session.active().expect("Dokument");
    let l = d.history.labels();
    format!("{} [{}]", d.history.undo_count(), l.join(", "))
}

fn view(h: &H) -> String {
    let d = st(h).session.active().expect("Dokument");
    format!("zoom={:.4} origin=({:.1},{:.1}) viewport=({:.0},{:.0})", d.view.zoom, d.view.origin.0, d.view.origin.1, d.view.viewport.0, d.view.viewport.1)
}

fn zoom(h: &H) -> f64 {
    st(h).session.active().map(|d| d.view.zoom).unwrap_or(0.0)
}

fn doc_at(h: &H, p: Pos2) -> (f32, f32) {
    let s = st(h);
    let d = s.session.active().expect("Dokument");
    let a = area(h);
    let q = d.view.screen_to_doc((p.x - a.left()) as f64, (p.y - a.top()) as f64);
    (q.x, q.y)
}

fn status(h: &H) -> String {
    st(h).status.as_ref().map(|s| s.0.clone()).unwrap_or_default()
}

fn active_tool(h: &H) -> String {
    format!("{:?}", st(h).session.tools.active)
}

fn cursor(h: &H) -> String {
    format!("{:?}", st(h).tool_cursor)
}

fn feedback(h: &H) -> String {
    let s = st(h);
    let d = s.session.active().expect("Dokument");
    let f = s.canvases.get(&d.id).map(|c| c.feedback.clone()).unwrap_or_default();
    let names: Vec<String> = f
        .iter()
        .map(|o| {
            let t = format!("{o:?}");
            t.split([' ', '(', '{']).next().unwrap_or("").to_string()
        })
        .collect();
    names.join(",")
}

type Case = (&'static str, fn() -> String);

fn cases() -> Vec<Case> {
    vec![
        // ---------------- Zoom und Bildlauf ----------------
        ("Z01 Strg+Rad 1 Rastung hoch", || {
            let mut h = basis();
            let p = screen(&h, 100.0, 85.0);
            let before = doc_at(&h, p);
            wheel(&mut h, p, MouseWheelUnit::Line, 0.0, 1.0, ctrl());
            let after = doc_at(&h, p);
            format!("{} | Punkt unter Maus vorher {:?} nachher {:?}", view(&h), before, after)
        }),
        ("Z02 Strg+Rad 1 Rastung runter", || {
            let mut h = basis();
            let p = screen(&h, 100.0, 85.0);
            wheel(&mut h, p, MouseWheelUnit::Line, 0.0, -1.0, ctrl());
            view(&h)
        }),
        ("Z03 Strg+Rad 5 Rastungen hoch", || {
            let mut h = basis();
            let p = screen(&h, 300.0, 200.0);
            let mut z = Vec::new();
            for _ in 0..5 {
                wheel(&mut h, p, MouseWheelUnit::Line, 0.0, 1.0, ctrl());
                z.push(format!("{:.2}", zoom(&h)));
            }
            z.join(" → ")
        }),
        ("Z04 Strg+Touchpad 12×10 Punkte", || {
            let mut h = basis();
            let p = screen(&h, 300.0, 200.0);
            for _ in 0..12 {
                wheel(&mut h, p, MouseWheelUnit::Point, 0.0, 10.0, ctrl());
            }
            view(&h)
        }),
        ("Z05 Rad ohne Taste (großes Dokument)", || {
            let mut h = harness_with(3000.0, 2000.0);
            let _ = run(&mut h, "view.setZoom", json!({"zoom": 1.0}));
            h.run_steps(2);
            let o = view(&h);
            let p = area(&h).center();
            wheel(&mut h, p, MouseWheelUnit::Line, 0.0, -1.0, Modifiers::NONE);
            format!("vorher {o} | nachher {}", view(&h))
        }),
        ("Z06 Umschalt+Rad (großes Dokument)", || {
            let mut h = harness_with(3000.0, 2000.0);
            let _ = run(&mut h, "view.setZoom", json!({"zoom": 1.0}));
            h.run_steps(2);
            let o = view(&h);
            let p = area(&h).center();
            wheel(&mut h, p, MouseWheelUnit::Line, 0.0, -1.0, shift());
            format!("vorher {o} | nachher {}", view(&h))
        }),
        ("Z07 Alt+Rad", || {
            let mut h = harness_with(3000.0, 2000.0);
            let _ = run(&mut h, "view.setZoom", json!({"zoom": 1.0}));
            h.run_steps(2);
            let o = view(&h);
            let p = area(&h).center();
            wheel(&mut h, p, MouseWheelUnit::Line, 0.0, 1.0, alt());
            format!("vorher {o} | nachher {}", view(&h))
        }),
        ("Z08 Strg++ und Strg+-", || {
            let mut h = basis();
            key(&mut h, Key::Plus, ctrl());
            let a = zoom(&h);
            key(&mut h, Key::Minus, ctrl());
            key(&mut h, Key::Minus, ctrl());
            format!("Strg++ → {a:.2}; 2× Strg+- → {:.2}", zoom(&h))
        }),
        ("Z09 Strg+0 Auswahl anpassen (R1 gewählt)", || {
            let mut h = basis();
            select(&mut h, &["R1"]);
            key(&mut h, Key::Num0, ctrl());
            view(&h)
        }),
        ("Z10 Strg+Alt+0 Alles anpassen", || {
            let mut h = basis();
            key(&mut h, Key::Num0, plus(ctrl(), alt()));
            view(&h)
        }),
        ("Z11 Strg+0 ohne Auswahl", || {
            let mut h = basis();
            key(&mut h, Key::Num0, ctrl());
            view(&h)
        }),
        ("Z12 Zoomwerkzeug Klick", || {
            let mut h = basis();
            tool(&mut h, ToolId::Zoom);
            let p = screen(&h, 100.0, 85.0);
            click(&mut h, p, Modifiers::NONE);
            format!("{} | Punkt unter Maus jetzt {:?}", view(&h), doc_at(&h, p))
        }),
        ("Z13 Zoomwerkzeug Alt+Klick", || {
            let mut h = basis();
            tool(&mut h, ToolId::Zoom);
            let p = screen(&h, 100.0, 85.0);
            click(&mut h, p, alt());
            view(&h)
        }),
        ("Z14 Zoomwerkzeug Rahmen ziehen 50,50→150,120", || {
            let mut h = basis();
            tool(&mut h, ToolId::Zoom);
            let (a, b) = (screen(&h, 50.0, 50.0), screen(&h, 150.0, 120.0));
            drag(&mut h, a, b, 10, Modifiers::NONE);
            view(&h)
        }),
        ("Z15 Leertaste+Ziehen (Zoom 400 %)", || {
            let mut h = basis();
            let _ = run(&mut h, "view.setZoom", json!({"zoom": 4.0}));
            h.run_steps(2);
            let o = view(&h);
            let c = area(&h).center();
            key_down(&mut h, Key::Space, Modifiers::NONE, false);
            let cur_space = format!("{:?}", st(&h).space_down);
            drag(&mut h, c, c + egui::vec2(100.0, 50.0), 10, Modifiers::NONE);
            key_up(&mut h, Key::Space, Modifiers::NONE);
            h.run_steps(2);
            format!("vorher {o} | nachher {} | space_down={cur_space} | Werkzeug {} | Elemente {}", view(&h), active_tool(&h), elements(&h))
        }),
        ("Z16 Leertaste während Rechteck-Aufziehen", || {
            let mut h = basis();
            tool(&mut h, ToolId::Rectangle);
            let a = screen(&h, 200.0, 200.0);
            let b = screen(&h, 260.0, 240.0);
            ev(&mut h, Event::PointerMoved(a));
            press(&mut h, a, PointerButton::Primary, true, Modifiers::NONE);
            for i in 1..=5 {
                ev(&mut h, Event::PointerMoved(a + (b - a) * (i as f32 / 5.0)));
            }
            key_down(&mut h, Key::Space, Modifiers::NONE, false);
            for i in 1..=5 {
                ev(&mut h, Event::PointerMoved(b + egui::vec2(10.0 * i as f32, 0.0)));
            }
            key_up(&mut h, Key::Space, Modifiers::NONE);
            let e = b + egui::vec2(50.0, 0.0);
            press(&mut h, e, PointerButton::Primary, false, Modifiers::NONE);
            h.run_steps(2);
            format!("{} | {}", view(&h), elements(&h))
        }),
        ("Z17 Hand-Werkzeug ziehen (400 %)", || {
            let mut h = basis();
            let _ = run(&mut h, "view.setZoom", json!({"zoom": 4.0}));
            tool(&mut h, ToolId::Hand);
            let o = view(&h);
            let c = area(&h).center();
            drag(&mut h, c, c + egui::vec2(100.0, 50.0), 10, Modifiers::NONE);
            format!("vorher {o} | nachher {}", view(&h))
        }),
        ("Z18 Mittlere Taste ziehen (400 %)", || {
            let mut h = basis();
            let _ = run(&mut h, "view.setZoom", json!({"zoom": 4.0}));
            h.run_steps(2);
            let o = view(&h);
            let c = area(&h).center();
            ev(&mut h, Event::PointerMoved(c));
            press(&mut h, c, PointerButton::Middle, true, Modifiers::NONE);
            for i in 1..=10 {
                ev(&mut h, Event::PointerMoved(c + egui::vec2(10.0 * i as f32, 5.0 * i as f32)));
            }
            press(&mut h, c + egui::vec2(100.0, 50.0), PointerButton::Middle, false, Modifiers::NONE);
            format!("vorher {o} | nachher {}", view(&h))
        }),
        ("Z19 Strg+Leertaste Klick (vorübergehend Zoom)", || {
            let mut h = basis();
            let p = screen(&h, 100.0, 85.0);
            key_down(&mut h, Key::Space, ctrl(), false);
            click(&mut h, p, ctrl());
            key_up(&mut h, Key::Space, ctrl());
            format!("{} | Werkzeug {} | Elemente {} | Auswahl {}", view(&h), active_tool(&h), elements(&h), selection(&h))
        }),
        ("Z20 Strg+Rad bei 6400 % weiter hoch", || {
            let mut h = basis();
            let _ = run(&mut h, "view.setZoom", json!({"zoom": 64.0}));
            let p = area(&h).center();
            wheel(&mut h, p, MouseWheelUnit::Line, 0.0, 1.0, ctrl());
            view(&h)
        }),
        ("Z21 Kleine Leinwand: Rad scrollt?", || {
            let mut h = basis();
            let o = view(&h);
            let p = area(&h).center();
            wheel(&mut h, p, MouseWheelUnit::Line, 0.0, -3.0, Modifiers::NONE);
            format!("vorher {o} | nachher {}", view(&h))
        }),
        ("Z22 Bildlaufgrenze: 100× Rad runter (3000×2000, 100 %)", || {
            let mut h = harness_with(3000.0, 2000.0);
            let _ = run(&mut h, "view.setZoom", json!({"zoom": 1.0}));
            let p = area(&h).center();
            for _ in 0..100 {
                wheel(&mut h, p, MouseWheelUnit::Line, 0.0, -1.0, Modifiers::NONE);
            }
            let a = area(&h);
            let d = st(&h).session.active().expect("d").view.clone();
            let bottom = d.doc_to_screen(fp_geom::Point::new(0.0, 2000.0)).1;
            format!("{} | Leinwand-Unterkante bei {:.0} px von {:.0} px Sichthöhe", view(&h), bottom, a.height())
        }),
        ("Z24 Kürzelzuordnung (aktiver Satz)", || {
            let h = basis();
            let s = &st(&h).session;
            let mut out = Vec::new();
            for (n, ctrl, alt, shift, vk) in [
                ("Strg+0", true, false, false, 0x30u16),
                ("Strg+Alt+0", true, true, false, 0x30),
                ("Strg+Umschalt+C", true, false, true, 0x43),
                ("Strg+Alt+D", true, true, false, 0x44),
                ("Strg+D", true, false, false, 0x44),
                ("Strg+Umschalt+V", true, false, true, 0x56),
                ("Strg+Alt+V", true, true, false, 0x56),
                ("Strg+Y", true, false, false, 0x59),
                ("Strg+Umschalt+Z", true, false, true, 0x5A),
                ("Strg+1", true, false, false, 0x31),
                ("Alt+Rechts", false, true, false, 0x27),
            ] {
                let sc = fp_engine::Shortcut { ctrl, alt, shift, vk };
                out.push(format!("{n}={:?}", s.command_for_shortcut(&sc)));
            }
            out.join(" ")
        }),
        ("Z25 Strg+0 mit Auswahl, Tastenereignis einzeln", || {
            let mut h = basis();
            select(&mut h, &["R1"]);
            let before = selection(&h);
            ev(&mut h, Event::Key { key: Key::Num0, physical_key: Some(Key::Num0), pressed: true, repeat: false, modifiers: ctrl() });
            let mid = format!("{} | Auswahl {}", view(&h), selection(&h));
            ev(&mut h, Event::Key { key: Key::Num0, physical_key: Some(Key::Num0), pressed: false, repeat: false, modifiers: ctrl() });
            format!("vorher Auswahl {before} | nach Drücken {mid} | ppp {}", h.ctx.pixels_per_point())
        }),
        // ---------------- Pfeiltasten ----------------
        ("N01 Pfeil rechts", || {
            let mut h = basis();
            select(&mut h, &["R1"]);
            key(&mut h, Key::ArrowRight, Modifiers::NONE);
            format!("{} | Verlauf {}", selection(&h), history(&h))
        }),
        ("N02 Umschalt+Pfeil unten", || {
            let mut h = basis();
            select(&mut h, &["R1"]);
            key(&mut h, Key::ArrowDown, shift());
            format!("{} | Verlauf {}", selection(&h), history(&h))
        }),
        ("N03 3× rechts + Umschalt+unten: Verlaufsschritte", || {
            let mut h = basis();
            select(&mut h, &["R1"]);
            let n0 = st(&h).session.active().expect("d").history.undo_count();
            for _ in 0..3 {
                key(&mut h, Key::ArrowRight, Modifiers::NONE);
            }
            key(&mut h, Key::ArrowDown, shift());
            let n1 = st(&h).session.active().expect("d").history.undo_count();
            format!("{} | neue Schritte {} | {}", selection(&h), n1 - n0, history(&h))
        }),
        ("N04 Tastenwiederholung 10× rechts", || {
            let mut h = basis();
            select(&mut h, &["R1"]);
            key_down(&mut h, Key::ArrowRight, Modifiers::NONE, false);
            for _ in 0..9 {
                key_down(&mut h, Key::ArrowRight, Modifiers::NONE, true);
            }
            key_up(&mut h, Key::ArrowRight, Modifiers::NONE);
            format!("{} | Verlauf {}", selection(&h), history(&h))
        }),
        ("N05 Alt+Pfeil rechts", || {
            let mut h = basis();
            select(&mut h, &["R1"]);
            key(&mut h, Key::ArrowRight, alt());
            format!("Auswahl {} | Elemente {} | Verlauf {}", selection(&h), elements(&h), history(&h))
        }),
        ("N06 Pfeil ohne Auswahl", || {
            let mut h = basis();
            key(&mut h, Key::ArrowRight, Modifiers::NONE);
            format!("{} | Status '{}' | Verlauf {}", elements(&h), status(&h), history(&h))
        }),
        ("N07 Strg+Pfeil rechts", || {
            let mut h = basis();
            select(&mut h, &["R1"]);
            key(&mut h, Key::ArrowRight, ctrl());
            format!("{} | Verlauf {}", selection(&h), history(&h))
        }),
        ("N08 Pfeil mit Rechteckwerkzeug aktiv", || {
            let mut h = basis();
            select(&mut h, &["R1"]);
            tool(&mut h, ToolId::Rectangle);
            key(&mut h, Key::ArrowRight, Modifiers::NONE);
            format!("{} | Verlauf {}", selection(&h), history(&h))
        }),
        ("N09 Pfeil bei Zoom 400 %", || {
            let mut h = basis();
            select(&mut h, &["R1"]);
            let _ = run(&mut h, "view.setZoom", json!({"zoom": 4.0}));
            key(&mut h, Key::ArrowRight, Modifiers::NONE);
            selection(&h)
        }),
        ("N10 Strg+Umschalt+Rechts (nächstes Objekt)", || {
            let mut h = basis();
            select(&mut h, &["R1"]);
            key(&mut h, Key::ArrowRight, plus(ctrl(), shift()));
            selection(&h)
        }),
        ("N11 Tab (Zeiger)", || {
            let mut h = basis();
            select(&mut h, &["R1"]);
            key(&mut h, Key::Tab, Modifiers::NONE);
            format!("{} | {}", selection(&h), elements(&h))
        }),
        // ---------------- Zeiger / Auswahl ----------------
        ("P01 Klick auf R1", || {
            let mut h = basis();
            tool(&mut h, ToolId::Pointer);
            let p = screen(&h, 100.0, 85.0);
            click(&mut h, p, Modifiers::NONE);
            format!("{} | Verlauf {}", selection(&h), history(&h))
        }),
        ("P02 Klick ins Leere hebt auf", || {
            let mut h = basis();
            select(&mut h, &["R1"]);
            tool(&mut h, ToolId::Pointer);
            let p = screen(&h, 200.0, 350.0);
            click(&mut h, p, Modifiers::NONE);
            format!("{} | Verlauf {}", selection(&h), history(&h))
        }),
        ("P03 Umschalt+Klick fügt hinzu", || {
            let mut h = basis();
            select(&mut h, &["R1"]);
            tool(&mut h, ToolId::Pointer);
            let p = screen(&h, 290.0, 100.0);
            click(&mut h, p, shift());
            selection(&h)
        }),
        ("P04 Umschalt+Klick auf Ausgewähltes entfernt", || {
            let mut h = basis();
            select(&mut h, &["R1", "R2"]);
            tool(&mut h, ToolId::Pointer);
            let p = screen(&h, 290.0, 100.0);
            click(&mut h, p, shift());
            selection(&h)
        }),
        ("P05 Gummiband teilweise über R1", || {
            let mut h = basis();
            tool(&mut h, ToolId::Pointer);
            let (a, b) = (screen(&h, 20.0, 20.0), screen(&h, 100.0, 100.0));
            drag(&mut h, a, b, 10, Modifiers::NONE);
            selection(&h)
        }),
        ("P06 Gummiband ganz um R1 und R2", || {
            let mut h = basis();
            tool(&mut h, ToolId::Pointer);
            let (a, b) = (screen(&h, 20.0, 20.0), screen(&h, 350.0, 160.0));
            drag(&mut h, a, b, 10, Modifiers::NONE);
            selection(&h)
        }),
        ("P07 Umschalt+Gummiband ergänzt", || {
            let mut h = basis();
            select(&mut h, &["T"]);
            tool(&mut h, ToolId::Pointer);
            let (a, b) = (screen(&h, 20.0, 20.0), screen(&h, 200.0, 160.0));
            drag(&mut h, a, b, 10, shift());
            selection(&h)
        }),
        ("P08 Klick auf Gruppenmitglied (Zeiger)", || {
            let mut h = basis();
            tool(&mut h, ToolId::Pointer);
            let p = screen(&h, 425.0, 75.0);
            click(&mut h, p, Modifiers::NONE);
            selection(&h)
        }),
        ("P09 Klick auf Gruppenmitglied (Teilauswahl)", || {
            let mut h = basis();
            tool(&mut h, ToolId::Subselection);
            let p = screen(&h, 425.0, 75.0);
            click(&mut h, p, Modifiers::NONE);
            selection(&h)
        }),
        ("P10 Doppelklick auf Gruppe (Zeiger)", || {
            let mut h = basis();
            tool(&mut h, ToolId::Pointer);
            let p = screen(&h, 425.0, 75.0);
            dclick(&mut h, p, Modifiers::NONE);
            format!("{} | Werkzeug {}", selection(&h), active_tool(&h))
        }),
        ("P11 Ziehen verschiebt (R1 +40,+20)", || {
            let mut h = basis();
            tool(&mut h, ToolId::Pointer);
            let (a, b) = (screen(&h, 100.0, 85.0), screen(&h, 140.0, 105.0));
            drag(&mut h, a, b, 10, Modifiers::NONE);
            format!("{} | Verlauf {}", selection(&h), history(&h))
        }),
        ("P12 Alt+Ziehen kopiert", || {
            let mut h = basis();
            tool(&mut h, ToolId::Pointer);
            let (a, b) = (screen(&h, 100.0, 85.0), screen(&h, 140.0, 105.0));
            drag(&mut h, a, b, 10, alt());
            format!("Auswahl {} | Elemente {} | Verlauf {}", selection(&h), elements(&h), history(&h))
        }),
        ("P13 Umschalt+Ziehen (schräg 40,10) rastet ein", || {
            let mut h = basis();
            tool(&mut h, ToolId::Pointer);
            let (a, b) = (screen(&h, 100.0, 85.0), screen(&h, 140.0, 95.0));
            drag(&mut h, a, b, 10, shift());
            selection(&h)
        }),
        ("P14 Ziehen 2 px (unter Schwelle)", || {
            let mut h = basis();
            tool(&mut h, ToolId::Pointer);
            let a = screen(&h, 100.0, 85.0);
            drag(&mut h, a, a + egui::vec2(2.0, 0.0), 2, Modifiers::NONE);
            format!("{} | Verlauf {}", selection(&h), history(&h))
        }),
        ("P15 Esc während Ziehen", || {
            let mut h = basis();
            tool(&mut h, ToolId::Pointer);
            let (a, b) = (screen(&h, 100.0, 85.0), screen(&h, 140.0, 105.0));
            ev(&mut h, Event::PointerMoved(a));
            press(&mut h, a, PointerButton::Primary, true, Modifiers::NONE);
            for i in 1..=5 {
                ev(&mut h, Event::PointerMoved(a + (b - a) * (i as f32 / 5.0)));
            }
            key(&mut h, Key::Escape, Modifiers::NONE);
            press(&mut h, b, PointerButton::Primary, false, Modifiers::NONE);
            h.run_steps(2);
            format!("{} | Verlauf {}", selection(&h), history(&h))
        }),
        ("P16 Überfahren zeigt Hervorhebung", || {
            let mut h = basis();
            tool(&mut h, ToolId::Pointer);
            let p = screen(&h, 100.0, 85.0);
            ev(&mut h, Event::PointerMoved(p));
            ev(&mut h, Event::PointerMoved(p + egui::vec2(1.0, 0.0)));
            format!("Rückmeldung [{}] Zeiger {}", feedback(&h), cursor(&h))
        }),
        ("P17 Zeiger über Griff", || {
            let mut h = basis();
            select(&mut h, &["R1"]);
            tool(&mut h, ToolId::Pointer);
            let p = screen(&h, 150.0, 120.0);
            ev(&mut h, Event::PointerMoved(p));
            ev(&mut h, Event::PointerMoved(p + egui::vec2(0.5, 0.0)));
            format!("Zeiger {}", cursor(&h))
        }),
        ("P18 Griff ziehen mit Umschalt (proportional)", || {
            let mut h = basis();
            select(&mut h, &["R2"]);
            tool(&mut h, ToolId::Pointer);
            let (a, b) = (screen(&h, 330.0, 140.0), screen(&h, 370.0, 150.0));
            drag(&mut h, a, b, 10, shift());
            selection(&h)
        }),
        ("P19 Griff ziehen mit Alt (von der Mitte)", || {
            let mut h = basis();
            select(&mut h, &["R2"]);
            tool(&mut h, ToolId::Pointer);
            let (a, b) = (screen(&h, 330.0, 140.0), screen(&h, 350.0, 160.0));
            drag(&mut h, a, b, 10, alt());
            selection(&h)
        }),
        ("P20 Hintere auswählen: 2× Klick auf Überlappung", || {
            let mut h = basis();
            {
                let s = &mut h.state_mut().state.session;
                rect(s, "OBEN", [80, 60, 200, 110], "#ffff00");
                let _ = s.execute("select.set", json!({"ids": []}));
            }
            tool(&mut h, ToolId::SelectBehind);
            let p = screen(&h, 100.0, 85.0);
            click(&mut h, p, Modifiers::NONE);
            let a = selection(&h);
            ev(&mut h, Event::PointerMoved(p + egui::vec2(0.0, 0.0)));
            h.run_steps(30);
            click(&mut h, p, Modifiers::NONE);
            format!("1. {a} | 2. {}", selection(&h))
        }),
        ("P21 Strg+A / Strg+D", || {
            let mut h = basis();
            key(&mut h, Key::A, ctrl());
            let a = selection(&h);
            key(&mut h, Key::D, ctrl());
            format!("Strg+A → {a} | Strg+D → {} | Verlauf {} | Elemente {}", selection(&h), history(&h), elements(&h))
        }),
        ("P22 Entf löscht Auswahl", || {
            let mut h = basis();
            select(&mut h, &["R1"]);
            key(&mut h, Key::Delete, Modifiers::NONE);
            format!("{} | Verlauf {}", elements(&h), history(&h))
        }),
        ("P23 Rücktaste löscht Auswahl", || {
            let mut h = basis();
            select(&mut h, &["R1"]);
            key(&mut h, Key::Backspace, Modifiers::NONE);
            format!("{} | Verlauf {}", elements(&h), history(&h))
        }),
        ("P24 Klick auf gesperrtes Objekt", || {
            let mut h = basis();
            select(&mut h, &["R1"]);
            let _ = run(&mut h, "element.setLocked", json!({"locked": true}));
            let _ = run(&mut h, "select.set", json!({"ids": []}));
            tool(&mut h, ToolId::Pointer);
            let p = screen(&h, 100.0, 85.0);
            click(&mut h, p, Modifiers::NONE);
            selection(&h)
        }),
        ("P25 Rechtsklick auf nicht gewähltes Objekt", || {
            let mut h = basis();
            tool(&mut h, ToolId::Pointer);
            let p = screen(&h, 100.0, 85.0);
            ev(&mut h, Event::PointerMoved(p));
            press(&mut h, p, PointerButton::Secondary, true, Modifiers::NONE);
            press(&mut h, p, PointerButton::Secondary, false, Modifiers::NONE);
            h.run_steps(2);
            let ctxm = st(&h).canvases.values().next().is_some_and(|c| c.context.is_some());
            format!("{} | Kontextmenü offen {ctxm}", selection(&h))
        }),
        // ---------------- Zwischenablage / Duplizieren ----------------
        ("C01 Kopieren+Einfügen gleiches Dokument", || {
            let mut h = basis();
            select(&mut h, &["R1"]);
            let _ = run(&mut h, "edit.copy", json!({}));
            let _ = run(&mut h, "select.set", json!({"ids": []}));
            let _ = run(&mut h, "edit.paste", json!({}));
            let a = selection(&h);
            let _ = run(&mut h, "edit.paste", json!({}));
            format!("1. {a} | 2. {} | Verlauf {}", selection(&h), history(&h))
        }),
        ("C02 Einfügen, Original nicht sichtbar (800 %, Mitte)", || {
            let mut h = basis();
            select(&mut h, &["R1"]);
            let _ = run(&mut h, "edit.copy", json!({}));
            let _ = run(&mut h, "view.setZoom", json!({"zoom": 8.0}));
            {
                let d = h.state_mut().state.session.active_mut().expect("d");
                let (vw, vh) = d.view.viewport;
                d.view.origin = (300.0 - vw / 16.0, 200.0 - vh / 16.0);
            }
            h.run_steps(2);
            let _ = run(&mut h, "edit.paste", json!({}));
            format!("{} | {}", selection(&h), view(&h))
        }),
        ("C03 Einfügen in kleineres Dokument, Objekt außerhalb", || {
            let mut h = basis();
            {
                let s = &mut h.state_mut().state.session;
                rect(s, "RF", [400, 300, 500, 380], "#00ff00");
                let _ = s.execute("edit.copy", json!({}));
                let _ = s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"}));
            }
            h.run_steps(4);
            let _ = run(&mut h, "edit.paste", json!({}));
            selection(&h)
        }),
        ("C04 Ausschneiden + Einfügen", || {
            let mut h = basis();
            select(&mut h, &["R1"]);
            let _ = run(&mut h, "edit.cut", json!({}));
            let a = elements(&h);
            let _ = run(&mut h, "edit.paste", json!({}));
            format!("nach Ausschneiden {a} | eingefügt {}", selection(&h))
        }),
        ("C05 Duplizieren (Strg+Alt+D) 2×", || {
            let mut h = basis();
            select(&mut h, &["R1"]);
            key(&mut h, Key::D, plus(ctrl(), alt()));
            let a = selection(&h);
            key(&mut h, Key::D, plus(ctrl(), alt()));
            format!("1. {a} | 2. {} | Verlauf {}", selection(&h), history(&h))
        }),
        ("C06 Klonen (Strg+Umschalt+C)", || {
            let mut h = basis();
            select(&mut h, &["R1"]);
            key(&mut h, Key::C, plus(ctrl(), shift()));
            format!("{} | {} | Verlauf {}", selection(&h), elements(&h), history(&h))
        }),
        ("C07 Innen einfügen (Strg+Umschalt+V)", || {
            let mut h = basis();
            select(&mut h, &["T"]);
            let _ = run(&mut h, "edit.copy", json!({}));
            select(&mut h, &["R1"]);
            let r = run(&mut h, "edit.pasteInside", json!({}));
            format!("{r} | {} | {}", selection(&h), elements(&h))
        }),
        ("C08 Fremder Text einfügen (Event::Paste)", || {
            let mut h = basis();
            ev(&mut h, Event::Paste("Hallo aus Notepad".into()));
            ev(&mut h, Event::Key { key: Key::V, physical_key: Some(Key::V), pressed: false, repeat: false, modifiers: ctrl() });
            h.run_steps(2);
            format!("{} | Status '{}'", selection(&h), status(&h))
        }),
        ("C09 Text bearbeiten: Strg+V fügt Text ein", || {
            let mut h = basis();
            tool(&mut h, ToolId::Text);
            let p = screen(&h, 300.0, 330.0);
            click(&mut h, p, Modifiers::NONE);
            text(&mut h, "AB");
            ev(&mut h, Event::Paste("xy".into()));
            key(&mut h, Key::Escape, Modifiers::NONE);
            format!("{} | Werkzeug {}", elements(&h), active_tool(&h))
        }),
        ("C10 Attribute einfügen", || {
            let mut h = basis();
            select(&mut h, &["R2"]);
            let _ = run(&mut h, "edit.copy", json!({}));
            select(&mut h, &["R1"]);
            key(&mut h, Key::V, plus(plus(ctrl(), alt()), shift()));
            format!("{} | Verlauf {}", selection(&h), history(&h))
        }),
        ("C11 Einfügen: Ziel-Ebene", || {
            let mut h = basis();
            select(&mut h, &["R1"]);
            let _ = run(&mut h, "edit.copy", json!({}));
            let _ = run(&mut h, "layer.add", json!({"name": "Neu"}));
            let _ = run(&mut h, "edit.paste", json!({}));
            let d = st(&h).session.active().expect("d");
            let layers: Vec<String> = fp_engine::docops::own_layers(&d.doc, d.page).iter().map(|(_, l)| format!("{}({})", l.name, l.cels.first().map(|c| c.elements.len()).unwrap_or(0))).collect();
            layers.join(", ")
        }),
        // ---------------- Text ----------------
        ("T01 Textwerkzeug Klick + Tippen + Esc", || {
            let mut h = basis();
            tool(&mut h, ToolId::Text);
            let p = screen(&h, 300.0, 330.0);
            click(&mut h, p, Modifiers::NONE);
            text(&mut h, "Neu");
            key(&mut h, Key::Escape, Modifiers::NONE);
            format!("{} | Werkzeug {} | Verlauf {} | Auswahl {}", elements(&h), active_tool(&h), history(&h), selection(&h))
        }),
        ("T02 Textwerkzeug Klick + Esc ohne Text", || {
            let mut h = basis();
            tool(&mut h, ToolId::Text);
            let p = screen(&h, 300.0, 330.0);
            click(&mut h, p, Modifiers::NONE);
            key(&mut h, Key::Escape, Modifiers::NONE);
            format!("{} | Werkzeug {} | Verlauf {}", elements(&h), active_tool(&h), history(&h))
        }),
        ("T03 Textrahmen ziehen 300,320→450,360", || {
            let mut h = basis();
            tool(&mut h, ToolId::Text);
            let (a, b) = (screen(&h, 300.0, 320.0), screen(&h, 450.0, 360.0));
            drag(&mut h, a, b, 10, Modifiers::NONE);
            text(&mut h, "Rahmen");
            key(&mut h, Key::Escape, Modifiers::NONE);
            format!("{}", elements(&h))
        }),
        ("T04 Doppelklick auf Text (Zeiger) → Bearbeiten", || {
            let mut h = basis();
            tool(&mut h, ToolId::Pointer);
            let p = screen(&h, 70.0, 262.0);
            dclick(&mut h, p, Modifiers::NONE);
            format!("Werkzeug {} | bearbeitet {} | Rückmeldung [{}]", active_tool(&h), st(&h).session.tool_is_editing_text(), feedback(&h))
        }),
        ("T05 Text: Enter = neue Zeile", || {
            let mut h = basis();
            tool(&mut h, ToolId::Text);
            let p = screen(&h, 300.0, 330.0);
            click(&mut h, p, Modifiers::NONE);
            text(&mut h, "A");
            key(&mut h, Key::Enter, Modifiers::NONE);
            text(&mut h, "B");
            let editing = st(&h).session.tool_is_editing_text();
            key(&mut h, Key::Escape, Modifiers::NONE);
            format!("noch im Editor nach Enter: {editing} | {}", elements(&h))
        }),
        ("T06 Text: Rücktaste am Anfang, Entf am Ende", || {
            let mut h = basis();
            tool(&mut h, ToolId::Text);
            let p = screen(&h, 300.0, 330.0);
            click(&mut h, p, Modifiers::NONE);
            text(&mut h, "AB");
            key(&mut h, Key::Delete, Modifiers::NONE);
            key(&mut h, Key::Home, Modifiers::NONE);
            key(&mut h, Key::Backspace, Modifiers::NONE);
            key(&mut h, Key::Escape, Modifiers::NONE);
            elements(&h)
        }),
        ("T07 Text: Strg+Rücktaste löscht Wort", || {
            let mut h = basis();
            tool(&mut h, ToolId::Text);
            let p = screen(&h, 300.0, 330.0);
            click(&mut h, p, Modifiers::NONE);
            text(&mut h, "Eins Zwei");
            key(&mut h, Key::Backspace, ctrl());
            key(&mut h, Key::Escape, Modifiers::NONE);
            elements(&h)
        }),
        ("T08 Text: Strg+Links springt wortweise, Umschalt markiert", || {
            let mut h = basis();
            tool(&mut h, ToolId::Text);
            let p = screen(&h, 300.0, 330.0);
            click(&mut h, p, Modifiers::NONE);
            text(&mut h, "Eins Zwei");
            key(&mut h, Key::ArrowLeft, plus(ctrl(), shift()));
            text(&mut h, "X");
            key(&mut h, Key::Escape, Modifiers::NONE);
            elements(&h)
        }),
        ("T09 Text: Tippen = 1 Verlaufsschritt", || {
            let mut h = basis();
            let n0 = st(&h).session.active().expect("d").history.undo_count();
            tool(&mut h, ToolId::Text);
            let p = screen(&h, 300.0, 330.0);
            click(&mut h, p, Modifiers::NONE);
            text(&mut h, "Hallo");
            key(&mut h, Key::Escape, Modifiers::NONE);
            let n1 = st(&h).session.active().expect("d").history.undo_count();
            format!("neue Schritte {} | {}", n1 - n0, history(&h))
        }),
        ("T10 Text: Klick außerhalb beendet", || {
            let mut h = basis();
            tool(&mut h, ToolId::Text);
            let p = screen(&h, 300.0, 330.0);
            click(&mut h, p, Modifiers::NONE);
            text(&mut h, "A");
            let q = screen(&h, 500.0, 200.0);
            click(&mut h, q, Modifiers::NONE);
            format!("{} | bearbeitet {} | Werkzeug {}", elements(&h), st(&h).session.tool_is_editing_text(), active_tool(&h))
        }),
        ("T11 Text: Doppelklick markiert Wort", || {
            let mut h = basis();
            tool(&mut h, ToolId::Pointer);
            let p = screen(&h, 70.0, 262.0);
            dclick(&mut h, p, Modifiers::NONE);
            h.run_steps(30);
            dclick(&mut h, p, Modifiers::NONE);
            text(&mut h, "Z");
            key(&mut h, Key::Escape, Modifiers::NONE);
            elements(&h)
        }),
        // ---------------- Werkzeuge mit Modifikatoren ----------------
        ("W01 Rechteck Umschalt (Quadrat)", || {
            let mut h = basis();
            tool(&mut h, ToolId::Rectangle);
            let (a, b) = (screen(&h, 200.0, 200.0), screen(&h, 260.0, 230.0));
            drag(&mut h, a, b, 10, shift());
            selection(&h)
        }),
        ("W02 Rechteck Alt (von Mitte)", || {
            let mut h = basis();
            tool(&mut h, ToolId::Rectangle);
            let (a, b) = (screen(&h, 200.0, 200.0), screen(&h, 260.0, 230.0));
            drag(&mut h, a, b, 10, alt());
            selection(&h)
        }),
        ("W03 Rechteck: Pfeil hoch während Ziehen (Rundung)", || {
            let mut h = basis();
            tool(&mut h, ToolId::Rectangle);
            let (a, b) = (screen(&h, 200.0, 200.0), screen(&h, 280.0, 260.0));
            ev(&mut h, Event::PointerMoved(a));
            press(&mut h, a, PointerButton::Primary, true, Modifiers::NONE);
            for i in 1..=5 {
                ev(&mut h, Event::PointerMoved(a + (b - a) * (i as f32 / 5.0)));
            }
            for _ in 0..5 {
                key(&mut h, Key::ArrowUp, Modifiers::NONE);
            }
            press(&mut h, b, PointerButton::Primary, false, Modifiers::NONE);
            h.run_steps(2);
            let d = st(&h).session.active().expect("d");
            let r = d.selection.elements.first().and_then(|id| fp_engine::docops::element_on_page(&d.doc, d.page, *id)).map(|e| format!("{:?}", e.kind).chars().filter(|c| !c.is_whitespace()).take(200).collect::<String>());
            format!("{} | {:?}", selection(&h), r)
        }),
        ("W04 Rechteck: Klick ohne Ziehen", || {
            let mut h = basis();
            tool(&mut h, ToolId::Rectangle);
            let p = screen(&h, 200.0, 200.0);
            click(&mut h, p, Modifiers::NONE);
            format!("{} | Werkzeug {}", elements(&h), active_tool(&h))
        }),
        ("W05 Ellipse Umschalt+Alt", || {
            let mut h = basis();
            tool(&mut h, ToolId::Ellipse);
            let (a, b) = (screen(&h, 200.0, 200.0), screen(&h, 260.0, 230.0));
            drag(&mut h, a, b, 10, plus(shift(), alt()));
            selection(&h)
        }),
        ("W06 Linie Umschalt (45°)", || {
            let mut h = basis();
            tool(&mut h, ToolId::Line);
            let (a, b) = (screen(&h, 200.0, 200.0), screen(&h, 260.0, 215.0));
            drag(&mut h, a, b, 10, shift());
            selection(&h)
        }),
        ("W07 Rechteck: Esc während Ziehen", || {
            let mut h = basis();
            tool(&mut h, ToolId::Rectangle);
            let (a, b) = (screen(&h, 200.0, 200.0), screen(&h, 260.0, 230.0));
            ev(&mut h, Event::PointerMoved(a));
            press(&mut h, a, PointerButton::Primary, true, Modifiers::NONE);
            for i in 1..=5 {
                ev(&mut h, Event::PointerMoved(a + (b - a) * (i as f32 / 5.0)));
            }
            key(&mut h, Key::Escape, Modifiers::NONE);
            press(&mut h, b, PointerButton::Primary, false, Modifiers::NONE);
            h.run_steps(2);
            elements(&h)
        }),
        ("W08 Rechteck: Umschalt erst während Ziehen", || {
            let mut h = basis();
            tool(&mut h, ToolId::Rectangle);
            let (a, b) = (screen(&h, 200.0, 200.0), screen(&h, 260.0, 230.0));
            ev(&mut h, Event::PointerMoved(a));
            press(&mut h, a, PointerButton::Primary, true, Modifiers::NONE);
            for i in 1..=5 {
                ev(&mut h, Event::PointerMoved(a + (b - a) * (i as f32 / 5.0)));
            }
            ev(&mut h, Event::ModifiersChanged(shift()));
            ev(&mut h, Event::PointerMoved(b + egui::vec2(0.5, 0.0)));
            press(&mut h, b, PointerButton::Primary, false, shift());
            h.run_steps(2);
            selection(&h)
        }),
        ("W09 Stift: Klick, Klick, Doppelklick", || {
            let mut h = basis();
            tool(&mut h, ToolId::Pen);
            let (a, b, c) = (screen(&h, 200.0, 200.0), screen(&h, 260.0, 200.0), screen(&h, 260.0, 260.0));
            click(&mut h, a, Modifiers::NONE);
            h.run_steps(30);
            click(&mut h, b, Modifiers::NONE);
            h.run_steps(30);
            dclick(&mut h, c, Modifiers::NONE);
            format!("{} | Werkzeug {}", selection(&h), active_tool(&h))
        }),
        ("W10 Stift: Esc beendet offen", || {
            let mut h = basis();
            tool(&mut h, ToolId::Pen);
            let (a, b) = (screen(&h, 200.0, 200.0), screen(&h, 260.0, 200.0));
            click(&mut h, a, Modifiers::NONE);
            h.run_steps(30);
            click(&mut h, b, Modifiers::NONE);
            key(&mut h, Key::Escape, Modifiers::NONE);
            format!("{} | {}", elements(&h), history(&h))
        }),
        ("W11 Stift: Enter beendet", || {
            let mut h = basis();
            tool(&mut h, ToolId::Pen);
            let (a, b) = (screen(&h, 200.0, 200.0), screen(&h, 260.0, 200.0));
            click(&mut h, a, Modifiers::NONE);
            h.run_steps(30);
            click(&mut h, b, Modifiers::NONE);
            key(&mut h, Key::Enter, Modifiers::NONE);
            format!("{} | {}", elements(&h), history(&h))
        }),
        ("W12 Auswahlrahmen Umschalt+Ziehen (Quadrat) und Hinzufügen", || {
            let mut h = basis();
            tool(&mut h, ToolId::Marquee);
            let (a, b) = (screen(&h, 200.0, 200.0), screen(&h, 260.0, 230.0));
            drag(&mut h, a, b, 10, shift());
            let d = st(&h).session.active().expect("d");
            format!("Pixelauswahl {:?}", d.selection.pixels.as_ref().map(|m| m.tight_bounds()))
        }),
        ("W13 Auswahlrahmen: Esc / Strg+D hebt auf", || {
            let mut h = basis();
            tool(&mut h, ToolId::Marquee);
            let (a, b) = (screen(&h, 200.0, 200.0), screen(&h, 260.0, 230.0));
            drag(&mut h, a, b, 10, Modifiers::NONE);
            key(&mut h, Key::Escape, Modifiers::NONE);
            let e = st(&h).session.active().expect("d").selection.has_pixels();
            drag(&mut h, a, b, 10, Modifiers::NONE);
            key(&mut h, Key::D, ctrl());
            let d = st(&h).session.active().expect("d").selection.has_pixels();
            format!("nach Esc noch Auswahl: {e} | nach Strg+D: {d}")
        }),
        ("W14 Zuschneiden: Enter übernimmt, Esc bricht ab", || {
            let mut h = basis();
            tool(&mut h, ToolId::Crop);
            let (a, b) = (screen(&h, 20.0, 20.0), screen(&h, 300.0, 200.0));
            drag(&mut h, a, b, 10, Modifiers::NONE);
            key(&mut h, Key::Escape, Modifiers::NONE);
            let x = {
                let d = st(&h).session.active().expect("d");
                let p = &d.doc.pages[d.page].canvas;
                format!("{}×{}", p.width, p.height)
            };
            drag(&mut h, a, b, 10, Modifiers::NONE);
            key(&mut h, Key::Enter, Modifiers::NONE);
            let d = st(&h).session.active().expect("d");
            let p = &d.doc.pages[d.page].canvas;
            format!("nach Esc {x} | nach Enter {}×{}", p.width, p.height)
        }),
        ("W15 Skalieren-Werkzeug: Esc bricht ab, Enter übernimmt", || {
            let mut h = basis();
            select(&mut h, &["R2"]);
            tool(&mut h, ToolId::Scale);
            let (a, b) = (screen(&h, 330.0, 140.0), screen(&h, 370.0, 180.0));
            drag(&mut h, a, b, 10, Modifiers::NONE);
            key(&mut h, Key::Escape, Modifiers::NONE);
            let e = selection(&h);
            drag(&mut h, a, b, 10, Modifiers::NONE);
            key(&mut h, Key::Enter, Modifiers::NONE);
            format!("nach Esc {e} | nach Enter {} | Werkzeug {}", selection(&h), active_tool(&h))
        }),
        ("W16 Pipette Klick auf R2", || {
            let mut h = basis();
            tool(&mut h, ToolId::Eyedropper);
            let p = screen(&h, 290.0, 100.0);
            click(&mut h, p, Modifiers::NONE);
            let t = &st(&h).session.tools;
            format!("Füllung {:?} Strich {:?}", t.fill_color, t.stroke_color)
        }),
        ("W17 Werkzeugtaste während Ziehen (R-Taste)", || {
            let mut h = basis();
            tool(&mut h, ToolId::Pointer);
            let (a, b) = (screen(&h, 100.0, 85.0), screen(&h, 140.0, 105.0));
            ev(&mut h, Event::PointerMoved(a));
            press(&mut h, a, PointerButton::Primary, true, Modifiers::NONE);
            for i in 1..=5 {
                ev(&mut h, Event::PointerMoved(a + (b - a) * (i as f32 / 5.0)));
            }
            key(&mut h, Key::R, Modifiers::NONE);
            press(&mut h, b, PointerButton::Primary, false, Modifiers::NONE);
            h.run_steps(2);
            format!("{} | Werkzeug {}", elements(&h), active_tool(&h))
        }),
        ("W18 Werkzeugtaste V/R/T/Z/H", || {
            let mut h = basis();
            let mut out = Vec::new();
            for k in [Key::R, Key::T, Key::Z, Key::H, Key::V, Key::P, Key::M, Key::L, Key::W, Key::I, Key::K, Key::E] {
                key(&mut h, k, Modifiers::NONE);
                out.push(format!("{k:?}={}", active_tool(&h)));
            }
            out.join(" ")
        }),
        ("W19 R zweimal (Flyout durchschalten)", || {
            let mut h = basis();
            key(&mut h, Key::R, Modifiers::NONE);
            let a = active_tool(&h);
            key(&mut h, Key::R, Modifiers::NONE);
            let b = active_tool(&h);
            key(&mut h, Key::R, Modifiers::NONE);
            format!("{a} → {b} → {}", active_tool(&h))
        }),
        // ---------------- Bitmapmodus ----------------
        ("B01 Doppelklick auf Bitmap (Zeiger)", || {
            let mut h = basis();
            let px = bitmap(&mut h);
            let _ = run(&mut h, "select.set", json!({"ids": []}));
            tool(&mut h, ToolId::Pointer);
            let p = screen(&h, 250.0, 250.0);
            dclick(&mut h, p, Modifiers::NONE);
            let d = st(&h).session.active().expect("d");
            format!("{px} | Bitmapmodus {} | Werkzeug {}", d.bitmap_mode, active_tool(&h))
        }),
        ("B02 Strg+E / Strg+Umschalt+D auf Bitmap", || {
            let mut h = basis();
            let _ = bitmap(&mut h);
            key(&mut h, Key::E, ctrl());
            let a = st(&h).session.active().expect("d").bitmap_mode;
            key(&mut h, Key::D, plus(ctrl(), shift()));
            let b = st(&h).session.active().expect("d").bitmap_mode;
            format!("nach Strg+E {a} | nach Strg+Umschalt+D {b}")
        }),
        ("B03 Umschalt+E / Strg+Umschalt+E (Web Standard)", || {
            let mut h = basis();
            let _ = bitmap(&mut h);
            key(&mut h, Key::E, shift());
            let a = st(&h).session.active().expect("d").bitmap_mode;
            key(&mut h, Key::E, plus(ctrl(), shift()));
            let b = st(&h).session.active().expect("d").bitmap_mode;
            format!("nach Umschalt+E {a} | nach Strg+Umschalt+E {b}")
        }),
        ("B04 Bitmapwerkzeug auf ausgewählter Bitmap (Radiergummi)", || {
            let mut h = basis();
            let _ = bitmap(&mut h);
            tool(&mut h, ToolId::Eraser);
            let (a, b) = (screen(&h, 220.0, 220.0), screen(&h, 280.0, 220.0));
            drag(&mut h, a, b, 10, Modifiers::NONE);
            let d = st(&h).session.active().expect("d");
            format!("Bitmapmodus {} | Verlauf {}", d.bitmap_mode, history(&h))
        }),
        ("C12 Strg+Umschalt+V wie egui-winit (Event::Paste statt Taste)", || {
            let mut h = basis();
            select(&mut h, &["T"]);
            let _ = run(&mut h, "edit.copy", json!({}));
            select(&mut h, &["R1"]);
            h.event(Event::ModifiersChanged(plus(ctrl(), shift())));
            // egui-winit: Strg(+Umschalt)+V → nur Paste (bei Text in der Ablage) bzw. gar nichts; danach Loslassen
            ev(&mut h, Event::Key { key: Key::V, physical_key: Some(Key::V), pressed: false, repeat: false, modifiers: plus(ctrl(), shift()) });
            h.run_steps(2);
            format!("{} | Verlauf {}", elements(&h), history(&h))
        }),
        ("C13 Strg+Umschalt+D Klonen (Web Standard)", || {
            let mut h = basis();
            select(&mut h, &["R1"]);
            key(&mut h, Key::D, plus(ctrl(), shift()));
            format!("{} | Verlauf {}", elements(&h), history(&h))
        }),
        ("Z26 Strg+Rad über Punkt bei großem Dokument (Anker)", || {
            let mut h = harness_with(3000.0, 2000.0);
            let _ = run(&mut h, "view.setZoom", json!({"zoom": 1.0}));
            h.run_steps(2);
            let a = area(&h);
            let p = a.min + egui::vec2(200.0, 150.0);
            let before = doc_at(&h, p);
            wheel(&mut h, p, MouseWheelUnit::Line, 0.0, 1.0, ctrl());
            format!("{} | Punkt unter Maus vorher {:?} nachher {:?}", view(&h), before, doc_at(&h, p))
        }),
        // ---------------- Rückgängig ----------------
        ("U01 Rechteck zeichnen + Strg+Z / Strg+Umschalt+Z / Strg+Y", || {
            let mut h = basis();
            tool(&mut h, ToolId::Rectangle);
            let (a, b) = (screen(&h, 200.0, 200.0), screen(&h, 260.0, 230.0));
            drag(&mut h, a, b, 10, Modifiers::NONE);
            let n = history(&h);
            key(&mut h, Key::Z, ctrl());
            let u = elements(&h);
            key(&mut h, Key::Z, plus(ctrl(), shift()));
            let r = elements(&h);
            key(&mut h, Key::Z, ctrl());
            key(&mut h, Key::Y, ctrl());
            format!("{n} | nach Strg+Z {u} | nach Strg+Umschalt+Z {r} | nach Strg+Y {}", elements(&h))
        }),
        ("U02 Klick-Auswahl ist kein Schritt; Rückgängig stellt Auswahl her", || {
            let mut h = basis();
            select(&mut h, &["R1"]);
            key(&mut h, Key::ArrowRight, Modifiers::NONE);
            tool(&mut h, ToolId::Pointer);
            let p = screen(&h, 290.0, 100.0);
            click(&mut h, p, Modifiers::NONE);
            key(&mut h, Key::Z, ctrl());
            format!("{} | Verlauf {}", selection(&h), history(&h))
        }),
        ("U03 Zoom ist kein Schritt", || {
            let mut h = basis();
            let a = history(&h);
            key(&mut h, Key::Plus, ctrl());
            format!("{a} → {}", history(&h))
        }),
        // ---------------- Hilfslinien / Fang ----------------
        ("G01 Fang an Hilfslinie beim Verschieben (5 px)", || {
            let mut h = basis();
            let _ = run(&mut h, "guide.add", json!({"orientation": "vertical", "position": 200}));
            tool(&mut h, ToolId::Pointer);
            let (a, b) = (screen(&h, 100.0, 85.0), screen(&h, 146.0, 85.0));
            drag(&mut h, a, b, 10, Modifiers::NONE);
            selection(&h)
        }),
        ("G02 Pfeiltaste ignoriert Fang", || {
            let mut h = basis();
            let _ = run(&mut h, "guide.add", json!({"orientation": "vertical", "position": 152}));
            select(&mut h, &["R1"]);
            key(&mut h, Key::ArrowRight, Modifiers::NONE);
            selection(&h)
        }),
        ("G03 Intelligente Hilfslinie (Oberkante R2=60, R1 auf 58)", || {
            let mut h = basis();
            tool(&mut h, ToolId::Pointer);
            let (a, b) = (screen(&h, 100.0, 85.0), screen(&h, 100.0, 93.0));
            drag(&mut h, a, b, 10, Modifiers::NONE);
            selection(&h)
        }),
        // ---------------- Drag & Drop ----------------
        ("D01 Datei auf Leinwand ablegen (offenes Dokument)", || {
            let mut h = basis();
            let dir = std::env::temp_dir().join("fp_uxa_drop");
            let _ = std::fs::create_dir_all(&dir);
            let p = dir.join("drop.png");
            let img = vec![255u8; 20 * 10 * 4];
            let _ = fp_ui::os_clipboard::write_png(&p, 20, 10, &img);
            let at = screen(&h, 100.0, 300.0);
            ev(&mut h, Event::PointerMoved(at));
            h.input_mut().dropped_files.push(std::sync::Arc::new(TestDrop(p.clone())));
            h.run_steps(4);
            let docs = st(&h).session.documents().len();
            format!("Dokumente {docs} | {} | Auswahl {}", elements(&h), selection(&h))
        }),
    ]
}

#[test]
#[ignore = "Messlauf für docs\\UX_ABGLEICH_A.md (--ignored --nocapture)"]
fn messen() {
    let filter = std::env::var("UXA_FILTER").unwrap_or_default();
    let mut out = String::new();
    for (name, f) in cases() {
        if !filter.is_empty() && !name.starts_with(&filter) {
            continue;
        }
        let r = std::panic::catch_unwind(f).unwrap_or_else(|e| format!("PANIK {:?}", e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))));
        let line = format!("UXA|{name}|{r}");
        println!("{line}");
        out.push_str(&line);
        out.push('\n');
    }
    if let Ok(p) = std::env::var("UXA_OUT") {
        let _ = std::fs::write(p, out);
    }
}
