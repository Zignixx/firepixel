# General texts (fp-i18n core). Area files: see ..\..\README.md

language-name = English

# number, size and date format
fmt-decimal-separator = .
fmt-size-bytes = { $n ->
    [one] { $n } byte
   *[other] { $n } bytes
}
fmt-date = { $month }/{ $day }/{ $year }
fmt-time = { $hour }:{ $minute }
fmt-datetime = { $date }, { $time }

# common buttons
btn-ok = OK
btn-cancel = Cancel
btn-yes = &Yes
btn-no = &No
btn-close = Close
btn-apply = Apply
btn-browse = Browse…

# units (abbreviations; unit("px") in fp-i18n)
unit-px = px
unit-percent = %
unit-pt = pt
unit-deg = °
unit-in = in
unit-cm = cm
unit-mm = mm
unit-ppi = Pixels/Inch
unit-ppcm = Pixels/cm
unit-pixels = Pixels
unit-ms = ms
unit-s = s
unit-kb = KB
unit-mb = MB
unit-gb = GB
unit-tb = TB

# key names in shortcuts (shortcut_label in fp-i18n)
key-ctrl = Ctrl
key-alt = Alt
key-shift = Shift
key-enter = Enter
key-esc = Esc
key-space = Space
key-backspace = Backspace
key-tab = Tab
key-del = Del
key-ins = Ins
key-home = Home
key-end = End
key-pageup = Page Up
key-pagedown = Page Down
key-left = Left
key-right = Right
key-up = Up
key-down = Down
key-num = Num
key-acute = Acute
key-circumflex = Circumflex

# language selection (View ▸ Language, Preferences)
lang-menu = La&nguage
lang-open-folder = Open Language Folder...
lang-pseudo = Pseudo-language (qps)
lang-label = Language:
lang-system = System language ({ $name })
lang-changed = Language: { $name }
lang-folder-hint = More languages: .ftl files in { $path }
lang-dev-options = Developer options (pseudo-language in menu)

# test text with placeholder and plural (core tests)
test-layers = { $n ->
    [one] { $n } layer
   *[other] { $n } layers
}
test-unsaved = Save changes to “{ $name }”?
test-zoom = Zoom { NUMBER($z, maximumFractionDigits: 1) }%
