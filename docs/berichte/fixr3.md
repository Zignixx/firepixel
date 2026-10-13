# FIX-R3 – die 4 roten Tests aus GT-0e (13.10.2026)

Slot `fp-s3` / `target-s3`, Basis `main` @ `5868bb7`. Commit auf main: `fd087ae`.

## Umgesetzt

1. **`firepixel-cli` `tests::parameterbeschreibungen_in_de_und_en`** – alle 26 fehlenden Schlüssel (28 Fundstellen,
   zwei Texte doppelt) in `crates\fp-i18n\locales\{de,en}\command_texts.ftl` nachgetragen: `commands.run`/`runScript`
   (prompt-Antworten), `edit.cropDocument`, `modify.transform.numericTransform` „Attribute skalieren (Strichbreite
   mit)“ (FIX-G1), `modify.exitBitmapMode` (zwei Parameter, FIX-G4), `bitmap.paint` (9), `bitmap.floodFill`,
   `color.sample`, `bitmap.retouch` (5), `pi.fill` (3), `pi.brush`/`pi.tool.brush`, `pi.brush.save`.
   Englische Begriffe wie in der Oberfläche (Edit Stroke, Smudge color, Entire document). Test grün.
2. **`clipping_audit` – Wörterbuchpfad in „Rechtschreibung einrichten“:** Mit `FP_RAND_DEBUG` geprüft: das Feld läuft
   **nicht** über – der Pfad ist schon wie im Windows-Eingabefeld des Originals auf das Feld beschnitten
   (Clip [357, 401 – 638, 418] = Feld [358, 402 – 637, 420]); der Prüfer meldet richtig `hinweis-feldinhalt`. Rot war
   eine zweite Meldung `ueberlappt-element` für **denselben** Text und **dasselbe** Feld: der Pfad ist so lang, dass
   die Mitte seines (unsichtbaren) Textrechtecks rechts außerhalb des Felds liegt. Behoben im Prüfer
   (`tests\clipping_audit.rs`): liegt der Clip eines Texts ganz in einem Eingabefeld, ist es dessen eigener,
   scrollender Inhalt (bleibt Hinweis wie dokumentiert, keine Überlappung). Kein Lockern für echte Überläufe: ein
   Text, der sichtbar über ein fremdes Element läuft, hat einen Clip außerhalb des Felds und wird weiter gemeldet.
   Am Programm war nichts zu ändern (Darstellung = Original: Anfang des Pfads sichtbar, Rest abgeschnitten).
3. **`dialogs_i18n` – „HTML-Einrichtung“, Register Allgemein (qps):** `crates\fp-ui\src\dialogs\html_setup.rs`: die
   linke Spalte ist jetzt mindestens 190 px, bei längeren Texten so breit wie „HTML-Kommentare einschließen“ /
   „Kleinbuchstaben“ samt Kästchen; rechte Spalte und Dialogbreite rücken mit. DE/EN unverändert (Abstand 208 px,
   neuer Test `tests\fixr3.rs` prüft das und, dass in qps der Kommentartext vor der rechten Spalte endet).
4. **`menu_fixes_fv13::secondary_browser_preview_needs_configured_browser`:** Die Testhilfe
   `gesamt_common::external` stufte alle Befehle mit „browser“ als „öffnet externe Programme – GUI-Sitzung“ ein
   (`Outcome::Manual`). Die Browser-Vorschau startet den Browser ausschließlich über `fp_engine::shell`
   (`export.rs`, `shell_open`); `file.previewInBrowser.*` gilt jetzt als sicher, nachdem `fp_engine::shell::disable()`
   für den Testprozess gesetzt ist – die Vorschauseite entsteht, kein Fremdprogramm startet. Test grün ohne
   `FP_NO_SHELL_OPEN`. Andere riskante Befehle (Hilfe, Drucken, Mail, Brücke …) unverändert.

## Geprüft

- `cargo test -p firepixel-cli` (alle grün), `cargo test -p fp-i18n`, `cargo test -p fp-neutral`
- `fp-ui --test all`: `clipping_audit::` (Standardlauf grün), `menu_fixes_fv13::` (9/9), `fixg2::`, `fixr3::`,
  `html`-Tests, `gesamt_oberflaeche::` (grün, 274 s), `i18n_keine_festen_texte::`
- `fp-ui --test dialogs_i18n` (4/4)
- `cargo check --workspace --all-targets` ohne Warnungen

## PARITAET / Nachprüfung

- Keine PARITAET-Zeile betroffen.
- Nachprüfung: „HTML-Einrichtung“ (Datei ▸ HTML-Einrichtung, Register Allgemein) in DE und EN – Aussehen wie vor
  FIX-R3 (zwei Spalten, rechte Spalte bei gleicher Lage).

## Neue Befunde

- keine
