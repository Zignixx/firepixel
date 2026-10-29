//! Oberfläche von Firepixel (Phase 3: Grundgerüst) – egui/eframe 0.36, detailgetreu nach dem
//! Hauptfenster des Originals (`re\ui-referenz\01_hauptfenster.png`, `re\11-bedienfelder-dialoge.md`).
//!
//! # Aufbau
//!
//! | Modul | Inhalt |
//! |---|---|
//! | [`theme`] | Design-System: Farben, Maße, Schriftgrößen; Designs Normal/Dunkel/OLED; Segoe UI |
//! | [`icons`] | eigene Vektor-Icons (Werkzeuge, Symbolleiste, Bedienfelder, Fußleiste) |
//! | [`widgets`] | Bausteine im Stil des Originals (Kopfzeilen, Trenner, Symbolknöpfe, Farbfelder) |
//! | [`appbar`] | Anwendungsleiste: Logo, Menüs, Schnellknöpfe, Arbeitsbereich-Umschalter, Fensterknöpfe |
//! | [`menu`] | Hauptmenü aus MENU 128 + Befehlsregister, Laufzeit-Untermenüs, Bedienfeldmenüs |
//! | [`toolbar`] | Hauptsymbolleiste „Standard“ |
//! | [`tools_panel`] | Werkzeugbedienfeld mit Flyouts und Farbfeldern |
//! | [`canvas`] | Dokument-Reiter, Ansichtsreiter, Leinwand (fp-render), Lineale, Fußleiste |
//! | [`inspector`] | Eigenschaften-Inspektor: Rahmen, Layout-Tabelle, Quadranten-Register |
//! | [`panels`] | Bedienfelder (Kennungen, Inhalte) |
//! | [`docking`] / [`workspace`] | Docking wie CS6, Arbeitsbereich-Layouts |
//! | [`dialogs`] | Neues Dokument, Ungespeichert-Abfrage, Voreinstellungen, Über |
//! | [`input`] | Tastenkürzel → fp-engine |
//! | [`files`] | Dateidialoge, Öffnen/Speichern, Drag & Drop |
//! | [`app_state`] / [`actions`] | Zustand und Aktions-Warteschlange |

pub mod actions;
pub mod app_menu; // macOS application menu (FB-05)
pub mod app_state;
pub mod appbar;
pub mod batch; // BATCH-2: Batch-Studio (Reiter „Batch“)
pub mod bridge;
pub mod builtin_templates; // Q-11: mitgelieferte Vorlagen für „Neu von Vorlage“
pub mod canvas;
pub mod dialogs;
pub mod docking;
pub mod file_dialog; // AU: Datei-Dialoge ohne Blockieren
pub mod files;
pub mod icons;
pub mod input;
pub mod inspector;
pub mod language;
pub mod menu;
pub mod menu_kbd;
pub mod menu_scroll;
pub mod os_clipboard;
pub mod panels;
pub mod playback; // FB-63: animation playback on the canvas
pub mod preview;
pub mod screen_mode; // TW-158: Standard / full screen with menus / full screen
pub mod screen_pick; // BA-114 (F5): eyedropper outside the window reads the screen
mod start_window;
pub mod theme;
pub mod tips; // FB-98: View > Tooltips keeps the names; delay preference
pub mod toolbar;
pub mod tools_panel;
pub mod ui_commands;
pub mod ui_kit;
pub mod welcome; // Agent DESIGN (Befund in BEFUNDE.md, Abschnitt I18N-SHELL)
pub mod widgets;
pub mod workspace;

use app_state::{Action, AppState, SETTINGS_KEY, UiSettings};
use egui::{Rect, pos2, vec2};
use fp_engine::{Preferences, Session};
use std::path::PathBuf;
use std::sync::Arc;
use theme::{Theme, ThemeKind};

/// Die Anwendung.
pub struct FirepixelApp {
    pub state: AppState,
    last_title: String,
    /// Sprachstand ([`fp_i18n::generation`]), für den `last_title`, Schriften und die
    /// Statuszeile gelten; ändert er sich, wird alles in der neuen Sprache neu aufgebaut.
    lang_generation: u64,
    fonts_ready: bool,
    /// `--design=…` gilt nur für diese Sitzung: (gespeichertes Design, Design aus der Kommandozeile).
    cli_design: Option<(fp_engine::Theme, fp_engine::Theme)>,
    /// Gezeichnete Frames. Die Fenstersymbole werden direkt nach dem Sichtbarwerden und 1,5 s
    /// später erneut gesetzt (`start_window::after_frame`), weil die Taskleiste ihren Knopf dann anlegt.
    frames: u32,
}

impl FirepixelApp {
    /// Start aus eframe: Schriften, Voreinstellungen, gespeicherter Arbeitsbereich, Dateien aus der
    /// Kommandozeile.
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        files: Vec<PathBuf>,
        design: Option<ThemeKind>,
    ) -> FirepixelApp {
        use fp_i18n::startup::{mark, time};
        mark("eframe: window + graphics ready (App::new)");
        time("fonts", || theme::install_fonts(&cc.egui_ctx));
        #[cfg(target_os = "windows")]
        if let Some(window) = cc.winit_window() {
            icons::apply_window_icons(window);
        }
        if let Some(window) = cc.winit_window() {
            start_window::prepare_hidden(window, cc.storage);
            file_dialog::set_parent_window(window.clone()); // AU: Besitzer der nativen Datei-Dialoge
        }
        mark("window icons + hidden placement");
        let prefs_path = Preferences::default_path();
        let mut prefs = time("preferences", || {
            prefs_path
                .as_deref()
                .map(Preferences::load_or_default)
                .unwrap_or_default()
        });
        dialogs::preferences::apply_original_dir(&prefs);
        if time("language (Fluent)", || language::init(&mut prefs)) != fp_i18n::SOURCE_LANG {
            time("fonts (language)", || theme::install_fonts(&cc.egui_ctx));
        }
        // eigene Befehle (Befehle-Menü) beim ersten Start in den Befehlsordner legen (einmal je
        // Ordner, gelöschte kommen nicht wieder; nur die echte App, nicht die Tests) – nach der
        // Sprachwahl, damit die Namen in der Sprache der Oberfläche stehen
        // Im Hintergrund (N30): das Prüfen/Schreiben der Dateien kostet beim Start 7–50 ms; das
        // Befehle-Menü liest den Ordner erst beim Öffnen.
        let spawned = std::thread::Builder::new()
            .name("fp-builtin-commands".into())
            .spawn(|| {
                if let Err(e) = time(
                    "command folder (background)",
                    fp_script::install_builtin_commands,
                ) {
                    log::warn!("Befehlsordner nicht angelegt: {e}");
                }
            });
        if spawned.is_err()
            && let Err(e) = fp_script::install_builtin_commands()
        {
            log::warn!("Befehlsordner nicht angelegt: {e}");
        }
        let mut cli_design = None;
        if let Some(d) = design {
            cli_design = Some((prefs.theme, d.to_engine()));
            prefs.theme = d.to_engine();
        }
        let mut settings: UiSettings = time("workspace settings", || {
            cc.storage
                .and_then(|s| eframe::get_value(s, SETTINGS_KEY))
                .unwrap_or_default()
        });
        settings.workspace.repair();
        // das Design steht in den Voreinstellungen von fp-engine (gemeinsam mit CLI/MCP)
        settings.theme = ThemeKind::from_engine(prefs.theme);
        // Register mit den Adapter-Befehlen der Oberfläche (Inspektor, Bedienfelder, Dialoge)
        let mut session = time("session + command registry", || {
            ui_commands::new_session(prefs)
        });
        session.prefs_path = prefs_path;
        let mut app = time("app state (renderer, script host)", || {
            FirepixelApp::with_session(&cc.egui_ctx, session, settings, true)
        });
        app.fonts_ready = true;
        app.cli_design = cli_design;
        for f in files {
            app.state.push(Action::OpenPath(f));
        }
        mark("App::new done");
        app
    }

    /// Anwendung ohne eframe (Tests, Bildschirmfotos): `threaded = false` rendert synchron.
    pub fn with_session(
        ctx: &egui::Context,
        session: Session,
        settings: UiSettings,
        threaded: bool,
    ) -> FirepixelApp {
        ctx.options_mut(|o| {
            o.zoom_with_keyboard = false;
        });
        fp_batch::install(); // BATCH-2: Befehle batch.* (auch über die MCP-Brücke)
        batch::register_templates(|| {
            fp_batch::templates::all()
                .into_iter()
                .map(|t| batch::Template {
                    title: t.title,
                    description: t.description,
                    recipe: t.recipe,
                })
                .collect()
        }); // BATCH-9: eingebaute Vorlagen
        let mut state = AppState::new(
            session,
            settings,
            if threaded { Some(ctx.clone()) } else { None },
        );
        state.sync_render = !threaded;
        FirepixelApp {
            state,
            last_title: String::new(),
            lang_generation: fp_i18n::generation(),
            fonts_ready: false,
            cli_design: None,
            frames: 0,
        }
    }

    /// Zeichnet das ganze Hauptfenster in `ui`.
    pub fn root_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        if !self.fonts_ready {
            // Schriften wirken erst ab dem nächsten Frame; vorher nichts zeichnen (fette Familie)
            theme::install_fonts(&ctx);
            self.fonts_ready = true;
            ctx.request_repaint();
            return;
        }
        // Sprachwechsel zur Laufzeit: selbst gespeicherte Texte verwerfen (Fenstertitel, Meldung der
        // Statuszeile) und die Schriften neu einrichten (Reihenfolge der CJK-Rückfallschriften)
        let lang_gen = fp_i18n::generation();
        if lang_gen != self.lang_generation {
            self.lang_generation = lang_gen;
            self.last_title.clear();
            self.state.status = None;
            theme::install_fonts(&ctx);
            ctx.request_repaint();
        }
        let st = &mut self.state;
        st.frame_counter += 1;
        st.painted_cursor = None;
        canvas::cursors::track_focus(&ctx);
        if st.theme_dirty {
            st.theme.apply(&ctx);
            st.theme_dirty = false;
        }
        // View ▸ Tooltips (FB-37, FB-98): tooltips of buttons, fields and panels from this frame on
        tips::apply(&ctx, &st.session.prefs, st.session.ui.tooltips);
        let theme: Arc<Theme> = Arc::clone(&st.theme);
        // fertige Renderergebnisse übernehmen
        for res in st.worker.poll() {
            // FB-97: an image for the pane whose cache is parked goes to the parked cache
            let cc = if st.cache_pane(res.key.doc) == res.key.pane {
                st.canvases.entry(res.key.doc).or_default()
            } else {
                st.pane_canvases
                    .entry((res.key.doc, res.key.pane))
                    .or_default()
            };
            if cc.requested.as_ref() == Some(&res.key) || cc.shown.is_none() {
                canvas::render::show_result(&ctx, cc, res);
            }
        }
        // Fenstertitel
        let title = st.window_title();
        if title != self.last_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.last_title = title;
        }
        // Schließen über das System (Alt+F4, Taskleiste): erst Rückfragen
        if ctx.input(|i| i.viewport().close_requested()) && !st.quit_confirmed {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            st.push(Action::Quit);
        }
        // macOS: Cmd+Q is the app's Quit command with the save prompt (FB-05, see `native_options`),
        // independent of the shortcut set and of a focused text field
        if cfg!(target_os = "macos") && !st.dialogs.any_open() && input::take_quit_key(&ctx) {
            st.push(Action::Quit);
        }
        // macOS application menu: About, Services, Hide, Quit with the save prompt (FB-05)
        if cfg!(target_os = "macos") {
            app_menu::frame(&ctx, st);
        }
        input::drop_alt_text(&ctx); // CT-A-06: Zugriffstasten (Alt+Buchstabe) schreiben nichts ins Feld
        batch::frame_input(&ctx, st); // BATCH-2: Ablegen/Tasten im Batch-Studio vor dem Dokument
        files::handle_dropped_files(&ctx, st);
        // Menüs per Tastatur (Alt+Buchstabe, F10, Pfeile …) vor den Kürzeln, Dokumentwechsel
        if !st.dialogs.any_open() {
            menu_kbd::keyboard(&ctx, st);
            switch_documents(&ctx, st);
        }
        // Klick bei offenem Popup gehört dem Popup, nicht der Leinwand (TW-52)
        canvas::note_popup_press(&ctx);
        // the inspector sets it again while a pop-up slider holds a pending value (FB-09/FB-10)
        st.inspector_preview = None;
        // FB-46: a press on the History panel discards an open pop-up slider before its click
        // jumps in the history; no pop-up commit of an earlier frame is left over
        inspector::preview::clear_commit_stamp();
        inspector::preview::discard_on_history_press(&ctx, st);
        input::handle(&ctx, st);
        // KI-Steuerkanal (MCP-Brücke, AU): Aufträge im UI-Thread abarbeiten
        crate::bridge::frame(&ctx, st);
        // FB-63: animation playback (advances the state after its delay)
        playback::frame(&ctx, st);
        // the selection the pop-up sliders of this frame start from (FB-46)
        inspector::preview::publish_target(&ctx, st);
        appbar::resize_edges(&ctx);

        let m = theme.metrics.clone();
        screen_mode::frame(st);
        let chrome = screen_mode::current(st);
        let hidden = st.settings.workspace.panels_hidden || st.session.ui.screen_mode == 2;
        // BATCH-2: Studio vorn → Werkzeuge, Dock und Inspektor ausblenden (Arbeitsbereich bleibt gespeichert)
        let studio = batch::is_front(st);
        let no_frame = egui::Frame::NONE;
        if chrome.app_bar {
            egui::Panel::top("fp-appbar")
                .exact_size(m.app_bar_height)
                .frame(no_frame)
                .resizable(false)
                .show_separator_line(false)
                .show(ui, |ui| appbar::show(ui, st));
        }
        if st.settings.workspace.show_toolbar && !hidden && chrome.toolbar {
            egui::Panel::top("fp-toolbar")
                .exact_size(m.toolbar_height)
                .frame(no_frame)
                .resizable(false)
                .show_separator_line(false)
                .show(ui, |ui| toolbar::show(ui, st));
        }
        if st.settings.workspace.show_tools && !hidden && !studio && chrome.tools_docked {
            egui::Panel::left("fp-tools")
                .exact_size(m.tools_width + m.splitter)
                .frame(no_frame)
                .resizable(false)
                .show_separator_line(false)
                .show(ui, |ui| {
                    let r = ui.max_rect();
                    let inner = Rect::from_min_max(r.min, pos2(r.right() - m.splitter, r.bottom()));
                    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
                    tools_panel::show(&mut child, st);
                    widgets::vsplitter(ui.painter(), r.right() - m.splitter, r.y_range(), &theme);
                });
        }
        if !hidden && !studio && chrome.dock_docked {
            // wie im Original: nicht schmaler als das breiteste angedockte Bedienfeld
            let dock_min = docking::dock_min_width(st, &theme);
            let dock_w = if st.settings.workspace.dock_mode == workspace::DockMode::Expanded {
                st.settings.workspace.dock_width.max(dock_min)
            } else if st.settings.workspace.dock_mode == workspace::DockMode::IconicWithNames {
                150.0
            } else {
                38.0
            };
            // Dock nie breiter als der Platz neben Werkzeugen und einer Mindestleinwand
            let tools_w = if st.settings.workspace.show_tools {
                m.tools_width + m.splitter
            } else {
                0.0
            };
            let max_dock =
                (ui.max_rect().width() - tools_w - 260.0).max(m.dock_min_width.min(160.0));
            let dock_w = dock_w.min(max_dock);
            let resp = egui::Panel::right("fp-dock")
                .exact_size(dock_w + m.splitter)
                .frame(no_frame)
                .resizable(false)
                .show_separator_line(false)
                .show(ui, |ui| {
                    let r = ui.max_rect();
                    let inner = Rect::from_min_max(pos2(r.left() + m.splitter, r.top()), r.max);
                    widgets::vsplitter(ui.painter(), r.left(), r.y_range(), &theme);
                    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
                    docking::show_dock(&mut child, st);
                });
            // Breite des Docks an der Trennlinie ziehen
            let r = resp.response.rect;
            let grip = Rect::from_min_size(r.min, vec2(m.splitter + 2.0, r.height()));
            let g = ui.interact(grip, egui::Id::new("dock-width"), egui::Sense::drag());
            if g.hovered() || g.dragged() {
                ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }
            if g.dragged() && st.settings.workspace.dock_mode == workspace::DockMode::Expanded {
                let w = (st.settings.workspace.dock_width.max(dock_min) - g.drag_delta().x)
                    .clamp(dock_min, 600.0f32.max(dock_min));
                st.settings.workspace.dock_width = w;
            }
        }
        if st.settings.workspace.show_inspector && !hidden && !studio && chrome.inspector {
            let room = ui.max_rect().height() - m.splitter;
            let h = inspector::height(&theme, st, room);
            let resp = egui::Panel::bottom("fp-inspector")
                .exact_size(h + m.splitter)
                .frame(no_frame)
                .resizable(false)
                .show_separator_line(false)
                .show(ui, |ui| {
                    let r = ui.max_rect();
                    widgets::hsplitter(ui.painter(), r.top(), r.x_range(), &theme);
                    let inner = Rect::from_min_max(pos2(r.left(), r.top() + m.splitter), r.max);
                    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
                    inspector::show(&mut child, st);
                });
            // Drag the top edge up or down (Firepixel extension TW-254, contributor PR #3). The
            // grip covers the splitter and 2 px of the document area above it, never the tab row.
            let r = resp.response.rect;
            let grip = Rect::from_min_max(
                pos2(r.left(), r.top() - 2.0),
                pos2(r.right(), r.top() + m.splitter),
            );
            let g = ui.interact(grip, egui::Id::new("inspector-height"), egui::Sense::drag());
            if g.hovered() || g.dragged() {
                ctx.set_cursor_icon(egui::CursorIcon::ResizeVertical);
            }
            if g.dragged()
                && let Some(p) = g.interact_pointer_pos()
            {
                // absolute from the pointer, so snapping to the discrete heights is not sticky
                let max_h = (room - inspector::DOC_MIN_VISIBLE)
                    .max(theme.metrics.inspector_header_height + 2.0);
                let new_h = (r.bottom() - p.y - m.splitter * 0.5)
                    .clamp(theme.metrics.inspector_header_height + 2.0, max_h);
                inspector::apply_dragged_height(
                    &mut st.settings.workspace,
                    theme.metrics.inspector_header_height,
                    theme.metrics.inspector_row_height,
                    new_h,
                );
            }
        }
        let doc_area = egui::CentralPanel::no_frame()
            .show(ui, |ui| {
                canvas::show_document_area(ui, st);
            })
            .response
            .rect;
        if !hidden {
            screen_mode::show_floating_chrome(&ctx, st, doc_area);
        }
        docking::show_floating(&ctx, st);
        docking::finish_drag(&ctx, st);
        dialogs::show_all(&ctx, st);
        // Öffnet/schließt eine Aktion am Ende des Bildes einen Dialog (z. B. Strg+W → Rückfrage),
        // sofort noch ein Bild zeichnen, auch ohne Folgeereignis (z. B. Aufträge der MCP-Brücke)
        // Noch im selben Bild zeigen: egui verwirft diesen Durchgang und zeichnet sofort einen
        // zweiten (ohne die schon verarbeiteten Eingaben), statt auf das nächste Bild zu warten –
        // so steht die Rückfrage nach Strg+W ein Bild früher da (P-HG1)
        let dialogs_before = st.dialogs.any_open();
        let title_before = st.window_title();
        st.process_actions(&ctx);
        st.handle_session_events();
        // a View ▸ Tooltips toggled by this frame's actions holds from the next pass on
        tips::apply(&ctx, &st.session.prefs, st.session.ui.tooltips);
        if st.dialogs.any_open() != dialogs_before || st.window_title() != title_before {
            ctx.request_discard("dialog or document changed by an action");
            ctx.request_repaint();
        }
        if st.close_window {
            st.close_window = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        // Vollbild-Modi (Werkzeugbedienfeld ▸ Ansicht)
        let want_full = st.session.ui.screen_mode > 0;
        let is_full = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
        if want_full != is_full {
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(want_full));
        }
    }
}

impl FirepixelApp {
    /// Schließen über die Taskleiste bzw. WM_CLOSE bei minimiertem Fenster (aus `logic`, das eframe
    /// dann als Einziges aufruft): mit ungespeicherten Dokumenten nicht sofort beenden (eframe
    /// schlösse ohne `CancelClose` gleich), sondern das Fenster wiederherstellen und den
    /// Schließwunsch merken; die Rückfrage stellt `root_ui` im nächsten Bild. Das Schließen-Ereignis
    /// selbst kommt dort nicht mehr an (CT-3: Fenster kam zurück, aber ohne Rückfrage). Liefert, ob
    /// der Schließwunsch übernommen wurde.
    pub fn close_while_minimized(
        &mut self,
        ctx: &egui::Context,
        minimized: bool,
        close: bool,
    ) -> bool {
        // a running batch also asks first (BA-106): "Batch is still running – cancel and close?"
        let ask = self.state.any_dirty()
            || crate::batch::studio(&self.state).is_some_and(|s| s.running());
        if !(minimized && close && !self.state.quit_confirmed && ask) {
            return false;
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        // eine zweite Anfrage, solange die Rückfrage offen ist, legt keine zweite an (Action::Quit)
        self.state.push(Action::Quit);
        ctx.request_repaint();
        true
    }
}

/// Freeze after minimise/restore on macOS (FB-04).
///
/// Root cause (egui-winit 0.36 / eframe 0.36 / winit 0.30): `ViewportCommand::Minimized(true)`
/// sets `ViewportInfo::minimized = Some(true)`, but on macOS `egui_winit::update_viewport_info`
/// deliberately never refreshes `minimized`/`maximized` after window creation (egui issue 3494),
/// and winit sends no event when the window is restored from the Dock (only `Occluded(false)`).
/// The flag therefore stays `Some(true)`, `ViewportInfo::visible()` stays `Some(false)`, and eframe's
/// `run_ui_and_paint` keeps taking its "nothing will be shown" branch: it only calls
/// [`eframe::App::logic`], never `ui`, and never paints – the restored window looks frozen.
///
/// `logic` still runs on every event in that state, so it compares the flag with the real window
/// state and, when the window is no longer minimised, clears the flag with
/// `ViewportCommand::Minimized(false)` (a no-op for the window itself) and asks for a new frame.
/// Returns whether the flag was stale. On Windows/Linux eframe refreshes the flag before every pass,
/// so the two never disagree there; the call site is macOS-only anyway.
pub fn heal_stale_minimized(ctx: &egui::Context, window_minimized: Option<bool>) -> bool {
    let flagged = ctx.input(|i| i.viewport().minimized == Some(true));
    if !(flagged && window_minimized == Some(false)) {
        return false;
    }
    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
    ctx.request_repaint();
    true
}

impl eframe::App for FirepixelApp {
    /// Läuft vor jedem Bild, bei minimiertem Fenster als Einziges (eframe ruft dann `ui` nicht).
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let (minimized, close) = ctx.input(|i| {
            (
                i.viewport().minimized == Some(true),
                i.viewport().close_requested(),
            )
        });
        // macOS (FB-04): heal a stale "minimised" flag after the window came back from the Dock
        #[cfg(target_os = "macos")]
        if let Some(window) = _frame.winit_window() {
            heal_stale_minimized(ctx, window.is_minimized());
        }
        self.close_while_minimized(ctx, minimized, close);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.frames = self.frames.saturating_add(1);
        // Startzeit-Messung (FP_STARTUP_TRACE): Beginn und Ende der ersten Frames
        let trace = self.frames <= 12 && fp_i18n::startup::enabled();
        if trace {
            fp_i18n::startup::mark(&format!("frame {} begin", self.frames));
        }
        self.root_ui(ui);
        if trace {
            fp_i18n::startup::mark(&format!("frame {} ui done", self.frames));
        }
        // Fenster nach dem Sichtbarwerden maximieren; Symbole dann erneut setzen (und 1,5 s später)
        let window = _frame.winit_window().map(|w| &**w);
        if start_window::after_frame(ui.ctx(), window) {
            #[cfg(target_os = "windows")]
            if let Some(window) = window {
                icons::apply_window_icons(window);
                // Die Taskleiste legt ihren Knopf an, während der UI-Thread noch zeichnet, bekommt
                // auf WM_GETICON keine Antwort und behält das Ersatzsymbol; WM_SETICON allein lässt
                // sie nicht neu abfragen, ein (unveränderter) WM_SETTEXT schon (nachgewiesen 11.10.).
                window.set_title(&window.title());
            }
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        // ein per Kommandozeile erzwungenes Design nicht dauerhaft speichern (außer der Nutzer hat es
        // in der Sitzung selbst umgestellt)
        let restore = self
            .cli_design
            .filter(|(_, cli)| self.state.session.prefs.theme == *cli)
            .map(|(saved, _)| saved);
        let mut settings = self.state.settings.clone();
        if let Some(saved) = restore {
            settings.theme = ThemeKind::from_engine(saved);
            let cur = std::mem::replace(&mut self.state.session.prefs.theme, saved);
            let _ = self.state.session.save_prefs();
            self.state.session.prefs.theme = cur;
        } else {
            let _ = self.state.session.save_prefs();
        }
        eframe::set_value(storage, SETTINGS_KEY, &settings);
    }

    /// Beenden (Strg+Q, WM_CLOSE, Alt+F4): KI-Steuerkanal anhalten, Token-Datei löschen (CT-C-10).
    fn on_exit(&mut self) {
        crate::bridge::shutdown();
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        self.state.theme.colors.frame_bg.to_normalized_gamma_f32()
    }
}

/// Strg+Tab bzw. Strg+F6 → nächstes Dokument, Strg+Umschalt+Tab bzw. Strg+Umschalt+F6 → voriges
/// (MDI-Standard wie im Original). Die Taste wird verbraucht.
fn switch_documents(ctx: &egui::Context, st: &mut AppState) {
    let mut dir = 0i32;
    ctx.input_mut(|i| {
        // mit Umschalt zuerst: `consume_key` vergleicht „logisch“ und nähme Strg+Umschalt+Tab sonst
        // schon als Strg+Tab (vorwärts) weg (CT-B-03)
        for (key, shift) in [
            (egui::Key::Tab, true),
            (egui::Key::F6, true),
            (egui::Key::Tab, false),
            (egui::Key::F6, false),
        ] {
            let m = egui::Modifiers {
                ctrl: true,
                command: true,
                shift,
                ..Default::default()
            };
            if i.consume_key(m, key) {
                dir = if shift { -1 } else { 1 };
            }
        }
    });
    // F132: Shift+F6 = Window ▸ Next Pane while the active document is split (Ctrl+Tab and
    // Ctrl+F6 cycle the documents; Shift+F6 is free in all five shortcut sets)
    if st.session.active().is_some_and(|d| d.is_split())
        && !st.session.tool_is_editing_text()
        && ctx.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, egui::Key::F6))
    {
        st.push(app_state::Action::Run(
            "window.nextPane".into(),
            serde_json::json!({}),
        ));
    }
    // Umschalt+W: Werkzeuge am Platz des Zauberstabs durchschalten (Zauberstab, KI-Auswahl)
    if !st.session.tool_is_editing_text()
        && ctx.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, egui::Key::W))
    {
        let members = fp_engine::tools::flyout_members("Zauberstab");
        if !members.is_empty() {
            let cur = members.iter().position(|t| *t == st.session.tools.active);
            let next = members[cur.map(|i| (i + 1) % members.len()).unwrap_or(0)];
            st.push(app_state::Action::SetTool(next));
        }
    }
    if dir == 0 {
        return;
    }
    let docs: Vec<fp_engine::DocId> = st.session.documents().iter().map(|d| d.id).collect();
    if docs.len() < 2 {
        return;
    }
    let cur = st
        .session
        .active_id()
        .and_then(|a| docs.iter().position(|d| *d == a))
        .unwrap_or(0) as i32;
    let n = docs.len() as i32;
    let next = docs[((cur + dir).rem_euclid(n)) as usize];
    st.push(app_state::Action::Activate(next));
}

/// Fensteroptionen: ohne Systemrahmen (eigene Anwendungsleiste wie CS6), maximiert.
pub fn native_options() -> eframe::NativeOptions {
    #[allow(unused_mut)]
    let mut options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(fp_i18n::DISPLAY_NAME)
            .with_app_id("firepixel")
            // own chrome on Windows/Linux; native title bar with traffic lights on macOS (FB-03)
            .with_decorations(appbar::native_title_bar())
            .with_inner_size([1500.0, 950.0])
            .with_min_inner_size([800.0, 500.0])
            .with_maximized(true)
            .with_drag_and_drop(true)
            .with_icon(Arc::new(icons::app_icon_data())),
        persist_window: true,
        // Fensterlage und Arbeitsbereich unter %APPDATA%\Firepixel\data wie die Voreinstellungen.
        // eframe fragt sonst den Systemordner direkt ab und übergeht eine umgesetzte
        // APPDATA-Variable: Testinstanzen mit eigenem APPDATA überschrieben dann die Fensterlage
        // des Nutzers (beobachtet 12.10.). In Testläufen das Testprofil (fp_i18n::profile, EB-3).
        persistence_path: fp_i18n::profile::roaming_dir().map(|d| d.join("data").join("app.ron")),
        // maximiert erst nach dem Erscheinen (sonst blitzt das Fenster beim Start zweimal auf)
        window_builder: Some(Box::new(start_window::strip_maximized)),
        wgpu_options: wgpu_options(),
        ..Default::default()
    };
    // macOS (FB-05): winit 0.30 installs a default application menu whose "Quit" item has the key
    // equivalent Cmd+Q and the action `terminate:`. AppKit handles menu key equivalents before the
    // key reaches the window, so the app never sees Cmd+Q; `terminate:` then runs winit's
    // `applicationWillTerminate:` (winit's delegate has no `applicationShouldTerminate:`), which
    // exits the event loop directly – there is no `CloseRequested`, so the unsaved-changes prompt
    // was skipped. The default menu is therefore off; `app_menu` installs Firepixel's own app menu
    // (muda, safe API) whose Quit item runs `Action::Quit` with the prompt, and `root_ui` also turns
    // a Cmd+Q key event into `Action::Quit` as a fallback.
    #[cfg(target_os = "macos")]
    {
        options.event_loop_builder = Some(Box::new(|builder| {
            use winit::platform::macos::EventLoopBuilderExtMacOS as _;
            builder.with_default_menu(false);
        }));
    }
    options
}

/// Grafik-Einrichtung (START-1): wgpu tastet sonst beim Start Vulkan, DX12 und OpenGL ab; auf
/// Rechnern ohne Vulkan/GL kostet das ≈ 35 ms. Unter Windows richtet Firepixel deshalb zuerst DX12
/// allein ein (Instanz, Adapter – auch WARP, der Software-Rasterer –, Gerät, mit denselben
/// Gerätewünschen wie egui) und übergibt das fertig an eframe. Klappt das nicht, bleibt alles wie
/// bisher (alle Backends, Rückfall auf Vulkan/GL). `WGPU_BACKEND` hat Vorrang;
/// `FP_WGPU_ALL_BACKENDS=1` schaltet die Vorauswahl ab.
fn wgpu_options() -> eframe::egui_wgpu::WgpuConfiguration {
    #[allow(unused_mut)]
    let mut cfg = eframe::egui_wgpu::WgpuConfiguration::default();
    #[cfg(windows)]
    {
        use eframe::egui_wgpu::{WgpuSetup, wgpu};
        let skip = std::env::var_os("WGPU_BACKEND").is_some()
            || std::env::var_os("FP_WGPU_ALL_BACKENDS").is_some_and(|v| v != "0" && !v.is_empty());
        if !skip
            && let WgpuSetup::CreateNew(create_new) = &cfg.wgpu_setup
            && create_new
                .instance_descriptor
                .backends
                .contains(wgpu::Backends::DX12)
            && let Some(existing) =
                fp_i18n::startup::time("wgpu: DX12 setup", || dx12_setup(create_new))
        {
            cfg.wgpu_setup = WgpuSetup::Existing(existing);
        }
    }
    cfg
}

/// Ein Future einmal abfragen (wgpu liefert nativ sofort); `None`, wenn es noch nicht fertig ist.
#[cfg(windows)]
fn poll_once<F: std::future::Future>(fut: F) -> Option<F::Output> {
    let mut fut = std::pin::pin!(fut);
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    match fut.as_mut().poll(&mut cx) {
        std::task::Poll::Ready(v) => Some(v),
        std::task::Poll::Pending => None,
    }
}

/// DX12 allein einrichten wie egui-wgpu (Energievorgabe, Gerätewünsche aus `create_new`).
#[cfg(windows)]
fn dx12_setup(
    create_new: &eframe::egui_wgpu::WgpuSetupCreateNew,
) -> Option<eframe::egui_wgpu::WgpuSetupExisting> {
    use eframe::egui_wgpu::wgpu;
    let base = &create_new.instance_descriptor;
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::DX12,
        flags: base.flags,
        backend_options: base.backend_options.clone(),
        memory_budget_thresholds: base.memory_budget_thresholds,
        display: None,
    });
    let options = wgpu::RequestAdapterOptions {
        power_preference: create_new.power_preference,
        force_fallback_adapter: false,
        compatible_surface: None,
        apply_limit_buckets: false,
    };
    let adapter = poll_once(instance.request_adapter(&options))?.ok()?;
    let desc = (create_new.device_descriptor)(&adapter);
    let (device, queue) = poll_once(adapter.request_device(&desc))?.ok()?;
    Some(eframe::egui_wgpu::WgpuSetupExisting {
        instance,
        adapter,
        device,
        queue,
    })
}

/// Hilfetext der Kommandozeile (`firepixel --help`) in der aktuellen Sprache.
pub fn cli_help() -> String {
    fp_i18n::t("ui-cli-help")
}

/// Schnellerer Start (Nutzerbefund N30, START-1): Was `FirepixelApp::new` braucht, aber nicht vom
/// Fenster abhängt, wird schon parallel zur Fenster- und Grafik-Einrichtung (winit, wgpu) in
/// Hintergrund-Threads geladen: die Schriftdateien der Oberfläche (≈ 50 MB), der Index der
/// Systemschriften (Vorgabeschrift des Textwerkzeugs) und die eingebauten Sprachdateien. Alle drei
/// liegen in prozessweiten `OnceLock`s; wer sie danach braucht, wartet höchstens auf den Rest und
/// bekommt dasselbe Ergebnis wie ohne Vorladen.
fn prewarm() {
    let jobs: [(&str, fn()); 3] = [
        ("ui fonts", theme::prewarm_fonts),
        ("system font index", || drop(fp_text::FontDb::shared())),
        ("language files", || drop(fp_i18n::languages())),
    ];
    for (name, job) in jobs {
        let spawned = std::thread::Builder::new()
            .name(format!("fp-prewarm-{}", name.replace(' ', "-")))
            .spawn(move || {
                fp_i18n::startup::time(&format!("prewarm: {name}"), job);
            });
        if spawned.is_err() {
            log::warn!("Vorladen beim Start ließ sich nicht starten: {name}");
        }
    }
}

/// Startet die Anwendung (Dateien aus der Kommandozeile werden geöffnet; `design` überschreibt
/// das gespeicherte Design, z. B. für Bildschirmfotos).
pub fn run(files: Vec<PathBuf>, design: Option<ThemeKind>) -> eframe::Result {
    // Rechner ohne Grafikkarte (Software-Rasterer, z. B. in Fernsitzungen): egui-wgpu tastet Texturen
    // mit `textureLoad` ab und schreibt Vertexdaten direkt – sonst brauchte dort jedes Bild 0,2–0,7 s
    // und getippte Zeichen erschienen 0,6–1,2 s verzögert (FV-8, `vendor\egui-wgpu`). Wirkt nur auf
    // Software-Rasterern.
    eframe::egui_wgpu::SOFTWARE_RASTERIZER_TUNING.store(true, std::sync::atomic::Ordering::Relaxed);
    prewarm();
    fp_i18n::startup::mark("eframe::run_native");
    let r = eframe::run_native(
        "Firepixel",
        native_options(),
        Box::new(move |cc| Ok(Box::new(FirepixelApp::new(cc, files, design)))),
    );
    // auch ohne `on_exit` (Fehler, Abbruch der Ereignisschleife): Token-Datei nicht liegen lassen
    crate::bridge::shutdown();
    r
}
