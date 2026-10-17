//! K-2: one GPU at a time in the shared test program `all` (BEFUNDE `[N]` NZ-2 „wgpu-Renderer
//! parallel“, GT-0k finding 3). egui_kittest creates a wgpu renderer (instance + device) per harness
//! on the first `render()`; many at once ended the program with `STATUS_ACCESS_VIOLATION`, now and
//! then even with `--test-threads 4`. NZ-2 measured: with a lock around image and teardown 0/30.
//!
//! [`renderer`] wraps the default lazy wgpu renderer: creating it, texture uploads, `render()` and
//! dropping it (device teardown) run under one process-wide lock. Everything else of the tests
//! stays parallel, no thread limit needed.
//!
//! Use: `Harness::builder().renderer(crate::gpu_common::renderer())` in every test file that
//! renders images (`h.render()`, snapshots).

use egui::TexturesDelta;
use egui_kittest::{LazyRenderer, TestRenderer};
use std::sync::{Mutex, MutexGuard};

static GPU: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    // a panicking test (failed assertion while rendering) must not block all others
    GPU.lock().unwrap_or_else(|e| e.into_inner())
}

/// The default renderer of egui_kittest, GPU work serialized (see module text).
pub struct GuardedRenderer(Option<LazyRenderer>);

/// New guarded renderer for `HarnessBuilder::renderer`.
pub fn renderer() -> GuardedRenderer {
    GuardedRenderer(Some(LazyRenderer::default()))
}

impl TestRenderer for GuardedRenderer {
    fn handle_delta(&mut self, delta: &mut TexturesDelta) {
        let _g = lock();
        if let Some(r) = self.0.as_mut() {
            r.handle_delta(delta);
        }
    }

    fn render(&mut self, ctx: &egui::Context, output: &egui::FullOutput) -> Result<image::RgbaImage, String> {
        let _g = lock();
        match self.0.as_mut() {
            Some(r) => r.render(ctx, output),
            None => Err("renderer dropped".into()),
        }
    }
}

impl Drop for GuardedRenderer {
    fn drop(&mut self) {
        let _g = lock();
        drop(self.0.take());
    }
}
