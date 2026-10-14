//! Q-11: mitgelieferte Vorlagen für „Neu von Vorlage“ – fünf Unterordner mit je zwei FW-PNG,
//! Ordner- und Dateinamen immer englisch (F7), einmalig angelegt (gelöschte kommen nicht wieder),
//! jede Datei lässt sich öffnen, hat die versprochenen Maße und eine passende Exportvorgabe.

use fp_ui::builtin_templates::{self, MARKER};
use std::path::{Path, PathBuf};

const FOLDERS: [&str; 5] = ["Document Presets", "Grids", "Mobile", "Web", "Wireframes"];

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
fn q11_vorlagen_deutsch_englische_namen_einmalig_und_lesbar() {
    assert!(fp_i18n::set_language("de"));
    let d = dir("de");
    let made = ensure(&d);
    assert_eq!(made.len(), builtin_templates::all().len(), "alle Vorlagen angelegt: {made:?}");
    for f in FOLDERS {
        let n = std::fs::read_dir(d.join(f)).map(|r| r.count()).unwrap_or(0);
        assert!((2..=3).contains(&n), "Ordner {f}: {n} Dateien");
    }
    for f in ["Dokumentvorgaben", "Raster", "Mobil", "Drahtmodelle"] {
        assert!(!d.join(f).exists(), "kein deutscher Ordner {f} in deutscher Oberfläche");
    }
    let banner_path = d.join("Web").join("Web Banner 728x90.fw.png");
    assert!(banner_path.is_file());
    assert!(d.join("Mobile").join("App Icon 1024x1024.fw.png").is_file());
    for (b, p) in builtin_templates::all().iter().zip(&made) {
        let doc = fp_engine::commands::load_document(p).expect("Vorlage lesbar");
        let page = &doc.pages[0];
        assert_eq!((page.canvas.width, page.canvas.height), (b.width, b.height), "{}", p.display());
        // F7: Exportvorgabe der Vorlage statt „GIF Adaptiv 256“
        let fmt = fp_engine::commands::export::format_name(&fp_export::doc_export::page_settings(&doc, 0));
        assert_eq!(fmt, b.export, "Exportvorgabe {}", p.display());
        assert_ne!(fmt, "gif");
    }
    // Reihenfolge: Hintergrund des Banners unten (letztes Element), Knopf darüber sichtbar
    let banner = fp_engine::commands::load_document(&made[6]).expect("Banner");
    let r = fp_render::Renderer::default().render_page(&banner, 0, 0).image;
    let px = r.get(600, 30).expect("Pixel");
    let bg = r.get(5, 5).expect("Pixel");
    assert_ne!(px, bg, "Knopf liegt über dem Hintergrund");
    assert!(d.join(MARKER).is_file());
    // zweites Öffnen: nichts Neues; gelöschte Vorlage kommt nicht wieder
    std::fs::remove_file(&banner_path).expect("löschen");
    assert!(ensure(&d).is_empty(), "nur einmal anlegen");
    assert!(!banner_path.exists(), "gelöschte Vorlage bleibt weg");
}

#[test]
fn q11_vorlagen_englisch() {
    assert!(fp_i18n::set_language("en"));
    let d = dir("en");
    let made = ensure(&d);
    assert_eq!(made.len(), builtin_templates::all().len());
    for f in FOLDERS {
        assert!(d.join(f).is_dir(), "Ordner {f}");
    }
    assert!(d.join("Grids").join("12 Columns 1200x800.fw.png").is_file());
    // „Neu von Vorlage“ übernimmt die Exportvorgabe der Vorlage (nicht GIF)
    let mut s = fp_engine::Session::default();
    let banner = d.join("Web").join("Web Banner 728x90.fw.png");
    s.execute("file.newFromTemplate", serde_json::json!({"path": banner.to_string_lossy()})).expect("Neu von Vorlage");
    let doc = &s.active().expect("Dokument").doc;
    assert_eq!(fp_engine::commands::export::format_name(&fp_export::doc_export::page_settings(doc, 0)), "png32");
    assert!(fp_i18n::set_language("de"));
}

/// Profil aus F5 mit deutschen Ordnern, aber ohne Merker: vorhandene Ordner weiterbenutzen, kein
/// zweiter englischer Ordner daneben.
#[test]
fn q11_deutsche_altordner_werden_nicht_gedoppelt() {
    let d = dir("legacy");
    for f in ["Dokumentvorgaben", "Raster", "Mobil", "Drahtmodelle"] {
        std::fs::create_dir_all(d.join(f)).expect("Altordner");
    }
    let made = ensure(&d);
    assert_eq!(made.len(), builtin_templates::all().len());
    for f in ["Document Presets", "Grids", "Mobile", "Wireframes"] {
        assert!(!d.join(f).exists(), "kein doppelter Ordner {f}");
    }
    assert!(d.join("Mobil").join("Smartphone 390x844.fw.png").is_file());
    assert!(d.join("Web").join("Social Media Post 1080x1080.fw.png").is_file());
}
