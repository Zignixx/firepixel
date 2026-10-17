//! Dialoge zu Menübefehlen, die im Original einen Dialog öffnen und deren Befehle in fp-engine
//! Parameter brauchen (11 §3.2): Importieren (Dateidialog), HTML aktualisieren, Exportassistent
//! (4717), HTML-Einrichtung (4766), HTML-Code kopieren (6046, in die Zwischenablage), Selektives
//! JPEG (19147), Seite einrichten / Drucken (10397), Batch-Verarbeitung (2160, vereinfacht auf eine
//! Seite), Skript ausführen, Gespeicherte Befehle verwalten (6042), Erweiterungen verwalten,
//! Datei-Info, Browser festlegen, Symbol austauschen (6560), Neues Symbol/Neue Schaltfläche (19104).

use super::form::{Field, FormDialog};
use super::panel_menus::pick_open;
use super::{DialogClose, dialog_frame, push_button, t};
use crate::app_state::{Action, AppState};
use crate::inspector::fields;
use crate::theme::Theme;
use crate::ui_kit;
use egui::{Rect, pos2, vec2};
use serde_json::{Value, json};

/// Dateifilter „Importieren“ (Textschlüssel, Endungen).
const IMPORT_FILES: &[(&str, &[&str])] = &[
    (
        "dlg-filter-all-readable",
        &[
            "png", "psd", "svg", "ai", "eps", "gif", "jpg", "jpeg", "bmp", "tif", "tiff", "tga",
            "webp", "jxl", "qoi", "ico",
        ],
    ),
    ("dlg-filter-all-files", &["*"]),
];

/// Zeilenhöhe der Tabelle „Gespeicherte Befehle verwalten“.
const SAVED_ROW_H: f32 = 20.0;

/// Gespeicherte Befehle verwalten (6042).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SavedCommandsDialog {
    pub names: Vec<String>,
    pub selected: Option<usize>,
    pub rename: Option<String>,
    pub folder: String,
}

fn refresh_saved(st: &mut AppState) -> SavedCommandsDialog {
    let v = crate::ui_commands::execute(
        &mut st.session,
        "commands.manageSavedCommands",
        json!({"action": "list"}),
    )
    .map(|o| o.value)
    .unwrap_or(Value::Null);
    let names = v
        .get("commands")
        .and_then(|c| c.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| {
                    x.as_str().map(|s| s.to_string()).or_else(|| {
                        x.get("name")
                            .and_then(|n| n.as_str())
                            .map(|s| s.to_string())
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    SavedCommandsDialog {
        names,
        selected: None,
        rename: None,
        folder: v
            .get("folder")
            .and_then(|f| f.as_str())
            .unwrap_or("")
            .to_string(),
    }
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

/// „HTML-Code kopieren“: HTML in die Zwischenablage, Bilder in den Ordner `dir` (ohne: nur der
/// Code). Erfolg in der Statuszeile, Fehler als Meldung.
pub fn copy_html(st: &mut AppState, dir: Option<&str>) {
    let mut params = json!({"style": "htmlAndImages"});
    if let Some(d) = dir {
        params["dir"] = json!(d);
    }
    copy_html_with(st, params);
}

/// `edit.copyHtmlCode` mit fertigen Parametern (Assistent: `dir`, `baseName`, `xhtml`).
pub fn copy_html_with(st: &mut AppState, params: Value) {
    let dir = params
        .get("dir")
        .and_then(|d| d.as_str())
        .map(|s| s.to_string());
    let dir = dir.as_deref();
    match crate::ui_commands::execute(&mut st.session, "edit.copyHtmlCode", params) {
        Ok(o) => {
            st.dialogs.clipboard = o
                .value
                .get("html")
                .and_then(|h| h.as_str())
                .map(|s| s.to_string());
            let n = o
                .value
                .get("written")
                .and_then(|w| w.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            let msg = match dir {
                Some(d) if n > 0 => fp_i18n::tr!("dlg-md-html-copied-images", n = n, path = d),
                _ => t("dlg-md-html-copied"),
            };
            st.push(Action::Status(msg));
        }
        Err(e) => st.dialogs.message = Some((t("dlg-md-copy-html"), e.to_string())),
    }
}

/// „HTML aktualisieren“: tauscht in der gewählten Seite die Firepixel-Tabelle aus und schreibt
/// die Bilder neu (ohne Tabelle: am Ende eingefügt). Erfolg in der Statuszeile, Fehler (keine
/// HTML-Seite, Datei fehlt) als Meldung.
pub fn update_html(st: &mut AppState, path: &str) {
    match crate::ui_commands::execute(&mut st.session, "file.updateHtml", json!({ "path": path })) {
        Ok(o) => {
            let n = o
                .value
                .get("files")
                .and_then(|w| w.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            let key = if o.value.get("inserted").and_then(|v| v.as_bool()) == Some(true) {
                "dlg-md-html-inserted"
            } else {
                "dlg-md-html-updated"
            };
            st.push(Action::Status(fp_i18n::tr!(key, n = n, path = path)));
        }
        Err(e) => st.dialogs.message = Some((t("dlg-md-update-html"), e.to_string())),
    }
}

/// Öffnet den Dialog zu einem Menübefehl ohne Parameter; `true`, wenn übernommen.
pub fn open(st: &mut AppState, name: &str) -> bool {
    let form = match name {
        "file.import" => {
            let names: Vec<String> = IMPORT_FILES.iter().map(|f| t(f.0)).collect();
            let filters: Vec<(&str, &[&str])> = IMPORT_FILES
                .iter()
                .zip(&names)
                .map(|(f, n)| (n.as_str(), f.1))
                .collect();
            pick_open(st, &t("dlg-md-import"), &filters, move |st, p| {
                crate::canvas::begin_place_import(st, p)
            });
            return true;
        }
        "file.updateHtml" => {
            pick_open(
                st,
                &t("dlg-md-pick-html"),
                &[(t("dlg-filter-html").as_str(), &["htm", "html"])],
                move |st, p| update_html(st, &p.to_string_lossy()),
            );
            return true;
        }
        "commands.runScript" => {
            let n = name.to_string();
            pick_open(
                st,
                &t("dlg-md-run-script"),
                &[(t("dlg-filter-script").as_str(), &["jsf", "js"])],
                move |st, p| st.run(&n, json!({"path": p.to_string_lossy()})),
            );
            return true;
        }
        "file.previewInBrowser.setPrimaryBrowser" | "file.previewInBrowser.setSecondaryBrowser" => {
            let n = name.to_string();
            pick_open(
                st,
                &t("dlg-md-set-browser"),
                &[(t("dlg-filter-programs").as_str(), &["exe"])],
                move |st, p| st.run(&n, json!({"path": p.to_string_lossy()})),
            );
            return true;
        }
        "file.browseInBridge" => {
            st.run(name, json!({"open": true}));
            return true;
        }
        "edit.copyHtmlCode" => {
            // Assistent wie im Original (TW-22): Stil, Basisname, Bilderordner, dann kopieren
            return super::copy_html::open(st);
        }
        "commands.manageExtensions" | "help.manageExtensions" => {
            let v = crate::ui_commands::execute(&mut st.session, name, json!({"action": "list"}))
                .map(|o| o.value)
                .unwrap_or(Value::Null);
            let list: Vec<String> = v
                .get("extensions")
                .and_then(|a| a.as_array())
                .map(|a| {
                    a.iter()
                        .map(|x| {
                            x.as_str().map(|s| s.to_string()).unwrap_or_else(|| {
                                x.get("name")
                                    .and_then(|n| n.as_str())
                                    .unwrap_or("?")
                                    .to_string()
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            let text = if list.is_empty() {
                t("dlg-md-no-extensions")
            } else {
                list.join("\n")
            };
            st.dialogs.message = Some((t("dlg-md-manage-extensions"), text));
            return true;
        }
        "commands.manageSavedCommands" => {
            let d = refresh_saved(st);
            if d.names.is_empty() {
                // Original: Info-Meldung mit dem Programmnamen als Titel statt der leeren Liste
                st.dialogs.message = Some((t("dlg-md-app-title"), t("dlg-md-no-saved-info")));
            } else {
                st.dialogs.saved_commands = Some(d);
            }
            return true;
        }
        "modify.selectiveJpeg.settings" => {
            let e = st
                .session
                .active()
                .and_then(|d| d.doc.pages.get(d.page))
                .and_then(|p| p.root.export.as_deref().cloned())
                .unwrap_or_default();
            Some(
                FormDialog::new(
                    "19147",
                    t("dlg-md-selective-jpeg").as_str(),
                    name,
                    vec![
                        Field::check(
                            "enabled",
                            t("dlg-md-enable-selective-ak").as_str(),
                            e.jpeg_sel_enabled,
                        ),
                        Field::number(
                            "quality",
                            t("dlg-lbl-quality").as_str(),
                            e.jpeg_sel_quality as f64,
                            1.0,
                            100.0,
                        ),
                        super::catalog::selective_jpeg_overlay_field(st),
                        Field::check(
                            "preserveText",
                            t("dlg-md-preserve-text-ak").as_str(),
                            e.jpeg_sel_preserve_text,
                        ),
                        Field::check(
                            "preserveButtons",
                            t("dlg-md-preserve-buttons-ak").as_str(),
                            e.jpeg_sel_preserve_buttons,
                        ),
                    ],
                )
                // Anordnung der Vorlage, ohne Haken gesperrt; Überlagerungsfarbe der Maske gemerkt
                .with_layout(super::catalog::SELECTIVE_JPEG_LAYOUT)
                .with_store_hook(super::catalog::store_selective_jpeg_overlay)
                .with_width(400.0)
                .with_focus_ok(),
            )
        }
        "file.pageSetup" => Some(
            FormDialog::new(
                "10397",
                t("dlg-md-page-setup").as_str(),
                name,
                vec![
                    Field::choice_of(
                        "paper",
                        t("dlg-md-paper").as_str(),
                        &["A4", "A3", "Letter"],
                        vec![json!("A4"), json!("A3"), json!("Letter")],
                        0,
                    ),
                    Field::radio_of(
                        "orientation",
                        t("dlg-md-orientation").as_str(),
                        &[
                            t("dlg-md-portrait").as_str(),
                            t("dlg-md-landscape").as_str(),
                        ],
                        vec![json!("portrait"), json!("landscape")],
                        0,
                    ),
                    Field::number("margins", t("dlg-md-margins").as_str(), 10.0, 0.0, 100.0)
                        .unit("mm"),
                    Field::number("scale", t("dlg-md-scale").as_str(), 100.0, 1.0, 1000.0)
                        .unit("%"),
                    Field::check("fit", t("dlg-md-fit-page").as_str(), false),
                    Field::number("dpi", t("dlg-md-print-dpi").as_str(), 300.0, 36.0, 1200.0)
                        .unit("dpi"),
                ],
            )
            .with_width(420.0)
            .with_transform(page_setup_params),
        ),
        "file.print" => Some(
            FormDialog::new(
                "10397",
                t("dlg-md-print").as_str(),
                name,
                vec![
                    Field::text("printer", t("dlg-md-printer").as_str(), ""),
                    Field::check("_all", t("dlg-md-print-all").as_str(), false),
                ],
            )
            .with_width(420.0)
            .with_transform(print_params),
        ),
        "file.batchProcess" => {
            // BATCH-2: Datei ▸ Batch-Verarbeitung… öffnet das Batch-Studio (Reiter „Batch“) statt
            // des alten Assistenten; mit Parametern (`files`/`recipe`) läuft der Befehl ohne Oberfläche
            crate::batch::open(st);
            return true;
        }
        "modify.symbol.swapSymbol" => {
            let names: Vec<String> = st
                .session
                .active()
                .map(|d| d.doc.symbols.iter().map(|s| s.name.clone()).collect())
                .unwrap_or_default();
            if names.is_empty() {
                st.dialogs.message = Some((t("dlg-md-swap-symbol"), t("dlg-md-no-symbols")));
                return true;
            }
            let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
            Some(
                FormDialog::new(
                    "6560",
                    &t("dlg-md-swap-symbol"),
                    name,
                    vec![Field::choice_of(
                        "symbol",
                        t("dlg-md-symbol").as_str(),
                        &refs,
                        names.iter().map(|n| json!(n)).collect(),
                        0,
                    )],
                )
                .with_width(380.0),
            )
        }
        // Neues Symbol: im Original derselbe Dialog wie „In Symbol konvertieren“ (Name, Typ,
        // 9-Slice); OK legt ein leeres Symbol an und öffnet seine Symbolbearbeitung (PM-5 D14)
        "edit.insert.newSymbol" => Some(
            FormDialog::new(
                "19104",
                t("dlg-title-convert-to-symbol").as_str(),
                name,
                vec![
                    Field::text(
                        "name",
                        t("dlg-lbl-name-ak").as_str(),
                        t("dlg-symbol-default-name").as_str(),
                    ),
                    Field::radio_of(
                        "kind",
                        t("dlg-lbl-type").as_str(),
                        &[
                            t("dlg-symbol-graphic-ak").as_str(),
                            t("dlg-symbol-animation-ak").as_str(),
                            t("dlg-symbol-button-ak").as_str(),
                        ],
                        vec![json!("graphic"), json!("animation"), json!("button")],
                        0,
                    ),
                    Field::check("nineSlice", t("dlg-lbl-enable-nineslice").as_str(), false),
                    // im Original vorhanden, aber grau (PM-5 D14)
                    Field::check("_common", t("dlg-lbl-save-common-library").as_str(), false)
                        .disabled(),
                ],
            )
            .with_base(json!({"edit": true}))
            .with_width(420.0)
            // Anordnung und Tab-Folge der Vorlage 19104 wie „In Symbol konvertieren“
            .with_layout(super::symbol_anim::CONVERT_LAYOUT),
        ),
        // Neue Schaltfläche: wie „Neues Symbol“ eine leere Schaltfläche anlegen und ihre
        // Symbolbearbeitung öffnen (Original: „im Schaltflächeneditor“)
        "edit.insert.newButton" => Some(
            FormDialog::new(
                "19104",
                &t("dlg-md-new-button"),
                name,
                vec![Field::text(
                    "name",
                    &t("dlg-lbl-name"),
                    &t("dlg-md-button-default-name"),
                )],
            )
            .with_base(json!({"edit": true}))
            .with_width(380.0),
        ),
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
            | "commands.manageExtensions"
            | "help.manageExtensions"
            | "commands.manageSavedCommands"
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
    let Some(mut d) = st.dialogs.saved_commands.clone() else {
        return;
    };
    let mut close = false;
    let mut action: Option<Value> = None;
    let btn_l = super::tl(&["dlg-md-rename", "dlg-md-delete", "dlg-md-done"]);
    let ok_l = t("btn-ok");
    let bw = super::buttons_w(ctx, &super::refs(&btn_l), 86.0);
    let ow = super::button_w(ctx, &ok_l, 60.0);
    let r = dialog_frame(
        ctx,
        "fp-saved-commands",
        &t("dlg-md-saved-title"),
        vec2(420f32.max(12.0 + 200.0 + 12.0 + bw + 12.0).ceil(), 300.0),
        |ui, body| {
            let theme = Theme::current(ui.ctx());
            // Tabelle mit Kopf, Zebra, Hover und Auswahl (DESIGN §8, §13 Prio 6)
            let list = Rect::from_min_max(
                body.min + vec2(12.0, 12.0),
                pos2(body.right() - 24.0 - bw, body.bottom() - 40.0),
            );
            let col = t("dlg-md-saved-col-name");
            let names = d.names.clone();
            let mut lu = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(list)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            let out = ui_kit::table(
                &mut lu,
                "fp-saved-commands-table",
                &[ui_kit::Column::rest(&col)],
                names.len(),
                SAVED_ROW_H,
                d.selected,
                list.height(),
                |cu, i, _| {
                    ui_kit::cell_text(cu, &names[i], false);
                },
            );
            if let Some(i) = out.clicked {
                d.selected = Some(i);
                d.rename = None;
            }
            if names.is_empty() {
                // leerer Zustand (nach dem Löschen des letzten Befehls im Dialog)
                let area = Rect::from_min_max(
                    pos2(list.left() + 1.0, list.top() + 1.0 + ui_kit::TABLE_HEAD_H),
                    list.max - vec2(1.0, 1.0),
                );
                let (title, hint) = (t("dlg-md-no-saved"), t("dlg-md-no-saved-info"));
                let h = ui_kit::empty_state_height(ui.ctx(), &title, Some(&hint), area.width())
                    .min(area.height());
                ui_kit::empty_state_at(
                    &ui.painter().with_clip_rect(area),
                    Rect::from_center_size(area.center(), vec2(area.width(), h)),
                    ui_kit::KitIcon::Commands,
                    &title,
                    Some(&hint),
                    &theme,
                );
            }
            let bx = body.right() - 12.0 - bw;
            let sel = d.selected.and_then(|i| d.names.get(i).cloned());
            if push_button(
                ui,
                Rect::from_min_size(pos2(bx, body.top() + 12.0), vec2(bw, 23.0)),
                &btn_l[0],
                false,
                sel.is_some(),
            ) {
                d.rename = sel.clone();
            }
            if push_button(
                ui,
                Rect::from_min_size(pos2(bx, body.top() + 40.0), vec2(bw, 23.0)),
                &btn_l[1],
                false,
                sel.is_some(),
            ) && let Some(n) = &sel
            {
                action = Some(json!({"action": "delete", "name": n}));
            }
            if let (Some(old), Some(buf)) = (sel.clone(), d.rename.clone()) {
                let er = Rect::from_min_size(
                    pos2(body.left() + 12.0, body.bottom() - 32.0),
                    vec2(list.width() - 10.0 - ow, 22.0),
                );
                if let Some(t) = fields::text(ui, "saved-rename", er, &buf, true) {
                    d.rename = Some(t);
                }
                if push_button(
                    ui,
                    Rect::from_min_size(pos2(er.right() + 6.0, er.top()), vec2(ow, 22.0)),
                    &ok_l,
                    true,
                    true,
                ) {
                    let new = d.rename.clone().unwrap_or_default();
                    if !new.trim().is_empty() && new != old {
                        action = Some(json!({"action": "rename", "name": old, "newName": new}));
                    }
                    d.rename = None;
                }
            }
            if push_button(
                ui,
                Rect::from_min_size(pos2(bx, body.bottom() - 34.0), vec2(bw, 23.0)),
                &btn_l[2],
                true,
                true,
            ) {
                close = true;
            }
        },
    );
    if let Some(a) = action {
        if let Err(e) =
            crate::ui_commands::execute(&mut st.session, "commands.manageSavedCommands", a)
        {
            st.push(Action::Status(e.to_string()));
        }
        let keep = d.selected;
        d = refresh_saved(st);
        d.selected = keep.filter(|i| *i < d.names.len());
    }
    st.dialogs.saved_commands = if close || r == DialogClose::Cancelled {
        None
    } else {
        Some(d)
    };
}
