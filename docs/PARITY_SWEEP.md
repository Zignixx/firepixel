# Vollabgleich Original ↔ Firepixel (PAR) – Verfahren

Nutzerwunsch 13.10.2026 ≈ 03:45: **alle** Funktionen, Unterfunktionen, Knöpfe, Felder, Filter usw. in beiden
Programmen nebeneinander ansehen, jeden Unterschied notieren, beheben und gegenprüfen. Gründlich, aber kein
Agent darf mit Bildern überladen werden: klein geschnitten, sortiert, Ergebnisse als Text.

## Grundidee: Fließband in vier Stufen

| Stufe | Agent | läuft | Ergebnis |
|---|---|---|---|
| **O – Original aufnehmen** | PAR-O-<einheit> | **nacheinander** (Sperre, nur einer am Original) | `docs\parity\<einheit>.orig.md`: je Prüfpunkt das Verhalten des Originals als Text (Werte, Grenzen, Texte, Reihenfolge, Grau/aktiv, Ergebnis-Hashes), höchstens wenige Bildausschnitte |
| **F – Firepixel vergleichen** | PAR-F-<einheit> | parallel, sobald `.orig.md` da ist | `docs\parity\<einheit>.md`: Tabelle Prüfpunkt · Original · Firepixel · ✅/❌ · Befund-ID; neue Befunde in BEFUNDE (Bereich PAR) |
| **X – beheben** | PAR-X-<bereich> | parallel, je Bereich ein Agent mit klaren Dateigrenzen | Commits + Tests nach §4b |
| **R – gegenprüfen** | PAR-R-<einheit> | parallel, nur Firepixel (Original-Aufnahme liegt ja vor) | ❌ → ✅ in `docs\parity\<einheit>.md`, Rest begründet |

Die Original-Aufnahme ist der Engpass (eine Sperre). Deshalb läuft Stufe O **ununterbrochen**, alles andere dahinter.

## Einheiten (≈ 50, endgültige Liste macht PAR-0)

Je Einheit **höchstens 25 Prüfpunkte**. Grundlage: `crates\fp-engine\data\commands.json` (alle Befehle), `PARITAET_A.md`,
`PARITAET_B.md`, `NACHPRUEFUNG.md`, `re\*.md`.

- **Werkzeuge (≈ 13):** Zeiger/Unterauswahl/Skalieren/Zuschneiden · Auswahlrahmen/Lasso/Zauberstab/KI · Pinsel/Buntstift · Radiergummi · Weichzeichner/Schärfen/Abwedeln/Nachbelichten/Wischfinger · Stempel/Farbe ersetzen/Rote Augen · Linie/Stift/Vektorpfad/Pfad neu zeichnen · Formen + automatische Formen · Text · Freiform/Messer/Pfad umformen · Slice/Hotspot · Pipette/Farbeimer/Verlauf · Hand/Zoom
- **Inspektor je Objektart (≈ 7):** Dokument · Pfad/Rechteck · Bitmap · Text · Gruppe/Symbol/Instanz · Slice/Hotspot · Füllung/Strich/Kanten/Texturen/Muster/Verläufe
- **Live-Effekte und Filter (≈ 4):** wie FX-1…4 (Ergebnis pixelgenau, Dialogfelder, Grenzen, Merken)
- **Bedienfelder (≈ 8):** Ebenen · Seiten/Status · Verlauf · Optimieren · Bibliotheken/Stile · Farbfelder/Mischer/Farbwähler · Info/Ausrichten/Pfade · Autoformen/Formen/Sonderzeichen/Bildbearbeitung/CSS
- **Menüs, Verhalten (≈ 6):** Datei · Bearbeiten · Ansicht · Auswählen · Modifizieren · Text/Befehle/Filter/Fenster/Hilfe (Beschriftungen sind in PARITAET_B schon gemessen – hier zählt die **Wirkung**)
- **Dialoge (≈ 5):** D01–D50 in Zehnergruppen, Verhalten (Felder, Grenzen, Tab, Merken, Esc/Enter, Wirkung)
- **Querschnitt (≈ 6):** Kontextmenüs · Tastenkürzel · Voreinstellungen · Bildvorschau/Export · Leinwand (Zoom, Hand, Hilfslinien, Raster, Lineale, Ausrichten) · Dateiformate Öffnen/Importieren/Speichern

## Referenzbilder als Richtlinie (Nutzerwunsch 13.10. ≈ 03:50, verbindlich)

PAR-O schreibt nicht nur Werte auf, sondern macht zu **jedem Prüfpunkt sortierte Bildschirmfotos des Originals**. Sie sind
die **Richtlinie** für Firepixel: Aussehen, Anordnung, Abstände, Texte, Zustände (normal, Maus darüber, gedrückt, grau,
aufgeklappt, Ergebnis auf der Leinwand) müssen danach nachgebaut bzw. geprüft werden.

- Ablage (außerhalb von `firepixel\`, Neutralität): `re\_testdateien\par\ref\<einheit>\<nr>_<prüfpunkt>_<zustand>.png`,
  z. B. `re\_testdateien\par\ref\werkzeug-pinsel\07_inspektor_textur_aufgeklappt.png`. Je Einheit eine `INDEX.md`
  (Nr · Prüfpunkt · Zustand · Bild · was darauf zu sehen ist · gemessene Werte).
- Gleiche Fenstergröße (1600 × 1000), gleiches Design/Zoom, gleiches Testdokument; Ausschnitt eng um das Prüfziel plus ein
  Übersichtsbild je Einheit; verlustfrei PNG, Originalgröße (nicht verkleinert gespeichert).
- PAR-F macht **dieselben Bilder mit denselben Namen** von Firepixel nach `docs\parity\img\<einheit>\` und erzeugt per Skript
  Vergleichsbilder nebeneinander + Differenz (`re\_testdateien\par\cmp\<einheit>\…`), plus eine Kennzahl je Paar.
- PAR-X und PAR-R schauen sich gezielt nur die Referenzbilder **ihrer** Prüfpunkte an (über `INDEX.md`).

Aufnehmen kostet keinen Kontext – nur **Ansehen** tut es. Darum: viele Bilder speichern, wenige ansehen.

## Regeln gegen Bilderflut (verbindlich für jeden PAR-Agenten)

1. Zustand **als Text** erfassen: Original per JSF (Werte, Grenzen, Objektbaum, `Files`-Log), Firepixel per Brücke
   (`ui_state`, `ui_find`, `get_document`, `run_script`). Bilder nur, wo Aussehen das Prüfziel ist.
2. Bilder in voller Größe **speichern** (siehe Referenzbilder), aber zum **Ansehen** nur Ausschnitte, verkleinert
   (≤ 900 px). Pixelvergleiche per Python-Skript (Differenz, Hash), nicht per Auge.
3. Höchstens **12 angesehene Bilder je Programm und Agent** (gespeichert werden dürfen beliebig viele). Reicht das nicht:
   Einheit teilen (Notiz an den Koordinator).
4. Ergebnisdatei ≤ 150 Zeilen, Tabelle statt Prosa. Gleiches Testdokument und gleiche Fenstergröße in beiden Programmen
   (`re\_testdateien\par\`).
5. Original nur unter der Sperre (`re\scripts\fwsteuerung\gui-sitzung.ps1`), je Sitzung ≤ 15 min, danach Fenster
   aufräumen; Firepixel nur über die Brücke in eigener Instanz (AGENTEN_REGELN §4d, §4e). Keine offenen Datei-Dialoge.
6. Neutralität (§1a) in allen Dateien; Bilder des Originals nur außerhalb von `firepixel\` ablegen
   (`re\_testdateien\par\img\`), in `docs\parity\img\` nur Firepixel-Bilder.

## Zeitbedarf (Schätzung)

PAR-0 Inventar ≈ 40 min. Stufe O ≈ 50 Einheiten × 10–15 min ≈ **9–12 h** am Stück (Engpass). F, X, R laufen dahinter und
enden ≈ 1–2 h nach der letzten Original-Aufnahme. Gesamt ≈ **11–14 h**, also über die Nacht hinaus bis in den Tag.
Reihenfolge der Einheiten: zuerst, wo der Nutzer Fehler gefunden hat (Werkzeuge/Pinsel, Inspektor, Effekte/Filter,
Farbwähler, Bedienfelder, Bildvorschau), dann Querschnitt, dann Menüs/Dialoge (schon teilweise gemessen).
