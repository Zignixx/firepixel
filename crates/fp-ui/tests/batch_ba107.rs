//! BA-107: Batch Studio UI gaps – Pro filter bar mirrored into the recipe step `file.filterSkip`
//! (file size, width/height, transparency, name pattern; rows hidden live), manual order by drag
//! and Ctrl+Up/Down (kept in the recipe), keys in the file list (Space, Enter, Delete, Ctrl+A,
//! focus ring), card menu "Save as Template…" / "Help", per-file progress and ETA.
//! Realistic input: clicks, typing character by character, key presses.

use egui::epaint::Shape;
use egui::{vec2, Event, Key, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::kittest::{By, Queryable};
use egui_kittest::Harness;
use fp_i18n::t;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::batch::{self, Mode};
use fp_ui::theme::ThemeKind;
use fp_ui::FirepixelApp;
use serde_json::json;
use std::path::{Path, PathBuf};

// ------------------------------------------------------------------ helpers

fn dir(name: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let k = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let d = std::env::temp_dir().join("firepixel-ba107").join(std::process::id().to_string()).join(format!("{name}-{k}"));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("folder");
    d
}

/// PNG file; `alpha` = right quarter transparent.
fn png_file(d: &Path, name: &str, w: u32, h: u32, alpha: bool) -> PathBuf {
    let p = d.join(name);
    let mut px = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let a = if alpha && x * 4 >= w * 3 { 0 } else { 255 };
            px.extend_from_slice(&[(x * 255 / w.max(1)) as u8, (y * 255 / h.max(1)) as u8, 160, a]);
        }
    }
    let f = std::fs::File::create(&p).expect("file");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().and_then(|mut wr| wr.write_image_data(&px)).expect("PNG");
    p
}

fn state() -> AppState {
    fp_engine::shell::disable();
    let session = fp_ui::ui_commands::new_session(fp_engine::Preferences::new());
    let mut st = AppState::new(session, UiSettings::default(), None);
    st.sync_render = true;
    st.os_clipboard = false;
    st.set_theme(ThemeKind::Normal);
    st
}

fn app(st: AppState) -> FirepixelApp {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, fp_engine::Session::default(), UiSettings::default(), false);
    app.state = st;
    app
}

type H = Harness<'static, FirepixelApp>;

fn harness(st: AppState) -> H {
    let b = Harness::builder().renderer(crate::gpu_common::renderer()).with_size(vec2(1440.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(30).with_step_dt(1.0 / 60.0);
    let mut hn = b.build_ui_state(|ui, a: &mut FirepixelApp| a.root_ui(ui), app(st));
    hn.run_steps(4);
    hn
}

fn st(h: &mut H) -> &mut AppState {
    &mut h.state_mut().state
}

fn studio(h: &mut H) -> &mut batch::Studio {
    batch::studio_mut(st(h)).expect("studio open")
}

fn settle(h: &mut H) {
    for _ in 0..900 {
        h.run_steps(1);
        let busy = batch::studio(&h.state().state).is_some_and(|s| s.preview.busy());
        if !busy && !batch::thumbnails_pending(&h.ctx) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    h.run_steps(3);
}

fn texts(h: &H) -> Vec<(String, Rect)> {
    fn walk(s: &Shape, out: &mut Vec<(String, Rect)>) {
        match s {
            Shape::Text(t) => {
                let text = t.galley.text().to_string();
                if !text.trim().is_empty() {
                    out.push((text, t.visual_bounding_rect()));
                }
            }
            Shape::Vec(v) => v.iter().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    for cs in &h.output().shapes {
        walk(&cs.shape, &mut out);
    }
    out
}

fn find_text(h: &H, needle: &str) -> Option<Rect> {
    texts(h).into_iter().find(|(t, _)| t == needle).map(|(_, r)| r)
}

fn has_text(h: &H, needle: &str) -> bool {
    texts(h).iter().any(|(t, _)| t.contains(needle))
}

fn click(h: &mut H, p: Pos2) {
    h.event(Event::PointerMoved(p));
    h.run_steps(1);
    for pressed in [true, false] {
        h.event(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
    }
    h.run_steps(2);
}

fn key(h: &mut H, k: Key, m: Modifiers) {
    h.event(Event::Key { key: k, physical_key: None, pressed: true, repeat: false, modifiers: m });
    h.event(Event::Key { key: k, physical_key: None, pressed: false, repeat: false, modifiers: m });
    h.run_steps(2);
}

fn type_text(h: &mut H, s: &str) {
    for c in s.chars() {
        h.event(Event::Text(c.to_string()));
        h.run_steps(1);
    }
    h.run_steps(1);
}

fn node(h: &H, label: &str, role: egui::accesskit::Role) -> Option<Rect> {
    h.query_all(By::new().label(label).role(role)).next().map(|n| n.rect())
}

fn names(h: &mut H) -> Vec<String> {
    let s = studio(h);
    s.visible().iter().map(|&i| s.files[i].info.file_name()).collect()
}

/// Studio with the given files, Pro mode.
fn open(files: &[PathBuf]) -> H {
    let mut st = state();
    st.run("file.batchProcess", json!({}));
    let mut h = harness(st);
    studio(&mut h).add_paths(files, false);
    studio(&mut h).mode = Mode::Pro;
    settle(&mut h);
    h
}

fn filter_step(h: &mut H) -> Option<(usize, serde_json::Value)> {
    studio(h).recipe.steps.iter().enumerate().find(|(_, s)| s.id == "file.filterSkip").map(|(i, s)| (i, s.params.clone()))
}

fn open_type_list(h: &mut H) {
    let kind = h.query_all(By::new().role(egui::accesskit::Role::ComboBox)).map(|n| n.rect()).next().expect("type list");
    click(h, kind.center());
}

// ------------------------------------------------------------------ 1. filters

#[test]
fn filter_bar_writes_filter_skip_and_hides_rows() {
    let d = dir("filter");
    let small = png_file(&d, "small_a.png", 64, 48, false);
    let wide = png_file(&d, "wide_a.png", 400, 100, false);
    let tall = png_file(&d, "tall_b.png", 100, 400, true);
    let mut h = open(&[small, wide, tall]);
    // an existing step stays below the inserted filter step
    studio(&mut h).edit("add", |r| {
        r.add_step("resize", json!({}));
        true
    });
    studio(&mut h).end_gesture();
    assert_eq!(names(&mut h).len(), 3);
    open_type_list(&mut h);
    let from = t("batch-ui-filter-min");
    let to = t("batch-ui-filter-max");
    // width from 200, typed character by character
    let f = node(&h, &format!("{} {from}", t("batch-ui-filter-width")), egui::accesskit::Role::TextInput).expect("width from");
    click(&mut h, f.center());
    type_text(&mut h, "200");
    assert_eq!(studio(&mut h).filter.min_w, Some(200));
    assert_eq!(names(&mut h), ["wide_a.png"], "rows hidden live");
    let (i, p) = filter_step(&mut h).expect("filter step in the recipe");
    assert_eq!(i, 0, "inserted at the top");
    assert_eq!(p["minWidth"], json!(200));
    assert_eq!(p["action"], json!("skip"));
    assert_eq!(studio(&mut h).recipe.steps.len(), 2);
    // three typed characters = one undo step
    assert_eq!(studio(&mut h).undo_len(), 2);
    // height to 150 and size range
    let f = node(&h, &format!("{} {to}", t("batch-ui-filter-height")), egui::accesskit::Role::TextInput).expect("height to");
    click(&mut h, f.center());
    type_text(&mut h, "150");
    assert_eq!(filter_step(&mut h).expect("step").1["maxHeight"], json!(150));
    assert_eq!(names(&mut h), ["wide_a.png"]);
    let f = node(&h, &format!("{} {to}", t("batch-ui-filter-size")), egui::accesskit::Role::TextInput).expect("size to");
    click(&mut h, f.center());
    type_text(&mut h, "0,01");
    assert_eq!(filter_step(&mut h).expect("step").1["maxKB"], json!(0.01));
    assert!(names(&mut h).is_empty(), "no file is that small");
    assert!(has_text(&h, &fp_i18n::tr!("batch-ui-files-count-filtered", shown = 0, total = 3)));
    // reset: the step held only filter conditions → removed again
    let reset = find_text(&h, &t("batch-ui-filter-reset")).unwrap_or_else(|| panic!("reset {:?}", texts(&h).iter().map(|x| x.0.clone()).collect::<Vec<_>>()));
    click(&mut h, reset.center());
    assert!(filter_step(&mut h).is_none(), "step removed with the last condition");
    assert_eq!(names(&mut h).len(), 3);
    assert_eq!(studio(&mut h).recipe.steps[0].id, "resize");
    // undo brings the conditions back (recipe history), also into the filter bar
    key(&mut h, Key::Escape, Modifiers::NONE);
    key(&mut h, Key::Z, Modifiers::COMMAND);
    assert!(filter_step(&mut h).is_some());
    assert_eq!(studio(&mut h).filter.min_w, Some(200));
    assert!(names(&mut h).is_empty());
}

#[test]
fn transparency_and_name_pattern_filter() {
    let d = dir("alpha");
    let opaque = png_file(&d, "logo_opaque.png", 40, 40, false);
    let clear = png_file(&d, "logo_clear.png", 40, 40, true);
    let other = png_file(&d, "photo.png", 40, 40, true);
    let mut h = open(&[opaque, clear, other]);
    open_type_list(&mut h);
    let with = find_text(&h, &t("batch-ui-filter-alpha-with")).expect("with transparency");
    click(&mut h, with.center());
    settle(&mut h);
    assert_eq!(filter_step(&mut h).expect("step").1["transparency"], json!("with"));
    assert_eq!(names(&mut h), ["logo_clear.png", "photo.png"]);
    let without = find_text(&h, &t("batch-ui-filter-alpha-without")).expect("without");
    click(&mut h, without.center());
    settle(&mut h);
    assert_eq!(names(&mut h), ["logo_opaque.png"]);
    let any = find_text(&h, &t("batch-ui-filter-alpha-any")).expect("any");
    click(&mut h, any.center());
    // name pattern with alternatives
    let g = node(&h, &t("batch-ui-filter-glob"), egui::accesskit::Role::TextInput).expect("name pattern");
    click(&mut h, g.center());
    type_text(&mut h, "LOGO_C*; pho?o.*");
    assert_eq!(filter_step(&mut h).expect("step").1["name"], json!("LOGO_C*; pho?o.*"));
    assert_eq!(names(&mut h), ["logo_clear.png", "photo.png"]);
    assert_eq!(studio(&mut h).to_process().len(), 2, "hidden files are not processed");
}

#[test]
fn loaded_filter_step_fills_the_filter_bar() {
    let d = dir("load");
    let a = png_file(&d, "a.png", 300, 300, false);
    let b = png_file(&d, "b.png", 30, 30, false);
    let mut h = open(&[a, b]);
    let mut r = fp_batch::Recipe::default();
    r.add_step("file.filterSkip", json!({"minWidth": 100, "transparency": "without", "name": "*.png", "action": "copy"}));
    studio(&mut h).set_recipe(r, None);
    h.run_steps(2);
    let f = studio(&mut h).filter.clone();
    assert_eq!((f.min_w, f.transparency.key(), f.glob.as_str()), (Some(100), "without", "*.png"));
    assert_eq!(names(&mut h), ["a.png"]);
    // editing in the bar keeps the step's own parameters (action copy)
    let mut f2 = studio(&mut h).filter.clone();
    f2.min_w = Some(10);
    studio(&mut h).set_pro_filter(&f2);
    let p = filter_step(&mut h).expect("step").1;
    assert_eq!((p["minWidth"].clone(), p["action"].clone()), (json!(10), json!("copy")));
    // clearing keeps a step with non-default parameters
    let mut f3 = f2.clone();
    f3.clear_pro();
    studio(&mut h).set_pro_filter(&f3);
    assert!(filter_step(&mut h).is_some(), "action copy is kept");
    // switching the step off releases the rows
    studio(&mut h).edit("toggle", |r| {
        r.steps[0].enabled = false;
        true
    });
    assert_eq!(names(&mut h).len(), 2);
}

// ------------------------------------------------------------------ 2. manual order

fn four_files(tag: &str) -> (H, Vec<PathBuf>) {
    let d = dir(tag);
    let files: Vec<PathBuf> = ["a.png", "b.png", "c.png", "d.png"].iter().map(|n| png_file(&d, n, 40, 30, false)).collect();
    let h = open(&files);
    (h, files)
}

fn grips(h: &H) -> Vec<Rect> {
    let mut v: Vec<Rect> = h.query_all(By::new().label(&t("batch-ui-file-drag"))).map(|n| n.rect()).collect();
    v.sort_by(|a, b| a.top().total_cmp(&b.top()));
    v
}

fn drag(h: &mut H, from: Pos2, to: Pos2) {
    h.event(Event::PointerMoved(from));
    h.run_steps(1);
    h.event(Event::PointerButton { pos: from, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.run_steps(1);
    for k in 1..=8 {
        let p = from + (to - from) * (k as f32 / 8.0);
        h.event(Event::PointerMoved(p));
        h.run_steps(1);
    }
    h.event(Event::PointerButton { pos: to, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(3);
}

fn order_list(h: &mut H) -> Vec<String> {
    studio(h).recipe.extra.get(batch::FILE_ORDER).and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default()
}

#[test]
fn drag_handle_reorders_and_keeps_order_in_recipe() {
    let (mut h, _) = four_files("drag");
    let g = grips(&h);
    assert_eq!(g.len(), 4, "a handle per row");
    // drag "a" below "c"
    let to = Pos2::new(g[0].center().x, g[2].bottom() + 2.0);
    drag(&mut h, g[0].center(), to);
    assert_eq!(names(&mut h), ["b.png", "c.png", "a.png", "d.png"]);
    assert_eq!(studio(&mut h).sort, batch::files::SortKey::Manual);
    assert_eq!(order_list(&mut h), ["b.png", "c.png", "a.png", "d.png"]);
    assert!(studio(&mut h).is_dirty(), "the order belongs to the recipe");
    // undo → previous order ("Added")
    key(&mut h, Key::Z, Modifiers::COMMAND);
    assert_eq!(names(&mut h), ["a.png", "b.png", "c.png", "d.png"]);
    assert!(order_list(&mut h).is_empty());
    key(&mut h, Key::Y, Modifiers::COMMAND);
    assert_eq!(names(&mut h), ["b.png", "c.png", "a.png", "d.png"]);
    // drag "d" to the top
    let g = grips(&h);
    drag(&mut h, g[3].center(), Pos2::new(g[0].center().x, g[0].top() + 3.0));
    assert_eq!(names(&mut h), ["d.png", "b.png", "c.png", "a.png"]);
}

#[test]
fn ctrl_arrows_move_the_selected_file() {
    let (mut h, files) = four_files("keys");
    studio(&mut h).selected = Some(0);
    h.run_steps(1);
    key(&mut h, Key::ArrowDown, Modifiers::COMMAND);
    key(&mut h, Key::ArrowDown, Modifiers::COMMAND);
    assert_eq!(names(&mut h), ["b.png", "c.png", "a.png", "d.png"]);
    assert_eq!(studio(&mut h).selected, Some(0), "the moved file stays selected");
    key(&mut h, Key::ArrowUp, Modifiers::COMMAND);
    assert_eq!(names(&mut h), ["b.png", "a.png", "c.png", "d.png"]);
    // at the edges nothing happens
    studio(&mut h).selected = Some(1);
    key(&mut h, Key::ArrowUp, Modifiers::COMMAND);
    assert_eq!(names(&mut h), ["b.png", "a.png", "c.png", "d.png"]);
    // saved and loaded with the files added afterwards: same order
    let path = dir("keys-recipe").join("order.fpbatch.json");
    studio(&mut h).recipe.save(&path).expect("save");
    let loaded = fp_batch::Recipe::load(&path).expect("load");
    assert_eq!(loaded.extra.get(batch::FILE_ORDER), studio(&mut h).recipe.extra.get(batch::FILE_ORDER));
    studio(&mut h).clear_files();
    studio(&mut h).set_recipe(fp_batch::Recipe::default(), None);
    assert_eq!(studio(&mut h).sort, batch::files::SortKey::Added);
    studio(&mut h).set_recipe(loaded, Some(path));
    studio(&mut h).add_paths(&files, false);
    assert_eq!(names(&mut h), ["b.png", "a.png", "c.png", "d.png"]);
    // another sort key leaves the manual order and removes it from the recipe
    studio(&mut h).set_sort(batch::files::SortKey::Name);
    assert!(order_list(&mut h).is_empty());
    assert_eq!(names(&mut h), ["a.png", "b.png", "c.png", "d.png"]);
}

// ------------------------------------------------------------------ 3. keys in the file list

/// Is a focus ring (1.5 px, focus colour) drawn around a file row?
fn focus_ring_rows(h: &H) -> usize {
    let focus = fp_ui::theme::Theme::current(&h.ctx).colors.focus;
    fn walk(s: &Shape, focus: egui::Color32, n: &mut usize) {
        match s {
            Shape::Rect(r) if (r.stroke.width - 1.5).abs() < 0.01 && r.stroke.color == focus && (r.rect.height() - batch::files::ROW_H).abs() < 0.5 => *n += 1,
            Shape::Vec(v) => v.iter().for_each(|x| walk(x, focus, n)),
            _ => {}
        }
    }
    let mut n = 0;
    for cs in &h.output().shapes {
        walk(&cs.shape, focus, &mut n);
    }
    n
}

#[test]
fn space_enter_delete_ctrl_a_and_focus_ring() {
    let (mut h, _) = four_files("keys3");
    let row_a = find_text(&h, "a.png").expect("row a");
    click(&mut h, row_a.center());
    assert_eq!(studio(&mut h).selected, Some(0));
    assert!(studio(&mut h).list_focus);
    assert_eq!(focus_ring_rows(&h), 1, "focus ring on the selected row");
    // Space switches the file off and on again
    key(&mut h, Key::Space, Modifiers::NONE);
    assert!(!studio(&mut h).files[0].enabled);
    assert_eq!(studio(&mut h).to_process().len(), 3);
    key(&mut h, Key::Space, Modifiers::NONE);
    assert!(studio(&mut h).files[0].enabled);
    // arrow down, Space on "b"
    key(&mut h, Key::ArrowDown, Modifiers::NONE);
    assert_eq!(studio(&mut h).selected, Some(1));
    key(&mut h, Key::Space, Modifiers::NONE);
    assert!(!studio(&mut h).files[1].enabled);
    // Ctrl+A switches all files on (never off)
    key(&mut h, Key::A, Modifiers::COMMAND);
    assert!(studio(&mut h).files.iter().all(|f| f.enabled));
    key(&mut h, Key::A, Modifiers::COMMAND);
    assert!(studio(&mut h).files.iter().all(|f| f.enabled));
    // Enter leaves the plan and shows the file in the preview
    studio(&mut h).view_tab = batch::preview::Tab::Plan;
    key(&mut h, Key::Enter, Modifiers::NONE);
    assert_eq!(studio(&mut h).view_tab, batch::preview::Tab::Compare);
    assert_eq!(studio(&mut h).narrow_tab, 2);
    // Delete removes the selected file
    key(&mut h, Key::Delete, Modifiers::NONE);
    assert_eq!(names(&mut h), ["a.png", "c.png", "d.png"]);
    // a click elsewhere takes the keyboard from the list: Space does nothing to the files
    let recipe_title = find_text(&h, &t("batch-ui-recipe-title")).expect("recipe card");
    click(&mut h, recipe_title.center() + vec2(0.0, 120.0));
    assert!(!studio(&mut h).list_focus);
    assert_eq!(focus_ring_rows(&h), 0);
    key(&mut h, Key::Space, Modifiers::NONE);
    assert!(studio(&mut h).files.iter().all(|f| f.enabled));
}

#[test]
fn enter_in_narrow_layout_switches_to_preview_tab() {
    let d = dir("narrow");
    let files: Vec<PathBuf> = ["x.png", "y.png"].iter().map(|n| png_file(&d, n, 40, 30, false)).collect();
    let mut st = state();
    st.run("file.batchProcess", json!({}));
    let b = Harness::builder().renderer(crate::gpu_common::renderer()).with_size(vec2(700.0, 700.0)).with_pixels_per_point(1.0).with_max_steps(30).with_step_dt(1.0 / 60.0);
    let mut h = b.build_ui_state(|ui, a: &mut FirepixelApp| a.root_ui(ui), app(st));
    h.run_steps(4);
    studio(&mut h).add_paths(&files, false);
    settle(&mut h);
    let row = find_text(&h, "y.png").expect("row y");
    click(&mut h, row.center());
    key(&mut h, Key::Enter, Modifiers::NONE);
    assert_eq!(studio(&mut h).narrow_tab, 2, "single column: tab Preview");
    assert!(find_text(&h, "y.png").is_none() || studio(&mut h).selected == Some(1));
}

// ------------------------------------------------------------------ 4. card menu

fn open_step_menu(h: &mut H) {
    let m = h.query_all(By::new().label(&t("batch-ui-step-menu"))).map(|n| n.rect()).next().expect("card menu ⋯");
    click(h, m.center());
}

#[test]
fn save_step_as_template_appears_in_template_list() {
    let d = dir("tpl");
    let mut h = open(&[png_file(&d, "a.png", 40, 30, false)]);
    studio(&mut h).edit("add", |r| {
        r.add_step("resize", json!({"mode": "longEdge", "size": 777}));
        true
    });
    studio(&mut h).end_gesture();
    h.run_steps(2);
    open_step_menu(&mut h);
    let item = find_text(&h, &t("batch-ui-step-save-template")).expect("menu entry");
    click(&mut h, item.center());
    assert!(studio(&mut h).template_prompt.is_some(), "name prompt open");
    assert!(has_text(&h, &t("batch-ui-template-title")));
    // Esc cancels without writing
    key(&mut h, Key::Escape, Modifiers::NONE);
    assert!(studio(&mut h).template_prompt.is_none());
    open_step_menu(&mut h);
    let item = find_text(&h, &t("batch-ui-step-save-template")).expect("menu entry");
    click(&mut h, item.center());
    h.run_steps(2);
    let name = format!("Web 777 {}", std::process::id());
    key(&mut h, Key::A, Modifiers::COMMAND);
    type_text(&mut h, &name);
    key(&mut h, Key::Enter, Modifiers::NONE);
    assert!(studio(&mut h).template_prompt.is_none(), "Enter saves");
    let file = batch::step_menu::template_dir().expect("folder").join(format!("{name}.fpbatch.json"));
    assert!(file.is_file(), "template written: {}", file.display());
    let r = fp_batch::Recipe::load(&file).expect("recipe");
    assert_eq!((r.name.as_str(), r.steps.len(), r.steps[0].params["size"].clone()), (name.as_str(), 1, json!(777)));
    assert!(has_text(&h, &fp_i18n::tr!("batch-ui-template-saved", name = name.clone())));
    // listed with the templates; choosing it loads the recipe
    let tpl = batch::templates().into_iter().find(|t| t.title == name).expect("in the template list");
    assert_eq!(tpl.recipe.steps[0].id, "resize");
    let menu = h.query_all(By::new().label(&t("batch-ui-recipes"))).map(|n| n.rect()).next().expect("Recipes ▾");
    click(&mut h, menu.center());
    let entry = find_text(&h, &name).expect("template in the Recipes menu");
    click(&mut h, entry.center());
    assert_eq!(studio(&mut h).recipe.name, name);
    let _ = std::fs::remove_file(file);
}

#[test]
fn help_names_the_section_of_the_step() {
    // the section lookup on the real documentation
    let (file, section) = batch::step_menu::help_section("file.filterSkip").expect("docs/BATCH.md found");
    assert!(file.ends_with("BATCH.md"));
    let section = section.expect("section");
    assert!(section.starts_with("3.1"), "{section}");
    assert!(batch::step_menu::help_section("resize").and_then(|x| x.1).is_some_and(|s| s.starts_with("3.3")));
    assert_eq!(batch::step_menu::section_for("# A\n## B\n| `x.y` | X |\n", "x.y").as_deref(), Some("B"));
    // the menu entry opens it (shell disabled in tests) and names the section
    let d = dir("help");
    let mut h = open(&[png_file(&d, "a.png", 40, 30, false)]);
    studio(&mut h).edit("add", |r| {
        r.add_step("file.filterSkip", json!({}));
        true
    });
    studio(&mut h).end_gesture();
    h.run_steps(2);
    open_step_menu(&mut h);
    // the main menu bar has a "Help" too: the menu entry is the last one drawn
    let item = texts(&h).into_iter().filter(|(x, _)| *x == t("batch-ui-step-help")).map(|(_, r)| r).next_back().expect("Help entry");
    click(&mut h, item.center());
    let n = studio(&mut h).notice.clone().expect("notice");
    assert!(n.text.contains(&section), "{}", n.text);
}

// ------------------------------------------------------------------ 5. per-file progress

#[test]
fn per_file_progress_eta_and_cancel() {
    use fp_batch::Progress;
    use fp_ui::batch::output_bar::{self, RunState};
    let d = dir("progress");
    let files: Vec<PathBuf> = ["p1.png", "p2.png", "p3.png"].iter().map(|n| png_file(&d, n, 40, 30, false)).collect();
    let mut h = open(&files);
    let map = studio(&mut h).to_process();
    studio(&mut h).run = Some(RunState::detached(map));
    output_bar::apply_events(studio(&mut h), vec![Progress::Started { total: 3 }, Progress::ItemStarted { index: 0 }, Progress::ItemProgress { index: 0, fraction: 0.42, step: String::new() }]);
    h.run_steps(2);
    assert_eq!(studio(&mut h).files[0].status, batch::EntryStatus::Running);
    let pct = batch::files::progress_label(0.42);
    assert!(has_text(&h, &pct), "row shows {pct}");
    let run = studio(&mut h).run.as_ref().expect("run");
    assert!((run.fraction() - 0.14).abs() < 0.001, "overall bar includes the running part");
    // the remaining time appears once a part is done
    std::thread::sleep(std::time::Duration::from_millis(30));
    output_bar::apply_events(studio(&mut h), vec![Progress::ItemProgress { index: 0, fraction: 0.6, step: String::new() }]);
    h.run_steps(2);
    assert!(studio(&mut h).run.as_ref().and_then(|r| r.remaining()).is_some(), "ETA estimated from partial progress");
    let remaining_word = fp_i18n::tr!("batch-ui-remaining", time = "0:00".to_string()).replace("0:00", "");
    assert!(has_text(&h, remaining_word.trim()), "ETA in the footer");
    // first file done, second running
    output_bar::apply_events(studio(&mut h), vec![Progress::ItemDone { index: 0, target: d.join("out1.png"), bytes_out: 10 }, Progress::ItemStarted { index: 1 }, Progress::ItemProgress { index: 1, fraction: 0.25, step: String::new() }]);
    h.run_steps(2);
    assert_eq!(studio(&mut h).files[0].status, batch::EntryStatus::Done);
    assert!(has_text(&h, &batch::files::progress_label(0.25)));
    assert!(!has_text(&h, &pct), "no percentage on a finished row");
    assert!(has_text(&h, &fp_i18n::tr!("batch-ui-progress", done = 1, total = 3)));
    // nonsense from the executor is clamped
    output_bar::apply_events(studio(&mut h), vec![Progress::ItemProgress { index: 1, fraction: 7.0, step: String::new() }, Progress::ItemProgress { index: 9, fraction: 0.5, step: String::new() }]);
    assert_eq!(studio(&mut h).files[1].progress, 1.0);
    // Cancel still works: the run ends, finished files keep their status
    let cancel = h.query_all(By::new().label(&t("batch-ui-cancel"))).map(|n| n.rect()).next().expect("Cancel");
    click(&mut h, cancel.center());
    h.run_steps(3);
    assert!(studio(&mut h).run.is_none(), "run ended");
    let st: Vec<_> = studio(&mut h).files.iter().map(|f| f.status).collect();
    assert_eq!(st, [batch::EntryStatus::Done, batch::EntryStatus::Cancelled, batch::EntryStatus::Cancelled]);
}

#[test]
fn real_run_resets_progress_and_finishes() {
    let d = dir("realrun");
    let files: Vec<PathBuf> = ["r1.png", "r2.png"].iter().map(|n| png_file(&d, n, 40, 30, false)).collect();
    let mut h = open(&files);
    studio(&mut h).files[0].progress = 0.7;
    let now = h.ctx.input(|i| i.time);
    batch::output_bar::start(studio(&mut h), now);
    assert_eq!(studio(&mut h).files[0].progress, 0.0, "a new run starts at 0");
    for _ in 0..600 {
        h.run_steps(1);
        if !studio(&mut h).running() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(!studio(&mut h).running());
    assert!(studio(&mut h).files.iter().all(|f| f.status == batch::EntryStatus::Done), "{:?}", studio(&mut h).files.iter().map(|f| (f.status, f.message.clone())).collect::<Vec<_>>());
}

// ------------------------------------------------------------------ remaining BA-107: "Read video" card

#[test]
fn adding_a_video_inserts_the_read_video_card() {
    let d = dir("video");
    let img = png_file(&d, "a.png", 40, 30, false);
    let v1 = d.join("clip.mp4");
    let v2 = d.join("clip2.mp4");
    std::fs::write(&v1, b"not a real video").expect("mp4");
    std::fs::write(&v2, b"not a real video").expect("mp4");
    let mut h = open(&[img]);
    assert!(studio(&mut h).recipe.steps.is_empty(), "no card for images");
    studio(&mut h).add_paths(std::slice::from_ref(&v1), false);
    h.run_steps(2);
    let ids: Vec<String> = studio(&mut h).recipe.steps.iter().map(|s| s.id.clone()).collect();
    assert_eq!(ids, ["video.import"]);
    assert_eq!(studio(&mut h).open_card, Some(batch::CardId::Step(0)), "card visible and open");
    assert!(has_text(&h, &fp_batch::step::step_label("video.import")));
    // a second video: no second card
    studio(&mut h).add_paths(std::slice::from_ref(&v2), false);
    assert_eq!(studio(&mut h).recipe.steps.len(), 1);
    // one undo step
    key(&mut h, Key::Z, Modifiers::COMMAND);
    assert!(studio(&mut h).recipe.steps.is_empty());
    // a recipe with file steps only (copy/rename) gets no card
    studio(&mut h).clear_files();
    let id = fp_batch::steps::files::COPY_ONLY;
    studio(&mut h).edit("add", |r| {
        r.add_step(id, json!({}));
        true
    });
    studio(&mut h).add_paths(&[v1, v2], false);
    assert_eq!(studio(&mut h).recipe.steps.len(), 1);
}
