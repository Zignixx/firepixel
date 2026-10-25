//! FB-38 (Firepixel extensions): the uniform dither density of a fill is stored only in the
//! Firepixel-own marker `fpDi` of the fill, the Screentone live filter only as a Firepixel-own
//! effect entry `fpLv` in the effect list (instead of an `EPSv`). Both round-trip; with the default
//! values (density 0, no Firepixel filter) the file carries none of the markers, i.e. it stays a
//! plain file of the reference format.

use std::collections::HashMap;
use std::sync::Arc;

use fp_color::Argb;
use fp_fwpng::read::{ReadCtx, read_effect_list, read_fill};
use fp_fwpng::write::{
    CachePolicy, EFFECT_EXT, FILL_DENSITY_EXT, WriteCtx, write_effect_list, write_fill,
};
use fp_geom::Rect;
use fp_model::effects::{Effect, EffectKind, EffectList, SCREENTONE_MOA_ID};
use fp_model::{
    Document, Element, ElementKind, Fill, LayerSlot, PathAttrs, RawNode, RawValue, TagKey,
};

const DROP_SHADOW: &str = "{a7944db8-6ce2-11d1-8c76000502701850}";
const GAUSSIAN: &str = "{d04ef8c0-71b3-11d1-8c8200a024cdc039}";

fn int_of(nodes: &[RawNode], key: &[u8; 4]) -> Option<i32> {
    nodes
        .iter()
        .find(|n| n.key == TagKey::new(key))
        .and_then(|n| match n.value {
            RawValue::Int(v) => Some(v as i32),
            _ => None,
        })
}

fn fill_back(nodes: &[RawNode]) -> Fill {
    let tiles = HashMap::new();
    let mut ctx = ReadCtx::new(&tiles);
    read_fill(nodes, &mut ctx)
}

fn effects_back(nodes: &[RawNode]) -> EffectList {
    let tiles = HashMap::new();
    let mut ctx = ReadCtx::new(&tiles);
    read_effect_list(nodes, &mut ctx)
}

/// Effect identity without the tag-order bookkeeping of the reader: (key, text) pairs in order.
fn pairs(e: &Effect) -> Vec<(String, Option<String>)> {
    e.params
        .entries
        .iter()
        .map(|d| (d.key.clone(), d.text_value().map(|t| t.to_string())))
        .collect()
}

fn all_pairs(l: &EffectList) -> Vec<Vec<(String, Option<String>)>> {
    l.effects.iter().map(pairs).collect()
}

fn screentone() -> Effect {
    let mut e = Effect::new(SCREENTONE_MOA_ID, true);
    e.set("screentone_type", "1");
    e.set("screentone_size", "12");
    e.set("screentone_angle", "30");
    e.set("screentone_keep_color", "true");
    e
}

#[test]
fn dither_density_lives_in_its_marker() {
    let mut f = Fill::solid();
    f.dither_density = 40;
    let mut w = WriteCtx::new(CachePolicy::Keep, Vec::new());
    let nodes = write_fill(&f, &mut w);
    assert_eq!(int_of(&nodes, b"fpDi"), Some(40));
    let back = fill_back(&nodes);
    assert_eq!(back.dither_density, 40);
    assert!(
        !back.extra.has_raw(FILL_DENSITY_EXT),
        "the marker is consumed, not kept as an unknown tag"
    );
    // a reader without the marker sees the plain fill
    let plain: Vec<RawNode> = nodes
        .iter()
        .filter(|n| n.key != FILL_DENSITY_EXT)
        .cloned()
        .collect();
    let mut seen = fill_back(&plain);
    seen.extra = Default::default();
    assert_eq!(seen, Fill::solid());
    // density 0: no marker, the fill nodes are exactly those of a plain fill
    let mut w = WriteCtx::new(CachePolicy::Keep, Vec::new());
    let zero = write_fill(&Fill::solid(), &mut w);
    assert_eq!(int_of(&zero, b"fpDi"), None);
    assert_eq!(zero, plain);
}

#[test]
fn screentone_is_a_marker_entry_in_place() {
    let list = EffectList {
        effects: vec![
            Effect::new(DROP_SHADOW, true),
            screentone(),
            Effect::new(GAUSSIAN, false),
        ],
        ..EffectList::default()
    };
    let nodes = write_effect_list(&list);
    let tags: Vec<[u8; 4]> = nodes
        .iter()
        .map(|n| n.key.0)
        .filter(|k| k == b"EPSv" || k == b"fpLv")
        .collect();
    assert_eq!(tags, vec![*b"EPSv", *b"fpLv", *b"EPSv"]);
    let back = effects_back(&nodes);
    assert_eq!(
        all_pairs(&back),
        all_pairs(&list),
        "order, parameters and visibility round-trip"
    );
    assert_eq!(back.effects[1].kind(), EffectKind::Screentone);
    assert!(back.effects[1].is_firepixel() && !back.effects[0].is_firepixel());
    assert!(!back.extra.has_raw(EFFECT_EXT));
    // a reader that skips the marker sees only the effects of the reference format
    let plain: Vec<RawNode> = nodes
        .iter()
        .filter(|n| n.key != EFFECT_EXT)
        .cloned()
        .collect();
    let seen = effects_back(&plain);
    assert_eq!(seen.effects.len(), 2);
    assert!(seen.effects.iter().all(|e| !e.is_firepixel()));
}

fn doc_with(fill: Fill, effects: Vec<Effect>) -> Document {
    let mut doc = Document::new(100.0, 80.0, Argb(0xffff_ffff));
    let mut attrs = PathAttrs::solid_fill(Argb(0xff00_0000));
    attrs.fill = Some(fill);
    let mut e = Element::new_rectangle(&mut doc.ids, Rect::new(10.0, 10.0, 60.0, 50.0), 0.0, attrs);
    e.common.effects = Some(EffectList {
        effects,
        ..EffectList::default()
    });
    let p = Arc::make_mut(&mut doc.pages[0]);
    if let LayerSlot::Own(l) = &mut p.root.layers[0] {
        Arc::make_mut(&mut Arc::make_mut(l).cels[0]).elements = vec![Arc::new(e)];
    }
    doc
}

fn bytes_of(doc: &Document) -> Vec<u8> {
    fp_fwpng::write(
        doc,
        &fp_fwpng::WriteOptions::default(),
        &fp_fwpng::NoFlattener,
    )
    .expect("write")
}

fn tags(nodes: &[RawNode], out: &mut Vec<[u8; 4]>) {
    for n in nodes {
        out.push(n.key.0);
        tags(n.children(), out);
    }
}

fn all_tags(bytes: &[u8]) -> Vec<[u8; 4]> {
    let raw = fp_fwpng::read_raw(bytes, false).expect("raw");
    let mut v = Vec::new();
    for t in [&raw.ts, &raw.bs].into_iter().flatten() {
        tags(t, &mut v);
    }
    v
}

#[test]
fn document_round_trip_keeps_density_and_screentone() {
    let mut f = Fill::solid();
    f.dither_density = 35;
    let doc = doc_with(f, vec![Effect::new(DROP_SHADOW, true), screentone()]);
    let bytes = bytes_of(&doc);
    let t = all_tags(&bytes);
    assert!(t.contains(b"fpDi") && t.contains(b"fpLv"));
    let back = fp_fwpng::read(&bytes).expect("read");
    let LayerSlot::Own(l) = &back.pages[0].root.layers[0] else {
        panic!("no layer")
    };
    let e = &l.cels[0].elements[0];
    let ElementKind::Path(p) = &e.kind else {
        panic!("no path")
    };
    assert_eq!(p.attrs.fill.as_ref().expect("fill").dither_density, 35);
    let fx = e.common.effects.as_ref().expect("effects");
    assert_eq!(fx.effects.len(), 2);
    assert_eq!(pairs(&fx.effects[1]), pairs(&screentone()));
}

#[test]
fn default_values_leave_the_reference_format_untouched() {
    let plain = doc_with(Fill::solid(), vec![Effect::new(DROP_SHADOW, true)]);
    let bytes = bytes_of(&plain);
    let t = all_tags(&bytes);
    for k in [b"fpDi", b"fpLv", b"fpEi"] {
        assert!(!t.contains(k), "{}", String::from_utf8_lossy(k));
    }
    // set and reset: a file with both extensions, read back, density set to 0 and the Screentone
    // filter removed, is written without any marker again
    let mut f = Fill::solid();
    f.dither_density = 60;
    let marked = bytes_of(&doc_with(
        f,
        vec![Effect::new(DROP_SHADOW, true), screentone()],
    ));
    let mut doc = fp_fwpng::read(&marked).expect("read");
    let page = Arc::make_mut(&mut doc.pages[0]);
    if let LayerSlot::Own(l) = &mut page.root.layers[0] {
        let cel = Arc::make_mut(&mut Arc::make_mut(l).cels[0]);
        let e = Arc::make_mut(&mut cel.elements[0]);
        if let ElementKind::Path(p) = &mut e.kind {
            Arc::make_mut(&mut p.attrs)
                .fill
                .as_mut()
                .expect("fill")
                .dither_density = 0;
        }
        if let Some(l) = &mut e.common.effects {
            l.effects.retain(|x| !x.is_firepixel());
        }
    }
    // the document's fill list keeps a copy of the used fill as well
    for f in page.presets.fills.iter_mut().flatten() {
        f.dither_density = 0;
    }
    // the stored copy of the current page (`MKBv`) is rebuilt from the edited page, as the
    // engine does after an edit
    doc.root_copy = None;
    let t = all_tags(&bytes_of(&doc));
    for k in [b"fpDi", b"fpLv"] {
        assert!(
            !t.contains(k),
            "after reset: {}",
            String::from_utf8_lossy(k)
        );
    }
    // reading the plain file gives density 0 and no Firepixel filter
    let back = fp_fwpng::read(&bytes).expect("read");
    let LayerSlot::Own(l) = &back.pages[0].root.layers[0] else {
        panic!("no layer")
    };
    let e = &l.cels[0].elements[0];
    let ElementKind::Path(p) = &e.kind else {
        panic!("no path")
    };
    assert_eq!(p.attrs.fill.as_ref().expect("fill").dither_density, 0);
    assert!(
        e.common
            .effects
            .as_ref()
            .is_some_and(|l| l.effects.iter().all(|x| !x.is_firepixel()))
    );
}

/// FB-86: the Screentone flag "anti-alias" (`screentone_anti_alias`) round-trips in the `fpLv`
/// entry; a file written before the flag existed has no key, which reads as off (hard dots).
#[test]
fn screentone_anti_alias_round_trips_and_is_optional() {
    let mut on = screentone();
    on.set("screentone_anti_alias", "true");
    let list = EffectList {
        effects: vec![on],
        ..EffectList::default()
    };
    let back = effects_back(&write_effect_list(&list));
    assert_eq!(all_pairs(&back), all_pairs(&list));
    assert_eq!(back.effects[0].get("screentone_anti_alias"), Some("true"));
    assert_eq!(
        back.effects[0].get_bool("screentone_anti_alias"),
        Some(true)
    );
    // older file: no key at all
    let old = EffectList {
        effects: vec![screentone()],
        ..EffectList::default()
    };
    let back = effects_back(&write_effect_list(&old));
    assert_eq!(back.effects[0].get("screentone_anti_alias"), None);
    assert_eq!(back.effects[0].kind(), EffectKind::Screentone);
}
