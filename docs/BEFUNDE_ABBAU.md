# Abbau der offenen Befunde

Stand 14.10.2026 (B-1). Quelle: alle 147 offenen Einträge (`- [ ]`) in `docs\BEFUNDE.md`. Zeilenangaben „Z.“ beziehen sich auf BEFUNDE.md vor der Markierung durch B-1; seit der neuen Kopfnotiz liegen alle Einträge 2 Zeilen tiefer (Suche nach „BA-0xx“ in BEFUNDE.md findet sie direkt). Herkunft = Kürzel aus „(von …)“. Aufwand S ≤ 30 min, M ≤ 2 h, L > 2 h.

## Zählung

| Paket | Einträge (BA) | BEFUNDE-Zeilen |
|---|---|---|
| M – im Original messen | 36 | 37 |
| B3 – Leinwand/Werkzeuge | 11 | 11 |
| B4 – Dialoge/Datei-Dialoge/Gestaltung | 18 | 18 |
| B5 – Skript/Export/Web | 11 | 11 |
| B6 – Aufräumen/Tests/Doku | 20 | 22 |
| V – bewusst verschoben | 15 | 21 |
| X – nicht machbar | 5 | 7 |
| I – Info/erledigt/überholt (in BEFUNDE abgehakt) | – | 24 |

Zusammengeführt (Doppel): Z. 203 + 966 (Raster-Popup-Schieber), 241 (A1/A2) + 959 (Pipette außerhalb), 409 + 410 (nativer Dialog), 707 + 726 (Startbildschirm-Bilder), 723 + 744 (unbenutzte dlg-md-Schlüssel), 238 + 323 + 367 (Leistung), 428 = 432 und 429 = 433 (Info). Gemischte Einträge haben mehrere BA (Z. 241: M/B3/X; Z. 409: B4/X; Z. 961: M/B5).

## M – im Original messen

### M-A · Vektor-Testdokument nach PARITAET_A (R1/R2, Pfade, Gruppen; Messplan NACHPRUEFUNG FV-14, Ablauf A–E in `berichte\au3.md`) (6)

1. **BA-001** Stift am Endpunkt nach Unterauswahl-Schritten: setzt das Original fort oder legt neuen Pfad an (Pfad noch gewählt?) – Z. 110 (PM-3) – `tools (Stift)` – S
1. **BA-002** Fang beim Verschieben und beim Stift: Kante/Mitte bei 4–5 px, Stiftpunkte an Kanten – Z. 117 (FV-14) – `tools\snap.rs` – M
1. **BA-003** Alt-Ziehen aus mittleren und End-Eckpunkten (nur Ausgangshenkel?) – Z. 118 (FV-14) – `tools (Stift)` – S
1. **BA-004** Feldtext beim Klick auf ein anderes Objekt: Übernahme an R1 oder R2 (Ablauf A–E in berichte\au3.md) – Z. 175 (FV-9) – `inspector\fields.rs, inspector\mod.rs, canvas\mod.rs` – M
1. **BA-005** Verbinden mit nur einem offenen Pfad (= schließen): Name bleibt? – Z. 958 (F4) – `fp-engine` – S
1. **BA-006** Inspektor eines maskierten Bilds: Werte der Maskengruppe oder des Bildes; Freigabe der Pfadknöpfe im Werkzeug-Inspektor – Z. 965 (FIX-G5/FIX-R1) – `inspector` – S

### M-B · Bitmap-Testdokument (B1 320,200,420,300; radierte Bitmap aus FV-11; Pinselzüge nach `re\24`; Auswahlbefehle NACHPRUEFUNG §3) (8)

1. **BA-007** Treffer auf durchsichtigen Bitmap-Pixeln: Ellipsen-Bitmap-Ecke, Umschalt-/Strg-Klick, Unterauswahl, Ziehen aus dem Loch – Z. 119 (FV-14) – `tools (Treffertest)` – M
1. **BA-008** Pinsel mit gewähltem Vektor nach Strg+Z-Schritten malt in vorige Bitmap – welcher Schritt bewirkt das – Z. 120 (FV-11) – `BrushTool` – S
1. **BA-009** Radieren bis zum Rand (Objektgrenzen), Einfügen fremder Pixel, leerer Import, Skalieren an Pixelgrenzen, Beschnitt JPEG/GIF/WebP (re\22 §1.4) – Z. 127 (BM-1) – `fp-engine Import/Bitmap` – M
1. **BA-010** Pinsel: Umschalten auf natürliche Engine durch deren Werte; „Tintenmenge“ = Deckkraft (re\24 Nr. 12) – Z. 130 (PINSEL-1) – `dialogs (Pinsel)` – S
1. **BA-011** Auswahlrahmen erweitern/begrenzen: nutzt das Original die Morphologie aus re\05 §2.2 (+6 bei 5 px)? (NACHPRUEFUNG §3) – Z. 315 (PM-6) – `fp-engine Auswahl` – S
1. **BA-012** Bewegungsunschärfe Winkel 0 an B1 (Umfang 20): 300 oder 301 links – Z. 362 (DLG-6) – `blur.rs` – S
1. **BA-013** Menüfilter (z. B. Umkehren) an Bitmap mit durchsichtigem Rand: Zuschnitt auf Alpha > 0? – Z. 363 (DLG-6) – `trim_growth` – S
1. **BA-014** Filter auf Vektor: wann konvertiert das Original, wie viele Verlaufsschritte – Z. 957 (F4) – `fp-ui Filterablauf` – S

### M-C · Text und Effekte: `fx_text.fw.png` (FX-4) und Text an Pfad (TX2/P2, TX3 aus Datei, M1) (3)

1. **BA-015** Objektpuffer der Live-Effekte an Text (fx_text.fw.png: 23,46–167,112), weitere Messpunkte für die Regel – Z. 370 (FX-4) – `compose.rs` – M
1. **BA-016** Helligkeit/Kontrast „klein“ am einfarbigen Vektor: Rechenweg des Mittelwerts (109 statt 113) – Z. 371 (FX-4) – `Filter Helligkeit/Kontrast` – S
1. **BA-017** Rahmen „roh“ beim Anfügen von Text an einen Pfad (TX2/P2 vs. TX3 aus Datei) – Z. 952 (F6/M1) – `fp-script, Text an Pfad` – S

### M-D · Dialoge, Menüs und Inspektor-Popups nach PARITAET_B (Messdokument UX-B; Datei-Info nach `berichte\dlg11.md`) (11)

1. **BA-018** Bildvorschau 2-/4-fach mit Skalierung (50 %): Lage/Streckung des Exportbilds – Z. 125 (BV-5) – `crates\fp-ui\src\preview.rs` – S
1. **BA-019** Raster bearbeiten: Bereich und Kennlinie der Popup-Schieber ▾ an beiden Abständen (Annahme 1 … 100) – Z. 203 (DLG-2), Z. 966 (FIX-G1) – `dialogs\catalog.rs grid_draw` – S
1. **BA-020** Animieren mit Richtung ≠ 0; „Größe ändern“ im Inspektor bei Animationsinstanzen – Z. 224 (DLG-9) – `Animation` – S
1. **BA-021** Englische Beschriftung von D39 („Sample radius:“) und englische Verlaufsnamen der Auswahlbefehle – Z. 228 (DLG-8) – `ftl en` – S
1. **BA-022** Tab-Folge von D45, D47–D50; Fokus ins Feld nach Klemmen beim OK – Z. 233 (FORM-1) – `dialogs\form.rs` – S
1. **BA-023** Datei-Info (D10): Register, Felder je Seite, Knöpfe, Startfokus, Esc, Größenänderung (Messauftrag berichte\dlg11.md) – Z. 234 (DLG-11) – `dialogs` – M
1. **BA-024** Farb-Popup: schließt der Strichlage-Knopf das Popup; Deckkraft einer Rasterwahl = Ausgangsfarbe? – Z. 241 (FARB-1b) – `inspector\popups.rs` – S
1. **BA-025** „Als Animation öffnen“ mit Dateien unterschiedlicher Größe und mit Symbolen – Z. 419 (NZ-4/NZ-5) – `files.rs` – M
1. **BA-026** Freigaben: Bitmap-Auswahl wiederherstellen/löschen (anderes Dokument), Hilfslinien entfernen, Pfade kombinieren ▸ Teilen, Vorschau in zweitem Browser – Z. 434 (FV-13) – `command_state` – S
1. **BA-027** Lage von Volle/Halbe Höhe des Inspektors; Optimieren-Palettenbefehle bei GIF – Z. 625 (MN-3) – `panels` – S
1. **BA-028** Wahl im Exportformat des Dokument-Inspektors: legt das Original einen Verlaufsschritt an? – Z. 842 (PAR-X-INSP-1) – `inspector (Dokument)` – S

### M-E · JSF-Skripte im Original (`re\_testdateien\ux\uxa\`, Referenz `re\api\fw_api.json`) (3)

1. **BA-029** JSF: setzt das Original beim Setzen einer Textur eine Menge (textureAmount)? – Z. 353 (AU-3) – `fp-script pi.fill` – S
1. **BA-030** JSF: currentLayerNum = Unterebene – liefert das Original die Unterebene? – Z. 961 (F8) – `fp-script, fp-engine` – S
1. **BA-031** JSF addNewLayer: Rückgabe Name (laut re\api\fw_api.json) oder ID – Z. 963 (F8) – `fp-script` – S

### M-F · Export eines neuen bzw. des Testdokuments (`re\17`, `re\18`; BMP/PNG-Auflösung, HTML-Exportart) (5)

1. **BA-032** Exportassistent: Ergebnisseiten-Texte, Farbanzahl des GIF-Vorschlags (re\18 10.5) – Z. 235 (E3) – `dialogs (Exportassistent)` – S
1. **BA-033** Vorgabe-Exporteinstellung einer neuen Seite: GIF Adaptiv 256 oder PNG32 (re\17 §5.1) – Z. 308 (BV-1) – `crates\fp-export\src\doc_export.rs, JSF exportOptions` – S
1. **BA-034** re\18: Raster-Layout der Sprites (10.15), Trennzeichen „_“ (10.7), PDF-Qualitätsstufen – Z. 320 (E2) – `fp-export` – S
1. **BA-035** BMP-/pHYs-Rundung der Auflösung (CT-4) – Z. 366 (NZ-4) – `fp-export\src\formats.rs, fp-fwpng\src\png.rs` – S
1. **BA-036** Merkt das Original die zuletzt gewählte HTML-Exportart auch für neue Dokumente? – Z. 967 (FIX-G2) – `Export HTML` – S

## B3 – Leinwand/Werkzeuge

1. **BA-037** KI-Objektauswahl: Vorschau der Objektmaske beim Überfahren (optional, Bonus) – Z. 31 (KI) – `canvas\*, fp_engine::commands::ai` – M
1. **BA-038** Esc mitten im Pinselzug wie im Original (Zug behalten, Zeiger, Schritt „Bitmapmodus beenden“) – Z. 128 (PINSEL-1) – `BrushTool, canvas\paint_preview.rs` – S
1. **BA-039** Pinsel-Restabweichungen: Vorgabe nach Neustart, Werkzeugwerte merken, Vorschau in Stapelfolge mit Deckkraft/Mischmodus, Bézier-Anpassung, gesperrte Ebene, +1/+1-Versatz (Druck/Geschwindigkeit der Maus nicht machbar) – Z. 129 (PINSEL-1) – `BrushTool, paint_preview.rs` – L
1. **BA-040** Weichzeichnen/Schärfen: Gauß r > 200 auf Alpha-Rechteck beschränken, Wachstum je Durchgang am Text; FX-3-Gegenprüfung – Z. 372 (FX-4/FX-T) – `compose.rs, blur.rs` – M
1. **BA-041** Stempel: Pixelwerte im Zug (Spitzenrasterung wie tool-retouch/13–14) – Z. 817 (PAR-F) – `Stempel-Werkzeug` – M
1. **BA-042** Pipette 3×3/5×5: Original nimmt linkes oberes Pixel – Nutzerentscheidung, ob nachbauen – Z. 838 (PAR-F) – `Pipette` – S
1. **BA-043** Verlaufs-/Mustergriffe auf der Leinwand ziehbar machen (ein Schritt) – Z. 841 (PAR-X-INSP-1) – `canvas\mod.rs fill_handles` – M
1. **BA-044** Innen einfügen: Pfad mit Vektormaske statt Maskengruppe (Objektmodell); „Als Maske einfügen“ Auswahl danach – Z. 951 (M1/F4) – `fp-engine commands\edit.rs` – L
1. **BA-045** Gamma 1.8: Schachbrett und Ansichten 2-/4-fach mit aufhellen – Z. 953 (F3) – `fp-ui canvas\render.rs` – S
1. **BA-046** Symbolbearbeitung im selben Dokument statt eigener Ansicht (Verlauf, Ebenen, Skript-API, Abblenden) – Z. 970 (F11) – `fp-engine commands\symbol_edit.rs, fp-ui canvas` – L
1. **BA-047** Farbwahl A3 (Leinwand-Pipette transparent → Weiß) und A4 (Werkzeugleiste Verlauf/Muster) bewusst entscheiden – Z. 241 (FARB-1b) – `inspector\popups.rs, tools_panel.rs` – S

## B4 – Dialoge/Datei-Dialoge/Gestaltung

1. **BA-048** Zoom-Rahmen in der Vorschaukachel gestrichelt wie auf der Leinwand – Z. 124 (BV-5) – `crates\fp-ui\src\preview.rs` – S
1. **BA-049** Tastaturkurzbefehle Rest: Symbolknöpfe beim Satz, Fokus/Pfeiltasten im Menübaum, „Befehle:“ als Kombifeld – Z. 227 (DLG-4) – `dialogs (Kürzel)` – M
1. **BA-050** Bitmap-Auswahl wiederherstellen: Live-Anzeige der entstehenden Auswahl – Z. 229 (KV-3) – `dialogs (bitmap_selection)` – M
1. **BA-051** Optimieren ▸ Einstellungen löschen: gewählte Einstellung direkt nach Rückfrage löschen – Z. 230 (MN-5) – `panels\optimize.rs, dialogs\panel_menus.rs` – S
1. **BA-052** Tab-Folge der Live-Effekt-Dialoge (Farbton/Sättigung, Helligkeit/Kontrast, Unscharf maskieren) wie im Menüdialog – Z. 231 (FORM-1) – `dialogs\effects.rs effect_form` – S
1. **BA-053** „Neues Symbol“ im Bibliothek-Bedienfeld wie D14 (9-Slice, Bibliothek-Kästchen) – Z. 232 (FORM-1) – `panel.library.newSymbol` – M
1. **BA-054** Exportdialog meldet „exportiert“ (Bildvorschau danach schließen, falls Messung re\17 §11.2 das ergibt) – Z. 236 (BV-4) – `dialogs\export.rs` – S
1. **BA-055** Pinsel-Inspektor: Knopfbreite DE, senkrechte Popup-Schieber 24 × 222, Mischmodus-Liste (56 Einträge) – Z. 242 (PINSEL-1) – `inspector` – M
1. **BA-056** Namen der Abschrägungs-Familie und Feld „Art“ – Nutzerentscheidung – Z. 244 (FX-2) – `Effekte, Glossar` – S
1. **BA-057** Verlaufsnamen der Pfadbefehle „Pfade erweitern“/„Pfade einlassen“ – Z. 316 (PM-6) – `command_texts.ftl` – S
1. **BA-058** Knopf „Optionen…“ im Exportdialog mit Accessibility-Namen – Z. 321 (E2) – `dialogs\export.rs` – S
1. **BA-059** shortcuts.json gegen PARITAET_B §1 abgleichen (gemessene Werte gelten) – Z. 322 (MN-5) – `fp-engine\data\shortcuts.json` – S
1. **BA-060** App-eigener Datei-Dialog: Vorschau und „XMP einschließen“ nachrüsten – Z. 409 (DLG-3) – `file_dialog.rs show_modal, files.rs` – M
1. **BA-061** Schrift-Menü: Schriften aus Inspektor/Texteditor in „zuletzt benutzt“ eintragen – Z. 492 (UX-B) – `menu.rs` – S
1. **BA-062** KI-Modelle verwalten: Modellspalte breiter oder „…“ mit Tooltip; Nachprüfung qps – Z. 522 (I18N-QA) – `dialogs (KI)` – S
1. **BA-063** Dialoge D18–D23 Rest: D18 Zusatzkästchen, D19 Druckgröße/Ketten, D20 Pfeilsymbole + B:/H:, D21 Anordnung, D22 Selektives JPEG, D23 Textgröße als Kombifeld; Zugriffstasten in allen Dialogen – Z. 532 (UX-B) – `dialogs\*` – L
1. **BA-064** Gestaltung Rest DES-1: Voreinstellungen Fußleiste, DESIGN §13 Prio 5–11, Knopfbreiten zentral – Z. 706 (DES-1) – `dialogs, ui_kit` – M
1. **BA-065** Startseite: Eintrag „Batch“ – Z. 753 (BATCH-9) – `welcome` – S

## B5 – Skript/Export/Web

1. **BA-066** JSF addNewHotspot("slice") → Segment; addNewText(rect, true) mit Voreinstellungen (Anti-Alias glatt, Unterschneidung aus, 120 %) – Z. 340 (UX-A) – `crates\fp-script\src\imp\doc.rs` – M
1. **BA-067** JSF pixelRect eines Texts = Glyphengrenzen (nach FX-T prüfen) – Z. 347 (PM-3) – `crates\fp-script\src\imp\elem.rs` – S
1. **BA-068** JSF fw.setActiveViewScale an die Ansicht anbinden (heute No-op) – Z. 348 (PM-3) – `crates\fp-script\src\imp\fw.rs:323` – S
1. **BA-069** JSF-Vorgabe Schlagschatten #000000a5 vs. Menü-Vorgabe #000000a2 – Z. 354 (FX-1) – `crates\fp-script\src\builtin_commands.rs` – S
1. **BA-070** Batch effect.live: Abstand/Weichheit in der Vorschau über StepContext::preview_scale umrechnen (Feld vorhanden) – Z. 738 (BATCH-4/BATCH-6) – `crates\fp-batch\src\step.rs, steps` – S
1. **BA-071** HTML-Export mit Schaltflächen-Segmenten (TW-19) prüfen – Z. 954 (F3) – `fp-export HTML` – M
1. **BA-072** TW-19: aktive Fläche aus dem Schaltflächensymbol auswerten – Z. 955 (F3) – `fp-export / Segmente` – S
1. **BA-073** JSF: undefined statt null (name, d.zoom, docTitle, brush.diameter) – Z. 956 (F4/FIX-R2) – `fp-script Wertumwandlung` – S
1. **BA-074** Unterebene als aktive Ebene; addNewSubLayer verschachtelt – Z. 961 (F8) – `fp-engine, fp-script` – M
1. **BA-075** reorderLayer/duplicateLayer für Unterebenen (Engine-Befehl fehlt) – Z. 962 (F8) – `fp-script, fp-engine` – M
1. **BA-076** SVG-Import: Stapelfolge von Text und Pfaden – Z. 964 (F2) – `fp-io vector.rs` – M

## B6 – Aufräumen/Tests/Doku

1. **BA-077** new_document.rs: eigene underline_access auf dialogs::access umstellen – Z. 237 (NZ-3) – `crates\fp-ui\src\dialogs\new_document.rs:171` – S
1. **BA-078** Unbenutzte Schlüssel pi-color-pipette/pi-color-transparent entfernen (kein .rs-Treffer) – Z. 239 (FARB-1b) – `inspector.ftl de/en` – S
1. **BA-079** Altpfad ColorChoice::Pipette / popups::start_pick prüfen und entfernen – Z. 240 (FARB-1b) – `crates\fp-ui\src\inspector\popups.rs:83` – S
1. **BA-080** misc::stroke_options_form unbenutzt – entfernen – Z. 243 (PINSEL-1) – `crates\fp-ui\src\dialogs\misc.rs:208` – S
1. **BA-081** Unbenutzte dlg-md-*-Schlüssel (Ziel/Größe/Batch, 14 + 4 je Sprache) entfernen – Z. 723 (E3), Z. 744 (BATCH-2) – `crates\fp-i18n\locales\*\dialogs.ftl` – S
1. **BA-082** Gesamttests: file.batchProcess öffnet das Studio – ausnehmen; Erlaubnis im clipping_audit streichen (prüfen) – Z. 745 (BATCH-2) – `fp-ui tests` – S
1. **BA-083** Golden-Hashes des Resamplers auf anderen OS (CI) neu erzeugen – Z. 746 (BATCH-4) – `fp-batch Tests` – S
1. **BA-084** AGENTS.md/AGENTS.en.md: Abschnitt „Video über ffmpeg“ (FIREPIXEL_FFMPEG) – Z. 750 (BATCH-7) – `docs\AGENTS*.md` – S
1. **BA-085** DESIGN.md §12 Startbildschirm; design_willkommen schreibt design_after_* statt design_nachher_*; Bilder neu – Z. 707 (DES-2), Z. 726 (TE2-E) – `docs\DESIGN.md, crates\fp-ui\tests\design_willkommen.rs` – S
1. **BA-086** Deutsche QA-Bereichs-IDs/Ordner in i18n_qa_* englisch (alte Ergebnisse weiter lesen) – Koordinator – Z. 725 (TE2-A/TE2-E) – `crates\fp-ui\tests\i18n_qa_*` – M
1. **BA-087** Automatische Berichte (I18N_PRUEFUNG, funktionstest\*_auto) mit neuen Ordner-/ftl-Namen neu erzeugen – Z. 719 (TE-1) – `docs` – S
1. **BA-088** gesamt_menu: Sprachwechsel prozessweit – Sprache zurücksetzen oder seriell (prüfen) – Z. 413 (FV-13) – `crates\fp-ui\tests\gesamt_menu.rs` – S
1. **BA-089** kittest/wgpu: gemeinsame Sperre um render() gegen Zugriffsverletzung im Programm all – Z. 416 (NZ-2) – `fp-ui tests` – M
1. **BA-090** Wackelnde Tests width_field_limited_to_6000_with_message, batch_ui::trockenlauf_und_lauf_zweimal – Z. 968 (GT-0f/FIX-F) – `fp-ui tests` – M
1. **BA-091** Runner-Lücken tools\parity\ (ui_map-Schlüssel, repeat, owner-drawn Untermenüs, Drehbücher) – Z. 969 (RUN-2/RUN-3/RUN-4/A3-2) – `tools\parity\` – L
1. **BA-092** kittest-Bildserien/Paarbilder mit Text nach Hinting neu erzeugen – Z. 374 (FX-T) – `fp-ui tests` – S
1. **BA-093** Korpus-Pixeltests (--ignored) nach InkBuffer::stamp-Änderung laufen lassen – Z. 375 (PINSEL-1) – `fp-vector` – S
1. **BA-094** „Befehle ▸ Demo - aktuelles Dokument“ im echten Fenster; judge meldet Fremdprogramm-Start – Z. 420 (MN-4) – `gesamt_common::judge` – S
1. **BA-095** winit focus_window() beim normalen Start nötig? (simulierter Alt-Druck) – Z. 421 (START-1) – `crates\fp-ui\src\start_window.rs:148` – S
1. **BA-096** bridge.rs: Bildschirmfoto-Zuordnung eindeutig machen – Z. 425 (FARB-1b) – `bridge.rs` – S

## V – bewusst verschoben

1. **BA-097** Leistung ohne GPU: erste Ziehbewegung großer Objekte (Vorschau-Textur auf Ausschnitt begrenzen) – Z. 121 (FV-8) – `canvas\ghost.rs drag_sprite` – M
1. **BA-098** Leistung ohne GPU: erstes Bild nach Zoom/Bildlauf (Teil-Upload) – Z. 122 (FV-8) – `canvas\mod.rs` – M
1. **BA-099** Leistung mit 3200 Objekten: Inspektor/Ebenen ≈ 45 ms, command_state bis 5 ms, Render-Cache nach Verschieben – Z. 238 (PERF-1), Z. 323 (PERF-1), Z. 367 (PERF-1) – `inspector, panels\layers, command_state, edit_at` – M
1. **BA-100** PDF „Textauswahl aktivieren“ (unsichtbarer Text) – Z. 317 (E2) – `fp-export PDF` – M
1. **BA-101** PDF JPEG 2000 (kein Kodierer) – Z. 318 (E2) – `fp-export PDF` – L
1. **BA-102** PSD-Export: Effekte/Vektor/Text, Ebene je Objekt – Z. 319 (E2) – `fp-export PSD` – L
1. **BA-103** HTML-Einrichtung: Standardeinstellungen, externe CSS, Navigationsleisten-HTML – Z. 324 (E3) – `fp-export HTML` – M
1. **BA-104** CMD-1: Bildschirmfoto, Symbolskript erstellen, Gemeinsame Palette als echte Funktion – Z. 351 (MN-4) – `fp-script\src\builtin_commands.rs` – L
1. **BA-105** Batch-Ausbau Kern: {w}x{h} nach resize, mehrere Ausgaben je Element, video.import-Parameter/Streaming – Z. 737 (BATCH-1/BATCH-4), Z. 740 (BATCH-7/BATCH-9), Z. 741 (BATCH-7) – `fp-batch` – L
1. **BA-106** Batch-Ausbau: Strg+C-Abbruch in der CLI, Rückfrage beim Schließen des Fensters – Z. 739 (BATCH-1), Z. 742 (BATCH-2) – `fp-batch, actions.rs` – S
1. **BA-107** Batch-Studio-Oberfläche: Filter, Sortieren, Fortschritt je Datei, Felder je Modus, GIF-Felder, ffmpeg-Zeile – Z. 743 (BATCH-2/7/8/9) – `fp-ui Batch` – L
1. **BA-108** Batch-Formate: WebP/AVIF alphaQuality/method, Metadaten AVIF/JXL/HEIF/TIFF, GPS/IPTC, Farbprofil – Z. 747 (BATCH-5), Z. 748 (BATCH-5) – `fp-export, fp-batch` – M
1. **BA-109** Batch-Schritte SPEC §3.5/§3.6 (subjectCenter, alphaCleanup, vignette, contactSheet) – Z. 749 (BATCH-6) – `fp-batch steps` – M
1. **BA-110** GIF-Optimierer −30 %, Parallelisierung, Abbrechen in der Kodierung – Z. 751 (BATCH-8) – `fp-batch GIF` – L
1. **BA-111** DX12 ohne Rückfall auf Vulkan/GL – nur auf Rechnern mit Grafikkarte beobachtbar – Z. 423 (FIX-A) – `wgpu-Start` – S

## X – nicht machbar

1. **BA-112** Native Datei-Dialoge mittig platzieren – nur per Win32/unsafe (Workspace unsafe_code = forbid) – Z. 408 (DLG-3) – `file_dialog.rs spawn_native` – S
1. **BA-113** Nativer Dialog: Zusatzfelder (IFileDialogCustomize = unsafe) und Filter-Vorwahl (rfd kann es nicht) – Z. 409 (DLG-3), Z. 410 (DLG-3) – `file_dialog.rs` – S
1. **BA-114** Systemfarbwähler (A1) und Pipette außerhalb des Fensters (A2) – nur mit unsafe bzw. Zusatzabhängigkeit – Z. 241 (FARB-1b), Z. 959 (F5) – `inspector\popups.rs` – S
1. **BA-115** XMP-Standardkennung enthält den Herstellernamen – Neutralitätsregel §1a verbietet es – Z. 447 (DLG-10) – `fp-fwpng XMP` – S
1. **BA-116** CFF-Schrift (andere Schriftfassung unter Windows) bleibt bei 52–72 % exakten Pixeln; gedrehter Text ungehintet – Z. 373 (FX-T) – `fp-text` – S

## I – abgehakt (Info/erledigt/überholt)

- Z. 104 (FV-6): Info/Messhinweis für Skripte, in AGENTS dokumentierbar; kein Fehler mit echter Maus
- Z. 274 (KI): Info „nichts zu tun“ (Einhängung KI-Befehle)
- Z. 399 (AU): Info: ui_input/ui_find umgesetzt, Anleitung docs\AGENTS.md §5a
- Z. 401 (VT-2): zur Kenntnis: Einhängung Nachzeichnen-Dialog
- Z. 403 (VT-2): zur Kenntnis: Inspektor Q5 Nachzeichnen
- Z. 404 (VT-1): zur Kenntnis: fp-trace im Workspace
- Z. 405 (FV-2): zur Kenntnis: N09 umgesetzt (FV-2)
- Z. 414 (FV-13): ui.toolKey umgesetzt (crates\fp-ui\src\canvas\context.rs:112/213)
- Z. 424 (START-1): Info Startzeit; START-1/GT-2 gemessen (gt2.md: Start 933/976 ms)
- Z. 428 (AU): Info, doppelt mit 432 (ui_input/ui_find, AGENTS §5a)
- Z. 429 (AU): Info, doppelt mit 433 (Datei-Dialoge blockieren nicht, AGENTS §5b)
- Z. 432 (AU): Info, zusammengeführt mit 428
- Z. 433 (AU): Info, zusammengeführt mit 429
- Z. 448 (NEUTRAL-1): Vorgehenshinweis für neue Bilder (wie FV-12), kein Codefehler
- Z. 521 (I18N-MENUE): erledigt: effects.rs:44 nimmt clean_label(command_key(cmd))
- Z. 523 (IBD): Abstimmungsnotiz IBD, überholt
- Z. 705 (UE-1): erledigt: ui_kit.rs:339 meldet den Hinweistext per widget_info
- Z. 712 (VT-4): erledigt: commands\mod.rs:30 pub mod trace
- Z. 720 (TE-1): Regel zur Kenntnis (§1c, Wächter technical_english)
- Z. 721 (TE-1): zur Kenntnis: Migrationsänderungen TE-1
- Z. 736 (BATCH-1): erledigt: legacy_area_names.rs:32 prüft ≥ 18
- Z. 755 (BATCH-9): Info Plattenplatz, überholt
- Z. 882 (CT-1b): erledigt: F12 0608eea (Texte ohne fremden Produktnamen)
- Z. 894 (CT-2): erledigt: Strich der Formwerkzeuge seit a5ed533 (berichte\fixg5.md)
