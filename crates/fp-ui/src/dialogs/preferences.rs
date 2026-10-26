//! Voreinstellungen (Dialog 4419, Kategorieliste 4423, Seiten 4426–4432, 14 §3.4): alle sieben
//! Kategorien mit den Schlüsseln und Vorgaben aus 14 §3.4, dazu unter „Allgemein ▸
//! Benutzeroberfläche“ die Design-Wahl Normal/Dunkel/OLED. Änderungen erst mit OK; danach
//! sofort gespeichert (`SavePrefsWhileRunning`).

use super::{DialogClose, dialog_frame, push_button, t};
use crate::app_state::{Action, AppState};
use crate::inspector::fields;
use crate::theme::{Theme, ThemeKind};
use egui::{Align2, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use fp_engine::{PrefValue, Preferences};
use std::collections::BTreeMap;

/// Kategorien in der Reihenfolge der Liste (wie im Original: „Starten und Bearbeiten“ vor
/// „Zusatzmodule“, PM-5 D16).
/// (Schlüssel der Texte in `dialogs.ftl`; ebenso die Beschriftungen der Einträge.)
pub const CATEGORIES: [&str; 7] = [
    "dlg-pref-cat-general",
    "dlg-pref-cat-edit",
    "dlg-pref-cat-guides",
    "dlg-pref-cat-type",
    "dlg-pref-cat-psimport",
    "dlg-pref-cat-launch",
    "dlg-pref-cat-plugins",
];

/// Index der Kategorie „Zusatzmodule“ (Hinweis unter den Feldern).
const CAT_PLUGINS: usize = 6;
/// Index der Kategorie „Text“ (Neustart-Hinweis unter den Feldern wie im Original, TW-47).
const CAT_TYPE: usize = 3;

/// Ein Eintrag einer Seite (Voreinstellungsschlüssel, Textschlüssel der Beschriftung, …).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Item {
    Check(&'static str, &'static str, bool),
    Int(&'static str, &'static str, i64, i64, i64),
    Color(&'static str, &'static str, &'static str),
    /// 1-basiertes Kombifeld (`EditTransformMode` usw.).
    Combo1(&'static str, &'static str, &'static [&'static str], i64),
    Text(&'static str, &'static str, &'static str),
    Caption(&'static str),
    /// Faktor, angezeigt in Prozent wie im Original („Zeilenabstand: 120“, TW-47).
    Pct(&'static str, &'static str, f64, f64, f64),
    /// Zahl mit „px“ hinter dem Feld (Grundlinienverschiebung).
    Px(&'static str, &'static str, f64, f64, f64),
    /// Kästchen mit Zahlenfeld in derselben Zeile (Anzahl der zuletzt verwendeten Schriften).
    CheckInt(
        &'static str,
        &'static str,
        bool,
        &'static str,
        i64,
        i64,
        i64,
    ),
    /// Kästchen mit Kombifeld in derselben Zeile; gespeichert wird der **angezeigte Text** des
    /// gewählten Eintrags (`FontPreviewSize`, re §3.4: Index als lokalisierter Text, unbekannter
    /// Text bzw. leer = Eintrag `def`).
    CheckText(
        &'static str,
        &'static str,
        bool,
        &'static str,
        &'static [&'static str],
        usize,
    ),
    /// Kästchen mit „Durchsuchen...“ rechts (Ordner der Zusatzmodule); der gewählte Ordner steht
    /// klein darunter.
    CheckBrowse(&'static str, &'static str, bool, &'static str),
}

/// Interpolationsverfahren (auch „Bildgröße“).
pub(crate) const INTERP: &[&str] = &[
    "dlg-interp-bicubic",
    "dlg-interp-bilinear",
    "dlg-interp-soft",
    "dlg-interp-nearest",
];
const LOOK: &[&str] = &[
    "dlg-pref-look-always",
    "dlg-pref-look-never",
    "dlg-pref-look-ask",
];
/// Größe der Schriftvorschau (Kombifeld 4532: Klein / Mittelgroß / Groß).
pub(crate) const FONT_PREVIEW: &[&str] = &[
    "dlg-pref-fontpreview-small",
    "dlg-pref-fontpreview-medium",
    "dlg-pref-fontpreview-large",
];

/// Eintrag des Kombifelds zu einem gespeicherten Text (re §3.4: „Klein“ → 0, „Groß“ → 2, sonst
/// Mittelgroß); auch Texte der anderen Sprache werden erkannt.
pub fn font_preview_index(stored: &str) -> usize {
    let s = stored.trim();
    let is = |k: &str| {
        ["de", "en"]
            .iter()
            .any(|l| fp_i18n::with_language(l, || fp_i18n::t(k)) == s)
    };
    if is(FONT_PREVIEW[0]) {
        0
    } else if is(FONT_PREVIEW[2]) {
        2
    } else {
        1
    }
}

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
            Check(
                "IconPanelsAutoCollapse",
                "dlg-pref-iconpanelsautocollapse",
                false,
            ),
            Check(
                "WriteThumbnailPreview",
                "dlg-pref-writethumbnailpreview",
                true,
            ),
            Check("AppendFwPng", "dlg-pref-appendfwpng", true),
            // Entwickleroptionen: Testsprache qps auch im Release-Build (Text aus common.ftl)
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
            Check(
                "SelectionMouseOverPreview",
                "dlg-pref-selectionmouseoverpreview",
                true,
            ),
            Check(
                "PointerToolPaintHandles",
                "dlg-pref-pointertoolpainthandles",
                true,
            ),
            Check("PreviewDrag", "dlg-pref-previewdrag", true),
            Int("PickDistance", "dlg-pref-pickdistance", 5, 1, 10),
            Check("UngroupAutoshapes", "dlg-pref-ungroupautoshapes", false),
            // object selection: preview while hovering (BA-037, own option)
            Check(
                crate::canvas::tools::ai_object::PREF_KEY,
                "dlg-pref-aiobjecthoverpreview",
                false,
            ),
            // Alt-marquee of the Pointer / Subselection deselects (FB-56, own option, off = reference)
            Check(
                fp_engine::tools::ALT_MARQUEE_DESELECT_PREF,
                "dlg-pref-altmarqueedeselects",
                false,
            ),
            // own options (FB-75 to FB-77), off = reference
            Check(
                fp_engine::tools::DUPLICATE_WITH_CTRL_DRAG_PREF,
                "dlg-pref-duplicatewithctrldrag",
                false,
            ),
            Check(
                fp_engine::tools::SHIFT_MARQUEE_ADDS_ONLY_PREF,
                "dlg-pref-shiftmarqueeaddsonly",
                false,
            ),
            Check(
                crate::canvas::SHIFT_WHEEL_HORIZONTAL_PREF,
                "dlg-pref-shiftwheelhorizontal",
                false,
            ),
            // own option (FB-94), off = reference
            Check(
                fp_engine::tools::ARROW_KEYS_MOVE_PIXELS_PREF,
                "dlg-pref-arrowkeysmovepixels",
                false,
            ),
        ],
        2 => &[
            Caption("dlg-pref-cap-guides"),
            Check("ShowGuides", "dlg-pref-showguides", true),
            Check("LockGuides", "dlg-pref-lockguides", false),
            Check("SnapToGuides", "dlg-pref-snaptoguides", true),
            Color("GuideColor", "dlg-pref-guidecolor", "#00ffff"),
            Color("SliceGuideColor", "dlg-pref-sliceguidecolor", "#ff4a4a"),
            Check("SliceGuidesVisible", "dlg-pref-sliceguidesvisible", true),
            Color(
                "NineSliceGuideColor",
                "dlg-pref-ninesliceguidecolor",
                "#0000ff",
            ),
            Caption("dlg-pref-cap-grid"),
            Check("ShowGrid", "dlg-pref-showgrid", false),
            Check("SnapToGrids", "dlg-pref-snaptogrids", false),
            Color("GridColor", "dlg-pref-gridcolor", "#00ffff"),
            Int(
                "GridHorizontalDistance",
                "dlg-pref-gridhorizontaldistance",
                36,
                1,
                10000,
            ),
            Int(
                "GridVerticalDistance",
                "dlg-pref-gridverticaldistance",
                36,
                1,
                10000,
            ),
            Caption("dlg-pref-cap-smartguides"),
            Check("ShowSmartGuides", "dlg-pref-showsmartguides", true),
            Check("SnapToSmartGuides", "dlg-pref-snaptosmartguides", true),
            Color("SmartGuideColor", "dlg-pref-smartguidecolor", "#ff4aff"),
            Int("SnapDistance", "dlg-pref-snapdistance", 5, 1, 50),
        ],
        3 => &[
            // Wortlaut und Gliederung wie im Original (TW-47)
            Pct("DefaultLeading", "dlg-pref-defaultleading", 1.2, 0.0, 10.0),
            Px(
                "DefaultBaselineShift",
                "dlg-pref-defaultbaselineshift",
                0.0,
                -999.0,
                999.0,
            ),
            Caption("dlg-pref-cap-options"),
            Check(
                "ShowFontNamesInEnglish",
                "dlg-pref-showfontnamesinenglish",
                false,
            ),
            CheckText(
                "ShowFontPreview",
                "dlg-pref-showfontpreview",
                true,
                "FontPreviewSize",
                FONT_PREVIEW,
                1,
            ),
            CheckInt(
                "ShowRecentFonts",
                "dlg-pref-showrecentfonts",
                true,
                "RecentFontLimit",
                6,
                0,
                20,
            ),
            Caption("dlg-pref-cap-missingfont"),
            Text("DefaultMissingFont", "dlg-pref-defaultmissingfont", "Arial"),
        ],
        4 => &[
            Check("ShowPSImportDialog", "dlg-pref-showpsimportdialog", true),
            Check("ShowPSOpenDialog", "dlg-pref-showpsopendialog", true),
            Check(
                "PSImageLayersEditable",
                "dlg-pref-psimagelayerseditable",
                true,
            ),
            Check("PSTextEditable", "dlg-pref-pstexteditable", true),
            Check(
                "PSShapeLayersEditable",
                "dlg-pref-psshapelayerseditable",
                true,
            ),
            Check(
                "PSShapeLayerEffectsEditable",
                "dlg-pref-psshapelayereffectseditable",
                true,
            ),
            Check("PSSharedLayers", "dlg-pref-pssharedlayers", false),
            Check("UseFWFilters", "dlg-pref-usefwfilters", false),
            Check(
                "PSClippingMasksEditable",
                "dlg-pref-psclippingmaskseditable",
                true,
            ),
            Check(
                "PSRetainAdjustmentLayersAppearance",
                "dlg-pref-psretainadjustmentlayersappearance",
                true,
            ),
        ],
        5 => &[
            Combo1("LegLookForPNG", "dlg-pref-leglookforpng", LOOK, 3),
            Combo1("LoptLookForPNG", "dlg-pref-loptlookforpng", LOOK, 3),
        ],
        _ => &[
            // wie im Original: Kästchen mit „Durchsuchen...“ je Ordner (TW-47)
            CheckBrowse(
                "UseExtraPhotoshopPluginsDir",
                "dlg-pref-useextraphotoshoppluginsdir",
                false,
                "ExtraPhotoshopPluginsDir",
            ),
            CheckBrowse(
                "UseExtraTexturesDir",
                "dlg-pref-useextratexturesdir",
                false,
                "ExtraTexturesDir",
            ),
            CheckBrowse(
                "UseExtraPatternsDir",
                "dlg-pref-useextrapatternsdir",
                false,
                "ExtraPatternsDir",
            ),
            Item::Caption("dlg-pref-cap-originaldir"),
            Text(ORIGINAL_DIR_KEY, "dlg-pref-originalresourcedir", ""),
            // the same setting as the Batch Studio's settings ("ffmpeg file…", BA-107)
            Item::Caption("dlg-pref-cap-ffmpeg"),
            Text(FFMPEG_KEY, "dlg-pref-ffmpeg", ""),
        ],
    }
}

/// Zeile der Seite „Allgemein“ (Gruppen wie im Original, PM-5 D16).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GeneralRow {
    /// Beschriftung rechtsbündig links der Steuerspalte (Textschlüssel oder keine), dann ein
    /// Eintrag aus `page_items(0)` (Schlüssel der Voreinstellung).
    Item(Option<&'static str>, &'static str),
    /// Rückgängig-Schritte mit Neustart-Hinweis rechts daneben.
    Undo,
    /// Farbfelder mit Beschriftung rechts (Schlüssel der Voreinstellung, Textschlüssel).
    Colors(&'static [(&'static str, &'static str)]),
    /// Design-Auswahl (Firepixel).
    Theme,
    /// Sprache (Firepixel).
    Language,
    /// „Alle Warnmeldungen zurücksetzen“.
    ResetWarnings,
}

/// Gruppen der Seite „Allgemein“: Überschrift mit Trennlinie (Textschlüssel, `None` = ohne) und Zeilen.
pub const GENERAL_GROUPS: &[(Option<&str>, &[GeneralRow])] = &[
    (
        None,
        &[
            GeneralRow::Item(Some("dlg-pref-grp-docoptions"), "ShowStartPage"),
            GeneralRow::Item(None, "XformToolXformsBFE"),
            GeneralRow::Undo,
            GeneralRow::Item(None, "EditTransformMode"),
            GeneralRow::Item(None, "OSBasedResample"),
        ],
    ),
    (
        Some("dlg-pref-grp-save"),
        &[
            GeneralRow::Item(None, "WriteThumbnailPreview"),
            GeneralRow::Item(None, "AppendFwPng"),
        ],
    ),
    (
        Some("dlg-pref-cap-ui"),
        &[
            GeneralRow::Item(Some("dlg-pref-grp-workspace"), "IconPanelsAutoCollapse"),
            GeneralRow::Theme,
            GeneralRow::Language,
            GeneralRow::Item(None, crate::language::DEV_PREF_KEY),
        ],
    ),
    (
        Some("dlg-pref-grp-colors"),
        &[
            GeneralRow::Colors(&[
                ("BrushDefaultColor", "dlg-pref-color-stroke"),
                ("FillDefaultColor", "dlg-pref-color-fill"),
            ]),
            GeneralRow::Colors(&[("FeedbackColor", "dlg-pref-color-highlight")]),
            GeneralRow::ResetWarnings,
        ],
    ),
];

/// Überschriften der Gruppen auf „Allgemein“ (übersetzt, in der Reihenfolge der Seite).
pub fn general_group_titles() -> Vec<String> {
    GENERAL_GROUPS
        .iter()
        .filter_map(|(g, _)| g.map(t))
        .collect()
}

fn general_item(key: &str) -> Option<&'static Item> {
    page_items(0).iter().find(|it| match it {
        Item::Check(k, ..)
        | Item::Int(k, ..)
        | Item::Color(k, ..)
        | Item::Combo1(k, ..)
        | Item::Text(k, ..)
        | Item::Pct(k, ..)
        | Item::Px(k, ..)
        | Item::CheckInt(k, ..)
        | Item::CheckText(k, ..)
        | Item::CheckBrowse(k, ..) => *k == key,
        Item::Caption(_) => false,
    })
}

/// Zeilenabstand der Seite „Allgemein“ (Steuerelemente 20–22 px hoch).
const GENERAL_ROW: f32 = 21.0;
/// Höhe einer Gruppenüberschrift mit Trennlinie.
const GENERAL_HEAD: f32 = 20.0;

/// Zeilenhöhe eines Eintrags der übrigen Seiten: Kästchen und Überschriften 18 px (Kästchen 16 px
/// hoch), Zeilen mit Feldern 21 px (Felder 20 px) – so passt auch „Hilfslinien und Raster“ mit
/// 19 Einträgen in die Höhe des Originals (CT-1a).
fn item_h(it: &Item) -> f32 {
    match it {
        Item::Check(..) | Item::Caption(_) => 18.0,
        Item::CheckInt(..) | Item::CheckText(..) => 23.0,
        Item::CheckBrowse(..) => 40.0,
        _ => 21.0,
    }
}

/// Höhe der Seite „Allgemein“ ab der Überschrift.
fn general_height() -> f32 {
    GENERAL_GROUPS.iter().map(|(g, rows)| if g.is_some() { GENERAL_HEAD } else { 0.0 } + rows.len() as f32 * GENERAL_ROW).sum::<f32>() + 28.0
}

/// Eintrag ohne eigene Beschriftung (die steht rechtsbündig links der Steuerspalte).
fn strip_label(it: &Item) -> Item {
    match *it {
        Item::Int(k, _, a, b, c) => Item::Int(k, "", a, b, c),
        Item::Combo1(k, _, items, def) => Item::Combo1(k, "", items, def),
        other => other,
    }
}

/// Zeichnet die Seite „Allgemein“ in Gruppen.
fn draw_general(
    ui: &mut Ui,
    d: &mut PreferencesDialog,
    prefs: &Preferences,
    page: Rect,
    ctrl_x: f32,
    reset_w: f32,
    st: &AppState,
) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let mut y = page.top() + 28.0;
    for (group, rows) in GENERAL_GROUPS {
        if let Some(g) = group {
            let txt = t(g);
            let gw = super::text_w(ui.ctx(), &txt) * 1.1;
            ui.painter().text(
                pos2(page.left(), y + 10.0),
                Align2::LEFT_CENTER,
                &txt,
                theme.bold(theme.fonts.ui),
                c.text,
            );
            ui.painter().hline(
                (page.left() + gw + 6.0)..=page.right(),
                y + 11.0,
                Stroke::new(1.0, c.list_border),
            );
            y += GENERAL_HEAD;
        }
        for row in *rows {
            let label = |ui: &mut Ui, k: &str| {
                let txt = t(k);
                ui.painter().text(
                    pos2(ctrl_x - 6.0, y + 10.0),
                    Align2::RIGHT_CENTER,
                    &txt,
                    theme.ui_font(),
                    c.text,
                );
            };
            let full = Rect::from_min_size(pos2(ctrl_x, y), vec2(page.right() - ctrl_x, 22.0));
            match *row {
                GeneralRow::Item(l, key) => {
                    if let Some(l) = l {
                        label(ui, l);
                    }
                    if let Some(it) = general_item(key) {
                        if let Item::Combo1(_, lk, ..) | Item::Int(_, lk, ..) = it {
                            label(ui, lk);
                        }
                        draw_item(ui, d, prefs, &strip_label(it), full, ctrl_x, st);
                    }
                }
                GeneralRow::Undo => {
                    label(ui, "dlg-pref-maxundos");
                    if let Some(it) = general_item("MaxUndos") {
                        draw_item(ui, d, prefs, &strip_label(it), full, ctrl_x, st);
                    }
                    crate::widgets::text_clipped(
                        ui.painter(),
                        pos2(ctrl_x + 68.0, y + 10.0),
                        Align2::LEFT_CENTER,
                        &t("dlg-pref-restart-note"),
                        theme.ui_font(),
                        c.text,
                        page.right() - ctrl_x - 68.0,
                    );
                }
                GeneralRow::Colors(list) => {
                    let mut x = ctrl_x;
                    for (key, lk) in list {
                        if let Some(Item::Color(k, _, def)) = general_item(key) {
                            let v = d
                                .get(prefs, k)
                                .and_then(|v| v.as_color())
                                .or_else(|| fp_color::Rgba8::parse_hex(def).map(|c| c.to_argb()));
                            if let Some(crate::inspector::popups::ColorChoice::Color(cc)) =
                                crate::inspector::popups::color_button(
                                    ui,
                                    ("pref", *k),
                                    Rect::from_min_size(pos2(x, y), vec2(26.0, 20.0)),
                                    st,
                                    v,
                                    true,
                                    false,
                                    false,
                                )
                            {
                                d.values.insert(
                                    (*k).into(),
                                    PrefValue::Str(format!(
                                        "#{:02x}{:02x}{:02x}",
                                        cc.r(),
                                        cc.g(),
                                        cc.b()
                                    )),
                                );
                            }
                            let txt = t(lk);
                            ui.painter().text(
                                pos2(x + 32.0, y + 10.0),
                                Align2::LEFT_CENTER,
                                &txt,
                                theme.ui_font(),
                                c.text,
                            );
                            x += 32.0 + super::text_w(ui.ctx(), &txt).max(80.0) + 20.0;
                        }
                    }
                }
                GeneralRow::Theme => {
                    label(ui, "dlg-pref-theme");
                    let names: Vec<String> = ThemeKind::ALL.iter().map(|k| k.name()).collect();
                    let cur = ThemeKind::ALL.iter().position(|k| *k == d.theme);
                    let refs = super::refs(&names);
                    if let Some(i) = fields::combo(
                        ui,
                        "pref-theme",
                        Rect::from_min_size(
                            pos2(ctrl_x, y),
                            vec2(super::combo_fit_w(ui.ctx(), &refs, 140.0), 20.0),
                        ),
                        &refs,
                        cur,
                        true,
                    ) {
                        d.theme = ThemeKind::ALL[i];
                    }
                }
                GeneralRow::Language => {
                    // Sprache (fp_ui::language): Eigennamen aller Sprachen, Auswahl als values["Language"]
                    // „Sprache:“ rechtsbündig vor dem Feld wie die übrigen Beschriftungen (CT-1a)
                    let lw = super::text_w(ui.ctx(), &t("lang-label"));
                    crate::language::pref_row(
                        ui,
                        Rect::from_min_size(pos2(ctrl_x - 6.0 - lw, y), vec2(lw, 20.0)),
                        Rect::from_min_size(pos2(ctrl_x, y), vec2(180.0, 20.0)),
                        &mut d.values,
                        prefs,
                    );
                }
                GeneralRow::ResetWarnings => {
                    let reset = t("dlg-pref-reset-warnings");
                    if fields::button(
                        ui,
                        "pref-resetwarn",
                        Rect::from_min_size(pos2(ctrl_x, y), vec2(reset_w, 22.0)),
                        &reset,
                        !d.reset_warnings,
                    ) {
                        d.reset_warnings = true;
                    }
                }
            }
            y += GENERAL_ROW;
        }
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
    /// „Durchsuchen...“ gedrückt: Ordner für diesen Schlüssel wählen (nach dem Zeichnen).
    pub browse: Option<String>,
}

impl PreferencesDialog {
    /// Werte aus Sitzung und Oberfläche.
    pub fn new(st: &AppState) -> PreferencesDialog {
        PreferencesDialog {
            category: 0,
            theme: st.settings.theme,
            values: BTreeMap::new(),
            reset_warnings: false,
            browse: None,
        }
    }

    fn get(&self, prefs: &Preferences, key: &str) -> Option<PrefValue> {
        if key == FFMPEG_KEY && !self.values.contains_key(key) {
            return Some(PrefValue::Str(
                fp_batch::video::ffmpeg::configured_path()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default(),
            ));
        }
        self.values.get(key).cloned().or_else(|| prefs.get(key))
    }

    /// Änderungen in die Voreinstellungen schreiben.
    pub fn apply(&self, prefs: &mut Preferences) {
        for (k, v) in &self.values {
            if k == FFMPEG_KEY {
                // not a preference: the ffmpeg file of the Batch Studio (`video.json` in the profile)
                let path = v.as_str().unwrap_or_default();
                let path = path.trim().trim_matches('"');
                let r = fp_batch::video::ffmpeg::set_configured_path(
                    (!path.is_empty()).then(|| std::path::Path::new(path)),
                );
                if let Err(e) = r {
                    log::warn!("ffmpeg setting: {e}");
                }
                continue;
            }
            prefs.set(k, v.clone());
        }
        if self.reset_warnings {
            prefs.reset_warnings();
        }
        apply_original_dir(prefs);
    }
}

/// Row "ffmpeg file" (BA-107): not stored in the preferences but in the Batch Studio's video settings
/// (`fp_batch::video::ffmpeg::configured_path`); empty = search automatically.
pub const FFMPEG_KEY: &str = "ffmpegPath";

/// Firepixel-eigene Voreinstellung: Ressourcenordner des Originals (CMYK-/JPEG-Tabellen, Texturen,
/// Bibliotheken; nur zur Laufzeit gelesen). Leer = automatische Suche (`fp_color::install`).
pub const ORIGINAL_DIR_KEY: &str = "OriginalResourceDir";

/// Überträgt die Voreinstellung an `fp_color::install` (beim Start und nach „OK“).
pub fn apply_original_dir(prefs: &Preferences) {
    let dir = prefs.get_str(ORIGINAL_DIR_KEY, "");
    let dir = dir.trim();
    fp_color::install::set_configured_dir((!dir.is_empty()).then(|| std::path::PathBuf::from(dir)));
}

fn draw_item(
    ui: &mut Ui,
    d: &mut PreferencesDialog,
    prefs: &Preferences,
    item: &Item,
    r: Rect,
    ctrl_x: f32,
    st: &AppState,
) {
    let tl = |k: &str| if k.is_empty() { String::new() } else { t(k) };
    match *item {
        Item::Check(k, label, def) => {
            let v = d.get(prefs, k).and_then(|v| v.as_bool()).unwrap_or(def);
            if let Some(b) = fields::checkbox(
                ui,
                ("pref", k),
                Rect::from_min_size(r.min, vec2(r.width(), 18.0)),
                &tl(label),
                v,
                true,
            ) {
                d.values.insert(k.into(), PrefValue::Bool(b));
            }
        }
        Item::Int(k, label, def, lo, hi) => {
            super::fit_label(
                ui,
                Rect::from_min_size(r.min, vec2(ctrl_x - r.left(), 20.0)),
                &tl(label),
                true,
            );
            let v = d.get(prefs, k).and_then(|v| v.as_int()).unwrap_or(def);
            if let Some(n) = fields::number(
                ui,
                ("pref", k),
                Rect::from_min_size(pos2(ctrl_x, r.top()), vec2(60.0, 20.0)),
                None,
                Some(v as f64),
                (lo as f64, hi as f64),
                0,
                true,
            ) {
                d.values.insert(k.into(), PrefValue::Int(n as i64));
            }
        }
        Item::Color(k, label, def) => {
            super::fit_label(
                ui,
                Rect::from_min_size(r.min, vec2(ctrl_x - r.left(), 20.0)),
                &tl(label),
                true,
            );
            let v = d
                .get(prefs, k)
                .and_then(|v| v.as_color())
                .or_else(|| fp_color::Rgba8::parse_hex(def).map(|c| c.to_argb()));
            if let Some(crate::inspector::popups::ColorChoice::Color(c)) =
                crate::inspector::popups::color_button(
                    ui,
                    ("pref", k),
                    Rect::from_min_size(pos2(ctrl_x, r.top()), vec2(26.0, 20.0)),
                    st,
                    v,
                    true,
                    false,
                    false,
                )
            {
                d.values.insert(
                    k.into(),
                    PrefValue::Str(format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b())),
                );
            }
        }
        Item::Combo1(k, label, items, def) => {
            super::fit_label(
                ui,
                Rect::from_min_size(r.min, vec2(ctrl_x - r.left(), 20.0)),
                &tl(label),
                true,
            );
            let v = d
                .get(prefs, k)
                .and_then(|v| v.as_int())
                .unwrap_or(def)
                .clamp(1, items.len() as i64);
            let names: Vec<String> = items.iter().map(|k| t(k)).collect();
            let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
            if let Some(i) = fields::combo(
                ui,
                ("pref", k),
                Rect::from_min_size(
                    pos2(ctrl_x, r.top()),
                    vec2((r.right() - ctrl_x).min(combo_w(ui.ctx())), 20.0),
                ),
                &refs,
                Some((v - 1) as usize),
                true,
            ) {
                d.values.insert(k.into(), PrefValue::Int(i as i64 + 1));
            }
        }
        Item::Text(k, label, def) => {
            super::fit_label(
                ui,
                Rect::from_min_size(r.min, vec2(ctrl_x - r.left(), 20.0)),
                &tl(label),
                true,
            );
            let v = d
                .get(prefs, k)
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| def.to_string());
            if let Some(t) = fields::text(
                ui,
                ("pref", k),
                Rect::from_min_size(
                    pos2(ctrl_x, r.top()),
                    vec2((r.right() - ctrl_x).min(220.0), 20.0),
                ),
                &v,
                true,
            ) {
                d.values.insert(k.into(), PrefValue::Str(t));
            }
        }
        Item::Caption(label) => {
            let th = Theme::current(ui.ctx());
            ui.painter().text(
                pos2(r.left(), r.top() + 10.0),
                Align2::LEFT_CENTER,
                tl(label),
                th.bold(th.fonts.ui),
                th.colors.text,
            );
        }
        Item::Pct(k, label, def, lo, hi) => {
            super::fit_label(
                ui,
                Rect::from_min_size(r.min, vec2(ctrl_x - r.left(), 20.0)),
                &tl(label),
                true,
            );
            let v = d.get(prefs, k).and_then(|v| v.as_real()).unwrap_or(def);
            if let Some(n) = fields::number(
                ui,
                ("pref", k),
                Rect::from_min_size(pos2(ctrl_x, r.top()), vec2(60.0, 20.0)),
                None,
                Some((v * 100.0).round()),
                (lo * 100.0, hi * 100.0),
                0,
                true,
            ) {
                d.values.insert(k.into(), PrefValue::Real(n / 100.0));
            }
        }
        Item::Px(k, label, def, lo, hi) => {
            super::fit_label(
                ui,
                Rect::from_min_size(r.min, vec2(ctrl_x - r.left(), 20.0)),
                &tl(label),
                true,
            );
            let v = d.get(prefs, k).and_then(|v| v.as_real()).unwrap_or(def);
            if let Some(n) = fields::number(
                ui,
                ("pref", k),
                Rect::from_min_size(pos2(ctrl_x, r.top()), vec2(60.0, 20.0)),
                None,
                Some(v),
                (lo, hi),
                2,
                true,
            ) {
                d.values.insert(k.into(), PrefValue::Real(n));
            }
            let th = Theme::current(ui.ctx());
            ui.painter().text(
                pos2(ctrl_x + 66.0, r.top() + 10.0),
                Align2::LEFT_CENTER,
                "px",
                th.ui_font(),
                th.colors.text,
            );
        }
        Item::CheckInt(k, label, def, ik, idef, lo, hi) => {
            let v = d.get(prefs, k).and_then(|v| v.as_bool()).unwrap_or(def);
            let lw = 20.0 + super::text_w(ui.ctx(), &tl(label));
            if let Some(b) = fields::checkbox(
                ui,
                ("pref", k),
                Rect::from_min_size(r.min + vec2(0.0, 2.0), vec2(lw, 18.0)),
                &tl(label),
                v,
                true,
            ) {
                d.values.insert(k.into(), PrefValue::Bool(b));
            }
            let n = d.get(prefs, ik).and_then(|v| v.as_int()).unwrap_or(idef);
            let fx = (r.left() + lw + 8.0).max(ctrl_x).min(r.right() - 40.0);
            if let Some(x) = fields::number(
                ui,
                ("pref", ik),
                Rect::from_min_size(pos2(fx, r.top()), vec2(40.0, 20.0)),
                None,
                Some(n as f64),
                (lo as f64, hi as f64),
                0,
                v,
            ) {
                d.values.insert(ik.into(), PrefValue::Int(x as i64));
            }
        }
        Item::CheckText(k, label, def, tk, items, idef) => {
            let v = d.get(prefs, k).and_then(|v| v.as_bool()).unwrap_or(def);
            let lw = 20.0 + super::text_w(ui.ctx(), &tl(label));
            if let Some(b) = fields::checkbox(
                ui,
                ("pref", k),
                Rect::from_min_size(r.min + vec2(0.0, 2.0), vec2(lw, 18.0)),
                &tl(label),
                v,
                true,
            ) {
                d.values.insert(k.into(), PrefValue::Bool(b));
            }
            let stored = d
                .get(prefs, tk)
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let cur = if stored.trim().is_empty() {
                idef
            } else {
                font_preview_index(&stored)
            };
            let names: Vec<String> = items.iter().map(|k| t(k)).collect();
            let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
            let cw = (super::max_text_w(ui.ctx(), &refs) + 28.0).ceil();
            let fx = (r.left() + lw + 8.0).max(ctrl_x).min(r.right() - cw);
            if let Some(i) = fields::combo(
                ui,
                ("pref", tk),
                Rect::from_min_size(pos2(fx, r.top()), vec2(cw, 20.0)),
                &refs,
                Some(cur),
                v,
            ) {
                d.values.insert(tk.into(), PrefValue::Str(names[i].clone()));
            }
        }
        Item::CheckBrowse(k, label, def, dir_key) => {
            let v = d.get(prefs, k).and_then(|v| v.as_bool()).unwrap_or(def);
            let browse = t("dlg-export-browse");
            let bw = super::button_w(ui.ctx(), &browse, 80.0);
            let br = Rect::from_min_size(pos2(r.right() - bw, r.top()), vec2(bw, 21.0));
            if let Some(b) = fields::checkbox(
                ui,
                ("pref", k),
                Rect::from_min_size(
                    r.min + vec2(0.0, 2.0),
                    vec2((br.left() - r.left() - 8.0).max(40.0), 18.0),
                ),
                &tl(label),
                v,
                true,
            ) {
                d.values.insert(k.into(), PrefValue::Bool(b));
            }
            if push_button(ui, br, &browse, false, true) {
                d.browse = Some(dir_key.to_string());
            }
            // gewählter Ordner klein darunter (ohne Ordner nichts)
            let dir = d
                .get(prefs, dir_key)
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            if !dir.is_empty() {
                let th = Theme::current(ui.ctx());
                crate::widgets::text_clipped(
                    ui.painter(),
                    pos2(r.left() + 20.0, r.top() + 31.0),
                    Align2::LEFT_CENTER,
                    &dir,
                    th.small_font(),
                    th.colors.text_weak,
                    br.left() - r.left() - 28.0,
                );
            }
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
        let list_w = CATEGORIES
            .iter()
            .map(|k| tw(k) + 14.0)
            .fold(170.0, f32::max)
            .ceil();
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
                    Item::Int(_, l, ..)
                    | Item::Color(_, l, _)
                    | Item::Combo1(_, l, ..)
                    | Item::Text(_, l, _)
                    | Item::Pct(_, l, ..)
                    | Item::Px(_, l, ..) => label_w = label_w.max(tw(l)),
                    Item::CheckInt(_, l, ..) => page_w = page_w.max(tw(l) + 20.0 + 8.0 + 40.0),
                    Item::CheckText(_, l, _, _, items, _) => {
                        let names: Vec<String> = items.iter().map(|k| t(k)).collect();
                        let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
                        page_w =
                            page_w.max(tw(l) + 20.0 + 8.0 + super::max_text_w(ctx, &refs) + 28.0)
                    }
                    Item::CheckBrowse(_, l, ..) => {
                        page_w = page_w.max(
                            tw(l)
                                + 20.0
                                + 16.0
                                + super::button_w(ctx, &t("dlg-export-browse"), 80.0),
                        )
                    }
                }
            }
            let extra = match cat {
                CAT_PLUGINS => 12.0 + 62.0,
                CAT_TYPE => 12.0 + 24.0,
                _ => 0.0,
            };
            page_h = page_h.max(if cat == 0 {
                general_height()
            } else {
                28.0 + items.iter().map(item_h).sum::<f32>() + extra
            });
        }
        // Allgemein: rechtsbündige Beschriftungen der Gruppen, Neustart-Hinweis und Farbbeschriftungen
        for k in ["dlg-pref-grp-docoptions", "dlg-pref-grp-workspace"] {
            label_w = label_w.max(tw(k));
        }
        let note_w = 68.0 + tw("dlg-pref-restart-note");
        let colors_w =
            32.0 + tw("dlg-pref-color-stroke").max(80.0) + 20.0 + 32.0 + tw("dlg-pref-color-fill");
        let ctrl_off = 230f32.max(label_w + 10.0).ceil();
        // Kästchen auf „Allgemein“ stehen in der Steuerspalte (alle, auch „Entwickleroptionen“)
        let general_checks_w = page_items(0)
            .iter()
            .filter_map(|it| {
                if let Item::Check(_, l, _) = *it {
                    Some(20.0 + tw(l))
                } else {
                    None
                }
            })
            .fold(20.0, f32::max);
        page_w = page_w.max(ctrl_off + note_w.max(colors_w).max(general_checks_w));
        let reset_w = 220f32.max(tw("dlg-pref-reset-warnings") + 16.0).ceil();
        // Kombifelder: bisher auf die Seitenbreite gekürzt (198 px); mehr Platz nur für längere Einträge
        page_w = page_w
            .max(ctrl_off + if cw > 220.0 { cw } else { 198.0 })
            .max(reset_w)
            .max(super::text_w(ctx, &t("dlg-pref-cap-ui")) * 1.1);
        let w = 640f32.max(12.0 + list_w + 16.0 + page_w + 14.0).ceil();
        // Seiten mit vielen Einträgen: Platz bis über die Knöpfe
        // (Original 656 × 442 außen; Firepixel hat zusätzlich Design/Sprache/Entwickleroptionen:
        // außen ≈ 480–520 px, CT-1a maß vorher 585)
        let h = 440f32.max(12.0 + page_h + 60.0).ceil();
        Layout {
            list_w,
            ctrl_off,
            reset_w,
            size: vec2(w, h),
        }
    }
}

/// Zeigt den Dialog.
pub fn show(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.dialogs.preferences.clone() else {
        return;
    };
    let mut result = None;
    let lay = Layout::measure(ctx);
    let (ok_l, cancel_l, help_l) = (t("btn-ok"), t("btn-cancel"), t("btn-help"));
    let mut help = false;
    let r = dialog_frame(
        ctx,
        "fp-prefs",
        &t("dlg-pref-title"),
        lay.size,
        |ui, body| {
            let theme = Theme::current(ui.ctx());
            let c = &theme.colors;
            // Liste endet 10 px früher, damit die Fußleiste (DESIGN §12.2) ganz darunter Platz hat (BA-064)
            // Überschrift „Kategorie“ über der Liste wie im Original (TW-47)
            ui.painter().text(
                body.min + vec2(12.0, 10.0),
                Align2::LEFT_TOP,
                t("dlg-pref-category"),
                theme.ui_font(),
                c.text,
            );
            let list = Rect::from_min_size(
                body.min + vec2(12.0, 30.0),
                vec2(lay.list_w, body.height() - 88.0),
            );
            ui.painter().rect_filled(list, 0.0, c.list_bg);
            ui.painter().rect_stroke(
                list,
                0.0,
                Stroke::new(1.0, c.list_border),
                StrokeKind::Inside,
            );
            // Kategorieliste wie eine Windows-Liste: **ein** Tab-Stopp (der gewählte Eintrag), hat beim
            // Öffnen den Fokus, Pfeiltasten/Pos1/Ende wählen (Original, PM-5 D16)
            let mut focus_to = None;
            for (i, cat) in CATEGORIES.iter().enumerate() {
                let rr = Rect::from_min_size(
                    list.min + vec2(1.0, 1.0 + i as f32 * 21.0),
                    vec2(list.width() - 2.0, 21.0),
                );
                let id = egui::Id::new(("prefcat", i));
                let resp = ui.interact(
                    rr,
                    id,
                    if d.category == i {
                        Sense::click()
                    } else {
                        Sense::CLICK
                    },
                );
                let label = t(cat);
                resp.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::WidgetType::SelectableLabel,
                        true,
                        d.category == i,
                        &label,
                    )
                });
                if d.category == i {
                    ui.painter().rect_filled(rr, 0.0, c.list_selected_bg);
                } else if resp.hovered() {
                    ui.painter().rect_filled(rr, 0.0, c.list_hover_bg);
                }
                ui.painter().text(
                    pos2(rr.left() + 6.0, rr.center().y),
                    Align2::LEFT_CENTER,
                    &label,
                    theme.ui_font(),
                    c.text,
                );
                if d.category == i {
                    // Startfokus wie im Original auf der Liste (vor allen Feldern gezeichnet)
                    let key = super::autofocus_id(ui.layer_id());
                    if resp.enabled() && ui.ctx().data(|dd| dd.get_temp::<u64>(key)).is_some() {
                        ui.ctx().data_mut(|dd| dd.remove::<u64>(key));
                        resp.request_focus();
                    }
                    if ui.memory(|m| m.has_focus(id)) {
                        ui.painter().rect_stroke(
                            rr.shrink(1.0),
                            0.0,
                            Stroke::new(1.0, c.focus),
                            StrokeKind::Inside,
                        );
                        ui.memory_mut(|m| {
                            m.set_focus_lock_filter(
                                id,
                                egui::EventFilter {
                                    horizontal_arrows: false,
                                    vertical_arrows: true,
                                    tab: false,
                                    escape: false,
                                },
                            )
                        });
                        let n = CATEGORIES.len();
                        let to = ui.input_mut(|inp| {
                            let mut to = i;
                            if inp.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown) {
                                to = (to + 1).min(n - 1);
                            }
                            if inp.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) {
                                to = to.saturating_sub(1);
                            }
                            if inp.consume_key(egui::Modifiers::NONE, egui::Key::Home) {
                                to = 0;
                            }
                            if inp.consume_key(egui::Modifiers::NONE, egui::Key::End) {
                                to = n - 1;
                            }
                            to
                        });
                        if to != i {
                            focus_to = Some(to);
                        }
                    }
                }
                if resp.clicked() {
                    focus_to = Some(i);
                }
            }
            if let Some(i) = focus_to {
                d.category = i;
                ui.memory_mut(|m| m.request_focus(egui::Id::new(("prefcat", i))));
            }
            let page = Rect::from_min_max(
                pos2(list.right() + 16.0, list.top()),
                pos2(body.right() - 14.0, list.bottom()),
            );
            ui.painter().text(
                page.min,
                Align2::LEFT_TOP,
                t(CATEGORIES[d.category]),
                theme.bold(theme.fonts.heading),
                c.text,
            );
            let mut y = page.top() + 28.0;
            let ctrl_x = page.left() + lay.ctrl_off;
            let prefs = st.session.prefs.clone();
            if d.category == 0 {
                // Gruppen wie im Original: Dokumentoptionen, Speichern, Benutzeroberfläche, Standardfarben
                draw_general(ui, &mut d, &prefs, page, ctrl_x, lay.reset_w, st);
            } else {
                for it in page_items(d.category) {
                    let ih = item_h(it);
                    draw_item(
                        ui,
                        &mut d,
                        &prefs,
                        it,
                        Rect::from_min_size(pos2(page.left(), y), vec2(page.width(), ih)),
                        ctrl_x,
                        st,
                    );
                    y += ih;
                }
            }
            if d.category == CAT_PLUGINS {
                // Hinweis wie im Original, darunter der eigene Hinweis zu den Ordnern (umbrochen)
                super::text_block(
                    ui,
                    Rect::from_min_size(pos2(page.left(), y + 12.0), vec2(page.width(), 20.0)),
                    egui::RichText::new(t("dlg-pref-plugins-restart")).color(c.text),
                );
                super::text_block(
                    ui,
                    Rect::from_min_size(pos2(page.left(), y + 34.0), vec2(page.width(), 40.0)),
                    egui::RichText::new(t("dlg-pref-plugins-note"))
                        .font(theme.small_font())
                        .color(c.text_weak),
                );
            }
            if d.category == CAT_TYPE {
                super::text_block(
                    ui,
                    Rect::from_min_size(pos2(page.left(), y + 12.0), vec2(page.width(), 20.0)),
                    egui::RichText::new(t("dlg-pref-restart-note")).color(c.text),
                );
            }
            let by = body.bottom() - 36.0;
            // „Hilfe“ unten links wie im Original (öffnet die Hilfe, der Dialog bleibt offen)
            let hw = super::button_w(ui.ctx(), &help_l, 72.0);
            if push_button(
                ui,
                Rect::from_min_size(pos2(body.left() + 12.0, by), vec2(hw, 23.0)),
                &help_l,
                false,
                true,
            ) || super::access::pressed(ui, &help_l)
            {
                help = true;
            }
            let b = super::button_row(
                ui.ctx(),
                body.right() - 16.0,
                by,
                &[&ok_l, &cancel_l],
                76.0,
                23.0,
                8.0,
            );
            if push_button(ui, b[0], &ok_l, true, true) {
                result = Some(true);
            }
            if push_button(ui, b[1], &cancel_l, false, true) {
                result = Some(false);
            }
        },
    );
    if help {
        st.run("help.firepixelHelp", serde_json::json!({}));
    }
    // „Durchsuchen...“ (Zusatzmodule): Ordner wählen und in den offenen Dialog übernehmen
    if let Some(key) = d.browse.take() {
        let req = crate::file_dialog::Request::folder(
            t("dlg-export-browse")
                .trim_end_matches(['.', '…'])
                .to_string(),
        );
        crate::file_dialog::request(st, req, move |st, paths| {
            if let (Some(p), Some(dlg)) = (paths.first(), st.dialogs.preferences.as_mut()) {
                dlg.values
                    .insert(key.clone(), PrefValue::Str(p.to_string_lossy().to_string()));
            }
        });
    }
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
                    assert!(
                        fp_engine::prefs::definition(k).is_some()
                            || p.get(k).is_some()
                            || *k == crate::language::DEV_PREF_KEY
                            // own key (no counterpart in the original's preference list)
                            || *k == crate::canvas::tools::ai_object::PREF_KEY
                            || *k == fp_engine::tools::ALT_MARQUEE_DESELECT_PREF
                            || *k == fp_engine::tools::DUPLICATE_WITH_CTRL_DRAG_PREF
                            || *k == fp_engine::tools::SHIFT_MARQUEE_ADDS_ONLY_PREF
                            || *k == crate::canvas::SHIFT_WHEEL_HORIZONTAL_PREF
                            || *k == fp_engine::tools::ARROW_KEYS_MOVE_PIXELS_PREF,
                        "unbekannter Schlüssel {k}"
                    );
                }
            }
        }
    }
}
