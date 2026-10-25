//! Arbeitsbereich: Anordnung der Bedienfelder (Dock → Bedienfeldgruppe → Bedienfeld, 11 §4.1/§4.2),
//! Sichtbarkeit von Werkzeugleisten und Inspektor. Wird mit eframe gespeichert und kann unter
//! Namen abgelegt werden (Fenster ▸ Arbeitsbereichlayouts bzw. Umschalter oben rechts).

use crate::panels::PanelId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Eine Bedienfeldgruppe mit Reitern.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PanelGroup {
    pub panels: Vec<PanelId>,
    /// Aktiver Reiter (Index in `panels`).
    pub active: usize,
    /// Eingeklappt (nur Reiterleiste sichtbar).
    pub collapsed: bool,
    /// Höhenanteil im Dock.
    pub weight: f32,
}

impl PanelGroup {
    /// Neue Gruppe.
    pub fn new(panels: &[PanelId], collapsed: bool) -> PanelGroup {
        PanelGroup {
            panels: panels.to_vec(),
            active: 0,
            collapsed,
            weight: 1.0,
        }
    }
}

/// Schwebende Gruppe.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FloatingGroup {
    pub group: PanelGroup,
    pub pos: [f32; 2],
    pub size: [f32; 2],
}

/// Ort eines Bedienfelds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Location {
    /// Rechtes Dock, Gruppe `g`, Reiter `i`.
    Dock { group: usize, index: usize },
    /// Schwebende Gruppe `f`, Reiter `i`.
    Floating { group: usize, index: usize },
    /// Unten neben „Eigenschaften“ (Symboleigenschaften).
    Bottom,
}

/// Darstellungsart des Docks (Expanded/Iconic Mode, 11 §4.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DockMode {
    #[default]
    Expanded,
    Iconic,
    IconicWithNames,
}

/// Der Arbeitsbereich.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Workspace {
    /// Anzeigename des Layouts (Umschalter oben rechts).
    pub name: String,
    pub dock: Vec<PanelGroup>,
    pub floating: Vec<FloatingGroup>,
    /// Geschlossene Bedienfelder.
    pub closed: BTreeSet<PanelId>,
    pub dock_width: f32,
    pub dock_mode: DockMode,
    pub show_toolbar: bool,
    pub show_tools: bool,
    pub show_inspector: bool,
    /// Inspektor eingeklappt (nur Kopfzeile).
    pub inspector_collapsed: bool,
    /// Inspektor in halber Höhe (eine Zeile).
    pub inspector_half: bool,
    /// Dragged Properties height in pixels (content, without the splitter).
    /// `0` keeps the discrete full / half / collapsed height.
    #[serde(default)]
    pub inspector_height: f32,
    /// Aktiver Reiter unten (0 Eigenschaften, 1 Symboleigenschaften).
    pub bottom_active: usize,
    /// „Bedienfelder ausblenden“ (Tab/F4).
    pub panels_hidden: bool,
}

impl Default for Workspace {
    fn default() -> Self {
        Workspace::expanded()
    }
}

impl Workspace {
    /// „Erweiterter Modus“ (Standardarbeitsbereich von CS6, 11 §4.2 und Laufzeit T1).
    pub fn expanded() -> Workspace {
        use PanelId::*;
        let floating_closed = [
            Info,
            ColorMixer,
            Url,
            Behaviors,
            FindReplace,
            AutoShapeProperties,
            CssProperties,
        ];
        let mut floating = Vec::new();
        for (i, p) in floating_closed.iter().enumerate() {
            let o = 160.0 + 24.0 * i as f32;
            let size = if *p == AutoShapeProperties {
                [600.0, 300.0]
            } else {
                [240.0, 260.0]
            };
            floating.push(FloatingGroup {
                group: PanelGroup::new(&[*p], false),
                pos: [o + 200.0, o],
                size,
            });
        }
        let mut dock = vec![
            PanelGroup::new(&[Optimize, History, Align], false),
            PanelGroup::new(&[Pages, States, Layers], false),
            PanelGroup::new(&[Styles, ColorPalette, Swatches], false),
            PanelGroup::new(&[SpecialCharacters, ImageEditing, Path, AutoShapes], true),
            PanelGroup::new(&[DocumentLibrary, CommonLibrary], true),
        ];
        for g in &mut dock {
            g.weight = 1.0;
        }
        // wie im Original beim Start: in [Seiten, Status, Ebenen] ist Ebenen vorn
        dock[1].active = 2;
        Workspace {
            name: "Erweiterter Modus".into(),
            dock,
            floating,
            closed: floating_closed.iter().copied().collect(),
            dock_width: 268.0,
            dock_mode: DockMode::Expanded,
            show_toolbar: true,
            show_tools: true,
            show_inspector: true,
            inspector_collapsed: false,
            inspector_half: false,
            inspector_height: 0.0,
            bottom_active: 0,
            panels_hidden: false,
        }
    }

    /// Mitgelieferte Layouts (Name, Arbeitsbereich).
    pub fn presets() -> Vec<Workspace> {
        let mut iconic = Workspace::expanded();
        iconic.name = "Symbolmodus".into();
        iconic.dock_mode = DockMode::Iconic;
        let mut named = Workspace::expanded();
        named.name = "Symbolmodus mit Bedienfeldnamen".into();
        named.dock_mode = DockMode::IconicWithNames;
        vec![Workspace::expanded(), iconic, named]
    }

    /// Wo liegt ein Bedienfeld?
    pub fn locate(&self, p: PanelId) -> Option<Location> {
        if p == PanelId::SymbolProperties {
            return Some(Location::Bottom);
        }
        for (g, grp) in self.dock.iter().enumerate() {
            if let Some(i) = grp.panels.iter().position(|q| *q == p) {
                return Some(Location::Dock { group: g, index: i });
            }
        }
        for (g, f) in self.floating.iter().enumerate() {
            if let Some(i) = f.group.panels.iter().position(|q| *q == p) {
                return Some(Location::Floating { group: g, index: i });
            }
        }
        None
    }

    /// Ist das Bedienfeld sichtbar (offen und nicht eingeklappt bzw. aktiv)?
    pub fn is_visible(&self, p: PanelId) -> bool {
        !self.closed.contains(&p) && self.locate(p).is_some()
    }

    /// Sichtbare Bedienfelder einer Gruppe.
    pub fn visible_panels(&self, g: &PanelGroup) -> Vec<PanelId> {
        g.panels
            .iter()
            .copied()
            .filter(|p| !self.closed.contains(p))
            .collect()
    }

    /// Bedienfeld zeigen und nach vorn holen (Gruppe aufklappen).
    pub fn show(&mut self, p: PanelId) {
        self.closed.remove(&p);
        self.panels_hidden = false;
        match self.locate(p) {
            Some(Location::Dock { group, index }) => {
                if let Some(g) = self.dock.get_mut(group) {
                    g.active = index;
                    g.collapsed = false;
                }
            }
            Some(Location::Floating { group, index }) => {
                if let Some(f) = self.floating.get_mut(group) {
                    f.group.active = index;
                    f.group.collapsed = false;
                }
            }
            Some(Location::Bottom) => self.bottom_active = 1,
            // neue schwebende Gruppen gestaffelt, nicht alle an derselben Stelle (CT-C-19)
            None => {
                let k = self.floating.len() as f32 % 8.0;
                self.floating.push(FloatingGroup {
                    group: PanelGroup::new(&[p], false),
                    pos: [300.0 + 28.0 * k, 200.0 + 28.0 * k],
                    size: [240.0, 260.0],
                })
            }
        }
    }

    /// Ein-/Ausblenden (Fenster-Menü): sichtbar und vorn → schließen, sonst zeigen.
    pub fn toggle(&mut self, p: PanelId) {
        if self.is_visible(p) && self.is_front(p) {
            self.closed.insert(p);
            if p == PanelId::SymbolProperties {
                self.bottom_active = 0;
            }
        } else {
            self.show(p);
        }
    }

    /// Ist das Bedienfeld der aktive Reiter einer aufgeklappten Gruppe?
    pub fn is_front(&self, p: PanelId) -> bool {
        match self.locate(p) {
            Some(Location::Dock { group, .. }) => self
                .dock
                .get(group)
                .is_some_and(|g| self.active_panel(g) == Some(p) && !g.collapsed),
            Some(Location::Floating { group, .. }) => self
                .floating
                .get(group)
                .is_some_and(|f| self.active_panel(&f.group) == Some(p)),
            // Symboleigenschaften teilen sich die untere Gruppe mit dem Eigenschaften-Inspektor: vorn
            // nur als aktiver Reiter (CT-C-11: ✓ im Fenster-Menü auch als hinterer Reiter)
            Some(Location::Bottom) => self.bottom_active == 1,
            None => false,
        }
    }

    /// Aktives sichtbares Bedienfeld einer Gruppe.
    pub fn active_panel(&self, g: &PanelGroup) -> Option<PanelId> {
        let vis = self.visible_panels(g);
        let want = g.panels.get(g.active).copied();
        match want {
            Some(p) if vis.contains(&p) => Some(p),
            _ => vis.first().copied(),
        }
    }

    /// Schließt alle Bedienfelder der Gruppe, in der `p` liegt.
    pub fn close_group_of(&mut self, p: PanelId) {
        let panels: Vec<PanelId> = match self.locate(p) {
            Some(Location::Dock { group, .. }) => self
                .dock
                .get(group)
                .map(|g| g.panels.clone())
                .unwrap_or_default(),
            Some(Location::Floating { group, .. }) => self
                .floating
                .get(group)
                .map(|f| f.group.panels.clone())
                .unwrap_or_default(),
            _ => vec![p],
        };
        self.closed.extend(panels);
    }

    fn remove_panel(&mut self, p: PanelId) {
        for g in &mut self.dock {
            if let Some(i) = g.panels.iter().position(|q| *q == p) {
                g.panels.remove(i);
                if g.active >= g.panels.len() {
                    g.active = g.panels.len().saturating_sub(1);
                }
            }
        }
        for f in &mut self.floating {
            if let Some(i) = f.group.panels.iter().position(|q| *q == p) {
                f.group.panels.remove(i);
                if f.group.active >= f.group.panels.len() {
                    f.group.active = f.group.panels.len().saturating_sub(1);
                }
            }
        }
        self.dock.retain(|g| !g.panels.is_empty());
        self.floating.retain(|f| !f.group.panels.is_empty());
    }

    /// Verschiebt ein Bedienfeld in eine Dock-Gruppe (an Reiterposition `index`).
    pub fn move_to_dock_group(&mut self, p: PanelId, group: usize, index: usize) {
        if p == PanelId::SymbolProperties {
            return;
        }
        let target = self.dock.get(group).cloned();
        self.remove_panel(p);
        // Gruppe nach dem Entfernen wiederfinden (Indizes können sich verschoben haben)
        let gi = match target {
            Some(t) => self
                .dock
                .iter()
                .position(|g| g.panels.iter().any(|q| t.panels.contains(q))),
            None => None,
        };
        match gi {
            Some(gi) => {
                let g = &mut self.dock[gi];
                let at = index.min(g.panels.len());
                g.panels.insert(at, p);
                g.active = at;
                g.collapsed = false;
            }
            None => self.dock.push(PanelGroup::new(&[p], false)),
        }
        self.closed.remove(&p);
    }

    /// Neue Dock-Gruppe an Position `at` mit einem Bedienfeld.
    pub fn new_dock_group(&mut self, p: PanelId, at: usize) {
        if p == PanelId::SymbolProperties {
            return;
        }
        let before: Vec<Vec<PanelId>> = self.dock.iter().map(|g| g.panels.clone()).collect();
        self.remove_panel(p);
        // Position nach dem Entfernen anpassen
        let mut pos = 0;
        for (i, g) in before.iter().enumerate() {
            if i >= at {
                break;
            }
            if g.iter().any(|q| *q != p) {
                pos += 1;
            }
        }
        let pos = pos.min(self.dock.len());
        self.dock.insert(pos, PanelGroup::new(&[p], false));
        self.closed.remove(&p);
    }

    /// Verschiebt ein Bedienfeld in eine schwebende Gruppe.
    pub fn move_to_floating_group(&mut self, p: PanelId, group: usize, index: usize) {
        if p == PanelId::SymbolProperties {
            return;
        }
        let target = self.floating.get(group).map(|f| f.group.panels.clone());
        self.remove_panel(p);
        let fi = target.and_then(|t| {
            self.floating
                .iter()
                .position(|f| f.group.panels.iter().any(|q| t.contains(q)))
        });
        match fi {
            Some(fi) => {
                let g = &mut self.floating[fi].group;
                let at = index.min(g.panels.len());
                g.panels.insert(at, p);
                g.active = at;
            }
            None => self.floating.push(FloatingGroup {
                group: PanelGroup::new(&[p], false),
                pos: [300.0, 200.0],
                size: [240.0, 260.0],
            }),
        }
        self.closed.remove(&p);
    }

    /// Löst ein Bedienfeld als schwebende Gruppe an `pos`.
    pub fn float_panel(&mut self, p: PanelId, pos: [f32; 2]) {
        if p == PanelId::SymbolProperties {
            return;
        }
        let size = [p.preferred_height().max(220.0), p.preferred_height() + 20.0];
        self.remove_panel(p);
        self.floating.push(FloatingGroup {
            group: PanelGroup::new(&[p], false),
            pos,
            size,
        });
        self.closed.remove(&p);
    }

    /// Dockt eine schwebende Gruppe als neue Dock-Gruppe an.
    pub fn dock_floating(&mut self, f: usize, at: usize) {
        if f < self.floating.len() {
            let g = self.floating.remove(f).group;
            let at = at.min(self.dock.len());
            self.dock.insert(at, g);
        }
    }

    /// Plausibilität nach dem Laden (neue Bedienfelder ergänzen, Doppelte entfernen).
    pub fn repair(&mut self) {
        let mut seen = BTreeSet::new();
        for g in &mut self.dock {
            g.panels.retain(|p| seen.insert(*p));
        }
        for f in &mut self.floating {
            f.group.panels.retain(|p| seen.insert(*p));
        }
        self.dock.retain(|g| !g.panels.is_empty());
        self.floating.retain(|f| !f.group.panels.is_empty());
        for p in PanelId::ALL {
            if *p != PanelId::SymbolProperties && !seen.contains(p) {
                self.floating.push(FloatingGroup {
                    group: PanelGroup::new(&[*p], false),
                    pos: [300.0, 200.0],
                    size: [240.0, 260.0],
                });
                self.closed.insert(*p);
            }
        }
        if !self.dock_width.is_finite() || self.dock_width < 120.0 {
            self.dock_width = 268.0;
        }
        if !self.inspector_height.is_finite() || self.inspector_height < 0.0 {
            self.inspector_height = 0.0;
        }
        for g in &mut self.dock {
            if !g.weight.is_finite() || g.weight <= 0.05 {
                g.weight = 1.0;
            }
        }
    }
}

/// Sprachschlüssel eines mitgelieferten Layouts. Gespeichert (eframe, Aktionen, Vergleich mit dem
/// aktiven Layout) wird immer die feste deutsche Kennung aus [`Workspace::presets`]; angezeigt wird
/// der Text des Schlüssels (`appbar::workspace_display_name`).
pub fn preset_key(name: &str) -> Option<&'static str> {
    match name {
        "Erweiterter Modus" => Some("ui-workspace-expanded"),
        "Symbolmodus" => Some("ui-workspace-iconic"),
        "Symbolmodus mit Bedienfeldnamen" => Some("ui-workspace-iconic-names"),
        _ => None,
    }
}

// ---------------------------------------------------------------------- Arbeitsbereich-XML

/// Befehls-ID des Eigenschaften-Inspektors im Arbeitsbereich-XML.
const PROPERTIES_APP_DATA: u32 = 35586;

fn panel_from_app_data(id: u32) -> Option<PanelId> {
    PanelId::ALL.iter().copied().find(|p| p.fw_id() == id)
}

/// Ein Element des Arbeitsbereich-XML (Name, Attribute, Kinder).
#[derive(Clone, Debug, Default, PartialEq)]
struct XmlNode {
    name: String,
    attrs: Vec<(String, String)>,
    children: Vec<XmlNode>,
}

impl XmlNode {
    fn attr(&self, k: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(a, _)| a == k)
            .map(|(_, v)| v.as_str())
    }
    fn flag(&self, k: &str) -> bool {
        self.attr(k) == Some("true")
    }
    fn pair(&self, k: &str) -> Option<[f32; 2]> {
        let mut it = self
            .attr(k)?
            .split_whitespace()
            .filter_map(|x| x.parse::<f32>().ok());
        Some([it.next()?, it.next()?])
    }
}

fn xml_unescape(s: &str) -> String {
    s.replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// Kleiner XML-Leser für das Arbeitsbereichformat (nur Elemente und Attribute).
fn parse_xml(text: &str) -> Result<XmlNode, String> {
    let mut stack: Vec<XmlNode> = vec![XmlNode {
        name: "#root".into(),
        ..Default::default()
    }];
    let mut rest = text;
    while let Some(start) = rest.find('<') {
        rest = &rest[start + 1..];
        let end = rest.find('>').ok_or("unvollständiges Element")?;
        let tag = &rest[..end];
        rest = &rest[end + 1..];
        if tag.starts_with('?') || tag.starts_with('!') {
            continue;
        }
        if let Some(name) = tag.strip_prefix('/') {
            let node = stack.pop().ok_or("unerwartetes Ende")?;
            if node.name != name.trim() {
                return Err(format!(
                    "Ende </{}> passt nicht zu <{}>",
                    name.trim(),
                    node.name
                ));
            }
            stack
                .last_mut()
                .ok_or("unerwartetes Ende")?
                .children
                .push(node);
            continue;
        }
        let self_closing = tag.ends_with('/');
        let tag = tag.trim_end_matches('/');
        let mut parts = tag.splitn(2, char::is_whitespace);
        let name = parts.next().unwrap_or("").to_string();
        let mut attrs = Vec::new();
        let mut a = parts.next().unwrap_or("");
        while let Some(eq) = a.find('=') {
            let key = a[..eq].trim().to_string();
            let after = a[eq + 1..].trim_start();
            let q = after.chars().next().ok_or("Attributwert fehlt")?;
            if q != '"' && q != '\'' {
                return Err(format!("Attribut {key} ohne Anführungszeichen"));
            }
            let close = after[1..].find(q).ok_or("Attributwert nicht beendet")?;
            attrs.push((key, xml_unescape(&after[1..1 + close])));
            a = &after[close + 2..];
        }
        let node = XmlNode {
            name,
            attrs,
            children: Vec::new(),
        };
        if self_closing {
            stack
                .last_mut()
                .ok_or("ungültige Struktur")?
                .children
                .push(node);
        } else {
            stack.push(node);
        }
    }
    if stack.len() != 1 {
        return Err("nicht geschlossene Elemente".into());
    }
    let root = stack.pop().unwrap_or_default();
    root.children
        .into_iter()
        .find(|n| n.name == "workspace")
        .ok_or_else(|| "kein <workspace>-Element".into())
}

impl Workspace {
    /// Liest ein Arbeitsbereichlayout im XML-Format des Originals (`readPanelStateFromFile`,
    /// 11 §4.2): Dock rechts (Gruppen, aktiver Reiter, minimiert = eingeklappt, Symbolmodus),
    /// schwebende Gruppen (Ursprung, Größe, geschlossen), Werkzeugleiste, Hauptsymbolleiste,
    /// Eigenschaften/Symboleigenschaften unten. Unbekannte Bedienfelder werden übersprungen.
    pub fn from_panel_state_xml(text: &str, name: &str) -> Result<Workspace, String> {
        let root = parse_xml(text)?;
        let mut w = Workspace {
            name: name.to_string(),
            dock: Vec::new(),
            floating: Vec::new(),
            closed: BTreeSet::new(),
            ..Workspace::expanded()
        };
        w.show_tools = false;
        w.show_toolbar = false;
        w.show_inspector = false;
        for dock in root.children.iter().filter(|n| n.name == "dock") {
            let anchor = dock.attr("anchor").unwrap_or("none");
            let dock_closed = dock.flag("is-closed");
            for item in &dock.children {
                match item.name.as_str() {
                    "toolbar" if item.attr("app-data") == Some("ToolPalette") => {
                        w.show_tools = !dock_closed && !item.flag("is-closed")
                    }
                    "control-bar" if item.attr("app-data") == Some("MainToolBar") => {
                        w.show_toolbar = !dock_closed && !item.flag("is-closed")
                    }
                    "tab-pane" => {
                        if anchor == "right" {
                            let iconic = item.attr("mode") == Some("icon");
                            let len: f32 = item
                                .attr("preferred-iconic-length")
                                .and_then(|v| v.parse().ok())
                                .unwrap_or(0.0);
                            w.dock_mode = if !iconic {
                                DockMode::Expanded
                            } else if len > 100.0 {
                                DockMode::IconicWithNames
                            } else {
                                DockMode::Iconic
                            };
                        }
                        for tg in item.children.iter().filter(|n| n.name == "tab-group") {
                            let mut group = PanelGroup::new(&[], false);
                            let mut size = [240.0, 260.0];
                            let active_id = tg.attr("active-palette").unwrap_or("");
                            for pal in tg.children.iter().filter(|n| n.name == "palette") {
                                let app: u32 = pal
                                    .attr("app-data")
                                    .and_then(|v| v.parse().ok())
                                    .unwrap_or(0);
                                if app == PROPERTIES_APP_DATA {
                                    w.show_inspector = !dock_closed && !pal.flag("is-closed");
                                    continue;
                                }
                                let Some(p) = panel_from_app_data(app) else {
                                    continue;
                                };
                                if p == PanelId::SymbolProperties {
                                    if dock_closed || pal.flag("is-closed") {
                                        w.closed.insert(p);
                                    }
                                    if pal.attr("id") == Some(active_id) {
                                        w.bottom_active = 1;
                                    }
                                    continue;
                                }
                                if pal.attr("id") == Some(active_id) {
                                    group.active = group.panels.len();
                                    group.collapsed = pal.flag("is-minimized")
                                        || tg.attr("is-drawer-open") == Some("false");
                                    if let Some(s) = pal.pair("preferred-unconstrained-size") {
                                        size = s;
                                    }
                                }
                                if dock_closed || tg.flag("is-closed") || pal.flag("is-closed") {
                                    w.closed.insert(p);
                                }
                                group.panels.push(p);
                            }
                            if group.panels.is_empty() {
                                continue;
                            }
                            if anchor == "right" {
                                w.dock.push(group);
                            } else {
                                let base = dock.pair("origin").unwrap_or([300.0, 200.0]);
                                let k = w.floating.len() as f32;
                                w.floating.push(FloatingGroup {
                                    group,
                                    pos: [base[0] + 12.0 * k, base[1] + 12.0 * k],
                                    size,
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        if w.dock.is_empty() && w.floating.is_empty() {
            return Err("keine bekannten Bedienfelder im Layout".into());
        }
        w.repair();
        Ok(w)
    }

    /// Schreibt den Arbeitsbereich im XML-Format des Originals (`writePanelStateToFile`).
    pub fn to_panel_state_xml(&self) -> String {
        fn palette(out: &mut String, app: u32, closed: bool, minimized: bool, size: [f32; 2]) {
            out.push_str(&format!(
                "\t\t\t\t<palette id=\"{app:08X}\" is-closed=\"{closed}\" preferred-unconstrained-size=\"{} {}\" preferred-constrained-size=\"{} {}\" current-state=\"1\" is-minimized=\"{minimized}\" app-data=\"{app}\" host-cached-sizes=\"false\"/>\n",
                size[0] as i32, size[1] as i32, size[0] as i32, size[1] as i32
            ));
        }
        let mut out = String::from("<workspace version=\"1\">\n");
        let (mode, len) = match self.dock_mode {
            DockMode::Expanded => ("expanded", 104),
            DockMode::Iconic => ("icon", 35),
            DockMode::IconicWithNames => ("icon", 173),
        };
        out.push_str("\t<dock anchor=\"right\" content=\"palette toolbar\" is-closed=\"false\">\n");
        out.push_str(&format!("\t\t<tab-pane mode=\"{mode}\" preferred-iconic-length=\"{len}\" layout-mode=\"auto-flow\">\n"));
        for g in &self.dock {
            let active = g.panels.get(g.active).copied().unwrap_or(g.panels[0]);
            out.push_str(&format!(
                "\t\t\t<tab-group active-palette=\"{:08X}\" is-closed=\"false\">\n",
                active.fw_id()
            ));
            for p in &g.panels {
                palette(
                    &mut out,
                    p.fw_id(),
                    self.closed.contains(p),
                    *p == active && g.collapsed,
                    [self.dock_width, p.preferred_height()],
                );
            }
            out.push_str("\t\t\t</tab-group>\n");
        }
        out.push_str("\t\t</tab-pane>\n\t</dock>\n");
        out.push_str(&format!(
            "\t<dock anchor=\"left\" content=\"palette toolbar\" is-closed=\"{}\">\n",
            !self.show_tools
        ));
        out.push_str(&format!("\t\t<toolbar id=\"00000001\" origin=\"1 24\" size-variant=\"vertical-wide\" is-closed=\"{}\" app-data=\"ToolPalette\"/>\n\t</dock>\n", !self.show_tools));
        out.push_str(&format!(
            "\t<dock anchor=\"top\" content=\"control-bar\" is-closed=\"{}\">\n",
            !self.show_toolbar
        ));
        out.push_str(&format!("\t\t<control-bar id=\"00000002\" origin=\"0 0\" size=\"778 28\" is-closed=\"{}\" app-data=\"MainToolBar\"/>\n\t</dock>\n", !self.show_toolbar));
        out.push_str("\t<dock anchor=\"bottom\" content=\"palette\" is-closed=\"false\">\n\t\t<tab-pane mode=\"expanded\" preferred-iconic-length=\"0\" layout-mode=\"auto-flow\">\n");
        let bottom_active = if self.bottom_active == 1 {
            PanelId::SymbolProperties.fw_id()
        } else {
            PROPERTIES_APP_DATA
        };
        out.push_str(&format!(
            "\t\t\t<tab-group active-palette=\"{bottom_active:08X}\" is-closed=\"false\">\n"
        ));
        palette(
            &mut out,
            PROPERTIES_APP_DATA,
            !self.show_inspector,
            self.inspector_collapsed,
            [923.0, 105.0],
        );
        palette(
            &mut out,
            PanelId::SymbolProperties.fw_id(),
            self.closed.contains(&PanelId::SymbolProperties),
            false,
            [214.0, 300.0],
        );
        out.push_str("\t\t\t</tab-group>\n\t\t</tab-pane>\n\t</dock>\n");
        for f in &self.floating {
            let all_closed = f.group.panels.iter().all(|p| self.closed.contains(p));
            let active = f
                .group
                .panels
                .get(f.group.active)
                .copied()
                .unwrap_or(f.group.panels[0]);
            out.push_str(&format!("\t<dock anchor=\"none\" content=\"palette toolbar\" origin=\"{} {}\" is-closed=\"{all_closed}\">\n", f.pos[0] as i32, f.pos[1] as i32));
            out.push_str("\t\t<tab-pane mode=\"expanded\" preferred-iconic-length=\"0\" layout-mode=\"manual\">\n");
            out.push_str(&format!(
                "\t\t\t<tab-group active-palette=\"{:08X}\" is-closed=\"false\">\n",
                active.fw_id()
            ));
            for p in &f.group.panels {
                palette(&mut out, p.fw_id(), self.closed.contains(p), false, f.size);
            }
            out.push_str("\t\t\t</tab-group>\n\t\t</tab-pane>\n\t</dock>\n");
        }
        out.push_str("</workspace>\n");
        out
    }
}

/// Ordner der Arbeitsbereichlayouts (`%APPDATA%\Firepixel\Workspace Layouts`): gespeicherte
/// Layouts als XML im Format des Originals.
pub fn layouts_dir() -> Option<std::path::PathBuf> {
    fp_engine::Preferences::default_path()
        .and_then(|p| p.parent().map(|d| d.join("Workspace Layouts")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_state_xml_roundtrip() {
        let mut w = Workspace::expanded();
        w.dock[2].active = 2;
        w.dock_mode = DockMode::IconicWithNames;
        w.show_toolbar = false;
        let xml = w.to_panel_state_xml();
        let back = Workspace::from_panel_state_xml(&xml, &w.name).unwrap_or_default();
        assert_eq!(
            back.dock
                .iter()
                .map(|g| g.panels.clone())
                .collect::<Vec<_>>(),
            w.dock.iter().map(|g| g.panels.clone()).collect::<Vec<_>>()
        );
        assert_eq!(back.dock[2].active, 2);
        assert_eq!(back.dock[3].collapsed, w.dock[3].collapsed);
        assert_eq!(back.dock_mode, DockMode::IconicWithNames);
        assert!(!back.show_toolbar && back.show_tools && back.show_inspector);
        assert_eq!(back.closed, w.closed);
    }

    #[test]
    fn reads_original_layouts() {
        // mitgelieferte Layouts des Originals (nur lesen, falls die Installationskopie vorhanden ist)
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..")
            .join("korpus")
            .join("Configuration")
            .join("Commands")
            .join("Workspace Layouts")
            .join("Win");
        let Ok(rd) = std::fs::read_dir(&dir) else {
            return;
        };
        let mut n = 0;
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "xml") {
                let text = std::fs::read_to_string(&p).unwrap_or_default();
                let w = Workspace::from_panel_state_xml(&text, "x");
                assert!(w.is_ok(), "{}: {:?}", p.display(), w.err());
                let w = w.unwrap_or_default();
                assert!(w.locate(PanelId::Layers).is_some());
                if p.file_stem().is_some_and(|s| s == "Expanded Mode") {
                    let exp = Workspace::expanded();
                    assert_eq!(
                        w.dock.iter().map(|g| g.panels.clone()).collect::<Vec<_>>(),
                        exp.dock
                            .iter()
                            .map(|g| g.panels.clone())
                            .collect::<Vec<_>>()
                    );
                    assert!(!w.is_visible(PanelId::Info));
                    assert!(w.dock[3].collapsed && w.dock[4].collapsed);
                }
                n += 1;
            }
        }
        assert!(n >= 3);
    }

    #[test]
    fn default_layout_like_cs6() {
        let w = Workspace::expanded();
        assert_eq!(w.dock.len(), 5);
        assert_eq!(
            w.dock[1].panels,
            vec![PanelId::Pages, PanelId::States, PanelId::Layers]
        );
        assert!(w.is_visible(PanelId::Layers));
        assert!(!w.is_visible(PanelId::Info));
        assert!(w.dock[3].collapsed);
    }

    #[test]
    fn toggle_and_move() {
        let mut w = Workspace::expanded();
        w.toggle(PanelId::Info);
        assert!(w.is_visible(PanelId::Info));
        w.toggle(PanelId::Info);
        assert!(!w.is_visible(PanelId::Info));
        w.move_to_dock_group(PanelId::Layers, 0, 1);
        assert_eq!(w.dock[0].panels[1], PanelId::Layers);
        assert_eq!(
            w.locate(PanelId::Layers),
            Some(Location::Dock { group: 0, index: 1 })
        );
        w.float_panel(PanelId::Pages, [10.0, 10.0]);
        assert!(matches!(
            w.locate(PanelId::Pages),
            Some(Location::Floating { .. })
        ));
        w.new_dock_group(PanelId::Pages, 0);
        assert_eq!(w.dock[0].panels, vec![PanelId::Pages]);
        let json = serde_json::to_string(&w).unwrap_or_default();
        let back: Workspace = serde_json::from_str(&json).unwrap_or_default();
        assert_eq!(back, w);
        let old: serde_json::Value = serde_json::from_str(&json).expect("json");
        let mut old = old;
        old.as_object_mut()
            .expect("object")
            .remove("inspector_height");
        let legacy: Workspace = serde_json::from_value(old).expect("legacy workspace");
        assert_eq!(legacy.inspector_height, 0.0);
    }

    #[test]
    fn repair_adds_missing() {
        let mut w = Workspace::expanded();
        w.dock.clear();
        w.floating.clear();
        w.repair();
        assert!(w.locate(PanelId::Layers).is_some());
    }
}
