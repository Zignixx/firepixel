//! Vergleiche mit Dateien der Installationskopie des Originals (`..\korpus` bzw. `FP_CORPUS_DIR`, nur lesen).
//! Aufruf: `cargo test -p fp-export --test installation -- --ignored --nocapture`.

mod common;

use common::{re_dir, read_gif};

/// GIF-LZW (08 §5.2): LZW-Ströme aller GIFs der Installation neu kodiert. Spezifikation 126/126.
#[test]
#[ignore]
fn gif_lzw_of_installation() {
    use fp_export::gif::{interlace_order, lzw_encode};
    let root = std::env::var_os("FP_CORPUS_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| re_dir().join("..").join("korpus"));
    let mut stack = vec![root.clone()];
    let (mut n, mut ok) = (0, 0);
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                if p.file_name().is_some_and(|f| f != "AMT") {
                    stack.push(p);
                }
            } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("gif"))
                && let Some(g) = read_gif(&p)
            {
                for f in &g.frames {
                    let mut idx = f.indices.clone();
                    if f.interlaced && f.w > 0 {
                        let mut v = Vec::new();
                        for row in interlace_order(f.h) {
                            v.extend_from_slice(&idx[row * f.w..(row + 1) * f.w]);
                        }
                        idx = v;
                    }
                    n += 1;
                    ok += usize::from(lzw_encode(&idx, f.min_code) == f.lzw);
                }
            }
        }
    }
    println!("GIF-LZW: {ok}/{n} Ströme byte-gleich");
    assert!(n >= 90, "{n}");
    assert_eq!(ok, n);
}
