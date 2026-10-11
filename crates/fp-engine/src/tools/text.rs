//! Textwerkzeug mit Text-Editor auf der Leinwand (10 §2.6 „Text“, Zustände
//! `CTextCommandHandlerState_EditText`/`_NoEditText`).
//!
//! Klick = Punkttext (automatische Breite), Ziehen = Rahmen fester Breite, Klick in vorhandenen
//! Text = bearbeiten. Der Editor hält Text, Caret und Auswahlanker als Zeichenindizes (Unicode-
//! Skalare wie fp-text); Zeilengeometrie (Klick, Pos1/Ende, Pfeil auf/ab) rechnet die UI mit dem
//! Textlayout aus und meldet das Ergebnis als [`ToolInput::TextCaret`]. Esc bzw. Werkzeugwechsel
//! übernimmt (`draw.text` bzw. `text.setText`), danach zurück zum Zeiger
//! (`TextToolRevertsToPointer`). Leerer neuer Text erzeugt nichts; leerer vorhandener Text wird
//! gelöscht (wie das Original).

use super::snap::Snapper;
use super::{dist, pt_json, rect_json, Cursor, EditOp, Invocation, Overlay, PointerKind, ToolContext, ToolEvent, ToolId, ToolInput, ToolKey, ToolMachine, ToolResponse};
use crate::docops::HitOptions;
use fp_color::Argb;
use fp_geom::{Point, Rect};
use fp_model::element::{Element, ElementKind};
use fp_model::text::TextRun;
use fp_model::{Document, Extra, IdGen};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Zustand des Text-Editors für die Anzeige (Overlay [`Overlay::TextEdit`]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextEditView {
    /// Bearbeitetes Textobjekt (None = neuer Text).
    pub target: Option<u64>,
    /// Linke obere Ecke des Rahmens (neuer Text).
    pub anchor: Point,
    /// Feste Breite (Rahmentext), None = automatische Breite.
    pub width: Option<f32>,
    pub text: String,
    /// Caret (Zeichenindex, vor diesem Zeichen).
    pub caret: usize,
    /// Auswahlanker (= Caret ohne Auswahl).
    pub sel_anchor: usize,
    /// IME-Komposition (wird am Caret eingeblendet).
    pub preedit: Option<String>,
    pub font: String,
    pub size: f32,
    pub color: Argb,
}

impl TextEditView {
    /// Auswahlbereich (Anfang, Ende) in Zeichen.
    pub fn selection(&self) -> (usize, usize) {
        (self.caret.min(self.sel_anchor), self.caret.max(self.sel_anchor))
    }

    /// Element für die Vorschau (fp-render): vorhandenes Textobjekt mit neuem Text bzw. neues
    /// Textobjekt wie `draw.text`. Die IME-Komposition wird am Caret eingefügt.
    pub fn preview_element(&self, doc: &Document) -> Option<Element> {
        let text = self.display_text();
        if let Some(id) = self.target {
            let e = find_element(doc, id)?;
            let mut e = (*e).clone();
            if let ElementKind::Text(t) = &mut e.kind {
                let attrs = t.runs.first().map(|r| r.attrs.clone()).unwrap_or_else(|| t.initial.clone());
                t.runs = vec![TextRun { attrs, text, extra: Extra::default() }];
                t.detach_story();
            }
            return Some(e);
        }
        let mut ids = IdGen::default();
        let mut e = Element::new_text(&mut ids, &text, &self.font, self.size, self.color, self.baseline_anchor());
        if let (Some(w), ElementKind::Text(t)) = (self.width, &mut e.kind) {
            t.frame_type = Some(1);
            t.bounds = Rect::new(self.anchor.x, self.anchor.y, self.anchor.x + w, self.anchor.y + self.size * 1.2);
            t.anchor_rect = Some(t.bounds);
        }
        Some(e)
    }

    /// Text inklusive IME-Komposition am Caret.
    pub fn display_text(&self) -> String {
        match &self.preedit {
            Some(p) if !p.is_empty() => {
                let mut s: Vec<char> = self.text.chars().collect();
                let at = self.caret.min(s.len());
                let ins: Vec<char> = p.chars().collect();
                s.splice(at..at, ins);
                s.into_iter().collect()
            }
            _ => self.text.clone(),
        }
    }

    /// Erste Grundlinie eines neuen Punkttexts (wie `draw.text` mit `anchor`).
    pub fn baseline_anchor(&self) -> Point {
        Point::new(self.anchor.x, self.anchor.y + self.size)
    }
}

fn find_element(doc: &Document, id: u64) -> Option<std::sync::Arc<Element>> {
    let eid = fp_model::ElementId(id);
    (0..doc.pages.len()).find_map(|p| crate::docops::element_on_page(doc, p, eid))
}

#[derive(Clone, Debug, PartialEq)]
enum TextState {
    Idle,
    Pressed { start: Point },
    Editing(Box<TextEditView>),
}

/// Text: Klick = Punkttext, Ziehen = fester Rahmen, Klick in vorhandenen Text = bearbeiten.
#[derive(Clone, Debug)]
pub struct TextTool {
    state: TextState,
    /// Rahmen beim Aufziehen (Rückmeldung).
    drag: Option<(Point, Point)>,
    /// Editor-Verlauf (nur während einer Bearbeitung).
    undo: Vec<EditSnap>,
    redo: Vec<EditSnap>,
    /// Letzte Änderung war Tippen (aufeinanderfolgendes Tippen = ein Schritt).
    last_typing: bool,
}

/// Schritt im Editor-Verlauf.
#[derive(Clone, Debug, PartialEq)]
struct EditSnap {
    key: (Option<u64>, u32, u32),
    text: String,
    caret: usize,
    anchor: usize,
}

impl Default for TextTool {
    fn default() -> Self {
        TextTool { state: TextState::Idle, drag: None, undo: Vec::new(), redo: Vec::new(), last_typing: false }
    }
}

fn char_len(s: &str) -> usize {
    s.chars().count()
}

/// Byte-Index eines Zeichenindex.
fn byte_at(s: &str, ci: usize) -> usize {
    s.char_indices().nth(ci).map(|(b, _)| b).unwrap_or(s.len())
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

impl TextTool {
    fn commit(&mut self, ctx: &ToolContext, revert: bool) -> ToolResponse {
        let st = std::mem::replace(&mut self.state, TextState::Idle);
        let TextState::Editing(v) = st else { return ToolResponse::default() };
        let mut resp = ToolResponse::default();
        if let Some(id) = v.target {
            let old = find_element(ctx.doc, id).and_then(|e| match &e.kind {
                ElementKind::Text(t) => Some(t.plain_text()),
                _ => None,
            });
            if old.as_deref() != Some(v.text.as_str()) {
                resp.commands.push(Invocation::new("text.setText", json!({"id": id, "text": v.text})));
            }
        } else if !v.text.is_empty() {
            let mut p = json!({"text": v.text, "font": v.font, "size": v.size});
            match v.width {
                Some(w) => p["rect"] = rect_json(Rect::new(v.anchor.x, v.anchor.y, v.anchor.x + w, v.anchor.y + v.size * 1.2)),
                None => p["anchor"] = pt_json(v.baseline_anchor()),
            }
            resp.commands.push(Invocation::new("draw.text", p));
        }
        if revert && ctx.prefs.get_bool("TextToolRevertsToPointer", true) {
            resp.switch_tool = Some(ToolId::Pointer);
        }
        resp
    }

    /// Aktueller Textpuffer (für die UI).
    pub fn buffer(&self) -> Option<&str> {
        match &self.state {
            TextState::Editing(v) => Some(&v.text),
            _ => None,
        }
    }

    fn editing(&mut self) -> Option<&mut TextEditView> {
        match &mut self.state {
            TextState::Editing(v) => Some(v),
            _ => None,
        }
    }

    fn delete_selection(v: &mut TextEditView) -> bool {
        let (a, b) = v.selection();
        if a == b {
            return false;
        }
        let (ba, bb) = (byte_at(&v.text, a), byte_at(&v.text, b));
        v.text.replace_range(ba..bb, "");
        v.caret = a;
        v.sel_anchor = a;
        true
    }

    fn insert(v: &mut TextEditView, s: &str) {
        Self::delete_selection(v);
        let b = byte_at(&v.text, v.caret);
        // Zeilenumbrüche als Absatzende `\r` wie das Original
        let clean: String = s.replace("\r\n", "\r").replace('\n', "\r").chars().filter(|c| *c == '\r' || *c == '\t' || !c.is_control()).collect();
        v.text.insert_str(b, &clean);
        v.caret += char_len(&clean);
        v.sel_anchor = v.caret;
        v.preedit = None;
    }

    fn word_left(v: &TextEditView) -> usize {
        let cs: Vec<char> = v.text.chars().collect();
        let mut i = v.caret.min(cs.len());
        while i > 0 && !is_word(cs[i - 1]) {
            i -= 1;
        }
        while i > 0 && is_word(cs[i - 1]) {
            i -= 1;
        }
        i
    }

    fn word_right(v: &TextEditView) -> usize {
        let cs: Vec<char> = v.text.chars().collect();
        let mut i = v.caret.min(cs.len());
        while i < cs.len() && is_word(cs[i]) {
            i += 1;
        }
        while i < cs.len() && !is_word(cs[i]) {
            i += 1;
        }
        i
    }

    fn start_editing(&mut self, ctx: &ToolContext, p: Point) {
        let opt = HitOptions { tolerance: ctx.pick_tolerance(), ..HitOptions::default() };
        let hit = crate::docops::hits_at(ctx.doc, ctx.page, ctx.frame, p, ctx.geometry, &opt)
            .into_iter()
            .filter_map(|path| ctx.doc.element_at(&crate::docops::deep_hit(ctx.doc, &path, p, ctx.geometry, &opt)).cloned())
            .find(|e| matches!(e.kind, ElementKind::Text(_)));
        if let Some(e) = hit
            && let ElementKind::Text(t) = &e.kind
        {
            let text = t.plain_text();
            let n = char_len(&text);
            let font = t.runs.first().and_then(|r| r.attrs.font.clone()).unwrap_or_else(|| ctx.options.text_font.clone());
            let size = t.runs.first().and_then(|r| r.attrs.size).unwrap_or(ctx.options.text_size);
            self.state = TextState::Editing(Box::new(TextEditView {
                target: Some(e.id.0),
                anchor: Point::new(t.bounds.left, t.bounds.top),
                width: None,
                text,
                caret: n,
                sel_anchor: n,
                preedit: None,
                font,
                size,
                color: ctx.fill_color,
            }));
        } else {
            self.state = TextState::Pressed { start: p };
        }
    }

    /// Zustand des Editors für den Editor-Verlauf: (Sitzungskennung, Text, Caret, Anker).
    fn snap(&self) -> Option<EditSnap> {
        match &self.state {
            TextState::Editing(v) => Some(EditSnap { key: (v.target, v.anchor.x.to_bits(), v.anchor.y.to_bits()), text: v.text.clone(), caret: v.caret, anchor: v.sel_anchor }),
            _ => None,
        }
    }

    /// Strg+Z / Strg+Y im Editor: Texteingabe zurücknehmen bzw. wiederholen (wie im Original
    /// zuerst im Editor, nicht im Dokumentverlauf).
    fn undo_redo(&mut self, undo: bool) -> ToolResponse {
        let Some(cur) = self.snap() else { return ToolResponse::default() };
        let (from, to) = if undo { (&mut self.undo, &mut self.redo) } else { (&mut self.redo, &mut self.undo) };
        if let Some(s) = from.pop() {
            to.push(cur);
            if let TextState::Editing(v) = &mut self.state {
                v.text = s.text;
                v.caret = s.caret;
                v.sel_anchor = s.anchor;
                v.preedit = None;
            }
        }
        self.last_typing = false;
        ToolResponse::with_overlay(self.overlay(), Cursor::Text)
    }

    fn handle_inner(&mut self, ev: &ToolEvent, ctx: &ToolContext) -> ToolResponse {
        match &ev.input {
            ToolInput::Pointer { kind: PointerKind::Down, pos } => {
                let mut resp = ToolResponse::default();
                if matches!(self.state, TextState::Editing(_)) {
                    resp = self.commit(ctx, false);
                    // die Befehle laufen erst nach dieser Antwort: Klick in den eben übernommenen Text
                    // trifft ihn noch nicht – dann einen neuen Text beginnen
                }
                self.start_editing(ctx, *pos);
                self.drag = None;
                resp.cursor = Cursor::Text;
                resp.overlay = self.overlay();
                resp
            }
            ToolInput::Pointer { kind: PointerKind::Move, pos } => {
                if let TextState::Pressed { start } = self.state
                    && dist(start, *pos) >= ctx.drag_threshold()
                {
                    self.drag = Some((start, *pos));
                }
                ToolResponse::with_overlay(self.overlay(), Cursor::Text)
            }
            ToolInput::Pointer { kind: PointerKind::Up, pos } => {
                if let TextState::Pressed { start } = self.state {
                    let snap = Snapper::new(ctx, &[]).without_smart();
                    let s = snap.point(start).point;
                    let e = snap.point(*pos).point;
                    let width = if dist(start, *pos) >= ctx.drag_threshold() { Some((e.x - s.x).abs().max(1.0)) } else { None };
                    // Klickpunkt bzw. aufgezogener Rahmen = linke obere Ecke des Textblocks
                    let anchor = if width.is_some() { Point::new(s.x.min(e.x), s.y.min(e.y)) } else { s };
                    self.state = TextState::Editing(Box::new(TextEditView {
                        target: None,
                        anchor,
                        width,
                        text: String::new(),
                        caret: 0,
                        sel_anchor: 0,
                        preedit: None,
                        font: ctx.options.text_font.clone(),
                        size: ctx.options.text_size,
                        color: ctx.fill_color,
                    }));
                }
                self.drag = None;
                ToolResponse::with_overlay(self.overlay(), Cursor::Text)
            }
            ToolInput::Pointer { kind: PointerKind::DoubleClick, .. } => {
                // Doppelklick im Text: Wort auswählen (Zeichenposition kommt als TextCaret von der UI)
                if let Some(v) = self.editing() {
                    let caret = v.caret;
                    let cs: Vec<char> = v.text.chars().collect();
                    let mut a = caret.min(cs.len());
                    let mut b = a;
                    while a > 0 && is_word(cs[a - 1]) {
                        a -= 1;
                    }
                    while b < cs.len() && is_word(cs[b]) {
                        b += 1;
                    }
                    v.sel_anchor = a;
                    v.caret = b;
                }
                ToolResponse::with_overlay(self.overlay(), Cursor::Text)
            }
            ToolInput::Text(s) => {
                if let Some(v) = self.editing() {
                    Self::insert(v, s);
                }
                ToolResponse::with_overlay(self.overlay(), Cursor::Text)
            }
            ToolInput::Preedit(s) => {
                if let Some(v) = self.editing() {
                    v.preedit = if s.is_empty() { None } else { Some(s.clone()) };
                }
                ToolResponse::with_overlay(self.overlay(), Cursor::Text)
            }
            ToolInput::TextCaret { index, extend } => {
                if let Some(v) = self.editing() {
                    v.caret = (*index).min(char_len(&v.text));
                    if !extend {
                        v.sel_anchor = v.caret;
                    }
                }
                ToolResponse::with_overlay(self.overlay(), Cursor::Text)
            }
            ToolInput::Edit(op) => {
                let mut resp = ToolResponse { cursor: Cursor::Text, ..ToolResponse::default() };
                if let Some(v) = self.editing() {
                    match op {
                        EditOp::SelectAll => {
                            v.sel_anchor = 0;
                            v.caret = char_len(&v.text);
                        }
                        EditOp::Copy | EditOp::Cut => {
                            let (a, b) = v.selection();
                            if a < b {
                                let (ba, bb) = (byte_at(&v.text, a), byte_at(&v.text, b));
                                resp.clipboard = Some(v.text[ba..bb].replace('\r', "\r\n"));
                                if *op == EditOp::Cut {
                                    Self::delete_selection(v);
                                }
                            }
                        }
                        EditOp::Paste(s) => Self::insert(v, s),
                        EditOp::Undo | EditOp::Redo => {}
                    }
                }
                resp.overlay = self.overlay();
                resp
            }
            ToolInput::Key(k) => {
                let Some(v) = self.editing() else { return ToolResponse::default() };
                let n = char_len(&v.text);
                let extend = ev.mods.shift;
                match k {
                    ToolKey::Escape => return self.commit(ctx, true),
                    ToolKey::Backspace => {
                        if !Self::delete_selection(v) && v.caret > 0 {
                            let from = if ev.mods.ctrl { Self::word_left(v) } else { v.caret - 1 };
                            let (ba, bb) = (byte_at(&v.text, from), byte_at(&v.text, v.caret));
                            v.text.replace_range(ba..bb, "");
                            v.caret = from;
                            v.sel_anchor = from;
                        }
                    }
                    ToolKey::Delete => {
                        if !Self::delete_selection(v) && v.caret < n {
                            let to = if ev.mods.ctrl { Self::word_right(v) } else { v.caret + 1 };
                            let (ba, bb) = (byte_at(&v.text, v.caret), byte_at(&v.text, to));
                            v.text.replace_range(ba..bb, "");
                        }
                    }
                    ToolKey::Enter => Self::insert(v, "\r"),
                    ToolKey::Tab => Self::insert(v, "\t"),
                    ToolKey::Left | ToolKey::Right => {
                        let (a, b) = v.selection();
                        let to = if *k == ToolKey::Left {
                            if ev.mods.ctrl {
                                Self::word_left(v)
                            } else if a != b && !extend {
                                a
                            } else {
                                v.caret.saturating_sub(1)
                            }
                        } else if ev.mods.ctrl {
                            Self::word_right(v)
                        } else if a != b && !extend {
                            b
                        } else {
                            (v.caret + 1).min(n)
                        };
                        v.caret = to;
                        if !extend {
                            v.sel_anchor = to;
                        }
                    }
                    ToolKey::Home | ToolKey::Up if ev.mods.ctrl || !matches!(k, ToolKey::Up) => {
                        // ohne Layout: Pos1 = Textanfang (die UI schickt zeilengenaue Carets)
                        v.caret = 0;
                        if !extend {
                            v.sel_anchor = 0;
                        }
                    }
                    ToolKey::End | ToolKey::Down if ev.mods.ctrl || !matches!(k, ToolKey::Down) => {
                        v.caret = n;
                        if !extend {
                            v.sel_anchor = n;
                        }
                    }
                    _ => {}
                }
                ToolResponse::with_overlay(self.overlay(), Cursor::Text)
            }
        }
    }

}

impl ToolMachine for TextTool {
    fn id(&self) -> ToolId {
        ToolId::Text
    }

    fn handle(&mut self, ev: &ToolEvent, ctx: &ToolContext) -> ToolResponse {
        if !ctx.has_document {
            return ToolResponse::default();
        }
        match &ev.input {
            ToolInput::Edit(EditOp::Undo) => return self.undo_redo(true),
            ToolInput::Edit(EditOp::Redo) => return self.undo_redo(false),
            _ => {}
        }
        let before = self.snap();
        let resp = self.handle_inner(ev, ctx);
        let after = self.snap();
        let typing = matches!(ev.input, ToolInput::Text(_));
        match (before, after) {
            (Some(b), Some(a)) if b.key == a.key => {
                if b.text != a.text {
                    // aufeinanderfolgendes Tippen ist ein Schritt; Löschen, Einfügen, Umbruch je ein Schritt
                    if !(typing && self.last_typing) {
                        self.undo.push(b);
                    }
                    self.redo.clear();
                    self.last_typing = typing;
                } else if !matches!(ev.input, ToolInput::Preedit(_)) {
                    self.last_typing = false;
                }
            }
            _ => {
                // neue bzw. beendete Bearbeitung: eigener Verlauf
                self.undo.clear();
                self.redo.clear();
                self.last_typing = false;
            }
        }
        resp
    }

    fn finish(&mut self, ctx: &ToolContext) -> ToolResponse {
        self.commit(ctx, false)
    }

    fn overlay(&self) -> Vec<Overlay> {
        match &self.state {
            TextState::Editing(v) => vec![Overlay::TextEdit(v.clone())],
            _ => match self.drag {
                Some((a, b)) => vec![Overlay::Rect { rect: Rect::new(a.x, a.y, b.x, b.y).normalized(), dashed: false }],
                None => Vec::new(),
            },
        }
    }

    fn is_editing_text(&self) -> bool {
        matches!(self.state, TextState::Editing(_))
    }
}
