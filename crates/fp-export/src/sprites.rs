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

/// Gültiger CSS-Klassenname (Buchstaben, Ziffern, `-`, `_`; nicht mit Ziffer beginnend).
pub fn css_ident(name: &str) -> String {
    let mut s: String = name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    if s.is_empty() || s.starts_with(|c: char| c.is_ascii_digit()) {
        s.insert(0, '_');
    }
    s
}

/// „Layout:“ der Sprite-Optionen (re\18 §4.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SpriteLayout {
    Horizontal,
    Vertical,
    /// „Raster“: gemessen lagen alle Bilder untereinander (re\18 §3, ? 10.15); wann das Original
    /// mehrspaltig wird, ist offen, darum wie „Vertikal“.
    #[default]
    Grid,
}

/// Exportoptionen für CSS-Sprites (re\18 §4.2), ohne die Vorgabe für die Optimierung.
#[derive(Clone, Debug, PartialEq)]
pub struct SpriteOptions {
    /// „CSS-Selektor:“ class(.) bzw. id(#).
    pub selector_is_class: bool,
    pub prefix: String,
    pub suffix: String,
    /// „Auffüllung:“ in px (Rand und Abstand).
    pub padding: usize,
    pub layout: SpriteLayout,
}

impl Default for SpriteOptions {
    fn default() -> Self {
        SpriteOptions { selector_is_class: true, prefix: String::new(), suffix: String::new(), padding: 10, layout: SpriteLayout::Grid }
    }
}

/// Selektor zu einem Segmentnamen: Präfix + Name + Suffix, bereinigt (`.`/`#` je nach Art).
pub fn sprite_selector(name: &str, o: &SpriteOptions) -> String {
    format!("{}{}", if o.selector_is_class { '.' } else { '#' }, css_ident(&format!("{}{name}{}", o.prefix, o.suffix)))
}

/// Sprite-Bild und CSS wie der Exportdialog (re\18 §3, gemessen): Teile untereinander mit
/// `padding` als Rand und Abstand; CSS zuerst `.<basis>{ background:url("<bild>") top left
/// no-repeat; }`, dann je Teil `.<name>{ width:…px; height:…px; background-position:-<p>px -<y>px; }`.
/// Liefert (Bild, CSS, Selektornamen wurden geändert).
pub fn pack_sprites_css(parts: &[(String, RgbaImage)], base: &str, image_url: &str, padding: usize) -> (RgbaImage, String, bool) {
    pack_sprites_with(parts, base, image_url, &SpriteOptions { padding, ..SpriteOptions::default() })
}

/// Wie [`pack_sprites_css`] mit allen Sprite-Optionen (Selektorart, Präfix, Suffix, Auffüllung,
/// Layout). Waagerecht liegen die Teile nebeneinander, sonst untereinander.
pub fn pack_sprites_with(parts: &[(String, RgbaImage)], base: &str, image_url: &str, o: &SpriteOptions) -> (RgbaImage, String, bool) {
    let padding = o.padding;
    let across = o.layout == SpriteLayout::Horizontal;
    let n = parts.len();
    let (w, h) = if across {
        (parts.iter().map(|p| p.1.width()).sum::<usize>() + padding * (n + 1), parts.iter().map(|p| p.1.height()).max().unwrap_or(0) + 2 * padding)
    } else {
        (parts.iter().map(|p| p.1.width()).max().unwrap_or(0) + 2 * padding, parts.iter().map(|p| p.1.height()).sum::<usize>() + padding * (n + 1))
    };
    let mut img = RgbaImage::new(IRect::new(0, 0, w.max(1) as i32, h.max(1) as i32)).unwrap_or_else(|_| RgbaImage::empty_at(0, 0));
    let c = if o.selector_is_class { '.' } else { '#' };
    let mut css = format!("{c}{}{{ background:url(\"{image_url}\") top left no-repeat; }}\n", css_ident(base));
    let mut used = std::collections::HashSet::new();
    let mut modified = false;
    let mut pos = padding;
    for (name, part) in parts {
        let raw = format!("{}{name}{}", o.prefix, o.suffix);
        let mut sel = css_ident(&raw);
        if sel != raw {
            modified = true;
        }
        if !used.insert(sel.clone()) {
            modified = true;
            let mut k = 1;
            while !used.insert(format!("{sel}_{k}")) {
                k += 1;
            }
            sel = format!("{sel}_{k}");
        }
        let (x, y) = if across { (pos, padding) } else { (padding, pos) };
        let placed = part.translated(x as i32 - part.bounds().left, y as i32 - part.bounds().top);
        img.paste(&placed);
        css.push_str(&format!("{c}{sel}{{ width:{}px; height:{}px; background-position:-{x}px -{y}px; }}\n", part.width(), part.height()));
        pos += if across { part.width() } else { part.height() } + padding;
    }
    (img, css, modified)
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
