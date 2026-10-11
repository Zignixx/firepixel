//! Firepixel: Grafik- und Webdesign-Programm.
//!
//! Aufruf: `firepixel [--design=normal|dunkel|oled] [--mcp[=PORT]] [Datei …]`
//!
//! * `--design=…` überschreibt das gespeicherte Design (z. B. für Bildschirmfotos).
//! * `--mcp[=PORT]` startet den KI-Steuerkanal (MCP über HTTP, nur 127.0.0.1, mit Token; siehe
//!   `docs\AGENTS.md`). Ohne Port: Vorgabe 47800 bzw. ein freier Port.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use fp_ui::theme::ThemeKind;

fn main() -> eframe::Result {
    let mut design = None;
    let mut files = Vec::new();
    for a in std::env::args_os().skip(1) {
        let s = a.to_string_lossy().to_string();
        if let Some(d) = s.strip_prefix("--design=") {
            design = match d.to_lowercase().as_str() {
                "normal" => Some(ThemeKind::Normal),
                "dunkel" | "dark" => Some(ThemeKind::Dark),
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
