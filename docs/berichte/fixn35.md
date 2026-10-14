# FIX-N35 – Innen einfügen wie im Original (15.10.2026)

Nutzerbefund **N35**: Nach *Bearbeiten ▸ Innen einfügen* (Strg+Umschalt+V) verschwand in Firepixel die Füllung des
Zielrechtecks, nur der eingefügte Inhalt war zu sehen. Umgesetzt **strikt nach der Spezifikation**
`re\26-masken.md` (RE-MASKE, 36/38 Messseiten pixelgenau). Ich habe nur das Dokument benutzt, keinen Code des Originals.

## Ursache

`fp-render` (`compose.rs`, `group_layer`) zeichnete bei einer Maskengruppe nur die Kinder × Maskenwert. Füllung und
Strich des Maskenpfads wurden nie gezeichnet, auch nicht bei `showAttrs` (= ohne `MNAb`). F9 hatte `showAttrs` für
*Innen einfügen* schon gesetzt, der Renderer wertete es aber nicht aus. Außerdem war die Deckung vertauscht: Ohne
„anzeigen“ wurde das gerenderte Alpha mit Strich genommen, mit „anzeigen“ die Fülldeckung.

## Umgesetzt (Commit `c1f7fff`)

| Punkt | Regel (26-masken.md) | Datei |
|---|---|---|
| Rendern mit „Füllung und Pinselstrich anzeigen“ (Pfadumriss + `!MNAb`) | Füllung der Maske, darüber der Inhalt mit α·cov (Geometrie mit Kantenglättung, **ohne** weiche Kante, Rundung (a·m+127)/255, `over` skalar), darüber der Strich der Maske (`over` MMX). Danach Effekte, Deckkraft und Mischmodus des maskierten Elements. Deckkraft und Mischmodus der Maske werden ignoriert. Bei Nicht-Pfad-Masken (Bitmap, Gruppe) wird das ganze Maskenelement unten gezeichnet | `crates\fp-render\src\compose.rs` (`group_layer`, `mask_attr_layers`, `Canvas::draw_variant`) |
| Rendern ohne „anzeigen“ (`MNAb`) | Inhalt × **Fülldeckung inkl. weicher Kante**. Farbe, Farbalpha und Strich der Maske zählen nicht. Graustufen (255−Grau) gibt es nur bei Bitmapmasken. Ein Pfadumriss mit Bitmap als Maskenelement nimmt das Alpha | `compose.rs` (`mask_values`) |
| `edit.pasteInside` | Ziel wird Maske: Pfadumriss auch bei Bitmap- und Gruppenzielen, verknüpft, aktiv, anzeigen (Flagbyte 0x17, kein `MNAb`). Ein Inhalt ohne eigene Effekte und ohne eigene Maske ergibt ein maskiertes Einzelelement; Deckkraft und Mischmodus des Inhalts wandern an das Element (D2). Sonst entsteht eine Gruppe mit Maske, Kinder in Zwischenablage-Reihenfolge. Effekte des Ziels wandern auf das Ergebnis (I). Ist das Ziel schon maskiert, bleibt dessen Maske und der neue Inhalt kommt oben dazu (N). Auswahl danach: Einzelelement leer, Gruppe ausgewählt. Ein Rückgängig-Schritt | `crates\fp-engine\src\commands\edit.rs` (`paste_inside_result`) |
| Einzelelement-Regel | `masked_single` richtet sich nach den Effekten des **Kindes**, nicht nach denen der Gruppe (Rundreise rt_I/rt_J) | `crates\fp-model\src\element.rs` |
| Treffertest | Bei „anzeigen“ trifft auch ein Klick auf die sichtbare Maskenfüllung | `crates\fp-engine\src\docops.rs` (`hit_depth`) |
| Ebenen-Bedienfeld | Kettensymbol zwischen Miniatur und Maskenminiatur, wenn die Maske verknüpft ist. Vektormasken-Miniatur: Form weiß auf Schwarz (wie Nutzerbild 15; Spezifikation offene Frage 8) | `crates\fp-ui\src\panels\layers.rs`, `thumbs.rs` |
| Doku | `docs\AGENTS.md` / `AGENTS.en.md`: Ergebnis und Auswahl von `edit.pasteInside` | – |

Der Datei-Schreiber war schon richtig und blieb unverändert: `GRPv{GRTi 2}` ohne `MNAb` bei *Innen einfügen*.
Speichern und Laden erhalten Struktur, Flags und Bild.

## Tests

- neu `crates\fp-engine\tests\fixn35.rs`, 14 Tests mit den Messfällen der Spezifikation:
  - A: Pixel Rand = Füllfarbe, Mitte = grün, Strich oben, Bild wie vorher
  - Beschnitt an der Form, Strich über dem Inhalt
  - Struktur: ein Objekt, verknüpft/aktiv/anzeigen, nichts ausgewählt
  - Rückgängig ein Schritt + Wiederholen
  - Speichern/Laden
  - D2: Deckkraft des Inhalts wirkt aufs Ganze
  - D: Deckkraft der Maske wird ignoriert
  - I: Effekt des Ziels wandert und wirkt aufs Ganze
  - J: Inhalt mit Effekt → Gruppe, ausgewählt
  - E: 2 Inhalte → Gruppe, ausgewählt, Beschnitt
  - G: Bitmap-Ziel bleibt Pfadumriss
  - N: schon maskiertes Ziel
  - Treffer auf der Maskenfüllung
  - Als Maske einfügen (ohne „anzeigen“): nur der Inhalt
- `cargo test -p fp-engine --test all`: 355 ok. `fp-render`, `fp-model`, `fp-fwpng`, `fp-export` (Golden): grün.
  `fp-ui --lib mask`: grün. `fp-script`: grün, auch `tw_f4`/`tw_f9` (Q-14, TW-35), bis auf
  `b7_messwerte::ba020_anim_instance_bounds_include_motion_path`. Das ist Animationssymbol-Grenzen, ohne Maske, und
  hängt nicht mit dieser Änderung zusammen. Es sollte gesondert geprüft werden.
- `cargo check --workspace --all-targets`: ohne Warnungen.
- **Echtes Fenster, Brücke** (Slot-Build `target-s3\debug\firepixel.exe`, eigenes Profil): großes graues Rechteck
  mit schwarzem Strich, grünes darüber. Grünes **angeklickt**, Strg+X, großes angeklickt, Strg+Umschalt+V, alles über
  `ui_input`. Ergebnis: Grau bleibt sichtbar, grün darin, ein Objekt „Rechteck“ mit Miniatur, Kette und
  Maskenminiatur. JSF `mask to path/linked/enabled/showAttrs = true`, Auswahl 0. Strg+Z → nur das Rechteck,
  Strg+Y → wieder Maske. Gespeichert und geladen: gleiches Bild, `pixelRect` 100,60,540,340 (Maske).
  Bilder: `re\_testdateien\par\fixn35\n35_1_vorher.png`, `n35_2_ausgeschnitten.png`, `n35_3_nachher.png`,
  `n35_4_rueckgaengig.png`, `n35_5_geladen.png`, `n35_6_ebenen.png`.

## Erledigte Befunde

- NUTZERBEFUNDE **N35** „Innen einfügen (Strg+Umschalt+V) sieht anders aus …“: 🔧 umgesetzt (`c1f7fff`).
- F9 Neuer Befund 1 („Der Renderer zeichnet die Maskenattribute bei `showAttrs` noch nicht sichtbar mit“): erledigt.
- F4 Hinweis Q-14 Struktur: Firepixel bleibt intern bei der Maskengruppe. Die Einzelelement-Sicht (`masked_single`)
  folgt jetzt der Leserregel des Originals.

## PARITAET: „umgesetzt, Nachprüfung ausstehend“

- Bearbeiten ▸ Innen einfügen (Darstellung, Struktur, Auswahl, Effekte des Ziels).
- Modifizieren ▸ Maske: Darstellung von Vektormasken mit und ohne „Füllung und Pinselstrich anzeigen“.
- Ebenen-Bedienfeld: Kettensymbol und Maskenminiatur.

## Nachprüfung

| Ablauf | Erwartet |
|---|---|
| Läufer `menu-edit` 05 (Innen einfügen) | wie Original: `pixelRect` = R1, Bild mit Füllung von R1 |
| Messdateien `re\masken\_scratch\jsf\pi_*.fw.png` in Firepixel öffnen und flach vergleichen | wie 26 §6.1 (36/38 exakt, Reste nur Strich „Soft Rounded“) |
| Korpus Start Screen, Home, Camcorder, Images (Maskengruppen) | wie 26 §6.2 |
| Ebenen-Bedienfeld neben dem Original (Bild 15) | Kette und Maskenminiatur an gleicher Stelle, Maskenminiatur weiß auf Schwarz |

## Offen / neue Befunde

1. **Deckkraft/Mischmodus beim Schreiben** (26 §4): Das Original schreibt `OPAi`/`BLDi` eines maskierten
   Einzelelements an Gruppe **und** Kind. Firepixel setzt sie bei *Innen einfügen* nur an die Gruppe (Kind 100 %).
   Das Bild stimmt. Für eine bytegleiche Rundreise müsste der Schreiber das Kind mit ausgeben, und der Leser die
   Kopie am Kind verwerfen (bei Dateien des Originals würde Firepixel die Deckkraft sonst doppelt anwenden).
2. `pixelRect` mit „anzeigen“: Der Effektrand ist nicht eingerechnet (26 §2.2, Fall I: 50,50–158,128).
3. *Als Maske einfügen*: Laut 26 §2.3 zeigt die Auswahl danach auf das **Maskenelement**. In Firepixel ist die
   Maskengruppe ausgewählt. Das ist unverändert.
4. Offene Fragen der Spezifikation (Text als Ziel, mehrere Ziele, `FOTb`, deaktivierte Maske mit „anzeigen“) sind
   so umgesetzt, wie A dort annimmt. Bei deaktivierter Maske gibt es keine Füllung und keinen Beschnitt.
5. `fp-script` `b7_messwerte::ba020` schlägt auf dem Slot fehl, ohne Bezug zu Masken (siehe Tests).
