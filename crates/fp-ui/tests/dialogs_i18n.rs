//! Mehrsprachigkeit der Dialoge (Phase 6, I18N-DIALOGE): Jeder Dialog wird in Deutsch, Englisch
//! und der Testsprache `qps` (Texte markiert und um ≈ 40 % verlängert) geöffnet und an den
//! gezeichneten Texten geprüft:
//! - nichts überlappt (Text-Rechtecke paarweise), nichts ragt aus dem Dialog,
//! - kein Knopf und keine Beschriftung meldet einen Überlauf (`dialogs::take_overflows`),
//! - kein Rohschlüssel ist sichtbar (Text = Schlüssel, z. B. `dlg-…`),
//! - in Deutsch bleiben Wortlaut und Maße wie bisher.
//! Gezeichnet werden nur die Dialoge (ohne Hauptfenster), damit dessen Texte nicht mitzählen.
//! Wie alle Testläufe mit `FP_NO_SHELL_OPEN=1` starten (AGENTEN_REGELN §4a).

use egui::epaint::{Shape, TextShape};
use egui::{Pos2, Rect};
use egui_kittest::Harness;
use fp_ui::app_state::UiSettings;
use fp_ui::dialogs::{self, Dialogs};
use fp_ui::FirepixelApp;
use serde_json::json;
use std::sync::Mutex;

/// Die Sprache ist global: Tests dieser Datei laufen nacheinander.
static LOCK: Mutex<()> = Mutex::new(());

struct S {
    app: FirepixelApp,
    only_dialogs: bool,
}

type H = Harness<'static, S>;

fn harness() -> H {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 80]})).expect("rect");
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let tmp = std::env::temp_dir().join(format!("fp-i18n-dialoge-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    app.state.file_override = Some(fp_ui::files::FilePickOverride { open: vec![tmp.join("nicht-vorhanden.png")], save: Some(tmp.join("ausgabe.png")) });
    let mut h = Harness::builder().with_size(egui::vec2(1800.0, 1200.0)).with_pixels_per_point(1.0).with_max_steps(8).with_step_dt(1.0 / 60.0).build_ui_state(
        |ui, s: &mut S| {
            if s.only_dialogs {
                dialogs::show_all(ui.ctx(), &mut s.app.state);
            } else {
                s.app.root_ui(ui);
            }
        },
        S { app, only_dialogs: false },
    );
    // Schriften und Design einrichten, Element auswählen
    h.run_steps(3);
    let _ = h.state_mut().app.state.session.execute("select.all", json!({}));
    h.run_steps(1);
    h.state_mut().only_dialogs = true;
    h
}

fn st(h: &mut H) -> &mut fp_ui::app_state::AppState {
    &mut h.state_mut().app.state
}

fn texts(shape: &Shape, out: &mut Vec<TextShape>) {
    match shape {
        Shape::Text(t) => out.push(t.clone()),
        Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
        _ => {}
    }
}

/// Texte mit Clip-Rechteck (Listen mit Bildlauf schneiden ihre Zeilen ab).
fn clipped_texts(h: &H) -> Vec<(TextShape, Rect)> {
    let mut out = Vec::new();
    for cs in &h.output().shapes {
        let mut v = Vec::new();
        texts(&cs.shape, &mut v);
        out.extend(v.into_iter().map(|t| (t, cs.clip_rect)));
    }
    out
}

/// Sieht der Text aus wie ein Schlüssel aus einer .ftl-Datei?
fn looks_like_key(s: &str) -> bool {
    const PREFIXES: [&str; 18] = ["dlg-", "btn-", "unit-", "key-", "fmt-", "filter-", "effect-", "blend-", "ui-", "panel-", "pi-", "cmd-", "menu-", "tool-", "err-", "msg-", "ai-", "lang-"];
    let s = s.trim();
    PREFIXES.iter().any(|p| s.starts_with(p)) && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.' || c.is_ascii_uppercase())
}

#[derive(Default)]
struct Report {
    problems: Vec<String>,
    /// Texte ohne Testsprachen-Markierung (Eigennamen, Inhalte, Zahlen – nur zur Durchsicht).
    unmarked: Vec<String>,
    dialogs: usize,
}

/// Prüft den gerade offenen Dialog; `what` für die Meldung.
fn check(h: &mut H, what: &str, lang: &str, rep: &mut Report) {
    // Überläufe aus dem letzten vollständigen Bild
    let _ = dialogs::take_overflows(&h.ctx);
    h.run_steps(1);
    let over = dialogs::take_overflows(&h.ctx);
    let Some(id) = dialogs::last_dialog_id(&h.ctx) else { return };
    let Some(dr) = dialogs::dialog_rect(&h.ctx, &id) else { return };
    rep.dialogs += 1;
    for o in over {
        rep.problems.push(format!("[{lang}] {what}: {o}"));
    }
    let mut rects: Vec<(String, Rect)> = Vec::new();
    for (t, clip) in clipped_texts(h) {
        let text = t.galley.job.text.clone();
        if text.trim().is_empty() {
            continue;
        }
        // sichtbarer Teil (Listen mit Bildlauf schneiden ihre Zeilen ab)
        let r = t.galley.rect.translate(t.pos.to_vec2()).intersect(clip);
        if !r.is_positive() {
            continue;
        }
        if !dr.contains(Pos2::new(r.center().x, r.center().y)) {
            continue; // Aufklapplisten/Tooltips außerhalb
        }
        if lang == fp_i18n::PSEUDO_LANG && !text.contains("[!!") && text.chars().any(|c| c.is_alphabetic()) {
            rep.unmarked.push(format!("{what}: „{text}“"));
        }
        if looks_like_key(&text) {
            rep.problems.push(format!("[{lang}] {what}: Rohschlüssel sichtbar „{text}“"));
        }
        if t.galley.elided {
            rep.problems.push(format!("[{lang}] {what}: abgeschnitten „{text}“"));
        }
        if r.right() > dr.right() + 1.0 || r.left() < dr.left() - 1.0 || r.bottom() > dr.bottom() + 1.0 {
            rep.problems.push(format!("[{lang}] {what}: „{text}“ ragt aus dem Dialog ({:.0}..{:.0} / Dialog {:.0}..{:.0})", r.left(), r.right(), dr.left(), dr.right()));
        }
        rects.push((text, r));
    }
    for i in 0..rects.len() {
        for j in i + 1..rects.len() {
            let (a, ra) = &rects[i];
            let (b, rb) = &rects[j];
            let ov = ra.shrink(1.5).intersect(rb.shrink(1.5));
            if ov.is_positive() && !(a == b && (ra.min - rb.min).length() < 0.5) {
                rep.problems.push(format!("[{lang}] {what}: „{a}“ überlappt „{b}“"));
            }
        }
    }
}

fn reset(h: &mut H) {
    let st = &mut h.state_mut().app.state;
    // Vorschauen zurücknehmen und alle Dialoge schließen
    if let Some(mut f) = st.dialogs.form.take() {
        dialogs::form::revert_preview(st, &mut f);
    }
    st.dialogs = Dialogs::default();
    h.run_steps(2);
    h.ctx.data_mut(|d| d.remove::<String>(egui::Id::new("fp-last-dialog")));
}

/// Alle Befehle, die einen Dialog öffnen können (Register), plus die direkt geöffneten Dialoge.
fn run_all(h: &mut H, lang: &str, rep: &mut Report) {
    assert!(fp_i18n::set_language(lang), "{lang}");
    h.run_steps(2);
    // direkt geöffnete Dialoge
    {
        let prefs = st(h).session.prefs.clone();
        st(h).dialogs.new_document = Some(dialogs::NewDocumentDialog::from_prefs(&prefs));
    }
    h.run_steps(3);
    check(h, "Neues Dokument", lang, rep);
    reset(h);
    let id = st(h).session.active_id().expect("Dokument");
    st(h).dialogs.unsaved = Some(dialogs::UnsavedFlow::new(vec![id], dialogs::PendingClose::CloseDocs));
    h.run_steps(3);
    check(h, "Ungespeichert", lang, rep);
    reset(h);
    for cat in 0..7 {
        let mut p = dialogs::PreferencesDialog::new(st(h));
        p.category = cat;
        st(h).dialogs.preferences = Some(p);
        h.run_steps(3);
        check(h, &format!("Voreinstellungen {cat}"), lang, rep);
        reset(h);
    }
    st(h).dialogs.about = true;
    h.run_steps(3);
    check(h, "Über", lang, rep);
    reset(h);
    st(h).dialogs.save_workspace = Some("Mein Layout".into());
    h.run_steps(3);
    check(h, "Arbeitsbereich speichern", lang, rep);
    reset(h);
    st(h).dialogs.html_slice = Some("<a href=\"#\">x</a>".into());
    h.run_steps(3);
    check(h, "HTML-Segment", lang, rep);
    reset(h);
    dialogs::models::open_manager(st(h));
    h.run_steps(3);
    check(h, "KI-Modelle verwalten", lang, rep);
    reset(h);
    let model = fp_engine::commands::ai::fp_ai::models::MODELS[0].id;
    st(h).dialogs.ai.missing = Some(dialogs::models::MissingDialog { model, pending: None, download: None, error: None, done: None });
    h.run_steps(3);
    check(h, "KI-Modell fehlt", lang, rep);
    reset(h);
    // Effektdialoge des Live-Filter-Menüs
    for i in 0..dialogs::effects::FX.len() {
        dialogs::effects::begin_add(st(h), i);
        h.run_steps(3);
        if st(h).dialogs.any_open() {
            check(h, &format!("Effekt {}", dialogs::effects::FX[i].name()), lang, rep);
        }
        reset(h);
    }
    // alle Befehle des Registers
    let names: Vec<String> = st(h).session.registry().iter().map(|c| c.info().name.to_string()).collect();
    for name in names {
        if name.starts_with("file.close") || name.starts_with("file.exit") || name.starts_with("file.quit") || name == "file.new" {
            continue;
        }
        let doc_before = st(h).session.active_id();
        if !dialogs::open_for_command(st(h), &name) {
            continue;
        }
        h.run_steps(3);
        if st(h).dialogs.any_open() {
            check(h, &name, lang, rep);
        }
        reset(h);
        // Dokument wie vorher (Befehle ohne Dialog dürfen nichts schließen)
        if st(h).session.active_id() != doc_before
            && let Some(d) = doc_before
        {
            let _ = st(h).session.activate(d);
        }
    }
}

#[test]
fn dialoge_in_allen_sprachen_ohne_ueberlauf_und_ohne_rohschluessel() {
    let _g = LOCK.lock();
    let mut h = harness();
    let mut rep = Report::default();
    for lang in ["de", "en", fp_i18n::PSEUDO_LANG] {
        run_all(&mut h, lang, &mut rep);
    }
    fp_i18n::set_language("de");
    rep.unmarked.sort();
    rep.unmarked.dedup();
    eprintln!("Nicht übersetzte Texte (qps, zur Durchsicht):\n{}", rep.unmarked.join("\n"));
    assert!(rep.dialogs > 3 * 60, "zu wenige Dialoge geprüft: {}", rep.dialogs);
    assert!(rep.problems.is_empty(), "{} Befunde in {} Dialogen:\n{}", rep.problems.len(), rep.dialogs, rep.problems.join("\n"));
}

/// Deutsch wie bisher: Wortlaut und Maße der festen Dialoge unverändert.
#[test]
fn deutsch_unveraendert() {
    let _g = LOCK.lock();
    fp_i18n::set_language("de");
    let mut h = harness();
    let size_of = |h: &mut H, id: &str| dialogs::dialog_geom(&h.ctx, id).map(|g| g.client).expect(id);
    let prefs = st(&mut h).session.prefs.clone();
    st(&mut h).dialogs.new_document = Some(dialogs::NewDocumentDialog::from_prefs(&prefs));
    h.run_steps(3);
    assert_eq!(size_of(&mut h, "fp-new-doc"), dialogs::dlu(260.0, 174.0), "Neues Dokument");
    reset(&mut h);
    assert!(dialogs::open_for_command(st(&mut h), "modify.canvas.canvasSize"));
    h.run_steps(3);
    assert_eq!(size_of(&mut h, "fp-canvas-size"), dialogs::dlu(222.0, 196.0), "Leinwandgröße");
    reset(&mut h);
    assert!(dialogs::open_for_command(st(&mut h), "modify.transform.numericTransform"));
    h.run_steps(3);
    assert_eq!(size_of(&mut h, "fp-numeric"), dialogs::dlu(214.0, 130.0), "Numerisch transformieren");
    reset(&mut h);
    let id = st(&mut h).session.active_id().expect("Dokument");
    st(&mut h).dialogs.unsaved = Some(dialogs::UnsavedFlow::new(vec![id], dialogs::PendingClose::CloseDocs));
    h.run_steps(3);
    assert_eq!(size_of(&mut h, "fp-unsaved"), egui::vec2(420.0, 120.0), "Ungespeichert");
    let mut all = Vec::new();
    for cs in &h.output().shapes {
        texts(&cs.shape, &mut all);
    }
    let words: Vec<String> = all.iter().map(|t| t.galley.job.text.clone()).collect();
    for w in ["Ja", "Nein", "Abbrechen"] {
        assert!(words.iter().any(|x| x == w), "Knopf „{w}“ fehlt: {words:?}");
    }
    reset(&mut h);
    assert!(dialogs::open_for_command(st(&mut h), "filter.blur.gaussian"));
    h.run_steps(3);
    assert_eq!(size_of(&mut h, "fp-form-5971").x, 340.0, "Formularbreite");
    reset(&mut h);
}

/// Zugriffstasten: „&Ja“/„&Nein“ in Deutsch, „&Yes“/„&No“ in Englisch, je Sprache eindeutig.
#[test]
fn zugriffstasten_je_sprache_eindeutig() {
    let _g = LOCK.lock();
    for (lang, yes, no) in [("de", egui::Key::J, egui::Key::N), ("en", egui::Key::Y, egui::Key::N)] {
        assert!(fp_i18n::set_language(lang));
        let y = fp_i18n::t("btn-yes");
        let n = fp_i18n::t("btn-no");
        assert_eq!(dialogs::access_key(&y), Some(yes), "{lang}: {y}");
        assert_eq!(dialogs::access_key(&n), Some(no), "{lang}: {n}");
        assert_ne!(dialogs::access_key(&y), dialogs::access_key(&n));
    }
    fp_i18n::set_language("de");
}

/// Alle Schlüssel der Dialoge gibt es in Deutsch und Englisch; im Englischen bleibt kein
/// deutscher Text stehen (Stichprobe: Umlaute).
#[test]
fn dialog_schluessel_vollstaendig() {
    let _g = LOCK.lock();
    let de = fp_i18n::keys("de");
    let en = fp_i18n::keys("en");
    let dlg: Vec<&String> = de.iter().filter(|k| k.starts_with("dlg-")).collect();
    assert!(dlg.len() > 400, "nur {} dlg-Schlüssel", dlg.len());
    for k in &dlg {
        assert!(en.contains(*k), "Englisch fehlt {k}");
    }
    assert!(fp_i18n::set_language("en"));
    for k in &dlg {
        let s = fp_i18n::t(k);
        assert!(!s.contains(['ä', 'ö', 'ü', 'Ä', 'Ö', 'Ü', 'ß']), "{k}: deutsch im Englischen: {s}");
    }
    fp_i18n::set_language("de");
}
