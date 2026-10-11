//! Lesen und Schreiben ganzer Dateien.
//!
//! Chunk-Reihenfolge beim Schreiben wie im Original (02 §2.1): `IHDR sBIT pHYs tEXt… prVW…|mkPG… mkBF mkTS
//! mkBS mkBT… [aTXT] [iTXt XMP] IDAT IEND`.

use crate::chunks::{self, Preview};
use crate::error::{FwError, Result};
use crate::flatten::Flattener;
use crate::png::{self as pngc, Chunk, Ihdr};
use crate::preview;
use crate::read::{ReadCtx, read_document};
use crate::write::{CachePolicy, WriteCtx, write_document};
use crate::{ate, tree, zlib};
use fp_model::*;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Obergrenze für entpackte Bäume (Schutz vor beschädigten Dateien).
const TREE_LIMIT: usize = 1 << 30;

/// Rohsicht einer Datei: Chunks, geparste Bäume, Kacheln.
#[derive(Debug)]
pub struct RawFile {
    pub chunks: Vec<Chunk>,
    pub ihdr: Option<Ihdr>,
    /// Entpackter `mkTS`-Strom und seine Wurzeln.
    pub ts_bytes: Option<Vec<u8>>,
    pub ts: Option<Vec<RawNode>>,
    /// Entpackter `mkBS`-Strom und seine Wurzeln.
    pub bs_bytes: Option<Vec<u8>>,
    pub bs: Option<Vec<RawNode>>,
    /// Kacheln nach ID (bei doppelten IDs gilt die erste).
    pub tiles: HashMap<u32, Arc<Tile>>,
    /// `aTXT`-Bytes.
    pub atxt: Option<Vec<u8>>,
}

impl RawFile {
    /// Ist es eine FW-PNG (mit `mkTS` oder `mkBS`)?
    pub fn is_fwpng(&self) -> bool {
        self.ts.is_some() || self.bs.is_some()
    }
}

/// Optionen beim Lesen.
#[derive(Clone, Copy, Debug)]
pub struct ReadOptions {
    /// Flaches Bild (`IDAT`) dekodieren und in `file_meta.flat_image` ablegen.
    pub decode_image: bool,
    /// Kacheln sofort entpacken (parallel). Sonst bleiben nur die zlib-Ströme erhalten.
    pub inflate_tiles: bool,
}

impl Default for ReadOptions {
    fn default() -> Self {
        ReadOptions { decode_image: true, inflate_tiles: true }
    }
}

/// Liest Chunks, Bäume und Kacheln (ohne Abbildung auf das Modell).
pub fn read_raw(bytes: &[u8], inflate_tiles: bool) -> Result<RawFile> {
    let chunks = pngc::read_chunks(bytes)?;
    let mut f = RawFile { chunks, ihdr: None, ts_bytes: None, ts: None, bs_bytes: None, bs: None, tiles: HashMap::new(), atxt: None };
    let mut tile_chunks: Vec<&[u8]> = Vec::new();
    for c in &f.chunks {
        match &c.kind {
            b"IHDR" => f.ihdr = Some(Ihdr::parse(&c.data)?),
            b"mkTS" if f.ts.is_none() => {
                let z = zlib::inflate_limited(&c.data, TREE_LIMIT)?;
                f.ts = Some(tree::parse(&z)?);
                f.ts_bytes = Some(z);
            }
            b"mkBS" if f.bs.is_none() => {
                let z = zlib::inflate_limited(&c.data, TREE_LIMIT)?;
                f.bs = Some(tree::parse(&z)?);
                f.bs_bytes = Some(z);
            }
            b"mkBT" => tile_chunks.push(&c.data),
            b"aTXT" if f.atxt.is_none() => f.atxt = Some(c.data.clone()),
            _ => {}
        }
    }
    let tiles: Vec<Result<Tile>> = tile_chunks.par_iter().map(|d| chunks::read_tile(d, inflate_tiles)).collect();
    for t in tiles {
        let t = t?;
        f.tiles.entry(t.id).or_insert_with(|| Arc::new(t));
    }
    Ok(f)
}

/// Ist es eine FW-PNG? (schnelle Prüfung auf `mkTS`/`mkBS`)
pub fn is_fwpng(bytes: &[u8]) -> bool {
    match pngc::read_chunks(bytes) {
        Ok(cs) => cs.iter().any(|c| c.is(b"mkTS") || c.is(b"mkBS")),
        Err(_) => false,
    }
}

/// Liest eine `.fw.png` in ein Dokument. Normale PNGs ohne `mkTS` liefern einen Fehler.
pub fn read(bytes: &[u8]) -> Result<Document> {
    read_with(bytes, ReadOptions::default())
}

/// Wie [`read`], mit Optionen.
pub fn read_with(bytes: &[u8], opts: ReadOptions) -> Result<Document> {
    let raw = read_raw(bytes, opts.inflate_tiles)?;
    document_from_raw(&raw, opts)
}

/// Enthält der Chunk-Inhalt ein XMP-Paket? Erkannt am Paket selbst (`<x:xmpmeta` bzw.
/// `<?xpacket`), nicht am Schlüsselwort des Text-Chunks; der Chunk wird unverändert durchgereicht.
fn is_xmp_packet(data: &[u8]) -> bool {
    data.windows(10).any(|w| w == b"<x:xmpmeta" || w == b"<?xpacket ")
}

/// Baut das Dokument aus einer Rohsicht.
pub fn document_from_raw(raw: &RawFile, opts: ReadOptions) -> Result<Document> {
    let Some(ts) = &raw.ts else {
        return Err(FwError::Document("keine FW-PNG (mkTS fehlt)".into()));
    };
    let ate_data = raw.atxt.as_ref().map(|d| ate::decode(d));
    let mut ctx = ReadCtx::new(&raw.tiles);
    let mut doc = read_document(ts, &mut ctx, ate_data)?;
    // Kacheln, die nur der Platzhalter `mkBS` (oder niemand) benutzt.
    let referenced = ctx.referenced.clone();
    let mut meta = FileMeta::default();
    let mut ids: Vec<u32> = raw.tiles.keys().copied().filter(|id| !referenced.contains(id)).collect();
    ids.sort_unstable();
    meta.compat_tiles = ids.iter().filter_map(|id| raw.tiles.get(id).cloned()).collect();
    meta.compat_tree = raw.bs.clone();
    let mut seen_image = false;
    for c in &raw.chunks {
        match &c.kind {
            b"IDAT" => seen_image = true,
            b"IHDR" | b"IEND" | b"mkTS" | b"mkBS" | b"mkBT" | b"aTXT" | b"sBIT" => {}
            b"tEXt" => {
                let (k, v) = pngc::parse_text(&c.data);
                if is_xmp_packet(v.as_bytes()) && meta.xmp_chunk.is_none() {
                    meta.xmp_chunk = Some(RawChunk { kind: c.kind, data: c.data.clone(), after_image: seen_image });
                } else {
                    meta.text_chunks.push((k, v));
                }
            }
            b"iTXt" if meta.xmp_chunk.is_none() && is_xmp_packet(&c.data) => {
                meta.xmp_chunk = Some(RawChunk { kind: c.kind, data: c.data.clone(), after_image: seen_image });
            }
            b"pHYs" => meta.phys = pngc::parse_phys(&c.data),
            b"prVW" | b"pRVW" | b"mkPG" => meta.previews.push(RawChunk { kind: c.kind, data: c.data.clone(), after_image: seen_image }),
            b"mkBF" => meta.format_block = Some(c.data.clone()),
            _ => meta.other_chunks.push(RawChunk { kind: c.kind, data: c.data.clone(), after_image: seen_image }),
        }
    }
    if opts.decode_image
        && let Ok(img) = pngc::decode_image(&raw.chunks) {
            meta.flat_image = Some(Arc::new(img));
        }
    doc.file_meta = meta;
    Ok(doc)
}

/// Umgang mit der Vorschau beim Schreiben.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PreviewMode {
    /// Mit Renderer neu erzeugen; ohne Renderer die Vorschauen der Originaldatei übernehmen, wenn ihre
    /// Anzahl zu den Seiten passt, sonst aus dem flachen Bild erzeugen.
    #[default]
    Auto,
    /// Immer neu erzeugen (ohne Renderer aus dem flachen Bild bzw. leer).
    Regenerate,
    /// Vorschauen der Originaldatei unverändert übernehmen.
    Keep,
    /// Keine Bilder: `mkPG` je Seite (Voreinstellung `WriteThumbnailPreview` aus).
    PageNamesOnly,
}

/// Optionen beim Schreiben.
#[derive(Clone, Debug)]
pub struct WriteOptions {
    pub preview: PreviewMode,
    pub caches: CachePolicy,
    /// Text für `tEXt Software`, falls die Datei keinen hat.
    pub software: String,
}

impl Default for WriteOptions {
    fn default() -> Self {
        WriteOptions { preview: PreviewMode::Auto, caches: CachePolicy::Keep, software: "Firepixel".to_string() }
    }
}

/// Schreibt ein Dokument als `.fw.png`.
pub fn write(doc: &Document, opts: &WriteOptions, flattener: &dyn Flattener) -> Result<Vec<u8>> {
    let meta = &doc.file_meta;
    let reserved: Vec<u32> = meta.compat_tiles.iter().map(|t| t.id).collect();
    let mut ctx = WriteCtx::new(opts.caches, reserved);
    let roots = write_document(doc, &mut ctx);
    let ts = tree::serialize(&roots);

    // Platzhalter mkBS: aus der Originaldatei oder eigener (ohne Kacheln, einfarbig weiß).
    let (bs_tree, compat_tiles): (Vec<RawNode>, Vec<Arc<Tile>>) = match &meta.compat_tree {
        Some(t) => (t.clone(), meta.compat_tiles.clone()),
        None => (placeholder_tree(), Vec::new()),
    };
    let bs = tree::serialize(&bs_tree);

    // Flaches Bild.
    let page = doc.active_page().cloned();
    let (cw, ch) = page
        .as_ref()
        .map(|p| (p.canvas.width.round().max(1.0) as u32, p.canvas.height.round().max(1.0) as u32))
        .unwrap_or((1, 1));
    let flat = flattener
        .flatten(doc, doc.current_page)
        .or_else(|| meta.flat_image.as_ref().filter(|i| i.width == cw && i.height == ch).map(|i| (**i).clone()))
        .unwrap_or_else(|| {
            let bg = page.as_ref().map(|p| p.root.background).unwrap_or(Argb(0x00ff_ffff));
            FlatImage::filled(cw, ch, [bg.r(), bg.g(), bg.b(), bg.a()])
        });

    let mut out: Vec<Chunk> = Vec::new();
    out.push(Chunk::new(b"IHDR", Ihdr::rgba8(flat.width, flat.height).to_bytes()));
    out.push(Chunk::new(b"sBIT", vec![8, 8, 8, 8]));
    match (meta.phys, &page) {
        (Some((x, y, u)), _) => {
            let mut d = Vec::with_capacity(9);
            d.extend_from_slice(&x.to_be_bytes());
            d.extend_from_slice(&y.to_be_bytes());
            d.push(u);
            out.push(Chunk::new(b"pHYs", d));
        }
        (None, Some(p)) => out.push(pngc::make_phys(p.canvas.resolution, p.canvas.resolution_unit)),
        _ => {}
    }
    if meta.text_chunks.is_empty() {
        out.push(pngc::make_text("Software", &opts.software));
    } else {
        for (k, v) in &meta.text_chunks {
            out.push(pngc::make_text(k, v));
        }
    }
    out.extend(preview_chunks(doc, opts, flattener, &flat));
    out.push(Chunk::new(b"mkBF", meta.format_block.clone().unwrap_or_else(chunks::make_format_block)));
    out.push(Chunk::new(b"mkTS", zlib::deflate(&ts)));
    out.push(Chunk::new(b"mkBS", zlib::deflate(&bs)));
    let mut tiles: Vec<(u32, Arc<Tile>)> = ctx.tiles.clone();
    tiles.extend(compat_tiles.into_iter().map(|t| (t.id, t)));
    tiles.sort_by_key(|(id, _)| *id);
    let mut seen = HashSet::new();
    tiles.retain(|(id, _)| seen.insert(*id));
    let tile_data: Vec<Vec<u8>> = tiles.par_iter().map(|(id, t)| chunks::make_tile(t, *id)).collect();
    for d in tile_data {
        out.push(Chunk::new(b"mkBT", d));
    }
    if let Some(a) = &doc.ate {
        let data = match &a.unparsed {
            Some(u) => u.clone(),
            None => ate::serialize(&a.root),
        };
        out.push(Chunk::new(b"aTXT", data));
    }
    if let Some(x) = &meta.xmp_chunk
        && !x.after_image {
            out.push(Chunk::new(&x.kind, x.data.clone()));
        }
    for c in meta.other_chunks.iter().filter(|c| !c.after_image) {
        out.push(Chunk::new(&c.kind, c.data.clone()));
    }
    out.extend(pngc::encode_idat(&flat)?);
    if let Some(x) = &meta.xmp_chunk
        && x.after_image {
            out.push(Chunk::new(&x.kind, x.data.clone()));
        }
    for c in meta.other_chunks.iter().filter(|c| c.after_image) {
        out.push(Chunk::new(&c.kind, c.data.clone()));
    }
    out.push(Chunk::new(b"IEND", Vec::new()));
    Ok(pngc::write_chunks(&out))
}

/// Seiten, die eine Vorschau bekommen (keine Masterseite, 02 §12.8).
fn preview_pages(doc: &Document) -> Vec<usize> {
    (0..doc.pages.len()).filter(|&i| !doc.pages[i].master()).collect()
}

fn preview_chunks(doc: &Document, opts: &WriteOptions, fl: &dyn Flattener, flat: &FlatImage) -> Vec<Chunk> {
    let pages = preview_pages(doc);
    let name = |i: usize| doc.pages[i].name.clone().unwrap_or_default();
    if opts.preview == PreviewMode::PageNamesOnly {
        return pages.iter().map(|&i| Chunk::new(b"mkPG", chunks::encode_page_name(&name(i)))).collect();
    }
    let originals = &doc.file_meta.previews;
    if opts.preview == PreviewMode::Keep && !originals.is_empty() {
        return originals.iter().map(|c| Chunk::new(&c.kind, c.data.clone())).collect();
    }
    let mut rendered: Vec<Option<Preview>> = Vec::new();
    let mut any_rendered = false;
    for &i in &pages {
        let p = &doc.pages[i];
        let pv = preview::thumbnail_geometry(p.canvas.width, p.canvas.height).and_then(|g| {
            fl.render_scaled(doc, i, g.scale, g.width, g.height).map(|img| preview::preview_from_render(&img, &g, Some(name(i))))
        });
        any_rendered |= pv.is_some();
        rendered.push(pv);
    }
    if opts.preview == PreviewMode::Auto && !any_rendered && !originals.is_empty() && originals.len() == pages.len() {
        return originals.iter().map(|c| Chunk::new(&c.kind, c.data.clone())).collect();
    }
    pages
        .iter()
        .zip(rendered)
        .map(|(&i, pv)| {
            let pv = pv.unwrap_or_else(|| {
                let p = &doc.pages[i];
                match preview::thumbnail_geometry(p.canvas.width, p.canvas.height) {
                    Some(g) if i == doc.current_page => {
                        let small = preview::downscale_nearest(flat, &g);
                        preview::preview_from_render(&small, &g, Some(name(i)))
                    }
                    _ => preview::blank_preview(Some(name(i))),
                }
            });
            Chunk::new(b"prVW", pv.encode())
        })
        .collect()
}

/// Eigener `mkBS`-Platzhalter für sehr alte Leser: gleiche Struktur wie im Original (ein 256×172-Bitmap
/// auf einer Ebene), aber mit einfarbig weißen Kacheln statt eines Bildes.
pub fn placeholder_tree() -> Vec<RawNode> {
    use crate::k;
    let tiles: Vec<RawNode> = (0..4).map(|_| RawNode::int(k!(b"TMCi"), 0xffff_ffff)).collect();
    let img = RawNode::list(
        k!(b"IMGv"),
        vec![
            RawNode::bool(k!(b"LCKb"), false),
            RawNode::num(k!(b"XOFf"), 0.0),
            RawNode::num(k!(b"YOFf"), 0.0),
            RawNode::num(k!(b"XLCf"), 0.0),
            RawNode::num(k!(b"YLCf"), 0.0),
            RawNode::int(k!(b"WPXi"), 256),
            RawNode::int(k!(b"HPXi"), 172),
            RawNode::int(k!(b"TSZi"), 128),
            RawNode::list(k!(b"TILv"), tiles),
            RawNode::list(k!(b"DTAv"), Vec::new()),
        ],
    );
    let cel = RawNode::list(k!(b"CELv"), vec![RawNode::bool(k!(b"LCKb"), false), RawNode::list(k!(b"ELMv"), vec![img])]);
    let layer = RawNode::list(
        k!(b"LAYv"),
        vec![
            RawNode::bool(k!(b"BKGb"), false),
            RawNode::bool(k!(b"URLb"), false),
            RawNode::str(k!(b"LNMs"), "Layer 1"),
            RawNode::bool(k!(b"MSLb"), false),
            RawNode::list(k!(b"CLLv"), vec![cel]),
        ],
    );
    vec![RawNode::list(
        k!(b"MKBv"),
        vec![
            RawNode::int(k!(b"FRCi"), 1),
            RawNode::num(k!(b"XLCf"), 0.0),
            RawNode::num(k!(b"YLCf"), 0.0),
            RawNode::num(k!(b"WIDf"), 256.0),
            RawNode::num(k!(b"HITf"), 172.0),
            RawNode::num(k!(b"RESf"), 72.0),
            RawNode::int(k!(b"RSUi"), 0),
            RawNode::int(k!(b"BGCi"), 0xffff_ffff),
            RawNode::list(k!(b"LYLv"), vec![layer]),
        ],
    )]
}

/// Tag-Baum → Modell → Tag-Baum für einen `mkTS`-Strom (Prüfhilfe für die verlustfreie Abbildung).
pub fn model_roundtrip(ts: &[RawNode], tiles: &HashMap<u32, Arc<Tile>>, atxt: Option<&[u8]>) -> Result<Vec<u8>> {
    let mut ctx = ReadCtx::new(tiles);
    let doc = read_document(ts, &mut ctx, atxt.map(ate::decode))?;
    let mut wctx = WriteCtx::new(CachePolicy::Keep, Vec::new());
    let roots = write_document(&doc, &mut wctx);
    Ok(tree::serialize(&roots))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flatten::NoFlattener;

    #[test]
    fn new_document_writes_and_reads_back() {
        let mut doc = Document::new(120.0, 80.0, Argb(0xffff_ffff));
        let mut ids = std::mem::take(&mut doc.ids);
        let rect = Element::new_rectangle(&mut ids, Rect::new(10.0, 10.0, 60.0, 40.0), 0.0, PathAttrs::solid_fill(Argb(0xffff_0000)));
        let text = Element::new_text(&mut ids, "Hallo", "Arial", 12.0, Argb(0xff00_0000), Point::new(10.0, 70.0));
        let mut tid = 100u32;
        let px: Vec<u8> = (0..20 * 10).flat_map(|i| [i as u8, 0, 0, 255]).collect();
        let bmp = Element::new_bitmap(&mut ids, Point::new(70.0, 10.0), 20, 10, &px, &mut tid);
        doc.ids = ids;
        {
            let page = doc.page_mut(0).unwrap();
            if let LayerSlot::Own(l) = &mut page.root.layers[0] {
                let cel = Arc::make_mut(&mut Arc::make_mut(l).cels[0]);
                cel.elements.push(Arc::new(rect));
                cel.elements.push(Arc::new(text));
                cel.elements.push(Arc::new(bmp));
            }
        }
        let bytes = write(&doc, &WriteOptions::default(), &NoFlattener).unwrap();
        let back = read(&bytes).unwrap();
        assert_eq!(back.pages.len(), 1);
        assert!(back.root_copy.is_some());
        let p = &back.pages[0];
        assert_eq!(p.canvas.width, 120.0);
        let LayerSlot::Own(l) = &p.root.layers[0] else { panic!("Ebene") };
        assert_eq!(l.cels[0].elements.len(), 3);
        assert!(matches!(l.cels[0].elements[0].kind, ElementKind::Path(_)));
        if let ElementKind::Bitmap(b) = &l.cels[0].elements[2].kind {
            let (rgba, missing) = b.image.to_rgba();
            assert!(!missing);
            assert_eq!(rgba, px);
        } else {
            panic!("Bitmap erwartet");
        }
        // Zweite Rundreise ist stabil.
        let bytes2 = write(&back, &WriteOptions { preview: PreviewMode::Keep, ..WriteOptions::default() }, &NoFlattener).unwrap();
        let raw1 = read_raw(&bytes, true).unwrap();
        let raw2 = read_raw(&bytes2, true).unwrap();
        assert_eq!(raw1.ts_bytes, raw2.ts_bytes);
        assert_eq!(raw1.bs_bytes, raw2.bs_bytes);
    }

    #[test]
    fn plain_png_is_not_fwpng() {
        let img = FlatImage::filled(2, 2, [1, 2, 3, 255]);
        let mut cs = vec![Chunk::new(b"IHDR", Ihdr::rgba8(2, 2).to_bytes())];
        cs.extend(pngc::encode_idat(&img).unwrap());
        cs.push(Chunk::new(b"IEND", vec![]));
        let b = pngc::write_chunks(&cs);
        assert!(!is_fwpng(&b));
        assert!(read(&b).is_err());
        assert!(!read_raw(&b, true).unwrap().is_fwpng());
    }
}
