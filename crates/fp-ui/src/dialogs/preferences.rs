//! Voreinstellungen (Dialog 4419, Kategorieliste 4423, Seiten 4426–4432, 14 §3.4): alle sieben
//! Kategorien mit den Schlüsseln und Vorgaben aus 14 §3.4, dazu unter „Allgemein ▸
//! Benutzeroberfläche“ die Design-Wahl Normal/Dunkel/OLED. Änderungen erst mit OK; danach
//! sofort gespeichert (`SavePrefsWhileRunning`).

use super::{dialog_frame, push_button, t, DialogClose};
use crate::app_state::{Action, AppState};
use crate::inspector::fields;
use crate::theme::{Theme, ThemeKind};
use egui::{pos2, vec2, Align2, Rect, Sense, Stroke, StrokeKind, Ui};
use fp_engine::{PrefValue, Preferences};
use std::collections::BTreeMap;

/// Kategorien in der Reihenfolge der Liste.
/// (Schlüssel der Texte in `dialoge.ftl`; ebenso die Beschriftungen der Einträge.)
pub const CATEGORIES: [&str; 7] = ["dlg-pref-cat-general", "dlg-pref-cat-edit", "dlg-pref-cat-guides", "dlg-pref-cat-type", "dlg-pref-cat-psimport", "dlg-pref-cat-plugins", "dlg-pref-cat-launch"];

/// Ein Eintrag einer Seite (Voreinstellungsschlüssel, Textschlüssel der Beschriftung, …).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Item {
    Check(&'static str, &'static str, bool),
    Int(&'static str, &'static str, i64, i64, i64),
    Real(&'static str, &'static str, f64, f64, f64),
    Color(&'static str, &'static str, &'static str),
    /// 1-basiertes Kombifeld (`EditTransformMode` usw.).
    Combo1(&'static str, &'static str, &'static [&'static str], i64),
    Text(&'static str, &'static str, &'static str),
    Caption(&'static str),
}

/// Interpolationsverfahren (auch „Bildgröße“).
pub(crate) const INTERP: &[&str] = &["dlg-interp-bicubic", "dlg-interp-bilinear", "dlg-interp-soft", "dlg-interp-nearest"];
const LOOK: &[&str] = &["dlg-pref-look-always", "dlg-pref-look-never", "dlg-pref-look-ask"];

fn page_items(cat: usize) -> &'static [Item] {
    use Item::*;
    match cat {
        0 => &[
            Int("MaxUndos", "dlg-pref-maxundos", 20, 0, 999),
            Check("ShowStartPage", "dlg-pref-showstartpage", true),
            Check("XformToolXformsBFE", "dlg-pref-xformtoolxformsbfe", true),
            Combo1("EditTransformMode", "dlg-pref-edittransformmode", INTERP, 1),
            Check("OSBasedResample", "dlg-pref-osbasedresample", false),
            Color("BrushDefaultColor", "dlg-pref-brushdefaultcolor", "#000000"),
            Color("FillDefaultColor", "dlg-pref-filldefaultcolor", "#ffffff"),
            Color("FeedbackColor", "dlg-pref-feedbackcolor", "#00a8ff"),
            Check("IconPanelsAutoCollapse", "dlg-pref-iconpanelsautocollapse", false),
            Check("WriteThumbnailPreview", "dlg-pref-writethumbnailpreview", true),
            Check("AppendFwPng", "dlg-pref-appendfwpng", true),
            // Entwickleroptionen: Testsprache qps auch im Release-Build (Text aus allgemein.ftl)
            Check(crate::language::DEV_PREF_KEY, "lang-dev-options", false),
        ],
        1 => &[
            Check("CropDocCropsImages", "dlg-pref-cropdoccropsimages", true),
            Check("DeletePaths", "dlg-pref-deletepaths", true),
            Check("UseBrushSizeCursors", "dlg-pref-usebrushsizecursors", true),
            Check("UsePrecisionCursors", "dlg-pref-useprecisioncursors", false),
            Caption("dlg-pref-cap-pen"),
            Check("ShowPenPreview", "dlg-pref-showpenpreview", true),
            Check("ShowSolidPoints", "dlg-pref-showsolidpoints", false),
            Caption("dlg-pref-cap-pointer"),
            Check("SelectionMouseOverPreview", "dlg-pref-selectionmouseoverpreview", true),
            Check("PointerToolPaintHandles", "dlg-pref-pointertoolpainthandles", true),
            Check("PreviewDrag", "dlg-pref-previewdrag", true),
            Int("PickDistance", "dlg-pref-pickdistance", 5, 1, 10),
            Check("UngroupAutoshapes", "dlg-pref-ungroupautoshapes", false),
        ],
        2 => &[
            Caption("dlg-pref-cap-guides"),
            Check("ShowGuides", "dlg-pref-showguides", true),
            Check("LockGuides", "dlg-pref-lockguides", false),
            Check("SnapToGuides", "dlg-pref-snaptoguides", true),
            Color("GuideColor", "dlg-pref-guidecolor", "#00ffff"),
            Color("SliceGuideColor", "dlg-pref-sliceguidecolor", "#ff4a4a"),
            Check("SliceGuidesVisible", "dlg-pref-sliceguidesvisible", true),
            Color("NineSliceGuideColor", "dlg-pref-ninesliceguidecolor", "#0000ff"),
            Caption("dlg-pref-cap-grid"),
            Check("ShowGrid", "dlg-pref-showgrid", false),
            Check("SnapToGrids", "dlg-pref-snaptogrids", false),
            Color("GridColor", "dlg-pref-gridcolor", "#00ffff"),
            Int("GridHorizontalDistance", "dlg-pref-gridhorizontaldistance", 36, 1, 10000),
            Int("GridVerticalDistance", "dlg-pref-gridverticaldistance", 36, 1, 10000),
            Caption("dlg-pref-cap-smartguides"),
            Check("ShowSmartGuides", "dlg-pref-showsmartguides", true),
            Check("SnapToSmartGuides", "dlg-pref-snaptosmartguides", true),
            Color("SmartGuideColor", "dlg-pref-smartguidecolor", "#ff4aff"),
            Int("SnapDistance", "dlg-pref-snapdistance", 5, 1, 50),
        ],
        3 => &[
            Real("DefaultLeading", "dlg-pref-defaultleading", 1.2, 0.0, 10.0),
            Real("DefaultBaselineShift", "dlg-pref-defaultbaselineshift", 0.0, -999.0, 999.0),
            Check("ShowFontNamesInEnglish", "dlg-pref-showfontnamesinenglish", false),
            Check("ShowFontPreview", "dlg-pref-showfontpreview", true),
            Check("ShowRecentFonts", "dlg-pref-showrecentfonts", true),
            Int("RecentFontLimit", "dlg-pref-recentfontlimit", 6, 0, 20),
            Text("DefaultMissingFont", "dlg-pref-defaultmissingfont", "Arial"),
        ],
        4 => &[
            Check("ShowPSImportDialog", "dlg-pref-showpsimportdialog", true),
            Check("ShowPSOpenDialog", "dlg-pref-showpsopendialog", true),
            Check("PSImageLayersEditable", "dlg-pref-psimagelayerseditable", true),
            Check("PSTextEditable", "dlg-pref-pstexteditable", true),
            Check("PSShapeLayersEditable", "dlg-pref-psshapelayerseditable", true),
            Check("PSShapeLayerEffectsEditable", "dlg-pref-psshapelayereffectseditable", true),
            Check("PSSharedLayers", "dlg-pref-pssharedlayers", false),
            Check("UseFWFilters", "dlg-pref-usefwfilters", false),
            Check("PSClippingMasksEditable", "dlg-pref-psclippingmaskseditable", true),
            Check("PSRetainAdjustmentLayersAppearance", "dlg-pref-psretainadjustmentlayersappearance", true),
        ],
        5 => &[
            Item::Caption("dlg-pref-cap-plugins"),
            Check("UseExtraPhotoshopPluginsDir", "dlg-pref-useextraphotoshoppluginsdir", false),
            Text("ExtraPhotoshopPluginsDir", "dlg-pref-extraphotoshoppluginsdir", ""),
            Check("UseExtraTexturesDir", "dlg-pref-useextratexturesdir", false),
            Text("ExtraTexturesDir", "dlg-pref-extratexturesdir", ""),
            Check("UseExtraPatternsDir", "dlg-pref-useextrapatternsdir", false),
            Text("ExtraPatternsDir", "dlg-pref-extrapatternsdir", ""),
            Item::Caption("dlg-pref-cap-originaldir"),
            Text(ORIGINAL_DIR_KEY, "dlg-pref-originalresourcedir", ""),
        ],
        _ => &[Combo1("LegLookForPNG", "dlg-pref-leglookforpng", LOOK, 3), Combo1("LoptLookForPNG", "dlg-pref-loptlookforpng", LOOK, 3)],
    }
}

/// Zustand des Dialogs (Änderungen erst mit OK übernehmen).
#[derive(Clone, Debug, PartialEq)]
pub struct PreferencesDialog {
    pub category: usize,
    pub theme: ThemeKind,
    /// Geänderte Werte (Schlüssel → Wert).
    pub values: BTreeMap<String, PrefValue>,
    pub reset_warnings: bool,
}

impl PreferencesDialog {
    /// Werte aus Sitzung und Oberfläche.
    pub fn new(st: &AppState) -> PreferencesDialog {
        PreferencesDialog { category: 0, theme: st.settings.theme, values: BTreeMap::new(), reset_warnings: false }
    }

    fn get(&self, prefs: &Preferences, key: &str) -> Option<PrefValue> {
        self.values.get(key).cloned().or_else(|| prefs.get(key))
    }

    /// Änderungen in die Voreinstellungen schreiben.
    pub fn apply(&self, prefs: &mut Preferences) {
        for (k, v) in &self.values {
            prefs.set(k, v.clone());
        }
        if self.reset_warnings {
            prefs.reset_warnings();
        }
        apply_original_dir(prefs);
    }
}

/// Firepixel-eigene Voreinstellung: Ressourcenordner des Originals (CMYK-/JPEG-Tabellen, Texturen,
/// Bibliotheken; nur zur Laufzeit gelesen). Leer = automatische Suche (`fp_color::install`).
pub const ORIGINAL_DIR_KEY: &str = "OriginalResourceDir";

/// Überträgt die Voreinstellung an `fp_color::install` (beim Start und nach „OK“).
pub fn apply_original_dir(prefs: &Preferences) {
    let dir = prefs.get_str(ORIGINAL_DIR_KEY, "");
    let dir = dir.trim();
    fp_color::install::set_configured_dir((!dir.is_empty()).then(|| std::path::PathBuf::from(dir)));
}

fn draw_item(ui: &mut Ui, d: &mut PreferencesDialog, prefs: &Preferences, item: &Item, r: Rect, ctrl_x: f32, st: &AppState) {
    let tl = |k: &str| t(k);
    match *item {
        Item::Check(k, label, def) => {
            let v = d.get(prefs, k).and_then(|v| v.as_bool()).unwrap_or(def);
            if let Some(b) = fields::checkbox(ui, ("pref", k), Rect::from_min_size(r.min, vec2(r.width(), 18.0)), &tl(label), v, true) {
                d.values.insert(k.into(), PrefValue::Bool(b));
            }
        }
        Item::Int(k, label, def, lo, hi) => {
            super::fit_label(ui, Rect::from_min_size(r.min, vec2(ctrl_x - r.left(), 20.0)), &tl(label), true);
            let v = d.get(prefs, k).and_then(|v| v.as_int()).unwrap_or(def);
            if let Some(n) = fields::number(ui, ("pref", k), Rect::from_min_size(pos2(ctrl_x, r.top()), vec2(60.0, 20.0)), None, Some(v as f64), (lo as f64, hi as f64), 0, true) {
                d.values.insert(k.into(), PrefValue::Int(n as i64));
            }
        }
        Item::Real(k, label, def, lo, hi) => {
            super::fit_label(ui, Rect::from_min_size(r.min, vec2(ctrl_x - r.left(), 20.0)), &tl(label), true);
            let v = d.get(prefs, k).and_then(|v| v.as_real()).unwrap_or(def);
            if let Some(n) = fields::number(ui, ("pref", k), Rect::from_min_size(pos2(ctrl_x, r.top()), vec2(60.0, 20.0)), None, Some(v), (lo, hi), 2, true) {
                d.values.insert(k.into(), PrefValue::Real(n));
            }
        }
        Item::Color(k, label, def) => {
            super::fit_label(ui, Rect::from_min_size(r.min, vec2(ctrl_x - r.left(), 20.0)), &tl(label), true);
            let v = d.get(prefs, k).and_then(|v| v.as_color()).or_else(|| fp_color::Rgba8::parse_hex(def).map(|c| c.to_argb()));
            if let Some(crate::inspector::popups::ColorChoice::Color(c)) = crate::inspector::popups::color_button(ui, ("pref", k), Rect::from_min_size(pos2(ctrl_x, r.top()), vec2(26.0, 20.0)), st, v, true, false, false) {
                d.values.insert(k.into(), PrefValue::Str(format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b())));
            }
        }
        Item::Combo1(k, label, items, def) => {
            super::fit_label(ui, Rect::from_min_size(r.min, vec2(ctrl_x - r.left(), 20.0)), &tl(label), true);
            let v = d.get(prefs, k).and_then(|v| v.as_int()).unwrap_or(def).clamp(1, items.len() as i64);
            let names: Vec<String> = items.iter().map(|k| t(k)).collect();
            let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
            if let Some(i) = fields::combo(ui, ("pref", k), Rect::from_min_size(pos2(ctrl_x, r.top()), vec2((r.right() - ctrl_x).min(combo_w(ui.ctx())), 20.0)), &refs, Some((v - 1) as usize), true) {
                d.values.insert(k.into(), PrefValue::Int(i as i64 + 1));
            }
        }
        Item::Text(k, label, def) => {
            super::fit_label(ui, Rect::from_min_size(r.min, vec2(ctrl_x - r.left(), 20.0)), &tl(label), true);
            let v = d.get(prefs, k).and_then(|v| v.as_str()).unwrap_or_else(|| def.to_string());
            if let Some(t) = fields::text(ui, ("pref", k), Rect::from_min_size(pos2(ctrl_x, r.top()), vec2((r.right() - ctrl_x).min(220.0), 20.0)), &v, true) {
                d.values.insert(k.into(), PrefValue::Str(t));
            }
        }
        Item::Caption(label) => {
            let th = Theme::current(ui.ctx());
            ui.painter().text(pos2(r.left(), r.top() + 10.0), Align2::LEFT_CENTER, tl(label), th.bold(th.fonts.ui), th.colors.text);
        }
    }
}

/// Breite der Kombifelder (längster Eintrag, mindestens wie bisher 220 px).
fn combo_w(ctx: &egui::Context) -> f32 {
    let names: Vec<String> = LOOK.iter().chain(INTERP.iter()).map(|k| t(k)).collect();
    let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
    220f32.max(super::max_text_w(ctx, &refs) + 28.0).ceil()
}

/// Maße des Dialogs in der aktuellen Sprache: Kategorienliste, Beschriftungsspalte und
/// Dialoggröße wachsen mit den Texten (Deutsch: wie bisher 170 / 230 px, 640 px breit).
struct Layout {
    list_w: f32,
    ctrl_off: f32,
    reset_w: f32,
    size: egui::Vec2,
}

impl Layout {
    fn measure(ctx: &egui::Context) -> Layout {
        let tw = |k: &str| super::text_w(ctx, &t(k));
        let list_w = CATEGORIES.iter().map(|k| tw(k) + 14.0).fold(170.0, f32::max).ceil();
        let mut label_w: f32 = tw("dlg-pref-theme").max(tw("lang-label"));
        let mut page_w: f32 = 0.0;
        let mut page_h: f32 = 0.0;
        let cw = combo_w(ctx);
        for cat in 0..CATEGORIES.len() {
            let items = page_items(cat);
            for it in items {
                match *it {
                    Item::Check(_, l, _) => page_w = page_w.max(tw(l) + 20.0),
                    Item::Caption(l) => page_w = page_w.max(tw(l) * 1.1 + 4.0),
                    Item::Int(_, l, ..) | Item::Real(_, l, ..) | Item::Color(_, l, _) | Item::Combo1(_, l, ..) | Item::Text(_, l, _) => label_w = label_w.max(tw(l)),
                }
            }
            let extra = match cat {
                0 => 6.0 + 24.0 + 26.0 + 30.0 + 22.0,
                5 => 12.0 + 40.0,
                _ => 0.0,
            };
            page_h = page_h.max(28.0 + items.len() as f32 * 24.0 + extra);
        }
        let ctrl_off = 230f32.max(label_w + 10.0).ceil();
        let reset_w = 220f32.max(tw("dlg-pref-reset-warnings") + 16.0).ceil();
        // Kombifelder: bisher auf die Seitenbreite gekürzt (198 px); mehr Platz nur für längere Einträge
        page_w = page_w.max(ctrl_off + if cw > 220.0 { cw } else { 198.0 }).max(reset_w).max(super::text_w(ctx, &t("dlg-pref-cap-ui")) * 1.1);
        let w = 640f32.max(12.0 + list_w + 16.0 + page_w + 14.0).ceil();
        // Seiten mit vielen Einträgen: Platz bis über die Knöpfe
        let h = 500f32.max(12.0 + page_h + 60.0).ceil();
        Layout { list_w, ctrl_off, reset_w, size: vec2(w, h) }
    }
}

/// Zeigt den Dialog.
pub fn show(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.dialogs.preferences.clone() else { return };
    let mut result = None;
    let lay = Layout::measure(ctx);
    let (ok_l, cancel_l) = (t("btn-ok"), t("btn-cancel"));
    let r = dialog_frame(ctx, "fp-prefs", &t("dlg-pref-title"), lay.size, |ui, body| {
        let theme = Theme::current(ui.ctx());
        let c = &theme.colors;
        let list = Rect::from_min_size(body.min + vec2(12.0, 12.0), vec2(lay.list_w, body.height() - 60.0));
        ui.painter().rect_filled(list, 0.0, c.list_bg);
        ui.painter().rect_stroke(list, 0.0, Stroke::new(1.0, c.list_border), StrokeKind::Inside);
        for (i, cat) in CATEGORIES.iter().enumerate() {
            let rr = Rect::from_min_size(list.min + vec2(1.0, 1.0 + i as f32 * 21.0), vec2(list.width() - 2.0, 21.0));
            let resp = ui.interact(rr, egui::Id::new(("prefcat", i)), Sense::click());
            if d.category == i {
                ui.painter().rect_filled(rr, 0.0, c.list_selected_bg);
            } else if resp.hovered() {
                ui.painter().rect_filled(rr, 0.0, c.list_hover_bg);
            }
            ui.painter().text(pos2(rr.left() + 6.0, rr.center().y), Align2::LEFT_CENTER, t(cat), theme.ui_font(), c.text);
            if resp.clicked() {
                d.category = i;
            }
        }
        let page = Rect::from_min_max(pos2(list.right() + 16.0, list.top()), pos2(body.right() - 14.0, list.bottom()));
        ui.painter().text(page.min, Align2::LEFT_TOP, t(CATEGORIES[d.category]), theme.bold(theme.fonts.heading), c.text);
        let mut y = page.top() + 28.0;
        let ctrl_x = page.left() + lay.ctrl_off;
        let prefs = st.session.prefs.clone();
        for it in page_items(d.category) {
            let h = 24.0;
            draw_item(ui, &mut d, &prefs, it, Rect::from_min_size(pos2(page.left(), y), vec2(page.width(), 22.0)), ctrl_x, st);
            y += h;
        }
        if d.category == 0 {
            y += 6.0;
            ui.painter().text(pos2(page.left(), y + 10.0), Align2::LEFT_CENTER, t("dlg-pref-cap-ui"), theme.bold(theme.fonts.ui), c.text);
            y += 24.0;
            super::fit_label(ui, Rect::from_min_size(pos2(page.left(), y), vec2(ctrl_x - page.left(), 20.0)), &t("dlg-pref-theme"), true);
            let names: Vec<String> = ThemeKind::ALL.iter().map(|k| k.name()).collect();
            let cur = ThemeKind::ALL.iter().position(|k| *k == d.theme);
            let refs = super::refs(&names);
            if let Some(i) = fields::combo(ui, "pref-theme", Rect::from_min_size(pos2(ctrl_x, y), vec2(super::combo_fit_w(ui.ctx(), &refs, 140.0), 20.0)), &refs, cur, true) {
                d.theme = ThemeKind::ALL[i];
            }
            // Sprache (fp_ui::language): Eigennamen aller Sprachen, Auswahl als values["Language"]
            y += 26.0;
            crate::language::pref_row(ui, Rect::from_min_size(pos2(page.left(), y), vec2(ctrl_x - page.left(), 20.0)), Rect::from_min_size(pos2(ctrl_x, y), vec2(180.0, 20.0)), &mut d.values, &prefs);
            y += 30.0;
            let reset = t("dlg-pref-reset-warnings");
            if fields::button(ui, "pref-resetwarn", Rect::from_min_size(pos2(page.left(), y), vec2(lay.reset_w, 22.0)), &reset, !d.reset_warnings) {
                d.reset_warnings = true;
            }
        }
        if d.category == 5 {
            // Hinweis umbrechen (war eine lange Zeile)
            super::text_block(ui, Rect::from_min_size(pos2(page.left(), y + 12.0), vec2(page.width(), 40.0)), egui::RichText::new(t("dlg-pref-plugins-note")).font(theme.small_font()).color(c.text_weak));
        }
        let by = body.bottom() - 36.0;
        let b = super::button_row(ui.ctx(), body.right() - 16.0, by, &[&ok_l, &cancel_l], 76.0, 23.0, 8.0);
        if push_button(ui, b[0], &ok_l, true, true) {
            result = Some(true);
        }
        if push_button(ui, b[1], &cancel_l, false, true) {
            result = Some(false);
        }
    });
    match (result, r) {
        (Some(true), _) => {
            st.dialogs.preferences = None;
            d.apply(&mut st.session.prefs);
            st.session.prefs_changed();
            // Sprachwechsel sofort, ohne Neustart
            crate::language::apply_prefs(st, ctx);
            st.push(Action::SetTheme(d.theme));
        }
        (Some(false), _) | (_, DialogClose::Cancelled) => st.dialogs.preferences = None,
        _ => st.dialogs.preferences = Some(d),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_category_has_items_and_known_keys() {
        for c in 0..CATEGORIES.len() {
            assert!(!page_items(c).is_empty());
        }
        let p = Preferences::new();
        for c in 0..CATEGORIES.len() {
            for it in page_items(c) {
                if let Item::Check(k, _, _) | Item::Int(k, _, _, _, _) = it {
                    assert!(fp_engine::prefs::definition(k).is_some() || p.get(k).is_some() || *k == crate::language::DEV_PREF_KEY, "unbekannter Schlüssel {k}");
                }
            }
        }
    }
}
