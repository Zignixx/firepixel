# RAND-2a: Pakete R2-1 und R2-2 des Prüfers für abgeschnittene Texte (Nutzerbefund N23)

Stand 13.10.2026, Slot s5. Commit `21edb24` auf main (im Slot war es `2831552`).

## Umgesetzt

**R2-1: Inspektor im schmalen Fenster** (`crates\fp-ui\src\inspector\mod.rs`)
- Den waagerechten Bildlauf gab es schon (seit I18N-INSPEKTOR, wenn der Inspektor schmaler als 902 pt ist). Zwei Dinge daran waren falsch:
  - Die feste Bildlaufleiste des Designs belegte 10 px Höhe. Dadurch war die untere Inspektorzeile unten abgeschnitten. Das war die eigentliche Ursache für das Kästchen „Autom. Unterschneidung“ (4 % abgeschnitten bei 100 %, 12 % bei 150 %).
  - Der Prüfer zählte Texte, die nur am Rand des Bildlaufs angeschnitten sind, als „abgeschnitten“.
- Die Leiste ist jetzt eine eigene Art (`hscroll_style`):
  - Sie schwebt über dem Inhalt und belegt keine Höhe. Sie liegt im 5-px-Rand unter Zeile 2.
  - In Ruhe ist sie 3 px breit und sichtbar (Deckkraft 0,7), damit man erkennt, dass sich der Inhalt verschieben lässt. Beim Darüberfahren wird sie 6 px breit.
- Im schmalen Inspektor blättert das Mausrad ohne Umschalttaste waagerecht (`always_scroll_the_only_direction`).
- Neue Funktion `inspector::hscroll_viewport(ctx)`: Sie gibt den sichtbaren Ausschnitt zurück, oder `None`, wenn der Inspektor ganz hineinpasst.
- Im großen Fenster bleibt alles wie bisher, wie im Original: kein Bildlauf, gleiche Lagen.
- Die Quadranten einzupassen habe ich verworfen. Bei 800 pt Fensterbreite ist der Inspektor etwa 450 pt breit, Q3 allein braucht aber 590 pt. Das ginge nur mit einem eigenen Layout aller Quadranten in allen Ansichten.

**R2-2** (`obj_quads.rs`)
- Kästchen „Autom. Unterschneidung“: 100 statt 92 DLU breit. Die Beschriftung braucht 97 DLU. Auf Deutsch wird nicht eingepasst („Deutsch unverändert“), deshalb war sie 7 px breiter als ihr Platz.
- Segment, Knopf „Bearbeiten...“ neben der Exportvorgabe:
  - Auf Deutsch jetzt 44 statt 53 DLU. Laut `dialoge.json` reicht er bis unter „Ziel:“, sein Text lief deshalb 3 px in die Beschriftung.
  - Jetzt endet der Knopf vor „Ziel:“, der Text hat 2,5 px Rand.
  - Englisch bleibt bei 30 DLU.

**Prüfer** (`crates\fp-ui\tests\clipping_audit.rs`)
- Ein Text, der genau an der linken oder rechten Kante von `hscroll_viewport` angeschnitten ist, zählt jetzt als `hinweis-bildlauf` und nicht mehr als `abgeschnitten`. Für diesen Text entfallen auch die Prüfung des Randabstands und die Überlappungsprüfung.
- Die Regel greift nur, wenn die Clip-Kante auf weniger als 1 px mit der Kante des Bildlaufs übereinstimmt. Ein Text, der am eigenen Quadranten oder Feld abgeschnitten ist, wird weiter gemeldet.

**Tests**
- Neue Datei `tests\inspector_rand2a.rs`:
  - Im 800 × 512-Fenster: Bildlauf vorhanden, der Ausschnitt reicht bis an den Inspektorrand, das Kästchen liegt ganz im Ausschnitt, das Mausrad blättert waagerecht.
  - Im 1920er-Fenster: kein Bildlauf, das Kästchen ist 150 px breit.
  - Segment: Knopf endet vor „Ziel:“, keine Überlappung, Text nicht gekürzt (über das Prüfprotokoll des Inspektors).
- In `inspector_i18n.rs` sind die vier behobenen Einträge aus `KNOWN_DE` gestrichen: Segment „Bearbeiten...“/„Ziel:“ und zweimal „Autom. Unterschneidung“. Der Test ist ohne sie grün.

## Zahlen

**Voller Lauf** (24 Varianten):

| | vorher (RAND-1) | nachher |
|---|---|---|
| Stellen insgesamt | 70 (408 Funde) | 17 (276 Funde) |
| davon Inspektor | 55 (198 Funde) | **0** |

Die übrigen 17 Stellen: Dialoge 11 (R2-3, R2-4, R2-5 Flecken entfernen) und Dokumentbibliothek 6 (RAND-2).

**Erlaubnisliste** `clipping_audit_allow.txt`: von 78 auf **17** Einträge.
- Gestrichen sind alle 61 Einträge, die nicht mehr gefunden werden. Das sind die Inspektor-Einträge aus R2-1/R2-2 und dazu Funde, die andere Pakete inzwischen behoben haben: Dokumentbibliothek „Graphic“, „Remove speckles“, Modelle „Remove…“ und die Bildvorschau mit dem Zeilensprung-Kästchen.
- Die Kommentare zu den gestrichenen Zeilen sind entfernt.

**Bericht** `docs\funktionstest\rand_auto.md` und die Ausschnitte `docs\screenshots\rand\` sind mit `FP_RAND_FULL=1` neu erzeugt.

## Geprüft
- `clipping_audit::`:
  - schneller Lauf grün, 0 neu (auch nach dem cherry-pick auf dem neuesten main)
  - voller Lauf mit `FP_RAND_FULL=1` grün: 0 neu, 0 nicht mehr gefunden
- `inspector_*::`: 54 bestanden, 1 ignoriert
- `cargo check --workspace --all-targets`: ohne Warnungen

## Für PARITAET / Nachprüfung
- In PARITAET: N23 / P-RAND, Pakete R2-1 und R2-2 auf „umgesetzt, Nachprüfung ausstehend“.
- **Nachprüfung im echten Fenster:**
  - Fenster 1024 × 768 bei 150 % (800 × 512 pt), Textwerkzeug wählen.
  - Erwartet: Unten im Inspektor eine dünne Leiste. „Autom. Unterschneidung“ ist ganz zu sehen. Das Mausrad über dem Inspektor schiebt den Inhalt nach rechts, bis Q5/Q6 (Filter) erscheinen.
  - Danach ein Segment wählen: „Bearbeiten...“ berührt „Ziel:“ nicht.
  - Im großen Fenster sieht der Inspektor unverändert aus.

## Neue Befunde
- Keine neuen. Offen bleiben R2-3 (Dialoge höher als das Fenster, 9 Stellen), R2-4 (`dialogs\models.rs`, „Entfernen…“ mit knappem Rand), R2-5 (Nachzeichnen-Dialog „Flecken entfernen“) und RAND-2 (Dokumentbibliothek).
