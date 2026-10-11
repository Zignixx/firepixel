//! Korpustests über die 710 FW-PNGs der Installationskopie (`..\korpus`) und die Testdateien in `re/_testdateien`.
//!
//! Laufen nur mit `cargo test -p fp-fwpng -- --ignored --nocapture` (ARCHITEKTUR §7). Die Zahlen stehen
//! in `STATUS.md`. Basisordner: Umgebungsvariable `FIREPIXEL_REA`, sonst `../../..` relativ zum Crate,
//! sonst `C:\Users\Administrator\Desktop\REA`. Korpusordner: `FP_CORPUS_DIR`, sonst `<Basis>\korpus`.

use fp_fwpng::chunks::Preview;
use fp_fwpng::{NoFlattener, PreviewMode, ReadOptions, WriteOptions, ate, preview, tree};
use fp_model::{Document, LayerSlot};
use std::path::{Path, PathBuf};

fn rea_root() -> PathBuf {
    if let Ok(p) = std::env::var("FIREPIXEL_REA") {
        return PathBuf::from(p);
    }
    let rel = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("..");
    if rel.join("re").is_dir() {
        return rel;
    }
    PathBuf::from(r"C:\Users\Administrator\Desktop\REA")
}

fn pngs(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        let mut entries: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")) {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

fn corpus() -> Vec<PathBuf> {
    let dir = std::env::var_os("FP_CORPUS_DIR").map(PathBuf::from).unwrap_or_else(|| rea_root().join("korpus"));
    pngs(&dir)
}

fn testfiles() -> Vec<PathBuf> {
    pngs(&rea_root().join("re").join("_testdateien"))
}

fn software(raw: &fp_fwpng::RawFile) -> String {
    raw.chunks
        .iter()
        .filter(|c| c.is(b"tEXt"))
        .map(|c| fp_fwpng::png::parse_text(&c.data))
        .find(|(k, _)| k == "Software")
        .map(|(_, v)| v)
        .unwrap_or_default()
}

/// (1) Alle Dateien lesen: Chunks, Bäume, Kacheln, Modell.
#[test]
#[ignore]
fn corpus_parse_all() {
    for (label, files) in [("Korpus", corpus()), ("Testdateien", testfiles())] {
        let (mut fw, mut plain, mut errors) = (0, 0, 0);
        let mut tiles = 0usize;
        let mut warn_invariants = 0;
        for f in &files {
            let bytes = std::fs::read(f).expect("lesbar");
            match fp_fwpng::read_raw(&bytes, true) {
                Ok(raw) if raw.ts.is_some() => {
                    tiles += raw.tiles.len();
                    match fp_fwpng::document_from_raw(&raw, ReadOptions { decode_image: true, inflate_tiles: true }) {
                        Ok(doc) => {
                            fw += 1;
                            if !doc.check_invariants().is_empty() {
                                warn_invariants += 1;
                            }
                        }
                        Err(e) => {
                            errors += 1;
                            println!("FEHLER Modell {}: {e}", f.display());
                        }
                    }
                }
                Ok(_) => plain += 1,
                Err(e) => {
                    errors += 1;
                    println!("FEHLER {}: {e}", f.display());
                }
            }
        }
        println!(
            "KORPUS[{label}] Dateien {} | FW-PNG gelesen {fw} | normale PNG {plain} | Fehler {errors} | Kacheln {tiles} | mit Invarianten-Hinweis {warn_invariants}",
            files.len()
        );
        assert_eq!(errors, 0, "{label}: Lesefehler");
    }
}

/// (2) Rundreise Tag-Baum → Bytes → Tag-Baum (Messweise wie `fwpng.py roundtrip`: je `mkTS`/`mkBS`
/// entpackt, geparst, serialisiert, Vergleich der Bytes).
#[test]
#[ignore]
fn corpus_tree_roundtrip() {
    for (label, files) in [("Korpus", corpus()), ("Testdateien", testfiles())] {
        let (mut ok, mut bad) = (0, 0);
        let (mut ate_ok, mut ate_bad) = (0, 0);
        for f in &files {
            let bytes = std::fs::read(f).expect("lesbar");
            let Ok(chunks) = fp_fwpng::png::read_chunks(&bytes) else { continue };
            for c in &chunks {
                if c.is(b"mkTS") || c.is(b"mkBS") {
                    let z = fp_fwpng::zlib::inflate(&c.data).expect("zlib");
                    let nodes = tree::parse(&z).expect("Baum");
                    if tree::serialize(&nodes) == z {
                        ok += 1;
                    } else {
                        bad += 1;
                        println!("ABWEICHUNG {} {}", f.display(), c.kind_str());
                    }
                }
                if c.is(b"aTXT") {
                    match ate::parse(&c.data) {
                        Ok(top) if ate::serialize(&top) == c.data => ate_ok += 1,
                        _ => {
                            ate_bad += 1;
                            println!("aTXT-ABWEICHUNG {}", f.display());
                        }
                    }
                }
            }
        }
        println!("KORPUS[{label}] Baum-Rundreise byte-identisch {ok}/{} | aTXT-Rundreise {ate_ok}/{}", ok + bad, ate_ok + ate_bad);
        assert_eq!(bad, 0);
        assert_eq!(ate_bad, 0);
    }
}

fn first_diff(a: &[u8], b: &[u8]) -> String {
    let n = a.iter().zip(b.iter()).position(|(x, y)| x != y).unwrap_or(a.len().min(b.len()));
    let lo = n.saturating_sub(60);
    format!(
        "Offset {n} (Länge {} / {})\n  Original: {:?}\n  Modell:   {:?}",
        a.len(),
        b.len(),
        String::from_utf8_lossy(&a[lo..(n + 60).min(a.len())]),
        String::from_utf8_lossy(&b[lo..(n + 60).min(b.len())])
    )
}

/// (3) Tag-Baum → fp-model → Tag-Baum verlustfrei (byte-identischer `mkTS`-Strom).
#[test]
#[ignore]
fn corpus_model_roundtrip() {
    for (label, files) in [("Korpus", corpus()), ("Testdateien", testfiles())] {
        let (mut ok, mut bad) = (0, 0);
        let mut shown = 0;
        for f in &files {
            let bytes = std::fs::read(f).expect("lesbar");
            let Ok(raw) = fp_fwpng::read_raw(&bytes, false) else { continue };
            let (Some(ts), Some(ts_bytes)) = (&raw.ts, &raw.ts_bytes) else { continue };
            let out = fp_fwpng::model_roundtrip(ts, &raw.tiles, raw.atxt.as_deref()).expect("Abbildung");
            if &out == ts_bytes {
                ok += 1;
            } else {
                bad += 1;
                if shown < 8 {
                    shown += 1;
                    println!("MODELL-ABWEICHUNG {}: {}", f.display(), first_diff(ts_bytes, &out));
                }
            }
        }
        println!("KORPUS[{label}] Tag-Baum → Modell → Tag-Baum byte-identisch {ok}/{}", ok + bad);
        assert_eq!(bad, 0);
    }
}

/// (3b) Datei → Modell → Datei: `mkTS`/`mkBS` identisch, alle Kacheln vorhanden und gleich, `aTXT` gleich.
#[test]
#[ignore]
fn corpus_file_roundtrip() {
    for (label, files) in [("Korpus", corpus()), ("Testdateien", testfiles())] {
        let (mut ok, mut bad) = (0, 0);
        for f in &files {
            let bytes = std::fs::read(f).expect("lesbar");
            let Ok(raw) = fp_fwpng::read_raw(&bytes, true) else { continue };
            if raw.ts.is_none() {
                continue;
            }
            let doc = fp_fwpng::document_from_raw(&raw, ReadOptions::default()).expect("Modell");
            let out = fp_fwpng::write(&doc, &WriteOptions { preview: PreviewMode::Keep, ..WriteOptions::default() }, &NoFlattener)
                .expect("schreiben");
            let back = fp_fwpng::read_raw(&out, true).expect("zurücklesen");
            let mut same = back.ts_bytes == raw.ts_bytes && back.bs_bytes == raw.bs_bytes && back.atxt == raw.atxt;
            same &= back.tiles.len() == raw.tiles.len()
                && raw.tiles.iter().all(|(id, t)| back.tiles.get(id).is_some_and(|b| b.pixels == t.pixels && b.kind == t.kind));
            let img_a = fp_fwpng::png::decode_image(&raw.chunks).ok();
            let img_b = fp_fwpng::png::decode_image(&back.chunks).ok();
            same &= img_a == img_b;
            if same {
                ok += 1;
            } else {
                bad += 1;
                println!("DATEI-ABWEICHUNG {}", f.display());
            }
        }
        println!("KORPUS[{label}] Datei → Modell → Datei (mkTS, mkBS, mkBT, aTXT, IDAT gleich) {ok}/{}", ok + bad);
        assert_eq!(bad, 0);
    }
}

/// Zählt Rohknoten in der JSON-Form des Modells: Einträge von `nodes` (Extra) und `Raw`-Varianten.
fn count_raw(v: &serde_json::Value, keys: &mut std::collections::BTreeMap<String, usize>, shadows: &mut usize) {
    use serde_json::Value;
    let mut stack = vec![v];
    while let Some(v) = stack.pop() {
        match v {
            Value::Object(m) => {
                for (k, x) in m {
                    match (k.as_str(), x) {
                        ("nodes", Value::Array(a)) => {
                            for n in a {
                                if let Some(key) = n.get("key").and_then(|k| k.as_str()) {
                                    *keys.entry(key.to_string()).or_default() += 1;
                                }
                            }
                        }
                        ("shadow", Value::Array(a)) => {
                            *shadows += a.len();
                            for n in a {
                                if let Some(key) = n.get("key").and_then(|k| k.as_str()) {
                                    *keys.entry(format!("Schatten:{key}")).or_default() += 1;
                                }
                            }
                        }
                        ("Raw", Value::Object(n)) if n.contains_key("key") => {
                            if let Some(key) = n.get("key").and_then(|k| k.as_str()) {
                                *keys.entry(key.to_string()).or_default() += 1;
                            }
                        }
                        ("extra_roots", Value::Array(a)) => {
                            for n in a {
                                if let Some(key) = n.get("key").and_then(|k| k.as_str()) {
                                    *keys.entry(key.to_string()).or_default() += 1;
                                }
                            }
                        }
                        ("compat_tree", _) => {}
                        _ => stack.push(x),
                    }
                }
            }
            Value::Array(a) => stack.extend(a.iter()),
            _ => {}
        }
    }
}

fn pages_for_previews(doc: &Document, n: usize) -> Vec<usize> {
    let all: Vec<usize> = (0..doc.pages.len()).collect();
    let nonmaster: Vec<usize> = all.iter().copied().filter(|&i| !doc.pages[i].master()).collect();
    if nonmaster.len() == n { nonmaster } else { all }
}

/// (4) `prVW`-Erzeugung gegen die gespeicherten Vorschauen (Messweisen wie `q_prvw_*.py`).
#[test]
#[ignore]
fn corpus_preview() {
    let grid = preview::grid_palette(6);
    let (mut total, mut struct_ok, mut pal_ok, mut encode_ok) = (0, 0, 0, 0);
    let (mut cs6_total, mut cs6_112) = (0, 0);
    let (mut fb_total, mut fb_ok) = (0, 0);
    let (mut rect_total, mut rect_in) = (0, 0);
    let (mut dith_total, mut dith_ok, mut dith_ok2) = (0, 0, 0);
    let (mut efef_total, mut efef_ok) = (0, 0);
    for f in corpus() {
        let bytes = std::fs::read(&f).expect("lesbar");
        let Ok(raw) = fp_fwpng::read_raw(&bytes, false) else { continue };
        if raw.ts.is_none() {
            continue;
        }
        let soft = software(&raw);
        let doc = fp_fwpng::document_from_raw(&raw, ReadOptions { decode_image: false, inflate_tiles: false }).expect("Modell");
        let pvs: Vec<Preview> =
            raw.chunks.iter().filter(|c| c.is(b"prVW") || c.is(b"pRVW")).filter_map(|c| Preview::decode(&c.data).ok()).collect();
        let n_chunks = raw.chunks.iter().filter(|c| c.is(b"prVW") || c.is(b"pRVW")).count();
        total += n_chunks;
        struct_ok += pvs.len();
        let seq = pages_for_previews(&doc, pvs.len());
        for (pv, &pi) in pvs.iter().zip(seq.iter()) {
            let (w, h) = (pv.width as usize, pv.height as usize);
            // Palette: nur benutzte Rasterfarben, lückenlos, in Rasterreihenfolge.
            let mut used = vec![false; 256];
            for y in 0..h {
                for x in 0..w {
                    used[pv.index(x, y) as usize] = true;
                }
            }
            let n_used = used.iter().rposition(|u| *u).map(|i| i + 1).unwrap_or(0);
            let contiguous = used[..n_used].iter().all(|u| *u);
            let cols: Vec<[u8; 3]> = (0..n_used).map(|i| [pv.palette[i * 3], pv.palette[i * 3 + 1], pv.palette[i * 3 + 2]]).collect();
            let pos: Option<Vec<usize>> = cols.iter().map(|c| grid.iter().position(|g| g == c)).collect();
            if contiguous && pos.as_ref().is_some_and(|p| p.windows(2).all(|w| w[0] < w[1])) {
                pal_ok += 1;
            }
            // Eigener Kodierer auf dem gespeicherten Bild: gleiche Palette und Indizes?
            let rgba = pv.to_rgba();
            let mine = preview::build_preview(&rgba, pv.width, pv.height, pv.page_name.clone());
            let same_idx = (0..h).all(|y| (0..w).all(|x| mine.index(x, y) == pv.index(x, y)));
            if same_idx && mine.palette[..n_used * 3] == pv.palette[..n_used * 3] {
                encode_ok += 1;
            }
            let page = &doc.pages[pi];
            let (pw, ph) = (page.canvas.width, page.canvas.height);
            if soft.contains("CS6") {
                cs6_total += 1;
                if (w, h) == (112, 112) {
                    cs6_112 += 1;
                }
            }
            if (w, h) != (112, 112) {
                fb_total += 1;
                if preview::fallback_size(pw, ph) == (pv.width, pv.height) {
                    fb_ok += 1;
                }
            } else if let Some(g) = preview::thumbnail_geometry(pw, ph) {
                // Inhalt im berechneten Miniaturbild-Rechteck (nicht ganz weiße Vorschauen).
                let mut bb: Option<(usize, usize, usize, usize)> = None;
                for y in 0..h {
                    for x in 0..w {
                        if pv.rgb(x, y) != [255, 255, 255] {
                            bb = Some(match bb {
                                None => (x, y, x + 1, y + 1),
                                Some((a, b, c, d)) => (a.min(x), b.min(y), c.max(x + 1), d.max(y + 1)),
                            });
                        }
                    }
                }
                if let Some((x0, y0, x1, y1)) = bb {
                    rect_total += 1;
                    let (rx0, ry0) = (g.dx as usize, g.dy as usize);
                    let (rx1, ry1) = (rx0 + g.width as usize, ry0 + g.height as usize);
                    if x0 >= rx0 && y0 >= ry0 && x1 <= rx1 && y1 <= ry1 {
                        rect_in += 1;
                    }
                }
            }
            // Fehlerdiffusion: Zeile 0 einer Vorschau, deren Rand die Leinwandfarbe zeigt.
            let row0: Vec<[u8; 3]> = (0..w).map(|x| pv.rgb(x, 0)).collect();
            if row0.iter().all(|c| *c == [255, 255, 255]) {
                continue;
            }
            dith_total += 1;
            let bg = page.root.background;
            // Messweise wie `q_prvw_dither.py`: konstante Fläche in Leinwandfarbe über die ganze Breite.
            let px: Vec<u8> = (0..w * 2).flat_map(|_| [bg.r(), bg.g(), bg.b(), bg.a()]).collect();
            let idx = preview::diffuse_to_grid(&px, w, 2, 6, 100);
            let sim0: Vec<[u8; 3]> = idx[..w].iter().map(|&i| grid[i as usize]).collect();
            let efef = bg.0 == 0xffef_efef;
            if efef {
                efef_total += 1;
            }
            if sim0 == row0 {
                dith_ok += 1;
                if efef {
                    efef_ok += 1;
                }
                let row1: Vec<[u8; 3]> = (0..w).map(|x| pv.rgb(x, 1.min(h.saturating_sub(1)))).collect();
                let sim1: Vec<[u8; 3]> = idx[w..].iter().map(|&i| grid[i as usize]).collect();
                if sim1 == row1 {
                    dith_ok2 += 1;
                }
            }
        }
    }
    println!(
        "KORPUS[prVW] Chunks {total} | Struktur {struct_ok}/{total} | Palette = benutzte Rasterfarben in Reihenfolge {pal_ok}/{struct_ok} | \
         eigener Kodierer identisch (Palette+Indizes) {encode_ok}/{struct_ok} | CS6 112×112 {cs6_112}/{cs6_total} | \
         Rückfallgröße {fb_ok}/{fb_total} | Inhalt im Miniaturbild-Rechteck {rect_in}/{rect_total} | \
         Dither Zeile 0 exakt {dith_ok}/{dith_total} geprüft (davon Leinwand #EFEFEF {efef_ok}, Zeile 0+1 {dith_ok2}; \
         Leinwand #EFEFEF insgesamt {efef_total})"
    );
    // Referenz `q_prvw_dither.py`: 75 geprüft, 25 exakt (alle #EFEFEF), Zeile 0+1: 22.
    assert!(dith_ok >= 25 && efef_ok >= 25 && dith_ok2 >= 22, "Dither schlechter als die Python-Referenz");
    assert_eq!(struct_ok, total);
    assert_eq!(pal_ok, struct_ok);
    assert_eq!(encode_ok, struct_ok);
    assert_eq!(cs6_112, cs6_total);
    assert_eq!(fb_ok, fb_total);
}

/// Schreiben mit neu erzeugter `MKBv`-Kopie (wie nach einer Änderung der aktiven Seite): Datei muss
/// lesbar bleiben, alle `PDCv` unverändert, `MKBv` mit `ILLb{0}` und Web-Ebene zuerst.
#[test]
#[ignore]
fn corpus_regenerated_root_copy() {
    let (mut ok, mut bad, mut total) = (0, 0, 0);
    for f in corpus() {
        let bytes = std::fs::read(&f).expect("lesbar");
        let Ok(raw) = fp_fwpng::read_raw(&bytes, false) else { continue };
        let Some(ts) = &raw.ts else { continue };
        let mut doc = fp_fwpng::document_from_raw(&raw, ReadOptions { decode_image: true, inflate_tiles: false }).expect("Modell");
        if doc.single_root {
            continue;
        }
        total += 1;
        doc.invalidate_root_copy();
        let out = fp_fwpng::write(&doc, &WriteOptions::default(), &NoFlattener).expect("schreiben");
        let back = fp_fwpng::read_raw(&out, false).expect("zurücklesen");
        let Some(ts2) = &back.ts else {
            bad += 1;
            continue;
        };
        let pdc_a: Vec<_> = ts.iter().filter(|n| n.key == fp_model::TagKey::new(b"PDCv")).collect();
        let pdc_b: Vec<_> = ts2.iter().filter(|n| n.key == fp_model::TagKey::new(b"PDCv")).collect();
        let mkb = &ts2[0];
        let illb = mkb.child(fp_model::TagKey::new(b"ILLb")).and_then(|n| n.as_bool()) == Some(false);
        let web_first = mkb
            .child(fp_model::TagKey::new(b"LYLv"))
            .and_then(|l| l.children().first())
            .and_then(|l| l.child(fp_model::TagKey::new(b"URLb")))
            .and_then(|b| b.as_bool())
            .unwrap_or(false);
        let readable = fp_fwpng::read(&out).is_ok();
        if pdc_a == pdc_b && illb && web_first && readable {
            ok += 1;
        } else {
            bad += 1;
            println!("MKBv-NEU {}: PDCv gleich {} ILLb {} Web zuerst {} lesbar {}", f.display(), pdc_a == pdc_b, illb, web_first, readable);
        }
    }
    println!("KORPUS[MKBv neu erzeugt] {ok}/{total} (PDCv unverändert, ILLb{{0}}, Web-Ebene zuerst, lesbar)");
    assert_eq!(bad, 0);
}

/// Kennzahlen des Modells über den Korpus (Seiten, Ebenen, Symbole, aufgelöste Verweise).
#[test]
#[ignore]
fn corpus_model_stats() {
    let (mut files, mut pages, mut layers, mut symbols, mut dup_ids, mut unresolved, mut stories, mut texts_with_story) =
        (0, 0, 0, 0, 0, 0, 0, 0);
    let (mut shared_refs, mut shared_ok, mut masters) = (0, 0, 0);
    let mut story_mismatch_shown = 0;
    let mut texts_with_aid = 0;
    let mut raw_keys: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut shadows = 0usize;
    for f in corpus() {
        let bytes = std::fs::read(&f).expect("lesbar");
        let Ok(doc) = fp_fwpng::read_with(&bytes, ReadOptions { decode_image: false, inflate_tiles: false }) else { continue };
        files += 1;
        pages += doc.pages.len();
        symbols += doc.symbols.len();
        let mut seen = std::collections::HashSet::new();
        for s in &doc.symbols {
            if !seen.insert(s.id) {
                dup_ids += 1;
            }
        }
        masters += doc.pages.iter().filter(|p| p.master()).count();
        for p in &doc.pages {
            for s in &p.root.layers {
                match s {
                    LayerSlot::Own(_) => layers += 1,
                    LayerSlot::Shared(r) => {
                        shared_refs += 1;
                        if doc.shared_layer(&r.id).is_some() {
                            shared_ok += 1;
                        }
                    }
                    _ => {}
                }
            }
        }
        if let Some(a) = &doc.ate {
            stories += a.stories.len();
        }
        let mut visit = |e: &std::sync::Arc<fp_model::Element>, count_instances: bool| match &e.kind {
            fp_model::ElementKind::Instance(i) => {
                if count_instances && doc.symbol_by_id(i.symbol_id).is_none() {
                    unresolved += 1;
                }
            }
            fp_model::ElementKind::Text(t) => {
                if let (Some(idx), Some(a)) = (t.story_index, &doc.ate) {
                    texts_with_aid += 1;
                    if let Some(s) = (idx as usize).checked_sub(1).and_then(|i| a.stories.get(i)) {
                        // Die Story endet mit `\r`, die Läufe in `TFSv` nicht.
                        if s.text.trim_end_matches('\r') == t.plain_text().trim_end_matches('\r') {
                            texts_with_story += 1;
                        } else if story_mismatch_shown < 3 {
                            story_mismatch_shown += 1;
                            println!("STORY {}: {:?} <> {:?}", f.display(), s.text, t.plain_text());
                        }
                    }
                }
            }
            _ => {}
        };
        doc.walk_elements(|_, _, e| visit(e, true));
        if let Some(copy) = &doc.root_copy {
            fp_model::traverse::walk_root(&copy.root, fp_model::RootRef::RootCopy, &mut |_, _, e| visit(e, false));
        }
        // Nicht modellierte Knoten (Extra.nodes, Raw-Varianten) und Schatten über die JSON-Form zählen.
        let v = serde_json::to_value(&doc).expect("serde");
        count_raw(&v, &mut raw_keys, &mut shadows);
    }
    let shadow_keys: Vec<(String, usize)> =
        raw_keys.iter().filter(|(k, _)| k.starts_with("Schatten:")).map(|(k, v)| (k.clone(), *v)).collect();
    raw_keys.retain(|k, _| !k.starts_with("Schatten:"));
    println!("KORPUS[Modell] Schatten je Tag: {shadow_keys:?}");
    let raw_total: usize = raw_keys.values().sum();
    let mut top: Vec<(&String, &usize)> = raw_keys.iter().collect();
    top.sort_by(|a, b| b.1.cmp(a.1));
    println!(
        "KORPUS[Modell] nicht modelliert (roh erhalten): {} Tag-Kombinationen, {raw_total} Vorkommen; Schatten {shadows}; häufigste: {:?}",
        raw_keys.len(),
        &top[..top.len().min(12)]
    );
    println!(
        "KORPUS[Modell] Dateien {files} | Seiten {pages} (Masterseiten {masters}) | eigene Ebenen {layers} | Symbole {symbols} (doppelte IDs {dup_ids}) | \
         Instanzen ohne Symbol {unresolved} | geteilte Ebenen aufgelöst {shared_ok}/{shared_refs} | ATE-Stories {stories}, \
         Text = Story-Text {texts_with_story}/{texts_with_aid}"
    );
}
