//! Tastaturkurzbefehle (6235): aktueller Satz (mitgelieferte und eigene Sätze), Satz duplizieren,
//! umbenennen, als HTML exportieren und löschen (6430), Befehlsbaum Menübefehle/Werkzeuge mit den
//! Kürzeln des Satzes, Suche, Kürzel hinzufügen (+), entfernen (−) und „Drücken Sie die Taste“ mit
//! Konfliktanzeige (6261: „Neu zuweisen“). Mitgelieferte Sätze sind schreibgeschützt – wie im
//! Original muss zuerst eine Kopie angelegt werden. Gearbeitet wird auf einer Kopie des Satzes;
//! erst OK speichert (fp-engine `shortcuts::save_custom_set`, Satzwahl über
//! `edit.keyboardShortcuts`), Abbrechen verwirft alles.

use super::{dialog_frame, push_button, refs, t, DialogClose};
use crate::app_state::{Action, AppState};
use crate::inspector::fields;
use crate::theme::Theme;
use egui::{pos2, vec2, Align2, Rect, Sense, Stroke, StrokeKind};
use fp_engine::shortcuts::{Shortcut, ShortcutSet};
use serde_json::json;
use std::collections::BTreeSet;

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
    /// Aufgeklappte Knoten des Menübaums (Pfad bis zum Knoten, Teile mit „ ▸ “ verbunden).
    pub expanded: BTreeSet<String>,
}

/// Zeile des Menübaums (Original: Baum der Menüs mit ⊞/⊟, PM-5 D17).
#[derive(Clone, Debug, PartialEq)]
pub enum TreeLine {
    /// Menü bzw. Untermenü: Tiefe, Beschriftung, Pfad (Schlüssel in `expanded`), aufgeklappt?
    Folder { depth: usize, label: String, path: String, open: bool },
    /// Befehl: Tiefe, Beschriftung, Index in den Zeilen.
    Leaf { depth: usize, label: String, row: usize },
}

/// Menübaum aus den Zeilen der Menübefehle (Pfade „Menü ▸ Untermenü ▸ Befehl“, nach Menüs
/// gruppiert). Nur Knoten unter aufgeklappten Menüs erscheinen.
pub fn tree_lines(rows: &[Row], expanded: &BTreeSet<String>) -> Vec<TreeLine> {
    let mut out = Vec::new();
    let mut shown: Vec<String> = Vec::new();
    for (i, (_, path, _)) in rows.iter().enumerate() {
        let parts: Vec<&str> = path.split(" ▸ ").collect();
        let n = parts.len();
        // gemeinsame Knoten mit der vorigen Zeile nicht wiederholen
        let common = shown.iter().zip(parts.iter()).take_while(|(a, b)| a.as_str() == **b).count().min(n - 1);
        shown.truncate(common);
        let mut visible = (0..common).all(|k| expanded.contains(&parts[..=k].join(" ▸ ")));
        for k in common..n - 1 {
            let p = parts[..=k].join(" ▸ ");
            let open = expanded.contains(&p);
            if visible {
                out.push(TreeLine::Folder { depth: k, label: parts[k].to_string(), path: p, open });
            }
            visible &= open;
            shown.push(parts[k].to_string());
        }
        if visible {
            out.push(TreeLine::Leaf { depth: n - 1, label: parts[n - 1].to_string(), row: i });
        }
    }
    out
}

/// Reihenfolge der Hauptmenüs (Datei, Bearbeiten, Ansicht …) in der aktuellen Sprache.
fn menu_order() -> Vec<String> {
    crate::menu::main_menu().iter().filter(|e| !e.sep).map(|e| crate::menu::clean_label(&e.local_text()).0).collect()
}

impl ShortcutsDialog {
    /// Mit dem aktuellen Satz.
    pub fn new(st: &AppState) -> ShortcutsDialog {
        ShortcutsDialog::for_set(st, &st.session.prefs.shortcut_set())
    }

    fn for_set(st: &AppState, name: &str) -> ShortcutsDialog {
        let work = fp_engine::shortcuts::any_set(name).cloned().unwrap_or_else(|| st.session.shortcut_set().clone());
        ShortcutsDialog { work, dirty: false, delete: Vec::new(), section: 0, filter: String::new(), selected: None, selected_key: None, pressed: None, capturing: false, name_edit: None, message: None, expanded: BTreeSet::new() }
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
            // Deutsch: Pfad aus dem Register wie bisher; andere Sprachen: übersetzter Menüpfad
            let path = if fp_i18n::current_language() == fp_i18n::SOURCE_LANG { info.menu_path.join(" ▸ ") } else { crate::menu::command_path(st, &info.name) };
            if !f.is_empty() && !path.to_lowercase().contains(&f) {
                continue;
            }
            out.push((format!("cmd:{cmd}"), path, set.command(&cmd).to_vec()));
        }
        // Hauptmenüs in Menüreihenfolge, darin alphabetisch (Untermenüs bleiben zusammen)
        let order = menu_order();
        let top = |p: &str| order.iter().position(|m| p.split(" ▸ ").next() == Some(m.as_str())).unwrap_or(usize::MAX);
        out.sort_by(|a, b| top(&a.1).cmp(&top(&b.1)).then_with(|| a.1.cmp(&b.1)));
    } else {
        for t in fp_engine::tools::TOOLS {
            let label = crate::panels::common::tool_label(t.id);
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
        _ => Err(t("dlg-sc-unknown-entry")),
    }
}

/// Wem ist das Kürzel im Satz schon zugeordnet (außer `except`)? Anzeigetext.
fn conflict(st: &AppState, set: &ShortcutSet, sc: &Shortcut, except: &str, tool: bool) -> Option<(String, String)> {
    if tool {
        let guid = set.tools_for(sc).into_iter().find(|g| format!("tool:{g}") != except)?;
        let label = fp_engine::tools::TOOLS.iter().find(|t| fp_engine::shortcuts::normalize_guid(t.guid) == guid).map(|t| crate::panels::common::tool_label(t.id)).unwrap_or_else(|| guid.to_string());
        return Some((format!("tool:{guid}"), label));
    }
    let cmd = set.commands_for(sc).into_iter().find(|c| format!("cmd:{c}") != except)?;
    let label = st
        .session
        .registry()
        .by_fw_cmd(cmd)
        .map(|c| if fp_i18n::current_language() == fp_i18n::SOURCE_LANG { c.info().menu_path.join(" ▸ ") } else { crate::menu::command_path(st, &c.info().name) })
        .unwrap_or_else(|| cmd.to_string());
    Some((format!("cmd:{cmd}"), label))
}

/// Beschreibung (Statuszeilentext) der gewählten Zeile: Menübefehl über den `cmd_`-Namen,
/// Werkzeug über seine Beschriftung.
pub fn selected_description(st: &AppState, id: Option<&str>) -> Option<String> {
    match id?.split_once(':')? {
        ("cmd", c) => {
            let name = st.session.registry().by_fw_cmd(c)?.info().name.clone();
            crate::menu::command_description(st, &name).filter(|s| !s.trim().is_empty())
        }
        _ => None,
    }
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
    let heading = fp_i18n::tr!("dlg-sc-html-title", name = set.name.as_str()).replace('&', "&amp;").replace('<', "&lt;");
    let mut s = format!("<!DOCTYPE html>\n<html><head><meta charset=\"utf-8\"><title>{heading}</title></head><body>\n<h1>{heading}</h1>\n");
    for (title, sec) in [(t("dlg-sc-menu-commands"), 0), (t("dlg-sc-tools"), 1)] {
        s.push_str(&format!("<h2>{title}</h2>\n<table border=\"1\" cellpadding=\"3\">\n"));
        for (_, label, keys) in rows(st, set, sec, "") {
            if keys.is_empty() {
                continue;
            }
            let k: Vec<String> = keys.iter().map(crate::menu::shortcut_text).collect();
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
    // Texte der aktuellen Sprache; Spalten und Knöpfe wachsen mit (Deutsch: Lage wie bisher)
    let set_l = t("dlg-sc-current-set");
    let btn_l = super::tl(&["dlg-sc-duplicate", "dlg-sc-rename", "dlg-sc-export", "dlg-sc-delete"]);
    let sections = super::tl(&["dlg-sc-menu-commands", "dlg-sc-tools"]);
    let (search_l, keys_l, press_l, ok_l, cancel_l) = (t("dlg-sc-search"), t("dlg-sc-shortcut"), t("dlg-sc-press-key"), t("btn-ok"), t("btn-cancel"));
    let assign_ls = [t("dlg-sc-assign"), t("dlg-sc-reassign")];
    let dx = super::extra_w(ctx, &[&set_l], 110.0 - 4.0);
    let bw = super::buttons_w(ctx, &refs(&btn_l), 68.0);
    let bx0 = 316.0 + dx;
    let sec_w = super::combo_fit_w(ctx, &refs(&sections), 160.0);
    let ds = sec_w - 160.0;
    let dk = super::extra_w(ctx, &[&keys_l], 110.0 - 4.0);
    let dp = super::extra_w(ctx, &[&press_l], 160.0 - 4.0);
    let aw = super::buttons_w(ctx, &[&assign_ls[0], &assign_ls[1]], 90.0);
    let width = 640f32
        .max(28.0 + bx0 + 4.0 * (bw + 4.0))
        .max(28.0 + 170.0 + ds + 136.0 + super::text_w(ctx, &search_l))
        .max(28.0 + 160.0 + dp + 170.0 + 6.0 + aw)
        .max(28.0 + 110.0 + dk + 220.0 + 6.0 + 58.0)
        .ceil();
    let r = dialog_frame(ctx, "fp-shortcuts", &t("dlg-sc-title"), vec2(width, 560.0), |ui, body| {
        let theme = Theme::current(ui.ctx());
        let c = &theme.colors;
        let o = body.min + vec2(14.0, 14.0);
        super::fit_label(ui, Rect::from_min_size(o, vec2(110.0 + dx, 20.0)), &set_l, true);
        let mut names = set_names();
        names.retain(|n| !d.delete.iter().any(|x| x.eq_ignore_ascii_case(n)));
        if !names.iter().any(|n| n.eq_ignore_ascii_case(&d.work.name)) {
            names.push(d.work.name.clone());
        }
        let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
        let cur = names.iter().position(|n| n.eq_ignore_ascii_case(&d.work.name));
        if let Some(i) = fields::combo(ui, "sc-set", Rect::from_min_size(o + vec2(116.0 + dx, 0.0), vec2(190.0, 20.0)), &refs, cur, true) {
            switch_to = Some(names[i].clone());
        }
        let ro = d.read_only();
        let btns = [(&btn_l[0], true), (&btn_l[1], !ro), (&btn_l[2], true), (&btn_l[3], !ro)];
        for (k, (label, en)) in btns.iter().enumerate() {
            let br = Rect::from_min_size(o + vec2(bx0 + k as f32 * (bw + 4.0), 0.0), vec2(bw, 20.0));
            if fields::button(ui, ("sc-btn", k), br, label, *en) {
                match k {
                    0 => d.name_edit = Some((false, fp_i18n::tr!("dlg-sc-copy-name", name = d.work.name.as_str()))),
                    1 => d.name_edit = Some((true, d.work.name.clone())),
                    2 => {
                        let html = set_html(st, &d.work);
                        let html_filter = t("dlg-sc-html-filter");
                        // Dateidialog ohne Blockieren: die Meldung landet im Dialog, sobald gewählt
                        super::panel_menus::pick_save(st, &t("dlg-sc-export-set"), &format!("{}.html", d.work.name), &[(html_filter.as_str(), &["html", "htm"])], move |st, p| {
                            let m = match std::fs::write(&p, html) {
                                Ok(()) => fp_i18n::tr!("dlg-export-file-done", path = p.display().to_string()),
                                Err(e) => fp_i18n::tr!("dlg-export-failed", error = e.to_string()),
                            };
                            if let Some(s) = st.dialogs.shortcuts.as_mut() {
                                s.message = Some(m);
                            }
                        });
                        if let Some(m) = st.dialogs.shortcuts.as_ref().and_then(|s| s.message.clone()) {
                            d.message = Some(m);
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
            let nr = Rect::from_min_size(o + vec2(bx0, 24.0), vec2(212.0, 20.0));
            let mut n = name.clone();
            if let Some(t) = fields::text(ui, "sc-name", nr, &name, true) {
                n = t;
                d.name_edit = Some((rename, n.clone()));
            }
            if fields::button(ui, "sc-name-ok", Rect::from_min_size(nr.right_top() + vec2(4.0, 0.0), vec2(super::button_w(ui.ctx(), &ok_l, 56.0), 20.0)), &ok_l, !n.trim().is_empty()) {
                let n = n.trim().to_string();
                if fp_engine::shortcuts::builtin_set(&n).is_some() {
                    d.message = Some(fp_i18n::tr!("dlg-sc-builtin-name", name = n.as_str()));
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
        if let Some(i) = fields::combo(ui, "sc-section", Rect::from_min_size(o + vec2(0.0, 30.0), vec2(sec_w, 20.0)), &super::refs(&sections), Some(d.section), true) {
            d.section = i;
            d.selected = None;
        }
        if let Some(t) = fields::text(ui, "sc-filter", Rect::from_min_size(o + vec2(170.0 + ds, 30.0), vec2(130.0, 20.0)), &d.filter, true) {
            d.filter = t;
        }
        ui.painter().text(o + vec2(306.0 + ds, 40.0), Align2::LEFT_CENTER, search_l.as_str(), theme.small_font(), c.text_weak);
        let list = Rect::from_min_max(o + vec2(0.0, 60.0), pos2(body.right() - 14.0, body.bottom() - 174.0));
        ui.painter().rect_filled(list, 0.0, c.list_bg);
        ui.painter().rect_stroke(list, 0.0, Stroke::new(1.0, c.list_border), StrokeKind::Inside);
        let rws = rows(st, &d.work, d.section, &d.filter);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(list.shrink(1.0)));
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(&mut child, |ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            // Menübefehle ohne Suche als Baum wie im Original, sonst (Suche, Werkzeuge) flach
            let lines: Vec<TreeLine> = if d.section == 0 && d.filter.trim().is_empty() {
                tree_lines(&rws, &d.expanded)
            } else {
                rws.iter().enumerate().map(|(i, r)| TreeLine::Leaf { depth: 0, label: r.1.clone(), row: i }).collect()
            };
            for line in &lines {
                let (rr, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 18.0), Sense::click());
                match line {
                    TreeLine::Folder { depth, label, path, open } => {
                        let x = rr.left() + 6.0 + *depth as f32 * 16.0;
                        if resp.hovered() {
                            ui.painter().rect_filled(rr, 0.0, c.list_hover_bg);
                        }
                        // Kästchen ⊞/⊟
                        let b = Rect::from_center_size(pos2(x + 4.5, rr.center().y), vec2(9.0, 9.0));
                        let stroke = Stroke::new(1.0, c.text_weak);
                        ui.painter().rect_stroke(b, 0.0, stroke, StrokeKind::Inside);
                        ui.painter().hline((b.left() + 2.0)..=(b.right() - 2.0), b.center().y, stroke);
                        if !open {
                            ui.painter().vline(b.center().x, (b.top() + 2.0)..=(b.bottom() - 2.0), stroke);
                        }
                        ui.painter().text(pos2(x + 14.0, rr.center().y), Align2::LEFT_CENTER, label, theme.ui_font(), c.text);
                        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
                        if resp.clicked() && !d.expanded.remove(path) {
                            d.expanded.insert(path.clone());
                        }
                    }
                    TreeLine::Leaf { depth, label, row } => {
                        let (id, _, keys) = &rws[*row];
                        let sel = d.selected.as_deref() == Some(id.as_str());
                        if sel {
                            ui.painter().rect_filled(rr, 0.0, c.list_selected_bg);
                        } else if resp.hovered() {
                            ui.painter().rect_filled(rr, 0.0, c.list_hover_bg);
                        }
                        let x = rr.left() + 6.0 + *depth as f32 * 16.0 + if *depth > 0 { 14.0 } else { 0.0 };
                        crate::widgets::text_clipped(ui.painter(), pos2(x, rr.center().y), Align2::LEFT_CENTER, label, theme.ui_font(), c.text, rr.left() + rr.width() * 0.68 - x);
                        let k: Vec<String> = keys.iter().map(crate::menu::shortcut_text).collect();
                        crate::widgets::text_clipped(ui.painter(), pos2(rr.left() + rr.width() * 0.7, rr.center().y), Align2::LEFT_CENTER, &k.join(", "), theme.ui_font(), c.text_weak, rr.width() * 0.3 - 6.0);
                        resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, sel, label));
                        if resp.clicked() {
                            d.selected = Some(id.clone());
                            d.selected_key = (!keys.is_empty()).then_some(0);
                        }
                    }
                }
            }
        });
        // Beschreibung des gewählten Befehls (Original: „Beschreibung:“ unter dem Baum)
        let desc_l = t("dlg-sc-description");
        let dy = list.bottom() + 6.0;
        let dw = super::text_w(ui.ctx(), &desc_l) + 8.0;
        ui.painter().text(pos2(o.x, dy + 10.0), Align2::LEFT_CENTER, &desc_l, theme.ui_font(), c.text);
        if let Some(desc) = selected_description(st, d.selected.as_deref()) {
            crate::widgets::text_clipped(ui.painter(), pos2(o.x + dw, dy + 10.0), Align2::LEFT_CENTER, &desc, theme.ui_font(), c.text, list.right() - o.x - dw);
        }
        // Kürzel der Auswahl
        let y = list.bottom() + 32.0;
        super::fit_label(ui, Rect::from_min_size(pos2(o.x, y), vec2(106.0 + dk, 20.0)), &keys_l, d.selected.is_some());
        let keys: Vec<Shortcut> = d.selected.as_deref().map(|id| keys_of(&d.work, id).to_vec()).unwrap_or_default();
        let kl = Rect::from_min_size(pos2(o.x + 110.0 + dk, y), vec2(220.0, 60.0));
        let inner = fields::list_box(ui, kl);
        let mut kc = ui.new_child(egui::UiBuilder::new().max_rect(inner));
        kc.spacing_mut().item_spacing = vec2(0.0, 0.0);
        for (i, k) in keys.iter().enumerate() {
            if fields::list_item(&mut kc, &crate::menu::shortcut_text(k), d.selected_key == Some(i)).clicked() {
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
        super::fit_label(ui, Rect::from_min_size(pos2(o.x, y + 68.0), vec2(156.0 + dp, 20.0)), &press_l, can_edit);
        let pr = Rect::from_min_size(pos2(o.x + 160.0 + dp, y + 68.0), vec2(170.0, 20.0));
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
        let ptxt = d.pressed.map(|k| crate::menu::shortcut_text(&k)).unwrap_or_else(|| if d.capturing { "…".into() } else { String::new() });
        ui.painter().text(pos2(pr.left() + 4.0, pr.center().y), Align2::LEFT_CENTER, ptxt, theme.ui_font(), c.field_text);
        // Konflikt (6261)
        let tool = d.section == 1;
        let conf = match (d.pressed, d.selected.as_deref()) {
            (Some(k), Some(id)) => conflict(st, &d.work, &k, id, tool),
            _ => None,
        };
        if let Some((_, label)) = &conf {
            crate::widgets::text_clipped(ui.painter(), pos2(o.x, y + 100.0), Align2::LEFT_CENTER, &fp_i18n::tr!("dlg-sc-conflict", target = label.as_str()), theme.small_font(), c.icon_red, body.width() - 28.0);
        }
        let assign_label = if conf.is_some() { &assign_ls[1] } else { &assign_ls[0] };
        let assign = fields::button(ui, "sc-assign", Rect::from_min_size(pos2(pr.right() + 6.0, pr.top()), vec2(aw, 20.0)), assign_label, can_edit && d.pressed.is_some());
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
            // Hinweis bricht bei längeren Texten um (Platz bis zur Meldungszeile)
            super::text_block(ui, Rect::from_min_size(pos2(o.x, y + 111.0), vec2(body.width() - 28.0, 28.0)), egui::RichText::new(t("dlg-sc-read-only")).font(theme.small_font()).color(c.text_weak));
        }
        let b = super::button_row(ui.ctx(), body.right() - 16.0, body.bottom() - 36.0, &[&ok_l, &cancel_l], 76.0, 23.0, 8.0);
        if let Some(m) = &d.message {
            crate::widgets::text_clipped(ui.painter(), pos2(o.x, body.bottom() - 24.0), Align2::LEFT_CENTER, m, theme.small_font(), c.text, b[0].left() - 8.0 - o.x);
        }
        if push_button(ui, b[0], &ok_l, true, true) {
            result = Some(true);
        }
        if push_button(ui, b[1], &cancel_l, false, true) {
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
