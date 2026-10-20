//! PERF-1: "Alles auswählen" auf Seiten mit Tausenden Objekten – Zeit je Bild.
//!
//! Messung (Release): `cargo test -p fp-ui --release --test all select_all_perf:: -- --ignored --nocapture`
//!
//! Je Bild: Oberfläche (`Harness::step`, CPU) und Zeichnen mit egui-wgpu wie im echten Fenster
//! (Zerlegen in Dreiecke, Puffer/Texturen hochladen, Zeichnen, auf das Ende warten, ohne Rücklesen),
//! mit der Anpassung für Software-Rasterer (`vendor\egui-wgpu`). Dokumente: ein erzeugtes mit
//! 3200 Rechtecken (100 · 2^5) und, falls vorhanden, die Korpusseite `Wireframes\iPad.png`.
//!
//! Die nicht ignorierten Tests prüfen ohne Zeitmessung, dass bei großer Auswahl das Bild klein bleibt
//! (Anzahl Formen/Eckpunkte) und dass Auswahl und Darstellung dieselben bleiben.

use eframe::egui_wgpu;
use eframe::wgpu;
use egui::{Event, Key, Modifiers};
use egui_kittest::Harness;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::FirepixelApp;
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::time::Instant;

type H = Harness<'static, FirepixelApp>;

fn corpus_page() -> Option<std::path::PathBuf> {
    let root = std::env::var_os("FP_REA_DIR").map(std::path::PathBuf::from).unwrap_or_else(|| std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join(".."));
    let p = root.join("korpus").join("First Run").join("Templates").join("Wireframes").join("iPad.png");
    p.is_file().then_some(p)
}

/// Zeichnet wie eframe in ein Ziel in Fenstergröße, ohne zurückzulesen; merkt sich Dauer und Eckpunkte.
struct PaintOnly {
    rs: egui_wgpu::RenderState,
    renderer: egui_wgpu::Renderer,
    target: Option<(wgpu::TextureView, [u32; 2])>,
    last: Arc<Mutex<(f64, usize)>>,
}

impl egui_kittest::TestRenderer for PaintOnly {
    fn handle_delta(&mut self, delta: &mut egui::TexturesDelta) {
        #[allow(clippy::iter_over_hash_type)] // Reihenfolge egal (wie egui_kittest)
        for (id, images) in delta.set.drain() {
            for image in images {
                self.renderer.update_texture(&self.rs.device, &self.rs.queue, id, &image);
            }
        }
        #[allow(clippy::iter_over_hash_type)]
        for id in delta.free.drain() {
            self.renderer.free_texture(&id);
        }
    }

    fn render(&mut self, ctx: &egui::Context, output: &egui::FullOutput) -> Result<image::RgbaImage, String> {
        let t = Instant::now();
        let ppp = ctx.pixels_per_point();
        let size = ctx.content_rect().size() * ppp;
        let px = [size.x.round() as u32, size.y.round() as u32];
        if self.target.as_ref().is_none_or(|(_, s)| *s != px) {
            let tex = self.rs.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("perf1-target"),
                size: wgpu::Extent3d { width: px[0], height: px[1], depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Bgra8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            self.target = Some((tex.create_view(&Default::default()), px));
        }
        let Some((view, _)) = &self.target else { return Err("no target".into()) };
        let prims = ctx.tessellate(output.shapes.clone(), ppp);
        let verts = prims
            .iter()
            .map(|p| match &p.primitive {
                egui::epaint::Primitive::Mesh(m) => m.vertices.len(),
                egui::epaint::Primitive::Callback(_) => 0,
            })
            .sum();
        let sd = egui_wgpu::ScreenDescriptor { size_in_pixels: px, pixels_per_point: ppp };
        let mut enc = self.rs.device.create_command_encoder(&Default::default());
        let user = self.renderer.update_buffers(&self.rs.device, &self.rs.queue, &mut enc, &prims, &sd);
        {
            let mut pass = enc
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment { view, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store }, depth_slice: None })],
                    ..Default::default()
                })
                .forget_lifetime();
            self.renderer.render(&mut pass, &prims, &sd);
        }
        self.rs.queue.submit(user.into_iter().chain(std::iter::once(enc.finish())));
        let _ = self.rs.device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None });
        if let Ok(mut m) = self.last.lock() {
            *m = (t.elapsed().as_secs_f64() * 1000.0, verts);
        }
        Ok(image::RgbaImage::new(1, 1))
    }
}

/// Sitzung mit einem erzeugten Dokument: `n` Rechtecke im Raster auf einer 6000×1600-Seite.
fn many_rects_state(n: usize) -> AppState {
    fp_engine::shell::disable();
    let session = fp_ui::ui_commands::new_session(fp_engine::Preferences::new());
    let mut st = AppState::new(session, UiSettings::default(), None);
    st.sync_render = true;
    st.os_clipboard = false;
    st.session.execute("file.new", json!({ "width": 6000, "height": 1600, "background": "#ffffff" })).expect("new doc");
    // eine Zeile mit 100 Rechtecken, dann Zeilen verdoppeln (Klonen + Verschieben): 100 · 2^k
    let cols = 100;
    for i in 0..cols {
        let x = i as f64 * 60.0;
        let fill = if i % 2 == 0 { "#3366cc" } else { "#cc6633" };
        st.session.execute("draw.rectangle", json!({ "rect": [x + 4.0, 4.0, x + 52.0, 40.0], "fillColor": fill })).expect("rect");
    }
    let mut rows = 1;
    while rows * cols < n {
        st.session.execute("select.all", json!({})).expect("all");
        st.session.execute("edit.clone", json!({})).expect("clone");
        st.session.execute("edit.move", json!({ "dx": 0, "dy": rows as f64 * 48.0 })).expect("move");
        rows *= 2;
    }
    let _ = st.session.execute("select.none", json!({}));
    st
}

fn corpus_state(path: &std::path::Path) -> AppState {
    fp_engine::shell::disable();
    let session = fp_ui::ui_commands::new_session(fp_engine::Preferences::new());
    let mut st = AppState::new(session, UiSettings::default(), None);
    st.sync_render = true;
    st.os_clipboard = false;
    st.session.execute("file.open", json!({ "path": path.to_string_lossy() })).expect("open corpus page");
    st
}

fn harness_gpu(mut st: AppState) -> (H, Arc<Mutex<(f64, usize)>>) {
    // wie im Fenster: Leinwand und Seitenminiaturen rechnen im Hintergrund
    st.sync_render = false;
    egui_wgpu::SOFTWARE_RASTERIZER_TUNING.store(true, std::sync::atomic::Ordering::Relaxed);
    let rs = egui_kittest::wgpu::create_render_state(egui_kittest::wgpu::default_wgpu_setup(), Default::default());
    eprintln!("   Adapter: {} ({:?})", rs.adapter.get_info().name, rs.adapter.get_info().device_type);
    let renderer = egui_wgpu::Renderer::new(&rs.device, wgpu::TextureFormat::Bgra8Unorm, Default::default());
    let last = Arc::new(Mutex::new((0.0, 0)));
    let paint = PaintOnly { rs, renderer, target: None, last: Arc::clone(&last) };
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, fp_engine::Session::default(), UiSettings::default(), false);
    app.state = st;
    let mut h = Harness::builder().renderer(crate::gpu_common::renderer())
        .with_size(egui::vec2(1800.0, 1000.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(20)
        .with_step_dt(1.0 / 60.0)
        .renderer(paint)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(6);
    // Leinwand und Miniaturen aus dem Hintergrund abwarten
    for _ in 0..40 {
        std::thread::sleep(std::time::Duration::from_millis(250));
        h.run_steps(1);
    }
    (h, last)
}

fn harness_cpu(st: AppState) -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, fp_engine::Session::default(), UiSettings::default(), false);
    app.state = st;
    let mut h = Harness::builder().renderer(crate::gpu_common::renderer()).with_size(egui::vec2(1800.0, 1000.0)).with_pixels_per_point(1.0).with_max_steps(20).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(6);
    h
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

fn stat(v: &[f64]) -> String {
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.total_cmp(b));
    format!("min {:.1}  Median {:.1}  max {:.1} ms", s[0], s[s.len() / 2], s[s.len() - 1])
}

fn median(v: &[f64]) -> f64 {
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.total_cmp(b));
    s[s.len() / 2]
}

fn key(h: &mut H, k: Key, m: Modifiers) {
    h.event(Event::Key { key: k, physical_key: None, pressed: true, repeat: false, modifiers: m });
    h.event(Event::Key { key: k, physical_key: None, pressed: false, repeat: false, modifiers: m });
}

/// Ein Bild: (Oberfläche, Zeichnen, Eckpunkte).
fn frame(h: &mut H, last: &Arc<Mutex<(f64, usize)>>) -> (f64, f64, usize) {
    let t = Instant::now();
    h.step();
    let ui = ms(t);
    let _ = h.render();
    let (gpu, v) = last.lock().map(|m| *m).unwrap_or((0.0, 0));
    (ui, gpu, v)
}

/// Strg+A, dann Ruhebilder und 10× Pfeil rechts; gibt den Median der Bildzeit (Oberfläche + Zeichnen) zurück.
fn measure(name: &str, st: AppState) -> (f64, f64) {
    eprintln!("== {name}");
    let (mut h, last) = harness_gpu(st);
    let n_objects = h.state().state.session.active().map(|d| fp_engine::docops::selectable_ids(&d.doc, d.page, d.frame).len()).unwrap_or(0);
    let idle: Vec<_> = (0..5).map(|_| frame(&mut h, &last)).collect();
    eprintln!("   ohne Auswahl ({n_objects} Objekte): Oberfläche {}, Zeichnen {}, Eckpunkte {}", stat(&idle.iter().map(|f| f.0).collect::<Vec<_>>()), stat(&idle.iter().map(|f| f.1).collect::<Vec<_>>()), idle[0].2);
    let t = Instant::now();
    key(&mut h, Key::A, Modifiers::COMMAND);
    let (ui_a, gpu_a, _) = frame(&mut h, &last);
    eprintln!("   Strg+A: Bild {:.1} ms (Oberfläche {ui_a:.1}, Zeichnen {gpu_a:.1}), gesamt mit Befehl {:.1} ms", ui_a + gpu_a, ms(t));
    let sel = h.state().state.session.active().map(|d| d.selection.elements.len()).unwrap_or(0);
    let idle: Vec<_> = (0..8).map(|_| frame(&mut h, &last)).collect();
    let tot: Vec<f64> = idle.iter().map(|f| f.0 + f.1).collect();
    eprintln!("   alles gewählt ({sel}): Oberfläche {}, Zeichnen {}, Eckpunkte {}", stat(&idle.iter().map(|f| f.0).collect::<Vec<_>>()), stat(&idle.iter().map(|f| f.1).collect::<Vec<_>>()), idle[0].2);
    let mut arrows = Vec::new();
    for _ in 0..10 {
        let t = Instant::now();
        key(&mut h, Key::ArrowRight, Modifiers::NONE);
        let (ui, gpu, _) = frame(&mut h, &last);
        let (ui2, gpu2, _) = frame(&mut h, &last);
        arrows.push((ui, gpu, ms(t), ui2 + gpu2));
    }
    eprintln!(
        "   Pfeil rechts: Bild mit Taste {} (Oberfläche {}, Zeichnen {}), folgendes Bild {}",
        stat(&arrows.iter().map(|a| a.0 + a.1).collect::<Vec<_>>()),
        stat(&arrows.iter().map(|a| a.0).collect::<Vec<_>>()),
        stat(&arrows.iter().map(|a| a.1).collect::<Vec<_>>()),
        stat(&arrows.iter().map(|a| a.3).collect::<Vec<_>>())
    );
    drag_first_object(&mut h, &last);
    (median(&tot), median(&arrows.iter().map(|a| a.0 + a.1).collect::<Vec<_>>()))
}

/// Nur gemessen (nicht umgebaut): Ziehen der ganzen Auswahl am ersten Objekt, 8 Mausschritte.
fn drag_first_object(h: &mut H, last: &Arc<Mutex<(f64, usize)>>) {
    let Some((pr, r, center)) = (|| {
        let s = &h.state().state;
        let d = s.session.active()?;
        let pr = s.canvases.get(&d.id)?.page_screen?;
        let first = d.selection.elements.first()?;
        let e = fp_engine::docops::element_on_page(&d.doc, d.page, *first)?;
        let b = fp_engine::docops::element_bounds(&d.doc, &e)?;
        let r = fp_render::Renderer::page_rect(&d.doc, d.page)?;
        Some((pr, r, ((b.left + b.right) / 2.0, (b.top + b.bottom) / 2.0)))
    })() else {
        eprintln!("   Ziehen: keine Seitenlage – übersprungen");
        return;
    };
    let (w, ht) = ((r.right - r.left) as f32, (r.bottom - r.top) as f32);
    let p0 = egui::pos2(pr.left() + (center.0 - r.left as f32) / w * pr.width(), pr.top() + (center.1 - r.top as f32) / ht * pr.height());
    h.input_mut().max_texture_side = Some(8192); // wie das Gerät im Fenster (kittest: 2048)
    h.event(Event::PointerMoved(p0));
    h.event(Event::PointerButton { pos: p0, button: egui::PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    let _ = frame(h, last);
    let mut steps = Vec::new();
    for i in 1..=8 {
        let p = p0 + egui::vec2(6.0 * i as f32, 3.0 * i as f32);
        let t = Instant::now();
        h.event(Event::PointerMoved(p));
        let (ui, gpu, _) = frame(h, last);
        steps.push((ui, gpu, ms(t)));
    }
    let p = p0 + egui::vec2(48.0, 24.0);
    h.event(Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    let _ = frame(h, last);
    eprintln!(
        "   Ziehen (nur gemessen): erster Mausschritt {:.1} ms (Oberfläche {:.1}, Zeichnen {:.1}), weitere {}",
        steps[0].0 + steps[0].1,
        steps[0].0,
        steps[0].1,
        stat(&steps[1..].iter().map(|s| s.0 + s.1).collect::<Vec<_>>())
    );
}

#[test]
#[ignore = "Messung (Release): cargo test -p fp-ui --release --test all select_all_perf:: -- --ignored --nocapture"]
fn select_all_3000_objects_frame_time() {
    let (idle, arrow) = measure("3200 Rechtecke (erzeugt)", many_rects_state(3000));
    eprintln!("ERGEBNIS 3200 Rechtecke: Bild bei Auswahl {idle:.1} ms, Bild mit Pfeiltaste {arrow:.1} ms");
    if let Some(p) = corpus_page() {
        let (idle, arrow) = measure("Korpus Wireframes\\iPad.png", corpus_state(&p));
        eprintln!("ERGEBNIS iPad.png: Bild bei Auswahl {idle:.1} ms, Bild mit Pfeiltaste {arrow:.1} ms");
    }
}

/// Einzelne Bausteine je Bild mit allen 3200 Objekten gewählt (CPU, ohne Zeichnen).
#[test]
#[ignore = "Messung: cargo test -p fp-ui --test all select_all_perf::probe -- --ignored --nocapture"]
fn probe_parts_with_large_selection() {
    let mut st = many_rects_state(3000);
    st.session.execute("select.all", json!({})).expect("all");
    let n = st.session.active().map(|d| d.selection.elements.len()).unwrap_or(0);
    let time = |name: &str, reps: usize, f: &mut dyn FnMut()| {
        let t = Instant::now();
        for _ in 0..reps {
            f();
        }
        eprintln!("   {name}: {:.2} ms", ms(t) / reps as f64);
    };
    eprintln!("== Bausteine, {n} gewählt");
    time("inspector::layouts::select_layout", 3, &mut || {
        let _ = fp_ui::inspector::layouts::select_layout(&st.session);
    });
    time("inspector::obj_quads::selection", 3, &mut || {
        let _ = fp_ui::inspector::obj_quads::selection(&st);
    });
    let names: Vec<String> = st.session.registry().iter().map(|c| c.info().name.clone()).collect();
    time(&format!("command_state aller {} Befehle", names.len()), 1, &mut || {
        for nm in &names {
            let _ = st.session.command_state(nm);
        }
    });
    let mut worst: Vec<(f64, String)> = names
        .iter()
        .map(|nm| {
            let t = Instant::now();
            let _ = st.session.command_state(nm);
            (ms(t), nm.clone())
        })
        .collect();
    worst.sort_by(|a, b| b.0.total_cmp(&a.0));
    eprintln!("   langsamste Befehle: {:?}", &worst[..8.min(worst.len())]);
    time("session.tool_feedback", 3, &mut || {
        let _ = st.session.tool_feedback();
    });
    {
        let d = st.session.active().expect("doc");
        let (doc, page) = (Arc::clone(&d.doc), d.page);
        let r = Arc::clone(&st.renderer);
        time("Seitenminiatur: render_page (ganze Seite 100 %)", 1, &mut || {
            let _ = r.render_page(&doc, page, 0);
        });
        let pr = fp_render::Renderer::page_rect(&doc, page).expect("page");
        time("Seitenminiatur: render_region (Zoom 64/6000)", 1, &mut || {
            let _ = r.render_region(&doc, page, 0, pr, 64.0 / 6000.0);
        });
    }
    // ganzes Bild (CPU) mit und ohne Bedienfelder/Inspektor
    let mut h = harness_cpu(st);
    h.run_steps(3);
    let t = |h: &mut H, name: &str| {
        let v: Vec<f64> = (0..4)
            .map(|_| {
                let t = Instant::now();
                h.step();
                ms(t)
            })
            .collect();
        eprintln!("   Bild {name}: {}", stat(&v));
    };
    t(&mut h, "mit Bedienfeldern");
    h.state_mut().state.settings.workspace.panels_hidden = true;
    h.run_steps(2);
    t(&mut h, "Bedienfelder ausgeblendet");
    h.state_mut().state.settings.workspace.panels_hidden = false;
    let _ = h.state_mut().state.session.execute("view.setZoom", json!({ "zoom": 4.0 }));
    h.run_steps(2);
    t(&mut h, "mit Bedienfeldern, Zoom 400 %");
}

/// Gebündelte Auswahlrahmen (`FrameBatch`) ergeben dasselbe Bild wie einzelne Formen: 300 gewählte
/// Objekte bei 100 % und bei 37 % (Rahmen kleiner, Griffe überlappen).
#[test]
fn frame_batch_draws_same_pixels() {
    use std::sync::atomic::Ordering::Relaxed;
    let render = |zoom: f64, batch: bool| -> image::RgbaImage {
        let st = many_rects_state(300);
        let ctx = egui::Context::default();
        let mut app = FirepixelApp::with_session(&ctx, fp_engine::Session::default(), UiSettings::default(), false);
        app.state = st;
        let mut h = Harness::builder().renderer(crate::gpu_common::renderer()).with_size(egui::vec2(1200.0, 800.0)).with_pixels_per_point(1.0).with_max_steps(20).renderer(crate::gpu_common::renderer()).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);

        let _ = h.state_mut().state.session.execute("view.setZoom", json!({ "zoom": zoom }));
        let _ = h.state_mut().state.session.execute("select.all", json!({}));
        h.run_steps(4);
        fp_ui::canvas::overlay::FRAME_BATCH_MIN.store(if batch { 32 } else { usize::MAX }, Relaxed);
        h.run_steps(2);
        let img = h.render().expect("render");
        fp_ui::canvas::overlay::FRAME_BATCH_MIN.store(32, Relaxed);
        img
    };
    for zoom in [1.0, 0.37] {
        let a = render(zoom, false);
        let b = render(zoom, true);
        assert_eq!(a.dimensions(), b.dimensions());
        let diff = a.pixels().zip(b.pixels()).filter(|(p, q)| p != q).count();
        assert_eq!(diff, 0, "Zoom {zoom}: {diff} Pixel verschieden");
    }
}

/// Ohne Zeitmessung: Mit 3000 gewählten Objekten bleibt das Bild klein, Auswahl und Pfeiltaste
/// wirken auf alle Objekte.
#[test]
fn select_all_3000_objects_stays_light() {
    let mut h = harness_cpu(many_rects_state(3000));
    key(&mut h, Key::A, Modifiers::COMMAND);
    h.run_steps(2);
    let d = h.state().state.session.active().expect("doc");
    assert!(d.selection.elements.len() >= 3000, "Strg+A wählt alle");
    let before: Vec<_> = d.selection.elements.iter().filter_map(|id| fp_engine::docops::element_on_page(&d.doc, d.page, *id)).filter_map(|e| fp_engine::docops::element_bounds(&d.doc, &e)).collect();
    key(&mut h, Key::ArrowRight, Modifiers::NONE);
    h.run_steps(2);
    let d = h.state().state.session.active().expect("doc");
    assert!(d.selection.elements.len() >= 3000, "Auswahl bleibt");
    let after: Vec<_> = d.selection.elements.iter().filter_map(|id| fp_engine::docops::element_on_page(&d.doc, d.page, *id)).filter_map(|e| fp_engine::docops::element_bounds(&d.doc, &e)).collect();
    assert_eq!(before.len(), after.len());
    for (b, a) in before.iter().zip(&after) {
        assert!((a.left - b.left - 1.0).abs() < 1e-3 && (a.top - b.top).abs() < 1e-3, "um 1 px nach rechts: {b:?} → {a:?}");
    }
}

/// Frames after a view change until the canvas image is settled (no render request pending):
/// (largest frame time, frame count, wall time to settle) in ms.
fn settle(h: &mut H, last: &Arc<Mutex<(f64, usize)>>) -> (f64, usize, f64) {
    let t0 = Instant::now();
    let (mut worst, mut n, mut calm) = (0.0f64, 0, 0);
    while n < 400 && calm < 3 {
        let (ui, gpu, _) = frame(h, last);
        worst = worst.max(ui + gpu);
        n += 1;
        let s = &h.state().state;
        let settled = s.session.active_id().and_then(|id| s.canvases.get(&id)).is_some_and(|cc| cc.requested.is_some() && cc.requested == cc.shown);
        calm = if settled { calm + 1 } else { 0 };
        std::thread::sleep(std::time::Duration::from_millis(8));
    }
    (worst, n, ms(t0))
}

/// BA-098: first frames after zoom and scroll on a large page (software rendering).
#[test]
#[ignore = "Messung (Release): cargo test -p fp-ui --release --test all select_all_perf::zoom_scroll -- --ignored --nocapture"]
fn zoom_scroll_frame_time() {
    let (mut h, last) = harness_gpu(many_rects_state(3000));
    h.input_mut().max_texture_side = Some(8192);
    for zoom in [1.0, 2.0, 4.0, 1.5] {
        let _ = h.state_mut().state.session.execute("view.setZoom", json!({ "zoom": zoom }));
        let (w, n, t) = settle(&mut h, &last);
        eprintln!("   zoom {:>3.0} %: worst frame {w:.1} ms, {n} frames, settled after {t:.0} ms", zoom * 100.0);
        let mut worst = Vec::new();
        for dx in [-1700.0, 1700.0, 1700.0, -1700.0] {
            let _ = h.state_mut().state.session.execute("view.pan", json!({ "dx": dx, "dy": 0.0 }));
            worst.push(settle(&mut h, &last).0);
        }
        eprintln!("   scroll at {:>3.0} % (4 × one view): worst frames {:?}", zoom * 100.0, worst.iter().map(|w| format!("{w:.1}")).collect::<Vec<_>>());
    }
}

/// BA-097: parts of the first drag step with 3200 selected objects (CPU, without drawing).
#[test]
#[ignore = "Messung (Release): cargo test -p fp-ui --release --test all select_all_perf::drag_parts -- --ignored --nocapture"]
fn drag_parts_with_large_selection() {
    let mut st = many_rects_state(3000);
    st.session.execute("select.all", json!({})).expect("all");
    let d = st.session.active().expect("doc");
    let ids: Vec<u64> = d.selection.elements.iter().map(|e| e.0).collect();
    let (doc, page, frame) = (Arc::clone(&d.doc), d.page, d.frame);
    let t = Instant::now();
    let plan = fp_ui::canvas::stack::plan(&doc, page, frame, &ids, false);
    eprintln!("   stack::plan: {:.1} ms ({} runs)", ms(t), plan.as_ref().map(|p| p.runs.len()).unwrap_or(0));
    let els: Vec<_> = ids.iter().filter_map(|id| fp_engine::docops::element_on_page(&doc, page, fp_model::ElementId(*id))).collect();
    for pass in 0..2 {
        let t = Instant::now();
        let img = fp_ui::canvas::render::render_elements(&st.renderer, &doc, frame, &els);
        eprintln!("   render_elements pass {pass}: {:.1} ms ({:?})", ms(t), img.map(|i| (i.width(), i.height())));
    }
    let t = Instant::now();
    let _ = st.renderer.render_page(&doc, page, frame);
    eprintln!("   render_page: {:.1} ms", ms(t));
    let t = Instant::now();
    let img = fp_ui::canvas::render::render_elements(&st.renderer, &doc, frame, &els);
    eprintln!("   render_elements after page render: {:.1} ms", ms(t));
    let t = Instant::now();
    let _ = img.map(|i| fp_ui::canvas::render::to_color_image(&i));
    eprintln!("   to_color_image: {:.1} ms", ms(t));
}
