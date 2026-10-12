//! Text ▸ Rechtschreibprüfung einrichten… (D24, Vorlage 6516, `re\11-bedienfelder-dialoge.md`):
//! Wörterbuchliste mit Kästchen, Pfad zum persönlichen Wörterbuch mit Ordnerknopf,
//! „Persönliches Wörterbuch bearbeiten“ (Unterdialog 6519) und vier Optionen (doppelte Wörter,
//! Wörter mit Zahlen, Internet-/Dateiadressen, GROSSBUCHSTABEN); Startfokus auf OK, Esc bricht ab.
//! Firepixel liefert keine Wörterbücher mit; darum gibt es zusätzlich das Feld „Wörterbuchordner“
//! (Ordner mit `<sprache>.aff`/`.dic`). OK schreibt alles über `text.spellingSetup` in die
//! Voreinstellungen.

use super::{button_row, dialog_frame, fit_label, push_button, t, DialogClose};
use crate::app_state::AppState;
use crate::file_dialog::{self, Request};
use crate::inspector::fields;
use crate::theme::Theme;
use egui::{vec2, Id, Pos2, Rect, Stroke, StrokeKind};
use serde_json::{json, Value};

/// Zustand des Dialogs.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpellingSetupDialog {
    /// Wörterbücher (Sprachkürzel) und ob sie aktiv sind.
    pub dictionaries: Vec<(String, bool)>,
    pub dictionary_dir: String,
    pub personal_path: String,
    /// Persönliches Wörterbuch (beim Öffnen und bearbeitet).
    pub words: Vec<String>,
    pub initial_words: Vec<String>,
    pub find_duplicates: bool,
    pub ignore_numbers: bool,
    pub ignore_urls: bool,
    pub ignore_uppercase: bool,
    /// Offener Unterdialog „Persönliches Wörterbuch bearbeiten“ (Text, ein Wort je Zeile).
    pub editing: Option<String>,
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default()
}

/// Anzeigename eines Wörterbuchs (`de_DE` → „Deutsch (Deutschland)“), sonst das Kürzel.
pub fn dictionary_name(code: &str) -> String {
    let key = format!("dlg-spell-lang-{}", code.to_lowercase().replace('_', "-"));
    if fp_i18n::has(&key) { t(&key) } else { code.to_string() }
}

impl SpellingSetupDialog {
    /// Aus der Antwort von `text.spellingSetup`.
    pub fn from_value(v: &Value) -> SpellingSetupDialog {
        let active = strings(&v["languages"]);
        let mut dictionaries: Vec<(String, bool)> = strings(&v["available"]).into_iter().map(|c| { let on = active.contains(&c); (c, on) }).collect();
        for a in &active {
            if !dictionaries.iter().any(|(c, _)| c == a) {
                dictionaries.push((a.clone(), true));
            }
        }
        let b = |k: &str| v[k].as_bool().unwrap_or(true);
        let words = strings(&v["userWords"]);
        SpellingSetupDialog {
            dictionaries,
            dictionary_dir: v["dictionaryDir"].as_str().unwrap_or("").to_string(),
            personal_path: v["personalDictionary"].as_str().unwrap_or("").to_string(),
            initial_words: words.clone(),
            words,
            find_duplicates: b("findDuplicates"),
            ignore_numbers: b("ignoreNumbers"),
            ignore_urls: b("ignoreUrls"),
            ignore_uppercase: b("ignoreUppercase"),
            editing: None,
        }
    }

    /// Liest die Einstellungen der Sitzung.
    pub fn new(st: &mut AppState) -> Option<SpellingSetupDialog> {
        let v = crate::ui_commands::execute(&mut st.session, "text.spellingSetup", json!({})).ok()?.value;
        Some(SpellingSetupDialog::from_value(&v))
    }

    /// Parameter für `text.spellingSetup` (OK).
    pub fn params(&self) -> Value {
        let add: Vec<&String> = self.words.iter().filter(|w| !self.initial_words.contains(w)).collect();
        let rem: Vec<&String> = self.initial_words.iter().filter(|w| !self.words.contains(w)).collect();
        let langs: Vec<&String> = self.dictionaries.iter().filter(|(_, on)| *on).map(|(c, _)| c).collect();
        json!({
            "languages": langs,
            "dictionaryDir": self.dictionary_dir,
            "personalDictionary": self.personal_path,
            "findDuplicates": self.find_duplicates,
            "ignoreNumbers": self.ignore_numbers,
            "ignoreUrls": self.ignore_urls,
            "ignoreUppercase": self.ignore_uppercase,
            "addWords": add,
            "removeWords": rem,
        })
    }

    /// Unterdialog übernehmen: ein Wort je Zeile, leere Zeilen und Doppelte fallen weg.
    pub fn commit_edit(&mut self, text: &str) {
        let mut w: Vec<String> = Vec::new();
        for l in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
            if !w.iter().any(|x| x == l) {
                w.push(l.to_string());
            }
        }
        self.words = w;
    }
}

/// Öffnet den Dialog.
pub fn open(st: &mut AppState) -> bool {
    match SpellingSetupDialog::new(st) {
        Some(d) => {
            st.dialogs.spelling_setup = Some(d);
            true
        }
        None => false,
    }
}

fn rr(o: Pos2, x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_min_size(o + vec2(x, y), vec2(w, h))
}

fn edit(ui: &mut egui::Ui, id: &str, r: Rect, value: &mut String, multiline: bool) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    ui.painter().rect_filled(r, 0.0, c.field_bg);
    ui.painter().rect_stroke(r, 0.0, Stroke::new(1.0, c.field_border), StrokeKind::Inside);
    let te = if multiline { egui::TextEdit::multiline(value) } else { egui::TextEdit::singleline(value) };
    let resp = ui.put(r.shrink(2.0), te.id(Id::new(("fp-spell-setup", id))).frame(egui::Frame::NONE).font(theme.ui_font()).desired_width(r.width() - 4.0));
    super::select_on_tab(ui, &resp, value.chars().count());
}

enum Pick {
    Personal,
    Folder,
}

/// Zeigt den Dialog (und ggf. den Unterdialog).
pub fn show(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.dialogs.spelling_setup.clone() else { return };
    let (ok_l, cancel_l) = (t("btn-ok"), t("btn-cancel"));
    let labs = super::tl(&["dlg-spell-setup-dictionaries", "dlg-spell-setup-options"]);
    let opts = super::tl(&["dlg-spell-setup-find-duplicates", "dlg-spell-setup-ignore-numbers", "dlg-spell-setup-ignore-urls", "dlg-spell-setup-ignore-upper"]);
    let edit_l = t("dlg-spell-setup-edit-personal");
    let lx = (super::max_text_w(ctx, &super::refs(&labs)) + 10.0).max(76.0).ceil();
    let width = 339f32.max(lx + 14.0 + super::max_text_w(ctx, &super::refs(&opts)) + 40.0).max(super::button_w(ctx, &edit_l, 210.0) + 40.0).ceil();
    let mut outcome: Option<bool> = None;
    let mut pick = None;
    let sub_open = d.editing.is_some();
    let r = dialog_frame(ctx, "fp-spelling-setup", &t("dlg-spell-setup-title"), vec2(width, 384.0), |ui, body| {
        super::autofocus_default_button(ui);
        let o = body.min;
        let lw = body.width() - lx - 14.0;
        fit_label(ui, rr(o, 14.0, 10.0, lx - 4.0, 16.0), &labs[0], true);
        let list = rr(o, lx, 8.0, lw, 112.0);
        let inner = fields::list_box(ui, list);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
        egui::ScrollArea::vertical().id_salt("fp-spell-setup-list").show(&mut child, |ui| {
            ui.set_min_width(inner.width());
            if d.dictionaries.is_empty() {
                let theme = Theme::current(ui.ctx());
                ui.label(egui::RichText::new(t("dlg-spell-setup-none-found")).font(theme.small_font()).color(theme.colors.text_disabled));
            }
            for (k, (code, on)) in d.dictionaries.iter_mut().enumerate() {
                let (row, _) = ui.allocate_exact_size(vec2(inner.width() - 4.0, 17.0), egui::Sense::hover());
                if let Some(v) = fields::checkbox(ui, ("fp-spell-dict", k), row.shrink2(vec2(3.0, 0.0)), &dictionary_name(code), *on, !sub_open) {
                    *on = v;
                }
            }
        });
        fit_label(ui, rr(o, 14.0, 128.0, body.width() - 28.0, 16.0), &t("dlg-spell-setup-folder"), true);
        edit(ui, "dir", rr(o, 14.0, 146.0, body.width() - 28.0 - 30.0, 22.0), &mut d.dictionary_dir, false);
        if push_button(ui, rr(o, body.width() - 14.0 - 26.0, 146.0, 26.0, 22.0), "…", false, !sub_open) {
            pick = Some(Pick::Folder);
        }
        fit_label(ui, rr(o, 14.0, 176.0, body.width() - 28.0, 16.0), &t("dlg-spell-setup-personal-path"), true);
        edit(ui, "personal", rr(o, 14.0, 194.0, body.width() - 28.0 - 30.0, 22.0), &mut d.personal_path, false);
        if push_button(ui, rr(o, body.width() - 14.0 - 26.0, 194.0, 26.0, 22.0), "…", false, !sub_open) {
            pick = Some(Pick::Personal);
        }
        let ew = super::button_w(ui.ctx(), &edit_l, 210.0);
        if push_button(ui, rr(o, (body.width() - ew) / 2.0, 226.0, ew, 23.0), &edit_l, false, !sub_open) {
            d.editing = Some(d.words.join("\n"));
        }
        fit_label(ui, rr(o, 14.0, 262.0, lx - 4.0, 16.0), &labs[1], true);
        let mut flags = [&mut d.find_duplicates, &mut d.ignore_numbers, &mut d.ignore_urls, &mut d.ignore_uppercase];
        for (k, f) in flags.iter_mut().enumerate() {
            if let Some(v) = fields::checkbox(ui, ("fp-spell-opt", k), rr(o, lx, 262.0 + k as f32 * 17.0, body.width() - lx - 14.0, 16.0), &opts[k], **f, !sub_open) {
                **f = v;
            }
        }
        let b = button_row(ui.ctx(), body.right() - 12.0, body.bottom() - 32.0, &[&ok_l, &cancel_l], 75.0, 23.0, 8.0);
        if push_button(ui, b[0], &ok_l, true, !sub_open) {
            outcome = Some(true);
        }
        if push_button(ui, b[1], &cancel_l, false, !sub_open) {
            outcome = Some(false);
        }
    });
    // Unterdialog „Persönliches Wörterbuch“ (6519)
    if let Some(mut text) = d.editing.clone() {
        let mut answer = None;
        let cr = dialog_frame(ctx, "fp-spelling-personal", &t("dlg-spell-personal-title"), vec2(320.0, 300.0), |ui, body| {
            let o = body.min;
            fit_label(ui, rr(o, 12.0, 10.0, body.width() - 24.0, 16.0), &t("dlg-spell-personal-hint"), true);
            edit(ui, "words", rr(o, 12.0, 30.0, body.width() - 24.0, body.height() - 80.0), &mut text, true);
            let b = button_row(ui.ctx(), body.right() - 12.0, body.bottom() - 34.0, &[&ok_l, &cancel_l], 75.0, 23.0, 8.0);
            if push_button(ui, b[0], &ok_l, true, true) {
                answer = Some(true);
            }
            if push_button(ui, b[1], &cancel_l, false, true) {
                answer = Some(false);
            }
        });
        match answer.or((cr == DialogClose::Cancelled).then_some(false)) {
            Some(true) => {
                d.commit_edit(&text);
                d.editing = None;
            }
            Some(false) => d.editing = None,
            None => d.editing = Some(text),
        }
        st.dialogs.spelling_setup = Some(d);
        return;
    }
    if let Some(p) = pick {
        st.dialogs.spelling_setup = Some(d);
        match p {
            Pick::Folder => file_dialog::request(st, Request::folder(t("dlg-spell-setup-folder")), |st, paths| {
                if let (Some(d), Some(p)) = (st.dialogs.spelling_setup.as_mut(), paths.first()) {
                    d.dictionary_dir = p.to_string_lossy().to_string();
                }
            }),
            Pick::Personal => file_dialog::request(st, Request::save(t("dlg-spell-setup-personal-path"), "Personal.dic").filter(t("dlg-spell-setup-filter-dic"), &["dic", "txt"]), |st, paths| {
                if let (Some(d), Some(p)) = (st.dialogs.spelling_setup.as_mut(), paths.first()) {
                    d.personal_path = p.to_string_lossy().to_string();
                }
            }),
        }
        return;
    }
    let outcome = outcome.or((r == DialogClose::Cancelled).then_some(false));
    match outcome {
        Some(true) => {
            st.dialogs.spelling_setup = None;
            st.run("text.spellingSetup", d.params());
        }
        Some(false) => st.dialogs.spelling_setup = None,
        None => st.dialogs.spelling_setup = Some(d),
    }
}
