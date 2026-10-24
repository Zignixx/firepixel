//! Effektketten: die Effektliste eines Elements (`EFLv`) in Listenreihenfolge anwenden
//! (erster Eintrag zuerst, jeder Effekt bekommt das Ergebnis des vorigen; 05 §2.6, 06 §2.4).
//!
//! Bitmap-Filter (Weichzeichnen, Schärfen, Tonwerte, …) laufen als Live-Filter über
//! [`fp_raster::Filter::from_effect`] (Rahmen mit zwei Kopien des Eingabebilds, 05 Hinweise).

use fp_raster::filters::PixelFilter;
use fp_raster::{Filter, RgbaImage};

use crate::EffectContext;
use crate::bevel::{BevelParams, bevel};
use crate::color_fill::{ColorFillParams, color_fill};
use crate::inner_glow::{InnerGlowParams, inner_glow};
use crate::params::{EffectParams, Params};
use crate::photoshop::{self, PsEffects};
use crate::screentone::{ScreentoneParams, screentone};
use crate::shadow::{ShadowParams, drop_shadow, inner_shadow};
use crate::solid::{SolidShadowParams, solid_shadow};

/// EffectMoaIDs der Original-eigenen Live-Effekte (kleingeschrieben, mit Klammern).
pub mod moa {
    pub const BEVEL: &str = "{7fe61102-6ce2-11d1-8c76000502701850}";
    pub const DROP_SHADOW: &str = "{a7944db8-6ce2-11d1-8c76000502701850}";
    pub const INNER_SHADOW: &str = "{5600f702-774c-11d3-baad0000861f4d01}";
    pub const INNER_GLOW: &str = "{2ba87123-8220-11d3-baad0000861f4d01}";
    pub const SOLID_SHADOW: &str = "{b90c950e-64df-11d8-aaf2000a9582f7d4}";
    pub const COLOR_FILL: &str = "{dd54adc0-a279-11d3-b92a000502f3fdbe}";
    pub const PHOTOSHOP: &str = "{8eeadf50-9efe-11da-a7460800200c9a66}";
    /// Screentone (Firepixel extension, FB-38; not an effect of the reference).
    pub const SCREENTONE: &str = "{firepixel-screentone}";
}

/// Ein Live-Effekt mit gelesenen Parametern.
#[derive(Debug, Clone, PartialEq)]
pub enum LiveEffect {
    /// Schlagschatten.
    DropShadow(ShadowParams),
    /// Innerer Schatten.
    InnerShadow(ShadowParams),
    /// Abschrägung/Prägung/Glühen.
    Bevel(BevelParams),
    /// Inneres Glühen.
    InnerGlow(InnerGlowParams),
    /// Massiver Schatten.
    SolidShadow(SolidShadowParams),
    /// Farbfüllung.
    ColorFill(ColorFillParams),
    /// Photoshop-Live-Effekte.
    Photoshop(PsEffects),
    /// Screentone / halftone screen (Firepixel extension, FB-38).
    Screentone(ScreentoneParams),
    /// Bitmap-Filter als Live-Effekt.
    Filter(Filter),
    /// Nicht darstellbar (Wolken, Photoshop-Plugins, unbekannte IDs): das Objekt bleibt unverändert.
    Unsupported {
        /// EffectMoaID wie in der Datei.
        moa_id: String,
    },
}

impl LiveEffect {
    /// Effekt aus EffectMoaID und Datei-Parametern.
    pub fn parse(moa_id: &str, params: &EffectParams) -> LiveEffect {
        let id = moa_id.trim().to_ascii_lowercase();
        let id = if id.starts_with('{') {
            id
        } else {
            format!("{{{id}}}")
        };
        let p = Params(params);
        match id.as_str() {
            moa::DROP_SHADOW => {
                LiveEffect::DropShadow(ShadowParams::read(p, ShadowParams::DROP_DEFAULT))
            }
            moa::INNER_SHADOW => {
                LiveEffect::InnerShadow(ShadowParams::read(p, ShadowParams::INNER_DEFAULT))
            }
            moa::BEVEL => LiveEffect::Bevel(BevelParams::read(p)),
            moa::INNER_GLOW => LiveEffect::InnerGlow(InnerGlowParams::read(p)),
            moa::SOLID_SHADOW => LiveEffect::SolidShadow(SolidShadowParams::read(p)),
            moa::COLOR_FILL => LiveEffect::ColorFill(ColorFillParams::read(p)),
            moa::PHOTOSHOP => LiveEffect::Photoshop(PsEffects::read(params)),
            moa::SCREENTONE => LiveEffect::Screentone(ScreentoneParams::read(p)),
            _ => match Filter::from_effect(&id, params) {
                Some(f) => LiveEffect::Filter(f),
                None => LiveEffect::Unsupported {
                    moa_id: moa_id.to_string(),
                },
            },
        }
    }

    /// Kurzname für Meldungen.
    pub fn name(&self) -> &'static str {
        match self {
            LiveEffect::DropShadow(_) => "drop_shadow",
            LiveEffect::InnerShadow(_) => "inner_shadow",
            LiveEffect::Bevel(b) => match b.kind {
                crate::bevel::BevelType::Inner => "inner_bevel",
                crate::bevel::BevelType::Outer => "outer_bevel",
                crate::bevel::BevelType::Raised => "raised_emboss",
                crate::bevel::BevelType::Inset => "inset_emboss",
                crate::bevel::BevelType::Glow => "glow",
            },
            LiveEffect::InnerGlow(_) => "inner_glow",
            LiveEffect::SolidShadow(_) => "solid_shadow",
            LiveEffect::ColorFill(_) => "color_fill",
            LiveEffect::Photoshop(_) => "photoshop",
            LiveEffect::Screentone(_) => "screentone",
            LiveEffect::Filter(_) => "filter",
            LiveEffect::Unsupported { .. } => "unsupported",
        }
    }

    /// Wendet den Effekt an. Liefert das Ergebnis und Meldungen über nicht umgesetzte Teile.
    pub fn apply(&self, obj: &RgbaImage, ctx: &EffectContext) -> (RgbaImage, Vec<String>) {
        match self {
            LiveEffect::DropShadow(p) => (drop_shadow(obj, p, ctx), Vec::new()),
            LiveEffect::InnerShadow(p) => (inner_shadow(obj, p, ctx), Vec::new()),
            LiveEffect::Bevel(p) => (bevel(obj, p), Vec::new()),
            LiveEffect::InnerGlow(p) => (inner_glow(obj, p, ctx), Vec::new()),
            LiveEffect::SolidShadow(p) => (solid_shadow(obj, p, ctx), Vec::new()),
            LiveEffect::ColorFill(p) => (color_fill(obj, p), Vec::new()),
            LiveEffect::Photoshop(p) => {
                let issues = p
                    .unsupported()
                    .into_iter()
                    .map(|n| format!("photoshop:{n}"))
                    .collect();
                (photoshop::render(obj, p), issues)
            }
            LiveEffect::Screentone(p) => (screentone(obj, p), Vec::new()),
            LiveEffect::Filter(f) => (f.apply(obj, None), Vec::new()),
            LiveEffect::Unsupported { moa_id } => {
                (obj.clone(), vec![format!("unsupported:{moa_id}")])
            }
        }
    }

    /// Größte Ausdehnung des Ergebnisses über das Objektrechteck hinaus in px (konservativ), z. B.
    /// um Teil-Renderings und Invalidierungen auf die betroffene Fläche zu begrenzen.
    pub fn max_growth(&self) -> i32 {
        match self {
            LiveEffect::DropShadow(p) => {
                p.blur.min(1024) as i32 + 1 + (p.distance.abs().min(1.0e6).ceil() as i32)
            }
            LiveEffect::InnerShadow(_)
            | LiveEffect::InnerGlow(_)
            | LiveEffect::ColorFill(_)
            | LiveEffect::Screentone(_) => 0,
            LiveEffect::Bevel(b) => b.extent() as i32 + 2,
            LiveEffect::SolidShadow(s) => s.distance.abs().min(1.0e6).ceil() as i32 + 2,
            LiveEffect::Photoshop(p) => p
                .get("dropShadow")
                .map(|d| {
                    (d.num("blur", 0.0).abs() + d.num("distance", 0.0).abs())
                        .min(1.0e6)
                        .ceil() as i32
                        + 2
                })
                .unwrap_or(0),
            LiveEffect::Filter(f) => {
                let r = fp_geom::IRect::new(0, 0, 1, 1);
                let o = f.output_bounds(r);
                (-o.left)
                    .max(-o.top)
                    .max(o.right - 1)
                    .max(o.bottom - 1)
                    .max(0)
            }
            LiveEffect::Unsupported { .. } => 0,
        }
    }
}

/// Ein Eintrag der Effektliste.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectEntry {
    pub effect: LiveEffect,
    /// `EffectIsVisible`.
    pub visible: bool,
}

/// Ergebnis einer Effektkette.
#[derive(Debug, Clone)]
pub struct ChainResult {
    /// Bild nach allen sichtbaren Effekten (Dokumentkoordinaten).
    pub image: RgbaImage,
    /// Nicht umgesetzte Effekte bzw. Teile.
    pub issues: Vec<String>,
}

/// Wendet eine Effektliste in Listenreihenfolge an; unsichtbare Effekte werden übersprungen.
pub fn apply_chain(obj: &RgbaImage, effects: &[EffectEntry], ctx: &EffectContext) -> ChainResult {
    let mut cur = obj.clone();
    let mut issues = Vec::new();
    for e in effects.iter().filter(|e| e.visible) {
        let (img, mut is) = e.effect.apply(&cur, ctx);
        cur = img;
        issues.append(&mut is);
    }
    ChainResult { image: cur, issues }
}

/// Konservative Gesamtausdehnung einer Effektliste über das Objektrechteck hinaus.
pub fn chain_growth(effects: &[EffectEntry]) -> i32 {
    effects
        .iter()
        .filter(|e| e.visible)
        .map(|e| e.effect.max_growth())
        .fold(0i32, |a, g| a.saturating_add(g))
}
