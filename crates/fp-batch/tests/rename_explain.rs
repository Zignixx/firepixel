//! BATCH-3: Klartext-Erklärung des Suchmusters, verständliche Fehler mit Stelle, Erklärung des
//! Ersetzen-Felds, Übersetzung Baukasten → gleichwertiges Muster, erweiterte Platzhalter,
//! Spickzettel, zeichenweiser Vergleich (SPEC-BATCH §4.2/§4.3). Logikfälle nach §4b: leere Eingabe,
//! ungültiges Muster, Unicode, sehr lange Namen, 50× Wert ändern.

use chrono::NaiveDate;
use fp_batch::ItemKind;
use fp_batch::explain::{
    self, CHEAT_SHEET, apply_sample, builder_to_regex, char_diff, check_pattern, explain_pattern,
    explain_replacement,
};
use fp_batch::item::{ExifSummary, ItemInfo};
use fp_batch::naming::{NameContext, NameHint, build_name};
use fp_batch::recipe::*;
use std::path::PathBuf;

/// Sprache ist hier prozessweit: Tests mit Texten laufen nacheinander.
static LANG: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn lang(code: &str) -> std::sync::MutexGuard<'static, ()> {
    let g = LANG.lock().unwrap_or_else(|e| e.into_inner());
    assert!(fp_i18n::set_language(code));
    g
}

fn de() -> std::sync::MutexGuard<'static, ()> {
    lang("de")
}

fn now() -> chrono::NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 10, 13)
        .and_then(|d| d.and_hms_opt(9, 5, 7))
        .expect("date")
}

fn info(path: &str) -> ItemInfo {
    ItemInfo {
        path: PathBuf::from(path),
        root: None,
        kind: ItemKind::Image,
        format: "png".into(),
        bytes: 10,
        width: 640,
        height: 480,
        frames: 1,
        modified: None,
        exif: None,
    }
}

fn ctx<'a>(i: &'a ItemInfo, idx: usize, folder_idx: usize) -> NameContext<'a> {
    NameContext {
        info: i,
        list_index: idx,
        folder_index: folder_idx,
        now: now(),
        out_ext: "webp".into(),
        out_format: "webp".into(),
    }
}

fn name(spec: &RenameSpec, i: &ItemInfo, idx: usize) -> String {
    build_name(spec, &ctx(i, idx, idx / 2))
        .expect("name")
        .file_name
}

// ------------------------------------------------------------------ Erklärung

#[test]
fn erklaerung_beispiel_aus_der_spezifikation() {
    let _g = de();
    assert_eq!(
        explain_pattern(r"^IMG_(\d+)").unwrap(),
        "am Anfang „IMG_“, dann eine oder mehrere Ziffern (Gruppe 1)"
    );
    assert_eq!(
        explain_pattern(r"(\d+)$").unwrap(),
        "eine oder mehrere Ziffern (Gruppe 1) am Ende"
    );
    assert_eq!(
        explain_pattern("^(.*)$").unwrap(),
        "der ganze Name: beliebig viele Zeichen (auch keine) (Gruppe 1)"
    );
    assert_eq!(
        explain_pattern(r"\s+").unwrap(),
        "ein oder mehrere Leerräume"
    );
    assert_eq!(
        explain_pattern(r"(\d{4})-(\d\d)-(\d\d)").unwrap(),
        "4 Ziffern (Gruppe 1), dann „-“, dann 2 Ziffern (Gruppe 2), dann „-“, dann 2 Ziffern (Gruppe 3)"
    );
    assert_eq!(
        explain_pattern("[a-z0-9]+").unwrap(),
        "ein oder mehrere Zeichen aus a–z, 0–9"
    );
    assert_eq!(explain_pattern("[^_]").unwrap(), "ein Zeichen außer _");
    assert_eq!(
        explain_pattern("foto|bild").unwrap(),
        "entweder „foto“ oder „bild“"
    );
    assert_eq!(
        explain_pattern(r"(?P<jahr>\d{4})").unwrap(),
        "4 Ziffern (Gruppe 1 „jahr“)"
    );
    assert_eq!(
        explain_pattern("a{2,4}?").unwrap(),
        "„a“, 2- bis 4-mal (so wenige wie möglich)"
    );
    assert_eq!(
        explain_pattern("(?i)img").unwrap(),
        "ab hier Groß-/Kleinschreibung egal, dann „img“"
    );
    assert_eq!(explain_pattern("x ").unwrap(), "„x “");
    assert_eq!(explain_pattern(" ").unwrap(), "ein Leerzeichen");
    assert_eq!(explain_pattern(r"\bfoo").unwrap(), "Wortgrenze, dann „foo“");
    assert_eq!(
        explain_pattern(r"(ab)+").unwrap(),
        "„ab“ (Gruppe 1), ein- oder mehrmals"
    );
    assert_eq!(
        explain_pattern(r"(?:_\d)?").unwrap(),
        "optional („_“, dann eine Ziffer)"
    );
    assert_eq!(
        explain_pattern(r"\p{L}").unwrap(),
        "ein Zeichen der Unicode-Klasse \\p{L}"
    );
}

#[test]
fn erklaerung_englisch() {
    let _g = lang("en");
    assert_eq!(
        explain_pattern(r"^IMG_(\d+)").unwrap(),
        "at the start \"IMG_\", then one or more digits (group 1)"
    );
    assert_eq!(
        explain_replacement("${1}_{n:3}"),
        "group 1 + \"_\" + number (3 digits)"
    );
    let p = check_pattern("(abc", false).expect("Fehler");
    assert!(
        p.message.starts_with("Error at position 1:"),
        "{}",
        p.message
    );
    drop(_g);
}

#[test]
fn leere_eingabe() {
    let _g = de();
    assert_eq!(
        explain_pattern("").unwrap(),
        "leeres Muster – passt an jeder Stelle"
    );
    assert_eq!(explain_pattern("^$").unwrap(), "ein leerer Name");
    assert_eq!(explain_pattern("^").unwrap(), "Anfang des Namens");
    assert_eq!(
        explain_replacement(""),
        "nichts – der Treffer wird entfernt"
    );
    assert!(check_pattern("", false).is_none());
    // leeres Muster im Lauf: Ersetzen fügt an jeder Stelle ein – gleich wie die regex-Crate
    let spec = RenameSpec {
        mode: RenameMode::Regex,
        regex: RegexRename {
            find: String::new(),
            replace: "-".into(),
            all: false,
            ..RegexRename::default()
        },
        ..RenameSpec::default()
    };
    assert_eq!(name(&spec, &info("C:\\a\\ab.png"), 0), "-ab.webp");
}

#[test]
fn ungueltiges_muster_verstaendlich_mit_stelle() {
    let _g = de();
    let cases = [
        ("(abc", 1, "Klammer „(“ wird nicht geschlossen"),
        ("abc)", 4, "Klammer „)“ ohne passende „(“"),
        ("[a-z", 1, "Eckige Klammer „[“ wird nicht geschlossen"),
        ("*abc", 1, "Wiederholung"),
        (r"(a)\1", 4, "Rückverweise"),
        ("(?=x)", 1, "Vorausschau"),
        ("[z-a]", 2, "Ungültiger Bereich"),
        (r"abc\", 4, "„\\“ am Ende"),
    ];
    for (pat, pos, start) in cases {
        let p = check_pattern(pat, false).unwrap_or_else(|| panic!("{pat} ist ungültig"));
        assert_eq!(p.position, Some(pos), "{pat}: {}", p.message);
        assert!(
            p.message.starts_with(&format!("Fehler an Stelle {pos}: ")),
            "{pat}: {}",
            p.message
        );
        assert!(p.message.contains(start), "{pat}: {}", p.message);
        assert!(!p.detail.is_empty());
        assert!(explain_pattern(pat).is_err());
    }
    // gültig nach der Syntax, aber unbekannte Unicode-Klasse → Meldung der Übersetzung
    assert!(check_pattern(r"\p{Gibtsnicht}", false).is_some());
    // gültige Muster
    for pat in [
        r"^IMG_(\d+)",
        "(?i)foto",
        r"(?<n>\d+)",
        "[[:alpha:]]+",
        r"\p{Greek}",
    ] {
        assert!(check_pattern(pat, false).is_none(), "{pat}");
        assert!(explain_pattern(pat).is_ok(), "{pat}");
    }
    // Lauf: ungültiges Muster → Fehler, keine Panik
    let spec = RenameSpec {
        mode: RenameMode::Regex,
        regex: RegexRename {
            find: "(abc".into(),
            ..RegexRename::default()
        },
        ..RenameSpec::default()
    };
    assert!(build_name(&spec, &ctx(&info("C:\\a\\x.png"), 0, 0)).is_err());
}

#[test]
fn erklaerung_ersetzen() {
    let _g = de();
    assert_eq!(
        explain_replacement("$3.$2.$1"),
        "Gruppe 3 + „.“ + Gruppe 2 + „.“ + Gruppe 1"
    );
    assert_eq!(
        explain_replacement("$1_x"),
        "Gruppe 1 + „_x“",
        "$1_x = Gruppe 1 + _x (wie der Lauf)"
    );
    assert_eq!(
        explain_replacement("${jahr}-$0"),
        "Gruppe „jahr“ + „-“ + der ganze Treffer"
    );
    assert_eq!(
        explain_replacement("{exif-date:YYYYMMDD}_{n:4:10:5}"),
        "Aufnahmedatum (YYYYMMDD) + „_“ + Nummer (4 Stellen, ab 10, Schritt 5)"
    );
    assert_eq!(
        explain_replacement("{nf:2}{w}x{h}"),
        "Nummer (2 Stellen) je Ordner + Breite + „x“ + Höhe"
    );
    assert_eq!(
        explain_replacement("{name:first:3}{name:trim}{folder}{ext}{format}{exif:lens}"),
        "erste 3 Zeichen des Namens + Name ohne Nummer am Ende + Ordnername + ursprüngliche Endung + Zielformat + Objektiv"
    );
    assert_eq!(explain_replacement("{unbekannt}$$"), "„{unbekannt}$“");
    assert_eq!(explain_replacement("{{n}"), "„{n}“");
}

// ------------------------------------------------------------------ Platzhalter im Lauf

#[test]
fn erweiterte_platzhalter() {
    let mut i = info("C:\\Urlaub\\Strand_0042.png");
    i.exif = Some(ExifSummary {
        model: Some("X100".into()),
        lens: Some("23mm".into()),
        ..ExifSummary::default()
    });
    let re = |rep: &str| RenameSpec {
        mode: RenameMode::Regex,
        regex: RegexRename {
            find: "^(.*)$".into(),
            replace: rep.into(),
            all: true,
            ..RegexRename::default()
        },
        ..RenameSpec::default()
    };
    let n = |rep: &str, idx: usize, fi: usize| {
        build_name(&re(rep), &ctx(&i, idx, fi)).expect("name").stem
    };
    assert_eq!(n("{n:3:10:5}", 2, 0), "020");
    assert_eq!(n("{nf:2:0:1}", 7, 3), "03");
    assert_eq!(n("{n}", 0, 0), "1");
    assert_eq!(n("{name:first:3}", 0, 0), "Str");
    assert_eq!(n("{name:last:4}", 0, 0), "0042");
    assert_eq!(n("{name:trim}", 0, 0), "Strand");
    assert_eq!(n("{exif:model}_{exif:lens}_{exif:iso}", 0, 0), "X100_23mm_");
    assert_eq!(n("{format}", 0, 0), "webp");
    assert_eq!(n("{{n}", 0, 0), "{n}");
    assert_eq!(
        n("{name:egal}", 0, 0),
        "{name_egal}",
        "unbekannt bleibt stehen (: wird zu _)"
    );
    assert_eq!(n("a$$b", 0, 0), "a$b");
}

#[test]
fn ersetzungen_gelten_auch_im_regex_modus() {
    let mut spec = RenameSpec {
        replacements: vec![Replacement {
            find: "x".into(),
            replace: "y".into(),
            all: true,
            match_case: true,
        }],
        ..RenameSpec::default()
    };
    let i = info("C:\\a\\axbx.png");
    assert_eq!(name(&spec, &i, 0), "ayby.webp");
    spec.mode = RenameMode::Regex;
    spec.regex = builder_to_regex(&spec);
    assert_eq!(name(&spec, &i, 0), "ayby.webp");
}

// ------------------------------------------------------------------ Baukasten → Regex

fn sample_specs() -> Vec<RenameSpec> {
    use NamePart as P;
    let parts: Vec<Vec<NamePart>> = vec![
        vec![P::Original {
            take: OriginalTake::Whole,
            count: 0,
        }],
        vec![
            P::Original {
                take: OriginalTake::Whole,
                count: 0,
            },
            P::Text {
                text: "_web".into(),
            },
        ],
        vec![
            P::Text {
                text: "Bild".into(),
            },
            P::Text { text: "_".into() },
            P::Counter {
                start: 1,
                step: 1,
                digits: 3,
                per_folder: false,
            },
        ],
        vec![
            P::Counter {
                start: 7,
                step: 5,
                digits: 2,
                per_folder: true,
            },
            P::Text { text: "-".into() },
            P::Original {
                take: OriginalTake::NoTrailingDigits,
                count: 0,
            },
        ],
        vec![
            P::Date {
                source: DateSource::Exif,
                format: "YYYY-MM-DD".into(),
            },
            P::Text { text: "_".into() },
            P::Counter {
                start: 1,
                step: 1,
                digits: 3,
                per_folder: false,
            },
        ],
        vec![
            P::Date {
                source: DateSource::Today,
                format: "DD.MM.YYYY".into(),
            },
            P::Original {
                take: OriginalTake::First,
                count: 4,
            },
            P::Original {
                take: OriginalTake::Last,
                count: 2,
            },
        ],
        vec![
            P::Size {
                which: SizeWhich::Both,
            },
            P::Size {
                which: SizeWhich::Width,
            },
            P::Size {
                which: SizeWhich::Height,
            },
            P::Folder,
            P::Format,
        ],
        vec![
            P::Exif {
                field: ExifField::Model,
            },
            P::Exif {
                field: ExifField::Iso,
            },
            P::Date {
                source: DateSource::Modified,
                format: "YYYYMMDD".into(),
            },
        ],
        // Sonderfälle im Text: $, {, Ziffern direkt nach dem Originalnamen, Leerzeichen, Unicode
        vec![
            P::Original {
                take: OriginalTake::Whole,
                count: 0,
            },
            P::Text {
                text: "2 $1 {n} ${x} 🌸 Größe".into(),
            },
        ],
        vec![P::Text { text: "{{".into() }, P::Text { text: "}".into() }],
        vec![],
    ];
    let mut out = Vec::new();
    for (k, p) in parts.into_iter().enumerate() {
        let mut s = RenameSpec {
            parts: p,
            ..RenameSpec::default()
        };
        // Optionen wechselnd dazu – sie gelten in beiden Arten gleich
        if k % 2 == 1 {
            s.case = CaseMode::Lower;
            s.spaces = Some("_".into());
            s.prefix = "pre ".into();
        }
        if k % 3 == 2 {
            s.transliterate = true;
            s.strip_special = true;
            s.replacements.push(Replacement {
                find: "_".into(),
                replace: "-".into(),
                all: false,
                match_case: true,
            });
            s.suffix = " Ende".into();
            s.case = CaseMode::Title;
        }
        out.push(s);
    }
    out
}

#[test]
fn baukasten_in_gleichwertiges_muster() {
    let mut files = [
        info("C:\\Urlaub 2026\\Strand_0042.png"),
        info("C:\\a\\IMG_1234.JPG"),
        info("C:\\a\\über größe.png"),
        info("C:\\a\\x.fw.png"),
        info("C:\\b\\🌸 blüte 7.png"),
        info("C:\\b\\.versteckt.png"),
    ];
    files[1].exif = Some(ExifSummary {
        date_taken: NaiveDate::from_ymd_opt(2024, 2, 29).and_then(|d| d.and_hms_opt(23, 59, 1)),
        model: Some("Cam $1".into()),
        iso: Some(200),
        ..ExifSummary::default()
    });
    for (k, b) in sample_specs().into_iter().enumerate() {
        let mut r = b.clone();
        r.mode = RenameMode::Regex;
        r.regex = builder_to_regex(&b);
        assert_eq!(r.regex.find, "^(.*)$");
        for (idx, f) in files.iter().enumerate() {
            let a = build_name(&b, &ctx(f, idx, idx / 2)).expect("Baukasten");
            let c = build_name(&r, &ctx(f, idx, idx / 2)).expect("Regex");
            assert_eq!(
                a,
                c,
                "Fall {k}, Datei {}: Ersetzen {:?}",
                f.path.display(),
                r.regex.replace
            );
        }
    }
}

#[test]
fn uebersetzung_50x_ist_stabil() {
    // 50× umschalten bzw. übersetzen: immer dasselbe Muster, keine angehäuften Teile
    let spec = &sample_specs()[2];
    let first = builder_to_regex(spec);
    let mut s = spec.clone();
    for _ in 0..50 {
        s.mode = RenameMode::Regex;
        if explain::regex_is_replaceable(&s) {
            s.regex = builder_to_regex(&s);
        }
        s.mode = RenameMode::Builder;
    }
    assert_eq!(s.regex, first);
    assert_eq!(s.parts, spec.parts, "Baukasten bleibt erhalten");
    assert_eq!(first.replace, "Bild_{n:3}");
    // eigenes Muster wird nicht überschrieben
    s.regex.find = r"^IMG_(\d+)".into();
    assert!(!explain::regex_is_replaceable(&s));
}

// ------------------------------------------------------------------ Spickzettel

#[test]
fn spickzettel_beispiele_gueltig() {
    let _g = de();
    assert_eq!(CHEAT_SHEET.len(), 8);
    for e in CHEAT_SHEET {
        assert!(check_pattern(e.find, false).is_none(), "{}", e.find);
        assert!(fp_i18n::has(e.key), "Text {}", e.key);
        let out = apply_sample(e.find, e.replace, e.sample).expect("anwendbar");
        assert_ne!(out, e.sample, "{} verändert das Beispiel", e.key);
        assert!(!explain_pattern(e.find).unwrap().is_empty());
    }
    assert_eq!(
        apply_sample(r"(\d{4})-(\d\d)-(\d\d)", "$3.$2.$1", "scan 2026-10-13").as_deref(),
        Some("scan 13.10.2026")
    );
    assert_eq!(
        apply_sample(r"[\s_-]*\d+$", "", "DSC_0042").as_deref(),
        Some("DSC")
    );
}

// ------------------------------------------------------------------ Vergleich alt → neu

fn join(v: &[explain::DiffPart]) -> String {
    v.iter().map(|p| p.text.as_str()).collect()
}

#[test]
fn zeichenweiser_vergleich() {
    let (a, b) = char_diff("IMG_1234.jpg", "Foto_1234.webp");
    assert_eq!(join(&a), "IMG_1234.jpg");
    assert_eq!(join(&b), "Foto_1234.webp");
    assert!(a.iter().any(|p| p.changed && p.text.contains("IMG")));
    assert!(b.iter().any(|p| !p.changed && p.text.contains("_1234.")));
    let (a, b) = char_diff("gleich.png", "gleich.png");
    assert!(a.iter().chain(&b).all(|p| !p.changed));
    let (a, b) = char_diff("", "neu");
    assert!(a.is_empty());
    assert_eq!(
        b,
        vec![explain::DiffPart {
            text: "neu".into(),
            changed: true
        }]
    );
    // Unicode und sehr lange Namen (kein Überlauf, keine Panik)
    let (a, b) = char_diff("🌸 Blüte.png", "bluete.webp");
    assert_eq!(join(&a), "🌸 Blüte.png");
    assert_eq!(join(&b), "bluete.webp");
    let long_a = "x".repeat(300) + "ä";
    let long_b = "y".repeat(250) + &"x".repeat(300);
    let (a, b) = char_diff(&long_a, &long_b);
    assert_eq!(join(&a), long_a);
    assert_eq!(join(&b), long_b);
}

#[test]
fn sehr_lange_namen_und_unicode() {
    let long = format!("C:\\a\\{}.png", "ä".repeat(400));
    let spec = RenameSpec {
        mode: RenameMode::Regex,
        regex: RegexRename {
            find: "ä".into(),
            replace: "ö$0".into(),
            all: true,
            ..RegexRename::default()
        },
        ..RenameSpec::default()
    };
    let r = build_name(&spec, &ctx(&info(&long), 0, 0)).expect("name");
    assert!(r.hints.contains(&NameHint::Truncated));
    assert!(r.file_name.chars().count() <= 255);
    assert!(r.file_name.ends_with(".webp"));
    let _g = de();
    assert!(explain_pattern(&"(a)".repeat(500)).is_ok(), "viele Gruppen");
    assert!(
        check_pattern(&"(".repeat(300), false).is_some(),
        "zu tief verschachtelt → Meldung, keine Panik"
    );
    assert_eq!(explain_pattern("ü+").unwrap(), "„ü“, ein- oder mehrmals");
}
