# KI-Modelle der KI-Auswahl (fp-ai)

Firepixel bettet keine KI-Gewichte in die exe ein (zu groß). Die Werkzeuge **Objektauswahl**,
**Motiv auswählen** und **Kante verfeinern** laden die Gewichte zur Laufzeit aus einem Modellordner.
Fehlt ein Modell, zeigt Firepixel den Dialog „KI-Modell fehlt“ mit Name, Quelle, Größe, Lizenz und
Zielordner; geladen wird **erst nach Klick auf „Herunterladen“** (mit Fortschritt, SHA-256-Prüfung und
„Abbrechen“). Ohne Zustimmung wird nichts heruntergeladen. „Befehle ▸ KI-Modelle verwalten…“
(`ai.manageModels`) zeigt alle Modelle mit Stand und erlaubt den Download einzeln (zweiter Klick
bestätigt nach Anzeige der Details).

## Modellordner (Suchreihenfolge)

1. Voreinstellung `AiModelFolder` (Dialog „KI-Modelle verwalten“ bzw. `ai.setModelFolder {path}`),
2. `%LOCALAPPDATA%\Firepixel\Models`,
3. `Models\` neben `firepixel.exe`.

Ältere Versionen hießen die Ordner `Modelle`; sie werden beim ersten Start einmal in `Models`
umbenannt (geht das nicht, z. B. ohne Schreibrecht neben der exe, wird der alte Ordner weiter benutzt).

Heruntergeladen wird in den ersten Ordner der Liste. Eine Datei gilt als vorhanden, wenn Name und
Größe stimmen; der Download prüft zusätzlich die SHA-256-Summe (fest im Code, `fp_ai::models::MODELS`)
und legt die Datei erst nach erfolgreicher Prüfung ab (vorher `*.part`).

**Manuell installieren:** die Datei von der angegebenen Adresse laden und unter dem Dateinamen der
Tabelle in einen der Ordner legen.

## Modelle

| Aufgabe | Modell (Dateiname) | Quelle (offiziell) | Größe | SHA-256 | Lizenz der Gewichte |
|---|---|---|---|---|---|
| Objektauswahl (Vorgabe) | SAM 2.1 Hiera Tiny (`sam2.1-hiera-tiny.safetensors`) | https://huggingface.co/facebook/sam2.1-hiera-tiny/resolve/main/model.safetensors | 155,9 MB | `48c14467e5cf9e51870511feb72c89688e82dd74523142c0538b663e193ac2a7` | Apache-2.0 |
| Objektauswahl (genauer) | SAM 2.1 Hiera Small (`sam2.1-hiera-small.safetensors`) | https://huggingface.co/facebook/sam2.1-hiera-small/resolve/main/model.safetensors | 184,3 MB | `0a4067b11ce1e23d5229203f11c718a823060d15a4b23fa2372a7d4b77cbbc60` | Apache-2.0 |
| Motiv auswählen (Vorgabe) | BiRefNet lite (`BiRefNet_lite.safetensors`) | https://huggingface.co/ZhengPeng7/BiRefNet_lite/resolve/main/model.safetensors | 177,6 MB | `4417d89795250e698c3cb0ae8df15743810065f646f48a694fdfa7ca052d0815` | MIT |
| Motiv auswählen (hohe Qualität) | BiRefNet (`BiRefNet.safetensors`) | https://huggingface.co/ZhengPeng7/BiRefNet/resolve/main/model.safetensors | 444,5 MB | `9ab37426bf4de0567af6b5d21b16151357149139362e6e8992021b8ce356a154` | MIT |
| Kante verfeinern (Vorgabe) | BiRefNet lite Matting (`BiRefNet_lite-matting.safetensors`) | https://huggingface.co/ZhengPeng7/BiRefNet_lite-matting/resolve/main/model.safetensors | 89,0 MB | `ce8bcfc045e336322c0424a5863dcfb7e9ce8fed0a5fd4d1b2b20adf12d97243` | MIT |
| Kante verfeinern (hohe Qualität) | BiRefNet HR Matting (`BiRefNet_HR-matting.safetensors`) | https://huggingface.co/ZhengPeng7/BiRefNet_HR-matting/resolve/main/model.safetensors | 444,5 MB | `a5a4de698739ea5e0e8bbab28e1b293dde95092b87a442d566cbc585c53cef55` | MIT |

Die Prüfsummen sind die LFS-SHA-256-Werte der Dateien auf Hugging Face (Stand 11.10.2026).

### Trainingsdaten – bitte beachten

* **SAM 2.1** (Meta, `facebookresearch/sam2`): Code und Gewichte Apache-2.0; Meta erlaubt die
  kommerzielle Nutzung der Gewichte. Trainiert auf SA-1B und SA-V.
* **BiRefNet** (Peng Zheng u. a., `ZhengPeng7/BiRefNet`): Code und Gewichte MIT. Laut Modell-Zoo des
  Autors sind die „general use“-Modelle (lite und Swin-L) u. a. mit DIS5K, DUTS, HRSOD, UHRSD, HRS10K,
  **P3M-10k** und „humans“ trainiert, die Matting-Modelle mit **P3M-10k, AM-2k, Distinctions-646,
  HIM2K** und „humans“. Mehrere dieser Datensätze sind von ihren Urhebern nur für nichtkommerzielle
  Forschung freigegeben. Die Gewichte selbst stehen unter MIT; ob die Datensatzbedingungen auf die
  Nutzung der Gewichte durchschlagen, ist rechtlich nicht geklärt. Der Auftrag hat BiRefNet und
  BiRefNet-Matting ausdrücklich vorgegeben; Firepixel zeigt den Hinweis im Download-Dialog an.
  **Kein** ViTMatte (Composition-1k) und **kein** RMBG-2.0 (Gewichte CC BY-NC).

## Eigene Umsetzung (Quelltext)

Die Netze sind in `crates\fp-ai` in reinem Rust nachgebaut (keine ONNX-Laufzeit, keine DLL, kein
`unsafe` im eigenen Code). Vorlagen für den Aufbau: `modeling_sam2.py` von Hugging Face Transformers
(Apache-2.0) bzw. das offizielle SAM-2-Repository (Apache-2.0), `birefnet.py` aus den Modell-Repos von
`ZhengPeng7` (MIT, enthält Swin Transformer von Microsoft, MIT). Geprüft gegen die Referenz in PyTorch
(Abweichung der Logits: BiRefNet lite ≤ 1,4·10⁻⁴, Matting lite ≤ 2,4·10⁻⁴, BiRefNet Swin-L ≤ 3,8·10⁻³;
SAM 2.1 Einbettung ≤ 1·10⁻⁵, Masken-Logits ≤ 4·10⁻⁴, Maske 100 % gleich).

## Feineinstellungen (Inspektor, Parameter)

Angeboten wird nur, was die Modelle bzw. unser Code wirklich hergeben (`crates\fp-ai\src\tuning.rs`).
Alles ist Nachbearbeitung der Modell-Ausgabe, die nach dem ersten Klick zwischengespeichert ist:
Ändert man einen Wert im Inspektor, bildet Firepixel die Auswahl desselben Werkzeugs sofort neu
(`retune: true`), ohne das Netz erneut laufen zu lassen.

| Einstellung | Objektauswahl (SAM 2.1) | Motiv auswählen (BiRefNet) | Kante verfeinern (Matting) | Warum |
|---|---|---|---|---|
| **Toleranz** 0…100 (50) | ✓ Schwelle der Masken-Logits, ±10 Logit-Einheiten | ✓ Tonwerte der weichen Maske (50 = unverändert) | – | SAM entscheidet über eine Schwelle (Vorgabe 0); BiRefNet liefert eine weiche Maske 0…1, deren Tonwerte sich monoton verschieben lassen. Beim Matting ist das Alpha die gemessene Deckkraft (Haare), eine Schwelle würde sie zerstören. |
| **Variante** Automatisch/Klein/Mittel/Groß | ✓ | – | – | Der SAM-Decoder rechnet immer eine Einzelmaske und drei Mehrfachmasken mit eigener Güte (IoU). „Automatisch“ = Wahl des Modells, sonst die Mehrfachmasken nach Fläche (Teil, Objekt, Objekt mit Umgebung). BiRefNet hat nur eine Ausgabe. |
| **Kante** Hart/Anti-Alias/Weiche Kante + Wert | ✓ | ✓ | – | wie beim Zauberstab; reine Nachbearbeitung. Beim Matting ist die Kante das Ergebnis selbst. |
| **Ausdehnen** −100…100 px | ✓ | ✓ | – | Grauwert-Morphologie mit Kreisscheibe: der weiche Rand wandert mit, statt hart zu werden. |
| **Glätten** 0…100 px | ✓ | ✓ | – | Dreiecksfilter und Straffen des Übergangs: rundet Ecken, entfernt Krümel und kleine Löcher. |
| **Radius** (0 = automatisch) | – | – | ✓ (wie bisher) | Breite des Kantenbereichs, in dem das Matting-Alpha gilt. Er bestimmt auch den Bildausschnitt für das Netz; ein anderer Radius braucht deshalb einen neuen Matting-Lauf und wird erst beim nächsten Klick angewandt. |

Nicht angeboten: eine Wahl der BiRefNet-Auflösung (das Netz ist für 1024 trainiert, siehe
`birefnet_size`), „Objektwert“-Schwelle von SAM (sie sagt nur, ob unter dem Punkt überhaupt ein
Objekt liegt, und steht im Ergebnis als `objectScore`), Punktdichte/Mehrfachpunkte im Inspektor
(Punkte und Rahmen kommen schon über Klick, Umschalt/Alt-Klick und Aufziehen).

Voreinstellungen: `AiObjectTolerance`, `AiObjectEdge`, `AiObjectFeather`, `AiObjectVariant`,
`AiObjectExpand`, `AiObjectSmooth` bzw. `AiSubject…`; Parameter der Befehle in `docs\AGENTS.md` §5
(„KI-Auswahl“).

## Testbilder (`crates\fp-ai\testdata`)

Frei lizenzierte Fotos von Wikimedia Commons (CC0), auf 1280 px Breite verkleinert:

| Datei | Quelle |
|---|---|
| `person_haare.jpg` | https://commons.wikimedia.org/wiki/File:Attractive_beautiful_redhead_girl.jpg (CC0, Pixabay) |
| `hund_gras.jpg` | https://commons.wikimedia.org/wiki/File:Blonde_dog_in_grass_(Unsplash).jpg (CC0) |
| `katze.jpg` | https://commons.wikimedia.org/wiki/File:Siamese-Cat_Andreas-LischkaPixabay.jpg (CC0) |
| `produkt_tasse.jpg` | https://commons.wikimedia.org/wiki/File:Coffee,_flowers_and_books_(Unsplash).jpg (CC0) |
