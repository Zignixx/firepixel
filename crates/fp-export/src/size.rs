//! „Optimieren auf Größe“ (Assistent für Dateigrößenoptimierung, 08 §7, `@00751270`): Bisektion über
//! die Farbanzahl (indiziert, gekoppelt mit dem Diffusionsdither) bzw. die JPEG-Qualität (gekoppelt mit
//! der Glättung). Jeder Schritt rechnet die komplette Pipeline neu; das Ergebnis ist die zuletzt
//! berechnete Einstellung (kann das Ziel überschreiten). Andere Formate: keine Suche.

use fp_raster::RgbaImage;

use crate::Result;
use crate::export::{EncodeExtras, export_image};
use crate::options::{ColorMode, Dither, ExportFormat, OptimizeSettings, PaletteKind};
use crate::pipeline::{ExportContext, Optimized, effective_color_mode};

/// Ergebnis der Suche.
#[derive(Clone, Debug, PartialEq)]
pub struct SizeResult {
    /// Gefundene Einstellungen.
    pub settings: OptimizeSettings,
    /// Dateigröße mit diesen Einstellungen.
    pub bytes: usize,
    /// Wurde gesucht (sonst Format ohne Suche, Einstellungen unverändert)?
    pub changed: bool,
}

/// Kleinste Dateigröße, die [`optimize_to_size`] erreichen kann: indiziert mit 2 Farben (und dem
/// unteren Ende des Ditherbereichs), JPEG mit Qualität 2 (und der stärksten Glättung der Suche).
/// `None` = Format ohne Suche (PNG 24/32, TIFF/BMP 24, WBMP …).
pub fn size_floor(
    img: &RgbaImage,
    s: &OptimizeSettings,
    ctx: &ExportContext,
    ex: &EncodeExtras,
) -> Result<Option<usize>> {
    let mut cur = s.clone();
    match (s.format, effective_color_mode(s)) {
        (_, ColorMode::Indexed) if s.format != ExportFormat::Wbmp => {
            if cur.palette_kind == PaletteKind::Exact {
                cur.palette_kind = PaletteKind::Adaptive;
            }
            if cur.palette_kind == PaletteKind::Uniform && cur.grid_levels.pow(3) > 125 {
                cur.palette_kind = PaletteKind::Web216;
            }
            cur.max_colors = 2;
            if let Dither::Diffusion(p) = cur.dither {
                cur.dither = Dither::Diffusion((p - 50).max(30).min(p));
            }
        }
        (ExportFormat::Jpeg, _) => {
            cur.jpeg.quality = 2;
            cur.jpeg.smoothing = (s.jpeg.smoothing + 4).clamp(0, 7);
        }
        _ => return Ok(None),
    }
    Ok(Some(export_image(img, &cur, ctx, ex)?.bytes.len()))
}

/// Sucht Einstellungen mit Dateigröße nahe `target` Byte.
pub fn optimize_to_size(
    img: &RgbaImage,
    s: &OptimizeSettings,
    target: usize,
    ctx: &ExportContext,
    ex: &EncodeExtras,
) -> Result<SizeResult> {
    let size_of = |st: &OptimizeSettings| -> Result<(usize, usize)> {
        let r = export_image(img, st, ctx, ex)?;
        let colors = match &r.optimized {
            Optimized::Indexed(ix) => ix.palette.len(),
            _ => 0,
        };
        Ok((r.bytes.len(), colors))
    };
    let mut cur = s.clone();
    let (mut size, colors) = size_of(&cur)?;
    match (s.format, effective_color_mode(s)) {
        (_, ColorMode::Indexed) if s.format != ExportFormat::Wbmp => {
            if cur.palette_kind == PaletteKind::Exact {
                cur.palette_kind = PaletteKind::Adaptive;
            }
            if cur.palette_kind == PaletteKind::Uniform && cur.grid_levels.pow(3) > 125 {
                cur.palette_kind = PaletteKind::Web216;
            }
            let ia = i32::from(cur.transparency == crate::options::Transparency::IndexAlpha);
            let n_now = colors as i32 + ia;
            let p = match cur.dither {
                Dither::Diffusion(p) => p,
                _ => 0,
            };
            let diffusion = matches!(cur.dither, Dither::Diffusion(_));
            let ((mut lo, mut hi), (mut dlo, mut dhi)) = if size < target {
                ((n_now, 256), (p, (p + 45).min(100)))
            } else {
                ((2, n_now), ((p - 50).max(30).min(p), p))
            };
            loop {
                let m = (lo + hi) / 2;
                if m <= lo {
                    break;
                }
                cur.max_colors = m;
                if diffusion {
                    cur.dither = Dither::Diffusion((dlo + dhi) / 2);
                }
                let (sz, cols) = size_of(&cur)?;
                size = sz;
                let m = if (cols as i32 + ia) < m {
                    hi = cols as i32 + ia;
                    hi
                } else {
                    m
                };
                if m <= lo {
                    break;
                }
                if size > target {
                    hi = m;
                    dhi = (dlo + dhi) / 2;
                } else {
                    lo = m;
                    dlo = (dlo + dhi) / 2;
                }
            }
            Ok(SizeResult {
                settings: cur,
                bytes: size,
                changed: true,
            })
        }
        (ExportFormat::Jpeg, _) => {
            let (q, sm) = (cur.jpeg.quality, cur.jpeg.smoothing);
            let grow = size < target;
            let (mut lo, mut hi) = if grow { (q, 100) } else { (2, q) };
            let smooth_at = |qq: i32| -> i32 {
                if grow {
                    // s bei q … 0 bei 100
                    (sm as f64 * (100 - qq) as f64 / (100 - q).max(1) as f64).round() as i32
                } else {
                    // s+4 bei 2 … s bei q
                    (sm as f64 + 4.0 * (q - qq) as f64 / (q - 2).max(1) as f64).round() as i32
                }
            };
            loop {
                let m = (lo + hi) / 2;
                if m == lo {
                    break;
                }
                cur.jpeg.quality = m;
                cur.jpeg.smoothing = smooth_at(m).clamp(0, 7);
                let (sz, _) = size_of(&cur)?;
                if sz == 0 {
                    break;
                }
                size = sz;
                if size > target {
                    hi = m;
                } else {
                    lo = m;
                }
            }
            Ok(SizeResult {
                settings: cur,
                bytes: size,
                changed: true,
            })
        }
        _ => Ok(SizeResult {
            settings: cur,
            bytes: size,
            changed: false,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bisection_reduces_size() {
        let r = fp_geom::IRect::new(0, 0, 48, 32);
        let data: Vec<[u8; 4]> = (0..48 * 32)
            .map(|i| {
                [
                    (i * 5 % 256) as u8,
                    (i * 11 % 256) as u8,
                    (i % 48 * 5) as u8,
                    255,
                ]
            })
            .collect();
        let img = RgbaImage::from_raw(r, data).unwrap();
        let ctx = ExportContext::default();
        let ex = EncodeExtras::standalone();
        let s = OptimizeSettings::default();
        let full = export_image(&img, &s, &ctx, &ex).unwrap().bytes.len();
        let res = optimize_to_size(&img, &s, full / 2, &ctx, &ex).unwrap();
        assert!(res.changed);
        assert!(res.settings.max_colors < 256);
        assert!(res.bytes < full);
        let j = OptimizeSettings {
            format: ExportFormat::Jpeg,
            color_mode: ColorMode::Rgb24,
            ..OptimizeSettings::default()
        };
        let jfull = export_image(&img, &j, &ctx, &ex).unwrap().bytes.len();
        let jr = optimize_to_size(&img, &j, jfull * 2 / 3, &ctx, &ex).unwrap();
        assert!(jr.settings.jpeg.quality < 80);
        let p = OptimizeSettings {
            format: ExportFormat::Png,
            color_mode: ColorMode::Rgba32,
            ..OptimizeSettings::default()
        };
        assert!(!optimize_to_size(&img, &p, 100, &ctx, &ex).unwrap().changed);
    }

    #[test]
    fn size_floor_is_the_smallest_reachable_size() {
        let r = fp_geom::IRect::new(0, 0, 64, 48);
        let data: Vec<[u8; 4]> = (0..64 * 48)
            .map(|i| {
                [
                    (i * 7 % 256) as u8,
                    (i * 13 % 256) as u8,
                    (i % 64 * 4) as u8,
                    255,
                ]
            })
            .collect();
        let img = RgbaImage::from_raw(r, data).unwrap();
        let ctx = ExportContext::default();
        let ex = EncodeExtras::standalone();
        let s = OptimizeSettings::default();
        let full = export_image(&img, &s, &ctx, &ex).unwrap().bytes.len();
        let floor = size_floor(&img, &s, &ctx, &ex)
            .unwrap()
            .expect("GIF hat eine Suche");
        assert!(floor < full, "{floor} < {full}");
        // jede Zwischenstufe der Suche ist mindestens so groß wie die Untergrenze
        let res = optimize_to_size(&img, &s, floor + (full - floor) / 3, &ctx, &ex).unwrap();
        assert!(res.bytes >= floor && res.settings.max_colors > 2, "{res:?}");
        let j = OptimizeSettings {
            format: ExportFormat::Jpeg,
            color_mode: ColorMode::Rgb24,
            ..OptimizeSettings::default()
        };
        let jf = size_floor(&img, &j, &ctx, &ex)
            .unwrap()
            .expect("JPEG hat eine Suche");
        assert!(jf < export_image(&img, &j, &ctx, &ex).unwrap().bytes.len());
        let p = OptimizeSettings {
            format: ExportFormat::Png,
            color_mode: ColorMode::Rgb24,
            ..OptimizeSettings::default()
        };
        assert_eq!(
            size_floor(&img, &p, &ctx, &ex).unwrap(),
            None,
            "PNG 24 ohne Suche"
        );
        let w = OptimizeSettings {
            format: ExportFormat::Wbmp,
            ..OptimizeSettings::default()
        };
        assert_eq!(
            size_floor(&img, &w, &ctx, &ex).unwrap(),
            None,
            "WBMP ohne Suche"
        );
    }
}
