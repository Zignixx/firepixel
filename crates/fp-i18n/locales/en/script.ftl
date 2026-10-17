# Messages from fp-script (JSF runtime, auto shapes, files) – agent AU
err-script-autoshape-runtime = Auto shapes ({ $part }): { $error }
err-script-autoshape = Auto shape: { $error }
err-script-autoshape-script = Auto shape script ({ $operation }): { $error }
err-script-shape-missing = shape is missing (name of the auto shape)
err-script-shape-unknown = Unknown auto shape “{ $name }” – autoShape.list shows all
err-script-no-layer = No layer to insert into (layer locked or Web Layer active?)
err-script-param-missing = { $param } is missing
err-script-params-missing = params is missing (object key → value)
err-script-not-autoshape = Not an auto shape
err-script-control-point = Control point { $index } does not exist (the shape has { $count })
err-script-op-unknown = Unknown operation “{ $op }” ({ $ops })
err-script-format-needs-io = Format .{ $ext } cannot be read without fp-io
err-script-png-only = { $path }: only PNG without fp-io
err-script-page-empty = Page is empty or missing
err-script-export-unsupported = Export format .{ $ext } is not supported
err-script-export-needs-feature = Export format .{ $ext } requires fp-export (feature “formats”)
err-script-open-failed = Opening failed
err-script-page-missing = Page is missing

# >>> UE-3
# UE-3
err-script-element-not-autoshape = element { $id } is not an auto shape
# <<< UE-3

# >>> UE-3 HR
# Texts for JSF error codes (Errors.*), UE-3
err-script-hr-ebad-param = A parameter is invalid.
err-script-hr-ebad-param-type = A parameter has the wrong type.
err-script-hr-eout-of-mem = Not enough memory.
err-script-hr-efile-not-found = The file was not found.
err-script-hr-edisk-full = The disk is full.
err-script-hr-egeneric-error-occurred = An error occurred.
err-script-hr-eno-sliceable-elems = There are no objects that slices can be created from.
err-script-hr-esharing-violation = The file is being used by another program.
err-script-hr-eunknown-reader-format = The file format is unknown.
err-script-hr-ebad-file-contents = The file contents are damaged.
err-script-hr-efile-is-read-only = The file is read-only.
err-script-hr-eno-files-selected = No files were selected.
err-script-hr-ebad-selection = The selection does not fit this operation.
err-script-hr-ebad-js-version = The script version is not supported.
err-script-hr-eno-active-document = No document is open.
err-script-hr-eno-active-selection = Nothing is selected.
err-script-hr-could-not-complete-your-request = The request could not be completed.
err-script-hr-internal = An internal error occurred.
# <<< UE-3 HR
# JSF runtime: API member not implemented (UE-5)
err-script-not-implemented = not implemented: { $member }
# Built-in command scripts in the Commands menu, read with fp.t(…) in the UI language (MN-5)
builtin-cmd-frame-layer = Frame
builtin-cmd-twist-copies = Number of copies
builtin-cmd-text-width = Width in pixels
builtin-cmd-alt-text = ALT text for the selected hotspots and slices
builtin-cmd-resize-percent = New size in percent
builtin-cmd-screenshot-unavailable = Screenshot: Firepixel cannot capture the screen itself.
builtin-cmd-screenshot-howto = Capture the image with the system (Windows: Win+Shift+S) and paste it with File ▸ New and Edit ▸ Paste (Ctrl+V).
builtin-cmd-demo-no-document = Demo Current Document: Open a document first.
builtin-cmd-symbol-script-no-instance = Create Symbol Script: Select an instance of a graphic symbol first.
builtin-cmd-symbol-script-unsupported = Create Symbol Script: Firepixel cannot create symbol scripts for the symbol “{ $name }” yet. Edit the properties of the instance in the Symbol Properties panel.
builtin-cmd-auto-mask-no-selection = Auto Vector Mask: Select one or more objects first.
builtin-cmd-shared-palette = Shared Palette: Firepixel does not create a shared color table for multiple slices. To make slices use the same colors, set the same color table for each slice in the Optimize panel.
