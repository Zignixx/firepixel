//! Bedienfeldmenüs (≡): Die Befehle selbst stehen in fp-engine (`panel.*`, mit Befehls-ID des
//! Originals). Ohne Parameter aus dem Menü aufgerufen, ergänzt die Oberfläche hier, was der Befehl
//! braucht – Dialoge (Neue Ebene 5987, Ebene duplizieren 19128, Ebene für Seiten freigeben 6554,
//! Miniaturoptionen 6272, Status hinzufügen/duplizieren 5990, Statuseigenschaften, Neues Symbol,
//! Symboleigenschaften 19104, Stiloptionen 6026, Einstellungen speichern 5974, Optimieren für
//! Dateigröße 4734, Exportassistent 4717), Dateidialoge und die aktuelle Auswahl im Bedienfeld
//! (Verlaufsschritte, Stile, Symbol, Palettenfarben, [`PanelSelection`]).

use super::form::{Field, FormDialog};
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

/// Dateifilter: (Beschreibung, Endungen).
type Filters<'a> = &'a [(&'a str, &'a [&'a str])];

/// Datei zum Öffnen wählen (in Tests über [`crate::files::FilePickOverride`]).
pub fn pick_open(st: &AppState, title: &str, filters: Filters) -> Option<PathBuf> {
    if let Some(o) = &st.file_override {
        return o.open.first().cloned();
    }
    let mut d = rfd::FileDialog::new().set_title(title);
    for (n, e) in filters {
        d = d.add_filter(*n, e);
    }
    d.pick_file()
}

/// „Befehl speichern“ (Vorlage 5968, wie im Original eine Namensabfrage statt Dateidialog): der
/// Befehl landet als `<Name>.jsf` im Befehlsordner und erscheint im Befehle-Menü.
pub fn open_save_command(st: &mut AppState, steps: &[usize]) {
    let n = std::fs::read_dir(fp_engine::commands::misc::commands_dir()).map(|r| r.count()).unwrap_or(0) + 1;
    let f = super::form::FormDialog::new("5968", "Befehl speichern", "panel.history.saveAsCommand", vec![super::form::Field::text("name", "Name:", &format!("Befehl {n}"))])
        .with_base(json!({ "steps": steps, "_dir": command_dir(st).to_string_lossy() }))
        .with_transform(save_command_params);
    st.dialogs.form = Some(f);
}

/// Befehlsordner (in Tests der Ordner der Speicher-Vorgabe aus [`crate::files::FilePickOverride`]).
fn command_dir(st: &AppState) -> PathBuf {
    st.file_override.as_ref().and_then(|o| o.save.as_ref()).and_then(|p| p.parent().map(|d| d.join("Befehle"))).unwrap_or_else(fp_engine::commands::misc::commands_dir)
}

fn save_command_params(d: &super::form::FormDialog) -> serde_json::Value {
    let name = d.get("name").as_str().unwrap_or("Befehl").trim().replace(['\\', '/', ':', '*', '?', '"', '<', '>', '|'], "_");
    let name = if name.is_empty() { "Befehl".to_string() } else { name };
    let dir = d.base.get("_dir").and_then(|v| v.as_str()).map(PathBuf::from).unwrap_or_else(fp_engine::commands::misc::commands_dir);
    let _ = std::fs::create_dir_all(&dir);
    json!({ "steps": d.base.get("steps").cloned().unwrap_or(json!([])), "path": dir.join(format!("{name}.jsf")).to_string_lossy() })
}

/// Speicherziel wählen (in Tests über [`crate::files::FilePickOverride`]).
pub fn pick_save(st: &AppState, title: &str, name: &str, filters: Filters) -> Option<PathBuf> {
    if let Some(o) = &st.file_override {
        return o.save.clone();
    }
    let mut d = rfd::FileDialog::new().set_title(title).set_file_name(name);
    for (n, e) in filters {
        d = d.add_filter(*n, e);
    }
    d.save_file()
}

const SWATCH_FILES: Filters = &[("Farbtabelle (*.act)", &["act"]), ("Swatch Exchange (*.ase)", &["ase"]), ("Farbfelder (*.aco)", &["aco"]), ("GIF (*.gif)", &["gif"])];
const SWATCH_SAVE: Filters = &[("Farbtabelle (*.act)", &["act"]), ("Swatch Exchange (*.ase)", &["ase"]), ("Farbfelder (*.aco)", &["aco"])];
const STYLE_FILES: Filters = &[("Stilbibliothek (*.json)", &["json"]), ("FW-PNG (*.png)", &["png"])];
const LIB_FILES: Filters = &[("FW-PNG-Bibliothek (*.png)", &["png"])];

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
    Field::radio_of("size", "Miniaturgröße:", &["Keine", "Klein", "Mittel", "Groß", "Sehr groß"], sizes.iter().map(|s| json!(s)).collect(), idx)
}

fn where_radio(default: usize) -> Field {
    Field::radio_of("where", "Einfügen:", &["Am Anfang", "Vor aktuellem Status", "Nach aktuellem Status", "Am Ende"], vec![json!("start"), json!("before"), json!("after"), json!("end")], default)
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
            Some(FormDialog::new("5987", "Neue Ebene", name, vec![Field::text("name", "Name:", &format!("Ebene {n}")), Field::check("shared", "Statusübergreifend freigeben", false)]).with_width(360.0))
        }
        "panel.layers.newSubLayer" => Some(FormDialog::new("5987", "Neue Unterebene", name, vec![Field::text("name", "Name:", "Unterebene")]).with_width(360.0)),
        "panel.layers.duplicate" => Some(
            FormDialog::new(
                "19128",
                "Ebene duplizieren",
                name,
                vec![Field::number("count", "Anzahl:", 1.0, 1.0, 100.0), Field::radio_of("position", "Einfügen:", &["Oben", "Vor aktueller Ebene", "Nach aktueller Ebene", "Unten"], vec![json!("top"), json!("above"), json!("below"), json!("bottom")], 1)],
            )
            .with_width(560.0),
        ),
        "panel.layers.shareToPages" => {
            let Some(d) = st.session.active() else { return false };
            let mut fields = vec![Field::caption("Ebene freigeben für:")];
            for (i, p) in d.doc.pages.iter().enumerate() {
                if i != d.page {
                    fields.push(Field::check(&format!("_page{i}"), &p.name.clone().unwrap_or_else(|| format!("Seite {}", i + 1)), false));
                }
            }
            if fields.len() == 1 {
                fields.push(Field::note("Das Dokument hat nur eine Seite."));
            }
            Some(FormDialog::new("6554", "Ebene für Seiten freigeben", name, fields).with_width(360.0).with_transform(share_pages_params))
        }
        "panel.layers.thumbnailOptions" => Some(FormDialog::new("6272", "Miniaturoptionen", name, vec![size_radio(prefs_int(st, "LayersList_ThumbnailSize", 32))]).with_width(420.0)),
        "panel.pages.thumbnailOptions" => Some(FormDialog::new("6272", "Optionen für Miniaturansicht", name, vec![size_radio(prefs_int(st, "PagesList_ThumbnailSize", 32))]).with_width(420.0)),
        // Status
        "panel.states.add" => Some(FormDialog::new("5990", "Status hinzufügen", name, vec![Field::number("count", "Anzahl:", 1.0, 1.0, 1000.0), where_radio(3)]).with_width(560.0)),
        "panel.states.duplicate" => Some(FormDialog::new("5990", "Status duplizieren", name, vec![Field::number("count", "Anzahl:", 1.0, 1.0, 1000.0), where_radio(2)]).with_width(560.0)),
        "panel.states.copyTo" => super::catalog::form_for(st, "panel.states.copyTo"),
        "panel.states.properties" => {
            let Some(d) = st.session.active() else { return false };
            let f = d.doc.pages.get(d.page).and_then(|p| p.root.export.as_ref()).map(|e| e.frame(d.frame)).unwrap_or_default();
            let n = crate::panels::state_name(st, d.frame);
            Some(FormDialog::new("states-props", "Statuseigenschaften", name, vec![Field::text("name", "Name:", &n), Field::number("delay", "Statusverzögerung (1/100 s):", f.delay as f64, 0.0, 9999.0), Field::check("visible", "Beim Exportieren einschließen", f.visible)]).with_width(380.0))
        }
        // Seiten
        "panel.pages.rename" => {
            let Some(d) = st.session.active() else { return false };
            let n = d.doc.pages.get(d.page).and_then(|p| p.name.clone()).unwrap_or_else(|| format!("Seite {}", d.page + 1));
            Some(FormDialog::new("page-rename", "Seite umbenennen", name, vec![Field::text("name", "Name:", &n)]))
        }
        "panel.pages.export" => {
            let n = st.session.active().map(|d| d.title.trim_end_matches(".png").trim_end_matches(".fw").to_string()).unwrap_or_default();
            let p = pick_save(st, "Seite exportieren", &format!("{n}.png"), &[("Bild (Format laut Optimieren)", &["png", "gif", "jpg", "webp"])]);
            run_with_path(st, name, p, json!({}));
            return true;
        }
        // Verlauf
        "panel.history.replaySelected" | "panel.history.copySteps" | "panel.history.saveAsCommand" => {
            let steps: Vec<usize> = sel.history.iter().copied().collect();
            if steps.is_empty() {
                st.push(Action::Status("Bitte zuerst Schritte im Verlauf auswählen.".into()));
                return true;
            }
            match name {
                "panel.history.copySteps" => match crate::ui_commands::execute(&mut st.session, name, json!({ "steps": steps })) {
                    Ok(o) => {
                        st.dialogs.clipboard = o.value.get("script").and_then(|v| v.as_str()).map(|s| s.to_string());
                        st.push(Action::Status("Schritte in die Zwischenablage kopiert".into()));
                    }
                    Err(e) => st.push(Action::Status(e.to_string())),
                },
                "panel.history.saveAsCommand" => open_save_command(st, &steps),
                _ => st.run(name, json!({ "steps": steps })),
            }
            return true;
        }
        "panel.history.clear" => {
            st.dialogs.confirm = Some(super::Confirm { title: "Verlauf löschen".into(), text: "Das Löschen des Verlaufs kann nicht rückgängig gemacht werden. Fortfahren?".into(), command: name.into(), params: json!({ "confirmed": true }) });
            return true;
        }
        // Stile
        "panel.styles.new" => {
            st.dialogs.open_new_style();
            return true;
        }
        "panel.styles.duplicate" | "panel.styles.redefine" => {
            let Some(n) = sel.styles.first() else {
                st.push(Action::Status("Bitte zuerst einen Stil auswählen.".into()));
                return true;
            };
            st.run(name, json!({ "name": n }));
            return true;
        }
        "panel.styles.delete" => {
            if sel.styles.is_empty() {
                st.push(Action::Status("Bitte zuerst einen Stil auswählen.".into()));
                return true;
            }
            st.run(name, json!({ "names": sel.styles }));
            st.dialogs.panel_sel.styles.clear();
            return true;
        }
        "panel.styles.rename" => {
            let Some(n) = sel.styles.first() else { return true };
            Some(FormDialog::new("19175", "Stil umbenennen", name, vec![Field::text("newName", "Name:", n)]).with_base(json!({ "name": n })))
        }
        "panel.styles.options" => {
            let Some(n) = sel.styles.first() else {
                st.push(Action::Status("Bitte zuerst einen Stil auswählen.".into()));
                return true;
            };
            let s = st.session.active().and_then(|d| d.doc.styles.as_ref().and_then(|v| v.iter().find(|x| &x.name == n).cloned()));
            let (fl, ot) = s.map(|x| (x.flags, x.other_flags)).unwrap_or((0x1ff, 0x1f));
            let mut f = super::misc::new_style_form(st);
            f.id = "6026".into();
            f.title = "Stil bearbeiten".into();
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
            let p = pick_open(st, "Stile laden", STYLE_FILES);
            run_with_path(st, name, p, json!({}));
            return true;
        }
        "panel.styles.saveLibrary" => {
            let p = pick_save(st, "Stilbibliothek speichern", "Stile.json", STYLE_FILES);
            run_with_path(st, name, p, json!({}));
            return true;
        }
        // Dokumentbibliothek
        "panel.library.newSymbol" => Some(
            FormDialog::new("19104", "Neues Symbol", name, vec![Field::text("name", "Name:", "Symbol"), Field::radio_of("kind", "Typ:", &["Grafik", "Animation", "Schaltfläche"], vec![json!("graphic"), json!("animation"), json!("button")], 0)]).with_width(420.0),
        ),
        "panel.library.duplicate" | "panel.library.edit" | "panel.library.nineSlice" | "panel.library.lockNineSlice" | "panel.library.delete" | "panel.library.properties" | "panel.library.saveToCommon" => {
            let Some(i) = sel.symbol else {
                st.push(Action::Status("Bitte zuerst ein Symbol in der Bibliothek auswählen.".into()));
                return true;
            };
            let sym = st.session.active().and_then(|d| d.doc.symbols.get(i).cloned());
            let Some(sym) = sym else { return true };
            match name {
                "panel.library.delete" => {
                    st.dialogs.confirm = Some(super::Confirm { title: "Symbol löschen".into(), text: format!("Das Symbol „{}“ und alle Instanzen löschen?", sym.name), command: name.into(), params: json!({ "index": i }) });
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
                            "Symboleigenschaften",
                            name,
                            vec![
                                Field::text("name", "Name:", &sym.name),
                                Field::radio_of("kind", "Typ:", &["Grafik", "Animation", "Schaltfläche"], vec![json!("graphic"), json!("animation"), json!("button")], k),
                                Field::check("nineSlice", "Hilfslinien für 9-Slice-Skalierung aktivieren", sym.nine_slice.is_some()),
                            ],
                        )
                        .with_base(json!({ "index": i }))
                        .with_width(420.0),
                    )
                }
                "panel.library.saveToCommon" => {
                    let dir = crate::panels::library::library_roots().into_iter().next().map(|(_, p)| p).unwrap_or_default();
                    Some(
                        FormDialog::new("6556", "In allgemeiner Bibliothek speichern", name, vec![Field::text("_file", "Dateiname:", &format!("{}.png", sym.name))])
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
                    st.push(Action::Status("Alle Symbole werden verwendet.".into()));
                }
            }
            return true;
        }
        "panel.library.import" => {
            let p = pick_open(st, "Symbole importieren", LIB_FILES);
            run_with_path(st, name, p, json!({}));
            return true;
        }
        "panel.library.export" => {
            let p = pick_save(st, "Symbole exportieren", "Symbole.png", LIB_FILES);
            run_with_path(st, name, p, json!({}));
            return true;
        }
        // Farbfelder
        "panel.swatches.add" | "panel.swatches.replace" => {
            let p = pick_open(st, "Farbfelder öffnen", SWATCH_FILES);
            run_with_path(st, name, p, json!({}));
            return true;
        }
        "panel.swatches.save" => {
            let p = pick_save(st, "Farbfelder speichern", "Farbfelder.act", SWATCH_SAVE);
            run_with_path(st, name, p, json!({}));
            return true;
        }
        // Optimieren
        "panel.optimize.saveSettings" => Some(FormDialog::new("5974", "Vorgabename", name, vec![Field::text("name", "Name:", "Eigene Einstellung")])),
        "panel.optimize.deleteSettings" => {
            let names: Vec<String> = crate::ui_commands::export_presets(&st.session.prefs).into_iter().map(|(n, _)| n).collect();
            if names.is_empty() {
                st.dialogs.message = Some(("Einstellungen löschen".into(), "Es gibt keine gespeicherten Einstellungen.".into()));
                return true;
            }
            let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
            Some(FormDialog::new("5974", "Einstellungen löschen", name, vec![Field::choice_of("name", "Vorgabe:", &refs, names.iter().map(|n| json!(n)).collect(), 0)]))
        }
        "panel.optimize.optimizeToSize" => Some(FormDialog::new("4734", "Für Dateigröße optimieren", name, vec![Field::number("kb", "Zielgröße:", 32.0, 0.1, 100000.0).decimals(1).unit("K")])),
        "panel.optimize.exportWizard" => Some(
            FormDialog::new(
                "4717",
                "Exportassistent",
                name,
                vec![
                    Field::radio_of("target", "Ziel:", &["Web", "Bildbearbeitung", "Webseite (HTML)"], vec![json!("web"), json!("imageEditing"), json!("dreamweaver")], 0),
                    Field::check("_limit", "Exportgröße festlegen", false),
                    Field::number("_kb", "Zielgröße:", 32.0, 0.1, 100000.0).decimals(1).unit("K"),
                    Field::note("Der Assistent analysiert das Bild (Fotos → JPEG, Flächen → GIF) und setzt die Exporteinstellungen der Seite."),
                ],
            )
            .with_width(480.0)
            .with_transform(wizard_params),
        ),
        "panel.optimize.editColor" => {
            let Some(c) = sel.palette.first().copied() else {
                st.push(Action::Status("Bitte zuerst eine Farbe in der Farbtabelle auswählen.".into()));
                return true;
            };
            let hex = format!("#{:06x}ff", c & 0x00ff_ffff);
            Some(FormDialog::new("4630", "Farbe bearbeiten", name, vec![Field::color("newColor", "Neue Farbe:", &hex)]).with_base(json!({ "color": format!("#{:06x}", c & 0x00ff_ffff) })))
        }
        "panel.optimize.addColorMenu" => Some(FormDialog::new("4630", "Farbe hinzufügen", "panel.optimize.addColor", vec![Field::color("color", "Farbe:", "#808080ff")])),
        "panel.optimize.editColorMenu" => return open(st, "panel.optimize.editColor"),
        "panel.optimize.deleteColorMenu" | "panel.optimize.snapToWebMenu" | "panel.optimize.lockColorMenu" => {
            if sel.palette.is_empty() {
                st.push(Action::Status("Bitte zuerst eine Farbe in der Farbtabelle auswählen.".into()));
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
            Some(FormDialog::new("19123", "Als Stil speichern", name, vec![Field::text("name", "Name:", &format!("Stil {n}"))]))
        }
        "panel.effects.locatePlugins" => {
            let dir = match &st.file_override {
                Some(o) => o.open.first().cloned(),
                None => rfd::FileDialog::new().set_title("Zusatzmodule suchen").pick_folder(),
            };
            run_with_path(st, name, dir, json!({}));
            return true;
        }
        "panel.optimize.makeTransparent" | "panel.optimize.removeEdit" => {
            if sel.palette.is_empty() {
                st.push(Action::Status("Bitte zuerst eine Farbe in der Farbtabelle auswählen.".into()));
                return true;
            }
            st.run(name, json!({ "colors": colors_hex(&sel.palette) }));
            return true;
        }
        "panel.optimize.loadPalette" => {
            let p = pick_open(st, "Palette laden", SWATCH_FILES);
            run_with_path(st, name, p, json!({}));
            return true;
        }
        "panel.optimize.savePalette" => {
            let p = pick_save(st, "Palette speichern", "Palette.act", SWATCH_SAVE);
            run_with_path(st, name, p, json!({}));
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
