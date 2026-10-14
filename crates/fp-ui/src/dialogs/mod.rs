//! Dialoge. Phase 3: „Neues Dokument“ (Vorlage 5961), Ungespeichert-Abfrage, Voreinstellungen
//! (Grundgerüst mit Design-Auswahl), „Über Firepixel“ und Meldungen. Phase 4 ergänzt die übrigen
//! Dialoge aus `re\bedienfelder\dialoge.json` nach demselben Muster ([`dialog_frame`] + eigenes
//! Zustands-Struct in [`Dialogs`]).

mod about;
pub mod access;
pub mod ai_progress;
pub mod catalog;
pub mod copy_html;
pub mod doc_dialogs;
pub mod effects;
pub mod export;
pub mod export_options;
pub mod file_info;
pub mod export_wizard;
pub mod html_setup;
pub mod image_preview;
pub mod form;
pub mod levels;
pub mod levels_common;
pub mod marquee;
pub mod curves;
pub mod bitmap_selection;
pub mod brush_edit;
pub mod menu_dialogs;
pub mod misc;
pub mod models;
mod new_document;
pub mod panel_menus;
pub mod path_dialogs;
pub mod popup_menu;
pub mod script_prompt;
pub(crate) mod preferences;
pub mod shortcuts;
pub mod spelling;
pub mod spelling_setup;
pub mod symbol_anim;
pub mod trace;
pub mod warning;
mod unsaved;

pub use misc::Confirm;
pub use new_document::NewDocumentDialog;
pub use preferences::{general_group_titles, PreferencesDialog, CATEGORIES as PREFERENCE_CATEGORIES};
pub use unsaved::{without_last_extension, PendingClose, UnsavedFlow};

use crate::app_state::AppState;
use crate::theme::Theme;
use egui::{pos2, vec2, Align2, Id, Rect, Sense, Stroke, StrokeKind, Ui};
pub use fp_i18n::t;

/// Offene Dialoge.
#[derive(Default)]
pub struct Dialogs {
    pub new_document: Option<NewDocumentDialog>,
    pub unsaved: Option<UnsavedFlow>,
    pub preferences: Option<PreferencesDialog>,
    pub about: bool,
    /// Meldung (Titel, Text).
    pub message: Option<(String, String)>,
    /// Name für „Arbeitsbereich speichern“.
    pub save_workspace: Option<String>,
    /// Formular-Dialog (Filter, Effekte, Auswahl, Pfad, Raster, Hilfslinien, …).
    pub form: Option<form::FormDialog>,
    /// Formular, das beim nächsten Bild geöffnet wird (aus Kontexten ohne `AppState`).
    pub pending_form: Option<PendingForm>,
    pub canvas_size: Option<doc_dialogs::CanvasSizeDialog>,
    pub image_size: Option<doc_dialogs::ImageSizeDialog>,
    pub numeric_transform: Option<doc_dialogs::NumericTransformDialog>,
    pub levels: Option<levels::LevelsDialog>,
    pub curves: Option<curves::CurvesDialog>,
    /// Bitmap-Auswahl speichern/wiederherstellen/löschen (KV-3).
    pub bitmap_selection: Option<bitmap_selection::BitmapSelectionDialog>,
    pub export: Option<export::ExportDialog>,
    /// Bildvorschau (BV-1).
    pub preview: Option<image_preview::ImagePreview>,
    pub shortcuts: Option<shortcuts::ShortcutsDialog>,
    pub confirm: Option<Confirm>,
    pub text_editor: Option<misc::TextEditorDialog>,
    /// Text für „HTML-Segment bearbeiten“.
    pub html_slice: Option<String>,
    /// Auswahl in den Bedienfeldern (für die Befehle der Bedienfeldmenüs).
    pub panel_sel: panel_menus::PanelSelection,
    /// Text, der beim nächsten Bild in die Zwischenablage kommt.
    pub clipboard: Option<String>,
    /// „Gespeicherte Befehle verwalten“.
    pub saved_commands: Option<menu_dialogs::SavedCommandsDialog>,
    /// „HTML-Einrichtung“ (E3).
    pub html_setup: Option<html_setup::HtmlSetupDialog>,
    /// Assistent "HTML-Code kopieren" (TW-22).
    pub copy_html: Option<copy_html::CopyHtmlWizard>,
    /// „Exportassistent“ (E3).
    pub wizard: Option<export_wizard::ExportWizard>,
    /// Rechtschreibprüfung.
    pub spelling: Option<spelling::SpellingDialog>,
    /// „Rechtschreibprüfung einrichten“ (D24, DLG-10).
    pub spelling_setup: Option<spelling_setup::SpellingSetupDialog>,
    /// „Datei-Info“ mit den Metadaten (D10, DLG-10).
    pub file_info: Option<file_info::FileInfoDialog>,
    /// Pop-up-Menü-Editor.
    pub popup_editor: Option<popup_menu::PopupEditor>,
    /// KI-Auswahl: Hintergrundrechnung, Dialoge „KI-Modell fehlt“ und „KI-Modelle verwalten“ (Agent KI).
    pub ai: models::AiDialogs,
    /// „Bitmap nachzeichnen…“ mit Live-Vorschau (VT-2).
    pub trace: Option<trace::TraceDialog>,
    /// Warnung „Befehl ohne passendes Ziel“ mit „Nicht mehr anzeigen“ (FV-5).
    pub warning: Option<warning::WarningDialog>,
    /// Rückfrage „konvertiert Vektoren in Bitmaps“ mit OK bestätigt: der nächste Filteraufruf fragt nicht (TW-21).
    pub filter_vectors_ok: bool,
    /// „Pinselstrich bearbeiten“ mit Registerkarten Optionen/Form/Empfindlichkeit (PINSEL-1).
    pub brush_edit: Option<brush_edit::BrushEditDialog>,
    /// Eingabeaufforderung `prompt()` eines Befehlsskripts (CT-1b MN-5).
    pub script_prompt: Option<script_prompt::ScriptPrompt>,
}

/// Verzögert zu öffnendes Formular.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PendingForm {
    StrokeOptions,
    /// „Pinselstrich bearbeiten“ für die Werkzeugvorgabe (Werkzeug-Inspektor).
    StrokeOptionsTool,
    NewStyle,
    StatusText,
}

impl Dialogs {
    /// Ist irgendein modaler Dialog offen (Tastenkürzel sperren)?
    pub fn any_open(&self) -> bool {
        self.new_document.is_some()
            || self.unsaved.is_some()
            || self.preferences.is_some()
            || self.about
            || self.message.is_some()
            || self.save_workspace.is_some()
            || self.form.is_some()
            || self.canvas_size.is_some()
            || self.image_size.is_some()
            || self.numeric_transform.is_some()
            || self.levels.is_some()
            || self.curves.is_some()
            || self.bitmap_selection.is_some()
            || self.export.is_some()
            || self.preview.is_some()
            || self.shortcuts.is_some()
            || self.confirm.is_some()
            || self.text_editor.is_some()
            || self.html_slice.is_some()
            || self.saved_commands.is_some()
            || self.html_setup.is_some()
            || self.copy_html.is_some()
            || self.wizard.is_some()
            || self.spelling.is_some()
            || self.spelling_setup.is_some()
            || self.file_info.is_some()
            || self.popup_editor.is_some()
            || self.ai.modal_open()
            || self.trace.is_some()
            || self.warning.is_some()
            || self.brush_edit.is_some()
            || self.script_prompt.is_some()
            || crate::file_dialog::is_open() // AU: offener Datei-Dialog sperrt wie ein modaler Dialog
    }

    /// „Strich bearbeiten“ öffnen (beim nächsten Bild, braucht die Auswahl).
    pub fn open_stroke_options(&mut self, _with_document: bool) {
        self.pending_form = Some(PendingForm::StrokeOptions);
    }
    /// „Pinselstrich bearbeiten“ für den Pinsel der Werkzeugvorgabe öffnen (auch mit Auswahl).
    pub fn open_stroke_options_for_tool(&mut self) {
        self.pending_form = Some(PendingForm::StrokeOptionsTool);
    }
    /// „Neuer Stil“ öffnen.
    pub fn open_new_style(&mut self) {
        self.pending_form = Some(PendingForm::NewStyle);
    }
    /// „Statusleistentext festlegen“ (Verhalten) öffnen.
    pub fn open_status_text(&mut self) {
        self.pending_form = Some(PendingForm::StatusText);
    }
    /// „HTML-Segment bearbeiten“ öffnen.
    pub fn open_html_slice(&mut self, html: String) {
        self.html_slice = Some(html);
    }
}

/// Öffnet den Dialog zu einem Befehl ohne Parameter; `true`, wenn ein Dialog übernommen hat.
pub fn open_for_command(st: &mut AppState, name: &str) -> bool {
    match name {
        "modify.canvas.canvasSize" => st.dialogs.canvas_size = doc_dialogs::CanvasSizeDialog::new(st),
        "modify.canvas.imageSize" => st.dialogs.image_size = doc_dialogs::ImageSizeDialog::new(st),
        "modify.transform.numericTransform" => st.dialogs.numeric_transform = doc_dialogs::NumericTransformDialog::new(st),
        "filter.adjustColor.levels" => st.dialogs.levels = Some(levels::LevelsDialog::new(st, false)),
        "filter.adjustColor.curves" => st.dialogs.curves = Some(curves::CurvesDialog::new(st, false)),
        "file.export" => {
            // Bereich des Werkzeugs „Exportbereich“ übernehmen
            let area = st.session.active_id().and_then(|id| st.canvases.get_mut(&id)).and_then(|c| c.export_area.take());
            st.dialogs.export = export::ExportDialog::new(st).map(|mut d| {
                d.area = area;
                d
            });
        }
        "file.imagePreview" => st.dialogs.preview = image_preview::ImagePreview::new(st),
        "file.htmlSetup" => return html_setup::open(st),
        "file.exportWizard" | "panel.optimize.exportWizard" => return export_wizard::open(st),
        "edit.keyboardShortcuts" => st.dialogs.shortcuts = Some(shortcuts::ShortcutsDialog::new(st)),
        "text.editor" => st.dialogs.text_editor = misc::TextEditorDialog::new(st),
        "text.checkSpelling" => spelling::open_check(st),
        "modify.popUpMenu.addPopUpMenu" | "modify.popUpMenu.editPopUpMenu" => {
            let edit = name.ends_with("editPopUpMenu");
            st.dialogs.popup_editor = popup_menu::PopupEditor::open(st, edit);
            if st.dialogs.popup_editor.is_none() {
                st.dialogs.message = Some((t("dlg-popup-title"), t("dlg-popup-need-selection")));
            }
        }
        "modify.traceBitmap" => return trace::open(st),
        "text.spellingSetup" => return spelling_setup::open(st),
        "file.fileInfo" => return file_info::open(st),
        _ if models::handles(name) => return models::open_command(st, name),
        _ if bitmap_selection::handles(name) => return bitmap_selection::open(st, name),
        _ if menu_dialogs::handles(name) => return menu_dialogs::open(st, name),
        _ if panel_menus::needs_ui(name) => return panel_menus::open(st, name),
        _ => match catalog::form_for(st, name) {
            Some(f) => st.dialogs.form = Some(catalog::with_doc_base(st, f)),
            None => return false,
        },
    }
    true
}

/// Umrechnung Dialogeinheiten → Pixel (11 §6: ≈ 1,5 px/DLU waagerecht, 1,625 px/DLU senkrecht).
pub fn dlu(x: f32, y: f32) -> egui::Vec2 {
    vec2(x * 1.5, y * 1.625)
}

/// Rechteck aus DLU-Angaben `[x, y, b, h]` relativ zu `origin`.
pub fn dlu_rect(origin: egui::Pos2, r: [f32; 4]) -> Rect {
    Rect::from_min_size(origin + dlu(r[0], r[1]), dlu(r[2], r[3]))
}

/// Ergebnis eines Dialograhmens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialogClose {
    Open,
    /// Über das Schließen-Kreuz bzw. Esc geschlossen.
    Cancelled,
}

/// Modaler Dialog im Windows-Stil: Titelleiste mit Schließen-Kreuz, Fläche in Rahmenfarbe,
/// Inhalt in einem Bereich fester Größe (`size` = Client-Bereich).
pub fn dialog_frame(ctx: &egui::Context, id: &str, title: &str, size: egui::Vec2, content: impl FnOnce(&mut Ui, Rect)) -> DialogClose {
    dialog_frame_ex(ctx, id, title, size, None, false, content)
}

/// Wie [`dialog_frame`], aber an Rändern und Ecken in der Größe änderbar; `min_size` ist die
/// Ausgangs- und Mindestgröße des Client-Bereichs.
pub fn dialog_frame_resizable(ctx: &egui::Context, id: &str, title: &str, min_size: egui::Vec2, content: impl FnOnce(&mut Ui, Rect)) -> DialogClose {
    dialog_frame_ex(ctx, id, title, min_size, None, true, content)
}

/// Wie [`dialog_frame_resizable`], aber mit eigener Ausgangsgröße beim Öffnen (`initial`, nie
/// kleiner als `min_size`), z. B. eine gemerkte, vom Nutzer gezogene Größe.
pub fn dialog_frame_resizable_sized(ctx: &egui::Context, id: &str, title: &str, min_size: egui::Vec2, initial: egui::Vec2, content: impl FnOnce(&mut Ui, Rect)) -> DialogClose {
    dialog_frame_ex(ctx, id, title, min_size, Some(initial), true, content)
}

/// Höhe der Titelleiste der Dialoge.
pub const DIALOG_TITLE_H: f32 = 28.0;
/// Mindestbreite der Titelleiste, die im Hauptfenster sichtbar bleibt.
pub const DIALOG_KEEP_VISIBLE: f32 = 80.0;

/// Lage und Client-Größe eines offenen Dialogs (egui-Speicher). Wie im Original wird ein Dialog
/// bei jedem Öffnen neu zentriert; die Lage gilt, solange er offen ist.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DialogGeom {
    pub pos: egui::Pos2,
    pub client: egui::Vec2,
    /// Durchlauf, in dem der Dialog zuletzt gezeichnet wurde (Lücke = neu geöffnet).
    pub pass: u64,
}

/// Fokus-Anforderung beim Öffnen: das erste Eingabefeld des Dialogs erhält den Fokus, Inhalt
/// ganz markiert (wie im Original, „800 ⇥ 600 ↵“ geht sofort).
fn autofocus_id(layer: egui::LayerId) -> Id {
    Id::new(("fp-dialog-autofocus", layer))
}

/// Von Eingabefeldern in Dialogen nach dem Anlegen aufzurufen: Ist der Dialog gerade geöffnet
/// worden und dies das erste Feld, bekommt es den Fokus mit ganz markiertem Inhalt.
pub fn autofocus_field(ui: &Ui, resp: &egui::Response, text_len: usize) {
    let key = autofocus_id(ui.layer_id());
    // im ersten Bild sind Felder mancher Dialoge noch gesperrt: die Anforderung gilt ein paar Bilder
    if ui.ctx().data(|d| d.get_temp::<u64>(key)).is_none() || !resp.enabled() {
        return;
    }
    ui.ctx().data_mut(|d| d.remove::<u64>(key));
    resp.request_focus();
    select_all(ui.ctx(), resp.id, text_len);
}

/// Wie [`autofocus_field`] für Bedienelemente ohne Text (Liste, Schieber; Original D26 Startfokus
/// auf der Liste, D42 auf dem Schieber): bekommt den Startfokus, wenn es als erstes danach fragt.
pub fn autofocus_widget(ui: &Ui, id: Id) {
    let key = autofocus_id(ui.layer_id());
    if !ui.is_enabled() || ui.ctx().data(|d| d.get_temp::<u64>(key)).is_none() {
        return;
    }
    ui.ctx().data_mut(|d| d.remove::<u64>(key));
    ui.memory_mut(|m| m.request_focus(id));
}

/// Per Tab erreichtes Feld: Inhalt ganz markieren (Windows-Verhalten); ein Mausklick setzt dagegen
/// nur die Schreibmarke.
pub fn select_on_tab(ui: &Ui, resp: &egui::Response, text_len: usize) {
    if resp.gained_focus() && !ui.input(|i| i.pointer.any_pressed() || i.pointer.any_down()) {
        select_all(ui.ctx(), resp.id, text_len);
    }
}

/// Markiert den ganzen Inhalt eines Textfelds.
pub fn select_all(ctx: &egui::Context, id: Id, text_len: usize) {
    let mut s = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
    s.cursor.set_char_range(Some(egui::text::CCursorRange::two(egui::text::CCursor::new(0), egui::text::CCursor::new(text_len))));
    s.store(ctx, id);
}

// ---------------------------------------------------------------------- Tab-Folge (PM-5, P-DLG1)

fn focus_ok_id(layer: egui::LayerId) -> Id {
    Id::new(("fp-dialog-focus-ok", layer))
}

/// Startfokus auf den Standardknopf statt auf das erste Feld (Original: Exportassistent „Weiter“,
/// HTML-Einrichtung, Selektives JPEG, Rechtschreibung einrichten). Vor den Feldern aufrufen.
pub fn autofocus_default_button(ui: &Ui) {
    let key = autofocus_id(ui.layer_id());
    if let Some(p) = ui.ctx().data(|d| d.get_temp::<u64>(key)) {
        ui.ctx().data_mut(|d| {
            d.remove::<u64>(key);
            d.insert_temp(focus_ok_id(ui.layer_id()), p);
        });
    }
}

/// Standardknopf: übernimmt den Startfokus, wenn der Dialog es verlangt
/// ([`autofocus_default_button`]) oder kein Feld ihn genommen hat (Dialoge ohne Eingabefeld und
/// Optionsfeld, wie ein Windows-Dialog; siehe Ende von `dialog_frame_ex`).
fn default_button_autofocus(ui: &Ui, resp: &egui::Response) {
    let layer = ui.layer_id();
    let asked = ui.ctx().data(|d| d.get_temp::<u64>(focus_ok_id(layer))).is_some();
    if resp.enabled() && asked {
        ui.ctx().data_mut(|d| {
            d.remove::<u64>(focus_ok_id(layer));
            d.remove::<u64>(autofocus_id(layer));
        });
        resp.request_focus();
    }
}

/// Optionsfeld-Gruppe wie unter Windows: **ein** Tab-Stopp (das gewählte Feld), Pfeiltasten wählen
/// innerhalb der Gruppe (mit Umlauf); ein Mausklick wählt und setzt den Fokus. Erstes Bedienelement
/// eines frisch geöffneten Dialogs: das gewählte Feld bekommt den Startfokus. Liefert die neue Wahl.
pub fn radio_group(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, rects: &[Rect], labels: &[&str], sel: usize, enabled: bool) -> Option<usize> {
    let gid = Id::new(id);
    let n = rects.len().min(labels.len());
    if n == 0 {
        return None;
    }
    let sel = sel.min(n - 1);
    let theme = Theme::current(ui.ctx());
    let c = theme.colors.clone();
    let ids: Vec<Id> = (0..n).map(|k| gid.with(k)).collect();
    let mut out = None;
    let mut focus_to = None;
    for k in 0..n {
        let r = rects[k];
        // nur das gewählte Feld ist fokussierbar (ein Tab-Stopp für die ganze Gruppe)
        let sense = if !enabled { Sense::hover() } else if k == sel { Sense::click() } else { Sense::CLICK };
        let resp = ui.interact(r, ids[k], sense);
        resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::RadioButton, enabled, k == sel, labels[k]));
        let center = pos2(r.left() + 6.5, r.center().y);
        ui.painter().circle_filled(center, 6.0, if enabled { c.field_bg } else { c.frame_bg });
        ui.painter().circle_stroke(center, 6.0, Stroke::new(1.0, if enabled && resp.hovered() { c.focus } else { c.field_border }));
        if k == sel {
            ui.painter().circle_filled(center, 3.0, if enabled { c.field_text } else { c.text_disabled });
        }
        let tr = Rect::from_min_max(pos2(r.left() + 17.0, r.top()), pos2(r.right().max(r.left() + 17.0), r.bottom()));
        if !labels[k].is_empty() {
            crate::inspector::fields::label(ui, tr, labels[k], enabled);
        }
        if k == sel && enabled {
            if ui.memory(|m| m.has_focus(ids[k])) {
                // Fokusrahmen um die Beschriftung (Windows: gepunktet)
                let tw = text_w(ui.ctx(), labels[k]).min(tr.width());
                let fr = Rect::from_min_size(pos2(tr.left() - 2.0, r.center().y - 8.0), vec2(tw + 4.0, 16.0));
                ui.painter().rect_stroke(fr, 0.0, Stroke::new(1.0, c.focus), StrokeKind::Inside);
                // Pfeiltasten bleiben in der Gruppe (sonst sucht egui das nächste Feld in der Richtung)
                ui.memory_mut(|m| m.set_focus_lock_filter(ids[k], egui::EventFilter { horizontal_arrows: true, vertical_arrows: true, tab: false, escape: false }));
                let step = ui.input_mut(|i| {
                    let mut s = 0i32;
                    for (key, d) in [(egui::Key::ArrowDown, 1), (egui::Key::ArrowRight, 1), (egui::Key::ArrowUp, -1), (egui::Key::ArrowLeft, -1)] {
                        if i.consume_key(egui::Modifiers::NONE, key) {
                            s += d;
                        }
                    }
                    s
                });
                if step != 0 {
                    let to = (sel as i32 + step).rem_euclid(n as i32) as usize;
                    out = Some(to);
                    focus_to = Some(to);
                }
            }
            // Startfokus, wenn die Gruppe das erste Bedienelement des Dialogs ist (nicht im
            // Größenbestimmungs-Bild, dort sind die Widgets gesperrt und die Felder davor kamen
            // noch nicht zum Zug, vgl. [`autofocus_field`])
            let key = autofocus_id(ui.layer_id());
            if resp.enabled() && ui.ctx().data(|d| d.get_temp::<u64>(key)).is_some() {
                ui.ctx().data_mut(|d| d.remove::<u64>(key));
                resp.request_focus();
            }
        }
        if enabled && resp.clicked() {
            if k != sel {
                out = Some(k);
            }
            focus_to = Some(k);
        }
    }
    if let Some(k) = focus_to {
        ui.memory_mut(|m| m.request_focus(ids[k]));
    }
    out
}

/// Bedienelement, das wie im Original kein Tab-Stopp ist, aber nur als fokussierbares egui-Widget
/// zu haben ist (z. B. das Farbfeld in „Neues Dokument“): erreicht Tab es, geht der Fokus im
/// nächsten Bild in derselben Richtung weiter. `key` kennzeichnet das Element; vor dem Zeichnen
/// [`skip_tab_stop_before`], danach [`skip_tab_stop_after`] mit der Antwort aufrufen.
pub fn skip_tab_stop_before(ui: &Ui, key: Id) {
    if let Some(back) = ui.ctx().data_mut(|d| d.remove_temp::<bool>(key.with("fp-skip-tab"))) {
        ui.memory_mut(|m| m.move_focus(if back { egui::FocusDirection::Previous } else { egui::FocusDirection::Next }));
    }
}

/// Siehe [`skip_tab_stop_before`].
pub fn skip_tab_stop_after(ui: &Ui, key: Id, resp: &egui::Response) {
    if resp.gained_focus() && !ui.input(|i| i.pointer.any_pressed() || i.pointer.any_down()) {
        let back = ui.input(|i| i.modifiers.shift);
        ui.ctx().data_mut(|d| d.insert_temp(key.with("fp-skip-tab"), back));
        ui.ctx().request_repaint();
    }
}

const LAST_DIALOG: &str = "fp-last-dialog";

/// Kennung des zuletzt gezeichneten Dialograhmens (für Tests und Diagnose).
pub fn last_dialog_id(ctx: &egui::Context) -> Option<String> {
    ctx.data(|d| d.get_temp(Id::new(LAST_DIALOG)))
}

/// Deckkraft der schwarzen Abdunklung hinter einem Dialog (die Pipette rechnet sie heraus, N27).
pub fn backdrop_alpha(theme: &Theme) -> u8 {
    if theme.glow { 150 } else { 40 }
}

/// Bildschirmrechteck eines Dialograhmens im letzten Bild.
pub fn dialog_rect(ctx: &egui::Context, id: &str) -> Option<Rect> {
    ctx.memory(|m| m.area_rect(Id::new(id)))
}

fn geom_id(id: &str) -> Id {
    Id::new((id, "fp-dialog-geom"))
}

/// Gemerkte Lage eines Dialogs (`None` = noch nie geöffnet → zentriert).
pub fn dialog_geom(ctx: &egui::Context, id: &str) -> Option<DialogGeom> {
    ctx.data(|d| d.get_temp(geom_id(id)))
}

/// Hält die Titelleiste im Hauptfenster: oben nie darüber hinaus, unten bleibt die Leiste ganz,
/// waagerecht bleiben mindestens [`DIALOG_KEEP_VISIBLE`] px sichtbar.
pub fn clamp_dialog_pos(pos: egui::Pos2, outer: egui::Vec2, screen: Rect) -> egui::Pos2 {
    let keep = DIALOG_KEEP_VISIBLE.min(outer.x);
    let x = pos.x.clamp(screen.left() - outer.x + keep, (screen.right() - keep).max(screen.left() - outer.x + keep));
    let y = pos.y.clamp(screen.top(), (screen.bottom() - DIALOG_TITLE_H).max(screen.top()));
    pos2(x, y)
}

fn dialog_frame_ex(ctx: &egui::Context, id: &str, title: &str, size: egui::Vec2, initial: Option<egui::Vec2>, resizable: bool, content: impl FnOnce(&mut Ui, Rect)) -> DialogClose {
    let theme = Theme::current(ctx);
    let c = theme.colors.clone();
    let mut close = DialogClose::Open;
    let title_h = DIALOG_TITLE_H;
    let screen = ctx.content_rect();
    let pass = ctx.cumulative_pass_nr();
    // längere Titel (andere Sprachen) verbreitern den Dialog, statt unter das Kreuz zu laufen
    let title_w = ctx.fonts_mut(|f| f.layout_no_wrap(title.to_string(), theme.menu_font(), egui::Color32::WHITE).size().x);
    let size = vec2(size.x.max((title_w + 10.0 + 44.0).ceil()), size.y);
    // neu geöffnet (im letzten Bild nicht gezeichnet): wie im Original zentriert, Ausgangsgröße
    let stored = dialog_geom(ctx, id).filter(|g| g.pass + 1 >= pass);
    let opened = stored.is_none();
    let client = if resizable { stored.map(|g| g.client).unwrap_or(initial.unwrap_or(size)).max(size) } else { size };
    // RAND-2b (R2-3): höher/breiter als das Hauptfenster → sichtbare Größe auf den Bildschirm
    // begrenzen, Inhalt mit Bildlauf, Knopfzeile unten fest (siehe [`push_button`])
    let max_client = vec2((screen.width() - 2.0).max(120.0), (screen.height() - title_h - 2.0).max(80.0));
    let scroll = client.x > max_client.x + 0.5 || client.y > max_client.y + 0.5;
    let bar = if scroll { ctx.global_style().spacing.scroll.allocated_width() } else { 0.0 };
    let shown = if scroll { vec2((client.x + bar).min(max_client.x), client.y.min(max_client.y)) } else { client };
    let outer = vec2(shown.x, shown.y + title_h) + vec2(2.0, 2.0);
    let mut pos = clamp_dialog_pos(stored.map(|g| g.pos).unwrap_or_else(|| screen.center() - outer / 2.0), outer, screen);
    // Titelleiste ziehen wie bei einem Windows-Dialog: der gegriffene Punkt bleibt unter dem
    // Zeiger. Versatz beim Drücken merken und die Lage schon vor dem Zeichnen aus der Zeigerlage
    // rechnen (kein Nachlaufen um ein Bild). Summieren der Bild-Deltas verlor die Bewegung bis zum
    // Erkennen des Ziehens, wenn Drücken und erste Bewegungen gebündelt ankommen (N02: ~80 %).
    let drag_id = Id::new((id, "title-drag"));
    let grab_id = Id::new((id, "title-grab"));
    if ctx.is_being_dragged(drag_id) {
        let grab = ctx.data(|d| d.get_temp::<egui::Vec2>(grab_id)).or_else(|| {
            let g = ctx.input(|i| i.pointer.press_origin()).map(|o| o - pos);
            if let Some(g) = g {
                ctx.data_mut(|d| d.insert_temp(grab_id, g));
            }
            g
        });
        if let (Some(g), Some(p)) = (grab, ctx.input(|i| i.pointer.latest_pos())) {
            pos = clamp_dialog_pos(p - g, outer, screen);
        }
    } else {
        ctx.data_mut(|d| d.remove::<egui::Vec2>(grab_id));
    }
    let mut geom = DialogGeom { pos, client, pass };
    let af = autofocus_id(egui::LayerId::new(egui::Order::Foreground, Id::new(id)));
    if opened {
        ctx.data_mut(|d| d.insert_temp(af, pass));
    }
    let area = egui::Area::new(Id::new(id))
        .kind(egui::UiKind::Modal)
        .sense(Sense::hover())
        .order(egui::Order::Foreground)
        .interactable(true)
        .movable(false)
        .constrain(false)
        .current_pos(pos);
    // DES-1: Schatten (OLED: Akzentschein) um den Rahmen; Lage und Größe bleiben gleich
    let frame = egui::Frame::new().fill(c.frame_bg).stroke(Stroke::new(1.0, if theme.glow { c.focus } else { c.line_dark })).inner_margin(egui::Margin::ZERO).shadow(theme.dialog_shadow());
    let resp = egui::Modal::new(Id::new(id)).area(area).frame(frame).backdrop_color(egui::Color32::from_black_alpha(backdrop_alpha(&theme))).show(ctx, |ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(shown.x, shown.y + title_h), Sense::hover());
        let tr = Rect::from_min_size(rect.min, vec2(rect.width(), title_h));
        // DES-1: Titelleiste als ruhiger Verlauf, im Dunkel-Design deutlich von der Fläche abgesetzt
        let (tt, tb) = theme.dialog_title_gradient();
        crate::widgets::vgradient(ui.painter(), tr, tt, tb);
        ui.painter().hline(tr.x_range(), tr.bottom() - 0.5, Stroke::new(1.0, theme.dialog_title_line()));
        ui.painter().text(pos2(tr.left() + 10.0, tr.center().y), Align2::LEFT_CENTER, title, theme.menu_font(), c.app_bar_text);
        let xr = Rect::from_center_size(pos2(tr.right() - 18.0, tr.center().y), vec2(30.0, 22.0));
        // Titelleiste ziehen verschiebt den Dialog (ohne das Schließen-Kreuz)
        let drag_r = Rect::from_min_max(tr.min, pos2(xr.left() - 2.0, tr.bottom()));
        // wie unter Windows kein Tab-Stopp (nur ziehen, nicht fokussierbar; PM-5 Tab-Folge)
        let dresp = ui.interact(drag_r, drag_id, Sense::DRAG);
        dresp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, fp_i18n::tr!("dlg-titlebar", title = title)));
        if dresp.dragged() {
            ui.ctx().request_repaint();
        }
        let xresp = ui.interact(xr, Id::new((id, "x")), Sense::CLICK);
        if xresp.hovered() {
            ui.painter().rect_filled(xr, 0.0, c.window_close_hover);
        }
        let st = crate::icons::IconStyle::mono(if xresp.hovered() { egui::Color32::WHITE } else { c.app_bar_text }, c.frame_bg);
        crate::icons::paint(ui.painter(), Rect::from_center_size(xr.center(), vec2(12.0, 12.0)), crate::icons::Icon::WindowClose, &st);
        if xresp.clicked() {
            close = DialogClose::Cancelled;
        }
        let body = Rect::from_min_max(pos2(rect.left(), tr.bottom()), rect.max);
        // DES-1: Platz für die Fußleiste vor dem Inhalt reservieren (liegt so unter allem
        // Gezeichneten); gefüllt wird sie nach dem Inhalt, wenn eine untere Knopfzeile da ist
        let layer = ui.layer_id();
        ui.ctx().data_mut(|d| {
            d.remove::<Vec<Rect>>(footer_buttons_id(layer));
            d.remove::<Rect>(footer_explicit_id(layer));
        });
        let slot = ui.painter().add(egui::Shape::Noop);
        let first_shape = ui.ctx().graphics(|g| g.get(layer).map(|p| p.next_idx().0).unwrap_or(0));
        let band = if scroll {
            // RAND-2b: feste Knopfzeile unten = Fußleiste
            Some(scrolled_body(ui, id, body, client, bar, content))
        } else {
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(body).layout(egui::Layout::top_down(egui::Align::Min)));
            content(&mut child, body);
            let explicit = ui.ctx().data(|d| d.get_temp::<Rect>(footer_explicit_id(layer)));
            explicit.or_else(|| {
                let buttons = ui.ctx().data(|d| d.get_temp::<Vec<Rect>>(footer_buttons_id(layer))).unwrap_or_default();
                footer_top(body, &buttons, &painted_rects(ui.ctx(), layer, first_shape)).map(|top| Rect::from_min_max(pos2(body.left(), top), body.max))
            })
        };
        if let Some(b) = band {
            ui.painter().set(slot, footer_shape(b, &theme));
        }
        ui.ctx().data_mut(|d| d.insert_temp(Id::new((id, "fp-dialog-footer")), band.map(|b| b.top())));
        // Größe an Rändern und Ecken ändern (nach dem Inhalt registriert, damit die Griffe oben liegen)
        if resizable {
            const G: f32 = 5.0;
            let zones: [(Rect, egui::CursorIcon, [f32; 4]); 8] = [
                (Rect::from_min_max(pos2(rect.left(), rect.top() + G), pos2(rect.left() + G, rect.bottom() - G)), egui::CursorIcon::ResizeHorizontal, [1.0, 0.0, 0.0, 0.0]),
                (Rect::from_min_max(pos2(rect.right() - G, rect.top() + G), pos2(rect.right(), rect.bottom() - G)), egui::CursorIcon::ResizeHorizontal, [0.0, 0.0, 1.0, 0.0]),
                (Rect::from_min_max(pos2(rect.left() + G, rect.top()), pos2(rect.right() - G, rect.top() + 3.0)), egui::CursorIcon::ResizeVertical, [0.0, 1.0, 0.0, 0.0]),
                (Rect::from_min_max(pos2(rect.left() + G, rect.bottom() - G), pos2(rect.right() - G, rect.bottom())), egui::CursorIcon::ResizeVertical, [0.0, 0.0, 0.0, 1.0]),
                (Rect::from_min_size(rect.min, vec2(G, G)), egui::CursorIcon::ResizeNwSe, [1.0, 1.0, 0.0, 0.0]),
                (Rect::from_min_size(pos2(rect.right() - G, rect.top()), vec2(G, G)), egui::CursorIcon::ResizeNeSw, [0.0, 1.0, 1.0, 0.0]),
                (Rect::from_min_size(pos2(rect.left(), rect.bottom() - G), vec2(G, G)), egui::CursorIcon::ResizeNeSw, [1.0, 0.0, 0.0, 1.0]),
                (Rect::from_min_size(rect.max - vec2(G + 3.0, G + 3.0), vec2(G + 3.0, G + 3.0)), egui::CursorIcon::ResizeNwSe, [0.0, 0.0, 1.0, 1.0]),
            ];
            for (k, (zr, cursor, [l, t, r, b])) in zones.into_iter().enumerate() {
                let zresp = ui.interact(zr, Id::new((id, "resize", k)), Sense::drag());
                if zresp.hovered() || zresp.dragged() {
                    ui.ctx().set_cursor_icon(cursor);
                }
                if zresp.dragged() {
                    let dlt = zresp.drag_delta();
                    let mut min = geom.pos;
                    let mut max = geom.pos + geom.client;
                    min.x += dlt.x * l;
                    min.y += dlt.y * t;
                    max.x += dlt.x * r;
                    max.y += dlt.y * b;
                    let mut sz = max - min;
                    // Mindestgröße: die feste Kante bleibt stehen
                    if sz.x < size.x {
                        if l > 0.0 {
                            min.x = max.x - size.x;
                        }
                        sz.x = size.x;
                    }
                    if sz.y < size.y {
                        if t > 0.0 {
                            min.y = max.y - size.y;
                        }
                        sz.y = size.y;
                    }
                    geom = DialogGeom { pos: min, client: sz, pass };
                    ui.ctx().request_repaint();
                }
            }
        }
    });
    // hat nach drei Bildern kein Feld und keine Optionsgruppe den Startfokus genommen, geht er an
    // den Standardknopf (Dialoge ohne Eingabefeld, wie unter Windows)
    if ctx.data(|d| d.get_temp::<u64>(af)).is_some_and(|p| p + 3 < pass) {
        let layer = egui::LayerId::new(egui::Order::Foreground, Id::new(id));
        // ein Feld, das sich den Fokus selbst geholt hat (eigene TextEdits), behält ihn
        let inside = ctx.memory(|m| m.focused()).and_then(|f| ctx.read_response(f)).is_some_and(|r| r.layer_id == layer);
        ctx.data_mut(|d| {
            d.remove::<u64>(af);
            if !inside {
                d.insert_temp(focus_ok_id(layer), pass);
            }
        });
        if !inside {
            ctx.request_repaint();
        }
    }
    geom.pos = clamp_dialog_pos(geom.pos, vec2(geom.client.x, geom.client.y + title_h) + vec2(2.0, 2.0), screen);
    ctx.data_mut(|d| {
        d.insert_temp(geom_id(id), geom);
        d.insert_temp(Id::new(LAST_DIALOG), id.to_string());
    });
    // Schließen nur mit Esc (oberster Dialog, kein Popup offen) bzw. Kreuz/Abbrechen. Ein Klick auf
    // den abgedunkelten Hintergrund schließt einen modalen Windows-Dialog nicht (Signalton, Befund
    // NZ-2); `Modal::should_close` zählte ihn als Abbrechen.
    let esc = resp.is_top_modal && !resp.any_popup_open && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
    if esc || resp.response.should_close() {
        close = DialogClose::Cancelled;
    }
    close
}

/// Höhe der festen Knopfzeile unten, wenn ein Dialog mit Bildlauf gezeigt wird (RAND-2b).
pub const DIALOG_PIN_STRIP: f32 = 48.0;

/// Feste Knopfzeile eines Dialogs mit Bildlauf: [`push_button`] verschiebt Knöpfe, deren Oberkante
/// im Inhalt unterhalb von `from_y` liegt, in den Streifen `clip` am unteren Rand.
#[derive(Clone, Copy, Debug)]
struct PinStrip {
    from_y: f32,
    mid_x: f32,
    dy: f32,
    dx_left: f32,
    dx_right: f32,
    clip: Rect,
}

fn pin_id() -> Id {
    Id::new("fp-dialog-pin-strip")
}

/// Inhalt eines Dialogs, der größer als das Hauptfenster ist: der Inhalt behält seine volle Größe
/// (`client`) und läuft in einem Bildlaufbereich; die Knopfzeile bleibt unten sichtbar. Liefert
/// den Streifen der Knopfzeile (Fläche und Linie zeichnet der Rahmen als Fußleiste, DES-1).
fn scrolled_body(ui: &mut Ui, id: &str, body: Rect, client: egui::Vec2, bar: f32, content: impl FnOnce(&mut Ui, Rect)) -> Rect {
    let strip_h = DIALOG_PIN_STRIP.min(body.height() / 2.0);
    let view = Rect::from_min_max(body.min, pos2(body.right(), body.bottom() - strip_h));
    let strip = Rect::from_min_max(pos2(body.left(), view.bottom()), body.max);
    let ctx = ui.ctx().clone();
    let mut sui = ui.new_child(egui::UiBuilder::new().max_rect(view).layout(egui::Layout::top_down(egui::Align::Min)));
    egui::ScrollArea::both().id_salt((id, "fp-dialog-scroll")).auto_shrink([false, false]).max_width(view.width()).max_height(view.height()).show(&mut sui, |ui| {
        let (vbody, _) = ui.allocate_exact_size(client, Sense::hover());
        let pin = PinStrip {
            from_y: vbody.bottom() - strip_h,
            mid_x: vbody.center().x,
            dy: body.bottom() - vbody.bottom(),
            dx_left: body.left() - vbody.left(),
            dx_right: (body.right() - bar) - vbody.right(),
            clip: strip,
        };
        ctx.data_mut(|d| d.insert_temp(pin_id(), pin));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(vbody).layout(egui::Layout::top_down(egui::Align::Min)));
        content(&mut child, vbody);
        ctx.data_mut(|d| d.remove::<PinStrip>(pin_id()));
    });
    strip
}

// ---------------------------------------------------------------------- Fußleiste (DES-1)

/// Knöpfe eines Dialogs im laufenden Bild (für die Fußleiste, je Ebene).
fn footer_buttons_id(layer: egui::LayerId) -> Id {
    Id::new(("fp-dialog-footer-buttons", layer.id))
}

/// Ausdrücklich gesetzte Fußleiste ([`set_footer`], z. B. aus `ui_kit::button_bar`).
fn footer_explicit_id(layer: egui::LayerId) -> Id {
    Id::new(("fp-dialog-footer-explicit", layer.id))
}

/// Oberkante der zuletzt gezeichneten Fußleiste eines Dialogs (für Tests; `None` = keine).
pub fn dialog_footer_top(ctx: &egui::Context, id: &str) -> Option<f32> {
    ctx.data(|d| d.get_temp::<Option<f32>>(Id::new((id, "fp-dialog-footer")))).flatten()
}

/// Legt die Fußleiste des Dialogs ausdrücklich fest (Knopfleisten mit reserviertem Platz).
pub fn set_footer(ui: &Ui, band: Rect) {
    ui.ctx().data_mut(|d| d.insert_temp(footer_explicit_id(ui.layer_id()), band));
}

/// Merkt einen Knopf für die Fußleiste (aus [`push_button`]).
fn note_footer_button(ui: &Ui, rect: Rect) {
    ui.ctx().data_mut(|d| d.get_temp_mut_or_default::<Vec<Rect>>(footer_buttons_id(ui.layer_id())).push(rect));
}

/// Oberkante der Fußleiste aus den Knöpfen und dem Gezeichneten: Knöpfe, die bis auf 18 px an
/// die Unterkante reichen, bilden die untere Knopfzeile; die Leiste beginnt 12 px darüber, aber
/// nie über Gezeichnetem (Felder, Beschriftungen, Listen). Ragt etwas Gezeichnetes oder ein
/// anderer Knopf in die Knopfzeile hinein (Knöpfe rechts untereinander, Liste bis unten), gibt
/// es keine Leiste – die Anordnung des Originals bleibt unberührt.
pub fn footer_top(body: Rect, buttons: &[Rect], shapes: &[Rect]) -> Option<f32> {
    let row: Vec<Rect> = buttons.iter().copied().filter(|r| r.bottom() >= body.bottom() - 18.0 && r.bottom() <= body.bottom() + 0.5 && r.top() > body.center().y).collect();
    let row_top = row.iter().map(|r| r.top()).fold(f32::INFINITY, f32::min);
    if !row_top.is_finite() {
        return None;
    }
    let mut lim = body.top();
    // andere Knöpfe darüber zählen wie Gezeichnetes (Spalte rechts untereinander → keine Leiste)
    for b in buttons.iter().filter(|b| !row.contains(b) && b.intersects(body)) {
        if b.bottom() > row_top {
            return None;
        }
        lim = lim.max(b.bottom());
    }
    for s in shapes {
        if s.top() >= row_top - 4.0 || !s.intersects(body) {
            continue; // liegt in der Knopfzeile (Kästchen „Vorschau“, Knöpfe) bzw. außerhalb
        }
        if s.bottom() > row_top {
            return None; // reicht von oben bis in die Knopfzeile
        }
        lim = lim.max(s.bottom());
    }
    let top = (row_top - 12.0).max((lim + 3.0).ceil()).round();
    (top <= row_top - 5.0).then_some(top)
}

/// Sichtbare Rechtecke aller ab `from` gezeichneten Formen der Ebene.
fn painted_rects(ctx: &egui::Context, layer: egui::LayerId, from: usize) -> Vec<Rect> {
    fn walk(s: &egui::Shape, clip: Rect, out: &mut Vec<Rect>) {
        match s {
            egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, clip, out)),
            egui::Shape::Noop => {}
            s => {
                let r = s.visual_bounding_rect().intersect(clip);
                if r.is_positive() {
                    out.push(r);
                }
            }
        }
    }
    let mut out = Vec::new();
    ctx.graphics(|g| {
        if let Some(pl) = g.get(layer) {
            for cs in pl.all_entries().skip(from) {
                walk(&cs.shape, cs.clip_rect, &mut out);
            }
        }
    });
    out
}

/// Fläche und geätzte Linie der Fußleiste.
fn footer_shape(band: Rect, theme: &Theme) -> egui::Shape {
    let (dark, light) = theme.dialog_footer_lines();
    let top = band.top().round();
    let mut v = vec![egui::Shape::rect_filled(band, 0.0, theme.dialog_footer_bg()), egui::Shape::hline(band.x_range(), top + 0.5, Stroke::new(1.0, dark))];
    if let Some(l) = light {
        v.push(egui::Shape::hline(band.x_range(), top + 1.5, Stroke::new(1.0, l)));
    }
    egui::Shape::Vec(v)
}

/// Setzt ein Widget linksbündig oben in `rect`. Anders als `ui.put` (zentriertes,
/// ausgerichtetes Layout) streckt das umbrechende Beschriftungen nicht auf Blocksatz.
pub fn put_left(ui: &mut Ui, rect: Rect, w: impl egui::Widget) -> egui::Response {
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(egui::Layout::top_down(egui::Align::Min)));
    child.add(w)
}

/// Mehrzeiliger Meldungstext linksbündig in `rect` (Umbruch an der Rechteckbreite, kein Blocksatz).
pub fn text_block(ui: &mut Ui, rect: Rect, text: impl Into<egui::WidgetText>) -> egui::Response {
    put_left(ui, rect, egui::Label::new(text).wrap())
}

/// Gruppenrahmen mit Beschriftung (GroupBox).
pub fn group_box(ui: &Ui, rect: Rect, label: &str) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let r = rect.shrink2(vec2(0.0, 0.0));
    let top = r.top() + 6.0;
    let font = theme.ui_font();
    let tw = crate::widgets::text_width(ui, label, &font);
    let p = ui.painter();
    // DES-1: geätzter Rahmen wie ein Windows-Gruppenfeld (dunkle Linie + Lichtkante), OLED fein
    let (dark, light) = theme.group_box_lines();
    let edges = |o: f32, col: egui::Color32| {
        let s = Stroke::new(1.0, col);
        let (l, t, rr, b) = (r.left() + o, top + o, r.right() + o, r.bottom() + o);
        p.line_segment([pos2(l, t), pos2(r.left() + 6.0, t)], s);
        p.line_segment([pos2(r.left() + 10.0 + tw + 4.0, t), pos2(rr, t)], s);
        p.line_segment([pos2(rr, t), pos2(rr, b)], s);
        p.line_segment([pos2(rr, b), pos2(l, b)], s);
        p.line_segment([pos2(l, b), pos2(l, t)], s);
    };
    if let Some(l) = light {
        edges(1.0, l);
    }
    edges(0.0, dark);
    p.text(pos2(r.left() + 10.0, top), Align2::LEFT_CENTER, label, font, c.text);
}

/// Standard-Knopf mit fester Größe; `default` zeichnet den Standardrahmen (OK).
pub fn push_button(ui: &mut Ui, rect: Rect, label: &str, default: bool, enabled: bool) -> bool {
    // Dialog mit Bildlauf: Knöpfe der unteren Zeile bleiben fest im sichtbaren Streifen
    if let Some(pin) = ui.ctx().data(|d| d.get_temp::<PinStrip>(pin_id()))
        && rect.top() >= pin.from_y - 0.5
    {
        let dx = if rect.center().x > pin.mid_x { pin.dx_right } else { pin.dx_left };
        let r = rect.translate(vec2(dx, pin.dy));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r.union(pin.clip)));
        child.set_clip_rect(pin.clip);
        return push_button_at(&mut child, r, label, default, enabled);
    }
    push_button_at(ui, rect, label, default, enabled)
}

fn push_button_at(ui: &mut Ui, rect: Rect, label: &str, default: bool, enabled: bool) -> bool {
    let theme = Theme::current(ui.ctx());
    if button_text_w(ui.ctx(), label) + BUTTON_PAD > rect.width() + 0.5 {
        report_overflow(ui.ctx(), format!("Knopf „{label}“ breiter als {:.0} px", rect.width()));
    }
    // gesperrt: grau wie im Original
    let text = access_key_text(ui, label, enabled);
    note_footer_button(ui, rect);
    // DES-1: Standardknopf leicht in der Fokusfarbe getönt (wie der Standardknopf unter Windows)
    let mut button = egui::Button::new(text).min_size(rect.size());
    if default && enabled {
        let hovered = ui.rect_contains_pointer(rect) && !ui.input(|i| i.pointer.any_down());
        button = button.fill(theme.default_button_fill(hovered));
    }
    let resp = ui.add_enabled_ui(enabled, |ui| ui.put(rect, button)).inner;
    // Knöpfe merken: hat einer davon (per Tab) den Fokus, löst Eingabe ihn aus statt des Standardknopfs
    let key = Id::new("fp-dialog-buttons");
    if !ui.ctx().data(|d| d.get_temp::<std::collections::HashSet<Id>>(key)).is_some_and(|s| s.contains(&resp.id)) {
        ui.ctx().data_mut(|d| d.get_temp_mut_or_default::<std::collections::HashSet<Id>>(key).insert(resp.id));
    }
    if default && enabled {
        ui.painter().rect_stroke(rect, 2.0, Stroke::new(1.0, theme.colors.focus), StrokeKind::Inside);
        default_button_autofocus(ui, &resp);
    }
    let clicked = enabled && resp.clicked();
    clicked || (default && enabled && enter_confirms(ui, resp.id))
}

/// Beschriftung mit Zugriffstaste: „&Ja“ unterstreicht das J (wie unter Windows).
fn access_key_text(ui: &Ui, label: &str, enabled: bool) -> egui::WidgetText {
    let Some(i) = label.find('&') else { return label.into() };
    let (a, rest) = label.split_at(i);
    let rest = &rest[1..];
    let Some(k) = rest.chars().next() else { return label.replace('&', "").into() };
    let font = egui::TextStyle::Button.resolve(ui.style());
    let color = if enabled { ui.visuals().text_color() } else { ui.visuals().weak_text_color() };
    let fmt = |underline: bool| egui::TextFormat { font_id: font.clone(), color, underline: if underline { Stroke::new(1.0, color) } else { Stroke::NONE }, ..Default::default() };
    let mut job = egui::text::LayoutJob::default();
    job.append(a, 0.0, fmt(false));
    job.append(&k.to_string(), 0.0, fmt(true));
    job.append(&rest[k.len_utf8()..], 0.0, fmt(false));
    job.into()
}

/// Zugriffstaste einer Beschriftung („&Ja“ → J, „&Yes“ → Y); `None` ohne `&`.
pub fn access_key(label: &str) -> Option<egui::Key> {
    let i = label.find('&')?;
    let c = label[i + 1..].chars().next()?.to_uppercase().next()?;
    egui::Key::from_name(&c.to_string())
}

// ---------------------------------------------------------------------- Textbreiten (Mehrsprachigkeit)

/// Innenabstand eines Knopfs links + rechts (`button_padding` 6 px der Designs).
pub const BUTTON_PAD: f32 = 12.0;

/// Breite eines Texts in der Dialogschrift (`&` der Zugriffstaste zählt nicht), auch vor dem Zeichnen.
pub fn text_w(ctx: &egui::Context, text: &str) -> f32 {
    let font = Theme::current(ctx).ui_font();
    ctx.fonts_mut(|f| f.layout_no_wrap(text.replace('&', ""), font, egui::Color32::WHITE).size().x)
}

/// Höhe eines an `width` umbrochenen Texts in der Dialogschrift.
pub fn wrapped_h(ctx: &egui::Context, text: &str, width: f32) -> f32 {
    let font = Theme::current(ctx).ui_font();
    ctx.fonts_mut(|f| f.layout(text.to_string(), font, egui::Color32::WHITE, width).size().y)
}

/// Breite einer Knopfbeschriftung (Knopfschrift).
pub fn button_text_w(ctx: &egui::Context, text: &str) -> f32 {
    let font = ctx.global_style().text_styles.get(&egui::TextStyle::Button).cloned().unwrap_or_else(|| Theme::current(ctx).ui_font());
    ctx.fonts_mut(|f| f.layout_no_wrap(text.replace('&', ""), font, egui::Color32::WHITE).size().x)
}

/// Breitester von mehreren Texten.
pub fn max_text_w(ctx: &egui::Context, texts: &[&str]) -> f32 {
    texts.iter().map(|s| text_w(ctx, s)).fold(0.0, f32::max)
}

/// Platz, den die breiteste Beschriftung über `avail` hinaus braucht (0 = passt wie bisher).
pub fn extra_w(ctx: &egui::Context, texts: &[&str], avail: f32) -> f32 {
    (max_text_w(ctx, texts) - avail).max(0.0).ceil()
}

/// Breite eines Kombifelds (Feld + Pfeil 22 px, wie `fields::combo` kürzt), damit der längste
/// Eintrag ganz passt; passt er schon, bleibt die Mindestbreite (Deutsch: wie bisher).
pub fn combo_fit_w(ctx: &egui::Context, items: &[&str], min: f32) -> f32 {
    min.max((max_text_w(ctx, items) + 22.0).ceil())
}

/// Texte zu Schlüsseln.
pub fn tl(keys: &[&str]) -> Vec<String> {
    keys.iter().map(|k| t(k)).collect()
}

/// `&[String]` → `Vec<&str>`.
pub fn refs(v: &[String]) -> Vec<&str> {
    v.iter().map(|s| s.as_str()).collect()
}

/// Knopfbreite: Mindestbreite wie bisher, längere Beschriftungen bekommen Platz.
pub fn button_w(ctx: &egui::Context, label: &str, min: f32) -> f32 {
    min.max((button_text_w(ctx, label) + BUTTON_PAD).ceil())
}

/// Gemeinsame Breite einer Knopfgruppe (alle gleich breit wie unter Windows).
pub fn buttons_w(ctx: &egui::Context, labels: &[&str], min: f32) -> f32 {
    labels.iter().map(|l| button_w(ctx, l, min)).fold(min, f32::max)
}

/// Knopfzeile rechtsbündig: `labels` von links nach rechts, rechter Rand `right`, Abstand `gap`;
/// liefert die Rechtecke (gleich breit, Mindestbreite `min`).
pub fn button_row(ctx: &egui::Context, right: f32, top: f32, labels: &[&str], min: f32, h: f32, gap: f32) -> Vec<Rect> {
    let w = buttons_w(ctx, labels, min);
    let n = labels.len() as f32;
    let x0 = right - n * w - (n - 1.0).max(0.0) * gap;
    (0..labels.len()).map(|i| Rect::from_min_size(pos2(x0 + i as f32 * (w + gap), top), vec2(w, h))).collect()
}

/// Gesamtbreite einer Knopfzeile (für die Mindestbreite eines Dialogs).
pub fn button_row_w(ctx: &egui::Context, labels: &[&str], min: f32, gap: f32) -> f32 {
    let n = labels.len() as f32;
    n * buttons_w(ctx, labels, min) + (n - 1.0).max(0.0) * gap
}

const OVERFLOW: &str = "fp-dialog-overflow";

/// Meldet einen Text, der nicht in seinen Platz passt (für die Layout-Tests aller Sprachen).
pub fn report_overflow(ctx: &egui::Context, what: String) {
    log::debug!("Dialog-Layout: {what}");
    ctx.data_mut(|d| {
        let v = d.get_temp_mut_or_default::<Vec<String>>(Id::new(OVERFLOW));
        if !v.contains(&what) {
            v.push(what);
        }
    });
}

/// Gemeldete Überläufe abholen (und leeren).
pub fn take_overflows(ctx: &egui::Context) -> Vec<String> {
    ctx.data_mut(|d| d.remove_temp::<Vec<String>>(Id::new(OVERFLOW))).unwrap_or_default()
}

/// Beschriftung, die in `r` passen muss (sonst Überlauf melden); links, senkrecht mittig.
pub fn fit_label(ui: &Ui, r: Rect, text: &str, enabled: bool) {
    if text_w(ui.ctx(), text) > r.width() + 0.5 {
        report_overflow(ui.ctx(), format!("Beschriftung „{text}“ breiter als {:.0} px", r.width()));
    }
    crate::inspector::fields::label(ui, r, text, enabled);
}

/// Eingabe löst wie im Original immer den Standardknopf des obersten Dialogs aus, auch wenn ein
/// Zahlen- oder Textfeld den Fokus hat. Damit der getippte Wert vorher übernommen ist (Felder
/// übernehmen beim Fokusverlust, der erst im Bild nach der Eingabe gemeldet wird), wirkt die Taste
/// ein Bild später. Ein mehrzeiliges Textfeld behält die Eingabe (Zeilenumbruch).
fn enter_confirms(ui: &Ui, own: Id) -> bool {
    let ctx = ui.ctx();
    let layer = ui.layer_id();
    if ctx.memory(|m| m.top_modal_layer()).is_some_and(|t| t != layer) {
        return false;
    }
    let key = Id::new(("fp-enter-pending", layer.id));
    let nr = ctx.cumulative_pass_nr();
    if let Some(p) = ctx.data(|d| d.get_temp::<u64>(key)) {
        if p + 1 == nr {
            ctx.data_mut(|d| d.remove::<u64>(key));
            // ein einzeiliges Feld gibt den Fokus mit Eingabe ab; ein mehrzeiliges behält ihn
            // (Zeilenumbruch) und bestätigt nicht
            return !ctx.text_edit_focused();
        }
        if p != nr {
            ctx.data_mut(|d| d.remove::<u64>(key));
        }
        return false;
    }
    let enter = ctx.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.alt && !i.modifiers.ctrl);
    // ein per Tab fokussierter Knopf löst selbst aus (egui: Eingabe = Klick auf den fokussierten
    // Knopf), dann nicht zusätzlich den Standardknopf
    let buttons: std::collections::HashSet<Id> = ctx.data(|d| d.get_temp(Id::new("fp-dialog-buttons"))).unwrap_or_default();
    // (der fokussierte Standardknopf selbst zählt nicht: hat er den Fokus erst in diesem Bild
    // bekommen, klickt egui ihn nicht)
    let other_focus = ctx.memory(|m| m.focused()).is_some_and(|f| f != own && buttons.contains(&f));
    // auch wenn ein Feld den Fokus hat, das erst nach dem Knopf gezeichnet wird (Knöpfe vorn in der
    // Tab-Folge, D28): ob es den Fokus abgegeben hat, zählt im nächsten Bild
    if enter && !other_focus {
        ctx.data_mut(|d| d.insert_temp(key, nr));
        ctx.request_repaint();
    }
    false
}

/// Zeigt alle offenen Dialoge.
pub fn show_all(ctx: &egui::Context, st: &mut AppState) {
    new_document::show(ctx, st);
    unsaved::show(ctx, st);
    preferences::show(ctx, st);
    about::show(ctx, st);
    if let Some(p) = st.dialogs.pending_form.take() {
        match p {
            PendingForm::StrokeOptions => brush_edit::open(st, false),
            PendingForm::StrokeOptionsTool => brush_edit::open(st, true),
            PendingForm::NewStyle => st.dialogs.form = Some(misc::new_style_form(st)),
            PendingForm::StatusText => st.dialogs.form = catalog::form_for(st, "panel.behaviors.statusText"),
        }
    }
    if let Some(mut f) = st.dialogs.form.take() {
        match form::show_form(ctx, st, &mut f) {
            Some(_) => {}
            None => {
                if st.dialogs.form.is_none() {
                    st.dialogs.form = Some(f);
                }
            }
        }
    }
    brush_edit::show(ctx, st);
    doc_dialogs::show_canvas_size(ctx, st);
    doc_dialogs::show_image_size(ctx, st);
    doc_dialogs::show_numeric_transform(ctx, st);
    levels::show_levels(ctx, st);
    curves::show_curves(ctx, st);
    bitmap_selection::show(ctx, st);
    export::show_export(ctx, st);
    // vor der HTML-Einrichtung: die öffnet der Assistent darüber
    copy_html::show(ctx, st);
    html_setup::show(ctx, st);
    export_wizard::show(ctx, st);
    image_preview::show(ctx, st);
    shortcuts::show(ctx, st);
    misc::show_confirm(ctx, st);
    misc::show_text_editor(ctx, st);
    misc::show_html_slice(ctx, st);
    menu_dialogs::show_saved_commands(ctx, st);
    spelling::show_check(ctx, st);
    spelling_setup::show(ctx, st);
    file_info::show(ctx, st);
    popup_menu::show(ctx, st);
    models::show(ctx, st);
    trace::show(ctx, st);
    warning::show(ctx, st);
    script_prompt::show(ctx, st);
    show_message(ctx, st);
    save_workspace(ctx, st);
    if let Some(t) = st.dialogs.clipboard.take() {
        ctx.copy_text(t);
    }
    crate::inspector::popups::poll_pick(ctx, st);
}

fn show_message(ctx: &egui::Context, st: &mut AppState) {
    let Some((title, text)) = st.dialogs.message.clone() else { return };
    let mut done = false;
    // Info-Symbol 32 px links wie im Original (D25), Text daneben (kurz: mittig zum Symbol);
    // längere Meldungen (andere Sprachen): Höhe wächst mit dem umbrochenen Text
    let tw = 380.0 - 60.0 - 16.0;
    let th = wrapped_h(ctx, &text, tw);
    let r = dialog_frame(ctx, "fp-message", &title, vec2(380.0, 120.0f32.max(th.max(32.0) + 16.0 + 50.0).ceil()), |ui, body| {
        let theme = Theme::current(ui.ctx());
        let icon = Rect::from_min_size(body.min + vec2(16.0, 16.0), vec2(32.0, 32.0));
        crate::ui_kit::paint_icon(ui.painter(), icon, crate::ui_kit::KitIcon::Info, &crate::icons::IconStyle::new(&theme, None, true));
        let ty = if th < icon.height() { icon.center().y - th / 2.0 } else { icon.top() };
        put_left(ui, Rect::from_min_size(pos2(body.left() + 60.0, ty), vec2(tw, (body.bottom() - 50.0 - ty).max(th))), egui::Label::new(text.as_str()).wrap());
        let ok = t("btn-ok");
        let w = button_w(ui.ctx(), &ok, 76.0);
        if push_button(ui, Rect::from_min_size(pos2(body.right() - 16.0 - w, body.bottom() - 34.0), vec2(w, 23.0)), &ok, true, true) {
            done = true;
        }
    });
    if done || r == DialogClose::Cancelled {
        st.dialogs.message = None;
    }
}

fn save_workspace(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut name) = st.dialogs.save_workspace.clone() else { return };
    let mut result = None;
    let name_l = t("dlg-save-workspace-name");
    let dx = extra_w(ctx, &[&name_l], 50.0);
    let (ok_l, cancel_l) = (t("btn-ok"), t("btn-cancel"));
    let r = dialog_frame(ctx, "fp-save-ws", &t("dlg-save-workspace-title"), vec2(340.0 + dx, 110.0), |ui, body| {
        put_left(ui, Rect::from_min_size(body.min + vec2(16.0, 16.0), vec2(54.0 + dx, 20.0)), egui::Label::new(name_l.as_str()).extend());
        let te = ui.put(Rect::from_min_size(body.min + vec2(70.0 + dx, 14.0), vec2(250.0, 22.0)), egui::TextEdit::singleline(&mut name));
        te.request_focus();
        let b = button_row(ui.ctx(), body.right() - 16.0, body.bottom() - 34.0, &[&ok_l, &cancel_l], 76.0, 23.0, 8.0);
        let ok = push_button(ui, b[0], &ok_l, true, !name.trim().is_empty());
        if ok || (ui.input(|i| i.key_pressed(egui::Key::Enter)) && !name.trim().is_empty()) {
            result = Some(true);
        }
        if push_button(ui, b[1], &cancel_l, false, true) {
            result = Some(false);
        }
    });
    match (result, r) {
        (Some(true), _) => {
            st.dialogs.save_workspace = None;
            st.push(crate::app_state::Action::SaveWorkspaceAs(name.trim().to_string()));
        }
        (Some(false), _) | (_, DialogClose::Cancelled) => st.dialogs.save_workspace = None,
        _ => st.dialogs.save_workspace = Some(name),
    }
}
