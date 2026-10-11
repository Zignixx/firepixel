//! Dokument-Dialoge mit eigener Anordnung: Leinwandgröße (5989, Anker im 3×3-Raster), Bildgröße
//! (6011, Pixelabmessungen/Druckgröße mit Einheiten, Proportionen, Neuauflösung) und
//! „Numerisch transformieren“ (6002). Lage der Steuerelemente in DLU wie in `dialoge.json`.

use super::{dialog_frame, dlu, dlu_rect, group_box, push_button, DialogClose};
use crate::app_state::{Action, AppState};
use crate::inspector::fields;
use egui::{vec2, Id, Rect, Ui};
use serde_json::json;

fn label(ui: &mut Ui, o: egui::Pos2, r: [f32; 4], t: &str) {
    fields::label(ui, dlu_rect(o, r), t, true);
}

// ---------------------------------------------------------------------- Leinwandgröße

/// Leinwandgröße (5989).
#[derive(Clone, Debug, PartialEq)]
pub struct CanvasSizeDialog {
    pub width: f64,
    pub height: f64,
    pub cur_w: f64,
    pub cur_h: f64,
    pub unit: usize,
    pub anchor: usize,
    pub current_page_only: bool,
    pub resolution: f64,
}

/// Ankernamen für `modify.canvas.canvasSize`.
pub const ANCHORS: [&str; 9] = ["topLeft", "top", "topRight", "left", "center", "right", "bottomLeft", "bottom", "bottomRight"];
const UNITS: [&str; 3] = ["Pixel", "Zoll", "Zentimeter"];

fn to_unit(px: f64, unit: usize, ppi: f64) -> f64 {
    match unit {
        1 => px / ppi,
        2 => px / ppi * 2.54,
        _ => px,
    }
}

fn from_unit(v: f64, unit: usize, ppi: f64) -> f64 {
    match unit {
        1 => v * ppi,
        2 => v * ppi / 2.54,
        _ => v,
    }
}

impl CanvasSizeDialog {
    /// Werte der aktuellen Seite.
    pub fn new(st: &AppState) -> Option<CanvasSizeDialog> {
        let d = st.session.active()?;
        let c = &d.doc.pages.get(d.page)?.canvas;
        let ppi = if c.resolution_unit == 1 { c.resolution as f64 * 2.54 } else { c.resolution as f64 };
        Some(CanvasSizeDialog { width: c.width as f64, height: c.height as f64, cur_w: c.width as f64, cur_h: c.height as f64, unit: 0, anchor: 4, current_page_only: true, resolution: ppi.max(1.0) })
    }

    /// Parameter (Pixel, gerundet, 1 … 30000) oder `None` bei ungültigen Werten.
    pub fn params(&self) -> Option<serde_json::Value> {
        let w = from_unit(self.width, self.unit, self.resolution).round();
        let h = from_unit(self.height, self.unit, self.resolution).round();
        ((1.0..=30000.0).contains(&w) && (1.0..=30000.0).contains(&h)).then(|| json!({"width": w, "height": h, "anchor": ANCHORS[self.anchor.min(8)]}))
    }
}

/// Zeigt den Dialog.
pub fn show_canvas_size(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.dialogs.canvas_size.clone() else { return };
    let mut result = None;
    let r = dialog_frame(ctx, "fp-canvas-size", "Leinwandgröße", dlu(222.0, 196.0), |ui, body| {
        let o = body.min;
        group_box(ui, dlu_rect(o, [7.0, 6.0, 208.0, 66.0]), "Neue Größe");
        label(ui, o, [20.0, 22.0, 30.0, 10.0], "B:");
        let wtxt = fields::fmt_num(d.width, if d.unit == 0 { 0 } else { 2 });
        if let Some(t) = fields::text(ui, "cs-w", dlu_rect(o, [34.0, 20.0, 50.0, 13.0]), &wtxt, true)
            && let Some(v) = fields::parse_num(&t)
        {
            d.width = v.max(0.0);
        }
        label(ui, o, [20.0, 42.0, 30.0, 10.0], "H:");
        let htxt = fields::fmt_num(d.height, if d.unit == 0 { 0 } else { 2 });
        if let Some(t) = fields::text(ui, "cs-h", dlu_rect(o, [34.0, 40.0, 50.0, 13.0]), &htxt, true)
            && let Some(v) = fields::parse_num(&t)
        {
            d.height = v.max(0.0);
        }
        for (k, key) in [(0usize, "cs-wu"), (1, "cs-hu")] {
            if let Some(u) = fields::combo(ui, key, dlu_rect(o, [88.0, 20.0 + k as f32 * 20.0, 60.0, 13.0]), &UNITS, Some(d.unit), true) {
                let (w, h) = (from_unit(d.width, d.unit, d.resolution), from_unit(d.height, d.unit, d.resolution));
                d.unit = u;
                d.width = to_unit(w, u, d.resolution);
                d.height = to_unit(h, u, d.resolution);
            }
        }
        // Anker 3 × 3 (5791–5800)
        label(ui, o, [158.0, 14.0, 40.0, 10.0], "Anker:");
        for (i, tip) in ANCHORS.iter().enumerate() {
            let r = dlu_rect(o, [158.0 + (i % 3) as f32 * 16.0, 24.0 + (i / 3) as f32 * 14.0, 14.0, 12.0]);
            if fields::toggle(ui, ("cs-anchor", i), r, crate::inspector::glyphs::Glyph::FillSolid, d.anchor == i, true, tip) {
                d.anchor = i;
            }
        }
        group_box(ui, dlu_rect(o, [7.0, 78.0, 208.0, 42.0]), "Aktuelle Größe");
        label(ui, o, [20.0, 92.0, 120.0, 10.0], &format!("B: {:.0} Pixel", d.cur_w));
        label(ui, o, [20.0, 104.0, 120.0, 10.0], &format!("H: {:.0} Pixel", d.cur_h));
        if let Some(b) = fields::checkbox(ui, "cs-page", dlu_rect(o, [9.0, 128.0, 150.0, 10.0]), "Nur aktuelle Seite", d.current_page_only, true) {
            d.current_page_only = b;
        }
        let p = d.params();
        if p.is_none() {
            label(ui, o, [9.0, 144.0, 200.0, 10.0], "Werte zwischen 1 und 30000 Pixel eingeben.");
        }
        if push_button(ui, dlu_rect(o, [110.0, 172.0, 50.0, 14.0]), "OK", true, p.is_some()) {
            result = Some(true);
        }
        if push_button(ui, dlu_rect(o, [165.0, 172.0, 50.0, 14.0]), "Abbrechen", false, true) {
            result = Some(false);
        }
    });
    match (result, r) {
        (Some(true), _) => {
            st.dialogs.canvas_size = None;
            if let Some(p) = d.params() {
                st.push(Action::Run("modify.canvas.canvasSize".into(), p));
            }
        }
        (Some(false), _) | (_, DialogClose::Cancelled) => st.dialogs.canvas_size = None,
        _ => st.dialogs.canvas_size = Some(d),
    }
}

// ---------------------------------------------------------------------- Bildgröße

/// Bildgröße (6011).
#[derive(Clone, Debug, PartialEq)]
pub struct ImageSizeDialog {
    pub cur_w: f64,
    pub cur_h: f64,
    /// Neue Pixelgröße.
    pub w: f64,
    pub h: f64,
    /// Einheit der Pixelabmessungen: 0 Pixel, 1 Prozent.
    pub px_unit: usize,
    /// Einheit der Druckgröße: 0 Prozent, 1 Zoll, 2 Zentimeter.
    pub print_unit: usize,
    /// Auflösung in Pixel/Zoll bzw. Pixel/cm.
    pub resolution: f64,
    pub res_unit: usize,
    pub constrain: bool,
    pub resample: bool,
    pub method: usize,
    pub current_page_only: bool,
}

const METHODS: [&str; 4] = ["Bikubisch", "Bilinear", "Weich", "Nächster Nachbar"];

impl ImageSizeDialog {
    /// Werte der aktuellen Seite.
    pub fn new(st: &AppState) -> Option<ImageSizeDialog> {
        let d = st.session.active()?;
        let c = &d.doc.pages.get(d.page)?.canvas;
        let method = (st.session.prefs.get_int("ImageSizeTransformMode", 1).clamp(1, 4) - 1) as usize;
        Some(ImageSizeDialog {
            cur_w: c.width as f64,
            cur_h: c.height as f64,
            w: c.width as f64,
            h: c.height as f64,
            px_unit: 0,
            print_unit: 1,
            resolution: c.resolution.max(1.0) as f64,
            res_unit: if c.resolution_unit == 1 { 1 } else { 0 },
            constrain: true,
            resample: true,
            method,
            current_page_only: true,
        })
    }

    fn ppi(&self) -> f64 {
        if self.res_unit == 1 { self.resolution * 2.54 } else { self.resolution }
    }

    /// Breite neu setzen (mit Proportionen).
    pub fn set_width(&mut self, w: f64) {
        if self.constrain && self.cur_w > 0.0 {
            self.h = (w * self.cur_h / self.cur_w).round();
        }
        self.w = w;
    }

    /// Höhe neu setzen (mit Proportionen).
    pub fn set_height(&mut self, h: f64) {
        if self.constrain && self.cur_h > 0.0 {
            self.w = (h * self.cur_w / self.cur_h).round();
        }
        self.h = h;
    }

    /// Parameter für `modify.canvas.imageSize` oder `None` bei ungültigen Werten.
    pub fn params(&self) -> Option<serde_json::Value> {
        let (w, h) = (self.w.round(), self.h.round());
        if !(1.0..=30000.0).contains(&w) || !(1.0..=30000.0).contains(&h) || !(1.0..=5000.0).contains(&self.resolution) {
            return None;
        }
        Some(json!({"width": w, "height": h, "constrain": self.constrain, "resample": self.resample, "resolution": self.ppi()}))
    }
}

/// Zeigt den Dialog.
pub fn show_image_size(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.dialogs.image_size.clone() else { return };
    let mut result = None;
    let r = dialog_frame(ctx, "fp-image-size", "Bildgröße", dlu(232.0, 214.0), |ui, body| {
        let o = body.min;
        let en_px = d.resample;
        group_box(ui, dlu_rect(o, [7.0, 6.0, 218.0, 50.0]), "Pixelabmessungen");
        for (k, (lab, key)) in [("Breite:", "w"), ("Höhe:", "h")].iter().enumerate() {
            let y = 20.0 + k as f32 * 18.0;
            label(ui, o, [14.0, y + 2.0, 40.0, 10.0], lab);
            let px = if k == 0 { d.w } else { d.h };
            let cur = if k == 0 { d.cur_w } else { d.cur_h };
            let shown = if d.px_unit == 1 { px / cur.max(1.0) * 100.0 } else { px };
            if let Some(t) = fields::text(ui, ("is-px", *key), dlu_rect(o, [60.0, y, 50.0, 13.0]), &fields::fmt_num(shown, if d.px_unit == 1 { 1 } else { 0 }), en_px)
                && let Some(v) = fields::parse_num(&t)
            {
                let v = if d.px_unit == 1 { v / 100.0 * cur } else { v };
                if k == 0 {
                    d.set_width(v);
                } else {
                    d.set_height(v);
                }
            }
            if let Some(u) = fields::combo(ui, ("is-pxu", k), dlu_rect(o, [114.0, y, 54.0, 13.0]), &["Pixel", "Prozent"], Some(d.px_unit), en_px) {
                d.px_unit = u;
            }
        }
        group_box(ui, dlu_rect(o, [7.0, 62.0, 218.0, 70.0]), "Druckgröße");
        let ppi = d.ppi();
        for (k, lab) in ["Breite:", "Höhe:"].iter().enumerate() {
            let y = 76.0 + k as f32 * 18.0;
            label(ui, o, [14.0, y + 2.0, 40.0, 10.0], lab);
            let px = if k == 0 { d.w } else { d.h };
            let cur = if k == 0 { d.cur_w } else { d.cur_h };
            let shown = match d.print_unit {
                0 => px / cur.max(1.0) * 100.0,
                2 => px / ppi * 2.54,
                _ => px / ppi,
            };
            if let Some(t) = fields::text(ui, ("is-pr", k), dlu_rect(o, [60.0, y, 50.0, 13.0]), &fields::fmt_num(shown, 2), true)
                && let Some(v) = fields::parse_num(&t)
            {
                let px = match d.print_unit {
                    0 => v / 100.0 * cur,
                    2 => v / 2.54 * ppi,
                    _ => v * ppi,
                };
                if d.resample {
                    if k == 0 {
                        d.set_width(px);
                    } else {
                        d.set_height(px);
                    }
                } else if px > 0.0 {
                    // ohne Neuauflösung ändert sich die Auflösung (Photoshop-Logik, A)
                    let inches = if d.print_unit == 2 { v / 2.54 } else { v };
                    if d.print_unit != 0 && inches > 0.0 {
                        let new_ppi = cur / inches;
                        d.resolution = if d.res_unit == 1 { new_ppi / 2.54 } else { new_ppi };
                    }
                }
            }
            if let Some(u) = fields::combo(ui, ("is-pru", k), dlu_rect(o, [114.0, y, 54.0, 13.0]), &["Prozent", "Zoll", "Zentimeter"], Some(d.print_unit), true) {
                d.print_unit = u;
            }
        }
        label(ui, o, [14.0, 114.0, 44.0, 10.0], "Auflösung:");
        if let Some(t) = fields::text(ui, "is-res", dlu_rect(o, [60.0, 112.0, 50.0, 13.0]), &fields::fmt_num(d.resolution, 2), true)
            && let Some(v) = fields::parse_num(&t)
        {
            let old_ppi = d.ppi();
            d.resolution = v.clamp(1.0, 5000.0);
            if d.resample && old_ppi > 0.0 {
                let f = d.ppi() / old_ppi;
                d.w = (d.w * f).round();
                d.h = (d.h * f).round();
            }
        }
        if let Some(u) = fields::combo(ui, "is-resu", dlu_rect(o, [114.0, 112.0, 54.0, 13.0]), &["Pixel/Zoll", "Pixel/cm"], Some(d.res_unit), true) {
            if u != d.res_unit {
                d.resolution = if u == 1 { d.resolution / 2.54 } else { d.resolution * 2.54 };
            }
            d.res_unit = u;
        }
        if let Some(b) = fields::checkbox(ui, "is-constrain", dlu_rect(o, [9.0, 140.0, 120.0, 10.0]), "Proportionen beibehalten", d.constrain, true) {
            d.constrain = b;
        }
        if let Some(b) = fields::checkbox(ui, "is-resample", dlu_rect(o, [9.0, 154.0, 70.0, 10.0]), "Bild neu auflösen", d.resample, true) {
            d.resample = b;
            if !b {
                d.w = d.cur_w;
                d.h = d.cur_h;
            }
        }
        if let Some(m) = fields::combo(ui, "is-method", dlu_rect(o, [84.0, 152.0, 84.0, 13.0]), &METHODS, Some(d.method), d.resample) {
            d.method = m;
        }
        if let Some(b) = fields::checkbox(ui, "is-page", dlu_rect(o, [9.0, 170.0, 120.0, 10.0]), "Nur aktuelle Seite", d.current_page_only, true) {
            d.current_page_only = b;
        }
        let p = d.params();
        if push_button(ui, dlu_rect(o, [120.0, 192.0, 50.0, 14.0]), "OK", true, p.is_some()) {
            result = Some(true);
        }
        if push_button(ui, dlu_rect(o, [175.0, 192.0, 50.0, 14.0]), "Abbrechen", false, true) {
            result = Some(false);
        }
    });
    match (result, r) {
        (Some(true), _) => {
            st.dialogs.image_size = None;
            if let Some(p) = d.params() {
                st.session.prefs.set("ImageSizeTransformMode", fp_engine::PrefValue::Int(d.method as i64 + 1));
                st.push(Action::Run("modify.canvas.imageSize".into(), p));
            }
        }
        (Some(false), _) | (_, DialogClose::Cancelled) => st.dialogs.image_size = None,
        _ => st.dialogs.image_size = Some(d),
    }
}

// ---------------------------------------------------------------------- Numerisch transformieren

/// Numerisch transformieren (6002).
#[derive(Clone, Debug, PartialEq)]
pub struct NumericTransformDialog {
    /// 0 Skalieren, 1 Größe ändern, 2 Drehen.
    pub mode: usize,
    pub sx: f64,
    pub sy: f64,
    pub w: f64,
    pub h: f64,
    pub cur_w: f64,
    pub cur_h: f64,
    pub angle: f64,
    pub scale_attrs: bool,
    pub constrain: bool,
    pub interpolate: bool,
}

impl NumericTransformDialog {
    /// Werte der Auswahl.
    pub fn new(st: &AppState) -> Option<NumericTransformDialog> {
        let d = st.session.active()?;
        let b = fp_engine::docops::ids_bounds(&d.doc, d.page, &d.selection.elements)?;
        Some(NumericTransformDialog { mode: 0, sx: 100.0, sy: 100.0, w: b.width() as f64, h: b.height() as f64, cur_w: b.width() as f64, cur_h: b.height() as f64, angle: 0.0, scale_attrs: st.session.prefs.get_bool("XformToolXformsBFE", true), constrain: true, interpolate: true })
    }

    /// Parameter für `modify.transform.numericTransform`.
    pub fn params(&self) -> Option<serde_json::Value> {
        match self.mode {
            1 => ((1.0..=30000.0).contains(&self.w) && (1.0..=30000.0).contains(&self.h)).then(|| json!({"mode": "resize", "width": self.w, "height": self.h, "constrain": self.constrain})),
            2 => self.angle.is_finite().then(|| json!({"mode": "rotate", "angle": self.angle})),
            _ => (self.sx > 0.0 && self.sy > 0.0 && self.sx <= 10000.0 && self.sy <= 10000.0).then(|| json!({"mode": "scale", "scaleX": self.sx, "scaleY": self.sy, "constrain": self.constrain})),
        }
    }
}

/// Zeigt den Dialog.
pub fn show_numeric_transform(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.dialogs.numeric_transform.clone() else { return };
    let mut result = None;
    let r = dialog_frame(ctx, "fp-numeric", "Numerisch transformieren", dlu(214.0, 130.0), |ui, body| {
        let o = body.min;
        if let Some(m) = fields::combo(ui, "nt-mode", dlu_rect(o, [8.0, 8.0, 120.0, 13.0]), &["Skalieren", "Größe ändern", "Drehen"], Some(d.mode), true) {
            d.mode = m;
        }
        match d.mode {
            2 => {
                label(ui, o, [14.0, 34.0, 40.0, 10.0], "Winkel:");
                if let Some(t) = fields::text(ui, "nt-angle", dlu_rect(o, [60.0, 32.0, 40.0, 13.0]), &fields::fmt_num(d.angle, 1), true)
                    && let Some(v) = fields::parse_num(&t)
                {
                    d.angle = v.clamp(-360.0, 360.0);
                }
                label(ui, o, [104.0, 34.0, 10.0, 10.0], "°");
            }
            m => {
                let (a, b, unit) = if m == 1 { (d.w, d.h, "Pixel") } else { (d.sx, d.sy, "%") };
                for (k, (lab, v)) in [("Breite:", a), ("Höhe:", b)].iter().enumerate() {
                    let y = 32.0 + k as f32 * 18.0;
                    label(ui, o, [14.0, y + 2.0, 40.0, 10.0], lab);
                    if let Some(t) = fields::text(ui, ("nt-v", m, k), dlu_rect(o, [60.0, y, 50.0, 13.0]), &fields::fmt_num(*v, 1), true)
                        && let Some(nv) = fields::parse_num(&t)
                    {
                        let nv = nv.max(0.0);
                        if m == 1 {
                            if k == 0 {
                                if d.constrain && d.cur_w > 0.0 {
                                    d.h = (nv * d.cur_h / d.cur_w).round();
                                }
                                d.w = nv;
                            } else {
                                if d.constrain && d.cur_h > 0.0 {
                                    d.w = (nv * d.cur_w / d.cur_h).round();
                                }
                                d.h = nv;
                            }
                        } else if k == 0 {
                            d.sx = nv;
                            if d.constrain {
                                d.sy = nv;
                            }
                        } else {
                            d.sy = nv;
                            if d.constrain {
                                d.sx = nv;
                            }
                        }
                    }
                    label(ui, o, [114.0, y + 2.0, 30.0, 10.0], unit);
                }
            }
        }
        if let Some(b) = fields::checkbox(ui, "nt-attrs", dlu_rect(o, [8.0, 74.0, 120.0, 10.0]), "Attribute skalieren", d.scale_attrs, d.mode != 2) {
            d.scale_attrs = b;
        }
        if let Some(b) = fields::checkbox(ui, "nt-constrain", dlu_rect(o, [8.0, 88.0, 120.0, 10.0]), "Proportionen beibehalten", d.constrain, d.mode != 2) {
            d.constrain = b;
        }
        let p = d.params();
        if push_button(ui, dlu_rect(o, [150.0, 8.0, 56.0, 14.0]), "OK", true, p.is_some()) {
            result = Some(true);
        }
        if push_button(ui, dlu_rect(o, [150.0, 26.0, 56.0, 14.0]), "Abbrechen", false, true) {
            result = Some(false);
        }
        let _ = Rect::NOTHING;
        let _ = vec2(0.0, 0.0);
        let _ = Id::NULL;
    });
    match (result, r) {
        (Some(true), _) => {
            st.dialogs.numeric_transform = None;
            if let Some(p) = d.params() {
                st.push(Action::Run("modify.transform.numericTransform".into(), p));
            }
        }
        (Some(false), _) | (_, DialogClose::Cancelled) => st.dialogs.numeric_transform = None,
        _ => st.dialogs.numeric_transform = Some(d),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn st_with_doc() -> AppState {
        let mut s = fp_engine::Session::default();
        let _ = s.execute("file.new", json!({"width": 200, "height": 100}));
        AppState::new(s, crate::app_state::UiSettings::default(), None)
    }

    #[test]
    fn canvas_and_image_size_validation() {
        let st = st_with_doc();
        let mut c = CanvasSizeDialog::new(&st).expect("Dialog");
        assert_eq!(c.params().map(|p| p["width"].clone()), Some(json!(200.0)));
        c.width = 0.0;
        assert!(c.params().is_none());
        c.width = 2.0;
        c.unit = 1;
        assert_eq!(c.params().map(|p| p["width"].clone()), Some(json!(144.0)));
        let mut i = ImageSizeDialog::new(&st).expect("Dialog");
        i.set_width(400.0);
        assert_eq!(i.h, 200.0);
        i.constrain = false;
        i.set_height(50.0);
        assert_eq!((i.w, i.h), (400.0, 50.0));
        i.w = 40000.0;
        assert!(i.params().is_none());
    }
}
