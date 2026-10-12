//! Werkzeuge als UI-unabhängige Zustandsautomaten (Spezifikation 10 §2).
//!
//! Die UI übersetzt Maus und Tastatur in [`ToolEvent`]s (Dokumentkoordinaten, Modifikatoren) und
//! gibt sie an [`crate::Session::tool_event`]. Das aktive Werkzeug ([`ToolMachine`]) antwortet mit
//! einer Vorschau ([`Overlay`]) und – beim Abschluss – mit Befehlsaufrufen ([`Invocation`]), die die
//! Sitzung über das Befehlsregister ausführt. So landen auch Werkzeugaktionen im Verlauf und sind
//! für MCP/JSF aufzeichnungsfähig.
//!
//! Gemeinsames Gerüst (10 §2.6): `Idle →Down→ Pressed →Bewegung ≥ 3/Zoom→ Dragging →Up→ Commit`;
//! Esc bricht ab. Regeln wie im Original: Umschalt = Seitenverhältnis bzw. 45°/15°, Alt = von der Mitte,
//! Leertaste beim Aufziehen = Form verschieben, Doppelklick schließt Polygone/Pfade.

pub mod ai_select;
mod bitmap;
mod draw;
pub mod pathedit;
mod pointer;
mod retouch;
pub mod snap;
pub mod text;
mod viewtools;

use crate::geometry::GeometryOps;
use crate::prefs::Preferences;
use crate::selection::Selection;
use crate::shortcuts::{Shortcut, ShortcutSet};
use crate::view::ViewState;
use fp_color::Argb;
use fp_geom::{Point, Rect};
use fp_model::{Contour, Document};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use ai_select::{AiClickTool, AiObjectTool};
pub use bitmap::{BrushTool, EyedropperTool, FillTool, LassoTool, MarqueeTool, WandTool};
pub use draw::{FreehandTool, LineTool, PenTool, ShapeTool, WebShapeTool};
pub use pathedit::{FreeformTool, KnifeTool, ScrubTool};
pub use pointer::{CropTool, PointerTool, TransformTool};
pub use retouch::RetouchTool;
pub use text::{TextEditView, TextTool};
pub use viewtools::{HandTool, ZoomTool};

// ------------------------------------------------------------------ Eingabe

/// Modifikatortasten (10 §2.2: Flags 0x004 Umschalt, 0x008 Alt, 0x010 Strg, 0x020 Feststell, 0x040 Leertaste).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Modifiers {
    pub shift: bool,
    pub alt: bool,
    pub ctrl: bool,
    /// Leertaste gedrückt (beim Aufziehen: Form verschieben).
    pub space: bool,
    /// Feststelltaste (präziser Cursor).
    pub caps: bool,
}

/// Art eines Zeigerereignisses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PointerKind {
    Down,
    /// Bewegung (mit oder ohne gedrückte Taste).
    Move,
    Up,
    DoubleClick,
}

/// Tasten, die Werkzeuge auswerten.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolKey {
    Escape,
    Enter,
    Up,
    Down,
    Left,
    Right,
    Backspace,
    Delete,
    Tab,
    /// Pos1 (Text-Editor: Zeilenanfang über [`ToolInput::TextCaret`] der UI).
    Home,
    /// Ende.
    End,
}

/// Bearbeiten-Befehle, die ein laufender Werkzeugvorgang selbst behandelt (Text-Editor).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum EditOp {
    Copy,
    Cut,
    /// Einfügen von Text aus der Systemzwischenablage.
    Paste(String),
    SelectAll,
    /// Rückgängig/Wiederholen innerhalb des Text-Editors (Strg+Z, Strg+Y bzw. Umschalt+Strg+Z).
    Undo,
    Redo,
    /// Zeichen-/Absatzattribute im Text-Editor setzen (Inspektor, Menü „Text“ während der
    /// Bearbeitung): mit Markierung nur die Markierung (Absatzattribute: ganze Absätze), ohne
    /// Markierung der ganze Text; der Editor bleibt offen.
    Attrs(fp_model::text::TextAttrs),
}

/// Eingabe an ein Werkzeug.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ToolInput {
    /// Zeiger in Dokumentkoordinaten.
    Pointer { kind: PointerKind, pos: Point },
    /// Taste gedrückt.
    Key(ToolKey),
    /// Texteingabe (Textwerkzeug).
    Text(String),
    /// Caret des Text-Editors setzen (Zeichenindex; `extend` = Auswahl erweitern). Die UI
    /// berechnet den Index aus dem Textlayout (Klick, Pos1/Ende, Pfeil auf/ab).
    TextCaret { index: usize, extend: bool },
    /// Bearbeiten-Befehl an den laufenden Vorgang.
    Edit(EditOp),
    /// IME-Vorschau (Komposition, noch nicht übernommen).
    Preedit(String),
}

impl ToolInput {
    /// Wird das Ereignis vom laufenden Vorgang behandelt, statt ein Menükürzel auszulösen?
    pub fn is_text_input(&self) -> bool {
        matches!(self, ToolInput::Text(_) | ToolInput::TextCaret { .. } | ToolInput::Edit(_) | ToolInput::Preedit(_))
    }
}

/// Ein Eingabe-Ereignis.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolEvent {
    pub input: ToolInput,
    pub mods: Modifiers,
    /// Stiftdruck 0..1 (Maus: 1).
    pub pressure: f32,
    /// Zeitstempel in ms.
    pub time_ms: u64,
}

impl ToolEvent {
    fn pointer(kind: PointerKind, x: f32, y: f32) -> ToolEvent {
        ToolEvent { input: ToolInput::Pointer { kind, pos: Point::new(x, y) }, mods: Modifiers::default(), pressure: 1.0, time_ms: 0 }
    }
    /// Taste gedrückt bei (x, y).
    pub fn down(x: f32, y: f32) -> ToolEvent {
        ToolEvent::pointer(PointerKind::Down, x, y)
    }
    /// Bewegung nach (x, y).
    pub fn drag(x: f32, y: f32) -> ToolEvent {
        ToolEvent::pointer(PointerKind::Move, x, y)
    }
    /// Taste losgelassen bei (x, y).
    pub fn up(x: f32, y: f32) -> ToolEvent {
        ToolEvent::pointer(PointerKind::Up, x, y)
    }
    /// Doppelklick bei (x, y).
    pub fn double(x: f32, y: f32) -> ToolEvent {
        ToolEvent::pointer(PointerKind::DoubleClick, x, y)
    }
    /// Taste.
    pub fn key(k: ToolKey) -> ToolEvent {
        ToolEvent { input: ToolInput::Key(k), mods: Modifiers::default(), pressure: 1.0, time_ms: 0 }
    }
    /// Texteingabe.
    pub fn text(s: &str) -> ToolEvent {
        ToolEvent { input: ToolInput::Text(s.into()), mods: Modifiers::default(), pressure: 1.0, time_ms: 0 }
    }
    /// Mit Umschalt.
    pub fn shift(mut self) -> ToolEvent {
        self.mods.shift = true;
        self
    }
    /// Mit Alt.
    pub fn alt(mut self) -> ToolEvent {
        self.mods.alt = true;
        self
    }
    /// Mit Strg.
    pub fn ctrl(mut self) -> ToolEvent {
        self.mods.ctrl = true;
        self
    }
    /// Mit Leertaste.
    pub fn space(mut self) -> ToolEvent {
        self.mods.space = true;
        self
    }
    /// Position eines Zeigerereignisses.
    pub fn pos(&self) -> Option<Point> {
        match &self.input {
            ToolInput::Pointer { pos, .. } => Some(*pos),
            _ => None,
        }
    }
    /// Art eines Zeigerereignisses.
    pub fn kind(&self) -> Option<PointerKind> {
        match &self.input {
            ToolInput::Pointer { kind, .. } => Some(*kind),
            _ => None,
        }
    }
}

// ------------------------------------------------------------------ Antwort

/// Ein Befehlsaufruf.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Invocation {
    pub command: String,
    pub params: Value,
}

impl Invocation {
    /// Neuer Aufruf.
    pub fn new(command: &str, params: Value) -> Invocation {
        Invocation { command: command.into(), params }
    }
}

/// Vorschau-Element (in Dokumentkoordinaten), das die UI über die Leinwand zeichnet.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Overlay {
    /// Rechteck (Aufziehen, Auswahlrahmen, Gummiband).
    Rect { rect: Rect, dashed: bool },
    /// Ellipse im Rechteck.
    Ellipse { rect: Rect, dashed: bool },
    /// Linie.
    Line { from: Point, to: Point },
    /// Polylinie.
    Polyline { points: Vec<Point>, closed: bool },
    /// Bézier-Konturen (Stift, Formen).
    Path { contours: Vec<Contour> },
    /// Griffe (Transformation, Pfadpunkte).
    Handles { points: Vec<Point> },
    /// Transformationsrahmen (vier Ecken, Mittelpunkt).
    TransformBox { quad: [Point; 4], center: Point },
    /// Textrahmen mit Text.
    Text { anchor: Point, width: Option<f32>, text: String },
    /// Pinselabdrücke (Kreise).
    Stroke { points: Vec<Point>, diameter: f32 },
    /// Intelligente Hilfslinie (Ausrichtung an anderen Objekten, Seitenrand, Mitte).
    SmartGuide { from: Point, to: Point },
    /// Beschriftung an einem Punkt (Abstand, Winkel, Größe).
    Label { at: Point, text: String },
    /// Hervorhebung beim Überfahren (Konturen, sonst Rahmen) – `SelectionMouseOverPreview`.
    Highlight { contours: Vec<Contour>, rect: Option<Rect> },
    /// Pfadpunkte: alle Anker (hohl), ausgewählte (gefüllt) mit Henkeln (`handles`).
    PathPoints { contours: Vec<Contour>, selected: Vec<(usize, usize)>, handles: bool },
    /// Vorschau beim Ziehen („Vorschau beim Ziehen“): die Elemente `ids` werden vom Rechteck `rect`
    /// auf das Viereck `quad` (LO, RO, RU, LU) abgebildet; die Originale blendet die UI aus.
    /// `copy` = Alt-Ziehen: das Original bleibt stehen.
    Ghost { ids: Vec<u64>, rect: Rect, quad: [Point; 4], copy: bool },
    /// Abdunkeln außerhalb des Rechtecks (Zuschneiden, Exportbereich).
    Shade { rect: Rect },
    /// Drehpunkt der Transformationswerkzeuge.
    Pivot { at: Point },
    /// Verlaufsgriffe (Anfang = Kreis, Ende = Quadrat).
    GradientHandles { start: Point, end: Point },
    /// Text-Editor auf der Leinwand (Caret, Auswahl, IME).
    TextEdit(Box<TextEditView>),
    /// Pinselspitze am Cursor (Bitmapwerkzeuge, `UseBrushSizeCursors`).
    BrushTip { at: Point, diameter: f32, round: bool },
    /// Markierung (Quelle des Stempels).
    Marker { at: Point },
    /// Steuerpunkt einer automatischen Form (gelbe Raute); `hot` = gerade gezogen/überfahren.
    ControlPoint { at: Point, tooltip: Option<String>, hot: bool },
}

/// Mauszeiger-Form (die UI zeichnet eigene Formen wie das Original).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Cursor {
    #[default]
    Arrow,
    Crosshair,
    Move,
    Copy,
    Hand,
    Grabbing,
    ZoomIn,
    ZoomOut,
    Text,
    Pen,
    PenClose,
    Rotate,
    Resize,
    Eyedropper,
    Bucket,
    Brush,
    NotAllowed,
    /// Zeiger über einem Pfadpunkt (Pfeil mit Quadrat).
    PointerPoint,
    /// Zeiger über einem Pfadsegment.
    PointerSegment,
    /// Teilauswahl (heller Pfeil).
    Subselect,
    /// Hintere auswählen.
    SelectBehind,
    /// Stift: Punkt hinzufügen / entfernen / Pfad fortsetzen.
    PenAdd,
    PenRemove,
    PenContinue,
    /// Griffe: waagerecht, senkrecht, diagonal (↗↙).
    ResizeH,
    ResizeV,
    ResizeNeSw,
    /// Neigen, Verzerren.
    Skew,
    Distort,
    /// Drehpunkt verschieben.
    Pivot,
    Knife,
    Freeform,
    Reshape,
    Pencil,
    Eraser,
    Stamp,
    Wand,
    Lasso,
    Gradient,
    /// Hilfslinie verschieben (↕ bzw. ↔).
    GuideH,
    GuideV,
    /// Zuschneiden (zwei verschränkte Winkel).
    Crop,
}

/// Antwort eines Werkzeugs.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ToolResponse {
    /// Aktuelle Vorschau.
    pub overlay: Vec<Overlay>,
    /// Auszuführende Befehle (in Reihenfolge).
    pub commands: Vec<Invocation>,
    /// Ergebnisse der Befehle (von der Sitzung gefüllt).
    pub results: Vec<Value>,
    pub cursor: Cursor,
    /// Werkzeugwechsel (z. B. Text → Zeiger nach `TextToolRevertsToPointer`).
    pub switch_tool: Option<ToolId>,
    /// Text für die Systemzwischenablage (Text-Editor: Kopieren/Ausschneiden).
    pub clipboard: Option<String>,
    /// Meldung für die Statuszeile.
    pub status: Option<String>,
    /// Ereignisse, die die Sitzung nach Befehlen und Werkzeugwechsel an das (neue) Werkzeug gibt
    /// (z. B. Doppelklick auf Text: Zeiger → Text, Klick an derselben Stelle).
    pub replay: Vec<ToolEvent>,
    /// Anfrage an die Oberfläche, die kein Befehl der Sitzung erledigen kann (z. B. Exportbereich →
    /// Export-Dialog mit `file.exportArea {rect}` öffnen). Ohne Oberfläche (MCP/CLI) ignoriert.
    pub ui_request: Option<Invocation>,
}

impl ToolResponse {
    fn with_overlay(o: Vec<Overlay>, cursor: Cursor) -> ToolResponse {
        ToolResponse { overlay: o, cursor, ..ToolResponse::default() }
    }
    fn commit(cmds: Vec<Invocation>) -> ToolResponse {
        ToolResponse { commands: cmds, ..ToolResponse::default() }
    }
}

/// Kontext für Werkzeuge (nur lesend).
pub struct ToolContext<'a> {
    pub doc: &'a Document,
    pub has_document: bool,
    pub page: usize,
    pub frame: usize,
    pub selection: &'a Selection,
    pub zoom: f64,
    pub prefs: &'a Preferences,
    pub options: &'a ToolOptions,
    pub geometry: &'a dyn GeometryOps,
    pub stroke_color: Argb,
    pub fill_color: Argb,
    pub bitmap_mode: bool,
    /// Ansicht (Fang an Raster/Hilfslinien/intelligenten Hilfslinien).
    pub view: &'a ViewState,
}

impl ToolContext<'_> {
    /// Ziehschwelle in Dokumentpixeln (3 Bildschirmpixel, B `@00d5bbc0`).
    pub fn drag_threshold(&self) -> f32 {
        (3.0 / self.zoom.max(1e-6)) as f32
    }
    /// Markierungsabstand in Dokumentpixeln.
    pub fn pick_tolerance(&self) -> f64 {
        self.prefs.pick_distance() / self.zoom.max(1e-6)
    }
    /// Abstand zum Greifen von Bézier-Henkeln in Dokumentpixeln (`BezHandlePickDist`).
    pub fn handle_tolerance(&self) -> f64 {
        self.prefs.get_int("BezHandlePickDist", 4).max(1) as f64 / self.zoom.max(1e-6)
    }
}

/// Ein Werkzeug-Zustandsautomat.
pub trait ToolMachine: Send {
    /// Werkzeug.
    fn id(&self) -> ToolId;
    /// Ereignis verarbeiten.
    fn handle(&mut self, ev: &ToolEvent, ctx: &ToolContext) -> ToolResponse;
    /// Laufenden Vorgang abschließen (Werkzeugwechsel, Menübefehl): z. B. Text übernehmen.
    fn finish(&mut self, _ctx: &ToolContext) -> ToolResponse {
        ToolResponse::default()
    }
    /// Aktuelle Vorschau.
    fn overlay(&self) -> Vec<Overlay> {
        Vec::new()
    }
    /// Rückmeldung auf der Leinwand für den aktuellen Zustand, auch ohne Ereignis (z. B. der
    /// Transformationsrahmen gleich nach dem Werkzeugwechsel, Pfadpunkte der Auswahl).
    fn feedback(&self, _ctx: &ToolContext) -> Vec<Overlay> {
        self.overlay()
    }
    /// Läuft ein Text-Editor (Tasten gehen dann an das Werkzeug statt an Menükürzel)?
    fn is_editing_text(&self) -> bool {
        false
    }
    /// Attribute im Text-Editor am Anfang der Markierung bzw. am Caret (für den Inspektor).
    fn text_edit_attrs(&self) -> Option<fp_model::text::TextAttrs> {
        None
    }
}


// ------------------------------------------------------------------ Optionen

/// Stil des Auswahlrahmens (`MarqueeToolMode`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum MarqueeStyle {
    Normal,
    FixedRatio(f32, f32),
    FixedSize(f32, f32),
}

/// Rand von Auswahl und Füllungen.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum EdgeMode {
    Hard,
    AntiAlias,
    Feather(u32),
}

/// Werkzeugoptionen (Eigenschaften-Inspektor), Vorgaben aus den Voreinstellungen.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolOptions {
    /// Eckenrundung neuer Rechtecke 0..1.
    pub rect_roundness: f32,
    pub polygon_sides: u32,
    pub polygon_star: bool,
    /// Spitzigkeit 0..1, `None` = automatisch.
    pub polygon_spikiness: Option<f32>,
    pub marquee_style: MarqueeStyle,
    pub marquee_edge: EdgeMode,
    pub lasso_edge: EdgeMode,
    pub wand_tolerance: u32,
    pub wand_edge: EdgeMode,
    /// Farbtoleranz des Farbeimers (`FillToolTolerance`).
    pub fill_tolerance: u32,
    /// Ausgewählten Bereich füllen (Farbeimer, `FillToolFillsSelection`, Werksvorgabe aus).
    pub fill_selection: bool,
    /// Farbtoleranz des Verlaufs (`GradientToolTolerance`); das Original merkt sie getrennt vom
    /// Farbeimer.
    #[serde(default = "tolerance_32")]
    pub gradient_tolerance: u32,
    /// Ausgewählten Bereich füllen (Verlauf, `GradientToolFillsSelection`, Werksvorgabe an,
    /// PARITAET_A W25); getrennt vom Farbeimer gemerkt.
    #[serde(default = "yes")]
    pub gradient_fill_selection: bool,
    /// Eigene Farben des Verlauf-Werkzeugs (gleichmäßig verteilt), unabhängig von Füll- und
    /// Strichfarbe; Werksvorgabe Schwarz → Weiß (im Original gemessen: Füllfarbe #808080,
    /// Strichfarbe #C04040 → Farbfeld Schwarz → Weiß, PARITAET_A W26).
    #[serde(default = "default_gradient_colors")]
    pub gradient_colors: Vec<Argb>,
    pub brush_size: f32,
    pub brush_soft: bool,
    pub pencil_antialias: bool,
    pub eraser_size: f32,
    pub eraser_round: bool,
    pub eraser_opacity: u8,
    /// Rand des Radiergummis 0..100 (`EraserToolSoftEdge` / 10).
    #[serde(default)]
    pub eraser_edge: u32,
    /// Pipette: 1, 3 oder 5 Pixel.
    pub eyedropper_samples: u32,
    pub text_font: String,
    pub text_size: f32,
    /// Eigene Textfarbe des Textwerkzeugs (`Text_FillColor`, Werksvorgabe schwarz) – nicht die
    /// Füllfarbe des Werkzeugbedienfelds.
    #[serde(default = "black")]
    pub text_color: Argb,
    /// Genauigkeit der Freihand-Kurvenanpassung (`PathToolPrecision`, 1..10).
    pub path_precision: u32,
    /// Formwerkzeuge: Hintergrund-Deckkraft (0..100).
    pub opacity: u8,
    /// Füllung neuer Vektorobjekte aus dem Eigenschaften-Inspektor (Parameter von `pi.fill`:
    /// Füllart, Verlaufsform, Kante, Textur …); `None` = einfarbig mit der Füllfarbe.
    #[serde(default)]
    pub new_fill: Option<Value>,
    /// Strich neuer Vektorobjekte (Parameter von `pi.stroke`: Vorgabe, Breite, Weichheit,
    /// Position, Textur); `None` = 1-Pixel-Strich mit der Strichfarbe.
    #[serde(default)]
    pub new_stroke: Option<Value>,
    /// Vollständiger Pinsel der Werkzeugvorgabe (`fp_model::Brush` als JSON, z. B. aus „Strich
    /// bearbeiten“ ohne Auswahl): Pinselart, Spitze, Empfindlichkeiten, Textur. Wird nach
    /// `new_stroke` auf neue Objekte gelegt (Strichfarbe bleibt die des Werkzeugbedienfelds).
    #[serde(default)]
    pub new_brush: Option<Value>,
    /// Gewählte automatische Form (Name aus dem Rechteck-Flyout bzw. Bedienfeld „Automatische
    /// Formen“), die das Werkzeug `AutoShape` aufzieht.
    #[serde(default)]
    pub auto_shape: String,
}

impl Default for ToolOptions {
    fn default() -> Self {
        ToolOptions::from_prefs(&Preferences::new())
    }
}

fn black() -> Argb {
    Argb(0xff00_0000)
}

fn yes() -> bool {
    true
}

fn tolerance_32() -> u32 {
    32
}

/// Werksvorgabe des Verlauf-Werkzeugs: Schwarz → Weiß.
pub fn default_gradient_colors() -> Vec<Argb> {
    vec![Argb(0xff00_0000), Argb(0xffff_ffff)]
}

/// Vorgabeschrift des Textwerkzeugs: wie im Original „Myriad Pro“, wenn installiert (UX-A W14,
/// `docs\screenshots\paritaet\a_133`), sonst `Text_Font` (Werksvorgabe Arial).
pub fn default_text_font(p: &Preferences) -> String {
    const ORIGINAL: &str = "Myriad Pro";
    // im Menü Text ▸ Schrift ohne Text gewählt (MN-2): gespeicherte Vorgabe gilt
    if let Some(f) = p.values.get("Text_Font").and_then(|v| v.as_str()).filter(|f| !f.trim().is_empty()) {
        return f;
    }
    if fp_text::FontDb::shared().find_family(ORIGINAL, false, false).is_some() {
        ORIGINAL.to_string()
    } else {
        p.get_str("Text_Font", "Arial")
    }
}

impl ToolOptions {
    /// „Ausgewählten Bereich füllen“ des Werkzeugs (Verlauf eigenes Feld, sonst Farbeimer).
    pub fn fills_selection(&self, tool: ToolId) -> bool {
        if tool == ToolId::Gradient { self.gradient_fill_selection } else { self.fill_selection }
    }

    /// Setzt „Ausgewählten Bereich füllen“ für Farbeimer bzw. Verlauf.
    pub fn set_fills_selection(&mut self, tool: ToolId, on: bool) {
        if tool == ToolId::Gradient {
            self.gradient_fill_selection = on;
        } else {
            self.fill_selection = on;
        }
    }

    /// Farben, die das Verlauf-Werkzeug malt (mindestens zwei; sonst die Werksvorgabe).
    pub fn gradient_tool_colors(&self) -> Vec<Argb> {
        if self.gradient_colors.len() >= 2 { self.gradient_colors.clone() } else { default_gradient_colors() }
    }

    /// Farbtoleranz des Werkzeugs (Verlauf eigenes Feld, sonst Farbeimer).
    pub fn tolerance_of(&self, tool: ToolId) -> u32 {
        if tool == ToolId::Gradient { self.gradient_tolerance } else { self.fill_tolerance }
    }

    /// Setzt die Farbtoleranz von Farbeimer bzw. Verlauf.
    pub fn set_tolerance_of(&mut self, tool: ToolId, v: u32) {
        if tool == ToolId::Gradient {
            self.gradient_tolerance = v;
        } else {
            self.fill_tolerance = v;
        }
    }

    /// Vorgaben aus den Voreinstellungen.
    pub fn from_prefs(p: &Preferences) -> ToolOptions {
        let edge = |aa: &str, fe: &str| {
            let f = p.get_int(fe, 0);
            if f > 0 {
                EdgeMode::Feather(f as u32)
            } else if p.get_bool(aa, false) {
                EdgeMode::AntiAlias
            } else {
                EdgeMode::Hard
            }
        };
        let style = match p.get_int("MarqueeToolMode", 0) {
            1 => MarqueeStyle::FixedRatio(p.get_int("MarqueeToolModeRatioX", 1) as f32, p.get_int("MarqueeToolModeRatioY", 2) as f32),
            2 => MarqueeStyle::FixedSize(p.get_int("MarqueeToolModeWidthX", 64) as f32, p.get_int("MarqueeToolModeWidthY", 64) as f32),
            _ => MarqueeStyle::Normal,
        };
        ToolOptions {
            rect_roundness: 0.0,
            polygon_sides: p.get_int("PolygonToolSides", 5).clamp(3, 360) as u32,
            polygon_star: p.get_bool("PolygonToolIsStar", false),
            polygon_spikiness: if p.get_bool("PolygonToolIsAuto", true) { None } else { Some(p.get_int("PolygonToolSpikiness", 38) as f32 / 100.0) },
            marquee_style: style,
            marquee_edge: edge("MarqueeToolAntiAliased", "MarqueeToolFeatherAmount"),
            lasso_edge: edge("LassoToolAntiAliased", "LassoToolFeatherAmount"),
            wand_tolerance: p.get_int("WandToolTolerance", 32).clamp(0, 255) as u32,
            wand_edge: edge("WandToolAntiAliased", "WandToolFeatherAmount"),
            fill_tolerance: p.get_int("FillToolTolerance", 32).clamp(0, 255) as u32,
            fill_selection: p.get_bool("FillToolFillsSelection", false),
            gradient_tolerance: p.get_int("GradientToolTolerance", 32).clamp(0, 255) as u32,
            gradient_fill_selection: p.get_bool("GradientToolFillsSelection", true),
            gradient_colors: default_gradient_colors(),
            brush_size: p.get_int("BrushTipSize", 1).max(1) as f32,
            brush_soft: false,
            pencil_antialias: p.get_bool("PencilAntiAlias", false),
            eraser_size: p.get_int("EraserToolDiameter", 13).max(1) as f32,
            eraser_round: p.get_bool("EraserToolIsCircle", true),
            eraser_opacity: p.get_int("EraserToolOpacity", 100).clamp(0, 100) as u8,
            eraser_edge: (p.get_int("EraserToolSoftEdge", 20).clamp(0, 1000) / 10) as u32,
            eyedropper_samples: match p.get_int("EyeDropperExtraSamples", 0) {
                1 => 3,
                2 => 5,
                _ => 1,
            },
            text_font: default_text_font(p),
            text_size: p.get_real("Text_Size", 12.0) as f32,
            text_color: p.get_color("Text_FillColor", black()),
            path_precision: p.get_int("PathToolPrecision", 5).clamp(1, 10) as u32,
            opacity: 100,
            new_fill: None,
            new_stroke: None,
            new_brush: None,
            auto_shape: "Abgerundetes Rechteck".into(),
        }
    }
}

/// Zeichenbefehl (`draw.*`) mit den Werkzeugattributen des Inspektors: ohne Attribute unverändert,
/// sonst `tool.shape` (Zeichnen + Füllung + Strich als **ein** Verlaufsschritt).
pub fn styled(inv: Invocation, opts: &ToolOptions) -> Invocation {
    if !inv.command.starts_with("draw.") || (opts.new_fill.is_none() && opts.new_stroke.is_none() && opts.new_brush.is_none()) {
        return inv;
    }
    Invocation::new(
        "tool.shape",
        serde_json::json!({"draw": inv.command, "params": inv.params, "fill": opts.new_fill, "stroke": opts.new_stroke, "brush": opts.new_brush}),
    )
}

// ------------------------------------------------------------------ Werkzeugtabelle

/// Die 49 Werkzeuge des Werkzeugbedienfelds (Registrierungsreihenfolge `@0068b4c0`, 10 §2.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ToolId {
    Pointer,
    SelectBehind,
    Subselection,
    Marquee,
    OvalMarquee,
    Lasso,
    PolygonLasso,
    Crop,
    ExportArea,
    MagicWand,
    Line,
    Pen,
    Rectangle,
    Ellipse,
    Polygon,
    AutoShape,
    Text,
    Pencil,
    VectorPath,
    RedrawPath,
    Brush,
    Scale,
    Skew,
    Distort,
    NineSliceScale,
    Freeform,
    Reshape,
    PathScrubAdd,
    PathScrubSubtract,
    Eyedropper,
    PaintBucket,
    Gradient,
    Eraser,
    Blur,
    Sharpen,
    Dodge,
    Burn,
    Smudge,
    RubberStamp,
    ReplaceColor,
    RedEyeRemoval,
    Knife,
    HotspotRect,
    HotspotCircle,
    HotspotPolygon,
    Slice,
    PolygonSlice,
    Hand,
    Zoom,
    /// KI-Auswahl (Agent KI): Objektauswahl, Motiv auswählen, Kante verfeinern – im Flyout des Zauberstabs.
    AiObject,
    AiSubject,
    AiRefineEdge,
}

/// Beschreibung eines Werkzeugs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToolInfo {
    pub id: ToolId,
    /// Stabiler Name (`tool.pointer` …).
    pub name: &'static str,
    /// Deutscher Name (Slot 4).
    pub label: &'static str,
    /// Klasse im Original.
    pub class: &'static str,
    /// Werkzeug-GUID (Slot 7; Schlüssel der Tastenkürzel-Sets).
    pub guid: &'static str,
    /// Flyout-Gruppe (Laufzeit T3).
    pub flyout: &'static str,
    /// Abschnitt im Werkzeugbedienfeld.
    pub section: &'static str,
    /// Hat einen Zustandsautomaten.
    pub implemented: bool,
}

macro_rules! tools_table {
    ($( $id:ident, $name:expr, $label:expr, $class:expr, $guid:expr, $fly:expr, $sec:expr, $imp:expr; )*) => {
        /// Alle Werkzeuge in Registrierungsreihenfolge.
        pub const TOOLS: &[ToolInfo] = &[
            $( ToolInfo { id: ToolId::$id, name: $name, label: $label, class: $class, guid: $guid, flyout: $fly, section: $sec, implemented: $imp }, )*
        ];
    };
}

tools_table! {
    Pointer, "tool.pointer", "Zeiger", "CSelectionTool", "{4e54aeee-c168-11d0-ba02-00059a80e4fd}", "Zeiger", "Auswählen", true;
    SelectBehind, "tool.selectBehind", "Hintere auswählen", "CSelectBehindTool", "{f4f7cbfa-c6f4-11d0-b598-000502701850}", "Zeiger", "Auswählen", true;
    Subselection, "tool.subselection", "Teilauswahl", "CSubSelectionTool", "{98bdebde-43c8-47ec-8699-76f6c211e055}", "Teilauswahl", "Auswählen", true;
    Marquee, "tool.marquee", "Auswahlrahmen", "CMarqueeTool", "{fda61c66-c6f4-11d0-b598-000502701850}", "Auswahlrahmen", "Bitmap", true;
    OvalMarquee, "tool.ovalMarquee", "Ovaler Auswahlrahmen", "COvalMarqueeTool", "{015621b2-c6f5-11d0-b598-000502701850}", "Auswahlrahmen", "Bitmap", true;
    Lasso, "tool.lasso", "Lasso", "CLassoTool", "{f9c58cf8-c6f4-11d0-b598-000502701850}", "Lasso", "Bitmap", true;
    PolygonLasso, "tool.polygonLasso", "Polygon-Lasso", "CPolyLassoTool", "{fb1bc11c-c6f4-11d0-b598-000502701850}", "Lasso", "Bitmap", true;
    Crop, "tool.crop", "Zuschneiden", "CCropDocumentTool", "{030d071e-c6f5-11d0-b598-000502701850}", "Zuschneiden", "Auswählen", true;
    ExportArea, "tool.exportArea", "Exportbereich", "CCropExportTool", "{030d073f-c6f6-11d0-b598-000502721851}", "Zuschneiden", "Auswählen", true;
    MagicWand, "tool.magicWand", "Zauberstab", "CWandTool", "{0491cc50-c6f5-11d0-b598-000502701850}", "Zauberstab", "Bitmap", true;
    AiObject, "tool.aiObject", "Objektauswahl", "FpAiObjectTool", "{6b1f3c2e-8d4a-4f6e-9a71-5c2d0e8b4a11}", "Zauberstab", "Bitmap", true;
    AiSubject, "tool.aiSubject", "Motiv auswählen", "FpAiSubjectTool", "{6b1f3c2e-8d4a-4f6e-9a71-5c2d0e8b4a12}", "Zauberstab", "Bitmap", true;
    AiRefineEdge, "tool.aiRefineEdge", "Kante verfeinern", "FpAiRefineEdgeTool", "{6b1f3c2e-8d4a-4f6e-9a71-5c2d0e8b4a13}", "Zauberstab", "Bitmap", true;
    Line, "tool.line", "Linie", "CLineTool", "{06190c96-c6f5-11d0-b598-000502701850}", "Linie", "Vektor", true;
    Pen, "tool.pen", "Stift", "CBezPenTool", "{d1216f78-25d2-11d1-beaf-00059a80e4fd}", "Stift", "Vektor", true;
    Rectangle, "tool.rectangle", "Rechteck", "CRectTool", "{07ceceae-c6f5-11d0-b598-000502701850}", "Rechteck", "Vektor", true;
    Ellipse, "tool.ellipse", "Ellipse", "COvalTool", "{097dd06a-c6f5-11d0-b598-000502701850}", "Rechteck", "Vektor", true;
    Polygon, "tool.polygon", "Polygon", "CStarTool", "{b5538880-ec84-11d0-b475-000502701850}", "Rechteck", "Vektor", true;
    AutoShape, "tool.autoShape", "Automatische Form", "CSmartShapeTool", "{24a62087-022f-43b2-9e44-0681904eba84}", "Rechteck", "Vektor", true;
    Text, "tool.text", "Text", "CTextTool", "{0b081bb6-c6f5-11d0-b598-000502701850}", "Text", "Vektor", true;
    Pencil, "tool.pencil", "Bleistift", "CPencilTool", "{7e7a7790-c6f7-11d0-9378-000502701850}", "Bleistift", "Bitmap", true;
    VectorPath, "tool.vectorPath", "Vektorpfad", "CPathTool", "{47c8e37e-c168-11d0-ba02-00059a80e4fd}", "Stift", "Vektor", true;
    RedrawPath, "tool.redrawPath", "Pfad nachzeichnen", "CPathTool", "{2f5b5766-f39c-11d0-b2ba-000502701850}", "Stift", "Vektor", true;
    Brush, "tool.brush", "Pinsel", "CBitmapPathTool", "{8f674c5e-47a3-4aa5-8984-08a0e6c88521}", "Pinsel", "Bitmap", true;
    Scale, "tool.scale", "Skalieren", "CRotateTool", "{8a2ee0f8-c6f7-11d0-9378-000502701850}", "Skalieren", "Auswählen", true;
    Skew, "tool.skew", "Neigen", "CSkewTool", "{81ab1b54-c6f7-11d0-9378-000502701850}", "Skalieren", "Auswählen", true;
    Distort, "tool.distort", "Verzerren", "CTransformTool", "{88dd5e14-c6f7-11d0-9378-000502701850}", "Skalieren", "Auswählen", true;
    NineSliceScale, "tool.nineSliceScale", "9-Slice-Skalierung", "C9ScaleTransformTool", "{4ce6b6a5-57b4-4d5a-aff4-ab88cf6c2ba9}", "Skalieren", "Auswählen", true;
    Freeform, "tool.freeform", "Freiform", "CEditFreeFormTool", "{c23a95ca-ec84-11d0-b475-000502701850}", "Freiform", "Vektor", true;
    Reshape, "tool.reshapeArea", "Bereich umformen", "CEditFreeFormTool", "{c6c8dc5a-ec84-11d0-b475-000502701850}", "Freiform", "Vektor", true;
    PathScrubAdd, "tool.pathScrubAdd", "Pfadausbesserung - additiv", "CScrubTool", "{ef34a56a-157a-11d1-8840-000502701850}", "Freiform", "Vektor", true;
    PathScrubSubtract, "tool.pathScrubSubtract", "Pfadausbesserung - subtraktiv", "CScrubRemoveTool", "{22798f8a-157b-11d1-8840-000502701850}", "Freiform", "Vektor", true;
    Eyedropper, "tool.eyedropper", "Pipette", "CEyedropperTool", "{83242ee4-c6f7-11d0-9378-000502701850}", "Pipette", "Farben", true;
    PaintBucket, "tool.paintBucket", "Füllwerkzeug", "CPaintbucketTool", "{84ba1638-c6f7-11d0-9378-000502701850}", "Füllwerkzeug", "Farben", true;
    Gradient, "tool.gradient", "Verlauf", "CGradientTool", "{80e51dc3-70b5-4d97-a5d0-8e08d5e89d20}", "Füllwerkzeug", "Farben", true;
    Eraser, "tool.eraser", "Radiergummi", "CEraserTool", "{8621c250-c6f7-11d0-9378-000502701850}", "Radiergummi", "Bitmap", true;
    Blur, "tool.blur", "Weichzeichner", "CBlurTool", "{fdd4cc1c-db25-42ec-8398-f3243982341e}", "Weichzeichner", "Bitmap", true;
    Sharpen, "tool.sharpen", "Scharfzeichner", "CSharpenTool", "{9eaed3be-3b9f-4a6f-adac-5c3c95d6e3d6}", "Weichzeichner", "Bitmap", true;
    Dodge, "tool.dodge", "Abwedler", "CDodgeTool", "{6fe2c2ae-9f64-4045-a84f-2606afec8df5}", "Weichzeichner", "Bitmap", true;
    Burn, "tool.burn", "Nachbelichter", "CBurnTool", "{5b769684-01d2-4413-abc5-c740c2c3dfff}", "Weichzeichner", "Bitmap", true;
    Smudge, "tool.smudge", "Verwischen", "CSmudgeTool", "{c7f9aaec-2840-4543-9ecd-416a9374695c}", "Weichzeichner", "Bitmap", true;
    RubberStamp, "tool.rubberStamp", "Stempel", "CStampTool", "{8789476c-c6f7-11d0-9378-000502701850}", "Stempel", "Bitmap", true;
    ReplaceColor, "tool.replaceColor", "Farbe ersetzen", "CReplaceColorTool", "{6127ddbf-54f5-43ba-b34d-0fe2fbf050d8}", "Stempel", "Bitmap", true;
    RedEyeRemoval, "tool.redEyeRemoval", "Rote Augen entfernen", "CRedEyeRemovalTool", "{3b81dcba-530b-4c66-a323-7838602e7aaf}", "Stempel", "Bitmap", true;
    Knife, "tool.knife", "Messer", "CKnifeTool", "{a93bdfd9-eb98-49a6-b2f2-68f74159ea9a}", "Messer", "Vektor", true;
    HotspotRect, "tool.hotspotRect", "Hotspot-Rechteck", "COvalURLTool", "{3b9abea6-cd49-11d0-b2d7-000502751952}", "Hotspot", "Web", true;
    HotspotCircle, "tool.hotspotCircle", "Hotspot-Kreis", "COvalURLTool", "{097def6a-c6f5-11d0-b798-000502751851}", "Hotspot", "Web", true;
    HotspotPolygon, "tool.hotspotPolygon", "Hotspot-Polygon", "CPolyURLTool", "{3b9abea6-cd49-11d0-b2d7-000502751852}", "Hotspot", "Web", true;
    Slice, "tool.slice", "Segment", "COvalURLTool", "{3b9abea6-cd49-11d0-b2d7-000502752052}", "Segment", "Web", true;
    PolygonSlice, "tool.polygonSlice", "Polygon-Segment", "CPolySliceTool", "{197dd06a-c6f4-11d1-b298-000502701850}", "Segment", "Web", true;
    Hand, "tool.hand", "Hand", "CPanTool", "{41bf2ccc-c168-11d0-ba02-00059a80e4fd}", "Hand", "Ansicht", true;
    Zoom, "tool.zoom", "Zoom", "CZoomTool", "{54ed11ba-c168-11d0-ba02-00059a80e4fd}", "Zoom", "Ansicht", true;
}

/// Alte Teilauswahl-GUID der Sets, die das Original zur Laufzeit auf `CSubSelectionTool` abbildet (4a T5).
pub const LEGACY_SUBSELECT_GUID: &str = "{4e54aeef-c168-11d0-ba02-00059a80e4fd}";

/// Beschreibung eines Werkzeugs.
pub fn info(id: ToolId) -> Option<&'static ToolInfo> {
    TOOLS.iter().find(|t| t.id == id)
}

/// Werkzeug zum stabilen Namen (`tool.pen`) oder deutschen Namen.
pub fn by_name(name: &str) -> Option<ToolId> {
    TOOLS.iter().find(|t| t.name == name || t.label.eq_ignore_ascii_case(name) || t.name.trim_start_matches("tool.").eq_ignore_ascii_case(name)).map(|t| t.id)
}

/// Werkzeug zur GUID (Schreibweise egal; alte Teilauswahl-GUID → Teilauswahl).
pub fn by_guid(guid: &str) -> Option<ToolId> {
    let g = crate::shortcuts::normalize_guid(guid);
    if g == LEGACY_SUBSELECT_GUID {
        return Some(ToolId::Subselection);
    }
    TOOLS.iter().find(|t| t.guid == g).map(|t| t.id)
}

/// Werkzeuge einer Flyout-Gruppe in Reihenfolge.
pub fn flyout_members(flyout: &str) -> Vec<ToolId> {
    TOOLS.iter().filter(|t| t.flyout == flyout).map(|t| t.id).collect()
}

/// Werkzeug-GUIDs ohne eigenes Werkzeug, deren Kürzel bei einem Flyout landen: abgerundetes
/// Rechteck (U im Satz „Photoshop“ → Rechteck-Flyout) und das Segment-artige Web-Werkzeug (K).
pub const LEGACY_ROUNDRECT_GUID: &str = "{bb05205e-ec84-11d0-b475-000502701850}";
pub const LEGACY_SLICE_GUID: &str = "{6d8765da-5023-493a-9269-9d7a7bba9df5}";

/// Pseudo-Werkzeuge der Kürzelsätze (Taste → Befehl, 10 §2.3): Standardfarben (D), Farben
/// tauschen (X), Bildschirmmodus umschalten (F), Segmente ein-/ausblenden (2).
pub const PSEUDO_TOOLS: &[(&str, &str)] = &[
    ("{5c3ab743-15e6-4886-a8fb-611275acd9ff}", "color.defaults"),
    ("{dbf6dd93-314d-4fdc-899a-b75e7fa4229b}", "color.swap"),
    ("{d5d2ad45-1121-430d-95cc-7482c5efd97e}", "view.toggleScreenMode"),
    ("{3f288332-3d0d-4998-8df5-53534060de4f}", "view.toggleSlices"),
];

/// Befehl eines Pseudo-Werkzeug-Kürzels im Set.
pub fn pseudo_command_for_key(set: &ShortcutSet, sc: &Shortcut) -> Option<&'static str> {
    PSEUDO_TOOLS.iter().find(|(g, _)| set.tool(g).contains(sc)).map(|(_, c)| *c)
}

/// Kürzel eines Werkzeugs im Set.
pub fn tool_shortcuts(set: &ShortcutSet, id: ToolId) -> Vec<Shortcut> {
    let Some(i) = info(id) else { return Vec::new() };
    let mut v = set.tool(i.guid).to_vec();
    match id {
        ToolId::Subselection => v.extend_from_slice(set.tool(LEGACY_SUBSELECT_GUID)),
        ToolId::Rectangle => v.extend_from_slice(set.tool(LEGACY_ROUNDRECT_GUID)),
        ToolId::Slice => v.extend_from_slice(set.tool(LEGACY_SLICE_GUID)),
        _ => {}
    }
    v
}

/// Nächstes Werkzeug für ein Werkzeugkürzel: Mehrere Werkzeuge mit gleichem Kürzel werden beim
/// wiederholten Drücken durchgeschaltet (10 §2.1, A).
pub fn cycle_tool_for_key(set: &ShortcutSet, sc: &Shortcut, current: ToolId) -> Option<ToolId> {
    if sc.has_modifiers() {
        return None;
    }
    let mut cands: Vec<ToolId> = Vec::new();
    for t in TOOLS {
        if tool_shortcuts(set, t.id).contains(sc) && !cands.contains(&t.id) {
            cands.push(t.id);
        }
    }
    if cands.is_empty() {
        return None;
    }
    match cands.iter().position(|t| *t == current) {
        Some(i) => Some(cands[(i + 1) % cands.len()]),
        None => Some(cands[0]),
    }
}

/// Zustandsautomat für ein Werkzeug.
pub fn make_machine(id: ToolId) -> Box<dyn ToolMachine> {
    match id {
        ToolId::Pointer | ToolId::SelectBehind | ToolId::Subselection => Box::new(PointerTool::new(id)),
        ToolId::Scale | ToolId::Skew | ToolId::Distort | ToolId::NineSliceScale => Box::new(TransformTool::new(id)),
        ToolId::Crop | ToolId::ExportArea => Box::new(CropTool::new(id)),
        ToolId::Marquee | ToolId::OvalMarquee => Box::new(MarqueeTool::new(id)),
        ToolId::Lasso | ToolId::PolygonLasso => Box::new(LassoTool::new(id)),
        ToolId::MagicWand => Box::new(WandTool),
        ToolId::AiObject => Box::new(AiObjectTool::default()),
        ToolId::AiSubject | ToolId::AiRefineEdge => Box::new(AiClickTool::new(id)),
        ToolId::Rectangle | ToolId::Ellipse | ToolId::Polygon | ToolId::AutoShape => Box::new(ShapeTool::new(id)),
        ToolId::Line => Box::new(LineTool::default()),
        ToolId::Pen => Box::new(PenTool::default()),
        ToolId::VectorPath | ToolId::RedrawPath => Box::new(FreehandTool::new(id)),
        ToolId::Brush | ToolId::Pencil | ToolId::Eraser => Box::new(BrushTool::new(id)),
        ToolId::Blur | ToolId::Sharpen | ToolId::Dodge | ToolId::Burn | ToolId::Smudge | ToolId::RubberStamp | ToolId::ReplaceColor | ToolId::RedEyeRemoval => {
            Box::new(RetouchTool::new(id))
        }
        ToolId::Knife => Box::new(KnifeTool::default()),
        ToolId::Freeform | ToolId::Reshape => Box::new(FreeformTool::new(id)),
        ToolId::PathScrubAdd | ToolId::PathScrubSubtract => Box::new(ScrubTool::new(id)),
        ToolId::Text => Box::new(TextTool::default()),
        ToolId::Eyedropper => Box::new(EyedropperTool),
        ToolId::PaintBucket | ToolId::Gradient => Box::new(FillTool::new(id)),
        ToolId::HotspotRect | ToolId::HotspotCircle | ToolId::HotspotPolygon | ToolId::Slice | ToolId::PolygonSlice => Box::new(WebShapeTool::new(id)),
        ToolId::Hand => Box::new(HandTool::default()),
        ToolId::Zoom => Box::new(ZoomTool::default()),
    }
}

// ------------------------------------------------------------------ gemeinsame Hilfen

/// Rechteck aus Start- und Endpunkt mit den Modifikatoren des Originals: Umschalt = Quadrat, Alt = Start
/// ist die Mitte.
pub fn constrain_rect(start: Point, cur: Point, square: bool, from_center: bool) -> Rect {
    let mut dx = cur.x - start.x;
    let mut dy = cur.y - start.y;
    if square {
        let s = dx.abs().max(dy.abs());
        dx = if dx < 0.0 { -s } else { s };
        dy = if dy < 0.0 { -s } else { s };
    }
    if from_center {
        Rect::new(start.x - dx.abs(), start.y - dy.abs(), start.x + dx.abs(), start.y + dy.abs())
    } else {
        Rect::new(start.x, start.y, start.x + dx, start.y + dy).normalized()
    }
}

/// Endpunkt auf Winkelraster (`step_deg`, z. B. 45° für Linien) einrasten.
pub fn constrain_angle(start: Point, cur: Point, step_deg: f64) -> Point {
    let (dx, dy) = ((cur.x - start.x) as f64, (cur.y - start.y) as f64);
    let len = dx.hypot(dy);
    if len == 0.0 {
        return cur;
    }
    let step = step_deg.to_radians();
    let a = (dy.atan2(dx) / step).round() * step;
    if (step_deg - 45.0).abs() < 1e-9 {
        // wie im Original: auf die Achse projizieren (0°/90°: die andere Komponente fällt weg,
        // 45°: d = (|dx| + |dy|) / 2 je Achse) – ganzzahlige Mausbewegung bleibt ganzzahlig
        let (ux, uy) = (a.cos().round(), a.sin().round());
        if ux == 0.0 || uy == 0.0 {
            return Point::new((start.x as f64 + dx * ux.abs()) as f32, (start.y as f64 + dy * uy.abs()) as f32);
        }
        let d = (dx * ux + dy * uy) / 2.0;
        return Point::new((start.x as f64 + d * ux) as f32, (start.y as f64 + d * uy) as f32);
    }
    Point::new((start.x as f64 + len * a.cos()) as f32, (start.y as f64 + len * a.sin()) as f32)
}

/// Abstand zweier Punkte.
pub fn dist(a: Point, b: Point) -> f32 {
    (a.x - b.x).hypot(a.y - b.y)
}

/// Punkt als JSON `[x, y]`.
pub fn pt_json(p: Point) -> Value {
    serde_json::json!([p.x, p.y])
}

/// Rechteck als JSON.
pub fn rect_json(r: Rect) -> Value {
    serde_json::json!({"left": r.left, "top": r.top, "right": r.right, "bottom": r.bottom})
}

/// Zahl für Beschriftungen: ganze Zahlen ohne Nachkommastellen, sonst eine Stelle (deutsches Komma).
pub fn fmt_num(v: f32) -> String {
    if (v - v.round()).abs() < 0.05 {
        format!("{:.0}", v.round())
    } else {
        // Dezimaltrennzeichen der Oberflächensprache (Deutsch „,“ wie bisher)
        fp_i18n::fmt_decimal(v as f64, 1)
    }
}

/// Beschriftung „B × H“ unten rechts an einem Rechteck (Rückmeldung beim Aufziehen).
pub fn size_label(r: Rect) -> Overlay {
    let r = r.normalized();
    Overlay::Label { at: Point::new(r.right, r.bottom), text: format!("{} × {}", fmt_num(r.width()), fmt_num(r.height())) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_and_shortcuts() {
        assert_eq!(TOOLS.len(), 52);
        let set = crate::shortcuts::builtin_set("Firepixel").cloned().unwrap_or_default();
        // R schaltet Rechteck ↔ Ellipse … (deutsches Set: R = Rechteck/Ellipse)
        let r = Shortcut::key(b'R' as u16);
        let first = cycle_tool_for_key(&set, &r, ToolId::Pointer);
        assert!(first.is_some());
        let second = first.and_then(|t| cycle_tool_for_key(&set, &r, t));
        assert_ne!(first, second);
        assert_eq!(by_guid("{4e54aeef-c168-11d0-ba0200059a80e4fd}"), Some(ToolId::Subselection));
        let rr = constrain_rect(Point::new(10.0, 10.0), Point::new(20.0, 15.0), true, false);
        assert_eq!((rr.width(), rr.height()), (10.0, 10.0));
        let c = constrain_rect(Point::new(10.0, 10.0), Point::new(20.0, 15.0), false, true);
        assert_eq!(c, Rect::new(0.0, 5.0, 20.0, 15.0));
        let a = constrain_angle(Point::new(0.0, 0.0), Point::new(10.0, 1.0), 45.0);
        assert!((a.y).abs() < 1e-4);
    }
}
