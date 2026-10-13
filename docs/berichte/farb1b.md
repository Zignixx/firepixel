# FARB-1b: Überall der Farbwähler des Originals (Nutzerbefund N27b)

Vorgabe: `re\22-ansicht-bitmap-farbe.md` §2, Referenzbilder `re\_testdateien\par\ref\color-popup\`
(angesehen: 01, 05, 07, 20; Nutzerbilder 5/6), Rasterfarben `grid_21x15.csv`. Laut `PARITY_SWEEP.md`
gilt beim Farbwähler ausdrücklich das Original; Rahmen, Fläche und Schrift kommen aus dem
Firepixel-Design (3 Designs).

## Umgesetzt

Neues Modul `crates\fp-ui\src\inspector\color_picker.rs` (ein Popup für alle Farbfelder);
`inspector\popups.rs` (`color_button`, `color_popup`, neu `color_button_with`) leitet dorthin um.

- **Aufbau 259 × 277** wie gemessen: Raster 21 × 15 (Zellen 11 px im 12-px-Raster, Rahmen 646464,
  Formel §2.2; alle 315 Zellen gleich der CSV – dafür werden die Stufen als ganze Werte mit Hälften
  zur geraden Zahl gebildet und die reine Farbe zuerst gerundet), Vorschaufeld, „HEX:“ mit
  Kopierknopf, Deckkraft (Schachbrett, Feld, Klappknopf mit senkrechtem Schieber 24 × 222, oben =
  100), „RGB:“ R/G/B mit Kopierknopf, Trennlinie, Fußleiste je Aufruf (`Footer`):
  - Werkzeugleiste Füllung: Ohne, Einfarbig (gedrückt, lässt offen), Verlauf, Muster, Farbkreis
  - Kontur (Werkzeugleiste, Inspektor Strich, Text-Strich): Ohne, Strich innen/mittig/außen, Farbkreis
  - Inspektor Füllung, Text, Effekte, Filter-/andere Dialoge, Mischer: nur Farbkreis
  - Farbfelder mit „keine Farbe“ (Leinwand transparent, Mattfarbe Optimieren/Bildvorschau, Bleistift): Ohne + Farbkreis
- **Pipette sofort beim Öffnen:** eigener Pipettenzeiger (außer über den Eingabefeldern, dort
  I-Balken); Vorschau, HEX und RGB zeigen live die Farbe unter dem Zeiger: Rasterzelle (heller
  Rahmen), Leinwand (genaues Dokumentpixel der Seite, auf Weiß), übriges App-Fenster (Bildschirmfoto
  des Fensters, aufgefrischt wenn der Zeiger sich bewegt, höchstens alle 0,4 s). Über dem
  Popup-Hintergrund/Feldern: Arbeitsfarbe.
- **Klick auf Rasterzelle** bzw. **außerhalb des Popups** übernimmt und schließt; ein Hintergrund
  schluckt den Klick (Leinwand/Dialog bekommen ihn nicht, Effekt-Dialog bleibt offen). Rechtsklick:
  nichts.
- **HEX/RGB/Deckkraft tippen:** live in Vorschau und den anderen Feldern, Tab R → G → B, **Enter**
  übernimmt und schließt (Deckkraft 50 → Alpha 80), **Esc** verwirft (bei offenem Schieber erst nur
  den Schieber, bei offenem Farbdialog erst nur den Dialog). Tippen ohne Feldklick wird ignoriert
  (Tastenkürzel ruhen, `input.rs`); Enter ohne Eingabe schließt ohne Änderung. Esc/Enter werden vom
  Popup verbraucht, der aufrufende Dialog schließt bzw. bestätigt nicht.
- **Kopierknöpfe:** `#RRGGBB` bzw. `rgba(r,g,b,a)` (a mit 2 Nachkommastellen, „Ohne“ →
  `rgba(0,0,0,0.00)`), danach zu.
- **Lage:** Unterkante auf der Oberkante des Farbfelds, linke Kante beim Klickpunkt, oben zu wenig
  Platz → y = 0 (verdeckt das Feld), rechts im Fenster gehalten.
- **Farbkreis:** Farbdialog (Abweichung A1), Popup bleibt dahinter offen; Esc/Abbrechen zurück ins
  Popup, Übernehmen setzt die Farbe und schließt.
- **Aufrufstellen:** Werkzeugleiste Strich/Füllung (vorher egui-Farbwähler), Inspektor (Füllung,
  Strich, Text, Text-Strich, Leinwand, Werkzeuge, Effekt-Unterfelder), Effekt- und Filterdialoge
  (`dialogs\form.rs`), Neues Dokument ▸ Benutzerdefiniert (vorher egui-Farbknopf), Mischer-Bedienfeld
  (Strich-/Füllfeld öffnen jetzt das Popup), Optimieren, Suchen/Ersetzen, Symboleigenschaften,
  Auto-Formen, Voreinstellungen, Popup-Menü-Editor, Katalog, Bildvorschau (Mattfarbe, Farbe
  hinzufügen) – alle über dieselbe Funktion.
- Ergebnis: genau ein Befehl je Wahl (ein Verlaufsschritt bzw. im Dialog ein Feldwert); das Objekt
  ändert sich wie im Original erst beim Klick, nicht beim Überfahren.

Geänderte Dateien: `crates\fp-ui\src\inspector\{color_picker.rs (neu), mod.rs (Einhängezeile),
popups.rs, obj_quads.rs, tool_quads.rs}`, `tools_panel.rs`, `panels\color.rs`,
`dialogs\new_document.rs`, `input.rs` (eine Bedingung, Datei von LW), `crates\fp-i18n\locales\{de,en}\inspector.ftl`
(`pi-color-hex`, `-rgb`, `-copy-hex`, `-copy-rgb`, `-opacity`, `-dialog-title`), Tests
`crates\fp-ui\tests\color_picker_farb1b.rs` (neu) und `color_picker_farb1a.rs` (an das neue Popup
angepasst: Farbdialog über den Farbkreis, Esc zweimal).

Commit: siehe `git log --grep "FARB-1b"` (ein Commit, Nachricht „FARB-1b: Farbwähler-Popup des Originals an allen Farbfeldern (N27b)“).

## Tests (§4b)
`cargo test -p fp-ui --test all color_picker_farb1` 15/15 (parallel, 7 s):
Raster = CSV (alle Zellen); Lage über dem Farbfeld und am oberen Rand; Rasterklick → genaue Farbe,
Dialog bleibt offen; HEX tippen live, Esc verwirft (Dialog bleibt), Enter übernimmt; RGB mit Tab +
Enter; Tippen ohne Feldklick wählt kein Werkzeug, Enter schließt ohne Änderung; Pipette über der
Leinwand live (weiß/grün) und Klick übernimmt das Pixel ohne Auswahländerung; Inspektor-Füllung per
Pipette = genau ein Verlaufsschritt; 50× wählen → höchstens ein Glühen, OK = ein Verlaufsschritt,
Effekt trägt die letzte Farbe; Werkzeugleiste, Inspektor, Neues Dokument und Effekt öffnen dasselbe
259 × 277-Popup; FARB-1a-Fälle (Ziehen im Farbdialog) weiter grün. Unit-Tests `color_picker::tests` 2/2.
Außerdem grün: `dialogs_dlg1/dlg1b/dlg5/ux/des1/effects`, `gesamt_kuerzel`, `panels_i18n`,
`i18n_keine_festen_texte` (mit ignorierten), `cargo test -p fp-i18n`, `cargo test -p fp-neutral`,
`cargo check --workspace --all-targets`, clippy ohne Meldungen in den geänderten Dateien.

## Erledigte Befunde
- N27b „überall genau der Farbwähler des Originals …“ (umgesetzt, Nachprüfung ausstehend).

## PARITAET
Farbwähler-Popup (alle Aufrufstellen): „umgesetzt, Nachprüfung ausstehend“.

## Nachprüfung (Zeilen für `NACHPRUEFUNG.md`)
- FARB-1b-1 | echtes Fenster | Rechteck wählen, Werkzeugleiste Füllfarbe anklicken | Popup 259 × 277 direkt über dem Farbfeld, linke Kante am Klickpunkt; Raster wie Referenzbild 01; Fußleiste Ohne/Einfarbig/Verlauf/Muster/Farbkreis.
- FARB-1b-2 | echtes Fenster | Popup offen, Zeiger über Leinwand, Bedienfelder, Menüleiste bewegen | Pipettenzeiger; Vorschau/HEX/RGB zeigen live die Farbe unter dem Zeiger (Leinwand exakt, übrige Flächen nach kurzem Ruhen des Zeigers); Klick übernimmt und schließt, die Leinwand reagiert nicht auf den Klick.
- FARB-1b-3 | echtes Fenster | HEX-Feld anklicken, 123456 tippen, Enter; erneut, R 255 Tab …, Esc | Enter → #123456, Esc → keine Änderung; Tippen ohne Feldklick löst keine Werkzeugkürzel aus.
- FARB-1b-4 | echtes Fenster | Effekt Glühen, Farbfeld, Pipette auf die Leinwand klicken | Glühfarbe = Pixel, Effekt-Editor bleibt offen; Esc im Popup schließt nur das Popup.
- FARB-1b-5 | echtes Fenster | Dialog mit Farbfeld nahe der oberen Fensterkante (z. B. Fenster niedrig) | Popup bei y = 0, verdeckt das Farbfeld.
- FARB-1b-6 | echtes Fenster | Deckkraft-Klappknopf, Schieber ziehen, Enter | Schieber 24 × 222 unter dem Knopf, oben = 100; Ergebnisfarbe mit Alpha; Esc schließt erst nur den Schieber.
- FARB-1b-7 | echtes Fenster | Kopierknöpfe HEX/RGB | Zwischenablage `#RRGGBB` bzw. `rgba(r,g,b,1.00)`, Popup zu.
- FARB-1b-8 | echtes Fenster | Farbkreis-Knopf | Farbdialog neben dem Popup, Popup bleibt offen; Übernehmen setzt die Farbe, Esc zurück ins Popup.
- FARB-1b-9 | Bildserie 3 Designs × de/en | Popup an Werkzeugleiste, Inspektor Strich, Glühen, Neues Dokument, Mischer | gleicher Aufbau, Fußleiste je Aufruf; nichts überläuft (HEX:/RGB: in en).

## Abweichungen (begründet)
- **A1 Systemfarbdialog:** Der Windows-Dialog `ChooseColor` ist nur über `unsafe` erreichbar (die
  Crate `windows` verlangt `unsafe`, `rfd` kennt nur Datei-Dialoge; keine andere sichere Bindung im
  Baum). Darum ein eigener Farbdialog (Farbfläche, Farbtonleiste, Übernehmen/Abbrechen) im
  Firepixel-Design.
- **A2 Pipette außerhalb des App-Fensters:** Bildschirmpixel anderer Fenster lassen sich ohne
  `unsafe` bzw. ohne neue Bildschirmfoto-Crate nicht lesen; dort zeigt das Popup die Arbeitsfarbe
  (egui bekommt außerhalb des Fensters ohnehin keinen Klick).
- **A3 Leinwand-Pipette** liest das Dokumentpixel der Seite (auf Weiß gemischt), nicht das
  Bildschirmpixel: bei Zoom identisch, transparente Stellen ergeben Weiß statt der Schachbrettfarbe.
  Übrige App-Flächen über ein Fensterfoto (in cargo-Testprogrammen abgeschaltet, weil egui_kittest
  jedes Foto mit eigenem wgpu-Renderer erzeugt und parallel abstürzt).
- **A4 Werkzeugleiste Verlauf/Muster:** statt in das Verlaufs-Popup zu wechseln, wird die Füllung der
  Auswahl auf Verlauf bzw. das erste Muster gesetzt; ohne Auswahl ausgegraut. Strichlage-Knöpfe der
  Werkzeugleiste ebenfalls nur mit Auswahl (Firepixel hat keine Strichlage im Werkzeug-Inspektor).
- **A5** Inspektor-Füllung: Fußleiste nur Farbkreis wie im Original („Ohne“ über die Füllart-Knöpfe
  daneben). Dialog „Farbe“ der Bildvorschau-Farbtabelle (Doppelklick) bleibt: er entspricht dort dem
  Systemfarbdialog des Originals.
- Annahmen (nicht gemessen): Strichlage-Knopf schließt das Popup; Deckkraft einer Rasterwahl = die der
  Ausgangsfarbe (bei „Ohne“ deckend); Klick ins Popup-Feld zwischen Popup und Farbdialog tut nichts.

## Neue Befunde
- `pi-color-pipette` und `pi-color-transparent` (inspector.ftl) werden nicht mehr benutzt (für I18N aufräumen).
- `ColorChoice::Pipette`/`popups::start_pick` werden vom Popup nicht mehr ausgelöst (Altpfad, kann entfallen).
- Bridge (`bridge.rs`): ein Bildschirmfoto ohne u64-Kennung wird dem ersten wartenden Foto zugeordnet; FARB-1b nutzt deshalb eine feste u64-Kennung – andere neue Fotonutzer sollten das ebenso tun.
