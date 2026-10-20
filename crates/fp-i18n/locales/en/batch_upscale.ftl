# Batch Studio, BA-109 (docs\BATCH.md §3.4): AI upscaling in crates\fp-batch\src\steps\upscale.rs.
# Steps batch-step-<id>, parameters batch-param-<id>-<param>, values batch-value-<id>-<param>-<value>.

## AI upscale
batch-step-ai-upscale = AI upscale
    .description = Enlarges images two or four times with AI (Real-ESRGAN), reconstructing details and edges.
batch-param-ai-upscale-scale = Factor
batch-param-ai-upscale-model = AI model
    .tooltip = If the model is missing, load it via “Commands ▸ Manage AI Models...”.
batch-param-ai-upscale-alpha = Transparency
batch-value-ai-upscale-scale-2 = 2×
batch-value-ai-upscale-scale-4 = 4×
batch-value-ai-upscale-model-auto = Matching the factor
batch-value-ai-upscale-model-realEsrganX4 = Real-ESRGAN x4plus
batch-value-ai-upscale-model-realEsrganX2 = Real-ESRGAN x2plus
batch-value-ai-upscale-alpha-network = With the AI model (more accurate)
batch-value-ai-upscale-alpha-bicubic = Bicubic (faster)
batch-upscale-summary = { $scale }× with { $model }
batch-upscale-preview-region = Preview: AI only in the detail in the middle ({ $size } × { $size } px), the rest bicubic. The run computes the whole image.
