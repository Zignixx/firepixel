# RUN-3: Runner zuverlässig machen (14.10.2026)

Anlass: Zweiter Volllauf (14.10. 00:25–02:08) abgebrochen, Referenzseite fiel massenhaft aus. Geändert nur
`tools\parity\runner\*` (kein Rust, keine Drehbücher). Kein Volllauf.

## Ursachen

| Fehlerbild | Ursache (nachgestellt) |
|---|---|
| `panel not visible: Eigenschaften` (ab inspector-group-symbol 04, danach alle Inspektor-Einheiten) | Schritt `Fenster > Symboleigenschaften` legt das Bedienfeld als Registerkarte **in dieselbe Dockgruppe** wie Eigenschaften und aktiviert es; Eigenschaften ist dann verborgen (nicht sichtbar). Nichts stellte es wieder her. Nicht das Aufräumen von RUN-2. |
| `menu did not open: Datei/Modifizieren/Auswählen` (dialogs-d01-d10 ab d10, dialogs-d11-d20, menu-file ab 17, menu-modify-a/-b, menu-select) | `Datei > Datei-Info` öffnet einen **Flash-Dialog** (Top-Level-Klasse `SWFPLAYER_SWFWINDOWCONTAINER`, keine Win32-Knöpfe). Escape und das Dialog-Schließen (nur `#32770`) wirken nicht → modal offen → Menüleiste tot. Der Fehler zählte nicht als „systemisch“, daher kein Neustart. |
| Original lief um 02:08 nicht mehr | Kein Absturz (keine Ereignisprotokoll-Einträge). Beim Abbruch beendet `shutdown()` das Original, weil der Runner es um 00:39 neu gestartet hatte (`started_by_us` wurde beim Neustart gesetzt). Im Testlauf zusätzlich gefunden: Schritt menu-file `21_exit_open` führt `Datei > Beenden` aus; die Sicherheitsfrage wird mit „Nein“ (nicht speichern) beantwortet → Programm endet wirklich. |

## Änderungen

- **Eigenschaften-Bedienfeld**: `OrigSide.panel()` zeigt ein nicht sichtbares Bedienfeld über seinen
  Menüeintrag (`Fenster > …`, nur wenn unsichtbar – der Eintrag schaltet sonst aus) und wartet darauf; genutzt für
  `inspector`/`panel:*`-Bereiche, `rel`-Locatoren und `panel_menu`. Vor jedem Schritt (`reset_light`) und am
  Einheitenbeginn wird Eigenschaften sichergestellt. `close_floating` schließt keine schwebende Gruppe, die
  Eigenschaften enthält (neuer Host-Befehl `kids`).
- **Flash-Dialoge**: zählen als Dialoge (Aufnahmebereich `dialog`, wartet bis 4 s auf späte Dialoge), werden mit
  `WM_CLOSE` abgebrochen (`dialog_close`), nie bestätigt.
- **Menüleiste**: vorher Flash-Dialoge und Menüs schließen, bis zu 3 Versuche (dazwischen Vordergrund an das
  Hauptfenster), Fehlermeldung nennt `main enabled`, Vordergrundklasse und offene Fenster. `menu did not open` und
  „panel not visible after …“ gelten als systemisch → nach 3 Fehlern in Folge sauberer Neustart.
- **Prozessende**: vor jeder Referenzseite eines Schritts Prüfung `running()`; sonst Neustart + Testdatei
  (Log „reference program not running - starting it again“). `started_by_us` wird nur beim ersten Start entschieden;
  lief das Original bei Laufbeginn, bleibt am Ende eine Instanz offen (ggf. neu gestartet, Fragen abgelehnt).
- **Gesperrte Menüeinträge**: `Datei > Beenden` wird im Original nie ausgeführt (`orig.forbidden_menu`,
  Vorgabe `["Datei > Beenden"]`) → Schritt `21_exit_open` meldet einen Fehler statt das Programm zu beenden.
- **ui_map**: Aliase (String-Locator = anderes Ziel); `dialog.radius` → `dialog.field.0` (beide Seiten).

## Test (`--units inspector-stroke,menu-modify-a,dialogs-d11-d20,menu-file`)

Ausgangszustand absichtlich kaputt: Symboleigenschaften aktiv + Datei-Info-Dialog offen.

| Einheit | Original vorher (Lauf 2): ok / Fehler | nachher: ok / Fehler | Firepixel unverändert |
|---|---|---|---|
| inspector-stroke | 1 / 50 | 25 / 2 (+23 unmapped, 1 unsupp.) | 46 ok |
| menu-modify-a | 2 / 24 | 24 / 2 | 26 ok |
| dialogs-d11-d20 | 19 / 11 | 27 / 2 (+7 unmapped) | 34 ok |
| menu-file | 13 / 9 | 17 / 5 → nach Sperre von Beenden: 21_exit_open sauber abgewiesen, Original läuft weiter | 21 ok |

Summe Referenzfehler: **94 → 11**. Restfehler (Drehbuch-/Locator-Ebene, keine Systemfehler): JSF-Fehler
(`03_width_clamp`, `12_text_stroke`, `18_sel_jpeg`, `22_undo_names` -2147216344), deaktivierte Menüeinträge
(`Als Maske gruppieren`, `Wiederherstellen`, `Alles speichern`), `Zuletzt geöffnet > 1` fehlt, D17-Kombifeld,
`set_text`-Zeitüberschreitung D19, `21_exit_open` (gesperrt).

## Offen

- `inspector.slice_type` (inspector-web) im Original nicht vermessen; `{popup_swatch}` (color-popup) im Original
  nicht umgesetzt; Verb `export` (export-results) unsupported; `no popup slider open` (inspector-fields).
- `21_exit_open` braucht für das Original einen sicheren Weg (Frage erscheint nur bei ungespeicherten Änderungen,
  Antwort muss „Abbrechen“ sein) – bis dahin bewusst gesperrt.
