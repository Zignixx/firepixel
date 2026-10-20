# Batch-Studio, BA-109 (docs\BATCH.md §3.4): Schritt ai.faceBlur in crates\fp-batch\src\steps\face_blur.rs.

## Gesichter weichzeichnen
batch-step-ai-faceBlur = Gesichter weichzeichnen
    .description = Findet Gesichter (von vorn) und zeichnet sie weich oder verpixelt sie. Die Gesichtserkennung ist eingebaut, es wird nichts heruntergeladen.
batch-param-ai-faceBlur-strength = Stärke
    .tooltip = Bezogen auf die Gesichtsgröße: 100 % = Weichzeichnungsradius halb so groß wie das Gesicht.
batch-param-ai-faceBlur-shape = Form
batch-param-ai-faceBlur-pixelate = Verpixeln statt weichzeichnen
batch-param-ai-faceBlur-padding = Rand
    .tooltip = Vergrößert die gefundene Fläche je Seite um diesen Anteil der Gesichtsgröße.
batch-param-ai-faceBlur-minSize = Kleinstes Gesicht
batch-param-ai-faceBlur-sensitivity = Empfindlichkeit
    .tooltip = Hoch findet mehr Gesichter, aber auch mehr Fehltreffer.
batch-param-ai-faceBlur-region = Suchbereich
    .tooltip = Leer = ganzes Bild, sonst {"{"}"x": 0, "y": 0, "width": 400, "height": 300{"}"} in Pixeln.
batch-value-ai-faceBlur-shape-ellipse = Ellipse
batch-value-ai-faceBlur-shape-rect = Rechteck
batch-value-ai-faceBlur-sensitivity-low = Niedrig
batch-value-ai-faceBlur-sensitivity-normal = Normal
batch-value-ai-faceBlur-sensitivity-high = Hoch
batch-face-summary = { $mode }, { $strength } %
batch-face-summary-blur = Weichzeichnen
batch-face-summary-pixelate = Verpixeln
batch-warn-face-none = Kein Gesicht gefunden (0 Gesichter), das Bild bleibt unverändert.
