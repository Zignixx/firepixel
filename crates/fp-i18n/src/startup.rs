//! Startzeit-Messung (Nutzerbefund N30, START-1).
//!
//! Ist die Umgebungsvariable `FP_STARTUP_TRACE` gesetzt, schreibt [`mark`] je Startphase eine Zeile
//! `startup <ms seit Programmstart> +<ms seit voriger Marke> epoch=<Unix-ms> <phase>`:
//! bei einem Pfad (`FP_STARTUP_TRACE=C:\temp\start.log`) angehängt an diese Datei, sonst (`=1`) auf
//! stderr. Die Unix-Zeit erlaubt, die Zeit vor `main` (exe laden) von außen zu messen.
//! Ohne die Variable kostet eine Marke nur eine Abfrage eines `OnceLock`.

use std::io::Write;
use std::sync::{Mutex, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

struct Trace {
    t0: Instant,
    last: Mutex<Instant>,
    file: Option<Mutex<std::fs::File>>,
}

fn trace() -> Option<&'static Trace> {
    static T: OnceLock<Option<Trace>> = OnceLock::new();
    T.get_or_init(|| {
        let v = std::env::var_os("FP_STARTUP_TRACE")?;
        let s = v.to_string_lossy().to_string();
        if s.is_empty() || s == "0" {
            return None;
        }
        let file = if s.contains(['\\', '/']) {
            std::fs::OpenOptions::new().create(true).append(true).open(&s).ok().map(Mutex::new)
        } else {
            None
        };
        let now = Instant::now();
        Some(Trace { t0: now, last: Mutex::new(now), file })
    })
    .as_ref()
}

/// Ist die Startzeit-Messung eingeschaltet?
pub fn enabled() -> bool {
    trace().is_some()
}

/// Beginn der Messung (erste Zeile in `main`); weitere Aufrufe wirken wie [`mark`].
pub fn start() {
    mark("main");
}

/// Zeitmarke für die Phase `phase` (nur mit `FP_STARTUP_TRACE`).
pub fn mark(phase: &str) {
    emit(phase, None);
}

/// Schreibt eine Zeile; `own` = eigene Dauer der Phase (sonst die Zeit seit der vorigen Marke,
/// die bei Phasen in mehreren Threads nicht die Dauer ist).
fn emit(phase: &str, own: Option<std::time::Duration>) {
    let Some(t) = trace() else { return };
    let now = Instant::now();
    let since_last = match t.last.lock() {
        Ok(mut l) => {
            let d = now.duration_since(*l);
            *l = now;
            d
        }
        Err(_) => Default::default(),
    };
    let since_last = own.unwrap_or(since_last);
    let epoch = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    let line = format!(
        "startup {:9.1} +{:8.1} epoch={epoch} {phase}\n",
        now.duration_since(t.t0).as_secs_f64() * 1000.0,
        since_last.as_secs_f64() * 1000.0
    );
    match &t.file {
        Some(f) => {
            if let Ok(mut f) = f.lock() {
                let _ = f.write_all(line.as_bytes());
            }
        }
        None => {
            let _ = std::io::stderr().write_all(line.as_bytes());
        }
    }
}

/// Misst `f` als Phase: Marke `<phase> (begin)` davor, danach `<phase>` mit der eigenen Dauer in
/// der `+`-Spalte (richtig auch für Phasen in Hintergrund-Threads).
pub fn time<R>(phase: &str, f: impl FnOnce() -> R) -> R {
    if trace().is_none() {
        return f();
    }
    mark(&format!("{phase} (begin)"));
    let start = Instant::now();
    let r = f();
    emit(phase, Some(start.elapsed()));
    r
}

#[cfg(test)]
mod tests {
    /// Ohne `FP_STARTUP_TRACE` (Testlauf) ist die Messung aus und `time` gibt das Ergebnis durch.
    #[test]
    fn disabled_without_env() {
        if std::env::var_os("FP_STARTUP_TRACE").is_some() {
            return;
        }
        assert!(!super::enabled());
        super::mark("x");
        assert_eq!(super::time("y", || 41 + 1), 42);
    }
}
