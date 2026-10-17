//! Schnittstelle zum Renderer für das flache Bild (`IDAT`) und die Vorschau (`prVW`).
//!
//! Der Dokument-Compositor kommt erst in Phase 2 (`fp-render`). Bis dahin übernimmt der Schreiber das
//! flache Bild der Originaldatei bzw. erzeugt ein leeres Bild in Leinwandfarbe.

use fp_model::{Document, FlatImage};

/// Rendert Seiten eines Dokuments.
pub trait Flattener {
    /// Seite `page` in Dokumentgröße (RGBA8, nicht vormultipliziert) für `IDAT`; Leinwandfarbe
    /// eingerechnet (bei transparenter Leinwand bleibt das Bild transparent). `None` = nicht verfügbar.
    fn flatten(&self, doc: &Document, page: usize) -> Option<FlatImage>;

    /// Seite `page` mit Faktor `scale` gerendert, Ergebnis `width`×`height` (02 §12.8: Ursprung
    /// `(⌊L·s2⌋, ⌊T·s2⌋)`, Bitmaps per nächstem Nachbarn, Vektoren neu gerastert). `None` = nicht verfügbar.
    fn render_scaled(
        &self,
        doc: &Document,
        page: usize,
        scale: f64,
        width: u32,
        height: u32,
    ) -> Option<FlatImage>;
}

/// Kein Renderer: liefert immer `None` (der Schreiber benutzt dann Rückfallwege).
#[derive(Clone, Copy, Debug, Default)]
pub struct NoFlattener;

impl Flattener for NoFlattener {
    fn flatten(&self, _doc: &Document, _page: usize) -> Option<FlatImage> {
        None
    }
    fn render_scaled(
        &self,
        _doc: &Document,
        _page: usize,
        _scale: f64,
        _w: u32,
        _h: u32,
    ) -> Option<FlatImage> {
        None
    }
}
