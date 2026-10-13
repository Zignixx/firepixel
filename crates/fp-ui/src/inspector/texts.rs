//! Texte des Inspektors (Mehrsprachigkeit): Schlüssel aus `locales\<sprache>\inspector.ftl`
//! (Präfix `pi-`), dazu Anzeigenamen für Listen, deren Daten an anderer Stelle stehen
//! (Mischmodi, Strichvorgaben, Verlaufsformen, Werkzeugnamen).

use fp_engine::commands::panel::{BrushPreset, BRUSH_PRESETS, GRADIENT_SHAPES};

pub use fp_i18n::t;

/// Text zu einem Schlüssel eines fremden Bereichs; fehlt er (noch), der mitgegebene Text.
pub fn t_or(key: &str, fallback: &str) -> String {
    if fp_i18n::has(key) { t(key) } else { fallback.to_string() }
}

/// Texte zu mehreren Schlüsseln.
pub fn tl(keys: &[&str]) -> Vec<String> {
    keys.iter().map(|k| t(k)).collect()
}

/// `&str`-Ansicht für die Kombifelder.
pub fn refs(v: &[String]) -> Vec<&str> {
    v.iter().map(|s| s.as_str()).collect()
}

// ---------------------------------------------------------------------- Mischmodi

/// Anzeigename eines Eintrags der Mischmodus-Liste (Schlüssel `blend-<name>` aus `filter.ftl`,
/// Trenner bleiben Trenner).
pub fn blend_label(i: usize) -> String {
    let Some((de, m)) = super::obj_quads::BLEND_ITEMS.get(i) else { return String::new() };
    if *m < 0 {
        return de.to_string();
    }
    match fp_color::BlendMode(*m as u8).name() {
        Some(n) => t_or(&format!("blend-{n}"), de),
        None => de.to_string(),
    }
}

/// Alle Einträge der Mischmodus-Liste in der aktuellen Sprache.
pub fn blend_labels() -> Vec<String> {
    (0..super::obj_quads::BLEND_ITEMS.len()).map(blend_label).collect()
}

// ---------------------------------------------------------------------- Strichvorgaben

fn slug(s: &str) -> String {
    s.trim_start_matches("bc_").trim_start_matches("bn_").to_lowercase().replace(' ', "-")
}

/// Schlüssel einer Strichkategorie (`bc_Air Brush` → `pi-brush-cat-air-brush`).
pub fn brush_category_key(category: &str) -> String {
    format!("pi-brush-cat-{}", slug(category))
}

/// Anzeigename einer Strichkategorie (interner Name `bc_…`).
pub fn brush_category_label(category: &str) -> String {
    t(&brush_category_key(category))
}

/// Schlüssel einer Strichvorgabe (`bc_Pencil`/`bn_1-Pixel Hard` → `pi-brush-pencil-1-pixel-hard`).
pub fn brush_preset_key(p: &BrushPreset) -> String {
    format!("pi-brush-{}-{}", slug(p.category), slug(p.name))
}

/// Anzeigename einer Strichvorgabe.
pub fn brush_preset_label(p: &BrushPreset) -> String {
    t(&brush_preset_key(p))
}

/// Index der Vorgabe zu einem Pinsel (Kategorie und interner Name).
pub fn brush_preset_index(b: &fp_model::Brush) -> Option<usize> {
    BRUSH_PRESETS.iter().position(|p| p.category == b.category && p.name == b.name)
}

/// Anzeigename eines Pinsels: Vorgabe übersetzt, sonst der eigene Name, sonst der interne Name.
pub fn brush_label(b: &fp_model::Brush) -> String {
    if let Some(i) = brush_preset_index(b) {
        return brush_preset_label(&BRUSH_PRESETS[i]);
    }
    if let Some(u) = &b.user_name {
        return u.clone();
    }
    b.name.trim_start_matches("bn_").to_string()
}

/// Kategorien der Strichvorgaben (interne Namen) in Menüreihenfolge.
pub fn brush_categories() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = Vec::new();
    for b in BRUSH_PRESETS {
        if !v.contains(&b.category) {
            v.push(b.category);
        }
    }
    v
}

// ---------------------------------------------------------------------- Verlaufsformen

/// Anzeigename einer Verlaufsform (Eintrag aus `GRADIENT_SHAPES`, Schlüssel nach dem internen Namen).
pub fn gradient_label(i: usize) -> String {
    GRADIENT_SHAPES.get(i).map(|g| t(&format!("pi-grad-{}", g.3.to_lowercase()))).unwrap_or_default()
}

/// Alle Verlaufsformen in der aktuellen Sprache.
pub fn gradient_labels() -> Vec<String> {
    (0..GRADIENT_SHAPES.len()).map(gradient_label).collect()
}

// ---------------------------------------------------------------------- Werkzeuge

/// Anzeigename eines Werkzeugs (Schlüssel von I18N-MENUE: `tool-<name>` bzw. `cmd-tool-<name>`,
/// sonst der deutsche Name aus fp-engine).
pub fn tool_label(info: &fp_engine::tools::ToolInfo) -> String {
    let short = info.name.trim_start_matches("tool.");
    for key in [format!("tool-{short}"), fp_i18n::command_key(info.name)] {
        if fp_i18n::has(&key) {
            return t(&key);
        }
    }
    info.label.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_of_lists_exist() {
        for p in BRUSH_PRESETS {
            assert!(fp_i18n::has(&brush_preset_key(p)), "{}", brush_preset_key(p));
            assert!(fp_i18n::has(&brush_category_key(p.category)), "{}", brush_category_key(p.category));
        }
        for g in GRADIENT_SHAPES {
            let k = format!("pi-grad-{}", g.3.to_lowercase());
            assert!(fp_i18n::has(&k), "{k}");
        }
        assert_eq!(blend_labels().len(), super::super::obj_quads::BLEND_ITEMS.len());
    }
}
