# DLG-8: Paket P-DLG8 „Auswahl“ (PM-6 D35–D39)

Slot s5, Commit `7543f7a` auf main (im Slot `0971fa3`).

## Umgesetzt

- **Anordnung D35–D39** wie in den Vorlagen 5976 und 19105–19108 (`re\bedienfelder\dialoge.json`):
  - Beschriftung rechtsbündig vor einem schmalen Feld (19 DLU), „Pixel“ dahinter.
  - OK und Abbrechen rechts untereinander (y 7/24 DLU).
  - Client-Größe je Vorlage: 205/214/214/187/226 × 46 bzw. 50 DLU.
  - Ist eine Beschriftung in einer anderen Sprache länger, rücken Feld, Einheit und Knöpfe nach rechts und der Dialog wird breiter. Kein Überlauf in de, en und qps.
  - Neue Datei `crates\fp-ui\src\dialogs\marquee.rs` (`LAYOUT`, eine `CustomLayout`), in `mod.rs` eine `pub mod`-Zeile. In `catalog.rs` an den 5 Zeilen nur `.with_layout(super::marquee::LAYOUT)` angehängt und die Schlüssel getauscht.
  - D36 und D37 haben im Original dieselbe Form (Bild `pm6_D36`). Sie nutzen die Anordnung mit, damit alle fünf gleich aussehen.
- **Beschriftungen:**
  - D38 „Breite:“ / en „Width:“ (Schlüssel `dlg-lbl-border-width`, ersetzt `dlg-lbl-border` „Rand:“).
  - D39 „Auswahlradius:“ / en „Sample radius:“ (Schlüssel `dlg-lbl-selection-radius`, ersetzt `dlg-lbl-sample-radius` „Stichprobenradius:“).
  - Das englische Original ist nicht gemessen. „Sample radius:“ bleibt deshalb, bis es nachgemessen ist.
- **Grenzen:**
  - 19105–19108: 1…100 wie in `re\11` §3.5.5. Firepixel klemmt beim Übernehmen des Felds, das war schon so.
  - Weiche Kante: 1…1000. Die Vorlage nennt keinen Bereich, deshalb unverändert.
  - Vorgabe 10, gemerkte Werte unverändert (DLG-5 bzw. FV-4).
- **Verlaufsnamen wie im Original** (Befund „Verlaufsnamen“, Teil Auswahl):
  - „Weiche Auswahlkante“, „Auswahl erweitern“, „Auswahl verkleinern“, „Rand auswählen“, „Auswahl glätten“.
  - en: „Feather Selection“, „Expand/Contract Selection“, „Border Selection“, „Smooth Selection“.
  - Code: `fp-engine\src\commands\select.rs`, `CommandOutput::labeled` mit `msg-step-*-selection` in `meldungen.ftl`.
  - D40 „Auswahl speichern“ ist nicht angefasst (KV-3).
- **Tests** in `crates\fp-ui\tests\dialogs_dlg8.rs`, 5 von 5 grün:
  - Beschriftungen de/en
  - Anordnung und Breite (± 12 px zum Original), Überlauf de/en/qps
  - Grenzen
  - OK über den Dialog: ein Schritt, Verlaufsname, Rückgängig
  - Wirkung der Befehle: Erweitern +1/+6/+11, Verkleinern −1/−5/−10, Rand, Glätten, Weiche Kante; englische Verlaufsnamen
- **Auch geprüft:**
  - grün: dialogs_dlg1, dlg5, enter, move, open, sliders, warnung
  - grün: fp-engine select/commands/all_commands/warnungen
  - grün: fp-i18n, Scanner `i18n_keine_festen_texte`, fp-neutral
  - `cargo check --workspace --all-targets` ohne Fehler und Warnungen

## Offene Frage aus PM-6: Erweitern um 5 → +6 px

**Ergebnis: nicht korrigiert, bleibt Befund für die Nachprüfung im Original.**

- In `re\06-bitmap-filter.md` und `re\10-werkzeuge-befehle.md` steht nichts zum Pixelbetrag. In 10 stehen nur die Befehle 35405–35408 → `selectAdjustPixelSel`.
- Die einzige Beschreibung ist `re\05-live-effekte.md` §2.2: Morphologie `@00835190` mit `SelectionAdjustTolerance` (Standard 1). Firepixel setzt sie in `fp-raster\src\mask.rs::morph` genau so um:
  - Ausdehnen: Dreiecksfilter mit r = max(1, trunc(1,2·Betrag)), danach v > t.
  - Bei 5 ist r = 6. Der äußerste Ring hat v = 255/49 = 5 > 1, also +6.
  - Bei 10 ist r = 12. Bei Abstand 12 ist v = 1, nicht > 1, also +11. Bei 1 ergibt sich +1.
  - Das deckt sich genau mit der PM-6-Messung. +6 ist also keine Off-by-one-Abweichung, sondern folgt aus der Spezifikation.
- Offen ist nur, ob die Auswahlbefehle im Original wirklich diese Morphologie nutzen. In 05 ist sie fürs Innere Glühen belegt, die Zuordnung zu den Auswahlbefehlen ist nicht belegt.
- **Begrenzen** (Rand): r = trunc(0,6·5) = 3, danach mit r erneut weichgezeichnet. Die Grenzen wachsen dadurch um 6, der Rand ist weich. Gleiche Frage.
- **D37 Verkleinern** ist nicht betroffen: r = Betrag, 330,210,380,260 → 335,215,375,255, genau −5 wie im Original.
- **D36 Erweitern:** Wert wie oben, keine Änderung.

## Zeilen für PARITAET_B (umgesetzt, Nachprüfung ausstehend)

- D35 Weiche Kante: Anordnung, Verlaufsname „Weiche Auswahlkante“ (DLG-8 `7543f7a`)
- D36/D37: Anordnung, Verlaufsnamen „Auswahl erweitern“/„Auswahl verkleinern“ (Wirkung war schon ✅)
- D38 Auswahlrahmen begrenzen: „Breite:“, Anordnung, Verlaufsname „Rand auswählen“
- D39 Auswahlrahmen glätten: „Auswahlradius:“, Anordnung, Verlaufsname „Auswahl glätten“

## Erledigte Befunde (BEFUNDE.md)

- „(von PM-6, PARITAET_B D27–D31/D38/D39/D41/D45–D50, klein) **Beschriftungen und Anordnung wie im Original** …“: Teil D38 „Breite“ und D39 „Auswahlradius“ erledigt (DLG-8, `7543f7a`).
- „(von PM-6, PARITAET_B D29–D31/D35–D50, ℹ️) **Verlaufsnamen.** …“: Teil Auswahl D35–D39 erledigt (DLG-8, `7543f7a`). Offen bleiben Pfade, Animieren und „Auswahl speichern“ (KV-3).
- „(von PM-6, PARITAET_B D36/D38, zu prüfen) **Auswahlrahmen erweitern/begrenzen um 5 px vergrößert die Grenzen um 6 px** …“: geklärt, dass es aus `re\05` §2.2 folgt (siehe oben). Bleibt offen für die Messung im Original.

## Nachprüfung (NACHPRUEFUNG)

1. Pixelauswahl 330,210,380,260, dann Auswählen ▸ Weiche Kante… / Auswahlrahmen erweitern… / verkleinern… / begrenzen… / glätten….
   - Erwartet: Beschriftung rechtsbündig vor dem schmalen Feld, „Pixel“ dahinter, OK oben rechts, Abbrechen darunter.
   - Breite ≈ 314/327/327/287/345 px. Texte „Radius:“, „Erweitern um:“, „Verkleinern um:“, „Breite:“, „Auswahlradius:“.
   - In en und qps nichts abgeschnitten.
2. In jedem dieser Dialoge 5 eintippen, Enter.
   - Erwartet als Verlaufsname: „Weiche Auswahlkante“, „Auswahl erweitern“, „Auswahl verkleinern“, „Rand auswählen“, „Auswahl glätten“.
   - Strg+Z stellt die Auswahl her.
3. **Im Original messen:** Grenzen nach „Auswahlrahmen erweitern“ um 1/5/10 und „begrenzen“ um 5. Weg: Auswahl in Pfad umwandeln über die Oberfläche statt JSF, oder Pixel auszählen.
   - Firepixel: +1/+6/+11, begrenzen +6 mit weichem Rand.

## Neue Befunde

- (klein) 19105–19108: Das Original klemmt einen Wert außerhalb 1…100 **beim OK**: Das Feld wird korrigiert, der Dialog bleibt offen (`re\11` §3.5.5). Firepixel klemmt beim Übernehmen des Felds und schließt bei OK mit dem geklemmten Wert. Für ein Offenbleiben müsste `form.rs` geändert werden, deshalb hier nicht umgesetzt.
- (ℹ️) Englische Beschriftung von D39 („Sample radius:“) und englische Verlaufsnamen sind nicht am Original gemessen.
