//! BATCH-3: Endkarte „Dateiname“ im Batch-Studio (`src\batch\rename_ui.rs`) – Baukasten mit
//! Bausteinen, Regex mit Klartext-Erklärung, Vorlagen, Live-Tabelle alt → neu mit Konflikten,
//! ungültigen Zeichen und unveränderten Namen. Logikfälle nach AGENTEN_REGELN §4b (leere Eingabe,
//! ungültiges Regex, Konflikte, Unicode, sehr lange Namen, 50× Wert ändern), 3 Designs × de/en ohne
//! Abschneiden. Bilder (`--ignored batch3_bilder`): `docs\screenshots\batch3_*.png`.

use egui::epaint::Shape;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fp_batch::naming::NameHint;
use fp_batch::recipe::{NamePart, RenameMode};
use fp_i18n::t;
use fp_ui::FirepixelApp;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::batch::{self, CardId, Mode, rename_ui};
use fp_ui::theme::ThemeKind;
use serde_json::json;
use std::path::{Path, PathBuf};

// ------------------------------------------------------------------ Hilfen

/// Eigener Ordner je Aufruf (Tests laufen parallel; gleiche Namen dürfen sich nicht löschen).
fn dir(name: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let k = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let d = std::env::temp_dir()
        .join("firepixel-batch3")
        .join(std::process::id().to_string())
        .join(format!("{k}-{name}"));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("Ordner");
    d
}

fn png_file(d: &Path, name: &str, w: u32, h: u32) -> PathBuf {
    let p = d.join(name);
    let mut px = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            px.extend_from_slice(&[(x * 255 / w) as u8, (y * 255 / h) as u8, 160, 255]);
        }
    }
    let f = std::fs::File::create(&p).expect("Datei");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()
        .and_then(|mut wr| wr.write_image_data(&px))
        .expect("PNG");
    p
}

fn state(kind: ThemeKind) -> AppState {
    fp_engine::shell::disable();
    let session = fp_ui::ui_commands::new_session(fp_engine::Preferences::new());
    let mut st = AppState::new(session, UiSettings::default(), None);
    st.sync_render = true;
    st.os_clipboard = false;
    st.set_theme(kind);
    st
}

type H = Harness<'static, FirepixelApp>;

fn harness_ex(st: AppState, w: f32, h: f32, ppp: f32, gpu: bool) -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(
        &ctx,
        fp_engine::Session::default(),
        UiSettings::default(),
        false,
    );
    app.state = st;
    let b = Harness::builder().renderer(crate::gpu_common::renderer())
        .with_size(vec2(w / ppp, h / ppp))
        .with_pixels_per_point(ppp)
        .with_max_steps(30)
        .with_step_dt(1.0 / 60.0);
    let b = if gpu { b.renderer(crate::gpu_common::renderer()) } else { b };
    let mut hn = b.build_ui_state(|ui, a: &mut FirepixelApp| a.root_ui(ui), app);
    hn.run_steps(4);
    hn
}

fn studio(h: &mut H) -> &mut batch::Studio {
    batch::studio_mut(&mut h.state_mut().state).expect("Studio offen")
}

/// Studio mit Dateien, Endkarte „Dateiname“ offen.
fn open(names: &[&str], kind: ThemeKind, mode: Mode, w: f32, hgt: f32, ppp: f32) -> (H, PathBuf) {
    let d = dir(&format!(
        "{}-{}-{w}-{ppp}-{:?}",
        names.len(),
        kind.label(),
        mode
    ));
    let mut st = state(kind);
    st.run("file.batchProcess", json!({}));
    let paths: Vec<PathBuf> = names
        .iter()
        .enumerate()
        .map(|(i, n)| png_file(&d, n, 40 + i as u32 * 8, 30))
        .collect();
    let mut h = harness_ex(st, w, hgt, ppp, false);
    {
        let s = studio(&mut h);
        s.mode = mode;
        if !paths.is_empty() {
            s.add_paths(&paths, false);
        }
        s.open_card = Some(CardId::Rename);
    }
    h.run_steps(4);
    (h, d)
}

fn open_default(names: &[&str], mode: Mode) -> (H, PathBuf) {
    open(names, ThemeKind::Normal, mode, 1440.0, 1600.0, 1.0)
}

#[derive(Debug, Clone)]
struct T {
    text: String,
    rect: Rect,
    clip: Rect,
}

fn texts(h: &H) -> Vec<T> {
    fn walk(s: &Shape, clip: Rect, out: &mut Vec<T>) {
        match s {
            Shape::Text(t) => {
                let text = t.galley.text().to_string();
                if !text.trim().is_empty() {
                    out.push(T {
                        text,
                        rect: t.visual_bounding_rect(),
                        clip,
                    });
                }
            }
            Shape::Vec(v) => v.iter().for_each(|x| walk(x, clip, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    for cs in &h.output().shapes {
        walk(&cs.shape, cs.clip_rect, &mut out);
    }
    out
}

fn find(h: &H, text: &str) -> Option<T> {
    texts(h).into_iter().find(|t| t.text == text)
}

fn find_containing(h: &H, part: &str) -> Option<T> {
    texts(h).into_iter().find(|t| t.text.contains(part))
}

fn click(h: &mut H, p: Pos2) {
    h.event(Event::PointerMoved(p));
    h.run_steps(1);
    for pressed in [true, false] {
        h.event(Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
    }
    h.run_steps(2);
}

fn key(h: &mut H, k: Key, m: Modifiers) {
    h.event(Event::Key {
        key: k,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: m,
    });
    h.event(Event::Key {
        key: k,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers: m,
    });
    h.run_steps(2);
}

fn lang(code: &str) {
    assert!(fp_i18n::set_language(code), "Sprache {code}");
}

fn table(h: &mut H) -> rename_ui::Table {
    let s = studio(h);
    let files: Vec<fp_batch::ItemInfo> = s
        .to_process()
        .into_iter()
        .map(|i| s.files[i].info.clone())
        .collect();
    rename_ui::compute_table(&s.recipe, &files)
}

/// Keine Rohschlüssel, nichts waagerecht abgeschnitten (außer am Rand eines Bildlaufbereichs bzw. mit „…“).
fn assert_clean(h: &H, label: &str) {
    let screen = h.ctx.content_rect();
    for x in texts(h) {
        assert!(
            !x.text.contains("batch-rn-")
                && !x.text.contains("batch-rx-")
                && !x.text.contains("batch-ui-"),
            "{label}: Rohschlüssel {:?}",
            x.text
        );
        let r = x.rect;
        let clip = x.clip.intersect(screen);
        if r.width() <= 0.0 || !clip.intersects(r) {
            continue;
        }
        let fully_v = r.top() >= clip.top() - 1.0 && r.bottom() <= clip.bottom() + 1.0;
        if fully_v {
            assert!(
                r.left() >= clip.left() - 1.5 && r.right() <= clip.right() + 1.5,
                "{label}: Text abgeschnitten {:?} {:?} Clip {:?}",
                x.text,
                r,
                x.clip
            );
        }
    }
}

// ------------------------------------------------------------------ Baukasten

#[test]
fn karte_zeigt_baukasten_optionen_und_tabelle() {
    lang("de");
    let (mut h, _d) = open_default(
        &["urlaub 01.png", "urlaub 02.png", "Strand.png"],
        Mode::Beginner,
    );
    assert!(
        find(&h, &t("batch-rn-part-original")).is_some(),
        "Chip Originalname"
    );
    assert!(find(&h, &t("batch-rn-add")).is_some(), "+ Baustein");
    assert!(find(&h, &t("batch-rn-section-preview")).is_some());
    assert!(
        find(&h, &t("batch-rn-table-old")).is_some()
            && find(&h, &t("batch-rn-table-new")).is_some()
    );
    // Vorgabe: Name bleibt gleich → unverändert markiert („=“)
    let tb = table(&mut h);
    assert_eq!(tb.rows.len(), 3);
    assert!(tb.rows.iter().all(|r| r.unchanged), "{:?}", tb.rows);
    assert!(find(&h, "=").is_some(), "Markierung unverändert");
    // Anfänger: kein Regex-Umschalter
    assert!(find(&h, &t("batch-ui-rename-mode-regex")).is_none());
    // Leerzeichen → _ und klein über das Rezept
    studio(&mut h).edit("rename", |r| {
        r.rename.spaces = Some("_".into());
        r.rename.case = fp_batch::recipe::CaseMode::Upper;
        true
    });
    h.run_steps(3);
    let tb = table(&mut h);
    assert_eq!(tb.rows[0].new, "URLAUB_01.png");
    assert_eq!(tb.changed, 3);
    assert!(
        find_containing(&h, "URLAUB_01").is_some(),
        "neue Namen in der Tabelle"
    );
    assert_clean(&h, "Baukasten");
}

#[test]
fn baustein_ueber_menue_hinzufuegen_und_entfernen() {
    lang("de");
    let (mut h, _d) = open_default(&["a.png", "b.png"], Mode::Beginner);
    let add = find(&h, &t("batch-rn-add"))
        .expect("+ Baustein")
        .rect
        .center();
    click(&mut h, add);
    let item = find(&h, &t("batch-rn-add-counter"))
        .expect("Menü: Nummer")
        .rect
        .center();
    click(&mut h, item);
    h.run_steps(2);
    assert_eq!(studio(&mut h).recipe.rename.parts.len(), 2);
    assert!(matches!(
        studio(&mut h).recipe.rename.parts[1],
        NamePart::Counter { digits: 3, .. }
    ));
    assert_eq!(
        rename_ui::selected(&h.ctx),
        Some(1),
        "neuer Baustein ist gewählt (Einstellungen offen)"
    );
    assert!(
        find(&h, &t("batch-rn-opt-digits")).is_some(),
        "Einstellungen der Nummer"
    );
    assert_eq!(table(&mut h).rows[1].new, "b002.png");
    // × am Chip entfernt
    let name = fp_i18n::tr!(
        "batch-rn-remove-part",
        part = rename_ui::chip_label(&studio(&mut h).recipe.rename.parts[1])
    );
    let x = h
        .query_by_label(&name)
        .map(|n| n.rect())
        .expect("× am Chip");
    click(&mut h, x.center());
    assert_eq!(studio(&mut h).recipe.rename.parts.len(), 1);
    // Rückgängig im Rezept holt ihn zurück
    batch::studio_mut(&mut h.state_mut().state)
        .expect("s")
        .end_gesture();
    assert!(studio(&mut h).undo());
    assert_eq!(studio(&mut h).recipe.rename.parts.len(), 2);
}

#[test]
fn bausteine_verschieben_tastatur_und_logik() {
    use fp_batch::recipe::RenameSpec;
    let mut spec = RenameSpec {
        parts: vec![
            NamePart::Text { text: "a".into() },
            NamePart::Text { text: "b".into() },
            NamePart::Text { text: "c".into() },
        ],
        ..RenameSpec::default()
    };
    assert_eq!(rename_ui::move_part(&mut spec, 0, 3), Some(2));
    assert_eq!(spec.parts[2], NamePart::Text { text: "a".into() });
    assert_eq!(rename_ui::move_part(&mut spec, 2, 0), Some(0));
    assert_eq!(
        rename_ui::move_part(&mut spec, 1, 1),
        None,
        "gleiche Stelle"
    );
    assert_eq!(rename_ui::move_part(&mut spec, 9, 0), None);
    // Einfügeposition aus der Zeigerposition (Fluss-Anordnung in Zeilen)
    let r = |x: f32, y: f32| Rect::from_min_size(Pos2::new(x, y), vec2(60.0, 24.0));
    let rects = [r(0.0, 0.0), r(70.0, 0.0), r(0.0, 30.0)];
    assert_eq!(rename_ui::insert_index(&rects, Pos2::new(10.0, 12.0)), 0);
    assert_eq!(rename_ui::insert_index(&rects, Pos2::new(120.0, 12.0)), 2);
    assert_eq!(rename_ui::insert_index(&rects, Pos2::new(50.0, 42.0)), 3);
    // Tastatur: Fokus auf Chip 0, Strg+→ verschiebt ihn nach rechts
    lang("de");
    let (mut h, _d) = open_default(&["a.png"], Mode::Beginner);
    studio(&mut h).edit("rename", |r| {
        r.rename.parts.push(NamePart::Text { text: "_x".into() });
        true
    });
    h.run_steps(2);
    let chip = h
        .query_by_label(&t("batch-rn-part-original"))
        .map(|n| n.rect())
        .expect("Chip");
    click(&mut h, chip.center());
    key(&mut h, Key::ArrowRight, Modifiers::COMMAND);

    let parts = studio(&mut h).recipe.rename.parts.clone();
    assert_eq!(parts[0], NamePart::Text { text: "_x".into() }, "{parts:?}");
    assert_eq!(table(&mut h).rows[0].new, "_xa.png");
}

#[test]
fn praefix_50x_tippen_ein_verlaufsschritt() {
    lang("de");
    let (mut h, _d) = open_default(&["foto.png", "bild.png"], Mode::Beginner);
    let n0 = studio(&mut h).undo_len();
    let field = h
        .query_by_label(&t("batch-rn-prefix"))
        .map(|n| n.rect())
        .expect("Feld Präfix");
    click(&mut h, field.center());
    for k in 0..50 {
        h.event(Event::Text(((b'a' + (k % 26) as u8) as char).to_string()));
        h.run_steps(1);
    }
    h.run_steps(2);
    let expect: String = (0..50).map(|k| (b'a' + (k % 26) as u8) as char).collect();
    assert_eq!(studio(&mut h).recipe.rename.prefix, expect);
    // Fokus weg (Esc) → Geste beendet: genau ein Rückgängig-Schritt
    key(&mut h, Key::Escape, Modifiers::NONE);
    h.run_steps(2);
    assert_eq!(
        studio(&mut h).undo_len(),
        n0 + 1,
        "50 Tasten = ein Verlaufsschritt"
    );
    assert_eq!(
        table(&mut h).rows[0].new,
        format!("{expect}foto.png"),
        "Endwert, kein Aufsummieren"
    );
    assert!(studio(&mut h).undo());
    assert_eq!(studio(&mut h).recipe.rename.prefix, "");
    assert!(studio(&mut h).redo());
    assert_eq!(studio(&mut h).recipe.rename.prefix, expect);
}

#[test]
fn vorlage_50x_anwenden_ohne_duplikate() {
    lang("de");
    let (mut h, _d) = open_default(&["a.png", "b.png"], Mode::Beginner);
    for _ in 0..50 {
        let p = find(&h, &t("batch-rn-presets"))
            .expect("Vorlagen")
            .rect
            .center();
        click(&mut h, p);
        let it = find(&h, &t("batch-rn-preset-exif-date"))
            .expect("Vorlage nach Datum")
            .rect
            .center();
        click(&mut h, it);
    }
    let s = studio(&mut h);
    assert_eq!(
        s.recipe.rename.parts.len(),
        3,
        "Vorlage ersetzt die Bausteine statt anzuhängen"
    );
    assert!(matches!(s.recipe.rename.parts[0], NamePart::Date { .. }));
    let tb = table(&mut h);
    assert!(tb.rows[1].new.ends_with("_002.png"), "{}", tb.rows[1].new);
    assert!(
        tb.rows[0].hints.contains(&NameHint::ExifDateMissing),
        "ⓘ EXIF-Datum fehlt"
    );
}

// ------------------------------------------------------------------ Regex (Profi)

#[test]
fn umschalten_uebersetzt_baukasten_gleichwertig() {
    lang("de");
    let (mut h, _d) = open_default(&["IMG_0001.png", "IMG_0002.png", "Strand 7.png"], Mode::Pro);
    studio(&mut h).edit("rename", |r| {
        rename_ui::preset_number(&mut r.rename);
        r.rename.prefix = "x-".into();
        r.rename.case = fp_batch::recipe::CaseMode::Lower;
        true
    });
    studio(&mut h).end_gesture();
    h.run_steps(2);
    let before: Vec<String> = table(&mut h).rows.iter().map(|r| r.new.clone()).collect();
    assert_eq!(before[0], "x-bild_001.png");
    let seg = find(&h, &t("batch-ui-rename-mode-regex"))
        .expect("Umschalter Regex")
        .rect
        .center();
    click(&mut h, seg);
    let spec = studio(&mut h).recipe.rename.clone();
    assert_eq!(spec.mode, RenameMode::Regex);
    assert_eq!(spec.regex.find, "^(.*)$");
    assert_eq!(spec.regex.replace, "Bild_{n:3}");
    let after: Vec<String> = table(&mut h).rows.iter().map(|r| r.new.clone()).collect();
    assert_eq!(before, after, "gleichwertiges Muster");
    // Erklärung sichtbar
    assert!(find_containing(&h, "Bedeutung: der ganze Name").is_some());
    assert!(find_containing(&h, "Neuer Name: „Bild_“ + Nummer (3 Stellen)").is_some());
    // 50× hin und her: nie doppelt, eigene Muster bleiben
    for _ in 0..25 {
        let b = find(&h, &t("batch-ui-rename-mode-builder"))
            .expect("Baukasten")
            .rect
            .center();
        click(&mut h, b);
        let r = find(&h, &t("batch-ui-rename-mode-regex"))
            .expect("Regex")
            .rect
            .center();
        click(&mut h, r);
    }
    let s2 = studio(&mut h).recipe.rename.clone();
    assert_eq!(s2.regex, spec.regex);
    assert_eq!(s2.parts.len(), 3);
    studio(&mut h).edit("rename", |r| {
        r.rename.regex.find = r"^IMG_(\d+)".into();
        r.rename.regex.replace = "Foto-$1".into();
        true
    });
    let b = find(&h, &t("batch-ui-rename-mode-builder"))
        .expect("Baukasten")
        .rect
        .center();
    click(&mut h, b);
    let r = find(&h, &t("batch-ui-rename-mode-regex"))
        .expect("Regex")
        .rect
        .center();
    click(&mut h, r);
    assert_eq!(
        studio(&mut h).recipe.rename.regex.find,
        r"^IMG_(\d+)",
        "eigenes Muster geht nicht verloren"
    );
    assert!(
        find_containing(
            &h,
            "am Anfang „IMG_“, dann eine oder mehrere Ziffern (Gruppe 1)"
        )
        .is_some()
    );
    assert_eq!(table(&mut h).rows[0].new, "x-foto-0001.png");
    assert!(table(&mut h).rows[2].unchanged || table(&mut h).rows[2].new == "x-strand 7.png");
}

#[test]
fn ungueltiges_regex_meldung_und_gesperrt() {
    lang("de");
    let (mut h, _d) = open_default(&["a.png", "b.png"], Mode::Pro);
    studio(&mut h).edit("rename", |r| {
        r.rename.mode = RenameMode::Regex;
        r.rename.regex.find = "(abc".into();
        true
    });
    h.run_steps(3);
    assert!(
        find_containing(&h, "Fehler an Stelle 1: Klammer „(“ wird nicht geschlossen").is_some(),
        "verständliche Meldung"
    );
    let tb = table(&mut h);
    assert!(tb.error.is_some());
    assert_eq!(tb.rows[0].old, "a.png", "alte Namen bleiben sichtbar");
    assert!(
        batch::output_bar::blocked(studio(&mut h)).is_some(),
        "Verarbeiten gesperrt"
    );
    // Spickzettel: Einfügen macht das Muster gültig
    let ch = find(&h, &t("batch-rn-cheat"))
        .expect("Spickzettel")
        .rect
        .center();
    click(&mut h, ch);
    h.run_steps(12);
    let k = fp_batch::explain::CHEAT_SHEET
        .iter()
        .position(|e| e.key == "batch-rn-cheat-append-number")
        .expect("Eintrag");
    let ins: Vec<Rect> = h
        .query_all_by_label(&t("batch-rn-cheat-insert"))
        .map(|n| n.rect())
        // below the cheat sheet toggle: the toolbar's Paste button has the same name ("Einfügen")
        .filter(|r| r.top() > ch.y)
        .collect();
    assert_eq!(ins.len(), 8, "8 Beispiele im Spickzettel");
    click(&mut h, ins[k].center());
    let spec = studio(&mut h).recipe.rename.clone();
    assert_eq!(spec.regex.replace, "${1}_{n:3}");
    assert!(batch::output_bar::blocked(studio(&mut h)).is_none());
    assert_eq!(table(&mut h).rows[1].new, "b_002.png");
    assert_clean(&h, "Spickzettel offen");
    // Anfänger-Modus mit Regex-Rezept: Hinweis + Wechsel zum Baukasten
    studio(&mut h).mode = Mode::Beginner;
    h.run_steps(3);
    assert!(h.query_by_label(&t("batch-rn-to-builder")).is_some());
}

// ------------------------------------------------------------------ Konflikte, leere Namen, Unicode

#[test]
fn konflikte_markiert_und_nummer_anhaengen() {
    lang("de");
    let (mut h, _d) = open_default(&["a.png", "b.png", "c.png"], Mode::Beginner);
    studio(&mut h).edit("rename", |r| {
        r.rename.parts = vec![NamePart::Text {
            text: "foto".into(),
        }];
        true
    });
    studio(&mut h).end_gesture();
    h.run_steps(3);
    let tb = table(&mut h);
    assert_eq!(tb.conflicts, 2);
    assert_eq!(
        tb.rows[1].conflict_with,
        Some(1),
        "gleicher Name wie Zeile 1"
    );
    assert_eq!(tb.rows[1].new, "foto (2).png");
    assert!(tb.rows[1].hints.contains(&NameHint::Conflict));
    assert!(find_containing(&h, "2 Konflikte").is_some());
    let fix = h
        .query_by_label(&t("batch-rn-add-counter-fix"))
        .map(|n| n.rect())
        .expect("Nummer anhängen");
    click(&mut h, fix.center());
    let tb = table(&mut h);
    assert_eq!(tb.conflicts, 0);
    assert_eq!(tb.rows[2].new, "foto_003.png");
}

#[test]
fn leere_eingaben_und_ungueltige_zeichen() {
    lang("de");
    // keine Dateien: Hinweis statt Tabelle
    let (h, _d) = open_default(&[], Mode::Beginner);
    assert!(find(&h, &t("batch-rn-table-empty")).is_some());
    assert_clean(&h, "leer");
    // keine Bausteine → leerer Name „_“; ungültige Zeichen → „_“
    let (mut h, _d) = open_default(&["a.png", "b.png"], Mode::Beginner);
    studio(&mut h).edit("rename", |r| {
        r.rename.parts.clear();
        true
    });
    h.run_steps(3);
    assert!(find(&h, &t("batch-rn-no-parts")).is_some());
    let tb = table(&mut h);
    assert_eq!(tb.rows[0].new, "_.png");
    assert!(tb.rows[0].hints.contains(&NameHint::EmptyName));
    studio(&mut h).edit("rename2", |r| {
        r.rename.parts = vec![
            NamePart::Original {
                take: fp_batch::recipe::OriginalTake::Whole,
                count: 0,
            },
            NamePart::Text {
                text: "<a|b>?".into(),
            },
        ];
        true
    });
    h.run_steps(3);
    let tb = table(&mut h);
    assert_eq!(tb.rows[0].new, "a_a_b__.png");
    assert!(tb.rows[0].hints.contains(&NameHint::InvalidChars));
    assert_clean(&h, "ungültige Zeichen");
}

#[test]
fn unicode_lange_namen_drei_designs_nichts_abgeschnitten() {
    let long = format!("{}.png", "sehr_langer_dateiname_".repeat(9));
    let names = [
        "🌸 Blüte groß.png",
        "ÄÖÜ äöü ß.png",
        long.as_str(),
        "IMG_0042.png",
    ];
    // qps = Testsprache mit verlängerten Texten (nichts darf überlaufen)
    for code in ["de", "en", "qps"] {
        lang(code);
        for kind in ThemeKind::ALL {
            for (w, hgt, ppp) in [(1024.0, 768.0, 1.0), (1920.0, 1080.0, 1.5)] {
                for mode in [Mode::Beginner, Mode::Pro] {
                    let (mut h, _d) = open(&names, kind, mode, w, hgt, ppp);
                    studio(&mut h).edit("rename", |r| {
                        r.rename.transliterate = true;
                        r.rename.spaces = Some("_".into());
                        r.rename.suffix = "_neu".into();
                        r.rename.parts.push(NamePart::Counter {
                            start: 1,
                            step: 1,
                            digits: 3,
                            per_folder: false,
                        });
                        if mode == Mode::Pro {
                            r.rename.mode = RenameMode::Regex;
                            r.rename.regex.find = r"^(\w+?)_?(\d+)$".into();
                            r.rename.regex.replace = "$2-$1".into();
                        }
                        true
                    });
                    rename_ui::set_selected(&h.ctx, Some(1));
                    h.run_steps(4);
                    let label = format!("{code}/{}/{w}x{hgt}@{ppp}/{mode:?}", kind.label());
                    assert_clean(&h, &label);
                    // Profi: Spickzettel aufklappen (falls schon sichtbar), dann mitprüfen
                    if mode == Mode::Pro
                        && let Some(ch) = find(&h, &t("batch-rn-cheat"))
                        && h.ctx.content_rect().contains(ch.rect.center())
                    {
                        click(&mut h, ch.rect.center());
                        h.run_steps(12);
                        assert_clean(&h, &format!("{label} Spickzettel"));
                    }
                    // ganze Karte prüfen: Rezept-Spalte schrittweise bis zur Live-Tabelle rollen
                    let at = find(&h, &t("batch-ui-end-rename"))
                        .map(|c| c.rect.center())
                        .unwrap_or_else(|| h.ctx.content_rect().center());
                    for k in 0..4 {
                        h.event(Event::PointerMoved(at));
                        h.event(Event::MouseWheel {
                            unit: egui::MouseWheelUnit::Point,
                            delta: vec2(0.0, -260.0),
                            modifiers: Modifiers::NONE,
                            phase: egui::TouchPhase::Move,
                        });
                        h.run_steps(6);
                        assert_clean(&h, &format!("{label} gerollt {k}"));
                    }
                    let tb = table(&mut h);
                    assert_eq!(tb.rows.len(), 4, "{label}");
                    if mode == Mode::Beginner {
                        assert!(
                            tb.rows[0].new.contains("Bluete_gross"),
                            "{label}: {}",
                            tb.rows[0].new
                        );
                        assert!(
                            tb.rows[2].hints.contains(&NameHint::Truncated)
                                || tb.rows[2].new.chars().count() <= 255,
                            "{label}"
                        );
                    }
                }
            }
        }
    }
    lang("de");
}

// ------------------------------------------------------------------ Bilder

fn save_png(path: &Path, img: &image::RgbaImage) {
    let f = std::fs::File::create(path).expect("Datei");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), img.width(), img.height());
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.set_compression(png::Compression::High);
    enc.write_header()
        .and_then(|mut wr| wr.write_image_data(img.as_raw()))
        .expect("PNG");
}

/// Bilder: 3 Designs × de/en, Baukasten (Anfänger) und Regex (Profi), 1440 × 1080; die Rezept-Spalte ist\n/// so weit gerollt, dass Bausteine bzw. Muster und die Live-Tabelle zu sehen sind.
#[test]
#[ignore]
fn batch3_bilder() {
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("docs")
        .join("screenshots");
    let _ = std::fs::create_dir_all(&out);
    let names = [
        "IMG_0815.png",
        "IMG_0816.png",
        "Strand am Meer.png",
        "Logo Entwurf (Kopie).png",
        "Größe ändern.png",
        "IMG_0815.jpg.png",
    ];
    for code in ["de", "en"] {
        lang(code);
        for (kind, dname) in [
            (ThemeKind::Normal, "normal"),
            (ThemeKind::Dark, "dunkel"),
            (ThemeKind::Oled, "oled"),
        ] {
            for variant in ["baukasten", "regex"] {
                let mode = if variant == "regex" {
                    Mode::Pro
                } else {
                    Mode::Beginner
                };
                let d = dir(&format!("bild-{code}-{dname}-{variant}"));
                let mut st = state(kind);
                st.run("file.batchProcess", json!({}));
                let paths: Vec<PathBuf> = names
                    .iter()
                    .enumerate()
                    .map(|(i, n)| png_file(&d, n, 320 + 40 * i as u32, 240))
                    .collect();
                let mut h = harness_ex(st, 1440.0, 1080.0, 1.0, true);
                {
                    let s = studio(&mut h);
                    s.mode = mode;
                    s.add_paths(&paths, false);
                    s.selected = Some(0);
                    s.open_card = Some(CardId::Rename);
                    s.edit("rename", |r| {
                        r.output.format = "webp".into();
                        if variant == "regex" {
                            r.rename.mode = RenameMode::Regex;
                            r.rename.regex.find = r"^IMG_(\d+)".into();
                            r.rename.regex.replace = "Foto_$1_{n:2}".into();
                            r.rename.case = fp_batch::recipe::CaseMode::Lower;
                        } else {
                            r.rename.parts.push(NamePart::Text { text: "_".into() });
                            r.rename.parts.push(NamePart::Counter {
                                start: 1,
                                step: 1,
                                digits: 3,
                                per_folder: false,
                            });
                            r.rename.spaces = Some("_".into());
                            r.rename.transliterate = true;
                        }
                        true
                    });
                    s.end_gesture();
                }
                if variant == "baukasten" {
                    rename_ui::set_selected(&h.ctx, Some(2));
                }
                h.run_steps(6);
                // Rezept-Spalte rollen (Mausrad über der Karte „Dateiname“)
                if let Some(card) = find(&h, &t("batch-ui-end-rename")) {
                    let at = card.rect.center() + vec2(120.0, 60.0);
                    h.event(Event::PointerMoved(at));
                    h.event(Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: vec2(0.0, if variant == "regex" { -330.0 } else { -300.0 }),
                        modifiers: Modifiers::NONE,
                        phase: egui::TouchPhase::Move,
                    });
                    h.run_steps(8);
                }
                for x in texts(&h) {
                    assert!(
                        fp_neutral::find_in_bytes(x.text.as_bytes()).is_empty(),
                        "gesperrter Name im Bild: {}",
                        x.text
                    );
                }
                let Ok(img) = h.render() else {
                    eprintln!("kein Grafikadapter");
                    return;
                };
                let p = out.join(format!("batch3_{variant}_{dname}_{code}.png"));
                save_png(&p, &img);
                let bytes = std::fs::read(&p).expect("PNG lesen");
                assert!(
                    fp_neutral::find_in_bytes(&bytes).is_empty(),
                    "gesperrter Name in {}",
                    p.display()
                );
            }
        }
    }
    lang("de");
}
