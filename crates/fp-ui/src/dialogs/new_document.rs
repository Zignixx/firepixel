//! Dialog „Neues Dokument“ (Vorlage 5961, Lage der Steuerelemente in DLU aus `dialoge.json`).

use super::{dialog_frame, dlu, dlu_rect, extra_w, group_box, max_text_w, push_button, t, text_w, DialogClose};
use crate::app_state::{Action, AppState};
use egui::{vec2, Color32, Id, Rect, Sense, Ui};
use serde_json::json;

/// Einheiten (Schlüssel, auch in Leinwand-/Bildgröße).
pub(crate) const UNITS: [&str; 3] = ["dlg-unit-pixels", "dlg-unit-inches", "dlg-unit-cm"];
pub(crate) const RES_UNITS: [&str; 2] = ["dlg-unit-ppi", "dlg-unit-ppcm"];

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

/// Größe der internen Zwischenablage in Pixeln (`None`: leer).
pub fn clipboard_size(session: &fp_engine::Session) -> Option<(f64, f64)> {
    let c = &session.clipboard;
    if let Some(px) = &c.pixels {
        return (px.width() > 0 && px.height() > 0).then(|| (px.width() as f64, px.height() as f64));
    }
    let doc = &session.active()?.doc;
    let b = c.elements.iter().filter_map(|e| fp_engine::docops::element_bounds(doc, e)).reduce(|a, b| a.union(&b))?;
    let b = b.round_out();
    (b.width() > 0 && b.height() > 0).then(|| (b.width() as f64, b.height() as f64))
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
        // Farbfeld „Benutzerdefiniert“ zeigt immer die gemerkte Farbe (Vorgabe schwarz), auch wenn
        // Weiß oder Transparent gewählt ist (D01)
        let bg = p.get_color("CustomCanvasColor", fp_color::Argb(0xff00_0000));
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

    /// TW-24: Liegt etwas in der internen Zwischenablage, schlägt das Original dessen Größe vor
    /// (Rahmen aller kopierten Objekte bzw. die kopierten Pixel).
    pub fn with_clipboard(mut self, session: &fp_engine::Session) -> NewDocumentDialog {
        if let Some((w, h)) = clipboard_size(session) {
            self.width = fmt_num(w);
            self.height = fmt_num(h);
            self.width_unit = 0;
            self.height_unit = 0;
        }
        self
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

/// Unterstreicht die Zugriffstaste eines links ausgerichteten, senkrecht mittigen Texts (Optionsfelder
/// zeichnen ihre Beschriftung selbst, ohne „&“).
fn underline_access(ui: &Ui, left_center: egui::Pos2, raw: &str) {
    let Some(i) = raw.find('&') else { return };
    let Some(ch) = raw[i + 1..].chars().next() else { return };
    let theme = crate::theme::Theme::current(ui.ctx());
    let font = theme.ui_font();
    let x0 = left_center.x + text_w(ui.ctx(), &raw[..i]);
    let x1 = x0 + text_w(ui.ctx(), &ch.to_string());
    let y = left_center.y + font.size * 0.55;
    ui.painter().line_segment([egui::pos2(x0, y), egui::pos2(x1, y)], egui::Stroke::new(1.0, theme.colors.text));
}

/// Zeigt den Dialog.
pub fn show(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.dialogs.new_document.clone() else { return };
    let mut result: Option<bool> = None;
    let mut templates = false;
    // Texte der aktuellen Sprache; längere Texte schieben die Spalten nach rechts (Deutsch: wie bisher)
    let units: Vec<String> = UNITS.iter().map(|k| t(k)).collect();
    let res_units: Vec<String> = RES_UNITS.iter().map(|k| t(k)).collect();
    // Zugriffstasten wie im Original (&Breite, &Höhe, &Auflösung, We&iß, &Transparent,
    // Be&nutzerdefiniert, PM-5 D01): Texte mit „&“, Lage und Breite ohne
    let labs_ak = [t("dlg-new-doc-width-ak"), t("dlg-new-doc-height-ak"), t("dlg-new-doc-resolution-ak")];
    let radios_ak = [t("dlg-canvas-white-ak"), t("dlg-canvas-transparent-ak"), t("dlg-canvas-custom-ak")];
    let labs = labs_ak.clone().map(|s| s.replace('&', ""));
    let wh = [t("dlg-short-w"), t("dlg-short-h")];
    let radios = radios_ak.clone().map(|s| s.replace('&', ""));
    let (tmpl_l, ok_l, cancel_l) = (t("dlg-new-doc-templates"), t("btn-ok"), t("btn-cancel"));
    let all_units: Vec<&str> = units.iter().chain(res_units.iter()).map(|s| s.as_str()).collect();
    let dx = extra_w(ctx, &[&labs[0], &labs[1], &labs[2]], dlu(75.0 - 12.0, 0.0).x);
    let dc = extra_w(ctx, &all_units, dlu(91.0, 0.0).x - 34.0);
    let dv = extra_w(ctx, &[&wh[0], &wh[1]], dlu(12.0, 0.0).x - 2.0);
    // Optionsfeld: Kreis 14 px + Abstand 4 px + Text
    let radio_w = max_text_w(ctx, &[&radios[0], &radios[1], &radios[2]]) + 18.0;
    let dsw = (dlu(13.0, 0.0).x + radio_w + 1.0 - dlu(96.0, 0.0).x).max(0.0).ceil();
    let bw = super::buttons_w(ctx, &[&tmpl_l, &ok_l, &cancel_l], dlu(50.0, 0.0).x);
    let gap = dlu(4.0, 0.0).x;
    let ex = (dx + dc + dv).max(dsw - dlu(250.0 - 110.0 - 4.0, 0.0).x).max(dlu(8.0 + 10.0, 0.0).x + 3.0 * bw + 2.0 * gap + 20.0 - dlu(260.0, 0.0).x);
    let size = dlu(260.0, 174.0) + vec2(ex, 0.0);
    // Verschiebung einer DLU-Spalte: rechts der Beschriftungen um dx, rechts der Einheiten um dc …
    let shift = move |x: f32| -> f32 {
        if x >= 218.0 {
            dx + dc + dv
        } else if x >= 206.0 {
            dx + dc
        } else if x >= 76.0 {
            dx
        } else {
            0.0
        }
    };
    let rr = move |o: egui::Pos2, r: [f32; 4]| dlu_rect(o, r).translate(vec2(shift(r[0]), 0.0));
    let r = dialog_frame(ctx, "fp-new-doc", &t("dlg-new-doc-title"), size, |ui, body| {
        let o = body.min;
        let bytes = d.pixel_size().map(|(w, h)| w * h * 4.0).unwrap_or(0.0);
        group_box(ui, dlu_rect(o, [8.0, 6.0, 242.0, 68.0]).with_max_x(body.right() - dlu(10.0, 0.0).x), &fp_i18n::tr!("dlg-new-doc-canvas-size", size = size_label(bytes)));
        // Beschriftungen rechtsbündig vor den Feldern (passen sie, steht der Text wie bisher)
        let right_label = |ui: &mut Ui, r: [f32; 4], text: &str| {
            let rect = dlu_rect(o, r);
            let right = dlu_rect(o, [75.0, 0.0, 0.0, 0.0]).left() + dx;
            let left = rect.left().min(right - text_w(ui.ctx(), text));
            let text = super::access_key_text(ui, text, true);
            super::put_left(ui, Rect::from_min_size(egui::pos2(left, rect.top()), vec2(right - left + 2.0, rect.height())), egui::Label::new(text).extend());
        };
        right_label(ui, [41.0, 21.0, 34.0, 10.0], &labs_ak[0]);
        right_label(ui, [43.0, 38.0, 32.0, 10.0], &labs_ak[1]);
        right_label(ui, [26.0, 55.0, 49.0, 11.0], &labs_ak[2]);
        // Alt+Zugriffstaste: Feld bzw. Optionsfeld wählen (wie unter Windows)
        let pressed = |ui: &mut Ui, raw: &str| super::access_key(raw).is_some_and(|k| ui.input_mut(|i| i.consume_key(egui::Modifiers::ALT, k)));
        let mut focus_field = None;
        for (k, raw) in labs_ak.iter().enumerate() {
            if pressed(ui, raw) {
                focus_field = Some(k);
            }
        }
        let mut pick_radio = None;
        for (k, raw) in radios_ak.iter().enumerate() {
            if pressed(ui, raw) {
                pick_radio = Some(k);
            }
        }
        let label = |ui: &mut Ui, r: Rect, t: &str| {
            super::put_left(ui, r, egui::Label::new(t).extend());
        };
        // Tab-Folge wie im Original: Breite → Höhe → Auflösung, dann die Einheiten („800 ⇥ 600 ↵“)
        for (k, (r, s)) in [([76.0, 18.0, 31.0, 13.0], &mut d.width), ([76.0, 35.0, 31.0, 13.0], &mut d.height), ([76.0, 52.0, 31.0, 13.0], &mut d.resolution)].into_iter().enumerate() {
            let te = ui.put(rr(o, r), egui::TextEdit::singleline(s).id(Id::new(("nd-field", k))));
            if focus_field == Some(k) {
                te.request_focus();
                super::select_all(ui.ctx(), te.id, s.chars().count());
            }
            super::autofocus_field(ui, &te, s.chars().count());
            super::select_on_tab(ui, &te, s.chars().count());
        }
        let urefs: Vec<&str> = units.iter().map(|s| s.as_str()).collect();
        let rrefs: Vec<&str> = res_units.iter().map(|s| s.as_str()).collect();
        let cw = |r: Rect| Rect::from_min_size(r.min, vec2(r.width() + dc, r.height()));
        combo(ui, cw(rr(o, [112.0, 18.0, 91.0, 13.0])), "nd-wu", &urefs, &mut d.width_unit);
        combo(ui, cw(rr(o, [112.0, 35.0, 91.0, 13.0])), "nd-hu", &urefs, &mut d.height_unit);
        combo(ui, cw(rr(o, [112.0, 52.0, 91.0, 13.0])), "nd-ru", &rrefs, &mut d.resolution_unit);
        let (pw, ph) = d.pixel_size().unwrap_or((0.0, 0.0));
        let short = |r: Rect| Rect::from_min_size(r.min, vec2(r.width() + dv, r.height()));
        label(ui, short(rr(o, [206.0, 21.0, 12.0, 10.0])), &wh[0]);
        label(ui, rr(o, [218.0, 21.0, 30.0, 10.0]), &format!("{pw:.0}"));
        label(ui, short(rr(o, [206.0, 39.0, 12.0, 10.0])), &wh[1]);
        label(ui, rr(o, [218.0, 39.0, 30.0, 10.0]), &format!("{ph:.0}"));
        group_box(ui, dlu_rect(o, [8.0, 78.0, 242.0, 70.0]).with_max_x(body.right() - dlu(10.0, 0.0).x), &t("dlg-canvas-color-group"));
        // Optionsgruppe = ein Tab-Stopp, Pfeiltasten wählen (Original: Tab … › Weiß › OK)
        let choices = [CanvasChoice::White, CanvasChoice::Transparent, CanvasChoice::Custom];
        let rects: Vec<Rect> = [90.0, 104.0, 118.0]
            .iter()
            .map(|y| {
                let rect = dlu_rect(o, [13.0, *y, 80.0, 11.0]);
                Rect::from_min_size(rect.min, vec2(rect.width().max(radio_w), rect.height()))
            })
            .collect();
        if let Some(k) = pick_radio {
            d.canvas = choices[k];
            ui.memory_mut(|m| m.request_focus(Id::new("nd-canvas").with(k)));
        }
        let sel = choices.iter().position(|c| *c == d.canvas).unwrap_or(0);
        if let Some(k) = super::radio_group(ui, "nd-canvas", &rects, &[radios[0].as_str(), radios[1].as_str(), radios[2].as_str()], sel, true) {
            d.canvas = choices[k];
        }
        for (r, raw) in rects.iter().zip(radios_ak.iter()) {
            underline_access(ui, egui::pos2(r.left() + 17.0, r.center().y), raw);
        }
        let sw = dlu_rect(o, [96.0, 116.0, 14.0, 14.0]).translate(vec2(dsw, 0.0));
        // das Farbfeld ist im Original kein Tab-Stopp; Klick öffnet das Farbwähler-Popup (N27b)
        let skip = egui::Id::new("nd-swatch");
        super::skip_tab_stop_before(ui, skip);
        let cur = fp_color::Argb(0xff00_0000 | (d.custom.r() as u32) << 16 | (d.custom.g() as u32) << 8 | d.custom.b() as u32);
        let sresp = crate::inspector::fields::swatch(ui, "nd-custom-color", sw, Some(cur), true);
        super::skip_tab_stop_after(ui, skip, &sresp);
        if let Some(crate::inspector::popups::ColorChoice::Color(c)) = crate::inspector::color_picker::popup(ui, &sresp, st, Some(cur), crate::inspector::color_picker::Footer::Plain) {
            d.custom = Color32::from_rgb(c.r(), c.g(), c.b());
            d.canvas = CanvasChoice::Custom;
        }
        let _ = ui.allocate_rect(body, Sense::hover());
        // „Vorlagen…“ öffnet wie im Original „Neu von Vorlage“ (Dateiwahl, 7490)
        let by = dlu_rect(o, [0.0, 154.0, 0.0, 14.0]);
        let valid = d.pixel_size().is_some();
        let b = super::button_row(ui.ctx(), body.right() - dlu(10.0, 0.0).x, by.top(), &[&ok_l, &cancel_l], bw, by.height(), gap);
        if push_button(ui, b[0], &ok_l, true, valid) {
            result = Some(true);
        }
        if push_button(ui, b[1], &cancel_l, false, true) {
            result = Some(false);
        }
        // nach OK/Abbrechen gezeichnet: Tab-Folge … › OK › Abbrechen › Vorlagen… wie im Original
        if push_button(ui, Rect::from_min_size(egui::pos2(o.x + dlu(8.0, 0.0).x, by.top()), vec2(bw, by.height())), &tmpl_l, false, true) {
            templates = true;
        }
    });
    match (result, r) {
        _ if templates => {
            st.dialogs.new_document = None;
            super::panel_menus::pick_open(st, &t("dlg-new-from-template"), &[("FW-PNG", &["png"])], |st, path| {
                st.push(Action::Run("file.newFromTemplate".into(), json!({"path": path.to_string_lossy()})));
            });
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
