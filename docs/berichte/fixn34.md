# FIX-N34 – Vorschau beim Ziehen in der Stapelfolge

Stand 15.10.2026, Slot `fp-s2`. Commit **47f8aa7** (main, nicht gepusht).

## Befund

N34 (Wortlaut-Anfang): „**Verschieben zieht Objekt kurz nach vorn** … Graues Rechteck liegt unter
einem grünen …“. Die Vorschau beim Ziehen (`canvas::ghost`, gerenderte Textur) wurde über alle Objekte
gelegt; erst nach dem Loslassen stimmte die Stapelfolge wieder.

## Umsetzung

- Neues Modul `crates\fp-ui\src\canvas\stack.rs`: Die Seite wird beim Ziehen in Teile zerlegt (ganze
  Ebenen ohne gezogenes Objekt bzw. Elemente; Gruppen und Unterebenen nur, wenn sie ein gezogenes
  Objekt enthalten). Daraus entstehen Schichten:
  - das Hintergrundbild mit allem unter dem untersten gezogenen Objekt,
  - je Lauf aufeinanderfolgender gezogener Objekte die Vorschau-Textur,
  - darüber je eine durchsichtige Schicht mit den Objekten bis zum nächsten Lauf.
- Die Schichten rendert der Render-Thread im selben Auftrag wie das Hintergrundbild, einmal je
  Ziehvorgang (und bei neuem Ausschnitt oder Zoom). Während des Ziehens werden nur Texturen
  übereinandergelegt, also genauso schnell wie bisher. Liegt das gezogene Objekt oben (häufigster
  Fall), gibt es keine zusätzliche Schicht. Web-Ebene, ausgeblendete Ebenen und Objekte zählen nicht.
- Effekte, Deckkraft und Mischmodus der Objekte darüber bleiben erhalten (fp-render rendert sie in
  der Schicht). Masken und Gruppen: Kinder einer Gruppe werden einzeln geschichtet. Ist eine Maske
  selbst gezogen oder ein Teil eines Compound Shapes, gilt wie bisher „Vorschau oben“.
- **Alt-Ziehen (Kopie):** Das Original bleibt stehen, die Vorschau liegt direkt darüber – wie das
  Duplikat nach dem Loslassen (`edit.duplicate` fügt schon direkt über dem Original ein).
- **Mehrfachauswahl** mit Objekten dazwischen: mehrere Läufe, jedes Objekt in seiner Lage.
- **Transformieren / Skalieren / Drehen / Unterauswahl** laufen über dieselbe Vorschau
  (`Overlay::Ghost`) und sind damit mit abgedeckt. Pfeiltasten verschieben direkt im Dokument
  (Neurendern), dort stimmte die Stapelfolge schon.
- Die Vorschauen werden jetzt direkt nach dem Hintergrundbild gezeichnet (`preview_stack`), also unter
  Raster, Hilfslinien, Auswahlrahmen und Werkzeug-Rückmeldung. Vorher lag die Vorschau auch über
  Rahmen und Hilfslinien.
- Bis das passende Hintergrundbild da ist (Render-Thread), bleibt das alte Bild stehen, statt die
  Vorschau kurz über alles zu legen.
- **Malvorschau (BA-039, Teil „Vorschau in Stapelfolge“):** Mit demselben Mechanismus gelöst.
  `PaintPreview` kennt jetzt die bemalte Bitmap (`target`), Objekte darüber verdecken den Strich schon
  während des Malens. Noch offen: Malen in eine **neue** Bitmap (`target = None`) sowie Deckkraft
  und Mischmodus der bemalten Bitmap selbst in der Vorschau.

Grenzen: Mischmodi von Objekten **über** dem gezogenen werden in der Schicht gegen Transparenz
statt gegen den Inhalt darunter gerechnet. Bei Normal ist das pixelgleich, bei anderen Modi nur
während des Ziehens eine Annäherung. Bei einer Alt-Kopie mehrerer Objekte in einem Lauf liegen die
Kopien gemeinsam über dem Lauf, nicht einzeln über ihren Originalen.

## Prüfungen

- `cargo test -p fp-ui --test all drag_stack_n34::` 4/4 (wgpu-Bildprüfung):
  - Ziehen: Pixel an der Überlappung = Grün, verschobenes Grau sichtbar, Original ausgeblendet.
  - Objekt oben: keine Zusatzschicht.
  - Alt-Kopie unter Grün.
  - Pinsel auf Bitmap unter Grün: Strich unter Grün verdeckt.
- `cargo test -p fp-ui --lib canvas::` 54/54, darin 4 neue Tests `canvas::stack::tests`.
- `--test all`:
  - brush_live_pinsel1 4/4
  - canvas* 25
  - drag* 23
  - bitmap* 18
  - move* 10
  - transform* 4
- fp-engine `paint` grün, fp-export `golden` 2/2, fp-neutral grün.
- `cargo check --workspace --all-targets` ohne Warnungen.
- **Echtes Fenster, Brücke** (eigene Instanz, Build aus dem Slot): Grau 50..150 unter Grün 160..260
  - `ui_input` mit move, down und mehreren move **ohne up**, danach `screenshot`: Grau liegt an
    110..210 hinter Grün (`re\_testdateien\fixn34\ziehen_grau_unter_gruen.png`). Nach dem up liegt
    Grau bei 110..210 unter Grün.
  - Alt gehalten: Die Kopie liegt während des Ziehens unter Grün, das Original bleibt stehen
    (`re\_testdateien\fixn34\alt_kopie_unter_gruen.png`).

## Für PARITAET / Nachprüfung

- N34 → umgesetzt, Nachprüfung ausstehend.
- BA-039 Teilpunkt „Vorschau in Stapelfolge“ → umgesetzt für bestehende Bitmaps. Offen: neue
  Bitmap, Deckkraft und Mischmodus der bemalten Bitmap.
- Nachprüfung (Ablauf): Gruppe mit Effekt (Schlagschatten) über dem gezogenen Objekt, Objekt in einer
  Maskengruppe per Unterauswahl ziehen, Ebene darüber mit Deckkraft 50 %, Skalieren-Werkzeug am
  unteren Objekt. Erwartet: Das Objekt darüber verdeckt die Vorschau während des Ziehens genau wie
  nach dem Loslassen. Leistung bei 400 % und großer Seite: kein Ruckeln, nur ein kurzes Warten zu
  Beginn des Ziehens.

## Neue Befunde

- Textbearbeitung (`Overlay::TextEdit`) legt die Textvorschau ebenfalls über alles. Das lässt sich
  mit demselben Mechanismus beheben, ist aber nicht Teil von N34.
