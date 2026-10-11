//! Offscreen-Rendering für Agenten (MCP `render`, CLI `render`): Seite, Ausschnitt oder einzelnes
//! Element als PNG, mit Zoom. Vergrößern tastet je Gerätepixel ab (scharfe Pixel wie in der
//! Leinwand), Verkleinern mittelt Flächen (gut lesbare Vorschau).

use crate::image::encode_png;
use fp_engine::DocumentState;
use fp_geom::{IRect, Rect};
use fp_raster::RgbaImage;
use fp_render::{PointMap, Renderer};
use serde_json::{json, Value};

/// Was gerendert werden soll.
#[derive(Clone, Debug)]
pub struct RenderRequest {
    /// Seite (Vorgabe: aktuelle Seite).
    pub page: Option<usize>,
    /// Status (Vorgabe: aktueller Status).
    pub state: Option<usize>,
    /// Zoomfaktor (1 = 100 %).
    pub zoom: f64,
    /// Ausschnitt in Dokumentkoordinaten (Vorgabe: ganze Leinwand).
    pub rect: Option<Rect>,
    /// Nur dieses Element (über transparentem Grund, Grenzen = Aussehen).
    pub element: Option<u64>,
    /// Größte erlaubte Ausgabe in Pixeln; größere Anfragen werden verkleinert.
    pub max_pixels: u64,
}

impl Default for RenderRequest {
    fn default() -> Self {
        RenderRequest { page: None, state: None, zoom: 1.0, rect: None, element: None, max_pixels: 16_000_000 }
    }
}

/// Ergebnis: PNG und Metadaten.
#[derive(Clone, Debug)]
pub struct Rendered {
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
    /// Linke obere Ecke des Bilds in Dokumentkoordinaten.
    pub origin: (i32, i32),
    /// Tatsächlicher Zoom (nach Begrenzung).
    pub zoom: f64,
    /// Hinweise (Begrenzung, fehlende Teile des Renderers).
    pub notes: Vec<String>,
}

impl Rendered {
    /// Metadaten als JSON (ohne Bilddaten).
    pub fn meta(&self) -> Value {
        json!({"width": self.width, "height": self.height, "origin": {"x": self.origin.0, "y": self.origin.1}, "zoom": self.zoom, "bytes": self.png.len(), "notes": self.notes})
    }
}

/// Rendert nach `req` und kodiert als PNG.
pub fn render(renderer: &Renderer, d: &DocumentState, req: &RenderRequest) -> Result<Rendered, String> {
    let doc = &d.doc;
    let page = req.page.unwrap_or(d.page);
    if page >= doc.pages.len() {
        return Err(format!("Seite {page} gibt es nicht (Dokument hat {} Seiten)", doc.pages.len()));
    }
    let frames = doc.pages[page].root.frame_count.max(1) as usize;
    let state = req.state.unwrap_or(if page == d.page { d.frame } else { 0 });
    if state >= frames {
        return Err(format!("Status {state} gibt es nicht (Seite hat {frames} Status)"));
    }
    let mut zoom = if req.zoom.is_finite() && req.zoom > 0.0 { req.zoom.clamp(1.0 / 64.0, 64.0) } else { 1.0 };
    let mut notes = Vec::new();
    let (base, origin) = match req.element {
        Some(id) => {
            let e = fp_engine::docops::element_on_page(doc, page, fp_model::ElementId(id)).ok_or_else(|| format!("Element {id} nicht auf Seite {page}"))?;
            let out = renderer.render_element(doc, &e, &PointMap::identity(), state);
            if !out.report.is_complete() {
                notes.push(format!("Renderer: {:?}", out.report));
            }
            if out.image.is_empty() {
                return Err(format!("Element {id} hat kein sichtbares Aussehen"));
            }
            let b = out.image.bounds();
            (out.image, (b.left, b.top))
        }
        None => {
            let region = match req.rect {
                Some(r) => r.normalized().round_out(),
                None => Renderer::page_rect(doc, page).ok_or("Seite ohne Leinwand")?,
            };
            if region.width() == 0 || region.height() == 0 {
                return Err("leerer Ausschnitt".into());
            }
            let out = renderer.render_rect(doc, page, state, region);
            if !out.report.is_complete() {
                notes.push(format!("Renderer: {:?}", out.report));
            }
            (out.image, (region.left, region.top))
        }
    };
    let (w0, h0) = (base.width() as f64, base.height() as f64);
    let want = (w0 * zoom).ceil() * (h0 * zoom).ceil();
    if want > req.max_pixels as f64 && want > 0.0 {
        let z2 = zoom * (req.max_pixels as f64 / want).sqrt();
        notes.push(format!("Zoom von {zoom} auf {z2:.4} verkleinert (höchstens {} Pixel)", req.max_pixels));
        zoom = z2;
    }
    let img = scale(&base, zoom);
    let png = encode_png(&img)?;
    Ok(Rendered { png, width: img.width() as u32, height: img.height() as u32, origin, zoom, notes })
}

/// Skaliert ein Bild: `zoom > 1` nächster Nachbar je Gerätepixelmitte, `zoom < 1` Flächenmittel.
pub fn scale(src: &RgbaImage, zoom: f64) -> RgbaImage {
    let (sw, sh) = (src.width(), src.height());
    if (zoom - 1.0).abs() < 1e-9 || sw == 0 || sh == 0 {
        let mut c = src.clone();
        c.set_origin(0, 0);
        return c;
    }
    let w = ((sw as f64) * zoom).ceil().max(1.0) as i32;
    let h = ((sh as f64) * zoom).ceil().max(1.0) as i32;
    let Ok(mut out) = RgbaImage::new(IRect::new(0, 0, w, h)) else {
        return RgbaImage::empty_at(0, 0);
    };
    let ob = src.bounds();
    let data = src.data();
    let px = |x: usize, y: usize| -> [u8; 4] { data.get(y * sw + x).copied().unwrap_or([0, 0, 0, 0]) };
    let ow = w as usize;
    let od = out.data_mut();
    if zoom > 1.0 {
        for j in 0..h as usize {
            let sy = (((j as f64) + 0.5) / zoom).floor().min((sh - 1) as f64) as usize;
            for i in 0..ow {
                let sx = (((i as f64) + 0.5) / zoom).floor().min((sw - 1) as f64) as usize;
                od[j * ow + i] = px(sx, sy);
            }
        }
    } else {
        // Flächenmittel mit Alpha-Gewichtung (vormultipliziert mitteln)
        for j in 0..h as usize {
            let y0 = ((j as f64) / zoom).floor() as usize;
            let y1 = ((((j + 1) as f64) / zoom).ceil() as usize).clamp(y0 + 1, sh);
            for i in 0..ow {
                let x0 = ((i as f64) / zoom).floor() as usize;
                let x1 = ((((i + 1) as f64) / zoom).ceil() as usize).clamp(x0 + 1, sw);
                let (mut r, mut g, mut b, mut a, mut n) = (0u64, 0u64, 0u64, 0u64, 0u64);
                for y in y0..y1.min(sh) {
                    for x in x0..x1.min(sw) {
                        let p = px(x, y);
                        let al = u64::from(p[3]);
                        r += u64::from(p[0]) * al;
                        g += u64::from(p[1]) * al;
                        b += u64::from(p[2]) * al;
                        a += al;
                        n += 1;
                    }
                }
                od[j * ow + i] = if a == 0 || n == 0 {
                    [0, 0, 0, 0]
                } else {
                    [((r + a / 2) / a) as u8, ((g + a / 2) / a) as u8, ((b + a / 2) / a) as u8, ((a + n / 2) / n) as u8]
                };
            }
        }
    }
    let _ = ob;
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use fp_engine::Session;

    #[test]
    fn renders_page_with_zoom() {
        let mut s = Session::default();
        assert!(s.execute("file.new", json!({"width": 40, "height": 20, "background": "#ffffff"})).is_ok());
        assert!(s.execute("draw.rectangle", json!({"rect": [0, 0, 20, 20]})).is_ok());
        let r = Renderer::default();
        let Some(d) = s.active() else { panic!("kein Dokument") };
        let out = render(&r, d, &RenderRequest { zoom: 2.0, ..RenderRequest::default() });
        let Ok(out) = out else { panic!("{out:?}") };
        assert_eq!((out.width, out.height), (80, 40));
        assert_eq!(&out.png[1..4], b"PNG");
        let half = render(&r, d, &RenderRequest { zoom: 0.5, ..RenderRequest::default() });
        assert!(matches!(half, Ok(ref h) if h.width == 20 && h.height == 10));
        let lim = render(&r, d, &RenderRequest { zoom: 10.0, max_pixels: 800, ..RenderRequest::default() });
        assert!(matches!(lim, Ok(ref l) if (l.width as u64 * l.height as u64) <= 900 && !l.notes.is_empty()));
    }
}
