//! Antworten für `prompt()` in Befehlsskripten (CT-1b MN-5, FIX-B).
//!
//! Skripte laufen synchron und können nicht auf eine Eingabe warten. Startet die Oberfläche ein
//! Skript (`commands.runScript`/`commands.run` mit `prompts`), gelten diese Antworten der Reihe
//! nach; fragt das Skript öfter, bricht der Lauf ab (das Dokument bleibt unverändert), und der
//! Befehl meldet die offene Frage (`promptNeeded`). Die Oberfläche zeigt sie und startet das Skript
//! mit einer Antwort mehr erneut. Ohne `prompts` (CLI, MCP, Batch, Tests) liefert `prompt()` den
//! Vorgabewert wie bisher.

use std::cell::RefCell;

/// Antwort auf eine Frage des Skripts.
#[derive(Clone, Debug, PartialEq)]
pub enum Answer {
    /// Kein Lauf aus der Oberfläche: Vorgabewert verwenden.
    Headless,
    /// Antwort der Oberfläche (`None` = abgebrochen → `null`).
    Value(Option<String>),
    /// Noch keine Antwort: Lauf abbrechen, die Oberfläche fragt.
    Pending,
}

#[derive(Default)]
struct State {
    answers: Vec<Option<String>>,
    next: usize,
    pending: Option<(String, Option<String>)>,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

/// Beginnt einen Lauf mit Antworten der Oberfläche. Liefert `false`, wenn schon ein Lauf aktiv ist
/// (verschachteltes Skript: es teilt die Antworten des äußeren).
pub(crate) fn begin(answers: Vec<Option<String>>) -> bool {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        if s.is_some() {
            return false;
        }
        *s = Some(State {
            answers,
            ..State::default()
        });
        true
    })
}

/// Beendet den Lauf; liefert die offene Frage (Text, Vorgabe), falls das Skript deshalb abbrach.
pub(crate) fn end() -> Option<(String, Option<String>)> {
    STATE.with(|s| s.borrow_mut().take().and_then(|st| st.pending))
}

/// Nächste Antwort für `prompt(text, vorgabe)` (aufgerufen von der Skript-Laufzeit).
pub fn next(text: &str, default: Option<&str>) -> Answer {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        let Some(st) = s.as_mut() else {
            return Answer::Headless;
        };
        if let Some(a) = st.answers.get(st.next) {
            st.next += 1;
            return Answer::Value(a.clone());
        }
        if st.pending.is_none() {
            st.pending = Some((text.to_string(), default.map(str::to_string)));
        }
        Answer::Pending
    })
}

/// Offene Frage des laufenden Skripts (die Laufzeit bricht danach ab)?
pub fn is_pending() -> bool {
    STATE.with(|s| s.borrow().as_ref().is_some_and(|st| st.pending.is_some()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_in_order_then_pending() {
        assert_eq!(next("a", Some("1")), Answer::Headless);
        assert!(begin(vec![Some("5".into()), None]));
        assert!(!begin(vec![]));
        assert_eq!(next("a", Some("1")), Answer::Value(Some("5".into())));
        assert_eq!(next("b", None), Answer::Value(None));
        assert_eq!(next("c", Some("x")), Answer::Pending);
        assert!(is_pending());
        assert_eq!(end(), Some(("c".into(), Some("x".into()))));
        assert_eq!(next("a", Some("1")), Answer::Headless);
    }
}
