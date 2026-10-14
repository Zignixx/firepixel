//! Umfang der JSF-Umsetzung: welche Member der Bindungstabelle umgesetzt sind und welche bewusst
//! nicht (mit Begründung).
//!
//! `"*"` = alle Member der Klasse. Member der Formen-Laufzeit (`SmartShape`, `RegisterMoveParms`,
//! `ControlPoint`, `ContourNode.Register*Move`) sind in [`crate::autoshape`] umgesetzt und wirken
//! während eines Steuerpunkt-Ziehens; die Datenobjekte (`Brush`, `Fill`, `Style` …) liefern alle
//! Felder aus dem Dokumentmodell.

use crate::bindings::{CLASSES, COUNTS};
use std::fmt::Write as _;

/// Umgesetzte Methoden je Klasse (Rest wirft „nicht implementiert“).
pub const METHODS: &[(&str, &[&str])] = &[
    (
        "Firepixel",
        &[
            "getDocumentDOM", "createFirepixelDocument", "createDocument", "openDocument", "openMultiDocument", "saveDocument", "saveDocumentAs", "saveDocumentAsync",
            "saveDocumentAsAsync", "saveDocumentCopyAs", "saveDocumentAsTemplate", "saveDocumentAsTemplateAsync", "saveAll", "saveAllAsync", "saveAs", "closeDocument",
            "revertDocument", "setActiveWindow", "moveFocusToDoc", "findOpenDocument", "getDocumentPath", "getPref", "setPref", "undo", "redo", "clipCopy", "clipCut",
            "clipPaste", "deleteSelection", "runScript", "yesNoDialog", "browseForFileURL", "browseForFolderURL", "browseDocument", "locateDocDialog",
            "chooseScriptTargetDialog", "popupColorPickerOverMouse", "popupColorPicker", "getRed", "getGreen", "getBlue", "makeRGBColor", "shiftKeyDown",
            "ctrlCmdKeyDown", "altOptKeyDown", "isPIExpanded", "getHideAllFloaters", "takeScreenshot", "associatePngwithFw", "isPIVisible", "getFloaterVisibility",
            "checkFwJsVersion", "setFloaterVisibility", "toggleFloater", "setHideAllFloaters", "growPIWindow", "shrinkPIWindow", "setPIPosition", "showPIWindow",
            "hidePIWindow", "reloadPatterns", "reloadTextures", "reloadCommonLibrary", "logPipEvent", "enableFlashDebugging", "disableFlashDebugging", "quit",
            "quitApplication", "resetWarnings", "addProjectEntry", "logChangeInProject", "removeAllProjectEntries", "setActiveViewScale", "getProjectEntries",
            "getPIPosition", "getFWLaunchLanguage", "getRootDirectory", "internalNameToUIName", "writePanelStateToFile", "readPanelStateFromFile", "exportDocumentAs",
            "getPlatformNameForPSFont", "getFamilyNameForPSFont", "getStyleNameForPSFont", "throwInternalError", "createDocumentWithDialog", "findNext", "findApp",
            "launchApp", "chooseBrowser", "launchBrowserTo", "exportPages", "exportCSSSpriteForJQuery", "exportHtmlAndImages", "exportCSSLayers", "exportFrames",
            "exportLayers", "exportPSD", "exportPDF", "exportAndCopyHTMLCode", "getHTMLFileForScript", "getNumberOfTables", "newDocumentFromTemplate", "browseHelp",
            "readNthTable", "replace", "replaceAll", "saveJsCommand", "setUpFindReplace", "updateHTML", "doBatchProcess", "eval", "addSwatches", "setActiveColor",
            "newMobileDocument", "getRotationMatrix", "getScaleMatrix",
        ],
    ),
    ("HistoryPalette", &["*"]),
    ("$", &["getenv", "sleep", "evalFile", "gc"]),
    (
        "Files",
        &[
            "exists", "isDirectory", "createDirectory", "createFile", "deleteFile", "deleteFileIfExisting", "copy", "rename", "swap", "getExtension", "getFilename",
            "setFilename", "getDirectory", "makePathFromDirAndFile", "getTempFilePath", "enumFiles", "getLanguageDirectory", "getLastErrorString", "open",
        ],
    ),
    ("FileReference", &["write", "writeUTF8", "readline", "readlineUTF8", "close"]),
    ("Document", &["*"]),
    ("DocumentStatic", &["*"]),
    ("fw.dialogs", &["*"]),
    ("Find", &["*"]),
    ("SetCurrentPage", &["*"]),
    ("Widget", &["*"]),
    ("Group", &["*"]),
    ("Element", &["*"]),
    ("ContourNode", &["*"]),
    ("ControlPoint", &["*"]),
    ("SmartShape", &["*"]),
    ("PathAttrs", &["*"]),
    ("ExportOptions", &["*"]),
    ("SliceHotspot", &["*"]),
];

/// Umgesetzte Properties je Klasse (lesend; schreibbare, soweit sinnvoll).
pub const PROPS: &[(&str, &[&str])] = &[
    ("Firepixel", &["*"]),
    ("Document", &["*"]),
    (
        "Element",
        &["left", "top", "width", "height", "pixelRect", "visible", "locked", "opacity", "blendMode", "name", "isLayer", "isSmartShape", "effectList", "customData", "mask"],
    ),
    ("Path", &["pathAttributes", "contours", "isEvenOddFill", "pathOperation", "randSeed", "textureOffset"]),
    ("RectanglePrimitive", &["roundness", "originalSides", "mode", "pathAttributes", "transform", "pathOperation"]),
    (
        "Text",
        &[
            "pathAttributes", "transform", "randSeed", "textureOffset", "textRuns", "textChars", "transformMode", "orientation", "autoKern", "antiAliased",
            "antiAliasMode", "rawLeft", "rawTop", "rawWidth", "rawHeight", "autoExpand", "italic", "underline", "bold", "justify", "alignment", "font", "fontsize",
        ],
    ),
    ("Group", &["elements", "groupType", "controlPoints", "smartShapeCode", "transformMode"]),
    ("Instance", &["symbolID", "symbolName", "transformMode", "transform", "instanceType", "altText", "urlText", "targetText"]),
    ("Hotspot", &["*"]),
    ("SliceHotspot", &["*"]),
    ("CompoundShape", &["resultantPath"]),
    ("Layer", &["isLayer", "frames", "layerType", "name", "sharing", "disclosure", "elems"]),
    ("Frame", &["topLayers", "layers", "visible", "delay", "disposal", "name"]),
    ("FrameNLayerIntersection", &["elemsandsublayers", "elements", "visible", "locked", "name"]),
    ("Contour", &["nodes", "isClosed"]),
    ("ContourNode", &["predX", "predY", "x", "y", "succX", "succY", "isCurvePoint", "isSelectedPoint", "randomSeed", "dynamicInfo", "name"]),
    ("ContourNodeDynamicInfo", &["*"]),
    ("Find", &["*"]),
    ("SetCurrentPage", &["*"]),
    ("Widget", &["*"]),
    ("SmartShape", &["*"]),
    ("RegisterMoveParms", &["*"]),
    ("ControlPoint", &["*"]),
    ("Brush", &["*"]),
    ("Fill", &["*"]),
    ("Gradient", &["*"]),
    ("GradientNode", &["*"]),
    ("Pattern", &["*"]),
    ("Texture", &["*"]),
    ("EffectList", &["*"]),
    ("PathAttrs", &["*"]),
    ("Style", &["*"]),
    ("Guides", &["*"]),
    ("ElementMask", &["*"]),
    ("Behavior", &["*"]),
    ("FwArray", &["*"]),
    ("Errors", &["*"]),
    ("Tools", &["*"]),
    ("System", &["*"]),
    ("$", &["version", "build", "buildDate", "locale", "os", "fileName"]),
    ("(DomObject-Basis)", &["javascriptString"]),
];

/// Bewusst nicht umgesetzte Member mit Begründung (Klasse, Member oder `*`, Grund).
pub const EXCEPTIONS: &[(&str, &str, &str)] = &[
    ("Firepixel", "exportSWF", "SWF-Export – Format veraltet, kein Exporter in Firepixel"),
    ("Firepixel", "exportFXG", "FXG-Export (Flex) – Format veraltet, kein Exporter in Firepixel"),
    ("Firepixel", "exportMXMLAndImages", "MXML-Export (Flex) – Format veraltet, kein Exporter in Firepixel"),
    ("Firepixel", "exportDirectorAsSlices", "DIR-Export (Multimedia) – Format veraltet, kein Exporter in Firepixel"),
    ("Firepixel", "exportDirectorAsLayers", "DIR-Export (Multimedia) – Format veraltet, kein Exporter in Firepixel"),
    ("Firepixel", "exportIllustrator", "Export als AI 8 – kein Exporter in Firepixel (SVG/PDF über exportPDF bzw. file.export)"),
    ("Style", "clipCopySmallPreviewImage", "Stil-Vorschaubild in die Zwischenablage – Firepixel erzeugt keine Stil-Vorschaubilder"),
    ("Style", "clipCopyLargePreviewImage", "Stil-Vorschaubild in die Zwischenablage – Firepixel erzeugt keine Stil-Vorschaubilder"),
    ("BehaviorInfo", "*", "Objekt der HTML-Exportvorlagen des Originals; Firepixel exportiert HTML/Verhalten nativ (fp-export) und führt diese Vorlagen nicht aus"),
    ("BehaviorInfo.menuItems[i]", "*", "wie BehaviorInfo (HTML-Exportvorlagen)"),
    ("BehaviorsList", "*", "wie BehaviorInfo (HTML-Exportvorlagen)"),
    ("ImageMap", "*", "wie BehaviorInfo (HTML-Exportvorlagen)"),
    ("ImagemapList", "*", "wie BehaviorInfo (HTML-Exportvorlagen)"),
    ("SliceInfo", "*", "wie BehaviorInfo (HTML-Exportvorlagen)"),
    ("Slices", "*", "wie BehaviorInfo (HTML-Exportvorlagen)"),
    ("exportDoc", "*", "wie BehaviorInfo (HTML-Exportvorlagen)"),
];

fn listed(table: &[(&str, &[&str])], cls: &str, name: &str) -> bool {
    table.iter().any(|(c, names)| *c == cls && (names.contains(&name) || names.contains(&"*")))
}

fn excepted(cls: &str, name: &str) -> bool {
    EXCEPTIONS.iter().any(|(c, m, _)| *c == cls && (*m == name || *m == "*"))
}

fn implemented(table: &[(&str, &[&str])], cls: &str, name: &str) -> bool {
    listed(table, cls, name) && !excepted(cls, name)
}

/// (umgesetzte Methoden, Methoden gesamt, umgesetzte Properties, Properties gesamt).
pub fn coverage() -> (usize, usize, usize, usize) {
    let mut m = 0;
    let mut p = 0;
    for c in CLASSES {
        m += c.methods.iter().filter(|x| implemented(METHODS, c.js, x.name)).count();
        p += c.props.iter().filter(|x| implemented(PROPS, c.js, x.name)).count();
    }
    (m, COUNTS.0, p, COUNTS.1)
}

/// Bewusste Ausnahmen: (Methoden, Properties).
pub fn exceptions() -> (usize, usize) {
    let mut m = 0;
    let mut p = 0;
    for c in CLASSES {
        m += c.methods.iter().filter(|x| excepted(c.js, x.name)).count();
        p += c.props.iter().filter(|x| excepted(c.js, x.name)).count();
    }
    (m, p)
}

/// Übersicht als Markdown (MCP-Ressource `firepixel://docs/jsf.md`).
pub fn coverage_markdown() -> String {
    let (m, mt, p, pt) = coverage();
    let (em, ep) = exceptions();
    let mut s = String::new();
    let _ = writeln!(s, "# JSF-Kompatibilität\n");
    let _ = writeln!(
        s,
        "Umgesetzt: **{m} von {mt} Methoden**, **{p} von {pt} Properties** der Bindungstabelle des Originals; bewusst ausgelassen: {em} Methoden, {ep} Properties (Begründung unten).\n"
    );
    let _ = writeln!(
        s,
        "Typregeln wie im Original: fehlende Argumente aus Vorgaben, sonst Ausnahme `-2147216337` (EBadParamCount); überzählige Argumente werden ignoriert; \
`\"5\"` gilt als Zahl, Zahlen als Text; `null` für boolean/int/float wirft `-2147216343` (EBadParamType); Zuweisung an read-only-Properties wird ignoriert; \
Fehler werden als HRESULT-Zahl geworfen. Nicht umgesetzte Member werfen `Error(\"nicht implementiert: Klasse.member\")`.\n"
    );
    let _ = writeln!(
        s,
        "Zusätzlich: `fp.run(name, params)` führt einen Befehl des Befehlsregisters aus (Format von „Schritte kopieren“ und gespeicherten Befehlen), \
`fp.available(name)` prüft, ob er im aktuellen Zustand verfügbar ist. Automatische Formen: `dom.insertSmartShapeAt`, `dom.dragControlPoint`, \
`dom.redrawSmartShape`; die Formen-Skripte laufen in einer eigenen Laufzeit mit `smartShape`, `Path`, `Contour`, `ContourNode`, `ControlPoint`.\n"
    );
    for c in CLASSES {
        let mi: Vec<&str> = c.methods.iter().filter(|x| implemented(METHODS, c.js, x.name)).map(|x| x.name).collect();
        let pi: Vec<&str> = c.props.iter().filter(|x| implemented(PROPS, c.js, x.name)).map(|x| x.name).collect();
        if mi.is_empty() && pi.is_empty() {
            if let Some((_, _, why)) = EXCEPTIONS.iter().find(|(k, m, _)| *k == c.js && *m == "*") {
                let _ = writeln!(s, "- **{}**: ausgelassen – {why}", c.js);
            } else {
                let _ = writeln!(s, "- **{}**: noch nicht umgesetzt ({} Methoden, {} Properties)", c.js, c.methods.len(), c.props.len());
            }
            continue;
        }
        let _ = writeln!(s, "- **{}**: Methoden {}/{}, Properties {}/{}", c.js, mi.len(), c.methods.len(), pi.len(), c.props.len());
        if !mi.is_empty() {
            let _ = writeln!(s, "  - Methoden: {}", mi.join(", "));
        }
        if !pi.is_empty() {
            let _ = writeln!(s, "  - Properties: {}", pi.join(", "));
        }
        for (_, m, why) in EXCEPTIONS.iter().filter(|(k, m, _)| *k == c.js && *m != "*") {
            let _ = writeln!(s, "  - ausgelassen: {m} – {why}");
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listed_members_exist() {
        for (cls, names) in METHODS {
            assert!(CLASSES.iter().any(|c| c.js == *cls), "Klasse {cls} fehlt");
            for n in names.iter().filter(|n| **n != "*") {
                let ok = CLASSES.iter().any(|c| c.js == *cls && c.methods.iter().any(|m| m.name == *n));
                assert!(ok, "{cls}.{n} nicht in der Bindungstabelle");
            }
        }
        for (cls, names) in PROPS {
            for n in names.iter().filter(|n| **n != "*") {
                let ok = CLASSES.iter().any(|c| c.js == *cls && c.props.iter().any(|m| m.name == *n));
                assert!(ok, "{cls}.{n} nicht in der Bindungstabelle");
            }
        }
        let (m, mt, p, pt) = coverage();
        let (em, ep) = exceptions();
        assert!(m + em <= mt && p + ep <= pt);
        assert!(m > 500 && p > 500, "{m} {p}");
    }
}
