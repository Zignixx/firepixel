//! Gruppe „Dateien“ (SPEC-BATCH §3.1) und die festen Endkarten: `rename` („Dateiname“, §4),
//! `output` („Speichern als“, §3.2) und `file.copyOnly` („Nur kopieren/verschieben“, ohne
//! Dekodieren, §5.3).
//!
//! Die Endkarten stehen nicht in `recipe.steps`, sondern in `recipe.rename` bzw. `recipe.output`;
//! sie sind hier nur registriert, damit Katalog, Galerie und Formular ihr Schema kennen.

use crate::error::Result;
use crate::item::{Item, ItemKind};
use crate::output::OutputFormat;
use crate::step::{ParamDef, Step, StepContext, StepGroup, StepKind, StepRegistry};
use serde_json::{json, Value};
use std::sync::Arc;

/// ID „Nur kopieren/verschieben“.
pub const COPY_ONLY: &str = "file.copyOnly";
/// ID der Endkarte „Dateiname“.
pub const RENAME: &str = "rename";
/// ID der Endkarte „Speichern als“.
pub const OUTPUT: &str = "output";
/// Option der Endkarte „Speichern als“: zusätzlich eine FW-PNG je Element schreiben (BATCH-11b).
pub const ALSO_FWPNG: &str = "alsoFwpng";

/// Registriert die Schritte der Gruppe.
pub fn register(r: &mut StepRegistry) {
    r.register(Arc::new(CopyOnly));
    r.register(Arc::new(Rename));
    r.register(Arc::new(Output));
}

/// `file.copyOnly`: Datei unverändert kopieren bzw. verschieben (nur mit „Speichern als: wie Quelle“).
struct CopyOnly;

impl Step for CopyOnly {
    fn id(&self) -> &str {
        COPY_ONLY
    }
    fn group(&self) -> StepGroup {
        StepGroup::Files
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![ParamDef::choice("action", &["copy", "move"], "copy").beginner()]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::ALL
    }
    fn preview_ok(&self) -> bool {
        false
    }
    fn file_only(&self) -> bool {
        true
    }
    fn apply(&self, _item: &mut Item, _params: &Value, _ctx: &mut StepContext) -> Result<()> {
        // Die Wirkung (kopieren/verschieben) übernimmt der Executor, weil nichts dekodiert wird.
        Ok(())
    }
    fn summary(&self, params: &Value) -> String {
        match params.get("action").and_then(Value::as_str) {
            Some("move") => fp_i18n::t("batch-summary-file-move"),
            _ => fp_i18n::t("batch-summary-file-copy"),
        }
    }
}

/// Endkarte „Dateiname“.
struct Rename;

impl Step for Rename {
    fn id(&self) -> &str {
        RENAME
    }
    fn group(&self) -> StepGroup {
        StepGroup::Files
    }
    fn kind(&self) -> StepKind {
        StepKind::Final
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::choice("mode", &["builder", "regex"], "builder").beginner(),
            ParamDef::json("parts", json!([{"kind": "original"}])).beginner(),
            ParamDef::json("replacements", json!([])).beginner(),
            ParamDef::text("spaces", "").beginner(),
            ParamDef::text("prefix", "").beginner(),
            ParamDef::text("suffix", "").beginner(),
            ParamDef::choice("case", &["keep", "lower", "upper", "sentence", "title", "kebab", "snake", "camel"], "keep").beginner(),
            ParamDef::bool("transliterate", false),
            ParamDef::bool("stripSpecial", false),
            ParamDef::choice("extension", &["lower", "keep", "upper"], "lower"),
            ParamDef::bool("jpegAsJpg", true),
            ParamDef::json("regex", json!({"find": "", "replace": "", "ignoreCase": false, "all": true, "wholeName": false})),
        ]
    }
    fn preview_ok(&self) -> bool {
        false
    }
    fn apply(&self, _item: &mut Item, _params: &Value, _ctx: &mut StepContext) -> Result<()> {
        Ok(())
    }
}

/// Endkarte „Speichern als“.
struct Output;

impl Step for Output {
    fn id(&self) -> &str {
        OUTPUT
    }
    fn group(&self) -> StepGroup {
        StepGroup::Format
    }
    fn kind(&self) -> StepKind {
        StepKind::Final
    }
    fn params(&self) -> Vec<ParamDef> {
        let formats: Vec<&str> = OutputFormat::ALL.iter().map(|f| f.name()).collect();
        // Felder je Format über `when("format", …)` (BATCH-11b). Gleichnamige Felder (Qualität,
        // Kompression) stehen je Format einmal da – mit eigener Vorgabe und eigenem Bereich, wie der
        // Kodierer sie benutzt; sichtbar ist immer nur eines. GIF-Optionen: Schema `output.gif`
        // (`steps::gifopt`), die Oberfläche hängt es bei Format GIF an.
        let not_fwpng: Vec<&str> = formats.iter().copied().filter(|f| *f != OutputFormat::FwPng.name()).collect();
        vec![
            ParamDef::choice("format", &formats, "same").beginner(),
            ParamDef::int("quality", 1, 100, 85).beginner().when("format", &["jpeg"]),
            ParamDef::int("quality", 0, 100, 82).beginner().when("format", &["webp"]),
            ParamDef::int("quality", 1, 100, 60).beginner().when("format", &["avif"]),
            ParamDef::bool("lossless", false).beginner().when("format", &["webp"]),
            ParamDef::choice("bitDepth", &["auto", "8", "24", "32"], "auto").beginner().when("format", &["png"]),
            ParamDef::int("palette", 2, 256, 256).when("bitDepth", &["8"]),
            ParamDef::bool("interlace", false).when("format", &["png"]),
            ParamDef::bool("progressive", true).when("format", &["jpeg"]),
            ParamDef::choice("subsampling", &["auto", "4:4:4", "4:2:2", "4:2:0"], "auto").when("format", &["jpeg"]),
            ParamDef::color("background", "#ffffff").when("format", &["jpeg", "bmp", "wbmp"]),
            ParamDef::int("speed", 1, 10, 6).when("format", &["avif"]),
            // PNG: fast/default/max (max = verlustfrei optimiert); TIFF: none/lzw/deflate/packbits
            // (default = lzw); TGA: none = unkomprimiert (BATCH-5)
            ParamDef::choice("compression", &["default", "fast", "max"], "default").when("format", &["png"]),
            ParamDef::choice("compression", &["default", "none", "lzw", "deflate", "packbits"], "default").when("format", &["tiff"]),
            ParamDef::choice("compression", &["default", "none"], "default").when("format", &["tga"]),
            // BATCH-11b: zusätzlich eine bearbeitbare FW-PNG neben jede Ausgabe (z. B. SVG + FW-PNG
            // aus „Nachzeichnen“ in einem Lauf; Befund BATCH-7/BATCH-9)
            ParamDef::bool(ALSO_FWPNG, false).when("format", &not_fwpng),
        ]
    }
    fn preview_ok(&self) -> bool {
        false
    }
    fn apply(&self, _item: &mut Item, _params: &Value, _ctx: &mut StepContext) -> Result<()> {
        Ok(())
    }
}
