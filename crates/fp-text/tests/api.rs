//! API-Tests mit Systemschriften: Schriftauflösung, Rahmenarten, Editor-Abfragen, HarfBuzz,
//! Rasterung. Ohne Myriad Pro/Arial werden die Tests übersprungen.

mod common;

use common::*;
use fp_color::Rgba8;
use fp_geom::{BezierContour, BezierNode, Point, Rect};
use fp_text::*;

fn attrs(font: &str, size: f64) -> CharAttrs {
    CharAttrs {
        font: font.into(),
        size,
        ..CharAttrs::default()
    }
}

macro_rules! need_fonts {
    () => {
        if !have_corpus_fonts() {
            eprintln!("übersprungen: Korpusschriften fehlen");
            return;
        }
    };
}

#[test]
fn font_resolution_and_fallback() {
    need_fonts!();
    let db = fonts();
    let r = db.resolve("ArialMT", false, false).unwrap();
    assert!(r.exact && !r.substituted);
    assert_eq!(r.font.ps_name(), "ArialMT");
    // Familienname mit Stil
    let b = db.resolve("Arial", true, false).unwrap();
    assert_eq!(b.font.ps_name(), "Arial-BoldMT");
    // PostScript-Name ohne Datei → Familie + Stil aus dem Namen
    let m = db.resolve("MyriadPro-Bold", false, false).unwrap();
    assert_eq!(m.font.ps_name(), "MyriadPro-Bold");
    // unbekannt → Rückfall Myriad Pro (wie ATE)
    let x = db.resolve("LucidaGrande", false, false).unwrap();
    assert!(x.substituted);
    assert_eq!(x.font.ps_name(), "MyriadPro-Regular");
    assert!(db.families().iter().any(|f| f == "Arial"));
    // Myriad Pro: asc aus BASE (ideo −163) bzw. gemessen 0,837006, desc 0,25
    let f = m.font;
    assert_eq!(f.ideo_baseline(), Some(-163));
    assert!((f.ascent_rule() - 0.837).abs() < 1e-12);
    assert_eq!(f.descent(), 0.25);
}

#[test]
fn missing_font_falls_back_without_panic() {
    need_fonts!();
    let block = TextBlock::point(
        10.0,
        50.0,
        vec![TextRun::new("Ersatz", attrs("GibtEsNicht-Regular", 20.0))],
    );
    let lay = layout(&block, fonts(), &LayoutOptions::default());
    assert_eq!(lay.missing_fonts, vec!["GibtEsNicht-Regular".to_string()]);
    assert_eq!(lay.glyphs.len(), 6);
    // ohne jede Schrift: leeres Layout
    let empty = layout(&block, &FontDb::new(), &LayoutOptions::default());
    assert!(empty.glyphs.is_empty());
    assert_eq!(empty.char_count, 6);
}

#[test]
fn point_text_alignment_and_tabs() {
    need_fonts!();
    let a = CharAttrs {
        alignment: Alignment::Center,
        ..attrs("MyriadPro-Regular", 20.0)
    };
    let lay = layout(
        &TextBlock::point(200.0, 100.0, vec![TextRun::new("Mitte", a)]),
        fonts(),
        &LayoutOptions::default(),
    );
    let l = &lay.lines[0];
    assert!((l.start + l.align_width / 2.0 - 200.0).abs() < 1e-9);
    // Tabulatoren: Raster 36 px, „aaaaaaa“ (70 px) springt noch auf 72 (T1)
    let t = layout(
        &TextBlock::point(
            20.0,
            50.0,
            vec![TextRun::new("aaaaaaa\tb", attrs("MyriadPro-Regular", 20.0))],
        ),
        fonts(),
        &LayoutOptions {
            ..LayoutOptions::default()
        },
    );
    let b = t.glyphs.iter().find(|g| g.ch == 'b').unwrap();
    assert_eq!(b.x, 20.0 + 72.0);
}

#[test]
fn fixed_frame_wraps_and_measures_height() {
    need_fonts!();
    // t2_frames: 200 × 100, Myriad 20 px → 4 Zeilen, Satzhöhe 80 px
    let text =
        "Dies ist ein laengerer Text, der nicht in eine einzige Zeile passt und umbrechen muss.";
    let block = TextBlock::fixed(
        Rect::new(20.0, 80.0, 220.0, 180.0),
        vec![TextRun::new(text, attrs("MyriadPro-Regular", 20.0))],
    );
    let lay = layout(&block, fonts(), &LayoutOptions::default());
    assert_eq!(lay.lines.len(), 4);
    assert_eq!(lay.frame, Rect::new(20.0, 80.0, 220.0, 160.0));
    for l in &lay.lines {
        assert!(l.start + l.align_width <= 220.0 + 1e-9);
    }
    // Blocksatz: innere Zeilen füllen die Breite
    let mut jb = block.clone();
    jb.runs[0].attrs.alignment = Alignment::Justify;
    let lj = layout(&jb, fonts(), &LayoutOptions::default());
    let first = &lj.lines[0];
    let last_g = &lj.glyphs[first.glyphs.end - 2];
    assert!(
        (last_g.x + last_g.advance - 220.0).abs() < 1.0,
        "Blocksatz endet nicht am Rand"
    );
}

#[test]
fn editor_queries() {
    need_fonts!();
    let block = TextBlock::point(
        10.0,
        40.0,
        vec![TextRun::new("ab\rcde", attrs("ArialMT", 20.0))],
    );
    let lay = layout(&block, fonts(), &LayoutOptions::default());
    assert_eq!(lay.lines.len(), 2);
    assert_eq!(lay.lines[0].chars, 0..3);
    assert_eq!(lay.lines[1].chars, 3..6);
    // Caret vor „d“ und Treffer daneben
    let c = lay.caret(4).unwrap();
    assert_eq!(c.line, 1);
    let hit = lay.hit_test(c.top[0] + 1.0, lay.lines[1].baseline - 5.0);
    assert_eq!(hit.char_index, 4);
    // Klick rechts neben Zeile 1 landet vor dem Absatzende
    assert_eq!(lay.hit_test(500.0, lay.lines[0].baseline).char_index, 2);
    assert_eq!(
        lay.move_vertical(1, 1),
        lay.hit_test(lay.caret(1).unwrap().top[0], lay.lines[1].baseline)
            .char_index
    );
    let sel = lay.selection_rects(1, 5);
    assert_eq!(sel.len(), 2);
    assert_eq!(lay.line_range(4), Some(3..6));
}

#[test]
fn harfbuzz_ligatures_and_rtl() {
    need_fonts!();
    let opts = LayoutOptions {
        shaping: Shaping::Harfbuzz,
        ..LayoutOptions::default()
    };
    let block = TextBlock::point(
        0.0,
        50.0,
        vec![TextRun::new("fit", attrs("MyriadPro-Regular", 30.0))],
    );
    let plain = layout(&block, fonts(), &LayoutOptions::default());
    let hb = layout(&block, fonts(), &opts);
    assert_eq!(plain.glyphs.len(), 3);
    assert!(hb.glyphs.len() < 3, "Myriad Pro hat die fi-Ligatur");
    assert_eq!(hb.glyphs[0].char_len, 2);
    assert!(hb.caret(1).is_some());
    // Hebräisch wird umgekehrt gesetzt (erstes Zeichen rechts)
    let heb = TextBlock::point(
        0.0,
        50.0,
        vec![TextRun::new(
            "\u{05D0}\u{05D1}\u{05D2}",
            attrs("ArialMT", 30.0),
        )],
    );
    let lh = layout(&heb, fonts(), &LayoutOptions::default());
    assert!(lh.glyphs[0].x > lh.glyphs[2].x);
    let lhb = layout(&heb, fonts(), &opts);
    assert_eq!(lhb.glyphs.len(), 3);
}

#[test]
fn vertical_and_path_text() {
    need_fonts!();
    // T5: vertikal, Rahmenbreite = Schriftgröße, auf dem Anker zentriert
    let mut v = TextBlock::point(
        100.0,
        221.0,
        vec![TextRun::new("ABC 123", attrs("MyriadPro-Regular", 24.0))],
    );
    v.orientation = Orientation::VerticalRtl;
    let lv = layout(&v, fonts(), &LayoutOptions::default());
    assert_eq!(lv.frame, Rect::new(88.0, 221.0, 112.0, 304.0));
    // T3: Text an einer Linie (Überlauf: nur sichtbare Glyphen im Rahmen)
    let line = BezierContour::new(
        vec![
            BezierNode::corner(Point::new(470.0, 520.0)),
            BezierNode::corner(Point::new(570.0, 520.0)),
        ],
        false,
    );
    let block = TextBlock {
        kind: FrameKind::OnPath,
        runs: vec![TextRun::new(
            "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
            attrs("ArialMT", 20.0),
        )],
        path: Some(TextPath {
            contours: vec![line],
            ..TextPath::default()
        }),
        ..TextBlock::default()
    };
    let lp = layout(&block, fonts(), &LayoutOptions::default());
    assert_eq!(lp.frame, Rect::new(469.0, 502.0, 566.0, 527.0));
    assert_eq!(lp.visible_glyphs().count(), 7);
}

#[test]
fn render_colors_and_contours() {
    need_fonts!();
    let red = CharAttrs {
        color: Rgba8::new(255, 0, 0, 255),
        underline: true,
        ..attrs("ArialMT", 30.0)
    };
    let blue = CharAttrs {
        color: Rgba8::new(0, 0, 255, 255),
        bold: true,
        italic: true,
        ..attrs("ArialMT", 30.0)
    };
    let block = TextBlock::point(
        5.0,
        40.0,
        vec![TextRun::new("Ro", red), TextRun::new("Bl", blue)],
    );
    let lay = layout(&block, fonts(), &LayoutOptions::default());
    assert_eq!(lay.decorations.len(), 1);
    let img = render::render(&lay, None);
    assert!(!img.is_empty());
    let (mut reds, mut blues) = (0, 0);
    for p in img.data() {
        if p[3] == 255 {
            reds += (p[0] == 255 && p[2] == 0) as usize;
            blues += (p[2] == 255 && p[0] == 0) as usize;
        }
    }
    assert!(reds > 20 && blues > 20);
    let cov = render::coverage(&lay, None);
    assert_eq!(cov.bounds(), img.bounds());
    let contours = render::glyph_contours(&lay);
    assert_eq!(contours.len(), 5); // 4 Glyphen + Unterstreichung
    // ohne Kantenglättung nur 0/255
    let mut hard = block.clone();
    hard.antialias.enabled = false;
    let lh = layout(&hard, fonts(), &LayoutOptions::default());
    assert!(
        render::coverage(&lh, None)
            .data()
            .iter()
            .all(|&v| v == 0 || v == 255)
    );
}

#[test]
fn odd_inputs_do_not_panic() {
    let dbs = [FontDb::new(), FontDb::system()];
    let texts = [
        "",
        "\r\r",
        "\t\t\u{3}x",
        "\u{FFFD}\u{0}\u{10FFFF}",
        "a\u{05D0}b\u{0627}c",
        "ﬁ ﬂ 🙂",
    ];
    let sizes = [f64::NAN, 0.0, -12.0, 1e-9, 1e12, 20.0];
    for db in &dbs {
        for (k, t) in texts.iter().enumerate() {
            for &s in &sizes {
                let a = CharAttrs {
                    size: s,
                    tracking: if k % 2 == 0 { 1e9 } else { -3.8 },
                    leading: f64::INFINITY,
                    h_scale: if k == 3 { 0.0 } else { 1.0 },
                    bold: k % 2 == 1,
                    italic: k == 4,
                    underline: true,
                    alignment: Alignment::from_jsti(k as i64),
                    ..attrs("ArialMT", s)
                };
                for kind in [
                    FrameKind::Point,
                    FrameKind::FixedWidth,
                    FrameKind::OnPath,
                    FrameKind::InPath,
                ] {
                    let mut b = TextBlock::point(0.0, 0.0, vec![TextRun::new(*t, a.clone())]);
                    b.kind = kind;
                    b.bounds = Rect::new(0.0, 0.0, if k == 2 { -5.0 } else { 50.0 }, 50.0);
                    b.orientation = Orientation::from_orii(k as i64 % 4);
                    b.story_kerning = Some(vec![StoryKernRun {
                        len: 1_000_000,
                        kind: Some(KerningKind::Off),
                    }]);
                    if kind != FrameKind::Point {
                        b.path = Some(TextPath {
                            contours: vec![BezierContour::new(
                                vec![BezierNode::corner(Point::new(0.0, 0.0))],
                                true,
                            )],
                            ..TextPath::default()
                        });
                    }
                    for shaping in [Shaping::Original, Shaping::Harfbuzz] {
                        let lay = layout(
                            &b,
                            db,
                            &LayoutOptions {
                                shaping,
                                ..LayoutOptions::default()
                            },
                        );
                        let _ = render::render(&lay, None);
                        let _ = render::glyph_contours(&lay);
                        let _ = lay.hit_test(3.0, 4.0);
                        let _ = lay.selection_rects(0, 100);
                        let _ = lay.caret(lay.char_count);
                        let _ = lay.move_vertical(0, 1);
                    }
                }
            }
        }
    }
}

#[test]
fn system_index_is_fast_enough() {
    let t = std::time::Instant::now();
    let db = FontDb::system();
    let ms = t.elapsed().as_millis();
    println!("Schriftindex: {} Einträge in {ms} ms", db.len());
    assert!(ms < 20_000);
}
