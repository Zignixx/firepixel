//! Logo-Nachbearbeitung an synthetischen Logos (Kreis, Rechteck, abgerundetes Rechteck,
//! schriftähnliche Balken; geglättet, mit Rauschen, als hochskaliertes JPEG): erkannte Formen,
//! Knotenzahl, Pixelübereinstimmung gegenüber dem Ablauf ohne Nachbearbeitung, exakt
//! achsparallele Geraden, gemeinsame Kanten, Symmetrie, Laufzeit.
//! Zahlen mit `cargo test -p fp-trace --test refine -- --nocapture`.

use fp_color::Rgba8;
use fp_geom::{BezierContour, BezierNode, IRect, Point};
use fp_raster::RgbaImage;
use fp_trace::*;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

const WHITE: [u8; 4] = [255, 255, 255, 255];
const BLUE: [u8; 4] = [20, 60, 200, 255];
const RED: [u8; 4] = [220, 30, 40, 255];
const GREEN: [u8; 4] = [30, 160, 80, 255];
const DARK: [u8; 4] = [40, 40, 48, 255];
const CLEAR: [u8; 4] = [0, 0, 0, 0];
const MAGENTA: [u8; 4] = [255, 0, 255, 255];

/// Bild aus einer Farbfunktion mit 4×4-Überabtastung (geglättete Kanten), optional Rauschen ±8.
fn supersample(bounds: IRect, noise: bool, f: impl Fn(f64, f64) -> [u8; 4]) -> RgbaImage {
    let mut img = RgbaImage::new(bounds).expect("Bild");
    let w = img.width();
    for (i, px) in img.data_mut().iter_mut().enumerate() {
        let (x, y) = ((i % w) as f64, (i / w) as f64);
        let mut acc = [0u32; 4];
        for sy in 0..4 {
            for sx in 0..4 {
                let c = f(x + (sx as f64 + 0.5) / 4.0, y + (sy as f64 + 0.5) / 4.0);
                let a = c[3] as u32;
                for k in 0..3 {
                    acc[k] += c[k] as u32 * a;
                }
                acc[3] += a;
            }
        }
        let a = acc[3];
        *px = match (acc[0].checked_div(a), acc[1].checked_div(a), acc[2].checked_div(a)) {
            (Some(r), Some(g), Some(b)) => [r as u8, g as u8, b as u8, (a / 16) as u8],
            _ => [0, 0, 0, 0],
        };
        if noise && px[3] > 0 {
            let hsh = ((i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 40) as i32;
            for (k, v) in px.iter_mut().take(3).enumerate() {
                let n = ((hsh >> (k * 5)) & 15) - 8;
                *v = (*v as i32 + n).clamp(0, 255) as u8;
            }
        }
    }
    img
}

fn rrect(x: f64, y: f64, l: f64, t: f64, r: f64, b: f64, rad: f64) -> bool {
    if x < l || x > r || y < t || y > b {
        return false;
    }
    let (cx, cy) = (x.clamp(l + rad, r - rad), y.clamp(t + rad, b - rad));
    (x - cx).hypot(y - cy) <= rad
}

/// Logo im Maßstab `s` (Grundgröße 512 × 512): blauer Kreis, rotes Rechteck, grünes abgerundetes
/// Rechteck, dunkles „E“ (rechte Winkel), schräger Strich (73°) und 45°-Strich auf `bg`.
fn logo_fn(s: f64, bg: [u8; 4]) -> impl Fn(f64, f64) -> [u8; 4] {
    move |x, y| {
        let (x, y) = (x / s, y / s);
        let e = ((40.0..64.0).contains(&x) && (270.0..440.0).contains(&y))
            || ((40.0..160.0).contains(&x) && (270.0..294.0).contains(&y))
            || ((40.0..140.0).contains(&x) && (343.0..367.0).contains(&y))
            || ((40.0..160.0).contains(&x) && (416.0..440.0).contains(&y));
        let slash = ((x - 180.0) - (440.0 - y) * 0.3).abs() < 12.0 && (270.0..440.0).contains(&y);
        let diag = ((x - y) + 200.0).abs() < 10.0 && (450.0..500.0).contains(&y);
        if (x - 130.0).hypot(y - 140.0) <= 90.0 {
            BLUE
        } else if (262.0..470.0).contains(&x) && (60.0..200.0).contains(&y) {
            RED
        } else if rrect(x, y, 250.0, 250.0, 480.0, 400.0, 30.0) {
            GREEN
        } else if e || slash || diag {
            DARK
        } else {
            bg
        }
    }
}

fn logo(s: f64, bg: [u8; 4], noise: bool) -> RgbaImage {
    let n = (512.0 * s) as i32;
    supersample(IRect::from_xywh(0, 0, n, n), noise, logo_fn(s, bg))
}

fn over(p: [u8; 4], bg: [u8; 4]) -> [u8; 3] {
    let a = p[3] as u32;
    let f = |c: u8, b: u8| ((c as u32 * a + b as u32 * (255 - a)) / 255) as u8;
    [f(p[0], bg[0]), f(p[1], bg[1]), f(p[2], bg[2])]
}

fn match_ratio(orig: &RgbaImage, traced: &RgbaImage, bg: [u8; 4], tol: i32) -> f64 {
    assert_eq!(orig.bounds(), traced.bounds());
    let ok = orig.data().iter().zip(traced.data()).filter(|(a, b)| {
        let (ca, cb) = (over(**a, bg), over(**b, bg));
        (0..3).all(|k| (ca[k] as i32 - cb[k] as i32).abs() <= tol)
    });
    ok.count() as f64 / orig.data().len() as f64
}

fn unrefined(mut o: TraceOptions) -> TraceOptions {
    o.refine.enabled = false;
    o
}

fn run(name: &str, img: &RgbaImage, o: &TraceOptions) -> (TraceResult, Option<RefineReport>, f64) {
    let t = Instant::now();
    let (r, rep) = trace_with_report(img, o, &AtomicBool::new(false), &mut |_| {}).expect("nachzeichnen");
    let ms = t.elapsed().as_secs_f64() * 1000.0;
    eprintln!("{name}: {}×{} in {ms:.0} ms – {} Pfade, {} Konturen, {} Knoten", img.width(), img.height(), r.paths.len(), r.contour_count(), r.node_count());
    if let Some(p) = &rep {
        eprintln!(
            "  Nachbearbeitung: Knoten {} → {}, Formen: {} Kreis(e), {} Ellipse(n), {} Rechteck(e), {} abgerundet; Geraden {} (eingerastet {}), gemeinsame Kanten {}, Palette {:?}, Symmetrie {:?}",
            p.nodes_before,
            p.nodes_after,
            p.circles,
            p.ellipses,
            p.rectangles,
            p.rounded_rectangles,
            p.lines,
            p.snapped_lines,
            p.shared_edges,
            p.palette.iter().map(|c| (c.r, c.g, c.b)).collect::<Vec<_>>(),
            p.symmetry
        );
    }
    (r, rep, ms)
}

fn near(c: Rgba8, t: [u8; 4], tol: i32) -> bool {
    (c.r as i32 - t[0] as i32).abs() <= tol && (c.g as i32 - t[1] as i32).abs() <= tol && (c.b as i32 - t[2] as i32).abs() <= tol
}

fn path_of(r: &TraceResult, c: [u8; 4]) -> Vec<&TracedPath> {
    r.paths.iter().filter(|p| near(p.color, c, 6)).collect()
}

fn is_line(a: &BezierNode, b: &BezierNode) -> bool {
    a.ctrl_out == a.pt && b.ctrl_in == b.pt
}

/// Jede Gerade, die höchstens 4° von waagerecht/senkrecht abweicht, ist exakt achsparallel.
fn assert_axis_exact(r: &TraceResult) -> usize {
    let mut n = 0;
    for p in &r.paths {
        for c in &p.contours {
            let k = c.nodes.len();
            for i in 0..k {
                let (a, b) = (&c.nodes[i], &c.nodes[(i + 1) % k]);
                if !is_line(a, b) {
                    continue;
                }
                let (dx, dy) = ((b.pt.x - a.pt.x) as f64, (b.pt.y - a.pt.y) as f64);
                if dx.hypot(dy) < 3.0 {
                    continue;
                }
                let ang = dy.atan2(dx).to_degrees().rem_euclid(90.0);
                if ang.min(90.0 - ang) <= 4.0 {
                    assert!(dx == 0.0 || dy == 0.0, "Gerade nicht exakt achsparallel: {:?} → {:?}", a.pt, b.pt);
                    n += 1;
                }
            }
        }
    }
    n
}

/// Ergebnis im Maßstab `s` rastern (Lückensuche).
fn render_scaled(r: &TraceResult, s: f32) -> RgbaImage {
    let map = |p: Point| Point::new((p.x - r.bounds.left as f32) * s, (p.y - r.bounds.top as f32) * s);
    let paths = r
        .paths
        .iter()
        .map(|p| TracedPath {
            contours: p.contours.iter().map(|c| BezierContour::new(c.nodes.iter().map(|n| BezierNode::new(map(n.pt), map(n.ctrl_in), map(n.ctrl_out))).collect(), true)).collect(),
            color: p.color,
            pixels: p.pixels,
        })
        .collect();
    let (w, h) = ((r.bounds.width() as f32 * s) as u32, (r.bounds.height() as f32 * s) as u32);
    TraceResult { paths, bounds: IRect::from_xywh(0, 0, w as i32, h as i32), traced_width: w, traced_height: h }.render()
}

fn transparent_pixels(img: &RgbaImage) -> usize {
    img.data().iter().filter(|p| p[3] == 0).count()
}

#[test]
fn logo_shapes_lines_and_nodes() {
    let img = logo(1.0, WHITE, true);
    let o = TraceOptions::from_preset(Preset::Logo);
    assert!(o.refine.enabled, "Vorgabe Logo: Nachbearbeitung an");
    let (r, rep, _) = run("Logo (Nachbearbeitung)", &img, &o);
    let rep = rep.expect("Kennzahlen");
    let (r0, rep0, _) = run("Logo (ohne Nachbearbeitung)", &img, &unrefined(o.clone()));
    assert!(rep0.is_none());

    // Formen
    assert!(rep.circles >= 1 && rep.rectangles >= 1 && rep.rounded_rectangles >= 1, "{rep:?}");
    // Palette: genau die 5 Originalfarben (±3 trotz Rauschen ±8)
    assert_eq!(rep.palette.len(), 5, "{:?}", rep.palette);
    for c in [WHITE, BLUE, RED, GREEN, DARK] {
        assert!(rep.palette.iter().any(|p| near(*p, c, 3)), "Farbe {c:?} fehlt in {:?}", rep.palette);
    }
    // Kreis: 4 Knoten, Mitte (130, 140), Radius 90
    let blue = path_of(&r, BLUE);
    assert_eq!(blue.len(), 1);
    let circle = &blue[0].contours[0];
    assert_eq!(circle.nodes.len(), 4, "{circle:?}");
    let cx = circle.nodes.iter().map(|n| n.pt.x).sum::<f32>() / 4.0;
    let cy = circle.nodes.iter().map(|n| n.pt.y).sum::<f32>() / 4.0;
    assert!((cx - 130.0).abs() <= 0.5 && (cy - 140.0).abs() <= 0.5, "Mitte {cx}, {cy}");
    for n in &circle.nodes {
        let rr = (n.pt.x - cx).hypot(n.pt.y - cy);
        assert!((rr - 90.0).abs() <= 0.75, "Radius {rr}");
    }
    // Rechteck exakt
    let red = path_of(&r, RED);
    let rect = &red[0].contours[0];
    let mut pts: Vec<(f32, f32)> = rect.nodes.iter().map(|n| (n.pt.x, n.pt.y)).collect();
    pts.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(pts, vec![(262.0, 60.0), (262.0, 200.0), (470.0, 60.0), (470.0, 200.0)]);
    // abgerundetes Rechteck: 8 Knoten
    let green = path_of(&r, GREEN);
    assert_eq!(green[0].contours[0].nodes.len(), 8);
    // dunkle Zeichen: „E“ 12 Ecken, Striche je 4
    let dark = path_of(&r, DARK);
    let mut counts: Vec<usize> = dark.iter().map(|p| p.node_count()).collect();
    counts.sort();
    eprintln!("  dunkle Formen: {counts:?} Knoten");
    assert_eq!(counts, vec![4, 4, 12]);

    // Knoten deutlich weniger
    eprintln!("  Knoten: ohne {} → mit {} ({:.0} %)", r0.node_count(), r.node_count(), 100.0 * r.node_count() as f64 / r0.node_count() as f64);
    assert!(r.node_count() * 3 <= r0.node_count(), "Knoten {} vs. {}", r.node_count(), r0.node_count());
    assert_eq!(rep.nodes_after, r.node_count());
    assert!(rep.nodes_before > 5 * rep.nodes_after);

    // Pixelübereinstimmung nicht schlechter
    let ideal = logo(1.0, WHITE, false);
    let m = match_ratio(&ideal, &r.render(), WHITE, 40);
    let m0 = match_ratio(&ideal, &r0.render(), WHITE, 40);
    eprintln!("  Übereinstimmung (±40 gegen das unverrauschte Bild): mit {:.2} %, ohne {:.2} %", m * 100.0, m0 * 100.0);
    assert!(m >= 0.99 && m + 0.0005 >= m0, "mit {m}, ohne {m0}");

    // Geraden exakt achsparallel
    let n_axis = assert_axis_exact(&r);
    eprintln!("  achsparallele Geraden (alle exakt): {n_axis}");
    assert!(n_axis >= 4 + 12 + 4 + 2);
    // 45°-Strich: lange Kanten exakt diagonal
    let diag = dark.iter().find(|p| p.contours[0].nodes.iter().all(|n| n.pt.y >= 449.0)).expect("45°-Strich");
    let c = &diag.contours[0];
    let k = c.nodes.len();
    let diagonals = (0..k).filter(|&i| {
        let (a, b) = (c.nodes[i].pt, c.nodes[(i + 1) % k].pt);
        (b.x - a.x).abs() > 10.0 && (b.x - a.x).abs() == (b.y - a.y).abs()
    });
    assert_eq!(diagonals.count(), 2, "{c:?}");
}

#[test]
fn shared_edges_close_gaps() {
    // Ausschneiden: Flächen mit Löchern, Nachbarn teilen dieselbe Kante → bei 4facher Rasterung
    // keine durchscheinenden (transparenten) Pixel.
    let img = logo(1.0, WHITE, true);
    let mut o = TraceOptions::from_preset(Preset::Logo);
    o.hierarchical = Hierarchy::Cutout;
    let (r, rep, _) = run("Logo ausschneiden (Nachbearbeitung)", &img, &o);
    let rep = rep.expect("Kennzahlen");
    assert!(rep.shared_edges >= 6, "{rep:?}");
    let bg = path_of(&r, WHITE);
    assert!(bg.iter().map(|p| p.contours.len()).sum::<usize>() >= 6, "Hintergrund ohne Löcher");
    let gaps = transparent_pixels(&render_scaled(&r, 4.0));
    let (r0, _, _) = run("Logo ausschneiden (ohne)", &img, &unrefined(o.clone()));
    let gaps0 = transparent_pixels(&render_scaled(&r0, 4.0));
    eprintln!("  Lücken bei 4facher Rasterung: mit {gaps}, ohne {gaps0} Pixel");
    assert_eq!(gaps, 0);
    let m = match_ratio(&logo(1.0, WHITE, false), &r.render(), WHITE, 40);
    eprintln!("  Übereinstimmung ausschneiden: {:.2} %", m * 100.0);
    assert!(m >= 0.99);

    // Lücken schließen aus: jede Fläche eigene Ketten
    o.refine.close_gaps = false;
    let (_, rep2, _) = run("Logo ausschneiden (ohne gemeinsame Kanten)", &img, &o);
    assert_eq!(rep2.expect("Kennzahlen").shared_edges, 0);

    // Stapeln: Löcher unter den kleineren Flächen entfallen
    o.hierarchical = Hierarchy::Stacked;
    o.refine.close_gaps = true;
    let (r, _, _) = run("Logo stapeln (Nachbearbeitung)", &img, &o);
    assert_eq!(path_of(&r, WHITE)[0].contours.len(), 1);
    assert_eq!(transparent_pixels(&render_scaled(&r, 4.0)), 0);
}

#[test]
fn transparent_background() {
    let img = logo(1.0, CLEAR, true);
    let o = TraceOptions::from_preset(Preset::Logo);
    let (r, rep, _) = run("Logo transparent (Nachbearbeitung)", &img, &o);
    let rep = rep.expect("Kennzahlen");
    assert_eq!(rep.palette.len(), 4, "{:?}", rep.palette);
    assert!(r.paths.iter().all(|p| !near(p.color, WHITE, 30)));
    let out = r.render();
    assert_eq!(out.get(1, 1).map(|p| p[3]), Some(0));
    let m = match_ratio(&logo(1.0, CLEAR, false), &out, MAGENTA, 40);
    let (r0, _, _) = run("Logo transparent (ohne)", &img, &unrefined(o.clone()));
    let m0 = match_ratio(&logo(1.0, CLEAR, false), &r0.render(), MAGENTA, 40);
    eprintln!("  Übereinstimmung transparent: mit {:.2} %, ohne {:.2} %", m * 100.0, m0 * 100.0);
    assert!(m >= 0.99 && m + 0.0005 >= m0);
}

/// Logo klein zeichnen, als JPEG (Qualität 70) speichern, laden und 4fach bilinear vergrößern.
fn jpeg_upscaled() -> RgbaImage {
    let small = logo(0.5, WHITE, false);
    let rgb: Vec<u8> = small.data().iter().flat_map(|p| [p[0], p[1], p[2]]).collect();
    let mut buf = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 70).encode(&rgb, 256, 256, image::ExtendedColorType::Rgb8).expect("JPEG");
    let dec = image::load_from_memory(&buf).expect("JPEG lesen").to_rgba8();
    let img = RgbaImage::from_rgba_bytes(IRect::from_xywh(0, 0, 256, 256), dec.as_raw()).expect("Bild");
    fp_raster::resample::resize(&img, 1024, 1024, fp_raster::Sampler::Bilinear)
}

#[test]
fn upscaled_jpeg_logo() {
    let img = jpeg_upscaled();
    let o = TraceOptions::from_preset(Preset::Logo);
    let (r, rep, _) = run("JPEG 256→1024 (Nachbearbeitung)", &img, &o);
    let rep = rep.expect("Kennzahlen");
    let (r0, _, _) = run("JPEG 256→1024 (ohne)", &img, &unrefined(o.clone()));
    assert!(rep.palette.len() <= 6, "Palette {:?}", rep.palette);
    assert!(rep.shape_count() >= 2, "{rep:?}");
    assert!(r.node_count() * 3 <= r0.node_count(), "Knoten {} vs. {}", r.node_count(), r0.node_count());
    let ideal = logo(2.0, WHITE, false);
    let m = match_ratio(&ideal, &r.render(), WHITE, 40);
    let m0 = match_ratio(&ideal, &r0.render(), WHITE, 40);
    eprintln!("  Knoten ohne {} → mit {}; Übereinstimmung gegen das ideale 1024er-Logo: mit {:.2} %, ohne {:.2} %", r0.node_count(), r.node_count(), m * 100.0, m0 * 100.0);
    assert!(m >= 0.99 && m + 0.0005 >= m0, "mit {m}, ohne {m0}");
    assert_axis_exact(&r);
}

#[test]
fn symmetric_logo() {
    // Schild: Rechteck oben, Spitze unten, Kreis in der Mitte, zwei gespiegelte Schrägbalken;
    // links etwas Rauschen, damit die Hälften nicht schon gleich sind.
    let f = |x: f64, y: f64| -> [u8; 4] {
        let dx = (x - 200.0).abs();
        let shield = (60.0..330.0).contains(&y) && dx <= 120.0 - (y - 220.0).max(0.0) * 0.75;
        if (x - 200.0).hypot(y - 170.0) <= 50.0 {
            WHITE
        } else if ((dx - 60.0) - (y - 250.0) * 0.5).abs() < 8.0 && (240.0..300.0).contains(&y) {
            RED
        } else if shield {
            BLUE
        } else {
            WHITE
        }
    };
    let img = supersample(IRect::from_xywh(0, 0, 400, 400), true, f);
    let mut o = TraceOptions::from_preset(Preset::Logo);
    o.refine.symmetry = true;
    let (r, rep, _) = run("Schild (Symmetrie)", &img, &o);
    let rep = rep.expect("Kennzahlen");
    assert_eq!(rep.symmetry, Some(Symmetry::Vertical));
    // Ankerpunkte je Farbe liegen spiegelbildlich zur Achse x = 200 (auch zwischen getrennten Flächen)
    for p in &r.paths {
        let pts: Vec<Point> = r.paths.iter().filter(|q| q.color == p.color).flat_map(|q| q.contours.iter().flat_map(|c| c.nodes.iter().map(|n| n.pt))).collect();
        for q in &pts {
            let m = Point::new(400.0 - q.x, q.y);
            assert!(pts.iter().any(|z| (z.x - m.x).abs() <= 0.011 && (z.y - m.y).abs() <= 0.011), "kein Spiegelpunkt zu {q:?} in {pts:?}");
        }
    }
    let m = match_ratio(&supersample(IRect::from_xywh(0, 0, 400, 400), false, f), &r.render(), WHITE, 40);
    eprintln!("  Übereinstimmung Schild: {:.2} %", m * 100.0);
    assert!(m >= 0.99);
}

#[test]
fn options_and_compat() {
    // ohne `refine` im JSON: aus (alte gespeicherte Einstellungen bleiben gültig)
    let mut v = serde_json::to_value(TraceOptions::from_preset(Preset::Logo)).expect("json");
    assert_eq!(v["refine"]["enabled"], serde_json::Value::Bool(true));
    v.as_object_mut().expect("Objekt").remove("refine");
    let o: TraceOptions = serde_json::from_value(v).expect("json");
    assert!(!o.refine.enabled);
    assert_eq!(o.colors, 16);
    let o: TraceOptions = serde_json::from_str(r#"{"preset":"logo","refine":{"enabled":true,"symmetry":true}}"#).expect("json");
    assert!(o.refine.enabled && o.refine.symmetry && o.refine.shapes);
    assert!(TraceOptions::from_preset(Preset::Logo).refine.enabled);
    for p in [Preset::Schwarzweiss, Preset::Illustration, Preset::Foto] {
        assert!(!TraceOptions::from_preset(p).refine.enabled, "{p:?}");
    }
    let mut ro = RefineOptions::logo();
    ro.fit_tolerance = f64::NAN;
    ro.color_tolerance = 1000.0;
    ro.snap_angle = -3.0;
    let v = ro.validated();
    assert_eq!((v.fit_tolerance, v.color_tolerance, v.snap_angle), (1.0, 128.0, 0.0));
    // trace_with_report ohne Nachbearbeitung: keine Kennzahlen
    let img = logo(0.5, WHITE, false);
    let (_, rep) = trace_with_report(&img, &TraceOptions::from_preset(Preset::Illustration), &AtomicBool::new(false), &mut |_| {}).expect("ok");
    assert!(rep.is_none());
    // Polygon-Modus: nur Ecken, Rechteck trotzdem erkannt
    let mut o = TraceOptions::from_preset(Preset::Logo);
    o.mode = CurveMode::Polygon;
    let (r, rep, _) = run("Logo Polygon (Nachbearbeitung)", &logo(1.0, WHITE, true), &o);
    assert!(r.paths.iter().flat_map(|p| &p.contours).flat_map(|c| &c.nodes).all(|n| n.is_corner()));
    assert!(rep.expect("Kennzahlen").rectangles >= 1);
    // Schwarzweiß mit Nachbearbeitung
    let mut o = TraceOptions::from_preset(Preset::Schwarzweiss);
    o.refine.enabled = true;
    let (r, rep, _) = run("Schwarzweiß (Nachbearbeitung)", &logo(1.0, WHITE, true), &o);
    assert!(r.paths.iter().all(|p| p.color == Rgba8::new(0, 0, 0, 255)));
    assert!(rep.expect("Kennzahlen").rectangles >= 1);
    // einzelne Schalter aus: läuft, Ergebnis plausibel
    let img = logo(1.0, WHITE, true);
    for k in 0..5 {
        let mut o = TraceOptions::from_preset(Preset::Logo);
        match k {
            0 => o.refine.snap_colors = false,
            1 => o.refine.straighten = false,
            2 => o.refine.shapes = false,
            3 => o.refine.reduce_nodes = false,
            _ => o.refine.fast = true,
        }
        let (r, _, _) = run(&format!("Logo, Schalter {k} aus/an"), &img, &o);
        let m = match_ratio(&logo(1.0, WHITE, false), &r.render(), WHITE, 40);
        eprintln!("  Übereinstimmung {:.2} %", m * 100.0);
        assert!(m >= 0.985, "Schalter {k}: {m}");
    }
}

#[test]
fn timing_1024() {
    let img = logo(2.0, WHITE, true);
    let o = TraceOptions::from_preset(Preset::Logo);
    let (r, _, ms) = run("1024² Logo (Nachbearbeitung)", &img, &o);
    let mut fast = o.clone();
    fast.refine.fast = true;
    let (_, _, ms_fast) = run("1024² Logo (schnelle Vorschau)", &img, &fast);
    let (r0, _, ms0) = run("1024² Logo (ohne)", &img, &unrefined(o.clone()));
    eprintln!("  Laufzeit 1024²: mit {ms:.0} ms, schnell {ms_fast:.0} ms, ohne {ms0:.0} ms; Knoten {} vs. {}", r.node_count(), r0.node_count());
    assert!(ms < 20_000.0);
    let m = match_ratio(&logo(2.0, WHITE, false), &r.render(), WHITE, 40);
    assert!(m >= 0.99, "{m}");
}

#[test]
fn cancel_refine() {
    let img = logo(1.0, WHITE, true);
    let cancel = AtomicBool::new(false);
    let mut seen = Vec::new();
    let r = trace_with_report(&img, &TraceOptions::default(), &cancel, &mut |p| seen.push(p));
    assert!(r.is_ok());
    assert!(seen.windows(2).all(|w| w[1].fraction >= w[0].fraction), "Fortschritt nicht monoton");
    assert_eq!(seen.last().map(|p| (p.stage, p.fraction)), Some((Stage::Done, 1.0)));
    let r = trace_with_report(&img, &TraceOptions::default(), &cancel, &mut |p| {
        if p.stage == Stage::Paths {
            cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    });
    assert_eq!(r.map(|_| ()), Err(TraceError::Cancelled));
}
