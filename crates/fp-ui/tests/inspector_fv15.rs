//! Funktionsvergleich mit dem Original (FV-15, Paket P-IBD1): Texturliste aus allen Texturordnern
//! (Original 77 = `Configuration\Textures` + Benutzerordner) mit „Andere…“ (W21-Rest), Musterliste
//! nach demselben Prinzip, `pi.text.antialias` wie `text.setProperties` (EB), Hinweis der
//! KI-Objektauswahl „Esc: Bitmapmodus beenden“ (HG). Realistische Eingaben wie egui-winit unter
//! Windows (je Ereignis ein Bild, Tippen als `Event::Text`, Tastendruck und Loslassen getrennt).

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::{By, NodeT, Queryable};
use egui_kittest::Harness;
use fp_engine::ToolId;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::inspector::obj_quads::{self, DEFAULT_TEXTURE};
use fp_ui::inspector::popups;
use fp_ui::FirepixelApp;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;

type H = Harness<'static, FirepixelApp>;

const W: f32 = 1800.0;
const HH: f32 = 1130.0;

fn harness(setup: impl FnOnce(&mut fp_engine::Session)) -> H {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 500, "height": 300, "background": "#ffffff"})).expect("Dokument");
    setup(&mut s);
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(W, HH)).with_pixels_per_point(1.0).with_max_steps(20).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn rect(s: &mut fp_engine::Session, r: [i32; 4], c: &str) -> u64 {
    let v = s.execute("draw.rectangle", json!({"rect": r, "fillColor": c, "strokeWidth": 0})).expect("Rechteck");
    v.value.get("id").and_then(|x| x.as_u64()).expect("id")
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn press(h: &mut H, p: Pos2, down: bool) {
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: down, modifiers: Modifiers::NONE });
}

fn click(h: &mut H, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    press(h, p, true);
    press(h, p, false);
    h.run_steps(2);
}

fn tool(h: &mut H, t: ToolId) {
    let _ = h.state_mut().state.session.set_tool(t);
    h.run_steps(3);
}

fn undo_count(h: &H) -> usize {
    st(h).session.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn doc(h: &H) -> Arc<fp_model::Document> {
    Arc::clone(&st(h).session.active().expect("Dokument").doc)
}

/// Kombifelder im Inspektor: Rechteck, Wert, bedienbar.
fn combos(h: &H) -> Vec<(egui::Rect, String, bool)> {
    let mut v: Vec<(egui::Rect, String, bool)> = h
        .get_all(By::new().role(egui::accesskit::Role::ComboBox))
        .map(|n| (n.rect(), n.value().unwrap_or_default(), !n.accesskit_node().is_disabled()))
        .filter(|(r, _, _)| r.top() > HH - 160.0)
        .collect();
    v.sort_by(|a, b| a.0.left().total_cmp(&b.0.left()).then(a.0.top().total_cmp(&b.0.top())));
    v
}

fn vein() -> String {
    obj_quads::texture_label(DEFAULT_TEXTURE)
}

/// Kombifelder der Texturliste (Füllung zuerst, dann Strich) mit der Vorgabetextur.
fn texture_combos(h: &H) -> Vec<egui::Rect> {
    combos(h).into_iter().filter(|(_, v, _)| *v == vein()).map(|(r, _, _)| r).collect()
}

fn tmp_dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fp_fv15_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("Ordner");
    d
}

/// Kleines Graustufen-PNG (Textur: Helligkeit = Maske).
fn write_png(p: &Path) {
    let mut img = image::RgbaImage::new(8, 8);
    for (x, y, px) in img.enumerate_pixels_mut() {
        let v = if (x + y) % 2 == 0 { 255 } else { 0 };
        *px = image::Rgba([v, v, v, 255]);
    }
    img.save(p).expect("PNG");
}

// ------------------------------------------------------------------ W21 Texturordner

/// Wie im Original: Texturen aus `Configuration\Textures`, der Vorlage des Benutzerordners
/// (`First Run\Textures`) und dem eigenen Benutzerordner `%APPDATA%\Firepixel\Textures` zusammen,
/// je Name einmal (erste Fundstelle), nur Bilddateien, nach Namen geordnet.
#[test]
fn w21_texture_folders_are_merged() {
    let base = tmp_dir("dirs");
    let orig = base.join("orig");
    let user = base.join("user");
    for d in [orig.join("Configuration").join("Textures"), orig.join("First Run").join("Textures"), user.join("Textures")] {
        std::fs::create_dir_all(d).expect("Ordner");
    }
    write_png(&orig.join("Configuration").join("Textures").join("Vein.png"));
    write_png(&orig.join("Configuration").join("Textures").join("Grid 1.png"));
    write_png(&orig.join("First Run").join("Textures").join("grid 1.png"));
    write_png(&orig.join("First Run").join("Textures").join("Paper 01.png"));
    write_png(&user.join("Textures").join("Eigene.png"));
    std::fs::write(user.join("Textures").join("liesmich.txt"), "x").expect("txt");
    let dirs = popups::resource_dirs(Some(&orig), Some(&user), popups::USER_TEXTURES_DIR);
    assert_eq!(dirs.len(), 3);
    assert!(dirs[2].ends_with(Path::new("user").join("Textures")), "technisch englischer Ordnername: {dirs:?}");
    let files = popups::resource_files_in(&dirs);
    let names: Vec<&str> = files.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["Eigene", "Grid 1", "Paper 01", "Vein"]);
    let grid = &files.iter().find(|(n, _)| n == "Grid 1").expect("Grid 1").1;
    assert!(grid.starts_with(orig.join("Configuration")), "erste Fundstelle gilt: {grid:?}");
    // fehlende Ordner zählen nicht
    assert!(popups::resource_files_in(&popups::resource_dirs(Some(&base.join("fehlt")), None, "Textures")).is_empty());
    let _ = std::fs::remove_dir_all(&base);
}

/// Mit dem installierten Original (Ressourcenordner gefunden) zeigt die Liste wie dort 77 Texturen
/// (52 + 74 des Benutzerordners) und die Musterliste 60 (50 + 60). Ohne Original: übersprungen.
#[test]
fn w21_counts_like_original_when_installed() {
    let Some(o) = fp_color::install::original_dir() else { return };
    if !o.join("First Run").join("Textures").is_dir() {
        return;
    }
    let user_extra = fp_i18n::user_config_dir().is_some_and(|u| u.join("Textures").is_dir() || u.join("Patterns").is_dir());
    if user_extra {
        return;
    }
    assert_eq!(popups::texture_names().len(), 77, "Texturen wie im Original");
    assert_eq!(popups::pattern_files().len(), 60, "Muster wie im Original");
    // die Texturen haben deutsche Namen (bis auf gleichlautende wie „Chiffon“, „Sand“ und
    // „Argyle 01/02“, die auch das Original nicht übersetzt, PAR 18/12)
    let same = popups::texture_names().iter().filter(|n| obj_quads::texture_label(n) == **n).count();
    assert!(same <= 6, "{same} Texturen ohne deutschen Namen");
    let (items, _) = obj_quads::texture_items("");
    let labels: Vec<String> = items.iter().map(|(_, l)| l.to_lowercase()).collect();
    let mut sorted = labels.clone();
    sorted.sort();
    assert_eq!(labels, sorted, "nach dem angezeigten Namen geordnet");
}

/// Die Texturliste endet wie im Original mit einer Trennlinie und „Andere…“.
#[test]
fn w21_texture_list_ends_with_other() {
    let (items, labels, sel) = obj_quads::texture_list_labels("");
    assert_eq!(labels.len(), items.len() + 2);
    assert_eq!(labels[labels.len() - 2], "-");
    assert_eq!(labels.last().map(|s| s.as_str()), Some("Andere..."));
    assert_eq!(items[sel].0, DEFAULT_TEXTURE);
    fp_i18n::with_language("en", || assert_eq!(obj_quads::texture_list_labels("").1.last().map(|s| s.as_str()), Some("Other...")));
}

/// „Andere…“ an einem gewählten Rechteck: Datei wählen → Textur mit Maske (wird gezeichnet), Name =
/// Dateiname, genau ein Verlaufsschritt, Rückgängig stellt her. „Andere…“ ist ohne Bildlauf
/// erreichbar (unter der Liste). Werkzeugvorgabe: Name ohne Verlaufsschritt. §4b: 5× wählen →
/// eine Textur, kein Duplikat.
#[test]
fn w21_other_texture_from_file() {
    let base = tmp_dir("other");
    let file = base.join("Meine Textur.png");
    write_png(&file);
    let mut h = harness(|s| {
        rect(s, [50, 50, 150, 120], "#ff0000");
    });
    h.state_mut().state.file_override = Some(fp_ui::files::FilePickOverride { open: vec![file.clone()], save: None });
    let c = texture_combos(&h);
    assert_eq!(c.len(), 2, "Füll- und Strichtextur: {:?}", combos(&h));
    let d0 = doc(&h);
    let n0 = undo_count(&h);
    for k in 0..5 {
        let c = combos(&h).into_iter().find(|(r, _, _)| r.min == texture_combos_or(&h, k).min).expect("Fülltextur").0;
        click(&mut h, c.center());
        let other = h.get_by_label("Andere...").rect();
        assert!(other.bottom() <= HH && other.top() >= 0.0, "„Andere…“ sichtbar: {other:?}");
        click(&mut h, other.center());
        h.run_steps(2);
    }
    let attrs = obj_quads::shown_attrs(st(&h)).expect("Attribute");
    let tex = attrs.fill_texture.as_ref().expect("Fülltextur");
    assert_eq!(tex.name, "Meine Textur");
    assert!(tex.mask.is_some(), "Textur aus Datei hat eine Maske");
    assert_eq!(undo_count(&h), n0 + 5, "je Wahl ein Schritt");
    for _ in 0..5 {
        h.state_mut().state.session.undo().expect("undo");
    }
    h.run_steps(2);
    assert_eq!(format!("{:?}", doc(&h)), format!("{d0:?}"), "Rückgängig stellt her");
    // Werkzeugvorgabe (Rechteck-Werkzeug ohne Auswahl): Name gesetzt, kein Verlaufsschritt
    let _ = h.state_mut().state.session.execute("select.none", json!({}));
    tool(&mut h, ToolId::Rectangle);
    let n1 = undo_count(&h);
    let c = texture_combos(&h)[0];
    click(&mut h, c.center());
    let other = h.get_by_label("Andere...").rect();
    click(&mut h, other.center());
    h.run_steps(2);
    // AU-3: als Vorgabe wird der volle Pfad gemerkt, angezeigt wird der Name
    assert_eq!(obj_quads::tool_attrs(&h.ctx).fill_texture, file.to_string_lossy());
    assert_eq!(obj_quads::texture_display_name(&obj_quads::tool_attrs(&h.ctx).fill_texture), "Meine Textur");
    assert_eq!(undo_count(&h), n1, "Werkzeugvorgabe ohne Verlaufsschritt");
    assert!(combos(&h).iter().any(|(_, v, _)| v == "Meine Textur"), "Liste zeigt die gewählte Datei: {:?}", combos(&h));
    let _ = std::fs::remove_dir_all(&base);
}

/// Das Fülltextur-Feld nach `k` Wahlen: zuerst „Ader“, danach der gewählte Dateiname.
fn texture_combos_or(h: &H, k: usize) -> egui::Rect {
    if k == 0 {
        return texture_combos(h)[0];
    }
    combos(h).into_iter().find(|(_, v, _)| v == "Meine Textur").expect("Feld mit „Meine Textur“").0
}

/// Wahl einer Textur aus den Texturordnern an einem gewählten Objekt: die Datei wird geladen
/// (Maske vorhanden), Name bleibt der Dateiname; ein Schritt.
#[test]
fn w21_listed_texture_gets_mask() {
    // erster Eintrag oben in der Liste (ohne Bildlauf erreichbar), der nicht die Vorgabe ist
    let Some(name) = obj_quads::texture_items("").0.into_iter().map(|(n, _)| n).find(|n| n != DEFAULT_TEXTURE && popups::texture_file(n).is_some()) else { return };
    let mut h = harness(|s| {
        rect(s, [50, 50, 150, 120], "#ff0000");
    });
    let c = texture_combos(&h)[0];
    let n0 = undo_count(&h);
    click(&mut h, c.center());
    let r = h.get_by_label(&obj_quads::texture_label(&name)).rect();
    click(&mut h, r.center());
    h.run_steps(2);
    let attrs = obj_quads::shown_attrs(st(&h)).expect("Attribute");
    let tex = attrs.fill_texture.as_ref().expect("Fülltextur");
    assert_eq!(tex.name, name);
    assert!(tex.mask.is_some(), "Textur aus dem Texturordner hat eine Maske");
    assert_eq!(undo_count(&h), n0 + 1);
}

/// Knopf „Muster“ an einem gewählten Rechteck (PAR 18/01, 18/08): stellt in einem Schritt auf das
/// Vorgabemuster „Baumwolle“ um und öffnet sofort das Muster-Popup; dessen Liste zeigt die Muster
/// mit deutschen Namen wie im Original (alphabetisch, darunter „Andere...“); Wahl eines Musters ist
/// ein Schritt und setzt die Musterfüllung.
#[test]
fn w21_pattern_list_like_texture_list() {
    if popups::pattern_files().is_empty() {
        return;
    }
    let mut h = harness(|s| {
        rect(s, [50, 50, 150, 120], "#ff0000");
    });
    // Muster-Knopf: vierter Füllart-Knopf rechts über der Fülltextur-Liste (Lage wie im echten Fenster)
    let tex = texture_combos(&h)[0];
    let pattern = egui::pos2(tex.left() + 119.0, tex.top() - 38.0);
    let n0 = undo_count(&h);
    click(&mut h, pattern);
    h.run_steps(2);
    let name = |h: &H| obj_quads::shown_attrs(st(h)).and_then(|a| a.fill.as_ref().and_then(|f| f.pattern.as_ref().map(|p| p.name.clone())));
    assert_eq!(name(&h).as_deref(), Some("Cotton"), "Vorgabemuster");
    assert_eq!(undo_count(&h), n0 + 1, "ein Schritt");
    assert!(popups::anchored_open(&h.ctx, obj_quads::fill_popup_ids(false).1), "Muster-Popup offen");
    // Liste im Popup aufklappen
    let combo = h.get_all(By::new().role(egui::accesskit::Role::ComboBox)).find(|n| n.value().as_deref() == Some("Baumwolle")).expect("Musterliste mit „Baumwolle“").rect();
    click(&mut h, combo.center());
    let other = h.get_by_label("Andere...").rect();
    assert!(other.bottom() <= HH, "„Andere…“ sichtbar: {other:?}");
    let items = popups::pattern_items();
    assert_eq!(items.first().map(|(_, l)| l.as_str()), Some("Baumwolle"), "alphabetisch wie im Original");
    let (second, label) = items[1].clone();
    let item = h.get_by_label(&label).rect();
    assert!(item.top() < other.top());
    click(&mut h, item.center());
    h.run_steps(2);
    assert_eq!(name(&h).as_deref(), Some(second.as_str()));
    assert_eq!(undo_count(&h), n0 + 2);
    assert!(popups::anchored_open(&h.ctx, obj_quads::fill_popup_ids(false).1), "Wahl in der Liste schließt das Popup nicht");
}

// ------------------------------------------------------------------ EB pi.text.antialias

/// `pi.text.antialias {mode}` ordnet wie beschrieben zu (0 kein = harte Füllkante, 1 scharf,
/// 2 stark, 3 glatt = `TAAi` 0, 4 benutzerdefiniert) und damit wie `text.setProperties`.
#[test]
fn eb_pi_text_antialias_mapping() {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("Dokument");
    let id = s.execute("draw.text", json!({"text": "Hallo", "anchor": [20, 60], "size": 24})).expect("Text").value.get("id").and_then(|v| v.as_u64()).expect("id");
    let read = |s: &fp_engine::Session| {
        let d = s.active().expect("Dokument");
        let e = fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(id)).expect("Text");
        let fp_model::ElementKind::Text(t) = &e.kind else { panic!("kein Text") };
        (t.antialias, t.attrs.fill.as_ref().map(|f| f.edge))
    };
    for (mode, name) in [(0, "none"), (1, "crisp"), (2, "strong"), (3, "smooth"), (4, "custom")] {
        s.execute("pi.text.antialias", json!({"mode": mode, "ids": [id]})).expect("pi.text.antialias");
        let a = read(&s);
        s.execute("text.setProperties", json!({"antialias": name, "ids": [id]})).expect("text.setProperties");
        assert_eq!(a, read(&s), "mode {mode} wie antialias {name}");
        if mode == 0 {
            assert_eq!(a.1, Some(0), "kein Anti-Alias = harte Füllkante");
        } else {
            assert_eq!(a.1, Some(1), "weiche Kante bei mode {mode}");
        }
        if mode == 3 {
            assert_eq!(a.0, 0, "glatt = TAAi 0");
        }
    }
}

// ------------------------------------------------------------------ HG Hinweis KI-Objektauswahl

/// Der Hinweis der KI-Objektauswahl nennt, was Esc tut (wie beim Zauberstab: Bitmapmodus beenden,
/// zum Zeiger), und passt in Deutsch und Englisch in sein Feld.
#[test]
fn hg_ai_object_hint_names_exit_bitmap_mode() {
    assert!(fp_i18n::t("pi-ai-object-hint2").ends_with("Esc: Bitmapmodus beenden"));
    fp_i18n::with_language("en", || assert!(fp_i18n::t("pi-ai-object-hint2").ends_with("Esc: exit bitmap mode")));
    for lang in ["de", "en"] {
        fp_i18n::with_language(lang, || {
            let mut h = harness(|_| {});
            tool(&mut h, ToolId::AiObject);
            fp_ui::inspector::fields::set_audit(&h.ctx, true);
            h.run_steps(1);
            let log = fp_ui::inspector::fields::take_audit(&h.ctx);
            fp_ui::inspector::fields::set_audit(&h.ctx, false);
            let hint = fp_i18n::t("pi-ai-object-hint2");
            let e = log.iter().find(|e| e.text == hint).unwrap_or_else(|| panic!("{lang}: Hinweis gezeichnet"));
            assert_eq!(e.fit, 0, "{lang}: Hinweis passt ({:?} in {:?})", e.drawn, e.avail);
        });
    }
}
