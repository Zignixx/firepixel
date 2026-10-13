//! Tastatur-Weiterleitung: Menükürzel (aktives Tastenkürzel-Set von fp-engine), Werkzeugkürzel
//! (ohne Modifikatoren, mehrfaches Drücken schaltet im Flyout durch), Esc/Eingabe/Text an das
//! aktive Werkzeug, Leertaste = vorübergehend Hand.

use crate::app_state::{Action, AppState};
use egui::{Event, Key};
use fp_engine::tools::{EditOp, Modifiers, ToolInput, ToolKey};
use fp_engine::{Shortcut, ToolEvent, ToolId};
use serde_json::json;

/// Windows-Virtual-Key einer egui-Taste (für die Tastenkürzel-Sets des Originals).
pub fn vk_of(key: Key) -> Option<u16> {
    use Key::*;
    let k = match key {
        A => 0x41,
        B => 0x42,
        C => 0x43,
        D => 0x44,
        E => 0x45,
        F => 0x46,
        G => 0x47,
        H => 0x48,
        I => 0x49,
        J => 0x4A,
        K => 0x4B,
        L => 0x4C,
        M => 0x4D,
        N => 0x4E,
        O => 0x4F,
        P => 0x50,
        Q => 0x51,
        R => 0x52,
        S => 0x53,
        T => 0x54,
        U => 0x55,
        V => 0x56,
        W => 0x57,
        X => 0x58,
        Y => 0x59,
        Z => 0x5A,
        Num0 => 0x30,
        Num1 => 0x31,
        Num2 => 0x32,
        Num3 => 0x33,
        Num4 => 0x34,
        Num5 => 0x35,
        Num6 => 0x36,
        Num7 => 0x37,
        Num8 => 0x38,
        Num9 => 0x39,
        F1 => 0x70,
        F2 => 0x71,
        F3 => 0x72,
        F4 => 0x73,
        F5 => 0x74,
        F6 => 0x75,
        F7 => 0x76,
        F8 => 0x77,
        F9 => 0x78,
        F10 => 0x79,
        F11 => 0x7A,
        F12 => 0x7B,
        ArrowLeft => 0x25,
        ArrowUp => 0x26,
        ArrowRight => 0x27,
        ArrowDown => 0x28,
        Delete => 0x2E,
        Backspace => 0x08,
        Tab => 0x09,
        Enter => 0x0D,
        Escape => 0x1B,
        Space => 0x20,
        PageUp => 0x21,
        PageDown => 0x22,
        End => 0x23,
        Home => 0x24,
        Insert => 0x2D,
        Plus | Equals => 0xBB,
        Minus => 0xBD,
        Comma => 0xBC,
        Period => 0xBE,
        Semicolon => 0xBA,
        Slash => 0xBF,
        Backtick => 0xDC,
        Backslash => 0xDC,
        OpenBracket => 0xDB,
        CloseBracket => 0xDD,
        Quote => 0xDE,
        _ => return None,
    };
    Some(k)
}

/// Tasten, die das aktive Werkzeug selbst auswertet (Pfeile verschieben, Entf löscht Punkte,
/// Esc/Eingabe beenden …).
fn tool_key(key: Key) -> Option<ToolKey> {
    Some(match key {
        Key::Escape => ToolKey::Escape,
        Key::Enter => ToolKey::Enter,
        Key::ArrowUp => ToolKey::Up,
        Key::ArrowDown => ToolKey::Down,
        Key::ArrowLeft => ToolKey::Left,
        Key::ArrowRight => ToolKey::Right,
        Key::Backspace => ToolKey::Backspace,
        Key::Delete => ToolKey::Delete,
        Key::Tab => ToolKey::Tab,
        Key::Home => ToolKey::Home,
        Key::End => ToolKey::End,
        _ => return None,
    })
}

fn tool_mods(m: egui::Modifiers) -> Modifiers {
    Modifiers { shift: m.shift, alt: m.alt, ctrl: m.ctrl || m.command, space: false, caps: false }
}

/// Ereignis an das Werkzeug geben und die Antwort auswerten (Cursor, Statuszeile, Zwischenablage).
/// Liefert, ob das Werkzeug etwas getan hat (Befehle, Vorschau, Werkzeugwechsel).
fn send(ctx: &egui::Context, st: &mut AppState, input: ToolInput, mods: Modifiers) -> bool {
    let ev = ToolEvent { input, mods, pressure: 1.0, time_ms: (ctx.input(|i| i.time) * 1000.0) as u64 };
    match st.session.tool_event(&ev) {
        Ok(r) => {
            st.tool_cursor = r.cursor;
            if let Some(s) = r.status {
                st.push(Action::Status(s));
            }
            let acted = !r.commands.is_empty() || r.switch_tool.is_some() || r.clipboard.is_some() || !r.overlay.is_empty();
            if let Some(c) = r.clipboard {
                ctx.copy_text(c);
            }
            let acted = acted || r.ui_request.is_some();
            if let Some(req) = r.ui_request {
                crate::canvas::ui_request(st, &req);
                // Eingabe nicht an den eben geöffneten Dialog weitergeben
                ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Enter));
            }
            acted
        }
        Err(e) => {
            st.push(Action::Status(e.to_string()));
            true
        }
    }
}

/// Text-Editor auf der Leinwand: Pos1/Ende/Pfeil auf/ab zeilengenau über das Textlayout.
fn text_caret_key(st: &AppState, key: Key, m: egui::Modifiers) -> Option<ToolInput> {
    if m.ctrl || m.command {
        return None;
    }
    let id = st.session.active_id()?;
    let lc = st.canvases.get(&id)?.text.as_ref()?;
    let v = st.canvases.get(&id)?.feedback.iter().find_map(|o| match o {
        fp_engine::Overlay::TextEdit(v) => Some(v.clone()),
        _ => None,
    })?;
    let index = match key {
        Key::Home => lc.line_edge(v.caret, false, &v.text),
        Key::End => lc.line_edge(v.caret, true, &v.text),
        Key::ArrowUp => lc.vertical(v.caret, -1),
        Key::ArrowDown => lc.vertical(v.caret, 1),
        _ => return None,
    };
    Some(ToolInput::TextCaret { index, extend: m.shift })
}

/// Menübefehl zum Kürzel. egui liefert für den Ziffernblock dieselben Tasten wie für die
/// Ziffernreihe: ohne Treffer zusätzlich `VK_NUMPAD0–9` bzw. `VK_ADD`/`VK_SUBTRACT` probieren.
pub(crate) fn command_for_key(st: &AppState, key: Key, sc: &Shortcut) -> Option<String> {
    if let Some(c) = st.session.command_for_shortcut(sc) {
        return Some(c);
    }
    let num = match key {
        Key::Num0 => Some(0x60),
        Key::Num1 => Some(0x61),
        Key::Num2 => Some(0x62),
        Key::Num3 => Some(0x63),
        Key::Num4 => Some(0x64),
        Key::Num5 => Some(0x65),
        Key::Num6 => Some(0x66),
        Key::Num7 => Some(0x67),
        Key::Num8 => Some(0x68),
        Key::Num9 => Some(0x69),
        Key::Plus | Key::Equals => Some(0x6B),
        Key::Minus => Some(0x6D),
        _ => None,
    }?;
    st.session.command_for_shortcut(&Shortcut { vk: num, ..*sc })
}

fn set_paste_seen(ctx: &egui::Context, v: bool) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("lw-paste-seen"), v));
}

fn take_paste_seen(ctx: &egui::Context) -> bool {
    let v = ctx.data(|d| d.get_temp::<bool>(egui::Id::new("lw-paste-seen"))).unwrap_or(false);
    set_paste_seen(ctx, false);
    v
}

/// Registrierter, umgesetzter Befehl, der gerade gesperrt ist (Voraussetzungen fehlen).
fn blocked(st: &AppState, name: &str) -> bool {
    st.session.registry().get(name).is_some_and(|c| c.info().implemented) && !st.session.is_available(name)
}

/// Strg+C/X/V kommen von egui-winit auch mit Umschalt/Alt nur als `Copy`/`Cut`/`Paste` an. Mit
/// Strg **und** Umschalt bzw. Alt gilt daher das Kürzel des aktiven Satzes (Innen einfügen
/// Strg+Umschalt+V, Als Maske einfügen Strg+Alt+V, Attribute einfügen, HTML-Code kopieren …); nur
/// ohne Treffer bzw. ohne Strg (Umschalt+Entf, Strg+Einfg, Umschalt+Einfg) das einfache
/// Kopieren/Ausschneiden/Einfügen. Nicht verfügbare Befehle bleiben stumm.
fn run_clip(st: &mut AppState, key: Key, m: egui::Modifiers, default: &str, params: serde_json::Value) {
    let ctrl = m.ctrl || m.command;
    if ctrl && (m.shift || m.alt)
        && let Some(vk) = vk_of(key)
    {
        let sc = Shortcut { ctrl: true, alt: m.alt, shift: m.shift, vk };
        if let Some(cmd) = command_for_key(st, key, &sc) {
            if !blocked(st, &cmd) {
                st.push(Action::Run(cmd, json!({})));
            }
            return;
        }
    }
    // Einfügen auch ohne internen Inhalt (fremdes Bild/Text in der Systemablage)
    if default == "edit.paste" || !blocked(st, default) {
        st.push(Action::Run(default.into(), params));
    }
}

/// Verarbeitet die Tastatureingaben dieses Frames.
pub fn handle(ctx: &egui::Context, st: &mut AppState) {
    let editing = st.session.tool_is_editing_text();
    st.space_down = ctx.input(|i| i.key_down(Key::Space)) && !ctx.text_edit_focused() && !editing;
    // offenes Farbwähler-Popup: Tippen ohne Feldklick wird ignoriert, Esc/Enter gehören dem Popup
    // (Original, re\22 §2.3; FARB-1b)
    if ctx.text_edit_focused() || st.dialogs.any_open() || crate::inspector::color_picker::any_open(ctx) {
        return;
    }
    let events = ctx.input(|i| i.events.clone());
    for ev in events {
        let editing = st.session.tool_is_editing_text();
        match ev {
            Event::Key { key, pressed: true, repeat, modifiers, .. } => {
                if key == Key::Space {
                    continue;
                }
                if key == Key::Escape && st.flyout.is_some() {
                    st.flyout = None;
                    continue;
                }
                // Esc in einem offenen Popup (Popup-Schieber, Kombifeld-Liste, Farbfeld-Popup)
                // schließt nur das Popup (egui erledigt das selbst), wie im Original; die Auswahl
                // auf der Leinwand bleibt
                if key == Key::Escape && (egui::Popup::is_any_open(ctx) || crate::inspector::popups::fill_popup_open(ctx)) {
                    continue;
                }
                let ctrl = modifiers.ctrl || modifiers.command;
                if editing {
                    // Text-Editor: Tasten gehen an das Werkzeug; Strg+A markiert alles
                    if ctrl && key == Key::A {
                        send(ctx, st, ToolInput::Edit(EditOp::SelectAll), tool_mods(modifiers));
                        continue;
                    }
                    // Strg+Z / Strg+Y bzw. Umschalt+Strg+Z: zuerst im Editor rückgängig/wiederholen
                    if ctrl && (key == Key::Z || key == Key::Y) {
                        let redo = key == Key::Y || modifiers.shift;
                        send(ctx, st, ToolInput::Edit(if redo { EditOp::Redo } else { EditOp::Undo }), tool_mods(modifiers));
                        continue;
                    }
                    if let Some(inp) = text_caret_key(st, key, modifiers) {
                        send(ctx, st, inp, tool_mods(modifiers));
                        continue;
                    }
                    if let Some(tk) = tool_key(key) {
                        send(ctx, st, ToolInput::Key(tk), tool_mods(modifiers));
                        continue;
                    }
                    // übrige Kürzel nur mit Strg (Buchstaben sind Texteingabe); Strg+Alt = AltGr
                    // unter Windows (@, €, { …) ist ebenfalls Texteingabe
                    if !ctrl || modifiers.alt {
                        continue;
                    }
                }
                // Werkzeugtasten (Pfeile, Entf, Esc, Eingabe) zuerst an das Werkzeug; nur wenn es
                // nichts tut, an die Menükürzel (z. B. Pfeil = Verschieben ohne Auswahl)
                if let Some(tk) = tool_key(key)
                    && !ctrl
                    && send(ctx, st, ToolInput::Key(tk), tool_mods(modifiers))
                {
                    continue;
                }
                let Some(vk) = vk_of(key) else { continue };
                let sc = Shortcut { ctrl, alt: modifiers.alt, shift: modifiers.shift, vk };
                if let Some(cmd) = command_for_key(st, key, &sc) {
                    // gesperrte Befehle tun wie im Original stumm nichts (keine Meldung)
                    if !blocked(st, &cmd) {
                        st.push(Action::Run(cmd, json!({})));
                    }
                    continue;
                }
                if !sc.has_modifiers() && !repeat && !editing {
                    match st.session.tool_shortcut(&sc) {
                        Ok(Some(_)) => {}
                        // Pseudo-Werkzeuge der Kürzelsätze (D, X, F, 2 …) lösen Befehle aus
                        Ok(None) => {
                            if let Some(cmd) = fp_engine::tools::pseudo_command_for_key(st.session.shortcut_set(), &sc) {
                                st.push(Action::Run(cmd.into(), json!({})));
                            }
                        }
                        Err(e) => st.push(Action::Status(e.to_string())),
                    }
                }
            }
            Event::Copy if editing => {
                send(ctx, st, ToolInput::Edit(EditOp::Copy), Modifiers::default());
            }
            Event::Cut if editing => {
                send(ctx, st, ToolInput::Edit(EditOp::Cut), Modifiers::default());
            }
            Event::Paste(s) if editing => {
                set_paste_seen(ctx, true);
                send(ctx, st, ToolInput::Edit(EditOp::Paste(s)), Modifiers::default());
            }
            Event::Copy => {
                let m = ctx.input(|i| i.modifiers);
                run_clip(st, Key::C, m, "edit.copy", json!({}));
            }
            Event::Cut => {
                let m = ctx.input(|i| i.modifiers);
                run_clip(st, Key::X, m, "edit.cut", json!({}));
            }
            Event::Paste(s) => {
                // Text in der Systemzwischenablage: `actions` entscheidet (fremder Text → Textobjekt
                // wie im Original, sonst eigene Objekte/Pixel bzw. fremdes Bild)
                set_paste_seen(ctx, true);
                let m = ctx.input(|i| i.modifiers);
                run_clip(st, Key::V, m, "edit.paste", json!({ "osText": s }));
            }
            // egui-winit schickt Strg+V nur als `Event::Paste`, wenn die Systemzwischenablage TEXT
            // enthält – nach dem Kopieren von Objekten liegt dort aber ein Bild, dann kommt beim
            // Drücken gar nichts. Das Loslassen von V kommt dagegen immer an: ohne vorheriges
            // Paste-Ereignis hier einfügen.
            Event::Key { key: Key::V, pressed: false, modifiers, .. } if modifiers.ctrl || modifiers.command => {
                if !take_paste_seen(ctx) {
                    if editing {
                        // Text-Editor: nur Text einfügen (ohne Text in der Ablage nichts)
                    } else {
                        run_clip(st, Key::V, modifiers, "edit.paste", json!({}));
                    }
                }
            }
            Event::Text(t) if editing || st.session.tools.active == ToolId::Text => {
                send(ctx, st, ToolInput::Text(t), Modifiers::default());
            }
            Event::Ime(egui::ImeEvent::Preedit { text, .. }) if editing => {
                send(ctx, st, ToolInput::Preedit(text), Modifiers::default());
            }
            Event::Ime(egui::ImeEvent::Commit(t)) if editing => {
                send(ctx, st, ToolInput::Preedit(String::new()), Modifiers::default());
                send(ctx, st, ToolInput::Text(t), Modifiers::default());
            }
            _ => {}
        }
    }
    // Rechteck aufziehen mit gehaltener ↑/↓-Taste: die Rundung wächst, solange die Taste gehalten
    // wird (PAR 10/05, Original 1,5 s → 0,97) – auch ohne Tastenwiederholung des Systems je Bild
    if st.session.tools.active == ToolId::Rectangle && ctx.input(|i| i.pointer.primary_down()) {
        let (up, down, m) = ctx.input(|i| (i.key_down(Key::ArrowUp), i.key_down(Key::ArrowDown), i.modifiers));
        if up != down {
            send(ctx, st, ToolInput::Key(if up { ToolKey::Up } else { ToolKey::Down }), tool_mods(m));
            ctx.request_repaint();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_and_function_keys_map_to_vk() {
        assert_eq!(vk_of(Key::V), Some(0x56));
        assert_eq!(vk_of(Key::F8), Some(0x77));
        assert_eq!(vk_of(Key::Num0), Some(0x30));
    }

    #[test]
    fn ctrl_n_is_file_new_in_default_set() {
        let s = fp_engine::Session::default();
        let sc = Shortcut { ctrl: true, alt: false, shift: false, vk: 0x4E };
        assert_eq!(s.command_for_shortcut(&sc).as_deref(), Some("file.new"));
    }
}
