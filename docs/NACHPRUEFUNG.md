# Nachprüfung am Ende (Nachtlauf)

Seit 12.10.2026 ≈ 18:00 auf Wunsch des Nutzers: Die aufwendigen Prüfungen laufen gesammelt am Ende, über Nacht. Dazu gehören Prüfungen im echten Fenster bzw. über die Brücke, Bildserien in 3 Designs × de/en und breite Testläufe über ganze Crates. Siehe AGENTEN_REGELN §7a.

Jeder Agent trägt hier ein, was er gebaut, aber noch nicht im echten Fenster geprüft hat: eine Zeile je Punkt, mit dem Ablauf, der zu prüfen ist. Die Prüf-Agenten am Ende (PLAN §5 Z: KT-1…4) arbeiten die Liste ab, haken ab (✅ / ❌ + Befund) und lassen ❌ beheben.

| Agent | Commit | Was prüfen (Ablauf, erwartetes Ergebnis) | Art | Stand |
|---|---|---|---|---|
| FV-10 | `f618672` | W19, W21, W22, W24, W26, W29, O2, O8, O16, falls PM-4 sie nicht schon abgehakt hat (siehe PARITAET_A) | Brücke | ⏳ |
| FV-15 | `5eee815` | F7 im frisch gestarteten Original messen (R1, „30“ tippen, R2 anklicken), dann Firepixel angleichen | Original + Brücke | ⏳ |
| FV-14 | `8b03953` | Fang L4/L35 im Original messen. Testdokument per `fw.openDocument` aus einer kopierten Datei öffnen, nicht neu anlegen: R1 50,50–150,120, E1 200,50–300,120, G 350–470 × 50–100. Zeiger-Ziehen ab 100,85 um +0/+3, +47/0, +0/+22, +0/−7, +0/+17, +97/0: In Firepixel rasten Kante an Kante und Mitte an Mitte, Kante an Mitte nicht. Stiftklicks 153,20 · 253,20 · 585,53 · 585,77: Firepixel x 150, x 253, y 50, y 77. Bei Abweichung `tools\snap.rs` anpassen | Original | ⏳ |
| FV-14 | `8b03953` | L33 im Original: Pfad 320,240 · 400,240 · 480,240 (Eckpunkte), Unterauswahl. Alt-Ziehen am mittleren Punkt nach 400,270, am Endpunkt nach 510,240, am Anfangspunkt nach 290,240. Firepixel: nur der Ausgangshenkel liegt unter dem Zeiger, der Eingangshenkel bleibt | Original | ⏳ |
| FV-14 | `8b03953` | L34 im Original: Ablauf L31–L33 wiederholen und vor dem Stiftklick `fw.selection` lesen. Prüfen, ob der neue Pfad nur entsteht, wenn der Pfad nicht mehr gewählt ist; so verhält sich Firepixel | Original | ⏳ |
| FV-14 | `8b03953` | Durchsichtige Bitmap-Pixel im Original: Klick in die Ecke einer Ellipsen-Bitmap (404,254 bei 400,250–500,330), Umschalt-, Strg- und Unterauswahl-Klick in eine Lücke, Ziehen aus der Lücke. Firepixel: Alle wählen bzw. verschieben die Bitmap, außerhalb des Rahmens nichts | Original | ⏳ |
| FV-14 | `8b03953` | Paarbilder `fv14_*` aus beiden Programmen (L4, L13, L25, L33, L35, durchsichtige Pixel). Firepixel ist schon geprüft: über die Brücke, L7 mit SendInput | Original + Brücke | ⏳ |
