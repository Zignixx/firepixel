//! Firepixel: Grafik- und Webdesign-Programm.
//!
//! Aufruf: `firepixel [--design=normal|dark|oled] [--mcp[=PORT]] [--batch [rezept] [dateien …]] [--help] [Datei …]`
//! (Hilfetext: Schlüssel `ui-cli-help` in `crates\fp-i18n\locales\<sprache>\ui.ftl`)
//!
//! * `--design=…` überschreibt das gespeicherte Design (z. B. für Bildschirmfotos).
//! * `--mcp[=PORT]` startet den KI-Steuerkanal (MCP über HTTP, nur 127.0.0.1, mit Token; siehe
//!   `docs\AGENTS.md`). Ohne Port: Vorgabe 47800 bzw. ein freier Port.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use fp_ui::theme::ThemeKind;

fn main() -> eframe::Result {
    // Startzeit-Messung (nur mit FP_STARTUP_TRACE, Nutzerbefund N30)
    fp_i18n::startup::start();
    // alte deutsche Ordnernamen (Sprachen, Modelle, …) beim ersten Zugriff einmal auf die englischen
    // umbenennen (AGENTEN_REGELN §1c; nicht in Testläufen mit FP_NO_SHELL_OPEN)
    fp_i18n::paths::enable_migration();
    let mut design = None;
    let mut files = Vec::new();
    // `--batch [rezept] [dateien/ordner …]` (BATCH-9): Batch-Studio mit Rezept (Pfad zu *.fpbatch.json
    // oder Name einer eingebauten Vorlage, auch `--batch=<rezept>`) und den folgenden Dateien/Ordnern.
    let mut batch: Option<(Option<String>, Vec<std::path::PathBuf>)> = None;
    for a in std::env::args_os().skip(1) {
        if let Some((recipe, list)) = batch.as_mut() {
            let s = a.to_string_lossy().to_string();
            if !s.starts_with("--") {
                let p = std::path::PathBuf::from(&a);
                if recipe.is_none()
                    && list.is_empty()
                    && (s.to_lowercase().ends_with(".fpbatch.json") || !p.exists())
                {
                    *recipe = Some(s);
                } else if p.exists() {
                    list.push(p);
                }
                continue;
            }
        }
        if let Some(r) = a.to_string_lossy().strip_prefix("--batch") {
            let r = r.trim_start_matches('=');
            batch = Some(((!r.is_empty()).then(|| r.to_string()), Vec::new()));
            continue;
        }
        let s = a.to_string_lossy().to_string();
        if matches!(s.as_str(), "--help" | "-h" | "/?") {
            // Hilfetext in der Sprache der Oberfläche (ohne Konsole nur bei umgeleiteter Ausgabe sichtbar)
            println!("{}", fp_ui::cli_help());
            return Ok(());
        }
        if let Some(d) = s.strip_prefix("--design=") {
            design = match d.to_lowercase().as_str() {
                "normal" => Some(ThemeKind::Normal),
                "dark" | "dunkel" => Some(ThemeKind::Dark), // `dunkel`: alter Wert, weiter gültig
                "oled" => Some(ThemeKind::Oled),
                _ => None,
            };
        } else if let Some(p) = s.strip_prefix("--mcp") {
            fp_ui::bridge::enable(p.trim_start_matches('=').parse().unwrap_or(0));
        } else {
            // Dateien aus der Kommandozeile (auch per Drag & Drop auf die exe) öffnen
            let p = std::path::PathBuf::from(a);
            if p.is_file() {
                files.push(p);
            }
        }
    }
    if let Some((recipe, list)) = batch {
        fp_ui::batch::request_launch(recipe, list);
    }
    fp_ui::run(files, design)
}
