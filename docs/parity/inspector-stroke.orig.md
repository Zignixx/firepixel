# 19 `inspector-stroke` – Strich im Inspektor: Aufnahme des Originals (PAR-O-14-15-19, 13.10.2026 ≈ 13:15–14:20)

T04 als Arbeitskopie `re\_testdateien\par\work\p1415_t04.fw.png` (T05 für 12), je Messung frisch mit `fw.openDocument(url)`. Fenster 1600 × 1000,
Zoom 100 %. Felder mit **echter Tastatur/Maus** (Klick, Pos1 + Umschalt+Ende, Eingabe, Enter), Werte danach per Skript (`pathAttributes.brush`,
`brushPlacement`, `fillOnTop`, `pixelRect`), Verlaufsnamen aus dem Bearbeiten-Menü. Ergebnis-Exporte (PNG 32 bit, ganzes Dokument):
`re\_testdateien\par\px\inspector-stroke\` + `pixels.csv` (Hash, geänderte Pixel gegenüber `b04_base`/`b05_base`). Bilder: `re\_testdateien\par\ref\inspector-stroke\` (+ `INDEX.md`).
Art: M = echte Eingabe, J = Skriptwert, W = Fenstertext/Zustand, E = Export/Hash, B = Bild. Vieles ergänzt Einheit 02 (`tool-brush-stroke-options.orig.md`).

## Prüfpunkte

| Nr | Kürzel | Original | Art |
|---|---|---|---|
| 01 | `category` | Liste (Tooltip „Pinselstrichkategorien“): **Ohne** · Trenner · Einfach ▸, Airbrush ▸, Bleistift ▸, Filzstift ▸, Gestrichelt ▸, Kalligrafie ▸, Kohlestift ▸, Pastellkreide ▸, Wasserfarbe ▸, Öl ▸, Unnatürlich ▸, Unregelmäßig ▸ · Trenner · **Pinselstrichoptionen…** (Untermenüs owner-drawn). Tastatur geht: ↓↓↓ = Airbrush, → öffnet Untermenü (erste Art markiert), Enter wählt. Danach zeigt das Feld nur den **Artnamen** („Einfach“), Kategorie wechselt auf `bc_Air Brush`/`bn_Basic`, Größe bleibt 1. Per Skript gesetzte Pinsel zeigen den **internen Namen** (T04: „bn_1-Pixel Hard“) | M, J, W, B `01` (2) |
| 02 | `type_result` | Grundart je Kategorie auf P1, Größe 10 (Kalligrafie/Pastellkreide/Gestrichelt: Art „Einfach“ bzw. „Einfach gestr.“, sonst erste Art). Grenzen: meist 195,165–425,245 (P1 ± 5); Airbrush 195,165–424,243; Kalligrafie 196,165–424,245; Öl 192,162–428,247; Unnatürlich (3D) 194,164–426,246; Unregelmäßig (Garn) 191,161–431,251. „1-Pixel hart“ (Bleistift) = „Präzise abger.“ **pixelgleich**. Zufallsarten sind **reproduzierbar** (Garn zweimal gleicher Hash) | J, E, B `02` (12) |
| 03 | `width` | Feld Größe (ohne Tooltip, ▼ daneben): Eingabe 0 → **1**, 2000 → **100**, „abc“ → bleibt; Feldgrenzen **1–100**. Per Skript sind 1–1000 erlaubt (0 und 2000 → Fehler −2147221503). Breite 1/10/100 → Grenzen 200,170–421,241 / 195,165–425,245 / 150,120–470,290 (Strich mittig, Hälfte je Seite) | M, J, E, B `03` (3) |
| 04 | `edge` | Feld Rand (Vorschau links mit Tooltip „Pinselvorschau“, ▼ rechts): 50 → `softness` 50; **150 → 0** (ungültig → 0, keine Meldung). Export 0/50/100: weiche Kante, 2/147/171 Farben im Strich | M, J, E, B `04` (3) |
| 05 | `texture` | Liste Textur „Ader“ (Skript: `Vein`), Mengenfeld zeigt **„50%“**; 150 → **„0%“**. Wirkung bei 10 px: Menge 50 mittlere Abw. 185,5, Menge 100 143,1 (statt 191 ohne Textur) | M, J, E, B `05` |
| 06 | `position` | Drei Knöpfe unter dem Farbfeld, Tooltips „Pinselstrich innen ausrichten“, „…mittig…“, „…außen ausrichten“; Vorgabe mittig (gedrückt). PG mit 10 px: innen 465,165–535,245 (= Pfad), mittig 460,160–540,250, außen 455,155–545,255. Text bekommt beim Strichsetzen **außen** | M, J, E, B `06` (4) |
| 07 | `options_popup` | Listeneintrag „Pinselstrichoptionen…“ öffnet ein **Popup** 257 × 240: Kategorie-Liste (13: Ohne, Einfach … Unregelmäßig), Art-Liste, Spitzenvorschau, „Spitze:“ Feld + senkrechter Schieber (0–1000), „Textur:“ Liste + %-Feld ▼, Liste **Pinselstrich innen / zentriert / außen**, Kontrollkästchen **„Füllung über Pinselstrich“**, Knopf **„Erweitert…“** (öffnet „Pinselstrich bearbeiten“ über dem Popup), 2 Symbolknöpfe (speichern/löschen). Esc schloss das Popup nicht; **Klick daneben übernimmt** | M, W, B `07` (2) |
| 08 | `edit_brush` | Knopf „Pinselstrich bearbeiten“ → Dialog wie Einheit 02 (344 × 525, Register Optionen/Form/Empfindlichkeit, OK/„Cancel“). Werte bei Airbrush/Einfach: Tintenmenge 90 %, Abstand 22 %, Farbfluss 80, Übermalen ✓, Textur 50 %, Randtextur 0 %, Randeffekt Ohne, Spitzen 1, Spitzenabstand 0 % (grau), Variation Zufällig (grau), Gestrichelt Ohne, Ein/Aus 10/4, 2/4, 2/4 (grau). Esc schließt | M, W, B `08` |
| 09 | `dash` | 6 Arten (Größe 4): Dick gestrichelt (1 Strich), Doppelt (2), Dreifach (3), Einfach gestr. (1), Gepunktet (`natural`, 0), Präzise gestr. (1, Rand 0 → nur 3 Farben, harte Kanten). Hashes `09_dash_*` | J, E, B `09` (6) |
| 10 | `color` | Farbfeld (Tooltip = Hex „#C04040“) → Popup 259 × 277: Farbfelder, Vorschau, Deckkraft 100 ▼, HEX-Feld, RGB 3 Felder, 2 Kopierknöpfe; untere Leiste: **Kein Strich**, 3 Positionsknöpfe, Systemfarbwähler. Kein „Strichoptionen“-Knopf | M, W, B `10` |
| 11 | `none` | „Ohne“ in der Liste → `brush = null`, Strich verschwindet (Export), Pixelgrenzen ungültig (−2147483648…); Schritt „Objekteigenschaften“ | M, J, E |
| 12 | `text_stroke` | Text-Inspektor hat **keinen** Strichbereich. Strich per Skript (2 px) an TX2 → Platzierung außen, Pixel 48,148–108,196 (Text 50,150–107,194) | J, E, B `12` (2) |
| 13 | `open_closed` | P1 offen vs. geschlossen (Kontur `isClosed`): geschlossen zeichnet das Schlusssegment mit (geänderte Pixel 7759 → 9536) | J, E, B `13` (2) |
| 14 | `fill_over` | Nur im Popup (07) bedienbar: Kontrollkästchen gesetzt + Klick daneben → `fillOnTop` true; PG 20 px mittig: Füllung deckt die innere Strichhälfte (geänderte Pixel 4844 → 2904). JSF `setFillOnTop` gibt es nicht | M, J, E, B `14` (3) |
| 15 | `scale` | `scaleSelection(2, 2, "autoTrimImages transformAttributes")` → Strich 10 → **20**; nur `"autoTrimImages"` → bleibt 10; ohne Flags → Fehler −2147216337 | J |
| 16 | `save_stroke` | Symbolknopf (Tooltip „Benutzerdefinierten Pinselstrich speichern“) → Dialog **„Pinselstrich speichern“** 285 × 110: „Name:“ + leeres Feld, OK, Abbrechen; Esc schließt. Nachbarknopf „Benutzerdefinierten Pinselstrich löschen“. (In Einheit 02 noch interner Fehler – jetzt bedienbar) | M, W, B `16` |
| 17 | `custom` | Nicht neu gemessen: nach Änderung im Dialog **kein** Kennzeichen in der Liste (Einheit 02, Nr. 16) | – |
| 18 | `disabled` | Bei „Ohne“ grau: Größe-Feld + ▼, Rand-Feld + ▼, „Textur:“, Texturliste, Menge + ▼, „Pinselstrich bearbeiten“, speichern, löschen. Aktiv bleiben: Farbfeld, Kategorienliste („Ohne“), 3 Positionsknöpfe, Randvorschau | M, W, B `18` |
| 19 | `undo` | Je Änderung **ein** Schritt: Kategorie/Art, Größe, Rand, Position, „Ohne“ → **„Objekteigenschaften“**; Texturmenge → **„Pinselstrich“** (Strg+Z stellt 50 wieder her). Skriptänderungen → „Befehlsskript“ | M, J |
| 20 | `tool_default` | Stift/Linie ohne Auswahl: Strichbereich mit Farbfeld, Größe 1, „Präzise abger.“, Rand 0, Textur Ader 0, „Pinselstrich bearbeiten“, speichern/löschen – **ohne** die 3 Positionsknöpfe; Füllbereich (Kante, Textur, Transparent) grau, „Kombinieren“ grau | M, W, B `20` |

## Hinweise

- Inspektorfelder übernehmen erst mit Enter oder Fokusverlust; per Nachricht gesetzter Text ohne echtes Enter wurde teils nicht übernommen.
- Strichpopup (07) und Farbpopup (10) sind eigene Fenster (`#32770`), keine Menüs.
