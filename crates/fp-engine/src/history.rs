//! Undo/Redo-Verlauf wie das Verlauf-Bedienfeld (Spezifikation 10 §3).
//!
//! Das Original speichert je Änderungsart ein Befehlsobjekt (157 Undo-Klassen). Firepixel nutzt statt
//! dessen **Copy-on-Write-Schnappschüsse**: Das Dokument liegt in einem `Arc`, alle Knoten darunter
//! ebenfalls (fp-model). Ein Schritt hält den Zustand vor und nach dem Befehl; unveränderte Teilbäume
//! werden geteilt, so dass ein Schritt nur die geänderten Knoten kostet.
//!
//! Verhalten wie das Original (B):
//! - Ein Schritt je Befehl (auch ein ganzes Skript = ein Schritt, Laufzeit T6).
//! - Reine Auswahländerungen sind eigene Schritte („Alles auswählen“ = `CSelectAllUndoable`).
//! - Nach dem Abschluss eines Schritts wird der Redo-Stapel geleert; ist der Undo-Stapel länger als
//!   `MaxUndos`, fällt **genau der älteste** Schritt weg (`@00513690`, Laufzeit T6: 2, 4, 5, 5, 5 …).
//! - Jeder Schritt speichert Befehlsnamen und Parameter, damit „Schritte wiederholen“, „Schritte
//!   kopieren“ und „Als Befehl speichern“ funktionieren ([`History::export`]).

use crate::selection::Selection;
use fp_model::Document;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Zustand eines Dokuments zu einem Zeitpunkt.
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub doc: Arc<Document>,
    pub selection: Selection,
    /// Aktuelle Seite, Status und Ebene (gehören zum Verlauf, weil Befehle sie ändern).
    pub page: usize,
    pub frame: usize,
    pub layer: Option<fp_model::LayerId>,
}

/// Ein aufgezeichneter Befehlsaufruf (für Aktionen, MCP und „Befehl speichern“).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecordedCommand {
    /// Befehlsname.
    pub command: String,
    /// Parameter (JSON-Objekt).
    #[serde(default)]
    pub params: serde_json::Value,
}

/// Ein Verlaufsschritt.
#[derive(Clone, Debug)]
pub struct HistoryStep {
    /// Beschriftung im Verlauf-Bedienfeld (deutsch).
    pub label: String,
    /// Befehlsname.
    pub command: String,
    /// Parameter des Aufrufs.
    pub params: serde_json::Value,
    /// Kann der Schritt wiederholt/aufgezeichnet werden (Original: Mausaktionen ohne JS-Entsprechung
    /// sind es nicht)? In Firepixel ist jeder Befehlsaufruf aufzeichnungsfähig.
    pub recordable: bool,
    pub before: Snapshot,
    pub after: Snapshot,
}

/// Undo/Redo-Verlauf eines Dokuments.
#[derive(Clone, Debug)]
pub struct History {
    undo: Vec<HistoryStep>,
    redo: Vec<HistoryStep>,
    max_undos: usize,
    dropped: usize,
}

impl History {
    /// Leerer Verlauf mit Höchstzahl `max_undos` (Voreinstellung `MaxUndos`, Vorgabe 20).
    pub fn new(max_undos: usize) -> History {
        History { undo: Vec::new(), redo: Vec::new(), max_undos, dropped: 0 }
    }

    /// Höchstzahl der Schritte.
    pub fn max_undos(&self) -> usize {
        self.max_undos
    }

    /// Ändert die Höchstzahl (wirkt beim nächsten Schritt wie im Original).
    pub fn set_max_undos(&mut self, n: usize) {
        self.max_undos = n;
    }

    /// Fügt einen abgeschlossenen Schritt an, leert den Redo-Stapel und kürzt auf `MaxUndos`.
    pub fn push(&mut self, step: HistoryStep) {
        self.redo.clear();
        self.undo.push(step);
        // Das Original entfernt je Abschluss genau einen Schritt; bei verkleinertem MaxUndos bleibt der
        // Stapel so lange länger, bis weitere Schritte folgen (A). Wir kürzen sofort auf die Grenze.
        while self.undo.len() > self.max_undos {
            self.undo.remove(0);
            self.dropped += 1;
        }
    }

    /// Letzter Schritt zum Zusammenfassen mit einem Folgeschritt (z. B. Pfeiltasten); nur, wenn
    /// nichts zum Wiederholen ansteht.
    pub fn last_mergeable(&mut self) -> Option<&mut HistoryStep> {
        if self.redo.is_empty() { self.undo.last_mut() } else { None }
    }

    /// Nimmt den letzten Schritt zurück und liefert den Zustand davor.
    pub fn undo(&mut self) -> Option<Snapshot> {
        let step = self.undo.pop()?;
        let snap = step.before.clone();
        self.redo.push(step);
        Some(snap)
    }

    /// Wiederholt den zuletzt zurückgenommenen Schritt und liefert den Zustand danach.
    pub fn redo(&mut self) -> Option<Snapshot> {
        let step = self.redo.pop()?;
        let snap = step.after.clone();
        self.undo.push(step);
        Some(snap)
    }

    /// Anzahl rückgängig machbarer Schritte (`fw.historyPalette.getStepCount`).
    pub fn undo_count(&self) -> usize {
        self.undo.len()
    }

    /// Anzahl der Schritte im Verlauf-Bedienfeld vor der aktuellen Position (= [`Self::undo_count`]).
    pub fn len(&self) -> usize {
        self.undo.len()
    }

    /// Kein rückgängig machbarer Schritt?
    pub fn is_empty(&self) -> bool {
        self.undo.is_empty()
    }

    /// Anzahl wiederholbarer Schritte.
    pub fn redo_count(&self) -> usize {
        self.redo.len()
    }

    /// Kann rückgängig gemacht werden?
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Kann wiederholt werden?
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Beschriftung des nächsten Rückgängig-Schritts (Menütext „Rückgängig …“).
    pub fn undo_label(&self) -> Option<&str> {
        self.undo.last().map(|s| s.label.as_str())
    }

    /// Beschriftung des nächsten Wiederholen-Schritts.
    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|s| s.label.as_str())
    }

    /// Zahl der wegen `MaxUndos` verworfenen Schritte.
    pub fn dropped(&self) -> usize {
        self.dropped
    }

    /// Alle Schritte wie im Verlauf-Bedienfeld: erst die ausgeführten (älteste zuerst), dann die
    /// zurückgenommenen (nächster Redo zuerst). Der Index [`History::undo_count`] markiert die
    /// aktuelle Position.
    pub fn steps(&self) -> Vec<&HistoryStep> {
        self.undo.iter().chain(self.redo.iter().rev()).collect()
    }

    /// Beschriftungen aller Schritte (siehe [`History::steps`]).
    pub fn labels(&self) -> Vec<String> {
        self.steps().iter().map(|s| s.label.clone()).collect()
    }

    /// Springt zu einer Position (Klick im Verlauf-Bedienfeld): `position` = Zahl der danach
    /// ausgeführten Schritte. Liefert den Zielzustand oder `None`, wenn sich nichts ändert.
    pub fn go_to(&mut self, position: usize) -> Option<Snapshot> {
        let mut snap = None;
        while self.undo.len() > position {
            snap = self.undo();
        }
        while self.undo.len() < position && !self.redo.is_empty() {
            snap = self.redo();
        }
        snap
    }

    /// Aufgezeichnete Befehle der ausgeführten Schritte `range` (Indizes wie [`History::steps`],
    /// nur ausgeführte Schritte). Für „Schritte wiederholen“, „Als Befehl speichern“ und MCP.
    pub fn export(&self, range: std::ops::Range<usize>) -> Vec<RecordedCommand> {
        self.undo
            .iter()
            .enumerate()
            .filter(|(i, s)| range.contains(i) && s.recordable)
            .map(|(_, s)| RecordedCommand { command: s.command.clone(), params: s.params.clone() })
            .collect()
    }

    /// Alle ausgeführten Schritte als Befehlsliste.
    pub fn export_all(&self) -> Vec<RecordedCommand> {
        self.export(0..self.undo.len())
    }

    /// Leert den Verlauf (z. B. nach „Wiederherstellen“).
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    /// Kürzt auf die letzten `n` Schritte (Speicherknappheit, `TrimHistorySteps`).
    pub fn trim_to(&mut self, n: usize) {
        while self.undo.len() > n {
            self.undo.remove(0);
            self.dropped += 1;
        }
        self.redo.clear();
    }
}

/// Erzeugt ein JS-artiges Skript aus aufgezeichneten Befehlen (für „Schritte kopieren“): je Zeile
/// `fp.run("<befehl>", <json>);`. fp-script bildet `fp.run` auf das Befehlsregister ab.
pub fn to_script(cmds: &[RecordedCommand]) -> String {
    let mut s = String::new();
    for c in cmds {
        let p = if c.params.is_null() { "{}".to_string() } else { c.params.to_string() };
        s.push_str(&format!("fp.run({}, {});\n", serde_json::Value::String(c.command.clone()), p));
    }
    s
}
