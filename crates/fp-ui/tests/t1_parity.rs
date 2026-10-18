//! T-1: Treue-Punkte aus dem Nachlauf 15.10. abends (TW-43, 44, 47, 59…64).

use egui::{Rect, Shape};
use egui_kittest::Harness;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::panels::PanelId;
use fp_ui::FirepixelApp;
use serde_json::{json, Value};

type H = Harness<'static, FirepixelApp>;

fn harness_with(s: fp_engine::Session, show: &[PanelId]) -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    for p in show {
        app.state.settings.workspace.show(*p);
    }
    let mut h = Harness::builder().renderer(crate::gpu_common::renderer()).with_size(egui::vec2(1600.0, 1000.0)).with_pixels_per_point(1.0).with_max_steps(20).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn painted_texts(h: &H) -> Vec<(String, Rect)> {
    fn walk(s: &Shape, out: &mut Vec<(String, Rect)>) {
        match s {
            Shape::Text(t) => out.push((t.galley.text().to_string(), t.visual_bounding_rect())),
            Shape::Vec(v) => v.iter().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    let mut v = Vec::new();
    for cs in &h.output().shapes {
        walk(&cs.shape, &mut v);
    }
    v
}

fn rect_of(v: &Value) -> [f32; 4] {
    let a = v.as_array().expect("Rechteck");
    [0, 1, 2, 3].map(|i| a[i].as_f64().expect("Zahl") as f32)
}

/// TW-59: `panel_rects` meldet für die Ebenen nur deren Gruppe (bis über die Reiterleiste der
/// Stile), und `ui_find {role: text}` findet die Reiter wieder als gezeichneten Text.
#[test]
fn tw59_layers_rect_excludes_styles_group() {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 200, "height": 100})).expect("doc");
    let mut h = harness_with(s, &[PanelId::Layers, PanelId::Styles]);
    h.run_steps(2);
    let ctx = h.ctx.clone();
    let v = fp_ui::bridge::ui_state(&ctx, st(&h));
    let rects = v["panel_rects"].as_object().expect("panel_rects");
    let layers = rect_of(rects.get("window.layers").unwrap_or_else(|| panic!("Ebenen fehlen: {rects:?}")));
    let styles = rect_of(rects.get("window.styles").unwrap_or_else(|| panic!("Stile fehlen: {rects:?}")));
    assert!(layers[3] <= styles[1] + 0.5, "Ebenen {layers:?} reichen in die Stile {styles:?}");
    let tab = PanelId::Layers.title().to_string();
    let painted = painted_texts(&h);
    let lt = painted.iter().find(|(t, r)| *t == tab && r.center().x > layers[0] && r.center().y < layers[1] + 30.0);
    assert!(lt.is_some(), "Reiter „{tab}“ oben im Ebenen-Rechteck {layers:?}");
    // Reiter der Stile als `role: text` (Läufer: Gruppengrenze)
    let stab = PanelId::Styles.title().to_string();
    let hits = fp_ui::bridge::find_in_output(&json!({"text": stab, "role": "text", "exact": true}), h.output());
    assert!(hits.iter().any(|x| (x["rect"][1].as_f64().unwrap_or(0.0) as f32 - styles[1]).abs() < 30.0), "Reiter Stile als Text: {hits:?}");
    // verdeckter Reiter (z. B. Verlauf hinter Optimieren) hat kein Rechteck
    assert!(!rects.contains_key("window.history") || st(&h).settings.workspace.is_front(PanelId::History));
}

/// TW-62: Neuer „Einfarbiger Schatten“ wie im Original: Winkel 45, Abstand 30, „Einfarbig“ aus
/// (= Objektfarben, wie die Vorgabe des Effekts selbst).
#[test]
fn tw62_solid_shadow_defaults() {
    let f = fp_ui::dialogs::effects::effect_form("solidShadow", None, None, None, fp_color::Argb::default());
    assert_eq!(f.get("angle").as_f64(), Some(45.0));
    assert_eq!(f.get("dist").as_f64(), Some(30.0));
    assert_eq!(f.get("solid").as_bool(), Some(false), "Einfarbig aus");
}

fn ev(h: &mut H, e: egui::Event) {
    h.event(e);
    h.run_steps(1);
}

fn click(h: &mut H, p: egui::Pos2) {
    use egui::{Event, Modifiers, PointerButton};
    ev(h, Event::PointerMoved(p));
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(3);
}

fn panel_rect(h: &H, name: &str) -> Rect {
    let v = fp_ui::bridge::ui_state(&h.ctx, st(h));
    let r = rect_of(v["panel_rects"].get(name).unwrap_or_else(|| panic!("{name} nicht gezeichnet: {}", v["panel_rects"])));
    Rect::from_min_max(egui::pos2(r[0], r[1]), egui::pos2(r[2], r[3]))
}

fn styles_menu_new_entry(h: &H) -> Option<bool> {
    let nodes = fp_ui::menu::build_panel_menu(st(h), PanelId::Styles);
    nodes.iter().find_map(|n| match n {
        fp_ui::menu::Node::Item { action: fp_ui::menu::ItemAction::Command(c), enabled, .. } if c == "panel.styles.new" => Some(*enabled),
        _ => None,
    })
}

/// TW-63/TW-48: „Neuer Stil“ ist wie im Original auch ohne Auswahl aktiv; über ≡ ▸ Neuer Stil...
/// öffnet sich der Dialog (mit und ohne Auswahl), OK legt den Stil an.
#[test]
fn tw63_new_style_enabled_and_opens_dialog() {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 200, "height": 100})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [10, 10, 50, 40], "fillColor": "#ff0000"})).expect("rect");
    s.execute("select.none", json!({})).expect("none");
    let mut h = harness_with(s, &[PanelId::Styles]);
    h.run_steps(2);
    assert_eq!(styles_menu_new_entry(&h), Some(true), "ohne Auswahl aktiv");
    assert!(st(&h).session.command_state("panel.styles.new").0, "Befehl ohne Auswahl verfügbar");
    for with_sel in [false, true] {
        if with_sel {
            h.state_mut().state.session.execute("select.all", json!({})).expect("all");
            h.run_steps(2);
        }
        // ≡ der Stile-Gruppe rechts in der Reiterleiste
        let r = panel_rect(&h, "window.styles");
        click(&mut h, egui::pos2(r.right() - 10.0, r.top() + 10.0));
        let label = fp_i18n::t("cmd-panel-styles-new");
        let entry = painted_texts(&h).into_iter().find(|(t, _)| *t == label).unwrap_or_else(|| panic!("Eintrag „{label}“ im Menü (Auswahl {with_sel})"));
        click(&mut h, entry.1.center());
        h.run_steps(2);
        let f = st(&h).dialogs.form.as_ref().unwrap_or_else(|| panic!("Dialog Neuer Stil offen (Auswahl {with_sel})"));
        assert_eq!(f.command, "pi.style.new");
        h.state_mut().state.dialogs.form = None;
        h.run_steps(1);
    }
    // ohne Auswahl: Stil aus den Werkzeugattributen
    h.state_mut().state.session.execute("select.none", json!({})).expect("none");
    h.state_mut().state.session.execute("panel.styles.new", json!({"name": "Ohne"})).expect("Stil ohne Auswahl");
    let d = st(&h).session.active().expect("doc");
    let sty = d.doc.styles.as_ref().and_then(|v| v.iter().find(|x| x.name == "Ohne").cloned()).expect("Stil angelegt");
    assert!(sty.attrs.is_some(), "Attribute aus dem Werkzeug");
}

fn menu_labels(nodes: &[fp_ui::menu::Node]) -> Vec<String> {
    nodes
        .iter()
        .map(|n| match n {
            fp_ui::menu::Node::Item { label, .. } | fp_ui::menu::Node::Sub { label, .. } => label.clone(),
            fp_ui::menu::Node::Separator => "-".into(),
        })
        .collect()
}

/// TW-64: Trennlinien im Kontextmenü von Segment/Hotspot wie im Original: keine zwischen
/// „CSS-Sprites exportieren...“ und „Anordnen“, eine vor „Alle Verhalten löschen“.
#[test]
fn tw64_web_context_menu_separators() {
    for slice in [true, false] {
        let mut s = fp_engine::Session::default();
        s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("doc");
        s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 80]})).expect("rect");
        s.execute("select.all", json!({})).expect("sel");
        s.execute(if slice { "edit.insert.slice" } else { "edit.insert.hotspot" }, json!({})).expect("web");
        let h = harness_with(s, &[]);
        let id = fp_ui::canvas::context::menu_for(st(&h));
        let l = menu_labels(&fp_ui::canvas::context::build(st(&h), fp_ui::canvas::context::entries(id)));
        let pos = |t: &str| l.iter().position(|x| x == t).unwrap_or_else(|| panic!("{t} fehlt: {l:?}"));
        let clear = pos("Alle Verhalten löschen");
        assert_eq!(l[clear - 1], "-", "Trennlinie vor „Alle Verhalten löschen“: {l:?}");
        assert_eq!(l[clear + 1], "-", "Trennlinie vor „Vollbildmodus beenden“: {l:?}");
        if slice {
            let a = pos("Anordnen");
            assert_eq!(l[a - 1], "CSS-Sprites exportieren...", "keine Trennlinie vor „Anordnen“: {l:?}");
            assert_eq!(l[a + 1], "-", "Trennlinie nach „Anordnen“: {l:?}");
        }
    }
}

fn tmp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("fp-t1-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("Ordner");
    dir
}

/// TW-60/TW-43: Bei einer Seite sind „Seite öffnen“ und „Zeichenfläche“ grau, bei mehreren aktiv;
/// die Felder stehen wie im Original (Einheit neben dem Feld, Pfade/Text/Art in einer Zeile).
#[test]
fn tw60_vector_options_page_choice_and_layout() {
    fp_i18n::with_language("de", || {
        let dir = tmp_dir("tw60");
        let ai = dir.join("probe.ai");
        std::fs::write(&ai, "%!PS-3.0\n%%Creator: Probe\n%%BoundingBox: 0 0 120 90\n%%EndSetup\n0 0 0 1 k\n10 10 m\n90 10 L\n90 40 L\n10 40 L\nf\n").expect("AI");
        let mut s = fp_engine::Session::default();
        s.execute("file.new", json!({"width": 300, "height": 200})).expect("doc");
        let mut h = harness_with(s, &[]);
        assert!(fp_ui::dialogs::vector_options::open(&mut h.state_mut().state, &ai), "Dialog für AI");
        {
            let f = st(&h).dialogs.form.as_ref().expect("Formular");
            let dis = |k: &str| f.fields.iter().find(|x| x.key == k).map(|x| x.disabled);
            assert_eq!((dis("convert"), dis("artboard")), (Some(true), Some(true)), "eine Seite: Seitenwahl grau");
        }
        h.run_steps(3);
        let texts = painted_texts(&h);
        let at = |t: &str| texts.iter().find(|(x, _)| x == t).map(|(_, r)| *r).unwrap_or_else(|| panic!("{t} nicht gezeichnet"));
        let (breite, pfade, glatt) = (at("Breite:"), at("Pfade"), at("Glatt"));
        // „Text“ steht auch in der Menüleiste: das Kästchen rechts von „Pfade“
        let text = texts.iter().filter(|(x, r)| x == "Text" && r.left() > pfade.right()).map(|(_, r)| *r).next().expect("Kästchen Text");
        let pixel = texts.iter().filter(|(x, _)| x == "Pixel").map(|(_, r)| *r).find(|r| (r.center().y - breite.center().y).abs() < 6.0);
        assert!(pixel.is_some_and(|p| p.left() > breite.right()), "Einheit rechts neben Breite in derselben Zeile");
        assert!((pfade.center().y - text.center().y).abs() < 3.0 && (text.center().y - glatt.center().y).abs() < 6.0, "Pfade, Text, Glatt in einer Zeile: {pfade:?} {text:?} {glatt:?}");
        assert!((at("Zeichenfläche:").center().y - at("Seite öffnen").center().y).abs() < 6.0, "Zeichenfläche neben Seite öffnen");
        // mehrere Seiten: aktiv; Zeichenfläche 2 wählt Seite 2, „Seiten als Status öffnen“ sperrt sie
        let mut f = fp_ui::dialogs::vector_options::form_pages(&ai, 120.0, 90.0, 3);
        assert!(f.fields.iter().filter(|x| x.key == "convert" || x.key == "artboard").all(|x| !x.disabled), "3 Seiten: aktiv");
        f.field_mut("artboard").expect("artboard").value = json!(1);
        let p = fp_ui::dialogs::vector_options::params(&f);
        assert_eq!((p["vector"]["page"].as_i64(), p["vector"]["pagesAsStates"].as_bool()), (Some(1), Some(false)));
        f.field_mut("convert").expect("convert").value = json!(1);
        f.field_mut("layers").expect("layers").value = json!(2);
        let p = fp_ui::dialogs::vector_options::params(&f);
        assert_eq!((p["vector"]["page"].as_i64(), p["vector"]["pagesAsStates"].as_bool(), p["vector"]["layers"].as_i64()), (Some(0), Some(true), Some(1)), "Seiten als Status: Ebenen in Status wird berücksichtigen");
        f.field_mut("groups").expect("groups").value = json!(true);
        f.field_mut("groupsN").expect("groupsN").value = json!(2.0);
        assert_eq!(fp_ui::dialogs::vector_options::params(&f)["vector"]["groupsOver"].as_u64(), Some(2));
        let _ = std::fs::remove_dir_all(&dir);
    });
}

fn element_names(e: &fp_model::Element, out: &mut Vec<String>) {
    let kind = match &e.kind {
        fp_model::ElementKind::Group(_) => "group",
        fp_model::ElementKind::Bitmap(_) => "bitmap",
        fp_model::ElementKind::Path(_) => "path",
        _ => "other",
    };
    out.push(format!("{}:{kind}", e.common.name.clone().unwrap_or_default()));
    if let fp_model::ElementKind::Group(g) = &e.kind {
        for c in &g.children {
            element_names(c, out);
        }
    }
}

/// Namen und Arten aller Objekte im Status `frame` der aktiven Seite (`Name:art`).
fn names_in(h: &H, frame: usize) -> Vec<String> {
    let d = st(h).session.active().expect("d");
    let mut out = Vec::new();
    for slot in &d.doc.pages[d.page].root.layers {
        if let fp_model::LayerSlot::Own(l) = slot
            && let Some(c) = l.cels.get(frame)
        {
            for e in c.elements.iter() {
                element_names(e, &mut out);
            }
        }
    }
    out
}

/// TW-43: Seitenwahl, „Seiten als Status öffnen“, „Ebenen in Status konvertieren“ und „Als Bilder
/// wiedergeben ▸ Gruppierungen über N Objekte“ wirken beim Import (`file.import` mit `vector`).
#[test]
fn tw43_vector_options_take_effect() {
    let dir = tmp_dir("tw43");
    let src = dir.join("quelle.fw.png");
    {
        // Quelle: Seite 1 mit zwei Ebenen (unten Rechteck A, oben Gruppe aus 3 Rechtecken), Seite 2 mit B
        let mut s = fp_engine::Session::default();
        s.execute("file.new", json!({"width": 200, "height": 200, "background": "#ffffff"})).expect("doc");
        s.execute("draw.rectangle", json!({"rect": [10, 10, 30, 30], "name": "A"})).expect("A");
        s.execute("layer.add", json!({})).expect("Ebene");
        for (i, n) in ["G1", "G2", "G3"].iter().enumerate() {
            let x = 50 + 20 * i as i32;
            s.execute("draw.rectangle", json!({"rect": [x, 50, x + 10, 60], "name": n})).expect("G");
        }
        s.execute("select.inRect", json!({"rect": [45, 45, 115, 65]})).expect("Auswahl");
        assert_eq!(s.active().expect("d").selection.elements.len(), 3, "drei G gewählt");
        s.execute("modify.group", json!({})).expect("Gruppe");
        s.execute("page.add", json!({})).expect("Seite");
        assert_eq!(s.active().expect("d").page, 1, "neue Seite aktiv");
        s.execute("draw.rectangle", json!({"rect": [100, 100, 140, 140], "name": "B"})).expect("B");
        s.execute("file.saveAs", json!({"path": src.to_string_lossy()})).expect("speichern");
    }
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 300, "background": "#ffffff"})).expect("doc");
    let mut h = harness_with(s, &[]);
    let has = |v: &[String], n: &str| v.iter().any(|x| x.starts_with(&format!("{n}:")));
    let import = |h: &mut H, v: Value, undo: bool| {
        let s = &mut h.state_mut().state.session;
        if undo {
            s.execute("edit.undo", json!({})).expect("Rückgängig");
        }
        s.execute("file.import", json!({"path": src.to_string_lossy(), "at": [0, 0], "vector": v})).expect("Import");
    };
    // Zeichenfläche 2 → nur B
    import(&mut h, json!({"page": 1}), false);
    let n = names_in(&h, 0);
    assert!(has(&n, "B") && !has(&n, "A"), "Seite 2: {n:?}");
    // Ebenen in Status konvertieren: oberste Ebene (Gruppe) in Status 1, A in Status 2
    import(&mut h, json!({"layers": 2}), true);
    assert!(st(&h).session.active().expect("d").frame_count() >= 2, "zweiter Status angelegt");
    let (f0, f1) = (names_in(&h, 0), names_in(&h, 1));
    assert!(has(&f0, "G1") && !has(&f0, "A"), "Status 1: {f0:?}");
    assert!(has(&f1, "A"), "Status 2: {f1:?}");
    // Seiten als Status öffnen: Seite 1 in Status 1, Seite 2 (B) in Status 2
    import(&mut h, json!({"pagesAsStates": true}), true);
    let (f0, f1) = (names_in(&h, 0), names_in(&h, 1));
    assert!(has(&f0, "A") && !has(&f0, "B"), "Status 1: {f0:?}");
    assert!(has(&f1, "B"), "Status 2: {f1:?}");
    // Gruppierungen über 2 Objekte als Bild: die Gruppe aus 3 Rechtecken wird zur Bitmap, A bleibt Pfad
    import(&mut h, json!({"layers": 1, "groupsOver": 2}), true);
    let n = names_in(&h, 0);
    assert!(n.iter().any(|x| x.ends_with(":bitmap")), "Gruppe mit 3 Objekten als Bitmap: {n:?}");
    assert!(!has(&n, "G1"), "Gruppe aufgelöst: {n:?}");
    assert!(n.iter().any(|x| x == "A:path"), "A bleibt Pfad: {n:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Bild des Dialogs „Vektordatei-Optionen“ zum Vergleich mit dem Original (von Hand:
/// `FP_T1_SHOT=<ordner> cargo test -p fp-ui --test all t1_parity::tw43_dialog_image -- --ignored`).
#[test]
#[ignore]
fn tw43_dialog_image() {
    let Some(out) = std::env::var_os("FP_T1_SHOT").map(std::path::PathBuf::from) else { return };
    fp_i18n::with_language("de", || {
        let dir = tmp_dir("tw43img");
        let ai = dir.join("probe.ai");
        std::fs::write(&ai, "%!PS-3.0\n%%BoundingBox: 0 0 120 90\n%%EndSetup\n0 0 0 1 k\n10 10 m\n90 10 L\n90 40 L\n10 40 L\nf\n").expect("AI");
        let mut s = fp_engine::Session::default();
        s.execute("file.new", json!({"width": 300, "height": 200})).expect("doc");
        let mut h = harness_with(s, &[]);
        assert!(fp_ui::dialogs::vector_options::open(&mut h.state_mut().state, &ai));
        h.run_steps(4);
        let _ = std::fs::create_dir_all(&out);
        if let Ok(img) = h.render() {
            let _ = img.save(out.join("tw43_vector_options.png"));
        }
        let _ = std::fs::remove_dir_all(&dir);
    });
}

fn slice_harness() -> (H, u64) {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 500, "height": 300, "background": "#ffffff"})).expect("doc");
    let r = s.execute("draw.rectangle", json!({"rect": [20, 20, 140, 90]})).expect("R").value["id"].as_u64().expect("id");
    s.execute("select.set", json!({"ids": [r]})).expect("Auswahl");
    let sid = s.execute("edit.insert.slice", json!({})).expect("Segment").value["id"].as_u64().expect("id");
    s.execute("select.set", json!({"ids": [sid]})).expect("Auswahl");
    let mut h = harness_with(s, &[]);
    h.run_steps(3);
    (h, sid)
}

fn slice_export_of(h: &H, id: u64) -> Option<fp_model::ExportSettings> {
    let d = st(h).session.active().expect("doc");
    match &fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(id)).expect("Segment").kind {
        fp_model::ElementKind::WebObject(w) => w.slice.as_ref().and_then(|s| s.export.as_deref().cloned()),
        _ => None,
    }
}

/// TW-61: Exportformat direkt unter „Typ“ (gleiche linke Kante), Bearbeiten-Symbol links davon; die
/// Liste beginnt mit PNG32, JPEG – Höhere Qualität … in der Reihenfolge des Originals, Zusatzformate
/// am Ende, und ist nicht breiter als nötig.
#[test]
fn tw61_slice_export_combo_position_and_list() {
    use egui_kittest::kittest::{By, Queryable};
    fp_i18n::with_language("de", || {
        let (mut h, _) = slice_harness();
        let combos: Vec<(Rect, String)> = h.get_all(By::new().role(egui::accesskit::Role::ComboBox)).map(|n| (n.rect(), n.value().unwrap_or_default())).collect();
        let ty = combos.iter().find(|(_, v)| v == "Vordergrundbild").map(|(r, _)| *r).unwrap_or_else(|| panic!("Typ: {combos:?}"));
        let ex = combos.iter().filter(|(_, v)| v == "PNG32").map(|(r, _)| *r).find(|r| (r.left() - ty.left()).abs() < 3.0).unwrap_or_else(|| panic!("Exportformat unter Typ: {combos:?}"));
        assert!(ex.top() > ty.bottom() - 1.0 && ex.top() < ty.bottom() + 12.0, "direkt unter Typ: {ty:?} {ex:?}");
        let pencil = h.query_by_label("Exportoptionen bearbeiten").map(|n| n.rect()).expect("Bearbeiten-Symbol");
        assert!(pencil.right() <= ex.left() + 1.0 && (pencil.center().y - ex.center().y).abs() < 4.0, "Symbol links neben dem Feld: {pencil:?} {ex:?}");
        click(&mut h, ex.center());
        let texts = painted_texts(&h);
        let order = ["PNG32", "JPEG – Höhere Qualität", "JPEG – Kleinere Datei", "GIF Web 216", "GIF WebSnap 256", "GIF WebSnap 128", "GIF Adaptiv 256", "Animiertes GIF WebSnap 128", "PNG8"];
        let labels: Vec<String> = order.iter().map(|n| fp_ui::panels::optimize::preset_label(n)).collect();
        let ys: Vec<f32> = labels
            .iter()
            .map(|n| texts.iter().filter(|(t, r)| t == n && !ex.contains(r.center())).map(|(_, r)| r.center().y).next().unwrap_or_else(|| panic!("{n} nicht in der Liste")))
            .collect();
        assert!(ys.windows(2).all(|w| w[0] < w[1]), "Reihenfolge wie im Original, Zusatzformate danach: {ys:?}");
    });
}

/// TW-44: Das Bearbeiten-Symbol öffnet die Bildvorschau nur für das Segment (Exportbereich =
/// Segment); OK schreibt die Einstellung in das Segment, die Seite bleibt unverändert.
#[test]
fn tw44_slice_preview_writes_slice_settings() {
    use egui_kittest::kittest::Queryable;
    fp_i18n::with_language("de", || {
        let (mut h, sid) = slice_harness();
        let page_before = {
            let d = st(&h).session.active().expect("d");
            serde_json::to_value(d.doc.pages[d.page].root.export.as_deref()).ok()
        };
        let pencil = h.query_by_label("Exportoptionen bearbeiten").map(|n| n.rect()).expect("Bearbeiten-Symbol");
        click(&mut h, pencil.center());
        let p = st(&h).dialogs.preview.as_ref().expect("Bildvorschau offen");
        assert_eq!(p.slice, Some(fp_model::ElementId(sid)), "nur für das Segment");
        assert_eq!(p.panes[0].settings.crop, Some([20, 20, 140, 90]), "Exportbereich = Segment");
        // OK mit JPEG: in das Segment, ohne Exportbereich
        let mut s = p.panes[0].settings.clone();
        s.format = fp_export::ExportFormat::Jpeg;
        h.state_mut().state.dialogs.preview = None;
        assert!(fp_ui::dialogs::image_preview::commit_slice_settings(&mut h.state_mut().state, fp_model::ElementId(sid), &s));
        h.run_steps(2);
        let e = slice_export_of(&h, sid).expect("Exporteinstellung des Segments");
        let o = fp_export::OptimizeSettings::from_model(&e, None);
        assert_eq!(o.format, fp_export::ExportFormat::Jpeg);
        assert_eq!(o.crop, None, "kein Exportbereich im Segment");
        let d = st(&h).session.active().expect("d");
        assert_eq!(serde_json::to_value(d.doc.pages[d.page].root.export.as_deref()).ok(), page_before, "Seite unverändert");
        // gleiche Einstellung noch einmal: kein Schritt
        assert!(!fp_ui::dialogs::image_preview::commit_slice_settings(&mut h.state_mut().state, fp_model::ElementId(sid), &s));
    });
}
