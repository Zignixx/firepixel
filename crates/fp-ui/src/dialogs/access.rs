//! Zugriffstasten in Dialogen (NZ-3): Beschriftungen und Kästchen zeigen den Buchstaben nach `&`
//! unterstrichen, Alt+Buchstabe wählt das zugehörige Feld bzw. schaltet das Kästchen um – wie unter
//! Windows. Knöpfe unterstreichen schon in [`super::push_button`]; [`pressed`] wertet die Taste aus.

use crate::inspector::fields;
use crate::theme::Theme;
use egui::{pos2, Id, Rect, Ui};
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
    let Some((before, ch)) = split_access(raw) else { return };
    let theme = Theme::current(ui.ctx());
    let x0 = left + super::text_w(ui.ctx(), &before);
    let x1 = x0 + super::text_w(ui.ctx(), &ch.to_string());
    let y = (center_y + theme.ui_font().size * 0.55).round() + 0.5;
    let col = if enabled { theme.colors.text } else { theme.colors.text_disabled };
    ui.painter().line_segment([pos2(x0, y), pos2(x1, y)], egui::Stroke::new(1.0, col));
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
pub fn checkbox(ui: &mut Ui, id: impl Hash + std::fmt::Debug, r: Rect, raw: &str, checked: bool, enabled: bool) -> Option<bool> {
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
