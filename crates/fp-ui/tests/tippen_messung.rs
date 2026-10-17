//! FV-8: Tippen mit dem Textwerkzeug auf einer großen Korpusseite – Zeit je Zeichen.
//!
//! `cargo test -p fp-ui --release --test tippen_messung -- --ignored --nocapture`
//!
//! Je Zeichen wird ein Bild der Oberfläche berechnet (Ereignis → Werkzeug → Textsatz → Vorschau)
//! und mit egui-wgpu gezeichnet wie im echten Fenster (Zerlegen in Dreiecke, Texturen und Puffer
//! hochladen, Zeichnen, auf das Ende warten – ohne das Rücklesen des Bildes, das nur Tests
//! brauchen). Auf Rechnern ohne Grafikkarte ist der Zeichner der Software-Rasterer von Windows;
//! gemessen wird einmal wie egui-wgpu 0.36.2 und einmal mit der Anpassung für Software-Rasterer
//! (`egui_wgpu::SOFTWARE_RASTERIZER_TUNING`, `vendor\egui-wgpu`).

use eframe::egui_wgpu;
use eframe::wgpu;
use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_ui::app_state::{Action, AppState, UiSettings};
use fp_ui::FirepixelApp;
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::time::Instant;

type H = Harness<'static, FirepixelApp>;

fn korpus_seite() -> Option<std::path::PathBuf> {
    let root = std::env::var_os("FP_REA_DIR").map(std::path::PathBuf::from).unwrap_or_else(|| std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join(".."));
    let p = root.join("korpus").join("First Run").join("Templates").join("Wireframes").join("iPad.png");
    p.is_file().then_some(p)
}

/// Zeichnet wie eframe in ein Ziel in Fenstergröße, ohne zurückzulesen; merkt sich die Dauer.
struct PaintOnly {
    rs: egui_wgpu::RenderState,
    renderer: egui_wgpu::Renderer,
    target: Option<(wgpu::TextureView, [u32; 2])>,
    last_ms: Arc<Mutex<f64>>,
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
                label: Some("fv8-ziel"),
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
        let Some((view, _)) = &self.target else { return Err("kein Ziel".into()) };
        let prims = ctx.tessellate(output.shapes.clone(), ppp);
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
        if let Ok(mut m) = self.last_ms.lock() {
            *m = t.elapsed().as_secs_f64() * 1000.0;
        }
        Ok(image::RgbaImage::new(1, 1))
    }
}

fn harness(path: &std::path::Path, tuning: bool) -> (H, Arc<Mutex<f64>>, String) {
    fp_engine::shell::disable();
    // vor dem Anlegen von Gerät und Renderer (wie `fp_ui::run`)
    egui_wgpu::SOFTWARE_RASTERIZER_TUNING.store(tuning, std::sync::atomic::Ordering::Relaxed);
    let rs = egui_kittest::wgpu::create_render_state(egui_kittest::wgpu::default_wgpu_setup(), Default::default());
    let info = format!("{} ({:?}), Merkmale {:?}", rs.adapter.get_info().name, rs.adapter.get_info().device_type, rs.device.features() & wgpu::Features::MAPPABLE_PRIMARY_BUFFERS);
    let renderer = egui_wgpu::Renderer::new(&rs.device, wgpu::TextureFormat::Bgra8Unorm, Default::default());
    let last_ms = Arc::new(Mutex::new(0.0));
    let paint = PaintOnly { rs, renderer, target: None, last_ms: Arc::clone(&last_ms) };
    let session = fp_ui::ui_commands::new_session(fp_engine::Preferences::new());
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, fp_engine::Session::default(), UiSettings::default(), false);
    let mut st = AppState::new(session, UiSettings::default(), None);
    st.sync_render = true;
    st.os_clipboard = false;
    st.session.execute("file.open", json!({ "path": path.to_string_lossy() })).expect("Korpusseite öffnen");
    app.state = st;
    let mut h = Harness::builder().renderer(crate::gpu_common::renderer())
        .with_size(egui::vec2(1800.0, 1000.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(20)
        .with_step_dt(1.0 / 60.0)
        .renderer(paint)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(6);
    (h, last_ms, info)
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

/// Ein Bild zeichnen; Dauer des Zeichnens in ms.
fn paint(h: &mut H, last: &Arc<Mutex<f64>>) -> f64 {
    let _ = h.render();
    last.lock().map(|m| *m).unwrap_or(0.0)
}

fn stat(v: &[f64]) -> String {
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.total_cmp(b));
    format!("min {:.1}  Median {:.1}  max {:.1}  Mittel {:.1} ms", s[0], s[s.len() / 2], s[s.len() - 1], s.iter().sum::<f64>() / s.len() as f64)
}

/// 50 Zeichen in einen neuen Text bei Zoom 400 % tippen; (Oberfläche, Zeichnen) je Zeichen.
fn tippen(path: &std::path::Path, tuning: bool) -> (Vec<f64>, Vec<f64>) {
    let (mut h, last, info) = harness(path, tuning);
    eprintln!("== Anpassung für Software-Rasterer: {} – {info}", if tuning { "an" } else { "aus" });
    let _ = h.state_mut().state.session.execute("view.setZoom", json!({ "zoom": 4.0 }));
    h.run_steps(3);
    let _ = paint(&mut h, &last);
    let idle: Vec<f64> = (0..3)
        .map(|_| {
            h.step();
            paint(&mut h, &last)
        })
        .collect();
    eprintln!("   Zeichnen ohne Eingabe: {idle:.1?} ms");
    h.state_mut().state.push(Action::SetTool(fp_engine::tools::ToolId::Text));
    h.run_steps(2);
    let area = {
        let s = &h.state().state;
        let d = s.session.active().expect("Dokument");
        s.canvases.get(&d.id).and_then(|c| c.area).expect("Leinwand")
    };
    let p = Pos2::new(area.min.x + 60.0, area.min.y + 80.0);
    h.event(Event::PointerMoved(p));
    h.event(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.run_steps(1);
    h.event(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
    let _ = paint(&mut h, &last);
    assert!(h.state().state.session.tool_is_editing_text(), "Text-Editor offen");
    let text = "Firepixel tippt fuenfzig Zeichen ohne Verzoegerung";
    assert_eq!(text.chars().count(), 50);
    let (mut ui, mut gpu) = (Vec::new(), Vec::new());
    for c in text.chars() {
        h.event(Event::Text(c.to_string()));
        let t = Instant::now();
        h.step();
        ui.push(ms(t));
        gpu.push(paint(&mut h, &last));
    }
    let buf = h.state_mut().state.session.tool_feedback().into_iter().find_map(|o| match o {
        fp_engine::Overlay::TextEdit(v) => Some(v.display_text()),
        _ => None,
    });
    assert_eq!(buf.as_deref(), Some(text), "alle Zeichen im Editor");
    let total: Vec<f64> = ui.iter().zip(&gpu).map(|(a, b)| a + b).collect();
    eprintln!("   Oberfläche je Zeichen:  {}", stat(&ui));
    eprintln!("   Zeichnen je Zeichen:    {}", stat(&gpu));
    eprintln!("   Bild je Zeichen gesamt: {}", stat(&total));
    (ui, gpu)
}

#[test]
#[ignore = "Messung (Release): cargo test -p fp-ui --release --test tippen_messung -- --ignored --nocapture"]
fn tippen_50_zeichen_auf_grosser_korpusseite() {
    let Some(path) = korpus_seite() else {
        eprintln!("Korpus fehlt – übersprungen");
        return;
    };
    let _ = tippen(&path, false);
    let (ui, gpu) = tippen(&path, true);
    // Ziel: ein Zeichen ist im nächsten Bild sichtbar (≤ 33 ms je Bild bei 30 Bildern/s)
    let mut total: Vec<f64> = ui.iter().zip(&gpu).map(|(a, b)| a + b).collect();
    total.sort_by(|a, b| a.total_cmp(b));
    eprintln!("Median je Zeichen mit Anpassung: {:.1} ms", total[total.len() / 2]);
}
