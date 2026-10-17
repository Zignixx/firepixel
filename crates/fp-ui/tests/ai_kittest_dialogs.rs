//! Aussehen der KI-Dialoge (Agent KI) in allen drei Designs bei 100 % und 150 %: „KI-Modell fehlt“
//! (Grundzustand, Download läuft, Fehler) und „KI-Modelle verwalten“ (4 von 6 vorhanden, ein Download
//! in der Zeile). Prüft den Aufbau (Beschriftungen, Knöpfe) und legt mit Grafikadapter Bilder nach
//! `docs\screenshots\phase5_ki_dialog_*.png` (`-- --ignored` für die Bilder).

use egui_kittest::Harness;
use egui_kittest::kittest::{By, Queryable};
use fp_ai::models::{self, ModelId};
use fp_engine::commands::ai::fp_ai;
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use fp_ui::dialogs::models::{ManagerDialog, MissingDialog, demo_download};
use fp_ui::theme::ThemeKind;
use serde_json::json;

/// Modellordner mit 4 der 6 Modelle (harte Links auf die vorhandenen Dateien, nichts wird kopiert).
fn partial_folder() -> Option<std::path::PathBuf> {
    let src = models::local_folder()?;
    let dir = std::env::temp_dir().join(format!("fp-ai-dialoge-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    for id in [
        ModelId::Sam2Tiny,
        ModelId::Sam2Small,
        ModelId::BiRefNetLite,
        ModelId::BiRefNetLiteMatting,
    ] {
        let f = models::info(id).file;
        let (a, b) = (src.join(f), dir.join(f));
        if !b.exists() && std::fs::hard_link(&a, &b).is_err() {
            return None;
        }
    }
    Some(dir)
}

fn harness(kind: ThemeKind, ppp: f32) -> Harness<'static, FirepixelApp> {
    fp_engine::shell::disable();
    fp_ai::download::disable();
    let mut s = fp_engine::Session::default();
    s.prefs.theme = kind.to_engine();
    let _ = s.execute(
        "file.new",
        json!({"width": 300, "height": 200, "background": "#ffffff"}),
    );
    let ctx = egui::Context::default();
    let settings = UiSettings {
        theme: kind,
        ..UiSettings::default()
    };
    let mut app = FirepixelApp::with_session(&ctx, s, settings, false);
    app.state.set_theme(kind);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1500.0, 950.0))
        .with_pixels_per_point(ppp)
        .with_max_steps(8)
        .wgpu()
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn shot(h: &mut Harness<'static, FirepixelApp>, name: &str, save: bool) {
    h.run_steps(3);
    if !save {
        return;
    }
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("docs")
        .join("screenshots");
    match h.render() {
        Ok(i) => {
            let _ = i.save(out.join(format!("phase5_ki_dialog_{name}.png")));
        }
        Err(e) => eprintln!("kein Grafikadapter: {e}"),
    }
}

fn missing(model: ModelId) -> MissingDialog {
    MissingDialog {
        model,
        pending: Some(fp_engine::tools::Invocation::new(
            "select.aiSubject",
            json!({}),
        )),
        download: None,
        error: None,
        done: None,
    }
}

fn run(save: bool) {
    let dir = partial_folder();
    if let Some(d) = &dir {
        models::set_search_override(vec![d.clone()]);
    }
    for (kind, kname) in [
        (ThemeKind::Normal, "normal"),
        (ThemeKind::Dark, "dunkel"),
        (ThemeKind::Oled, "oled"),
    ] {
        for (ppp, pname) in [(1.0f32, "100"), (1.5, "150")] {
            let mut h = harness(kind, ppp);
            // „KI-Modell fehlt“ – Grundzustand
            h.state_mut().state.dialogs.ai.missing = Some(missing(ModelId::BiRefNetLite));
            h.run_steps(3);
            for l in [
                "Modell",
                "Aufgabe",
                "Größe",
                "Lizenz",
                "Quelle",
                "Prüfsumme",
                "Zielordner",
                "Herunterladen",
                "Abbrechen",
                "Kopieren",
            ] {
                assert!(
                    h.get_all(By::new().label(l)).next().is_some(),
                    "{kname}/{pname}: „{l}“ fehlt"
                );
            }
            assert!(
                h.get_all(By::new().label("huggingface.co/ZhengPeng7/BiRefNet_lite"))
                    .next()
                    .is_some(),
                "kurze Quelle"
            );
            assert!(
                h.get_all(By::new().label_contains("nichtkommerzielle"))
                    .next()
                    .is_some(),
                "Trainingsdaten-Hinweis"
            );
            shot(&mut h, &format!("fehlt_{kname}_{pname}"), save);
            // Download läuft
            if let Some(m) = h.state_mut().state.dialogs.ai.missing.as_mut() {
                m.download = Some(demo_download(ModelId::BiRefNetLite, 0.17));
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
            h.run_steps(3);
            assert!(
                h.get_all(By::new().label_contains("von 178 MB"))
                    .next()
                    .is_some(),
                "Fortschrittstext"
            );
            shot(
                &mut h,
                &format!("fehlt_download_{kname}_{pname}"),
                save && pname == "100",
            );
            if let Some(m) = h.state_mut().state.dialogs.ai.missing.as_mut()
                && let Some(d) = m.download.take()
            {
                drop(d);
            }
            // Fehler mit „Erneut versuchen“
            if let Some(m) = h.state_mut().state.dialogs.ai.missing.as_mut() {
                m.error = Some(
                    "Download fehlgeschlagen: Verbindung zu huggingface.co unterbrochen".into(),
                );
            }
            h.run_steps(3);
            assert!(
                h.get_all(By::new().label("Erneut versuchen"))
                    .next()
                    .is_some()
            );
            shot(
                &mut h,
                &format!("fehlt_fehler_{kname}_{pname}"),
                save && pname == "100",
            );
            h.state_mut().state.dialogs.ai.missing = None;
            // „KI-Modelle verwalten“
            let mut d = ManagerDialog::default();
            if dir.is_some() {
                d.download = Some(demo_download(ModelId::BiRefNet, 0.42));
            }
            h.state_mut().state.dialogs.ai.manager = Some(d);
            std::thread::sleep(std::time::Duration::from_millis(200));
            h.run_steps(4);
            for l in [
                "Speicherort",
                "Durchsuchen…",
                "Standard",
                "Ordner öffnen",
                "Zwischenspeicher leeren",
                "Schließen",
            ] {
                assert!(
                    h.get_all(By::new().label(l)).next().is_some(),
                    "{kname}/{pname}: „{l}“ fehlt"
                );
            }
            if dir.is_some() {
                assert!(
                    h.get_all(By::new().label_contains("4 von 6 Modellen vorhanden"))
                        .next()
                        .is_some(),
                    "Zusammenfassung"
                );
                assert!(h.get_all(By::new().label("✓ Vorhanden")).count() >= 4);
            }
            shot(&mut h, &format!("verwalten_{kname}_{pname}"), save);
            h.state_mut().state.dialogs.ai.manager = None;
            h.run_steps(2);
        }
    }
    models::set_search_override(Vec::new());
    if let Some(d) = dir {
        let _ = std::fs::remove_dir_all(d);
    }
}

#[test]
fn ki_dialoge_aufbau_in_allen_designs() {
    run(false);
}

/// `cargo test -p fp-ui --test ai_kittest_dialogs bilder -- --ignored`
#[test]
#[ignore]
fn bilder() {
    run(true);
}
