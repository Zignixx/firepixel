//! Tastaturkurzbefehle (6235): aktueller Satz (mitgelieferte und eigene Sätze), Satz duplizieren,
//! umbenennen, als HTML exportieren und löschen (6430), Befehlsbaum Menübefehle/Werkzeuge mit den
//! Kürzeln des Satzes, Suche, Kürzel hinzufügen (+), entfernen (−) und „Drücken Sie die Taste“ mit
//! Konfliktanzeige (6261: „Neu zuweisen“). Mitgelieferte Sätze sind schreibgeschützt – wie im
//! Original muss zuerst eine Kopie angelegt werden. Gearbeitet wird auf einer Kopie des Satzes;
//! erst OK speichert (fp-engine `shortcuts::save_custom_set`, Satzwahl über
//! `edit.keyboardShortcuts`), Abbrechen verwirft alles.

use super::{dialog_frame, push_button, DialogClose};
use crate::app_state::{Action, AppState};
use crate::inspector::fields;
use crate::theme::Theme;
use egui::{pos2, vec2, Align2, Rect, Sense, Stroke, StrokeKind};
use fp_engine::shortcuts::{Shortcut, ShortcutSet};
use serde_json::json;

/// Zeile der Liste: Kennung (`cmd:<cmd_-Name>` bzw. `tool:<GUID>`), Anzeigetext, Kürzel.
pub type Row = (String, String, Vec<Shortcut>);

/// Zustand des Dialogs.
#[derive(Clone, Debug, PartialEq)]
pub struct ShortcutsDialog {
    /// Arbeitskopie des aktuellen Satzes.
    pub work: ShortcutSet,
    /// Wurde die Arbeitskopie geändert?
    pub dirty: bool,
    /// Beim OK zu löschende eigene Sätze.
    pub delete: Vec<String>,
    pub section: usize,
    pub filter: String,
    /// Ausgewählte Zeile (Kennung).
    pub selected: Option<String>,
    /// Ausgewähltes Kürzel der Zeile.
    pub selected_key: Option<usize>,
    /// Erfasste Taste („Drücken Sie die Taste“).
    pub pressed: Option<Shortcut>,
    pub capturing: bool,
    /// Eingabe für Duplizieren/Umbenennen: (umbenennen?, Name).
    pub name_edit: Option<(bool, String)>,
    pub message: Option<String>,
}

impl ShortcutsDialog {
    /// Mit dem aktuellen Satz.
    pub fn new(st: &AppState) -> ShortcutsDialog {
        ShortcutsDialog::for_set(st, &st.session.prefs.shortcut_set())
    }

    fn for_set(st: &AppState, name: &str) -> ShortcutsDialog {
        let work = fp_engine::shortcuts::any_set(name).cloned().unwrap_or_else(|| st.session.shortcut_set().clone());
        ShortcutsDialog { work, dirty: false, delete: Vec::new(), section: 0, filter: String::new(), selected: None, selected_key: None, pressed: None, capturing: false, name_edit: None, message: None }
    }

    /// Ist die Arbeitskopie ein mitgelieferter Satz (schreibgeschützt)?
    pub fn read_only(&self) -> bool {
        fp_engine::shortcuts::builtin_set(&self.work.name).is_some()
    }
}

/// Namen aller Sätze (mitgeliefert, dann eigene).
pub fn set_names() -> Vec<String> {
    let mut v: Vec<String> = fp_engine::shortcuts::builtin_set_names().into_iter().map(|s| s.to_string()).collect();
    v.extend(fp_engine::shortcuts::custom_set_names());
    v
}

/// Zeilen eines Satzes.
pub fn rows(st: &AppState, set: &ShortcutSet, section: usize, filter: &str) -> Vec<Row> {
    let f = filter.to_lowercase();
    let mut out = Vec::new();
    if section == 0 {
        for c in st.session.registry().iter() {
            let info = c.info();
            if info.menu_path.is_empty() {
                continue;
            }
            let Some(cmd) = info.fw_cmd.clone() else { continue };
            let path = info.menu_path.join(" ▸ ");
            if !f.is_empty() && !path.to_lowercase().contains(&f) {
                continue;
            }
            out.push((format!("cmd:{cmd}"), path, set.command(&cmd).to_vec()));
        }
        out.sort_by(|a, b| a.1.cmp(&b.1));
    } else {
        for t in fp_engine::tools::TOOLS {
            let label = t.label.to_string();
            if !f.is_empty() && !label.to_lowercase().contains(&f) {
                continue;
            }
            out.push((format!("tool:{}", t.guid), label, set.tool(t.guid).to_vec()));
        }
    }
    out
}

fn keys_of<'a>(set: &'a ShortcutSet, id: &str) -> &'a [Shortcut] {
    match id.split_once(':') {
        Some(("cmd", c)) => set.command(c),
        Some(("tool", g)) => set.tool(g),
        _ => &[],
    }
}

fn set_keys(set: &mut ShortcutSet, id: &str, keys: Vec<Shortcut>) -> Result<(), String> {
    match id.split_once(':') {
        Some(("cmd", c)) => set.set_command(c, keys),
        Some(("tool", g)) => set.set_tool(g, keys),
        _ => Err("unbekannter Eintrag".into()),
    }
}

/// Wem ist das Kürzel im Satz schon zugeordnet (außer `except`)? Anzeigetext.
fn conflict(st: &AppState, set: &ShortcutSet, sc: &Shortcut, except: &str, tool: bool) -> Option<(String, String)> {
    if tool {
        let guid = set.tools_for(sc).into_iter().find(|g| format!("tool:{g}") != except)?;
        let label = fp_engine::tools::TOOLS.iter().find(|t| fp_engine::shortcuts::normalize_guid(t.guid) == guid).map(|t| t.label.to_string()).unwrap_or_else(|| guid.to_string());
        return Some((format!("tool:{guid}"), label));
    }
    let cmd = set.commands_for(sc).into_iter().find(|c| format!("cmd:{c}") != except)?;
    let label = st.session.registry().by_fw_cmd(cmd).map(|c| c.info().menu_path.join(" ▸ ")).unwrap_or_else(|| cmd.to_string());
    Some((format!("cmd:{cmd}"), label))
}

/// Erfasst eine gedrückte Taste mit Modifikatoren.
fn captured_key(ui: &egui::Ui) -> Option<Shortcut> {
    ui.input(|i| {
        i.events.iter().find_map(|e| match e {
            egui::Event::Key { key, pressed: true, modifiers, .. } => crate::input::vk_of(*key).map(|vk| Shortcut { ctrl: modifiers.command || modifiers.ctrl, alt: modifiers.alt, shift: modifiers.shift, vk }),
            _ => None,
        })
    })
}

/// HTML-Übersicht eines Satzes (Exportieren).
pub fn set_html(st: &AppState, set: &ShortcutSet) -> String {
    let mut s = format!("<!DOCTYPE html>\n<html><head><meta charset=\"utf-8\"><title>Tastaturkurzbefehle: {}</title></head><body>\n<h1>Tastaturkurzbefehle: {}</h1>\n", set.name, set.name);
    for (title, sec) in [("Menübefehle", 0), ("Werkzeuge", 1)] {
        s.push_str(&format!("<h2>{title}</h2>\n<table border=\"1\" cellpadding=\"3\">\n"));
        for (_, label, keys) in rows(st, set, sec, "") {
            if keys.is_empty() {
                continue;
            }
            let k: Vec<String> = keys.iter().map(|k| k.display_de()).collect();
            s.push_str(&format!("<tr><td>{}</td><td>{}</td></tr>\n", label.replace('&', "&amp;").replace('<', "&lt;"), k.join(", ")));
        }
        s.push_str("</table>\n");
    }
    s.push_str("</body></html>\n");
    s
}

/// Zeigt den Dialog.
pub fn show(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.dialogs.shortcuts.clone() else { return };
    let mut result = None;
    let mut switch_to: Option<String> = None;
    let r = dialog_frame(ctx, "fp-shortcuts", "Tastaturkurzbefehle", vec2(640.0, 560.0), |ui, body| {
        let theme = Theme::current(ui.ctx());
        let c = &theme.colors;
        let o = body.min + vec2(14.0, 14.0);
        fields::label(ui, Rect::from_min_size(o, vec2(110.0, 20.0)), "Aktueller Satz:", true);
        let mut names = set_names();
        names.retain(|n| !d.delete.iter().any(|x| x.eq_ignore_ascii_case(n)));
        if !names.iter().any(|n| n.eq_ignore_ascii_case(&d.work.name)) {
            names.push(d.work.name.clone());
        }
        let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
        let cur = names.iter().position(|n| n.eq_ignore_ascii_case(&d.work.name));
        if let Some(i) = fields::combo(ui, "sc-set", Rect::from_min_size(o + vec2(116.0, 0.0), vec2(190.0, 20.0)), &refs, cur, true) {
            switch_to = Some(names[i].clone());
        }
        let ro = d.read_only();
        let btns = [("Duplizieren", true), ("Umbenennen", !ro), ("Exportieren", true), ("Löschen", !ro)];
        for (k, (label, en)) in btns.iter().enumerate() {
            let br = Rect::from_min_size(o + vec2(316.0 + k as f32 * 72.0, 0.0), vec2(68.0, 20.0));
            if fields::button(ui, ("sc-btn", k), br, label, *en) {
                match k {
                    0 => d.name_edit = Some((false, format!("{} Kopie", d.work.name))),
                    1 => d.name_edit = Some((true, d.work.name.clone())),
                    2 => {
                        let html = set_html(st, &d.work);
                        if let Some(p) = super::panel_menus::pick_save(st, "Satz exportieren", &format!("{}.html", d.work.name), &[("HTML (*.html)", &["html", "htm"])]) {
                            d.message = Some(match std::fs::write(&p, html) {
                                Ok(()) => format!("Exportiert: {}", p.display()),
                                Err(e) => format!("Export fehlgeschlagen: {e}"),
                            });
                        }
                    }
                    _ => {
                        d.delete.push(d.work.name.clone());
                        switch_to = fp_engine::shortcuts::builtin_set_names().first().map(|s| s.to_string());
                    }
                }
            }
        }
        // Name für Duplizieren/Umbenennen
        if let Some((rename, name)) = d.name_edit.clone() {
            let nr = Rect::from_min_size(o + vec2(316.0, 24.0), vec2(212.0, 20.0));
            let mut n = name.clone();
            if let Some(t) = fields::text(ui, "sc-name", nr, &name, true) {
                n = t;
                d.name_edit = Some((rename, n.clone()));
            }
            if fields::button(ui, "sc-name-ok", Rect::from_min_size(nr.right_top() + vec2(4.0, 0.0), vec2(56.0, 20.0)), "OK", !n.trim().is_empty()) {
                let n = n.trim().to_string();
                if fp_engine::shortcuts::builtin_set(&n).is_some() {
                    d.message = Some(format!("„{n}“ ist der Name eines mitgelieferten Satzes."));
                } else {
                    if rename && !d.work.name.eq_ignore_ascii_case(&n) {
                        d.delete.push(d.work.name.clone());
                    }
                    d.work.name = n;
                    d.dirty = true;
                }
                d.name_edit = None;
            }
        }
        if let Some(i) = fields::combo(ui, "sc-section", Rect::from_min_size(o + vec2(0.0, 30.0), vec2(160.0, 20.0)), &["Menübefehle", "Werkzeuge"], Some(d.section), true) {
            d.section = i;
            d.selected = None;
        }
        if let Some(t) = fields::text(ui, "sc-filter", Rect::from_min_size(o + vec2(170.0, 30.0), vec2(130.0, 20.0)), &d.filter, true) {
            d.filter = t;
        }
        ui.painter().text(o + vec2(306.0, 40.0), Align2::LEFT_CENTER, "Suchen", theme.small_font(), c.text_weak);
        let list = Rect::from_min_max(o + vec2(0.0, 60.0), pos2(body.right() - 14.0, body.bottom() - 150.0));
        ui.painter().rect_filled(list, 0.0, c.list_bg);
        ui.painter().rect_stroke(list, 0.0, Stroke::new(1.0, c.list_border), StrokeKind::Inside);
        let rws = rows(st, &d.work, d.section, &d.filter);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(list.shrink(1.0)));
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(&mut child, |ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            for (id, path, keys) in &rws {
                let sel = d.selected.as_deref() == Some(id.as_str());
                let (rr, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 18.0), Sense::click());
                if sel {
                    ui.painter().rect_filled(rr, 0.0, c.list_selected_bg);
                } else if resp.hovered() {
                    ui.painter().rect_filled(rr, 0.0, c.list_hover_bg);
                }
                crate::widgets::text_clipped(ui.painter(), pos2(rr.left() + 6.0, rr.center().y), Align2::LEFT_CENTER, path, theme.ui_font(), c.text, rr.width() * 0.68);
                let k: Vec<String> = keys.iter().map(|k| k.display_de()).collect();
                ui.painter().text(pos2(rr.left() + rr.width() * 0.7, rr.center().y), Align2::LEFT_CENTER, k.join(", "), theme.ui_font(), c.text_weak);
                if resp.clicked() {
                    d.selected = Some(id.clone());
                    d.selected_key = (!keys.is_empty()).then_some(0);
                }
            }
        });
        // Kürzel der Auswahl
        let y = list.bottom() + 8.0;
        fields::label(ui, Rect::from_min_size(pos2(o.x, y), vec2(100.0, 20.0)), "Tastenkürzel:", d.selected.is_some());
        let keys: Vec<Shortcut> = d.selected.as_deref().map(|id| keys_of(&d.work, id).to_vec()).unwrap_or_default();
        let kl = Rect::from_min_size(pos2(o.x + 110.0, y), vec2(220.0, 60.0));
        let inner = fields::list_box(ui, kl);
        let mut kc = ui.new_child(egui::UiBuilder::new().max_rect(inner));
        kc.spacing_mut().item_spacing = vec2(0.0, 0.0);
        for (i, k) in keys.iter().enumerate() {
            if fields::list_item(&mut kc, &k.display_de(), d.selected_key == Some(i)).clicked() {
                d.selected_key = Some(i);
            }
        }
        let can_edit = d.selected.is_some() && !ro;
        let plus = Rect::from_min_size(pos2(kl.right() + 6.0, y), vec2(24.0, 20.0));
        let minus = Rect::from_min_size(pos2(kl.right() + 34.0, y), vec2(24.0, 20.0));
        let add_clicked = fields::button(ui, "sc-plus", plus, "+", can_edit);
        if fields::button(ui, "sc-minus", minus, "−", can_edit && d.selected_key.is_some())
            && let (Some(id), Some(i)) = (d.selected.clone(), d.selected_key)
        {
            let mut v = keys.clone();
            if i < v.len() {
                v.remove(i);
            }
            if let Err(e) = set_keys(&mut d.work, &id, v) {
                d.message = Some(e);
            } else {
                d.dirty = true;
                d.selected_key = None;
            }
        }
        fields::label(ui, Rect::from_min_size(pos2(o.x, y + 68.0), vec2(160.0, 20.0)), "Drücken Sie die Taste:", can_edit);
        let pr = Rect::from_min_size(pos2(o.x + 160.0, y + 68.0), vec2(170.0, 20.0));
        let presp = ui.interact(pr, egui::Id::new("sc-press"), if can_edit { Sense::click() } else { Sense::hover() });
        if presp.clicked() {
            d.capturing = true;
        }
        if d.capturing
            && can_edit
            && let Some(k) = captured_key(ui)
        {
            d.pressed = Some(k);
            d.capturing = false;
        }
        fields::field_frame(ui, pr, can_edit, d.capturing && can_edit);
        let ptxt = d.pressed.map(|k| k.display_de()).unwrap_or_else(|| if d.capturing { "…".into() } else { String::new() });
        ui.painter().text(pos2(pr.left() + 4.0, pr.center().y), Align2::LEFT_CENTER, ptxt, theme.ui_font(), c.field_text);
        // Konflikt (6261)
        let tool = d.section == 1;
        let conf = match (d.pressed, d.selected.as_deref()) {
            (Some(k), Some(id)) => conflict(st, &d.work, &k, id, tool),
            _ => None,
        };
        if let Some((_, label)) = &conf {
            ui.painter().text(pos2(o.x, y + 100.0), Align2::LEFT_CENTER, format!("Bereits vergeben an: {label}"), theme.small_font(), c.icon_red);
        }
        let assign_label = if conf.is_some() { "Neu zuweisen" } else { "Zuweisen" };
        let assign = fields::button(ui, "sc-assign", Rect::from_min_size(pos2(pr.right() + 6.0, pr.top()), vec2(90.0, 20.0)), assign_label, can_edit && d.pressed.is_some());
        if (assign || (add_clicked && d.pressed.is_some()))
            && let (Some(id), Some(k)) = (d.selected.clone(), d.pressed)
        {
            if let Some((other, _)) = &conf {
                let rest: Vec<Shortcut> = keys_of(&d.work, other).iter().copied().filter(|x| *x != k).collect();
                let _ = set_keys(&mut d.work, other, rest);
            }
            let mut v = keys.clone();
            if !v.contains(&k) {
                v.push(k);
            }
            match set_keys(&mut d.work, &id, v) {
                Ok(()) => {
                    d.dirty = true;
                    d.pressed = None;
                    d.message = None;
                }
                Err(e) => d.message = Some(e),
            }
        } else if add_clicked {
            d.capturing = true;
        }
        if ro {
            ui.painter().text(pos2(o.x, y + 118.0), Align2::LEFT_CENTER, "Mitgelieferte Sätze sind schreibgeschützt: zum Ändern zuerst „Duplizieren“.", theme.small_font(), c.text_weak);
        }
        if let Some(m) = &d.message {
            ui.painter().text(pos2(o.x, body.bottom() - 24.0), Align2::LEFT_CENTER, m, theme.small_font(), c.text);
        }
        if push_button(ui, Rect::from_min_size(pos2(body.right() - 176.0, body.bottom() - 36.0), vec2(76.0, 23.0)), "OK", true, true) {
            result = Some(true);
        }
        if push_button(ui, Rect::from_min_size(pos2(body.right() - 92.0, body.bottom() - 36.0), vec2(76.0, 23.0)), "Abbrechen", false, true) {
            result = Some(false);
        }
    });
    if let Some(n) = switch_to {
        let keep = (d.delete.clone(), d.section);
        d = ShortcutsDialog::for_set(st, &n);
        d.delete = keep.0;
        d.section = keep.1;
    }
    match (result, r) {
        (Some(true), _) => {
            st.dialogs.shortcuts = None;
            apply(st, &d);
        }
        (Some(false), _) | (_, DialogClose::Cancelled) => st.dialogs.shortcuts = None,
        _ => st.dialogs.shortcuts = Some(d),
    }
}

/// Übernimmt den Dialog: geänderten eigenen Satz speichern, gelöschte Sätze entfernen, Satz wählen.
pub fn apply(st: &mut AppState, d: &ShortcutsDialog) {
    if d.dirty
        && !d.read_only()
        && let Err(e) = fp_engine::shortcuts::save_custom_set(&d.work)
    {
        st.push(Action::Status(e));
        return;
    }
    for n in &d.delete {
        if !n.eq_ignore_ascii_case(&d.work.name) {
            let _ = crate::ui_commands::execute(&mut st.session, "edit.keyboardShortcuts", json!({"deleteSet": n}));
        }
    }
    if let Err(e) = crate::ui_commands::execute(&mut st.session, "edit.keyboardShortcuts", json!({"set": d.work.name})) {
        st.push(Action::Status(e.to_string()));
    }
}
