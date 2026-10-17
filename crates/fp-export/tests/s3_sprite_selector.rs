//! S-3 (re 28 E6, BA-034): selector of a CSS sprite = "." + prefix + name + suffix, measured
//! `.Psp_fw_r7_c9S` – no separator "_" between prefix/suffix and the name; dots become "_" and the
//! export reports modified selector names.

use fp_export::sprites::{SpriteLayout, SpriteOptions, pack_sprites_with, sprite_selector};
use fp_geom::IRect;
use fp_raster::RgbaImage;

fn part(w: i32, h: i32) -> RgbaImage {
    RgbaImage::new(IRect::new(0, 0, w, h)).unwrap_or_else(|_| panic!("image"))
}

#[test]
fn prefix_and_suffix_without_separator() {
    let o = SpriteOptions {
        prefix: "P".into(),
        suffix: "S".into(),
        padding: 10,
        layout: SpriteLayout::Horizontal,
        ..SpriteOptions::default()
    };
    assert_eq!(sprite_selector("sp.fw_r7_c9", &o), ".Psp_fw_r7_c9S");
    let (_, css, modified) = pack_sprites_with(
        &[("sp.fw_r7_c9".into(), part(160, 180))],
        "sp.fw",
        "sp.fw.png",
        &o,
    );
    assert!(
        css.contains(
            ".Psp_fw_r7_c9S{ width:160px; height:180px; background-position:-10px -10px; }"
        ),
        "{css}"
    );
    assert!(
        css.starts_with(".sp_fw{ background:url(\"sp.fw.png\") top left no-repeat; }"),
        "{css}"
    );
    assert!(
        modified,
        "dot in the name: message about modified selector names"
    );
}

#[test]
fn empty_prefix_and_suffix_keep_the_name() {
    let o = SpriteOptions::default();
    assert_eq!(sprite_selector("sp_fw_r2_c2", &o), ".sp_fw_r2_c2");
    let id = SpriteOptions {
        selector_is_class: false,
        prefix: "a-".into(),
        ..SpriteOptions::default()
    };
    assert_eq!(sprite_selector("x", &id), "#a-x");
}
