//! PDF-Ebenen aus optionalen Inhalten (OCG, U-1).
//!
//! hayro rendert eine Seite nur als Ganzes nach SVG und reicht keine Inhaltsgruppen durch.
//! Deshalb wird je Ebene ein eigener Durchgang gerendert: der Inhaltsstrom der Seite wird
//! zerlegt (hayro-syntax), jeder Befehl seiner OCG zugeordnet (innerste `/OC … BDC`-Klammer,
//! bei `Do` auch das `/OC` des XObjects) und alles, was nicht zur Ziel-Ebene gehört, in eine
//! Klammer mit einer ausgeschalteten Gruppe gesetzt. Zustandsbefehle (q/Q, cm, Farben,
//! Beschneidungen) laufen dabei weiter, nur das Zeichnen entfällt. Die geänderten Objekte
//! (Katalog mit eigener `/OCProperties`-Vorgabe, Inhaltsströme) hängt eine inkrementelle
//! Aktualisierung an eine Kopie der Datei an; die Originaldaten bleiben unberührt.

use std::collections::{HashMap, HashSet};
use std::ops::Range;

use hayro_svg::hayro_syntax::Pdf;
use hayro_svg::hayro_syntax::content::UntypedIter;
use hayro_svg::hayro_syntax::object::{Array, Dict, MaybeRef, Object, ObjectIdentifier, Stream};
use hayro_svg::hayro_syntax::page::Page;

/// Höchstzahl der Ebenen je Seite (je Ebene ein Renderdurchgang); darüber eine Ebene wie bisher.
const MAX_LAYERS: usize = 64;

/// Optionale Inhalte des Dokuments.
pub(crate) struct OcInfo {
    /// Alle Gruppen aus `/OCGs`.
    ocgs: Vec<ObjectIdentifier>,
    /// Anzeigename je Gruppe (Verschachtelung aus `/Order` als „Eltern / Kind“).
    names: HashMap<ObjectIdentifier, String>,
    /// In der Vorgabe (`/D`) ausgeschaltete Gruppen.
    hidden: HashSet<ObjectIdentifier>,
    /// Katalog: ID und Rohdaten (`<< … >>`).
    root: ObjectIdentifier,
    catalog: Vec<u8>,
    /// Offset der letzten Querverweistabelle und erste freie Objektnummer.
    prev: usize,
    free_id: i32,
}

/// Ebene einer Seite: Gruppe (`None` = Inhalt außerhalb jeder Gruppe), Name, ausgeblendet.
pub(crate) struct LayerPass {
    pub target: Option<ObjectIdentifier>,
    pub name: Option<String>,
    pub hidden: bool,
}

/// Zerlegte Seite: dekodierter Inhalt, Befehle und die Ebenen (unten → oben).
pub(crate) struct PagePlan {
    content: Vec<u8>,
    segs: Vec<Seg>,
    tail: usize,
    streams: Vec<ObjectIdentifier>,
    pub layers: Vec<LayerPass>,
}

struct Seg {
    range: Range<usize>,
    /// `None` = Klammerbefehl (BDC/BMC/EMC), nie eingeklammert.
    owner: Option<Option<ObjectIdentifier>>,
}

/// PDF-Textzeichenkette (UTF-16BE mit BOM, UTF-8 mit BOM, sonst PDFDocEncoding ≈ Latin-1).
fn text_string(b: &[u8]) -> String {
    if let Some(r) = b.strip_prefix(&[0xfe, 0xff]) {
        let u: Vec<u16> = r
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_be_bytes(*c))
            .collect();
        String::from_utf16_lossy(&u)
    } else if let Some(r) = b.strip_prefix(&[0xef, 0xbb, 0xbf]) {
        String::from_utf8_lossy(r).into_owned()
    } else {
        b.iter().map(|&c| c as char).collect()
    }
}

fn find_last(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).rposition(|w| w == needle)
}

fn parse_uint_after(data: &[u8], at: usize) -> Option<usize> {
    let s = &data[at..];
    let start = s.iter().position(|c| !c.is_ascii_whitespace())?;
    let digits: Vec<u8> = s[start..]
        .iter()
        .take_while(|c| c.is_ascii_digit())
        .copied()
        .collect();
    std::str::from_utf8(&digits).ok()?.parse().ok()
}

impl OcInfo {
    /// Optionale Inhalte lesen; `None` ohne Gruppen, bei verschlüsselten oder unlesbaren Dateien.
    pub(crate) fn read(pdf: &Pdf, data: &[u8]) -> Option<Self> {
        let xref = pdf.xref();
        if !xref.has_optional_content_groups() || find_last(data, b"/Encrypt").is_some() {
            return None;
        }
        let root = xref.root_id();
        let cat = xref.get::<Dict<'_>>(root)?;
        let props = cat.get::<Dict<'_>>(b"OCProperties")?;
        let ocgs: Vec<ObjectIdentifier> = props
            .get::<Array<'_>>(b"OCGs")?
            .raw_iter()
            .filter_map(|o| o.as_obj_ref().map(Into::into))
            .collect();
        if ocgs.is_empty() {
            return None;
        }
        let mut names = HashMap::new();
        for id in &ocgs {
            let n = xref
                .get::<Dict<'_>>(*id)
                .and_then(|d| d.get::<hayro_svg::hayro_syntax::object::String<'_>>(b"Name"))
                .map(|s| text_string(s.as_bytes()))
                .unwrap_or_default();
            names.insert(*id, n.trim().to_string());
        }
        let mut hidden = HashSet::new();
        if let Some(cfg) = props.get::<Dict<'_>>(b"D") {
            let refs = |k: &[u8]| -> Vec<ObjectIdentifier> {
                cfg.get::<Array<'_>>(k)
                    .map(|a| {
                        a.raw_iter()
                            .filter_map(|o| o.as_obj_ref().map(Into::into))
                            .collect()
                    })
                    .unwrap_or_default()
            };
            if cfg
                .get::<hayro_svg::hayro_syntax::object::Name<'_>>(b"BaseState")
                .is_some_and(|n| n.as_ref() == b"OFF")
            {
                hidden.extend(ocgs.iter().copied());
            }
            for id in refs(b"ON") {
                hidden.remove(&id);
            }
            hidden.extend(refs(b"OFF"));
            // Verschachtelung aus /Order: Kinder heißen „Eltern / Kind“
            if let Some(order) = cfg.get::<Array<'_>>(b"Order") {
                let mut paths = HashMap::new();
                order_paths(pdf, &order, "", &names, &mut paths, 0);
                names.extend(paths);
            }
        }
        let start = find_last(data, b"startxref")?;
        let prev = parse_uint_after(data, start + 9)?;
        let size = find_last(data, b"/Size")
            .and_then(|p| parse_uint_after(data, p + 5))
            .unwrap_or(9_999_998);
        let free_id = ocgs
            .iter()
            .chain([&root])
            .map(|i| i.obj_number + 1)
            .fold(i32::try_from(size).ok()?.max(1), i32::max);
        let catalog = cat.data().to_vec();
        if !catalog.starts_with(b"<<") {
            return None;
        }
        Some(Self {
            ocgs,
            names,
            hidden,
            root,
            catalog,
            prev,
            free_id,
        })
    }

    /// Seite zerlegen; `None`, wenn sie keine Gruppe direkt nutzt oder nicht zerlegbar ist.
    pub(crate) fn plan(&self, page: &Page<'_>, default_name: &str) -> Option<PagePlan> {
        let xref = page.xref();
        let raw = page.raw();
        // Inhaltsströme (immer indirekt)
        let streams: Vec<ObjectIdentifier> = if let Some(r) = raw.get_ref(b"Contents") {
            let id: ObjectIdentifier = r.into();
            if xref.get::<Stream<'_>>(id).is_some() {
                vec![id]
            } else {
                xref.get::<Array<'_>>(id)?
                    .raw_iter()
                    .map(|o| o.as_obj_ref().map(Into::into))
                    .collect::<Option<Vec<_>>>()?
            }
        } else {
            raw.get::<Array<'_>>(b"Contents")?
                .raw_iter()
                .map(|o| o.as_obj_ref().map(Into::into))
                .collect::<Option<Vec<_>>>()?
        };
        if streams.is_empty() {
            return None;
        }
        let content = page.page_stream()?.to_vec();
        let res = page.resources();
        let is_ocg = |id: ObjectIdentifier| -> bool {
            self.ocgs.contains(&id)
                || xref
                    .get::<Dict<'_>>(id)
                    .and_then(|d| d.get::<hayro_svg::hayro_syntax::object::Name<'_>>(b"Type"))
                    .is_some_and(|t| t.as_ref() == b"OCG")
        };
        let base = content.as_ptr() as usize;
        let mut segs = Vec::new();
        let mut stack: Vec<Option<ObjectIdentifier>> = Vec::new();
        // erste Zeichenstelle je Ebene (für die Stapelfolge)
        let mut first: Vec<(Option<ObjectIdentifier>, usize)> = Vec::new();
        let mut prev_end = 0usize;
        let mut it = UntypedIter::new(&content);
        while let Some(ins) = it.next() {
            let op: &[u8] = &ins.operator[..];
            let p = (op.as_ptr() as usize).checked_sub(base)?;
            let mut end = p + op.len();
            if end > content.len() || p < prev_end {
                return None;
            }
            let cur = stack.last().copied().flatten();
            let owner = match op {
                b"BDC" => {
                    let mut ops = ins.operands();
                    let _tag = ops.next();
                    let oc = match ops.next() {
                        Some(Object::Name(n)) => res
                            .properties
                            .get_ref(n.as_ref())
                            .map(ObjectIdentifier::from),
                        Some(Object::Dict(d)) => d.get_ref(b"OC").map(ObjectIdentifier::from),
                        _ => None,
                    };
                    stack.push(oc.filter(|id| is_ocg(*id)).or(cur));
                    None
                }
                b"BMC" => {
                    stack.push(cur);
                    None
                }
                b"EMC" => {
                    stack.pop();
                    None
                }
                _ => {
                    let mut owner = cur;
                    if op == b"Do"
                        && let Some(Object::Name(n)) = ins.operands().next()
                        && let Some(x) = res.x_objects.get::<Stream<'_>>(n.as_ref())
                        && let Some(r) = x.dict().get_ref(b"OC")
                        && is_ocg(r.into())
                    {
                        owner = Some(r.into());
                    }
                    if op == b"BI" {
                        // Ende hinter „EI“ der Bilddaten
                        let s = ins.operands().find_map(|o| o.clone().into_stream())?;
                        let d = s.raw_data();
                        let e = (d.as_ptr() as usize).checked_sub(base)? + d.len();
                        if !content.get(e..)?.starts_with(b"EI") {
                            return None;
                        }
                        end = e + 2;
                    }
                    if matches!(
                        op,
                        b"f" | b"F"
                            | b"f*"
                            | b"B"
                            | b"B*"
                            | b"b"
                            | b"b*"
                            | b"S"
                            | b"s"
                            | b"sh"
                            | b"Do"
                            | b"Tj"
                            | b"TJ"
                            | b"'"
                            | b"\""
                            | b"BI"
                    ) && !first.iter().any(|(o, _)| *o == owner)
                    {
                        first.push((owner, segs.len()));
                    }
                    Some(owner)
                }
            };
            segs.push(Seg {
                range: prev_end..end,
                owner,
            });
            prev_end = end;
        }
        if !first.iter().any(|(o, _)| o.is_some()) || first.len() > MAX_LAYERS {
            return None;
        }
        let layers = first
            .into_iter()
            .map(|(target, _)| match target {
                None => LayerPass {
                    target,
                    name: Some(default_name.to_string()),
                    hidden: false,
                },
                Some(id) => LayerPass {
                    target,
                    name: self.names.get(&id).filter(|n| !n.is_empty()).cloned(),
                    hidden: self.hidden.contains(&id),
                },
            })
            .collect();
        Some(PagePlan {
            content,
            segs,
            tail: prev_end,
            streams,
            layers,
        })
    }

    /// Kopie der Datei, in der auf der geplanten Seite nur `target` gezeichnet wird.
    pub(crate) fn pass_pdf(
        &self,
        data: &[u8],
        plan: &PagePlan,
        target: Option<ObjectIdentifier>,
    ) -> Vec<u8> {
        let off = self.free_id;
        // Inhaltsstrom: fremde Befehlsfolgen in eine ausgeschaltete Klammer
        let mut c = Vec::with_capacity(plan.content.len() + 256);
        let mut open = false;
        for s in &plan.segs {
            let wrap = matches!(s.owner, Some(o) if o != target);
            if wrap && !open {
                c.extend_from_slice(format!(" /OC <</OC {off} 0 R>> BDC ").as_bytes());
                open = true;
            } else if !wrap && open {
                c.extend_from_slice(b" EMC ");
                open = false;
            }
            c.extend_from_slice(&plan.content[s.range.clone()]);
        }
        if open {
            c.extend_from_slice(b" EMC ");
        }
        c.extend_from_slice(&plan.content[plan.tail..]);

        // Inkrementelle Aktualisierung: Katalog + Inhaltsströme
        let mut out = data.to_vec();
        out.extend_from_slice(b"\n");
        let mut entries: Vec<(ObjectIdentifier, usize)> = Vec::new();
        // Katalog mit eigener Vorgabe: alle Gruppen an (auch ausgeblendete, sie werden als
        // ausgeblendete Ebenen eingelesen), nur die Hilfsgruppe aus
        entries.push((self.root, out.len()));
        let refs = |ids: &mut dyn Iterator<Item = &ObjectIdentifier>| -> String {
            ids.map(|i| format!("{} {} R", i.obj_number, i.gen_number))
                .collect::<Vec<_>>()
                .join(" ")
        };
        let ocgs = refs(&mut self.ocgs.iter());
        let mut old = self.catalog.clone();
        if let Some(p) = find_last(&old, b"/OCProperties") {
            old.splice(p..p + 13, b"/FpOldOCProps".iter().copied());
        }
        out.extend_from_slice(
            format!(
                "{} {} obj\n<< /OCProperties << /OCGs [{ocgs} {off} 0 R] /D << /BaseState /ON /OFF [{off} 0 R] >> >>",
                self.root.obj_number, self.root.gen_number
            )
            .as_bytes(),
        );
        out.extend_from_slice(&old[2..]);
        out.extend_from_slice(b"\nendobj\n");
        for (k, id) in plan.streams.iter().enumerate() {
            entries.push((*id, out.len()));
            let body: &[u8] = if k == 0 { &c } else { b"" };
            out.extend_from_slice(
                format!(
                    "{} {} obj\n<< /Length {} >>\nstream\n",
                    id.obj_number,
                    id.gen_number,
                    body.len()
                )
                .as_bytes(),
            );
            out.extend_from_slice(body);
            out.extend_from_slice(b"\nendstream\nendobj\n");
        }
        entries.sort_by_key(|(id, _)| id.obj_number);
        entries.dedup_by_key(|(id, _)| id.obj_number);
        let xref_at = out.len();
        out.extend_from_slice(b"xref\n0 1\n0000000000 65535 f \n");
        for (id, at) in &entries {
            out.extend_from_slice(
                format!("{} 1\n{:010} {:05} n \n", id.obj_number, at, id.gen_number).as_bytes(),
            );
        }
        out.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root {} {} R /Prev {} >>\nstartxref\n{xref_at}\n%%EOF\n",
                off + 1,
                self.root.obj_number,
                self.root.gen_number,
                self.prev
            )
            .as_bytes(),
        );
        out
    }
}

/// Namen aus `/Order`: verschachtelte Felder sind Kinder der vorangehenden Gruppe, ein Feld mit
/// führender Zeichenkette ist eine benannte Sammlung.
fn order_paths(
    pdf: &Pdf,
    arr: &Array<'_>,
    prefix: &str,
    names: &HashMap<ObjectIdentifier, String>,
    out: &mut HashMap<ObjectIdentifier, String>,
    depth: u32,
) {
    if depth > 8 {
        return;
    }
    let mut last: Option<String> = None;
    let mut items = arr.raw_iter().peekable();
    // benannte Sammlung: erste Angabe ist eine Zeichenkette
    let mut prefix = prefix.to_string();
    if let Some(MaybeRef::NotRef(Object::String(s))) = items.peek() {
        let label = text_string(s.as_bytes());
        prefix = format!("{prefix}{} / ", label.trim());
        items.next();
    }
    for item in items {
        match item {
            MaybeRef::Ref(r) => {
                let id: ObjectIdentifier = r.into();
                if let Some(n) = names.get(&id) {
                    let full = format!("{prefix}{n}");
                    out.insert(id, full.clone());
                    last = Some(full);
                } else if let Some(a) = pdf.xref().get::<Array<'_>>(id) {
                    let p = child_prefix(&a, &prefix, last.as_deref());
                    order_paths(pdf, &a, &p, names, out, depth + 1);
                }
            }
            MaybeRef::NotRef(Object::Array(a)) => {
                let p = child_prefix(&a, &prefix, last.as_deref());
                order_paths(pdf, &a, &p, names, out, depth + 1);
            }
            _ => {}
        }
    }
}

/// Präfix eines verschachtelten Felds: benannte Sammlungen stehen neben der vorigen Gruppe,
/// sonst sind es deren Kinder.
fn child_prefix(a: &Array<'_>, prefix: &str, last: Option<&str>) -> String {
    let label = matches!(
        a.raw_iter().next(),
        Some(MaybeRef::NotRef(Object::String(_)))
    );
    match last {
        Some(l) if !label => format!("{l} / "),
        _ => prefix.to_string(),
    }
}
