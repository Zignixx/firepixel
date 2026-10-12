//! Rechtschreibprüfung (6515) und „Rechtschreibprüfung einrichten“ (6516) mit persönlichem
//! Wörterbuch (6519). Die Prüfung selbst (Hunspell-kompatible `.aff`/`.dic`, persönliche
//! Wortliste) macht fp-engine (`text.checkSpelling`, `text.spellingSetup`); der Dialog geht die
//! gemeldeten Wörter nacheinander durch: Ignorieren, Alle ignorieren, Ändern, Alle ändern,
//! Hinzufügen (persönliches Wörterbuch), Löschen (Wort entfernen).

use super::form::{Field, FormDialog};
use super::{dialog_frame, push_button, t, DialogClose};
use crate::app_state::{Action, AppState};
use crate::inspector::fields;
use crate::theme::Theme;
use egui::{pos2, vec2, Align2, Rect};
use serde_json::{json, Value};

/// Ein gemeldetes Wort.
#[derive(Clone, Debug, PartialEq)]
pub struct Issue {
    pub id: u64,
    pub word: String,
    pub suggestions: Vec<String>,
}

/// Zustand der Rechtschreibprüfung.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpellingDialog {
    pub issues: Vec<Issue>,
    /// Übersprungene Vorkommen (Text-ID, Wort, Nummer des Vorkommens).
    pub skipped: Vec<(u64, String, usize)>,
    /// „Alle ignorieren“ (nur in dieser Prüfung).
    pub ignored: Vec<String>,
    pub change_to: String,
    pub selected: Option<usize>,
    pub note: Option<String>,
    pub done: bool,
}

fn parse(v: &Value) -> Vec<Issue> {
    v.get("misspelled")
        .and_then(|m| m.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| {
                    Some(Issue {
                        id: x.get("id")?.as_u64()?,
                        word: x.get("word")?.as_str()?.to_string(),
                        suggestions: x.get("suggestions").and_then(|s| s.as_array()).map(|s| s.iter().filter_map(|w| w.as_str().map(|w| w.to_string())).collect()).unwrap_or_default(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

impl SpellingDialog {
    /// Aktuelles Wort (ohne übersprungene/ignorierte).
    pub fn current(&self) -> Option<&Issue> {
        let mut seen: Vec<(u64, &str)> = Vec::new();
        for i in &self.issues {
            let n = seen.iter().filter(|(id, w)| *id == i.id && *w == i.word).count();
            seen.push((i.id, &i.word));
            if self.ignored.contains(&i.word) || self.skipped.contains(&(i.id, i.word.clone(), n)) {
                continue;
            }
            return Some(i);
        }
        None
    }

    fn occurrence(&self, cur: &Issue) -> usize {
        self.skipped.iter().filter(|(id, w, _)| *id == cur.id && *w == cur.word).count()
    }

    fn reset_choice(&mut self) {
        self.change_to = self.current().and_then(|i| i.suggestions.first().cloned()).unwrap_or_else(|| self.current().map(|i| i.word.clone()).unwrap_or_default());
        self.selected = self.current().filter(|i| !i.suggestions.is_empty()).map(|_| 0);
    }
}

/// Prüfung neu laufen lassen (nach jeder Änderung).
fn recheck(st: &mut AppState, d: &mut SpellingDialog, params: Value) {
    match crate::ui_commands::execute(&mut st.session, "text.checkSpelling", params) {
        Ok(o) => {
            d.issues = parse(&o.value);
            d.note = o.value.get("note").and_then(|n| n.as_str()).map(|s| s.to_string());
        }
        Err(e) => {
            d.note = Some(e.to_string());
            d.issues.clear();
        }
    }
    d.reset_choice();
}

/// Startet die Rechtschreibprüfung (Auswahl bzw. alle Texte der Seite).
pub fn open_check(st: &mut AppState) {
    let mut d = SpellingDialog::default();
    recheck(st, &mut d, json!({}));
    if d.current().is_none() {
        let msg = match &d.note {
            Some(n) => fp_i18n::tr!("dlg-spell-no-errors-note", note = n.as_str()),
            None => t("dlg-spell-no-errors"),
        };
        st.dialogs.message = Some((t("dlg-spell-title"), msg));
        return;
    }
    st.dialogs.spelling = Some(d);
}

/// Zeigt die Rechtschreibprüfung.
pub fn show_check(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.dialogs.spelling.clone() else { return };
    let mut op: Option<&str> = None;
    // Texte der aktuellen Sprache; Beschriftungsspalte und Knöpfe wachsen mit (Deutsch: wie bisher)
    let labs = super::tl(&["dlg-spell-not-found", "dlg-spell-change-to", "dlg-spell-suggestions"]);
    let btn_l = super::tl(&["dlg-spell-ignore", "dlg-spell-ignore-all", "dlg-spell-change", "dlg-spell-change-all", "dlg-spell-add", "dlg-spell-delete", "btn-close"]);
    let dx = super::extra_w(ctx, &super::refs(&labs), 150.0 - 4.0);
    let bw = super::buttons_w(ctx, &super::refs(&btn_l), 108.0);
    let width = 470f32.max(12.0 + 150.0 + dx + 170.0 + 12.0 + bw + 12.0).ceil();
    let lx = 150.0 + dx;
    let r = dialog_frame(ctx, "fp-spelling", &t("dlg-spell-title"), vec2(width, 300.0), |ui, body| {
        let theme = Theme::current(ui.ctx());
        let o = body.min + vec2(12.0, 12.0);
        let cur = d.current().cloned();
        super::fit_label(ui, Rect::from_min_size(o, vec2(lx - 4.0, 18.0)), &labs[0], true);
        let wr = Rect::from_min_size(o + vec2(lx, 0.0), vec2(170.0, 20.0));
        fields::field_frame(ui, wr, false, false);
        ui.painter().text(pos2(wr.left() + 4.0, wr.center().y), Align2::LEFT_CENTER, cur.as_ref().map(|c| c.word.as_str()).unwrap_or(""), theme.bold(theme.fonts.ui), theme.colors.icon_red);
        super::fit_label(ui, Rect::from_min_size(o + vec2(0.0, 28.0), vec2(lx - 4.0, 18.0)), &labs[1], true);
        if let Some(t) = fields::text(ui, "spell-change", Rect::from_min_size(o + vec2(lx, 28.0), vec2(170.0, 20.0)), &d.change_to, true) {
            d.change_to = t;
        }
        super::fit_label(ui, Rect::from_min_size(o + vec2(0.0, 56.0), vec2(lx - 4.0, 18.0)), &labs[2], true);
        let list = Rect::from_min_size(o + vec2(lx, 56.0), vec2(170.0, 150.0));
        let inner = fields::list_box(ui, list);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
        child.spacing_mut().item_spacing = vec2(0.0, 0.0);
        if let Some(c) = &cur {
            for (i, s) in c.suggestions.iter().enumerate() {
                if fields::list_item(&mut child, s, d.selected == Some(i)).clicked() {
                    d.selected = Some(i);
                    d.change_to = s.clone();
                }
            }
            if c.suggestions.is_empty() {
                child.painter().text(inner.left_top() + vec2(4.0, 4.0), Align2::LEFT_TOP, t("dlg-spell-no-suggestions"), theme.small_font(), theme.colors.text_disabled);
            }
        }
        let bx = body.right() - 12.0 - bw;
        let has = cur.is_some();
        let buttons = [(&btn_l[0], "ignore"), (&btn_l[1], "ignoreAll"), (&btn_l[2], "change"), (&btn_l[3], "changeAll"), (&btn_l[4], "add"), (&btn_l[5], "delete")];
        for (k, (label, id)) in buttons.iter().enumerate() {
            if push_button(ui, Rect::from_min_size(pos2(bx, body.top() + 12.0 + k as f32 * 28.0), vec2(bw, 23.0)), label, k == 2, has) {
                op = Some(id);
            }
        }
        if let Some(n) = &d.note {
            super::put_left(ui, Rect::from_min_size(pos2(body.left() + 12.0, body.bottom() - 70.0), vec2(body.width() - 32.0 - bw, 32.0)), egui::Label::new(egui::RichText::new(n).font(theme.small_font()).color(theme.colors.text_weak)).wrap());
        }
        if push_button(ui, Rect::from_min_size(pos2(bx, body.bottom() - 34.0), vec2(bw, 23.0)), &btn_l[6], false, true) {
            op = Some("close");
        }
    });
    let cur = d.current().cloned();
    match (op, cur) {
        (Some("close"), _) => d.done = true,
        (Some("ignore"), Some(c)) => {
            let n = d.occurrence(&c);
            d.skipped.push((c.id, c.word.clone(), n));
            d.reset_choice();
        }
        (Some("ignoreAll"), Some(c)) => {
            d.ignored.push(c.word.clone());
            d.reset_choice();
        }
        (Some("change"), Some(c)) => {
            let p = json!({"replace": [{"word": c.word, "with": d.change_to, "id": c.id}]});
            recheck(st, &mut d, p)
        }
        (Some("changeAll"), Some(c)) => {
            let p = json!({"replace": [{"word": c.word, "with": d.change_to}]});
            recheck(st, &mut d, p)
        }
        (Some("delete"), Some(c)) => recheck(st, &mut d, json!({"replace": [{"word": c.word, "with": "", "id": c.id}]})),
        (Some("add"), Some(c)) => recheck(st, &mut d, json!({"addToDictionary": [c.word]})),
        _ => {}
    }
    if r == DialogClose::Cancelled {
        d.done = true;
    }
    if !d.done && op.is_some() && d.current().is_none() {
        st.dialogs.message = Some((t("dlg-spell-title"), t("dlg-spell-done")));
        d.done = true;
    }
    st.dialogs.spelling = if d.done { None } else { Some(d) };
}

fn setup_params(d: &FormDialog) -> Value {
    let old: Vec<String> = d.base.get("_words").and_then(|w| w.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect()).unwrap_or_default();
    let new: Vec<String> = d.get("_dict").as_str().unwrap_or("").lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect();
    let add: Vec<&String> = new.iter().filter(|w| !old.contains(w)).collect();
    let rem: Vec<&String> = old.iter().filter(|w| !new.contains(w)).collect();
    json!({
        "language": d.get("language"),
        "dictionaryDir": d.get("dictionaryDir"),
        "ignoreUppercase": d.get("ignoreUppercase"),
        "ignoreNumbers": d.get("ignoreNumbers"),
        "addWords": add,
        "removeWords": rem,
    })
}

/// „Rechtschreibprüfung einrichten“ (6516) mit persönlichem Wörterbuch (6519) als Formular.
pub fn setup_form(st: &mut AppState) -> Option<FormDialog> {
    let v = match crate::ui_commands::execute(&mut st.session, "text.spellingSetup", json!({})) {
        Ok(o) => o.value,
        Err(e) => {
            st.push(Action::Status(e.to_string()));
            return None;
        }
    };
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    let b = |k: &str, d: bool| v.get(k).and_then(|x| x.as_bool()).unwrap_or(d);
    let words: Vec<String> = v.get("userWords").or_else(|| v.get("personalWords")).or_else(|| v.get("words")).and_then(|w| w.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect()).unwrap_or_default();
    Some(
        FormDialog::new(
            "6516",
            &t("dlg-spell-setup-title"),
            "text.spellingSetup",
            vec![
                Field::text("language", &t("dlg-spell-setup-language"), &s("language")),
                Field::text("dictionaryDir", &t("dlg-spell-setup-folder"), &s("dictionaryDir")),
                Field::check("ignoreUppercase", &t("dlg-spell-setup-ignore-upper"), b("ignoreUppercase", true)),
                Field::check("ignoreNumbers", &t("dlg-spell-setup-ignore-numbers"), b("ignoreNumbers", true)),
                Field::multiline("_dict", &t("dlg-spell-setup-personal"), &words.join("\n")),
            ],
        )
        .with_base(json!({"_words": words}))
        .with_width(460.0)
        .with_transform(setup_params),
    )
}
