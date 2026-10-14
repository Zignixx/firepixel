//! Ausführung der Aktionen: Befehle aus dem Register, wobei die Oberfläche die Befehle abfängt, die
//! einen Dialog brauchen (Neu, Öffnen, Speichern unter, Schließen mit Rückfrage, Voreinstellungen,
//! Fenster-Befehle für Bedienfelder) – ganz wie das Original, das diese Befehle über App/Hauptfenster
//! leitet (10 §4.1 Nr. 3a).

use crate::app_state::{Action, AppState};
use crate::dialogs::{NewDocumentDialog, PendingClose, PreferencesDialog, UnsavedFlow};
use crate::docking::panel_from_name;
use crate::panels::PanelId;
use crate::workspace::{DockMode, Workspace};
use fp_engine::EngineError;
use serde_json::{json, Value};

fn now(ctx: &egui::Context) -> f64 {
    ctx.input(|i| i.time)
}

fn has_param(p: &Value, key: &str) -> bool {
    p.get(key).is_some_and(|v| !v.is_null())
}

/// Menüzustand für Befehle, die die Oberfläche selbst führt: (aktiv, Häkchen).
pub fn ui_command_state(st: &AppState, name: &str) -> Option<(bool, Option<bool>)> {
    let ws = &st.settings.workspace;
    if let Some(p) = PanelId::from_command(name) {
        // Häkchen wie im Original nur für das vorderste Bedienfeld seiner sichtbaren Gruppe
        return Some((true, Some(ws.is_visible(p) && ws.is_front(p))));
    }
    match name {
        "window.toolbars.main" => Some((true, Some(ws.show_toolbar))),
        "window.tools" => Some((true, Some(ws.show_tools))),
        "window.properties" => Some((true, Some(ws.show_inspector))),
        "window.hidePanels" => Some((true, Some(ws.panels_hidden))),
        n if n.starts_with("ui.") => Some((true, None)),
        "file.new" | "file.open" | "file.newFromTemplate" | "help.about" => Some((true, None)),
        "file.saveAs" | "file.saveAsTemplate" | "file.close" | "file.closeAll" => Some((st.session.active().is_some(), None)),
        // wie im Original: Speichern/Wiederherstellen grau, solange das Dokument unverändert ist
        // (nur der Menüzustand; Skripte/MCP dürfen den Befehl weiter ausführen)
        "file.save" => Some((st.session.active().is_some_and(|d| d.is_dirty()), None)),
        "file.revert" => Some((st.session.active().is_some_and(|d| d.is_dirty() && d.path.is_some()), None)),
        _ => None,
    }
}

/// Führt eine Aktion aus.
pub fn execute(st: &mut AppState, ctx: &egui::Context, a: Action) {
    match a {
        Action::Run(name, params) => run_command(st, ctx, &name, params),
        Action::SetTool(t) => {
            if let Err(e) = st.session.set_tool(t) {
                st.set_status(e.to_string(), now(ctx));
            }
        }
        Action::SetTheme(k) => st.set_theme(k),
        Action::TogglePanel(p) => {
            st.settings.workspace.toggle(p);
            st.sync_panels_to_engine();
        }
        Action::ShowPanel(p) => {
            st.settings.workspace.show(p);
            st.dock.prefer = Some(p);
            st.sync_panels_to_engine();
        }
        Action::Activate(id) => {
            let _ = st.session.activate(id);
        }
        Action::CloseDocument(id) => close_documents(st, vec![id], PendingClose::CloseDocs),
        Action::OpenPath(p) => crate::files::open_path(st, &p),
        Action::SetZoom(z) => {
            if let Err(e) = st.session.execute("view.setZoom", json!({ "zoom": z })) {
                st.set_status(e.to_string(), now(ctx));
            }
        }
        // Seiten-/Statuswechsel über das Befehlsregister (wie Menü, CLI, MCP und JSF)
        Action::SetPage(i) => {
            if st.session.active().is_some_and(|d| d.page != i) {
                run_command(st, ctx, "page.setCurrent", json!({ "index": i }));
            }
        }
        Action::SetFrame(i) => {
            if st.session.active().is_some_and(|d| d.frame != i) {
                run_command(st, ctx, "state.setCurrent", json!({ "index": i }));
            }
        }
        Action::SetColors { stroke, fill } => st.session.set_colors(stroke, fill),
        Action::ResetWorkspace => {
            st.settings.workspace = Workspace::expanded();
            st.sync_panels_to_engine();
        }
        Action::LoadWorkspace(name) => {
            let found = Workspace::presets()
                .into_iter()
                .find(|w| w.name == name)
                .or_else(|| st.settings.named_workspaces.iter().find(|(n, _)| *n == name).map(|(_, w)| w.clone()))
                .or_else(|| {
                    // Layout-Datei im XML-Format des Originals
                    let p = crate::workspace::layouts_dir()?.join(format!("{name}.xml"));
                    let text = std::fs::read_to_string(p).ok()?;
                    Workspace::from_panel_state_xml(&text, &name).ok()
                });
            if let Some(mut w) = found {
                w.name = name;
                w.repair();
                st.settings.workspace = w;
                st.sync_panels_to_engine();
            }
        }
        Action::SaveWorkspaceAs(name) => {
            let mut w = st.settings.workspace.clone();
            w.name = name.clone();
            st.settings.named_workspaces.retain(|(n, _)| *n != name);
            // zusätzlich als XML im Format des Originals (Layoutordner)
            if let Some(dir) = crate::workspace::layouts_dir() {
                fp_i18n::profile::guard_write(&dir);
                let _ = std::fs::create_dir_all(&dir);
                if let Err(e) = std::fs::write(dir.join(format!("{name}.xml")), w.to_panel_state_xml()) {
                    st.set_status(fp_i18n::tr!("ui-workspace-save-failed", error = e.to_string()), now(ctx));
                }
            }
            st.settings.named_workspaces.push((name.clone(), w));
            st.settings.workspace.name = name;
        }
        Action::Quit => {
            if crate::batch::settings::intercept_quit(st) {
                return; // BATCH-11a: Batch läuft noch → erst Rückfrage im Studio
            }
            let ids: Vec<_> = st.session.documents().iter().map(|d| d.id).collect();
            if ids.iter().any(|id| st.session.document(*id).is_some_and(|d| d.is_dirty())) {
                st.dialogs.unsaved = Some(UnsavedFlow::new(ids, PendingClose::Quit));
            } else {
                st.quit_confirmed = true;
                st.close_window = true;
            }
        }
        Action::Status(s) => st.set_status(s, now(ctx)),
    }
}

fn close_documents(st: &mut AppState, ids: Vec<fp_engine::DocId>, mode: PendingClose) {
    if ids.is_empty() {
        return;
    }
    st.dialogs.unsaved = Some(UnsavedFlow::new(ids, mode));
}

fn run_command(st: &mut AppState, ctx: &egui::Context, name: &str, params: Value) {
    // Bedienfelder (Fenster-Menü)
    if let Some(p) = PanelId::from_command(name) {
        execute(st, ctx, Action::TogglePanel(p));
        return;
    }
    if crate::language::run_action(st, ctx, name, &params) {
        return;
    }
    match name {
        "file.new" if !has_param(&params, "width") => {
            st.dialogs.new_document = Some(NewDocumentDialog::from_prefs(&st.session.prefs).with_clipboard(&st.session));
            return;
        }
        "file.open" if !has_param(&params, "path") => {
            // mit „Als ‚Unbenannt‘ öffnen“/„Als Animation öffnen“ (Dialog in der App, NZ-4)
            crate::files::pick_and_open(st);
            return;
        }
        "file.newFromTemplate" if !has_param(&params, "path") => {
            let n = name.to_string();
            crate::files::pick(st, crate::files::DialogKind::NewFromTemplate, "", move |st, paths| {
                if let Some(p) = paths.into_iter().next() {
                    st.run(&n, json!({ "path": p.to_string_lossy() }));
                }
            });
            return;
        }
        "file.import" if !has_param(&params, "path") => {
            let n = name.to_string();
            crate::files::pick(st, crate::files::DialogKind::Import, "", move |st, r| {
                if let Some(p) = r.into_iter().next() {
                    st.run(&n, json!({ "path": p.to_string_lossy() }));
                }
            });
            return;
        }
        "file.updateHtml" if !has_param(&params, "path") => {
            crate::files::pick(st, crate::files::DialogKind::UpdateHtml, "", move |st, r| {
                if let Some(p) = r.into_iter().next() {
                    // Erfolg in der Statuszeile, Fehler (keine HTML-Seite …) als Meldung im Dialog
                    crate::dialogs::menu_dialogs::update_html(st, &p.to_string_lossy());
                }
            });
            return;
        }
        "commands.runScript" if !has_param(&params, "path") && !has_param(&params, "code") => {
            let n = name.to_string();
            crate::files::pick_one(st, &fp_i18n::t("ui-file-run-script-title"), &fp_i18n::t("ui-filter-scripts"), &["jsf", "js"], move |st, p| st.run(&n, json!({ "path": p.to_string_lossy() })));
            return;
        }
        "file.revert" if !has_param(&params, "confirmed") => {
            // Rückfrage wie im Original (OK = Standard)
            let name = st.session.active().map(|d| d.path.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| d.title.clone())).unwrap_or_default();
            st.dialogs.confirm = Some(crate::dialogs::Confirm {
                title: "Firepixel".into(),
                text: fp_i18n::tr!("ui-revert-confirm", name = name),
                command: "file.revert".into(),
                params: json!({ "confirmed": true }),
            });
            return;
        }
        // Optimieren-Bedienfeld ▸ Einstellungen löschen: Rückfrage 3169 wie im Original (re\17 §7, MN-5);
        // „Nein“/Schließen löscht nichts. Skripte und MCP rufen den Befehl ohne Rückfrage auf.
        "panel.optimize.deleteSettings" if has_param(&params, "name") && !has_param(&params, "confirmed") => {
            let mut p = params;
            p["confirmed"] = json!(true);
            st.dialogs.confirm = Some(crate::dialogs::Confirm { title: "Firepixel".into(), text: fp_i18n::t("dlg-pmenu-delete-settings-confirm"), command: name.into(), params: p });
            return;
        }
        "file.revert" => {
            let _ = params;
            if let Err(e) = crate::ui_commands::execute(&mut st.session, "file.revert", json!({})) {
                st.set_status(e.to_string(), now(ctx));
            }
            return;
        }
        "file.saveAll" => {
            // wie das Original: ungespeicherte Dokumente fragen nach einem Namen
            let active = st.session.active_id();
            let ids: Vec<_> = st.session.documents().iter().map(|d| d.id).collect();
            crate::files::save_all(st, ids, active);
            return;
        }
        "file.previewInBrowser.setPrimaryBrowser" | "file.previewInBrowser.setSecondaryBrowser" if !has_param(&params, "path") => {
            let title = fp_i18n::t(if name.ends_with("PrimaryBrowser") { "ui-file-primary-browser-title" } else { "ui-file-secondary-browser-title" });
            let n = name.to_string();
            crate::files::pick_one(st, &title, &fp_i18n::t("ui-filter-programs"), &["exe"], move |st, p| st.run(&n, json!({ "path": p.to_string_lossy() })));
            return;
        }
        "file.openRecent" => {
            let i = params.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            if let Some(p) = st.session.prefs.recent_files.get(i).cloned() {
                crate::files::open_path(st, &p);
            }
            return;
        }
        "file.save" if !has_param(&params, "path") => {
            crate::files::save_active(st, false, |_, _| {});
            return;
        }
        "file.saveAs" if !has_param(&params, "path") => {
            crate::files::save_active(st, true, |_, _| {});
            return;
        }
        "file.saveAsTemplate" if !has_param(&params, "path") => {
            let title = st.session.active().map(|d| d.title.clone()).unwrap_or_default();
            let n = name.to_string();
            crate::files::pick(st, crate::files::DialogKind::SaveAsTemplate, &crate::files::suggested_name(&title), move |st, r| {
                if let Some(p) = r.into_iter().next().map(crate::files::save_target) {
                    st.run(&n, json!({ "path": p.to_string_lossy() }));
                }
            });
            return;
        }
        "file.close" if !params.get("discard").is_some_and(|v| v.as_bool() == Some(true)) => {
            if let Some(id) = st.session.active_id() {
                close_documents(st, vec![id], PendingClose::CloseDocs);
            }
            return;
        }
        "file.closeAll" => {
            let ids = st.session.documents().iter().map(|d| d.id).collect();
            close_documents(st, ids, PendingClose::CloseDocs);
            return;
        }
        "file.exit" => {
            execute(st, ctx, Action::Quit);
            return;
        }
        "edit.preferences" if params.as_object().is_none_or(|m| m.is_empty()) => {
            st.dialogs.preferences = Some(PreferencesDialog::new(st));
            return;
        }
        "help.about" => {
            st.dialogs.about = true;
            return;
        }
        "window.toolbars.main" => {
            st.settings.workspace.show_toolbar = !st.settings.workspace.show_toolbar;
            st.sync_panels_to_engine();
            return;
        }
        "window.tools" => {
            st.settings.workspace.show_tools = !st.settings.workspace.show_tools;
            st.sync_panels_to_engine();
            return;
        }
        "window.properties" => {
            st.settings.workspace.show_inspector = !st.settings.workspace.show_inspector;
            st.sync_panels_to_engine();
            return;
        }
        "window.hidePanels" => {
            st.settings.workspace.panels_hidden = !st.settings.workspace.panels_hidden;
            st.session.ui.panels_hidden = st.settings.workspace.panels_hidden;
            return;
        }
        "ui.copyColor" => {
            // Kontextmenü der Pipette (auch über Aktionen erreichbar): Farbe als Hex in die Zwischenablage
            let c = match params.get("which").and_then(|v| v.as_str()) {
                Some("stroke") => st.session.tools.stroke_color,
                _ => st.session.tools.fill_color,
            };
            let hex = format!("#{:06X}", c.0 & 0x00ff_ffff);
            ctx.copy_text(hex.clone());
            st.set_status(fp_i18n::tr!("ui-color-copied", hex = hex), now(ctx));
            return;
        }
        "ui.mcpToggle" => {
            // KI-Steuerkanal (MCP über HTTP, nur 127.0.0.1) ein/aus
            if crate::bridge::running_url().is_some() {
                crate::bridge::disable();
                st.set_status(fp_i18n::t("ui-mcp-off"), now(ctx));
            } else {
                crate::bridge::enable(0);
                st.set_status(fp_i18n::t("ui-mcp-starting"), now(ctx));
            }
            return;
        }
        "ui.resetWarnings" => {
            st.session.prefs.reset_warnings();
            st.session.prefs_changed();
            st.set_status(fp_i18n::t("ui-warnings-reset"), now(ctx));
            return;
        }
        "ui.closeGroup" => {
            if let Some(p) = params.get("panel").and_then(|v| v.as_str()).and_then(panel_from_name) {
                st.settings.workspace.close_group_of(p);
                st.sync_panels_to_engine();
            }
            return;
        }
        "ui.saveWorkspace" => {
            st.dialogs.save_workspace = Some(String::new());
            return;
        }
        "ui.dock.toggleIconic" => {
            let ws = &mut st.settings.workspace;
            ws.dock_mode = if ws.dock_mode == DockMode::Expanded { DockMode::Iconic } else { DockMode::Expanded };
            st.dock.iconic_open = None;
            return;
        }
        "ui.dock.tabClicked" => {
            let idx = params.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let ws = &mut st.settings.workspace;
            let floating = params.get("floating").and_then(|v| v.as_u64());
            let dock_gi = params.get("group").and_then(|v| v.as_u64()).map(|g| g as usize);
            // nur wegen der Fensterhöhe eingeklappt angezeigt (CT-3): Klick klappt auf
            let forced = floating.is_none() && dock_gi.is_some_and(|g| st.dock.forced.contains(&g));
            let g = if let Some(f) = floating {
                ws.floating.get_mut(f as usize).map(|f| &mut f.group)
            } else if let Some(gi) = dock_gi {
                ws.dock.get_mut(gi)
            } else {
                None
            };
            if let Some(g) = g {
                if g.active == idx && !forced {
                    g.collapsed = !g.collapsed;
                } else {
                    g.active = idx;
                    g.collapsed = false;
                }
                if !g.collapsed && floating.is_none() {
                    st.dock.prefer = g.panels.get(idx).copied();
                }
            }
            return;
        }
        "ui.dock.dockFloating" => {
            let f = params.get("floating").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let at = params.get("at").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            st.settings.workspace.dock_floating(f, at);
            return;
        }
        "ui.dock.drop" => {
            if let Some(p) = params.get("panel").and_then(|v| v.as_str()).and_then(panel_from_name) {
                let num = |k: &str| params.get(k).and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let ws = &mut st.settings.workspace;
                match params.get("kind").and_then(|v| v.as_str()) {
                    Some("dockTabs") => ws.move_to_dock_group(p, num("group"), num("index")),
                    Some("dockGap") => ws.new_dock_group(p, num("at")),
                    Some("floatTabs") => ws.move_to_floating_group(p, num("group"), num("index")),
                    _ => {
                        let pos = params.get("pos").and_then(|v| v.as_array()).map(|a| [a.first().and_then(|x| x.as_f64()).unwrap_or(300.0) as f32, a.get(1).and_then(|x| x.as_f64()).unwrap_or(200.0) as f32]).unwrap_or([300.0, 200.0]);
                        ws.float_panel(p, pos);
                    }
                }
                st.sync_panels_to_engine();
            }
            return;
        }
        _ => {}
    }
    // Immer aktive Befehle ohne passendes Ziel: Warnung des Originals statt Dialog/Wirkung (FV-5)
    if crate::dialogs::warning::intercept(st, name) {
        return;
    }
    // Filter auf Vektorobjekte: Rückfrage „Dieser Vorgang konvertiert Vektoren in Bitmaps.“ (TW-21)
    if crate::dialogs::warning::intercept_filter_vectors(st, name, &params) {
        return;
    }
    // Befehle ohne Parameter aus Menü/Kürzel: zugehörigen Dialog öffnen (dialoge.json)
    let empty = params.as_object().is_none_or(|o| o.is_empty()) || params.is_null();
    if empty && crate::dialogs::open_for_command(st, name) {
        return;
    }
    // Einfügen aus der Systemzwischenablage wie im Original: fremdes Bild → Bitmap, fremder Text →
    // Textobjekt (12 pt, mittig im sichtbaren Teil), sonst der eigene Inhalt
    if name == "edit.paste" {
        let os_text = params.get("osText").and_then(|v| v.as_str()).map(str::to_string);
        if (empty || os_text.is_some()) && crate::os_clipboard::paste_foreign(st) {
            return;
        }
        if let Some(t) = os_text {
            let own = !st.session.clipboard.elements.is_empty() || st.session.clipboard.pixels.is_some();
            // eigene Kopien legen ein Bild (keinen Text) in die Systemzwischenablage: liegt dort Text,
            // ist er neuer als der eigene Inhalt (ohne Systemzwischenablage gilt der eigene)
            if !t.trim().is_empty() && (!own || st.os_clipboard) {
                if let Err(e) = crate::os_clipboard::paste_text(st, &t) {
                    st.set_status(e, now(ctx));
                }
                return;
            }
        }
        if let Err(e) = crate::ui_commands::execute(&mut st.session, "edit.paste", json!({})) {
            st.set_status(e.to_string(), now(ctx));
        }
        return;
    }
    // Befehlsskripte aus der Oberfläche: prompt() fragt im Dialog (CT-1b MN-5); ohne Antworten
    // meldet der Befehl die erste Frage, der Dialog startet das Skript dann erneut
    let script = matches!(name, "commands.runScript" | "commands.run");
    let mut params = params;
    if script && params.is_object() && params.get("prompts").is_none() {
        params["prompts"] = json!([]);
    }
    let script_params = script.then(|| params.clone());
    match crate::ui_commands::execute(&mut st.session, name, params) {
        Ok(out) => {
            if matches!(name, "edit.copy" | "edit.cut" | "edit.copyAsVectors") {
                crate::os_clipboard::after_copy(st);
            }
            if let Some(q) = out.value["result"].get("promptNeeded").filter(|q| q.is_object())
                && let Some(mut p) = script_params
            {
                let answers = out.value["result"]["prompts"].as_array().cloned().unwrap_or_default();
                if let Some(m) = p.as_object_mut() {
                    m.remove("prompts");
                }
                st.dialogs.script_prompt = Some(crate::dialogs::script_prompt::ScriptPrompt::new(name, p, answers, q["text"].as_str().unwrap_or_default(), q["default"].as_str()));
                return;
            }
            // alert() eines Befehlsskripts als Meldung zeigen (Titel = Programmname wie im Original, MN-4)
            if script {
                let alerts: Vec<String> = out.value["result"]["alerts"]
                    .as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_str()).filter(|s| !s.starts_with("confirm: ") && !s.starts_with("yesNoDialog: ") && !s.starts_with("prompt: ")).map(str::to_string).collect())
                    .unwrap_or_default();
                if !alerts.is_empty() {
                    st.dialogs.message = Some(("Firepixel".into(), alerts.join("\n\n")));
                }
            }
        }
        Err(EngineError::NotImplemented(n)) => {
            let label = crate::menu::command_label(st, &n);
            st.set_status(fp_i18n::tr!("ui-not-implemented", label = label), now(ctx));
        }
        Err(e) => st.set_status(e.to_string(), now(ctx)),
    }
}
