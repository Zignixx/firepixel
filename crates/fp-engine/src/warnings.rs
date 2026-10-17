//! Warnungen bei immer aktiven Menübefehlen ohne passendes Ziel.
//!
//! Das Original lässt einige Befehle im Menü immer aktiv und meldet erst beim Aufruf, dass das
//! Ziel fehlt (gemessen in der Lage „Dokument ohne Auswahl“, `docs\BEFUNDE.md` EB, Z01–Z18):
//! Meldungsfenster mit Programmname als Titel, Warnsymbol, Text, darunter links das Kästchen
//! „Nicht mehr anzeigen“ und rechts „OK“; kein Verlaufsschritt. Angehakte Kästchen stehen in den
//! Voreinstellungen (`DisableNotice…`) und werden von „Warnmeldungen zurücksetzen“ gelöscht
//! ([`crate::Preferences::reset_warnings`]). „Popupmenü hinzufügen“ fragt stattdessen, ob ein
//! Hotspot oder ein Segment entstehen soll.
//!
//! Die Oberfläche fragt vor dem Ausführen (und vor einem Befehlsdialog) [`Session::target_warning`]
//! ab; [`Session::execute`] meldet dieselbe Warnung (außer abgeschaltete) als [`crate::SessionEvent::Warning`]
//! und bricht mit [`crate::EngineError::Selection`] (Wortlaut der Warnung) ab – Skripte und Agenten
//! sehen damit wie bisher einen Auswahlfehler.

use crate::commands::selected_elements;
use crate::prefs::PrefValue;
use crate::session::{DocumentState, Session};
use fp_model::ElementKind;
use serde::{Deserialize, Serialize};

/// Auswahlknopf einer Rückfrage.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WarningChoice {
    /// Beschriftung mit Zugriffstaste (`&`), aktuelle Sprache.
    pub label: String,
    /// Befehl, den die Wahl ausführt.
    pub command: String,
}

/// Warnung zu einem Befehl ohne passendes Ziel.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetWarning {
    /// Schlüssel der Warnung (mehrere Befehle teilen sich einen, z. B. `combinePaths`).
    pub key: String,
    /// Auslösender Befehl.
    pub command: String,
    /// Text in der aktuellen Sprache.
    pub text: String,
    /// Voreinstellung des Kästchens „Nicht mehr anzeigen“; `None` = ohne Kästchen (Rückfrage).
    pub pref: Option<String>,
    /// Rückfrage: Auswahlknöpfe (danach immer „Abbrechen“); leer = Meldung mit „OK“.
    pub choices: Vec<WarningChoice>,
    /// Früher mit „Nicht mehr anzeigen“ abgeschaltet: die Oberfläche zeigt nichts, der Befehl
    /// bleibt trotzdem ohne Wirkung.
    pub suppressed: bool,
}

struct Entry {
    key: &'static str,
    commands: &'static [&'static str],
    text: &'static str,
    pref: Option<&'static str>,
    choices: &'static [(&'static str, &'static str)],
    missing: fn(&DocumentState) -> bool,
}

fn count_kind(d: &DocumentState, f: fn(&ElementKind) -> bool) -> usize {
    selected_elements(d)
        .iter()
        .filter(|(_, e)| f(&e.kind))
        .count()
}

fn is_path(k: &ElementKind) -> bool {
    matches!(k, ElementKind::Path(_))
}

fn is_instance(k: &ElementKind) -> bool {
    matches!(k, ElementKind::Instance(_))
}

fn is_web(k: &ElementKind) -> bool {
    matches!(k, ElementKind::WebObject(_))
}

/// Weniger als zwei Instanzen desselben Bibliothekssymbols ausgewählt.
fn no_tween_pair(d: &DocumentState) -> bool {
    let mut per_symbol: Vec<(fp_model::SymbolId, usize)> = Vec::new();
    for (_, e) in selected_elements(d) {
        if let ElementKind::Instance(i) = &e.kind {
            match per_symbol.iter_mut().find(|(s, _)| *s == i.symbol_id) {
                Some((_, n)) => *n += 1,
                None => per_symbol.push((i.symbol_id, 1)),
            }
        }
    }
    per_symbol.iter().all(|(_, n)| *n < 2)
}

/// Die Warnungen des Originals (Wortlaut Z01–Z18).
const ENTRIES: &[Entry] = &[
    Entry {
        key: "combinePaths",
        commands: &[
            "modify.combinePaths.union",
            "modify.combinePaths.intersect",
            "modify.combinePaths.punch",
            "modify.combinePaths.crop",
        ],
        text: "msg-warn-combine-paths",
        pref: Some("DisableNoticeCombinePaths"),
        choices: &[],
        missing: |d| count_kind(d, is_path) < 2,
    },
    Entry {
        key: "alterPath",
        commands: &[
            "modify.alterPath.simplify",
            "modify.alterPath.expandStroke",
            "modify.alterPath.insetPath",
        ],
        text: "msg-warn-alter-path",
        pref: Some("DisableNoticeAlterPath"),
        choices: &[],
        missing: |d| count_kind(d, is_path) == 0,
    },
    Entry {
        key: "tweenInstances",
        commands: &["modify.symbol.tweenInstances"],
        text: "msg-warn-tween-instances",
        pref: Some("DisableNoticeTweenInstances"),
        choices: &[],
        missing: no_tween_pair,
    },
    Entry {
        key: "breakApart",
        commands: &["modify.symbol.breakApart"],
        text: "msg-warn-break-apart",
        pref: Some("DisableNoticeBreakApart"),
        choices: &[],
        missing: |d| count_kind(d, is_instance) == 0,
    },
    Entry {
        key: "insertHotspot",
        commands: &["edit.insert.hotspot"],
        text: "msg-warn-insert-hotspot",
        pref: Some("DisableNoticeInsertHotspot"),
        choices: &[],
        missing: |d| selected_elements(d).is_empty(),
    },
    Entry {
        key: "insertSlice",
        commands: &["edit.insert.slice", "edit.insert.polygonSlice"],
        text: "msg-warn-insert-slice",
        pref: Some("DisableNoticeInsertSlice"),
        choices: &[],
        missing: |d| selected_elements(d).is_empty(),
    },
    Entry {
        key: "selectSimilar",
        commands: &["select.similar"],
        text: "msg-warn-select-similar",
        pref: Some("DisableNoticeSelectSimilar"),
        choices: &[],
        missing: |d| d.selection.is_empty(),
    },
    Entry {
        key: "feather",
        commands: &["select.feather"],
        text: "msg-warn-feather",
        pref: Some("DisableNoticeFeather"),
        choices: &[],
        missing: |d| !d.selection.has_pixels(),
    },
    Entry {
        key: "addPopUpMenu",
        commands: &["modify.popUpMenu.addPopUpMenu"],
        text: "msg-warn-popup-hotspot-or-slice",
        pref: None,
        choices: &[
            ("msg-warn-choice-hotspot", "edit.insert.hotspot"),
            ("msg-warn-choice-slice", "edit.insert.slice"),
        ],
        missing: |d| count_kind(d, is_web) == 0,
    },
];

/// Zweite Meldung nach „Abbrechen“/Esc einer Rückfrage: (Schlüssel der Rückfrage, Folgemeldung).
/// Original: „Popupmenü hinzufügen“ abgebrochen → „Wählen Sie ein Segment oder einen Hotspot aus,
/// bevor Sie den Befehl „Verhalten einfügen“ verwenden.“ mit „Nicht mehr anzeigen“ (PM-6 D34).
const AFTER_CANCEL: &[(&str, Entry)] = &[(
    "addPopUpMenu",
    Entry {
        key: "insertBehavior",
        commands: &[],
        text: "msg-warn-insert-behavior",
        pref: Some("DisableNoticeInsertBehavior"),
        choices: &[],
        missing: |_| true,
    },
)];

/// Alle Warnschlüssel mit ihren Befehlen (für Tests und Dokumentation).
pub fn all() -> Vec<(&'static str, &'static [&'static str])> {
    ENTRIES.iter().map(|e| (e.key, e.commands)).collect()
}

fn entry_for(name: &str) -> Option<&'static Entry> {
    ENTRIES.iter().find(|e| e.commands.contains(&name))
}

impl Session {
    /// Warnung, die das Original beim Aufruf von `name` zeigt, weil das Ziel fehlt (siehe
    /// Moduldoku); `None`, wenn der Befehl ein Ziel hat, keine solche Warnung kennt oder kein
    /// Dokument offen ist. Ist sie abgeschaltet, steht `suppressed` auf `true`.
    pub fn target_warning(&self, name: &str) -> Option<TargetWarning> {
        let e = entry_for(name)?;
        let d = self.active()?;
        if !(e.missing)(d) {
            return None;
        }
        Some(TargetWarning {
            key: e.key.to_string(),
            command: name.to_string(),
            text: fp_i18n::t(e.text),
            pref: e.pref.map(str::to_string),
            choices: e
                .choices
                .iter()
                .map(|(l, c)| WarningChoice {
                    label: fp_i18n::t(l),
                    command: (*c).to_string(),
                })
                .collect(),
            suppressed: e.pref.is_some_and(|p| self.prefs.get_bool(p, false)),
        })
    }

    /// Meldung, die das Original zeigt, nachdem die Rückfrage `warning` abgebrochen wurde (siehe
    /// `AFTER_CANCEL`); `None`, wenn es keine gibt. Abgeschaltet: `suppressed` = `true`.
    pub fn warning_after_cancel(&self, warning: &TargetWarning) -> Option<TargetWarning> {
        let (_, e) = AFTER_CANCEL.iter().find(|(k, _)| *k == warning.key)?;
        Some(TargetWarning {
            key: e.key.to_string(),
            command: warning.command.clone(),
            text: fp_i18n::t(e.text),
            pref: e.pref.map(str::to_string),
            choices: Vec::new(),
            suppressed: e.pref.is_some_and(|p| self.prefs.get_bool(p, false)),
        })
    }

    /// Kästchen „Nicht mehr anzeigen“ einer Warnung setzen (Voreinstellung `pref` der Warnung)
    /// und die Voreinstellungen speichern.
    pub fn set_warning_disabled(&mut self, warning: &TargetWarning, disabled: bool) {
        let Some(p) = &warning.pref else { return };
        if disabled {
            self.prefs.set(p, PrefValue::Bool(true));
        } else {
            self.prefs.reset(p);
        }
        self.prefs_changed();
    }
}
