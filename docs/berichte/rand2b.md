# RAND-2b – Prüfer für abgeschnittene Texte, Pakete R2-3 bis R2-5, NZ-2-Fund, Menüs und Tooltips (13.10.2026)

Slot s6, das Original nicht benutzt. Auf main nach RAND-2a (`21edb24`, `20be741`) zusammengeführt;
die Regel von RAND-2a für den waagerechten Inspektor-Bildlauf (`inspector::hscroll_viewport`) bleibt.

## Umgesetzt

- **R2-3: Dialoge, die höher oder breiter als das Fenster sind** (`dialogs\mod.rs`, kleiner, in sich geschlossener Edit in `dialog_frame_ex`)
  - Ist die Client-Größe größer als das Hauptfenster, wird die sichtbare Größe darauf begrenzt.
  - Der Inhalt behält seine volle Größe und läuft in einer `ScrollArea::both` (neue Funktion `scrolled_body`).
  - Ein Streifen von 48 px unten (`DIALOG_PIN_STRIP`) bleibt fest stehen. `push_button` verschiebt Knöpfe, deren Oberkante im unteren Streifen des Inhalts liegt, in diesen festen Streifen. Rechte Knöpfe bleiben dabei am rechten Rand, linke am linken („Hilfe“). Klicks, Eingabe als Standardknopf und Fokus bleiben unverändert.
  - Dialoge, die passen, sind unverändert (kein zusätzlicher Code-Pfad).
  - Betrifft Voreinstellungen (alle 7 Seiten), Tastaturbefehle und KI-Modelle verwalten bei 800 × 512 pt.
- **R2-4** (`dialogs\models.rs`): „Entfernen…/Remove…“ ist jetzt `frame(true).frame_when_inactive(false)`.
  - Der Knopf hat damit den Innenabstand eines Knopfs.
  - Den Rahmen sieht man nur unter der Maus. Das Aussehen ohne Maus bleibt gleich.
- **R2-5**
  - **Nachzeichnen, „Flecken entfernen“:** Das ist kein Layoutfehler. Die Zeile hat einen eigenen Clip, und die Spalte läuft mit Bildlauf. Bei 150 %/1920 × 1080 lag die Zeile genau an der Unterkante des Bildlaufs.
    - Der Prüfer meldet das jetzt als `hinweis-bildlauf`, wenn die Schnittkante die Kante eines hohen Bereichs derselben Ebene ist.
    - Dasselbe gilt für Kontrollkästchen am Rand eines hohen Clips.
  - **Dokumentbibliothek, „Graphic“** (`panels\library.rs`): Die Vorschau schrumpft in niedrigen Docks (zwischen 24 und 96 px), damit die Liste zwei Zeilen behält.
- **NZ-2-Fund** (`panels\common.rs` `list_frame`): Die eigentliche Ursache war die Mindesthöhe der egui-`ScrollArea` von 64 px, nicht nur die eigenen 24 px.
  - Jetzt gilt `min_scrolled_height(0)` und `max_height` gleich der Listenhöhe.
  - Die Liste reicht nie mehr über die Fußleiste. In einem zu niedrigen Bedienfeld wird sie kleiner; unter 4 px wird sie nicht gezeichnet.
- **Prüfer erweitert** (`tests\clipping_audit.rs`), 27 neue Ansichten je Variante:
  - 10 Hauptmenüs (Popup `("fp-main-menu", i)`)
  - die Bedienfeldmenüs ≡, per Klick auf jeden ≡-Knopf (im Standardlayout öffnen 6)
  - die Kontextmenüs von Leinwand, Dock-Reiter und Inspektor-Reiter, per Rechtsklick
  - je 6 Tooltip-Stichproben in der Werkzeugleiste und im Inspektor (Zeiger darüber, Verzögerung 0)

  Geprüft wird nur die Menü- bzw. Tooltip-Ebene. Neue Bereiche im Bericht: „Menüs“ und „Tooltips“.
- **Neuer Test** `tests\dialog_scroll_rand2b.rs` (2 Tests):
  - Bei 800 × 512 liegen die drei Dialoge im Fenster.
  - OK bzw. Schließen liegt unten im Dialog, und ein Klick darauf schließt ihn.
  - Bei 1500 × 950 haben die Tastaturbefehle die volle Höhe.

## Zahlen (voller Lauf, 24 Varianten)

- **Vorher:**
  - Erlaubnisliste 78 Schlüssel (RAND-1: 408 Funde an 70 Stellen)
  - nach RAND-2a 17 Schlüssel, alle aus R2-3/R2-4/R2-5/NZ-2
- **Nachher:** 24 Funde an 3 Stellen, dazu 360 Hinweise.
  - Die 17 Einträge sind alle gestrichen („nicht mehr gefunden: 17“).
  - Neu in der Liste stehen nur die Menüs, die höher als das Fenster sind.
  - Bedienfelder, Inspektor, Dialoge, Startseite und Tooltips: 0.

## Neue Befunde

- **HG (`menu.rs`): Menüs, die höher als das Hauptfenster sind**, ragen unten hinaus. egui-Popups haben keinen Bildlauf, die unteren Einträge sind dann nicht erreichbar.
  - Hauptmenü 1 (Datei) und das zweite Bedienfeldmenü bei 800 × 512 pt
  - Hauptmenü 9 (Fenster) auch bei 1024 × 768
  - das Kontextmenü der Leinwand bei 800 × 512 pt, 694 pt hoch; das meldete der Prüfer nur in einem Teillauf

  Vorschlag: Menühöhe auf das Fenster begrenzen und einen Bildlauf bzw. Pfeile wie unter Windows einbauen. In der Erlaubnisliste stehen 4 Zeilen mit Kommentar.

## Nachprüfung (NACHPRUEFUNG)

- Im echten Fenster bei 1024 × 768 und 150 % jeweils öffnen: Voreinstellungen, Bearbeiten ▸ Tastaturbefehle, Befehle ▸ KI-Modelle verwalten.
  - Erwartet: Der Dialog liegt ganz im Fenster, und der Inhalt lässt sich mit Mausrad bzw. Bildlaufleiste blättern.
  - OK, Abbrechen, Hilfe und Schließen stehen immer unten und wirken. Enter löst den Standardknopf aus.
- Dokumentbibliothek in einer niedrigen Dockgruppe: Die Liste bleibt über der Fußleiste, die Fußleistenknöpfe bleiben frei klickbar.
- KI-Modelle verwalten, Zeile eines vorhandenen Modells: „Entfernen…“ hat Luft zum Rahmen, und der Rahmen erscheint unter der Maus.

## PARITAET

Keine Paritätszeile betroffen; es geht um Layout und Prüfer. N23 (P-RAND): R2-3 bis R2-5 auf „umgesetzt, Nachprüfung ausstehend“.

## Geprüft

- `cargo check --workspace --all-targets` ohne Warnungen
- Grün: `clipping_audit::` schnell und einmal `FP_RAND_FULL=1`, `dialogs_open::`, `dialogs_move::`, `dialog_scroll_rand2b::` (zusammen 11 Tests)
- Keine neuen Texte, deshalb weder fp-i18n noch der Scanner. Das neue Bild `docs\screenshots\rand\rand_01_*.png` (Fenster-Menü) zeigt keine gesperrten Namen.
