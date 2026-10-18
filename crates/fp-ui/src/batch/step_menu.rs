//! Card menu ⋯ of a step (BA-107): "Save as Template…" writes a recipe with this step into the
//! user's template folder (`BatchRecipes\Templates`, name asked in a small overlay); it then
//! appears in the banner's template list ([`user_templates`], registered as a template source).
//! "Help" opens the batch documentation (`BATCH.md`) and names the section that describes the
//! step ([`help_section`]); without that file it falls back to the general help.

use super::kit::{self, BtnKind};
use super::{Studio, Template};
use crate::app_state::AppState;
use crate::theme::{Theme, with_alpha};
use crate::ui_kit::{KitIcon, RowIcon, Tone, sp, type_scale};
use egui::{Color32, Id, Key, Rect, Sense, Ui, pos2, vec2};
use fp_batch::Recipe;
use fp_i18n::t;
use std::path::{Path, PathBuf};

/// Subfolder of the recipe folder with the user's templates.
pub const TEMPLATE_FOLDER: &str = "Templates";
/// File name of the batch documentation.
pub const HELP_FILE: &str = "BATCH.md";

/// Open "Save as Template" prompt.
#[derive(Clone, Debug)]
pub struct TemplatePrompt {
    /// Step index in the recipe.
    pub step: usize,
    pub name: String,
    /// Focus the name field in the next frame.
    pub(crate) focus: bool,
}

/// Folder of the user's templates (`%APPDATA%\Firepixel\BatchRecipes\Templates`).
pub fn template_dir() -> Option<PathBuf> {
    fp_batch::recipe_dir().map(|d| d.join(TEMPLATE_FOLDER))
}

/// Template source for the banner's list: every recipe in the user's template folder.
pub fn user_templates() -> Vec<Template> {
    let Some(dir) = template_dir() else {
        return Vec::new();
    };
    let ext = format!(".{}", fp_batch::recipe::RECIPE_EXTENSION);
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| p.to_string_lossy().to_lowercase().ends_with(&ext))
                .collect()
        })
        .unwrap_or_default();
    paths.sort_by(|a, b| fp_batch::item::natural_cmp(&a.to_string_lossy(), &b.to_string_lossy()));
    paths
        .into_iter()
        .filter_map(|p| {
            let recipe = Recipe::load(&p).ok()?;
            let title = if recipe.name.trim().is_empty() {
                super::recipe_name_from_path(&p)
            } else {
                recipe.name.clone()
            };
            let description = recipe
                .steps
                .iter()
                .map(|s| fp_batch::step::step_label(&s.id))
                .collect::<Vec<_>>()
                .join(" → ");
            Some(Template {
                title,
                description,
                recipe,
            })
        })
        .collect()
}

/// Opens the name prompt for step `i` (name proposed from the step's title).
pub fn ask_template_name(s: &mut Studio, i: usize) {
    let Some(step) = s.recipe.steps.get(i) else {
        return;
    };
    s.template_prompt = Some(TemplatePrompt {
        step: i,
        name: fp_batch::step::step_label(&step.id),
        focus: true,
    });
}

/// Writes step `i` as template `name` into the template folder. Returns the file.
pub fn save_step_template(s: &mut Studio, i: usize, name: &str, now: f64) -> Option<PathBuf> {
    let name = name.trim();
    let step = s.recipe.steps.get(i)?.clone();
    if name.is_empty() {
        return None;
    }
    let Some(dir) = template_dir() else {
        s.notify(Tone::Error, t("batch-ui-template-no-folder"), now);
        return None;
    };
    let mut hints = Vec::new();
    let stem = fp_batch::naming::sanitize_stem(name, &mut hints);
    let path = dir.join(format!("{stem}.{}", fp_batch::recipe::RECIPE_EXTENSION));
    let mut recipe = Recipe::named(name);
    recipe.steps.push(step);
    let res = std::fs::create_dir_all(&dir)
        .map_err(|e| e.to_string())
        .and_then(|_| recipe.save(&path).map_err(|e| e.to_string()));
    match res {
        Ok(()) => {
            s.notify(
                Tone::Success,
                fp_i18n::tr!("batch-ui-template-saved", name = name.to_string()),
                now,
            );
            Some(path)
        }
        Err(e) => {
            s.notify(
                Tone::Error,
                fp_i18n::tr!("batch-ui-template-save-failed", error = e),
                now,
            );
            None
        }
    }
}

/// Candidate places of `BATCH.md`: next to the manual, `docs` beside the program and its parents,
/// and `docs` in the working folder and its parents.
fn help_candidates() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(d) = fp_engine::commands::misc::help_file().parent() {
        v.push(d.join(HELP_FILE));
        v.push(d.join("docs").join(HELP_FILE));
    }
    let mut ancestors = |start: Option<PathBuf>| {
        let mut d = start;
        for _ in 0..6 {
            let Some(x) = d else { break };
            v.push(x.join("docs").join(HELP_FILE));
            v.push(x.join(HELP_FILE));
            d = x.parent().map(Path::to_path_buf);
        }
    };
    ancestors(
        std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(Path::to_path_buf)),
    );
    ancestors(std::env::current_dir().ok());
    v
}

/// The batch documentation file, if it is installed.
pub fn help_file() -> Option<PathBuf> {
    help_candidates().into_iter().find(|p| p.is_file())
}

/// Heading of the section in `text` that describes step `id` (its row in the step catalogue
/// starts with `` | `id` ``), or the first heading that names the id.
pub fn section_for(text: &str, id: &str) -> Option<String> {
    let row = format!("| `{id}`");
    let code = format!("`{id}`");
    let mut heading: Option<String> = None;
    let mut fallback: Option<String> = None;
    for line in text.lines() {
        if line.starts_with('#') {
            heading = Some(line.trim_start_matches('#').trim().to_string());
            continue;
        }
        if line.trim_start().starts_with(&row) {
            return heading;
        }
        if fallback.is_none() && line.contains(&code) {
            fallback = heading.clone();
        }
    }
    fallback
}

/// Documentation file and section for step `id`.
pub fn help_section(id: &str) -> Option<(PathBuf, Option<String>)> {
    let f = help_file()?;
    let text = std::fs::read_to_string(&f).ok()?;
    Some((f.clone(), section_for(&text, id)))
}

/// "Help" in the card menu: opens `BATCH.md` (section named in a notice) or the general help.
pub fn open_help(st: &mut AppState, s: &mut Studio, i: usize, now: f64) {
    let Some(id) = s.recipe.steps.get(i).map(|c| c.id.clone()) else {
        return;
    };
    match help_section(&id) {
        Some((file, section)) => {
            let _ = fp_engine::shell::shell_open(&file, None);
            let where_ = section.unwrap_or_else(|| HELP_FILE.to_string());
            s.notify(
                Tone::Info,
                fp_i18n::tr!(
                    "batch-ui-step-help-opened",
                    step = fp_batch::step::step_label(&id),
                    section = where_
                ),
                now,
            );
        }
        None => {
            st.run("help.firepixelHelp", serde_json::json!({}));
        }
    }
}

/// Draws the name prompt over the Studio (Enter saves, Esc cancels).
pub fn prompt_overlay(ui: &mut Ui, s: &mut Studio, area: Rect, now: f64) {
    let Some(mut pr) = s.template_prompt.clone() else {
        return;
    };
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let layer = egui::LayerId::new(egui::Order::Foreground, Id::new("fp-batch-template-prompt"));
    let painter = ui.ctx().layer_painter(layer);
    painter.rect_filled(
        area,
        0.0,
        with_alpha(Color32::BLACK, if theme.glow { 150 } else { 90 }),
    );
    let mut lay = ui.new_child(egui::UiBuilder::new().layer_id(layer).max_rect(area));
    let _ = lay.interact(
        area,
        Id::new("fp-batch-template-block"),
        Sense::click_and_drag(),
    );
    let w = 420.0f32.min(area.width() - 2.0 * sp::L).max(260.0);
    let field_h = 24.0;
    let h = kit::HEAD_H + kit::PAD + 16.0 + sp::XS + field_h + sp::L + 26.0 + kit::PAD;
    let r = Rect::from_center_size(area.center(), vec2(w, h));
    let inner = kit::card(
        &painter,
        r,
        &theme,
        kit::accent(&theme, 1),
        RowIcon::Kit(KitIcon::Commands),
        &t("batch-ui-template-title"),
        None,
    );
    let label = t("batch-ui-template-name");
    kit::text_left(
        &painter,
        inner.left(),
        inner.top() + 8.0,
        inner.width(),
        &label,
        theme.font(type_scale::BODY),
        kit::weak(&theme),
    );
    let fr = Rect::from_min_size(
        pos2(inner.left(), inner.top() + 16.0 + sp::XS),
        vec2(inner.width(), field_h),
    );
    let mut fc = lay.new_child(egui::UiBuilder::new().max_rect(fr));
    crate::theme::field_widgets(&mut fc);
    let fid = Id::new("fp-batch-template-name");
    let resp = fc.add_sized(
        fr.size(),
        egui::TextEdit::singleline(&mut pr.name)
            .id(fid)
            .desired_width(fr.width()),
    );
    resp.widget_info(|| super::params::text_info(&label));
    if pr.focus {
        resp.request_focus();
        pr.focus = false;
    }
    let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
    let esc = ui.input(|i| i.key_pressed(Key::Escape));
    let ok_l = t("batch-ui-template-save");
    let cancel_l = t("batch-ui-template-cancel");
    let ow = kit::btn_w(ui.ctx(), &ok_l, false);
    let cw = kit::btn_w(ui.ctx(), &cancel_l, false);
    let y = inner.bottom() - 26.0;
    let okr = Rect::from_min_size(pos2(inner.right() - ow, y), vec2(ow, 26.0));
    let cr = Rect::from_min_size(pos2(okr.left() - sp::S - cw, y), vec2(cw, 26.0));
    let valid = !pr.name.trim().is_empty();
    let cancel = kit::button(
        &mut lay,
        cr,
        Id::new("fp-batch-template-cancel"),
        &cancel_l,
        None,
        BtnKind::Secondary,
        true,
        &cancel_l,
    )
    .clicked();
    let ok = kit::button(
        &mut lay,
        okr,
        Id::new("fp-batch-template-ok"),
        &ok_l,
        None,
        BtnKind::Primary,
        valid,
        &ok_l,
    )
    .clicked();
    let _ = c;
    if cancel || esc {
        s.template_prompt = None;
    } else if (ok || enter) && valid {
        s.template_prompt = None;
        save_step_template(s, pr.step, &pr.name, now);
    } else {
        s.template_prompt = Some(pr);
    }
}
