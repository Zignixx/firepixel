//! Gesamttest des Hauptmenüs und der Bedienfeldmenüs (HG, Phase 5): jeder Eintrag wird über
//! denselben Weg wie ein Mausklick ausgeführt (`Action` → `actions::execute`), in einem
//! Testdokument mit allen Objektarten. Je Eintrag wird eine passende Auswahl gesucht (Verfügbarkeit
//! wie im Menü), danach geprüft: keine Panik, keine „noch nicht umgesetzt“-Meldung, sichtbare
//! Wirkung (Dokument mit Verlaufsschritt, Dialog, Ansicht/Oberfläche) und bei Dokumentänderungen,
//! dass Rückgängig den Ausgangszustand exakt wiederherstellt.
//!
//! Ergebnisse: `docs\funktionstest\menue_auto.md`, `bedienfeldmenues_auto.md` (Grundlage für
//! `docs\FUNKTIONSTEST.md`). Der Test schlägt nur bei Paniken fehl; fehlende Funktionen werden als
//! Befund gemeldet und im Bericht gezählt.

use crate::full_common;

use fp_ui::menu;
use fp_ui::panels::PanelId;
use full_common::*;

fn assert_no_panics(rows: &[Row]) {
    let panics: Vec<_> = rows.iter().filter(|r| matches!(r.outcome, Outcome::Panic(_))).map(|r| format!("{} ({:?})", r.path, r.outcome)).collect();
    assert!(panics.is_empty(), "Paniken: {panics:#?}");
}

#[test]
fn full_main_menu() {
    let (st, _ctx, _f) = fixture();
    let menus = menu::build_main_menu(&st);
    assert_eq!(menus.len(), 10, "10 Hauptmenüs");
    let mut items = Vec::new();
    for (label, nodes) in &menus {
        walk(nodes, label, &mut items);
    }
    drop(st);
    let rows = run_nodes(items);
    let (ok, n) = write_report("menue_auto.md", "Hauptmenü", "full_menu.rs", &rows);
    eprintln!("Hauptmenü: {ok}/{n} ✅");
    assert_no_panics(&rows);
}

/// Sondersituationen nach AGENTEN_REGELN §4b: jeder Menübefehl, der in der Situation verfügbar ist,
/// läuft ohne Panik; gesperrte bzw. ausgeblendete Objekte bleiben unverändert (außer bei Befehlen,
/// die genau das umschalten); Rückgängig stellt exakt her.
#[test]
fn full_main_menu_situations() {
    use fp_ui::app_state::Action;
    use serde_json::json;
    let (st, _ctx, _f) = fixture();
    let mut items = Vec::new();
    for (label, nodes) in menu::build_main_menu(&st) {
        walk(&nodes, &label, &mut items);
    }
    drop(st);
    let cmds: Vec<(String, String)> = items
        .into_iter()
        .filter_map(|(p, a)| match a {
            fp_ui::menu::ItemAction::Command(c) if !external(&c) && !c.starts_with("file.") && !c.starts_with("window.") && !c.starts_with("help.") => Some((p, c)),
            _ => None,
        })
        .collect();
    // (Name, Vorbereitung) – Vorbereitung bekommt die IDs des Testdokuments
    type Prep = fn(&mut fp_ui::app_state::AppState, &Fix) -> Vec<u64>;
    let situations: Vec<(&str, Prep)> = vec![
        ("leere Auswahl", |_, _| vec![]),
        ("alles ausgewählt", |st, _| ex(st, "select.all", json!({}))),
        ("gesperrtes Rechteck", |st, f| {
            let _ = st.session.execute("element.setLocked", json!({ "ids": f.rect, "locked": true }));
            f.rect.clone()
        }),
        ("ausgeblendetes Rechteck", |st, f| {
            let _ = st.session.execute("element.setVisible", json!({ "ids": f.rect, "visible": false }));
            f.rect.clone()
        }),
        ("Gruppe", |_, f| f.group.clone()),
        ("Symbolinstanz", |_, f| f.instance.clone()),
        ("Seite 2 mit Objekt", |st, _| {
            let _ = st.session.execute("page.setCurrent", json!({ "index": 1 }));
            ex(st, "draw.rectangle", json!({ "rect": [10, 10, 90, 90] }))
        }),
        ("Status 2", |st, f| {
            let _ = st.session.execute("state.setCurrent", json!({ "index": 1 }));
            f.rect.clone()
        }),
    ];
    let mut rows = Vec::new();
    for (sit, prep) in &situations {
        let mut ok = 0;
        let mut problems: Vec<String> = Vec::new();
        let mut ran = 0;
        for (path, c) in &cmds {
            let (mut st, ctx, f) = fixture();
            let ids = prep(&mut st, &f);
            select(&mut st, &ids);
            if let Some(d) = st.session.active_mut() {
                d.history.clear();
            }
            st.status = None;
            if !available(&st, c) {
                continue;
            }
            ran += 1;
            let rect_id = fp_model::ElementId(f.rect.first().copied().unwrap_or(0));
            let rect_json = |st: &fp_ui::app_state::AppState| {
                st.session.active().and_then(|d| fp_engine::docops::element_on_page(&d.doc, 0, rect_id)).map(|e| serde_json::to_value(&*e).unwrap_or_default())
            };
            let before_rect = rect_json(&st);
            let before = probe(&st);
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                st.push(Action::Run(c.clone(), json!({})));
                st.process_actions(&ctx);
                st
            }));
            let mut st = match r {
                Ok(s) => s,
                Err(e) => {
                    problems.push(format!("PANIK bei {path}: {}", panic_msg(&e)));
                    continue;
                }
            };
            let toggles = c.contains("lock") || c.contains("Lock") || c.contains("hide") || c.contains("Hide") || c.contains("show") || c.contains("Show") || c.contains("delete") || c.contains("cut") || c.contains("clear");
            // ganze Leinwand (Drehen, Zuschneiden, Anpassen) betrifft auch gesperrte Objekte – wie im Original
            let canvas_wide = c.starts_with("modify.canvas.") || c == "edit.cropDocument";
            if *sit == "gesperrtes Rechteck" && !toggles && !canvas_wide && rect_json(&st) != before_rect && rect_json(&st).is_some() {
                problems.push(format!("{path}: {sit} wurde verändert"));
            }
            let after = probe(&st);
            if after.undo > before.undo {
                let _ = st.session.undo();
                if probe(&st).doc != before.doc {
                    problems.push(format!("{path}: Rückgängig nicht exakt"));
                    continue;
                }
            }
            ok += 1;
        }
        let o = if problems.is_empty() { Outcome::Doc } else { Outcome::Error(format!("{} Fund(e)", problems.len())) };
        rows.push(Row { path: format!("Situation „{sit}“"), command: format!("{ran} verfügbare Menübefehle"), outcome: o, note: if problems.is_empty() { format!("{ok}/{ran} ohne Befund") } else { problems.join("; ") } });
    }
    let (ok, n) = write_report("menue_situationen_auto.md", "Hauptmenü in Sondersituationen", "full_menu.rs", &rows);
    eprintln!("Situationen: {ok}/{n} ✅");
    let panics: Vec<_> = rows.iter().filter(|r| r.note.contains("PANIK")).map(|r| r.note.clone()).collect();
    assert!(panics.is_empty(), "Paniken: {panics:#?}");
}

/// Kontextmenüs der Leinwand (alle Ressourcen-Menüs, je Werkzeug/Auswahl).
#[test]
fn full_context_menus() {
    let (st, _ctx, _f) = fixture();
    let mut items = Vec::new();
    let names = [("6914", "Nichts ausgewählt"), ("6925", "Objekt ausgewählt"), ("6915", "Text"), ("6913", "Pixel"), ("6916", "Transformieren"), ("6910", "Zuschneiden"), ("6906", "Zoom"), ("49120", "Pipette")];
    for (id, label) in names {
        let nodes = fp_ui::canvas::context::build(&st, fp_ui::canvas::context::entries(id));
        walk(&nodes, &format!("Kontextmenü {label}"), &mut items);
    }
    drop(st);
    let rows = run_nodes(items);
    let (ok, n) = write_report("kontextmenues_auto.md", "Kontextmenüs der Leinwand", "full_menu.rs", &rows);
    eprintln!("Kontextmenüs: {ok}/{n} ✅");
    assert_no_panics(&rows);
}

#[test]
fn full_panel_menus() {
    let (st, _ctx, _f) = fixture();
    let mut items = Vec::new();
    for p in PanelId::ALL {
        let nodes = menu::build_panel_menu(&st, *p);
        walk(&nodes, p.title(), &mut items);
    }
    drop(st);
    let rows = run_nodes(items);
    let (ok, n) = write_report("bedienfeldmenues_auto.md", "Bedienfeldmenüs", "full_menu.rs", &rows);
    eprintln!("Bedienfeldmenüs: {ok}/{n} ✅");
    assert_no_panics(&rows);
}
