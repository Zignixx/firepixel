//! Dialog „Neues Dokument“ (Vorlage 5961, Lage der Steuerelemente in DLU aus `dialoge.json`).

use super::{dialog_frame, dlu, dlu_rect, group_box, push_button, DialogClose};
use crate::app_state::{Action, AppState};
use egui::{vec2, Color32, Rect, Sense, Ui};
use serde_json::json;

const UNITS: [&str; 3] = ["Pixel", "Zoll", "Zentimeter"];
const RES_UNITS: [&str; 2] = ["Pixel/Zoll", "Pixel/cm"];

/// Leinwandfarbe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanvasChoice {
    White,
    Transparent,
    Custom,
}

/// Zustand des Dialogs.
#[derive(Clone, Debug)]
pub struct NewDocumentDialog {
    pub width: String,
    pub height: String,
    pub width_unit: usize,
    pub height_unit: usize,
    pub resolution: String,
    pub resolution_unit: usize,
    pub canvas: CanvasChoice,
    pub custom: Color32,
}

fn fmt_num(v: f64) -> String {
    let s = format!("{v:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    s.replace('.', ",")
}

fn parse_num(s: &str) -> Option<f64> {
    s.trim().replace(',', ".").parse::<f64>().ok().filter(|v| v.is_finite())
}

impl NewDocumentDialog {
    /// Vorgaben aus den Voreinstellungen (`DefaultDocSize`, Auflösung, Leinwandfarbe).
    pub fn from_prefs(p: &fp_engine::Preferences) -> NewDocumentDialog {
        let (w, h) = p.new_document_size();
        let res = p.new_document_resolution();
        let canvas = match p.canvas_default() {
            fp_engine::prefs::CanvasDefault::White => CanvasChoice::White,
            fp_engine::prefs::CanvasDefault::Transparent => CanvasChoice::Transparent,
            fp_engine::prefs::CanvasDefault::Custom => CanvasChoice::Custom,
        };
        let bg = p.new_document_background();
        NewDocumentDialog {
            width: fmt_num(w as f64),
            height: fmt_num(h as f64),
            width_unit: 0,
            height_unit: 0,
            resolution: fmt_num(res.value as f64),
            resolution_unit: if res.per_cm { 1 } else { 0 },
            canvas,
            custom: Color32::from_rgb((bg.0 >> 16 & 0xff) as u8, (bg.0 >> 8 & 0xff) as u8, (bg.0 & 0xff) as u8),
        }
    }

    fn ppi(&self) -> f64 {
        let r = parse_num(&self.resolution).unwrap_or(72.0).max(1.0);
        if self.resolution_unit == 1 { r * 2.54 } else { r }
    }

    fn to_pixels(&self, v: &str, unit: usize) -> Option<f64> {
        let x = parse_num(v)?;
        let px = match unit {
            1 => x * self.ppi(),
            2 => x * self.ppi() / 2.54,
            _ => x,
        };
        Some(px.round())
    }

    /// Breite/Höhe in Pixeln, falls gültig (1 … 30000).
    pub fn pixel_size(&self) -> Option<(f64, f64)> {
        let w = self.to_pixels(&self.width, self.width_unit)?;
        let h = self.to_pixels(&self.height, self.height_unit)?;
        ((1.0..=30000.0).contains(&w) && (1.0..=30000.0).contains(&h)).then_some((w, h))
    }

    /// Parameter für `file.new`.
    pub fn params(&self) -> Option<serde_json::Value> {
        let (w, h) = self.pixel_size()?;
        let bg: u32 = match self.canvas {
            CanvasChoice::White => 0xffff_ffff,
            CanvasChoice::Transparent => 0x00ff_ffff,
            CanvasChoice::Custom => 0xff00_0000 | (self.custom.r() as u32) << 16 | (self.custom.g() as u32) << 8 | self.custom.b() as u32,
        };
        let res = parse_num(&self.resolution).unwrap_or(72.0).clamp(1.0, 5000.0);
        let res_ppi = if self.resolution_unit == 1 { res * 2.54 } else { res };
        Some(json!({ "width": w, "height": h, "background": bg, "resolution": res_ppi }))
    }
}

/// Größe der Leinwand im Gruppentitel wie im Original: „5.9K“ bzw. „1.1M“ (Punkt, eine Stelle).
pub fn size_label(bytes: f64) -> String {
    if bytes < 1_048_576.0 {
        format!("{:.1}K", bytes / 1024.0)
    } else {
        format!("{:.1}M", bytes / 1_048_576.0)
    }
}

impl NewDocumentDialog {
    /// Bestätigte Werte als Vorgabe für das nächste Öffnen ablegen (wie im Original).
    pub fn remember(&self, p: &mut fp_engine::Preferences) {
        use fp_engine::prefs::PrefValue;
        if let Some((w, h)) = self.pixel_size() {
            p.set("DefaultDocSize", PrefValue::Str(format!("{w} {h}")));
        }
        if let Some(r) = parse_num(&self.resolution) {
            p.set("DefaultDocRes", PrefValue::Real(r));
        }
        p.set("DefaultDocResUnitName", PrefValue::Str(if self.resolution_unit == 1 { "cm" } else { "inch" }.into()));
        let code = match self.canvas {
            CanvasChoice::White => 1,
            CanvasChoice::Transparent => 2,
            CanvasChoice::Custom => 3,
        };
        p.set("DefaultDocColorState", PrefValue::Int(code));
        if self.canvas == CanvasChoice::Custom {
            p.set("CustomCanvasColor", PrefValue::Str(format!("#{:02x}{:02x}{:02x}", self.custom.r(), self.custom.g(), self.custom.b())));
        }
    }
}

fn combo(ui: &mut Ui, rect: Rect, id: &str, items: &[&str], sel: &mut usize) {
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    egui::ComboBox::from_id_salt(id).width(rect.width() - 8.0).selected_text(items.get(*sel).copied().unwrap_or("")).show_ui(&mut child, |ui| {
        for (i, it) in items.iter().enumerate() {
            ui.selectable_value(sel, i, *it);
        }
    });
}

/// Zeigt den Dialog.
pub fn show(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.dialogs.new_document.clone() else { return };
    let mut result: Option<bool> = None;
    let mut templates = false;
    let size = dlu(260.0, 174.0);
    let r = dialog_frame(ctx, "fp-new-doc", "Neues Dokument", size, |ui, body| {
        let o = body.min;
        let bytes = d.pixel_size().map(|(w, h)| w * h * 4.0).unwrap_or(0.0);
        group_box(ui, dlu_rect(o, [8.0, 6.0, 242.0, 68.0]), &format!("Leinwandgröße: {}", size_label(bytes)));
        let label = |ui: &mut Ui, r: [f32; 4], t: &str| {
            super::put_left(ui, dlu_rect(o, r), egui::Label::new(t));
        };
        label(ui, [41.0, 21.0, 34.0, 10.0], "Breite:");
        label(ui, [43.0, 38.0, 32.0, 10.0], "Höhe:");
        label(ui, [26.0, 55.0, 49.0, 11.0], "Auflösung:");
        // Tab-Folge wie im Original: Breite → Höhe → Auflösung, dann die Einheiten („800 ⇥ 600 ↵“)
        for (r, s) in [([76.0, 18.0, 31.0, 13.0], &mut d.width), ([76.0, 35.0, 31.0, 13.0], &mut d.height), ([76.0, 52.0, 31.0, 13.0], &mut d.resolution)] {
            let te = ui.put(dlu_rect(o, r), egui::TextEdit::singleline(s));
            super::autofocus_field(ui, &te, s.chars().count());
            super::select_on_tab(ui, &te, s.chars().count());
        }
        combo(ui, dlu_rect(o, [112.0, 18.0, 91.0, 13.0]), "nd-wu", &UNITS, &mut d.width_unit);
        combo(ui, dlu_rect(o, [112.0, 35.0, 91.0, 13.0]), "nd-hu", &UNITS, &mut d.height_unit);
        combo(ui, dlu_rect(o, [112.0, 52.0, 91.0, 13.0]), "nd-ru", &RES_UNITS, &mut d.resolution_unit);
        let (pw, ph) = d.pixel_size().unwrap_or((0.0, 0.0));
        label(ui, [206.0, 21.0, 12.0, 10.0], "B:");
        label(ui, [218.0, 21.0, 30.0, 10.0], &format!("{pw:.0}"));
        label(ui, [206.0, 39.0, 12.0, 10.0], "H:");
        label(ui, [218.0, 39.0, 30.0, 10.0], &format!("{ph:.0}"));
        group_box(ui, dlu_rect(o, [8.0, 78.0, 242.0, 70.0]), "Leinwandfarbe");
        for (y, label, choice) in [(90.0, "Weiß", CanvasChoice::White), (104.0, "Transparent", CanvasChoice::Transparent), (118.0, "Benutzerdefiniert:", CanvasChoice::Custom)] {
            let mut row = ui.new_child(egui::UiBuilder::new().max_rect(dlu_rect(o, [13.0, y, 80.0, 11.0])).layout(egui::Layout::left_to_right(egui::Align::Center)));
            if row.radio(d.canvas == choice, label).clicked() {
                d.canvas = choice;
            }
        }
        let sw = dlu_rect(o, [96.0, 116.0, 14.0, 14.0]);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(sw));
        let mut rgb = [d.custom.r(), d.custom.g(), d.custom.b()];
        if child.color_edit_button_srgb(&mut rgb).changed() {
            d.custom = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
            d.canvas = CanvasChoice::Custom;
        }
        let _ = ui.allocate_rect(body, Sense::hover());
        // „Vorlagen…“ öffnet wie im Original „Neu von Vorlage“ (Dateiwahl, 7490)
        if push_button(ui, dlu_rect(o, [8.0, 154.0, 50.0, 14.0]), "Vorlagen...", false, true) {
            templates = true;
        }
        let valid = d.pixel_size().is_some();
        if push_button(ui, dlu_rect(o, [146.0, 154.0, 50.0, 14.0]), "OK", true, valid) {
            result = Some(true);
        }
        if push_button(ui, dlu_rect(o, [200.0, 154.0, 50.0, 14.0]), "Abbrechen", false, true) {
            result = Some(false);
        }
        let _ = vec2(0.0, 0.0);
    });
    match (result, r) {
        _ if templates => {
            st.dialogs.new_document = None;
            if let Some(path) = super::panel_menus::pick_open(st, "Neu von Vorlage", &[("FW-PNG", &["png"])]) {
                st.push(Action::Run("file.newFromTemplate".into(), json!({"path": path.to_string_lossy()})));
            }
        }
        (Some(true), _) => {
            st.dialogs.new_document = None;
            d.remember(&mut st.session.prefs);
            if let Some(p) = d.params() {
                st.push(Action::Run("file.new".into(), p));
            }
        }
        (Some(false), _) | (_, DialogClose::Cancelled) => st.dialogs.new_document = None,
        _ => st.dialogs.new_document = Some(d),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units_convert() {
        let mut d = NewDocumentDialog::from_prefs(&fp_engine::Preferences::new());
        d.width = "2".into();
        d.width_unit = 1;
        d.height = "100".into();
        d.resolution = "72".into();
        assert_eq!(d.pixel_size(), Some((144.0, 100.0)));
        d.canvas = CanvasChoice::Transparent;
        let p = d.params().unwrap_or_default();
        assert_eq!(p["background"], json!(0x00ff_ffffu32));
        d.width = "abc".into();
        assert!(d.params().is_none());
    }
}
