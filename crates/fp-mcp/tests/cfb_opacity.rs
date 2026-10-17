//! `get_document` reports the opacity like the original's JSF `opacity`: the stored byte as b·100/255,
//! not rounded (re 40 §3, M8: field 30 → 30.19607843137255). F25 reverts CF-B `8c886bb`, which had
//! rounded it to whole percent; only the inspector field shows round(b/2.55).

fn find_opacity(v: &serde_json::Value, out: &mut Vec<f64>) {
    match v {
        serde_json::Value::Object(m) => {
            if m.get("type").is_some()
                && let Some(o) = m.get("opacity").and_then(|o| o.as_f64())
            {
                out.push(o);
            }
            for x in m.values() {
                find_opacity(x, out);
            }
        }
        serde_json::Value::Array(a) => a.iter().for_each(|x| find_opacity(x, out)),
        _ => {}
    }
}

#[test]
fn document_json_opacity_is_byte_percentage() {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", serde_json::json!({"width": 200, "height": 200}))
        .expect("doc");
    for v in [50, 33, 1, 99] {
        s.execute(
            "draw.rectangle",
            serde_json::json!({"rect": [10, 10, 60, 60]}),
        )
        .expect("rect");
        s.execute("element.setOpacity", serde_json::json!({"opacity": v}))
            .expect("opacity");
    }
    let j = fp_mcp::docjson::document_json(
        s.active().expect("doc"),
        &fp_mcp::docjson::DocJsonOptions::default(),
    );
    let mut found = Vec::new();
    find_opacity(&j, &mut found);
    found.sort_by(|a, b| a.total_cmp(b));
    // bytes 3, 84, 128, 252 (round(p·2.55))
    let want: Vec<f64> = [3.0, 84.0, 128.0, 252.0]
        .iter()
        .map(|b| b * 100.0 / 255.0)
        .collect();
    assert_eq!(found, want, "{j}");
}
