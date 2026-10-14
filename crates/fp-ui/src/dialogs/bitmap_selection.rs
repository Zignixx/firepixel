//! Auswählen ▸ Bitmap-Auswahl speichern… / wiederherstellen… / löschen… (re\19-kurven-tonwert.md
//! §5, KV-3).
//!
//! * Speichern (§5.2): Gruppe „Ziel“ mit Dokument (nur offene Dokumente gleicher Größe), Auswahl
//!   (erster Eintrag „Neu“, dann die gespeicherten Namen) und Name (Vorgabe „Standard“, nur bei
//!   „Neu“ aktiv); Gruppe „Vorgang“ mit vier Optionsfeldern, die drei Verknüpfungen nur bei einer
//!   vorhandenen Auswahl. Gleicher Name bei „Neu“ → Rückfrage OK/Abbrechen (Abbrechen kehrt in den
//!   Dialog zurück).
//! * Wiederherstellen (§5.3): Quelle mit Dokument, Auswahl und „Umkehren“; Vorgang mit vier
//!   Optionsfeldern, Hinzufügen/Abziehen/Schnittmenge nur bei bestehender Pixelauswahl.
//! * Löschen (§5.4): Dokument und Auswahl; OK löscht ohne Rückfrage.
//!
//! Nichts wird gemerkt (§5.5). Die Arbeit machen die Befehle `select.saveBitmapSelection`,
//! `select.restoreBitmapSelection` und `select.deleteBitmapSelection` (je ein Verlaufsschritt).

use super::{button_row, dialog_frame, fit_label, group_box, push_button, t, DialogClose};
use crate::app_state::AppState;
use crate::inspector::fields;
use egui::{vec2, Id, Pos2, Rect};
use fp_engine::session::DocId;
use serde_json::{json, Value};

/// Art des Dialogs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Save,
    Restore,
    Delete,
}

/// Verknüpfungen in der Reihenfolge der Optionsfelder.
pub const OPS: [&str; 4] = ["replace", "add", "subtract", "intersect"];

/// Zustand eines der drei Dialoge.
#[derive(Clone, Debug, PartialEq)]
pub struct BitmapSelectionDialog {
    pub kind: Kind,
    /// Dokumentliste (ID, Titel).
    pub docs: Vec<(DocId, String)>,
    /// Gewähltes Dokument (Index in `docs`).
    pub doc: usize,
    /// Namen der gespeicherten Auswahlen des gewählten Dokuments (aktive Seite).
    pub names: Vec<String>,
    /// Gewählte Auswahl. Speichern: 0 = „Neu“, sonst `names[sel − 1]`; sonst Index in `names`.
    pub sel: usize,
    /// Name (nur Speichern mit „Neu“).
    pub name: String,
    /// Gewählter Vorgang (Index in [`OPS`]).
    pub op: usize,
    /// „Umkehren“ (nur Wiederherstellen).
    pub invert: bool,
    /// Hat das aktuelle Dokument eine Pixelauswahl?
    pub has_pixels: bool,
    /// Rückfrage „gleichnamige Auswahl ersetzen?“ offen.
    pub confirm: bool,
    /// Live-Anzeige beim Wiederherstellen (BA-050): Auswahl des aktiven Dokuments vor dem Dialog …
    pub live_orig: Option<fp_engine::Selection>,
    /// … und die Parameter der zuletzt angezeigten Auswahl.
    pub live_key: Option<Value>,
}

/// Befehle dieses Moduls.
pub fn handles(name: &str) -> bool {
    matches!(name, "select.saveBitmapSelection" | "select.restoreBitmapSelection" | "select.deleteBitmapSelection")
}

/// Öffnet den Dialog zum Befehl (`false`, wenn kein Dokument offen ist).
pub fn open(st: &mut AppState, name: &str) -> bool {
    let kind = match name {
        "select.saveBitmapSelection" => Kind::Save,
        "select.restoreBitmapSelection" => Kind::Restore,
        "select.deleteBitmapSelection" => Kind::Delete,
        _ => return false,
    };
    match BitmapSelectionDialog::new(st, kind) {
        Some(d) => {
            st.dialogs.bitmap_selection = Some(d);
            true
        }
        None => false,
    }
}

fn canvas_size(d: &fp_engine::DocumentState) -> (i32, i32) {
    d.doc.pages.get(d.page).map(|p| (p.canvas.width.round() as i32, p.canvas.height.round() as i32)).unwrap_or_default()
}

/// Namen der gespeicherten Auswahlen auf der aktiven Seite eines Dokuments.
pub fn saved_names(st: &AppState, id: DocId) -> Vec<String> {
    st.session
        .document(id)
        .and_then(|d| d.doc.pages.get(d.page))
        .and_then(|p| p.saved_selections.as_ref())
        .map(|v| v.iter().enumerate().map(|(i, s)| s.name.clone().unwrap_or_else(|| fp_i18n::tr!("dlg-selection-default-name", n = i + 1))).collect())
        .unwrap_or_default()
}

impl BitmapSelectionDialog {
    /// Neuer Dialog mit den Vorgaben des Originals (nichts gemerkt): aktuelles Dokument, „Neu“ bzw.
    /// die erste Auswahl, Name „Standard“, „Neue Auswahl“, Umkehren aus.
    pub fn new(st: &AppState, kind: Kind) -> Option<BitmapSelectionDialog> {
        let a = st.session.active()?;
        let size = canvas_size(a);
        let mut docs: Vec<(DocId, String)> = Vec::new();
        for d in st.session.documents() {
            if d.id != a.id && canvas_size(d) != size {
                continue;
            }
            // Quelle zum Wiederherstellen/Löschen: nur Dokumente mit gespeicherten Auswahlen (A)
            if kind != Kind::Save && d.id != a.id && saved_names(st, d.id).is_empty() {
                continue;
            }
            docs.push((d.id, d.title.clone()));
        }
        let doc = docs.iter().position(|(id, _)| *id == a.id).unwrap_or(0);
        let mut d = BitmapSelectionDialog {
            kind,
            docs,
            doc,
            names: Vec::new(),
            sel: 0,
            name: t("sel-saved-default-name"),
            op: 0,
            invert: false,
            has_pixels: a.selection.has_pixels(),
            confirm: false,
            live_orig: None,
            live_key: None,
        };
        d.set_doc(st, doc);
        Some(d)
    }

    /// Dokument wählen: die Auswahl-Liste wird neu gefüllt (Speichern: wieder „Neu“).
    pub fn set_doc(&mut self, st: &AppState, i: usize) {
        self.doc = i.min(self.docs.len().saturating_sub(1));
        self.names = self.docs.get(self.doc).map(|(id, _)| saved_names(st, *id)).unwrap_or_default();
        self.set_selection(0);
    }

    /// Auswahl wählen (Speichern: 0 = „Neu“). Ein Vorgang, der nicht mehr erlaubt ist, springt
    /// auf den ersten zurück.
    pub fn set_selection(&mut self, i: usize) {
        let n = if self.kind == Kind::Save { self.names.len() + 1 } else { self.names.len() };
        self.sel = i.min(n.saturating_sub(1));
        if !self.op_enabled(self.op) {
            self.op = 0;
        }
    }

    /// Einträge der Auswahl-Liste.
    pub fn selection_items(&self) -> Vec<String> {
        let mut v = Vec::new();
        if self.kind == Kind::Save {
            v.push(t("bsel-new"));
        }
        v.extend(self.names.iter().cloned());
        v
    }

    /// Ist das Namensfeld aktiv (Speichern mit „Neu“)?
    pub fn name_enabled(&self) -> bool {
        self.kind == Kind::Save && self.sel == 0
    }

    /// Ist das Optionsfeld `k` aktiv? Speichern: Verknüpfungen nur in eine vorhandene Auswahl;
    /// Wiederherstellen: nur bei bestehender Pixelauswahl.
    pub fn op_enabled(&self, k: usize) -> bool {
        k == 0
            || match self.kind {
                Kind::Save => self.sel > 0,
                Kind::Restore => self.has_pixels,
                Kind::Delete => false,
            }
    }

    /// Beschriftungen der Optionsfelder (Speichern in eine vorhandene Auswahl: „Auswahl ersetzen“).
    pub fn op_labels(&self) -> [String; 4] {
        let first = if self.kind == Kind::Save && self.sel > 0 { t("bsel-op-replace") } else { t("bsel-op-new") };
        [first, t("bsel-op-add"), t("bsel-op-subtract"), t("bsel-op-intersect")]
    }

    /// Ziel- bzw. Quelldokument.
    pub fn doc_id(&self) -> Option<DocId> {
        self.docs.get(self.doc).map(|(id, _)| *id)
    }

    /// Braucht OK erst die Rückfrage (Speichern unter „Neu“ mit einem vorhandenen Namen)?
    pub fn needs_confirm(&self) -> bool {
        self.kind == Kind::Save && self.sel == 0 && self.names.iter().any(|n| *n == self.name)
    }

    /// Befehl und Parameter für OK.
    pub fn invocation(&self) -> (&'static str, Value) {
        let doc = self.doc_id().map(|d| d.0).unwrap_or(0);
        let op = OPS[if self.op_enabled(self.op) { self.op } else { 0 }];
        match self.kind {
            Kind::Save => {
                let (name, mode) = if self.sel == 0 { (self.name.clone(), "replace") } else { (self.names[self.sel - 1].clone(), op) };
                ("select.saveBitmapSelection", json!({"name": name, "mode": mode, "document": doc}))
            }
            Kind::Restore => ("select.restoreBitmapSelection", json!({"index": self.sel, "mode": op, "invert": self.invert, "document": doc})),
            Kind::Delete => ("select.deleteBitmapSelection", json!({"index": self.sel, "document": doc})),
        }
    }
}

/// Ergebnis eines Bilds.
enum Outcome {
    Ok,
    Cancel,
}

fn rr(o: Pos2, x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_min_size(o + vec2(x, y), vec2(w, h))
}

pub fn show(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.dialogs.bitmap_selection.clone() else { return };
    let (title, mut size, id) = match d.kind {
        Kind::Save => (t("dlg-title-save-selection"), vec2(296.0, 274.0), "fp-bsel-save"),
        Kind::Restore => (t("dlg-title-restore-selection"), vec2(297.0, 295.0), "fp-bsel-restore"),
        Kind::Delete => (t("dlg-title-delete-selection"), vec2(284.0, 125.0), "fp-bsel-delete"),
    };
    let (ok_l, cancel_l) = (t("btn-ok"), t("btn-cancel"));
    let docs: Vec<String> = d.docs.iter().map(|(_, n)| n.clone()).collect();
    let items = d.selection_items();
    let mut outcome = None;
    let mut new_doc = None;
    let mut new_sel = None;
    // Lage der Steuerelemente nach den Vorlagen (§5.2–§5.4, Fensterkoordinaten ohne Titelleiste)
    let (lx, cx, cw, doc_y, sel_y) = match d.kind {
        Kind::Save => (24.0, 92.0, 188.0, 31.0, 57.0),
        Kind::Restore => (24.0, 99.0, 182.0, 31.0, 70.0),
        Kind::Delete => (14.0, 89.0, 185.0, 24.0, 52.0),
    };
    // Längere Beschriftungen (Übersetzungen, Testsprache) schieben die Felder nach rechts und
    // verbreitern den Dialog; im Deutschen bleibt die Lage des Originals (Verschiebung 0).
    let mut lbls = vec![t("bsel-lbl-document"), t("bsel-lbl-selection")];
    if d.kind == Kind::Save {
        lbls.push(t("bsel-lbl-name"));
    }
    let need = super::max_text_w(ctx, &super::refs(&lbls)).ceil() + 4.0;
    let dx = (lx + need - cx).max(0.0);
    let cx = cx + dx;
    size.x += dx;
    let r = dialog_frame(ctx, id, &title, size, |ui, body| {
        let o = body.min;
        match d.kind {
            Kind::Save => group_box(ui, rr(o, 14.0, 9.0, 275.0 + dx, 106.0), &t("bsel-group-target")),
            Kind::Restore => group_box(ui, rr(o, 14.0, 9.0, 276.0 + dx, 117.0), &t("bsel-group-source")),
            Kind::Delete => {}
        }
        fit_label(ui, rr(o, lx, doc_y + 1.0, cx - lx - 4.0, 20.0), &t("bsel-lbl-document"), true);
        if let Some(i) = fields::combo(ui, ("bsel-doc", id), rr(o, cx, doc_y, cw, 21.0), &super::refs(&docs), Some(d.doc), !docs.is_empty()) {
            new_doc = Some(i);
        }
        fit_label(ui, rr(o, lx, sel_y + 1.0, cx - lx - 4.0, 20.0), &t("bsel-lbl-selection"), true);
        if let Some(i) = fields::combo(ui, ("bsel-sel", id), rr(o, cx, sel_y, cw, 21.0), &super::refs(&items), Some(d.sel), !items.is_empty()) {
            new_sel = Some(i);
        }
        if d.kind == Kind::Save {
            fit_label(ui, rr(o, lx, 84.0, cx - lx - 4.0, 20.0), &t("bsel-lbl-name"), true);
            let enabled = d.name_enabled() && !d.confirm;
            ui.add_enabled_ui(enabled, |ui| {
                ui.put(rr(o, cx, 83.0, cw, 23.0), egui::TextEdit::singleline(&mut d.name).id(Id::new("bsel-name")));
            });
        }
        if d.kind == Kind::Restore
            && let Some(v) = fields::checkbox(ui, "bsel-invert", rr(o, 98.0 + dx, 99.0, 176.0, 16.0), &t("bsel-invert"), d.invert, true)
        {
            d.invert = v;
        }
        if d.kind != Kind::Delete {
            let (gy, ys): (f32, [f32; 4]) = if d.kind == Kind::Save { (130.0, [151.0, 169.0, 187.0, 204.0]) } else { (139.0, [164.0, 183.0, 203.0, 222.0]) };
            group_box(ui, rr(o, 14.0, gy, 275.0 + dx + if d.kind == Kind::Restore { 1.0 } else { 0.0 }, if d.kind == Kind::Save { 102.0 } else { 112.0 }), &t("bsel-group-operation"));
            let labels = d.op_labels();
            for (k, y) in ys.into_iter().enumerate() {
                if fields::radio(ui, ("bsel-op", id, k), rr(o, 39.0, y, 237.0 + dx, 16.0), &labels[k], d.op == k, d.op_enabled(k)) && d.op_enabled(k) {
                    d.op = k;
                }
            }
        }
        let b = button_row(ui.ctx(), body.right() - 9.0, body.bottom() - 34.0, &[&ok_l, &cancel_l], 75.0, 23.0, 13.0);
        let can_ok = !d.confirm && (d.kind == Kind::Save || !d.names.is_empty());
        if push_button(ui, b[0], &ok_l, true, can_ok) {
            outcome = Some(Outcome::Ok);
        }
        if push_button(ui, b[1], &cancel_l, false, !d.confirm) {
            outcome = Some(Outcome::Cancel);
        }
    });
    if let Some(i) = new_doc {
        d.set_doc(st, i);
    }
    if let Some(i) = new_sel {
        d.set_selection(i);
    }
    if d.kind == Kind::Restore {
        live_update(st, &mut d);
    }
    // Rückfrage über dem Dialog: OK ersetzt, Abbrechen kehrt in den offenen Dialog zurück
    if d.confirm {
        let text = t("bsel-confirm-replace");
        let mut answer = None;
        let w = 360.0;
        let th = super::wrapped_h(ctx, &text, w - 32.0);
        let cr = dialog_frame(ctx, "fp-bsel-confirm", "Firepixel", vec2(w, (16.0 + th + 16.0 + 40.0).ceil()), |ui, body| {
            super::text_block(ui, Rect::from_min_size(body.min + vec2(16.0, 16.0), vec2(w - 32.0, th + 4.0)), text.as_str());
            let b = button_row(ui.ctx(), body.right() - 12.0, body.bottom() - 34.0, &[&ok_l, &cancel_l], 75.0, 23.0, 8.0);
            if push_button(ui, b[0], &ok_l, true, true) {
                answer = Some(true);
            }
            if push_button(ui, b[1], &cancel_l, false, true) {
                answer = Some(false);
            }
        });
        let answer = answer.or((cr == DialogClose::Cancelled).then_some(false));
        match answer {
            Some(true) => {
                d.confirm = false;
                outcome = Some(Outcome::Ok);
                return finish(st, d, outcome, true);
            }
            Some(false) => d.confirm = false,
            None => {}
        }
        st.dialogs.bitmap_selection = Some(d);
        return;
    }
    let outcome = outcome.or((r == DialogClose::Cancelled).then_some(Outcome::Cancel));
    finish(st, d, outcome, false);
}

fn finish(st: &mut AppState, mut d: BitmapSelectionDialog, outcome: Option<Outcome>, confirmed: bool) {
    match outcome {
        Some(Outcome::Ok) => {
            if !confirmed && d.needs_confirm() {
                d.confirm = true;
                st.dialogs.bitmap_selection = Some(d);
                return;
            }
            live_revert(st, &mut d);
            let (cmd, p) = d.invocation();
            st.dialogs.bitmap_selection = None;
            st.run(cmd, p);
        }
        Some(Outcome::Cancel) => {
            live_revert(st, &mut d);
            st.dialogs.bitmap_selection = None;
        }
        None => st.dialogs.bitmap_selection = Some(d),
    }
}

/// Live-Anzeige der entstehenden Auswahl beim Wiederherstellen (re\19 §5.3, BA-050): Bei jeder
/// Änderung von Dokument, Auswahl, Vorgang oder „Umkehren“ zeigt die Leinwand das Ergebnis, ohne
/// Verlaufsschritt. OK nimmt die Anzeige zurück und führt den Befehl aus (ein Schritt), Abbrechen
/// stellt die Auswahl von vorher wieder her.
pub fn live_update(st: &mut AppState, d: &mut BitmapSelectionDialog) {
    if d.names.is_empty() {
        live_revert(st, d);
        return;
    }
    let (_, p) = d.invocation();
    if d.live_key.as_ref() == Some(&p) {
        return;
    }
    let Some(active) = st.session.active_mut() else { return };
    let orig = d.live_orig.get_or_insert_with(|| active.selection.clone()).clone();
    active.selection = orig;
    if let Ok(px) = fp_engine::commands::restore_bitmap_selection_preview(&st.session, p.clone())
        && let Some(active) = st.session.active_mut()
    {
        active.selection.pixels = px;
    }
    d.live_key = Some(p);
}

/// Nimmt die Live-Anzeige zurück (Auswahl wie vor dem Dialog).
pub fn live_revert(st: &mut AppState, d: &mut BitmapSelectionDialog) {
    if let Some(orig) = d.live_orig.take()
        && let Some(active) = st.session.active_mut()
    {
        active.selection = orig;
    }
    d.live_key = None;
}
