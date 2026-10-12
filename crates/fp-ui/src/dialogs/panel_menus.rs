//! Bedienfeldmenüs (≡): Die Befehle selbst stehen in fp-engine (`panel.*`, mit Befehls-ID des
//! Originals). Ohne Parameter aus dem Menü aufgerufen, ergänzt die Oberfläche hier, was der Befehl
//! braucht – Dialoge (Neue Ebene 5987, Ebene duplizieren 19128, Ebene für Seiten freigeben 6554,
//! Miniaturoptionen 6272, Status hinzufügen/duplizieren 5990, Statuseigenschaften, Neues Symbol,
//! Symboleigenschaften 19104, Stiloptionen 6026, Einstellungen speichern 5974, Optimieren für
//! Dateigröße 4734, Exportassistent 4717), Dateidialoge und die aktuelle Auswahl im Bedienfeld
//! (Verlaufsschritte, Stile, Symbol, Palettenfarben, [`PanelSelection`]).

use super::form::{Field, FormDialog};
use super::t;
use crate::app_state::{Action, AppState};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::PathBuf;

/// Auswahl in den Bedienfeldern (die Bedienfelder tragen sie je Bild ein).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PanelSelection {
    /// Ausgewählte Verlaufsschritte.
    pub history: BTreeSet<usize>,
    /// Ausgewählte Stile (Namen).
    pub styles: Vec<String>,
    /// Ausgewähltes Symbol der Dokumentbibliothek.
    pub symbol: Option<usize>,
    /// Ausgewählte Farben der Farbtabelle im Optimieren-Bedienfeld (ARGB).
    pub palette: Vec<u32>,
}

/// Dateifilter: (Beschreibung bzw. Textschlüssel `dlg-…`, Endungen).
type Filters<'a> = &'a [(&'a str, &'a [&'a str])];

/// Datei zum Öffnen wählen, ohne Blockieren (in Tests über [`crate::files::FilePickOverride`]):
/// `done` läuft nur, wenn eine Datei gewählt wurde.
pub fn pick_open(st: &mut AppState, title: &str, filters: Filters, done: impl FnOnce(&mut AppState, PathBuf) + Send + 'static) {
    if let Some(o) = &st.file_override {
        if let Some(p) = o.open.first().cloned() {
            done(st, p);
        }
        return;
    }
    let mut req = crate::file_dialog::Request::open(title);
    for (n, e) in filters {
        req = req.filter(filter_name(n), e);
    }
    crate::file_dialog::request(st, req, move |st, r| {
        if let Some(p) = r.into_iter().next() {
            done(st, p);
        }
    });
}

/// Beschreibung eines Dateifilters: Textschlüssel (`dlg-…`) übersetzt, sonst wie angegeben.
fn filter_name(n: &str) -> String {
    if n.starts_with("dlg-") { t(n) } else { n.to_string() }
}

/// „Befehl speichern“ (Vorlage 5968, wie im Original eine Namensabfrage statt Dateidialog): der
/// Befehl landet als `<Name>.jsf` im Befehlsordner und erscheint im Befehle-Menü.
pub fn open_save_command(st: &mut AppState, steps: &[usize]) {
    let n = std::fs::read_dir(fp_engine::commands::misc::commands_dir()).map(|r| r.count()).unwrap_or(0) + 1;
    let f = super::form::FormDialog::new("5968", &t("dlg-pmenu-save-command"), "panel.history.saveAsCommand", vec![super::form::Field::text("name", &t("dlg-lbl-name"), &fp_i18n::tr!("dlg-pmenu-command-default-name", n = n))])
        .with_base(json!({ "steps": steps, "_dir": command_dir(st).to_string_lossy() }))
        .with_transform(save_command_params);
    st.dialogs.form = Some(f);
}

/// Befehlsordner (in Tests der Ordner der Speicher-Vorgabe aus [`crate::files::FilePickOverride`]).
fn command_dir(st: &AppState) -> PathBuf {
    st.file_override.as_ref().and_then(|o| o.save.as_ref()).and_then(|p| p.parent().map(|d| d.join(t("dlg-pmenu-commands-folder")))).unwrap_or_else(fp_engine::commands::misc::commands_dir)
}

fn save_command_params(d: &super::form::FormDialog) -> serde_json::Value {
    let name = d.get("name").as_str().map(str::to_string).unwrap_or_else(|| t("dlg-pmenu-command-fallback")).trim().replace(['\\', '/', ':', '*', '?', '"', '<', '>', '|'], "_");
    let name = if name.is_empty() { t("dlg-pmenu-command-fallback") } else { name };
    let dir = d.base.get("_dir").and_then(|v| v.as_str()).map(PathBuf::from).unwrap_or_else(fp_engine::commands::misc::commands_dir);
    let _ = std::fs::create_dir_all(&dir);
    json!({ "steps": d.base.get("steps").cloned().unwrap_or(json!([])), "path": dir.join(format!("{name}.jsf")).to_string_lossy() })
}

/// Speicherziel wählen, ohne Blockieren (in Tests über [`crate::files::FilePickOverride`]): `done`
/// läuft nur, wenn ein Ziel gewählt wurde.
pub fn pick_save(st: &mut AppState, title: &str, name: &str, filters: Filters, done: impl FnOnce(&mut AppState, PathBuf) + Send + 'static) {
    if let Some(o) = &st.file_override {
        if let Some(p) = o.save.clone() {
            done(st, p);
        }
        return;
    }
    let mut req = crate::file_dialog::Request::save(title, name);
    for (n, e) in filters {
        req = req.filter(filter_name(n), e);
    }
    crate::file_dialog::request(st, req, move |st, r| {
        if let Some(p) = r.into_iter().next() {
            done(st, p);
        }
    });
}

/// Datei wählen und den Befehl `name` mit `path` ausführen.
fn open_then_run(st: &mut AppState, title: &str, filters: Filters, name: &str) {
    let n = name.to_string();
    pick_open(st, title, filters, move |st, p| run_with_path(st, &n, Some(p), json!({})));
}

/// Speicherziel wählen und den Befehl `name` mit `path` ausführen.
fn save_then_run(st: &mut AppState, title: &str, file: &str, filters: Filters, name: &str) {
    let n = name.to_string();
    pick_save(st, title, file, filters, move |st, p| run_with_path(st, &n, Some(p), json!({})));
}

const SWATCH_FILES: Filters = &[("dlg-filter-act", &["act"]), ("dlg-filter-ase", &["ase"]), ("dlg-filter-aco", &["aco"]), ("GIF (*.gif)", &["gif"])];
const SWATCH_SAVE: Filters = &[("dlg-filter-act", &["act"]), ("dlg-filter-ase", &["ase"]), ("dlg-filter-aco", &["aco"])];
const STYLE_FILES: Filters = &[("dlg-filter-style-library", &["json"]), ("FW-PNG (*.png)", &["png"])];
const LIB_FILES: Filters = &[("dlg-filter-fwpng-library", &["png"])];

fn run_with_path(st: &mut AppState, name: &str, path: Option<PathBuf>, mut extra: Value) {
    let Some(p) = path else { return };
    extra["path"] = json!(p.to_string_lossy());
    st.run(name, extra);
}

fn colors_hex(v: &[u32]) -> Vec<String> {
    v.iter().map(|c| format!("#{:06x}", c & 0x00ff_ffff)).collect()
}

fn size_radio(key_value: i64) -> Field {
    let sizes = [0, 16, 32, 48, 64];
    let idx = sizes.iter().position(|s| *s == key_value).unwrap_or(2);
    Field::radio_of("size", t("dlg-pmenu-thumb-size").as_str(), &[t("dlg-pmenu-thumb-none").as_str(), t("dlg-pmenu-thumb-small").as_str(), t("dlg-pmenu-thumb-medium").as_str(), t("dlg-pmenu-thumb-large").as_str(), t("dlg-pmenu-thumb-xlarge").as_str()], sizes.iter().map(|s| json!(s)).collect(), idx)
}

fn where_radio(default: usize) -> Field {
    Field::radio_of("where", t("dlg-lbl-insert").as_str(), &[t("dlg-insert-start").as_str(), t("dlg-insert-before-state").as_str(), t("dlg-insert-after-state").as_str(), t("dlg-insert-end").as_str()], vec![json!("start"), json!("before"), json!("after"), json!("end")], default)
}

fn share_pages_params(d: &FormDialog) -> Value {
    let pages: Vec<usize> = d.fields.iter().filter(|f| f.key.starts_with("_page") && f.value.as_bool() == Some(true)).filter_map(|f| f.key.trim_start_matches("_page").parse().ok()).collect();
    json!({ "pages": pages })
}

fn symbol_form_params(d: &FormDialog) -> Value {
    let mut m = match &d.base {
        Value::Object(o) => o.clone(),
        _ => serde_json::Map::new(),
    };
    for f in &d.fields {
        if !f.key.starts_with('_') {
            m.insert(f.key.clone(), f.param());
        }
    }
    if let (Some(dir), Some(name)) = (d.base.get("_dir").and_then(|v| v.as_str()), d.get("_file").as_str()) {
        let file = if name.to_lowercase().ends_with(".png") { name.to_string() } else { format!("{name}.png") };
        m.insert("path".into(), json!(std::path::Path::new(dir).join(file).to_string_lossy()));
    }
    m.remove("_dir");
    Value::Object(m)
}

fn wizard_params(d: &FormDialog) -> Value {
    let mut v = json!({ "target": d.get("target") });
    if d.get("_limit").as_bool() == Some(true) {
        v["kb"] = d.get("_kb");
    }
    v
}

fn style_options_params(d: &FormDialog) -> Value {
    let mut v = super::misc::style_params(d);
    v["name"] = d.base.get("name").cloned().unwrap_or(Value::Null);
    v["newName"] = d.get("name");
    v
}

/// Ergänzt den Menübefehl `name` (ohne Parameter) um Dialog, Datei oder Auswahl; `true`, wenn
/// übernommen.
pub fn open(st: &mut AppState, name: &str) -> bool {
    let sel = st.dialogs.panel_sel.clone();
    let prefs_int = |st: &AppState, k: &str, d: i64| st.session.prefs.get_int(k, d);
    let form = match name {
        // Ebenen
        "panel.layers.newLayer" => {
            let n = st.session.active().map(|d| fp_engine::docops::own_layers(&d.doc, d.page).len()).unwrap_or(1);
            Some(FormDialog::new("5987", t("dlg-title-new-layer").as_str(), name, vec![Field::text("name", t("dlg-lbl-name").as_str(), &fp_i18n::tr!("msg-default-layer", n = n.to_string())), Field::check("shared", t("dlg-lbl-share-across-states").as_str(), false)]).with_width(360.0))
        }
        "panel.layers.newSubLayer" => Some(FormDialog::new("5987", t("dlg-pmenu-new-sublayer").as_str(), name, vec![Field::text("name", t("dlg-lbl-name").as_str(), &t("msg-default-sublayer"))]).with_width(360.0)),
        "panel.layers.duplicate" => Some(
            FormDialog::new(
                "19128",
                t("dlg-title-duplicate-layer").as_str(),
                name,
                vec![Field::number("count", t("dlg-lbl-count").as_str(), 1.0, 1.0, 100.0), Field::radio_of("position", t("dlg-lbl-insert").as_str(), &[t("dlg-insert-top").as_str(), t("dlg-insert-before-layer").as_str(), t("dlg-insert-after-layer").as_str(), t("dlg-insert-bottom").as_str()], vec![json!("top"), json!("above"), json!("below"), json!("bottom")], 1)],
            )
            .with_width(560.0),
        ),
        "panel.layers.shareToPages" => {
            let Some(d) = st.session.active() else { return false };
            let mut fields = vec![Field::caption(t("dlg-pmenu-share-for").as_str())];
            for (i, p) in d.doc.pages.iter().enumerate() {
                if i != d.page {
                    fields.push(Field::check(&format!("_page{i}"), &p.name.clone().unwrap_or_else(|| fp_i18n::tr!("msg-default-page", n = (i + 1).to_string())), false));
                }
            }
            if fields.len() == 1 {
                fields.push(Field::note(t("dlg-pmenu-only-one-page").as_str()));
            }
            Some(FormDialog::new("6554", t("dlg-pmenu-share-to-pages").as_str(), name, fields).with_width(360.0).with_transform(share_pages_params))
        }
        "panel.layers.thumbnailOptions" => Some(FormDialog::new("6272", t("dlg-pmenu-thumb-options").as_str(), name, vec![size_radio(prefs_int(st, "LayersList_ThumbnailSize", 32))]).with_width(420.0)),
        "panel.pages.thumbnailOptions" => Some(FormDialog::new("6272", t("dlg-pmenu-thumb-view-options").as_str(), name, vec![size_radio(prefs_int(st, "PagesList_ThumbnailSize", 32))]).with_width(420.0)),
        // Status
        "panel.states.add" => Some(FormDialog::new("5990", t("dlg-title-add-states").as_str(), name, vec![Field::number("count", t("dlg-lbl-count").as_str(), 1.0, 1.0, 1000.0), where_radio(3)]).with_width(560.0)),
        "panel.states.duplicate" => Some(FormDialog::new("5990", t("dlg-pmenu-duplicate-states").as_str(), name, vec![Field::number("count", t("dlg-lbl-count").as_str(), 1.0, 1.0, 1000.0), where_radio(2)]).with_width(560.0)),
        "panel.states.copyTo" => super::catalog::form_for(st, "panel.states.copyTo"),
        "panel.states.properties" => {
            let Some(d) = st.session.active() else { return false };
            let f = d.doc.pages.get(d.page).and_then(|p| p.root.export.as_ref()).map(|e| e.frame(d.frame)).unwrap_or_default();
            let n = crate::panels::state_name(st, d.frame);
            Some(FormDialog::new("states-props", t("dlg-pmenu-state-props").as_str(), name, vec![Field::text("name", t("dlg-lbl-name").as_str(), &n), Field::number("delay", t("dlg-pmenu-state-delay").as_str(), f.delay as f64, 0.0, 9999.0), Field::check("visible", t("dlg-pmenu-include-export").as_str(), f.visible)]).with_width(380.0))
        }
        // Seiten
        "panel.pages.rename" => {
            let Some(d) = st.session.active() else { return false };
            let n = d.doc.pages.get(d.page).and_then(|p| p.name.clone()).unwrap_or_else(|| fp_i18n::tr!("msg-default-page", n = (d.page + 1).to_string()));
            Some(FormDialog::new("page-rename", t("dlg-pmenu-rename-page").as_str(), name, vec![Field::text("name", t("dlg-lbl-name").as_str(), &n)]))
        }
        "panel.pages.export" => {
            let n = st.session.active().map(|d| d.title.trim_end_matches(".png").trim_end_matches(".fw").to_string()).unwrap_or_default();
            save_then_run(st, &t("dlg-pmenu-export-page"), &format!("{n}.png"), &[("dlg-filter-image-optimize", &["png", "gif", "jpg", "webp"])], name);
            return true;
        }
        // Verlauf
        "panel.history.replaySelected" | "panel.history.copySteps" | "panel.history.saveAsCommand" => {
            let steps: Vec<usize> = sel.history.iter().copied().collect();
            if steps.is_empty() {
                st.push(Action::Status(t("dlg-pmenu-need-steps")));
                return true;
            }
            match name {
                "panel.history.copySteps" => match crate::ui_commands::execute(&mut st.session, name, json!({ "steps": steps })) {
                    Ok(o) => {
                        st.dialogs.clipboard = o.value.get("script").and_then(|v| v.as_str()).map(|s| s.to_string());
                        st.push(Action::Status(t("dlg-pmenu-steps-copied")));
                    }
                    Err(e) => st.push(Action::Status(e.to_string())),
                },
                "panel.history.saveAsCommand" => open_save_command(st, &steps),
                _ => st.run(name, json!({ "steps": steps })),
            }
            return true;
        }
        "panel.history.clear" => {
            st.dialogs.confirm = Some(super::Confirm { title: t("dlg-pmenu-clear-history"), text: t("dlg-pmenu-clear-history-text"), command: name.into(), params: json!({ "confirmed": true }) });
            return true;
        }
        // Stile
        "panel.styles.new" => {
            st.dialogs.open_new_style();
            return true;
        }
        "panel.styles.duplicate" | "panel.styles.redefine" => {
            let Some(n) = sel.styles.first() else {
                st.push(Action::Status(t("dlg-pmenu-need-style")));
                return true;
            };
            st.run(name, json!({ "name": n }));
            return true;
        }
        "panel.styles.delete" => {
            if sel.styles.is_empty() {
                st.push(Action::Status(t("dlg-pmenu-need-style")));
                return true;
            }
            st.run(name, json!({ "names": sel.styles }));
            st.dialogs.panel_sel.styles.clear();
            return true;
        }
        "panel.styles.rename" => {
            let Some(n) = sel.styles.first() else { return true };
            Some(FormDialog::new("19175", t("dlg-pmenu-rename-style").as_str(), name, vec![Field::text("newName", t("dlg-lbl-name").as_str(), n)]).with_base(json!({ "name": n })))
        }
        "panel.styles.options" => {
            let Some(n) = sel.styles.first() else {
                st.push(Action::Status(t("dlg-pmenu-need-style")));
                return true;
            };
            let s = st.session.active().and_then(|d| d.doc.styles.as_ref().and_then(|v| v.iter().find(|x| &x.name == n).cloned()));
            let (fl, ot) = s.map(|x| (x.flags, x.other_flags)).unwrap_or((0x1ff, 0x1f));
            let mut f = super::misc::new_style_form(st);
            f.id = "6026".into();
            f.title = t("dlg-pmenu-edit-style");
            f.command = name.into();
            f.base = json!({ "name": n });
            for fd in &mut f.fields {
                let on = match fd.key.as_str() {
                    "name" => {
                        fd.value = json!(n);
                        continue;
                    }
                    "_strokeType" => fl & 1 != 0,
                    "_strokeColor" => fl & 2 != 0,
                    "_fillType" => fl & 4 != 0,
                    "_fillColor" => fl & 8 != 0,
                    "_font" => ot & 1 != 0,
                    "_size" => ot & 2 != 0,
                    "_style" => ot & 4 != 0,
                    "_effect" => ot & 8 != 0,
                    "_other" => ot & 16 != 0,
                    _ => continue,
                };
                fd.value = json!(on);
            }
            Some(f.with_transform(style_options_params))
        }
        "panel.styles.selectUnused" => {
            if let Ok(o) = crate::ui_commands::execute(&mut st.session, name, json!({})) {
                st.dialogs.panel_sel.styles = o.value.get("names").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect()).unwrap_or_default();
            }
            return true;
        }
        "panel.styles.load" | "panel.styles.importLibrary" => {
            open_then_run(st, &t("dlg-pmenu-load-styles"), STYLE_FILES, name);
            return true;
        }
        "panel.styles.saveLibrary" => {
            save_then_run(st, &t("dlg-pmenu-save-style-library"), &format!("{}.json", t("dlg-pmenu-styles-file")), STYLE_FILES, name);
            return true;
        }
        // Dokumentbibliothek
        "panel.library.newSymbol" => Some(
            FormDialog::new("19104", t("dlg-md-new-symbol").as_str(), name, vec![Field::text("name", t("dlg-lbl-name").as_str(), &t("dlg-symbol-default-name")), Field::radio_of("kind", t("dlg-lbl-type").as_str(), &[t("dlg-symbol-graphic").as_str(), t("dlg-symbol-animation").as_str(), t("dlg-symbol-button").as_str()], vec![json!("graphic"), json!("animation"), json!("button")], 0)]).with_width(420.0),
        ),
        "panel.library.duplicate" | "panel.library.edit" | "panel.library.nineSlice" | "panel.library.lockNineSlice" | "panel.library.delete" | "panel.library.properties" | "panel.library.saveToCommon" => {
            let Some(i) = sel.symbol else {
                st.push(Action::Status(t("dlg-pmenu-need-symbol")));
                return true;
            };
            let sym = st.session.active().and_then(|d| d.doc.symbols.get(i).cloned());
            let Some(sym) = sym else { return true };
            match name {
                "panel.library.delete" => {
                    st.dialogs.confirm = Some(super::Confirm { title: t("dlg-pmenu-delete-symbol"), text: fp_i18n::tr!("dlg-pmenu-delete-symbol-text", name = sym.name.as_str()), command: name.into(), params: json!({ "index": i }) });
                    st.dialogs.panel_sel.symbol = None;
                    return true;
                }
                "panel.library.properties" => {
                    let k = match sym.kind() {
                        fp_model::SymbolKind::Animation => 1,
                        fp_model::SymbolKind::Button => 2,
                        _ => 0,
                    };
                    Some(
                        FormDialog::new(
                            "19104",
                            t("dlg-pmenu-symbol-props").as_str(),
                            name,
                            vec![
                                Field::text("name", t("dlg-lbl-name").as_str(), &sym.name),
                                Field::radio_of("kind", t("dlg-lbl-type").as_str(), &[t("dlg-symbol-graphic").as_str(), t("dlg-symbol-animation").as_str(), t("dlg-symbol-button").as_str()], vec![json!("graphic"), json!("animation"), json!("button")], k),
                                Field::check("nineSlice", t("dlg-lbl-enable-nineslice").as_str(), sym.nine_slice.is_some()),
                            ],
                        )
                        .with_base(json!({ "index": i }))
                        .with_width(420.0),
                    )
                }
                "panel.library.saveToCommon" => {
                    let dir = crate::panels::library::library_roots().into_iter().next().map(|(_, p)| p).unwrap_or_default();
                    Some(
                        FormDialog::new("6556", t("dlg-pmenu-save-common").as_str(), name, vec![Field::text("_file", t("dlg-export-file-name").as_str(), &format!("{}.png", sym.name))])
                            .with_base(json!({ "index": i, "_dir": dir.to_string_lossy() }))
                            .with_width(380.0)
                            .with_transform(symbol_form_params),
                    )
                }
                _ => {
                    st.run(name, json!({ "index": i }));
                    return true;
                }
            }
        }
        "panel.library.selectUnused" => {
            if let Ok(o) = crate::ui_commands::execute(&mut st.session, name, json!({})) {
                let first = o.value.get("indices").and_then(|v| v.as_array()).and_then(|a| a.first()).and_then(|v| v.as_u64()).map(|v| v as usize);
                st.dialogs.panel_sel.symbol = first;
                if first.is_none() {
                    st.push(Action::Status(t("dlg-pmenu-all-symbols-used")));
                }
            }
            return true;
        }
        "panel.library.import" => {
            open_then_run(st, &t("dlg-pmenu-import-symbols"), LIB_FILES, name);
            return true;
        }
        "panel.library.export" => {
            save_then_run(st, &t("dlg-pmenu-export-symbols"), &format!("{}.png", t("dlg-pmenu-symbols-file")), LIB_FILES, name);
            return true;
        }
        // Farbfelder
        "panel.swatches.add" | "panel.swatches.replace" => {
            open_then_run(st, &t("dlg-pmenu-open-swatches"), SWATCH_FILES, name);
            return true;
        }
        "panel.swatches.save" => {
            save_then_run(st, &t("dlg-pmenu-save-swatches"), &format!("{}.act", t("dlg-pmenu-swatches-file")), SWATCH_SAVE, name);
            return true;
        }
        // Optimieren
        "panel.optimize.saveSettings" => Some(FormDialog::new("5974", t("dlg-pmenu-preset-name").as_str(), name, vec![Field::text("name", t("dlg-lbl-name").as_str(), &t("dlg-pmenu-own-setting"))])),
        "panel.optimize.deleteSettings" => {
            let names: Vec<String> = crate::ui_commands::export_presets(&st.session.prefs).into_iter().map(|(n, _)| n).collect();
            if names.is_empty() {
                st.dialogs.message = Some((t("dlg-pmenu-delete-settings"), t("dlg-pmenu-no-settings")));
                return true;
            }
            let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
            Some(FormDialog::new("5974", t("dlg-pmenu-delete-settings").as_str(), name, vec![Field::choice_of("name", t("dlg-pmenu-preset").as_str(), &refs, names.iter().map(|n| json!(n)).collect(), 0)]))
        }
        "panel.optimize.optimizeToSize" => Some(FormDialog::new("4734", t("dlg-pmenu-optimize-size").as_str(), name, vec![Field::number("kb", t("dlg-md-target-size").as_str(), 32.0, 0.1, 100000.0).decimals(1).unit("K")])),
        "panel.optimize.exportWizard" => Some(
            FormDialog::new(
                "4717",
                t("dlg-md-export-wizard").as_str(),
                name,
                vec![
                    Field::radio_of("target", t("dlg-md-destination").as_str(), &[t("dlg-md-dest-web").as_str(), t("dlg-md-dest-image").as_str(), t("dlg-md-dest-html").as_str()], vec![json!("web"), json!("imageEditing"), json!("dreamweaver")], 0),
                    Field::check("_limit", t("dlg-md-limit-size").as_str(), false),
                    Field::number("_kb", t("dlg-md-target-size").as_str(), 32.0, 0.1, 100000.0).decimals(1).unit("K"),
                    Field::note(t("dlg-pmenu-wizard-note").as_str()),
                ],
            )
            .with_width(480.0)
            .with_transform(wizard_params),
        ),
        "panel.optimize.editColor" => {
            let Some(c) = sel.palette.first().copied() else {
                st.push(Action::Status(t("dlg-pmenu-need-color")));
                return true;
            };
            let hex = format!("#{:06x}ff", c & 0x00ff_ffff);
            Some(FormDialog::new("4630", t("dlg-pmenu-edit-color").as_str(), name, vec![Field::color("newColor", t("dlg-pmenu-new-color").as_str(), &hex)]).with_base(json!({ "color": format!("#{:06x}", c & 0x00ff_ffff) })))
        }
        "panel.optimize.addColorMenu" => Some(FormDialog::new("4630", t("dlg-pmenu-add-color").as_str(), "panel.optimize.addColor", vec![Field::color("color", t("dlg-lbl-color").as_str(), "#808080ff")])),
        "panel.optimize.editColorMenu" => return open(st, "panel.optimize.editColor"),
        "panel.optimize.deleteColorMenu" | "panel.optimize.snapToWebMenu" | "panel.optimize.lockColorMenu" => {
            if sel.palette.is_empty() {
                st.push(Action::Status(t("dlg-pmenu-need-color")));
                return true;
            }
            let target = match name {
                "panel.optimize.deleteColorMenu" => "panel.optimize.deleteColors",
                "panel.optimize.snapToWebMenu" => "panel.optimize.snapToWeb",
                _ => "panel.optimize.lockColors",
            };
            st.run(target, json!({ "colors": colors_hex(&sel.palette) }));
            return true;
        }
        "panel.effects.saveAsStyle" => {
            let n = st.session.active().and_then(|d| d.doc.styles.as_ref().map(|s| s.len())).unwrap_or(0) + 1;
            Some(FormDialog::new("19123", t("dlg-pmenu-save-as-style").as_str(), name, vec![Field::text("name", t("dlg-lbl-name").as_str(), &fp_i18n::tr!("dlg-style-default-name", n = n))]))
        }
        "panel.effects.locatePlugins" => {
            match &st.file_override {
                Some(o) => {
                    let dir = o.open.first().cloned();
                    run_with_path(st, name, dir, json!({}));
                }
                None => {
                    let n = name.to_string();
                    crate::file_dialog::request(st, crate::file_dialog::Request::folder(t("dlg-pmenu-locate-plugins")), move |st, r| run_with_path(st, &n, r.into_iter().next(), json!({})));
                }
            }
            return true;
        }
        "panel.optimize.makeTransparent" | "panel.optimize.removeEdit" => {
            if sel.palette.is_empty() {
                st.push(Action::Status(t("dlg-pmenu-need-color")));
                return true;
            }
            st.run(name, json!({ "colors": colors_hex(&sel.palette) }));
            return true;
        }
        "panel.optimize.loadPalette" => {
            open_then_run(st, &t("dlg-pmenu-load-palette"), SWATCH_FILES, name);
            return true;
        }
        "panel.optimize.savePalette" => {
            save_then_run(st, &t("dlg-pmenu-save-palette"), &format!("{}.act", t("dlg-pmenu-palette-file")), SWATCH_SAVE, name);
            return true;
        }
        _ => return false,
    };
    if let Some(f) = form {
        st.dialogs.form = Some(super::catalog::with_doc_base(st, f));
    }
    true
}

/// Befehle der Bedienfeldmenüs, die ohne Parameter einen Dialog bzw. eine Auswahl brauchen.
pub fn needs_ui(name: &str) -> bool {
    matches!(
        name,
        "panel.layers.newLayer"
            | "panel.layers.newSubLayer"
            | "panel.layers.duplicate"
            | "panel.layers.shareToPages"
            | "panel.layers.thumbnailOptions"
            | "panel.pages.thumbnailOptions"
            | "panel.states.add"
            | "panel.states.duplicate"
            | "panel.states.copyTo"
            | "panel.states.properties"
            | "panel.pages.rename"
            | "panel.pages.export"
            | "panel.history.replaySelected"
            | "panel.history.copySteps"
            | "panel.history.saveAsCommand"
            | "panel.history.clear"
            | "panel.styles.new"
            | "panel.styles.duplicate"
            | "panel.styles.redefine"
            | "panel.styles.delete"
            | "panel.styles.rename"
            | "panel.styles.options"
            | "panel.styles.selectUnused"
            | "panel.styles.load"
            | "panel.styles.importLibrary"
            | "panel.styles.saveLibrary"
            | "panel.library.newSymbol"
            | "panel.library.duplicate"
            | "panel.library.edit"
            | "panel.library.nineSlice"
            | "panel.library.lockNineSlice"
            | "panel.library.delete"
            | "panel.library.properties"
            | "panel.library.saveToCommon"
            | "panel.library.selectUnused"
            | "panel.library.import"
            | "panel.library.export"
            | "panel.swatches.add"
            | "panel.swatches.replace"
            | "panel.swatches.save"
            | "panel.optimize.saveSettings"
            | "panel.optimize.deleteSettings"
            | "panel.optimize.optimizeToSize"
            | "panel.optimize.exportWizard"
            | "panel.optimize.editColor"
            | "panel.optimize.makeTransparent"
            | "panel.optimize.removeEdit"
            | "panel.optimize.loadPalette"
            | "panel.optimize.savePalette"
            | "panel.optimize.addColorMenu"
            | "panel.optimize.editColorMenu"
            | "panel.optimize.deleteColorMenu"
            | "panel.optimize.snapToWebMenu"
            | "panel.optimize.lockColorMenu"
            | "panel.effects.saveAsStyle"
            | "panel.effects.locatePlugins"
    )
}
