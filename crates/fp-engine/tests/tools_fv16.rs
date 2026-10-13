//! FV-16 (PLAN P-LW2): Werksvorgaben „Farbe ersetzen“ (PARITAET_A W8, `data\prefs.json`),
//! getrennte Optionen „Auswahl füllen“/Toleranz für Farbeimer und Verlauf, Verlauf auf Bitmaps
//! mit beiden Optionen, Texturen und Muster nur per Name aus den Ressourcenordnern.

use crate::common;
use common::*;
use fp_engine::prefs::{PrefValue, Preferences};
use fp_engine::tools::ToolOptions;
use fp_engine::{Session, ToolId};
use fp_model::{ElementId, ElementKind};
use serde_json::json;

fn px(s: &Session, id: ElementId, x: u32, y: u32) -> [u8; 4] {
    match &el(s, id).kind {
        ElementKind::Bitmap(b) => b.image.pixel(x, y).map(|c| [c.r, c.g, c.b, c.a]).unwrap_or([0; 4]),
        _ => [0; 4],
    }
}

// ------------------------------------------------------------------ W8 Farbe ersetzen

#[test]
fn replace_color_factory_defaults_are_integers() {
    let p = Preferences::new();
    assert_eq!(p.get("ReplaceColorToolTolerance"), Some(PrefValue::Int(32)));
    assert_eq!(p.get("ReplaceColorToolStrength"), Some(PrefValue::Int(200)));
    assert_eq!(p.get_str("ReplaceColorToolSrcColor", ""), "#00FF00");
    assert_eq!(p.get_str("ReplaceColorToolDestColor", ""), "#FF0000");
}

// ------------------------------------------------------------------ Auswahl füllen / Toleranz je Werkzeug

#[test]
fn fill_and_gradient_options_are_separate() {
    let mut o = ToolOptions::from_prefs(&Preferences::new());
    assert!(!o.fill_selection, "Farbeimer: Werksvorgabe aus");
    assert!(o.gradient_fill_selection, "Verlauf: Werksvorgabe an");
    assert!(o.fills_selection(ToolId::Gradient) && !o.fills_selection(ToolId::PaintBucket));
    assert_eq!((o.tolerance_of(ToolId::PaintBucket), o.tolerance_of(ToolId::Gradient)), (32, 32));
    // ändern: das andere Werkzeug bleibt unberührt, auch 50× hintereinander
    for _ in 0..50 {
        o.set_fills_selection(ToolId::Gradient, false);
        o.set_tolerance_of(ToolId::Gradient, 77);
    }
    assert!(!o.gradient_fill_selection && !o.fill_selection);
    assert_eq!((o.fill_tolerance, o.gradient_tolerance), (32, 77));
    o.set_fills_selection(ToolId::PaintBucket, true);
    assert!(o.fill_selection && !o.gradient_fill_selection);
    // aus Voreinstellungen je Werkzeug
    let mut p = Preferences::new();
    p.set("GradientToolFillsSelection", PrefValue::Bool(false));
    p.set("GradientToolTolerance", PrefValue::Int(10));
    let o = ToolOptions::from_prefs(&p);
    assert!(!o.gradient_fill_selection && !o.fill_selection);
    assert_eq!((o.fill_tolerance, o.gradient_tolerance), (32, 10));
    // ältere gespeicherte Optionen ohne die neuen Felder: Werksvorgaben
    let mut v = serde_json::to_value(ToolOptions::default()).expect("json");
    let m = v.as_object_mut().expect("obj");
    m.remove("gradient_fill_selection");
    m.remove("gradient_tolerance");
    let o: ToolOptions = serde_json::from_value(v).expect("alt");
    assert!(o.gradient_fill_selection);
    assert_eq!(o.gradient_tolerance, 32);
}

/// Bitmap: links rot, rechts blau (je 10 px breit, 10 px hoch) an (10, 10).
fn two_color_bitmap(s: &mut Session) -> ElementId {
    let b = bitmap(s, 10, 10, 20, 10, [255, 0, 0, 255]);
    ex(s, "select.pixels", json!({"shape": "rect", "rect": [20, 10, 30, 20]}));
    ex(s, "bitmap.floodFill", json!({"point": [25, 15], "color": "#0000ff", "fillSelection": true}));
    ex(s, "select.none", json!({}));
    select(s, &[b]);
    assert_eq!(px(s, b, 15, 5), [0, 0, 255, 255]);
    b
}

#[test]
fn bitmap_gradient_respects_fill_selection_and_tolerance() {
    let mut s = doc_session(100.0, 60.0);
    let b = two_color_bitmap(&mut s);
    // ohne „Auswahl füllen“: nur der rote Farbbereich am Anfangspunkt (Toleranz 0)
    ex(&mut s, "bitmap.gradient", json!({"from": [10, 15], "to": [20, 15], "colors": ["#000000", "#ffffff"], "fillSelection": false, "tolerance": 0}));
    assert!(px(&s, b, 0, 5)[0] < 30 && px(&s, b, 0, 5)[2] < 30, "links Verlaufsanfang {:?}", px(&s, b, 0, 5));
    assert_eq!(px(&s, b, 15, 5), [0, 0, 255, 255], "blau bleibt");
    // ein Schritt im Verlauf, Rückgängig stellt rot wieder her
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(px(&s, b, 0, 5), [255, 0, 0, 255]);
    // mit „Auswahl füllen“ (Vorgabe) ohne Pixelauswahl: die ganze Bitmap
    ex(&mut s, "bitmap.gradient", json!({"from": [10, 15], "to": [30, 15], "colors": ["#000000", "#ffffff"]}));
    // ohne Pixelauswahl wächst die Bitmap auf die Leinwand (PAR 07/07): Bildkoordinaten = Dokument
    assert!(px(&s, b, 29, 15)[2] > 200 && px(&s, b, 29, 15)[0] > 200, "rechts Verlaufsende {:?}", px(&s, b, 29, 15));
}

#[test]
fn gradient_tool_uses_its_own_fill_selection() {
    use fp_engine::ToolEvent;
    let mut s = doc_session(100.0, 60.0);
    let b = two_color_bitmap(&mut s);
    // Farbeimer „Auswahl füllen“ an, Verlauf aus: der Verlauf füllt nur den Farbbereich
    s.tools.options.fill_selection = true;
    s.tools.options.gradient_fill_selection = false;
    s.tools.options.gradient_tolerance = 0;
    s.set_tool(ToolId::Gradient).expect("Werkzeug");
    events(&mut s, &[ToolEvent::down(11.0, 15.0), ToolEvent::drag(15.0, 15.0), ToolEvent::drag(19.0, 15.0), ToolEvent::up(19.0, 15.0)]);
    assert_ne!(px(&s, b, 0, 5), [255, 0, 0, 255], "rot übermalt");
    assert_eq!(px(&s, b, 15, 5), [0, 0, 255, 255], "blau bleibt (Toleranz 0, nicht der Farbeimer-Wert)");
}

// ------------------------------------------------------------------ Texturen und Muster per Name

fn write_png(path: &std::path::Path) {
    let mut s = doc_session(16.0, 16.0);
    let _ = rect(&mut s, 0.0, 0.0, 8.0, 16.0);
    std::fs::create_dir_all(path.parent().expect("Ordner")).expect("Ordner anlegen");
    ex(&mut s, "file.exportArea", json!({"rect": [0, 0, 16, 16], "path": path.to_string_lossy(), "settings": {"format": "png32"}}));
    assert!(path.is_file(), "{path:?}");
}

#[test]
fn texture_and_pattern_by_name_load_from_resource_folders() {
    let _ = doc_session(10.0, 10.0); // Testprofil
    let user = fp_i18n::profile::roaming_dir().expect("Testprofil");
    write_png(&user.join("Textures").join("Fv16Textur.png"));
    write_png(&user.join("Patterns").join("Fv16Muster.png"));
    assert!(fp_engine::resources::texture_file("fv16textur").is_some(), "ohne Rücksicht auf Groß-/Kleinschreibung");
    assert!(fp_engine::resources::texture_file("Fv16Textur.png").is_some(), "mit Endung");
    assert!(fp_engine::resources::texture_file("gibt-es-nicht").is_none());

    let mut s = doc_session(200.0, 100.0);
    let v = ex(&mut s, "draw.rectangle", json!({"rect": [10, 10, 90, 60], "fillColor": "#808080", "strokeColor": "#000000", "strokeWidth": 2}));
    let r = ElementId(v["id"].as_u64().unwrap_or(0));
    select(&mut s, &[r]);
    ex(&mut s, "pi.fill", json!({"texture": "Fv16Textur", "textureAmount": 50}));
    let tex = |s: &Session, stroke: bool| match &el(s, r).kind {
        ElementKind::Path(p) => if stroke { p.attrs.stroke_texture.clone() } else { p.attrs.fill_texture.clone() },
        _ => None,
    };
    assert!(tex(&s, false).is_some_and(|t| t.mask.is_some() && t.name == "Fv16Textur"), "Fülltextur mit Maske");
    ex(&mut s, "pi.stroke", json!({"texture": "Fv16Textur", "textureAmount": 30}));
    assert!(tex(&s, true).is_some_and(|t| t.mask.is_some()), "Strichtextur mit Maske");
    // unbekannter Name: Fehlermeldung, Textur bleibt (AGENTS §3; FIX-D nach CT-2)
    assert!(s.execute("pi.fill", json!({"texture": "gibt-es-nicht"})).is_err());
    assert!(tex(&s, false).is_some_and(|t| t.mask.is_some() && t.name == "Fv16Textur"));
    // Werkzeugvorgabe für neue Objekte (`tool.shape` mit `pi.fill {texture}`)
    s.tools.options.new_fill = Some(json!({"texture": "Fv16Textur", "textureAmount": 40}));
    let inv = fp_engine::tools::styled(fp_engine::tools::Invocation::new("draw.rectangle", json!({"rect": [100, 10, 150, 50]})), &s.tools.options);
    let v = ex(&mut s, &inv.command, inv.params.clone());
    let n = ElementId(v["id"].as_u64().unwrap_or(0));
    let has = match &el(&s, n).kind {
        ElementKind::Path(p) => p.attrs.fill_texture.as_ref().is_some_and(|t| t.mask.is_some()),
        _ => false,
    };
    assert!(has, "neues Objekt mit Textur aus der Werkzeugvorgabe hat eine Maske");

    // Muster per Name: pi.fill und element.setPattern, ein Schritt, Rückgängig
    select(&mut s, &[r]);
    let before = s.active().map(|d| d.history.undo_count()).unwrap_or(0);
    ex(&mut s, "pi.fill", json!({"pattern": "fv16muster"}));
    let pat = |s: &Session| match &el(s, r).kind {
        ElementKind::Path(p) => p.attrs.fill.as_ref().and_then(|f| f.pattern.as_ref()).is_some_and(|p| p.image.is_some()),
        _ => false,
    };
    assert!(pat(&s), "Muster mit Bild");
    assert_eq!(s.active().map(|d| d.history.undo_count()).unwrap_or(0), before + 1);
    ex(&mut s, "edit.undo", json!({}));
    assert!(!pat(&s));
    ex(&mut s, "element.setPattern", json!({"name": "Fv16Muster"}));
    assert!(pat(&s), "element.setPattern nur per Name");
    assert!(s.execute("pi.fill", json!({"pattern": "gibt-es-nicht"})).is_err(), "unbekanntes Muster: Fehler");
    assert!(s.execute("element.setPattern", json!({"name": "gibt-es-nicht"})).is_err());
    // element.setTexture nur per Name
    ex(&mut s, "element.setTexture", json!({"name": "Fv16Textur", "amount": 20}));
    assert!(tex(&s, false).is_some_and(|t| t.mask.is_some()));
    ex(&mut s, "element.setTexture", json!({}));
    assert!(tex(&s, false).is_none(), "ohne Bild und Name: Textur entfernt");
}

/// Das Verlauf-Werkzeug malt seine eigenen Farben (Werksvorgabe Schwarz → Weiß), nicht Füll- →
/// Strichfarbe; auf Vektorobjekten ebenso.
#[test]
fn gradient_tool_paints_own_colors() {
    use fp_engine::ToolEvent;
    let mut s = doc_session(100.0, 60.0);
    let b = bitmap(&mut s, 10, 10, 40, 10, [128, 128, 128, 255]);
    select(&mut s, &[b]);
    ex(&mut s, "color.set", json!({"stroke": "#c04040", "fill": "#808080"}));
    s.set_tool(ToolId::Gradient).expect("Werkzeug");
    events(&mut s, &[ToolEvent::down(10.0, 15.0), ToolEvent::drag(30.0, 15.0), ToolEvent::drag(50.0, 15.0), ToolEvent::up(50.0, 15.0)]);
    let (l, r) = (px(&s, b, 10, 15), px(&s, b, 49, 15));
    assert!(l[0] < 20 && l[1] < 20 && l[2] < 20, "Anfang schwarz {l:?}");
    assert!(r[0] > 235 && r[1] > 235 && r[2] > 235, "Ende weiß {r:?}");
    // eigene Werkzeugfarben
    s.tools.options.gradient_colors = vec![fp_color::Argb(0xffff_0000), fp_color::Argb(0xff00_00ff)];
    events(&mut s, &[ToolEvent::down(10.0, 15.0), ToolEvent::drag(30.0, 15.0), ToolEvent::drag(50.0, 15.0), ToolEvent::up(50.0, 15.0)]);
    let (l, r) = (px(&s, b, 10, 15), px(&s, b, 49, 15));
    assert!(l[0] > 235 && l[2] < 20 && r[2] > 235 && r[0] < 20, "rot → blau {l:?} {r:?}");
    // Vektorobjekt: Verlauf mit den Werkzeugfarben
    s.tools.options.gradient_colors = fp_engine::tools::default_gradient_colors();
    let v = ex(&mut s, "draw.rectangle", json!({"rect": [10, 30, 90, 55], "fillColor": "#808080"}));
    let id = ElementId(v["id"].as_u64().unwrap_or(0));
    select(&mut s, &[id]);
    s.set_tool(ToolId::Gradient).expect("Werkzeug");
    events(&mut s, &[ToolEvent::down(12.0, 40.0), ToolEvent::drag(50.0, 40.0), ToolEvent::drag(88.0, 40.0), ToolEvent::up(88.0, 40.0)]);
    let cols: Vec<u32> = match &el(&s, id).kind {
        ElementKind::Path(p) => p.attrs.fill.as_ref().and_then(|f| f.gradient.as_ref()).map(|g| g.color_stops().iter().map(|c| c.color.0 | 0xff00_0000).collect()).unwrap_or_default(),
        _ => Vec::new(),
    };
    assert_eq!(cols, vec![0xff00_0000, 0xffff_ffff]);
}
