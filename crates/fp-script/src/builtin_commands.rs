//! Mitgelieferte Befehlsskripte für das Menü „Befehle“ (eigener Code). Beim ersten Start schreibt
//! die Oberfläche sie in den Befehlsordner (`fp_engine::commands::misc::commands_dir()`);
//! Unterordner werden Untermenüs. Vorhandene Dateien bleiben unverändert (Nutzer dürfen sie
//! anpassen oder löschen).

use std::path::{Path, PathBuf};

/// (relativer Pfad, Quelltext).
pub const BUILTIN: &[(&str, &str)] = &[
    // ------------------------------------------------------------- Batch-Befehle
    ("Batch-Befehle/Alles auswählen.jsf", "fw.getDocumentDOM().selectAll();\n"),
    ("Batch-Befehle/Auswahl automatisch tonwertkorrigieren.jsf", "var d = fw.getDocumentDOM();\nif (fw.selection.length) d.filterSelectionByName(\"Adjust Color\", \"Auto Levels\");\n"),
    ("Batch-Befehle/Auswahl weichzeichnen.jsf", "var d = fw.getDocumentDOM();\nif (fw.selection.length) d.filterSelectionByName(\"Blur\", \"Blur\");\n"),
    ("Batch-Befehle/Auswahl scharfzeichnen.jsf", "var d = fw.getDocumentDOM();\nif (fw.selection.length) d.filterSelectionByName(\"Sharpen\", \"Sharpen\");\n"),
    ("Batch-Befehle/Farbe der Auswahl umkehren.jsf", "var d = fw.getDocumentDOM();\nif (fw.selection.length) d.filterSelectionByName(\"Adjust Color\", \"Invert\");\n"),
    ("Batch-Befehle/Dokument um 90° drehen.jsf", "fw.getDocumentDOM().rotateDocument(90);\n"),
    ("Batch-Befehle/Dokument um 180° drehen.jsf", "fw.getDocumentDOM().rotateDocument(180);\n"),
    ("Batch-Befehle/Dokument um 270° drehen.jsf", "fw.getDocumentDOM().rotateDocument(270);\n"),
    ("Batch-Befehle/Auswahl um 90° drehen.jsf", "if (fw.selection.length) fw.getDocumentDOM().rotateSelection(90, \"autoTrimImages transformAttributes\");\n"),
    ("Batch-Befehle/Auswahl um 180° drehen.jsf", "if (fw.selection.length) fw.getDocumentDOM().rotateSelection(180, \"autoTrimImages transformAttributes\");\n"),
    ("Batch-Befehle/Auswahl um 270° drehen.jsf", "if (fw.selection.length) fw.getDocumentDOM().rotateSelection(270, \"autoTrimImages transformAttributes\");\n"),
    // ------------------------------------------------------------- Kreativ
    (
        "Kreativ/Schatten hinzufügen.jsf",
        "// Schlagschatten auf die Auswahl (vorhandene Effekte bleiben erhalten)\n\
var d = fw.getDocumentDOM();\n\
if (fw.selection.length) {\n\
  var list = fw.selection[0].effectList;\n\
  var effects = list ? list.effects.slice(0) : [];\n\
  effects.push({ EffectMoaID: \"{a7944db8-6ce2-11d1-8c76000502701850}\", EffectIsVisible: true, ShadowAngle: 315, ShadowBlur: 4, ShadowColor: \"#000000a5\", ShadowDistance: 7, ShadowType: 0, category: \"Shadow and Glow\", name: \"Drop Shadow\" });\n\
  d.applyEffects({ category: \"UNUSED\", name: \"UNUSED\", effects: effects });\n\
}\n",
    ),
    (
        "Kreativ/In Graustufen umwandeln.jsf",
        "var d = fw.getDocumentDOM();\nif (fw.selection.length) fp.run(\"filter.adjustColor.hueSaturation\", { hue: 0, saturation: -100, lightness: 0, live: true });\n",
    ),
    (
        "Kreativ/In Sepiaton umwandeln.jsf",
        "var d = fw.getDocumentDOM();\nif (fw.selection.length) fp.run(\"filter.adjustColor.hueSaturation\", { colorize: true, hue: 30, saturation: 25, lightness: 0, live: true });\n",
    ),
    (
        "Kreativ/Bilderrahmen hinzufügen.jsf",
        "// Rahmen um die Leinwand auf einer eigenen Ebene\n\
var d = fw.getDocumentDOM();\n\
var w = Math.max(4, Math.round(Math.min(d.width, d.height) * 0.05));\n\
d.addNewLayer(\"Rahmen\", false);\n\
d.addNewRectanglePrimitive({ left: w / 2, top: w / 2, right: d.width - w / 2, bottom: d.height - w / 2 }, 0);\n\
d.setFill(null);\n\
d.setBrushNColor({ category: \"bc_Pencil\", name: \"bn_1-Pixel Hard\", diameter: w, softness: 0, shape: \"square\", type: \"simple\" }, \"#5a3a1e\");\n",
    ),
    (
        "Kreativ/Verdrehen und ausblenden.jsf",
        "// Kopien der Auswahl, jeweils gedreht, verschoben und blasser\n\
var d = fw.getDocumentDOM();\n\
var n = parseInt(prompt(\"Anzahl der Kopien\", \"5\"), 10) || 5;\n\
if (fw.selection.length) {\n\
  for (var i = 1; i <= n; i++) {\n\
    d.cloneSelection();\n\
    d.rotateSelection(10, \"autoTrimImages transformAttributes\");\n\
    d.moveSelectionBy({ x: 6, y: 6 }, false, false);\n\
    d.setOpacity(Math.max(5, 100 - i * Math.floor(90 / n)));\n\
  }\n\
}\n",
    ),
    (
        "Kreativ/Pfeilspitzen hinzufügen.jsf",
        "// Pfeilspitze am Ende jedes ausgewählten offenen Pfads\n\
var d = fw.getDocumentDOM();\n\
var sel = fw.selection.slice(0);\n\
for (var i = 0; i < sel.length; i++) {\n\
  var e = sel[i];\n\
  if (!e.contours || !e.contours.length || e.contours[0].isClosed) continue;\n\
  var ns = e.contours[0].nodes, n = ns.length;\n\
  if (n < 2) continue;\n\
  var p = ns[n - 1], q = ns[n - 2];\n\
  var a = Math.atan2(p.y - q.y, p.x - q.x), s = 10;\n\
  d.addNewLine({ x: p.x - s * Math.cos(a - 0.45), y: p.y - s * Math.sin(a - 0.45) }, { x: p.x, y: p.y });\n\
  d.addNewLine({ x: p.x - s * Math.cos(a + 0.45), y: p.y - s * Math.sin(a + 0.45) }, { x: p.x, y: p.y });\n\
}\n",
    ),
    // ------------------------------------------------------------- Dokument
    (
        "Dokument/Auf Ebenen verteilen.jsf",
        "// jedes ausgewählte Objekt auf eine eigene neue Ebene\n\
var d = fw.getDocumentDOM();\n\
var sel = fw.selection.slice(0);\n\
for (var i = sel.length - 1; i >= 0; i--) {\n\
  fw.selection = [sel[i]];\n\
  d.moveSelectionToNewLayer(false);\n\
}\n",
    ),
    (
        "Dokument/Andere Ebenen ausblenden.jsf",
        "var d = fw.getDocumentDOM();\nvar cur = d.currentLayerNum;\nfor (var i = 0; i < d.layers.length; i++) if (i != cur && d.layers[i].layerType != \"web\") d.setLayerVisible(i, -1, false, false);\n",
    ),
    (
        "Dokument/Andere Ebenen sperren.jsf",
        "var d = fw.getDocumentDOM();\nvar cur = d.currentLayerNum;\nfor (var i = 0; i < d.layers.length; i++) if (i != cur && d.layers[i].layerType != \"web\") d.setLayerLocked(i, -1, true, false);\n",
    ),
    (
        "Dokument/Status umkehren.jsf",
        "// Reihenfolge der Status umkehren\n\
var d = fw.getDocumentDOM();\n\
var n = d.frames.length;\n\
for (var i = 0; i < n - 1; i++) d.reorderFrame(n - 1, i, false);\n",
    ),
    // ------------------------------------------------------------- Auswahl
    ("Auswahl/Verbessertes Vektor-Rendering verwenden.jsf", "fw.getDocumentDOM().setUseAGMRenderingForSelection(true);\n"),
    ("Auswahl/Bisheriges Vektor-Rendering verwenden.jsf", "fw.getDocumentDOM().setUseAGMRenderingForSelection(false);\n"),
    // ------------------------------------------------------------- Text
    (
        "Text/Kleinbuchstaben.jsf",
        "var sel = fw.selection;\nfor (var i = 0; i < sel.length; i++) if (sel[i].textChars !== undefined) sel[i].textChars = String(sel[i].textChars).toLowerCase();\n",
    ),
    (
        "Text/Großbuchstaben.jsf",
        "var sel = fw.selection;\nfor (var i = 0; i < sel.length; i++) if (sel[i].textChars !== undefined) sel[i].textChars = String(sel[i].textChars).toUpperCase();\n",
    ),
    (
        "Text/Erster Buchstabe groß.jsf",
        "var sel = fw.selection;\nfor (var i = 0; i < sel.length; i++) if (sel[i].textChars !== undefined) sel[i].textChars = String(sel[i].textChars).replace(/(^|[\\s\\-])([a-zäöüß])/g, function (m, a, b) { return a + b.toUpperCase(); });\n",
    ),
    (
        "Text/Blindtext.jsf",
        "// Platzhaltertext in das ausgewählte Textobjekt bzw. ein neues Textfeld\n\
var t = \"Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.\";\n\
var d = fw.getDocumentDOM();\n\
var sel = fw.selection, done = false;\n\
for (var i = 0; i < sel.length; i++) if (sel[i].textChars !== undefined) { sel[i].textChars = t; done = true; }\n\
if (!done) { d.addNewText({ left: 10, top: 10, right: Math.min(d.width - 10, 310), bottom: 60 }, true); d.setTextRuns({ initialAttrs: { size: \"12pt\" }, textRuns: [{ changedAttrs: {}, characters: t }] }); }\n",
    ),
    ("Text/In Pfade umwandeln.jsf", "if (fw.selection.length) fw.getDocumentDOM().convertToPaths();\n"),
    ("Text/In Pixel umwandeln.jsf", "if (fw.selection.length) fw.getDocumentDOM().flattenSelection();\n"),
    (
        "Text/Breite festlegen.jsf",
        "var w = parseFloat(prompt(\"Breite in Pixel\", \"200\"));\nvar d = fw.getDocumentDOM(), sel = fw.selection;\nif (w > 0) for (var i = 0; i < sel.length; i++) if (sel[i].textChars !== undefined) { fw.selection = [sel[i]]; d.setTextRectangle({ left: sel[i].left, top: sel[i].top, right: sel[i].left + w, bottom: sel[i].top + sel[i].height }); }\nfw.selection = sel;\n",
    ),
    (
        "Text/Feste Breite umschalten.jsf",
        "var d = fw.getDocumentDOM(), sel = fw.selection;\nfor (var i = 0; i < sel.length; i++) if (sel[i].textChars !== undefined) { fw.selection = [sel[i]]; d.setTextAutoExpand(!sel[i].autoExpand); }\nfw.selection = sel;\n",
    ),
    ("Text/Verfestigen.jsf", "// Text in Pfade umwandeln und zu einem Objekt verbinden\nvar d = fw.getDocumentDOM();\nif (fw.selection.length) { d.convertToPaths(); d.ungroup(); d.pathUnion(); }\n"),
    // ------------------------------------------------------------- Web
    (
        "Web/Leeren ALT-Text auswählen.jsf",
        "// Hotspots und Segmente ohne ALT-Text auswählen\n\
var d = fw.getDocumentDOM(), web = d.layers[d.layers.length - 1].frames[d.currentFrameNum].elements, pick = [];\n\
for (var i = 0; i < web.length; i++) if (!web[i].altText) pick.push(web[i]);\n\
fw.selection = pick;\n",
    ),
    (
        "Web/ALT-Text festlegen.jsf",
        "var t = prompt(\"ALT-Text für die ausgewählten Hotspots und Segmente\", \"\");\nif (t !== null) { var sel = fw.selection; for (var i = 0; i < sel.length; i++) if (sel[i].altText !== undefined) sel[i].altText = t; }\n",
    ),
    // ------------------------------------------------------------- oberste Ebene
    (
        "Größe ausgewählter Objekte ändern.jsf",
        "var p = parseFloat(prompt(\"Neue Größe in Prozent\", \"50\"));\nif (p > 0 && fw.selection.length) fw.getDocumentDOM().scaleSelection(p / 100, p / 100, \"autoTrimImages transformAttributes\");\n",
    ),
    ("Warndialoge zurücksetzen.jsf", "fw.resetWarnings();\n"),
];

/// Englische Datei- und Ordnernamen (Menütexte) der mitgelieferten Befehle, gleiche Reihenfolge wie
/// [`BUILTIN`]. Wie im Original liegen die Befehle in der Sprache der Installation im Befehlsordner.
pub const BUILTIN_EN: &[&str] = &[
    "Batch Commands/Select All.jsf",
    "Batch Commands/Auto Levels Selection.jsf",
    "Batch Commands/Blur Selection.jsf",
    "Batch Commands/Sharpen Selection.jsf",
    "Batch Commands/Invert Selection Color.jsf",
    "Batch Commands/Rotate Document 90°.jsf",
    "Batch Commands/Rotate Document 180°.jsf",
    "Batch Commands/Rotate Document 270°.jsf",
    "Batch Commands/Rotate Selection 90°.jsf",
    "Batch Commands/Rotate Selection 180°.jsf",
    "Batch Commands/Rotate Selection 270°.jsf",
    "Creative/Add Drop Shadow.jsf",
    "Creative/Convert to Grayscale.jsf",
    "Creative/Convert to Sepia Tone.jsf",
    "Creative/Add Picture Frame.jsf",
    "Creative/Twist and Fade.jsf",
    "Creative/Add Arrowheads.jsf",
    "Document/Distribute to Layers.jsf",
    "Document/Hide Other Layers.jsf",
    "Document/Lock Other Layers.jsf",
    "Document/Reverse States.jsf",
    "Selection/Use Improved Vector Rendering.jsf",
    "Selection/Use Legacy Vector Rendering.jsf",
    "Text/Lowercase.jsf",
    "Text/Uppercase.jsf",
    "Text/Capitalize First Letter.jsf",
    "Text/Placeholder Text.jsf",
    "Text/Convert to Paths.jsf",
    "Text/Convert to Pixels.jsf",
    "Text/Set Width.jsf",
    "Text/Toggle Fixed Width.jsf",
    "Text/Flatten.jsf",
    "Web/Select Blank ALT Text.jsf",
    "Web/Set ALT Text.jsf",
    "Resize Selected Objects.jsf",
    "Reset Warning Dialogs.jsf",
];

/// Schreibt die mitgelieferten Befehle (deutsche Namen) in `dir`, soweit sie dort noch fehlen;
/// liefert die Zahl der neu geschriebenen Dateien.
pub fn install_into(dir: &Path) -> std::io::Result<usize> {
    install_into_lang(dir, "de")
}

/// Wie [`install_into`] mit Datei-/Ordnernamen in der Sprache `lang` (`de`, sonst Englisch).
pub fn install_into_lang(dir: &Path, lang: &str) -> std::io::Result<usize> {
    let german = lang == "de" || lang.starts_with("de-");
    let mut n = 0;
    for (i, (rel_de, src)) in BUILTIN.iter().enumerate() {
        let rel = if german { rel_de } else { BUILTIN_EN.get(i).unwrap_or(rel_de) };
        let p: PathBuf = rel.split('/').fold(dir.to_path_buf(), |acc, part| acc.join(part));
        if p.exists() {
            continue;
        }
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&p, src)?;
        n += 1;
    }
    Ok(n)
}

/// Markierungsdatei im Befehlsordner: eingebaute Befehle sind schon installiert.
pub const MARKER_FILE: &str = ".firepixel-commands";
/// Alter deutscher Name der Markierungsdatei.
pub const LEGACY_MARKER_FILE: &str = ".firepixel-befehle";

/// Fehlerhafte Fassungen früherer Versionen (Index in [`BUILTIN`], alter Quelltext): `setLayerVisible`/
/// `setLayerLocked` brauchen wie im Original vier Argumente, mit drei bricht das Skript ab (FV-13).
const OUTDATED: &[(usize, &str)] = &[
    (18, "var d = fw.getDocumentDOM();\nvar cur = d.currentLayerNum;\nfor (var i = 0; i < d.layers.length; i++) if (i != cur && d.layers[i].layerType != \"web\") d.setLayerVisible(i, -1, false);\n"),
    (19, "var d = fw.getDocumentDOM();\nvar cur = d.currentLayerNum;\nfor (var i = 0; i < d.layers.length; i++) if (i != cur && d.layers[i].layerType != \"web\") d.setLayerLocked(i, -1, true);\n"),
];

/// Ersetzt mitgelieferte Befehle in `dir`, die noch wortgleich in einer fehlerhaften alten Fassung
/// vorliegen (deutsche und englische Namen); vom Nutzer geänderte Dateien bleiben unberührt.
/// Liefert die Zahl der ersetzten Dateien.
pub fn update_outdated(dir: &Path) -> std::io::Result<usize> {
    let mut n = 0;
    for (i, old) in OUTDATED {
        let Some((rel_de, src)) = BUILTIN.get(*i) else { continue };
        for rel in [Some(*rel_de), BUILTIN_EN.get(*i).copied()].into_iter().flatten() {
            let p: PathBuf = rel.split('/').fold(dir.to_path_buf(), |acc, part| acc.join(part));
            if std::fs::read_to_string(&p).is_ok_and(|s| s == *old) {
                std::fs::write(&p, src)?;
                n += 1;
            }
        }
    }
    Ok(n)
}

/// Wie [`install_into`] für den Befehlsordner der Anwendung. Einmal je Ordner: eine Markierungsdatei
/// verhindert, dass gelöschte Befehle beim nächsten Start wiederkommen. Die Namen folgen der
/// aktuellen Sprache (`fp_i18n::current_language`) – deshalb erst nach der Sprachwahl aufrufen.
/// Fehlerhafte alte Fassungen mitgelieferter Befehle werden bei jedem Start ersetzt ([`update_outdated`]).
pub fn install_builtin_commands() -> std::io::Result<usize> {
    let dir = fp_engine::commands::misc::commands_dir();
    let _ = update_outdated(&dir);
    // Markierung technisch englisch (AGENTEN_REGELN §1c); die alte `.firepixel-befehle` zählt weiter
    let marker = fp_i18n::paths::migrate_file(&dir.join(MARKER_FILE), &dir.join(LEGACY_MARKER_FILE));
    if marker.exists() {
        return Ok(0);
    }
    let n = install_into_lang(&dir, &fp_i18n::current_language())?;
    std::fs::write(marker, env!("CARGO_PKG_VERSION"))?;
    Ok(n)
}
