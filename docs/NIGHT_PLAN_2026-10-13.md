# Nachtplan 13.10.2026 – durchgehend ohne Pausen

Nutzerentscheidung 13.10. ≈ 03:35: ein einziges „los“, danach läuft alles ohne Zwischenstopp durch. Meldungen nur im Chat, kein Push.
T0 = Zeitpunkt des „los“ (geplant ≈ 03:45). Höchstens **8 Agenten gleichzeitig**, alle Opus, je Paket ein frischer Agent.
Messungen im Original nur unter der Sperre (immer nur ein Agent am Original). Zeiten sind Schätzungen.

Lizenz seit 13.10.: **Apache-2.0** (`LICENSE`, `Cargo.toml`). Keine GPL/AGPL/LGPL-Bibliotheken einbinden.

## Block 1 – Welle 5a + Batch-Kern (T0 → ≈ T+1:30)

| # | Paket | Slot | Start → Ende | Inhalt |
|---|---|---|---|---|
| 1 | TE2-G | s1 | läuft → T+0:15 | Reste von TE-2, Befunde, PLAN §7 Zahlen |
| 2 | RE-5 | Original | T0 → T+0:45 | Messen: Bitmap-Grenzen (zuerst, für BM-1), Farbwähler-Popup (für FARB-1b), 2-/4-fach synchron (für BV-5) → `re\22-ansicht-bitmap-farbe.md` |
| 3 | BV-5 | s2 | T0 → T+0:45 | N25: gemeinsame Ansichtslage in 2-/4-fach (Bildlauf, Rad, Leertaste, Hand, Zoom) |
| 4 | BM-1 | s3 | T0 → T+0:50 | N26: Bitmap-Grenzen beim FW-PNG-Laden/Speichern, nach RE-5 angleichen |
| 5 | FARB-1a | s4 | T0 → T+0:20 | N27a: Farbe springt beim Ziehen auf Grau zurück |
| 6 | CMD-TXT | s4 | T+0:20 → T+0:35 | roter Test: 19 Befehlstexte fehlen in `command_texts.ftl` (de/en) |
| 7 | VT-6 | s5 | T0 → T+1:20 | N29: zweites Nachzeichnen-Verfahren (Potrace-Verfahren eigenständig nach Fachartikel), Auswahl im Dialog |
| 8 | START-1 | s6 | T0 → T+0:45 | N30: Startzeit messen und verkürzen, Zahlen vorher/nachher |
| 9 | BATCH-1 | s8 | T+0:15 → T+1:15 | Batch-Kern: Crate `fp-batch`, Pipeline, Rezept-JSON, Thread-Pool, Abbrechen |
| 9a | RE-PINSEL | Original | T+0:45 → T+1:15 | N31: Pinsel im Original messen – Strich live, Strich-Kategorien/-Arten, Kante, Textur, Deckkraft, Mischmodi, Transparenz beibehalten, Dialog „Pinselstrich bearbeiten“ → `re\24-pinsel.md` |
| 9b | PINSEL-1 | s11 | T+0:45 → T+2:15 | N31: Pinsel live malen (kein Stempel-Umriss), danach alle Inspektor-Werte wirksam und „Pinselstrich bearbeiten“ nach `re\24` |

## Block 2 – Welle 5b + Batch-Gerüst (≈ T+0:45 → T+3:25)

| # | Paket | Slot | Start → Ende | Inhalt |
|---|---|---|---|---|
| 10 | RE-FX | Original | T+1:15 → T+2:35 | Messreihe aller Live-Effekte und Filter (je 3 Einstellungen, 3 Testbilder, PNG 32), **in Gruppen geliefert**: Schatten/Glühen → Abschrägung/Prägen → Weichzeichner/Schärfen → Rest → `re\23-effekte.md` |
| 11 | FARB-1b | s4 | T+0:50 → T+1:50 | N27b: Farbwähler wie im Original an allen Stellen (Raster, HEX/RGB, Pipette im Popup, Systemfarbwähler) |
| 12 | FX-1 | s7 | ab Gruppe 1 (≈ T+1:40) → T+2:30 | N28: Schatten/Glühen pixelgleich (Glühen zuerst) |
| 13 | FX-2 | s2 | ab Gruppe 2 (≈ T+2:00) → T+2:50 | Abschrägung/Prägen |
| 14 | FX-3 | s3 | ab Gruppe 3 (≈ T+2:20) → T+3:10 | Weichzeichner/Schärfen |
| 15 | FX-4 | s6 | ab Gruppe 4 (≈ T+2:35) → T+3:25 | Farbe/Rauschen/übrige Filter |
| 16 | BATCH-2 | s8 | T+1:15 → T+2:15 | Batch-UI-Gerüst im Stil des Startbildschirms (Reiter, Dateiliste, Rezept-Karten, Vorschau, Ziel/Start) |
| 17 | BATCH-3 | s9 | T+2:15 → T+3:05 | Umbenennen: Anfänger-Baukasten + Regex mit Klartext und Tabelle alt → neu |
| 18 | BATCH-4 | s10 | T+2:15 → T+3:15 | Schritte Geometrie/Farbe/Filter/Live-Effekte |

## Block 3 – Abschluss Welle 5, Testversion (≈ T+3:25 → T+3:50)

| # | Paket | Slot | Start → Ende | Inhalt |
|---|---|---|---|---|
| 19 | DOC-4 | – | T+3:25 → T+3:40 | Berichte der Welle 5 in NUTZERBEFUNDE/BEFUNDE/PARITAET/NACHPRUEFUNG/PLAN |
| 20 | GT-0 | s1 + fp-rel | T+3:25 → T+3:50 | warme Slots s1–s7 auf den neuen Stand vorwärmen, Release-Build, Testversion `Firepixel-Vorschau-<Datum>_<Zeit>` auf den Desktop (mit NEU.md) – diese exe testen CT-1…4 |

## Block 4 – Nachttests + Batch-Spur (≈ T+3:50 → T+5:50)

| # | Paket | Slot | Start → Ende | Inhalt |
|---|---|---|---|---|
| 21 | GT-1 | s1 | T+3:50 → T+4:35 | alle Tests, Funktionsberichte, Abschneide-Prüflauf, Neutralität inkl. Bildtext, technisches Englisch, Scanner |
| 22 | CT-1a | Brücke | T+4:05 → T+5:05 | Dialoge im echten Fenster (NACHPRUEFUNG §3) |
| 23 | CT-1b | Brücke | T+4:05 → T+5:05 | Menüs (§7) |
| 24 | CT-2 | Brücke | T+4:05 → T+5:05 | Werkzeuge, Leinwand, Inspektor, KI (§1, §2, §6) – inkl. N25–N31 |
| 25 | CT-3 | Brücke | T+4:05 → T+5:05 | Bedienfelder, Docking, Datei, Bildvorschau/Export, Startseite (§4, §5) |
| 26 | ~~CT-4~~ → PAR-O (Original) + GT-2 (Englisch/Designs/Fenstergrößen) | – | T+4:05 → T+5:05 | Englisch, 3 Designs, Fenstergrößen, alle Messungen im Original |
| 27 | BATCH-5 | s9 | T+3:15 → T+4:15 | Metadaten (behalten/entfernen/setzen), Optimieren, Zielgröße in KB |
| 28 | BATCH-6 | s10 | T+3:15 → T+4:15 | Hintergrund entfernen (KI), Transparenz → Farbe, Wasserzeichen/Text/Rahmen |
| 29 | FIX-1…4 | s2–s7 | T+5:05 → T+5:50 | jedes ❌ aus GT-1/CT-1…4 und die Fix-Liste (BEFUNDE „Fix-Liste 1–7“) beheben und erneut prüfen |
| 30 | BATCH-7 | s9 | T+4:15 → T+5:15 | Animation kürzen/Tempo/Schleife mit Vorschau, Video → GIF/APNG/WebP über ein **installiertes** ffmpeg (wird nicht heruntergeladen; fehlt es, werden diese Tests mit Vermerk übersprungen) |
| 31 | BATCH-8 | s10 | T+4:15 → T+5:15 | GIF-Optimierer: kleine GIFs in hoher Qualität (eigene Verfahren, ohne GPL-Bibliotheken) |

## Block 5 – Batch fertig bauen, testen, Gesamtlauf (≈ T+5:15 → T+8:20)

| # | Paket | Slot | Start → Ende | Inhalt |
|---|---|---|---|---|
| 32 | BATCH-9 | s9 | T+5:15 → T+6:00 | Nachzeichnen → SVG als Schritt, Rezept-Vorlagen, Befehle des Registers als Schritt, CLI `batch --recipe`, MCP |
| 33 | BATCH-11 | s10 | T+6:00 → T+6:50 | Feinschliff: 3 Designs, DE/EN, Abschneide-Prüfer, Tastatur, Fehlerliste/Bericht |
| 34 | BATCH-T | s1 | T+6:50 → T+7:40 | Batch testen (erst jetzt, nachdem alle UI-Tests und FIX vorbei sind): automatische Tests, Logikfälle, Bedienung im echten Fenster über die Brücke; Fehler direkt beheben |
| 35 | GT-1b | s1 + fp-rel | T+7:40 → T+8:20 | Zwischen-Gesamtlauf inkl. Batch, **Zwischenversion auf den Desktop** (`Firepixel-Vorschau-<Datum>_<Zeit>` mit NEU.md), Zwischenbericht im Chat |

## Spur PAR – Vollabgleich Original ↔ Firepixel (Nutzerwunsch 13.10. ≈ 03:45, Verfahren `docs\PARITY_SWEEP.md`)

Läuft **neben** den Blöcken oben, als Fließband: Original aufnehmen (immer nur ein Agent, Sperre) → Firepixel vergleichen → beheben → gegenprüfen. Jeder Agent hat eine kleine Einheit (≤ 25 Prüfpunkte, ≤ 12 angesehene Bilder je Programm).

| # | Paket | Start → Ende | Inhalt |
|---|---|---|---|
| P0 | PAR-0 | T0 → T+0:40 | Inventar: alle Befehle, Werkzeuge, Inspektor-Felder, Bedienfelder, Filter, Dialoge, Kürzel → ≈ 50 Einheiten mit Prüfpunkten und Reihenfolge (`docs\parity\INVENTORY.md`) |
| P1 | PAR-O-1…50 | T+2:35 → ≈ T+13:00 | Original aufnehmen, eine Einheit nach der anderen, **lückenlos** nach RE-FX (die Messungen von CT-4 im Original laufen hier mit) |
| P2 | PAR-F-n | je ≈ 15 min nach der Aufnahme | Firepixel vergleichen, Befunde eintragen (bis zu 2 gleichzeitig während der Nachttests, danach bis zu 3) |
| P3 | PAR-X-<bereich> | ab T+5:50 (nach FIX) laufend | Unterschiede beheben, je Bereich ein Agent (Werkzeuge, Inspektor, Effekte, Bedienfelder, Menüs/Dialoge, Querschnitt) |
| P4 | PAR-R-n | nach jedem PAR-X | gegenprüfen (nur Firepixel, gegen die Aufnahme) |

Höchstgrenze bleibt 8 Agenten: CT-4 entfällt als eigenes Paket (Englisch/Designs/Fenstergrößen → GT-2, Messungen im Original → PAR-O); PAR-F/X/R füllen jeweils die freien Plätze.

## Schluss (≈ T+13:00 → T+14:30)

| # | Paket | Inhalt |
|---|---|---|
| 36 | PAR-X/R Rest | letzte Unterschiede beheben und gegenprüfen |
| 37 | DOC-5 | alle PAR-Ergebnisse in PARITAET/BEFUNDE/NUTZERBEFUNDE/NACHPRUEFUNG, Zählung |
| 38 | GT-2 | letzter Gesamtlauf inkl. Batch und PAR-Fixes, Englisch, 3 Designs, Fenstergrößen, Release-Build, **Endversion auf den Desktop**, Abschlussbericht im Chat |

**Ende ≈ T+14:30** (bei „los“ um 03:50 ≈ 18:20). Zwischenversion mit Welle 5 + Batch ≈ T+8:20 (≈ 12:10). Nicht dabei: BATCH-10 (Video eingebaut, P2), CMD-1, DES-3, CI.

## Annahmen (Empfehlungen, gelten ohne Gegenbescheid)

- Der neue Batch ersetzt den alten Batch-Assistenten im Menü; Reiter im Hauptfenster; Vorgabe-Ziel `batch-output`, Originale nur auf ausdrücklichen Wunsch überschreiben.
- H.264 nicht eingebaut (nur über ffmpeg); `qcms` (MPL) nicht zulassen; KI-Hochskalieren später (P3).
- Nichts wird heruntergeladen oder installiert; keine Systemeinstellungen; keine Prozesse des Nutzers beenden.
- Läuft ein Paket fest oder schlägt fehl: einmal mit frischem Agenten wiederholen, sonst als Befund vermerken und weitermachen.
- Der Nutzer hält die RDP-Sitzung offen (Mac bleibt an), damit das Original sichtbar zeichnet. Ist der Bildschirm trotzdem nicht erreichbar (schwarze Bildschirmfotos): Original-Aufnahmen pausieren, nur Firepixel-Seite (Brücke) weiter, im Chat melden.
