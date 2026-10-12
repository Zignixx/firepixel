//! Nutzerbefund: Dialoge, Meldungen und modale Fenster lassen sich an der Titelleiste frei
//! verschieben. Für jeden Dialog, 2 Hauptfenstergrößen (1500 × 950, 800 × 600) und alle
//! 3 Designs: jedes Öffnen zentriert (wie im Original), Ziehen verschiebt um den Mausweg,
//! Leinwand/Dokument bleiben unverändert, die Titelleiste bleibt im Hauptfenster;
//! größenveränderliche Dialoge lassen sich an Rand und Ecke aufziehen.

use egui::{pos2, vec2, Pos2, Rect};
use egui_kittest::Harness;
use fp_engine::Session;
use fp_ui::app_state::UiSettings;
use fp_ui::dialogs::{dialog_rect, last_dialog_id, DIALOG_KEEP_VISIBLE, DIALOG_TITLE_H};
use fp_ui::theme::ThemeKind;
use fp_ui::FirepixelApp;
use serde_json::json;
use std::sync::Arc;

fn session() -> Session {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 80]})).expect("rect");
    s.execute("draw.text", json!({"text": "Hallo Wlet", "anchor": [30, 150], "size": 20})).expect("text");
    s.execute("draw.ellipse", json!({"rect": [150, 20, 250, 90]})).expect("ellipse");
    s.execute("modify.symbol.convertToSymbol", json!({"name": "Sym"})).expect("symbol");
    s.execute("draw.rectangle", json!({"rect": [160, 120, 260, 180]})).expect("rect2");
    s.execute("pi.style.new", json!({"name": "Stil 1"})).expect("style");
    s
}

fn harness(size: egui::Vec2, theme: ThemeKind) -> Harness<'static, FirepixelApp> {
    let ctx = egui::Context::default();
    let settings = UiSettings { theme, ..UiSettings::default() };
    let mut a = FirepixelApp::with_session(&ctx, session(), settings, false);
    a.state.sync_render = true;
    let tmp = std::env::temp_dir().join("fp-ibd-dialogs-move");
    let _ = std::fs::create_dir_all(&tmp);
    a.state.file_override = Some(fp_ui::files::FilePickOverride { open: vec![tmp.join("nicht-vorhanden.png")], save: Some(tmp.join("ausgabe.png")) });
    a.state.dialogs.panel_sel.styles = vec!["Stil 1".into()];
    a.state.dialogs.panel_sel.symbol = Some(0);
    Harness::builder().with_size(size).with_pixels_per_point(1.0).with_max_steps(16).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a)
}

/// Dialoge über ihre Befehle (wie `dialogs_open.rs`) plus Meldungen.
const DIALOG_COMMANDS: &[&str] = &[
    "modify.canvas.canvasSize",
    "modify.canvas.imageSize",
    "modify.transform.numericTransform",
    "filter.adjustColor.levels",
    "filter.adjustColor.curves",
    "file.export",
    "file.imagePreview",
    "edit.keyboardShortcuts",
    "text.spellingSetup",
    "text.checkSpelling",
    "modify.popUpMenu.addPopUpMenu",
    "filter.blur.gaussian",
    "filter.blur.motion",
    "filter.sharpen.unsharpMask",
    "filter.noise.addNoise",
    "filter.adjustColor.brightnessContrast",
    "filter.adjustColor.hueSaturation",
    "select.feather",
    "select.expandMarquee",
    "select.saveBitmapSelection",
    "modify.alterPath.simplify",
    "modify.alterPath.insetPath",
    "modify.alterPath.expandStroke",
    "text.size.other",
    "modify.symbol.tweenInstances",
    "modify.symbol.convertToSymbol",
    "modify.animation.animateSelection",
    "modify.animation.settings",
    "modify.canvas.canvasColor",
    "view.grid.editGrid",
    "view.guides.editGuides",
    "edit.insert.layer",
    "edit.insert.state",
    "file.exportWizard",
    "file.htmlSetup",
    "modify.selectiveJpeg.settings",
    "file.pageSetup",
    "file.print",
    "file.batchProcess",
    "modify.symbol.swapSymbol",
    "edit.insert.newSymbol",
    "edit.insert.newButton",
    "commands.manageSavedCommands",
    "panel.layers.newLayer",
    "panel.layers.duplicate",
    "panel.layers.shareToPages",
    "panel.layers.thumbnailOptions",
    "panel.states.add",
    "panel.states.properties",
    "panel.pages.rename",
    "panel.styles.rename",
    "panel.library.properties",
    "panel.optimize.saveSettings",
    "panel.optimize.optimizeToSize",
    "panel.effects.saveAsStyle",
    "@about",
    "@message",
    "@stroke",
];

/// Zustand, der sich durch Ziehen im Dialog nicht ändern darf (Dokument, Auswahl, Verlauf, Werkzeug).
fn canvas_state(h: &Harness<'_, FirepixelApp>) -> (Arc<fp_model::Document>, Vec<fp_model::ElementId>, usize, String) {
    let st = &h.state().state;
    let d = st.session.active().expect("doc");
    (Arc::clone(&d.doc), d.selection.elements.clone(), d.history.undo_count(), format!("{:?}", st.session.tools.active))
}

fn open(h: &mut Harness<'_, FirepixelApp>, name: &str) {
    match name {
        "@about" => h.state_mut().state.dialogs.about = true,
        "@message" => fp_ui::dialogs::misc::not_available_message(&mut h.state_mut().state, "Hinweis", "Testmeldung"),
        "@stroke" => h.state_mut().state.dialogs.open_stroke_options(true),
        _ => h.state_mut().state.run(name, json!({})),
    }
    h.run_steps(3);
}

fn drag(h: &mut Harness<'_, FirepixelApp>, from: Pos2, to: Pos2) {
    h.event(egui::Event::PointerMoved(from));
    h.run_steps(1);
    h.event(egui::Event::PointerButton { pos: from, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() });
    h.run_steps(1);
    for i in 1..=5 {
        let p = from + (to - from) * (i as f32 / 5.0);
        h.event(egui::Event::PointerMoved(p));
        h.run_steps(1);
    }
    h.event(egui::Event::PointerButton { pos: to, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() });
    h.run_steps(2);
}

fn current(h: &Harness<'_, FirepixelApp>) -> (String, Rect) {
    let ctx = &h.ctx;
    let id = last_dialog_id(ctx).expect("Dialograhmen gezeichnet");
    let r = dialog_rect(ctx, &id).unwrap_or_else(|| panic!("{id}: kein Bereich"));
    (id, r)
}

/// Titelleiste im Hauptfenster: oben nicht darüber, ganz über dem unteren Rand, waagerecht ≥ 80 px.
fn assert_visible(name: &str, r: Rect, screen: Rect) {
    assert!(r.top() >= screen.top() - 0.5, "{name}: Titelleiste oben außerhalb {r:?}");
    assert!(r.top() + DIALOG_TITLE_H <= screen.bottom() + 0.5, "{name}: Titelleiste unten außerhalb {r:?}");
    let keep = DIALOG_KEEP_VISIBLE.min(r.width()) - 1.0;
    assert!(r.right() >= screen.left() + keep && r.left() <= screen.right() - keep, "{name}: Titelleiste seitlich außerhalb {r:?}");
}

fn run_all(size: egui::Vec2, theme: ThemeKind) {
    let mut h = harness(size, theme);
    h.run_steps(2);
    let screen = Rect::from_min_size(Pos2::ZERO, size);
    let mut checked = 0;
    let mut not_opened = Vec::new();
    for name in DIALOG_COMMANDS {
        // Lage vom vorigen Dialog darf nicht mitwandern: Kennung zurücksetzen
        h.ctx.data_mut(|d| d.remove::<String>(egui::Id::new("fp-last-dialog")));
        open(&mut h, name);
        if !h.state().state.dialogs.any_open() || last_dialog_id(&h.ctx).is_none() {
            not_opened.push(*name);
            continue;
        }
        // Zustand nach dem Öffnen (Dialoge mit Live-Vorschau ändern das Dokument schon beim Öffnen)
        let before = canvas_state(&h);
        let (id, r0) = current(&h);
        // jedes Öffnen: zentriert wie im Original (soweit die Titelleiste im Fenster bleibt)
        if r0.height() <= size.y && r0.width() <= size.x {
            assert!((r0.center() - screen.center()).length() < 3.0, "{name}: nicht zentriert {r0:?}");
        }
        assert_visible(name, r0, screen);
        // Titelleiste ziehen
        let grab = pos2(r0.left() + 30.0, r0.top() + 12.0);
        let delta = vec2(-60.0, 40.0);
        drag(&mut h, grab, grab + delta);
        assert!(h.state().state.dialogs.any_open(), "{name}: Ziehen hat den Dialog geschlossen");
        let (_, r1) = current(&h);
        assert!(((r1.min - r0.min) - delta).length() < 1.5, "{name} ({id}): um {:?} statt {delta:?} verschoben", r1.min - r0.min);
        assert_visible(name, r1, screen);
        // weit hinaus ziehen: Titelleiste bleibt sichtbar
        let grab = pos2(r1.left() + 30.0, r1.top() + 12.0);
        drag(&mut h, grab, grab + vec2(3000.0, 3000.0));
        let (_, r2) = current(&h);
        assert_visible(name, r2, screen);
        let grab = pos2(r2.left() + 30.0, r2.top() + 12.0).clamp(screen.min, screen.max - vec2(1.0, 1.0));
        drag(&mut h, grab, pos2(-3000.0, -3000.0));
        let (_, r3) = current(&h);
        assert_visible(name, r3, screen);
        // an eine bekannte Stelle ziehen (die Lage gilt, solange der Dialog offen ist)
        let grab = pos2(r3.left().max(0.0) + 30.0, r3.top() + 12.0);
        let target = pos2(40.0, 30.0);
        drag(&mut h, grab, grab + (target - r3.min));
        let (_, r4) = current(&h);
        assert!((r4.min - target).length() < 1.5, "{name}: Lage {:?} statt {target:?}", r4.min);
        // Leinwand/Dokument unverändert
        let after = canvas_state(&h);
        assert!(Arc::ptr_eq(&before.0, &after.0) || *before.0 == *after.0, "{name}: Ziehen hat das Dokument geändert");
        assert_eq!((before.1.clone(), before.2, before.3.clone()), (after.1, after.2, after.3), "{name}: Auswahl/Verlauf/Werkzeug geändert");
        h.key_press(egui::Key::Escape);
        h.run_steps(3);
        assert!(!h.state().state.dialogs.any_open(), "{name}: Esc schließt nicht");
        if !name.starts_with('@') || *name == "@about" {
            open(&mut h, name);
            if h.state().state.dialogs.any_open() {
                let (id2, r5) = current(&h);
                // Original: beim erneuten Öffnen wieder mittig (UX-B D21)
                if id2 == id && r5.height() <= size.y && r5.width() <= size.x {
                    assert!((r5.center() - screen.center()).length() < 3.0, "{name}: beim Wiederöffnen nicht zentriert {r5:?}");
                }
                h.key_press(egui::Key::Escape);
                h.run_steps(3);
            }
        }
        checked += 1;
    }
    assert!(checked >= DIALOG_COMMANDS.len() - 3, "{theme:?} {size:?}: nur {checked} geprüft, ohne Dialog: {not_opened:?}");
}

#[test]
fn dialogs_move_normal_large_and_small() {
    run_all(vec2(1500.0, 950.0), ThemeKind::Normal);
    run_all(vec2(800.0, 600.0), ThemeKind::Normal);
}

#[test]
fn dialogs_move_dark_large_and_small() {
    run_all(vec2(1500.0, 950.0), ThemeKind::Dark);
    run_all(vec2(800.0, 600.0), ThemeKind::Dark);
}

#[test]
fn dialogs_move_oled_large_and_small() {
    run_all(vec2(1500.0, 950.0), ThemeKind::Oled);
    run_all(vec2(800.0, 600.0), ThemeKind::Oled);
}

/// Größenveränderliche Dialoge (Bildvorschau): rechte untere Ecke und linker Rand ziehen.
#[test]
fn resizable_dialog_resizes_at_edges() {
    let mut h = harness(vec2(1500.0, 950.0), ThemeKind::Normal);
    h.run_steps(2);
    let before = canvas_state(&h);
    open(&mut h, "file.imagePreview");
    let (id, r0) = current(&h);
    assert_eq!(id, "fp-imgpreview");
    drag(&mut h, r0.max - vec2(3.0, 3.0), r0.max + vec2(120.0, 80.0));
    let (_, r1) = current(&h);
    assert!((r1.size() - r0.size() - vec2(120.0, 80.0)).length() < 5.0, "Ecke: {:?} → {:?}", r0.size(), r1.size());
    assert!((r1.min - r0.min).length() < 1.0, "Ecke rechts unten verschiebt nicht");
    // linker Rand: nach links größer, rechte Kante bleibt
    let y = r1.center().y;
    drag(&mut h, pos2(r1.left() + 2.0, y), pos2(r1.left() - 50.0, y));
    let (_, r2) = current(&h);
    assert!((r2.width() - r1.width() - 50.0).abs() < 5.0 && (r2.right() - r1.right()).abs() < 1.5, "linker Rand: {r1:?} → {r2:?}");
    // kleiner als die Mindestgröße geht nicht
    drag(&mut h, r2.max - vec2(3.0, 3.0), r2.min + vec2(10.0, 10.0));
    let (_, r3) = current(&h);
    assert!(r3.width() >= 820.0 && r3.height() >= 520.0, "Mindestgröße: {r3:?}");
    let after = canvas_state(&h);
    assert!(*before.0 == *after.0 && before.2 == after.2, "Größe ändern berührt das Dokument nicht");
    // nicht größenveränderlich: Bildgröße behält die Größe
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    open(&mut h, "modify.canvas.imageSize");
    let (_, s0) = current(&h);
    drag(&mut h, s0.max - vec2(3.0, 3.0), s0.max + vec2(100.0, 100.0));
    let (_, s1) = current(&h);
    assert!((s1.size() - s0.size()).length() < 1.0, "Bildgröße ist fest");
}

/// Nutzerbefund N02 (PM-1): im echten Fenster folgte „Bildgröße“ nur zu ~80 % der Maus, weil die
/// Bewegung bis zum Erkennen des Ziehens verloren ging (Drücken und erste Bewegungen kommen oft
/// gebündelt in einem Bild an). Erwartet wie bei einem Windows-Dialog: Der gegriffene Punkt bleibt
/// unter dem Zeiger, Versatz = Mausweg – bei 100 % und 150 % Skalierung, auch mit Rückweg.
#[test]
fn drag_follows_pointer_one_to_one() {
    for ppp in [1.0_f32, 1.5] {
        for batched in [0usize, 1, 5] {
            let ctx = egui::Context::default();
            let mut a = FirepixelApp::with_session(&ctx, session(), UiSettings::default(), false);
            a.state.sync_render = true;
            let mut h = Harness::builder()
                .with_size(vec2(1500.0, 950.0))
                .with_pixels_per_point(ppp)
                .with_max_steps(16)
                .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a);
            h.run_steps(2);
            open(&mut h, "modify.canvas.imageSize");
            let (_, r0) = current(&h);
            let grab = pos2(r0.left() + 40.0, r0.top() + 12.0);
            let delta = vec2(-160.0, 90.0);
            const STEPS: usize = 25;
            let at = |i: usize| grab + delta * (i as f32 / STEPS as f32);
            h.event(egui::Event::PointerMoved(grab));
            h.run_steps(1);
            // Drücken und die ersten `batched` Bewegungen im selben Bild
            h.event(egui::Event::PointerButton { pos: grab, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() });
            for i in 1..=batched {
                h.event(egui::Event::PointerMoved(at(i)));
            }
            h.run_steps(1);
            for i in batched + 1..=STEPS {
                h.event(egui::Event::PointerMoved(at(i)));
                h.run_steps(1);
                let (_, r) = current(&h);
                // nach dem Erkennen des Ziehens liegt der gegriffene Punkt genau unter dem Zeiger
                if i > batched + 1 {
                    assert!(((r.min - r0.min) - (at(i) - grab)).length() < 1.01, "ppp {ppp}, gebündelt {batched}, Schritt {i}: Versatz {:?} statt {:?}", r.min - r0.min, at(i) - grab);
                }
            }
            // Rückweg im selben Zug
            for i in 1..=10 {
                h.event(egui::Event::PointerMoved(at(STEPS) - delta * 0.5 * (i as f32 / 10.0)));
                h.run_steps(1);
            }
            let end = at(STEPS) - delta * 0.5;
            h.event(egui::Event::PointerButton { pos: end, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() });
            h.run_steps(2);
            let (_, r1) = current(&h);
            assert!(((r1.min - r0.min) - (end - grab)).length() < 1.01, "ppp {ppp}, gebündelt {batched}: Endlage {:?} statt {:?}", r1.min - r0.min, end - grab);
            // zweites Ziehen setzt mit neuem Griffpunkt an (kein Sprung)
            let g2 = pos2(r1.left() + 100.0, r1.top() + 8.0);
            drag(&mut h, g2, g2 + vec2(30.0, -20.0));
            let (_, r2) = current(&h);
            assert!(((r2.min - r1.min) - vec2(30.0, -20.0)).length() < 1.01, "ppp {ppp}: zweites Ziehen {:?}", r2.min - r1.min);
        }
    }
}
