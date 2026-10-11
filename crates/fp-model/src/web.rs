//! Web-Objekte (Hotspot/Slice, `URLv`), Verhalten (`BEVv{BEHv…}`) und Pop-up-Menüs (Spezifikation 15).

use crate::element::Contour;
use crate::export::ExportSettings;
use crate::raw::Extra;
use crate::serde_ext;
use crate::style::Style;
use fp_color::Argb;
use fp_geom::Rect;
use serde::{Deserialize, Serialize};

/// Form eines Web-Objekts (`URSi`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WebShape {
    Rect,
    Circle,
    Polygon,
    Other(i32),
}

impl WebShape {
    pub fn from_code(v: i32) -> WebShape {
        match v {
            0 => WebShape::Rect,
            1 => WebShape::Circle,
            2 => WebShape::Polygon,
            o => WebShape::Other(o),
        }
    }
    pub fn code(self) -> i32 {
        match self {
            WebShape::Rect => 0,
            WebShape::Circle => 1,
            WebShape::Polygon => 2,
            WebShape::Other(o) => o,
        }
    }
}

/// Hotspot bzw. Slice (`URLv`, Tags 15 §1.2). Slices tragen `TSLi` (dann ist `slice` gesetzt).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WebObjectElem {
    /// Link-URL (`INMs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Alt-Text (`A2Ts`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alt_text: Option<String>,
    /// Ziel (`ALTs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// Überlagerungsfarbe im Editor (`FCLi`; Slices grün `ff00ff00`, Hotspots `ff00ffff`).
    #[serde(with = "serde_ext::argb")]
    pub color: Argb,
    /// Form (`URSi`).
    pub shape: WebShape,
    /// Rahmen in Dokumentpixeln (`LFTf TOPf RITf BOTf`).
    #[serde(with = "serde_ext::rect")]
    pub bounds: Rect,
    /// Polygonkontur (`PBPv`, nur bei Polygon).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polygon: Option<Contour>,
    /// Basis-Dateiname (`FILs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    /// Slice-Eigenschaften.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slice: Option<SliceProps>,
    /// Verhalten (`BEVv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub behaviors: Option<Behaviors>,
    /// Statusleistentext alter Dateien (`UNMs`, nur lesen; das Original macht daraus ein Verhalten).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legacy_status: Option<String>,
}

/// Slice-Eigenschaften (15 §1.2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SliceProps {
    /// Slice-Art (`TSLi`): 0 Bild, 2 Hintergrundbild, 3 leer/HTML, 1 alte Rollover-Slice.
    pub slice_type: i32,
    /// HTML-Text einer HTML-Slice (`HTMs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub html: Option<String>,
    /// Zusatzattribute der `<td>` (`TDTs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub td_text: Option<String>,
    /// Slice-ID (`MSNi`), Ziel von Austausch-Verhalten; das Original vergibt sie beim Laden neu.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slice_id: Option<u32>,
    /// Eigene Exporteinstellungen (`EXPv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub export: Option<Box<ExportSettings>>,
    /// Hintergrund-Optionen (`CBTb CBRi CBAi CBHi CHVi CBVi CVVi`); Rohwerte bleiben erhalten, auch
    /// wenn sie außerhalb der Enums liegen (15 §1.2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<SliceBackground>,
}

/// Hintergrund-Optionen einer Slice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SliceBackground {
    /// `CBTb`: Slice ist Hintergrundbild.
    pub is_background: bool,
    /// `CBRi`: 0 no-repeat, 1 repeat, 2 repeat-x, 3 repeat-y.
    pub repeat: i32,
    /// `CBAi`: 0 fixed, 1 scroll.
    pub attachment: i32,
    /// `CBHi`: 0 left, 1 center, 2 right, 3 Wert.
    pub h_align: i32,
    /// `CHVi`: Pixelwert (sonst −1).
    pub h_value: i32,
    /// `CBVi`: 0 top, 1 center, 2 bottom, 3 Wert.
    pub v_align: i32,
    /// `CVVi`: Pixelwert (sonst −1).
    pub v_value: i32,
}

impl Default for SliceBackground {
    fn default() -> Self {
        SliceBackground { is_background: false, repeat: 0, attachment: 1, h_align: 1, h_value: -1, v_align: 1, v_value: -1 }
    }
}

/// Verhaltensliste (`BEVv{BEHv…}`), Reihenfolge wie im Verhalten-Bedienfeld.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Behaviors {
    pub items: Vec<Behavior>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

/// Verhaltensart (`BETi`, 15 §2.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BehaviorAction {
    /// 0 bzw. 3: Navigationsleistenbild festlegen (`CDbButtonRollover`).
    NavBar,
    /// 1: Statusleistentext.
    StatusMessage,
    /// 2: Bild austauschen.
    SwapImage,
    /// 3: Altwert von 0.
    NavBarLegacy,
    /// 4: Navigationsleiste – Down.
    NavBarDown,
    /// 5: Bildaustausch wiederherstellen.
    SwapImageRestore,
    /// 6: Navigationsleiste – Over.
    NavBarOver,
    /// 7: Navigationsleiste wiederherstellen.
    NavBarRestore,
    /// 8: Einfaches Rollover.
    SimpleRollover,
    /// 9: Pop-up-Menü.
    PopupMenu,
    /// 10: eigenes Verhalten (HTML-Import).
    Custom,
    /// 11: Sound abspielen.
    PlaySound,
    Other(i32),
}

impl BehaviorAction {
    pub fn from_code(v: i32) -> Self {
        use BehaviorAction::*;
        match v {
            0 => NavBar,
            1 => StatusMessage,
            2 => SwapImage,
            3 => NavBarLegacy,
            4 => NavBarDown,
            5 => SwapImageRestore,
            6 => NavBarOver,
            7 => NavBarRestore,
            8 => SimpleRollover,
            9 => PopupMenu,
            10 => Custom,
            11 => PlaySound,
            o => Other(o),
        }
    }
    pub fn code(self) -> i32 {
        use BehaviorAction::*;
        match self {
            NavBar => 0,
            StatusMessage => 1,
            SwapImage => 2,
            NavBarLegacy => 3,
            NavBarDown => 4,
            SwapImageRestore => 5,
            NavBarOver => 6,
            NavBarRestore => 7,
            SimpleRollover => 8,
            PopupMenu => 9,
            Custom => 10,
            PlaySound => 11,
            Other(o) => o,
        }
    }
}

/// Ereignis (`BEEi`): 0 onMouseOver, 1 onClick, 2 onMouseOut, 3 onLoad.
pub const BEHAVIOR_EVENTS: [&str; 4] = ["onMouseOver", "onClick", "onMouseOut", "onLoad"];

/// Ein Verhalten (`BEHv`). Alle artabhängigen Felder sind optional: Das Original schreibt je Art nur die
/// passenden Tags; fehlende gelten mit den Lesevorgaben aus 15 §2.2.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Behavior {
    /// Art (`BETi`, Pflicht).
    pub action: i32,
    /// Ereignis (`BEEi`, Pflicht).
    pub event: i32,
    /// Ziel-Slice (`MSNi`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_slice: Option<u32>,
    /// 0-basierter Status (`FRCi`, Vorgabe 2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<i32>,
    /// Down-Highlight-Status (`FRTi`, Vorgabe 3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub down_frame: Option<i32>,
    /// Externe Datei bzw. Sounddatei (`FILs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Down-Highlight-Datei (`INMs`, Art 6).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub down_file: Option<String>,
    /// Vorladen (`PREb`, Vorgabe true).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preload: Option<bool>,
    /// Bei onMouseOut wiederherstellen (`RMOb`, Vorgabe true, nur Art 2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restore: Option<bool>,
    /// Flags (`ROLi`): Bit 1 Navigationsleiste, Bit 0 OverDown einschließen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flags: Option<i32>,
    /// „Beim Laden Down-Bild anzeigen“ (`CHKb`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checked: Option<bool>,
    /// Statustext (`UNMs`, Art 1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_text: Option<String>,
    /// `ENDb` (Altlast, gelesen und verworfen).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<bool>,
    /// Pop-up-Menü (Art 9).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub popup: Option<Box<PopupMenu>>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

impl Behavior {
    /// Neues Verhalten ohne artabhängige Felder.
    pub fn new(action: BehaviorAction, event: i32) -> Behavior {
        Behavior {
            action: action.code(),
            event,
            target_slice: None,
            frame: None,
            down_frame: None,
            file: None,
            down_file: None,
            preload: None,
            restore: None,
            flags: None,
            checked: None,
            status_text: None,
            end: None,
            popup: None,
            extra: Extra::default(),
        }
    }

    pub fn kind(&self) -> BehaviorAction {
        BehaviorAction::from_code(self.action)
    }
}

/// Pop-up-Menü (`BEHv` mit `BETi = 9`, Tags 15 §3.2). Alle Felder optional (Lesevorgaben in 15 §3.2).
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct PopupMenu {
    /// Erstellungsdatum `YYYYMMDDhhmmss` (`CDTs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creation_date: Option<String>,
    /// Menüposition relativ zur Slice (`XLCf`, `YLCf`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub horz_offset: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vert_offset: Option<f32>,
    /// Menübaum als Text (`TRNs`, Grammatik 15 §3.3; siehe [`parse_menu_items`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub items_text: Option<String>,
    /// Textfarbe Up/Over (`FCLi`, `FETi`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_up_color: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_over_color: Option<u32>,
    /// Textausrichtung (`JSTi`) und vertikale Ausrichtung (`VERi`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_alignment: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vert_alignment: Option<i32>,
    /// Ausblendverzögerung in ms (`TMOi`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hide_timeout: Option<i32>,
    /// Untermenü-Versatz (`XSLf`, `YSLf`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub horz_submenu_offset: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vert_submenu_offset: Option<f32>,
    /// `RELb` (submenuRelativeToItem).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub submenu_relative: Option<bool>,
    /// Zellenabstand/-auffüllung (`CSPi`, `CPDi`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_spacing: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_padding: Option<i32>,
    /// Deckender Hintergrund (`OPQb`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opaque: Option<bool>,
    /// Hervorhebungs-, Schatten-, Rahmenfarbe (`HICi`, `CSHi`, `BOCi`), Rahmenbreite (`BOSi`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hilite_color: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow_color: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_color: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_size: Option<i32>,
    /// Breite/Höhe nicht automatisch (`OVWb`, `OVHb`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub override_width: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub override_height: Option<bool>,
    /// Texteinzug (`MITi`), vertikal (`MIVb`), Menü-ID (`MIDi`), Zellgröße (`MIWi`, `MIHi`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_indent: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub menu_id: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub menu_width: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub menu_height: Option<i32>,
    /// Bei onMouseOut ausblenden (`HIMb`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hide_on_mouseout: Option<bool>,
    /// Schrift: fett, kursiv, Familie, Größe (`BOLb`, `ITLb`, `FONs`, `PTSf`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<f32>,
    /// Nur Text/HTML-Zellen (`ATEb`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_only: Option<bool>,
    /// Zellfarbe Up/Over (`BCLi`, `BCMi`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell_up_color: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell_over_color: Option<u32>,
    /// Stile der Grafik-Zellen: erster = Up, zweiter = Over (`ZSTv` ×2).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub styles: Vec<Style>,
}

/// Menüeintrag eines Pop-up-Menüs.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct MenuItem {
    pub text: String,
    pub url: String,
    pub target: String,
    pub submenu: Vec<MenuItem>,
}

/// Liest den Menübaum (`TRNs`, 15 §3.3): `liste := '{' eintrag* '}'`,
/// `eintrag := '{' str ',' str ',' str ',' liste '}'`, `str := '"' … '"'` (endet an `",`; kein Escaping).
pub fn parse_menu_items(s: &str) -> Option<Vec<MenuItem>> {
    let b: Vec<char> = s.chars().collect();
    let mut i = 0usize;
    let r = parse_list(&b, &mut i)?;
    Some(r)
}

fn parse_list(b: &[char], i: &mut usize) -> Option<Vec<MenuItem>> {
    if b.get(*i) != Some(&'{') {
        return None;
    }
    *i += 1;
    let mut out = Vec::new();
    loop {
        match b.get(*i) {
            Some('}') => {
                *i += 1;
                return Some(out);
            }
            Some('{') => {
                *i += 1;
                let text = parse_str(b, i)?;
                expect(b, i, ',')?;
                let url = parse_str(b, i)?;
                expect(b, i, ',')?;
                let target = parse_str(b, i)?;
                expect(b, i, ',')?;
                let submenu = parse_list(b, i)?;
                expect(b, i, '}')?;
                out.push(MenuItem { text, url, target, submenu });
            }
            _ => return None,
        }
    }
}

fn expect(b: &[char], i: &mut usize, c: char) -> Option<()> {
    if b.get(*i) == Some(&c) {
        *i += 1;
        Some(())
    } else {
        None
    }
}

fn parse_str(b: &[char], i: &mut usize) -> Option<String> {
    expect(b, i, '"')?;
    let start = *i;
    while *i + 1 < b.len() {
        if b[*i] == '"' && b[*i + 1] == ',' {
            let s: String = b[start..*i].iter().collect();
            *i += 1;
            return Some(s);
        }
        *i += 1;
    }
    None
}

/// Schreibt den Menübaum im Original-Format. Texte mit der Folge `",` können nicht dargestellt werden;
/// sie werden durch `&quot;,` ersetzt (15 §3.3).
pub fn format_menu_items(items: &[MenuItem]) -> String {
    fn clean(s: &str) -> String {
        s.replace("\",", "&quot;,")
    }
    fn list(items: &[MenuItem], out: &mut String) {
        if items.is_empty() {
            out.push_str("{}");
            return;
        }
        out.push('{');
        for it in items {
            out.push_str("{\"");
            out.push_str(&clean(&it.text));
            out.push_str("\",\"");
            out.push_str(&clean(&it.url));
            out.push_str("\",\"");
            out.push_str(&clean(&it.target));
            out.push_str("\",");
            list(&it.submenu, out);
            out.push('}');
        }
        out.push('}');
    }
    let mut s = String::new();
    list(items, &mut s);
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_roundtrip() {
        let items = vec![
            MenuItem { text: "Home".into(), url: "index.htm".into(), target: "_self".into(), submenu: vec![] },
            MenuItem {
                text: "Mehr".into(),
                url: "".into(),
                target: "".into(),
                submenu: vec![MenuItem { text: "A".into(), url: "a.htm".into(), target: "".into(), submenu: vec![] }],
            },
        ];
        let s = format_menu_items(&items);
        assert_eq!(s, "{{\"Home\",\"index.htm\",\"_self\",{}}{\"Mehr\",\"\",\"\",{{\"A\",\"a.htm\",\"\",{}}}}}");
        assert_eq!(parse_menu_items(&s), Some(items));
        assert_eq!(parse_menu_items("{}"), Some(vec![]));
        assert_eq!(parse_menu_items("{"), None);
    }
}
