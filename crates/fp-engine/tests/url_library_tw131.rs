//! TW-131: URL library of the URL panel: add used URLs, delete unused URLs, import and export of
//! `.htm`/`.txt` URL lists.

use fp_engine::Session;
use fp_engine::url_library::{UrlLibrary, parse_url_list, used_urls};
use serde_json::json;

/// Document with two slices linking to `a` and `b`, plus one slice without a link.
fn doc_with_links() -> Session {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 300, "height": 120})).expect("doc");
    for (i, url) in ["http://a.example/", "http://b.example/", ""].iter().enumerate() {
        let x = 10 + i as i64 * 90;
        s.execute("draw.rectangle", json!({"rect": [x, 10, x + 60, 60], "fillColor": "#ff0000"}))
            .expect("rect");
        s.execute("edit.insert.slice", json!({})).expect("slice");
        if !url.is_empty() {
            s.execute("pi.web", json!({"url": url})).expect("url");
        }
    }
    s
}

fn used(s: &Session) -> Vec<String> {
    used_urls(s.documents().iter().map(|d| d.doc.as_ref()))
}

#[test]
fn url_add_used_collects_slice_links_once() {
    let s = doc_with_links();
    let u = used(&s);
    let mut sorted = u.clone();
    sorted.sort();
    assert_eq!(sorted, vec!["http://a.example/".to_string(), "http://b.example/".to_string()]);
    let mut lib = UrlLibrary::new("URLs.htm");
    lib.add("http://a.example/");
    assert_eq!(lib.add_used(&u), 1);
    lib.urls.sort();
    assert_eq!(lib.urls, sorted);
    assert_eq!(lib.add_used(&u), 0, "no duplicates");
}

#[test]
fn url_delete_unused_keeps_only_referenced() {
    let s = doc_with_links();
    let mut lib = UrlLibrary::new("URLs.htm");
    for u in ["http://x.example/", "http://b.example/", "http://y.example/"] {
        lib.add(u);
    }
    assert_eq!(lib.delete_unused(&used(&s)), 2);
    assert_eq!(lib.urls, vec!["http://b.example/".to_string()]);
}

#[test]
fn url_add_edit_delete() {
    let mut lib = UrlLibrary::new("URLs.htm");
    assert!(lib.add(" http://a.example/ "));
    assert!(!lib.add("http://a.example/"));
    assert!(!lib.add("  "));
    assert!(lib.add("http://b.example/"));
    assert!(!lib.edit(1, "http://a.example/"), "edit must not create a duplicate");
    assert!(lib.edit(1, "http://c.example/"));
    assert!(lib.remove(0));
    assert!(!lib.remove(5));
    assert_eq!(lib.urls, vec!["http://c.example/".to_string()]);
}

#[test]
fn url_import_txt_and_htm() {
    let txt = "http://a.example/\r\n\r\n  http://b.example/  \nhttp://a.example/\n";
    assert_eq!(parse_url_list(txt), vec!["http://a.example/", "http://b.example/"]);
    let htm = "<html><body>\n<a href=\"http://a.example/?x=1&amp;y=2\">A</a><br>\n<A HREF='mailto:b@example.org'>B</A>\n<a href=http://c.example/>C</a>\n</body></html>";
    assert_eq!(
        parse_url_list(htm),
        vec!["http://a.example/?x=1&y=2", "mailto:b@example.org", "http://c.example/"]
    );
    // an HTML page without anchors yields no URLs (not its lines)
    assert!(parse_url_list("<html><body>nothing</body></html>").is_empty());
}

#[test]
fn url_export_and_reimport_files() {
    let dir = std::env::temp_dir().join(format!("fp-url-tw131-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let mut lib = UrlLibrary::new("URLs.htm");
    lib.add("http://a.example/?q=\"x\"&r=1");
    lib.add("http://b.example/");
    let txt = dir.join("urls.txt");
    let htm = dir.join("urls.htm");
    lib.export_file(&txt).expect("txt");
    lib.export_file(&htm).expect("htm");
    let t = std::fs::read_to_string(&txt).expect("read txt");
    assert_eq!(t, "http://a.example/?q=\"x\"&r=1\r\nhttp://b.example/\r\n");
    let h = std::fs::read_to_string(&htm).expect("read htm");
    assert!(h.contains("<a href=\"http://b.example/\">http://b.example/</a>"), "{h}");
    assert!(h.contains("href=\"http://a.example/?q=&quot;x&quot;&amp;r=1\""), "{h}");
    for p in [&txt, &htm] {
        let mut back = UrlLibrary::new("other.htm");
        assert_eq!(back.import_file(p).expect("import"), 2);
        assert_eq!(back.urls, lib.urls);
    }
    let _ = std::fs::remove_dir_all(&dir);
}
