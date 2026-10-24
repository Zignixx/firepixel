# Command line firepixel-cli: help (`cli-<command>`, `.examples`, `cli-<command>-arg-<option>`, `.value`), messages, language check – UE-3
cli-root = Firepixel Studio – command line (edit FW-PNG, render, scripts, MCP server for AI agents)
    .examples = 
        Examples:
          firepixel-cli commands --filter align
          firepixel-cli info draft.fw.png --json
          firepixel-cli run draft.fw.png --cmd select.all --cmd modify.align.left --out aligned.fw.png
          firepixel-cli run --new 400x300 --cmd draw.rectangle --params '{"{"}"rect":[20,20,200,120],"fillColor":"#3366ff"{"}"}' --out new.fw.png
          firepixel-cli render draft.fw.png --zoom 0.5 --out preview.png
          firepixel-cli batch --actions actions.json --in input --out output
          firepixel-cli script macro.jsf draft.fw.png --out result.fw.png
          firepixel-cli mcp                      (MCP over stdio, e.g. for Claude Code)
          firepixel-cli mcp --http --port 47800  (MCP over HTTP on 127.0.0.1 with token)
          firepixel-cli mcp --bridge             (stdio MCP to the running user interface, firepixel.exe --mcp)

        Command names: <menu>.<item> in camelCase (file.save, modify.transform.flipHorizontal, filter.blur.gaussian)
        plus draw.*, element.*, layer.*, page.*, state.*, select.*. Parameters: firepixel-cli commands --filter <name> --json.
cli-run = Apply commands to a file and save
    .examples = 
        Examples:
          firepixel-cli run in.fw.png --cmd select.all --cmd modify.group --out out.fw.png
          firepixel-cli run in.fw.png --cmd filter.blur.gaussian --params '{"{"}"radius":2{"}"}' --out out.fw.png
          firepixel-cli run --new 200x100 --cmd draw.text --params '{"{"}"text":"Hello","anchor":[10,60],"size":32{"}"}' --out text.png
          firepixel-cli run in.fw.png --steps steps.json --out out.fw.png

        --params belongs to the preceding --cmd. --out with the extension .png (without .fw) writes a flat PNG,
        otherwise an FW-PNG. --steps takes a JSON list [{"{"}"cmd": …, "params": {"{"}…{"}"}{"}"}, …] (file or text).
cli-run-arg-file = Input file (.fw.png or .png); not needed with --new
    .value = FILE
cli-run-arg-new = New document WIDTHxHEIGHT[:#color] instead of a file, e.g. 800x600 or 64x64:#00000000
cli-run-arg-cmds = Command name (can be repeated)
cli-run-arg-params = JSON parameters for the preceding --cmd
cli-run-arg-steps = List of steps as a JSON file or JSON text
    .value = JSON|FILE
cli-run-arg-group = All commands as one undo step
cli-run-arg-out = Target file (default: overwrite the input file only with --in-place)
    .value = FILE
cli-run-arg-in-place = Overwrite the input file
cli-batch = Apply the same actions to all files of a folder
    .examples = 
        Example actions.json:
          {"["} {"{"}"cmd": "select.all"{"}"},
            {"{"}"cmd": "modify.canvas.imageSize", "params": {"{"}"percent": 50{"}"}{"}"},
            {"{"}"cmd": "filter.sharpen.sharpen"{"}"} ]

          firepixel-cli batch --actions actions.json --in input --out output
          firepixel-cli batch --actions '[{"{"}"cmd":"modify.transform.flipHorizontal"{"}"}]' --in a --out b --format png
cli-batch-arg-actions = Actions as a JSON file or JSON text
    .value = JSON|FILE
cli-batch-arg-input = Input folder
    .value = FOLDER
cli-batch-arg-output = Output folder (created if needed)
    .value = FOLDER
cli-batch-arg-format = Output format: fwpng (default, editable), png, gif, jpg, webp, avif, bmp, tiff, qoi, ico, apng
cli-batch-arg-filter = Only files whose name contains this text
    .value = TEXT
cli-render = Render a page, region or element as PNG (offscreen)
    .examples = 
        Examples:
          firepixel-cli render file.fw.png --out page.png
          firepixel-cli render file.fw.png --page 1 --state 2 --zoom 2 --out s2.png
          firepixel-cli render file.fw.png --rect 0,0,100,100 --zoom 4 --out region.png
          firepixel-cli render file.fw.png --element 17 --out element.png
cli-render-arg-rect = Region l,t,r,b in document pixels
cli-render-arg-element = Only this element (ID from `info`)
    .value = ID
cli-info = Show the document tree (pages, states, layers, elements with IDs)
    .examples = 
        Examples:
          firepixel-cli info file.fw.png
          firepixel-cli info file.fw.png --json --depth 1
          firepixel-cli info file.fw.png --json --all-pages --types text
cli-info-arg-types = Only these element types (comma-separated: text,path,rectangle,bitmap,group,instance …)
    .value = TYPES
cli-info-arg-custom-data = Output customData
cli-commands = Show the command catalog
    .examples = 
        Examples:
          firepixel-cli commands
          firepixel-cli commands --category modify.align
          firepixel-cli commands --filter blur --json
cli-commands-arg-filter = Search text in name, label, status text, menu
    .value = TEXT
cli-commands-arg-category = Name prefix (draw, modify, filter …)
    .value = CATEGORY
cli-commands-arg-all = Also placeholders without effect
cli-convert = Convert a file (format from the extension)
    .examples = 
        Examples:
          firepixel-cli convert draft.fw.png draft.png         (flat PNG32)
          firepixel-cli convert draft.fw.png draft.jpg         (JPEG through the optimization as in the original)
          firepixel-cli convert layers.psd layers.fw.png       (PSD with layers as FW-PNG)
          firepixel-cli convert photo.png photo.webp
        Read: .fw.png .png .psd .gif .jpg .bmp .tif .tga .webp .jxl .qoi .ico .svg .ai .eps
        Write: .fw.png .png .gif .jpg .webp .avif .bmp .tif .qoi .ico .apng
cli-script = Run JSF JavaScript (.jsf)
    .examples = 
        Examples:
          firepixel-cli script macro.jsf
          firepixel-cli script macro.jsf draft.fw.png --out result.fw.png
          firepixel-cli script --code "fw.createFirepixelDocument({"{"}x:100,y:100{"}"},{"{"}pixelsPerUnit:72,units:'inch'{"}"},'#ffffff').width"
cli-script-arg-script = Script file
    .value = SCRIPT
cli-script-arg-file = Document that is opened first
    .value = FILE
cli-script-arg-code = Script as text instead of a file
cli-script-arg-out = Save the active document afterwards
    .value = FILE
cli-script-arg-arg = Value of the global variable FP_ARG
    .value = VALUE
cli-mcp = MCP server for AI agents (stdio or HTTP on 127.0.0.1 with token)
    .examples = 
        Examples – setup in Claude Code (stdio):
          claude mcp add firepixel -- firepixel-cli mcp
        HTTP (the token is then in %LOCALAPPDATA%\Firepixel\mcp-http.json):
          firepixel-cli mcp --http --port 47800
          claude mcp add --transport http firepixel http://127.0.0.1:47800/mcp --header "Authorization: Bearer <token>"
        Bridge to the running user interface (firepixel.exe --mcp or FIREPIXEL_MCP=1; token in %LOCALAPPDATA%\Firepixel\mcp-bridge.json):
          claude mcp add firepixel-gui -- firepixel-cli mcp --bridge
        Docs: docs/AGENTS.md or MCP resource firepixel://docs/agents.md
cli-mcp-arg-http = Streamable HTTP instead of stdio
cli-mcp-arg-bridge = stdio forwarding to the running Firepixel user interface (bridge mode)
cli-mcp-arg-port = Port on 127.0.0.1 (0 = pick a free one)
cli-mcp-arg-token = Fixed token (at least 16 characters; default: random)
    .value = TOKEN
cli-mcp-arg-token-file = File for address and token (default in the user profile)
    .value = FILE
cli-language = Check language files, create a template for a new language, list languages
cli-language-check = Check language files: missing/orphaned keys, placeholders, syntax, access keys
    .examples = 
        Examples:
          firepixel-cli language check %APPDATA%\Firepixel\Languages         (all languages in the folder)
          firepixel-cli language check Languages\fr                            (one language, folder with *.ftl)
          firepixel-cli language check fr.ftl                                 (one language in one file)
          firepixel-cli language check Languages\fr\menu.ftl                   (only this area)
          firepixel-cli language check --builtin                              (the built-in languages)

        The reference is the built-in German. Errors (exit code 1): syntax, duplicate keys, placeholders
        the program does not know. Warnings: missing/orphaned keys, missing placeholders or
        access keys (&), duplicate access keys in one menu or dialog.
cli-language-check-arg-path = Folder or .ftl file
    .value = PATH
cli-language-check-arg-builtin = Check the built-in languages
cli-language-check-arg-short = Do not list missing keys one by one (count only)
cli-language-check-arg-strict = Treat warnings as errors (exit code 1)
cli-language-template = Create a template for a new language (copy of the English files)
    .examples = 
        Example:
          firepixel-cli language template fr %APPDATA%\Firepixel\Languages
        creates %APPDATA%\Firepixel\Languages\fr\*.ftl (English texts as a starting point).
        Then translate to the right of the =, set language-name, check and restart Firepixel
        (or View ▸ Language ▸ Open Language Folder...).
cli-language-template-arg-code = Language code (e.g. fr, es, pt-BR, ja)
    .value = CODE
cli-language-template-arg-target-dir = Target folder (the subfolder <code> is created in it)
    .value = TARGET_FOLDER
cli-language-template-arg-overwrite = Overwrite existing files
cli-language-template-arg-from = Source language of the template
    .value = FROM
cli-language-list = Available languages (built-in and from the language folders) with completeness
cli-root-arg-json = Output as JSON (machine-readable)
cli-root-arg-lang = Language of the output (e.g. de, en; default: FIREPIXEL_LANG or the system language)
    .value = CODE
cli-render-arg-out = Target file (PNG)
    .value = FILE
cli-render-arg-file = Input file
    .value = FILE
cli-render-arg-page = Page index (default: current page)
    .value = N
cli-render-arg-state = State index
    .value = N
cli-render-arg-zoom = Magnification (1 = 100 %)
    .value = FACTOR
cli-info-arg-file = Input file
    .value = FILE
cli-info-arg-depth = Depth of the tree (0 = layers only)
    .value = N
cli-info-arg-all-pages = All pages in detail
cli-info-arg-page = Page shown in detail
    .value = N
cli-convert-arg-input = Input file
    .value = INPUT
cli-convert-arg-output = Output file (format from the extension)
    .value = OUTPUT
cli-help = Show help
cli-version = Show version
cli-usage = Usage:
cli-usage-options = [OPTIONS]
cli-help-default = default
cli-help-possible-values = possible values
cli-heading-arguments = Arguments
cli-heading-options = Options
cli-heading-commands = Commands
cli-value-command = COMMAND
cli-error = Error: { $message }
cli-err-unknown-argument = unknown option “{ $arg }”
cli-err-unknown-subcommand = unknown command “{ $value }”
cli-err-invalid-value = invalid value “{ $value }” for { $arg }
cli-err-missing-argument = required argument missing: { $arg }
cli-err-missing-subcommand = command missing
cli-err-conflict = { $arg } cannot be combined with { $other }
cli-err-number-of-values = wrong number of values for { $arg }
cli-err-invalid-call = invalid call
cli-err-help-hint = Help: firepixel-cli --help or firepixel-cli <command> --help
cli-err-json = invalid JSON ({ $error }): { $text }
cli-err-new-size = --new expects WIDTHxHEIGHT, not “{ $spec }”
cli-err-width = width “{ $value }”
cli-err-height = height “{ $value }”
cli-err-export-formats = { $error } (formats: .fw.png, { $formats })
cli-err-input-or-new = give an input file or --new
cli-err-params-without-cmd = --params without a preceding --cmd
cli-err-json-list = { $option }: JSON list expected
cli-err-step-without-cmd = step without "cmd"
cli-msg-run-done = { $n } command(s) executed{ $saved }
cli-msg-saved = , saved: { $path }
cli-err-action-without-cmd = every action needs "cmd"
cli-err-format = --format: fwpng or { $formats }, not “{ $value }”
cli-msg-batch-done = { $ok } of { $n } files processed → { $dir }
cli-err-rect = --rect l,t,r,b expected, not “{ $value }”
cli-err-rect-four = --rect needs four numbers
cli-msg-tree-doc = { $title } – { $n } page(s)
cli-msg-quoted = “{ $text }”
cli-msg-tree-page = Page { $index } “{ $name }” { $width }×{ $height } px, { $states } state(s), canvas { $background }
cli-msg-tree-layer = Layer #{ $id } “{ $name }” ({ $kind }{ $hidden })
cli-msg-tree-hidden = , hidden
cli-msg-commands-count = { $n } commands (* = required parameter)
cli-err-script-or-code = give a script file or --code
cli-err-no-document-to-save = no document open to save
cli-msg-script-result = Result: { $result }
cli-msg-script-not-implemented = not implemented: { $list }
cli-err-script = Script error: { $error }
cli-msg-mcp-http-running = Firepixel MCP (HTTP) is running on { $url }
cli-msg-mcp-token = Token: { $token }
cli-msg-mcp-header = Header: Authorization: Bearer { $token }
cli-msg-mcp-token-file = Token file: { $path }
cli-msg-mcp-stop = Stop with Ctrl+C.
cli-msg-mcp-bridge = Firepixel MCP: forwarding to the running user interface ({ $address })
cli-lang-kind-syntax = Syntax errors
cli-lang-kind-doppelt = Duplicate keys
cli-lang-kind-platzhalter-unbekannt = Unknown placeholders (the program does not pass them)
cli-lang-kind-platzhalter-fehlt = Missing placeholders (value is not shown)
cli-lang-kind-zugriffstaste-konflikt = Access key conflicts (same && key in the same menu/dialog)
cli-lang-kind-zugriffstaste-fehlt = Access key (&&) missing
cli-lang-kind-fehlt = Missing keys (fallback to English)
cli-lang-kind-verwaist = Orphaned keys (do not exist in the program)
cli-lang-no-name = without language-name
cli-lang-summary = Language { $code } ({ $name }): { $keys } keys, { NUMBER($percent, minimumFractionDigits: 1, maximumFractionDigits: 1) } % translated, { $missing } missing – { $files } file(s)
cli-lang-result = Result: { $errors } errors, { $warnings } warnings
cli-lang-errors-warnings = { $errors } errors, { $warnings } warnings
cli-lang-path-missing = give a path (folder or .ftl file) or --builtin
cli-lang-bad-code = “{ $code }” is not a valid language code (examples: fr, es, pt-BR, zh-TW)
cli-lang-no-builtin = no built-in language “{ $code }”
cli-lang-template-header = 
    # Firepixel – language “{ $code }” (template from “{ $from }”, file { $file }).
    # Translate to the right of the =; keep the keys on the left, placeholders {"{"} $name {"}"} and selector variants [one]/[other].
    # & marks the access key (Alt+letter), && is a literal &. Check: firepixel-cli language check <folder>
cli-lang-template-skipped = , { $n } existing skipped (--overwrite)
cli-lang-template-done = 
    Template for “{ $code }” in { $dir }: { $n } file(s) written{ $skipped }.
    Next steps: set language-name in common.ftl, translate, then
      firepixel-cli language check "{ $dir }"
cli-lang-col-code = Code
cli-lang-col-name = Name
cli-lang-col-origin = Source
cli-lang-col-translated = translated
cli-lang-builtin = built-in
cli-lang-folder = language folder
cli-lang-pseudo = Pseudo-language
cli-lang-folders = Language folders:
cli-lang-folder-missing = (does not exist)
cli-check-error = Error
cli-check-warning = Warning
cli-check-syntax = { $kind } – “{ $text }”
cli-check-duplicate = already in { $file }:{ $line }
cli-check-source-text = Source text: “{ $text }”
cli-check-allowed = { $vars } (allowed: { $allowed })
cli-check-access-missing = Source text “{ $text }” has an access key (&)
cli-check-access-conflict = Key “{ $key }” used several times in group { $group }: { $list }
cli-check-none = none
cli-check-not-utf8 = { $path }: not a UTF-8 file (please save as UTF-8)
cli-check-no-path = { $path }: neither a file nor a folder
cli-check-no-ftl = { $path }: no .ftl files found
