//! `Files` und FileReference (Dateizugriff über `file://`-URLs).

use super::ni;
use crate::bindings::hr;
use crate::conv::{self, Arg};
use crate::runtime;
use crate::state::{with_state, FileRefState, FwErr, FwResult, Handle};
use crate::urls::{path_to_url, url_to_path};
use boa_engine::{Context, JsValue};
use std::path::{Path, PathBuf};

fn p(a: Option<&Arg>) -> PathBuf {
    url_to_path(&a.and_then(|x| x.str()).unwrap_or_default())
}

fn set_err(msg: String) {
    let _ = with_state(|s| s.last_file_error = msg);
}

fn is_url(a: Option<&Arg>) -> bool {
    a.and_then(|x| x.str()).is_some_and(|s| s.to_ascii_lowercase().starts_with("file:"))
}

/// Ausgabe wie die Eingabe: URL bleibt URL, Pfad bleibt Pfad.
fn same_form(input_url: bool, path: &Path) -> String {
    if input_url { path_to_url(path) } else { path.to_string_lossy().to_string() }
}

pub(super) fn method(name: &str, args: &[Arg], ctx: &mut Context) -> FwResult<JsValue> {
    let a0 = args.first();
    // Testläufe schreiben nie ins echte Nutzerprofil (fp_i18n::profile)
    if matches!(name, "createDirectory" | "createFile" | "deleteFile" | "deleteFileIfExisting" | "rename" | "swap") {
        fp_i18n::profile::guard_write(&p(a0));
    }
    if matches!(name, "copy" | "swap") {
        fp_i18n::profile::guard_write(&p(args.get(1)));
    }
    match name {
        "exists" => {
            let path = super::fw::resolve_read(&p(a0));
            Ok(JsValue::from(path.exists()))
        }
        "isDirectory" => Ok(JsValue::from(super::fw::resolve_read(&p(a0)).is_dir())),
        "createDirectory" => {
            let path = p(a0);
            let ok = std::fs::create_dir_all(&path).is_ok();
            if !ok {
                set_err(format!("{}: Ordner konnte nicht angelegt werden", path.display()));
            }
            Ok(JsValue::from(ok))
        }
        "createFile" => {
            let path = p(a0);
            if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
                let _ = std::fs::create_dir_all(dir);
            }
            match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
                Ok(_) => Ok(JsValue::from(true)),
                Err(e) => {
                    set_err(e.to_string());
                    Ok(JsValue::from(false))
                }
            }
        }
        "deleteFile" | "deleteFileIfExisting" => {
            let path = p(a0);
            let r = if path.is_dir() { std::fs::remove_dir_all(&path) } else { std::fs::remove_file(&path) };
            match r {
                Ok(()) => Ok(JsValue::from(true)),
                Err(e) if name == "deleteFileIfExisting" && e.kind() == std::io::ErrorKind::NotFound => Ok(JsValue::from(true)),
                Err(e) => {
                    set_err(e.to_string());
                    if name == "deleteFile" { Err(FwErr::Hr(hr::E_FILE_NOT_FOUND, e.to_string())) } else { Ok(JsValue::from(false)) }
                }
            }
        }
        "copy" => {
            let (src, dst) = (super::fw::resolve_read(&p(a0)), p(args.get(1)));
            Ok(JsValue::from(std::fs::copy(&src, &dst).is_ok()))
        }
        "rename" => {
            let src = p(a0);
            let new_name = args.get(1).and_then(|x| x.str()).unwrap_or_default();
            let dst = if new_name.contains('/') || new_name.contains('\\') || new_name.starts_with("file:") { url_to_path(&new_name) } else { src.with_file_name(&new_name) };
            Ok(JsValue::from(std::fs::rename(&src, &dst).is_ok()))
        }
        "swap" => {
            let (a, b) = (p(a0), p(args.get(1)));
            let tmp = a.with_extension("fpswap.tmp");
            let ok = std::fs::rename(&a, &tmp).is_ok() && std::fs::rename(&b, &a).is_ok() && std::fs::rename(&tmp, &b).is_ok();
            Ok(JsValue::from(ok))
        }
        "getExtension" => {
            let s = a0.and_then(|x| x.str()).unwrap_or_default();
            let name = s.rsplit(['/', '\\']).next().unwrap_or("");
            Ok(conv::s(&name.rfind('.').map(|i| name[i..].to_string()).unwrap_or_default()))
        }
        "getFilename" => {
            let s = a0.and_then(|x| x.str()).unwrap_or_default();
            let mut name = s.trim_end_matches(['/', '\\']).rsplit(['/', '\\']).next().unwrap_or("").to_string();
            if args.get(1).is_some_and(|x| x.bool())
                && let Some(i) = name.rfind('.')
            {
                name.truncate(i);
            }
            Ok(conv::s(&name))
        }
        "setFilename" => {
            let s = a0.and_then(|x| x.str()).unwrap_or_default();
            let n = args.get(1).and_then(|x| x.str()).unwrap_or_default();
            let base = match s.rfind(['/', '\\']) {
                Some(i) => &s[..=i],
                None => "",
            };
            Ok(conv::s(&format!("{base}{n}")))
        }
        "getDirectory" => {
            let s = a0.and_then(|x| x.str()).unwrap_or_default();
            let t = s.trim_end_matches(['/', '\\']);
            Ok(conv::s(match t.rfind(['/', '\\']) {
                Some(i) => &t[..i],
                None => "",
            }))
        }
        "makePathFromDirAndFile" => {
            let d = a0.and_then(|x| x.str()).unwrap_or_default();
            let f = args.get(1).and_then(|x| x.str()).unwrap_or_default();
            let sep = if d.starts_with("file:") || d.contains('/') { "/" } else { "\\" };
            Ok(conv::s(&if d.ends_with('/') || d.ends_with('\\') || d.is_empty() { format!("{d}{f}") } else { format!("{d}{sep}{f}") }))
        }
        "getTempFilePath" => {
            let dir = match a0.and_then(|x| x.str()) {
                Some(d) if !d.is_empty() => url_to_path(&d),
                _ => std::env::temp_dir(),
            };
            let n = with_state(|s| s.handles.len())?;
            let path = dir.join(format!("fp_{}_{n}.tmp", std::process::id()));
            Ok(conv::s(&same_form(true, &path)))
        }
        "enumFiles" => {
            let input_url = is_url(a0);
            let dir = super::fw::resolve_read(&p(a0));
            let mut list: Vec<PathBuf> = std::fs::read_dir(&dir).map(|rd| rd.filter_map(|e| e.ok().map(|e| e.path())).collect()).unwrap_or_default();
            list.sort();
            let v: Vec<JsValue> = list.iter().map(|x| conv::s(&same_form(input_url, x))).collect();
            Ok(conv::array(v, ctx))
        }
        "getLanguageDirectory" => {
            let custom = with_state(|s| s.opts.language_dir.clone())?;
            let dir = custom.unwrap_or_else(|| fp_i18n::profile::roaming_dir().unwrap_or_else(|| std::env::temp_dir().join("Firepixel")).join("Language"));
            Ok(conv::s(&path_to_url(&dir)))
        }
        "getLastErrorString" => Ok(conv::s(&with_state(|s| s.last_file_error.clone())?)),
        "open" => {
            let path = super::fw::resolve_read(&p(a0));
            let write = args.get(1).is_some_and(|x| x.bool());
            let st = if write {
                fp_i18n::profile::guard_write(&path);
                if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
                    let _ = std::fs::create_dir_all(dir);
                }
                if std::fs::write(&path, b"").is_err() {
                    set_err(format!("{}: nicht beschreibbar", path.display()));
                    return Ok(JsValue::null());
                }
                FileRefState { path, write: true, lines: Vec::new(), pos: 0, buffer: String::new() }
            } else {
                match std::fs::read(&path) {
                    Ok(b) => {
                        let text = crate::decode_text(&b);
                        let lines = text.replace("\r\n", "\n").replace('\r', "\n").split('\n').map(String::from).collect();
                        FileRefState { path, write: false, lines, pos: 0, buffer: String::new() }
                    }
                    Err(e) => {
                        set_err(e.to_string());
                        return Ok(JsValue::null());
                    }
                }
            };
            let idx = with_state(|s| {
                s.files.push(Some(st));
                s.files.len() - 1
            })?;
            runtime::wrap(ctx, "FileReference", Handle::FileRef(idx))
        }
        _ => ni("Files", name),
    }
}

pub(super) fn fileref_method(name: &str, h: &Handle, args: &[Arg], _ctx: &mut Context) -> FwResult<JsValue> {
    let Handle::FileRef(i) = h else { return Err(FwErr::bad("keine Datei")) };
    let i = *i;
    match name {
        "write" | "writeUTF8" => {
            let t = args.first().and_then(|a| a.str()).unwrap_or_default();
            with_state(|s| {
                if let Some(Some(f)) = s.files.get_mut(i) {
                    f.buffer.push_str(&t);
                }
            })?;
            Ok(JsValue::from(true))
        }
        "readline" | "readlineUTF8" => {
            let line = with_state(|s| match s.files.get_mut(i) {
                Some(Some(f)) if f.pos < f.lines.len() => {
                    let l = f.lines[f.pos].clone();
                    f.pos += 1;
                    // letzte leere Zeile nach abschließendem Umbruch ist kein Eintrag
                    if f.pos == f.lines.len() && l.is_empty() { None } else { Some(l) }
                }
                _ => None,
            })?;
            Ok(line.map(|l| conv::s(&l)).unwrap_or_else(JsValue::null))
        }
        "close" => {
            let f = with_state(|s| s.files.get_mut(i).and_then(|x| x.take()))?;
            if let Some(f) = f
                && f.write
                && let Err(e) = std::fs::write(&f.path, f.buffer.as_bytes())
            {
                set_err(e.to_string());
                return Ok(JsValue::from(false));
            }
            Ok(JsValue::from(true))
        }
        _ => ni("FileReference", name),
    }
}
