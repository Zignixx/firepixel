//! Ansichtszustand eines Dokumentfensters: Zoom, Bildlauf, Raster/Hilfslinien/Lineale (10 §2.6, §4.4).

use crate::prefs::Preferences;
use fp_geom::{Point, Rect};
use serde::{Deserialize, Serialize};

/// Die 17 Zoomstufen aus `.data 0x1b45a50` (exakte `double`-Werte, 0,16/0,33/0,66 sind keine Brüche).
pub const ZOOM_STEPS: [f64; 17] = [0.0625, 0.125, 0.16, 0.25, 0.33, 0.5, 0.66, 1.0, 1.5, 2.0, 3.0, 4.0, 6.0, 8.0, 16.0, 32.0, 64.0];

/// Kleinster und größter Zoom.
pub const MIN_ZOOM: f64 = 0.0625;
pub const MAX_ZOOM: f64 = 64.0;

/// Rand je Seite (Bildschirmpixel) bei „Alles anpassen“/„Auswahl anpassen“ (gemessen im Original).
pub const FIT_MARGIN: f64 = 15.0;

/// Abzug vom Sichtbereich (Bildschirmpixel, je Richtung insgesamt) beim Zoomwerkzeug-Rahmen
/// (gemessen im Original, PARITAET_A L25: 1462 × 822 → 1445 × 805).
pub const ZOOM_RECT_INSET: f64 = 17.0;

/// Nächste größere Zoomstufe (B `@005e4e80`): exakter Treffer → Nachbar (geklemmt), sonst die obere
/// Grenze des umschließenden Intervalls.
pub fn zoom_step_in(z: f64) -> f64 {
    if let Some(i) = ZOOM_STEPS.iter().position(|&s| s == z) {
        return ZOOM_STEPS[(i + 1).min(ZOOM_STEPS.len() - 1)];
    }
    ZOOM_STEPS.iter().copied().find(|&s| s > z).unwrap_or(MAX_ZOOM)
}

/// Nächste kleinere Zoomstufe.
pub fn zoom_step_out(z: f64) -> f64 {
    if let Some(i) = ZOOM_STEPS.iter().position(|&s| s == z) {
        return ZOOM_STEPS[i.saturating_sub(1)];
    }
    ZOOM_STEPS.iter().rev().copied().find(|&s| s < z).unwrap_or(MIN_ZOOM)
}

/// Darstellungsart des Dokumentfensters (Reiter Original/Vorschau/2-fach/4-fach).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PreviewMode {
    #[default]
    Original,
    Preview,
    TwoUp,
    FourUp,
}

/// Ansichtszustand.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ViewState {
    /// Zoomfaktor (1 = 100 %).
    pub zoom: f64,
    /// Dokumentpunkt an der linken oberen Ecke des Sichtbereichs.
    pub origin: (f64, f64),
    /// Größe des Sichtbereichs in Bildschirmpixeln (von der UI gemeldet; Vorgabe 1000 × 700).
    pub viewport: (f64, f64),
    pub show_grid: bool,
    pub snap_to_grid: bool,
    pub show_guides: bool,
    pub lock_guides: bool,
    pub snap_to_guides: bool,
    pub show_smart_guides: bool,
    pub snap_to_smart_guides: bool,
    pub show_rulers: bool,
    pub show_slice_guides: bool,
    pub show_slice_overlay: bool,
    /// „Ränder“ (Strg+H): Auswahlränder zeigen.
    pub show_edges: bool,
    /// Ansicht ▸ Gamma 1.8.
    pub gamma_preview: bool,
    pub lock_nine_slice_guides: bool,
    /// Segmente und Hotspots ausgeblendet (Werkzeug „Segmente aus-/einblenden“).
    pub hide_slices: bool,
    pub preview_mode: PreviewMode,
}

impl Default for ViewState {
    fn default() -> Self {
        ViewState::from_prefs(&Preferences::new())
    }
}

impl ViewState {
    /// Anfangszustand aus den Voreinstellungen.
    pub fn from_prefs(p: &Preferences) -> ViewState {
        ViewState {
            zoom: 1.0,
            origin: (0.0, 0.0),
            viewport: (1000.0, 700.0),
            show_grid: p.get_bool("ShowGrid", false),
            snap_to_grid: p.get_bool("SnapToGrids", false),
            show_guides: p.get_bool("ShowGuides", true),
            lock_guides: p.get_bool("LockGuides", false),
            snap_to_guides: p.get_bool("SnapToGuides", true),
            show_smart_guides: p.get_bool("ShowSmartGuides", true),
            snap_to_smart_guides: p.get_bool("SnapToSmartGuides", true),
            show_rulers: p.get_bool("NewWindowsShowRulers", false),
            show_slice_guides: p.get_bool("SliceGuidesVisible", true),
            show_slice_overlay: true,
            show_edges: true,
            gamma_preview: false,
            lock_nine_slice_guides: false,
            hide_slices: false,
            preview_mode: PreviewMode::Original,
        }
    }

    /// Bildschirm → Dokument.
    pub fn screen_to_doc(&self, x: f64, y: f64) -> Point {
        let z = self.zoom.max(1e-9);
        Point::new((self.origin.0 + x / z) as f32, (self.origin.1 + y / z) as f32)
    }

    /// Dokument → Bildschirm.
    pub fn doc_to_screen(&self, p: Point) -> (f64, f64) {
        ((p.x as f64 - self.origin.0) * self.zoom, (p.y as f64 - self.origin.1) * self.zoom)
    }

    /// Setzt den Zoom so, dass der Bildschirmpunkt `anchor` (Vorgabe: Mitte) auf demselben
    /// Dokumentpunkt bleibt.
    pub fn set_zoom(&mut self, zoom: f64, anchor: Option<(f64, f64)>) {
        let z = if zoom.is_finite() { zoom.clamp(MIN_ZOOM, MAX_ZOOM) } else { 1.0 };
        let (ax, ay) = anchor.unwrap_or((self.viewport.0 / 2.0, self.viewport.1 / 2.0));
        let p = self.screen_to_doc(ax, ay);
        self.zoom = z;
        self.origin = (p.x as f64 - ax / z, p.y as f64 - ay / z);
    }

    /// Größer zoomen.
    pub fn zoom_in(&mut self, anchor: Option<(f64, f64)>) {
        self.set_zoom(zoom_step_in(self.zoom), anchor);
    }

    /// Kleiner zoomen.
    pub fn zoom_out(&mut self, anchor: Option<(f64, f64)>) {
        self.set_zoom(zoom_step_out(self.zoom), anchor);
    }

    /// Rechteck einpassen (Alles/Auswahl anpassen): größte Zoomstufe der Tabelle, bei der das Rechteck
    /// in den Sichtbereich passt, dann zentrieren.
    pub fn fit_rect(&mut self, r: Rect) {
        if r.is_empty() || !r.left.is_finite() {
            return;
        }
        let (vw, vh) = (self.viewport.0.max(1.0), self.viewport.1.max(1.0));
        let fit = (vw / r.width() as f64).min(vh / r.height() as f64);
        let z = ZOOM_STEPS.iter().rev().copied().find(|&s| s <= fit).unwrap_or(MIN_ZOOM);
        self.zoom = z;
        let c = (((r.left + r.right) / 2.0) as f64, ((r.top + r.bottom) / 2.0) as f64);
        self.origin = (c.0 - vw / 2.0 / z, c.1 - vh / 2.0 / z);
    }

    /// Alles anpassen / Auswahl anpassen wie im Original: stufenloser Zoom, bei dem das Rechteck mit
    /// je 15 Bildschirmpixeln Rand in den Sichtbereich passt (gemessen: 600×400 in 2222×1132 →
    /// 275,5 %, 1000×200 → 219,2 %, 50×30 → 3673 %), begrenzt auf 6,25 %–6400 %, dann zentriert.
    pub fn fit_rect_exact(&mut self, r: Rect) {
        if r.is_empty() || !r.left.is_finite() {
            return;
        }
        let (vw, vh) = (self.viewport.0.max(1.0), self.viewport.1.max(1.0));
        let m = 2.0 * FIT_MARGIN;
        let fit = ((vw - m).max(1.0) / r.width() as f64).min((vh - m).max(1.0) / r.height() as f64);
        let z = fit.clamp(MIN_ZOOM, MAX_ZOOM);
        self.zoom = z;
        let c = (((r.left + r.right) / 2.0) as f64, ((r.top + r.bottom) / 2.0) as f64);
        self.origin = (c.0 - vw / 2.0 / z, c.1 - vh / 2.0 / z);
    }

    /// Zoomwerkzeug-Rahmen wie im Original: stufenlos so, dass das Rechteck in den Sichtbereich
    /// abzüglich [`ZOOM_RECT_INSET`] je Richtung passt, begrenzt auf 6,25 %–6400 %, dann zentriert.
    /// Gemessen (PARITAET_A L25, Sichtbereich 1462 × 822): 100 × 70 → 1150 %, 200 × 70 → 722,5 %,
    /// 100 × 20 → 1445 %, also Einpassen in 1445 × 805.
    pub fn fit_zoom_rect(&mut self, r: Rect) {
        if r.is_empty() || !r.left.is_finite() {
            return;
        }
        let (vw, vh) = (self.viewport.0.max(1.0), self.viewport.1.max(1.0));
        let fit = ((vw - ZOOM_RECT_INSET).max(1.0) / r.width() as f64).min((vh - ZOOM_RECT_INSET).max(1.0) / r.height() as f64);
        let z = fit.clamp(MIN_ZOOM, MAX_ZOOM);
        self.zoom = z;
        let c = (((r.left + r.right) / 2.0) as f64, ((r.top + r.bottom) / 2.0) as f64);
        self.origin = (c.0 - vw / 2.0 / z, c.1 - vh / 2.0 / z);
    }

    /// Ansicht verschieben (Hand-Werkzeug), Δ in Bildschirmpixeln.
    pub fn pan(&mut self, dx: f64, dy: f64) {
        let z = self.zoom.max(1e-9);
        self.origin.0 -= dx / z;
        self.origin.1 -= dy / z;
    }

    /// Sichtbarer Dokumentbereich.
    pub fn visible_rect(&self) -> Rect {
        let a = self.screen_to_doc(0.0, 0.0);
        let b = self.screen_to_doc(self.viewport.0, self.viewport.1);
        Rect::new(a.x, a.y, b.x, b.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_steps_like_original() {
        assert_eq!(zoom_step_in(1.0), 1.5);
        assert_eq!(zoom_step_out(1.0), 0.66);
        assert_eq!(zoom_step_in(0.25), 0.33);
        assert_eq!(zoom_step_in(64.0), 64.0);
        assert_eq!(zoom_step_out(0.0625), 0.0625);
        assert_eq!(zoom_step_in(1.2), 1.5);
        assert_eq!(zoom_step_out(1.2), 1.0);
        let mut v = ViewState { viewport: (500.0, 500.0), ..ViewState::default() };
        v.fit_rect(Rect::new(0.0, 0.0, 1000.0, 1000.0));
        assert_eq!(v.zoom, 0.5);
        let p = v.screen_to_doc(250.0, 250.0);
        assert_eq!((p.x, p.y), (500.0, 500.0));
    }

    #[test]
    fn fit_exact_like_original() {
        // Messwerte aus dem Original (Sichtbereich 2222 × 1132)
        let mut v = ViewState { viewport: (2222.0, 1132.0), ..ViewState::default() };
        v.fit_rect_exact(Rect::new(0.0, 0.0, 600.0, 400.0));
        assert!((v.zoom - 2.755).abs() < 1e-6, "{}", v.zoom);
        v.fit_rect_exact(Rect::new(0.0, 0.0, 1000.0, 200.0));
        assert!((v.zoom - 2.192).abs() < 1e-6, "{}", v.zoom);
        v.fit_rect_exact(Rect::new(10.0, 10.0, 60.0, 40.0));
        assert!((v.zoom - 1102.0 / 30.0).abs() < 1e-6, "{}", v.zoom);
        // Mitte des Rechtecks in der Mitte des Sichtbereichs
        let c = v.screen_to_doc(1111.0, 566.0);
        assert!((c.x - 35.0).abs() < 1e-3 && (c.y - 25.0).abs() < 1e-3, "{c:?}");
        // danach „Größer zoomen“ = nächste Tabellenstufe (gemessen: 2,192 → 3)
        v.fit_rect_exact(Rect::new(0.0, 0.0, 1000.0, 200.0));
        v.zoom_in(None);
        assert_eq!(v.zoom, 3.0);
        // sehr kleines Rechteck: höchstens 6400 %
        v.fit_rect_exact(Rect::new(0.0, 0.0, 1.0, 1.0));
        assert_eq!(v.zoom, MAX_ZOOM);
    }
}
