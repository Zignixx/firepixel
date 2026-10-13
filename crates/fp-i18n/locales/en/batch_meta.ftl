# Batch Studio: metadata and optimization (BATCH-5, docs\BATCH_MODERN.md §3.7, §3.12).
# Prefixes: batch-meta-, batch-step-metadata-*, batch-step-optimize-*, batch-param-metadata-*,
# batch-param-optimize-*, batch-param-output-compression.

## Steps
batch-step-metadata-edit = Metadata
    .description = Keep or remove metadata, e.g. GPS location, camera or thumbnail.
batch-step-metadata-set = Set metadata
    .description = Enter author, copyright, description, keywords or date taken.
batch-step-metadata-orientation = Apply orientation
    .description = Rotates the image according to the camera orientation and resets the tag.
batch-step-optimize-png = Optimize PNG losslessly
    .description = Makes PNG files smaller without changing a single pixel.
batch-step-optimize-quantize = Reduce colors
    .description = Reduces the number of colors (e.g. for small PNG8 or GIF files).
batch-step-optimize-targetSize = Target size
    .description = Picks the best quality at which the file stays within the size.

## Parameters
batch-param-metadata-edit-keep = Keep
batch-param-metadata-edit-remove = Remove
batch-param-metadata-edit-tags = Remove single EXIF tags
batch-param-metadata-set-artist = Author
batch-param-metadata-set-copyright = Copyright
batch-param-metadata-set-description = Description
batch-param-metadata-set-title = Title
batch-param-metadata-set-keywords = Keywords
batch-param-metadata-set-rating = Rating
batch-param-metadata-set-dateTaken = Date taken
batch-param-metadata-set-gps = GPS location
batch-param-optimize-png-level = Level
batch-param-optimize-png-strip = Remove metadata
batch-param-optimize-png-alphaClean = Clean invisible colors
batch-param-optimize-quantize-colors = Colors
batch-param-optimize-quantize-dither = Dithering
batch-param-optimize-quantize-ditherAmount = Dither amount
batch-param-optimize-targetSize-maxKB = At most
batch-param-optimize-targetSize-strategy = Method
batch-param-optimize-targetSize-minQuality = Minimum quality
batch-param-optimize-targetSize-minScale = Smallest size
batch-param-output-compression = Compression

## Categories
batch-meta-cat-gps = GPS location
batch-meta-cat-camera = Camera
batch-meta-cat-software = Software
batch-meta-cat-thumbnails = Thumbnail
batch-meta-cat-xmp = XMP
batch-meta-cat-iptc = IPTC
batch-meta-cat-comments = Comments
batch-meta-cat-icc = Color profile
batch-meta-what-comment = comment
batch-meta-dither-none = no dithering
batch-meta-dither-ordered = ordered dithering
batch-meta-dither-floydSteinberg = Floyd-Steinberg

## Summaries
batch-meta-summary-remove-all = Remove all metadata
batch-meta-summary-keep-all = Keep all metadata
batch-meta-summary-remove = Remove { $list }, keep the rest
batch-meta-summary-orientation = Rotate by camera orientation
batch-meta-summary-png = Lossless, level { $level }
batch-meta-summary-quantize = { $colors } colors, { $dither }
batch-meta-summary-target = At most { $kb } KB

## Notes and errors
batch-meta-warn-format-drops = { $format } does not store metadata ({ $what }); it is missing in the new file.
batch-meta-warn-png-only = PNG optimization only works with “Save as PNG” (selected: { $format }).
batch-meta-warn-target-format = Target size: { $format } has no quality setting; only “Quality + size” can make the file smaller.
batch-meta-warn-target-missed = Target size { $target } KB not reached, smallest file: { $size } KB.
batch-meta-info-target-resized = Scaled down to { $percent } % for the target size.
batch-meta-info-target-lossy = Saved lossy instead of lossless for the target size.
batch-meta-warn-no-date = Date taken not changed: the file has no date to shift.
batch-meta-issue-tags = Invalid EXIF tags (number, “0x0112” or a name like “Make”).
batch-meta-issue-date = Invalid date (e.g. “2026-10-13 14:30” or “+2h”, “-1d 30m”).
batch-meta-err-size = { $format } allows at most { $max } × { $max } pixels.
