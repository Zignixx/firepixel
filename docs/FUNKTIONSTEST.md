# Firepixel – Funktionstest (Gesamtliste)

Gepflegt von HG. Jede Funktion des Programms steht hier mit Status und Nachweis. Die Einzelzeilen
erzeugen die automatischen Gesamttests als Berichte unter `docs\funktionstest\*_auto.md`; diese
Datei fasst sie zusammen, nennt offene Punkte mit dem zuständigen Agenten (Befund in
`docs\BEFUNDE.md`) und die Prüfungen, die nur in einer echten Sitzung gehen.

Legende: ✅ funktioniert (der Test prüft die Wirkung) · ❌ fehlt/fehlerhaft (Befund eingetragen) ·
🔶 nur von Hand prüfbar (Fremdprogramme oder Geräte; im Test abgeschaltet).

## Tests ausführen

```
$env:FP_NO_SHELL_OPEN = '1'          # nie Fremdprogramme starten (AGENTEN_REGELN §4a)
cargo test -j 4 -p fp-ui --test gesamt_menu --test gesamt_werkzeuge --test gesamt_logik `
  --test gesamt_oberflaeche --test gesamt_kuerzel --test gesamt_dateien --test gesamt_groessen --test layout
cargo test -j 4 -p fp-ui --test layout bildschirmfotos -- --ignored        # Bildschirmfotos (docs\screenshots)
cargo test -j 4 -p fp-ui --test gesamt_groessen groessen_bilder -- --ignored  # Bilder je Größe/DPI/Design
```

**Testdokument** (`crates\fp-ui\tests\gesamt_common\mod.rs`), 800 × 600, enthält:

- Rechteck, Ellipse, zwei Linien, Text, Bitmap,
- Gruppe, Maskengruppe,
- Grafik-, Schaltflächen- und Animationssymbol, zusätzlich eine zweite Instanz,
- Segment, Hotspot, Stil,
- zwei Seiten und zwei Status,
- eine gefüllte Zwischenablage,
- eine gespeicherte FW-PNG-Datei mit Symbol sowie Skript-, HTML- und Programmdatei für die
  Dateidialoge.

**Testsituationen:** Jeder Eintrag wird der Reihe nach in allen passenden Situationen probiert,
bis er eine Wirkung zeigt:

- verschiedene Auswahlarten, Pixelauswahl,
- gespeichert und danach geändert,
- nach Rückgängig, nach einem Filter,
- Text auf Pfad, Segment mit Pop-up-Menü, JPEG-Maske,
- mehrere Ebenen, freigegebene Ebene, Masterseite,
- importiertes Symbol, zweites Dokument.

Ein Dokumentschritt gilt nur als ✅, wenn Rückgängig das Dokument exakt wiederherstellt.

## Übersicht (Stand 11.10.2026, nach dem Audit der Grundbedienung)

**Ehrlicher Hinweis zur Art der Prüfung:** Die meisten Bereiche wurden **synthetisch** geprüft. Die
Tests rufen dort die Aktion bzw. den Befehl direkt auf (Menüeintrag → `Action`, Werkzeug →
`Session::tool_event`) und gehen nicht den Weg über echte Maus- und Tastaturereignisse. So prüfen
sie die Wirkung, aber nicht die Bedienung. Deshalb blieben Grundfehler unentdeckt, die der Nutzer
sofort fand:

- Strg+C/V/X ging nicht (egui-winit liefert dafür `Event::Copy/Cut/Paste` statt Tastenereignissen),
- Strg+Mausrad sprang auf 6 % bzw. 6400 %,
- Dialoge ließen sich nicht verschieben,
- die Vorschau beim Verschieben malte über den Leinwandrand.

Seit dem Audit läuft die Grundbedienung zusätzlich mit **realistischen Eingaben**
(`gesamt_alltag.rs`); die Art steht in der zweiten Spalte.

| Bereich | Art | ✅ / gesamt | Nachweis |
|---|---|---|---|
| **Grundbedienung mit realistischen Eingaben** (Bildzeit 1/60 s): Klick, Umschalt+Klick, Copy/Cut/Paste wie egui-winit, fremder Text einfügen (Textobjekt 12 pt mittig im sichtbaren Teil), Duplizieren, Strg+D, Pfeiltasten mit Wiederholung, Ziehen über 20 Bilder, Gruppieren, Text tippen, Rückgängig/Wiederholen, Strg+Mausrad Rastung/Touchpad, Mausrad-Bildlauf, Dialog verschieben + Eingabe, Speichern, Doppelklick auf Text, Drag & Drop, Menüs per Tastatur (Alt+Buchstabe, Alt allein, F10, Pfeile, Eingabe, Esc), Strg+Tab/Strg+F6, viele Dokumentreiter mit „»“, Reiter-Kontextmenü, Fenstertitel, Speichern/Wiederherstellen grau + Rückfrage, Ebenen vorn, Doppelklick Zoom/Hand im Werkzeugbedienfeld | **realistisch** | 28 / 28 ✅ | `funktionstest\alltag_auto.md` (Mausrad-Testpunkt jetzt Leinwandmitte) |
| Hauptmenü: 10 Menüs, jeder Eintrag einschließlich der Laufzeit-Untermenüs (mit KI-Einträgen) | synthetisch (Aktion je Eintrag) | 306 / 306 | `funktionstest\menue_auto.md` |
| Hauptmenü in Sondersituationen: leere Auswahl, alles, gesperrt, ausgeblendet, Gruppe, Symbol, Seite 2, Status 2 | synthetisch | 8 / 8 | `funktionstest\menue_situationen_auto.md` |
| Bedienfeldmenüs (23 Bedienfelder) | synthetisch | 181 / 181 | `funktionstest\bedienfeldmenues_auto.md` |
| Kontextmenüs der Leinwand (8 Menüs) | synthetisch | 282 / 282 | `funktionstest\kontextmenues_auto.md` |
| Werkzeuge (52 mit KI-Auswahl), Mausbedienung | synthetisch (`tool_event`, ohne Leinwand-Eingabeweg) | 52 / 52 ✅ | `funktionstest\werkzeuge_auto.md` (Skalieren/Neigen/Verzerren/9-Slice übernehmen wie im Original erst mit Eingabe) |
| Oberfläche in 3 Designs: 23 Bedienfelder, Inspektor je Objektart (14) und je Werkzeug (49), Ansichten, alle Menüdialoge | offscreen gezeichnet (keine Bedienung) | 342 / 342 | `funktionstest\oberflaeche_auto.md` |
| Logik der Dialoge: 50× ändern, Vorschau, Abbrechen, OK, Rückgängig/Wiederholen, Auswahlwechsel | synthetisch (Felder direkt gesetzt) | 72 / 72 | `funktionstest\logik_auto.md` |
| Logik der Setzbefehle aus Inspektor und Bedienfeldern: 50× ändern, Rückgängig/Wiederholen | synthetisch (Befehle) | 32 / 32 | `funktionstest\logik_setzbefehle_auto.md` |
| Tastenkürzel: 5 Sätze, Ziffernblock, Durchstich über die Tastatur | Satzprüfung synthetisch, Durchstich mit Tastenereignissen | 16 / 16 | `funktionstest\kuerzel_auto.md` |
| Dateien: FW-PNG-Rundreise, 16 Exportformate mit Wiederöffnen, Import, Testdateien, Drag & Drop, zuletzt geöffnet | Befehle bzw. abgelegte Datei | 24 / 24 | `funktionstest\dateien_auto.md` |
| UI über Größen, DPI, Designs und Zustände (§4c) | offscreen gezeichnet, Widget-Rechtecke | 168 / 168 | `funktionstest\groessen_auto.md` |
| Echtes Fenster (Release-exe, wgpu, winit-Schleife), gesteuert über die MCP-Brücke: Öffnen, Dialog, 3 Designs | echt (ohne Maus/Tastatur) | 4 Bilder | `docs\screenshots\phase5_hg_echt_{normal,dialog,dunkel,oled}.png` |

**Nicht automatisiert prüfbar bzw. offen:**

- Bildschirm und Eingabe der RDP-Sitzung sind nicht abgreifbar (Bildschirmaufnahme: „Handle ungültig“). Damit gehen weder echte Maus- und Tastatureingaben noch eine Fernprüfung des Originals für Drag & Drop. Befund an AU: Ereignis-Einspeisung über die Brücke.
- Drag & Drop von Text und Bildern aus anderen Programmen ist nicht möglich: winit 0.30 liefert nur Dateien und meldet beim Ablegen keine Position. Firepixel nimmt deshalb die erste Mausbewegung danach als Ablagestelle.
- Ziehen innerhalb von Firepixel ist noch offen (Befunde LW/IBD):
  - Objekt auf einen anderen Dokumentreiter,
  - Farbfeld auf ein Objekt,
  - Symbol aus der Bibliothek auf die Leinwand.

Einträge, die Fremdprogramme starten (Vorschau im Browser, E-Mail, Bridge, Hilfe, Online-Angebote),
gelten als ✅, wenn der Befehl das Ziel liefert, das gestartet würde: Datei, Adresse oder
Anleitung. Den Start selbst unterdrückt der Test. Die Sichtprüfung steht unter 12.

## 1. Programm, Fenster, Designs

| Funktion | Status | Nachweis |
|---|---|---|
| Eigenes App-Icon: Flamme aus Pixeln, 16–256 px, eigene pixelgenaue Stufen für 16/20/24/32 px | ✅ | `assets\icon\`, `docs\screenshots\phase5_icon_vorschau.png` |
| Icon und Versionsangaben als exe-Ressource (Explorer) | ✅ | `apps\firepixel\build.rs`; Release-exe: Icon und „Firepixel 0.1.0“ aus der Ressource gelesen; `phase5_hg_icon_exe.png` |
| Fenster- und Taskleisten-Icon (`with_icon`) | ✅ eingebaut, 🔶 Sichtprüfung | Bildschirm der RDP-Sitzung nicht abgreifbar, siehe 12 |
| Logo in Anwendungsleiste und Über-Dialog | ✅ | `phase5_hg_normal.png` |
| Fenster ohne Systemrahmen: eigene Fensterknöpfe, Ziehen, Doppelklick, Ränder zum Ändern der Größe | ✅ | `layout.rs`, Bildschirmfotos |
| Designs Normal, Dunkel und OLED (Ansicht ▸ Design, Voreinstellungen) | ✅ | `layout.rs::theme_switch_at_runtime`, `phase5_hg_normal/dunkel/oled.png` |
| Start mit Dateien, `--design=` (gilt nur für die Sitzung), `--mcp[=PORT]` | ✅ | `apps\firepixel\src\main.rs`, `lib.rs` (`cli_design`) |
| Eine einzige exe, nur System-DLLs, Start aus leerem Ordner | ✅ | `dumpbin /dependents`: nur Windows-System-DLLs; Start aus leerem Temp-Ordner mit Korpusdatei, Fenstertitel „Firepixel - [Sample.png @ 100%]“, sauberes Schließen; CLI ebenso |
| Keine Erwähnung von Originalhersteller oder Originalname in Oberfläche und exe | NT | Prüfung durch NT: `cargo test -p fp-neutral` |

## 2. Hauptmenü

Alle 10 Menüs stehen in Originalreihenfolge (`crates\fp-ui\data\menu_main.json`). Zustand, Häkchen
und Kürzel kommen aus dem Befehlsregister. Ist das Fenster zu schmal, wandern die übrigen Menüs in
„»“; es verschwindet nichts.

| Funktion | Status | Nachweis |
|---|---|---|
| Datei ▸ Zuletzt geöffnete Dateien | ✅ | `menue_auto.md`, `dateien_auto.md` |
| Text ▸ Schrift: Systemschriften, Untermenüs A–Z | ✅ | `menue_auto.md` |
| Befehle: Befehlsordner (Unterordner werden Untermenüs), Skript ausführen, Warnmeldungen zurücksetzen, KI-Steuerkanal | ✅ | `menue_auto.md` |
| Filter mit allen Untermenüs, einschließlich Farbfüllung | ✅ | `menue_auto.md` |
| Ansicht ▸ Design; Fenster ▸ Arbeitsbereichlayouts, Bedienfelder, Dokumentliste | ✅ | `menue_auto.md` |
| Datei: Importieren, HTML aktualisieren, Skript ausführen, Browser festlegen, Alles speichern | ✅ | `menue_auto.md` |

## 3. Bedienfelder, Bedienfeldmenüs, Inspektor

| Funktion | Status | Nachweis |
|---|---|---|
| 23 Bedienfelder zeichnen fehlerfrei, 3 Designs | ✅ | `oberflaeche_auto.md` |
| Bedienfeldmenüs (≡) aller Bedienfelder, auch Hilfe | ✅ | `bedienfeldmenues_auto.md` |
| Inspektor je Objektart: Dokument, Rechteck, Ellipse, Pfad, Text, Bitmap, Gruppe, Maske, 3 Symbolarten, Segment, Hotspot, Mehrfachauswahl | ✅ | `oberflaeche_auto.md` |
| Inspektor je Werkzeug (49) | ✅ | `oberflaeche_auto.md` |
| Setzbefehle 50× ohne Duplikate | ✅ | `logik_setzbefehle_auto.md` |

## 4. Werkzeuge und Werkzeugbedienfeld

| Funktion | Status | Nachweis |
|---|---|---|
| 49 Werkzeuge: Icon, Wahl über Bedienfeld oder Kürzel, Mausbedienung mit Wirkung und Rückgängig | ✅ | `werkzeuge_auto.md` |
| Flyouts, Platzgedächtnis, automatische Formen im Rechteck-Flyout | ✅ | `tools_panel.rs`, `oberflaeche_auto.md` |
| Farbfelder, Standardfarben (D), Tauschen (X), Keine | ✅ | `kuerzel_auto.md` |
| Bildschirmmodi (Knöpfe und F) | ✅ | `oberflaeche_auto.md`, `kuerzel_auto.md` |
| Bildlauf mit dem Mausrad bei niedrigem Fenster, Pfeile zeigen weiteren Inhalt | ✅ | `groessen_auto.md` |

## 5. Docking und Arbeitsbereiche

| Funktion | Status | Nachweis |
|---|---|---|
| Dock, Gruppen, Reiter, einklappen, Höhe und Breite ziehen, lösen, schwebend, andocken | ✅ | `workspace::tests`, `layout.rs` |
| Mindestgrößen: Dock höchstens so breit, dass eine Leinwand bleibt; Gruppen mindestens 120 px hoch (bei Platzmangel klappen untere Gruppen nur in der Anzeige ein); schwebende Gruppen mindestens 220 × 160 und immer greifbar im Fenster | ✅ | `groessen_auto.md` |
| Symbolmodus, Symbolmodus mit Namen, Bedienfelder ausblenden (Tab) | ✅ | `oberflaeche_auto.md` |
| Arbeitsbereich-XML des Originals lesen (alle mitgelieferten Layouts) und schreiben | ✅ | `workspace::tests::reads_original_layouts`, `panel_state_xml_roundtrip` |
| Benannte Layouts speichern und laden (`%APPDATA%\Firepixel\Workspace Layouts\*.xml`) | ✅ | `menue_auto.md` |

## 6. Ansicht

| Funktion | Status | Nachweis |
|---|---|---|
| Zoomstufen, Einpassen, Lineale, Raster, Hilfslinien | ✅ | `menue_auto.md` |
| Vorschau, 2-fach und 4-fach mit Exportbild; je Kachel Format, Größe und Ladezeit | ✅ | `src\preview.rs`, `phase5_hg_vierfach.png` |
| Leinwandtextur nie größer als der Grafikadapter erlaubt (hohe Vergrößerung auf großem Fenster) | ✅ | `canvas\mod.rs`; vorher Absturz beim Durchstich Strg+Num 8 |
| Seiten- und Statuswechsel als Befehl | ✅ | `actions.rs` |

## 7. Dateien, Zwischenablage, Drag & Drop

| Funktion | Status | Nachweis |
|---|---|---|
| Neu, Öffnen, Speichern, Speichern unter, Alles speichern, Als Vorlage, Wiederherstellen, Schließen mit Rückfrage | ✅ | `menue_auto.md` |
| FW-PNG-Rundreise, Import, 16 Exportformate mit Wiederöffnen, Öffnen von GIF/JPEG/HEIC/PNG | ✅ | `dateien_auto.md` |
| Drag & Drop einer Datei: über der Leinwand als Objekt an der Ablagestelle (linke obere Ecke, aktive Ebene, ausgewählt, ein Verlaufsschritt; mehrere Dateien zusammen ein Schritt), außerhalb der Leinwand bzw. FW-PNG als Dokument öffnen | ✅ realistisch | `alltag_auto.md`, `dateien_auto.md`; das Verhalten des Originals (Mitte oder Ecke, mehrere Dateien) ließ sich nicht fernprüfen, weil Bildschirm und Eingabe der Sitzung fehlen. Umgesetzt wie Datei ▸ Importieren (Ecke an der Klickstelle). |
| Bilder in die Systemzwischenablage und fremde Bilder einfügen | ✅ eingebaut, 🔶 Austausch mit anderen Programmen | `src\os_clipboard.rs` (Tests berühren die Systemzwischenablage nicht) |

## 8. Dialoge

| Funktion | Status | Nachweis |
|---|---|---|
| Jeder Menüdialog öffnet und zeichnet in 3 Designs; Esc schließt | ✅ | `oberflaeche_auto.md` |
| Formular-Dialoge und Live-Effekte (hinzufügen und bearbeiten) bestehen alle Logikfälle | ✅ | `logik_auto.md` |
| Dialoge im kleinsten Fenster (800 × 600 bei 200 %) bleiben im Fenster | ✅ | `groessen_auto.md` (Zustand „Dialog“) |

## 9. Tastenkürzel

| Funktion | Status | Nachweis |
|---|---|---|
| Menükürzel aller 5 Sätze: erreichbar und eindeutig, Ziffernblock-Rückfall | ✅ | `kuerzel_auto.md` |
| Kürzel des Werkzeugbedienfelds (Werkzeuge, D, X, F, 2) | ✅ | `kuerzel_auto.md` |

## 10. Automatisierung

| Funktion | Status | Nachweis |
|---|---|---|
| CLI: `commands`, `info`, `run`, `batch`, `render`, `convert`, `script`, `mcp` | ✅ | `cargo test -p firepixel-cli`; Release-CLI aus leerem Ordner (`info --json`) |
| MCP über stdio und HTTP; Brücke in der App-Schleife, `--mcp`, Befehle ▸ KI-Steuerkanal | ✅ | `cargo test -p fp-mcp` (`tests\bridge.rs`) |
| JSF-Skripte (Befehlsordner, Skript ausführen) | ✅ | `menue_auto.md`, fp-script-Tests (AU) |

## 11. Logik- und Größen-Sonderfälle (§4b, §4c)

| Fall | Status | Nachweis |
|---|---|---|
| Wert 50× ändern ohne Duplikate | ✅ | `logik_auto.md`, `logik_setzbefehle_auto.md` |
| Vorschau und Abbrechen ergeben exakt den Ausgangszustand | ✅ | `logik_auto.md` |
| OK erzeugt genau einen Verlaufsschritt; Rückgängig und Wiederholen sind exakt | ✅ | `logik_auto.md` |
| Auswahlwechsel mitten in der Bearbeitung | ✅ | `logik_auto.md` |
| Leere Auswahl, Mehrfachauswahl, Gruppen, Symbole, Seite 2, Status 2 | ✅ | `menue_situationen_auto.md` |
| Gesperrte Objekte bleiben unverändert | ✅ | `menue_situationen_auto.md` |
| Fenstergrößen 800 × 600 bis 3840 × 2160, schmal/hoch, breit/flach; DPI 100/150/200 %; alle Designs | ✅ | `groessen_auto.md`: kein bedienbares Element über dem Fensterrand, keine teilweisen Überlappungen, kein Springen zwischen zwei Bildern |
| Zustände: Dock schmal/breit, eingeklappt, Symbolmodus, schwebend sehr klein/groß, Inspektor aus, Dialog offen, 40 Ebenen/25 Seiten/30 Status mit langen Namen, leeres Programm | ✅ | `groessen_auto.md` |
| Bilder je Größe, DPI und Design zur Sichtprüfung | ✅ | `target\fp-ui-groessen\*.png` (`groessen_bilder`) |

## 12. Handprüfungen (echte Sitzung)

| Prüfung | Stand |
|---|---|
| Icon in der Taskleiste | 🔶 Die Ressource der exe ist geprüft. Ein Bildschirmfoto ist in der aktuellen RDP-Sitzung nicht möglich, weil die Aufnahme „Das Handle ist ungültig“ meldet. Nachholen, sobald der Bildschirm abgreifbar ist. |
| Start der Fremdprogramme (Browser, E-Mail, Hilfe-Webseiten) | 🔶 Nur mit Zustimmung des Nutzers prüfen, weil dabei Fenster bei ihm aufgehen. |
| Systemzwischenablage mit anderen Programmen | 🔶 |
