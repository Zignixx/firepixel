//! CSS-Sprites (`exportFilesStyle = 6`, 08 §5.4): Slice-Bilder untereinander in ein PNG packen und je
//! Slice eine CSS-Klasse mit `background-position` erzeugen (A: Packverfahren des Originals und
//! Klassennamen sind nicht spezifiziert; hier senkrecht in Rasterreihenfolge, Klasse = Zellname).

use fp_geom::IRect;
use fp_raster::RgbaImage;

/// Packt die Teile und liefert (Sprite-Bild, CSS).
pub fn pack_sprites(parts: &[(String, RgbaImage)], sprite_url: &str) -> (RgbaImage, String) {
    let w = parts.iter().map(|p| p.1.width()).max().unwrap_or(1).max(1);
    let h: usize = parts.iter().map(|p| p.1.height()).sum::<usize>().max(1);
    let mut img = RgbaImage::new(IRect::new(0, 0, w as i32, h as i32))
        .unwrap_or_else(|_| RgbaImage::empty_at(0, 0));
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
    let mut s: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
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
    /// „Raster“: nach Höhe absteigend in Regale der Breite max(B) + 2p gepackt (re 28 E6).
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
        SpriteOptions {
            selector_is_class: true,
            prefix: String::new(),
            suffix: String::new(),
            padding: 10,
            layout: SpriteLayout::Grid,
        }
    }
}

/// Selektor zu einem Segmentnamen: Präfix + Name + Suffix, bereinigt (`.`/`#` je nach Art).
pub fn sprite_selector(name: &str, o: &SpriteOptions) -> String {
    format!(
        "{}{}",
        if o.selector_is_class { '.' } else { '#' },
        css_ident(&format!("{}{name}{}", o.prefix, o.suffix))
    )
}

/// Sprite-Bild und CSS wie der Exportdialog (re\18 §3, gemessen): Teile untereinander mit
/// `padding` als Rand und Abstand; CSS zuerst `.<basis>{ background:url("<bild>") top left
/// no-repeat; }`, dann je Teil `.<name>{ width:…px; height:…px; background-position:-<p>px -<y>px; }`.
/// Liefert (Bild, CSS, Selektornamen wurden geändert).
pub fn pack_sprites_css(
    parts: &[(String, RgbaImage)],
    base: &str,
    image_url: &str,
    padding: usize,
) -> (RgbaImage, String, bool) {
    pack_sprites_with(
        parts,
        base,
        image_url,
        &SpriteOptions {
            padding,
            ..SpriteOptions::default()
        },
    )
}

/// Lage der Teile (Breite, Höhe je Teil) im Sprite-Bild wie im Original (re 28 E6, gemessen an
/// fünf Segmenten mit p = 10/11): waagerecht/senkrecht in der gegebenen Reihenfolge mit `p` als
/// Rand und Abstand; Raster nach Höhe absteigend (stabil), Regalpacken (erstes passendes Regal)
/// auf die Blattbreite max(B) + 2p: ein Teil kommt in das erste Regal, in dem rechts noch
/// Breite + p Platz ist, sonst in ein neues Regal darunter (Regalhöhe = erstes Teil + p).
/// Liefert (Bildbreite, Bildhöhe, Lage je Teil).
pub fn sprite_layout(
    sizes: &[(usize, usize)],
    padding: usize,
    layout: SpriteLayout,
) -> (usize, usize, Vec<(usize, usize)>) {
    let p = padding;
    let n = sizes.len();
    match layout {
        SpriteLayout::Horizontal => {
            let mut x = p;
            let pos = sizes
                .iter()
                .map(|s| {
                    let r = (x, p);
                    x += s.0 + p;
                    r
                })
                .collect();
            (
                sizes.iter().map(|s| s.0).sum::<usize>() + p * (n + 1),
                sizes.iter().map(|s| s.1).max().unwrap_or(0) + 2 * p,
                pos,
            )
        }
        SpriteLayout::Vertical => {
            let mut y = p;
            let pos = sizes
                .iter()
                .map(|s| {
                    let r = (p, y);
                    y += s.1 + p;
                    r
                })
                .collect();
            (
                sizes.iter().map(|s| s.0).max().unwrap_or(0) + 2 * p,
                sizes.iter().map(|s| s.1).sum::<usize>() + p * (n + 1),
                pos,
            )
        }
        SpriteLayout::Grid => {
            let sheet = sizes.iter().map(|s| s.0).max().unwrap_or(0) + 2 * p;
            let mut order: Vec<usize> = (0..n).collect();
            order.sort_by(|a, b| sizes[*b].1.cmp(&sizes[*a].1));
            // Regale: (oben, nächstes x)
            let mut shelves: Vec<(usize, usize)> = Vec::new();
            let mut pos = vec![(0, 0); n];
            let mut bottom = p;
            for i in order {
                let (w, h) = sizes[i];
                match shelves.iter_mut().find(|s| s.1 + w + p <= sheet) {
                    Some(s) => {
                        pos[i] = (s.1, s.0);
                        s.1 += w + p;
                    }
                    None => {
                        pos[i] = (p, bottom);
                        shelves.push((bottom, p + w + p));
                        bottom += h + p;
                    }
                }
            }
            (sheet, bottom, pos)
        }
    }
}

/// Wie [`pack_sprites_css`] mit allen Sprite-Optionen (Selektorart, Präfix, Suffix, Auffüllung,
/// Layout); Lage nach [`sprite_layout`].
pub fn pack_sprites_with(
    parts: &[(String, RgbaImage)],
    base: &str,
    image_url: &str,
    o: &SpriteOptions,
) -> (RgbaImage, String, bool) {
    let sizes: Vec<(usize, usize)> = parts.iter().map(|p| (p.1.width(), p.1.height())).collect();
    let (w, h, places) = sprite_layout(&sizes, o.padding, o.layout);
    let mut img = RgbaImage::new(IRect::new(0, 0, w.max(1) as i32, h.max(1) as i32))
        .unwrap_or_else(|_| RgbaImage::empty_at(0, 0));
    let c = if o.selector_is_class { '.' } else { '#' };
    let mut css = format!(
        "{c}{}{{ background:url(\"{image_url}\") top left no-repeat; }}\n",
        css_ident(base)
    );
    let mut used = std::collections::HashSet::new();
    let mut modified = false;
    for ((name, part), &(x, y)) in parts.iter().zip(&places) {
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
        let placed = part.translated(x as i32 - part.bounds().left, y as i32 - part.bounds().top);
        img.paste(&placed);
        css.push_str(&format!(
            "{c}{sel}{{ width:{}px; height:{}px; background-position:-{x}px -{y}px; }}\n",
            part.width(),
            part.height()
        ));
    }
    (img, css, modified)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_vertically() {
        let a = RgbaImage::filled(IRect::new(5, 5, 9, 7), fp_color::Rgba8::new(255, 0, 0, 255))
            .unwrap();
        let b = RgbaImage::filled(IRect::new(0, 0, 2, 3), fp_color::Rgba8::new(0, 0, 255, 255))
            .unwrap();
        let (img, css) = pack_sprites(&[("a".into(), a), ("b".into(), b)], "s.png");
        assert_eq!((img.width(), img.height()), (4, 5));
        assert_eq!(img.pixel(0, 2), fp_color::Rgba8::new(0, 0, 255, 255));
        assert!(css.contains(".b {\n\tbackground: url(\"s.png\") no-repeat 0 -2px;"));
    }
}
