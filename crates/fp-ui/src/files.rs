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

/// Filter „Alle lesbaren Formate“, danach je Format (Reihenfolge des Originals) und „Alle Dateien“.
fn readable_filters(mut r: Request) -> Request {
    r = r.filter(fp_i18n::t("ui-filter-all-readable"), &readable_extensions());
    for f in fp_io::detect::REGISTRY.iter().filter(|f| f.in_file_dialog) {
        r = r.filter(format!("{} ({})", f.name, f.pattern), f.extensions);
    }
    r.filter(fp_i18n::t("ui-filter-all-files"), &["*"])
}
/// Wie eine Datei unter einem Dateityp von „Speichern unter“ geschrieben wird.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveKind {
    /// FW-PNG mit allen Ebenen (das Dokument trägt danach diesen Namen).
    FwPng,
    /// Reduziertes Bild über den Export (`file.export` mit Format), als Kopie.
    Export(&'static str),
    /// Export-Art (`file.export` mit `style`), als Kopie.
    Style(&'static str),
}

/// Dateitypen von „Speichern unter“: die Typen des Originals, soweit Firepixel sie schreibt, und
/// die modernen Formate (Beschriftung, Endungen, Art).
pub const SAVE_TYPES: &[(&str, &[&str], SaveKind)] = &[
    ("FW-PNG (*.fw.png)", &["png"], SaveKind::FwPng),
    ("BMP (*.bmp)", &["bmp"], SaveKind::Export("bmp")),
    ("GIF (*.gif)", &["gif"], SaveKind::Export("gif")),
    ("JPEG (*.jpg)", &["jpg", "jpeg"], SaveKind::Export("jpeg")),
    ("Photoshop PSD (*.psd)", &["psd"], SaveKind::Style("psd")),
    ("TIFF (*.tif)", &["tif", "tiff"], SaveKind::Export("tiff")),
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
    let mut req = Request::save(title, default_name).dir(dir);
    for (label, exts, _) in SAVE_TYPES {
        req = req.filter(*label, exts);
    }
    file_dialog::request(st, req, move |st, r| {
        let p = r.into_iter().next().map(|p| if save_kind(&p) == SaveKind::FwPng { ensure_fw_png(p) } else { p });
        done(st, p);
    });
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
    let dir = d.path.as_ref().and_then(|p| p.parent().map(Path::to_path_buf));
    if has_path && !force_dialog {
        let ok = write_active(st, None);
        done(st, ok);
        return;
    }
    pick_save(st, &fp_i18n::t("ui-file-save-as-title"), &suggested_name(&title), dir.as_deref(), move |st, p| {
        // das Dokument kann während des Dialogs gewechselt haben: wieder aktivieren
        let ok = match p {
            Some(p) if st.session.activate(id).is_ok() => write_active(st, Some(p)),
            _ => false,
        };
        done(st, ok);
    });
}

/// Schreibt das aktive Dokument (`None` = unter seinem Namen).
fn write_active(st: &mut AppState, path: Option<PathBuf>) -> bool {
    // anderer Dateityp: reduziert exportieren (Kopie, das Dokument behält Namen und Status)
    if let Some(p) = &path
        && save_kind(p) != SaveKind::FwPng
    {
        let args = match save_kind(p) {
            SaveKind::Export(f) => json!({ "path": p.to_string_lossy(), "settings": { "format": f } }),
            SaveKind::Style(s) => json!({ "path": p.to_string_lossy(), "style": s }),
            SaveKind::FwPng => json!({}),
        };
        return match st.session.execute("file.export", args) {
            Ok(_) => {
                st.status = Some((fp_i18n::tr!("ui-file-saved-copy", path = p.display().to_string()), 0.0));
                true
            }
            Err(e) => {
                st.dialogs.message = Some(("Firepixel".into(), fp_i18n::tr!("ui-file-save-failed", error = e.to_string())));
                false
            }
        };
    }
    let (name, params) = match &path {
        Some(p) => ("file.saveAs", json!({ "path": p.to_string_lossy() })),
        None => ("file.save", json!({})),
    };
    match st.session.execute(name, params) {
        Ok(_) => true,
        Err(e) => {
            st.dialogs.message = Some(("Firepixel".into(), fp_i18n::tr!("ui-file-save-failed", error = e.to_string())));
            false
        }
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
