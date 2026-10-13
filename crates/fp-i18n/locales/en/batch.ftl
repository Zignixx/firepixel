# Modern batch mode (Batch Studio, docs\BATCH_MODERN.md) – core crates\fp-batch, commands batch.*,
# firepixel-cli batch --recipe. Created by BATCH-1; BATCH-2 and later add keys here or in batch_*.ftl.
# Prefix batch-; steps batch-step-<id> (dot as -), parameters batch-param-<id>-<param>.

## Groups
batch-group-files = Files
batch-group-format = Format
batch-group-geometry = Geometry
batch-group-color = Color and Effects
batch-group-background = Background
batch-group-overlay = Overlay
batch-group-metadata = Metadata
batch-group-animation = Animation
batch-group-optimize = Optimize
batch-group-document = Document
batch-group-advanced = Advanced

## Steps (BATCH-1)
batch-step-file-copyOnly = Copy/Move Only
    .description = Copies or moves the files unchanged, without opening them.
batch-step-rename = File Name
    .description = Sets how the new files are named.
batch-step-output = Save As
    .description = Format and format options of the new files.
batch-param-file-copyOnly-action = Action
batch-param-rename-mode = Mode
batch-param-output-format = Format
batch-param-output-quality = Quality
batch-summary-file-copy = Copy files
batch-summary-file-move = Move files

## Errors
batch-err-recipe-json = The recipe is not valid JSON: { $error }
batch-err-recipe-not-object = The recipe must be a JSON object.
batch-err-recipe-format = Unknown recipe format “{ $format }”.
batch-err-recipe-newer = The recipe has version { $version }; this version of Firepixel reads up to version { $supported }.
batch-err-recipe-missing = No recipe given.
batch-err-no-files = No files given.
batch-err-conflict-value = Unknown conflict rule “{ $value }” (rename, skip, overwrite, ask).
batch-err-regex = Invalid regular expression: { $error }
batch-err-format-unavailable = The { $format } format cannot be written yet.
batch-err-unknown-format = Unknown file format: { $path }
batch-err-empty-file = The file is empty.
batch-err-empty-page = The image has no pixels.
batch-err-no-frames = The animation has no frames.
batch-err-not-image = Not an image file: { $path }
batch-err-not-collect = The step { $id } is not a collecting step.
batch-err-worker-panic = Batch processing stopped unexpectedly.
err-backend-batch-runtime = batch processing (fp-batch)

## Recipe check
batch-issue-unknown-step = Unknown step “{ $id }” (skipped, settings are kept).
batch-issue-final-step = “{ $id }” is a fixed end card and does not belong in the step list.
batch-issue-missing-folder = Please choose an output folder.
batch-issue-unknown-format = Unknown output format “{ $format }”.
batch-issue-copy-only-combined = “Copy/Move Only” works only without image steps and with “Save As: same as source”.
batch-issue-param-type = Invalid value for “{ $param }”.
batch-issue-param-range = “{ $param }” must be between { $min } and { $max }.
batch-issue-param-enum = “{ $param }” must be one of: { $values }.
batch-issue-params-not-object = The parameters must be a JSON object.

## Notes on names and targets
batch-hint-invalid-chars = Invalid characters replaced
batch-hint-reserved-name = Reserved name, “_” appended
batch-hint-truncated = Name shortened
batch-hint-empty-name = Empty name, “_” used
batch-hint-leading-dot = Leading dot replaced
batch-hint-exif-date-missing = EXIF date missing, modification date used
batch-hint-conflict = Conflict: name already taken
batch-hint-overwrite = Existing file will be overwritten
batch-hint-replace-original = Original will be replaced (backup in “Original Files”)
batch-hint-backup-forced = Backup turned on because the original is replaced
batch-hint-skipped = Skipped
batch-hint-long-path = Path longer than 260 characters
batch-hint-format-fallback = Source format cannot be written, PNG used
batch-hint-same-fallback-png = The { $format } format cannot be written; PNG is used instead.

## Messages
batch-msg-cancelled = Canceled.
batch-msg-summary = { $done } done, { $skipped } skipped, { $failed } failed · { $before } → { $after }
batch-msg-cancelled-summary = Canceled: { $done } done, { $rest } not processed
batch-msg-dry-run-summary = Dry run: { $planned } of { $total } files would be written, { $skipped } skipped
batch-warn-first-frame-only = { $format } does not store animation; only the first frame is written.
batch-warn-not-applicable = “{ $step }” does not apply to this file.
batch-warn-preview-skipped = “{ $step }” is not calculated in the preview.
batch-warn-unchanged = Unchanged (same name in the same place).

## Command registry (batch.*)
cmd-batch-run = Run Batch
    .description = Runs a batch recipe on a list of files and returns the report
cmd-batch-plan = Plan Batch
    .description = Plans source and target of every file with conflicts, without writing
cmd-batch-preview = Batch Preview
    .description = Calculates the before and after preview of a file
cmd-batch-steps = Batch Steps
    .description = Catalog of all batch steps with parameters
cmd-batch-validate = Check Batch Recipe
    .description = Checks a batch recipe and reports errors and warnings per step
cmdtext-liste-der-dateien = List of files
cmdtext-ordner-mit-den-dateien = Folder with the files
cmdtext-unterordner-einbeziehen = Include subfolders
cmdtext-rezept-objekt-oder-pfad-einer-rezept-datei = Recipe (object) or path of a recipe file
cmdtext-nur-planen-nichts-schreiben = only plan, write nothing
cmdtext-zahl-der-arbeits-threads = Number of worker threads
cmdtext-ausgabeordner-ueberschreibt-das-ziel-des-rezepts = Output folder (overrides the recipe target)
cmdtext-konfliktregel-ueberschreibt-das-rezept = Conflict rule (overrides the recipe)
cmdtext-datei = File
cmdtext-laengste-seite-der-vorschau-in-pixeln = longest side of the preview in pixels

## Command line: firepixel-cli batch --recipe / --steps
batch-cli-about = Apply a batch recipe to files and folders (modern batch mode)
batch-cli-examples =
    Examples:
      firepixel-cli batch --recipe web.fpbatch.json --in photos --recursive
      firepixel-cli batch --recipe web.fpbatch.json a.png b.jpg --out output --dry-run
      firepixel-cli batch --steps --json
    Exit code: 0 all done, 1 at least one error, 2 wrong call, 3 canceled.
batch-cli-arg-recipe = Recipe file (.fpbatch.json) or JSON text
batch-cli-arg-steps = Show the catalog of all steps
batch-cli-arg-in = Input folder
batch-cli-arg-recursive = Include subfolders
batch-cli-arg-files = Files or folders
batch-cli-arg-out = Output folder (overrides the recipe target)
batch-cli-arg-conflict = Conflict rule: rename, skip or overwrite
batch-cli-arg-dry-run = Only plan, write nothing
batch-cli-arg-threads = Number of worker threads
batch-cli-arg-report = Write the report (JSON) to this file
batch-cli-arg-json = Machine-readable output (progress as NDJSON on stderr)
batch-cli-arg-lang = Output language (de, en …)
batch-cli-err-args = Wrong call: { $error }
batch-cli-skipped-file = Skipped (unsupported format): { $path }
batch-cli-progress-done = [{ $n }/{ $total }] { $source } → { $target }
batch-cli-progress-skipped = [{ $n }/{ $total }] { $source }: { $reason }
batch-cli-progress-failed = [{ $n }/{ $total }] { $source }: Error: { $error }
batch-cli-report-written = Report: { $path }
