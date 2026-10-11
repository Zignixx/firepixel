//! HEIC/HEIF lesen über `heic-decoder` (reines Rust, `forbid(unsafe_code)`, Ebenen bitgleich zum
//! HEVC-Referenzdekoder HM): Standbild, iPhone-Raster, Alpha-Hilfsbild, `clap`/`irot`/`imir`,
//! 8/10/12 Bit, 4:0:0/4:2:0/4:2:2/4:4:4. RGB-Umrechnung nach `colr`/nclx bzw. SPS-VUI
//! (Rückfall BT.709 begrenzt), Chroma bilinear. Vormultipliziertes Alpha wird zurückgerechnet.
//! HEIF-Dateien mit AV1-Inhalt gehen an [`crate::avif_in`]. Das ICC-Profil (z. B. Display P3 bei
//! iPhone-Fotos) wird wie bei den übrigen Importen nicht angewandt.

use std::panic::{catch_unwind, AssertUnwindSafe};

use crate::raster::RasterImage;
use crate::{check_size, ImportError, Result};

fn derr(e: heic_decoder::HeifError) -> ImportError {
    ImportError::Decode("HEIC", e.to_string())
}

/// HEIC/HEIF-Standbild lesen.
pub fn decode_heic(d: &[u8]) -> Result<RasterImage> {
    let run = || -> Result<RasterImage> {
        let info = heic_decoder::probe(d).map_err(derr)?;
        let (w, h) = (u32::try_from(info.width).unwrap_or(u32::MAX), u32::try_from(info.height).unwrap_or(u32::MAX));
        check_size(w, h)?;
        let img = heic_decoder::decode(d).map_err(derr)?;
        let rgba = img.to_rgba8().map_err(derr)?;
        let (w, h) = (rgba.width as u32, rgba.height as u32);
        let mut data = rgba.data;
        if rgba.premultiplied {
            for p in data.as_chunks_mut::<4>().0 {
                let a = p[3] as u32;
                if a > 0 && a < 255 {
                    for c in &mut p[..3] {
                        *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
                    }
                }
            }
        }
        RasterImage::from_rgba(w, h, data)
    };
    catch_unwind(AssertUnwindSafe(run)).unwrap_or_else(|_| Err(ImportError::Decode("HEIC", "HEVC-Dekoder abgebrochen (beschädigte Daten)".into())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn garbage_is_error() {
        assert!(decode_heic(b"\0\0\0\x18ftypheic\0\0\0\0mif1heic").is_err());
        assert!(decode_heic(&[]).is_err());
    }
}
