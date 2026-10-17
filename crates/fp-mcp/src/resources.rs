//! Eingebaute Doku als MCP-Ressourcen (Markdown).

use fp_engine::CommandRegistry;
use std::fmt::Write as _;

/// Eine Ressource (Titel und Beschreibung in der aktuellen Sprache, `mcp-res-<name>`).
#[derive(Clone, Debug)]
pub struct DocResource {
    pub uri: &'static str,
    pub name: &'static str,
    pub title: String,
    pub description: String,
}

/// Alle Ressourcen.
pub fn resources() -> Vec<DocResource> {
    [
        "agents",
        "commands",
        "document-model",
        "examples",
        "jsf",
        "batch",
    ]
    .into_iter()
    .zip([
        "firepixel://docs/agents.md",
        "firepixel://docs/commands.md",
        "firepixel://docs/document-model.md",
        "firepixel://docs/examples.md",
        "firepixel://docs/jsf.md",
        crate::batch::RESOURCE_URI,
    ])
    .map(|(name, uri)| DocResource {
        uri,
        name,
        title: fp_i18n::t(&format!("mcp-res-{name}")),
        description: fp_i18n::t(&format!("mcp-res-{name}.description")),
    })
    .collect()
}

/// Inhalt einer Ressource.
pub fn read(uri: &str, registry: &CommandRegistry) -> Option<String> {
    match uri {
        "firepixel://docs/agents.md" => Some(by_language(AGENTS_MD, AGENTS_MD_EN).to_string()),
        "firepixel://docs/commands.md" => Some(commands_markdown(registry)),
        "firepixel://docs/document-model.md" => {
            Some(by_language(DOCUMENT_MODEL_MD, DOCUMENT_MODEL_MD_EN).to_string())
        }
        "firepixel://docs/examples.md" => {
            Some(by_language(EXAMPLES_MD, EXAMPLES_MD_EN).to_string())
        }
        "firepixel://docs/jsf.md" => Some(jsf_markdown()),
        crate::batch::RESOURCE_URI => Some(crate::batch::batch_markdown()), // BATCH-9
        _ => None,
    }
}

/// Anleitung für Agenten (aus `docs/AGENTS.md`).
pub const AGENTS_MD: &str = include_str!("../../../docs/AGENTS.md");
/// Anleitung für Agenten, englisch (aus `docs/AGENTS.en.md`).
pub const AGENTS_MD_EN: &str = include_str!("../../../docs/AGENTS.en.md");

/// Schlüssel der Begründung einer bewusst ausgelassenen JSF-Klasse/-Methode
/// (`mcp-res-jsf-why-<klasse>-<member>`, Kleinbuchstaben, sonst Bindestriche).
pub fn jsf_reason_key(class: &str, member: &str) -> String {
    let mut k = String::from("mcp-res-jsf-why");
    for part in [class, member] {
        k.push('-');
        let mut last_dash = true;
        for c in part.chars().flat_map(char::to_lowercase) {
            if c.is_ascii_alphanumeric() {
                k.push(c);
                last_dash = false;
            } else if !last_dash {
                k.push('-');
                last_dash = true;
            }
        }
        while k.ends_with('-') {
            k.pop();
        }
    }
    k
}

/// JSF-Kompatibilitätsliste (Ressource `firepixel://docs/jsf.md`) in der aktuellen Sprache: dieselben
/// Zahlen und Listen wie [`fp_script::coverage_markdown`], Deutsch wortgleich mit ihr.
pub fn jsf_markdown() -> String {
    use fp_script::coverage::{EXCEPTIONS, METHODS, PROPS};
    let listed = |t: &[(&str, &[&str])], c: &str, n: &str| {
        t.iter()
            .any(|(k, ns)| *k == c && (ns.contains(&n) || ns.contains(&"*")))
    };
    let excepted = |c: &str, n: &str| {
        EXCEPTIONS
            .iter()
            .any(|(k, m, _)| *k == c && (*m == n || *m == "*"))
    };
    let why = |c: &str, m: &str, de: &str| {
        let key = jsf_reason_key(c, m);
        if fp_i18n::has(&key) {
            fp_i18n::t(&key)
        } else {
            de.to_string()
        }
    };
    let (m, mt, p, pt) = fp_script::coverage();
    let (em, ep) = fp_script::coverage::exceptions();
    let mut s = String::new();
    let _ = writeln!(s, "# {}\n", fp_i18n::t("mcp-res-jsf-title"));
    let _ = writeln!(
        s,
        "{}\n",
        fp_i18n::tr!(
            "mcp-res-jsf-intro",
            m = m,
            mt = mt,
            p = p,
            pt = pt,
            em = em,
            ep = ep
        )
    );
    let _ = writeln!(s, "{}\n", fp_i18n::t("mcp-res-jsf-rules"));
    let _ = writeln!(s, "{}\n", fp_i18n::t("mcp-res-jsf-extras"));
    for c in fp_script::bindings::CLASSES {
        let mi: Vec<&str> = c
            .methods
            .iter()
            .filter(|x| listed(METHODS, c.js, x.name) && !excepted(c.js, x.name))
            .map(|x| x.name)
            .collect();
        let pi: Vec<&str> = c
            .props
            .iter()
            .filter(|x| listed(PROPS, c.js, x.name) && !excepted(c.js, x.name))
            .map(|x| x.name)
            .collect();
        if mi.is_empty() && pi.is_empty() {
            let line = match EXCEPTIONS.iter().find(|(k, m, _)| *k == c.js && *m == "*") {
                Some((k, m, de)) => fp_i18n::tr!(
                    "mcp-res-jsf-class-omitted",
                    class = c.js,
                    reason = why(k, m, de)
                ),
                None => fp_i18n::tr!(
                    "mcp-res-jsf-class-missing",
                    class = c.js,
                    methods = c.methods.len(),
                    props = c.props.len()
                ),
            };
            let _ = writeln!(s, "- {line}");
            continue;
        }
        let counts = fp_i18n::tr!(
            "mcp-res-jsf-class-counts",
            class = c.js,
            m = mi.len(),
            mt = c.methods.len(),
            p = pi.len(),
            pt = c.props.len()
        );
        let _ = writeln!(s, "- {counts}");
        if !mi.is_empty() {
            let _ = writeln!(
                s,
                "  - {}",
                fp_i18n::tr!("mcp-res-jsf-methods", list = mi.join(", "))
            );
        }
        if !pi.is_empty() {
            let _ = writeln!(
                s,
                "  - {}",
                fp_i18n::tr!("mcp-res-jsf-props", list = pi.join(", "))
            );
        }
        for (k, m, de) in EXCEPTIONS
            .iter()
            .filter(|(k, m, _)| *k == c.js && *m != "*")
        {
            let _ = writeln!(
                s,
                "  - {}",
                fp_i18n::tr!(
                    "mcp-res-jsf-member-omitted",
                    member = *m,
                    reason = why(k, m, de)
                )
            );
        }
    }
    s
}

/// Befehlskatalog als Markdown.
pub fn commands_markdown(reg: &CommandRegistry) -> String {
    let mut s = String::new();
    let implemented = reg.iter().filter(|c| c.info().implemented).count();
    let _ = writeln!(s, "# {}\n", fp_i18n::t("msg-mcp-cat-title"));
    let _ = writeln!(
        s,
        "{}\n",
        fp_i18n::tr!(
            "msg-mcp-cat-intro",
            n = reg.len(),
            implemented = implemented
        )
    );
    let mut cats: Vec<String> = reg
        .iter()
        .map(|c| c.info().name.split('.').next().unwrap_or("").to_string())
        .collect();
    cats.sort();
    cats.dedup();
    for cat in cats {
        let _ = writeln!(s, "## {cat}\n");
        let mut items: Vec<_> = reg
            .iter()
            .map(|c| c.info())
            .filter(|i| i.name.split('.').next() == Some(cat.as_str()))
            .collect();
        items.sort_by(|a, b| a.name.cmp(&b.name));
        for i in items {
            let params: Vec<String> = i
                .params
                .iter()
                .map(|p| {
                    let ty = match &p.kind {
                        fp_engine::ParamKind::Enum { values } => values.join("|"),
                        k => format!("{k:?}")
                            .split([' ', '{'])
                            .next()
                            .unwrap_or("")
                            .to_lowercase(),
                    };
                    format!("`{}{}`: {}", p.name, if p.required { "*" } else { "" }, ty)
                })
                .collect();
            let reqs: Vec<String> = i
                .requires
                .iter()
                .filter(|r| !matches!(r, fp_engine::Requirement::Document))
                .map(|r| r.reason())
                .collect();
            let _ = write!(s, "- **`{}`** – {}", i.name, crate::tools::command_label(i));
            if !i.implemented {
                let _ = write!(s, " _({})_", fp_i18n::t("msg-mcp-cat-unimplemented"));
            }
            if !params.is_empty() {
                let _ = write!(s, " · {}", params.join(", "));
            }
            if !reqs.is_empty() {
                let _ = write!(
                    s,
                    " · {}",
                    fp_i18n::tr!("msg-mcp-cat-needs", list = reqs.join("; "))
                );
            }
            let _ = writeln!(s);
        }
        let _ = writeln!(s);
    }
    s
}

/// Überblick Dokumentmodell (Deutsch).
pub const DOCUMENT_MODEL_MD: &str = include_str!("../docs/de/document-model.md");
/// Überblick Dokumentmodell (Englisch).
pub const DOCUMENT_MODEL_MD_EN: &str = include_str!("../docs/en/document-model.md");

/// Beispiele (Deutsch).
pub const EXAMPLES_MD: &str = include_str!("../docs/de/examples.md");
/// Beispiele (Englisch).
pub const EXAMPLES_MD_EN: &str = include_str!("../docs/en/examples.md");

/// Markdown-Dokument in der aktuellen Sprache: Deutsch für `de`, sonst Englisch (Dokumente mit
/// JSON-Beispielen passen nicht in Fluent-Dateien; weitere Sprachen fallen auf Englisch zurück).
fn by_language(de: &'static str, en: &'static str) -> &'static str {
    if fp_i18n::current_language() == "de" {
        de
    } else {
        en
    }
}
