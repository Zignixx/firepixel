//! Zugriffstasten in Dialogen (NZ-3): Beschriftungen und Kästchen zeigen den Buchstaben nach `&`
//! unterstrichen, Alt+Buchstabe wählt das zugehörige Feld bzw. schaltet das Kästchen um – wie unter
//! Windows. Knöpfe unterstreichen schon in [`super::push_button`]; [`pressed`] wertet die Taste aus.

use crate::inspector::fields;
use crate::theme::Theme;
use egui::{Id, Rect, Ui, pos2};
use std::hash::Hash;

/// Anzeigetext ohne Zugriffstaste (`&&` bleibt ein `&`).
pub fn plain(raw: &str) -> String {
    fp_i18n::strip_access(raw).0
}

/// Anzeigetext vor der Zugriffstaste und die Taste selbst (`"E&ingabe"` → `("E", 'i')`); `&&` ist
/// ein wörtliches `&`. `None` ohne Zugriffstaste.
pub fn split_access(raw: &str) -> Option<(String, char)> {
    let mut before = String::new();
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '&' {
            match chars.peek().copied() {
                Some('&') => {
                    chars.next();
                    before.push('&');
                }
                Some(n) if !n.is_whitespace() => return Some((before, n)),
                _ => {}
            }
        } else {
            before.push(c);
        }
    }
    None
}

/// Alt+Zugriffstaste von `raw` gedrückt? Verbraucht die Taste.
pub fn pressed(ui: &Ui, raw: &str) -> bool {
    super::access_key(raw).is_some_and(|k| ui.input_mut(|i| i.consume_key(egui::Modifiers::ALT, k)))
}

/// Unterstreicht die Zugriffstaste eines links ausgerichteten, senkrecht mittigen Texts in der
/// Dialogschrift, der bei `left` beginnt.
pub fn underline(ui: &Ui, left: f32, center_y: f32, raw: &str, enabled: bool) {
    let Some((before, ch)) = split_access(raw) else {
        return;
    };
    let theme = Theme::current(ui.ctx());
    let x0 = left + super::text_w(ui.ctx(), &before);
    let x1 = x0 + super::text_w(ui.ctx(), &ch.to_string());
    let y = (center_y + theme.ui_font().size * 0.55).round() + 0.5;
    let col = if enabled {
        theme.colors.text
    } else {
        theme.colors.text_disabled
    };
    ui.painter()
        .line_segment([pos2(x0, y), pos2(x1, y)], egui::Stroke::new(1.0, col));
}

/// Rechtsbündige Beschriftung vor einem Feld ([`fields::label_right`]) mit unterstrichener
/// Zugriffstaste.
pub fn label_right(ui: &Ui, r: Rect, raw: &str, enabled: bool) {
    let text = plain(raw);
    fields::label_right(ui, r, &text, enabled);
    let w = super::text_w(ui.ctx(), &text);
    underline(ui, r.right() - w, r.center().y, raw, enabled);
}

/// Kontrollkästchen ([`fields::checkbox`]) mit unterstrichener Zugriffstaste; Alt+Taste schaltet um.
pub fn checkbox(
    ui: &mut Ui,
    id: impl Hash + std::fmt::Debug,
    r: Rect,
    raw: &str,
    checked: bool,
    enabled: bool,
) -> Option<bool> {
    let out = fields::checkbox(ui, id, r, &plain(raw), checked, enabled);
    // Text beginnt 5 px rechts vom 13-px-Kästchen (wie in `fields::checkbox`)
    underline(ui, r.left() + 18.0, r.center().y, raw, enabled);
    if out.is_none() && enabled && pressed(ui, raw) {
        return Some(!checked);
    }
    out
}

/// Kennung des Eingabefelds eines [`fields::number`]-/[`fields::text`]-Felds mit der Kennung `id`.
pub fn field_edit_id(id: impl Hash + std::fmt::Debug) -> Id {
    Id::new(id).with("te")
}

/// Gibt dem Feld `id` den Fokus und markiert seinen Inhalt (`text_len` Zeichen), wie Alt+Taste
/// bzw. ein Klick auf den zugehörigen Griff im Original.
pub fn focus_field(ui: &Ui, id: impl Hash + std::fmt::Debug, text_len: usize) {
    let te = field_edit_id(id);
    ui.memory_mut(|m| m.request_focus(te));
    super::select_all(ui.ctx(), te, text_len);
}

/// Feld, das gerade den Fokus hat, ist das Feld `id`?
pub fn field_has_focus(ctx: &egui::Context, id: impl Hash + std::fmt::Debug) -> bool {
    let te = field_edit_id(id);
    ctx.memory(|m| m.has_focus(te))
}

/// Optionsfelder ([`super::radio_group`], eine Gruppe = ein Tab-Stopp) mit unterstrichener
/// Zugriffstaste je Feld (`raw` mit `&`); Alt+Taste wählt das Feld und gibt ihm den Fokus wie ein
/// Klick (S-4, `re\32`: D14 G/A/S, D21 I/T/B). Gesperrt wirkt die Taste nicht.
pub fn radio(
    ui: &mut Ui,
    id: impl Hash + std::fmt::Debug,
    rects: &[Rect],
    raw: &[&str],
    sel: usize,
    enabled: bool,
) -> Option<usize> {
    let gid = Id::new(id);
    let shown: Vec<String> = raw.iter().map(|s| plain(s)).collect();
    let refs: Vec<&str> = shown.iter().map(String::as_str).collect();
    let mut out = super::radio_group(ui, gid, rects, &refs, sel, enabled);
    let n = rects.len().min(raw.len());
    for (k, r) in rects.iter().enumerate().take(n) {
        // Text beginnt 17 px rechts der Feldkante (wie in `radio_group`)
        underline(ui, r.left() + 17.0, r.center().y, raw[k], enabled);
        if enabled && pressed(ui, raw[k]) {
            ui.memory_mut(|m| m.request_focus(radio_id(gid, k)));
            out = (k != sel).then_some(k).or(out);
        }
    }
    out
}

/// Kennung des Optionsfelds `k` einer Gruppe aus [`radio`] mit der Gruppenkennung `gid`.
pub fn radio_id(gid: Id, k: usize) -> Id {
    Id::new(gid).with(k)
}

/// Kennung eines Farbknopfs ([`crate::inspector::popups::color_button`]) mit der Kennung `id`
/// (dort zweimal gehasht: `color_button_with` → `fields::swatch`).
pub fn color_button_id(id: impl Hash + std::fmt::Debug) -> Id {
    Id::new(Id::new(id))
}

/// Gibt dem Farbknopf `id` den Fokus, ohne ihn zu öffnen (Alt+Taste an seiner Beschriftung, D22).
pub fn focus_color_button(ui: &Ui, id: impl Hash + std::fmt::Debug) {
    ui.memory_mut(|m| m.request_focus(color_button_id(id)));
}

const FOCUS_NEXT: &str = "fp-access-focus-next";

/// Ziel einer Beschriftungstaste ist gesperrt: wie unter Windows geht der Fokus zum nächsten
/// aktiven Tab-Stopp. `from` ist das letzte aktive Element vor dem gesperrten Ziel; es bekommt jetzt
/// den Fokus, [`pending_focus_next`] schiebt ihn im nächsten Bild eins weiter (D22 Alt+G → OK).
pub fn focus_next_after(ui: &Ui, from: Id) {
    ui.memory_mut(|m| m.request_focus(from));
    ui.ctx()
        .data_mut(|d| d.insert_temp(Id::new(FOCUS_NEXT), Some(from)));
    ui.ctx().request_repaint();
}

/// Siehe [`focus_next_after`]; vor den Bedienelementen des Dialogs aufrufen.
pub fn pending_focus_next(ui: &Ui) {
    if let Some(from) = ui
        .ctx()
        .data_mut(|d| d.remove_temp::<Option<Id>>(Id::new(FOCUS_NEXT)))
        .flatten()
        && ui.memory(|m| m.has_focus(from))
    {
        ui.memory_mut(|m| m.move_focus(egui::FocusDirection::Next));
        ui.ctx().request_repaint();
    }
}
