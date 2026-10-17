//! Layout-Tabelle des Eigenschaften-Inspektors (11 §2.3.2, `Original-EXE@00e0e570`): geordnete
//! Liste, erster Treffer gewinnt, Vorlage 0 = Quadrant bleibt unverändert. Die beiden GUID-Zeilen
//! ohne registriertes Werkzeug sind weggelassen (11 §2.3.2 letzter Absatz), ebenso „Importieren“
//! (kein Werkzeug im Bedienfeld).

use fp_engine::command::{ObjectKind, element_is};
use fp_engine::{Session, ToolId};

/// Vorlagen-ID eines Quadranten (eigener Typ, nicht mit Steuerelement-IDs mischen, 11 §6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TemplateId(pub u16);

/// Bedingung einer Layout-Zeile.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pred {
    /// Kein Dokument offen.
    NoDocument,
    /// Aktives Werkzeug.
    Tool(ToolId),
    /// Alle ausgewählten Elemente sind von dieser Art (mindestens eins).
    Sel(ObjectKind),
    /// Mehr als ein Element ausgewählt UND alle von dieser Art.
    Multi(ObjectKind),
    /// Die Pixelauswahl ist das unveränderte Ergebnis von „Motiv auswählen“ und ein Auswahl- bzw.
    /// Zeigerwerkzeug ist aktiv: die KI-Feineinstellungen bleiben auch nach dem Menübefehl
    /// erreichbar (N21).
    AiSubjectResult,
    /// Genau eine Maske gewählt (Maskenbearbeitung, re 31 §5.3 / RT-3 S5): `true` = Vektormaske
    /// (Pfadumriss), `false` = Bitmapmaske.
    MaskSel(bool),
    /// Zustände, die fp-engine noch nicht meldet (Vektormaske/Bitmapmaske/Text bearbeiten,
    /// gemischte Mehrfachauswahl): trifft vorerst nie zu.
    Never,
}

/// Eine Zeile der Tabelle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PiLayout {
    /// Laufende Nummer (Hilfe-Kontext `Nummer + 0x80000`).
    pub number: u16,
    pub pred: Pred,
    /// Q1…Q6 (0 = unverändert).
    pub quads: [u16; 6],
    /// Trennlinien-Maske (9 Bit).
    pub mask: u16,
}

const fn row(number: u16, pred: Pred, quads: [u16; 6], mask: u16) -> PiLayout {
    PiLayout {
        number,
        pred,
        quads,
        mask,
    }
}

use ObjectKind as K;
use Pred::*;

const TOOL_EMPTY: [u16; 6] = [14006, 14080, 14081, 14082, 14083, 14084];
const DOC_Q: [u16; 6] = [14100, 14080, 14031, 6564, 14083, 14084];
const DOC_BOX: [u16; 6] = [14100, 14007, 14031, 6564, 14083, 14084];

/// Die Tabelle in Original-Reihenfolge.
pub const LAYOUTS: &[PiLayout] = &[
    row(
        1,
        NoDocument,
        [14079, 14080, 14081, 14082, 14083, 14084],
        73,
    ),
    row(
        74,
        AiSubjectResult,
        [
            14006,
            14080,
            super::ai_quads::SUBJECT_Q3,
            super::ai_quads::SUBJECT_Q4,
            14083,
            14084,
        ],
        73,
    ),
    row(
        2,
        Tool(ToolId::Brush),
        [14006, 14080, 14050, 14051, 14083, 14084],
        118,
    ),
    row(
        3,
        Tool(ToolId::Eraser),
        [14006, 14080, 14052, 14119, 14083, 14084],
        73,
    ),
    row(
        4,
        Tool(ToolId::Eyedropper),
        [14006, 14080, 14053, 14119, 14083, 14084],
        73,
    ),
    row(
        5,
        Tool(ToolId::Lasso),
        [14006, 14007, 14057, 14119, 14083, 14084],
        73,
    ),
    row(
        6,
        Tool(ToolId::PolygonLasso),
        [14006, 14007, 14057, 14119, 14083, 14084],
        73,
    ),
    row(
        7,
        Tool(ToolId::MagicWand),
        [14006, 14080, 14060, 14119, 14083, 14084],
        73,
    ),
    // KI-Auswahl (Agent KI, eigene Vorlagen in `ai_quads.rs`)
    row(
        73,
        Tool(ToolId::AiObject),
        [
            14006,
            14080,
            super::ai_quads::OBJECT_Q3,
            super::ai_quads::OBJECT_Q4,
            14083,
            14084,
        ],
        73,
    ),
    row(
        74,
        Tool(ToolId::AiSubject),
        [
            14006,
            14080,
            super::ai_quads::SUBJECT_Q3,
            super::ai_quads::SUBJECT_Q4,
            14083,
            14084,
        ],
        73,
    ),
    row(
        75,
        Tool(ToolId::AiRefineEdge),
        [
            14006,
            14080,
            super::ai_quads::REFINE_Q3,
            super::ai_quads::REFINE_Q4,
            14083,
            14084,
        ],
        73,
    ),
    row(
        8,
        Tool(ToolId::Marquee),
        [14006, 14007, 14061, 14119, 14083, 14084],
        73,
    ),
    row(
        9,
        Tool(ToolId::OvalMarquee),
        [14006, 14007, 14061, 14119, 14083, 14084],
        73,
    ),
    row(
        10,
        Tool(ToolId::PaintBucket),
        [14006, 14080, 14107, 14108, 14083, 14084],
        78,
    ),
    row(
        11,
        Tool(ToolId::Gradient),
        [14006, 14080, 14109, 14110, 14083, 14084],
        78,
    ),
    row(
        12,
        Tool(ToolId::RubberStamp),
        [14006, 14080, 14070, 14119, 14083, 14084],
        73,
    ),
    row(
        13,
        Tool(ToolId::Pencil),
        [14006, 14080, 14067, 14119, 14083, 14084],
        73,
    ),
    row(
        14,
        Tool(ToolId::Blur),
        [14006, 14080, 14136, 14119, 14083, 14084],
        73,
    ),
    row(
        15,
        Tool(ToolId::Sharpen),
        [14006, 14080, 14138, 14119, 14083, 14084],
        73,
    ),
    row(
        16,
        Tool(ToolId::Dodge),
        [14006, 14080, 14968, 14119, 14083, 14084],
        73,
    ),
    row(
        17,
        Tool(ToolId::Burn),
        [14006, 14080, 14978, 14119, 14083, 14084],
        73,
    ),
    row(
        18,
        Tool(ToolId::Smudge),
        [14006, 14080, 15006, 14119, 14083, 14084],
        73,
    ),
    row(
        71,
        Tool(ToolId::RedEyeRemoval),
        [14006, 14080, 15082, 14119, 14083, 14084],
        73,
    ),
    row(
        72,
        Tool(ToolId::ReplaceColor),
        [14006, 14080, 15091, 14119, 14083, 14084],
        73,
    ),
    row(
        57,
        Tool(ToolId::Freeform),
        [14006, 14080, 14054, 14082, 14083, 14084],
        73,
    ),
    row(
        64,
        Tool(ToolId::Reshape),
        [14006, 14080, 14055, 14082, 14083, 14084],
        73,
    ),
    row(
        65,
        Tool(ToolId::PathScrubAdd),
        [14006, 14080, 14090, 14082, 14083, 14084],
        73,
    ),
    row(
        66,
        Tool(ToolId::PathScrubSubtract),
        [14006, 14080, 14090, 14082, 14083, 14084],
        73,
    ),
    row(
        19,
        MaskSel(true),
        [14101, 14007, 14140, 14098, 14083, 15105],
        441,
    ),
    row(20, Never, [14102, 14007, 14129, 14130, 14132, 15101], 438),
    row(
        21,
        MaskSel(false),
        [14101, 14007, 14128, 14082, 14083, 14084],
        57,
    ),
    row(
        22,
        Multi(K::Slice),
        [14102, 14007, 14123, 14043, 14083, 14084],
        73,
    ),
    row(
        22,
        Sel(K::Slice),
        [14101, 14007, 14123, 14043, 14083, 14084],
        73,
    ),
    row(
        23,
        Multi(K::Hotspot),
        [14102, 14007, 14093, 14124, 14083, 14084],
        73,
    ),
    row(
        23,
        Sel(K::Hotspot),
        [14101, 14007, 14093, 14124, 14083, 14084],
        73,
    ),
    row(
        58,
        Multi(K::RectPrimitive),
        [14102, 14007, 14129, 14131, 14132, 15180],
        438,
    ),
    row(
        58,
        Sel(K::RectPrimitive),
        [14101, 14007, 14129, 14131, 14132, 15180],
        438,
    ),
    row(24, Never, [14101, 14007, 14147, 14142, 14083, 14084], 73),
    row(
        25,
        Multi(K::Text),
        [14102, 14007, 15010, 14143, 14083, 15104],
        441,
    ),
    row(
        25,
        Sel(K::Text),
        [14101, 14007, 15010, 14143, 14083, 15104],
        441,
    ),
    // Abweichung in der Reihenfolge: `ObjectKind::PathAttrs` von fp-engine schließt Gruppen und
    // zusammengesetzte Formen ein (`IDbPathAttrs` des Originals nicht, Laufzeit 11 T5) – daher
    // stehen deren Zeilen vor den Pfad-Zeilen.
    row(
        67,
        Multi(K::CompoundShape),
        [14102, 14007, 14129, 14130, 14132, 15101],
        438,
    ),
    row(
        26,
        Sel(K::CompoundShape),
        [14101, 14007, 14129, 14130, 14132, 15101],
        438,
    ),
    row(
        67,
        Multi(K::Group),
        [14102, 14007, 14129, 14130, 14083, 15101],
        438,
    ),
    row(
        26,
        Sel(K::Group),
        [14101, 14007, 14129, 14130, 14083, 15101],
        438,
    ),
    row(
        26,
        Multi(K::PathAttrs),
        [14102, 14007, 14129, 14130, 14132, 15101],
        438,
    ),
    row(
        26,
        Sel(K::PathAttrs),
        [14101, 14007, 14129, 14130, 14132, 15101],
        438,
    ),
    row(
        27,
        Multi(K::Bitmap),
        [
            14102,
            14007,
            14037,
            15008,
            crate::dialogs::trace::INSPECTOR_Q5,
            15103,
        ],
        432,
    ),
    row(
        27,
        Sel(K::Bitmap),
        [
            14101,
            14007,
            14037,
            15008,
            crate::dialogs::trace::INSPECTOR_Q5,
            15103,
        ],
        432,
    ),
    row(
        68,
        Multi(K::GraphicInstance),
        [14102, 14007, 14037, 15009, 14083, 15102],
        441,
    ),
    row(
        68,
        Sel(K::GraphicInstance),
        [14101, 14007, 14037, 15009, 14083, 15102],
        441,
    ),
    row(
        28,
        Multi(K::ButtonInstance),
        [14102, 14007, 14095, 14096, 14083, 15106],
        457,
    ),
    row(
        28,
        Sel(K::ButtonInstance),
        [14101, 14007, 14095, 14096, 14083, 15106],
        457,
    ),
    row(
        29,
        Multi(K::AnimationInstance),
        [14102, 14007, 14045, 15009, 14083, 15102],
        441,
    ),
    row(
        29,
        Sel(K::AnimationInstance),
        [14101, 14007, 14045, 15009, 14083, 15102],
        441,
    ),
    row(
        69,
        Multi(K::Instance),
        [14102, 14007, 14037, 15009, 14083, 15102],
        441,
    ),
    row(
        69,
        Sel(K::Instance),
        [14101, 14007, 14037, 15009, 14083, 15102],
        441,
    ),
    row(30, Tool(ToolId::Pointer), DOC_Q, 73),
    row(31, Tool(ToolId::Subselection), DOC_Q, 73),
    row(32, Tool(ToolId::SelectBehind), DOC_Q, 73),
    row(33, Tool(ToolId::Hand), DOC_Q, 73),
    row(34, Tool(ToolId::Zoom), DOC_Q, 73),
    row(35, Tool(ToolId::Crop), DOC_BOX, 73),
    row(36, Tool(ToolId::Distort), DOC_BOX, 73),
    row(37, Tool(ToolId::ExportArea), DOC_BOX, 73),
    row(39, Tool(ToolId::Scale), [0; 6], 73),
    row(40, Tool(ToolId::Skew), [0; 6], 73),
    row(
        43,
        Tool(ToolId::Pen),
        [14006, 14080, 14068, 14065, 14132, 14084],
        54,
    ),
    row(
        44,
        Tool(ToolId::Line),
        [14006, 14080, 14058, 14059, 14083, 14084],
        54,
    ),
    row(
        45,
        Tool(ToolId::Rectangle),
        [14006, 14007, 14068, 14069, 14132, 14084],
        54,
    ),
    row(
        70,
        Tool(ToolId::AutoShape),
        [14006, 14007, 14068, 14069, 14083, 14084],
        54,
    ),
    row(
        47,
        Tool(ToolId::Polygon),
        [14006, 14080, 14111, 14113, 14132, 14084],
        54,
    ),
    row(
        48,
        Tool(ToolId::Ellipse),
        [14006, 14007, 14068, 14065, 14132, 14084],
        54,
    ),
    row(
        49,
        Tool(ToolId::HotspotRect),
        [14006, 14080, 14056, 14082, 14083, 14084],
        73,
    ),
    row(
        50,
        Tool(ToolId::HotspotPolygon),
        [14006, 14080, 14056, 14082, 14083, 14084],
        73,
    ),
    row(
        51,
        Tool(ToolId::HotspotCircle),
        [14006, 14080, 14056, 14082, 14083, 14084],
        73,
    ),
    row(52, Tool(ToolId::Slice), TOOL_EMPTY, 73),
    row(53, Tool(ToolId::PolygonSlice), TOOL_EMPTY, 73),
    row(
        54,
        Tool(ToolId::Text),
        [14006, 14080, 14009, 14143, 14083, 14084],
        57,
    ),
    row(
        55,
        Tool(ToolId::VectorPath),
        [14006, 14080, 14099, 14106, 14132, 14084],
        54,
    ),
    row(
        56,
        Tool(ToolId::RedrawPath),
        [14006, 14080, 14099, 14106, 14132, 14084],
        54,
    ),
];

/// Erfüllt die Sitzung die Bedingung?
pub fn matches(pred: &Pred, s: &Session) -> bool {
    let d = s.active();
    match pred {
        NoDocument => d.is_none(),
        Tool(t) => d.is_some() && s.tools.active == *t,
        Never => false,
        MaskSel(vector) => d.is_some_and(|d| {
            d.selection.elements.len() == 1
                && fp_engine::docops::mask_owner(&d.doc, d.page, d.selection.elements[0]).is_some_and(|o| matches!(&o.kind, fp_model::ElementKind::Group(g) if (g.kind == fp_model::GroupKind::VectorMask) == *vector))
        }),
        AiSubjectResult => d.is_some_and(|d| d.selection.has_pixels()) && TUNE_TOOLS.contains(&s.tools.active) && subject_result(s),
        Sel(k) | Multi(k) => {
            let Some(d) = d else { return false };
            let ids = &d.selection.elements;
            if ids.is_empty() || (matches!(pred, Multi(_)) && ids.len() < 2) {
                return false;
            }
            // erst das erste Objekt (schnelle Absage), dann alle in einem Durchlauf statt einer
            // Suche je ID (PERF-1: Tausende gewählte Objekte)
            if !fp_engine::docops::element_on_page(&d.doc, d.page, ids[0]).is_some_and(|e| element_is(insp_elem(&e), *k, &d.doc)) {
                return false;
            }
            fp_engine::docops::find_many_on_page(&d.doc, d.page, ids).iter().all(|p| p.as_ref().and_then(|p| d.doc.element_at(p)).is_some_and(|e| element_is(insp_elem(e), *k, &d.doc)))
        }
    }
}

/// Werkzeuge, bei denen eine Motivauswahl die KI-Feineinstellungen zeigt (N21).
const TUNE_TOOLS: [ToolId; 7] = [
    ToolId::Pointer,
    ToolId::Subselection,
    ToolId::Marquee,
    ToolId::OvalMarquee,
    ToolId::Lasso,
    ToolId::PolygonLasso,
    ToolId::MagicWand,
];

thread_local! {
    /// Letzte Prüfung (Kennung der Maske → Ergebnis); die volle Prüfsumme der Maske nur bei Änderung.
    static SUBJECT_TUNE: std::cell::Cell<Option<((usize, i32, i32, i32, i32), bool)>> = const { std::cell::Cell::new(None) };
}

/// Ist die Pixelauswahl das unveränderte Ergebnis der letzten Motivauswahl?
fn subject_result(s: &Session) -> bool {
    let Some(m) = s.active().and_then(|d| d.selection.pixels.as_ref()) else {
        return false;
    };
    let key = crate::canvas::overlay::mask_key(m);
    if let Some((k, v)) = SUBJECT_TUNE.with(|c| c.get())
        && k == key
    {
        return v;
    }
    let v = fp_engine::commands::ai::can_retune(s, "select.aiSubject");
    SUBJECT_TUNE.with(|c| c.set(Some((key, v))));
    v
}

/// Element, nach dessen Art der Inspektor gewählt wird: Ein maskiertes Einzelelement (Maske ▸ Alles
/// einblenden/ausblenden auf einem Objekt, fixg5 Q-02) zeigt wie im Original den Inspektor seines
/// Inhalts (Bild mit Maske), nicht den der Gruppe.
pub fn insp_elem(e: &fp_model::Element) -> &fp_model::Element {
    e.masked_single().map(|(c, _)| &**c).unwrap_or(e)
}

/// Erste passende Zeile.
pub fn select_layout(s: &Session) -> Option<&'static PiLayout> {
    LAYOUTS.iter().find(|l| matches(&l.pred, s))
}

/// Klassenname einer Vorlage (für Platzhalter und Phase 4).
pub fn template_name(id: u16) -> &'static str {
    match id {
        14050 => "CBrushToolQuad3",
        14051 => "CBrushToolQuad4",
        14052 => "CEraserToolQuad3",
        14053 => "CEyedropToolQuad3",
        14054 => "CFreeformToolQuad3",
        14055 => "CReshapeToolQuad3",
        14056 => "CURLToolQuad3",
        14057 => "CLassoToolQuad3",
        14058 => "CLineToolQuad3",
        14059 => "CLineToolQuad4",
        14060 => "CWandToolQuad3",
        14061 => "CMarqueeToolQuad3",
        14065 => "CPenToolQuad4",
        14067 => "CPencilToolQuad3",
        14068 => "CRectToolQuad3",
        14069 => "CRectToolQuad4",
        14070 => "CStampToolQuad3",
        14090 => "CScrubToolQuad3",
        14099 => "CPathToolQuad3",
        14106 => "CPathToolQuad4",
        14107 => "CFillToolQuad3",
        14108 => "CFillToolQuad4",
        14109 => "CGradientToolQuad3",
        14110 => "CGradientToolQuad4",
        14111 => "CPolyToolQuad3",
        14113 => "CPolyToolQuad4",
        14119 => "Masken-Teilfeld",
        14136 => "CBlurToolQuad3",
        14138 => "CSharpenToolQuad3",
        14968 => "CDodgeToolQuad3",
        14978 => "CBurnToolQuad3",
        15006 => "CSmudgeToolQuad3",
        15082 => "CRedEyeRemovalToolQuad3",
        15091 => "CReplaceColorToolQuad3",
        14009 => "CTextToolQuad3",
        14143 => "CTextToolQuad4",
        14129 => "CPathQuad3",
        14130 => "CPathQuad4",
        14131 => "CRectQuad4",
        14132 => "CRectQuad5",
        15101 => "CPathQuad6",
        15180 => "CRectQuad6",
        14037 => "CBitmapQuad3",
        15008 => "CBitmapQuad4",
        15103 => "CBitmapQuad6",
        15010 => "CPITextObjectQuad3",
        15104 => "CTextToolQuad6",
        14123 => "CPISliceQuad3",
        14043 => "CPISliceQuad4",
        14093 => "CPIHotspotQuad3",
        14124 => "CPIHotspotQuad4",
        14095 => "CButtonQuad3",
        14096 => "CButtonQuad4",
        15106 => "CButtonQuad6",
        14045 => "CPIAnimationQuad3",
        15009 => "CPIAnimationQuad4",
        15102 => "CPIAnimationQuad6",
        14031 => "CDocumentQuad3",
        6564 => "CDocumentQuad4",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_layout_for_pointer_without_selection() {
        let mut s = Session::default();
        assert_eq!(select_layout(&s).map(|l| l.number), Some(1));
        let _ = s.execute("file.new", serde_json::json!({"width": 100, "height": 100}));
        assert_eq!(select_layout(&s).map(|l| l.number), Some(30));
        let _ = s.set_tool(ToolId::Brush);
        assert_eq!(select_layout(&s).map(|l| l.number), Some(2));
        let _ = s.set_tool(ToolId::Rectangle);
        assert_eq!(select_layout(&s).map(|l| l.number), Some(45));
    }

    #[test]
    fn rectangle_selection_selects_rect_layout() {
        let mut s = Session::default();
        let _ = s.execute("file.new", serde_json::json!({"width": 100, "height": 100}));
        let _ = s.execute(
            "draw.rectangle",
            serde_json::json!({"rect": [10, 10, 60, 40]}),
        );
        let l = select_layout(&s).map(|l| l.number);
        // ein neu gezeichnetes Rechteck ist ausgewählt (Rechteck-Grundform bzw. Pfad)
        assert!(matches!(l, Some(58) | Some(26) | Some(30)), "{l:?}");
    }

    /// FIX-R1 (fixg5 Q-02): nach Maske ▸ Alles einblenden/ausblenden zeigt der Inspektor das Bild
    /// (Bitmap-Inspektor 27, Typ „Bitmap“) wie im Original, nicht die Gruppe (26).
    #[test]
    fn masked_bitmap_shows_bitmap_inspector() {
        for cmd in ["modify.mask.revealAll", "modify.mask.hideAll"] {
            let mut s = Session::default();
            let _ = s.execute("file.new", serde_json::json!({"width": 200, "height": 200}));
            let img = fp_raster::RgbaImage::filled(
                fp_geom::IRect::from_xywh(10, 10, 50, 40),
                fp_color::Rgba8::new(200, 30, 30, 255),
            )
            .ok();
            s.clipboard.clear();
            s.clipboard.pixels = img;
            assert!(s.execute("edit.paste", serde_json::json!({})).is_ok());
            assert_eq!(select_layout(&s).map(|l| l.number), Some(27));
            assert!(s.execute(cmd, serde_json::json!({})).is_ok(), "{cmd}");
            assert_eq!(select_layout(&s).map(|l| l.number), Some(27), "{cmd}");
            let d = s.active().map(|d| {
                (
                    std::sync::Arc::clone(&d.doc),
                    d.page,
                    d.selection.elements[0],
                )
            });
            let (doc, page, id) = d.unwrap_or_else(|| panic!("Dokument"));
            let e = fp_engine::docops::element_on_page(&doc, page, id)
                .unwrap_or_else(|| panic!("Element"));
            assert!(e.masked_single().is_some());
            assert_eq!(
                super::super::obj_quads::type_label(&e),
                fp_i18n::t("pi-type-bitmap"),
                "{cmd}"
            );
        }
    }
}
