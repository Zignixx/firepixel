//! Karte ② Rezept (SPEC-BATCH §2.4): Schrittkarten (zugeklappt 56 px: Griff, Symbolchip in
//! Gruppenfarbe, Titel, Zusammenfassung, Schalter, Menü ⋯, Pfeil), Akkordeon (höchstens eine
//! Karte offen), Ziehen zum Umsortieren mit Einfügelinie, „+ Schritt hinzufügen“ mit der
//! Schritt-Galerie als Überlagerung, feste Endkarten „Dateiname“ und „Speichern als“.

use super::kit::{self, BtnKind};
use super::params::{self, CardCtx};
use super::{CardId, Mode, Studio};
use crate::app_state::AppState;
use crate::theme::{Theme, mix, with_alpha};
use crate::tips::HoverTip;
use crate::ui_kit::{self, KitIcon, RowIcon, sp, type_scale};
use egui::{Align2, CornerRadius, Id, Rect, Sense, Shape, Stroke, StrokeKind, Ui, pos2, vec2};
use fp_batch::output::OutputFormat;
use fp_batch::recipe::{CaseMode, RenameMode, RenameSpec};
use fp_batch::step::{ParamDef, StepKind, step_label, with_defaults};
use fp_batch::{ItemInfo, Recipe, Severity, StepGroup, StepRegistry};
use fp_i18n::t;
use serde_json::{Value, json};

/// Höhe einer zugeklappten Schrittkarte.
pub const CARD_H: f32 = 56.0;
/// Höhe einer Kachel der Galerie.
pub const TILE_H: f32 = 54.0;

/// Zustand der Schritt-Galerie.
#[derive(Clone, Debug, Default)]
pub struct Gallery {
    pub query: String,
    /// Markierter Eintrag (Index in der gefilterten Liste).
    pub sel: usize,
    pub(crate) focus: bool,
}

/// Ziehen einer Schrittkarte.
#[derive(Clone, Copy, Debug)]
pub struct DragState {
    pub from: usize,
    pub pointer_y: f32,
}

enum Act {
    Toggle(usize),
    Open(CardId),
    Remove(usize),
    Duplicate(usize),
    Move(usize, usize),
    PickPath(usize, String),
    SaveTemplate(usize),
    Help(usize),
}

/// Erscheint ein Schritt im Anfänger-Modus? (mindestens ein Parameter mit `beginner()` oder gar keine Parameter)
pub fn is_beginner_step(step: &dyn fp_batch::Step) -> bool {
    let p = step.params();
    p.is_empty() || p.iter().any(|d| d.beginner)
}

/// Schritte der Galerie (ohne Endkarten), nach Gruppe und Name, gefiltert nach Suche und Modus.
pub fn gallery_steps(query: &str, mode: Mode) -> Vec<(StepGroup, String, String)> {
    let reg = StepRegistry::global();
    let q = query.trim().to_lowercase();
    let mut v: Vec<(StepGroup, String, String)> = reg
        .iter()
        .filter(|s| s.kind() != StepKind::Final)
        .filter(|s| mode == Mode::Pro || is_beginner_step(s.as_ref()))
        .map(|s| (s.group(), s.id().to_string(), step_label(s.id())))
        .filter(|(_, id, name)| {
            if q.is_empty() {
                return true;
            }
            let desc = description(id).to_lowercase();
            name.to_lowercase().contains(&q)
                || desc.contains(&q)
                || (mode == Mode::Pro && id.to_lowercase().contains(&q))
        })
        .collect();
    v.sort_by_key(|a| (a.0, a.2.to_lowercase()));
    v
}

/// Erklärung eines Schritts (`batch-step-<id>.description`).
pub fn description(id: &str) -> String {
    let k = format!("{}.description", fp_batch::step::step_key(id));
    if fp_i18n::has(&k) {
        t(&k)
    } else {
        String::new()
    }
}

/// Fügt einen Schritt hinzu (Galerie): eine Karte, offen, ein Rückgängig-Schritt.
pub fn add_step(s: &mut Studio, id: &str) {
    s.end_gesture();
    let mut idx = 0;
    if s.edit("add", |r| {
        idx = r.add_step(id, json!({}));
        true
    }) {
        s.open_card = Some(CardId::Step(idx));
        s.show_more = false;
    }
    s.end_gesture();
    s.gallery = None;
}

/// Zeichnet die Karte.
pub fn show(ui: &mut Ui, st: &mut AppState, s: &mut Studio, rect: Rect, now: f64, running: bool) {
    let theme = Theme::current(ui.ctx());
    let col = kit::accent(&theme, 1);
    let p = ui.painter().clone();
    let n = s.recipe.steps.len();
    let count = fp_i18n::tr!("batch-ui-recipe-steps", n = n);
    let inner = kit::card(
        &p,
        rect,
        &theme,
        col,
        RowIcon::Kit(KitIcon::Commands),
        &t("batch-ui-recipe-title"),
        Some(&count),
    );
    if inner.height() < 10.0 {
        return;
    }
    if s.gallery.is_some() && !running {
        gallery(ui, s, inner);
        return;
    }
    let _ = now;
    let reg = StepRegistry::global();
    let issues = s.issues();
    let vis = s.visible();
    let files: Vec<ItemInfo> = vis
        .iter()
        .take(200)
        .map(|&i| s.files[i].info.clone())
        .collect();
    let enabled = !running;
    let mut acts: Vec<Act> = Vec::new();
    let mut card_rects: Vec<Rect> = Vec::new();
    let app: &AppState = st; // BATCH-11a: Farbwähler des Originals im Formular
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    child.set_clip_rect(inner.intersect(ui.clip_rect()));
    egui::ScrollArea::vertical()
        .id_salt("fp-batch-recipe-scroll")
        .auto_shrink([false, false])
        .show(&mut child, |ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, sp::S);
            let w = ui.available_width();
            if n == 0 {
                let title = t("batch-ui-recipe-empty");
                let hint = t("batch-ui-recipe-empty-hint");
                let h = ui_kit::empty_state_height(ui.ctx(), &title, Some(&hint), w);
                let (r, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
                ui_kit::empty_state_at(
                    ui.painter(),
                    r,
                    KitIcon::EmptyTray,
                    &title,
                    Some(&hint),
                    &theme,
                );
            }
            for i in 0..n {
                let r = step_card(ui, s, i, w, &issues, &files, enabled, &mut acts, &reg, app);
                card_rects.push(r);
            }
            // „+ Schritt hinzufügen“
            let (ar, _) = ui.allocate_exact_size(vec2(w, 36.0), Sense::hover());
            let add_l = t("batch-ui-add-step");
            let resp = kit::button(
                ui,
                ar,
                Id::new("fp-batch-add-step"),
                &add_l,
                Some(RowIcon::Icon(crate::icons::Icon::Plus)),
                BtnKind::Ghost,
                enabled,
                &add_l,
            )
            .hover_tip(t("batch-ui-add-step.tooltip"));
            kit::dashed_rect(
                ui.painter(),
                ar,
                if resp.hovered() {
                    theme.colors.focus
                } else {
                    mix(theme.colors.line_mid, kit::card_fill(&theme), 0.3)
                },
            );
            if resp.clicked() {
                s.gallery = Some(Gallery {
                    focus: true,
                    ..Gallery::default()
                });
            }
            // Endkarten
            ui.add_space(sp::XS);
            let (cap, _) = ui.allocate_exact_size(vec2(w, 18.0), Sense::hover());
            ui_kit::section_at(ui.painter(), cap, &t("batch-ui-end-section"), &theme);
            rename_card(ui, s, w, &files, enabled, app);
            output_card(ui, s, w, &files, enabled, app);
        });
    // Einfügelinie beim Ziehen
    if let Some(d) = s.drag {
        let target = insert_index(&card_rects, d.pointer_y, d.from);
        let y = if target >= card_rects.len() {
            card_rects.last().map(|r| r.bottom() + sp::S / 2.0)
        } else if target > d.from {
            card_rects.get(target).map(|r| r.bottom() + sp::S / 2.0)
        } else {
            card_rects.get(target).map(|r| r.top() - sp::S / 2.0)
        };
        if let Some(y) = y {
            ui.painter().with_clip_rect(inner).hline(
                inner.x_range(),
                y,
                Stroke::new(2.0, theme.colors.focus),
            );
        }
        if !ui.input(|i| i.pointer.any_down()) {
            s.drag = None;
            if target != d.from {
                acts.push(Act::Move(d.from, target.min(n.saturating_sub(1))));
            }
        }
    }
    for a in acts {
        apply(st, s, a, now);
    }
}

/// Namen der Parameter, die sich zwischen `a` und `b` unterscheiden (sortiert) – Schlüssel der Geste
/// für den Rezept-Verlauf: Schieber A ziehen, dann Feld B tippen = zwei Rückgängig-Schritte.
pub fn changed_names(a: &Value, b: &Value) -> Vec<String> {
    let empty = serde_json::Map::new();
    let (ao, bo) = (
        a.as_object().unwrap_or(&empty),
        b.as_object().unwrap_or(&empty),
    );
    let mut v: Vec<String> = ao
        .keys()
        .chain(bo.keys())
        .filter(|k| ao.get(*k) != bo.get(*k))
        .cloned()
        .collect();
    v.sort();
    v.dedup();
    v
}

/// Einfügeposition beim Ziehen (Index nach dem Verschieben).
pub fn insert_index(rects: &[Rect], y: f32, from: usize) -> usize {
    let mut to = from;
    for (i, r) in rects.iter().enumerate() {
        if i < from && y < r.center().y {
            return i;
        }
        if i > from && y > r.center().y {
            to = i;
        }
    }
    to
}

fn apply(st: &mut AppState, s: &mut Studio, a: Act, now: f64) {
    s.end_gesture();
    match a {
        Act::Toggle(i) => {
            s.edit("toggle", |r| {
                r.steps[i].enabled = !r.steps[i].enabled;
                true
            });
        }
        Act::Open(c) => {
            s.open_card = if s.open_card == Some(c) {
                None
            } else {
                Some(c)
            };
            s.show_more = false;
        }
        Act::Remove(i) => {
            s.edit("remove", |r| r.remove_step(i).is_some());
            s.open_card = match s.open_card {
                Some(CardId::Step(j)) if j == i => None,
                Some(CardId::Step(j)) if j > i => Some(CardId::Step(j - 1)),
                o => o,
            };
        }
        Act::Duplicate(i) => {
            let copy = s.recipe.steps[i].clone();
            s.edit("duplicate", |r| {
                r.steps.insert(i + 1, copy);
                true
            });
            s.open_card = Some(CardId::Step(i + 1));
        }
        Act::Move(from, to) => {
            if s.edit("move", |r| r.move_step(from, to))
                && matches!(s.open_card, Some(CardId::Step(j)) if j == from)
            {
                s.open_card = Some(CardId::Step(to));
            }
        }
        Act::SaveTemplate(i) => super::step_menu::ask_template_name(s, i),
        Act::Help(i) => super::step_menu::open_help(st, s, i, now),
        Act::PickPath(i, param) => {
            let inbox = s.inbox_sender();
            crate::files::pick_one(
                st,
                &t("batch-ui-choose-file"),
                &t("ui-filter-all-files"),
                &["*"],
                move |_st, path| {
                    inbox
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .push(super::Inbox::ParamPath {
                            step: i,
                            param,
                            path,
                        });
                },
            );
        }
    }
    s.end_gesture();
}

/// Kopf einer Karte (Schritt oder Endkarte). Liefert (Kopf-Antwort, Rechteck des Kopfes).
#[allow(clippy::too_many_arguments)]
fn card_head(
    ui: &mut Ui,
    r: Rect,
    id: Id,
    col: egui::Color32,
    icon: RowIcon,
    title: &str,
    summary: &str,
    open: bool,
    warn: Option<&str>,
    dim: bool,
) -> egui::Response {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let p = ui.painter().clone();
    let resp = ui.interact(r, id, Sense::click());
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, open, title));
    let chip = Rect::from_min_size(pos2(r.left() + 26.0, r.center().y - 14.0), vec2(28.0, 28.0));
    p.rect_filled(
        chip,
        CornerRadius::same(7),
        if theme.glow {
            with_alpha(col, 40)
        } else {
            kit::tint(&theme, col, 1.3)
        },
    );
    kit::paint_icon(
        &p,
        Rect::from_center_size(chip.center(), vec2(18.0, 18.0)),
        icon,
        &theme,
        !dim,
    );
    let x = chip.right() + sp::M - 2.0;
    let right = r.right() - 84.0;
    let tw = (right - x).max(20.0);
    let title_col = if dim {
        c.text_disabled
    } else if theme.glow && (open || resp.hovered()) {
        col
    } else {
        c.text
    };
    let mut tx = x;
    if let Some(w) = warn {
        let ir = Rect::from_min_size(pos2(x, r.center().y - 17.0), vec2(14.0, 14.0));
        ui_kit::paint_icon(
            &p,
            ir,
            KitIcon::Warning,
            &crate::icons::IconStyle::new(&theme, None, true),
        );
        tx += 18.0;
        let _ = w;
    }
    kit::text_left(
        &p,
        tx,
        r.center().y - 9.0,
        tw - (tx - x),
        title,
        theme.bold(type_scale::SECTION),
        title_col,
    );
    if !summary.is_empty() {
        kit::text_left(
            &p,
            x,
            r.center().y + 9.0,
            tw,
            summary,
            theme.font(type_scale::CAPTION),
            if dim {
                c.text_disabled
            } else {
                kit::weak(&theme)
            },
        );
    }
    resp
}

#[allow(clippy::too_many_arguments)]
fn step_card(
    ui: &mut Ui,
    s: &mut Studio,
    i: usize,
    w: f32,
    issues: &[fp_batch::Issue],
    files: &[ItemInfo],
    enabled: bool,
    acts: &mut Vec<Act>,
    reg: &StepRegistry,
    app: &AppState,
) -> Rect {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let cfg = s.recipe.steps[i].clone();
    let step = reg.get(&cfg.id).cloned();
    let group = step
        .as_ref()
        .map(|s| s.group())
        .unwrap_or(StepGroup::Advanced);
    let col = kit::group_color(&theme, group);
    let open = s.open_card == Some(CardId::Step(i));
    let bg = ui.painter().add(Shape::Noop);
    let (head, _) = ui.allocate_exact_size(vec2(w, CARD_H), Sense::hover());
    let defs: Vec<ParamDef> = step.as_ref().map(|s| s.params()).unwrap_or_default();
    let (title, summary) = match &step {
        Some(stp) => {
            let full = with_defaults(&defs, &cfg.params);
            let sum = stp.summary(&full);
            let sum = if sum.is_empty() {
                params::auto_summary(&cfg.id, &defs, &cfg.params, s.mode)
            } else {
                sum
            };
            (step_label(&cfg.id), sum)
        }
        None => (
            fp_i18n::tr!("batch-ui-step-unknown", id = cfg.id.clone()),
            t("batch-ui-step-unknown-hint"),
        ),
    };
    let my_issues: Vec<&fp_batch::Issue> = issues.iter().filter(|x| x.step == Some(i)).collect();
    // „Video lesen“ ohne ffmpeg: Hinweis im Kopf und in der Karte (BATCH-11b)
    let now_s = ui.input(|i| i.time);
    let ffmpeg_missing =
        cfg.id == fp_batch::steps::anim::VIDEO_IMPORT && cfg.enabled && !s.ffmpeg_available(now_s);
    let warn = my_issues
        .iter()
        .find(|x| x.severity == Severity::Error)
        .or(my_issues.first())
        .map(|x| x.message.clone())
        .or_else(|| ffmpeg_missing.then(|| t("batch-ui-ffmpeg-missing-row")));
    let dim = !cfg.enabled || step.is_none();
    // Kopf: Klick klappt auf/zu
    let hresp = card_head(
        ui,
        head,
        Id::new(("fp-batch-step", i)),
        col,
        kit::group_icon(group),
        &title,
        &summary,
        open,
        warn.as_deref(),
        dim,
    );
    let hresp = match &warn {
        Some(m) => hresp.hover_tip(m.as_str()),
        None => hresp,
    };
    if hresp.clicked() && enabled {
        acts.push(Act::Open(CardId::Step(i)));
    }
    // Griff ⠿
    let gr = Rect::from_min_size(
        pos2(head.left() + 6.0, head.top()),
        vec2(16.0, head.height()),
    );
    let gresp = ui.interact(
        gr,
        Id::new(("fp-batch-step-grip", i)),
        if enabled {
            Sense::drag()
        } else {
            Sense::hover()
        },
    );
    gresp.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Other, enabled, t("batch-ui-step-drag"))
    });
    if gresp.hovered() || gresp.dragged() {
        ui.ctx().set_cursor_icon(if gresp.dragged() {
            egui::CursorIcon::Grabbing
        } else {
            egui::CursorIcon::Grab
        });
    }
    kit::grip(
        ui.painter(),
        gr.center(),
        if gresp.hovered() { c.text } else { c.text_weak },
    );
    if gresp.dragged()
        && let Some(pp) = ui.input(|inp| inp.pointer.interact_pos())
    {
        s.drag = Some(DragState {
            from: i,
            pointer_y: pp.y,
        });
    }
    // rechts: Pfeil, Menü ⋯, Schalter
    let chev = Rect::from_center_size(pos2(head.right() - 14.0, head.center().y), vec2(20.0, 20.0));
    kit::chevron(ui.painter(), chev.center(), open, c.text_weak);
    let dots_r =
        Rect::from_center_size(pos2(head.right() - 38.0, head.center().y), vec2(22.0, 22.0));
    let menu_name = t("batch-ui-step-menu");
    let dresp = kit::button(
        ui,
        dots_r,
        Id::new(("fp-batch-step-menu", i)),
        "",
        None,
        BtnKind::Ghost,
        enabled,
        &menu_name,
    );
    kit::dots(ui.painter(), dots_r.center(), c.text);
    let n = s.recipe.steps.len();
    egui::Popup::menu(&dresp)
        .id(Id::new(("fp-batch-step-menu-pop", i)))
        .show(|ui| {
            crate::theme::field_widgets(ui);
            if ui.button(t("batch-ui-step-duplicate")).clicked() {
                acts.push(Act::Duplicate(i));
            }
            if ui
                .add_enabled(i > 0, egui::Button::new(t("batch-ui-step-up")))
                .clicked()
            {
                acts.push(Act::Move(i, i - 1));
            }
            if ui
                .add_enabled(i + 1 < n, egui::Button::new(t("batch-ui-step-down")))
                .clicked()
            {
                acts.push(Act::Move(i, i + 1));
            }
            ui.separator();
            if ui.button(t("batch-ui-step-save-template")).clicked() {
                acts.push(Act::SaveTemplate(i));
            }
            if ui.button(t("batch-ui-step-help")).clicked() {
                acts.push(Act::Help(i));
            }
            ui.separator();
            if ui.button(t("batch-ui-step-remove")).clicked() {
                acts.push(Act::Remove(i));
            }
        });
    let tg = Rect::from_center_size(pos2(head.right() - 66.0, head.center().y), vec2(32.0, 20.0));
    if step.is_some() {
        let tresp = kit::toggle(
            ui,
            tg,
            Id::new(("fp-batch-step-on", i)),
            cfg.enabled,
            &t("batch-ui-step-enabled"),
        );
        if tresp.clicked() && enabled {
            acts.push(Act::Toggle(i));
        }
    }
    // Körper
    let mut bottom = head.bottom();
    if open {
        let body_top = head.bottom();
        let x0 = head.left() + 26.0;
        let bw = (head.right() - sp::M - x0).max(60.0);
        let mut params_v = cfg.params.clone();
        let mut show_more = s.show_more;
        let recipe_snapshot = s.recipe.clone();
        let mut pick: Option<String> = None;
        let mut changed = false;
        let mut open_settings = false;
        let resp = ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(Rect::from_min_size(pos2(x0, body_top), vec2(bw, 4000.0)))
                .layout(egui::Layout::top_down(egui::Align::Min)),
            |ui| {
                ui.spacing_mut().item_spacing = vec2(sp::S, sp::XS + 2.0);
                ui.set_width(bw);
                if step.is_none() {
                    // unbekannter Schritt: Parameter bleiben erhalten (Profi sieht sie als JSON)
                    if s.mode == Mode::Pro {
                        let txt = serde_json::to_string_pretty(&cfg.params).unwrap_or_default();
                        ui.label(
                            egui::RichText::new(txt)
                                .monospace()
                                .size(type_scale::CAPTION),
                        );
                    }
                    return;
                }
                let mut ctx = CardCtx {
                    step_id: &cfg.id,
                    index: Some(i),
                    params: &mut params_v,
                    defs: &defs,
                    mode: s.mode,
                    show_more: &mut show_more,
                    files,
                    recipe: &recipe_snapshot,
                    enabled,
                    pick_path: None,
                    app: Some(app),
                };
                changed = match params::card_body(&cfg.id) {
                    Some(f) => f(ui, &mut ctx),
                    None => params::generic_body(ui, &mut ctx),
                };
                pick = ctx.pick_path.take();
                // Hinweise des Schritts (Validierung)
                for iss in &my_issues {
                    let tone = match iss.severity {
                        Severity::Error => ui_kit::Tone::Error,
                        Severity::Warning => ui_kit::Tone::Warning,
                        Severity::Info => ui_kit::Tone::Info,
                    };
                    if iss.param.is_none() {
                        ui_kit::notice(ui, tone, None, &iss.message);
                    }
                }
                if ffmpeg_missing {
                    ui.add_space(sp::XS);
                    ui_kit::notice(
                        ui,
                        ui_kit::Tone::Warning,
                        None,
                        &t("batch-ui-ffmpeg-missing-card"),
                    );
                    let l = t("batch-ui-ffmpeg-choose");
                    let bw = kit::btn_w(ui.ctx(), &l, true);
                    let (br, _) = ui.allocate_exact_size(vec2(bw, kit::BTN_H), Sense::hover());
                    if kit::button(
                        ui,
                        br,
                        Id::new(("fp-batch-step-ffmpeg", i)),
                        &l,
                        Some(RowIcon::Icon(crate::icons::Icon::Folder)),
                        BtnKind::Secondary,
                        enabled,
                        &l,
                    )
                    .hover_tip(t("batch-ui-ffmpeg-choose.tooltip"))
                    .clicked()
                    {
                        open_settings = true;
                    }
                }
                ui.add_space(sp::S);
            },
        );
        bottom = resp.response.rect.bottom().max(body_top + sp::S);
        s.show_more = show_more;
        if changed && enabled {
            // ein Rückgängig-Schritt je Geste und Parameter (BATCH-11a)
            let key = format!(
                "step{i}:{}",
                changed_names(&cfg.params, &params_v).join(",")
            );
            s.edit(&key, |r| {
                if let Some(st) = r.steps.get_mut(i) {
                    st.params = params_v;
                }
                true
            });
        }
        if let Some(pn) = pick {
            acts.push(Act::PickPath(i, pn));
        }
        if open_settings {
            super::settings::open(ui.ctx(), s);
        }
    }
    let card = Rect::from_min_max(head.min, pos2(head.right(), bottom));
    ui.advance_cursor_after_rect(card);
    // Hintergrund und Leitstreifen (unter den Inhalt)
    let cr = CornerRadius::same(7);
    let hovered = ui.rect_contains_pointer(card);
    let fill = if open && !theme.glow {
        kit::tint(&theme, col, 0.35)
    } else {
        kit::item_fill(&theme)
    };
    let border = if open || hovered {
        if theme.glow {
            with_alpha(col, 200)
        } else {
            mix(kit::card_fill(&theme), col, 0.55)
        }
    } else {
        kit::item_border(&theme, col)
    };
    let dragging = s.drag.is_some_and(|d| d.from == i);
    let mut shapes = vec![
        Shape::rect_filled(
            card,
            cr,
            if dragging {
                with_alpha(fill, 160)
            } else {
                fill
            },
        ),
        Shape::rect_stroke(card, cr, Stroke::new(1.0, border), StrokeKind::Inside),
    ];
    let stripe = Rect::from_min_size(
        card.min + vec2(0.0, 6.0),
        vec2(3.0, (card.height() - 12.0).max(4.0)),
    );
    shapes.push(Shape::rect_filled(
        stripe,
        CornerRadius::same(2),
        if dim {
            mix(col, c.text_disabled, 0.6)
        } else {
            col
        },
    ));
    ui.painter().set(bg, Shape::Vec(shapes));
    card
}

/// Beispiel für die Endkarte „Dateiname“: (alt, neu) für die erste Datei der Liste.
pub fn example_name(recipe: &Recipe, info: Option<&ItemInfo>) -> Option<(String, String)> {
    let info = info?;
    let reg = StepRegistry::global();
    let file_only = fp_batch::pipeline::is_file_only(recipe, &reg);
    let fmt = OutputFormat::parse(&recipe.output.format).unwrap_or(OutputFormat::Same);
    let resolved = if file_only {
        OutputFormat::Same
    } else {
        fp_batch::output::resolve_format(fmt, &info.format).0
    };
    let out_ext =
        fp_batch::naming::target_extension(&recipe.rename, resolved, &info.file_name(), file_only);
    let ctx = fp_batch::naming::NameContext {
        info,
        list_index: 0,
        folder_index: 0,
        now: fp_batch::naming::now_local(),
        out_ext,
        out_format: resolved.name().to_string(),
    };
    let r = fp_batch::naming::build_name(&recipe.rename, &ctx).ok()?;
    Some((info.file_name(), r.file_name))
}

#[allow(clippy::too_many_arguments)]
fn end_head(
    ui: &mut Ui,
    s: &mut Studio,
    w: f32,
    card: CardId,
    icon: RowIcon,
    title: &str,
    summary: &str,
    enabled: bool,
) -> (Rect, bool, egui::layers::ShapeIdx) {
    let theme = Theme::current(ui.ctx());
    let bg = ui.painter().add(Shape::Noop);
    let (head, _) = ui.allocate_exact_size(vec2(w, CARD_H), Sense::hover());
    let open = s.open_card == Some(card);
    let col = kit::group_color(&theme, StepGroup::Format);
    let resp = card_head(
        ui,
        head,
        Id::new(("fp-batch-end", format!("{card:?}"))),
        col,
        icon,
        title,
        summary,
        open,
        None,
        false,
    );
    kit::chevron(
        ui.painter(),
        pos2(head.right() - 14.0, head.center().y),
        open,
        theme.colors.text_weak,
    );
    // Schloss-Hinweis: feste Endkarte
    let lr = Rect::from_center_size(pos2(head.right() - 38.0, head.center().y), vec2(14.0, 14.0));
    crate::icons::paint(
        ui.painter(),
        lr,
        crate::icons::Icon::Lock,
        &crate::icons::IconStyle::mono(kit::weak(&theme), kit::card_fill(&theme)),
    );
    let lresp = ui.interact(
        lr,
        Id::new(("fp-batch-end-lock", format!("{card:?}"))),
        Sense::hover(),
    );
    lresp.hover_tip(t("batch-ui-end-fixed"));
    if resp.clicked() && enabled {
        s.open_card = if open { None } else { Some(card) };
    }
    (head, open, bg)
}

fn end_finish(ui: &mut Ui, head: Rect, bottom: f32, bg: egui::layers::ShapeIdx, open: bool) {
    let theme = Theme::current(ui.ctx());
    let col = kit::group_color(&theme, StepGroup::Format);
    let card = Rect::from_min_max(head.min, pos2(head.right(), bottom.max(head.bottom())));
    ui.advance_cursor_after_rect(card);
    let cr = CornerRadius::same(7);
    let hovered = ui.rect_contains_pointer(card);
    let fill = if open && !theme.glow {
        kit::tint(&theme, col, 0.35)
    } else {
        kit::item_fill(&theme)
    };
    let border = if open || hovered {
        if theme.glow {
            with_alpha(col, 200)
        } else {
            mix(kit::card_fill(&theme), col, 0.55)
        }
    } else {
        kit::item_border(&theme, col)
    };
    let stripe = Rect::from_min_size(
        card.min + vec2(0.0, 6.0),
        vec2(3.0, (card.height() - 12.0).max(4.0)),
    );
    ui.painter().set(
        bg,
        Shape::Vec(vec![
            Shape::rect_filled(card, cr, fill),
            Shape::rect_stroke(card, cr, Stroke::new(1.0, border), StrokeKind::Inside),
            Shape::rect_filled(stripe, CornerRadius::same(2), col),
        ]),
    );
}

fn rename_card(
    ui: &mut Ui,
    s: &mut Studio,
    w: f32,
    files: &[ItemInfo],
    enabled: bool,
    app: &AppState,
) {
    let ex = example_name(&s.recipe, files.first());
    let summary = match &ex {
        Some((a, b)) if a == b => fp_i18n::tr!("batch-ui-rename-same", name = b.clone()),
        Some((a, b)) => format!("{a} → {b}"),
        None => t("batch-ui-rename-hint"),
    };
    let (head, open, bg) = end_head(
        ui,
        s,
        w,
        CardId::Rename,
        RowIcon::Icon(crate::icons::Icon::Text),
        &t("batch-ui-end-rename"),
        &summary,
        enabled,
    );
    let mut bottom = head.bottom();
    if open {
        let x0 = head.left() + 26.0;
        let bw = (head.right() - sp::M - x0).max(60.0);
        let mut spec = s.recipe.rename.clone();
        let snapshot = s.recipe.clone();
        let mode = s.mode;
        let mut show_more = s.show_more;
        let resp = ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(Rect::from_min_size(
                    pos2(x0, head.bottom()),
                    vec2(bw, 4000.0),
                ))
                .layout(egui::Layout::top_down(egui::Align::Min)),
            |ui| {
                ui.spacing_mut().item_spacing = vec2(sp::S, sp::XS + 2.0);
                ui.set_width(bw);
                let changed = match params::card_body(fp_batch::steps::files::RENAME) {
                    Some(f) => {
                        let mut v = serde_json::to_value(&spec).unwrap_or(Value::Null);
                        let mut ctx = CardCtx {
                            step_id: fp_batch::steps::files::RENAME,
                            index: None,
                            params: &mut v,
                            defs: &[],
                            mode,
                            show_more: &mut show_more,
                            files,
                            recipe: &snapshot,
                            enabled,
                            pick_path: None,
                            app: Some(app),
                        };
                        let ch = f(ui, &mut ctx);
                        if ch && let Ok(n) = serde_json::from_value::<RenameSpec>(v) {
                            spec = n;
                        }
                        ch
                    }
                    None => quick_rename(ui, &mut spec, mode, enabled),
                };
                if params::card_body(fp_batch::steps::files::RENAME).is_none()
                    && let Some((a, b)) = example_name(
                        &Recipe {
                            rename: spec.clone(),
                            ..snapshot.clone()
                        },
                        files.first(),
                    )
                {
                    ui.label(
                        egui::RichText::new(format!("{a} → {b}"))
                            .size(type_scale::CAPTION)
                            .color(kit::weak(&Theme::current(ui.ctx()))),
                    );
                }
                ui.add_space(sp::S);
                changed
            },
        );
        s.show_more = show_more;
        bottom = resp.response.rect.bottom();
        if resp.inner && enabled {
            s.edit("rename", |r| {
                r.rename = spec;
                true
            });
        }
    }
    end_finish(ui, head, bottom, bg, open);
}

/// Schnelles Umbenennen (bis BATCH-3 die volle Karte anmeldet): Präfix, Suffix, Schreibweise,
/// Leerzeichen; Profi zusätzlich regulärer Ausdruck.
fn quick_rename(ui: &mut Ui, spec: &mut RenameSpec, mode: Mode, enabled: bool) -> bool {
    let theme = Theme::current(ui.ctx());
    let mut changed = false;
    let labels = [
        t("batch-ui-rename-prefix"),
        t("batch-ui-rename-suffix"),
        t("batch-ui-rename-case"),
        t("batch-ui-rename-spaces"),
        t("batch-ui-rename-find"),
        t("batch-ui-rename-replace"),
    ];
    let refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
    let lw = ui_kit::label_width(ui.ctx(), &refs, 70.0).min(ui.available_width() * 0.42);
    let row = |ui: &mut Ui, label: &str, add: &mut dyn FnMut(&mut Ui, f32) -> bool| -> bool {
        ui.horizontal(|ui| {
            let (lr, _) = ui.allocate_exact_size(vec2(lw, 22.0), Sense::hover());
            let shown = ui_kit::elide(ui.ctx(), label, &theme.font(type_scale::BODY), lw - sp::XS);
            ui.painter().text(
                lr.left_center(),
                Align2::LEFT_CENTER,
                shown,
                theme.font(type_scale::BODY),
                kit::weak(&theme),
            );
            let w = (ui.available_width() - 2.0).max(40.0);
            ui.add_enabled_ui(enabled, |ui| {
                crate::theme::field_widgets(ui);
                add(ui, w)
            })
            .inner
        })
        .inner
    };
    if mode == Mode::Pro {
        let modes = vec![
            t("batch-ui-rename-mode-builder"),
            t("batch-ui-rename-mode-regex"),
        ];
        let cur = if spec.mode == RenameMode::Regex { 1 } else { 0 };
        let (r, _) =
            ui.allocate_exact_size(vec2(ui.available_width().min(280.0), 24.0), Sense::hover());
        if let Some(k) = kit::segmented(ui, r, Id::new("fp-batch-rename-mode"), &modes, cur, false)
        {
            spec.mode = if k == 1 {
                RenameMode::Regex
            } else {
                RenameMode::Builder
            };
            changed = true;
        }
    }
    if spec.mode == RenameMode::Regex && mode == Mode::Pro {
        changed |= row(ui, &labels[4], &mut |ui, w| {
            ui.add(
                egui::TextEdit::singleline(&mut spec.regex.find)
                    .desired_width(w)
                    .code_editor(),
            )
            .changed()
        });
        changed |= row(ui, &labels[5], &mut |ui, w| {
            ui.add(
                egui::TextEdit::singleline(&mut spec.regex.replace)
                    .desired_width(w)
                    .code_editor(),
            )
            .changed()
        });
        if let Err(e) = fp_batch::naming::compile_regex(&spec.regex) {
            ui.label(
                egui::RichText::new(e.to_string())
                    .color(theme.colors.icon_red)
                    .size(type_scale::CAPTION),
            );
        }
        return changed;
    }
    changed |= row(ui, &labels[0], &mut |ui, w| {
        ui.add(egui::TextEdit::singleline(&mut spec.prefix).desired_width(w))
            .changed()
    });
    changed |= row(ui, &labels[1], &mut |ui, w| {
        ui.add(egui::TextEdit::singleline(&mut spec.suffix).desired_width(w))
            .changed()
    });
    let cases = [
        (CaseMode::Keep, "batch-ui-case-keep"),
        (CaseMode::Lower, "batch-ui-case-lower"),
        (CaseMode::Upper, "batch-ui-case-upper"),
        (CaseMode::Sentence, "batch-ui-case-sentence"),
        (CaseMode::Title, "batch-ui-case-title"),
        (CaseMode::Kebab, "batch-ui-case-kebab"),
        (CaseMode::Snake, "batch-ui-case-snake"),
        (CaseMode::Camel, "batch-ui-case-camel"),
    ];
    changed |= row(ui, &labels[2], &mut |ui, w| {
        let before = spec.case;
        let cur = cases
            .iter()
            .find(|c| c.0 == spec.case)
            .map(|c| t(c.1))
            .unwrap_or_default();
        egui::ComboBox::from_id_salt("fp-batch-rename-case")
            .selected_text(cur)
            .width((w - 8.0).min(220.0))
            .show_ui(ui, |ui| {
                for (cm, k) in cases {
                    if mode == Mode::Beginner
                        && matches!(cm, CaseMode::Kebab | CaseMode::Snake | CaseMode::Camel)
                    {
                        continue;
                    }
                    ui.selectable_value(&mut spec.case, cm, t(k));
                }
            });
        spec.case != before
    });
    changed |= row(ui, &labels[3], &mut |ui, _w| {
        let mut on = spec.spaces.is_some();
        let r = ui.checkbox(&mut on, "");
        if r.changed() {
            spec.spaces = if on { Some("_".to_string()) } else { None };
            true
        } else {
            false
        }
    });
    changed
}

/// Schema der Endkarte „Speichern als“ (Felder je Format über `when("format", …)`, BATCH-11b).
pub fn output_defs() -> Vec<ParamDef> {
    StepRegistry::global()
        .get(fp_batch::steps::files::OUTPUT)
        .map(|s| s.params())
        .unwrap_or_default()
}

/// Schema der GIF-Optionen (`output.gif`), bei Format GIF unter „Speichern als“.
pub fn gif_defs() -> Vec<ParamDef> {
    StepRegistry::global()
        .get(fp_batch::steps::gifopt::OUTPUT_GIF)
        .map(|s| s.params())
        .unwrap_or_default()
}

/// Qualität einer GIF-Voreinstellung (wie `GifOptions::base_quality`).
pub fn gif_preset_quality(preset: &str) -> i64 {
    match preset {
        "smallest" => 60,
        "best" => 100,
        _ => 80,
    }
}

/// Nach einem Formatwechsel: Optionen entfernen, die das neue Format so nicht annimmt (z. B.
/// Kompression „LZW“ von TIFF bei PNG). Werte ohne passendes Feld bleiben erhalten.
pub fn clean_output_options(v: &mut Value) {
    let mut defs = output_defs();
    if v.get("format").and_then(Value::as_str) == Some("gif") {
        defs.extend(gif_defs());
    }
    let snapshot = v.clone();
    let Some(o) = v.as_object_mut() else { return };
    o.retain(|k, val| {
        if k == "format" || val.is_null() {
            return true;
        }
        let visible: Vec<&ParamDef> = defs
            .iter()
            .filter(|d| d.name == *k && d.is_visible(&defs, &snapshot))
            .collect();
        visible.is_empty() || visible.iter().any(|d| d.check(val).is_none())
    });
}

/// Zusammenfassung „Speichern als“.
pub fn output_summary(recipe: &Recipe) -> String {
    let fmt = OutputFormat::parse(&recipe.output.format).unwrap_or(OutputFormat::Same);
    let name = if fmt == OutputFormat::Same {
        t("batch-ui-output-same")
    } else {
        fmt.display_name().to_string()
    };
    let q = recipe.output.options.get("quality").and_then(Value::as_i64);
    match (fmt, q) {
        (OutputFormat::Jpeg | OutputFormat::WebP | OutputFormat::Avif, Some(q)) => format!(
            "{name} · {}",
            fp_i18n::tr!("batch-ui-output-quality", q = q)
        ),
        (OutputFormat::Jpeg | OutputFormat::WebP | OutputFormat::Avif, None) => format!(
            "{name} · {}",
            fp_i18n::tr!(
                "batch-ui-output-quality",
                q = fp_batch::output::default_quality(fmt).unwrap_or(85)
            )
        ),
        _ => name,
    }
}

fn output_card(
    ui: &mut Ui,
    s: &mut Studio,
    w: f32,
    files: &[ItemInfo],
    enabled: bool,
    app: &AppState,
) {
    let summary = output_summary(&s.recipe);
    let (head, open, bg) = end_head(
        ui,
        s,
        w,
        CardId::Output,
        RowIcon::Icon(crate::icons::Icon::Save),
        &t("batch-ui-end-output"),
        &summary,
        enabled,
    );
    let mut bottom = head.bottom();
    if open {
        let x0 = head.left() + 26.0;
        let bw = (head.right() - sp::M - x0).max(60.0);
        let fmt = OutputFormat::parse(&s.recipe.output.format).unwrap_or(OutputFormat::Same);
        let defs = output_defs();
        let gdefs = if fmt == OutputFormat::Gif {
            gif_defs()
        } else {
            Vec::new()
        };
        let mut v = Value::Object(s.recipe.output.options.clone());
        v["format"] = json!(s.recipe.output.format);
        let snapshot = s.recipe.clone();
        let mode = s.mode;
        let mut show_more = s.show_more;
        let size_line = gif_size_line(s, fmt);
        let hints = output_hints(&s.recipe, fmt);
        let resp = ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(Rect::from_min_size(
                    pos2(x0, head.bottom()),
                    vec2(bw, 4000.0),
                ))
                .layout(egui::Layout::top_down(egui::Align::Min)),
            |ui| {
                ui.spacing_mut().item_spacing = vec2(sp::S, sp::XS + 2.0);
                ui.set_width(bw);
                if let Some(f) = params::card_body(fp_batch::steps::files::OUTPUT) {
                    let mut ctx = CardCtx {
                        step_id: fp_batch::steps::files::OUTPUT,
                        index: None,
                        params: &mut v,
                        defs: &defs,
                        mode,
                        show_more: &mut show_more,
                        files,
                        recipe: &snapshot,
                        enabled,
                        pick_path: None,
                        app: Some(app),
                    };
                    let ch = f(ui, &mut ctx);
                    ui.add_space(sp::S);
                    return ch;
                }
                // eigene Felder je Format (BATCH-11b): „Speichern als“ + bei GIF der GIF-Optimierer
                let cur = v.clone();
                let (main, hid_main) = params::form_defs(&defs, &cur, mode, show_more);
                let (gif, hid_gif) = params::form_defs(&gdefs, &cur, mode, show_more);
                let main: Vec<ParamDef> = main.into_iter().cloned().collect();
                let gif: Vec<ParamDef> = gif.into_iter().cloned().collect();
                let mut labels: Vec<String> = main
                    .iter()
                    .map(|d| params::param_label(fp_batch::steps::files::OUTPUT, &d.name))
                    .collect();
                labels
                    .extend(gif.iter().map(|d| {
                        params::param_label(fp_batch::steps::gifopt::OUTPUT_GIF, &d.name)
                    }));
                let label_w = params::label_column(ui, &labels);
                let mut ch;
                {
                    let mut ctx = CardCtx {
                        step_id: fp_batch::steps::files::OUTPUT,
                        index: None,
                        params: &mut v,
                        defs: &defs,
                        mode,
                        show_more: &mut show_more,
                        files,
                        recipe: &snapshot,
                        enabled,
                        pick_path: None,
                        app: Some(app),
                    };
                    ch = params::form_rows(ui, &mut ctx, &main, label_w);
                }
                if !gif.is_empty() {
                    section_caption(ui, &t("batch-ui-gif-section"));
                    let before_preset = v.get("preset").cloned();
                    let mut ctx = CardCtx {
                        step_id: fp_batch::steps::gifopt::OUTPUT_GIF,
                        index: None,
                        params: &mut v,
                        defs: &gdefs,
                        mode,
                        show_more: &mut show_more,
                        files,
                        recipe: &snapshot,
                        enabled,
                        pick_path: None,
                        app: Some(app),
                    };
                    ch |= params::form_rows(ui, &mut ctx, &gif, label_w);
                    // Voreinstellung gewählt → Qualität folgt ihr (der Schieber zeigt den wirksamen Wert)
                    let after_preset = v.get("preset").cloned();
                    if after_preset != before_preset {
                        let q = gif_preset_quality(
                            after_preset
                                .as_ref()
                                .and_then(Value::as_str)
                                .unwrap_or("balanced"),
                        );
                        ch |= params::set_param(&mut v, "quality", json!(q));
                    }
                    if let Some((tone, text)) = &size_line {
                        ui.add_space(sp::XS);
                        ui_kit::notice(ui, *tone, None, text);
                    }
                }
                for h in &hints {
                    ui.add_space(sp::XS);
                    ui_kit::notice(ui, ui_kit::Tone::Info, None, h);
                }
                let any_pro = defs.iter().chain(gdefs.iter()).any(|d| !d.beginner);
                params::more_toggle(ui, mode, &mut show_more, hid_main + hid_gif, any_pro);
                ui.add_space(sp::S);
                ch
            },
        );
        s.show_more = show_more;
        bottom = resp.response.rect.bottom();
        if resp.inner && enabled {
            let mut old = Value::Object(s.recipe.output.options.clone());
            old["format"] = json!(s.recipe.output.format);
            if old.get("format") != v.get("format") {
                clean_output_options(&mut v);
            }
            let names = changed_names(&old, &v);
            let key = format!("output:{}", names.join(","));
            s.edit(&key, |r| {
                if let Some(o) = v.as_object() {
                    let mut opts = serde_json::Map::new();
                    for (k, val) in o {
                        if k == "format" {
                            if let Some(f) = val.as_str() {
                                r.output.format = f.to_string();
                            }
                        } else {
                            opts.insert(k.clone(), val.clone());
                        }
                    }
                    // entfernte Werte (↺, Formatwechsel) gelten wieder als Vorgabe bzw. „Automatisch“
                    r.output.options = opts;
                }
                true
            });
        }
    }
    end_finish(ui, head, bottom, bg, open);
}

/// Abschnittsüberschrift innerhalb einer Karte (fett in `section_label`, feine Linie dahinter).
fn section_caption(ui: &mut Ui, text: &str) {
    let theme = Theme::current(ui.ctx());
    ui.add_space(sp::XS);
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 20.0), Sense::hover());
    let font = theme.bold(type_scale::CAPTION + 0.5);
    let shown = ui_kit::elide(ui.ctx(), text, &font, r.width());
    let tw = ui_kit::text_w(ui.ctx(), &shown, &font);
    ui.painter().text(
        r.left_center(),
        Align2::LEFT_CENTER,
        &shown,
        font,
        theme.colors.section_label,
    );
    let x = (r.left() + tw + sp::S).min(r.right());
    if x < r.right() {
        ui.painter().line_segment(
            [pos2(x, r.center().y), pos2(r.right(), r.center().y)],
            Stroke::new(1.0, mix(kit::card_fill(&theme), theme.colors.line_mid, 0.6)),
        );
    }
}

/// GIF: erreichte Größe der Vorschaudatei bzw. ⚠ Zielgröße nicht erreichbar.
fn gif_size_line(s: &Studio, fmt: OutputFormat) -> Option<(ui_kit::Tone, String)> {
    if fmt != OutputFormat::Gif {
        return None;
    }
    let max_kb = s
        .recipe
        .output
        .options
        .get("maxKB")
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
        .max(0.0) as u64;
    let shown = s.preview.shown.as_ref()?;
    let target = fp_i18n::fmt_size(max_kb * 1024);
    Some(match shown.bytes_after {
        None => (ui_kit::Tone::Info, t("batch-ui-gif-size-pending")),
        Some(b) if max_kb == 0 => (
            ui_kit::Tone::Info,
            fp_i18n::tr!("batch-ui-gif-size", size = fp_i18n::fmt_size(b)),
        ),
        Some(b) if b <= max_kb * 1024 => (
            ui_kit::Tone::Success,
            fp_i18n::tr!(
                "batch-ui-gif-reached",
                size = fp_i18n::fmt_size(b),
                target = target
            ),
        ),
        Some(b) => (
            ui_kit::Tone::Warning,
            fp_i18n::tr!(
                "batch-ui-gif-missed",
                size = fp_i18n::fmt_size(b),
                target = target
            ),
        ),
    })
}

/// Hinweise der Endkarte: „Nachzeichnen“ mit Pixelformat, JPEG füllt Transparenz.
pub fn output_hints(recipe: &Recipe, fmt: OutputFormat) -> Vec<String> {
    let mut out = Vec::new();
    let has = |id: &str| recipe.steps.iter().any(|st| st.enabled && st.id == id);
    if has("vector.trace") && !matches!(fmt, OutputFormat::Svg | OutputFormat::FwPng) {
        out.push(t("batch-ui-trace-pixel-hint"));
    }
    if fmt == OutputFormat::Jpeg && !has("background.flatten") {
        out.push(t("batch-ui-jpeg-white-hint"));
    }
    out
}

/// Schritt-Galerie als Überlagerung der Rezept-Karte: Suchfeld, Gruppen, Kacheln, Pfeiltasten.
fn gallery(ui: &mut Ui, s: &mut Studio, inner: Rect) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let Some(mut g) = s.gallery.clone() else {
        return;
    };
    let p = ui.painter().clone();
    // Kopf: Titel, Schließen
    let head = Rect::from_min_size(inner.min, vec2(inner.width(), 24.0));
    kit::text_left(
        &p,
        head.left(),
        head.center().y,
        head.width() - 30.0,
        &t("batch-ui-gallery-title"),
        theme.bold(type_scale::SECTION + 1.0),
        c.text,
    );
    let close_r =
        Rect::from_center_size(pos2(head.right() - 11.0, head.center().y), vec2(22.0, 22.0));
    let close_name = t("batch-ui-gallery-close");
    if kit::icon_button(
        ui,
        close_r,
        Id::new("fp-batch-gallery-close"),
        RowIcon::Icon(crate::icons::Icon::Close),
        &close_name,
        true,
    )
    .clicked()
    {
        s.gallery = None;
        return;
    }
    // Suche
    let sr = Rect::from_min_size(
        pos2(inner.left(), head.bottom() + sp::S),
        vec2(inner.width(), 24.0),
    );
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(sr));
    crate::theme::field_widgets(&mut child);
    let hint = t("batch-ui-gallery-search");
    let sresp = child.add_sized(
        sr.size(),
        egui::TextEdit::singleline(&mut g.query)
            .hint_text(hint.clone())
            .id(Id::new("fp-batch-gallery-search")),
    );
    sresp.widget_info(|| egui::WidgetInfo::text_edit(true, "", &hint, ""));
    if g.focus {
        sresp.request_focus();
        g.focus = false;
    }
    if sresp.changed() {
        g.sel = 0;
    }
    let list = gallery_steps(&g.query, s.mode);
    // Tasten im Suchfeld: ↑/↓ wählt, Eingabe fügt hinzu, Esc schließt
    let (down, up, enter, esc) = ui.input_mut(|i| {
        (
            i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
            i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
            i.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
            i.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
        )
    });
    if esc {
        s.gallery = None;
        return;
    }
    if down && !list.is_empty() {
        g.sel = (g.sel + 1).min(list.len() - 1);
    }
    if up {
        g.sel = g.sel.saturating_sub(1);
    }
    if enter && let Some((_, id, _)) = list.get(g.sel) {
        let id = id.clone();
        add_step(s, &id);
        return;
    }
    let lr = Rect::from_min_max(pos2(inner.left(), sr.bottom() + sp::S), inner.max);
    let mut lc = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(lr)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    lc.set_clip_rect(lr.intersect(ui.clip_rect()));
    let mut chosen: Option<String> = None;
    egui::ScrollArea::vertical()
        .id_salt("fp-batch-gallery-scroll")
        .auto_shrink([false, false])
        .show(&mut lc, |ui| {
            let w = ui.available_width();
            if list.is_empty() {
                let title = t("batch-ui-gallery-empty");
                let hint = t(if s.mode == Mode::Beginner {
                    "batch-ui-gallery-empty-hint"
                } else {
                    "batch-ui-gallery-empty-hint-pro"
                });
                let h = ui_kit::empty_state_height(ui.ctx(), &title, Some(&hint), w);
                let (r, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
                ui_kit::empty_state_at(
                    ui.painter(),
                    r,
                    KitIcon::EmptyTray,
                    &title,
                    Some(&hint),
                    &theme,
                );
                return;
            }
            ui.spacing_mut().item_spacing = vec2(0.0, sp::XS);
            let cols = if w >= 460.0 { 2 } else { 1 };
            let tile_w = ((w - (cols as f32 - 1.0) * sp::S) / cols as f32).floor();
            let mut k = 0usize;
            let mut start = 0usize;
            while start < list.len() {
                let grp = list[start].0;
                let end = list[start..]
                    .iter()
                    .position(|x| x.0 != grp)
                    .map(|p| start + p)
                    .unwrap_or(list.len());
                ui.add_space(sp::XS);
                let (cap, _) = ui.allocate_exact_size(vec2(w, 18.0), Sense::hover());
                ui_kit::section_at(ui.painter(), cap, &grp.label(), &theme);
                for chunk in list[start..end].chunks(cols) {
                    let (row, _) = ui.allocate_exact_size(vec2(w, TILE_H), Sense::hover());
                    for (j, (g2, id, name)) in chunk.iter().enumerate() {
                        let r = Rect::from_min_size(
                            pos2(row.left() + j as f32 * (tile_w + sp::S), row.top()),
                            vec2(tile_w, TILE_H),
                        );
                        let resp = tile(ui, r, id, name, *g2, k == g.sel, &theme);
                        if resp.clicked() {
                            chosen = Some(id.clone());
                        }
                        if resp.hovered() {
                            g.sel = k;
                        }
                        k += 1;
                    }
                }
                start = end;
            }
        });
    if let Some(id) = chosen {
        add_step(s, &id);
        return;
    }
    if s.gallery.is_some() {
        s.gallery = Some(g);
    }
}

fn tile(
    ui: &mut Ui,
    rect: Rect,
    id: &str,
    name: &str,
    grp: StepGroup,
    sel: bool,
    theme: &Theme,
) -> egui::Response {
    let c = &theme.colors;
    let col = kit::group_color(theme, grp);
    let resp = ui.interact(rect, Id::new(("fp-batch-tile", id)), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name));
    let hot = sel || resp.hovered() || resp.has_focus();
    let p = ui.painter();
    let cr = CornerRadius::same(6);
    let (bg, border) = if hot {
        (
            if theme.glow {
                with_alpha(col, 34)
            } else {
                kit::tint(theme, col, 0.9)
            },
            if theme.glow {
                with_alpha(col, 220)
            } else {
                mix(kit::card_fill(theme), col, 0.55)
            },
        )
    } else {
        (kit::item_fill(theme), kit::item_border(theme, col))
    };
    p.rect_filled(rect, cr, bg);
    p.rect_stroke(rect, cr, Stroke::new(1.0, border), StrokeKind::Inside);
    if resp.has_focus() {
        p.rect_stroke(
            rect.shrink(1.0),
            CornerRadius::same(5),
            Stroke::new(1.5, c.focus),
            StrokeKind::Inside,
        );
    }
    let chip = Rect::from_min_size(
        pos2(rect.left() + sp::S + 2.0, rect.center().y - 17.0),
        vec2(34.0, 34.0),
    );
    p.rect_filled(
        chip,
        CornerRadius::same(7),
        if theme.glow {
            with_alpha(col, 40)
        } else {
            kit::tint(theme, col, 1.2)
        },
    );
    kit::paint_icon(
        p,
        Rect::from_center_size(chip.center(), vec2(20.0, 20.0)),
        kit::group_icon(grp),
        theme,
        true,
    );
    let x = chip.right() + sp::M;
    let tw = (rect.right() - sp::S - x).max(20.0);
    kit::text_left(
        p,
        x,
        rect.center().y - 8.0,
        tw,
        name,
        theme.bold(type_scale::SECTION),
        if hot && theme.glow { col } else { c.text },
    );
    let desc = description(id);
    let shown = ui_kit::elide(ui.ctx(), &desc, &theme.font(type_scale::CAPTION), tw);
    p.text(
        pos2(x, rect.center().y + 9.0),
        Align2::LEFT_CENTER,
        &shown,
        theme.font(type_scale::CAPTION),
        kit::weak(theme),
    );
    let resp = if resp.hovered() {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    };
    if shown != desc {
        resp.hover_tip(desc)
    } else {
        resp
    }
}
