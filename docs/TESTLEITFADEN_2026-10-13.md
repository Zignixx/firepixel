# Testleitfaden Nachtlauf 13.10.2026 (GT-1, CT-1…4, FIX, GT-2)

Leitfaden für die Nachttests. Sie starten **erst nach dem „los“ des Nutzers** im Chat und nie automatisch (PLAN §6, Entscheidung 13.10. ≈ 01:13). Gesamtdauer ≈ 2–2,5 h.

Grundlage:
- Ablauf und offene Punkte: `docs\progress-2026-10-13.md` §2 und §4
- Regeln: `docs\AGENTEN_REGELN.md` §1a, §4a, §4d, §4e, §7a, §7b
- Brücke: `docs\AGENTS.md` §5a/§5b
- Prüfliste: `docs\NACHPRUEFUNG.md` (Abschnitte §1–§7)
- Berichte der Welle 3 unter `docs\berichte\` (seit `02a087f`)

Dieser Leitfaden verweist auf die NACHPRUEFUNG-Abschnitte und kopiert keine Zeilen. Maßgeblich ist der Stand der Datei beim Start des Pakets; DOC-3 erweitert sie gerade um die Zeilen aus den Welle-3-Berichten.

Für den Hands-on-Test des Nutzers gilt weiter `docs\TESTLEITFADEN_2026-10-12.md`. Dieser Leitfaden hier ist für die Prüf-Agenten.

**Neutralität (§1a):** In Protokollen, Befunden, Dateinamen und Bildschirmfotos steht nur „das Original“. Bildschirmfotos des Originals mit Namen in der Titelleiste werden vor dem Commit geschwärzt bzw. nur zugeschnitten abgelegt; `cargo test -p fp-neutral` prüft auch den Bildtext.

---

## 0. Ablauf auf einen Blick

| ab „los“ | Paket | Slot / Instanz | Ergebnis |
|---|---|---|---|
| 0–45 min | **GT-1** | `fp-s1` (`target-s1`), Release in `fp-rel` (`target-rel`) | `docs\funktionstest\gt1_2026-10-13.md` |
| 15–60 min | **CT-1** | Brücke, Ecke unten rechts | `docs\funktionstest\ct1_2026-10-13.md` |
| 15–60 min | **CT-2** | Brücke, Ecke unten links | `docs\funktionstest\ct2_2026-10-13.md` |
| 15–60 min | **CT-3** | Brücke, Ecke oben rechts | `docs\funktionstest\ct3_2026-10-13.md` |
| 15–60 min | **CT-4** | Brücke, Ecke oben links + Original unter der Sperre | `docs\funktionstest\ct4_2026-10-13.md` |
| 60–105 min | **FIX-1…n** | `fp-s3` … `fp-s8` | Commits + `docs\berichte\fix<n>.md` |
| 105–135 min | **GT-2** | `fp-s1`, Release in `fp-rel` | Endversion auf dem Desktop, Bericht im Chat |

Höchstens **8 Agenten gleichzeitig**, alle Opus, kleine Aufträge. Die CT-Pakete starten nach 15 min, sobald GT-1 die Release-exe gebaut hat (§1.1). Sie warten nicht auf das Ende der Testläufe von GT-1.

---

## 1. Voraussetzungen

### 1.1 Programmstand

- **Testversion:** Ordner `C:\Users\Administrator\Desktop\Firepixel-Vorschau-2026-10-13_<HH-MM>` mit `NEU.md` (aus `docs\preview-notes-2026-10-13.md`).
  - Laufen TE-2/DOC-3 **nach** der Testversion und **vor** dem „los“ (Variante a in progress §3), baut GT-1 zuerst eine neue Release-exe.
  - Ablauf: sauberer Worktree `C:\Users\Administrator\Desktop\REA\fp-rel` auf `main`, `CARGO_TARGET_DIR=C:\Users\Administrator\Desktop\REA\firepixel\target-rel`, `cargo build --release -j 6 -p firepixel -p firepixel-cli`.
  - Die CT-Pakete nehmen dann diese exe. Sonst nehmen sie die exe der Testversion.
- **Jede CT-Instanz kopiert die exe vorher in ihren eigenen Ordner** (siehe 1.3). Dann sperrt der GT-2-Build keine laufende Datei, und alle prüfen denselben Stand.
- **Commit-Stand notieren:** `git -C C:\Users\Administrator\Desktop\REA\firepixel rev-parse --short HEAD` steht im Kopf jeder Ergebnisdatei.

### 1.2 Warme Slots (§7b)

- Worktrees `C:\Users\Administrator\Desktop\REA\fp-s1 … fp-s12`, je mit eigenem `target-s<N>`.
- Zu Beginn muss `git -C ..\fp-s<N> status --short` leer sein, dann `git -C ..\fp-s<N> checkout -q --detach main`.
- **Nach TE-2 vorwärmen:** TE-2 benennt um:
  - die Sprachdateien in englische Namen (`allgemein.ftl` → `common.ftl`, `menue.ftl` → `menu.ftl` …),
  - deutsche Quelldateinamen,
  - die Umgebungsvariablen (z. B. `FP_QA_ONLY`).

  Alte Datei- und Variablennamen werden weiter gelesen; Testnamen (`gesamt_*`, `clipping_audit` …) bleiben.
- Deshalb nach TE-2 in jedem Slot, der in der Nacht baut (s1, s3–s8), einmal vorwärmen:
  - `git checkout -q --detach main`
  - `cargo test -p fp-ui --test all --no-run -j 3`
  - `cargo test -p fp-engine --test all --no-run -j 3`

  Läuft TE-2 erst nach den Nachttests, entfällt das.
- Im Slot: `CARGO_INCREMENTAL=1`, `-j 3`, kein `RUSTC_WRAPPER`, kein `cargo clean`. Vor großen Builds den freien Platz prüfen mit `(Get-PSDrive C).Free`; unter 30 GB dem Koordinator melden.
- Am Ende Slot und `target-s<N>` **nicht löschen**, nur sauber hinterlassen.

### 1.3 Eigene Profile, eigene Fenster

Jede Firepixel-Instanz läuft mit eigenem `APPDATA`/`LOCALAPPDATA` und `FP_NO_SHELL_OPEN=1` (§4a). Sie läuft nie im Profil des Nutzers.

**Werkzeug:** `C:\Users\Administrator\Desktop\REA\re\_testdateien\ux\uxa\werkzeug\fpmcp.ps1`, per Punkt einbinden.

Nach dem Einbinden umstellen, weil das Skript sonst den gemeinsamen Ordner `fphome` und eine alte exe nimmt:

```powershell
. C:\Users\Administrator\Desktop\REA\re\_testdateien\ux\uxa\werkzeug\fpmcp.ps1
$global:FPHOME = '<Scratchpad>\ct1\fphome'        # je Paket eigener Ordner: ct1, ct2, ct3, ct4
$global:FPEXE  = '<Scratchpad>\ct1\bin\firepixel.exe'  # Kopie aus 1.1
$p = FP-Start                                     # startet --mcp mit eigenem Profil und FP_NO_SHELL_OPEN=1
FP ui_state @{}                                   # Verbindung prüfen
```

Weitere Funktionen von `fpmcp.ps1`:
- `FP <werkzeug> @{…}` ruft ein Brückenwerkzeug auf (`ui_find`, `ui_input`, `ui_state`, `ui_action`, `screenshot`, `ui_file_dialog`, `run_command`, `get_document` …).
- `FP-Fenster <pid>` liefert das Fensterhandle.

**Fenster:**
- Größe 900 × 650, sichtbar, **nicht aktiviert**. Mit `SetWindowPos` und `SWP_NOACTIVATE` in die zugeteilte Ecke legen (§0). Nie `SetForegroundWindow` auf eine Brücken-Instanz.
- Die Brücke antwortet nur, solange das Fenster sichtbar ist (§4e).
- Ist eine größere Fenstergröße nötig (CT-4), diese nur kurz setzen und danach wieder 900 × 650.

**Testdateien:** nur Kopien im eigenen Scratchpad-Unterordner, z. B. `<Scratchpad>\ct2\dateien\`. Nie Dateien des Nutzers.

### 1.4 Die Sperre nur fürs Original (§4e)

- **Firepixel über die Brücke:** ohne Sperre, parallel. Protokoll: „**echtes Fenster, Brücke**“.
- **Unter der Sperre:** `C:\Users\Administrator\Desktop\REA\re\scripts\fwsteuerung\gui-sitzung.ps1 -Skript <schritte.ps1>` brauchen nur:
  - alles, was das **Original** bedient. JSF-Läufe gehen über `jsf-ausfuehren.ps1 -Skript x.jsf`, innerhalb einer Sitzung mit `-OhneSperre`. Hilfen: `dialog.ps1`, `shot.ps1`, `input.ps1`.
  - Prüfungen mit **echter Systemmaus/-tastatur** an Firepixel, wo die Brücke das Verhalten nicht abbildet: Zeitverhalten, Fensterverschieben/-größe durch Windows, Fokus/Taskleiste, Drag & Drop aus dem Explorer, Doppelklick-Zeiten, Mausrad-Hardware, IME, Startverhalten. Protokoll: „**echte Maus**“.
- **Sitzungen kurz halten** (< 5 min), danach die Fenster schließen.
- **Original:**
  - Es ist zurzeit beendet. CT-4 startet es unter der Sperre neu.
  - Vorher prüfen, ob fremde ungespeicherte Dokumente offen sind (Titel mit `*`).
  - Nur kopierte Testdateien per `fw.openDocument` öffnen. Aus JSF keine neuen Dokumente anlegen.
  - Fehlercode `-1073741819` → Original beenden und neu starten (README in `fwsteuerung`).

### 1.5 Sicherheitsregeln (gelten für jedes Paket)

1. **Eingaben nur ins richtige Fenster (§4d):** Vor **jeder** echten Eingabe per `GetForegroundWindow` Prozess-ID **und** Fensterklasse prüfen (Original bzw. eigene Testinstanz). Stimmt eins nicht, nichts senden und abbrechen. **Strg+Alt+Leertaste nie senden.** Echte Eingaben gehen nie in eine Brücken-Instanz eines anderen Pakets.
2. **Keine offenen Datei-Dialoge:**
   - Vor jeder Sitzung auf offene `#32770`-Fenster prüfen.
   - Einen Datei-Dialog nur auslösen, wenn genau er geprüft wird, und im selben Schritt schließen.
   - In der Brücke erscheint der Dialog in der App. Mit `FP ui_file_dialog @{action='choose'; path=…}` oder `@{action='cancel'}` beantworten.
   - Am Ende muss `ui_state` → `file_dialog` leer sein.
3. **Keine fremden Programme:** `FP_NO_SHELL_OPEN=1` in jeder Instanz und jedem Testlauf. Befehle wie Vorschau im Browser, E-Mail oder Hilfe prüfen nur, ob die Datei erzeugt wurde.
4. **Instanzen beenden:**
   - Am Paketende jede eigene Firepixel-Instanz beenden (`$p.Kill()` bzw. `Stop-Process -Id <eigene pid>`), nur die eigenen.
   - Das Original lässt CT-4 so zurück, wie es es vorgefunden hat (beendet bzw. ohne eigene Dokumente).
   - Nie fremde Prozesse beenden.
5. **Nur eigene Dateien:**
   - CT-Pakete schreiben **nur** ihre Ergebnisdatei, Bildschirmfotos unter `docs\screenshots\<paket>\` und Hilfsdateien im Scratchpad-Unterordner `<paket>\`.
   - NACHPRUEFUNG, BEFUNDE, PARITAET, STATUS und PLAN überträgt erst GT-2 bzw. ein Doku-Agent (§7b.2).
6. **git:**
   - Nur mit Pfadangabe committen: `git commit -F <nachricht.txt> -- <eigene Pfade>`, Präfix `<Paket>:`, letzte Zeile `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
   - Nie `git add -A`, Reset, Clean, Rebase oder Force. **Kein Push.**
7. **Abbruch:** Abbrechen, die Instanz beenden und den Abbruch im Protokoll vermerken, wenn
   - die Brücke 3-mal hintereinander nicht antwortet,
   - ein fremdes Fenster den Vordergrund hat, während echte Eingaben nötig sind,
   - ein Absturz auftritt (dann ❌ mit Log aus dem eigenen `LOCALAPPDATA\Firepixel`),
   - weniger als 30 GB frei sind.

   Danach mit dem nächsten Abschnitt weitermachen; ein abgebrochener Punkt wird ⏭ mit Grund.

### 1.6 Protokollformat (Ergebnisdatei)

```markdown
# CT-1 – Menüs und Dialoge, 13.10.2026
Stand: <commit> · exe: <Pfad> · Profil: <Scratchpad>\ct1\fphome · Fenster 900×650 unten rechts · Sprache de · Design Normal
Beginn/Ende: HH:MM–HH:MM · ✅ n · ❌ n · ⏭ n

| NACHPRUEFUNG | Punkt | Art | Ergebnis | Beleg | Befund / Fix |
|---|---|---|---|---|---|
| §3 | DLG-11 Datei-Info 680×620, Titel = Dateiname | echtes Fenster, Brücke | ✅ | docs\screenshots\ct1\dlg11.png | – |
| §7 | MN-6 Menü höher als Fenster, Pfeilleisten | echtes Fenster, Brücke | ❌ | docs\screenshots\ct1\mn6_1.png | erwartet …, passiert … → FIX |
```

- **Art:** genau einer der Werte „echtes Fenster, Brücke“, „echte Maus“, „Original“ oder „automatisch“ (nur GT).
- **Ergebnis:**
  - ✅ geprüft und wie erwartet
  - ❌ abweichend: ein Satz „erwartet … / passiert …“ und die Datei bzw. den Befehl, wenn bekannt
  - ⏭ nicht prüfbar, mit Grund
- **NACHPRUEFUNG:** Abschnitt und der Wortlaut-Anfang der Zeile, damit GT-2 sie eindeutig abhakt.
- Am Ende der Datei: Liste „**Für FIX**“, je ❌ eine Zeile mit vermuteter Datei bzw. zuständigem Bereich.

---

## 2. GT-1 – Gesamttestlauf (0–45 min)

**Slot:** `fp-s1`. Für den Release-Build: `fp-rel` mit `target-rel`.

**Ablauf** (PowerShell, PATH nach AGENTEN_REGELN §2, `$env:FP_NO_SHELL_OPEN='1'`, `$env:CARGO_TARGET_DIR='…\target-s1'`, `-j 3` bzw. für den Release-Build `-j 6`):

1. **Release-exe zuerst**, falls 1.1 das verlangt. Danach den CT-Paketen den Pfad melden (über den Koordinator).
2. `cargo test --workspace -j 3`. Darin enthalten sind:
   - die Gesamttests `gesamt_*`; sie schreiben `docs\funktionstest\*_auto.md`
   - der schnelle Abschneide-Prüflauf
   - `fp-i18n` (Vollständigkeit)
   - `fp-neutral` mit Doku- und Bildtext-Prüfung (NEUTRAL-1) und `technical_english` (TE-1)
3. **Voller Abschneide-Prüflauf:** `$env:FP_RAND_FULL='1'; cargo test -p fp-ui --test all clipping_audit::`. Erwartet: **0 Funde**, Erlaubnisliste `clipping_audit_allow.txt` leer, Bericht `docs\funktionstest\rand_auto.md` neu.
4. **i18n-Scanner:** `cargo test -p fp-ui --test all i18n_keine_festen_texte:: -- --ignored --nocapture` → 0 feste Texte.
5. **Eigene Prozesse** (siehe §2 der Regeln), falls nicht schon in Schritt 2 gelaufen: `ux_b_abgleich`, `ai_kittest*`, `ai_progress_ki2`, `dialogs_i18n`, `file_dialogs_dlg3`.
6. **Leistung:** `select_all_perf::select_all_3000` mit `--release` (Nachprüfung aus `berichte\perf1.md`). Den Vergleich mit dem Stand vor PERF-1 (`42dabe7`) nur, wenn noch Zeit ist.

**Protokoll:** `docs\funktionstest\gt1_2026-10-13.md` mit:
- je Lauf Befehl, Dauer, grün/rot und Zahl der Tests
- rote Tests mit Name und erster Fehlerzeile
- die geänderten `*_auto.md` (`git status --short docs\funktionstest`)

Committen: die Ergebnisdatei und die neu erzeugten `*_auto.md`, mit Pfadangabe.

**Abbruch:** Bricht ein Test-Build wegen einer Fremddatei, ist das ein ❌ für FIX. Nicht selbst reparieren, weiter mit dem nächsten Lauf.

---

## 3. CT-1 – Menüs und Dialoge (15–60 min)

**Instanz:** Brücke, unten rechts, Profil `<Scratchpad>\ct1\fphome`, de/Normal.

**NACHPRUEFUNG:** **§3 Dialoge** und **§7 Menüs**, vollständig, dazu die neuen Zeilen von DOC-3 in diesen Abschnitten. Schwerpunkte aus Welle 3:
- DES-1 Dialograhmen (N11, Teil Dialoge), FORM-1 Tab-Folge/Klemmen, DLG-10/11 Datei-Info und Rechtschreibung
- MN-4/5/6 Befehle-Menü, „Wiederholen: <Filter>“, Menüs mit Bildlauf-Pfeilen
- RAND-2b Dialoge mit Bildlauf und fester Knopfzeile
- N23 Stichproben im echten Fenster bei 900 × 650

**Ablauf je Punkt:**
1. Menü über `FP ui_find @{text='<Menü>'; exact=$true}` suchen und mit `FP ui_input @{events=@(@{type='click'; pos=<center>})}` öffnen, Eintrag ebenso.
2. Im Dialog Werte ändern, dann OK und Abbrechen. Tab-Folge mit `key Tab` prüfen.
3. `FP screenshot @{}` → `docs\screenshots\ct1\<punkt>.png`.
4. Mit `FP ui_action @{close_dialogs=$true}` aufräumen und `ui_state` prüfen (`dialog_open` = false).

**Dazu Fix-Liste Punkt 7:** *Befehle ▸ Demo – aktuelles Dokument* im echten Fenster. Weil `FP_NO_SHELL_OPEN=1` gesetzt ist, öffnet sich kein Browser. Geprüft wird, dass die Vorschau-Datei im Temp-Ordner `Preview` des Testprofils entsteht und eine Statusmeldung erscheint.

**Logikfehler gezielt (§4b):** in drei Dialogen je einen Wert 10-mal ändern, Abbrechen, und Rückgängig/Wiederholen-Folgen.

---

## 4. CT-2 – Werkzeuge, Leinwand, Inspektor, KI (15–60 min)

**Instanz:** Brücke, unten links, Profil `<Scratchpad>\ct2\fphome`.
- Testdokument 600 × 400 mit Rechteck, Ellipse, Text und Bitmap (Foto aus `<Scratchpad>\ct2\dateien\`).
- KI-Modelle: nur wenn sie im Testprofil vorhanden sind. Die Modelle des Nutzers bleiben lesbar (Wächter); **nichts herunterladen**.

**NACHPRUEFUNG:** **§1 Leinwand und Werkzeuge**, **§2 Inspektor**, **§6 KI-Werkzeuge**.

**Nutzerbefunde mit offener Fensterprüfung:**
- **N01:** Glühen 10-mal ändern, OK → ein Effekt und ein Verlaufsschritt; Abbrechen → nichts.
- **N21:** KI-Karte und Feineinstellungen ohne neue Karte.
- **N24:** Schriftliste mit Bildlaufleiste.

**Ablauf:**
- Werkzeuge mit `FP ui_action @{tool='tool.<name>'}` wählen.
- Ziehen und Klicken über `ui_input` (`drag` mit `steps` ≥ 12, Doppelklick mit `wait`).
- Seitenkoordinaten aus `ui_state` → `documents[].page_on_screen` umrechnen (AGENTS §5a).
- Ergebnis mit `get_document` und `screenshot` belegen.

**Nur mit echter Maus** (unter der Sperre, eine kurze Sitzung): Punkte aus §1, die Zeitverhalten, Doppelklick-Zeit oder Mausrad-Hardware betreffen. Sie sind in NACHPRUEFUNG als „echte Maus“ markiert.

**Leistung (PERF-1, Fix-Liste Punkt 8):** `iPad.png` (Kopie) öffnen, Strg+A, Pfeiltaste, erste Ziehbewegung. Die Zeiten aus der Antwort von `ui_input` notieren. Das ist ein Befund, keine Fehlfunktion: die Werte werden nur protokolliert.

---

## 5. CT-3 – Bedienfelder, Docking, Datei-Abläufe, Bildvorschau/Export, Startseite (15–60 min)

**Instanz:** Brücke, oben rechts, Profil `<Scratchpad>\ct3\fphome`, Ausgabeordner `<Scratchpad>\ct3\out\`.

**NACHPRUEFUNG:** **§4 Datei-Dialoge und Schließen**, **§5 Bildvorschau und Optimieren**.

**Schwerpunkte:**
- **N22:** Bildvorschau komplett (BV-3/BV-4) – OK/Exportieren als ein Verlaufsschritt, Esc beim Ziehen des Exportbereichs, Mausrad.
- **Export:** E1/E2/E3 – *Datei ▸ Exportieren…* mit allen 10 Arten in den Ausgabeordner, Optionen-Dialoge, HTML-Einrichtung, Exportassistent, HTML kopieren/aktualisieren.
- **Fix-Liste Punkt 1:** ALT-Text speichern und wieder öffnen. Wahrscheinlich ❌, wird nur festgehalten.
- **EB-2:** Speichern unter ▸ Reduziertes PNG (`ui_file_dialog` mit `filter`, `save_as_copy`).
- **NZ-4/NZ-5:** Zusatzkästchen `append_fw_png`, `open_untitled`, `open_as_animation`; Startordner „Bilder“ beim Export (im Testprofil).
- **DES-2:** Startseite (N11, Teil Startbildschirm) – drei Karten, Tastatur Tab/Pfeile/Eingabe, Vorschaubilder unter „Zuletzt geöffnet“.
- **Bedienfelder:**
  - ≡-Menüs (MN-3)
  - andocken/lösen/schwebend (über `ui_input`-Ziehen)
  - Fußknopf „Auf Status verteilen“

**Datei-Dialoge:** Jeder Dialog wird im selben Schritt mit `ui_file_dialog` beantwortet (1.5 Nr. 2). Nur in `<Scratchpad>\ct3\out\` speichern.

**Nur mit echter Maus:** Drag & Drop einer PNG aus dem Explorer auf die Leinwand und Schließen über die Taskleiste (NACHPRUEFUNG §4). Beides in einer kurzen Sitzung unter der Sperre; vorher den Vordergrund prüfen (§4d).

---

## 6. CT-4 – Englisch, 3 Designs, Fenstergrößen, Messungen im Original (15–60 min)

### 6.1 Firepixel (Brücke, oben links, Profil `<Scratchpad>\ct4\fphome`)

- **Englisch:** `FP ui_action @{run='view.language…'}` bzw. *View ▸ Language ▸ English*. Danach je ein Durchgang durch:
  - Menüs, Inspektor, 10 Dialoge, Startseite, Bildvorschau
  - Warnungen (z. B. *Simplify…* ohne Auswahl)

  Kein deutsches Wort, keine Rohschlüssel (`dlg-…`, `dlg-md-*`).
- **3 Designs:** `FP ui_action @{theme='normal'|'dark'|'oled'}`. Je Design Bildschirmfotos von Startseite, Hauptfenster, 3 Dialogen und einem Menü mit Bildlauf (N11 – DES-1/DES-2).
- **Fenstergrößen:** 800 × 600, 1024 × 768, 1366 × 768, 1920 × 1080. Je Größe Inspektor, ein hoher Dialog und ein langes Menü (N23, RAND-2a/2b, MN-6). Erwartet: nichts abgeschnitten, Bildlauf bzw. Pfeilleisten, wo nötig. Danach wieder 900 × 650.
- **Bildserie:** `docs\screenshots\ct4\<design>_<sprache>_<ansicht>.png`.
- **NACHPRUEFUNG:** alle Zeilen mit Art „Original“ in §1–§7 und alle Zeilen zu Englisch, Designs oder Fenstergröße.

### 6.2 Messungen im Original (unter der Sperre)

Messungen sind **keine Prüfung, sondern die Vorgabe** (§7a „nicht verschoben“). Ergebnisse kommen mit Zahlen in `ct4_2026-10-13.md`, Abschnitt „Messungen“: Wert, Ablauf, Bildschirmfoto; Fensterlisten mit IDs und Rechtecken.

**Ablauf:**
1. Sperre über `gui-sitzung.ps1` nehmen.
2. Original starten und Testkopien öffnen.
3. Je Messung höchstens eine Sitzung von < 5 min.
4. Mit `dialog.ps1 -Abbrechen` aufräumen.

Die Fensterliste kommt wie in `re\scripts\bildvorschau\fenster.ps1`.

**Aus progress §2:**

| Nr | Messung | Kurzablauf |
|---|---|---|
| O1 | **F7** Feldtext beim Objektwechsel | Läufe A–E wie in `docs\berichte\au3.md` (Wert ins Inspektorfeld tippen, ohne Eingabe auf anderes Objekt bzw. ins Leere klicken; Lauf E ist die Gegenprobe) |
| O2 | Bewegungsunschärfe Winkel 0° | Bitmap, *Filter ▸ Weichzeichnen ▸ Bewegungsunschärfe*, Winkel 0, Abstand 10 → Pixel vorher/nachher per JSF exportieren |
| O3 | Auswahl erweitern 1/5/10, begrenzen 5 | Pixelauswahl 20 × 20, *Auswählen ▸ Auswahlrahmen erweitern* 1, 5, 10 und *Rand* 5 → Grenzen der Auswahl je Wert |
| O4 | Animieren mit Richtung ≠ 0 | Symbol, *Modifizieren ▸ Animation ▸ Auswahl animieren…*, Richtung 45 / 90 → Lage je Status |
| O5 | Verlauf-Werkzeug | Inspektor ohne Auswahl, Farbfeld, Wechsel der Füll-/Strichfarbe, „Auswahl füllen“/Toleranz → Bildschirmfotos |
| O6 | Datei-Info-Register | *Datei ▸ Datei-Info*: Register und Felder, Fensterliste (DLG-11 hatte sie nicht messbar) |
| O7 | „Als Animation öffnen“ | 3 PNG unterschiedlicher Größe mit Kästchen öffnen → Leinwandgröße, Symbole oder Bitmaps je Status |
| O8 | Texturmenge 0 | Rechteck mit Textur, Menge 0 → Wirkung (sichtbar/unsichtbar), JSF-Wert |
| O9 | BMP-/pHYs-Rundung | Dokument 72/96/300 ppi als BMP und PNG exportieren → Header-Werte (pHYs, biXPelsPerMeter) |
| O10 | Tab-Folge D45, D47–D50 | jeden Dialog öffnen, Tab-Kette mit Fensterliste (Fokus-ID je Tab) |
| O11 | Fokus nach dem Klemmen | Wert außerhalb des Bereichs tippen, OK → welche Meldung, welches Feld hat danach den Fokus |
| O12 | Strg+Umschalt+K | Kürzel auslösen (mit und ohne Auswahl) → Wirkung, Menüeintrag |

**Batch-Assistent, `re\21-batch.md` §14 (für BATCH-1):**

| Nr | Kurzablauf (Einzelheiten im §14) |
|---|---|
| M1 | Batch-Verarbeitung öffnen, Dialog 19150 vermessen; 3 Dateien (eine doppelt) und „Alle hinzuf.“ in einem Ordner mit 5 PNG → Liste 6395 fotografieren |
| M2 | frisches Profil: Dateiauswahl → Weiter; Startzustand 2160/2167/2161, Kästchen 6402 |
| M3 | in 2167 jede Aufgabe hinzufügen, erneut, Befehl doppelt, Doppelklick links/rechts, ▲/▼ an den Enden |
| M4 | Ordner „Befehle“ in 2197 aufklappen → Baum oder flach, Wirkung von `//--not for batch--` |
| M5 | Export-Seite: Liste 2184 aufklappen, „Bearbeiten“ → welcher Dialog, neuer Listeneintrag |
| M6 | Skalieren: alle 4 Modi, Schieber 2168/2174 bis Anschlag, Werte 0 / 5000 / −3 |
| M7 | Schritt 2: „Benutzerdefinierte Position“ (Ordnerdialog?), Backups an/aus |
| M8 | Einstellungen ändern, Abbrechen, erneut öffnen; „Skript speichern“ (Titel, Name, Filter), Pref-Datei vorher/nachher |
| M9 | Lauf 2 PNG + 1 kaputte Datei, Export GIF → Fortschrittsfenster, Meldung 3194, Ort und Inhalt der Protokolldatei des Batchlaufs |
| M10 | „Export JPEG“ auf `a.png` im selben Ordner zweimal → Name, Überschreiben, Backup-Ordner |
| M11 | gespeichertes Skript aus M8 nur im Datenteil ansehen (Operationsliste, `outputDir`, Umlaut-Pfad). Das Skript bleibt **privat** und kommt nicht ins Projekt |
| M12 | Skript über *Befehle ▸ Skript ausführen…* starten, 2143 vermessen, „...“ → welcher Dateidialog |
| M13 | „nur Umbenennen“ (Präfix) gleiche Position / eigener Ordner; „Umbenennen + Skalieren“ ohne Export |

**Regeln für die Messungen:**
- Alle Batch-Läufe nur in `<Scratchpad>\ct4\batch\`.
- Jeden Dateidialog des Originals im selben Schritt schließen.
- Nach M9–M13 prüfen, dass keine Dateien außerhalb des Ordners entstanden sind.
- Ist eine Messung nicht möglich, wird sie ⏭ mit Grund; sie bleibt dann für eine spätere Sitzung offen.

---

## 7. FIX-1…n (60–105 min)

### 7.1 Eingang

1. **Jedes ❌** aus `gt1_…` und `ct1_…` bis `ct4_…`, Liste „Für FIX“.
2. **Fix-Liste aus progress §2:**

| Nr | Punkt | Herkunft | Hinweis |
|---|---|---|---|
| F1 | Standard-ALT-Text (`HtmlSettings::default_alt`) geht beim Speichern als FW-PNG verloren | E3 | in den FW-PNG-Tag aufnehmen, Rundlauf-Test |
| F2 | Tab-Folge in den Live-Effekt-Dialogen Farbton/Sättigung, Helligkeit/Kontrast, Unscharf maskieren | FORM-1 | Regel wie D41/D42/D46 |
| F3 | „Neues Symbol“ im Bibliothek-Bedienfeld mit der Anordnung von D14 | FORM-1 | – |
| F4 | Optimieren ▸ Einstellungen löschen: Original löscht direkt die gewählte Einstellung | MN-5 | Vorgabe-Abfrage entfernen, Rückfrage 3169 bleibt |
| F5 | `shortcuts.json` einmal gegen PARITAET_B §1 abgleichen | MN-5 | nur abweichende statische Kürzel |
| F6 | Alte Texte `dlg-md-*` entfernen | E3 | gehört zu TE-2; ist TE-2 gelaufen, nur prüfen |
| F7 | „Demo – aktuelles Dokument“ im echten Fenster | MN-4 | Ergebnis kommt aus CT-1; nur bei ❌ ein Fix |
| F8 | Erste Ziehbewegung bei Tausenden Objekten (2,2 s / 269 ms) | PERF-1 | Befund, keine Fehlfunktion; nur ändern, wenn klein und sicher, sonst in BEFUNDE lassen |

### 7.2 Verteilung

- Der Koordinator bündelt die ❌ nach Dateien bzw. Bereich zu kleinen Aufträgen: ein Agent je Bündel, **höchstens 6 FIX-Agenten** (Slots `fp-s3` … `fp-s8`). Zusammen mit GT-2 bleiben es ≤ 8 Agenten.
- **Messwerte aus CT-4** (O1–O12) gehen als Vorgabe in passende Fix-Aufträge. M1–M13 bleiben für BATCH-1 und werden nicht in der Nacht umgesetzt.

### 7.3 Regeln für parallele Fix-Agenten

1. **Ein Bündel, eigene Dateien:** Zwei Agenten bearbeiten nie dieselbe Datei. Braucht einer eine fremde Datei, trägt er einen Befund in `docs\berichte\fix<n>.md` ein, statt sie selbst zu ändern (§1).
2. **Warmer Slot (§7b):**
   - Anfang: `status --short` leer, `checkout -q --detach main`.
   - Im Slot arbeiten und bauen (`-j 3`).
   - Den Commit per `cherry-pick` bzw. Patch auf `main` im Hauptbaum bringen, mit Pfadangabe.
   - Präfix `FIX-<n>:`, letzte Zeile `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
3. **Prüfen (§7b.3):**
   - gezielter Test für den Fix in der eigenen Testdatei
   - `cargo check --workspace --all-targets`
   - bei Texten `cargo test -p fp-i18n` und der Scanner
   - `cargo test -p fp-neutral`, wenn Texte oder Bilder dazukommen
4. **Erneut im echten Fenster prüfen:**
   - Der ❌-Punkt wird nach dem Fix mit dem eigenen Release- oder Debug-Build über die Brücke wiederholt (eigene Instanz nach 1.3, Ecke vom Koordinator).
   - Ergebnis im Bericht: „✅ nach Fix, echtes Fenster, Brücke“ bzw. weiter ❌ mit Grund.
5. **Logikfehler zuerst (§4b):** Fixes für doppelte Effekte, falsches Rückgängig oder einen falschen Verlaufsschritt gehen vor Schönheitsfehlern.
6. **Bericht:** `docs\berichte\fix<n>.md` mit:
   - behobene ❌ (Paket und Wortlaut-Anfang) mit Commit
   - NACHPRUEFUNG-Zeilen, die jetzt ✅ sind
   - verbliebene Punkte
   - neue Befunde

   Keine gemeinsamen Doku-Dateien ändern.
7. **Zeitgrenze:** Ein Fix, der bis 105 min nicht grün ist, wird nicht halb committet. Der Stand kommt als Befund in den Bericht, der Slot bleibt sauber.

---

## 8. GT-2 – letzter Gesamtlauf und Endversion (105–135 min)

**Slot** `fp-s1`, Release in `fp-rel`.

1. **Warten**, bis alle FIX-Agenten fertig sind. Dann `git log --oneline` seit dem GT-1-Stand ins Protokoll.
2. **Tests:**
   - `cargo test --workspace -j 3`
   - voller Abschneide-Prüflauf (`FP_RAND_FULL=1`)
   - i18n-Scanner
   - `fp-neutral` inklusive `technical_english`

   Alles grün bzw. 0 Funde. Rote Tests, die GT-1 noch nicht hatte, sind neu durch FIX entstanden: den betreffenden Fix zurückdrehen ist **nicht** erlaubt. Stattdessen nennen und dem Koordinator melden.
3. **Doku übertragen:** ✅ und ❌ aus `ct1…ct4`, `fix<n>.md` und Messungen nach `NACHPRUEFUNG.md` (abhaken), `BEFUNDE.md`, `NUTZERBEFUNDE.md` (N01, N21, N22, N23, N24) und `STATUS.md`. Nur mit kleinen Edits und als Commit mit Pfadangabe (Präfix `GT-2:`).
4. **Endversion:**
   - Release-Build in `C:\Users\Administrator\Desktop\REA\fp-rel`: sauber, `git checkout -q --detach main`, `CARGO_TARGET_DIR=…\firepixel\target-rel`, `cargo build --release -p firepixel -p firepixel-cli`.
   - Neuer Ordner `C:\Users\Administrator\Desktop\Firepixel-Vorschau-<YYYY-MM-DD>_<HH-MM>`, ohne Doppelpunkte.
   - Inhalt: `firepixel.exe`, `firepixel-cli.exe`, die Anleitungen de/en (nach TE-2 unter ihrem englischen Namen) und **`NEU.md`**.
   - `NEU.md` sagt, was seit der Testversion vom 13.10. neu ist (Fixes, Messungen), was geprüft ist und was offen bleibt.
   - Die exe einmal mit eigenem Profil und `FP_NO_SHELL_OPEN=1` starten und über die Brücke `ui_state` abfragen, dann beenden.
5. **Abschlussbericht nur im Chat** an den Koordinator bzw. Nutzer:
   - Stand (Commit), Testzahlen
   - ✅/❌/⏭ je Paket, behobene Punkte
   - Messergebnisse in Kürze
   - ehrliche Liste dessen, was offen bleibt
   - Pfad der Endversion

   **Kein Push**, keine Meldung über andere Kanäle.
6. **Aufräumen:**
   - alle eigenen Instanzen beenden, keine offenen `#32770`-Fenster
   - Slots sauber auf `main`, Slots und `target-s<N>` nicht löschen
   - `target-rel\release` bleibt bis zur Freigabe des Nutzers stehen

Danach folgen CMD-1, BATCH-1 (mit M1–M13) und CI nur nach Entscheidung des Nutzers (progress §3).
