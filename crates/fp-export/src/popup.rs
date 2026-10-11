//! Pop-up-Menüs als HTML/CSS/JavaScript (15 §3.5): Namen, Geometrie, CSS-Stylesheet, `<div>`-Baum
//! (CSS-Variante) und Rumpf von `mmLoadMenus()` (JS-Variante).
//!
//! Zeichengenau nach den Rechenregeln, die `re\verhalten\fwbehavior.py` aus den Original-Vorlagen
//! nachgebildet hat (300/300 Zufallsmenüs gleich der Originalausgabe, Spezifikation 15 §3.5). Eigene
//! Formulierung; die Laufzeit-Skripte (`fp_css_menu.js`, `fp_menu.js`) liefert [`crate::behavior`].

use fp_model::web::{parse_menu_items, PopupMenu};

/// Menüeintrag mit gemessener Textbreite.
#[derive(Clone, Debug, PartialEq)]
pub struct MenuItemView {
    pub text: String,
    pub url: String,
    pub target: String,
    /// Breite des Texts in px (`maxTextWidth`, gemessen mit Schrift und Größe des Menüs).
    pub max_text_width: f64,
    pub submenu: Vec<MenuItemView>,
}

/// Datum/Zeit aus `CDTs` (Monat 1-basiert).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct MenuDate {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

impl MenuDate {
    /// `YYYYMMDDhhmmss` lesen (höchstens 15 Zeichen ausgewertet, 15 §3.2).
    pub fn parse(s: &str) -> Option<MenuDate> {
        let s: String = s.chars().take(15).collect();
        let n = |a: usize, b: usize| s.get(a..b)?.parse::<u32>().ok();
        Some(MenuDate { year: n(0, 4)? as i32, month: n(4, 6)?, day: n(6, 8)?, hour: n(8, 10)?, minute: n(10, 12)?, second: n(12, 14)? })
    }
    /// `YYYYMMDDhhmmss` schreiben.
    pub fn format(&self) -> String {
        format!("{:04}{:02}{:02}{:02}{:02}{:02}", self.year, self.month, self.day, self.hour, self.minute, self.second)
    }
    /// `MMDDhhmmss` mit **0-basiertem** Monat (Eigenheit `DateToUniqueID`).
    pub fn uid(&self) -> String {
        format!("{:02}{:02}{:02}{:02}{:02}", self.month as i64 - 1, self.day, self.hour, self.minute, self.second)
    }
}

/// Aufgelöste Menüeigenschaften, wie die Vorlage sie über `BehaviorInfo` sieht.
#[derive(Clone, Debug, PartialEq)]
pub struct MenuView {
    pub items: Vec<MenuItemView>,
    pub creation: MenuDate,
    pub menu_id: i64,
    /// Ereignis (0 onMouseOver ⇒ Ausblenden bei onMouseOut).
    pub event: i32,
    pub menu_width: f64,
    pub menu_height: f64,
    pub text_only: bool,
    pub override_width: bool,
    pub override_height: bool,
    pub padding: f64,
    pub spacing: f64,
    pub text_indent: f64,
    pub border_size: f64,
    pub vertical: bool,
    pub submenu_relative: bool,
    pub horz_submenu_offset: f64,
    pub vert_submenu_offset: f64,
    pub horz_offset: f64,
    pub vert_offset: f64,
    pub hide_timeout: f64,
    pub opaque: bool,
    pub family: String,
    pub size: f64,
    pub text_up: String,
    pub text_over: String,
    pub cell_up: String,
    pub cell_over: String,
    pub align: String,
    pub valign: String,
    pub bold: bool,
    pub italic: bool,
    pub hilite: String,
    pub shadow: String,
    pub border: String,
    /// Grafik-Zellen: Bildnamen Up/Over (`mmmenu<n>_<B>x<H>_up/_over.<ext>`).
    pub image_path: String,
    pub image_path2: String,
}

/// ARGB → `#rrggbb` (Alpha verworfen).
pub fn css_color(argb: u32) -> String {
    format!("#{:06x}", argb & 0xFF_FFFF)
}

/// Zahl wie JavaScript sie in einen String wandelt (ganzzahlig ohne „.0“).
pub fn js_num(x: f64) -> String {
    if x.is_finite() && x == x.trunc() && x.abs() < 1e21 {
        format!("{}", x as i64)
    } else {
        format!("{x}")
    }
}

fn b(v: bool) -> &'static str {
    if v { "true" } else { "false" }
}

impl MenuView {
    /// Aus dem Modell (Vorgaben 15 §3.2). `measure(text, family, size, bold, italic)` misst die
    /// Textbreite; `creation_now` ersetzt ein fehlendes `CDTs` (Original: Ladezeit).
    pub fn from_model(
        m: &PopupMenu,
        event: i32,
        menu_id: i64,
        creation_now: MenuDate,
        measure: &dyn Fn(&str, &str, f64, bool, bool) -> f64,
    ) -> MenuView {
        let family = m.font.clone().unwrap_or_default();
        let size = m.size.unwrap_or(0.0).max(0.0) as f64;
        let bold = m.bold.unwrap_or(false);
        let italic = m.italic.unwrap_or(false);
        fn conv(items: &[fp_model::MenuItem], f: &dyn Fn(&str) -> f64) -> Vec<MenuItemView> {
            items
                .iter()
                .map(|i| MenuItemView { text: i.text.clone(), url: i.url.clone(), target: i.target.clone(), max_text_width: f(&i.text), submenu: conv(&i.submenu, f) })
                .collect()
        }
        let items = parse_menu_items(m.items_text.as_deref().unwrap_or("{}")).unwrap_or_default();
        let mf = |t: &str| measure(t, &family, size, bold, italic);
        let text_only = m.text_only.unwrap_or(false) || m.styles.len() < 2;
        let align = ["left", "center", "right", "justify", "stretch"].get(m.text_alignment.unwrap_or(0).clamp(0, 4) as usize).copied().unwrap_or("left");
        let valign = ["top", "middle", "bottom"].get(m.vert_alignment.unwrap_or(1).clamp(0, 2) as usize).copied().unwrap_or("middle");
        MenuView {
            items: conv(&items, &mf),
            creation: m.creation_date.as_deref().and_then(MenuDate::parse).unwrap_or(creation_now),
            menu_id: if m.menu_id.unwrap_or(-1) < 0 { menu_id } else { m.menu_id.unwrap_or(0) as i64 },
            event,
            menu_width: m.menu_width.unwrap_or(0) as f64,
            menu_height: m.menu_height.unwrap_or(0) as f64,
            text_only,
            override_width: m.override_width.unwrap_or(false),
            override_height: m.override_height.unwrap_or(false),
            padding: m.item_padding.unwrap_or(0) as f64,
            spacing: m.item_spacing.unwrap_or(0) as f64,
            text_indent: m.text_indent.unwrap_or(0) as f64,
            border_size: m.border_size.unwrap_or(1) as f64,
            vertical: m.vertical.unwrap_or(true),
            submenu_relative: m.submenu_relative.unwrap_or(true),
            horz_submenu_offset: m.horz_submenu_offset.unwrap_or(0.0) as f64,
            vert_submenu_offset: m.vert_submenu_offset.unwrap_or(0.0) as f64,
            horz_offset: m.horz_offset.unwrap_or(0.0) as f64,
            vert_offset: m.vert_offset.unwrap_or(0.0) as f64,
            hide_timeout: m.hide_timeout.unwrap_or(1000) as f64,
            opaque: m.opaque.unwrap_or(true),
            family,
            size,
            text_up: css_color(m.text_up_color.unwrap_or(0xFF00_0000)),
            text_over: css_color(m.text_over_color.unwrap_or(0)),
            cell_up: css_color(m.cell_up_color.unwrap_or(0)),
            cell_over: css_color(m.cell_over_color.unwrap_or(0)),
            align: align.into(),
            valign: valign.into(),
            bold,
            italic,
            hilite: css_color(m.hilite_color.unwrap_or(0xFFFF_FFFF)),
            shadow: css_color(m.shadow_color.unwrap_or(0xFF55_5555)),
            border: css_color(m.border_color.unwrap_or(0xFF77_7777)),
            image_path: String::new(),
            image_path2: String::new(),
        }
    }

    /// Namen der CSS-Variante: (Menü, Container).
    pub fn css_names(&self) -> (String, String) {
        let uid = format!("{}_{}", self.creation.uid(), self.menu_id);
        (format!("MMMenu{uid}"), format!("MMMenuContainer{uid}"))
    }

    /// Name der JS-Variante.
    pub fn js_name(&self) -> String {
        format!("mm_menu_{}_{}", self.creation.uid(), self.menu_id)
    }

    fn last_text_width(items: &[MenuItemView]) -> f64 {
        items.last().map(|i| i.max_text_width).unwrap_or(120.0)
    }

    fn item_w(&self, items: &[MenuItemView]) -> f64 {
        if !self.override_width && self.text_only {
            Self::last_text_width(items) + 2.0 * self.padding + self.text_indent + 2.0 * self.border_size
        } else {
            self.menu_width
        }
    }

    fn item_h(&self, first: bool, horiz: bool) -> f64 {
        let mut h = self.menu_height;
        if self.text_only {
            h += self.border_size;
            if first || horiz {
                h += self.border_size;
            }
        }
        h
    }

    fn menu_w(&self, items: &[MenuItemView], top: bool) -> f64 {
        let mut w = self.item_w(items);
        if top && !self.vertical {
            let n = items.len() as f64;
            w = w * n + (n - 1.0) * self.spacing;
        }
        if self.text_only {
            w += 2.0 * self.border_size;
        }
        w
    }

    fn menu_h(&self, items: &[MenuItemView], top: bool) -> f64 {
        let mut h = self.item_h(true, top && !self.vertical);
        if !top || self.vertical {
            let n = items.len() as f64;
            h += self.item_h(false, false) * (n - 1.0) + (n - 1.0) * self.spacing;
        }
        if self.text_only {
            h += 2.0 * self.border_size;
        }
        h
    }

    fn item_top(&self, ptop: f64, idx: usize, top: bool) -> f64 {
        let mut t = ptop;
        if !top || self.vertical {
            t += idx as f64 * self.spacing;
            if idx > 0 {
                t += self.item_h(true, false);
                if self.text_only {
                    t -= self.border_size;
                }
                t += (idx as f64 - 1.0) * self.item_h(false, false);
            }
        }
        t
    }

    fn item_left(&self, items: &[MenuItemView], pleft: f64, idx: usize, top: bool) -> f64 {
        let mut l = pleft;
        if top && !self.vertical {
            l += idx as f64 * self.spacing + if self.text_only { idx as f64 * self.border_size } else { 0.0 };
            if idx > 0 {
                l += self.item_w(items) + (idx as f64 - 1.0) * self.item_w(items);
            }
        }
        l
    }

    fn sub_left(&self, pitems: &[MenuItemView], pleft: f64, idx: usize, ptop: bool) -> f64 {
        let l = if self.submenu_relative {
            let mut w = self.item_w(pitems);
            if self.text_only {
                w -= 2.0 * self.border_size;
            }
            self.item_left(pitems, pleft, idx, ptop) + w
        } else {
            pleft + self.menu_w(pitems, ptop)
        };
        l + self.horz_submenu_offset
    }

    fn sub_top(&self, ptop_v: f64, idx: usize, ptop: bool) -> f64 {
        let t = if self.submenu_relative { self.item_top(ptop_v, idx, ptop) } else { ptop_v };
        t + self.vert_submenu_offset
    }

    fn origin(&self, items: &[MenuItemView], porig: (f64, f64), top: bool, cur: (f64, f64)) -> (f64, f64) {
        let (mut cx, mut cy) = cur;
        for (idx, it) in items.iter().enumerate() {
            if !it.submenu.is_empty() {
                let sl = self.sub_left(items, porig.0, idx, top);
                let st = self.sub_top(porig.1, idx, top);
                cx = cx.min(sl);
                cy = cy.min(st);
                let r = self.origin(&it.submenu, (sl, st), false, (cx, cy));
                cx = r.0;
                cy = r.1;
            }
        }
        (cx, cy)
    }

    /// Versatz des Wurzelmenüs im Container (Betrag des Minimums aller Untermenü-Positionen).
    pub fn top_origin(&self) -> (f64, f64) {
        let (x, y) = self.origin(&self.items, (0.0, 0.0), true, (0.0, 0.0));
        (x.abs(), y.abs())
    }

    fn item_class(name: &str, horiz: bool, first: bool) -> String {
        format!("MMMI{}{}Style{name}", if first { "F" } else { "" }, if horiz { "H" } else { "V" })
    }

    fn css_item_class(&self, out: &mut String, items: &[MenuItemView], name: &str, horiz: bool, first: bool) {
        let cls = Self::item_class(name, horiz, first);
        out.push_str(&format!(".{cls} {{\n"));
        out.push_str(&format!("/* This class determines the general characteristics of the menu items in menu {name} */\n"));
        let bs = js_num(self.border_size);
        if self.text_only {
            if (first && !horiz) || horiz {
                out.push_str(&format!("\tborder-top:{bs}px solid {};\n", self.hilite));
            } else {
                out.push_str("\tborder-top:0px;\n");
            }
            out.push_str(&format!("\tborder-left:{bs}px solid {};\n", self.hilite));
            out.push_str(&format!("\tborder-bottom:{bs}px solid {};\n", self.shadow));
            out.push_str(&format!("\tborder-right:{bs}px solid {};\n", self.shadow));
        }
        let w = self.item_w(items);
        let h = self.item_h(first, horiz);
        let bb = if self.text_only { self.border_size } else { 0.0 };
        out.push_str(&format!("\twidth:{}px;\n", js_num(w)));
        out.push_str(&format!("\theight:{}px;\n", js_num(h)));
        out.push_str("\tvoice-family: \"\\\"}\\\"\";\n");
        out.push_str("\tvoice-family:inherit;\n");
        let w2 = w - 2.0 * self.padding - 2.0 * bb;
        let mut h2 = h - 2.0 * self.padding - bb;
        if first && !horiz {
            h2 -= bb;
        }
        out.push_str(&format!("\twidth:{}px;\n", js_num(w2)));
        out.push_str(&format!("\theight:{}px;\n", js_num(h2)));
        out.push_str("}\n\n");
    }

    fn css_menu(&self, out: &mut String, items: &[MenuItemView], orig: (f64, f64), name: &str, top: bool) {
        out.push_str(&format!("#{name} {{\n"));
        out.push_str(&format!("/* This class defines things about menu {name}'s div. */\n"));
        out.push_str(&format!("\tposition:absolute;\n\tleft:{}px;\n\ttop:{}px;\n\tvisibility:hidden;\n\tbackground-color:", js_num(orig.0), js_num(orig.1)));
        out.push_str(if self.opaque { &self.shadow } else { "transparent" });
        out.push_str(";\n");
        let mut bb = 0.0;
        if self.text_only && self.opaque {
            out.push_str(&format!("\tborder:{}px solid {};\n", js_num(self.border_size), self.border));
            bb = self.border_size;
        }
        out.push_str(&format!("\twidth:{}px;\n", js_num(self.menu_w(items, top) - 2.0 * bb)));
        let mut fh = self.menu_h(items, top) - bb;
        if self.vertical || !top {
            fh -= bb;
        }
        out.push_str(&format!("\theight:{}px;\n", js_num(fh)));
        out.push_str("}\n\n");
        let horiz = !self.vertical && top;
        self.css_item_class(out, items, name, horiz, true);
        if items.len() > 1 {
            self.css_item_class(out, items, name, horiz, false);
        }
        for idx in 0..items.len() {
            let (l, t) = if self.vertical || !top {
                let mut t = self.item_h(true, true) * idx as f64;
                if self.text_only && idx > 0 {
                    t -= (idx as f64 - 1.0) * self.border_size;
                }
                if idx > 0 {
                    t += idx as f64 * self.spacing;
                }
                (0.0, t)
            } else {
                let mut l = self.item_w(items) * idx as f64;
                if idx > 0 {
                    l += idx as f64 * self.spacing;
                }
                (l, 0.0)
            };
            let iid = format!("{name}_Item_{idx}");
            out.push_str(&format!("#{iid} {{\n"));
            out.push_str(&format!("/* Unique ID for item {idx} of menu {name} so we can set its position */\n"));
            out.push_str(&format!("\tleft:{}px;\n", js_num(l)));
            out.push_str(&format!("\ttop:{}px;\n", js_num(t)));
            out.push_str("}\n\n");
        }
        let mut n_sub = 1;
        let mut has_sub = false;
        for (idx, it) in items.iter().enumerate() {
            if !it.submenu.is_empty() {
                let sl = self.sub_left(items, orig.0, idx, top);
                let st = self.sub_top(orig.1, idx, top);
                self.css_menu(out, &it.submenu, (sl, st), &format!("{name}_{n_sub}"), false);
                n_sub += 1;
                has_sub = true;
            }
        }
        if has_sub {
            let w = self.item_w(items);
            let mut sw = w - 8.0 - 4.0;
            if self.text_only {
                sw -= self.padding - self.border_size;
            }
            out.push_str(&format!(".MMMenuItemSpan{name} {{\n"));
            out.push_str(&format!("/* The span class definition so we can make sure that arrows and menu text in menu {name} look correct */\n"));
            out.push_str("\tfloat:left;\n");
            out.push_str(&format!("\twidth:{}px;\n", js_num(sw)));
            out.push_str("}\n\n");
            let h = self.item_h(true, horiz);
            let mut al = w - 8.0 - 3.0;
            if self.text_only {
                al -= self.padding - self.border_size;
            }
            let at = h / 2.0 - 8.0 / 2.0;
            out.push_str(&format!(".MMArrowStyle{name} {{\n"));
            out.push_str(&format!("/* This class allows us to position the submenu arrows for menu {name} */\n"));
            out.push_str("\tposition:absolute;\n");
            out.push_str(&format!("\tleft:{}px;\n", js_num(al)));
            out.push_str(&format!("\ttop:{}px;\n", js_num(at)));
            out.push_str("}\n\n");
        }
    }

    /// CSS eines Pop-up-Menüs (CSS-Variante); `slice_left/top` = Lage der Slice.
    pub fn css(&self, slice_left: f64, slice_top: f64, images_dir: &str) -> String {
        let mut out = String::new();
        let (ox, oy) = self.top_origin();
        let (name, cont) = self.css_names();
        out.push_str(&format!("#{cont} {{\n"));
        out.push_str(&format!("/* This ID is related to the master menu div for menu {cont} and contains the important positioning information for the menu as a whole */\n"));
        out.push_str(&format!(
            "\tposition:absolute;\n\tleft:{}px;\n\ttop:{}px;\n\tvisibility:hidden;\n\tz-index:300;\n}}\n\n",
            js_num(slice_left + self.horz_offset - ox),
            js_num(slice_top + self.vert_offset - oy)
        ));
        self.css_menu(&mut out, &self.items, (ox, oy), &name, true);
        out.push_str(&format!("#{cont} img {{\n"));
        out.push_str("/* needed for Mozilla/Camino/Netscape */\n");
        out.push_str("\tborder:0px;\n}\n\n");
        let mut fam = self.family.clone();
        let rf = fam.rfind("mono").map(|i| i as i64).unwrap_or(-1);
        if rf == fam.chars().count() as i64 - 4 {
            fam.push_str("space");
        }
        out.push_str(&format!("#{cont} a {{\n"));
        out.push_str(&format!("/* Controls the general apperance for menu {cont}'s items, including color and font */\n"));
        out.push_str(&format!(
            "\ttext-decoration:none;\n\tfont-family:{fam};\n\tfont-size:{}px;\n\tcolor:{};\n\ttext-align:{};\n\tvertical-align:{};\n\tpadding:{}px;\n",
            js_num(self.size),
            self.text_up,
            self.align,
            self.valign,
            js_num(self.padding)
        ));
        if !self.text_only {
            out.push_str(&format!("\tbackground: url(\"{images_dir}{}\") {};\n", self.image_path, self.cell_up));
        } else {
            out.push_str(&format!("\tbackground-color:{};\n", self.cell_up));
        }
        out.push_str(if self.bold { "\tfont-weight:bold;\n" } else { "\tfont-weight:normal;\n" });
        out.push_str(if self.italic { "\tfont-style:italic;\n" } else { "\tfont-style:normal;\n" });
        out.push_str("\tdisplay:block;\n\tposition:absolute;\n");
        out.push_str("}\n\n");
        out.push_str(&format!("#{cont} a:hover {{\n"));
        out.push_str(&format!("/* Controls the mouse over effects for menu {cont} */\n"));
        out.push_str(&format!("\tcolor:{};\n", self.text_over));
        if !self.text_only {
            out.push_str(&format!("\tbackground: url(\"{images_dir}{}\") {};\n", self.image_path2, self.cell_over));
        } else {
            out.push_str(&format!("\tbackground-color:{};\n", self.cell_over));
        }
        out.push_str("}\n");
        out
    }

    #[allow(clippy::too_many_arguments)]
    fn html_menu(&self, out: &mut String, items: &[MenuItemView], name: &str, hide: bool, vertical: bool, xhtml: bool, arrow: &str) {
        out.push_str(&format!("\t<div id=\"{name}\""));
        if hide {
            out.push_str(if xhtml { " onmouseout=\"" } else { " onMouseOut=\"" });
            out.push_str(&format!("MM_menuStartTimeout({});\"", js_num(self.hide_timeout)));
        }
        out.push_str(if xhtml { " onmouseover=\"" } else { " onMouseOver=\"" });
        out.push_str("MM_menuResetTimeout();\">\n");
        let mut first = true;
        let mut n_sub = 1;
        for (idx, it) in items.iter().enumerate() {
            out.push_str(&format!("\t\t<a href=\"{}\"", if it.url.is_empty() { "javascript:;" } else { &it.url }));
            if !it.target.is_empty() {
                out.push_str(&format!(" target=\"{}\"", it.target));
            }
            out.push_str(&format!(" id=\"{name}_Item_{idx}\""));
            out.push_str(&format!(" class=\"{}\"", Self::item_class(name, !vertical, first)));
            out.push_str(if xhtml { " onmouseover=\"" } else { " onMouseOver=\"" });
            out.push_str(&format!("MM_menuOverMenuItem('{name}'"));
            if !it.submenu.is_empty() {
                out.push_str(&format!(",'{n_sub}'"));
                n_sub += 1;
            }
            out.push_str(");\">\n");
            if !it.submenu.is_empty() {
                out.push_str(&format!("\t\t\t<span class=\"MMMenuItemSpan{name}\">{}</span>\n", it.text));
                out.push_str(&format!("\t\t\t<img src=\"{arrow}\" alt=\"\" class=\"MMArrowStyle{name}\""));
                out.push_str(if xhtml { " />\n" } else { ">\n" });
            } else {
                out.push_str(&format!("\t\t\t{}\n", it.text));
            }
            out.push_str("\t\t</a>\n");
            first = false;
        }
        out.push_str("\t</div>\n");
        let mut n_sub = 1;
        for it in items {
            if !it.submenu.is_empty() {
                self.html_menu(out, &it.submenu, &format!("{name}_{n_sub}"), hide, true, xhtml, arrow);
                n_sub += 1;
            }
        }
    }

    /// `<div>`-Baum (CSS-Variante).
    pub fn html(&self, xhtml: bool, arrow_path: &str) -> String {
        let (name, cont) = self.css_names();
        let mut out = format!("<div id=\"{cont}\">\n");
        self.html_menu(&mut out, &self.items, &name, self.event == 0, self.vertical, xhtml, arrow_path);
        out.push_str("</div>\n");
        out
    }
}

fn esc_js(t: &str) -> String {
    t.replace('\\', "\\\\")
}

/// Rumpf von `mmLoadMenus()` für alle Menüs (JS-Variante).
pub fn menus_js(menus: &[MenuView], images_dir: &str, arrow_path: &str) -> String {
    struct St {
        out: String,
        wrote: bool,
        last: String,
    }
    #[allow(clippy::too_many_arguments)]
    fn write(st: &mut St, m: &MenuView, items: &[MenuItemView], text: &str, indent: &str, name: &str, hide: bool, images_dir: &str, arrow: &str) {
        if !st.wrote {
            st.out.push_str("function mmLoadMenus() {\n");
            st.out.push_str(&format!("  if (window.{name}) return;\n"));
            st.wrote = true;
        }
        let mut has_sub = false;
        let mut n_sub = 1;
        let mut iew = 120.0;
        let mut sub_names: Vec<Option<String>> = Vec::with_capacity(items.len());
        for it in items {
            iew = it.max_text_width;
            if !it.submenu.is_empty() {
                let sname = format!("{name}_{n_sub}");
                n_sub += 1;
                write(st, m, &it.submenu, &format!("\"{}\"", it.text), &format!("{indent}  "), &sname, hide, images_dir, arrow);
                sub_names.push(Some(sname));
                has_sub = true;
            } else {
                sub_names.push(None);
            }
        }
        st.last = name.to_string();
        let mut mw = m.menu_width;
        if !m.override_width && m.text_only {
            mw = iew + 2.0 * m.padding + m.text_indent;
        }
        let mut args = format!(
            ",{},{},\"{}\",{},\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",{},{},{},{},{},{},{}",
            js_num(mw),
            js_num(m.menu_height),
            m.family,
            js_num(m.size),
            m.text_up,
            m.text_over,
            m.cell_up,
            m.cell_over,
            m.align,
            m.valign,
            js_num(m.padding),
            js_num(m.spacing),
            js_num(m.hide_timeout),
            js_num(m.horz_submenu_offset),
            js_num(m.vert_submenu_offset),
            b(m.submenu_relative),
            b(m.opaque)
        );
        if text == "\"root\"" {
            args.push_str(&format!(",{}", b(m.vertical)));
        } else {
            args.push_str(",true");
        }
        args.push_str(&format!(",{},{},{}", js_num(m.text_indent), b(!m.override_width), b(!m.override_height)));
        st.out.push_str(&format!("{indent}window.{name} = new Menu({}{args});\n", esc_js(text)));
        for (it, sn) in items.iter().zip(&sub_names) {
            let label = match sn {
                Some(s) => s.clone(),
                None if !it.text.is_empty() => esc_js(&format!("\"{}\"", it.text)),
                None => continue,
            };
            st.out.push_str(&format!("{indent}{name}.addMenuItem({label}"));
            if !it.url.is_empty() {
                if !it.target.is_empty() {
                    st.out.push_str(&format!(",\"window.open('{}', '{}');\"", it.url, it.target));
                } else {
                    st.out.push_str(&format!(",\"location='{}'\"", it.url));
                }
            }
            st.out.push_str(");\n");
        }
        let tm = format!("{indent} {name}.");
        if !m.text_only {
            st.out.push_str(&format!("{tm}bgImageUp=\"{images_dir}{}\";\n", m.image_path));
            st.out.push_str(&format!("{tm}bgImageOver=\"{images_dir}{}\";\n", m.image_path2));
        }
        if m.bold {
            st.out.push_str(&format!("{tm}fontWeight=\"bold\";\n"));
        }
        if m.italic {
            st.out.push_str(&format!("{tm}fontStyle=\"italic\";\n"));
        }
        st.out.push_str(&format!("{tm}{}", if hide { "hideOnMouseOut=true;\n" } else { "hideOnMouseOut=false;\n" }));
        if has_sub {
            st.out.push_str(&format!("{tm}childMenuIcon=\"{arrow}\";\n"));
        }
        if m.text_only {
            st.out.push_str(&format!("{tm}menuBorder={};\n", js_num(m.border_size)));
            st.out.push_str(&format!("{tm}menuItemBorder={};\n", js_num(m.border_size)));
            st.out.push_str(&format!("{tm}menuLiteBgColor='{}';\n", m.hilite));
            st.out.push_str(&format!("{tm}menuBorderBgColor='{}';\n", m.border));
        }
        st.out.push_str(&format!("{tm}bgColor='{}';\n", m.shadow));
    }
    let mut st = St { out: String::new(), wrote: false, last: String::new() };
    for m in menus {
        let name = m.js_name();
        write(&mut st, m, &m.items, "\"root\"", "  ", &name, m.event == 0, images_dir, arrow_path);
    }
    if st.wrote {
        st.out.push_str(&format!("\n  {}.writeMenus();\n", st.last));
        st.out.push_str("} // mmLoadMenus()\n\n");
    }
    st.out
}

/// Positions-Schnelltasten des Pop-up-Editors (15 §3.4): (XLCf, YLCf) bzw. (XSLf, YSLf).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PositionPresets {
    pub menu_bottom_right: (f32, f32),
    pub menu_below: (f32, f32),
    pub menu_above: (f32, f32),
    pub menu_right_top: (f32, f32),
    pub sub_bottom_right: (f32, f32),
    pub sub_below: (f32, f32),
    pub sub_right_top: (f32, f32),
}

/// Schnelltasten für Slice-Größe `sw × sh`, Zellgröße `iw × ih`, `n` Einträge oberster Ebene.
pub fn position_presets(sw: i32, sh: i32, iw: i32, ih: i32, vertical: bool, n: usize) -> PositionPresets {
    let mh = if vertical { n as i32 * ih } else { ih };
    PositionPresets {
        menu_bottom_right: ((sw - 5) as f32, (sh - 5) as f32),
        menu_below: (0.0, sh as f32),
        menu_above: (0.0, -(mh as f32)),
        menu_right_top: (sw as f32, 0.0),
        sub_bottom_right: (-5.0, (ih as f64 - ih as f64 / 3.0) as f32),
        sub_below: (-(iw as f32), ih as f32),
        sub_right_top: (0.0, 0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_numbers() {
        let d = MenuDate::parse("20260111093005").unwrap();
        assert_eq!(d.uid(), "0011093005");
        assert_eq!(d.format(), "20260111093005");
        assert_eq!(js_num(7.5), "7.5");
        assert_eq!(js_num(-3.0), "-3");
        assert_eq!(css_color(0xFF123456), "#123456");
        let p = position_presets(120, 30, 120, 20, false, 3);
        assert_eq!(p.menu_right_top, (120.0, 0.0));
        assert!((p.sub_bottom_right.1 - 13.333333).abs() < 1e-4);
    }
}
