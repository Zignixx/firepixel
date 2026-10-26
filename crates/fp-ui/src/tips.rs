//! Tooltip switches (FB-37, FB-98, FB-99): View ▸ Tooltips and the two options under
//! Preferences ▸ General.
//!
//! - Tooltips on: every tooltip in full ("Pointer (V, 0)" plus the description line).
//! - Tooltips off: only the name line stays ("Pointer (V, 0)"); the description lines go.
//!   A tooltip of one line (a panel button, an inspector control) stays as it is; a tooltip of
//!   several lines whose first line is not a name line goes completely (FB-99).
//! - Tooltips off with "Hide tool names too" (`Firepixel_TooltipsHideNames`): no tooltip at all.
//! - The delay before a tooltip opens is `Firepixel_TooltipDelayMs` (default 600 ms), applied
//!   from the next frame on.
//! - Explanatory tooltips (why a history step has a red marker, why a button is disabled,
//!   warnings) are not tool descriptions: they show in every mode (FB-99).
//!
//! Widgets attach their tooltip with [`HoverTip`] instead of `on_hover_text`:
//! - [`HoverTip::hover_tip`]: a description; with several lines the first counts as the name
//!   line only when it reads "Name (keys)" ([`is_name_line`]).
//! - [`HoverTip::name_tip`]: the first line is the name (tools built from the tool table).
//! - [`HoverTip::explain_tip`] / [`HoverTip::disabled_explain_tip`]: explanations, never hidden.

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

/// Kind of a tooltip text: how the modes treat it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TipKind {
    /// A description; with several lines the first is the name only if it reads like one.
    Describe,
    /// The first line is the name (and shortcut) of a tool or button.
    Named,
    /// An explanation or warning: shown in every mode.
    Explain,
}

/// Whether `line` reads "Name (keys)": a name, then in brackets one or more shortcuts separated
/// by ", ", each without spaces ("Pointer (V, 0)", "Undo (Ctrl+Z)").
pub fn is_name_line(line: &str) -> bool {
    let Some(inner) = line.trim().strip_suffix(')') else {
        return false;
    };
    let Some((name, keys)) = inner.rsplit_once(" (") else {
        return false;
    };
    !name.trim().is_empty()
        && keys
            .split(", ")
            .all(|k| !k.is_empty() && k.chars().count() <= 20 && !k.contains(char::is_whitespace))
}

/// What a tooltip shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TipMode {
    /// The whole text.
    Full,
    /// Only the name line (name and shortcut).
    NamesOnly,
    /// Nothing (except explanations).
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

    /// The part of a description `text` shown in this mode (`None` = no tooltip).
    pub fn shown(self, text: &str) -> Option<&str> {
        self.shown_as(text, TipKind::Describe)
    }

    /// The part of `text` of the given kind shown in this mode (`None` = no tooltip).
    pub fn shown_as(self, text: &str, kind: TipKind) -> Option<&str> {
        match (self, kind) {
            (_, TipKind::Explain) | (TipMode::Full, _) => Some(text),
            (TipMode::Off, _) => None,
            (TipMode::NamesOnly, kind) => match text.split_once('\n') {
                None => Some(text),
                Some((first, _)) if kind == TipKind::Named || is_name_line(first) => Some(first),
                Some(_) => None,
            },
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
/// Last delay written to the style (ms).
const TIMING_KEY: &str = "fp-tooltip-timing";

/// Current mode (set every frame by [`apply`]; `Full` before the first frame).
pub fn mode(ctx: &Context) -> TipMode {
    ctx.data(|d| d.get_temp::<TipMode>(Id::new(MODE_KEY)))
        .unwrap_or(TipMode::Full)
}

/// Applies View ▸ Tooltips and the two preferences: stores the mode for [`HoverTip`] and sets the
/// egui tooltip delay from the preference. The delay stays finite in every mode: a tooltip that
/// may not show is simply not attached, so explanations still open (FB-99). The style is written
/// only when the wanted delay changes, so a delay set elsewhere (tests) stays.
pub fn apply(ctx: &Context, prefs: &Preferences, tooltips: bool) {
    let m = TipMode::from_switches(tooltips, prefs.get_bool(HIDE_NAMES_PREF, false));
    let ms = delay_ms(prefs);
    let last = ctx.data_mut(|d| {
        d.insert_temp(Id::new(MODE_KEY), m);
        d.get_temp::<i64>(Id::new(TIMING_KEY))
    });
    if last == Some(ms) {
        return;
    }
    ctx.data_mut(|d| d.insert_temp(Id::new(TIMING_KEY), ms));
    ctx.all_styles_mut(|s| s.interaction.tooltip_delay = ms as f32 / 1000.0);
}

/// Makes the next [`apply`] write the timing again (after the theme rebuilt the style).
pub fn invalidate(ctx: &Context) {
    ctx.data_mut(|d| d.remove::<i64>(Id::new(TIMING_KEY)));
}

/// `on_hover_text` that follows View ▸ Tooltips (see the module documentation).
pub trait HoverTip {
    /// A description: with the tooltips off only a single line or a name line "Name (keys)"
    /// stays.
    fn hover_tip(self, text: impl Into<WidgetText>) -> Response;
    /// Name (and shortcut) on the first line, description below: the name stays with the
    /// tooltips off.
    fn name_tip(self, text: impl Into<WidgetText>) -> Response;
    /// An explanation or warning: shown whatever View ▸ Tooltips says.
    fn explain_tip(self, text: impl Into<WidgetText>) -> Response;
    /// Why the widget is disabled (`on_disabled_hover_text`): an explanation, always shown.
    fn disabled_explain_tip(self, text: impl Into<WidgetText>) -> Response;
}

fn attach(r: Response, text: impl Into<WidgetText>, kind: TipKind) -> Response {
    let m = mode(&r.ctx);
    if m == TipMode::Full || kind == TipKind::Explain {
        return r.on_hover_text(text);
    }
    let w: WidgetText = text.into();
    let whole = w.text().len();
    match m.shown_as(w.text(), kind).map(str::to_string) {
        Some(s) if s.len() == whole => r.on_hover_text(w),
        Some(s) => r.on_hover_text(s),
        None => r,
    }
}

impl HoverTip for Response {
    fn hover_tip(self, text: impl Into<WidgetText>) -> Response {
        attach(self, text, TipKind::Describe)
    }

    fn name_tip(self, text: impl Into<WidgetText>) -> Response {
        attach(self, text, TipKind::Named)
    }

    fn explain_tip(self, text: impl Into<WidgetText>) -> Response {
        attach(self, text, TipKind::Explain)
    }

    fn disabled_explain_tip(self, text: impl Into<WidgetText>) -> Response {
        self.on_disabled_hover_text(text)
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
        // FB-99: a description of several lines has no name line to keep
        let d = "Smooths the edges.\nWorks on bitmaps only.";
        assert_eq!(TipMode::NamesOnly.shown(d), None);
        assert_eq!(
            TipMode::NamesOnly.shown_as(d, TipKind::Named),
            Some("Smooths the edges.")
        );
        assert_eq!(TipMode::Full.shown_as(d, TipKind::Named), Some(d));
        assert_eq!(TipMode::Off.shown_as(t, TipKind::Named), None);
        // explanations show in every mode
        for m in [TipMode::Full, TipMode::NamesOnly, TipMode::Off] {
            assert_eq!(m.shown_as(d, TipKind::Explain), Some(d));
        }
    }

    #[test]
    fn name_line_pattern() {
        for yes in [
            "Pointer (V, 0)",
            "Undo (Ctrl+Z)",
            "Verwischen (R)",
            "Zoom (Strg+Umschalt+Z)",
        ] {
            assert!(is_name_line(yes), "{yes}");
        }
        for no in [
            "Selects objects.",
            "Paints (with the brush colour)",
            "(V)",
            "Pointer ()",
            "Pointer",
        ] {
            assert!(!is_name_line(no), "{no}");
        }
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
