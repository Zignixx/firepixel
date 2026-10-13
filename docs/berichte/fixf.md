# FIX-F – bekannte Befunde aus FIX-B/C/D, PAR-X-INSP-2, PAR-X-TOOLS-2, BATCH-11b (13.10.2026)

Slot `fp-s2`, Build `target-s2`. Nur belegte Punkte, keine neuen Messungen im Original.

## Umgesetzt (Commits auf main)

| Punkt | Commit | Was |
|---|---|---|
| 1 `i18n_menue` rot | b55ce76 | `menu.ftl` de/en: `cmd-select-liveEdge` („Weiche Auswahlkante“ / „Feather Selection Edge“), `cmd-select-moveMarquee` („Verschieben“ / „Move Marquee“), `cmd-select-movePixels` („Verschieben“ / „Move Pixels“). Deutsch = Beschriftung im Register. |
| 2 `menu_fixes_fv13`/`menu_mn4` nur parallel rot | cba7e3c | Ursache: gemeinsamer Befehlsordner im Testprofil (z. B. `dialogs_dlg4` speichert kurz einen eigenen Befehl), dazu der 2-s-Zwischenspeicher des Befehle-Menüs (`menu.rs::command_submenus`), der den Stand eines anderen Tests weitergab. Neu: `fp_engine::commands::misc::set_thread_commands_dir` (Befehlsordner nur für den Thread, Vorrang vor dem Prozessordner), `gesamt_common::own_commands_dir()` (eigener Ordner je Test unter dem Testprofil mit den mitgelieferten Befehlen); beide Testmodule nehmen über ein lokales `fixture()` je Test einen eigenen Ordner. Der Menü-Zwischenspeicher gilt jetzt je Ordner. |
| 3 wiederholtes `pi.fill` | 8163f80 | `panel.rs`: `edit_each_changed` – ändert sich bei keinem Element Füllung (`PathAttrs`) oder Griffe, bleibt das Dokument dasselbe, also kein Verlaufsschritt. Echte Änderungen (andere Art, Farbe, Form) wie bisher je ein Schritt. `parx_insp1::gradient_default_and_handles` angepasst: 50× „Winkel 0“ = ein Schritt, Rückgängig führt zur Kontur-Vorgabe zurück. |
| 4 Trennlinien in ≡-Menüs (FIX-B-6b) | 386ef32 | Klick-Fänger geprüft: neuer kittest `fixf_separator` im ganzen Programm (1120 × 720, 125 %), Trennlinien aus den gezeichneten Formen, alle ≡-Menüs, je Trenner Klicks auf der Linie, ±1/±3 px, 1 px innerhalb der Trennerzeile, mittig, links in der Häkchenspalte und rechts hinter dem Linienende (546 Klicks): Menü bleibt offen, nichts ausgeführt (Verlauf, Dokument, Arbeitsbereich unverändert). Gegenprobe: Klick mitten in den Eintrag darunter führt ihn aus. **Ohne den Fänger schlägt der Test fehl** (Klick links in der Häkchenspalte der Trennerzeile schließt das Menü) – der Fehler ist damit offscreen nachgestellt und durch den Fänger behoben. |
| 5 Stempel-Zugende (PAR 05/03) | c726f31 | `tooling.rs::retouch`: beim Stempel liegt der letzte Abdruck am Endpunkt des Zugs (vorher endeten die Abdrücke bis zu einem Abstand ≈ 2,1 px davor). Zug 330 → 390 auf p57_t02: vorher x 322–397, jetzt 322–399 wie die Messung des Originals. Test `parx_tools2::stamp_live_preview_equals_result` prüft (322, 399); Vorschau = Ergebnis bleibt. Nur der Stempel, die übrigen Retusche-Werkzeuge (kalibriert) unverändert. |
| 6 Doku | b4877f0 | `docs\AGENTS.md`/`AGENTS.en.md`: §5c `output.options.alsoFwpng` statt „zweimal laufen lassen“, `items[].extraTargets`, `batch-report.html` (`output.reportHtml`), Vorlage `video-to-gif` in der Tabelle; „Bitmap nachzeichnen“: das Konturverfahren nutzt `refine.snapColors`/`refine.colorTolerance`, Absatz „Farbeinrasten“ zu „Glatte Kurven“ (Vorgabe Logo, ohne Einrasten k-Means). |

## Befunde erledigt (BEFUNDE.md)

- (von CT-1b) „Bedienfeld-≡-Menüs: Trenner sind nicht wirkungslos …“ – abgehakt (kittest, siehe Punkt 4).
- (von PAR-F) „tool-stamp-replace-redeye/03 Stempel: Versatz stimmt, aber Zugende 2 px kürzer …“ – Zugende erledigt vermerkt, Zeile bleibt offen für die Pixelwerte im Zug (nicht neu gemessen).
- Noch nicht in BEFUNDE übertragen, hier erledigt: FIX-D „`i18n_menue::every_command_and_tool_has_texts_in_de_and_en` ist rot …“, FIX-D „`menu_fixes_fv13::*`/`menu_mn4::*` scheitern … nur parallel“, PAR-X-INSP-2 „Wiederholtes `pi.fill` mit derselben Art legt … je Aufruf einen Schritt an“, BATCH-11b „`docs\AGENTS.md`/`AGENTS.en.md` §5c: ‚Beides auf einmal: zweimal laufen lassen‘ ist überholt …“, FIX-C „`docs\AGENTS.md`/`AGENTS.en.md` ‚Bitmap nachzeichnen‘: Das Konturverfahren nutzt mit `refine.snapColors` …“.

## PARITAET → „umgesetzt, Nachprüfung ausstehend“

- tool-stamp-replace-redeye 03 (Zugende)
- Bedienfeld-≡-Menüs: Trennlinien wirkungslos (CT-1b MN-6)

## Nachprüfung (echtes Fenster, Brücke)

1. FIX-B-6b wie in `fixb.md` (Ebenen-≡, Klicks auf/um jede Trennlinie, auch links in der Häkchenspalte): nichts ausgeführt, Menü bleibt offen.
2. Stempel (PAR 05/03): T02, Quelle Alt-Klick 120,268, zwei Züge wie in der Messung → Ausdehnung 322,252–399,309; Pixelabweichung zum Original notieren.
3. Inspektor: Rechteck, Füllart „Verlauf“ zweimal hintereinander wählen → Verlauf-Bedienfeld zeigt nur einen Schritt.

## Tests

- fp-engine `--test all` (287) und `--lib` grün, neu `fixf::repeated_fill_same_kind_adds_no_step`.
- fp-ui `--test all` komplett **parallel** grün (605 ok, 47 ignoriert), neu `fixf_separator::` (2).
- fp-mcp, fp-script grün; fp-i18n grün, fp-neutral (inkl. Scanner/technical_english) grün; `cargo check --workspace --all-targets` ohne Warnungen.

## Neue Befunde

- Ein früherer Vollauf von fp-ui war einmal rot in `batch_ui::trockenlauf_und_lauf_zweimal` (bekannter Wackler aus BATCH-11b, beim zweiten Vollauf grün) – unverändert offen.
- Hinweis: Beim ersten Edit (Punkt 1) landete der Text versehentlich im Hauptbaum (relativer Pfad); sofort in den Slot übernommen und die zwei Dateien im Hauptbaum per `git checkout -- <pfad>` zurückgesetzt, bevor jemand committete.
