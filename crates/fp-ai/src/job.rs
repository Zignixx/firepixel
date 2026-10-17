//! Hintergrund-Aufträge mit Fortschritt und Abbrechen (die Oberfläche fragt je Frame nach).

use crate::cancel::Progress;
use crate::error::{AiError, Result};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Instant;

/// Ein laufender Auftrag.
pub struct Job<T> {
    rx: Option<Receiver<Result<T>>>,
    /// Fortschritt (0…1) und Abbruch-Merker.
    pub progress: Progress,
    /// Beschriftung für die Statuszeile.
    pub label: String,
    started: Instant,
}

impl<T: Send + 'static> Job<T> {
    /// Startet `f` in einem eigenen Thread (Rechnen im KI-Thread-Pool).
    pub fn spawn(
        label: impl Into<String>,
        f: impl FnOnce(&Progress) -> Result<T> + Send + 'static,
    ) -> Job<T> {
        let progress = Progress::new();
        let p2 = progress.clone();
        let (tx, rx) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("fp-ai-job".into())
            .spawn(move || {
                let r = crate::nn::pool().install(|| f(&p2));
                let _ = tx.send(r);
            });
        let rx = match spawned {
            Ok(_) => Some(rx),
            Err(e) => {
                let (tx2, rx2) = mpsc::channel();
                let _ = tx2.send(Err(AiError::Input(fp_i18n::tr!(
                    "ai-err-d-thread",
                    error = e.to_string()
                ))));
                Some(rx2)
            }
        };
        Job {
            rx,
            progress,
            label: label.into(),
            started: Instant::now(),
        }
    }
}

impl<T> Job<T> {
    /// Abbrechen anfordern (das Ergebnis kommt dann als `Err(Cancelled)`).
    pub fn cancel(&self) {
        self.progress.cancel.cancel();
    }

    /// Ergebnis abholen, falls fertig (danach liefert der Auftrag nichts mehr).
    pub fn poll(&mut self) -> Option<Result<T>> {
        let rx = self.rx.as_ref()?;
        match rx.try_recv() {
            Ok(r) => {
                self.rx = None;
                Some(r)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.rx = None;
                Some(Err(AiError::Input(fp_i18n::t("ai-err-d-job-cancelled"))))
            }
        }
    }

    /// Blockierend warten (Tests, CLI).
    pub fn wait(mut self) -> Result<T> {
        match self.rx.take() {
            Some(rx) => rx
                .recv()
                .unwrap_or_else(|_| Err(AiError::Input(fp_i18n::t("ai-err-d-job-cancelled")))),
            None => Err(AiError::Input(fp_i18n::t("ai-err-d-result-taken"))),
        }
    }

    /// Läuft noch?
    pub fn running(&self) -> bool {
        self.rx.is_some()
    }

    /// Laufzeit in Sekunden.
    pub fn elapsed(&self) -> f32 {
        self.started.elapsed().as_secs_f32()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_midway() {
        let j = Job::spawn("Test", |p| {
            for i in 0..1000 {
                p.step(0.0, 1.0, i as f32 / 1000.0)?;
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            Ok(42)
        });
        std::thread::sleep(std::time::Duration::from_millis(20));
        j.cancel();
        assert!(matches!(j.wait(), Err(AiError::Cancelled)));
        let j = Job::spawn("Test", |_| Ok(7));
        assert_eq!(j.wait().unwrap_or(0), 7);
    }
}
