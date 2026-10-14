//! Q-11: mitgelieferte Vorlagen für „Neu von Vorlage“ – fünf Unterordner mit je zwei FW-PNG,
//! Namen in der Sprache der Oberfläche, einmalig angelegt (gelöschte kommen nicht wieder), jede
//! Datei lässt sich öffnen und hat die versprochenen Maße.

use fp_ui::builtin_templates::{self, MARKER};
use std::path::{Path, PathBuf};

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join("firepixel-q11").join(std::process::id().to_string()).join(name);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("Ordner");
    d
}

fn ensure(d: &Path) -> Vec<PathBuf> {
    let s = fp_engine::Session::default();
    builtin_templates::ensure(d, &|doc, p| fp_engine::commands::save_document(&s, doc, 0, p).map_err(|e| e.to_string()))
}

#[test]
fn q11_vorlagen_deutsch_einmalig_und_lesbar() {
    assert!(fp_i18n::set_language("de"));
    let d = dir("de");
    let made = ensure(&d);
    assert_eq!(made.len(), builtin_templates::all().len(), "alle Vorlagen angelegt: {made:?}");
    for f in ["Dokumentvorgaben", "Raster", "Mobil", "Web", "Drahtmodelle"] {
        let n = std::fs::read_dir(d.join(f)).map(|r| r.count()).unwrap_or(0);
        assert!((2..=3).contains(&n), "Ordner {f}: {n} Dateien");
    }
    assert!(d.join("Web").join("Web-Banner 728x90.fw.png").is_file());
    assert!(d.join("Mobil").join("App-Symbol 1024x1024.fw.png").is_file());
    for (b, p) in builtin_templates::all().iter().zip(&made) {
        let doc = fp_engine::commands::load_document(p).expect("Vorlage lesbar");
        let page = &doc.pages[0];
        assert_eq!((page.canvas.width, page.canvas.height), (b.width, b.height), "{}", p.display());
    }
    // Reihenfolge: Hintergrund des Banners unten (letztes Element), Knopf darüber sichtbar
    let banner = fp_engine::commands::load_document(&made[6]).expect("Banner");
    let r = fp_render::Renderer::default().render_page(&banner, 0, 0).image;
    let px = r.get(600, 30).expect("Pixel");
    let bg = r.get(5, 5).expect("Pixel");
    assert_ne!(px, bg, "Knopf liegt über dem Hintergrund");
    assert!(d.join(MARKER).is_file());
    // zweites Öffnen: nichts Neues; gelöschte Vorlage kommt nicht wieder
    std::fs::remove_file(d.join("Web").join("Web-Banner 728x90.fw.png")).expect("löschen");
    assert!(ensure(&d).is_empty(), "nur einmal anlegen");
    assert!(!d.join("Web").join("Web-Banner 728x90.fw.png").exists(), "gelöschte Vorlage bleibt weg");
}

#[test]
fn q11_vorlagen_englisch() {
    assert!(fp_i18n::set_language("en"));
    let d = dir("en");
    let made = ensure(&d);
    assert_eq!(made.len(), builtin_templates::all().len());
    for f in ["Document Presets", "Grids", "Mobile", "Web", "Wireframes"] {
        assert!(d.join(f).is_dir(), "Ordner {f}");
    }
    assert!(d.join("Grids").join("12 Columns 1200x800.fw.png").is_file());
    assert!(fp_i18n::set_language("de"));
}
