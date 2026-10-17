//! CT-C-18 (re 29 §3): „Status in Dateien“ mit „Bilder zuschneiden“ überspringt einen Status ohne
//! sichtbare Pixel (nur Webebene): keine Datei, die übrigen behalten ihre Statusnummer.

use fp_color::Argb;
use fp_export::doc_export::{DocExportOptions, FilesStyle, PanelOptions, export_document};
use fp_export::export::EncodeExtras;
use fp_export::render::RenderRequest;
use fp_geom::IRect;
use fp_model::Document;
use fp_raster::RgbaImage;

/// Status 1 (Index 0): Inhalt 0,0–380,280; Status 2: leer (durchsichtig gerendert).
fn render(_d: &Document, req: &RenderRequest) -> Result<RgbaImage, String> {
    let r = IRect::new(0, 0, 400, 300);
    let transparent = matches!(req.background, fp_export::render::Background::Transparent);
    let bytes: Vec<u8> = (0..300)
        .flat_map(|y| {
            (0..400).flat_map(move |x| {
                let inside = req.frame == 0 && x < 380 && y < 280;
                if inside {
                    [40, 80, 160, 255]
                } else if transparent {
                    [0, 0, 0, 0]
                } else {
                    [255, 255, 255, 255]
                }
            })
        })
        .collect();
    RgbaImage::from_rgba_bytes(r, &bytes).map_err(|e| e.to_string())
}

#[test]
fn ct_c18_empty_state_writes_no_file_when_trimming() {
    let mut d = Document::new(400.0, 300.0, Argb(0xffff_ffff));
    std::sync::Arc::make_mut(&mut d.pages[0]).root.frame_count = 2;
    let mut o = DocExportOptions::new("st");
    o.style = FilesStyle::FramesToFiles;
    o.panel = Some(PanelOptions::default());
    let files = export_document(&d, &render, &o, &EncodeExtras::default()).expect("Export");
    let names: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(names.len(), 1, "{names:?}");
    assert!(names[0].starts_with("s01."), "{names:?}");
    // ohne „Bilder zuschneiden“: beide Status (A: volle Leinwand)
    o.panel = Some(PanelOptions {
        trim_images: false,
        ..PanelOptions::default()
    });
    assert_eq!(
        export_document(&d, &render, &o, &EncodeExtras::default())
            .expect("Export")
            .len(),
        2
    );
}
