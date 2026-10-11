//! Text (`TXTv`, Spezifikation 09) und die Rohdaten der Text-Engine (ATE) (`aTXT`, 02 §12.7).

use crate::attrs::{FillHandles, PathAttrs};
use crate::element::Element;
use crate::raw::Extra;
use crate::serde_ext;
use fp_geom::{Matrix3, Point, Rect};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Zeichen- und Absatzattribute (Anfangsattribute im `TXTv`-Kopf bzw. Attributblock eines Laufs).
/// Alle Felder optional: In Läufen erben fehlende Werte den vorherigen Wert (09 §3.3).
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct TextAttrs {
    /// Schrift (`FONs`): im Kopf Anzeigename, im Lauf PostScript-Name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    /// PostScript-Name der Anfangsschrift (`CFTs`, nur Kopf und Stile).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ps_font: Option<String>,
    /// Schriftgröße in Pixeln (`PTSf`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<f32>,
    /// Faux-Fett (`BOLb`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    /// Faux-Kursiv (`ITLb`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    /// Unterstrichen (`UNDb`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underline: Option<bool>,
    /// Horizontale Skalierung, 1 = 100 % (`HSCf`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub horizontal_scale: Option<f32>,
    /// Manuelles Paar-Kerning (`KRNf`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kerning: Option<f32>,
    /// Laufweite (`RKNf`): zusätzlicher Abstand `RKNf·PTSf/10` px je Glyphe.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tracking: Option<f32>,
    /// Grundlinienversatz in px, positiv hebt an (`BLSf`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline_shift: Option<f32>,
    /// Zeilenabstand (`LEDf`): Faktor bei Modus 0, px bei Modus 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leading: Option<f32>,
    /// Zeilenabstandsmodus (`LDMi`): 0 Prozent, 1 exakt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leading_mode: Option<i32>,
    /// Ausrichtung (`JSTi`): 0 links, 1 zentriert, 2 rechts, 3 Blocksatz, 4 erzwungen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alignment: Option<i32>,
    /// Textfarbe ARGB (`FCLi`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<u32>,
    /// Erstzeileneinzug px (`PINf`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indent: Option<f32>,
    /// Abstand vor/nach dem Absatz px (`PSBf`, `PSAf`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space_before: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space_after: Option<f32>,
}

impl TextAttrs {
    /// Überträgt alle gesetzten Werte aus `other` (Vererbung in Läufen).
    pub fn apply(&mut self, other: &TextAttrs) {
        macro_rules! take {
            ($($f:ident),*) => { $( if other.$f.is_some() { self.$f = other.$f.clone(); } )* };
        }
        take!(font, ps_font, size, bold, italic, underline, horizontal_scale, kerning, tracking, baseline_shift,
              leading, leading_mode, alignment, color, indent, space_before, space_after);
    }
}

/// Ein Lauf (`[Attributblock] TRNs`). `TRNs`-Sonderzeichen: `\r` Absatzende, `\x03` weicher Umbruch,
/// `\t` Tabulator.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextRun {
    /// Attributblock vor dem Text (nur die gesetzten Werte).
    pub attrs: TextAttrs,
    /// Text (`TRNs`).
    pub text: String,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

/// Text auf/in Pfad (`TAPv{TAMi CHOf PTHv}`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextOnPath {
    /// Modus (`TAMi`): 0 rotate, 1 vertical, 2 skew vertical, 3 skew horizontal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<i32>,
    /// Versatz entlang des Pfads px (`CHOf`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<f32>,
    /// Pfad (`PTHv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<Arc<Element>>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

/// Textobjekt (`TXTv`, Tags in Schreibreihenfolge 09 §3.2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextElem {
    /// Layout-Rahmen (`LFTf TOPf RITf BOTf`, Lesevorgabe 0/0/250/250).
    #[serde(with = "serde_ext::rect")]
    pub bounds: Rect,
    /// Benutzerrahmen/Anker (`LFAf TOAf RIAf BOAf`): Punkttext → (Anker x, erste Grundlinie, 0, 0).
    #[serde(default, skip_serializing_if = "Option::is_none", with = "serde_ext::opt_rect")]
    pub anchor_rect: Option<Rect>,
    /// Objektart (`TFTi`): 0 Punkttext, 1 fester Rahmen, 2 Text auf Pfad, 3 Text in Pfad.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_type: Option<i32>,
    /// 1-basierter Index der ATE-Story in `aTXT` (`AIDi`, nur mit `FOAb = 1`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub story_index: Option<u32>,
    /// Inhalt beim Laden aus der Story übernehmen (`FOAb`). Bei `true` gewinnt die Story gegen `TFSv`
    /// (B, Laufzeit 02 T6). Nach einer Textänderung ohne neue Story auf `false` setzen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_story: Option<bool>,
    /// Füllung/Strich (`PATv`).
    pub attrs: Arc<PathAttrs>,
    /// Strichposition (`BRPi`).
    pub stroke_placement: i32,
    /// `PTTi`.
    pub ptt: i32,
    /// Füllung über Strich (`FOTb`).
    pub fill_on_top: bool,
    /// Zufalls-Startwert (`RNDi`).
    pub random_seed: u32,
    /// Texturversatz (`TOXf TOYf`).
    #[serde(with = "serde_ext::point")]
    pub texture_offset: Point,
    /// Orientierung (`ORIi`): 0 horizontal, 1 vertikal r→l, 2 horizontal r→l, 3 vertikal l→r.
    pub orientation: i32,
    /// Füll-Handles (`PCFb PS* PE* PF*`).
    pub fill_handles: FillHandles,
    /// Transformation (`MTXv`).
    #[serde(default, skip_serializing_if = "Option::is_none", with = "serde_ext::opt_matrix")]
    pub transform: Option<Matrix3>,
    /// Transformationsmodus (`XFMi`): 0 paths, 1 pixels (Lesevorgabe 1).
    pub transform_mode: i32,
    /// Kantenglättung (`TAAi`): 0 glatt, 1 scharf, 2 stark, 3 System (→ 1), 4 benutzerdefiniert.
    pub antialias: i32,
    /// Benutzerdefinierte Glättung: Schärfe (`TA1i`, Vorgabe 192), Stärke (`TA2i`, 64), Überabtastung (`TOSi`, 8).
    pub aa_sharpness: i32,
    pub aa_strength: i32,
    pub aa_oversample: i32,
    /// ATE-Text (`ATEb`; fehlt in Dateien vor CS3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ate: Option<bool>,
    /// Automatisches Kerning (`ATKb`, Vorgabe 1).
    pub auto_kern: bool,
    /// Anfangsattribute (Kopf).
    pub initial: TextAttrs,
    /// Läufe (`TFSv`).
    pub runs: Vec<TextRun>,
    /// Attributblock nach dem letzten `TRNs` (selten).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trailing_attrs: Option<TextRun>,
    /// Ist `TFSv` vorhanden?
    pub has_runs_node: bool,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub runs_extra: Extra,
    /// Text auf/in Pfad (`TAPv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_path: Option<TextOnPath>,
}

impl TextElem {
    /// Gesamter Text (Verkettung aller Läufe).
    pub fn plain_text(&self) -> String {
        self.runs.iter().map(|r| r.text.as_str()).collect()
    }

    /// Löst den Text von seiner ATE-Story (`FOAb = 0`, kein `AIDi`). Nach einer Textänderung nötig,
    /// solange keine neue Story mit gesetztem Layout erzeugt wird: Das Original lädt den Text dann aus `TFSv`
    /// (02 §12.7, 09 §4.1). Liefert den bisherigen Story-Index.
    pub fn detach_story(&mut self) -> Option<u32> {
        let old = self.story_index.take();
        if self.from_story.is_some() || old.is_some() {
            self.from_story = Some(false);
        }
        old
    }

    /// Läufe mit aufgelösten Attributen (Vererbung über die Läufe, Start mit den Anfangsattributen).
    pub fn resolved_runs(&self) -> Vec<(TextAttrs, &str)> {
        let mut cur = self.initial.clone();
        let mut out = Vec::with_capacity(self.runs.len());
        for r in &self.runs {
            cur.apply(&r.attrs);
            out.push((cur.clone(), r.text.as_str()));
        }
        out
    }
}

/// Wert der COS-ähnlichen ATE-Syntax (02 §12.7).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CosValue {
    /// `<< /Name Wert … >>`: geordnete Schlüssel/Wert-Paare.
    Dict(CosDict),
    /// `[ Wert … ]`.
    Array(Vec<CosValue>),
    /// `/Name` (ohne Schrägstrich).
    Name(String),
    /// Zahl mit Originaltext (z. B. `.5`, `12.0`).
    Number(String),
    /// `( … )`: Rohbytes ohne Klammern, Escapes aufgelöst. Text meist UTF-16BE mit BOM.
    String(Vec<u8>),
    /// `true`/`false`.
    Bool(bool),
}

/// Geordnetes COS-Wörterbuch.
pub type CosDict = Vec<(String, CosValue)>;

impl CosValue {
    /// Wörterbuch-Zugriff.
    pub fn get(&self, key: &str) -> Option<&CosValue> {
        match self {
            CosValue::Dict(d) => d.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    /// Array-Zugriff.
    pub fn index(&self, i: usize) -> Option<&CosValue> {
        match self {
            CosValue::Array(a) => a.get(i),
            _ => None,
        }
    }
    pub fn as_array(&self) -> Option<&[CosValue]> {
        match self {
            CosValue::Array(a) => Some(a),
            _ => None,
        }
    }
    pub fn as_dict(&self) -> Option<&CosDict> {
        match self {
            CosValue::Dict(d) => Some(d),
            _ => None,
        }
    }
    /// Zahl als `f64`.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            CosValue::Number(t) => t.parse().ok(),
            _ => None,
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            CosValue::Bool(b) => Some(*b),
            _ => None,
        }
    }
    pub fn as_name(&self) -> Option<&str> {
        match self {
            CosValue::Name(n) => Some(n),
            _ => None,
        }
    }
    /// String als Text: UTF-16BE mit BOM `FE FF`, sonst Latin-1.
    pub fn as_text(&self) -> Option<String> {
        match self {
            CosValue::String(b) => Some(decode_cos_text(b)),
            _ => None,
        }
    }
}

/// Dekodiert einen COS-String (UTF-16BE mit BOM, sonst Latin-1).
pub fn decode_cos_text(b: &[u8]) -> String {
    if b.len() >= 2 && b[0] == 0xFE && b[1] == 0xFF {
        let units: Vec<u16> = b[2..].as_chunks::<2>().0.iter().map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
        String::from_utf16_lossy(&units)
    } else {
        b.iter().map(|&c| c as char).collect()
    }
}

/// Kodiert Text als COS-String (UTF-16BE mit BOM).
pub fn encode_cos_text(s: &str) -> Vec<u8> {
    let mut out = vec![0xFE, 0xFF];
    for u in s.encode_utf16() {
        out.extend_from_slice(&u.to_be_bytes());
    }
    out
}

/// Inhalt des `aTXT`-Chunks: Rohbaum (verlustfrei) und die dekodierten Stories.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AteData {
    /// Oberste Ebene (Folge von Schlüssel/Wert-Paaren). Maßgeblich beim Schreiben.
    pub root: CosDict,
    /// Unveränderte Chunk-Bytes, falls der Chunk nicht geparst werden konnte (dann ist `root` leer).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unparsed: Option<Vec<u8>>,
    /// Stream-Version (`/98/0`: 4 bei CS3/CS4, 6 bei CS5/CS6).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<i64>,
    /// Schriften der Ressourcen (`/0/1/0[]`, PostScript-Namen).
    pub fonts: Vec<String>,
    /// Dekodierte Stories, Index = `AIDi − 1`. Nur Sicht, wird beim Schreiben nicht benutzt.
    pub stories: Vec<AteStory>,
}

/// Dekodierte ATE-Story (`/1/1[i]`).
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct AteStory {
    /// Text (`/0/0`, endet mit `\r`).
    pub text: String,
    /// Absatzläufe: Länge in UTF-16-Einheiten und aufgelöste Absatzattribute.
    pub paragraph_runs: Vec<(u32, AteParagraphAttrs)>,
    /// Stil-Läufe: Länge und aufgelöste Zeichenattribute.
    pub style_runs: Vec<(u32, AteCharAttrs)>,
    /// Hat die Story eine gespeicherte Ansicht (`/1`, gesetztes Layout)? Ohne sie öffnet das Original die
    /// Datei nicht (B, Laufzeit 02 T6).
    pub has_view: bool,
    /// Glättung der Story (`/0/10/0`, StorySheet AntiAlias).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub antialias: Option<i64>,
}

/// Zeichenattribute einer ATE-Story (StyleSheetData, Schlüssel 02 §12.7). Fehlende Werte: weder im Lauf,
/// noch im Elternstil, noch in den Dokumentvorgaben gefunden.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct AteCharAttrs {
    /// 0 Font: PostScript-Name (aufgelöst über die Schriftliste).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    /// 1 FontSize.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f64>,
    /// 2 FauxBold, 3 FauxItalic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faux_bold: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faux_italic: Option<bool>,
    /// 4 AutoLeading, 5 Leading.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_leading: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leading: Option<f64>,
    /// 6 HorizontalScale, 7 VerticalScale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub horizontal_scale: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_scale: Option<f64>,
    /// 8 Tracking (1/1000 em; = `RKNf·100`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tracking: Option<f64>,
    /// 9 BaselineShift.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline_shift: Option<f64>,
    /// 11 AutoKern: 0 aus, 1 Metrik, 2 optisch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_kern: Option<i64>,
    /// 12 FontCaps, 13 FontBaseline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_caps: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_baseline: Option<i64>,
    /// 15 StrikethroughPosition, 16 UnderlinePosition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strikethrough: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underline: Option<i64>,
    /// 18 Ligatures.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ligatures: Option<bool>,
    /// 38 Language.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<i64>,
    /// 53 FillColor (SimplePaint: Typ und Werte, z. B. Typ 1 = [A, R, G, B] in 0..1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_color: Option<Vec<f64>>,
    /// 54 StrokeColor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_color: Option<Vec<f64>>,
    /// 56 FillFlag, 57 StrokeFlag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_flag: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_flag: Option<bool>,
}

/// Absatzattribute einer ATE-Story (ParagraphProperties, 02 §12.7).
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct AteParagraphAttrs {
    /// 0 Justification.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub justification: Option<i64>,
    /// 1 FirstLineIndent, 2 StartIndent, 3 EndIndent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_line_indent: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_indent: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_indent: Option<f64>,
    /// 4 SpaceBefore, 5 SpaceAfter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space_before: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space_after: Option<f64>,
    /// 7 AutoLeading (Faktor), 8 LeadingType.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_leading: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leading_type: Option<i64>,
    /// 9 AutoHyphenate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_hyphenate: Option<bool>,
    /// 31 DefaultTabWidth.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_tab_width: Option<f64>,
    /// 30 TabStops: (Position, Art) je Tabulator, soweit dekodierbar.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tab_stops: Vec<(f64, i64)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cos_text_codec() {
        let b = encode_cos_text("Button\r");
        assert_eq!(&b[..2], &[0xFE, 0xFF]);
        assert_eq!(decode_cos_text(&b), "Button\r");
        assert_eq!(decode_cos_text(b"Abc"), "Abc");
    }

    #[test]
    fn attrs_inherit() {
        let mut a = TextAttrs { size: Some(12.0), bold: Some(false), ..Default::default() };
        a.apply(&TextAttrs { bold: Some(true), ..Default::default() });
        assert_eq!(a.size, Some(12.0));
        assert_eq!(a.bold, Some(true));
    }
}
