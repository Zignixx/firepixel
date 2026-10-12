//! Eingebaute automatische Formen: Namen (englisch wie der Formname in Dateien, deutsch wie in der
//! Oberfläche), Gruppe (`tool` = Rechteck-Flyout, `panel` = Bedienfeld „Automatische Formen“),
//! eigenes Skript und die Parameter für das Eigenschaften-Bedienfeld.

/// Parameter einer Form (Eigenschaften-Bedienfeld, `autoShape.describe`).
#[derive(Clone, Copy, Debug)]
pub struct ParamDef {
    pub key: &'static str,
    pub label: &'static str,
    /// `number`, `integer`, `bool`, `choice`, `text`, `color`.
    pub kind: &'static str,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub options: &'static [&'static str],
}

const fn num(key: &'static str, label: &'static str, min: f64, max: f64) -> ParamDef {
    ParamDef { key, label, kind: "number", min: Some(min), max: Some(max), options: &[] }
}
const fn int(key: &'static str, label: &'static str, min: f64, max: f64) -> ParamDef {
    ParamDef { key, label, kind: "integer", min: Some(min), max: Some(max), options: &[] }
}
const fn flag(key: &'static str, label: &'static str) -> ParamDef {
    ParamDef { key, label, kind: "bool", min: None, max: None, options: &[] }
}
const fn color(key: &'static str, label: &'static str) -> ParamDef {
    ParamDef { key, label, kind: "color", min: None, max: None, options: &[] }
}
const fn text(key: &'static str, label: &'static str) -> ParamDef {
    ParamDef { key, label, kind: "text", min: None, max: None, options: &[] }
}

/// Eine Form.
#[derive(Clone, Copy, Debug)]
pub struct ShapeDef {
    /// Formname (englisch, steht in `ISSv/FILs`).
    pub en: &'static str,
    /// Deutscher Name (Werkzeug-Flyout, Bedienfeld).
    pub de: &'static str,
    /// `tool` oder `panel`.
    pub group: &'static str,
    /// Skript (eigener Code, siehe `shapes/*.js`).
    pub source: &'static str,
    pub params: &'static [ParamDef],
}

const W: f64 = 100_000.0;

const fn choice(key: &'static str, label: &'static str, options: &'static [&'static str]) -> ParamDef {
    ParamDef { key, label, kind: "choice", min: None, max: None, options }
}

const HEADS: &[&str] = &["Ohne", "Spitze 1", "Spitze 2", "Querstrich", "Kästchen"];

const RECT_PARAMS: &[ParamDef] = &[
    num("width", "Breite", 1.0, W),
    num("height", "Höhe", 1.0, W),
    num("corner0", "Ecke oben links", 0.0, W),
    num("corner1", "Ecke oben rechts", 0.0, W),
    num("corner2", "Ecke unten rechts", 0.0, W),
    num("corner3", "Ecke unten links", 0.0, W),
    choice("cornerType0", "Eckenart", &["gerade", "rund", "nach innen"]),
    flag("locked", "Ecken gemeinsam"),
];

const OBLIQUE_PARAMS: &[ParamDef] = &[num("size", "Größe", 4.0, W), num("dx", "Versatz X", -W, W), num("dy", "Versatz Y", -W, W), num("inner", "Innerer Radius (Röhre)", 0.0, 0.95)];
const BUBBLE_PARAMS: &[ParamDef] = &[num("rx", "Breite", 4.0, W), num("ry", "Höhe", 4.0, W), num("tx", "Zeiger X", -W, W), num("ty", "Zeiger Y", -W, W)];
const PIE_PARAMS: &[ParamDef] = &[num("radius0", "Radius", 1.0, W), num("radius1", "Innerer Radius", 0.0, W)];

/// Alle eingebauten Formen.
pub static SHAPES: &[ShapeDef] = &[
    ShapeDef {
        en: "Rounded Rectangle",
        de: "Abgerundetes Rechteck",
        group: "tool",
        source: include_str!("shapes/rounded_rectangle.js"),
        params: RECT_PARAMS,

    },
    ShapeDef {
        en: "Beveled Rectangle",
        de: "Abgeflachtes Rechteck",
        group: "tool",
        source: concat!("var __cornerDefault = 0;\n", include_str!("shapes/rounded_rectangle.js")),
        params: RECT_PARAMS,
    },
    ShapeDef {
        en: "Chamfer Rectangle",
        de: "Abgefastes Rechteck",
        group: "tool",
        source: concat!("var __cornerDefault = 2;\n", include_str!("shapes/rounded_rectangle.js")),
        params: RECT_PARAMS,
    },
    ShapeDef {
        en: "Arrow",
        de: "Pfeil",
        group: "tool",
        source: include_str!("shapes/arrow.js"),
        params: &[num("width", "Schaftlänge", 1.0, W), num("height", "Schaftdicke", 1.0, W), num("arrowWidth", "Breite der Spitze", 1.0, W), num("arrowTip", "Länge der Spitze", 1.0, W), num("roundness", "Rundung", 0.0, W)],
    },
    ShapeDef {
        en: "Arrow Line",
        de: "Pfeillinie",
        group: "tool",
        source: include_str!("shapes/arrow_line.js"),
        params: &[choice("head1", "Anfang", HEADS), choice("head2", "Ende", HEADS), num("size", "Spitzengröße", 1.0, 1000.0)],
    },
    ShapeDef {
        en: "Connector Line",
        de: "Verbindungslinie",
        group: "tool",
        source: include_str!("shapes/connector.js"),
        params: &[choice("mode", "Richtung", &["waagerecht – senkrecht – waagerecht", "senkrecht – waagerecht – senkrecht"]), num("corner1", "Rundung Knick 1", 0.0, W), num("corner2", "Rundung Knick 2", 0.0, W)],
    },
    ShapeDef {
        en: "Doughnut",
        de: "Donut",
        group: "tool",
        source: concat!("var __pieInner = 0.4; var __pieAngles = [-90, -90];\n", include_str!("shapes/pie.js")),
        params: PIE_PARAMS,
    },
    ShapeDef {
        en: "Pie",
        de: "Torte",
        group: "tool",
        source: concat!("var __pieInner = 0; var __pieAngles = [-90, 0, 270];\n", include_str!("shapes/pie.js")),
        params: PIE_PARAMS,
    },
    ShapeDef {
        en: "L-Shape",
        de: "L-Form",
        group: "tool",
        source: include_str!("shapes/l_shape.js"),
        params: &[num("width", "Breite", 1.0, W), num("height", "Höhe", 1.0, W), num("thickness", "Dicke", 1.0, W), num("roundness", "Rundung", 0.0, W)],
    },
    ShapeDef {
        en: "Measure Tool",
        de: "Messwerkzeug",
        group: "tool",
        source: include_str!("shapes/measure.js"),
        params: &[choice("units", "Einheit", &["px", "inch", "cm"])],
    },
    ShapeDef {
        en: "Smart Polygon",
        de: "Intelligentes Polygon",
        group: "tool",
        source: include_str!("shapes/smart_polygon.js"),
        params: &[int("sides", "Seiten", 3.0, 360.0), num("radius", "Radius", 1.0, W), num("rotation", "Drehung", -360.0, 360.0), num("inner", "Innenausschnitt", 0.0, W)],
    },
    ShapeDef {
        en: "Spiral",
        de: "Spirale",
        group: "tool",
        source: include_str!("shapes/spiral.js"),
        params: &[num("turns", "Windungen", 0.25, 50.0), num("radius", "Radius", 2.0, W), flag("clockwise", "Im Uhrzeigersinn"), flag("closed", "Geschlossen")],
    },
    ShapeDef {
        en: "Star",
        de: "Stern",
        group: "tool",
        source: include_str!("shapes/star.js"),
        params: &[int("nodes", "Zacken", 3.0, 50.0), num("radius0", "Radius 1", 1.0, W), num("radius1", "Radius 2", 0.0, W), num("round0", "Rundung 1", 0.0, W), num("round1", "Rundung 2", 0.0, W)],
    },
    ShapeDef {
        en: "3D Box",
        de: "3D-Kasten",
        group: "panel",
        source: include_str!("shapes/box3d.js"),
        params: &[num("size", "Größe", 4.0, W), num("rotX", "Drehung X", -360.0, 360.0), num("rotY", "Drehung Y", -360.0, 360.0), num("focal", "Brennweite", 60.0, 5000.0)],
    },
    ShapeDef {
        en: "Annotation",
        de: "Anmerkung",
        group: "panel",
        source: concat!("var __miscKind = \"annotation\";\n", include_str!("shapes/misc.js")),
        params: &[text("text", "Text"), num("w", "Breite", 20.0, W), num("h", "Höhe", 16.0, W)],
    },
    ShapeDef {
        en: "Calendar",
        de: "Kalender",
        group: "panel",
        source: concat!("var __miscKind = \"calendar\";\n", include_str!("shapes/misc.js")),
        params: &[int("year", "Jahr", 1.0, 9999.0), int("month", "Monat (0–11)", 0.0, 11.0)],
    },
    ShapeDef {
        en: "Clock",
        de: "Uhr",
        group: "panel",
        source: concat!("var __miscKind = \"clock\";\n", include_str!("shapes/misc.js")),
        params: &[int("hour", "Stunde", 0.0, 11.0), int("minute", "Minute", 0.0, 59.0), num("r", "Radius", 8.0, W)],
    },
    ShapeDef {
        en: "Cog",
        de: "Zahnrad",
        group: "panel",
        source: concat!("var __curveKind = \"cog\";\n", include_str!("shapes/curves.js")),
        params: &[int("teeth", "Zähne", 3.0, 120.0), num("r", "Radius", 4.0, W), num("depth", "Zahntiefe", 1.0, W), num("hole", "Loch", 0.0, W)],
    },
    ShapeDef {
        en: "Crescent",
        de: "Mondsichel",
        group: "panel",
        source: concat!("var __curveKind = \"crescent\";\n", include_str!("shapes/curves.js")),
        params: &[num("arc", "Bogen", 10.0, 350.0), num("hollow", "Hohl (0–1)", 0.05, 0.95), num("r", "Radius", 2.0, W)],
    },
    ShapeDef {
        en: "Cube",
        de: "Würfel",
        group: "panel",
        source: concat!("var __obliqueKind = 0;\n", include_str!("shapes/oblique.js")),
        params: OBLIQUE_PARAMS,
    },
    ShapeDef {
        en: "Cylinder",
        de: "Zylinder",
        group: "panel",
        source: concat!("var __obliqueKind = 1;\n", include_str!("shapes/oblique.js")),
        params: OBLIQUE_PARAMS,
    },
    ShapeDef {
        en: "File Info",
        de: "Dateiinfo",
        group: "panel",
        source: concat!("var __miscKind = \"fileinfo\";\n", include_str!("shapes/misc.js")),
        params: &[int("fmt", "Format", 0.0, 2.0)],
    },
    ShapeDef {
        en: "Flowchart",
        de: "Flussdiagramm",
        group: "panel",
        source: concat!("var __miscKind = \"flowchart\";\n", include_str!("shapes/misc.js")),
        params: &[int("sel", "Symbol (-1 = Auswahl)", -1.0, 27.0), num("w", "Breite", 8.0, W), num("h", "Höhe", 8.0, W)],
    },
    ShapeDef {
        en: "Frame",
        de: "Rahmen",
        group: "panel",
        source: concat!("var __miscKind = \"frame\";\n", include_str!("shapes/misc.js")),
        params: &[num("fw", "Rahmenbreite", 1.0, W), num("w", "Breite", 4.0, W), num("h", "Höhe", 4.0, W), color("color", "Farbe")],
    },
    ShapeDef {
        en: "Frame 2",
        de: "Rahmen 2",
        group: "panel",
        source: concat!("var __miscKind = \"frame2\";\n", include_str!("shapes/misc.js")),
        params: &[num("fw", "Rahmenbreite", 1.0, W), num("over", "Überhang", 0.0, 200.0), num("w", "Breite", 4.0, W), num("h", "Höhe", 4.0, W)],
    },
    ShapeDef {
        en: "Frame 3",
        de: "Rahmen 3",
        group: "panel",
        source: concat!("var __miscKind = \"frame3\";\n", include_str!("shapes/misc.js")),
        params: &[num("off", "Rahmenversatz", 0.0, W), num("corner", "Eckgröße", 0.0, W), num("w", "Breite", 4.0, W), num("h", "Höhe", 4.0, W)],
    },
    ShapeDef {
        en: "Heart",
        de: "Herz",
        group: "panel",
        source: concat!("var __curveKind = \"heart\";\n", include_str!("shapes/curves.js")),
        params: &[num("w", "Breite", 2.0, W), num("gap", "Spalt", -W, W), num("apex", "Scheitelpunkt", 2.0, W), num("apexArc", "Scheitelbogen", 0.0, W)],
    },
    ShapeDef {
        en: "Inspire",
        de: "Anregung",
        group: "panel",
        source: concat!("var __miscKind = \"inspire\";\n", include_str!("shapes/misc.js")),
        params: &[int("seed", "Zufallswert", 1.0, 100_000.0), num("r", "Größe", 8.0, W)],
    },
    ShapeDef {
        en: "Perspective",
        de: "Perspektive",
        group: "panel",
        source: concat!("var __miscKind = \"perspective\";\n", include_str!("shapes/misc.js")),
        params: &[int("n", "Unterteilungen", 1.0, 40.0)],
    },
    ShapeDef {
        en: "Pie Chart",
        de: "Tortendiagramm",
        group: "panel",
        source: concat!("var __pieInner = 0; var __pieCentered = true; var __pieAngles = [-90, 0, 135, 270];\n", include_str!("shapes/pie.js")),
        params: PIE_PARAMS,
    },
    ShapeDef {
        en: "Pinwheel",
        de: "Windrad",
        group: "panel",
        source: concat!("var __curveKind = \"pinwheel\";\n", include_str!("shapes/curves.js")),
        params: &[int("blades", "Anzahl", 1.0, 36.0), num("curve", "Kurve", -170.0, 170.0), num("r", "Größe", 4.0, W), num("rot", "Drehung", -360.0, 360.0), int("style", "Stil (0 einfarbig, 1 zweifarbig)", 0.0, 1.0)],
    },
    ShapeDef {
        en: "Posable Fig",
        de: "Gliederpuppe",
        group: "panel",
        source: concat!("var __miscKind = \"posable\";\n", include_str!("shapes/misc.js")),
        params: &[],
    },
    ShapeDef {
        en: "Revolve Path",
        de: "Rotationspfad",
        group: "panel",
        source: concat!("var __curveKind = \"revolve\";\n", include_str!("shapes/curves.js")),
        params: &[int("div", "Unterteilungen", 2.0, 64.0)],
    },
    ShapeDef {
        en: "Rounded Polygon",
        de: "Abgerundetes Polygon",
        group: "panel",
        source: concat!("var __curveKind = \"roundpoly\";\n", include_str!("shapes/curves.js")),
        params: &[int("sides", "Seiten", 3.0, 64.0), num("curve", "Kurvenradius", 0.0, W), num("r", "Radius", 2.0, W)],
    },
    ShapeDef {
        en: "Ruler",
        de: "Lineal",
        group: "panel",
        source: concat!("var __miscKind = \"ruler\";\n", include_str!("shapes/misc.js")),
        params: &[],
    },
    ShapeDef {
        en: "Save Time Stamp",
        de: "Zeitstempel",
        group: "panel",
        source: concat!("var __miscKind = \"timestamp\";\n", include_str!("shapes/misc.js")),
        params: &[int("fmt", "Format", 0.0, 1.0)],
    },
    ShapeDef {
        en: "Tabs",
        de: "Registerkarten",
        group: "panel",
        source: concat!("var __miscKind = \"tabs\";\n", include_str!("shapes/misc.js")),
        params: &[int("count", "Anzahl", 1.0, 20.0), num("hue", "Farbton", 0.0, 360.0), num("tw", "Breite", 4.0, W), num("th", "Höhe", 4.0, W)],
    },
    ShapeDef {
        en: "Talking",
        de: "Sprechblase",
        group: "panel",
        source: concat!("var __miscKind = \"talking\";\n", include_str!("shapes/misc.js")),
        params: BUBBLE_PARAMS,
    },
    ShapeDef {
        en: "Thinking",
        de: "Gedankenblase",
        group: "panel",
        source: concat!("var __miscKind = \"thinking\";\n", include_str!("shapes/misc.js")),
        params: BUBBLE_PARAMS,
    },
    ShapeDef {
        en: "Tube",
        de: "Röhre",
        group: "panel",
        source: concat!("var __obliqueKind = 2;\n", include_str!("shapes/oblique.js")),
        params: OBLIQUE_PARAMS,
    },
    ShapeDef {
        en: "Wave",
        de: "Welle",
        group: "panel",
        source: concat!("var __curveKind = \"wave\";\n", include_str!("shapes/curves.js")),
        params: &[num("len", "Länge", 4.0, W), num("h", "Höhe", 0.0, W), num("wl", "Wellenlänge", 2.0, W), num("curv", "Krümmung", 0.0, 100.0), num("angle", "Drehung", -360.0, 360.0)],
    },
];

/// Form zum Namen (englisch oder deutsch, Groß-/Kleinschreibung und Leer-/Bindestriche egal).
pub fn find_shape(name: &str) -> Option<&'static ShapeDef> {
    let norm = |s: &str| s.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect::<String>();
    let n = norm(name.trim_end_matches(".jsf"));
    SHAPES.iter().find(|s| norm(s.en) == n || norm(s.de) == n)
}
