//! Eingabe-Einspeisung (`ui_input`) und Widget-Suche (`ui_find`) für den KI-Steuerkanal.
//!
//! Die Ereignisse laufen über ein egui-[`Plugin`](egui::Plugin): `input_hook` hängt sie vor
//! jedem Frame an die rohe Eingabe ([`egui::RawInput`]) an – derselbe Weg wie echte Maus- und
//! Tastatureingaben von egui-winit. Die Übersetzung bildet egui-winit unter Windows nach:
//! Strg+C/X/V (und Strg+Einfg, Umschalt+Entf/Einfg) werden beim Drücken zu
//! `Event::Copy`/`Cut`/`Paste` (Paste nur, wenn die Systemzwischenablage Text enthält),
//! Text kommt nur ohne Strg, Umschalttasten als `Event::ModifiersChanged`, Mausrad mit den
//! aktuellen Umschalttasten, Dateien als `dropped_files` (Zeigerposition erst danach, wie bei
//! OLE-Drag&Drop). Jeder Schritt bekommt eigene Frames; Wartezeiten und Bildschirmfotos
//! zwischen den Schritten sind möglich.
//!
//! `ui_find` schaltet für einen Frame den AccessKit-Baum von egui ein und durchsucht ihn
//! zusammen mit den gezeichneten Texten (`FullOutput::shapes`) im `output_hook`.

use egui::epaint::{ClippedShape, Shape};
use egui::{pos2, vec2, Event, Key, Modifiers, MouseWheelUnit, PointerButton, Pos2, Rect, TouchPhase};
use fp_mcp::{PendingUi, UiImage, UiReply};
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Kennungen der Bildschirmfotos aus Eingabefolgen (getrennt von `screenshot`).
const SHOT_BASE: u64 = 1 << 40;
/// Höchstzahl Bilder je `ui_input`.
const MAX_IMAGES: usize = 24;
/// Höchstzahl Frames je `ui_input`.
const MAX_STEPS: usize = 6000;
/// Höchstzahl gemeldeter Widget-Ereignisse.
const MAX_WIDGET_EVENTS: usize = 200;

/// Ein Ereignis innerhalb eines Frames (wird erst beim Einspeisen mit dem aktuellen Zustand
/// – Umschalttasten, Zeiger, Zwischenablage – zu egui-Ereignissen).
#[derive(Clone, Debug)]
enum Ev {
    /// Umschalttasten setzen (Grundzustand, ersetzt alle vorübergehenden).
    Mods(Modifiers),
    /// Umschalttasten vorübergehend dazu (für einen Schritt).
    PushMods(Modifiers),
    /// Vorübergehende Umschalttasten wieder lösen.
    PopMods,
    Move(Pos2),
    Button { button: PointerButton, pressed: bool },
    KeyDown(Key),
    KeyUp(Key),
    /// Zeichen tippen: Taste (falls vorhanden) + Text.
    TypeChar(char),
    Text(String),
    Wheel { unit: MouseWheelUnit, delta: egui::Vec2 },
    Copy,
    Cut,
    /// Einfügen mit festem Text (`None` = Text der Systemzwischenablage, leer → nichts).
    Paste(Option<String>),
    Zoom(f32),
    Gone,
    Focus(bool),
    Hover(Vec<PathBuf>),
    Drop(Vec<PathBuf>),
}

#[derive(Clone, Debug)]
enum Step {
    /// Ereignisse eines Frames.
    Frame(Vec<Ev>),
    /// Mindestens so lange warten (Millisekunden).
    Wait(u64),
    /// So viele Frames ohne Eingabe.
    Frames(u32),
    /// Bildschirmfoto (Ausschnitt in Punkten).
    Shot(Option<[f32; 4]>),
}

/// Eine laufende Eingabefolge (`ui_input`).
pub(crate) struct Script {
    steps: VecDeque<Step>,
    reply: Option<PendingUi>,
    labels: Vec<String>,
    settle: u32,
    final_shot: Option<Option<[f32; 4]>>,
    not_before: Option<Instant>,
    frames_left: u32,
    shot_request: Option<u64>,
    shot_wait: Option<(u64, Option<[f32; 4]>, u32)>,
    images: Vec<UiImage>,
    widget_events: Vec<Value>,
    copied: Vec<String>,
    idle_frames: u32,
    injected: usize,
    frames: u32,
    started: Option<Instant>,
}

struct FindReq {
    query: Value,
    reply: PendingUi,
    armed: bool,
}

struct Shared {
    scripts: VecDeque<Script>,
    finds: Vec<FindReq>,
    /// Gehaltene Umschalttasten (Grundzustand).
    base_mods: Modifiers,
    /// Vorübergehende Umschalttasten je Schritt (Stapel).
    mods_stack: Vec<Modifiers>,
    pointer: Option<Pos2>,
    next_shot: u64,
    accesskit_by_us: bool,
    /// Dateien, die gerade über dem Fenster schweben (`hover_files`): gelten bis `drop_files`,
    /// `hover_end` bzw. `pointer_gone` in jedem Frame – wie beim echten Ziehen (BATCH-T: vorher nur
    /// einen Frame, die Rückmeldung „Zum Hinzufügen loslassen“ war nicht zu fotografieren).
    hover: Vec<PathBuf>,
    /// AccessKit ist ohnehin an (Barrierefreiheit, Testumgebung) – dann nie abschalten.
    accesskit_external: bool,
}

impl Shared {
    fn mods(&self) -> Modifiers {
        self.mods_stack.iter().fold(self.base_mods, |a, b| a | *b)
    }
}

static SHARED: Mutex<Shared> = Mutex::new(Shared {
    scripts: VecDeque::new(),
    finds: Vec::new(),
    base_mods: Modifiers::NONE,
    mods_stack: Vec::new(),
    pointer: None,
    next_shot: SHOT_BASE,
    accesskit_by_us: false,
    hover: Vec::new(),
    accesskit_external: false,
});

fn shared() -> std::sync::MutexGuard<'static, Shared> {
    SHARED.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// egui-Plugin für Einspeisung und Suche (einmal je Kontext registrieren; doppelt wird ignoriert).
pub(crate) struct InputPlugin;

impl egui::Plugin for InputPlugin {
    fn debug_name(&self) -> &'static str {
        "Firepixel KI-Steuerkanal (Eingabe)"
    }

    fn input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        input_hook(ctx, raw);
    }

    fn output_hook(&mut self, ctx: &egui::Context, out: &mut egui::FullOutput) {
        output_hook(ctx, out);
    }
}

/// Registriert das Plugin.
pub(crate) fn install(ctx: &egui::Context) {
    ctx.add_plugin(InputPlugin);
}

/// Bricht alle laufenden Folgen und Suchen ab (Steuerkanal beendet).
pub(crate) fn cancel_all() {
    let (scripts, finds) = {
        let mut g = shared();
        g.mods_stack.clear();
        g.base_mods = Modifiers::NONE;
        (std::mem::take(&mut g.scripts), std::mem::take(&mut g.finds))
    };
    for mut s in scripts {
        if let Some(r) = s.reply.take() {
            r.respond(UiReply::Error(super::at("err-mcp-ui-channel-closed")));
        }
    }
    for f in finds {
        f.reply.respond(UiReply::Error(super::at("err-mcp-ui-channel-closed")));
    }
}

/// Nimmt eine `ui_input`-Anfrage an (Fehler im Aufbau → sofort beantwortet).
pub(crate) fn start_input(ctx: &egui::Context, args: &Value, reply: PendingUi) {
    match parse(args) {
        Ok(mut s) => {
            s.reply = Some(reply);
            shared().scripts.push_back(s);
            ctx.request_repaint();
        }
        Err(e) => reply.respond(UiReply::Error(e)),
    }
}

/// Nimmt eine `ui_find`-Anfrage an: AccessKit für den nächsten Frame einschalten.
pub(crate) fn start_find(ctx: &egui::Context, args: &Value, reply: PendingUi) {
    {
        let mut g = shared();
        if !g.accesskit_external {
            g.accesskit_by_us = true;
        }
        g.finds.push(FindReq { query: args.clone(), reply, armed: false });
    }
    ctx.enable_accesskit();
    ctx.request_repaint();
}

/// Je Frame aus [`super::UiBridge::process`]: Bildschirmfotos anfordern, fertige Folgen
/// beantworten (mit Dialog-/Statuszustand der Oberfläche).
pub(crate) fn frame(ctx: &egui::Context, dialog_open: bool, status: Option<String>) {
    let mut done: Option<Script> = None;
    {
        let mut g = shared();
        let pointer = g.pointer;
        let Some(s) = g.scripts.front_mut() else { return };
        s.frames += 1;
        if let Some(id) = s.shot_request.take() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(id)));
        }
        if let Some(w) = &mut s.shot_wait {
            w.2 += 1;
            if w.2 > 120 {
                s.shot_wait = None;
                s.labels.push(super::at("err-mcp-ui-no-screenshot"));
            }
        }
        let idle = s.steps.is_empty() && s.not_before.is_none() && s.frames_left == 0 && s.shot_wait.is_none() && s.shot_request.is_none();
        if idle {
            if s.idle_frames >= s.settle {
                if let Some(rect) = s.final_shot.take() {
                    s.steps.push_back(Step::Shot(rect));
                } else {
                    done = g.scripts.pop_front();
                }
            } else {
                s.idle_frames += 1;
            }
        }
        if let Some(s) = &mut done {
            let mods = g.mods();
            let res = json!({
                "ok": true,
                "steps": s.labels,
                "frames": s.frames,
                "events_injected": s.injected,
                "seconds": s.started.map(|t| (t.elapsed().as_secs_f64() * 1000.0).round() / 1000.0),
                "pointer": pointer.map(|p| [p.x, p.y]),
                "held_modifiers": mods_json(mods),
                "widget_events": s.widget_events,
                "copied_text": s.copied,
                "dialog_open": dialog_open,
                "status": status,
                "images": s.images.len(),
            });
            if let Some(r) = s.reply.take() {
                let images = std::mem::take(&mut s.images);
                r.respond(if images.is_empty() { UiReply::Json(res) } else { UiReply::JsonImages(res, images) });
            }
        }
    }
    if done.is_some() || !shared().scripts.is_empty() {
        ctx.request_repaint();
    }
}

fn mods_json(m: Modifiers) -> Vec<&'static str> {
    let mut v = Vec::new();
    if m.ctrl {
        v.push("ctrl");
    }
    if m.shift {
        v.push("shift");
    }
    if m.alt {
        v.push("alt");
    }
    v
}

// ------------------------------------------------------------------ Einspeisen

fn input_hook(ctx: &egui::Context, raw: &mut egui::RawInput) {
    let ppp = ctx.pixels_per_point();
    let mut g = shared();
    for f in &mut g.finds {
        f.armed = true;
    }
    let mut mods_stack = std::mem::take(&mut g.mods_stack);
    let mut base = g.base_mods;
    let mut pointer = g.pointer;
    let mut hover = std::mem::take(&mut g.hover);
    let mut injected = 0usize;
    let next_shot = g.next_shot;
    let mut used_shot = false;
    if let Some(s) = g.scripts.front_mut() {
        if s.started.is_none() {
            s.started = Some(Instant::now());
        }
        // eingetroffene Bildschirmfotos
        if let Some((id, rect, _)) = s.shot_wait {
            for e in &raw.events {
                if let Event::Screenshot { user_data, image, .. } = e
                    && user_data.data.as_ref().and_then(|d| d.downcast_ref::<u64>().copied()) == Some(id)
                {
                    if let UiReply::Image { width, height, rgba } = super::crop_image(image, rect, ppp) {
                        s.images.push(UiImage { width, height, rgba });
                    }
                    s.shot_wait = None;
                }
            }
        }
        let blocked = s.shot_wait.is_some() || s.shot_request.is_some() || s.not_before.is_some_and(|t| Instant::now() < t);
        if !blocked {
            s.not_before = None;
            if s.frames_left > 0 {
                s.frames_left -= 1;
            } else if let Some(step) = s.steps.pop_front() {
                match step {
                    Step::Frame(evs) => {
                        for ev in evs {
                            match &ev {
                                Ev::Hover(p) => hover = p.clone(),
                                Ev::Drop(_) | Ev::Gone => hover.clear(),
                                _ => {}
                            }
                            injected += apply(ev, raw, &mut base, &mut mods_stack, &mut pointer);
                        }
                        s.injected += injected;
                        s.idle_frames = 0;
                    }
                    Step::Wait(ms) => s.not_before = Some(Instant::now() + Duration::from_millis(ms)),
                    Step::Frames(n) => s.frames_left = n.saturating_sub(1),
                    Step::Shot(rect) => {
                        if s.images.len() < MAX_IMAGES {
                            s.shot_request = Some(next_shot);
                            s.shot_wait = Some((next_shot, rect, 0));
                            used_shot = true;
                        }
                    }
                }
            }
        }
    }
    if used_shot {
        g.next_shot += 1;
    }
    if !hover.is_empty() && raw.hovered_files.is_empty() && raw.dropped_files.is_empty() {
        for p in &hover {
            raw.hovered_files.push(egui::HoveredFile { path: Some(p.clone()), ..Default::default() });
        }
    }
    g.hover = hover;
    g.mods_stack = mods_stack;
    g.base_mods = base;
    g.pointer = pointer;
}

/// Wendet ein Ereignis auf die rohe Eingabe an (wie egui-winit); liefert die Zahl der Ereignisse.
fn apply(ev: Ev, raw: &mut egui::RawInput, base: &mut Modifiers, stack: &mut Vec<Modifiers>, pointer: &mut Option<Pos2>) -> usize {
    let mods = |base: &Modifiers, stack: &Vec<Modifiers>| stack.iter().fold(*base, |a, b| a | *b);
    let before = raw.events.len() + raw.dropped_files.len() + raw.hovered_files.len();
    match ev {
        Ev::Mods(m) => {
            *base = m;
            stack.clear();
            raw.events.push(Event::ModifiersChanged(m));
        }
        Ev::PushMods(m) => {
            stack.push(m);
            let now = mods(base, stack);
            raw.events.push(Event::ModifiersChanged(now));
        }
        Ev::PopMods => {
            stack.pop();
            raw.events.push(Event::ModifiersChanged(mods(base, stack)));
        }
        Ev::Move(p) => {
            *pointer = Some(p);
            raw.events.push(Event::PointerMoved(p));
        }
        Ev::Button { button, pressed } => {
            // egui-winit verwirft Klicks ohne bekannte Zeigerposition
            if let Some(pos) = *pointer {
                raw.events.push(Event::PointerButton { pos, button, pressed, modifiers: mods(base, stack) });
            }
        }
        Ev::KeyDown(key) => key_down(raw, key, mods(base, stack), None),
        Ev::KeyUp(key) => raw.events.push(Event::Key { key, physical_key: Some(key), pressed: false, repeat: false, modifiers: mods(base, stack) }),
        Ev::TypeChar(c) => {
            let key = key_for_char(c);
            match key {
                Some(k) => key_down(raw, k, mods(base, stack), Some(c)),
                None => {
                    let m = mods(base, stack);
                    if !m.ctrl && !m.command {
                        raw.events.push(Event::Text(c.to_string()));
                    }
                }
            }
        }
        Ev::Text(t) => raw.events.push(Event::Text(t)),
        Ev::Wheel { unit, delta } => raw.events.push(Event::MouseWheel { unit, delta, phase: TouchPhase::Move, modifiers: mods(base, stack) }),
        Ev::Copy => raw.events.push(Event::Copy),
        Ev::Cut => raw.events.push(Event::Cut),
        Ev::Paste(t) => {
            let text = t.or_else(clipboard_text).unwrap_or_default().replace("\r\n", "\n");
            if !text.is_empty() {
                raw.events.push(Event::Paste(text));
            }
        }
        Ev::Zoom(f) => raw.events.push(Event::Zoom(f)),
        Ev::Gone => {
            *pointer = None;
            raw.events.push(Event::PointerGone);
        }
        Ev::Focus(f) => {
            raw.focused = f;
            if !f {
                *base = Modifiers::NONE;
                stack.clear();
            }
            raw.events.push(Event::WindowFocused(f));
        }
        Ev::Hover(paths) => {
            for p in paths {
                raw.hovered_files.push(egui::HoveredFile { path: Some(p), ..Default::default() });
            }
        }
        Ev::Drop(paths) => {
            raw.hovered_files.clear();
            for p in paths {
                raw.dropped_files.push(Arc::new(InjectedFile(p)));
            }
        }
    }
    (raw.events.len() + raw.dropped_files.len() + raw.hovered_files.len()).saturating_sub(before).max(1)
}

/// Taste gedrückt – wie `egui_winit::State::on_keyboard_input` unter Windows.
fn key_down(raw: &mut egui::RawInput, key: Key, m: Modifiers, ch: Option<char>) {
    if key == Key::Cut || (m.command && key == Key::X) || (m.shift && key == Key::Delete) {
        raw.events.push(Event::Cut);
        return;
    }
    if key == Key::Copy || (m.command && key == Key::C) || (m.ctrl && key == Key::Insert) {
        raw.events.push(Event::Copy);
        return;
    }
    if key == Key::Paste || (m.command && key == Key::V) || (m.shift && key == Key::Insert) {
        // egui-winit liest die Systemzwischenablage; ohne Text kommt beim Drücken nichts
        if let Some(t) = clipboard_text() {
            let t = t.replace("\r\n", "\n");
            if !t.is_empty() {
                raw.events.push(Event::Paste(t));
            }
        }
        return;
    }
    raw.events.push(Event::Key { key, physical_key: Some(key), pressed: true, repeat: false, modifiers: m });
    let text = ch.map(String::from).or_else(|| key_text(key, m.shift));
    if let Some(t) = text
        && !m.ctrl
        && !m.command
        && t.chars().all(|c| !c.is_control())
    {
        raw.events.push(Event::Text(t));
    }
}

/// Text einer Taste ohne Zeichen-Angabe (Buchstaben, Ziffern, Leertaste).
fn key_text(key: Key, shift: bool) -> Option<String> {
    if key == Key::Space {
        return Some(" ".into());
    }
    let name = key.name();
    let mut it = name.chars();
    let c = it.next()?;
    if it.next().is_some() {
        return None;
    }
    if c.is_ascii_alphabetic() {
        Some(if shift { c.to_ascii_uppercase() } else { c.to_ascii_lowercase() }.to_string())
    } else if c.is_ascii_digit() {
        Some(c.to_string())
    } else {
        None
    }
}

fn key_for_char(c: char) -> Option<Key> {
    if c == ' ' {
        return Some(Key::Space);
    }
    if c == '\n' {
        return Some(Key::Enter);
    }
    if c == '\t' {
        return Some(Key::Tab);
    }
    Key::from_name(&c.to_ascii_uppercase().to_string())
}

/// Tests lesen die Systemzwischenablage nie.
static NO_OS_CLIPBOARD: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn clipboard_text() -> Option<String> {
    if NO_OS_CLIPBOARD.load(std::sync::atomic::Ordering::Relaxed) {
        return None;
    }
    arboard::Clipboard::new().ok()?.get_text().ok()
}

/// Eingespeiste Datei (wie `NativeFile` von egui-winit).
#[derive(Debug)]
struct InjectedFile(PathBuf);

impl egui::DroppedFile for InjectedFile {
    fn path(&self) -> &Path {
        &self.0
    }

    fn bytes(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.0).map_err(|e| e.to_string())
    }
}

// ------------------------------------------------------------------ Ausgabe: Widgets, Suche

fn output_hook(ctx: &egui::Context, out: &mut egui::FullOutput) {
    let mut g = shared();
    if out.platform_output.accesskit_update.is_some() && !g.accesskit_by_us {
        g.accesskit_external = true;
    }
    if let Some(s) = g.scripts.front_mut() {
        for e in &out.platform_output.events {
            if s.widget_events.len() < MAX_WIDGET_EVENTS {
                s.widget_events.push(describe_output_event(e));
            }
        }
        for c in &out.platform_output.commands {
            if let egui::OutputCommand::CopyText(t) = c {
                s.copied.push(t.clone());
            }
        }
    }
    let (armed, rest): (Vec<FindReq>, Vec<FindReq>) = std::mem::take(&mut g.finds).into_iter().partition(|f| f.armed);
    g.finds = rest;
    let disable = g.finds.is_empty() && g.accesskit_by_us && !g.accesskit_external && !armed.is_empty();
    if disable {
        g.accesskit_by_us = false;
    }
    let repaint_now = !g.finds.is_empty() || g.scripts.front().is_some_and(|s| s.not_before.is_none());
    let repaint_later = g.scripts.front().and_then(|s| s.not_before).map(|t| t.saturating_duration_since(Instant::now()));
    drop(g);
    for f in armed {
        let hits = find_hits(&f.query, out);
        let n = hits.len();
        f.reply.respond(UiReply::Json(json!({"count": n, "hits": hits})));
    }
    if disable {
        ctx.disable_accesskit();
    }
    if repaint_now {
        ctx.request_repaint();
    } else if let Some(d) = repaint_later {
        ctx.request_repaint_after(d);
    }
}

fn describe_output_event(e: &egui::output::OutputEvent) -> Value {
    use egui::output::OutputEvent as O;
    let (kind, info) = match e {
        O::Clicked(i) => ("clicked", i),
        O::DoubleClicked(i) => ("double_clicked", i),
        O::TripleClicked(i) => ("triple_clicked", i),
        O::FocusGained(i) => ("focus_gained", i),
        O::TextSelectionChanged(i) => ("text_selection_changed", i),
        O::ValueChanged(i) => ("value_changed", i),
    };
    json!({
        "event": kind, "widget": snake(&format!("{:?}", info.typ)), "label": info.label, "enabled": info.enabled,
        "text": info.current_text_value, "selected": info.selected, "value": info.value,
    })
}

/// `CheckBox` → `check_box`.
fn snake(s: &str) -> String {
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

struct Cand {
    text: String,
    role: String,
    source: &'static str,
    rect: Rect,
    order: usize,
    enabled: Option<bool>,
    toggled: Option<bool>,
}

fn norm(s: &str) -> String {
    s.replace("...", "…").replace('\u{a0}', " ").to_lowercase().trim().to_string()
}

fn collect_texts(shape: &Shape, clip: Rect, order: &mut usize, out: &mut Vec<Cand>) {
    match shape {
        Shape::Vec(v) => {
            for s in v {
                collect_texts(s, clip, order, out);
            }
        }
        Shape::Text(t) => {
            *order += 1;
            let text = t.galley.job.text.trim();
            if text.is_empty() || t.opacity_factor <= 0.0 {
                return;
            }
            let r = t.galley.rect.translate(t.pos.to_vec2());
            let vis = r.intersect(clip);
            if vis.width() <= 0.5 || vis.height() <= 0.5 {
                return;
            }
            out.push(Cand { text: text.to_string(), role: "text".into(), source: "text", rect: vis, order: *order, enabled: None, toggled: None });
        }
        _ => *order += 1,
    }
}

/// Sucht Treffer in AccessKit-Baum und gezeichneten Texten.
fn find_hits(q: &Value, out: &egui::FullOutput) -> Vec<Value> {
    let text = q.get("text").and_then(Value::as_str).map(norm).filter(|s| !s.is_empty());
    let role = q.get("role").and_then(Value::as_str).map(|r| snake(r).replace("__", "_").to_lowercase());
    let exact = q.get("exact").and_then(Value::as_bool).unwrap_or(false);
    let limit = q.get("limit").and_then(Value::as_u64).unwrap_or(50).clamp(1, 500) as usize;
    let region = q.get("region").and_then(Value::as_array).filter(|a| a.len() == 4).map(|a| {
        let f = |i: usize| a[i].as_f64().unwrap_or(0.0) as f32;
        Rect::from_min_max(pos2(f(0).min(f(2)), f(1).min(f(3))), pos2(f(0).max(f(2)), f(1).max(f(3))))
    });
    // gezeichnete Texte (Zeichenreihenfolge = oben liegt später)
    let mut texts = Vec::new();
    let mut order = 0usize;
    for ClippedShape { clip_rect, shape } in &out.shapes {
        collect_texts(shape, *clip_rect, &mut order, &mut texts);
    }
    // AccessKit-Knoten
    let mut nodes = Vec::new();
    if let Some(upd) = &out.platform_output.accesskit_update {
        for (_, node) in &upd.nodes {
            let Some(b) = node.bounds() else { continue };
            let r = Rect::from_min_max(pos2(b.x0 as f32, b.y0 as f32), pos2(b.x1 as f32, b.y1 as f32));
            if r.width() <= 0.0 || r.height() <= 0.0 {
                continue;
            }
            let label = node.label().or_else(|| node.value()).unwrap_or_default().trim().to_string();
            let role = snake(&format!("{:?}", node.role()));
            if label.is_empty() && matches!(role.as_str(), "unknown" | "generic_container" | "window" | "label") {
                continue;
            }
            // Zeichenreihenfolge: oberster Text innerhalb des Knotens
            let ord = texts.iter().filter(|t| r.expand(1.0).contains_rect(t.rect)).map(|t| t.order).max().unwrap_or(0);
            let toggled = node.toggled().map(|t| matches!(t, egui::accesskit::Toggled::True));
            nodes.push(Cand { text: label, role, source: "accesskit", rect: r, order: ord, enabled: Some(!node.is_disabled()), toggled });
        }
    }
    // Texte, die schon als Widget-Name vorkommen, nicht doppelt
    let texts: Vec<Cand> = texts
        .into_iter()
        .filter(|t| !nodes.iter().any(|n| norm(&n.text) == norm(&t.text) && n.rect.expand(2.0).contains_rect(t.rect)))
        .collect();
    let mut all: Vec<(u8, Cand)> = nodes
        .into_iter()
        .chain(texts)
        .filter_map(|c| {
            if let Some(reg) = region
                && !reg.intersects(c.rect)
            {
                return None;
            }
            if let Some(r) = &role
                && !(c.role == *r || (r == "menu_item" && c.role.starts_with("menu_item")) || (r == "text" && c.source == "text"))
            {
                return None;
            }
            let quality = match &text {
                None => 1,
                Some(t) => {
                    let n = norm(&c.text);
                    if n == *t {
                        0
                    } else if exact {
                        return None;
                    } else if n.starts_with(t.as_str()) {
                        1
                    } else if n.contains(t.as_str()) {
                        2
                    } else {
                        return None;
                    }
                }
            };
            Some((quality, c))
        })
        .collect();
    // beste Übereinstimmung, dann oben liegend, dann Widget vor Text
    all.sort_by(|(qa, a), (qb, b)| qa.cmp(qb).then(b.order.cmp(&a.order)).then((a.source == "text").cmp(&(b.source == "text"))));
    all.truncate(limit);
    let r1 = |v: f32| (v * 10.0).round() / 10.0;
    all.into_iter()
        .map(|(q, c)| {
            let mut v = json!({
                "text": c.text, "role": c.role, "source": c.source,
                "rect": [r1(c.rect.min.x), r1(c.rect.min.y), r1(c.rect.max.x), r1(c.rect.max.y)],
                "center": [r1(c.rect.center().x), r1(c.rect.center().y)],
                "exact": q == 0, "order": c.order,
            });
            if let Some(e) = c.enabled {
                v["enabled"] = json!(e);
            }
            if let Some(t) = c.toggled {
                v["toggled"] = json!(t);
            }
            v
        })
        .collect()
}

// ------------------------------------------------------------------ Aufbau aus JSON

fn pos_of(v: Option<&Value>) -> Option<Pos2> {
    let v = v?;
    if let Some(a) = v.as_array() {
        if a.len() >= 2 {
            return Some(pos2(a[0].as_f64()? as f32, a[1].as_f64()? as f32));
        }
        return None;
    }
    Some(pos2(v.get("x")?.as_f64()? as f32, v.get("y")?.as_f64()? as f32))
}

/// „<feld> fehlt“ in der Antwortsprache der Agenten.
fn missing(field: &str) -> String {
    super::agent_text(|| fp_i18n::tr!("err-mcp-param-missing", param = field))
}

fn mods_of(v: Option<&Value>) -> Result<Modifiers, String> {
    let mut m = Modifiers::NONE;
    let Some(v) = v else { return Ok(m) };
    let names: Vec<String> = match v {
        Value::Array(a) => a.iter().filter_map(|x| x.as_str().map(str::to_lowercase)).collect(),
        Value::String(s) => s.split(['+', ',', ' ']).filter(|x| !x.is_empty()).map(str::to_lowercase).collect(),
        Value::Object(o) => o.iter().filter(|(_, b)| b.as_bool() == Some(true)).map(|(k, _)| k.to_lowercase()).collect(),
        Value::Null => Vec::new(),
        _ => return Err(super::at("err-mcp-ui-modifiers")),
    };
    for n in names {
        match n.as_str() {
            "ctrl" | "strg" | "control" | "cmd" | "command" => {
                m.ctrl = true;
                m.command = true;
            }
            "shift" | "umschalt" => m.shift = true,
            "alt" => m.alt = true,
            other => return Err(super::agent_text(|| fp_i18n::tr!("err-mcp-ui-unknown-modifier", name = other))),
        }
    }
    Ok(m)
}

fn button_of(v: Option<&Value>) -> Result<PointerButton, String> {
    match v.and_then(Value::as_str).unwrap_or("left").to_lowercase().as_str() {
        "left" | "links" | "primary" => Ok(PointerButton::Primary),
        "right" | "rechts" | "secondary" => Ok(PointerButton::Secondary),
        "middle" | "mitte" => Ok(PointerButton::Middle),
        "back" | "extra1" => Ok(PointerButton::Extra1),
        "forward" | "extra2" => Ok(PointerButton::Extra2),
        other => Err(super::agent_text(|| fp_i18n::tr!("err-mcp-ui-unknown-button", name = other))),
    }
}

fn key_of(v: Option<&Value>) -> Result<Key, String> {
    let name = v.and_then(Value::as_str).ok_or_else(|| super::at("err-mcp-ui-key-missing"))?;
    let alias = match name.to_lowercase().as_str() {
        "esc" => "Escape".to_string(),
        "del" | "entf" => "Delete".to_string(),
        "ins" | "einfg" => "Insert".to_string(),
        "return" | "eingabe" => "Enter".to_string(),
        "left" => "ArrowLeft".to_string(),
        "right" => "ArrowRight".to_string(),
        "up" => "ArrowUp".to_string(),
        "down" => "ArrowDown".to_string(),
        "pgup" => "PageUp".to_string(),
        "pgdn" => "PageDown".to_string(),
        "backspace" | "rück" => "Backspace".to_string(),
        "space" | "leer" => "Space".to_string(),
        _ if name.chars().count() == 1 => name.to_uppercase(),
        _ => name.to_string(),
    };
    Key::from_name(&alias).ok_or_else(|| super::agent_text(|| fp_i18n::tr!("err-mcp-ui-unknown-key", key = name)))
}

fn paths_of(v: Option<&Value>) -> Result<Vec<PathBuf>, String> {
    let a = v.and_then(Value::as_array).ok_or("paths: Liste von Dateipfaden")?;
    let p: Vec<PathBuf> = a.iter().filter_map(Value::as_str).map(PathBuf::from).collect();
    if p.is_empty() {
        return Err(super::at("err-mcp-ui-paths-empty"));
    }
    Ok(p)
}

fn rect_of(v: Option<&Value>) -> Option<[f32; 4]> {
    let a = v?.as_array()?;
    if a.len() != 4 {
        return None;
    }
    let f = |i: usize| a[i].as_f64().unwrap_or(0.0) as f32;
    Some([f(0), f(1), f(2), f(3)])
}

fn fmt_pos(p: Pos2) -> String {
    format!("({:.0}, {:.0})", p.x, p.y)
}

/// Baut eine Eingabefolge aus `{"events": [...], ...}`.
fn parse(args: &Value) -> Result<Script, String> {
    let events = args.get("events").and_then(Value::as_array).ok_or_else(|| super::at("err-mcp-ui-events-missing"))?;
    let each = args.get("screenshot_each").and_then(Value::as_bool).unwrap_or(false);
    let settle = args.get("settle_frames").and_then(Value::as_u64).unwrap_or(2).min(60) as u32;
    let final_shot = match args.get("screenshot") {
        Some(Value::Bool(true)) => Some(None),
        Some(v @ Value::Array(_)) => Some(Some(rect_of(Some(v)).ok_or_else(|| super::at("err-mcp-ui-screenshot-arg"))?)),
        _ => None,
    };
    let mut steps: VecDeque<Step> = VecDeque::new();
    let mut labels = Vec::new();
    let frame = |steps: &mut VecDeque<Step>, evs: Vec<Ev>| steps.push_back(Step::Frame(evs));
    for (i, e) in events.iter().enumerate() {
        let t = e.get("type").and_then(Value::as_str).ok_or_else(|| super::agent_text(|| fp_i18n::tr!("err-mcp-ui-step-type", n = i + 1)))?;
        let err = |m: String| super::agent_text(|| fp_i18n::tr!("err-mcp-ui-step", n = i + 1, kind = t, error = m));
        let pos = pos_of(e.get("pos"));
        let mods = mods_of(e.get("modifiers")).map_err(err)?;
        let with_mods = !mods.is_none();
        match t {
            "move" => {
                let p = pos.ok_or_else(|| err(missing("pos")))?;
                frame(&mut steps, vec![Ev::Move(p)]);
                labels.push(format!("move {}", fmt_pos(p)));
            }
            "down" | "up" => {
                let button = button_of(e.get("button")).map_err(err)?;
                if let Some(p) = pos {
                    frame(&mut steps, vec![Ev::Move(p)]);
                }
                frame(&mut steps, vec![Ev::Button { button, pressed: t == "down" }]);
                labels.push(format!("{t} {:?}", button));
            }
            "click" | "double_click" | "right_click" => {
                let button = if t == "right_click" { PointerButton::Secondary } else { button_of(e.get("button")).map_err(err)? };
                let count = if t == "double_click" { 2 } else { e.get("count").and_then(Value::as_u64).unwrap_or(1).clamp(1, 3) };
                if with_mods {
                    frame(&mut steps, vec![Ev::PushMods(mods)]);
                }
                if let Some(p) = pos {
                    frame(&mut steps, vec![Ev::Move(p)]);
                }
                for _ in 0..count {
                    frame(&mut steps, vec![Ev::Button { button, pressed: true }]);
                    frame(&mut steps, vec![Ev::Button { button, pressed: false }]);
                }
                if with_mods {
                    frame(&mut steps, vec![Ev::PopMods]);
                }
                labels.push(format!("{t}{} {}", if count > 1 { format!(" ×{count}") } else { String::new() }, pos.map(fmt_pos).unwrap_or_else(|| super::at("msg-mcp-ui-pointer"))));
            }
            "drag" => {
                let from = pos_of(e.get("from")).or(pos).ok_or_else(|| err(missing("from")))?;
                let to = pos_of(e.get("to")).ok_or_else(|| err(missing("to")))?;
                let n = e.get("steps").and_then(Value::as_u64).unwrap_or(12).clamp(1, 400) as usize;
                let ms = e.get("ms").and_then(Value::as_u64).unwrap_or(16).min(2000);
                let hold = e.get("hold_ms").and_then(Value::as_u64).unwrap_or(0).min(5000);
                let button = button_of(e.get("button")).map_err(err)?;
                if with_mods {
                    frame(&mut steps, vec![Ev::PushMods(mods)]);
                }
                frame(&mut steps, vec![Ev::Move(from)]);
                frame(&mut steps, vec![Ev::Button { button, pressed: true }]);
                if hold > 0 {
                    steps.push_back(Step::Wait(hold));
                }
                for k in 1..=n {
                    if ms > 0 {
                        steps.push_back(Step::Wait(ms));
                    }
                    let f = k as f32 / n as f32;
                    frame(&mut steps, vec![Ev::Move(from + (to - from) * f)]);
                }
                frame(&mut steps, vec![Ev::Button { button, pressed: false }]);
                if with_mods {
                    frame(&mut steps, vec![Ev::PopMods]);
                }
                labels.push(super::agent_text(|| fp_i18n::tr!("msg-mcp-ui-drag", from = fmt_pos(from), to = fmt_pos(to), n = n)));
            }
            "wheel" | "scroll" => {
                let d = match e.get("delta") {
                    Some(Value::Array(a)) if a.len() == 2 => vec2(a[0].as_f64().unwrap_or(0.0) as f32, a[1].as_f64().unwrap_or(0.0) as f32),
                    Some(Value::Number(n)) => vec2(0.0, n.as_f64().unwrap_or(0.0) as f32),
                    _ => return Err(err(super::at("err-mcp-ui-delta"))),
                };
                let unit = match e.get("unit").and_then(Value::as_str).unwrap_or("line") {
                    "line" | "zeile" => MouseWheelUnit::Line,
                    "point" | "punkt" | "pixel" => MouseWheelUnit::Point,
                    "page" | "seite" => MouseWheelUnit::Page,
                    other => return Err(err(super::agent_text(|| fp_i18n::tr!("err-mcp-ui-bad-value", name = "unit", value = other, list = "line, point, page")))),
                };
                if with_mods {
                    frame(&mut steps, vec![Ev::PushMods(mods)]);
                }
                if let Some(p) = pos {
                    frame(&mut steps, vec![Ev::Move(p)]);
                }
                let repeat = e.get("repeat").and_then(Value::as_u64).unwrap_or(1).clamp(1, 50);
                for _ in 0..repeat {
                    frame(&mut steps, vec![Ev::Wheel { unit, delta: d }]);
                }
                if with_mods {
                    frame(&mut steps, vec![Ev::PopMods]);
                }
                labels.push(format!("wheel {:?} ({}, {}) ×{repeat}", unit, d.x, d.y));
            }
            "key" => {
                let key = key_of(e.get("key")).map_err(err)?;
                let action = e.get("action").and_then(Value::as_str).unwrap_or("tap");
                let repeat = e.get("repeat").and_then(Value::as_u64).unwrap_or(1).clamp(1, 100);
                if with_mods {
                    frame(&mut steps, vec![Ev::PushMods(mods)]);
                }
                for _ in 0..repeat {
                    match action {
                        "tap" => {
                            frame(&mut steps, vec![Ev::KeyDown(key)]);
                            frame(&mut steps, vec![Ev::KeyUp(key)]);
                        }
                        "press" | "down" => frame(&mut steps, vec![Ev::KeyDown(key)]),
                        "release" | "up" => frame(&mut steps, vec![Ev::KeyUp(key)]),
                        other => return Err(err(super::agent_text(|| fp_i18n::tr!("err-mcp-ui-bad-value", name = "action", value = other, list = "tap, press, release")))),
                    }
                }
                if with_mods {
                    frame(&mut steps, vec![Ev::PopMods]);
                }
                let m = mods_json(mods);
                labels.push(format!("key {}{} {action}", if m.is_empty() { String::new() } else { format!("{}+", m.join("+")) }, key.name()));
            }
            "modifiers" | "hold" => {
                frame(&mut steps, vec![Ev::Mods(mods)]);
                labels.push(format!("modifiers {:?}", mods_json(mods)));
            }
            "text" => {
                let s = e.get("text").and_then(Value::as_str).ok_or_else(|| err(missing("text")))?;
                frame(&mut steps, vec![Ev::Text(s.to_string())]);
                labels.push(format!("text {s:?}"));
            }
            "type" => {
                let s = e.get("text").and_then(Value::as_str).ok_or_else(|| err(missing("text")))?;
                let ms = e.get("ms").and_then(Value::as_u64).unwrap_or(0).min(1000);
                for c in s.chars() {
                    frame(&mut steps, vec![Ev::TypeChar(c)]);
                    if let Some(k) = key_for_char(c) {
                        frame(&mut steps, vec![Ev::KeyUp(k)]);
                    }
                    if ms > 0 {
                        steps.push_back(Step::Wait(ms));
                    }
                }
                labels.push(format!("type {s:?}"));
            }
            "copy" => {
                frame(&mut steps, vec![Ev::Copy]);
                labels.push("copy".into());
            }
            "cut" => {
                frame(&mut steps, vec![Ev::Cut]);
                labels.push("cut".into());
            }
            "paste" => {
                frame(&mut steps, vec![Ev::Paste(e.get("text").and_then(Value::as_str).map(str::to_string))]);
                labels.push("paste".into());
            }
            "zoom" => {
                let f = e.get("factor").and_then(Value::as_f64).ok_or_else(|| err(missing("factor")))? as f32;
                frame(&mut steps, vec![Ev::Zoom(f)]);
                labels.push(format!("zoom ×{f}"));
            }
            "hover_files" => {
                let p = paths_of(e.get("paths")).map_err(err)?;
                frame(&mut steps, vec![Ev::Hover(p)]);
                labels.push("hover_files".into());
            }
            "hover_end" => {
                frame(&mut steps, vec![Ev::Hover(Vec::new())]);
                labels.push("hover_end".into());
            }
            "drop_files" => {
                let p = paths_of(e.get("paths")).map_err(err)?;
                let at = pos.ok_or_else(|| err(missing("pos")))?;
                // wie OLE-Drag&Drop unter Windows: Schweben, Ablegen, danach erst Zeigerposition
                frame(&mut steps, vec![Ev::Hover(p.clone())]);
                frame(&mut steps, vec![Ev::Drop(p.clone())]);
                frame(&mut steps, vec![Ev::Move(at)]);
                labels.push(super::agent_text(|| fp_i18n::tr!("msg-mcp-ui-drop", n = p.len(), pos = fmt_pos(at))));
            }
            "wait" | "sleep" => {
                let ms = e.get("ms").and_then(Value::as_u64).ok_or_else(|| err(missing("ms")))?.min(30_000);
                steps.push_back(Step::Wait(ms));
                labels.push(format!("wait {ms} ms"));
            }
            "frames" => {
                let n = e.get("n").and_then(Value::as_u64).unwrap_or(1).clamp(1, 240) as u32;
                steps.push_back(Step::Frames(n));
                labels.push(format!("frames {n}"));
            }
            "screenshot" => {
                steps.push_back(Step::Shot(rect_of(e.get("rect"))));
                labels.push("screenshot".into());
            }
            "pointer_gone" => {
                frame(&mut steps, vec![Ev::Gone]);
                labels.push("pointer_gone".into());
            }
            "focus" => {
                let f = e.get("focused").and_then(Value::as_bool).unwrap_or(true);
                frame(&mut steps, vec![Ev::Focus(f)]);
                labels.push(format!("focus {f}"));
            }
            other => {
                return Err(err(super::agent_text(|| fp_i18n::tr!("err-mcp-ui-unknown-type", kind = other, list = "move, click, double_click, right_click, down, up, drag, wheel, key, modifiers, text, type, copy, cut, paste, zoom, hover_files, hover_end, drop_files, wait, frames, screenshot, pointer_gone, focus"))));
            }
        }
        if each && t != "screenshot" && t != "wait" && t != "frames" {
            steps.push_back(Step::Shot(None));
        }
        if steps.len() > MAX_STEPS {
            return Err(super::agent_text(|| fp_i18n::tr!("err-mcp-ui-too-many-frames", max = MAX_STEPS)));
        }
    }
    Ok(Script {
        steps,
        reply: None,
        labels,
        settle,
        final_shot,
        not_before: None,
        frames_left: 0,
        shot_request: None,
        shot_wait: None,
        images: Vec::new(),
        widget_events: Vec::new(),
        copied: Vec::new(),
        idle_frames: 0,
        injected: 0,
        frames: 0,
        started: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_after(evs: Vec<Ev>, base: &mut Modifiers, stack: &mut Vec<Modifiers>, pointer: &mut Option<Pos2>) -> egui::RawInput {
        let mut raw = egui::RawInput::default();
        for e in evs {
            apply(e, &mut raw, base, stack, pointer);
        }
        raw
    }

    #[test]
    fn ctrl_c_becomes_copy_like_egui_winit() {
        let (mut b, mut s, mut p) = (Modifiers::NONE, Vec::new(), None);
        let ctrl = Modifiers { ctrl: true, command: true, ..Modifiers::NONE };
        let raw = raw_after(vec![Ev::PushMods(ctrl), Ev::KeyDown(Key::C), Ev::KeyUp(Key::C), Ev::PopMods], &mut b, &mut s, &mut p);
        assert!(matches!(raw.events[0], Event::ModifiersChanged(m) if m.ctrl));
        assert!(matches!(raw.events[1], Event::Copy));
        assert!(matches!(raw.events[2], Event::Key { key: Key::C, pressed: false, modifiers, .. } if modifiers.ctrl));
        assert!(matches!(raw.events[3], Event::ModifiersChanged(m) if !m.ctrl));
        assert!(s.is_empty());
        // ohne Strg: Taste + Text, kein Copy
        let raw = raw_after(vec![Ev::KeyDown(Key::C)], &mut b, &mut s, &mut p);
        assert!(matches!(&raw.events[..], [Event::Key { key: Key::C, pressed: true, .. }, Event::Text(t)] if t == "c"));
        // Strg+X → Cut, Umschalt+Entf → Cut
        let raw = raw_after(vec![Ev::PushMods(ctrl), Ev::KeyDown(Key::X), Ev::PopMods], &mut b, &mut s, &mut p);
        assert!(matches!(raw.events[1], Event::Cut));
        let shift = Modifiers { shift: true, ..Modifiers::NONE };
        let raw = raw_after(vec![Ev::PushMods(shift), Ev::KeyDown(Key::Delete), Ev::PopMods], &mut b, &mut s, &mut p);
        assert!(matches!(raw.events[1], Event::Cut));
    }

    #[test]
    fn click_needs_pointer_position_and_carries_modifiers() {
        let (mut b, mut s, mut p) = (Modifiers::NONE, Vec::new(), None);
        let raw = raw_after(vec![Ev::Button { button: PointerButton::Primary, pressed: true }], &mut b, &mut s, &mut p);
        assert!(raw.events.is_empty(), "ohne Zeigerposition verwirft egui-winit den Klick");
        let shift = Modifiers { shift: true, ..Modifiers::NONE };
        let raw = raw_after(vec![Ev::Mods(shift), Ev::Move(pos2(5.0, 6.0)), Ev::Button { button: PointerButton::Primary, pressed: true }], &mut b, &mut s, &mut p);
        assert!(matches!(raw.events[2], Event::PointerButton { pos, pressed: true, modifiers, .. } if pos == pos2(5.0, 6.0) && modifiers.shift));
        let raw = raw_after(vec![Ev::PushMods(Modifiers { ctrl: true, command: true, ..Modifiers::NONE }), Ev::Wheel { unit: MouseWheelUnit::Line, delta: vec2(0.0, 1.0) }, Ev::PopMods], &mut b, &mut s, &mut p);
        assert!(matches!(raw.events[1], Event::MouseWheel { modifiers, .. } if modifiers.ctrl && modifiers.shift));
        assert!(matches!(raw.events[2], Event::ModifiersChanged(m) if m.shift && !m.ctrl), "Grundzustand bleibt");
    }

    #[test]
    fn files_and_text() {
        let (mut b, mut s, mut p) = (Modifiers::NONE, Vec::new(), None);
        let raw = raw_after(vec![Ev::Hover(vec!["a.png".into()]), Ev::Drop(vec!["a.png".into()])], &mut b, &mut s, &mut p);
        assert!(raw.hovered_files.is_empty() && raw.dropped_files.len() == 1);
        assert_eq!(raw.dropped_files[0].path(), Path::new("a.png"));
        let raw = raw_after(vec![Ev::TypeChar('H'), Ev::TypeChar('ä')], &mut b, &mut s, &mut p);
        assert!(matches!(&raw.events[..], [Event::Key { key: Key::H, .. }, Event::Text(a), Event::Text(b)] if a == "H" && b == "ä"));
    }

    #[test]
    fn structure_and_errors() {
        let s = parse(&json!({"events": [
            {"type": "click", "pos": [10, 20], "count": 2},
            {"type": "drag", "from": [0, 0], "to": [100, 0], "steps": 4, "ms": 0},
            {"type": "key", "key": "v", "modifiers": ["ctrl"]},
            {"type": "wheel", "delta": [0, 1], "modifiers": "ctrl", "pos": [5, 5]},
            {"type": "drop_files", "paths": ["x.png"], "pos": [50, 60]},
            {"type": "wait", "ms": 5}
        ], "screenshot": true}))
        .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(s.labels.len(), 6);
        // click ×2: Move + 4 Tasten-Frames; drag: Move, Press, 4 Moves, Release
        let frames = s.steps.iter().filter(|st| matches!(st, Step::Frame(_))).count();
        assert_eq!(frames, 5 + 7 + 4 + 4 + 3);
        assert!(s.final_shot.is_some());
        assert!(parse(&json!({"events": [{"type": "fliegen"}]})).is_err_and(|e| e.contains("unknown type")), "Antwortsprache der Agenten: Englisch");
        assert!(parse(&json!({"events": [{"type": "key", "key": "Hyper"}]})).is_err_and(|e| e.contains("unknown key")));
        assert!(parse(&json!({"events": [{"type": "click", "modifiers": ["meta"]}]})).is_err_and(|e| e.contains("modifier key")));
        assert!(parse(&json!({})).is_err());
    }

    /// Ganzes Hauptfenster offscreen: ui_find findet Menü und Leinwand-Beschriftungen, ui_input
    /// wählt ein Objekt per Klick, Strg+C/Strg+V verdoppelt es, Strg+Mausrad zoomt, Wiederholung
    /// ohne Eskalation (§4b).
    #[test]
    fn main_window_driven_by_input() {
        use crate::app_state::UiSettings;
        use crate::FirepixelApp;
        use fp_mcp::SessionHost;
        NO_OS_CLIPBOARD.store(true, std::sync::atomic::Ordering::Relaxed);
        let mut session = fp_engine::Session::default();
        let _ = session.execute("file.new", json!({"width": 400, "height": 300, "background": "#ffffff"}));
        let _ = session.execute("draw.rectangle", json!({"rect": [100, 100, 200, 180]}));
        let _ = session.execute("select.none", json!({}));
        let ctx = egui::Context::default();
        let mut app = FirepixelApp::with_session(&ctx, session, UiSettings::default(), false);
        app.state.os_clipboard = false;
        type St = (FirepixelApp, Option<super::super::UiBridge>);
        let mut h = egui_kittest::Harness::builder()
            .with_size(egui::vec2(1500.0, 950.0))
            .with_pixels_per_point(1.0)
            .with_max_steps(8)
            .build_ui_state(
                |ui, s: &mut St| {
                    s.0.root_ui(ui);
                    if let Some(b) = s.1.as_mut() {
                        b.process(ui.ctx(), &mut s.0.state);
                    }
                },
                (app, None),
            );
        let (host, bridge) = super::super::UiBridge::detached(&h.ctx);
        h.state_mut().1 = Some(bridge);
        h.run_steps(4);

        let ask = |h: &mut egui_kittest::Harness<'static, St>, req: fp_mcp::UiRequest| -> UiReply {
            let (tx, rx) = std::sync::mpsc::channel();
            host.ui_request(req, Box::new(move |r| {
                let _ = tx.send(r);
            }))
            .unwrap_or_else(|e| panic!("{e}"));
            for _ in 0..400 {
                h.step();
                if let Ok(r) = rx.try_recv() {
                    return r;
                }
            }
            panic!("keine Antwort");
        };
        let json_of = |r: UiReply| match r {
            UiReply::Json(v) | UiReply::JsonImages(v, _) => v,
            other => panic!("{other:?}"),
        };
        // Menü „Datei“ finden
        let found = json_of(ask(&mut h, fp_mcp::UiRequest::Find(json!({"text": "Datei", "exact": true}))));
        assert!(found["count"].as_u64().unwrap_or(0) >= 1, "{found}");
        let c = &found["hits"][0]["center"];
        assert!(c[1].as_f64().unwrap_or(999.0) < 60.0, "Menüleiste oben: {found}");

        // Objektmitte auf dem Bildschirm
        let center = {
            let st = &h.state().0.state;
            let d = st.session.active().unwrap_or_else(|| panic!("Dokument"));
            let area = st.canvases.get(&d.id).and_then(|c| c.area).unwrap_or_else(|| panic!("Leinwand"));
            let (sx, sy) = d.view.doc_to_screen(fp_geom::Point::new(150.0, 140.0));
            pos2(area.min.x + sx as f32, area.min.y + sy as f32)
        };
        let count = |h: &egui_kittest::Harness<'static, St>| {
            let d = h.state().0.state.session.active().unwrap_or_else(|| panic!());
            fp_engine::docops::selectable_ids(&d.doc, d.page, d.frame).len()
        };
        assert_eq!(count(&h), 1);
        let _ = h.state_mut().0.state.session.set_tool(fp_engine::ToolId::Pointer);
        let r = json_of(ask(
            &mut h,
            fp_mcp::UiRequest::Input(json!({"events": [
                {"type": "click", "pos": [center.x, center.y]},
                {"type": "key", "key": "C", "modifiers": ["ctrl"]},
                {"type": "key", "key": "V", "modifiers": ["ctrl"]}
            ]})),
        ));
        assert_eq!(r["ok"], true, "{r}");
        assert_eq!(count(&h), 2, "Strg+C/Strg+V verdoppelt das Objekt: {r}");
        assert_eq!(r["held_modifiers"], json!([]), "keine hängenden Umschalttasten");
        // Wiederholung: jedes Strg+V genau ein weiteres Objekt
        for k in 0..3 {
            let _ = json_of(ask(&mut h, fp_mcp::UiRequest::Input(json!({"events": [{"type": "key", "key": "V", "modifiers": ["ctrl"]}]}))));
            assert_eq!(count(&h), 3 + k);
        }
        // Strg+Mausrad über der Leinwand zoomt
        let z0 = h.state().0.state.session.active().map(|d| d.view.zoom).unwrap_or(0.0);
        let _ = json_of(ask(&mut h, fp_mcp::UiRequest::Input(json!({"events": [{"type": "wheel", "delta": [0, 1], "pos": [center.x, center.y], "modifiers": ["ctrl"]}]}))));
        let z1 = h.state().0.state.session.active().map(|d| d.view.zoom).unwrap_or(0.0);
        assert!(z1 > z0, "Strg+Mausrad nach oben vergrößert: {z0} → {z1}");
        // Fehler im Aufbau kommt sofort zurück
        assert!(matches!(ask(&mut h, fp_mcp::UiRequest::Input(json!({"events": [{"type": "x"}]}))), UiReply::Error(_)));
    }
}
