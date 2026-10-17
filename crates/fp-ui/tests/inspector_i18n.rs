//! Inspektor mehrsprachig (Phase 6): alle Inspektor-Layouts je Elementart und je Werkzeug in
//! Deutsch, Englisch und der Testsprache `qps` (Texte markiert und ~40 % länger). Geprüft über das
//! Layout-Protokoll der Felder (`fields::set_audit`):
//! - keine Rohschlüssel (`pi-…`, `blend-…`) in der Oberfläche,
//! - keine Beschriftung ragt aus ihrer Fläche, aus dem Quadranten oder in ein Feld bzw. eine andere
//!   Beschriftung,
//! - Deutsch sieht aus wie bisher (kein Einpassen; bekannte Mängel von vorher in `KNOWN_DE`),
//!   Englisch wird nicht gekürzt,
//! - in `qps` ist jede feste Beschriftung markiert (kein fest eingebauter Text mehr).

use egui_kittest::Harness;
use fp_engine::Session;
use fp_ui::app_state::UiSettings;
use fp_ui::inspector::fields::{self, TextAudit};
use fp_ui::FirepixelApp;
use serde_json::json;

fn ex(s: &mut Session, name: &str, p: serde_json::Value) {
    s.execute(name, p).unwrap_or_else(|e| panic!("{name}: {e}"));
}

fn new_doc() -> Session {
    let mut s = Session::default();
    ex(&mut s, "file.new", json!({"width": 400, "height": 300}));
    s
}

fn app(s: Session) -> FirepixelApp {
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.sync_render = true;
    a
}

fn harness(a: FirepixelApp) -> Harness<'static, FirepixelApp> {
    Harness::builder().with_size(egui::vec2(1800.0, 1130.0)).with_pixels_per_point(1.0).with_max_steps(16).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a)
}

/// Dokumente mit je einer Elementart.
fn cases() -> Vec<(&'static str, Session)> {
    let mut v = Vec::new();
    v.push(("Dokument", new_doc()));
    let mut s = new_doc();
    ex(&mut s, "draw.rectangle", json!({"rect": [20, 20, 120, 80]}));
    ex(&mut s, "pi.stroke", json!({"width": 3, "color": "#0000ff"}));
    v.push(("Rechteck", s));
    let mut s = new_doc();
    ex(&mut s, "draw.ellipse", json!({"rect": [20, 20, 120, 80]}));
    ex(&mut s, "pi.stroke", json!({"width": 2, "color": "#000000"}));
    ex(&mut s, "element.addEffect", json!({"effect": "dropShadow"}));
    v.push(("Pfad mit Effekt", s));
    let mut s = new_doc();
    ex(&mut s, "draw.text", json!({"text": "Hallo", "anchor": [30, 60], "size": 24}));
    v.push(("Text", s));
    let mut s = new_doc();
    ex(&mut s, "draw.ellipse", json!({"rect": [20, 20, 120, 80]}));
    ex(&mut s, "modify.flattenSelection", json!({}));
    v.push(("Bitmap", s));
    let mut s = new_doc();
    ex(&mut s, "draw.ellipse", json!({"rect": [20, 20, 120, 80]}));
    ex(&mut s, "draw.ellipse", json!({"rect": [140, 20, 220, 80]}));
    ex(&mut s, "select.all", json!({}));
    ex(&mut s, "modify.group", json!({}));
    v.push(("Gruppe", s));
    let mut s = new_doc();
    ex(&mut s, "draw.ellipse", json!({"rect": [20, 20, 120, 80]}));
    ex(&mut s, "draw.ellipse", json!({"rect": [60, 20, 160, 80]}));
    ex(&mut s, "select.all", json!({}));
    v.push(("Mehrere", s));
    let mut s = new_doc();
    ex(&mut s, "draw.ellipse", json!({"rect": [20, 20, 120, 80]}));
    ex(&mut s, "modify.symbol.convertToSymbol", json!({"name": "S", "kind": "graphic"}));
    v.push(("Grafiksymbol", s));
    let mut s = new_doc();
    ex(&mut s, "draw.ellipse", json!({"rect": [20, 20, 120, 80]}));
    ex(&mut s, "modify.symbol.convertToSymbol", json!({"name": "B", "kind": "button"}));
    v.push(("Schaltfläche", s));
    let mut s = new_doc();
    ex(&mut s, "draw.ellipse", json!({"rect": [20, 20, 120, 80]}));
    ex(&mut s, "modify.symbol.convertToSymbol", json!({"name": "A", "kind": "animation"}));
    v.push(("Animation", s));
    for ty in 0..3 {
        let mut s = new_doc();
        ex(&mut s, "web.addSlice", json!({"shape": "rect", "rect": [10, 10, 90, 60]}));
        ex(&mut s, "pi.web", json!({"sliceType": ty}));
        v.push((["Segment Bild", "Segment Hintergrund", "Segment HTML"][ty], s));
    }
    let mut s = new_doc();
    ex(&mut s, "web.addHotspot", json!({"shape": "rect", "rect": [10, 10, 90, 60]}));
    v.push(("Hotspot", s));
    // Maske bearbeiten (Pinsel): Masken-Teilfeld sichtbar
    let mut s = new_doc();
    ex(&mut s, "draw.rectangle", json!({"rect": [20, 20, 120, 80]}));
    ex(&mut s, "draw.ellipse", json!({"rect": [30, 30, 100, 70]}));
    ex(&mut s, "select.all", json!({}));
    ex(&mut s, "modify.mask.groupAsMask", json!({}));
    ex(&mut s, "panel.layers.editMask", json!({}));
    s.set_tool(fp_engine::ToolId::Brush).expect("Pinsel");
    v.push(("Maske bearbeiten (Pinsel)", s));
    v
}

/// Die Sprache ist prozessweit: Tests dieser Datei laufen nacheinander.
static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Bekannte Mängel der deutschen Darstellung, die schon vor Phase 6 bestanden (Regel „Deutsch
/// unverändert“, daher nicht geändert; Befund an den Koordinator): (Fall, Text).
const KNOWN_DE: &[(&str, &str)] = &[
    // RAND-2a: „Bearbeiten...“/„Ziel:“ beim Segment und „Autom. Unterschneidung“ behoben
    ("Gruppe", "Gruppieren (2)"),
    ("Segment Hintergrund", "Horizontale Position:"),
];

const TEXT_KINDS: [&str; 5] = ["label", "name", "check", "radio", "button"];

fn is_raw_key(s: &str) -> bool {
    let s = s.trim_start_matches("[!! ");
    let word = s.split_whitespace().next().unwrap_or("");
    ["pi-", "blend-", "tool-", "cmd-", "btn-", "fmt-", "filter-", "effect-", "panel-", "dlg-"].iter().any(|p| word.starts_with(p)) && word.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
}

/// Ein Bild zeichnen und das Protokoll des Inspektors holen (ohne Doppelungen aus Mehrfachdurchläufen).
fn audit(h: &mut Harness<'_, FirepixelApp>) -> Vec<TextAudit> {
    h.run_steps(2);
    fields::set_audit(&h.ctx, true);
    h.run_steps(1);
    let mut v: Vec<TextAudit> = fields::take_audit(&h.ctx).into_iter().filter(|e| e.inspector).collect();
    fields::set_audit(&h.ctx, false);
    let mut out: Vec<TextAudit> = Vec::new();
    for e in v.drain(..) {
        if !out.contains(&e) {
            out.push(e);
        }
    }
    out
}

fn shrink(r: egui::Rect, x: f32, y: f32) -> egui::Rect {
    r.shrink2(egui::vec2(x, y))
}

/// Prüft ein Protokoll; `de_overflow` = Überstand der Kombifeld-Einträge im Deutschen (je Feld).
fn check(lang: &str, case: &str, log: &[TextAudit], de_overflow: &mut std::collections::HashMap<String, f32>, errors: &mut Vec<String>) {
    let fields_r: Vec<egui::Rect> = log.iter().filter(|e| e.kind == "field").map(|e| e.avail).collect();
    let texts: Vec<&TextAudit> = log.iter().filter(|e| TEXT_KINDS.contains(&e.kind)).collect();
    let known = |e: &TextAudit| lang == "de" && KNOWN_DE.iter().any(|(c, t)| *c == case && *t == e.text);
    for e in log.iter().filter(|e| e.kind != "field") {
        let at = format!("[{lang}] {case}: {} „{}“", e.kind, e.text);
        if known(e) {
            eprintln!("Hinweis {at}: bekannter Mangel vor Phase 6 (Deutsch unverändert)");
            continue;
        }
        if is_raw_key(&e.text) {
            errors.push(format!("{at}: Rohschlüssel"));
        }
        if e.kind == "combo-item" {
            let key = format!("{case}|{:?}", e.avail.min);
            let over = (e.drawn.width() - e.avail.width()).max(0.0);
            if lang == "de" {
                let m = de_overflow.entry(key).or_insert(0.0);
                *m = m.max(over);
            } else if lang == "en" {
                let allowed = de_overflow.get(&key).copied().unwrap_or(0.0);
                if over > allowed + 1.0 {
                    errors.push(format!("{at}: Eintrag passt nicht ins Kombifeld ({:.0} px zu breit)", over));
                }
            }
            continue;
        }
        // Text innerhalb seiner Fläche und des Quadranten
        let d = e.drawn;
        if d.left() < e.avail.left() - 1.0 || d.right() > e.avail.right() + 1.0 {
            errors.push(format!("{at}: ragt aus der Fläche ({:.0}–{:.0} statt {:.0}–{:.0})", d.left(), d.right(), e.avail.left(), e.avail.right()));
        }
        if d.left() < e.clip.left() - 1.0 || d.right() > e.clip.right() + 1.0 {
            errors.push(format!("{at}: vom Quadranten abgeschnitten"));
        }
        match lang {
            // Deutsch: höchstens leicht verkleinert (nur wo der Text schon vorher nicht in sein Feld passte)
            "de" if e.fit != 0 => errors.push(format!("{at}: im Deutschen verkleinert/gekürzt (Deutsch muss aussehen wie bisher)")),
            "en" if e.fit == 2 => errors.push(format!("{at}: im Englischen gekürzt")),
            "qps" if e.kind != "name" && !e.text.starts_with("[!!") && !e.text.trim().is_empty() => errors.push(format!("{at}: fester, nicht übersetzter Text")),
            _ => {}
        }
        // keine Überlappung mit Feldern oder anderen Beschriftungen
        let core = shrink(d, 1.0, 3.0);
        if core.width() <= 0.0 || core.height() <= 0.0 {
            continue;
        }
        let own = if e.kind == "button" { Some(e.avail.expand2(egui::vec2(3.0, 0.0))) } else { None };
        for f in &fields_r {
            if own.is_some_and(|o| o.contains_rect(*f) && f.contains_rect(o.shrink(0.5))) {
                continue;
            }
            if e.kind == "button" && f.contains_rect(core) {
                continue;
            }
            if shrink(*f, 1.0, 1.0).intersects(core) {
                errors.push(format!("{at}: überlappt ein Feld bei {:?}", f.min));
                break;
            }
        }
        for o in &texts {
            if std::ptr::eq(*o, e) || known(o) {
                continue;
            }
            if shrink(o.drawn, 1.0, 3.0).intersects(core) {
                errors.push(format!("{at}: überlappt „{}“", o.text));
                break;
            }
        }
    }
}

/// Schmales Fenster (800 × 600): Die Quadranten behalten ihre Breite, der Inspektor lässt sich
/// waagerecht verschieben; jede Beschriftung, die links nicht zu sehen ist, ist nach dem Bildlauf
/// ganz sichtbar (vorher wurden Quadrant 3/4 abgeschnitten).
#[test]
fn inspector_narrow_window_scrolls() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    fp_i18n::set_language("de");
    let mut s = new_doc();
    ex(&mut s, "draw.text", json!({"text": "Hallo", "anchor": [30, 60], "size": 24}));
    let mut h = Harness::builder().with_size(egui::vec2(800.0, 600.0)).with_pixels_per_point(1.0).with_max_steps(16).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app(s));
    let before = audit(&mut h);
    let visible = |e: &TextAudit| e.drawn.left() >= e.clip.left() - 1.0 && e.drawn.right() <= e.clip.right() + 1.0;
    let hidden: Vec<(&'static str, String)> = before.iter().filter(|e| TEXT_KINDS.contains(&e.kind) && !visible(e)).map(|e| (e.kind, e.text.clone())).collect();
    assert!(!hidden.is_empty(), "bei 800 px sollte rechts etwas außerhalb liegen");
    // Mausrad waagerecht über dem Inspektor
    let over = before.iter().find(|e| e.kind == "field").map(|e| e.avail.center()).expect("Feld im Inspektor");
    h.hover_at(over);
    h.run_steps(1);
    // in kleinen Schritten nach rechts; jede vorher verdeckte Beschriftung muss irgendwann ganz zu sehen sein
    let mut seen: Vec<(&'static str, String)> = Vec::new();
    for _ in 0..24 {
        h.event(egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Point, delta: egui::vec2(-60.0, 0.0), phase: egui::TouchPhase::Move, modifiers: egui::Modifiers::NONE });
        h.run_steps(4);
        for e in audit(&mut h).iter().filter(|e| TEXT_KINDS.contains(&e.kind) && visible(e)) {
            if !seen.iter().any(|(k, t)| *k == e.kind && *t == e.text) {
                seen.push((e.kind, e.text.clone()));
            }
        }
    }
    let never: Vec<&String> = hidden.iter().filter(|h| !seen.contains(h)).map(|h| &h.1).collect();
    assert!(never.is_empty(), "auch mit Bildlauf nie ganz sichtbar: {never:?}");
}

#[test]
fn inspector_layouts_in_all_languages() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut errors = Vec::new();
    let mut de_overflow = std::collections::HashMap::new();
    let mut counts = Vec::new();
    for lang in ["de", "en", "qps"] {
        assert!(fp_i18n::set_language(lang), "{lang}");
        let mut n = 0usize;
        for (name, s) in cases() {
            let mut h = harness(app(s));
            let log = audit(&mut h);
            assert!(log.iter().any(|e| e.kind != "field"), "[{lang}] {name}: keine Beschriftungen im Protokoll");
            n += log.len();
            check(lang, name, &log, &mut de_overflow, &mut errors);
        }
        // jedes Werkzeug (Werkzeug-Layouts)
        let mut h = harness(app(new_doc()));
        h.run_steps(2);
        for t in fp_engine::tools::TOOLS {
            if h.state_mut().state.session.set_tool(t.id).is_err() {
                continue;
            }
            let log = audit(&mut h);
            n += log.len();
            check(lang, &format!("Werkzeug {}", t.name), &log, &mut de_overflow, &mut errors);
        }
        // Verlaufs-Editor (Popup am Füllfeld eines Rechtecks mit Verlauf)
        let mut s = new_doc();
        ex(&mut s, "draw.rectangle", json!({"rect": [20, 20, 120, 80]}));
        ex(&mut s, "pi.fill", json!({"category": "gradient"}));
        let mut h = harness(app(s));
        let log = audit(&mut h);
        let edge = fp_i18n::t("pi-edge");
        let q3 = log.iter().filter(|e| e.kind == "label" && e.text == edge).map(|e| e.clip.min).min_by(|a, b| a.x.total_cmp(&b.x)).expect("Füllung: Beschriftung Rand");
        // Füllfeld: DLU (1 + 14 … 29, 0 … 13) im Quadranten 3
        let at = q3 + egui::vec2((1.0 + 21.5) * 1.5, 6.5 * 1.625);
        h.hover_at(at);
        h.run_steps(1);
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE });
            h.run_steps(1);
        }
        let log = audit(&mut h);
        let reverse = fp_i18n::t("pi-grad-reverse");
        assert!(log.iter().any(|e| e.kind == "button" && e.text == reverse), "[{lang}] Verlaufs-Editor nicht geöffnet");
        n += log.len();
        check(lang, "Verlaufs-Editor", &log, &mut de_overflow, &mut errors);
        counts.push(format!("{lang}: {n} Einträge"));
    }
    fp_i18n::set_language("de");
    eprintln!("{}", counts.join(", "));
    errors.sort();
    errors.dedup();
    assert!(errors.is_empty(), "{} Befunde:\n{}", errors.len(), errors.join("\n"));
}
