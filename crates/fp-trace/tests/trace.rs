//! Prüfungen des Nachzeichnens an synthetischen Bildern: Pfadzahl, Lage in Dokumentkoordinaten,
//! Pixelübereinstimmung des gerasterten Ergebnisses mit dem Original, Abbruch, Fortschritt,
//! Herunterskalieren, Vorgaben. Laufzeiten für 512 × 512 mit `--nocapture`.

use fp_color::Rgba8;
use fp_geom::IRect;
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
                // vormultipliziert mitteln
                let a = c[3] as u32;
                for k in 0..3 {
                    acc[k] += c[k] as u32 * a;
                }
                acc[3] += a;
            }
        }
        let a = acc[3];
        *px = match (
            acc[0].checked_div(a),
            acc[1].checked_div(a),
            acc[2].checked_div(a),
        ) {
            (Some(r), Some(g), Some(b)) => [r as u8, g as u8, b as u8, (a / 16) as u8],
            _ => [0, 0, 0, 0],
        };
    }
    img
}

const WHITE: [u8; 4] = [255, 255, 255, 255];
const BLUE: [u8; 4] = [20, 60, 200, 255];
const RED: [u8; 4] = [220, 30, 40, 255];
const CLEAR: [u8; 4] = [0, 0, 0, 0];
const MAGENTA: [u8; 4] = [255, 0, 255, 255];

/// Zweifarbiges Logo: blauer Kreis und rotes Rechteck auf `bg`, Ursprung (100, 50).
fn logo(bg: [u8; 4]) -> RgbaImage {
    supersample(IRect::from_xywh(100, 50, 512, 512), |x, y| {
        if (x - 180.0).hypot(y - 200.0) <= 110.0 {
            BLUE
        } else if (300.0..470.0).contains(&x) && (250.0..450.0).contains(&y) {
            RED
        } else {
            bg
        }
    })
}

/// Schrift-ähnliche Schwarzweiß-Form: „F“, „O“ (Ring mit Loch), schräger Balken, Punkte.
fn letters() -> RgbaImage {
    supersample(IRect::from_xywh(0, 0, 512, 512), |x, y| {
        let black = [0, 0, 0, 255];
        let f = ((60.0..95.0).contains(&x) && (100.0..400.0).contains(&y))
            || ((60.0..220.0).contains(&x) && (100.0..135.0).contains(&y))
            || ((60.0..190.0).contains(&x) && (230.0..262.0).contains(&y));
        let d = (x - 340.0).hypot(y - 250.0);
        let o = (90.0..140.0).contains(&d);
        // schräger Balken (Abstand zur Geraden)
        let bar = ((x - 0.5 * y - 200.0).abs() < 14.0) && (420.0..500.0).contains(&y);
        let dots = (x - 120.0).hypot(y - 460.0) < 12.0 || (x - 160.0).hypot(y - 460.0) < 12.0;
        if f || o || bar || dots { black } else { WHITE }
    })
}

/// Horizontaler Farbverlauf rot → blau mit leichtem vertikalem Helligkeitsverlauf.
fn gradient() -> RgbaImage {
    supersample(IRect::from_xywh(0, 0, 512, 512), |x, y| {
        let t = x / 512.0;
        let l = 1.0 - 0.3 * y / 512.0;
        [
            ((255.0 * (1.0 - t)) * l) as u8,
            (40.0 * l) as u8,
            ((255.0 * t) * l) as u8,
            255,
        ]
    })
}

/// Pixel über `bg` verrechnet.
fn over(p: [u8; 4], bg: [u8; 4]) -> [u8; 3] {
    let a = p[3] as u32;
    let f = |c: u8, b: u8| ((c as u32 * a + b as u32 * (255 - a)) / 255) as u8;
    [f(p[0], bg[0]), f(p[1], bg[1]), f(p[2], bg[2])]
}

/// Anteil der Pixel, deren Farbe (beide über `bg` verrechnet) je Kanal höchstens `tol` abweicht.
/// Transparenz wird über die Wahl von `bg` geprüft (z. B. Magenta).
fn match_ratio(orig: &RgbaImage, traced: &RgbaImage, bg: [u8; 4], tol: i32) -> f64 {
    assert_eq!(orig.bounds(), traced.bounds());
    let mut ok = 0usize;
    for (a, b) in orig.data().iter().zip(traced.data()) {
        let (ca, cb) = (over(*a, bg), over(*b, bg));
        if (0..3).all(|k| (ca[k] as i32 - cb[k] as i32).abs() <= tol) {
            ok += 1;
        }
    }
    ok as f64 / orig.data().len() as f64
}

/// Mittlere absolute Abweichung je Kanal (beide über Weiß verrechnet).
fn mean_error(orig: &RgbaImage, traced: &RgbaImage) -> f64 {
    let mut s = 0u64;
    for (a, b) in orig.data().iter().zip(traced.data()) {
        let (a, b) = (over(*a, WHITE), over(*b, WHITE));
        for k in 0..3 {
            s += (a[k] as i32 - b[k] as i32).unsigned_abs() as u64;
        }
    }
    s as f64 / (orig.data().len() * 3) as f64
}

fn timed(name: &str, img: &RgbaImage, o: &TraceOptions) -> TraceResult {
    let t = Instant::now();
    let r = trace(img, o).expect("nachzeichnen");
    eprintln!(
        "{name}: {}×{} in {:.0} ms – {} Pfade, {} Konturen, {} Knoten",
        img.width(),
        img.height(),
        t.elapsed().as_secs_f64() * 1000.0,
        r.paths.len(),
        r.contour_count(),
        r.node_count()
    );
    r
}

fn nodes_inside(r: &TraceResult) {
    let b = r.bounds;
    for p in &r.paths {
        for c in &p.contours {
            assert!(c.closed);
            for n in &c.nodes {
                // Anker und Henkel bleiben im Bild (auf die Bitmap begrenzt)
                for q in [n.pt, n.ctrl_in, n.ctrl_out] {
                    let m = 0.01f32;
                    assert!(
                        q.x >= b.left as f32 - m
                            && q.x <= b.right as f32 + m
                            && q.y >= b.top as f32 - m
                            && q.y <= b.bottom as f32 + m,
                        "Knoten {q:?} außerhalb {b:?}"
                    );
                }
            }
        }
    }
}

fn near(c: Rgba8, t: [u8; 4], tol: i32) -> bool {
    (c.r as i32 - t[0] as i32).abs() <= tol
        && (c.g as i32 - t[1] as i32).abs() <= tol
        && (c.b as i32 - t[2] as i32).abs() <= tol
}

#[test]
fn logo_two_colors_on_white() {
    let img = logo(WHITE);
    let r = timed(
        "Logo (Vorgabe Logo)",
        &img,
        &TraceOptions::from_preset(Preset::Logo),
    );
    assert_eq!(r.bounds, img.bounds());
    assert!(!r.downscaled());
    // Hintergrund + Kreis + Rechteck, höchstens ein paar Kantenreste
    assert!((3..=8).contains(&r.paths.len()), "Pfade: {}", r.paths.len());
    assert!(
        r.paths.iter().any(|p| near(p.color, BLUE, 12)),
        "blauer Pfad fehlt"
    );
    assert!(
        r.paths.iter().any(|p| near(p.color, RED, 12)),
        "roter Pfad fehlt"
    );
    // Der Hintergrund (größte Fläche) liegt zuunterst.
    assert!(near(r.paths[0].color, WHITE, 8));
    // Splines: Kreis hat Henkel
    assert!(
        r.paths
            .iter()
            .flat_map(|p| &p.contours)
            .flat_map(|c| &c.nodes)
            .any(|n| !n.is_corner())
    );
    nodes_inside(&r);
    let m = match_ratio(&img, &r.render(), WHITE, 40);
    eprintln!("Logo: Übereinstimmung {:.2} %", m * 100.0);
    assert!(m >= 0.97, "Übereinstimmung {m}");
}

#[test]
fn logo_on_transparent_ignores_background() {
    let img = logo(CLEAR);
    let r = timed(
        "Logo transparent",
        &img,
        &TraceOptions::from_preset(Preset::Logo),
    );
    assert!((2..=6).contains(&r.paths.len()), "Pfade: {}", r.paths.len());
    assert!(
        r.paths
            .iter()
            .all(|p| !near(p.color, WHITE, 30) || p.pixels < 200),
        "Hintergrundpfad trotz Transparenz"
    );
    let out = r.render();
    let m = match_ratio(&img, &out, MAGENTA, 40);
    eprintln!("Logo transparent: Übereinstimmung {:.2} %", m * 100.0);
    assert!(m >= 0.97, "Übereinstimmung {m}");
    // Ecke bleibt transparent
    assert_eq!(out.get(101, 51).map(|p| p[3]), Some(0));
}

#[test]
fn logo_cutout_and_polygon() {
    let img = logo(WHITE);
    let mut o = TraceOptions::from_preset(Preset::Logo);
    o.hierarchical = Hierarchy::Cutout;
    let r = timed("Logo ausschneiden", &img, &o);
    let m = match_ratio(&img, &r.render(), WHITE, 40);
    eprintln!("Logo ausschneiden: Übereinstimmung {:.2} %", m * 100.0);
    assert!(m >= 0.97, "Übereinstimmung {m}");
    // Ausschneiden: der Hintergrund hat Löcher (mehr als eine Kontur)
    let bg = r
        .paths
        .iter()
        .find(|p| near(p.color, WHITE, 8))
        .expect("Hintergrund");
    assert!(
        bg.contours.len() >= 3,
        "Hintergrund ohne Löcher: {}",
        bg.contours.len()
    );

    o.hierarchical = Hierarchy::Stacked;
    o.mode = CurveMode::Polygon;
    let r = timed("Logo Polygon", &img, &o);
    assert!(
        r.paths
            .iter()
            .flat_map(|p| &p.contours)
            .flat_map(|c| &c.nodes)
            .all(|n| n.is_corner())
    );
    let m = match_ratio(&img, &r.render(), WHITE, 40);
    eprintln!("Logo Polygon: Übereinstimmung {:.2} %", m * 100.0);
    assert!(m >= 0.97, "Übereinstimmung {m}");
}

#[test]
fn black_white_letters() {
    let img = letters();
    let r = timed(
        "Schrift (Vorgabe Schwarzweiß)",
        &img,
        &TraceOptions::from_preset(Preset::BlackWhite),
    );
    // F, O, Balken, 2 Punkte
    assert_eq!(r.paths.len(), 5, "Pfade: {}", r.paths.len());
    assert!(r.paths.iter().all(|p| p.color == Rgba8::new(0, 0, 0, 255)));
    // Das „O“ hat ein Loch.
    assert!(
        r.paths.iter().any(|p| p.contours.len() == 2),
        "Ring ohne Loch"
    );
    nodes_inside(&r);
    let m = match_ratio(&img, &r.render(), WHITE, 40);
    eprintln!("Schrift: Übereinstimmung {:.2} %", m * 100.0);
    assert!(m >= 0.97, "Übereinstimmung {m}");
    // Flecken unter 4×4 px fallen weg
    let mut o = TraceOptions::from_preset(Preset::BlackWhite);
    o.filter_speckle = 30;
    let r2 = trace(&img, &o).expect("nachzeichnen");
    assert_eq!(r2.paths.len(), 3, "große Fleckengrenze entfernt die Punkte");
}

#[test]
fn gradient_presets() {
    let img = gradient();
    for p in [Preset::Illustration, Preset::Photo] {
        let r = timed(
            &format!("Verlauf ({})", p.name()),
            &img,
            &TraceOptions::from_preset(p),
        );
        assert!(
            (4..=3000).contains(&r.paths.len()),
            "{p:?}: {} Pfade",
            r.paths.len()
        );
        let out = r.render();
        let e = mean_error(&img, &out);
        let m = match_ratio(&img, &out, WHITE, 40);
        eprintln!(
            "Verlauf {}: mittlere Abweichung {e:.1}, Übereinstimmung (±40) {:.2} %",
            p.name(),
            m * 100.0
        );
        assert!(e <= 20.0, "{p:?}: mittlere Abweichung {e}");
        assert!(m >= 0.9, "{p:?}: Übereinstimmung {m}");
    }
}

#[test]
fn all_presets_on_logo_run() {
    let img = logo(WHITE);
    for p in Preset::ALL {
        let r = timed(
            &format!("Logo mit Vorgabe {}", p.name()),
            &img,
            &TraceOptions::from_preset(p),
        );
        assert!(!r.paths.is_empty(), "{p:?}");
        nodes_inside(&r);
    }
}

#[test]
fn cancel_and_progress() {
    let img = gradient();
    let cancel = AtomicBool::new(true);
    assert_eq!(
        trace_with(&img, &TraceOptions::default(), &cancel, &mut |_| {}),
        Err(TraceError::Cancelled)
    );

    let cancel = AtomicBool::new(false);
    let mut seen: Vec<Progress> = Vec::new();
    let r = trace_with(
        &img,
        &TraceOptions::from_preset(Preset::Illustration),
        &cancel,
        &mut |p| seen.push(p),
    );
    assert!(r.is_ok());
    assert!(seen.len() >= 3, "{} Fortschrittsmeldungen", seen.len());
    assert!(
        seen.windows(2).all(|w| w[1].fraction >= w[0].fraction),
        "Fortschritt nicht monoton"
    );
    assert_eq!(
        seen.last().map(|p| (p.stage, p.fraction)),
        Some((Stage::Done, 1.0))
    );

    // Abbruch mitten im Lauf über den Rückruf
    let cancel = AtomicBool::new(false);
    let r = trace_with(
        &img,
        &TraceOptions::from_preset(Preset::Photo),
        &cancel,
        &mut |p| {
            if p.stage == Stage::Cluster {
                cancel.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        },
    );
    assert_eq!(r, Err(TraceError::Cancelled));
}

#[test]
fn large_images_are_downscaled() {
    // 2400 × 1600, Kreis in der Mitte
    let b = IRect::from_xywh(-50, 20, 2400, 1600);
    let mut img = RgbaImage::filled(b, Rgba8::new(255, 255, 255, 255)).expect("Bild");
    let w = img.width();
    for (i, px) in img.data_mut().iter_mut().enumerate() {
        let (x, y) = ((i % w) as f64, (i / w) as f64);
        if (x - 1200.0).hypot(y - 800.0) < 500.0 {
            *px = BLUE;
        }
    }
    let mut o = TraceOptions::from_preset(Preset::Logo);
    o.max_pixels = 1_000_000;
    let r = timed("Groß (auf 1 MP)", &img, &o);
    assert!(r.downscaled());
    assert!(r.traced_width as u64 * r.traced_height as u64 <= 1_000_000);
    assert_eq!(r.bounds, b);
    nodes_inside(&r);
    let m = match_ratio(&img, &r.render(), WHITE, 40);
    eprintln!("Groß: Übereinstimmung {:.2} %", m * 100.0);
    assert!(m >= 0.97, "Übereinstimmung {m}");

    o.downscale = false;
    assert_eq!(
        trace(&img, &o),
        Err(TraceError::TooLarge {
            width: 2400,
            height: 1600
        })
    );
}

#[test]
fn edge_cases() {
    assert_eq!(
        trace(&RgbaImage::empty_at(5, 5), &TraceOptions::default()),
        Err(TraceError::Empty)
    );
    // ganz transparent → keine Pfade
    let img =
        RgbaImage::filled(IRect::from_xywh(0, 0, 40, 30), Rgba8::new(0, 0, 0, 0)).expect("Bild");
    assert!(
        trace(&img, &TraceOptions::default())
            .expect("ok")
            .paths
            .is_empty()
    );
    // einfarbig → genau eine Fläche über das ganze Bild
    let img = RgbaImage::filled(
        IRect::from_xywh(10, 10, 40, 30),
        Rgba8::new(10, 200, 10, 255),
    )
    .expect("Bild");
    let r = trace(&img, &TraceOptions::default()).expect("ok");
    assert_eq!(r.paths.len(), 1);
    assert_eq!(match_ratio(&img, &r.render(), WHITE, 0), 1.0);
    // 1 × 1
    let img =
        RgbaImage::filled(IRect::from_xywh(0, 0, 1, 1), Rgba8::new(10, 20, 30, 255)).expect("Bild");
    assert!(trace(&img, &TraceOptions::default()).is_ok());
    // unsinnige Werte werden begrenzt
    let o = TraceOptions {
        corner_threshold: f64::NAN,
        length_threshold: 1000.0,
        color_precision: 0,
        colors: 1,
        ..TraceOptions::default()
    };
    let v = o.validated();
    assert_eq!(
        (
            v.corner_threshold,
            v.length_threshold,
            v.color_precision,
            v.colors
        ),
        (60.0, 10.0, 1, 0)
    );
    assert!(trace(&logo(WHITE), &o).is_ok());
}

#[test]
fn preset_names() {
    for p in Preset::ALL {
        assert_eq!(Preset::from_name(p.name()), Some(p));
    }
    assert_eq!(Preset::from_name("foto"), Some(Preset::Photo));
    assert_eq!(Preset::from_name("schwarzweiss"), Some(Preset::BlackWhite));
    assert_eq!(Preset::from_name("x"), None);
    assert_eq!(
        TraceOptions::default(),
        TraceOptions::from_preset(Preset::Logo)
    );
}

/// Drei Rasterbilder aus dem Korpus (nur lesen): Pixelübereinstimmung je Vorgabe ausgeben.
/// `cargo test -p fp-trace --test trace -- --ignored --nocapture`
#[test]
#[ignore]
fn corpus_bitmaps() {
    let root = std::env::var("FP_CORPUS_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join("..")
                .join("korpus")
        });
    let files = [
        ("Configuration/Auto Shapes/Cog.gif", Preset::Logo),
        ("Configuration/Auto Shape Tools/Arrow.png", Preset::Logo),
        ("Configuration/Patterns/Blue Wave.jpg", Preset::Photo),
    ];
    let mut done = 0;
    for (rel, preset) in files {
        let path = root.join(rel);
        let Ok(dynimg) = image::open(&path) else {
            eprintln!("übersprungen (nicht lesbar): {}", path.display());
            continue;
        };
        let rgba = dynimg.to_rgba8();
        let (w, h) = rgba.dimensions();
        let img =
            RgbaImage::from_rgba_bytes(IRect::from_xywh(0, 0, w as i32, h as i32), rgba.as_raw())
                .expect("Bild");
        for p in [preset, Preset::Illustration] {
            let r = timed(
                &format!("{rel} ({})", p.name()),
                &img,
                &TraceOptions::from_preset(p),
            );
            let out = r.render();
            let m = match_ratio(&img, &out, WHITE, 40);
            eprintln!(
                "  → Übereinstimmung (±40) {:.2} %, mittlere Abweichung {:.1}",
                m * 100.0,
                mean_error(&img, &out)
            );
        }
        done += 1;
    }
    assert!(
        done > 0,
        "kein Korpusbild gefunden unter {}",
        root.display()
    );
}
