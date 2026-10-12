//! Abbrechen und Fortschritt langer Berechnungen.

use crate::error::{AiError, Result};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

/// Abbruch-Merker (zwischen Threads geteilt).
#[derive(Clone, Debug, Default)]
pub struct Cancel(Arc<AtomicBool>);

impl Cancel {
    /// Neuer, nicht gesetzter Merker.
    pub fn new() -> Cancel {
        Cancel::default()
    }
    /// Abbrechen anfordern.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
    /// Wurde abgebrochen?
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
    /// `Err(Cancelled)`, wenn abgebrochen wurde.
    pub fn check(&self) -> Result<()> {
        if self.is_cancelled() { Err(AiError::Cancelled) } else { Ok(()) }
    }
}

/// Stufe einer KI-Rechnung (für die Fortschrittsanzeige; nur Stufen, die der Code wirklich
/// unterscheidet).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Stage {
    /// Noch nichts begonnen.
    #[default]
    Start,
    /// Gewichte werden aus dem Modellordner geladen (kein Teilfortschritt bekannt).
    LoadModel,
    /// Das Netz rechnet (Bild-Encoder bzw. Freistellung, Fortschritt 0…1).
    Analyze,
    /// Nachbearbeitung zur Maske (Kantenbereich, Skalierung; kein Teilfortschritt).
    Mask,
}

impl Stage {
    fn from_u32(v: u32) -> Stage {
        match v {
            1 => Stage::LoadModel,
            2 => Stage::Analyze,
            3 => Stage::Mask,
            _ => Stage::Start,
        }
    }
    fn to_u32(self) -> u32 {
        match self {
            Stage::Start => 0,
            Stage::LoadModel => 1,
            Stage::Analyze => 2,
            Stage::Mask => 3,
        }
    }
}

/// Fortschritt 0…1 (zwischen Threads geteilt) mit Abbruch-Merker und Stufe.
#[derive(Clone, Debug, Default)]
pub struct Progress {
    value: Arc<AtomicU32>,
    stage: Arc<AtomicU32>,
    /// Abbruch-Merker dieses Vorgangs.
    pub cancel: Cancel,
}

impl Progress {
    /// Neuer Fortschritt (0).
    pub fn new() -> Progress {
        Progress::default()
    }
    /// Fortschritt mit vorhandenem Abbruch-Merker.
    pub fn with_cancel(cancel: Cancel) -> Progress {
        Progress { value: Arc::default(), stage: Arc::default(), cancel }
    }
    /// Setzt den Fortschritt (0…1).
    pub fn set(&self, v: f32) {
        self.value.store(v.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }
    /// Aktueller Fortschritt.
    pub fn get(&self) -> f32 {
        f32::from_bits(self.value.load(Ordering::Relaxed))
    }
    /// Setzt die Stufe.
    pub fn set_stage(&self, s: Stage) {
        self.stage.store(s.to_u32(), Ordering::Relaxed);
    }
    /// Aktuelle Stufe.
    pub fn stage(&self) -> Stage {
        Stage::from_u32(self.stage.load(Ordering::Relaxed))
    }
    /// Setzt den Fortschritt `a + (b - a) * t` und prüft auf Abbruch.
    pub fn step(&self, a: f32, b: f32, t: f32) -> Result<()> {
        self.set(a + (b - a) * t);
        self.cancel.check()
    }
}
