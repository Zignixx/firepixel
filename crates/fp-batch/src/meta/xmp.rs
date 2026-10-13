//! Kleine XMP-Bearbeitung ohne XML-Bibliothek (SPEC-BATCH §3.7): Eigenschaften entfernen (GPS,
//! Kamera …), Ausrichtung setzen, Felder für „Metadaten setzen“ eintragen bzw. ein neues Paket
//! bauen. Arbeitet auf dem Text des Pakets; unbekannte Teile bleiben unverändert.

use regex::Regex;

/// Entfernt alle Eigenschaften, deren qualifizierter Name mit einem der Präfixe beginnt (z. B.
/// `exif:GPS`, `tiff:Make`), als Attribut (`exif:GPSLatitude="…"`) und als Element.
pub fn remove_props(xmp: &str, prefixes: &[&str]) -> String {
    let mut s = xmp.to_string();
    for p in prefixes {
        let q = regex::escape(p);
        if let Ok(re) = Regex::new(&format!(r#"\s{q}[\w.-]*\s*=\s*("[^"]*"|'[^']*')"#)) {
            s = re.replace_all(&s, "").into_owned();
        }
        if let Ok(re) = Regex::new(&format!(r"(?s)\s*<{q}[\w.-]*(\s[^>]*)?/>")) {
            s = re.replace_all(&s, "").into_owned();
        }
        if let Ok(re) = Regex::new(&format!(r"(?s)\s*<{q}[\w.-]*(\s[^>]*)?>.*?</{q}[\w.-]*>")) {
            s = re.replace_all(&s, "").into_owned();
        }
    }
    s
}

/// Hat das Paket eine Eigenschaft mit diesem Präfix?
pub fn has_prop(xmp: &str, prefix: &str) -> bool {
    let q = regex::escape(prefix);
    Regex::new(&format!(r"[\s<]{q}[\w.-]*[\s=>/]")).is_ok_and(|re| re.is_match(xmp))
}

/// Setzt `tiff:Orientation` (Attribut oder Element), falls vorhanden.
pub fn set_orientation(xmp: &str, v: u16) -> String {
    let mut s = xmp.to_string();
    if let Ok(re) = Regex::new(r#"tiff:Orientation\s*=\s*("[^"]*"|'[^']*')"#) {
        s = re.replace_all(&s, format!("tiff:Orientation=\"{v}\"").as_str()).into_owned();
    }
    if let Ok(re) = Regex::new(r"<tiff:Orientation>\s*\d+\s*</tiff:Orientation>") {
        s = re.replace_all(&s, format!("<tiff:Orientation>{v}</tiff:Orientation>").as_str()).into_owned();
    }
    s
}

/// XML-Text maskieren.
pub fn escape(t: &str) -> String {
    t.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Felder von „Metadaten setzen“ (None = unverändert, `Some("")` = entfernen).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fields {
    pub creator: Option<String>,
    pub rights: Option<String>,
    pub description: Option<String>,
    pub title: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub rating: Option<i64>,
}

const DC_NS: &str = "http://purl.org/dc/elements/1.1/";

/// Namensraum der XMP-Grundeigenschaften (`xmp:Rating`).
pub fn xmp_basic_ns() -> String {
    format!("http://ns.{}.com/xap/1.0/", super::container::vendor())
}

fn alt(v: &str) -> String {
    format!("<rdf:Alt><rdf:li xml:lang=\"x-default\">{}</rdf:li></rdf:Alt>", escape(v))
}

/// Leeres Paket.
pub fn empty_packet() -> String {
    format!(
        "<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n<x:xmpmeta xmlns:x=\"{}:ns:meta/\">\n <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n  <rdf:Description rdf:about=\"\">\n  </rdf:Description>\n </rdf:RDF>\n</x:xmpmeta>\n<?xpacket end=\"w\"?>",
        super::container::vendor()
    )
}

/// Trägt Felder in ein Paket ein (bzw. in ein neues). Vorhandene Werte dieser Felder werden ersetzt.
pub fn set_fields(xmp: Option<&str>, f: &Fields) -> String {
    let mut s = xmp.map(str::to_string).unwrap_or_else(empty_packet);
    let mut remove: Vec<&str> = Vec::new();
    let mut add = String::new();
    let mut need_dc = false;
    let mut push = |name: &'static str, body: Option<String>, remove: &mut Vec<&'static str>| {
        remove.push(name);
        if let Some(b) = body {
            add.push_str(&format!("\n   <{name}>{b}</{name}>"));
        }
    };
    if let Some(v) = &f.creator {
        need_dc |= !v.is_empty();
        push("dc:creator", (!v.is_empty()).then(|| format!("<rdf:Seq><rdf:li>{}</rdf:li></rdf:Seq>", escape(v))), &mut remove);
    }
    if let Some(v) = &f.rights {
        need_dc |= !v.is_empty();
        push("dc:rights", (!v.is_empty()).then(|| alt(v)), &mut remove);
    }
    if let Some(v) = &f.description {
        need_dc |= !v.is_empty();
        push("dc:description", (!v.is_empty()).then(|| alt(v)), &mut remove);
    }
    if let Some(v) = &f.title {
        need_dc |= !v.is_empty();
        push("dc:title", (!v.is_empty()).then(|| alt(v)), &mut remove);
    }
    if let Some(k) = &f.keywords {
        need_dc |= !k.is_empty();
        let items: String = k.iter().map(|w| format!("<rdf:li>{}</rdf:li>", escape(w))).collect();
        push("dc:subject", (!k.is_empty()).then(|| format!("<rdf:Bag>{items}</rdf:Bag>")), &mut remove);
    }
    let mut need_xmp = false;
    if let Some(r) = f.rating {
        remove.push("xmp:Rating");
        if r >= 0 {
            need_xmp = true;
            add.push_str(&format!("\n   <xmp:Rating>{r}</xmp:Rating>"));
        }
    }
    s = remove_props(&s, &remove);
    // erstes rdf:Description finden (selbstschließend → öffnen)
    let Some(start) = s.find("<rdf:Description") else {
        return set_fields(None, f);
    };
    let Some(rel_end) = s[start..].find('>') else { return s };
    let end = start + rel_end;
    let self_closing = s[..end].ends_with('/');
    let mut open = s[start..if self_closing { end - 1 } else { end }].to_string();
    if need_dc && !open.contains("xmlns:dc=") {
        open.push_str(&format!(" xmlns:dc=\"{DC_NS}\""));
    }
    if need_xmp && !open.contains("xmlns:xmp=") {
        open.push_str(&format!(" xmlns:xmp=\"{}\"", xmp_basic_ns()));
    }
    let tail = if self_closing { format!(">{add}\n  </rdf:Description>") } else { format!(">{add}") };
    s.replace_range(start..=end, &format!("{open}{tail}"));
    s
}
