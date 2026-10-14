# G-B3 – Paket B3 aus der Mess-Sitzung B-2b

Stand 14.10.2026 spät. Grundlage: `docs\berichte\b2b.md` (Soll-Regeln, Codestellen). Slot `fp-s2`, Commit auf main.

## Umgesetzt (Commit `07b992c`)

| BA | Soll (B-2b) | Umsetzung |
|---|---|---|
| BA-003 | Alt-Ziehen mit dem Stift am mittleren bzw. End-Eckpunkt eines gewählten Pfads = glatter Punkt (beide Henkel); Alt-Ziehen am Anfangspunkt danach schließt, nur Eingangshenkel | `fp-engine tools\draw.rs` `PenTool`: neues `alt_point` (mittlerer Punkt: Henkel symmetrisch, Schritt „Stift“; Alt-Klick ohne Bewegung wie bisher Klick), `smooth_drag` (Fortsetzen am Endpunkt mit Alt: beide Henkel). Der soeben gesetzte Startpunkt eines neuen Pfads behält nur den Ausgangshenkel (PAR 09/09) |
| BA-007 | Treffer auf Bitmaps nur an Pixeln mit Alpha > 0 | `fp-engine docops.rs` `hit_depth` → `bitmap_hit` (Pixel unter dem Punkt, Schwelle 0); gilt für alle Werkzeuge mit Treffertest; Ziehen aus durchsichtigem Bereich = Auswahlrahmen |
| BA-020 | Grenzen der Animationsinstanz = Objekt ∪ Objekt waagerecht um „Verschieben“, Richtung ohne Wirkung | `fp-engine docops.rs` (Grenzen der Instanz): Versatz je Status = t · Länge des Vektors waagerecht; die gezeichneten Status-Lagen bleiben beim Vektor (nicht gemessen) |
| BA-039 | Gesperrte aktive Ebene: Pinsel legt die neue Bitmap ohne Meldung auf der nächsten freien Ebene darunter an, die aktiv wird | `fp-engine commands\bitmap.rs` `apply_paint_job` → `unlocked_layer_below` (gilt auch für den Bleistift, gleicher Befehl) |
| BA-119 | Rastern backt Deckkraft und Mischmodus ein (Deckkraft 0 fällt weg, 50 % → Alpha 128, Ergebnis Deckkraft 100) | `fp-engine render_backend.rs` und `fp-ui canvas\render.rs` `render_elements`: Komposition mit Deckkraft/Mischmodus je Element (wirkt auch auf Zieh-Vorschau und Kopieren als Bild) |
| BA-128 | Werkzeugleiste: Verlauf/Muster immer bedienbar, öffnen das Verlaufs- bzw. Muster-Popup (erst dessen Wahl ändert Auswahl bzw. Werkzeugvorgabe); Strich-Popup ohne Strichlage | `fp-ui tools_panel.rs` (Fußleiste `Fill { kinds: true }`, Strich `NoneOnly`), `inspector\obj_quads.rs` `toolbar_fill_popups` (eigene Popup-Ids `tb-fill`, Ziel Auswahl mit Vektoren, sonst Werkzeugvorgabe). Keine neuen Texte |

BA-008 nicht umgesetzt: Nutzerentscheid 14.10. ~23:25 – bewusst anders (Strg+Z entfernt die Bitmap ganz), in BEFUNDE/BEFUNDE_ABBAU geschlossen.

## Tests

- Neu `crates\fp-engine\tests\gb3_paket.rs` (6 Tests: BA-003 ×2, 007, 020, 039, 119 mit den Messwerten des Originals) und `crates\fp-ui\tests\gb3_werkzeugleiste.rs` (3 kittests: Verlauf/Muster ohne Auswahl, Abbrechen mit Auswahl, Strich-Popup ohne Strichlage).
- Angepasst auf die neuen Messungen: `canvas_fv14` (FV-11-Test → BA-007: Klick in die Lücke wählt nichts) und `dialogs_dlg9` (D28: Richtung 90 → Grenzen 20,20,170,100).
- `fp-engine --test all` 338/338 grün; fp-ui-Module gb3_werkzeugleiste, canvas_fv14, dialogs_dlg9, color_picker_farb1b, fixd_ct2, bitmapwerkzeuge_fv11, brush_live_pinsel1, canvas_f3, tools_fv16, gesamt_werkzeuge, dialogs_dlg5/6, dialogs_f1, fixc_dialogs, form_form1, inspector_fields, levels_kv2, menu_mn2, nz1, nz3, tw_f4, ux_b_paritaet, inspector_snapshots grün.
- Golden (`--include-ignored`): fp-export 2/2, fp-vector render_golden 1/1, fp-raster 2/2, fp-text 4/4, fp-effects 1/1 grün. `fp-neutral` grün. `cargo check --workspace --all-targets` ohne Warnungen.

## Brücke (Slot-Build `target-s2\debug\firepixel.exe --mcp`, eigenes Profil APPDATA/LOCALAPPDATA im Scratchpad)

- BA-128: Füllfarbfeld → Popup, „Verlauf“ ohne Auswahl aktiv → Verlaufs-Popup (Linear, Leiste, Umkehren, Winkel grau); mit R1: Verlaufs-Popup, Esc → R1 einfarbig, kein Verlaufsschritt; Strichfarbfeld → nur Ohne + Farbkreis.
- BA-003: Pfad 200,200 · 300,160 · 400,200, Stift, Alt-Ziehen wie gemessen → `[200,200 pred 230,200] [300,160 pred 270,160 succ 330,160] [400,200 pred 370,200 succ 430,200]`, geschlossen – gleich dem Original.
- BA-007: gerasterte Ellipse 500,300–600,400: Klick 503,303 → nichts, Mitte → EB, Ziehen aus der Ecke → Rahmen wählt EB, keine Verschiebung.
- BA-039: Ebene „Gesperrt“ aktiv und gesperrt, Pinselzug 60,450 → 200,450: keine Meldung, Bitmap auf „Ebene 1“, aktiv danach Ebene 1.
- BA-119 (App-Renderer): gelb + blau 50 % + rot 0 % rastern → Grenzen 20,150–170,250, Pixel 100,200 = 127,127,128,255, 150,200 = 0,0,255,128 – gleich dem Original.

## PARITAET-Zeilen auf „umgesetzt, Nachprüfung ausstehend“

BA-003 (PARITAET_A L33), BA-007 (FV-11/FV-14), BA-020 (PARITAET_B D28), BA-119, BA-128 (FARB-1b A4).

## Nachprüfung

- BA-020 „Größe ändern“: laut B-2b nimmt das Feld B im Inspektor einer Animationsinstanz im Original keinen Wert an – mit Bild bestätigen, dann ggf. Firepixel anpassen (nicht umgesetzt).
- BA-128: Wahl einer Verlaufsart im Werkzeugleisten-Popup ohne Auswahl → Werkzeugvorgabe Verlauf; mit R1 → R1 Verlauf, ein Schritt. Startfarben des Verlaufs-Popups (Weiß → Füllfarbe) sind nicht gemessen.
- BA-007 mit Strg-Klick und Restalpha im radierten Loch (FV-11-Hinweis) im echten Fenster gegenprüfen.

## Neue Befunde

- Keine. Hinweis: Das Rastern backt jetzt auch für die Zieh-Vorschau und „Kopieren als Bild“ Deckkraft/Mischmodus ein (sieht aus wie auf der Leinwand); nicht gegen das Original gemessen.
