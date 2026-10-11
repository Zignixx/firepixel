//! Einstellungen des Renderers und Bericht über ein Rendering.

use std::collections::BTreeMap;

use fp_effects::EffectContext;
use fp_raster::Sampler;

/// Welche gespeicherten Render-Caches (`IMGv`-Kinder) zur Anzeige benutzt werden (02 §10, §12.9,
/// Laufzeit T5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CachePolicy {
    /// Alle gespeicherten Caches (Pfade, Texte, Gruppen, Instanzen, Compound Shapes, Effekt-
    /// ergebnisse) – „Stufe A“ der Referenz `symrender.py`, schnellste Anzeige frisch geladener Dateien.
    All,
    /// Wie das Original nach dem Laden: Gruppen-, Compound-, Text- und Effekt-Caches anzeigen,
    /// Instanz-Caches nur ab `VRSi` 10 und nicht bei Animationssymbolen, **Pfade neu rastern**.
    #[default]
    Original,
    /// Nur Text-Caches und Effektergebnisse von Bitmaps (entspricht `vektor/page.py` auf der
    /// `MKBv`-Kopie, die fast keine anderen Caches trägt).
    TextOnly,
    /// Keine Caches: alles aus dem Modell neu rechnen.
    None,
}

/// Umgang mit der Leinwandfarbe (`BGCi`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackgroundMode {
    /// Wie `Fill`, aber `UnderlayLast`, sobald die Seite ein Element im Mischmodus „Radieren“
    /// (25) enthält (A: Korpus 649 gegen 646 Seiten für `Fill`, Laufzeit 07 T1 „erase“ nur unterlegt exakt).
    #[default]
    Auto,
    /// Leinwand zuerst mit der Leinwandfarbe füllen, dann zeichnen (wie `symrender`/`page.py`).
    Fill,
    /// Auf transparentem Grund zeichnen und die Leinwandfarbe zuletzt unterlegen (Laufzeit 07 T1:
    /// nötig für den Mischmodus „Radieren“).
    UnderlayLast,
    /// Leinwandfarbe ignorieren (transparenter Grund, z. B. für Effekt-Prüfungen und Ebenen-Vorschau).
    Transparent,
}

/// Einstellungen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderOptions {
    pub cache_policy: CachePolicy,
    /// Vektoren und Texte selbst rastern, wenn kein nutzbarer Cache vorliegt. `false` = nur
    /// gespeicherte Pixel (Stufe A): solche Elemente fehlen und werden im Bericht gemeldet.
    pub rasterize: bool,
    pub background: BackgroundMode,
    /// Gruppen ohne Maske/Effekt/Deckkraft isoliert zusammensetzen, außer ein Kind hat einen
    /// Mischmodus (Referenzregel `page.py`, 07 §7.5). `false` = Kinder direkt auf die Leinwand.
    pub isolate_groups: bool,
    /// Live-Effekte rechnen (sonst werden Elemente mit Effekten ohne Cache gemeldet).
    pub effects: bool,
    /// Mischmodi von Elementen innerhalb von Symbolinstanzen anwenden. Vorgabe `false`: das Original'
    /// Instanz-Caches entsprechen der Komposition im Modus Normal (Korpus, siehe STATUS.md).
    pub symbol_blend_modes: bool,
    /// Einstellungen der Live-Effekte.
    pub effect_context: EffectContext,
    /// Abtaster für transformierte Bitmaps und Caches (`AliasTransformMode`, Vorgabe bikubisch).
    pub sampler: Sampler,
    /// Obergrenze des Element-Caches in Pixeln (0 = kein Cache).
    pub cache_budget_pixels: usize,
    /// Seitenkacheln (256 × 256) zwischenspeichern (`tiles`, ein Viertel des Cache-Budgets).
    pub tile_cache: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        RenderOptions {
            cache_policy: CachePolicy::Original,
            rasterize: true,
            background: BackgroundMode::Auto,
            isolate_groups: true,
            effects: true,
            symbol_blend_modes: false,
            effect_context: EffectContext::default(),
            sampler: Sampler::Bicubic,
            cache_budget_pixels: 64 << 20,
            tile_cache: true,
        }
    }
}

impl RenderOptions {
    /// Stufe A der Referenz `symrender.py`: nur gespeicherte Pixel, alle Caches.
    pub fn stored_pixels_only() -> RenderOptions {
        RenderOptions { cache_policy: CachePolicy::All, rasterize: false, effects: false, isolate_groups: false, ..Self::default() }
    }
}

/// Bericht eines Renderings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RenderReport {
    /// Nicht (vollständig) darstellbare Teile: Art → Anzahl (z. B. `PTHv`, `ALSv:symbol-fehlt`,
    /// `effect:photoshop:bevelEmboss`). Leer = vollständig.
    pub missing: BTreeMap<String, usize>,
    /// Näherungen (z. B. resampelte Caches): Art → Anzahl.
    pub approximations: BTreeMap<String, usize>,
    /// Gezeichnete Elemente.
    pub drawn: usize,
    /// Davon selbst gerasterte Pfade bzw. Texte.
    pub rasterized_paths: usize,
    pub rasterized_texts: usize,
    /// Gerechnete Effektketten.
    pub effect_chains: usize,
    /// Aus gespeicherten Caches gezeichnet.
    pub from_file_cache: usize,
}

impl RenderReport {
    /// Vollständig darstellbar?
    pub fn is_complete(&self) -> bool {
        self.missing.is_empty()
    }
    pub(crate) fn miss(&mut self, what: impl Into<String>) {
        *self.missing.entry(what.into()).or_default() += 1;
    }
    pub(crate) fn approx(&mut self, what: impl Into<String>) {
        *self.approximations.entry(what.into()).or_default() += 1;
    }
    pub(crate) fn merge(&mut self, o: &RenderReport) {
        for (k, v) in &o.missing {
            *self.missing.entry(k.clone()).or_default() += v;
        }
        for (k, v) in &o.approximations {
            *self.approximations.entry(k.clone()).or_default() += v;
        }
        self.drawn += o.drawn;
        self.rasterized_paths += o.rasterized_paths;
        self.rasterized_texts += o.rasterized_texts;
        self.effect_chains += o.effect_chains;
        self.from_file_cache += o.from_file_cache;
    }
}
