# FX-2 – Abschrägung und Prägung (Nutzerbefund N28, Gruppe 2)

Stand 13.10.2026. Grundlage: `re\23-effekte.md` Gruppe 2 (14 Fälle × Vektor/Text/Bitmap = 42 Referenz-PNG),
`re\05-live-effekte.md` §3 (Vorgabeliste `EFDv`), Dialogbilder `re\_testdateien\par\ref\fx-2-abschraegung-praegung\`.

## Ergebnis in Kürze

- **Effektrechnung (fp-effects) war schon gleich:** Alle 42 Fälle mit derselben Effektliste wie im Original liegen bei
  höchstens **2 Stufen** je Kanal (vormultipliziert), 0 Pixel mit Abweichung > 3. Höhenprofil je Kantenform,
  Lichtwinkel, Kontrast, Weichheit, Breite, Rundungen und Tastenstatus stimmen. Kein Umbau an `bevel.rs` (dort arbeitet
  auch FX-1 am Glühen).
- **Die Abweichung kam aus dem Dialog (fp-ui `dialogs\effects.rs`):**
  1. Prägung: Feld „Weichheit“ schrieb `AngleSoftness` (glättet die Normalen) statt `MaskSoftness` (weichzeichnen der
     Licht-/Schattenmaske, re\23). Vorgabe stand auf 3 statt 2.
  2. Prägung: Vorgabe Breite 10 statt 2.
  3. Neue Effekte bekamen nur die Dialogfelder. Fehlende Schlüssel nahm der Renderer aus den Lese-Vorgaben:
     u. a. keine graue Fläche bei der vertieften Prägung (`EmbossFaceColor #0000001e` fehlte), `MaskSoftness 0`.
     Das Original speichert immer die vollständige Liste.
  4. Feldreihenfolge wich ab (Farbe stand bei „außen“ am Ende).
- **Behoben:** Neuer Effekt schreibt die vollständige Vorgabeliste (`effects::bevel_defaults`, 20 Schlüssel wie `EFDv`),
  Prägung Weichheit ↔ `MaskSoftness` (Vorgabe 2), Breite 2. Beim Wechsel erhaben ↔ vertieft im Feld „Art“ wird die
  Fläche der neuen Art gesetzt. Reihenfolge wie im Original: [Kantenform], Breite, [Farbe nur außen], Kontrast,
  Weichheit, Winkel, Tastenstatus bzw. „Objekt anzeigen“. Grenzen: Breite 0–35, Kontrast 0–100, Weichheit 0–10,
  Winkel 0–360, 7 Kantenformen, 4 Tastenstatus (waren schon richtig, jetzt im Test).

## Vorher/nachher je Fall

Max = größte Abweichung je Kanal (vormultipliziert, 0–255) gegen das Referenz-PNG, ganze Leinwand 240 × 180.
„Effekt“ = Effektkette mit der Liste aus `params.json` (Rechenweg der App, unverändert). „Dialog“ = Effekt, wie ihn der
Dialog mit seinen Vorgaben anlegt (vorher: alter Dialog, über `firepixel-cli run … element.addEffect` gerendert;
nachher: Dialog schreibt die Liste aus `EFDv`, die Werte sind gleich, Test `dialogs_fx2`).

| Fall | Effekt V/T/B (vorher = nachher) | Dialog vorher V/B | Dialog nachher V/B |
|---|---|---|---|
| Abschrägung innen, Vorgabe | 0 / 0 / 0 | 0 / 0 | 0 / 0 |
| Abschrägung innen, klein (1, 25 %, 0, 45°, Glatt) | 0 / 0 / 0 | – | – |
| Abschrägung innen, groß (35, 100 %, 10, 300°, Ring) | 2 / 0 / 0 | – | – |
| Abschrägung innen, gedrückt | 0 / 0 / 0 | – | – |
| Abschrägung innen, Rüsche | 1 / 0 / 1 | – | – |
| Abschrägung außen, Vorgabe | 0 / 0 / 0 | 0 / 0 (mit `#df0000`, siehe Annahme 1) | 0 / 0 |
| Abschrägung außen, klein | 1 / 1 / 1 | – | – |
| Abschrägung außen, groß (+ `#3366cc`) | 1 / 1 / 1 | – | – |
| Prägung erhaben, Vorgabe | 0 / 0 / 0 | **191 / 191** | 0 / 0 |
| Prägung erhaben, klein | 0 / 0 / 0 | – | – |
| Prägung erhaben, groß (Objekt anzeigen) | 0 / 0 / 1 | – | – |
| Prägung vertieft, Vorgabe | 0 / 0 / 0 | **191 / 191** | 0 / 0 |
| Prägung vertieft, klein | 0 / 0 / 0 | – | – |
| Prägung vertieft, groß | 0 / 0 / 1 | – | – |

(V/T/B = Vektor/Text/Bitmap. Die Abweichungen 1–2 liegen an einzelnen Eckpixeln, wie in 05 §11 für die Profile 1/2/5/6
beschrieben; Ziel ≤ 2–3 erfüllt.)

## Tests

- `crates\fp-effects\tests\fx2_parity.rs`
  - `golden_fx2`: 10 Fälle × 3 Objekte = 30 Referenzbilder in `crates\fp-effects\tests\data\fx2\` (Ergebnisbilder des
    Originals ohne Metadaten neu gespeichert, Objekt als `base_<objekt>.png`, Liste `cases.json`), erlaubt 2 Stufen:
    je Effekt Vorgabe + Extremfall „gross“, dazu „gedrückt“ und „Rüsche“.
  - `slopes_and_button_states_distinct_and_deterministic`, `bevel_limits` (Kontrast 0 = unverändert, Weichheit > 10
    wie 10, Breite > 60 wie 60, Breite 0 ohne Panik, Ausdehnung außen = Breite).
  - `sweep_fx2` (`--ignored`): alle 42 Fälle direkt aus `re\_testdateien\par\fx\` (`FX2_OUT`, `FX2_FILTER`).
- `crates\fp-ui\tests\dialogs_fx2.rs` (§4b): Vorgaben, Grenzen und Feldreihenfolge aller vier Dialoge, vollständige
  Vorgabeliste nach OK; Prägung Weichheit → `MaskSoftness` bei 50 Änderungen = 1 Effekt, 1 Verlaufsschritt,
  Rückgängig; Abschrägung Weichheit/Kantenform/Tastenstatus; Bearbeiten vertieft → erhaben setzt die Fläche, derselbe
  Eintrag, Rückgängig/Wiederholen exakt; Abbrechen stellt exakt her; Auswahlwechsel bei offenem Dialog wirkt auf das
  beim Öffnen gewählte Element.
- `cargo check --workspace --all-targets`, `cargo test -p fp-neutral`, `cargo test -p fp-effects`,
  `cargo test -p fp-ui --test all -- dialogs_fx2:: dialogs_effects::` grün; clippy ohne Warnungen in den eigenen Dateien.

## Gemeinsame Dateien

- `crates\fp-ui\src\dialogs\effects.rs`: nur der Zweig der Abschrägungsfamilie (`effect_params` für `BevelType` 0–3,
  `effect_form` Zweig `_`) und die neue öffentliche Funktion `bevel_defaults`. Glühen/Schatten (FX-1) unberührt.
- `crates\fp-effects\src\bevel.rs`: nicht geändert (FX-1 ändert dort das Glühen-Band).

## Annahmen

1. **Farbe „Abgeflachte Kante außen“ bei neuem Effekt:** Firepixel nimmt weiter die Füllfarbe des Werkzeugs
   (bisheriges Verhalten), `EFDv` hat `#df0000`. Ob das Menü des Originals die Füllfarbe einsetzt, ist nicht gemessen
   (re\23 setzt per JSF). Zur Nachprüfung vorgemerkt.
2. Das zusätzliche Feld „Art“ (innen/außen bzw. erhaben/vertieft) gibt es im Original nicht; es bleibt als
   Firepixel-Komfort (Bewertung „bewusst anders“, Frage an den Nutzer). Die Form selbst wechselt dabei nicht (Farbfeld erscheint erst nach
   erneutem Öffnen).
3. Prägung Weichheit ↔ `MaskSoftness` wie in re\23 (dort als Annahme vermerkt, durch die Rücklesung „gross“
   `MaskSoftness 10`, `AngleSoftness 3` gestützt).

## PARITAET-Zeilen „umgesetzt, Nachprüfung ausstehend“

- Live-Effekte: Abgeflachte Kante innen/außen, Relief-/Senkprägung (Ergebnis pixelgleich ≤ 2, Dialogvorgaben,
  Feldzuordnung, Reihenfolge, Grenzen).

## Nachprüfung (NACHPRUEFUNG)

- Echtes Fenster/Brücke: Rechteck wählen, „+“ → Abschrägung und Prägung → Senkprägung, OK; Export PNG 32 gegen
  `re\_testdateien\par\fx\2-abschraegung-praegung\praegung-vertieft_vorgabe_vektor.png` (mit `fx_vector.fw.png`):
  erwartet max ≤ 2. Ebenso Reliefprägung, Weichheit im Dialog auf 10 → Ränder werden weicher (nicht die Winkel).
- Original: Farbe der neuen „Abgefl. Kante außen“ über das Menü (Füllfarbe oder `#df0000`?).

## Neue Befunde

- **Text-Rasterung (fp-text/fp-render, nicht FX-2):** Sobald ein Textobjekt geändert wird (z. B. Effekt hinzufügen oder
  nur Deckkraft 100 setzen), rastert Firepixel den Text neu; gegen das Bild des Originals weicht der Alphawert an
  431 Pixeln um bis zu 159 ab (`fx_text.fw.png`, „Fx Ag“ Arial-BoldMT 40 pt, Glättung „smooth“). Unverändert wird das
  gespeicherte Bild gezeigt (gleich). Deshalb weichen alle Gruppe-2-Fälle auf Text in der App um bis zu 255 ab,
  obwohl die Effektrechnung auf dem Objektbild des Originals gleich ist. Prüfen:
  `firepixel-cli run fx_text.fw.png --cmd select.all --cmd element.setOpacity --params {"opacity":100} --out x.png`
  gegen das Basisbild.
- Menünamen weichen vom Original ab („Abschrägung innen/außen“, „Prägung erhaben/eingelassen“ statt „Abgeflachte Kante
  innen“, „Abgefl. Kante außen“, „Reliefprägung“, „Senkprägung“; Kantenform „Abfallend/Rahmen 1/Rahmen 2/Kräuseln“).
  Nicht geändert (Glossar-/Menü-Einheit), Frage an den Koordinator.
