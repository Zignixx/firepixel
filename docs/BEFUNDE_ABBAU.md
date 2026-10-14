# Abbau der offenen Befunde

Stand 14.10.2026 spät (B-8, Endstand siehe unten). Ursprünglich B-1. Quelle: alle 147 offenen Einträge (`- [ ]`) in `docs\BEFUNDE.md`. Zeilenangaben „Z.“ beziehen sich auf BEFUNDE.md vor der Markierung durch B-1; seit der neuen Kopfnotiz liegen alle Einträge 2 Zeilen tiefer (Suche nach „BA-0xx“ in BEFUNDE.md findet sie direkt). Herkunft = Kürzel aus „(von …)“. Aufwand S ≤ 30 min, M ≤ 2 h, L > 2 h.

## Endstand nach dem Befunde-Abbau (B-8, 14.10.2026 spät)

Gezählt je BA nach dem letzten Ergebnis (B-2 … B-9). Teilweise erledigte BA zählen nach ihrem offenen Rest. „Mess-Sitzung“ = Messung im Original über die GUI bzw. der nächste Gesamt-/Bildlauf – beides nur mit Freigabe des Nutzers (erteilt 14.10. 21:12). Neu aus dem Abbau: BA-117 … BA-128 (Abschnitt N).

| Paket | BA | erledigt | wartet auf Mess-Sitzung (freigegeben) | entschieden, Umsetzung offen | verschoben V | nicht machbar X |
|---|---|---|---|---|---|---|
| M – im Original messen | 36 | 7 (005, 011, 016, 029, 030, 031, 033) | 29 (davon 23 = X von B-2; teilweise erledigt: 009, 013, 014, 017, 020, 035) | – | – | – |
| B3 – Leinwand/Werkzeuge | 11 | 7 (038, 040, 041, 042 und 047 bewusst anders, 043, 045) | 1 (039 Rest) | – | 3 (037, 044, 046) | – |
| B4 – Dialoge/Datei-Dialoge/Gestaltung | 18 | 13 (048–053, 057–062, 065) | 2 (054 Rest, 055 Rest) | 1 (056 Effektnamen wie im Original) | 2 (063 Zugriffstasten, 064 Rest → DES-3) | – |
| B5 – Skript/Export/Web | 11 | 11 | – | – | – | – |
| B6 – Aufräumen/Tests/Doku | 20 | 14 | 2 (087, 092: nächster Gesamt-/Bildlauf) | – | 4 (083 CI, 086 Entscheid Koordinator, 089, 091 L) | – |
| V – bewusst verschoben | 15 | – | – | – | 15 | – |
| X – nicht machbar | 5 | – | – | – | – | 5 |
| N – neu aus dem Abbau | 12 | – | 7 (119, 122, 123, 124, 126, 127, 128) | – | 5 (117, 118, 120, 121, 125) | – |
| **Summe** | **128** | **52** | **41** | **1** | **29** | **5** |

**Offen gesamt: 75 BA** (41 Mess-Sitzung + 29 verschoben + 5 nicht machbar; BA-056 von F15 umgesetzt). Nutzerentscheid 14.10. 21:12: BA-047 geschlossen (A3 bewusst anders, A4 → BA-128), BA-056 entschieden, Mess-Sitzung freigegeben (die 23 X-Punkte von B-2 und die übrigen Messpunkte). In `BEFUNDE.md`: 614 erledigt, 81 offen (70 + 11 neue offene Einträge aus dem Abbau; dazu 1 erledigter).

## Zählung bei B-1

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

Ergebnisse B-2 (14.10.): `docs\berichte\b2.md` (gleich 3, Unterschied 10, X 23).

### M-A · Vektor-Testdokument nach PARITAET_A (R1/R2, Pfade, Gruppen; Messplan NACHPRUEFUNG FV-14, Ablauf A–E in `berichte\au3.md`) (6)

1. **BA-001** Stift am Endpunkt nach Unterauswahl-Schritten: setzt das Original fort oder legt neuen Pfad an (Pfad noch gewählt?) – Z. 110 (PM-3) – `tools (Stift)` – S – **Ergebnis B-2:** X (GUI-Sitzung gesperrt)
1. **BA-002** Fang beim Verschieben und beim Stift: Kante/Mitte bei 4–5 px, Stiftpunkte an Kanten – Z. 117 (FV-14) – `tools\snap.rs` – M – **Ergebnis B-2:** X (GUI-Sitzung gesperrt)
1. **BA-003** Alt-Ziehen aus mittleren und End-Eckpunkten (nur Ausgangshenkel?) – Z. 118 (FV-14) – `tools (Stift)` – S – **Ergebnis B-2:** X (GUI-Sitzung gesperrt)
1. **BA-004** Feldtext beim Klick auf ein anderes Objekt: Übernahme an R1 oder R2 (Ablauf A–E in berichte\au3.md) – Z. 175 (FV-9) – `inspector\fields.rs, inspector\mod.rs, canvas\mod.rs` – M – **Ergebnis B-2:** X (GUI-Sitzung gesperrt)
1. **BA-005** Verbinden mit nur einem offenen Pfad (= schließen): Name bleibt? – Z. 958 (F4) – `fp-engine` – S – **Ergebnis B-2:** Unterschied: Original Name nach Verbinden immer null → B3 – **Ergebnis B-3:** erledigt (B-3, 220459c) – Verbinden ohne Namen, geschlossener Einzelpfad kein Fehler
1. **BA-006** Inspektor eines maskierten Bilds: Werte der Maskengruppe oder des Bildes; Freigabe der Pfadknöpfe im Werkzeug-Inspektor – Z. 965 (FIX-G5/FIX-R1) – `inspector` – S – **Ergebnis B-2:** X (GUI-Sitzung gesperrt)

### M-B · Bitmap-Testdokument (B1 320,200,420,300; radierte Bitmap aus FV-11; Pinselzüge nach `re\24`; Auswahlbefehle NACHPRUEFUNG §3) (8)

1. **BA-007** Treffer auf durchsichtigen Bitmap-Pixeln: Ellipsen-Bitmap-Ecke, Umschalt-/Strg-Klick, Unterauswahl, Ziehen aus dem Loch – Z. 119 (FV-14) – `tools (Treffertest)` – M – **Ergebnis B-2:** X (GUI-Sitzung gesperrt)
1. **BA-008** Pinsel mit gewähltem Vektor nach Strg+Z-Schritten malt in vorige Bitmap – welcher Schritt bewirkt das – Z. 120 (FV-11) – `BrushTool` – S – **Ergebnis B-2:** X (GUI-Sitzung gesperrt)
1. **BA-009** Radieren bis zum Rand (Objektgrenzen), Einfügen fremder Pixel, leerer Import, Skalieren an Pixelgrenzen, Beschnitt JPEG/GIF/WebP (re\22 §1.4) – Z. 127 (BM-1) – `fp-engine Import/Bitmap` – M – **Ergebnis B-2:** Import-Beschnitt gleich; Einpassen 1 px (runden) → B5; Rest X (GUI-Sitzung gesperrt) · **Ergebnis B-7:** Einpassen rundet (158) ✓
1. **BA-010** Pinsel: Umschalten auf natürliche Engine durch deren Werte; „Tintenmenge“ = Deckkraft (re\24 Nr. 12) – Z. 130 (PINSEL-1) – `dialogs (Pinsel)` – S – **Ergebnis B-2:** X (GUI-Sitzung gesperrt)
1. **BA-011** Auswahlrahmen erweitern/begrenzen: nutzt das Original die Morphologie aus re\05 §2.2 (+6 bei 5 px)? (NACHPRUEFUNG §3) – Z. 315 (PM-6) – `fp-engine Auswahl` – S – **Ergebnis B-2:** gleich (Morphologie +1/+6/+11, runde Ecken); Rand 5 px 2 px schmaler → B3-Hinweis; getPixelMask fehlt → B5
1. **BA-012** Bewegungsunschärfe Winkel 0 an B1 (Umfang 20): 300 oder 301 links – Z. 362 (DLG-6) – `blur.rs` – S – **Ergebnis B-2:** X (JSF im Original wirkungslos, Dialog nur GUI)
1. **BA-013** Menüfilter (z. B. Umkehren) an Bitmap mit durchsichtigem Rand: Zuschnitt auf Alpha > 0? – Z. 363 (DLG-6) – `trim_growth` – S – **Ergebnis B-2:** Filterfrage X; Unterschied: Rastern beschneidet auf Alpha > 0 → B3 – **Ergebnis B-3:** Rasterteil erledigt (B-3, 220459c) – Zuschnitt auf Alpha > 0
1. **BA-014** Filter auf Vektor: wann konvertiert das Original, wie viele Verlaufsschritte – Z. 957 (F4) – `fp-ui Filterablauf` – S – **Ergebnis B-2:** Unterschied (JSF): Filter rastert Vektor ohne Rückfrage → B5; Menüablauf X · **Ergebnis B-7:** JSF rastert ohne Rückfrage ✓, UI-Rückfrage bleibt

### M-C · Text und Effekte: `fx_text.fw.png` (FX-4) und Text an Pfad (TX2/P2, TX3 aus Datei, M1) (3)

1. **BA-015** Objektpuffer der Live-Effekte an Text (fx_text.fw.png: 23,46–167,112), weitere Messpunkte für die Regel – Z. 370 (FX-4) – `compose.rs` – M – **Ergebnis B-2:** X (Rauschanalyse nötig)
1. **BA-016** Helligkeit/Kontrast „klein“ am einfarbigen Vektor: Rechenweg des Mittelwerts (109 statt 113) – Z. 371 (FX-4) – `Filter Helligkeit/Kontrast` – S – **Ergebnis B-2:** gleich (16 Messpunkte pixelgleich)
1. **BA-017** Rahmen „roh“ beim Anfügen von Text an einen Pfad (TX2/P2 vs. TX3 aus Datei) – Z. 952 (F6/M1) – `fp-script, Text an Pfad` – S – **Ergebnis B-2:** Unterschied: raw = Pfadanfang/-breite, auch aus Datei → B5 · **Ergebnis B-7:** raw Links/Oben/Breite = Pfad ✓, Höhe offen

### M-D · Dialoge, Menüs und Inspektor-Popups nach PARITAET_B (Messdokument UX-B; Datei-Info nach `berichte\dlg11.md`) (11)

1. **BA-018** Bildvorschau 2-/4-fach mit Skalierung (50 %): Lage/Streckung des Exportbilds – Z. 125 (BV-5) – `crates\fp-ui\src\preview.rs` – S – **Ergebnis B-2:** X (GUI-Sitzung gesperrt)
1. **BA-019** Raster bearbeiten: Bereich und Kennlinie der Popup-Schieber ▾ an beiden Abständen (Annahme 1 … 100) – Z. 203 (DLG-2), Z. 966 (FIX-G1) – `dialogs\catalog.rs grid_draw` – S – **Ergebnis B-2:** X (GUI-Sitzung gesperrt)
1. **BA-020** Animieren mit Richtung ≠ 0; „Größe ändern“ im Inspektor bei Animationsinstanzen – Z. 224 (DLG-9) – `Animation` – S – **Ergebnis B-2:** Unterschied (JSF): Instanzgrenzen inkl. Weg → B5; Dialog X · **Ergebnis B-7:** JSF-Argumente korrigiert, Grenzen inkl. Weg ✓
1. **BA-021** Englische Beschriftung von D39 („Sample radius:“) und englische Verlaufsnamen der Auswahlbefehle – Z. 228 (DLG-8) – `ftl en` – S – **Ergebnis B-2:** X (nur deutsche Oberfläche installiert)
1. **BA-022** Tab-Folge von D45, D47–D50; Fokus ins Feld nach Klemmen beim OK – Z. 233 (FORM-1) – `dialogs\form.rs` – S – **Ergebnis B-2:** X (GUI-Sitzung gesperrt)
1. **BA-023** Datei-Info (D10): Register, Felder je Seite, Knöpfe, Startfokus, Esc, Größenänderung (Messauftrag berichte\dlg11.md) – Z. 234 (DLG-11) – `dialogs` – M – **Ergebnis B-2:** X (GUI-Sitzung gesperrt)
1. **BA-024** Farb-Popup: schließt der Strichlage-Knopf das Popup; Deckkraft einer Rasterwahl = Ausgangsfarbe? – Z. 241 (FARB-1b) – `inspector\popups.rs` – S – **Ergebnis B-2:** X (GUI-Sitzung gesperrt)
1. **BA-025** „Als Animation öffnen“ mit Dateien unterschiedlicher Größe und mit Symbolen – Z. 419 (NZ-4/NZ-5) – `files.rs` – M – **Ergebnis B-2:** X (nur über verbotenen openDocument-Aufruf bzw. GUI)
1. **BA-026** Freigaben: Bitmap-Auswahl wiederherstellen/löschen (anderes Dokument), Hilfslinien entfernen, Pfade kombinieren ▸ Teilen, Vorschau in zweitem Browser – Z. 434 (FV-13) – `command_state` – S – **Ergebnis B-2:** X (GUI-Sitzung gesperrt)
1. **BA-027** Lage von Volle/Halbe Höhe des Inspektors; Optimieren-Palettenbefehle bei GIF – Z. 625 (MN-3) – `panels` – S – **Ergebnis B-2:** X (GUI-Sitzung gesperrt)
1. **BA-028** Wahl im Exportformat des Dokument-Inspektors: legt das Original einen Verlaufsschritt an? – Z. 842 (PAR-X-INSP-1) – `inspector (Dokument)` – S – **Ergebnis B-2:** X (GUI-Sitzung gesperrt)

### M-E · JSF-Skripte im Original (`re\_testdateien\ux\uxa\`, Referenz `re\api\fw_api.json`) (3)

1. **BA-029** JSF: setzt das Original beim Setzen einer Textur eine Menge (textureAmount)? – Z. 353 (AU-3) – `fp-script pi.fill` – S – **Ergebnis B-2:** gleich (fillTexture gesetzt, textureBlend 0)
1. **BA-030** JSF: currentLayerNum = Unterebene – liefert das Original die Unterebene? – Z. 961 (F8) – `fp-script, fp-engine` – S – **Ergebnis B-2:** Unterschied: Unterebene aktiv, addNewSubLayer aktiviert/Name/verschachtelt → B5 – **Ergebnis B-5:** erledigt – Unterebene aktivierbar (Engine), `addNewSubLayer` verschachtelt/aktiviert/Name
1. **BA-031** JSF addNewLayer: Rückgabe Name (laut re\api\fw_api.json) oder ID – Z. 963 (F8) – `fp-script` – S – **Ergebnis B-2:** Unterschied: Rückgabe Name statt ID → B5 – **Ergebnis B-5:** erledigt – Rückgabe Name

### M-F · Export eines neuen bzw. des Testdokuments (`re\17`, `re\18`; BMP/PNG-Auflösung, HTML-Exportart) (5)

1. **BA-032** Exportassistent: Ergebnisseiten-Texte, Farbanzahl des GIF-Vorschlags (re\18 10.5) – Z. 235 (E3) – `dialogs (Exportassistent)` – S – **Ergebnis B-2:** X (GUI-Sitzung gesperrt)
1. **BA-033** Vorgabe-Exporteinstellung einer neuen Seite: GIF Adaptiv 256 oder PNG32 (re\17 §5.1) – Z. 308 (BV-1) – `crates\fp-export\src\doc_export.rs, JSF exportOptions` – S – **Ergebnis B-2:** Unterschied: Original PNG32, Firepixel GIF → B5 · **Ergebnis B-7:** PNG32 für neue Dokumente/Seiten ✓
1. **BA-034** re\18: Raster-Layout der Sprites (10.15), Trennzeichen „_“ (10.7), PDF-Qualitätsstufen – Z. 320 (E2) – `fp-export` – S – **Ergebnis B-2:** X (GUI-Sitzung gesperrt)
1. **BA-035** BMP-/pHYs-Rundung der Auflösung (CT-4) – Z. 366 (NZ-4) – `fp-export\src\formats.rs, fp-fwpng\src\png.rs` – S – **Ergebnis B-2:** Unterschied: pHYs = floor(trunc(dpi)/0,0254), auch JSF-Export → B5; BMP X · **Ergebnis B-7:** pHYs trunc/floor in FW-PNG, PNG, JSF ✓; BMP X
1. **BA-036** Merkt das Original die zuletzt gewählte HTML-Exportart auch für neue Dokumente? – Z. 967 (FIX-G2) – `Export HTML` – S – **Ergebnis B-2:** X (GUI-Sitzung gesperrt)

## B3 – Leinwand/Werkzeuge

Ergebnisse B-3 (14.10.): `docs\berichte\b3.md` (erledigt 7 inkl. BA-005/BA-013, teilweise 1, Entscheidung 2, verschoben 3), Commit `220459c`.

1. **BA-037** KI-Objektauswahl: Vorschau der Objektmaske beim Überfahren (optional, Bonus) – Z. 31 (KI) – `canvas\*, fp_engine::commands::ai` – M – **Ergebnis B-3:** verschoben – Bonus, ohne Modelle nicht prüfbar
1. **BA-038** Esc mitten im Pinselzug wie im Original (Zug behalten, Zeiger, Schritt „Bitmapmodus beenden“) – Z. 128 (PINSEL-1) – `BrushTool, canvas\paint_preview.rs` – S – **Ergebnis B-3:** war schon erledigt (67db4dd), Tests vorhanden; NACHPRUEFUNG §1 anpassen
1. **BA-039** Pinsel-Restabweichungen: Vorgabe nach Neustart, Werkzeugwerte merken, Vorschau in Stapelfolge mit Deckkraft/Mischmodus, Bézier-Anpassung, gesperrte Ebene, +1/+1-Versatz (Druck/Geschwindigkeit der Maus nicht machbar) – Z. 129 (PINSEL-1) – `BrushTool, paint_preview.rs` – L – **Ergebnis B-3:** teilweise: Neustart-Vorgabe und +1/+1 schon erledigt; Vorschau-Stapelfolge verschoben (L), Bézier und gesperrte Ebene wartet auf Messung (B-2), Druck X
1. **BA-040** Weichzeichnen/Schärfen: Gauß r > 200 auf Alpha-Rechteck beschränken, Wachstum je Durchgang am Text; FX-3-Gegenprüfung – Z. 372 (FX-4/FX-T) – `compose.rs, blur.rs` – M – **Ergebnis B-3:** war schon erledigt (FX-3); Regressionstest b3_fx_text
1. **BA-041** Stempel: Pixelwerte im Zug (Spitzenrasterung wie tool-retouch/13–14) – Z. 817 (PAR-F) – `Stempel-Werkzeug` – M – **Ergebnis B-3:** erledigt – 4 Pixel > 16 (max 18) gegen das Original, Test b3_stamp
1. **BA-042** Pipette 3×3/5×5: Original nimmt linkes oberes Pixel – Nutzerentscheidung, ob nachbauen – Z. 838 (PAR-F) – `Pipette` – S – **Ergebnis B-3:** wartet auf Nutzerentscheidung (Vorschlag: Mittelwert behalten) – **Ergebnis B-9:** geschlossen, bewusst anders – Nutzerentscheid 13.10. (Mittelwert behalten)
1. **BA-043** Verlaufs-/Mustergriffe auf der Leinwand ziehbar machen (ein Schritt) – Z. 841 (PAR-X-INSP-1) – `canvas\mod.rs fill_handles` – M – **Ergebnis B-3:** erledigt (220459c) – Griffe mit dem Zeiger ziehbar, ein Schritt; runder Griff: ungemessen
1. **BA-044** Innen einfügen: Pfad mit Vektormaske statt Maskengruppe (Objektmodell); „Als Maske einfügen“ Auswahl danach – Z. 951 (M1/F4) – `fp-engine commands\edit.rs` – L – **Ergebnis B-3:** verschoben (L)
1. **BA-045** Gamma 1.8: Schachbrett und Ansichten 2-/4-fach mit aufhellen – Z. 953 (F3) – `fp-ui canvas\render.rs` – S – **Ergebnis B-3:** erledigt (220459c) – Schachbrett und 2-/4-fach mit Gamma
1. **BA-046** Symbolbearbeitung im selben Dokument statt eigener Ansicht (Verlauf, Ebenen, Skript-API, Abblenden) – Z. 970 (F11) – `fp-engine commands\symbol_edit.rs, fp-ui canvas` – L – **Ergebnis B-3:** verschoben (L)
1. **BA-047** Farbwahl A3 (Leinwand-Pipette transparent → Weiß) und A4 (Werkzeugleiste Verlauf/Muster) bewusst entscheiden – Z. 241 (FARB-1b) – `inspector\popups.rs, tools_panel.rs` – S – **Ergebnis B-3:** wartet auf Nutzerentscheidung (Vorschlag A3 behalten, A4 nach Messung) – **Nutzerentscheid 14.10. 21:12:** geschlossen, bewusst anders – Pipette auf transparenter Leinwand nimmt Weiß; A4 (Werkzeugleiste) → BA-128, wartet auf Mess-Sitzung

## B4 – Dialoge/Datei-Dialoge/Gestaltung

1. **BA-048** Zoom-Rahmen in der Vorschaukachel gestrichelt wie auf der Leinwand – Z. 124 (BV-5) – `crates\fp-ui\src\preview.rs` – S – **Ergebnis B-4:** erledigt (B-4, f2808dc) – gestrichelter Leinwand-Rahmen (`canvas::dashed_rect`)
1. **BA-049** Tastaturkurzbefehle Rest: Symbolknöpfe beim Satz, Fokus/Pfeiltasten im Menübaum, „Befehle:“ als Kombifeld – Z. 227 (DLG-4) – `dialogs (Kürzel)` – M – **Ergebnis B-4:** erledigt (B-4, f2808dc) – 4 Symbolknöpfe (Name = Tooltip), „Befehle:“ + Kombifeld, Baum fokussierbar mit Fokusrahmen, ↑/↓/←/→/Pos1/Ende
1. **BA-050** Bitmap-Auswahl wiederherstellen: Live-Anzeige der entstehenden Auswahl – Z. 229 (KV-3) – `dialogs (bitmap_selection)` – M – **Ergebnis B-4:** erledigt (B-4, f2808dc) – Live-Anzeige ohne Verlaufsschritt, Abbrechen stellt zurück, OK = ein Schritt
1. **BA-051** Optimieren ▸ Einstellungen löschen: gewählte Einstellung direkt nach Rückfrage löschen – Z. 230 (MN-5) – `panels\optimize.rs, dialogs\panel_menus.rs` – S – **Ergebnis B-4:** erledigt (B-4, f2808dc) – gewählte eigene Einstellung → direkt Rückfrage; ohne gewählte eigene Einstellung (Original ungemessen) Auswahl wie bisher
1. **BA-052** Tab-Folge der Live-Effekt-Dialoge (Farbton/Sättigung, Helligkeit/Kontrast, Unscharf maskieren) wie im Menüdialog – Z. 231 (FORM-1) – `dialogs\effects.rs effect_form` – S – **Ergebnis B-4:** erledigt (B-4, f2808dc) – Tab-Folge wie Menüdialog (Unit-Test)
1. **BA-053** „Neues Symbol“ im Bibliothek-Bedienfeld wie D14 (9-Slice, Bibliothek-Kästchen) – Z. 232 (FORM-1) – `panel.library.newSymbol` – M – **Ergebnis B-4:** erledigt (B-4, f2808dc) – Anordnung D14, 9-Slice wirkt (`nineSlice`), Bibliothek-Kästchen grau; Verhalten (Bearbeitung öffnen?) ungemessen → M
1. **BA-054** Exportdialog meldet „exportiert“ (Bildvorschau danach schließen, falls Messung re\17 §11.2 das ergibt) – Z. 236 (BV-4) – `dialogs\export.rs` – S – **Ergebnis B-4:** teilweise (B-4, f2808dc) – Zähler `dialogs.exported`; Schließen der Bildvorschau wartet auf B-2 (re\17 §11.2)
1. **BA-055** Pinsel-Inspektor: Knopfbreite DE, senkrechte Popup-Schieber 24 × 222, Mischmodus-Liste (56 Einträge) – Z. 242 (PINSEL-1) – `inspector` – M – **Ergebnis B-4:** teilweise (B-4, f2808dc) – Knopftext ganz lesbar (`fields::button_fit`); Popup-Schieber schon senkrecht (parity tool-brush 07/09 ✅); Mischmodus-Liste (56) wartet auf B-2/eigenen Auftrag
1. **BA-056** Namen der Abschrägungs-Familie und Feld „Art“ – Nutzerentscheidung – Z. 244 (FX-2) – `Effekte, Glossar` – S – **Ergebnis B-4:** wartet auf Nutzerentscheidung (Namen der Abschrägungs-Familie, Feld „Art“) – kein Code – **Nutzerentscheid 14.10. 21:12:** Effektnamen wie im Original (Abgeflachte Kante innen/außen, Relief erhaben/vertieft, Gruppe „Abgeflachte Kante und Relief“) – **erledigt (F15)**: Namen DE/EN überall, Feld „Art“ entfernt, Dialogtitel = Effektname (`berichte15.md`)
1. **BA-057** Verlaufsnamen der Pfadbefehle „Pfade erweitern“/„Pfade einlassen“ – Z. 316 (PM-6) – `command_texts.ftl` – S – **Ergebnis B-4:** erledigt (B-4, f2808dc) – „Pfade erweitern“/„Pfade einlassen“ (en Expand/Inset Paths)
1. **BA-058** Knopf „Optionen…“ im Exportdialog mit Accessibility-Namen – Z. 321 (E2) – `dialogs\export.rs` – S – **Ergebnis B-4:** erledigt (B-4, f2808dc) – alle `fields::button` mit Accessibility-Namen
1. **BA-059** shortcuts.json gegen PARITAET_B §1 abgleichen (gemessene Werte gelten) – Z. 322 (MN-5) – `fp-engine\data\shortcuts.json` – S – **Ergebnis B-4:** geprüft (B-4, f2808dc) – aktiver Vorgabesatz zeigt die gemessenen Werte (Umschalt+F3, Strg+Alt+O, F5; Test); Abweichungen nur im nicht gemessenen Satz „Firepixel“ → ohne Messwerte keine Änderung (wartet auf B-2)
1. **BA-060** App-eigener Datei-Dialog: Vorschau und „XMP einschließen“ nachrüsten – Z. 409 (DLG-3) – `file_dialog.rs show_modal, files.rs` – M – **Ergebnis B-4:** erledigt (B-4, f2808dc) – Vorschau (Öffnen) und „XMP einschließen“ (grau, ☑) im App-Dialog; nativer Dialog bleibt X (BA-113)
1. **BA-061** Schrift-Menü: Schriften aus Inspektor/Texteditor in „zuletzt benutzt“ eintragen – Z. 492 (UX-B) – `menu.rs` – S – **Ergebnis B-4:** erledigt (B-4, f2808dc) – Inspektor und Texteditor tragen in `RecentFontList` ein
1. **BA-062** KI-Modelle verwalten: Modellspalte breiter oder „…“ mit Tooltip; Nachprüfung qps – Z. 522 (I18N-QA) – `dialogs (KI)` – S – **Ergebnis B-4:** erledigt (B-4, f2808dc) – Spalte bis 340 px nach längstem Namen, sonst „…“ + Tooltip; Nachprüfung qps
1. **BA-063** Dialoge D18–D23 Rest: D18 Zusatzkästchen, D19 Druckgröße/Ketten, D20 Pfeilsymbole + B:/H:, D21 Anordnung, D22 Selektives JPEG, D23 Textgröße als Kombifeld; Zugriffstasten in allen Dialogen – Z. 532 (UX-B) – `dialogs\*` – L – **Ergebnis B-4:** D18–D23 im Code erledigt (DLG-1b/DLG-2, geprüft B-4); Zugriffstasten in allen Formulardialogen → V (L, braucht &-Buchstaben je Dialog aus den Vorlagen)
1. **BA-064** Gestaltung Rest DES-1: Voreinstellungen Fußleiste, DESIGN §13 Prio 5–11, Knopfbreiten zentral – Z. 706 (DES-1) – `dialogs, ui_kit` – M – **Ergebnis B-4:** teilweise (B-4, f2808dc) – Voreinstellungen mit Fußleiste (Liste 10 px kürzer; leere Texte sperren die Fußleiste nicht mehr); DESIGN §13 Prio 5–11 und zentrale Knopfbreiten → V (DES-3)
1. **BA-065** Startseite: Eintrag „Batch“ – Z. 753 (BATCH-9) – `welcome` – S – **Ergebnis B-4:** erledigt (B-4, f2808dc) – „Batch-Verarbeitung…“ unter „Erweitern“

## B5 – Skript/Export/Web

1. **BA-066** JSF addNewHotspot("slice") → Segment; addNewText(rect, true) mit Voreinstellungen (Anti-Alias glatt, Unterschneidung aus, 120 %) – Z. 340 (UX-A) – `crates\fp-script\src\imp\doc.rs` – M – **Ergebnis B-5:** erledigt – Segment; Textvorgaben; JSF-Glättung nach `TAAi` korrigiert
1. **BA-067** JSF pixelRect eines Texts = Glyphengrenzen (nach FX-T prüfen) – Z. 347 (PM-3) – `crates\fp-script\src\imp\elem.rs` – S – **Ergebnis B-5:** erledigt – geprüft (43,60–148,98 wie Original), Test
1. **BA-068** JSF fw.setActiveViewScale an die Ansicht anbinden (heute No-op) – Z. 348 (PM-3) – `crates\fp-script\src\imp\fw.rs:323` – S – **Ergebnis B-5:** erledigt – `view.setZoom {center}`
1. **BA-069** JSF-Vorgabe Schlagschatten #000000a5 vs. Menü-Vorgabe #000000a2 – Z. 354 (FX-1) – `crates\fp-script\src\builtin_commands.rs` – S – **Ergebnis B-5:** erledigt – `#000000a2`
1. **BA-070** Batch effect.live: Abstand/Weichheit in der Vorschau über StepContext::preview_scale umrechnen (Feld vorhanden) – Z. 738 (BATCH-4/BATCH-6) – `crates\fp-batch\src\step.rs, steps` – S – **Ergebnis B-5:** erledigt – war umgesetzt (BATCH-11a), Test
1. **BA-071** HTML-Export mit Schaltflächen-Segmenten (TW-19) prüfen – Z. 954 (F3) – `fp-export HTML` – M – **Ergebnis B-5:** erledigt – Schaltflächen-Segmente in der Tabelle (ohne Rollover → neuer Befund)
1. **BA-072** TW-19: aktive Fläche aus dem Schaltflächensymbol auswerten – Z. 955 (F3) – `fp-export / Segmente` – S – **Ergebnis B-5:** erledigt – aktiver Bereich aus der Symbol-Web-Ebene (Export + Leinwand)
1. **BA-073** JSF: undefined statt null (name, d.zoom, docTitle, brush.diameter) – Z. 956 (F4/FIX-R2) – `fp-script Wertumwandlung` – S – **Ergebnis B-5:** erledigt – `null` als Lücke in JS-Arrays (Beleg M2 q18)
1. **BA-074** Unterebene als aktive Ebene; addNewSubLayer verschachtelt – Z. 961 (F8) – `fp-engine, fp-script` – M – **Ergebnis B-5:** erledigt – mit BA-030
1. **BA-075** reorderLayer/duplicateLayer für Unterebenen (Engine-Befehl fehlt) – Z. 962 (F8) – `fp-script, fp-engine` – M – **Ergebnis B-5:** erledigt – duplizieren/umsortieren in fp-script
1. **BA-076** SVG-Import: Stapelfolge von Text und Pfaden – Z. 964 (F2) – `fp-io vector.rs` – M – **Ergebnis B-5:** erledigt – Platzhalterpfade für usvg, Text in SVG-Stapelfolge

## B6 – Aufräumen/Tests/Doku

Ergebnisse B-6 (14.10.): `docs\berichte\b6.md` (erledigt 14, offen/verschoben 6), Commit `ab45d1e`.

1. **BA-077** new_document.rs: eigene underline_access auf dialogs::access umstellen – Z. 237 (NZ-3) – `crates\fp-ui\src\dialogs\new_document.rs:171` – S – **Ergebnis B-6:** erledigt – `access::underline` statt eigener Funktion
1. **BA-078** Unbenutzte Schlüssel pi-color-pipette/pi-color-transparent entfernen (kein .rs-Treffer) – Z. 239 (FARB-1b) – `inspector.ftl de/en` – S – **Ergebnis B-6:** erledigt – beide Schlüssel de/en entfernt
1. **BA-079** Altpfad ColorChoice::Pipette / popups::start_pick prüfen und entfernen – Z. 240 (FARB-1b) – `crates\fp-ui\src\inspector\popups.rs:83` – S – **Ergebnis B-6:** erledigt – Variante, `start_pick`, `poll_pick`, `PickState` entfernt
1. **BA-080** misc::stroke_options_form unbenutzt – entfernen – Z. 243 (PINSEL-1) – `crates\fp-ui\src\dialogs\misc.rs:208` – S – **Ergebnis B-6:** erledigt – samt Hilfsfunktionen und 29 nur dort benutzten Schlüsseln je Sprache
1. **BA-081** Unbenutzte dlg-md-*-Schlüssel (Ziel/Größe/Batch, 14 + 4 je Sprache) entfernen – Z. 723 (E3), Z. 744 (BATCH-2) – `crates\fp-i18n\locales\*\dialogs.ftl` – S – **Ergebnis B-6:** erledigt – 19 Schlüssel je Sprache entfernt (Grep ohne Treffer)
1. **BA-082** Gesamttests: file.batchProcess öffnet das Studio – ausnehmen; Erlaubnis im clipping_audit streichen (prüfen) – Z. 745 (BATCH-2) – `fp-ui tests` – S – **Ergebnis B-6:** erledigt (geprüft) – Studio zählt seit FIX-B als Wirkung, keine Erlaubnis-Einträge mehr; Bericht beim nächsten GT neu
1. **BA-083** Golden-Hashes des Resamplers auf anderen OS (CI) neu erzeugen – Z. 746 (BATCH-4) – `fp-batch Tests` – S – **Ergebnis B-6:** offen – braucht CI auf anderem OS (→ V)
1. **BA-084** AGENTS.md/AGENTS.en.md: Abschnitt „Video über ffmpeg“ (FIREPIXEL_FFMPEG) – Z. 750 (BATCH-7) – `docs\AGENTS*.md` – S – **Ergebnis B-6:** erledigt – AGENTS.md/AGENTS.en.md §5c „Video über ffmpeg“
1. **BA-085** DESIGN.md §12 Startbildschirm; design_willkommen schreibt design_after_* statt design_nachher_*; Bilder neu – Z. 707 (DES-2), Z. 726 (TE2-E) – `docs\DESIGN.md, crates\fp-ui\tests\design_willkommen.rs` – S – **Ergebnis B-6:** erledigt – `design_after_welcome*` neu, alte Entwurfsbilder entfernt, 102 `design_vorher_*` → `design_before_*` mit Verweisen
1. **BA-086** Deutsche QA-Bereichs-IDs/Ordner in i18n_qa_* englisch (alte Ergebnisse weiter lesen) – Koordinator – Z. 725 (TE2-A/TE2-E) – `crates\fp-ui\tests\i18n_qa_*` – M – **Ergebnis B-6:** offen – Entscheidung Koordinator ausstehend
1. **BA-087** Automatische Berichte (I18N_PRUEFUNG, funktionstest\*_auto) mit neuen Ordner-/ftl-Namen neu erzeugen – Z. 719 (TE-1) – `docs` – S – **Ergebnis B-6:** offen – entsteht beim nächsten GT-Lauf
1. **BA-088** gesamt_menu: Sprachwechsel prozessweit – Sprache zurücksetzen oder seriell (prüfen) – Z. 413 (FV-13) – `crates\fp-ui\tests\gesamt_menu.rs` – S – **Ergebnis B-6:** erledigt – `run_item` setzt die Sprache nach jedem Versuch zurück
1. **BA-089** kittest/wgpu: gemeinsame Sperre um render() gegen Zugriffsverletzung im Programm all – Z. 416 (NZ-2) – `fp-ui tests` – M – **Ergebnis B-6:** verschoben – Sperre müsste Bild und Abbau umschließen (25 Dateien), kein Absturz in GT-0f…GT-2
1. **BA-090** Wackelnde Tests width_field_limited_to_6000_with_message, batch_ui::trockenlauf_und_lauf_zweimal – Z. 968 (GT-0f/FIX-F) – `fp-ui tests` – M – **Ergebnis B-6:** erledigt – Ursachen behoben (Sprachzähler je Thread; eigener Testordner je Batch-Test), Regressionstest
1. **BA-091** Runner-Lücken tools\parity\ (ui_map-Schlüssel, repeat, owner-drawn Untermenüs, Drehbücher) – Z. 969 (RUN-2/RUN-3/RUN-4/A3-2) – `tools\parity\` – L – **Ergebnis B-6:** offen (L) – Läufer gegen das Original, nicht in B6 bearbeitet
1. **BA-092** kittest-Bildserien/Paarbilder mit Text nach Hinting neu erzeugen – Z. 374 (FX-T) – `fp-ui tests` – S – **Ergebnis B-6:** offen – nächster Bildlauf (NACHPRUEFUNG)
1. **BA-093** Korpus-Pixeltests (--ignored) nach InkBuffer::stamp-Änderung laufen lassen – Z. 375 (PINSEL-1) – `fp-vector` – S – **Ergebnis B-6:** erledigt – fp-vector `--ignored` grün (7/7)
1. **BA-094** „Befehle ▸ Demo - aktuelles Dokument“ im echten Fenster; judge meldet Fremdprogramm-Start – Z. 420 (MN-4) – `gesamt_common::judge` – S – **Ergebnis B-6:** erledigt (judge) – unterdrückte Fremdprogramm-Starts → „im echten Fenster prüfen“; Sichtprüfung NACHPRUEFUNG
1. **BA-095** winit focus_window() beim normalen Start nötig? (simulierter Alt-Druck) – Z. 421 (START-1) – `crates\fp-ui\src\start_window.rs:148` – S – **Ergebnis B-6:** erledigt – `focus_window` nur bei nicht aktivem Fenster; Sichtprüfung NACHPRUEFUNG
1. **BA-096** bridge.rs: Bildschirmfoto-Zuordnung eindeutig machen – Z. 425 (FARB-1b) – `bridge.rs` – S – **Ergebnis B-6:** erledigt – Zuordnung nur über die eigene Kennung

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

1. **BA-112** Native Datei-Dialoge mittig platzieren – nur per Win32/unsafe (Workspace unsafe_code = forbid) – Z. 408 (DLG-3) – `file_dialog.rs spawn_native` – S – **Ergebnis B-4:** bleibt X: `SetWindowPos` nur `unsafe` oder neue Abhängigkeit; ein unsichtbares Besitzerfenster ginge nur über einen eigenen winit-Fenstergriff, den eframe für Unterfenster nicht herausgibt
1. **BA-113** Nativer Dialog: Zusatzfelder (IFileDialogCustomize = unsafe) und Filter-Vorwahl (rfd kann es nicht) – Z. 409 (DLG-3), Z. 410 (DLG-3) – `file_dialog.rs` – S – **Ergebnis B-4:** bleibt X (`IFileDialogCustomize` nur über COM/`unsafe`); Ersatz: alle Zusatzfelder im App-Dialog (BA-060)
1. **BA-114** Systemfarbwähler (A1) und Pipette außerhalb des Fensters (A2) – nur mit unsafe bzw. Zusatzabhängigkeit – Z. 241 (FARB-1b), Z. 959 (F5) – `inspector\popups.rs` – S
1. **BA-115** XMP-Standardkennung enthält den Herstellernamen – Neutralitätsregel §1a verbietet es – Z. 447 (DLG-10) – `fp-fwpng XMP` – S
1. **BA-116** CFF-Schrift (andere Schriftfassung unter Windows) bleibt bei 52–72 % exakten Pixeln; gedrehter Text ungehintet – Z. 373 (FX-T) – `fp-text` – S

## N – neu aus dem Abbau (B-2 … B-9, eingetragen von B-8)

1. **BA-117** Auswahlrahmen: Rand 5 px in x 2 px breiter als im Original, erweitern 10 unten 1 px kürzer (Maskengrenze) – aus B-2 (BA-011) – `fp-engine Auswahl` – S – **V** (nächstes Code-Paket)
1. **BA-118** JSF `dom.getPixelMask` fehlt (liefert `null`) – aus B-2 (BA-011) – `crates\fp-script\src\imp\doc2.rs` – M – **V**
1. **BA-119** In Bitmap konvertieren ignoriert die Deckkraft mitgewählter Objekte (Deckkraft 0 wird deckend gerastert; Original: fällt weg) – bäckt das Original die Deckkraft ein? – aus B-3 – `fp-engine render_backend.rs`, `fp-ui canvas\render.rs` – M – **Mess-Sitzung**
1. **BA-120** Verlaufsgriffe: beim Ziehen bleiben die alten Griffe sichtbar (kosmetisch) – aus B-3 – `fp-ui canvas` – S – **V**
1. **BA-121** HTML-Export: Schaltflächen ohne Rollover (Bild austauschen/Navigationsleiste, Bilder der Status 2–4) – aus B-5 – `fp-export` – L – **V**
1. **BA-122** JSF-Name eines Segments aus `addNewHotspot("slice")` (`null`; Original zeigt „_r2_c2“) – aus B-5 – `fp-script` – S – **Mess-Sitzung**
1. **BA-123** `reorderLayer` einer Unterebene auf eine Ebene der obersten Stufe (Annahme) – aus B-5 – `fp-script` – S – **Mess-Sitzung**
1. **BA-124** pHYs bei Einheit Pixel/cm (Original nur in dpi gemessen) – aus B-7 – `fp-fwpng`, `fp-export` – S – **Mess-Sitzung**
1. **BA-125** `fw.createFirepixelDocument(…, {pixelsPerUnit, units: "centimeter"})` übernimmt die Zahl anscheinend als dpi – aus B-7 (Nebenbefund B-2) – `fp-script` – S – **V** (prüfen)
1. **BA-126** Aktive Unterebene im Original ungemessen: Klick auf die Unterebenenzeile, „Für Seiten freigeben“ grau, `layer.move`-Index in der Elternebene, Klick auf die Ebenenzeile hebt die Unterebenenzeile mit hervor – aus B-9 – `fp-ui panels\layers.rs` – S – **Mess-Sitzung**
1. **BA-127** Vorgabe-Kürzelsatz: Testsitzung mit „Web Standard“, `prefs.rs`-Rückfall „Firepixel“ – welcher Satz ist im Original Vorgabe (mit BA-059) – aus B-4 – `fp-ui prefs.rs` – S – **Mess-Sitzung**
1. **BA-128** Farbwahl A4: Werkzeugleiste Verlauf/Muster (Original wechselt ins Verlaufs-Popup?) und Strichlage-Knöpfe ohne Auswahl – Rest von BA-047 nach Nutzerentscheid 14.10. 21:12 – `tools_panel.rs` – S – **Mess-Sitzung**

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
