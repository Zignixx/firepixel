//! Fensterstart ohne Flackern: Das Hauptfenster erscheint genau einmal, gleich in voller
//! Arbeitsbereichsgröße.
//!
//! eframe erzeugt das Fenster unsichtbar und zeigt es nach dem ersten gezeichneten Frame. winit
//! maximiert unter Windows aber mit `ShowWindow(SW_MAXIMIZE)`, das ein unsichtbares Fenster
//! sichtbar macht, und versteckt es danach wieder. Ergebnis war ein kurzer weißer Blitz
//! (sichtbar → unsichtbar → sichtbar). Deshalb wird das Maximieren beim Erzeugen unterdrückt
//! ([`strip_maximized`]), das noch unsichtbare Fenster auf die Fläche des maximierten Fensters
//! gelegt ([`prepare_hidden`]) und erst nach dem Erscheinen maximiert ([`after_frame`]). Das
//! Fenster hat keinen Systemrahmen, darum sieht man diesen letzten Schritt nicht.

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};

/// Soll das Fenster nach dem Erscheinen maximiert werden (aus Vorgabe oder gespeicherter Sitzung)?
static START_MAXIMIZED: AtomicBool = AtomicBool::new(false);

/// `NativeOptions::window_builder`: merkt sich „maximiert“ und erzeugt das Fenster unmaximiert.
pub fn strip_maximized(builder: egui::ViewportBuilder) -> egui::ViewportBuilder {
    let max = builder.maximized.unwrap_or(false) && !builder.fullscreen.unwrap_or(false);
    START_MAXIMIZED.store(max, Ordering::Relaxed);
    if max { builder.with_maximized(false) } else { builder }
}

/// Von eframe gespeicherte Fensterlage (Schlüssel „window“); nur die hier gebrauchten Felder.
#[derive(serde::Deserialize)]
struct Stored {
    inner_position_pixels: Option<Xy>,
    inner_size_points: Option<Xy>,
    #[serde(default)]
    maximized: bool,
}

#[derive(serde::Deserialize)]
struct Xy {
    x: f32,
    y: f32,
}

/// Noch unsichtbar (vor dem ersten Frame): das Fenster deckungsgleich auf die Fläche legen, die es
/// maximiert einnimmt. War die letzte Sitzung maximiert, ist das ihre gespeicherte Innenfläche
/// (= Arbeitsbereich des Monitors); sonst der ganze Monitor.
pub fn prepare_hidden(window: &winit::window::Window, storage: Option<&dyn eframe::Storage>) {
    if !START_MAXIMIZED.load(Ordering::Relaxed) {
        return;
    }
    let stored: Option<Stored> = storage.and_then(|s| eframe::get_value(s, "window"));
    if let Some(Stored { inner_position_pixels: Some(p), inner_size_points: Some(_), maximized: true }) = stored {
        // eframe legt das Fenster an die äußere Lage des maximierten Fensters (Rahmen außerhalb des
        // Bildschirms); ohne Rahmen ist außen = innen, also die Innenlage nehmen.
        window.set_outer_position(winit::dpi::PhysicalPosition::new(p.x as i32, p.y as i32));
    } else if let Some(m) = window.current_monitor() {
        window.set_outer_position(m.position());
        let _ = window.request_inner_size(m.size());
    }
}

/// Startphase: 0 = noch unsichtbar, 1 = sichtbar (Folge-Frame ausstehend), 2 = fertig.
static PHASE: AtomicU8 = AtomicU8::new(0);
/// Zeitpunkt des Sichtbarwerdens (`InputState::time`, f64-Bits).
static SHOWN_AT: AtomicU64 = AtomicU64::new(0);
/// Abstand des Folge-Frames nach dem Sichtbarwerden. Gemessen (Software-Grafik, 2560 × 1392):
/// Die ersten Frames nach dem Erscheinen brauchen je etwa 250 ms und laufen etwa 1 s lang ohne
/// Pause; danach antwortet der Thread sofort. Das erneute Setzen fällt in diese ruhige Phase.
const FOLLOW_UP: f64 = 1.5;

/// Nach jedem Frame. Sobald eframe das Fenster gezeigt hat (nach dem ersten gezeichneten Frame,
/// der UI-Thread reagiert also): einmal maximieren. Gibt `true` zurück, wenn die Fenstersymbole
/// jetzt (erneut) gesetzt werden sollen: direkt nach dem Sichtbarwerden und einmal 1,5 s später
/// (die Taskleiste legt ihren Knopf beim Sichtbarwerden an).
pub fn after_frame(ctx: &egui::Context, window: Option<&winit::window::Window>) -> bool {
    let Some(w) = window else {
        PHASE.store(2, Ordering::Relaxed);
        return false;
    };
    let now = ctx.input(|i| i.time);
    match PHASE.load(Ordering::Relaxed) {
        0 if w.is_visible() == Some(true) => {
            if START_MAXIMIZED.swap(false, Ordering::Relaxed) {
                w.set_maximized(true);
            }
            PHASE.store(1, Ordering::Relaxed);
            SHOWN_AT.store(now.to_bits(), Ordering::Relaxed);
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(FOLLOW_UP));
            true
        }
        0 => {
            // eframe zeigt das Fenster am Ende des ersten Frames; danach noch ein Frame
            ctx.request_repaint();
            false
        }
        1 => {
            let left = f64::from_bits(SHOWN_AT.load(Ordering::Relaxed)) + FOLLOW_UP - now;
            if left <= 0.0 {
                PHASE.store(2, Ordering::Relaxed);
                true
            } else {
                ctx.request_repaint_after(std::time::Duration::from_secs_f64(left));
                false
            }
        }
        _ => false,
    }
}
