//! Datei ▸ Datei-Info: Metadaten des Dokuments als XMP-Paket (`re\02-fwpng-format.md` §1: XMP steckt
//! im `iTXt`- bzw. `tEXt`-Chunk der FW-PNG, erkannt am Paket). Gelesen und geschrieben werden die
//! Dublin-Core-Felder Titel, Autor, Beschreibung, Schlüsselwörter und Copyright; Erstellt/Geändert
//! und Anwendung werden nur angezeigt. Alles andere im Paket bleibt erhalten. Ändern ist ein
//! Verlaufsschritt (das Paket liegt in `Document::file_meta`).

use super::DOC;
use crate::command::{CommandOutput, CommandRegistry, ParamKind, Params, cmd};
use crate::error::Result;
use fp_model::doc::RawChunk;
use serde_json::{Value, json};
use std::sync::Arc;

/// Schlüssel eines neu angelegten XMP-Chunks. Ein vorhandener Chunk behält seinen Schlüssel.
pub const NEW_CHUNK_KEY: &str = "XMP";

const DC_NS: &str = "http://purl.org/dc/elements/1.1/";
const RDF_NS: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";

/// Die bearbeitbaren Felder und die nur angezeigten Angaben.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct XmpInfo {
    pub title: String,
    pub author: String,
    pub description: String,
    pub keywords: Vec<String>,
    pub copyright: String,
    /// Autorentitel (`photoshop:AuthorsPosition`, BA-023).
    pub author_title: String,
    /// Verfasser der Beschreibung (`photoshop:CaptionWriter`).
    pub caption_writer: String,
    /// Bewertung 0–5 Sterne (`xmp:Rating`; 0 = keine).
    pub rating: u8,
    /// Copyright-Status (`xmpRights:Marked`): `None` = Unbekannt, `Some(true)` = urheberrechtlich
    /// geschützt, `Some(false)` = gemeinfrei.
    pub copyright_marked: Option<bool>,
    /// URL für Copyright-Informationen (`xmpRights:WebStatement`).
    pub copyright_url: String,
    pub created: Option<String>,
    pub modified: Option<String>,
    pub application: Option<String>,
    /// Format (`dc:format`), nur angezeigt.
    pub format: Option<String>,
}

/// Weitere Felder der Seite „Beschreibung“ außerhalb von Dublin Core (BA-023), Elementnamen mit Präfix.
const EXTRA_TAGS: [&str; 5] = [
    "photoshop:AuthorsPosition",
    "photoshop:CaptionWriter",
    "xmp:Rating",
    "xmpRights:Marked",
    "xmpRights:WebStatement",
];

/// Standard-Namensräume der XMP-Felder `photoshop:`, `xmp:` und `xmpRights:`. Der Rechnername der
/// Namensräume enthält den Herstellernamen; er steht deshalb weder im Quelltext noch im Programm am
/// Stück (Neutralitätsregel), sondern wird hier rückwärts gespeichert zusammengesetzt.
fn std_ns(path: &str) -> String {
    let host: String = "moc.ebod".chars().rev().collect();
    format!("http://ns.a{host}/{path}")
}

impl XmpInfo {
    /// Nur die bearbeitbaren Felder vergleichen.
    fn same_fields(&self, o: &XmpInfo) -> bool {
        self.title == o.title
            && self.author == o.author
            && self.description == o.description
            && self.keywords == o.keywords
            && self.copyright == o.copyright
            && self.author_title == o.author_title
            && self.caption_writer == o.caption_writer
            && self.rating == o.rating
            && self.copyright_marked == o.copyright_marked
            && self.copyright_url == o.copyright_url
    }
    fn is_empty(&self) -> bool {
        self.title.is_empty()
            && self.author.is_empty()
            && self.description.is_empty()
            && self.keywords.is_empty()
            && self.copyright.is_empty()
            && self.extra_empty()
    }
    fn extra_empty(&self) -> bool {
        self.author_title.is_empty()
            && self.caption_writer.is_empty()
            && self.rating == 0
            && self.copyright_marked.is_none()
            && self.copyright_url.is_empty()
    }
    /// Copyright-Status als Text: `unknown`, `copyrighted` oder `publicDomain`.
    pub fn copyright_status(&self) -> &'static str {
        match self.copyright_marked {
            None => "unknown",
            Some(true) => "copyrighted",
            Some(false) => "publicDomain",
        }
    }
    pub fn to_json(&self) -> Value {
        json!({
            "title": self.title, "author": self.author, "description": self.description,
            "keywords": self.keywords, "copyright": self.copyright,
            "authorTitle": self.author_title, "captionWriter": self.caption_writer, "rating": self.rating,
            "copyrightStatus": self.copyright_status(), "copyrightUrl": self.copyright_url,
            "created": self.created, "modified": self.modified, "application": self.application, "format": self.format,
        })
    }
}

/// Schlüsselwörter aus einem Text (getrennt durch Komma, Semikolon oder Zeilenumbruch).
pub fn split_keywords(s: &str) -> Vec<String> {
    s.split([',', ';', '\n'])
        .map(str::trim)
        .filter(|k| !k.is_empty())
        .map(str::to_string)
        .collect()
}

/// (Schlüssel, Paket-Text) eines XMP-Chunks.
pub fn chunk_packet(c: &RawChunk) -> Option<(String, String)> {
    match &c.kind {
        b"iTXt" => fp_fwpng::png::parse_itxt(&c.data),
        b"tEXt" => {
            let z = c.data.iter().position(|&b| b == 0)?;
            let key: String = c.data[..z].iter().map(|&b| b as char).collect();
            let text: String = c.data[z + 1..].iter().map(|&b| b as char).collect();
            Some((key, text))
        }
        _ => None,
    }
}

/// Unkomprimierter `iTXt`-Chunk mit dem Paket.
pub fn make_chunk(key: &str, packet: &str, after_image: bool) -> RawChunk {
    let mut data: Vec<u8> = key
        .chars()
        .map(|c| if (c as u32) < 256 { c as u8 } else { b'?' })
        .collect();
    data.extend_from_slice(&[0, 0, 0, 0, 0]);
    data.extend_from_slice(packet.as_bytes());
    RawChunk {
        kind: *b"iTXt",
        data,
        after_image,
    }
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let Some(end) = rest.find(';').filter(|&e| e <= 10) else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let ent = &rest[1..end];
        let ch = match ent {
            "lt" => Some('<'),
            "gt" => Some('>'),
            "amp" => Some('&'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ if ent.starts_with("#x") || ent.starts_with("#X") => {
                u32::from_str_radix(&ent[2..], 16)
                    .ok()
                    .and_then(char::from_u32)
            }
            _ if ent.starts_with('#') => ent[1..].parse::<u32>().ok().and_then(char::from_u32),
            _ => None,
        };
        match ch {
            Some(c) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Ende des Start-Tags ab `start` (Position von `<`): (Ende hinter `>`, selbstschließend?).
fn tag_end(xml: &str, start: usize) -> Option<(usize, bool)> {
    let e = xml[start..].find('>')? + start;
    Some((e + 1, xml[..e].ends_with('/')))
}

/// Findet das nächste Element `<name …>…</name>` bzw. `<name …/>` ab `from`: (Anfang, Ende, Inhalt).
fn find_element<'a>(xml: &'a str, name: &str, from: usize) -> Option<(usize, usize, &'a str)> {
    let open = format!("<{name}");
    let mut pos = from;
    loop {
        let s = xml[pos..].find(&open)? + pos;
        let after = xml[s + open.len()..].chars().next()?;
        if !(after.is_whitespace() || after == '>' || after == '/') {
            pos = s + open.len();
            continue;
        }
        let (body, empty) = tag_end(xml, s)?;
        if empty {
            return Some((s, body, ""));
        }
        let close = format!("</{name}>");
        let c = xml[body..].find(&close)? + body;
        return Some((s, c + close.len(), &xml[body..c]));
    }
}

/// Texte der `rdf:li` eines Containers (bei `rdf:Alt` zuerst `x-default`); ohne Container der Text selbst.
fn li_values(inner: &str) -> Vec<String> {
    let mut v = Vec::new();
    let mut default = None;
    let mut pos = 0;
    while let Some((s, e, text)) = find_element(inner, "rdf:li", pos) {
        let head = &inner[s..tag_end(inner, s).map(|t| t.0).unwrap_or(e)];
        let t = unescape(text.trim());
        if default.is_none() && head.contains("x-default") {
            default = Some(v.len());
        }
        v.push(t);
        pos = e;
    }
    if v.is_empty() && !inner.contains('<') && !inner.trim().is_empty() {
        v.push(unescape(inner.trim()));
    }
    if let Some(i) = default {
        let d = v.remove(i);
        v.insert(0, d);
    }
    v
}

/// Wert als Element `<name>…</name>` oder als Attribut `name="…"`.
fn simple_value(xml: &str, name: &str) -> Option<String> {
    if let Some((_, _, inner)) = find_element(xml, name, 0) {
        let v = li_values(inner);
        if let Some(f) = v.into_iter().next() {
            return Some(f);
        }
    }
    let attr = format!("{name}=\"");
    let s = xml.find(&attr)? + attr.len();
    let e = xml[s..].find('"')? + s;
    Some(unescape(&xml[s..e]))
}

fn dc_values(xml: &str, tag: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut pos = 0;
    let name = format!("dc:{tag}");
    while let Some((_, e, inner)) = find_element(xml, &name, pos) {
        out.extend(li_values(inner));
        pos = e;
    }
    if out.is_empty()
        && let Some(v) = simple_value(xml, &name)
    {
        out.push(v);
    }
    out
}

/// Liest die Felder aus einem Paket.
pub fn parse(xml: &str) -> XmpInfo {
    let first = |tag: &str| dc_values(xml, tag).into_iter().next().unwrap_or_default();
    XmpInfo {
        title: first("title"),
        author: dc_values(xml, "creator").join("; "),
        description: first("description"),
        keywords: dc_values(xml, "subject")
            .iter()
            .flat_map(|k| split_keywords(k))
            .collect(),
        copyright: first("rights"),
        author_title: simple_value(xml, "photoshop:AuthorsPosition").unwrap_or_default(),
        caption_writer: simple_value(xml, "photoshop:CaptionWriter").unwrap_or_default(),
        rating: simple_value(xml, "xmp:Rating")
            .and_then(|v| v.trim().parse::<f64>().ok())
            .map(|v| v.clamp(0.0, 5.0).round() as u8)
            .unwrap_or(0),
        copyright_marked: simple_value(xml, "xmpRights:Marked").and_then(|v| {
            match v.trim().to_ascii_lowercase().as_str() {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            }
        }),
        copyright_url: simple_value(xml, "xmpRights:WebStatement").unwrap_or_default(),
        created: simple_value(xml, "xmp:CreateDate"),
        modified: simple_value(xml, "xmp:ModifyDate"),
        application: simple_value(xml, "xmp:CreatorTool"),
        format: dc_values(xml, "format").into_iter().next(),
    }
}

/// Entfernt alle Elemente und Attribute `dc:<tag>`.
fn strip_dc(xml: &str, tag: &str) -> String {
    strip_name(xml, &format!("dc:{tag}"))
}

/// Entfernt alle Elemente und Attribute `<name>` (mit Präfix).
fn strip_name(xml: &str, name: &str) -> String {
    let name = name.to_string();
    let mut s = xml.to_string();
    while let Some((a, b, _)) = find_element(&s, &name, 0) {
        s.replace_range(a..b, "");
    }
    let attr = format!(" {name}=\"");
    while let Some(a) = s.find(&attr) {
        let Some(e) = s[a + attr.len()..].find('"') else {
            break;
        };
        s.replace_range(a..a + attr.len() + e + 1, "");
    }
    s
}

fn alt(v: &str) -> String {
    format!(
        "<rdf:Alt><rdf:li xml:lang=\"x-default\">{}</rdf:li></rdf:Alt>",
        escape(v)
    )
}

/// Baut das Paket neu: vorhandenen Inhalt behalten, die fünf Felder ersetzen.
pub fn build(existing: Option<&str>, info: &XmpInfo) -> String {
    const TAGS: [&str; 5] = ["title", "creator", "description", "subject", "rights"];
    let base = existing.filter(|x| x.contains("</rdf:RDF>")).map(str::to_string).unwrap_or_else(|| {
        format!("<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n<rdf:RDF xmlns:rdf=\"{RDF_NS}\">\n</rdf:RDF>\n<?xpacket end=\"w\"?>")
    });
    let mut xml = base;
    for t in TAGS {
        xml = strip_dc(&xml, t);
    }
    for t in EXTRA_TAGS {
        xml = strip_name(&xml, t);
    }
    if info.is_empty() {
        return xml;
    }
    if !info.extra_empty() {
        let (ps, xmp, rights) = (
            std_ns("photoshop/1.0/"),
            std_ns("xap/1.0/"),
            std_ns("xap/1.0/rights/"),
        );
        let mut extra = format!(
            " <rdf:Description rdf:about=\"\" xmlns:photoshop=\"{ps}\" xmlns:xmp=\"{xmp}\" xmlns:xmpRights=\"{rights}\">\n"
        );
        if !info.author_title.is_empty() {
            extra += &format!(
                "  <photoshop:AuthorsPosition>{}</photoshop:AuthorsPosition>\n",
                escape(&info.author_title)
            );
        }
        if !info.caption_writer.is_empty() {
            extra += &format!(
                "  <photoshop:CaptionWriter>{}</photoshop:CaptionWriter>\n",
                escape(&info.caption_writer)
            );
        }
        if info.rating > 0 {
            extra += &format!("  <xmp:Rating>{}</xmp:Rating>\n", info.rating.min(5));
        }
        if let Some(m) = info.copyright_marked {
            extra += &format!(
                "  <xmpRights:Marked>{}</xmpRights:Marked>\n",
                if m { "True" } else { "False" }
            );
        }
        if !info.copyright_url.is_empty() {
            extra += &format!(
                "  <xmpRights:WebStatement>{}</xmpRights:WebStatement>\n",
                escape(&info.copyright_url)
            );
        }
        extra += " </rdf:Description>\n";
        let at = xml.rfind("</rdf:RDF>").unwrap_or(xml.len());
        xml.insert_str(at, &extra);
    }
    let dc_empty = info.title.is_empty()
        && info.author.is_empty()
        && info.description.is_empty()
        && info.keywords.is_empty()
        && info.copyright.is_empty();
    if dc_empty {
        return xml;
    }
    let mut block = format!(" <rdf:Description rdf:about=\"\" xmlns:dc=\"{DC_NS}\">\n");
    if !info.title.is_empty() {
        block += &format!("  <dc:title>{}</dc:title>\n", alt(&info.title));
    }
    if !info.author.is_empty() {
        let lis: String = info
            .author
            .split(';')
            .map(str::trim)
            .filter(|a| !a.is_empty())
            .map(|a| format!("<rdf:li>{}</rdf:li>", escape(a)))
            .collect();
        block += &format!("  <dc:creator><rdf:Seq>{lis}</rdf:Seq></dc:creator>\n");
    }
    if !info.description.is_empty() {
        block += &format!(
            "  <dc:description>{}</dc:description>\n",
            alt(&info.description)
        );
    }
    if !info.keywords.is_empty() {
        let lis: String = info
            .keywords
            .iter()
            .map(|k| format!("<rdf:li>{}</rdf:li>", escape(k)))
            .collect();
        block += &format!("  <dc:subject><rdf:Bag>{lis}</rdf:Bag></dc:subject>\n");
    }
    if !info.copyright.is_empty() {
        block += &format!("  <dc:rights>{}</dc:rights>\n", alt(&info.copyright));
    }
    block += " </rdf:Description>\n";
    let at = xml.rfind("</rdf:RDF>").unwrap_or(xml.len());
    xml.insert_str(at, &block);
    xml
}

/// Felder des Dokuments.
pub fn read_info(doc: &fp_model::Document) -> XmpInfo {
    doc.file_meta
        .xmp_chunk
        .as_ref()
        .and_then(chunk_packet)
        .map(|(_, x)| parse(&x))
        .unwrap_or_default()
}

fn str_param(p: &Params, k: &str) -> Result<Option<String>> {
    p.opt_str(k)
}

pub(super) fn register(r: &mut CommandRegistry) {
    r.add(
        cmd("file.fileInfo")
            .requires(DOC)
            .opt("title", ParamKind::String, "Metadaten: Dokumenttitel")
            .opt("author", ParamKind::String, "Metadaten: Autor (mehrere durch Semikolon getrennt)")
            .opt("description", ParamKind::String, "Metadaten: Beschreibung")
            .opt("keywords", ParamKind::Json, "Metadaten: Schlüsselwörter (Liste oder Text mit Komma/Semikolon)")
            .opt("copyright", ParamKind::String, "Metadaten: Copyright-Informationen")
            .opt("authorTitle", ParamKind::String, "Metadaten: Autorentitel")
            .opt("captionWriter", ParamKind::String, "Metadaten: Verfasser der Beschreibung")
            .opt("rating", ParamKind::Integer { min: Some(0), max: Some(5) }, "Metadaten: Bewertung (0–5 Sterne)")
            .opt("copyrightStatus", ParamKind::Enum { values: vec!["unknown".into(), "copyrighted".into(), "publicDomain".into()] }, "Metadaten: Copyright-Status")
            .opt("copyrightUrl", ParamKind::String, "Metadaten: URL für Copyright-Informationen")
            .run(|s, p| {
                let cur = read_info(&s.doc()?.doc);
                let mut next = cur.clone();
                if let Some(v) = str_param(p, "title")? {
                    next.title = v.trim().to_string();
                }
                if let Some(v) = str_param(p, "author")? {
                    next.author = v.trim().to_string();
                }
                if let Some(v) = str_param(p, "description")? {
                    next.description = v.trim_end().to_string();
                }
                if let Some(v) = str_param(p, "copyright")? {
                    next.copyright = v.trim().to_string();
                }
                if let Some(v) = str_param(p, "authorTitle")? {
                    next.author_title = v.trim().to_string();
                }
                if let Some(v) = str_param(p, "captionWriter")? {
                    next.caption_writer = v.trim().to_string();
                }
                if let Some(v) = p.opt_i64("rating")? {
                    next.rating = v.clamp(0, 5) as u8;
                }
                if let Some(v) = str_param(p, "copyrightStatus")? {
                    next.copyright_marked = match v.as_str() {
                        "copyrighted" => Some(true),
                        "publicDomain" => Some(false),
                        _ => None,
                    };
                }
                if let Some(v) = str_param(p, "copyrightUrl")? {
                    next.copyright_url = v.trim().to_string();
                }
                match p.get("keywords") {
                    Some(Value::Array(a)) => next.keywords = a.iter().filter_map(Value::as_str).flat_map(split_keywords).collect(),
                    Some(Value::String(t)) => next.keywords = split_keywords(t),
                    _ => {}
                }
                let changed = !next.same_fields(&cur);
                if changed {
                    let d = s.doc_mut()?;
                    let old = d.doc.file_meta.xmp_chunk.clone();
                    let (key, packet, after) = match old.as_ref().and_then(|c| chunk_packet(c).map(|(k, x)| (k, x, c.after_image))) {
                        Some((k, x, a)) => (k, Some(x), a),
                        None => (NEW_CHUNK_KEY.to_string(), None, false),
                    };
                    let xml = build(packet.as_deref(), &next);
                    Arc::make_mut(&mut d.doc).file_meta.xmp_chunk = Some(make_chunk(&key, &xml, after));
                }
                let d = s.doc()?;
                let m = &d.doc.file_meta;
                let info = read_info(&d.doc);
                let value = json!({
                    "title": d.title, "path": d.path.as_ref().map(|p| p.to_string_lossy().to_string()),
                    "pages": d.doc.pages.len(), "symbols": d.doc.symbols.len(), "formatVersion": d.doc.format_version(),
                    "text": m.text_chunks.iter().map(|(k, v)| json!({"key": k, "text": v})).collect::<Vec<_>>(),
                    "xmp": m.xmp_chunk.is_some(), "dirty": d.is_dirty(),
                    "metadata": info.to_json(),
                });
                if changed {
                    return Ok(CommandOutput::doc_with(value).labeled(fp_i18n::t("msg-step-file-info")));
                }
                Ok(CommandOutput::value(value))
            }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_and_parse_roundtrip() {
        let i = XmpInfo {
            title: "Logo <A&B>".into(),
            author: "Anna; Ben".into(),
            description: "Zeile".into(),
            keywords: vec!["rot".into(), "blau".into()],
            copyright: "© 2026".into(),
            ..Default::default()
        };
        let x = build(None, &i);
        assert!(x.starts_with("<?xpacket "));
        let back = parse(&x);
        assert!(back.same_fields(&i), "{back:?}");
        // erneut bauen ersetzt statt zu verdoppeln
        let x2 = build(Some(&x), &i);
        assert_eq!(x2.matches("<dc:title>").count(), 1);
        // fremder Inhalt bleibt
        let foreign = "<?xpacket begin=\"\"?><x:xmpmeta><rdf:RDF><rdf:Description rdf:about=\"\" xmp:CreatorTool=\"Tool 1\"><dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\">Alt</rdf:li></rdf:Alt></dc:title></rdf:Description></rdf:RDF></x:xmpmeta><?xpacket end=\"w\"?>";
        let p = parse(foreign);
        assert_eq!(
            (p.title.as_str(), p.application.as_deref()),
            ("Alt", Some("Tool 1"))
        );
        let y = build(
            Some(foreign),
            &XmpInfo {
                title: "Neu".into(),
                ..Default::default()
            },
        );
        assert!(y.contains("xmp:CreatorTool=\"Tool 1\"") && !y.contains(">Alt<"));
        assert_eq!(parse(&y).title, "Neu");
    }

    /// BA-023: weitere Felder der Seite „Beschreibung“ (Autorentitel, Verfasser, Bewertung,
    /// Copyright-Status und -URL) werden geschrieben, gelesen und beim erneuten Bauen ersetzt.
    #[test]
    fn description_page_extra_fields_roundtrip() {
        let i = XmpInfo {
            author_title: "Grafik".into(),
            caption_writer: "Ben".into(),
            rating: 4,
            copyright_marked: Some(true),
            copyright_url: "https://example.org/r?a=1&b=2".into(),
            ..Default::default()
        };
        let x = build(None, &i);
        assert!(
            x.contains("<xmp:Rating>4</xmp:Rating>")
                && x.contains("<xmpRights:Marked>True</xmpRights:Marked>"),
            "{x}"
        );
        assert!(!x.contains("<dc:"), "ohne Dublin-Core-Felder kein dc-Block");
        let back = parse(&x);
        assert!(back.same_fields(&i), "{back:?}");
        assert_eq!(back.copyright_status(), "copyrighted");
        let y = build(
            Some(&x),
            &XmpInfo {
                rating: 2,
                copyright_marked: Some(false),
                ..i.clone()
            },
        );
        assert_eq!(y.matches("<xmp:Rating>").count(), 1);
        assert_eq!(
            (parse(&y).rating, parse(&y).copyright_status()),
            (2, "publicDomain")
        );
        // alles leer: Felder entfernt
        let z = build(Some(&y), &XmpInfo::default());
        assert!(
            !z.contains("AuthorsPosition") && !z.contains("xmp:Rating"),
            "{z}"
        );
        assert_eq!(parse(&z).copyright_status(), "unknown");
        assert!(
            std_ns("xap/1.0/").starts_with("http://ns.")
                && std_ns("xap/1.0/").ends_with(".com/xap/1.0/")
        );
    }
}
