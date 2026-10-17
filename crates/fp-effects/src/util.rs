//! Gemeinsame Bausteine: Bilder auf einen Rahmen legen, Masken weichzeichnen, Pixel mischen.

use fp_color::Rgba8;
use fp_color::blend::{orig_over, orig_over_mmx};
use fp_geom::IRect;
use fp_raster::{Mask, RgbaImage};

/// Bild auf den Rahmen `frame` legen (außerhalb transparent `(0,0,0,0)`); bei unsinniger Größe ein
/// leeres Bild.
pub(crate) fn on_frame(img: &RgbaImage, frame: IRect) -> RgbaImage {
    img.copy_rect(frame)
        .unwrap_or_else(|_| RgbaImage::empty_at(frame.left, frame.top))
}

/// Maske auf den Rahmen `frame` legen (außerhalb 0).
pub(crate) fn mask_on(m: &Mask, frame: IRect) -> Mask {
    m.copy_rect(frame)
        .unwrap_or_else(|_| Mask::empty_at(frame.left, frame.top))
}

/// Leeres Bild mit `frame`.
pub(crate) fn new_image(frame: IRect) -> RgbaImage {
    RgbaImage::new(frame).unwrap_or_else(|_| RgbaImage::empty_at(frame.left, frame.top))
}

/// Dreiecksfilter (05 §2.2) mit Radius `r` in beiden Richtungen; `r = 0` liefert die Maske
/// unverändert (wie `fx.tent_blur`). `last_row_screen` = Eigenheit der letzten Zeile.
pub(crate) fn tent(m: &Mask, r: u32, last_row_screen: bool) -> Mask {
    if r == 0 {
        return m.clone();
    }
    m.tent_blur(r, r, last_row_screen)
        .unwrap_or_else(|_| m.clone())
}

/// Wie [`tent`], Ergebnis auf die Eingabegrenzen beschnitten (`fx._blur_same`).
pub(crate) fn tent_same(m: &Mask, r: u32) -> Mask {
    if r == 0 {
        return m.clone();
    }
    mask_on(&tent(m, r, false), m.bounds())
}

/// `round(a·b/255)` (`@008306a0`, identisch zu `(a·b+127)//255`).
#[inline]
pub(crate) fn mul(a: u8, b: u8) -> u8 {
    fp_color::round::mul255(a as u32, b as u32) as u8
}

/// `div255` der Effekte (`t = v+128; ((t>>8)+t)>>8`).
#[inline]
pub(crate) fn div255(v: u32) -> u32 {
    fp_color::round::div255(v)
}

/// Original-„over“ skalar bzw. MMX.
#[inline]
pub(crate) fn over(top: Rgba8, bottom: Rgba8, mmx: bool) -> Rgba8 {
    if mmx {
        orig_over_mmx(top, bottom)
    } else {
        orig_over(top, bottom)
    }
}

/// Pixel als `Rgba8`.
#[inline]
pub(crate) fn px(p: [u8; 4]) -> Rgba8 {
    Rgba8::from_array(p)
}

/// Voll transparente Pixel auf `(0,0,0,0)` setzen (wie `out[out[...,3]==0] = 0` der Referenz).
pub(crate) fn clear_transparent(img: &mut RgbaImage) {
    for p in img.data_mut() {
        if p[3] == 0 {
            *p = [0, 0, 0, 0];
        }
    }
}

/// Vereinigung zweier Rechtecke (leere zählen nicht).
pub(crate) fn union(a: IRect, b: IRect) -> IRect {
    if a.is_empty() {
        b
    } else if b.is_empty() {
        a
    } else {
        a.union(&b)
    }
}
