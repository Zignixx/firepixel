//! S-6: Prüffälle Vereinfachen und Pfad einlassen aus dem Original (re\33 §3, Knotenlisten
//! `re\_testdateien\messungen\2026-10-16-m6\daten\p3_res.txt`, `p3_inset.txt`).

use fp_geom::Vec2;
use fp_geom::offset::{JoinKind, OffsetOptions};
use fp_vector::pathops::offset::inset_path_refit;
use fp_vector::pathops::*;

fn v(x: f64, y: f64) -> Vec2 {
    Vec2::new(x, y)
}

fn n(a: (f64, f64), i: (f64, f64), o: (f64, f64)) -> Node {
    Node::new(v(a.0, a.1), v(i.0, i.1), v(o.0, o.1))
}

/// P1 = M 200,200 C 240,160 260,160 290,200 C 320,240 340,240 370,200 C 380,180 400,200 420,240.
fn p1() -> Contour {
    Contour::new(
        vec![
            n((200.0, 200.0), (200.0, 200.0), (240.0, 160.0)),
            n((290.0, 200.0), (260.0, 160.0), (320.0, 240.0)),
            n((370.0, 200.0), (340.0, 240.0), (380.0, 180.0)),
            n((420.0, 240.0), (400.0, 200.0), (420.0, 240.0)),
        ],
        false,
    )
}

/// E1 = Ellipse 50,170–150,240 (Knoten wie im Original ausgelesen).
fn e1() -> Contour {
    Contour::new(
        vec![
            n((50.0, 205.0), (50.0, 224.3302), (50.0, 185.6698)),
            n((100.0, 170.0), (72.3854, 170.0), (127.6146, 170.0)),
            n((150.0, 205.0), (150.0, 185.6698), (150.0, 224.3302)),
            n((100.0, 240.0), (127.6146, 240.0), (72.3854, 240.0)),
        ],
        true,
    )
}

fn dump(name: &str, cs: &[Contour]) -> String {
    let mut s = format!("{name}: {} Kontur(en)", cs.len());
    for c in cs {
        s += &format!(
            "\n  {} n={}",
            if c.closed { "z" } else { "o" },
            c.nodes.len()
        );
        for k in &c.nodes {
            s += &format!(
                " {:.4}/{:.4}({:.4}/{:.4}|{:.4}/{:.4})",
                k.anchor.x, k.anchor.y, k.ctrl_in.x, k.ctrl_in.y, k.ctrl_out.x, k.ctrl_out.y
            );
        }
    }
    s
}

/// Anker innerhalb 0,01 px, Henkel innerhalb `h` px (re §1.4: Henkel bei P1 bis 1,8 px).
fn close_to(got: &Contour, want: &[[f64; 6]], h: f64) -> Result<(), String> {
    if got.nodes.len() != want.len() {
        return Err(format!("{} statt {} Knoten", got.nodes.len(), want.len()));
    }
    for (k, w) in got.nodes.iter().zip(want) {
        let da = (k.anchor.x - w[0]).abs().max((k.anchor.y - w[1]).abs());
        let dh = [(k.ctrl_in, w[2], w[3]), (k.ctrl_out, w[4], w[5])]
            .iter()
            .map(|(p, x, y)| (p.x - x).abs().max((p.y - y).abs()))
            .fold(0.0, f64::max);
        if da > 0.01 || dh > h {
            return Err(format!(
                "Knoten {:.4}/{:.4}: Anker {da:.4}, Henkel {dh:.4}",
                k.anchor.x, k.anchor.y
            ));
        }
    }
    Ok(())
}

/// Endergebnis des Originals (Stufe 7 in `re34/daten/stufen.txt`) als Knoten (Anker, Henkel ein, Henkel aus).
fn e1_want(w: i32) -> Vec<[f64; 6]> {
    match w {
        -5 => vec![
            [
                56.22516, 212.284576, 40.72187, 176.311127, 73.23474, 251.753189,
            ],
            [
                143.774841, 197.715424, 159.278091, 233.688873, 126.765305, 158.2468,
            ],
        ],
        _ => vec![
            [
                52.4509659, 213.609512, 34.9119072, 173.415878, 72.2022, 258.8727,
            ],
            [
                147.549042, 196.390488, 165.087967, 236.584122, 127.797958, 151.127289,
            ],
        ],
    }
}

/// T-7: E1 innen 5 und 1 wie das Original (Ablauf re §1.2): Endknoten innerhalb 0,01 px.
#[test]
fn inset_e1_against_original() {
    for (d, w) in [(5.0, -5), (1.0, -1)] {
        let r = inset_path_refit(
            &[e1()],
            d,
            JoinKind::Bevel,
            10.0,
            false,
            &OffsetOptions::ORIGINAL,
        );
        assert_eq!(r.len(), 1, "{}", dump("E1", &r));
        if let Err(e) = close_to(&r[0], &e1_want(w), 0.25) {
            panic!(
                "E1 {w}: {e}
{}",
                dump("E1", &r)
            );
        }
    }
}

/// T-7: P1 innen 5/1 – Konturzahl, Reihenfolge und die bereits gleichen Konturen.
#[test]
fn inset_p1_partial_against_original() {
    let r = inset_path_refit(
        &[p1()],
        5.0,
        JoinKind::Bevel,
        10.0,
        false,
        &OffsetOptions::ORIGINAL,
    );
    assert_eq!(r.len(), 2, "{}", dump("P1 -5", &r));
    let want = [
        [
            210.409988, 196.81, 251.696259, 157.521576, 210.409988, 196.81,
        ],
        [
            293.384979, 211.894989, 293.384979, 211.894989, 264.018921, 176.620956,
        ],
    ];
    close_to(&r[0], &want, 2.0).unwrap_or_else(|e| panic!("P1 -5 C0: {e}"));
    assert!(
        (r[1].nodes[0].anchor.x - 355.13).abs() < 0.01,
        "{}",
        dump("P1 -5", &r)
    );
    let r = inset_path_refit(
        &[p1()],
        1.0,
        JoinKind::Bevel,
        10.0,
        false,
        &OffsetOptions::ORIGINAL,
    );
    assert_eq!(r.len(), 3, "{}", dump("P1 -1", &r));
    let want = [
        [
            340.514984, 226.565, 330.165344, 231.358246, 340.514984, 226.565,
        ],
        [310.85498, 221.17, 310.85498, 221.17, 319.37323, 228.080383],
    ];
    close_to(&r[1], &want, 2.0).unwrap_or_else(|e| panic!("P1 -1 C1: {e}"));
    assert!(
        (r[0].nodes[0].anchor.x - 202.055).abs() < 0.01
            || (r[0].nodes[0].anchor.x - 304.16).abs() < 0.01,
        "{}",
        dump("P1 -1", &r)
    );
}

/// Endknoten des Originals (Stufe 7 in `re\re34\daten\stufen.txt`) je Fall und Kontur.
fn final_anchors(name: &str) -> Vec<Vec<(f64, f64)>> {
    match name {
        "P1 innen 5" => vec![
            vec![(210.409988, 196.81), (293.384979, 211.894989)],
            vec![(355.13, 223.12), (410.849976, 233.25499)],
        ],
        "P1 innen 1" => vec![
            vec![(304.16, 217.92), (202.055, 199.355)],
            vec![(340.514984, 226.565), (310.85498, 221.17)],
            vec![(418.205, 238.655), (346.25, 225.575)],
        ],
        "P1 aussen 5" => vec![vec![
            (309.60498, 214.845),
            (196.454712, 196.474243),
            (199.1056, 204.919357),
            (305.06, 224.18),
            (343.94, 231.25),
            (419.105469, 244.919327),
            (424.474731, 237.769135),
            (342.625, 220.849991),
        ]],
        "E1 innen 5" => vec![vec![(56.22516, 212.284576), (143.774841, 197.715424)]],
        _ => vec![vec![(52.4509659, 213.609512), (147.549042, 196.390488)]],
    }
}

/// Anker einer Kontur gegen das Original, Startpunkt frei (zyklisch): `Ok(Versatz des Starts)`.
fn anchors_cyclic(got: &Contour, want: &[(f64, f64)], tol: f64) -> Result<usize, String> {
    let n = want.len();
    if got.nodes.len() != n {
        return Err(format!("{} statt {n} Knoten", got.nodes.len()));
    }
    (0..n)
        .find(|&r| {
            (0..n).all(|i| {
                let (a, w) = (got.nodes[(i + r) % n].anchor, want[i]);
                (a.x - w.0).abs().max((a.y - w.1).abs()) <= tol
            })
        })
        .ok_or_else(|| "Anker weichen ab".to_string())
}

fn inset_cases() -> [(&'static str, Contour, f64, bool); 5] {
    [
        ("P1 innen 5", p1(), 5.0, false),
        ("P1 innen 1", p1(), 1.0, false),
        ("P1 aussen 5", p1(), 5.0, true),
        ("E1 innen 5", e1(), 5.0, false),
        ("E1 innen 1", e1(), 1.0, false),
    ]
}

/// K-4: P1 innen 1 und außen 5 wie das Original – Konturzahl, Knotenzahl und alle Anker ≤ 0,05 px
/// (Startpunkt frei). Ursachen behoben: geschlossene Zwei-Knoten-Teilfläche wird vereinfacht und
/// zerlegt (Stufen 3/4), Innenecken des Versatzes geschnitten statt über den Quellknoten verbunden
/// (Stufe 5).
#[test]
fn inset_p1_inner_corner_cut() {
    for (name, c, d, expand) in inset_cases()
        .into_iter()
        .filter(|c| c.0 == "P1 innen 1" || c.0 == "P1 aussen 5")
    {
        let r = inset_path_refit(
            &[c],
            d,
            JoinKind::Bevel,
            10.0,
            expand,
            &OffsetOptions::ORIGINAL,
        );
        let want = final_anchors(name);
        assert_eq!(r.len(), want.len(), "{}", dump(name, &r));
        for (g, w) in r.iter().zip(&want) {
            anchors_cyclic(g, w, 0.05).unwrap_or_else(|e| {
                panic!(
                    "{name}: {e}
{}",
                    dump(name, &r)
                )
            });
        }
    }
    // Stufen 3/4: die mittlere Teilfläche (2 Knoten, geschlossen) wird am y-Extremum geteilt und zerlegt
    let st = fp_vector::pathops::offset::inset_path_stages(
        &[p1()],
        5.0,
        JoinKind::Bevel,
        10.0,
        false,
        &OffsetOptions::ORIGINAL,
    );
    let mid = |cs: &[Contour]| {
        cs.iter()
            .find(|c| {
                c.nodes.len() < 30
                    && c.nodes.iter().any(|k| (k.anchor.x - 307.31).abs() < 0.01)
                    && c.nodes.iter().all(|k| k.anchor.x > 300.0)
            })
            .map(|c| c.nodes.len())
    };
    assert_eq!(mid(&st[2]), Some(3));
    assert_eq!(mid(&st[3]), Some(18));
}

/// Pfad einlassen gegen das Original (re 34, Stufe 7 in `stufen.txt`): alle 5 Fälle, Anker ≤ 0,05 px.
/// Offen (K-4, `docs/berichte/k4.md`): P1 innen 5 C1 hat einen Zusatzknoten 380,545/197,175. Das
/// Original teilt im Versatz (Stufe 5) das Quellstück 378,43 → 374,00 bei t = 0,5 (Knoten
/// 379,224/198,120), Firepixel nicht; das Fehlermaß von `@10002160` ist unbekannt (re 16 §3.3, A).
/// Startpunkte: P1 innen 1 C0/C2 und P1 außen 5 beginnen an einem anderen Knoten.
#[test]

fn inset_against_original() {
    for (name, c, d, expand) in inset_cases() {
        let r = inset_path_refit(
            &[c],
            d,
            JoinKind::Bevel,
            10.0,
            expand,
            &OffsetOptions::ORIGINAL,
        );
        let want = final_anchors(name);
        assert_eq!(r.len(), want.len(), "{}", dump(name, &r));
        for (g, w) in r.iter().zip(&want) {
            anchors_cyclic(g, w, 0.05).unwrap_or_else(|e| {
                panic!(
                    "{name}: {e}
{}",
                    dump(name, &r)
                )
            });
        }
    }
}

/// Ergebnisse ausgeben (`cargo test -p fp-vector --test m6_refit -- --nocapture`).
#[test]
fn print_all_cases() {
    let mut out = String::new();
    for (name, c, amt) in [
        ("P1 simplify 1", p1(), 1.0),
        ("P1 simplify 10", p1(), 10.0),
        ("E1 simplify 1", e1(), 1.0),
    ] {
        out += &dump(name, &simplify(&[c], amt));
        out += "\n";
    }
    for (name, c, d, expand) in [
        ("P1 inset 5", p1(), 5.0, false),
        ("P1 inset 1", p1(), 1.0, false),
        ("P1 outset 5", p1(), 5.0, true),
        ("E1 inset 5", e1(), 5.0, false),
        ("E1 inset 1", e1(), 1.0, false),
    ] {
        out += &dump(
            name,
            &inset_path_refit(
                &[c],
                d,
                JoinKind::Bevel,
                10.0,
                expand,
                &OffsetOptions::ORIGINAL,
            ),
        );
        out += "\n";
    }
    println!("{out}");
}

/// K-4: all stages of `inset_path_stages` in the format of `re\re34\daten\stufen.txt`
/// (`FP_STAGES_OUT=<file> cargo test -p fp-vector --test m6_refit dump_stages -- --ignored`).
#[test]
#[ignore = "diagnostic dump"]
fn dump_stages() {
    use fp_vector::pathops::offset::inset_path_stages;
    let fmt = |c: &Contour| {
        let mut s = String::new();
        let n = c.nodes.len();
        s += &format!("M {},{}\n", c.nodes[0].anchor.x, c.nodes[0].anchor.y);
        let segs = if c.closed { n } else { n - 1 };
        for k in 0..segs {
            let (a, b) = (c.nodes[k], c.nodes[(k + 1) % n]);
            if a.ctrl_out == a.anchor && b.ctrl_in == b.anchor {
                s += &format!("L {},{}\n", b.anchor.x, b.anchor.y);
            } else {
                s += &format!(
                    "C {},{} {},{} {},{}\n",
                    a.ctrl_out.x, a.ctrl_out.y, b.ctrl_in.x, b.ctrl_in.y, b.anchor.x, b.anchor.y
                );
            }
        }
        s + "Z\n"
    };
    let mut out = String::new();
    for (name, c) in [("P1", p1()), ("E1", e1())] {
        for w in [-5, -1, 5] {
            let st = inset_path_stages(
                &[c.clone()],
                (w as f64).abs(),
                JoinKind::Bevel,
                10.0,
                w > 0,
                &OffsetOptions::ORIGINAL,
            );
            for (i, s) in st.iter().enumerate().skip(1) {
                out += &format!("== {name} w={w} | Stufe {}\n", i + 1);
                for c in s {
                    out += &fmt(c);
                }
            }
        }
    }
    match std::env::var("FP_STAGES_OUT") {
        Ok(p) => std::fs::write(p, out).unwrap(),
        Err(_) => println!("{out}"),
    }
}

/// T-9 (re 39 §1.3): the piece of P1 inset 5 is inexact (α 0.5044, β 1.0893), chord 5.59 > 2.5
/// ⇒ one split at t = 0.5; the new node is 379.224/198.120 like stage 5 of the original. Almost
/// straight pieces are never split.
#[test]
fn t9_inset_split_rule() {
    use fp_geom::{CubicBez, Vec2};
    use fp_vector::pathops::offset::{offset_piece, offset_segment_inset};
    let seg = CubicBez::new(
        Vec2::new(378.432556, 192.631241),
        Vec2::new(376.974152, 193.369614),
        Vec2::new(375.135254, 194.99527),
        Vec2::new(374.002441, 196.035309),
    );
    assert!(offset_piece(&seg, -5.0).1, "inexact");
    let r = offset_segment_inset(&seg, -5.0);
    assert_eq!(r.len(), 2);
    assert!(
        r[0].p3.distance(Vec2::new(379.224, 198.120)) < 0.001,
        "{:?}",
        r[0].p3
    );
    assert!(
        !offset_piece(&seg.split(0.5).0, -5.0).1,
        "first half accepted"
    );
    let straight = CubicBez::new(
        Vec2::new(0.0, 0.0),
        Vec2::new(10.0, 0.5),
        Vec2::new(20.0, -0.5),
        Vec2::new(30.0, 0.0),
    );
    assert_eq!(offset_segment_inset(&straight, 5.0).len(), 1);
}
