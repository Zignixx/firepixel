//! Bindungstabelle der JSF-API (03 §10): Klassen, Methoden mit Parametertypen und
//! Vorgaben, Properties mit Typ und read-only. Die Daten erzeugt
//! `tools/golden/fp-script/gen_bindings.py` aus `re/api/fw_api.json` ([`crate::bindings_data`]).

pub use crate::bindings_data::{CLASSES, COUNTS, ERRORS};

/// Typcode eines Arguments bzw. einer Property (03 §10.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeCode {
    /// `%Aa` Liste (JS-Array oder FwArray).
    Array,
    /// `%Ab` boolean.
    Bool,
    /// `%Ac` Farbe `"#RRGGBB[AA]"`.
    Color,
    /// `%Ad` Wörterbuch (tief konvertiertes Objekt).
    Dict,
    /// `%Ae` Datum.
    Date,
    /// `%Af` Gleitkommazahl.
    Float,
    /// `%Ai` Ganzzahl.
    Int,
    /// `%Am` Matrix `{matrix:[9]}`.
    Matrix,
    /// `%Ao` Objekt (Wrapper oder tief konvertiert).
    Object,
    /// `%Ap` Punkt `{x, y}`.
    Point,
    /// `%Ar` Rechteck `{left, top, right, bottom}`.
    Rect,
    /// `%As` String.
    Str,
    /// `%At` beliebig.
    Any,
    /// `%Au` Datei-URL.
    Url,
    /// `%Az` Auflösung `{pixelsPerUnit, units}`.
    Resolution,
    /// `%Av` kein Wert.
    Void,
}

impl TypeCode {
    /// Wird `null`/`undefined` angenommen (03 §6, §10.2)? Nein bei boolean/int/float/color/point/rect/resolution.
    pub fn null_ok(self) -> bool {
        !matches!(self, TypeCode::Bool | TypeCode::Int | TypeCode::Float | TypeCode::Color | TypeCode::Point | TypeCode::Rect | TypeCode::Resolution)
    }
}

/// Vorgabe eines Parameters (03 §10.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DefaultArg {
    /// Pflicht.
    Req,
    Null,
    Bool(bool),
    Int(i64),
}

/// Parameter einer Methode.
#[derive(Clone, Copy, Debug)]
pub struct ParamSpec {
    pub name: &'static str,
    pub code: TypeCode,
    pub default: DefaultArg,
}

/// Methode.
#[derive(Clone, Copy, Debug)]
pub struct MethodSpec {
    pub name: &'static str,
    pub params: &'static [ParamSpec],
    pub ret: TypeCode,
}

/// Property.
#[derive(Clone, Copy, Debug)]
pub struct PropSpec {
    pub name: &'static str,
    pub code: TypeCode,
    pub readonly: bool,
}

/// Klasse.
#[derive(Clone, Copy, Debug)]
pub struct ClassSpec {
    /// Klassen-Tag im Binary (`CJs_Document`).
    pub tag: &'static str,
    /// JS-Name (`Document`).
    pub js: &'static str,
    pub methods: &'static [MethodSpec],
    pub props: &'static [PropSpec],
}

/// Errors-Eintrag.
#[derive(Clone, Copy, Debug)]
pub struct ErrorSpec {
    pub name: &'static str,
    pub hresult: u32,
}

/// Index einer Klasse zum JS-Namen.
pub fn class_index(js: &str) -> Option<usize> {
    CLASSES.iter().position(|c| c.js == js)
}

/// Basisklassen (Vererbung laut 03 §2; nicht in der Bindungstabelle).
pub fn parents(js: &str) -> &'static [&'static str] {
    match js {
        "Path" | "Text" | "Image" | "Group" | "Instance" | "RectanglePrimitive" | "Hotspot" | "CompoundShape" => &["Element", "(DomObject-Basis)"],
        "SliceHotspot" => &["Hotspot", "Element", "(DomObject-Basis)"],
        "Element" => &["(DomObject-Basis)"],
        _ => &[],
    }
}

/// Klasse und ihre Basisklassen (eigene zuerst).
pub fn chain(js: &str) -> Vec<&'static ClassSpec> {
    let mut v: Vec<&'static ClassSpec> = CLASSES.iter().filter(|c| c.js == js).collect();
    for p in parents(js) {
        v.extend(CLASSES.iter().filter(|c| c.js == *p));
    }
    v
}

/// Property einer Klasse (mit Vererbung).
pub fn find_prop(js: &str, name: &str) -> Option<&'static PropSpec> {
    chain(js).into_iter().find_map(|c| c.props.iter().find(|p| p.name == name))
}

/// HRESULT eines Errors-Namens.
pub fn error_hr(name: &str) -> Option<u32> {
    ERRORS.iter().find(|e| e.name == name).map(|e| e.hresult)
}

/// Wichtige HRESULT-Codes (03 §10.4).
pub mod hr {
    pub const E_BAD_PARAM: u32 = 0x8004_0001;
    pub const E_FILE_NOT_FOUND: u32 = 0x8004_0006;
    pub const E_INTERNAL: u32 = 0x8004_0014;
    pub const E_GENERIC: u32 = 0x8004_1417;
    pub const E_BAD_SELECTION: u32 = 0x8004_1428;
    pub const E_BAD_PARAM_TYPE: u32 = 0x8004_1429;
    pub const E_NO_ACTIVE_DOCUMENT: u32 = 0x8004_142b;
    pub const E_BAD_PARAM_COUNT: u32 = 0x8004_142f;
    pub const E_NOT_IMPLEMENTED: u32 = 0x8000_4001;
    pub const E_USER_CANCELED: u32 = 0x8004_5208;
    pub const E_ARRAY_INDEX: u32 = 0x8004_1410;
}

/// Kurzer Text zu einem Errors-Namen (eigene Formulierung) in der aktuellen Sprache (skript.ftl, `err-script-hr-*`).
pub fn error_text(name: &str) -> String {
    let key = match name {
        "EBadParam" | "EBadParamCount" => "err-script-hr-ebad-param",
        "EBadParamType" => "err-script-hr-ebad-param-type",
        "EOutOfMem" | "ELowOnMem" => "err-script-hr-eout-of-mem",
        "EFileNotFound" => "err-script-hr-efile-not-found",
        "EDiskFull" => "err-script-hr-edisk-full",
        "EGenericErrorOccurred" => "err-script-hr-egeneric-error-occurred",
        "ENoSliceableElems" => "err-script-hr-eno-sliceable-elems",
        "ESharingViolation" => "err-script-hr-esharing-violation",
        "EUnknownReaderFormat" => "err-script-hr-eunknown-reader-format",
        "EBadFileContents" => "err-script-hr-ebad-file-contents",
        "EFileIsReadOnly" => "err-script-hr-efile-is-read-only",
        "ENoFilesSelected" => "err-script-hr-eno-files-selected",
        "EBadSelection" => "err-script-hr-ebad-selection",
        "EBadJsVersion" => "err-script-hr-ebad-js-version",
        "ENoActiveDocument" => "err-script-hr-eno-active-document",
        "ENoActiveSelection" => "err-script-hr-eno-active-selection",
        "CouldNotCompleteYourRequest" => "err-script-hr-could-not-complete-your-request",
        _ => "err-script-hr-internal",
    };
    fp_i18n::t(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_match_spec() {
        assert_eq!(COUNTS, (565, 721));
        let doc = &CLASSES[class_index("Document").unwrap_or(0)];
        assert!(doc.methods.iter().any(|m| m.name == "addNewRectangle"));
        assert_eq!(error_hr("EBadParamCount"), Some(hr::E_BAD_PARAM_COUNT));
        assert!(find_prop("Path", "left").is_some_and(|p| !p.readonly));
        assert!(find_prop("Path", "width").is_some_and(|p| p.readonly));
    }
}
