# DOC-5 – Doku auf den Stand vom 14.10.2026 nachmittags

Grundlage: die Berichte der Nacht und von heute unter `docs\berichte\`, außerdem `docs\parity\TWEAKS_2026-10-13.md`
und `TWEAKS_2026-10-14.md`:
- fixb–fixf, fixg1–fixg5, fixn32, fixn33, fixr1–fixr3
- i18n-check, gt0e–gt0g, run2–run4
- batcht, np1, m1, f1–f7

Stand der Quellen ist `main` @ `489eb72`. Nur Doku-Dateien wurden geändert, kein Code. Der Skript-Nachlauf und F8
liefen parallel, ich habe sie nicht angefasst.

## Geändert

| Datei | Änderung |
|---|---|
| `docs\parity\TWEAKS_2026-10-14.md` | Neuer Abschnitt „Stand 14.10.2026 nachmittags“ mit einer Tabelle für TW-14…32 und Q-11…17 (Stand, Commit, Bericht, Rest). 18 Tweaks und 5 Fragen sind umgesetzt. TW-14 und Q-12 sind ⚪ „kein Produktfehler (Läufer)“, Q-17 ist gleich und brauchte nur ein neues Drehbuch. Dazu der Stand der Läufer-Befunde 1–7 und der offenen Drehbuchfehler. Am Ende steht der leere Abschnitt **„Nachlauf 14.10.“** für die Auswertung. |
| `docs\BEFUNDE.md` | Kopfzeile neu: **535 erledigt, 144 offen** (vorher 520/130 laut Zählung der Kästchen). 2 Einträge abgehakt, beide von BATCH-T belegt: HTML-Bericht `batch-report.html` und Vorlage Video → GIF. 2 Nachträge: `--batch` steht jetzt in der CLI-Hilfe; weitere Produktnamen eines Fremdprogramms rund um PSD (GT-0g, F5). In der berührten Zeile ist der Produktname jetzt ausgelassen. Neuer Abschnitt **F** mit 13 gleich erledigten und 16 offenen Einträgen, siehe unten. |
| `docs\NUTZERBEFUNDE.md` | Neue Stand-Zeile: 29 ✅ · 3 🔧 (N01, N21, N27) · 1 ❌ teilweise (N11). N21 und N27 haben einen Nachweis-Vorsatz: in der Testversion 15-33 enthalten, Prüfung mit echter Maus offen. N32 und N33 sind seit NP-1 ✅, das ist vermerkt. |
| `docs\PLAN.md`, `STATUS.md` | Gleicher Block „Stand 14.10.2026 nachmittags“ jeweils oben (siehe unten). In PLAN verweist die Kopfzeile darauf, die Nutzerbefunde-Spanne heißt jetzt N01–N33. |
| `docs\MANUAL.md` / `MANUAL.en.md` | Inhaltsgleiche Ergänzungen in beiden Sprachen, siehe unten. |
| `docs\AGENTS.md` / `AGENTS.en.md` | Inhaltsgleiche Ergänzungen in beiden Sprachen, siehe unten. |

## Neue offene Befunde (Abschnitt F)

1. Innen einfügen ergibt eine Maskengruppe, das Original einen Pfad mit Vektormaske. Ungemessen ist auch die Auswahl nach „Als Maske einfügen“ (M1, F4).
2. Rahmen „roh“ beim Anfügen an einen Pfad: Pfadanfang und Pfadbreite gegenüber dem Tintenrahmen, Regel unklar (F6, M1).
3. Gamma 1.8 hellt weder das Schachbrett noch die 2-/4-fach-Vorschau auf (F3).
4. HTML-Export mit Schaltflächen-Segmenten ist ungeprüft (F3).
5. Die aktive Fläche aus dem Schaltflächensymbol wird nicht ausgewertet (F3).
6. Unbenannte Objekte liefern `name` = `null` statt `undefined`. Dasselbe gilt für `d.zoom`, `docTitle` und `brush.diameter` (F4, FIX-R2).
7. Filter auf Vektor: Zeitpunkt der Konvertierung und Zahl der Verlaufsschritte im Original ungemessen (F4).
8. TW-26: Name nach Verbinden mit nur einem offenen Pfad ungemessen (F4).
9. Pipette außerhalb des Fensters: begründete Abweichung, der Nutzer entscheidet (F5).
10. Ebenennummern in JSF-Funktionen zählen nur die oberste Stufe. Daran arbeitet gerade F8 (F6).
11. SVG-Text liegt immer über den Pfaden (F2).
12. Inspektor eines maskierten Bilds und Freigabe der Pfadknöpfe: im Original ungemessen (FIX-G5, FIX-R1).
13. Raster bearbeiten: Popup-Schieber im Original ungemessen (FIX-G1).
14. Ob die HTML-Exportart dokumentübergreifend gemerkt wird, ist ungemessen (FIX-G2).
15. Wackelnde Tests: `dialogs_ux::width_field_limited_to_6000_with_message` und `batch_ui::trockenlauf_und_lauf_zweimal`.
16. Runner-Lücken, gesammelt aus RUN-2/3/4 und A3-2.

## Handbuch und AGENTS: neu seit dem I18N-Check

**Handbuch:**
- Assistent „HTML-Code kopieren“
- Popupmenü-Editor mit Weiter/Zurück
- „Durchsuchen...“ neben „Hintergrundbild:“
- PSD-Formebenen und Gruppen, AI-/SVG-Text
- Unbenannt-1.fw.png
- Gamma 1.8 (Macintosh-Gamma)
- Schaltflächen-Segmente, Bewegungspfad
- Esc hebt die Auswahl auf
- KI-Auswahl sichtbar, mit Feineinstellungen auch bei Zeiger/Auswahlwerkzeugen
- Rückfrage Vektor → Bitmap
- Neu nach Strg+C, Ähnliches auswählen
- Neu von Vorlage (10 Vorlagen)
- Farbwähler-Pipette
- Batch: Abbruch-Karte und Plan mit „≈“-Größen
- Von Pfad lösen
- `dom.layers` flach

**AGENTS:**
- `text.detachFromPath`, `select.similar`, `edit.pasteInside` (danach nichts gewählt), `modify.canvas.trimCanvas`, `edit.copyHtmlCode {baseName, xhtml}`, `file.newFromTemplate`
- Konventionen: Dokumentname, Rückfrage bei Filtern (`DisableConfirmFilterVectors`), Esc
- JSF: `dom.layers`/`topLayers`/`elemsandsublayers`, `pixelRect` von Text bzw. mit Vektormaske, `guides.locked`, Hinweis auf die Ebenennummern (F8)
- `ui_state`: `visible_panels`, `panels_hidden`, `batch_front`
- `hover_files` bis `hover_end`

Die Beschriftungen stammen aus den ftl-Dateien (de/en).

## Stand für PLAN/STATUS (Kurzfassung)

**Fertig:**
- Welle 5
- Batch-Studio einschließlich BATCH-T und F5
- Vergleichsläufe mit TWEAKS 13.10. und 14.10.
- Fixes F1–F7, F8 läuft
- Klicktests der Nacht mit FIX-B…F
- DE/EN vollständig (I18N-CHECK, 6633/6633 Schlüssel)
- privates GitHub-Repository als `origin` (Push nur auf Anweisung)
- Testversion 2026-10-14_15-33

**Offen:**
- GT-2/Endversion: Der Nutzer entscheidet.
- Prüfung mit echter Maus für N01/N21/N27.
- Später: CI, CMD-1, DES-3, BATCH-10, Runner-Lücken.
- Restliste BEFUNDE nach Art: IBD/UX/DESIGN 29, Batch 18, HG 17, Leinwand/Werkzeuge 21, Engine/Export/Render 22, Abschnitt F 16, AU/TE/I18N/NT/VT 21. Davon sind etwa 40 reine Nachmessungen im Original.

## Prüfung

- `cargo test -p fp-neutral` (`CARGO_TARGET_DIR=…\target-s2`): grün, alle 6 Testprogramme.
- Neutralität von Hand geprüft: In den geänderten Zeilen steht kein Name des Originals, des Herstellers oder eines Herstellerprodukts. Die Formatkennung PSD bleibt.
