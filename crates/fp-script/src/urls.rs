//! Datei-URLs wie im Original (`file:///C|/Ordner/Datei.png`) ↔ native Pfade.

use std::path::{Path, PathBuf};

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            let hex = std::str::from_utf8(&b[i + 1..i + 3])
                .ok()
                .and_then(|h| u8::from_str_radix(h, 16).ok());
            if let Some(v) = hex {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// URL oder Pfad → nativer Pfad. Akzeptiert `file:///C|/…`, `file:///C:/…`, `file://localhost/…`
/// und native Pfade.
pub fn url_to_path(u: &str) -> PathBuf {
    let t = u.trim();
    let lower = t.to_ascii_lowercase();
    if !lower.starts_with("file:") {
        return PathBuf::from(t);
    }
    let mut rest = &t[5..];
    rest = rest.trim_start_matches('/');
    if rest.to_ascii_lowercase().starts_with("localhost/") {
        rest = &rest[10..];
    }
    let mut p = percent_decode(rest);
    // Laufwerk: "C|/x" oder "C:/x"
    let bytes = p.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && (bytes[1] == b'|' || bytes[1] == b':')
    {
        p = format!("{}:{}", &p[..1], &p[2..]);
        if cfg!(windows) {
            return PathBuf::from(p.replace('/', "\\"));
        }
        return PathBuf::from(p);
    }
    if cfg!(windows) {
        PathBuf::from(format!("\\\\{}", p.replace('/', "\\")))
    } else {
        PathBuf::from(format!("/{p}"))
    }
}

/// Nativer Pfad → Original-URL (`file:///C|/…`).
pub fn path_to_url(p: &Path) -> String {
    let s = p.to_string_lossy().replace('\\', "/");
    let b = s.as_bytes();
    if b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':' {
        format!("file:///{}|{}", &s[..1], &s[2..])
    } else if let Some(unc) = s.strip_prefix("//") {
        format!("file://{unc}")
    } else {
        format!(
            "file://{}",
            if s.starts_with('/') {
                s.clone()
            } else {
                format!("/{s}")
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_windows_form() {
        let p = url_to_path("file:///C|/Temp/a%20b.png");
        if cfg!(windows) {
            assert_eq!(p, PathBuf::from("C:\\Temp\\a b.png"));
            assert_eq!(path_to_url(&p), "file:///C|/Temp/a b.png");
            assert_eq!(
                url_to_path("file:///C:/x/y.jsf"),
                PathBuf::from("C:\\x\\y.jsf")
            );
        }
        assert_eq!(url_to_path("relativ.txt"), PathBuf::from("relativ.txt"));
    }
}
