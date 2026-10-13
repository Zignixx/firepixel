//! PAR-X-BRUSH-1 in der Oberfläche (Einheiten 01/02, `docs\parity\tool-brush*.md`): Pinsel-Inspektor
//! zeigt die wirksame Werkzeugvorgabe (Rand nach Strichart-Wahl, Werte aus `pi.tool.brush`), Knopf
//! „Pinselstrich bearbeiten“ ganz lesbar, „Pinselstrich speichern“ aktiv, Strichart-Menü wie das
//! Original („Ohne“, schmal, Untermenü rechts daneben), Mischmodus-Namen, Pinselvorgabe in den
//! Voreinstellungen (Neustart), Dialogfelder mit „%“ (AGENTEN_REGELN §4b: 50× ändern, Abbrechen).

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_engine::tools::ToolId;
use fp_engine::Session;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn app_with(s: Session) -> FirepixelApp {
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.sync_render = true;
    a
}

fn session() -> Session {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 600, "height": 400, "background": "#ffffff"})).expect("Dokument");
    s.execute("select.none", json!({})).expect("nichts");
    s.set_colors(Some(fp_color::Argb(0xffc0_4040)), None);
    s
}

fn harness(a: FirepixelApp) -> H {
    Harness::builder().with_size(egui::vec2(1600.0, 1000.0)).with_pixels_per_point(1.0).with_max_steps(16).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a)
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn set_tool(h: &mut H, t: ToolId) {
    h.state_mut().state.session.set_tool(t).expect("Werkzeug");
    h.run_steps(3);
}

fn preset_index(name: &str) -> usize {
    fp_engine::commands::panel::find_preset(name).and_then(|f| fp_engine::commands::panel::BRUSH_PRESETS.iter().position(|b| b == f)).expect("Vorgabe")
}

fn click(h: &mut H, p: Pos2) {
    h.event(Event::PointerMoved(p));
    h.run_steps(1);
    h.event(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.run_steps(1);
    h.event(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

fn rect_of(h: &H, id: egui::Id) -> egui::Rect {
    h.ctx.read_response(id).unwrap_or_else(|| panic!("{id:?} nicht gezeichnet")).rect
}

/// 01/01: Strichart-Wahl bringt Rand und Texturmenge mit (vorher alter Wert), Werte aus
/// `pi.tool.brush` erscheinen im Inspektor; 50× wechseln = einmal mit dem Endwert.
#[test]
fn inspector_shows_effective_tool_brush() {
    let mut h = harness(app_with(session()));
    set_tool(&mut h, ToolId::Brush);
    let ctx = h.ctx.clone();
    fp_ui::inspector::obj_quads::apply_tool_stroke(&ctx, &mut h.state_mut().state, json!({"preset": "Einfach/Weich abger.", "width": 1}));
    h.run_steps(2);
    assert_eq!(fp_ui::inspector::obj_quads::tool_attrs(&ctx).softness, 50, "Weich abger.: Rand 50");
    for k in 0..50 {
        let p = if k % 2 == 0 { "Bleistift/Graphit" } else { "Unregelmäßig/Punkte" };
        fp_ui::inspector::obj_quads::apply_tool_stroke(&ctx, &mut h.state_mut().state, json!({"preset": p}));
        h.run_steps(1);
    }
    fp_ui::inspector::obj_quads::apply_tool_stroke(&ctx, &mut h.state_mut().state, json!({"preset": "Einfach/Präzise abger."}));
    h.run_steps(2);
    let ta = fp_ui::inspector::obj_quads::tool_attrs(&ctx);
    assert_eq!((ta.stroke_preset, ta.softness, ta.stroke_texture_amount), (Some(preset_index("Einfach/Präzise abger.")), 0, 0), "Präzise abger.: Rand 0 wie gemalt");
    let o = &st(&h).session.tools.options;
    assert_eq!(o.new_stroke.as_ref().and_then(|v| v.get("softness")).and_then(|v| v.as_i64()), Some(0), "Strichvorgabe = Anzeige");
    // Werte aus `pi.tool.brush` (Vorgabepinsel) zeigt der Inspektor
    h.state_mut().state.session.execute("pi.tool.brush", json!({"preset": "Einfach/Weich abger.", "width": 35, "softness": 37})).expect("pi.tool.brush");
    h.run_steps(3);
    let b = fp_engine::commands::panel::tool_brush(&st(&h).session.tools.options);
    assert_eq!((b.diameter, b.softness), (35, 370), "Vorgabepinsel");
    // Anzeige der Felder Größe/Strichart/Rand
    let shown = fp_ui::inspector::obj_quads::shown_tool_stroke(st(&h), &ctx);
    assert_eq!((shown.0, shown.1, shown.2), (35, fp_ui::inspector::texts::brush_preset_label(&fp_engine::commands::panel::BRUSH_PRESETS[preset_index("Einfach/Weich abger.")]), 37));
}

/// 01/01: „Pinselstrich bearbeiten“ steht ganz im Knopf (kein „…“); 02/15: „Pinselstrich
/// speichern“ ist im Werkzeug-Inspektor aktiv und speichert die Werkzeugvorgabe.
#[test]
fn edit_button_full_text_and_save_enabled() {
    let mut h = harness(app_with(session()));
    set_tool(&mut h, ToolId::Brush);
    let ctx = h.ctx.clone();
    fp_ui::inspector::obj_quads::apply_tool_stroke(&ctx, &mut h.state_mut().state, json!({"preset": "Einfach/Weich abger.", "width": 10}));
    h.run_steps(2);
    fp_ui::inspector::fields::set_audit(&ctx, true);
    h.run_steps(1);
    let audit = fp_ui::inspector::fields::take_audit(&ctx);
    fp_ui::inspector::fields::set_audit(&ctx, false);
    let text = fp_i18n::t("pi-stroke-edit");
    let a = audit.iter().find(|a| a.kind == "button" && a.text == text).expect("Knopf im Protokoll");
    let font = fp_ui::theme::Theme::current(&h.ctx).ui_font();
    let tw = h.ctx.fonts_mut(|f| f.layout_no_wrap(text.clone(), font.clone(), egui::Color32::WHITE).size().x);
    assert!(tw <= a.avail.width() + 0.5, "Text {tw} px passt in {} px (Original 122 px Knopf)", a.avail.width());
    assert!(a.drawn.width() <= a.avail.width() + 0.5, "„{text}“ passt ganz ({:?} in {:?})", a.drawn, a.avail);
    assert!(a.fit < 2, "nicht gekürzt");
    // Speichern aktiv
    let save = egui::Id::new(egui::Id::new(("pi-sedit", true)).with("save"));
    let r = h.ctx.read_response(save).expect("Symbolknopf Speichern");
    assert!(r.sense.senses_click(), "Pinselstrich speichern aktiv");
    click(&mut h, r.rect.center());
    let f = st(&h).dialogs.form.as_ref().expect("Speichern-Dialog");
    assert_eq!(f.command, "pi.brush.save");
    assert_eq!(f.base, json!({"tool": true}));
    // Wirkung (Befehl wie beim OK): Werkzeugvorgabe als gespeicherter Pinselstrich im Dokument
    h.state_mut().state.dialogs.form = None;
    h.state_mut().state.session.execute("pi.brush.save", json!({"name": "Mein Pinsel", "tool": true})).expect("speichern");
    h.run_steps(2);
    assert!(fp_ui::inspector::obj_quads::saved_brushes(st(&h)).contains(&"Mein Pinsel".to_string()));
}

/// 02/01: Strichart-Menü: „Ohne“, schmal (≈ 220 px statt ≈ 600), Untermenü rechts daneben.
#[test]
fn stroke_menu_like_original() {
    assert_eq!(fp_i18n::t("pi-stroke-cat-none"), "Ohne");
    let mut h = harness(app_with(session()));
    set_tool(&mut h, ToolId::Brush);
    let ctx = h.ctx.clone();
    fp_ui::inspector::obj_quads::apply_tool_stroke(&ctx, &mut h.state_mut().state, json!({"preset": "Einfach/Weich abger.", "width": 10}));
    h.run_steps(2);
    let cat = egui::Id::new(("pi-stroke", true)).with("cat");
    let combo = rect_of(&h, cat);
    click(&mut h, combo.center());
    let areas = |h: &H| -> Vec<egui::Rect> {
        h.ctx.memory(|m| m.areas().visible_layer_ids().into_iter().filter(|l| l.order == egui::Order::Foreground).filter_map(|l| m.area_rect(l.id)).collect())
    };
    let menus = areas(&h);
    let menu = *menus.iter().max_by(|a, b| a.height().total_cmp(&b.height())).expect("Menü offen");
    assert!(menu.width() < 320.0, "Menü schmal: {}", menu.width());
    // erste Kategorie: unter „Ohne“ und dem Trenner; Maus darüber öffnet das Untermenü rechts daneben
    let item_h = fp_ui::theme::Theme::current(&h.ctx).metrics.menu_item_height;
    let p = egui::pos2(menu.left() + 40.0, menu.top() + 2.0 + item_h + crate_sep() + item_h * 0.5);
    h.event(Event::PointerMoved(p));
    h.run_steps(4);
    let after = areas(&h);
    let sub = after.iter().find(|r| !menus.contains(r)).copied().expect("Untermenü offen");
    assert!(sub.left() >= menu.right() - 4.0, "Untermenü rechts daneben ({sub:?} / {menu:?})");
}

fn crate_sep() -> f32 {
    fp_ui::menu_scroll::SEPARATOR_H
}

/// 02/08: Mischmodus-Namen und Reihenfolge wie die Liste des Originals (46 Modi, Trenner
/// `-----`; `tool-brush-stroke-options.orig.md` Nr. 08).
#[test]
fn blend_names_like_original() {
    let orig = "Normal, Durchschnitt, Auflösen, -----, Abdunkeln, Multiplizieren, Farbig nachbelichten, Umgek. farb. nachb., Weich nachbelichten, Linear nachbelichten, -----, Aufhellen, Negativ multiplizieren, Farbig abwedeln, Umgek. farb. abw., Weich abwedeln, Linear abwedeln, -----, Überlagern, Weiches Licht, Unscharfes Licht, Hartes Licht, Strahlendes Licht, Lineares Licht, Lichtpunkt, Hart mischen, -----, Differenz, Ausschluss, Negation, -----, Farbton, Sättigung, Farbe, Luminanz, Rot, Grün, Blau, -----, Spiegeln, Glühen, Abkühlen, Erwärmen, -----, Additiv, Subtraktiv, Subtrahieren, Interpolation, Stempel, XOR, -----, Umkehren, Farbton, -----, Löschen";
    let want: Vec<&str> = orig.split(", ").collect();
    let names = fp_ui::inspector::texts::blend_labels();
    let got: Vec<&str> = names.iter().map(|s| if s.starts_with("---") { "-----" } else { s.as_str() }).collect();
    assert_eq!(got, want);
}

/// 02/18: Pinselvorgabe (Strichart, Größe, Rand) über den Neustart gemerkt.
#[test]
fn brush_tool_remembered_after_restart() {
    let mut h = harness(app_with(session()));
    set_tool(&mut h, ToolId::Brush);
    let ctx = h.ctx.clone();
    fp_ui::inspector::obj_quads::apply_tool_stroke(&ctx, &mut h.state_mut().state, json!({"preset": "Einfach/Weich abger.", "width": 1}));
    h.run_steps(3);
    fp_ui::inspector::obj_quads::apply_tool_stroke(&ctx, &mut h.state_mut().state, json!({"softness": 100}));
    h.run_steps(3);
    let prefs = st(&h).session.prefs.clone();
    assert_eq!(prefs.get_str(fp_ui::canvas::overlay::BRUSH_PREF_STROKE, ""), "bc_Basic/bn_Soft Rounded");
    assert_eq!(prefs.get_int(fp_ui::canvas::overlay::BRUSH_PREF_SOFT, -1), 100);
    // „Neustart“: neue Sitzung mit diesen Voreinstellungen, neue Oberfläche
    let mut s = Session::new(prefs);
    s.execute("file.new", json!({"width": 600, "height": 400, "background": "#ffffff"})).expect("Dokument");
    s.set_colors(Some(fp_color::Argb(0xffc0_4040)), None);
    let mut h2 = harness(app_with(s));
    set_tool(&mut h2, ToolId::Pointer);
    set_tool(&mut h2, ToolId::Brush);
    h2.run_steps(2);
    let ta = fp_ui::inspector::obj_quads::tool_attrs(&h2.ctx);
    assert_eq!((ta.stroke_preset, ta.stroke_width, ta.softness), (Some(preset_index("Einfach/Weich abger.")), 1, 100));
    assert_eq!(st(&h2).session.tools.options.brush_size, 1.0);
    let b = fp_engine::commands::panel::tool_brush(&st(&h2).session.tools.options);
    assert_eq!((b.name.as_str(), b.softness), ("bn_Soft Rounded", 1000), "gemalt wird mit der gemerkten Vorgabe");
}
