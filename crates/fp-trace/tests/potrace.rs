//! Zweites Verfahren „Konturen“ (`engine = potrace`, VT-6): Golden-Tests an einfachen Formen
//! (Quadrat, Kreis, Buchstaben T und O), glatte Kurven mit wenigen Knoten, Flecken, Abbiegeregel,
//! Eckenschwelle, Kurvenoptimierung, Farbebenen, Transparenz, Wiederholbarkeit, Abbruch und der
//! Vergleich beider Verfahren an einer Strichzeichnung (Zeit, Knoten; `--nocapture`).

use fp_color::Rgba8;
use fp_geom::{BezierContour, IRect, Point};
use fp_raster::RgbaImage;
use fp_trace::*;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

/// Bild aus einer Farbfunktion mit 4×4-Überabtastung (geglättete Kanten).
fn supersample(bounds: IRect, f: impl Fn(f64, f64) -> [u8; 4]) -> RgbaImage {
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
    }
    img
}

/// Bild ohne Kantenglättung (Pixelmitte).
fn pixels(bounds: IRect, f: impl Fn(i32, i32) -> [u8; 4]) -> RgbaImage {
    let mut img = RgbaImage::new(bounds).expect("Bild");
    let w = img.width();
    for (i, px) in img.data_mut().iter_mut().enumerate() {
        *px = f((i % w) as i32, (i / w) as i32);
    }
    img
}

const WHITE: [u8; 4] = [255, 255, 255, 255];
const BLACK: [u8; 4] = [0, 0, 0, 255];
const BLUE: [u8; 4] = [20, 60, 200, 255];
const RED: [u8; 4] = [220, 30, 40, 255];
const CLEAR: [u8; 4] = [0, 0, 0, 0];

fn bw() -> TraceOptions {
    TraceOptions { engine: Engine::Potrace, ..TraceOptions::from_preset(Preset::BlackWhite) }
}

fn color() -> TraceOptions {
    TraceOptions { engine: Engine::Potrace, ..TraceOptions::from_preset(Preset::Illustration) }
}

fn is_corner(n: &fp_geom::BezierNode) -> bool {
    n.ctrl_in == n.pt && n.ctrl_out == n.pt
}

/// Punkte auf der Kontur (je Segment `k` Stützstellen).
fn sample(c: &BezierContour, k: usize) -> Vec<Point> {
    let n = c.nodes.len();
    let mut out = Vec::new();
    for i in 0..n {
        let (a, b) = (&c.nodes[i], &c.nodes[(i + 1) % n]);
        for s in 0..k {
            let t = s as f32 / k as f32;
            let m = 1.0 - t;
            let f = |p0: f32, p1: f32, p2: f32, p3: f32| m * m * m * p0 + 3.0 * m * m * t * p1 + 3.0 * m * t * t * p2 + t * t * t * p3;
            out.push(Point::new(f(a.pt.x, a.ctrl_out.x, b.ctrl_in.x, b.pt.x), f(a.pt.y, a.ctrl_out.y, b.ctrl_in.y, b.pt.y)));
        }
    }
    out
}

/// Anteil der Pixel (%), deren Belegung (Schwarzweiß: dunkel ↔ gedeckt) übereinstimmt.
fn bw_match(src: &RgbaImage, res: &TraceResult) -> f64 {
    let r = res.render();
    let same = src.data().iter().zip(r.data()).filter(|(s, d)| {
        let dark = s[3] >= 128 && (299 * s[0] as u32 + 587 * s[1] as u32 + 114 * s[2] as u32) / 1000 < 128;
        dark == (d[3] >= 128)
    });
    same.count() as f64 * 100.0 / src.data().len() as f64
}

/// Anteil der Pixel (%), deren Farbe höchstens 40 je Kanal abweicht (transparent = transparent).
fn color_match(src: &RgbaImage, res: &TraceResult) -> f64 {
    let r = res.render();
    let same = src.data().iter().zip(r.data()).filter(|(s, d)| match (s[3] >= 128, d[3] >= 128) {
        (false, false) => true,
        (true, true) => (0..3).all(|k| (s[k] as i32 - d[k] as i32).abs() <= 40),
        _ => false,
    });
    same.count() as f64 * 100.0 / src.data().len() as f64
}

#[test]
fn square_is_four_exact_corners() {
    let img = pixels(IRect::from_xywh(0, 0, 100, 100), |x, y| if (20..70).contains(&x) && (30..80).contains(&y) { BLACK } else { WHITE });
    let res = trace(&img, &bw()).expect("nachzeichnen");
    assert_eq!(res.paths.len(), 1);
    assert_eq!(res.contour_count(), 1);
    let c = &res.paths[0].contours[0];
    assert_eq!(c.nodes.len(), 4, "{:?}", c.nodes);
    assert!(c.nodes.iter().all(is_corner), "Ecken ohne Henkel");
    let mut pts: Vec<(i32, i32)> = c.nodes.iter().map(|n| (n.pt.x.round() as i32, n.pt.y.round() as i32)).collect();
    pts.sort();
    assert_eq!(pts, vec![(20, 30), (20, 80), (70, 30), (70, 80)]);
    assert!(c.nodes.iter().all(|n| (n.pt.x - n.pt.x.round()).abs() < 1e-3 && (n.pt.y - n.pt.y.round()).abs() < 1e-3));
    assert_eq!(bw_match(&img, &res), 100.0);
}

#[test]
fn circle_is_smooth_with_few_nodes() {
    let img = supersample(IRect::from_xywh(10, 20, 120, 120), |x, y| if (x - 60.0).hypot(y - 60.0) <= 40.0 { BLACK } else { WHITE });
    let res = trace(&img, &bw()).expect("nachzeichnen");
    assert_eq!(res.contour_count(), 1);
    let c = &res.paths[0].contours[0];
    assert!((3..=8).contains(&c.nodes.len()), "{} Knoten", c.nodes.len());
    assert!(c.nodes.iter().all(|n| !is_corner(n)), "keine Ecken am Kreis");
    // Kurve liegt auf dem Kreis (Dokumentkoordinaten: Ursprung 10/20)
    for p in sample(c, 16) {
        let r = ((p.x - 70.0) as f64).hypot((p.y - 80.0) as f64);
        assert!((r - 40.0).abs() < 0.75, "Radius {r}");
    }
    assert!(bw_match(&img, &res) > 99.0);
    // ohne Kurvenoptimierung: ein Bogen je Polygonpunkt → deutlich mehr Knoten
    let mut o = bw();
    o.potrace.opt_curve = false;
    let plain = trace(&img, &o).expect("nachzeichnen");
    assert!(plain.node_count() > 2 * res.node_count(), "{} gegen {}", plain.node_count(), res.node_count());
    assert!(bw_match(&img, &plain) > 99.0);
}

#[test]
fn letters_t_and_o() {
    // T: zwei Rechtecke → 8 Ecken
    let t = pixels(IRect::from_xywh(0, 0, 80, 90), |x, y| if ((10..70).contains(&x) && (10..22).contains(&y)) || ((34..46).contains(&x) && (22..80).contains(&y)) { BLACK } else { WHITE });
    let res = trace(&t, &bw()).expect("T");
    assert_eq!(res.contour_count(), 1);
    let c = &res.paths[0].contours[0];
    assert_eq!(c.nodes.len(), 8, "{:?}", c.nodes);
    assert!(c.nodes.iter().all(is_corner));
    assert_eq!(bw_match(&t, &res), 100.0);
    // O: Ring mit Loch → zwei glatte Konturen in einem Pfad
    let o = supersample(IRect::from_xywh(0, 0, 100, 100), |x, y| {
        let r = (x - 50.0).hypot(y - 50.0);
        if (24.0..=40.0).contains(&r) { BLACK } else { WHITE }
    });
    let res = trace(&o, &bw()).expect("O");
    assert_eq!(res.paths.len(), 1);
    assert_eq!(res.contour_count(), 2, "außen und Loch");
    for c in &res.paths[0].contours {
        assert!(c.nodes.len() <= 8 && c.nodes.iter().all(|n| !is_corner(n)), "{} Knoten", c.nodes.len());
    }
    assert!(bw_match(&o, &res) > 99.0);
    // Loch bleibt frei (gerade/ungerade)
    let r = res.render();
    assert_eq!(r.data()[50 * 100 + 50][3], 0, "Mitte des O ist leer");
    assert_eq!(r.data()[50 * 100 + 18][3], 255, "Ring ist gefüllt");
}

#[test]
fn turd_size_removes_speckles() {
    let img = pixels(IRect::from_xywh(0, 0, 60, 60), |x, y| {
        let speck = [(5, 5), (50, 8), (8, 50)].iter().any(|&(sx, sy)| (sx..sx + 2).contains(&x) && (sy..sy + 2).contains(&y));
        if speck || ((20..40).contains(&x) && (20..40).contains(&y)) { BLACK } else { WHITE }
    });
    let mut o = bw();
    o.potrace.turd_size = 2;
    assert_eq!(trace(&img, &o).expect("a").contour_count(), 4, "Flecken mit 4 Pixeln bleiben");
    o.potrace.turd_size = 4;
    assert_eq!(trace(&img, &o).expect("b").contour_count(), 1, "Flecken bis 4 Pixel entfallen");
    // kleines Loch wird gefüllt
    let holed = pixels(IRect::from_xywh(0, 0, 40, 40), |x, y| if (10..30).contains(&x) && (10..30).contains(&y) && !((19..21).contains(&x) && (19..21).contains(&y)) { BLACK } else { WHITE });
    o.potrace.turd_size = 2;
    assert_eq!(trace(&holed, &o).expect("c").contour_count(), 2);
    o.potrace.turd_size = 4;
    assert_eq!(trace(&holed, &o).expect("d").contour_count(), 1);
}

#[test]
fn turn_policy_decides_diagonal_contacts() {
    // zwei 3×3-Blöcke, die sich nur an einer Ecke berühren
    let img = pixels(IRect::from_xywh(0, 0, 30, 30), |x, y| if ((8..11).contains(&x) && (8..11).contains(&y)) || ((11..14).contains(&x) && (11..14).contains(&y)) { BLACK } else { WHITE });
    let n = |p: TurnPolicy| {
        let mut o = bw();
        o.potrace.turd_size = 0;
        o.potrace.turn_policy = p;
        trace(&img, &o).expect("nachzeichnen").contour_count()
    };
    assert_eq!(n(TurnPolicy::Black), 1, "schwarz verbunden");
    assert_eq!(n(TurnPolicy::White), 2, "weiß verbunden");
    assert_eq!(n(TurnPolicy::Minority), 1, "Schwarz ist hier in der Minderheit");
    assert_eq!(n(TurnPolicy::Majority), 2);
    for p in TurnPolicy::ALL {
        assert_eq!(TurnPolicy::from_name(p.name()), Some(p));
    }
}

#[test]
fn alpha_max_controls_corners() {
    let img = supersample(IRect::from_xywh(0, 0, 120, 120), |x, y| if (x - 60.0).hypot(y - 60.0) <= 40.0 { BLACK } else { WHITE });
    let mut o = bw();
    o.potrace.alpha_max = 0.0;
    let poly = trace(&img, &o).expect("Polygon");
    assert!(poly.paths[0].contours[0].nodes.iter().all(is_corner), "alphaMax 0: nur Ecken");
    assert!(bw_match(&img, &poly) > 98.5);
    // Quadrat: mit alphaMax 4/3 keine Ecken mehr
    let sq = pixels(IRect::from_xywh(0, 0, 60, 60), |x, y| if (10..50).contains(&x) && (10..50).contains(&y) { BLACK } else { WHITE });
    o.potrace.alpha_max = ALPHA_MAX_LIMIT;
    let round = trace(&sq, &o).expect("rund");
    assert!(round.paths[0].contours[0].nodes.iter().all(|n| !is_corner(n)));
    o.potrace.alpha_max = 1.0;
    assert!(trace(&sq, &o).expect("eckig").paths[0].contours[0].nodes.iter().all(is_corner));
}

#[test]
fn color_layers_are_stacked_without_gaps() {
    let img = supersample(IRect::from_xywh(100, 50, 200, 160), |x, y| {
        if (x - 60.0).hypot(y - 70.0) <= 40.0 {
            BLUE
        } else if (110.0..180.0).contains(&x) && (30.0..130.0).contains(&y) {
            RED
        } else {
            WHITE
        }
    });
    let mut o = color();
    o.colors = 3;
    let res = trace(&img, &o).expect("nachzeichnen");
    assert_eq!(res.paths.len(), 3, "eine Ebene je Farbe");
    // unterste Ebene: größte Fläche (Weiß) über das ganze Bild, als Rechteck
    let bottom = &res.paths[0];
    assert!(bottom.color.r > 240 && bottom.color.g > 240 && bottom.color.b > 240, "{:?}", bottom.color);
    assert_eq!(bottom.contours.len(), 1);
    assert_eq!(bottom.contours[0].nodes.len(), 4);
    let near = |c: Rgba8, t: [u8; 4]| (0..3).all(|k| ([c.r, c.g, c.b][k] as i32 - t[k] as i32).abs() < 20);
    let cols: Vec<Rgba8> = res.paths.iter().map(|p| p.color).collect();
    assert!(cols.iter().any(|c| near(*c, BLUE)) && cols.iter().any(|c| near(*c, RED)), "{cols:?}");
    assert!(color_match(&img, &res) > 99.0, "{}", color_match(&img, &res));
    // Lage: Dokumentkoordinaten der Bitmap
    let n = &bottom.contours[0].nodes;
    assert!(n.iter().any(|q| q.pt == Point::new(100.0, 50.0)) && n.iter().any(|q| q.pt == Point::new(300.0, 210.0)));
}

#[test]
fn transparent_background_gives_no_paths() {
    let img = supersample(IRect::from_xywh(0, 0, 100, 100), |x, y| if (x - 50.0).hypot(y - 50.0) <= 30.0 { BLUE } else { CLEAR });
    let res = trace(&img, &color()).expect("nachzeichnen");
    assert!(!res.paths.is_empty());
    let r = res.render();
    assert_eq!(r.data()[0][3], 0, "Ecke bleibt transparent");
    assert!(color_match(&img, &res) > 99.0);
    // ganz transparent: keine Pfade, kein Fehler
    let empty = pixels(IRect::from_xywh(0, 0, 20, 20), |_, _| CLEAR);
    assert!(trace(&empty, &color()).expect("leer").paths.is_empty());
}

#[test]
fn repeatable_and_options_round_trip() {
    let img = supersample(IRect::from_xywh(0, 0, 120, 100), |x, y| if (x - 40.0).hypot(y - 50.0) <= 30.0 || ((70.0..110.0).contains(&x) && (20.0..80.0).contains(&y)) { BLACK } else { WHITE });
    let o = bw();
    let a = trace(&img, &o).expect("a");
    // Werte 50× hin und her: gleiches Ergebnis wie einmal mit dem Endwert
    let mut p = o.clone();
    for k in 0..50 {
        p.potrace.alpha_max = (k % 7) as f64 * 0.2;
        p.potrace.turd_size = k % 5;
        let _ = trace(&img, &p).expect("Zwischenschritt");
    }
    p.potrace = o.potrace.clone();
    assert_eq!(trace(&img, &p).expect("b"), a, "gleiche Optionen, gleiches Ergebnis");
    // JSON: ohne `engine` das bisherige Verfahren; Konturverfahren bleibt beim Speichern erhalten
    let mut v = serde_json::to_value(TraceOptions::default()).expect("JSON");
    v.as_object_mut().expect("Objekt").remove("engine");
    v.as_object_mut().expect("Objekt").remove("potrace");
    let back: TraceOptions = serde_json::from_value(v).expect("lesen");
    assert_eq!(back.engine, Engine::Clusters);
    assert_eq!(back.potrace, PotraceOptions::default());
    let mut q = bw();
    q.potrace = PotraceOptions { turd_size: 7, turn_policy: TurnPolicy::Right, alpha_max: 0.4, opt_curve: false, opt_tolerance: 0.6 };
    let s = serde_json::to_string(&q).expect("JSON");
    assert!(s.contains("\"engine\":\"potrace\"") && s.contains("\"turnPolicy\":\"right\"") && s.contains("\"turdSize\":7"), "{s}");
    assert_eq!(serde_json::from_str::<TraceOptions>(&s).expect("lesen"), q);
    // Bereiche
    q.potrace.alpha_max = 9.0;
    q.potrace.opt_tolerance = f64::NAN;
    q.potrace.turd_size = 1_000_000;
    let v = q.validated().potrace;
    assert_eq!((v.alpha_max, v.opt_tolerance, v.turd_size), (ALPHA_MAX_LIMIT, 0.2, 10_000));
    assert_eq!(Engine::from_name("potrace"), Some(Engine::Potrace));
    assert_eq!(Engine::from_name("clusters"), Some(Engine::Clusters));
}

#[test]
fn cancel_and_progress() {
    let img = supersample(IRect::from_xywh(0, 0, 200, 200), |x, y| if (x - 100.0).hypot(y - 100.0) <= 70.0 { BLUE } else { WHITE });
    let stop = AtomicBool::new(true);
    assert_eq!(trace_with(&img, &color(), &stop, &mut |_| {}), Err(TraceError::Cancelled));
    let mut seen = Vec::new();
    trace_with(&img, &color(), &AtomicBool::new(false), &mut |p| seen.push(p.fraction)).expect("nachzeichnen");
    assert!(seen.windows(2).all(|w| w[1] >= w[0]), "monoton: {seen:?}");
    assert_eq!(seen.last().copied(), Some(1.0));
}

/// Strichzeichnung: Linien in verschiedenen Winkeln, Kreisringe, eine Welle, Schrift-ähnliche Bögen.
fn line_drawing() -> RgbaImage {
    supersample(IRect::from_xywh(0, 0, 512, 512), |x, y| {
        let seg = |ax: f64, ay: f64, bx: f64, by: f64, w: f64| {
            let (dx, dy) = (bx - ax, by - ay);
            let t = (((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
            (x - ax - t * dx).hypot(y - ay - t * dy) <= w / 2.0
        };
        let ring = |cx: f64, cy: f64, r: f64, w: f64| ((x - cx).hypot(y - cy) - r).abs() <= w / 2.0;
        let wave = (x > 40.0 && x < 470.0) && (y - (420.0 + 30.0 * (x / 40.0).sin())).abs() <= 2.0;
        let ink = seg(30.0, 30.0, 480.0, 30.0, 4.0)
            || seg(30.0, 30.0, 30.0, 330.0, 4.0)
            || seg(60.0, 300.0, 250.0, 60.0, 3.0)
            || seg(260.0, 300.0, 480.0, 120.0, 5.0)
            || seg(300.0, 60.0, 470.0, 260.0, 2.5)
            || ring(150.0, 200.0, 70.0, 4.0)
            || ring(370.0, 250.0, 40.0, 3.0)
            || ring(370.0, 250.0, 15.0, 6.0)
            || wave;
        if ink { BLACK } else { WHITE }
    })
}

#[test]
fn compare_engines_on_line_drawing() {
    let img = line_drawing();
    let clusters = TraceOptions::from_preset(Preset::BlackWhite);
    let potrace = bw();
    let mut plain = bw();
    plain.potrace.opt_curve = false;
    let mut rows = Vec::new();
    for (name, o) in [("clusters", &clusters), ("potrace", &potrace), ("potrace ohne Kurvenoptimierung", &plain)] {
        let _ = trace(&img, o).expect("Aufwärmen");
        let t0 = Instant::now();
        let r = trace(&img, o).expect("nachzeichnen");
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        let m = bw_match(&img, &r);
        println!("Strichzeichnung 512×512 {name}: {ms:.1} ms, {} Pfade, {} Konturen, {} Knoten, Übereinstimmung {m:.2} %", r.paths.len(), r.contour_count(), r.node_count());
        rows.push((ms, r.node_count(), m));
        assert!(m > 98.0, "{name}: {m}");
    }
    assert!(rows[1].1 < rows[0].1, "Konturen-Verfahren mit weniger Knoten: {} gegen {}", rows[1].1, rows[0].1);
}

/// CT-1a (FIX-C): Logo-Vorgabe mit „Glatte Kurven“ an einem blauen Quadrat mit gelbem,
/// kantengeglättetem Kreis: die Mischtöne am Kreisrand bilden keine eigenen Farbebenen – zwei
/// Farben, wenige Knoten (vorher 9 Farben/477 Punkte), Kreis glatt auf dem Radius.
#[test]
fn logo_preset_snaps_antialias_colors() {
    const YELLOW: [u8; 4] = [250, 220, 30, 255];
    let img = supersample(IRect::from_xywh(0, 0, 100, 100), |x, y| if (x - 50.0).hypot(y - 50.0) <= 30.0 { YELLOW } else { BLUE });
    let o = TraceOptions { engine: Engine::Potrace, ..TraceOptions::from_preset(Preset::Logo) };
    let res = trace(&img, &o).expect("nachzeichnen");
    assert_eq!(res.paths.len(), 2, "zwei Farben: {:?}", res.paths.iter().map(|p| p.color).collect::<Vec<_>>());
    let nodes: usize = res.paths.iter().flat_map(|p| &p.contours).map(|c| c.nodes.len()).sum();
    assert!(nodes <= 16, "{nodes} Knoten");
    let circle = res.paths.iter().find(|p| p.color.r > 200).expect("gelbe Ebene");
    for p in sample(&circle.contours[0], 16) {
        let r = ((p.x - 50.0) as f64).hypot((p.y - 50.0) as f64);
        assert!((r - 30.0).abs() < 1.0, "Radius {r}");
    }
    assert!(color_match(&img, &res) > 97.0, "Übereinstimmung (Kantenpixel zählen als Abweichung) {}", color_match(&img, &res));
}