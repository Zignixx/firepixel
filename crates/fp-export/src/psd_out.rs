//! Photoshop-Export (12 §3.10, 08 §5.4 „Photoshop PSD“): RGB 8 Bit, eine Pixelebene je
//! Original-Ebene (Name, Deckkraft, Mischmodus, Sichtbarkeit), Composite mit Alpha, Auflösung
//! (Ressource 1005) und Hilfslinien (1032).
//!
//! Abweichung (A): Das Original schreibt über die PSL-Bibliothek bearbeitbare Texte, Vektormasken und
//! Ebeneneffekte; hier werden alle Ebenen gerastert (Live-Effekte eingerechnet, so wie der Renderer
//! sie liefert). Unterebenen werden in ihre Ebene eingerechnet.

use crate::formats::packbits;
use crate::{ExportError, Result};
use fp_raster::RgbaImage;

/// Mischmodus des Originals (`BLDi`) → PSD-Blend-Key (Umkehrung von 12 §3.5).
pub fn blend_key(bldi: u8) -> &'static [u8; 4] {
    match bldi {
        2 => b"mul ",
        3 => b"diff",
        4 => b"over",
        5 => b"scrn",
        7 => b"diss",
        8 => b"lbrn",
        9 => b"dark",
        10 => b"lite",
        11 => b"hue ",
        12 => b"sat ",
        13 => b"colr",
        14 => b"lum ",
        18 => b"lddg",
        19 => b"vLit",
        20 => b"lLit",
        21 => b"pLit",
        22 => b"hMix",
        32 => b"smud",
        33 => b"hLit",
        34 => b"sLit",
        36 => b"div ",
        37 => b"idiv",
        _ => b"norm",
    }
}

/// Eine PSD-Ebene.
#[derive(Clone, Debug)]
pub struct PsdLayer {
    pub name: String,
    /// Pixel (Dokumentkoordinaten; leere Bilder ergeben eine leere Ebene).
    pub image: RgbaImage,
    pub opacity: u8,
    pub blend: u8,
    pub visible: bool,
}

/// Hilfslinie (Position in px, vertikal?).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PsdGuide {
    pub pos: f32,
    pub vertical: bool,
}

fn rle_channel(rows: &[Vec<u8>]) -> (Vec<u8>, Vec<u16>) {
    let mut data = Vec::new();
    let mut counts = Vec::with_capacity(rows.len());
    for r in rows {
        let p = packbits(r);
        counts.push(p.len().min(65535) as u16);
        data.extend_from_slice(&p);
    }
    (data, counts)
}

fn channel_rows(img: &RgbaImage, ch: usize) -> Vec<Vec<u8>> {
    (0..img.height())
        .map(|y| img.row_local(y).iter().map(|p| p[ch]).collect())
        .collect()
}

fn pascal_name(name: &str) -> Vec<u8> {
    let mut b: Vec<u8> = name
        .chars()
        .map(|c| if (c as u32) < 128 { c as u8 } else { b'_' })
        .take(255)
        .collect();
    b.insert(0, b.len() as u8);
    while !b.len().is_multiple_of(4) {
        b.push(0);
    }
    b
}

/// Schreibt eine PSD-Datei (Ebenen unten → oben).
pub fn write_psd(
    width: usize,
    height: usize,
    layers: &[PsdLayer],
    composite: &RgbaImage,
    dpi: f32,
    guides: &[PsdGuide],
) -> Result<Vec<u8>> {
    if width == 0 || height == 0 || width > 30000 || height > 30000 {
        return Err(ExportError::BadSize(width, height));
    }
    let mut out = Vec::new();
    out.extend_from_slice(b"8BPS");
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&[0u8; 6]);
    out.extend_from_slice(&4u16.to_be_bytes());
    out.extend_from_slice(&(height as u32).to_be_bytes());
    out.extend_from_slice(&(width as u32).to_be_bytes());
    out.extend_from_slice(&8u16.to_be_bytes());
    out.extend_from_slice(&3u16.to_be_bytes());
    out.extend_from_slice(&0u32.to_be_bytes());
    // Bildressourcen
    let mut res = Vec::new();
    let mut add_res = |id: u16, data: &[u8]| {
        res.extend_from_slice(b"8BIM");
        res.extend_from_slice(&id.to_be_bytes());
        res.extend_from_slice(&[0, 0]);
        res.extend_from_slice(&(data.len() as u32).to_be_bytes());
        res.extend_from_slice(data);
        if data.len() % 2 == 1 {
            res.push(0);
        }
    };
    let fixed = ((if dpi.is_finite() && dpi > 0.0 {
        dpi
    } else {
        72.0
    }) as f64
        * 65536.0)
        .round() as u32;
    let mut r1005 = Vec::new();
    for _ in 0..2 {
        r1005.extend_from_slice(&fixed.to_be_bytes());
        r1005.extend_from_slice(&1u16.to_be_bytes());
        r1005.extend_from_slice(&1u16.to_be_bytes());
    }
    add_res(1005, &r1005);
    if !guides.is_empty() {
        let mut g = Vec::new();
        g.extend_from_slice(&1u32.to_be_bytes());
        g.extend_from_slice(&0u32.to_be_bytes());
        g.extend_from_slice(&0u32.to_be_bytes());
        g.extend_from_slice(&(guides.len() as u32).to_be_bytes());
        for gd in guides {
            g.extend_from_slice(&((gd.pos * 32.0).round() as i32).to_be_bytes());
            g.push(if gd.vertical { 0 } else { 1 });
        }
        add_res(1032, &g);
    }
    out.extend_from_slice(&(res.len() as u32).to_be_bytes());
    out.extend_from_slice(&res);
    // Ebenen
    let mut recs = Vec::new();
    let mut chdata = Vec::new();
    for l in layers {
        let b = l.image.bounds();
        let (t, le, bo, ri) = if l.image.is_empty() {
            (0, 0, 0, 0)
        } else {
            (b.top, b.left, b.bottom, b.right)
        };
        for v in [t, le, bo, ri] {
            recs.extend_from_slice(&v.to_be_bytes());
        }
        recs.extend_from_slice(&4u16.to_be_bytes());
        let mut chans = Vec::new();
        for (id, ch) in [(-1i16, 3usize), (0, 0), (1, 1), (2, 2)] {
            let rows = channel_rows(&l.image, ch);
            let (data, counts) = rle_channel(&rows);
            let mut c = Vec::new();
            c.extend_from_slice(&1u16.to_be_bytes());
            for n in &counts {
                c.extend_from_slice(&n.to_be_bytes());
            }
            c.extend_from_slice(&data);
            recs.extend_from_slice(&id.to_be_bytes());
            recs.extend_from_slice(&(c.len() as u32).to_be_bytes());
            chans.push(c);
        }
        recs.extend_from_slice(b"8BIM");
        recs.extend_from_slice(blend_key(l.blend));
        recs.push(l.opacity);
        recs.push(0);
        recs.push(if l.visible { 0 } else { 2 });
        recs.push(0);
        let mut extra = Vec::new();
        extra.extend_from_slice(&0u32.to_be_bytes());
        extra.extend_from_slice(&0u32.to_be_bytes());
        extra.extend_from_slice(&pascal_name(&l.name));
        // luni (Unicode-Name)
        let u: Vec<u16> = l.name.encode_utf16().collect();
        let mut luni = Vec::new();
        luni.extend_from_slice(&(u.len() as u32).to_be_bytes());
        for c in &u {
            luni.extend_from_slice(&c.to_be_bytes());
        }
        while luni.len() % 4 != 0 {
            luni.push(0);
        }
        extra.extend_from_slice(b"8BIMluni");
        extra.extend_from_slice(&(luni.len() as u32).to_be_bytes());
        extra.extend_from_slice(&luni);
        recs.extend_from_slice(&(extra.len() as u32).to_be_bytes());
        recs.extend_from_slice(&extra);
        for c in chans {
            chdata.extend_from_slice(&c);
        }
    }
    let mut layer_info = Vec::new();
    layer_info.extend_from_slice(&(-(layers.len() as i16)).to_be_bytes());
    layer_info.extend_from_slice(&recs);
    layer_info.extend_from_slice(&chdata);
    if layer_info.len() % 2 == 1 {
        layer_info.push(0);
    }
    let mut lm = Vec::new();
    lm.extend_from_slice(&(layer_info.len() as u32).to_be_bytes());
    lm.extend_from_slice(&layer_info);
    lm.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&(lm.len() as u32).to_be_bytes());
    out.extend_from_slice(&lm);
    // Composite (RLE, R G B A; Farben gegen Weiß mattiert wie Photoshop)
    let ext = composite.extended_to(fp_geom::IRect::new(
        composite.bounds().left,
        composite.bounds().top,
        composite.bounds().left + width as i32,
        composite.bounds().top + height as i32,
    ));
    let comp = ext.unwrap_or_else(|_| composite.clone());
    let mut planes: Vec<Vec<Vec<u8>>> = Vec::new();
    for ch in 0..4 {
        let rows = (0..comp.height().min(height))
            .map(|y| {
                comp.row_local(y)
                    .iter()
                    .take(width)
                    .map(|p| {
                        if ch == 3 {
                            p[3]
                        } else {
                            let a = p[3] as u32;
                            ((p[ch] as u32 * a + 255 * (255 - a) + 127) / 255) as u8
                        }
                    })
                    .collect()
            })
            .collect();
        planes.push(rows);
    }
    out.extend_from_slice(&1u16.to_be_bytes());
    let mut counts = Vec::new();
    let mut data = Vec::new();
    for p in &planes {
        let (d, c) = rle_channel(p);
        counts.extend(c);
        data.extend(d);
    }
    for c in counts {
        out.extend_from_slice(&c.to_be_bytes());
    }
    out.extend_from_slice(&data);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn psd_header_and_layers() {
        let r = fp_geom::IRect::new(1, 1, 3, 3);
        let img = RgbaImage::filled(r, fp_color::Rgba8::new(255, 0, 0, 255)).unwrap();
        let comp = RgbaImage::filled(
            fp_geom::IRect::new(0, 0, 4, 4),
            fp_color::Rgba8::new(255, 255, 255, 255),
        )
        .unwrap();
        let l = PsdLayer {
            name: "Rot".into(),
            image: img,
            opacity: 255,
            blend: 0,
            visible: true,
        };
        let psd = write_psd(
            4,
            4,
            &[l],
            &comp,
            72.0,
            &[PsdGuide {
                pos: 2.0,
                vertical: true,
            }],
        )
        .unwrap();
        assert_eq!(&psd[..4], b"8BPS");
        assert_eq!(u16::from_be_bytes([psd[12], psd[13]]), 4);
        assert_eq!(blend_key(5), b"scrn");
    }
}
