//! G-B4 (Paket B4 aus docs\berichte\b2b.md):
//! - BA-022 Tab-Folge und Startfokus der Filterdialoge D45/D47–D50, OK klemmt still und schließt
//! - BA-023 Datei-Info: Felder der Seite „Beschreibung“ wie im Original, Register, Fußzeile
//! - BA-027 Inspektor: Volle/Halbe Höhe über das Symbol im Reiter, Reiter-Kontextmenü ohne Höhe;
//!   Optimieren-Menü bei GIF (Freigaben und Häkchen)
//! - BA-032 Exportassistent: Ergebnistexte und Vorschlag GIF Web 216 / JPEG Höhere Qualität 80
//!
//! Realistische Eingaben: Tasten wie egui-winit (Drücken/Loslassen), Tippen mit Taste + Text,
//! Mausklicks über mehrere Bilder.

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::kittest::{By, NodeT, Queryable};
use egui_kittest::Harness;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness_with(s: fp_engine::Session) -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(12).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(3);
    h
}

/// Dokument mit der Bitmap B1 (Filterdialoge).
fn bitmap_session() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [40, 40, 140, 120], "fillColor": "#3366cc", "name": "B1"})).expect("rect");
    s.execute("modify.flattenSelection", json!({})).expect("reduzieren");
    s
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn key(h: &mut H, k: Key) {
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: Modifiers::NONE });
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

fn typ(h: &mut H, s: &str) {
    for c in s.chars() {
        let k = Key::from_name(&c.to_string());
        if let Some(k) = k {
            h.event(Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: Modifiers::NONE });
        }
        ev(h, Event::Text(c.to_string()));
        if let Some(k) = k {
            ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: Modifiers::NONE });
        }
    }
}

fn click(h: &mut H, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

fn open(h: &mut H, cmd: &str) {
    h.state_mut().state.run(cmd, json!({}));
    h.run_steps(4);
    assert!(st(h).dialogs.any_open(), "{cmd}: Dialog offen");
}

fn dialog_rect(h: &H) -> Rect {
    fp_ui::dialogs::dialog_rect(&h.ctx, &fp_ui::dialogs::last_dialog_id(&h.ctx).expect("Dialog")).expect("Rechteck")
}

/// Rolle und Name (bzw. Wert) des fokussierten Bedienelements.
fn focused(h: &H) -> Option<(Role, String)> {
    h.ctx.memory(|m| m.focused())?;
    h.get_all(By::new()).find(|n| n.accesskit_node().is_focused()).map(|n| {
        let node = n.accesskit_node();
        (node.role(), node.label().map(|s| s.to_string()).or_else(|| node.value().map(|s| s.to_string())).unwrap_or_default())
    })
}

fn tabs(h: &mut H, n: usize) -> Vec<(Role, String)> {
    (0..n)
        .map(|_| {
            key(h, Key::Tab);
            focused(h).unwrap_or((Role::Unknown, String::new()))
        })
        .collect()
}

fn button(h: &H, label: &str) -> Rect {
    let dr = dialog_rect(h);
    h.query_all(By::new().role(Role::Button).label(label)).map(|n| n.rect()).find(|r| dr.contains(r.center())).unwrap_or_else(|| panic!("Knopf {label}"))
}

fn text_fields(h: &H) -> Vec<Rect> {
    let dr = dialog_rect(h);
    let mut v: Vec<Rect> = h.get_all(By::new().role(Role::TextInput)).map(|n| n.rect()).filter(|r| dr.contains(r.center())).collect();
    v.sort_by(|a, b| a.top().total_cmp(&b.top()));
    v
}

fn ok_l() -> String {
    fp_i18n::t("btn-ok")
}

fn cancel_l() -> String {
    fp_i18n::t("btn-cancel")
}

fn roles(seq: &[(Role, String)]) -> Vec<Role> {
    seq.iter().map(|x| x.0).collect()
}

// ---------------------------------------------------------------------- BA-022

/// D45 Rauschen: Startfokus OK; Menge › Farbe › OK › Abbrechen › Vorschau.
#[test]
fn ba022_d45_noise_tab_order() {
    let mut h = harness_with(bitmap_session());
    open(&mut h, "filter.noise.addNoise");
    assert_eq!(focused(&h), Some((Role::Button, ok_l())), "Startfokus OK");
    let seq = tabs(&mut h, 5);
    assert_eq!(roles(&seq), vec![Role::Button, Role::CheckBox, Role::TextInput, Role::CheckBox, Role::Button], "Tab-Folge {seq:?}");
    assert_eq!(seq[0].1, cancel_l());
    assert_eq!(seq[1].1, fp_i18n::t("dlg-preview"));
    assert_eq!(seq[3].1, fp_i18n::t("dlg-lbl-color-check"), "Farbe nach Menge");
    assert_eq!(seq[4].1, ok_l());
    key(&mut h, Key::Escape);
}

/// D47 Bewegungsunschärfe und D49 Radial: Startfokus OK; OK › Abbrechen › Vorschau › Feld › Feld.
#[test]
fn ba022_d47_d49_buttons_first() {
    for cmd in ["filter.blur.motion", "filter.blur.radial"] {
        let mut h = harness_with(bitmap_session());
        open(&mut h, cmd);
        assert_eq!(focused(&h), Some((Role::Button, ok_l())), "{cmd}: Startfokus OK");
        let seq = tabs(&mut h, 5);
        assert_eq!(roles(&seq), vec![Role::Button, Role::CheckBox, Role::TextInput, Role::TextInput, Role::Button], "{cmd}: Tab-Folge {seq:?}");
        assert_eq!(seq[0].1, cancel_l());
        assert_eq!(seq[4].1, ok_l(), "{cmd}: Runde zurück auf OK");
        key(&mut h, Key::Escape);
    }
}

/// D48 Gauß: Startfokus Feld; Radius › OK › Abbrechen › Vorschau › Schieber.
#[test]
fn ba022_d48_gauss_slider_last() {
    let mut h = harness_with(bitmap_session());
    open(&mut h, "filter.blur.gaussian");
    assert_eq!(focused(&h).map(|f| f.0), Some(Role::TextInput), "Startfokus Feld");
    let seq = tabs(&mut h, 5);
    assert_eq!(roles(&seq), vec![Role::Button, Role::Button, Role::CheckBox, Role::Slider, Role::TextInput], "Tab-Folge {seq:?}");
    assert_eq!(seq[0].1, ok_l());
    assert_eq!(seq[1].1, cancel_l());
    key(&mut h, Key::Escape);
}

/// D50 Zentriert: Startfokus OK; Anzahl › Qualität › OK › Abbrechen › Vorschau.
#[test]
fn ba022_d50_zoom_fields_first() {
    let mut h = harness_with(bitmap_session());
    open(&mut h, "filter.blur.zoom");
    assert_eq!(focused(&h), Some((Role::Button, ok_l())), "Startfokus OK");
    let seq = tabs(&mut h, 5);
    assert_eq!(roles(&seq), vec![Role::Button, Role::CheckBox, Role::TextInput, Role::TextInput, Role::Button], "Tab-Folge {seq:?}");
    key(&mut h, Key::Escape);
}

/// D45: „999“ in Menge und OK → keine Meldung, der Dialog schließt, der Wert ist geklemmt (400).
#[test]
fn ba022_ok_clamps_silently_and_closes() {
    for via_enter in [false, true] {
        let mut h = harness_with(bitmap_session());
        open(&mut h, "filter.noise.addNoise");
        let field = text_fields(&h)[0];
        click(&mut h, field.center());
        key(&mut h, Key::End);
        for _ in 0..4 {
            key(&mut h, Key::Backspace);
        }
        typ(&mut h, "999");
        if via_enter {
            key(&mut h, Key::Enter);
        } else {
            let ok = button(&h, &ok_l());
            click(&mut h, ok.center());
        }
        h.run_steps(4);
        assert!(st(&h).dialogs.form.is_none(), "Enter={via_enter}: Dialog geschlossen");
        assert!(st(&h).dialogs.message.is_none(), "Enter={via_enter}: keine Meldung");
        assert_eq!(st(&h).session.prefs.get_int("AddNoiseAmount", 0), 400, "Enter={via_enter}: geklemmt");
    }
}

// ---------------------------------------------------------------------- Brücke (RUN-5)

/// `ui_action {"panel": …}` erreicht jedes Bedienfeld des Fenster-Menüs (auch Ausrichten, Pfad,
/// Farbpalette, Sonderzeichen, Bildbearbeitung, Symbol- und Formeigenschaften), mit und ohne
/// `window.` sowie per Original-Befehls-ID; `ui_state.visible_panels` meldet es danach.
#[test]
fn bridge_reaches_every_window_menu_panel() {
    use fp_ui::panels::PanelId;
    let mut h = harness_with(bitmap_session());
    for p in PanelId::ALL {
        let name = p.bridge_name();
        assert!(name.starts_with("window.") || name == "edit.findAndReplace", "{p:?}: {name}");
        assert_eq!(PanelId::from_bridge_name(name), Some(*p), "{name}");
        assert_eq!(PanelId::from_bridge_name(&p.fw_id().to_string()), Some(*p), "{p:?}: Befehls-ID");
        if let Some(short) = name.strip_prefix("window.") {
            assert_eq!(PanelId::from_bridge_name(short), Some(*p), "{short}");
        }
    }
    for name in ["align", "window.path", "colorPalette", "specialCharacters", "imageEditing", "symbolProperties", "autoShapeProperties"] {
        let ctx = h.ctx.clone();
        let reply = fp_ui::bridge::ui_action(&ctx, &mut h.state_mut().state, &json!({"panel": name}));
        assert!(!matches!(reply, fp_mcp::UiReply::Error(_)), "{name}: bekannt");
        h.run_steps(3);
        let id = PanelId::from_bridge_name(name).expect("Bedienfeld");
        assert!(st(&h).settings.workspace.is_visible(id), "{name}: sichtbar");
        let ui = fp_ui::bridge::ui_state(&h.ctx, st(&h));
        let vis: Vec<&str> = ui["visible_panels"].as_array().expect("Liste").iter().filter_map(|v| v.as_str()).collect();
        assert!(vis.contains(&id.bridge_name()), "{name}: in visible_panels {vis:?}");
    }
    let ctx = h.ctx.clone();
    assert!(matches!(fp_ui::bridge::ui_action(&ctx, &mut h.state_mut().state, &json!({"panel": "gibtEsNicht"})), fp_mcp::UiReply::Error(_)));
}

// ---------------------------------------------------------------------- BA-023

/// Datei-Info: Register, Felder der Seite „Beschreibung“ in der Reihenfolge des Originals (von oben
/// nach unten), Sterne setzen die Bewertung, OK schreibt alle Felder in einem Verlaufsschritt.
#[test]
fn ba023_file_info_description_page() {
    for lang in ["de", "en"] {
        fp_i18n::set_language(lang);
        let mut s = fp_engine::Session::default();
        s.execute("file.new", json!({"width": 200, "height": 120})).expect("doc");
        let mut h = harness_with(s);
        open(&mut h, "file.fileInfo");
        let g = fp_ui::dialogs::dialog_geom(&h.ctx, "fp-file-info").expect("Lage");
        let win = egui::Rect::from_min_size(g.pos, g.client + egui::vec2(2.0, 2.0 + fp_ui::dialogs::DIALOG_TITLE_H));
        // Eingabefelder von oben nach unten: Titel, Autor, Autorentitel (einzeilig), Beschreibung
        // (mehrzeilig), Verfasser (einzeilig), Stichwörter (mehrzeilig), Copyright (mehrzeilig), URL
        let mut f: Vec<(f32, bool)> = Vec::new();
        for (role, multi) in [(Role::TextInput, false), (Role::MultilineTextInput, true)] {
            for n in h.query_all_by_role(role) {
                let r = n.rect();
                if win.contains(r.center()) {
                    assert!(win.expand(0.5).contains_rect(r), "{lang}: Feld {r:?} im Fenster");
                    f.push((r.top(), multi));
                }
            }
        }
        f.sort_by(|a, b| a.0.total_cmp(&b.0));
        assert_eq!(f.iter().map(|x| x.1).collect::<Vec<_>>(), vec![false, false, false, true, false, true, true, false], "{lang}: Felder {f:?}");
        h.run_steps(1);
        let over = fp_ui::dialogs::take_overflows(&h.ctx);
        assert!(over.is_empty(), "{lang}: Beschriftungen passen: {over:?}");
        for k in ["dlg-fileinfo-keywords-hint", "dlg-fileinfo-tab-description", "dlg-fileinfo-tab-audio"] {
            let txt = fp_i18n::t(k);
            assert!(!txt.is_empty() && !txt.starts_with("dlg-"), "{lang}: {k}");
        }
        for k in ["dlg-fileinfo-goto-url", "dlg-fileinfo-preferences", "btn-ok", "btn-cancel"] {
            let label = fp_i18n::t(k);
            let r = h.query_all(By::new().role(Role::Button).label(label.as_str())).map(|n| n.rect()).find(|r| win.contains(r.center())).unwrap_or_else(|| panic!("{lang}: Knopf {k}"));
            assert!(win.expand(0.5).contains_rect(r), "{lang}: Knopf {k}");
        }
        // vierter Stern → Bewertung 4
        let star = h.query_all(By::new().label(format!("{} 4", fp_i18n::t("dlg-fileinfo-rating")).as_str())).map(|n| n.rect()).next().expect("Stern 4");
        click(&mut h, star.center());
        assert_eq!(st(&h).dialogs.file_info.as_ref().expect("offen").rating, 4);
        {
            let d = h.state_mut().state.dialogs.file_info.as_mut().expect("offen");
            d.author_title = "Grafik".into();
            d.caption_writer = "Ben".into();
            d.copyright_status = 1;
            d.copyright_url = "https://example.org".into();
        }
        let before = st(&h).session.active().expect("doc").history.undo_count();
        let ok_label = ok_l();
        let ok = h.query_all(By::new().role(Role::Button).label(ok_label.as_str())).map(|n| n.rect()).find(|r| win.contains(r.center())).expect("OK");
        click(&mut h, ok.center());
        h.run_steps(3);
        assert!(st(&h).dialogs.file_info.is_none(), "{lang}: OK schließt");
        assert_eq!(st(&h).session.active().expect("doc").history.undo_count(), before + 1, "{lang}: ein Schritt");
        let m = h.state_mut().state.session.execute("file.fileInfo", json!({})).expect("info").value["metadata"].clone();
        assert_eq!((m["authorTitle"].as_str(), m["captionWriter"].as_str(), m["rating"].as_u64()), (Some("Grafik"), Some("Ben"), Some(4)), "{lang}");
        assert_eq!((m["copyrightStatus"].as_str(), m["copyrightUrl"].as_str()), (Some("copyrighted"), Some("https://example.org")), "{lang}");
    }
    fp_i18n::set_language("de");
}

// ---------------------------------------------------------------------- BA-032

/// Analyseergebnisse: vier Absätze wie im Original, Vorschlag GIF Web 216 und JPEG Höhere Qualität
/// (80) – auch mit frischem Profil (Vorgabe nicht von einer früheren Wahl abhängig).
#[test]
fn ba032_wizard_texts_and_web_proposal() {
    use fp_ui::dialogs::export_wizard::{self, Destination, ExportWizard, ResultPage, Step};
    for lang in ["de", "en"] {
        fp_i18n::set_language(lang);
        let (title, body) = export_wizard::result_page_text(ResultPage::WebAnalysis);
        assert_eq!(title, fp_i18n::t("dlg-wiz-res-analysis-title"));
        assert_eq!(body.split("\n\n").count(), 4, "{lang}: vier Absätze");
        assert!(body.contains("GIF") && body.contains("JPEG"), "{lang}");
    }
    fp_i18n::set_language("de");
    assert!(export_wizard::result_page_text(ResultPage::WebAnalysis).1.starts_with("Es wird empfohlen, das Format GIF oder JPEG zu verwenden."));
    assert_eq!(fp_i18n::t("dlg-wiz-dest-page"), "Webeditor");
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 200, "height": 120})).expect("doc");
    let mut st = AppState::new(s, UiSettings::default(), None);
    let mut w = ExportWizard::new(&st).expect("Assistent");
    w.analyze = false;
    w.advance();
    assert_eq!(w.step, Step::Destination);
    w.destination = Destination::Web;
    w.advance();
    assert_eq!(w.step, Step::Result(ResultPage::WebAnalysis));
    export_wizard::finish(&mut st, &w);
    let p = st.dialogs.preview.as_ref().expect("Bildvorschau");
    let (gif, jpeg) = (&p.panes[0].settings, &p.panes[1].settings);
    assert_eq!((gif.format, gif.palette_kind, gif.max_colors), (fp_export::options::ExportFormat::Gif, fp_export::options::PaletteKind::Web216, 216));
    assert_eq!((jpeg.format, jpeg.jpeg.quality), (fp_export::options::ExportFormat::Jpeg, 80));
}

// ---------------------------------------------------------------------- BA-027

fn rows(nodes: &[fp_ui::menu::Node]) -> Vec<String> {
    use fp_ui::menu::Node;
    nodes
        .iter()
        .map(|n| match n {
            Node::Separator => "---".to_string(),
            Node::Item { label, enabled, checked, .. } => {
                let mut s = label.clone();
                if !*enabled {
                    s.push_str(" (grau)");
                }
                if *checked == Some(true) {
                    s.push_str(" (✓)");
                }
                s
            }
            Node::Sub { label, .. } => format!("{label} ▸"),
        })
        .collect()
}

/// Reiter-Kontextmenü ohne Höhe-Einträge; das ◊-Symbol im Reiter schaltet volle ↔ halbe Höhe,
/// „Minimieren“ klappt auf den Reiter ein.
#[test]
fn ba027_inspector_height_symbol_and_tab_menu() {
    fp_i18n::set_language("de");
    let mut h = harness_with(bitmap_session());
    let m = rows(&fp_ui::menu::build_inspector_tab_menu(st(&h)));
    assert_eq!(m, ["Schließen", "Registerkartengruppe schließen", "Minimieren"]);
    assert!(!st(&h).settings.workspace.inspector_half);
    let sym = h.get_by_label(&fp_i18n::t("pi-height-toggle")).rect();
    click(&mut h, sym.center());
    assert!(st(&h).settings.workspace.inspector_half, "◊ → halbe Höhe");
    assert!(!st(&h).settings.workspace.inspector_collapsed, "Klick auf ◊ klappt nicht ein");
    let sym = h.get_by_label(&fp_i18n::t("pi-height-toggle")).rect();
    click(&mut h, sym.center());
    assert!(!st(&h).settings.workspace.inspector_half, "◊ → volle Höhe");
    // eingeklappt: ◊ öffnet in voller Höhe
    let mut ws = st(&h).settings.workspace.clone();
    ws.inspector_collapsed = true;
    fp_ui::inspector::toggle_half(&mut ws);
    assert!(!ws.inspector_collapsed && !ws.inspector_half);
}

/// Optimieren-Menü bei GIF Web 216: Sperrung/Bearbeitungen nur mit gesperrten bzw. bearbeiteten
/// Farben aktiv; „Nicht verwendete Farben entfernen“ und „Farbfeld-Rückmeldung“ mit Häkchen.
#[test]
fn ba027_optimize_menu_gif_web216() {
    fp_i18n::set_language("de");
    let mut s = bitmap_session();
    let preset = fp_export::options::builtin_presets().into_iter().find(|p| p.name == "GIF Web 216").expect("Vorgabe").settings;
    let base = s.active().and_then(|d| d.doc.pages.get(d.page)).and_then(|p| p.root.export.as_deref().cloned()).unwrap_or_default();
    s.execute("pi.export.settings", json!({"settings": serde_json::to_value(preset.to_model(&base)).expect("json")})).expect("GIF Web 216");
    let mut st = AppState::new(s, UiSettings::default(), None);
    let m = rows(&fp_ui::menu::build_panel_menu(&st, fp_ui::panels::PanelId::Optimize));
    for e in ["Mit Zeilensprung", "Progressives JPEG (grau)", "JPEG-Ränder scharfzeichnen (grau)", "Sperrung aller Farben aufheben (grau)", "Nicht verwendete Farben entfernen (✓)", "Farbfeld-Rückmeldung anzeigen (✓)", "Alle Bearbeitungen entfernen (grau)"] {
        assert!(m.iter().any(|r| r == e), "GIF Web 216: {e} erwartet in {m:#?}");
    }
    st.session.execute("panel.optimize.lockColors", json!({"colors": ["#ffffff"]})).expect("sperren");
    st.session.execute("panel.optimize.editColor", json!({"color": "#3366cc", "newColor": "#00ff00"})).expect("bearbeiten");
    let m = rows(&fp_ui::menu::build_panel_menu(&st, fp_ui::panels::PanelId::Optimize));
    for e in ["Sperrung aller Farben aufheben", "Alle Bearbeitungen entfernen"] {
        assert!(m.iter().any(|r| r == e), "mit Sperrung/Bearbeitung: {e} aktiv in {m:#?}");
    }
}
