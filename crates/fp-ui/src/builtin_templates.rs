//! Mitgelieferte Vorlagen für *Datei ▸ Neu von Vorlage* (Q-11, Nutzerentscheidung 14.10.2026).
//!
//! Firepixel bringt eigene, selbst gestaltete Vorlagen mit (keine Inhalte anderer Programme): je
//! Unterordner zwei bis drei einfache FW-PNG-Dokumente – Dokumentvorgaben, Raster, Mobil, Web,
//! Drahtmodelle. Sie werden beim ersten Öffnen des Vorlagenordners (`%APPDATA%\Firepixel\Templates`)
//! aus dem Programm heraus erzeugt, Ordner-, Datei- und Beschriftungsnamen in der Sprache der
//! Oberfläche in diesem Moment. Danach gehören sie dem Nutzer: gelöschte oder geänderte Vorlagen
//! kommen nicht wieder (Merker `builtin-templates.json`, technisch englisch).

use fp_color::Argb;
use fp_geom::{Point, Rect};
use fp_model::attrs::{Brush, PathAttrs};
use fp_model::ids::IdGen;
use fp_model::layer::LayerSlot;
use fp_model::{Document, Element};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Merker im Vorlagenordner: Version der mitgelieferten Vorlagen und die angelegten Dateien.
pub const MARKER: &str = "builtin-templates.json";
/// Version des Vorlagensatzes (steigt, wenn neue Vorlagen dazukommen; nur neue werden ergänzt).
pub const VERSION: u32 = 1;

/// Eine Vorlage: technische ID (englisch), Ordner-Schlüssel, Name-Schlüssel, Maße, Zeichnung.
pub struct Builtin {
    pub id: &'static str,
    pub folder: &'static str,
    pub width: f32,
    pub height: f32,
    draw: fn(&mut Canvas),
}

/// Alle mitgelieferten Vorlagen (Reihenfolge = Ordner, dann Datei).
pub fn all() -> Vec<Builtin> {
    vec![
        Builtin { id: "a4-portrait", folder: "presets", width: 595.0, height: 842.0, draw: draw_a4 },
        Builtin { id: "presentation-1920", folder: "presets", width: 1920.0, height: 1080.0, draw: draw_presentation },
        Builtin { id: "grid-12-columns", folder: "grids", width: 1200.0, height: 800.0, draw: |c| draw_columns(c, 12, 20.0, 40.0) },
        Builtin { id: "grid-6-columns-tablet", folder: "grids", width: 768.0, height: 1024.0, draw: |c| draw_columns(c, 6, 16.0, 32.0) },
        Builtin { id: "phone-390", folder: "mobile", width: 390.0, height: 844.0, draw: draw_phone },
        Builtin { id: "app-icon-1024", folder: "mobile", width: 1024.0, height: 1024.0, draw: draw_app_icon },
        Builtin { id: "web-banner-728", folder: "web", width: 728.0, height: 90.0, draw: draw_banner },
        Builtin { id: "social-post-1080", folder: "web", width: 1080.0, height: 1080.0, draw: draw_social },
        Builtin { id: "website-home-1440", folder: "wireframes", width: 1440.0, height: 1024.0, draw: draw_website_wireframe },
        Builtin { id: "mobile-wireframe-390", folder: "wireframes", width: 390.0, height: 844.0, draw: draw_phone_wireframe },
    ]
}

impl Builtin {
    /// Ordnername in der aktuellen Sprache.
    pub fn folder_name(&self) -> String {
        fp_i18n::t(&format!("tpl-folder-{}", self.folder))
    }
    /// Dateiname in der aktuellen Sprache (mit Maßen, Endung `.fw.png`).
    pub fn file_name(&self) -> String {
        let name = fp_i18n::t(&format!("tpl-{}", self.id));
        format!("{} {}x{}.fw.png", safe(&name), self.width as u32, self.height as u32)
    }
    /// Das Dokument.
    pub fn document(&self) -> Document {
        let mut doc = Document::new(self.width, self.height, Argb(0xffff_ffff));
        let mut c = Canvas { ids: std::mem::take(&mut doc.ids), els: Vec::new(), w: self.width, h: self.height };
        (self.draw)(&mut c);
        doc.ids = c.ids;
        if let Some(LayerSlot::Own(l)) = doc.page_mut(0).map(|p| &mut p.root.layers[0]) {
            let cel = Arc::make_mut(&mut Arc::make_mut(l).cels[0]);
            // in der Ebene liegt das erste Element oben: zuerst Gezeichnetes (Hintergrund) nach unten
            cel.elements.extend(c.els.into_iter().rev().map(Arc::new));
        }
        doc
    }
}

/// Zeichen, die in Dateinamen nicht gehen, ersetzen.
fn safe(s: &str) -> String {
    s.chars().map(|c| if "<>:\"/\\|?*".contains(c) || c.is_control() { '-' } else { c }).collect::<String>().trim().to_string()
}

/// Zeichenfläche beim Erzeugen einer Vorlage.
pub struct Canvas {
    ids: IdGen,
    els: Vec<Element>,
    w: f32,
    h: f32,
}

const INK: Argb = Argb(0xff33_3a44);
const MUTED: Argb = Argb(0xff8a_93a0);
const LINE: Argb = Argb(0xffc8_ced6);
const BOX: Argb = Argb(0xffe9_edf2);
const ACCENT: Argb = Argb(0xff2f_7de1);
const WHITE: Argb = Argb(0xffff_ffff);

impl Canvas {
    fn rect(&mut self, l: f32, t: f32, w: f32, h: f32, fill: Option<Argb>, stroke: Option<Argb>, round: f32) {
        let mut a = match fill {
            Some(c) => PathAttrs::solid_fill(c),
            None => PathAttrs::default(),
        };
        if fill.is_none() {
            a.fill = None;
        }
        if let Some(s) = stroke {
            a.stroke_color = s;
            a.brush = Some(Brush::basic(1));
        }
        self.els.push(Element::new_rectangle(&mut self.ids, Rect::new(l, t, l + w, t + h), round, a));
    }
    fn text(&mut self, key: &str, x: f32, baseline: f32, size: f32, color: Argb) {
        let s = fp_i18n::t(key);
        self.els.push(Element::new_text(&mut self.ids, &s, "Arial", size, color, Point::new(x, baseline)));
    }
    /// Platzhalter-Kasten mit Kreuz und Beschriftung (Drahtmodell).
    fn placeholder(&mut self, l: f32, t: f32, w: f32, h: f32, key: &str) {
        self.rect(l, t, w, h, Some(BOX), Some(LINE), 0.0);
        let size = (h * 0.18).clamp(11.0, 22.0);
        self.text(key, l + 12.0, t + 12.0 + size, size, MUTED);
    }
}

fn draw_a4(c: &mut Canvas) {
    // Satzspiegel (Ränder 56 pt), Kopf und Fuß
    let m = 56.0;
    c.rect(m, m, c.w - 2.0 * m, c.h - 2.0 * m, None, Some(LINE), 0.0);
    c.text("tpl-text-headline", m, m + 40.0, 28.0, INK);
    c.text("tpl-text-body", m, m + 72.0, 12.0, MUTED);
    c.text("tpl-text-footer", m, c.h - m + 24.0, 9.0, MUTED);
}

fn draw_presentation(c: &mut Canvas) {
    c.rect(0.0, 0.0, c.w, 12.0, Some(ACCENT), None, 0.0);
    c.text("tpl-text-headline", 120.0, 420.0, 96.0, INK);
    c.text("tpl-text-subline", 124.0, 500.0, 40.0, MUTED);
    c.rect(120.0, 980.0, c.w - 240.0, 1.0, Some(LINE), None, 0.0);
}

fn draw_columns(c: &mut Canvas, n: usize, gutter: f32, margin: f32) {
    let col = (c.w - 2.0 * margin - gutter * (n as f32 - 1.0)) / n as f32;
    for k in 0..n {
        let x = margin + k as f32 * (col + gutter);
        c.rect(x, 0.0, col, c.h, Some(Argb(0x33ff_4d6a)), None, 0.0);
    }
}

fn draw_phone(c: &mut Canvas) {
    // Statusleiste, Navigationsleiste, Inhaltsbereich, Tab-Leiste (sichere Bereiche)
    c.rect(0.0, 0.0, c.w, 47.0, Some(BOX), None, 0.0);
    c.rect(0.0, 47.0, c.w, 44.0, Some(WHITE), Some(LINE), 0.0);
    c.text("tpl-text-title", 16.0, 76.0, 17.0, INK);
    c.rect(0.0, c.h - 83.0, c.w, 83.0, Some(WHITE), Some(LINE), 0.0);
    for k in 0..4 {
        let x = 30.0 + k as f32 * 95.0;
        c.rect(x + 8.0, c.h - 72.0, 24.0, 24.0, Some(LINE), None, 0.3);
    }
    c.rect(c.w / 2.0 - 67.0, c.h - 13.0, 134.0, 5.0, Some(INK), None, 1.0);
}

fn draw_app_icon(c: &mut Canvas) {
    // Fläche mit abgerundeten Ecken, innere sichere Zone (≈ 80 %)
    c.rect(0.0, 0.0, c.w, c.h, Some(ACCENT), None, 0.45);
    let s = c.w * 0.8;
    c.rect((c.w - s) / 2.0, (c.h - s) / 2.0, s, s, None, Some(Argb(0x99ff_ffff)), 0.45);
    c.text("tpl-text-icon", c.w * 0.18, c.h * 0.56, 120.0, WHITE);
}

fn draw_banner(c: &mut Canvas) {
    c.rect(0.0, 0.0, c.w, c.h, Some(Argb(0xff1e_2a3a)), None, 0.0);
    c.text("tpl-text-message", 24.0, 54.0, 26.0, WHITE);
    c.rect(c.w - 168.0, 25.0, 144.0, 40.0, Some(ACCENT), None, 0.25);
    c.text("tpl-text-button", c.w - 150.0, 51.0, 16.0, WHITE);
}

fn draw_social(c: &mut Canvas) {
    c.rect(0.0, 0.0, c.w, c.h, Some(Argb(0xfff4_f1ea)), None, 0.0);
    c.rect(80.0, 80.0, c.w - 160.0, 620.0, Some(BOX), Some(LINE), 0.0);
    c.text("tpl-text-image", 110.0, 140.0, 28.0, MUTED);
    c.text("tpl-text-headline", 80.0, 800.0, 72.0, INK);
    c.text("tpl-text-subline", 84.0, 870.0, 32.0, MUTED);
    c.text("tpl-text-handle", 84.0, 1000.0, 26.0, ACCENT);
}

fn draw_website_wireframe(c: &mut Canvas) {
    let m = 80.0;
    c.placeholder(0.0, 0.0, c.w, 80.0, "tpl-text-header");
    c.placeholder(m, 120.0, c.w - 2.0 * m, 420.0, "tpl-text-hero");
    let gap = 32.0;
    let w = (c.w - 2.0 * m - 2.0 * gap) / 3.0;
    for k in 0..3 {
        c.placeholder(m + k as f32 * (w + gap), 580.0, w, 260.0, "tpl-text-card");
    }
    c.placeholder(0.0, c.h - 120.0, c.w, 120.0, "tpl-text-footer");
}

fn draw_phone_wireframe(c: &mut Canvas) {
    c.placeholder(0.0, 0.0, c.w, 91.0, "tpl-text-header");
    c.placeholder(16.0, 107.0, c.w - 32.0, 200.0, "tpl-text-image");
    for k in 0..4 {
        c.placeholder(16.0, 323.0 + k as f32 * 92.0, c.w - 32.0, 76.0, "tpl-text-list-item");
    }
    c.placeholder(0.0, c.h - 83.0, c.w, 83.0, "tpl-text-tabbar");
}

/// Legt die mitgelieferten Vorlagen im Ordner `dir` an, falls dort noch kein Merker der aktuellen
/// Version liegt. Vorhandene Dateien bleiben unberührt. Liefert die neu geschriebenen Pfade.
pub fn ensure(dir: &Path, save: &dyn Fn(&Document, &Path) -> Result<(), String>) -> Vec<PathBuf> {
    let marker = dir.join(MARKER);
    let old: Option<serde_json::Value> = std::fs::read_to_string(&marker).ok().and_then(|s| serde_json::from_str(&s).ok());
    let have = old.as_ref().and_then(|v| v.get("version")).and_then(serde_json::Value::as_u64).unwrap_or(0);
    let known: Vec<String> = old.as_ref().and_then(|v| v.get("ids")).and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default();
    if have >= VERSION as u64 {
        return Vec::new();
    }
    fp_i18n::profile::guard_write(dir);
    let mut written = Vec::new();
    let mut ids = known.clone();
    for b in all() {
        if known.iter().any(|k| k == b.id) {
            continue; // in einer früheren Version angelegt (vielleicht vom Nutzer gelöscht): nicht wieder
        }
        let folder = dir.join(safe(&b.folder_name()));
        let path = folder.join(b.file_name());
        ids.push(b.id.to_string());
        if path.exists() || std::fs::create_dir_all(&folder).is_err() {
            continue;
        }
        if save(&b.document(), &path).is_ok() {
            written.push(path);
        }
    }
    let v = serde_json::json!({"format": "firepixel-builtin-templates", "version": VERSION, "ids": ids});
    let _ = std::fs::write(&marker, serde_json::to_string_pretty(&v).unwrap_or_default());
    written
}
