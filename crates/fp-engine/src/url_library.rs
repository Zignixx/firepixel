//! URL libraries of the URL panel (TW-131).
//!
//! A library is a named list of URLs (default `URLs.htm`). The panel menu offers: add the URLs used
//! in the open documents (slices, hotspots and buttons), delete the URLs no open document uses,
//! add/edit/delete one URL, a new empty library, and import/export of URL list files. Two file
//! formats: `.txt` is one URL per line, `.htm`/`.html` is a page with one `<a href="…">` anchor per
//! URL. Import reads the anchors of an HTML file, otherwise every non-empty line.

use fp_model::{Document, ElementKind};
use std::path::Path;

/// Name of the default library.
pub const DEFAULT_NAME: &str = "URLs.htm";

/// A named URL list (order as added, no duplicates, no empty entries).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UrlLibrary {
    /// Display name (as in the library combo, e.g. `URLs.htm`).
    pub name: String,
    /// The URLs.
    pub urls: Vec<String>,
}

impl UrlLibrary {
    /// Empty library with this name.
    pub fn new(name: impl Into<String>) -> UrlLibrary {
        UrlLibrary {
            name: name.into(),
            urls: Vec::new(),
        }
    }

    /// Adds `url` (trimmed) unless it is empty or already present; `true` if added.
    pub fn add(&mut self, url: &str) -> bool {
        let u = url.trim();
        if u.is_empty() || self.urls.iter().any(|x| x == u) {
            return false;
        }
        self.urls.push(u.to_string());
        true
    }

    /// Replaces the URL at `index`; `false` for a bad index, an empty URL or a duplicate of another
    /// entry.
    pub fn edit(&mut self, index: usize, url: &str) -> bool {
        let u = url.trim();
        if u.is_empty() || index >= self.urls.len() {
            return false;
        }
        if self
            .urls
            .iter()
            .enumerate()
            .any(|(i, x)| i != index && x == u)
        {
            return false;
        }
        self.urls[index] = u.to_string();
        true
    }

    /// Removes the URL at `index`; `true` if removed.
    pub fn remove(&mut self, index: usize) -> bool {
        if index < self.urls.len() {
            self.urls.remove(index);
            true
        } else {
            false
        }
    }

    /// Adds every URL of `used` that is missing; returns how many were added.
    pub fn add_used(&mut self, used: &[String]) -> usize {
        used.iter().filter(|u| self.add(u)).count()
    }

    /// Removes every URL that is not in `used`; returns how many were removed.
    pub fn delete_unused(&mut self, used: &[String]) -> usize {
        let before = self.urls.len();
        self.urls.retain(|u| used.iter().any(|x| x == u));
        before - self.urls.len()
    }

    /// Adds the URLs of a URL list file's text (see [`parse_url_list`]); returns how many were added.
    pub fn import_text(&mut self, text: &str) -> usize {
        self.add_used(&parse_url_list(text))
    }

    /// Imports a `.htm`/`.html`/`.txt` file; returns how many URLs were added.
    pub fn import_file(&mut self, path: &Path) -> Result<usize, String> {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        Ok(self.import_text(&String::from_utf8_lossy(&bytes)))
    }

    /// One URL per line.
    pub fn to_txt(&self) -> String {
        let mut s = self.urls.join("\r\n");
        if !s.is_empty() {
            s.push_str("\r\n");
        }
        s
    }

    /// HTML page with one anchor per URL.
    pub fn to_htm(&self) -> String {
        let mut s = String::from("<html>\r\n<head>\r\n<title>");
        s.push_str(&escape(&self.name));
        s.push_str("</title>\r\n</head>\r\n<body>\r\n");
        for u in &self.urls {
            let e = escape(u);
            s.push_str(&format!("<a href=\"{e}\">{e}</a><br>\r\n"));
        }
        s.push_str("</body>\r\n</html>\r\n");
        s
    }

    /// Writes the library to `path`: HTML for `.htm`/`.html`, one URL per line otherwise.
    pub fn export_file(&self, path: &Path) -> Result<(), String> {
        let text = if is_html_path(path) {
            self.to_htm()
        } else {
            self.to_txt()
        };
        std::fs::write(path, text).map_err(|e| e.to_string())
    }
}

/// `true` for `.htm`/`.html`.
pub fn is_html_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("htm") || e.eq_ignore_ascii_case("html"))
}

/// URLs of a URL list file: the `href` values of an HTML page, otherwise every non-empty line.
pub fn parse_url_list(text: &str) -> Vec<String> {
    let hrefs = hrefs(text);
    let raw = if hrefs.is_empty() && !looks_like_html(text) {
        text.lines().map(|l| l.trim().to_string()).collect()
    } else {
        hrefs
    };
    let mut out: Vec<String> = Vec::new();
    for u in raw {
        if !u.is_empty() && !out.contains(&u) {
            out.push(u);
        }
    }
    out
}

fn looks_like_html(text: &str) -> bool {
    let l = text.to_ascii_lowercase();
    l.contains("<html") || l.contains("<body") || l.contains("<a ")
}

/// Values of all `href` attributes (quoted or not), with the basic entities decoded.
fn hrefs(text: &str) -> Vec<String> {
    let lower = text.to_ascii_lowercase();
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(p) = lower[from..].find("href") {
        let mut i = from + p + 4;
        from = i;
        let rest = &text[i..];
        let ws = rest.len() - rest.trim_start().len();
        i += ws;
        if !text[i..].starts_with('=') {
            continue;
        }
        i += 1;
        let rest = &text[i..];
        i += rest.len() - rest.trim_start().len();
        let rest = &text[i..];
        let value = match rest.chars().next() {
            Some(q @ ('"' | '\'')) => rest[1..].split(q).next().unwrap_or(""),
            Some(_) => rest
                .split(|c: char| c.is_whitespace() || c == '>')
                .next()
                .unwrap_or(""),
            None => "",
        };
        from = i + value.len();
        out.push(unescape(value.trim()));
    }
    out
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn unescape(s: &str) -> String {
    s.replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

/// URLs used in `docs`: the links of all slices and hotspots, button instances and button symbols
/// (first occurrence order, no duplicates, no empty links).
pub fn used_urls<'a>(docs: impl IntoIterator<Item = &'a Document>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut push = |u: Option<&String>| {
        if let Some(u) = u.map(|u| u.trim())
            && !u.is_empty()
            && !out.iter().any(|x| x == u)
        {
            out.push(u.to_string());
        }
    };
    for doc in docs {
        doc.walk_elements(|_, _, e| match &e.kind {
            ElementKind::WebObject(w) => push(w.url.as_ref()),
            ElementKind::Instance(i) => push(i.button.url.as_ref()),
            _ => {}
        });
        for s in &doc.symbols {
            push(s.button.url.as_ref());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn htm_round_trip_keeps_special_characters() {
        let mut l = UrlLibrary::new(DEFAULT_NAME);
        l.add("http://a.example/?x=1&y=\"2\"");
        l.add("mailto:b@example.org");
        let back = parse_url_list(&l.to_htm());
        assert_eq!(back, l.urls);
        assert_eq!(parse_url_list(&l.to_txt()), l.urls);
    }
}
