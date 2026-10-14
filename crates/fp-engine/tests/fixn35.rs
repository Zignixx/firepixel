//! FIX-N35: Innen einfügen (Strg+Umschalt+V) wie im Original. Ablauf des Nutzers: großes Rechteck
//! (Füllung grau, Strich), kleines grünes darüber; grünes ausschneiden, großes wählen, Innen
//! einfügen. Ergebnis: **ein** Objekt (Vektormaske am großen Rechteck), sichtbar bleiben Füllung
//! und Strich des großen Rechtecks, das grüne liegt darin, an der Form beschnitten.

use crate::common;
use common::*;
use fp_engine::Session;
use fp_model::{ElementId, ElementKind};
use serde_json::json;

const GRAY: [u8; 3] = [0x99, 0x99, 0x99];
const GREEN: [u8; 3] = [0x00, 0xff, 0x00];
const RED: [u8; 3] = [0xff, 0x00, 0x00];
const WHITE: [u8; 3] = [0xff, 0xff, 0xff];

/// Großes Rechteck 50,50–350,250 (grau, 4 px roter Strich), grünes 120,100–`green_right`,200.
fn scene(green_right: f32) -> (Session, ElementId, ElementId) {
    let mut s = doc_session(400.0, 300.0);
    let big = ElementId(ex(&mut s, "draw.rectangle", json!({"rect": [50, 50, 350, 250], "fillColor": "#999999", "strokeWidth": 4, "strokeColor": "#ff0000"}))["id"].as_u64().unwrap_or(0));
    let green = ElementId(ex(&mut s, "draw.rectangle", json!({"rect": [120, 100, green_right, 200], "fillColor": "#00ff00", "strokeWidth": 0}))["id"].as_u64().unwrap_or(0));
    (s, big, green)
}

/// Der Ablauf des Nutzers: grünes wählen, ausschneiden, großes wählen, Innen einfügen.
fn paste_inside(s: &mut Session, big: ElementId, green: ElementId) -> ElementId {
    select(s, &[green]);
    ex(s, "edit.cut", json!({}));
    select(s, &[big]);
    let v = ex(s, "edit.pasteInside", json!({}));
    ElementId(v["id"].as_u64().unwrap_or(0))
}

fn rgb_at(s: &Session, pts: &[(i32, i32)]) -> Vec<[u8; 3]> {
    let d = s.active().expect("Dokument");
    let img = fp_engine::render::default_renderer().expect("Renderer").render_page(&d.doc, d.page, d.frame).expect("Bild");
    pts.iter()
        .map(|&(x, y)| {
            let p = img.pixel(x, y);
            [p.r, p.g, p.b]
        })
        .collect()
}

/// Randbereich zeigt die Füllung des großen Rechtecks, die Mitte das grüne, der Strich liegt oben.
#[test]
fn fill_and_stroke_of_target_stay_visible() {
    let (mut s, big, green) = scene(280.0);
    let before = rgb_at(&s, &[(60, 150), (200, 150), (50, 150), (20, 20)]);
    assert_eq!(before, vec![GRAY, GREEN, RED, WHITE]);
    paste_inside(&mut s, big, green);
    let after = rgb_at(&s, &[(60, 150), (200, 150), (50, 150), (20, 20)]);
    assert_eq!(after, before, "Bild wie vorher (Original 13.png → 15.png)");
}

/// Inhalt über den Rand hinaus: an der Form beschnitten, Strich oben über dem Inhalt.
#[test]
fn content_is_clipped_to_shape() {
    let (mut s, big, green) = scene(390.0);
    paste_inside(&mut s, big, green);
    let px = rgb_at(&s, &[(300, 150), (370, 150), (350, 150), (60, 150)]);
    assert_eq!(px, vec![GREEN, WHITE, RED, GRAY]);
}

/// Ein Objekt: Vektormaske (verknüpft, aktiv, Attribute sichtbar) mit dem grünen als Inhalt.
#[test]
fn one_object_with_linked_vector_mask() {
    let (mut s, big, green) = scene(280.0);
    let g = paste_inside(&mut s, big, green);
    let top = top_elements(&s);
    assert_eq!(top.len(), 1);
    assert_eq!(top[0].id, g);
    let (child, mask) = top[0].masked_single().expect("ein maskiertes Objekt");
    assert_eq!(mask.id, big);
    assert!(matches!(child.kind, ElementKind::Path(_)));
    let ElementKind::Group(gr) = &top[0].kind else { panic!("keine Maske") };
    assert_eq!(gr.kind, fp_model::GroupKind::VectorMask);
    assert!(gr.mask_flags.linked && gr.mask_flags.enabled && !gr.mask_flags.hide_attrs);
    assert!(s.active().is_some_and(|d| d.selection.elements.is_empty()));
}

/// Rückgängig ist ein Schritt: danach wieder nur das große Rechteck (grau in der Mitte).
#[test]
fn undo_is_one_step() {
    let (mut s, big, green) = scene(280.0);
    paste_inside(&mut s, big, green);
    ex(&mut s, "edit.undo", json!({}));
    let top = top_elements(&s);
    assert_eq!(top.len(), 1);
    assert_eq!(top[0].id, big);
    assert_eq!(rgb_at(&s, &[(200, 150)]), vec![GRAY]);
    ex(&mut s, "edit.redo", json!({}));
    assert_eq!(rgb_at(&s, &[(60, 150), (200, 150)]), vec![GRAY, GREEN]);
}

/// Speichern und Laden (FW-PNG) erhält Struktur, Flags und Bild.
#[test]
fn save_and_load_keeps_mask() {
    let (mut s, big, green) = scene(390.0);
    paste_inside(&mut s, big, green);
    let dir = std::env::temp_dir().join(format!("fp-fixn35-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("n35.png");
    ex(&mut s, "file.saveAs", json!({"path": path.to_string_lossy()}));
    let mut s2 = Session::default();
    ex(&mut s2, "file.open", json!({"path": path.to_string_lossy()}));
    let top = top_elements(&s2);
    assert_eq!(top.len(), 1);
    assert!(top[0].masked_single().is_some());
    let ElementKind::Group(gr) = &top[0].kind else { panic!("keine Maske") };
    assert!(!gr.mask_flags.hide_attrs);
    let pts = [(60, 150), (200, 150), (370, 150), (350, 150)];
    assert_eq!(rgb_at(&s2, &pts), rgb_at(&s, &pts));
    let _ = std::fs::remove_dir_all(&dir);
}

const INVERT: &str = "{d2541291-70d6-11d1-8c8000a024cdc039}";

/// Fall D2: Deckkraft des Inhalts gilt für das maskierte Element, also auch für die Füllung der Maske.
#[test]
fn content_opacity_applies_to_whole_result() {
    let (mut s, big, green) = scene(280.0);
    select(&mut s, &[green]);
    ex(&mut s, "element.setOpacity", json!({"opacity": 50}));
    let g = paste_inside(&mut s, big, green);
    assert_eq!(el(&s, g).common.opacity.0, 128);
    let px = rgb_at(&s, &[(60, 150), (200, 150)]);
    // Grau 0x99 bzw. Grün halb über Weiß
    assert!(px[0].iter().all(|&c| (203..=205).contains(&c)), "Füllung halbdeckend: {:?}", px[0]);
    assert!(px[1][0] >= 126 && px[1][0] <= 129 && px[1][1] == 255, "Inhalt halbdeckend über Weiß: {:?}", px[1]);
}

/// Fall D: Deckkraft der Maske (des Ziels) wird nicht gezeichnet.
#[test]
fn mask_opacity_is_ignored() {
    let (mut s, big, green) = scene(280.0);
    select(&mut s, &[big]);
    ex(&mut s, "element.setOpacity", json!({"opacity": 50}));
    paste_inside(&mut s, big, green);
    assert_eq!(rgb_at(&s, &[(60, 150), (200, 150)]), vec![GRAY, GREEN]);
}

/// Fall I: Ein Effekt des Ziels wandert auf das maskierte Element und wirkt auf das Ganze.
#[test]
fn target_effect_moves_to_result() {
    let (mut s, big, green) = scene(280.0);
    select(&mut s, &[big]);
    ex(&mut s, "element.addEffect", json!({"effect": INVERT}));
    let g = paste_inside(&mut s, big, green);
    let e = el(&s, g);
    assert!(e.common.effects.as_ref().is_some_and(|l| l.effects.len() == 1));
    let (_, mask) = e.masked_single().expect("Einzelelement trotz Effekt");
    assert!(mask.common.effects.as_ref().is_none_or(|l| l.effects.is_empty()));
    assert_eq!(rgb_at(&s, &[(60, 150), (200, 150)]), vec![[0x66, 0x66, 0x66], [0xff, 0x00, 0xff]]);
}

/// Fall J: Inhalt mit eigenem Effekt → Gruppe mit Maske (ausgewählt), Effekt bleibt am Inhalt.
#[test]
fn content_with_effect_gives_group() {
    let (mut s, big, green) = scene(280.0);
    select(&mut s, &[green]);
    ex(&mut s, "element.addEffect", json!({"effect": INVERT}));
    let g = paste_inside(&mut s, big, green);
    let e = el(&s, g);
    assert!(e.masked_single().is_none());
    assert_eq!(s.active().map(|d| d.selection.elements.clone()), Some(vec![g]));
    assert_eq!(rgb_at(&s, &[(60, 150), (200, 150)]), vec![GRAY, [0xff, 0x00, 0xff]]);
}

/// Fall E: zwei Inhalte → Gruppe mit Maske, Kinder in Zwischenablage-Reihenfolge, Gruppe gewählt.
#[test]
fn two_contents_give_group() {
    let (mut s, big, green) = scene(280.0);
    let red = ElementId(ex(&mut s, "draw.rectangle", json!({"rect": [300, 60, 400, 120], "fillColor": "#0000ff", "strokeWidth": 0}))["id"].as_u64().unwrap_or(0));
    select(&mut s, &[green, red]);
    ex(&mut s, "edit.cut", json!({}));
    select(&mut s, &[big]);
    let g = ElementId(ex(&mut s, "edit.pasteInside", json!({}))["id"].as_u64().unwrap_or(0));
    let e = el(&s, g);
    assert!(e.masked_single().is_none());
    let ElementKind::Group(gr) = &e.kind else { panic!("keine Gruppe") };
    assert_eq!(gr.children.len(), 2);
    assert_eq!(s.active().map(|d| d.selection.elements.clone()), Some(vec![g]));
    // blau bis zum Maskenrand (Strich oben), außerhalb abgeschnitten
    assert_eq!(rgb_at(&s, &[(320, 90), (380, 90), (200, 150)]), vec![[0, 0, 0xff], WHITE, GREEN]);
}

/// Fall G: Bitmap als Ziel → Maske bleibt Pfadumriss mit sichtbaren Attributen; die Bitmap liegt unten.
#[test]
fn bitmap_target_stays_path_mode() {
    let mut s = doc_session(400.0, 300.0);
    let bm = bitmap(&mut s, 50, 50, 300, 200, [0x99, 0x99, 0x99, 255]);
    let green = ElementId(ex(&mut s, "draw.rectangle", json!({"rect": [120, 100, 390, 200], "fillColor": "#00ff00", "strokeWidth": 0}))["id"].as_u64().unwrap_or(0));
    let g = paste_inside(&mut s, bm, green);
    let ElementKind::Group(gr) = &el(&s, g).kind else { panic!("keine Maske") };
    assert_eq!(gr.kind, fp_model::GroupKind::VectorMask);
    assert!(!gr.mask_flags.hide_attrs);
    assert_eq!(rgb_at(&s, &[(60, 150), (200, 150), (370, 150)]), vec![GRAY, GREEN, WHITE]);
}

/// Fall N: Ziel ist schon maskiert → vorhandene Maske bleibt, neuer Inhalt oben dazu.
#[test]
fn target_already_masked_keeps_mask() {
    let (mut s, big, green) = scene(280.0);
    let g = paste_inside(&mut s, big, green);
    let blue = ElementId(ex(&mut s, "draw.rectangle", json!({"rect": [60, 60, 100, 90], "fillColor": "#0000ff", "strokeWidth": 0}))["id"].as_u64().unwrap_or(0));
    select(&mut s, &[blue]);
    ex(&mut s, "edit.cut", json!({}));
    select(&mut s, &[g]);
    let g2 = ElementId(ex(&mut s, "edit.pasteInside", json!({}))["id"].as_u64().unwrap_or(0));
    let top = top_elements(&s);
    assert_eq!(top.len(), 1);
    let ElementKind::Group(gr) = &el(&s, g2).kind else { panic!("keine Gruppe") };
    assert_eq!(gr.mask.as_ref().map(|m| m.id), Some(big));
    assert_eq!(gr.children.len(), 2);
    // neuer Inhalt oben (Einfügen vergibt neue IDs, daher über die Grenzen)
    assert!(rect_approx(fp_engine::docops::element_bounds(&s.active().expect("Dokument").doc, &gr.children[0]).expect("Grenzen"), fp_geom::Rect::new(60.0, 60.0, 100.0, 90.0)));
    assert_eq!(rgb_at(&s, &[(80, 75), (200, 150), (60, 150)]), vec![[0, 0, 0xff], GREEN, GRAY]);
}

/// Klick auf die sichtbare Füllung der Maske trifft das Element (re 26 §3.1).
#[test]
fn click_on_mask_fill_hits() {
    let (mut s, big, green) = scene(280.0);
    let g = paste_inside(&mut s, big, green);
    let d = s.active().expect("Dokument");
    let e = el(&s, g);
    let opt = fp_engine::docops::HitOptions::default();
    assert!(fp_engine::docops::hit_element(&d.doc, &e, fp_geom::Point::new(70.0, 230.0), &fp_engine::geometry::BasicGeometry, &opt));
    assert!(!fp_engine::docops::hit_element(&d.doc, &e, fp_geom::Point::new(20.0, 20.0), &fp_engine::geometry::BasicGeometry, &opt));
}

/// Ohne „anzeigen“ (Als Maske einfügen): nur der Inhalt, beschnitten an der Fülldeckung; Farbe und
/// Strich der Maske zählen nicht.
#[test]
fn paste_as_mask_shows_only_content() {
    let (mut s, big, green) = scene(280.0);
    // Maske = grünes Rechteck, Inhalt = großes Rechteck
    select(&mut s, &[green]);
    ex(&mut s, "edit.cut", json!({}));
    select(&mut s, &[big]);
    ex(&mut s, "edit.pasteAsMask", json!({}));
    assert_eq!(rgb_at(&s, &[(200, 150), (110, 150), (60, 150)]), vec![GRAY, WHITE, WHITE]);
}
