# Vollabgleich – Inventar der Einheiten (PAR-0, 13.10.2026)

Endgültige Liste für das Fließband aus `docs\PARITY_SWEEP.md` (Stufen O → F → X → R). **61 Einheiten**, jede für
einen Agenten in 10–15 min im Original aufnehmbar, höchstens 25 Prüfpunkte. Reihenfolge laut Nutzer: Werkzeuge
(Bitmap zuerst) → Inspektor → Effekte/Filter → Farbwähler/Farbfelder → Bedienfelder → Bildvorschau/Export →
Querschnitt → Menüs (Wirkung) → Dialoge. Ordner, Bildnamen und Vergleichsskript: `docs\parity\README.md`.

Quellen: `crates\fp-engine\data\commands.json` (280 Befehle: Datei 33, Bearbeiten 29, Ansicht 36, Auswählen 15,
Modifizieren 74, Text 39, Befehle 3, Filter 1, Fenster 23, Hilfe 7, ohne Menü 20), `PARITAET_A.md` (W/F/T/O/L/K),
`PARITAET_B.md` (§1 Menüs, §2 D01–D50, §3 ≡-Menüs), `NACHPRUEFUNG.md`, `NUTZERBEFUNDE.md` (N01–N31),
`crates\fp-ui\src\tools_panel.rs`, `inspector\`, `panels\`, `dialogs\`, `fp-effects`, `fp-raster\src\filters`, `re\*.md`.

## Legende

- **Zustände** (auch Bildnamen-Suffix): `normal` · `hover` · `pressed` · `disabled` · `open` (aufgeklappt) ·
  `result` (Ergebnis auf der Leinwand/Datei) · `focus`. In den Tabellen abgekürzt n/h/p/d/o/r/f.
- **Methode Original:** `J` JSF-Wert (`fw.*`/`dom.*`, Werte ins `Files`-Log) · `W` Fenstertext/Zustand per
  Fensternachricht (`pi.ps1`, WM_GETTEXT, aktiv/grau) · `E` Export als PNG + Hash/Pixelwerte · `B` Bild.
- **Methode Firepixel (Brücke):** `S` `ui_state` · `Fi` `ui_find` · `D` `get_document` · `R` `run_script`
  (dieselbe JSF wie im Original) · `A` `ui_action`/`ui_input` (Vorgang auslösen) · `E` `render`/Export + Hash ·
  `B` `screenshot`.
- **Bilder** = erwartete Referenzbilder des Originals je Prüfpunkt (Firepixel dieselbe Zahl, dieselben Namen).
  Bildname: `<nr>_<kürzel>_<zustand>.png`, `<kürzel>` steht in Spalte 2. Je Einheit zusätzlich `00_overview_normal.png`.
- **🎨** = bewusst anders gestaltet, nicht zurückbauen (Quelle). **Bef.** = bekannte Befunde.
- **Abh.** = Abhängigkeit von laufenden Paketen: Stufe O darf jederzeit laufen (Original ändert sich nicht);
  **Stufe F erst nach dem Commit des genannten Pakets**, sonst misst sie einen veralteten Stand.

## Grundeinstellung (alle Einheiten, beide Programme)

Fenster 1600 × 1000 (nicht maximiert), Zoom 100 %, Sprache Deutsch, Firepixel-Design „Standard“ (Gestaltung ohnehin 🎨
DES-1), eigene Firepixel-Instanz mit eigenem APPDATA und `--mcp` (AGENTEN_REGELN §4d/§4e). Vor jeder Einheit per JSF:
Füllfarbe `#808080`, Strichfarbe `#C04040`, Strich „Einfach/Hart 1 px“, Deckkraft 100 %, Mischmodus Normal, Auswahl leer.
Werkzeugvorgaben des Originals sind seine Werksvorgaben (PREF 1001); abweichende gespeicherte Werte im Kopf der
`.orig.md` notieren. Keine Befehle, die Datei-Dialoge offen stehen lassen; Original nie beenden.

## Benötigte Testdateien (`re\_testdateien\par\`, erzeugt von PAR-O-1, außerhalb von `firepixel\`)

| Kürzel | Datei | Inhalt | benutzt von |
|---|---|---|---|
| T01 | `par_empty.fw.png` | 800 × 600, weiß, eine Ebene, leer | 01–03, 09–15, 24, 43–46 |
| T02 | `par_bitmap.fw.png` | 800 × 600; Bitmap B1 480 × 320 bei 40,40 (Verlaufsfläche, harte Kanten, Hauttöne, zwei rote Kreise r = 12 als „Augen“, feines Schachbrett) | 03–07, 20, 28, 29, 52 |
| T03 | `par_bitmap_edge.fw.png` | 600 × 500, transparenter Hintergrund, Bitmap mit Motiv 200 × 300 und breitem transparentem Rand (Fall N26) | 06, 20, 47 |
| T04 | `par_vector.fw.png` | R1 Rechteck 50,50–150,120 ohne Strich; R2 wie R1 bei 200,50, 50 % Deckkraft; E1 Ellipse 50,170–150,240; P1 offener Bézierpfad 4 Punkte; PG Polygon 6 Ecken; ST Stern-Autoform; L1 Linie | 08–11, 13, 17–19, 24, 26, 27, 43, 50, 53, 54 |
| T05 | `par_text.fw.png` | TX1 Punkttext „Abc 123“ 24 pt Vorgabeschrift; TX2 Rahmentext 200 × 80, drei Zeilen; TX3 Text an Pfad; ein Pfad P2 zum Anfügen | 12, 21, 55 |
| T06 | `par_symbol_web.fw.png` | Gruppe G (2 Objekte), Grafiksymbol + Instanz, Schaltfläche (4 Zustände), Animationssymbol (3 Bilder), Segment S1, Polygonsegment S2, Hotspots Rechteck/Kreis/Polygon, 2 Seiten | 14, 22, 23, 37, 39, 53 |
| T07 | `par_layers.fw.png` | 3 Ebenen + 1 Unterebene, eine ausgeblendet, eine gesperrt, gemeinsame Ebene, Webebene; 2 Seiten, 3 Zustände | 33–35 |
| T08 | `par_effects.fw.png` | 4 Objekte ohne Effekt (Rechteck, Text, Bitmap, Ellipse) + Rechteck FX mit Stapel Schlagschatten → Glühen → Abschrägung | 25–29 |
| T09 | `par_export.fw.png` | 400 × 300: Foto-Bitmap, Flächen, transparente Ecke, 2 Zustände, 2 Segmente | 36, 40–42, 49 |
| T10 | `par_large.fw.png` | 2400 × 1600, Raster aus nummerierten Kacheln (für Bildlauf, Zoom, Lineale, Hilfslinien) | 41, 46, 51 |
| T11 | `par_import\` | `par.png` (RGBA), `par.jpg`, `par.gif` (animiert, 3 Bilder), `par.bmp`, `par.tif`, `par.psd` (2 Ebenen + Text), `par.svg`, eine Vektordatei aus dem Korpus (`re\_testdateien\` vorhandene Beispiele kopieren) | 47, 49 |
| T12 | `par_swatches.act` | 16 Farben für Farbfelder laden/ersetzen | 32 |

Jede Datei einmal im Original anlegen (JSF), als FW-PNG speichern; Firepixel öffnet **dieselbe** Datei. Lage der Objekte
in `re\_testdateien\par\TESTFILES.md` (PAR-O-1) mit Grenzen per JSF.

## Übersicht

| Nr | Kennung | Titel | Punkte | Bilder ≈ | Abh. |
|---|---|---|---:|---:|---|
| 01 | `tool-brush` | Pinsel: Strich live, Grundfelder | 18 | 27 | PINSEL-1, RE-PINSEL |
| 02 | `tool-brush-stroke-options` | Pinsel: Strichart, Kante, Textur, Mischmodus, „Pinselstrich bearbeiten“ | 20 | 30 | PINSEL-1, RE-PINSEL |
| 03 | `tool-pencil-eraser` | Buntstift, Radiergummi | 16 | 19 | PINSEL-1 |
| 04 | `tool-retouch` | Weichzeichner, Scharfzeichner, Abwedler, Nachbelichter, Verwischen | 18 | 27 | – |
| 05 | `tool-stamp-replace-redeye` | Stempel, Farbe ersetzen, Rote Augen entfernen | 15 | 22 | – |
| 06 | `tool-bitmap-selection` | Auswahlrahmen, Ellipse, Lasso, Polygonlasso, Zauberstab, Bitmapmodus | 22 | 30 | BM-1 |
| 07 | `tool-fill-gradient-eyedropper` | Farbeimer, Verlauf, Pipette | 16 | 20 | FARB-1b |
| 08 | `tool-pointer-transform-crop` | Zeiger, Unterauswahl, Hintere auswählen, Skalieren, Verzerren, Verkrümmen, Zuschneiden, Exportbereich | 24 | 27 | – |
| 09 | `tool-line-pen` | Linie, Stift, Vektorpfad, Pfad neu zeichnen | 18 | 19 | – |
| 10 | `tool-shapes` | Rechteck, Ellipse, Polygon | 16 | 15 | – |
| 11 | `tool-autoshapes` | 12 automatische Formen und ihre Griffe | 16 | 29 | – |
| 12 | `tool-text` | Textwerkzeug auf der Leinwand | 20 | 20 | – |
| 13 | `tool-path-reshape` | Freiform, Bereich umformen, Pfadausbesserung ±, Messer | 14 | 18 | – |
| 14 | `tool-web` | Hotspots, Segment, Polygonsegment, Ein-/Ausblenden | 15 | 19 | – |
| 15 | `toolbox` | Werkzeugleiste: Gruppen, Ausklapplisten, Tooltips, Hand, Zoom, Ansichtsmodi | 18 | 28 | – |
| 16 | `inspector-document` | Inspektor ohne Auswahl (Dokument) | 14 | 12 | – |
| 17 | `inspector-path-rect` | Inspektor Rechteck / Pfad / Ellipse | 18 | 18 | – |
| 18 | `inspector-fill` | Füllung: Kategorien, Verläufe, Kanten, Texturen, Muster | 22 | 34 | FARB-1b |
| 19 | `inspector-stroke` | Strich: Kategorien, Pinsel, Position, Textur, „Pinselstrich bearbeiten“ | 20 | 31 | PINSEL-1 |
| 20 | `inspector-bitmap` | Inspektor Bitmap | 12 | 13 | BM-1 |
| 21 | `inspector-text` | Inspektor Text (alle Felder, live) | 24 | 28 | – |
| 22 | `inspector-group-symbol` | Gruppe, Symbol, Instanz, Schaltfläche, Animation, Mehrfachauswahl | 18 | 14 | – |
| 23 | `inspector-web` | Segment und Hotspot | 14 | 11 | – |
| 24 | `inspector-fields` | Feldverhalten aller Zahlenfelder, Popup-Schieber, Rückgängig je Feld | 24 | 17 | – |
| 25 | `fx-stack` | Effektliste im Inspektor (+/−, Haken, Reihenfolge, Optionen, als Stil) | 18 | 16 | FX-1 |
| 26 | `fx-shadow-glow` | Schlagschatten, Innerer Schatten, Glühen, Inneres Glühen, Massiver Schatten | 20 | 16 | FX-1, FARB-1a, RE-FX |
| 27 | `fx-bevel-emboss-fill` | Abschrägung, Prägung, Farbfüllung, übernommene Ebeneneffekte | 18 | 15 | FX-2, RE-FX |
| 28 | `filters-adjust-color` | Farbe anpassen (Helligkeit/Kontrast, Farbton/Sättigung, Kurven, Tonwert, Auto-Tonwert, Umkehren, Farbfüllung) | 20 | 17 | FX-4, RE-FX |
| 29 | `filters-blur-sharpen-noise` | Weichzeichnen, Scharfzeichnen, Rauschen, Sonstige | 22 | 23 | FX-3, FX-4, RE-FX |
| 30 | `color-popup` | Farbwähler-Popup (Original gilt, N27b) | 22 | 18 | FARB-1a, FARB-1b |
| 31 | `color-popup-contexts` | Farbwähler an allen Stellen, Farbbereich der Werkzeugleiste, Systemfarbwähler | 18 | 19 | FARB-1b |
| 32 | `panel-swatches-mixer` | Farbfelder, Mischer, Farbpalette | 20 | 17 | FARB-1b |
| 33 | `panel-layers` | Ebenen: Zeilen, Augen, Schloss, Ziehen, Umbenennen | 22 | 17 | – |
| 34 | `panel-layers-menu` | Ebenen: ≡-Menü, Kontextmenü, Fußleiste, Ebenenoptionen | 18 | 12 | – |
| 35 | `panel-pages-states-history` | Seiten, Status, Verlauf | 22 | 16 | – |
| 36 | `panel-optimize` | Optimieren | 20 | 21 | – |
| 37 | `panel-library-styles` | Dokumentbibliothek, Allgemeine Bibliothek, Stile, URL | 18 | 15 | – |
| 38 | `panel-info-align-path` | Info, Ausrichten, Pfad | 18 | 18 | – |
| 39 | `panel-misc` | Autoformen, Autoform-Eigenschaften, Sonderzeichen, Bildbearbeitung, CSS, Suchen, Verhalten, Symboleigenschaften | 20 | 19 | – |
| 40 | `preview-options` | Bildvorschau: Registerkarte Optionen, Formate, Palette, Transparenz | 24 | 27 | – |
| 41 | `preview-views` | Bildvorschau: 1/2/4-fach, synchron, Werkzeuge, Datei/Animation | 20 | 22 | BV-5 |
| 42 | `export-results` | Export-Ergebnisse (Dateien, Namen, Segmente, Zustände, HTML) | 16 | 11 | – |
| 43 | `xs-context-menus` | Kontextmenüs (Leinwand je Objektart, Reiter, Bedienfeldgruppe) | 16 | 20 | – |
| 44 | `xs-shortcuts` | Tastenkürzel (Befehle, Werkzeugbuchstaben, Modifikatoren) | 22 | 5 | – |
| 45 | `xs-preferences` | Voreinstellungen: Wirkung je Kategorie | 20 | 13 | – |
| 46 | `xs-canvas` | Leinwand: Zoom, Bildlauf, Lineale, Hilfslinien, Raster, Fang, Pixelraster | 22 | 19 | – |
| 47 | `xs-file-formats` | Öffnen/Importieren/Speichern je Format | 18 | 14 | BM-1 |
| 48 | `xs-windows-tabs` | Reiter, Fenster, Docking, Statuszeile, Startbildschirm | 20 | 20 | – |
| 49 | `menu-file` | Menü Datei – Wirkung | 22 | 6 | – |
| 50 | `menu-edit` | Menü Bearbeiten – Wirkung | 22 | 8 | – |
| 51 | `menu-view` | Menü Ansicht – Wirkung | 24 | 12 | – |
| 52 | `menu-select` | Menü Auswählen – Wirkung | 15 | 8 | – |
| 53 | `menu-modify-a` | Modifizieren: Leinwand, Bild, Animation, Symbol, Popup, Maske, Selektives JPEG, Bitmap | 24 | 10 | – |
| 54 | `menu-modify-b` | Modifizieren: Transformieren, Anordnen, Ausrichten, Pfad, Kombinieren, Gruppieren | 24 | 6 | – |
| 55 | `menu-text` | Menü Text – Wirkung | 20 | 10 | – |
| 56 | `menu-commands-window-help` | Befehle, Filter (Menü), Fenster, Hilfe – Wirkung | 18 | 9 | – |
| 57 | `dialogs-d01-d10` | D01–D10 Verhalten | 20 | 20 | – |
| 58 | `dialogs-d11-d20` | D11–D20 Verhalten | 22 | 26 | VT-6 (nur Nachzeichnen, 🎨) |
| 59 | `dialogs-d21-d30` | D21–D30 Verhalten | 22 | 18 | – |
| 60 | `dialogs-d31-d40` | D31–D40 Verhalten | 20 | 19 | – |
| 61 | `dialogs-d41-d50` | D41–D50 Verhalten (Filterdialoge, Felder; Pixel → RE-FX) | 22 | 23 | FX-3, FX-4 |

Summe 1181 Prüfpunkte, ≈ 1113 Referenzbilder inkl. Übersicht (gespeichert; angesehen höchstens 12 je Agent und Programm).
Zeit Stufe O: 61 × 10–15 min ≈ 10–15 h; bei Engpass zuerst 01–42 (Nutzerschwerpunkte), 43–61 danach.

---

## A. Werkzeuge

### 01 · `tool-brush` – Pinsel: Strich live, Grundfelder
**Test:** T01 + T02. Werkzeug B (Pinsel), Werksvorgaben. **Abh.:** PINSEL-1 (F erst danach), Messreihe RE-PINSEL `re\24-pinsel.md` (Werte dort, hier Bilder/Verhalten).
**🎨:** – · **Bef.:** N31 (Stempel-Umriss statt Strich, Felder ohne Wirkung), PARITAET_A W1/W20/L36, BEFUNDE LW (Pinsel).

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `inspector_idle` | Inspektor Pinsel ohne Auswahl: Felder, Reihenfolge, Vorgaben | n | W, B | S, Fi, B | 1 |
| 02 | `stroke_live` | Strich **während** des Ziehens sichtbar (kein Kreis-Umriss) | p | B (mitten im Zug) | B | 2 |
| 03 | `stroke_result` | Strich nach Loslassen, 1 px Hart, waagrecht 100 px | r | E (Pixel) | E | 1 |
| 04 | `target_empty` | Malen auf leerer Fläche: neues Bitmap-Objekt, Grenzen | r | J (Objekt, Grenzen) | D | 1 |
| 05 | `target_bitmap` | Malen auf gewählter Bitmap B1: kein neues Objekt | r | J | D | 1 |
| 06 | `target_vector` | Malen bei gewähltem Vektorobjekt → Pinsel ist Vektor? (L36) | r | J | D | 1 |
| 07 | `size` | Größe 1/10/50: Strichbreite | r | E | E | 3 |
| 08 | `edge` | Rand/Weiche Kante 0/50/100 | r | E | E | 3 |
| 09 | `opacity` | Deckkraft 50 %: Strich und Überlappung im selben Zug | r | E | E | 1 |
| 10 | `blend` | Mischmodus „Durchschnitt“/„Multiplizieren“ über B1 | r | E | E | 2 |
| 11 | `preserve_alpha` | „Transparenz beibehalten“ an/aus auf T03 | r | E | E | 2 |
| 12 | `shift_line` | Umschalt: gerade Linie / 45° | r | E | E | 2 |
| 13 | `click_dot` | Einzelklick: ein Abdruck | r | E | E | 1 |
| 14 | `esc` | Esc während/nach dem Zug | r | J | D, H | 1 |
| 15 | `cursor` | Mauszeiger (Größe als Kreis, Fadenkreuz bei Feststelltaste) | n | B | B | 2 |
| 16 | `selection_clip` | Malen mit Auswahlrahmen: nur innen | r | E | E | 1 |
| 17 | `undo` | ein Zug = ein Verlaufsschritt, Name | r | J (history) | H | 0 |
| 18 | `no_frame` | kein Auswahlrahmen um die Bitmap während des Malens (FV-11) | p | B | B | 1 |

### 02 · `tool-brush-stroke-options` – Pinsel: Strichart, Kante, Textur, Mischmodus, Pinselstrich bearbeiten
**Test:** T01. **Abh.:** PINSEL-1, RE-PINSEL. **🎨:** – · **Bef.:** N31 (Strichart „Weich“, Textur „Herz 01“ ohne Wirkung; „Pinselstrich bearbeiten…“ unklar), W21, PARITAET_A „Popup-Schieber Kante/Textur/Strich“ ungeprüft.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `category_list` | Liste Strichkategorien (Reihenfolge, Namen) | o | W, B | Fi, B | 1 |
| 02 | `type_list` | Arten je Kategorie (z. B. Airbrush, Kalligrafie, Kohle, Buntstift, Filzstift, Weich…) | o | W, B | Fi, B | 2 |
| 03 | `type_result` | je Kategorie ein Probestrich (Grundart) | r | E | E | 12 |
| 04 | `edge_slider` | Popup-Schieber „Kante“ (senkrecht?, Grenzen) | o | W, B | S, B | 1 |
| 05 | `texture_list` | Texturliste (77 Einträge, Breite, „Andere…“) | o | W, B | Fi, B | 1 |
| 06 | `texture_result` | Textur „Herz 01“ 50 % auf Strich | r | E | E | 1 |
| 07 | `texture_amount` | Menge 0/100 | r | E | E | 2 |
| 08 | `blend_list` | Mischmodus-Liste vollständig | o | W | Fi | 1 |
| 09 | `edit_open` | „Pinselstrich bearbeiten…“: Dialog, Register (Optionen/Form/Empfindlichkeit) | o | W, B | Fi, B | 3 |
| 10 | `edit_options` | Register Optionen: Tinte, Abstand, Fluss, Textur, Kantenstruktur, Streuung | n | W | S | 0 |
| 11 | `edit_shape` | Register Form: Größe, Kantenweiche, Rundheit, Winkel, Vorschau | n | W | S | 0 |
| 12 | `edit_sensitivity` | Register Empfindlichkeit: Druck/Geschwindigkeit je Eigenschaft | n | W | S | 0 |
| 13 | `edit_apply` | geänderte Einstellung wirkt auf nächsten Strich | r | E | E | 1 |
| 14 | `edit_cancel` | Abbrechen verwirft | r | J | D | 0 |
| 15 | `save_stroke` | „Pinsel speichern…“ in den Kategorie-Optionen | o | W | Fi | 1 |
| 16 | `custom_marker` | geänderter Pinsel zeigt Kennzeichen (z. B. „Benutzerdef.“) | n | W | S | 1 |
| 17 | `tool_vs_object` | Werkzeugvorgabe vs. am Objekt gespeicherter Strich | r | J | D | 0 |
| 18 | `remember` | Einstellung nach Werkzeugwechsel/Neustart gemerkt | n | W | S | 0 |
| 19 | `preview_swatch` | Strichvorschau im Inspektor | n | B | B | 1 |
| 20 | `disabled_states` | graue Felder je Strichart | d | W | S | 1 |

### 03 · `tool-pencil-eraser` – Buntstift, Radiergummi
**Test:** T01, T02. **Abh.:** PINSEL-1 (gemeinsamer Malkern). **Bef.:** W2, W3, W35, L37.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `pencil_inspector` | Buntstift-Inspektor (Anti-Alias, Autom. löschen, Transparenz, Deckkraft, Modus) | n | W | S | 1 |
| 02 | `pencil_stroke` | 1-px-Strich ohne Glättung | r | E | E | 1 |
| 03 | `pencil_aa` | Anti-Alias an | r | E | E | 1 |
| 04 | `pencil_autoerase` | Autom. löschen: Start auf Strichfarbe → Füllfarbe | r | E | E | 1 |
| 05 | `pencil_live` | Strich live beim Ziehen | p | B | B | 1 |
| 06 | `pencil_cursor` | Mauszeiger | n | B | B | 1 |
| 07 | `eraser_inspector` | Radiergummi: Form rund/eckig (gedrückter Knopf), Größe 13, Rand 2, Deckkraft | n, p | W, B | S, B | 2 |
| 08 | `eraser_round` | rund, Größe 13 | r | E | E | 1 |
| 09 | `eraser_square` | eckig | r | E | E | 1 |
| 10 | `eraser_edge` | Rand 0/50 | r | E | E | 2 |
| 11 | `eraser_opacity` | Deckkraft 50 % | r | E | E | 1 |
| 12 | `eraser_vector` | Radieren bei Vektorauswahl (Meldung / Umwandlung?) | r | J, B | D, B | 1 |
| 13 | `eraser_selection` | nur innerhalb Auswahlrahmen | r | E | E | 1 |
| 14 | `eraser_live` | live beim Ziehen | p | B | B | 1 |
| 15 | `eraser_cursor` | Mauszeiger je Form | n | B | B | 2 |
| 16 | `undo` | Verlaufsschritte | r | J | H | 0 |

### 04 · `tool-retouch` – Weichzeichner, Scharfzeichner, Abwedler, Nachbelichter, Verwischen
**Test:** T02 (B1 gewählt). **Bef.:** W4–W6, L38; PARITAET_A „Abwedler/Nachbelichter, Verwischen offen“.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `blur_inspector` | Weichzeichner 13/100/50, Form | n | W | S | 1 |
| 02 | `blur_result` | ein Zug über Kante | r | E | E | 1 |
| 03 | `blur_intensity` | Intensität 10/100 | r | E | E | 2 |
| 04 | `sharpen_result` | Scharfzeichner ein Zug | r | E | E | 1 |
| 05 | `dodge_inspector` | Abwedler 13/100/50, Bereich-Liste | n, o | W | S, Fi | 2 |
| 06 | `dodge_ranges` | Schatten/Mitteltöne/Lichter | r | E | E | 3 |
| 07 | `burn_ranges` | Nachbelichter je Bereich | r | E | E | 3 |
| 08 | `smudge_inspector` | Verwischen 22/50/50, Farbfeld, „Farbe verwischen“ | n | W | S | 1 |
| 09 | `smudge_result` | ein Zug | r | E | E | 1 |
| 10 | `smudge_color` | mit Farbe | r | E | E | 1 |
| 11 | `smudge_all_layers` | „Alle Ebenen verwenden“ | r | E | E | 1 |
| 12 | `repeat_stroke` | zweiter Zug verstärkt | r | E | E | 1 |
| 13 | `shape_square` | eckige Form | r | E | E | 1 |
| 14 | `edge` | Rand 0 vs 100 | r | E | E | 2 |
| 15 | `vector_target` | ohne Bitmap: Verhalten/Meldung | r | J, B | D, B | 1 |
| 16 | `live` | Wirkung während des Ziehens | p | B | B | 2 |
| 17 | `cursor` | Mauszeiger je Werkzeug | n | B | B | 2 |
| 18 | `undo` | ein Zug = ein Schritt | r | J | H | 0 |

### 05 · `tool-stamp-replace-redeye` – Stempel, Farbe ersetzen, Rote Augen entfernen
**Test:** T02. **Bef.:** W7–W10, W28, L39; PARITAET_A „Farbe ersetzen offen“.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `stamp_inspector` | Stempel 16/50, Quelle ausgerichtet, Alle Ebenen, Deckkraft, Modus | n | W | S | 1 |
| 02 | `stamp_source` | Alt-Klick Quelle, Anzeige der Quelle | p | B | B | 2 |
| 03 | `stamp_aligned` | ausgerichtet an/aus | r | E | E | 2 |
| 04 | `stamp_live` | live mit Quellkreuz | p | B | B | 1 |
| 05 | `stamp_no_source` | ohne Quelle: Verhalten | r | J, B | D, B | 1 |
| 06 | `replace_inspector` | Größe 13/Toleranz 32/Stärke 200, „Von“-Farbfeld, „Farben ersetzen/Farbton/Farbe/Helligkeit“ | n, o | W | S, Fi | 2 |
| 07 | `replace_result` | Rot → Blau über B1 | r | E | E | 1 |
| 08 | `replace_tolerance` | Toleranz 0/100 | r | E | E | 2 |
| 09 | `replace_from_pick` | „Von“ aus Pipette/Farbfeld | o | B | B | 1 |
| 10 | `redeye_inspector` | Toleranz 50/Stärke 50, zweizeiliger Kopf (W10) | n | W, B | S, B | 1 |
| 11 | `redeye_result` | Zug über rote Kreise | r | E | E | 1 |
| 12 | `redeye_tolerance` | Toleranz 10/100 | r | E | E | 2 |
| 13 | `cursor` | Mauszeiger je Werkzeug | n | B | B | 3 |
| 14 | `vector_target` | ohne Bitmap | r | J | D | 1 |
| 15 | `undo` | Verlaufsschritte | r | J | H | 0 |

### 06 · `tool-bitmap-selection` – Auswahlrahmen, Ellipse, Lasso, Polygonlasso, Zauberstab, Bitmapmodus
**Test:** T02, T03. **Abh.:** BM-1 (Grenzen bei T03). **🎨:** KI-Zauberstäbe/KI-Karte (eigene Funktion, N12/N21). **Bef.:** W22, W24, W30, W36, L13, N26.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `marquee_inspector` | Stil Normal/Fest/Feste Größe, ↔/↕ grau, Rand Hart/Glätten/Weich + Wert | n, o, d | W | S, Fi | 3 |
| 02 | `marquee_draw` | Rahmen aufziehen, „wandernde Ameisen“ | r | J (Grenzen), B | D, B | 1 |
| 03 | `marquee_shift_alt` | Umschalt = Quadrat/addieren, Alt = von Mitte/abziehen, Umschalt+Alt = Schnitt | r | J | D | 3 |
| 04 | `marquee_fixed` | feste Größe 100 × 50 | r | J | D | 1 |
| 05 | `marquee_move` | Rahmen innen verschieben, Pfeiltasten | r | J | D | 1 |
| 06 | `ellipse` | Ellipse, Glätten | r | E | E | 1 |
| 07 | `lasso` | Lasso frei | r | J, B | D, B | 1 |
| 08 | `poly_lasso` | Polygonlasso, Doppelklick schließt, Esc | r | J | D | 1 |
| 09 | `wand_inspector` | Zauberstab 32 / Hart / Live ☑ | n | W | S | 1 |
| 10 | `wand_click` | Klick, Toleranz 0/32/100 | r | J, B | D, B | 3 |
| 11 | `wand_add` | Umschalt-Klick addiert | r | J | D | 1 |
| 12 | `bitmap_mode` | Bitmapmodus Rahmen um Bitmap, Strg+E/Strg+Umschalt+D | n | B | B | 1 |
| 13 | `bitmap_bounds` | Rahmen um T03-Bitmap (nur Motiv?) | n | J (Grenzen), B | D, B | 1 |
| 14 | `delete_fill` | Entf in Auswahl → transparent | r | E | E | 1 |
| 15 | `copy_paste` | Strg+C/V aus Auswahl → neue Bitmap, Lage | r | J | D | 1 |
| 16 | `drag_move` | Ziehen mit Zeiger verschiebt Pixel | r | E | E | 1 |
| 17 | `live_marquee` | „Live-Auswahlrahmen“ an/aus | n | B | B | 2 |
| 18 | `esc` | Esc hebt Auswahl auf / verlässt Bitmapmodus | r | J | D | 0 |
| 19 | `feather_edge` | Rand weich 10 → Löschen | r | E | E | 1 |
| 20 | `cursor` | Mauszeiger (+/−/∩ bei Modifikatoren) | n | B | B | 4 |
| 21 | `empty_doc` | Zauberstab/Rahmen ohne Bitmap | r | J | D | 0 |
| 22 | `undo` | Auswahl ändern = Verlaufsschritt? | r | J | H | 0 |

### 07 · `tool-fill-gradient-eyedropper` – Farbeimer, Verlauf, Pipette
**Test:** T02, T04. **Abh.:** FARB-1b (Farbfelder im Inspektor). **Bef.:** W21, W25, W26, W31, W32; NACHPRUEFUNG FV-16 (Verlauf ziehen im Original).

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `bucket_inspector` | Toleranz 32, Auswahl füllen, Transparenz beibehalten, Deckkraft, Modus | n | W | S | 1 |
| 02 | `bucket_bitmap` | Klick in B1, Toleranz 0/32/100 | r | E | E | 3 |
| 03 | `bucket_vector` | Klick auf Vektor = Füllfarbe setzen | r | J | D | 1 |
| 04 | `bucket_fill_selection` | „Auswahl füllen“ mit Rahmen | r | E | E | 1 |
| 05 | `bucket_edge` | Rand Hart/Glätten/Weich | r | E | E | 2 |
| 06 | `gradient_inspector` | Füllfarbfeld zeigt Verlauf, Toleranz grau | n, d | W, B | S, B | 1 |
| 07 | `gradient_drag_bitmap` | Verlauf ziehen in Bitmap-Auswahl | r | E | E | 1 |
| 08 | `gradient_drag_vector` | Verlauf ziehen auf Vektor: Griffe (Start/Ende/Breite) | r | J, B | D, B | 2 |
| 09 | `gradient_types` | Linear/Radial/Kegel/… Liste | o | W | Fi | 1 |
| 10 | `eye_inspector` | Pipette 1 Pixel / 3 × 3 / 5 × 5, HEX | n, o | W | S | 1 |
| 11 | `eye_stroke_fill` | Ziel Strich bzw. Füllung je aktivem Farbfeld | r | J | D | 1 |
| 12 | `eye_average` | 3 × 3 Mittelwert | r | J | D | 0 |
| 13 | `eye_outside` | Klick außerhalb des Dokuments | r | J | D | 0 |
| 14 | `cursor` | Mauszeiger je Werkzeug | n | B | B | 3 |
| 15 | `texture_none` | Textur bei „keiner“: Name mit 0 % (W21) | n | W | S | 1 |
| 16 | `undo` | Verlaufsschritte | r | J | H | 0 |

### 08 · `tool-pointer-transform-crop` – Zeiger, Unterauswahl, Hintere auswählen, Skalieren, Verzerren, Verkrümmen, Zuschneiden, Exportbereich
**Test:** T04. **Bef.:** W22, W38, L1–L35 (Fang, Eckgriffe), N04, PARITAET_A „Skalieren/Verzerren/Drehen mit Modifikatoren ungeprüft“.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `pointer_click` | Klick wählt, Hover-Rahmen rot | h, r | B | B | 2 |
| 02 | `pointer_rubber` | Gummiband (berührt/umschließt) | r | J | D | 1 |
| 03 | `pointer_drag` | Ziehen, Umschalt 45°, Alt = Kopie, Vorschau nur in Leinwand (N04) | p | J, B | D, B | 2 |
| 04 | `pointer_corner` | Eckgriff ändert Größe | r | J | D | 1 |
| 05 | `nudge` | Pfeil 1 px, Umschalt+Pfeil 10 px | r | J | D | 0 |
| 06 | `subselect` | Punkte zeigen, Punkt ziehen, Henkel | r | J, B | D, B | 2 |
| 07 | `subselect_group` | Unterauswahl in Gruppe | r | J | D | 0 |
| 08 | `select_behind` | Hintere auswählen: zweiter Klick tiefer | r | J | D | 0 |
| 09 | `scale_handles` | Skalieren: 8 Griffe + Drehpunkt | n | B | B | 1 |
| 10 | `scale_shift` | Umschalt proportional, Alt von Mitte | r | J | D | 2 |
| 11 | `rotate` | Drehen außerhalb, Umschalt 15° | r | J | D | 1 |
| 12 | `pivot` | Drehpunkt verschieben | r | J | D | 0 |
| 13 | `skew` | Verzerren | r | J | D | 1 |
| 14 | `distort` | Verkrümmen | r | J | D | 1 |
| 15 | `transform_enter_esc` | Enter übernimmt, Esc verwirft, Doppelklick | r | J | D | 0 |
| 16 | `transform_bitmap` | Skalieren einer Bitmap: Neuberechnung | r | E | E | 1 |
| 17 | `crop_frame` | Zuschneiden: Rahmen, Griffe, Abdunkeln | n | B | B | 1 |
| 18 | `crop_apply` | Doppelklick/Enter → Leinwand, Esc | r | J | D | 1 |
| 19 | `crop_bitmap` | Zuschneiden bei gewählter Bitmap = Bitmap beschneiden | r | J | D | 1 |
| 20 | `export_area` | Exportbereich → Exportvorschau öffnet | o | B | B | 1 |
| 21 | `inspector_values` | Inspektor B/H/X/Y während Ziehen | p | W | S | 1 |
| 22 | `cursor` | Mauszeiger (Bewegen, Griffe, Drehen) | n | B | B | 6 |
| 23 | `snap` | Fang an Objektkanten/-mitten | r | J | D | 0 |
| 24 | `undo` | Verlaufsschritte und Namen | r | J | H | 0 |

### 09 · `tool-line-pen` – Linie, Stift, Vektorpfad, Pfad neu zeichnen
**Test:** T01, T04 (P1). **Bef.:** W20, W37, L31–L35, BEFUNDE LW „Stift-Doppelklick verwirft den Pfad (hoch)“.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `line_draw` | Linie, Umschalt 45° | r | J | D | 1 |
| 02 | `line_inspector` | Inspektor Linie | n | W | S | 1 |
| 03 | `pen_corner` | Klicks = Eckpunkte, Vorschau-Gummiband | p, r | J, B | D, B | 2 |
| 04 | `pen_curve` | Ziehen = Kurvenpunkt mit Henkeln | r | J | D | 1 |
| 05 | `pen_close` | Klick auf Start schließt (Zeigersymbol) | r | J, B | D, B | 1 |
| 06 | `pen_dblclick` | Doppelklick beendet offenen Pfad | r | J | D | 0 |
| 07 | `pen_esc_enter` | Esc / Enter beenden | r | J | D | 0 |
| 08 | `pen_continue` | Pfad am Endpunkt fortsetzen | r | J | D | 0 |
| 09 | `pen_alt_handle` | Alt-Ziehen am Punkt: nur Ausgangshenkel | r | J | D | 1 |
| 10 | `pen_add_delete` | Klick auf Segment = Punkt einfügen, auf Punkt = löschen? | r | J | D | 0 |
| 11 | `pen_snap` | Fang an Objektmitte (L35) | r | J | D | 0 |
| 12 | `vector_path` | Vektorpfad freihand, Genauigkeit 5 | r | J | D | 1 |
| 13 | `vector_path_brush` | Vektorpfad mit Pinselstrichart | r | E | E | 1 |
| 14 | `redraw_path` | Pfad neu zeichnen über P1 | r | J | D | 1 |
| 15 | `points_display` | Darstellung Punkte (gefüllt/hohl, Größe) | n | B | B | 2 |
| 16 | `cursor` | Mauszeiger (Stift, Schließen, Fortsetzen, Punkt +/−) | n | B | B | 6 |
| 17 | `stroke_defaults` | neue Pfade: Strich/Füllung aus Werkzeug | r | J | D | 0 |
| 18 | `undo` | Verlaufsschritte | r | J | H | 0 |

### 10 · `tool-shapes` – Rechteck, Ellipse, Polygon
**Test:** T01. **Bef.:** W19, W22, W23, BEFUNDE LW „Rechteck mit Umschalt/Alt und Fang am Startpunkt (hoch)“, „Eckgriff am Rechteck (hoch)“.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `rect_draw` | Rechteck aufziehen, Vorschau | p, r | J, B | D, B | 2 |
| 02 | `rect_shift` | Umschalt = Quadrat | r | J | D | 0 |
| 03 | `rect_alt` | Alt = von Mitte | r | J | D | 0 |
| 04 | `rect_click` | Einzelklick (kein Objekt / Standardgröße?) | r | J | D | 0 |
| 05 | `rect_arrows` | Pfeiltasten/Esc beim Aufziehen | r | J | D | 0 |
| 06 | `rect_corner_handles` | Eckrundungs-Griffe (Unterauswahl) | n, p | B | B | 2 |
| 07 | `rect_inspector` | Rundung im Werkzeug (unsichtbar, W23) | n | W | S | 1 |
| 08 | `ellipse` | Ellipse, Umschalt Kreis | r | J | D | 1 |
| 09 | `polygon_inspector` | Polygon/Stern, Seiten, Winkel, Autom. | n, o, d | W | S | 2 |
| 10 | `polygon_draw` | Polygon 6 aufziehen, Drehung beim Ziehen | r | J | D | 1 |
| 11 | `star_draw` | Stern, Winkel 38 / Autom. | r | J | D | 1 |
| 12 | `fill_stroke` | Füllung/Strich aus Vorgaben | r | J | D | 0 |
| 13 | `snap_start` | Fang am Startpunkt (Raster/Hilfslinie) | r | J | D | 0 |
| 14 | `cursor` | Mauszeiger | n | B | B | 3 |
| 15 | `info_live` | Info-Bedienfeld während des Ziehens | p | W | S | 1 |
| 16 | `undo` | Verlaufsschritt „Rechteck“ usw. | r | J | H | 0 |

### 11 · `tool-autoshapes` – automatische Formen und Griffe
**Test:** T01. **Bef.:** W27. **Formen:** Pfeil, Pfeillinie, Abgeschrägtes Rechteck, Abgefastes Rechteck, Verbindungslinie, Ring, L-Form, Tortenstück, Abgerundetes Rechteck, Intelligentes Polygon, Spirale, Stern.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `flyout` | Ausklappliste: Reihenfolge, Symbole, Namen | o | B, W | B, Fi | 1 |
| 02 | `draw_each` | jede Form mit gleichem Ziehweg (100,100 → 220,200) | r | J (Pfadpunkte), E | D, E | 12 |
| 03 | `handles` | gelbe Rauten-Griffe je Form | n | B | B | 6 |
| 04 | `handle_tooltip` | Tooltip über Griff | h | B | B | 2 |
| 05 | `handle_drag` | Griff ziehen (Rundung, Speichen, Windungen) | r | J | D | 3 |
| 06 | `handle_modifiers` | Alt/Strg/Umschalt-Klick auf Griffe | r | J | D | 0 |
| 07 | `inspector_head` | Kopf = Formname | n | W | S | 1 |
| 08 | `properties_panel` | Autoform-Eigenschaften synchron | n | W | S | 1 |
| 09 | `ungroup` | Gruppierung aufheben → normale Pfade | r | J | D | 0 |
| 10 | `scale` | Skalieren erhält Autoform? | r | J | D | 0 |
| 11 | `connector` | Verbindungslinie an Objekte | r | J | D | 1 |
| 12 | `smart_polygon` | Seitenzahl per Griff | r | J | D | 0 |
| 13 | `spiral` | Windungen | r | J | D | 0 |
| 14 | `shift` | Umschalt beim Aufziehen | r | J | D | 0 |
| 15 | `cursor` | Mauszeiger über Griff | h | B | B | 1 |
| 16 | `undo` | Verlaufsschritte | r | J | H | 0 |

### 12 · `tool-text` – Textwerkzeug auf der Leinwand
**Test:** T01, T05. **Bef.:** W14–W18, T1–T6, O4–O8, N13–N15, BEFUNDE LW „Textwerkzeug: Schrift/Farbe, Zeilenabstand 120 %, 2 px Rand, Lage nach Esc“, PARITAET_A „Kanten, Rücktaste, Pfeiltasten, Textfluss“ offen. **🎨:** Schriftsuche N19, Bildlaufleiste N24.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `click_text` | Klick → Punkttext, Einfügemarke, Rahmen | f | B | B | 1 |
| 02 | `drag_frame` | Ziehen → fester Rahmen, Umbruch | r | J, B | D, B | 1 |
| 03 | `typing` | „Abc 123“ tippen, Rahmen wächst | r | J, E | D, E | 1 |
| 04 | `esc` | Esc beendet, Werkzeug danach | r | J | D, S | 0 |
| 05 | `click_out` | Klick außerhalb beendet / neuer Text | r | J | D | 0 |
| 06 | `edit_existing` | Doppelklick auf Text = bearbeiten | f | B | B | 1 |
| 07 | `select_text` | Ziehen/Doppelklick/Umschalt+Pfeil markiert | f | B | B | 2 |
| 08 | `arrows` | Pfeile, Pos1/Ende, Strg+Pfeil | f | J | D | 0 |
| 09 | `backspace` | Rücktaste/Entf | r | J | D | 0 |
| 10 | `enter` | Eingabe = Absatz | r | J | D | 0 |
| 11 | `size_live` | Größe ändern während Bearbeitung (T1, T6) | r | J, E | D, E | 1 |
| 12 | `font_live` | Schrift wechseln während Bearbeitung | r | E | E | 1 |
| 13 | `color_live` | Farbe markierter Zeichen | r | E | E | 1 |
| 14 | `bounds` | Rahmen 2 px um Text (O5) | n | J | D | 1 |
| 15 | `frame_resize` | Rahmengriff ziehen → Umbruch | r | J | D | 1 |
| 16 | `overflow` | Text länger als Rahmen | r | B | B | 1 |
| 17 | `vertical` | vertikale Richtung | r | E | E | 1 |
| 18 | `aa_render` | Glättung (Ohne/Scharf/Stark/Glatt) Pixel | r | E | E | 4 |
| 19 | `cursor` | I-Balken, Rahmen-Zeiger | n | B | B | 2 |
| 20 | `undo` | Tippen = ein Schritt? | r | J | H | 0 |

### 13 · `tool-path-reshape` – Freiform, Bereich umformen, Pfadausbesserung ±, Messer
**Test:** T04 (P1, R1). **Bef.:** W11–W13, W28.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `freeform_inspector` | Größe 72, Druck, Vorschau | n | W | S | 1 |
| 02 | `freeform_push` | Schieben von außen (Kreiszeiger) | r | J, B | D, B | 2 |
| 03 | `freeform_pull` | Ziehen am Pfad (Haken-Zeiger) | r | J | D | 1 |
| 04 | `freeform_size_keys` | Größe mit Pfeiltasten / [ ] | r | W | S | 0 |
| 05 | `reshape_area` | Bereich umformen Größe 72/Stärke 80 | r | J | D | 1 |
| 06 | `scrubber_add` | Pfadausbesserung additiv | r | J | D | 1 |
| 07 | `scrubber_sub` | subtraktiv | r | J | D | 1 |
| 08 | `knife_cut` | Messer über R1 → zwei offene Pfade | r | J | D | 1 |
| 09 | `knife_shift` | Messer mit Umschalt (gerade) | r | J | D | 0 |
| 10 | `knife_no_selection` | ohne Auswahl: keine Wirkung | r | J | D | 0 |
| 11 | `rect_to_path` | Rechteck wird zum Pfad (Meldung?) | r | J, B | D, B | 1 |
| 12 | `inspector_heads` | Köpfe zweizeilig (W28) | n | B | B | 3 |
| 13 | `cursor` | Mauszeiger je Werkzeug | n | B | B | 5 |
| 14 | `undo` | Verlaufsschritte | r | J | H | 0 |

### 14 · `tool-web` – Hotspots, Segment, Polygonsegment, Ein-/Ausblenden
**Test:** T06. **Bef.:** W28, W33, W34, O11, O17, BEFUNDE AU `addNewHotspot("slice")`, LW „Segmente/Hotspots unsichtbar“.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `hotspot_rect` | Hotspot-Rechteck aufziehen, Farbe cyan | r | J, B | D, B | 1 |
| 02 | `hotspot_circle` | Kreis | r | J | D | 1 |
| 03 | `hotspot_polygon` | Polygon per Klicks | r | J | D | 1 |
| 04 | `slice_draw` | Segment, Farbe grün, Hilfslinien rot | r | J, B | D, B | 1 |
| 05 | `slice_auto_name` | automatischer Name `_r2_c2` | n | J | D | 0 |
| 06 | `poly_slice` | Polygonsegment | r | J | D | 1 |
| 07 | `slice_guides_drag` | Segmenthilfslinie ziehen → Nachbarsegmente | r | J | D | 1 |
| 08 | `overlay_dim` | übrige Fläche aufgehellt | n | B | B | 1 |
| 09 | `hide_show` | Segmente ein/aus (Knöpfe, Kürzel 2) | n, p | B | B | 2 |
| 10 | `web_layer` | Objekte landen in Webebene | r | J | D | 0 |
| 11 | `behavior_handle` | Verhaltensgriff (Mitte) + Ziehen auf Segment | n, o | B | B | 2 |
| 12 | `insert_from_selection` | Bearbeiten ▸ Einfügen ▸ Segment/Hotspot aus Auswahl (eins/mehrere) | o, r | J | D | 1 |
| 13 | `inspector` | Inspektor nach dem Zeichnen | n | W | S | 2 |
| 14 | `cursor` | Mauszeiger | n | B | B | 4 |
| 15 | `undo` | Verlaufsschritte | r | J | H | 0 |

### 15 · `toolbox` – Werkzeugleiste: Gruppen, Ausklapplisten, Tooltips, Hand, Zoom, Ansichtsmodi
**Test:** T10. **Bef.:** L7, L25, K1–K11, N03, BEFUNDE LW „Strg+Leertaste-Zoom, Zoomrahmen, Mausrad“. **🎨:** Symbolstil/Design DES-1 (Inhalt, Reihenfolge und Gruppen müssen gleich sein).

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `groups` | Gruppen Auswahl/Bitmap/Vektor/Web/Farben/Ansicht, Reihenfolge | n | B, W | B, Fi | 1 |
| 02 | `flyouts` | jede Ausklappliste (Dreieck, Inhalt, Kürzel rechts) | o | B | B | 12 |
| 03 | `tooltips` | Tooltip-Text mit Kürzel | h | W, B | Fi, B | 3 |
| 04 | `pressed` | gewähltes Werkzeug gedrückt | p | B | B | 1 |
| 05 | `letter_cycle` | Buchstabe wiederholt → nächstes Werkzeug der Gruppe | r | J | S | 0 |
| 06 | `hand` | Hand ziehen, Leertaste = Hand temporär | r | J | S | 0 |
| 07 | `hand_dblclick` | Doppelklick Hand = einpassen | r | J | S | 0 |
| 08 | `zoom_click` | Zoom Klick/Alt-Klick Stufen | r | J | S | 0 |
| 09 | `zoom_rect` | Zoomrahmen (Rand −17 px, L25) | r | J | S | 0 |
| 10 | `zoom_dblclick` | Doppelklick Zoom = 100 % | r | J | S | 0 |
| 11 | `ctrl_space` | Strg+Leertaste+Klick (L7) | r | J | S | 0 |
| 12 | `wheel` | Mausrad/Strg+Rad (N03) | r | J | S | 0 |
| 13 | `view_modes` | Standard/Vollbild mit Menü/Vollbild (F) | r | B | B | 3 |
| 14 | `color_section` | Strich/Füllung-Farbfelder, Tauschen, Standard, Ohne → Einheit 31 | n | B | B | 1 |
| 15 | `cursor` | Hand/Zoom +/− | n | B | B | 3 |
| 16 | `collapse` | Werkzeugleiste einspaltig/zweispaltig, lösen | n | B | B | 2 |
| 17 | `ai_tools` | KI-Werkzeuge (🎨, nur vorhanden/erreichbar) | n | – | Fi | 0 |
| 18 | `disabled` | ohne Dokument: Werkzeuge grau? | d | W, B | S, B | 1 |

## B. Inspektor

### 16 · `inspector-document` – ohne Auswahl
**Test:** T01, T09. **Bef.:** W29, W38, O2.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `layout` | Kopf „Dokument“, Felder, Lage | n | W, B | S, B | 1 |
| 02 | `canvas_color` | Leinwandfarbe-Feld → Popup | o | B | B | 1 |
| 03 | `canvas_size_btn` | „Leinwandgröße…“ öffnet D20 | o | W | Fi | 1 |
| 04 | `image_size_btn` | „Bildgröße…“ öffnet D19 | o | W | Fi | 1 |
| 05 | `fit_canvas` | „Leinwand anpassen“ | r | J | D | 1 |
| 06 | `size_label` | Anzeige Größe/Auflösung | n | W | S | 0 |
| 07 | `export_format` | Exportformat-Liste = Optimieren | o | W | Fi | 1 |
| 08 | `optimize_sync` | Wechsel → Optimieren-Bedienfeld folgt | r | J | S | 0 |
| 09 | `state_delay` | Status-Verzögerung (bei mehreren Status) | n | W | S | 1 |
| 10 | `thumbnail` | Vorschaubild | n | B | B | 1 |
| 11 | `collapsed` | Inspektor eingeklappt/halbe Höhe | n | B | B | 2 |
| 12 | `tool_inspector` | Hand/Zoom/Zeiger zeigen Dokument | n | W | S | 0 |
| 13 | `no_document` | ohne Dokument: leer | n | B | B | 1 |
| 14 | `undo` | Leinwandfarbe = Verlaufsschritt | r | J | H | 0 |

### 17 · `inspector-path-rect` – Rechteck, Pfad, Ellipse
**Test:** T04. **Bef.:** O1–O3, F12–F15, W20.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `rect_layout` | Rechteck: Felder, Reihenfolge | n | W, B | S, B | 1 |
| 02 | `path_layout` | Pfad: Felder (keine Rundung) | n | W, B | S, B | 1 |
| 03 | `ellipse_layout` | Ellipse | n | W | S | 1 |
| 04 | `xywh` | B/H/X/Y tippen → Objekt | r | J | D | 0 |
| 05 | `roundness` | Rundung + Einheit % / px | o, r | J | D | 1 |
| 06 | `opacity` | Deckkraft | r | J | D | 0 |
| 07 | `blend` | Mischmodus-Liste vollständig | o | W | Fi | 1 |
| 08 | `stroke_position` | Strichposition innen/mitte/außen | o, r | J, E | D, E | 3 |
| 09 | `fill_over_stroke` | „Füllung über Strich“ | r | E | E | 1 |
| 10 | `path_ops` | Pfadoperationen-Knöpfe (Vereinen, Schnitt, …) | n, d | W, B | S, B | 1 |
| 11 | `style_list` | Stil-Liste | o | W | Fi | 1 |
| 12 | `filters_plus` | „+“ Live-Filter → Einheit 25 | o | B | B | 1 |
| 13 | `name_field` | Objektname-Feld (Ebenen folgt) | r | J | D | 0 |
| 14 | `thumbnail` | Vorschaubild (O2) | n | B | B | 1 |
| 15 | `multi` | 2 Objekte: gemischte Werte leer | n | W | S | 1 |
| 16 | `scale_strokes` | Skalieren mit Strich/Effekten (Voreinstellung) | r | J | D | 0 |
| 17 | `field_tooltips` | Tooltips | h | W | Fi | 2 |
| 18 | `disabled` | grau bei gesperrter Ebene | d | W | S | 1 |

### 18 · `inspector-fill` – Füllung: Kategorien, Verläufe, Kanten, Texturen, Muster
**Test:** T04 (R1). **Abh.:** FARB-1b (Popup), FARB-1a. **Bef.:** W21, NACHPRUEFUNG AU-3 (Texturname, Menge 0).

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `category` | Ohne/Einfarbig/Web-Dither/Muster/Verlauf-Liste | o | W | Fi | 1 |
| 02 | `gradient_list` | Verlaufsarten (11) | o | W | Fi | 1 |
| 03 | `gradient_result` | je Verlaufsart Bild | r | E | E | 11 |
| 04 | `gradient_editor` | Verlaufs-Popup: Farbfelder, Deckkraftfelder, Vorgaben | o | B | B | 2 |
| 05 | `gradient_stops` | Stopp hinzufügen/löschen/ziehen | r | J | D | 1 |
| 06 | `gradient_presets` | Vorgaben-Liste | o | W | Fi | 1 |
| 07 | `gradient_handles` | Griffe auf der Leinwand | n, r | J, B | D, B | 1 |
| 08 | `pattern_list` | Musterliste, „Andere…“ | o | W | Fi | 1 |
| 09 | `pattern_result` | Muster „Ader“ | r | E | E | 1 |
| 10 | `web_dither` | Web-Dither-Popup | o | B | B | 1 |
| 11 | `edge` | Kante Hart/Geglättet/Weich + Wert | o, r | E | E | 3 |
| 12 | `texture_list` | Texturliste (Breite, Einträge) | o | W, B | Fi, B | 1 |
| 13 | `texture_amount` | Menge 0/50/100 | r | E | E | 3 |
| 14 | `texture_transparent` | „Transparent“ | r | E | E | 1 |
| 15 | `texture_other` | „Andere…“ als Werkzeugvorgabe | o | W | Fi | 0 |
| 16 | `fill_options_popup` | Füllungsoptionen-Popup (Ganzes) | o | B | B | 1 |
| 17 | `color_swatch` | Farbfeld → Popup (Einheit 30) | o | B | B | 1 |
| 18 | `none_disabled` | „Ohne“: übrige Felder grau | d | W | S | 1 |
| 19 | `script_name` | Texturname per JSF lesen/setzen | r | J | R | 0 |
| 20 | `undo` | jede Änderung ein Schritt | r | J | H | 0 |
| 21 | `tool_default` | gleiche Felder im Werkzeug-Inspektor | n | W | S | 1 |
| 22 | `save_style` | Füllung bleibt bei neuem Objekt | r | J | D | 0 |

### 19 · `inspector-stroke` – Strich
**Test:** T04 (P1). **Abh.:** PINSEL-1 (Strichrenderer). **Bef.:** O3, W20, F15.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `category` | Strichkategorien-Liste | o | W | Fi | 1 |
| 02 | `type_result` | je Kategorie Grundart auf P1 | r | E | E | 12 |
| 03 | `width` | Breite 1/10/100, Klemmen | r | J, E | D, E | 2 |
| 04 | `edge` | Kante (Weichheit) | r | E | E | 1 |
| 05 | `texture` | Textur + Menge | r | E | E | 1 |
| 06 | `position` | innen/mitte/außen | r | E | E | 3 |
| 07 | `options_popup` | Strichoptionen-Popup (Ganzes) | o | B | B | 1 |
| 08 | `edit_brush` | „Pinselstrich bearbeiten…“ (wie 02-09) | o | W | Fi | 1 |
| 09 | `dash` | gestrichelte Arten | r | E | E | 1 |
| 10 | `color` | Strichfarbe → Popup | o | B | B | 1 |
| 11 | `none` | „Ohne“ | r | J | D | 0 |
| 12 | `text_stroke` | Strich an Text | r | E | E | 1 |
| 13 | `open_closed` | offen/geschlossen | r | E | E | 1 |
| 14 | `fill_over` | Füllung über Strich | r | E | E | 1 |
| 15 | `scale` | Strich beim Skalieren | r | J | D | 0 |
| 16 | `save_stroke` | „Strich speichern…“ | o | W | Fi | 1 |
| 17 | `custom` | benutzerdefinierter Strich als Kennzeichen | n | W | S | 0 |
| 18 | `disabled` | grau bei „Ohne“ | d | W | S | 1 |
| 19 | `undo` | Verlaufsschritte | r | J | H | 0 |
| 20 | `tool_default` | Werkzeug-Inspektor | n | W | S | 1 |

### 20 · `inspector-bitmap` – Bitmap
**Test:** T02, T03. **Abh.:** BM-1. **Bef.:** O10, N26.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `layout` | Felder, Kopf „Bitmap“ | n | W, B | S, B | 1 |
| 02 | `bounds` | B/H/X/Y = Objektgrenzen (T03!) | n | J, W | D, S | 1 |
| 03 | `resize` | B ändern → Neuberechnung | r | E | E | 1 |
| 04 | `opacity_blend` | Deckkraft, Mischmodus | r | E | E | 2 |
| 05 | `filters` | Live-Filter-Liste | n | W | S | 1 |
| 06 | `style` | Stil-Liste | o | W | Fi | 1 |
| 07 | `bitmap_mode` | im Bitmapmodus: Werkzeug-Inspektor | n | W | S | 1 |
| 08 | `mask` | Bitmap mit Maske: Maskenteilfeld | n | W, B | S, B | 2 |
| 09 | `thumbnail` | Vorschaubild | n | B | B | 1 |
| 10 | `transparent_click` | Klick auf transparentes Pixel wählt | r | J | D | 0 |
| 11 | `name` | Objektname | r | J | D | 0 |
| 12 | `multi` | 2 Bitmaps | n | W | S | 1 |

### 21 · `inspector-text` – Text, alle Felder
**Test:** T05. **Bef.:** W14–W18, T1–T6, O4–O8, I18N-INSPEKTOR „Knopf ab/cd, Pinselstrich-Feld, Text-Zeile 2 grau“. **🎨:** Schriftsuche N19, Bildlaufleiste N24.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `layout` | Felder, drei Zeilen, Reihenfolge | n | W, B | S, B | 1 |
| 02 | `font_list` | Schriftliste (Vorschau, zuletzt benutzte) | o | B | B | 1 |
| 03 | `style_list` | Schnitte | o | W | Fi | 1 |
| 04 | `size_slider` | Größe ▾ senkrechter Schieber 8–96 | o | B | B | 1 |
| 05 | `color` | Textfarbe → Popup | o | B | B | 1 |
| 06 | `bold_italic_underline` | F/K/U Knöpfe | p, r | E | E | 3 |
| 07 | `direction` | ab/cd Richtung-Menü | o, r | W, E | Fi, E | 2 |
| 08 | `kerning` | Unterschneidung ±, autom. | r | E | E | 1 |
| 09 | `leading` | Zeilenabstand + Einheit %/px | o, r | E | E | 1 |
| 10 | `tracking` | Laufweite | r | E | E | 1 |
| 11 | `baseline` | Grundlinie | r | E | E | 1 |
| 12 | `hscale` | Horizontalskala 100 % | r | E | E | 1 |
| 13 | `indent` | Absatzeinzug | r | E | E | 1 |
| 14 | `space_before_after` | Abstand vor/nach | r | E | E | 1 |
| 15 | `align` | 5 Ausrichtungen + vertikal | p, r | E | E | 2 |
| 16 | `antialias` | Glättungsliste | o | W | Fi | 1 |
| 17 | `stroke_row` | Pinselstrich-Feld Zeile 2 | n | W | S | 1 |
| 18 | `fill_row` | Füllung/Textur am Text | r | E | E | 1 |
| 19 | `opacity_blend` | Deckkraft/Mischmodus | r | E | E | 1 |
| 20 | `mixed` | gemischte Werte bei Teilmarkierung leer | n | W | S | 1 |
| 21 | `live` | Änderung während Bearbeitung wirkt sofort | r | B | B | 1 |
| 22 | `undo` | ein Schritt je Feld | r | J | H | 0 |
| 23 | `tool_vs_object` | Textwerkzeug-Inspektor gleich | n | W | S | 1 |
| 24 | `disabled` | Zeile 2 grau ohne Strich | d | W | S | 1 |

### 22 · `inspector-group-symbol` – Gruppe, Symbol, Instanz, Schaltfläche, Animation, Mehrfachauswahl
**Test:** T06, T04. **Bef.:** O9, O13–O16, PARITAET_A „Schaltfläche, Animation, Instanz mit Eigenschaften ungeprüft“.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `group` | „Gruppieren (2)“, Felder | n | W, B | S, B | 1 |
| 02 | `group_fill` | Füllung an Gruppe → alle | r | J | D | 0 |
| 03 | `symbol_instance` | Instanz: Kopf, Felder | n | W, B | S, B | 1 |
| 04 | `instance_props` | Symboleigenschaften (Komponenten-Felder) | n | W | S | 1 |
| 05 | `button` | Schaltfläche: Text, Link, Alt, Ziel, Zustände | n | W, B | S, B | 1 |
| 06 | `button_edit` | Doppelklick → Schaltflächen-Editor | o | B | B | 1 |
| 07 | `animation` | Animationssymbol: Bilder, Skalierung, Deckkraft, Drehung | n | W, B | S, B | 1 |
| 08 | `animation_change` | Bilderzahl ändern | r | J | D | 0 |
| 09 | `animation_path` | Bewegungspfad (grün/rot) | n | B | B | 1 |
| 10 | `multi_same` | 2 Rechtecke: Felder, gemischt leer | n | W | S | 1 |
| 11 | `multi_mixed` | Rechteck + Text | n | W | S | 1 |
| 12 | `multi_fill` | Füllfarbe des zuletzt gewählten (O15) | n | B | B | 1 |
| 13 | `multi_style` | Stil leer (O16) | n | W | S | 0 |
| 14 | `mask_group` | Maskengruppe | n | W | S | 1 |
| 15 | `edit_symbol` | Symbol bearbeiten (Doppelklick) Ansicht | o | B | B | 1 |
| 16 | `break_apart` | Symbol aufheben | r | J | D | 0 |
| 17 | `thumbnail` | Vorschaubilder | n | B | B | 1 |
| 18 | `undo` | Verlaufsschritte | r | J | H | 0 |

### 23 · `inspector-web` – Segment und Hotspot
**Test:** T06. **Bef.:** O11, O12, N23 (RAND-2a „Bearbeiten…“). 

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `slice_layout` | Segment: Typ, Format, Link, Alt, Ziel, Name | n | W, B | S, B | 1 |
| 02 | `slice_type` | Typ Vordergrundbild/Hintergrund/HTML | o, r | W | Fi | 1 |
| 03 | `slice_html_edit` | HTML-Text „Bearbeiten…“ | o | W, B | Fi, B | 1 |
| 04 | `slice_format` | Exportformat-Liste je Segment | o | W | Fi | 1 |
| 05 | `slice_link_list` | Link-Liste (URL-Bibliothek) | o | W | Fi | 1 |
| 06 | `slice_target` | Ziel-Liste (_blank …) | o | W | Fi | 1 |
| 07 | `slice_name_auto` | automatischer Name, eigener Name | r | J | D | 0 |
| 08 | `slice_color` | Segmentfarbe | o | B | B | 1 |
| 09 | `hotspot_layout` | Hotspot: Form-Liste, Farbe, Link, Alt, Ziel | n | W, B | S, B | 1 |
| 10 | `hotspot_shape` | Form wechseln | r | J | D | 0 |
| 11 | `xywh` | B/H/X/Y | r | J | D | 0 |
| 12 | `multi` | 2 Segmente | n | W | S | 1 |
| 13 | `narrow` | schmales Fenster (N23) | n | B | B | 1 |
| 14 | `undo` | Verlaufsschritte | r | J | H | 0 |

### 24 · `inspector-fields` – Feldverhalten, Popup-Schieber, Rückgängig je Feld
**Test:** T04, T05. **Bef.:** F1–F15, BEFUNDE I18N-INSPEKTOR „Popup-Schieber, Esc im Popup hebt Auswahl auf (hoch), Ziffernfelder, gemischte Werte“; PARITAET_A „Felder B, H, X, Y, Rundung, Strich, Kante, Textur, Laufweite … ungeprüft“.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `h_field` | H: „9999“, „-5“, „0“, „12,5“, „x“ | r | W, J | S, D | 0 |
| 02 | `y_field` | Y: dieselben Eingaben | r | W, J | S, D | 0 |
| 03 | `edge_field` | Kante: Eingaben + Grenzen | r | W | S | 0 |
| 04 | `texture_field` | Texturmenge | r | W | S | 0 |
| 05 | `tracking_field` | Laufweite (negativ erlaubt?) | r | W | S | 0 |
| 06 | `leading_field` | Zeilenabstand | r | W | S | 0 |
| 07 | `baseline_field` | Grundlinie | r | W | S | 0 |
| 08 | `indent_field` | Einzug, Abstände | r | W | S | 0 |
| 09 | `opacity_popup` | Popup-Schieber Deckkraft (senkrecht, Lage) | o | B | B | 1 |
| 10 | `edge_popup` | Popup-Schieber Kante | o | B | B | 1 |
| 11 | `texture_popup` | Popup-Schieber Textur | o | B | B | 1 |
| 12 | `stroke_popup` | Popup-Schieber Strichbreite | o | B | B | 1 |
| 13 | `roundness_popup` | Popup-Schieber Rundung | o | B | B | 1 |
| 14 | `popup_drag_live` | Ziehen im Schieber wirkt live | p | B | B | 2 |
| 15 | `popup_esc` | Esc im Popup: Auswahl bleibt | r | J | D | 0 |
| 16 | `popup_click_out` | Klick daneben übernimmt | r | J | D | 0 |
| 17 | `focus_ring` | Fokus/Markierung beim Klick ins Feld | f | B | B | 2 |
| 18 | `tab_order` | Tab-Folge im Inspektor | f | W | S | 0 |
| 19 | `enter_canvas` | Eingabe → Fokus zurück zur Leinwand? | f | J | S | 0 |
| 20 | `undo_each` | Rückgängig je Feld (B, X, Rundung, Strich, Kante, Textur, Text-Felder) | r | J | H | 0 |
| 21 | `mixed_empty` | gemischte Werte leer, Tippen setzt alle | r | J | D | 1 |
| 22 | `units` | Einheiten-Listen (%/px, pt) | o | W | Fi | 2 |
| 23 | `disabled_look` | graue Felder Aussehen | d | B | B | 2 |
| 24 | `hover` | Hover über Feldern/Pfeilen | h | B | B | 2 |

## C. Effekte und Filter

Pixelgenaue Ergebnisse misst **RE-FX** (`re\23-effekte.md`, entsteht) und beheben **FX-1…4**. PAR-O prüft hier nur
Dialog, Felder, Grenzen, Merken, Verhalten; Ergebnisbilder werden aus RE-FX übernommen (Verweis), nicht neu gemacht.

### 25 · `fx-stack` – Effektliste im Inspektor
**Test:** T08. **Abh.:** FX-1. **Bef.:** N01 (Stapeln beim mehrfachen Ändern), NACHPRUEFUNG NZ-1 (N01), KV-3 (Kurven/Tonwert als Live-Filter).

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `plus_menu` | „+“-Menü: Kategorien, Einträge, Reihenfolge | o | W, B | Fi, B | 2 |
| 02 | `submenus` | Untermenüs (Abschrägung und Prägung, Schatten und Glühen, Farbe anpassen, …) | o | B | B | 6 |
| 03 | `list_row` | Listenzeile: Haken, Name, Info-Knopf | n | B | B | 1 |
| 04 | `checkbox` | Haken aus → Effekt aus | r | E | E | 1 |
| 05 | `info_btn` | Info/Doppelklick öffnet Optionen | o | B | B | 1 |
| 06 | `minus` | „−“ löscht gewählten | r | J | D | 0 |
| 07 | `order_drag` | Reihenfolge ziehen | r | J | D | 1 |
| 08 | `repeat_change` | Feld 5× ändern: ein Effekt (N01) | r | J | D | 0 |
| 09 | `cancel` | Abbrechen im Popup | r | J | D | 0 |
| 10 | `options_menu` | Optionen-Menü (Alle ein/aus, Als Stil speichern, Benutzerdef. löschen…) | o | W, B | Fi, B | 1 |
| 11 | `save_style` | „Als Stil speichern“ | r | J | D | 0 |
| 12 | `multi` | Mehrfachauswahl: Effekt an allen | r | J | D | 0 |
| 13 | `group` | Effekt an Gruppe | r | E | E | 1 |
| 14 | `bitmap_menu_vs_live` | Filter-Menü (destruktiv) vs Live | r | J | D | 0 |
| 15 | `photoshop_plugins` | Fremd-Filter-Einträge/Ordner (Voreinstellung) | o | W | Fi | 1 |
| 16 | `bounds` | Objektgrenzen mit Effekt (Inspektor B/H) | n | J | D | 0 |
| 17 | `undo` | Verlaufsschritte | r | J | H | 0 |
| 18 | `defaults_memory` | gemerkte Werte beim nächsten Hinzufügen | r | J | D | 0 |

### 26 · `fx-shadow-glow` – Schlagschatten, Innerer Schatten, Glühen, Inneres Glühen, Massiver Schatten
**Test:** T08. **Abh.:** FX-1, FARB-1a (Farbe springt auf Grau, N27a), RE-FX. **Bef.:** N01, N27a, N28; Spez. `re\05-live-effekte.md` §3, §4, §6, §7.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `shadow_popup` | Schlagschatten-Popup: Felder, Lage | o | W, B | Fi, B | 1 |
| 02 | `shadow_fields` | Distanz/Farbe/Deckkraft/Weichheit/Winkel-Rad/„Objekt ausblenden“ | n | W | S | 0 |
| 03 | `shadow_limits` | Grenzen je Feld (tippen 9999/−5) | r | W | S | 0 |
| 04 | `shadow_angle` | Winkelrad ziehen, Wert | p | B | B | 1 |
| 05 | `shadow_live` | Wirkung live beim Ziehen der Schieber | p | B | B | 1 |
| 06 | `inner_shadow` | Innerer Schatten Popup/Felder | o | W, B | Fi, B | 1 |
| 07 | `glow_popup` | Glühen: Breite, Farbe, Deckkraft, Weichheit, Versatz | o | W, B | Fi, B | 1 |
| 08 | `glow_color` | Farbe im Glühen: Popup → Rot ziehen bleibt Rot (N27a) | o, r | B | B | 2 |
| 09 | `inner_glow` | Inneres Glühen | o | W, B | Fi, B | 1 |
| 10 | `solid_shadow` | Massiver Schatten: Winkel, Distanz, Farbe | o | W, B | Fi, B | 1 |
| 11 | `sliders` | Popup-Schieber je Feld (nicht linear?) | o | W | S | 2 |
| 12 | `enter_esc` | Enter/Esc/Klick daneben im Popup | r | J | D | 0 |
| 13 | `result_ref` | Ergebnisbild = RE-FX (Verweis, Hash) | r | E (RE-FX) | E | 0 |
| 14 | `text_target` | an Text | r | E | E | 1 |
| 15 | `bitmap_target` | an Bitmap mit Transparenz | r | E | E | 1 |
| 16 | `knockout` | „Objekt ausblenden“ | r | E | E | 1 |
| 17 | `stack_order` | Schatten + Glühen Reihenfolge | r | E | E | 1 |
| 18 | `remember` | zuletzt benutzte Werte | r | J | D | 0 |
| 19 | `bounds` | Grenzen mit Schatten | n | J | D | 0 |
| 20 | `undo` | ein Schritt je Änderung | r | J | H | 0 |

### 27 · `fx-bevel-emboss-fill` – Abschrägung, Prägung, Farbfüllung, übernommene Ebeneneffekte
**Test:** T08. **Abh.:** FX-2, RE-FX. **Spez.:** `re\05-live-effekte.md` §5, §8, §9.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `bevel_inner_popup` | Innere Abschrägung: Kantenform-Liste, Breite, Kontrast, Weichheit, Winkel, Knopfvorgabe | o | W, B | Fi, B | 2 |
| 02 | `bevel_shapes` | Kantenformen (Abgeflacht, Glatt, Schräg 1/2, …) | o | W, B | Fi, B | 1 |
| 03 | `bevel_button_preset` | Knopfvorgabe Erhaben/Hervorgehoben/Eingedrückt/Gedrückt | o | W | Fi | 1 |
| 04 | `bevel_outer` | Äußere Abschrägung + Farbe | o | W, B | Fi, B | 1 |
| 05 | `emboss_inner` | Innere Prägung: Breite, Kontrast, Weichheit, Winkel, Original anzeigen | o | W, B | Fi, B | 1 |
| 06 | `emboss_outer` | Äußere Prägung | o | W | Fi | 1 |
| 07 | `limits` | Grenzen je Feld | r | W | S | 0 |
| 08 | `angle` | Winkelrad | p | B | B | 1 |
| 09 | `live` | Wirkung live | p | B | B | 1 |
| 10 | `color_fill` | Farbfüllung: Farbe, Deckkraft, Mischmodus | o | W, B | Fi, B | 1 |
| 11 | `ps_effects` | übernommene Ebeneneffekte (PSD) – Anzeige, Bearbeitung | o | W, B | Fi, B | 2 |
| 12 | `result_ref` | Ergebnis = RE-FX (Verweis) | r | E (RE-FX) | E | 0 |
| 13 | `text_target` | an Text | r | E | E | 1 |
| 14 | `stack` | mit Schatten kombiniert | r | E | E | 1 |
| 15 | `remember` | gemerkte Werte | r | J | D | 0 |
| 16 | `enter_esc` | Enter/Esc | r | J | D | 0 |
| 17 | `bounds` | Grenzen | n | J | D | 0 |
| 18 | `undo` | Verlaufsschritte | r | J | H | 0 |

### 28 · `filters-adjust-color` – Farbe anpassen
**Test:** T02 (B1), T08 (Live). **Abh.:** FX-4, RE-FX; Dialogfelder zusätzlich in Einheit 61 (D41–D44). **Spez.:** `re\06-bitmap-filter.md` §4.1–4.6, §5, `re\19-kurven-tonwert.md`.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `menu_entries` | Filter ▸ Farbe anpassen: Einträge | o | W, B | Fi, B | 1 |
| 02 | `brightness_live` | Helligkeit/Kontrast als Live-Filter: Dialog | o | B | B | 1 |
| 03 | `brightness_preview` | Vorschau-Kästchen wirkt | r | B | B | 1 |
| 04 | `hue_colorize` | Farbton/Sättigung „Färben“ | r | E | E | 1 |
| 05 | `curves_live` | Kurven als Live-Filter, erneut bearbeiten | o | B | B | 1 |
| 06 | `levels_live` | Tonwert als Live-Filter | o | B | B | 1 |
| 07 | `auto_levels` | Auto-Tonwert (ohne Dialog) | r | E | E | 1 |
| 08 | `invert` | Umkehren | r | E | E | 1 |
| 09 | `color_fill` | Farbfüllung-Filter | r | E | E | 1 |
| 10 | `destructive` | Menü auf Vektor: Meldung „in Bitmap umwandeln“ | o | W, B | Fi, B | 1 |
| 11 | `selection_only` | nur Auswahlrahmen-Bereich | r | E | E | 1 |
| 12 | `repeat_last` | „Letzten Filter wiederholen“ (Strg+Alt+Umschalt+X) | r | J | D | 0 |
| 13 | `remember` | gemerkte Werte | r | J | D | 0 |
| 14 | `cancel` | Abbrechen ohne Wirkung | r | J | D | 0 |
| 15 | `result_ref` | Ergebnis = RE-FX | r | E (RE-FX) | E | 0 |
| 16 | `hist` | Histogramm-Darstellung | n | B | B | 2 |
| 17 | `eyedroppers` | Pipetten in Kurven/Tonwert | p | B | B | 2 |
| 18 | `channel_list` | Kanal-Liste | o | W | Fi | 1 |
| 19 | `live_edit_again` | Live-Filter-Info öffnet mit Werten | o | W | S | 0 |
| 20 | `undo` | Verlaufsschritt-Namen | r | J | H | 0 |

### 29 · `filters-blur-sharpen-noise` – Weichzeichnen, Scharfzeichnen, Rauschen, Sonstige
**Test:** T02, T08. **Abh.:** FX-3, FX-4, RE-FX. **Spez.:** `re\06-bitmap-filter.md` §4.7–4.16, §6, §8. Dialogfelder D45–D50 in Einheit 61.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `menu_blur` | Filter ▸ Weichzeichnen: Einträge | o | W, B | Fi, B | 1 |
| 02 | `menu_sharpen` | Scharfzeichnen | o | W | Fi | 1 |
| 03 | `menu_noise_other` | Rauschen, Sonstige (Kanten finden, In Alpha konvertieren) | o | W | Fi | 1 |
| 04 | `blur_simple` | Weichzeichnen / Stark weichzeichnen | r | E | E | 2 |
| 05 | `sharpen_simple` | Schärfen / Stark schärfen | r | E | E | 2 |
| 06 | `gauss_live` | Gauß als Live-Filter | o, r | B | B | 1 |
| 07 | `unsharp_live` | Unscharf maskieren live | o | B | B | 1 |
| 08 | `motion` | Bewegungsunschärfe Grenzen der Bitmap (DLG-6) | r | J | D | 1 |
| 09 | `radial` | Radial | r | E | E | 1 |
| 10 | `zoom_blur` | Zentriert | r | E | E | 1 |
| 11 | `noise_color` | Rauschen Farbe an/aus | r | E | E | 2 |
| 12 | `find_edges` | Kanten finden | r | E | E | 1 |
| 13 | `to_alpha` | In Alpha | r | E | E | 1 |
| 14 | `clouds` | Wolken/Differenzwolken (falls im Menü) | r | E | E | 1 |
| 15 | `selection_only` | nur Auswahl | r | E | E | 1 |
| 16 | `preview_box` | Vorschau-Kästchen | r | B | B | 2 |
| 17 | `vector_warning` | auf Vektor: Meldung | o | W | Fi | 1 |
| 18 | `transparent_edge` | Rand an transparenten Kanten | r | E | E | 1 |
| 19 | `result_ref` | Ergebnis = RE-FX | r | E (RE-FX) | E | 0 |
| 20 | `remember` | gemerkte Werte | r | J | D | 0 |
| 21 | `plugins_folder` | Zusatzfilter-Ordner (Voreinstellung) | n | W | S | 0 |
| 22 | `undo` | Verlaufsschritte | r | J | H | 0 |

## D. Farbwähler und Farbfelder

### 30 · `color-popup` – Farbwähler-Popup (hier gilt das Original, N27b)
**Test:** T04 (R1). **Abh.:** FARB-1a (Rücksprung auf Grau), FARB-1b (Popup wie Original), Messung RE-5 `re\22-ansicht-bitmap-farbe.md`. **🎨:** keine – Ausnahme von DES-1, Original gilt.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `open` | Popup-Lage zum Farbfeld, Größe | o | B | B | 1 |
| 02 | `grid` | Farbfeld-Raster (Zeilen/Spalten, Feldgröße, Reihenfolge) | o | B | B | 1 |
| 03 | `swatch_hover` | Hover über Feld: Rahmen, Vorschau, HEX-Anzeige | h | B | B | 2 |
| 04 | `pick_swatch` | Klick übernimmt und schließt | r | J | D | 0 |
| 05 | `hex_field` | HEX-Feld: Tippen, Enter, ungültige Eingabe | f, r | J | D | 1 |
| 06 | `eyedropper_live` | Pipette: Überfahren nimmt Farbe live auf (auch außerhalb des Popups/Fensters) | h | B | B | 2 |
| 07 | `eyedropper_click` | Klick übernimmt | r | J | D | 0 |
| 08 | `none_btn` | Knopf „Transparent/Ohne“ | p, r | J, B | D, B | 1 |
| 09 | `system_picker` | Knopf Systemfarbwähler öffnet Windows-Dialog | o | B | B | 1 |
| 10 | `palette_menu` | Palettenmenü ▸ (Farbfelder, Websichere, Graustufen, …) | o | W, B | Fi, B | 1 |
| 11 | `options` | Optionen (Farbmodell HEX/RGB …) | o | W | Fi | 1 |
| 12 | `rgb_fields` | RGB-Felder (falls vorhanden) | n | W | S | 0 |
| 13 | `fill_options` | „Füllungsoptionen“-Link im Popup | o | B | B | 1 |
| 14 | `esc` | Esc schließt ohne Änderung, Auswahl bleibt | r | J | D | 0 |
| 15 | `click_out` | Klick daneben | r | J | D | 0 |
| 16 | `keyboard` | Pfeiltasten im Raster? | r | J | D | 0 |
| 17 | `drag_red` | Ziehen in Feld nach Rot bleibt Rot (N27a) | p, r | B | B | 2 |
| 18 | `live_preview` | Leinwand-Vorschau beim Hover? | h | B | B | 1 |
| 19 | `recent` | zuletzt benutzte Farben? | n | B | B | 1 |
| 20 | `cursor` | Pipetten-Zeiger | h | B | B | 1 |
| 21 | `undo` | ein Schritt je Übernahme | r | J | H | 0 |
| 22 | `measure` | Maße (Popup-Größe, Feldgröße px) aus RE-5 | n | B | B | 0 |

### 31 · `color-popup-contexts` – Farbwähler an allen Stellen
**Test:** T04, T05, T08. **Abh.:** FARB-1b. **Bef.:** N27b.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `toolbox_stroke` | Werkzeugleiste Strichfarbe | o | B | B | 1 |
| 02 | `toolbox_fill` | Werkzeugleiste Füllfarbe | o | B | B | 1 |
| 03 | `toolbox_swap_default` | Tauschen, Standardfarben, Ohne | p, r | J | D | 2 |
| 04 | `toolbox_active` | aktives Feld (Strich/Füllung) Kennzeichnung | n | B | B | 1 |
| 05 | `inspector_fill` | Inspektor Füllung | o | B | B | 1 |
| 06 | `inspector_stroke` | Inspektor Strich | o | B | B | 1 |
| 07 | `inspector_text` | Textfarbe | o | B | B | 1 |
| 08 | `effect_shadow` | Effekt-Popup (Schatten) | o | B | B | 1 |
| 09 | `gradient_stop` | Verlaufsstopp | o | B | B | 1 |
| 10 | `dialog_canvas` | Dialog Leinwandfarbe/Neues Dokument | o | B | B | 1 |
| 11 | `dialog_grid` | Raster/Hilfslinien bearbeiten | o | B | B | 1 |
| 12 | `panel_mixer` | Mischer-Farbfelder | o | B | B | 1 |
| 13 | `panel_layers_color` | Ebenenfarbe/Segmentfarbe | o | B | B | 1 |
| 14 | `preview_matte` | Bildvorschau/Optimieren Hintergrund (Matte) | o | B | B | 1 |
| 15 | `tool_colors` | Werkzeug-Farbfelder (Text, Verwischen, Farbe ersetzen) | o | B | B | 2 |
| 16 | `same_popup` | überall gleiches Popup | – | B | B | 0 |
| 17 | `system_picker` | Systemfarbwähler aus jeder Stelle | o | B | B | 1 |
| 18 | `ctrl_click` | Strg/Umschalt-Klick auf Farbfeld | r | J | D | 0 |

### 32 · `panel-swatches-mixer` – Farbfelder, Mischer, Farbpalette
**Test:** T01, T12. **Abh.:** FARB-1b. **Bef.:** PARITAET_B §3 (≡-Menüs), §„Farbfelder Strg/Umschalt-Klick“, ❔ Ausrichten/Farbpalette.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `swatches_layout` | Farbfelder: Raster, Feldgröße | n | B | B | 1 |
| 02 | `swatches_click` | Klick setzt aktive Farbe | r | J | D | 0 |
| 03 | `swatches_add` | Klick in leeren Bereich fügt hinzu | r | J | D | 0 |
| 04 | `swatches_ctrl` | Strg-Klick löscht, Umschalt-Klick ersetzt | r | J | D | 0 |
| 05 | `swatches_menu` | ≡-Menü (Laden, Ersetzen, Speichern, Löschen, Sortieren, Paletten) | o | W, B | Fi, B | 1 |
| 06 | `swatches_load` | T12 hinzufügen/ersetzen | r | J | D | 1 |
| 07 | `swatches_sort` | Nach Farbe sortieren | r | J | D | 1 |
| 08 | `swatches_tooltip` | Tooltip HEX | h | B | B | 1 |
| 09 | `mixer_layout` | Mischer: Strich/Füllung, Modell, Felder, Spektrum | n | B | B | 1 |
| 10 | `mixer_models` | RGB/HEX/HSB/CMY/Graustufen | o, r | W | Fi | 5 |
| 11 | `mixer_spectrum` | Klick ins Spektrum | r | J | D | 0 |
| 12 | `mixer_fields` | Feldgrenzen/Tippen | r | W | S | 0 |
| 13 | `mixer_sliders` | Popup-Schieber je Kanal | o | B | B | 1 |
| 14 | `palette_panel` | Farbpalette: Register | n | B | B | 2 |
| 15 | `palette_harmony` | Farbharmonie/Mischen | r | J | S | 1 |
| 16 | `palette_menu` | ≡-Menü | o | W | Fi | 1 |
| 17 | `sync` | Werkzeugleiste ↔ Mischer synchron | r | J | S | 0 |
| 18 | `none` | Ohne-Farbe | r | J | D | 0 |
| 19 | `selected_object` | Farbe wirkt auf gewähltes Objekt | r | J | D | 0 |
| 20 | `undo` | Verlaufsschritte | r | J | H | 0 |

## E. Bedienfelder

### 33 · `panel-layers` – Ebenen: Zeilen, Augen, Schloss, Ziehen, Umbenennen
**Test:** T07. **Bef.:** PARITAET_B „Bedienfelder Knopf für Knopf“, UX_ABGLEICH_B E-Zeilen.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `layout` | Kopf (Deckkraft, Mischmodus), Zeilen, Symbole | n | B | B | 1 |
| 02 | `row_states` | Zeile normal/hover/gewählt/Ziel | n, h, p | B | B | 3 |
| 03 | `eye` | Auge klicken, Alt-Klick | r | J | D | 1 |
| 04 | `eye_drag` | Ziehen über Augen | r | J | D | 0 |
| 05 | `lock` | Schloss | r | J | D | 1 |
| 06 | `expand` | Ebene auf-/zuklappen, Alt-Klick alle | r | B | B | 1 |
| 07 | `select_object` | Klick auf Objektzeile wählt auf Leinwand | r | J | D | 0 |
| 08 | `shift_ctrl` | Umschalt/Strg-Mehrfachauswahl | r | J | D | 0 |
| 09 | `drag_reorder` | Objekt/Ebene ziehen, Einfügemarke | p, r | J, B | D, B | 1 |
| 10 | `drag_into_layer` | in andere Ebene ziehen | r | J | D | 0 |
| 11 | `rename` | Doppelklick → Umbenennen (Feld / Dialog) | f | B | B | 1 |
| 12 | `selection_dot` | Auswahlkennzeichen (blauer Punkt) ziehen | r | J | D | 0 |
| 13 | `thumbnails` | Vorschaubilder, Größe | n | B | B | 1 |
| 14 | `opacity_blend` | Deckkraft/Mischmodus im Kopf | r | J | D | 0 |
| 15 | `web_layer` | Webebene oben, nicht verschiebbar | n | J | D | 0 |
| 16 | `shared_layer` | gemeinsame Ebene (Symbol) | n | B | B | 1 |
| 17 | `sublayer` | Unterebene | n | B | B | 0 |
| 18 | `mask_row` | Masken-Zeile, Verknüpfung | n | B | B | 1 |
| 19 | `bitmap_mode_row` | Zeile im Bitmapmodus | n | B | B | 1 |
| 20 | `scroll` | Bildlauf bei vielen Zeilen | n | B | B | 1 |
| 21 | `tooltip` | Tooltips | h | W | Fi | 2 |
| 22 | `undo` | Verlaufsschritte | r | J | H | 0 |

### 34 · `panel-layers-menu` – Ebenen: ≡-Menü, Kontextmenü, Fußleiste, Ebenenoptionen
**Test:** T07. **Bef.:** PARITAET_B §3 Ebenen, Menü 6923.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `menu_open` | ≡-Menü: Einträge, grau/aktiv | o | W, B | Fi, B | 1 |
| 02 | `menu_states_no_sel` | ohne Auswahl | o | W | Fi | 1 |
| 03 | `context_layer` | Rechtsklick Ebene | o | W, B | Fi, B | 1 |
| 04 | `context_object` | Rechtsklick Objekt | o | W, B | Fi, B | 1 |
| 05 | `footer` | Fußleiste: Knöpfe, Tooltips | n, h | B | B | 2 |
| 06 | `new_layer` | Neue Ebene (Dialog?) | o, r | W | Fi | 1 |
| 07 | `new_sublayer` | Neue Unterebene | r | J | D | 0 |
| 08 | `duplicate` | Ebene duplizieren (Dialog Anzahl/Position) | o | W | Fi | 1 |
| 09 | `delete` | Löschen (Papierkorb, Entf) | r | J | D | 0 |
| 10 | `options` | Ebenenoptionen-Dialog | o | W, B | Fi, B | 1 |
| 11 | `share` | Ebene auf Status/Seiten teilen | r | J | D | 0 |
| 12 | `single_layer_edit` | Einzelne Ebene bearbeiten | r | J | D | 0 |
| 13 | `hide_all_show_all` | Alle aus-/einblenden | r | J | D | 0 |
| 14 | `lock_all` | Alle sperren | r | J | D | 0 |
| 15 | `merge_down` | Mit darunter reduzieren (auch bei Text) | r | J | D | 0 |
| 16 | `flatten` | Auswahl reduzieren | r | J | D | 0 |
| 17 | `thumbnail_options` | Miniaturoptionen | o | W | Fi | 1 |
| 18 | `mask_buttons` | Maske hinzufügen/bearbeiten | r | J | D | 1 |

### 35 · `panel-pages-states-history` – Seiten, Status, Verlauf
**Test:** T07, T04. **Bef.:** PARITAET_B §3 (Seiten, Status, Verlauf), NZ-4 (Status).

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `pages_layout` | Seiten: Zeilen, Vorschaubild, Nummer | n | B | B | 1 |
| 02 | `pages_menu` | ≡-Menü | o | W | Fi | 1 |
| 03 | `pages_new_dup_delete` | Neu/Duplizieren/Löschen | r | J | D | 0 |
| 04 | `pages_rename_drag` | Umbenennen, Reihenfolge ziehen | r | J | D | 0 |
| 05 | `pages_master` | Masterseite | r | J | D | 1 |
| 06 | `states_layout` | Status: Zeilen, Verzögerung, Zwiebelschicht | n | B | B | 1 |
| 07 | `states_menu` | ≡-Menü | o | W | Fi | 1 |
| 08 | `states_delay` | Verzögerung Doppelklick/Feld, Ein-/Ausschluss | o | W, B | Fi, B | 1 |
| 09 | `states_onion` | Zwiebelschicht-Menü | o, r | B | B | 2 |
| 10 | `states_loop` | Schleife / GIF-Wiederholung | o | W | Fi | 1 |
| 11 | `states_disposal` | Löschmethode | o | W | Fi | 1 |
| 12 | `states_play` | Abspielen (Statusleiste) | r | J | S | 0 |
| 13 | `states_distribute` | Auf Status verteilen | r | J | D | 0 |
| 14 | `history_layout` | Verlauf: Schritte, Schieber, Trennlinie „gespeichert“ | n | B | B | 1 |
| 15 | `history_slider` | Schieber ziehen | r | J | H | 1 |
| 16 | `history_replay` | Wiederholen-Knopf (Schritte erneut) | r | J | D | 0 |
| 17 | `history_copy` | Schritte in Zwischenablage kopieren | r | J | R | 0 |
| 18 | `history_save_cmd` | Als Befehl speichern | o | W | Fi | 1 |
| 19 | `history_unrepeatable` | nicht wiederholbare Schritte (rotes X) | n | B | B | 1 |
| 20 | `history_menu` | ≡-Menü, Löschen | o | W | Fi | 1 |
| 21 | `history_merge_nudge` | Pfeiltasten verschmolzen | r | J | H | 0 |
| 22 | `history_limit` | Anzahl Schritte (Voreinstellung) | n | J | H | 0 |

### 36 · `panel-optimize` – Optimieren
**Test:** T09. **Bef.:** PARITAET_B §3 Optimieren (MN-5), N23 (Zeilensprung/Progressiv abgeschnitten), `re\17-bildvorschau.md` §2 (gleiche Felder).

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `layout_gif` | GIF: Felder, Lage | n | W, B | S, B | 1 |
| 02 | `layout_jpeg` | JPEG: Qualität, Glättung, Zeilensprung, Progressiv | n | W, B | S, B | 1 |
| 03 | `layout_png` | PNG 8/24/32 | n | W, B | S, B | 2 |
| 04 | `preset_list` | Vorgaben-Liste | o | W | Fi | 1 |
| 05 | `format_list` | Formatliste | o | W | Fi | 1 |
| 06 | `palette_list` | Palette (Adaptiv, Websicher, …) | o | W | Fi | 1 |
| 07 | `colors_list` | Farbenzahl | o | W | Fi | 1 |
| 08 | `loss_dither` | Verlust, Rastern + Schieber | o | B | B | 1 |
| 09 | `transparency` | Transparenz-Liste | o | W | Fi | 1 |
| 10 | `matte` | Hintergrund (Matte) | o | B | B | 1 |
| 11 | `color_table` | Farbtabelle: Felder, Anzahl, Neu berechnen | n | B | B | 1 |
| 12 | `color_table_buttons` | Knöpfe unten (sperren, websicher, bearbeiten, transparent, löschen, hinzufügen) | n, d | B | B | 1 |
| 13 | `color_table_context` | Kontextmenü der Farbtabelle | o | W | Fi | 1 |
| 14 | `eyedroppers` | Transparenz-Pipetten | p | B | B | 1 |
| 15 | `menu` | ≡-Menü (Einstellungen speichern, Zielgröße, Exportassistent …) | o | W | Fi | 1 |
| 16 | `save_preset` | Einstellungen speichern | o | W | Fi | 1 |
| 17 | `optimize_to_size` | Auf Größe optimieren | o | W | Fi | 1 |
| 18 | `selective_jpeg` | Selektive Qualität | n | W | S | 1 |
| 19 | `slice_specific` | je Segment eigene Einstellung | r | J | D | 0 |
| 20 | `preview_tab_sync` | Leinwand-Vorschau-Ansicht folgt | r | B | B | 1 |

### 37 · `panel-library-styles` – Dokumentbibliothek, Allgemeine Bibliothek, Stile, URL
**Test:** T06. **Bef.:** PARITAET_B §3 Stile.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `doclib_layout` | Dokumentbibliothek: Vorschau, Liste, Spalten | n | B | B | 1 |
| 02 | `doclib_menu` | ≡-Menü | o | W | Fi | 1 |
| 03 | `doclib_drag` | Symbol auf Leinwand ziehen = Instanz | r | J | D | 0 |
| 04 | `doclib_props` | Eigenschaften (Doppelklick) | o | W | Fi | 1 |
| 05 | `doclib_footer` | Fußleiste (Neu, Löschen) | n | B | B | 1 |
| 06 | `commonlib_tree` | Allgemeine Bibliothek: Ordnerbaum | n, o | B | B | 2 |
| 07 | `commonlib_insert` | Element einfügen | r | J | D | 0 |
| 08 | `commonlib_menu` | ≡-Menü | o | W | Fi | 1 |
| 09 | `styles_grid` | Stile: Raster, Größe | n | B | B | 1 |
| 10 | `styles_apply` | Klick wendet an | r | E | E | 1 |
| 11 | `styles_new` | Neuer Stil-Dialog (Eigenschaften-Kästchen) | o | W, B | Fi, B | 1 |
| 12 | `styles_menu` | ≡-Menü (Bibliotheken, Ansicht, Löschen) | o | W | Fi | 1 |
| 13 | `styles_libraries` | Stilbibliotheken laden | o | W | Fi | 1 |
| 14 | `styles_break_link` | Verknüpfung aufheben | r | J | D | 0 |
| 15 | `url_layout` | URL: Liste, Feld, + | n | B | B | 1 |
| 16 | `url_menu` | ≡-Menü (Importieren/Exportieren) | o | W | Fi | 1 |
| 17 | `url_apply` | Link an Segment | r | J | D | 0 |
| 18 | `undo` | Verlaufsschritte | r | J | H | 0 |

### 38 · `panel-info-align-path` – Info, Ausrichten, Pfad
**Test:** T04. **Bef.:** PARITAET_B ❔ Ausrichten/Farbpalette, „Flash-Bedienfelder ohne ≡“.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `info_layout` | Info: B/H/X/Y, Farbe unter Zeiger, Zeigerlage | n | B | B | 1 |
| 02 | `info_live` | Werte live beim Bewegen/Ziehen | h | W | S | 0 |
| 03 | `info_menu` | Farbmodell/Einheit (≡) | o | W | Fi | 1 |
| 04 | `info_fields` | Eingabe B/H/X/Y | r | J | D | 0 |
| 05 | `align_layout` | Ausrichten: Gruppen, Knöpfe, „Zu Leinwand“ | n | B | B | 1 |
| 06 | `align_buttons` | je Knopf Ergebnis (Ausrichten 6, Verteilen 6, Größe 3, Abstand 2) | r | J | D | 2 |
| 07 | `align_anchor` | Ankerliste/Position | o | W | Fi | 1 |
| 08 | `align_hover` | Hover-Tooltips | h | B | B | 2 |
| 09 | `align_disabled` | grau ohne Auswahl / bei 1 Objekt | d | B | B | 1 |
| 10 | `path_layout` | Pfad: Gruppen (Formen, Pfade bearbeiten, Punkte) | n | B | B | 1 |
| 11 | `path_buttons` | Knöpfe: Vereinen, Schnitt, … Wirkung | r | J | D | 2 |
| 12 | `path_points` | Punkte-Knöpfe (Glätten, Ecke …) | r | J | D | 0 |
| 13 | `path_hover` | Tooltips | h | B | B | 1 |
| 14 | `path_disabled` | grau ohne Pfad | d | B | B | 1 |
| 15 | `undo` | Verlaufsschritte | r | J | H | 0 |
| 16 | `panel_resize` | Breite ändern: Knöpfe umbrechen | n | B | B | 2 |
| 17 | `no_doc` | ohne Dokument | d | B | B | 1 |
| 18 | `scale_to_canvas` | „Zu Leinwand“ an/aus | r | J | D | 0 |

### 39 · `panel-misc` – Autoformen, Autoform-Eigenschaften, Sonderzeichen, Bildbearbeitung, CSS, Suchen, Verhalten, Symboleigenschaften
**Test:** T04, T05, T06.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `autoshapes_grid` | Autoformen: Raster, Namen | n | B | B | 1 |
| 02 | `autoshapes_drag` | Ziehen/Doppelklick fügt ein | r | J | D | 0 |
| 03 | `autoshape_props` | Autoform-Eigenschaften je Form, Felder | n | W, B | S, B | 2 |
| 04 | `autoshape_props_edit` | Werte ändern → Form | r | J | D | 0 |
| 05 | `special_chars` | Sonderzeichen: Raster, Einfügen in Text | n, r | B | B | 1 |
| 06 | `image_editing` | Bildbearbeitung: Knöpfe = Werkzeuge/Filter | n | B | B | 1 |
| 07 | `image_editing_click` | Klick wählt Werkzeug / öffnet Filter | r | J | S | 0 |
| 08 | `css_props` | CSS-Eigenschaften: Inhalt für R1 | n | W, B | S, B | 1 |
| 09 | `css_copy` | Kopieren (alles/Auswahl) | r | J | R | 0 |
| 10 | `find_layout` | Suchen: Bereich-Liste, Art-Liste, Felder | n | W, B | S, B | 1 |
| 11 | `find_text` | Text suchen/ersetzen, Optionen | r | J | D | 0 |
| 12 | `find_font_color_url` | Schrift/Farbe/URL suchen | r | J | D | 1 |
| 13 | `behaviors_layout` | Verhalten: Liste, +/− | n | B | B | 1 |
| 14 | `behaviors_add` | + Menü, Austauschbild, Dialog | o | W, B | Fi, B | 2 |
| 15 | `behaviors_events` | Ereignis-Liste | o | W | Fi | 1 |
| 16 | `symbol_props` | Symboleigenschaften: Felder | n | W, B | S, B | 1 |
| 17 | `symbol_props_edit` | Wert ändern → Instanz | r | J | D | 0 |
| 18 | `menus` | ≡-Menüs der genannten Felder | o | W | Fi | 3 |
| 19 | `no_selection` | Zustände ohne Auswahl | d | B | B | 2 |
| 20 | `undo` | Verlaufsschritte | r | J | H | 0 |

## F. Bildvorschau und Export

### 40 · `preview-options` – Bildvorschau: Registerkarte Optionen
**Test:** T09. **Spez.:** `re\17-bildvorschau.md` §1–2, §8. **Bef.:** N22, D08, NACHPRUEFUNG §5. **🎨:** Fensterrahmen/Fußleiste DES-1.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `window` | Fenstergröße, Gliederung 1-fach | n | W, B | S, B | 1 |
| 02 | `tabs` | Register Optionen/Datei/Animation | n, p | B | B | 3 |
| 03 | `preset_list` | Vorgaben + „+“/„−“ | o | W | Fi | 1 |
| 04 | `format_list` | Formatliste (4610) | o | W | Fi | 1 |
| 05 | `fields_per_format` | Felder je Format (GIF/JPEG/PNG8/24/32/…) | n | W | S | 6 |
| 06 | `palette_list` | Palettenliste (4616) | o | W | Fi | 1 |
| 07 | `colors` | Farbenzahl | o | W | Fi | 0 |
| 08 | `loss_dither` | Verlust, Rastern | r | E | E | 1 |
| 09 | `transparency` | Transparenz + Pipetten | o, p | B | B | 2 |
| 10 | `matte` | Hintergrund | o | B | B | 1 |
| 11 | `remove_unused` | Nicht verwendete Farben entfernen | r | E | E | 0 |
| 12 | `interlace` | Zeilensprung/Progressiv | r | E | E | 0 |
| 13 | `jpeg_quality` | Qualität, Glättung, Selektiv | r | E | E | 1 |
| 14 | `color_table` | Farbtabelle + Kontextmenü | n, o | B | B | 2 |
| 15 | `options_menu` | Optionsmenü ≡ (4624) | o | W | Fi | 1 |
| 16 | `sub_dialogs` | Unterdialoge der Knöpfe (§2.8) | o | W, B | Fi, B | 2 |
| 17 | `size_time` | Größe/Ladezeit-Anzeige | n | W | S | 0 |
| 18 | `ok_cancel_export` | OK/Abbrechen/Exportieren | r | J | D | 0 |
| 19 | `remember` | Einstellungen gemerkt im Dokument | r | J | D | 0 |
| 20 | `user_preset` | Benutzervorgabe speichern/löschen | r | J | D | 1 |
| 21 | `esc` | Esc schließt | r | J | S | 0 |
| 22 | `tooltips` | Tooltips | h | W | Fi | 2 |
| 23 | `optimize_sync` | Übernahme ins Optimieren-Bedienfeld | r | J | D | 0 |
| 24 | `result_hash` | Exportdatei je Format = Hash/Größe | r | E | E | 0 |

### 41 · `preview-views` – Bildvorschau: 1/2/4-fach, synchron, Werkzeuge, Datei, Animation
**Test:** T09, T10. **Abh.:** BV-5 (synchron, N25), Messung RE-5. **Spez.:** `re\17-bildvorschau.md` §3–7.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `mode_1` | 1-fach | n | B | B | 1 |
| 02 | `mode_2` | 2-fach (Lage, Trennlinie) | n | B | B | 1 |
| 03 | `mode_4` | 4-fach | n | B | B | 1 |
| 04 | `field_select` | aktives Feld (Rahmen), eigene Einstellungen | p | B | B | 1 |
| 05 | `field_header` | Kopfzeile je Feld (Format, Größe, Zeit, Farben) | n | W, B | S, B | 1 |
| 06 | `preview_off` | „Vorschau“ aus = Original | n | B | B | 1 |
| 07 | `sync_scroll` | Bildlauf synchron | r | B | B | 2 |
| 08 | `sync_wheel` | Mausrad synchron | r | B | B | 1 |
| 09 | `sync_space` | Leertaste+Ziehen synchron | r | B | B | 1 |
| 10 | `sync_hand` | Hand-Werkzeug synchron | r | B | B | 1 |
| 11 | `sync_zoom` | Zoom synchron, Zoomliste | r, o | B | B | 2 |
| 12 | `tool_crop` | Zuschneiden-Werkzeug | r | J | D | 1 |
| 13 | `tool_pointer` | Zeiger | r | B | B | 0 |
| 14 | `file_tab` | Datei: Skalieren %, B/H, Exportbereich | n | W, B | S, B | 1 |
| 15 | `export_area` | Exportbereich-Felder/Griffe | r | J | D | 1 |
| 16 | `anim_tab` | Animation: Status-Liste, Verzögerung, Schleife | n | W, B | S, B | 1 |
| 17 | `anim_buttons` | Abspielknöpfe | p | B | B | 2 |
| 18 | `anim_crop` | Zuschneiden der Status | r | J | D | 0 |
| 19 | `resize_window` | Fenster vergrößern: Felder wachsen | n | B | B | 1 |
| 20 | `checkerboard` | Transparenz-Schachbrett | n | B | B | 1 |

### 42 · `export-results` – Export-Ergebnisse
**Test:** T09. **Spez.:** `re\08-export.md`, `re\18-export-dialoge.md`. **Bef.:** D07, D11, D13, D15, NACHPRUEFUNG E1–E3. **Verfahren:** gleiche Einstellungen, Ergebnisdateien per Hash/Größe/Pixel vergleichen.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `images_only` | Nur Bilder: Dateinamen, Anzahl | r | J (Dateiliste) | R | 0 |
| 02 | `slices_names` | Segmentnamen `_r1_c1`, `_s2` | r | J | R | 0 |
| 03 | `html_images` | HTML und Bilder: HTML-Text | r | J (Text) | R | 0 |
| 04 | `states_files` | Status als Dateien | r | J | R | 0 |
| 05 | `layers_files` | Ebenen als Dateien (Präfix/Suffix) | r | J | R | 0 |
| 06 | `pages_files` | Seiten als Dateien | r | J | R | 0 |
| 07 | `gif_anim` | animiertes GIF: Bilder, Verzögerung, Schleife | r | E | E | 1 |
| 08 | `png_variants` | PNG8/24/32 Bytes/Pixel | r | E | E | 3 |
| 09 | `jpeg` | JPEG Qualität 80 | r | E | E | 1 |
| 10 | `pdf` | PDF-Export Optionen | r | J | R | 1 |
| 11 | `css_sprites` | CSS-Sprites | r | J | R | 1 |
| 12 | `selected_slices` | nur gewählte Segmente | r | J | R | 0 |
| 13 | `export_area_result` | Exportbereich | r | E | E | 1 |
| 14 | `wizard_flow` | Exportassistent Ablauf | r | W | S | 1 |
| 15 | `html_setup_effect` | HTML-Einrichtung wirkt auf Ausgabe | r | J | R | 0 |
| 16 | `copy_html` | HTML-Code kopieren Ergebnis | r | J | R | 1 |

## G. Querschnitt

### 43 · `xs-context-menus` – Kontextmenüs
**Test:** T04–T07. **Bef.:** PARITAET_B „Reiter-Kontextmenü (Original belegt)“.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `canvas_empty` | Leinwand leer | o | W, B | Fi, B | 1 |
| 02 | `rect` | Rechteck | o | W, B | Fi, B | 1 |
| 03 | `path` | Pfad | o | W | Fi | 1 |
| 04 | `text` | Text (und im Bearbeitungsmodus) | o | W, B | Fi, B | 2 |
| 05 | `bitmap` | Bitmap / Bitmapmodus mit Auswahl | o | W, B | Fi, B | 2 |
| 06 | `group_symbol` | Gruppe, Instanz | o | W | Fi | 2 |
| 07 | `slice_hotspot` | Segment, Hotspot | o | W | Fi | 2 |
| 08 | `multi` | Mehrfachauswahl | o | W | Fi | 1 |
| 09 | `tab` | Dokumentreiter | o | W, B | Fi, B | 1 |
| 10 | `panel_group` | Bedienfeldgruppe/-titel | o | W | Fi | 1 |
| 11 | `ruler_guides` | Lineal, Hilfslinie | o | W | Fi | 2 |
| 12 | `field_text` | Textfeld (Kopieren/Einfügen) | o | W | Fi | 1 |
| 13 | `submenus` | Untermenüs (Anordnen, Transformieren, …) | o | B | B | 2 |
| 14 | `disabled` | graue Einträge je Fall | d | W | Fi | 0 |
| 15 | `effect` | Einträge wirken (Stichprobe 5) | r | J | D | 0 |
| 16 | `esc` | Esc schließt, Auswahl bleibt | r | J | D | 0 |

### 44 · `xs-shortcuts` – Tastenkürzel
**Test:** T04. **Quelle:** `commands.json` `shortcuts_de` (alle Einträge, auch 20 ohne Menü: Pfeile, Strg+E, Umschalt+./,, Strg+Bild auf/ab, Strg+Umschalt+Links/Rechts, Strg+Alt+Umschalt+S/G, Strg+Umschalt+E, Strg+F10). **Methode:** Tabelle Kürzel → Wirkung, keine Bilder außer Dialog D17.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `file` | Datei-Kürzel (Strg+N/O/S/W/Q/P, Umschalt-Varianten, Strg+R Import, Strg+Umschalt+X Bildvorschau) | r | J | S, D | 0 |
| 02 | `edit` | Strg+Z/Y/X/C/V, Strg+Umschalt+V, Strg+D, Strg+Alt+D, Strg+A, Entf | r | J | D | 0 |
| 03 | `view` | Strg++/−, Strg+0, Strg+1, Strg+Alt+0, Strg+Umschalt+H, Strg+Alt+R/; | r | J | S | 0 |
| 04 | `select` | Strg+Umschalt+A/D/I, Strg+Umschalt+Links/Rechts | r | J | D | 0 |
| 05 | `modify` | Strg+G/Umschalt+G, Strg+Umschalt+Pfeil hoch/ab, Strg+Alt+… Ausrichten, Strg+T/Umschalt+T, Strg+F8 | r | J | D | 0 |
| 06 | `text` | Strg+Umschalt+B/I/U, Strg+Umschalt+E, Strg+Alt+Umschalt+S, Größe ± | r | J | D | 0 |
| 07 | `window` | F2, F7, F8, F10, Umschalt+F2 …, Strg+F10, F4/Tab (Bedienfelder) | r | J | S | 0 |
| 08 | `tools` | Werkzeugbuchstaben V, A, Q, C, M, L, W, B, Y, E, R, K, P, U, T, N, S, I, G, H, Z, D, X, J, 2 | r | J | S | 0 |
| 09 | `tool_cycle` | Buchstabe wiederholt | r | J | S | 0 |
| 10 | `nudge` | Pfeile, Umschalt+Pfeile (Objekt, Punkt, Auswahl) | r | J | D | 0 |
| 11 | `pages_states` | Strg+Bild auf/ab, Umschalt+./, | r | J | S | 0 |
| 12 | `bitmap_mode` | Strg+E, Strg+Umschalt+D, Enter/Esc | r | J | S | 0 |
| 13 | `modifiers_drag` | Umschalt/Alt/Strg beim Ziehen (Liste) | r | J | D | 0 |
| 14 | `space_hand` | Leertaste temporär Hand, Strg+Leertaste Zoom | r | J | S | 0 |
| 15 | `in_text` | Kürzel während Textbearbeitung gesperrt? | r | J | D | 0 |
| 16 | `in_field` | Kürzel im Inspektorfeld (Strg+A markiert Feld) | r | J | D | 0 |
| 17 | `menu_display` | Anzeige im Menü (Schreibweise) | n | W | Fi | 1 |
| 18 | `dialog_d17` | Tastaturkurzbefehle-Dialog: Satz ändern, Konflikt | o | W, B | Fi, B | 2 |
| 19 | `custom_set` | eigener Satz wirkt sofort | r | J | S | 0 |
| 20 | `alt_menus` | Alt+Buchstabe öffnet Menü (Zugriffstasten) | o | W | Fi | 1 |
| 21 | `escape_chain` | Esc-Reihenfolge (Popup → Werkzeug → Auswahl) | r | J | D | 0 |
| 22 | `enter_canvas` | Enter auf Leinwand (Gruppe öffnen, Bitmapmodus) | r | J | D | 0 |

### 45 · `xs-preferences` – Voreinstellungen: Wirkung
**Test:** T04. **Bef.:** D16, BEFUNDE EB (prefs.json W8), LW „Werkzeugvorgaben aus den Voreinstellungen“. **🎨:** Firepixel-eigene Kategorien (KI, Design, Sprache).

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `categories` | Kategorienliste, Reihenfolge | n | W, B | Fi, B | 1 |
| 02 | `general_undo` | Rückgängig-Schritte | r | J | H | 1 |
| 03 | `general_color` | Standardfarben | r | J | D | 0 |
| 04 | `general_interpolation` | Interpolation (Skalieren-Ergebnis) | r | E | E | 0 |
| 05 | `general_workspace` | Startseite, Arbeitsbereich | r | J | S | 0 |
| 06 | `edit_delete_paths` | „Pfade beim Zuschneiden löschen“ usw. | r | J | D | 1 |
| 07 | `edit_pen_preview` | Stiftvorschau, gefüllte Punkte | r | B | B | 1 |
| 08 | `edit_precise_cursors` | Präzise Cursor | r | B | B | 1 |
| 09 | `edit_pick_distance` | Auswahlabstand/Fangabstand | r | J | D | 0 |
| 10 | `guides_grid` | Hilfslinien/Raster-Farbe | r | B | B | 1 |
| 11 | `type` | Text: Standardschrift, Glättung, Einheit | r | J | D | 1 |
| 12 | `ps_import` | Import mehrschichtiger Dateien (Ebenen/Text) | n | W | S | 1 |
| 13 | `launch_edit` | Externe Bearbeitung | n | W | S | 1 |
| 14 | `plugins` | Zusatzordner | n | W | S | 1 |
| 15 | `scale_strokes` | Strich/Effekte skalieren | r | J | D | 0 |
| 16 | `reset_warnings` | Warnmeldungen zurücksetzen (Z01–Z18) | r | J | S | 1 |
| 17 | `cancel` | Abbrechen verwirft | r | J | S | 0 |
| 18 | `persist` | nach Neustart erhalten | r | J | S | 0 |
| 19 | `tool_defaults` | Werkzeugvorgaben aus Voreinstellungen | r | W | S | 0 |
| 20 | `tab_focus` | Fokus, Pfeil wechselt Seite | f | B | B | 1 |

### 46 · `xs-canvas` – Leinwand: Zoom, Bildlauf, Lineale, Hilfslinien, Raster, Fang, Pixelraster
**Test:** T10. **Bef.:** L1–L30, N03, N04, N10, D18.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `zoom_steps` | Stufentabelle 6 %…6400 % | r | J | S | 0 |
| 02 | `zoom_center` | Zoom um Mauszeiger/Mitte, kein Versatz (N10) | r | B | B | 2 |
| 03 | `fit` | Einpassen 15 px Rand | r | J | S | 1 |
| 04 | `scrollbars` | Bildlaufleisten, Rand um Dokument | n | B | B | 1 |
| 05 | `rulers` | Lineale: Einheit, Nullpunkt ziehen | n, r | B | B | 2 |
| 06 | `guides_drag` | Hilfslinie ziehen, Wert-Tooltip | p | B | B | 1 |
| 07 | `guides_dblclick` | Doppelklick → Hilfslinie verschieben-Dialog | o | W, B | Fi, B | 1 |
| 08 | `guides_snap` | An Hilfslinien ausrichten | r | J | D | 0 |
| 09 | `guides_lock` | Hilfslinien sperren | r | J | D | 0 |
| 10 | `guides_remove` | aus Leinwand ziehen löscht | r | J | D | 0 |
| 11 | `grid_show` | Raster ein, Farbe/Abstand | n | B | B | 1 |
| 12 | `grid_snap` | An Raster ausrichten | r | J | D | 0 |
| 13 | `smart_guides` | Intelligente Hilfslinien (falls vorhanden) | p | B | B | 1 |
| 14 | `pixel_grid` | Pixelraster bei ≥ 400 %? | n | B | B | 1 |
| 15 | `snap_pixels` | An Pixeln ausrichten | r | J | D | 0 |
| 16 | `edges` | Ansicht ▸ Kanten (Hover-Rahmen) | n | B | B | 1 |
| 17 | `drag_clip` | Ziehen über Leinwandrand abgeschnitten (N04) | p | B | B | 1 |
| 18 | `checker` | Transparente Leinwand Schachbrett | n | B | B | 1 |
| 19 | `outside_click` | Klick außerhalb hebt Auswahl auf | r | J | D | 0 |
| 20 | `macro_view` | Ansicht vollbild, Lineale/Raster bleiben? | n | B | B | 1 |
| 21 | `status_bar` | Statusleiste unten (Größe, Zoom-Feld, Abspielen) | n, o | W, B | S, B | 2 |
| 22 | `cursor_coords` | Info-Koordinaten = Linealmarke | h | B | B | 1 |

### 47 · `xs-file-formats` – Öffnen/Importieren/Speichern je Format
**Test:** T11, T03. **Abh.:** BM-1. **Spez.:** `re\02-fwpng-format.md`, `re\12-import.md`, `re\20-ai8swf.md`. **Methode:** Datei öffnen über JSF (`fw.openDocument`/`importFile`) bzw. Brücke `ui_file_dialog` (Testmodus), keine offenen Dialoge.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `open_png` | PNG öffnen: Objekt, Grenzen | r | J | D | 1 |
| 02 | `open_jpg` | JPEG | r | J | D | 0 |
| 03 | `open_gif_anim` | animiertes GIF → Status | r | J | D | 1 |
| 04 | `open_bmp_tif` | BMP, TIFF | r | J | D | 0 |
| 05 | `open_psd` | PSD: Ebenen, Text editierbar?, Effekte | r | J, B | D, B | 2 |
| 06 | `open_svg` | SVG | r | J, B | D, B | 1 |
| 07 | `open_vector` | Vektordatei aus Korpus | r | J, B | D, B | 1 |
| 08 | `open_fwpng_bounds` | FW-PNG mit transparentem Rand: Bitmapgrenzen (N26) | r | J | D | 1 |
| 09 | `import_place` | Importieren: Platzierungszeiger, Klick/Ziehen | p, r | J, B | D, B | 2 |
| 10 | `import_multipage` | mehrseitige Datei: Seitenwahl | o | W | Fi | 1 |
| 11 | `drop_file` | Datei in offenes Dokument ziehen (N07) | r | J | D | 0 |
| 12 | `save_fwpng` | Speichern FW-PNG, wieder öffnen = gleich | r | J | D | 0 |
| 13 | `save_as_types` | Speichern unter: Typenliste | o | W | Fi | 1 |
| 14 | `save_as_psd` | als PSD | r | J | R | 0 |
| 15 | `save_flat` | als PNG/JPEG/GIF (flach), Warnung | o, r | W | Fi | 1 |
| 16 | `open_as_untitled` | „Als Unbenannt öffnen“ | r | J | D | 0 |
| 17 | `open_as_anim` | „Als Animation öffnen“ (mehrere Dateien) | r | J | D | 0 |
| 18 | `recent` | Zuletzt geöffnet | o | W | Fi | 1 |

### 48 · `xs-windows-tabs` – Reiter, Fenster, Docking, Statuszeile, Startbildschirm
**Test:** T01, T04, T05. **Bef.:** PARITAET_B „Reiter und Fenster“, „Docking und Arbeitsbereiche“, N02, N06, N09. **🎨:** Startbildschirm DES-2, Dialograhmen DES-1, N11.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `tabs` | Reiter: Name, *, Schließen-x, Hover | n, h | B | B | 2 |
| 02 | `tab_switch` | Strg+Tab, Mittelklick | r | J | S | 0 |
| 03 | `tab_overflow` | viele Reiter: Überlauf | n | B | B | 1 |
| 04 | `tab_context` | Reiter-Kontextmenü (siehe 43) | o | W | Fi | 0 |
| 05 | `tab_float` | Reiter herausziehen → eigenes Fenster | r | B | B | 1 |
| 06 | `cascade_tile` | Überlappend/Nebeneinander | r | B | B | 2 |
| 07 | `duplicate_window` | Fenster duplizieren | r | J | S | 0 |
| 08 | `unsaved_prompt` | Ungespeichert-Abfrage Wortlaut (N06) | o | W, B | Fi, B | 1 |
| 09 | `panel_dock` | Bedienfeld lösen/andocken | r | B | B | 2 |
| 10 | `panel_collapse` | Gruppe einklappen, Symbolmodus | r | B | B | 2 |
| 11 | `panel_group_menu` | Gruppen-Kontextmenü | o | W | Fi | 1 |
| 12 | `workspace_switch` | Arbeitsbereich-Umschalter | o | W, B | Fi, B | 1 |
| 13 | `workspace_restore` | Layout nach Neustart | r | B | B | 1 |
| 14 | `hide_panels` | F4/Tab blendet aus | r | B | B | 1 |
| 15 | `status_bar` | Statuszeile/Fußleiste Inhalte | n | W, B | S, B | 1 |
| 16 | `start_screen` | Startbildschirm Inhalte (Neu, Zuletzt) – Inhalt prüfen, Gestaltung 🎨 | n | W, B | Fi, B | 1 |
| 17 | `title_bar` | Titel = Dokument + Zoom | n | W | S | 0 |
| 18 | `min_size` | kleinstes Fenster | n | B | B | 1 |
| 19 | `close_last` | letztes Dokument schließen → Startbildschirm | r | B | B | 1 |
| 20 | `dialog_move` | Dialoge frei verschiebbar (N02) | p | J | S | 0 |

## H. Menüs – Wirkung

Beschriftung, Kürzel, grau/aktiv, Häkchen sind in `PARITAET_B.md` §1 gemessen (276 ✅, 44 ⏳). Hier zählt nur die
**Wirkung** je Befehl (Objektbaum vorher/nachher per JSF bzw. `get_document`, Verlaufsname, Meldungen). Die Befehle stehen
vollständig in `commands.json`; Prüfpunkte fassen Gruppen zusammen, die `.orig.md` listet je Befehl eine Zeile.
Bilder nur für Meldungen und Ansichtsänderungen.

### 49 · `menu-file` – Datei (33 Befehle)
**Test:** T04, T09. Keine Datei-Dialoge offen stehen lassen (Testmodus `ui_file_dialog`).

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `new` | Neu → D01-Vorgaben = Zwischenablage-Größe | r | J | D | 0 |
| 02 | `new_from_template` | Neu von Vorlage | r | J | D | 0 |
| 03 | `open_recent` | Zuletzt geöffnet, Liste leeren | r | J | S | 0 |
| 04 | `reopen` | Wiederherstellen (Zuletzt gespeichert) Rückfrage | o | W | Fi | 1 |
| 05 | `close` | Schließen, Alle schließen | r | J | S | 0 |
| 06 | `save` | Speichern ohne Namen → Speichern unter | r | J | S | 0 |
| 07 | `save_copy` | Kopie speichern | r | J | R | 0 |
| 08 | `save_all` | Alle speichern | r | J | S | 0 |
| 09 | `save_template` | Als Vorlage speichern | r | J | R | 0 |
| 10 | `import` | Importieren (→ 47) | r | J | D | 0 |
| 11 | `export` | Exportieren (→ 42) | r | J | R | 0 |
| 12 | `export_again` | Erneut exportieren (Strg+Umschalt+R) | r | J | R | 0 |
| 13 | `preview` | Bildvorschau öffnet | o | W | Fi | 0 |
| 14 | `update_html` | HTML aktualisieren | o | W | Fi | 1 |
| 15 | `file_info` | Datei-Info | o | W | Fi | 0 |
| 16 | `batch` | Batch-Verarbeitung (🎨 BATCH_MODERN, nur erreichbar) | o | – | Fi | 0 |
| 17 | `print` | Drucken, Seite einrichten | o | W | Fi | 1 |
| 18 | `scan_twain` | Scannen/Erfassen (deaktiviert?) | d | W | Fi | 0 |
| 19 | `browse` | Dateibrowser-Eintrag (ℹ️ neutral) | o | W | Fi | 0 |
| 20 | `share` | Weitergeben/E-Mail | o | W | Fi | 1 |
| 21 | `exit` | Beenden mit ungespeicherten Dokumenten | o | W | Fi | 1 |
| 22 | `undo_names` | Verlaufsnamen nach Import/Neu | r | J | H | 0 |

### 50 · `menu-edit` – Bearbeiten (29 Befehle)
**Test:** T04. **Bef.:** N05, Z01–Z18 (Warnungen), „Wiederholen: <letzter Schritt>“.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `undo_redo` | Rückgängig/Wiederholen mit Schrittnamen | o, r | W | Fi | 1 |
| 02 | `redo_last` | Wiederholen ohne Rückgängig = letzten Befehl | r | J | D | 0 |
| 03 | `cut_copy_paste` | Ausschneiden/Kopieren/Einfügen: Lage | r | J | D | 0 |
| 04 | `paste_offset` | erneutes Einfügen versetzt? | r | J | D | 0 |
| 05 | `paste_inside` | Als Maske einfügen / Innen einfügen | r | J | D | 0 |
| 06 | `paste_attributes` | Attribute einfügen | r | J | D | 0 |
| 07 | `paste_special` | Inhalte einfügen (Formate) | o | W | Fi | 1 |
| 08 | `copy_path_outlines` | Pfadkonturen kopieren | r | J | R | 0 |
| 09 | `clear` | Löschen/Entf | r | J | D | 0 |
| 10 | `duplicate` | Duplizieren Versatz | r | J | D | 0 |
| 11 | `clone` | Klonen gleiche Lage | r | J | D | 0 |
| 12 | `find` | Suchen öffnet Bedienfeld | r | J | S | 0 |
| 13 | `insert_menu` | Einfügen ▸ Symbol/Schaltfläche/Ebene/Seite/Segment/Hotspot | r | J | D | 1 |
| 14 | `insert_slice_multi` | aus 2 Objekten: Rückfrage eins/mehrere | o | W, B | Fi, B | 1 |
| 15 | `crop_selected_bitmap` | Gewählte Bitmap zuschneiden | r | J | D | 0 |
| 16 | `crop_document` | Dokument zuschneiden | r | J | D | 0 |
| 17 | `warnings_no_target` | Warnungen bei fehlendem Ziel, „Nicht mehr anzeigen“ | o | W, B | Fi, B | 2 |
| 18 | `prefs_shortcuts` | Voreinstellungen/Tastaturkurzbefehle öffnen | o | W | Fi | 0 |
| 19 | `select_all_bitmap` | Strg+A im Bitmapmodus | r | J | D | 0 |
| 20 | `deselect` | Strg+D | r | J | D | 0 |
| 21 | `paste_external` | Bild aus Zwischenablage (fremd) | r | J | D | 1 |
| 22 | `undo_names` | Verlaufsnamen | r | J | H | 0 |

### 51 · `menu-view` – Ansicht (36 Befehle)
**Test:** T10, T06.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `zoom_in_out` | Vergrößern/Verkleinern Stufen | r | J | S | 0 |
| 02 | `magnification` | Vergrößerung ▸ Stufen | r | J | S | 0 |
| 03 | `fit_selection` | Auswahl einpassen | r | J | S | 0 |
| 04 | `fit_all` | Alles einpassen | r | J | S | 0 |
| 05 | `full_display` | Volle Anzeige / Entwurfsanzeige (Drahtgitter) | r | B | B | 2 |
| 06 | `macintosh_gamma` | Gamma-Ansicht | r | B | B | 1 |
| 07 | `hide_show_selection` | Auswahl aus-/einblenden, Alle einblenden | r | J | D | 0 |
| 08 | `rulers` | Lineale | r | B | B | 0 |
| 09 | `grid_menu` | Raster ▸ Anzeigen/Ausrichten/Bearbeiten | r | J | S | 0 |
| 10 | `guides_menu` | Hilfslinien ▸ Anzeigen/Sperren/Ausrichten/Bearbeiten | r | J | S | 1 |
| 11 | `slice_guides` | Segmenthilfslinien | r | B | B | 1 |
| 12 | `slice_overlay` | Segmentüberlagerung | r | B | B | 1 |
| 13 | `edges` | Kanten | r | B | B | 1 |
| 14 | `tooltips` | Werkzeug-Tooltips | r | B | B | 0 |
| 15 | `hide_panels` | Bedienfelder ausblenden | r | B | B | 1 |
| 16 | `status_bar` | Statusleiste | r | B | B | 0 |
| 17 | `screen_modes` | Bildschirmmodi | r | B | B | 1 |
| 18 | `smart_guides` | Intelligente Hilfslinien/Pixel ausrichten | r | J | S | 0 |
| 19 | `pages_states_nav` | Seite/Status vor/zurück | r | J | S | 0 |
| 20 | `edit_guides_dialog` | Hilfslinien bearbeiten (Strg+Alt+Umschalt+G) | o | W, B | Fi, B | 1 |
| 21 | `preview_modes` | Original/Vorschau/2-fach/4-fach-Ansicht im Dokument | r | B | B | 1 |
| 22 | `hotspots_show` | Hotspots ein/aus | r | B | B | 0 |
| 23 | `checkmarks` | Häkchen synchron mit Zustand | n | W | Fi | 0 |
| 24 | `persist` | Ansichtszustand je Dokument gemerkt | r | J | S | 0 |

### 52 · `menu-select` – Auswählen (15 Befehle)
**Test:** T02, T04. Dialoge D35–D40 in Einheit 60.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `all` | Alles auswählen (Objekt / Bitmapmodus) | r | J | D | 0 |
| 02 | `deselect` | Auswahl aufheben | r | J | D | 0 |
| 03 | `superselect_subselect` | Übergeordnet / Untergeordnet auswählen | r | J | D | 0 |
| 04 | `similar` | Ähnliche auswählen | r | J | D | 0 |
| 05 | `inverse` | Umkehren (Pixel) | r | E | E | 1 |
| 06 | `feather` | Weiche Kante (Pixel) | r | E | E | 1 |
| 07 | `expand_contract` | Erweitern/Verkleinern Grenzen | r | J | D | 0 |
| 08 | `border` | Begrenzen | r | E | E | 1 |
| 09 | `smooth` | Glätten | r | E | E | 1 |
| 10 | `save_restore` | Bitmap-Auswahl speichern/wiederherstellen/löschen | r | J | D | 1 |
| 11 | `similar_pixels` | Ähnliches (Pixel, Toleranz) | r | E | E | 1 |
| 12 | `next_prev` | Nächstes/Vorheriges (Strg+Umschalt+Pfeile) | r | J | D | 0 |
| 13 | `disabled_states` | grau je Lage (ohne Bitmapmodus) | d | W | Fi | 0 |
| 14 | `warning` | Meldung ohne Pixelauswahl | o | W, B | Fi, B | 1 |
| 15 | `undo_names` | Verlaufsnamen | r | J | H | 0 |

### 53 · `menu-modify-a` – Modifizieren: Leinwand, Bild, Animation, Symbol, Popup, Maske, Selektives JPEG, Bitmap
**Test:** T02, T04, T06.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `canvas_size` | Leinwandgröße (→ D20) Wirkung | r | J | D | 0 |
| 02 | `image_size` | Bildgröße (→ D19) Resample | r | E | E | 1 |
| 03 | `canvas_color` | Leinwandfarbe | r | J | D | 0 |
| 04 | `trim_fit` | Leinwand beschneiden/anpassen | r | J | D | 0 |
| 05 | `rotate_canvas` | Leinwand drehen 180/90/90 | r | J | D | 1 |
| 06 | `flatten` | Auswahl reduzieren, Ebenen reduzieren | r | J | D | 0 |
| 07 | `merge_down` | Mit darunter reduzieren | r | J | D | 0 |
| 08 | `convert_bitmap` | In Bitmap umwandeln | r | E | E | 1 |
| 09 | `animation_menu` | Animation ▸ Auswahl animieren/Einstellungen/Entfernen | r | J | D | 0 |
| 10 | `symbol_convert` | In Symbol konvertieren / Aufheben | r | J | D | 0 |
| 11 | `symbol_tween` | Instanzen tweenen (Meldung ohne 2) | o, r | W, J | Fi, D | 1 |
| 12 | `symbol_edit` | Symbol bearbeiten | o | B | B | 1 |
| 13 | `popup_add` | Popupmenü hinzufügen (Rückfrage ohne Ziel, D34) | o | W | Fi | 1 |
| 14 | `popup_edit_delete` | bearbeiten/löschen | r | J | D | 0 |
| 15 | `mask_reveal_hide` | Maske ▸ Alles zeigen/verbergen/Auswahl zeigen | r | J, E | D, E | 1 |
| 16 | `mask_paste` | Als Maske einfügen/Gruppieren als Maske | r | J | D | 0 |
| 17 | `mask_disable_delete` | Maske deaktivieren/löschen/anwenden | r | J | D | 0 |
| 18 | `sel_jpeg` | Selektives JPEG ▸ Speichern/Wiederherstellen/Entfernen (Verlaufsnamen) | r | J | H | 0 |
| 19 | `bitmap_alpha` | Transparenz → Alpha | r | E | E | 1 |
| 20 | `marquee_to_path` | Auswahlrahmen in Pfad | r | J | D | 1 |
| 21 | `path_to_marquee` | Pfad in Auswahlrahmen (→ D32) | r | E | E | 0 |
| 22 | `exit_bitmap` | Bitmapmodus verlassen | r | J | S | 0 |
| 23 | `disabled_states` | grau je Lage | d | W | Fi | 0 |
| 24 | `undo_names` | Verlaufsnamen | r | J | H | 0 |

### 54 · `menu-modify-b` – Modifizieren: Transformieren, Anordnen, Ausrichten, Pfad, Kombinieren, Gruppieren
**Test:** T04. **Spez.:** `re\16-pfadoperationen.md`.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `free_transform` | Frei transformieren/Skalieren/Verzerren/Verkrümmen startet Griffe | r | B | B | 1 |
| 02 | `numeric` | Numerisch transformieren (→ D26) Wirkung | r | J | D | 0 |
| 03 | `rotate_flip` | Drehen 180/90 im/gegen UZS, Horizontal/Vertikal spiegeln | r | J | D | 0 |
| 04 | `remove_transform` | Transformationen entfernen | r | J | D | 0 |
| 05 | `arrange` | In den Vordergrund/nach vorn/nach hinten/Hintergrund | r | J | D | 0 |
| 06 | `align` | Ausrichten ▸ 6 + Verteilen 2 | r | J | D | 0 |
| 07 | `combine_union` | Vereinen | r | J (Punkte) | D | 1 |
| 08 | `combine_intersect` | Schnittmenge | r | J | D | 1 |
| 09 | `combine_punch` | Ausstanzen | r | J | D | 1 |
| 10 | `combine_crop` | Zuschneiden | r | J | D | 0 |
| 11 | `combine_split` | Teilen | r | J | D | 0 |
| 12 | `join_split` | Verbinden/Teilen (Pfade) | r | J | D | 0 |
| 13 | `simplify` | Vereinfachen (→ D29) | r | J | D | 0 |
| 14 | `expand_stroke` | Strich erweitern (→ D30) | r | J | D | 1 |
| 15 | `inset_path` | Pfad einlassen (→ D31) | r | J | D | 0 |
| 16 | `add_points` | Punkte hinzufügen | r | J | D | 0 |
| 17 | `reverse` | Richtung umkehren | r | J | D | 0 |
| 18 | `hard_soft_fill` | Harte/Weiche Füllung entfernen? | r | J | D | 0 |
| 19 | `group_ungroup` | Gruppieren/Aufheben | r | J | D | 0 |
| 20 | `mask_group` | Als Maske gruppieren | r | J | D | 0 |
| 21 | `text_to_paths` | (bei Text) In Pfade konvertieren | r | J | D | 0 |
| 22 | `rect_corner` | Rechteck-Ecken-Befehle | r | J | D | 0 |
| 23 | `disabled_states` | grau je Lage | d | W | Fi | 0 |
| 24 | `undo_names` | Verlaufsnamen | r | J | H | 0 |

### 55 · `menu-text` – Text (39 Befehle)
**Test:** T05. **Bef.:** PARITAET_B „Aufbau des Schrift-Menüs (zuletzt benutzte oben, Familien flach)“. **🎨:** –.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `font_menu` | Schrift ▸ Aufbau (zuletzt benutzte, Familien, Schnitte) | o | W, B | Fi, B | 2 |
| 02 | `size_menu` | Größe ▸ Stufen, Andere (→ D23) | r | J | D | 0 |
| 03 | `style_menu` | Stil ▸ Fett/Kursiv/Unterstrichen | r | J | D | 0 |
| 04 | `align_menu` | Ausrichten ▸ 5 + Blocksatz gestreckt | r | J | D | 0 |
| 05 | `editor` | Texteditor (Strg+Umschalt+E) öffnen/übernehmen | o | W, B | Fi, B | 1 |
| 06 | `attach_path` | An Pfad anfügen | r | J, E | D, E | 1 |
| 07 | `attach_shape` | In Pfad anfügen | r | E | E | 1 |
| 08 | `detach` | Von Pfad lösen | r | J | D | 0 |
| 09 | `orientation` | Ausrichtung am Pfad (4 Arten) | r | E | E | 2 |
| 10 | `reverse_direction` | Richtung umkehren | r | E | E | 1 |
| 11 | `convert_paths` | In Pfade konvertieren | r | J | D | 0 |
| 12 | `spelling` | Rechtschreibung prüfen (Dialog, Wirkung) | o | W, B | Fi, B | 1 |
| 13 | `spelling_setup` | Rechtschreibprüfung einrichten (→ D24) | o | W | Fi | 0 |
| 14 | `replace_fonts` | Fehlende Schriften ersetzen | o | W | Fi | 0 |
| 15 | `in_edit_mode` | Befehle während Textbearbeitung (nur markierte) | r | J | D | 0 |
| 16 | `multi_text` | auf 2 Texte | r | J | D | 0 |
| 17 | `disabled_states` | grau ohne Text | d | W | Fi | 0 |
| 18 | `checkmarks` | Häkchen aktueller Stil/Größe | n | W | Fi | 0 |
| 19 | `vertical` | Vertikaler Text-Befehle | r | E | E | 0 |
| 20 | `undo_names` | Verlaufsnamen | r | J | H | 0 |

### 56 · `menu-commands-window-help` – Befehle, Filter (Menü), Fenster, Hilfe
**Test:** T04. **Bef.:** PARITAET_B „Befehle-Menü im frischen Profil, 3 Einträge fehlen“, „Häkchen im Fenster-Menü nur beim vordersten Reiter“. **🎨:** Firepixel-eigene Befehle (KI, Nachzeichnen, Batch).

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `commands_list` | Befehle-Menü im frischen Profil: Ordner, Einträge | o | W, B | Fi, B | 2 |
| 02 | `commands_run` | Stichprobe 5 eingebaute Befehle: Wirkung | r | J | D | 0 |
| 03 | `commands_manage` | Gespeicherte Befehle verwalten (→ D25) | o | W | Fi | 0 |
| 04 | `commands_run_script` | Skript ausführen (Testmodus) | r | J | R | 0 |
| 05 | `filter_menu` | Filter-Menü: Kategorien, „Letzten wiederholen“ | o | W | Fi | 1 |
| 06 | `window_panels` | Fenster ▸ jedes Bedienfeld öffnet/schließt, Häkchen | r | W | Fi | 0 |
| 07 | `window_toolbars` | Symbolleisten | r | B | B | 1 |
| 08 | `window_hide_panels` | Bedienfelder ausblenden | r | B | B | 0 |
| 09 | `window_workspace` | Arbeitsbereich-Layouts | o | W | Fi | 1 |
| 10 | `window_arrange` | Überlappend/Nebeneinander | r | B | B | 0 |
| 11 | `window_docs` | Dokumentliste mit Häkchen vorn | o | W | Fi | 1 |
| 12 | `window_extensions` | Erweiterungs-Bedienfelder (ℹ️) | o | W | Fi | 0 |
| 13 | `help_entries` | Hilfe-Einträge | o | W | Fi | 1 |
| 14 | `help_about` | Info-Dialog Inhalt (🎨 Gestaltung) | o | W | Fi | 1 |
| 15 | `help_online` | Online-Einträge (Shell-Öffnen unterdrückt) | r | J | S | 0 |
| 16 | `disabled_states` | grau ohne Dokument | d | W | Fi | 0 |
| 17 | `no_doc` | Menüs ohne Dokument | o | W | Fi | 0 |
| 18 | `undo_names` | Verlaufsnamen der Befehle | r | J | H | 0 |

## I. Dialoge – Verhalten

Aufbau und Felder sind in `PARITAET_B.md` §2 gemessen (D01–D50, 46 ⏳ nach DLG-/E-/KV-/BV-/NZ-Paketen). Hier je Dialog:
Felder/Grenzen/Klemmen, Tab-Folge, Enter/Esc, Merken beim nächsten Öffnen, Wirkung, Fehlermeldungen bei ungültigen
Werten. **🎨 für alle:** Rahmen, Titel-/Fußleiste, OK-Hervorhebung, Abstände (DES-1, N11). Je Dialog 2 Prüfpunkte
(a: Felder/Grenzen/Tab/Enter/Esc · b: Wirkung/Merken), Bilder: Übersicht + aufgeklappte Listen.

### 57 · `dialogs-d01-d10`
**Test:** T04, T09. **Bef.:** D01–D10, N06, N16, NACHPRUEFUNG DLG-1b (D01), E1 (D07), BV (D08).

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `d01_fields` | Neues Dokument: Felder, Einheiten, Grenzen, Zugriffstasten | n, o, f | W, B | Fi, B | 3 |
| 02 | `d01_effect` | Wirkung, Merken, Zwischenablage-Größe | r | J | D | 0 |
| 03 | `d02_fields` | Neu von Vorlage: Ordner, Vorschau | n | W, B | Fi, B | 1 |
| 04 | `d02_effect` | Wirkung | r | J | D | 0 |
| 05 | `d03_fields` | Öffnen: Zusatzknöpfe, Vorschau | n | W, B | Fi, B | 2 |
| 06 | `d03_effect` | Als Unbenannt/Als Animation | r | J | D | 0 |
| 07 | `d04_fields` | Speichern unter: Typenliste, Startordner | o | W, B | Fi, B | 2 |
| 08 | `d04_effect` | Wirkung, Warnung flach | r | J | R | 0 |
| 09 | `d05_effect` | Als Vorlage speichern | r | J | R | 0 |
| 10 | `d06_fields` | Importieren: Filterliste (12), Vorschau | o | W, B | Fi, B | 2 |
| 11 | `d06_effect` | Platzieren | r | J | D | 0 |
| 12 | `d07_fields` | Exportieren: Art, HTML, Segmente, Optionen | n, o | W, B | Fi, B | 3 |
| 13 | `d07_effect` | → Einheit 42 | r | – | – | 0 |
| 14 | `d08_fields` | Bildvorschau (→ 40/41), Esc | r | W | S | 0 |
| 15 | `d09_fields` | HTML aktualisieren | n | W, B | Fi, B | 1 |
| 16 | `d09_effect` | Wirkung | r | J | R | 0 |
| 17 | `d10_fields` | Datei-Info: Felder, Register | n | W, B | Fi, B | 2 |
| 18 | `d10_effect` | Speichern in Datei, Merken | r | J | D | 0 |
| 19 | `tab_enter_esc` | Tab-Folge, Enter/Esc je Dialog (Tabelle) | f | W | S | 3 |
| 20 | `no_owner` | Datei-Dialoge mittig, mit Besitzer | n | W | S | 0 |

### 58 · `dialogs-d11-d20`
**Test:** T04, T09. **Abh.:** VT-6 betrifft nur den Firepixel-eigenen Nachzeichnen-Dialog (🎨 N20/N29, hier nicht). **Bef.:** D11–D20, D12 Batch ❌ zurückgestellt (🎨 BATCH_MODERN), DLG-1b/2/4, E3.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `d11_pages` | Exportassistent: Seiten, Weiter/Zurück, Fokus | n | W, B | Fi, B | 3 |
| 02 | `d11_effect` | Ergebnis, kein Verlaufsschritt | r | J | H | 0 |
| 03 | `d12_reach` | Batch: nur erreichbar (🎨) | o | – | Fi | 0 |
| 04 | `d13_tabs` | HTML-Einrichtung: 3 Register, Felder | n, o | W, B | Fi, B | 3 |
| 05 | `d13_effect` | Merken, Wirkung auf Export | r | J | R | 0 |
| 06 | `d14_fields` | Neues Symbol: Name, Typ, 9-teilig | n | W, B | Fi, B | 1 |
| 07 | `d14_effect` | Symbol-Editor öffnet | r | J | D | 0 |
| 08 | `d15_pages` | HTML-Code kopieren: Seiten | n | W, B | Fi, B | 2 |
| 09 | `d15_effect` | Zwischenablage-Inhalt | r | J | R | 0 |
| 10 | `d16_pages` | Voreinstellungen: jede Kategorie | n | W, B | Fi, B | 6 |
| 11 | `d16_effect` | → Einheit 45 | r | – | – | 0 |
| 12 | `d17_fields` | Tastaturkurzbefehle: Satz, Baum, Knöpfe | n, o | W, B | Fi, B | 2 |
| 13 | `d17_effect` | Satz duplizieren/umbenennen/löschen, Konflikt | r | J | S | 1 |
| 14 | `d18_fields` | Raster bearbeiten: Farbe, Abstände, Grenzen | n | W, B | Fi, B | 1 |
| 15 | `d18_effect` | Wirkung, Merken | r | J | S | 0 |
| 16 | `d19_fields` | Bildgröße: Felder, Kette, Einheiten, Resample-Liste | n, o | W, B | Fi, B | 2 |
| 17 | `d19_limits` | Grenzen, gekoppelte Werte | r | W | S | 0 |
| 18 | `d19_effect` | Wirkung | r | E | E | 0 |
| 19 | `d20_fields` | Leinwandgröße: Anker, ↔/↕ | n | W, B | Fi, B | 1 |
| 20 | `d20_effect` | Anker je Lage | r | J | D | 0 |
| 21 | `tab_enter_esc` | Tab, Enter/Esc | f | W | S | 3 |
| 22 | `remember` | gemerkte Werte | r | J | S | 0 |

### 59 · `dialogs-d21-d30`
**Test:** T02, T04, T05. **Bef.:** D21–D30, DLG-2 (D21–D23), DLG-4 (D25), DLG-7 (D26, D29, D30), DLG-9 (D27, D28).

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `d21_fields` | Leinwandfarbe | n | W, B | Fi, B | 1 |
| 02 | `d21_effect` | Wirkung | r | J | D | 0 |
| 03 | `d22_fields` | Selektives JPEG: Haken, graue Felder | n, d | W, B | Fi, B | 2 |
| 04 | `d22_effect` | Wirkung auf Export | r | E | E | 0 |
| 05 | `d23_fields` | Textgröße Andere: Schieber 8..96 | n, o | W, B | Fi, B | 2 |
| 06 | `d23_effect` | Wirkung | r | J | D | 0 |
| 07 | `d24_fields` | Rechtschreibprüfung einrichten: Wörterbücher, Optionen | n | W, B | Fi, B | 1 |
| 08 | `d24_effect` | Merken | r | J | S | 0 |
| 09 | `d25_msg` | Gespeicherte Befehle verwalten (Meldung frisches Profil) | o | W, B | Fi, B | 1 |
| 10 | `d25_list` | mit gespeichertem Befehl: Umbenennen/Löschen | n | W | Fi | 1 |
| 11 | `d26_fields` | Numerisch transformieren: Liste, Kette, Kästchen | n, o | W, B | Fi, B | 2 |
| 12 | `d26_effect` | Skalieren/Größe/Drehen je Modus | r | J | D | 0 |
| 13 | `d27_fields` | In Symbol konvertieren | n | W, B | Fi, B | 1 |
| 14 | `d27_effect` | Wirkung | r | J | D | 0 |
| 15 | `d28_fields` | Auswahl animieren: Felder, Grenzen | n | W, B | Fi, B | 1 |
| 16 | `d28_effect` | Wirkung (Status, Bewegungspfad) | r | J | D | 0 |
| 17 | `d29_fields` | Vereinfachen | n, o | W, B | Fi, B | 1 |
| 18 | `d29_effect` | Punktzahl danach | r | J | D | 0 |
| 19 | `d30_fields` | Strich erweitern: Breite, Ecken, Gehrung, Enden | n | W, B | Fi, B | 1 |
| 20 | `d30_effect` | Ergebnis-Pfad | r | J | D | 0 |
| 21 | `tab_enter_esc` | Tab, Enter/Esc | f | W | S | 3 |
| 22 | `remember` | gemerkte Werte | r | J | S | 0 |

### 60 · `dialogs-d31-d40`
**Test:** T02, T04, T06. **Bef.:** D31–D40, DLG-7 (D31, D32), DLG-9 (D33, D34), DLG-8 (D35–D39), KV-3 (D40).

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `d31_fields` | Pfad einlassen: Richtung, Breite, Ecken | n | W, B | Fi, B | 1 |
| 02 | `d31_effect` | Ergebnis | r | J | D | 0 |
| 03 | `d32_fields` | Pfad in Auswahlrahmen: Rand-Liste | n, o | W, B | Fi, B | 2 |
| 04 | `d32_effect` | Ergebnis | r | E | E | 0 |
| 05 | `d33_msg` | Instanzen tweenen: Meldung / Dialog (Schritte, Auf Status verteilen) | o | W, B | Fi, B | 2 |
| 06 | `d33_effect` | Wirkung mit 2 Instanzen | r | J | D | 0 |
| 07 | `d34_msg` | Popupmenü hinzufügen: Rückfrage | o | W, B | Fi, B | 1 |
| 08 | `d34_editor` | Popupmenü-Editor Register | n | W, B | Fi, B | 2 |
| 09 | `d35_fields` | Weiche Auswahlkante | n | W, B | Fi, B | 1 |
| 10 | `d35_effect` | Ergebnis | r | E | E | 0 |
| 11 | `d36_d37` | Erweitern/Verkleinern: Grenzen 1/5/10 | r | J | D | 1 |
| 12 | `d38` | Begrenzen | r | E | E | 1 |
| 13 | `d39` | Glätten | r | E | E | 1 |
| 14 | `d40_fields` | Auswahl speichern: Ziel, Vorgang | n | W, B | Fi, B | 1 |
| 15 | `d40_effect` | Wiederherstellen-Vorgänge | r | J | D | 1 |
| 16 | `invalid_values` | ungültige Werte → Meldung | o | W, B | Fi, B | 2 |
| 17 | `enter_esc` | Enter/Esc | r | J | S | 0 |
| 18 | `tab_order` | Tab-Folge | f | W | S | 2 |
| 19 | `remember` | gemerkte Werte | r | J | S | 0 |
| 20 | `undo_names` | Verlaufsnamen | r | J | H | 0 |

### 61 · `dialogs-d41-d50` – Filterdialoge (Felder, Schieber, Merken; Pixel → RE-FX)
**Test:** T02 (B1). **Abh.:** FX-3, FX-4, RE-FX. **Bef.:** D41–D50, „Schieber stückweise nicht linear“ (BEFUNDE IBD, FV-4), DLG-5/6, KV-1/2. **Spez.:** `re\06-bitmap-filter.md` §8.1, `re\19-kurven-tonwert.md`.

| Nr | Kürzel | Prüfpunkt | Zust. | Orig. | FP | Bilder |
|---|---|---|---|---|---|---:|
| 01 | `d41_fields` | Farbton/Sättigung: Felder, Färben, Vorschau | n | W, B | Fi, B | 1 |
| 02 | `d41_sliders` | Schieber-Abbildung (Stützstellen) | p | W | S | 1 |
| 03 | `d42_fields` | Helligkeit/Kontrast −100…100 | n | W, B | Fi, B | 1 |
| 04 | `d43_fields` | Kurven: Kanal, Raster, Punkte, Pipetten, Auto | n, o | W, B | Fi, B | 2 |
| 05 | `d43_mouse` | Mausregeln (Punkt setzen/ziehen/entfernen) | p | J | D | 1 |
| 06 | `d44_fields` | Tonwertkorrektur: Histogramm, Griffe, Felder | n | W, B | Fi, B | 2 |
| 07 | `d44_mouse` | Griffe ziehen, Mittelgriff | p | J | D | 1 |
| 08 | `d45_fields` | Rauschen: Menge ▾, Farbe | n, o | W, B | Fi, B | 2 |
| 09 | `d46_fields` | Unscharf maskieren: 3 Felder ▾ | n | W, B | Fi, B | 1 |
| 10 | `d47_fields` | Bewegungsunschärfe: Winkel/Umfang | n | W, B | Fi, B | 1 |
| 11 | `d48_fields` | Gauß: Radius ▾ | n, o | W, B | Fi, B | 2 |
| 12 | `d49_fields` | Radial: Menge/Qualität | n | W, B | Fi, B | 1 |
| 13 | `d50_fields` | Zentriert: Anzahl/Qualität | n | W, B | Fi, B | 1 |
| 14 | `slider_popups` | senkrechte Popup-Schieber ▾ | o | B | B | 2 |
| 15 | `limits` | Grenzen/Klemmen je Feld (Tabelle) | r | W | S | 0 |
| 16 | `preview` | Vorschau-Kästchen an/aus | r | B | B | 1 |
| 17 | `remember` | gemerkte Werte je Dialog | r | J | S | 0 |
| 18 | `enter_esc` | Enter/Esc, Abbrechen ohne Wirkung | r | J | D | 0 |
| 19 | `tab_order` | Tab-Folge | f | W | S | 1 |
| 20 | `live_vs_menu` | als Live-Filter gleiche Felder | o | W | Fi | 1 |
| 21 | `result_ref` | Ergebnis = RE-FX | r | E (RE-FX) | E | 0 |
| 22 | `undo_names` | Verlaufsnamen | r | J | H | 0 |

---

## Bewusst anders (🎨) – Sammelliste für alle Einheiten

| Bereich | Quelle | gilt für |
|---|---|---|
| Dialograhmen, Titel-/Fußleiste, Schatten, OK-Hervorhebung, Abstände, Schriften, Farben, 3 Designs | DES-1, `DESIGN.md` §12/§13, N11 | alle Dialoge und Bedienfelder |
| Startbildschirm | DES-2 | 48 |
| Suche in der Schriftliste, Bildlaufleisten | N19, N24 | 12, 21 |
| Größerer Nachzeichnen-Dialog, zweites Verfahren | N20, N29 (VT-6) | (eigene Funktion, nicht im Abgleich) |
| KI-Werkzeuge, KI-Karte | N12, N21 | 06, 15 |
| Batch-Verarbeitung modern | `BATCH_MODERN.md` | 49, 58 (D12) |
| Neutraler Dateibrowser-Eintrag | PARITAET_B §1 ℹ️ | 49 |
| **Ausnahme: Farbwähler-Popup = Original** | N27b | 30, 31 |

## Übergabe an PAR-O / PAR-F

- PAR-O-1 legt zuerst T01–T12 und `re\_testdateien\par\TESTFILES.md` an, dann Einheit 01.
- Je Einheit: `docs\parity\<kennung>.orig.md` (Text, ≤ 150 Zeilen) + Bilder unter `re\_testdateien\par\ref\<kennung>\` +
  `INDEX.md` dort. Prüfpunkt-Nummern und Kürzel aus dieser Datei unverändert übernehmen.
- PAR-F: gleiche Bildnamen unter `docs\parity\img\<kennung>\`, dann
  `python -I tools\parity\compare.py <kennung>`, Ergebnis `docs\parity\<kennung>.md`.
- Teilt ein Agent eine Einheit, heißen die Teile `<kennung>-a`, `<kennung>-b` (Nummer bleibt, Prüfpunkte aufgeteilt).
