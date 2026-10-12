//! Hintergrundrechnung der Bildvorschau (re\17-bildvorschau.md §5.7, §13 „Hintergrundthread und
//! Abbruch“): je Feld höchstens ein laufender Auftrag; ein neuer Auftrag setzt das Abbruchflag des
//! alten, veraltete Ergebnisse werden verworfen. Höchstens [`MAX_PARALLEL`] Aufträge rechnen
//! gleichzeitig, damit die Oberfläche flüssig bleibt. Das Rendern des Dokuments ist für alle Felder
//! gleich und wird über [`RenderCache`] geteilt; Optimieren und Kodieren laufen je Feld mit denselben
//! Kodierern und Parametern wie der echte Export ([`fp_export::doc_export::preview_whole`],
//! `EncodeExtras::default()`).

use crate::panels::optimize::ExportRenderer;
use fp_export::export::EncodeExtras;
use fp_export::options::OptimizeSettings;
use fp_export::render::{Background, PageRenderer, RenderRequest};
use fp_raster::RgbaImage;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Condvar, Mutex, OnceLock};

/// Höchstens so viele Rechnungen gleichzeitig (re\17 §13).
pub const MAX_PARALLEL: usize = 2;

/// Ergebnis eines Auftrags.
#[derive(Clone, Debug)]
pub struct PaneResult {
    /// Schlüssel der Eingaben (Dokumentstand + Einstellungen), siehe [`settings_key`].
    pub key: u64,
    /// Dateigröße in Byte (echte Kodierung).
    pub bytes: usize,
    /// Anzahl Farben der Palette (indiziert) bzw. 0.
    pub colors: usize,
    pub palette: Vec<u32>,
    /// Farbtabelle (Farbe, Zähler, gesperrt, bearbeitet, websicher, transparent) aus demselben Lauf
    /// wie Bild und Größe (`fp_export::palette_edit::entries_from_result`, kein zweiter Lauf).
    pub entries: Vec<fp_export::palette_edit::PaletteEntry>,
    /// Paletteneintrag je angezeigtem Pixel (indiziert, sonst leer) – Pipetten, Klick in die Vorschau.
    pub indices: Arc<Vec<u8>>,
    /// Angezeigtes Bild (Ausgabepixel).
    pub image: Option<Arc<RgbaImage>>,
    pub error: Option<String>,
}

impl PaneResult {
    /// Paletteneintrag (Index in [`PaneResult::entries`]) des Bildpunkts `(x, y)` im angezeigten Bild.
    pub fn index_at(&self, x: i64, y: i64) -> Option<usize> {
        let img = self.image.as_ref()?;
        let (w, h) = (img.width() as i64, img.height() as i64);
        if x < 0 || y < 0 || x >= w || y >= h || self.indices.len() != (w * h) as usize {
            return None;
        }
        let i = self.indices[(y * w + x) as usize] as usize;
        (i < self.entries.len()).then_some(i)
    }
}

/// Was gerechnet wird.
#[derive(Clone, Debug)]
pub enum JobKind {
    /// Export-Vorschau mit diesen Einstellungen.
    Export(Box<OptimizeSettings>),
    /// Unverändertes Original (gerendertes Dokument ohne Exportoptimierung, „Vorschau“ aus, §5.3).
    Original,
}

/// Eingaben eines Auftrags.
#[derive(Clone)]
pub struct JobInput {
    pub doc: Arc<fp_model::Document>,
    pub renderer: Arc<fp_render::Renderer>,
    pub page: usize,
    pub frame: usize,
    /// Dokumentstand für den Render-Zwischenspeicher (Kennung, Generation).
    pub doc_key: (u64, u64),
    pub kind: JobKind,
    pub key: u64,
}

/// Laufender Auftrag.
pub struct Job {
    pub key: u64,
    cancel: Arc<AtomicBool>,
    rx: Receiver<Option<PaneResult>>,
}

impl Job {
    /// Abbrechen: das Ergebnis wird verworfen, ein wartender Auftrag rechnet gar nicht erst.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }
    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }
    /// Fertiges Ergebnis abholen (`Some(None)` = abgebrochen bzw. Thread beendet).
    pub fn poll(&self) -> Option<Option<PaneResult>> {
        match self.rx.try_recv() {
            Ok(r) => Some(r),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(None),
        }
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        // Dialog geschlossen oder Auftrag ersetzt: nicht weiterrechnen
        self.cancel();
    }
}

/// Zähler für Tests und Diagnose (gestartet, abgebrochen, fertig gerechnet).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counters {
    pub started: usize,
    pub cancelled: usize,
    pub finished: usize,
}

static STARTED: AtomicUsize = AtomicUsize::new(0);
static CANCELLED: AtomicUsize = AtomicUsize::new(0);
static FINISHED: AtomicUsize = AtomicUsize::new(0);
static HOLD: AtomicBool = AtomicBool::new(false);

/// Zählerstand (prozessweit).
pub fn counters() -> Counters {
    Counters { started: STARTED.load(Ordering::SeqCst), cancelled: CANCELLED.load(Ordering::SeqCst), finished: FINISHED.load(Ordering::SeqCst) }
}

/// Für Tests: neue Aufträge warten vor dem Rechnen, bis `false` gesetzt wird (oder sie abgebrochen
/// werden). So lässt sich der Abbruch eines laufenden Auftrags sicher prüfen.
pub fn hold_jobs(on: bool) {
    HOLD.store(on, Ordering::SeqCst);
}

/// Schlüssel eines Auftrags: Dokumentstand, Seite, Status und Einstellungen.
pub fn settings_key(doc_key: (u64, u64), page: usize, frame: usize, kind: &JobKind) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    doc_key.hash(&mut h);
    page.hash(&mut h);
    frame.hash(&mut h);
    match kind {
        JobKind::Export(s) => format!("{s:?}").hash(&mut h),
        JobKind::Original => "original".hash(&mut h),
    }
    h.finish()
}

/// Dieselben Kodierer-Eingaben wie beim echten Export (`EncodeExtras::default()`, Tabellen
/// einmal je Prozess geladen).
fn encode_extras() -> &'static EncodeExtras {
    static EX: OnceLock<EncodeExtras> = OnceLock::new();
    EX.get_or_init(EncodeExtras::default)
}

/// Gemeinsamer Render-Zwischenspeicher aller Felder eines Dialogs: je (Dokumentstand, Anfrage)
/// ein Bild. Wer zuerst kommt, rendert; die anderen warten an der Sperre und nehmen das Ergebnis.
#[derive(Default)]
pub struct RenderCache {
    map: Mutex<(u64, u64, HashMap<String, Arc<RgbaImage>>)>,
}

struct CachingRenderer<'a> {
    inner: ExportRenderer,
    cache: &'a RenderCache,
    doc_key: (u64, u64),
}

impl PageRenderer for CachingRenderer<'_> {
    fn render(&self, doc: &fp_model::Document, req: &RenderRequest) -> Result<RgbaImage, String> {
        let key = format!("{}-{}-{:?}-{:?}-{:?}", req.page, req.frame, req.background, req.area, req.layers);
        let mut g = self.cache.map.lock().map_err(|e| e.to_string())?;
        if (g.0, g.1) != self.doc_key {
            *g = (self.doc_key.0, self.doc_key.1, HashMap::new());
        }
        if let Some(img) = g.2.get(&key) {
            return Ok((**img).clone());
        }
        let img = self.inner.render(doc, req)?;
        g.2.insert(key, Arc::new(img.clone()));
        Ok(img)
    }
}

/// Begrenzt die Zahl gleichzeitig rechnender Aufträge.
struct Slots {
    busy: Mutex<usize>,
    cv: Condvar,
}

fn slots() -> &'static Slots {
    static S: OnceLock<Slots> = OnceLock::new();
    S.get_or_init(|| Slots { busy: Mutex::new(0), cv: Condvar::new() })
}

struct SlotGuard;

impl Drop for SlotGuard {
    fn drop(&mut self) {
        let s = slots();
        if let Ok(mut b) = s.busy.lock() {
            *b = b.saturating_sub(1);
        }
        s.cv.notify_one();
    }
}

/// Wartet auf einen freien Platz; `None`, wenn der Auftrag inzwischen abgebrochen wurde.
fn acquire(cancel: &AtomicBool) -> Option<SlotGuard> {
    let s = slots();
    let mut b = s.busy.lock().ok()?;
    loop {
        if cancel.load(Ordering::SeqCst) {
            return None;
        }
        if *b < MAX_PARALLEL {
            *b += 1;
            return Some(SlotGuard);
        }
        b = s.cv.wait_timeout(b, std::time::Duration::from_millis(20)).ok()?.0;
    }
}

/// Startet einen Auftrag in einem eigenen Thread; am Ende wird neu gezeichnet.
pub fn spawn(input: JobInput, cache: Arc<RenderCache>, ctx: &egui::Context) -> Job {
    let cancel = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel();
    let key = input.key;
    let c2 = Arc::clone(&cancel);
    let ctx2 = ctx.clone();
    STARTED.fetch_add(1, Ordering::SeqCst);
    let spawned = std::thread::Builder::new().name("fp-image-preview".into()).spawn(move || {
        let res = run(&input, &cache, &c2);
        if res.is_none() {
            CANCELLED.fetch_add(1, Ordering::SeqCst);
        } else {
            FINISHED.fetch_add(1, Ordering::SeqCst);
        }
        let _ = tx.send(res);
        ctx2.request_repaint();
    });
    if let Err(e) = spawned {
        log::warn!("image preview worker: {e}");
    }
    Job { key, cancel, rx }
}

fn run(input: &JobInput, cache: &RenderCache, cancel: &AtomicBool) -> Option<PaneResult> {
    while HOLD.load(Ordering::SeqCst) {
        if cancel.load(Ordering::SeqCst) {
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    let _slot = acquire(cancel)?;
    if cancel.load(Ordering::SeqCst) {
        return None;
    }
    let r = compute(input, cache, cancel);
    (!cancel.load(Ordering::SeqCst)).then_some(r)
}

/// Rechnet einen Auftrag (ohne Thread; auch für Tests).
pub fn compute(input: &JobInput, cache: &RenderCache, cancel: &AtomicBool) -> PaneResult {
    let r = CachingRenderer { inner: ExportRenderer { normal: Arc::clone(&input.renderer) }, cache, doc_key: input.doc_key };
    let empty = |error: Option<String>| PaneResult { key: input.key, bytes: 0, colors: 0, palette: Vec::new(), entries: Vec::new(), indices: Arc::new(Vec::new()), image: None, error };
    match &input.kind {
        JobKind::Original => {
            let req = RenderRequest { page: input.page, frame: input.frame, background: Background::Canvas, ..Default::default() };
            match r.render(&input.doc, &req) {
                Ok(img) => PaneResult { image: Some(Arc::new(img)), ..empty(None) },
                Err(e) => empty(Some(e)),
            }
        }
        JobKind::Export(s) => {
            let cancelled = || cancel.load(Ordering::SeqCst);
            match fp_export::doc_export::preview_whole(&r, &input.doc, input.page, input.frame, s, encode_extras(), &cancelled) {
                Ok(Some(p)) => {
                    let entries = fp_export::palette_edit::entries_from_result(&p.palette, &p.counts, p.transparent_index, s);
                    PaneResult { key: input.key, bytes: p.bytes, colors: p.colors, palette: p.palette, entries, indices: Arc::new(p.indices), image: Some(Arc::new(p.image)), error: None }
                }
                Ok(None) => empty(None),
                Err(e) => empty(Some(e.to_string())),
            }
        }
    }
}
