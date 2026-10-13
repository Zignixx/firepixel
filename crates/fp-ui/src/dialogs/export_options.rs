//! **Optionen-Dialoge des Exports** (re\18-export-dialoge.md §4, Paket E2): „Optionen…“ im Dialog
//! „Exportieren“ öffnet je Art einen eigenen kleinen Dialog:
//! - PDF ⇒ „PDF-Exportoptionen“ (§4.1; Werte in den Voreinstellungen `PDFExport_*`, Listen 1-basiert,
//!   Kennwörter nur für diesen Export, nie gemerkt)
//! - CSS-Sprites ⇒ „Exportoptionen für CSS-Sprites“ (§4.2; Werte im Dokument, Vorgaben für neue
//!   Dokumente aus den Voreinstellungen `Sprite*`)
//! - Ebenen/Status/Seiten in Dateien ⇒ „Exportoptionen - <Art>“ (§4.3; Präfix, Suffix, Dateityp in den
//!   Voreinstellungen `FilesExport_<Art>_*`, nach §11)
//! - Photoshop PSD ⇒ „Photoshop-Exportoptionen“ (§4.5; Voreinstellungen `PsdExport_*`, 1-basiert)
//! - HTML-Arten ⇒ weiter die Bildvorschau (HTML-Einrichtung: Paket E3); Nur Bilder und SVG: grau.
//!
//! Der Dialog arbeitet auf einer Kopie; OK übernimmt sie in den Exportdialog (und die
//! Voreinstellungen), Abbrechen verwirft sie.

use super::{dialog_frame, push_button, refs, t, DialogClose};
use crate::app_state::AppState;
use crate::inspector::fields;
use crate::panels::optimize;
use egui::{vec2, Rect};
use fp_engine::prefs::PrefValue;
use fp_export::doc_export::{FileAffix, FilesStyle};
use fp_export::options::OptimizeSettings;
use fp_export::sprites::{SpriteLayout, SpriteOptions};

/// Was „Optionen…“ je Art öffnet (Tabelle am Anfang von re\18 §2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptionsTarget {
    Pdf,
    Sprites,
    Files,
    Psd,
    /// HTML-Arten: Bildvorschau (bis die HTML-Einrichtung angebunden ist).
    ImagePreview,
    /// Knopf grau (Nur Bilder, SVG).
    None,
}

/// Ziel des Knopfs „Optionen…“ für eine Art.
pub fn options_target(kind: FilesStyle) -> OptionsTarget {
    match kind {
        FilesStyle::Pdf => OptionsTarget::Pdf,
        FilesStyle::CssSprites => OptionsTarget::Sprites,
        FilesStyle::LayersToFiles | FilesStyle::FramesToFiles | FilesStyle::PagesToFiles => OptionsTarget::Files,
        FilesStyle::Psd => OptionsTarget::Psd,
        FilesStyle::HtmlAndImages | FilesStyle::CssAndImages => OptionsTarget::ImagePreview,
        FilesStyle::ImagesOnly | FilesStyle::Svg => OptionsTarget::None,
    }
}

/// Eingebaute Vorgaben der Listen „Vorgabe für die Optimierung“ und „Dateityp“ (re\18 §4.2, §4.3)
/// in der Listenreihenfolge PNG32, JPEG höhere Qualität, JPEG kleinere Datei, GIF Web 216, GIF
/// WebSnap 256, GIF WebSnap 128, GIF adaptiv 256, animiertes GIF WebSnap 128 – als Plätze in
/// `fp_export::options::builtin_presets` (gemerkt wird der Name der Vorgabe).
const PRESET_ORDER: [usize; 8] = [9, 4, 5, 0, 1, 2, 3, 6];

/// Namen der acht Vorgaben (wie in `builtin_presets`).
pub fn sprite_presets() -> Vec<&'static str> {
    let all = fp_export::options::builtin_presets();
    PRESET_ORDER.iter().filter_map(|&i| all.get(i).map(|p| p.name)).collect()
}

fn preset_name(i: usize) -> &'static str {
    let v = sprite_presets();
    v.get(i).or(v.first()).copied().unwrap_or_default()
}

fn preset_pos(name: &str) -> Option<usize> {
    sprite_presets().iter().position(|n| *n == name)
}

/// Optimieren-Einstellungen einer eingebauten Vorgabe.
pub fn preset_settings(name: &str) -> Option<OptimizeSettings> {
    fp_export::options::builtin_presets().into_iter().find(|p| p.name == name).map(|p| p.settings)
}

fn preset_items() -> Vec<String> {
    sprite_presets().iter().map(|n| optimize::preset_label(n)).collect()
}

const SEP: &str = "——";

// ---------------------------------------------------------------------- Werte

/// „PDF-Exportoptionen“ (§4.1). Listenplätze 0-basiert.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PdfOpts {
    /// 0 = PDF 1.3 … 3 = PDF 1.6 (Vorgabe).
    pub compatibility: usize,
    /// 0 Ohne, 1 ZIP, 2 JPEG (Vorgabe), 3 JPEG 2000.
    pub compression: usize,
    /// 0 Hoch (Vorgabe) … 4 Niedrig.
    pub quality: usize,
    pub grayscale: bool,
    pub selectable_text: bool,
    /// Anschnitt in px.
    pub bleed: i32,
    pub use_open_password: bool,
    pub open_password: String,
    pub use_permissions_password: bool,
    pub permissions_password: String,
    pub allow_print: bool,
    pub allow_copy: bool,
    pub allow_edit: bool,
    pub allow_comment: bool,
}

/// JPEG-Qualität je Stufe (A: die Zuordnung des Originals ist nicht gemessen).
pub const PDF_QUALITY: [i32; 5] = [90, 75, 60, 45, 30];

impl PdfOpts {
    pub fn from_prefs(st: &AppState) -> PdfOpts {
        let p = &st.session.prefs;
        let idx = |k: &str, d: i64, n: usize| (p.get_int(k, d) - 1).clamp(0, n as i64 - 1) as usize;
        PdfOpts {
            compatibility: idx("PDFExport_Compatibility", 4, 4),
            compression: idx("PDFExport_Compression", 3, 4),
            quality: idx("PDFExport_Quality", 1, 5),
            grayscale: p.get_bool("PDFExport_GrayScale", false),
            selectable_text: p.get_bool("PDFExport_SelectableText", true),
            bleed: p.get_int("PDFExport_BleedValue", 0).clamp(0, 9999) as i32,
            use_open_password: false,
            open_password: String::new(),
            use_permissions_password: false,
            permissions_password: String::new(),
            allow_print: false,
            allow_copy: false,
            allow_edit: false,
            allow_comment: false,
        }
    }

    /// OK: in die Voreinstellungen (ohne Kennwörter, §7.3).
    pub fn to_prefs(&self, st: &mut AppState) {
        let p = &mut st.session.prefs;
        p.set("PDFExport_Compatibility", PrefValue::Int(self.compatibility as i64 + 1));
        p.set("PDFExport_Compression", PrefValue::Int(self.compression as i64 + 1));
        p.set("PDFExport_Quality", PrefValue::Int(self.quality as i64 + 1));
        p.set("PDFExport_GrayScale", PrefValue::Bool(self.grayscale));
        p.set("PDFExport_SelectableText", PrefValue::Bool(self.selectable_text));
        p.set("PDFExport_BleedValue", PrefValue::Int(self.bleed as i64));
        st.session.prefs_changed();
    }

    /// Optionen für den Schreiber; `dpi` = Auflösung der Seite (Anschnitt px ⇒ pt).
    pub fn writer(&self, dpi: f32) -> fp_export::vector_out::PdfOptions {
        use fp_export::vector_out::PdfCompression as C;
        fp_export::vector_out::PdfOptions {
            minor_version: 3 + self.compatibility.min(3) as u8,
            compression: [C::None, C::Zip, C::Jpeg, C::Jpeg2000][self.compression.min(3)],
            jpeg_quality: PDF_QUALITY[self.quality.min(4)],
            grayscale: self.grayscale,
            bleed_pt: self.bleed.max(0) as f64 * 72.0 / dpi.max(1.0) as f64,
            open_password: if self.use_open_password { self.open_password.clone() } else { String::new() },
            permissions_password: (self.use_permissions_password && !self.permissions_password.is_empty()).then(|| {
                (self.permissions_password.clone(), fp_export::pdf_crypt::PdfPermissions { print: self.allow_print, copy: self.allow_copy, edit: self.allow_edit, comment: self.allow_comment })
            }),
        }
    }
}

/// „Exportoptionen für CSS-Sprites“ (§4.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpriteOpts {
    pub selector_is_class: bool,
    pub prefix: String,
    pub suffix: String,
    pub padding: i32,
    /// 0 Horizontal, 1 Vertikal, 2 Raster (Vorgabe).
    pub layout: usize,
    /// Platz in [`sprite_presets`] (0 = PNG32).
    pub preset: usize,
}

impl SpriteOpts {
    /// Aus dem Dokument, sonst aus den Vorgaben für neue Dokumente (`Sprite*`).
    pub fn load(st: &AppState, h: &fp_model::HtmlSettings) -> SpriteOpts {
        let p = &st.session.prefs;
        let pref_preset = p.get_str("SpriteExportPreset", "");
        let preset_name = h.sprite_preset.clone().unwrap_or(pref_preset);
        SpriteOpts {
            selector_is_class: h.sprite_selector_is_class.unwrap_or_else(|| p.get_bool("SpriteSelectorIsClass", true)),
            prefix: h.sprite_css_prefix.clone().unwrap_or_else(|| p.get_str("SpriteCssPrefix", "")),
            suffix: h.sprite_css_suffix.clone().unwrap_or_else(|| p.get_str("SpriteCssSuffix", "")),
            // das Dokument eines neuen Dokuments hat 10 (gemessen), die Voreinstellung 11
            padding: h.sprite_padding.unwrap_or(10).clamp(0, 999),
            layout: (h.sprite_orientation.map(|v| v as i64).unwrap_or_else(|| p.get_int("SpriteOrientation", 3)) - 1).clamp(0, 2) as usize,
            preset: preset_pos(&preset_name).unwrap_or(0),
        }
    }

    /// Beim Export ins Dokument (§7.2).
    pub fn store(&self, h: &mut fp_model::HtmlSettings) {
        h.sprite_selector_is_class = Some(self.selector_is_class);
        h.sprite_css_prefix = Some(self.prefix.clone());
        h.sprite_css_suffix = Some(self.suffix.clone());
        h.sprite_padding = Some(self.padding);
        h.sprite_orientation = Some(self.layout as i32 + 1);
        h.sprite_preset = Some(preset_name(self.preset).to_string());
    }

    pub fn writer(&self) -> SpriteOptions {
        SpriteOptions {
            selector_is_class: self.selector_is_class,
            prefix: self.prefix.clone(),
            suffix: self.suffix.clone(),
            padding: self.padding.max(0) as usize,
            layout: [SpriteLayout::Horizontal, SpriteLayout::Vertical, SpriteLayout::Grid][self.layout.min(2)],
        }
    }

    /// Optimieren-Einstellungen des Sprite-Bilds (`None` = PNG32).
    pub fn settings(&self) -> Option<OptimizeSettings> {
        (self.preset > 0).then(|| preset_settings(preset_name(self.preset))).flatten()
    }

    /// Endung des Sprite-Bilds.
    pub fn ext(&self) -> String {
        self.settings().map(|s| still_format(s.format).extension().to_string()).unwrap_or_else(|| "png".into())
    }

    /// „Beispiel:“ – Selektor für ein Segment „Segment“.
    pub fn example(&self) -> String {
        fp_export::sprites::sprite_selector(&t("dlg-expopt-sprite-sample"), &self.writer())
    }
}

fn still_format(f: fp_export::ExportFormat) -> fp_export::ExportFormat {
    use fp_export::ExportFormat as F;
    match f {
        F::AnimatedGif => F::Gif,
        F::Apng => F::Png,
        f if f.is_animated() => F::WebPLossless,
        f => f,
    }
}

/// „Exportoptionen - Ebenen/Status/Seiten in Dateien“ (§4.3).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct FilesOpts {
    pub add_prefix: bool,
    /// 0 Keine, 1 Dokumentname, 2 # (1, 2, 3…), 3 # (01, 02, 03…).
    pub prefix: usize,
    pub add_suffix: bool,
    pub suffix: usize,
    /// 0 = Dokumentoptimierungseinstellungen, sonst 1 + Platz in [`sprite_presets`].
    pub file_type: usize,
}

/// Technischer Name der Art in den Voreinstellungen.
fn files_key(kind: FilesStyle) -> &'static str {
    match kind {
        FilesStyle::LayersToFiles => "Layers",
        FilesStyle::FramesToFiles => "Frames",
        _ => "Pages",
    }
}

impl FilesOpts {
    pub fn from_prefs(st: &AppState, kind: FilesStyle) -> FilesOpts {
        let p = &st.session.prefs;
        let k = files_key(kind);
        let affix = |name: &str| p.get_int(&format!("FilesExport_{k}_{name}"), 0).clamp(0, 3) as usize;
        let ty = p.get_str(&format!("FilesExport_{k}_FileType"), "");
        FilesOpts {
            add_prefix: affix("Prefix") > 0,
            prefix: affix("Prefix"),
            add_suffix: affix("Suffix") > 0,
            suffix: affix("Suffix"),
            file_type: preset_pos(&ty).map(|i| i + 1).unwrap_or(0),
        }
    }

    pub fn to_prefs(&self, st: &mut AppState, kind: FilesStyle) {
        let k = files_key(kind);
        let p = &mut st.session.prefs;
        p.set(&format!("FilesExport_{k}_Prefix"), PrefValue::Int(if self.add_prefix { self.prefix as i64 } else { 0 }));
        p.set(&format!("FilesExport_{k}_Suffix"), PrefValue::Int(if self.add_suffix { self.suffix as i64 } else { 0 }));
        let ty = if self.file_type == 0 { String::new() } else { preset_name(self.file_type - 1).to_string() };
        p.set(&format!("FilesExport_{k}_FileType"), PrefValue::Str(ty));
        st.session.prefs_changed();
    }

    pub fn affix(on: bool, v: usize) -> FileAffix {
        if !on {
            return FileAffix::None;
        }
        [FileAffix::None, FileAffix::DocName, FileAffix::Number, FileAffix::Number2][v.min(3)]
    }

    /// Optimieren-Einstellungen des Dateityps (`None` = die des Dokuments).
    pub fn settings(&self) -> Option<OptimizeSettings> {
        (self.file_type > 0).then(|| preset_settings(preset_name(self.file_type - 1))).flatten()
    }

    /// Endung des Dateityps (`None` = die des Dokuments).
    pub fn ext(&self) -> Option<String> {
        self.settings().map(|s| still_format(s.format).extension().to_string())
    }
}

/// „Photoshop-Exportoptionen“ (§4.5). Listenplätze 0-basiert.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct PsdOpts {
    /// 0 Bearbeitbarkeit, 1 Erscheinungsbild, 2 Kleinere Datei, 3 Benutzerdefiniert.
    pub settings: usize,
    /// Objekte: 0 in Ebenen konvertieren, 1 auf Hintergrundebene reduzieren.
    pub objects: usize,
    /// Effekte: 0 Bearbeitbarkeit, 1 wiedergeben.
    pub effects: usize,
    /// Vektor: 0 Bearbeitbarkeit, 1 Erscheinungsbild.
    pub vector: usize,
    /// Text: 0 Bearbeitbarkeit, 1 rendern.
    pub text: usize,
}

impl PsdOpts {
    pub fn from_prefs(st: &AppState) -> PsdOpts {
        let p = &st.session.prefs;
        let idx = |k: &str, n: i64| (p.get_int(k, 1) - 1).clamp(0, n - 1) as usize;
        PsdOpts { settings: idx("PsdExport_Settings", 4), objects: idx("PsdExport_Layers", 2), effects: idx("PsdExport_Effects", 2), vector: idx("PsdExport_Vector", 2), text: idx("PsdExport_Text", 2) }
    }

    pub fn to_prefs(&self, st: &mut AppState) {
        let p = &mut st.session.prefs;
        for (k, v) in [("PsdExport_Settings", self.settings), ("PsdExport_Layers", self.objects), ("PsdExport_Effects", self.effects), ("PsdExport_Vector", self.vector), ("PsdExport_Text", self.text)] {
            p.set(k, PrefValue::Int(v as i64 + 1));
        }
        st.session.prefs_changed();
    }

    /// Eine Vorgabe setzt die vier Einzellisten (§4.5); „Benutzerdefiniert“ lässt sie stehen.
    pub fn set_settings(&mut self, i: usize) {
        self.settings = i.min(3);
        let v = match self.settings {
            0 => [0, 0, 0, 0],
            1 => [0, 1, 1, 1],
            2 => [1, 1, 1, 1],
            _ => return,
        };
        (self.objects, self.effects, self.vector, self.text) = (v[0], v[1], v[2], v[3]);
    }
}

// ---------------------------------------------------------------------- Dialog

/// Offener Optionen-Dialog mit seiner Arbeitskopie.
#[derive(Clone, Debug, PartialEq)]
pub enum OptionsDialog {
    Pdf(PdfOpts),
    Sprites(SpriteOpts),
    Files(FilesStyle, FilesOpts),
    Psd(PsdOpts),
}

/// Zeigt den offenen Optionen-Dialog; `Some(true)` = OK, `Some(false)` = Abbrechen.
pub fn show(ctx: &egui::Context, dlg: &mut OptionsDialog, kind_name: &str) -> Option<bool> {
    match dlg {
        OptionsDialog::Pdf(o) => show_pdf(ctx, o),
        OptionsDialog::Sprites(o) => show_sprites(ctx, o),
        OptionsDialog::Files(_, o) => show_files(ctx, o, kind_name),
        OptionsDialog::Psd(o) => show_psd(ctx, o),
    }
}

/// OK/Abbrechen unten rechts; liefert die Antwort.
fn ok_cancel(ui: &mut egui::Ui, body: Rect) -> Option<bool> {
    let (ok_l, cancel_l) = (t("btn-ok"), t("btn-cancel"));
    let rects = super::button_row(ui.ctx(), body.right() - 12.0, body.bottom() - 35.0, &[&ok_l, &cancel_l], 75.0, 23.0, 8.0);
    if push_button(ui, rects[0], &ok_l, true, true) {
        return Some(true);
    }
    if push_button(ui, rects[1], &cancel_l, false, true) {
        return Some(false);
    }
    None
}

fn closed(c: DialogClose, ans: Option<bool>) -> Option<bool> {
    if c == DialogClose::Cancelled { Some(false) } else { ans }
}

/// Zeilenraster: Beschriftung links (Breite `lw`), Feld rechts.
struct Grid {
    o: egui::Pos2,
    lw: f32,
}

impl Grid {
    fn label(&self, y: f32) -> Rect {
        Rect::from_min_size(self.o + vec2(0.0, y + 1.0), vec2(self.lw, 20.0))
    }
    fn field(&self, y: f32, w: f32) -> Rect {
        Rect::from_min_size(self.o + vec2(self.lw + 8.0, y), vec2(w, 22.0))
    }
}

fn show_pdf(ctx: &egui::Context, o: &mut PdfOpts) -> Option<bool> {
    let l = |k: &str| t(k);
    let (compat_l, comp_l, qual_l, bleed_l, open_l, perm_l, allow_l) = (l("dlg-expopt-pdf-compat"), l("dlg-expopt-pdf-compression"), l("dlg-expopt-pdf-quality"), l("dlg-expopt-pdf-bleed"), l("dlg-expopt-pdf-open"), l("dlg-expopt-pdf-perm"), l("dlg-expopt-pdf-allow"));
    let compat_items: Vec<String> = (3..=6).map(|v| format!("PDF 1.{v}")).collect();
    let comp_items = vec![l("dlg-export-none"), "ZIP".into(), "JPEG".into(), "JPEG 2000".into()];
    let qual_items: Vec<String> = ["high", "medium-high", "medium", "medium-low", "low"].iter().map(|k| l(&format!("dlg-expopt-pdf-quality-{k}"))).collect();
    let (gray_l, text_l, use_open_l, use_perm_l) = (l("dlg-expopt-pdf-gray"), l("dlg-expopt-pdf-text"), l("dlg-expopt-pdf-use-open"), l("dlg-expopt-pdf-use-perm"));
    let allow = [l("dlg-expopt-pdf-print"), l("dlg-expopt-pdf-copy"), l("dlg-expopt-pdf-edit"), l("dlg-expopt-pdf-comment")];
    let lw = super::max_text_w(ctx, &[&compat_l, &comp_l, &qual_l, &bleed_l, &open_l, &perm_l, &allow_l]).max(110.0).ceil();
    let fw = super::combo_fit_w(ctx, &refs(&qual_items), 150.0).max(super::combo_fit_w(ctx, &refs(&compat_items), 150.0));
    let aw = allow.iter().map(|a| 22.0 + super::text_w(ctx, a)).fold(0f32, f32::max).ceil();
    let cw = [&gray_l, &text_l].iter().map(|a| 22.0 + super::text_w(ctx, a)).fold(0f32, f32::max).ceil();
    let head_w = [&use_open_l, &use_perm_l].iter().map(|a| 22.0 + super::text_w(ctx, a)).fold(0f32, f32::max).ceil();
    let width = (32.0 + lw + 8.0 + fw.max(cw).max(aw * 2.0 + 8.0)).max(32.0 + head_w).max(342.0).ceil();
    let mut ans = None;
    let c = dialog_frame(ctx, "fp-export-pdf-options", &t("dlg-expopt-pdf-title"), vec2(width, 400.0), |ui, body| {
        let g = Grid { o: body.min + vec2(16.0, 14.0), lw };
        super::fit_label(ui, g.label(0.0), &compat_l, true);
        if let Some(i) = fields::combo(ui, "pdf-compat", g.field(0.0, fw), &refs(&compat_items), Some(o.compatibility), true) {
            o.compatibility = i;
        }
        super::fit_label(ui, g.label(28.0), &comp_l, true);
        if let Some(i) = fields::combo(ui, "pdf-comp", g.field(28.0, fw), &refs(&comp_items), Some(o.compression), true) {
            o.compression = i;
        }
        // „Qualität“ wirkt nur bei JPEG/JPEG 2000 (A: grau bei Ohne/ZIP)
        let q_on = o.compression >= 2;
        super::fit_label(ui, g.label(56.0), &qual_l, q_on);
        if let Some(i) = fields::combo(ui, "pdf-quality", g.field(56.0, fw), &refs(&qual_items), Some(o.quality), q_on) {
            o.quality = i;
        }
        let check = |ui: &mut egui::Ui, id: &str, r: Rect, text: &str, v: &mut bool, on: bool| {
            if let Some(b) = fields::checkbox(ui, id, r, text, *v, on) {
                *v = b;
            }
        };
        let fx = g.field(0.0, 0.0).min.x;
        check(ui, "pdf-gray", Rect::from_min_size(egui::pos2(fx, g.o.y + 84.0), vec2(cw, 18.0)), &gray_l, &mut o.grayscale, true);
        check(ui, "pdf-text", Rect::from_min_size(egui::pos2(fx, g.o.y + 106.0), vec2(cw, 18.0)), &text_l, &mut o.selectable_text, true);
        super::fit_label(ui, g.label(130.0), &bleed_l, true);
        if let Some(v) = fields::number(ui, "pdf-bleed", g.field(130.0, 60.0), None, Some(o.bleed as f64), (0.0, 9999.0), 0, true) {
            o.bleed = v.round() as i32;
        }
        ui.painter().text(g.field(130.0, 60.0).right_center() + vec2(6.0, 0.0), egui::Align2::LEFT_CENTER, t("unit-px"), crate::theme::Theme::current(ui.ctx()).ui_font(), crate::theme::Theme::current(ui.ctx()).colors.text);
        // Kennwort zum Öffnen
        check(ui, "pdf-use-open", Rect::from_min_size(g.o + vec2(0.0, 168.0), vec2(head_w, 18.0)), &use_open_l, &mut o.use_open_password, true);
        super::fit_label(ui, g.label(192.0), &open_l, o.use_open_password);
        password(ui, "pdf-open-pw", g.field(192.0, fw), &mut o.open_password, o.use_open_password);
        // Kennwort zum Einschränken von Aufgaben
        check(ui, "pdf-use-perm", Rect::from_min_size(g.o + vec2(0.0, 230.0), vec2(head_w, 18.0)), &use_perm_l, &mut o.use_permissions_password, true);
        let on = o.use_permissions_password;
        super::fit_label(ui, g.label(254.0), &perm_l, on);
        password(ui, "pdf-perm-pw", g.field(254.0, fw), &mut o.permissions_password, on);
        super::fit_label(ui, g.label(284.0), &allow_l, on);
        let vals = [&mut o.allow_print, &mut o.allow_copy, &mut o.allow_edit, &mut o.allow_comment];
        for (n, (v, text)) in vals.into_iter().zip(allow.iter()).enumerate() {
            let r = Rect::from_min_size(egui::pos2(fx + (n % 2) as f32 * (aw + 8.0), g.o.y + 284.0 + (n / 2) as f32 * 22.0), vec2(aw, 18.0));
            check(ui, &format!("pdf-allow-{n}"), r, text, v, on);
        }
        ans = ok_cancel(ui, body);
    });
    closed(c, ans)
}

/// Kennwortfeld (Zeichen verdeckt).
fn password(ui: &mut egui::Ui, id: &str, r: Rect, value: &mut String, enabled: bool) {
    fields::field_frame(ui, r, enabled, false);
    let te = egui::TextEdit::singleline(value).password(true).frame(egui::Frame::NONE).id_salt(id).margin(vec2(4.0, 2.0));
    ui.add_enabled_ui(enabled, |ui| ui.put(r.shrink(1.0), te));
}

/// Textfeld, das jede Eingabe sofort in `value` schreibt.
fn live_text(ui: &mut egui::Ui, id: &str, r: Rect, value: &mut String) {
    fields::field_frame(ui, r, true, false);
    let theme = crate::theme::Theme::current(ui.ctx());
    let te = egui::TextEdit::singleline(value).frame(egui::Frame::NONE).id_salt(id).font(theme.ui_font()).text_color(theme.colors.field_text).margin(vec2(4.0, 2.0)).vertical_align(egui::Align::Center);
    ui.put(r.shrink(1.0), te);
}

fn show_sprites(ctx: &egui::Context, o: &mut SpriteOpts) -> Option<bool> {
    let l = |k: &str| t(k);
    let (sel_l, pre_l, suf_l, pad_l, lay_l, preset_l) = (l("dlg-expopt-sprite-selector"), l("dlg-expopt-prefix"), l("dlg-expopt-suffix"), l("dlg-expopt-sprite-padding"), l("dlg-expopt-sprite-layout"), l("dlg-expopt-preset"));
    let (class_l, id_l) = (l("dlg-expopt-sprite-class"), l("dlg-expopt-sprite-id"));
    let lay_items = vec![l("dlg-expopt-layout-horizontal"), l("dlg-expopt-layout-vertical"), l("dlg-expopt-layout-grid")];
    let presets = preset_items();
    let lw = super::max_text_w(ctx, &[&sel_l, &pre_l, &suf_l, &pad_l, &lay_l, &preset_l]).max(100.0).ceil();
    let fw = super::combo_fit_w(ctx, &refs(&presets), 198.0);
    let example = fp_i18n::tr!("dlg-expopt-sprite-example", selector = o.example());
    let width = (32.0 + lw + 8.0 + fw).max(32.0 + super::text_w(ctx, &example)).max(347.0).ceil();
    let mut ans = None;
    let c = dialog_frame(ctx, "fp-export-sprite-options", &t("dlg-expopt-sprite-title"), vec2(width, 262.0), |ui, body| {
        let g = Grid { o: body.min + vec2(16.0, 14.0), lw };
        super::fit_label(ui, g.label(0.0), &sel_l, true);
        let f = g.field(0.0, fw);
        let rw = (22.0 + super::text_w(ui.ctx(), &class_l)).max(22.0 + super::text_w(ui.ctx(), &id_l)).ceil() + 8.0;
        if fields::radio(ui, "sprite-class", Rect::from_min_size(f.min, vec2(rw, 20.0)), &class_l, o.selector_is_class, true) {
            o.selector_is_class = true;
        }
        if fields::radio(ui, "sprite-id", Rect::from_min_size(f.min + vec2(rw, 0.0), vec2(rw, 20.0)), &id_l, !o.selector_is_class, true) {
            o.selector_is_class = false;
        }
        super::fit_label(ui, g.label(28.0), &pre_l, true);
        // Präfix/Suffix wirken sofort (Beispielzeile live wie im Original, OK übernimmt den
        // getippten Wert auch ohne Eingabe/Fokuswechsel; CT-1a: „1 x“ ging still verloren)
        live_text(ui, "sprite-prefix", g.field(28.0, fw), &mut o.prefix);
        super::fit_label(ui, g.label(56.0), &suf_l, true);
        live_text(ui, "sprite-suffix", g.field(56.0, fw), &mut o.suffix);
        let theme = crate::theme::Theme::current(ui.ctx());
        ui.painter().text(g.field(86.0, fw).left_center(), egui::Align2::LEFT_CENTER, &example, theme.ui_font(), theme.colors.text_weak);
        super::fit_label(ui, g.label(116.0), &pad_l, true);
        if let Some(v) = fields::number(ui, "sprite-padding", g.field(116.0, 47.0), None, Some(o.padding as f64), (0.0, 999.0), 0, true) {
            o.padding = v.round() as i32;
        }
        ui.painter().text(g.field(116.0, 47.0).right_center() + vec2(6.0, 0.0), egui::Align2::LEFT_CENTER, t("unit-px"), theme.ui_font(), theme.colors.text);
        super::fit_label(ui, g.label(144.0), &lay_l, true);
        if let Some(i) = fields::combo(ui, "sprite-layout", g.field(144.0, fw), &refs(&lay_items), Some(o.layout), true) {
            o.layout = i;
        }
        super::fit_label(ui, g.label(172.0), &preset_l, true);
        if let Some(i) = fields::combo(ui, "sprite-preset", g.field(172.0, fw), &refs(&presets), Some(o.preset), true) {
            o.preset = i;
        }
        ans = ok_cancel(ui, body);
    });
    closed(c, ans)
}

/// Listenplatz (mit Trennstrichen) ⇔ Wert für Präfix/Suffix: Keine, ——, Dokumentname, ——, #, #0.
const AFFIX_ROWS: [Option<usize>; 6] = [Some(0), None, Some(1), None, Some(2), Some(3)];

fn show_files(ctx: &egui::Context, o: &mut FilesOpts, kind_name: &str) -> Option<bool> {
    let l = |k: &str| t(k);
    let (name_l, pre_l, suf_l, type_l) = (l("dlg-expopt-files-name"), l("dlg-expopt-add-prefix"), l("dlg-expopt-add-suffix"), l("dlg-expopt-file-type"));
    let affix_items = vec![l("dlg-expopt-affix-none"), SEP.into(), l("dlg-expopt-affix-doc"), SEP.into(), l("dlg-expopt-affix-num"), l("dlg-expopt-affix-num2")];
    let mut type_items = vec![l("dlg-expopt-doc-settings"), SEP.into()];
    type_items.extend(preset_items());
    let lw = super::max_text_w(ctx, &[&name_l, &type_l]).max(60.0).ceil();
    let cw = (22.0 + super::max_text_w(ctx, &[&pre_l, &suf_l])).ceil() + 6.0;
    let aw = super::combo_fit_w(ctx, &refs(&affix_items), 140.0);
    let tw = super::combo_fit_w(ctx, &refs(&type_items), 243.0).max(cw + aw);
    let width = (32.0 + lw + 8.0 + tw).max(339.0).ceil();
    let mut ans = None;
    let title = fp_i18n::tr!("dlg-expopt-files-title", kind = kind_name);
    let c = dialog_frame(ctx, "fp-export-files-options", &title, vec2(width, 150.0), |ui, body| {
        let g = Grid { o: body.min + vec2(16.0, 14.0), lw };
        super::fit_label(ui, g.label(0.0), &name_l, true);
        let fx = g.field(0.0, 0.0).min.x;
        for (y, id, text, on, v) in [(0.0, "files-prefix", &pre_l, &mut o.add_prefix, &mut o.prefix), (28.0, "files-suffix", &suf_l, &mut o.add_suffix, &mut o.suffix)] {
            if let Some(b) = fields::checkbox(ui, (id, 0), Rect::from_min_size(egui::pos2(fx, g.o.y + y + 2.0), vec2(cw, 18.0)), text, *on, true) {
                *on = b;
            }
            let sel = AFFIX_ROWS.iter().position(|r| *r == Some(*v));
            if let Some(i) = fields::combo(ui, (id, 1), Rect::from_min_size(egui::pos2(fx + cw, g.o.y + y), vec2(aw, 22.0)), &refs(&affix_items), sel, *on)
                && let Some(Some(x)) = AFFIX_ROWS.get(i)
            {
                *v = *x;
            }
        }
        super::fit_label(ui, g.label(56.0), &type_l, true);
        let sel = if o.file_type == 0 { 0 } else { o.file_type + 1 };
        if let Some(i) = fields::combo(ui, "files-type", g.field(56.0, tw), &refs(&type_items), Some(sel), true)
            && i != 1
        {
            o.file_type = if i == 0 { 0 } else { i - 1 };
        }
        ans = ok_cancel(ui, body);
    });
    closed(c, ans)
}

fn show_psd(ctx: &egui::Context, o: &mut PsdOpts) -> Option<bool> {
    let l = |k: &str| t(k);
    let labels = [l("dlg-expopt-psd-settings"), l("dlg-expopt-psd-objects"), l("dlg-expopt-psd-effects"), l("dlg-expopt-psd-vector"), l("dlg-expopt-psd-text")];
    let keep = l("dlg-expopt-psd-keep-editable");
    let lists: [Vec<String>; 5] = [
        vec![l("dlg-expopt-psd-set-editable"), l("dlg-expopt-psd-set-appearance"), l("dlg-expopt-psd-set-smaller"), l("dlg-expopt-psd-set-custom")],
        vec![l("dlg-expopt-psd-obj-layers"), l("dlg-expopt-psd-obj-flatten")],
        vec![keep.clone(), l("dlg-expopt-psd-eff-render")],
        vec![keep.clone(), l("dlg-expopt-psd-vec-appearance")],
        vec![keep, l("dlg-expopt-psd-text-render")],
    ];
    let lw = super::max_text_w(ctx, &refs(&labels)).max(80.0).ceil();
    let fw = lists.iter().map(|v| super::combo_fit_w(ctx, &refs(v), 272.0)).fold(272f32, f32::max);
    let width = (32.0 + lw + 8.0 + fw).max(402.0).ceil();
    let mut ans = None;
    let c = dialog_frame(ctx, "fp-export-psd-options", &t("dlg-expopt-psd-title"), vec2(width, 186.0), |ui, body| {
        let g = Grid { o: body.min + vec2(16.0, 14.0), lw };
        let custom = o.settings == 3;
        let vals = [o.settings, o.objects, o.effects, o.vector, o.text];
        for (n, (label, items)) in labels.iter().zip(lists.iter()).enumerate() {
            let y = n as f32 * 27.0;
            let on = n == 0 || custom;
            super::fit_label(ui, g.label(y), label, on);
            if let Some(i) = fields::combo(ui, ("psd", n), g.field(y, fw), &refs(items), Some(vals[n]), on) {
                match n {
                    0 => o.set_settings(i),
                    1 => o.objects = i,
                    2 => o.effects = i,
                    3 => o.vector = i,
                    _ => o.text = i,
                }
            }
        }
        ans = ok_cancel(ui, body);
    });
    closed(c, ans)
}
