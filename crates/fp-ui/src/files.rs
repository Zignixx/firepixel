//! Datei-Grundfunktionen der Oberfläche: Dateidialoge (über [crate::file_dialog], ohne Blockieren),
//! Speichern mit „Speichern unter“-Rückfall, Drag & Drop.

use crate::app_state::{Action, AppState};
use crate::file_dialog::{self, Request};
use serde_json::json;
use std::path::{Path, PathBuf};

/// Alle Endungen, die fp-io öffnen bzw. importieren kann (ohne Punkt).
pub fn readable_extensions() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = Vec::new();
    for r in fp_io::detect::REGISTRY.iter().filter(|r| r.in_file_dialog) {
        for e in r.extensions {
            let e = e.trim_start_matches('.');
            if !v.contains(&e) {
                v.push(e);
            }
        }
    }
    v
}

/// Reihenfolge der Einzelformate in „Öffnen“/„Importieren“ wie im Original gemessen (DLG-3,
/// 12.10.: FW-PNG, PSD, GIF, JPEG, WBMP, TIFF, Targa, BMP, Illustrator, EPS; RTF/ASCII-Text liest
/// Firepixel nicht), danach die übrigen Formate von fp-io in dessen Reihenfolge.
const OPEN_ORDER: &[&str] = &["PNG", "PS30", "kMoaCfFormat_GIF", "kMoaCfFormat_JPEG", "kMoaCfFormat_WBMP", "kMoaCfFormat_TIFF", "kMoaCfFormat_TGA", "kMoaCfFormat_BMP", "AI3", "EPSF"];

/// Filter von „Öffnen“ und „Importieren“ wie im Original: die Einzelformate, dann „Alle lesbaren
/// Dateien“ (vorgewählt) und „Alle Dateien“.
fn readable_filters(mut r: Request) -> Request {
    let formats: Vec<&fp_io::detect::ReaderInfo> = fp_io::detect::REGISTRY.iter().filter(|f| f.in_file_dialog).collect();
    let mut ordered: Vec<&fp_io::detect::ReaderInfo> = OPEN_ORDER.iter().filter_map(|id| formats.iter().copied().find(|f| f.id == *id)).collect();
    for f in &formats {
        if !ordered.iter().any(|o| o.id == f.id) {
            ordered.push(f);
        }
    }
    for f in ordered {
        r = r.filter(format!("{} ({})", f.name, f.pattern), f.extensions);
    }
    r = r.filter(fp_i18n::t("ui-filter-all-readable"), &readable_extensions());
    let all_readable = r.filters.len() - 1;
    r.default_filter(all_readable).filter(fp_i18n::t("ui-filter-all-files"), &["*"])
}

/// Art eines Datei-Dialogs der Oberfläche: bestimmt Startordner, Titel, Filter und den
/// Voreinstellungsschlüssel, unter dem der Ordner gemerkt wird (Original: `Last…Location`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialogKind {
    /// Datei ▸ Öffnen (D03)
    Open,
    /// Datei ▸ Neu von Vorlage (D02)
    NewFromTemplate,
    /// Datei ▸ Speichern unter (D04)
    SaveAs,
    /// Datei ▸ Als Vorlage speichern (D05)
    SaveAsTemplate,
    /// Datei ▸ Importieren (D06)
    Import,
    /// Datei ▸ HTML aktualisieren (D09)
    UpdateHtml,
}

impl DialogKind {
    /// Voreinstellung, unter der der zuletzt benutzte Ordner steht.
    pub fn pref_key(self) -> Option<&'static str> {
        match self {
            DialogKind::Open => Some("LastOpenLocation"),
            DialogKind::SaveAs => Some("LastSaveLocation"),
            DialogKind::Import => Some("LastImportLocation"),
            _ => None,
        }
    }

    /// Fenstertitel wie im Original (D05 hat dort den Standardtitel „Speichern unter“, D09
    /// „HTML-Datei suchen:“).
    pub fn title(self) -> String {
        fp_i18n::t(match self {
            DialogKind::Open => "ui-file-open-title",
            DialogKind::NewFromTemplate => "ui-file-new-from-template-title",
            DialogKind::SaveAs | DialogKind::SaveAsTemplate => "ui-file-save-as-title",
            DialogKind::Import => "ui-file-import-title",
            DialogKind::UpdateHtml => "ui-file-update-html-title",
        })
    }

    /// Dialog mit Titel, Filtern und Mehrfachauswahl wie im Original (ohne Startordner).
    pub fn request(self, file_name: &str) -> Request {
        match self {
            DialogKind::Open => readable_filters(Request::open_many(self.title())),
            DialogKind::Import => readable_filters(Request::open(self.title())),
            DialogKind::NewFromTemplate => Request::open(self.title()).filter(fp_i18n::t("ui-filter-fw-png"), &["png"]),
            DialogKind::SaveAsTemplate => Request::save(self.title(), file_name).filter(fp_i18n::t("ui-filter-fw-png"), &["png"]),
            DialogKind::SaveAs => save_type_filters(Request::save(self.title(), file_name)).save_extras(save_as_extras()),
            DialogKind::UpdateHtml => Request::open(self.title()).filter(fp_i18n::t("ui-filter-html"), &["htm", "html"]).filter(fp_i18n::t("ui-filter-all-files"), &["*"]),
        }
    }
}

/// Ordner „Dokumente“ des Nutzers (Rückfall aller Datei-Dialoge), sonst sein Home-Ordner.
/// Über den Profil-Wächter: im Testlauf liegt er im Testprofil, nie im echten Profil (NZ-5).
pub fn documents_dir() -> Option<PathBuf> {
    // nur gelesen, nie beschrieben (Startordner des Dialogs)
    fp_i18n::profile::known_folder(fp_i18n::profile::KnownFolder::Documents)
}

/// Vorlagenordner (`%APPDATA%\Firepixel\Templates`, im Testlauf im Testprofil).
pub fn templates_dir() -> Option<PathBuf> {
    fp_i18n::profile::roaming_dir().map(|d| d.join("Templates"))
}

/// Gemerkter Ordner einer Voreinstellung, wenn es ihn noch gibt.
fn remembered_dir(prefs: &fp_engine::Preferences, key: &str) -> Option<PathBuf> {
    let v = prefs.get_str(key, "");
    let v = v.trim();
    let p = PathBuf::from(v);
    (!v.is_empty() && p.is_dir()).then_some(p)
}

/// Startordner wie im Original (gemessen DLG-3, 12.10.):
/// - Öffnen und HTML aktualisieren: zuletzt beim Öffnen benutzter Ordner (`LastOpenLocation`),
///   sonst der Ordner des gespeicherten Dokuments;
/// - Importieren: `LastImportLocation`, sonst `LastOpenLocation`, sonst Ordner des Dokuments;
/// - Speichern unter: Ordner des gespeicherten Dokuments, sonst `LastSaveLocation`;
/// - Neu von Vorlage, Als Vorlage speichern: Vorlagenordner;
/// - sonst „Dokumente“.
pub fn start_dir(prefs: &fp_engine::Preferences, kind: DialogKind, doc_path: Option<&Path>) -> Option<PathBuf> {
    let doc_dir = || doc_path.and_then(Path::parent).filter(|d| d.is_dir()).map(Path::to_path_buf);
    let last = |k: &str| remembered_dir(prefs, k);
    let dir = match kind {
        DialogKind::Open | DialogKind::UpdateHtml => last("LastOpenLocation").or_else(doc_dir),
        DialogKind::Import => last("LastImportLocation").or_else(|| last("LastOpenLocation")).or_else(doc_dir),
        DialogKind::SaveAs => doc_dir().or_else(|| last("LastSaveLocation")),
        DialogKind::NewFromTemplate | DialogKind::SaveAsTemplate => templates_dir().filter(|d| d.is_dir()),
    };
    dir.or_else(documents_dir)
}

/// Merkt den Ordner einer bestätigten Wahl unter dem Schlüssel der Art (Datei → ihr Ordner).
/// Gibt `true` zurück, wenn sich etwas geändert hat.
pub fn remember_dir(prefs: &mut fp_engine::Preferences, kind: DialogKind, chosen: &Path) -> bool {
    let Some(key) = kind.pref_key() else { return false };
    let dir = if chosen.is_dir() { Some(chosen) } else { chosen.parent() };
    let Some(dir) = dir.filter(|d| !d.as_os_str().is_empty()) else { return false };
    let v = dir.display().to_string();
    if prefs.get_str(key, "") == v {
        return false;
    }
    prefs.set(key, fp_engine::prefs::PrefValue::Str(v));
    true
}

/// Datei-Dialog der Art `kind` für das aktive Dokument: Startordner, Titel und Filter wie im
/// Original; nach einer bestätigten Wahl wird der Ordner gemerkt. Ergebnis leer = abgebrochen.
pub fn pick(st: &mut AppState, kind: DialogKind, file_name: &str, done: impl FnOnce(&mut AppState, Vec<PathBuf>) + Send + 'static) {
    if matches!(kind, DialogKind::NewFromTemplate | DialogKind::SaveAsTemplate)
        && let Some(t) = templates_dir()
    {
        fp_i18n::profile::guard_write(&t);
        let _ = std::fs::create_dir_all(&t);
    }
    let doc_path = st.session.active().and_then(|d| d.path.clone());
    let dir = start_dir(&st.session.prefs, kind, doc_path.as_deref());
    let req = kind.request(file_name).dir(dir.as_deref());
    file_dialog::request(st, req, move |st, r| {
        if let Some(first) = r.first()
            && remember_dir(&mut st.session.prefs, kind, first)
        {
            st.session.prefs_changed();
        }
        done(st, r);
    });
}

/// Wie eine Datei unter einem Dateityp von „Speichern unter“ geschrieben wird.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveKind {
    /// FW-PNG mit allen Ebenen (das Dokument trägt danach diesen Namen).
    FwPng,
    /// „Reduziertes PNG“ (re\20 §5): aktuelle Seite flach mit den Optimierungseinstellungen des
    /// Dokuments, Format PNG. Ohne „Als Kopie speichern“ gehört das Dokument danach zur PNG-Datei.
    FlatPng,
    /// Reduziertes Bild über den Export (`file.export` mit Format), als Kopie.
    Export(&'static str),
    /// Export-Art (`file.export` mit `style`), als Kopie.
    Style(&'static str),
}

/// Dateitypen von „Speichern unter“: die Typen des Originals, soweit Firepixel sie schreibt, in
/// dessen Reihenfolge, und die modernen Formate (Beschriftung, Endungen, Art). Übersetzte
/// Beschriftungen (leer hier) liefert [`save_type_label`].
pub const SAVE_TYPES: &[(&str, &[&str], SaveKind)] = &[
    ("FW-PNG (*.png)", &["png"], SaveKind::FwPng),
    ("BMP (*.bmp)", &["bmp"], SaveKind::Export("bmp")),
    ("GIF (*.gif)", &["gif"], SaveKind::Export("gif")),
    ("JPEG (*.jpg; *.jpeg; *.jpe)", &["jpg", "jpeg", "jpe"], SaveKind::Export("jpeg")),
    ("", &["png"], SaveKind::FlatPng),
    ("Photoshop PSD (*.psd)", &["psd"], SaveKind::Style("psd")),
    ("TIFF (*.tif; *.tiff)", &["tif", "tiff"], SaveKind::Export("tiff")),
    ("WBMP (*.wbmp)", &["wbmp"], SaveKind::Export("wbmp")),
    ("WebP (*.webp)", &["webp"], SaveKind::Export("webp")),
    ("AVIF (*.avif)", &["avif"], SaveKind::Export("avif")),
    ("QOI (*.qoi)", &["qoi"], SaveKind::Export("qoi")),
    ("ICO (*.ico)", &["ico"], SaveKind::Export("ico")),
    ("SVG (*.svg)", &["svg"], SaveKind::Style("svg")),
    ("PDF (*.pdf)", &["pdf"], SaveKind::Style("pdf")),
];

/// Art zum gewählten Dateinamen (über die Endung; `.png` = FW-PNG).
pub fn save_kind(p: &Path) -> SaveKind {
    let n = p.to_string_lossy().to_lowercase();
    SAVE_TYPES.iter().find(|(_, exts, _)| exts.iter().any(|e| n.ends_with(&format!(".{e}")))).map(|t| t.2).unwrap_or(SaveKind::FwPng)
}

/// Beschriftung eines Dateityps von „Speichern unter“ in der Sprache der Oberfläche.
pub fn save_type_label(i: usize) -> String {
    match SAVE_TYPES.get(i) {
        Some((_, _, SaveKind::FlatPng)) => fp_i18n::t("ui-filter-flattened-png"),
        Some((label, _, _)) => label.to_string(),
        None => String::new(),
    }
}

/// Filter von „Speichern unter“ (alle Dateitypen in [`SAVE_TYPES`]-Reihenfolge).
fn save_type_filters(mut r: Request) -> Request {
    for (i, (_, exts, _)) in SAVE_TYPES.iter().enumerate() {
        r = r.filter(save_type_label(i), exts);
    }
    r
}

/// Index eines Dateityps in [`SAVE_TYPES`].
pub fn save_type_index(kind: SaveKind) -> Option<usize> {
    SAVE_TYPES.iter().position(|t| t.2 == kind)
}

/// Art zum gewählten Namen und Dateityp (wie im Original entscheidet der gewählte Typ, nicht nur die
/// Endung: `.png` unter „Reduziertes PNG“ ist flach, unter „FW-PNG“ nicht). Passt der Name nicht
/// zum Typ (andere Endung), entscheidet die Endung; ohne Endung der Typ.
pub fn save_kind_for(p: &Path, filter: Option<usize>) -> SaveKind {
    let n = p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    if let Some((_, exts, kind)) = filter.and_then(|i| SAVE_TYPES.get(i))
        && (exts.iter().any(|e| n.ends_with(&format!(".{e}"))) || p.extension().is_none())
    {
        return *kind;
    }
    save_kind(p)
}

/// Gewählter Name → Ziel für eine Art: FW-PNG bekommt `.fw.png`, Reduziertes PNG endet auf `.png`
/// (ohne `.fw.png`), andere Typen bekommen ohne Endung ihre erste Endung.
pub fn save_target_for(p: PathBuf, kind: SaveKind) -> PathBuf {
    save_target_for_ex(p, kind, true)
}

/// Wie [`save_target_for`]; `append_fw_png` = Kästchen „.fw.png anhängen“. Aus: FW-PNG behält den
/// gewählten Namen (ohne Endung bekommt er `.png`).
pub fn save_target_for_ex(p: PathBuf, kind: SaveKind, append_fw_png: bool) -> PathBuf {
    if kind == SaveKind::FwPng && !append_fw_png {
        return if p.extension().is_some() { p } else { p.with_extension("png") };
    }
    let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let lower = name.to_lowercase();
    match kind {
        SaveKind::FwPng => ensure_fw_png(p),
        SaveKind::FlatPng if lower.ends_with(".fw.png") => p.with_file_name(format!("{}.png", &name[..name.len() - ".fw.png".len()])),
        SaveKind::FlatPng if lower.ends_with(".png") => p,
        SaveKind::FlatPng => p.with_file_name(format!("{name}.png")),
        _ if p.extension().is_some() => p,
        _ => match SAVE_TYPES.iter().find(|t| t.2 == kind).and_then(|t| t.1.first()) {
            Some(e) => p.with_file_name(format!("{name}.{e}")),
            None => p,
        },
    }
}

/// Typen, zwischen denen nach dem Windows-Dialog nachgefragt wird: ein Name auf `.png` (nicht
/// `.fw.png`) passt zu FW-PNG und Reduziertes PNG.
fn ambiguous_save_types(p: &Path) -> Vec<usize> {
    let n = p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    if n.ends_with(".png") && !n.ends_with(".fw.png") {
        [SaveKind::FwPng, SaveKind::FlatPng].into_iter().filter_map(save_type_index).collect()
    } else {
        Vec::new()
    }
}

/// Zusatzfelder je Dateityp wie im Original (re\20 §1.1): FW-PNG „Als Kopie speichern“ wählbar;
/// Reduziertes PNG zusätzlich Hinweis und „Optionen…“ (Bildvorschau); alle anderen Typen sind
/// immer Kopien (Kästchen an und gesperrt) und reduzieren ebenfalls (Hinweis).
pub fn save_type_extras(kind: SaveKind) -> file_dialog::TypeExtras {
    use file_dialog::{CopyBox, TypeExtras};
    match kind {
        SaveKind::FwPng => TypeExtras { copy: CopyBox::Off, options: false, hint: false, append_fw: true },
        SaveKind::FlatPng => TypeExtras { copy: CopyBox::Off, options: true, hint: true, append_fw: false },
        _ => TypeExtras { copy: CopyBox::Locked, options: false, hint: true, append_fw: false },
    }
}

/// „Optionen…“ bei Reduziertes PNG: Bildvorschau mit den Optimierungseinstellungen des Dokuments.
fn open_save_options(st: &mut AppState, i: usize) {
    if SAVE_TYPES.get(i).map(|t| t.2) == Some(SaveKind::FlatPng) {
        st.dialogs.preview = crate::dialogs::image_preview::ImagePreview::new(st);
    }
}

fn save_options_open(st: &AppState) -> bool {
    st.dialogs.preview.is_some()
}

/// Zusatzfelder des Dialogs „Speichern unter“.
pub fn save_as_extras() -> file_dialog::SaveExtras {
    file_dialog::SaveExtras {
        types: SAVE_TYPES.iter().map(|t| save_type_extras(t.2)).collect(),
        ambiguous: ambiguous_save_types,
        on_options: open_save_options,
        options_open: save_options_open,
    }
}

/// Vorgegebene Ergebnisse statt nativer Dateidialoge (Tests, Automatisierung).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FilePickOverride {
    /// Ergebnis von „Öffnen“ (leer = abgebrochen).
    pub open: Vec<PathBuf>,
    /// Ergebnis von „Speichern unter“ (None = abgebrochen).
    pub save: Option<PathBuf>,
}

/// Dateien zum Öffnen wählen (Mehrfachauswahl, ohne Blockieren; mit [`FilePickOverride`] sofort).
pub fn pick_open(st: &mut AppState, dir: Option<&Path>, done: impl FnOnce(&mut AppState, Vec<PathBuf>) + Send + 'static) {
    let req = readable_filters(Request::open_many(fp_i18n::t("ui-file-open-title"))).dir(dir);
    file_dialog::request(st, req, done);
}

/// Eine Datei mit bestimmten Endungen wählen (Importieren, Skript, HTML); mit [`FilePickOverride`]
/// wird die erste passende vorgegebene Datei genommen.
pub fn pick_one(st: &mut AppState, title: &str, filter: &str, exts: &[&str], done: impl FnOnce(&mut AppState, PathBuf) + Send + 'static) {
    let owned: Vec<String> = exts.iter().map(|e| e.to_lowercase()).collect();
    let matches = move |p: &PathBuf| {
        let n = p.to_string_lossy().to_lowercase();
        owned.iter().any(|e| e == "*" || n.ends_with(&format!(".{e}")))
    };
    if let Some(o) = &st.file_override {
        if let Some(p) = o.open.iter().find(|p| matches(p)).cloned() {
            done(st, p);
        }
        return;
    }
    let dir = st.session.active().and_then(|d| d.path.as_ref()).and_then(|p| p.parent().map(Path::to_path_buf));
    let req = Request::open(title).filter(filter, exts).filter(fp_i18n::t("ui-filter-all-files"), &["*"]).dir(dir.as_deref());
    file_dialog::request(st, req, move |st, r| {
        if let Some(p) = r.into_iter().next() {
            done(st, p);
        }
    });
}

/// Speicherziel wählen (Dateitypen von „Speichern unter“, ohne Blockieren; mit [`FilePickOverride`]
/// sofort). `done` läuft nur, wenn ein Ziel gewählt wurde; `cancelled` sonst.
pub fn pick_save(st: &mut AppState, title: &str, default_name: &str, dir: Option<&Path>, done: impl FnOnce(&mut AppState, Option<PathBuf>) + Send + 'static) {
    if let Some(o) = &st.file_override {
        let p = o.save.clone().map(ensure_fw_png);
        done(st, p);
        return;
    }
    let req = save_type_filters(Request::save(title, default_name).dir(dir));
    file_dialog::request(st, req, move |st, r| {
        let p = r.into_iter().next().map(save_target);
        done(st, p);
    });
}

/// Gewählter Speichername → Ziel: FW-PNG bekommt `.fw.png` (Kästchen „.fw.png anhängen“ des
/// Originals, Vorgabe an), andere Dateitypen bleiben.
pub fn save_target(p: PathBuf) -> PathBuf {
    if save_kind(&p) == SaveKind::FwPng { ensure_fw_png(p) } else { p }
}
/// Hängt `.fw.png` an (wie das Kästchen „.fw.png anhängen“ des Originals, Vorgabe an): aus
/// `name` und `name.png` wird `name.fw.png`.
pub fn ensure_fw_png(p: PathBuf) -> PathBuf {
    let name = p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    if name.ends_with(".fw.png") {
        p
    } else if name.ends_with(".png") {
        p.with_extension("fw.png")
    } else {
        let mut s = p.into_os_string();
        s.push(".fw.png");
        PathBuf::from(s)
    }
}

/// Vorgeschlagener Dateiname zum Speichern („Unbenannt-1.png“ → „Unbenannt-1.fw.png“).
pub fn suggested_name(title: &str) -> String {
    let base = title.trim_start_matches('*');
    if base.to_lowercase().ends_with(".fw.png") {
        base.to_string()
    } else if let Some(stem) = base.strip_suffix(".png") {
        format!("{stem}.fw.png")
    } else {
        format!("{base}.fw.png")
    }
}

/// Speichert das aktive Dokument; ohne Dateinamen mit „Speichern unter“ (ohne Blockieren). `done`
/// bekommt `true`, wenn gespeichert wurde.
pub fn save_active(st: &mut AppState, force_dialog: bool, done: impl FnOnce(&mut AppState, bool) + Send + 'static) {
    let Some(d) = st.session.active() else {
        done(st, false);
        return;
    };
    let has_path = d.path.is_some();
    let title = d.title.clone();
    let id = d.id;
    if has_path && !force_dialog {
        let ok = write_active(st, None, SaveKind::FwPng, false);
        done(st, ok);
        return;
    }
    pick_save_as(st, &suggested_name(&title), move |st, choice| {
        // das Dokument kann während des Dialogs gewechselt haben: wieder aktivieren
        let ok = match choice {
            Some(c) if st.session.activate(id).is_ok() => save_chosen(st, c),
            _ => false,
        };
        done(st, ok);
    });
}

/// Dialog „Speichern unter“ mit Typliste und Zusatzfeldern (Startordner und Merken wie [`pick`]).
pub fn pick_save_as(st: &mut AppState, file_name: &str, done: impl FnOnce(&mut AppState, Option<file_dialog::SaveChoice>) + Send + 'static) {
    let kind = DialogKind::SaveAs;
    let doc_path = st.session.active().and_then(|d| d.path.clone());
    let dir = start_dir(&st.session.prefs, kind, doc_path.as_deref());
    let req = kind.request(file_name).dir(dir.as_deref());
    file_dialog::request_save(st, req, move |st, c| {
        if let Some(c) = &c
            && remember_dir(&mut st.session.prefs, kind, &c.path)
        {
            st.session.prefs_changed();
        }
        done(st, c);
    });
}

/// Speichert das aktive Dokument nach der Wahl in „Speichern unter“: Art aus Name und gewähltem
/// Typ, Ziel mit passender Endung, „Als Kopie speichern“. `true` = gespeichert.
pub fn save_chosen(st: &mut AppState, c: file_dialog::SaveChoice) -> bool {
    let kind = save_kind_for(&c.path, c.filter);
    let target = save_target_for_ex(c.path, kind, c.append_fw_png);
    write_active(st, Some(target), kind, c.copy)
}

/// Schreibt das aktive Dokument (`None` = unter seinem Namen und in seiner Art).
fn write_active(st: &mut AppState, path: Option<PathBuf>, kind: SaveKind, copy: bool) -> bool {
    let failed = |st: &mut AppState, e: fp_engine::EngineError| {
        st.dialogs.message = Some(("Firepixel".into(), fp_i18n::tr!("ui-file-save-failed", error = e.to_string())));
        false
    };
    let Some(p) = path else {
        return match st.session.execute("file.save", json!({})) {
            Ok(_) => true,
            Err(e) => failed(st, e),
        };
    };
    let (name, args) = match kind {
        SaveKind::FwPng => ("file.saveAs", json!({ "path": p.to_string_lossy(), "copy": copy })),
        SaveKind::FlatPng => ("file.saveAs", json!({ "path": p.to_string_lossy(), "flattened": true, "copy": copy })),
        // anderer Dateityp: reduziert exportieren (immer Kopie, das Dokument behält Namen und Status)
        SaveKind::Export(f) => ("file.export", json!({ "path": p.to_string_lossy(), "settings": { "format": f } })),
        SaveKind::Style(s) => ("file.export", json!({ "path": p.to_string_lossy(), "style": s })),
    };
    let is_copy = copy || name == "file.export";
    match st.session.execute(name, args) {
        Ok(_) => {
            if is_copy {
                st.status = Some((fp_i18n::tr!("ui-file-saved-copy", path = p.display().to_string()), 0.0));
            }
            true
        }
        Err(e) => failed(st, e),
    }
}

/// „Alle speichern“: nacheinander jedes geänderte bzw. unbenannte Dokument (unbenannte fragen nach
/// einem Namen), danach wieder das vorher aktive; Abbrechen eines Dialogs geht zum nächsten.
pub fn save_all(st: &mut AppState, mut ids: Vec<fp_engine::DocId>, active: Option<fp_engine::DocId>) {
    while !ids.is_empty() {
        let id = ids.remove(0);
        if st.session.activate(id).is_ok() && st.session.active().is_some_and(|d| d.is_dirty() || d.path.is_none()) {
            save_active(st, false, move |st, _| save_all(st, ids, active));
            return;
        }
    }
    if let Some(a) = active {
        let _ = st.session.activate(a);
    }
}
/// Datei ▸ Öffnen: Dialog (Startordner und Merken wie [`pick`]) mit den Zusatzfeldern „Als
/// ‚Unbenannt‘ öffnen“ und „Als Animation öffnen“ (nur im Dialog in der App), dann öffnen.
pub fn pick_and_open(st: &mut AppState) {
    let kind = DialogKind::Open;
    let doc_path = st.session.active().and_then(|d| d.path.clone());
    let dir = start_dir(&st.session.prefs, kind, doc_path.as_deref());
    let req = kind.request("").dir(dir.as_deref());
    file_dialog::request_open(st, req, move |st, paths, choice| {
        if let Some(first) = paths.first()
            && remember_dir(&mut st.session.prefs, kind, first)
        {
            st.session.prefs_changed();
        }
        open_chosen(st, &paths, choice);
    });
}

/// Öffnet die gewählten Dateien nach den Kästchen von „Öffnen“: „Als Animation öffnen“ (ab zwei
/// Dateien) legt ein unbenanntes Dokument mit einem Status je Datei an, „Als ‚Unbenannt‘ öffnen“
/// öffnet jede Datei ohne Dateinamen, sonst normal.
pub fn open_chosen(st: &mut AppState, paths: &[PathBuf], choice: file_dialog::OpenChoice) {
    let failed = |st: &mut AppState, p: &Path, e: fp_engine::EngineError| {
        st.dialogs.message = Some(("Firepixel".into(), fp_i18n::tr!("ui-file-open-failed", path = p.display().to_string(), error = e.to_string())));
    };
    if choice.animation && paths.len() > 1 {
        let rest: Vec<String> = paths[1..].iter().map(|p| p.to_string_lossy().to_string()).collect();
        if let Err(e) = st.session.execute("file.open", json!({ "path": paths[0].to_string_lossy(), "frames": rest })) {
            failed(st, &paths[0], e);
        }
        return;
    }
    for p in paths {
        if choice.untitled {
            if let Err(e) = st.session.execute("file.open", json!({ "path": p.to_string_lossy(), "untitled": true })) {
                failed(st, p, e);
            }
        } else {
            open_path(st, p);
        }
    }
}

/// Öffnet eine Datei über das Befehlsregister (Fehler als Meldung).
pub fn open_path(st: &mut AppState, p: &Path) {
    // schon offen? → aktivieren
    if let Some(d) = st.session.documents().iter().find(|d| d.path.as_deref() == Some(p)) {
        let id = d.id;
        st.push(Action::Activate(id));
        return;
    }
    if let Err(e) = st.session.execute("file.open", json!({ "path": p.to_string_lossy() })) {
        st.dialogs.message = Some(("Firepixel".into(), fp_i18n::tr!("ui-file-open-failed", path = p.display().to_string(), error = e.to_string())));
    }
}

/// Auf das Fenster gezogene Dateien, deren Ablageort noch bestimmt wird.
#[derive(Clone, Debug, PartialEq)]
pub struct PendingDrop {
    pub files: Vec<PathBuf>,
    /// Zeitpunkt des Ablegens (egui-Zeit).
    pub time: f64,
    /// Zeigerposition vor dem Ablegen (winit meldet beim Ablegen keine Position; die nächste
    /// Mausbewegung nach dem Ablegen liefert sie).
    pub pointer_before: Option<egui::Pos2>,
}

/// Dateiendung, die immer als Dokument geöffnet wird (FW-PNG).
fn opens_as_document(p: &Path) -> bool {
    p.file_name().map(|n| n.to_string_lossy().to_lowercase().ends_with(".fw.png")).unwrap_or(false)
}

/// Auf das Fenster gezogene Dateien wie im Original: über der Leinwand eines offenen Dokuments
/// abgelegt, wird jede Datei (außer FW-PNG) als Objekt an der Ablagestelle (linke obere Ecke) auf
/// der aktiven Ebene importiert – alle zusammen als ein Verlaufsschritt, danach ausgewählt.
/// Außerhalb einer Leinwand (Reiterleiste, Bedienfelder, kein Dokument offen) und für FW-PNG wird
/// geöffnet.
pub fn handle_dropped_files(ctx: &egui::Context, st: &mut AppState) {
    let files: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).collect());
    let now = ctx.input(|i| i.time);
    if !files.is_empty() {
        let before = ctx.input(|i| i.pointer.latest_pos());
        match &mut st.pending_drop {
            Some(p) => p.files.extend(files),
            None => st.pending_drop = Some(PendingDrop { files, time: now, pointer_before: before }),
        }
        ctx.request_repaint();
    }
    let Some(p) = st.pending_drop.clone() else { return };
    // Ablageort: erste Zeigerposition nach dem Ablegen, spätestens nach 0,3 s die letzte bekannte
    let pos = ctx.input(|i| i.pointer.latest_pos());
    let moved = pos.is_some() && pos != p.pointer_before;
    if !moved && now - p.time < 0.3 {
        ctx.request_repaint();
        return;
    }
    st.pending_drop = None;
    drop_files_at(st, &p.files, pos);
}

/// Legt Dateien an einer Bildschirmposition ab (siehe [`handle_dropped_files`]).
pub fn drop_files_at(st: &mut AppState, files: &[PathBuf], pos: Option<egui::Pos2>) {
    // über der Leinwand des aktiven Dokuments?
    let target = st.session.active().and_then(|d| {
        let area = st.canvases.get(&d.id).and_then(|c| c.area)?;
        let p = pos.filter(|p| area.contains(*p))?;
        let q = d.view.screen_to_doc((p.x - area.min.x) as f64, (p.y - area.min.y) as f64);
        Some([q.x.round(), q.y.round()])
    });
    let (import, open): (Vec<&PathBuf>, Vec<&PathBuf>) = files.iter().partition(|f| target.is_some() && !opens_as_document(f));
    if let Some(at) = target
        && !import.is_empty()
    {
        let label = fp_i18n::t(if import.len() == 1 { "ui-history-import" } else { "ui-history-import-files" });
        let paths: Vec<String> = import.iter().map(|f| f.to_string_lossy().to_string()).collect();
        let mut ids: Vec<u64> = Vec::new();
        let r = st.session.run_group(&label, |s| {
            for p in &paths {
                s.execute("file.import", json!({ "path": p, "at": at }))?;
                if let Some(d) = s.active() {
                    ids.extend(d.selection.elements.iter().map(|e| e.0));
                }
            }
            Ok(())
        });
        match r {
            Ok(()) => {
                let _ = st.session.execute("select.set", json!({ "ids": ids }));
            }
            Err(e) => st.dialogs.message = Some(("Firepixel".into(), fp_i18n::tr!("ui-file-import-failed", error = e.to_string()))),
        }
    }
    for f in open {
        st.push(Action::OpenPath(f.clone()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(suggested_name("Unbenannt-1.png"), "Unbenannt-1.fw.png");
        assert_eq!(suggested_name("*a.fw.png"), "a.fw.png");
        assert_eq!(ensure_fw_png(PathBuf::from("x")), PathBuf::from("x.fw.png"));
        assert_eq!(ensure_fw_png(PathBuf::from("x.png")), PathBuf::from("x.fw.png"));
        assert_eq!(ensure_fw_png(PathBuf::from("x.fw.png")), PathBuf::from("x.fw.png"));
    }

    #[test]
    fn save_types_and_readable_formats() {
        assert_eq!(save_kind(Path::new("a.fw.png")), SaveKind::FwPng);
        assert_eq!(save_kind(Path::new("a.JPG")), SaveKind::Export("jpeg"));
        assert_eq!(save_kind(Path::new("a.psd")), SaveKind::Style("psd"));
        let r = readable_extensions();
        for e in ["png", "psd", "gif", "jpg", "bmp", "tif", "webp", "svg", "avif", "heic", "jxl", "qoi", "ico"] {
            assert!(r.contains(&e), "{e} fehlt im Filter „Alle lesbaren Formate“");
        }
    }
}
