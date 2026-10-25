//! Eigene Vektor-Icons (Clean Room): alle Symbole sind hier im Code gezeichnet, auf einem Raster von
//! 16 × 16 Einheiten, und werden auf jede Größe skaliert. Keine Grafik stammt aus dem Original.
//!
//! Form, Größe und Lage folgen funktional den Originalen (Pfeil für Zeiger, gestrichelter Rahmen für
//! den Auswahlrahmen, Feder für den Stift …), damit man sich sofort zurechtfindet. Die Farben kommen
//! aus [`IconStyle`], das aus dem aktiven [`Theme`] abgeleitet wird: im Normal- und Dunkel-Design
//! überwiegend einfarbig mit kleinen farbigen Akzenten, im OLED-Design kräftig farbig in der
//! Akzentfarbe der Werkzeuggruppe.

use crate::theme::{Theme, ToolGroup, mix};
use egui::epaint::{Mesh, PathShape, PathStroke};
use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, pos2};
use fp_engine::ToolId;

/// Alle Icons der Oberfläche.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Icon {
    // Werkzeuge
    Pointer,
    SelectBehind,
    Subselection,
    /// Mask Element Selection (FB-80): Subselection arrow with a mask badge.
    MaskElementSelect,
    Scale,
    Skew,
    Distort,
    NineSlice,
    Crop,
    ExportArea,
    Marquee,
    OvalMarquee,
    Lasso,
    PolygonLasso,
    MagicWand,
    Brush,
    Pencil,
    Eraser,
    Blur,
    Sharpen,
    Dodge,
    Burn,
    Smudge,
    RubberStamp,
    ReplaceColor,
    RedEye,
    Line,
    Pen,
    VectorPath,
    RedrawPath,
    Rectangle,
    Ellipse,
    Polygon,
    AutoShape,
    Text,
    Freeform,
    Reshape,
    PathScrubAdd,
    PathScrubSubtract,
    Knife,
    HotspotRect,
    HotspotCircle,
    HotspotPolygon,
    Slice,
    PolygonSlice,
    HideSlices,
    ShowSlices,
    Eyedropper,
    PaintBucket,
    Gradient,
    Hand,
    Zoom,
    // Farben und Ansicht im Werkzeugbedienfeld
    StrokeColor,
    FillColor,
    DefaultColors,
    NoColor,
    SwapColors,
    ScreenStandard,
    ScreenFullMenus,
    ScreenFull,
    // Hauptsymbolleiste
    New,
    Save,
    Open,
    Import,
    Export,
    Print,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Group,
    Ungroup,
    Join,
    Split,
    BringToFront,
    BringForward,
    SendBackward,
    SendToBack,
    EditSymbol,
    AlignLeft,
    RotateCcw,
    RotateCw,
    FlipHorizontal,
    FlipVertical,
    // Bedienfelder, Fußleiste, Rahmen
    PanelMenu,
    CollapseRight,
    CollapseLeft,
    NewPage,
    Trash,
    NewLayer,
    NewSublayer,
    NewBitmap,
    AddMask,
    Duplicate,
    Eye,
    Lock,
    Folder,
    TriangleRight,
    TriangleDown,
    FirstFrame,
    PrevFrame,
    Play,
    /// Stop (the Play button while an animation plays, FB-63).
    Stop,
    NextFrame,
    LastFrame,
    ExitBitmapMode,
    ViewOriginal,
    ViewPreview,
    View2Up,
    View4Up,
    PageSmall,
    PageNavigate,
    Close,
    DropDown,
    Check,
    Browse,
    Info,
    Plus,
    Minus,
    AddFilter,
    LinkProportions,
    Document,
    DistributeStates,
    OnionSkin,
    Loop,
    Minimize,
    Maximize,
    Restore,
    WindowClose,
    Logo,
    Star,
    Swatches,
    // KI-Auswahlwerkzeuge (Flyout am Zauberstab)
    AiObject,
    AiSubject,
    AiRefineEdge,
}

/// Farben für ein Icon.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IconStyle {
    /// Linien und Hauptflächen.
    pub fg: Color32,
    /// Helle Nebenlinien, Schatten.
    pub light: Color32,
    /// Papier-/Füllflächen („weiß“).
    pub paper: Color32,
    /// Farbiger Akzent.
    pub accent: Color32,
    /// Zweiter Akzent.
    pub accent2: Color32,
    pub red: Color32,
    pub green: Color32,
    pub blue: Color32,
    pub yellow: Color32,
}

impl IconStyle {
    /// Stil für ein Icon einer Werkzeuggruppe (bzw. ohne Gruppe) im Zustand `enabled`.
    pub fn new(theme: &Theme, group: Option<ToolGroup>, enabled: bool) -> IconStyle {
        let c = &theme.colors;
        let mut st = if theme.colorful_icons {
            let accent = group.map(|g| theme.accent(g)).unwrap_or(c.icon_accent);
            IconStyle {
                fg: accent,
                light: mix(accent, Color32::BLACK, 0.55),
                paper: mix(accent, Color32::BLACK, 0.82),
                accent: if group == Some(ToolGroup::Bitmap) {
                    c.icon_yellow
                } else {
                    c.icon_accent2
                },
                accent2: c.icon_yellow,
                red: c.icon_red,
                green: c.icon_green,
                blue: c.icon_blue,
                yellow: c.icon_yellow,
            }
        } else {
            IconStyle {
                fg: c.icon_fg,
                light: c.icon_light,
                paper: c.icon_paper,
                accent: c.icon_accent,
                accent2: c.icon_accent2,
                red: c.icon_red,
                green: c.icon_green,
                blue: c.icon_blue,
                yellow: c.icon_yellow,
            }
        };
        if !enabled {
            let d = c.icon_disabled;
            st = IconStyle {
                fg: d,
                light: mix(d, c.frame_bg, 0.5),
                paper: c.frame_bg,
                accent: d,
                accent2: d,
                red: d,
                green: d,
                blue: d,
                yellow: d,
            };
        }
        st
    }

    /// Stil in einer festen Farbe (z. B. für Pfeile in Kopfzeilen).
    pub fn mono(color: Color32, paper: Color32) -> IconStyle {
        IconStyle {
            fg: color,
            light: color,
            paper,
            accent: color,
            accent2: color,
            red: color,
            green: color,
            blue: color,
            yellow: color,
        }
    }
}

/// Icon eines Werkzeugs.
pub fn tool_icon(id: ToolId) -> Icon {
    match id {
        ToolId::Pointer => Icon::Pointer,
        ToolId::SelectBehind => Icon::SelectBehind,
        ToolId::Subselection => Icon::Subselection,
        ToolId::MaskElementSelect => Icon::MaskElementSelect,
        ToolId::Marquee => Icon::Marquee,
        ToolId::OvalMarquee => Icon::OvalMarquee,
        ToolId::Lasso => Icon::Lasso,
        ToolId::PolygonLasso => Icon::PolygonLasso,
        ToolId::Crop => Icon::Crop,
        ToolId::ExportArea => Icon::ExportArea,
        ToolId::MagicWand => Icon::MagicWand,
        ToolId::Line => Icon::Line,
        ToolId::Pen => Icon::Pen,
        ToolId::Rectangle => Icon::Rectangle,
        ToolId::Ellipse => Icon::Ellipse,
        ToolId::Polygon => Icon::Polygon,
        ToolId::AutoShape => Icon::AutoShape,
        ToolId::Text => Icon::Text,
        ToolId::Pencil => Icon::Pencil,
        ToolId::VectorPath => Icon::VectorPath,
        ToolId::RedrawPath => Icon::RedrawPath,
        ToolId::Brush => Icon::Brush,
        ToolId::Scale => Icon::Scale,
        ToolId::Skew => Icon::Skew,
        ToolId::Distort => Icon::Distort,
        ToolId::NineSliceScale => Icon::NineSlice,
        ToolId::Freeform => Icon::Freeform,
        ToolId::Reshape => Icon::Reshape,
        ToolId::PathScrubAdd => Icon::PathScrubAdd,
        ToolId::PathScrubSubtract => Icon::PathScrubSubtract,
        ToolId::Eyedropper => Icon::Eyedropper,
        ToolId::PaintBucket => Icon::PaintBucket,
        ToolId::Gradient => Icon::Gradient,
        ToolId::Eraser => Icon::Eraser,
        ToolId::Blur => Icon::Blur,
        ToolId::Sharpen => Icon::Sharpen,
        ToolId::Dodge => Icon::Dodge,
        ToolId::Burn => Icon::Burn,
        ToolId::Smudge => Icon::Smudge,
        ToolId::RubberStamp => Icon::RubberStamp,
        ToolId::ReplaceColor => Icon::ReplaceColor,
        ToolId::RedEyeRemoval => Icon::RedEye,
        ToolId::Knife => Icon::Knife,
        ToolId::HotspotRect => Icon::HotspotRect,
        ToolId::HotspotCircle => Icon::HotspotCircle,
        ToolId::HotspotPolygon => Icon::HotspotPolygon,
        ToolId::Slice => Icon::Slice,
        ToolId::PolygonSlice => Icon::PolygonSlice,
        ToolId::Hand => Icon::Hand,
        ToolId::Zoom => Icon::Zoom,
        // KI-Auswahl (Agent KI): vorläufig das Zauberstab-Symbol, eigene Icons siehe BEFUNDE (HG)
        ToolId::AiObject => Icon::AiObject,
        ToolId::AiSubject => Icon::AiSubject,
        ToolId::AiRefineEdge => Icon::AiRefineEdge,
    }
}

/// Zeichnet ein Icon in `rect` (quadratisch empfohlen; das 16er-Raster wird eingepasst).
pub fn paint(painter: &Painter, rect: Rect, icon: Icon, st: &IconStyle) {
    let side = rect.width().min(rect.height());
    let s = side / 16.0;
    let o = pos2(rect.center().x - side / 2.0, rect.center().y - side / 2.0);
    let p = Pen {
        painter,
        o,
        s,
        st: *st,
    };
    draw(&p, icon);
}

/// Das App-Icon (eigenes Design: Flamme aus Pixeln, die in Funken zerfällt) in allen gerenderten
/// Größen; Quelle `assets\icon\firepixel.svg` bzw. `assets\icon\icon_erzeugen.py`.
pub const APP_ICON_PNGS: &[(u32, &[u8])] = &[
    (
        16,
        include_bytes!("../../../assets/icon/png/firepixel_16.png"),
    ),
    (
        20,
        include_bytes!("../../../assets/icon/png/firepixel_20.png"),
    ),
    (
        24,
        include_bytes!("../../../assets/icon/png/firepixel_24.png"),
    ),
    (
        32,
        include_bytes!("../../../assets/icon/png/firepixel_32.png"),
    ),
    (
        40,
        include_bytes!("../../../assets/icon/png/firepixel_40.png"),
    ),
    (
        48,
        include_bytes!("../../../assets/icon/png/firepixel_48.png"),
    ),
    (
        64,
        include_bytes!("../../../assets/icon/png/firepixel_64.png"),
    ),
    (
        96,
        include_bytes!("../../../assets/icon/png/firepixel_96.png"),
    ),
    (
        128,
        include_bytes!("../../../assets/icon/png/firepixel_128.png"),
    ),
    (
        256,
        include_bytes!("../../../assets/icon/png/firepixel_256.png"),
    ),
];

/// App-Icon als Fenster-/Taskleisten-Icon (256 px).
pub fn app_icon_data() -> egui::IconData {
    let bytes = APP_ICON_PNGS[APP_ICON_PNGS.len() - 1].1;
    eframe::icon_data::from_png_bytes(bytes).unwrap_or_default()
}

/// Kleinste gerenderte Größe ≥ `want` px als winit-Icon (sonst die größte).
#[cfg(target_os = "windows")]
fn winit_icon(want: f64) -> Option<winit::window::Icon> {
    let want = want.round().max(1.0) as u32;
    let (_, bytes) = APP_ICON_PNGS
        .iter()
        .find(|(s, _)| *s >= want)
        .or(APP_ICON_PNGS.last())?;
    let d = eframe::icon_data::from_png_bytes(bytes).ok()?;
    winit::window::Icon::from_rgba(d.rgba, d.width, d.height).ok()
}

/// Setzt Fenstersymbol (klein, 16 px × DPI) und Taskleisten-Symbol (groß, 32 px × DPI) aus den
/// pixelgenauen Größen. winit setzt über `with_icon` nur ICON_SMALL; ohne ICON_BIG zeigt die
/// Taskleiste kein Programmsymbol.
#[cfg(target_os = "windows")]
pub fn apply_window_icons(window: &winit::window::Window) {
    use winit::platform::windows::WindowExtWindows as _;
    let scale = window.scale_factor();
    window.set_window_icon(winit_icon(16.0 * scale));
    window.set_taskbar_icon(winit_icon(32.0 * scale));
}

/// Zeichnet das App-Logo in `rect` (quadratisch eingepasst). Passt die Größe in Gerätepixeln genau
/// zu einer gerenderten Stufe, wird diese pixelgenau gezeichnet (16/20/24/32 px sind eigene
/// Pixelvarianten), sonst die nächstgrößere Stufe verkleinert.
pub fn paint_app_logo(painter: &Painter, rect: Rect) {
    let side = rect.width().min(rect.height());
    if side <= 0.0 {
        return;
    }
    let ctx = painter.ctx();
    let ppp = ctx.pixels_per_point();
    let want = (side * ppp).round() as u32;
    let (size, bytes) = APP_ICON_PNGS
        .iter()
        .copied()
        .find(|(s, _)| *s >= want)
        .unwrap_or(APP_ICON_PNGS[APP_ICON_PNGS.len() - 1]);
    let exact = size == want;
    let id = egui::Id::new(("fp-app-logo", size, exact));
    let tex = ctx
        .data_mut(|d| d.get_temp::<egui::TextureHandle>(id))
        .or_else(|| {
            let icon = eframe::icon_data::from_png_bytes(bytes).ok()?;
            let img = egui::ColorImage::from_rgba_unmultiplied(
                [icon.width as usize, icon.height as usize],
                &icon.rgba,
            );
            let opts = if exact {
                egui::TextureOptions::NEAREST
            } else {
                egui::TextureOptions::LINEAR
            };
            let t = ctx.load_texture(format!("fp-app-logo-{size}"), img, opts);
            ctx.data_mut(|d| d.insert_temp(id, t.clone()));
            Some(t)
        });
    let Some(tex) = tex else { return };
    // auf Gerätepixel ausrichten, damit die Pixelvarianten scharf bleiben
    let min = pos2(
        ((rect.center().x - side / 2.0) * ppp).round() / ppp,
        ((rect.center().y - side / 2.0) * ppp).round() / ppp,
    );
    let r = Rect::from_min_size(min, egui::vec2(want as f32 / ppp, want as f32 / ppp));
    painter.image(
        tex.id(),
        r,
        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
        Color32::WHITE,
    );
}

/// Zeichen-Hilfe im 16er-Raster.
struct Pen<'a> {
    painter: &'a Painter,
    o: Pos2,
    s: f32,
    st: IconStyle,
}

type Pt = (f32, f32);

impl Pen<'_> {
    fn pt(&self, p: Pt) -> Pos2 {
        pos2(self.o.x + p.0 * self.s, self.o.y + p.1 * self.s)
    }
    fn w(&self, units: f32) -> f32 {
        (units * self.s).max(1.0)
    }
    /// Offene Linie.
    fn line(&self, c: Color32, pts: &[Pt]) {
        self.line_w(c, 1.0, pts);
    }
    fn line_w(&self, c: Color32, width: f32, pts: &[Pt]) {
        let v: Vec<Pos2> = pts.iter().map(|&q| self.pt(q)).collect();
        self.painter
            .add(Shape::line(v, Stroke::new(self.w(width), c)));
    }
    /// Geschlossener Umriss.
    fn outline(&self, c: Color32, pts: &[Pt]) {
        let v: Vec<Pos2> = pts.iter().map(|&q| self.pt(q)).collect();
        self.painter.add(Shape::Path(PathShape {
            points: v,
            closed: true,
            fill: Color32::TRANSPARENT,
            stroke: PathStroke::new(self.w(1.0), c),
        }));
    }
    /// Gefülltes Polygon (auch konkav) mit optionalem Umriss.
    fn poly(&self, fill: Option<Color32>, stroke: Option<Color32>, pts: &[Pt]) {
        let v: Vec<Pos2> = pts.iter().map(|&q| self.pt(q)).collect();
        if let Some(f) = fill {
            fill_polygon(self.painter, &v, f);
        }
        if let Some(c) = stroke {
            self.painter.add(Shape::Path(PathShape {
                points: v,
                closed: true,
                fill: Color32::TRANSPARENT,
                stroke: PathStroke::new(self.w(1.0), c),
            }));
        }
    }
    /// Rechteck (x, y, b, h) mit Füllung/Umriss; Umrisse liegen auf halben Einheiten.
    fn rect(&self, x: f32, y: f32, w: f32, h: f32, fill: Option<Color32>, stroke: Option<Color32>) {
        let r = Rect::from_min_max(self.pt((x, y)), self.pt((x + w, y + h)));
        if let Some(f) = fill {
            self.painter.rect_filled(r, 0.0, f);
        }
        if let Some(c) = stroke {
            self.painter.rect_stroke(
                r,
                0.0,
                Stroke::new(self.w(1.0), c),
                egui::StrokeKind::Inside,
            );
        }
    }
    fn circle(&self, c: Pt, r: f32, fill: Option<Color32>, stroke: Option<Color32>) {
        let center = self.pt(c);
        if let Some(f) = fill {
            self.painter.circle_filled(center, r * self.s, f);
        }
        if let Some(col) = stroke {
            self.painter
                .circle_stroke(center, r * self.s, Stroke::new(self.w(1.0), col));
        }
    }
    fn ellipse(&self, c: Pt, rx: f32, ry: f32, fill: Option<Color32>, stroke: Option<Color32>) {
        let pts: Vec<Pt> = (0..32)
            .map(|i| {
                let a = i as f32 / 32.0 * std::f32::consts::TAU;
                (c.0 + rx * a.cos(), c.1 + ry * a.sin())
            })
            .collect();
        self.poly(fill, stroke, &pts);
    }
    /// Kreisbogen (Winkel in Grad, 0 = rechts, im Uhrzeigersinn wegen y nach unten).
    fn arc(&self, col: Color32, c: Pt, r: f32, a0: f32, a1: f32) {
        let n = 16;
        let pts: Vec<Pt> = (0..=n)
            .map(|i| {
                let a = (a0 + (a1 - a0) * i as f32 / n as f32).to_radians();
                (c.0 + r * a.cos(), c.1 + r * a.sin())
            })
            .collect();
        self.line(col, &pts);
    }
    /// Gestrichelte Linie.
    fn dashed(&self, col: Color32, a: Pt, b: Pt, dash: f32) {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = (dx * dx + dy * dy).sqrt();
        if len <= 0.0 {
            return;
        }
        let n = (len / dash).ceil() as usize;
        for i in (0..n).step_by(2) {
            let t0 = i as f32 * dash / len;
            let t1 = ((i + 1) as f32 * dash / len).min(1.0);
            self.line(
                col,
                &[
                    (a.0 + dx * t0, a.1 + dy * t0),
                    (a.0 + dx * t1, a.1 + dy * t1),
                ],
            );
        }
    }
    fn dashed_rect(&self, col: Color32, x: f32, y: f32, w: f32, h: f32) {
        self.dashed(col, (x, y), (x + w, y), 2.0);
        self.dashed(col, (x + w, y), (x + w, y + h), 2.0);
        self.dashed(col, (x + w, y + h), (x, y + h), 2.0);
        self.dashed(col, (x, y + h), (x, y), 2.0);
    }
    /// Kleiner Text (Buchstaben-Icons wie „T“ werden als Flächen gezeichnet, nicht als Schrift).
    fn arrow_head(&self, col: Color32, tip: Pt, dir: Pt, size: f32) {
        let len = (dir.0 * dir.0 + dir.1 * dir.1).sqrt().max(1e-6);
        let (ux, uy) = (dir.0 / len, dir.1 / len);
        let (px, py) = (-uy, ux);
        let base = (tip.0 - ux * size, tip.1 - uy * size);
        self.poly(
            Some(col),
            None,
            &[
                tip,
                (base.0 + px * size * 0.6, base.1 + py * size * 0.6),
                (base.0 - px * size * 0.6, base.1 - py * size * 0.6),
            ],
        );
    }
}

/// Füllt ein einfaches (auch konkaves) Polygon per Ohrenschneiden.
fn fill_polygon(painter: &Painter, pts: &[Pos2], color: Color32) {
    if pts.len() < 3 {
        return;
    }
    let mut mesh = Mesh::default();
    for p in pts {
        mesh.colored_vertex(*p, color);
    }
    // Orientierung bestimmen
    let area: f32 = (0..pts.len())
        .map(|i| {
            let a = pts[i];
            let b = pts[(i + 1) % pts.len()];
            a.x * b.y - b.x * a.y
        })
        .sum();
    let ccw = area > 0.0;
    let mut idx: Vec<usize> = (0..pts.len()).collect();
    let mut guard = 0;
    while idx.len() > 3 && guard < 10_000 {
        guard += 1;
        let n = idx.len();
        let mut clipped = false;
        for i in 0..n {
            let (ia, ib, ic) = (idx[(i + n - 1) % n], idx[i], idx[(i + 1) % n]);
            let (a, b, c) = (pts[ia], pts[ib], pts[ic]);
            let cross = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
            if (cross > 0.0) != ccw || cross.abs() < 1e-7 {
                continue;
            }
            let inside = idx
                .iter()
                .any(|&j| j != ia && j != ib && j != ic && point_in_tri(pts[j], a, b, c));
            if inside {
                continue;
            }
            mesh.add_triangle(ia as u32, ib as u32, ic as u32);
            idx.remove(i);
            clipped = true;
            break;
        }
        if !clipped {
            break;
        }
    }
    if idx.len() == 3 {
        mesh.add_triangle(idx[0] as u32, idx[1] as u32, idx[2] as u32);
    } else if idx.len() > 3 {
        // entartetes Polygon: Fächer als Rückfall
        for k in 1..idx.len() - 1 {
            mesh.add_triangle(idx[0] as u32, idx[k] as u32, idx[k + 1] as u32);
        }
    }
    painter.add(Shape::mesh(mesh));
}

fn point_in_tri(p: Pos2, a: Pos2, b: Pos2, c: Pos2) -> bool {
    let d1 = (p.x - b.x) * (a.y - b.y) - (a.x - b.x) * (p.y - b.y);
    let d2 = (p.x - c.x) * (b.y - c.y) - (b.x - c.x) * (p.y - c.y);
    let d3 = (p.x - a.x) * (c.y - a.y) - (c.x - a.x) * (p.y - a.y);
    let neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(neg && pos)
}

// ---------------------------------------------------------------------- Bausteine

const ARROW: [Pt; 7] = [
    (4.0, 1.5),
    (4.0, 13.5),
    (6.8, 10.8),
    (8.8, 14.8),
    (10.6, 14.0),
    (8.6, 10.0),
    (12.3, 10.0),
];

fn pencil_shape(p: &Pen, fill: Color32, line: Color32, tip: Color32) {
    // Stift diagonal von unten links (Spitze) nach oben rechts
    p.poly(
        Some(fill),
        Some(line),
        &[
            (2.5, 13.5),
            (3.5, 10.0),
            (11.5, 2.0),
            (14.0, 4.5),
            (6.0, 12.5),
        ],
    );
    p.poly(Some(tip), None, &[(2.5, 13.5), (3.2, 11.0), (5.0, 12.8)]);
    p.line(line, &[(3.5, 10.0), (6.0, 12.5)]);
    p.line(line, &[(10.0, 3.5), (12.5, 6.0)]);
}

fn brush_shape(p: &Pen, line: Color32, tip: Color32) {
    // Stiel
    p.line_w(line, 1.6, &[(14.0, 2.0), (8.5, 7.5)]);
    // Zwinge
    p.poly(
        Some(p.st.light),
        Some(line),
        &[(7.0, 7.5), (8.5, 6.0), (10.0, 7.5), (8.5, 9.0)],
    );
    // Spitze
    p.poly(
        Some(tip),
        Some(line),
        &[(7.0, 7.5), (8.5, 9.0), (6.5, 12.0), (2.0, 14.0), (4.0, 9.5)],
    );
}

fn chain(p: &Pen, x: f32, y: f32, col: Color32) {
    p.ellipse((x, y), 2.2, 1.4, None, Some(col));
    p.ellipse((x + 2.8, y + 1.8), 2.2, 1.4, None, Some(col));
}

fn knife_blade(p: &Pen, x: f32, y: f32, col: Color32) {
    p.poly(
        Some(p.st.paper),
        Some(col),
        &[
            (x, y),
            (x + 5.0, y - 5.0),
            (x + 6.0, y - 4.0),
            (x + 1.5, y + 0.5),
        ],
    );
}

fn page(p: &Pen, x: f32, y: f32, w: f32, h: f32) {
    let f = 3.0f32.min(w / 2.0);
    p.poly(
        Some(p.st.paper),
        Some(p.st.fg),
        &[
            (x, y),
            (x + w - f, y),
            (x + w, y + f),
            (x + w, y + h),
            (x, y + h),
        ],
    );
    p.line(
        p.st.fg,
        &[(x + w - f, y), (x + w - f, y + f), (x + w, y + f)],
    );
}

fn stack(p: &Pen, front: usize) {
    // drei gestaffelte Quadrate; das Quadrat `front` (0 = hinten) hervorgehoben
    let boxes = [(2.5, 2.5), (5.0, 5.0), (7.5, 7.5)];
    for (i, &(x, y)) in boxes.iter().enumerate() {
        let fill = if i == front { p.st.fg } else { p.st.paper };
        p.rect(x, y, 6.0, 6.0, Some(fill), Some(p.st.fg));
    }
}

fn curved_arrow(p: &Pen, cw: bool) {
    let col = p.st.fg;
    let thick = |a0: f32, a1: f32| {
        let pts: Vec<Pt> = (0..=16)
            .map(|i| {
                let a = (a0 + (a1 - a0) * i as f32 / 16.0).to_radians();
                (8.0 + 5.0 * a.cos(), 10.0 + 5.0 * a.sin())
            })
            .collect();
        p.line_w(col, 2.0, &pts);
    };
    if cw {
        thick(200.0, 330.0);
        p.line_w(col, 1.8, &[(3.5, 10.5), (3.3, 9.0)]);
        p.arrow_head(col, (13.5, 8.8), (1.0, 1.4), 3.4);
    } else {
        thick(210.0, 340.0);
        p.line_w(col, 1.8, &[(12.6, 10.5), (12.8, 9.0)]);
        p.arrow_head(col, (2.5, 8.8), (-1.0, 1.4), 3.4);
    }
}

fn hand_shape(p: &Pen, fill: Color32, line: Color32) {
    // Finger als einzelne Streifen, darunter der Handballen (konvexe Teile, sauber gefüllt)
    let fingers = [
        (3.6, 4.0, 9.5),
        (5.9, 2.2, 9.5),
        (8.2, 1.6, 9.5),
        (10.5, 3.0, 9.5),
    ];
    for (x, top, bottom) in fingers {
        p.rect(x, top, 2.3, bottom - top, Some(fill), Some(line));
    }
    // Daumen
    p.poly(
        Some(fill),
        Some(line),
        &[(1.5, 8.2), (2.8, 7.4), (4.6, 9.4), (4.0, 11.2)],
    );
    // Handballen
    p.poly(
        Some(fill),
        Some(line),
        &[
            (3.6, 9.0),
            (12.8, 9.0),
            (12.8, 11.5),
            (11.2, 14.5),
            (5.8, 14.5),
            (3.8, 12.0),
        ],
    );
    p.rect(3.9, 8.6, 8.7, 1.2, Some(fill), None);
}

fn bucket_shape(p: &Pen, fill: Color32, line: Color32, paint: Color32) {
    p.poly(
        Some(fill),
        Some(line),
        &[(2.0, 7.0), (7.0, 2.0), (12.5, 7.5), (7.5, 12.5)],
    );
    p.line(line, &[(4.5, 4.5), (6.5, 2.0), (8.0, 3.0)]);
    p.poly(
        Some(paint),
        None,
        &[
            (12.5, 7.5),
            (14.5, 10.5),
            (14.5, 12.5),
            (13.0, 13.0),
            (12.0, 11.0),
        ],
    );
}

fn dropper_shape(p: &Pen, fill: Color32, line: Color32) {
    p.poly(
        Some(fill),
        Some(line),
        &[
            (2.0, 14.0),
            (2.0, 12.5),
            (8.5, 6.0),
            (10.0, 7.5),
            (3.5, 14.0),
        ],
    );
    p.poly(
        Some(line),
        None,
        &[
            (8.0, 5.0),
            (11.0, 8.0),
            (12.5, 6.5),
            (14.0, 5.5),
            (14.5, 3.0),
            (13.0, 1.5),
            (10.5, 2.0),
            (9.5, 3.5),
        ],
    );
}

// ---------------------------------------------------------------------- Zeichnungen

fn draw(p: &Pen, icon: Icon) {
    let st = p.st;
    let (fg, light, paper, acc, acc2) = (st.fg, st.light, st.paper, st.accent, st.accent2);
    match icon {
        // ---------------------------------------------------- Auswählen
        Icon::Pointer => p.poly(Some(fg), None, &ARROW),
        Icon::SelectBehind => {
            p.rect(9.0, 2.0, 5.0, 5.0, Some(light), Some(light));
            p.poly(Some(paper), Some(fg), &ARROW);
        }
        Icon::Subselection => p.poly(Some(paper), Some(fg), &ARROW),
        // FB-80: the Subselection arrow with a mask badge (dark square with a light disc, the
        // look of a mask thumbnail) at the upper right, clear of the arrow
        Icon::MaskElementSelect => {
            p.poly(Some(paper), Some(fg), &ARROW);
            p.rect(9.0, 1.0, 6.0, 6.0, Some(fg), Some(fg));
            p.circle((12.0, 4.0), 1.8, Some(paper), None);
        }
        Icon::Scale => {
            p.rect(1.5, 1.5, 9.0, 9.0, Some(paper), Some(fg));
            p.line(fg, &[(4.0, 4.0), (12.5, 12.5)]);
            p.arrow_head(fg, (13.5, 13.5), (1.0, 1.0), 3.0);
            p.rect(1.0, 1.0, 2.0, 2.0, Some(fg), None);
            p.rect(9.5, 9.5, 2.0, 2.0, Some(fg), None);
        }
        Icon::Skew => {
            p.poly(
                Some(paper),
                Some(fg),
                &[(4.5, 3.5), (14.5, 3.5), (11.5, 12.5), (1.5, 12.5)],
            );
            for q in [(4.5, 3.5), (14.5, 3.5), (11.5, 12.5), (1.5, 12.5)] {
                p.rect(q.0 - 1.0, q.1 - 1.0, 2.0, 2.0, Some(acc), None);
            }
        }
        Icon::Distort => {
            p.poly(
                Some(paper),
                Some(fg),
                &[(2.5, 4.5), (13.5, 2.5), (11.5, 13.5), (4.5, 11.5)],
            );
            for q in [(2.5, 4.5), (13.5, 2.5), (11.5, 13.5), (4.5, 11.5)] {
                p.rect(q.0 - 1.0, q.1 - 1.0, 2.0, 2.0, Some(acc), None);
            }
        }
        Icon::NineSlice => {
            p.rect(1.5, 1.5, 13.0, 13.0, Some(paper), Some(fg));
            p.dashed(acc, (5.5, 1.5), (5.5, 14.5), 1.5);
            p.dashed(acc, (10.5, 1.5), (10.5, 14.5), 1.5);
            p.dashed(acc, (1.5, 5.5), (14.5, 5.5), 1.5);
            p.dashed(acc, (1.5, 10.5), (14.5, 10.5), 1.5);
        }
        Icon::Crop => {
            p.line_w(fg, 1.6, &[(4.5, 1.0), (4.5, 11.5), (15.0, 11.5)]);
            p.line_w(fg, 1.6, &[(1.0, 4.5), (11.5, 4.5), (11.5, 15.0)]);
            p.line(light, &[(4.5, 11.5), (11.5, 4.5)]);
        }
        Icon::ExportArea => {
            p.dashed_rect(fg, 1.5, 3.5, 10.0, 10.0);
            p.line_w(acc, 1.5, &[(7.0, 8.0), (13.0, 2.0)]);
            p.arrow_head(acc, (14.5, 1.5), (1.0, -1.0), 3.5);
        }
        // ---------------------------------------------------- Bitmap
        Icon::Marquee => p.dashed_rect(fg, 2.5, 3.5, 11.0, 9.0),
        Icon::OvalMarquee => {
            let n = 24;
            for i in (0..n).step_by(2) {
                let a0 = i as f32 / n as f32 * 360.0;
                let a1 = (i + 1) as f32 / n as f32 * 360.0;
                let pts: Vec<Pt> = (0..=4)
                    .map(|k| {
                        let a = (a0 + (a1 - a0) * k as f32 / 4.0).to_radians();
                        (8.0 + 6.0 * a.cos(), 8.0 + 4.8 * a.sin())
                    })
                    .collect();
                p.line(fg, &pts);
            }
        }
        Icon::Lasso => {
            p.ellipse((9.0, 6.0), 5.5, 3.8, None, Some(fg));
            p.line(fg, &[(5.0, 8.8), (4.0, 11.0), (5.5, 12.5), (3.5, 15.0)]);
            p.circle((5.0, 11.0), 0.9, Some(acc), None);
        }
        Icon::PolygonLasso => {
            p.outline(
                fg,
                &[
                    (4.0, 6.0),
                    (8.0, 2.0),
                    (14.0, 3.5),
                    (13.0, 8.5),
                    (7.0, 10.0),
                ],
            );
            p.line(fg, &[(6.0, 9.5), (4.5, 12.0), (5.5, 13.0), (3.5, 15.0)]);
        }
        Icon::MagicWand => {
            p.line_w(fg, 1.8, &[(2.0, 14.0), (9.0, 7.0)]);
            p.line_w(paper, 1.0, &[(7.5, 8.5), (9.0, 7.0)]);
            let c = (11.5, 4.5);
            for (dx, dy) in [
                (0.0, -3.5),
                (0.0, 3.5),
                (-3.5, 0.0),
                (3.5, 0.0),
                (2.4, 2.4),
                (-2.4, -2.4),
                (2.4, -2.4),
                (-2.4, 2.4),
            ] {
                let f = if dx == 0.0 || dy == 0.0 { 1.0 } else { 0.6 };
                p.line(acc2, &[c, (c.0 + dx * f, c.1 + dy * f)]);
            }
        }
        Icon::Brush => brush_shape(p, fg, acc),
        Icon::Pencil => pencil_shape(p, paper, fg, fg),
        Icon::Eraser => {
            p.poly(
                Some(paper),
                Some(fg),
                &[(1.5, 10.0), (8.5, 3.0), (14.5, 9.0), (7.5, 16.0)],
            );
            p.poly(
                Some(light),
                Some(fg),
                &[(1.5, 10.0), (4.5, 7.0), (10.5, 13.0), (7.5, 16.0)],
            );
        }
        Icon::Blur => {
            p.poly(
                Some(light),
                Some(fg),
                &[
                    (8.0, 1.5),
                    (11.5, 7.5),
                    (12.5, 10.5),
                    (11.0, 13.5),
                    (8.0, 14.5),
                    (5.0, 13.5),
                    (3.5, 10.5),
                    (4.5, 7.5),
                ],
            );
            p.line(paper, &[(6.0, 10.0), (6.5, 12.0)]);
        }
        Icon::Sharpen => {
            p.poly(
                Some(light),
                Some(fg),
                &[(8.0, 1.5), (13.0, 14.5), (3.0, 14.5)],
            );
            p.line(paper, &[(7.5, 5.0), (5.5, 12.0)]);
        }
        Icon::Dodge => {
            p.line_w(fg, 1.6, &[(8.0, 9.0), (8.0, 15.0)]);
            p.circle((8.0, 5.5), 4.0, Some(paper), Some(fg));
        }
        Icon::Burn => {
            p.poly(
                Some(light),
                Some(fg),
                &[
                    (3.0, 15.0),
                    (3.0, 9.0),
                    (6.0, 6.0),
                    (6.5, 2.0),
                    (9.5, 2.0),
                    (10.0, 6.0),
                    (13.0, 9.0),
                    (13.0, 15.0),
                ],
            );
            p.circle((8.0, 10.5), 1.5, Some(fg), None);
        }
        Icon::Smudge => {
            p.poly(
                Some(paper),
                Some(fg),
                &[
                    (6.0, 1.5),
                    (9.0, 1.5),
                    (9.5, 8.0),
                    (12.5, 9.5),
                    (12.5, 14.5),
                    (5.0, 14.5),
                    (4.0, 10.0),
                    (5.5, 8.5),
                ],
            );
            p.rect(6.5, 2.5, 2.0, 2.5, Some(light), None);
        }
        Icon::RubberStamp => {
            p.circle((8.0, 3.5), 2.5, Some(fg), None);
            p.rect(6.5, 5.0, 3.0, 4.0, Some(fg), None);
            p.rect(2.5, 9.0, 11.0, 3.0, Some(paper), Some(fg));
            p.line_w(acc, 1.5, &[(3.0, 14.0), (13.0, 14.0)]);
        }
        Icon::ReplaceColor => {
            brush_shape(p, fg, st.red);
            p.rect(10.0, 10.0, 5.0, 5.0, Some(st.blue), Some(fg));
        }
        Icon::RedEye => {
            p.poly(
                Some(paper),
                Some(fg),
                &[
                    (1.0, 8.0),
                    (4.0, 4.5),
                    (8.0, 3.5),
                    (12.0, 4.5),
                    (15.0, 8.0),
                    (12.0, 11.5),
                    (8.0, 12.5),
                    (4.0, 11.5),
                ],
            );
            p.circle((8.0, 8.0), 3.0, Some(st.red), None);
            p.circle((8.0, 8.0), 1.2, Some(fg), None);
        }
        // ---------------------------------------------------- Vektor
        Icon::Line => p.line_w(fg, 1.4, &[(2.5, 2.5), (13.5, 13.5)]),
        Icon::Pen => {
            p.poly(
                Some(paper),
                Some(fg),
                &[
                    (8.0, 1.5),
                    (12.5, 8.0),
                    (10.5, 12.0),
                    (5.5, 12.0),
                    (3.5, 8.0),
                ],
            );
            p.line(fg, &[(8.0, 1.5), (8.0, 7.5)]);
            p.circle((8.0, 8.0), 1.0, Some(fg), None);
            p.rect(5.0, 12.5, 6.0, 2.5, Some(fg), None);
        }
        Icon::VectorPath => {
            brush_shape(p, fg, acc);
            p.line(acc, &[(9.0, 14.5), (11.0, 11.0), (14.5, 12.5)]);
        }
        Icon::RedrawPath => {
            pencil_shape(p, paper, fg, fg);
            p.arc(acc, (11.0, 12.5), 3.0, 180.0, 360.0);
        }
        Icon::Rectangle => {
            p.rect(1.5, 3.5, 13.0, 10.0, Some(paper), Some(fg));
            p.rect(2.0, 9.0, 12.0, 4.0, Some(mix(paper, light, 0.6)), None);
        }
        Icon::Ellipse => p.ellipse((8.0, 8.0), 6.5, 5.5, Some(paper), Some(fg)),
        Icon::Polygon => {
            let pts: Vec<Pt> = (0..6)
                .map(|i| {
                    let a = (i as f32 * 60.0 - 90.0).to_radians();
                    (8.0 + 6.5 * a.cos(), 8.5 + 6.5 * a.sin())
                })
                .collect();
            p.poly(Some(paper), Some(fg), &pts);
        }
        Icon::AutoShape | Icon::Star => {
            let pts: Vec<Pt> = (0..10)
                .map(|i| {
                    let r = if i % 2 == 0 { 7.0 } else { 3.0 };
                    let a = (i as f32 * 36.0 - 90.0).to_radians();
                    (8.0 + r * a.cos(), 8.5 + r * a.sin())
                })
                .collect();
            p.poly(Some(paper), Some(fg), &pts);
        }
        Icon::Text => {
            p.poly(
                Some(fg),
                None,
                &[
                    (2.0, 2.0),
                    (14.0, 2.0),
                    (14.0, 5.5),
                    (13.0, 5.5),
                    (12.0, 3.5),
                    (9.5, 3.5),
                    (9.5, 12.5),
                    (11.0, 13.5),
                    (11.0, 14.5),
                    (5.0, 14.5),
                    (5.0, 13.5),
                    (6.5, 12.5),
                    (6.5, 3.5),
                    (4.0, 3.5),
                    (3.0, 5.5),
                    (2.0, 5.5),
                ],
            );
        }
        Icon::Freeform => {
            p.line(
                fg,
                &[
                    (1.5, 13.0),
                    (4.0, 10.0),
                    (7.0, 8.5),
                    (10.0, 9.5),
                    (14.5, 7.0),
                ],
            );
            p.circle((7.0, 5.0), 2.6, Some(paper), Some(fg));
            p.line(acc, &[(7.0, 7.6), (7.0, 8.5)]);
        }
        Icon::Reshape => {
            p.circle((8.0, 8.0), 5.5, None, Some(light));
            p.line(
                fg,
                &[
                    (1.5, 13.0),
                    (5.0, 11.0),
                    (8.0, 6.0),
                    (11.0, 11.0),
                    (14.5, 13.0),
                ],
            );
            p.circle((8.0, 6.0), 1.0, Some(acc), None);
        }
        Icon::PathScrubAdd | Icon::PathScrubSubtract => {
            brush_shape(p, fg, acc);
            p.line_w(st.green, 1.4, &[(10.0, 12.5), (15.0, 12.5)]);
            if icon == Icon::PathScrubAdd {
                p.line_w(st.green, 1.4, &[(12.5, 10.0), (12.5, 15.0)]);
            }
        }
        Icon::Knife => {
            p.poly(
                Some(paper),
                Some(fg),
                &[
                    (6.0, 1.5),
                    (10.0, 1.5),
                    (10.0, 11.0),
                    (8.0, 15.0),
                    (6.0, 11.0),
                ],
            );
            for y in [4.0, 6.0, 8.0] {
                p.line(fg, &[(6.0, y), (7.5, y)]);
            }
        }
        // ---------------------------------------------------- Web
        Icon::HotspotRect => {
            p.rect(1.5, 3.5, 10.0, 9.0, Some(mix(paper, light, 0.3)), Some(fg));
            chain(p, 10.0, 9.5, st.blue);
        }
        Icon::HotspotCircle => {
            p.circle((6.5, 7.5), 5.0, Some(mix(paper, light, 0.3)), Some(fg));
            chain(p, 10.0, 9.5, st.blue);
        }
        Icon::HotspotPolygon => {
            p.poly(
                Some(mix(paper, light, 0.3)),
                Some(fg),
                &[
                    (1.5, 6.0),
                    (6.0, 2.0),
                    (11.5, 4.0),
                    (10.0, 12.5),
                    (3.0, 11.5),
                ],
            );
            chain(p, 10.0, 9.5, st.blue);
        }
        Icon::Slice => {
            p.rect(
                1.5,
                4.5,
                10.0,
                10.0,
                Some(mix(st.green, paper, 0.6)),
                Some(fg),
            );
            knife_blade(p, 8.0, 7.0, fg);
        }
        Icon::PolygonSlice => {
            p.poly(
                Some(mix(st.green, paper, 0.6)),
                Some(fg),
                &[
                    (1.5, 8.0),
                    (5.0, 4.5),
                    (11.5, 6.0),
                    (10.0, 14.5),
                    (3.0, 13.5),
                ],
            );
            knife_blade(p, 8.0, 7.0, fg);
        }
        Icon::HideSlices => {
            p.rect(1.5, 2.5, 13.0, 11.0, Some(paper), Some(st.blue));
            p.dashed_rect(st.blue, 4.0, 5.0, 8.0, 6.0);
        }
        Icon::ShowSlices => {
            p.rect(1.5, 2.5, 13.0, 11.0, Some(paper), Some(st.blue));
            p.rect(
                3.5,
                4.5,
                9.0,
                7.0,
                Some(mix(st.green, paper, 0.5)),
                Some(st.red),
            );
            p.line(st.red, &[(8.0, 4.5), (8.0, 11.5)]);
        }
        // ---------------------------------------------------- Farben
        Icon::Eyedropper => dropper_shape(p, paper, fg),
        Icon::PaintBucket => bucket_shape(p, paper, fg, acc),
        Icon::Gradient => {
            p.rect(1.5, 3.5, 13.0, 10.0, None, Some(fg));
            for i in 0..12 {
                let t = i as f32 / 11.0;
                p.rect(2.0 + i as f32, 4.0, 1.0, 9.0, Some(mix(paper, fg, t)), None);
            }
        }
        Icon::StrokeColor => pencil_shape(p, paper, fg, fg),
        Icon::FillColor => bucket_shape(p, paper, fg, fg),
        Icon::DefaultColors => {
            p.rect(1.5, 1.5, 8.0, 8.0, Some(fg), Some(fg));
            p.rect(6.5, 6.5, 8.0, 8.0, Some(paper), Some(fg));
        }
        Icon::NoColor => {
            p.rect(1.5, 1.5, 13.0, 13.0, Some(paper), Some(fg));
            p.line_w(st.red, 1.6, &[(2.5, 13.5), (13.5, 2.5)]);
        }
        Icon::SwapColors => {
            p.rect(1.5, 1.5, 6.0, 6.0, Some(paper), Some(fg));
            p.arc(fg, (6.0, 10.0), 6.0, 270.0, 360.0);
            p.arrow_head(fg, (12.0, 14.5), (0.0, 1.0), 3.0);
            p.line(fg, &[(9.0, 5.0), (12.0, 5.0)]);
            p.arrow_head(fg, (8.0, 5.0), (-1.0, 0.0), 2.5);
        }
        // ---------------------------------------------------- Ansicht
        Icon::Hand => hand_shape(p, paper, fg),
        Icon::Zoom => {
            p.circle((6.5, 6.5), 4.8, Some(paper), Some(fg));
            p.line_w(fg, 2.4, &[(10.0, 10.0), (14.5, 14.5)]);
        }
        Icon::ScreenStandard => {
            p.rect(1.5, 2.5, 13.0, 11.0, Some(paper), Some(fg));
            p.rect(1.5, 2.5, 13.0, 2.5, Some(light), Some(fg));
            p.rect(4.0, 7.0, 6.0, 4.0, None, Some(fg));
        }
        Icon::ScreenFullMenus => {
            p.rect(1.5, 2.5, 13.0, 11.0, Some(paper), Some(fg));
            p.rect(1.5, 2.5, 13.0, 2.5, Some(light), Some(fg));
        }
        Icon::ScreenFull => {
            p.rect(1.5, 2.5, 13.0, 11.0, Some(light), Some(fg));
            p.line(paper, &[(2.5, 3.5), (13.5, 3.5)]);
        }
        // ---------------------------------------------------- Symbolleiste
        Icon::New => page(p, 3.0, 1.5, 10.0, 13.0),
        Icon::Save => {
            p.rect(1.5, 1.5, 13.0, 13.0, Some(fg), Some(fg));
            p.rect(4.0, 1.5, 8.0, 5.0, Some(paper), None);
            p.rect(9.0, 2.5, 2.0, 3.0, Some(fg), None);
            p.rect(3.5, 9.0, 9.0, 5.5, Some(light), None);
        }
        Icon::Open => {
            p.poly(
                Some(light),
                Some(fg),
                &[
                    (1.5, 3.5),
                    (6.0, 3.5),
                    (7.5, 5.0),
                    (12.5, 5.0),
                    (12.5, 13.5),
                    (1.5, 13.5),
                ],
            );
            p.poly(
                Some(paper),
                Some(fg),
                &[(1.5, 13.5), (4.0, 7.5), (15.0, 7.5), (12.5, 13.5)],
            );
        }
        Icon::Import => {
            page(p, 6.0, 1.5, 8.5, 13.0);
            p.line_w(fg, 2.0, &[(0.5, 8.0), (7.5, 8.0)]);
            p.arrow_head(fg, (11.5, 8.0), (1.0, 0.0), 4.5);
        }
        Icon::Export => {
            page(p, 1.5, 1.5, 8.5, 13.0);
            p.line_w(fg, 2.0, &[(5.0, 8.0), (11.0, 8.0)]);
            p.arrow_head(fg, (15.5, 8.0), (1.0, 0.0), 4.5);
        }
        Icon::Print => {
            p.rect(4.0, 1.5, 8.0, 5.0, Some(paper), Some(fg));
            p.rect(1.5, 6.5, 13.0, 6.0, Some(light), Some(fg));
            p.rect(4.0, 10.5, 8.0, 4.0, Some(paper), Some(fg));
            p.line(fg, &[(5.5, 12.5), (10.5, 12.5)]);
        }
        Icon::Undo => curved_arrow(p, false),
        Icon::Redo => curved_arrow(p, true),
        Icon::Cut => {
            p.line_w(fg, 1.3, &[(5.0, 1.5), (10.0, 10.5)]);
            p.line_w(fg, 1.3, &[(11.0, 1.5), (6.0, 10.5)]);
            p.circle((5.0, 12.5), 2.0, None, Some(fg));
            p.circle((11.0, 12.5), 2.0, None, Some(fg));
        }
        Icon::Copy => {
            page(p, 1.5, 1.5, 8.0, 10.0);
            page(p, 6.5, 4.5, 8.0, 10.0);
        }
        Icon::Paste => {
            p.rect(2.5, 2.5, 11.0, 12.0, Some(light), Some(fg));
            p.rect(5.5, 1.5, 5.0, 2.5, Some(fg), None);
            p.rect(4.5, 5.5, 7.0, 7.5, Some(paper), Some(fg));
        }
        Icon::Group => {
            p.dashed_rect(fg, 1.5, 2.5, 13.0, 11.0);
            p.rect(3.5, 5.5, 4.0, 4.0, Some(fg), None);
            p.rect(9.0, 7.0, 3.5, 3.5, Some(light), Some(fg));
        }
        Icon::Ungroup => {
            p.rect(2.5, 4.5, 4.0, 4.0, Some(fg), None);
            p.rect(9.5, 6.5, 4.0, 4.0, Some(light), Some(fg));
        }
        Icon::Join => {
            p.circle((6.0, 8.0), 4.5, Some(fg), None);
            p.circle((10.5, 8.0), 4.5, Some(fg), None);
            p.circle((10.5, 8.0), 2.0, Some(paper), None);
        }
        Icon::Split => {
            p.circle((5.5, 7.0), 4.0, Some(light), Some(fg));
            p.circle((11.0, 10.0), 3.5, None, Some(fg));
        }
        Icon::BringToFront => stack(p, 2),
        Icon::BringForward => stack(p, 1),
        Icon::SendBackward => stack(p, 1),
        Icon::SendToBack => stack(p, 0),
        Icon::EditSymbol => {
            p.rect(1.5, 5.5, 9.0, 9.0, Some(paper), Some(fg));
            p.arc(fg, (10.0, 6.0), 4.0, 180.0, 300.0);
            p.arrow_head(fg, (13.0, 2.5), (1.0, -0.4), 3.0);
        }
        Icon::AlignLeft => {
            p.line_w(fg, 1.2, &[(1.5, 1.0), (1.5, 15.0)]);
            p.rect(2.5, 3.0, 11.0, 4.0, Some(paper), Some(fg));
            p.rect(2.5, 9.0, 7.0, 4.0, Some(paper), Some(fg));
        }
        Icon::RotateCcw => {
            p.rect(1.5, 6.5, 8.0, 8.0, Some(paper), Some(fg));
            p.arc(fg, (8.0, 7.0), 5.0, 280.0, 360.0);
            p.arrow_head(fg, (5.5, 2.0), (-1.0, 0.0), 3.0);
        }
        Icon::RotateCw => {
            p.rect(6.5, 6.5, 8.0, 8.0, Some(paper), Some(fg));
            p.arc(fg, (8.0, 7.0), 5.0, 180.0, 260.0);
            p.arrow_head(fg, (10.5, 2.0), (1.0, 0.0), 3.0);
        }
        Icon::FlipHorizontal => {
            p.poly(Some(fg), None, &[(7.0, 2.0), (7.0, 14.0), (1.5, 14.0)]);
            p.poly(
                Some(paper),
                Some(fg),
                &[(9.0, 2.0), (9.0, 14.0), (14.5, 14.0)],
            );
            p.dashed(light, (8.0, 0.5), (8.0, 15.5), 1.5);
        }
        Icon::FlipVertical => {
            p.poly(Some(fg), None, &[(2.0, 7.0), (14.0, 7.0), (14.0, 1.5)]);
            p.poly(
                Some(paper),
                Some(fg),
                &[(2.0, 9.0), (14.0, 9.0), (14.0, 14.5)],
            );
            p.dashed(light, (0.5, 8.0), (15.5, 8.0), 1.5);
        }
        // ---------------------------------------------------- Bedienfelder
        Icon::PanelMenu => {
            p.poly(Some(fg), None, &[(1.0, 6.0), (6.0, 6.0), (3.5, 9.0)]);
            for y in [4.5, 7.5, 10.5] {
                p.line(fg, &[(7.0, y), (15.0, y)]);
            }
        }
        Icon::CollapseRight => {
            p.poly(Some(fg), None, &[(3.0, 4.0), (7.0, 8.0), (3.0, 12.0)]);
            p.poly(Some(fg), None, &[(9.0, 4.0), (13.0, 8.0), (9.0, 12.0)]);
        }
        Icon::CollapseLeft => {
            p.poly(Some(fg), None, &[(7.0, 4.0), (3.0, 8.0), (7.0, 12.0)]);
            p.poly(Some(fg), None, &[(13.0, 4.0), (9.0, 8.0), (13.0, 12.0)]);
        }
        Icon::NewPage => {
            page(p, 3.0, 1.5, 10.0, 13.0);
            p.line(acc, &[(10.0, 9.0), (10.0, 15.0)]);
            p.line(acc, &[(7.0, 12.0), (13.0, 12.0)]);
        }
        Icon::Trash => {
            p.rect(3.5, 4.5, 9.0, 10.0, Some(paper), Some(fg));
            p.line(fg, &[(2.0, 3.5), (14.0, 3.5)]);
            p.rect(6.0, 1.5, 4.0, 2.0, None, Some(fg));
            for x in [6.0, 8.0, 10.0] {
                p.line(light, &[(x, 6.5), (x, 13.0)]);
            }
        }
        Icon::NewLayer | Icon::Folder => {
            p.poly(
                Some(mix(
                    paper,
                    st.yellow,
                    if icon == Icon::Folder { 0.35 } else { 0.0 },
                )),
                Some(fg),
                &[
                    (1.5, 3.5),
                    (6.0, 3.5),
                    (7.5, 5.0),
                    (14.5, 5.0),
                    (14.5, 13.5),
                    (1.5, 13.5),
                ],
            );
            if icon == Icon::NewLayer {
                p.line(acc, &[(11.0, 7.0), (11.0, 13.0)]);
                p.line(acc, &[(8.0, 10.0), (14.0, 10.0)]);
            }
        }
        Icon::NewSublayer => {
            p.poly(
                Some(paper),
                Some(fg),
                &[
                    (4.5, 5.5),
                    (8.0, 5.5),
                    (9.5, 7.0),
                    (14.5, 7.0),
                    (14.5, 14.5),
                    (4.5, 14.5),
                ],
            );
            p.line(fg, &[(1.5, 1.5), (1.5, 10.0), (4.0, 10.0)]);
        }
        Icon::NewBitmap => {
            p.rect(1.5, 2.5, 13.0, 11.0, Some(paper), Some(fg));
            p.poly(
                Some(st.green),
                None,
                &[
                    (2.0, 13.0),
                    (6.0, 8.0),
                    (9.0, 11.0),
                    (11.0, 9.0),
                    (14.0, 13.0),
                ],
            );
            p.circle((11.0, 5.5), 1.5, Some(st.yellow), None);
        }
        Icon::AddMask => {
            p.rect(1.5, 2.5, 13.0, 11.0, Some(paper), Some(fg));
            p.circle((8.0, 8.0), 3.5, Some(fg), None);
        }
        Icon::Duplicate => {
            p.rect(1.5, 1.5, 9.0, 9.0, Some(paper), Some(fg));
            p.rect(5.5, 5.5, 9.0, 9.0, Some(paper), Some(fg));
        }
        Icon::Eye => {
            p.poly(
                Some(paper),
                Some(fg),
                &[
                    (1.0, 8.0),
                    (4.0, 4.5),
                    (8.0, 3.5),
                    (12.0, 4.5),
                    (15.0, 8.0),
                    (12.0, 11.5),
                    (8.0, 12.5),
                    (4.0, 11.5),
                ],
            );
            p.circle((8.0, 8.0), 2.5, Some(fg), None);
        }
        Icon::Lock => {
            p.arc(fg, (8.0, 6.5), 3.0, 180.0, 360.0);
            p.line(fg, &[(5.0, 6.5), (5.0, 8.0)]);
            p.line(fg, &[(11.0, 6.5), (11.0, 8.0)]);
            p.rect(3.5, 7.5, 9.0, 7.0, Some(acc2), Some(fg));
        }
        Icon::TriangleRight => p.poly(Some(fg), None, &[(5.0, 3.0), (11.0, 8.0), (5.0, 13.0)]),
        Icon::TriangleDown => p.poly(Some(fg), None, &[(3.0, 5.0), (13.0, 5.0), (8.0, 11.0)]),
        Icon::FirstFrame => {
            p.rect(3.0, 3.0, 2.0, 10.0, Some(fg), None);
            p.poly(Some(fg), None, &[(13.0, 3.0), (13.0, 13.0), (6.0, 8.0)]);
        }
        Icon::PrevFrame => {
            p.poly(Some(fg), None, &[(9.0, 3.0), (9.0, 13.0), (3.0, 8.0)]);
            p.rect(11.0, 3.0, 2.0, 10.0, Some(fg), None);
        }
        Icon::Play => p.poly(
            Some(paper),
            Some(fg),
            &[(4.0, 2.5), (13.0, 8.0), (4.0, 13.5)],
        ),
        Icon::Stop => p.rect(4.0, 4.0, 8.0, 8.0, Some(fg), None),
        Icon::NextFrame => {
            p.rect(3.0, 3.0, 2.0, 10.0, Some(fg), None);
            p.poly(Some(fg), None, &[(7.0, 3.0), (13.0, 8.0), (7.0, 13.0)]);
        }
        Icon::LastFrame => {
            p.poly(Some(fg), None, &[(3.0, 3.0), (10.0, 8.0), (3.0, 13.0)]);
            p.rect(11.0, 3.0, 2.0, 10.0, Some(fg), None);
        }
        Icon::ExitBitmapMode => {
            p.circle((8.0, 8.0), 6.0, Some(fg), None);
            p.line_w(paper, 1.6, &[(5.5, 5.5), (10.5, 10.5)]);
            p.line_w(paper, 1.6, &[(10.5, 5.5), (5.5, 10.5)]);
        }
        Icon::ViewOriginal => {
            pencil_shape(p, st.yellow, fg, fg);
            p.poly(
                Some(acc2),
                None,
                &[(11.5, 2.0), (14.0, 4.5), (12.8, 5.7), (10.3, 3.2)],
            );
        }
        Icon::ViewPreview => {
            p.rect(
                1.5,
                2.5,
                13.0,
                11.0,
                Some(mix(st.blue, paper, 0.55)),
                Some(fg),
            );
            p.poly(
                Some(st.green),
                None,
                &[
                    (2.0, 13.0),
                    (6.0, 7.0),
                    (9.0, 10.5),
                    (11.0, 8.5),
                    (14.0, 13.0),
                ],
            );
        }
        Icon::View2Up => {
            p.rect(1.5, 2.5, 13.0, 11.0, Some(paper), Some(fg));
            p.line(fg, &[(8.0, 2.5), (8.0, 13.5)]);
        }
        Icon::View4Up => {
            p.rect(1.5, 2.5, 13.0, 11.0, Some(paper), Some(fg));
            p.line(fg, &[(8.0, 2.5), (8.0, 13.5)]);
            p.line(fg, &[(1.5, 8.0), (14.5, 8.0)]);
        }
        Icon::PageSmall | Icon::Document => page(p, 3.5, 1.5, 9.0, 13.0),
        Icon::PageNavigate => {
            p.line_w(fg, 1.3, &[(4.5, 2.0), (4.5, 12.0)]);
            p.arrow_head(fg, (4.5, 15.0), (0.0, 1.0), 3.5);
            p.line_w(fg, 1.3, &[(11.0, 14.0), (11.0, 4.0)]);
            p.arrow_head(fg, (11.0, 1.0), (0.0, -1.0), 3.5);
        }
        Icon::Close => {
            p.line_w(fg, 1.4, &[(4.5, 4.5), (11.5, 11.5)]);
            p.line_w(fg, 1.4, &[(11.5, 4.5), (4.5, 11.5)]);
        }
        Icon::DropDown => p.poly(Some(fg), None, &[(4.5, 6.5), (11.5, 6.5), (8.0, 10.0)]),
        Icon::Check => p.line_w(fg, 1.6, &[(3.0, 8.5), (6.5, 12.0), (13.0, 4.0)]),
        Icon::Browse => {
            p.poly(
                Some(light),
                Some(fg),
                &[
                    (1.5, 3.5),
                    (6.0, 3.5),
                    (7.5, 5.0),
                    (14.5, 5.0),
                    (14.5, 13.5),
                    (1.5, 13.5),
                ],
            );
            p.circle((8.5, 9.0), 2.5, Some(paper), Some(fg));
            p.line_w(fg, 1.5, &[(10.3, 10.8), (12.5, 13.0)]);
        }
        Icon::Info => {
            p.circle((8.0, 8.0), 6.5, Some(acc), None);
            p.rect(7.0, 7.0, 2.0, 5.5, Some(paper), None);
            p.rect(7.0, 3.8, 2.0, 2.0, Some(paper), None);
        }
        Icon::Plus => {
            p.line_w(fg, 1.6, &[(8.0, 2.5), (8.0, 13.5)]);
            p.line_w(fg, 1.6, &[(2.5, 8.0), (13.5, 8.0)]);
        }
        Icon::Minus => p.line_w(fg, 1.6, &[(2.5, 8.0), (13.5, 8.0)]),
        Icon::AddFilter => {
            p.line_w(fg, 1.8, &[(7.0, 2.0), (7.0, 12.0)]);
            p.line_w(fg, 1.8, &[(2.0, 7.0), (12.0, 7.0)]);
            p.poly(Some(fg), None, &[(10.0, 12.0), (15.0, 12.0), (12.5, 15.0)]);
        }
        Icon::LinkProportions => {
            p.line(fg, &[(8.0, 1.0), (4.0, 1.0), (4.0, 6.0)]);
            p.rect(2.5, 6.5, 3.0, 3.0, Some(paper), Some(fg));
            p.line(fg, &[(4.0, 10.0), (4.0, 15.0), (8.0, 15.0)]);
        }
        Icon::DistributeStates => {
            for (i, x) in [1.5f32, 6.0, 10.5].iter().enumerate() {
                p.rect(*x, 4.5 + i as f32, 4.0, 7.0, Some(paper), Some(fg));
            }
        }
        Icon::OnionSkin => {
            p.circle((6.0, 8.0), 4.5, None, Some(light));
            p.circle((10.0, 8.0), 4.5, Some(paper), Some(fg));
        }
        Icon::Loop => {
            p.arc(fg, (8.0, 8.0), 5.0, 20.0, 320.0);
            p.arrow_head(fg, (13.0, 6.5), (0.3, 1.0), 3.0);
        }
        Icon::Minimize => p.line_w(fg, 2.0, &[(4.0, 11.0), (12.0, 11.0)]),
        Icon::Maximize => {
            p.rect(3.5, 3.5, 9.0, 9.0, None, Some(fg));
            p.line_w(fg, 1.6, &[(3.5, 4.3), (12.5, 4.3)]);
        }
        Icon::Restore => {
            p.rect(5.5, 2.5, 8.0, 7.0, None, Some(fg));
            p.rect(2.5, 6.0, 8.0, 7.0, Some(paper), Some(fg));
            p.line_w(fg, 1.6, &[(2.5, 6.8), (10.5, 6.8)]);
        }
        Icon::WindowClose => {
            p.line_w(fg, 1.8, &[(4.0, 4.0), (12.0, 12.0)]);
            p.line_w(fg, 1.8, &[(12.0, 4.0), (4.0, 12.0)]);
        }
        Icon::Logo => {
            // eigenes App-Icon (Flamme aus Pixeln), in allen Designs gleich
            paint_app_logo(
                p.painter,
                Rect::from_min_size(p.o, egui::vec2(16.0 * p.s, 16.0 * p.s)),
            );
        }
        Icon::AiObject => {
            // Objektauswahl: gestrichelter Rahmen um eine Figur, Zeigerpfeil unten rechts
            p.dashed_rect(fg, 1.5, 1.5, 10.0, 10.0);
            p.circle((6.5, 5.5), 1.8, Some(acc), None);
            p.poly(
                Some(acc),
                None,
                &[(4.0, 10.0), (5.0, 7.8), (8.0, 7.8), (9.0, 10.0)],
            );
            p.poly(
                Some(paper),
                Some(fg),
                &[
                    (10.0, 9.0),
                    (15.0, 12.5),
                    (12.6, 12.8),
                    (13.8, 15.2),
                    (12.6, 15.8),
                    (11.4, 13.4),
                    (10.0, 15.0),
                ],
            );
        }
        Icon::AiSubject => {
            // Motiv auswählen: Personen-Silhouette mit gestricheltem Umriss
            p.circle((8.0, 5.0), 2.6, Some(acc), None);
            p.poly(
                Some(acc),
                None,
                &[
                    (3.0, 14.5),
                    (3.6, 11.0),
                    (5.6, 9.0),
                    (10.4, 9.0),
                    (12.4, 11.0),
                    (13.0, 14.5),
                ],
            );
            p.dashed(fg, (1.5, 14.5), (1.5, 2.0), 1.5);
            p.dashed(fg, (1.5, 2.0), (14.5, 2.0), 1.5);
            p.dashed(fg, (14.5, 2.0), (14.5, 14.5), 1.5);
            p.dashed(fg, (14.5, 14.5), (1.5, 14.5), 1.5);
        }
        Icon::AiRefineEdge => {
            // Kante verfeinern: Pinselspitze über einer gefransten (haarigen) Kante
            p.poly(
                Some(light),
                None,
                &[(1.0, 15.5), (1.0, 9.0), (7.0, 9.0), (7.0, 15.5)],
            );
            for (x, h) in [
                (1.5f32, 6.0f32),
                (2.7, 4.5),
                (3.9, 6.5),
                (5.1, 4.0),
                (6.3, 5.5),
                (7.5, 3.5),
                (8.7, 5.0),
            ] {
                p.line(fg, &[(x, 15.5), (x + 0.6, 15.5 - h)]);
            }
            brush_shape(p, fg, acc);
        }
        Icon::Swatches => {
            let cols = [st.red, st.yellow, st.green, st.blue];
            for (i, c) in cols.iter().enumerate() {
                let x = 1.5 + (i % 2) as f32 * 6.5;
                let y = 1.5 + (i / 2) as f32 * 6.5;
                p.rect(x, y, 6.0, 6.0, Some(*c), Some(fg));
            }
        }
    }
}

/// Alle Icons (für Tests und die Icon-Übersicht).
pub const ALL_ICONS: &[Icon] = &[
    Icon::Pointer,
    Icon::SelectBehind,
    Icon::Subselection,
    Icon::MaskElementSelect,
    Icon::Scale,
    Icon::Skew,
    Icon::Distort,
    Icon::NineSlice,
    Icon::Crop,
    Icon::ExportArea,
    Icon::Marquee,
    Icon::OvalMarquee,
    Icon::Lasso,
    Icon::PolygonLasso,
    Icon::MagicWand,
    Icon::Brush,
    Icon::Pencil,
    Icon::Eraser,
    Icon::Blur,
    Icon::Sharpen,
    Icon::Dodge,
    Icon::Burn,
    Icon::Smudge,
    Icon::RubberStamp,
    Icon::ReplaceColor,
    Icon::RedEye,
    Icon::Line,
    Icon::Pen,
    Icon::VectorPath,
    Icon::RedrawPath,
    Icon::Rectangle,
    Icon::Ellipse,
    Icon::Polygon,
    Icon::AutoShape,
    Icon::Text,
    Icon::Freeform,
    Icon::Reshape,
    Icon::PathScrubAdd,
    Icon::PathScrubSubtract,
    Icon::Knife,
    Icon::HotspotRect,
    Icon::HotspotCircle,
    Icon::HotspotPolygon,
    Icon::Slice,
    Icon::PolygonSlice,
    Icon::HideSlices,
    Icon::ShowSlices,
    Icon::Eyedropper,
    Icon::PaintBucket,
    Icon::Gradient,
    Icon::Hand,
    Icon::Zoom,
    Icon::StrokeColor,
    Icon::FillColor,
    Icon::DefaultColors,
    Icon::NoColor,
    Icon::SwapColors,
    Icon::ScreenStandard,
    Icon::ScreenFullMenus,
    Icon::ScreenFull,
    Icon::New,
    Icon::Save,
    Icon::Open,
    Icon::Import,
    Icon::Export,
    Icon::Print,
    Icon::Undo,
    Icon::Redo,
    Icon::Cut,
    Icon::Copy,
    Icon::Paste,
    Icon::Group,
    Icon::Ungroup,
    Icon::Join,
    Icon::Split,
    Icon::BringToFront,
    Icon::BringForward,
    Icon::SendBackward,
    Icon::SendToBack,
    Icon::EditSymbol,
    Icon::AlignLeft,
    Icon::RotateCcw,
    Icon::RotateCw,
    Icon::FlipHorizontal,
    Icon::FlipVertical,
    Icon::PanelMenu,
    Icon::CollapseRight,
    Icon::CollapseLeft,
    Icon::NewPage,
    Icon::Trash,
    Icon::NewLayer,
    Icon::NewSublayer,
    Icon::NewBitmap,
    Icon::AddMask,
    Icon::Duplicate,
    Icon::Eye,
    Icon::Lock,
    Icon::Folder,
    Icon::TriangleRight,
    Icon::TriangleDown,
    Icon::FirstFrame,
    Icon::PrevFrame,
    Icon::Play,
    Icon::Stop,
    Icon::NextFrame,
    Icon::LastFrame,
    Icon::ExitBitmapMode,
    Icon::ViewOriginal,
    Icon::ViewPreview,
    Icon::View2Up,
    Icon::View4Up,
    Icon::PageSmall,
    Icon::PageNavigate,
    Icon::Close,
    Icon::DropDown,
    Icon::Check,
    Icon::Browse,
    Icon::Info,
    Icon::Plus,
    Icon::Minus,
    Icon::AddFilter,
    Icon::LinkProportions,
    Icon::Document,
    Icon::DistributeStates,
    Icon::OnionSkin,
    Icon::Loop,
    Icon::Minimize,
    Icon::Maximize,
    Icon::Restore,
    Icon::WindowClose,
    Icon::Logo,
    Icon::Star,
    Icon::Swatches,
    Icon::AiObject,
    Icon::AiSubject,
    Icon::AiRefineEdge,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tool_has_an_icon() {
        for t in fp_engine::tools::TOOLS {
            let _ = tool_icon(t.id);
        }
        assert!(ALL_ICONS.len() >= 120);
    }

    #[test]
    fn concave_polygon_triangulates() {
        let ctx = egui::Context::default();
        let painter = Painter::new(
            ctx,
            egui::LayerId::background(),
            Rect::from_min_size(Pos2::ZERO, egui::vec2(32.0, 32.0)),
        );
        let pts: Vec<Pos2> = ARROW.iter().map(|&(x, y)| pos2(x, y)).collect();
        fill_polygon(&painter, &pts, Color32::BLACK);
    }
}
