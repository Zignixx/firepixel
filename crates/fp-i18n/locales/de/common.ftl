# Allgemeine Texte (Kern fp-i18n). Bereichsdateien: siehe ..\..\README.md

language-name = Deutsch

# Zahlen-, Größen- und Datumsformat
fmt-decimal-separator = ,
fmt-size-bytes = { $n } Byte
fmt-date = { $day }.{ $month }.{ $year }
fmt-time = { $hour }:{ $minute }
fmt-datetime = { $date }, { $time }

# häufige Knöpfe
btn-ok = OK
btn-cancel = Abbrechen
btn-yes = &Ja
btn-no = &Nein
btn-close = Schließen
btn-apply = Übernehmen
btn-browse = Durchsuchen…
btn-help = &Hilfe

# Einheiten (Kürzel; unit("px") in fp-i18n)
unit-px = px
unit-percent = %
unit-pt = pt
unit-deg = °
unit-in = in
unit-cm = cm
unit-mm = mm
unit-ppi = Pixel/Zoll
unit-ppcm = Pixel/cm
unit-pixels = Pixel
unit-ms = ms
unit-s = s
unit-kb = KB
unit-mb = MB
unit-gb = GB
unit-tb = TB

# Tastennamen in Kürzeln (shortcut_label in fp-i18n; Wortlaut wie im Original, gemessen in allen Menüs)
key-ctrl = Strg
key-alt = Alt
key-shift = Umschalt
key-enter = Eingabe
key-esc = Esc
key-space = Leertaste
key-backspace = Rück
key-tab = Tab
key-del = Löschen-Taste
key-ins = Einfg
key-home = Pos1
key-end = Ende
key-pageup = Bild auf
key-pagedown = Bild ab
key-left = Nach-links-Taste
key-right = Nach-rechts-Taste
key-up = Nach-oben-Taste
key-down = Nach-unten-Taste
key-num = Num
key-acute = AKUT
key-circumflex = ZIRKUMFLEX

# Sprachwahl (Ansicht ▸ Sprache, Voreinstellungen)
lang-menu = Spra&che
lang-open-folder = Sprachordner öffnen...
lang-pseudo = Testsprache (qps)
lang-label = Sprache:
lang-system = Systemsprache ({ $name })
lang-changed = Sprache: { $name }
lang-folder-hint = Weitere Sprachen: .ftl-Dateien in { $path }
lang-dev-options = Entwickleroptionen (Testsprache im Menü)

# Testtext mit Platzhalter und Mehrzahl (für die Kerntests)
test-layers = { $n ->
    [one] { $n } Ebene
   *[other] { $n } Ebenen
}
test-unsaved = Änderungen an „{ $name }“ speichern?
test-zoom = Zoom { NUMBER($z, maximumFractionDigits: 1) } %
