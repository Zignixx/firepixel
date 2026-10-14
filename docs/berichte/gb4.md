# G-B4 – Paket B4 aus der Mess-Sitzung B-2b

Stand 15.10.2026. Grundlage `docs\berichte\b2b.md` (Unterschiede BA-022, 023, 027, 032, 127). Slot `fp-s3`.

## Umgesetzt (Commit `8b6cc87`)

| BA | Änderung | Codestelle |
|---|---|---|
| BA-022 | Tab-Folge der Filterdialoge wie gemessen: D47 Bewegungsunschärfe und D49 Radial OK › Abbrechen › Vorschau › Felder (neue Folge `SideOrder::ButtonsFirst`), D48 Gauß Feld › OK › Abbrechen › Vorschau › Schieber (`SideOrder::FieldsButtonsSliders`, auch im Live-Effekt-Dialog); D45 und D50 waren schon wie gemessen. Startfokus OK (D45/D47/D49/D50) bzw. Feld (D48) bestätigt. OK mit „999“ in D45 schließt ohne Meldung mit dem geklemmten Wert 400 (per Klick und per Enter) – war schon so, jetzt mit Test | `fp-ui dialogs\form.rs`, `catalog.rs`, `effects.rs` |
| BA-023 | Datei-Info wie im Original: Register Beschreibung, IPTC, IPTC Extension, Kameradaten, GPS-Daten, Videodaten, Audiodaten, ▸ (nur „Beschreibung“ bedienbar); Felder Dokumenttitel, Autor, Autorentitel, Beschreibung, Bewertung (5 Sterne, Klick auf den aktuellen Stern = 0), Verfasser der Beschreibung, Stichwörter mit Hinweis, Copyright-Status (Unbekannt/Urheberrechtlich geschützt/Gemeinfrei), Copyright-Informationen, URL + „Gehe zu URL…“ (nur bei http/https); Fußzeile Erstellt/Geändert/Anwendung/Format; Knöpfe Voreinstellungen · Importieren… ▾ · OK · Abbrechen. `file.fileInfo` hat neue Parameter `authorTitle`, `captionWriter`, `rating` (0–5), `copyrightStatus` (`unknown`/`copyrighted`/`publicDomain`), `copyrightUrl` (XMP `photoshop:AuthorsPosition`, `photoshop:CaptionWriter`, `xmp:Rating`, `xmpRights:Marked`, `xmpRights:WebStatement`; die Standard-Namensräume werden zur Laufzeit zusammengesetzt, damit kein Herstellername im Quelltext/Programm steht) | `fp-ui dialogs\file_info.rs`, `fp-engine commands\file_info.rs` |
| BA-027 | (1) Inspektor: das ◊ links im Reiter „Eigenschaften“ ist jetzt ein Knopf und schaltet volle ↔ halbe Höhe (eingeklappt → volle Höhe); Reiter-Kontextmenü Schließen · Registerkartengruppe schließen · Minimieren (ohne Höhe-Einträge). (2) Optimieren-Menü: „Sperrung aller Farben aufheben“ nur mit gesperrter, „Alle Bearbeitungen entfernen“ nur mit bearbeiteter Farbe aktiv. Die Haken bei „Nicht verwendete Farben entfernen“/„Farbfeld-Rückmeldung anzeigen“ waren im Code schon da (Vorgabe an) und erscheinen im echten Fenster bei GIF Web 216 – die fehlenden Haken im Bild `fp_g4_optmenu_gif.png` ließen sich nicht nachstellen (vermutlich abweichendes Profil der Messinstanz) | `fp-ui inspector\mod.rs`, `menu.rs`; `fp-engine commands\panel.rs` |
| BA-032 | Analyseergebnisse mit den vier Absätzen des Originals, Ziel „Webeditor“ (EN „A web editor“), Vorschlag der Bildvorschau = eingebaute Vorgaben „GIF Web 216“ und „JPEG – Höhere Qualität“ (80). Mit frischem Profil über die Brücke gegengeprüft: GIF Web 216 / 216 Farben und JPEG Qualität 80 | `fp-ui dialogs\export_wizard.rs` |
| BA-127 | Vorgabe und Rückfall des Kürzelsatzes „Web Standard“ (`fp_engine::prefs::DEFAULT_SHORTCUT_SET`, auch in `Session::shortcut_set`); die Definition in `prefs.json` war schon „Web Standard“, der Name enthält keinen Produktnamen | `fp-engine prefs.rs`, `session.rs` |
| Zusatz (RUN-5) | Brücke: `ui_action {"panel": …}` erreicht jedes Bedienfeld des Fenster-Menüs, auch Ausrichten, Pfad, Farbpalette, Sonderzeichen, Bildbearbeitung, Symboleigenschaften und Eigenschaften für automatische Form – Name wie im Menü-Befehl (`window.align`, `window.path`, `window.colorPalette`, `window.specialCharacters`, `window.imageEditing`, `window.symbolProperties`, `window.autoShapeProperties`), mit oder ohne `window.`, oder die Original-Befehls-ID (z. B. `48601`). `ui_state.visible_panels` meldet diese Bedienfelder unter denselben Namen | `fp-ui panels\mod.rs` (`PanelId::bridge_name`/`from_bridge_name`), `bridge.rs` |

Texte DE+EN in `dialogs.ftl` (Datei-Info, Exportassistent), `inspector.ftl` (`pi-height-toggle`), `menu.ftl` (`pmenu-minimize`). Entfernt: `dlg-wiz-res-analysis`, `dlg-fileinfo-group-*`, `-path`, `-pages`, `-symbols`, `-version`. i18n-Ausnahme `export_wizard.rs | JPEG – Höhere Qualität` (Kennung der Vorgabe wie in `preview.rs`).

## Prüfungen

- Tests: `fp-ui --test all gb4::` (10: BA-022 ×5, BA-023, BA-027 ×2, BA-032, Brücke), `fp-engine --test all gb4::` (3) und `--lib file_info` (neuer Rundlauf-Test); angepasst `panel_menus_mn3` (Reitermenü, Sperrung grau), `export_e3` (GIF Web 216), `dialogs_dlg11` (8 Eingabefelder). Grün auch `dialogs_dlg5/6/10`, `form_form1`, `fixg2`, `batch_ui`, `ue3_befunde`, `gesamt_kuerzel`, `f14_set_labels`, Engine `registry`, `phase5`, `commands`, `tools`, `canvas_tools`, `all_commands`, `--lib optimize`.
- `cargo check --workspace --all-targets` ohne Warnungen (auch nach dem Cherry-Pick auf main mit G-B3/G-B5/RUN-5).
- `cargo test -p fp-i18n`, i18n-Scanner (0 feste Texte), `cargo test -p fp-neutral` grün.
- Echtes Fenster, Brücke (Slot-Build, eigenes Profil im Scratchpad, DE und EN): Datei-Info DE/EN (Register, Felder, Sterne, Knöpfe im Fenster), Exportassistent bis „Analyseergebnisse“ DE/EN und Bildvorschau GIF Web 216 / JPEG – Höhere Qualität, Optimieren-Menü bei GIF Web 216 (Haken und Freigaben wie im Original), ◊ halbiert den Inspektor, Reiter-Kontextmenü mit drei Einträgen, `ui_action {panel: "path"}` und `{panel: "48601"}`. Instanzen danach über „Beenden“ geschlossen.

## PARITAET – „umgesetzt, Nachprüfung ausstehend“

D10 Datei-Info, D45–D50 (Tab-Folge), Exportassistent (re\18 §8), Inspektor-Reiter (Volle/Halbe Höhe), Optimieren-Bedienfeldmenü bei GIF, Kürzelsatz-Vorgabe.

## Nachprüfung (für NACHPRUEFUNG.md)

1. D47/D49/D48: Dialog öffnen, Tab fünfmal → Folge wie oben; D45 „999“ + OK → schließt, erneutes Öffnen zeigt 400.
2. Datei-Info neben dem Original: Abstände/Höhen der Felder und Register, Stern-Grafik, Liste Copyright-Status (Einträge des Originals ablesen, DE „Gemeinfrei“ vs. „Public Domain“), Esc per echter Taste.
3. Exportassistent: Zeilenumbruch der vier Absätze auf der Ergebnisseite (Original 401 × 299).
4. Inspektor: ◊ klicken → halbe Höhe, erneut → volle Höhe; Doppelklick auf den Reiter weiter reihum.
5. Frisches Profil: Tastenkürzel wie „Web Standard“ (z. B. Strg+Y = Wiederholen, Strg+1 = 100 %).

## Offen / neue Befunde

- BA-023 Rest (V): Register IPTC … Audiodaten sowie „Voreinstellungen“ und „Importieren… ▾“ sind sichtbar, aber gesperrt (keine Funktion nachgebaut). Entscheidung: nachbauen oder so lassen.
- BA-032: „Das erste Bild ist kleiner.“ steht fest im Text wie im gemessenen Fall; ob das Original den Satz je nach Größe tauscht, ist ungemessen.
- `commands\file.rs:428`: Löschen des aktiven eigenen Kürzelsatzes fällt auf den ersten eingebauten Satz („Firepixel“) zurück, nicht auf „Web Standard“ – Verhalten des Originals ungemessen.
- Im Hauptbaum liegt eine fremde, nicht versionierte Datei `doc2.rs` im Wurzelordner (nicht von G-B4).
