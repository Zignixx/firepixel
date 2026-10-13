# Bericht PINSEL-1 (13.10.2026, Nutzerbefund N31)

Pinsel, Bleistift und Radiergummi malen live; alle Werte des Inspektors wirken auf den gemalten
Strich; „Pinselstrich bearbeiten…“ ist ein funktionierender Dialog mit drei Registerkarten.
Spezifikation: `re\07-vektor-rendering.md` §6/§7 (Stempel-Engine, Mischmodi), `re\24-pinsel.md`
mit `presets.csv` (54 Arten), Dialogvorlagen 5963/5964/5965.

## Commits (Slot fp-s11, per cherry-pick auf main)

1. `PINSEL-1: Pinsel, Bleistift und Radiergummi malen live (N31) …` – Mal-Engine `fp-engine\src\paint.rs`,
   BrushTool live, `bitmap.paint`/`bitmap.erase` neu, Leinwand-Vorschau `fp-ui\src\canvas\paint_preview.rs`
2. `PINSEL-1: Dialog "Pinselstrich bearbeiten" mit Registerkarten …` – `fp-ui\src\dialogs\brush_edit.rs`
3. `PINSEL-1: Stricharten nach re\24 - 12 Kategorien, 54 Arten …` – Vorlagentabelle, Namen de/en
4. dieser Bericht

## Umgesetzt

- **Live malen:** Beim Drücken legt das Werkzeug den ganzen Auftrag fest (Pinsel aus Strichvorgabe bzw.
  Vorgabepinsel, Textur + Menge, Strichfarbe, Deckkraft, Mischmodus, „Transparenz beibehalten“; beim
  Bleistift „Automatisch löschen“; beim Radiergummi Größe/Form/Rand/Deckkraft). Je Mausbewegung rechnet
  `LivePaint` nur das Endstück des Zugs (plus schmale Randstreifen, wenn die Strichfläche wächst) in
  256er-Kacheln; die Leinwand lädt nur geänderte Kacheln hoch und lässt die Vorschau nach dem Loslassen
  stehen, bis das Hintergrundbild den Strich zeigt. Die blauen Kreis-Umrisse (Bild 9) sind weg; der
  Cursor-Kreis der Spitzengröße bleibt. Messung 4000 × 3000-Bitmap, Größe 35: ≈ 13 ms je Bewegung
  (Testprofil, optimiert), größter neu gerechneter Bereich 0,19 Mio. px.
- **Ein Verlaufsschritt:** Loslassen schickt denselben Auftrag als `bitmap.paint` bzw. `bitmap.erase`;
  das Ergebnis ist pixelgleich mit der letzten Vorschau (Tests). Esc bzw. Werkzeugwechsel mitten im Zug
  verwirft ihn ohne Schritt. Ohne Änderung (z. B. „Transparenz beibehalten“ auf leerer Fläche) kein Schritt.
- **Alle Inspektor-Werte wirksam:** Strichart (54 Arten, simple/natural-Engine aus fp-vector), Größe,
  Kante/Rand, Textur + Menge, Deckkraft (ganzer Zug, Überlappung ohne Verstärkung), Mischmodus (alle
  Modi aus 07 §7, z. B. Durchschnitt, Multiplizieren), „Transparenz beibehalten“ (Alpha der Bitmap bleibt),
  Pixelauswahl beschneidet. Gilt ebenso für Bleistift (1 px, Kantenglättung, Deckkraft, Mischmodus,
  Transparenz beibehalten, Automatisch löschen) und Radiergummi (Vorschau zeigt Radiertes über der Leinwandfarbe).
- **`bitmap.paint`** neue Parameter `brush`, `texture`, `blend`, `preserveTransparency`, `trimEnd`, `tool`;
  der alte Parameter `draw` wird in einen Pinsel übersetzt (MCP/Skripte bleiben lauffähig).
- **Stricharten nach re\24:** 12 Kategorien, 54 Arten in Namen/Reihenfolge des Originals mit allen Werten
  aus `presets.csv`; Artwechsel behält die Größe; frühere Namen („Kohle/Pastell“, „Einfach/Harte Linie“,
  `bn_1-Pixel Hard` …) gelten als Alias weiter. „Pinselstrichoptionen…“ am Ende des Strichart-Menüs.
- **Dialog „Pinselstrich bearbeiten“** (Lage nach 5963–5965): Optionen (Tintenmenge, Abstand, Farbfluss,
  Übermalen, Textur, Randtextur, Randeffekt in Original-Reihenfolge, Spitzen, Spitzenabstand, Variation,
  Gestrichelt + 3 × Ein/Aus, Grau-Zustände wie re\24 Nr. 20), Form (Spitzenvorschau, Viereckig, Größe,
  Rand, Breite, Winkel), Empfindlichkeit (7 Eigenschaften × Druck/Geschwindigkeit/Horizontal/Vertikal/
  Zufällig); Probestrich mit der Mal-Engine. Aus dem Werkzeug-Inspektor wirkt er auf die Werkzeugvorgabe –
  **auch mit ausgewählter Bitmap** (vorher kam nur ein Hinweis: „ohne Funktion“). OK = `pi.tool.brush {brush}`
  (kein Dokumentschritt, Werte gemerkt) bzw. bei Objekten `pi.brush {brush}` (ein Schritt); Abbrechen verwirft.
- **Sperren/Mehrfachauswahl:** Ziel ist die oberste ausgewählte, ungesperrte Bitmap (Vorschau und Befehl
  gleich); gesperrte Objekte zählen wie in der Sitzung als nicht ausgewählt und bleiben unverändert.

## Tests (alle grün)

- `cargo test -p fp-engine --test all paint_live_pinsel1::` (15): Vorschau = Ergebnis über viele Bilder,
  Weichheit, Größe 1/10/50, Deckkraft 21 % ohne Verstärkung, Durchschnitt/Multiplizieren/Normal,
  Transparenz beibehalten, Textur Menge 0/100, 50× Werte ändern, Esc, Rückgängig/Wiederholen-Folgen,
  Mehrfachauswahl/gesperrte Bitmap/gesperrte Ebene, Bleistift + Automatisch löschen, Radiergummi,
  Umschalt-Linie, Einzelklick, Dialogwerte, große Bitmap; dazu `paint::tests`.
- `cargo test -p fp-ui --test all brush_live_pinsel1::` (4 + Bilder `shots`, ignoriert): echtes Ziehen mit
  Zeigerereignissen über ≥ 30 Bilder, Inspektorwerte (Weich abger. 35, Kante 37, Deckkraft 21,
  Durchschnitt), Klick auf „Pinselstrich bearbeiten“ mit ausgewählter Bitmap, Registerkarten, 50×
  ändern + Abbrechen/OK, Empfindlichkeit. `dialogs_tool_brush.rs` auf den neuen Dialog umgestellt.
- Außerdem grün: fp-engine `--lib`, `tools::`, `canvas_tools::`, `commands::`, `all_commands::`; fp-ui
  `bitmapwerkzeuge_fv11::`, `inspector_paritaet::`, `inspector_i18n::`, `dialogs_move::`, `--lib inspector::
  canvas:: dialogs::`; `cargo test -p fp-i18n`, Scanner `i18n_keine_festen_texte` (0 feste Texte),
  `cargo test -p fp-neutral`, `cargo check --workspace --all-targets` ohne Warnungen.

## Befunde erledigt

- N31 „Der Bitmap-Pinsel zeichnet nicht live …“ – erledigt (PINSEL-1), Nachprüfung im echten Fenster offen.
- INVENTORY 01 `stroke_live`, `opacity`, `blend`, `preserve_alpha`, `esc`, `undo`; 02 `category_list`,
  `type_list`, `edit_open`, `edit_options`, `edit_shape`, `edit_sensitivity`, `edit_apply`, `edit_cancel`,
  `remember` (Sitzung), `disabled_states` (Abstand/Farbfluss/Spitzenabstand/Variation/Ein-Aus):
  → in PARITAET auf „umgesetzt, Nachprüfung ausstehend“.

## Für NACHPRUEFUNG.md (echtes Fenster, Brücke)

| Was | Ablauf | Erwartet |
|---|---|---|
| Pinsel live (N31) | Pinsel, Strich 35, Weich abger., Rand 37; auf leerer Fläche langsam und schnell ziehen | Strich erscheint während des Ziehens fließend unter dem Cursor, keine Kreis-Umrisse; Loslassen: kein Flackern, ein Verlaufsschritt „Pinsel“ |
| Große Bitmap | 4000 × 3000-Foto öffnen, Bitmap wählen, Größe 35 quer über das Bild ziehen (100 % und 400 %) | flüssig (kein Ruckeln), Strich bleibt nach dem Loslassen gleich |
| Inspektorwerte | Deckkraft 21, Durchschnitt, Transparenz beibehalten an, Textur Herz 01 Menge 50 auf halbtransparenter Bitmap | Strich halbdurchsichtig, gemischt, außerhalb deckender Pixel nichts, Textur sichtbar |
| Esc/Rückgängig | Mitten im Zug Esc; dann Zug, Strg+Z, Strg+Y | Esc: nichts gemalt; Rückgängig/Wiederholen exakt |
| Radiergummi live | Bitmap wählen, Radiergummi 16, ziehen | Radiertes ist während des Ziehens sichtbar |
| Dialog | Werkzeug-Inspektor „Pinselstrich bearbeiten“ mit gewählter Bitmap; Register wechseln; Form: Größe 30, Viereckig; OK; ziehen | Dialog öffnet (kein Hinweis), Strich 30 eckig; Werte beim erneuten Öffnen da |
| Stricharten | Strichart-Menü öffnen | 12 Kategorien, 54 Arten wie re\24, „Pinselstrichoptionen…“ unten; Artwechsel behält die Größe |
| de/en/qps | Dialog in allen Sprachen und 3 Designs | nichts abgeschnitten/überlappend |

## Offene Abweichungen zu re\24 / Annahmen

- Druck/Geschwindigkeit: die Maus liefert Druck 1 und Geschwindigkeit 0 (fp-vector-Dynamik);
  Empfindlichkeiten auf Druck/Geschwindigkeit wirken deshalb mit der Maus nicht, Zufall/Richtung schon.
  Geschwindigkeit aus der Zeigerbewegung (55-px-Fenster, 10 §6) ist offen.
- Werte nur der natürlichen Engine (Spitzen, Variation, Breite, Winkel, Empfindlichkeiten) schalten einen
  einfachen Pinsel im Dialog auf die natürliche Engine um (Annahme; im Original nicht gemessen).
- „Tintenmenge“ in der Liste Pinselstricheigenschaft = Deckkraft (`O`); im Original offen (re\24 Nr. 12).
- Original zeigt im Dialog „Cancel“ englisch und „Optionen “ mit Leerzeichen – bewusst „Abbrechen“/„Optionen“.
- Mischmodus-Liste: Namen weichen teils ab (Original „Auflösen“, „Überlagern“, „Luminanz“, „Farbton“ statt
  Tönung, „Löschen“; 56 Einträge) – nicht geändert (gemeinsam mit dem Objekt-Inspektor, eigener Auftrag).
- Popups von Größe/Rand/Menge/Deckkraft als senkrechte Schieber 24 × 222 (re\24 Nr. 9) – nicht geändert.
- Knopf „Pinselstrich bearbeiten“ im Inspektor ist im deutschen Layout zu schmal („…bearbeit…“, Bild 8).
- Vorgabe des Pinsels im Original nach Neustart „Weich abger., Größe 1, Rand 100“; Firepixel merkt die
  Werkzeugwerte nur in der Sitzung (kein Speichern in den Voreinstellungen) und startet mit „1-Pixel hart“.
- Vorschau liegt über allen Objekten (verdeckte Teile der Bitmap erscheinen während des Zugs oben;
  Deckkraft/Mischmodus/Effekte des Bitmap-Objekts selbst fließen nicht in die Vorschau ein).
- Neue Bitmap auf gesperrter aktiver Ebene: wie bei den Zeichenwerkzeugen nicht abgefangen.
- Bitmap-Pinsel folgt der Polylinie der Mauspunkte (keine Bézier-Anpassung wie `BezFitTolerance2`).

## Neue Befunde / Hinweise für andere Bereiche

- fp-vector (RF): `InkBuffer::stamp`/`ArgbInk::stamp` überspringen Stempel ganz außerhalb des Fensters
  (nur Tempo, Ergebnis unverändert) – Korpus-Pixeltests (`--ignored`) bei RF einmal laufen lassen.
- fp-ui `dialogs\mod.rs` (IBD): neues Feld `brush_edit`, `PendingForm::StrokeOptionsTool`,
  `open_stroke_options_for_tool`; `misc::stroke_options_form` wird nicht mehr benutzt.
- fp-engine `commands\mod.rs`: `pub(crate) mod bitmap` (Mal-Engine nutzt Radiergummi-Abdruck).
- `ToolOptions`: neue Felder `paint_blend`, `keep_transparency`, `auto_erase` (UI überträgt sie aus den
  Werkzeugattributen in `canvas\overlay.rs::sync_tool_attrs`).
