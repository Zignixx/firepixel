//! PNG-Kodierung und Base64 für Bildinhalte (MCP `image`-Blöcke, CLI `render`).

use fp_raster::RgbaImage;

/// Kodiert ein RGBA8-Bild (nicht vormultipliziert) als PNG.
pub fn encode_png(img: &RgbaImage) -> Result<Vec<u8>, String> {
    encode_png_raw(img.width() as u32, img.height() as u32, img.as_bytes())
}

/// Kodiert rohe RGBA8-Daten als PNG.
pub fn encode_png_raw(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    if width == 0 || height == 0 {
        return Err(fp_i18n::t("err-mcp-image-empty"));
    }
    if rgba.len() != width as usize * height as usize * 4 {
        return Err(fp_i18n::t("err-mcp-image-size"));
    }
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, width, height);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().map_err(|e| e.to_string())?;
        w.write_image_data(rgba).map_err(|e| e.to_string())?;
        w.finish().map_err(|e| e.to_string())?;
    }
    Ok(out)
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Base64 (Standardalphabet mit Auffüllung).
pub fn base64(data: &[u8]) -> String {
    let mut s = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [chunk[0], chunk.get(1).copied().unwrap_or(0), chunk.get(2).copied().unwrap_or(0)];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        s.push(B64[(n >> 18) as usize & 63] as char);
        s.push(B64[(n >> 12) as usize & 63] as char);
        s.push(if chunk.len() > 1 { B64[(n >> 6) as usize & 63] as char } else { '=' });
        s.push(if chunk.len() > 2 { B64[n as usize & 63] as char } else { '=' });
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_rfc4648() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn png_signature() {
        let png = encode_png_raw(2, 1, &[255, 0, 0, 255, 0, 0, 255, 128]).unwrap_or_default();
        assert_eq!(&png[..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
        assert!(encode_png_raw(0, 1, &[]).is_err());
    }
}
