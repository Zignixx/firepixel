//! Textwerkzeug mit Text-Editor auf der Leinwand (10 §2.6 „Text“, Zustände
//! `CTextCommandHandlerState_EditText`/`_NoEditText`).
//!
//! Klick = Punkttext (automatische Breite), Ziehen = Rahmen fester Breite, Klick in vorhandenen
//! Text = bearbeiten. Der Editor hält Text, Caret und Auswahlanker als Zeichenindizes (Unicode-
//! Skalare wie fp-text) sowie die Formatierung als Läufe mit aufgelösten Attributen
//! ([`EditSpan`]); Zeilengeometrie (Klick, Pos1/Ende, Pfeil auf/ab) rechnet die UI mit dem
//! Textlayout aus und meldet das Ergebnis als [`ToolInput::TextCaret`]. Attribute aus Inspektor
//! und Menü „Text“ kommen während der Bearbeitung als [`EditOp::Attrs`]: mit Markierung nur für die
//! Markierung (Absatzattribute für ganze Absätze), ohne Markierung für den ganzen Text; der
//! Editor bleibt offen. Esc bzw. Werkzeugwechsel übernimmt (`draw.text` bzw. `text.setText`, je
//! mit `runs`), danach zurück zum Zeiger (`TextToolRevertsToPointer`). Leerer neuer Text erzeugt
//! nichts; leerer vorhandener Text wird gelöscht (wie das Original).

use super::snap::Snapper;
use super::{dist, pt_json, rect_json, Cursor, EditOp, Invocation, Overlay, PointerKind, ToolContext, ToolEvent, ToolId, ToolInput, ToolKey, ToolMachine, ToolResponse};
use crate::docops::HitOptions;
use fp_color::Argb;
use fp_geom::{Point, Rect};
use fp_model::element::{Element, ElementKind};
use fp_model::text::{TextAttrs, TextElem, TextRun};
use fp_model::{Document, Extra, IdGen};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

/// Formatierter Abschnitt des Editortexts (`len` Zeichen mit vollständig aufgelösten Attributen).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EditSpan {
    pub len: usize,
    pub attrs: TextAttrs,
}

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
    /// Schrift, Größe und Farbe der Grundattribute (Spiegel von `base`).
    pub font: String,
    pub size: f32,
    pub color: Argb,
    /// Formatierung (Summe der Längen = Zeichenzahl von `text`).
    #[serde(default)]
    pub spans: Vec<EditSpan>,
    /// Attribute für Text an leerer Stelle (neuer bzw. ganz gelöschter Text).
    #[serde(default)]
    pub base: TextAttrs,
}

/// Grundattribute eines neuen Texts (wie `draw.text`).
fn new_attrs(font: &str, size: f32, color: Argb) -> TextAttrs {
    let mut ids = IdGen::default();
    let e = Element::new_text(&mut ids, "x", font, size, color, Point::new(0.0, 0.0));
    match &e.kind {
        ElementKind::Text(t) => t.runs.first().map(|r| r.attrs.clone()).unwrap_or_default(),
        _ => TextAttrs::default(),
    }
}

/// Formatierung eines vorhandenen Texts als Editor-Abschnitte (Farbe ohne Laufwert = Textfarbe).
pub fn spans_of(t: &TextElem) -> Vec<EditSpan> {
    let mut out: Vec<EditSpan> = Vec::new();
    for (mut a, s) in t.resolved_runs() {
        let len = s.chars().count();
        if len == 0 {
            continue;
        }
        a.ps_font = None;
        a.kerning = None;
        if a.color.is_none() {
            a.color = Some(t.attrs.fill_color.0);
        }
        out.push(EditSpan { len, attrs: a });
    }
    normalize(&mut out);
    out
}

/// Setzt die Läufe eines Texts (Editor, `text.setText`/`draw.text` mit `runs`): Anfangsattribute
/// = erster Lauf (09 §3.4), einheitliche Laufarbe auch als Textfarbe, Story lösen.
pub fn apply_runs(t: &mut TextElem, runs: Vec<TextRun>) {
    if let Some(first) = runs.first() {
        let mut head = first.attrs.clone();
        head.color = None;
        head.ps_font = None;
        if let Some(f) = &head.font {
            t.initial.ps_font = Some(f.clone());
        }
        t.initial.apply(&head);
    }
    let colors: Vec<Option<u32>> = runs.iter().filter(|r| !r.text.is_empty()).map(|r| r.attrs.color).collect();
    if let Some(Some(c)) = colors.first()
        && colors.iter().all(|x| *x == Some(*c))
    {
        Arc::make_mut(&mut t.attrs).fill_color = Argb(*c);
    }
    t.runs = runs;
    t.detach_story();
}

/// Leere Abschnitte entfernen, gleiche Nachbarn zusammenfassen.
fn normalize(spans: &mut Vec<EditSpan>) {
    spans.retain(|s| s.len > 0);
    let mut i = 1;
    while i < spans.len() {
        if spans[i].attrs == spans[i - 1].attrs {
            spans[i - 1].len += spans[i].len;
            spans.remove(i);
        } else {
            i += 1;
        }
    }
}

/// Sorgt für eine Abschnittsgrenze bei Zeichen `i`; liefert den Index des dort beginnenden Abschnitts.
fn split_at(spans: &mut Vec<EditSpan>, i: usize) -> usize {
    let mut pos = 0;
    for k in 0..spans.len() {
        if pos == i {
            return k;
        }
        let l = spans[k].len;
        if i < pos + l {
            let right = EditSpan { len: pos + l - i, attrs: spans[k].attrs.clone() };
            spans[k].len = i - pos;
            spans.insert(k + 1, right);
            return k + 1;
        }
        pos += l;
    }
    spans.len()
}

/// Absatzattribute (gelten für ganze Absätze).
fn para_part(p: &TextAttrs) -> (TextAttrs, TextAttrs) {
    let mut ch = p.clone();
    let para = TextAttrs { alignment: ch.alignment.take(), indent: ch.indent.take(), space_before: ch.space_before.take(), space_after: ch.space_after.take(), ..TextAttrs::default() };
    (ch, para)
}

impl TextEditView {
    /// Auswahlbereich (Anfang, Ende) in Zeichen.
    pub fn selection(&self) -> (usize, usize) {
        (self.caret.min(self.sel_anchor), self.caret.max(self.sel_anchor))
    }

    /// Attribute des Zeichens `i` (bzw. Grundattribute).
    fn attrs_of_char(&self, i: usize) -> TextAttrs {
        let mut pos = 0;
        for s in &self.spans {
            if i < pos + s.len {
                return s.attrs.clone();
            }
            pos += s.len;
        }
        self.base.clone()
    }

    /// Attribute für Text, der den Bereich `a..b` ersetzt: erstes ersetztes Zeichen, sonst das
    /// Zeichen vor dem Caret, am Anfang das erste Zeichen, bei leerem Text die Grundattribute.
    fn insert_attrs(&self, a: usize, b: usize) -> TextAttrs {
        let n = self.text.chars().count();
        if a < b {
            self.attrs_of_char(a)
        } else if a > 0 {
            self.attrs_of_char(a - 1)
        } else if n > 0 {
            self.attrs_of_char(0)
        } else {
            self.base.clone()
        }
    }

    /// Attribute am Anfang der Markierung bzw. am Caret.
    pub fn current_attrs(&self) -> TextAttrs {
        let (a, b) = self.selection();
        self.insert_attrs(a, b)
    }

    /// Ersetzt die Zeichen `a..b` durch `s` (Formatierung wie [`Self::insert_attrs`]).
    fn splice(&mut self, a: usize, b: usize, s: &str) {
        let attrs = self.insert_attrs(a, b);
        self.sanitize();
        let (ba, bb) = (byte_at(&self.text, a), byte_at(&self.text, b));
        self.text.replace_range(ba..bb, s);
        let k1 = split_at(&mut self.spans, a);
        let k2 = split_at(&mut self.spans, b);
        self.spans.drain(k1..k2);
        let n = char_len(s);
        if n > 0 {
            self.spans.insert(k1, EditSpan { len: n, attrs });
        }
        normalize(&mut self.spans);
    }

    /// Abschnitte passend zur Zeichenzahl halten (Absicherung).
    fn sanitize(&mut self) {
        let n = char_len(&self.text);
        let total: usize = self.spans.iter().map(|s| s.len).sum();
        if total != n {
            self.spans = if n > 0 { vec![EditSpan { len: n, attrs: self.base.clone() }] } else { Vec::new() };
        }
    }

    /// Attribute anwenden: mit Markierung auf die Markierung (Absatzattribute auf die berührten
    /// Absätze), ohne Markierung auf den ganzen Text und die Grundattribute.
    fn apply_attrs(&mut self, patch: &TextAttrs) {
        self.sanitize();
        let cs: Vec<char> = self.text.chars().collect();
        let n = cs.len();
        let (mut a, mut b) = self.selection();
        let whole = a == b;
        if whole {
            a = 0;
            b = n;
        }
        let (ch, para) = para_part(patch);
        let mut pa = a;
        while pa > 0 && cs[pa - 1] != '\r' {
            pa -= 1;
        }
        let mut pb = b;
        if pb > pa && pb <= n && cs[pb - 1] == '\r' {
            // Markierung endet direkt hinter einem Absatzende: dieser Absatz zählt nicht mehr dazu
        } else {
            while pb < n && cs[pb] != '\r' {
                pb += 1;
            }
            if pb < n {
                pb += 1;
            }
        }
        for (from, to, p) in [(a, b, &ch), (pa, pb, &para)] {
            if from >= to || *p == TextAttrs::default() {
                continue;
            }
            let k1 = split_at(&mut self.spans, from);
            let k2 = split_at(&mut self.spans, to);
            for s in &mut self.spans[k1..k2] {
                s.attrs.apply(p);
            }
        }
        normalize(&mut self.spans);
        if whole || (a == 0 && b == n) {
            self.base.apply(patch);
            self.sync_base();
        }
    }

    /// Schrift/Größe/Farbe aus den Grundattributen spiegeln.
    fn sync_base(&mut self) {
        if let Some(f) = &self.base.font {
            self.font = f.clone();
        }
        if let Some(s) = self.base.size {
            self.size = s;
        }
        if let Some(c) = self.base.color {
            self.color = Argb(c);
        }
    }

    /// Läufe des Editortexts (leerer Text: ein leerer Lauf mit den Grundattributen).
    pub fn runs(&self) -> Vec<TextRun> {
        self.runs_of(&self.text)
    }

    fn runs_of(&self, text: &str) -> Vec<TextRun> {
        let cs: Vec<char> = text.chars().collect();
        let total: usize = self.spans.iter().map(|s| s.len).sum();
        if cs.is_empty() || total != cs.len() {
            let mut a = self.base.clone();
            a.ps_font = None;
            return vec![TextRun { attrs: a, text: text.to_string(), extra: Extra::default() }];
        }
        let mut out = Vec::new();
        let mut pos = 0;
        for s in &self.spans {
            out.push(TextRun { attrs: s.attrs.clone(), text: cs[pos..pos + s.len].iter().collect(), extra: Extra::default() });
            pos += s.len;
        }
        out
    }

    /// Läufe mit eingeblendeter IME-Komposition (Formatierung des Zeichens vor dem Caret).
    fn display_runs(&self) -> Vec<TextRun> {
        match &self.preedit {
            Some(p) if !p.is_empty() => {
                let mut v = self.clone();
                v.preedit = None;
                v.splice(self.caret, self.caret, p);
                v.runs()
            }
            _ => self.runs(),
        }
    }

    /// Element für die Vorschau (fp-render): vorhandenes Textobjekt mit neuem Text bzw. neues
    /// Textobjekt wie `draw.text`. Die IME-Komposition wird am Caret eingefügt.
    pub fn preview_element(&self, doc: &Document) -> Option<Element> {
        let runs = self.display_runs();
        if let Some(id) = self.target {
            let e = find_element(doc, id)?;
            let mut e = (*e).clone();
            if let ElementKind::Text(t) = &mut e.kind {
                apply_runs(t, runs);
            }
            return Some(e);
        }
        let mut ids = IdGen::default();
        let mut e = Element::new_text(&mut ids, &self.display_text(), &self.font, self.size, self.color, self.baseline_anchor());
        if let ElementKind::Text(t) = &mut e.kind {
            apply_runs(t, runs);
            if let Some(w) = self.width {
                t.frame_type = Some(1);
                t.bounds = Rect::new(self.anchor.x, self.anchor.y, self.anchor.x + w, self.anchor.y + self.size * 1.2);
                t.anchor_rect = Some(t.bounds);
            }
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
    spans: Vec<EditSpan>,
    base: TextAttrs,
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
        let runs = serde_json::to_value(v.runs()).unwrap_or_default();
        if let Some(id) = v.target {
            let old = find_element(ctx.doc, id).and_then(|e| match &e.kind {
                ElementKind::Text(t) => Some((t.plain_text(), spans_of(t))),
                _ => None,
            });
            if old.as_ref().is_none_or(|(s, sp)| *s != v.text || *sp != v.spans) {
                let mut p = json!({"id": id, "text": v.text});
                if !v.text.is_empty() {
                    p["runs"] = runs;
                }
                resp.commands.push(Invocation::new("text.setText", p));
            }
        } else if !v.text.is_empty() {
            let mut p = json!({"text": v.text, "font": v.font, "size": v.size, "runs": runs});
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
        v.splice(a, b, "");
        v.caret = a;
        v.sel_anchor = a;
        true
    }

    fn insert(v: &mut TextEditView, s: &str) {
        // Zeilenumbrüche als Absatzende `\r` wie das Original
        let clean: String = s.replace("\r\n", "\r").replace('\n', "\r").chars().filter(|c| *c == '\r' || *c == '\t' || !c.is_control()).collect();
        let (a, b) = v.selection();
        v.splice(a, b, &clean);
        v.caret = a + char_len(&clean);
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
            let spans = spans_of(t);
            let mut base = spans.first().map(|s| s.attrs.clone()).unwrap_or_else(|| {
                let mut a = t.initial.clone();
                a.ps_font = None;
                a.kerning = None;
                a
            });
            if base.font.is_none() {
                base.font = Some(ctx.options.text_font.clone());
            }
            if base.size.is_none() {
                base.size = Some(ctx.options.text_size);
            }
            if base.color.is_none() {
                base.color = Some(t.attrs.fill_color.0);
            }
            let mut v = TextEditView {
                target: Some(e.id.0),
                anchor: Point::new(t.bounds.left, t.bounds.top),
                width: None,
                text,
                caret: n,
                sel_anchor: n,
                preedit: None,
                font: String::new(),
                size: 0.0,
                color: ctx.fill_color,
                spans,
                base,
            };
            v.sync_base();
            v.sanitize();
            self.state = TextState::Editing(Box::new(v));
        } else {
            self.state = TextState::Pressed { start: p };
        }
    }

    /// Zustand des Editors für den Editor-Verlauf: (Sitzungskennung, Text, Caret, Anker, Format).
    fn snap(&self) -> Option<EditSnap> {
        match &self.state {
            TextState::Editing(v) => Some(EditSnap {
                key: (v.target, v.anchor.x.to_bits(), v.anchor.y.to_bits()),
                text: v.text.clone(),
                caret: v.caret,
                anchor: v.sel_anchor,
                spans: v.spans.clone(),
                base: v.base.clone(),
            }),
            _ => None,
        }
    }

    /// Strg+Z / Strg+Y im Editor: Texteingabe bzw. Formatierung zurücknehmen bzw. wiederholen
    /// (wie im Original zuerst im Editor, nicht im Dokumentverlauf).
    fn undo_redo(&mut self, undo: bool) -> ToolResponse {
        let Some(cur) = self.snap() else { return ToolResponse::default() };
        let (from, to) = if undo { (&mut self.undo, &mut self.redo) } else { (&mut self.redo, &mut self.undo) };
        if let Some(s) = from.pop() {
            to.push(cur);
            if let TextState::Editing(v) = &mut self.state {
                v.text = s.text;
                v.caret = s.caret;
                v.sel_anchor = s.anchor;
                v.spans = s.spans;
                v.base = s.base;
                v.preedit = None;
                v.sync_base();
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
                    let mut base = new_attrs(&ctx.options.text_font, ctx.options.text_size, ctx.fill_color);
                    base.color = Some(ctx.fill_color.0);
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
                        spans: Vec::new(),
                        base,
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
                        EditOp::Attrs(a) => v.apply_attrs(a),
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
                            v.splice(from, v.caret, "");
                            v.caret = from;
                            v.sel_anchor = from;
                        }
                    }
                    ToolKey::Delete => {
                        if !Self::delete_selection(v) && v.caret < n {
                            let to = if ev.mods.ctrl { Self::word_right(v) } else { v.caret + 1 };
                            v.splice(v.caret, to, "");
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
                if b.text != a.text || b.spans != a.spans || b.base != a.base {
                    // aufeinanderfolgendes Tippen ist ein Schritt; Löschen, Einfügen, Umbruch und
                    // jede Formatierung je ein Schritt
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

    fn text_edit_attrs(&self) -> Option<TextAttrs> {
        match &self.state {
            TextState::Editing(v) => Some(v.current_attrs()),
            _ => None,
        }
    }
}
