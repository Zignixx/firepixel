//! Tooltip switches (FB-37, FB-98): View ▸ Tooltips and the two options under Preferences ▸
//! General.
//!
//! - Tooltips on: every tooltip in full ("Pointer (V, 0)" plus the description line).
//! - Tooltips off: only the short first line stays ("Pointer (V, 0)"); the description lines go.
//!   A tooltip of one line (a panel button, an inspector control) stays as it is.
//! - Tooltips off with "Hide tool names too" (`Firepixel_TooltipsHideNames`): no tooltip at all.
//! - The delay before a tooltip opens is `Firepixel_TooltipDelayMs` (default 600 ms), applied
//!   from the next frame on.
//!
//! Widgets attach their tooltip with [`HoverTip::hover_tip`] instead of `on_hover_text`, so the
//! first-line rule holds everywhere.

use egui::{Context, Id, Response, WidgetText};
use fp_engine::Preferences;

/// Preference: with View ▸ Tooltips off, hide the tool names too (no tooltip at all).
pub const HIDE_NAMES_PREF: &str = "Firepixel_TooltipsHideNames";
/// Preference: delay before a tooltip opens, in milliseconds.
pub const DELAY_PREF: &str = "Firepixel_TooltipDelayMs";
/// Default delay (the 0.6 s Firepixel has always used).
pub const DEFAULT_DELAY_MS: i64 = 600;
/// Largest delay the preference accepts.
pub const MAX_DELAY_MS: i64 = 5000;

/// What a tooltip shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TipMode {
    /// The whole text.
    Full,
    /// Only the first line (name and shortcut).
    NamesOnly,
    /// Nothing.
    Off,
}

impl TipMode {
    /// Mode for the state of View ▸ Tooltips and the "hide names too" option.
    pub fn from_switches(tooltips: bool, hide_names: bool) -> TipMode {
        match (tooltips, hide_names) {
            (true, _) => TipMode::Full,
            (false, false) => TipMode::NamesOnly,
            (false, true) => TipMode::Off,
        }
    }

    /// The part of `text` shown in this mode (`None` = no tooltip).
    pub fn shown(self, text: &str) -> Option<&str> {
        match self {
            TipMode::Full => Some(text),
            TipMode::NamesOnly => Some(text.split('\n').next().unwrap_or_default()),
            TipMode::Off => None,
        }
    }
}

/// Tooltip delay from the preferences, clamped to 0–5000 ms.
pub fn delay_ms(prefs: &Preferences) -> i64 {
    prefs
        .get_int(DELAY_PREF, DEFAULT_DELAY_MS)
        .clamp(0, MAX_DELAY_MS)
}

const MODE_KEY: &str = "fp-tooltip-mode";
/// Last timing written to the style (`off`, delay in ms).
const TIMING_KEY: &str = "fp-tooltip-timing";

/// Current mode (set every frame by [`apply`]; `Full` before the first frame).
pub fn mode(ctx: &Context) -> TipMode {
    ctx.data(|d| d.get_temp::<TipMode>(Id::new(MODE_KEY)))
        .unwrap_or(TipMode::Full)
}

/// Applies View ▸ Tooltips and the two preferences: stores the mode for [`HoverTip`] and sets the
/// egui tooltip timing - an infinite delay and no grace time when nothing may show (otherwise a
/// tooltip shown a moment ago lets the next one open at once), else the preference's delay. The
/// style is written only when the wanted timing changes, so a delay set elsewhere (tests) stays.
pub fn apply(ctx: &Context, prefs: &Preferences, tooltips: bool) {
    let m = TipMode::from_switches(tooltips, prefs.get_bool(HIDE_NAMES_PREF, false));
    let ms = delay_ms(prefs);
    let want = (m == TipMode::Off, ms);
    let last = ctx.data_mut(|d| {
        d.insert_temp(Id::new(MODE_KEY), m);
        d.get_temp::<(bool, i64)>(Id::new(TIMING_KEY))
    });
    if last == Some(want) {
        return;
    }
    ctx.data_mut(|d| d.insert_temp(Id::new(TIMING_KEY), want));
    let grace = egui::style::Interaction::default().tooltip_grace_time;
    ctx.all_styles_mut(|s| {
        if want.0 {
            s.interaction.tooltip_delay = f32::INFINITY;
            s.interaction.tooltip_grace_time = 0.0;
        } else {
            s.interaction.tooltip_delay = ms as f32 / 1000.0;
            s.interaction.tooltip_grace_time = grace;
        }
    });
}

/// Makes the next [`apply`] write the timing again (after the theme rebuilt the style).
pub fn invalidate(ctx: &Context) {
    ctx.data_mut(|d| d.remove::<(bool, i64)>(Id::new(TIMING_KEY)));
}

/// `on_hover_text` that follows View ▸ Tooltips: with the switch off only the first line shows.
pub trait HoverTip {
    fn hover_tip(self, text: impl Into<WidgetText>) -> Response;
}

impl HoverTip for Response {
    fn hover_tip(self, text: impl Into<WidgetText>) -> Response {
        match mode(&self.ctx) {
            TipMode::NamesOnly => {
                let w: WidgetText = text.into();
                match w.text().split_once('\n') {
                    Some((first, _)) => {
                        let first = first.to_string();
                        self.on_hover_text(first)
                    }
                    None => self.on_hover_text(w),
                }
            }
            // `Off` is an infinite delay; the text is attached as usual
            _ => self.on_hover_text(text),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_and_shown_text() {
        assert_eq!(TipMode::from_switches(true, true), TipMode::Full);
        assert_eq!(TipMode::from_switches(false, false), TipMode::NamesOnly);
        assert_eq!(TipMode::from_switches(false, true), TipMode::Off);
        let t = "Pointer (V, 0)\nSelects objects.";
        assert_eq!(TipMode::Full.shown(t), Some(t));
        assert_eq!(TipMode::NamesOnly.shown(t), Some("Pointer (V, 0)"));
        assert_eq!(TipMode::NamesOnly.shown("Size"), Some("Size"));
        assert_eq!(TipMode::Off.shown(t), None);
    }

    #[test]
    fn delay_is_clamped() {
        let mut p = Preferences::new();
        assert_eq!(delay_ms(&p), DEFAULT_DELAY_MS);
        p.set(DELAY_PREF, fp_engine::PrefValue::Int(9000));
        assert_eq!(delay_ms(&p), MAX_DELAY_MS);
        p.set(DELAY_PREF, fp_engine::PrefValue::Int(-5));
        assert_eq!(delay_ms(&p), 0);
    }
}
