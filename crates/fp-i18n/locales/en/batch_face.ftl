# Batch Studio, BA-109 (docs\BATCH.md §3.4): step ai.faceBlur in crates\fp-batch\src\steps\face_blur.rs.

## Blur faces
batch-step-ai-faceBlur = Blur faces
    .description = Finds faces (frontal) and blurs or pixelates them. Face detection is built in, nothing is downloaded.
batch-param-ai-faceBlur-strength = Strength
    .tooltip = Relative to the face size: 100 % = blur radius of half the face.
batch-param-ai-faceBlur-shape = Shape
batch-param-ai-faceBlur-pixelate = Pixelate instead of blur
batch-param-ai-faceBlur-padding = Margin
    .tooltip = Grows the found area on every side by this share of the face size.
batch-param-ai-faceBlur-minSize = Smallest face
batch-param-ai-faceBlur-sensitivity = Sensitivity
    .tooltip = High finds more faces, but also more false hits.
batch-param-ai-faceBlur-region = Search area
    .tooltip = Empty = whole image, otherwise {"{"}"x": 0, "y": 0, "width": 400, "height": 300{"}"} in pixels.
batch-value-ai-faceBlur-shape-ellipse = Ellipse
batch-value-ai-faceBlur-shape-rect = Rectangle
batch-value-ai-faceBlur-sensitivity-low = Low
batch-value-ai-faceBlur-sensitivity-normal = Normal
batch-value-ai-faceBlur-sensitivity-high = High
batch-face-summary = { $mode }, { $strength } %
batch-face-summary-blur = Blur
batch-face-summary-pixelate = Pixelate
batch-warn-face-none = No face found (0 faces), the image stays unchanged.
