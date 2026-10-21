//! Rendern der Leinwand über fp-render: Ausschnitt in Gerätepixeln (`Renderer::render_region`) auf
//! einem Hintergrund-Thread, Ergebnis als egui-Textur. Die Oberfläche blockiert nie (ARCHITEKTUR
//! §3): bis das neue Bild da ist, bleibt das alte stehen. Für Tests und Bildschirmfotos gibt es den
//! synchronen Weg.
//!
//! Beim Ziehen („Vorschau beim Ziehen“) und beim Bearbeiten von Text werden die betroffenen
//! Elemente im Hintergrundbild ausgeblendet (`hidden`, eigener Schlüssel `variant`) und als
//! Vorschau-Textur ([`super::ghost`]) darübergelegt – das Dokument wird während des Ziehens nicht
//! neu gerendert. Liegen Objekte über den gezogenen, rendert derselbe Auftrag sie zusätzlich als
//! durchsichtige Schichten über der Vorschau (Stapelfolge, [`super::stack`], N34).

use fp_engine::DocId;
use fp_engine::render::PageRenderer;
use fp_geom::IRect;
use fp_model::{Document, Element};
use fp_raster::RgbaImage;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

/// fp-render als Renderer der Sitzung (Pipette, Speichern mit flachem Bild).
pub struct EngineRenderer(pub Arc<fp_render::Renderer>);

impl PageRenderer for EngineRenderer {
    fn render_page(&self, doc: &Document, page: usize, frame: usize) -> Option<RgbaImage> {
        let out = self.0.render_page(doc, page, frame);
        (out.image.width() > 0).then_some(out.image)
    }

    fn render_elements(
        &self,
        doc: &Document,
        _page: usize,
        frame: usize,
        elements: &[Arc<Element>],
    ) -> Option<RgbaImage> {
        render_elements(&self.0, doc, frame, elements)
    }

    fn flattener(&self) -> Option<&dyn fp_fwpng::Flattener> {
        Some(&*self.0)
    }

    /// Glyphenkonturen für „In Pfade konvertieren“ (gleicher Satz wie beim Rendern).
    fn text_outlines(
        &self,
        doc: &Document,
        t: &fp_model::TextElem,
    ) -> Option<Vec<Vec<fp_geom::BezierContour>>> {
        let block = fp_render::convert::text_block(doc, t);
        let mut lay = fp_text::layout(&block, self.0.fonts(), &fp_text::LayoutOptions::default());
        if let Some(a) = t.transform.as_ref().filter(|a| !a.is_identity()) {
            let aff = fp_text::raster::Affine {
                a: a.m[0][0] as f64,
                b: a.m[0][1] as f64,
                c: a.m[1][0] as f64,
                d: a.m[1][1] as f64,
                e: a.m[2][0] as f64,
                f: a.m[2][1] as f64,
            };
            for g in &mut lay.glyphs {
                g.transform = g.transform.then(&aff);
            }
            for d in &mut lay.decorations {
                for q in &mut d.quad {
                    *q = aff.apply(*q);
                }
            }
        }
        Some(fp_text::render::hinted_glyph_contours(&lay))
    }

    /// Satzrahmen eines Textes (Auswahlrahmen, Treffertest, Info).
    fn text_frame(&self, doc: &Document, t: &fp_model::TextElem) -> Option<fp_geom::Rect> {
        fp_engine::render::fp_text_frame(self.0.fonts(), doc, t)
    }
}

/// Elemente (oben zuerst) übereinander rendern; `None`, wenn nichts sichtbar ist. Deckkraft und
/// Mischmodus jedes Elements werden eingebacken wie beim Rastern im Original (B-2b BA-119).
pub fn render_elements(
    r: &fp_render::Renderer,
    doc: &Document,
    frame: usize,
    elements: &[Arc<Element>],
) -> Option<RgbaImage> {
    render_elements_in(r, doc, frame, elements, None).0
}

/// Like [`render_elements`], only the part inside `clip` (document pixels; same pixels there).
/// Also returns the bounds of the whole result (empty when nothing is visible).
pub fn render_elements_in(
    r: &fp_render::Renderer,
    doc: &Document,
    frame: usize,
    elements: &[Arc<Element>],
    clip: Option<IRect>,
) -> (Option<RgbaImage>, IRect) {
    let map = fp_render::PointMap::identity();
    let parts: Vec<(RgbaImage, &Arc<Element>)> = elements
        .iter()
        .rev()
        .map(|e| (r.render_element(doc, e, &map, frame).image, e))
        .filter(|(i, _)| i.width() > 0)
        .collect();
    let full = parts
        .iter()
        .fold(IRect::default(), |acc, (i, _)| acc.union(&i.bounds()));
    let bounds = match clip {
        Some(c) => full.intersect(&c),
        None => full,
    };
    if bounds.is_empty() {
        return (None, full);
    }
    let Ok(mut out) = RgbaImage::new(bounds) else {
        return (None, full);
    };
    for (p, e) in &parts {
        let opt = fp_raster::CompositeOptions {
            mode: e.common.blend,
            opacity: e.common.opacity,
            ..fp_raster::CompositeOptions::default()
        };
        fp_raster::composite(&mut out, p, None, &opt);
    }
    (Some(out), full)
}

/// Schlüssel eines gerenderten Ausschnitts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderKey {
    pub doc: DocId,
    pub generation: u64,
    pub epoch: u64,
    pub page: usize,
    pub frame: usize,
    /// Dokumentpixel, deren Gerätepixel alle im Bild liegen (für Prüfungen in Dokumentkoordinaten).
    pub region: IRect,
    /// Seitenrechteck: sein linker oberer Punkt ist der Ursprung des Gerätepixelgitters, außerhalb
    /// bleibt das Bild durchsichtig.
    pub page_rect: IRect,
    /// Gerätepixel des Bilds im Gitter der Seite (BA-098: Kacheln eines Gitters passen zusammen).
    pub dev: IRect,
    /// Zoom in Gerätepixeln (Ansichtszoom × Pixel pro Punkt).
    pub zoom: f64,
    /// Kennung der ausgeblendeten Elemente (0 = keine).
    pub variant: u64,
    /// Ansicht ▸ Macintosh-Gamma: Bild nur für die Anzeige aufhellen (TW-18).
    pub gamma: bool,
    /// Key of the inspector slider preview rendered into this image (0 = none, FB-09/FB-10).
    pub preview: u64,
}

/// Stand der Leinwand eines Dokuments (Render-Cache und Interaktionszustand der Werkzeuge).
#[derive(Default)]
pub struct CanvasCache {
    /// Kacheln der angezeigten Leinwand (BA-098, [`show_result`]).
    pub tiles: Vec<CanvasTile>,
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
    /// Bildschirmpunkt beim Drücken auf die verschobene Hilfslinie (Klick ohne Ziehen erkennen, CT-B-04).
    pub guide_press: Option<egui::Pos2>,
    /// Letzter Klick auf eine Hilfslinie (Zeit, Punkt, Hilfslinie) für den Doppelklick (CT-B-04).
    pub guide_click: Option<(f64, egui::Pos2, (bool, usize))>,
    /// Im Hintergrundbild ausgeblendete Elemente (Ziehen, Text bearbeiten).
    pub hidden: Vec<u64>,
    /// Vorschau-Texturen (Ziehen, Text).
    pub ghosts: super::ghost::GhostCache,
    /// Schichten über den Vorschauen zur angezeigten Textur (N34; `RenderResult::layers`).
    pub layers: Vec<Option<egui::TextureHandle>>,
    /// Bildschirmrechteck, auf das die angezeigte Textur (und ihre Schichten) gezeichnet wurde.
    pub tex_rect: Option<egui::Rect>,
    /// Schichtung der laufenden Vorschau (Ziehen, Malen) in diesem Bild.
    pub stack: Option<super::stack::StackPlan>,
    /// Kennung der Schichtung, die das Hintergrundbild haben muss.
    pub stack_variant: u64,
    /// Quelle von `stack` ([`super::stack::source_key`]): nur bei Änderung neu berechnen.
    pub stack_src: u64,
    /// Umriss der Pixelauswahl („laufende Ameisen“).
    pub ants: Option<super::overlay::AntsCache>,
    /// Layout des bearbeiteten Texts (Caret, Auswahl, Treffertest).
    pub text: Option<super::textedit::TextLayoutCache>,
    /// Kontextmenü (Rechtsklick) ist offen: Inhalt und Lage.
    pub context: Option<super::context::ContextMenu>,
    /// Werkzeug-Info für die Statuszeile (Position, Größe).
    pub info: Option<String>,
    /// Text of the coordinate tag drawn at the pointer in this frame (View ▸ Tooltips, RE spec 57 §4).
    pub pointer_tag: Option<String>,
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
    /// Letzter Klick auf der Leinwand (Zeit, Bildschirmpunkt) für die Doppelklick-Erkennung.
    pub last_click: Option<(f64, egui::Pos2)>,
    /// Last click inside the open text editor (time, screen point, click count) for word/paragraph selection.
    pub text_clicks: Option<(f64, egui::Pos2, u8)>,
    pub pinch_acc: f32,
    /// Umschalttasten am Ende des letzten Bildes (Startzustand für die Ereignisfolge des nächsten).
    pub frame_mods: Option<egui::Modifiers>,
    /// Zeit (s) des letzten Bildes: Zeigerereignisse eines Bildes werden gleichmäßig auf die Zeit
    /// seitdem verteilt (K-8, FP1-06: egui liefert keine Zeitstempel je Ereignis).
    pub last_input_time: Option<f64>,
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

/// Seitenlänge einer Leinwandkachel in Gerätepixeln.
pub const TILE: i32 = 512;

/// Eine Kachel der Leinwandtextur: Gerätepixel `dev` im Gitter der Seite ([`RenderKey::dev`]).
pub struct CanvasTile {
    pub dev: IRect,
    pub texture: egui::TextureHandle,
}

/// Erster Gerätepixel i (ab dem Gitterursprung beim Dokumentpunkt `origin`), dessen Abtastpunkt
/// `origin + ⌊(i + ½) / zoom⌋` bei oder hinter der Kante `e` liegt – mit genau den
/// Gleitkomma-Schritten von fp-render (sonst kippt die letzte Zeile).
pub fn device_edge(origin: i64, e: i64, zoom: f64) -> i64 {
    let sample = |i: i64| origin + ((i as f64 + 0.5) / zoom).floor() as i64;
    let mut i = ((e - origin) as f64 * zoom - 0.5).ceil() as i64;
    while sample(i - 1) >= e {
        i -= 1;
    }
    while sample(i) < e {
        i += 1;
    }
    i
}

/// Gerätepixel `dev` auf ganze Kacheln erweitert, begrenzt auf `extent`.
pub fn snap_to_tiles(dev: IRect, extent: IRect) -> IRect {
    let down = |v: i32| v.div_euclid(TILE) * TILE;
    let up = |v: i32| (v + TILE - 1).div_euclid(TILE) * TILE;
    IRect::new(down(dev.left), down(dev.top), up(dev.right), up(dev.bottom)).intersect(&extent)
}

/// Dokumentpixel, deren Gerätepixel alle in `dev` liegen (Gitter am Ursprung von `page`).
pub fn covered_region(page: IRect, dev: IRect, zoom: f64) -> IRect {
    if dev.is_empty() {
        return IRect::default();
    }
    let sample = |o: i32, i: i32| o + (((i as f64) + 0.5) / zoom).floor() as i32;
    let full_w = device_edge(page.left as i64, page.right as i64, zoom) as i32;
    let full_h = device_edge(page.top as i64, page.bottom as i64, zoom) as i32;
    let l = if dev.left <= 0 {
        page.left
    } else {
        sample(page.left, dev.left - 1) + 1
    };
    let t = if dev.top <= 0 {
        page.top
    } else {
        sample(page.top, dev.top - 1) + 1
    };
    let r = if dev.right >= full_w {
        page.right
    } else {
        sample(page.left, dev.right)
    };
    let b = if dev.bottom >= full_h {
        page.bottom
    } else {
        sample(page.top, dev.bottom)
    };
    IRect::new(l, t, r.max(l), b.max(t)).intersect(&page)
}

/// Malt die Kacheln von `cc` so, dass das ganze Bild (`shown.dev`) das Rechteck `target` füllt.
pub fn paint_tiles(painter: &egui::Painter, cc: &CanvasCache, target: egui::Rect) {
    let Some(k) = cc.shown else { return };
    let (w, h) = (k.dev.width().max(1) as f32, k.dev.height().max(1) as f32);
    let sx = target.width() / w;
    let sy = target.height() / h;
    for t in &cc.tiles {
        let r = egui::Rect::from_min_size(
            target.min
                + egui::vec2(
                    (t.dev.left - k.dev.left) as f32 * sx,
                    (t.dev.top - k.dev.top) as f32 * sy,
                ),
            egui::vec2(t.dev.width() as f32 * sx, t.dev.height() as f32 * sy),
        );
        painter.image(
            t.texture.id(),
            r,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
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
    /// Im Hintergrundbild auszublendende Teile.
    pub hidden: Vec<super::stack::Atom>,
    /// Schichten über den Vorschauen (N34): je Schicht die auszublendenden Teile, `None` = leer.
    pub layers: Vec<Option<Vec<super::stack::Atom>>>,
}

/// Ergebnis.
pub struct RenderResult {
    pub key: RenderKey,
    pub image: RgbaImage,
    /// Durchsichtige Schichten über den Vorschauen (gleicher Ausschnitt und Zoom wie `image`).
    pub layers: Vec<Option<RgbaImage>>,
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
        let spawned = std::thread::Builder::new()
            .name("fp-canvas-render".into())
            .spawn(move || {
                while let Ok(mut job) = job_rx.recv() {
                    // nur der neueste Auftrag zählt
                    while let Ok(newer) = job_rx.try_recv() {
                        job = newer;
                    }
                    let (image, layers) = render_job(&r, &job);
                    if res_tx
                        .send(RenderResult {
                            key: job.key,
                            image,
                            layers,
                        })
                        .is_err()
                    {
                        break;
                    }
                    if let Some(c) = &ctx {
                        c.request_repaint();
                    }
                }
            });
        match spawned {
            Ok(_) => RenderWorker {
                tx: Some(job_tx),
                rx: Some(res_rx),
                renderer,
            },
            Err(_) => RenderWorker {
                tx: None,
                rx: None,
                renderer,
            },
        }
    }

    /// Auftrag senden; ohne Thread (oder `sync`) sofort rendern und das Ergebnis zurückgeben.
    pub fn request(&self, job: RenderJob, sync: bool) -> Option<RenderResult> {
        if !sync && let Some(tx) = &self.tx {
            let _ = tx.send(job);
            return None;
        }
        let (image, layers) = render_job(&self.renderer, &job);
        Some(RenderResult {
            key: job.key,
            image,
            layers,
        })
    }

    /// Fertige Ergebnisse abholen.
    pub fn poll(&self) -> Vec<RenderResult> {
        match &self.rx {
            Some(rx) => rx.try_iter().collect(),
            None => Vec::new(),
        }
    }
}

/// Hintergrundbild und Schichten eines Auftrags.
fn render_job(r: &fp_render::Renderer, job: &RenderJob) -> (RgbaImage, Vec<Option<RgbaImage>>) {
    let image = render_now(r, &job.doc, &job.key, &job.hidden, false);
    let layers = job
        .layers
        .iter()
        .map(|l| {
            l.as_ref()
                .map(|h| render_now(r, &job.doc, &job.key, h, true))
        })
        .collect();
    (image, layers)
}

fn render_now(
    r: &fp_render::Renderer,
    doc: &Document,
    key: &RenderKey,
    hidden: &[super::stack::Atom],
    clear: bool,
) -> RgbaImage {
    let mut img = render_plain(r, doc, key, hidden, clear);
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
    for px in img.as_bytes_mut().as_chunks_mut::<4>().0 {
        px[0] = t[px[0] as usize];
        px[1] = t[px[1] as usize];
        px[2] = t[px[2] as usize];
    }
}

/// Eine Anzeigefarbe durch die Gamma-Vorschau (Schachbrett transparenter Seiten, BA-045).
pub fn gamma_color(c: egui::Color32) -> egui::Color32 {
    let t = gamma_table();
    egui::Color32::from_rgba_premultiplied(
        t[c.r() as usize],
        t[c.g() as usize],
        t[c.b() as usize],
        c.a(),
    )
}

/// Tabelle der Gamma-Vorschau (einmal berechnet).
pub fn gamma_table() -> &'static [u8; 256] {
    static T: std::sync::OnceLock<[u8; 256]> = std::sync::OnceLock::new();
    T.get_or_init(|| fp_color::models::gamma_preview_table(2.2, 1.8))
}

/// `clear`: durchsichtige Schicht (ohne Leinwandfarbe) für die Stapelfolge der Vorschau (N34).
fn render_plain(
    r: &fp_render::Renderer,
    doc: &Document,
    key: &RenderKey,
    hidden: &[super::stack::Atom],
    clear: bool,
) -> RgbaImage {
    let device = |d: &Document| {
        let pr = key.page_rect;
        r.render_device(
            d,
            key.page,
            key.frame,
            pr,
            (pr.left, pr.top),
            key.zoom,
            key.dev,
        )
        .image
    };
    if hidden.is_empty() && !clear {
        return device(doc);
    }
    // Kopie mit ausgeblendeten Teilen (Copy-on-Write: nur die betroffenen Zweige werden neu)
    let mut d = doc.clone();
    super::stack::hide(&mut d, key.page, hidden);
    if clear && let Some(p) = d.pages.get_mut(key.page) {
        Arc::make_mut(p).root.background = fp_color::Argb(0);
    }
    device(&d)
}

/// RGBA-Bild → egui-Bild.
pub fn to_color_image(img: &RgbaImage) -> egui::ColorImage {
    egui::ColorImage::from_rgba_unmultiplied([img.width(), img.height()], img.as_bytes())
}

/// Fertiges Renderergebnis als Leinwandtextur zeigen: als Kacheln von [`TILE`] Gerätepixeln im
/// Gitter der Seite. Auf Rechnern ohne Grafikkarte schafft das Hochladen nur etwa 30 MB/s; eine
/// ganze Leinwandtextur bei 400 % (bis 4400×2500) brauchte dort über 1 s je Bearbeitung (FV-8) und
/// je Rollen oder Zoomen (BA-098). Darum wird nur hochgeladen, was neu ist: Kacheln, die schon zu
/// sehen waren, bleiben stehen (Rollen) bzw. bekommen nur ihr geändertes Rechteck (Bearbeitung).
pub fn show_result(ctx: &egui::Context, cc: &mut CanvasCache, res: RenderResult) {
    let k = res.key;
    let same_grid = cc.shown.is_some_and(|s| {
        s.doc == k.doc && s.page_rect == k.page_rect && (s.zoom - k.zoom).abs() < 1e-12
    });
    let mut old = std::mem::take(&mut cc.tiles);
    let prev = match (same_grid, cc.shown, cc.shown_image.as_ref()) {
        (true, Some(s), Some(img)) if img.width() as u32 == s.dev.width() => Some((s, img)),
        _ => {
            old.clear();
            None
        }
    };
    let img = &res.image;
    let ok = img.width() as u32 == k.dev.width() && img.height() as u32 == k.dev.height();
    let mut tiles = Vec::new();
    if ok && !k.dev.is_empty() {
        let mut ty = k.dev.top.div_euclid(TILE) * TILE;
        while ty < k.dev.bottom {
            let mut tx = k.dev.left.div_euclid(TILE) * TILE;
            while tx < k.dev.right {
                let t = IRect::new(tx, ty, tx + TILE, ty + TILE).intersect(&k.dev);
                let (ox, oy) = ((t.left - k.dev.left) as usize, (t.top - k.dev.top) as usize);
                let (w, h) = (t.width() as usize, t.height() as usize);
                let reuse = prev.and_then(|(s, pimg)| {
                    let pos = old.iter().position(|o| o.dev == t)?;
                    (s.dev.intersect(&t) == t).then_some((pos, s, pimg))
                });
                match reuse {
                    Some((pos, s, pimg)) => {
                        let mut tile = old.swap_remove(pos);
                        let p = ((t.left - s.dev.left) as usize, (t.top - s.dev.top) as usize);
                        if let Some((x, y, cw, ch)) =
                            changed_rect_at(pimg, p, img, (ox, oy), (w, h))
                        {
                            tile.texture.set_partial(
                                [x, y],
                                sub_image(img, ox + x, oy + y, cw, ch),
                                egui::TextureOptions::NEAREST,
                            );
                        }
                        tiles.push(tile);
                    }
                    None => tiles.push(CanvasTile {
                        dev: t,
                        texture: ctx.load_texture(
                            format!("canvas-{}-{}-{}", k.doc.0, t.left, t.top),
                            sub_image(img, ox, oy, w, h),
                            egui::TextureOptions::NEAREST,
                        ),
                    }),
                }
                tx += TILE;
            }
            ty += TILE;
        }
    }
    cc.tiles = tiles;
    cc.layers = res
        .layers
        .iter()
        .enumerate()
        .map(|(i, l)| {
            l.as_ref().filter(|img| img.width() > 0).map(|img| {
                ctx.load_texture(
                    format!("canvas-{}-layer-{i}", res.key.doc.0),
                    to_color_image(img),
                    egui::TextureOptions::NEAREST,
                )
            })
        })
        .collect();
    cc.shown = Some(res.key);
    cc.shown_image = Some(res.image);
}

/// Kleinstes Rechteck (x, y, Breite, Höhe) im Teil der Größe `size` ab `at_b` von `b`, in dem er
/// sich vom gleich großen Teil ab `at_a` von `a` unterscheidet (relativ zum Teil); `None` bei
/// gleichem Inhalt.
pub fn changed_rect_at(
    a: &RgbaImage,
    at_a: (usize, usize),
    b: &RgbaImage,
    at_b: (usize, usize),
    size: (usize, usize),
) -> Option<(usize, usize, usize, usize)> {
    let (w, h) = size;
    if w == 0 || h == 0 {
        return None;
    }
    let (sa, sb) = (a.width() * 4, b.width() * 4);
    let row_a = |y: usize| {
        let o = (at_a.1 + y) * sa + at_a.0 * 4;
        &a.as_bytes()[o..o + w * 4]
    };
    let row_b = |y: usize| {
        let o = (at_b.1 + y) * sb + at_b.0 * 4;
        &b.as_bytes()[o..o + w * 4]
    };
    let rows: Vec<usize> = (0..h).filter(|&y| row_a(y) != row_b(y)).collect();
    let (&top, &bottom) = (rows.first()?, rows.last()?);
    let (mut left, mut right) = (w, 0);
    for &r in &rows {
        let (x, y) = (row_a(r), row_b(r));
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

/// Kleinstes Rechteck (x, y, Breite, Höhe) in Bildpixeln, in dem sich zwei gleich große Bilder
/// unterscheiden; `None` bei gleichem Inhalt.
pub fn changed_rect(a: &RgbaImage, b: &RgbaImage) -> Option<(usize, usize, usize, usize)> {
    let (w, h) = (b.width(), b.height());
    if a.width() != w || a.height() != h || w == 0 || h == 0 {
        return (w > 0 && h > 0).then_some((0, 0, w, h));
    }
    let stride = w * 4;
    let (ra, rb) = (
        a.as_bytes().chunks_exact(stride),
        b.as_bytes().chunks_exact(stride),
    );
    let rows: Vec<usize> = ra
        .zip(rb)
        .enumerate()
        .filter(|(_, (x, y))| x != y)
        .map(|(i, _)| i)
        .collect();
    let (&top, &bottom) = (rows.first()?, rows.last()?);
    let (mut left, mut right) = (w, 0);
    for &r in &rows {
        let (x, y) = (
            &a.as_bytes()[r * stride..(r + 1) * stride],
            &b.as_bytes()[r * stride..(r + 1) * stride],
        );
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
        assert_eq!(
            gamma_color(egui::Color32::from_rgb(204, 204, 204)).r(),
            gamma_table()[204]
        );
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
