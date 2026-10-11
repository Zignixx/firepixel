//! Dialoge zu Menübefehlen, die im Original einen Dialog öffnen und deren Befehle in fp-engine
//! Parameter brauchen (11 §3.2): Importieren (Dateidialog), HTML aktualisieren, Exportassistent
//! (4717), HTML-Einrichtung (4766), HTML-Code kopieren (6046, in die Zwischenablage), Selektives
//! JPEG (19147), Seite einrichten / Drucken (10397), Batch-Verarbeitung (2160, vereinfacht auf eine
//! Seite), Skript ausführen, Gespeicherte Befehle verwalten (6042), Erweiterungen verwalten,
//! Datei-Info, Browser festlegen, Symbol austauschen (6560), Neues Symbol/Neue Schaltfläche (19104).

use super::form::{Field, FormDialog};
use super::panel_menus::pick_open;
use super::{dialog_frame, push_button, DialogClose};
use crate::app_state::{Action, AppState};
use crate::inspector::fields;
use crate::theme::Theme;
use egui::{pos2, vec2, Align2, Rect};
use serde_json::{json, Value};

const IMPORT_FILES: &[(&str, &[&str])] = &[
    ("Alle lesbaren Formate", &["png", "psd", "svg", "ai", "eps", "gif", "jpg", "jpeg", "bmp", "tif", "tiff", "tga", "webp", "jxl", "qoi", "ico"]),
    ("Alle Dateien (*.*)", &["*"]),
];

/// Gespeicherte Befehle verwalten (6042).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SavedCommandsDialog {
    pub names: Vec<String>,
    pub selected: Option<usize>,
    pub rename: Option<String>,
    pub folder: String,
}

fn refresh_saved(st: &mut AppState) -> SavedCommandsDialog {
    let v = crate::ui_commands::execute(&mut st.session, "commands.manageSavedCommands", json!({"action": "list"})).map(|o| o.value).unwrap_or(Value::Null);
    let names = v.get("commands").and_then(|c| c.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string()).or_else(|| x.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))).collect()).unwrap_or_default();
    SavedCommandsDialog { names, selected: None, rename: None, folder: v.get("folder").and_then(|f| f.as_str()).unwrap_or("").to_string() }
}

fn wizard_params(d: &FormDialog) -> Value {
    let mut v = json!({"destination": d.get("destination"), "apply": true});
    if d.get("_limit").as_bool() == Some(true) {
        v["targetSize"] = json!((d.get("_kb").as_f64().unwrap_or(32.0) * 1024.0).round() as i64);
    }
    v
}

fn html_setup_params(d: &FormDialog) -> Value {
    json!({"settings": {
        "file_extension": d.get("file_extension"),
        "images_dir": d.get("images_dir"),
        "xhtml": if d.get("_xhtml").as_bool() == Some(true) { 1 } else { 0 },
        "slice_along_guides": d.get("slice_along_guides"),
        "discard_unspecified_slices": d.get("discard_unspecified_slices"),
        "generate_demo_html": d.get("generate_demo_html"),
    }})
}

fn page_setup_params(d: &FormDialog) -> Value {
    json!({"orientation": d.get("orientation"), "scale": d.get("scale"), "fit": d.get("fit"), "paper": d.get("paper"), "margins": d.get("margins"), "dpi": d.get("dpi")})
}

fn print_params(d: &FormDialog) -> Value {
    let mut v = json!({"open": true});
    if d.get("_all").as_bool() == Some(true) {
        v["pages"] = json!("all");
    }
    if let Some(p) = d.get("printer").as_str().filter(|p| !p.trim().is_empty()) {
        v["printer"] = json!(p);
    }
    v
}

fn batch_params(d: &FormDialog) -> Value {
    let mut v = json!({
        "files": d.base.get("files").cloned().unwrap_or(json!([])),
        "outDir": d.get("outDir"),
        "output": d.get("output"),
        "backup": d.get("backup"),
    });
    for k in ["prefix", "suffix"] {
        if let Some(s) = d.get(k).as_str().filter(|s| !s.is_empty()) {
            v[k] = json!(s);
        }
    }
    let scale = d.get("_scale").as_f64().unwrap_or(100.0);
    if (scale - 100.0).abs() > 0.01 {
        v["commands"] = json!([{"name": "modify.canvas.imageSize", "params": {"percent": scale}}]);
    }
    v
}

/// Öffnet den Dialog zu einem Menübefehl ohne Parameter; `true`, wenn übernommen.
pub fn open(st: &mut AppState, name: &str) -> bool {
    let form = match name {
        "file.import" => {
            let p = pick_open(st, "Importieren", IMPORT_FILES);
            if let Some(p) = p {
                st.run(name, json!({"path": p.to_string_lossy()}));
            }
            return true;
        }
        "file.updateHtml" => {
            if let Some(p) = pick_open(st, "HTML-Datei auswählen", &[("HTML (*.htm, *.html)", &["htm", "html"])]) {
                st.run(name, json!({"path": p.to_string_lossy()}));
            }
            return true;
        }
        "commands.runScript" => {
            if let Some(p) = pick_open(st, "Skript ausführen", &[("Befehl (*.jsf, *.js)", &["jsf", "js"])]) {
                st.run(name, json!({"path": p.to_string_lossy()}));
            }
            return true;
        }
        "file.previewInBrowser.setPrimaryBrowser" | "file.previewInBrowser.setSecondaryBrowser" => {
            if let Some(p) = pick_open(st, "Browser festlegen", &[("Programme (*.exe)", &["exe"])]) {
                st.run(name, json!({"path": p.to_string_lossy()}));
            }
            return true;
        }
        "file.browseInBridge" => {
            st.run(name, json!({"open": true}));
            return true;
        }
        "edit.copyHtmlCode" => {
            match crate::ui_commands::execute(&mut st.session, name, json!({"style": "htmlAndImages"})) {
                Ok(o) => {
                    st.dialogs.clipboard = o.value.get("html").and_then(|h| h.as_str()).map(|s| s.to_string());
                    st.push(Action::Status("HTML-Code in die Zwischenablage kopiert".into()));
                }
                Err(e) => st.dialogs.message = Some(("HTML-Code kopieren".into(), e.to_string())),
            }
            return true;
        }
        "file.fileInfo" => {
            match crate::ui_commands::execute(&mut st.session, name, json!({})) {
                Ok(o) => {
                    let v = &o.value;
                    let mut t = format!(
                        "Titel: {}\nPfad: {}\nSeiten: {}\nSymbole: {}\nFormatversion: {}",
                        v["title"].as_str().unwrap_or(""),
                        v["path"].as_str().unwrap_or("(nicht gespeichert)"),
                        v["pages"],
                        v["symbols"],
                        v["formatVersion"]
                    );
                    for e in v["text"].as_array().into_iter().flatten() {
                        t.push_str(&format!("\n{}: {}", e["key"].as_str().unwrap_or(""), e["text"].as_str().unwrap_or("")));
                    }
                    st.dialogs.message = Some(("Datei-Info".into(), t));
                }
                Err(e) => st.dialogs.message = Some(("Datei-Info".into(), e.to_string())),
            }
            return true;
        }
        "commands.manageExtensions" | "help.manageExtensions" => {
            let v = crate::ui_commands::execute(&mut st.session, name, json!({"action": "list"})).map(|o| o.value).unwrap_or(Value::Null);
            let list: Vec<String> = v.get("extensions").and_then(|a| a.as_array()).map(|a| a.iter().map(|x| x.as_str().map(|s| s.to_string()).unwrap_or_else(|| x.get("name").and_then(|n| n.as_str()).unwrap_or("?").to_string())).collect()).unwrap_or_default();
            let text = if list.is_empty() { "Es sind keine Erweiterungen installiert.".to_string() } else { list.join("\n") };
            st.dialogs.message = Some(("Erweiterungen verwalten".into(), text));
            return true;
        }
        "commands.manageSavedCommands" => {
            st.dialogs.saved_commands = Some(refresh_saved(st));
            return true;
        }
        "file.exportWizard" => Some(
            FormDialog::new(
                "4717",
                "Exportassistent",
                name,
                vec![
                    Field::radio_of("destination", "Ziel:", &["Web", "Bildbearbeitung", "Webseite (HTML)"], vec![json!("web"), json!("image"), json!("dreamweaver")], 0),
                    Field::check("_limit", "Exportgröße festlegen", false),
                    Field::number("_kb", "Zielgröße:", 32.0, 0.1, 100000.0).decimals(1).unit("K"),
                ],
            )
            .with_width(480.0)
            .with_transform(wizard_params),
        ),
        "file.htmlSetup" => {
            let h = st.session.active().and_then(|d| d.doc.pages.get(d.page)).and_then(|p| p.root.export.as_ref()).and_then(|e| e.html.as_deref().cloned()).unwrap_or_else(fp_model::HtmlSettings::cs6_default);
            Some(
                FormDialog::new(
                    "4766",
                    "HTML-Einrichtung",
                    name,
                    vec![
                        Field::caption("Allgemein"),
                        Field::text("file_extension", "Erweiterung:", h.file_extension.as_deref().unwrap_or(".htm")),
                        Field::check("_xhtml", "XHTML-kompatibel", h.xhtml.unwrap_or(1) != 0),
                        Field::caption("Dokumentspezifisch"),
                        Field::text("images_dir", "Bildordner:", h.images_dir.as_deref().unwrap_or("")),
                        Field::check("slice_along_guides", "Segmente an Hilfslinien", h.slice_along_guides.unwrap_or(false)),
                        Field::check("discard_unspecified_slices", "Nicht angegebene Segmente verwerfen", h.discard_unspecified_slices.unwrap_or(false)),
                        Field::check("generate_demo_html", "Demo-HTML erzeugen", h.generate_demo_html.unwrap_or(false)),
                    ],
                )
                .with_width(440.0)
                .with_transform(html_setup_params),
            )
        }
        "modify.selectiveJpeg.settings" => {
            let e = st.session.active().and_then(|d| d.doc.pages.get(d.page)).and_then(|p| p.root.export.as_deref().cloned()).unwrap_or_default();
            Some(
                FormDialog::new(
                    "19147",
                    "Selektive JPEG-Einstellungen",
                    name,
                    vec![
                        Field::check("enabled", "Selektive Qualität aktivieren", e.jpeg_sel_enabled),
                        Field::number("quality", "Qualität:", e.jpeg_sel_quality as f64, 1.0, 100.0),
                        Field::check("preserveText", "Textqualität beibehalten", e.jpeg_sel_preserve_text),
                        Field::check("preserveButtons", "Schaltflächenqualität beibehalten", e.jpeg_sel_preserve_buttons),
                    ],
                )
                .with_width(400.0),
            )
        }
        "file.pageSetup" => Some(
            FormDialog::new(
                "10397",
                "Seite einrichten",
                name,
                vec![
                    Field::choice_of("paper", "Papier:", &["A4", "A3", "Letter"], vec![json!("A4"), json!("A3"), json!("Letter")], 0),
                    Field::radio_of("orientation", "Ausrichtung:", &["Hochformat", "Querformat"], vec![json!("portrait"), json!("landscape")], 0),
                    Field::number("margins", "Ränder:", 10.0, 0.0, 100.0).unit("mm"),
                    Field::number("scale", "Skalierung:", 100.0, 1.0, 1000.0).unit("%"),
                    Field::check("fit", "Auf Seite einpassen", false),
                    Field::number("dpi", "Druckauflösung:", 300.0, 36.0, 1200.0).unit("dpi"),
                ],
            )
            .with_width(420.0)
            .with_transform(page_setup_params),
        ),
        "file.print" => Some(FormDialog::new("10397", "Drucken", name, vec![Field::text("printer", "Drucker (leer = Standard):", ""), Field::check("_all", "Alle Seiten drucken", false)]).with_width(420.0).with_transform(print_params)),
        "file.batchProcess" => {
            let files: Vec<String> = match &st.file_override {
                Some(o) => o.open.iter().map(|p| p.to_string_lossy().to_string()).collect(),
                None => rfd::FileDialog::new().set_title("Zu verarbeitende Dateien").add_filter("Bilder", &["png", "gif", "jpg", "jpeg", "psd", "bmp", "tif", "tiff", "webp"]).pick_files().unwrap_or_default().iter().map(|p| p.to_string_lossy().to_string()).collect(),
            };
            if files.is_empty() {
                return true;
            }
            let out = std::path::Path::new(&files[0]).parent().map(|p| p.join("Batch").to_string_lossy().to_string()).unwrap_or_default();
            Some(
                FormDialog::new(
                    "2160",
                    "Batch-Verarbeitung",
                    name,
                    vec![
                        Field::note(&format!("{} Datei(en) ausgewählt.", files.len())),
                        Field::text("outDir", "Zielordner:", &out),
                        Field::radio_of("output", "Ergebnis:", &["Exportieren", "Als FW-PNG speichern"], vec![json!("export"), json!("fwpng")], 0),
                        Field::number("_scale", "Skalieren:", 100.0, 1.0, 1000.0).unit("%"),
                        Field::text("prefix", "Präfix:", ""),
                        Field::text("suffix", "Suffix:", ""),
                        Field::check("backup", "Originale sichern", false),
                    ],
                )
                .with_base(json!({"files": files}))
                .with_width(480.0)
                .with_transform(batch_params),
            )
        }
        "modify.symbol.swapSymbol" => {
            let names: Vec<String> = st.session.active().map(|d| d.doc.symbols.iter().map(|s| s.name.clone()).collect()).unwrap_or_default();
            if names.is_empty() {
                st.dialogs.message = Some(("Symbol austauschen".into(), "Das Dokument enthält keine Symbole.".into()));
                return true;
            }
            let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
            Some(FormDialog::new("6560", "Symbol austauschen", name, vec![Field::choice_of("symbol", "Symbol:", &refs, names.iter().map(|n| json!(n)).collect(), 0)]).with_width(380.0))
        }
        "edit.insert.newSymbol" | "edit.insert.newButton" => {
            let (title, def) = if name == "edit.insert.newButton" { ("Neue Schaltfläche", "Schaltfläche") } else { ("Neues Symbol", "Symbol") };
            Some(FormDialog::new("19104", title, name, vec![Field::text("name", "Name:", def)]).with_width(380.0))
        }
        _ => return false,
    };
    if let Some(f) = form {
        st.dialogs.form = Some(f);
    }
    true
}

/// Befehle mit eigenem Dialog in diesem Modul.
pub fn handles(name: &str) -> bool {
    matches!(
        name,
        "file.import"
            | "file.updateHtml"
            | "commands.runScript"
            | "file.previewInBrowser.setPrimaryBrowser"
            | "file.previewInBrowser.setSecondaryBrowser"
            | "file.browseInBridge"
            | "edit.copyHtmlCode"
            | "file.fileInfo"
            | "commands.manageExtensions"
            | "help.manageExtensions"
            | "commands.manageSavedCommands"
            | "file.exportWizard"
            | "file.htmlSetup"
            | "modify.selectiveJpeg.settings"
            | "file.pageSetup"
            | "file.print"
            | "file.batchProcess"
            | "modify.symbol.swapSymbol"
            | "edit.insert.newSymbol"
            | "edit.insert.newButton"
    )
}

/// „Gespeicherte Befehle verwalten“ (6042): Liste, Umbenennen, Löschen.
pub fn show_saved_commands(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.dialogs.saved_commands.clone() else { return };
    let mut close = false;
    let mut action: Option<Value> = None;
    let r = dialog_frame(ctx, "fp-saved-commands", "Gespeicherte Befehle verwalten", vec2(420.0, 300.0), |ui, body| {
        let theme = Theme::current(ui.ctx());
        let list = Rect::from_min_max(body.min + vec2(12.0, 12.0), pos2(body.right() - 110.0, body.bottom() - 40.0));
        let inner = fields::list_box(ui, list);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
        child.spacing_mut().item_spacing = vec2(0.0, 0.0);
        for (i, n) in d.names.iter().enumerate() {
            if fields::list_item(&mut child, n, d.selected == Some(i)).clicked() {
                d.selected = Some(i);
                d.rename = None;
            }
        }
        if d.names.is_empty() {
            ui.painter().text(inner.left_top() + vec2(6.0, 6.0), Align2::LEFT_TOP, "Keine gespeicherten Befehle", theme.small_font(), theme.colors.text_disabled);
        }
        let bx = body.right() - 98.0;
        let sel = d.selected.and_then(|i| d.names.get(i).cloned());
        if push_button(ui, Rect::from_min_size(pos2(bx, body.top() + 12.0), vec2(86.0, 23.0)), "Umbenennen", false, sel.is_some()) {
            d.rename = sel.clone();
        }
        if push_button(ui, Rect::from_min_size(pos2(bx, body.top() + 40.0), vec2(86.0, 23.0)), "Löschen", false, sel.is_some())
            && let Some(n) = &sel
        {
            action = Some(json!({"action": "delete", "name": n}));
        }
        if let (Some(old), Some(buf)) = (sel.clone(), d.rename.clone()) {
            let er = Rect::from_min_size(pos2(body.left() + 12.0, body.bottom() - 32.0), vec2(list.width() - 70.0, 22.0));
            if let Some(t) = fields::text(ui, "saved-rename", er, &buf, true) {
                d.rename = Some(t);
            }
            if push_button(ui, Rect::from_min_size(pos2(er.right() + 6.0, er.top()), vec2(60.0, 22.0)), "OK", true, true) {
                let new = d.rename.clone().unwrap_or_default();
                if !new.trim().is_empty() && new != old {
                    action = Some(json!({"action": "rename", "name": old, "newName": new}));
                }
                d.rename = None;
            }
        }
        if push_button(ui, Rect::from_min_size(pos2(bx, body.bottom() - 34.0), vec2(86.0, 23.0)), "Fertig", true, true) {
            close = true;
        }
    });
    if let Some(a) = action {
        if let Err(e) = crate::ui_commands::execute(&mut st.session, "commands.manageSavedCommands", a) {
            st.push(Action::Status(e.to_string()));
        }
        let keep = d.selected;
        d = refresh_saved(st);
        d.selected = keep.filter(|i| *i < d.names.len());
    }
    st.dialogs.saved_commands = if close || r == DialogClose::Cancelled { None } else { Some(d) };
}
