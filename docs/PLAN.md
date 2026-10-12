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
    - **höchstens 4 Agenten gleichzeitig, alle auf Opus 5.5, kleine Aufträge**
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

## 4. Läuft gerade (höchstens 4)

| Agent | Auftrag | Schreibt in |
|---|---|---|
| **FV-8** | Tippen im echten Fenster 0,3–0,5 s verzögert: Ursache messen, beheben (Ziel ≤ 16–33 ms), andere Eingaben prüfen | fp-ui (canvas, lib.rs, Tests `tippen_messung`) |
| **FV-11** | Bitmap-Werkzeuge: Pinsel malt Bitmap statt Vektorpfad (L36, hoch), Radiergummi, Weichzeichner, Stempel inkl. Esc (L37–L39) | fp-engine/fp-ui Werkzeuge, `bitmapwerkzeuge_fv11.rs` |
| **RE-BV** | Spezifikation `re\17-bildvorschau.md` aus Code und laufendem Original, mit Lückenliste und Umsetzungspaketen | nur `re\` |
| **TE-1** | Technisches auf Englisch: Bestandsaufnahme `docs\TECHNISCH_ENGLISCH.md`, Laufzeitpfade umbenennen (`Sprachen` → `Languages`, `Modelle` → `Models` …) mit Migration, Wächter-Test | fp-i18n, fp-ai, fp-neutral, CLI/MCP |

---

## 5. Warteschlange (in dieser Reihenfolge; jedes Paket = ein kleiner Agent)

Die Reihenfolge gilt, solange nichts Dringenderes vom Nutzer kommt. Sobald ein Platz frei wird, startet das oberste Paket, dessen Dateien nicht gerade ein anderer Agent bearbeitet.

### A. Abgleich mit dem Original: Befunde beheben

| Paket | Inhalt | Quelle |
|---|---|---|
| ☐ **P-LW1** | Leinwand-Rest von PM-3: Fang an Objektmitten beim Verschieben und bei Stiftpunkten (L4/L35), Alt-Ziehen aus Eckpunkt (L33), Bitmap nach Esc beim Zauberstab nicht mehr gewählt (L13), Zoomrahmen-Rand (L25), Strg+Leertaste+Klick (L7), Stift am Endpunkt (L34) | BEFUNDE LW |
| ☐ **P-LW2** | Verlauf-Werkzeug: Farben (im Original nachmessen) und Füllart-Knöpfe (W25/W26); Farbe ersetzen: Werksvorgaben (W8) + `prefs.json`-Typen; `fill_selection` gemeinsam für Eimer und Verlauf | BEFUNDE LW/IBD/EB |
| ☐ **P-IBD1** | Texturliste: Benutzer-Texturordner (77) + „Andere…“ (W21-Rest); Feldtext beim Klick auf ein anderes Objekt (F7); `pi.text.antialias` Zuordnung; Inspektor-Hinweis „Esc: neues Objekt“ bei der Objektauswahl | BEFUNDE IBD/EB/HG |
| ☐ **P-PRUEF** | **Nachprüfung im echten Firepixel-Fenster:** FV-10 (W19, W21, W22, W24, W26, W29, O2, O8, O16; die Prüfung wurde während der Testpause abgelehnt), FV-9-Reste (W10/W15/W17/W27/W28), N01, N13, N14, N16–N18; danach die Zähltabelle in PARITAET_A nachführen | FV-10/FV-9-Berichte, NUTZERBEFUNDE |
| ☐ **P-MENU1** | 8 Menüeinträge, die im Gesamttest durchfallen: Bitmap-Auswahl wiederherstellen/löschen, Hilfslinien entfernen, Pfade kombinieren ▸ Teilen, Vorschau im zweiten Browser (in keiner Testsituation verfügbar), Befehlsskripte „Andere Ebenen ausblenden/sperren“, „Warndialoge zurücksetzen“ (keine erkennbare Wirkung) | `docs\funktionstest\menue_auto.md` |
| ☐ **P-AU1** | JSF wie im Original: zwei Aufrufe → andere Objekte (O11/O6), `pixelRect` beim Text ohne 2-px-Rand (L22), `fw.setActiveViewScale` in der Brücke, `addNewHotspot` „slice“ | BEFUNDE AU |
| ☐ **P-HG1** | Datei-Dialoge im zuletzt benutzten Ordner; Rückfrage nach Strg+W erst nach 1,3–2 s; WM_CLOSE von außen beendet die App nicht | BEFUNDE HG |

### B. Menüs, Dialoge, Bedienfeldmenüs (PARITAET_B)

| Paket | Inhalt |
|---|---|
| ☐ **P-PM4** | **Neue Messrunde PARITAET_B** in beiden Programmen: Hauptmenüs (311/5/5), alle 50 Dialoge Feld für Feld (Stand 04:30: 3/16/30; seitdem vieles behoben, aber nicht nachgemessen), Bedienfeldmenüs (1/3/4); neue Befunde in kleine Pakete aufteilen |
| ☐ **P-DLG1…n** | Dialoge nach der Messung in Paketen zu 3–5 Dialogen, z. B. D18/D20/D22/D23; Originalbeschriftungen; Hinweis-Meldungen; Live-Filter-Menü „+“; Überlauf bei 1024 × 768 / 150 % |
| ☐ **P-DLG-X** | unvollständige große Dialoge: Exportieren, Exportassistent, HTML-Einrichtung, Kurven/Tonwertkorrektur (Pipetten, Histogramm), Auswahl speichern (Bildvorschau siehe P-BV) |
| ☐ **P-MENU2** | Menüaufbau: Schrift-Menü (zuletzt benutzte oben, Familien flach), „Bridge durchsuchen“ aktiv/grau, „Mit darunter liegender reduzieren“ bei Text, Befehle-Menü im frischen Profil, Bedienfeldmenüs (≡) |
| ☐ **P-EB1** | Speichern unter: Dateitypen AI8, SWF, „Reduziertes PNG“ |

### C. Wünsche des Nutzers aus der Testversion

| Paket | Inhalt |
|---|---|
| ☐ **P-N19-20** | N19 Suche in der Schriftarten-Liste (Inspektor und Text-Menü, Tippen filtert, Pfeiltasten/Eingabe); N20 Nachzeichnen-Dialog standardmäßig ≈ 1050 × 830, Vorschau größer, „Aufräumen“ ohne Scrollen sichtbar, Größe gemerkt |
| ☐ **P-KI** | N21: Fortschrittsfenster der KI-Werkzeuge schön gestalten (Design wie der Nachzeichnen-Dialog, alle 3 Designs); prüfen, welche Stellschrauben SAM 2.1/BiRefNet wirklich hergeben (Maskenschwelle, Kante weich/ausdehnen/zusammenziehen, Maskenvariante bei SAM), nur diese im Inspektor anbieten; optional: Vorschau der Objektmaske beim Überfahren |
| ☐ **P-BV1…3** | N22 Bildvorschau: **Cleanroom**-Agenten, die *nur* `re\17-bildvorschau.md` lesen, in den dort vorgeschlagenen 2–4 Paketen (z. B. 1: Fensteraufbau, 1/2/4-fach, Feld-Vorgaben, Kopfzeilen; 2: Optionen mit Palette, Farbtabelle, Transparenz-Pipetten; 3: Datei/Exportbereich, Animation, Werkzeuge, Zoom) |
| ☐ **P-TE2** | Technisches Englisch, Teil 2: `.ftl`-Dateinamen (`befehlstexte.ftl`, `nachzeichnen.ftl` …) und deutsche Datei- und Testnamen im Quelltext umbenennen, nach der Liste von TE-1. Erst wenn wenige Agenten in den `.ftl`-Dateien arbeiten |

### D. Gestaltung und Feinschliff

| Paket | Inhalt |
|---|---|
| ☐ **P-DESIGN** | N11: Dialoge und Startbildschirm schöner (DESIGN.md §13), Vorher/Nachher-Bilder in 3 Designs |
| ☐ **P-RAND** | N23: **Prüfung der ganzen App auf abgeschnittene Texte und zu knappe Ränder.** Alle Bedienfelder, Dialoge, Inspektor-Seiten, Menüs, Tooltips; 3 Designs × de/en × 100/150 % × kleine/große Fenster. Beispiel: „Zeilensprung/Progressiv“ unter „Glättung“ halb abgeschnitten. Automatischer Test, der Textüberlauf in kittest erkennt, und Korrekturen |

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
