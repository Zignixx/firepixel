//! Golden-Test des Pfad-Renderings gegen die Python-Referenz `re\vektor\pathrender.py`
//! (synthetische Pfade: alle Rasterzweige, harte/weiche Kante, alle Verlaufsformen mit
//! Deckkraftstopps, Web-Dither, simple-Pinsel mit/ohne AA, Platzierung innen/außen, Strichmuster,
//! `FOTb`, Farb-Alpha, `BFBi = 2`). Daten: `tools\golden\fp-vector\gen_render_golden.py`.

mod common;

use common::*;
use fp_geom::IRect;
use serde_json::Value;

const DATA: &str = include_str!("data/render_golden.json");

#[test]
fn render_against_reference() {
    let cases: Vec<Value> = serde_json::from_str(DATA).expect("JSON");
    let (mut n, mut ok) = (0, 0);
    for c in &cases {
        let w = &c["win"];
        let g = |i: usize| w[i].as_i64().unwrap_or(0) as i32;
        let rect = IRect::from_xywh(g(0), g(1), g(2), g(3));
        let p = vector_path(Tags::from(&c["pth"]), &Extra::default(), false);
        let (px, _) = fp_vector::render_path_pixels(&p, rect);
        let want = b64(c["rgba"].as_str().unwrap_or(""));
        let (same, _, _, total) = compare(&px, &want);
        n += 1;
        if same == total {
            ok += 1;
        } else {
            eprintln!("{}: {same} von {total} Pixeln gleich", c["name"]);
        }
    }
    println!("Rendering: {ok} von {n} Fällen bitgleich zur Referenz");
    assert_eq!(ok, n);
}
