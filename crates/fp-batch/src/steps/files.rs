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
        vec![
            ParamDef::choice("format", &formats, "same").beginner(),
            ParamDef::int("quality", 0, 100, 85).beginner(),
            ParamDef::bool("lossless", false).beginner(),
            ParamDef::choice("bitDepth", &["auto", "8", "24", "32"], "auto").beginner(),
            ParamDef::int("palette", 2, 256, 256),
            ParamDef::bool("interlace", false),
            ParamDef::bool("progressive", true),
            ParamDef::choice("subsampling", &["auto", "4:4:4", "4:2:2", "4:2:0"], "auto"),
            ParamDef::color("background", "#ffffff"),
            ParamDef::int("colors", 2, 256, 256).beginner(),
            ParamDef::choice("dither", &["none", "ordered", "floydSteinberg", "blueNoise"], "floydSteinberg"),
            ParamDef::int("ditherAmount", 0, 100, 80).unit("%"),
            ParamDef::bool("transparency", true),
            ParamDef::int("speed", 1, 10, 6),
            // PNG: fast/default/max (max = verlustfrei optimiert); TIFF: none/lzw/deflate/packbits
            // (default = lzw); TGA: none = unkomprimiert (BATCH-5)
            ParamDef::choice("compression", &["default", "fast", "max", "none", "lzw", "deflate", "packbits"], "default"),
        ]
    }
    fn preview_ok(&self) -> bool {
        false
    }
    fn apply(&self, _item: &mut Item, _params: &Value, _ctx: &mut StepContext) -> Result<()> {
        Ok(())
    }
}
