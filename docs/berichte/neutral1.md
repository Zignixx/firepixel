# NEUTRAL-1: Paket P-NEUTRAL (13.10.2026)

## Umgesetzt

`crates\fp-neutral` prüft jetzt nachweislich auch Dokumente und Bilder (§1a):

- **Textdateien:** Die Prüfung `projekt_ist_neutral` scannt wie bisher alle Dateien und Namen unter
  `firepixel\` außer `target*` und `.git`. `vendor\` bleibt mit drin, denn es ist sauber und §1a ist
  strenger. Neu ist der Test `scan_covers_documents_and_images`: Er schlägt fehl, sobald `STATUS.md`,
  `docs\*.md`, `docs\berichte\`, `docs\funktionstest\` oder die PNGs unter `docs\screenshots\` nicht
  mehr erfasst werden. Dazu kommt `fp_neutral::scan_with_files` (liefert die geprüften Dateien) und
  `fp_neutral::fnv64`.
- **Erlaubnisliste `ALLOWED`** in `tests\neutral.rs`, mit Pfad und Begründung. Der Test erzwingt eine
  Begründung und verbietet Einträge unter `docs/screenshots`. Nicht mehr gebrauchte Einträge lassen
  den Test rot werden. Die Liste ist derzeit **leer**.
- **Bilder:** Der Test `screenshots_show_no_blocked_names` liest den sichtbaren Text aller PNGs unter
  `docs\screenshots\` per Windows-Texterkennung aus (`tests\ocr_words.ps1`, kennt selbst keine Namen)
  und prüft jede Zeile und jedes Wort mit `fp_neutral::find_in_bytes`. Gemeldet werden Pfad und
  Wortrechteck, nie der Text selbst. Saubere Bilder merkt sich ein Zwischenspeicher in
  `CARGO_TARGET_TMPDIR` (FNV-1a plus Länge). Der erste Lauf über 596 Bilder dauert ca. 100 s, danach
  ca. 1 s. Ohne Texterkennung (Exit-Code 3) wird der Test übersprungen. Bei anderen Fehlern schlägt
  er fehl. Dateinamen und Bytes der Bilder deckt weiterhin der allgemeine Scan ab.
- **Gegenprobe:** Ein ungeschwärztes Bild aus main wurde vorübergehend unter `docs\screenshots`
  abgelegt. Der Test wurde rot und hat alle 6 Stellen gemeldet. Danach ist das Bild wieder entfernt.

## Funde behoben

- Text (`STATUS.md`, `docs\*.md`, `docs\berichte`, `docs\funktionstest`): **0 Funde**. Die Treffer
  vom 12.10. waren schon behoben.
- Bilder: **10 Bilder mit 16 Wortstellen** neu erzeugt. Die Wörter sind mit Hintergrundfarbe
  übermalt und gepunktet umrandet, wie das vorhandene „[Name entfernt]“. Alle Bilder liegen unter
  `docs\screenshots\paritaet\` und zeigen die Oberfläche des Originals:
  `b_D02`, `b_D04`, `b_D05` (Dateityp in Dateidialogen), `b_D16` (Voreinstellungen), `b_D24` und
  `pm5_D24` (Wörterbuchname), `b_D25`, `b_D33`, `b_D34` (Titelleiste von Meldungen), `b_M10`
  (Hilfe-Menü, 6 Stellen).
- Schriftlisten: Kein Bild zeigt Systemschriften mit gesperrtem Namen. Diese Schriften sind auf dem
  Rechner installiert, die Sperre von FV-12 hat also gegriffen.

## Neue Befunde

- Grenze der Texterkennung: Kleine, stark stilisierte Schrift wie Logos oder Programmsymbole
  erkennt sie nicht. Ein Logo des Originals in einem Bild würde sie also nicht melden. Neue Bilder
  vom Original sollten deshalb beim Erzeugen über die bekannten Texte geprüft werden, so wie bei
  FV-12.

## Nachprüfung

`cargo test -p fp-neutral` muss grün sein. Zum Gegentest ein Bild mit dem Namen unter
`docs\screenshots` ablegen, dann muss `screenshots_show_no_blocked_names` rot werden.
