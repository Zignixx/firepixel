//! Pop-up-Menü-Code (15 §3.5) gegen die Ausgabe der Original-Vorlagenfunktionen für 300 Zufallsmenüs
//! (`..\re\verhalten\_scratch\menus_in.json` / `menus_xtt.json`, erzeugt von `test_fwbehavior.py` per Node).
//! Aufruf: `cargo test -p fp-export --test popup_menus -- --ignored --nocapture`.

mod common;

use fp_export::popup::{menus_js, MenuDate, MenuItemView, MenuView};
use serde_json::Value;

fn items(v: &Value) -> Vec<MenuItemView> {
    v.as_array()
        .map(|a| {
            a.iter()
                .map(|i| MenuItemView {
                    text: i["text"].as_str().unwrap_or("").to_string(),
                    url: i["url"].as_str().unwrap_or("").to_string(),
                    target: i["target"].as_str().unwrap_or("").to_string(),
                    max_text_width: i["maxTextWidth"].as_f64().unwrap_or(120.0),
                    submenu: items(&i["subMenu"]),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn menu(m: &Value) -> MenuView {
    let f = |k: &str| m[k].as_f64().unwrap_or(0.0);
    let bo = |k: &str| m[k].as_bool().unwrap_or(false);
    let s = |k: &str| m[k].as_str().unwrap_or("").to_string();
    let d = &m["creationDate"];
    let di = |i: usize| d[i].as_i64().unwrap_or(0);
    MenuView {
        items: items(&m["menuItems"]),
        creation: MenuDate { year: di(0) as i32, month: di(1) as u32, day: di(2) as u32, hour: di(3) as u32, minute: di(4) as u32, second: di(5) as u32 },
        menu_id: m["menuID"].as_i64().unwrap_or(0),
        event: m["event"].as_i64().unwrap_or(0) as i32,
        menu_width: f("menuWidth"),
        menu_height: f("menuHeight"),
        text_only: bo("textOnly"),
        override_width: bo("overrideMenuWidth"),
        override_height: bo("overrideMenuHeight"),
        padding: f("menuItemPadding"),
        spacing: f("menuItemSpacing"),
        text_indent: f("textIndent"),
        border_size: f("borderSize"),
        vertical: bo("vertical"),
        submenu_relative: bo("submenuRelativeToItem"),
        horz_submenu_offset: f("horzSubmenuOffset"),
        vert_submenu_offset: f("vertSubmenuOffset"),
        horz_offset: f("horzOffset"),
        vert_offset: f("vertOffset"),
        hide_timeout: f("hideTimeout"),
        opaque: bo("opaqueBackground"),
        family: s("textFamily"),
        size: f("textSize"),
        text_up: s("textUpColor"),
        text_over: s("textOverColor"),
        cell_up: s("cellUpColor"),
        cell_over: s("cellOverColor"),
        align: s("textAlignment"),
        valign: s("vertAlignment"),
        bold: bo("textBold"),
        italic: bo("textItalic"),
        hilite: s("hiliteColor"),
        shadow: s("shadowColor"),
        border: s("borderColor"),
        image_path: s("menuImagePath"),
        image_path2: s("menuImagePath2"),
    }
}

#[test]
#[ignore]
fn popup_menus_300() {
    let dir = common::re_dir().join("verhalten").join("_scratch");
    let (Ok(inp), Ok(refs)) = (std::fs::read_to_string(dir.join("menus_in.json")), std::fs::read_to_string(dir.join("menus_xtt.json"))) else {
        eprintln!("menus_*.json fehlen");
        return;
    };
    let cases: Value = serde_json::from_str(&inp).expect("menus_in.json");
    let refs: Value = serde_json::from_str(&refs).expect("menus_xtt.json");
    let (mut css, mut html, mut js, mut n) = (0, 0, 0, 0);
    let mut first: Option<String> = None;
    for (c, r) in cases.as_array().unwrap().iter().zip(refs.as_array().unwrap()) {
        let m = menu(&c["menu"]);
        let got_css = m.css(c["sliceLeft"].as_f64().unwrap_or(0.0), c["sliceTop"].as_f64().unwrap_or(0.0), "images/");
        let got_html = m.html(c["xhtml"].as_bool().unwrap_or(false), "images/arrows.gif");
        let got_js = menus_js(std::slice::from_ref(&m), "images/", "images/arrows.gif");
        n += 1;
        for (k, got, cnt) in [("css", got_css, &mut css), ("html", got_html, &mut html), ("js", got_js, &mut js)] {
            let want = r[k].as_str().unwrap_or("");
            if got == want {
                *cnt += 1;
            } else if first.is_none() {
                let p = got.chars().zip(want.chars()).position(|(a, b)| a != b).unwrap_or(got.len().min(want.len()));
                let g: String = got.chars().skip(p.saturating_sub(60)).take(140).collect();
                let w: String = want.chars().skip(p.saturating_sub(60)).take(140).collect();
                first = Some(format!("Fall {n} {k}:\n  Rust: {g:?}\n  XTT : {w:?}"));
            }
        }
    }
    if let Some(f) = &first {
        println!("erste Abweichung: {f}");
    }
    println!("Pop-up-Menüs: CSS {css}/{n}, HTML {html}/{n}, JS {js}/{n}");
    assert_eq!((css, html, js), (n, n, n));
    assert_eq!(n, 300);
}
