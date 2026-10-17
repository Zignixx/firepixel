# Batch Studio: step card form, batch settings (ffmpeg), confirmation when closing during a run
# (BATCH-11a, docs\BATCH_MODERN.md §2.4, §2.8, §5.5). Choice values live per package under
# batch-value-<id>-<param>-<value>.

## Form
batch-form-auto = Automatic
batch-form-transparent = transparent
batch-form-on = On
batch-form-off = Off
batch-form-reset = Reset to default
batch-form-reset-auto = Reset to “Automatic”
batch-form-more-count = { $label } ({ $n })
batch-form-time-hint = e.g. 1500 or 0:01.5
batch-form-choose = Choose...
batch-form-no-file = No file chosen
batch-form-json-invalid = Not valid JSON – the change is applied once the content is valid.
batch-form-range = { $label }: please enter a number from { $min } to { $max }.
batch-form-color =
    .tooltip = Choose color (swatches, HEX/RGB, eyedropper)

## Batch settings
batch-settings-open = Settings...
batch-settings-title = Batch settings
batch-settings-section-general = General
batch-settings-section-video = Videos (ffmpeg)
batch-settings-mode = Mode
batch-settings-ffmpeg = ffmpeg file
batch-settings-ffmpeg-choose = ffmpeg file...
batch-settings-ffmpeg-auto = Find automatically
batch-settings-ffmpeg-recheck = Check again
batch-settings-ffmpeg-checking = Checking ...
batch-settings-ffmpeg-ok = ffmpeg { $version }
batch-settings-ffmpeg-missing = ffmpeg not found
batch-settings-ffmpeg-by-override = set by the program
batch-settings-ffmpeg-by-setting = chosen in the settings
batch-settings-ffmpeg-by-environment = from FIREPIXEL_FFMPEG
batch-settings-ffmpeg-by-path = found on the system path (PATH)
batch-settings-ffmpeg-by-nextToExe = found next to Firepixel
batch-settings-ffmpeg-hint = Firepixel does not include ffmpeg and downloads nothing. For videos, install ffmpeg yourself and choose the file ffmpeg.exe here – or add its folder to the system path. Images and animations do not need ffmpeg.
batch-settings-ffmpeg-saved = ffmpeg file saved.
batch-settings-ffmpeg-cleared = ffmpeg is found automatically again.
batch-settings-ffmpeg-filter = Programs
batch-settings-close = Close

## Confirmation when closing during a run
batch-quit-title = Batch still running
batch-quit-text = A batch is still running – cancel it and close?
batch-quit-detail = { $done } of { $total } files are done. Finished files are kept; the rest will not be processed.
batch-quit-confirm = Cancel and close
batch-quit-keep = Keep running
