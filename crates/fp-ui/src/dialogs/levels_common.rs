//! Gemeinsame Bausteine der Kurven- und Tonwertdialoge (re\19-kurven-tonwert.md §1):
//! app-weit gemerkter Vorschau-Schalter (§1.2), Kanal-Histogramme des Originalbilds (Auto),
//! Pipetten-Knöpfe und Aufnahme der **angezeigten** Farbe auf der Leinwand (§1.3).
//!
//! Angelegt von KV-1 (Kurven), mitbenutzt von KV-2 (Tonwertkorrektur).

use crate::app_state::AppState;
use crate::inspector::{fields, glyphs::Glyph};
use crate::theme::Theme;
use egui::{Pos2, Rect, Stroke, StrokeKind, Ui, pos2, vec2};
use fp_engine::PrefValue;
pub use fp_raster::filters::curve_edit::Pipette;

/// Voreinstellung des gemeinsamen Vorschau-Schalters (Kurven und Tonwert, Vorgabe an). Wird beim
/// Umschalten sofort geschrieben, auch wenn der Dialog danach abgebrochen wird.
pub const PREVIEW_PREF: &str = "AdjustDialogPreview";

/// Kanal-Liste „RGB, Rot, Grün, Blau“ (Start immer RGB).
pub const CHANNELS: [&str; 4] = [
    "dlg-channel-rgb",
    "dlg-channel-red",
    "dlg-channel-green",
    "dlg-channel-blue",
];

/// Gemerkter Zustand des Vorschau-Schalters.
pub fn preview_pref(st: &AppState) -> bool {
    st.session.prefs.get_bool(PREVIEW_PREF, true)
}

/// Vorschau-Schalter sofort merken (app-weit, beide Dialoge).
pub fn set_preview_pref(st: &mut AppState, on: bool) {
    st.session.prefs.set(PREVIEW_PREF, PrefValue::Bool(on));
}

/// Histogramme R, G, B der gewählten Objekte (Bitmaps direkt, andere gerendert), nur Pixel mit
/// Alpha > 0. Beim Öffnen des Dialogs rechnen, solange noch keine Vorschau angewandt ist.
pub fn channel_histograms(st: &AppState) -> [[u64; 256]; 3] {
    let mut h = [[0u64; 256]; 3];
    let Some(d) = st.session.active() else {
        return h;
    };
    for id in &d.selection.elements {
        let Some(e) = fp_engine::docops::element_on_page(&d.doc, d.page, *id) else {
            continue;
        };
        let img = match &e.kind {
            fp_model::ElementKind::Bitmap(b) => fp_engine::docops::bitmap_image(b),
            _ => Some(
                st.renderer
                    .render_element(&d.doc, &e, &fp_render::PointMap::identity(), d.frame)
                    .image,
            ),
        };
        let Some(img) = img else { continue };
        for p in img.data() {
            if p[3] == 0 {
                continue;
            }
            for c in 0..3 {
                h[c][p[c] as usize] += 1;
            }
        }
    }
    h
}

/// Tooltip-Schlüssel einer Pipette.
pub fn pipette_tip_key(p: Pipette) -> &'static str {
    match p {
        Pipette::Shadow => "dlg-pipette-shadow",
        Pipette::Midtone => "dlg-pipette-midtone",
        Pipette::Highlight => "dlg-pipette-highlight",
    }
}

/// Drei Pipetten-Knöpfe (je 24 × 23) an den linken Kanten `xs`, Oberkante `y`. Der aktive
/// (Aufnahmemodus) erscheint gedrückt. Liefert die angeklickte Pipette.
pub fn pipette_buttons(
    ui: &mut Ui,
    salt: &str,
    xs: [f32; 3],
    y: f32,
    active: Option<Pipette>,
) -> Option<Pipette> {
    let t = Theme::current(ui.ctx());
    let mut out = None;
    for (k, p) in Pipette::ALL.into_iter().enumerate() {
        let r = Rect::from_min_size(pos2(xs[k], y), vec2(24.0, 23.0));
        if active == Some(p) {
            ui.painter().rect_filled(r, 2.0, t.colors.button_pressed_bg);
        }
        ui.painter().rect_stroke(
            r,
            2.0,
            Stroke::new(1.0, t.colors.button_border),
            StrokeKind::Inside,
        );
        if fields::icon_button(
            ui,
            (salt, "pipette", k),
            r,
            Glyph::Eyedropper,
            true,
            &fp_i18n::t(pipette_tip_key(p)),
        )
        .clicked()
        {
            out = Some(p);
        }
    }
    out
}

/// Im Aufnahmemodus vor dem Dialograhmen aufrufen: verbraucht Esc (beendet nur die Aufnahme, nicht
/// den Dialog). `true`, wenn Esc gedrückt wurde.
pub fn consume_escape(ctx: &egui::Context) -> bool {
    ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
}

/// Angezeigte Farbe an der Bildschirmstelle `p` (Leinwand des aktiven Dokuments). Mit
/// eingeschalteter Vorschau ist das schon die korrigierte Farbe, weil die Vorschau im Dokument
/// steht. Außerhalb der Leinwand bzw. Seite: `None`.
pub fn displayed_color_at(st: &AppState, p: Pos2) -> Option<[u8; 3]> {
    let d = st.session.active()?;
    let area = st.canvases.get(&d.id)?.area?;
    if !area.contains(p) {
        return None;
    }
    let q = d
        .view
        .screen_to_doc((p.x - area.left()) as f64, (p.y - area.top()) as f64);
    let (x, y) = (q.x.floor() as i32, q.y.floor() as i32);
    let page = fp_render::Renderer::page_rect(&d.doc, d.page)?;
    if !page.contains(x, y) {
        return None;
    }
    let img = st
        .renderer
        .render_rect(
            &d.doc,
            d.page,
            d.frame,
            fp_geom::IRect::from_xywh(x, y, 1, 1),
        )
        .image;
    let px = img.get(x, y)?;
    Some([px[0], px[1], px[2]])
}

/// Klick außerhalb des Dialogs im Aufnahmemodus: Lage des Klicks, falls in diesem Bild einer
/// ausgelöst wurde (der Hintergrund des modalen Dialogs meldet ihn sonst als „schließen“).
pub fn outside_click(ctx: &egui::Context, dialog: Rect) -> Option<Pos2> {
    ctx.input(|i| {
        let released = i.pointer.primary_released();
        let pos = i.pointer.interact_pos()?;
        (released && !dialog.contains(pos)).then_some(pos)
    })
}
