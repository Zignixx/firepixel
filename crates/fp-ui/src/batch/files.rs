//! Karte ① Dateien (SPEC-BATCH §2.3): Hinzufügen (Dateien, Ordner, Ordner mit Unterordnern,
//! offene Dokumente), Ziehen & Ablegen, Filter (Suche, Typ; Profi: regulärer Ausdruck), Sortierung,
//! virtuelle Liste mit Miniaturen, Zustand je Datei, leerer Zustand.

use super::kit::{self, BtnKind};
use super::{EntryStatus, FileEntry, Inbox, Mode, Studio};
use crate::app_state::AppState;
use crate::theme::{mix, with_alpha, Theme, ThemeKind};
use crate::ui_kit::{self, sp, type_scale, KitIcon, RowIcon, Tone};
use egui::{pos2, vec2, Align2, Color32, CornerRadius, Id, Rect, Sense, Stroke, StrokeKind, Ui};
use fp_batch::ItemKind;
use fp_i18n::t;
use std::path::PathBuf;

/// Höhe einer Dateizeile (wie „Zuletzt geöffnet“ auf dem Startbildschirm).
pub const ROW_H: f32 = 44.0;

/// Sortierung (SPEC §2.3). Die Reihenfolge bestimmt die Nummerierung beim Umbenennen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortKey {
    Added,
    Name,
    Date,
    Size,
    Pixels,
}

impl SortKey {
    pub const ALL: [SortKey; 5] = [SortKey::Added, SortKey::Name, SortKey::Date, SortKey::Size, SortKey::Pixels];
    pub fn label(self) -> String {
        t(match self {
            SortKey::Added => "batch-ui-sort-added",
            SortKey::Name => "batch-ui-sort-name",
            SortKey::Date => "batch-ui-sort-date",
            SortKey::Size => "batch-ui-sort-size",
            SortKey::Pixels => "batch-ui-sort-pixels",
        })
    }
}

/// Filter der Liste. Gefilterte Dateien werden nicht verarbeitet.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Filter {
    /// Suchtext im Namen (Groß/klein egal).
    pub text: String,
    /// Typ (None = alle).
    pub kind: Option<ItemKind>,
    /// Profi: regulärer Ausdruck auf den Namen.
    pub regex: String,
    /// Nur Dateien mit Fehler (nach einem Lauf).
    pub only_failed: bool,
    /// Profi: Dateigröße ab/bis (MB; Befund BATCH-2, SPEC §2.3).
    pub min_mb: Option<f64>,
    pub max_mb: Option<f64>,
    /// Profi: längste Seite ab/bis (Pixel; unbekannte Maße passen immer).
    pub min_px: Option<u32>,
    pub max_px: Option<u32>,
}

impl Filter {
    /// Kennzahl für den Zwischenspeicher der Reihenfolge.
    pub fn version(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.text.hash(&mut h);
        self.kind.map(|k| k as u8).hash(&mut h);
        self.regex.hash(&mut h);
        self.only_failed.hash(&mut h);
        self.min_mb.map(f64::to_bits).hash(&mut h);
        self.max_mb.map(f64::to_bits).hash(&mut h);
        self.min_px.hash(&mut h);
        self.max_px.hash(&mut h);
        h.finish()
    }

    pub fn is_active(&self) -> bool {
        !self.text.is_empty() || self.kind.is_some() || !self.regex.is_empty() || self.only_failed || self.has_size_filter()
    }

    /// Filter nach Größe bzw. Pixeln gesetzt?
    pub fn has_size_filter(&self) -> bool {
        self.min_mb.is_some() || self.max_mb.is_some() || self.min_px.is_some() || self.max_px.is_some()
    }

    /// Passt eine Datei zu Größe und Pixeln?
    pub fn size_ok(&self, info: &fp_batch::ItemInfo) -> bool {
        let mb = info.bytes as f64 / (1024.0 * 1024.0);
        let long = info.width.max(info.height);
        self.min_mb.is_none_or(|m| mb >= m)
            && self.max_mb.is_none_or(|m| mb <= m)
            && (long == 0 || (self.min_px.is_none_or(|m| long >= m) && self.max_px.is_none_or(|m| long <= m)))
    }
}

/// Einfache Platzhalter im Suchtext: `*` (beliebig viele Zeichen) und `?` (ein Zeichen), ohne
/// Groß-/Kleinschreibung, über den ganzen Namen (`*.png`, `urlaub_??.*`).
pub fn glob_match(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    let n: Vec<char> = name.to_lowercase().chars().collect();
    let (mut pi, mut ni) = (0usize, 0usize);
    let (mut star, mut mark) = (None::<usize>, 0usize);
    while ni < n.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == n[ni]) {
            pi += 1;
            ni += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ni;
            pi += 1;
        } else if let Some(st) = star {
            pi = st + 1;
            mark += 1;
            ni = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

/// Filtert und sortiert: Indizes in `files`.
pub fn order(files: &[FileEntry], f: &Filter, sort: SortKey, mode: Mode) -> Vec<usize> {
    let needle = f.text.to_lowercase();
    // Suchtext mit * oder ? = Platzhalter über den ganzen Namen (BATCH-11b)
    let glob = needle.contains(['*', '?']);
    let size_filter = mode == Mode::Pro && f.has_size_filter();
    let re = if mode == Mode::Pro && !f.regex.is_empty() { fp_batch::naming::compile_regex(&fp_batch::recipe::RegexRename { find: f.regex.clone(), ignore_case: true, ..Default::default() }).ok() } else { None };
    let mut v: Vec<usize> = (0..files.len())
        .filter(|&i| {
            let e = &files[i];
            let name = e.info.file_name();
            (needle.is_empty() || if glob { glob_match(&needle, &name) } else { name.to_lowercase().contains(&needle) })
                && (!size_filter || f.size_ok(&e.info))
                && f.kind.is_none_or(|k| e.info.kind == k)
                && re.as_ref().is_none_or(|r| r.is_match(&name))
                && (!f.only_failed || e.status == EntryStatus::Failed)
        })
        .collect();
    match sort {
        SortKey::Added => v.sort_by_key(|&i| files[i].added),
        SortKey::Name => v.sort_by(|&a, &b| fp_batch::item::natural_cmp(&files[a].info.file_name(), &files[b].info.file_name())),
        SortKey::Date => v.sort_by_key(|&i| files[i].info.modified),
        SortKey::Size => v.sort_by_key(|&i| files[i].info.bytes),
        SortKey::Pixels => v.sort_by_key(|&i| files[i].info.width as u64 * files[i].info.height as u64),
    }
    v
}

/// Bekannte Endungen für den Datei-Dialog.
fn extensions() -> Vec<&'static str> {
    let mut v = crate::files::readable_extensions();
    for e in fp_batch::item::VIDEO_EXTENSIONS {
        if !v.contains(e) {
            v.push(e);
        }
    }
    v
}

/// „Dateien…“: Mehrfachauswahl.
///
/// Das Studio kommt als Parameter (N33): während es sich zeichnet, ist es aus `st.batch`
/// herausgenommen (`batch::show`). Früher holte die Funktion es von dort, kehrte darum still
/// zurück, und „+ Dateien…“/„Ordner…“ öffneten keinen Dialog.
pub fn pick_files(st: &mut AppState, s: &Studio) {
    let inbox = s.inbox_sender();
    let exts = extensions();
    let req = crate::file_dialog::Request::open_many(t("batch-ui-files-add-title")).filter(t("batch-ui-filter-supported"), &exts).filter(t("ui-filter-all-files"), &["*"]);
    crate::file_dialog::request(st, req, move |_st, paths| {
        if !paths.is_empty() {
            inbox.lock().unwrap_or_else(|e| e.into_inner()).push(Inbox::AddPaths(paths, false));
        }
    });
}

/// „Ordner…“ bzw. „Ordner mit Unterordnern…“.
/// Tooltip des Knopfs „Ordner…“: sagt, ob Unterordner mitgelesen werden (Einstellung im ▾-Menü).
fn folder_tooltip(recursive: bool) -> String {
    t(if recursive { "batch-ui-files-add-folder.tooltip-rec" } else { "batch-ui-files-add-folder.tooltip-flat" })
}

pub fn pick_folder(st: &mut AppState, s: &Studio, recursive: bool) {
    let inbox = s.inbox_sender();
    let req = crate::file_dialog::Request::folder(t("batch-ui-files-folder-title"));
    crate::file_dialog::request(st, req, move |_st, paths| {
        if !paths.is_empty() {
            inbox.lock().unwrap_or_else(|e| e.into_inner()).push(Inbox::AddPaths(paths, recursive));
        }
    });
}

/// Offene Dokumente mit Datei auf der Festplatte hinzufügen.
fn add_open_documents(st: &mut AppState, s: &mut Studio, now: f64) {
    let paths: Vec<PathBuf> = st.session.documents().iter().filter_map(|d| d.path.clone()).collect();
    if paths.is_empty() {
        s.notify(Tone::Info, t("batch-ui-files-no-open-docs"), now);
    } else {
        s.add_paths_notice(&paths, false, now);
    }
}

fn kind_label(k: Option<ItemKind>) -> String {
    t(match k {
        None => "batch-ui-filter-all",
        Some(ItemKind::Image) => "batch-ui-filter-images",
        Some(ItemKind::Animation) => "batch-ui-filter-animations",
        Some(ItemKind::Video) => "batch-ui-filter-videos",
        Some(ItemKind::Document) => "batch-ui-filter-documents",
        Some(ItemKind::Other) => "batch-ui-filter-other",
    })
}

/// Zeichnet die Karte.
pub fn show(ui: &mut Ui, st: &mut AppState, s: &mut Studio, rect: Rect, now: f64, running: bool) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let col = kit::accent(&theme, 0);
    let p = ui.painter().clone();
    let vis = s.visible();
    let total_bytes: u64 = vis.iter().map(|&i| s.files[i].info.bytes).sum();
    let count = if s.filter.is_active() || vis.len() != s.files.len() {
        fp_i18n::tr!("batch-ui-files-count-filtered", shown = vis.len(), total = s.files.len())
    } else {
        fp_i18n::tr!("batch-ui-files-count", n = s.files.len())
    };
    let inner = kit::card(&p, rect, &theme, col, RowIcon::Icon(crate::icons::Icon::Folder), &t("batch-ui-files-title"), Some(&count));
    // Ablegen: Karte getönt, gestrichelter Rand
    if s.drop_hover {
        p.rect_filled(rect.shrink(1.0), CornerRadius::same(kit::RADIUS), with_alpha(c.focus, 28));
        kit::dashed_rect(&p, rect.shrink(3.0), c.focus);
    }
    let enabled = !running;
    // ------------------------------------------------ Knopfzeile
    let mut y = inner.top();
    let add_l = t("batch-ui-files-add");
    let folder_l = t("batch-ui-files-add-folder");
    let w_add = kit::btn_w(ui.ctx(), &add_l, true);
    let w_folder = kit::btn_w(ui.ctx(), &folder_l, true);
    let bar = Rect::from_min_size(pos2(inner.left(), y), vec2(inner.width(), kit::BTN_H));
    let more_w = 24.0;
    let fits_two = w_add + w_folder + more_w + 2.0 * sp::XS <= bar.width();
    let w1 = if fits_two { w_add } else { (bar.width() - more_w - sp::XS).max(40.0) };
    let r_add = Rect::from_min_size(bar.min, vec2(w1, kit::BTN_H));
    if kit::button(ui, r_add, Id::new("fp-batch-add-files"), &add_l, Some(RowIcon::Icon(crate::icons::Icon::Plus)), BtnKind::Secondary, enabled, &add_l).on_hover_text(t("batch-ui-files-add.tooltip")).clicked() {
        pick_files(st, s);
    }
    let mut x = r_add.right() + sp::XS;
    if fits_two {
        let r_f = Rect::from_min_size(pos2(x, bar.top()), vec2(w_folder, kit::BTN_H));
        if kit::button(ui, r_f, Id::new("fp-batch-add-folder"), &folder_l, Some(RowIcon::Icon(crate::icons::Icon::Folder)), BtnKind::Secondary, enabled, &folder_l).on_hover_text(folder_tooltip(s.recursive)).clicked() {
            let rec = s.recursive;
            pick_folder(st, s, rec);
        }
        x = r_f.right() + sp::XS;
    }
    let r_more = Rect::from_min_size(pos2(x, bar.top()), vec2(more_w, kit::BTN_H));
    let more_name = t("batch-ui-files-more");
    let mresp = kit::button(ui, r_more, Id::new("fp-batch-add-more"), "", Some(RowIcon::Icon(crate::icons::Icon::DropDown)), BtnKind::Ghost, enabled, &more_name);
    egui::Popup::menu(&mresp).id(Id::new("fp-batch-add-menu")).show(|ui| {
        crate::theme::field_widgets(ui);
        // eigene Beschriftung: „Ordner…“ der Knopfzeile folgt „Unterordner einbeziehen“, dieser
        // Eintrag liest immer flach (BATCH-T: gleiche Beschriftung, anderes Verhalten)
        if ui.button(t("batch-ui-files-add-folder-flat")).clicked() {
            pick_folder(st, s, false);
        }
        if ui.button(t("batch-ui-files-add-folder-rec")).clicked() {
            pick_folder(st, s, true);
        }
        if ui.button(t("batch-ui-files-add-open")).clicked() {
            add_open_documents(st, s, now);
        }
        ui.separator();
        let mut rec = s.recursive;
        if ui.checkbox(&mut rec, t("batch-ui-recursive")).changed() {
            s.recursive = rec;
            st.session.prefs.set(super::PREF_RECURSIVE, fp_engine::PrefValue::Bool(rec));
            st.session.prefs_changed();
        }
    });
    y = bar.bottom() + sp::S;
    // ------------------------------------------------ Filterzeile
    if !s.files.is_empty() {
        let fr = Rect::from_min_size(pos2(inner.left(), y), vec2(inner.width(), kit::BTN_H));
        filter_row(ui, s, fr);
        y = fr.bottom() + sp::S;
    }
    // ------------------------------------------------ Fuß: Summe, Entfernen, Leeren
    let foot_h = if s.files.is_empty() { 0.0 } else { kit::BTN_H };
    let foot = Rect::from_min_max(pos2(inner.left(), inner.bottom() - foot_h), inner.max);
    let list = Rect::from_min_max(pos2(inner.left(), y), pos2(inner.right(), foot.top() - if foot_h > 0.0 { sp::S } else { 0.0 }));
    if s.files.is_empty() {
        empty_state(ui, st, s, list, enabled);
    } else {
        file_list(ui, s, list, &vis, &theme, now, enabled);
        footer(ui, s, foot, total_bytes, enabled);
    }
    // Hinweis über dem Fuß (z. B. „3 Dateien waren schon in der Liste“)
    if let Some(n) = s.notice.clone() {
        let w = inner.width();
        let h = ui_kit::notice_height(ui.ctx(), None, &n.text, w);
        let nr = Rect::from_min_size(pos2(inner.left(), (foot.top() - sp::S - h).max(inner.top())), vec2(w, h));
        p.rect_filled(nr.expand(2.0), CornerRadius::same(6), kit::card_fill(&theme));
        ui_kit::notice_at(&p, nr, n.tone, None, &n.text, &theme);
    }
}

fn filter_row(ui: &mut Ui, s: &mut Studio, r: Rect) {
    let theme = Theme::current(ui.ctx());
    let kind_w = 92.0f32.min(r.width() * 0.3);
    let sort_w = 92.0f32.min(r.width() * 0.3);
    let text_r = Rect::from_min_max(r.min, pos2(r.right() - kind_w - sort_w - 2.0 * sp::XS, r.bottom()));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(text_r).layout(egui::Layout::left_to_right(egui::Align::Center)));
    crate::theme::field_widgets(&mut child);
    let pro = s.mode == Mode::Pro;
    // Das Feld ist schmal: der Hinweis bleibt kurz, die Regex-Erklärung (Profi) steht im Tooltip
    // (BATCH-T: „Filtern... (mit / da…“ war abgeschnitten).
    let hint = t("batch-ui-filter-hint");
    let mut f = s.filter.clone();
    let field = if pro && f.text.is_empty() && !f.regex.is_empty() { &mut f.regex } else { &mut f.text };
    let resp = child.add_sized(text_r.size(), egui::TextEdit::singleline(field).hint_text(hint.clone()).desired_width(text_r.width()));
    resp.widget_info(|| egui::WidgetInfo::text_edit(true, "", &hint, ""));
    let tip = if pro { format!("{}\n{}", t("batch-ui-filter-glob-hint"), t("batch-ui-filter-hint-pro")) } else { t("batch-ui-filter-glob-hint") };
    let resp = resp.on_hover_text(tip);
    let _ = resp;
    let _ = theme;
    // Typ
    let kr = Rect::from_min_size(pos2(text_r.right() + sp::XS, r.top()), vec2(kind_w, r.height()));
    let mut kc = ui.new_child(egui::UiBuilder::new().max_rect(kr));
    crate::theme::field_widgets(&mut kc);
    let kind_text = if pro && f.has_size_filter() { format!("{} •", kind_label(f.kind)) } else { kind_label(f.kind) };
    egui::ComboBox::from_id_salt("fp-batch-filter-kind").selected_text(kind_text).width(kind_w).truncate().close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show_ui(&mut kc, |ui| {
        for k in [None, Some(ItemKind::Image), Some(ItemKind::Animation), Some(ItemKind::Video), Some(ItemKind::Document)] {
            ui.selectable_value(&mut f.kind, k, kind_label(k));
        }
        if s.files.iter().any(|e| e.status == EntryStatus::Failed) {
            ui.separator();
            ui.checkbox(&mut f.only_failed, t("batch-ui-filter-failed"));
        }
        if pro {
            ui.separator();
            size_filter_fields(ui, &mut f);
        }
    });
    // Sortierung
    let sr = Rect::from_min_size(pos2(kr.right() + sp::XS, r.top()), vec2(sort_w, r.height()));
    let mut sc = ui.new_child(egui::UiBuilder::new().max_rect(sr));
    crate::theme::field_widgets(&mut sc);
    let mut sort = s.sort;
    egui::ComboBox::from_id_salt("fp-batch-sort").selected_text(sort.label()).width(sort_w).truncate().show_ui(&mut sc, |ui| {
        for k in SortKey::ALL {
            ui.selectable_value(&mut sort, k, k.label());
        }
    })
    .response
    .on_hover_text(t("batch-ui-sort"));
    if pro && !f.regex.is_empty() && f.text.is_empty() {
        // Feld zeigt den regulären Ausdruck
    } else if pro && f.text.starts_with('/') && f.text.len() > 1 {
        // „/ausdruck“ = regulärer Ausdruck (Profi)
        f.regex = f.text[1..].to_string();
        f.text.clear();
    }
    if f != s.filter {
        s.filter = f;
        s.files_changed();
    }
    if sort != s.sort {
        s.sort = sort;
        s.files_changed();
    }
}

/// Profi: Felder „Dateigröße (MB) ab … bis …“ und „Längste Seite (px) ab … bis …“ in der Typ-Liste.
fn size_filter_fields(ui: &mut Ui, f: &mut Filter) {
    let theme = Theme::current(ui.ctx());
    let num = |ui: &mut Ui, id: &str, label: &str, v: &mut Option<f64>| {
        let buf_id = Id::new(("fp-batch-filter-num", id));
        let focused = ui.memory(|m| m.has_focus(buf_id));
        let cur = v.map(|x| super::params::format_number(x, false)).unwrap_or_default();
        let mut buf = if focused { ui.ctx().data(|m| m.get_temp::<String>(buf_id.with("b"))).unwrap_or(cur.clone()) } else { cur };
        let r = ui.add(egui::TextEdit::singleline(&mut buf).id(buf_id).desired_width(52.0).horizontal_align(egui::Align::RIGHT));
        r.widget_info(|| super::params::text_info(label));
        if r.changed() {
            *v = super::params::parse_number(&buf, None).filter(|x| *x >= 0.0);
            ui.ctx().data_mut(|m| m.insert_temp(buf_id.with("b"), buf.clone()));
        } else if !r.has_focus() {
            ui.ctx().data_mut(|m| m.remove::<String>(buf_id.with("b")));
        }
    };
    let from = t("batch-ui-filter-min");
    let to = t("batch-ui-filter-max");
    egui::Grid::new("fp-batch-filter-size-grid").num_columns(5).spacing(vec2(sp::XS + 2.0, sp::XS)).show(ui, |ui| {
        let size_l = t("batch-ui-filter-size");
        ui.label(egui::RichText::new(&size_l).color(kit::weak(&theme)));
        ui.label(&from);
        num(ui, "min-mb", &format!("{size_l} {from}"), &mut f.min_mb);
        ui.label(&to);
        num(ui, "max-mb", &format!("{size_l} {to}"), &mut f.max_mb);
        ui.end_row();
        let px_l = t("batch-ui-filter-pixels");
        ui.label(egui::RichText::new(&px_l).color(kit::weak(&theme)));
        let mut lo = f.min_px.map(|x| x as f64);
        let mut hi = f.max_px.map(|x| x as f64);
        ui.label(&from);
        num(ui, "min-px", &format!("{px_l} {from}"), &mut lo);
        ui.label(&to);
        num(ui, "max-px", &format!("{px_l} {to}"), &mut hi);
        f.min_px = lo.map(|x| x.round() as u32);
        f.max_px = hi.map(|x| x.round() as u32);
        ui.end_row();
    });
    if f.has_size_filter() && ui.button(t("batch-ui-filter-reset")).clicked() {
        f.min_mb = None;
        f.max_mb = None;
        f.min_px = None;
        f.max_px = None;
    }
}

fn empty_state(ui: &mut Ui, st: &mut AppState, s: &mut Studio, r: Rect, enabled: bool) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let p = ui.painter().clone();
    if r.height() < 40.0 {
        return;
    }
    let zone = r.shrink(2.0);
    p.rect_filled(zone, CornerRadius::same(8), if theme.glow { with_alpha(c.focus, 10) } else { mix(kit::card_fill(&theme), c.canvas_bg, 0.25) });
    kit::dashed_rect(&p, zone, if s.drop_hover { c.focus } else { mix(c.line_mid, kit::card_fill(&theme), 0.2) });
    let title = t(if s.drop_hover { "batch-ui-files-drop" } else { "batch-ui-files-empty" });
    let hint = t("batch-ui-files-empty-hint");
    let w = (zone.width() - 2.0 * sp::M).max(60.0);
    let tf = theme.bold(type_scale::BODY + 1.0);
    let hf = theme.font(type_scale::CAPTION);
    let tg = ui.ctx().fonts_mut(|f| {
        let mut job = egui::text::LayoutJob::simple(title.clone(), tf.clone(), c.text, w);
        job.halign = egui::Align::Center;
        f.layout_job(job)
    });
    let hg = ui.ctx().fonts_mut(|f| {
        let mut job = egui::text::LayoutJob::simple(hint.clone(), hf.clone(), kit::weak(&theme), w);
        job.halign = egui::Align::Center;
        f.layout_job(job)
    });
    let add_l = t("batch-ui-files-add");
    let folder_l = t("batch-ui-files-add-folder");
    let bw = kit::btn_w(ui.ctx(), &add_l, true).max(kit::btn_w(ui.ctx(), &folder_l, true));
    let two_rows = 2.0 * bw + sp::S > w;
    let btn_block = if two_rows { 2.0 * kit::BTN_H + sp::XS } else { kit::BTN_H };
    let icon = 48.0;
    let total = icon + sp::M + tg.size().y + sp::XS + hg.size().y + sp::L + btn_block;
    let mut y = zone.center().y - total / 2.0;
    y = y.max(zone.top() + sp::S);
    let ir = Rect::from_center_size(pos2(zone.center().x, y + icon / 2.0), vec2(icon, icon));
    kit::paint_icon_mono(&p, ir, RowIcon::Kit(KitIcon::Image), if s.drop_hover { c.focus } else { mix(c.icon_light, c.frame_bg, 0.15) }, &theme);
    y = ir.bottom() + sp::M;
    let th = tg.size().y;
    p.galley(pos2(zone.center().x, y), tg, c.text);
    y += th + sp::XS;
    let hh = hg.size().y;
    p.galley(pos2(zone.center().x, y), hg, kit::weak(&theme));
    y += hh + sp::L;
    let (r1, r2) = if two_rows {
        (Rect::from_center_size(pos2(zone.center().x, y + kit::BTN_H / 2.0), vec2(bw.min(w), kit::BTN_H)), Rect::from_center_size(pos2(zone.center().x, y + kit::BTN_H * 1.5 + sp::XS), vec2(bw.min(w), kit::BTN_H)))
    } else {
        (Rect::from_min_size(pos2(zone.center().x - bw - sp::XS, y), vec2(bw, kit::BTN_H)), Rect::from_min_size(pos2(zone.center().x + sp::XS, y), vec2(bw, kit::BTN_H)))
    };
    if r1.bottom() <= zone.bottom() + 0.5 && kit::button(ui, r1, Id::new("fp-batch-empty-files"), &add_l, Some(RowIcon::Icon(crate::icons::Icon::Plus)), BtnKind::Primary, enabled, &add_l).clicked() {
        pick_files(st, s);
    }
    if r2.bottom() <= zone.bottom() + 0.5 && kit::button(ui, r2, Id::new("fp-batch-empty-folder"), &folder_l, Some(RowIcon::Icon(crate::icons::Icon::Folder)), BtnKind::Secondary, enabled, &folder_l).on_hover_text(folder_tooltip(s.recursive)).clicked() {
        let rec = s.recursive;
        pick_folder(st, s, rec);
    }
}

#[allow(clippy::too_many_arguments)]
fn file_list(ui: &mut Ui, s: &mut Studio, r: Rect, vis: &[usize], theme: &Theme, now: f64, enabled: bool) {
    if r.height() < 8.0 {
        return;
    }
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r).layout(egui::Layout::top_down(egui::Align::Min)));
    child.set_clip_rect(r.intersect(ui.clip_rect()));
    let mut area = egui::ScrollArea::vertical().id_salt("fp-batch-file-list").auto_shrink([false, false]);
    if s.scroll_to_selected {
        s.scroll_to_selected = false;
        if let Some(pos) = s.selected.and_then(|sel| vis.iter().position(|&i| i == sel)) {
            let off = pos as f32 * ROW_H;
            let cur = child.ctx().data(|d| d.get_temp::<f32>(Id::new("fp-batch-file-list-off"))).unwrap_or(0.0);
            if off < cur || off + ROW_H > cur + r.height() {
                area = area.vertical_scroll_offset((off - r.height() / 2.0).max(0.0));
            }
        }
    }
    let mut clicked: Option<usize> = None;
    let mut toggled: Option<usize> = None;
    let mut ffmpeg_click = false;
    let ffmpeg_missing = vis.iter().any(|&i| s.files[i].info.kind == ItemKind::Video) && !s.ffmpeg_available(now);
    let out = area.show_rows(&mut child, ROW_H, vis.len(), |ui, range| {
        for k in range {
            let i = vis[k];
            let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_H), Sense::hover());
            let e = &s.files[i];
            let (row_click, box_click, ff) = file_row(ui, row, i, e, s.selected == Some(i), theme, now, enabled, ffmpeg_missing);
            ffmpeg_click |= ff;
            if row_click {
                clicked = Some(i);
            }
            if box_click {
                toggled = Some(i);
            }
        }
    });
    child.ctx().data_mut(|d| d.insert_temp(Id::new("fp-batch-file-list-off"), out.state.offset.y));
    if let Some(i) = clicked {
        s.selected = Some(i);
    }
    if let Some(i) = toggled {
        s.files[i].enabled = !s.files[i].enabled;
        s.files_changed();
    }
    if ffmpeg_click {
        // ffmpeg fehlt: Batch-Einstellungen mit Abschnitt „Videos (ffmpeg)“ (BATCH-11b)
        super::settings::open(ui.ctx(), s);
    }
}

/// Eine Zeile: Kästchen, Miniatur 36 px, Name, Maße · Größe · Format, Zustand rechts.
#[allow(clippy::too_many_arguments)]
fn file_row(ui: &mut Ui, rect: Rect, i: usize, e: &FileEntry, selected: bool, theme: &Theme, now: f64, enabled: bool, ffmpeg_missing: bool) -> (bool, bool, bool) {
    let c = &theme.colors;
    let name = e.info.file_name();
    let resp = ui.interact(rect, Id::new(("fp-batch-file", i)), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, &name));
    let hot = resp.hovered() || resp.has_focus();
    let p = ui.painter();
    let cr = CornerRadius::same(5);
    if selected {
        p.rect_filled(rect, cr, if theme.glow { with_alpha(c.focus, 40) } else { kit::tint(theme, c.focus, 1.3) });
        if theme.glow {
            p.rect_stroke(rect, cr, Stroke::new(1.0, with_alpha(c.focus, 160)), StrokeKind::Inside);
        }
    } else if hot {
        p.rect_filled(rect, cr, if theme.glow { with_alpha(c.focus, 22) } else { c.list_hover_bg });
    }
    if resp.has_focus() {
        p.rect_stroke(rect, cr, Stroke::new(1.5, c.focus), StrokeKind::Inside);
    }
    // Kästchen
    let cb = Rect::from_center_size(pos2(rect.left() + sp::XS + 7.0, rect.center().y), vec2(14.0, 14.0));
    let cresp = ui.interact(cb.expand(3.0), Id::new(("fp-batch-file-on", i)), if enabled { Sense::click() } else { Sense::hover() });
    let on_label = fp_i18n::tr!("batch-ui-file-enabled", name = name.clone());
    cresp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, enabled, e.enabled, &on_label));
    p.rect_filled(cb, CornerRadius::same(3), if e.enabled { if theme.glow { with_alpha(c.focus, 90) } else { c.focus } } else { c.field_bg });
    p.rect_stroke(cb, CornerRadius::same(3), Stroke::new(1.0, if e.enabled { c.focus } else { c.field_border }), StrokeKind::Inside);
    if e.enabled {
        let s = Stroke::new(1.6, if theme.glow { c.text } else { Color32::WHITE });
        p.line_segment([cb.left_center() + vec2(3.0, 0.0), cb.center_bottom() + vec2(-1.0, -3.5)], s);
        p.line_segment([cb.center_bottom() + vec2(-1.0, -3.5), cb.right_top() + vec2(-3.0, 3.5)], s);
    }
    // Miniatur
    let bx = Rect::from_min_size(pos2(cb.right() + sp::S, rect.center().y - 18.0), vec2(36.0, 36.0));
    let box_bg = if theme.kind == ThemeKind::Normal { c.field_bg } else { mix(kit::card_fill(theme), c.field_bg, 0.6) };
    p.rect_filled(bx, CornerRadius::same(4), box_bg);
    let thumb = if ui.is_rect_visible(rect) { super::thumbs::get(ui.ctx(), &e.info.path, e.info.modified, e.info.bytes) } else { None };
    match thumb {
        Some(tex) => {
            let sz = tex.size_vec2();
            let sc = (34.0 / sz.x.max(1.0)).min(34.0 / sz.y.max(1.0));
            let ir = Rect::from_center_size(bx.center(), sz * sc);
            if ir.width() >= 8.0 && ir.height() >= 8.0 {
                crate::widgets::checker(p, ir, 4.0, theme);
            }
            p.image(tex.id(), ir, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        }
        None => {
            let icon = match e.info.kind {
                ItemKind::Video => RowIcon::Icon(crate::icons::Icon::Play),
                ItemKind::Document => RowIcon::Kit(KitIcon::FwDocument),
                ItemKind::Other => RowIcon::Icon(crate::icons::Icon::Document),
                _ => RowIcon::Kit(KitIcon::Image),
            };
            kit::paint_icon(p, Rect::from_center_size(bx.center(), vec2(20.0, 20.0)), icon, theme, true);
        }
    }
    p.rect_stroke(bx, CornerRadius::same(4), Stroke::new(1.0, if theme.glow { with_alpha(c.focus, 90) } else { mix(c.line_mid, box_bg, 0.55) }), StrokeKind::Inside);
    // Abzeichen: Animation/Video
    let badge = match e.info.kind {
        ItemKind::Animation if e.info.frames > 1 => Some(fp_i18n::tr!("batch-ui-badge-frames", n = e.info.frames)),
        ItemKind::Video => Some("▶".to_string()),
        _ => None,
    };
    if let Some(b) = badge {
        let f = theme.font(9.0);
        let bw = ui_kit::text_w(ui.ctx(), &b, &f) + 6.0;
        let br = Rect::from_min_size(pos2(bx.right() - bw.min(36.0), bx.bottom() - 12.0), vec2(bw.min(36.0), 12.0));
        p.rect_filled(br, CornerRadius::same(3), with_alpha(Color32::BLACK, 170));
        let shown = ui_kit::elide(ui.ctx(), &b, &f, br.width() - 2.0);
        p.text(br.center(), Align2::CENTER_CENTER, shown, f, Color32::WHITE);
    }
    // Name und Angaben
    let status_w = 18.0;
    let x = bx.right() + sp::S + 2.0;
    let tw = (rect.right() - sp::XS - status_w - x).max(20.0);
    let name_font = theme.font(type_scale::BODY + 0.5);
    let name_col = if !e.enabled { c.text_disabled } else if hot && !theme.glow && !selected { c.menu_hover_text } else { c.text };
    let shown = ui_kit::elide_middle(ui.ctx(), &name, &name_font, tw);
    p.text(pos2(x, rect.center().y - 8.0), Align2::LEFT_CENTER, &shown, name_font, name_col);
    let mut meta = Vec::new();
    if e.info.width > 0 {
        meta.push(format!("{}×{}", e.info.width, e.info.height));
    }
    meta.push(fp_i18n::fmt_size(e.info.bytes));
    if !e.info.format.is_empty() {
        meta.push(e.info.format.to_uppercase());
    }
    let rel = e.info.rel_dir();
    if !rel.as_os_str().is_empty() {
        meta.push(rel.display().to_string());
    }
    let f = theme.font(type_scale::CAPTION);
    let mut ffmpeg_click = false;
    if ffmpeg_missing && e.info.kind == ItemKind::Video {
        // ffmpeg fehlt: Hinweis statt der Angaben, Klick öffnet die Einstellungen (BATCH-11b)
        let txt = format!("⚠ {}", t("batch-ui-ffmpeg-missing-row"));
        let shown = ui_kit::elide(ui.ctx(), &txt, &f, tw);
        let lw = ui_kit::text_w(ui.ctx(), &shown, &f);
        let lr = Rect::from_min_size(pos2(x, rect.center().y + 2.0), vec2(lw.min(tw), 14.0));
        let lresp = ui.interact(lr, Id::new(("fp-batch-file-ffmpeg", i)), if enabled { Sense::click() } else { Sense::hover() });
        let choose = t("batch-ui-ffmpeg-choose");
        lresp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Link, enabled, &choose));
        let warn = if theme.kind == ThemeKind::Normal { mix(c.icon_yellow, Color32::BLACK, 0.4) } else { c.icon_yellow };
        let col = if lresp.hovered() { c.focus } else { warn };
        ui.painter().text(pos2(x, rect.center().y + 9.0), Align2::LEFT_CENTER, &shown, f.clone(), col);
        if lresp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            ui.painter().line_segment([pos2(x, lr.bottom() - 1.0), pos2(x + lw.min(tw), lr.bottom() - 1.0)], Stroke::new(1.0, col));
        }
        ffmpeg_click = lresp.on_hover_text(format!("{choose}\n{}", t("batch-ui-ffmpeg-choose.tooltip"))).clicked();
    } else {
        let meta = ui_kit::elide(ui.ctx(), &meta.join(" · "), &f, tw);
        p.text(pos2(x, rect.center().y + 9.0), Align2::LEFT_CENTER, meta, f, if e.enabled { kit::weak(theme) } else { c.text_disabled });
    }
    let p = ui.painter();
    kit::status_mark(p, pos2(rect.right() - sp::XS - 8.0, rect.center().y), e.status, theme, now);
    let mut tip = e.info.path.display().to_string();
    if let Some(m) = &e.message {
        tip = format!("{tip}\n{m}");
    }
    if let Some(tg) = &e.target {
        tip = format!("{tip}\n→ {}", tg.display());
    }
    let resp = resp.on_hover_text(tip);
    let box_click = cresp.clicked();
    (resp.clicked() && !box_click && !ffmpeg_click, box_click, ffmpeg_click)
}

fn footer(ui: &mut Ui, s: &mut Studio, r: Rect, total_bytes: u64, enabled: bool) {
    let theme = Theme::current(ui.ctx());
    let p = ui.painter().clone();
    let rem_l = t("batch-ui-files-remove");
    let clr_l = t("batch-ui-files-clear");
    let wr = kit::btn_w(ui.ctx(), &rem_l, false);
    let wc = kit::btn_w(ui.ctx(), &clr_l, false);
    let rc = Rect::from_min_size(pos2(r.right() - wc, r.top()), vec2(wc, r.height()));
    let rr = Rect::from_min_size(pos2(rc.left() - sp::XS - wr, r.top()), vec2(wr, r.height()));
    let text_w = (rr.left() - sp::S - r.left()).max(0.0);
    if text_w > 20.0 {
        let videos = s.files.iter().filter(|f| f.info.kind == ItemKind::Video).count();
        let mut txt = fp_i18n::fmt_size(total_bytes);
        if videos > 0 {
            txt = format!("{txt} · {}", fp_i18n::tr!("batch-ui-files-videos", n = videos));
        }
        kit::text_left(&p, r.left(), r.center().y, text_w, &txt, theme.font(type_scale::CAPTION), kit::weak(&theme));
    }
    if kit::button(ui, rr, Id::new("fp-batch-files-remove"), &rem_l, None, BtnKind::Ghost, enabled && s.selected.is_some(), &rem_l).on_hover_text(t("batch-ui-files-remove.tooltip")).clicked()
        && let Some(sel) = s.selected
    {
        s.remove_files(&[sel]);
    }
    if kit::button(ui, rc, Id::new("fp-batch-files-clear"), &clr_l, None, BtnKind::Ghost, enabled, &clr_l).clicked() {
        s.clear_files();
    }
}
