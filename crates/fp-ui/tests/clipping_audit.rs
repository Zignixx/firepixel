//! RAND-1 (Nutzerbefund N23): automatischer Prüfer für abgeschnittene Texte und zu knappe Ränder.
//!
//! **Was geöffnet wird** (je Variante in einer eigenen Oberfläche mit dem Testdokument der
//! Gesamttests, alle Objektarten): das Hauptfenster, jedes Bedienfeld angedockt, im schmalen Dock
//! und – wo es schwebend startet – schwebend; der Inspektor für jede Objektart und jedes Werkzeug
//! (dazu „Symboleigenschaften“); jeder Dialog, den ein Befehl des Registers öffnet (Auswahl wie in
//! den Gesamttests), dazu Neues Dokument, alle Seiten der Voreinstellungen, Über, Arbeitsbereich
//! speichern, Ungespeichert-Abfrage, Rückfrage, Meldung und Warnung; zuletzt die Startseite.
//!
//! **Varianten:** 3 Designs × de/en × Skalierung 100/150 % × Fenster 1024 × 768 und 1920 × 1080
//! (Bildpunkte des Bildschirms; kleiner als 800 × 500 Punkte lässt Firepixel das Fenster nicht
//! werden, so groß wird dann gemessen).
//!
//! **Wie gemessen wird:** Nach dem Zeichnen werden im selben Durchlauf die Zeichenlisten je Ebene
//! (`Context::graphics`, mit Zeichenreihenfolge und Clip-Bereich), die Widget-Rechtecke des
//! Durchlaufs und der Barrierefreiheitsbaum (Rolle, Name, Rechteck – wie `ui_find` in der Brücke)
//! ausgewertet. Gemeldet wird:
//!
//! | Art | Bedeutung |
//! |---|---|
//! | `abgeschnitten` | Text ragt über seinen Clip-Bereich bzw. das Fenster (waagerecht, oder senkrecht in einem Bereich, der kaum höher ist als der Text) |
//! | `verdeckt` | ein später in derselben Ebene gezeichnetes, deckendes Rechteck liegt über dem Text (so sah N23 aus: die Farbtabelle über „Zeilensprung/Progressiv“) |
//! | `kaestchen` | Kontrollkästchen bzw. Optionsfeld nicht ganz sichtbar (Clip, Fenster oder verdeckt) |
//! | `ueberlappt-text` | zwei Texte derselben Ebene überlappen |
//! | `ueberlappt-element` | ein Text läuft in ein fremdes Bedienelement (Knopf, Feld, Liste, Kästchen …) |
//! | `ragt-heraus` | ein Text ist breiter bzw. höher als sein eigenes Bedienelement |
//! | `rand-knapp` | weniger als `FP_RAND_GAP_X` (2 px) waagerecht bzw. `FP_RAND_GAP_Y` (1 px) senkrecht zwischen Text und Rahmen eines Knopfs/Felds/Klappfelds, bzw. weniger als `FP_RAND_GAP_EDGE` (2 px) zum linken/rechten Rand von Dialog, Bedienfeld oder Inspektor |
//! | `dialog-ragt-hinaus` | ein Dialog bzw. ein schwebendes Fenster ragt über das Hauptfenster |
//! | `hinweis-bildlauf` | (nur Hinweis) Text am oberen/unteren Rand eines hohen Bereichs angeschnitten – Bildlauf bzw. zu wenig Dockhöhe, gewollt; ebenso an der linken/rechten Kante des waagerechten Bildlaufs im schmalen Inspektor (`inspector::hscroll_viewport`, RAND-2a) |
//! | `hinweis-feldinhalt` | (nur Hinweis) Inhalt eines Eingabefelds länger als das Feld (scrollt) |
//!
//! Warum senkrecht nur 1 px: Die kompakten Felder des Originals sind 13 DLU (21 px) hoch und haben
//! bei 8-pt-Schrift oben und unten nur 2–3 px Luft; in den Leisten (Fußleiste, Inspektor-Kopf)
//! ist das gewollt. Waagerecht verlangt Windows mindestens 2 px (Rahmen + Innenabstand), darunter
//! wirkt der Text „angeklebt“. Alle Grenzwerte lassen sich über die Umgebung einstellen.
//!
//! **Ausgabe:** `docs\funktionstest\rand_auto.md` (Fundstelle mit Ansicht, Variante, Text,
//! Rechteck, Art; Zahlen je Bereich) und Ausschnitte der schlimmsten Funde unter
//! `docs\screenshots\rand\` (nur mit Grafikadapter; Ausschnitte mit gesperrten Namen, z. B.
//! Systemschriften, werden nicht gespeichert, AGENTEN_REGELN §1a).
//!
//! **Erlaubnisliste:** `clipping_audit_allow.txt` enthält die bekannten Funde (`Art|Ansicht|Text`,
//! unabhängig von der Variante). Der Test schlägt nur fehl, wenn ein Fund dazukommt, der nicht in
//! der Liste steht (Hinweise zählen nie). Behobene Einträge nennt der Bericht unter „nicht mehr
//! gefunden“; RAND-2 streicht sie. `FP_RAND_FULL=1` + `FP_RAND_UPDATE_ALLOW=1` schreibt die Liste neu.
//!
//! **Umfang:** Standard (auch in `cargo test --workspace`) ist der schnelle Lauf Normal/de+en/
//! 100+150 %/1024 × 768 gegen die Erlaubnisliste. `FP_RAND_FULL=1` prüft die volle Matrix
//! (24 Varianten) und schreibt Bericht und Bilder; `FP_RAND_VIEWS=<Teiltext>` nur passende
//! Ansichten, `FP_RAND_THREADS` (4) Varianten gleichzeitig, `FP_RAND_SHOTS` (12) Ausschnittsbilder.
//!
//! Die Sprache ist prozessweit: erst alle deutschen, dann alle englischen Varianten; eigene
//! Test-Binärdatei.

use crate::gesamt_common;

use egui::accesskit::Role;
use egui::{pos2, vec2, Id, LayerId, Order, Pos2, Rect, Shape};
use egui_kittest::Harness;
use fp_ui::app_state::{Action, AppState, UiSettings};
use fp_ui::panels::PanelId;
use fp_ui::theme::ThemeKind;
use fp_ui::workspace::Workspace;
use fp_ui::FirepixelApp;
use gesamt_common::{apply_candidate, available, candidates, fixture, Candidate, Fix};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

// ------------------------------------------------------------------ Einstellungen

#[derive(Clone, Copy, Debug)]
struct Cfg {
    /// Mindestabstand Text ↔ Rahmen waagerecht (Knöpfe, Felder, Klappfelder).
    gap_x: f32,
    /// Mindestabstand Text ↔ Rahmen senkrecht.
    gap_y: f32,
    /// Mindestabstand Text ↔ linker/rechter Rand von Dialog, Bedienfeld, Inspektor.
    gap_edge: f32,
}

fn env_f(name: &str, d: f32) -> f32 {
    std::env::var(name).ok().and_then(|v| v.replace(',', ".").parse().ok()).unwrap_or(d)
}

impl Cfg {
    fn from_env() -> Cfg {
        Cfg { gap_x: env_f("FP_RAND_GAP_X", 2.0), gap_y: env_f("FP_RAND_GAP_Y", 1.0), gap_edge: env_f("FP_RAND_GAP_EDGE", 2.0) }
    }
}

#[derive(Clone, Copy, Debug)]
struct Var {
    theme: ThemeKind,
    lang: &'static str,
    ppp: f32,
    win: (f32, f32),
}

impl Var {
    fn label(&self) -> String {
        format!("{}/{}/{:.0} %/{}×{}", self.theme.label(), self.lang, self.ppp * 100.0, self.win.0, self.win.1)
    }
    /// Fenstergröße in Punkten (nie kleiner als die Mindestgröße des Hauptfensters).
    fn logical(&self) -> egui::Vec2 {
        vec2((self.win.0 / self.ppp).max(800.0).round(), (self.win.1 / self.ppp).max(500.0).round())
    }
}

fn variants(lang: &'static str) -> Vec<Var> {
    let quick = std::env::var_os("FP_RAND_FULL").is_none();
    let mut v = Vec::new();
    for theme in ThemeKind::ALL {
        if quick && theme != ThemeKind::Normal {
            continue;
        }
        for ppp in [1.0f32, 1.5] {
            for win in [(1024.0f32, 768.0f32), (1920.0, 1080.0)] {
                if quick && win.0 > 1024.0 {
                    continue;
                }
                v.push(Var { theme, lang, ppp, win });
            }
        }
    }
    v
}

// ------------------------------------------------------------------ Ansichten

#[derive(Clone, Debug, PartialEq)]
enum PanelMode {
    Dock,
    Narrow,
    Floating,
}

#[derive(Clone, Debug)]
enum View {
    Main,
    /// Bedienfeld; beim Optimieren-Bedienfeld optional ein anderes Exportformat (der Aufbau
    /// hängt vom Format ab, N23 zeigte sich bei JPEG).
    Panel(PanelId, PanelMode, Option<&'static str>),
    InspectorSel(usize),
    InspectorTool(fp_engine::ToolId),
    InspectorSymbol,
    Dialog { cmd: String, cand: usize },
    Extra(&'static str, usize),
    Start,
}

impl View {
    fn area(&self) -> &'static str {
        match self {
            View::Main => "Hauptfenster",
            View::Panel(..) => "Bedienfelder",
            View::InspectorSel(_) | View::InspectorTool(_) | View::InspectorSymbol => "Inspektor",
            View::Dialog { .. } | View::Extra(..) => "Dialoge",
            View::Start => "Startseite",
        }
    }
    /// Stabiler Name (Schlüssel der Erlaubnisliste, unabhängig von der Sprache).
    fn name(&self, cands: &[Candidate]) -> String {
        match self {
            View::Main => "Hauptfenster".into(),
            View::Panel(p, m, fmt) => format!("Bedienfeld {p:?}{} ({})", fmt.map(|f| format!(" {f}")).unwrap_or_default(), match m {
                PanelMode::Dock => "Dock",
                PanelMode::Narrow => "schmales Dock",
                PanelMode::Floating => "schwebend",
            }),
            View::InspectorSel(i) => format!("Inspektor {}", cands[*i].label),
            View::InspectorTool(t) => format!("Inspektor Werkzeug {}", fp_engine::tools::info(*t).map(|i| i.name).unwrap_or("?")),
            View::InspectorSymbol => "Inspektor Symboleigenschaften".into(),
            View::Dialog { cmd, .. } => format!("Dialog {cmd}"),
            View::Extra(k, i) => if *i == 0 { format!("Dialog {k}") } else { format!("Dialog {k} {i}") },
            View::Start => "Startseite".into(),
        }
    }
}

/// Seiten der Voreinstellungen (`dialogs\preferences.rs`, `CATEGORIES`).
const PREF_PAGES: usize = 7;

const EXTRA: &[(&str, usize)] =&[("Neues Dokument", 1), ("Voreinstellungen", PREF_PAGES), ("Über Firepixel", 1), ("Arbeitsbereich speichern", 1), ("Ungespeichert-Abfrage", 1), ("Rückfrage Zurücksetzen", 1), ("Meldung", 1), ("Warnung", 1)];

fn close_dialogs(st: &mut AppState) {
    let ps = std::mem::take(&mut st.dialogs.panel_sel);
    st.dialogs = Default::default();
    st.dialogs.panel_sel = ps;
}

/// Befehle, deren Dialog das Register öffnet, mit der ersten passenden Testsituation.
fn discover_dialogs(cands: &[Candidate]) -> Vec<(String, usize)> {
    let (mut st, _ctx, _f) = fixture();
    let mut names: Vec<String> = st.session.registry().iter().map(|c| c.info().name.clone()).collect();
    names.sort();
    names.dedup();
    let mut out = Vec::new();
    for n in names {
        if gesamt_common::external(&n) {
            continue;
        }
        for (i, c) in cands.iter().enumerate() {
            close_dialogs(&mut st);
            apply_candidate(&mut st, c);
            if !available(&st, &n) {
                continue;
            }
            let opened = fp_ui::dialogs::open_for_command(&mut st, &n) && (st.dialogs.any_open() || st.dialogs.pending_form.is_some());
            close_dialogs(&mut st);
            if opened {
                out.push((n.clone(), i));
            }
            break;
        }
    }
    close_dialogs(&mut st);
    out
}

fn all_views(cands: &[Candidate], dialogs: &[(String, usize)]) -> Vec<View> {
    let mut v = vec![View::Main];
    let floating = Workspace::expanded().closed;
    for &p in PanelId::ALL {
        if p == PanelId::SymbolProperties {
            continue;
        }
        v.push(View::Panel(p, PanelMode::Dock, None));
        v.push(View::Panel(p, PanelMode::Narrow, None));
        if floating.contains(&p) {
            v.push(View::Panel(p, PanelMode::Floating, None));
        }
        if p == PanelId::Optimize {
            for (f, _, _) in OPT_FORMATS.iter().skip(1) {
                v.push(View::Panel(p, PanelMode::Dock, Some(f)));
                v.push(View::Panel(p, PanelMode::Narrow, Some(f)));
            }
        }
    }
    for i in 0..cands.len() {
        v.push(View::InspectorSel(i));
    }
    for t in fp_engine::tools::TOOLS {
        v.push(View::InspectorTool(t.id));
    }
    v.push(View::InspectorSymbol);
    for (cmd, cand) in dialogs {
        v.push(View::Dialog { cmd: cmd.clone(), cand: *cand });
    }
    for (k, n) in EXTRA {
        for i in 0..*n {
            v.push(View::Extra(k, i));
        }
    }
    v.push(View::Start);
    if let Ok(f) = std::env::var("FP_RAND_VIEWS") {
        let f = f.to_lowercase();
        v.retain(|x| x.name(cands).to_lowercase().contains(&f));
    }
    v
}

/// Exportformate für das Optimieren-Bedienfeld (Name, Format, Farbmodus); das erste ist die Vorgabe.
const OPT_FORMATS: &[(&str, fp_export::options::ExportFormat, fp_export::options::ColorMode)] = &[
    ("GIF", fp_export::options::ExportFormat::Gif, fp_export::options::ColorMode::Indexed),
    ("JPEG", fp_export::options::ExportFormat::Jpeg, fp_export::options::ColorMode::Rgb24),
    ("PNG32", fp_export::options::ExportFormat::Png, fp_export::options::ColorMode::Rgba32),
    ("AVIF", fp_export::options::ExportFormat::Avif, fp_export::options::ColorMode::Rgba32),
];

fn set_format(st: &mut AppState, name: &str) {
    let Some(&(_, f, m)) = OPT_FORMATS.iter().find(|x| x.0 == name) else { return };
    let Some(d) = st.session.active() else { return };
    let mut s = fp_export::doc_export::page_settings(&d.doc, d.page);
    if s.format != f || s.color_mode != m {
        s.format = f;
        s.color_mode = m;
        fp_ui::panels::optimize::apply_settings(st, &s);
    }
}

/// Testsituationen ohne Vorbereitungsbefehle (die das Dokument verändern würden).
fn plain_candidates(f: &Fix) -> Vec<Candidate> {
    candidates(f).into_iter().filter(|c| c.prep.is_empty() && c.label != "importiertes Symbol").collect()
}

/// Stellt eine Ansicht ein (vor dem ersten Bild).
fn setup(st: &mut AppState, view: &View, cands: &[Candidate], extra_collapse: usize) {
    close_dialogs(st);
    st.settings.workspace = Workspace::expanded();
    st.flyout = None;
    st.push(Action::SetTool(fp_engine::ToolId::Pointer));
    let none = &cands[0];
    // Exportformat (Optimieren): nur die Formatansichten weichen von GIF ab
    let fmt = match view {
        View::Panel(_, _, Some(f)) => f,
        _ => OPT_FORMATS[0].0,
    };
    set_format(st, fmt);
    match view {
        View::Main => apply_candidate(st, &cands[1.min(cands.len() - 1)]),
        View::Panel(p, mode, _) => {
            apply_candidate(st, &cands[1.min(cands.len() - 1)]);
            let ws = &mut st.settings.workspace;
            if *mode != PanelMode::Floating {
                // Standardanordnung wie beim Nutzer (N23 zeigte sich nur, weil die Gruppe mit den
                // anderen aufgeklappten Gruppen wenig Höhe hat); schwebende Bedienfelder angedockt
                if ws.locate(*p).is_some_and(|l| !matches!(l, fp_ui::workspace::Location::Dock { .. })) {
                    ws.move_to_dock_group(*p, 0, 0);
                }
                ws.show(*p);
                // Klappt das Dock die Gruppe mangels Höhe nur für die Anzeige ein (`show_dock`),
                // so viele andere Gruppen von unten einklappen, bis sie zu sehen ist (wie der
                // Nutzer, der Platz schafft)
                let own = match ws.locate(*p) {
                    Some(fp_ui::workspace::Location::Dock { group, .. }) => group,
                    _ => usize::MAX,
                };
                let mut k = extra_collapse;
                for (i, g) in ws.dock.iter_mut().enumerate().rev() {
                    if k == 0 {
                        break;
                    }
                    if i != own && !g.collapsed {
                        g.collapsed = true;
                        k -= 1;
                    }
                }
                if *mode == PanelMode::Narrow {
                    ws.dock_width = 120.0;
                }
            } else {
                ws.show(*p);
            }
            st.sync_panels_to_engine();
        }
        View::InspectorSel(i) => apply_candidate(st, &cands[*i]),
        View::InspectorTool(t) => {
            apply_candidate(st, none);
            st.push(Action::SetTool(*t));
        }
        View::InspectorSymbol => {
            apply_candidate(st, none);
            st.settings.workspace.show(PanelId::SymbolProperties);
        }
        View::Dialog { cmd, cand } => {
            apply_candidate(st, &cands[*cand]);
            let _ = fp_ui::dialogs::open_for_command(st, cmd);
        }
        View::Extra(k, i) => {
            apply_candidate(st, &cands[1.min(cands.len() - 1)]);
            match *k {
                "Neues Dokument" => st.run("file.new", json!({})),
                "Voreinstellungen" => {
                    let mut d = fp_ui::dialogs::PreferencesDialog::new(st);
                    d.category = *i;
                    st.dialogs.preferences = Some(d);
                }
                "Über Firepixel" => st.dialogs.about = true,
                "Arbeitsbereich speichern" => st.dialogs.save_workspace = Some(String::new()),
                "Ungespeichert-Abfrage" => {
                    if let Some(id) = st.session.active_id() {
                        st.dialogs.unsaved = Some(fp_ui::dialogs::UnsavedFlow::new(vec![id], fp_ui::dialogs::PendingClose::CloseDocs));
                    }
                }
                "Rückfrage Zurücksetzen" => st.run("file.revert", json!({})),
                "Meldung" => st.dialogs.message = Some(("Firepixel".into(), fp_i18n::t("dlg-popup-need-selection"))),
                "Warnung" => {
                    apply_candidate(st, none);
                    for c in ["modify.alterPath.simplify", "modify.combinePaths.union", "modify.ungroup", "text.attachToPath"] {
                        if fp_ui::dialogs::warning::intercept(st, c) {
                            break;
                        }
                    }
                }
                _ => {}
            }
        }
        View::Start => {
            apply_candidate(st, none);
            while st.session.active_id().is_some() {
                if st.session.execute("file.close", json!({ "discard": true })).is_err() {
                    break;
                }
            }
        }
    }
    let _ = st.session.drain_events();
    st.status = None;
}

// ------------------------------------------------------------------ Aufzeichnung

#[derive(Clone, Debug)]
struct TextRec {
    layer: usize,
    idx: usize,
    text: String,
    rect: Rect,
    clip: Rect,
}

#[derive(Clone, Debug)]
struct BoxRec {
    layer: usize,
    idx: usize,
    rect: Rect,
    clip: Rect,
    opaque: bool,
}

#[derive(Clone, Debug)]
struct WidgetRec {
    layer: usize,
    rect: Rect,
    /// sichtbarer, bedienbarer Teil (Rechteck ∩ Clip)
    interact: Rect,
}

#[derive(Default, Clone, Debug)]
struct Cap {
    layers: Vec<LayerId>,
    texts: Vec<TextRec>,
    boxes: Vec<BoxRec>,
    widgets: HashMap<u64, WidgetRec>,
    areas: Vec<(usize, Rect)>,
}

fn walk(shape: &Shape, clip: Rect, layer: usize, idx: &mut usize, cap: &mut Cap) {
    match shape {
        Shape::Vec(v) => {
            for s in v {
                walk(s, clip, layer, idx, cap);
            }
        }
        Shape::Text(t) => {
            *idx += 1;
            let s = t.galley.text();
            if !s.trim().is_empty() && t.opacity_factor > 0.0 {
                cap.texts.push(TextRec { layer, idx: *idx, text: s.to_string(), rect: t.visual_bounding_rect(), clip });
            }
        }
        Shape::Rect(r) => {
            *idx += 1;
            if r.rect.is_positive() {
                cap.boxes.push(BoxRec { layer, idx: *idx, rect: r.rect, clip, opaque: r.fill.a() >= 230 });
            }
        }
        Shape::Circle(c) => {
            *idx += 1;
            cap.boxes.push(BoxRec { layer, idx: *idx, rect: Rect::from_center_size(c.center, vec2(2.0 * c.radius, 2.0 * c.radius)), clip, opaque: false });
        }
        _ => *idx += 1,
    }
}

/// Liest im laufenden Durchlauf (nach dem Zeichnen) alle Ebenen mit Zeichenreihenfolge.
fn capture(ctx: &egui::Context) -> Cap {
    let mut cap = Cap::default();
    let mut layers: Vec<LayerId> = ctx.memory(|m| m.layer_ids().collect());
    if !layers.contains(&LayerId::background()) {
        layers.insert(0, LayerId::background());
    }
    ctx.graphics(|g| {
        for (li, id) in layers.iter().enumerate() {
            if let Some(pl) = g.get(*id) {
                let mut idx = 0;
                for cs in pl.all_entries() {
                    walk(&cs.shape, cs.clip_rect, li, &mut idx, &mut cap);
                }
            }
        }
    });
    let index: HashMap<LayerId, usize> = layers.iter().enumerate().map(|(i, l)| (*l, i)).collect();
    ctx.viewport(|v| {
        for (l, ws) in v.this_pass.widgets.layers() {
            let Some(li) = index.get(l) else { continue };
            for w in ws {
                cap.widgets.insert(w.id.value(), WidgetRec { layer: *li, rect: w.rect, interact: w.interact_rect });
            }
        }
    });
    // nur Bereiche, die in diesem Bild gezeichnet wurden (der Speicher behält alte Dialoglagen)
    let visible = ctx.memory(|m| m.areas().visible_layer_ids());
    for (i, l) in layers.iter().enumerate() {
        if l.order != Order::Background
            && visible.contains(l)
            && cap.texts.iter().any(|t| t.layer == i) | cap.boxes.iter().any(|b| b.layer == i)
            && let Some(r) = ctx.memory(|m| m.area_rect(l.id))
        {
            cap.areas.push((i, r));
        }
    }
    cap.layers = layers;
    cap
}

struct S {
    app: FirepixelApp,
    want: bool,
    cap: Option<Cap>,
}

type H = Harness<'static, S>;

fn harness(app: FirepixelApp, var: &Var, gpu: bool) -> H {
    let b = Harness::builder().with_size(var.logical()).with_pixels_per_point(var.ppp).with_max_steps(12).with_step_dt(1.0 / 60.0);
    let b = if gpu { b.wgpu() } else { b };
    b.build_ui_state(
        |ui, s: &mut S| {
            s.app.root_ui(ui);
            if s.want {
                s.cap = Some(capture(ui.ctx()));
            }
        },
        S { app, want: false, cap: None },
    )
}

fn app_for(st: AppState, var: &Var) -> FirepixelApp {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, fp_engine::Session::default(), UiSettings::default(), false);
    app.state = st;
    app.state.set_theme(var.theme);
    app
}

#[derive(Clone, Debug)]
struct NodeRec {
    role: Role,
    rect: Rect,
    interact: Rect,
    layer: Option<usize>,
}

/// Barrierefreiheitsbaum (wie `ui_find`): Rolle je Widget, Rechteck und Ebene aus dem Durchlauf.
fn nodes(h: &H, cap: &Cap) -> Vec<NodeRec> {
    use egui_kittest::kittest::{By, NodeT, Queryable};
    let mut out = Vec::new();
    for n in h.query_all(By::new()) {
        let ak = n.accesskit_node();
        let id = ak.locate().0 .0;
        let (rect, interact, layer) = match cap.widgets.get(&id) {
            Some(w) => (w.rect, w.interact, Some(w.layer)),
            None => (n.rect(), n.rect(), None),
        };
        if rect.is_positive() {
            out.push(NodeRec { role: ak.role(), rect, interact, layer });
        }
    }
    out
}

/// Bild für Bild zeichnen, im letzten aufzeichnen.
/// Angedockte Bedienfelder, deren Gruppe das Dock mangels Höhe einklappt, werden mit weiteren
/// eingeklappten Gruppen wiederholt (höchstens viermal).
fn run_view(h: &mut H, view: &View, cands: &[Candidate]) -> (Cap, Vec<NodeRec>, Rect, Roi) {
    let mut extra = 0;
    loop {
        setup(&mut h.state_mut().app.state, view, cands, extra);
        h.run_steps(2);
        h.state_mut().want = true;
        h.step();
        h.state_mut().want = false;
        let cap = h.state_mut().cap.take().unwrap_or_default();
        let ns = nodes(h, &cap);
        let screen = h.ctx.content_rect();
        let (roi, shown) = roi_for(h, view, &cap, screen);
        if shown || extra >= 4 {
            return (cap, ns, screen, roi);
        }
        extra += 1;
    }
}

/// Gruppe des Bedienfelds im Dock: von seiner Reiterleiste (Widget `("dock-group", i)` + „bg“)
/// bis zur nächsten; dazu die Höhe der Reiterleiste.
fn dock_group_rect(st: &AppState, cap: &Cap, p: PanelId, dock: Rect) -> Option<(Rect, f32)> {
    let ws = &st.settings.workspace;
    let Some(fp_ui::workspace::Location::Dock { group, .. }) = ws.locate(p) else { return None };
    let strip = |i: usize| cap.widgets.get(&Id::new(("dock-group", i)).with("bg").value()).map(|w| w.rect);
    let s = strip(group)?;
    let next = (group + 1..ws.dock.len()).filter_map(strip).map(|r| r.top()).next().unwrap_or(dock.bottom());
    Some((Rect::from_min_max(pos2(dock.left(), s.top()), pos2(dock.right(), next)), s.height()))
}

// ------------------------------------------------------------------ Bereich je Ansicht

#[derive(Clone, Debug)]
struct Roi {
    area: Rect,
    exclude: Vec<Rect>,
    /// nur diese Ebenen (Dialoge, schwebende Bedienfelder)
    layers: Option<Vec<usize>>,
    /// Rahmen für den Randabstand (links/rechts)
    frame: Option<Rect>,
    /// reine Zahlen auslassen (Lineale)
    no_numbers: bool,
    /// sichtbarer Ausschnitt eines waagerechten Bildlaufs (Inspektor im schmalen Fenster)
    hscroll: Option<Rect>,
}

fn panel_rect(ctx: &egui::Context, id: &str) -> Option<Rect> {
    egui::containers::panel::PanelState::load(ctx, Id::new(id)).map(|p| p.outer_rect)
}

/// Prüfbereich der Ansicht; `false`, wenn das angedockte Bedienfeld nicht zu sehen ist.
fn roi_for(h: &H, view: &View, cap: &Cap, screen: Rect) -> (Roi, bool) {
    let ctx = &h.ctx;
    let dock = panel_rect(ctx, "fp-dock");
    let insp = panel_rect(ctx, "fp-inspector");
    let all = Roi { area: screen, exclude: vec![], layers: None, frame: None, no_numbers: false, hscroll: None };
    if let View::Panel(p, PanelMode::Dock | PanelMode::Narrow, _) = view {
        let bg = Some(vec![layer_index(cap, LayerId::background())]);
        return match dock.and_then(|d| dock_group_rect(&h.state().app.state, cap, *p, d)) {
            Some((g, strip)) => (Roi { area: g, frame: Some(g), layers: bg, ..all }, g.height() - strip > 40.0),
            None => (Roi { area: dock.unwrap_or(screen), frame: dock, layers: bg, ..all }, false),
        };
    }
    let roi = match view {
        View::Main => Roi { exclude: dock.into_iter().chain(insp).collect(), no_numbers: true, layers: Some(vec![layer_index(cap, LayerId::background())]), ..all },
        View::Panel(_, PanelMode::Floating, _) => {
            let st = &h.state().app.state;
            match st.settings.workspace.floating.iter().rev().find(|f| !f.group.panels.iter().all(|p| st.settings.workspace.closed.contains(p))) {
                Some(f) => {
                    let r = Rect::from_min_size(pos2(f.pos[0], f.pos[1]), vec2(f.size[0], f.size[1]));
                    let layers: Vec<usize> = cap.areas.iter().filter(|(_, a)| a.intersects(r)).map(|(i, _)| *i).collect();
                    let frame = cap.areas.iter().filter(|(_, a)| a.intersects(r)).map(|(_, a)| *a).next();
                    Roi { area: frame.unwrap_or(r).expand(2.0), layers: Some(layers), frame, ..all }
                }
                None => all,
            }
        }
        View::Panel(..) => all.clone(),
        View::InspectorSel(_) | View::InspectorTool(_) | View::InspectorSymbol => Roi { area: insp.unwrap_or(screen), frame: insp, layers: Some(vec![layer_index(cap, LayerId::background())]), hscroll: fp_ui::inspector::hscroll_viewport(ctx), ..all },
        View::Dialog { .. } | View::Extra(..) => {
            // oberster Dialog (Vordergrund) und alles darüber
            let fg: Vec<usize> = cap.areas.iter().filter(|(i, _)| cap.layers[*i].order == Order::Foreground).map(|(i, _)| *i).collect();
            let id = fp_ui::dialogs::last_dialog_id(ctx);
            let rect = id.as_deref().and_then(|i| fp_ui::dialogs::dialog_rect(ctx, i));
            match (fg.is_empty(), rect) {
                (false, Some(r)) => Roi { area: r.union(screen), layers: Some(fg), frame: Some(r), ..all },
                (false, None) => Roi { layers: Some(fg), ..all },
                _ => Roi { layers: Some(vec![usize::MAX]), ..all },
            }
        }
        View::Start => {
            // Mitte: rechts der Werkzeuge, links vom Dock, über dem Inspektor
            let tools = panel_rect(ctx, "fp-tools");
            let top = panel_rect(ctx, "fp-toolbar").or(panel_rect(ctx, "fp-appbar")).map(|r| r.bottom()).unwrap_or(screen.top());
            let area = Rect::from_min_max(pos2(tools.map(|r| r.right()).unwrap_or(screen.left()), top), pos2(dock.map(|r| r.left()).unwrap_or(screen.right()), insp.map(|r| r.top()).unwrap_or(screen.bottom())));
            Roi { area, layers: Some(vec![layer_index(cap, LayerId::background())]), ..all }
        }
    };
    (roi, true)
}

fn layer_index(cap: &Cap, l: LayerId) -> usize {
    cap.layers.iter().position(|x| *x == l).unwrap_or(usize::MAX)
}

// ------------------------------------------------------------------ Auswertung

#[derive(Clone, Debug)]
struct Finding {
    kind: &'static str,
    text: String,
    rect: Rect,
    /// Schwere 0..1 (Anteil, der fehlt bzw. überdeckt ist)
    score: f32,
    detail: String,
}

fn hint(kind: &str) -> bool {
    kind.starts_with("hinweis")
}

fn weight(kind: &str) -> f32 {
    match kind {
        "kaestchen" => 6.0,
        "verdeckt" => 5.0,
        "abgeschnitten" => 4.0,
        "dialog-ragt-hinaus" => 3.5,
        "ueberlappt-text" | "ueberlappt-element" => 3.0,
        "ragt-heraus" => 2.5,
        "rand-knapp" => 1.0,
        _ => 0.0,
    }
}

fn r1(r: Rect) -> String {
    format!("[{:.0}, {:.0} – {:.0}, {:.0}]", r.min.x, r.min.y, r.max.x, r.max.y)
}

fn is_number(s: &str) -> bool {
    let t = s.trim();
    !t.is_empty() && t.chars().all(|c| c.is_ascii_digit() || matches!(c, '-' | '−' | '.' | ','))
}

fn framed(r: Role) -> bool {
    matches!(r, Role::Button | Role::TextInput | Role::MultilineTextInput | Role::ComboBox | Role::SpinButton | Role::SearchInput)
}

fn control(r: Role) -> bool {
    framed(r) || matches!(r, Role::CheckBox | Role::RadioButton | Role::Slider | Role::Switch)
}

fn analyze(cap: &Cap, ns: &[NodeRec], screen: Rect, roi: &Roi, cfg: &Cfg) -> (Vec<Finding>, usize) {
    let tol = 0.5;
    let mut out = Vec::new();
    let in_layer = |l: usize| roi.layers.as_ref().is_none_or(|ls| ls.contains(&l));
    let in_roi = |c: Pos2, l: usize| in_layer(l) && roi.area.contains(c) && !roi.exclude.iter().any(|e| e.contains(c));
    let texts: Vec<&TextRec> = cap.texts.iter().filter(|t| in_roi(t.rect.center(), t.layer) && !(roi.no_numbers && is_number(&t.text))).collect();
    let mut shortened = 0usize;
    // abgeschnittene Texte nicht noch einmal als „Rand zu knapp“ o. Ä. melden
    let mut cut = vec![false; texts.len()];
    // sichtbarer Teil je Text
    let vis: Vec<Rect> = texts.iter().map(|t| t.rect.intersect(t.clip).intersect(screen)).collect();
    let fields: Vec<&NodeRec> = ns.iter().filter(|n| matches!(n.role, Role::TextInput | Role::MultilineTextInput | Role::SearchInput | Role::SpinButton)).collect();
    for (k, t) in texts.iter().enumerate() {
        let v = vis[k];
        if !v.is_positive() || v.width() < 1.0 || v.height() < 1.0 {
            continue;
        }
        let ell = t.text.trim_end().ends_with('…');
        if ell {
            shortened += 1;
        }
        let bound = t.clip.intersect(screen);
        let cut_h = t.rect.left() < bound.left() - tol || t.rect.right() > bound.right() + tol;
        let cut_v = t.rect.top() < bound.top() - tol || t.rect.bottom() > bound.bottom() + tol;
        if (cut_h && !ell) || cut_v {
            let lost = 1.0 - v.area() / t.rect.area().max(1.0);
            let in_field = fields.iter().any(|f| f.rect.expand(1.0).contains_rect(bound));
            let at_hscroll = cut_h && !cut_v && roi.hscroll.is_some_and(|sv| (t.rect.right() > bound.right() + tol && (bound.right() - sv.right()).abs() < 1.0) || (t.rect.left() < bound.left() - tol && (bound.left() - sv.left()).abs() < 1.0));
            let kind = if in_field {
                "hinweis-feldinhalt"
            } else if cut_v && !cut_h && bound.height() >= 2.5 * t.rect.height() {
                "hinweis-bildlauf"
            } else if at_hscroll {
                // Text endet an der Kante des waagerechten Bildlaufs (Inspektor im schmalen
                // Fenster): er ist nur außerhalb des Ausschnitts und mit Bildlauf erreichbar (RAND-2a)
                "hinweis-bildlauf"
            } else {
                "abgeschnitten"
            };
            // auch am Bildlauf-Rand keine Rand-/Überlappungsprüfung für den angeschnittenen Text
            cut[k] = kind == "abgeschnitten" || at_hscroll;
            out.push(Finding { kind, text: t.text.clone(), rect: t.rect, score: lost, detail: format!("Clip {}", r1(t.clip)) });
        }
        // später gezeichnetes, deckendes Rechteck derselben Ebene
        let mut covered = 0.0f32;
        let mut by = Rect::NOTHING;
        for b in cap.boxes.iter().filter(|b| b.opaque && b.layer == t.layer && b.idx > t.idx) {
            let fv = b.rect.intersect(b.clip);
            let o = fv.intersect(v);
            if o.is_positive() && o.area() > 2.0 {
                // derselbe Text noch einmal darüber gezeichnet (Auswahl, Glühen): nicht verdeckt
                let redrawn = cap.texts.iter().any(|u| u.layer == t.layer && u.idx > b.idx && u.text == t.text && (u.rect.center() - t.rect.center()).length() < 1.5);
                if !redrawn && o.area() / v.area() > covered {
                    covered = o.area() / v.area();
                    by = fv;
                }
            }
        }
        if covered > 0.08 {
            out.push(Finding { kind: "verdeckt", text: t.text.clone(), rect: t.rect, score: covered, detail: format!("{:.0} % verdeckt von {}", covered * 100.0, r1(by)) });
        }
    }
    // Texte überlappen
    for (a, ta) in texts.iter().enumerate() {
        for (b, tb) in texts.iter().enumerate().skip(a + 1) {
            if ta.layer != tb.layer {
                continue;
            }
            let o = vis[a].intersect(vis[b]);
            if !o.is_positive() || o.area() <= 6.0 {
                continue;
            }
            if ta.text == tb.text && (ta.rect.center() - tb.rect.center()).length() < 2.5 {
                continue;
            }
            let (first, second) = if ta.idx < tb.idx { (ta, tb) } else { (tb, ta) };
            out.push(Finding { kind: "ueberlappt-text", text: first.text.clone(), rect: first.rect, score: o.area() / vis[a].area().min(vis[b].area()).max(1.0), detail: format!("mit „{}“ {}", short(&second.text), r1(second.rect)) });
        }
    }
    // Texte und Bedienelemente
    let ctrls: Vec<&NodeRec> = ns.iter().filter(|n| control(n.role) && n.layer.is_some_and(in_layer) && n.interact.is_positive() && roi.area.intersects(n.interact)).collect();
    for (k, t) in texts.iter().enumerate() {
        let v = vis[k];
        if !v.is_positive() || cut[k] {
            continue;
        }
        for c in &ctrls {
            if c.layer != Some(t.layer) {
                continue;
            }
            let o = v.intersect(c.interact);
            if !o.is_positive() || o.area() <= 4.0 {
                continue;
            }
            if c.rect.expand(tol).contains_rect(v) {
                if framed(c.role) && !fields.iter().any(|f| std::ptr::eq(*f, *c)) || matches!(c.role, Role::Button | Role::ComboBox) {
                    let gl = v.left() - c.rect.left();
                    let gr = c.rect.right() - v.right();
                    let gt = v.top() - c.rect.top();
                    let gb = c.rect.bottom() - v.bottom();
                    if gl.min(gr) < cfg.gap_x - 0.01 {
                        out.push(Finding { kind: "rand-knapp", text: t.text.clone(), rect: t.rect, score: (cfg.gap_x - gl.min(gr)) / cfg.gap_x, detail: format!("waagerecht {:.1}/{:.1} px zum Rahmen {} ({:?})", gl, gr, r1(c.rect), c.role) });
                    } else if gt.min(gb) < cfg.gap_y - 0.01 {
                        out.push(Finding { kind: "rand-knapp", text: t.text.clone(), rect: t.rect, score: (cfg.gap_y - gt.min(gb)) / cfg.gap_y.max(0.5), detail: format!("senkrecht {:.1}/{:.1} px zum Rahmen {} ({:?})", gt, gb, r1(c.rect), c.role) });
                    }
                }
            } else if c.rect.contains(t.rect.center()) {
                // nur, wenn außerhalb des Elements wirklich etwas sichtbar ist
                // Glyphen dürfen wie bei egui üblich ein Pixel über die Layoutbreite hinausragen
                let over = (c.rect.left() - v.left()).max(v.right() - c.rect.right()).max(c.rect.top() - v.top()).max(v.bottom() - c.rect.bottom());
                let outside = if over > 1.5 { v.area() - o.area() } else { 0.0 };
                if outside > 4.0 && fields.iter().any(|f| std::ptr::eq(*f, *c)) {
                    // Inhalt eines Eingabefelds, das waagerecht scrollt
                    out.push(Finding { kind: "hinweis-feldinhalt", text: t.text.clone(), rect: t.rect, score: 0.0, detail: format!("Feld {} ({:?})", r1(c.rect), c.role) });
                } else if outside > 4.0 {
                    out.push(Finding { kind: "ragt-heraus", text: t.text.clone(), rect: t.rect, score: outside / v.area().max(1.0), detail: format!("Element {} ({:?})", r1(c.rect), c.role) });
                }
            } else if !t.text.trim_end().ends_with('…') {
                out.push(Finding { kind: "ueberlappt-element", text: t.text.clone(), rect: t.rect, score: o.area() / v.area().max(1.0), detail: format!("läuft in {} ({:?})", r1(c.rect), c.role) });
            }
        }
    }
    // Kontrollkästchen und Optionsfelder ganz sichtbar
    for c in ns.iter().filter(|n| matches!(n.role, Role::CheckBox | Role::RadioButton) && n.layer.is_some_and(in_layer) && n.interact.is_positive() && roi.area.contains(n.rect.center())) {
        let Some(layer) = c.layer else { continue };
        let s = c.rect.height().min(13.0);
        let want = Rect::from_min_size(pos2(c.rect.left(), c.rect.center().y - s / 2.0), vec2(s, s));
        let found = cap.boxes.iter().filter(|b| b.layer == layer && c.rect.expand(1.0).intersects(b.rect) && (b.rect.center() - want.center()).length() < 4.0 && (b.rect.width() - want.width()).abs() < 5.0).min_by_key(|b| b.idx);
        let (bx, clip, idx) = match found {
            Some(b) => (b.rect, b.clip, b.idx),
            None => (want, screen, 0),
        };
        let visb = bx.intersect(clip).intersect(screen);
        let lost = 1.0 - if visb.is_positive() { visb.area() / bx.area().max(1.0) } else { 0.0 };
        let mut hid = 0.0f32;
        if idx > 0 {
            for b in cap.boxes.iter().filter(|b| b.opaque && b.layer == layer && b.idx > idx && !c.rect.expand(1.0).contains_rect(b.rect)) {
                let o = b.rect.intersect(b.clip).intersect(visb);
                if o.is_positive() {
                    hid = hid.max(o.area() / bx.area().max(1.0));
                }
            }
        }
        let worst = lost.max(hid);
        if worst > 0.02 {
            let label = texts.iter().filter(|t| t.layer == layer && c.rect.expand(2.0).intersects(t.rect)).map(|t| t.text.clone()).next().unwrap_or_default();
            out.push(Finding { kind: "kaestchen", text: label, rect: c.rect, score: worst, detail: format!("{:?}: {:.0} % des Kästchens {} nicht sichtbar{}", c.role, worst * 100.0, r1(bx), if hid > lost { " (verdeckt)" } else { "" }) });
        }
    }
    // Rand zu Dialog/Bedienfeld/Inspektor
    if let Some(fr) = roi.frame {
        for (k, t) in texts.iter().enumerate() {
            let v = vis[k];
            if !v.is_positive() || cut[k] || !fr.contains_rect(v) {
                continue;
            }
            let g = (v.left() - fr.left()).min(fr.right() - v.right());
            if g < cfg.gap_edge - 0.01 {
                out.push(Finding { kind: "rand-knapp", text: t.text.clone(), rect: t.rect, score: (cfg.gap_edge - g) / cfg.gap_edge, detail: format!("{g:.1} px zum Rand {}", r1(fr)) });
            }
        }
    }
    // Dialog bzw. schwebendes Fenster ragt hinaus
    for (i, a) in &cap.areas {
        if !in_layer(*i) || cap.layers[*i].order == Order::Tooltip || !a.is_positive() {
            continue;
        }
        if !screen.expand(tol).contains_rect(*a) {
            let v = a.intersect(screen);
            let lost = 1.0 - if v.is_positive() { v.area() / a.area() } else { 0.0 };
            out.push(Finding { kind: "dialog-ragt-hinaus", text: String::new(), rect: *a, score: lost, detail: format!("Fenster {}", r1(screen)) });
        }
    }
    (out, shortened)
}

fn short(s: &str) -> String {
    let s = s.replace(['\n', '\r', '|'], " ");
    if s.chars().count() > 60 { format!("{}…", s.chars().take(59).collect::<String>()) } else { s }
}

/// Texte mit gesperrten Namen (z. B. Systemschriften) nie in Bericht oder Liste (§1a).
fn neutral(s: &str) -> String {
    if fp_neutral::find_in_bytes(s.as_bytes()).is_empty() { short(s) } else { "(gesperrter Name, §1a)".into() }
}

// ------------------------------------------------------------------ Lauf

#[derive(Clone, Debug)]
struct Hit {
    var: Var,
    area: &'static str,
    view_idx: usize,
    view: String,
    f: Finding,
}

impl Hit {
    fn key(&self) -> String {
        // nur der Anfang des Texts: geänderte Enden langer Hinweise ergeben keinen „neuen“ Fund
        let t: String = neutral(&stable(&self.f.text)).chars().take(32).collect();
        format!("{}|{}|{}", self.f.kind, self.view, t.trim_end_matches('…').trim_end())
    }
}

/// Wechselnde Teile (Testordner mit Prozessnummer) für den Schlüssel der Erlaubnisliste ersetzen.
fn stable(s: &str) -> String {
    if !s.contains("firepixel-gesamttest") && !s.contains(":\\") {
        return s.to_string();
    }
    let mut out = String::new();
    let mut digits = String::new();
    for ch in s.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
            continue;
        }
        out.push_str(if digits.len() >= 2 { "#" } else { &digits });
        digits.clear();
        out.push(ch);
    }
    out.push_str(if digits.len() >= 2 { "#" } else { &digits });
    out
}

fn run_variant(var: Var, views: &[View], cands_label: &[Candidate], cfg: &Cfg, timing: &std::sync::Mutex<Vec<(String, f64)>>) -> (Vec<Hit>, usize) {
    let t0 = std::time::Instant::now();
    let (st, _ctx, f) = fixture();
    let cands = plain_candidates(&f);
    assert_eq!(cands.len(), cands_label.len(), "gleiche Testsituationen");
    let mut h = harness(app_for(st, &var), &var, false);
    h.run_steps(3);
    let mut hits = Vec::new();
    let mut shortened = 0;
    for (vi, v) in views.iter().enumerate() {
        let (cap, ns, screen, roi) = run_view(&mut h, v, &cands);
        let (fs, sh) = analyze(&cap, &ns, screen, &roi, cfg);
        shortened += sh;
        let name = v.name(&cands);
        if std::env::var_os("FP_RAND_DEBUG").is_some() {
            eprintln!("== {name} [{}] Bereich {} Ebenen {:?} Rahmen {:?}", var.label(), r1(roi.area), roi.layers, roi.frame.map(r1));
            for (i, l) in cap.layers.iter().enumerate() {
                eprintln!("   Ebene {i}: {l:?} Texte {} Flächen {} Bereich {:?}", cap.texts.iter().filter(|t| t.layer == i).count(), cap.boxes.iter().filter(|t| t.layer == i).count(), cap.areas.iter().find(|a| a.0 == i).map(|a| r1(a.1)));
            }
            for t in &cap.texts {
                eprintln!("   T{} #{} „{}“ {} Clip {}", t.layer, t.idx, short(&t.text), r1(t.rect), r1(t.clip));
            }
            for n in &ns {
                eprintln!("   N {:?} {} Ebene {:?}", n.role, r1(n.rect), n.layer);
            }
            for b in cap.boxes.iter().filter(|b| b.opaque && roi.area.intersects(b.rect)) {
                eprintln!("   B{} #{} {} Clip {}", b.layer, b.idx, r1(b.rect), r1(b.clip));
            }
            for f in &fs {
                eprintln!("   => {} „{}“ {}", f.kind, short(&f.text), f.detail);
            }
        }
        for f in fs {
            hits.push(Hit { var, area: v.area(), view_idx: vi, view: name.clone(), f });
        }
    }
    if let Ok(mut t) = timing.lock() {
        t.push((var.label(), t0.elapsed().as_secs_f64()));
    }
    (hits, shortened)
}

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn allow_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("clipping_audit_allow.txt")
}

fn read_allow() -> BTreeSet<String> {
    std::fs::read_to_string(allow_path()).unwrap_or_default().lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')).map(str::to_string).collect()
}

#[test]
fn clipping_audit() {
    fp_engine::shell::disable();
    let cfg = Cfg::from_env();
    let quick = std::env::var_os("FP_RAND_FULL").is_none() || std::env::var_os("FP_RAND_VIEWS").is_some();
    let threads: usize = std::env::var("FP_RAND_THREADS").ok().and_then(|v| v.parse().ok()).unwrap_or(4).max(1);
    let t0 = std::time::Instant::now();
    assert!(fp_i18n::set_language("de"));
    let (_, _, f) = fixture();
    let cands = plain_candidates(&f);
    let dialogs = discover_dialogs(&cands);
    let views = all_views(&cands, &dialogs);
    eprintln!("RAND-1: {} Ansichten ({} Dialoge), Auswahl in {:.1} s", views.len(), dialogs.len(), t0.elapsed().as_secs_f64());
    let timing = std::sync::Mutex::new(Vec::new());
    let mut hits: Vec<Hit> = Vec::new();
    let mut shortened = 0usize;
    let mut nvars = 0usize;
    for lang in ["de", "en"] {
        assert!(fp_i18n::set_language(lang), "Sprache {lang}");
        let vars = variants(lang);
        nvars += vars.len();
        for chunk in vars.chunks(threads) {
            let res: Vec<(Vec<Hit>, usize)> = std::thread::scope(|s| {
                let hs: Vec<_> = chunk.iter().map(|v| {
                    let (views, cands, cfg, timing) = (&views, &cands, &cfg, &timing);
                    let v = *v;
                    std::thread::Builder::new().stack_size(64 << 20).spawn_scoped(s, move || run_variant(v, views, cands, cfg, timing)).expect("Thread")
                }).collect();
                hs.into_iter().map(|h| h.join().unwrap_or_else(|e| std::panic::resume_unwind(e))).collect()
            });
            for (h, sh) in res {
                hits.extend(h);
                shortened += sh;
            }
        }
    }
    assert!(fp_i18n::set_language("de"));
    for (v, s) in timing.lock().map(|t| t.clone()).unwrap_or_default() {
        eprintln!("  {v}: {s:.1} s");
    }
    // Auswertung gegen die Erlaubnisliste
    let allow = read_allow();
    let mut by_key: BTreeMap<String, Vec<&Hit>> = BTreeMap::new();
    for h in hits.iter().filter(|h| !hint(h.f.kind)) {
        by_key.entry(h.key()).or_default().push(h);
    }
    let new: Vec<&String> = by_key.keys().filter(|k| !allow.contains(*k)).collect();
    let stale: Vec<&String> = allow.iter().filter(|k| !by_key.contains_key(*k)).collect();
    eprintln!("RAND-1: {} Funde ({} Schlüssel), {} Hinweise, {} mit „…“ gekürzt; neu: {}, nicht mehr gefunden: {}; {:.0} s", hits.iter().filter(|h| !hint(h.f.kind)).count(), by_key.len(), hits.iter().filter(|h| hint(h.f.kind)).count(), shortened, new.len(), stale.len(), t0.elapsed().as_secs_f64());
    let update = std::env::var_os("FP_RAND_UPDATE_ALLOW").is_some();
    if update && quick {
        eprintln!("FP_RAND_UPDATE_ALLOW wirkt nur zusammen mit FP_RAND_FULL=1 (ohne FP_RAND_VIEWS) – Liste unverändert");
    }
    let (allow, stale) = if update && !quick { (by_key.keys().cloned().collect::<BTreeSet<String>>(), Vec::new()) } else { (allow.clone(), stale) };
    if update && !quick {
        let mut s = String::from("# Erlaubnisliste des Prüfers clipping_audit.rs (RAND-1, N23): bekannte Funde `Art|Ansicht|Text`.\n# Behobene Zeilen streichen (der Bericht docs\\funktionstest\\rand_auto.md nennt sie unter „nicht mehr gefunden“).\n# Neu schreiben: FP_RAND_FULL=1 FP_RAND_UPDATE_ALLOW=1 cargo test -p fp-ui --test clipping_audit\n");
        for k in by_key.keys() {
            s.push_str(k);
            s.push('\n');
        }
        std::fs::write(allow_path(), s).expect("Erlaubnisliste schreiben");
    }
    if let Ok(d) = std::env::var("FP_RAND_SHOT_DIR") {
        // nur Bilder (z. B. zusammen mit FP_RAND_VIEWS), ohne Bericht
        screenshots(&by_key, &views, &cands, Path::new(&d));
    } else if !quick {
        let shots = screenshots(&by_key, &views, &cands, &repo().join("docs").join("screenshots").join("rand"));
        write_report(&hits, &by_key, &allow, &stale, shortened, nvars, views.len(), dialogs.len(), &cfg, &shots);
    }
    if !update {
        assert!(new.is_empty(), "{} neue Funde (nicht in clipping_audit_allow.txt):\n{}", new.len(), new.iter().take(80).map(|k| format!("  {k}  e.g. {}", by_key[*k][0].f.detail)).collect::<Vec<_>>().join("\n"));
    }
}

// ------------------------------------------------------------------ Bericht und Bilder

fn esc(s: &str) -> String {
    neutral(s).replace('|', "/")
}

#[allow(clippy::too_many_arguments)]
fn write_report(hits: &[Hit], by_key: &BTreeMap<String, Vec<&Hit>>, allow: &BTreeSet<String>, stale: &[&String], shortened: usize, nvars: usize, nviews: usize, ndialogs: usize, cfg: &Cfg, shots: &[(String, String)]) {
    let mut md = String::new();
    md.push_str("# Rand- und Abschneide-Prüfung (automatisch, RAND-1)\n\n");
    md.push_str("Erzeugt von `crates\\fp-ui\\tests\\clipping_audit.rs` (Nutzerbefund N23). Nicht von Hand bearbeiten.\n\n");
    md.push_str(&format!("- {nviews} Ansichten ({ndialogs} Dialoge aus dem Befehlsregister) × {nvars} Varianten (3 Designs × de/en × 100/150 % × 1024 × 768 / 1920 × 1080)\n"));
    md.push_str(&format!("- Grenzwerte: waagerecht {} px, senkrecht {} px zum Rahmen, {} px zum Rand (`FP_RAND_GAP_X`, `FP_RAND_GAP_Y`, `FP_RAND_GAP_EDGE`)\n", cfg.gap_x, cfg.gap_y, cfg.gap_edge));
    let real: Vec<&Hit> = hits.iter().filter(|h| !hint(h.f.kind)).collect();
    md.push_str(&format!("- **{} Funde** an **{} Stellen** (Art + Ansicht + Text), dazu {} Hinweise; {} Texte sind gewollt mit „…“ gekürzt\n", real.len(), by_key.len(), hits.len() - real.len(), shortened));
    md.push_str(&format!("- Erlaubnisliste: {} Einträge, davon {} nicht mehr gefunden\n\n", allow.len(), stale.len()));
    // Zahlen je Bereich und Art
    let kinds = ["kaestchen", "verdeckt", "abgeschnitten", "dialog-ragt-hinaus", "ueberlappt-text", "ueberlappt-element", "ragt-heraus", "rand-knapp", "hinweis-bildlauf", "hinweis-feldinhalt"];
    let areas = ["Hauptfenster", "Bedienfelder", "Inspektor", "Dialoge", "Startseite"];
    md.push_str("## Stellen je Bereich (Funde in Klammern)\n\n| Bereich |");
    for k in kinds {
        md.push_str(&format!(" {k} |"));
    }
    md.push_str(" Summe |\n|---|");
    for _ in kinds {
        md.push_str("---|");
    }
    md.push_str("---|\n");
    for a in areas {
        md.push_str(&format!("| {a} |"));
        let mut sum = (0, 0);
        for k in kinds {
            let hs: Vec<&Hit> = hits.iter().filter(|h| h.area == a && h.f.kind == k).collect();
            let keys: BTreeSet<String> = hs.iter().map(|h| h.key()).collect();
            if !hint(k) {
                sum.0 += keys.len();
                sum.1 += hs.len();
            }
            md.push_str(&if hs.is_empty() { " – |".to_string() } else { format!(" {} ({}) |", keys.len(), hs.len()) });
        }
        md.push_str(&format!(" **{}** ({}) |\n", sum.0, sum.1));
    }
    if !shots.is_empty() {
        md.push_str("\n## Ausschnitte der schlimmsten Funde\n\n");
        for (key, file) in shots {
            md.push_str(&format!("- `{}` → `docs\\screenshots\\rand\\{file}`\n", esc(key)));
        }
    }
    md.push_str("\n## Fundstellen\n\nJe Stelle: Art, Text, Rechteck (Punkte, erste Variante), Zahl der Varianten, Beispielvariante und Erklärung. Sortiert nach Bereich, Ansicht und Schwere.\n");
    type ByView<'a> = BTreeMap<(&'a str, String), Vec<(&'a String, &'a Vec<&'a Hit>)>>;
    let mut by_view: ByView = BTreeMap::new();
    for (k, v) in by_key {
        by_view.entry((v[0].area, v[0].view.clone())).or_default().push((k, v));
    }
    let mut cur_area = "";
    for ((area, view), mut items) in by_view {
        if area != cur_area {
            md.push_str(&format!("\n### {area}\n"));
            cur_area = area;
        }
        items.sort_by(|a, b| (weight(b.1[0].f.kind)).total_cmp(&weight(a.1[0].f.kind)).then(a.0.cmp(b.0)));
        md.push_str(&format!("\n**{}** ({} Stellen)\n\n| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |\n|---|---|---|---|---|---|\n", esc(&view), items.len()));
        for (_, hs) in items {
            let h = hs.iter().max_by(|a, b| a.f.score.total_cmp(&b.f.score)).copied().unwrap_or(hs[0]);
            let nv: BTreeSet<String> = hs.iter().map(|h| h.var.label()).collect();
            md.push_str(&format!("| {} | {} | {} | {} | {} | {} |\n", h.f.kind, esc(&h.f.text), r1(h.f.rect), nv.len(), h.var.label(), esc(&h.f.detail)));
        }
    }
    let hints: Vec<&Hit> = hits.iter().filter(|h| hint(h.f.kind)).collect();
    if !hints.is_empty() {
        let mut hk: BTreeMap<String, usize> = BTreeMap::new();
        for h in &hints {
            *hk.entry(format!("{} | {} | {}", h.f.kind, esc(&h.view), esc(&h.f.text))).or_default() += 1;
        }
        md.push_str(&format!("\n## Hinweise (zählen nicht, {} Stellen)\n\n<details><summary>Liste</summary>\n\n", hk.len()));
        for (k, n) in hk {
            md.push_str(&format!("- {k} ({n}×)\n"));
        }
        md.push_str("\n</details>\n");
    }
    if !stale.is_empty() {
        md.push_str("\n## In der Erlaubnisliste, aber nicht mehr gefunden (streichen)\n\n");
        for s in stale {
            md.push_str(&format!("- `{}`\n", esc(s)));
        }
    }
    let path = repo().join("docs").join("funktionstest").join("rand_auto.md");
    let _ = std::fs::create_dir_all(path.parent().expect("Ordner"));
    std::fs::write(&path, md).expect("Bericht schreiben");
}

/// Ausschnittsbilder der schlimmsten Stellen (mit Grafikadapter).
fn screenshots(by_key: &BTreeMap<String, Vec<&Hit>>, views: &[View], cands_label: &[Candidate], dir: &Path) -> Vec<(String, String)> {
    let n: usize = std::env::var("FP_RAND_SHOTS").ok().and_then(|v| v.parse().ok()).unwrap_or(12);
    let _ = std::fs::create_dir_all(dir);
    // alte Bilder des Prüfers ersetzen (nur rand_*.png)
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            if e.file_name().to_string_lossy().starts_with("rand_") {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    // je Ansicht höchstens ein Bild, schwerste zuerst
    let mut best: Vec<(&String, &Hit)> = by_key.iter().map(|(k, v)| (k, *v.iter().max_by(|a, b| a.f.score.total_cmp(&b.f.score)).expect("Fund"))).collect();
    best.sort_by(|a, b| (weight(b.1.f.kind) + b.1.f.score).total_cmp(&(weight(a.1.f.kind) + a.1.f.score)));
    // erst je Art das schwerste Beispiel, dann der Rest
    let mut kinds_seen = BTreeSet::new();
    let (first, rest): (Vec<_>, Vec<_>) = best.into_iter().partition(|(_, h)| kinds_seen.insert(h.f.kind));
    let best: Vec<(&String, &Hit)> = first.into_iter().chain(rest).collect();
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for (key, h) in best {
        if out.len() >= n {
            break;
        }
        // je Ansicht und je Art + Text höchstens ein Bild
        if seen.contains(&h.view) || !seen.insert(format!("{}|{}", h.f.kind, neutral(&h.f.text))) {
            continue;
        }
        seen.insert(h.view.clone());
        assert!(fp_i18n::set_language(h.var.lang));
        let (st, _ctx, f) = fixture();
        let cands = plain_candidates(&f);
        assert_eq!(cands.len(), cands_label.len());
        let mut hh = harness(app_for(st, &h.var), &h.var, true);
        hh.run_steps(3);
        let (cap, _, screen, _) = run_view(&mut hh, &views[h.view_idx], &cands);
        let img = match hh.render() {
            Ok(i) => i,
            Err(e) => {
                eprintln!("kein Grafikadapter, keine Ausschnitte: {e}");
                break;
            }
        };
        let crop = h.f.rect.expand(40.0).intersect(screen);
        // §1a: keine Ausschnitte mit gesperrten Namen (z. B. installierte Systemschriften)
        if cap.texts.iter().any(|t| crop.intersects(t.rect) && !fp_neutral::find_in_bytes(t.text.as_bytes()).is_empty()) {
            eprintln!("{key}: gesperrter Name im Ausschnitt – Bild nicht gespeichert");
            continue;
        }
        let ppp = h.var.ppp;
        let (x0, y0) = ((crop.left() * ppp).max(0.0) as u32, (crop.top() * ppp).max(0.0) as u32);
        let (w, hgt) = (((crop.width() * ppp) as u32).min(img.width().saturating_sub(x0)), ((crop.height() * ppp) as u32).min(img.height().saturating_sub(y0)));
        if w < 4 || hgt < 4 {
            continue;
        }
        let mut sub = image::imageops::crop_imm(&img, x0, y0, w, hgt).to_image();
        // Fundstelle rot umranden
        let fr = h.f.rect.expand(1.0);
        let (fx0, fy0, fx1, fy1) = (((fr.left() * ppp) as i64 - x0 as i64), ((fr.top() * ppp) as i64 - y0 as i64), ((fr.right() * ppp) as i64 - x0 as i64), ((fr.bottom() * ppp) as i64 - y0 as i64));
        let mut put = |x: i64, y: i64| {
            if x >= 0 && y >= 0 && (x as u32) < sub.width() && (y as u32) < sub.height() {
                sub.put_pixel(x as u32, y as u32, image::Rgba([230, 20, 20, 255]));
            }
        };
        for x in fx0..=fx1 {
            put(x, fy0);
            put(x, fy1);
        }
        for y in fy0..=fy1 {
            put(fx0, y);
            put(fx1, y);
        }
        let slug: String = h.view.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' }).collect::<String>().split('_').filter(|s| !s.is_empty()).collect::<Vec<_>>().join("_");
        let file = format!("rand_{:02}_{}_{}.png", out.len() + 1, h.f.kind.replace('-', "_"), slug.chars().take(48).collect::<String>());
        if sub.save(dir.join(&file)).is_ok() {
            out.push((key.clone(), file));
        }
    }
    assert!(fp_i18n::set_language("de"));
    out
}
