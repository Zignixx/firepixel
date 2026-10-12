//! Dokumentbibliothek (`CSymLibPanel`, 4226), Allgemeine Bibliothek (4305) und Automatische Formen.
//!
//! - Dokumentbibliothek: Vorschau des gewählten Symbols (fp-render), Liste mit Name und Art,
//!   Doppelklick/„Einfügen“ setzt eine Instanz in die Leinwandmitte, Umbenennen, Löschen (mit
//!   Instanzen), Neues Symbol aus der Auswahl (Dialog „In Symbol konvertieren“).
//! - Allgemeine Bibliothek: eigene Bibliotheksordner (`%APPDATA%\Firepixel\Allgemeine Bibliothek`)
//!   und – nur zur Laufzeit, falls vorhanden – die Ordner im Ressourcenordner des Originals
//!   (`Configuration\Libraries`). Es wird nichts eingebettet. Einfügen über `pi.symbol.import`.
//! - Automatische Formen: Liste der Formen; die Skripte brauchen fp-script („noch nicht verfügbar“).

use super::common::{self, FooterButton};
use super::thumbs;
use crate::app_state::{Action, AppState};
use crate::inspector::glyphs::Glyph;
use crate::theme::Theme;
use egui::{pos2, vec2, Align2, Color32, Id, Rect, Sense, Stroke, StrokeKind, Ui};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

const RENAME: &str = "doclib-rename";

fn kind_label(k: fp_model::SymbolKind) -> String {
    common::t(match k {
        fp_model::SymbolKind::Graphic => "panel-doclib-kind-graphic",
        fp_model::SymbolKind::Button => "panel-doclib-kind-button",
        fp_model::SymbolKind::Animation => "panel-doclib-kind-animation",
        fp_model::SymbolKind::Widget => "panel-doclib-kind-component",
        fp_model::SymbolKind::Other(_) => "panel-doclib-kind-symbol",
    })
}

/// Dokumentbibliothek.
pub fn document(ui: &mut Ui, st: &mut AppState) {
    let theme = Theme::current(ui.ctx());
    let c = theme.colors.clone();
    let Some(d) = st.session.active() else { return };
    let doc = Arc::clone(&d.doc);
    let (doc_id, doc_gen) = (d.id, d.generation);
    let sel: Option<usize> = st.dialogs.panel_sel.symbol.filter(|i| *i < doc.symbols.len());
    let rename: Option<usize> = ui.data(|dd| dd.get_temp(Id::new(RENAME)));
    let mut actions = Vec::new();
    let mut new_sel = None;
    // Vorschau
    let (pv, _) = ui.allocate_exact_size(vec2(ui.available_width(), 96.0), Sense::hover());
    let pr = pv.shrink(4.0);
    ui.painter().rect_filled(pr, 0.0, Color32::WHITE);
    ui.painter().rect_stroke(pr, 0.0, Stroke::new(1.0, c.list_border), StrokeKind::Inside);
    if let Some(i) = sel
        && let Some(tex) = thumbs::symbol_thumb(ui.ctx(), st, doc_id, doc_gen, &doc, i, 128)
    {
        let ts = tex.size_vec2();
        let k = (pr.width() / ts.x).min(pr.height() / ts.y).min(1.0) * 0.95;
        ui.painter().image(tex.id(), Rect::from_center_size(pr.center(), ts * k), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
    }
    // Kopfzeile der Liste
    let (hr, _) = ui.allocate_exact_size(vec2(ui.available_width(), 16.0), Sense::hover());
    let count = fp_i18n::tr!("panel-doclib-count", n = doc.symbols.len());
    common::fit_text(ui, Rect::from_min_max(pos2(hr.left() + 26.0, hr.top()), pos2(hr.right() - 74.0, hr.bottom())), Align2::LEFT_CENTER, &count, theme.small_font(), c.text);
    common::fit_text(ui, Rect::from_min_max(pos2(hr.right() - 70.0, hr.top()), pos2(hr.right() - 2.0, hr.bottom())), Align2::LEFT_CENTER, &common::t("panel-doclib-kind"), theme.small_font(), c.text);
    common::list_frame(ui, "doclib-list", common::FOOTER_H, |ui| {
        for (i, s) in doc.symbols.iter().enumerate() {
            let (r, resp) = common::row(ui, 20.0, sel == Some(i));
            crate::inspector::glyphs::paint(ui.painter(), Rect::from_min_size(pos2(r.left() + 4.0, r.top() + 3.0), vec2(14.0, 14.0)), if s.kind() == fp_model::SymbolKind::Button { Glyph::FillSolid } else { Glyph::Bitmap }, &theme, true);
            let nr = Rect::from_min_max(pos2(r.left() + 24.0, r.top() + 1.0), pos2(r.right() - 74.0, r.bottom() - 1.0));
            if rename == Some(i) {
                if let Some(res) = common::inline_rename(ui, Id::new(("sym-rename", i)), nr, &s.name) {
                    if let Some(n) = res {
                        actions.push(Action::Run("pi.symbol.rename".into(), json!({"index": i, "name": n})));
                    }
                    ui.data_mut(|dd| dd.remove::<usize>(Id::new(RENAME)));
                }
            } else {
                crate::widgets::text_clipped(ui.painter(), pos2(nr.left(), r.center().y), Align2::LEFT_CENTER, &s.name, theme.ui_font(), c.text, nr.width());
            }
            crate::widgets::text_clipped(ui.painter(), pos2(r.right() - 70.0, r.center().y), Align2::LEFT_CENTER, &kind_label(s.kind()), theme.small_font(), c.text_weak, 68.0);
            if resp.clicked() {
                new_sel = Some(i);
            }
            // Symbol auf die Leinwand ziehen: Instanz an der Ablagestelle
            if resp.drag_started() {
                super::drag_drop::start(ui.ctx(), super::drag_drop::Payload::Symbol(i));
                new_sel = Some(i);
            }
            if resp.double_clicked() {
                if resp.interact_pointer_pos().is_some_and(|p| p.x < nr.right()) {
                    ui.data_mut(|dd| dd.insert_temp(Id::new(RENAME), i));
                } else {
                    actions.push(Action::Run("pi.symbol.place".into(), json!({"index": i})));
                }
            }
        }
    });
    let has_sel = st.session.active().is_some_and(|d| !d.selection.elements.is_empty());
    let tips: [String; 3] = ["panel-doclib-place", "panel-doclib-new-symbol", "panel-doclib-delete"].map(common::t);
    let buttons = [
        FooterButton { glyph: Glyph::Play, tip: &tips[0], enabled: sel.is_some(), action: sel.map(|i| Action::Run("pi.symbol.place".into(), json!({"index": i}))) },
        FooterButton { glyph: Glyph::StyleNew, tip: &tips[1], enabled: has_sel, action: Some(Action::Run("modify.symbol.convertToSymbol".into(), json!({}))) },
        FooterButton { glyph: Glyph::Trash, tip: &tips[2], enabled: sel.is_some(), action: sel.map(|i| Action::Run("panel.doclib.delete".into(), json!({"index": i}))) },
    ];
    if let Some(a) = common::footer(ui, 8.0, "", &buttons) {
        actions.push(a);
    }
    for a in actions {
        match &a {
            Action::Run(n, p) if n == "panel.doclib.delete" => {
                let i = p.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let name = doc.symbols.get(i).map(|s| s.name.clone()).unwrap_or_default();
                st.dialogs.confirm = Some(crate::dialogs::Confirm {
                    title: common::t("panel-doclib-delete-title"),
                    text: fp_i18n::tr!("panel-doclib-delete-text", name = name.as_str()),
                    command: "pi.symbol.delete".into(),
                    params: json!({"index": i}),
                });
                st.dialogs.panel_sel.symbol = None;
            }
            _ => st.push(a),
        }
    }
    if let Some(i) = new_sel {
        st.dialogs.panel_sel.symbol = Some(i);
    }
}

// ---------------------------------------------------------------------- Allgemeine Bibliothek

/// Bibliotheksordner: eigener Ordner und (zur Laufzeit) der Ressourcenordner des Originals.
pub fn library_roots() -> Vec<(String, PathBuf)> {
    let mut v = Vec::new();
    if let Some(dir) = fp_engine::Preferences::default_path().and_then(|p| p.parent().map(|d| d.join("Allgemeine Bibliothek"))) {
        let _ = std::fs::create_dir_all(&dir);
        v.push((common::t("panel-commonlib-own"), dir));
    }
    if let Some(p) = fp_color::install::original_subdir("Configuration\\Libraries") {
        v.push((common::t("panel-commonlib-original"), p));
    }
    v
}

fn list_dir(p: &std::path::Path) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut dirs = Vec::new();
    let mut files = Vec::new();
    if let Ok(rd) = std::fs::read_dir(p) {
        for e in rd.flatten() {
            let path = e.path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|x| x.eq_ignore_ascii_case("png")) {
                files.push(path);
            }
        }
    }
    dirs.sort();
    files.sort();
    (dirs, files)
}

/// Allgemeine Bibliothek.
pub fn common(ui: &mut Ui, st: &mut AppState) {
    let theme = Theme::current(ui.ctx());
    let c = theme.colors.clone();
    let open_key = Id::new("commonlib-open");
    let mut open: std::collections::BTreeSet<String> = ui.data(|d| d.get_temp(open_key)).unwrap_or_default();
    let sel_key = Id::new("commonlib-sel");
    let sel: Option<PathBuf> = ui.data(|d| d.get_temp(sel_key));
    let mut new_sel = None;
    let mut toggle = None;
    let mut insert = None;
    let roots = library_roots();
    common::list_frame(ui, "commonlib-tree", common::FOOTER_H, |ui| {
        #[allow(clippy::too_many_arguments)]
        fn walk(ui: &mut Ui, p: &std::path::Path, label: &str, depth: usize, open: &std::collections::BTreeSet<String>, sel: &Option<PathBuf>, theme: &Theme, toggle: &mut Option<String>, new_sel: &mut Option<PathBuf>, insert: &mut Option<PathBuf>) {
            let key = p.to_string_lossy().to_string();
            let is_open = open.contains(&key);
            let (r, resp) = common::row(ui, 20.0, false);
            let x = r.left() + 4.0 + depth as f32 * 14.0;
            let tri = Rect::from_min_size(pos2(x, r.top() + 5.0), vec2(10.0, 10.0));
            let cc = tri.center();
            let pts = if is_open { vec![pos2(cc.x - 4.0, cc.y - 2.0), pos2(cc.x + 4.0, cc.y - 2.0), pos2(cc.x, cc.y + 3.0)] } else { vec![pos2(cc.x - 2.0, cc.y - 4.0), pos2(cc.x + 3.0, cc.y), pos2(cc.x - 2.0, cc.y + 4.0)] };
            ui.painter().add(egui::Shape::convex_polygon(pts, theme.colors.icon_fg, Stroke::NONE));
            crate::inspector::glyphs::paint(ui.painter(), Rect::from_min_size(pos2(x + 12.0, r.top() + 2.0), vec2(16.0, 16.0)), Glyph::Folder, theme, true);
            crate::widgets::text_clipped(ui.painter(), pos2(x + 32.0, r.center().y), Align2::LEFT_CENTER, label, theme.ui_font(), theme.colors.text, r.right() - x - 34.0);
            if resp.clicked() {
                *toggle = Some(key.clone());
            }
            if !is_open {
                return;
            }
            let (dirs, files) = list_dir(p);
            for d in dirs {
                let name = d.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                walk(ui, &d, &name, depth + 1, open, sel, theme, toggle, new_sel, insert);
            }
            for f in files {
                let selected = sel.as_ref() == Some(&f);
                let (r2, resp2) = common::row(ui, 20.0, selected);
                let name = f.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                let name = name.trim_end_matches(".png").trim_end_matches(".graphic").to_string();
                crate::inspector::glyphs::paint(ui.painter(), Rect::from_min_size(pos2(r2.left() + 18.0 + (depth + 1) as f32 * 14.0, r2.top() + 3.0), vec2(14.0, 14.0)), Glyph::Bitmap, theme, true);
                let nx = r2.left() + 36.0 + (depth + 1) as f32 * 14.0;
                crate::widgets::text_clipped(ui.painter(), pos2(nx, r2.center().y), Align2::LEFT_CENTER, &name, theme.ui_font(), theme.colors.text, r2.right() - nx - 2.0);
                if resp2.clicked() {
                    *new_sel = Some(f.clone());
                }
                if resp2.drag_started() {
                    super::drag_drop::start(ui.ctx(), super::drag_drop::Payload::LibraryFile(f.clone()));
                }
                if resp2.double_clicked() {
                    *insert = Some(f.clone());
                }
            }
        }
        for (label, p) in &roots {
            walk(ui, p, label, 0, &open, &sel, &theme, &mut toggle, &mut new_sel, &mut insert);
        }
        if roots.is_empty() {
            ui.label(egui::RichText::new(common::t("panel-commonlib-empty")).color(c.text_disabled));
        }
    });
    if let Some(k) = toggle {
        if !open.remove(&k) {
            open.insert(k);
        }
        ui.data_mut(|d| d.insert_temp(open_key, open));
    }
    if let Some(s) = new_sel {
        ui.data_mut(|d| d.insert_temp(sel_key, s));
    }
    let has_doc = st.session.active().is_some();
    let target = insert;
    let tip_insert = common::t("panel-commonlib-insert");
    let buttons = [FooterButton { glyph: Glyph::Play, tip: &tip_insert, enabled: has_doc && sel.is_some(), action: sel.clone().map(|p| Action::Run("pi.symbol.import".into(), json!({"path": p.to_string_lossy()}))) }];
    if let Some(a) = common::footer(ui, 8.0, "", &buttons) {
        st.push(a);
    }
    if let Some(p) = target
        && has_doc
    {
        st.run("pi.symbol.import", json!({"path": p.to_string_lossy()}));
    }
}
