//! Fensterstart ohne Flackern: Das Hauptfenster erscheint genau einmal, fertig gezeichnet und
//! gleich in voller Arbeitsbereichsgröße (Nutzerbefund N09).
//!
//! eframe erzeugt das Fenster unsichtbar und zeigt es nach dem ersten gezeichneten Frame. Dabei
//! gab es drei sichtbare Störungen (gemessen mit schnellen Bildschirmfotos, Software-Grafik):
//! 1. winit maximiert unter Windows mit `ShowWindow(SW_MAXIMIZE)`, das ein unsichtbares Fenster
//!    sichtbar macht und danach wieder versteckt (sichtbar → unsichtbar → sichtbar, auch der
//!    Taskleistenknopf). Deshalb wird das Maximieren beim Erzeugen unterdrückt
//!    ([`strip_maximized`]) und erst nach dem Erscheinen nachgeholt ([`after_frame`]).
//! 2. Das erste sichtbare Bild ist weiß (der im Verborgenen gezeichnete Frame wird nicht
//!    angezeigt), bis der nächste Frame fertig ist.
//! 3. Etwa 0,2–0,5 s danach zeigt die Grafik einige Bilder lang einen vergrößerten Ausschnitt
//!    (Swapchain noch in der Größe vom Erzeugen), dann wieder das richtige Bild.
//!
//! Gegen 2 und 3 wird das Fenster unsichtbar außerhalb aller Bildschirme vorbereitet
//! ([`prepare_hidden`]) und erst an seinen Platz gelegt, wenn es dort einige Frames lang ruhig
//! gezeichnet hat ([`after_frame`]); im selben Schritt wird es maximiert. Das Fenster hat keinen
//! Systemrahmen und liegt schon auf der Fläche des maximierten Fensters, darum sieht man diesen
//! letzten Schritt nicht.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, Ordering};
use std::sync::Mutex;

/// Soll das Fenster nach dem Erscheinen maximiert werden (aus Vorgabe oder gespeicherter Sitzung)?
static START_MAXIMIZED: AtomicBool = AtomicBool::new(false);
/// Äußere Lage, an die das außerhalb vorbereitete Fenster gehört (physische Pixel).
static TARGET: Mutex<Option<(i32, i32)>> = Mutex::new(None);

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

/// Linke Kante rechts neben allen Bildschirmen (physische Pixel), mit Abstand.
fn offscreen_x(window: &winit::window::Window) -> Option<i32> {
    window.available_monitors().map(|m| m.position().x + m.size().width as i32).max().map(|r| r + 64)
}

/// Noch unsichtbar (vor dem ersten Frame): Zielfläche bestimmen und das Fenster außerhalb aller
/// Bildschirme ablegen. Beim maximierten Start ist die Zielfläche die des maximierten Fensters:
/// war die letzte Sitzung maximiert, ihre gespeicherte Innenfläche (= Arbeitsbereich des
/// Monitors), sonst der ganze Monitor. Sonst bleibt die von eframe gesetzte Lage das Ziel.
pub fn prepare_hidden(window: &winit::window::Window, storage: Option<&dyn eframe::Storage>) {
    let target = if START_MAXIMIZED.load(Ordering::Relaxed) {
        let stored: Option<Stored> = storage.and_then(|s| eframe::get_value(s, "window"));
        if let Some(Stored { inner_position_pixels: Some(p), inner_size_points: Some(_), maximized: true }) = stored {
            // eframe legt das Fenster an die äußere Lage des maximierten Fensters (Rahmen außerhalb
            // des Bildschirms); ohne Rahmen ist außen = innen, also die Innenlage nehmen.
            Some((p.x as i32, p.y as i32))
        } else if let Some(m) = window.current_monitor() {
            let _ = window.request_inner_size(m.size());
            Some((m.position().x, m.position().y))
        } else {
            None
        }
    } else {
        window.outer_position().ok().map(|p| (p.x, p.y))
    };
    let Some((x, y)) = target else { return };
    if let Ok(mut t) = TARGET.lock() {
        *t = Some((x, y));
    }
    match offscreen_x(window) {
        Some(off) => window.set_outer_position(winit::dpi::PhysicalPosition::new(off, y)),
        None => window.set_outer_position(winit::dpi::PhysicalPosition::new(x, y)),
    }
}

/// Startphase: 0 = noch unsichtbar, 3 = sichtbar außerhalb (beruhigt sich), 1 = am Platz
/// (Folge-Frame für die Symbole ausstehend), 2 = fertig.
static PHASE: AtomicU8 = AtomicU8::new(0);
/// Zeitpunkt des Sichtbarwerdens bzw. des Platzierens (`InputState::time`, f64-Bits).
static SHOWN_AT: AtomicU64 = AtomicU64::new(0);
/// Frames seit dem Sichtbarwerden (außerhalb).
static FRAMES: AtomicU32 = AtomicU32::new(0);
/// Startzeit-Messung (`FP_STARTUP_TRACE`): erster Frame nach dem Platzieren schon gemeldet?
static INTERACTIVE_MARKED: AtomicBool = AtomicBool::new(false);
/// So lange (und mindestens [`SETTLE_FRAMES`] Frames) zeichnet das Fenster außerhalb, bevor es an
/// seinen Platz kommt. Gemessen (Software-Grafik, Debug-Build): weißes erstes Bild ≈ 0,1 s,
/// vergrößerter Ausschnitt bis ≈ 0,45 s nach dem Sichtbarwerden. Im Release-Build sind die Frames
/// 4- bis 5-mal schneller (≈ 60 ms statt 250 ms); 0,5 s deckt die Debug-Messung weiter ab und
/// zeigt das Fenster 0,2 s früher als die frühere Vorgabe 0,7 s (N30, START-1).
const SETTLE: f64 = 0.5;
const SETTLE_FRAMES: u32 = 4;
/// Abstand des Folge-Frames nach dem Platzieren (Fenstersymbole erneut setzen). Gemessen
/// (Software-Grafik, 2560 × 1392): Die ersten Frames nach dem Erscheinen brauchen je etwa 250 ms
/// und laufen etwa 1 s lang ohne Pause; danach antwortet der Thread sofort.
const FOLLOW_UP: f64 = 1.5;

/// Nach jedem Frame. Sobald eframe das Fenster (außerhalb) gezeigt hat: weiterzeichnen, bis es
/// sich beruhigt hat, dann an seinen Platz legen und maximieren. Gibt `true` zurück, wenn die
/// Fenstersymbole jetzt (erneut) gesetzt werden sollen: direkt nach dem Sichtbarwerden, beim
/// Platzieren und einmal 1,5 s später (die Taskleiste legt ihren Knopf beim Sichtbarwerden an).
pub fn after_frame(ctx: &egui::Context, window: Option<&winit::window::Window>) -> bool {
    let Some(w) = window else {
        PHASE.store(2, Ordering::Relaxed);
        return false;
    };
    let now = ctx.input(|i| i.time);
    match PHASE.load(Ordering::Relaxed) {
        0 if w.is_visible() == Some(true) => {
            PHASE.store(3, Ordering::Relaxed);
            fp_i18n::startup::mark("window shown (offscreen, settling)");
            SHOWN_AT.store(now.to_bits(), Ordering::Relaxed);
            FRAMES.store(0, Ordering::Relaxed);
            ctx.request_repaint();
            true
        }
        0 => {
            // eframe zeigt das Fenster am Ende des ersten Frames; danach noch ein Frame
            ctx.request_repaint();
            false
        }
        3 => {
            let n = FRAMES.fetch_add(1, Ordering::Relaxed) + 1;
            let since = now - f64::from_bits(SHOWN_AT.load(Ordering::Relaxed));
            if since < SETTLE || n < SETTLE_FRAMES {
                ctx.request_repaint();
                return false;
            }
            if let Some((x, y)) = TARGET.lock().ok().and_then(|t| *t) {
                w.set_outer_position(winit::dpi::PhysicalPosition::new(x, y));
            }
            if START_MAXIMIZED.swap(false, Ordering::Relaxed) {
                w.set_maximized(true);
            }
            // Testinstanzen (FP_NO_SHELL_OPEN, FP_TEST_PROFILE …) holen sich den Fokus nicht: winit
            // erzwingt ihn sonst mit einem simulierten Alt-Tastendruck (SendInput), der im gerade
            // aktiven fremden Fenster landet (AGENTEN_REGELN §4d/§4e, START-1)
            if !fp_i18n::profile::is_test_run() {
                w.focus_window();
            }
            PHASE.store(1, Ordering::Relaxed);
            fp_i18n::startup::mark("window placed (visible to the user)");
            SHOWN_AT.store(now.to_bits(), Ordering::Relaxed);
            ctx.request_repaint();
            true
        }
        1 => {
            if !INTERACTIVE_MARKED.swap(true, Ordering::Relaxed) {
                fp_i18n::startup::mark("first frame after placement (interactive)");
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// N09: Das Fenster wird nie maximiert erzeugt (winit würde es dabei kurz zeigen und wieder
    /// verstecken); „maximiert“ wird gemerkt und nach dem Erscheinen nachgeholt.
    #[test]
    fn maximized_is_stripped_and_remembered() {
        let b = strip_maximized(egui::ViewportBuilder::default().with_maximized(true));
        assert_eq!(b.maximized, Some(false), "nicht maximiert erzeugen");
        assert!(START_MAXIMIZED.load(Ordering::Relaxed), "für nach dem Erscheinen gemerkt");
        // Vollbild hat Vorrang: dann nichts nachholen
        let b = strip_maximized(egui::ViewportBuilder::default().with_maximized(true).with_fullscreen(true));
        assert_eq!(b.maximized, Some(true));
        assert!(!START_MAXIMIZED.load(Ordering::Relaxed));
    }

    /// Ohne natives Fenster (Tests, Bildschirmfotos) gibt es keine Startphase.
    #[test]
    fn without_window_nothing_happens() {
        let ctx = egui::Context::default();
        assert!(!after_frame(&ctx, None));
        assert_eq!(PHASE.load(Ordering::Relaxed), 2);
        assert!(!after_frame(&ctx, None), "auch nicht beim nächsten Frame");
    }

    /// Fensterlage/Arbeitsbereich liegen unter %APPDATA%\Firepixel\data\app.ron (wie die
    /// Voreinstellungen), damit Testinstanzen mit eigenem APPDATA die des Nutzers nicht anfassen.
    #[test]
    fn window_state_follows_appdata() {
        let Some(base) = fp_i18n::profile::roaming_dir() else { return };
        let o = crate::native_options();
        let want = base.join("data").join("app.ron");
        assert_eq!(o.persistence_path.as_deref(), Some(want.as_path()));
        // im Test nie die Fensterlage des Nutzers (EB-3)
        assert!(!fp_i18n::profile::is_real(&want), "{want:?}");
        assert!(o.persist_window);
    }

    /// Die Wartezeit außerhalb deckt die gemessenen Störungen ab (weißes Bild, vergrößerter
    /// Ausschnitt bis ≈ 0,45 s) und hält den Start trotzdem kurz.
    #[test]
    fn settle_covers_measured_glitches() {
        const { assert!(SETTLE >= 0.5 && SETTLE <= 1.0) };
        const { assert!(SETTLE_FRAMES >= 3) };
    }
}
