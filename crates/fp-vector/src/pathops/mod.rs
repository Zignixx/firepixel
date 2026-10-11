//! Pfadoperationen (Spezifikation `re\16-pfadoperationen.md`, Referenzen `re\pfade\`).
//!
//! | Modul | Inhalt | Spezifikation |
//! |---|---|---|
//! | [`geom`] | [`Contour`]/[`Node`] in `f64`, Abflachung, Fläche, Windungszahl, Grenzen | 16 §1 |
//! | [`mod@boolean`] | Vereinigen/Schnittmenge/Differenz/XOR auf Bézier-Ebene mit Original-Raster (0,005/0,015) | 16 §2.3, §2.7 |
//! | [`compound`] | Compound Shapes (`POPi`, unten → oben, Konturreihenfolge), destruktive Befehle Vereinigen/Überschneiden/Ausstanzen/Zuschneiden | 16 §2.1–§2.5 |
//! | [`offset`] | Strich erweitern, Pfad einlassen | 16 §3.3, §3.4 |
//! | [`mod@simplify`] | Vereinfachen (Schneider-Refit) | 16 §3.2 |
//! | [`edit`] | Verbinden/Teilen, Punkte einfügen/entfernen, Freiform, Bereich umformen | 16 §3.1, §3.5–§3.7 |
//! | [`shapes`] | Ellipse (κ = 0,55229187), Rechteck/Rundrechteck, Polygon/Stern | 16 §3.10–§3.12 |
//!
//! Ergebnisse boolescher Operationen haben immer `EOFb = 1` (16 §1.6). Konturen kommen aus dem
//! Modell als `fp_geom::BezierContour` (`f32`) und werden mit [`Contour::from_bezier`] /
//! [`Contour::to_bezier`] umgewandelt.

pub mod boolean;
pub mod compound;
pub mod edit;
pub mod geom;
pub mod offset;
pub mod shapes;
pub mod simplify;

pub use boolean::{boolean, merge_collinear, polygon_boolean, BoolOp, FillRule, Region};
pub use compound::{
    create_compound_ops, crop_paths, evaluate_compound, original_order, intersect_paths, order_contours, punch_paths, union_paths, Operand,
    PathOp,
};
pub use edit::{join_paths, push_pull, reshape_area, split_path, End, JoinResult};
pub use geom::{area, Contour, Node};
pub use offset::{expand_stroke, inset_path};
pub use simplify::simplify;
