//! FIX-E (CT-3): `ui_action {"run": "file.open"}` bzw. `"file.saveAs"` über die Brücke ließ die App
//! hängen. Ursache: `bridge::frame` hält die Sperre der Brücke, während es Aufträge abarbeitet; der
//! Datei-Dialog fragt über `bridge::running_url()` nach, ob er in der App erscheinen soll, und
//! `running_url` wollte dieselbe Sperre → der UI-Thread wartete auf sich selbst.
//!
//! Eigener Testprozess (Cargo.toml): die Brücke ist prozessweit eingehängt (wie `--mcp`) und schaltet
//! alle Datei-Dialoge auf den Dialog in der App um. Ablauf genau wie in `docs\AGENTS.md` §5b.

use egui_kittest::Harness;
use fp_mcp::{SessionHost, UiReply, UiRequest};
use fp_ui::app_state::UiSettings;
use fp_ui::FirepixelApp;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;

type H = Harness<'static, FirepixelApp>;

fn harness(s: fp_engine::Session) -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1200.0, 800.0)).with_pixels_per_point(1.0).with_max_steps(12).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(2);
    h
}

/// Wächter: hängt der UI-Thread (alter Fehler), beendet er den Testprozess mit Meldung statt ewig
/// zu warten.
fn watchdog(secs: u64) -> Arc<std::sync::atomic::AtomicBool> {
    let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let d = done.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(secs));
        if !d.load(std::sync::atomic::Ordering::SeqCst) {
            eprintln!("bridge_file_dialog: UI-Thread hängt (ui_action mit Datei-Dialog) – Verklemmung");
            std::process::exit(101);
        }
    });
    done
}

fn ask(h: &mut H, host: &fp_mcp::BridgeHost, req: UiRequest) -> Value {
    let (tx, rx) = std::sync::mpsc::channel();
    host.ui_request(req, Box::new(move |r| {
        let _ = tx.send(r);
    }))
    .unwrap_or_else(|e| panic!("{e}"));
    for _ in 0..200 {
        h.step();
        if let Ok(r) = rx.try_recv() {
            return match r {
                UiReply::Json(v) | UiReply::JsonImages(v, _) => v,
                UiReply::Error(e) => json!({"error": e}),
                other => panic!("{other:?}"),
            };
        }
    }
    panic!("keine Antwort der Brücke");
}

#[test]
fn ui_action_open_and_save_as_through_the_bridge_show_the_in_app_dialog() {
    let done = watchdog(60);
    let dir = std::env::temp_dir().join(format!("fp-fixe-bridge-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("Testordner");
    let src = dir.join("quelle.fw.png");
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 40, "height": 30})).expect("doc");
    s.execute("file.saveAs", json!({"path": src.to_string_lossy()})).expect("save");
    s.execute("file.close", json!({})).expect("close");
    let mut h = harness(s);
    let host = fp_ui::bridge::install_detached_for_tests(&h.ctx);
    assert!(fp_ui::bridge::running_url().is_some());

    // 1. Öffnen über ui_action: Antwort kommt, Dialog in der App wartet
    let r = ask(&mut h, &host, UiRequest::Action(json!({"run": "file.open"})));
    assert_eq!(r["ok"], true, "{r}");
    let fd = ask(&mut h, &host, UiRequest::FileDialog(json!({})));
    assert_eq!(fd["open"]["mode"], "internal", "{fd}");
    assert_eq!(fd["open"]["kind"], "open_many", "{fd}");
    let st = ask(&mut h, &host, UiRequest::State);
    assert_eq!(st["dialog_open"], true, "{st}");
    // beantworten wie ein Agent
    let r = ask(&mut h, &host, UiRequest::FileDialog(json!({"action": "choose", "paths": [src.display().to_string()]})));
    assert!(r.get("error").is_none(), "{r}");
    h.run_steps(3);
    assert!(h.state().state.session.documents().iter().any(|d| d.path.as_deref() == Some(src.as_path())), "Datei geöffnet");
    assert!(fp_ui::file_dialog::status_json()["open"].is_null());

    // 2. Speichern unter über ui_action (Dokument ändern, damit der Titel einen Stern hat)
    h.state_mut().state.session.execute("draw.rectangle", json!({"rect": [1, 1, 10, 10]})).expect("rect");
    let r = ask(&mut h, &host, UiRequest::Action(json!({"run": "file.saveAs"})));
    assert_eq!(r["ok"], true, "{r}");
    let fd = ask(&mut h, &host, UiRequest::FileDialog(json!({})));
    assert_eq!(fd["open"]["mode"], "internal", "{fd}");
    assert_eq!(fd["open"]["kind"], "save", "{fd}");
    let out = dir.join("kopie.fw.png");
    let r = ask(&mut h, &host, UiRequest::FileDialog(json!({"action": "choose", "path": out.display().to_string()})));
    assert!(r.get("error").is_none(), "{r}");
    h.run_steps(4);
    assert!(out.is_file(), "gespeichert: {out:?}");
    let st = ask(&mut h, &host, UiRequest::State);
    let active = st["documents"].as_array().and_then(|a| a.iter().find(|d| d["active"] == true).cloned()).expect("aktives Dokument");
    assert_eq!(active["dirty"], false, "{st}");

    // 3. Wiederholung (§4b): zweimal hintereinander ui_action file.open → nur ein Dialog, der zweite
    //    wird als „busy“ abgelehnt, nichts hängt
    let _ = ask(&mut h, &host, UiRequest::Action(json!({"run": "file.open"})));
    let _ = ask(&mut h, &host, UiRequest::Action(json!({"run": "file.open"})));
    let fd = ask(&mut h, &host, UiRequest::FileDialog(json!({})));
    assert_eq!(fd["open"]["kind"], "open_many", "{fd}");
    let r = ask(&mut h, &host, UiRequest::FileDialog(json!({"action": "cancel"})));
    assert!(r.get("error").is_none(), "{r}");
    h.run_steps(2);
    assert!(fp_ui::file_dialog::status_json()["open"].is_null());

    // 4. Steuerkanal ausschalten (Menü Befehle ▸ KI-Steuerkanal): URL ist danach weg
    fp_ui::bridge::disable();
    h.run_steps(2);
    assert!(fp_ui::bridge::running_url().is_none(), "Steuerkanal aus");

    done.store(true, std::sync::atomic::Ordering::SeqCst);
    let _ = std::fs::remove_dir_all(&dir);
}
