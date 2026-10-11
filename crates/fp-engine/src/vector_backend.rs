//! Anbindung der Pfadoperationen von fp-vector (Feature `vector`).
//!
//! Einzige Stelle, die die API von fp-vector benutzt; ändert sie sich, ist nur diese Datei anzupassen
//! (oder das Feature abzuschalten, dann gilt [`crate::geometry::BasicGeometry`]).

use crate::error::{EngineError, Result};
use crate::geometry::{to_bezier, from_bezier, GeometryOps, PathBoolOp, PathShape};
use fp_geom::offset::{CapKind, JoinKind, OffsetOptions};
use fp_model::Contour;
use fp_vector::pathops::{self, Operand, PathOp};

/// Pfadoperationen über fp-vector (16).
#[derive(Clone, Copy, Debug, Default)]
pub struct VectorGeometry;

fn to_v(c: &[Contour]) -> Vec<pathops::Contour> {
    c.iter().map(|k| pathops::Contour::from_bezier(&to_bezier(k))).collect()
}

fn from_v(c: &[pathops::Contour]) -> Vec<Contour> {
    c.iter().map(|k| from_bezier(&k.to_bezier())).collect()
}

impl GeometryOps for VectorGeometry {
    fn name(&self) -> &'static str {
        "fp-vector"
    }

    fn combine(&self, op: PathBoolOp, paths: &[PathShape]) -> Result<Vec<Vec<Contour>>> {
        let ops: Vec<Operand> = paths.iter().map(|p| Operand::new(to_v(&p.contours), p.even_odd, PathOp::Normal)).collect();
        let none = || EngineError::Selection(fp_i18n::t("err-sel-two-paths-needed"));
        Ok(match op {
            PathBoolOp::Union => vec![from_v(&pathops::union_paths(&ops).ok_or_else(none)?)],
            PathBoolOp::Intersect => vec![from_v(&pathops::intersect_paths(&ops).ok_or_else(none)?)],
            PathBoolOp::Punch => pathops::punch_paths(&ops).ok_or_else(none)?.iter().map(|c| from_v(c)).collect(),
            PathBoolOp::Crop => pathops::crop_paths(&ops).ok_or_else(none)?.iter().map(|c| from_v(c)).collect(),
        })
    }

    fn expand_stroke(&self, contours: &[Contour], width: f64, join: u32, cap: u32, miter: f64) -> Result<Vec<Contour>> {
        let r = pathops::expand_stroke(
            &to_v(contours),
            width,
            JoinKind::from_u32(join),
            miter,
            CapKind::from_u32(cap),
            &OffsetOptions::ORIGINAL,
        );
        Ok(from_v(&r))
    }

    fn inset(&self, contours: &[Contour], distance: f64, expand: bool, join: u32, miter: f64) -> Result<Vec<Contour>> {
        let r = pathops::inset_path(&to_v(contours), distance, JoinKind::from_u32(join), miter, expand, &OffsetOptions::ORIGINAL);
        Ok(from_v(&r))
    }

    fn simplify(&self, contours: &[Contour], amount: f64) -> Result<Vec<Contour>> {
        Ok(from_v(&pathops::simplify(&to_v(contours), amount)))
    }
}
