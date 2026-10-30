//! F151 / TW-285: `dom.setFillNColor(fill, color)` takes the fill object's Web Dither kind with
//! its colours (`ditherColors`, `webDitherTransparent`), the feather edge (`edgeType` + `feather`),
//! `gradient.dither` and the fill colour of a pattern fill, like the reference (RE spec 78), and
//! the fill getter reads them back.

use fp_script::{ScriptOptions, run_source};
use serde_json::json;

fn run(s: &mut fp_engine::Session, src: &str) -> String {
    let o = run_source(s, src, &ScriptOptions::default());
    assert!(
        o.ok,
        "error: {:?}\ndiagnostics: {:?}",
        o.error, o.diagnostics
    );
    o.result.unwrap_or_default()
}

fn session_with_rect() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute(
        "file.new",
        json!({"width": 200, "height": 160, "background": "#ffffff"}),
    )
    .expect("document");
    s.execute(
        "draw.rectangle",
        json!({"rect": [20, 20, 180, 140], "fillColor": "#336699", "strokeWidth": 0}),
    )
    .expect("rectangle");
    s
}

fn model(s: &fp_engine::Session) -> fp_model::PathAttrs {
    let d = s.active().expect("doc");
    let id = d.selection.elements[0];
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("element");
    fp_engine::docops::path_attrs_of(&e)
        .map(|a| (*a).clone())
        .expect("path attrs")
}

const FILL: &str = "var f = fw.selection[0].pathAttributes.fill; ";

#[test]
fn web_dither_with_two_colours_lands_and_reads_back() {
    let mut s = session_with_rect();
    run(
        &mut s,
        "fw.getDocumentDOM().setFillNColor({category: 'fc_WebDither', name: 'fn_Normal', \
         shape: 'web dither', edgeType: 'hard', feather: 0, \
         ditherColors: ['#FF0000', '#0000FF'], webDitherTransparent: false}, '#FF0000'); 0",
    );
    let a = model(&s);
    let f = a.fill.as_ref().expect("fill");
    assert_eq!(f.shape, 16, "Web Dither shape");
    assert_eq!(f.category, "fc_WebDither");
    assert_eq!(
        f.dither_colors,
        Some([0xffff_0000, 0xff00_00ff, 0xff00_00ff, 0xffff_0000])
    );
    assert_eq!(f.edge, fp_model::Fill::EDGE_HARD);
    assert_eq!(
        run(
            &mut s,
            &format!("{FILL}[f.category, f.shape, f.ditherColors.join(',')].join('|')")
        ),
        "fc_WebDither|web dither|#ff0000,#0000ff,#0000ff,#ff0000"
    );
    // a render is a checkerboard of the two colours
    let d = s.active().expect("doc");
    let img = fp_render::Renderer::default()
        .render_page(&d.doc, d.page, 0)
        .image;
    let rgb = |x: i32, y: i32| {
        let p = img.pixel(x, y);
        (p.r, p.g, p.b)
    };
    let (a0, b0) = (rgb(60, 60), rgb(61, 60));
    assert_ne!(a0, b0, "neighbours differ");
    for (x, y) in [(60, 60), (61, 61), (100, 80), (101, 81)] {
        assert_eq!(rgb(x, y), a0, "same phase at ({x},{y})");
    }
    for (x, y) in [(61, 60), (60, 61), (101, 80), (100, 81)] {
        assert_eq!(rgb(x, y), b0, "other phase at ({x},{y})");
    }
    let mut pair = [a0, b0];
    pair.sort();
    assert_eq!(pair, [(0, 0, 255), (255, 0, 0)], "red and blue cells");
}

#[test]
fn web_dither_transparent_and_round_trip() {
    let mut s = session_with_rect();
    run(
        &mut s,
        "fw.getDocumentDOM().setFillNColor({category: 'fc_WebDither', \
         ditherColors: ['#FF0000', '#FF0000'], webDitherTransparent: true}, '#FF0000'); 0",
    );
    let f = model(&s).fill.expect("fill");
    assert_eq!(f.transparent_dither, Some(true));
    assert_eq!(
        run(&mut s, &format!("{FILL}f.webDitherTransparent")),
        "true"
    );
    // read-modify-write keeps the kind and the colours
    run(
        &mut s,
        &format!(
            "{FILL}f.ditherColors = ['#00FF00', '#000000']; \
             fw.getDocumentDOM().setFillNColor(f, '#00FF00'); 0"
        ),
    );
    let f = model(&s).fill.expect("fill");
    assert_eq!(f.shape, 16);
    assert_eq!(
        f.dither_colors,
        Some([0xff00_ff00, 0xff00_0000, 0xff00_0000, 0xff00_ff00])
    );
}

#[test]
fn feather_lands_as_feather_edge_and_reads_back() {
    let mut s = session_with_rect();
    run(
        &mut s,
        "fw.getDocumentDOM().setFillNColor({category: 'fc_Solid', name: 'fn_Normal', \
         shape: 'solid', edgeType: 'antialiased', feather: 20}, '#FF0000'); 0",
    );
    let a = model(&s);
    let f = a.fill.as_ref().expect("fill");
    assert_eq!((f.edge, f.feather), (fp_model::Fill::EDGE_FEATHER, 20));
    assert_eq!(a.fill_color, fp_color::Argb(0xffff_0000));
    // the reference reports a feather edge as "antialiased" with feather > 0
    assert_eq!(
        run(&mut s, &format!("{FILL}f.edgeType + '|' + f.feather")),
        "antialiased|20"
    );
    // explicit "feather" and a hard edge
    run(
        &mut s,
        "fw.getDocumentDOM().setFillNColor({category: 'fc_Solid', edgeType: 'feather', \
         feather: 7}, '#FF0000'); 0",
    );
    let f = model(&s).fill.expect("fill");
    assert_eq!((f.edge, f.feather), (fp_model::Fill::EDGE_FEATHER, 7));
    run(
        &mut s,
        "fw.getDocumentDOM().setFillNColor({category: 'fc_Solid', edgeType: 'hard', \
         feather: 0}, '#FF0000'); 0",
    );
    let f = model(&s).fill.expect("fill");
    assert_eq!((f.edge, f.feather), (fp_model::Fill::EDGE_HARD, 0));
}

#[test]
fn gradient_dither_lands_and_reads_back() {
    let mut s = session_with_rect();
    run(
        &mut s,
        "fw.getDocumentDOM().setFillNColor({category: 'fc_Linear', name: 'fn_Normal', \
         shape: 'linear', edgeType: 'antialiased', feather: 0, \
         gradient: {name: 'cn_Custom', dither: true, nodes: [\
           {color: '#FF0000', position: 0, isOpacityNode: false}, \
           {color: '#FF0000', position: 1, isOpacityNode: false}]}}, '#FF0000'); 0",
    );
    let f = model(&s).fill.expect("fill");
    let g = f.gradient.as_ref().expect("gradient");
    assert_eq!(g.stops.as_ref().and_then(|st| st.dither), Some(true));
    assert_eq!(run(&mut s, &format!("{FILL}f.gradient.dither")), "true");
    // and off again
    run(
        &mut s,
        &format!(
            "{FILL}f.gradient.dither = false; fw.getDocumentDOM().setFillNColor(f, '#FF0000'); 0"
        ),
    );
    assert_eq!(run(&mut s, &format!("{FILL}f.gradient.dither")), "false");
}

#[test]
fn pattern_fill_takes_the_fill_colour() {
    let mut s = session_with_rect();
    s.execute("pi.fill", json!({"category": "pattern"}))
        .expect("pattern");
    let name = run(&mut s, &format!("{FILL}f.pattern.name"));
    assert!(!name.is_empty(), "pattern name");
    run(
        &mut s,
        &format!(
            "fw.getDocumentDOM().setFillNColor({{category: 'fc_Pattern', shape: 'pattern', \
             pattern: {{name: '{name}'}}}}, '#FF0000'); 0"
        ),
    );
    let a = model(&s);
    assert_eq!(
        a.fill.as_ref().map(|f| f.shape),
        Some(15),
        "still a pattern"
    );
    assert_eq!(
        a.fill_color,
        fp_color::Argb(0xffff_0000),
        "pattern fill colour"
    );
    assert_eq!(
        run(&mut s, "fw.selection[0].pathAttributes.fillColor"),
        "#ff0000"
    );
}

#[test]
fn plain_solid_object_leaves_the_edge_alone() {
    // existing scripts without edgeType/feather keep the element's edge (TW-285 adds fields only)
    let mut s = session_with_rect();
    s.execute("pi.fill", json!({"edge": 0})).expect("hard");
    run(
        &mut s,
        "fw.getDocumentDOM().setFillNColor({category: 'fc_Solid', name: 'fn_Normal', \
         shape: 'solid'}, '#3366cc'); 0",
    );
    let f = model(&s).fill.expect("fill");
    assert_eq!(f.edge, fp_model::Fill::EDGE_HARD);
    assert_eq!(f.shape, 0);
}
