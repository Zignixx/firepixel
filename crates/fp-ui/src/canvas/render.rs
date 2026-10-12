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

/// Elemente (oben zuerst) übereinander rendern; `None`, wenn nichts sichtbar ist.
pub fn render_elements(r: &fp_render::Renderer, doc: &Document, frame: usize, elements: &[Arc<Element>]) -> Option<RgbaImage> {
    let map = fp_render::PointMap::identity();
    let parts: Vec<RgbaImage> = elements.iter().rev().map(|e| r.render_element(doc, e, &map, frame).image).filter(|i| i.width() > 0).collect();
    let bounds = parts.iter().fold(IRect::default(), |acc, i| acc.union(&i.bounds()));
    if bounds.is_empty() {
        return None;
    }
    let mut out = RgbaImage::new(bounds).ok()?;
    for p in &parts {
        fp_raster::composite(&mut out, p, None, &fp_raster::CompositeOptions::default());
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
}

/// Stand der Leinwand eines Dokuments (Render-Cache und Interaktionszustand der Werkzeuge).
#[derive(Default)]
pub struct CanvasCache {
    pub texture: Option<egui::TextureHandle>,
    /// Schlüssel der angezeigten Textur.
    pub shown: Option<RenderKey>,
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
    /// Farbfeld-Rückmeldung (Optimieren-Bedienfeld).
    pub swatch: Option<super::overlay::SwatchCache>,
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
