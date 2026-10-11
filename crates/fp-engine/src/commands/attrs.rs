//! Attribut- und Strukturbefehle für Skripte, Agenten und Inspektor, die es als Menübefehl nicht
//! gibt: customData, Musterfüllung, Texturen aus Bilddateien, Pinsel-Empfindlichkeiten,
//! Ebenen aufklappen, Leinwandursprung (`Document.left/top`).

use super::{target_ids, with_doc, DOC};
use crate::command::{cmd, CommandOutput, CommandRegistry, ParamKind, Params};
use crate::docops;
use crate::error::{EngineError, Result};
use fp_model::attrs::Sensitivity;
use fp_model::data::{DataDict, DataEntry, DataList, DataValue};
use fp_model::element::BitmapNode;
use fp_model::{Fill, Texture};
use serde_json::{json, Value};
use std::sync::Arc;

fn bad(p: &Params, m: impl Into<String>) -> EngineError {
    EngineError::InvalidParams { name: p.command.clone(), message: m.into() }
}

/// JSON → customData-Wert (Zahlen im `%.17g`-Format des Originals).
pub fn data_value(v: &Value) -> DataValue {
    match v {
        Value::Null => DataValue::None,
        Value::Bool(b) => DataValue::Scalar(if *b { "true".into() } else { "false".into() }),
        Value::Number(n) => DataValue::Scalar(fp_model::data::format_g17(n.as_f64().unwrap_or(0.0))),
        Value::String(s) => DataValue::Scalar(s.clone()),
        Value::Array(a) => DataValue::List(DataList { items: a.iter().map(data_value).collect(), extra: Default::default() }),
        Value::Object(o) => DataValue::Dict(DataDict {
            entries: o.iter().map(|(k, x)| DataEntry { tag: Default::default(), key: k.clone(), value: data_value(x), text: None, extra: Default::default() }).collect(),
            extra: Default::default(),
        }),
    }
}

/// customData-Wert → JSON (Skalare typisiert nach der Leseregel des Originals).
pub fn data_json(v: &DataValue) -> Value {
    match v {
        DataValue::None => Value::Null,
        DataValue::Scalar(_) => match v.typed() {
            Some(fp_model::data::TypedScalar::Bool(b)) => json!(b),
            Some(fp_model::data::TypedScalar::Number(n)) if n.fract() == 0.0 && n.abs() < 9.0e15 => json!(n as i64),
            Some(fp_model::data::TypedScalar::Number(n)) => json!(n),
            Some(fp_model::data::TypedScalar::String(s)) => json!(s),
            None => Value::Null,
        },
        DataValue::List(l) => Value::Array(l.items.iter().map(data_json).collect()),
        DataValue::Dict(d) => dict_json(d),
        DataValue::Points(p) => Value::Array(p.points.iter().map(|q| json!({"x": q.x, "y": q.y})).collect()),
    }
}

/// Wörterbuch → JSON-Objekt.
pub fn dict_json(d: &DataDict) -> Value {
    let mut m = serde_json::Map::new();
    for e in &d.entries {
        m.entry(e.key.clone()).or_insert_with(|| data_json(&e.value));
    }
    Value::Object(m)
}

/// Bild einer Datei (PNG, JPEG, GIF, BMP, TIFF, WebP …) als RGBA.
pub fn load_image_file(path: &str) -> Result<fp_raster::RgbaImage> {
    let doc = super::file::load_document(std::path::Path::new(path))?;
    let r = crate::export_backend::ExportRenderer(None);
    use fp_export::render::PageRenderer as _;
    r.render(&doc, &fp_export::render::RenderRequest { background: fp_export::render::Background::Transparent, ..Default::default() }).map_err(EngineError::Failed)
}

/// Graustufenmaske (Textur) aus einem Bild: Helligkeit × Deckung, Kacheln 256 × 256 wie `MSKv`.
pub fn texture_mask(img: &fp_raster::RgbaImage) -> fp_model::MaskData {
    let (w, h) = (img.width(), img.height());
    let ts = 256usize;
    let (cols, rows) = (w.div_ceil(ts), h.div_ceil(ts));
    let mut tiles = Vec::with_capacity(cols * rows);
    let mut id = 1u32;
    for ty in 0..rows {
        for tx in 0..cols {
            let mut buf = vec![0u8; ts * ts];
            for yy in 0..ts.min(h - ty * ts) {
                for xx in 0..ts.min(w - tx * ts) {
                    let p = img.data()[(ty * ts + yy) * w + tx * ts + xx];
                    let l = (p[0] as u32 * 299 + p[1] as u32 * 587 + p[2] as u32 * 114) / 1000;
                    buf[yy * ts + xx] = (l * p[3] as u32 / 255) as u8;
                }
            }
            tiles.push(fp_model::image::TileEntry::Tile(Arc::new(fp_model::image::Tile {
                id,
                kind: fp_model::image::TileKind::Mask,
                pixels: buf.into(),
                compressed: None,
                reserved: None,
            })));
            id += 1;
        }
    }
    fp_model::MaskData {
        grid: fp_model::image::TileGrid { width: w as u32, height: h as u32, tile_size: ts as u32, tiles, tiles_extra: Default::default() },
        extra: Default::default(),
    }
}

fn edit_targets(s: &mut crate::session::Session, p: &Params, f: impl Fn(&mut fp_model::Element)) -> Result<CommandOutput> {
    with_doc(s, |d| {
        let (page, ids) = (d.page, target_ids(d, p)?);
        let doc = Arc::make_mut(&mut d.doc);
        for id in ids {
            docops::edit_by_id(doc, page, id, &f).ok_or(EngineError::ElementNotFound(id.0))?;
        }
        Ok(CommandOutput::doc())
    })
}

pub(super) fn register(r: &mut CommandRegistry) {
    r.add(
        cmd("element.setCustomData")
            .label("Eigene Daten")
            .requires(DOC)
            .opt("ids", ParamKind::Ids, "Elemente (Vorgabe: Auswahl)")
            .opt("data", ParamKind::Json, "Schlüssel → Wert (Zahl, Text, Wahrheitswert, Liste, Objekt; null entfernt)")
            .opt_default("replace", ParamKind::Bool, json!(false), "vorhandene Daten vollständig ersetzen")
            .js("customData")
            .run(|s, p| {
                let Some(data) = p.get("data").cloned() else {
                    // Lesen
                    let d = s.doc()?;
                    let ids = target_ids(d, p)?;
                    let out: Vec<Value> = ids.iter().filter_map(|id| docops::element_on_page(&d.doc, d.page, *id)).map(|e| json!({"id": e.id.0, "data": dict_json(&e.common.custom_data)})).collect();
                    return Ok(CommandOutput::value(Value::Array(out)));
                };
                let Value::Object(m) = data else { return Err(bad(p, "data: Objekt erwartet")) };
                let replace = p.bool_or("replace", false)?;
                edit_targets(s, p, |e| {
                    let cd = &mut e.common.custom_data;
                    if replace {
                        cd.entries.clear();
                    }
                    for (k, v) in &m {
                        match v {
                            Value::Null => {
                                cd.remove(k);
                            }
                            Value::Array(_) | Value::Object(_) => cd.set_structured(k, data_value(v)),
                            Value::Bool(b) => cd.set_bool(k, *b),
                            Value::Number(n) => cd.set_number(k, n.as_f64().unwrap_or(0.0)),
                            Value::String(t) => cd.set_scalar(k, t.clone()),
                        }
                    }
                })
            }),
    );
    r.add(
        cmd("layer.setExpanded")
            .label("Ebene aufklappen")
            .requires(DOC)
            .opt("layer", ParamKind::Integer { min: Some(0), max: None }, "Ebenen-ID (Vorgabe: aktive Ebene)")
            .param("expanded", ParamKind::Bool, "aufgeklappt")
            .js("disclosure")
            .run(|s, p| {
                let v = p.bool_or("expanded", true)?;
                with_doc(s, |d| {
                    let id = match p.opt_i64("layer")? {
                        Some(x) => fp_model::LayerId(x as u64),
                        None => d.layer.ok_or_else(|| EngineError::NotFound("aktive Ebene".into()))?,
                    };
                    let page = d.page;
                    let slot = docops::layer_slot(&d.doc, page, id).ok_or_else(|| EngineError::NotFound("Ebene".into()))?;
                    let l = docops::layer_mut(Arc::make_mut(&mut d.doc), page, slot).ok_or_else(|| EngineError::NotFound("Ebene".into()))?;
                    l.expanded = v;
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(
        cmd("document.setOrigin")
            .label("Leinwandursprung")
            .requires(DOC)
            .opt("left", ParamKind::Number { min: None, max: None }, "Leinwand links (Document.left)")
            .opt("top", ParamKind::Number { min: None, max: None }, "Leinwand oben (Document.top)")
            .opt("page", ParamKind::Integer { min: Some(0), max: None }, "Seite (Vorgabe aktuelle)")
            .run(|s, p| {
                let (l, t) = (p.opt_f64("left")?, p.opt_f64("top")?);
                with_doc(s, |d| {
                    let page = p.opt_i64("page")?.map(|v| v.max(0) as usize).unwrap_or(d.page);
                    let pg = d.doc_mut().page_mut(page).ok_or_else(|| EngineError::NotFound("Seite".into()))?;
                    if let Some(l) = l {
                        pg.canvas.origin.x = l as f32;
                    }
                    if let Some(t) = t {
                        pg.canvas.origin.y = t as f32;
                    }
                    let o = pg.canvas.origin;
                    d.doc_mut().invalidate_root_copy();
                    Ok(CommandOutput::doc_with(json!({"left": o.x, "top": o.y})))
                })
            }),
    );
    r.add(
        cmd("element.setPattern")
            .label("Musterfüllung")
            .requires(DOC)
            .opt("ids", ParamKind::Ids, "Elemente (Vorgabe: Auswahl)")
            .param("image", ParamKind::Path, "Musterbild (PNG, JPEG, GIF …)")
            .opt("name", ParamKind::String, "Mustername (Vorgabe: Dateiname)")
            .run(|s, p| {
                let path = p.str("image")?;
                let img = load_image_file(&path)?;
                let name = p.opt_str("name")?.unwrap_or_else(|| std::path::Path::new(&path).file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Muster".into()));
                // feste Kachelnummern: der Schreiber nummeriert ohnehin neu, so bleibt das Ergebnis reproduzierbar
                let mut next = 1u32;
                let node = Arc::new(BitmapNode::from_rgba(fp_geom::Point::new(0.0, 0.0), img.width() as u32, img.height() as u32, img.as_bytes(), &mut next));
                edit_targets(s, p, |e| {
                    let node = Arc::clone(&node);
                    let name = name.clone();
                    docops::edit_path_attrs(e, &move |a| {
                        let mut f = a.fill.clone().unwrap_or_else(Fill::solid);
                        f.category = "fc_Pattern".into();
                        f.name = name.clone();
                        f.user_name = Some(name.clone());
                        f.shape = 15;
                        f.shape_ext = 0;
                        f.gradient = None;
                        f.pattern = Some(fp_model::attrs::Pattern { name: name.clone(), user_name: Some(name.clone()), image: Some(Arc::clone(&node)), extra: Default::default() });
                        a.fill = Some(f);
                    });
                })
            }),
    );
    r.add(
        cmd("element.setTexture")
            .label("Textur")
            .requires(DOC)
            .opt("ids", ParamKind::Ids, "Elemente (Vorgabe: Auswahl)")
            .opt_default("target", ParamKind::Enum { values: vec!["fill".into(), "stroke".into()] }, json!("fill"), "Füllungs- oder Strichtextur")
            .opt("image", ParamKind::Path, "Texturbild (Helligkeit = Maske); ohne: Textur entfernen")
            .opt("name", ParamKind::String, "Texturname")
            .opt("amount", ParamKind::Integer { min: Some(0), max: Some(100) }, "Texturmenge in %")
            .run(|s, p| {
                let stroke = p.opt_str("target")?.as_deref() == Some("stroke");
                let tex = match p.opt_str("image")? {
                    Some(path) => {
                        let img = load_image_file(&path)?;
                        let name = p.opt_str("name")?.unwrap_or_else(|| std::path::Path::new(&path).file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Textur".into()));
                        Some(Texture { name: name.clone(), user_name: Some(name), mask: Some(texture_mask(&img)), extra: Default::default() })
                    }
                    None => None,
                };
                let amount = p.opt_i64("amount")?.map(|v| v as i32 * 10);
                edit_targets(s, p, |e| {
                    let tex = tex.clone();
                    docops::edit_path_attrs(e, &move |a| {
                        if stroke {
                            a.stroke_texture = tex.clone();
                            if let (Some(m), Some(b)) = (amount, a.brush.as_mut()) {
                                b.texture_blend = m;
                            }
                        } else {
                            a.fill_texture = tex.clone();
                            if let (Some(m), Some(f)) = (amount, a.fill.as_mut()) {
                                f.texture_amount = m;
                            }
                        }
                    });
                })
            }),
    );
    r.add(
        cmd("element.setBrushSensitivity")
            .label("Strich-Empfindlichkeit")
            .requires(DOC)
            .opt("ids", ParamKind::Ids, "Elemente (Vorgabe: Auswahl)")
            .param("values", ParamKind::Json, "{\"PZ\": 50, …}: Quelle P Druck/S Geschwindigkeit/H horizontal/V vertikal/R Zufall × Ziel Z Größe/A Winkel/O Deckkraft/B Schwärze/R Streuung/H Farbton/L Helligkeit/S Sättigung, Wert in % (0 entfernt)")
            .run(|s, p| {
                let Some(Value::Object(m)) = p.get("values").cloned() else { return Err(bad(p, "values: Objekt erwartet")) };
                let mut list = Vec::new();
                for (k, v) in &m {
                    let b = k.as_bytes();
                    if b.len() != 2 || !fp_model::attrs::SENSITIVITY_SOURCES.contains(&b[0]) || !fp_model::attrs::SENSITIVITY_TARGETS.contains(&b[1]) {
                        return Err(bad(p, format!("ungültiger Schlüssel {k}")));
                    }
                    let pct = v.as_f64().ok_or_else(|| bad(p, format!("{k}: Zahl erwartet")))?;
                    list.push((b[0], b[1], (pct.clamp(0.0, 100.0) * 10.0).round() as i32));
                }
                edit_targets(s, p, |e| {
                    let list = list.clone();
                    docops::edit_path_attrs(e, &move |a| {
                        let Some(br) = a.brush.as_mut() else { return };
                        for &(src, dst, val) in &list {
                            br.sensitivities.retain(|x| !(x.source == src && x.target == dst));
                            if val > 0 {
                                br.sensitivities.push(Sensitivity { source: src, target: dst, value: val });
                            }
                        }
                    });
                })
            }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_roundtrip() {
        let v = json!({"a": 1.5, "b": true, "c": "x", "d": [1, "zwei", {"e": 3}]});
        let Value::Object(m) = &v else { return };
        let mut d = DataDict::default();
        for (k, x) in m {
            d.entries.push(DataEntry { tag: Default::default(), key: k.clone(), value: data_value(x), text: None, extra: Default::default() });
        }
        assert_eq!(dict_json(&d), v);
    }
}
