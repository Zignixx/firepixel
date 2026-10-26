//! Gemeinsame Gestaltungsbausteine für Dialoge, Fenster und den Startbildschirm (Agent DESIGN,
//! Regeln in `docs\DESIGN.md`).
//!
//! Alle Bausteine
//! * nehmen ihre **Texte als Parameter** (die Aufrufer holen sie über `fp_i18n::t`), der Baukasten
//!   selbst enthält keinen Nutzertext,
//! * lesen **Farben nur aus dem [`Theme`]** und sehen in allen drei Designs stimmig aus,
//! * folgen dem **8er-Raster** ([`sp`]) und kürzen zu lange Texte mit „…“ statt sie abzuschneiden.
//!
//! | Baustein | Zweck |
//! |---|---|
//! | [`dialog_header`] | Kopfbereich: Symbol, Titel, Erklärsatz |
//! | [`section`] / [`separator`] | Abschnitt mit Überschrift und Linie, Trennlinie |
//! | [`label_width`] / [`kv_row`] | Beschriftung-Wert-Raster |
//! | [`notice`] | Hinweis-, Warn-, Fehler- und Erfolgsblock |
//! | [`link_row`] / [`link_row_at`] / [`more_link`] | Zeile mit Symbol (Startbildschirm, Listen) |
//! | [`table`] | Tabelle mit Kopf, Zebra, Hover und Auswahl |
//! | [`empty_state`] | leerer Zustand |
//! | [`progress`] | Fortschrittsbalken (bestimmt/unbestimmt) |
//! | [`button_bar`] | Knopfleiste (Standardknopf rechts, gleiche Breiten) |
//! | [`paint_icon`] | zusätzliche eigene Symbole ([`KitIcon`]) im Stil von `icons.rs` |

use crate::icons::{self, IconStyle};
use crate::theme::{Theme, ThemeKind, mix, with_alpha};
use crate::tips::HoverTip;
use egui::{
    Align2, Color32, CornerRadius, FontId, Id, Painter, Pos2, Rect, Response, Sense, Shape, Stroke,
    StrokeKind, Ui, pos2, vec2,
};

/// Abstände im 8er-Raster (logische Pixel). Halbe Schritte (4) nur innerhalb eines Bausteins.
pub mod sp {
    /// Viertelschritt: Abstand Symbol ↔ Text in dichten Zeilen.
    pub const XS: f32 = 4.0;
    /// Grundschritt: Abstand zwischen Elementen einer Gruppe, zwischen Knöpfen.
    pub const S: f32 = 8.0;
    /// Abstand zwischen Beschriftung und Feld, Innenabstand von Hinweisblöcken.
    pub const M: f32 = 12.0;
    /// Dialogrand, Abstand zwischen Abschnitten.
    pub const L: f32 = 16.0;
    /// Großer Abstand (Kopfbereiche, Startbildschirm).
    pub const XL: f32 = 24.0;
    /// Zeilenhöhe einer Beschriftung-Wert-Zeile bzw. Listenzeile.
    pub const ROW: f32 = 24.0;
    /// Knopfhöhe (wie bisher in allen Dialogen).
    pub const BUTTON_H: f32 = 23.0;
    /// Mindestbreite eines Knopfs der Knopfleiste.
    pub const BUTTON_MIN_W: f32 = 76.0;
    /// Höhe der Knopfleiste inklusive Linie und Rändern.
    pub const BUTTON_BAR_H: f32 = 1.0 + 12.0 + BUTTON_H + 12.0;
}

/// Schriftstufen (Größe, fett) – nur diese verwenden.
pub mod type_scale {
    /// Kleine Hinweise, Tabellenköpfe, Fußzeilen.
    pub const CAPTION: f32 = 11.0;
    /// Fließtext, Beschriftungen, Felder (= `Theme::fonts.ui`).
    pub const BODY: f32 = 11.5;
    /// Abschnittsüberschriften (fett).
    pub const SECTION: f32 = 12.0;
    /// Dialogtitel im Kopfbereich (fett).
    pub const TITLE: f32 = 15.0;
    /// Programmname im Startbildschirm (fett).
    pub const DISPLAY: f32 = 30.0;
}

// ---------------------------------------------------------------------- Farben

/// Art eines Hinweisblocks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Info,
    Warning,
    Error,
    Success,
}

impl Tone {
    /// Leitfarbe aus dem Design.
    pub fn color(self, theme: &Theme) -> Color32 {
        let c = &theme.colors;
        match self {
            Tone::Info => c.focus,
            Tone::Warning => c.icon_yellow,
            Tone::Error => c.icon_red,
            Tone::Success => c.icon_green,
        }
    }

    fn icon(self) -> KitIcon {
        match self {
            Tone::Info => KitIcon::Info,
            Tone::Warning => KitIcon::Warning,
            Tone::Error => KitIcon::Error,
            Tone::Success => KitIcon::Success,
        }
    }
}

/// Ist das Design hell (Normal)?
pub fn is_light(theme: &Theme) -> bool {
    theme.kind == ThemeKind::Normal
}

/// Fläche „Karte“ (Startbildschirm, Kopfbereiche): etwas heller bzw. dunkler als der Rahmen.
pub fn card_bg(theme: &Theme) -> Color32 {
    theme.colors.menu_bg
}

/// Getönte Fläche für Hinweise und Hervorhebungen.
pub fn tint(theme: &Theme, color: Color32) -> Color32 {
    let t = match theme.kind {
        ThemeKind::Normal => 0.12,
        ThemeKind::Dark => 0.20,
        ThemeKind::Oled => 0.16,
    };
    mix(card_bg(theme), color, t)
}

/// Barrierefreiheit für selbst gezeichnete Symbolknöpfe („+“, „−“, F/K/U, „ab/cd“, Fenster-,
/// Werkzeug- und Bedienfeldknöpfe): Rolle „Knopf“ mit dem übersetzten Namen (meist der Tooltip),
/// bei Umschaltern mit dem Zustand (gedrückt) wie bei Kästchen. Damit finden Bildschirmleser,
/// egui_kittest und `ui_find` den Knopf.
pub fn icon_button_info(resp: &Response, enabled: bool, name: &str, pressed: Option<bool>) {
    resp.widget_info(|| match pressed {
        Some(on) => egui::WidgetInfo::selected(egui::WidgetType::Button, enabled, on, name),
        None => egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, name),
    });
}

/// Zeichnet eine „geätzte“ waagerechte Linie (dunkel + Lichtkante; OLED nur eine Linie).
pub fn hline(painter: &Painter, x: egui::Rangef, y: f32, theme: &Theme) {
    let c = &theme.colors;
    if theme.glow {
        painter.hline(x, y.round() + 0.5, Stroke::new(1.0, c.line_dark));
    } else {
        let light = theme.kind == ThemeKind::Normal;
        painter.hline(
            x,
            y.round() + 0.5,
            Stroke::new(
                1.0,
                if light {
                    mix(c.line_mid, c.frame_bg, 0.45)
                } else {
                    c.line_dark
                },
            ),
        );
        painter.hline(x, y.round() + 1.5, Stroke::new(1.0, c.line_light));
    }
}

/// Senkrechte geätzte Linie.
pub fn vline(painter: &Painter, x: f32, y: egui::Rangef, theme: &Theme) {
    let c = &theme.colors;
    if theme.glow {
        painter.vline(x.round() + 0.5, y, Stroke::new(1.0, c.line_dark));
    } else {
        let light = theme.kind == ThemeKind::Normal;
        painter.vline(
            x.round() + 0.5,
            y,
            Stroke::new(
                1.0,
                if light {
                    mix(c.line_mid, c.frame_bg, 0.45)
                } else {
                    c.line_dark
                },
            ),
        );
        painter.vline(x.round() + 1.5, y, Stroke::new(1.0, c.line_light));
    }
}

// ---------------------------------------------------------------------- Text

/// Breite eines Texts in einer Schrift.
pub fn text_w(ctx: &egui::Context, text: &str, font: &FontId) -> f32 {
    ctx.fonts_mut(|f| {
        f.layout_no_wrap(text.to_string(), font.clone(), Color32::WHITE)
            .size()
            .x
    })
}

/// Kürzt `text` mit „…“ am Ende, bis er in `max_w` passt.
pub fn elide(ctx: &egui::Context, text: &str, font: &FontId, max_w: f32) -> String {
    if text_w(ctx, text, font) <= max_w {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    // binäre Suche nach der längsten passenden Länge
    let (mut lo, mut hi) = (0usize, chars.len());
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let s: String = chars[..mid].iter().collect::<String>() + "…";
        if text_w(ctx, &s, font) <= max_w {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    chars[..lo]
        .iter()
        .collect::<String>()
        .trim_end()
        .to_string()
        + "…"
}

/// Kürzt einen Pfad in der Mitte („C:\Daten\…\Datei.fw.png“), damit Anfang und Dateiname sichtbar bleiben.
pub fn elide_middle(ctx: &egui::Context, text: &str, font: &FontId, max_w: f32) -> String {
    if text_w(ctx, text, font) <= max_w {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut keep = n;
    while keep > 2 {
        keep -= 1;
        let head = keep / 2;
        let tail = keep - head;
        let s: String = chars[..head].iter().collect::<String>()
            + "…"
            + &chars[n - tail..].iter().collect::<String>();
        if text_w(ctx, &s, font) <= max_w {
            return s;
        }
    }
    "…".into()
}

/// Text in `rect` zeichnen, links und senkrecht mittig, mit „…“ gekürzt. Liefert, ob gekürzt wurde.
pub fn paint_text_fit(
    painter: &Painter,
    rect: Rect,
    text: &str,
    font: FontId,
    color: Color32,
) -> bool {
    let shown = elide(painter.ctx(), text, &font, rect.width());
    let cut = shown != text;
    painter.with_clip_rect(rect.expand(1.0)).text(
        pos2(rect.left(), rect.center().y),
        Align2::LEFT_CENTER,
        shown,
        font,
        color,
    );
    cut
}

// ---------------------------------------------------------------------- Kopfbereich

/// Höhe des Kopfbereichs für einen Erklärsatz der Breite `w` (zum Vorausberechnen der Dialoggröße).
pub fn dialog_header_height(
    ctx: &egui::Context,
    theme: &Theme,
    has_icon: bool,
    text: Option<&str>,
    w: f32,
) -> f32 {
    let text_x = if has_icon {
        sp::L + 32.0 + sp::M
    } else {
        sp::L
    };
    let tw = (w - text_x - sp::L).max(40.0);
    let th = text
        .map(|t| {
            ctx.fonts_mut(|f| {
                f.layout(
                    t.to_string(),
                    theme.font(type_scale::BODY),
                    Color32::WHITE,
                    tw,
                )
                .size()
                .y
            })
        })
        .unwrap_or(0.0);
    let content = type_scale::TITLE + 5.0 + if th > 0.0 { sp::XS + th } else { 0.0 };
    (content.max(if has_icon { 32.0 } else { 0.0 }) + 2.0 * sp::M).ceil() + 2.0
}

/// Kopfbereich eines Dialogs: Symbol (32 px), Titel (fett) und Erklärsatz (umbrochen), helle bzw.
/// dunkle Kartenfläche, geätzte Linie unten. Nimmt die volle Breite von `ui` ein.
pub fn dialog_header(
    ui: &mut Ui,
    icon: Option<HeaderIcon>,
    title: &str,
    text: Option<&str>,
) -> Rect {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let w = ui.available_width();
    let h = dialog_header_height(ui.ctx(), &theme, icon.is_some(), text, w);
    let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 0.0, card_bg(&theme));
    if theme.glow {
        // OLED: feiner Akzentstreifen links
        p.rect_filled(
            Rect::from_min_size(rect.min, vec2(3.0, rect.height() - 2.0)),
            0.0,
            c.focus,
        );
    }
    hline(p, rect.x_range(), rect.bottom() - 2.0, &theme);
    let mut x = rect.left() + sp::L;
    if let Some(i) = icon {
        let ir = Rect::from_min_size(pos2(x, rect.top() + sp::M), vec2(32.0, 32.0));
        match i {
            HeaderIcon::App => icons::paint_app_logo(p, ir),
            HeaderIcon::Icon(icon) => {
                icons::paint(p, ir, icon, &IconStyle::new(&theme, None, true))
            }
            HeaderIcon::Kit(k) => paint_icon(p, ir, k, &IconStyle::new(&theme, None, true)),
        }
        x += 32.0 + sp::M;
    }
    let tw = (rect.right() - sp::L - x).max(40.0);
    let title_r = Rect::from_min_size(
        pos2(x, rect.top() + sp::M),
        vec2(tw, type_scale::TITLE + 5.0),
    );
    paint_text_fit(p, title_r, title, theme.bold(type_scale::TITLE), c.text);
    if let Some(t) = text {
        let g = p.layout(t.to_string(), theme.font(type_scale::BODY), c.text_weak, tw);
        p.galley(pos2(x, title_r.bottom() + sp::XS), g, c.text_weak);
    }
    rect
}

/// Symbol im Kopfbereich.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HeaderIcon {
    /// Das App-Logo (Flamme aus Pixeln).
    App,
    /// Ein Symbol aus `icons.rs`.
    Icon(icons::Icon),
    /// Ein Symbol aus dem Baukasten.
    Kit(KitIcon),
}

// ---------------------------------------------------------------------- Abschnitte

/// Abschnittsüberschrift: fett in der Abschnittsfarbe, dahinter eine Linie bis zum rechten Rand.
/// Danach folgt der Inhalt mit [`sp::S`] Abstand.
pub fn section(ui: &mut Ui, title: &str) -> Rect {
    let theme = Theme::current(ui.ctx());
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 18.0), Sense::hover());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, title));
    section_at(ui.painter(), rect, title, &theme);
    ui.add_space(sp::XS);
    rect
}

/// Abschnittsüberschrift an fester Stelle (für Dialoge mit Rechteck-Layout).
pub fn section_at(painter: &Painter, rect: Rect, title: &str, theme: &Theme) {
    let font = theme.bold(type_scale::SECTION);
    let max = (rect.width() - 24.0).max(20.0);
    let shown = elide(painter.ctx(), title, &font, max);
    let tw = text_w(painter.ctx(), &shown, &font);
    painter.text(
        pos2(rect.left(), rect.center().y),
        Align2::LEFT_CENTER,
        shown,
        font,
        theme.colors.section_label,
    );
    let x0 = rect.left() + tw + sp::S;
    if x0 < rect.right() - 4.0 {
        hline(
            painter,
            egui::Rangef::new(x0, rect.right()),
            rect.center().y.floor(),
            theme,
        );
    }
}

/// Trennlinie über die volle Breite mit [`sp::S`] Abstand oben und unten.
pub fn separator(ui: &mut Ui) {
    let theme = Theme::current(ui.ctx());
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, sp::S * 2.0 + 2.0), Sense::hover());
    hline(ui.painter(), rect.x_range(), rect.center().y - 1.0, &theme);
}

// ---------------------------------------------------------------------- Beschriftung-Wert-Raster

/// Breite der Beschriftungsspalte: breiteste Beschriftung + Abstand (mindestens `min`).
pub fn label_width(ctx: &egui::Context, labels: &[&str], min: f32) -> f32 {
    let font = Theme::current(ctx).font(type_scale::BODY);
    labels
        .iter()
        .map(|l| text_w(ctx, &l.replace('&', ""), &font))
        .fold(min, f32::max)
        .ceil()
        + sp::M
}

/// Eine Zeile im Beschriftung-Wert-Raster: Beschriftung links (Spaltenbreite `label_w`, senkrecht
/// mittig, „…“ falls zu lang), rechts der Inhalt aus `add`.
pub fn kv_row<R>(ui: &mut Ui, label: &str, label_w: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    let theme = Theme::current(ui.ctx());
    ui.horizontal(|ui| {
        ui.set_min_height(sp::ROW);
        let (r, _) = ui.allocate_exact_size(vec2(label_w, sp::ROW), Sense::hover());
        let enabled = ui.is_enabled();
        let color = if enabled {
            theme.colors.text
        } else {
            theme.colors.text_disabled
        };
        let rr = Rect::from_min_max(r.min, pos2(r.right() - sp::M + 2.0, r.max.y));
        if paint_text_fit(ui.painter(), rr, label, theme.font(type_scale::BODY), color) {
            crate::dialogs::report_overflow(
                ui.ctx(),
                format!("Beschriftung „{label}“ breiter als {:.0} px", rr.width()),
            );
        }
        add(ui)
    })
    .inner
}

// ---------------------------------------------------------------------- Hinweisblöcke

/// Hinweisblock: getönte Fläche, Leitstreifen links, Symbol, optional fetter Titel, umbrochener Text.
pub fn notice(ui: &mut Ui, tone: Tone, title: Option<&str>, text: &str) -> Response {
    let theme = Theme::current(ui.ctx());
    let w = ui.available_width();
    let tx = sp::M + 16.0 + sp::S;
    let tw = (w - tx - sp::M).max(40.0);
    let ctx = ui.ctx().clone();
    let tg = title.map(|t| {
        ctx.fonts_mut(|f| {
            f.layout(
                t.to_string(),
                theme.bold(type_scale::BODY),
                Color32::WHITE,
                tw,
            )
        })
    });
    let bg = ctx.fonts_mut(|f| {
        f.layout(
            text.to_string(),
            theme.font(type_scale::BODY),
            Color32::WHITE,
            tw,
        )
    });
    let th = tg.as_ref().map(|g| g.size().y + 2.0).unwrap_or(0.0) + bg.size().y;
    let h = (th.max(16.0) + 2.0 * sp::S + 2.0).ceil();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    // Barrierefreiheit (und egui_kittest/`ui_find`): Titel und Text als Beschriftung
    resp.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Label,
            true,
            title
                .map(|t| format!("{t}\n{text}"))
                .unwrap_or_else(|| text.to_string()),
        )
    });
    notice_at(ui.painter(), rect, tone, title, text, &theme);
    resp
}

/// Höhe eines Hinweisblocks der Breite `w`.
pub fn notice_height(ctx: &egui::Context, title: Option<&str>, text: &str, w: f32) -> f32 {
    let theme = Theme::current(ctx);
    let tw = (w - (sp::M + 16.0 + sp::S) - sp::M).max(40.0);
    let t = title
        .map(|t| {
            ctx.fonts_mut(|f| {
                f.layout(
                    t.to_string(),
                    theme.bold(type_scale::BODY),
                    Color32::WHITE,
                    tw,
                )
                .size()
                .y + 2.0
            })
        })
        .unwrap_or(0.0);
    let b = ctx.fonts_mut(|f| {
        f.layout(
            text.to_string(),
            theme.font(type_scale::BODY),
            Color32::WHITE,
            tw,
        )
        .size()
        .y
    });
    ((t + b).max(16.0) + 2.0 * sp::S + 2.0).ceil()
}

/// Hinweisblock an fester Stelle.
pub fn notice_at(
    p: &Painter,
    rect: Rect,
    tone: Tone,
    title: Option<&str>,
    text: &str,
    theme: &Theme,
) {
    let c = &theme.colors;
    let col = tone.color(theme);
    let r = CornerRadius::same(3);
    p.rect_filled(rect, r, tint(theme, col));
    p.rect_stroke(
        rect,
        r,
        Stroke::new(
            1.0,
            mix(card_bg(theme), col, if theme.glow { 0.8 } else { 0.45 }),
        ),
        StrokeKind::Inside,
    );
    p.rect_filled(
        Rect::from_min_size(rect.min, vec2(3.0, rect.height())),
        CornerRadius {
            nw: 3,
            sw: 3,
            ne: 0,
            se: 0,
        },
        col,
    );
    let ir = Rect::from_min_size(
        pos2(rect.left() + sp::M, rect.top() + sp::S),
        vec2(16.0, 16.0),
    );
    paint_icon(p, ir, tone.icon(), &IconStyle::new(theme, None, true));
    let x = ir.right() + sp::S;
    let tw = (rect.right() - sp::M - x).max(40.0);
    let mut y = rect.top() + sp::S;
    if let Some(t) = title {
        let g = p.layout(t.to_string(), theme.bold(type_scale::BODY), c.text, tw);
        let gh = g.size().y;
        p.galley(pos2(x, y), g, c.text);
        y += gh + 2.0;
    }
    let g = p.layout(text.to_string(), theme.font(type_scale::BODY), c.text, tw);
    p.galley(pos2(x, y), g, c.text);
}

// ---------------------------------------------------------------------- Link-Zeilen

/// Symbol einer Link-Zeile.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RowIcon {
    Icon(icons::Icon),
    Kit(KitIcon),
    None,
}

/// Zeile mit Symbol und Text über die volle Breite (Startbildschirm, Auswahllisten). Hover:
/// getönte Fläche und unterstrichener Text; Text mit „…“ gekürzt; Tooltip optional.
pub fn link_row(ui: &mut Ui, icon: RowIcon, text: &str, tooltip: Option<&str>) -> Response {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, sp::ROW), Sense::hover());
    link_row_at(
        ui,
        rect,
        Id::new(("fp-link-row", text)),
        icon,
        text,
        tooltip,
        true,
    )
}

/// Link-Zeile an fester Stelle. `strong = false` zeichnet den Text gedämpft (z. B. fehlende Datei).
pub fn link_row_at(
    ui: &mut Ui,
    rect: Rect,
    id: Id,
    icon: RowIcon,
    text: &str,
    tooltip: Option<&str>,
    strong: bool,
) -> Response {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let resp = ui.interact(rect, id, Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Link, true, text));
    let p = ui.painter();
    let hovered = resp.hovered() || resp.has_focus();
    if hovered {
        let r = CornerRadius::same(3);
        if theme.glow {
            p.rect_filled(rect, r, with_alpha(c.focus, 30));
            p.rect_stroke(
                rect,
                r,
                Stroke::new(1.0, with_alpha(c.focus, 160)),
                StrokeKind::Inside,
            );
        } else {
            p.rect_filled(rect, r, c.list_hover_bg);
        }
    }
    let mut x = rect.left() + sp::XS + 2.0;
    if icon != RowIcon::None {
        let ir = Rect::from_center_size(pos2(x + 8.0, rect.center().y), vec2(16.0, 16.0));
        let st = IconStyle::new(&theme, None, strong);
        match icon {
            RowIcon::Icon(i) => icons::paint(p, ir, i, &st),
            RowIcon::Kit(k) => paint_icon(p, ir, k, &st),
            RowIcon::None => {}
        }
        x += 16.0 + sp::S;
    }
    let tr = Rect::from_min_max(
        pos2(x, rect.top()),
        pos2(rect.right() - sp::XS, rect.bottom()),
    );
    let font = theme.font(type_scale::BODY);
    let color = if !strong {
        c.text_disabled
    } else if hovered {
        if theme.glow {
            c.focus
        } else {
            c.menu_hover_text
        }
    } else {
        c.text
    };
    let shown = elide(ui.ctx(), text, &font, tr.width());
    let g = p.layout_no_wrap(shown.clone(), font, color);
    let gw = g.size().x;
    let pos = pos2(tr.left(), tr.center().y - g.size().y / 2.0);
    p.galley(pos, g, color);
    if hovered {
        p.hline(
            tr.left()..=tr.left() + gw,
            (tr.center().y + 7.0).round() + 0.5,
            Stroke::new(1.0, with_alpha(color, 160)),
        );
    }
    let resp = if resp.hovered() {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    };
    match tooltip {
        Some(t) if !t.is_empty() => resp.hover_tip(t),
        _ if shown != text => resp.hover_tip(text),
        _ => resp,
    }
}

/// Weiterführender Link mit »-Pfeil (Startbildschirm unten, „Mehr erfahren“).
pub fn more_link(ui: &mut Ui, text: &str, tooltip: Option<&str>) -> Response {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, 22.0), Sense::hover());
    more_link_at(ui, rect, Id::new(("fp-more-link", text)), text, tooltip)
}

/// »-Link an fester Stelle.
pub fn more_link_at(
    ui: &mut Ui,
    rect: Rect,
    id: Id,
    text: &str,
    tooltip: Option<&str>,
) -> Response {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let resp = ui.interact(rect, id, Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Link, true, text));
    let hovered = resp.hovered() || resp.has_focus();
    let p = ui.painter();
    let link = if theme.glow {
        c.focus
    } else if is_light(&theme) {
        mix(c.focus, c.text, 0.25)
    } else {
        mix(c.focus, c.text, 0.35)
    };
    let font = theme.font(type_scale::BODY);
    let max = (rect.width() - 18.0).max(10.0);
    let shown = elide(ui.ctx(), text, &font, max);
    let g = p.layout_no_wrap(shown.clone(), font, link);
    let gw = g.size().x;
    let y = rect.center().y - g.size().y / 2.0;
    p.galley(pos2(rect.left(), y), g, link);
    paint_icon(
        p,
        Rect::from_center_size(
            pos2(rect.left() + gw + 9.0, rect.center().y + 0.5),
            vec2(12.0, 12.0),
        ),
        KitIcon::Chevrons,
        &IconStyle::mono(link, c.frame_bg),
    );
    if hovered {
        p.hline(
            rect.left()..=rect.left() + gw,
            (rect.center().y + 7.0).round() + 0.5,
            Stroke::new(1.0, link),
        );
    }
    let resp = if resp.hovered() {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    };
    match tooltip {
        Some(t) if !t.is_empty() => resp.hover_tip(t),
        _ if shown != text => resp.hover_tip(text),
        _ => resp,
    }
}

// ---------------------------------------------------------------------- Tabelle

/// Spalte einer [`table`].
#[derive(Clone, Copy, Debug)]
pub struct Column<'a> {
    pub title: &'a str,
    /// Breite in Pixeln; `0` = Restbreite (genau eine Spalte sollte 0 haben).
    pub width: f32,
    /// Inhalt rechtsbündig (Zahlen).
    pub right: bool,
}

impl<'a> Column<'a> {
    pub fn new(title: &'a str, width: f32) -> Column<'a> {
        Column {
            title,
            width,
            right: false,
        }
    }
    pub fn rest(title: &'a str) -> Column<'a> {
        Column {
            title,
            width: 0.0,
            right: false,
        }
    }
    pub fn right(mut self) -> Column<'a> {
        self.right = true;
        self
    }
}

/// Ergebnis einer [`table`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TableOut {
    pub clicked: Option<usize>,
    pub double_clicked: Option<usize>,
    pub secondary_clicked: Option<usize>,
}

/// Höhe des Tabellenkopfs.
pub const TABLE_HEAD_H: f32 = 22.0;

/// Tabelle mit Kopf (Verlauf wie Bedienfeld-Kopfzeilen), Zebrastreifen, Hover und Auswahl, in
/// `height` Höhe mit senkrechtem Bildlauf. `cell(ui, zeile, spalte)` zeichnet den Inhalt einer
/// Zelle in einer beschnittenen Unter-Ui (Text dort am besten mit [`cell_text`]).
// Drawing helper whose parameters mirror the primitive; same convention as elsewhere.
#[allow(clippy::too_many_arguments)]
pub fn table(
    ui: &mut Ui,
    id: impl egui::AsId,
    cols: &[Column],
    rows: usize,
    row_h: f32,
    selected: Option<usize>,
    height: f32,
    mut cell: impl FnMut(&mut Ui, usize, usize),
) -> TableOut {
    let theme = Theme::current(ui.ctx());
    let c = theme.colors.clone();
    let id = Id::new(id);
    let w = ui.available_width();
    let (outer, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
    let p = ui.painter().clone();
    p.rect_filled(outer, 0.0, c.list_bg);
    // Spaltenbreiten
    let fixed: f32 = cols.iter().map(|k| k.width).sum();
    let rest = (outer.width() - 2.0 - fixed - 14.0).max(30.0);
    let widths: Vec<f32> = cols
        .iter()
        .map(|k| if k.width <= 0.0 { rest } else { k.width })
        .collect();
    // Kopf
    let head = Rect::from_min_size(
        outer.min + vec2(1.0, 1.0),
        vec2(outer.width() - 2.0, TABLE_HEAD_H),
    );
    crate::widgets::header_bar(&p, head, &theme);
    let mut x = head.left();
    for (k, col) in cols.iter().enumerate() {
        let cr = Rect::from_min_size(pos2(x, head.top()), vec2(widths[k], head.height()));
        let tr = cr.shrink2(vec2(sp::S, 0.0));
        let font = theme.bold(type_scale::CAPTION);
        let shown = elide(ui.ctx(), col.title, &font, tr.width());
        let (pos, al) = if col.right {
            (pos2(tr.right(), tr.center().y), Align2::RIGHT_CENTER)
        } else {
            (pos2(tr.left(), tr.center().y), Align2::LEFT_CENTER)
        };
        p.with_clip_rect(cr).text(pos, al, shown, font, c.tab_text);
        if k + 1 < cols.len() {
            p.vline(
                cr.right() - 0.5,
                (cr.top() + 4.0)..=(cr.bottom() - 4.0),
                Stroke::new(1.0, c.header_line),
            );
        }
        x += widths[k];
    }
    let body = Rect::from_min_max(
        pos2(outer.left() + 1.0, head.bottom()),
        outer.max - vec2(1.0, 1.0),
    );
    let mut out = TableOut::default();
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(body)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    child.set_clip_rect(body.intersect(ui.clip_rect()));
    egui::ScrollArea::vertical()
        .id_salt(id.with("scroll"))
        .auto_shrink([false, false])
        .max_height(body.height())
        .show_rows(&mut child, row_h, rows, |ui, range| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for i in range {
                let (rr, _) =
                    ui.allocate_exact_size(vec2(ui.available_width(), row_h), Sense::hover());
                let resp = ui.interact(rr, id.with(("row", i)), Sense::click());
                let sel = selected == Some(i);
                let bg = if sel {
                    c.list_selected_bg
                } else if resp.hovered() {
                    c.list_hover_bg
                } else if i % 2 == 1 {
                    zebra(&theme)
                } else {
                    c.list_bg
                };
                ui.painter().rect_filled(rr, 0.0, bg);
                if sel && theme.glow {
                    ui.painter().rect_stroke(
                        rr,
                        0.0,
                        Stroke::new(1.0, c.focus),
                        StrokeKind::Inside,
                    );
                }
                let mut x = rr.left();
                for (k, wk) in widths.iter().enumerate() {
                    let cr = Rect::from_min_size(pos2(x, rr.top()), vec2(*wk, row_h));
                    let inner = cr.shrink2(vec2(sp::S, 0.0));
                    let mut cu = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(
                        if cols[k].right {
                            egui::Layout::right_to_left(egui::Align::Center)
                        } else {
                            egui::Layout::left_to_right(egui::Align::Center)
                        },
                    ));
                    cu.set_clip_rect(inner.intersect(ui.clip_rect()));
                    if sel {
                        cu.visuals_mut().override_text_color = Some(c.list_selected_text);
                    }
                    cell(&mut cu, i, k);
                    x += wk;
                }
                if resp.clicked() {
                    out.clicked = Some(i);
                }
                if resp.double_clicked() {
                    out.double_clicked = Some(i);
                }
                if resp.secondary_clicked() {
                    out.secondary_clicked = Some(i);
                }
            }
        });
    p.rect_stroke(
        outer,
        0.0,
        Stroke::new(1.0, c.list_border),
        StrokeKind::Inside,
    );
    out
}

/// Farbe der Zebrastreifen.
pub fn zebra(theme: &Theme) -> Color32 {
    let c = &theme.colors;
    if theme.glow {
        Color32::from_gray(10)
    } else {
        mix(c.list_bg, c.list_row_line, 0.35)
    }
}

/// Text in einer Tabellenzelle (eine Zeile, „…“ falls zu lang, Tooltip mit dem vollen Text).
pub fn cell_text(ui: &mut Ui, text: &str, weak: bool) -> Response {
    let theme = Theme::current(ui.ctx());
    let font = theme.font(type_scale::BODY);
    let max = ui.available_width();
    let shown = elide(ui.ctx(), text, &font, max);
    let color = ui
        .visuals()
        .override_text_color
        .filter(|_| !weak)
        .unwrap_or(if weak {
            theme.colors.text_weak
        } else {
            theme.colors.text
        });
    let r = ui.add(
        egui::Label::new(egui::RichText::new(shown.clone()).font(font).color(color))
            .selectable(false)
            .truncate(),
    );
    if shown != text { r.hover_tip(text) } else { r }
}

// ---------------------------------------------------------------------- leerer Zustand, Fortschritt

/// Leerer Zustand: gedämpftes Symbol (32 px), Titel und optionaler Hinweis, zentriert.
pub fn empty_state(ui: &mut Ui, icon: KitIcon, title: &str, hint: Option<&str>) -> Rect {
    let theme = Theme::current(ui.ctx());
    let w = ui.available_width();
    let h = empty_state_height(ui.ctx(), title, hint, w);
    let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    empty_state_at(ui.painter(), rect, icon, title, hint, &theme);
    rect
}

/// Höhe eines leeren Zustands der Breite `w`.
pub fn empty_state_height(ctx: &egui::Context, title: &str, hint: Option<&str>, w: f32) -> f32 {
    let theme = Theme::current(ctx);
    let tw = (w - 2.0 * sp::S).max(40.0);
    let t = ctx.fonts_mut(|f| {
        f.layout(
            title.to_string(),
            theme.bold(type_scale::BODY),
            Color32::WHITE,
            tw,
        )
        .size()
        .y
    });
    let hh = hint
        .map(|s| {
            ctx.fonts_mut(|f| {
                f.layout(
                    s.to_string(),
                    theme.font(type_scale::CAPTION),
                    Color32::WHITE,
                    tw,
                )
                .size()
                .y
            }) + sp::XS
        })
        .unwrap_or(0.0);
    (sp::S + 32.0 + sp::S + t + hh + sp::S).ceil()
}

/// Leerer Zustand an fester Stelle.
pub fn empty_state_at(
    p: &Painter,
    rect: Rect,
    icon: KitIcon,
    title: &str,
    hint: Option<&str>,
    theme: &Theme,
) {
    let c = &theme.colors;
    let ir = Rect::from_center_size(
        pos2(rect.center().x, rect.top() + sp::S + 16.0),
        vec2(32.0, 32.0),
    );
    let weak = mix(c.icon_light, c.frame_bg, 0.15);
    paint_icon(p, ir, icon, &IconStyle::mono(weak, card_bg(theme)));
    let tw = (rect.width() - 2.0 * sp::S).max(40.0);
    let mut job = egui::text::LayoutJob::simple(
        title.to_string(),
        theme.bold(type_scale::BODY),
        c.text_weak,
        tw,
    );
    job.halign = egui::Align::Center;
    let g = p.ctx().fonts_mut(|f| f.layout_job(job));
    let gh = g.size().y;
    let y = ir.bottom() + sp::S;
    p.galley(pos2(rect.center().x, y), g, c.text_weak);
    if let Some(h) = hint {
        let mut job = egui::text::LayoutJob::simple(
            h.to_string(),
            theme.font(type_scale::CAPTION),
            c.text_weak,
            tw,
        );
        job.halign = egui::Align::Center;
        let g = p.ctx().fonts_mut(|f| f.layout_job(job));
        p.galley(pos2(rect.center().x, y + gh + sp::XS), g, c.text_weak);
    }
}

/// Fortschrittsbalken (8 px hoch) mit Text darüber. `fraction = None` = unbestimmt (wandernder
/// Abschnitt, fordert Neuzeichnen an).
pub fn progress(ui: &mut Ui, fraction: Option<f32>, text: &str) -> Rect {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let w = ui.available_width();
    let has_text = !text.is_empty();
    let (rect, _) = ui.allocate_exact_size(
        vec2(w, if has_text { 18.0 + sp::XS + 8.0 } else { 8.0 }),
        Sense::hover(),
    );
    let p = ui.painter();
    if has_text {
        paint_text_fit(
            p,
            Rect::from_min_size(rect.min, vec2(w, 18.0)),
            text,
            theme.font(type_scale::BODY),
            c.text,
        );
    }
    let bar = Rect::from_min_size(pos2(rect.left(), rect.bottom() - 8.0), vec2(w, 8.0));
    let r = CornerRadius::same(4);
    p.rect_filled(bar, r, c.field_bg);
    p.rect_stroke(bar, r, Stroke::new(1.0, c.field_border), StrokeKind::Inside);
    let inner = bar.shrink(1.0);
    match fraction {
        Some(f) => {
            let f = f.clamp(0.0, 1.0);
            if f > 0.0 {
                p.rect_filled(
                    Rect::from_min_size(inner.min, vec2(inner.width() * f, inner.height())),
                    r,
                    c.focus,
                );
            }
        }
        None => {
            let t = ui.input(|i| i.time) as f32;
            let seg = inner.width() * 0.3;
            let x = inner.left() + ((t * 0.6).fract() * (inner.width() + seg)) - seg;
            let s = Rect::from_min_max(
                pos2(x.max(inner.left()), inner.top()),
                pos2((x + seg).min(inner.right()), inner.bottom()),
            );
            if s.width() > 0.0 {
                p.rect_filled(s, r, c.focus);
            }
            ui.ctx().request_repaint();
        }
    }
    rect
}

// ---------------------------------------------------------------------- Knopfleiste

/// Knopf der [`button_bar`].
#[derive(Clone, Copy, Debug)]
pub struct BarButton<'a> {
    pub label: &'a str,
    /// Standardknopf (Rahmen, Eingabetaste).
    pub default: bool,
    pub enabled: bool,
}

impl<'a> BarButton<'a> {
    pub fn new(label: &'a str) -> BarButton<'a> {
        BarButton {
            label,
            default: false,
            enabled: true,
        }
    }
    pub fn default_button(label: &'a str) -> BarButton<'a> {
        BarButton {
            label,
            default: true,
            enabled: true,
        }
    }
    pub fn enabled(mut self, on: bool) -> BarButton<'a> {
        self.enabled = on;
        self
    }
}

/// Geklickter Knopf einer [`button_bar`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BarClick {
    /// Index in `left` (Hilfe, Zurücksetzen …).
    pub left: Option<usize>,
    /// Index in `right` (OK, Abbrechen …).
    pub right: Option<usize>,
}

/// Mindestbreite einer Knopfleiste (für die Dialogbreite).
pub fn button_bar_width(ctx: &egui::Context, left: &[BarButton], right: &[BarButton]) -> f32 {
    let labels: Vec<&str> = left.iter().chain(right.iter()).map(|b| b.label).collect();
    let w = crate::dialogs::buttons_w(ctx, &labels, sp::BUTTON_MIN_W);
    let n = labels.len() as f32;
    2.0 * sp::L + n * w + (n - 1.0).max(0.0) * sp::S + if left.is_empty() { 0.0 } else { sp::L }
}

/// Knopfleiste am unteren Rand von `rect` (volle Breite): geätzte Linie, rechts die Knöpfe in
/// Windows-Reihenfolge (OK, Abbrechen; Standardknopf mit Rahmen), links Nebenknöpfe. Alle Knöpfe
/// gleich breit (längste Beschriftung, mindestens 76 px), Höhe 23 px. Eingabetaste löst den
/// Standardknopf aus (über `dialogs::push_button`).
pub fn button_bar(ui: &mut Ui, rect: Rect, left: &[BarButton], right: &[BarButton]) -> BarClick {
    let bar = Rect::from_min_max(
        pos2(rect.left(), rect.bottom() - sp::BUTTON_BAR_H),
        rect.max,
    );
    // Fläche und geätzte Linie zeichnet der Dialograhmen als Fußleiste (DES-1) unter dem Inhalt
    crate::dialogs::set_footer(
        ui,
        Rect::from_min_max(pos2(bar.left(), bar.top() - 1.0), bar.max),
    );
    let labels: Vec<&str> = left.iter().chain(right.iter()).map(|b| b.label).collect();
    let w = crate::dialogs::buttons_w(ui.ctx(), &labels, sp::BUTTON_MIN_W);
    let top = bar.bottom() - sp::M - sp::BUTTON_H;
    let mut out = BarClick::default();
    let n = right.len() as f32;
    let x0 = bar.right() - sp::L - n * w - (n - 1.0).max(0.0) * sp::S;
    for (i, b) in right.iter().enumerate() {
        let r = Rect::from_min_size(
            pos2(x0 + i as f32 * (w + sp::S), top),
            vec2(w, sp::BUTTON_H),
        );
        if crate::dialogs::push_button(ui, r, b.label, b.default, b.enabled) {
            out.right = Some(i);
        }
    }
    for (i, b) in left.iter().enumerate() {
        let r = Rect::from_min_size(
            pos2(bar.left() + sp::L + i as f32 * (w + sp::S), top),
            vec2(w, sp::BUTTON_H),
        );
        if crate::dialogs::push_button(ui, r, b.label, b.default, b.enabled) {
            out.left = Some(i);
        }
    }
    out
}

// ---------------------------------------------------------------------- Symbole

/// Zusätzliche Symbole des Baukastens (eigenes Design, 16er-Raster wie `icons.rs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KitIcon {
    /// FW-PNG-Dokument: Blatt mit drei Pixeln in Flammenfarben.
    FwDocument,
    /// Bilddatei: Blatt mit Berg und Sonne.
    Image,
    /// Skript (.jsf): Blatt mit Eingabezeichen.
    Script,
    /// Neues Dokument: Blatt mit Plus.
    NewDocument,
    /// Vorlage: Blatt mit Layoutblöcken.
    Template,
    /// Erweitern: Puzzleteil.
    Extend,
    /// Gespeicherte Befehle: Liste mit Häkchen.
    Commands,
    /// Kurzanleitung: aufgeschlagenes Buch.
    Book,
    /// Tastenkürzel: Tastatur.
    Keyboard,
    /// KI-Modell: Chip mit Funken.
    AiChip,
    /// Automatisierung: Zahnrad.
    Gear,
    /// Hinweis.
    Info,
    /// Warnung.
    Warning,
    /// Fehler.
    Error,
    /// Erfolg.
    Success,
    /// »-Pfeile.
    Chevrons,
    /// Leerer Zustand: leere Ablage.
    EmptyTray,
}

/// Alle Symbole des Baukastens (für Tests und Bildtafeln).
pub const ALL_KIT_ICONS: &[KitIcon] = &[
    KitIcon::FwDocument,
    KitIcon::Image,
    KitIcon::Script,
    KitIcon::NewDocument,
    KitIcon::Template,
    KitIcon::Extend,
    KitIcon::Commands,
    KitIcon::Book,
    KitIcon::Keyboard,
    KitIcon::AiChip,
    KitIcon::Gear,
    KitIcon::Info,
    KitIcon::Warning,
    KitIcon::Error,
    KitIcon::Success,
    KitIcon::Chevrons,
    KitIcon::EmptyTray,
];

struct G<'a> {
    p: &'a Painter,
    o: Pos2,
    s: f32,
}

impl G<'_> {
    fn pt(&self, x: f32, y: f32) -> Pos2 {
        pos2(self.o.x + x * self.s, self.o.y + y * self.s)
    }
    fn w(&self, u: f32) -> f32 {
        (u * self.s).max(1.0)
    }
    fn line(&self, c: Color32, width: f32, pts: &[(f32, f32)]) {
        let v: Vec<Pos2> = pts.iter().map(|&(x, y)| self.pt(x, y)).collect();
        self.p.add(Shape::line(v, Stroke::new(self.w(width), c)));
    }
    fn convex(&self, fill: Color32, stroke: Option<Color32>, pts: &[(f32, f32)]) {
        let v: Vec<Pos2> = pts.iter().map(|&(x, y)| self.pt(x, y)).collect();
        self.p.add(Shape::convex_polygon(
            v,
            fill,
            stroke
                .map(|c| Stroke::new(self.w(1.0), c))
                .unwrap_or(Stroke::NONE),
        ));
    }
    fn rect(&self, x: f32, y: f32, w: f32, h: f32, fill: Option<Color32>, stroke: Option<Color32>) {
        let r = Rect::from_min_max(self.pt(x, y), self.pt(x + w, y + h));
        if let Some(f) = fill {
            self.p.rect_filled(r, 0.0, f);
        }
        if let Some(c) = stroke {
            self.p
                .rect_stroke(r, 0.0, Stroke::new(self.w(1.0), c), StrokeKind::Inside);
        }
    }
    // Drawing helper whose parameters mirror the primitive; same convention as elsewhere.
    #[allow(clippy::too_many_arguments)]
    fn rrect(
        &self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        rad: f32,
        fill: Option<Color32>,
        stroke: Option<Color32>,
    ) {
        let r = Rect::from_min_max(self.pt(x, y), self.pt(x + w, y + h));
        let cr = CornerRadius::same((rad * self.s).round().clamp(0.0, 255.0) as u8);
        if let Some(f) = fill {
            self.p.rect_filled(r, cr, f);
        }
        if let Some(c) = stroke {
            self.p
                .rect_stroke(r, cr, Stroke::new(self.w(1.0), c), StrokeKind::Inside);
        }
    }
    fn circle(&self, x: f32, y: f32, r: f32, fill: Option<Color32>, stroke: Option<Color32>) {
        let c = self.pt(x, y);
        if let Some(f) = fill {
            self.p.circle_filled(c, r * self.s, f);
        }
        if let Some(s) = stroke {
            self.p
                .circle_stroke(c, r * self.s, Stroke::new(self.w(1.0), s));
        }
    }
    /// Blatt mit Eselsohr (x, y, b, h).
    fn page(&self, st: &IconStyle, x: f32, y: f32, w: f32, h: f32) {
        let f = 3.5;
        self.convex(
            st.paper,
            None,
            &[
                (x, y),
                (x + w - f, y),
                (x + w, y + f),
                (x + w, y + h),
                (x, y + h),
            ],
        );
        self.line(
            st.fg,
            1.0,
            &[
                (x + 0.5, y + 0.5),
                (x + w - f, y + 0.5),
                (x + w - 0.5, y + f),
                (x + w - 0.5, y + h - 0.5),
                (x + 0.5, y + h - 0.5),
                (x + 0.5, y + 0.5),
            ],
        );
        self.line(
            st.fg,
            1.0,
            &[
                (x + w - f, y + 0.5),
                (x + w - f, y + f),
                (x + w - 0.5, y + f),
            ],
        );
    }
}

/// Zeichnet ein Baukasten-Symbol in `rect` (quadratisch eingepasst).
pub fn paint_icon(painter: &Painter, rect: Rect, icon: KitIcon, st: &IconStyle) {
    let side = rect.width().min(rect.height());
    let g = G {
        p: painter,
        o: pos2(rect.center().x - side / 2.0, rect.center().y - side / 2.0),
        s: side / 16.0,
    };
    let (fg, paper, light) = (st.fg, st.paper, st.light);
    match icon {
        KitIcon::FwDocument => {
            g.page(st, 2.5, 1.0, 11.0, 14.0);
            // drei Pixel in Flammenfarben, treppenförmig
            g.rect(4.5, 10.0, 2.5, 2.5, Some(st.red), None);
            g.rect(7.0, 7.5, 2.5, 2.5, Some(st.accent2), None);
            g.rect(9.5, 10.0, 2.5, 2.5, Some(st.yellow), None);
            g.rect(
                7.0,
                10.0,
                2.5,
                2.5,
                Some(mix(st.red, st.accent2, 0.5)),
                None,
            );
        }
        KitIcon::Image => {
            g.page(st, 2.5, 1.0, 11.0, 14.0);
            g.convex(
                st.green,
                None,
                &[(3.5, 13.5), (7.0, 8.5), (9.0, 11.0), (3.5, 13.5)],
            );
            g.convex(
                mix(st.green, fg, 0.25),
                None,
                &[(6.5, 13.5), (9.5, 9.5), (12.5, 13.5)],
            );
            g.circle(10.0, 5.5, 1.4, Some(st.yellow), None);
        }
        KitIcon::Script => {
            g.page(st, 2.5, 1.0, 11.0, 14.0);
            g.line(st.accent, 1.3, &[(4.5, 7.0), (6.5, 9.0), (4.5, 11.0)]);
            g.line(fg, 1.2, &[(7.5, 11.5), (11.0, 11.5)]);
        }
        KitIcon::NewDocument => {
            g.page(st, 2.5, 1.0, 10.0, 13.5);
            g.circle(12.0, 12.0, 3.6, Some(st.accent), None);
            g.line(paper, 1.4, &[(12.0, 10.0), (12.0, 14.0)]);
            g.line(paper, 1.4, &[(10.0, 12.0), (14.0, 12.0)]);
        }
        KitIcon::Template => {
            g.page(st, 2.5, 1.0, 11.0, 14.0);
            g.rect(4.5, 4.0, 5.0, 3.0, Some(st.accent), None);
            g.rect(4.5, 8.5, 3.0, 4.5, Some(light), None);
            g.rect(8.5, 8.5, 3.0, 1.5, Some(light), None);
            g.rect(8.5, 11.5, 3.0, 1.5, Some(light), None);
        }
        KitIcon::Extend => {
            // Puzzleteil: Quadrat mit Nase oben und rechts
            g.rect(2.5, 5.0, 9.0, 9.0, Some(st.accent), None);
            g.circle(7.0, 4.2, 2.0, Some(st.accent), None);
            g.circle(12.3, 9.5, 2.0, Some(st.accent), None);
            g.circle(2.5, 9.5, 1.6, Some(paper), None);
            g.circle(7.0, 14.0, 1.6, Some(paper), None);
        }
        KitIcon::Commands => {
            for (i, y) in [3.5f32, 8.0, 12.5].iter().enumerate() {
                g.rrect(1.5, y - 2.0, 4.0, 4.0, 0.8, Some(paper), Some(fg));
                if i < 2 {
                    g.line(st.green, 1.2, &[(2.4, *y), (3.3, y + 1.0), (4.8, y - 1.2)]);
                }
                g.line(fg, 1.3, &[(7.5, *y), (14.5, *y)]);
            }
        }
        KitIcon::Book => {
            g.convex(
                paper,
                Some(fg),
                &[(1.5, 3.0), (7.5, 4.0), (7.5, 14.0), (1.5, 13.0)],
            );
            g.convex(
                paper,
                Some(fg),
                &[(8.5, 4.0), (14.5, 3.0), (14.5, 13.0), (8.5, 14.0)],
            );
            g.line(st.accent, 1.0, &[(3.0, 6.0), (6.0, 6.5)]);
            g.line(light, 1.0, &[(3.0, 8.5), (6.0, 9.0)]);
            g.line(light, 1.0, &[(10.0, 6.5), (13.0, 6.0)]);
            g.line(light, 1.0, &[(10.0, 9.0), (13.0, 8.5)]);
        }
        KitIcon::Keyboard => {
            g.rrect(0.5, 4.0, 15.0, 9.0, 1.5, Some(paper), Some(fg));
            for row in 0..2 {
                for k in 0..5 {
                    g.rect(
                        2.2 + k as f32 * 2.4,
                        5.8 + row as f32 * 2.3,
                        1.5,
                        1.4,
                        Some(fg),
                        None,
                    );
                }
            }
            g.rect(4.5, 10.4, 7.0, 1.4, Some(st.accent), None);
        }
        KitIcon::AiChip => {
            for k in 0..3 {
                let t = 5.0 + k as f32 * 3.0;
                g.line(fg, 1.0, &[(t, 1.0), (t, 3.0)]);
                g.line(fg, 1.0, &[(t, 13.0), (t, 15.0)]);
                g.line(fg, 1.0, &[(1.0, t), (3.0, t)]);
                g.line(fg, 1.0, &[(13.0, t), (15.0, t)]);
            }
            g.rrect(3.0, 3.0, 10.0, 10.0, 1.5, Some(paper), Some(fg));
            // Funke
            g.convex(
                st.accent,
                None,
                &[
                    (8.0, 4.8),
                    (9.0, 7.0),
                    (11.2, 8.0),
                    (9.0, 9.0),
                    (8.0, 11.2),
                    (7.0, 9.0),
                    (4.8, 8.0),
                    (7.0, 7.0),
                ],
            );
        }
        KitIcon::Gear => {
            for k in 0..8 {
                let a = k as f32 * std::f32::consts::FRAC_PI_4;
                let (s, c) = a.sin_cos();
                g.line(
                    fg,
                    2.2,
                    &[
                        (8.0 + c * 4.5, 8.0 + s * 4.5),
                        (8.0 + c * 7.0, 8.0 + s * 7.0),
                    ],
                );
            }
            g.circle(8.0, 8.0, 5.0, Some(fg), None);
            g.circle(8.0, 8.0, 2.2, Some(paper), None);
        }
        KitIcon::Info => {
            g.circle(8.0, 8.0, 7.0, Some(st.blue), None);
            g.rect(7.1, 6.8, 1.8, 5.5, Some(Color32::WHITE), None);
            g.rect(7.1, 3.6, 1.8, 1.8, Some(Color32::WHITE), None);
        }
        KitIcon::Warning => {
            g.convex(
                st.yellow,
                Some(mix(st.yellow, Color32::BLACK, 0.35)),
                &[(8.0, 1.0), (15.2, 14.5), (0.8, 14.5)],
            );
            g.rect(7.1, 5.5, 1.8, 5.0, Some(Color32::BLACK), None);
            g.rect(7.1, 11.5, 1.8, 1.8, Some(Color32::BLACK), None);
        }
        KitIcon::Error => {
            g.circle(8.0, 8.0, 7.0, Some(st.red), None);
            g.line(Color32::WHITE, 1.8, &[(5.2, 5.2), (10.8, 10.8)]);
            g.line(Color32::WHITE, 1.8, &[(10.8, 5.2), (5.2, 10.8)]);
        }
        KitIcon::Success => {
            g.circle(8.0, 8.0, 7.0, Some(st.green), None);
            g.line(Color32::WHITE, 1.8, &[(4.5, 8.3), (7.0, 10.8), (11.5, 5.5)]);
        }
        KitIcon::Chevrons => {
            g.line(fg, 1.4, &[(3.0, 4.0), (7.0, 8.0), (3.0, 12.0)]);
            g.line(fg, 1.4, &[(8.5, 4.0), (12.5, 8.0), (8.5, 12.0)]);
        }
        KitIcon::EmptyTray => {
            g.line(
                fg,
                1.2,
                &[
                    (1.5, 9.0),
                    (4.0, 3.0),
                    (12.0, 3.0),
                    (14.5, 9.0),
                    (14.5, 13.5),
                    (1.5, 13.5),
                    (1.5, 9.0),
                ],
            );
            g.line(
                fg,
                1.2,
                &[
                    (1.5, 9.0),
                    (5.0, 9.0),
                    (6.0, 11.0),
                    (10.0, 11.0),
                    (11.0, 9.0),
                    (14.5, 9.0),
                ],
            );
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Führt `f` in einem Frame mit geladenen Schriften aus.
    pub(crate) fn with_ctx(f: impl FnOnce(&egui::Context)) {
        let ctx = egui::Context::default();
        crate::theme::install_fallback_fonts(&ctx);
        ctx.run_ui(Default::default(), |_| {})
            .textures_delta
            .clear();
        let mut f = Some(f);
        ctx.run_ui(Default::default(), |ui| {
            if let Some(f) = f.take() {
                f(ui.ctx());
            }
        })
        .textures_delta
        .clear();
    }

    #[test]
    fn elide_shortens_with_ellipsis() {
        with_ctx(elide_checks);
    }

    fn elide_checks(ctx: &egui::Context) {
        let font = FontId::proportional(12.0);
        let long = "Ein sehr langer Dateiname mit vielen Wörtern.fw.png";
        let s = elide(ctx, long, &font, 80.0);
        assert!(s.ends_with('…'));
        assert!(text_w(ctx, &s, &font) <= 80.0);
        assert_eq!(elide(ctx, "kurz", &font, 200.0), "kurz");
        let m = elide_middle(
            ctx,
            r"C:\Daten\Projekte\2026\Kunde\Entwurf\banner.fw.png",
            &font,
            140.0,
        );
        assert!(m.contains('…') && m.ends_with("png"));
    }

    #[test]
    fn button_bar_width_grows_with_labels() {
        with_ctx(|ctx| {
            let short = button_bar_width(
                ctx,
                &[],
                &[BarButton::default_button("OK"), BarButton::new("Abbrechen")],
            );
            let long = button_bar_width(
                ctx,
                &[],
                &[
                    BarButton::default_button("OK"),
                    BarButton::new("Ein wirklich sehr langer Knopf"),
                ],
            );
            assert!(short >= 2.0 * sp::L + 2.0 * sp::BUTTON_MIN_W + sp::S);
            assert!(long > short);
        });
    }
}
