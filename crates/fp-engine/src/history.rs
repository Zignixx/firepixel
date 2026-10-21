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
    /// Toolbox stroke and fill colours, set only for steps that changed them (Swap Colours,
    /// TW-186: Ctrl+Z restores the objects and the toolbox colours). `None` = leave them.
    pub tool_colors: Option<(fp_color::Argb, fp_color::Argb)>,
}

/// Befehle, nach deren Rückgängigmachen keine Objekte gewählt sind (im Original gemessen: Einfügen).
pub const UNDO_CLEARS_SELECTION: &[&str] = &["edit.paste"];

/// Ein aufgezeichneter Befehlsaufruf (für Aktionen, MCP und „Befehl speichern“).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecordedCommand {
    /// Befehlsname.
    pub command: String,
    /// Parameter (JSON-Objekt).
    #[serde(default)]
    pub params: serde_json::Value,
}

/// Language-independent source of a step name (K-2): the name is translated when shown, so a
/// step created in one language reads correctly in another (panel, MCP `history`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StepKey {
    /// Step name of this command ([`crate::command::CommandInfo::step_label`]).
    Command(String),
    /// Text key (`msg-step-…`, `tool-…`, `cmd-…`) without arguments.
    Text(String),
}

/// Ein Verlaufsschritt.
#[derive(Clone, Debug)]
pub struct HistoryStep {
    /// Beschriftung im Verlauf-Bedienfeld in der Sprache der Oberfläche beim Anlegen (Rückfall,
    /// wenn `key` fehlt: eigene Namen, ältere Schritte).
    pub label: String,
    /// Sprachneutrale Quelle des Namens; `None` = `label` unverändert anzeigen.
    pub key: Option<StepKey>,
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
    /// The last change of the position was a redo (TW-173, re 55 §7.6): Edit ▸ Redo then has
    /// nothing to repeat ("Can't Redo") until the next new step or an undo.
    after_redo: bool,
}

impl History {
    /// Leerer Verlauf mit Höchstzahl `max_undos` (Voreinstellung `MaxUndos`, Vorgabe 20).
    pub fn new(max_undos: usize) -> History {
        History {
            undo: Vec::new(),
            redo: Vec::new(),
            max_undos,
            dropped: 0,
            after_redo: false,
        }
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
        self.after_redo = false;
        self.undo.push(step);
        // Das Original entfernt je Abschluss genau einen Schritt; bei verkleinertem MaxUndos bleibt der
        // Stapel so lange länger, bis weitere Schritte folgen (A). Wir kürzen sofort auf die Grenze.
        while self.undo.len() > self.max_undos {
            self.undo.remove(0);
            self.dropped += 1;
        }
    }

    /// Letzter Schritt zum Zusammenfassen mit einem Folgeschritt (z. B. Pfeiltasten); nur, wenn
    /// nichts zum Wiederholen ansteht (and not right after a redo: a redone row stays as it was).
    pub fn last_mergeable(&mut self) -> Option<&mut HistoryStep> {
        if self.redo.is_empty() && !self.after_redo {
            self.undo.last_mut()
        } else {
            None
        }
    }

    /// Nimmt den letzten Schritt zurück und liefert den Zustand davor.
    pub fn undo(&mut self) -> Option<Snapshot> {
        let step = self.undo.pop()?;
        self.after_redo = false;
        let mut snap = step.before.clone();
        // Einfügen merkt sich im Original keine Vorher-Auswahl (Auswahl-Schritt ohne Slots 7/8,
        // re\10 §Undo-Klassen „clipPaste“): Rückgängig hebt die Objektauswahl auf, statt die Auswahl
        // vor dem Einfügen herzustellen (PM-1: R1, Strg+C, Strg+V, Strg+V, 2× Strg+Z → nichts gewählt).
        if UNDO_CLEARS_SELECTION.contains(&step.command.as_str()) {
            snap.selection.elements.clear();
            snap.selection.points.clear();
        }
        self.redo.push(step);
        Some(snap)
    }

    /// Wiederholt den zuletzt zurückgenommenen Schritt und liefert den Zustand danach.
    pub fn redo(&mut self) -> Option<Snapshot> {
        let step = self.redo.pop()?;
        self.after_redo = true;
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

    /// Was the last change of the position a redo (no new step and no undo since)? Edit ▸ Redo
    /// then does not repeat the last step (TW-173, re 55 §7.6).
    pub fn after_redo(&self) -> bool {
        self.after_redo
    }

    /// The last executed step (the one Edit ▸ Undo takes back).
    pub fn top(&self) -> Option<&HistoryStep> {
        self.undo.last()
    }

    /// The last executed step, mutable (renaming a repeated row, TW-173).
    pub(crate) fn top_mut(&mut self) -> Option<&mut HistoryStep> {
        self.undo.last_mut()
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
        let mut snap: Option<Snapshot> = None;
        // toolbox colours of the last crossed step that changed them (TW-186)
        let mut colors = None;
        while self.undo.len() > position {
            snap = self.undo();
            colors = snap.as_ref().and_then(|s| s.tool_colors).or(colors);
        }
        while self.undo.len() < position && !self.redo.is_empty() {
            snap = self.redo();
            colors = snap.as_ref().and_then(|s| s.tool_colors).or(colors);
        }
        snap.map(|mut s| {
            s.tool_colors = colors;
            s
        })
    }

    /// Aufgezeichnete Befehle der ausgeführten Schritte `range` (Indizes wie [`History::steps`],
    /// nur ausgeführte Schritte). Für „Schritte wiederholen“, „Als Befehl speichern“ und MCP.
    pub fn export(&self, range: std::ops::Range<usize>) -> Vec<RecordedCommand> {
        self.undo
            .iter()
            .enumerate()
            .filter(|(i, s)| range.contains(i) && s.recordable)
            .map(|(_, s)| RecordedCommand {
                command: s.command.clone(),
                params: s.params.clone(),
            })
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
        self.after_redo = false;
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
        let p = if c.params.is_null() {
            "{}".to_string()
        } else {
            c.params.to_string()
        };
        s.push_str(&format!(
            "fp.run({}, {});\n",
            serde_json::Value::String(c.command.clone()),
            p
        ));
    }
    s
}
