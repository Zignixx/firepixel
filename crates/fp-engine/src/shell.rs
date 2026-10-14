//! Zentrale Stelle für alles, was ein Fremdprogramm startet (Datei mit dem zugeordneten Programm
//! öffnen, Browser, E-Mail-Entwurf, Drucken, Hilfe, Explorer, Windows-Bilderfassung).
//!
//! Nichts wird gestartet, wenn
//! * die Umgebungsvariable `FP_NO_SHELL_OPEN` (oder die ältere `FP_NO_LAUNCH`) gesetzt ist oder
//! * der Prozess [`disable`] aufgerufen hat (Tests: `std::env::set_var` ist in Edition 2024
//!   `unsafe` und im Workspace verboten, deshalb dieser Schalter).
//!
//! Die Befehle liefern dann trotzdem die erzeugten Dateien und melden `launched: false`.

use crate::error::{EngineError, Result};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

static DISABLED: AtomicBool = AtomicBool::new(false);

thread_local! {
    /// Abgeschaltete Starts in diesem Thread (siehe [`suppressed_in_thread`]).
    static SUPPRESSED: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Wie viele Fremdprogramm-Starts in diesem Thread unterdrückt wurden (abgeschaltet). Gesamttests
/// erkennen daran Befehle, deren einzige Wirkung ein Fremdprogramm ist (z. B. ein Skript, das den
/// Browser öffnet), und melden sie als „im echten Fenster prüfen“ statt „keine Wirkung“ (B-6).
pub fn suppressed_in_thread() -> u64 {
    SUPPRESSED.with(|c| c.get())
}

fn note_suppressed() {
    SUPPRESSED.with(|c| c.set(c.get() + 1));
}

/// Wie [`allowed`]; zählt ein abgeschaltetes Starten zusätzlich mit ([`suppressed_in_thread`]). Für
/// Befehle, die vor dem Start selbst prüfen (Vorschau im Browser, E-Mail-Entwurf).
pub fn allowed_or_count() -> bool {
    let ok = allowed();
    if !ok {
        note_suppressed();
    }
    ok
}

/// Schaltet das Starten von Fremdprogrammen für diesen Prozess ab (Tests, Stapelbetrieb).
pub fn disable() {
    DISABLED.store(true, Ordering::SeqCst);
}

/// Darf ein Fremdprogramm gestartet werden?
pub fn allowed() -> bool {
    !DISABLED.load(Ordering::SeqCst) && std::env::var_os("FP_NO_SHELL_OPEN").is_none() && std::env::var_os("FP_NO_LAUNCH").is_none()
}

/// Startet einen vorbereiteten Prozess (ohne zu warten). `Ok(false)` = abgeschaltet.
pub fn spawn(mut c: std::process::Command) -> Result<bool> {
    if !allowed() {
        note_suppressed();
        return Ok(false);
    }
    c.spawn().map(|_| true).map_err(|e| EngineError::Failed(fp_i18n::tr!("err-program-start", detail = e.to_string())))
}

/// Startet einen Prozess und wartet auf seine Ausgabe. `Ok(None)` = abgeschaltet.
pub fn run(mut c: std::process::Command) -> Result<Option<std::process::Output>> {
    if !allowed() {
        note_suppressed();
        return Ok(None);
    }
    c.output().map(Some).map_err(|e| EngineError::Failed(fp_i18n::tr!("err-program-start", detail = e.to_string())))
}

/// Öffnet eine Datei bzw. einen Ordner mit dem zugeordneten Programm (oder mit `program`).
/// `Ok(false)` = abgeschaltet.
pub fn shell_open(path: &Path, program: Option<&str>) -> Result<bool> {
    let c = match program {
        Some(prog) if !prog.is_empty() => {
            let mut c = std::process::Command::new(prog);
            c.arg(path);
            c
        }
        _ => {
            #[cfg(windows)]
            let mut c = std::process::Command::new("explorer.exe");
            #[cfg(target_os = "macos")]
            let mut c = std::process::Command::new("open");
            #[cfg(not(any(windows, target_os = "macos")))]
            let mut c = std::process::Command::new("xdg-open");
            c.arg(path);
            c
        }
    };
    spawn(c)
}

#[cfg(test)]
mod tests {
    #[test]
    fn suppressed_launch_is_counted_per_thread() {
        super::disable();
        let n = super::suppressed_in_thread();
        assert_eq!(super::spawn(std::process::Command::new("firepixel-does-not-exist")).ok(), Some(false));
        assert!(!super::allowed_or_count());
        assert_eq!(super::suppressed_in_thread(), n + 2);
        std::thread::spawn(|| assert_eq!(super::suppressed_in_thread(), 0)).join().expect("Thread");
    }
}
