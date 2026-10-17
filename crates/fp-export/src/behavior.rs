//! Verhalten beim HTML-Export (15 §2.5, §2.6): Zerlegung der Dokument-Verhalten in Grundaktionen
//! (`@00775d10`), Ereignis-Code je Zelle bzw. `<area>`, `onLoad`/Vorladeliste und die eigenen
//! JavaScript-Laufzeitfunktionen (`MM_swapImage`, `MM_swapImgRestore`, `MM_nbGroup`,
//! `MM_preloadImages`, `MM_findObj`, `MM_displayStatusMsg`, `MM_controlSound`, Pop-up-Menüs).
//!
//! Die Funktionsnamen sind die funktionalen Namen, die Original-HTML benutzt (damit „HTML
//! aktualisieren“ und Dreamweaver sie wiedererkennen); die Rümpfe sind eigene Implementierungen,
//! keine Kopie der Vorlagen des Originals.

use fp_model::{Behavior, PopupMenu};

/// Ziel einer Austausch-Aktion.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// Die eigene Slice.
    Own,
    /// Slice über ihre ID (`MSNi`).
    Slice(u32),
}

/// Grundaktion für die HTML-Ausgabe (`BehaviorInfo`).
#[derive(Clone, Debug, PartialEq)]
pub struct ExportAction {
    /// 1 Statusleiste, 2 Bild austauschen, 4 Nav-Down, 5 Austausch wiederherstellen, 6 Nav-Over,
    /// 7 Nav wiederherstellen, 9 Pop-up, 10 benutzerdefiniert, 11 Sound.
    pub action: i32,
    /// 0 onMouseOver, 1 onClick, 2 onMouseOut, 3 onLoad.
    pub event: i32,
    pub target: Target,
    /// 0-basierter Status des Austauschbildes.
    pub frame: i32,
    /// Externe Datei statt Status.
    pub file: String,
    pub preload: bool,
    pub restore: bool,
    pub down_highlight: bool,
    pub dh_frame: i32,
    pub dh_file: String,
    pub status_text: String,
    pub popup: Option<Box<PopupMenu>>,
}

impl ExportAction {
    fn simple(action: i32, event: i32) -> ExportAction {
        ExportAction {
            action,
            event,
            target: Target::Own,
            frame: 0,
            file: String::new(),
            preload: false,
            restore: false,
            down_highlight: false,
            dh_frame: 0,
            dh_file: String::new(),
            status_text: String::new(),
            popup: None,
        }
    }

    fn swap(
        action: i32,
        event: i32,
        target: Target,
        frame: i32,
        file: &str,
        preload: bool,
        restore: bool,
    ) -> ExportAction {
        ExportAction {
            target,
            frame,
            file: file.to_string(),
            preload,
            restore,
            ..ExportAction::simple(action, event)
        }
    }
}

/// Zerlegung (`@00775d10`, `param_7 = 0`). `explicit_restore` = Schalter `param_6` (statt
/// `MM_swapImgRestore` ein Austausch auf Status 0 bei onMouseOut); `num_frames` = Anzahl Status;
/// `slice_exists` prüft Ziel-Slice-IDs. Ungültige Austausch-Verhalten werden still verworfen.
pub fn expand_for_export(
    b: &Behavior,
    explicit_restore: bool,
    num_frames: usize,
    slice_exists: &dyn Fn(u32) -> bool,
) -> Vec<ExportAction> {
    let restore = |target: Target, preload: bool| {
        if explicit_restore {
            ExportAction::swap(2, 2, target, 0, "", preload, false)
        } else {
            ExportAction::simple(5, 2)
        }
    };
    let flags = b.flags.unwrap_or(0);
    let t = b.action;
    let is_nav = (t == 0 || t == 3) && flags & 2 != 0;
    let is_simple = t == 8 || ((t == 0 || t == 3) && flags & 2 == 0);
    if is_simple {
        return vec![
            ExportAction::swap(2, 0, Target::Own, 1, "", true, !explicit_restore),
            restore(Target::Own, true),
        ];
    }
    if is_nav {
        let over_down = flags & 1 != 0;
        let mut v = vec![
            ExportAction {
                down_highlight: true,
                dh_frame: if over_down { 3 } else { 2 },
                ..ExportAction::swap(6, 0, Target::Own, 1, "", true, false)
            },
            ExportAction::simple(7, 2),
            ExportAction::swap(4, 1, Target::Own, 2, "", true, false),
        ];
        if b.checked.unwrap_or(false) {
            v.push(ExportAction::swap(4, 3, Target::Own, 2, "", true, false));
        }
        return v;
    }
    let file = b.file.clone().unwrap_or_default();
    let frame = b.frame.unwrap_or(2);
    if matches!(t, 2 | 4 | 6) {
        let Some(id) = b.target_slice else {
            return vec![];
        };
        if !slice_exists(id) || (file.is_empty() && (frame < 0 || frame as usize >= num_frames)) {
            return vec![];
        }
        let preload = b.preload.unwrap_or(true);
        if t == 2 {
            let rs = b.restore.unwrap_or(true);
            let a = ExportAction::swap(2, b.event, Target::Slice(id), frame, &file, preload, rs);
            return if rs {
                vec![a, restore(Target::Slice(id), preload)]
            } else {
                vec![a]
            };
        }
        let mut a = ExportAction::swap(t, b.event, Target::Slice(id), frame, &file, preload, false);
        if t == 6 {
            a.down_highlight = true;
            a.dh_frame = b.down_frame.unwrap_or(3);
            a.dh_file = b.down_file.clone().unwrap_or_default();
        }
        return vec![a];
    }
    let mut a = ExportAction::simple(t, b.event);
    match t {
        1 => a.status_text = b.status_text.clone().unwrap_or_default(),
        9 => a.popup = b.popup.clone(),
        11 => a.file = file,
        10 => return vec![],
        _ => {}
    }
    vec![a]
}

/// Welche Laufzeitfunktionen eine Seite braucht.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct JsNeeds {
    pub status: bool,
    pub sound: bool,
    pub swap: bool,
    pub nav: bool,
    pub preload: bool,
    pub css_menus: bool,
    pub js_menus: bool,
}

/// Auflösung von Aktionen zu Zellnamen und Bilddateien (vom HTML-Schreiber bereitgestellt).
pub trait CellResolver {
    /// Zellname (`name`/`id` des `<img>`) einer Slice (`None` = eigene Slice).
    fn cell_name(&self, slice: Option<u32>) -> Option<String>;
    /// Bildpfad eines Status (0-basiert) der Slice.
    fn frame_file(&self, slice: Option<u32>, frame: i32) -> Option<String>;
    /// Pop-up-Menü: (Container, Menüname bzw. JS-Variable, x, y) für den Aufruf.
    fn menu_ref(&self, _menu: &PopupMenu, _css: bool) -> Option<(String, String, f64, f64)> {
        None
    }
}

/// Erzeugter Ereignis-Code einer Zelle bzw. Fläche.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventCode {
    pub on_mouse_over: String,
    pub on_mouse_out: String,
    pub on_click: String,
    /// Beitrag zu `FWLoadInit` (onLoad-Verhalten).
    pub on_load: String,
    /// Down beim Laden: (Down-Bild statt `src`, `onLoad` des `<img>`).
    pub down_on_load: Option<(String, String)>,
    /// Vorzuladende Bilder.
    pub preload: Vec<String>,
}

fn js_str(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "\\'")
}

/// Ereignis-Code aus Grundaktionen (15 §2.6, Reihenfolge der Aufrufe wie die Vorlage).
pub fn event_code(
    actions: &[ExportAction],
    r: &dyn CellResolver,
    css_menus: bool,
    needs: &mut JsNeeds,
) -> EventCode {
    let mut ec = EventCode::default();
    for event in [0, 2, 1, 3] {
        let mut code = String::new();
        let (mut swaps, mut downs, mut overs) = (Vec::new(), Vec::new(), Vec::new());
        let (mut restore_swap, mut restore_nav) = (false, false);
        let mut ret_false = false;
        let mut ret_status = false;
        for a in actions.iter().filter(|a| a.event == event) {
            let target = match &a.target {
                Target::Own => None,
                Target::Slice(id) => Some(*id),
            };
            let img = |frame: i32, file: &str| -> Option<String> {
                if !file.is_empty() {
                    Some(file.to_string())
                } else {
                    r.frame_file(target, frame)
                }
            };
            match a.action {
                1 => {
                    needs.status = true;
                    code.push_str(&format!(
                        "MM_displayStatusMsg('{}');",
                        js_str(&a.status_text)
                    ));
                    ret_status = true;
                }
                11 => {
                    needs.sound = true;
                    code.push_str(&format!(
                        "MM_controlSound('play','document.CS{}','{}');",
                        ec.preload.len(),
                        js_str(&a.file)
                    ));
                    ret_false = event == 1;
                }
                9 => {
                    if let Some(p) = &a.popup
                        && let Some((cont, menu, x, y)) = r.menu_ref(p, css_menus)
                    {
                        let cell = r.cell_name(None).unwrap_or_default();
                        let (xs, ys) = (crate::popup::js_num(x), crate::popup::js_num(y));
                        if css_menus {
                            needs.css_menus = true;
                            code.push_str(&format!(
                                "MM_menuShowMenu('{cont}','{menu}',{xs},{ys},'{cell}');"
                            ));
                        } else {
                            needs.js_menus = true;
                            code.push_str(&format!(
                                "MM_showMenu(window.{menu},{xs},{ys},null,'{cell}');"
                            ));
                        }
                    }
                }
                5 => restore_swap = true,
                7 => restore_nav = true,
                2 | 4 | 6 => {
                    let Some(cell) = r.cell_name(target) else {
                        continue;
                    };
                    let Some(file) = img(a.frame, &a.file) else {
                        continue;
                    };
                    if a.preload {
                        ec.preload.push(file.clone());
                    }
                    match a.action {
                        2 => {
                            needs.swap = true;
                            swaps.push(format!("'{cell}','{}'", js_str(&file)));
                        }
                        4 => {
                            needs.nav = true;
                            if event == 3 {
                                let up = r.frame_file(target, 0).unwrap_or_default();
                                ec.preload.push(up.clone());
                                ec.down_on_load = Some((
                                    file.clone(),
                                    format!(
                                        "MM_nbGroup('init','navbar1','{cell}','{}',1)",
                                        js_str(&up)
                                    ),
                                ));
                            } else {
                                downs.push(format!("'{cell}','{}'", js_str(&file)));
                            }
                        }
                        _ => {
                            needs.nav = true;
                            let dh = if !a.dh_file.is_empty() {
                                Some(a.dh_file.clone())
                            } else {
                                r.frame_file(target, a.dh_frame)
                            };
                            if let Some(d) = &dh {
                                ec.preload.push(d.clone());
                            }
                            overs.push(format!(
                                "'{cell}','{}','{}'",
                                js_str(&file),
                                js_str(&dh.unwrap_or_default())
                            ));
                        }
                    }
                }
                _ => {}
            }
        }
        if !downs.is_empty() && event != 3 {
            code.push_str(&format!(
                "MM_nbGroup('down','navbar1',{},1);",
                downs.join(",")
            ));
        }
        if !overs.is_empty() {
            code.push_str(&format!("MM_nbGroup('over',{},1);", overs.join(",")));
        }
        if !swaps.is_empty() {
            code.push_str(&format!("MM_swapImage({},1);", swaps.join(",")));
        }
        if restore_swap {
            code.push_str("MM_swapImgRestore();");
        }
        if restore_nav {
            code.push_str("MM_nbGroup('out');");
        }
        if ret_false {
            code.push_str("return false;");
        } else if ret_status {
            code.push_str("return document.MM_returnValue;");
        }
        match event {
            0 => ec.on_mouse_over = code,
            2 => ec.on_mouse_out = code,
            1 => ec.on_click = code,
            _ => ec.on_load = code,
        }
    }
    if needs.swap || needs.nav {
        needs.preload |= !ec.preload.is_empty();
    }
    let mut seen = std::collections::HashSet::new();
    ec.preload
        .retain(|p| !p.is_empty() && seen.insert(p.clone()));
    ec
}

/// Eigene Laufzeitfunktionen (nur die benötigten).
pub fn runtime_js(n: &JsNeeds) -> String {
    let mut s = String::new();
    if n.swap || n.nav {
        s.push_str(
            "function MM_findObj(n, d) {\n  d = d || document;\n  var x = d.getElementById ? d.getElementById(n) : null;\n  if (!x && d.images) x = d.images[n];\n  if (!x && d.forms) for (var i = 0; !x && i < d.forms.length; i++) x = d.forms[i][n];\n  return x;\n}\n",
        );
    }
    if n.swap {
        s.push_str(
            "function MM_swapImgRestore() {\n  var a = document.MM_sr || [];\n  for (var i = 0; i < a.length; i++) if (a[i] && a[i].oSrc) a[i].src = a[i].oSrc;\n  document.MM_sr = [];\n}\nfunction MM_swapImage() {\n  var a = arguments, last = (a.length % 2) ? a[a.length - 1] : 0;\n  document.MM_sr = [];\n  for (var i = 0; i + 1 < a.length; i += 2) {\n    var x = MM_findObj(a[i]);\n    if (x) { if (!x.oSrc) x.oSrc = x.src; x.src = a[i + 1]; document.MM_sr.push(x); }\n  }\n  return last;\n}\n",
        );
    }
    if n.nav {
        s.push_str(
            "function MM_nbGroup(ev, grp) {\n  var a = arguments, i, x;\n  document.MM_nb = document.MM_nb || {};\n  var g = document.MM_nb[grp] = document.MM_nb[grp] || { down: [], over: [] };\n  if (ev == 'init' && a.length > 4) {\n    x = MM_findObj(a[2]);\n    if (x) { x.MM_up = a[3]; x.MM_down = x.src; x.MM_dn = true; g.down = [x]; }\n  } else if (ev == 'over') {\n    g = document.MM_nb['navbar1'] = document.MM_nb['navbar1'] || { down: [], over: [] };\n    g.over = [];\n    for (i = 1; i + 2 < a.length; i += 3) {\n      x = MM_findObj(a[i]);\n      if (x) { if (!x.MM_up) x.MM_up = x.src; x.src = (x.MM_dn && a[i + 2]) ? a[i + 2] : a[i + 1]; g.over.push(x); }\n    }\n  } else if (ev == 'out') {\n    g = document.MM_nb['navbar1'] || { down: [], over: [] };\n    for (i = 0; i < g.over.length; i++) { x = g.over[i]; x.src = x.MM_dn ? (x.MM_down || x.src) : (x.MM_up || x.src); }\n    g.over = [];\n  } else if (ev == 'down') {\n    for (i = 0; i < g.down.length; i++) { x = g.down[i]; if (x.MM_up) x.src = x.MM_up; x.MM_dn = false; }\n    g.down = [];\n    for (i = 2; i + 1 < a.length; i += 2) {\n      x = MM_findObj(a[i]);\n      if (x) { if (!x.MM_up) x.MM_up = x.src; x.src = x.MM_down = a[i + 1]; x.MM_dn = true; g.down.push(x); }\n    }\n  }\n}\n",
        );
    }
    if n.preload {
        s.push_str(
            "function MM_preloadImages() {\n  document.MM_p = document.MM_p || [];\n  for (var i = 0; i < arguments.length; i++) {\n    var img = new Image();\n    img.src = arguments[i];\n    document.MM_p.push(img);\n  }\n}\n",
        );
    }
    if n.status {
        s.push_str("function MM_displayStatusMsg(msg) {\n  window.status = msg;\n  document.MM_returnValue = true;\n}\n");
    }
    if n.sound {
        s.push_str(
            "function MM_controlSound(cmd, name, file) {\n  if (cmd != 'play') return;\n  try { var a = new Audio(file); a.play(); } catch (e) {}\n}\n",
        );
    }
    s
}

/// Eigenes Skript für CSS-Pop-up-Menüs (`fp_css_menu.js`).
pub const CSS_MENU_RUNTIME: &str = "/* Firepixel: CSS-Pop-up-Menüs */\nvar MM_menuTimer = null, MM_menuOpen = [];\nfunction MM_menuHideAll() {\n  for (var i = 0; i < MM_menuOpen.length; i++) MM_menuOpen[i].style.visibility = 'hidden';\n  MM_menuOpen = [];\n}\nfunction MM_menuShowMenu(cont, menu, x, y, trigger) {\n  MM_menuResetTimeout();\n  MM_menuHideAll();\n  var c = document.getElementById(cont), m = document.getElementById(menu);\n  if (!c || !m) return;\n  c.style.visibility = 'visible';\n  m.style.visibility = 'visible';\n  MM_menuOpen.push(c, m);\n}\nfunction MM_menuOverMenuItem(menu, sub) {\n  MM_menuResetTimeout();\n  var keep = [];\n  for (var i = 0; i < MM_menuOpen.length; i++) {\n    var id = MM_menuOpen[i].id;\n    if (menu.indexOf(id) == 0 || id.indexOf('MMMenuContainer') == 0) keep.push(MM_menuOpen[i]);\n    else MM_menuOpen[i].style.visibility = 'hidden';\n  }\n  MM_menuOpen = keep;\n  if (sub) {\n    var s = document.getElementById(menu + '_' + sub);\n    if (s) { s.style.visibility = 'visible'; MM_menuOpen.push(s); }\n  }\n}\nfunction MM_menuStartTimeout(t) {\n  MM_menuResetTimeout();\n  MM_menuTimer = setTimeout(MM_menuHideAll, t);\n}\nfunction MM_menuResetTimeout() {\n  if (MM_menuTimer) clearTimeout(MM_menuTimer);\n  MM_menuTimer = null;\n}\n";

/// Eigenes Skript für JS-Pop-up-Menüs (`fp_menu.js`): `Menu`-Objekt mit derselben Konstruktor-
/// Signatur wie die Ausgabe des Originals, `addMenuItem`, `writeMenus`, `MM_showMenu`, `MM_startTimeout`.
pub const JS_MENU_RUNTIME: &str = "/* Firepixel: JavaScript-Pop-up-Menüs */\nvar FP_menus = [], FP_timer = null;\nfunction Menu(label, w, h, font, size, fgUp, fgOver, bgUp, bgOver, align, valign, pad, space, timeout, subX, subY, rel, opaque, vertical, indent, autoW, autoH) {\n  this.label = label; this.w = w; this.h = h; this.font = font; this.size = size;\n  this.fgUp = fgUp; this.fgOver = fgOver; this.bgUp = bgUp; this.bgOver = bgOver;\n  this.align = align; this.valign = valign; this.pad = pad; this.space = space; this.timeout = timeout;\n  this.subX = subX; this.subY = subY; this.rel = rel; this.opaque = opaque; this.vertical = vertical;\n  this.indent = indent; this.items = []; this.hideOnMouseOut = true; this.bgColor = '#555555';\n  this.menuBorder = 1; this.menuItemBorder = 1; this.menuLiteBgColor = '#ffffff'; this.menuBorderBgColor = '#777777';\n  this.fontWeight = 'normal'; this.fontStyle = 'normal'; this.childMenuIcon = ''; this.div = null;\n  FP_menus.push(this);\n}\nMenu.prototype.addMenuItem = function (item, action) { this.items.push({ item: item, action: action || '' }); };\nMenu.prototype.build = function () {\n  if (this.div) return this.div;\n  var d = document.createElement('div'), self = this;\n  d.style.cssText = 'position:absolute;visibility:hidden;z-index:300;background:' + (this.opaque ? this.bgColor : 'transparent');\n  for (var i = 0; i < this.items.length; i++) (function (it, i) {\n    var a = document.createElement('div'), isMenu = typeof it.item == 'object';\n    a.textContent = isMenu ? it.item.label : it.item;\n    a.style.cssText = 'position:absolute;cursor:pointer;box-sizing:border-box;width:' + self.w + 'px;height:' + self.h + 'px;padding:' + self.pad + 'px;font:' + self.fontStyle + ' ' + self.fontWeight + ' ' + self.size + 'px ' + self.font + ';color:' + self.fgUp + ';background:' + self.bgUp + ';text-align:' + self.align;\n    if (self.vertical) a.style.top = (i * (self.h + self.space)) + 'px'; else a.style.left = (i * (self.w + self.space)) + 'px';\n    a.onmouseover = function () { FP_reset(); a.style.color = self.fgOver; a.style.background = self.bgOver; if (isMenu) { var s = it.item.build(); s.style.left = (d.offsetLeft + (self.vertical ? self.w : a.offsetLeft) + self.subX) + 'px'; s.style.top = (d.offsetTop + (self.vertical ? a.offsetTop : self.h) + self.subY) + 'px'; s.style.visibility = 'visible'; } };\n    a.onmouseout = function () { a.style.color = self.fgUp; a.style.background = self.bgUp; if (self.hideOnMouseOut) MM_startTimeout(); };\n    if (it.action) a.onclick = function () { eval(it.action); };\n    d.appendChild(a);\n  })(this.items[i], i);\n  var n = this.items.length;\n  d.style.width = (this.vertical ? this.w : n * (this.w + this.space)) + 'px';\n  d.style.height = (this.vertical ? n * (this.h + this.space) : this.h) + 'px';\n  document.body.appendChild(d);\n  this.div = d;\n  return d;\n};\nMenu.prototype.writeMenus = function () {};\nfunction FP_hideAll() { for (var i = 0; i < FP_menus.length; i++) if (FP_menus[i].div) FP_menus[i].div.style.visibility = 'hidden'; }\nfunction FP_reset() { if (FP_timer) clearTimeout(FP_timer); FP_timer = null; }\nfunction MM_startTimeout() { FP_reset(); FP_timer = setTimeout(FP_hideAll, 1000); }\nfunction MM_showMenu(menu, x, y, child, imgName) {\n  if (!menu) return;\n  FP_reset(); FP_hideAll();\n  var img = document.getElementById(imgName) || document.images[imgName], d = menu.build();\n  var r = img ? img.getBoundingClientRect() : { left: 0, top: 0 };\n  d.style.left = (r.left + window.pageXOffset + x) + 'px';\n  d.style.top = (r.top + window.pageYOffset + y) + 'px';\n  d.style.visibility = 'visible';\n}\n";

#[cfg(test)]
mod tests {
    use super::*;
    use fp_model::BehaviorAction;

    fn b(action: BehaviorAction, event: i32) -> Behavior {
        Behavior::new(action, event)
    }

    #[test]
    fn expansion_like_00775d10() {
        let all = |_: u32| true;
        let seq = |v: Vec<ExportAction>| {
            v.iter()
                .map(|a| (a.action, a.event, a.frame))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            seq(expand_for_export(
                &b(BehaviorAction::SimpleRollover, 0),
                false,
                4,
                &all
            )),
            vec![(2, 0, 1), (5, 2, 0)]
        );
        let mut nav = b(BehaviorAction::NavBar, 0);
        nav.flags = Some(2);
        assert_eq!(
            seq(expand_for_export(&nav, false, 4, &all)),
            vec![(6, 0, 1), (7, 2, 0), (4, 1, 2)]
        );
        nav.flags = Some(3);
        nav.checked = Some(true);
        let v = expand_for_export(&nav, false, 4, &all);
        assert_eq!(
            seq(v.clone()),
            vec![(6, 0, 1), (7, 2, 0), (4, 1, 2), (4, 3, 2)]
        );
        assert_eq!(v[0].dh_frame, 3);
        let mut sw = b(BehaviorAction::SwapImage, 0);
        sw.target_slice = Some(7);
        sw.frame = Some(4);
        sw.restore = Some(true);
        assert_eq!(
            seq(expand_for_export(&sw, false, 5, &|i| i == 7)),
            vec![(2, 0, 4), (5, 2, 0)]
        );
        sw.frame = Some(5);
        assert!(expand_for_export(&sw, false, 5, &|i| i == 7).is_empty());
        assert!(expand_for_export(&b(BehaviorAction::Custom, 0), false, 5, &all).is_empty());
        let e = expand_for_export(&b(BehaviorAction::SimpleRollover, 0), true, 4, &all);
        assert_eq!((e[1].action, e[1].event, e[1].frame), (2, 2, 0));
    }

    struct R;
    impl CellResolver for R {
        fn cell_name(&self, s: Option<u32>) -> Option<String> {
            Some(match s {
                None => "own".into(),
                Some(i) => format!("c{i}"),
            })
        }
        fn frame_file(&self, s: Option<u32>, f: i32) -> Option<String> {
            Some(format!(
                "images/{}_f{}.gif",
                s.map(|v| v.to_string()).unwrap_or("own".into()),
                f + 1
            ))
        }
    }

    #[test]
    fn event_code_order() {
        let all = |_: u32| true;
        let acts = expand_for_export(&b(BehaviorAction::SimpleRollover, 0), false, 4, &all);
        let mut n = JsNeeds::default();
        let ec = event_code(&acts, &R, true, &mut n);
        assert_eq!(
            ec.on_mouse_over,
            "MM_swapImage('own','images/own_f2.gif',1);"
        );
        assert_eq!(ec.on_mouse_out, "MM_swapImgRestore();");
        assert_eq!(ec.preload, vec!["images/own_f2.gif".to_string()]);
        assert!(n.swap && n.preload);
        assert!(runtime_js(&n).contains("function MM_swapImage()"));
    }
}
