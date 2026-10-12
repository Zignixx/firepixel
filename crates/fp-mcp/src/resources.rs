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
    ["agents", "commands", "document-model", "examples", "jsf"]
        .into_iter()
        .zip(["firepixel://docs/agents.md", "firepixel://docs/commands.md", "firepixel://docs/document-model.md", "firepixel://docs/examples.md", "firepixel://docs/jsf.md"])
        .map(|(name, uri)| DocResource { uri, name, title: fp_i18n::t(&format!("mcp-res-{name}")), description: fp_i18n::t(&format!("mcp-res-{name}.description")) })
        .collect()
}

/// Inhalt einer Ressource.
pub fn read(uri: &str, registry: &CommandRegistry) -> Option<String> {
    match uri {
        "firepixel://docs/agents.md" => Some(AGENTS_MD.to_string()),
        "firepixel://docs/commands.md" => Some(commands_markdown(registry)),
        "firepixel://docs/document-model.md" => Some(by_language(DOCUMENT_MODEL_MD, DOCUMENT_MODEL_MD_EN).to_string()),
        "firepixel://docs/examples.md" => Some(by_language(EXAMPLES_MD, EXAMPLES_MD_EN).to_string()),
        "firepixel://docs/jsf.md" => Some(fp_script::coverage_markdown()),
        _ => None,
    }
}

/// Anleitung für Agenten (aus `docs/AGENTS.md`).
pub const AGENTS_MD: &str = include_str!("../../../docs/AGENTS.md");

/// Befehlskatalog als Markdown.
pub fn commands_markdown(reg: &CommandRegistry) -> String {
    let mut s = String::new();
    let implemented = reg.iter().filter(|c| c.info().implemented).count();
    let _ = writeln!(s, "# {}\n", fp_i18n::t("msg-mcp-cat-title"));
    let _ = writeln!(s, "{}\n", fp_i18n::tr!("msg-mcp-cat-intro", n = reg.len(), implemented = implemented));
    let mut cats: Vec<String> = reg.iter().map(|c| c.info().name.split('.').next().unwrap_or("").to_string()).collect();
    cats.sort();
    cats.dedup();
    for cat in cats {
        let _ = writeln!(s, "## {cat}\n");
        let mut items: Vec<_> = reg.iter().map(|c| c.info()).filter(|i| i.name.split('.').next() == Some(cat.as_str())).collect();
        items.sort_by(|a, b| a.name.cmp(&b.name));
        for i in items {
            let params: Vec<String> = i
                .params
                .iter()
                .map(|p| {
                    let ty = match &p.kind {
                        fp_engine::ParamKind::Enum { values } => values.join("|"),
                        k => format!("{k:?}").split([' ', '{']).next().unwrap_or("").to_lowercase(),
                    };
                    format!("`{}{}`: {}", p.name, if p.required { "*" } else { "" }, ty)
                })
                .collect();
            let reqs: Vec<String> = i.requires.iter().filter(|r| !matches!(r, fp_engine::Requirement::Document)).map(|r| r.reason()).collect();
            let _ = write!(s, "- **`{}`** – {}", i.name, crate::tools::command_label(i));
            if !i.implemented {
                let _ = write!(s, " _({})_", fp_i18n::t("msg-mcp-cat-unimplemented"));
            }
            if !params.is_empty() {
                let _ = write!(s, " · {}", params.join(", "));
            }
            if !reqs.is_empty() {
                let _ = write!(s, " · {}", fp_i18n::tr!("msg-mcp-cat-needs", list = reqs.join("; ")));
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
    if fp_i18n::current_language() == "de" { de } else { en }
}
