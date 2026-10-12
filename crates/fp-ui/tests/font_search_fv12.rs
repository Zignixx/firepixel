//! Suche in der Schriftarten-Liste (FV-12, Nutzerbefund N19, über das Original hinaus): die
//! aufgeklappte Familienliste im Text-Inspektor (Objekt und Textwerkzeug) und im Texteditor hat oben
//! ein Suchfeld mit sofortigem Fokus. Tippen filtert ohne Groß-/Kleinschreibung und mit
//! Teilwörtern, ↑/↓ bewegen die Markierung, Eingabe übernimmt (ein Verlaufsschritt), Esc schließt
//! ohne Änderung (die Auswahl auf der Leinwand bleibt), kein Treffer zeigt „Keine Schrift gefunden“
//! (de/en). Ohne Tippen wie bisher. Realistische Eingaben wie egui-winit unter Windows (je
//! Ereignis ein Bild, Tippen als `Event::Text`, Drücken und Loslassen getrennt).
//! Wie alle Testläufe mit `FP_NO_SHELL_OPEN=1` starten (AGENTEN_REGELN §4a).

use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::{By, NodeT, Queryable};
use egui_kittest::Harness;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::inspector::fonts;
use fp_ui::theme::ThemeKind;
use fp_ui::FirepixelApp;
use serde_json::json;
use std::sync::{Arc, Mutex};

type H = Harness<'static, FirepixelApp>;

const W: f32 = 1800.0;
const HH: f32 = 1130.0;

/// Sprache ist global: Tests dieser Datei laufen nacheinander.
static LOCK: Mutex<()> = Mutex::new(());

struct Guard(#[allow(dead_code)] std::sync::MutexGuard<'static, ()>);

impl Drop for Guard {
    fn drop(&mut self) {
        fp_i18n::set_language("de");
    }
}

fn lock() -> Guard {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    fp_i18n::set_language("de");
    Guard(g)
}

/// Wartet auf den Schriftindex des Systems (im Hintergrund aufgebaut).
fn wait_fonts() -> Arc<Vec<fonts::Family>> {
    fonts::warm_up();
    let t0 = std::time::Instant::now();
    while fonts::families().len() <= 5 && t0.elapsed().as_secs() < 60 {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    fonts::families()
}

fn harness_with(setup: impl FnOnce(&mut fp_engine::Session), size: egui::Vec2, wgpu: bool, theme: ThemeKind) -> H {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 500, "height": 300, "background": "#ffffff"})).expect("Dokument");
    setup(&mut s);
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    app.state.set_theme(theme);
    let b = Harness::builder().with_size(size).with_pixels_per_point(1.0).with_max_steps(20).with_step_dt(1.0 / 60.0);
    let b = if wgpu { b.wgpu() } else { b };
    let mut h = b.build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn harness(setup: impl FnOnce(&mut fp_engine::Session)) -> H {
    harness_with(setup, egui::vec2(W, HH), false, ThemeKind::Normal)
}

/// Text „Hallo“ in Arial, ausgewählt; liefert seine ID.
fn text_session(s: &mut fp_engine::Session) -> u64 {
    let v = s.execute("draw.text", json!({"text": "Hallo", "anchor": [30, 60], "size": 24, "font": "Arial"})).expect("Text");
    let id = v.value.get("id").and_then(|x| x.as_u64()).expect("id");
    s.execute("select.set", json!({"ids": [id]})).expect("Auswahl");
    id
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
}

fn type_text(h: &mut H, t: &str) {
    for c in t.chars() {
        // wie egui-winit: Tastendruck (falls Buchstabe), Text, Loslassen
        let k = Key::from_name(&c.to_ascii_uppercase().to_string());
        if let Some(k) = k {
            h.event(Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: Modifiers::NONE });
        }
        h.event(Event::Text(c.to_string()));
        h.run_steps(1);
        if let Some(k) = k {
            ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: Modifiers::NONE });
        }
    }
    h.run_steps(1);
}

fn click(h: &mut H, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

fn undo_count(h: &H) -> usize {
    st(h).session.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn sel(h: &H) -> Vec<u64> {
    st(h).session.active().map(|d| d.selection.elements.iter().map(|e| e.0).collect()).unwrap_or_default()
}

/// Kombifelder im Inspektor (unterer Fensterteil): Rechteck, Wert.
fn combos(h: &H) -> Vec<(egui::Rect, String)> {
    h.query_all(By::new().role(egui::accesskit::Role::ComboBox)).map(|n| (n.rect(), n.value().unwrap_or_default())).filter(|(r, _)| r.top() > h.ctx.content_rect().bottom() - 160.0).collect()
}

fn combo_with(h: &H, value: &str) -> egui::Rect {
    let all = combos(h);
    let m: Vec<_> = all.iter().filter(|(_, v)| v == value).collect();
    assert_eq!(m.len(), 1, "genau ein Kombifeld mit „{value}“: {all:?}");
    m[0].0
}

/// Sichtbare Einträge der offenen Liste (Text, als aktuelle Schrift gekennzeichnet).
fn rows(h: &H) -> Vec<(String, bool)> {
    let fams = fonts::families();
    h.query_all(By::new().role(egui::accesskit::Role::Button))
        .filter_map(|n| {
            let l = n.accesskit_node().label()?;
            if !fams.iter().any(|f| f.name == l) {
                return None;
            }
            let sel = n.accesskit_node().toggled().map(|t| t == egui::accesskit::Toggled::True);
            sel.map(|s| (l, s))
        })
        .collect()
}

/// Rechteck des Suchfelds (fokussiertes Eingabefeld).
fn search_field(h: &H) -> egui::Rect {
    h.query_all(By::new().role(egui::accesskit::Role::TextInput)).filter(|n| n.accesskit_node().is_focused()).map(|n| n.rect()).next().expect("Suchfeld mit Fokus")
}

fn list_open(h: &H) -> bool {
    egui::Popup::is_any_open(&h.ctx)
}

fn text_debug(h: &H, id: u64) -> String {
    let d = st(h).session.active().expect("Dokument");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(id)).expect("Element");
    format!("{:?}", e.kind)
}

fn names(f: &[fonts::Family]) -> Vec<&str> {
    f.iter().map(|x| x.name.as_str()).collect()
}

/// Reine Filterlogik: Teilwort, ohne Groß-/Kleinschreibung, Fundstelle als Byte-Bereich.
#[test]
fn filter_is_case_insensitive_substring() {
    let n = ["Arial", "Arial Black", "Segoe UI", "Times New Roman", "Bahnschrift", "Ärger Grotesk"];
    assert_eq!(fonts::filter_names(&n, "ari"), vec![0, 1]);
    assert_eq!(fonts::filter_names(&n, "ARI"), vec![0, 1]);
    assert_eq!(fonts::filter_names(&n, "new ro"), vec![3]);
    assert_eq!(fonts::filter_names(&n, "  ui "), vec![2]);
    assert_eq!(fonts::filter_names(&n, "är"), vec![5]);
    assert_eq!(fonts::filter_names(&n, "ÄR"), vec![5]);
    assert!(fonts::filter_names(&n, "zzqx").is_empty());
    assert_eq!(fonts::filter_names(&n, "").len(), n.len(), "leer: alles");
    assert_eq!(fonts::find_ci("Times New Roman", "new"), Some((6, 9)));
    assert_eq!(fonts::find_ci("Ärger", "rg"), Some((2, 4)));
    // flüssig: 5000 Familien filtern dauert nur Bruchteile eines Bildes
    let many: Vec<String> = (0..5000).map(|i| format!("Family {i:04} Sans Condensed")).collect();
    let refs: Vec<&str> = many.iter().map(|s| s.as_str()).collect();
    let t0 = std::time::Instant::now();
    for _ in 0..10 {
        assert_eq!(fonts::filter_names(&refs, "0042").len(), 1);
    }
    let per = t0.elapsed().as_secs_f64() / 10.0;
    assert!(per < 0.02, "Filtern von 5000 Namen: {:.1} ms", per * 1000.0);
}

/// Inspektor (Text ausgewählt): Aufklappen gibt dem Suchfeld den Fokus, Tippen filtert, ↑/↓ und
/// Eingabe übernehmen genau eine Schrift als einen Verlaufsschritt.
#[test]
fn inspector_search_filter_arrows_enter() {
    let _g = lock();
    let fams = wait_fonts();
    let all = names(&fams);
    let q = "ar";
    let hits = fonts::filter_names(&all, q);
    assert!(hits.len() >= 2 && all.contains(&"Arial"), "System mit Arial und weiteren Treffern für „{q}“");
    let mut id = 0;
    let mut h = harness(|s| id = text_session(s));
    h.run_steps(2);
    let combo = combo_with(&h, "Arial");
    let n0 = undo_count(&h);
    click(&mut h, combo.center());
    assert!(list_open(&h), "Liste offen");
    assert!(h.ctx.text_edit_focused(), "Suchfeld hat sofort den Fokus");
    // ohne Tippen: die aktuelle Schrift ist markiert und in Sicht, nur sichtbare Zeilen gezeichnet
    assert_eq!(fonts::marked_family(&h.ctx).as_deref(), Some("Arial"));
    let r0 = rows(&h);
    assert!(r0.iter().any(|(l, s)| l == "Arial" && *s), "Arial in Sicht und als aktuell gekennzeichnet: {r0:?}");
    if fams.len() > 60 {
        assert!(r0.len() < 40, "nur sichtbare Zeilen ({} von {})", r0.len(), fams.len());
    }
    // Tippen filtert (Großbuchstaben, Teilwort)
    type_text(&mut h, "AR");
    let r1 = rows(&h);
    assert!(!r1.is_empty() && r1.iter().all(|(l, _)| l.to_lowercase().contains(q)), "nur Treffer: {r1:?}");
    assert_eq!(fonts::marked_family(&h.ctx).as_deref(), Some(all[hits[0]]), "erster Treffer markiert");
    key(&mut h, Key::ArrowDown);
    assert_eq!(fonts::marked_family(&h.ctx).as_deref(), Some(all[hits[1]]), "↓: zweiter Treffer");
    key(&mut h, Key::ArrowDown);
    key(&mut h, Key::ArrowUp);
    assert_eq!(fonts::marked_family(&h.ctx).as_deref(), Some(all[hits[1]]), "↓↑: wieder der zweite");
    // weiter tippen: der Filter wird enger; Rücktaste macht ihn wieder weiter
    type_text(&mut h, "ial");
    let narrow = fonts::filter_names(&all, "arial");
    assert_eq!(fonts::marked_family(&h.ctx).as_deref(), Some(all[narrow[0]]));
    key(&mut h, Key::Backspace);
    key(&mut h, Key::Backspace);
    key(&mut h, Key::Backspace);
    assert_eq!(fonts::marked_family(&h.ctx).as_deref(), Some(all[hits[0]]), "„ar“: wieder der erste Treffer");
    key(&mut h, Key::ArrowDown);
    let want = all[hits[1]].to_string();
    assert_eq!(undo_count(&h), n0, "Tippen und Pfeile ändern nichts am Dokument");
    key(&mut h, Key::Enter);
    h.run_steps(3);
    assert!(!list_open(&h), "Eingabe schließt die Liste");
    assert_eq!(undo_count(&h), n0 + 1, "genau ein Verlaufsschritt");
    combo_with(&h, &want);
    let ps = fonts::ps_name(&want, &fonts::base_style(fams.iter().find(|f| f.name == want).expect("Familie")));
    assert!(text_debug(&h, id).contains(&ps) || text_debug(&h, id).contains(&want), "Text hat die neue Schrift {want}/{ps}");
    assert_eq!(sel(&h), vec![id], "Auswahl bleibt");
    // erneut aufklappen: Suchtext leer, ganze Liste wie bisher
    let combo = combo_with(&h, &want);
    click(&mut h, combo.center());
    assert_eq!(fonts::marked_family(&h.ctx).as_deref(), Some(want.as_str()));
    let r2 = rows(&h);
    assert!(r2.iter().any(|(l, _)| !l.to_lowercase().contains(q)), "ungefiltert: {r2:?}");
    key(&mut h, Key::Escape);
    // Rückgängig stellt Arial wieder her
    fp_ui::app_state::AppState::push(&mut h.state_mut().state, fp_ui::app_state::Action::Run("edit.undo".into(), json!({})));
    h.run_steps(3);
    combo_with(&h, "Arial");
}

/// Esc schließt ohne Änderung; die Taste geht nicht an die Leinwand (Auswahl bleibt), ein Klick
/// in das Suchfeld schließt die Liste nicht.
#[test]
fn escape_closes_without_change() {
    let _g = lock();
    wait_fonts();
    let mut id = 0;
    let mut h = harness(|s| id = text_session(s));
    h.run_steps(2);
    let n0 = undo_count(&h);
    let before = text_debug(&h, id);
    let combo = combo_with(&h, "Arial");
    click(&mut h, combo.center());
    let field0 = search_field(&h);
    type_text(&mut h, "times");
    assert_eq!(search_field(&h), field0, "die Liste springt beim Tippen nicht");
    // Klick in das Suchfeld: bleibt offen, Fokus bleibt
    let field = search_field(&h);
    click(&mut h, field.center());
    assert!(list_open(&h), "Klick ins Suchfeld schließt die Liste nicht");
    assert!(h.ctx.text_edit_focused());
    key(&mut h, Key::ArrowDown);
    key(&mut h, Key::Escape);
    h.run_steps(2);
    assert!(!list_open(&h), "Esc schließt");
    assert_eq!(undo_count(&h), n0, "kein Verlaufsschritt");
    assert_eq!(text_debug(&h, id), before, "Text unverändert");
    assert_eq!(sel(&h), vec![id], "Esc hebt die Auswahl nicht auf");
    combo_with(&h, "Arial");
    // ein Klick auf einen Eintrag übernimmt wie bisher
    click(&mut h, combo.center());
    type_text(&mut h, "verdana");
    let verdana = h.query_all(By::new().label("Verdana")).map(|n| n.rect()).find(|r| r.top() > combo.bottom() - 1.0);
    if let Some(r) = verdana {
        click(&mut h, r.center());
        assert_eq!(undo_count(&h), n0 + 1);
        combo_with(&h, "Verdana");
    }
}

/// Kein Treffer: Hinweis in de und en; Eingabe übernimmt nichts.
#[test]
fn no_match_hint_de_en() {
    let _g = lock();
    wait_fonts();
    for (lang, want) in [("de", "Keine Schrift gefunden"), ("en", "No font found")] {
        fp_i18n::set_language(lang);
        assert_eq!(fp_i18n::t("pi-font-not-found"), want);
        let mut id = 0;
        let mut h = harness(|s| id = text_session(s));
        h.run_steps(2);
        let n0 = undo_count(&h);
        let combo = combo_with(&h, "Arial");
        click(&mut h, combo.center());
        let field0 = search_field(&h);
        type_text(&mut h, "zzqxw");
        assert_eq!(search_field(&h), field0, "{lang}: die Liste springt beim Tippen nicht");
        assert!(h.query_all(By::new().label(want)).next().is_some(), "{lang}: Hinweis „{want}“");
        assert!(rows(&h).is_empty(), "{lang}: keine Einträge");
        assert_eq!(fonts::marked_family(&h.ctx), None);
        key(&mut h, Key::Enter);
        assert!(list_open(&h), "{lang}: Eingabe ohne Treffer lässt die Liste offen");
        assert_eq!(undo_count(&h), n0);
        key(&mut h, Key::Escape);
        assert!(!list_open(&h));
        combo_with(&h, "Arial");
    }
}

/// Textwerkzeug ohne Auswahl: die Suche setzt die Vorgabeschrift des Werkzeugs.
#[test]
fn text_tool_defaults_search() {
    let _g = lock();
    let fams = wait_fonts();
    if !fams.iter().any(|f| f.name == "Verdana") {
        eprintln!("Verdana nicht installiert – übersprungen");
        return;
    }
    let mut h = harness(|_| {});
    h.state_mut().state.session.tools.options.text_font = "Arial".into();
    let _ = h.state_mut().state.session.set_tool(fp_engine::ToolId::Text);
    h.run_steps(3);
    let combo = combo_with(&h, "Arial");
    click(&mut h, combo.center());
    assert!(h.ctx.text_edit_focused());
    type_text(&mut h, "rdan");
    assert_eq!(fonts::marked_family(&h.ctx).as_deref(), Some("Verdana"));
    key(&mut h, Key::Enter);
    h.run_steps(3);
    combo_with(&h, "Verdana");
    let (fam, _) = fonts::lookup(&st(&h).session.tools.options.text_font);
    assert_eq!(fam, "Verdana", "Werkzeugvorgabe übernommen: {}", st(&h).session.tools.options.text_font);
}

/// Während ein Text auf der Leinwand bearbeitet wird, landen die Buchstaben der Suche nicht im Text.
#[test]
fn typing_in_search_does_not_reach_the_text() {
    let _g = lock();
    wait_fonts();
    let mut id = 0;
    let mut h = harness(|s| id = text_session(s));
    h.run_steps(2);
    let _ = h.state_mut().state.session.set_tool(fp_engine::ToolId::Text);
    h.run_steps(2);
    let d = st(&h).session.active().expect("Dokument");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(id)).expect("Element");
    let vis = fp_engine::docops::element_bounds(&d.doc, &e).expect("Grenzen");
    let area = st(&h).canvases.get(&d.id).and_then(|c| c.area).expect("Leinwand");
    let (sx, sy) = d.view.doc_to_screen(fp_geom::Point::new(vis.left + 12.0, vis.top + vis.height() / 2.0));
    click(&mut h, Pos2::new(area.min.x + sx as f32, area.min.y + sy as f32));
    assert!(st(&h).session.tool_is_editing_text(), "Text in Bearbeitung");
    let combo = combo_with(&h, "Arial");
    click(&mut h, combo.center());
    type_text(&mut h, "tim");
    assert!(st(&h).session.tool_is_editing_text(), "Bearbeitung läuft weiter");
    key(&mut h, Key::Escape);
    assert!(!list_open(&h));
    assert!(st(&h).session.tool_is_editing_text(), "Esc schließt nur die Liste, die Textbearbeitung bleibt");
    // zweites Esc beendet die Bearbeitung wie bisher; der Text enthält die Suchbuchstaben nicht
    key(&mut h, Key::Escape);
    h.run_steps(2);
    assert!(!st(&h).session.tool_is_editing_text(), "Esc auf der Leinwand beendet die Bearbeitung");
    let txt = text_debug(&h, id);
    assert!(txt.contains("Hallo") && !txt.contains("tim"), "Suchtext nicht im Text: {txt}");
}

/// Texteditor (Text ▸ Editor…): dieselbe Suche in der Schriftliste.
#[test]
fn text_editor_dialog_search() {
    let _g = lock();
    let fams = wait_fonts();
    if !fams.iter().any(|f| f.name == "Verdana") {
        eprintln!("Verdana nicht installiert – übersprungen");
        return;
    }
    let mut id = 0;
    let mut h = harness(|s| id = text_session(s));
    h.run_steps(2);
    fp_ui::app_state::AppState::push(&mut h.state_mut().state, fp_ui::app_state::Action::Run("text.editor".into(), json!({})));
    h.run_steps(3);
    assert!(st(&h).dialogs.text_editor.is_some(), "Texteditor offen");
    let dlg = fp_ui::dialogs::dialog_rect(&h.ctx, "fp-texteditor").expect("Editor gezeichnet");
    let combo = h.query_all(By::new().role(egui::accesskit::Role::ComboBox)).filter(|n| n.value().as_deref() == Some("Arial")).map(|n| n.rect()).find(|r| dlg.contains_rect(*r)).expect("Schriftfeld im Editor");
    click(&mut h, combo.center());
    assert!(h.ctx.text_edit_focused());
    type_text(&mut h, "VERD");
    assert_eq!(fonts::marked_family(&h.ctx).as_deref(), Some("Verdana"));
    key(&mut h, Key::Escape);
    assert!(st(&h).dialogs.text_editor.is_some(), "Esc schließt nur die Liste, nicht den Editor");
    click(&mut h, combo.center());
    type_text(&mut h, "verd");
    key(&mut h, Key::Enter);
    h.run_steps(2);
    assert!(st(&h).dialogs.text_editor.is_some(), "Eingabe in der Liste bestätigt nicht den Editor");
    let d = st(&h).dialogs.text_editor.as_ref().expect("Editor");
    assert_eq!(fonts::lookup(&d.font).0, "Verdana", "Editor-Schrift: {}", d.font);
}

/// Bilder für NUTZERBEFUNDE (N19): Liste ohne Tippen, gefiltert, ohne Treffer – Normal/Dunkel/OLED,
/// de/en (ganzes Fenster 1280 × 800). Nur auf Wunsch:
/// `cargo test -p fp-ui --test font_search_fv12 -- --ignored`.
#[test]
#[ignore]
fn n19_screenshots() {
    let _g = lock();
    wait_fonts();
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/screenshots/nutzerbefunde");
    for lang in ["de", "en"] {
        fp_i18n::set_language(lang);
        for (theme, tname) in [(ThemeKind::Normal, "normal"), (ThemeKind::Dark, "dunkel"), (ThemeKind::Oled, "oled")] {
            let mut h = harness_with(
                |s| {
                    let id = text_session(s);
                    s.execute("text.setAttributes", json!({"ids": [id], "font": "Verdana"})).expect("Verdana");
                },
                egui::vec2(1280.0, 800.0),
                true,
                theme,
            );
            h.run_steps(2);
            let combo = combo_with(&h, "Verdana");
            let steps: &[(&str, &str)] = if theme == ThemeKind::Normal { &[("liste", ""), ("gefiltert", "goe"), ("kein_treffer", "zzqx")] } else { &[("gefiltert", "goe"), ("kein_treffer", "zzqx")] };
            for (step, text) in steps {
                click(&mut h, combo.center());
                type_text(&mut h, text);
                h.run_steps(3);
                // §1a: keine Schriftnamen mit gesperrten Namen im Bild (installierte Fremdschriften)
                if rows(&h).iter().any(|(l, _)| !fp_neutral::find_in_bytes(l.as_bytes()).is_empty()) {
                    eprintln!("{step}: sichtbare Schrift mit gesperrtem Namen – Bild nicht gespeichert");
                    key(&mut h, Key::Escape);
                    continue;
                }
                match h.render() {
                    Ok(img) => {
                        let _ = img.save(out.join(format!("N19_schriftsuche_{step}_{tname}_{lang}.png")));
                    }
                    Err(e) => eprintln!("kein Grafikadapter: {e}"),
                }
                key(&mut h, Key::Escape);
                h.run_steps(2);
            }
        }
    }
}
