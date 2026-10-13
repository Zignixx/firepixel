# Firepixel – Gesamtplan

Stand: **13.10.2026 ≈ 02:50, nach Welle 3 und den Nachläufern** (auf den Ist-Stand gebracht von DOC3-4; vorher DOC-2 nach Welle 2, DOC-1 in der Pause ≈ 20:15). Zusammenfassung für den Nutzer: `docs\progress-2026-10-13.md`. Dieses Dokument ist der Fahrplan bis „fertig“. Es sagt, was erledigt ist, was gerade läuft und was in welcher Reihenfolge noch kommt. Die Einzelheiten stehen in den verlinkten Dateien. Wer ein Paket abschließt, hakt es hier ab (☐ → ☑) und trägt den Commit ein.

Verwandte Dateien:
- `STATUS.md`: Stand je Crate und je Agent
- `ARCHITEKTUR.md`: verbindlicher Bauplan
- `docs\AGENTEN_REGELN.md`: Regeln für alle Agenten
- `docs\BEFUNDE.md`: offene und erledigte Befunde je Bereich
- `docs\NUTZERBEFUNDE.md`: vom Nutzer gemeldete Fehler und Wünsche N01–N24
- `docs\berichte\*.md`: Berichte der Agenten ab Welle 2 (AGENTEN_REGELN §7b.2), vom Doku-Agenten übertragen
- `docs\PARITAET_A.md` / `PARITAET_B.md`: Abgleich mit dem Original, Knopf für Knopf
- `docs\FUNKTIONSTEST.md` und `docs\funktionstest\*_auto.md`: automatischer Gesamttest
- `docs\NACHPRUEFUNG.md`: Prüfungen im echten Fenster für den Nachtlauf (§7a)
- `docs\TESTLEITFADEN_2026-10-12.md`: Hands-on-Test für den Nutzer, alle Änderungen seit 12.10. 04:00

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
| **Abgleich mit dem Original** | PARITAET_A: **123 ✅ / 1 ⚠️ / 2 ❌** von 126 Zeilen (vorher 31/18/27, 86/22/9, 106/16/4); Warnungen Z01–Z18 mit „Nicht mehr anzeigen“, Schieberkurven, gemerkte Dialogwerte, Wiederholen im Menü, Fang, Frei transformieren, Zeiger, Inspektor-Felder (FV-1…FV-16, PM-1…PM-7, EB-1, DLG-1…DLG-9, KV-1…KV-3, E1/E2, MN-2/MN-3) | 🔄 PARITAET_B: Menüs 276 ✅ / 32 ⏳, Dialoge 3 ✅ / 40 ⏳ / 6 ❌, Bedienfeldmenüs 1 ✅ / 8 ⏳ (§7) |
| **Fehler des Nutzers** | N02–N10, N12–N20 ✅ im echten Fenster | 🔄 siehe §3 |
| **Testversion** | `Desktop\Firepixel-Vorschau-12-10\` (Stand `d71dba1`) an den Nutzer übergeben | ☑ |

---

## 3. Fehler und Wünsche des Nutzers (`NUTZERBEFUNDE.md`)

| Nr | Kurz | Stand | Paket |
|---|---|---|---|
| N01 | Filter-Größe mehrfach ändern stapelt den Effekt | 🔧 Rest (Abbrechen ohne Verlaufsrest) von NZ-1 `ec4ac99` behoben, Nachprüfung im Nachtlauf (unverändert nach Welle 3) | P-PRUEF ☑ / NACHPRUEFUNG |
| N02–N10, N12–N18 | Dialoge ziehen, Zoom, Kopieren, Start, Text, Speichern unter, Neutralität … | ✅ im echten Fenster | ☑ |
| N11 | Dialoge/Startbildschirm schöner | ❌ teilweise: Dialograhmen DES-1 `4109217`, Startbildschirm DES-2 `a728586` umgesetzt; offen Tabellen/Listen einzelner Dialoge (DESIGN §13 Prio 5–11), Voreinstellungen ohne Fußleiste | DES-1/DES-2 ☑, Rest DES-3 ☐ |
| N19 | Suche in der Schriftarten-Liste | ✅ FV-12 `e8dd1ad` | P-N19-20 ☑ |
| N20 | „Bitmap nachzeichnen“ standardmäßig größer | ✅ FV-12 `e8dd1ad` | P-N19-20 ☑ |
| N21 | KI-Fortschrittskarte + Feineinstellungen | 🔧 KI-2 `39c6e6a`, Nachprüfung im Nachtlauf (unverändert nach Welle 3) | P-KI ☑ |
| N22 | Datei ▸ Bildvorschau vollständig | 🔧 **komplett umgesetzt** (P1–P4: BV-1, BV-2, BV-3, BV-4 `ad67a00`), Nachprüfung im echten Fenster offen | BV-4 ☑ |
| N23 | abgeschnittene Texte, zu knappe Ränder | 🔧 umgesetzt: RAND-1 `5f2d009`, RAND-2a `21edb24`, RAND-2b `247ce0c`/`e1d8909`, MN-6 `a6fd9d3`; **Prüflauf 0 Funde, Erlaubnisliste leer** (vorher 70 Stellen / 408 Funde); Nachprüfung im echten Fenster offen | RAND-2a/b ☑ |
| N24 | Schriftliste ohne Bildlaufleiste | 🔧 NZ-2 `8a79ab9`, app-weit NZ-3 `3a3342b`; Nachprüfung im Nachtlauf | ☑ |

---

## 4. Stand nach Welle 3 (13.10.2026 ≈ 02:50)

**Es laufen TE-2 (seit ≈ 02:45, 6 Agenten TE2-A…F) und DOC-3 (6 Agenten DOC3-1…6).** Welle 3 und alle Nachläufer sind fertig (Tabelle unten). **Zusammenfassung, Offenes, Entscheidungen und nächster Ablauf: `docs\progress-2026-10-13.md`.** Die Berichte unter `docs\berichte\` überträgt DOC-3 in BEFUNDE/PARITAET/NACHPRUEFUNG/NUTZERBEFUNDE/STATUS/PLAN (`docs\berichte\README.md`). Testversion 13.10. auf dem Desktop (`NEU.md` = `docs\preview-notes-2026-10-13.md`). Die Nachttests starten nur nach „los“ des Nutzers (§4c Nachtlauf, §6).

### Welle 3 und Nachläufer (13.10. 00:00–01:55, warme Bau-Slots)

| Agent | Paket | Commits | Kurzergebnis |
|---|---|---|---|
| ☑ BV-4 | Bildvorschau P4 | `ad67a00`, Bericht `1dbd2a4` | OK/Exportieren als ein Verlaufsschritt, Exportdialog über der Vorschau, Esc, Mausrad, Vorgaben → **N22 komplett** |
| ☑ E3 | HTML-Einrichtung, Exportassistent | `42dabe7` | 3 Registerkarten, Assistent als ein Fenster bis zur Bildvorschau, HTML kopieren/aktualisieren, Rollover `_s2` |
| ☑ DLG-10 | D10, D24 | `d01f3cc` | Datei-Info als Metadaten-Fenster (XMP im Dokument, neutral), Rechtschreibprüfung einrichten (nur Ordnerwahl) |
| ☑ DLG-11 | D10 Größe | `1fe6899` | Datei-Info außen 680 × 620, Titel = Dateiname; Register des Originals nicht messbar |
| ☑ MN-4 | Menü-Beschriftungen | `0172907` | Befehle-Menü (5 fehlende Befehle), Strg+AKUT/ZIRKUMFLEX, „Wiederholen“ mit Namen, „Im Dateibrowser durchsuchen…“ |
| ☑ MN-5 | Menü-Rest | `3cc575f`, Bericht `a682eda` | „Wiederholen: <Filter>“, Skripttexte DE/EN (`fp.t`), „Maske deaktivieren“ ohne Kürzel, Rückfrage im Optimieren-Bedienfeld |
| ☑ MN-6 | zu hohe Menüs | `a6fd9d3`, Bericht `bfb8e3d` | Pfeilleisten wie native Windows-Menüs (Haupt-, Unter-, Kontext-, ≡-Menüs, Tastatur); Erlaubnisliste 4 → 0 |
| ☑ RAND-2a | N23 R2-1/R2-2 | `21edb24`, Bericht `20be741` | Inspektor im schmalen Fenster, Text-Inspektor; 55 → 0 Stellen |
| ☑ RAND-2b | N23 R2-3…R2-5 | `247ce0c`, `e1d8909` | Dialoge auf das Fenster begrenzt (Bildlauf, feste Knopfzeile), Prüfer für Menüs/Tooltips; Erlaubnisliste 17 → 4 → (MN-6) 0, **0 Funde** |
| ☑ AU-3 | JSF (gekürzt) | `cf504bb` | Textrichtung wie `re\09` T5, Texturnamen in Skripten, „Andere…“ merkt den Pfad; F7 bleibt bis zur Messung offen |
| ☑ PERF-1 | Tausende Objekte | `0b208c8`, `9a60015`, Bericht `69c2d1a` | iPad.png: Bild bei voller Auswahl 180–220 → 24 ms, Pfeiltaste 250–430 → 56 ms (Release) |
| ☑ NZ-3 | Nachzügler Oberfläche | `3a3342b`, Bericht `ff6ff0a` | Bildlaufleisten app-weit, Schrift-Menü ✓, Zugriffstasten, Info-Symbol, Hilfe-Knopf |
| ☑ NZ-4 | Nachzügler Engine/Export | `08187d5`, `a285413`, Bericht `e29a8f1` | pHYs 2834, Neue Schaltfläche, „Auf Status verteilen“, Sprite-Werte im FW-PNG, Zusatzfelder Dateidialog, Startordner „Bilder“ |
| ☑ NZ-5 | Brücke/Dateidialog | `cbcc2b4`, Bericht `1dd9df8` | `ui_file_dialog` kennt die neuen Kästchen, Benutzerordner nur über den Testprofil-Wächter |
| ☑ FORM-1 | Formular-Grundlagen | `64d5511` | Tab-Folge je Vorlage, ▾ kein Tab-Stopp, gemerkte Auswahllisten, Klemmen beim OK, Klick neben Dialog schließt nicht |
| ☑ DES-1 | N11 Dialoge | `4109217` | Fußleiste, Titelleiste, Schatten, getönter Standardknopf, Gruppenrahmen |
| ☑ DES-2 | N11 Startbildschirm | `a728586` | Banner, drei Karten mit Vorschaubildern, Tastatur, 3 Designs |
| ☑ NEUTRAL-1 | Neutralität Doku/Bild | `8275f78` | fp-neutral prüft Doku-Abdeckung und Bildtext (Texterkennung, 596 Bilder), 10 Bilder geschwärzt |
| ☑ RE-BATCH | Spezifikation Batch | (`re\21-batch.md`, nicht im Repository) | ≈ 420 Zeilen, 13 offene Messungen M1–M13 |
| ☑ Koordinator | Stand 13.10. | `d6671bb` | `progress-2026-10-13.md`, Testversions-Notizen |

### Früher: Stand nach Welle 2 (12.10.2026 nachts)

**Kein Agent lief.** Welle 1 und Welle 2 sind fertig; die Doku hat DOC-2 übertragen (Berichte unter `docs\berichte\`). Weiter mit Welle 3 (§4c), sobald der Koordinator startet.

### Welle 1 (nach der Pause, 12.10. ab ≈ 21:00)

| Agent | Paket | Commits | Kurzergebnis |
|---|---|---|---|
| ☑ EB-2 | P-EB1 Reduziertes PNG | `f5169d8`, Doku `67c06f3` | Typ „Reduziertes PNG“ in Speichern unter, „Als Kopie speichern“, Hinweis, „Optionen…“ → Bildvorschau; SWF/AI8 gestrichen |
| ☑ KV-1 | Kurven nach `re\19` | `5548dab`, `0afa445`, `a4e258f`, `5c1d4cb`, `177d99a`, Doku `431fc13` | Feld 208 × 208, Rampen, Mausregeln, Auto und Pipetten im Dialog; D43 ⏳ |
| ☑ KV-2 | Tonwertkorrektur nach `re\19` | `2e8221e`, Doku `35fe6e7` | Histogramm, Schieber mit Griffen, Gamma, Auto und Pipetten; D44 ⏳ |
| ☑ E1 | Export-Dialog nach `re\18` | `4ff82ef` | ein Ablauf mit Ordner, Name, 10 Arten, Unterpaneelen, Rückfrage, Merken; D07 ⏳ |
| ☑ BV-3 | Bildvorschau P3 | `16741b9`, Doku `636ea41` | Registerkarte Datei, Werkzeug Exportbereich, Registerkarte Animation, Abspielknöpfe |
| ☑ MN-2 | Menüfreigabe und -aufbau | `06c3203`, Doku `3c0e38f` | 32 Menüzeilen ⏳ (21 ❌ + 11 ⚠️): Pixelauswahl, Segment/Gruppe, Untermenü-Köpfe, Text ▸ Schrift |
| ☑ DLG-5 | P-DLG5 Farbe/Schärfe | `5f06b0c`, Doku `5db2a58` | D41, D42, D45, D46, D48 ⏳; gemerkte Werte in allen Filterdialogen, „Bild filtern“ |
| ☑ NZ-2 | Nachzügler: rote Tests, N24 | `8a79ab9`, `1a56ce8`, `efbe9e8`, `56ce03e`, `5b41c8a` + `12f01dd`, Doku `7a28386` | N24 Bildlaufleiste der Schriftliste, `ux_b_abgleich` parallel stabil, `clipping_audit` grün, `file_dialog::is_open` |

### Welle 2 (12.10. ab ≈ 22:30, warme Bau-Slots, AGENTEN_REGELN §7b)

| Agent | Paket | Commit | Kurzergebnis |
|---|---|---|---|
| ☑ E2 | Optionen-Dialoge des Exports | `28abb4f` | PDF (Version, Komprimierung, Kennwortschutz), CSS-Sprites (Selektor, Präfix/Suffix, Layout, Meldung), Ebenen/Status/Seiten in Dateien, Photoshop; „Optionen…“ je Art; D07 ⏳ |
| ☑ KV-3 | Kurven/Tonwert live, Bitmap-Auswahl | `0447b9f` | „+“ öffnet den Dialog, ⓘ bearbeitet mit den gespeicherten Werten; Bitmap-Auswahl speichern/wiederherstellen/löschen nach `re\19` §5; D40 ⏳ |
| ☑ DLG-4 | P-DLG4 | `ac6e85c` | D14 Neues Symbol öffnet die Symbolbearbeitung, D25 Meldung ohne eigene Befehle, D16 Kategorien/Gruppen/Hilfe, D17 Menübaum; alle ⏳ |
| ☑ DLG-6 | P-DLG6 Weichzeichner | `43e6bbe` | D47/D49/D50 Grenzen wie im Original (Zuschnitt auf Alpha > 0, Bewegung bis Abstand − 1), Beschriftungen, Anordnung, Startfokus OK; ⏳ |
| ☑ DLG-7 | P-DLG7 Pfad/Transformieren | `8c704d0` | D26 live und gemerkt, D29–D31 Beschriftungen/Anordnung, neuer Dialog D32; ⏳ |
| ☑ DLG-8 | P-DLG8 Auswahl | `7543f7a`, Bericht `eba28e8` | D35–D39 Anordnung, „Breite:“/„Auswahlradius:“, Verlaufsnamen; +6 bei „Erweitern um 5“ folgt aus `re\05`, Messung im Original offen |
| ☑ DLG-9 | P-DLG9 Symbol/Animation | `285599d` | D27/D28 Anordnung, Rechteck mit Bewegungsweg, Schritt „In Symbol konvertieren“, D34 zweite Meldung; ⏳ |
| ☑ MN-3 | Bedienfeldmenüs ≡ | `5489b22` | 8 Zeilen ⏳: Freigaben je Lage, Schließen im Reiter-Kontextmenü, Eigenschaften-≡ nur „Hilfe“, „+ ▸ Optionen“ |
| ☑ DOC-2 | Doku der Wellen 1 und 2 | siehe `git log --grep "^DOC-2:"` | PARITAET_B, BEFUNDE (11 erledigt, 22 neu), NACHPRUEFUNG (+24 Zeilen), PLAN, STATUS, NUTZERBEFUNDE |

Beschleuniger (Nutzerentscheidung 12.10. ≈ 17:00), gelten weiter:
1. Die Firepixel-Seite wird über die Brücke ohne GUI-Sperre geprüft (AGENTEN_REGELN §4e). Echte Systemmaus nur fürs Original, für Eingaben, die vom Betriebssystem abhängen, und fürs Durchklicken am Ende.
2. Reine Spezifikations-Agenten ohne Build laufen zusätzlich.
3. ~~sccache~~: abgeschaltet (0 % Treffer). Schneller wird es über rust-lld (`843de17`) und das gebündelte Testprogramm (INF-1).

### Fertig vor der Pause (12.10. nachmittags/abends)

| Agent | Paket | Commits |
|---|---|---|
| ☑ FV-14 | P-LW1 Leinwand-Rest (L4/L35, L7, L13, L25, L33, durchsichtige Bitmap-Pixel) | `8b03953`, `aae2d00` |
| ☑ PM-4 | P-PRUEF Nachprüfung im echten Fenster | `2797c0e` |
| ☑ PM-5 | P-PM4 Teil 1, Dialoge D01–D25 | `360b0c7` |
| ☑ PM-6 | P-PM4 Teil 2, Dialoge D26–D50 (3/16/6), Pakete P-DLG5–9 | `0b5eaa4` |
| ☑ PM-7 | P-PM4 Teil 3, Hauptmenüs (276/20/25/29) und Bedienfeldmenüs (1/4/4/1), Pakete MN-2–4 | `d597680` |
| ☑ DLG-1 | P-DLG1 Teil 1: Zahlenfelder, Tab-Folge, Wert beim Klick in ein anderes Feld | `fc7df14`, `173bca1`, `d45812a`, `a902429`, `fd33481` |
| ☑ DLG-1b | P-DLG1 Teil 2: D19, D20, D01, Verlaufsschritte, D16 | `6c557e6`, `91a4d90`, `00809ba`, `aabc863`, `5d5feae` |
| ☑ DLG-2 | P-DLG2: D18, D21, D22, D23 | `9469497`, `6c46015` |
| ☑ DLG-3 | P-DLG3 Datei-Dialoge + P-HG1 Schließen | `9c7c28b`, `af8fe21`, `18ab983`, `7540512` |
| ☑ NZ-1 | Nachzügler PM-4: N01-Rest, W28, Texturliste, Barrierefreiheit | `ec4ac99`, `64d976e` |
| ☑ FV-15 | P-IBD1 Texturliste + „Andere…“, `pi.text.antialias`, Hinweis KI | `5eee815`, `ae5581d`, `7ef703c` |
| ☑ FV-16 | P-LW2 Verlauf-Werkzeug, Farbe ersetzen, Texturen per Name | `118aada`, `be5d176` |
| ☑ KI-2 | P-KI (N21) Fortschrittskarte + Feineinstellungen | `39c6e6a`, `8482c8f` |
| ☑ RAND-1 | P-RAND Teil 1: Prüfer `clipping_audit.rs`, N23-Beispiel | `5f2d009`, `101cd74` |
| ☑ BV-1 / BV-2 / BV-3 | Bildvorschau P1 / P2 / P3 | `9906252`, `0c01836`, `71577d5` / `9fc13e6`, `f476f35` / `16741b9` |
| ☑ FV-12, FV-13, EB-3, TE-1, FV-8, FV-11 | siehe §5 und STATUS | `e8dd1ad`, `9e14aab`, `777ce3d`, `c1e16e1`, `8f6bcfc`, `24e9524` … |
| ☑ RE-EX | Spezifikation `re\18-export-dialoge.md`, Pakete E1–E3 | (`re\`, nicht im Repository) |
| ☑ RE-KT | Spezifikation `re\19-kurven-tonwert.md`, Pakete KV-1–3 (früher KT-1–3) | (`re\`) |
| ☑ RE-SWF | Spezifikation `re\20-ai8swf.md`; umgesetzt wird nur Reduziertes PNG (Pakete A, B) | (`re\`) |
| ☑ INF-1 | rust-lld als Linker; ein Testprogramm `all` je Crate für fp-ui und fp-engine (neue Testdateien automatisch, Sprache je Test-Thread), 8 + 1 Einzelprozess-Tests; Regeln in AGENTEN_REGELN §2 | `843de17`, `b656ad5` |

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

Danach geht es weiter, wenn der Nutzer „weiter“ sagt. **Reihenfolge nach der Pause:** EB-2 → KV-1/KV-2 → E1 → BV-3 → DLG-4 → MN-2 → P-DLG5–9 → danach E2, KV-3, BV-4, MN-3, MN-4, RAND-2 usw. – Wellen 1 und 2 erledigt (§4), weiter mit §4c. Die Kürzungen in §4b bleiben gültig.

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

## 4c. Welle 3, Welle 4 und Nachtlauf (Stand 13.10. ≈ 02:50, DOC3-4)

Namen: Kurven/Tonwert-Pakete heißen **KV-1…3** (früher KT-1…3), die Klicktests am Ende **CT-1…4** (früher ebenfalls KT). Jede Welle höchstens 8 Agenten, Berichte nach `docs\berichte\`, danach ein Doku-Agent.

**Welle 3 (Funktion):**

| Paket | Inhalt | Quelle |
|---|---|---|
| ☑ **BV-4** (`ad67a00`) | Bildvorschau P4: OK schreibt die Einstellung des aktiven Feldes, Exportieren, Esc schließt, Vorgabenreihenfolge, Tooltips (N22) | `re\17` §10, NUTZERBEFUNDE N22 |
| ☑ **E3** (`42dabe7`) | HTML-Einrichtung (D13) und Exportassistent (D11) **nur funktional** (§4b); HTML-Einrichtung als „Optionen…“ der HTML-Arten; Reste aus E1 (Rollover `_s2`/`_f2`, Hinweise 10120–10122/3254, Startordner „Bilder“) | `re\18`, STATUS E1 |
| ☑ **DLG-10** (`d01f3cc`) | P-DLG-X Rest: D10 Datei-Info (XMP-Fenster), D24 Rechtschreibprüfung einrichten, D15 HTML-Code kopieren (nur funktional); Hinweis-Meldungen aus dem alten P-DLG5 | PARITAET_B §2 (❌ D10, D15, D24) |
| ☑ **MN-4** (`0172907`) | Beschriftungen im Menü: Befehle-Menü (fehlende mitgelieferte Befehle, Namen 1:1), Kürzeltexte AKUT/ZIRKUMFLEX, „Maske deaktivieren“, „Filter wiederholen“ mit Filternamen, „Bridge durchsuchen“ | PARITAET_B §1 (9 ⚠️, 4 ❌) |
| ☑ **RAND-2a** (`21edb24`) | abgeschnittene Texte: R2-1 Inspektor im 800-pt-Fenster, R2-2 Text-Inspektor, R2-3 Dialoge höher als das Fenster | STATUS RAND-1, N23 |
| ☑ **RAND-2b** (`247ce0c`, `e1d8909`) | R2-4/R2-5 Kleinkram, `list_frame` über der Fußleiste (NZ-2), Zeilensprung-Kästchen der Bildvorschau; Prüfer um Menüs und Tooltips erweitern | STATUS RAND-1, BEFUNDE |
| ☑ **AU-3** (`cf504bb`; F7 offen bis zur Messung) | gekürzt (§4b): F7 Feldübernahme beim Objektwechsel, vertauschte JSF-Textrichtung | BEFUNDE AU |
| ☑ **PERF-1** (`0b208c8`, `9a60015`) | gekürzt (§4b): Alles auswählen und Pfeiltasten auf Seiten mit Tausenden Objekten | BEFUNDE LW/HG (FV-8) |
| ☑ **DLG-11** (`1fe6899`, neu) | Datei-Info in gemessener Größe (außen 680 × 620), Titel = Dateiname | PARITAET_B D10 |
| ☑ **MN-5** (`3cc575f`, neu) | „Wiederholen: <Filter>“, Skripttexte DE/EN (`fp.t`), „Maske deaktivieren“ ohne Kürzel, Rückfrage beim Löschen im Optimieren-Bedienfeld | PARITAET_B §1 |
| ☑ **MN-6** (`a6fd9d3`, neu) | Menüs höher als das Fenster mit Pfeilleisten wie native Windows-Menüs; Erlaubnisliste 4 → 0 | RAND-2b, Befund HG |
| ☑ **NZ-5** (`cbcc2b4`, neu) | `ui_file_dialog` mit den neuen Dateidialog-Kästchen, Benutzerordner nur über den Testprofil-Wächter | NZ-4 |
| ☑ **FORM-1** (`64d5511`, neu) | Formular-Grundlagen: Tab-Folge je Vorlage (D27/D28/D14), ▾ kein Tab-Stopp, `load_prefs` mit Text-Auswahllisten, Klemmen beim OK, Klick neben modalen Dialog schließt nicht, D26 Startfokus | NZ-3-Liste |
| ☑ **RE-BATCH** (neu, `re\21-batch.md`) | Spezifikation des Batch-Assistenten (≈ 420 Zeilen, 13 offene Messungen M1–M13) | PARITAET_B D12 |

**Welle 4 (Aufräumen und Gestaltung):**

| Paket | Inhalt |
|---|---|
| ☑ **NEUTRAL-1** (`8275f78`) | fp-neutral prüft auch `STATUS.md`, `docs\*.md`, `docs\berichte\*`, `docs\funktionstest\*` (P-NEUTRAL) |
| 🔄 **TE-2** – läuft (13.10. ≈ 02:45, 6 Agenten TE2-A…F) | Technisches Englisch Teil 2: `.ftl`-Dateinamen, Quelldateien, Anleitung, Umgebungsvariablen; Testnamen bleiben (§4b) |
| ☑ **DES-1** (`4109217`) | N11: Dialoge schöner (DESIGN.md §13), Vorher/Nachher in 3 Designs |
| ☑ **DES-2** (`a728586`) | N11: Startbildschirm schöner, Vorher/Nachher in 3 Designs |
| ☑ **NZ-3** (`3a3342b`; Formular-Punkte von FORM-1 `64d5511`) | Nachzügler Oberfläche (Restpunkte aus Welle 1/2, je Befund in BEFUNDE): Popup-Schieber ▾ kein Tab-Stopp; `FormDialog::load_prefs` bei Text-Auswahllisten; Klemmen beim OK (19105–19108); Tab-Folge der Knöpfe in Formularen (D27/D28); D14 gesperrtes Kästchen „In allgemeiner Bibliothek speichern“; Info-Symbol in `show_message` (D25); D17 Rest (Symbolknöpfe, Fokus/Pfeiltasten im Baum, Kombifeld „Befehle:“); D26 Startfokus auf der Liste; Verlaufsnamen der Pfade („Pfade erweitern“/„Pfade einlassen“); Live-Anzeige in „Bitmap-Auswahl wiederherstellen“; Fußknopf „Auf Status verteilen“ ab 1 Objekt; Accessibility-Name für „Optionen…“ im Exportdialog; Klick neben modalen Dialog schließt ihn (NZ-2); Bildlaufleisten app-weit sichtbar (HG, NZ-2); Aufräumen: alte Formulare in `catalog.rs`, unbenutzte Schlüssel `dlg-fx-*`, `dlg-op-*`, `btn-help` in `allgemein.ftl` |
| ☑ **NZ-4** (`08187d5`, `a285413`; Messungen im Original → CT-4) | Nachzügler Engine/Export und Messungen: Sprite-Felder als FW-PNG-Tag; PDF „Textauswahl aktivieren“ (unsichtbarer Text); Photoshop-Optionen Effekte/Vektor/Text und Ebene je Objekt; offene Fragen aus `re\18` (Raster-Layout, „_“, PDF-Qualität); wgpu-Renderer im Testprogramm `all` (INF, NZ-2). **Im Original messen** (eine GUI-Sitzung): Bewegungsunschärfe Winkel 0, Auswahl erweitern 1/5/10 und begrenzen 5, Animieren mit Richtung ≠ 0, „Neue Schaltfläche“, Lage von Volle/Halbe Höhe, Optimieren-Palettenbefehle bei GIF, Fußknopf „Auf Status verteilen“, englische Texte D39 und Verlaufsnamen |
| 🔄 **DOC-3** – läuft (6 Agenten DOC3-1…6) | Berichte der Welle 3 und Nachläufer in BEFUNDE, PARITAET, NACHPRUEFUNG, NUTZERBEFUNDE, STATUS, PLAN übertragen; Testleitfaden; DESIGN §12 + Bilder |

**Neu offen (aus `progress-2026-10-13.md` §2):**

| Paket | Inhalt |
|---|---|
| ☐ **CMD-1** | Bildschirmfoto, Symbolskript erstellen, Gemeinsame Palette echt umsetzen (Nutzerentscheidung 13.10.), ≈ 35 min |
| ☐ **BATCH-1** | Batch-Assistent nach `re\21-batch.md` (Nutzer: ja, aber später; braucht vorher die Messungen M1–M13 im Original), ≈ 60–80 min |
| ☐ **DES-3** | N11-Rest: Tabellen/Listen einzelner Dialoge (DESIGN §13 Prio 5–11: KI-Modelle, Gespeicherte Befehle, Tastenkürzel, Pop-up-Editor, Export-Leerraum), Voreinstellungen ohne Fußleiste (Kategorienliste kürzen) |
| ☐ **Fix-Liste → FIX** | 8 kleine Punkte aus `progress-2026-10-13.md` §2 (Standard-ALT-Text im FW-PNG, Tab-Folge der Live-Effekt-Dialoge, „Neues Symbol“ im Bibliothek-Bedienfeld, Optimieren ▸ Einstellungen löschen ohne Rückfrage, `shortcuts.json` gegen PARITAET_B §1, alte Texte `dlg-md-*` (mit TE-2), „Demo – aktuelles Dokument“ im Fenster, erste Ziehbewegung bei Tausenden Objekten) – im FIX-Schritt des Nachtlaufs oder früher |

## 4d. Welle 5 – Nutzerbefunde N25–N30 (gemeldet 13.10. ≈ 03:00; **vor** den Nachttests, Start nur nach Freigabe)

Nutzervorgabe: aufschreiben, nicht sofort starten; diese Punkte laufen **vor** dem Nachtlauf. Messungen im Original nur unter der Sperre (AGENTEN_REGELN §4e), also nacheinander; alles andere parallel auf warmen Slots. Bilder des Nutzers: `re\_nutzerbilder\2026-10-13\` (außerhalb von `firepixel\`, weil sie das Original zeigen).

**Teil 5a (parallel, ≈ 60–90 min):**

| Paket | Nutzerbefund | Inhalt | Dauer |
|---|---|---|---|
| ☐ **RE-5** | N25, N26, N27 | Messungen im Original (Sperre): Bildvorschau 2-/4-fach – was läuft synchron (Bildlauf, Mausrad, Leertaste + Ziehen, Hand, Zoom, Auswahl/Slice-Werkzeug); Grenzen eines Bitmap-Objekts in einer FW-PNG mit transparentem Rand (JSF `pixelRect`/`left/top/width/height`, Datei aus Firepixel und aus dem Original gespeichert); Farbwähler-Popup (Raster, Farbanordnung, Maße, Felder, Pipette beim Überfahren, Systemfarbwähler, Tastatur, Esc/Eingabe) mit Bildern → `re\22-ansicht-bitmap-farbe.md` | ≈ 45 min |
| ☐ **RE-FX** | N28 | Messreihe im Original nach RE-5 (Sperre): jeder Live-Effekt (Schatten, Innerer Schatten, Glühen, Inneres Glühen, Abschrägung innen/außen, Prägen, …) und jeder Filter mit je 3 Einstellungen auf 3 festen Testbildern (Vektor, Text, Foto-Bitmap mit Transparenz), Export als PNG 32 → Referenzbilder + `re\23-effekte.md` | ≈ 60–90 min |
| ☐ **BV-5** | N25 | gemeinsame Ansichtslage für alle Felder der 2-/4-fach-Ansicht (Bildvorschau und Dokumentansicht): Bildlauf, Rad, Leertaste, Hand, Zoom wirken überall; Feinheiten nach RE-5 | ≈ 40 min |
| ☐ **BM-1** | N26 | Ursache suchen: FW-PNG-Import/-Export und Bitmap-Grenzen (Objekt = Leinwand?); nach RE-5 angleichen; Test mit nachgebauter Datei | ≈ 40 min |
| ☐ **FARB-1a** | N27a | Fehler: Ziehen im Farbfeld des Popups springt auf die alte Farbe zurück (z. B. Glühen ▸ Farbe) – an allen Stellen mit diesem Popup | ≈ 20 min |
| ☐ **VT-6** | N29 | zweites Nachzeichnen-Verfahren nach dem veröffentlichten Potrace-Fachartikel, eigenständig umgesetzt (keine GPL-Bibliothek), Auswahl „Verfahren“ im Dialog mit Vorschau, MCP/CLI-Parameter `engine`; Vergleich auf Strichzeichnung und Logo | ≈ 60–80 min |
| ☐ **START-1** | N30 | Startzeit der Release-exe messen und aufschlüsseln; teure Schritte verzögern bzw. in den Hintergrund; Zahlen vorher/nachher | ≈ 45 min |

**Teil 5b (nach RE-5/RE-FX, parallel, ≈ 45–60 min):**

| Paket | Nutzerbefund | Inhalt |
|---|---|---|
| ☐ **FARB-1b** | N27b | Farbwähler-Popup wie im Original an allen Stellen (Werkzeugleiste, Inspektor, Effekte, Dialoge, Bedienfelder) nach `re\22` |
| ☐ **FX-1…4** | N28 | Pixelvergleich mit den Referenzbildern aus RE-FX und Abweichungen beheben, aufgeteilt: FX-1 Schatten/Glühen (Glühen zuerst), FX-2 Abschrägung/Prägen, FX-3 Weichzeichner/Schärfen, FX-4 Farbe/Rauschen/übrige; Toleranz und Ergebnis je Effekt im Bericht |

Gesamt Welle 5 ≈ 2–3 h. Danach Doku-Übertrag (kurz), Slots vorwärmen, neue Testversion, dann Nachtlauf nach „los“.

## 4e. Moderner Batch-Modus (Nutzerwunsch 13.10. ≈ 03:00, ersetzt BATCH-1)

Spezifikation `docs\BATCH_MODERN.md` (SPEC-BATCH). **Gebaut während der Nachttests** als eigene Spur, je Paket ein frischer Agent nacheinander (Nutzer: „mit einem Subagenten während die UI-Tests laufen“); eigenes Crate/eigene Dateien, keine Überschneidung mit den FIX-Agenten. **Getestet erst, wenn alle UI-Tests und FIX vorbei sind** (Paket BATCH-T), danach GT-2.

## 4f. Reihenfolge ab 13.10. ≈ 03:05 (Nutzerwunsch)

1. TE2-C/D/E und SPEC-BATCH fertig laufen lassen; Koordinator: Restsuche alte Namen, TECHNISCH_ENGLISCH §E abgleichen, §7 Zahlen, Testleitfaden auf neue Namen. **Stopp → Freigabe Welle 5.**
2. Welle 5a (§4d), 3. Welle 5b, 4. Doku-Übertrag Welle 5 (DOC-4), 5. Slots vorwärmen + neue Testversion. **Stopp → „los“.**
6. Nachtlauf: GT-1 · CT-1a/1b (Dialoge, Menüs) · CT-2 · CT-3 · CT-4, parallel Spur BATCH-1…n (Bau) → FIX-1…n → BATCH-T (Batch testen und beheben) → GT-2 (Gesamtlauf inkl. Batch, Endversion auf den Desktop, Bericht im Chat).

**Nachtlauf (Abschluss Z), Ablauf laut `progress-2026-10-13.md` §4:**

1. Testversion vom 13.10. liegt auf dem Desktop (`Firepixel-Vorschau-2026-10-13_HH-MM`, mit `NEU.md`).
2. TE-2 + DOC-3 laufen (seit ≈ 02:45), danach **Welle 5 (§4d)**. **Danach die warmen Slots vorwärmen und eine neue Testversion bauen – erst nach Freigabe des Nutzers.**
3. Nachttests **nur nach „los“** des Nutzers (≈ 2–2,5 h):

   | ab „los“ | Paket | Inhalt |
   |---|---|---|
   | 0–45 min | GT-1 | alle Tests, alle automatischen Funktionsberichte, voller Abschneide-Prüflauf, Neutralität inkl. Bildtext, technisches Englisch, Scanner |
   | 15–60 min | CT-1 | Menüs und Dialoge im echten Fenster über die Brücke (NACHPRUEFUNG §3, §7) |
   | 15–60 min | CT-2 | Werkzeuge, Leinwand, Inspektor, KI (§1, §2, §6) |
   | 15–60 min | CT-3 | Bedienfelder, Docking, Datei-Abläufe, Bildvorschau/Export, Startseite (§4, §5) |
   | 15–60 min | CT-4 | Englisch, 3 Designs, Fenstergrößen + alle Messungen im Original (Liste in `progress-2026-10-13.md` §2) |
   | 60–105 min | FIX-1…n | jedes ❌ beheben, Fix-Liste oben |
   | 105–135 min | GT-2 | letzter Gesamtlauf, Endversion auf den Desktop, Abschlussbericht im Chat |

4. Danach CMD-1, BATCH-1, DES-3, CI nach Entscheidung des Nutzers.

Pakete:

| Paket | Inhalt |
|---|---|
| ☐ **GT-1** | Gesamttestlauf: `cargo test --workspace`, alle `funktionstest\*_auto.md`, i18n-Scanner 0, fp-neutral, technisches Englisch; Release-Build als Testversion |
| ☐ **CT-1** | Klicktest Menüs und Dialoge im echten Fenster, dabei NACHPRUEFUNG §3 und §7 abarbeiten |
| ☐ **CT-2** | Klicktest Werkzeuge, Leinwand, Inspektor, KI (NACHPRUEFUNG §1, §2, §6) |
| ☐ **CT-3** | Klicktest Bedienfelder, Docking, Datei-Abläufe, Bildvorschau/Export (NACHPRUEFUNG §4, §5) |
| ☐ **CT-4** | Englisch, 3 Designs, Fenstergrößen, Prüfungen im Original (Zeilen mit Art „Original“) |
| ☐ **FIX** | jedes ❌ aus GT-1/CT-1…4 beheben und erneut prüfen; danach Schritt Z 4 |

## 5. Warteschlange (in dieser Reihenfolge; jedes Paket = ein kleiner Agent)

Die Reihenfolge gilt, solange nichts Dringenderes vom Nutzer kommt. Sobald ein Platz frei wird, startet das oberste Paket, dessen Dateien nicht gerade ein anderer Agent bearbeitet.

### A. Abgleich mit dem Original: Befunde beheben

| Paket | Inhalt | Quelle |
|---|---|---|
| ☑ **P-LW1** | Leinwand-Rest von PM-3: Fang an Objektmitten beim Verschieben und bei Stiftpunkten (L4/L35), Alt-Ziehen aus Eckpunkt (L33), Bitmap nach Esc beim Zauberstab nicht mehr gewählt (L13), Zoomrahmen-Rand (L25), Strg+Leertaste+Klick (L7), Stift am Endpunkt (L34) | BEFUNDE LW ☑ FV-14 (`8b03953`): L4/L35, L7, L13, L25, L33 und der FV-11-Rest (durchsichtige Bitmap-Pixel) behoben; L34 und Restmessungen im Original in NACHPRUEFUNG |
| ☑ **P-LW2** | Verlauf-Werkzeug: Farben (im Original nachmessen) und Füllart-Knöpfe (W25/W26); Farbe ersetzen: Werksvorgaben (W8) + `prefs.json`-Typen; `fill_selection` gemeinsam für Eimer und Verlauf | BEFUNDE LW/IBD/EB ☑ FV-16 (`118aada`): Verlauf mit eigenen Farben (gemessen: Farbfeld Schwarz → Weiß unabhängig von Füll-/Strichfarbe) ohne Füllart-Knöpfe, Farbe ersetzen mit Werksvorgaben, `prefs.json`-Typen, Auswahl füllen/Toleranz je Werkzeug, dazu Texturen/Muster nur per Name (FV-15-Befund); Malen des Originals beim Ziehen nicht messbar (interner Fehler), NACHPRUEFUNG |
| ☑ **P-IBD1** | Texturliste: Benutzer-Texturordner (77) + „Andere…“ (W21-Rest); Feldtext beim Klick auf ein anderes Objekt (F7); `pi.text.antialias` Zuordnung; Inspektor-Hinweis „Esc: neues Objekt“ bei der Objektauswahl – FV-15: W21, Musterliste, `pi.text.antialias`, Hinweis erledigt; F7 im Original nicht messbar (interner Fehler bei jedem Leinwandklick), Befund offen bis zum Neustart des Originals | BEFUNDE IBD/EB/HG |
| ☑ **P-PRUEF** (PM-4, `2797c0e`) | **Nachprüfung im echten Firepixel-Fenster:** FV-10 (W19, W21, W22, W24, W26, W29, O2, O8, O16; die Prüfung wurde während der Testpause abgelehnt), FV-9-Reste (W10/W15/W17/W27/W28), N01, N13, N14, N16–N18; danach die Zähltabelle in PARITAET_A nachführen | FV-10/FV-9-Berichte, NUTZERBEFUNDE |
| ☑ **P-MENU1** | 8 Menüeinträge, die im Gesamttest durchfallen: Bitmap-Auswahl wiederherstellen/löschen, Hilfslinien entfernen, Pfade kombinieren ▸ Teilen, Vorschau im zweiten Browser (in keiner Testsituation verfügbar), Befehlsskripte „Andere Ebenen ausblenden/sperren“, „Warndialoge zurücksetzen“ (keine erkennbare Wirkung) – erledigt (FV-13: menue_auto 345/345, kontextmenues_auto 283/283) | `docs\funktionstest\menue_auto.md` |
| ☑ **P-AU1** → **AU-3** (`cf504bb`, gekürzt §4b; F7 offen bis zur Messung) | JSF wie im Original: zwei Aufrufe → andere Objekte (O11/O6), `pixelRect` beim Text ohne 2-px-Rand (L22), `fw.setActiveViewScale` in der Brücke, `addNewHotspot` „slice“ | BEFUNDE AU |
| ☑ **P-PERF** → **PERF-1** (`0b208c8`, `9a60015`, gekürzt §4b; erste Ziehbewegung → Fix-Liste) | Restliche langsame Stellen von FV-8: erste Ziehbewegung großer Objekte bis 1 s, erstes Bild nach Zoom/Bildlauf bis 1,4 s, Alles auswählen auf Seiten mit Tausenden Objekten (Pfeiltasten 250–430 ms), Inspektorfelder messen; Ziel ≤ 50 ms auch auf dem Software-Rasterer | BEFUNDE LW/HG (FV-8) |
| ☑ **P-NEUTRAL** → **NEUTRAL-1** (`8275f78`) | fp-neutral prüft auch `STATUS.md`, `docs\*.md` und `docs\funktionstest\*` (am 12.10. stand ein JSF-Funktionsname mit dem Namen des Originals in STATUS/BEFUNDE, vom Koordinator entfernt `620f021`, der Test hatte es nicht gefunden); Testreste im echten Profil sind entfernt (`Testbefehl*.jsf`, `Allgemeine Bibliothek\Sym.png` → Scratchpad), seit EB-3 `777ce3d` verhindert ein Wächter neue | BEFUNDE EB-3 |
| ☑ **P-HG1** | Datei-Dialoge im zuletzt benutzten Ordner; Rückfrage nach Strg+W erst nach 1,3–2 s; WM_CLOSE von außen beendet die App nicht | BEFUNDE HG – erledigt DLG-3 (9c7c28b, af8fe21; Lage/Zusatzfelder als Befund HG) |

### B. Menüs, Dialoge, Bedienfeldmenüs (PARITAET_B)

| Paket | Inhalt |
|---|---|
| ☑ **P-PM4** | **Neue Messrunde PARITAET_B** in beiden Programmen, erledigt in drei Teilen: PM-5 D01–D25 (`360b0c7`), PM-6 D26–D50 (`0b5eaa4`), PM-7 Hauptmenüs und Bedienfeldmenüs (`d597680`). Stand nach der Messung: Menüs 276/20/25/29 ℹ️, Dialoge 3/21/15/5 ⏳/6 🔧, Bedienfeldmenüs 1/4/4/1 ❔; nach Welle 2 siehe §7 |
| ☑ **P-PM4 Teil 1** | PM-5 (`360b0c7`): D01–D25 nachgemessen → 0 ✅ / 10 ⚠️ / 14 ❌ / 1 ⏳; 13 neue Befunde (IBD) |
| ☑ **P-DLG1** | Teil 1 (DLG-1, `fc7df14`, `173bca1`): Zahlenfelder nehmen wieder Eingaben an (Ursache: Fensterfokus in `buffered_edit`), Tab-Folge aller Dialoge (Titelleiste, Startfokus, Optionsgruppen); D19/D20/D01/Verlauf → DLG-1b; Teil 2 (DLG-1b, `6c557e6`, `91a4d90`, `00809ba`, `aabc863`): D19 live, D20 Anker/B:H:/Auswahl, D01 Zugriffstasten, keine leeren Verlaufsschritte, D16 Liste per Tab |
| ☑ **P-DLG2** | DLG-2 (`9469497`): D18 Raster (ohne Kästchen, ↔/↕, Lage der Vorlage), D21 untereinander, D22 gesperrt ohne Haken + Überlagerungsfarbe, D23 Größe der Auswahl/zuletzt benutzt + Popup-Schieber; Rest: Popup-Schieber in D18 im Original messen (Befund DLG-2) |
| ☑ **P-DLG3** | Dateidialoge D02–D06, D09: mittig über dem Fenster statt oben links, Startordner (nicht immer „Dokumente“), Titel, Filter, Zusatzfelder bei Öffnen/Speichern unter (zusammen mit P-HG1) – erledigt DLG-3 (af8fe21, 18ab983; Lage/Zusatzfelder als Befund HG) |
| ☑ **P-DLG4** | D14 Neues Symbol (OK öffnet die Symbolbearbeitung statt einer leeren Instanz bei 0,0), D25 gespeicherte Befehle (ohne eigene nur eine Meldung), D16, D17 – erledigt (DLG-4 `ac6e85c`; Reste in NZ-3) |
| ☑ **P-DLG5** | Farbe/Schärfe (IBD, von PM-6): D41 Farbton/Sättigung (Kennlinie, „Kolorieren“), D42 Helligkeit/Kontrast, D46 Unscharf maskieren, D48 Gaußscher Weichzeichner, D45 Rauschen; gemerkte Werte nach OK, Feld neben dem Schieber, Knöpfe rechts, Beschriftungen und Startfokus wie im Original – erledigt (DLG-5 `5f06b0c`; Nachprüfung in NACHPRUEFUNG) |
| ☑ **P-DLG6** | Weichzeichner-Wirkung (RF + IBD, von PM-6): D47 Bewegungsunschärfe (nur in Bewegungsrichtung, „Umfang“), D49 Radial (Stärke/Grenzen, „Menge“), D50 Zentriert („Anzahl“), gemerkte Werte – erledigt (DLG-6 `43e6bbe`; Messung Winkel 0 in NZ-4) |
| ☑ **P-DLG7** | Pfad/Transformieren (IBD, von PM-6): D26 Numerisch transformieren (B/H live, Symbole, Kette, gemerkt), D29 „Menge“, D30/D31 („Winkelgrenze“ 10.00, Anordnung), **neuer Dialog D32** Pfad in Auswahlrahmen (Rand + Breite) – erledigt (DLG-7 `8c704d0`; Startfokus D26 und ▾-Tab-Stopp in NZ-3) |
| ☑ **P-DLG8** | Auswahl (IBD + EB, von PM-6): D35 Weiche Kante (gemerkt), D38/D39 Beschriftungen, D40 Auswahl speichern mit Ziel und Vorgang, D36 Pixelfrage klären – erledigt (DLG-8 `7543f7a`: D35–D39, Pixelfrage aus `re\05` geklärt, Messung in NZ-4; D40 von KV-3 `0447b9f`) |
| ☑ **P-DLG9** | Symbol/Animation/Verhalten (IBD + EB, von PM-6): D27 Anordnung, D28 Beschriftungen/Trennlinien/Startfokus/Symbolrechteck, D34 zweite Meldung, D33 (nur Tab) – erledigt (DLG-9 `285599d`; Tab-Folge der Knöpfe in NZ-3) |
| ☑ **Gemerkte Werte** | in **allen** Filterdialogen (D41, D42, D45–D50) sowie D35 und D26 die per OK bestätigten Werte merken wie im Original; wird in P-DLG5–8 mit erledigt. Dazu Verlaufsnamen („Bild filtern“) als kleine Entscheidung für EB. Weiter offen aus dem alten P-DLG5…: Hinweis-Meldungen, Live-Filter-Menü „+“, Überlauf bei 1024 × 768 / 150 %. **Stand:** gemerkte Werte in allen Filterdialogen, D35 und D26 sowie Verlaufsname „Bild filtern“ für Filter erledigt (DLG-5 `5f06b0c`); Rest verteilt: Hinweis-Meldungen → DLG-10, Live-Filter-Menü-Namen → NZ-3, Überlauf → RAND-2a/b |
| ☑ **E1** | Export-Dialog (von RE-EX, `re\18` §9.6): ein Ablauf mit Artenliste (PDF, HTML und Bilder, Nur Bilder, CSS und Bilder, CSS-Sprites, Ebenen/Status/Seiten in Dateien, PSD, SVG), Unterpaneele mit Grau-Regeln, Ordner + Name in einem Dialog, Dateien je Art – umgesetzt (E1, 12.10.; Nachprüfung im Fenster ausstehend, Reste für E2/E3 in STATUS) |
| ☑ **E2** | Optionen-Dialoge (von RE-EX): PDF-, CSS-Sprite-, „Ebenen/Status/Seiten in Dateien“-, Photoshop-Optionen; „Optionen…“ je Art richtig verdrahten – erledigt (E2 `28abb4f`; Reste in NZ-4) |
| ☑ **E3** (`42dabe7`) | HTML-Einrichtung (D13) und Exportassistent (D11) **nur funktional** (von RE-EX, §4b): Felder, die der HTML-Export auswertet; Assistent als Zustandsautomat, „Beenden“ öffnet die Bildvorschau |
| ☑ **KV-1** | Kurven originalgetreu (von RE-KT, `re\19` §7.4): Layout, Zeichnung, Mausregeln, Felder, Auto und Pipetten im Dialog, Tests aus `ktref.py` – umgesetzt (KV-1, `5548dab`, `0afa445`; Tests `curves_kv1.rs`, `fp-raster` `curve_edit`), Nachprüfung im Fenster in NACHPRUEFUNG §3 |
| ☑ **KV-2** | Tonwertkorrektur originalgetreu (von RE-KT): Histogramm, Schieber-Widget mit Griffen, Gamma-Abbildung, Auto und Pipetten, gemeinsamer Vorschau-Schalter – umgesetzt (KV-2, `2e8221e`; Tests `levels_kv2.rs`, `fp-raster` `levels_math`), Nachprüfung im Fenster in NACHPRUEFUNG §3 |
| ☑ **KV-3** | Live-Bearbeitung und Auswahl-Dialoge (von RE-KT): Kurven/Tonwert als bearbeitbare Live-Filter, Dialoge „Bitmap-Auswahl speichern/wiederherstellen/löschen“ – erledigt (KV-3 `0447b9f`) |
| ☑ **P-DLG-X Rest** → **DLG-10** (`d01f3cc`), **DLG-11** (`1fe6899`); D12 → BATCH-1 | laut PM-5 außerdem unvollständig: D07, D10 (XMP-Fenster fehlt), D12 (Batch, „Später“), D15 (nur funktional, §4b); Bildvorschau D24 siehe P-BV |
| ☑ **MN-2** | Menüfreigabe und -aufbau (EB + Menü, von PM-7, ersetzt P-MENU2): Freigabe bei Pixelauswahl (12 Objektbefehle grau), Untermenü-Köpfe immer aktiv (11), Text ▸ Schrift wie im Original (auch ohne Text aktiv, ✓, zuletzt benutzte oben, Familien flach), Freigabe bei Segment/Gruppe (7). „Mit darunter liegender reduzieren“ und das Befehle-Menü im frischen Profil sind laut PM-7 schon gleich – erledigt `06c3203` (Nachprüfung im Fenster: NACHPRUEFUNG §7) |
| ☑ **MN-3** | Bedienfeldmenüs ≡ (von PM-7): Verlauf/Stile ohne Auswahl grau, Status/Seiten bei einem Element, Ebenen „Maske bearbeiten“, Optimieren (✓ Farbfeld-Rückmeldung, JPEG-Einträge), „Schließen“ ins Reiter-Kontextmenü, Eigenschaften-≡ – erledigt (MN-3 `5489b22`; Messfragen in NZ-4) |
| ☑ **MN-4** (`0172907`), Rest **MN-5** (`3cc575f`) | Beschriftungen (klein, von PM-7): Befehle-Menü (fehlende mitgelieferte Befehle, Namen 1:1, doppeltes „Warnmeldungen zurücksetzen“), Kürzeltexte AKUT/ZIRKUMFLEX, „Maske deaktivieren“ ohne Kürzel, „Filter wiederholen“ mit Filternamen, Entscheidung „Bridge durchsuchen“ |
| ☑ **EB-2** (P-EB1) | **nur „Reduziertes PNG“** (von RE-SWF, `re\20` §5, §4b): Paket A Typ in „Speichern unter“ (Filterindex, `.png` ohne `.fw.png`, Inhalt über die Export-Pipeline), Paket B „Als Kopie speichern“, Hinweistext, „Optionen…“ → Bildvorschau. SWF/AI8 entfallen – erledigt `f5169d8` (Zusatzfelder im Dialog der App, Folgedialog nach dem Windows-Dialog; Nachprüfung ausstehend) |

### C. Wünsche des Nutzers aus der Testversion

| Paket | Inhalt |
|---|---|
| ☑ **P-N19-20** (FV-12, e8dd1ad) | N19 Suche in der Schriftarten-Liste (Inspektor und Text-Menü, Tippen filtert, Pfeiltasten/Eingabe); N20 Nachzeichnen-Dialog standardmäßig ≈ 1050 × 830, Vorschau größer, „Aufräumen“ ohne Scrollen sichtbar, Größe gemerkt |
| ☑ **P-KI** (KI-2, `39c6e6a`) | N21: Fortschrittsfenster der KI-Werkzeuge schön gestalten (Design wie der Nachzeichnen-Dialog, alle 3 Designs); prüfen, welche Stellschrauben SAM 2.1/BiRefNet wirklich hergeben (Maskenschwelle, Kante weich/ausdehnen/zusammenziehen, Maskenvariante bei SAM), nur diese im Inspektor anbieten; optional: Vorschau der Objektmaske beim Überfahren |
| ☑ **P-BV1…4** | N22 Bildvorschau, **Cleanroom**-Agenten, die *nur* `re\17-bildvorschau.md` (§10) lesen. ☑ P1 Mehrfachansicht und Kern (erledigt, BV-1, Commits 9906252, 0c01836, 71577d5); ☑ P2 Registerkarte Optionen vollständig (Format/Palette/Rastern/Glättung/Transparenz mit 3 Pipetten, Farbtabelle mit Menü und Statuszeile, „Optimieren auf Größe“; erledigt, BV-2, Commit 9fc13e6, Nachprüfung im echten Fenster in NACHPRUEFUNG); ☑ **BV-3** P3 Datei (Skalieren, Exportbereich mit Werkzeug), Animation, Abspielknöpfe (erledigt, Commit 16741b9, Nachprüfung im echten Fenster in NACHPRUEFUNG); ☑ **BV-4** (`ad67a00`) P4 Verhalten (OK schreibt die Einstellung des aktiven Feldes, Exportieren, Esc schließt), Vorgaben, Texte. Nebenbefund: PARITAET_B D08 „Esc schließt nicht“ ist falsch, im Original schließt Esc |
| 🔄 **P-TE2** → **TE-2** – läuft (13.10. ≈ 02:45, 6 Agenten TE2-A…F) | Technisches Englisch, Teil 2: `.ftl`-Dateinamen (`befehlstexte.ftl`, `nachzeichnen.ftl` …) und deutsche Datei- und Testnamen im Quelltext umbenennen, nach der Liste von TE-1. Erst wenn wenige Agenten in den `.ftl`-Dateien arbeiten |

### D. Gestaltung und Feinschliff

| Paket | Inhalt |
|---|---|
| ☑ **P-DESIGN** → **DES-1** (`4109217`) / **DES-2** (`a728586`); Rest → ☐ **DES-3** | N11: Dialoge und Startbildschirm schöner (DESIGN.md §13), Vorher/Nachher-Bilder in 3 Designs |
| ☑ **P-RAND** (RAND-1, RAND-2a/b, MN-6: Prüflauf 0 Funde) | N23: **Prüfung der ganzen App auf abgeschnittene Texte und zu knappe Ränder.** Alle Bedienfelder, Dialoge, Inspektor-Seiten, Menüs, Tooltips; 3 Designs × de/en × 100/150 % × kleine/große Fenster. Beispiel: „Zeilensprung/Progressiv“ unter „Glättung“ halb abgeschnitten. Automatischer Test, der Textüberlauf in kittest erkennt, und Korrekturen. ☑ **Teil RAND-1** (`5f2d009`): Prüfer `clipping_audit.rs` + Bericht `rand_auto.md`, Nutzerbeispiel behoben; 70 Stellen in der Erlaubnisliste. ☑ **Teil RAND-2** → **RAND-2a** (`21edb24`) / **RAND-2b** (`247ce0c`, `e1d8909`), Menüs **MN-6** (`a6fd9d3`): Erlaubnisliste leer, 0 Funde: die Pakete R2-1…R2-5 aus `STATUS.md` (RAND-1) abbauen, offen sind noch Menüs und Tooltips (prüft der Prüfer noch nicht) |

### E. Später, niedrige Priorität

| Paket | Inhalt |
|---|---|
| ☐ **P-CI** | GitHub Actions wie bei photocraft (`ci.yml`: fmt, clippy, Tests, i18n-Scanner, fp-neutral; `release.yml`: Artefakte je Betriebssystem) |
| ☐ **P-OS** | Builds für Windows, macOS und Linux: Windows-eigene Teile hinter `cfg` (Startfenster, Datei-Dialoge, Shell-Öffnen, crt-static, Schriften), Prüfbuild für Linux/macOS. Ein GitHub-Repository bzw. Push nur nach Freigabe durch den Nutzer |
| ☐ **P-AU2** → **BATCH-1** | Batch-Assistent wie im Original nach `re\21-batch.md` (RE-BATCH ☑; Nutzer: ja, aber später, nach den Messungen M1–M13) |
| ☐ **P-VT5** | optional: KI-Vektorisieren (StarVector-1B) als Download, nur nach Vergleich auf echten Logos des Nutzers |

### Z. Abschluss (erst wenn A–D erledigt sind)

1. ☐ **Gesamttestlauf (GT-1):** `cargo test --workspace` + alle `funktionstest\*_auto.md` grün, i18n-Scanner 0, fp-neutral grün, technisches Englisch grün.
2. ☐ **Release-Build** als einzelne EXE + CLI, Testversion an den Nutzer.
3. ☐ **Komplettes Durchklicken im echten Fenster (CT-1…CT-4, Fehler beheben: FIX):**
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
- Entschieden 13.10. ≈ 01:05: Rechtschreibung **nur Ordnerwahl** (keine Wörterbücher mitliefern); Bildschirmfoto, Symbolskript erstellen, Gemeinsame Palette **echt umsetzen** (Paket CMD-1 nach MN-5).
- Entschieden 13.10. ≈ 01:13: **Keine neuen Agenten**, die laufenden fertig werden lassen; Batch (BATCH-1) erst später; Testversion vor den Nachttests, Nachttests nur nach ausdrücklichem „los“ des Nutzers; Meldungen nur im Chat.
- Entschieden 13.10. ≈ 01:10: XMP-Schlüssel bleibt **neutral** (`XMP`, nur Dublin Core, keine Ausnahme in fp-neutral); „Bridge durchsuchen“ bleibt **aktiv** als „Im Dateibrowser durchsuchen…“ (öffnet den Ordner des Dokuments im Dateibrowser des Systems).

---

## 7. Zahlen auf einen Blick (Stand nach Welle 2, Aktualisierung nach DOC-3)

| Messgröße | Wert |
|---|---|
| PARITAET_A (Werkzeuge, Inspektor, Leinwand, Zeiger) | 123 ✅ / 1 ⚠️ / 2 ❌ (126 Zeilen) |
| PARITAET_B Hauptmenüs (350 Zeilen) | 276 ✅ · 9 ⚠️ · 4 ❌ · 29 ℹ️ · 32 ⏳ umgesetzt, Nachprüfung ausstehend (vorher 276/20/25/29) |
| PARITAET_B Dialoge (50) | 3 ✅ · 0 ⚠️ · 6 ❌ · 1 ❔ · 40 ⏳ (davon 6 🔧 DLG-3) (vorher 3/21/15, 5 ⏳, 6 🔧) |
| PARITAET_B Bedienfeldmenüs (10) | 1 ✅ · 1 ❔ · 8 ⏳ (vorher 1/4/4/1) |
| Automatischer Gesamttest | Menüs 345/345 · Kontextmenüs 283/283 (FV-13); Bedienfeldmenüs 135 Einträge neu erzeugt (MN-3); übrige Berichte vom Lauf 10:30 |
| BEFUNDE.md | 313 erledigt · 88 offen (DOC-2: 11 abgehakt, 22 neu aus den Berichten) |
| NACHPRUEFUNG.md | 82 Zeilen: 1 ✅ · 81 ⏳ |
| Nutzerbefunde | 18 ✅ · 4 🔧 (N01, N21, N23 teilweise, N24) · 1 ⚠️ (N22) · 1 ❌ teilweise (N11) |
| Fest eingebaute Texte | 0 |
| Umfang | 20 Crates, ≈ 212.000 Zeilen Rust, 177 Testdateien, 235 Commits |
