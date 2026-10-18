//! Logiktests der interaktiven Funktionen (HG, Phase 5, AGENTEN_REGELN §4b): jeder Formular-Dialog
//! (alle Menübefehle mit Dialog, alle Live-Effekte „hinzufügen“ und „bearbeiten“) wird ohne
//! Bildschirm durchgespielt:
//!
//! 1. Wert 50× ändern mit Vorschau → genau ein Verlaufsschritt, Ergebnis gleich einmaligem Anwenden
//!    (keine Duplikate wie mehrfach angehängte Effekte),
//! 2. Abbrechen → Ausgangszustand exakt,
//! 3. OK → genau ein Verlaufsschritt, Rückgängig exakt, Wiederholen = Ergebnis,
//! 4. Auswahlwechsel mitten in der Vorschau, danach Abbrechen → Ausgangszustand exakt.
//!
//! Ergebnis: `docs\funktionstest\logik_auto.md`.

use crate::full_common;

use fp_ui::app_state::{Action, AppState};
use fp_ui::dialogs::effects::{begin_add, begin_edit, FxKind, FX};
use fp_ui::dialogs::form::{apply_preview, revert_preview, FieldKind, FormDialog};
use full_common::*;
use serde_json::{json, Value};

/// Woher der Dialog kommt.
#[derive(Clone, Debug)]
enum Source {
    Menu(String),
    EffectAdd(usize),
    EffectEdit(usize),
}

fn doc_json(st: &AppState) -> Value {
    st.session.active().map(|d| norm_json(&d.doc)).unwrap_or(Value::Null)
}

/// Dokument als JSON zum Vergleichen. Bildkacheln tragen eine fortlaufende ID (jede neu berechnete
/// Kachel bekommt eine neue), die Pixel selbst sind nicht im JSON: die ID wird deshalb durch eine
/// Prüfsumme der Pixel ersetzt (GT-0j – sonst meldet jede Bitmap-Vorschau „≠ einmal anwenden“).
fn norm_json(doc: &fp_model::Document) -> Value {
    use std::hash::{Hash, Hasher};
    let mut sums = std::collections::HashMap::new();
    doc.walk_elements(|_, _, e| {
        if let fp_model::ElementKind::Bitmap(b) = &e.kind {
            for t in &b.image.grid.tiles {
                if let fp_model::TileEntry::Tile(t) = t {
                    let mut h = std::collections::hash_map::DefaultHasher::new();
                    t.pixels.hash(&mut h);
                    sums.insert(u64::from(t.id), h.finish());
                }
            }
        }
    });
    fn walk(v: &mut Value, sums: &std::collections::HashMap<u64, u64>) {
        match v {
            Value::Object(m) => {
                if let Some(Value::Object(t)) = m.get_mut("Tile")
                    && let Some(sum) = t.get("id").and_then(Value::as_u64).and_then(|id| sums.get(&id))
                {
                    t.insert("id".into(), Value::String(format!("px:{sum:016x}")));
                }
                m.values_mut().for_each(|x| walk(x, sums));
            }
            Value::Array(a) => a.iter_mut().for_each(|x| walk(x, sums)),
            _ => {}
        }
    }
    let mut v = serde_json::to_value(doc).unwrap_or(Value::Null);
    walk(&mut v, &sums);
    v
}

fn undo_count(st: &AppState) -> usize {
    st.session.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

/// Inhalt eines bestimmten Dokuments (auch wenn inzwischen eine Symbolbearbeitung aktiv ist).
fn doc_json_of(st: &AppState, id: Option<fp_engine::DocId>) -> Value {
    id.and_then(|i| st.session.document(i)).map(|d| norm_json(&d.doc)).unwrap_or(Value::Null)
}

fn undo_count_of(st: &AppState, id: Option<fp_engine::DocId>) -> usize {
    id.and_then(|i| st.session.document(i)).map(|d| d.history.undo_count()).unwrap_or(0)
}

/// Befehle, die laut Messung im Original **keinen** Verlaufsschritt anlegen, obwohl sie das
/// Dokument ändern: erwartet werden 0 Schritte, und Rückgängig lässt die Änderung stehen.
const NO_STEP_BY_DESIGN: &[(&str, &str)] = &[("view.grid.editGrid", "PM-5 D18: Raster bearbeiten ohne Verlaufsschritt, Strg+Z ändert das Raster nicht")];

fn no_step_by_design(cmd: &str) -> bool {
    NO_STEP_BY_DESIGN.iter().any(|(c, _)| *c == cmd)
}

/// Raster aller Seiten (was „Raster bearbeiten“ ändert; ein früherer Schritt darf zurückgenommen werden).
fn grids(v: &Value) -> Vec<Value> {
    v["pages"].as_array().map(|a| a.iter().map(|p| p["root"]["grid"].clone()).collect()).unwrap_or_default()
}

/// Öffnet den Dialog in einem frischen Testdokument (`cand` = Index der Testsituation).
fn open(src: &Source, cand: usize) -> Option<(AppState, egui::Context, Fix, FormDialog)> {
    let (mut st, ctx, f) = fixture();
    let cands = candidates(&f);
    let c = cands.get(cand)?.clone();
    match src {
        Source::Menu(cmd) => {
            apply_candidate(&mut st, &c);
            if !available(&st, cmd) {
                return None;
            }
            st.push(Action::Run(cmd.clone(), json!({})));
            st.process_actions(&ctx);
        }
        Source::EffectAdd(i) => {
            select(&mut st, &f.rect);
            begin_add(&mut st, *i);
            st.process_actions(&ctx);
        }
        Source::EffectEdit(i) => {
            select(&mut st, &f.rect);
            begin_add(&mut st, *i);
            let d = st.dialogs.form.take()?;
            let (cmd, p) = d.invocation();
            fp_ui::ui_commands::execute(&mut st.session, &cmd, p).ok()?;
            let doc = st.session.active()?;
            let id = fp_model::ElementId(*f.rect.first()?);
            let e = fp_engine::docops::element_on_page(&doc.doc, doc.page, id)?;
            let eff = e.common.effects.as_ref()?.effects.first()?.clone();
            if let Some(d) = st.session.active_mut() {
                d.history.clear();
            }
            begin_edit(&mut st, 0, &eff);
        }
    }
    let d = st.dialogs.form.take()?;
    st.status = None;
    Some((st, ctx, f, d))
}

/// Erstes Zahlenfeld (Schieber/Zahl) und dessen Bereich.
fn number_field(d: &FormDialog) -> Option<(usize, f64, f64)> {
    d.fields.iter().enumerate().find_map(|(i, f)| match f.kind {
        FieldKind::Number { min, max, .. } if !f.key.starts_with('_') && max > min => Some((i, min, max)),
        _ => None,
    })
}

fn set_value(d: &mut FormDialog, i: usize, k: usize, min: f64, max: f64) -> Value {
    let span = (max - min).min(200.0);
    let v = (min + span * ((k % 7) as f64 + 1.0) / 8.0).round();
    let v = if v > max { max } else { v };
    // wie eine Eingabe im Zahlenfeld bzw. am Schieber (Bereichsprüfung, Nachkommastellen)
    d.fields[i].set_number_text(&format!("{v}"));
    d.fields[i].value.clone()
}

/// Spielt einen Dialog durch; liefert (Ergebnis, Hinweis).
fn protocol(src: &Source, cand: usize) -> Option<(Outcome, String)> {
    let (mut st, _ctx, _f, mut d) = open(src, cand)?;
    let base = doc_json(&st);
    let base_undo = undo_count(&st);
    let field = number_field(&d);
    let mut problems: Vec<String> = Vec::new();
    // 1. 50× ändern
    let mut last = None;
    if d.preview {
        d.preview_on = true;
        for k in 0..50 {
            if let Some((i, min, max)) = field {
                last = Some(set_value(&mut d, i, k, min, max));
            }
            apply_preview(&mut st, &mut d);
        }
        if undo_count(&st) != base_undo + 1 {
            problems.push(format!("Vorschau 50×: {} Verlaufsschritte statt 1", undo_count(&st) as i64 - base_undo as i64));
        }
    } else if let Some((i, min, max)) = field {
        for k in 0..50 {
            last = Some(set_value(&mut d, i, k, min, max));
        }
    }
    let after_preview = doc_json(&st);
    // 2. Abbrechen
    if d.preview {
        revert_preview(&mut st, &mut d);
        if doc_json(&st) != base {
            problems.push("Abbrechen nach Vorschau stellt nicht exakt her".into());
        }
    }
    // 3. OK: einmal mit dem Endwert anwenden (im selben Dokument)
    let (cmd, p) = d.invocation();
    if cmd.is_empty() {
        return Some((Outcome::Ui, "Hinweisdialog ohne Befehl".into()));
    }
    let _ = last;
    // Gemessen wird am Ausgangsdokument: „Neues Symbol…“/„Neue Schaltfläche…“ öffnen nach OK wie
    // im Original die Symbolbearbeitung (eigenes Dokumentfenster); der Schritt gehört dem Dokument.
    let orig = st.session.active_id();
    if let Err(e) = fp_ui::ui_commands::execute(&mut st.session, &cmd, p) {
        return Some((Outcome::Error(e.to_string()), format!("OK mit Endwert: {e}")));
    }
    let editor = st.session.active_id().filter(|a| Some(*a) != orig);
    if let (Some(e), Some(o)) = (editor, orig) {
        // Symbolbearbeitung schließen (wie „Fertig“), zurück ins Dokument
        let _ = st.session.close_document(e);
        let _ = st.session.activate(o);
    }
    let steps = undo_count_of(&st, orig) as i64 - base_undo as i64;
    let reference = doc_json_of(&st, orig);
    if no_step_by_design(&cmd) {
        if steps != 0 {
            problems.push(format!("OK: {steps} Verlaufsschritte statt 0 (wie im Original)"));
        }
        let _ = st.session.undo();
        if grids(&doc_json(&st)) != grids(&reference) {
            problems.push("Rückgängig nimmt die Änderung zurück (Original: bleibt)".into());
        }
    } else if steps == 0 && reference == base {
        // reine Einstellung ohne Dokumentänderung (z. B. Seite einrichten): richtig ohne Verlaufsschritt
    } else {
        if steps != 1 {
            problems.push(format!("OK: {steps} Verlaufsschritte statt 1"));
        }
        let _ = st.session.undo();
        if doc_json(&st) != base {
            problems.push("OK + Rückgängig stellt nicht exakt her".into());
        }
        let _ = st.session.redo();
        if doc_json(&st) != reference {
            problems.push("Wiederholen ergibt anderes Ergebnis".into());
        }
        let _ = st.session.undo();
    }
    if d.preview && after_preview != reference {
        problems.push("Vorschau 50× ≠ einmal anwenden (Duplikate oder Restwirkung)".into());
    }
    if d.preview {
        // 4. Auswahlwechsel mitten in der Vorschau, dann Abbrechen
        d.applied = None;
        d.applied_params = None;
        apply_preview(&mut st, &mut d);
        let undo_before = undo_count(&st);
        let _ = st.session.execute("select.set", json!({ "ids": [] }));
        if undo_count(&st) != undo_before {
            problems.push("Auswahlwechsel erzeugt einen Verlaufsschritt".into());
        }
        revert_preview(&mut st, &mut d);
        if doc_json(&st) != base {
            problems.push("Auswahlwechsel während der Vorschau + Abbrechen: Ausgangszustand nicht exakt".into());
        }
    }
    let label = format!("{}{}", candidates_label(cand), if d.preview { ", mit Vorschau" } else { "" });
    Some(if problems.is_empty() { (Outcome::Doc, label) } else { (Outcome::Error(problems.join("; ")), label) })
}

fn candidates_label(i: usize) -> &'static str {
    let (_, _, f) = fixture();
    candidates(&f).get(i).map(|c| c.label).unwrap_or("?")
}

/// Menübefehle, die einen Formular-Dialog öffnen (aus dem Hauptmenü ermittelt).
fn menu_form_commands() -> Vec<(String, String)> {
    let (st, _ctx, _f) = fixture();
    let mut items = Vec::new();
    for (label, nodes) in fp_ui::menu::build_main_menu(&st) {
        walk(&nodes, &label, &mut items);
    }
    items
        .into_iter()
        .filter_map(|(path, a)| match a {
            fp_ui::menu::ItemAction::Command(c) if path.ends_with("...") || path.ends_with('…') => Some((path, c)),
            _ => None,
        })
        .collect()
}

/// Struktur eines Dokuments: Längen aller Listen (Elemente, Effekte, Seiten …) in Baumreihenfolge.
fn structure(v: &Value, out: &mut Vec<usize>) {
    match v {
        Value::Array(a) => {
            out.push(a.len());
            for x in a {
                structure(x, out);
            }
        }
        Value::Object(m) => {
            for (_, x) in m {
                structure(x, out);
            }
        }
        _ => {}
    }
}

fn shape(st: &AppState) -> Vec<usize> {
    let mut out = Vec::new();
    structure(&doc_json(st), &mut out);
    out
}

/// Parameter für einen Setzbefehl; `k` variiert den ersten veränderlichen Wert.
fn synth_params(info: &fp_engine::CommandInfo, k: usize) -> Option<Value> {
    use fp_engine::ParamKind as K;
    let mut p = serde_json::Map::new();
    let mut varied = false;
    for spec in &info.params {
        let vary = !varied;
        let v = match &spec.kind {
            K::Bool => Some(json!(k.is_multiple_of(2))),
            K::Integer { min, max } => {
                let lo = min.unwrap_or(0);
                let hi = max.unwrap_or(lo + 100).min(lo + 100);
                Some(json!(if vary { lo + ((k as i64 * 7) % (hi - lo + 1).max(1)) } else { lo.max(0).min(hi) }))
            }
            K::Number { min, max } => {
                let lo = min.unwrap_or(0.0);
                let hi = max.unwrap_or(lo + 100.0).min(lo + 100.0);
                Some(json!(if vary { lo + (hi - lo) * ((k % 9) as f64 + 1.0) / 10.0 } else { lo.max(0.0).min(hi) }))
            }
            K::Enum { values } => values.get(if vary { k % values.len().max(1) } else { 0 }).map(|s| json!(s)),
            K::Color => Some(json!(format!("#{:02x}{:02x}{:02x}", (k * 37) % 256, (k * 91) % 256, (k * 13) % 256))),
            K::String => Some(json!(format!("Wert {k}"))),
            _ => None,
        };
        match v {
            Some(v) => {
                if matches!(spec.kind, K::Integer { .. } | K::Number { .. } | K::Enum { .. } | K::Color) {
                    varied = true;
                }
                p.insert(spec.name.clone(), v);
            }
            None if spec.required => return None,
            None => {}
        }
    }
    varied.then_some(Value::Object(p))
}

/// Setzbefehle (Inspektor/Bedienfelder: `pi.*`, `element.set*`, `text.set*`, `layer.set*` …) 50×
/// mit wechselnden Werten: Struktur gleich einmaligem Anwenden (keine Duplikate), je Aufruf ein
/// Verlaufsschritt, 50× Rückgängig stellt exakt her, 50× Wiederholen = Endergebnis.
#[test]
fn full_logic_set_commands() {
    let (st0, _, f0) = fixture();
    let infos: Vec<fp_engine::CommandInfo> = st0
        .session
        .registry()
        .iter()
        .map(|c| c.info().clone())
        .filter(|i| {
            let n = i.name.as_str();
            // nur Setzbefehle (Wert setzen, nicht hinzufügen/kopieren/ausrichten)
            let adds = ["pi.style.new", "pi.state.copyTo", "pi.align", "pi.layer.addSub", "pi.behavior.add", "pi.symbol.place", "pi.symbol.import"];
            (n.starts_with("pi.") || n.contains(".set") || n.starts_with("text.") || n.starts_with("view.")) && !n.contains("setCurrent") && !adds.contains(&n) && i.implemented
        })
        .collect();
    drop(st0);
    let ncand = candidates(&f0).len();
    let mut rows = Vec::new();
    for info in infos {
        let Some(p0) = synth_params(&info, 0) else { continue };
        let mut result: Option<(Outcome, String)> = None;
        for cand in 0..ncand {
            let (mut st, _ctx, f) = fixture();
            let c = candidates(&f)[cand].clone();
            apply_candidate(&mut st, &c);
            if !st.session.is_available(&info.name) {
                continue;
            }
            // genug Rückgängig-Schritte für 50 Änderungen (Vorgabe wie im Original: 20)
            if let Some(d) = st.session.active_mut() {
                d.history.set_max_undos(200);
            }
            let base = doc_json(&st);
            let base_undo = undo_count(&st);
            // einmal (mit dem Wert des letzten Durchlaufs) – Bezug
            let last = synth_params(&info, 49).unwrap_or(p0.clone());
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| st.session.execute(&info.name, last.clone())));
            match r {
                Err(e) => {
                    result = Some((Outcome::Panic(panic_msg(&e)), c.label.into()));
                    break;
                }
                Ok(Err(_)) => continue,
                Ok(Ok(_)) => {}
            }
            let once = shape(&st);
            let once_doc = doc_json(&st);
            let once_steps = undo_count(&st) - base_undo;
            while undo_count(&st) > base_undo {
                let _ = st.session.undo();
            }
            let mut problems = Vec::new();
            let mut errors = 0;
            for k in 0..50 {
                let p = synth_params(&info, k).unwrap_or(p0.clone());
                if st.session.execute(&info.name, p).is_err() {
                    errors += 1;
                }
            }
            let fifty_steps = undo_count(&st) - base_undo;
            if shape(&st) != once {
                problems.push("50× ändern: andere Struktur als einmal (Duplikate?)".to_string());
            }
            if doc_json(&st) != once_doc && once_steps > 0 {
                problems.push("50× ändern mit gleichem Endwert ≠ einmal".to_string());
            }
            if errors > 0 {
                problems.push(format!("{errors} von 50 Aufrufen mit Fehler"));
            }
            if no_step_by_design(&info.name) {
                // wie im Original ohne Verlaufsschritt: 0 Schritte, Rückgängig ändert nichts
                if fifty_steps != 0 {
                    problems.push(format!("{fifty_steps} Verlaufsschritte statt 0 (wie im Original)"));
                }
                let after = grids(&doc_json(&st));
                let _ = st.session.undo();
                if grids(&doc_json(&st)) != after {
                    problems.push("Rückgängig nimmt die Änderung zurück (Original: bleibt)".to_string());
                }
            } else {
                for _ in 0..fifty_steps {
                    let _ = st.session.undo();
                }
                if doc_json(&st) != base {
                    problems.push("Rückgängig nach 50× stellt nicht exakt her".to_string());
                }
            }
            for _ in 0..fifty_steps {
                let _ = st.session.redo();
            }
            if fifty_steps > 0 && doc_json(&st) != once_doc && once_steps > 0 {
                problems.push("Wiederholen ergibt nicht das Endergebnis".to_string());
            }
            let note = format!("{}; {} Verlaufsschritte für 50 Änderungen", c.label, fifty_steps);
            result = Some(if problems.is_empty() { (Outcome::Doc, note) } else { (Outcome::Error(problems.join("; ")), note) });
            break;
        }
        let Some((o, note)) = result else { continue };
        rows.push(Row { path: info.menu_path.join(" ▸ ").to_string() + if info.menu_path.is_empty() { info.label.as_str() } else { "" }, command: info.name.clone(), outcome: o, note });
    }
    let (ok, n) = write_report("logik_setzbefehle_auto.md", "Logik der Setzbefehle (50× ändern, Rückgängig/Wiederholen)", "full_logic.rs", &rows);
    eprintln!("Logik Setzbefehle: {ok}/{n} ✅");
    let panics: Vec<_> = rows.iter().filter(|r| matches!(r.outcome, Outcome::Panic(_))).map(|r| format!("{} {:?}", r.command, r.outcome)).collect();
    assert!(panics.is_empty(), "Paniken: {panics:#?}");
}

#[test]
fn full_logic_dialogs() {
    let (_, _, f0) = fixture();
    let ncand = candidates(&f0).len();
    let mut rows = Vec::new();
    let mut sources: Vec<(String, Source)> = menu_form_commands().into_iter().map(|(p, c)| (p, Source::Menu(c))).collect();
    for (i, fx) in FX.iter().enumerate() {
        if !matches!(fx.kind, FxKind::Simple(_)) {
            sources.push((format!("Filter (+) ▸ {} ▸ {} – hinzufügen", fx.group, fx.label), Source::EffectAdd(i)));
            sources.push((format!("Filter (+) ▸ {} ▸ {} – bearbeiten", fx.group, fx.label), Source::EffectEdit(i)));
        }
    }
    for (path, src) in sources {
        let mut result = None;
        for cand in 0..ncand {
            let r = std::panic::catch_unwind(|| protocol(&src, cand));
            match r {
                Ok(Some(x)) => {
                    result = Some(x);
                    break;
                }
                Ok(None) => {}
                Err(e) => {
                    result = Some((Outcome::Panic(panic_msg(&e)), String::new()));
                    break;
                }
            }
            if matches!(src, Source::EffectAdd(_) | Source::EffectEdit(_)) {
                break;
            }
        }
        // Menüeinträge ohne Formular-Dialog (eigene Dialogarten) prüft full_menu
        let Some((o, note)) = result else { continue };
        let cmd = match &src {
            Source::Menu(c) => c.clone(),
            Source::EffectAdd(i) | Source::EffectEdit(i) => format!("{:?}", FX[*i].kind),
        };
        rows.push(Row { path, command: cmd, outcome: o, note });
    }
    let (ok, n) = write_report("logik_auto.md", "Logik der Dialoge (50× ändern, Vorschau, Abbrechen, OK, Rückgängig/Wiederholen, Auswahlwechsel)", "full_logic.rs", &rows);
    eprintln!("Logik Dialoge: {ok}/{n} ✅");
    let panics: Vec<_> = rows.iter().filter(|r| matches!(r.outcome, Outcome::Panic(_))).map(|r| format!("{} {:?}", r.path, r.outcome)).collect();
    assert!(panics.is_empty(), "Paniken: {panics:#?}");
}
