//! Weitere Bedienfelder: Pfad (Path.swf), Info (4120), Sonderzeichen, Bildbearbeitung
//! (Image Editing.swf), URL (4142), Verhalten (4128), Suchen und ersetzen (4129),
//! Symboleigenschaften (Rich Symbols), Eigenschaften automatischer Formen, CSS-Eigenschaften.

use super::common;
use crate::app_state::{Action, AppState};
use crate::inspector::fields::{self, Dl};
use crate::inspector::glyphs::Glyph;
use crate::theme::Theme;
use egui::{pos2, vec2, Align2, Color32, Id, Rect, Sense, Stroke, StrokeKind, Ui};
use fp_engine::ToolId;
use fp_model::ElementKind;
use serde_json::{json, Value};
use std::sync::Arc;

fn first(st: &AppState) -> Option<Arc<fp_model::Element>> {
    crate::inspector::obj_quads::selection(st).into_iter().next()
}

/// Überschrift und Befehlsknöpfe; `title` und die Beschriftungen sind Schlüssel. Knöpfe brechen um
/// und werden nie breiter als das Bedienfeld (sonst Text mit „…“, voller Text als Tooltip).
fn cmd_buttons(ui: &mut Ui, st: &mut AppState, title: &str, items: &[(&str, &str)]) {
    common::caption(ui, &common::t(title));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
        let max_w = ui.available_width().max(30.0);
        for (key, cmd) in items {
            let label = common::t(key);
            let enabled = st.session.is_available(cmd);
            let full = crate::widgets::text_width(ui, &label, &Theme::current(ui.ctx()).ui_font()) + 16.0;
            let (r, resp) = ui.allocate_exact_size(vec2(full.min(max_w), 20.0), Sense::hover());
            if full > max_w {
                let _ = resp.on_hover_text(&label);
            }
            if fields::button(ui, ("cmdbtn", *cmd), r, &label, enabled) {
                st.run(cmd, json!({}));
            }
        }
    });
    ui.add_space(6.0);
}

/// Überschrift (Schlüssel); bricht bei schmalen Bedienfeldern um.
fn caption_key(ui: &mut Ui, key: &str) {
    common::caption(ui, &common::t(key));
}

// ---------------------------------------------------------------------- Pfad

/// Pfad-Bedienfeld (Path.swf, nativ): Pfade kombinieren, Pfad ändern, Punkte.
pub fn path(ui: &mut Ui, st: &mut AppState) {
    ui.add_space(4.0);
    cmd_buttons(
        ui,
        st,
        "panel-path-combine",
        &[
            ("panel-path-join", "modify.combinePaths.join"),
            ("panel-path-split", "modify.combinePaths.split"),
            ("panel-path-union", "modify.combinePaths.union"),
            ("panel-path-intersect", "modify.combinePaths.intersect"),
            ("panel-path-punch", "modify.combinePaths.punch"),
            ("panel-path-crop", "modify.combinePaths.crop"),
        ],
    );
    cmd_buttons(
        ui,
        st,
        "panel-path-alter",
        &[
            ("panel-path-simplify", "modify.alterPath.simplify"),
            ("panel-path-expand-stroke", "modify.alterPath.expandStroke"),
            ("panel-path-inset", "modify.alterPath.insetPath"),
            ("panel-path-hard-fill", "modify.alterPath.hardFill"),
            ("panel-path-anti-alias-fill", "modify.alterPath.antiAliasFill"),
            ("panel-path-feather-fill", "modify.alterPath.featherFill"),
        ],
    );
    cmd_buttons(ui, st, "panel-path-edit-points", &[("panel-path-snap-pixel", "modify.snapToPixel"), ("panel-path-to-marquee", "modify.convertPathToMarquee")]);
    let compound = first(st).is_some_and(|e| matches!(e.kind, ElementKind::CompoundShape(_)));
    caption_key(ui, "panel-path-compound");
    // Knöpfe 84 px breit wie bisher; passen nicht alle vier in eine Zeile, so viele je Zeile wie
    // möglich (statt über den Rand zu laufen), bei sehr schmalen Bedienfeldern etwas schmaler
    let ops = [("panel-path-union", 1), ("panel-path-punch", 3), ("panel-path-intersect", 2), ("panel-path-crop", 4)];
    let gap = ui.spacing().item_spacing.x;
    let avail = ui.available_width();
    let per_row = (1..=4usize).rev().find(|n| *n as f32 * 78.0 + (*n as f32 - 1.0) * gap <= avail + 0.5).unwrap_or(1);
    let w = ((avail - (per_row as f32 - 1.0) * gap) / per_row as f32).min(84.0).floor().max(30.0);
    let paths = crate::inspector::obj_quads::selection(st).iter().filter(|e| matches!(e.kind, ElementKind::Path(_))).count();
    for chunk in ops.chunks(per_row) {
        ui.horizontal(|ui| {
            for (key, op) in chunk {
                let (r, _) = ui.allocate_exact_size(vec2(w, 20.0), Sense::hover());
                if fields::button(ui, ("pathop", *op), r, &common::t(key), compound || paths >= 2) {
                    st.run("pi.compound.setOp", json!({"op": op}));
                }
            }
        });
    }
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width().clamp(30.0, 84.0), 20.0), Sense::hover());
        if fields::button(ui, "pathop-combine", r, &common::t("panel-path-combine-compound"), compound) {
            st.run("pi.compound.combine", json!({}));
        }
    });
}

// ---------------------------------------------------------------------- Info

/// Info-Bedienfeld (4120): Farbwerte unter dem Mauszeiger (Modell aus `InfoPanelColorModel`),
/// Mausposition, Auswahl B/H/X/Y (editierbar, verschiebt/skaliert die Auswahl).
pub fn info(ui: &mut Ui, st: &mut AppState) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let (area, _) = ui.allocate_exact_size(vec2(ui.available_width().max(210.0), 110.0), Sense::hover());
    let d = Dl::at(area.min + vec2(4.0, 4.0));
    let model = st.session.prefs.get_int("InfoPanelColorModel", 1);
    let labels: [&str; 4] = match model {
        2 => ["C:", "M:", "Y:", "A:"],
        3 => ["H:", "S:", "B:", "A:"],
        _ => ["R:", "G:", "B:", "A:"],
    };
    let color = pixel_under_pointer(ui.ctx(), st);
    let vals: [String; 4] = match color {
        Some(px) => fp_color::models::info_panel_values(px.a, px.r, px.g, px.b, model),
        None => Default::default(),
    };
    crate::inspector::glyphs::paint(ui.painter(), d.r(1.0, 8.0, 12.0, 11.0), Glyph::Eyedropper, &theme, true);
    for (i, l) in labels.iter().enumerate() {
        fields::label_right(ui, d.r(10.0, 1.0 + i as f32 * 8.0, 13.0, 8.0), l, true);
        fields::label(ui, d.r(28.0, 1.0 + i as f32 * 8.0, 30.0, 8.0), &vals[i], true);
    }
    ui.painter().vline(d.r(67.0, 0.0, 1.0, 1.0).left(), d.r(0.0, 5.0, 1.0, 30.0).y_range(), Stroke::new(1.0, c.line_mid));
    // Einheiten aus dem Bedienfeldmenü (Pixel/Zoll/Zentimeter) über die Dokumentauflösung
    let unit = st.session.prefs.get_int("InfoPanelRulerUnits", 0);
    let ppi = st.session.active().and_then(|d| d.doc.pages.get(d.page)).map(|p| p.canvas.resolution as f64).filter(|r| *r > 0.0).unwrap_or(72.0);
    let fmt_u = |v: f64| match unit {
        1 => fp_i18n::fmt_decimal(v / ppi, 2),
        2 => fp_i18n::fmt_decimal(v / ppi * 2.54, 2),
        _ => format!("{v:.0}"),
    };
    let (x, y) = st.pointer_doc.map(|p| (p.x.round(), p.y.round())).unwrap_or((0.0, 0.0));
    fields::label_right(ui, d.r(84.0, 8.0, 10.0, 8.0), "X:", true);
    fields::label(ui, d.r(99.0, 8.0, 30.0, 8.0), &fmt_u(x as f64), true);
    fields::label_right(ui, d.r(84.0, 16.0, 10.0, 8.0), "Y:", true);
    fields::label(ui, d.r(99.0, 16.0, 30.0, 8.0), &fmt_u(y as f64), true);
    ui.painter().hline(d.r(0.0, 0.0, 139.0, 1.0).x_range(), d.r(0.0, 35.0, 1.0, 1.0).top(), Stroke::new(1.0, c.line_mid));
    // im offenen Text-Editor: Rahmen des Texts mit allen Änderungen (wächst live mit), sonst die Auswahl
    let b = if st.session.tool_is_editing_text() { st.session.text_edit_frame() } else { None }.or_else(|| st.session.active().and_then(|d| fp_engine::docops::ids_bounds(&d.doc, d.page, &d.selection.elements)));
    let en = b.is_some();
    let (bw, bh, bx, by) = b.map(|r| (r.width() as f64, r.height() as f64, r.left as f64, r.top as f64)).unwrap_or((0.0, 0.0, 0.0, 0.0));
    let (lw, lh) = (common::t("panel-info-width"), common::t("panel-info-height"));
    let rows = [(lw.as_str(), bw, "width", d.r(12.0, 40.0, 11.0, 8.0), d.r(26.0, 38.0, 30.0, 12.0)), (lh.as_str(), bh, "height", d.r(12.0, 53.0, 11.0, 8.0), d.r(26.0, 51.0, 30.0, 12.0)), ("X:", bx, "x", d.r(84.0, 40.0, 10.0, 8.0), d.r(97.0, 38.0, 30.0, 12.0)), ("Y:", by, "y", d.r(84.0, 53.0, 10.0, 8.0), d.r(97.0, 51.0, 30.0, 12.0))];
    for (l, v, key, lr, er) in rows {
        fields::label_right(ui, lr, l, en);
        if let Some(n) = fields::number(ui, ("info", key), er, None, en.then_some(v.round()), (-30000.0, 30000.0), 0, en) {
            st.run("element.setBounds", json!({ key: n }));
        }
    }
}

fn pixel_under_pointer(ctx: &egui::Context, st: &AppState) -> Option<fp_color::Rgba8> {
    let d = st.session.active()?;
    let p = st.pointer_doc?;
    let (x, y) = (p.x.floor() as i32, p.y.floor() as i32);
    let key = Id::new("info-pixel");
    if let Some((k, c)) = ctx.data(|dd| dd.get_temp::<((u64, u64, i32, i32), Option<fp_color::Rgba8>)>(key))
        && k == (d.id.0, d.generation, x, y)
    {
        return c;
    }
    let rect = fp_geom::IRect::new(x, y, x + 1, y + 1);
    let out = st.renderer.render_rect(&d.doc, d.page, d.frame, rect);
    let px = if out.image.width() == 1 && out.image.height() == 1 { Some(out.image.pixel(x, y)) } else { None };
    ctx.data_mut(|dd| dd.insert_temp(key, ((d.id.0, d.generation, x, y), px)));
    px
}

// ---------------------------------------------------------------------- Sonderzeichen

const SPECIAL: &[char] = &['©', '®', '™', '€', '£', '¥', '¢', '•', '…', '–', '—', '„', '“', '”', '‚', '‘', '’', '«', '»', '‹', '›', '§', '¶', '†', '‡', '°', '±', '×', '÷', '½', '¼', '¾', '¹', '²', '³', 'µ', '¿', '¡', 'ß', 'ä', 'ö', 'ü', 'Ä', 'Ö', 'Ü', 'é', 'è', 'ê', 'à', 'ç', 'ñ', 'ø', 'Ø', 'å', 'Å', 'æ', 'œ', '≈', '≠', '≤', '≥', '∞', '√', '←', '→', '↑', '↓', '♥', '✓'];

/// Sonderzeichen: Klick hängt das Zeichen an den ausgewählten Text an.
pub fn special_characters(ui: &mut Ui, st: &mut AppState) {
    let theme = Theme::current(ui.ctx());
    let text = first(st).and_then(|e| match &e.kind {
        ElementKind::Text(t) => Some((e.id, t.plain_text())),
        _ => None,
    });
    let cell = 22.0;
    let cols = ((ui.available_width() / cell).floor() as usize).max(1);
    let rows = SPECIAL.len().div_ceil(cols);
    let (area, _) = ui.allocate_exact_size(vec2(cols as f32 * cell, rows as f32 * cell), Sense::hover());
    for (i, ch) in SPECIAL.iter().enumerate() {
        let r = Rect::from_min_size(pos2(area.left() + (i % cols) as f32 * cell, area.top() + (i / cols) as f32 * cell), vec2(cell - 1.0, cell - 1.0));
        let resp = ui.interact(r, Id::new(("sc", i)), Sense::click());
        ui.painter().rect_filled(r, 0.0, if resp.hovered() { theme.colors.list_hover_bg } else { theme.colors.list_bg });
        ui.painter().rect_stroke(r, 0.0, Stroke::new(1.0, theme.colors.list_row_line), StrokeKind::Inside);
        ui.painter().text(r.center(), Align2::CENTER_CENTER, ch.to_string(), theme.font(13.0), theme.colors.text);
        let resp = resp.on_hover_text(format!("U+{:04X}", *ch as u32));
        if resp.clicked() {
            match &text {
                Some((id, t)) => st.run("text.setText", json!({"id": id.0, "text": format!("{t}{ch}")})),
                None => {
                    ui.ctx().copy_text(ch.to_string());
                    st.push(Action::Status(fp_i18n::tr!("panel-special-copied", ch = ch.to_string())));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------- Bildbearbeitung

/// Bildbearbeitung (Image Editing.swf, nativ): Bitmap-Werkzeuge, Anpassungen, Filter.
pub fn image_editing(ui: &mut Ui, st: &mut AppState) {
    let theme = Theme::current(ui.ctx());
    caption_key(ui, "panel-imgedit-tools");
    let tools = [
        ToolId::Marquee,
        ToolId::OvalMarquee,
        ToolId::Lasso,
        ToolId::PolygonLasso,
        ToolId::MagicWand,
        ToolId::Brush,
        ToolId::Pencil,
        ToolId::Eraser,
        ToolId::Blur,
        ToolId::Sharpen,
        ToolId::Dodge,
        ToolId::Burn,
        ToolId::Smudge,
        ToolId::RubberStamp,
        ToolId::ReplaceColor,
        ToolId::RedEyeRemoval,
        ToolId::Scale,
        ToolId::Skew,
        ToolId::Distort,
    ];
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
        for t in tools {
            let tip = common::tool_label(t);
            let look = crate::widgets::ButtonLook { selected: st.session.tools.active == t, enabled: true, group: Some(crate::theme::ToolGroup::Bitmap) };
            if crate::widgets::icon_button(ui, crate::icons::tool_icon(t), vec2(24.0, 22.0), 16.0, look, &tip).clicked() {
                st.push(Action::SetTool(t));
            }
        }
    });
    ui.add_space(6.0);
    let _ = theme;
    cmd_buttons(
        ui,
        st,
        "panel-imgedit-adjust",
        &[
            ("panel-imgedit-auto-levels", "filter.adjustColor.autoLevels"),
            ("panel-imgedit-brightness", "filter.adjustColor.brightnessContrast"),
            ("panel-imgedit-curves", "filter.adjustColor.curves"),
            ("panel-imgedit-levels", "filter.adjustColor.levels"),
            ("panel-imgedit-hue-sat", "filter.adjustColor.hueSaturation"),
            ("panel-imgedit-invert", "filter.adjustColor.invert"),
        ],
    );
    cmd_buttons(
        ui,
        st,
        "panel-imgedit-filters",
        &[
            ("panel-imgedit-gaussian", "filter.blur.gaussian"),
            ("panel-imgedit-unsharp", "filter.sharpen.unsharpMask"),
            ("panel-imgedit-noise", "filter.noise.addNoise"),
            ("panel-imgedit-numeric", "modify.transform.numericTransform"),
        ],
    );
}

// ---------------------------------------------------------------------- URL

const URL_LIB: &str = "url-library";

fn url_file() -> Option<std::path::PathBuf> {
    fp_engine::Preferences::default_path().and_then(|p| p.parent().map(|d| d.join("URLs.txt")))
}

fn url_library(ctx: &egui::Context) -> Vec<String> {
    if let Some(v) = ctx.data(|d| d.get_temp::<Vec<String>>(Id::new(URL_LIB))) {
        return v;
    }
    let v: Vec<String> = url_file().and_then(|p| std::fs::read_to_string(p).ok()).map(|s| s.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect()).unwrap_or_default();
    ctx.data_mut(|d| d.insert_temp(Id::new(URL_LIB), v.clone()));
    v
}

fn set_url_library(ctx: &egui::Context, v: Vec<String>) {
    if let Some(p) = url_file() {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(p, v.join("\n"));
    }
    ctx.data_mut(|d| d.insert_temp(Id::new(URL_LIB), v));
}

/// URL-Bedienfeld (4142): aktuelle URL, Bibliothek, Liste, Hinzufügen/Löschen.
pub fn url(ui: &mut Ui, st: &mut AppState) {
    let theme = Theme::current(ui.ctx());
    let cur = first(st).and_then(|e| match &e.kind {
        ElementKind::WebObject(w) => Some(w.url.clone().unwrap_or_default()),
        ElementKind::Instance(i) => Some(i.button.url.clone().unwrap_or_default()),
        _ => None,
    });
    let en = cur.is_some();
    let mut lib = url_library(ui.ctx());
    let (head, _) = ui.allocate_exact_size(vec2(ui.available_width(), 50.0), Sense::hover());
    let d = Dl::at(head.min + vec2(4.0, 2.0));
    fields::label(ui, d.r(0.0, 2.0, 40.0, 8.0), "URL:", en);
    if let Some(v) = fields::combo_edit(ui, "url-cur", d.r(30.0, 0.0, 110.0, 13.0), cur.as_deref().unwrap_or(""), &lib, en) {
        st.run("pi.web", json!({"url": v}));
    }
    let _ = fields::combo(ui, "url-lib", d.r(30.0, 15.0, 110.0, 13.0), &["URLs.htm"], Some(0), true);
    let sel: Option<usize> = ui.data(|dd| dd.get_temp(Id::new("url-sel")));
    let mut pick = None;
    common::list_frame(ui, "url-list", common::FOOTER_H, |ui| {
        for (i, u) in lib.iter().enumerate() {
            let (r, resp) = common::row(ui, 18.0, sel == Some(i));
            crate::widgets::text_clipped(ui.painter(), pos2(r.left() + 6.0, r.center().y), Align2::LEFT_CENTER, u, theme.ui_font(), theme.colors.text, r.width() - 8.0);
            if resp.clicked() {
                pick = Some(i);
            }
        }
    });
    if let Some(i) = pick {
        ui.data_mut(|dd| dd.insert_temp(Id::new("url-sel"), i));
        if en {
            st.run("pi.web", json!({"url": lib[i]}));
        }
    }
    let rect = ui.available_rect_before_wrap();
    let y = rect.bottom() - common::FOOTER_H + 3.0;
    if fields::icon_button(ui, "url-add", Rect::from_min_size(pos2(rect.right() - 44.0, y), vec2(19.0, 16.0)), Glyph::Plus, cur.as_ref().is_some_and(|u| !u.is_empty()), &common::t("panel-url-add")).clicked()
        && let Some(u) = cur
        && !lib.contains(&u)
    {
        lib.push(u);
        set_url_library(ui.ctx(), lib.clone());
    }
    if fields::icon_button(ui, "url-del", Rect::from_min_size(pos2(rect.right() - 23.0, y), vec2(19.0, 16.0)), Glyph::Trash, sel.is_some(), &common::t("panel-url-delete")).clicked()
        && let Some(i) = sel
        && i < lib.len()
    {
        lib.remove(i);
        set_url_library(ui.ctx(), lib);
        ui.data_mut(|dd| dd.remove::<usize>(Id::new("url-sel")));
    }
}

// ---------------------------------------------------------------------- Verhalten

/// Aktionen des Verhalten-Menüs (6928): Schlüssel des Anzeigenamens, `BETi`.
pub const BEHAVIOR_ACTIONS: &[(&str, i32)] = &[
    ("panel-beh-simple-rollover", 8),
    ("panel-beh-swap-image", 2),
    ("panel-beh-swap-restore", 5),
    ("panel-beh-nav-image", 0),
    ("panel-beh-nav-over", 6),
    ("panel-beh-nav-down", 4),
    ("panel-beh-nav-restore", 7),
    ("panel-beh-status-text", 1),
];

/// Anzeigename einer Verhaltensaktion (`BETi`).
pub fn behavior_label(a: i32) -> String {
    common::t(match a {
        9 => "panel-beh-popup",
        11 => "panel-beh-sound",
        3 => "panel-beh-nav-image",
        _ => BEHAVIOR_ACTIONS.iter().find(|b| b.1 == a).map(|b| b.0).unwrap_or("panel-beh-generic"),
    })
}

/// Verhalten-Bedienfeld (4128).
pub fn behaviors(ui: &mut Ui, st: &mut AppState) {
    let theme = Theme::current(ui.ctx());
    let e = first(st);
    let list: Option<Vec<fp_model::Behavior>> = e.as_ref().and_then(|e| match &e.kind {
        ElementKind::WebObject(w) => Some(w.behaviors.as_ref().map(|b| b.items.clone()).unwrap_or_default()),
        ElementKind::Instance(i) => Some(i.behaviors.as_ref().map(|b| b.items.clone()).unwrap_or_default()),
        _ => None,
    });
    let en = list.is_some();
    let (head, _) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::hover());
    let add = fields::icon_button(ui, "beh-add", Rect::from_min_size(head.min + vec2(4.0, 3.0), vec2(19.0, 17.0)), Glyph::Plus, en, &common::t("panel-beh-add"));
    egui::Popup::menu(&add).frame(crate::menu::menu_frame(&theme)).show(|ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        for (label, a) in BEHAVIOR_ACTIONS {
            if fields::list_item(ui, &common::t(label), false).clicked() {
                if *a == 1 {
                    st.dialogs.open_status_text();
                } else {
                    st.run("pi.behavior.add", json!({"action": a, "event": if *a == 5 || *a == 7 { 2 } else { 0 }}));
                }
                ui.close();
            }
        }
        if fields::list_item(ui, &common::t("panel-beh-popup-menu"), false).clicked() {
            // Pop-up-Menü-Editor (6283) legt das Verhalten „Popupmenü anzeigen“ an
            st.run("modify.popUpMenu.addPopUpMenu", json!({}));
            ui.close();
        }
    });
    let sel: Option<usize> = ui.data(|d| d.get_temp(Id::new("beh-sel")));
    if fields::icon_button(ui, "beh-del", Rect::from_min_size(head.min + vec2(25.0, 3.0), vec2(19.0, 17.0)), Glyph::Minus, en && sel.is_some(), &common::t("panel-beh-remove")).clicked()
        && let Some(i) = sel
    {
        st.run("pi.behavior.remove", json!({"index": i}));
        ui.data_mut(|d| d.remove::<usize>(Id::new("beh-sel")));
    }
    let mut actions = Vec::new();
    common::list_frame(ui, "beh-list", 0.0, |ui| {
        let (hr, _) = ui.allocate_exact_size(vec2(ui.available_width(), 18.0), Sense::hover());
        ui.painter().rect_filled(hr, 0.0, theme.colors.header_top);
        common::fit_text(ui, Rect::from_min_max(pos2(hr.left() + 6.0, hr.top()), pos2(hr.left() + 106.0, hr.bottom())), Align2::LEFT_CENTER, &common::t("panel-beh-events"), theme.small_font(), theme.colors.text);
        common::fit_text(ui, Rect::from_min_max(pos2(hr.left() + 110.0, hr.top()), pos2(hr.right() - 2.0, hr.bottom())), Align2::LEFT_CENTER, &common::t("panel-beh-actions"), theme.small_font(), theme.colors.text);
        for (i, b) in list.clone().unwrap_or_default().iter().enumerate() {
            let (r, resp) = common::row(ui, 20.0, sel == Some(i));
            let er = Rect::from_min_size(pos2(r.left() + 2.0, r.top() + 1.0), vec2(104.0, 18.0));
            if let Some(ev) = fields::combo(ui, ("beh-ev", i), er, &fp_model::web::BEHAVIOR_EVENTS, Some(b.event.clamp(0, 3) as usize), true) {
                actions.push(Action::Run("pi.behavior.event".into(), json!({"index": i, "event": ev})));
            }
            crate::widgets::text_clipped(ui.painter(), pos2(r.left() + 110.0, r.center().y), Align2::LEFT_CENTER, &behavior_label(b.action), theme.ui_font(), theme.colors.text, r.width() - 112.0);
            if resp.clicked() {
                ui.data_mut(|d| d.insert_temp(Id::new("beh-sel"), i));
            }
        }
        if !en {
            ui.label(egui::RichText::new(common::t("panel-beh-select-hint")).color(theme.colors.text_disabled).font(theme.small_font()));
        }
    });
    for a in actions {
        st.push(a);
    }
}

// ---------------------------------------------------------------------- Suchen und ersetzen

#[derive(Clone, Debug, PartialEq, Default)]
struct FindState {
    scope: usize,
    kind: usize,
    find: String,
    replace: String,
    whole_word: bool,
    case: bool,
    find_color: Option<u32>,
    replace_color: Option<u32>,
    last: Option<u64>,
}

fn matches_text(hay: &str, needle: &str, case: bool, whole: bool) -> bool {
    if needle.is_empty() {
        return false;
    }
    let (h, n) = if case { (hay.to_string(), needle.to_string()) } else { (hay.to_lowercase(), needle.to_lowercase()) };
    if !whole {
        return h.contains(&n);
    }
    h.split(|c: char| !c.is_alphanumeric()).any(|w| w == n)
}

fn replace_text(hay: &str, needle: &str, rep: &str, case: bool, whole: bool) -> String {
    if needle.is_empty() {
        return hay.to_string();
    }
    if case && !whole {
        return hay.replace(needle, rep);
    }
    // einfache Suche ohne Groß-/Kleinschreibung bzw. ganze Wörter
    let mut out = String::new();
    let chars: Vec<char> = hay.chars().collect();
    let nchars: Vec<char> = needle.chars().collect();
    let eq = |a: char, b: char| if case { a == b } else { a.to_lowercase().eq(b.to_lowercase()) };
    let mut i = 0;
    while i < chars.len() {
        let fits = i + nchars.len() <= chars.len() && nchars.iter().enumerate().all(|(k, c)| eq(chars[i + k], *c));
        let boundary = !whole || ((i == 0 || !chars[i - 1].is_alphanumeric()) && (i + nchars.len() >= chars.len() || !chars[i + nchars.len()].is_alphanumeric()));
        if fits && boundary {
            out.push_str(rep);
            i += nchars.len();
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// Treffer und Ersetzungsbefehle für den aktuellen Suchzustand.
fn find_hits(st: &AppState, fs: &FindState) -> Vec<(u64, Value)> {
    let Some(d) = st.session.active() else { return Vec::new() };
    let mut out = Vec::new();
    let sel: Vec<u64> = d.selection.elements.iter().map(|e| e.0).collect();
    let Some(pg) = d.doc.pages.get(d.page) else { return out };
    fp_model::traverse::walk_root(&pg.root, fp_model::RootRef::Page(d.page), &mut |_, _, e| {
        if fs.scope == 0 && !sel.contains(&e.id.0) {
            return;
        }
        match (fs.kind, &e.kind) {
            (0, ElementKind::Text(t)) => {
                let s = t.plain_text();
                if matches_text(&s, &fs.find, fs.case, fs.whole_word) {
                    out.push((e.id.0, json!({"command": "text.setText", "params": {"id": e.id.0, "text": replace_text(&s, &fs.find, &fs.replace, fs.case, fs.whole_word)}})));
                }
            }
            (1, ElementKind::Text(t)) => {
                let f = t.resolved_runs().first().and_then(|(a, _)| a.font.clone()).unwrap_or_default();
                if !fs.find.is_empty() && f.to_lowercase().contains(&fs.find.to_lowercase()) {
                    out.push((e.id.0, json!({"command": "text.setAttributes", "params": {"ids": [e.id.0], "font": fs.replace}})));
                }
            }
            (2, _) => {
                if let (Some(a), Some(fc), Some(rc)) = (fp_engine::docops::path_attrs_of(e), fs.find_color, fs.replace_color) {
                    let hex = crate::ui_commands::hex(fp_color::Argb(rc | 0xff00_0000));
                    if (a.fill_color.0 & 0xffffff) == (fc & 0xffffff) && a.fill.is_some() {
                        out.push((e.id.0, json!({"command": "pi.fill", "params": {"ids": [e.id.0], "color": hex}})));
                    }
                    if (a.stroke_color.0 & 0xffffff) == (fc & 0xffffff) && a.brush.is_some() {
                        out.push((e.id.0, json!({"command": "pi.stroke", "params": {"ids": [e.id.0], "color": hex}})));
                    }
                }
            }
            (4, _) => {
                // Nicht-Web216: Füll-/Strichfarben außerhalb des Web-216-Würfels → auf websicher einrasten
                if let Some(a) = fp_engine::docops::path_attrs_of(e) {
                    let web = |c: fp_color::Argb| [c.r(), c.g(), c.b()].iter().all(|v| v % 51 == 0);
                    let snap = |c: fp_color::Argb| {
                        let s = |v: u8| (((v as u32) + 25) / 51 * 51) as u8;
                        crate::ui_commands::hex(fp_color::Argb(0xff00_0000 | (s(c.r()) as u32) << 16 | (s(c.g()) as u32) << 8 | s(c.b()) as u32))
                    };
                    if a.fill.is_some() && !web(a.fill_color) {
                        out.push((e.id.0, json!({"command": "pi.fill", "params": {"ids": [e.id.0], "color": snap(a.fill_color)}})));
                    }
                    if a.brush.is_some() && !web(a.stroke_color) {
                        out.push((e.id.0, json!({"command": "pi.stroke", "params": {"ids": [e.id.0], "color": snap(a.stroke_color)}})));
                    }
                }
            }
            (3, ElementKind::WebObject(w)) => {
                if let Some(u) = &w.url
                    && matches_text(u, &fs.find, fs.case, fs.whole_word)
                {
                    out.push((e.id.0, json!({"command": "pi.web", "params": {"ids": [e.id.0], "url": replace_text(u, &fs.find, &fs.replace, fs.case, fs.whole_word)}})));
                }
            }
            _ => {}
        }
    });
    out
}

/// Suchen und ersetzen (4129 mit Unterseiten Text 4130, Schrift 4132, Farbe 4133, URL 4131).
pub fn find_replace(ui: &mut Ui, st: &mut AppState) {
    let key = Id::new("find-state");
    let mut fs: FindState = ui.data(|d| d.get_temp(key)).unwrap_or_default();
    let before = fs.clone();
    let (area, _) = ui.allocate_exact_size(vec2(ui.available_width(), 170.0), Sense::hover());
    let d = Dl::at(area.min + vec2(4.0, 4.0));
    let tx = |keys: &[&str]| -> Vec<String> { keys.iter().map(|k| common::t(k)).collect() };
    let scopes = tx(&["panel-find-scope-selection", "panel-find-scope-page"]);
    let scope_refs: Vec<&str> = scopes.iter().map(|s| s.as_str()).collect();
    if let Some(i) = fields::combo(ui, "find-scope", d.r(0.0, 0.0, 70.0, 13.0), &scope_refs, Some(fs.scope), true) {
        fs.scope = i;
    }
    let kinds = tx(&["panel-find-kind-text", "panel-find-kind-font", "panel-find-kind-color", "panel-find-kind-url", "panel-find-kind-nonweb"]);
    let kind_refs: Vec<&str> = kinds.iter().map(|s| s.as_str()).collect();
    if let Some(i) = fields::combo(ui, "find-kind", d.r(74.0, 0.0, 66.0, 13.0), &kind_refs, Some(fs.kind), true) {
        fs.kind = i;
    }
    let (l_find, l_change) = (common::t("panel-find-find"), common::t("panel-find-change"));
    match fs.kind {
        2 => {
            common::label_fit(ui, d.r(0.0, 20.0, 43.0, 8.0), &l_find, true);
            if let Some(popups_choice) = crate::inspector::popups::color_button(ui, "find-c1", d.r(44.0, 18.0, 16.0, 13.0), st, fs.find_color.map(fp_color::Argb), true, false, false)
                && let crate::inspector::popups::ColorChoice::Color(c) = popups_choice
            {
                fs.find_color = Some(c.0);
            }
            common::label_fit(ui, d.r(0.0, 36.0, 43.0, 8.0), &l_change, true);
            if let Some(crate::inspector::popups::ColorChoice::Color(c)) = crate::inspector::popups::color_button(ui, "find-c2", d.r(44.0, 34.0, 16.0, 13.0), st, fs.replace_color.map(fp_color::Argb), true, false, false) {
                fs.replace_color = Some(c.0);
            }
        }
        4 => {
            crate::dialogs::put_left(ui, d.r(0.0, 20.0, 140.0, 40.0), egui::Label::new(egui::RichText::new(common::t("panel-find-nonweb-hint")).font(Theme::current(ui.ctx()).small_font())).wrap());
        }
        _ => {
            common::label_fit(ui, d.r(0.0, 20.0, 43.0, 8.0), &l_find, true);
            if let Some(v) = fields::text(ui, "find-text", d.r(44.0, 18.0, 96.0, 13.0), &fs.find, true) {
                fs.find = v;
            }
            common::label_fit(ui, d.r(0.0, 36.0, 43.0, 8.0), &l_change, true);
            if let Some(v) = fields::text(ui, "find-rep", d.r(44.0, 34.0, 96.0, 13.0), &fs.replace, true) {
                fs.replace = v;
            }
            if fs.kind != 1 {
                if let Some(b) = common::checkbox_fit(ui, "find-word", d.r(0.0, 52.0, 70.0, 10.0), &common::t("panel-find-whole-word"), fs.whole_word, true) {
                    fs.whole_word = b;
                }
                if let Some(b) = common::checkbox_fit(ui, "find-case", d.r(72.0, 52.0, 70.0, 10.0), &common::t("panel-find-match-case"), fs.case, true) {
                    fs.case = b;
                }
            }
        }
    }
    let hits = find_hits(st, &fs);
    let y = 68.0;
    if fields::button(ui, "find-next", d.r(0.0, y, 44.0, 14.0), &common::t("panel-find-next"), !hits.is_empty()) {
        let next = hits.iter().map(|h| h.0).find(|id| fs.last.is_none_or(|l| *id > l)).or_else(|| hits.first().map(|h| h.0));
        if let Some(id) = next {
            fs.last = Some(id);
            st.run("select.set", json!({"ids": [id]}));
        }
    }
    let (l_rep, l_all) = (common::t("panel-find-replace"), common::t("panel-find-replace-all"));
    if fields::button(ui, "find-rep1", d.r(48.0, y, 44.0, 14.0), &l_rep, !hits.is_empty()) {
        let target = fs.last.and_then(|l| hits.iter().find(|h| h.0 == l)).or(hits.first());
        if let Some((_, c)) = target {
            st.run("history.replay", json!({"commands": [c], "label": l_rep}));
        }
    }
    if fields::button(ui, "find-all", d.r(96.0, y, 44.0, 14.0), &l_all, !hits.is_empty()) {
        let cmds: Vec<Value> = hits.iter().map(|h| h.1.clone()).collect();
        st.run("history.replay", json!({"commands": cmds, "label": l_all}));
    }
    let theme = Theme::current(ui.ctx());
    common::fit_text(ui, d.r(0.0, y + 20.0, 140.0, 8.0), Align2::LEFT_CENTER, &fp_i18n::tr!("panel-find-hits", n = hits.len()), theme.small_font(), theme.colors.text_weak);
    if fs != before {
        ui.data_mut(|dd| dd.insert_temp(key, fs));
    }
}

// ---------------------------------------------------------------------- Symboleigenschaften

/// Symboleigenschaften (Symbol Properties.swf): Parameter des ausgewählten Rich Symbols.
pub fn symbol_properties(ui: &mut Ui, st: &mut AppState) {
    let theme = Theme::current(ui.ctx());
    let params = first(st).and_then(|e| match &e.kind {
        ElementKind::Instance(i) => Some(i.widget_values()),
        _ => None,
    });
    let Some(params) = params.filter(|p| !p.is_empty()) else {
        ui.add_space(8.0);
        ui.label(egui::RichText::new(common::t("panel-symprops-none")).color(theme.colors.text_disabled).font(theme.small_font()));
        return;
    };
    let (hr, _) = ui.allocate_exact_size(vec2(ui.available_width(), 18.0), Sense::hover());
    ui.painter().rect_filled(hr, 0.0, theme.colors.header_top);
    common::fit_text(ui, Rect::from_min_max(pos2(hr.left() + 6.0, hr.top()), pos2(hr.center().x - 4.0, hr.bottom())), Align2::LEFT_CENTER, &common::t("panel-symprops-name"), theme.small_font(), theme.colors.text);
    common::fit_text(ui, Rect::from_min_max(pos2(hr.center().x, hr.top()), pos2(hr.right() - 2.0, hr.bottom())), Align2::LEFT_CENTER, &common::t("panel-symprops-value"), theme.small_font(), theme.colors.text);
    for (i, p) in params.iter().enumerate() {
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 22.0), Sense::hover());
        crate::widgets::text_clipped(ui.painter(), pos2(r.left() + 6.0, r.center().y), Align2::LEFT_CENTER, &p.name, theme.ui_font(), theme.colors.text, r.width() / 2.0 - 10.0);
        let vr = Rect::from_min_max(pos2(r.center().x, r.top() + 1.0), pos2(r.right() - 4.0, r.bottom() - 1.0));
        if p.kind.eq_ignore_ascii_case("boolean") {
            let on = p.value == "true";
            if let Some(b) = fields::checkbox(ui, ("symp", i), vr, "", on, true) {
                st.run("pi.symbol.params", json!({"values": {p.name.clone(): if b { "true" } else { "false" }}}));
            }
        } else if p.kind.eq_ignore_ascii_case("color") {
            let col = fp_color::Rgba8::parse_hex(&p.value).map(|c| c.to_argb());
            if let Some(crate::inspector::popups::ColorChoice::Color(c)) = crate::inspector::popups::color_button(ui, ("symp-c", i), Rect::from_min_size(vr.min, vec2(20.0, vr.height())), st, col, true, false, false) {
                st.run("pi.symbol.params", json!({"values": {p.name.clone(): format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b())}}));
            }
        } else if let Some(v) = fields::text(ui, ("symp", i), vr, &p.value, true) {
            st.run("pi.symbol.params", json!({"values": {p.name.clone(): v}}));
        }
    }
    common::not_available(ui, &common::t("panel-symprops-note"));
}

// ---------------------------------------------------------------------- CSS-Eigenschaften

fn css_color(c: fp_color::Argb) -> String {
    if c.a() == 255 { format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b()) } else { format!("rgba({}, {}, {}, {:.2})", c.r(), c.g(), c.b(), c.a() as f32 / 255.0) }
}

/// CSS-Text der Auswahl (Text: Schrift, Größe, Farbe; Rechteck: Hintergrund, Rundung, Rahmen, Schatten).
pub fn css_for(st: &AppState) -> String {
    let Some(e) = first(st) else { return String::new() };
    let mut out = Vec::new();
    if let Some(d) = st.session.active()
        && let Some(b) = fp_engine::docops::element_bounds(&d.doc, &e)
    {
        out.push(format!("width: {:.0}px;", b.width()));
        out.push(format!("height: {:.0}px;", b.height()));
    }
    match &e.kind {
        ElementKind::Text(t) => {
            if let Some((a, _)) = t.resolved_runs().first() {
                if let Some(f) = &a.font {
                    let (fam, _) = crate::inspector::fonts::lookup(f);
                    out.push(format!("font-family: \"{fam}\";"));
                }
                if let Some(s) = a.size {
                    out.push(format!("font-size: {s}px;"));
                }
                if a.bold == Some(true) {
                    out.push("font-weight: bold;".into());
                }
                if a.italic == Some(true) {
                    out.push("font-style: italic;".into());
                }
                if a.underline == Some(true) {
                    out.push("text-decoration: underline;".into());
                }
                let col = a.color.map(fp_color::Argb).unwrap_or(t.attrs.fill_color);
                out.push(format!("color: {};", css_color(col)));
                if let Some(al) = a.alignment {
                    out.push(format!("text-align: {};", match al {
                        1 => "center",
                        2 => "right",
                        3 | 4 => "justify",
                        _ => "left",
                    }));
                }
            }
        }
        _ => {
            if let Some(a) = fp_engine::docops::path_attrs_of(&e) {
                if let Some(f) = &a.fill {
                    if let Some(g) = &f.gradient {
                        let stops: Vec<String> = g.color_stops().iter().map(|s| format!("{} {:.0}%", css_color(s.color), s.position * 100.0)).collect();
                        out.push(format!("background: linear-gradient(to right, {});", stops.join(", ")));
                    } else {
                        out.push(format!("background-color: {};", css_color(a.fill_color)));
                    }
                }
                if let Some(b) = &a.brush {
                    out.push(format!("border: {}px solid {};", b.diameter, css_color(a.stroke_color)));
                }
            }
            if let ElementKind::Path(p) = &e.kind
                && let Some(pr) = &p.primitive
                && pr.roundness > 0.0
            {
                let r = pr.roundness * pr.rect.width().abs().min(pr.rect.height().abs()) / 2.0;
                out.push(format!("border-radius: {r:.0}px;"));
            }
        }
    }
    if e.common.opacity.0 < 255 {
        out.push(format!("opacity: {:.2};", e.common.opacity.0 as f32 / 255.0));
    }
    if let Some(l) = &e.common.effects {
        for fx in &l.effects {
            if fx.kind() == fp_model::EffectKind::DropShadow && fx.is_visible() {
                let dist = fx.get_f64("ShadowDistance").unwrap_or(7.0);
                let ang = fx.get_f64("ShadowAngle").unwrap_or(315.0).to_radians();
                let blur = fx.get_f64("ShadowBlur").unwrap_or(4.0);
                let col = fx.get_color("ShadowColor").unwrap_or(fp_color::Argb(0xa600_0000));
                out.push(format!("box-shadow: {:.0}px {:.0}px {:.0}px {};", (dist * ang.cos()).trunc(), (-dist * ang.sin()).trunc(), blur, css_color(col)));
            }
        }
    }
    out.join("\n")
}

/// CSS-Eigenschaften (CSXS-Erweiterung CSSProperties, nativ).
pub fn css_properties(ui: &mut Ui, st: &mut AppState) {
    let theme = Theme::current(ui.ctx());
    let mut css = css_for(st);
    if css.is_empty() {
        ui.add_space(8.0);
        ui.label(egui::RichText::new(common::t("panel-css-select")).color(theme.colors.text_disabled).font(theme.small_font()));
        return;
    }
    let avail = ui.available_rect_before_wrap();
    let r = Rect::from_min_max(avail.min + vec2(2.0, 2.0), pos2(avail.right() - 2.0, avail.bottom() - 28.0));
    ui.painter().rect_filled(r, 0.0, theme.colors.list_bg);
    ui.put(r.shrink(2.0), egui::TextEdit::multiline(&mut css).font(egui::FontId::monospace(11.0)).interactive(false).frame(egui::Frame::NONE));
    if fields::button(ui, "css-copy", Rect::from_min_size(pos2(avail.left() + 2.0, avail.bottom() - 24.0), vec2(80.0_f32.max(crate::widgets::text_width(ui, &common::t("panel-css-copy-all"), &theme.ui_font()) + 12.0).min(avail.width() - 4.0), 20.0)), &common::t("panel-css-copy-all"), true) {
        ui.ctx().copy_text(css_for(st));
        st.push(Action::Status(common::t("panel-css-copied")));
    }
    let _ = Color32::WHITE;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_and_replace_text() {
        assert!(matches_text("Hallo Welt", "welt", false, false));
        assert!(!matches_text("Hallo Welt", "welt", true, false));
        assert!(!matches_text("Weltall", "welt", false, true));
        assert_eq!(replace_text("Hallo Welt, WELT!", "welt", "Mond", false, false), "Hallo Mond, Mond!");
        assert_eq!(replace_text("Welt Weltall", "Welt", "X", true, true), "X Weltall");
    }
}
