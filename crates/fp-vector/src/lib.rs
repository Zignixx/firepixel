//! Vektor-Rendering und Pfadoperationen wie das Original.
//!
//! Grundlage: Spezifikation `re\07-vektor-rendering.md` (Rasterizer, Füllungen, Striche/Pinsel)
//! und `re\16-pfadoperationen.md` (boolesche Operationen, Compound Shapes, Pfadbefehle) mit den
//! Python-Referenzen `re\vektor\` und `re\pfade\`. Dieses Crate hängt **nicht** vom Dokumentmodell
//! ab: fp-render befüllt die schlanken Eingabe-Strukturen aus [`attrs`].
//!
//! # Überblick
//!
//! | Modul | Inhalt | Spezifikation |
//! |---|---|---|
//! | [`attrs`] | Eingabe: [`VectorPath`] (Polylinien + Attribute), [`Fill`], [`Stroke`], [`BrushParams`] | 07 §5.1, §6.1 |
//! | [`coverage`] | Deckungsmasken: Verfahren A (XOR), B (Windung), G (AGM), harte Kante | 07 §4 |
//! | [`feather`] | weiche Kante (Dreiecksfilter mit screen-Eigenheit der letzten Zeile) | 07 §5.6 |
//! | [`gradient`] | Verlaufsparameter aller Formen, Farbtabelle (2001 Einträge, abgeschnittene Gewichte) | 07 §5.2, §5.3 |
//! | [`paint`] | Vollton, Muster, Web-Dither, Fülltextur, Transparenz-Dither | 07 §5.4, §5.5 |
//! | [`contour_gradient`] | Konturverlauf (Abstand zum Pfad) | 07 §5.7 |
//! | [`brush`] | Spitzenmaske, Subpixel-Verschiebung, Strichmuster, Läufer simple, MINSTD | 07 §6.2–§6.4, §6.7 |
//! | [`natural`] | natürliche Pinsel, Empfindlichkeiten, Mehrfachspitzen, alphaRemap, Strichtextur | 07 §6.4, §6.8 |
//! | [`stroke`] | Strichebene: wirksame Platzierung, Stempeln, Puffergrenzen, Innen/außen-Maske | 07 §6.5, §6.6 |
//! | [`render`] | Pfad → RGBA-Ebene (Füllung/Strich, skalares over), Grenzen | 07 §7.5 |
//! | [`hit`] | Trefferprüfung Punkt in Füllung/Strich | 16 §1.5 |
//! | [`pathops`] | boolesche Operationen, Compound Shapes, Strich erweitern, Einlassen, Vereinfachen, Verbinden/Teilen, Umformen, Grundformen | 16 |
//!
//! # Schnellstart für fp-render
//!
//! ```
//! use fp_vector::{VectorPath, PathContour, Fill, render_path};
//! use fp_color::Argb;
//! use fp_geom::{IRect, Vec2};
//!
//! let sq = PathContour::new(vec![Vec2::new(2.0, 2.0), Vec2::new(8.0, 2.0), Vec2::new(8.0, 8.0), Vec2::new(2.0, 8.0)], true);
//! let mut path = VectorPath::new(vec![sq]);
//! path.fill = Some(Fill::solid(Argb(0xFFFF0000)));
//! let img = render_path(&path, IRect::new(0, 0, 10, 10));
//! assert_eq!(img.pixel(5, 5).a, 255);
//! ```
//!
//! # Konventionen
//!
//! - Koordinaten in Dokumentpixeln, y nach unten, `f64` ([`fp_geom::Vec2`]).
//! - Pixel `[r, g, b, a]` nicht vormultipliziert; Abtastung von Verläufen an der linken oberen
//!   Pixelecke, Deckung mit 4×4-Teilabtastung wie das Original.
//! - Kein `unsafe`, keine Panik auf Eingaben; Rechenergebnisse sind unabhängig von der Thread-Zahl.

pub mod attrs;
pub mod brush;
pub mod contour_gradient;
pub mod coverage;
pub mod feather;
pub mod gradient;
pub mod hit;
pub mod natural;
pub mod paint;
pub mod pathops;
pub mod render;
pub mod stroke;

pub use attrs::{
    BrushParams, BrushType, Feedback, Fill, FillEdge, FillKind, FillTexture, Gradient,
    GradientHandles, GradientShape, GradientStop, PathContour, PointDynamics, SenseSource,
    SenseTarget, StampingMode, Stroke, StrokePlacement, StrokeTexture, TipColoringMode, TipShape,
    TipSpacingMode, VectorPath,
};
pub use coverage::{Coverage, Rasterizer, fill_coverage};
pub use hit::{HitPart, hit_test};
pub use render::{
    Missing, PathLayer, path_bounds, path_bounds_with_stroke, render_bounds, render_path,
    render_path_layer, render_path_pixels,
};
