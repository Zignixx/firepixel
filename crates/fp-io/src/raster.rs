//! Zwischenformat der Bild-Dekoder: RGBA8-Bild mit Metadaten bzw. Animation.

use fp_geom::IRect;
use fp_raster::RgbaImage;

/// Ein dekodiertes Einzelbild.
#[derive(Clone, Debug, PartialEq)]
pub struct RasterImage {
    /// Pixel (nicht vormultipliziert), Ursprung (0, 0).
    pub image: RgbaImage,
    /// Auflösung in Pixel je Zoll, falls die Datei sie angibt.
    pub dpi: Option<f32>,
    /// Palette der Datei (RGB), z. B. für die Exporteinstellung „Exakt“/GIF-Palette.
    pub palette: Option<Vec<[u8; 3]>>,
    /// Die Datei enthält Transparenz (Alphakanal oder Transparenzindex).
    pub has_transparency: bool,
    /// Name der Ebene, wie das Original sie anlegt (z. B. „Hintergrund“).
    pub layer_name: Option<String>,
}

impl RasterImage {
    /// Aus RGBA-Bytes (`w·h·4`).
    pub fn from_rgba(w: u32, h: u32, rgba: Vec<u8>) -> crate::Result<RasterImage> {
        crate::check_size(w, h)?;
        let need = w as usize * h as usize * 4;
        if rgba.len() < need {
            return Err(crate::ImportError::Corrupt(format!("Pixeldaten zu kurz ({} < {need})", rgba.len())));
        }
        let has_transparency = rgba[..need].as_chunks::<4>().0.iter().any(|p| p[3] != 255);
        let image = RgbaImage::from_rgba_bytes(IRect::new(0, 0, w as i32, h as i32), &rgba[..need]).map_err(|e| crate::ImportError::Corrupt(e.to_string()))?;
        Ok(RasterImage { image, dpi: None, palette: None, has_transparency, layer_name: None })
    }

    pub fn width(&self) -> u32 {
        self.image.width() as u32
    }

    pub fn height(&self) -> u32 {
        self.image.height() as u32
    }
}

/// Ein Einzelbild einer Animation.
#[derive(Clone, Debug, PartialEq)]
pub struct AnimationFrame {
    /// Pixel des Einzelbildes an `(x, y)` der Leinwand.
    pub image: RgbaImage,
    pub x: i32,
    pub y: i32,
    /// Verzögerung in 1/100 s.
    pub delay_cs: u32,
    /// Löschmethode wie GIF (0 nicht angegeben, 1 nicht entfernen, 2 Hintergrund, 3 vorheriges).
    pub disposal: u8,
    /// Das Einzelbild ersetzt die Leinwand in seinem Rechteck, statt darüber gemalt zu werden
    /// (APNG/WebP/JPEG XL liefern bereits zusammengesetzte Vollbilder; GIF: `false`).
    pub replace: bool,
}

/// Animation (GIF, APNG, WebP, JPEG XL).
#[derive(Clone, Debug, PartialEq)]
pub struct Animation {
    pub width: u32,
    pub height: u32,
    pub frames: Vec<AnimationFrame>,
    /// Schleife wie `FLPi`: 0 endlos, −1 keine Schleife, n > 0 Wiederholungen.
    pub loop_count: i32,
    pub has_transparency: bool,
    pub palette: Option<Vec<[u8; 3]>>,
}

impl Animation {
    /// Setzt alle Einzelbilder zu vollen Leinwandbildern zusammen (Löschmethoden beachtet).
    pub fn composed_frames(&self) -> Vec<RgbaImage> {
        let r = IRect::new(0, 0, self.width as i32, self.height as i32);
        let Ok(mut canvas) = RgbaImage::new(r) else { return Vec::new() };
        let mut out = Vec::with_capacity(self.frames.len());
        for f in &self.frames {
            let before = canvas.clone();
            let placed = f.image.translated(f.x - f.image.bounds().left, f.y - f.image.bounds().top);
            if f.replace {
                canvas.fill_rect(placed.bounds(), fp_color::Rgba8::TRANSPARENT);
                canvas.paste(&placed);
            } else {
                fp_raster::composite(&mut canvas, &placed, None, &fp_raster::CompositeOptions::scalar());
            }
            out.push(canvas.clone());
            match f.disposal {
                2 => canvas.fill_rect(placed.bounds(), fp_color::Rgba8::TRANSPARENT),
                3 => canvas = before,
                _ => {}
            }
        }
        out
    }
}
