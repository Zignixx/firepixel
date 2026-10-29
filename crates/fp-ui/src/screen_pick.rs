//! BA-114 (F5): reading the real screen colour outside the Firepixel window for the Eyedropper
//! tool and the colour pop-up's eyedropper. The reference application samples the screen pixel
//! anywhere on the screen; inside our own window the existing paths (document pixel, window
//! screenshot) stay unchanged, only points outside the window come here.
//!
//! Only the small region around the cursor is read (1 × 1, 3 × 3 or 5 × 5 screen pixels), and only
//! when the pointer reports a new position. Backends (no `unsafe` in our code):
//! - Windows and macOS: the `xcap` crate (GDI on Windows, Core Graphics on macOS). macOS asks for
//!   the Screen Recording permission; without it the system returns only the desktop background.
//! - Linux X11: `x11rb` (pure Rust X11 client), `GetImage` on the root window.
//! - Linux Wayland sessions: the compositor does not let clients read the screen without a portal
//!   dialog, so no colour is read and the callers keep the working colour (documented limit).

use egui::Pos2;
use fp_color::Argb;

/// A captured screen region, RGBA bytes row by row (`width * height * 4`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Region {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Something that can read a rectangle of the screen in physical screen coordinates (macOS:
/// points). The region may come back clipped to the screen or at a higher pixel density (Retina);
/// the whole returned image always stands for the requested rectangle.
pub trait ScreenSource {
    fn capture(&self, x: i32, y: i32, width: u32, height: u32) -> Option<Region>;
}

/// Mean colour of all pixels of the region (each channel rounded), opaque. `None` for an empty or
/// malformed region.
pub fn average(region: &Region) -> Option<Argb> {
    let n = region.width as usize * region.height as usize;
    if n == 0 || region.rgba.len() < n * 4 {
        return None;
    }
    let mut sum = [0u64; 3];
    for px in region.rgba.as_chunks::<4>().0.iter().take(n) {
        sum[0] += px[0] as u64;
        sum[1] += px[1] as u64;
        sum[2] += px[2] as u64;
    }
    let n = n as u64;
    let ch = |v: u64| ((v + n / 2) / n) as u32;
    Some(Argb(
        0xff00_0000 | ch(sum[0]) << 16 | ch(sum[1]) << 8 | ch(sum[2]),
    ))
}

/// Colour at the screen point `(x, y)`: `samples` = 1, 3 or 5 (the Eyedropper's Sample option:
/// 1 pixel, 3 × 3 or 5 × 5 average), centred on the point. Other values count as 1.
pub fn sample_at(src: &dyn ScreenSource, x: i32, y: i32, samples: u32) -> Option<Argb> {
    let n = match samples {
        3 => 3,
        5 => 5,
        _ => 1,
    };
    let r = (n as i32 - 1) / 2;
    average(&src.capture(x - r, y - r, n, n)?)
}

/// Screen coordinates (physical pixels; macOS: points) of the egui point `p` of the main window,
/// from the window's inner position. `None` when the platform reports no window position (tests,
/// some Wayland compositors).
pub fn to_screen(ctx: &egui::Context, p: Pos2) -> Option<(i32, i32)> {
    let (inner, native) = ctx.input(|i| {
        let v = i.viewport();
        (v.inner_rect, v.native_pixels_per_point)
    });
    let inner = inner?;
    let ppp = ctx.pixels_per_point();
    Some(map_to_screen(inner.min, p, ppp, native.unwrap_or(1.0)))
}

/// Pure part of [`to_screen`]: `origin` = window inner position in egui points, `ppp` = egui
/// pixels per point, `native` = the system scale factor (macOS works in points: divided out).
pub fn map_to_screen(origin: Pos2, p: Pos2, ppp: f32, native: f32) -> (i32, i32) {
    let (mut x, mut y) = ((origin.x + p.x) * ppp, (origin.y + p.y) * ppp);
    if cfg!(target_os = "macos") && native > 0.0 {
        x /= native;
        y /= native;
    }
    (x.floor() as i32, y.floor() as i32)
}

/// The screen colour under the egui point `p` (outside our window), averaged over `samples`;
/// `None` when the screen cannot be read (Wayland, missing permission, no window position).
pub fn screen_color(ctx: &egui::Context, p: Pos2, samples: u32) -> Option<Argb> {
    let (x, y) = to_screen(ctx, p)?;
    sample_at(&SystemScreen, x, y, samples)
}

/// The system screen (see the module notes for the backends).
pub struct SystemScreen;

impl ScreenSource for SystemScreen {
    fn capture(&self, x: i32, y: i32, width: u32, height: u32) -> Option<Region> {
        backend::capture(x, y, width, height)
    }
}

/// Clip the rectangle `(x, y, w, h)` to the bounds `(bx, by, bw, bh)`; `None` if nothing is left.
pub fn clip(
    (x, y, w, h): (i32, i32, u32, u32),
    (bx, by, bw, bh): (i32, i32, u32, u32),
) -> Option<(i32, i32, u32, u32)> {
    let l = x.max(bx) as i64;
    let t = y.max(by) as i64;
    let r = (x as i64 + w as i64).min(bx as i64 + bw as i64);
    let b = (y as i64 + h as i64).min(by as i64 + bh as i64);
    (r > l && b > t).then(|| (l as i32, t as i32, (r - l) as u32, (b - t) as u32))
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
mod backend {
    use super::{Region, clip};

    pub fn capture(x: i32, y: i32, width: u32, height: u32) -> Option<Region> {
        let cx = x + (width / 2) as i32;
        let cy = y + (height / 2) as i32;
        let m = xcap::Monitor::from_point(cx, cy).ok()?;
        let (mx, my) = (m.x().ok()?, m.y().ok()?);
        let (mw, mh) = (m.width().ok()?, m.height().ok()?);
        // like the reference at the canvas edge: the window is cut at the monitor edge
        let (l, t, w, h) = clip((x, y, width, height), (mx, my, mw, mh))?;
        let img = m
            .capture_region((l - mx) as u32, (t - my) as u32, w, h)
            .ok()?;
        let (width, height) = (img.width(), img.height());
        Some(Region {
            width,
            height,
            rgba: img.into_raw(),
        })
    }
}

#[cfg(target_os = "linux")]
mod backend {
    use super::{Region, clip};
    use std::sync::{Mutex, OnceLock};
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{ConnectionExt, ImageFormat, ImageOrder};
    use x11rb::rust_connection::RustConnection;

    /// A Wayland session: XWayland's root window does not show native Wayland windows, so reading
    /// it would return wrong colours. No reading there (the callers keep the working colour).
    fn wayland_session() -> bool {
        std::env::var("XDG_SESSION_TYPE").is_ok_and(|v| v.eq_ignore_ascii_case("wayland"))
            || (std::env::var_os("WAYLAND_DISPLAY").is_some()
                && std::env::var_os("DISPLAY").is_none())
    }

    fn connection() -> &'static Mutex<Option<(RustConnection, usize)>> {
        static CONN: OnceLock<Mutex<Option<(RustConnection, usize)>>> = OnceLock::new();
        CONN.get_or_init(|| Mutex::new(x11rb::connect(None).ok()))
    }

    pub fn capture(x: i32, y: i32, width: u32, height: u32) -> Option<Region> {
        if wayland_session() {
            return None;
        }
        let guard = connection().lock().ok()?;
        let (conn, screen_num) = guard.as_ref()?;
        let setup = conn.setup();
        let screen = setup.roots.get(*screen_num)?;
        let bounds = (
            0,
            0,
            screen.width_in_pixels as u32,
            screen.height_in_pixels as u32,
        );
        let (l, t, w, h) = clip((x, y, width, height), bounds)?;
        // only the common 24/32-bit TrueColor layout (red 0xff0000, green 0xff00, blue 0xff)
        let visual = screen
            .allowed_depths
            .iter()
            .flat_map(|d| d.visuals.iter())
            .find(|v| v.visual_id == screen.root_visual)?;
        if (visual.red_mask, visual.green_mask, visual.blue_mask) != (0xff_0000, 0xff00, 0xff) {
            return None;
        }
        let reply = conn
            .get_image(
                ImageFormat::Z_PIXMAP,
                screen.root,
                l as i16,
                t as i16,
                w as u16,
                h as u16,
                !0,
            )
            .ok()?
            .reply()
            .ok()?;
        let bpp = setup
            .pixmap_formats
            .iter()
            .find(|f| f.depth == reply.depth)
            .map(|f| f.bits_per_pixel)?;
        if bpp != 32 || reply.data.len() < (w * h * 4) as usize {
            return None;
        }
        let lsb = setup.image_byte_order == ImageOrder::LSB_FIRST;
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for px in reply.data.as_chunks::<4>().0.iter().take((w * h) as usize) {
            let (r, g, b) = if lsb {
                (px[2], px[1], px[0])
            } else {
                (px[1], px[2], px[3])
            };
            rgba.extend_from_slice(&[r, g, b, 255]);
        }
        Some(Region {
            width: w,
            height: h,
            rgba,
        })
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
mod backend {
    use super::Region;

    pub fn capture(_x: i32, _y: i32, _width: u32, _height: u32) -> Option<Region> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fake screen: pixel colour = function of the screen position; records the requested rect.
    struct Fake {
        f: fn(i32, i32) -> [u8; 3],
        asked: std::cell::RefCell<Vec<(i32, i32, u32, u32)>>,
        /// pixel density (2 = Retina-like: each point comes back as 2 × 2 pixels)
        density: u32,
    }

    impl ScreenSource for Fake {
        fn capture(&self, x: i32, y: i32, w: u32, h: u32) -> Option<Region> {
            self.asked.borrow_mut().push((x, y, w, h));
            let d = self.density;
            let mut rgba = Vec::new();
            for yy in 0..h * d {
                for xx in 0..w * d {
                    let c = (self.f)(x + (xx / d) as i32, y + (yy / d) as i32);
                    rgba.extend_from_slice(&[c[0], c[1], c[2], 255]);
                }
            }
            Some(Region {
                width: w * d,
                height: h * d,
                rgba,
            })
        }
    }

    fn fake(f: fn(i32, i32) -> [u8; 3]) -> Fake {
        Fake {
            f,
            asked: Default::default(),
            density: 1,
        }
    }

    #[test]
    fn one_pixel_reads_exactly_the_point() {
        let src = fake(|x, y| [x as u8, y as u8, 7]);
        assert_eq!(sample_at(&src, 10, 20, 1), Some(Argb(0xff0a_1407)));
        assert_eq!(src.asked.borrow()[0], (10, 20, 1, 1));
    }

    #[test]
    fn three_and_five_average_around_the_point() {
        // red left of x = 100, blue from x = 100
        let src = fake(|x, _| if x < 100 { [255, 0, 0] } else { [0, 0, 255] });
        // 3 × 3 at (100, 5): columns 99, 100, 101 = 1 red + 2 blue
        assert_eq!(sample_at(&src, 100, 5, 3), Some(Argb(0xff55_00aa)));
        assert_eq!(src.asked.borrow()[0], (99, 4, 3, 3));
        // 5 × 5 at (101, 5): columns 99..=103 = 1 red + 4 blue (true mean, rounded)
        assert_eq!(sample_at(&src, 101, 5, 5), Some(Argb(0xff33_00cc)));
        assert_eq!(src.asked.borrow()[1], (99, 3, 5, 5));
        // other sample counts are one pixel
        assert_eq!(sample_at(&src, 99, 5, 2), Some(Argb(0xffff_0000)));
    }

    #[test]
    fn high_density_region_averages_all_returned_pixels() {
        let mut src = fake(|x, _| {
            if x % 2 == 0 {
                [200, 100, 0]
            } else {
                [0, 100, 200]
            }
        });
        src.density = 2;
        assert_eq!(sample_at(&src, 4, 0, 1), Some(Argb(0xffc8_6400)));
        assert_eq!(sample_at(&src, 4, 0, 3), Some(Argb(0xff43_6485)));
    }

    #[test]
    fn empty_or_short_region_gives_none() {
        let r = Region {
            width: 2,
            height: 1,
            rgba: vec![0; 4],
        };
        assert_eq!(average(&r), None);
        let r = Region {
            width: 0,
            height: 0,
            rgba: vec![],
        };
        assert_eq!(average(&r), None);
    }

    #[test]
    fn clip_cuts_at_the_monitor_edge() {
        let mon = (-1920, 0, 1920, 1080);
        assert_eq!(clip((-2, 5, 5, 5), mon), Some((-2, 5, 2, 5)));
        assert_eq!(clip((-1922, -2, 5, 5), mon), Some((-1920, 0, 3, 3)));
        assert_eq!(clip((0, 0, 5, 5), mon), None);
    }

    #[test]
    fn egui_points_map_to_screen_pixels() {
        // window at (100, 50) points, pointer 30 points left of it, scale 1.5
        let (x, y) = map_to_screen(Pos2::new(100.0, 50.0), Pos2::new(-30.0, 10.0), 1.5, 1.5);
        if cfg!(target_os = "macos") {
            assert_eq!((x, y), (70, 60));
        } else {
            assert_eq!((x, y), (105, 90));
        }
    }

    /// Reads the real screen (needs a desktop session); run with `-- --ignored`.
    #[test]
    #[ignore]
    fn real_screen_returns_a_colour() {
        let c = sample_at(&SystemScreen, 5, 5, 3);
        assert!(c.is_some(), "screen could not be read");
    }
}
