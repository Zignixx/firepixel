//! Feineinstellungen der KI-Auswahl (KI-2, Nutzerbefund N21): Toleranz, Maskenvariante, Kante,
//! Ausdehnen/Zusammenziehen und Glätten wirken; `retune: true` bildet die letzte Auswahl aus der
//! zwischengespeicherten Modell-Ausgabe neu, ohne ein Modell zu laden oder zu rechnen.
//!
//! Braucht die echten Modelle (sonst übersprungen). Die Tests dieser Datei laufen nacheinander
//! (gemeinsame Sperre), damit „kein Modell geladen“ eindeutig prüfbar ist. Eigener Prozess
//! (`[[test]]` im `Cargo.toml`): der KI-Zwischenspeicher ist prozessweit, im gemeinsamen Programm
//! `all` laden und entladen Tests anderer Dateien gleichzeitig Modelle.

mod common;

use common::*;
use fp_engine::Session;
use fp_engine::commands::ai;
use fp_geom::IRect;
use fp_raster::{Mask, RgbaImage};
use serde_json::json;
use std::sync::Mutex;

static LOCK: Mutex<()> = Mutex::new(());

const W: i32 = 400;
const H: i32 = 300;
const DISK: (f32, f32, f32) = (130.0, 150.0, 60.0);
const SQUARE: (i32, i32, i32, i32) = (260, 80, 350, 170);

fn have_models() -> bool {
    use ai::fp_ai::models::{ModelId, available};
    available(ModelId::Sam2Tiny) && available(ModelId::BiRefNetLite)
}

macro_rules! need_models {
    () => {
        if !have_models() {
            eprintln!("KI-Modelle fehlen – Test übersprungen");
            return;
        }
    };
}

fn test_image() -> RgbaImage {
    let mut img =
        RgbaImage::new(IRect::from_xywh(0, 0, W, H)).unwrap_or_else(|_| RgbaImage::empty_at(0, 0));
    let w = img.width();
    for (i, px) in img.data_mut().iter_mut().enumerate() {
        let (x, y) = ((i % w) as i32, (i / w) as i32);
        let n = ((x * 7919 + y * 104_729) ^ (x * y)) & 63;
        let stripe = if (x / 11 + y / 7) % 2 == 0 { 24 } else { 0 };
        let mut c = [70 + n as u8 / 2 + stripe, 110 + n as u8, 60 + stripe, 255];
        let (dx, dy) = (x as f32 - DISK.0, y as f32 - DISK.1);
        if dx * dx + dy * dy <= DISK.2 * DISK.2 {
            c = [225, 40 + (n as u8 / 4), 35, 255];
        }
        if x >= SQUARE.0 && x < SQUARE.2 && y >= SQUARE.1 && y < SQUARE.3 {
            c = [35, 70, 205 + (n as u8 / 8), 255];
        }
        *px = c;
    }
    img
}

fn setup() -> Session {
    ai::fp_ai::download::disable();
    let mut s = doc_session(W as f32, H as f32);
    s.clipboard.clear();
    s.clipboard.pixels = Some(test_image());
    ex(&mut s, "edit.paste", json!({}));
    ex(&mut s, "select.none", json!({}));
    s
}

fn sel(s: &Session) -> Option<Mask> {
    s.active().and_then(|d| d.selection.pixels.clone())
}

fn area(m: &Option<Mask>) -> u64 {
    m.as_ref()
        .map(|m| m.data().iter().map(|v| *v as u64).sum::<u64>() / 255)
        .unwrap_or(0)
}

fn bounds(m: &Option<Mask>) -> IRect {
    m.as_ref().map(|m| m.tight_bounds()).unwrap_or_default()
}

const CLICK: [f32; 2] = [DISK.0, DISK.1];

#[test]
fn object_tolerance_expand_edge_and_variant_work() {
    need_models!();
    let _g = LOCK.lock();
    let mut s = setup();
    let v = ex(&mut s, "select.aiObject", json!({"point": CLICK}));
    assert_eq!(v["tuning"]["tolerance"], 50.0);
    let base = sel(&s);
    let b0 = bounds(&base);
    assert!((b0.width() as i32 - 121).abs() <= 6, "Scheibe {b0:?}");
    // Ausdehnen / Zusammenziehen um 8 px: Grenzen wandern um 8 px
    ex(
        &mut s,
        "select.aiObject",
        json!({"point": CLICK, "expand": 8}),
    );
    let e = bounds(&sel(&s));
    assert!(
        (e.left - (b0.left - 8)).abs() <= 1 && (e.right - (b0.right + 8)).abs() <= 1,
        "{b0:?} → {e:?}"
    );
    ex(
        &mut s,
        "select.aiObject",
        json!({"point": CLICK, "expand": -8}),
    );
    let c = bounds(&sel(&s));
    assert!(
        (c.left - (b0.left + 8)).abs() <= 1 && (c.right - (b0.right - 8)).abs() <= 1,
        "{b0:?} → {c:?}"
    );
    // Kante hart: nur 0/255; weiche Kante: breiter Übergang
    ex(
        &mut s,
        "select.aiObject",
        json!({"point": CLICK, "edge": "hard"}),
    );
    let h = sel(&s).expect("Auswahl");
    assert!(h.data().iter().all(|v| *v == 0 || *v == 255));
    let soft = |m: &Mask| m.data().iter().filter(|v| **v > 0 && **v < 255).count();
    ex(
        &mut s,
        "select.aiObject",
        json!({"point": CLICK, "edge": "feather", "feather": 8}),
    );
    let f = sel(&s).expect("Auswahl");
    assert!(
        soft(&f) > 4 * soft(base.as_ref().expect("Auswahl")),
        "weiche Kante {} vs {}",
        soft(&f),
        soft(base.as_ref().expect("Auswahl"))
    );
    // altes `feather` ohne `edge` bleibt eine weiche Kante
    ex(
        &mut s,
        "select.aiObject",
        json!({"point": CLICK, "feather": 8}),
    );
    assert_eq!(sel(&s), Some(f));
    // Toleranz: höher wählt nicht weniger aus, niedriger nicht mehr
    ex(
        &mut s,
        "select.aiObject",
        json!({"point": CLICK, "tolerance": 95}),
    );
    let hi = area(&sel(&s));
    ex(
        &mut s,
        "select.aiObject",
        json!({"point": CLICK, "tolerance": 5}),
    );
    let lo = area(&sel(&s));
    let mid = area(&base);
    assert!(
        lo <= mid && mid <= hi && lo < hi,
        "Toleranz 5/50/95: {lo} {mid} {hi}"
    );
    // Varianten: klein ≤ mittel ≤ groß
    let mut areas = Vec::new();
    for v in ["small", "medium", "large"] {
        ex(
            &mut s,
            "select.aiObject",
            json!({"point": CLICK, "variant": v}),
        );
        areas.push(area(&sel(&s)));
    }
    assert!(areas[0] <= areas[1] && areas[1] <= areas[2], "{areas:?}");
    // unbekannte Werte werden abgewiesen
    assert!(
        s.execute("select.aiObject", json!({"point": CLICK, "edge": "wobbly"}))
            .is_err()
    );
    assert!(
        s.execute(
            "select.aiObject",
            json!({"point": CLICK, "variant": "huge"})
        )
        .is_err()
    );
}

#[test]
fn retune_recomputes_without_running_a_model() {
    need_models!();
    let _g = LOCK.lock();
    let mut s = setup();
    let v = ex(&mut s, "select.aiObject", json!({"point": CLICK}));
    let first = sel(&s);
    let token = v["session"].as_u64().unwrap_or(0);
    // Modelle entladen und Zwischenspeicher leeren: jedes Modell-Laufen müsste jetzt neu laden
    ai::fp_ai::cache::clear_results();
    ai::fp_ai::cache::unload_all();
    let t = std::time::Instant::now();
    let r = ex(
        &mut s,
        "select.aiObject",
        json!({"retune": true, "expand": 6}),
    );
    assert_eq!(r["retuned"], true);
    assert_eq!(r["session"].as_u64(), Some(token));
    let e = bounds(&sel(&s));
    assert!((e.left - (bounds(&first).left - 6)).abs() <= 1, "{e:?}");
    // 50 × denselben Wert: dasselbe Ergebnis (keine Anhäufung)
    let once = sel(&s);
    for _ in 0..50 {
        ex(
            &mut s,
            "select.aiObject",
            json!({"retune": true, "expand": 6}),
        );
    }
    assert_eq!(sel(&s), once);
    // zurück auf die Vorgabe: exakt die erste Auswahl
    ex(
        &mut s,
        "select.aiObject",
        json!({"retune": true, "expand": 0}),
    );
    assert_eq!(sel(&s), first);
    assert!(
        !ai::fp_ai::cache::is_loaded(ai::fp_ai::models::ModelId::Sam2Tiny),
        "kein Modell geladen"
    );
    assert!(t.elapsed().as_secs_f32() < 10.0);
    // Auswahl geändert → nichts mehr nachzustellen
    ex(&mut s, "select.none", json!({}));
    assert!(!ai::can_retune(&s, "select.aiObject"));
    assert!(
        s.execute("select.aiObject", json!({"retune": true, "expand": 3}))
            .is_err()
    );
    assert_eq!(sel(&s), None);
}

#[test]
fn retune_keeps_the_combination_mode_and_uses_preferences() {
    need_models!();
    let _g = LOCK.lock();
    let mut s = setup();
    ex(
        &mut s,
        "select.aiObject",
        json!({"box": [SQUARE.0 - 8, SQUARE.1 - 8, SQUARE.2 + 8, SQUARE.3 + 8]}),
    );
    let square = sel(&s);
    ex(
        &mut s,
        "select.aiObject",
        json!({"point": CLICK, "mode": "add"}),
    );
    let both = sel(&s);
    assert!(area(&both) > area(&square));
    // Wert aus den Voreinstellungen (Inspektor), wenn kein Parameter
    s.prefs
        .set("AiObjectExpand", fp_engine::PrefValue::Int(-10));
    ex(&mut s, "select.aiObject", json!({"retune": true}));
    let m = sel(&s).expect("Auswahl");
    // das Quadrat (Basis) bleibt unverändert, nur die Scheibe schrumpft
    let sq = square.as_ref().expect("Quadrat");
    assert_eq!(
        m.get(SQUARE.0 + 2, SQUARE.1 + 2),
        sq.get(SQUARE.0 + 2, SQUARE.1 + 2)
    );
    assert_eq!(
        m.get((DISK.0 - DISK.2 + 4.0) as i32, DISK.1 as i32),
        0,
        "Scheibe zusammengezogen"
    );
    assert!(area(&sel(&s)) < area(&both));
    s.prefs.set("AiObjectExpand", fp_engine::PrefValue::Int(0));
}

#[test]
fn subject_tolerance_edge_and_retune() {
    need_models!();
    let _g = LOCK.lock();
    let mut s = setup();
    ex(&mut s, "select.aiSubject", json!({}));
    let first = sel(&s);
    assert!(area(&first) > 0);
    assert!(ai::can_retune(&s, "select.aiSubject"));
    assert!(!ai::can_retune(&s, "select.aiObject"));
    ex(
        &mut s,
        "select.aiSubject",
        json!({"retune": true, "tolerance": 90}),
    );
    let hi = area(&sel(&s));
    ex(
        &mut s,
        "select.aiSubject",
        json!({"retune": true, "tolerance": 10}),
    );
    let lo = area(&sel(&s));
    assert!(
        lo <= area(&first) && area(&first) <= hi && lo < hi,
        "{lo} {} {hi}",
        area(&first)
    );
    ex(
        &mut s,
        "select.aiSubject",
        json!({"retune": true, "tolerance": 50, "edge": "hard"}),
    );
    assert!(
        sel(&s)
            .expect("Motiv")
            .data()
            .iter()
            .all(|v| *v == 0 || *v == 255)
    );
    ex(
        &mut s,
        "select.aiSubject",
        json!({"retune": true, "edge": "antiAlias", "expand": 5, "smooth": 3}),
    );
    assert!(area(&sel(&s)) > area(&first));
    ex(
        &mut s,
        "select.aiSubject",
        json!({"retune": true, "edge": "antiAlias"}),
    );
    assert_eq!(sel(&s), first, "zurück auf die Vorgabe");
    // Motiv-Auswahl hat keine Maskenvariante: wird ignoriert, nicht abgewiesen
    ex(
        &mut s,
        "select.aiSubject",
        json!({"retune": true, "variant": "small"}),
    );
    assert_eq!(sel(&s), first);
}

#[test]
fn retune_without_ai_selection_fails_cleanly() {
    let _g = LOCK.lock();
    let mut s = setup();
    assert!(!ai::can_retune(&s, "select.aiObject"));
    assert!(
        s.execute("select.aiObject", json!({"retune": true}))
            .is_err()
    );
    assert!(
        s.execute("select.aiSubject", json!({"retune": true}))
            .is_err()
    );
    assert!(
        ai::prepare(&s, "select.aiObject", &json!({"retune": true}))
            .ok()
            .flatten()
            .is_none(),
        "keine Hintergrundrechnung"
    );
}
