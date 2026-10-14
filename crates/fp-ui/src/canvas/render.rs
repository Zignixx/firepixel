//! Rendern der Leinwand über fp-render: Ausschnitt in Gerätepixeln (`Renderer::render_region`) auf
//! einem Hintergrund-Thread, Ergebnis als egui-Textur. Die Oberfläche blockiert nie (ARCHITEKTUR
//! §3): bis das neue Bild da ist, bleibt das alte stehen. Für Tests und Bildschirmfotos gibt es den
//! synchronen Weg.
//!
//! Beim Ziehen („Vorschau beim Ziehen“) und beim Bearbeiten von Text werden die betroffenen
//! Elemente im Hintergrundbild ausgeblendet (`hidden`, eigener Schlüssel `variant`) und als
//! Vorschau-Textur ([`super::ghost`]) darübergelegt – das Dokument wird während des Ziehens nicht
//! neu gerendert.

use fp_engine::render::PageRenderer;
use fp_engine::DocId;
use fp_geom::IRect;
use fp_model::{Document, Element, ElementId};
use fp_raster::RgbaImage;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

/// fp-render als Renderer der Sitzung (Pipette, Speichern mit flachem Bild).
pub struct EngineRenderer(pub Arc<fp_render::Renderer>);

impl PageRenderer for EngineRenderer {
    fn render_page(&self, doc: &Document, page: usize, frame: usize) -> Option<RgbaImage> {
        let out = self.0.render_page(doc, page, frame);
        (out.image.width() > 0).then_some(out.image)
    }

    fn render_elements(&self, doc: &Document, _page: usize, frame: usize, elements: &[Arc<Element>]) -> Option<RgbaImage> {
        render_elements(&self.0, doc, frame, elements)
    }

    fn flattener(&self) -> Option<&dyn fp_fwpng::Flattener> {
        Some(&*self.0)
    }

    /// Glyphenkonturen für „In Pfade konvertieren“ (gleicher Satz wie beim Rendern).
    fn text_outlines(&self, doc: &Document, t: &fp_model::TextElem) -> Option<Vec<Vec<fp_geom::BezierContour>>> {
        let block = fp_render::convert::text_block(doc, t);
        let mut lay = fp_text::layout(&block, self.0.fonts(), &fp_text::LayoutOptions::default());
        if let Some(a) = t.transform.as_ref().filter(|a| !a.is_identity()) {
            let aff = fp_text::raster::Affine { a: a.m[0][0] as f64, b: a.m[0][1] as f64, c: a.m[1][0] as f64, d: a.m[1][1] as f64, e: a.m[2][0] as f64, f: a.m[2][1] as f64 };
            for g in &mut lay.glyphs {
                g.transform = g.transform.then(&aff);
            }
            for d in &mut lay.decorations {
                for q in &mut d.quad {
                    *q = aff.apply(*q);
                }
            }
        }
        Some(fp_text::render::glyph_contours(&lay))
    }

    /// Satzrahmen eines Textes (Auswahlrahmen, Treffertest, Info).
    fn text_frame(&self, doc: &Document, t: &fp_model::TextElem) -> Option<fp_geom::Rect> {
        fp_engine::render::fp_text_frame(self.0.fonts(), doc, t)
    }
}

/// Elemente (oben zuerst) übereinander rendern; `None`, wenn nichts sichtbar ist. Deckkraft und
/// Mischmodus jedes Elements werden eingebacken wie beim Rastern im Original (B-2b BA-119).
pub fn render_elements(r: &fp_render::Renderer, doc: &Document, frame: usize, elements: &[Arc<Element>]) -> Option<RgbaImage> {
    let map = fp_render::PointMap::identity();
    let parts: Vec<(RgbaImage, &Arc<Element>)> = elements.iter().rev().map(|e| (r.render_element(doc, e, &map, frame).image, e)).filter(|(i, _)| i.width() > 0).collect();
    let bounds = parts.iter().fold(IRect::default(), |acc, (i, _)| acc.union(&i.bounds()));
    if bounds.is_empty() {
        return None;
    }
    let mut out = RgbaImage::new(bounds).ok()?;
    for (p, e) in &parts {
        let opt = fp_raster::CompositeOptions { mode: e.common.blend, opacity: e.common.opacity, ..fp_raster::CompositeOptions::default() };
        fp_raster::composite(&mut out, p, None, &opt);
    }
    Some(out)
}

/// Schlüssel eines gerenderten Ausschnitts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderKey {
    pub doc: DocId,
    pub generation: u64,
    pub epoch: u64,
    pub page: usize,
    pub frame: usize,
    /// Ausschnitt in Dokumentpixeln.
    pub region: IRect,
    /// Zoom in Gerätepixeln (Ansichtszoom × Pixel pro Punkt).
    pub zoom: f64,
    /// Kennung der ausgeblendeten Elemente (0 = keine).
    pub variant: u64,
    /// Ansicht ▸ Macintosh-Gamma: Bild nur für die Anzeige aufhellen (TW-18).
    pub gamma: bool,
}

/// Stand der Leinwand eines Dokuments (Render-Cache und Interaktionszustand der Werkzeuge).
#[derive(Default)]
pub struct CanvasCache {
    pub texture: Option<egui::TextureHandle>,
    /// Schlüssel der angezeigten Textur.
    pub shown: Option<RenderKey>,
    /// Bild der angezeigten Textur (für das Hochladen nur geänderter Bereiche, [`show_result`]).
    pub shown_image: Option<RgbaImage>,
    /// Zuletzt angeforderter Schlüssel.
    pub requested: Option<RenderKey>,
    /// Zähler für erzwungenes Neuzeichnen.
    pub epoch: u64,
    /// Beim nächsten Frame einpassen/zentrieren.
    pub needs_initial_fit: bool,
    /// Primärtaste wurde auf der Leinwand gedrückt (Down an das Werkzeug geschickt).
    pub pointer_down: bool,
    /// Letzte an das Werkzeug gemeldete Position.
    pub last_pos: Option<fp_geom::Point>,
    /// Hand-Verschieben läuft (Leertaste / mittlere Taste).
    pub panning: bool,
    /// Hilfslinie wird aus dem Lineal gezogen (true = waagerecht).
    pub guide_drag: Option<bool>,
    /// Vorhandene Hilfslinie wird verschoben: (waagerecht, Index).
    pub guide_move: Option<(bool, usize)>,
    /// Im Hintergrundbild ausgeblendete Elemente (Ziehen, Text bearbeiten).
    pub hidden: Vec<u64>,
    /// Vorschau-Texturen (Ziehen, Text).
    pub ghosts: super::ghost::GhostCache,
    /// Umriss der Pixelauswahl („laufende Ameisen“).
    pub ants: Option<super::overlay::AntsCache>,
    /// Layout des bearbeiteten Texts (Caret, Auswahl, Treffertest).
    pub text: Option<super::textedit::TextLayoutCache>,
    /// Kontextmenü (Rechtsklick) ist offen: Inhalt und Lage.
    pub context: Option<super::context::ContextMenu>,
    /// Werkzeug-Info für die Statuszeile (Position, Größe).
    pub info: Option<String>,
    /// Rückmeldung des Werkzeugs aus diesem Frame.
    pub feedback: Vec<fp_engine::Overlay>,
    /// Im bearbeiteten Text wird mit gedrückter Taste markiert.
    pub text_drag: bool,
    /// Bildschirmfläche der Leinwand im letzten Frame (ohne Lineale).
    pub area: Option<egui::Rect>,
    /// Seitenrechteck auf dem Bildschirm, wie es in diesem Bild gezeichnet wurde.
    pub page_screen: Option<egui::Rect>,
    /// Mit dem Werkzeug „Exportbereich“ übernommener Bereich (Dokumentkoordinaten) für den
    /// Export-Dialog; der Dialog nimmt ihn mit `take()` (`file.exportArea {rect, path}`).
    pub export_area: Option<fp_geom::Rect>,
    /// Gezeichnete Auswahlrahmen dieses Frames (Dokumentkoordinaten; wandern beim Ziehen mit).
    pub sel_frames: Vec<[fp_geom::Point; 4]>,
    /// Angesammelte Rastungen von Strg+Mausrad bzw. Zwei-Finger-Zoom (eine Zoomstufe je Rastung).
    pub wheel_acc: f32,
    /// Dokumentpunkt des letzten Drückens auf der Leinwand (Greifpunkt beim Ziehen).
    pub press_pos: Option<fp_geom::Point>,
    /// Einfügemarke nach Doppelklick auf Text setzen, sobald das Textlayout vorliegt.
    pub pending_caret: Option<fp_geom::Point>,
    /// Letzter Klick auf der Leinwand (Zeit, Bildschirmpunkt) für die Doppelklick-Erkennung.
    pub last_click: Option<(f64, egui::Pos2)>,
    pub pinch_acc: f32,
    /// Umschalttasten am Ende des letzten Bildes (Startzustand für die Ereignisfolge des nächsten).
    pub frame_mods: Option<egui::Modifiers>,
    /// Drückpunkt (Bildschirm) des laufenden Klicks für die Klick-/Doppelklick-Erkennung.
    pub click_press: Option<egui::Pos2>,
    /// Farbfeld-Rückmeldung (Optimieren-Bedienfeld).
    pub swatch: Option<super::overlay::SwatchCache>,
    /// Zeit des letzten Bildes mit offenem Text-Editor (Textvorschau nach dem Übernehmen stehen
    /// lassen, bis das neue Hintergrundbild da ist).
    pub text_linger: Option<f64>,
    /// Ansicht (Original/Vorschau/2-fach/4-fach), für die zuletzt der Sichtbereich gemeldet wurde:
    /// Wechselt sie, bleibt der Dokumentpunkt in der Mitte stehen (N25).
    pub view_mode: Option<fp_engine::view::PreviewMode>,
    /// Genauer Dokument-Mittelpunkt beim letzten Ansichtswechsel (ohne die Pixel-Ausrichtung):
    /// mehrfaches Umschalten ohne Bewegung wandert so nicht um halbe Pixel weiter.
    pub switch_center: Option<(f64, f64)>,
}

impl CanvasCache {
    /// Inhalt hat sich geändert: neu rendern.
    pub fn invalidate(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
    }
}

/// Kennung einer Menge ausgeblendeter Elemente.
pub fn hidden_variant(ids: &[u64]) -> u64 {
    if ids.is_empty() {
        return 0;
    }
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for id in ids {
        for b in id.to_le_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    }
    h | 1
}

/// Ein Auftrag an den Render-Thread.
pub struct RenderJob {
    pub key: RenderKey,
    pub doc: Arc<Document>,
    /// Auszublendende Elemente.
    pub hidden: Vec<u64>,
}

/// Ergebnis.
pub struct RenderResult {
    pub key: RenderKey,
    pub image: RgbaImage,
}

/// Hintergrund-Thread für die Leinwand.
pub struct RenderWorker {
    tx: Option<Sender<RenderJob>>,
    rx: Option<Receiver<RenderResult>>,
    renderer: Arc<fp_render::Renderer>,
}

impl RenderWorker {
    /// Startet den Thread (ohne Kontext keine Neuzeichnen-Anforderung, z. B. in Tests).
    pub fn new(renderer: Arc<fp_render::Renderer>, ctx: Option<egui::Context>) -> RenderWorker {
        let (job_tx, job_rx) = channel::<RenderJob>();
        let (res_tx, res_rx) = channel::<RenderResult>();
        let r = Arc::clone(&renderer);
        let spawned = std::thread::Builder::new().name("fp-canvas-render".into()).spawn(move || {
            while let Ok(mut job) = job_rx.recv() {
                // nur der neueste Auftrag zählt
                while let Ok(newer) = job_rx.try_recv() {
                    job = newer;
                }
                let image = render_now(&r, &job.doc, &job.key, &job.hidden);
                if res_tx.send(RenderResult { key: job.key, image }).is_err() {
                    break;
                }
                if let Some(c) = &ctx {
                    c.request_repaint();
                }
            }
        });
        match spawned {
            Ok(_) => RenderWorker { tx: Some(job_tx), rx: Some(res_rx), renderer },
            Err(_) => RenderWorker { tx: None, rx: None, renderer },
        }
    }

    /// Auftrag senden; ohne Thread (oder `sync`) sofort rendern und das Ergebnis zurückgeben.
    pub fn request(&self, job: RenderJob, sync: bool) -> Option<RenderResult> {
        if !sync && let Some(tx) = &self.tx {
            let _ = tx.send(job);
            return None;
        }
        let image = render_now(&self.renderer, &job.doc, &job.key, &job.hidden);
        Some(RenderResult { key: job.key, image })
    }

    /// Fertige Ergebnisse abholen.
    pub fn poll(&self) -> Vec<RenderResult> {
        match &self.rx {
            Some(rx) => rx.try_iter().collect(),
            None => Vec::new(),
        }
    }
}

fn render_now(r: &fp_render::Renderer, doc: &Document, key: &RenderKey, hidden: &[u64]) -> RgbaImage {
    let mut img = render_plain(r, doc, key, hidden);
    if key.gamma {
        apply_gamma_preview(&mut img);
    }
    img
}

/// Gamma-Vorschau (Ansicht ▸ Macintosh-Gamma, TW-18): R, G, B des fertigen Leinwandbilds über
/// [`fp_color::models::gamma_preview_table`] (Windows: lokal 2,2, andere Plattform 1,8). Nur Anzeige –
/// Dokument und Export bleiben unverändert.
pub fn apply_gamma_preview(img: &mut RgbaImage) {
    let t = gamma_table();
    for px in img.as_bytes_mut().chunks_exact_mut(4) {
        px[0] = t[px[0] as usize];
        px[1] = t[px[1] as usize];
        px[2] = t[px[2] as usize];
    }
}

/// Eine Anzeigefarbe durch die Gamma-Vorschau (Schachbrett transparenter Seiten, BA-045).
pub fn gamma_color(c: egui::Color32) -> egui::Color32 {
    let t = gamma_table();
    egui::Color32::from_rgba_premultiplied(t[c.r() as usize], t[c.g() as usize], t[c.b() as usize], c.a())
}

/// Tabelle der Gamma-Vorschau (einmal berechnet).
pub fn gamma_table() -> &'static [u8; 256] {
    static T: std::sync::OnceLock<[u8; 256]> = std::sync::OnceLock::new();
    T.get_or_init(|| fp_color::models::gamma_preview_table(2.2, 1.8))
}

fn render_plain(r: &fp_render::Renderer, doc: &Document, key: &RenderKey, hidden: &[u64]) -> RgbaImage {
    if hidden.is_empty() {
        return r.render_region(doc, key.page, key.frame, key.region, key.zoom).image;
    }
    // Kopie mit ausgeblendeten Elementen (Copy-on-Write: nur die betroffenen Zweige werden neu)
    let mut d = doc.clone();
    for id in hidden {
        fp_engine::docops::edit_by_id(&mut d, key.page, ElementId(*id), |e| e.common.visible = false);
    }
    r.render_region(&d, key.page, key.frame, key.region, key.zoom).image
}

/// RGBA-Bild → egui-Bild.
pub fn to_color_image(img: &RgbaImage) -> egui::ColorImage {
    egui::ColorImage::from_rgba_unmultiplied([img.width(), img.height()], img.as_bytes())
}

/// Fertiges Renderergebnis als Leinwandtextur zeigen. Zeigt die Textur schon denselben Ausschnitt
/// im selben Zoom, wird nur das geänderte Rechteck hochgeladen: Auf Rechnern ohne Grafikkarte
/// schafft das Hochladen nur etwa 30 MB/s, eine ganze Leinwandtextur bei 400 % (bis 4400×2500)
/// brauchte dort über 1 s je Bearbeitung (FV-8).
pub fn show_result(ctx: &egui::Context, cc: &mut CanvasCache, res: RenderResult) {
    let same_layout = cc.shown.is_some_and(|k| k.region == res.key.region && (k.zoom - res.key.zoom).abs() < 1e-12);
    match (&mut cc.texture, &cc.shown_image) {
        (Some(tex), Some(prev)) if same_layout && prev.width() == res.image.width() && prev.height() == res.image.height() && tex.size() == [res.image.width(), res.image.height()] => {
            if let Some((x, y, w, h)) = changed_rect(prev, &res.image) {
                tex.set_partial([x, y], sub_image(&res.image, x, y, w, h), egui::TextureOptions::NEAREST);
            }
        }
        (Some(tex), _) => tex.set(to_color_image(&res.image), egui::TextureOptions::NEAREST),
        (None, _) => cc.texture = Some(ctx.load_texture(format!("canvas-{}", res.key.doc.0), to_color_image(&res.image), egui::TextureOptions::NEAREST)),
    }
    cc.shown = Some(res.key);
    cc.shown_image = Some(res.image);
}

/// Kleinstes Rechteck (x, y, Breite, Höhe) in Bildpixeln, in dem sich zwei gleich große Bilder
/// unterscheiden; `None` bei gleichem Inhalt.
pub fn changed_rect(a: &RgbaImage, b: &RgbaImage) -> Option<(usize, usize, usize, usize)> {
    let (w, h) = (b.width(), b.height());
    if a.width() != w || a.height() != h || w == 0 || h == 0 {
        return (w > 0 && h > 0).then_some((0, 0, w, h));
    }
    let stride = w * 4;
    let (ra, rb) = (a.as_bytes().chunks_exact(stride), b.as_bytes().chunks_exact(stride));
    let rows: Vec<usize> = ra.zip(rb).enumerate().filter(|(_, (x, y))| x != y).map(|(i, _)| i).collect();
    let (&top, &bottom) = (rows.first()?, rows.last()?);
    let (mut left, mut right) = (w, 0);
    for &r in &rows {
        let (x, y) = (&a.as_bytes()[r * stride..(r + 1) * stride], &b.as_bytes()[r * stride..(r + 1) * stride]);
        let px = |i: usize| x[i * 4..i * 4 + 4] != y[i * 4..i * 4 + 4];
        if let Some(l) = (0..left.min(w)).find(|&i| px(i)) {
            left = l;
        }
        if let Some(rr) = (right..w).rev().find(|&i| px(i)) {
            right = rr;
        }
    }
    let right = right.max(left);
    Some((left, top, right + 1 - left, bottom + 1 - top))
}

fn sub_image(img: &RgbaImage, x: usize, y: usize, w: usize, h: usize) -> egui::ColorImage {
    let stride = img.width() * 4;
    let mut bytes = Vec::with_capacity(w * h * 4);
    for row in y..y + h {
        bytes.extend_from_slice(&img.as_bytes()[row * stride + x * 4..row * stride + (x + w) * 4]);
    }
    egui::ColorImage::from_rgba_unmultiplied([w, h], &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img(w: i32, h: i32) -> RgbaImage {
        RgbaImage::new(IRect::new(0, 0, w, h)).expect("Bild")
    }

    fn set(i: &mut RgbaImage, x: usize, y: usize, v: u8) {
        let w = i.width();
        let p = (y * w + x) * 4;
        i.as_bytes_mut()[p + 1] = v;
        i.as_bytes_mut()[p + 3] = 255;
    }

    #[test]
    fn gamma_preview_brightens_skin_tone_tw18() {
        // Messwert TWEAKS TW-18: Hautton 198,147,115 → mit Macintosh-Gamma 207,162,133
        let mut a = img(1, 1);
        a.as_bytes_mut().copy_from_slice(&[198, 147, 115, 255]);
        apply_gamma_preview(&mut a);
        let b = a.as_bytes();
        assert_eq!(b[3], 255, "Alpha bleibt");
        assert_eq!(b[0], 207);
        assert!((b[1] as i32 - 162).abs() <= 1, "G {}", b[1]);
        assert!((b[2] as i32 - 133).abs() <= 1, "B {}", b[2]);
    }

    #[test]
    fn gamma_color_brightens_checker_like_the_image_ba045() {
        // Schachbrettfarben folgen derselben Tabelle wie das Bild (Hautton wie TW-18)
        let c = gamma_color(egui::Color32::from_rgb(198, 147, 115));
        assert_eq!((c.r(), c.a()), (207, 255));
        assert_eq!(gamma_color(egui::Color32::from_rgb(204, 204, 204)).r(), gamma_table()[204]);
        assert!(gamma_table()[204] > 204, "Grau wird heller");
        assert_eq!(gamma_color(egui::Color32::WHITE), egui::Color32::WHITE);
    }

    #[test]
    fn changed_rect_is_bounding_box_of_differences() {
        let a = img(40, 30);
        let mut b = img(40, 30);
        assert_eq!(changed_rect(&a, &b), None);
        set(&mut b, 7, 5, 9);
        assert_eq!(changed_rect(&a, &b), Some((7, 5, 1, 1)));
        set(&mut b, 2, 20, 9);
        set(&mut b, 39, 12, 9);
        assert_eq!(changed_rect(&a, &b), Some((2, 5, 38, 16)));
        // andere Größe: ganzes Bild
        assert_eq!(changed_rect(&img(10, 10), &b), Some((0, 0, 40, 30)));
    }

    #[test]
    fn sub_image_copies_the_rectangle() {
        let mut a = img(8, 6);
        set(&mut a, 3, 2, 200);
        let s = sub_image(&a, 3, 2, 2, 3);
        assert_eq!(s.size, [2, 3]);
        assert_eq!(s.pixels[0].g(), 200);
        assert_eq!(s.pixels[1].g(), 0);
    }
}
