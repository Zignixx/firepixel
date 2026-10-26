//! Brush tip outline at the pointer (Brush, Eraser, retouch tools; RE spec 74).
//!
//! Measured rule (RE spec 74 §1, §4): the outline has no colour of its own. Each of its pixels is
//! the screen pixel underneath XOR #FFFF00 (red and green inverted, blue kept: white → #0000FF,
//! black → #FFFF00, red → #00FF00). The line is 1 screen pixel wide at every zoom, the outer size is
//! the tip diameter × zoom + 1. Tool colour, mask editing (alpha channel with or without the
//! "Show fill and stroke" underlay, grayscale) and the brush category have no influence.
//!
//! egui has no XOR blend, so the canvas samples the displayed canvas image (`CanvasCache::
//! shown_image`) under each outline pixel and paints that pixel in the XOR colour; outside the
//! image the pasteboard colour is the underlying colour.

use egui::{Color32, Mesh, Painter, Pos2, Rect, pos2};
use fp_raster::RgbaImage;
use std::collections::BTreeSet;

/// XOR mask of the outline (RE spec 74 §1).
pub const XOR: [u8; 3] = [0xFF, 0xFF, 0x00];

/// Colour of an outline pixel over the displayed colour `under`.
pub fn outline_color(under: Color32) -> Color32 {
    Color32::from_rgb(under.r() ^ XOR[0], under.g() ^ XOR[1], under.b() ^ XOR[2])
}

/// Device pixels of the 1 px outline of a tip centred at (`cx`, `cy`) (device pixels, continuous)
/// with the screen diameter `diameter` (tip size × zoom × pixels per point). Round tips give an
/// 8-connected circle, square tips the border of a square; the outer size is `diameter + 1` for a
/// centre on a pixel centre (RE spec 74 §3).
pub fn ring_pixels(cx: f64, cy: f64, diameter: f64, round: bool) -> Vec<(i32, i32)> {
    let r = (diameter / 2.0).max(0.5);
    let (x0, x1) = ((cx - r).floor() as i32, (cx + r).floor() as i32);
    let (y0, y1) = ((cy - r).floor() as i32, (cy + r).floor() as i32);
    let mut set = BTreeSet::new();
    if !round {
        for x in x0..=x1 {
            set.insert((x, y0));
            set.insert((x, y1));
        }
        for y in y0..=y1 {
            set.insert((x0, y));
            set.insert((x1, y));
        }
        return set.into_iter().collect();
    }
    // column pass and row pass: together a closed ring without gaps at any slope
    for x in x0..=x1 {
        let dx = (x as f64 + 0.5 - cx).clamp(-r, r);
        let dy = (r * r - dx * dx).max(0.0).sqrt();
        set.insert((x, ((cy - dy).floor() as i32).clamp(y0, y1)));
        set.insert((x, ((cy + dy).floor() as i32).clamp(y0, y1)));
    }
    for y in y0..=y1 {
        let dy = (y as f64 + 0.5 - cy).clamp(-r, r);
        let dx = (r * r - dy * dy).max(0.0).sqrt();
        set.insert((((cx - dx).floor() as i32).clamp(x0, x1), y));
        set.insert((((cx + dx).floor() as i32).clamp(x0, x1), y));
    }
    set.into_iter().collect()
}

/// Displayed colour at the screen point `p` (points): the canvas image `img` shown in `tex_rect`
/// (straight alpha, composited over white like the page's checkerboard light squares), else the
/// pasteboard colour `pasteboard`.
pub fn under_color(
    img: Option<&RgbaImage>,
    tex_rect: Option<Rect>,
    pasteboard: Color32,
    p: Pos2,
) -> Color32 {
    let (Some(img), Some(tr)) = (img, tex_rect) else {
        return pasteboard;
    };
    if !tr.contains(p) || tr.width() <= 0.0 || tr.height() <= 0.0 {
        return pasteboard;
    }
    let (w, h) = (img.width() as f32, img.height() as f32);
    let ix = (((p.x - tr.min.x) / tr.width()) * w).floor() as i32;
    let iy = (((p.y - tr.min.y) / tr.height()) * h).floor() as i32;
    let b = img.bounds();
    let Some(px) = img.get(b.left + ix, b.top + iy) else {
        return pasteboard;
    };
    let a = px[3] as u32;
    let over_white = |c: u8| ((c as u32 * a + 255 * (255 - a) + 127) / 255) as u8;
    Color32::from_rgb(over_white(px[0]), over_white(px[1]), over_white(px[2]))
}

/// Paints the outline of a tip at the screen point `at` (points) with the screen diameter
/// `diameter_pt` (points); `ppp` = pixels per point; `under` gives the displayed colour at a
/// screen point (points).
pub fn paint(
    painter: &Painter,
    at: Pos2,
    diameter_pt: f32,
    round: bool,
    ppp: f32,
    under: impl Fn(Pos2) -> Color32,
) {
    let ppp = ppp.max(0.01);
    let px = ring_pixels(
        (at.x * ppp) as f64,
        (at.y * ppp) as f64,
        (diameter_pt * ppp) as f64,
        round,
    );
    let mut mesh = Mesh::default();
    for (x, y) in px {
        let r = Rect::from_min_max(
            pos2(x as f32 / ppp, y as f32 / ppp),
            pos2((x + 1) as f32 / ppp, (y + 1) as f32 / ppp),
        );
        mesh.add_colored_rect(r, outline_color(under(r.center())));
    }
    painter.add(mesh);
}

#[cfg(test)]
mod tests {
    use super::*;
    use fp_geom::IRect;

    fn bbox(px: &[(i32, i32)]) -> (i32, i32) {
        let (x0, x1) = (
            px.iter().map(|p| p.0).min().unwrap(),
            px.iter().map(|p| p.0).max().unwrap(),
        );
        let (y0, y1) = (
            px.iter().map(|p| p.1).min().unwrap(),
            px.iter().map(|p| p.1).max().unwrap(),
        );
        (x1 - x0 + 1, y1 - y0 + 1)
    }

    /// RE spec 74 §1: measured pairs underlying colour → outline colour.
    #[test]
    fn outline_colour_is_the_pixel_below_xor_yellow_re74() {
        for (under, want) in [
            ((255, 255, 255), (0, 0, 255)),
            ((0, 0, 0), (255, 255, 0)),
            ((255, 0, 0), (0, 255, 0)),
            ((0, 200, 0), (255, 55, 0)),
            ((0, 0, 255), (255, 255, 255)),
            ((128, 128, 128), (127, 127, 128)),
            ((255, 255, 0), (0, 0, 0)),
            ((0x30, 0x50, 0xE0), (0xCF, 0xAF, 0xE0)),
            ((0xD3, 0xDA, 0xF8), (0x2C, 0x25, 0xF8)),
        ] {
            assert_eq!(
                outline_color(Color32::from_rgb(under.0, under.1, under.2)),
                Color32::from_rgb(want.0, want.1, want.2),
                "under {under:?}"
            );
        }
    }

    /// RE spec 74 §3: outer size = diameter + 1, 1 px line, closed ring.
    #[test]
    fn ring_is_one_pixel_wide_with_outer_size_diameter_plus_one_re74() {
        for d in [2.0, 4.0, 6.0, 8.0, 12.0, 13.0, 30.0, 45.0] {
            let px = ring_pixels(100.5, 80.5, d, true);
            let (w, h) = bbox(&px);
            assert_eq!((w, h), (d as i32 + 1, d as i32 + 1), "diameter {d}");
            let set: BTreeSet<_> = px.iter().copied().collect();
            for &(x, y) in &px {
                // every pixel lies on the circle within one pixel
                let dist =
                    ((x as f64 + 0.5 - 100.5).powi(2) + (y as f64 + 0.5 - 80.5).powi(2)).sqrt();
                assert!((dist - d / 2.0).abs() <= 1.0, "d {d}: ({x},{y}) at {dist}");
                // closed: at least two 8-neighbours on the ring
                let n = (-1..=1)
                    .flat_map(|dx| (-1..=1).map(move |dy| (dx, dy)))
                    .filter(|&(dx, dy)| (dx, dy) != (0, 0) && set.contains(&(x + dx, y + dy)))
                    .count();
                assert!(n >= 2, "d {d}: ({x},{y}) open");
            }
            // no filled interior: the centre stays free
            assert!(!set.contains(&(100, 80)) || d < 3.0);
        }
        // the 30 px brush of RE spec 74 §2 has about the measured 85 pixels
        let n = ring_pixels(100.5, 80.5, 30.0, true).len();
        assert!((80..=96).contains(&n), "{n}");
        // a square tip: the border of a (d + 1)-square
        let sq = ring_pixels(10.5, 10.5, 13.0, false);
        assert_eq!(bbox(&sq), (14, 14));
        assert_eq!(sq.len(), 4 * 13);
    }

    fn page() -> RgbaImage {
        // 4 × 1 device pixels: white, black, red, transparent
        let mut img = RgbaImage::new(IRect::new(0, 0, 4, 1)).expect("image");
        for (x, c) in [
            [255, 255, 255, 255],
            [0, 0, 0, 255],
            [255, 0, 0, 255],
            [0, 0, 0, 0],
        ]
        .into_iter()
        .enumerate()
        {
            img.set_pixel(x as i32, 0, fp_color::Rgba8::from_array(c));
        }
        img
    }

    /// RE spec 74 §2: the colour depends only on what is displayed below (canvas, bitmap,
    /// transparent area over the white page, pasteboard), never on the tool colour or a mask.
    #[test]
    fn outline_colour_per_situation_re74() {
        let img = page();
        let tr = Some(Rect::from_min_size(pos2(10.0, 10.0), egui::vec2(4.0, 1.0)));
        let pb = Color32::from_gray(200);
        let at = |x: f32| outline_color(under_color(Some(&img), tr, pb, pos2(10.0 + x, 10.5)));
        assert_eq!(at(0.5), Color32::from_rgb(0, 0, 255), "white canvas");
        assert_eq!(at(1.5), Color32::from_rgb(255, 255, 0), "black bitmap");
        assert_eq!(at(2.5), Color32::from_rgb(0, 255, 0), "red bitmap");
        assert_eq!(
            at(3.5),
            Color32::from_rgb(0, 0, 255),
            "transparent over white"
        );
        assert_eq!(
            outline_color(under_color(Some(&img), tr, pb, pos2(2.0, 2.0))),
            Color32::from_rgb(55, 55, 200),
            "pasteboard"
        );
        assert_eq!(
            outline_color(under_color(None, None, pb, pos2(10.5, 10.5))),
            Color32::from_rgb(55, 55, 200),
            "no canvas image yet"
        );
    }

    /// The painted mesh has one 1-device-pixel quad per outline pixel in its XOR colour.
    #[test]
    fn paint_adds_one_quad_per_pixel_re74() {
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            paint(ui.painter(), pos2(50.5, 50.5), 13.0, true, 1.0, |_| {
                Color32::WHITE
            });
        });
        out.textures_delta.clear();
        let meshes: Vec<_> = out
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Mesh(m) => Some(m.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(meshes.len(), 1);
        let m = &meshes[0];
        assert_eq!(
            m.vertices.len(),
            4 * ring_pixels(50.5, 50.5, 13.0, true).len()
        );
        assert!(
            m.vertices
                .iter()
                .all(|v| v.color == Color32::from_rgb(0, 0, 255))
        );
    }
}
