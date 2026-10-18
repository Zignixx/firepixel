//! UX-B: Funktions-Paritätsabgleich (docs\PARITAET_B.md). Erhebt die Firepixel-Seite automatisch:
//! jeder Hauptmenü-Eintrag in 7 Lagen (aktiv/Häkchen/Kürzel), Wirkung beim Auslösen (Dialog, Verlaufsschritt,
//! Formularfelder mit Bereich/Vorgabe/Nachkommastellen/Schieber/Einheit/Vorschau) und die Bedienfeldmenüs.
//! Ausgabe als TSV nach `target-uxb\paritaet\` (bzw. `%FP_UXB_OUT%`); nichts wird geprüft.
//! Aufruf: `cargo test -p fp-ui --test ux_b_parity -- --nocapture --test-threads 1`.

use egui::{Event, Key, Modifiers};
use egui_kittest::Harness;
use fp_engine::Session;
use fp_ui::app_state::UiSettings;
use fp_ui::dialogs::form::FieldKind;
use fp_ui::menu::{ItemAction, Node};
use fp_ui::FirepixelApp;
use serde_json::json;
use std::fmt::Write as _;

type H = Harness<'static, FirepixelApp>;

const LAGEN: [&str; 7] = ["ohne Dokument", "Dokument ohne Auswahl", "1 Rechteck", "2 Objekte", "Text", "Bitmap", "Pixelauswahl"];

fn out_dir() -> std::path::PathBuf {
    let d = std::env::var("FP_UXB_OUT").map(std::path::PathBuf::from).unwrap_or_else(|_| std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target-uxb/paritaet"));
    let _ = std::fs::create_dir_all(&d);
    d
}

fn ids_of(s: &Session) -> Vec<u64> {
    s.active()
        .map(|d| d.doc.resolved_layers(d.page).iter().flat_map(|l| l.layer.cel(d.frame).map(|c| c.elements.iter().map(|e| e.id.0).collect::<Vec<_>>()).unwrap_or_default()).collect())
        .unwrap_or_default()
}

/// Sitzung in Lage `k` (siehe [`LAGEN`]).
fn session(k: usize) -> Session {
    let mut s = Session::default();
    if k == 0 {
        return s;
    }
    s.execute("file.new", json!({"width": 500, "height": 400})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 100]})).expect("r1");
    let r1 = ids_of(&s);
    s.execute("draw.rectangle", json!({"rect": [200, 50, 300, 150]})).expect("r2");
    let r2: Vec<u64> = ids_of(&s).into_iter().filter(|i| !r1.contains(i)).collect();
    s.execute("draw.text", json!({"text": "Hallo", "anchor": [30, 300], "size": 24})).expect("text");
    let t: Vec<u64> = ids_of(&s).into_iter().filter(|i| !r1.contains(i) && !r2.contains(i)).collect();
    s.execute("draw.rectangle", json!({"rect": [320, 200, 420, 300]})).expect("r3");
    s.execute("modify.flattenSelection", json!({})).expect("bitmap");
    let b: Vec<u64> = ids_of(&s).into_iter().filter(|i| !r1.contains(i) && !r2.contains(i) && !t.contains(i)).collect();
    let sel = match k {
        2 => r1.clone(),
        3 => [r1.clone(), r2.clone()].concat(),
        4 => t.clone(),
        5 | 6 => b.clone(),
        _ => vec![],
    };
    s.execute("select.set", json!({"ids": sel})).expect("sel");
    if k == 6 {
        s.execute("select.pixels", json!({"shape": "rect", "rect": [330, 210, 380, 260]})).expect("pix");
    }
    s
}

fn harness(k: usize) -> H {
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, session(k), UiSettings::default(), false);
    a.state.sync_render = true;
    let tmp = std::env::temp_dir().join("fp-uxb-paritaet");
    let _ = std::fs::create_dir_all(&tmp);
    a.state.file_override = Some(fp_ui::files::FilePickOverride { open: vec![], save: Some(tmp.join("p.fw.png")) });
    let mut h = Harness::builder().with_size(egui::vec2(1600.0, 1000.0)).with_pixels_per_point(1.0).with_step_dt(1.0 / 60.0).with_max_steps(12).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a);
    h.run_steps(2);
    h
}

/// One menu row as collected by [`walk`].
type MenuRow = (String, String, Option<String>, bool, Option<bool>);

fn walk(nodes: &[Node], path: &str, out: &mut Vec<MenuRow>) {
    for n in nodes {
        match n {
            Node::Separator => {}
            Node::Item { label, shortcut, action, enabled, checked } => {
                let cmd = match action {
                    ItemAction::Command(c) => c.clone(),
                    ItemAction::Ui(a) => format!("ui:{a:?}").chars().take(60).collect(),
                    ItemAction::None => "-".into(),
                };
                out.push((format!("{path} > {label}"), cmd, shortcut.clone(), *enabled, *checked));
            }
            Node::Sub { label, children, enabled } => {
                out.push((format!("{path} > {label} ▸"), "sub".into(), None, *enabled, None));
                walk(children, &format!("{path} > {label}"), out);
            }
        }
    }
}

fn menu_rows(h: &H) -> Vec<MenuRow> {
    let mut out = Vec::new();
    for (top, nodes) in fp_ui::menu::build_main_menu(&h.state().state) {
        walk(&nodes, &top, &mut out);
    }
    out
}

fn shell_off() {
    fp_engine::shell::disable();
}

#[test]
fn parity_menus() {
    shell_off();
    let mut tab: Vec<(String, String, Option<String>, Vec<String>)> = Vec::new();
    for k in 0..LAGEN.len() {
        let h = harness(k);
        for (i, (path, cmd, sc, en, ck)) in menu_rows(&h).into_iter().enumerate() {
            let st = format!("{}{}", if en { "an" } else { "aus" }, match ck { Some(true) => "✓", _ => "" });
            if k == 0 {
                tab.push((path, cmd, sc, vec![st]));
            } else if let Some(r) = tab.iter_mut().find(|r| r.0 == path) {
                r.3.push(st);
            } else {
                let _ = i;
            }
        }
    }
    let mut s = String::from("Pfad\tBefehl\tKürzel");
    for l in LAGEN {
        let _ = write!(s, "\t{l}");
    }
    s.push('\n');
    for (p, c, sc, st) in &tab {
        let _ = writeln!(s, "{p}\t{c}\t{}\t{}", sc.clone().unwrap_or_default(), st.join("\t"));
    }
    std::fs::write(out_dir().join("fp_menues.tsv"), s).expect("tsv");
    println!("UXB-P Menüzeilen: {}", tab.len());
}

fn esc(h: &mut H) {
    h.event(Event::Key { key: Key::Escape, physical_key: Some(Key::Escape), pressed: true, repeat: false, modifiers: Modifiers::NONE });
    h.run_steps(1);
    h.event(Event::Key { key: Key::Escape, physical_key: Some(Key::Escape), pressed: false, repeat: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

#[test]
fn parity_trigger() {
    shell_off();
    // alle Befehle des Hauptmenüs aus Lage 1 (für die Liste)
    let rows = menu_rows(&harness(1));
    let mut s = String::from("Pfad\tBefehl\tLage\tDialog\tVerlauf\tDokumente\tStatus/Fehler\n");
    let mut f = String::from("Dialog-ID\tTitel\tBefehl\tFeld\tArt\tBereich\tNachkomma\tSchieber\tEinheit\tVorgabe\tVorschau\n");
    let order = [2usize, 4, 5, 6, 3, 1, 0];
    let skip = ["file.exit", "file.quit"];
    for (path, cmd, _, _, _) in rows {
        if cmd == "sub" || cmd == "-" || cmd.starts_with("ui:") || skip.contains(&cmd.as_str()) {
            continue;
        }
        // erste Lage, in der der Befehl aktiv ist
        let mut lage = None;
        for k in order {
            let h = harness(k);
            if h.state().state.session.command_state(&cmd).0 || fp_ui::actions::ui_command_state(&h.state().state, &cmd).is_some_and(|x| x.0) {
                lage = Some((k, h));
                break;
            }
        }
        let Some((k, mut h)) = lage else {
            let _ = writeln!(s, "{path}\t{cmd}\tnie aktiv\t\t\t\t");
            continue;
        };
        let st0 = &h.state().state;
        let u0 = st0.session.active().map(|d| d.history.undo_count()).unwrap_or(0);
        let n0 = st0.session.documents().len();
        h.state_mut().state.run(&cmd, json!({}));
        h.run_steps(3);
        let st = &h.state().state;
        let dlg = if st.dialogs.any_open() { fp_ui::dialogs::last_dialog_id(&h.ctx).unwrap_or_else(|| "?".into()) } else { String::new() };
        let u1 = st.session.active().map(|d| d.history.undo_count()).unwrap_or(0);
        let n1 = st.session.documents().len();
        let status = st.status.as_ref().map(|s| s.0.clone()).unwrap_or_default().replace(['\t', '\n'], " ");
        let _ = writeln!(s, "{path}\t{cmd}\t{}\t{dlg}\t{}\t{}\t{status}", LAGEN[k], u1 as i64 - u0 as i64, n1 as i64 - n0 as i64);
        if let Some(fd) = &st.dialogs.form {
            for fl in &fd.fields {
                let (art, ber, dec, sl, unit) = match &fl.kind {
                    FieldKind::Number { min, max, decimals, slider, unit } => ("Zahl".to_string(), format!("{min}..{max}"), decimals.to_string(), slider.to_string(), unit.to_string()),
                    FieldKind::Choice(v) => ("Liste".into(), v.join(";"), String::new(), String::new(), String::new()),
                    FieldKind::Radio(v) => ("Optionen".into(), v.join(";"), String::new(), String::new(), String::new()),
                    other => (format!("{other:?}"), String::new(), String::new(), String::new(), String::new()),
                };
                let _ = writeln!(f, "{}\t{}\t{cmd}\t{}\t{art}\t{ber}\t{dec}\t{sl}\t{unit}\t{}\t{}", fd.id, fd.title, fl.label, fl.value, fd.preview);
            }
        }
        if st.dialogs.any_open() {
            esc(&mut h);
        }
    }
    std::fs::write(out_dir().join("fp_ausloesen.tsv"), s).expect("tsv");
    std::fs::write(out_dir().join("fp_formfelder.tsv"), f).expect("tsv");
    println!("UXB-P fertig");
}

/// Bedienfeldmenüs (≡) aller Bedienfelder in Lage „1 Rechteck“ (wie im Original gemessen).
#[test]
fn parity_panel_menus() {
    shell_off();
    let h = harness(2);
    let st = &h.state().state;
    let mut s = String::from("Bedienfeld\tEintrag\tZustand\n");
    for p in fp_ui::panels::PanelId::ALL {
        let nodes = fp_ui::menu::build_panel_menu(st, *p);
        let mut rows = Vec::new();
        walk(&nodes, p.title(), &mut rows);
        for (path, _cmd, sc, en, ck) in rows {
            let _ = writeln!(s, "{}\t{}\t{}{}{}", p.title(), path, if en { "an" } else { "grau" }, if ck == Some(true) { " ✓" } else { "" }, sc.map(|x| format!(" ⇥ {x}")).unwrap_or_default());
        }
    }
    std::fs::write(out_dir().join("fp_bedienfeldmenues.tsv"), s).expect("tsv");
}
