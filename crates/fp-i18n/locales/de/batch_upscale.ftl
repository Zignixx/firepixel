# Batch-Studio, BA-109 (docs\BATCH.md §3.4): KI-Hochskalierung in crates\fp-batch\src\steps\upscale.rs.
# Schritte batch-step-<id>, Parameter batch-param-<id>-<param>, Werte batch-value-<id>-<param>-<wert>.

## KI-Hochskalieren
batch-step-ai-upscale = KI-Hochskalieren
    .description = Vergrößert Bilder mit KI (Real-ESRGAN) auf das Doppelte oder Vierfache und rekonstruiert dabei Details und Kanten.
batch-param-ai-upscale-scale = Faktor
batch-param-ai-upscale-model = KI-Modell
    .tooltip = Fehlt das Modell, über „Befehle ▸ KI-Modelle verwalten...“ laden.
batch-param-ai-upscale-alpha = Transparenz
batch-value-ai-upscale-scale-2 = 2×
batch-value-ai-upscale-scale-4 = 4×
batch-value-ai-upscale-model-auto = Passend zum Faktor
batch-value-ai-upscale-model-realEsrganX4 = Real-ESRGAN x4plus
batch-value-ai-upscale-model-realEsrganX2 = Real-ESRGAN x2plus
batch-value-ai-upscale-alpha-network = Mit dem KI-Modell (genauer)
batch-value-ai-upscale-alpha-bicubic = Bikubisch (schneller)
batch-upscale-summary = { $scale }× mit { $model }
batch-upscale-preview-region = Vorschau: KI nur im Ausschnitt in der Mitte ({ $size } × { $size } px), der Rest bikubisch. Der Lauf rechnet das ganze Bild.
