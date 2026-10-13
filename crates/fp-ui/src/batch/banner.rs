//! Banner des Studios (SPEC-BATCH §2.2, §2.5, §2.10): Logo, „Batch-Verarbeitung“, Untertitel
//! (Dateien · Größe · Rezept, ungespeichert •), Umschalter Anfänger | Profi, Menü „Rezepte ▾“
//! (Vorlagen, Meine Rezepte, Öffnen, Speichern, Speichern unter, Leeres Rezept), Menü ⋯.

use super::kit::{self, BtnKind};
use super::{Inbox, Mode, Studio};
use crate::app_state::AppState;
use crate::theme::Theme;
use crate::ui_kit::{sp, RowIcon, Tone};
use egui::{pos2, vec2, Id, Rect, Ui};
use fp_batch::Recipe;
use fp_i18n::t;
use std::path::{Path, PathBuf};

/// Rezept laden (Antwort des Datei-Dialogs bzw. „Meine Rezepte“).
pub fn load_recipe(s: &mut Studio, path: &Path, now: f64) {
    match Recipe::load(path) {
        Ok(r) => {
            s.set_recipe(r, Some(path.to_path_buf()));
            s.notify(Tone::Success, fp_i18n::tr!("batch-ui-recipe-loaded", name = super::recipe_name_from_path(path)), now);
        }
        Err(e) => s.notify(Tone::Error, fp_i18n::tr!("batch-ui-recipe-load-failed", error = e.to_string()), now),
    }
}

/// Mit Endung `.fpbatch.json`.
pub fn with_extension(p: &Path) -> PathBuf {
    let s = p.to_string_lossy().to_string();
    let ext = format!(".{}", fp_batch::recipe::RECIPE_EXTENSION);
    if s.to_lowercase().ends_with(&ext) {
        p.to_path_buf()
    } else {
        let base = s.strip_suffix(".json").unwrap_or(&s);
        PathBuf::from(format!("{base}{ext}"))
    }
}

/// Rezept speichern.
pub fn save_recipe(s: &mut Studio, path: &Path, now: f64) {
    let path = with_extension(path);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if s.recipe.name.trim().is_empty() {
        s.recipe.name = super::recipe_name_from_path(&path);
    }
    match s.recipe.save(&path) {
        Ok(()) => {
            s.mark_saved(path.clone());
            s.notify(Tone::Success, fp_i18n::tr!("batch-ui-recipe-saved", name = super::recipe_name_from_path(&path)), now);
        }
        Err(e) => s.notify(Tone::Error, e.to_string(), now),
    }
}

/// Gespeicherte Rezepte im Rezeptordner.
pub fn my_recipes() -> Vec<PathBuf> {
    let Some(dir) = fp_batch::recipe_dir() else { return Vec::new() };
    let ext = format!(".{}", fp_batch::recipe::RECIPE_EXTENSION);
    let mut v: Vec<PathBuf> = std::fs::read_dir(&dir).map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.to_string_lossy().to_lowercase().ends_with(&ext)).collect()).unwrap_or_default();
    v.sort_by(|a, b| fp_batch::item::natural_cmp(&a.to_string_lossy(), &b.to_string_lossy()));
    v
}

fn ask_save_as(st: &mut AppState, s: &Studio) {
    let inbox = s.inbox_sender();
    let dir = fp_batch::recipe_dir();
    if let Some(d) = &dir {
        let _ = std::fs::create_dir_all(d);
    }
    let name = format!("{}.{}", sanitize(&s.recipe_title()), fp_batch::recipe::RECIPE_EXTENSION);
    crate::files::pick_save(st, &t("batch-ui-menu-save-as"), &name, dir.as_deref(), move |_st, path| {
        if let Some(p) = path {
            inbox.lock().unwrap_or_else(|e| e.into_inner()).push(Inbox::SaveRecipe(p));
        }
    });
}

fn sanitize(name: &str) -> String {
    let mut hints = Vec::new();
    fp_batch::naming::sanitize_stem(name, &mut hints)
}

fn ask_open(st: &mut AppState, s: &Studio) {
    let inbox = s.inbox_sender();
    crate::files::pick_one(st, &t("batch-ui-menu-open"), &t("batch-ui-filter-recipes"), &["json"], move |_st, p| {
        inbox.lock().unwrap_or_else(|e| e.into_inner()).push(Inbox::LoadRecipe(p));
    });
}

/// Zeichnet das Banner.
pub fn show(ui: &mut Ui, st: &mut AppState, s: &mut Studio, r: Rect, now: f64) {
    let theme = Theme::current(ui.ctx());
    let p = ui.painter().clone();
    let running = s.running();
    // rechts: Umschalter, Rezepte ▾, ⋯
    let narrow = r.width() < 560.0;
    let menu_l = t("batch-ui-recipes");
    let mw = kit::btn_w(ui.ctx(), &menu_l, true);
    let dots = 26.0;
    let modes = vec![t("batch-ui-mode-beginner"), t("batch-ui-mode-pro")];
    let seg_w = modes.iter().map(|m| kit::btn_w(ui.ctx(), m, false)).sum::<f32>() + 4.0;
    let cy = r.center().y;
    let x_dots = r.right() - sp::L - dots;
    let x_menu = x_dots - sp::S - mw;
    let show_seg = !narrow || r.width() > seg_w + mw + dots + 200.0;
    let x_seg = x_menu - sp::M - seg_w;
    let text_right = if show_seg { x_seg - sp::M } else { x_menu - sp::M };
    // Untertitel
    let vis = s.visible();
    let bytes: u64 = vis.iter().map(|&i| s.files[i].info.bytes).sum();
    let dirty = if s.is_dirty() && (s.recipe_path.is_some() || !s.recipe.steps.is_empty()) { format!(" {}", t("batch-ui-unsaved")) } else { String::new() };
    let subtitle = fp_i18n::tr!("batch-ui-subtitle", n = vis.len(), size = fp_i18n::fmt_size(bytes), recipe = s.recipe_title()) + &dirty;
    let pixels_right = if show_seg { x_seg } else { x_menu } - sp::S;
    kit::paint_banner(&p, r, &theme, &t("batch-ui-title"), &subtitle, text_right, pixels_right);
    if show_seg {
        let sr = Rect::from_min_size(pos2(x_seg, cy - 13.0), vec2(seg_w, 26.0));
        let cur = if s.mode == Mode::Pro { 1 } else { 0 };
        if let Some(k) = kit::segmented(ui, sr, Id::new("fp-batch-mode"), &modes, cur, true) {
            s.mode = if k == 1 { Mode::Pro } else { Mode::Beginner };
            s.show_more = false;
            s.files_changed();
            st.session.prefs.set(super::PREF_MODE, fp_engine::PrefValue::Str(if k == 1 { "pro".into() } else { "beginner".into() }));
            st.session.prefs_changed();
        }
    }
    // Rezepte ▾
    let mr = Rect::from_min_size(pos2(x_menu, cy - 13.0), vec2(mw, 26.0));
    let mresp = kit::button(ui, mr, Id::new("fp-batch-recipes"), &menu_l, Some(RowIcon::Icon(crate::icons::Icon::DropDown)), BtnKind::Secondary, !running, &menu_l).on_hover_text(t("batch-ui-recipes.tooltip"));
    egui::Popup::menu(&mresp).id(Id::new("fp-batch-recipes-menu")).show(|ui| {
        crate::theme::field_widgets(ui);
        ui.set_min_width(220.0);
        let tpl = super::templates();
        if !tpl.is_empty() {
            ui.label(egui::RichText::new(t("batch-ui-menu-templates")).strong());
            for tp in tpl {
                if ui.button(&tp.title).on_hover_text(&tp.description).clicked() {
                    s.set_recipe(tp.recipe.clone(), None);
                }
            }
            ui.separator();
        }
        ui.label(egui::RichText::new(t("batch-ui-menu-mine")).strong());
        let mine = my_recipes();
        if mine.is_empty() {
            ui.label(egui::RichText::new(t("batch-ui-menu-none")).weak());
        }
        for path in mine.iter().take(20) {
            if ui.button(super::recipe_name_from_path(path)).on_hover_text(path.display().to_string()).clicked() {
                load_recipe(s, path, now);
            }
        }
        ui.separator();
        if ui.button(t("batch-ui-menu-open")).clicked() {
            ask_open(st, s);
        }
        if ui.button(t("batch-ui-menu-save")).clicked() {
            match s.recipe_path.clone() {
                Some(p) => save_recipe(s, &p, now),
                None => ask_save_as(st, s),
            }
        }
        if ui.button(t("batch-ui-menu-save-as")).clicked() {
            ask_save_as(st, s);
        }
        ui.separator();
        if ui.button(t("batch-ui-menu-new")).clicked() {
            s.set_recipe(Recipe::default(), None);
        }
    });
    // ⋯: Modus (schmal), Unterordner, Rückgängig/Wiederholen, Schließen
    let dr = Rect::from_min_size(pos2(x_dots, cy - 13.0), vec2(dots, 26.0));
    let more_name = t("batch-ui-more");
    let dresp = kit::button(ui, dr, Id::new("fp-batch-more"), "", None, BtnKind::Secondary, true, &more_name);
    kit::dots(ui.painter(), dr.center(), theme.colors.text);
    egui::Popup::menu(&dresp).id(Id::new("fp-batch-more-menu")).show(|ui| {
        crate::theme::field_widgets(ui);
        ui.set_min_width(200.0);
        if !show_seg {
            let mut pro = s.mode == Mode::Pro;
            if ui.checkbox(&mut pro, t("batch-ui-mode-pro")).changed() {
                s.mode = if pro { Mode::Pro } else { Mode::Beginner };
                s.files_changed();
            }
        }
        let mut rec = s.recursive;
        if ui.checkbox(&mut rec, t("batch-ui-recursive")).changed() {
            s.recursive = rec;
            st.session.prefs.set(super::PREF_RECURSIVE, fp_engine::PrefValue::Bool(rec));
            st.session.prefs_changed();
        }
        ui.separator();
        if ui.add_enabled(s.can_undo() && !running, egui::Button::new(t("batch-ui-undo")).shortcut_text(fp_i18n::shortcut_label("Ctrl+Z"))).clicked() {
            s.undo();
        }
        if ui.add_enabled(s.can_redo() && !running, egui::Button::new(t("batch-ui-redo")).shortcut_text(fp_i18n::shortcut_label("Ctrl+Y"))).clicked() {
            s.redo();
        }
        ui.separator();
        // BATCH-11a: Batch-Einstellungen (ffmpeg-Datei …)
        if ui.button(t("batch-settings-open")).clicked() {
            super::settings::open(ui.ctx(), s);
        }
        if ui.button(t("batch-ui-close")).clicked() {
            s.close_requested = true;
        }
    });
}
