//! Q-58 (TW-192, RE spec 58 section 2): `addNewRectangle`, `addNewRectanglePrimitive` and
//! `addNewOval` take the current tool attributes (`dom.pathAttributes`) exactly - fill, stroke
//! colour, tip, size and placement; `addNewLine` takes the tool stroke and no fill, and with the
//! tool stroke "none" still gets the 1-pixel tip, 1 px, in the tool stroke colour. The tool
//! attributes follow the last selected object after the script (not inside it) and keep them after
//! deselecting.

use fp_engine::Session;
use fp_script::{ScriptOptions, run_source};

fn run(s: &mut Session, src: &str) -> String {
    let o = run_source(s, src, &ScriptOptions::default());
    assert!(
        o.ok,
        "error: {:?}\ndiagnostics: {:?}",
        o.error, o.diagnostics
    );
    o.result.unwrap_or_default()
}

const NEW: &str = "var d = fw.createFirepixelDocument({x: 800, y: 500}, {pixelsPerUnit: 72, units: 'inch'}, '#ffffff'); d = fw.getDocumentDOM();\n";

/// Attributes of a `pathAttributes` object: fill, fill colour, brush name, size, colour, placement.
const ATTRS: &str = "function attrs(p) { return [p.fill ? 'fill' : 'nofill', p.fill ? p.fillColor : '-', \
    p.brush ? p.brush.name + ' ' + p.brush.diameter : 'nobrush', p.brush ? p.brushColor : '-', \
    p.brush ? p.brushPlacement : '-'].join(' '); }\n\
    function sel() { return attrs(fw.selection[0].pathAttributes); }\n";

/// The four shapes of the measurement (`q58.jsf`), each drawn with nothing selected before.
const SHAPES: &str = "var out = [];\n\
    d.selectNone(); d.addNewRectangle({left: 20, top: 20, right: 120, bottom: 80}, 0); out.push('R ' + sel());\n\
    d.selectNone(); d.addNewOval({left: 140, top: 20, right: 240, bottom: 80}); out.push('E ' + sel());\n\
    d.selectNone(); d.addNewLine({x: 260, y: 20}, {x: 360, y: 80}); out.push('L ' + sel());\n\
    d.selectNone(); d.addNewRectanglePrimitive({left: 380, top: 20, right: 480, bottom: 80}, 0); out.push('P ' + sel());\n\
    out.push('T ' + attrs(d.pathAttributes));\n\
    out.join('|')";

#[test]
fn shapes_take_tool_stroke_none() {
    let mut s = Session::default();
    let got = run(
        &mut s,
        &format!(
            "{NEW}{ATTRS} d.setBrushColor('#ff0000'); d.setFillColor('#0000ff'); d.setBrush(null);\n{SHAPES}"
        ),
    );
    assert_eq!(
        got,
        "R fill #0000ff nobrush - -|E fill #0000ff nobrush - -|\
         L nofill - bn_1-Pixel 1 #ff0000 center|P fill #0000ff nobrush - -|\
         T fill #0000ff nobrush - -"
    );
}

#[test]
fn shapes_take_tool_stroke_1px_inside() {
    let mut s = Session::default();
    let got = run(
        &mut s,
        &format!(
            "{NEW}{ATTRS} d.setFillColor('#0000ff');\n\
             d.setBrushNColor({{category: 'bc_Pencil', name: 'bn_1-Pixel Hard', diameter: 1, softness: 0}}, '#000000');\n\
             d.setBrushPlacement('inside');\n{SHAPES}"
        ),
    );
    let one = "bn_1-Pixel 1 #000000 inside";
    assert_eq!(
        got,
        format!(
            "R fill #0000ff {one}|E fill #0000ff {one}|L nofill - {one}|P fill #0000ff {one}|T fill #0000ff {one}"
        )
    );
}

#[test]
fn shapes_take_tool_stroke_2px_center_and_4px_outside() {
    let mut s = Session::default();
    let got = run(
        &mut s,
        &format!(
            "{NEW}{ATTRS} d.setFillColor('#0000ff');\n\
             d.setBrushNColor({{category: 'bc_Pencil', name: 'bn_1-Pixel Hard', diameter: 2, softness: 0}}, '#000000');\n\
             d.setBrushPlacement('center');\n{SHAPES}"
        ),
    );
    let two = "bn_1-Pixel 2 #000000 center";
    assert_eq!(
        got,
        format!(
            "R fill #0000ff {two}|E fill #0000ff {two}|L nofill - {two}|P fill #0000ff {two}|T fill #0000ff {two}"
        )
    );
    let got = run(
        &mut s,
        &format!(
            "d = fw.getDocumentDOM();\n{ATTRS} d.selectNone();\n\
             d.setBrushNColor({{category: 'bc_Pencil', name: 'bn_1-Pixel Hard', diameter: 4, softness: 0}}, '#cc00cc');\n\
             d.setBrushPlacement('outside');\n{SHAPES}"
        ),
    );
    let four = "bn_1-Pixel 4 #cc00cc outside";
    assert_eq!(
        got,
        format!(
            "R fill #0000ff {four}|E fill #0000ff {four}|L nofill - {four}|P fill #0000ff {four}|T fill #0000ff {four}"
        )
    );
}

/// `setFillColor` on the new selection changes the tool fill too: the next shape gets it.
#[test]
fn set_fill_color_on_selection_changes_the_tool_fill() {
    let mut s = Session::default();
    let got = run(
        &mut s,
        &format!(
            "{NEW}{ATTRS} d.setFillColor('#0000ff'); d.setBrush(null);\n\
             d.addNewRectangle({{left: 20, top: 20, right: 120, bottom: 80}}, 0); d.setFillColor('#3366cc');\n\
             var r = sel(); d.addNewOval({{left: 140, top: 20, right: 240, bottom: 80}}); r + '|' + sel()"
        ),
    );
    assert_eq!(got, "fill #3366cc nobrush - -|fill #3366cc nobrush - -");
}

/// RE spec 58 section 2.2: selecting an object inside a script leaves the tool attributes; the next
/// script sees the selected object's attributes, which stay after deselecting.
#[test]
fn tool_attributes_follow_the_selection_after_the_script() {
    let mut s = Session::default();
    // R: 4 px outside #cc00cc; then the tool stroke 1 px inside #000000 with nothing selected
    run(
        &mut s,
        &format!(
            "{NEW} d.setFillColor('#0000ff');\n\
             d.setBrushNColor({{category: 'bc_Pencil', name: 'bn_1-Pixel Hard', diameter: 4, softness: 0}}, '#cc00cc');\n\
             d.setBrushPlacement('outside');\n\
             d.addNewRectangle({{left: 400, top: 20, right: 600, bottom: 120}}, 0); fw.selection[0].name = 'R';\n\
             d.selectNone();\n\
             d.setBrushNColor({{category: 'bc_Pencil', name: 'bn_1-Pixel Hard', diameter: 1, softness: 0}}, '#000000');\n\
             d.setBrushPlacement('inside'); ''"
        ),
    );
    const FIND: &str = "function byName(n) { var e = d.layers[d.currentLayerNum].frames[0].elements; \
        for (var i = 0; i < e.length; i++) if (e[i].name == n) return e[i]; return null; }\n";
    // same script: selecting R does not change the tool attributes
    let got = run(
        &mut s,
        &format!(
            "d = fw.getDocumentDOM();\n{ATTRS}{FIND} fw.selection = [byName('R')]; var t = attrs(d.pathAttributes);\n\
             d.selectNone(); d.addNewRectangle({{left: 20, top: 200, right: 120, bottom: 260}}, 0);\n\
             t + '|' + sel()"
        ),
    );
    assert_eq!(
        got,
        "fill #0000ff bn_1-Pixel 1 #000000 inside|fill #0000ff bn_1-Pixel 1 #000000 inside"
    );
    // script A selects R; script B (after idle) sees R's attributes and deselects; script C draws
    run(
        &mut s,
        &format!("d = fw.getDocumentDOM();\n{FIND} fw.selection = [byName('R')]; ''"),
    );
    let got = run(
        &mut s,
        &format!(
            "d = fw.getDocumentDOM();\n{ATTRS} var t = attrs(d.pathAttributes); d.selectNone(); t"
        ),
    );
    assert_eq!(got, "fill #0000ff bn_1-Pixel 4 #cc00cc outside");
    let got = run(
        &mut s,
        &format!(
            "d = fw.getDocumentDOM();\n{ATTRS} var t = attrs(d.pathAttributes);\n\
             d.addNewRectangle({{left: 20, top: 300, right: 120, bottom: 360}}, 0); t + '|' + sel()"
        ),
    );
    assert_eq!(
        got,
        "fill #0000ff bn_1-Pixel 4 #cc00cc outside|fill #0000ff bn_1-Pixel 4 #cc00cc outside"
    );
}
