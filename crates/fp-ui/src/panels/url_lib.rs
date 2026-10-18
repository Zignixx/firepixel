//! URL panel libraries and the actions of its panel menu (TW-131).
//!
//! The model and the file formats live in [`fp_engine::url_library`]; this module keeps the
//! libraries of the panel (the default `URLs.htm` plus the ones made with "New URL Library"), the
//! selected URL, and runs the menu actions `ui.url.*` (dialogs and file dialogs included).
//! Storage: the default library in `URLs.txt` next to the preferences (as before), every other
//! library as `<name>` (HTML) in the folder `URL Libraries` there.

use crate::app_state::AppState;
use crate::dialogs::form::{Field, FormDialog};
use fp_engine::url_library::{self, DEFAULT_NAME, UrlLibrary};
use serde_json::{Value, json};
use std::path::PathBuf;

/// Libraries and selection of the URL panel.
#[derive(Clone, Debug, Default)]
pub struct UrlPanelState {
    loaded: bool,
    /// All libraries (the first is the default library).
    pub libs: Vec<UrlLibrary>,
    /// Index of the library shown in the panel.
    pub current: usize,
    /// Selected URL in the list.
    pub sel: Option<usize>,
}

fn profile_dir() -> Option<PathBuf> {
    fp_engine::Preferences::default_path().and_then(|p| p.parent().map(|d| d.to_path_buf()))
}

fn default_file() -> Option<PathBuf> {
    profile_dir().map(|d| d.join("URLs.txt"))
}

fn libs_dir() -> Option<PathBuf> {
    profile_dir().map(|d| d.join("URL Libraries"))
}

impl UrlPanelState {
    /// Loads the libraries from the profile on first use.
    pub fn ensure_loaded(&mut self) {
        if self.loaded {
            return;
        }
        self.loaded = true;
        let mut def = UrlLibrary::new(DEFAULT_NAME);
        if let Some(text) = default_file().and_then(|p| std::fs::read_to_string(p).ok()) {
            for l in text.lines() {
                def.add(l);
            }
        }
        self.libs = vec![def];
        if let Some(rd) = libs_dir().and_then(|d| std::fs::read_dir(d).ok()) {
            let mut files: Vec<PathBuf> = rd
                .flatten()
                .map(|e| e.path())
                .filter(|p| url_library::is_html_path(p))
                .collect();
            files.sort();
            for p in files {
                let Some(name) = p.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                if name.eq_ignore_ascii_case(DEFAULT_NAME) {
                    continue;
                }
                let mut l = UrlLibrary::new(name);
                let _ = l.import_file(&p);
                self.libs.push(l);
            }
        }
        self.current = 0;
        self.sel = None;
    }

    /// The library shown in the panel.
    pub fn lib(&self) -> Option<&UrlLibrary> {
        self.libs.get(self.current)
    }

    fn lib_mut(&mut self) -> &mut UrlLibrary {
        self.ensure_loaded();
        if self.current >= self.libs.len() {
            self.current = 0;
        }
        if self.libs.is_empty() {
            self.libs.push(UrlLibrary::new(DEFAULT_NAME));
        }
        &mut self.libs[self.current]
    }

    /// Selected URL (index valid in the current library).
    pub fn selected(&self) -> Option<usize> {
        self.sel
            .filter(|i| self.lib().is_some_and(|l| *i < l.urls.len()))
    }

    /// Writes the current library to the profile.
    fn save_current(&self) {
        let Some(l) = self.lib() else { return };
        let target = if self.current == 0 {
            default_file().map(|p| (p, l.to_txt()))
        } else {
            libs_dir().map(|d| (d.join(&l.name), l.to_htm()))
        };
        if let Some((p, text)) = target {
            fp_i18n::profile::guard_write(&p);
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(p, text);
        }
    }

    /// Adds `url` to the current library (and saves); `true` if added.
    pub fn add(&mut self, url: &str) -> bool {
        let ok = self.lib_mut().add(url);
        if ok {
            self.save_current();
        }
        ok
    }

    /// Deletes the URL at `index` of the current library.
    pub fn delete(&mut self, index: usize) -> bool {
        let ok = self.lib_mut().remove(index);
        if ok {
            self.sel = None;
            self.save_current();
        }
        ok
    }

    /// Switches to the library `index`.
    pub fn set_current(&mut self, index: usize) {
        self.ensure_loaded();
        if index < self.libs.len() && index != self.current {
            self.current = index;
            self.sel = None;
        }
    }
}

/// URLs used in the open documents.
fn used(st: &AppState) -> Vec<String> {
    url_library::used_urls(st.session.documents().iter().map(|d| d.doc.as_ref()))
}

const LIST_FILES: &[(&str, &[&str])] = &[
    ("dlg-filter-url-list", &["htm", "html", "txt"]),
    ("dlg-filter-url-htm", &["htm", "html"]),
    ("dlg-filter-url-txt", &["txt"]),
];
const SAVE_FILES: &[(&str, &[&str])] = &[
    ("dlg-filter-url-htm", &["htm"]),
    ("dlg-filter-url-txt", &["txt"]),
];

fn str_param<'a>(p: &'a Value, k: &str) -> Option<&'a str> {
    p.get(k).and_then(|v| v.as_str())
}

/// Runs a URL panel action (`ui.url.*`); `true` if `name` was one.
pub fn run(st: &mut AppState, name: &str, params: &Value) -> bool {
    let Some(action) = name.strip_prefix("ui.url.") else {
        return false;
    };
    let u = &mut st.dialogs.url;
    u.ensure_loaded();
    match action {
        "addUsed" => {
            let used = used(st);
            let u = &mut st.dialogs.url;
            if u.lib_mut().add_used(&used) > 0 {
                u.save_current();
            }
        }
        "deleteUnused" => {
            let used = used(st);
            let u = &mut st.dialogs.url;
            if u.lib_mut().delete_unused(&used) > 0 {
                u.sel = None;
                u.save_current();
            }
        }
        "add" => match str_param(params, "url") {
            Some(v) => {
                u.add(v);
            }
            None => {
                st.dialogs.form = Some(
                    FormDialog::new(
                        "url.add",
                        &fp_i18n::t("dlg-url-add-title"),
                        name,
                        vec![Field::text("url", &fp_i18n::t("dlg-url-lbl-url"), "")],
                    )
                    .with_width(360.0),
                )
            }
        },
        "edit" => {
            let index = params
                .get("index")
                .and_then(|v| v.as_u64())
                .map(|i| i as usize)
                .or(u.selected());
            let Some(i) = index else { return true };
            match str_param(params, "url") {
                Some(v) => {
                    if u.lib_mut().edit(i, v) {
                        u.save_current();
                    }
                }
                None => {
                    let cur = u
                        .lib()
                        .and_then(|l| l.urls.get(i).cloned())
                        .unwrap_or_default();
                    st.dialogs.form = Some(
                        FormDialog::new(
                            "url.edit",
                            &fp_i18n::t("dlg-url-edit-title"),
                            name,
                            vec![Field::text("url", &fp_i18n::t("dlg-url-lbl-url"), &cur)],
                        )
                        .with_base(json!({ "index": i }))
                        .with_width(360.0),
                    )
                }
            }
        }
        "delete" => {
            let index = params
                .get("index")
                .and_then(|v| v.as_u64())
                .map(|i| i as usize)
                .or(u.selected());
            if let Some(i) = index {
                u.delete(i);
            }
        }
        "newLibrary" => match str_param(params, "name") {
            Some(n) => new_library(u, n),
            None => {
                st.dialogs.form = Some(
                    FormDialog::new(
                        "url.newLibrary",
                        &fp_i18n::t("dlg-url-new-library-title"),
                        name,
                        vec![Field::text(
                            "name",
                            &fp_i18n::t("dlg-url-lbl-library-name"),
                            &fp_i18n::t("dlg-url-default-library"),
                        )],
                    )
                    .with_width(360.0),
                )
            }
        },
        "library" => {
            if let Some(i) = params.get("index").and_then(|v| v.as_u64()) {
                u.set_current(i as usize);
            }
        }
        "import" => match str_param(params, "path") {
            Some(p) => import(u, std::path::Path::new(p)),
            None => crate::dialogs::panel_menus::pick_open(
                st,
                &fp_i18n::t("dlg-url-import-title"),
                LIST_FILES,
                |st, p| import(&mut st.dialogs.url, &p),
            ),
        },
        "export" => match str_param(params, "path") {
            Some(p) => {
                if let Some(l) = u.lib() {
                    let _ = l.export_file(std::path::Path::new(p));
                }
            }
            None => {
                let file = u.lib().map(|l| l.name.clone()).unwrap_or_default();
                crate::dialogs::panel_menus::pick_save(
                    st,
                    &fp_i18n::t("dlg-url-export-title"),
                    &file,
                    SAVE_FILES,
                    |st, p| {
                        if let Some(l) = st.dialogs.url.lib() {
                            let _ = l.export_file(&p);
                        }
                    },
                )
            }
        },
        _ => {}
    }
    true
}

fn import(u: &mut UrlPanelState, p: &std::path::Path) {
    if u.lib_mut().import_file(p).unwrap_or(0) > 0 {
        u.save_current();
    }
}

fn new_library(u: &mut UrlPanelState, name: &str) {
    let n = name
        .trim()
        .replace(['\\', '/', ':', '*', '?', '"', '<', '>', '|'], "_");
    if n.is_empty() {
        return;
    }
    let n = if url_library::is_html_path(std::path::Path::new(&n)) {
        n
    } else {
        format!("{n}.htm")
    };
    if let Some(i) = u.libs.iter().position(|l| l.name.eq_ignore_ascii_case(&n)) {
        u.set_current(i);
        return;
    }
    u.libs.push(UrlLibrary::new(n));
    u.current = u.libs.len() - 1;
    u.sel = None;
    u.save_current();
}

/// Entries of the panel menu (≡) before "Help": (text key, action, enabled), `None` = separator.
pub fn menu_entries(st: &AppState) -> Vec<Option<(&'static str, &'static str, bool)>> {
    let has_sel = st.dialogs.url.selected().is_some();
    vec![
        Some(("pmenu-url-addUsed", "ui.url.addUsed", true)),
        Some(("pmenu-url-deleteUnused", "ui.url.deleteUnused", true)),
        None,
        Some(("pmenu-url-add", "ui.url.add", true)),
        Some(("pmenu-url-edit", "ui.url.edit", has_sel)),
        Some(("pmenu-url-delete", "ui.url.delete", has_sel)),
        None,
        Some(("pmenu-url-newLibrary", "ui.url.newLibrary", true)),
        Some(("pmenu-url-import", "ui.url.import", true)),
        Some(("pmenu-url-export", "ui.url.export", true)),
    ]
}
