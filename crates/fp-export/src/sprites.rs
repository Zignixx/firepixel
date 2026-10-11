//! CSS-Sprites (`exportFilesStyle = 6`, 08 §5.4): Slice-Bilder untereinander in ein PNG packen und je
//! Slice eine CSS-Klasse mit `background-position` erzeugen (A: Packverfahren des Originals und
//! Klassennamen sind nicht spezifiziert; hier senkrecht in Rasterreihenfolge, Klasse = Zellname).

use fp_geom::IRect;
use fp_raster::RgbaImage;

/// Packt die Teile und liefert (Sprite-Bild, CSS).
pub fn pack_sprites(parts: &[(String, RgbaImage)], sprite_url: &str) -> (RgbaImage, String) {
    let w = parts.iter().map(|p| p.1.width()).max().unwrap_or(1).max(1);
    let h: usize = parts.iter().map(|p| p.1.height()).sum::<usize>().max(1);
    let mut img = RgbaImage::new(IRect::new(0, 0, w as i32, h as i32)).unwrap_or_else(|_| RgbaImage::empty_at(0, 0));
    let mut css = String::new();
    let mut y = 0usize;
    for (name, part) in parts {
        let placed = part.translated(-part.bounds().left, y as i32 - part.bounds().top);
        img.paste(&placed);
        css.push_str(&format!(
            ".{name} {{\n\tbackground: url(\"{sprite_url}\") no-repeat 0 {}px;\n\twidth: {}px;\n\theight: {}px;\n}}\n",
            -(y as i64),
            part.width(),
            part.height()
        ));
        y += part.height();
    }
    (img, css)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_vertically() {
        let a = RgbaImage::filled(IRect::new(5, 5, 9, 7), fp_color::Rgba8::new(255, 0, 0, 255)).unwrap();
        let b = RgbaImage::filled(IRect::new(0, 0, 2, 3), fp_color::Rgba8::new(0, 0, 255, 255)).unwrap();
        let (img, css) = pack_sprites(&[("a".into(), a), ("b".into(), b)], "s.png");
        assert_eq!((img.width(), img.height()), (4, 5));
        assert_eq!(img.pixel(0, 2), fp_color::Rgba8::new(0, 0, 255, 255));
        assert!(css.contains(".b {\n\tbackground: url(\"s.png\") no-repeat 0 -2px;"));
    }
}
