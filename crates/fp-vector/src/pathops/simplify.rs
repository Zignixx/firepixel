//! Simplify (re 30 §2.3; replaces the former textbook refit of spec 16 §3.2).
//!
//! The command calls the refit with the dialog amount as tolerance in pixels (1..25, default 10),
//! corner angle 30° and 16 samples. The vector path tool uses the same refit with `10 − precision`.

use super::geom::Contour;
pub use super::refit::refit_contour;

/// Default of the preference `SimplifyAmount`.
pub const DEFAULT_AMOUNT: f64 = 10.0;

/// "Modify → Alter Path → Simplify": `amount` = dialog value, tolerance in pixels.
pub fn simplify(contours: &[Contour], amount: f64) -> Vec<Contour> {
    contours
        .iter()
        .map(|c| refit_contour(c, amount, 30.0, 16))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use fp_geom::Vec2;

    #[test]
    fn rectangle_stays() {
        let rect = Contour::polygon(
            &[
                Vec2::new(0.0, 0.0),
                Vec2::new(10.0, 0.0),
                Vec2::new(10.0, 5.0),
                Vec2::new(0.0, 5.0),
            ],
            true,
        );
        let r = simplify(std::slice::from_ref(&rect), 10.0);
        assert_eq!(r[0].anchors(), rect.anchors());
    }
}
