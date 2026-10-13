# Batch Studio: animation and video (BATCH-7, docs\BATCH_MODERN.md §3.8, §5.5).
# Prefixes: batch-anim-, batch-step-anim-*, batch-step-video-import, batch-param-anim-*,
# batch-param-video-import-*.

## Steps
batch-step-video-import = Read video
    .description = Reads a video through ffmpeg as a frame sequence: range, frames per second and size.
batch-step-anim-trim = Trim
    .description = Cuts off the start and end or removes single frames.
batch-step-anim-speed = Speed
    .description = Plays the animation faster or slower.
batch-step-anim-loop = Loop
    .description = How often the animation plays, backwards or back and forth.
batch-step-anim-dropFrames = Reduce frames
    .description = Keeps only every nth frame or merges nearly identical frames.
batch-step-anim-frameDelay = Frame duration
    .description = Sets the display time of all frames or lengthens the first or last frame.
batch-step-anim-toFrames = Animation → frames
    .description = Saves every frame of the animation as a file of its own.
batch-step-anim-fromFrames = Frames → animation
    .description = Combines all images of the list, in their order, into one animation.
batch-step-anim-spriteSheet = Sprite sheet
    .description = Lays out the frames of an animation in a grid or splits a grid into frames.

## Parameters
batch-param-video-import-start = Start
batch-param-video-import-end = End
batch-param-video-import-fps = Frames per second
batch-param-video-import-maxFrames = Maximum frames
batch-param-video-import-maxSize = Longest side
batch-param-video-import-decoder = Decoder
batch-param-video-import-decoder-auto = Automatic
batch-param-video-import-decoder-builtin = Built-in
batch-param-video-import-decoder-ffmpeg = ffmpeg
batch-param-anim-trim-unit = Given as
batch-param-anim-trim-unit-time = Time
batch-param-anim-trim-unit-frame = Frame number
batch-param-anim-trim-start = Start
batch-param-anim-trim-end = End
batch-param-anim-trim-removeFrames = Remove frames
batch-param-anim-speed-factor = Factor
batch-param-anim-speed-fps = Fixed frames per second
batch-param-anim-speed-minDelay = Shortest frame duration
batch-param-anim-loop-mode = Repeat
batch-param-anim-loop-mode-infinite = Forever
batch-param-anim-loop-mode-once = Play once
batch-param-anim-loop-mode-count = Count
batch-param-anim-loop-count = Repetitions
batch-param-anim-loop-pingPong = Back and forth
batch-param-anim-loop-reverse = Reverse
batch-param-anim-dropFrames-keepEvery = Keep every nth frame
batch-param-anim-dropFrames-targetFps = Target frames per second
batch-param-anim-dropFrames-mergeSimilar = Merge similar frames
batch-param-anim-dropFrames-threshold = Similar below
batch-param-anim-frameDelay-mode = Duration
batch-param-anim-frameDelay-mode-all = All the same
batch-param-anim-frameDelay-mode-keep = Unchanged
batch-param-anim-frameDelay-delay = Duration per frame
batch-param-anim-frameDelay-first = First frame
batch-param-anim-frameDelay-last = Pause at the end
batch-param-anim-toFrames-pattern = File name
batch-param-anim-fromFrames-delay = Duration per frame
batch-param-anim-fromFrames-size = Size
batch-param-anim-fromFrames-size-first = Like the first image
batch-param-anim-fromFrames-size-largest = Largest image
batch-param-anim-fromFrames-size-fixed = Fixed
batch-param-anim-fromFrames-width = Width
batch-param-anim-fromFrames-height = Height
batch-param-anim-fromFrames-background = Background
batch-param-anim-fromFrames-loop = Repeat
batch-param-anim-fromFrames-loop-infinite = Forever
batch-param-anim-fromFrames-loop-once = Play once
batch-param-anim-spriteSheet-direction = Direction
batch-param-anim-spriteSheet-direction-toSheet = Animation → sheet
batch-param-anim-spriteSheet-direction-fromSheet = Sheet → animation
batch-param-anim-spriteSheet-columns = Columns
batch-param-anim-spriteSheet-rows = Rows
batch-param-anim-spriteSheet-spacing = Spacing
batch-param-anim-spriteSheet-padding = Margin
batch-param-anim-spriteSheet-count = Number of frames
batch-param-anim-spriteSheet-delay = Duration per frame

## Summaries
batch-anim-end-of-clip = end
batch-anim-summary-video = { $start } to { $end }, { $fps } fps
batch-anim-summary-trim-time = { $start } to { $end }
batch-anim-summary-trim-frames = Frame { $start } to { $end }
batch-anim-summary-speed = { $factor } ×
batch-anim-summary-speed-fps = { $fps } fps
batch-anim-summary-drop = every { $n }th frame
batch-anim-summary-drop-fps = { $fps } fps
batch-anim-summary-delay = { $ms } ms per frame
batch-anim-loop-infinite = Forever
batch-anim-loop-once = Once
batch-anim-loop-count = { $count } repetitions

## Notes and errors
batch-anim-err-empty = No frame would be left.
batch-anim-err-sheet-grid = The grid does not fit the image (too many columns/rows or too large a margin).
batch-anim-err-ffmpeg-missing = ffmpeg not found – choose its path in the settings.
batch-anim-ffmpeg-found = Found ffmpeg { $version }: { $path }
batch-anim-err-ffmpeg-start = Could not start ffmpeg ({ $path }): { $error }
batch-anim-err-ffmpeg-timeout = ffmpeg does not respond ({ $path }).
batch-anim-err-ffmpeg-not-ffmpeg = The chosen file is not ffmpeg: { $path }
batch-anim-err-ffmpeg-pipe = The connection to ffmpeg was lost.
batch-anim-err-video-unreadable = Cannot read video: { $path } ({ $detail })
batch-anim-err-video-no-frames = The video has no frames in the chosen range: { $path }
batch-anim-err-no-profile = No settings folder available.
batch-anim-warn-not-video = “Read video” only works on videos.
batch-anim-warn-no-builtin = A built-in video decoder is not available yet; using ffmpeg.
batch-anim-issue-time = Cannot read the time in “{ $param }” (for example 1500, 2.5s or 0:02.500).
batch-anim-issue-end-before-start = The end is before the start.
batch-anim-issue-ranges = Cannot read the frame range “{ $part }” (for example 1-3, 10, 20-).

## Preview timeline
batch-anim-timeline-title = Timeline
batch-anim-timeline-start = Start
batch-anim-timeline-end = End
batch-anim-timeline-kept = { $kept } of { $total } frames, { $duration }
batch-anim-timeline-frame = Frame { $n }: { $ms } ms
batch-anim-timeline-remove = Remove frame
batch-anim-timeline-restore = Keep frame
batch-anim-timeline-reset = Reset
batch-anim-timeline-hint = Drag the handles to trim, click a frame to remove it.
