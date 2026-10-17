//! „Vektordatei-Optionen“ (TW-43, Spezifikation `re\12-import.md` §4.2): erscheint beim Importieren
//! einer Vektordatei (AI, EPS, PDF) vor dem Platzieren. Felder, Vorgaben und Anordnung wie im
//! Original (Lauf 4 `xs-file-formats/10`): Skalieren 100 mit Schieber, Breite/Höhe der Datei in
//! Pixel, Auflösung 72 Pixel/Zoll, Anti-Alias Pfade und Text mit „Glatt“ in einer Zeile; Gruppe
//! „Dateikonvertierung“ mit Seitenwahl („Seite öffnen“ / „Seiten als Status öffnen“, Zeichenfläche –
//! nur bei mehr als einer Seite aktiv, TW-60), Ebenen ignorieren/berücksichtigen/in Status
//! konvertieren, unsichtbare Ebenen; Gruppe „Als Bilder wiedergeben“ (Gruppierungen über N Objekte,
//! Mischungen grau wie im Original, Flächenfüllungen). Alles außer „Hintergrundebenen“ (immer an)
//! und „Flächenfüllungen“ (Muster liest der Import nicht) wirkt über `file.import` Parameter `vector`.

use super::form::{CustomLayout, Field, FieldKind, FormDialog, number_input_aligned};
use super::{t, text_w};
use crate::app_state::AppState;
use crate::inspector::fields;
use crate::theme::Theme;
use egui::{Align2, Rect, Ui, pos2, vec2};
use serde_json::{Value, json};
use std::path::Path;

/// Ist die Datei eine Vektordatei, für die das Original „Vektordatei-Optionen“ zeigt?
pub fn is_vector_file(path: &Path) -> Option<(Vec<u8>, fp_io::FileFormat)> {
    let data = std::fs::read(path).ok()?;
    let fmt = fp_io::detect_format(&data, &path.to_string_lossy());
    matches!(
        fmt,
        fp_io::FileFormat::Illustrator
            | fp_io::FileFormat::Pdf
            | fp_io::FileFormat::Eps
            | fp_io::FileFormat::EpsBinary
    )
    .then_some((data, fmt))
}

/// Dialog zur Datei `path` öffnen; `false`, wenn es keine Vektordatei ist.
pub fn open(st: &mut AppState, path: &Path) -> bool {
    let Some((data, _)) = is_vector_file(path) else {
        return false;
    };
    // Größe der Grafik in Pixel bei 72 Pixel/Zoll (Seitenrechteck der Datei) und Seitenzahl
    let doc = fp_io::import_bytes(
        &path.to_string_lossy(),
        &data,
        &fp_io::ImportOptions::default(),
    )
    .ok()
    .map(|r| r.document);
    let (w, h) = doc
        .as_ref()
        .and_then(|d| {
            d.pages.first().map(|p| {
                (
                    p.canvas.width.round() as f64,
                    p.canvas.height.round() as f64,
                )
            })
        })
        .unwrap_or((100.0, 100.0));
    let pages = doc.as_ref().map(|d| d.pages.len()).unwrap_or(1).max(1);
    st.dialogs.form = Some(form_pages(path, w.max(1.0), h.max(1.0), pages));
    true
}

/// Formular mit den Vorgaben des Originals (eine Seite).
pub fn form(path: &Path, w: f64, h: f64) -> FormDialog {
    form_pages(path, w, h, 1)
}

/// Formular mit den Vorgaben des Originals für eine Datei mit `pages` Seiten bzw. Zeichenflächen.
pub fn form_pages(path: &Path, w: f64, h: f64, pages: usize) -> FormDialog {
    let pages = pages.max(1);
    let units = [
        t("dlg-vec-unit-px"),
        t("dlg-vec-unit-in"),
        t("dlg-vec-unit-cm"),
    ];
    let units: Vec<&str> = units.iter().map(|s| s.as_str()).collect();
    let res_units = [t("dlg-vec-res-ppi"), t("dlg-vec-res-ppcm")];
    let res_units: Vec<&str> = res_units.iter().map(|s| s.as_str()).collect();
    let aa = [
        t("dlg-vec-aa-system"),
        t("dlg-vec-aa-crisp"),
        t("dlg-vec-aa-strong"),
        t("dlg-vec-aa-smooth"),
    ];
    let aa: Vec<&str> = aa.iter().map(|s| s.as_str()).collect();
    let conv = [t("dlg-vec-open-page"), t("dlg-vec-open-pages-states")];
    let conv: Vec<&str> = conv.iter().map(|s| s.as_str()).collect();
    let layers = [
        t("dlg-vec-layers-ignore"),
        t("dlg-vec-layers-keep"),
        t("dlg-vec-layers-states"),
    ];
    let layers: Vec<&str> = layers.iter().map(|s| s.as_str()).collect();
    // Zeichenflächen „1“…„n“ (`%ld`, re\12 §4.2)
    let boards: Vec<String> = (1..=pages).map(|i| i.to_string()).collect();
    let boards: Vec<&str> = boards.iter().map(|s| s.as_str()).collect();
    let single = pages == 1;
    let lock = |f: Field| if single { f.disabled() } else { f };
    let fields = vec![
        Field::number("scale", &t("dlg-vec-scale"), 100.0, 1.0, 10000.0).popup(),
        Field::number("width", &t("dlg-vec-width"), w, 0.0, 100000.0).decimals(2),
        Field::choice("widthUnit", "", &units, 0),
        Field::number("height", &t("dlg-vec-height"), h, 0.0, 100000.0).decimals(2),
        Field::choice("heightUnit", "", &units, 0),
        Field::number("resolution", &t("dlg-vec-resolution"), 72.0, 10.0, 1200.0).decimals(2),
        Field::choice("resolutionUnit", "", &res_units, 0),
        Field::check("antialiasPaths", &t("dlg-vec-aa-paths"), true),
        Field::check("antialiasText", &t("dlg-vec-aa-text"), true),
        Field::choice("antialiasKind", &t("dlg-vec-aa"), &aa, 3),
        Field::caption(&t("dlg-vec-conversion")),
        // Seitenwahl nur bei mehr als einer Seite (TW-60: bei einer Seite grau wie im Original)
        lock(Field::choice("convert", "", &conv, 0)),
        lock(Field::choice(
            "artboard",
            &t("dlg-vec-artboard"),
            &boards,
            0,
        )),
        Field::choice("layers", "", &layers, 1),
        Field::check("includeInvisible", &t("dlg-vec-include-invisible"), false),
        Field::check("_background", &t("dlg-vec-include-background"), true).disabled(),
        Field::caption(&t("dlg-vec-render-as-images")),
        Field::check("groups", &t("dlg-vec-groups-over"), false),
        Field::number("groupsN", &t("dlg-vec-objects"), 1.0, 1.0, 1000.0),
        Field::check("_blends", &t("dlg-vec-blends-over"), false).disabled(),
        Field::number("_blendsN", &t("dlg-vec-steps"), 1.0, 1.0, 1000.0).disabled(),
        Field::check("tiled", &t("dlg-vec-tiled-over"), false),
        Field::number("tiledN", &t("dlg-vec-objects"), 1.0, 1.0, 1000.0),
    ];
    FormDialog::new("vector-options", &t("dlg-vec-title"), "file.import", fields)
        .with_base(json!({ "path": path.to_string_lossy(), "_w0": w, "_h0": h, "_pages": pages }))
        .with_layout(LAYOUT)
        .with_transform(params)
}

/// Pixel einer Längenangabe (0 Pixel, 1 Zoll, 2 Zentimeter) bei `ppi`.
fn to_px(v: f64, unit: i64, ppi: f64) -> f64 {
    match unit {
        1 => v * ppi,
        2 => v * ppi / 2.54,
        _ => v,
    }
}

/// Parameter für `file.import`: Pfad und `vector`. Geänderte Breite bzw. Höhe bestimmen die
/// Skalierung (wie die Kopplung im Original), sonst gilt „Skalieren“.
pub fn params(d: &FormDialog) -> Value {
    let n = |k: &str| d.get(k).as_f64().unwrap_or(0.0);
    let i = |k: &str| d.get(k).as_i64().unwrap_or(0);
    let b = |k: &str| d.get(k).as_bool().unwrap_or(false);
    let (w0, h0) = (
        d.base["_w0"].as_f64().unwrap_or(1.0).max(1.0),
        d.base["_h0"].as_f64().unwrap_or(1.0).max(1.0),
    );
    let pages = d.base["_pages"].as_u64().unwrap_or(1).max(1);
    let res = n("resolution").clamp(10.0, 1200.0);
    let ppi = if i("resolutionUnit") == 1 {
        res * 2.54
    } else {
        res
    };
    let mut scale = n("scale").clamp(1.0, 10000.0);
    // Breite/Höhe in Pixel bei der gewählten Auflösung; die Ausgangswerte gelten bei 72 Pixel/Zoll
    let base_px = |v0: f64| v0 * scale / 100.0 * ppi / 72.0;
    let w = to_px(n("width"), i("widthUnit"), ppi);
    let h = to_px(n("height"), i("heightUnit"), ppi);
    if (w - w0).abs() > 0.5 && (w - base_px(w0)).abs() > 0.5 {
        scale = w / (w0 * ppi / 72.0) * 100.0;
    } else if (h - h0).abs() > 0.5 && (h - base_px(h0)).abs() > 0.5 {
        scale = h / (h0 * ppi / 72.0) * 100.0;
    }
    let as_states = pages > 1 && i("convert") == 1;
    // „Seiten als Status öffnen“ setzt „Ebenen in Status konvertieren“ auf „berücksichtigen“ (re\12 §4.2)
    let layers = if as_states && i("layers") == 2 {
        1
    } else {
        i("layers")
    };
    let mut v = json!({
        "scale": scale,
        "resolution": res,
        "resolutionUnit": i("resolutionUnit"),
        "antialiasPaths": b("antialiasPaths"),
        "antialiasText": b("antialiasText"),
        "antialiasKind": i("antialiasKind"),
        "layers": layers,
        "includeInvisible": b("includeInvisible"),
        "page": if pages > 1 && !as_states { i("artboard").clamp(0, pages as i64 - 1) } else { 0 },
        "pagesAsStates": as_states,
    });
    if b("groups") {
        v["groupsOver"] = json!(n("groupsN").round().max(1.0) as u64);
    }
    json!({ "path": d.base["path"], "vector": v })
}

// ------------------------------------------------- Anordnung (Ressource 5986)
//
// Pixel im Client-Bereich, gemessen am Bild des Originals (Lauf 4 `xs-file-formats/10`, 432 × 408):
// Beschriftungen rechtsbündig bis 112, Felder ab 117 (Skalieren 43 breit mit ▾, Breite/Höhe/
// Auflösung 46 breit mit Einheitenliste bis zum rechten Rand); Zeile 100: Pfade [117], Text [192],
// Art [270 …]. Gruppe „Dateikonvertierung“ [8,118 … 250]: Umwandlung [17,148,238], „Zeichenfläche:“
// rechtsbündig vor der Liste [rechts − 56, 148]; Ebenen [17,175,238]; Kästchen [17,203] und [17,223].
// Gruppe „Als Bilder wiedergeben“ [8,253 … 363]: Kästchen [17,278/303/328], Zahl [182,·,33] und
// Einheit dahinter. OK und Abbrechen [·,378,72,22] rechts unten. Längere Beschriftungen schieben nach
// rechts und verbreitern den Dialog.

const LABEL_KEYS: [&str; 5] = ["scale", "width", "height", "resolution", "antialiasKind"];

fn field_at(d: &FormDialog, key: &str) -> usize {
    d.fields
        .iter()
        .position(|f| f.key == key)
        .unwrap_or(usize::MAX)
}

fn label_of(d: &FormDialog, key: &str) -> String {
    d.fields
        .get(field_at(d, key))
        .map(|f| f.label.replace('&', ""))
        .unwrap_or_default()
}

fn items_of(d: &FormDialog, key: &str) -> Vec<String> {
    match d.fields.get(field_at(d, key)).map(|f| &f.kind) {
        Some(FieldKind::Choice(v)) => v.clone(),
        _ => Vec::new(),
    }
}

/// Breite eines Kästchens mit Text.
fn check_w(ctx: &egui::Context, d: &FormDialog, key: &str) -> f32 {
    18.0 + text_w(ctx, &label_of(d, key)) + 8.0
}

/// Verschiebung der Feldspalte, wenn eine Beschriftung nicht vor 112 passt.
fn shift(ctx: &egui::Context, d: &FormDialog) -> f32 {
    let w = LABEL_KEYS
        .iter()
        .map(|k| text_w(ctx, &label_of(d, k)))
        .fold(0.0, f32::max);
    (w + 8.0 - 112.0).max(0.0).ceil()
}

/// Verschiebung der Zahlenfelder in „Als Bilder wiedergeben“ hinter längeren Kästchentexten.
fn shift_n(ctx: &egui::Context, d: &FormDialog) -> f32 {
    let w = ["groups", "_blends", "tiled"]
        .iter()
        .map(|k| check_w(ctx, d, k))
        .fold(0.0, f32::max);
    (17.0 + w - 182.0).max(0.0).ceil()
}

fn size(ctx: &egui::Context, d: &FormDialog) -> egui::Vec2 {
    let dx = shift(ctx, d);
    let dn = shift_n(ctx, d);
    let unit_w = ["groupsN", "_blendsN", "tiledN"]
        .iter()
        .map(|k| text_w(ctx, &label_of(d, k)))
        .fold(0.0, f32::max);
    vec2((432.0 + dx).max(220.0 + dn + unit_w + 30.0).ceil(), 408.0)
}

fn rr(o: egui::Pos2, x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_min_size(o + vec2(x, y), vec2(w, h))
}

fn label(ui: &Ui, x: f32, row: Rect, text: &str, enabled: bool, align: Align2) {
    let theme = Theme::current(ui.ctx());
    let c = if enabled {
        theme.colors.text
    } else {
        theme.colors.text_disabled
    };
    ui.painter()
        .text(pos2(x, row.center().y), align, text, theme.ui_font(), c);
}

fn number_at(ui: &mut Ui, d: &mut FormDialog, key: &str, r: Rect) -> bool {
    let i = field_at(d, key);
    let id = d.id.clone();
    d.fields.get_mut(i).is_some_and(|f| {
        let en = !f.disabled;
        // numbers right-aligned as in the original (T-1 report, run 4 `xs-file-formats/orig/10`)
        number_input_aligned(ui, &id, i, f, r, en, true)
    })
}

fn check_at(ui: &mut Ui, d: &mut FormDialog, key: &str, r: Rect) -> bool {
    let i = field_at(d, key);
    let id = d.id.clone();
    let Some(f) = d.fields.get_mut(i) else {
        return false;
    };
    if let Some(b) = fields::checkbox(
        ui,
        (id.as_str(), i),
        r,
        &f.label,
        f.value.as_bool().unwrap_or(false),
        !f.disabled,
    ) {
        f.value = json!(b);
        return true;
    }
    false
}

fn choice_at(ui: &mut Ui, d: &mut FormDialog, key: &str, r: Rect, enabled: bool) -> bool {
    let i = field_at(d, key);
    let items = items_of(d, key);
    let id = d.id.clone();
    let Some(f) = d.fields.get_mut(i) else {
        return false;
    };
    let refs: Vec<&str> = items.iter().map(|s| s.as_str()).collect();
    let sel = f.value.as_u64().map(|v| v as usize);
    if let Some(k) = fields::combo(
        ui,
        (id.as_str(), i, "combo"),
        r,
        &refs,
        sel,
        enabled && !f.disabled,
    ) {
        f.value = json!(k);
        return true;
    }
    false
}

fn draw(ui: &mut Ui, _st: &AppState, d: &mut FormDialog, body: Rect) -> (bool, [Rect; 2]) {
    let o = body.min;
    let ctx = ui.ctx().clone();
    let dx = shift(&ctx, d);
    let dn = shift_n(&ctx, d);
    let right = body.right() - 10.0;
    let lr = o.x + 112.0 + dx;
    let fx = 117.0 + dx;
    let mut changed = false;
    // Skalieren mit Popup-Schieber
    let r = rr(o, fx, 3.0, 43.0, 20.0);
    label(ui, lr, r, &label_of(d, "scale"), true, Align2::RIGHT_CENTER);
    changed |= number_at(ui, d, "scale", r);
    // Breite, Höhe, Auflösung mit Einheit
    for (key, unit, y) in [
        ("width", "widthUnit", 26.0),
        ("height", "heightUnit", 49.0),
        ("resolution", "resolutionUnit", 76.0),
    ] {
        let r = rr(o, fx, y, 46.0, 20.0);
        label(ui, lr, r, &label_of(d, key), true, Align2::RIGHT_CENTER);
        changed |= number_at(ui, d, key, r);
        let ur = Rect::from_min_max(pos2(r.right() + 4.0, r.top()), pos2(right, r.bottom()));
        changed |= choice_at(ui, d, unit, ur, true);
    }
    // Anti-Alias: Pfade, Text, Art (Art nur mit „Text“ aktiv, re\12 §4.2)
    let row = rr(o, fx, 100.0, 70.0, 20.0);
    label(
        ui,
        lr,
        row,
        &label_of(d, "antialiasKind"),
        true,
        Align2::RIGHT_CENTER,
    );
    let pw = check_w(&ctx, d, "antialiasPaths");
    changed |= check_at(
        ui,
        d,
        "antialiasPaths",
        Rect::from_min_size(row.min, vec2(pw, 20.0)),
    );
    let tx = (row.left() + pw + 10.0).max(o.x + 192.0 + dx);
    let tw = check_w(&ctx, d, "antialiasText");
    changed |= check_at(
        ui,
        d,
        "antialiasText",
        Rect::from_min_size(pos2(tx, row.top()), vec2(tw, 20.0)),
    );
    let text_on = d.get("antialiasText").as_bool().unwrap_or(false);
    let kx = (tx + tw + 10.0).max(o.x + 270.0 + dx);
    changed |= choice_at(
        ui,
        d,
        "antialiasKind",
        Rect::from_min_max(pos2(kx, row.top()), pos2(right, row.bottom())),
        text_on,
    );
    // Dateikonvertierung
    let g1 = Rect::from_min_max(o + vec2(8.0, 118.0), pos2(right + 2.0, o.y + 250.0));
    super::group_box(ui, g1, &t("dlg-vec-conversion"));
    let multi = d.base["_pages"].as_u64().unwrap_or(1) > 1;
    let cw = (g1.width() - 176.0).max(238.0);
    let conv = rr(o, 17.0, 148.0, cw, 20.0);
    changed |= choice_at(ui, d, "convert", conv, multi);
    let as_states = multi && d.get("convert").as_u64() == Some(1);
    let board_on = multi && !as_states;
    let br = Rect::from_min_max(pos2(right - 56.0, conv.top()), pos2(right, conv.bottom()));
    label(
        ui,
        br.left() - 5.0,
        br,
        &label_of(d, "artboard"),
        board_on,
        Align2::RIGHT_CENTER,
    );
    changed |= choice_at(ui, d, "artboard", br, board_on);
    changed |= choice_at(ui, d, "layers", rr(o, 17.0, 175.0, cw, 20.0), true);
    // „Seiten als Status öffnen“ setzt „Ebenen in Status konvertieren“ auf „berücksichtigen“ (`@00fb3240`)
    if as_states
        && d.get("layers").as_u64() == Some(2)
        && let Some(f) = d.field_mut("layers")
    {
        f.value = json!(1);
        changed = true;
    }
    let w = check_w(&ctx, d, "includeInvisible");
    changed |= check_at(ui, d, "includeInvisible", rr(o, 17.0, 203.0, w, 18.0));
    let w = check_w(&ctx, d, "_background");
    changed |= check_at(ui, d, "_background", rr(o, 17.0, 223.0, w, 18.0));
    // Als Bilder wiedergeben
    let g2 = Rect::from_min_max(o + vec2(8.0, 253.0), pos2(right + 2.0, o.y + 363.0));
    super::group_box(ui, g2, &t("dlg-vec-render-as-images"));
    for (ck, nk, y) in [
        ("groups", "groupsN", 278.0),
        ("_blends", "_blendsN", 303.0),
        ("tiled", "tiledN", 328.0),
    ] {
        let w = check_w(&ctx, d, ck);
        changed |= check_at(ui, d, ck, rr(o, 17.0, y, w, 18.0));
        let nr = rr(o, 182.0 + dn, y - 1.0, 33.0, 20.0);
        changed |= number_at(ui, d, nk, nr);
        let en = d.fields.get(field_at(d, nk)).is_some_and(|f| !f.disabled);
        label(
            ui,
            nr.right() + 5.0,
            nr,
            &label_of(d, nk),
            en,
            Align2::LEFT_CENTER,
        );
    }
    // OK, Abbrechen rechts unten
    let bw = super::buttons_w(&ctx, &[&t("btn-ok"), &t("btn-cancel")], 72.0);
    let cancel = Rect::from_min_size(pos2(body.right() - 16.0 - bw, o.y + 378.0), vec2(bw, 22.0));
    let ok = Rect::from_min_size(pos2(cancel.left() - 14.0 - bw, o.y + 378.0), vec2(bw, 22.0));
    super::form::ok_cancel(ui, d, [ok, cancel]);
    (changed, [ok, cancel])
}

/// Anordnung der „Vektordatei-Optionen“ wie im Original.
pub const LAYOUT: CustomLayout = CustomLayout { size, draw };
