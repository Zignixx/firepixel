//! F147 (FB-131 … FB-135): a live move drag reuses the looks of the moved elements. With
//! [`fp_render::MovedLooks`] a moved element (new allocation, moved by whole pixels) takes the
//! cached look of its source shifted by the offset: no cache miss, no new effect chain, and the
//! page is pixel-identical to a fresh render of the moved document. Looks that depend on the
//! document position (dithered fills, Screentone) are not paired and render normally.

use std::sync::Arc;

use fp_color::Argb;
use fp_geom::Rect;
use fp_model::{Document, Effect, EffectList, Element, Fill, LayerSlot, PathAttrs};
use fp_render::{MovedLooks, Renderer, look_moves_along};

/// Effect list of the moved rectangle.
fn effects(kind: &str) -> Option<EffectList> {
    let mut list = EffectList::default();
    let mut add = |moa: &str, params: &[(&str, &str)]| {
        let mut e = Effect::new(moa, true);
        for (k, v) in params {
            e.set(k, *v);
        }
        list.effects.push(e);
    };
    match kind {
        "bevel" => add(fp_effects::chain::moa::BEVEL, &[("BevelType", "0")]),
        "glow" => add(
            fp_effects::chain::moa::BEVEL,
            &[("BevelType", "4"), ("BevelWidth", "10")],
        ),
        "shadow" => add(fp_effects::chain::moa::DROP_SHADOW, &[]),
        "innerglow" => add(fp_effects::chain::moa::INNER_GLOW, &[]),
        "screentone" => add(fp_effects::chain::moa::SCREENTONE, &[]),
        _ => return None,
    }
    Some(list)
}

/// Page 400 × 300: a blue rectangle below, the moved rectangle M (`attrs`, effects `fx`) moved by
/// `d`, a grey rectangle with blend mode Multiply above. Fresh ids, so M has the same id in every
/// variant.
fn doc(fx: &str, attrs: PathAttrs, d: (f32, f32)) -> Document {
    let mut doc = Document::new(400.0, 300.0, Argb(0xffff_ffff));
    let below = Element::new_rectangle(
        &mut doc.ids,
        Rect::new(20.0, 20.0, 200.0, 200.0),
        0.0,
        PathAttrs::solid_fill(Argb(0xff33_66ff)),
    );
    let r = Rect::new(80.5 + d.0, 60.5 + d.1, 230.5 + d.0, 160.5 + d.1);
    let mut m = Element::new_rectangle(&mut doc.ids, r, 0.0, attrs);
    m.common.effects = effects(fx);
    let mut above = Element::new_rectangle(
        &mut doc.ids,
        Rect::new(180.0, 40.0, 330.0, 240.0),
        0.0,
        PathAttrs::solid_fill(Argb(0xff80_8080)),
    );
    above.common.blend = fp_color::BlendMode(1);
    let page = Arc::make_mut(&mut doc.pages[0]);
    if let LayerSlot::Own(l) = &mut page.root.layers[0] {
        Arc::make_mut(&mut Arc::make_mut(l).cels[0]).elements =
            vec![Arc::new(above), Arc::new(m), Arc::new(below)];
    }
    doc
}

/// The moved rectangle (index 1 of the first cel).
fn m_of(doc: &Document) -> Arc<Element> {
    match &doc.pages[0].root.layers[0] {
        LayerSlot::Own(l) => Arc::clone(&l.cels[0].elements[1]),
        _ => panic!("layer"),
    }
}

/// `base` with M replaced by the M of `moved` (the other elements keep their allocations, like
/// the render-only copy of the canvas).
fn with_m(base: &Document, moved: &Document) -> Document {
    let mut out = base.clone();
    let page = Arc::make_mut(&mut out.pages[0]);
    if let LayerSlot::Own(l) = &mut page.root.layers[0] {
        Arc::make_mut(&mut Arc::make_mut(l).cels[0]).elements[1] = m_of(moved);
    }
    out
}

/// (effect, fill attributes) of a position-dependent case.
type Case = (&'static str, fn() -> PathAttrs);

fn solid() -> PathAttrs {
    PathAttrs::solid_fill(Argb(0xffff_cc00))
}

fn check_reuse(fx: &str) {
    let base = doc(fx, solid(), (0.0, 0.0));
    let r = Renderer::default();
    let start = r.render_page(&base, 0, 0).image;
    let src = m_of(&base);
    for (step, d) in [(37, 23), (38, 23), (52, -11)].into_iter().enumerate() {
        let moved = with_m(&base, &doc(fx, solid(), (d.0 as f32, d.1 as f32)));
        let expected = Renderer::default().render_page(&moved, 0, 0).image;
        assert_ne!(
            expected.as_bytes(),
            start.as_bytes(),
            "{fx}: the move shows"
        );
        let ml = MovedLooks::new(&moved, &[(m_of(&moved), Arc::clone(&src))], d);
        assert_eq!(ml.len(), 1, "{fx}: M is paired");
        let before = r.with_cache(|c| c.stats());
        let got = r.with_moved_looks(&ml, |r| r.render_page(&moved, 0, 0).image);
        let after = r.with_cache(|c| c.stats());
        assert_eq!(
            after.misses, before.misses,
            "{fx} step {step}: no cache miss"
        );
        assert_eq!(
            after.moved_hits,
            before.moved_hits + 1,
            "{fx} step {step}: M from its source"
        );
        let diff = got
            .as_bytes()
            .iter()
            .zip(expected.as_bytes())
            .filter(|(a, b)| a != b)
            .count();
        assert_eq!(
            diff, 0,
            "{fx} step {step}: equals a fresh render of the move"
        );
    }
    // the pairs are gone after the render: the moved M is a new element again
    let moved = with_m(&base, &doc(fx, solid(), (37.0, 23.0)));
    let before = r.with_cache(|c| c.stats());
    let _ = r.render_page(&moved, 0, 0);
    assert!(r.with_cache(|c| c.stats()).misses > before.misses);
}

#[test]
fn moved_look_reused_plain() {
    check_reuse("none");
}

#[test]
fn moved_look_reused_inner_bevel() {
    check_reuse("bevel");
}

#[test]
fn moved_look_reused_glow() {
    check_reuse("glow");
}

#[test]
fn moved_look_reused_drop_shadow() {
    check_reuse("shadow");
}

#[test]
fn moved_look_reused_inner_glow() {
    check_reuse("innerglow");
}

/// Looks that depend on the document position are not paired; the render stays exact.
#[test]
fn position_dependent_looks_render_normally() {
    let dither_edge = || {
        let mut a = solid();
        if let Some(f) = a.fill.as_mut() {
            f.edge = Fill::EDGE_DITHER;
            f.feather = 6;
        }
        a
    };
    let density = || {
        let mut a = solid();
        if let Some(f) = a.fill.as_mut() {
            f.dither_density = 40;
        }
        a
    };
    let cases: [Case; 3] = [
        ("screentone", solid),
        ("none", dither_edge),
        ("none", density),
    ];
    for (fx, attrs) in cases {
        let base = doc(fx, attrs(), (0.0, 0.0));
        assert!(!look_moves_along(&base, &m_of(&base)));
        let moved = with_m(&base, &doc(fx, attrs(), (37.0, 23.0)));
        let ml = MovedLooks::new(&moved, &[(m_of(&moved), m_of(&base))], (37, 23));
        assert!(ml.is_empty(), "not paired");
        let r = Renderer::default();
        let _ = r.render_page(&base, 0, 0);
        let got = r.with_moved_looks(&ml, |r| r.render_page(&moved, 0, 0).image);
        let expected = Renderer::default().render_page(&moved, 0, 0).image;
        assert_eq!(got.as_bytes(), expected.as_bytes());
    }
    // a plain rectangle moves along
    let base = doc("bevel", solid(), (0.0, 0.0));
    assert!(look_moves_along(&base, &m_of(&base)));
}
