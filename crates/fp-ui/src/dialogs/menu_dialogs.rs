//! Dialoge zu Menübefehlen, die im Original einen Dialog öffnen und deren Befehle in fp-engine
//! Parameter brauchen (11 §3.2): Importieren (Dateidialog), HTML aktualisieren, Exportassistent
//! (4717), HTML-Einrichtung (4766), HTML-Code kopieren (6046, in die Zwischenablage), Selektives
//! JPEG (19147), Seite einrichten / Drucken (10397), Batch-Verarbeitung (2160, vereinfacht auf eine
//! Seite), Skript ausführen, Gespeicherte Befehle verwalten (6042), Erweiterungen verwalten,
//! Datei-Info, Browser festlegen, Symbol austauschen (6560), Neues Symbol/Neue Schaltfläche (19104).

use super::form::{Field, FormDialog};
use super::panel_menus::pick_open;
use super::{dialog_frame, push_button, t, DialogClose};
use crate::app_state::{Action, AppState};
use crate::inspector::fields;
use crate::theme::Theme;
use egui::{pos2, vec2, Align2, Rect};
use serde_json::{json, Value};

/// Dateifilter „Importieren“ (Textschlüssel, Endungen).
const IMPORT_FILES: &[(&str, &[&str])] = &[
    ("dlg-filter-all-readable", &["png", "psd", "svg", "ai", "eps", "gif", "jpg", "jpeg", "bmp", "tif", "tiff", "tga", "webp", "jxl", "qoi", "ico"]),
    ("dlg-filter-all-files", &["*"]),
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
            let names: Vec<String> = IMPORT_FILES.iter().map(|f| t(f.0)).collect();
            let filters: Vec<(&str, &[&str])> = IMPORT_FILES.iter().zip(&names).map(|(f, n)| (n.as_str(), f.1)).collect();
            let n = name.to_string();
            pick_open(st, &t("dlg-md-import"), &filters, move |st, p| st.run(&n, json!({"path": p.to_string_lossy()})));
            return true;
        }
        "file.updateHtml" => {
            let n = name.to_string();
            pick_open(st, &t("dlg-md-pick-html"), &[(t("dlg-filter-html").as_str(), &["htm", "html"])], move |st, p| st.run(&n, json!({"path": p.to_string_lossy()})));
            return true;
        }
        "commands.runScript" => {
            let n = name.to_string();
            pick_open(st, &t("dlg-md-run-script"), &[(t("dlg-filter-script").as_str(), &["jsf", "js"])], move |st, p| st.run(&n, json!({"path": p.to_string_lossy()})));
            return true;
        }
        "file.previewInBrowser.setPrimaryBrowser" | "file.previewInBrowser.setSecondaryBrowser" => {
            let n = name.to_string();
            pick_open(st, &t("dlg-md-set-browser"), &[(t("dlg-filter-programs").as_str(), &["exe"])], move |st, p| st.run(&n, json!({"path": p.to_string_lossy()})));
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
                    st.push(Action::Status(t("dlg-md-html-copied")));
                }
                Err(e) => st.dialogs.message = Some((t("dlg-md-copy-html"), e.to_string())),
            }
            return true;
        }
        "file.fileInfo" => {
            match crate::ui_commands::execute(&mut st.session, name, json!({})) {
                Ok(o) => {
                    let v = &o.value;
                    let mut text = fp_i18n::tr!(
                        "dlg-md-file-info-text",
                        title = v["title"].as_str().unwrap_or(""),
                        path = v["path"].as_str().map(str::to_string).unwrap_or_else(|| t("dlg-md-not-saved")),
                        pages = v["pages"].to_string(),
                        symbols = v["symbols"].to_string(),
                        version = v["formatVersion"].to_string()
                    );
                    for e in v["text"].as_array().into_iter().flatten() {
                        text.push_str(&format!("\n{}: {}", e["key"].as_str().unwrap_or(""), e["text"].as_str().unwrap_or("")));
                    }
                    st.dialogs.message = Some((t("dlg-md-file-info"), text));
                }
                Err(e) => st.dialogs.message = Some((t("dlg-md-file-info"), e.to_string())),
            }
            return true;
        }
        "commands.manageExtensions" | "help.manageExtensions" => {
            let v = crate::ui_commands::execute(&mut st.session, name, json!({"action": "list"})).map(|o| o.value).unwrap_or(Value::Null);
            let list: Vec<String> = v.get("extensions").and_then(|a| a.as_array()).map(|a| a.iter().map(|x| x.as_str().map(|s| s.to_string()).unwrap_or_else(|| x.get("name").and_then(|n| n.as_str()).unwrap_or("?").to_string())).collect()).unwrap_or_default();
            let text = if list.is_empty() { t("dlg-md-no-extensions") } else { list.join("\n") };
            st.dialogs.message = Some((t("dlg-md-manage-extensions"), text));
            return true;
        }
        "commands.manageSavedCommands" => {
            st.dialogs.saved_commands = Some(refresh_saved(st));
            return true;
        }
        "file.exportWizard" => Some(
            FormDialog::new(
                "4717",
                t("dlg-md-export-wizard").as_str(),
                name,
                vec![
                    Field::radio_of("destination", t("dlg-md-destination").as_str(), &[t("dlg-md-dest-web").as_str(), t("dlg-md-dest-image").as_str(), t("dlg-md-dest-html").as_str()], vec![json!("web"), json!("image"), json!("dreamweaver")], 0),
                    Field::check("_limit", t("dlg-md-limit-size").as_str(), false),
                    Field::number("_kb", t("dlg-md-target-size").as_str(), 32.0, 0.1, 100000.0).decimals(1).unit("K"),
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
                    t("dlg-md-html-setup").as_str(),
                    name,
                    vec![
                        Field::caption(t("dlg-md-cap-general").as_str()),
                        Field::text("file_extension", t("dlg-md-extension").as_str(), h.file_extension.as_deref().unwrap_or(".htm")),
                        Field::check("_xhtml", t("dlg-md-xhtml").as_str(), h.xhtml.unwrap_or(1) != 0),
                        Field::caption(t("dlg-md-cap-document").as_str()),
                        Field::text("images_dir", t("dlg-md-images-dir").as_str(), h.images_dir.as_deref().unwrap_or("")),
                        Field::check("slice_along_guides", t("dlg-md-slice-guides").as_str(), h.slice_along_guides.unwrap_or(false)),
                        Field::check("discard_unspecified_slices", t("dlg-md-discard-slices").as_str(), h.discard_unspecified_slices.unwrap_or(false)),
                        Field::check("generate_demo_html", t("dlg-md-demo-html").as_str(), h.generate_demo_html.unwrap_or(false)),
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
                    t("dlg-md-selective-jpeg").as_str(),
                    name,
                    vec![
                        Field::check("enabled", t("dlg-md-enable-selective").as_str(), e.jpeg_sel_enabled),
                        Field::number("quality", t("dlg-lbl-quality").as_str(), e.jpeg_sel_quality as f64, 1.0, 100.0),
                        Field::check("preserveText", t("dlg-md-preserve-text").as_str(), e.jpeg_sel_preserve_text),
                        Field::check("preserveButtons", t("dlg-md-preserve-buttons").as_str(), e.jpeg_sel_preserve_buttons),
                    ],
                )
                .with_width(400.0),
            )
        }
        "file.pageSetup" => Some(
            FormDialog::new(
                "10397",
                t("dlg-md-page-setup").as_str(),
                name,
                vec![
                    Field::choice_of("paper", t("dlg-md-paper").as_str(), &["A4", "A3", "Letter"], vec![json!("A4"), json!("A3"), json!("Letter")], 0),
                    Field::radio_of("orientation", t("dlg-md-orientation").as_str(), &[t("dlg-md-portrait").as_str(), t("dlg-md-landscape").as_str()], vec![json!("portrait"), json!("landscape")], 0),
                    Field::number("margins", t("dlg-md-margins").as_str(), 10.0, 0.0, 100.0).unit("mm"),
                    Field::number("scale", t("dlg-md-scale").as_str(), 100.0, 1.0, 1000.0).unit("%"),
                    Field::check("fit", t("dlg-md-fit-page").as_str(), false),
                    Field::number("dpi", t("dlg-md-print-dpi").as_str(), 300.0, 36.0, 1200.0).unit("dpi"),
                ],
            )
            .with_width(420.0)
            .with_transform(page_setup_params),
        ),
        "file.print" => Some(FormDialog::new("10397", t("dlg-md-print").as_str(), name, vec![Field::text("printer", t("dlg-md-printer").as_str(), ""), Field::check("_all", t("dlg-md-print-all").as_str(), false)]).with_width(420.0).with_transform(print_params)),
        "file.batchProcess" => {
            // Dateidialog ohne Blockieren: der Dialog erscheint, sobald Dateien gewählt sind
            let req = crate::file_dialog::Request::open_many(t("dlg-md-batch-files")).filter(t("dlg-filter-images"), &["png", "gif", "jpg", "jpeg", "psd", "bmp", "tif", "tiff", "webp"]);
            crate::file_dialog::request(st, req, |st, paths| {
                let files: Vec<String> = paths.iter().map(|p| p.to_string_lossy().to_string()).collect();
                if !files.is_empty() {
                    st.dialogs.form = Some(batch_form(files));
                }
            });
            return true;
        }
        "modify.symbol.swapSymbol" => {
            let names: Vec<String> = st.session.active().map(|d| d.doc.symbols.iter().map(|s| s.name.clone()).collect()).unwrap_or_default();
            if names.is_empty() {
                st.dialogs.message = Some((t("dlg-md-swap-symbol"), t("dlg-md-no-symbols")));
                return true;
            }
            let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
            Some(FormDialog::new("6560", &t("dlg-md-swap-symbol"), name, vec![Field::choice_of("symbol", t("dlg-md-symbol").as_str(), &refs, names.iter().map(|n| json!(n)).collect(), 0)]).with_width(380.0))
        }
        "edit.insert.newSymbol" | "edit.insert.newButton" => {
            let (title, def) = if name == "edit.insert.newButton" { (t("dlg-md-new-button"), t("dlg-md-button-default-name")) } else { (t("dlg-md-new-symbol"), t("dlg-symbol-default-name")) };
            Some(FormDialog::new("19104", &title, name, vec![Field::text("name", &t("dlg-lbl-name"), &def)]).with_width(380.0))
        }
        _ => return false,
    };
    if let Some(f) = form {
        st.dialogs.form = Some(f);
    }
    true
}

/// Stapelverarbeitung (2160) für die gewählten Dateien.
fn batch_form(files: Vec<String>) -> FormDialog {
    let name = "file.batchProcess";
    let out = std::path::Path::new(&files[0]).parent().map(|p| p.join(t("dlg-md-batch-folder")).to_string_lossy().to_string()).unwrap_or_default();
    FormDialog::new(
        "2160",
        t("dlg-md-batch").as_str(),
        name,
        vec![
            Field::note(&fp_i18n::tr!("dlg-md-files-selected", n = files.len())),
            Field::text("outDir", t("dlg-md-out-dir").as_str(), &out),
            Field::radio_of("output", t("dlg-md-output").as_str(), &[t("dlg-md-output-export").as_str(), t("dlg-md-output-fwpng").as_str()], vec![json!("export"), json!("fwpng")], 0),
            Field::number("_scale", t("dlg-preview-scale").as_str(), 100.0, 1.0, 1000.0).unit("%"),
            Field::text("prefix", t("dlg-md-prefix").as_str(), ""),
            Field::text("suffix", t("dlg-md-suffix").as_str(), ""),
            Field::check("backup", t("dlg-md-backup").as_str(), false),
        ],
    )
    .with_base(json!({"files": files}))
    .with_width(480.0)
    .with_transform(batch_params)
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
    let btn_l = super::tl(&["dlg-md-rename", "dlg-md-delete", "dlg-md-done"]);
    let ok_l = t("btn-ok");
    let bw = super::buttons_w(ctx, &super::refs(&btn_l), 86.0);
    let ow = super::button_w(ctx, &ok_l, 60.0);
    let r = dialog_frame(ctx, "fp-saved-commands", &t("dlg-md-saved-title"), vec2(420f32.max(12.0 + 200.0 + 12.0 + bw + 12.0).ceil(), 300.0), |ui, body| {
        let theme = Theme::current(ui.ctx());
        let list = Rect::from_min_max(body.min + vec2(12.0, 12.0), pos2(body.right() - 24.0 - bw, body.bottom() - 40.0));
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
            ui.painter().text(inner.left_top() + vec2(6.0, 6.0), Align2::LEFT_TOP, t("dlg-md-no-saved"), theme.small_font(), theme.colors.text_disabled);
        }
        let bx = body.right() - 12.0 - bw;
        let sel = d.selected.and_then(|i| d.names.get(i).cloned());
        if push_button(ui, Rect::from_min_size(pos2(bx, body.top() + 12.0), vec2(bw, 23.0)), &btn_l[0], false, sel.is_some()) {
            d.rename = sel.clone();
        }
        if push_button(ui, Rect::from_min_size(pos2(bx, body.top() + 40.0), vec2(bw, 23.0)), &btn_l[1], false, sel.is_some())
            && let Some(n) = &sel
        {
            action = Some(json!({"action": "delete", "name": n}));
        }
        if let (Some(old), Some(buf)) = (sel.clone(), d.rename.clone()) {
            let er = Rect::from_min_size(pos2(body.left() + 12.0, body.bottom() - 32.0), vec2(list.width() - 10.0 - ow, 22.0));
            if let Some(t) = fields::text(ui, "saved-rename", er, &buf, true) {
                d.rename = Some(t);
            }
            if push_button(ui, Rect::from_min_size(pos2(er.right() + 6.0, er.top()), vec2(ow, 22.0)), &ok_l, true, true) {
                let new = d.rename.clone().unwrap_or_default();
                if !new.trim().is_empty() && new != old {
                    action = Some(json!({"action": "rename", "name": old, "newName": new}));
                }
                d.rename = None;
            }
        }
        if push_button(ui, Rect::from_min_size(pos2(bx, body.bottom() - 34.0), vec2(bw, 23.0)), &btn_l[2], true, true) {
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
