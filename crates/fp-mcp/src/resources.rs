//! Eingebaute Doku als MCP-Ressourcen (Markdown).

use fp_engine::CommandRegistry;
use std::fmt::Write as _;

/// Eine Ressource.
#[derive(Clone, Debug)]
pub struct DocResource {
    pub uri: &'static str,
    pub name: &'static str,
    pub title: &'static str,
    pub description: &'static str,
}

/// Alle Ressourcen.
pub fn resources() -> Vec<DocResource> {
    vec![
        DocResource { uri: "firepixel://docs/agents.md", name: "agents", title: "Anleitung für KI-Agenten", description: "Einrichtung, typische Abläufe, Befehlsnamen-Schema, Beispiele" },
        DocResource { uri: "firepixel://docs/commands.md", name: "commands", title: "Befehlskatalog", description: "Alle Befehle nach Kategorie mit Parametern, Pflichtangaben und Verfügbarkeit" },
        DocResource { uri: "firepixel://docs/document-model.md", name: "document-model", title: "Dokumentmodell", description: "Seiten, Status, Ebenen, Elemente, IDs, Koordinaten, Farben" },
        DocResource { uri: "firepixel://docs/examples.md", name: "examples", title: "Beispiele", description: "Fertige Werkzeugaufrufe für häufige Aufgaben" },
        DocResource { uri: "firepixel://docs/jsf.md", name: "jsf", title: "JSF-Kompatibilität", description: "Welche Teile der JSF-JavaScript-API umgesetzt sind" },
    ]
}

/// Inhalt einer Ressource.
pub fn read(uri: &str, registry: &CommandRegistry) -> Option<String> {
    match uri {
        "firepixel://docs/agents.md" => Some(AGENTS_MD.to_string()),
        "firepixel://docs/commands.md" => Some(commands_markdown(registry)),
        "firepixel://docs/document-model.md" => Some(DOCUMENT_MODEL_MD.to_string()),
        "firepixel://docs/examples.md" => Some(EXAMPLES_MD.to_string()),
        "firepixel://docs/jsf.md" => Some(fp_script::coverage_markdown()),
        _ => None,
    }
}

/// Anleitung für Agenten (aus `docs/AGENTS.md`).
pub const AGENTS_MD: &str = include_str!("../../../docs/AGENTS.md");

/// Befehlskatalog als Markdown.
pub fn commands_markdown(reg: &CommandRegistry) -> String {
    let mut s = String::new();
    let implemented = reg.iter().filter(|c| c.info().implemented).count();
    let _ = writeln!(s, "# Firepixel-Befehlskatalog\n");
    let _ = writeln!(
        s,
        "{} Befehle, davon {} mit Wirkung. Aufruf über MCP `run_command` ({{\"name\": …, \"params\": {{…}}}}), CLI `firepixel-cli run --cmd …` oder JSF. \
`*` = Pflichtparameter. Platzhalter ohne Wirkung sind mit _(nicht umgesetzt)_ markiert.\n",
        reg.len(),
        implemented
    );
    let mut cats: Vec<String> = reg.iter().map(|c| c.info().name.split('.').next().unwrap_or("").to_string()).collect();
    cats.sort();
    cats.dedup();
    for cat in cats {
        let _ = writeln!(s, "## {cat}\n");
        let mut items: Vec<_> = reg.iter().map(|c| c.info()).filter(|i| i.name.split('.').next() == Some(cat.as_str())).collect();
        items.sort_by(|a, b| a.name.cmp(&b.name));
        for i in items {
            let params: Vec<String> = i
                .params
                .iter()
                .map(|p| {
                    let ty = match &p.kind {
                        fp_engine::ParamKind::Enum { values } => values.join("|"),
                        k => format!("{k:?}").split([' ', '{']).next().unwrap_or("").to_lowercase(),
                    };
                    format!("`{}{}`: {}", p.name, if p.required { "*" } else { "" }, ty)
                })
                .collect();
            let reqs: Vec<String> = i.requires.iter().filter(|r| !matches!(r, fp_engine::Requirement::Document)).map(|r| r.reason()).collect();
            let _ = write!(s, "- **`{}`** – {}", i.name, i.label);
            if !i.implemented {
                let _ = write!(s, " _(nicht umgesetzt)_");
            }
            if !params.is_empty() {
                let _ = write!(s, " · {}", params.join(", "));
            }
            if !reqs.is_empty() {
                let _ = write!(s, " · braucht: {}", reqs.join("; "));
            }
            let _ = writeln!(s);
        }
        let _ = writeln!(s);
    }
    s
}

/// Überblick Dokumentmodell.
pub const DOCUMENT_MODEL_MD: &str = r##"# Firepixel-Dokumentmodell (Kurzfassung für Agenten)

Firepixel bildet das Dokumentmodell des Originals nach. Dateien sind FW-PNGs (`.fw.png`):
ein normales PNG (flaches Bild) mit eingebetteten, editierbaren Daten.

## Aufbau

```
Dokument
└─ Seiten (pages[], eine aktuelle Seite; höchstens eine Masterseite)
   ├─ Leinwand: Breite, Höhe, Ursprung, Auflösung, Leinwandfarbe
   ├─ Status (states, im Original auch „Frames“; für Animation/Rollover)
   └─ Ebenen (layers[], Reihenfolge unten → oben; die Web-Ebene ist die letzte)
      └─ je Status ein Inhalt (bei „über Status geteilten“ Ebenen nur einer)
         └─ Elemente oben → unten
            ├─ rectangle   Rechteck-Grundform (Eckenrundung)
            ├─ path        Vektorpfad (Konturen aus Bézier-Knoten)
            ├─ text        Text (Punkt- oder Rahmentext, Läufe mit Schrift/Größe/Farbe)
            ├─ bitmap      Pixelbild
            ├─ group       Gruppe (children); maskGroup: erstes Element ist die Maske
            ├─ autoShape   Auto-Form (Gruppe mit Skript und Steuerpunkten)
            ├─ compoundShape  zusammengesetzte Form (boolesche Operanden)
            ├─ instance    Symbolinstanz (Grafik, Schaltfläche, Animation)
            └─ slice / hotspot  Web-Objekte (nur auf der Web-Ebene)
```

## IDs und Auswahl

- Jedes Element hat eine **ID** (Ganzzahl), die während der Sitzung stabil bleibt (auch nach Verschieben,
  Gruppieren der Nachbarn, Rückgängig). Neue Objekte liefern ihre ID im Ergebnis von `run_command`.
- Ebenen haben ebenfalls IDs (`layer`-Parameter der `layer.*`-Befehle).
- Die meisten Befehle wirken auf die **Auswahl** (`select`, `select.set`). Einige nehmen `ids` direkt.
- Nach einem Befehl liefert `run_command` die neue Auswahl mit Grenzen zurück.

## Koordinaten und Werte

- Einheit: Dokumentpixel, Ursprung oben links der Leinwand (Leinwandursprung kann ≠ 0 sein).
- Rechteck: `[links, oben, rechts, unten]` oder `{left, top, right, bottom}` oder `{x, y, width, height}`.
- Punkt: `[x, y]` oder `{x, y}`.
- Farbe: `"#rrggbb"` oder `"#rrggbbaa"` (Alpha zuletzt), `"none"` = transparent.
- Deckkraft: Prozent 0–100. Mischmodi: `normal`, `multiply`, `screen`, `overlay`, `darken`, `lighten`, …
- Grenzen in `get_document` sind die sichtbaren Grenzen (inkl. Strich), gerundet auf 0,01 px.

## Verlauf

- Jeder Dokument- oder Auswahlbefehl ist ein Rückgängig-Schritt (`undo`, `redo`, `history`).
- `run_command` mit `steps` fasst mehrere Befehle zu einem Schritt zusammen.
- Ansichtsbefehle (Zoom, Raster) erzeugen keinen Schritt.

## Status und Seiten

- `page.setCurrent {index}`, `state.setCurrent {index}` wechseln Seite/Status; `get_document` zeigt
  `currentPage`/`currentState`. `render` und `get_document` nehmen `page`/`state` auch direkt.
"##;

/// Beispiele.
pub const EXAMPLES_MD: &str = r##"# Beispiele (MCP-Werkzeugaufrufe)

## Neues Banner mit Text, rendern, exportieren

1. `new_document {"width": 600, "height": 200, "background": "#ffffff"}`
2. `run_command {"name": "draw.rectangle", "params": {"rect": [0, 0, 600, 200], "fillColor": "#1e3a8a", "strokeWidth": 0, "name": "Hintergrund"}}`
3. `run_command {"name": "draw.text", "params": {"text": "Sommerangebot", "anchor": [40, 120], "size": 48, "color": "#ffffff"}}`
4. `render {"zoom": 0.5}` – Ergebnis ansehen
5. `export {"path": "C:\\out\\banner.png"}` und `save {"path": "C:\\out\\banner.fw.png"}`

## Objekte ausrichten

1. `get_document {"depth": 1}` – IDs suchen
2. `select {"ids": [12, 15, 19]}`
3. `run_command {"name": "modify.align.left"}` dann `run_command {"name": "modify.align.distributeHeights"}`

## Schlagschatten und Weichzeichner

- `run_command {"name": "element.addEffect", "params": {"effect": "dropShadow", "params": {"ShadowDistance": "7", "ShadowColor": "#000000"}}}`
- `run_command {"name": "filter.blur.gaussian", "params": {"radius": 2.5}}` (auf Bitmaps destruktiv, auf Vektoren als Live-Effekt)

## Zeichnen wie ein Mensch

- `tool_event {"tool": "ellipse", "drag": {"from": [50, 50], "to": [150, 120], "shift": true}}` – Kreis
- `tool_event {"tool": "pen", "events": [{"type":"down","x":10,"y":10},{"type":"up","x":10,"y":10},{"type":"down","x":100,"y":10},{"type":"move","x":140,"y":60},{"type":"up","x":140,"y":60},{"type":"key","key":"enter"}]}` – Pfad mit Kurve

## Mehrere Schritte als ein Rückgängig-Schritt

`run_command {"steps": [{"name": "select.all"}, {"name": "modify.group"}, {"name": "modify.transform.flipHorizontal"}], "label": "Spiegeln"}`

## Ebenen

- `run_command {"name": "layer.add", "params": {"name": "Hintergrund"}}` → neue aktive Ebene
- `run_command {"name": "layer.setVisible", "params": {"layer": 7, "visible": false}}`

## KI-Auswahl (Modelle lokal, siehe `ai.models`)

- `run_command {"name": "ai.models"}` – welche Modelle da sind (`available`), Ordner, Download-Ziel
- `run_command {"name": "select.aiObject", "params": {"point": [220, 140]}}` – Objekt unter dem Punkt als Pixelauswahl; Ergebnis `{session, iou, objectScore, bounds, source}`
- `run_command {"name": "select.aiObject", "params": {"points": [[260, 150, 0]], "session": 3, "refine": true}}` – dieselbe Auswahl verfeinern (Label 0 = Hintergrund, 1 = Objekt)
- `run_command {"name": "select.aiObject", "params": {"box": [180, 90, 320, 220], "model": "sam2Small"}}` – Objekt im Rahmen
- `run_command {"name": "select.aiSubject", "params": {"source": "page"}}` – Hauptmotiv der sichtbaren Seite
- `run_command {"name": "select.aiRefineEdge", "params": {"output": "mask"}}` – Kante (Haare, Fell) verfeinern, Ergebnis als Bitmapmaske (vorher Pixelauswahl nötig)

## JSF-JavaScript

`run_script {"code": "var d = fw.getDocumentDOM(); d.addNewRectangle({left:0, top:0, right:100, bottom:50}, 0); d.setFillColor('#ff8800'); fw.selection.length"}`
"##;
