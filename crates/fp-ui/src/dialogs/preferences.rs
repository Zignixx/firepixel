//! Voreinstellungen (Dialog 4419, Kategorieliste 4423, Seiten 4426–4432, 14 §3.4): alle sieben
//! Kategorien mit den Schlüsseln und Vorgaben aus 14 §3.4, dazu unter „Allgemein ▸
//! Benutzeroberfläche“ die Design-Wahl Normal/Dunkel/OLED. Änderungen erst mit OK; danach
//! sofort gespeichert (`SavePrefsWhileRunning`).

use super::{dialog_frame, push_button, DialogClose};
use crate::app_state::{Action, AppState};
use crate::inspector::fields;
use crate::theme::{Theme, ThemeKind};
use egui::{pos2, vec2, Align2, Rect, Sense, Stroke, StrokeKind, Ui};
use fp_engine::{PrefValue, Preferences};
use std::collections::BTreeMap;

/// Kategorien in der Reihenfolge der Liste.
pub const CATEGORIES: [&str; 7] = ["Allgemein", "Bearbeiten", "Hilfslinien und Raster", "Text", "Photoshop-Import/Öffnen", "Zusatzmodule", "Starten und Bearbeiten"];

/// Ein Eintrag einer Seite.
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

const INTERP: &[&str] = &["Bikubisch", "Bilinear", "Weich", "Nächster Nachbar"];
const LOOK: &[&str] = &["Immer PNG-Quelldatei verwenden", "Nie PNG-Quelldatei verwenden", "Beim Starten fragen"];

fn page_items(cat: usize) -> &'static [Item] {
    use Item::*;
    match cat {
        0 => &[
            Int("MaxUndos", "Rückgängig-Schritte:", 20, 0, 999),
            Check("ShowStartPage", "Startseite anzeigen", true),
            Check("XformToolXformsBFE", "Pinselstriche und Effekte skalieren", true),
            Combo1("EditTransformMode", "Interpolation:", INTERP, 1),
            Check("OSBasedResample", "Schnellere, aber weniger präzise Neuauflösung", false),
            Color("BrushDefaultColor", "Standardfarbe Pinselstrich:", "#000000"),
            Color("FillDefaultColor", "Standardfarbe Füllung:", "#ffffff"),
            Color("FeedbackColor", "Hervorhebung:", "#00a8ff"),
            Check("IconPanelsAutoCollapse", "Bedienfeldsymbole automatisch minimieren", false),
            Check("WriteThumbnailPreview", "Einzelseitenminiaturen speichern", true),
            Check("AppendFwPng", ".fw.png anhängen", true),
        ],
        1 => &[
            Check("CropDocCropsImages", "Objekte beim Zuschneiden löschen", true),
            Check("DeletePaths", "Pfade beim Konvertieren in Auswahlrahmen löschen", true),
            Check("UseBrushSizeCursors", "Pinselstärke in Malen-Cursor", true),
            Check("UsePrecisionCursors", "Präzise Cursor", false),
            Caption("Stift-Werkzeug:"),
            Check("ShowPenPreview", "Stiftvorschau anzeigen", true),
            Check("ShowSolidPoints", "Einfarbige Punkte anzeigen", false),
            Caption("Zeiger und Teilauswahl:"),
            Check("SelectionMouseOverPreview", "Mausaktivierte Hervorhebung", true),
            Check("PointerToolPaintHandles", "Füllungsgriffe anzeigen", true),
            Check("PreviewDrag", "Vorschau beim Ziehen", true),
            Int("PickDistance", "Markierungsabstand (Pixel):", 5, 1, 10),
            Check("UngroupAutoshapes", "Gruppierung automatischer Formen bei 9-Slice aufheben", false),
        ],
        2 => &[
            Caption("Hilfslinien:"),
            Check("ShowGuides", "Anzeigen", true),
            Check("LockGuides", "Sperren", false),
            Check("SnapToGuides", "Ausrichten", true),
            Color("GuideColor", "Hilfslinienfarbe:", "#00ffff"),
            Color("SliceGuideColor", "Segmenthilfslinien:", "#ff4a4a"),
            Check("SliceGuidesVisible", "Segmenthilfslinien anzeigen", true),
            Color("NineSliceGuideColor", "9-Slice-Hilfslinien:", "#0000ff"),
            Caption("Raster:"),
            Check("ShowGrid", "Raster anzeigen", false),
            Check("SnapToGrids", "Am Raster ausrichten", false),
            Color("GridColor", "Rasterfarbe:", "#00ffff"),
            Int("GridHorizontalDistance", "Rasterabstand horizontal:", 36, 1, 10000),
            Int("GridVerticalDistance", "Rasterabstand vertikal:", 36, 1, 10000),
            Caption("Intelligente Hilfslinien:"),
            Check("ShowSmartGuides", "Anzeigen", true),
            Check("SnapToSmartGuides", "Ausrichten", true),
            Color("SmartGuideColor", "Farbe:", "#ff4aff"),
            Int("SnapDistance", "Ausrichtungsabstand (Pixel):", 5, 1, 50),
        ],
        3 => &[
            Real("DefaultLeading", "Standard-Zeilenabstand (Faktor):", 1.2, 0.0, 10.0),
            Real("DefaultBaselineShift", "Grundlinienverschiebung (px):", 0.0, -999.0, 999.0),
            Check("ShowFontNamesInEnglish", "Schriftnamen auf Englisch anzeigen", false),
            Check("ShowFontPreview", "Schriftvorschau", true),
            Check("ShowRecentFonts", "Zuletzt verwendete Schriften anzeigen", true),
            Int("RecentFontLimit", "Anzahl zuletzt verwendeter Schriften:", 6, 0, 20),
            Text("DefaultMissingFont", "Ersatz für fehlende Schriften:", "Arial"),
        ],
        4 => &[
            Check("ShowPSImportDialog", "Dialog beim Importieren anzeigen", true),
            Check("ShowPSOpenDialog", "Dialog beim Öffnen anzeigen", true),
            Check("PSImageLayersEditable", "Bildebenen bearbeitbar", true),
            Check("PSTextEditable", "Text bearbeitbar", true),
            Check("PSShapeLayersEditable", "Formebenen bearbeitbar", true),
            Check("PSShapeLayerEffectsEditable", "Effekte der Formebenen bearbeitbar", true),
            Check("PSSharedLayers", "Ebenen über Status teilen", false),
            Check("UseFWFilters", "Native Filter haben Vorrang", false),
            Check("PSClippingMasksEditable", "Schnittmasken bearbeitbar", true),
            Check("PSRetainAdjustmentLayersAppearance", "Erscheinungsbild der Einstellungsebenen beibehalten", true),
        ],
        5 => &[
            Item::Caption("Zusatzmodule (Neustart erforderlich):"),
            Check("UseExtraPhotoshopPluginsDir", "Photoshop-Zusatzmodule", false),
            Text("ExtraPhotoshopPluginsDir", "Ordner:", ""),
            Check("UseExtraTexturesDir", "Texturen", false),
            Text("ExtraTexturesDir", "Ordner:", ""),
            Check("UseExtraPatternsDir", "Muster", false),
            Text("ExtraPatternsDir", "Ordner:", ""),
            Item::Caption("Ressourcenordner des Originals (leer = automatische Suche):"),
            Text(ORIGINAL_DIR_KEY, "Ordner:", ""),
        ],
        _ => &[Combo1("LegLookForPNG", "FW-PNG-Quelldateien (Bearbeiten):", LOOK, 3), Combo1("LoptLookForPNG", "FW-PNG-Quelldateien (Optimieren):", LOOK, 3)],
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
    match *item {
        Item::Check(k, label, def) => {
            let v = d.get(prefs, k).and_then(|v| v.as_bool()).unwrap_or(def);
            if let Some(b) = fields::checkbox(ui, ("pref", k), Rect::from_min_size(r.min, vec2(r.width(), 18.0)), label, v, true) {
                d.values.insert(k.into(), PrefValue::Bool(b));
            }
        }
        Item::Int(k, label, def, lo, hi) => {
            fields::label(ui, Rect::from_min_size(r.min, vec2(ctrl_x - r.left(), 20.0)), label, true);
            let v = d.get(prefs, k).and_then(|v| v.as_int()).unwrap_or(def);
            if let Some(n) = fields::number(ui, ("pref", k), Rect::from_min_size(pos2(ctrl_x, r.top()), vec2(60.0, 20.0)), None, Some(v as f64), (lo as f64, hi as f64), 0, true) {
                d.values.insert(k.into(), PrefValue::Int(n as i64));
            }
        }
        Item::Real(k, label, def, lo, hi) => {
            fields::label(ui, Rect::from_min_size(r.min, vec2(ctrl_x - r.left(), 20.0)), label, true);
            let v = d.get(prefs, k).and_then(|v| v.as_real()).unwrap_or(def);
            if let Some(n) = fields::number(ui, ("pref", k), Rect::from_min_size(pos2(ctrl_x, r.top()), vec2(60.0, 20.0)), None, Some(v), (lo, hi), 2, true) {
                d.values.insert(k.into(), PrefValue::Real(n));
            }
        }
        Item::Color(k, label, def) => {
            fields::label(ui, Rect::from_min_size(r.min, vec2(ctrl_x - r.left(), 20.0)), label, true);
            let v = d.get(prefs, k).and_then(|v| v.as_color()).or_else(|| fp_color::Rgba8::parse_hex(def).map(|c| c.to_argb()));
            if let Some(crate::inspector::popups::ColorChoice::Color(c)) = crate::inspector::popups::color_button(ui, ("pref", k), Rect::from_min_size(pos2(ctrl_x, r.top()), vec2(26.0, 20.0)), st, v, true, false, false) {
                d.values.insert(k.into(), PrefValue::Str(format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b())));
            }
        }
        Item::Combo1(k, label, items, def) => {
            fields::label(ui, Rect::from_min_size(r.min, vec2(ctrl_x - r.left(), 20.0)), label, true);
            let v = d.get(prefs, k).and_then(|v| v.as_int()).unwrap_or(def).clamp(1, items.len() as i64);
            if let Some(i) = fields::combo(ui, ("pref", k), Rect::from_min_size(pos2(ctrl_x, r.top()), vec2((r.right() - ctrl_x).min(220.0), 20.0)), items, Some((v - 1) as usize), true) {
                d.values.insert(k.into(), PrefValue::Int(i as i64 + 1));
            }
        }
        Item::Text(k, label, def) => {
            fields::label(ui, Rect::from_min_size(r.min, vec2(ctrl_x - r.left(), 20.0)), label, true);
            let v = d.get(prefs, k).and_then(|v| v.as_str()).unwrap_or_else(|| def.to_string());
            if let Some(t) = fields::text(ui, ("pref", k), Rect::from_min_size(pos2(ctrl_x, r.top()), vec2((r.right() - ctrl_x).min(220.0), 20.0)), &v, true) {
                d.values.insert(k.into(), PrefValue::Str(t));
            }
        }
        Item::Caption(label) => {
            let t = Theme::current(ui.ctx());
            ui.painter().text(pos2(r.left(), r.top() + 10.0), Align2::LEFT_CENTER, label, t.bold(t.fonts.ui), t.colors.text);
        }
    }
}

/// Zeigt den Dialog.
pub fn show(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.dialogs.preferences.clone() else { return };
    let mut result = None;
    let r = dialog_frame(ctx, "fp-prefs", "Voreinstellungen", vec2(640.0, 500.0), |ui, body| {
        let theme = Theme::current(ui.ctx());
        let c = &theme.colors;
        let list = Rect::from_min_size(body.min + vec2(12.0, 12.0), vec2(170.0, body.height() - 60.0));
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
            ui.painter().text(pos2(rr.left() + 6.0, rr.center().y), Align2::LEFT_CENTER, *cat, theme.ui_font(), c.text);
            if resp.clicked() {
                d.category = i;
            }
        }
        let page = Rect::from_min_max(pos2(list.right() + 16.0, list.top()), pos2(body.right() - 14.0, list.bottom()));
        ui.painter().text(page.min, Align2::LEFT_TOP, CATEGORIES[d.category], theme.bold(theme.fonts.heading), c.text);
        let mut y = page.top() + 28.0;
        let ctrl_x = page.left() + 230.0;
        let prefs = st.session.prefs.clone();
        for it in page_items(d.category) {
            let h = 24.0;
            draw_item(ui, &mut d, &prefs, it, Rect::from_min_size(pos2(page.left(), y), vec2(page.width(), 22.0)), ctrl_x, st);
            y += h;
        }
        if d.category == 0 {
            y += 6.0;
            ui.painter().text(pos2(page.left(), y + 10.0), Align2::LEFT_CENTER, "Benutzeroberfläche", theme.bold(theme.fonts.ui), c.text);
            y += 24.0;
            fields::label(ui, Rect::from_min_size(pos2(page.left(), y), vec2(200.0, 20.0)), "Design:", true);
            let names: Vec<&str> = ThemeKind::ALL.iter().map(|k| k.label()).collect();
            let cur = ThemeKind::ALL.iter().position(|k| *k == d.theme);
            if let Some(i) = fields::combo(ui, "pref-theme", Rect::from_min_size(pos2(ctrl_x, y), vec2(140.0, 20.0)), &names, cur, true) {
                d.theme = ThemeKind::ALL[i];
            }
            y += 30.0;
            if fields::button(ui, "pref-resetwarn", Rect::from_min_size(pos2(page.left(), y), vec2(220.0, 22.0)), "Alle Warnmeldungen zurücksetzen", !d.reset_warnings) {
                d.reset_warnings = true;
            }
        }
        if d.category == 5 {
            ui.painter().text(pos2(page.left(), y + 12.0), Align2::LEFT_TOP, "Hinweis: Firepixel bettet keine Texturen oder Muster des Originals ein; Ordner werden nur zur Laufzeit gelesen.", theme.small_font(), c.text_weak);
        }
        let by = body.bottom() - 36.0;
        if push_button(ui, Rect::from_min_size(pos2(body.right() - 176.0, by), vec2(76.0, 23.0)), "OK", true, true) {
            result = Some(true);
        }
        if push_button(ui, Rect::from_min_size(pos2(body.right() - 92.0, by), vec2(76.0, 23.0)), "Abbrechen", false, true) {
            result = Some(false);
        }
    });
    match (result, r) {
        (Some(true), _) => {
            st.dialogs.preferences = None;
            d.apply(&mut st.session.prefs);
            st.session.prefs_changed();
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
                    assert!(fp_engine::prefs::definition(k).is_some() || p.get(k).is_some(), "unbekannter Schlüssel {k}");
                }
            }
        }
    }
}
