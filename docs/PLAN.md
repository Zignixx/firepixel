# Firepixel – Gesamtplan

Stand: **12.10.2026, ≈ 14:15** (Koordinator). Dieses Dokument ist der Fahrplan bis „fertig“. Es sagt, was erledigt ist, was gerade läuft und was in welcher Reihenfolge noch kommt. Die Einzelheiten stehen in den verlinkten Dateien. Wer ein Paket abschließt, hakt es hier ab (☐ → ☑) und trägt den Commit ein.

Verwandte Dateien:
- `STATUS.md`: Stand je Crate und je Agent
- `ARCHITEKTUR.md`: verbindlicher Bauplan
- `docs\AGENTEN_REGELN.md`: Regeln für alle Agenten
- `docs\BEFUNDE.md`: offene und erledigte Befunde je Bereich
- `docs\NUTZERBEFUNDE.md`: vom Nutzer gemeldete Fehler und Wünsche N01–N23
- `docs\PARITAET_A.md` / `PARITAET_B.md`: Abgleich mit dem Original, Knopf für Knopf
- `docs\FUNKTIONSTEST.md` und `docs\funktionstest\*_auto.md`: automatischer Gesamttest

---

## 1. Ziel und verbindliche Vorgaben des Nutzers

1. **Firepixel** ist ein privater, nativer Nachbau des klassischen Grafikprogramms (Original) in Rust: eine einzelne EXE, öffnet und speichert dessen PNG-Dateien.
2. **Bedienung 1:1 wie im Original.** Jede Funktion und jeder Knopf muss gleich funktionieren und reagieren. Nachgewiesen wird das im echten Fenster mit echter Maus und Tastatur, beide Programme nebeneinander.
3. **Komplett übersetzbar, vollständig Deutsch und Englisch.** Kein fest eingebauter Text, auch nicht in MCP- und CLI-Ausgaben, Tooltips oder der Statusleiste.
4. **Technisches immer Englisch:** Ordner, Dateien, Schlüssel, CLI, MCP, Umgebungsvariablen und `.ftl`-Namen (Regel §1c). Übersetzt wird nur Oberflächentext.
5. **Neutralität:** Weder in der Oberfläche noch im Quelltext, in der Doku oder in der EXE steht der Name des Originals oder seines Herstellers. Wächter ist `cargo test -p fp-neutral`.
6. **Clean Room:** Umgesetzt wird nur nach den Spezifikationen in `re\`. Kein dekompilierter Code, keine fremden Assets.
7. **Drei Designs:** Normal, Dunkel und OLED (schwarz, bewusst farbenfroh).
8. **Moderne Formate:** WebP, APNG, AVIF, JPEG XL, HEIC (Import), QOI, ICO, SVG.
9. **KI-Steuerung „so gut wie photocraft“:** ein Befehlsregister für UI, CLI, MCP und JSF.
10. **Arbeitsweise:**
    - autonom, ohne Rückfragen
    - **höchstens 8 Agenten gleichzeitig** (seit 12.10. ≈ 16:40; vorher 6 bzw. 4), alle auf Opus 5.5, kleine Aufträge; höchstens 1–2 davon bedienen die Oberfläche stark
    - nur einer bedient die Oberfläche (Sperre `re\scripts\fwsteuerung\gui-sitzung.ps1`)
    - RDP bleibt offen
11. **„Fertig“ gibt es erst nach dem kompletten Durchklicken der ganzen App im echten Fenster** (§5, Schritt Z).

---

## 2. Was erledigt ist

### Phasen 1–6 (11./12.10.)

| Phase | Inhalt | Stand |
|---|---|---|
| RE | 16 Spezifikationen + Python-Referenzen, Lückenschluss, Laufzeittests im Original | ☑ (`re\01…16-*.md`) |
| 1 | fp-geom, fp-color, fp-raster, fp-model, fp-fwpng | ☑ |
| 2 | fp-vector, fp-text, fp-effects, fp-render (648/651 Korpusseiten pixelgenau) | ☑ |
| 3 | fp-engine (Befehlsregister, Undo, Werkzeuge), fp-io, fp-export, fp-ui-Hülle, 3 Designs | ☑ |
| 4 | fp-script (JSF), fp-mcp, firepixel-cli, Inspektor, Bedienfelder, Dialoge, Leinwand | ☑ |
| 5 | Integration, App-Icon, Live-Brücke (`ui_input`/`ui_find`/`screenshot`), nicht blockierende Datei-Dialoge | ☑ |
| 5b | KI-Auswahl (fp-ai): Objektauswahl (SAM 2.1), Motiv auswählen und Kante verfeinern (BiRefNet) | ☑ |
| 6 | Mehrsprachigkeit fp-i18n (Fluent), 8 Agenten | ☑ |

Umfang heute: 19 Crates, ≈ 169.000 Zeilen Rust, 132 Testdateien, 16 Sprachdateien je Sprache, über 110 Commits.

### Phase 7 (ab 12.10. mittags)

| Schwerpunkt | Ergebnis | Stand |
|---|---|---|
| **Übersetzung** (UE-1…UE-5) | fest eingebaute Texte 389 → 0, Scanner für den ganzen Workspace scharf; MCP antwortet standardmäßig englisch, CLI mit `--lang`; englische Doku (`AGENTS.en.md`, `ANLEITUNG.en.md`); Hilfe öffnet die Anleitung in der UI-Sprache | ☑ (Rest: englischer Klickdurchgang in Z) |
| **Bild → SVG** (VT-1…VT-4) | Crate fp-trace (visioncortex, MIT/Apache), Befehl `modify.traceBitmap`, Dialog mit Live-Vorschau, 4 Vorgaben, Vorher/Nachher, „Aufräumen“ (Logo 1108 → 30 Knoten, 99,3 %), SVG speichern; erreichbar über Menü, Kontextmenü und Inspektor | ☑ (Rest: N20 größeres Fenster) |
| **Abgleich mit dem Original** | PARITAET_A: **106 ✅ / 16 ⚠️ / 4 ❌** von 126 Zeilen (vorher 31/18/27 bzw. 86/22/9); Warnungen Z01–Z18 mit „Nicht mehr anzeigen“, Schieberkurven, gemerkte Dialogwerte, Wiederholen im Menü, Fang, Frei transformieren, Zeiger, Inspektor-Felder (FV-1…FV-10, PM-1…PM-3, EB-1) | 🔄 siehe §4 |
| **Fehler des Nutzers** | N02–N10, N12, N15 ✅ im echten Fenster | 🔄 siehe §3 |
| **Testversion** | `Desktop\Firepixel-Vorschau-12-10\` (Stand `d71dba1`) an den Nutzer übergeben | ☑ |

---

## 3. Fehler und Wünsche des Nutzers (`NUTZERBEFUNDE.md`)

| Nr | Kurz | Stand | Paket |
|---|---|---|---|
| N01 | Filter-Größe mehrfach ändern stapelt den Effekt | 🔧 Nachprüfung im Fenster fehlt | P-PRUEF |
| N11 | Dialoge/Startbildschirm schöner | ❌ teilweise | P-DESIGN |
| N13 | Text nach Esc kurz unsichtbar | ⚠️ von FV-6 behoben, Nachprüfung im Fenster fehlt | P-PRUEF |
| N14 | Schriftgrößen-Schieber in 100er-Schritten | 🔧 Nachprüfung im Fenster fehlt | P-PRUEF |
| N16–N18 | Speichern-Dialog hängt / Neutralität / „Pick an app“ | 🔧 | P-PRUEF |
| N19 | Suche in der Schriftarten-Liste (Wunsch) | ❌ | P-N19-20 |
| N20 | Fenster „Bitmap nachzeichnen“ standardmäßig größer (≈ 1050 × 830) | ❌ | P-N19-20 |
| N21 | KI-Fortschrittsfenster schöner + Einstellungen zum Feintunen, sofern die Modelle sie hergeben | ❌ | P-KI |
| N22 | Datei ▸ Bildvorschau vollständig (1/2/4-fach-Vergleich …) | 🔄 Spezifikation läuft (RE-BV) | P-BV |
| N23 | abgeschnittene Texte, zu knappe Ränder → Prüfung der ganzen App | ❌ | P-RAND (vor Z) |

---

## 4. Läuft gerade (höchstens 8 bauende Agenten, dazu reine Analyse-Agenten ohne Build)

**Beschleuniger (Nutzerentscheidung 12.10. ≈ 17:00):**
1. Die Firepixel-Seite wird über die Brücke ohne GUI-Sperre parallel geprüft (AGENTEN_REGELN §4e). Echte Systemmaus nur fürs Original, für Eingaben, die vom Betriebssystem abhängen, und fürs Durchklicken am Ende.
2. Reine Spezifikations-Agenten laufen zusätzlich: Export-Dialoge, Kurven/Tonwert, AI8/SWF.
3. Gemeinsamer Build-Cache sccache (`tools\sccache\`, §2).

Abgelehnt bzw. nicht gewählt: zweite GUI-Spur über einen zweiten Windows-Benutzer.

| Agent | Auftrag | Schreibt in |
|---|---|---|
| **FV-14** | P-LW1: Leinwand-Rest (Fang an Objektmitten L4/L35, Alt-Eckpunkt L33, Esc nach Zauberstab L13, Zoomrahmen L25, Strg+Leertaste L7, Stift L34, Klick auf transparentes Bitmap-Pixel) | fp-ui canvas, fp-engine tools, `canvas_fv14.rs` |
| **PM-4** | P-PRUEF: Nachprüfung im echten Fenster (FV-10 W19…O16, FV-9 W10/W15/W17/W27/W28, FV-11 kurz, N01/N13/N14/N16–N18), Zähltabelle PARITAET_A nachführen | nur Doku + Bilder |
| **PM-6** | P-PM4 Teil 2: Messrunde Dialoge D26–D50 in beiden Programmen (Firepixel über die Brücke), Zahlenfeld-Fehler je Dialog prüfen, Paketvorschläge | nur Doku + Bilder |
| **INF-1** | schnellere Builds: alle Integrationstests je Crate in **ein** Testprogramm (Dateien bleiben, `build.rs` sammelt sie automatisch), rust-lld als Linker (zuerst committen, sobald geprüft) | Cargo-Manifeste, `build.rs`, `.cargo\config.toml`, AGENTEN_REGELN §2 |
| **DLG-1** | P-DLG1: **Zahlenfelder nehmen nach einigen Öffnungen keine Eingabe mehr an** (hoch, Leinwand-/Bildgröße, Raster, Textgröße), D19 live nachrechnen, D20 Auswahl/Anker/B:H:, D01-Rest, Tab-Folge in allen Dialogen, keine leeren Verlaufsschritte | `dialogs\*`, `dialogs_dlg1.rs` |
| **RE-EX** (Analyse, ohne Build) | Spezifikation `re\18-export-dialoge.md`: Exportieren (alle Arten + Optionen), Exportassistent, HTML-Einrichtung, mit Lückenliste und Paketen → danach Cleanroom-Agenten DLX-1 | nur `re\` |
| **RE-KT** (Analyse, ohne Build) | Spezifikation `re\19-kurven-tonwert.md`: Kurven, Tonwertkorrektur (Pipetten, Histogramm), Auto-Tonwert, Bitmap-Auswahl speichern/wiederherstellen → danach DLX-2/3 | nur `re\` |
| **RE-SWF** (Analyse, ohne Build) | Spezifikation `re\20-ai8-swf.md`: Speichern als AI8, SWF (inkl. Optionen), Reduziertes PNG, mit Python-Referenz → danach Cleanroom-Agent EB-2 | nur `re\` |
| **KI-2** | P-KI: N21 KI-Fortschrittsfenster schön (3 Designs, de/en, kein Aufblitzen) + Feineinstellungen, die die Modelle wirklich hergeben (Schwelle, Kante, ausdehnen/zusammenziehen, Maskenvariante) im Werkzeug-Inspektor | fp-ai, KI-Befehle, KI-Inspektor, `ai_progress_ki2.rs` |
| **FV-15** | P-IBD1: Texturliste 77 + „Andere…“, Feldtext beim Klick auf ein anderes Objekt (F7), `pi.text.antialias`, Hinweistext der Objektauswahl | Inspektor, `panel.rs`, `inspector_fv15.rs` |
| **RAND-1** | P-RAND Teil 1: automatischer Prüfer `clipping_audit.rs` für abgeschnittene Texte und knappe Ränder (alle Bedienfelder, Inspektor-Seiten, Dialoge; 3 Designs × de/en × 100/150 % × 2 Fenstergrößen), Bericht `rand_auto.md`, sichere Sofortkorrekturen inkl. Nutzerbeispiel „Zeilensprung/Progressiv“ (N23) | neuer Test, kleine Abstands-Korrekturen |

Fertig seit dem Plan: ☑ **FV-8** Tippverzögerung (`8f6bcfc`, `5a821e0`): Ursache ist der Software-Rasterer dieses Rechners (keine Grafikkarte, jedes Bild 150–700 ms). Gegenmittel: egui-wgpu-Kopie in `vendor\` nur für Software-Rasterer, Schachbrett als eine Textur, Teil-Upload der Leinwand, Ebenen-Miniaturen nur sichtbar. Zeit je Zeichen 640 → 77 ms (große Seite 1190 → 96 ms) · ☑ **TE-1** Technisches auf Englisch, Teil 1 (`c1e16e1`): 31 Namen, u. a. `Sprachen` → `Languages`, `Modelle` → `Models`, `Allgemeine Bibliothek` → `Common Library`, CLI `sprache` → `language`, `FP_DATEIDIALOG` → `FP_FILE_DIALOG`; Migration erst beim nächsten Start durch den Nutzer; Wächter `fp-neutral\tests\technical_english.rs`; Liste und Teil 2 in `docs\TECHNISCH_ENGLISCH.md` §D · ☑ **RE-BV** (`re\17-bildvorschau.md`) · ☑ **FV-11** Bitmap-Werkzeuge: Pinsel malt Pixel, Esc in allen Malwerkzeugen, Abdruckmodell für Radiergummi/Weichzeichner/Stempel, kein Rahmen beim Malen; L-Abschnitt 30/7/2 → 34/4/1 (`24e9524`, `37aaac0`, `8072c14`, `a6c6a61`). Neuer Rest LW: Zeiger-Klick auf ein transparentes Bitmap-Pixel wählt nichts (Original: Bitmap); Abdruckmodell für Scharfzeichner, Abwedler usw. ungemessen → in P-LW1.

---

## 4a. Pause ≈ 20:30 (Nutzerentscheidung 12.10.2026 ≈ 19:15)

Ab ≈ 19:15 startet der Koordinator **keine neuen Agenten** mehr. Die laufenden Agenten machen ihren Auftrag fertig und übernehmen nichts zusätzlich: DLG-1, DLG-1b, DLG-3, NZ-1, FV-16, INF-1, PM-6, PM-7, RE-EX und RE-KT.

Wenn der letzte fertig ist, beginnt die **Pause**. In der Pause:
1. **Aufräumen:** übrig gebliebene Worktrees, `target-*`-Ordner und Testinstanzen. Im Original lagen am Abend viele leere Testdokumente; bei Bedarf startet der Nutzer es neu.
2. **Stand prüfen:** `cargo check --workspace --all-targets`, fp-i18n, fp-neutral, i18n-Scanner.
3. **Testversion:** Release-Build aus einer sauberen Kopie von HEAD, auf den Desktop, Ordnername mit Datum und Uhrzeit, mit `NEU.md`.
4. **Doku nachziehen** (Nutzerwunsch, ein Doku-Agent ohne Build):
   - `ANLEITUNG.md`/`.en.md`: neue Funktionen seit 12.10. mittags, z. B. Schriftsuche, KI-Feineinstellungen und Fortschrittskarte, Bildvorschau 1/2/4-fach mit Optionen, Texturen „Andere…“, Pinsel auf Bitmap, Datei-Dialoge
   - `AGENTS.md`/`.en.md` und MCP-Ressourcen: neue Parameter
   - `STATUS.md` oben eine aktuelle Gesamtübersicht statt nur Agenten-Abschnitten
   - `PLAN.md` auf den Ist-Stand
   - `NUTZERBEFUNDE.md`, `NACHPRUEFUNG.md` aufgeräumt
   - `REA\UEBERGABE.md` mit dem Stand 12.10. abends
   - **neu `docs\TESTLEITFADEN_2026-10-12.md`** (Nutzerwunsch): **alles, was seit 12.10. ≈ 04:00 geändert wurde**, nach Bereichen geordnet. Quelle sind `git log` seit 04:00, NUTZERBEFUNDE, BEFUNDE und PARITAET. Je Änderung:
     - was neu bzw. anders ist
     - **wie man hinkommt**: Menüpfad, Werkzeug, Kürzel, Bedienfeld
     - **ein kurzer Handgriff zum Ausprobieren** mit dem erwarteten Ergebnis
     - der Vergleich mit dem Original, wo das sinnvoll ist
     - die Nr. aus NUTZERBEFUNDE (N…), falls es ein Bug des Nutzers war
     - ganz oben die Punkte, die der Nutzer selbst gemeldet hat
   - Eine Kopie davon kommt als `NEU.md` in den Ordner der Testversion.

Danach geht es weiter, wenn der Nutzer „weiter“ sagt, mit EB-2, DLX-2, DLX-1, BV-3, DLG-4 und dann Welle B.

## 4b. Gekürzt bzw. verkleinert (Nutzerentscheidung 12.10.2026 ≈ 18:45)

| Paket | Entscheidung |
|---|---|
| EB-2 (P-EB1) | **Nur „Reduziertes PNG“ bauen.** SWF und Illustrator 8 fallen weg: Flash ist tot, AI8 stammt von 1998, und SVG-Export gibt es schon. Die Spezifikation `re\20-ai8-swf.md` bleibt für später erhalten. |
| HTML-Teil | Für HTML-Einrichtung (D13), HTML-Code kopieren (D15), HTML aktualisieren (D09), Export „HTML und Bilder“ und Exportassistent (D11) gilt: **funktioniert, aber kein Feld-für-Feld-Abgleich** mit dem Original. |
| CI-1 (P-CI) | → **Später**, zusammen mit den Builds für macOS und Linux; sinnvoll erst, wenn es ein GitHub-Repository gibt. |
| AU-3 (P-AU1) | Nur F7 (Feldübernahme beim Objektwechsel) und die vertauschte JSF-Textrichtung. **Weg fallen:** `pixelRect` bei Text, `setActiveViewScale` in der Brücke, Hotspot-Typ „slice“ und JSF-Doppelaufruf. |
| PERF-1 (P-PERF) | Nur „Seiten mit Tausenden Objekten“ (Alles auswählen, Pfeiltasten). Der Rest kommt hauptsächlich vom Software-Rasterer dieses Servers. |
| TE-2 (P-TE2) | Englische Namen für Sprachdateien, Quelldateien, Anleitung und Umgebungsvariablen. **Testdateien und Testfunktionen behalten ihre Namen.** |
| VT-5 | **Gestrichen** (KI-Vektorisieren mit StarVector). |
| D12 Batch-Verarbeitung | Bleibt wie vom Nutzer zurückgestellt unter „Später“ (P-AU2) und gehört nicht zu DLX-5. |
| Nicht gekürzt | Seltene Dialoge D26–D50 bleiben **feldgenau** wie im Original. |

## 5. Warteschlange (in dieser Reihenfolge; jedes Paket = ein kleiner Agent)

Die Reihenfolge gilt, solange nichts Dringenderes vom Nutzer kommt. Sobald ein Platz frei wird, startet das oberste Paket, dessen Dateien nicht gerade ein anderer Agent bearbeitet.

### A. Abgleich mit dem Original: Befunde beheben

| Paket | Inhalt | Quelle |
|---|---|---|
| ☑ **P-LW1** | Leinwand-Rest von PM-3: Fang an Objektmitten beim Verschieben und bei Stiftpunkten (L4/L35), Alt-Ziehen aus Eckpunkt (L33), Bitmap nach Esc beim Zauberstab nicht mehr gewählt (L13), Zoomrahmen-Rand (L25), Strg+Leertaste+Klick (L7), Stift am Endpunkt (L34) | BEFUNDE LW ☑ FV-14 (`8b03953`): L4/L35, L7, L13, L25, L33 und der FV-11-Rest (durchsichtige Bitmap-Pixel) behoben; L34 und Restmessungen im Original in NACHPRUEFUNG |
| ☐ **P-LW2** | Verlauf-Werkzeug: Farben (im Original nachmessen) und Füllart-Knöpfe (W25/W26); Farbe ersetzen: Werksvorgaben (W8) + `prefs.json`-Typen; `fill_selection` gemeinsam für Eimer und Verlauf | BEFUNDE LW/IBD/EB |
| ☑ **P-IBD1** | Texturliste: Benutzer-Texturordner (77) + „Andere…“ (W21-Rest); Feldtext beim Klick auf ein anderes Objekt (F7); `pi.text.antialias` Zuordnung; Inspektor-Hinweis „Esc: neues Objekt“ bei der Objektauswahl – FV-15: W21, Musterliste, `pi.text.antialias`, Hinweis erledigt; F7 im Original nicht messbar (interner Fehler bei jedem Leinwandklick), Befund offen bis zum Neustart des Originals | BEFUNDE IBD/EB/HG |
| ☑ **P-PRUEF** (PM-4, `2797c0e`) | **Nachprüfung im echten Firepixel-Fenster:** FV-10 (W19, W21, W22, W24, W26, W29, O2, O8, O16; die Prüfung wurde während der Testpause abgelehnt), FV-9-Reste (W10/W15/W17/W27/W28), N01, N13, N14, N16–N18; danach die Zähltabelle in PARITAET_A nachführen | FV-10/FV-9-Berichte, NUTZERBEFUNDE |
| ☑ **P-MENU1** | 8 Menüeinträge, die im Gesamttest durchfallen: Bitmap-Auswahl wiederherstellen/löschen, Hilfslinien entfernen, Pfade kombinieren ▸ Teilen, Vorschau im zweiten Browser (in keiner Testsituation verfügbar), Befehlsskripte „Andere Ebenen ausblenden/sperren“, „Warndialoge zurücksetzen“ (keine erkennbare Wirkung) – erledigt (FV-13: menue_auto 345/345, kontextmenues_auto 283/283) | `docs\funktionstest\menue_auto.md` |
| ☐ **P-AU1** | JSF wie im Original: zwei Aufrufe → andere Objekte (O11/O6), `pixelRect` beim Text ohne 2-px-Rand (L22), `fw.setActiveViewScale` in der Brücke, `addNewHotspot` „slice“ | BEFUNDE AU |
| ☐ **P-PERF** | Restliche langsame Stellen von FV-8: erste Ziehbewegung großer Objekte bis 1 s, erstes Bild nach Zoom/Bildlauf bis 1,4 s, Alles auswählen auf Seiten mit Tausenden Objekten (Pfeiltasten 250–430 ms), Inspektorfelder messen; Ziel ≤ 50 ms auch auf dem Software-Rasterer | BEFUNDE LW/HG (FV-8) |
| ☐ **P-NEUTRAL** | fp-neutral prüft auch `STATUS.md`, `docs\*.md` und `docs\funktionstest\*` (am 12.10. stand ein JSF-Funktionsname mit dem Namen des Originals in STATUS/BEFUNDE, vom Koordinator entfernt `620f021`, der Test hatte es nicht gefunden); Testreste im echten Profil sind entfernt (`Testbefehl*.jsf`, `Allgemeine Bibliothek\Sym.png` → Scratchpad), seit EB-3 `777ce3d` verhindert ein Wächter neue | BEFUNDE EB-3 |
| ☐ **P-HG1** | Datei-Dialoge im zuletzt benutzten Ordner; Rückfrage nach Strg+W erst nach 1,3–2 s; WM_CLOSE von außen beendet die App nicht | BEFUNDE HG |

### B. Menüs, Dialoge, Bedienfeldmenüs (PARITAET_B)

| Paket | Inhalt |
|---|---|
| ☐ **P-PM4** | **Neue Messrunde PARITAET_B** in beiden Programmen: Hauptmenüs (311/5/5), alle 50 Dialoge Feld für Feld (Stand 04:30: 3/16/30; seitdem vieles behoben, aber nicht nachgemessen), Bedienfeldmenüs (1/3/4); neue Befunde in kleine Pakete aufteilen |
| ☑ **P-PM4 Teil 1** | PM-5 (`360b0c7`): D01–D25 nachgemessen → 0 ✅ / 10 ⚠️ / 14 ❌ / 1 ⏳; 13 neue Befunde (IBD) |
| ☑ **P-DLG1** | Teil 1 (DLG-1, `fc7df14`, `173bca1`): Zahlenfelder nehmen wieder Eingaben an (Ursache: Fensterfokus in `buffered_edit`), Tab-Folge aller Dialoge (Titelleiste, Startfokus, Optionsgruppen); D19/D20/D01/Verlauf → DLG-1b |
| ☐ **P-DLG2** | D18 Raster, D23 Textgröße (zeigt immer 12), D21, D22 (Felder ohne Haken nicht grau, Überlagerungsfarbe fehlt) |
| ☐ **P-DLG3** | Dateidialoge D02–D06, D09: mittig über dem Fenster statt oben links, Startordner (nicht immer „Dokumente“), Titel, Filter, Zusatzfelder bei Öffnen/Speichern unter (zusammen mit P-HG1) |
| ☐ **P-DLG4** | D14 Neues Symbol (OK öffnet die Symbolbearbeitung statt einer leeren Instanz bei 0,0), D25 gespeicherte Befehle (ohne eigene nur eine Meldung), D16, D17 |
| ☐ **P-DLG5…** | nach PM-6 (D26–D50) in Paketen zu 3–5 Dialogen; Originalbeschriftungen; Hinweis-Meldungen; Live-Filter-Menü „+“; Überlauf bei 1024 × 768 / 150 % |
| ☐ **P-DLG-X** | unvollständige große Dialoge: Exportieren, Exportassistent, HTML-Einrichtung (Spezifikation RE-EX), Kurven/Tonwertkorrektur (Pipetten, Histogramm), Auswahl speichern (Spezifikation RE-KT); laut PM-5 außerdem unvollständig: D07, D10 (XMP-Fenster fehlt), D11, D12, D13, D15, D24 (Bildvorschau siehe P-BV) |
| ☐ **P-MENU2** | Menüaufbau: Schrift-Menü (zuletzt benutzte oben, Familien flach), „Bridge durchsuchen“ aktiv/grau, „Mit darunter liegender reduzieren“ bei Text, Befehle-Menü im frischen Profil, Bedienfeldmenüs (≡) |
| ☐ **P-EB1** | Speichern unter: Dateitypen AI8, SWF, „Reduziertes PNG“ |

### C. Wünsche des Nutzers aus der Testversion

| Paket | Inhalt |
|---|---|
| ☑ **P-N19-20** (FV-12, e8dd1ad) | N19 Suche in der Schriftarten-Liste (Inspektor und Text-Menü, Tippen filtert, Pfeiltasten/Eingabe); N20 Nachzeichnen-Dialog standardmäßig ≈ 1050 × 830, Vorschau größer, „Aufräumen“ ohne Scrollen sichtbar, Größe gemerkt |
| ☑ **P-KI** (KI-2, `39c6e6a`) | N21: Fortschrittsfenster der KI-Werkzeuge schön gestalten (Design wie der Nachzeichnen-Dialog, alle 3 Designs); prüfen, welche Stellschrauben SAM 2.1/BiRefNet wirklich hergeben (Maskenschwelle, Kante weich/ausdehnen/zusammenziehen, Maskenvariante bei SAM), nur diese im Inspektor anbieten; optional: Vorschau der Objektmaske beim Überfahren |
| ☐ **P-BV1…4** | N22 Bildvorschau, **Cleanroom**-Agenten, die *nur* `re\17-bildvorschau.md` (§10) lesen. ☑ P1 Mehrfachansicht und Kern (erledigt, BV-1, Commits 9906252, 0c01836, 71577d5); ☑ P2 Registerkarte Optionen vollständig (Format/Palette/Rastern/Glättung/Transparenz mit 3 Pipetten, Farbtabelle mit Menü und Statuszeile, „Optimieren auf Größe“; erledigt, BV-2, Commit 9fc13e6, Nachprüfung im echten Fenster in NACHPRUEFUNG); P3 Datei (Skalieren, Exportbereich mit Werkzeug) und Animation; P4 Verhalten (OK schreibt die Einstellung des aktiven Feldes, Exportieren, Esc schließt), Vorgaben, Texte. Nebenbefund: PARITAET_B D08 „Esc schließt nicht“ ist falsch, im Original schließt Esc |
| ☐ **P-TE2** | Technisches Englisch, Teil 2: `.ftl`-Dateinamen (`befehlstexte.ftl`, `nachzeichnen.ftl` …) und deutsche Datei- und Testnamen im Quelltext umbenennen, nach der Liste von TE-1. Erst wenn wenige Agenten in den `.ftl`-Dateien arbeiten |

### D. Gestaltung und Feinschliff

| Paket | Inhalt |
|---|---|
| ☐ **P-DESIGN** | N11: Dialoge und Startbildschirm schöner (DESIGN.md §13), Vorher/Nachher-Bilder in 3 Designs |
| ☐ **P-RAND** | N23: **Prüfung der ganzen App auf abgeschnittene Texte und zu knappe Ränder.** Alle Bedienfelder, Dialoge, Inspektor-Seiten, Menüs, Tooltips; 3 Designs × de/en × 100/150 % × kleine/große Fenster. Beispiel: „Zeilensprung/Progressiv“ unter „Glättung“ halb abgeschnitten. Automatischer Test, der Textüberlauf in kittest erkennt, und Korrekturen. ☑ **Teil RAND-1** (`5f2d009`): Prüfer `clipping_audit.rs` + Bericht `rand_auto.md`, Nutzerbeispiel behoben; 70 Stellen in der Erlaubnisliste. ☐ **Teil RAND-2**: die Pakete R2-1…R2-5 aus `STATUS.md` (RAND-1) abbauen, offen sind noch Menüs und Tooltips (prüft der Prüfer noch nicht) |

### E. Später, niedrige Priorität

| Paket | Inhalt |
|---|---|
| ☐ **P-CI** | GitHub Actions wie bei photocraft (`ci.yml`: fmt, clippy, Tests, i18n-Scanner, fp-neutral; `release.yml`: Artefakte je Betriebssystem) |
| ☐ **P-OS** | Builds für Windows, macOS und Linux: Windows-eigene Teile hinter `cfg` (Startfenster, Datei-Dialoge, Shell-Öffnen, crt-static, Schriften), Prüfbuild für Linux/macOS. Ein GitHub-Repository bzw. Push nur nach Freigabe durch den Nutzer |
| ☐ **P-AU2** | Batch-Assistent wie im Original (vom Nutzer zurückgestellt) |
| ☐ **P-VT5** | optional: KI-Vektorisieren (StarVector-1B) als Download, nur nach Vergleich auf echten Logos des Nutzers |

### Z. Abschluss (erst wenn A–D erledigt sind)

1. ☐ **Gesamttestlauf:** `cargo test --workspace` + alle `funktionstest\*_auto.md` grün, i18n-Scanner 0, fp-neutral grün, technisches Englisch grün.
2. ☐ **Release-Build** als einzelne EXE + CLI, Testversion an den Nutzer.
3. ☐ **Komplettes Durchklicken im echten Fenster:**
   - jedes Menü, jedes Werkzeug (auf der Leinwand zeichnen), jedes Bedienfeld samt Menü, jedes Inspektor-Feld, jeder Dialog (Werte ändern, OK, Abbrechen), Kontextmenüs
   - Öffnen, Speichern und Exportieren mit echten Dateien
   - alle 3 Designs, Deutsch **und Englisch**, Fenstergrößen klein/groß/maximiert, Bedienfelder an- und abdocken
   - gezielt Logikfehler suchen: Werte mehrfach ändern, Abbrechen, Undo/Redo-Folgen, Duplikate
   - Ergebnis in `FUNKTIONSTEST.md`, jedes ❌ beheben und erneut prüfen
4. ☐ erst dann „fertig“ an den Nutzer melden, mit ehrlicher Liste dessen, was nicht 1:1 geht.

---

## 6. Offene Fragen an den Nutzer

- Keine blockierenden. Annahme: Code-Kommentare und deutsche Doku-Dateien bleiben deutsch, nur Technisches wird englisch (§1c).
- Für P-CI/P-OS braucht es später die Entscheidung, ob und wo ein GitHub-Repository angelegt wird.

---

## 7. Zahlen auf einen Blick (12.10., 14:15)

| Messgröße | Wert |
|---|---|
| PARITAET_A (Werkzeuge, Inspektor, Leinwand, Zeiger) | 106 ✅ / 16 ⚠️ / 4 ❌ (126 Zeilen) |
| PARITAET_B (Stand 04:30, Nachmessung P-PM4) | Menüs 311/5/5 · Dialoge 3/16/30 · Bedienfeldmenüs 1/3/4 |
| Automatischer Gesamttest | Oberfläche 361/361 · Kontextmenüs 282/283 · Bedienfeldmenüs 181/181 · Menüs 339/347 · Größen 168/168 · Logik 70/70 · Werkzeuge 52/52 |
| BEFUNDE.md | 238 erledigt · ≈ 50 offen (davon ≈ 10 nur Info) |
| Nutzerbefunde | 11 ✅ · 1 ⚠️ · 5 🔧 · 6 ❌ (N11, N19–N23) |
| Fest eingebaute Texte | 0 |
