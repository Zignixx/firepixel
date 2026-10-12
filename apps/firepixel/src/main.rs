//! Firepixel: Grafik- und Webdesign-Programm.
//!
//! Aufruf: `firepixel [--design=normal|dark|oled] [--mcp[=PORT]] [--help] [Datei …]`
//! (Hilfetext: Schlüssel `ui-cli-help` in `crates\fp-i18n\locales\<sprache>\oberflaeche.ftl`)
//!
//! * `--design=…` überschreibt das gespeicherte Design (z. B. für Bildschirmfotos).
//! * `--mcp[=PORT]` startet den KI-Steuerkanal (MCP über HTTP, nur 127.0.0.1, mit Token; siehe
//!   `docs\AGENTS.md`). Ohne Port: Vorgabe 47800 bzw. ein freier Port.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use fp_ui::theme::ThemeKind;

fn main() -> eframe::Result {
    // alte deutsche Ordnernamen (Sprachen, Modelle, …) beim ersten Zugriff einmal auf die englischen
    // umbenennen (AGENTEN_REGELN §1c; nicht in Testläufen mit FP_NO_SHELL_OPEN)
    fp_i18n::paths::enable_migration();
    let mut design = None;
    let mut files = Vec::new();
    for a in std::env::args_os().skip(1) {
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
    fp_ui::run(files, design)
}
