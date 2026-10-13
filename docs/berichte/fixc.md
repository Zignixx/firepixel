# Bericht FIX-C – Befunde von CT-1a (Dialoge), 13.10.2026

Slot `fp-s2` / `target-s2`. Commits auf `main` (je Punkt einer, Präfix `FIX-C:`).

## Umgesetzt

| Punkt | Commit | Änderung |
|---|---|---|
| D43 Kurven: Raster und Kurvenlinie fehlen | `f743b94` | `dialogs\curves.rs`: Einzelpixel (gepunktetes 4×4-Raster `#808080`, Kurve ein Pixel je Spalte) als ungeglättetes Netz. Ursache: `rect_filled` 1 × 1 wird vom Tessellator als geglättetes Linienstück gezeichnet und ist kaum sichtbar. |
| D42 Helligkeit/Kontrast: Pfeiltasten | `cd76c1b` | `dialogs\form.rs`: Schieber mit Tab-Stopp bekommen bei Klick/Ziehen den Fokus (wie die Win32-Trackbar) und zeigen einen Fokusrahmen. Pfeiltasten wirkten in kittest schon ab Startfokus und nach Tab; nach einem Klick auf die Leiste hatte sie keinen Fokus. |
| D21 Leinwandfarbe: unsichtbarer Tab-Stopp | `c6c45ce` | `dialogs\form.rs`: eingerücktes Farbfeld ohne Beschriftung wird per `skip_tab_stop_*` übersprungen (wie D01/D18). Weiß › ↓ Transparent › Tab → OK › Abbrechen. |
| D16 Voreinstellungen: Höhe, „Sprache:“ | `260648f` | `dialogs\preferences.rs`: „Allgemein“ 21-px-Zeilen, 20-px-Gruppenüberschriften; übrige Seiten 18 px je Kästchen/Überschrift, 21 px je Feldzeile; Mindesthöhe 440 statt 500. Außen jetzt ≈ 500 px (de und en, Test 480–520). „Sprache:“ rechtsbündig vor dem Feld wie „Design:“. Fußleiste unverändert. |
| FX-4 Live-Effekt Rauschen startet mit 20 | `a9aa8a6` | `dialogs\catalog.rs`: Live-Filter-Dialoge laden und merken die Menüwerte nicht mehr; ein neuer Effekt beginnt mit den Effektvorgaben (Rauschen 50, `re\23` „vorgabe“). Gilt für alle Live-Filter. |
| KV-3 Live-Filter „Gradationskurven“ + Zahnrad | `c419943` | `dialogs\effects.rs`: Kurven/Tonwertkorrektur in der Filterliste mit dem Menünamen („Kurven...“, „Tonwertkorrektur...“, `re\19` §6). `inspector\glyphs.rs`: neues Symbol `Glyph::Info` (ⓘ). `inspector\obj_quads.rs`: Reihenfolge Kästchen › ⓘ › Name wie im Original; ⓘ nur bei Effekten mit Einstellungen; liegt jetzt links und ist auch im schmalen Inspektor sichtbar. |
| E2 CSS-Sprites: Präfix „1 x“ still verworfen | `1eedbd9` | `dialogs\export_options.rs`: Präfix/Suffix als Live-Felder (jede Eingabe sofort übernommen, Beispielzeile live). Damit kommt der ungültige Präfix beim Export an und löst die vorhandene Meldung „Einige der CSS-Selektornamen …“ aus (§4.2: Meldung beim Export, nicht beim OK). |
| E3 HTML-Export ignoriert HTML-Einrichtung | `f608f3e` | `dialogs\export.rs`: Der Exportdialog setzte `html_prefs` (Kleinbuchstaben, leere Zellen, Endung, Kommentare) nicht, nur der Engine-Befehl `file.export`. Jetzt `fp_engine::commands::export::html_prefs(&prefs)`. |
| VT-6 „Glatte Kurven“ mehr statt weniger Punkte | `8b38824` | `fp-trace\src\potrace.rs`: mit `refine.snapColors` (Vorgabe Logo) nimmt das Konturverfahren dieselbe kantenbewusste Palette wie das Flächenverfahren (`refine::palette`, dafür `pub(crate) mod palette`). Mischtöne der Kantenglättung bekommen keine eigene Farbebene mehr. Blaues Quadrat mit gelbem Kreis: 2 Farben, ≤ 16 Knoten, Kreis glatt auf dem Radius (vorher 9 Farben/477 Punkte). Ohne Einrasten (Illustration, Foto) bleibt k-Means. |

## Tests

- neu `crates\fp-ui\tests\fixc_dialogs.rs` (8 Tests): `d43_curves_grid_and_line_visible`, `d42_arrow_keys_move_focused_slider` (Startfokus, Tab, Klick), `d21_canvas_color_no_hidden_tab_stop` (auch Umschalt+Tab), `d16_preferences_height_and_language_label` (de+en), `kv3_live_curves_label_and_info_button` (Klick auf ⓘ öffnet Kurven), `e2_sprite_prefix_typed_then_ok_is_kept`, `e3_export_dialog_uses_html_setup`.
- Unit-Tests: `catalog::tests::filter_forms_have_ranges` (Live-Rauschen 50, Menü 20), `effects::tests` (Kurven-Name), `fp-trace --test potrace logo_preset_snaps_antialias_colors`.
- Grün außerdem: `form_form1`, `curves_kv1`, `dialogs_dlg*`, `dialogs_des1`, `dialog_scroll_rand2b`, `clipping_audit`, `i18n_sprachwahl`, `kv3`, `dialogs_effects`, `inspector_*`, `export_e*`, `trace*` (fp-ui/fp-engine), `fp-trace` komplett, `cargo check --workspace --all-targets` ohne Warnungen, `fp-neutral`, `fp-i18n`.

## Befunde abgehakt (BEFUNDE, Abschnitt CT, „von CT-1a“)

D21 Leinwandfarbe …, D43 Kurven …, D16 Voreinstellungen …, Live-Filter Kurven heißt „Gradationskurven“ …, CSS-Sprites-Optionen: ungültiger Präfix …, HTML-Export ignoriert die HTML-Einrichtung …, D42 Helligkeit/Kontrast …, Bitmap nachzeichnen „Glatte Kurven“ …, Live-Effekt „Rauschen hinzufügen“ … – alle neun.

## PARITAET → „umgesetzt, Nachprüfung ausstehend“

PARITAET_B D16 (Höhe/„Sprache:“), D21 (Tab-Folge), D42 (Pfeiltasten/Fokusrahmen), D43 (Raster/Kurve); KV-3 (Name/ⓘ), E2 (Sprite-Präfix), E3 (HTML-Einrichtung im Export), VT-6 (Glatte Kurven), FX-4 (Rauschen-Vorgabe).

## Für die Nachprüfung (Brücke, echtes Fenster)

1. D43: Filter ▸ Farbe anpassen ▸ Kurven… auf Bitmap → gepunktetes 4×4-Raster grau und Diagonale als schwarze Pixellinie sichtbar; Punkt ziehen → Kurve folgt.
2. D42: Helligkeit/Kontrast → Startfokus Schieber mit Rahmen, → ×5 = 5; Klick auf die Kontrast-Leiste, dann ← → Wert ändert sich, Rahmen an der Leiste.
3. D21: Leinwandfarbe → ↓ (Transparent) → Tab → Fokus auf OK (kein unsichtbarer Stopp).
4. D16: Voreinstellungen außen 480–520 px hoch; „Sprache:“ endet bündig mit „Design:“; Seite „Hilfslinien und Raster“ ohne Überlappung oder Abschneiden (18/21-px-Zeilen); en und qps ohne Überlauf.
5. KV-3: Inspektor ▸ Filter + ▸ Farbe anpassen ▸ Kurven… ▸ OK → Eintrag „✓ ⓘ Kurven...“; Klick auf ⓘ öffnet den Dialog mit den Effektwerten, auch bei 900 × 650.
6. FX-4: Menüfilter Rauschen mit 20 bestätigen, dann Live-Effekt Rauschen hinzufügen → Dialog zeigt 50.
7. E2: Export CSS-Sprites ▸ Optionen ▸ Präfix „1 x“ (Beispielzeile ändert sich beim Tippen) ▸ OK ▸ Export → Meldung „Einige der CSS-Selektornamen …“; Optionen erneut öffnen → „1 x“.
8. E3: HTML-Einrichtung „Dateiname in Kleinbuchstaben“, Tabelle „Einzelne Tabelle ohne Platzhalter“, Zellfarbe `#123456`, Inhalt geschütztes Leerzeichen, „Bereiche ohne Segmente einschließen“ aus → Export „MyPage“ über den Exportdialog: `mypage.htm`, `mypage_r1_c1.gif`, leere Zellen `bgcolor="#123456">&nbsp;`.
9. VT-6: Testbitmap blaues Quadrat mit gelbem Kreis, Glatte Kurven (Vorgabe Logo) → Kennzahlen 2 Farben, wenige Punkte (≈ 10–16), Kreisrand glatt.

## Annahmen

- D42: In kittest bewegten die Pfeiltasten den Schieber schon ab Startfokus. Fehlend war der Fokus nach einem Klick auf die Leiste und der sichtbare Fokusrahmen; vermutlich hatte CT-1a vorher auf die Leiste geklickt. Behoben ist beides; die Brückenprüfung bestätigt es.
- D21: Das Farbfeld ist nie ein Tab-Stopp, auch bei „Benutzerdefiniert“ (gemessen ist nur Tab Weiß › OK › Abbrechen).
- D16: Kompaktere Zeilen statt Bildlauf oder zweier Spalten. Die Seite „Hilfslinien und Raster“ (19 Einträge) bestimmt sonst die Höhe.
- KV-3: Effekte ohne Einstellungen zeigen kein Symbol, der Name bleibt eingerückt. Nur Kurven/Tonwert tragen den Menünamen mit „...“; die übrigen Namen bleiben wie bisher (nicht gemessen).
- FX-4: „Live-Effekte starten mit den Vorgaben“ gilt für alle Live-Filter, nicht nur für Rauschen.
- VT-6: Das Konturverfahren nutzt jetzt auch `refine.snapColors`/`colorTolerance` aus der Vorgabe.

## Neue Befunde

- (für Doku/AU) `docs\AGENTS.md`/`AGENTS.en.md` „Bitmap nachzeichnen“: Das Konturverfahren nutzt mit `refine.snapColors` (Vorgabe Logo) die eingerastete Palette. Die Liste der genutzten Optionen um `refine.snapColors`/`colorTolerance` ergänzen.
