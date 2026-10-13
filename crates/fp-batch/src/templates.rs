//! Eingebaute Rezept-Vorlagen (SPEC-BATCH §2.5, Paket BATCH-9).
//!
//! Die Vorlagen liegen als gewöhnliche Rezeptdateien in `crates\fp-batch\data\recipes\<id>.fpbatch.json`
//! (Schlüssel englisch, Name/Beschreibung dort nur als englischer Rückfall) und sind in die exe
//! eingebettet. Titel und Beschreibung in der Oberflächensprache: `batch-template-<id>` und
//! `batch-template-<id>.description` (`locales\<code>\batch_vector.ftl`).
//!
//! Nutzung: Menü „Rezepte ▸ Vorlagen“ im Batch-Studio (fp-ui meldet [`all`] an), CLI
//! `firepixel-cli batch --recipe <id>` / `--templates`, `batch.run {"recipe": "<id>"}` über MCP/JSF.

use crate::recipe::Recipe;
use crate::step::StepGroup;
use serde_json::{json, Value};

/// Eine eingebaute Vorlage.
#[derive(Clone, Debug)]
pub struct Template {
    /// Technische ID (englisch, = Dateiname ohne `.fpbatch.json`).
    pub id: &'static str,
    /// Titel in der aktuellen Sprache.
    pub title: String,
    /// Ein Satz in der aktuellen Sprache.
    pub description: String,
    /// Gruppe für Symbol und Farbe der Karte.
    pub group: StepGroup,
    /// Das Rezept (Name/Beschreibung in der aktuellen Sprache).
    pub recipe: Recipe,
}

/// (ID, Gruppe, Inhalt) in der Reihenfolge des Menüs.
const BUILTIN: [(&str, StepGroup, &str); 12] = [
    ("web-webp", StepGroup::Geometry, include_str!("../data/recipes/web-webp.fpbatch.json")),
    ("email-photos", StepGroup::Geometry, include_str!("../data/recipes/email-photos.fpbatch.json")),
    ("save-as-jpeg", StepGroup::Format, include_str!("../data/recipes/save-as-jpeg.fpbatch.json")),
    ("remove-background", StepGroup::Background, include_str!("../data/recipes/remove-background.fpbatch.json")),
    ("transparency-to-white", StepGroup::Background, include_str!("../data/recipes/transparency-to-white.fpbatch.json")),
    ("logos-to-svg", StepGroup::Advanced, include_str!("../data/recipes/logos-to-svg.fpbatch.json")),
    ("small-gifs", StepGroup::Animation, include_str!("../data/recipes/small-gifs.fpbatch.json")),
    ("video-to-gif", StepGroup::Animation, include_str!("../data/recipes/video-to-gif.fpbatch.json")), // BATCH-11b
    ("strip-metadata", StepGroup::Metadata, include_str!("../data/recipes/strip-metadata.fpbatch.json")),
    ("rename-by-date", StepGroup::Files, include_str!("../data/recipes/rename-by-date.fpbatch.json")),
    ("square-1080", StepGroup::Geometry, include_str!("../data/recipes/square-1080.fpbatch.json")),
    ("thumbnails-256", StepGroup::Geometry, include_str!("../data/recipes/thumbnails-256.fpbatch.json")),
];

/// IDs aller eingebauten Vorlagen (Menüreihenfolge).
pub fn ids() -> Vec<&'static str> {
    BUILTIN.iter().map(|b| b.0).collect()
}

fn text_or(key: &str, fallback: &str) -> String {
    if fp_i18n::has(key) { fp_i18n::t(key) } else { fallback.to_string() }
}

fn build(id: &'static str, group: StepGroup, json: &str) -> Option<Template> {
    let mut recipe = Recipe::from_json(json).ok()?;
    let key = format!("batch-template-{id}");
    let title = text_or(&key, &recipe.name);
    let description = text_or(&format!("{key}.description"), &recipe.description);
    recipe.name = title.clone();
    recipe.description = description.clone();
    Some(Template { id, title, description, group, recipe })
}

/// Alle eingebauten Vorlagen in der aktuellen Sprache.
pub fn all() -> Vec<Template> {
    BUILTIN.iter().filter_map(|(id, g, j)| build(id, *g, j)).collect()
}

/// Vorlage nach ID (ohne Groß-/Kleinschreibung, `_` = `-`, auch mit `.fpbatch.json`) oder nach
/// Titel in Deutsch, Englisch bzw. der aktuellen Sprache.
pub fn find(name: &str) -> Option<Template> {
    let norm = |s: &str| s.trim().to_lowercase().replace('_', "-");
    let n = norm(name.trim_end_matches(".fpbatch.json"));
    if n.is_empty() {
        return None;
    }
    if let Some((id, g, j)) = BUILTIN.iter().find(|b| b.0 == n) {
        return build(id, *g, j);
    }
    let lang = fp_i18n::current_language();
    for code in [lang.as_str(), "de", "en"] {
        let hit = fp_i18n::with_language(code, || BUILTIN.iter().find(|(id, _, _)| norm(&text_or(&format!("batch-template-{id}"), "")) == n).map(|b| b.0));
        if let Some(id) = hit {
            return BUILTIN.iter().find(|b| b.0 == id).and_then(|(id, g, j)| build(id, *g, j));
        }
    }
    None
}

/// Rezept einer Vorlage (siehe [`find`]).
pub fn recipe(name: &str) -> Option<Recipe> {
    find(name).map(|t| t.recipe)
}

/// Katalog für Agenten, CLI `batch --templates --json` und das MCP-Werkzeug `batch`.
pub fn catalog_json() -> Value {
    let list: Vec<Value> = all()
        .into_iter()
        .map(|t| {
            json!({
                "id": t.id,
                "title": t.title,
                "description": t.description,
                "group": t.group.name(),
                "steps": t.recipe.steps.iter().map(|s| s.id.clone()).collect::<Vec<_>>(),
                "format": t.recipe.output.format,
                "recipe": serde_json::to_value(&t.recipe).unwrap_or(Value::Null),
            })
        })
        .collect();
    json!({"format": "firepixel-batch-templates", "version": crate::recipe::RECIPE_VERSION, "templates": list})
}
