# Bericht BV-5 – N25: 2-/4-fach-Felder laufen synchron (13.10.2026)

## Umgesetzt

Commit: `BV-5: 2-/4-fach synchron in Dokumentansicht und Bildvorschau (N25)` (Hash siehe `git log --grep "BV-5"`).

**Ursache N25 (Nutzerbild `re\_nutzerbilder\2026-10-13\2.webp`):** Das Bild zeigt die Reiter 2-fach/4-fach des Dokumentfensters. Dort war nur die Originalkachel (Feld oben links = Leinwand) bewegbar. Die Vorschaukacheln zeigten das Exportbild immer mittig, ohne Bildlauf, und bei DPI ≠ 100 % zusätzlich in falscher Größe (Teilung durch die Pixeldichte).

- `crates\fp-ui\src\preview.rs`: Es gibt eine gemeinsame Lage für alle Felder, nämlich die Ansicht des Dokuments (`view.zoom`, `view.origin`, `view.viewport`).
  - Jede Vorschaukachel legt den Sichtbereich mittig in ihre Bildfläche (`virtual_min`). Felder verschiedener Größe zentrieren dadurch auf denselben Dokumentpunkt.
  - Das Exportbild deckt die Seite bzw. ihren Exportbereich in Dokumentkoordinaten ab (`image_screen_rect`).
  - Neu ist das Bewegen in jeder Vorschaukachel (`navigate`), mit derselben Wirkung wie auf der Leinwand: Mausrad (52 px je Rastung), Strg+Rad bzw. Zwei-Finger-Zoom (eine Stufe je Rastung um den Zeiger), Hand-Werkzeug, Leertaste+Ziehen, mittlere Taste (der gegriffene Punkt bleibt unter dem Zeiger), Zoom-Werkzeug (Klick, Alt+Klick, Rahmen aufziehen) und Strg+Leertaste+Klick.
  - Die Felder werden erst bewegt, dann alle gezeichnet. So stimmen sie im selben Bild überein.
  - Die Originalkachel (Leinwand), Bildlaufleisten, Zoomliste und Strg+/− wirken ohnehin auf dieselbe Ansicht.
- `crates\fp-ui\src\canvas\mod.rs` + `render.rs`:
  - `report_viewport`: Beim Wechsel Original/Vorschau/2-fach/4-fach bleibt der Dokumentpunkt in der Mitte stehen. Bisher blieb die linke obere Ecke stehen.
  - `switch_center` verhindert, dass sich halbe Pixel aufsummieren, wenn man mehrfach umschaltet.
  - `clamp_view`, `page_rect`, `wheel_notches` und `take_step` sind jetzt `pub(crate)`.
- `crates\fp-ui\src\dialogs\image_preview\mod.rs` (Datei ▸ Bildvorschau):
  - Der gemeinsame Mittelpunkt `center` ist jetzt in **Dokumentpixeln** angegeben (`pane_map`, `center_doc`, `image_rect`). Vorher waren es Bildpixel des aktiven Feldes. Felder mit eigenem Exportbereich, eigener Skalierung, „Vorschau“ aus (Original) oder dem Werkzeug „Exportbereich“ (ganzes Bild) zeigen damit jetzt denselben Ausschnitt.
  - Neu: Leertaste+Ziehen und die mittlere Taste verschieben mit jedem Werkzeug.
  - Strg+Rad und Rad rechnen mit dem Maßstab des Feldes unter dem Zeiger.
- Tests: `crates\fp-ui\tests\preview_sync_bv5.rs` (3 kittests, realistische Eingaben):
  - Dokumentfenster 4-fach: Rad in Feld 1, Leertaste+Ziehen in Feld 1, mittlere Taste in Feld 2, Hand in Feld 3 und Feld 0, Strg+Rad in Feld 3 (Punkt unter dem Zeiger bleibt fest), Zoom-Werkzeug und Alt-Klick in Feld 2, Zoomliste 200 %. Nach jedem Schritt ist gemessen, dass alle vier gezeichneten Bilder dieselbe Lage und denselben Zoom haben.
  - Umschaltfolge Original→4→2→4→Vorschau→2→Original: Zoom und Mittelpunkt bleiben. Dazu 2→4 nach dem Bewegen und 50 Rastungen hin und zurück ohne Drift.
  - Dialog 4-fach mit Exportbereich, 50 % Skalierung und Original in je einem Feld: Rad, Leertaste+Ziehen bei aktivem Zoom-Werkzeug, Zoomklick in Feld 3, Strg+Rad im 50-%-Feld, mittlere Taste, 4→2→4.
  - Grün sind außerdem `image_preview_p1..p4`, `export_wizard`, die Lib-Tests `canvas` und `cargo check --workspace --all-targets`.

## Abgleich mit dem Original (`re\22-ansicht-bitmap-farbe.md` §3)

- Hand, Leertaste+Ziehen, Zoom-Werkzeug, Strg+/−, Zoomliste und Umschalten verhalten sich wie im Original: synchron, die Lage bleibt.
- 🎨 **Bewusste Abweichung (Nutzerwunsch N25):** Das **Mausrad** rollt in Firepixel alle Felder synchron. Im Original rollt es im Dokument-Reiter nur das Feld unter dem Zeiger und hat im Dialog Bildvorschau keine Wirkung.
- Zusätzlich gegenüber dem Original: mittlere Taste und Strg+Rad-Zoom in den Feldern.
- Grenze am Dokumentrand: Im Dokumentfenster gilt die bestehende Begrenzung der Leinwand (60 px Rand), im Dialog endet der Ausschnitt am Bildrand.

## PARITAET

N25 / Bildvorschau §5.6 „Synchrones Verschieben und Zoomen“ und die Ansichtsreiter 2-fach/4-fach: „umgesetzt, Nachprüfung ausstehend“.

## Zeile für NACHPRUEFUNG

- (BV-5, N25) Echtes Fenster:
  - Ablauf: Großes Bild öffnen (z. B. 1200 × 900), Reiter 4-fach, Zoom 150 %. In **jedem** der vier Felder nacheinander ausführen: Mausrad (3 Rastungen), Leertaste+Ziehen, Hand-Werkzeug ziehen, Zoom-Werkzeug Klick/Alt+Klick, Strg+Rad, Zoomliste unten rechts. Danach 2-fach ↔ 4-fach ↔ Original umschalten. Dasselbe in Datei ▸ Bildvorschau 2-/4-fach, auch mit einem Feld „Vorschau“ aus und einem Feld mit Exportbereich.
  - Erwartet: Alle Felder zeigen jederzeit denselben Ausschnitt und Zoom, kein Feld springt oder hinkt ein Bild nach. Beim Umschalten bleibt der Mittelpunkt. Bei 125/150 % DPI ist das Exportbild genauso groß wie das Original.

## Befunde / Annahmen

- Annahme: Bei einem Feld mit Skalierung (z. B. 50 %) zeigt die Dokumentansicht das Exportbild auf Seitengröße gestreckt (pixelig), damit es deckungsgleich zum Original liegt. Im Dialog wird es in Bildpixeln gezeigt und auf denselben Dokumentpunkt zentriert.
- Der Rahmen des Zoom-Werkzeugs in einer Vorschaukachel ist ein einfacher Rahmen in Fokusfarbe, nicht der gestrichelte Leinwand-Rahmen.
- Offen: Die Leinwandbegrenzung des Dokumentfensters hat 60 px Rand, das Original endet laut §3.1 genau am Dokumentrand. Das ist eine bestehende Leinwand-Regel und gilt auch für die 1-fach-Ansicht, deshalb hier nicht geändert. Kandidat für einen eigenen Paritätsbefund.
