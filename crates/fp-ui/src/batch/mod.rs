//! Batch-Studio (SPEC-BATCH `docs\BATCH_MODERN.md` §2, Paket BATCH-2): moderner Batch-Modus als
//! Reiter „Batch“ im Dokumentbereich des Hauptfensters. Gestaltung wie der Startbildschirm
//! (DESIGN §12.1): Banner, drei Karten (① Dateien, ② Rezept, ③ Vorschau), unten die Ausgabe-Leiste.
//!
//! ```text
//! ╭ Banner: [Logo] Batch-Verarbeitung · 23 Dateien · Rezept     [Anfänger|Profi] [Rezepte ▾] [⋯] ╮
//! ╭ ① Dateien ─────────╮ ╭ ② Rezept ───────────────────╮ ╭ ③ Vorschau ─────────────────────╮
//! │ [+ Dateien…][▾]    │ │ ⠿ ▣ Schritt         [●] ⋯ ▾ │ │ [Vorher|Nachher|Vergleich|Plan] │
//! │ [Filter…] [Typ▾]   │ │   Zusammenfassung           │ │   Bild (Teiler, Zoom)           │
//! │ ▢ [36] name.jpg  ✓ │ │      [+ Schritt hinzufügen] │ │ Vorher 4032×3024 · 3,1 MB       │
//! │ 23 Dateien         │ │ ▣ Dateiname   ▣ Speichern … │ │ Nachher 1600×1200 · 284 KB      │
//! ╰────────────────────╯ ╰─────────────────────────────╯ ╰─────────────────────────────────╯
//!  Ziel [Neuer Ordner ▾] [Ordner…]  Bei Konflikt [Umbenennen ▾]   [Trockenlauf] ▕███░░▏ [▶ Start]
//! ```
//!
//! Zustand: [`Studio`] in `AppState::batch` (höchstens ein Studio). Öffnen über *Datei ▸
//! Batch-Verarbeitung…* (`file.batchProcess` ohne Parameter), [`open`]/[`open_with_files`].
//! Solange das Studio vorn ist, blendet das Hauptfenster Werkzeuge, Dock und Inspektor aus
//! ([`is_front`]). Die Arbeit macht `fp-batch` (Rezept, Plan, Lauf, Vorschau).
//!
//! Schnittstellen für spätere Pakete: eigene Kartenformulare je Schritt über
//! [`params::register_card_body`], Vorlagen über [`register_templates`]; siehe
//! `docs\berichte\batch2.md`. Texte: Präfix `batch-ui-` in `locales\<sprache>\batch.ftl`.

pub mod banner;
pub mod files;
pub mod kit;
pub mod output_bar;
pub mod params;
pub mod preview;
pub mod recipe;
pub mod rename_ui;
pub mod report_ui;
pub mod timeline; // BATCH-7
mod thumbs;

use crate::app_state::AppState;
use crate::theme::Theme;
use crate::ui_kit::{sp, Tone};
use egui::{pos2, vec2, Id, Key, Modifiers, Rect, Ui};
use fp_batch::{ItemInfo, Recipe, StepRegistry};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Voreinstellung: zuletzt gewählter Modus (`beginner`/`pro`).
pub const PREF_MODE: &str = "BatchMode";
/// Voreinstellung: Ordner rekursiv hinzufügen.
pub const PREF_RECURSIVE: &str = "BatchRecursive";
/// Höchstzahl gemerkter Rezept-Zustände für Rückgängig.
const UNDO_LIMIT: usize = 200;

/// Zustand einer Datei in der Liste.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryStatus {
    Waiting,
    Running,
    Done,
    Warning,
    Skipped,
    Failed,
    Cancelled,
}

/// Eine Datei der Liste.
#[derive(Clone, Debug)]
pub struct FileEntry {
    pub info: ItemInfo,
    /// Aktiviert (Kästchen): nur aktivierte, sichtbare Dateien werden verarbeitet.
    pub enabled: bool,
    pub status: EntryStatus,
    /// Meldung (Fehler, Warnung, Grund fürs Überspringen).
    pub message: Option<String>,
    /// Ziel nach dem Lauf.
    pub target: Option<PathBuf>,
    /// Reihenfolge des Hinzufügens.
    pub added: usize,
}

/// Anfänger- oder Profi-Modus (SPEC §2.10).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Beginner,
    Pro,
}

/// Welche Karte im Rezept aufgeklappt ist (Akkordeon).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardId {
    Step(usize),
    Rename,
    Output,
}

/// Kurzer Hinweis im Studio (verschwindet nach einigen Sekunden).
#[derive(Clone, Debug)]
pub struct Notice {
    pub tone: Tone,
    pub text: String,
    pub until: f64,
}

/// Ergebnisse von Datei-Dialogen (laufen ohne Blockieren; die Antwort kommt über diesen Eingang).
#[derive(Clone, Debug)]
pub(crate) enum Inbox {
    AddPaths(Vec<PathBuf>, bool),
    LoadRecipe(PathBuf),
    SaveRecipe(PathBuf),
    TargetFolder(PathBuf),
    SaveReport(PathBuf),
    ParamPath { step: usize, param: String, path: PathBuf },
}

/// Vorlage (eingebaut oder von BATCH-9 geliefert) für das Menü „Rezepte“.
#[derive(Clone, Debug)]
pub struct Template {
    pub title: String,
    pub description: String,
    pub recipe: Recipe,
}

/// Lieferant der Vorlagen (BATCH-9: eingebaute Vorlagen aus `fp-batch\data\recipes`).
pub type TemplateSource = fn() -> Vec<Template>;

fn template_sources() -> &'static std::sync::RwLock<Vec<TemplateSource>> {
    static S: std::sync::OnceLock<std::sync::RwLock<Vec<TemplateSource>>> = std::sync::OnceLock::new();
    S.get_or_init(|| std::sync::RwLock::new(Vec::new()))
}

/// Meldet eine Quelle für Vorlagen an (einmal je Prozess; doppelte werden ignoriert).
pub fn register_templates(f: TemplateSource) {
    if let Ok(mut v) = template_sources().write()
        && !v.iter().any(|g| std::ptr::fn_addr_eq(*g, f))
    {
        v.push(f);
    }
}

/// Alle angemeldeten Vorlagen.
pub fn templates() -> Vec<Template> {
    template_sources().read().map(|v| v.iter().flat_map(|f| f()).collect()).unwrap_or_default()
}

/// Der Zustand des Studios.
pub struct Studio {
    /// Reiter „Batch“ vorn (sonst die Dokumente).
    pub front: bool,
    pub mode: Mode,
    pub files: Vec<FileEntry>,
    /// Markierte Datei (Index in `files`), Ziel der Vorschau.
    pub selected: Option<usize>,
    pub filter: files::Filter,
    pub sort: files::SortKey,
    /// Ordner mit Unterordnern hinzufügen.
    pub recursive: bool,
    pub recipe: Recipe,
    /// Datei des Rezepts (gespeichert/geladen).
    pub recipe_path: Option<PathBuf>,
    saved_json: String,
    undo: Vec<Recipe>,
    redo: Vec<Recipe>,
    edit_key: Option<String>,
    pub open_card: Option<CardId>,
    /// „Mehr Optionen“ in der offenen Karte (Anfänger-Modus).
    pub show_more: bool,
    pub gallery: Option<recipe::Gallery>,
    pub preview: preview::PreviewState,
    pub view_tab: preview::Tab,
    pub run: Option<output_bar::RunState>,
    pub report: Option<fp_batch::Report>,
    /// Ende-Karte über der Vorschau sichtbar.
    pub show_summary: bool,
    pub plan: Option<fp_batch::Plan>,
    pub plan_error: Option<String>,
    pub notice: Option<Notice>,
    /// Reiter in schmalen Fenstern (0 = Dateien, 1 = Rezept, 2 = Vorschau).
    pub narrow_tab: usize,
    /// Zieht gerade Dateien über das Fenster.
    pub drop_hover: bool,
    /// Breite der Dateien-Karte (ziehbar).
    pub files_w: f32,
    /// Anteil der Vorschau-Karte an der Breite (ziehbar).
    pub preview_frac: f32,
    pub(crate) inbox: Arc<Mutex<Vec<Inbox>>>,
    pub(crate) tab_rect: Option<Rect>,
    pub(crate) drag: Option<recipe::DragState>,
    pub(crate) last_active_doc: Option<fp_engine::DocId>,
    pub(crate) files_version: u64,
    order_cache: Option<(u64, Vec<usize>)>,
    issues_cache: Option<(String, Vec<fp_batch::Issue>)>,
    next_added: usize,
    pub(crate) close_requested: bool,
    /// Dateiliste zur markierten Datei blättern (Pfeiltasten).
    pub(crate) scroll_to_selected: bool,
}

impl Studio {
    /// Neues, leeres Studio.
    pub fn new(mode: Mode) -> Studio {
        rename_ui::register();
        let recipe = Recipe::default();
        let saved_json = recipe.to_json();
        Studio {
            front: true,
            mode,
            files: Vec::new(),
            selected: None,
            filter: files::Filter::default(),
            sort: files::SortKey::Added,
            recursive: true,
            recipe,
            recipe_path: None,
            saved_json,
            undo: Vec::new(),
            redo: Vec::new(),
            edit_key: None,
            open_card: None,
            show_more: false,
            gallery: None,
            preview: preview::PreviewState::default(),
            view_tab: preview::Tab::Compare,
            run: None,
            report: None,
            show_summary: false,
            plan: None,
            plan_error: None,
            notice: None,
            narrow_tab: 0,
            drop_hover: false,
            files_w: 300.0,
            preview_frac: 0.38,
            inbox: Arc::new(Mutex::new(Vec::new())),
            tab_rect: None,
            drag: None,
            last_active_doc: None,
            files_version: 0,
            order_cache: None,
            issues_cache: None,
            next_added: 0,
            close_requested: false,
            scroll_to_selected: false,
        }
    }

    // ------------------------------------------------------------------ Dateien

    /// Fügt Dateien bzw. Ordner hinzu. Liefert (neu, doppelt, übersprungen).
    pub fn add_paths(&mut self, paths: &[PathBuf], recursive: bool) -> (usize, usize, usize) {
        let reg = StepRegistry::global();
        // andere Dateiarten nur, wenn das Rezept ausdrücklich nur Dateischritte hat (z. B. nur kopieren/umbenennen)
        let allow_other = fp_batch::pipeline::is_file_only(&self.recipe, &reg) && self.recipe.enabled_steps().any(|(_, s)| reg.get(&s.id).is_some_and(|st| st.file_only()));
        let existing: Vec<ItemInfo> = self.files.iter().map(|f| f.info.clone()).collect();
        let c = fp_batch::collect(paths, recursive, allow_other, &existing);
        let n = c.items.len();
        for info in c.items {
            self.files.push(FileEntry { info, enabled: true, status: EntryStatus::Waiting, message: None, target: None, added: self.next_added });
            self.next_added += 1;
        }
        if self.selected.is_none() && n > 0 {
            self.selected = Some(self.files.len() - n);
        }
        self.files_changed();
        (n, c.duplicates.len(), c.skipped.len())
    }

    /// Fügt hinzu und meldet Doppelte/Übersprungene als Hinweis.
    pub fn add_paths_notice(&mut self, paths: &[PathBuf], recursive: bool, now: f64) {
        let (_, dup, skipped) = self.add_paths(paths, recursive);
        let mut parts = Vec::new();
        if dup > 0 {
            parts.push(fp_i18n::tr!("batch-ui-files-duplicates", n = dup));
        }
        if skipped > 0 {
            parts.push(fp_i18n::tr!("batch-ui-files-skipped", n = skipped));
        }
        if !parts.is_empty() {
            self.notify(Tone::Info, parts.join(" · "), now);
        }
    }

    /// Entfernt Dateien (Indizes in `files`).
    pub fn remove_files(&mut self, idx: &[usize]) {
        let mut sorted: Vec<usize> = idx.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        let sel_path = self.selected.and_then(|i| self.files.get(i)).map(|f| f.info.path.clone());
        for i in sorted.into_iter().rev() {
            if i < self.files.len() {
                self.files.remove(i);
            }
        }
        self.selected = sel_path.and_then(|p| self.files.iter().position(|f| f.info.path == p));
        if self.selected.is_none() {
            self.selected = self.visible().first().copied();
        }
        self.files_changed();
    }

    /// Leert die Liste.
    pub fn clear_files(&mut self) {
        self.files.clear();
        self.selected = None;
        self.files_changed();
    }

    pub(crate) fn files_changed(&mut self) {
        self.files_version += 1;
        self.order_cache = None;
        self.plan = None;
        self.preview.invalidate();
    }

    /// Sichtbare Dateien (Filter, Sortierung) als Indizes in `files`.
    pub fn visible(&mut self) -> Vec<usize> {
        let key = self.files_version ^ self.filter.version().rotate_left(17) ^ (self.sort as u64).rotate_left(40);
        if let Some((k, v)) = &self.order_cache
            && *k == key
        {
            return v.clone();
        }
        let v = files::order(&self.files, &self.filter, self.sort, self.mode);
        self.order_cache = Some((key, v.clone()));
        v
    }

    /// Dateien, die verarbeitet werden (sichtbar und aktiviert), in Listenreihenfolge.
    pub fn to_process(&mut self) -> Vec<usize> {
        self.visible().into_iter().filter(|&i| self.files[i].enabled).collect()
    }

    // ------------------------------------------------------------------ Rezept

    /// Ändert das Rezept über `f` (liefert `true` bei Änderung). Gleicher `key` in einer Geste
    /// (Schieber ziehen, Tippen im selben Feld) ergibt **einen** Rückgängig-Schritt (§4b).
    pub fn edit(&mut self, key: &str, f: impl FnOnce(&mut Recipe) -> bool) -> bool {
        let before = self.recipe.clone();
        if !f(&mut self.recipe) || self.recipe == before {
            return false;
        }
        if self.edit_key.as_deref() != Some(key) {
            self.undo.push(before);
            if self.undo.len() > UNDO_LIMIT {
                self.undo.remove(0);
            }
            self.redo.clear();
            self.edit_key = Some(key.to_string());
        }
        self.recipe_changed();
        true
    }

    /// Beendet die laufende Geste (nächste Änderung wird ein neuer Rückgängig-Schritt).
    pub fn end_gesture(&mut self) {
        self.edit_key = None;
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Rückgängig im Rezept (eigener Verlauf, nicht der Dokumentverlauf).
    pub fn undo(&mut self) -> bool {
        let Some(prev) = self.undo.pop() else { return false };
        self.redo.push(std::mem::replace(&mut self.recipe, prev));
        self.edit_key = None;
        self.fix_open_card();
        self.recipe_changed();
        true
    }

    /// Wiederholen im Rezept.
    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else { return false };
        self.undo.push(std::mem::replace(&mut self.recipe, next));
        self.edit_key = None;
        self.fix_open_card();
        self.recipe_changed();
        true
    }

    /// Zahl der Einträge im Rezept-Verlauf (Tests).
    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    fn fix_open_card(&mut self) {
        if let Some(CardId::Step(i)) = self.open_card
            && i >= self.recipe.steps.len()
        {
            self.open_card = None;
        }
    }

    pub(crate) fn recipe_changed(&mut self) {
        self.plan = None;
        self.plan_error = None;
        self.issues_cache = None;
        self.order_cache = None;
        self.preview.invalidate();
    }

    /// Neues Rezept (z. B. geladen oder Vorlage); Verlauf beginnt neu.
    pub fn set_recipe(&mut self, recipe: Recipe, path: Option<PathBuf>) {
        self.recipe = recipe;
        self.saved_json = self.recipe.to_json();
        self.recipe_path = path;
        self.undo.clear();
        self.redo.clear();
        self.edit_key = None;
        self.open_card = None;
        self.recipe_changed();
    }

    /// Ungespeicherte Änderungen am Rezept?
    pub fn is_dirty(&self) -> bool {
        self.recipe.to_json() != self.saved_json
    }

    /// Als gespeichert merken.
    pub fn mark_saved(&mut self, path: PathBuf) {
        self.saved_json = self.recipe.to_json();
        self.recipe_path = Some(path);
    }

    /// Prüfergebnis des Rezepts (zwischengespeichert bis zur nächsten Änderung).
    pub fn issues(&mut self) -> Vec<fp_batch::Issue> {
        let json = self.recipe.to_json();
        if let Some((k, v)) = &self.issues_cache
            && *k == json
        {
            return v.clone();
        }
        let v = self.recipe.validate(&StepRegistry::global());
        self.issues_cache = Some((json, v.clone()));
        v
    }

    /// Hat das Rezept Fehler (Verarbeiten gesperrt)?
    pub fn has_errors(&mut self) -> bool {
        self.issues().iter().any(|i| i.severity == fp_batch::Severity::Error)
    }

    /// Name des Rezepts für die Anzeige.
    pub fn recipe_title(&self) -> String {
        if !self.recipe.name.trim().is_empty() {
            return self.recipe.name.clone();
        }
        match &self.recipe_path {
            Some(p) => recipe_name_from_path(p),
            None => fp_i18n::t("batch-ui-recipe-untitled"),
        }
    }

    /// Hinweis zeigen.
    pub fn notify(&mut self, tone: Tone, text: impl Into<String>, now: f64) {
        self.notice = Some(Notice { tone, text: text.into(), until: now + 4.0 });
    }

    /// Läuft gerade ein Auftrag?
    pub fn running(&self) -> bool {
        self.run.is_some()
    }

    pub(crate) fn inbox_sender(&self) -> Arc<Mutex<Vec<Inbox>>> {
        Arc::clone(&self.inbox)
    }

    fn take_inbox(&self) -> Vec<Inbox> {
        std::mem::take(&mut *self.inbox.lock().unwrap_or_else(|e| e.into_inner()))
    }
}

impl Drop for Studio {
    fn drop(&mut self) {
        // Studio geschlossen: laufenden Auftrag sauber abbrechen (Temporärdateien werden verworfen)
        if let Some(r) = &self.run {
            r.cancel();
        }
        self.preview.cancel();
    }
}

/// Name eines Rezepts aus dem Dateinamen (ohne `.fpbatch.json`).
pub fn recipe_name_from_path(p: &Path) -> String {
    let n = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let ext = format!(".{}", fp_batch::recipe::RECIPE_EXTENSION);
    n.strip_suffix(&ext).or_else(|| n.strip_suffix(".json")).unwrap_or(&n).to_string()
}

// ---------------------------------------------------------------------- Öffentliche Haken

/// Öffnet das Studio (bzw. holt es nach vorn).
pub fn open(st: &mut AppState) {
    if st.batch.is_none() {
        let mode = if st.session.prefs.get_str(PREF_MODE, "beginner") == "pro" { Mode::Pro } else { Mode::Beginner };
        let mut s = Studio::new(mode);
        s.recursive = st.session.prefs.get_bool(PREF_RECURSIVE, true);
        s.last_active_doc = st.session.active_id();
        st.batch = Some(Box::new(s));
    }
    if let Some(s) = st.batch.as_mut() {
        s.front = true;
        s.last_active_doc = st.session.active_id();
    }
}

/// Öffnet das Studio mit Dateien (Kommandozeile `--batch`, JSF, Ziehen auf den Reiter).
pub fn open_with_files(st: &mut AppState, paths: &[PathBuf]) {
    open(st);
    if let Some(s) = st.batch.as_mut() {
        let r = s.recursive;
        s.add_paths(paths, r);
    }
}

// ------------------------------------------------------------------ Start mit `--batch` (BATCH-9)

/// Start-Auftrag aus der Kommandozeile: Rezept (Pfad oder Vorlagenname) und Dateien/Ordner.
type Launch = (Option<String>, Vec<PathBuf>);

fn launch_slot() -> &'static std::sync::Mutex<Option<Launch>> {
    static L: std::sync::OnceLock<std::sync::Mutex<Option<Launch>>> = std::sync::OnceLock::new();
    L.get_or_init(|| std::sync::Mutex::new(None))
}

/// `firepixel --batch [rezept] [dateien…]`: Das Studio öffnet sich im ersten Frame mit dem Rezept
/// (Pfad zu `*.fpbatch.json` oder Name einer eingebauten Vorlage) und den Dateien/Ordnern.
pub fn request_launch(recipe: Option<String>, files: Vec<PathBuf>) {
    if let Ok(mut l) = launch_slot().lock() {
        *l = Some((recipe, files));
    }
}

/// Führt einen angemeldeten Start-Auftrag einmal aus (aus [`frame_input`]).
pub fn apply_launch(st: &mut AppState) {
    let Some((recipe, files)) = launch_slot().lock().ok().and_then(|mut l| l.take()) else { return };
    launch(st, recipe.as_deref(), &files);
}

/// Öffnet das Studio mit Dateien und Rezept (Pfad zu `*.fpbatch.json` oder Vorlagenname). Liefert
/// `false`, wenn das Rezept weder Datei noch Vorlage ist (Studio offen, Rezept unverändert).
pub fn launch(st: &mut AppState, recipe: Option<&str>, files: &[PathBuf]) -> bool {
    open_with_files(st, files);
    let Some(name) = recipe else { return true };
    let path = PathBuf::from(name);
    let loaded = if path.is_file() { Recipe::load(&path).ok().map(|r| (r, Some(path))) } else { fp_batch::templates::recipe(name).map(|r| (r, None)) };
    match (loaded, st.batch.as_mut()) {
        (Some((r, p)), Some(s)) => {
            s.set_recipe(r, p);
            true
        }
        _ => false,
    }
}

/// Schließt das Studio (bricht einen laufenden Auftrag ab).
pub fn close(st: &mut AppState) {
    st.batch = None;
}

/// Ist das Studio offen?
pub fn is_open(st: &AppState) -> bool {
    st.batch.is_some()
}

/// Ist das Studio vorn (dann blendet das Hauptfenster Werkzeuge, Dock und Inspektor aus)?
pub fn is_front(st: &AppState) -> bool {
    st.batch.as_ref().is_some_and(|s| s.front)
}

/// Eingaben vor den Tastenkürzeln des Hauptfensters (in `root_ui` vor `files::handle_dropped_files`):
/// abgelegte Dateien gehen ins Studio, wenn es vorn ist oder sie auf den Reiter „Batch“ fallen;
/// Tasten des Studios (Strg+Z/Y auf das Rezept, Entf, Pfeile, Strg+Eingabe, F5, Strg+O) werden
/// verbraucht, damit sie nicht auf das verdeckte Dokument wirken.
pub fn frame_input(ctx: &egui::Context, st: &mut AppState) {
    apply_launch(st); // BATCH-9: `firepixel --batch …`
    let dialogs_open = st.dialogs.any_open();
    let Some(s) = st.batch.as_mut() else { return };
    s.drop_hover = s.front && ctx.input(|i| !i.raw.hovered_files.is_empty());
    let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).collect());
    if !dropped.is_empty() {
        let pos = ctx.input(|i| i.pointer.latest_pos());
        let on_tab = pos.zip(s.tab_rect).is_some_and(|(p, r)| r.contains(p));
        if (s.front || on_tab) && !s.running() {
            s.front = true;
            let now = ctx.input(|i| i.time);
            let r = s.recursive;
            s.add_paths_notice(&dropped, r, now);
            ctx.input_mut(|i| i.raw.dropped_files.clear());
        }
    }
    if !s.front || dialogs_open || ctx.text_edit_focused() || egui::Popup::is_any_open(ctx) {
        return;
    }
    let now = ctx.input(|i| i.time);
    keyboard(ctx, st, now);
}

fn consume(ctx: &egui::Context, m: Modifiers, k: Key) -> bool {
    ctx.input_mut(|i| i.consume_key(m, k))
}

fn keyboard(ctx: &egui::Context, st: &mut AppState, now: f64) {
    let Some(s) = st.batch.as_mut() else { return };
    let ctrl = Modifiers::COMMAND;
    // Esc schließt Überlagerungen (Galerie, Ende-Karte), nie den Lauf
    if s.gallery.is_some() {
        if consume(ctx, Modifiers::NONE, Key::Escape) {
            s.gallery = None;
        }
        return;
    }
    if s.show_summary && consume(ctx, Modifiers::NONE, Key::Escape) {
        s.show_summary = false;
    }
    if !s.running() {
        if consume(ctx, ctrl, Key::Z) {
            s.undo();
        }
        if consume(ctx, ctrl, Key::Y) || consume(ctx, ctrl | Modifiers::SHIFT, Key::Z) {
            s.redo();
        }
        // Alt+↑/↓ verschiebt die offene Schrittkarte
        if let Some(CardId::Step(i)) = s.open_card {
            if consume(ctx, Modifiers::ALT, Key::ArrowUp) && i > 0 && s.edit("move", |r| r.move_step(i, i - 1)) {
                s.open_card = Some(CardId::Step(i - 1));
                s.end_gesture();
            }
            if consume(ctx, Modifiers::ALT, Key::ArrowDown) && i + 1 < s.recipe.steps.len() && s.edit("move", |r| r.move_step(i, i + 1)) {
                s.open_card = Some(CardId::Step(i + 1));
                s.end_gesture();
            }
        }
        // Dateiliste: ↑/↓ wählt, Entf entfernt, Strg+A aktiviert alle
        let vis = s.visible();
        let pos = s.selected.and_then(|sel| vis.iter().position(|&i| i == sel));
        if consume(ctx, Modifiers::NONE, Key::ArrowDown) && !vis.is_empty() {
            s.selected = Some(vis[pos.map(|p| (p + 1).min(vis.len() - 1)).unwrap_or(0)]);
            s.scroll_to_selected = true;
        }
        if consume(ctx, Modifiers::NONE, Key::ArrowUp) && !vis.is_empty() {
            s.selected = Some(vis[pos.map(|p| p.saturating_sub(1)).unwrap_or(0)]);
            s.scroll_to_selected = true;
        }
        if (consume(ctx, Modifiers::NONE, Key::Delete) || consume(ctx, Modifiers::NONE, Key::Backspace))
            && let Some(sel) = s.selected
        {
            s.remove_files(&[sel]);
        }
        if consume(ctx, ctrl, Key::A) {
            let all_on = s.files.iter().all(|f| f.enabled);
            for f in &mut s.files {
                f.enabled = !all_on;
            }
            s.files_changed();
        }
        if consume(ctx, Modifiers::NONE, Key::F5) {
            output_bar::dry_run(s, now);
        }
        if consume(ctx, ctrl, Key::Enter) {
            output_bar::start(st, now);
            return;
        }
        if consume(ctx, ctrl, Key::O) {
            files::pick_files(st);
            return;
        }
    }
    // Pfeile links/rechts nie an das verdeckte Dokument (Objekte verschieben)
    let _ = consume(ctx, Modifiers::NONE, Key::ArrowLeft) || consume(ctx, Modifiers::NONE, Key::ArrowRight);
}

/// Reiter „Batch“ links im Dokument-Reiterstreifen (in `canvas::doc_tabs`). Liefert den Rest des
/// Streifens für die Dokumentreiter.
pub fn doc_tab(ui: &mut Ui, st: &mut AppState, rect: Rect) -> Rect {
    let active = st.session.active_id();
    let Some(s) = st.batch.as_mut() else { return rect };
    // ein Dokument wurde aktiviert, geöffnet oder angelegt → Dokument nach vorn
    if active != s.last_active_doc {
        if active.is_some() {
            s.front = false;
        }
        s.last_active_doc = active;
    }
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let label = fp_i18n::t("batch-ui-tab");
    let font = theme.bold(theme.fonts.ui);
    let w = (crate::widgets::text_width(ui, &label, &font) + 16.0 + 6.0 + 34.0).min(200.0);
    let tr = Rect::from_min_size(pos2(rect.left() + 8.0, rect.top() + 2.0), vec2(w, rect.height() - 2.0));
    s.tab_rect = Some(tr);
    let resp = ui.interact(tr, Id::new("fp-batch-doc-tab"), egui::Sense::click());
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, s.front, &label));
    let p = ui.painter();
    if s.front {
        p.rect_filled(tr, 0.0, c.doc_tab_active_bg);
        p.rect_stroke(tr, 0.0, egui::Stroke::new(1.0, c.doc_tab_border), egui::StrokeKind::Inside);
        p.hline(tr.x_range().shrink(1.0), tr.bottom() - 0.5, egui::Stroke::new(1.0, c.doc_tab_active_bg));
        if theme.glow {
            p.rect_filled(Rect::from_min_size(tr.min, vec2(tr.width(), 2.0)), 0.0, c.group_accent[0]);
        }
    } else if resp.hovered() {
        p.rect_filled(tr.shrink(1.0), 0.0, c.tab_hover_bg);
    }
    if s.drop_hover || (ui.input(|i| !i.raw.hovered_files.is_empty()) && resp.hovered()) {
        p.rect_stroke(tr.shrink(1.0), 0.0, egui::Stroke::new(1.5, c.focus), egui::StrokeKind::Inside);
    }
    let ir = Rect::from_min_size(pos2(tr.left() + 7.0, tr.center().y - 7.0), vec2(14.0, 14.0));
    kit::paint_icon(p, ir, crate::ui_kit::RowIcon::Kit(crate::ui_kit::KitIcon::Commands), &theme, true);
    crate::widgets::text_clipped(p, pos2(ir.right() + 5.0, tr.center().y), egui::Align2::LEFT_CENTER, &label, font, c.doc_tab_text, w - 30.0 - 19.0);
    let xr = Rect::from_center_size(pos2(tr.right() - 11.0, tr.center().y), vec2(12.0, 12.0));
    let xresp = ui.interact(xr, Id::new("fp-batch-doc-tab-x"), egui::Sense::click());
    let xc = if xresp.hovered() { c.icon_red } else { c.text_weak };
    crate::icons::paint(p, xr.shrink(1.0), crate::icons::Icon::Close, &crate::icons::IconStyle::mono(xc, c.frame_bg));
    let close_label = fp_i18n::t("batch-ui-close");
    crate::ui_kit::icon_button_info(&xresp, true, &close_label, None);
    let tip = fp_i18n::t("batch-ui-tab.tooltip");
    if xresp.on_hover_text(close_label).clicked() || resp.middle_clicked() {
        s.close_requested = true;
    } else if resp.on_hover_text(tip).clicked() {
        s.front = true;
    }
    // Klick auf einen Dokumentreiter holt die Dokumente nach vorn
    let rest = Rect::from_min_max(pos2(tr.right() + 1.0 - 8.0, rect.top()), rect.max);
    if s.front && ui.input(|i| i.pointer.primary_clicked() && i.pointer.interact_pos().is_some_and(|p| rest.contains(p) && p.x > tr.right() + 2.0)) && active.is_some() {
        s.front = false;
    }
    let close = s.close_requested;
    if close {
        close_studio(st);
    }
    rest
}

fn close_studio(st: &mut AppState) {
    st.batch = None;
}

// ---------------------------------------------------------------------- Zeichnen

/// Aufteilung der Fläche.
#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub banner: Rect,
    pub bar: Rect,
    /// 3 = drei Spalten, 2 = Dateien | Rezept/Vorschau, 1 = Reiter.
    pub columns: usize,
    pub files: Rect,
    pub recipe: Rect,
    pub preview: Rect,
    /// Reiterleiste (nur bei weniger als drei Spalten).
    pub tabs: Option<Rect>,
}

/// Mindestbreiten (SPEC §2.2).
pub const FILES_MIN_W: f32 = 260.0;
pub const FILES_MAX_W: f32 = 380.0;
pub const RECIPE_MIN_W: f32 = 340.0;
pub const PREVIEW_MIN_W: f32 = 300.0;
pub const PREVIEW_MAX_W: f32 = 640.0;

impl Layout {
    /// Teilt `rect` (Dokumentbereich unter den Reitern) auf.
    pub fn compute(rect: Rect, files_w: f32, preview_frac: f32) -> Layout {
        let narrow = rect.width() < 720.0;
        let margin = if narrow { sp::M } else { sp::L };
        let bar_h = output_bar::height(rect.width());
        let bar = Rect::from_min_max(pos2(rect.left(), rect.bottom() - bar_h), rect.max);
        let inner = Rect::from_min_max(pos2(rect.left() + margin, rect.top() + margin), pos2(rect.right() - margin, bar.top() - margin));
        let banner_h = if narrow || rect.height() < 560.0 { 56.0 } else { 72.0 };
        let banner = Rect::from_min_size(inner.min, vec2(inner.width(), banner_h));
        let body = Rect::from_min_max(pos2(inner.left(), banner.bottom() + margin), inner.max);
        let gap = margin;
        let w = body.width();
        let three = w >= FILES_MIN_W + RECIPE_MIN_W + PREVIEW_MIN_W + 2.0 * gap;
        let two = !three && w >= FILES_MIN_W + RECIPE_MIN_W + gap;
        if three {
            let mut fw = files_w.clamp(FILES_MIN_W, FILES_MAX_W);
            let mut pw = (w * preview_frac).clamp(PREVIEW_MIN_W, PREVIEW_MAX_W);
            let over = (fw + pw + RECIPE_MIN_W + 2.0 * gap) - w;
            if over > 0.0 {
                let take_p = over.min(pw - PREVIEW_MIN_W);
                pw -= take_p;
                fw -= (over - take_p).min(fw - FILES_MIN_W);
            }
            let files = Rect::from_min_size(body.min, vec2(fw.floor(), body.height()));
            let preview = Rect::from_min_max(pos2((body.right() - pw).floor(), body.top()), body.max);
            let recipe = Rect::from_min_max(pos2(files.right() + gap, body.top()), pos2(preview.left() - gap, body.bottom()));
            Layout { banner, bar, columns: 3, files, recipe, preview, tabs: None }
        } else if two {
            let fw = files_w.clamp(FILES_MIN_W, (w - RECIPE_MIN_W - gap).max(FILES_MIN_W)).min(FILES_MAX_W).floor();
            let files = Rect::from_min_size(body.min, vec2(fw, body.height()));
            let right = Rect::from_min_max(pos2(files.right() + gap, body.top()), body.max);
            let tabs = Rect::from_min_size(right.min, vec2(right.width(), 28.0));
            let card = Rect::from_min_max(pos2(right.left(), tabs.bottom() + sp::S), right.max);
            Layout { banner, bar, columns: 2, files, recipe: card, preview: card, tabs: Some(tabs) }
        } else {
            let tabs = Rect::from_min_size(body.min, vec2(body.width(), 28.0));
            let card = Rect::from_min_max(pos2(body.left(), tabs.bottom() + sp::S), body.max);
            Layout { banner, bar, columns: 1, files: card, recipe: card, preview: card, tabs: Some(tabs) }
        }
    }
}

/// Zeichnet das Studio in `rect` (Dokumentbereich unter den Reitern).
pub fn show(ui: &mut Ui, st: &mut AppState, rect: Rect) {
    let ctx = ui.ctx().clone();
    let now = ctx.input(|i| i.time);
    let theme = Theme::current(&ctx);
    ui.painter().rect_filled(rect, 0.0, theme.colors.canvas_bg);
    if st.batch.is_none() || rect.width() < 80.0 || rect.height() < 80.0 {
        return;
    }
    poll(&ctx, st, now);
    let Some(mut s) = st.batch.take() else { return };
    let lay = Layout::compute(rect, s.files_w, s.preview_frac);
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(egui::Layout::top_down(egui::Align::Min)));
    child.set_clip_rect(rect.intersect(ui.clip_rect()));
    let ui = &mut child;
    banner::show(ui, st, &mut s, lay.banner, now);
    let running = s.running();
    match lay.columns {
        3 => {
            files::show(ui, st, &mut s, lay.files, now, running);
            recipe::show(ui, st, &mut s, lay.recipe, now, running);
            preview::show(ui, st, &mut s, lay.preview, now);
            splitters(ui, &mut s, &lay);
        }
        2 => {
            files::show(ui, st, &mut s, lay.files, now, running);
            if let Some(t) = lay.tabs {
                let labels = vec![fp_i18n::t("batch-ui-recipe-title"), fp_i18n::t("batch-ui-preview-title")];
                let cur = s.narrow_tab.clamp(1, 2) - 1;
                if let Some(i) = kit::segmented(ui, t, Id::new("fp-batch-tabs2"), &labels, cur, false) {
                    s.narrow_tab = i + 1;
                }
            }
            if s.narrow_tab == 2 {
                preview::show(ui, st, &mut s, lay.preview, now);
            } else {
                recipe::show(ui, st, &mut s, lay.recipe, now, running);
            }
        }
        _ => {
            if let Some(t) = lay.tabs {
                let labels = vec![fp_i18n::t("batch-ui-files-title"), fp_i18n::t("batch-ui-recipe-title"), fp_i18n::t("batch-ui-preview-title")];
                if let Some(i) = kit::segmented(ui, t, Id::new("fp-batch-tabs1"), &labels, s.narrow_tab.min(2), false) {
                    s.narrow_tab = i;
                }
            }
            match s.narrow_tab {
                0 => files::show(ui, st, &mut s, lay.files, now, running),
                1 => recipe::show(ui, st, &mut s, lay.recipe, now, running),
                _ => preview::show(ui, st, &mut s, lay.preview, now),
            }
        }
    }
    output_bar::show(ui, st, &mut s, lay.bar, now);
    output_bar::ask_overlay(ui, &mut s, rect);
    // Geste beendet: Maus los und kein Feld im Fokus → nächste Änderung ist ein neuer Schritt
    if !ui.input(|i| i.pointer.any_down()) && !ui.ctx().text_edit_focused() {
        s.end_gesture();
    }
    preview::schedule(&ctx, &mut s, now);
    if s.notice.as_ref().is_some_and(|n| n.until < now) {
        s.notice = None;
    }
    if s.running() || s.preview.busy() || thumbs::pending(&ctx) {
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
    if s.notice.is_some() {
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
    }
    if !s.close_requested {
        st.batch = Some(s);
    }
}

/// Trennlinien zwischen den Spalten ziehen (Breite der Dateien- bzw. Vorschau-Karte).
fn splitters(ui: &mut Ui, s: &mut Studio, lay: &Layout) {
    let a = Rect::from_min_max(pos2(lay.files.right() + 2.0, lay.files.top()), pos2(lay.recipe.left() - 2.0, lay.files.bottom()));
    let b = Rect::from_min_max(pos2(lay.recipe.right() + 2.0, lay.files.top()), pos2(lay.preview.left() - 2.0, lay.files.bottom()));
    let ra = ui.interact(a, Id::new("fp-batch-split-files"), egui::Sense::drag());
    let rb = ui.interact(b, Id::new("fp-batch-split-preview"), egui::Sense::drag());
    if ra.hovered() || ra.dragged() || rb.hovered() || rb.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }
    if ra.dragged() {
        s.files_w = (s.files_w + ra.drag_delta().x).clamp(FILES_MIN_W, FILES_MAX_W);
    }
    if rb.dragged() {
        let total = (lay.preview.right() - lay.files.left()).max(1.0);
        let pw = (lay.preview.width() - rb.drag_delta().x).clamp(PREVIEW_MIN_W, PREVIEW_MAX_W);
        s.preview_frac = (pw / total).clamp(0.2, 0.6);
    }
}

/// Ergebnisse abholen: Datei-Dialoge, Fortschritt des Laufs, Vorschau.
fn poll(ctx: &egui::Context, st: &mut AppState, now: f64) {
    let Some(s) = st.batch.as_mut() else { return };
    for msg in s.take_inbox() {
        match msg {
            Inbox::AddPaths(p, rec) => s.add_paths_notice(&p, rec, now),
            Inbox::LoadRecipe(p) => banner::load_recipe(s, &p, now),
            Inbox::SaveRecipe(p) => banner::save_recipe(s, &p, now),
            Inbox::TargetFolder(p) => {
                s.edit("target", |r| {
                    r.output.target.kind = fp_batch::recipe::TargetKind::Folder;
                    r.output.target.path = Some(p.clone());
                    true
                });
                s.end_gesture();
            }
            Inbox::SaveReport(p) => report_ui::save_report(s, &p, now),
            Inbox::ParamPath { step, param, path } => {
                s.edit("path", |r| r.set_param(step, &param, serde_json::json!(path.to_string_lossy())));
                s.end_gesture();
            }
        }
    }
    output_bar::poll(ctx, s, now);
    preview::poll(ctx, s);
}

/// Hilfsfunktion für Tests und die Brücke: Studio-Zustand (falls offen).
pub fn studio(st: &AppState) -> Option<&Studio> {
    st.batch.as_deref()
}

/// Veränderbarer Studio-Zustand (Tests).
pub fn studio_mut(st: &mut AppState) -> Option<&mut Studio> {
    st.batch.as_deref_mut()
}

/// Werden noch Vorschaubilder der Dateiliste gelesen? (Tests, Bildserien.)
pub fn thumbnails_pending(ctx: &egui::Context) -> bool {
    thumbs::pending(ctx)
}
