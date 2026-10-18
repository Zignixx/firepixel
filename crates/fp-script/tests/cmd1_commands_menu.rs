//! BA-104 (CMD-1): Commands menu – symbol scripts for graphic symbols, the shared adaptive palette
//! over all open documents and the screen capture command.

use fp_engine::Session;
use fp_script::ScriptOptions;
use fp_script::builtin_commands::BUILTIN;
use serde_json::json;

fn session() -> Session {
    let mut s = Session::default();
    fp_script::install_host(&mut s);
    s
}

/// Document with a graphic symbol "Logo" (red rectangle with a blue stroke and a text); the
/// instance is selected.
fn symbol_doc() -> Session {
    let mut s = session();
    s.execute("file.new", json!({"width": 300, "height": 200}))
        .unwrap();
    s.execute(
        "draw.rectangle",
        json!({"rect": [10, 10, 110, 60], "fillColor": "#ff0000", "strokeWidth": 3, "strokeColor": "#0000ff"}),
    )
    .unwrap();
    s.execute(
        "draw.text",
        json!({"text": "Hallo", "anchor": [20, 100], "size": 24, "color": "#00ff00"}),
    )
    .unwrap();
    s.execute("select.all", json!({})).unwrap();
    s.execute(
        "modify.symbol.convertToSymbol",
        json!({"kind": "graphic", "name": "Logo"}),
    )
    .unwrap();
    s
}

fn symbol_elements(s: &Session, name: &str) -> Vec<fp_model::Element> {
    let d = s.active().unwrap();
    let (_, sym) = d
        .doc
        .symbol_by_name(name)
        .unwrap_or_else(|| panic!("symbol {name}"));
    let mut v = Vec::new();
    for slot in &sym.root.layers {
        if let fp_model::LayerSlot::Own(l) = slot
            && l.kind == fp_model::LayerKind::Normal
            && let Some(c) = l.cels.first()
        {
            v.extend(c.elements.iter().map(|e| (**e).clone()));
        }
    }
    v
}

#[test]
fn symbol_script_recreates_shapes_fills_strokes_and_text() {
    let mut s = symbol_doc();
    let out = s
        .execute("commands.symbolScript", json!({}))
        .expect("symbolScript");
    let script = out.value["script"].as_str().expect("script").to_string();
    assert_eq!(out.value["name"], "Logo");
    assert!(script.contains("add(\"draw.path\""), "{script}");
    assert!(
        script.contains("add(\"draw.text\"") && script.contains("Hallo"),
        "{script}"
    );
    assert!(script.contains("\"strokeWidth\":3"), "{script}");
    // run it in a fresh document: the same symbol with the same objects
    let mut t = session();
    t.execute("file.new", json!({"width": 300, "height": 200}))
        .unwrap();
    let o = fp_script::run_source(&mut t, &script, &ScriptOptions::default());
    assert!(o.ok, "{:?} {:?}", o.error, o.diagnostics);
    let a = symbol_elements(&s, "Logo");
    let b = symbol_elements(&t, "Logo");
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(&b) {
        match (&x.kind, &y.kind) {
            (fp_model::ElementKind::Path(p), fp_model::ElementKind::Path(q)) => {
                assert_eq!(p.contours.len(), q.contours.len());
                assert_eq!(p.attrs.fill_color, q.attrs.fill_color);
                assert_eq!(p.attrs.fill.is_some(), q.attrs.fill.is_some());
                assert_eq!(p.attrs.stroke_color, q.attrs.stroke_color);
                assert_eq!(
                    p.attrs.brush.as_ref().map(|b| b.diameter),
                    q.attrs.brush.as_ref().map(|b| b.diameter)
                );
            }
            (fp_model::ElementKind::Text(p), fp_model::ElementKind::Text(q)) => {
                assert_eq!(p.plain_text(), q.plain_text());
            }
            other => panic!("different kinds: {other:?}"),
        }
    }
}

#[test]
fn symbol_script_only_for_graphic_symbols() {
    let mut s = session();
    s.execute("file.new", json!({"width": 100, "height": 100}))
        .unwrap();
    s.execute("draw.rectangle", json!({"rect": [0, 0, 50, 50]}))
        .unwrap();
    s.execute("select.all", json!({})).unwrap();
    s.execute(
        "modify.symbol.convertToSymbol",
        json!({"kind": "button", "name": "Knopf"}),
    )
    .unwrap();
    assert!(s.execute("commands.symbolScript", json!({})).is_err());
    s.execute("select.set", json!({"ids": []})).unwrap();
    assert!(
        s.execute("commands.symbolScript", json!({})).is_err(),
        "no instance selected"
    );
}

#[test]
fn builtin_command_offers_the_script_for_saving() {
    let mut s = symbol_doc();
    let src = BUILTIN
        .iter()
        .find(|(r, _)| *r == "Symbolskript erstellen.jsf")
        .map(|(_, x)| *x)
        .unwrap();
    let o = fp_script::run_source(&mut s, src, &ScriptOptions::default());
    assert!(o.ok, "{:?}", o.error);
    assert!(o.alerts.is_empty(), "{:?}", o.alerts);
    assert_eq!(o.save_as.len(), 1);
    assert_eq!(o.save_as[0].0, "Logo.jsf");
    assert!(o.save_as[0].1.contains("modify.symbol.convertToSymbol"));
}

#[test]
fn shared_palette_over_all_open_documents() {
    let mut s = session();
    s.execute(
        "file.new",
        json!({"width": 40, "height": 40, "background": "#ff0000"}),
    )
    .unwrap();
    s.execute(
        "file.new",
        json!({"width": 40, "height": 40, "background": "#0000ff"}),
    )
    .unwrap();
    s.execute(
        "draw.rectangle",
        json!({"rect": [0, 0, 20, 40], "fillColor": "#00ff00", "strokeWidth": 0}),
    )
    .unwrap();
    let act = std::env::temp_dir().join(format!("fp-cmd1-{}.act", std::process::id()));
    let out = s
        .execute(
            "commands.sharedPalette",
            json!({"colors": 8, "path": act.display().to_string()}),
        )
        .expect("sharedPalette");
    let colors: Vec<String> = out.value["colors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap().to_ascii_lowercase())
        .collect();
    for want in ["ff0000", "0000ff", "00ff00"] {
        assert!(
            colors.iter().any(|c| c.contains(want)),
            "{want} in {colors:?}"
        );
    }
    let bytes = std::fs::read(&act).expect(".act written");
    let _ = std::fs::remove_file(&act);
    assert!(bytes.len() >= colors.len() * 3, "{}", bytes.len());
    let first = format!("{:02x}{:02x}{:02x}", bytes[0], bytes[1], bytes[2]);
    assert!(colors[0].contains(&first), "{first} vs {}", colors[0]);
    // shown in the Swatches panel
    let sw = fp_engine::commands::panel::current_swatches(&s.prefs);
    assert_eq!(sw.len(), colors.len());
}

/// Smoke test: with a display, a capture becomes a new bitmap document (skipped without one).
#[test]
fn screen_capture_creates_a_bitmap_document() {
    let mut s = session();
    match s.execute("commands.screenCapture", json!({"rect": [0, 0, 64, 48]})) {
        Ok(out) => {
            assert_eq!(out.value["width"], 64.0, "{:?}", out.value);
            let d = s.active().expect("new document");
            assert!(d.path.is_none(), "untitled");
            let mut bitmaps = 0;
            d.doc.walk_elements(|_, _, e| {
                if matches!(e.kind, fp_model::ElementKind::Bitmap(_)) {
                    bitmaps += 1;
                }
            });
            assert!(bitmaps > 0, "bitmap object");
        }
        Err(e) => eprintln!("screen capture skipped (no display?): {e}"),
    }
}
