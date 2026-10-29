//! Dokument-Compositor (CPU-Referenz) von Firepixel: Seite × Status → [`fp_raster::RgbaImage`].
//!
//! Grundlage: `re\13-symbole.md` (Komposition, Symbole, 9-Slice, Animation, Laufzeit T1–T7),
//! `re\02-fwpng-format.md` §4.3/§10/§12.9 (Zeichenreihenfolge, Caches, Masterseite), `re\04-objektmodell.md`
//! §1.6b/§2.5/§6.1 (Masken, geteilte Ebenen zeigen Cel 0, Unterebenen `cels[0]`), `re\07-vektor-rendering.md`
//! §7 und §12.1 (Mischmodi, Masken, Gruppen-Isolation, Farb-Alpha, keine Ebenen-Deckkraft in CS6);
//! Referenzen `re\symbole\symrender.py` und `re\vektor\page.py`.
//!
//! # Aufbau
//!
//! | Modul | Inhalt |
//! |---|---|
//! | [`compose`] | [`Renderer`]: Seiten, Ausschnitte mit Zoom, Symbole, einzelne Elemente; Ebenen, Gruppen, Masken, Instanzen, Effekte |
//! | [`nine`] | Punktabbildungen ([`PointMap`]): Matrizen und 9-Slice je Achse |
//! | [`convert`] | Adapter Modell → `fp-vector`/`fp-text`/`fp-effects`/`fp-raster` |
//! | [`text_edge`] | Feather and Dither fill edges of text objects (the path edge filters on the text coverage) |
//! | [`cache`] | Element-Cache mit Invalidierung über `Arc`-Zeiger und Änderungs-Generation |
//! | [`hit`] | Treffertest (oberstes Element an einem Punkt) |
//! | [`options`] | [`RenderOptions`], [`CachePolicy`], [`BackgroundMode`], [`RenderReport`] |
//! | [`flatten`] | `fp_fwpng::Flattener` für `IDAT` und `prVW` |
//!
//! # Regeln der Komposition
//!
//! - Ebenen unten → oben ([`fp_model::Document::resolved_layers`]: geteilte Ebenen, Masterseite an
//!   der Stelle des Platzhalters), Elemente in `ELMv` oben → unten (rückwärts zeichnen).
//! - Web-Ebene, Slices und Hotspots kommen nicht ins Bild; `PRPb`-Stellvertreterpfade werden verworfen.
//! - Status: nicht geteilte Ebenen der Tiefe 0 zeigen `cels[Status]`, geteilte Ebenen und
//!   Unterebenen `cels[0]` (Slot-119-Regel).
//! - Mischen: Modus Normal mit der MMX-Variante von Original-over, andere Modi über
//!   `fp_color::blend` (skalar); Deckkraft als Byte, auch auf Caches; Ebenen-Deckkraft gibt es in
//!   CS6 nicht (07 T4) und wird ignoriert.
//! - Gruppen: isoliert (Zwischenfläche), außer ein Kind hat einen Mischmodus (07 T3). Masken:
//!   erstes Kind bzw. `MRMv`; Vektormaske = Alpha des gerenderten Maskenpfads (mit `MNAb`) bzw.
//!   Fülldeckung; Bitmapmaske = Alpha·(255 − Grau) (mit `MNAb`) bzw. Alpha.
//! - Instanzen: erste Definition mit der Symbol-ID; Grafik/Schaltfläche/Rich Symbol zeigen
//!   Status 1, Animationen `Status mod FRC` mit Versatz/Skalierung/Drehung/Deckkraft (13 §7.3, T6);
//!   9-Slice bei skalierten Instanzen (Pfade punktweise, Strichbreite bleibt, Bitmaps in 9 Teilen,
//!   Text nur verschoben); `ATMv` wird nicht benutzt.
//! - Render-Caches nach [`CachePolicy`] (Vorgabe wie das Original: Pfade neu rastern).
//! - Live-Effekte über `fp-effects`, in Listenreihenfolge, vor Deckkraft und Mischmodus.
//!
//! # Beispiel
//!
//! ```no_run
//! let bytes = std::fs::read("bild.fw.png").unwrap();
//! let doc = fp_fwpng::read(&bytes).unwrap();
//! let r = fp_render::Renderer::default();
//! let out = r.render_page(&doc, doc.current_page, 0);
//! println!("{}×{}, vollständig: {}", out.image.width(), out.image.height(), out.report.is_complete());
//! // Speichern mit echtem flachem Bild und echter Vorschau:
//! let file = fp_fwpng::write(&doc, &fp_fwpng::WriteOptions::default(), &r).unwrap();
//! # let _ = file;
//! ```

pub mod cache;
pub mod compose;
pub mod convert;
pub mod flatten;
pub mod hit;
pub mod nine;
pub mod options;
pub mod text_edge;
pub mod tiles;

pub use cache::{CacheStats, Look, RenderCache};
pub use compose::{
    PixelOverride, RenderOutput, Renderer, find_bitmap, has_visible_effects, kind_tag, placement,
};
pub use hit::{Hit, HitOptions};
pub use nine::{AxisMap, PointMap, Stage};
pub use options::{BackgroundMode, CachePolicy, RenderOptions, RenderReport};
pub use tiles::{TileCache, TileStats};
