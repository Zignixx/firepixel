# Moderner Batch-Modus „Batch-Studio“ (Spezifikation SPEC-BATCH, 13.10.2026)

Nutzerwunsch (13.10.2026): Die Batch-Verarbeitung des Originals war eine gute Grundidee; Firepixel
macht sie **komplett modern und schön** neu. Viele Dateien (auch Videos) hinzufügen, per
Drag & Drop, eine Kette von Schritten zusammenklicken, live sehen, was herauskommt, und los.
Gestaltung wie der neue Startbildschirm (DES-2, `crates\fp-ui\src\welcome.rs`, `DESIGN.md` §12.1):
clean, Karten, Banner, ruhige Flächen.

Gebaut wird nachts von mehreren Agenten (§6). Dieses Dokument ist die einzige Vorgabe dafür.
Ausgangspunkt für den Umfang des Originals: `re\21-batch.md` (nur gelesen). Regeln:
`AGENTEN_REGELN.md` §1a (Neutralität), §1b (alle Texte in `.ftl`), §1c (Technisches englisch),
§4b (Logikfälle).

Begriffe in diesem Dokument:
- **Studio**: die neue Batch-Oberfläche.
- **Rezept** (`recipe`): geordnete Liste von **Schritten** (`steps`) plus Ausgabe-Einstellungen.
- **Auftrag** (`job`): ein Lauf eines Rezepts über eine Dateiliste.
- **Element** (`item`): eine Eingabedatei bzw. ein Video oder eine Animation in der Liste.

---

## 1. Ziel und Abgrenzung

### 1.1 Ziel

1. Ein **eigener Arbeitsmodus** im Hauptfenster (kein kleiner fester Dialog), erreichbar über
   *Datei ▸ Batch-Verarbeitung…*, eine Karte auf dem Startbildschirm und `firepixel.exe --batch`.
2. **Dateien rein → Rezept → Vorschau → Ausgabe**: alles auf einem Bildschirm, ohne Assistenten-Seiten.
3. **Anfänger-Modus** (Vorlagen und Baukästen, keine Fachbegriffe) und **Profi-Modus** (Regex,
   alle Parameter, Ausdrücke, Befehle des Registers, JSF).
4. **Nie Daten verlieren**: Vorgabe schreibt in einen neuen Ausgabeordner, Originale bleiben
   unberührt; Überschreiben nur mit ausdrücklicher Wahl, mit Sicherung.
5. **Dieselbe Pipeline** für Oberfläche, CLI (`firepixel-cli batch --recipe`), MCP und JSF.
6. Schnell: parallel auf allen Kernen, mit Speichergrenze, jederzeit abbrechbar.

### 1.2 Was vom Original enthalten sein muss (aus `re\21-batch.md`)

| Original (Abschnitt in 21) | Im Studio | Schritt/Ort |
|---|---|---|
| Dateiauswahl mit Liste, Hinzufügen, Alle hinzufügen, Entfernen (§2) | Dateiliste links, Drag & Drop, Ordner (rekursiv), Filter | §2.3 |
| „Aktuell geöffnete Dateien einschließen“ (§2.2, 6402) | Knopf „Offene Dokumente hinzufügen“; ungespeicherte werden als Speicherstand im Arbeitsspeicher verarbeitet, **ohne** Rückfrage „speichern?“ (Abweichung, sicherer) | §2.3 |
| Aufgabe **Exportieren** mit Vorgabe, „Einstellungen aus jeder Datei“, „Bearbeiten“ → Bildvorschau, Optimieren auf Größe (§4.3, §7.6) | Schritt `format.convert` mit Vorgaben der Exportliste, Option „Einstellungen der Datei“, Knopf „Feinabstimmen…“ öffnet die vorhandene Bildvorschau; `optimize.targetSize` | §3.2, §3.12 |
| Aufgabe **Skalieren** mit 4 Modi und „nur verkleinern“ (§4.4, §7.4) | `resize` mit Modi `none`, `exact`, `fit`, `fill`, `percent`, `width`, `height`, `longEdge`, `shortEdge`, Kästchen „Nur verkleinern“ | §3.3 |
| Aufgabe **Suchen und ersetzen** (Text, Schrift, Farbe, URL, Nicht-Web216) in Objektdateien (§4.5, §6, §7.5) | `document.findReplace` (nur FW-PNG/PSD/SVG-Ebenen, Hinweis wie im Original neutral formuliert) | §3.11 |
| Aufgabe **Umbenennen**: Ersetzen (erstes Vorkommen, wörtlich), Leerzeichen ersetzen, Präfix, Suffix (§4.6, §7.7) | Umbenennen-Baukasten (Anfänger) und Regex (Profi), deutlich mehr Bausteine | §4 |
| Aufgabe **Befehl** (`.jsf` aus dem Befehlsordner, ohne `//--not for batch--` und ohne gespeicherte Batch-Skripte) (§4.2, §7.3) | `script.command` (gespeicherter Befehl / `.jsf`), `script.registry` (beliebiger Befehl des Registers), `script.code` (JSF-Code) | §3.13 |
| Reihenfolge per ▲/▼ (§4.1) | Karten ziehen, Alt+↑/↓ | §2.4 |
| Ausgabe gleicher Ort / eigener Ordner (§5) | Ausgabe-Leiste: neuer Ordner (Vorgabe), gleicher Ort, eigener Ordner, Unterordner-Struktur beibehalten | §2.6 |
| Backups „Original Files“, überschreiben / inkrementell (§5, §7.3) | Konfliktregel „Original ersetzen“ nur mit Sicherung in `Original Files` (Name technisch englisch), Sicherung überschreiben oder `name-1.ext` … | §2.7 |
| Skript speichern / Skript ausführen (§5, §8) | Rezept speichern/laden (JSON, `.fpbatch.json`); zusätzlich „Als JSF exportieren“ (P3) erzeugt ein `.jsf`, das `fp.batch(recipe)` aufruft | §2.5 |
| Fortschritt „x von y Dateien“, Abbrechen, Ende-Meldung, Protokoll (§7.1, §7.2) | Fortschritt in der Ausgabe-Leiste und je Datei in der Liste, Abbrechen sofort (laufende Dateien werden sauber verworfen), Fehlerliste, Bericht (HTML/JSON), Protokoll `batch-log.txt` | §2.8 |
| „Nur Umbenennen“ ohne Öffnen der Datei (§7.3) | Erkennung „nur Dateischritte“ → Datei wird kopiert/verschoben, nicht dekodiert (auch für unlesbare Formate) | §5.3 |
| Export/Umbenennen immer zuletzt (§7.3) | Im Studio gilt die **sichtbare Reihenfolge**; Umbenennen und Speichern sind feste Endkarten („Dateiname“, „Speichern als“) und stehen immer unten. Damit ist das Verhalten des Originals ohne Überraschung abgebildet | §2.4 |
| Pfadlänge > 256 Zeichen (§7.3) | Prüfung im Trockenlauf; unter Windows mit `\\?\`-Pfaden schreiben, Warnung statt Abbruch | §2.7 |

**Bewusst nicht übernommen** (Eigenheiten/Fehler des Originals, `re\21-batch.md` §11): Abbruch des
ganzen Laufs bei einer fehlerhaften Datei (Studio: Datei als Fehler markieren, weiter), Zeichenklassen-
Ersetzung, „Umbenennen ohne Export“ mit zwei Dateien am Originalort, toter Beschneiden-Zweig,
festes Englisch in Meldungen, Mac-Dateitypen, unsichtbares Verändern bereits offener Dokumente
(Studio arbeitet immer auf einer Kopie; das offene Dokument bleibt unverändert).

### 1.3 Was neu ist (Überblick)

- Moderne Oberfläche mit Miniaturen, Drag & Drop, Ordner rekursiv, Filter, Live-Vorschau vorher/nachher
  mit Pixelgröße und **Dateigröße**, Trockenlauf.
- Rezepte als Karten, Vorlagen-Galerie („Für das Web verkleinern“, „Instagram-Quadrat“, „Fotos
  umbenennen nach Datum“, „Video → kleines GIF“, „Hintergrund entfernen“ …), Teilen als Datei.
- Umbenennen mit Baukasten (Nummern, Datum, EXIF-Datum, Bildgröße …) und Regex mit Klartext-Erklärung.
- Viele Formate: PNG, JPEG, GIF, WebP, AVIF, JPEG XL (P2), TIFF, BMP, TGA, QOI, ICO, WBMP, FW-PNG,
  PSD, SVG (über Nachzeichnen), PDF (P2), animierte GIF/APNG/WebP.
- **Video → animiertes Bild** (MP4/MOV/WebM/MKV → GIF/APNG/WebP) mit Zeitbereich, Bildrate, Größe.
- **Sehr kleine GIFs bei sehr guter Qualität** (eigener Optimierer, §3.10, §5.6).
- Animationen kürzen/zuschneiden/Geschwindigkeit/Schleife mit Zeitleisten-Vorschau.
- Metadaten behalten/entfernen/setzen (EXIF/XMP/IPTC/ICC, GPS gezielt entfernen).
- KI: Hintergrund entfernen (BiRefNet, vorhanden in `fp-ai`), Transparenz → Farbe, Motiv zentrieren.
- Wasserzeichen (Text/Bild), Rahmen, Schatten, Firepixel-Live-Effekte als Schritt.
- Optimieren auf Zielgröße in KB, verlustfreie PNG-Optimierung.
- Nachzeichnen → SVG im Batch (`fp-trace`).
- Jeder Befehl des Registers und JSF als Schritt.

---

## 2. Oberfläche

### 2.1 Einstieg und Modus

- **Ort**: Das Studio ersetzt beim Öffnen den Dokumentbereich (wie der Startbildschirm, `welcome.rs`
  zeichnet in ein `rect`). Menüleiste und Werkzeugleisten bleiben sichtbar; Bedienfelder und Inspektor
  werden ausgeblendet, solange das Studio vorn ist (Arbeitsbereich wird beim Schließen wiederhergestellt).
  Im Dokument-Reiterstreifen erscheint ein Reiter **„Batch“** mit Symbol, schließbar (×). Es gibt
  höchstens ein Studio.
- **Aufruf**: *Datei ▸ Batch-Verarbeitung…* (`file.batchProcess` öffnet in der Oberfläche das Studio;
  mit Parametern ohne Oberfläche bleibt das bisherige Verhalten), Startbildschirm-Karte „Neu erstellen“
  bekommt einen dritten Eintrag „Batch-Verarbeitung“ (P2), Dateien auf den Reiter „Batch“ ziehen,
  Kommandozeile `firepixel.exe --batch [dateien…]`, JSF `fw.doBatchProcess(files)` öffnet das Studio
  mit der Liste.
- **Mindestgröße**: 1024 × 640 Punkte für dreispaltig; darunter zweispaltig (Vorschau als Reiter neben
  dem Rezept), unter 720 px einspaltig mit Reitern „Dateien | Rezept | Vorschau“ (§2.9).

### 2.2 Aufbau (Maße auf dem 8er-Raster, `ui_kit::sp`)

```
┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│ Banner 72 px: [Logo 40] Batch-Verarbeitung              [Anfänger|Profi]  [Vorlagen ▾] [⋯]     │
│               23 Dateien · 412 MB · Rezept: „Für das Web“ (ungespeichert •)                     │
├──────────────────────────┬───────────────────────────────────────┬─────────────────────────────┤
│ ① DATEIEN  (Karte)       │ ② REZEPT  (Karte)                     │ ③ VORSCHAU (Karte)          │
│ [+ Dateien] [+ Ordner] ▾ │ ┌───────────────────────────────────┐ │ [Vorher | Nachher | Teilen] │
│ [Filter: *.png, >1 MB ▾] │ │ ⠿ ▣ Größe ändern          [●] ⋯ ▾│ │ ┌─────────────────────────┐ │
│ ┌──────────────────────┐ │ │   Längste Seite 1600 px,         │ │ │                         │ │
│ │▢ [36] urlaub_01.jpg  │ │ │   nur verkleinern                 │ │ │   Bild mit Teiler       │ │
│ │       4032×3024 3,1MB│ │ └───────────────────────────────────┘ │ │   ◀────│────▶          │ │
│ │▢ [36] urlaub_02.jpg ✓│ │ ┌───────────────────────────────────┐ │ │                         │ │
│ │▢ [36] clip.mp4  ▶0:12│ │ │ ⠿ ▣ Metadaten             [●] ⋯ ▾│ │ └─────────────────────────┘ │
│ │ …                    │ │ │   GPS entfernen, Rest behalten    │ │ Vorher 4032×3024 · 3,1 MB   │
│ └──────────────────────┘ │ └───────────────────────────────────┘ │ Nachher 1600×1200 · 284 KB  │
│ 23 Dateien, 2 Videos     │       [ + Schritt hinzufügen ]        │ −91 %   ⏱ 0,4 s            │
│ [Entfernen] [Leeren]     │ ┌ Dateiname (feste Endkarte) ───────┐ │ [◀ Datei 3/23 ▶] [Zoom ▾]  │
│                          │ │  urlaub_{n:3}  → urlaub_001.webp  │ │ Zeitleiste (bei Animation)  │
│                          │ ├ Speichern als (feste Endkarte) ───┤ │ ▕██████[====]█████▏ 0:02–0:07│
│                          │ │  WebP, Qualität 82                │ │                             │
│                          │ └───────────────────────────────────┘ │                             │
├──────────────────────────┴───────────────────────────────────────┴─────────────────────────────┤
│ Ausgabe 64 px: Ziel [Neuer Ordner „…\batch-output“ ▾] [Ordner…]  Bei Konflikt [Umbenennen ▾]   │
│ [Trockenlauf]   ▕██████████░░░░░░░░▏ 14/23 · 0:41 übrig      [Abbrechen]  [▶ 23 Dateien verarbeiten]│
└────────────────────────────────────────────────────────────────────────────────────────────────┘
```

| Bereich | Maß | Regel |
|---|---|---|
| Außenrand | `sp::L` 16 | wie Startbildschirm |
| Banner | Höhe 72 (schmal 56), Radius 8 | `welcome::paint_banner` wiederverwenden (Pixelfeld nach rechts dichter), Logo 40 px, Titel `TITLE` 15 fett, Untertitel `CAPTION` gedämpft |
| Spaltenabstand | `sp::L` 16 | |
| Karte ① Dateien | Breite 300 (min 260, max 380), ziehbar | Kartenkopf 34 px mit Symbolchip, Titel, Zähler rechts |
| Karte ② Rezept | Rest, min 360 | Schrittkarten Breite = Spalte − 2·12 |
| Karte ③ Vorschau | Breite 38 % (min 320, max 640), ziehbar | |
| Ausgabe-Leiste | Höhe 64 (zweizeilig 96 unter 1200 px Breite) | Fläche `dialog_footer_bg`, Linie oben wie §12.2 |
| Dateizeile | 44 px, Miniatur 36 px | wie `recent_row` im Startbildschirm (Schachbrett hinter Transparenz) |
| Schrittkarte zu | 56 px (Titel + Zusammenfassung) | auf: Höhe nach Inhalt, Beschriftung-Wert-Raster §5 |
| Knöpfe | Höhe 23 (Hauptknopf „Verarbeiten“ 32, Fläche `focus`) | |

Farben und Designs wie §12.1: Normal helle Karten mit weichem Schatten, Dunkel Karten heller als die
Fläche, OLED schwarz mit Leuchtrand je Karte in Gruppenfarbe (Dateien grün, Rezept cyan,
Vorschau orange). Schrittkarten tragen die Farbe ihrer **Gruppe** (§3) als 3-px-Leitstreifen links und
im Symbolchip. Nur Theme-Farben (`theme.rs`, `ui_kit::tint`, `card_bg`).

### 2.3 Karte ① Dateien

- **Hinzufügen**: Knopf „Dateien…“ (Mehrfachauswahl, Filter „Alle unterstützten“, Bilder, Animationen,
  Videos), Pfeil ▾ mit „Ordner…“, „Ordner mit Unterordnern…“, „Offene Dokumente“, „Aus Zwischenablage“
  (P2). **Drag & Drop** aus dem Explorer auf die ganze Karte (bzw. das ganze Studio): Dateien und
  Ordner; Ordner werden rekursiv gelesen (Kästchen in den Einstellungen ⋯: „Unterordner einbeziehen“,
  Vorgabe an). Beim Schweben: Karte getönt (`ui_kit::tint` focus), gestrichelter Rand, Text
  „Zum Hinzufügen loslassen“.
- **Doppelte** (gleicher kanonischer Pfad) werden nicht erneut aufgenommen; Hinweis „3 Dateien waren
  schon in der Liste“ als `notice` (Info) für 4 s.
- **Unbekannte Dateien** (keine Bild-/Video-Endung, nicht lesbar) werden nicht aufgenommen; Zähler
  „5 übersprungen“ mit Tooltip-Liste. Ausnahme: Rezept enthält **nur Dateischritte** → alle Dateien
  erlaubt (Umbenennen beliebiger Dateien).
- **Zeile**: Kästchen (für Auswahl-basierte Aktionen), Miniatur 36 px (im Hintergrund gelesen, Zwischen-
  speicher wie `welcome\thumbs.rs`, Video: erstes Bild + ▶-Abzeichen + Dauer, Animation: Abzeichen
  „GIF 48 Bilder“), Name (Mitte gekürzt), darunter gedämpft „4032×3024 · 3,1 MB · JPEG“ bzw. Ordner
  relativ zur Wurzel. Rechts Zustandssymbol: wartet (leer), läuft (Kreis), fertig ✓ grün, Warnung ⚠
  gelb, Fehler ✕ rot (Tooltip mit Meldung).
- **Filter** (Feld mit ▾): Suchtext im Namen, Typ (Bild/Animation/Video/FW-PNG), Größe (> / <), Pixel
  (> / <), mit/ohne Transparenz. Profi: Glob (`**/*.png`) und Regex. Gefilterte Dateien werden
  **nicht** verarbeitet; Zähler „23 von 40 Dateien“.
- **Sortierung** ▾: Name (natürlich: `bild2` vor `bild10`), Datum, Größe, Pixel, Hinzugefügt. Die
  Reihenfolge bestimmt die Nummerierung beim Umbenennen und „Bildfolge → Animation“. Manuell ziehen
  (P2).
- **Leerer Zustand**: `ui_kit::empty_state` groß: Symbol 48 px, „Bilder, Animationen oder Videos
  hierher ziehen“, darunter die Knöpfe „Dateien…“ / „Ordner…“, gedämpft die unterstützten Formate.
- **Tasten**: Entf entfernt, Strg+A, Strg+O fügt hinzu, Leertaste zeigt die Datei in der Vorschau,
  Eingabe öffnet die Datei als Dokument (P2).
- Große Listen: virtuelle Liste (nur sichtbare Zeilen zeichnen), getestet mit 10 000 Dateien.

### 2.4 Karte ② Rezept

- **Schrittkarte** (zugeklappt 56 px): Griff ⠿ (ziehen zum Umsortieren), Symbolchip in Gruppenfarbe,
  Titel (`SECTION` 12 fett), Zusammenfassung in Klartext (eine Zeile, gedämpft, z. B. „Längste Seite
  1600 px, nur verkleinern“), rechts Schalter an/aus (●), Menü ⋯ (Duplizieren, Nach oben, Nach unten,
  Entfernen, Als Vorlage speichern, Hilfe), Pfeil ▾ zum Aufklappen. Es ist immer **höchstens eine**
  Karte aufgeklappt (Akkordeon); die zuletzt hinzugefügte ist offen.
- **Aufgeklappt**: Parameter im Beschriftung-Wert-Raster (DESIGN §5), Schieber mit Zahlenfeld, Einheiten
  gedämpft. Anfänger-Modus zeigt nur die mit **A** markierten Parameter (§3); „Mehr Optionen“ klappt
  den Rest auf. Ungültige Werte: Feld mit roter Kante + `notice` Fehler in der Karte; die Karte zeigt
  im Kopf ⚠, „Verarbeiten“ ist gesperrt mit Tooltip.
- **„+ Schritt hinzufügen“**: öffnet die **Schritt-Galerie** als Überlagerung über der Rezept-Karte:
  Suchfeld oben (Fokus), Gruppen als Kacheln (wie „Neu erstellen“ im Startbildschirm, 54 px, Symbol,
  Titel, Erklärung), Pfeiltasten + Eingabe. Im Anfänger-Modus nur die häufigen Schritte (Markierung
  **A** in §3), Profi alle.
- **Feste Endkarten** (nicht entfernbar, immer unten): **„Dateiname“** (§4) und **„Speichern als“**
  (Format und Formatoptionen, §3.2). Damit sieht der Nutzer immer, was am Ende geschrieben wird.
- **Reihenfolge**: Ziehen mit Einfügelinie (2 px `focus`), Alt+↑/↓. Abhängige Hinweise: z. B.
  „Transparenz → Farbe“ nach „Speichern als JPEG“ ist unnötig, weil JPEG ohnehin keine Transparenz
  hat → Hinweis „JPEG füllt Transparenz automatisch mit Weiß; Farbe hier wählen“ (der Schritt
  `background.flatten` wird beim JPEG-Ziel automatisch mit Weiß vorgeschlagen).
- **Rückgängig/Wiederholen im Rezept**: Strg+Z/Strg+Y wirken im Studio auf das Rezept (eigener
  Verlauf, nicht der Dokumentverlauf). Ein Schieberzug = **ein** Schritt (§4b).
- **Validierung**: je Schritt `validate()` gegen die Element-Arten (z. B. „Zeitbereich“ nur für
  Animation/Video → bei Standbildern grau mit Hinweis „wirkt nur auf Animationen und Videos“).

### 2.5 Vorlagen und Rezepte

- Menü **„Vorlagen ▾“** im Banner: Galerie eingebauter Vorlagen (Karten mit Symbol, Titel, Satz), darunter
  „Meine Rezepte“, dann „Rezept öffnen…“, „Rezept speichern“, „Speichern unter…“, „Als JSF exportieren“
  (P3), „Leeres Rezept“.
- Speicherort: `%APPDATA%\Firepixel\BatchRecipes\*.fpbatch.json` (technisch englisch; `~/.config/
  firepixel/BatchRecipes` auf anderen Systemen; Pfad über `fp_i18n::profile::roaming_dir()`).
- **Teilen**: Datei weitergeben; Rezepte enthalten keine absoluten Pfade außer optional dem Ausgabeordner
  und Bild-Wasserzeichen (beim Speichern: Kästchen „Bilddateien einbetten“ → Base64 im JSON).
- Ein Rezept, das ein unbekannter (neuerer) Schritt enthält, lädt trotzdem: Karte „Unbekannter
  Schritt `xyz`“ (deaktiviert, Parameter erhalten).
- Eingebaute Vorlagen (P1 fett): **Für das Web verkleinern** (längste Seite 1920, WebP 82, Metadaten
  ohne GPS), **Als JPEG speichern** (Transparenz → Weiß, Qualität 85), **Fotos nach Datum umbenennen**
  (`{exif-date:YYYY-MM-DD}_{n:3}`), **Video → kleines GIF** (480 px, 12 fps, GIF-Optimierer
  „ausgewogen“), **Hintergrund entfernen** (KI → PNG), **Transparenz auf Farbe**, **Metadaten
  entfernen**, Quadrat 1080 (Instagram), Vorschaubilder 256, Favicon-Satz (P2), Sprite-Sheet (P2),
  Logos → SVG (P2).

**Rezept-Datei (JSON, Schlüssel englisch):**

```json
{
  "format": "firepixel-batch-recipe",
  "version": 1,
  "name": "Für das Web",
  "description": "",
  "steps": [
    { "id": "resize", "enabled": true, "params": { "mode": "longEdge", "size": 1600, "onlyShrink": true, "filter": "lanczos3" } },
    { "id": "metadata.edit", "enabled": true, "params": { "keep": "all", "remove": ["gps"] } },
    { "id": "background.flatten", "enabled": false, "params": { "color": "#ffffff" } }
  ],
  "rename": { "mode": "builder", "parts": [ { "kind": "original" }, { "kind": "text", "text": "_web" } ], "case": "lower", "spaces": "_" },
  "output": {
    "format": "webp", "options": { "quality": 82, "lossless": false },
    "target": { "kind": "newFolder", "path": null, "folderName": "batch-output", "keepStructure": true },
    "conflict": "rename", "backup": { "enabled": false, "incremental": false },
    "keepTimestamps": false
  }
}
```

Unbekannte Schlüssel werden beim Lesen ignoriert und beim Speichern erhalten (`#[serde(flatten)] extra`).
`version` steigt bei inkompatiblen Änderungen; ältere Versionen werden migriert.

### 2.6 Ausgabe-Leiste und Ziel

- **Ziel** ▾: **Neuer Ordner neben den Quellen** (Vorgabe, Name `batch-output`, bei Vorhandensein
  `batch-output-2` …; Name im Feld änderbar), **Eigener Ordner…**, **Gleicher Ort wie Original**
  (dann zwingend Konfliktregel §2.7). Kästchen „Ordnerstruktur beibehalten“ (bei rekursiv
  hinzugefügten Ordnern, Vorgabe an).
- **Bei Konflikt** ▾ (§2.7). **Zeitstempel übernehmen** (⋯, P2): Änderungsdatum der Quelle setzen.
- **Trockenlauf**: rechnet nur Namen, Ziele, Konflikte und geschätzte Größen (Schätzung aus der
  Vorschaudatei hochgerechnet, gekennzeichnet „≈“); zeigt eine Tabelle (Quelle → Ziel, Größe, Hinweis)
  im Vorschau-Bereich unter dem Reiter „Plan“. Nichts wird geschrieben.
- **Start** „▶ 23 Dateien verarbeiten“ (Zahl = gefilterte, aktivierte Elemente). Gesperrt bei: leerer
  Liste, ungültigem Schritt, fehlendem KI-Modell (Hinweis mit Knopf „KI-Modelle verwalten…“, Laden nur
  nach Zustimmung im vorhandenen Dialog), fehlendem Video-Decoder (§5.5).

### 2.7 Konfliktregeln

| Regel (`conflict`) | Wirkung | Vorgabe |
|---|---|---|
| `rename` „Neuen Namen wählen“ | `name (2).ext`, `name (3).ext` … | **ja** |
| `skip` „Überspringen“ | Datei bleibt, Element bekommt ⚠ „übersprungen“ | |
| `overwrite` „Überschreiben“ | nur bei Ziel ≠ Quelle ohne Rückfrage; Ziel = Quelle → Sicherung Pflicht | |
| `ask` „Jedes Mal fragen“ | Lauf pausiert, Rückfrage mit „Für alle übernehmen“ | |

- **Original ersetzen** (Ziel = Quelle) geht nur mit Sicherung: Unterordner `Original Files` (technisch
  englisch, wie das Original-Pref) im Quellordner; „Sicherungen behalten“ = inkrementell `name-1.ext`.
  Schreiben immer erst in eine Temporärdatei im Zielordner, dann atomar umbenennen; erst danach wird
  das Original verschoben. Bricht etwas ab, bleibt das Original unversehrt.
- **Konflikte innerhalb des Laufs** (zwei Quellen ergeben denselben Zielnamen, z. B. `a.png` und
  `a.jpg` → `a.webp`): im Trockenlauf und vor dem Start als Warnung angezeigt; aufgelöst nach der
  gewählten Regel in Listenreihenfolge (deterministisch, auch bei paralleler Abarbeitung: Ziele werden
  **vor** dem Start vergeben, §5.2).
- Ungültige Zeichen (`<>:"/\|?*`, Steuerzeichen, Endpunkt/Leerzeichen am Ende, reservierte Namen `CON`,
  `NUL`, `COM1` …) werden durch `_` ersetzt, Hinweis in der Namensvorschau. Länge > 255 je Name wird
  gekürzt (Endung bleibt), > 260 Gesamtpfad: `\\?\`-Pfad, Warnung im Trockenlauf.

### 2.8 Fortschritt, Abbrechen, Fehler, Bericht

- Während des Laufs: Rezept und Liste sind gesperrt (gedämpft, Hinweis „Läuft …“), die Vorschau zeigt
  die zuletzt fertige Datei (vorher/nachher). Leiste: Balken (`ui_kit::progress`), „14 von 23“,
  Restzeit (gleitender Mittelwert), Durchsatz, Abbrechen.
- **Abbrechen**: setzt `CancelToken`; laufende Schritte prüfen ihn zwischen Kacheln/Bildern/Frames
  (spätestens alle 100 ms); Temporärdateien werden gelöscht; fertige Dateien bleiben. Meldung
  „Abgebrochen: 14 fertig, 9 nicht verarbeitet“.
- **Ende**: Zusammenfassung als Karte über der Vorschau: ✓ 21 fertig, ⚠ 1 übersprungen, ✕ 1 Fehler;
  Summe vorher/nachher (z. B. 412 MB → 38 MB, −91 %), Dauer; Knöpfe „Ordner öffnen“ (nur ohne
  `FP_NO_SHELL_OPEN`), „Bericht speichern…“, „Fehlgeschlagene erneut“, „Neues Rezept“. Elemente in der
  Liste behalten ihren Zustand; Filter „nur Fehler“.
- **Fehlerliste**: Reiter „Fehler (1)“ in der Vorschau-Karte: Datei, Schritt, Meldung (übersetzt),
  Knopf „Kopieren“.
- **Bericht**: `batch-report.json` (Schlüssel englisch: `recipe`, `started`, `finished`, `items[]` mit
  `source`, `target`, `status`, `bytesIn`, `bytesOut`, `ms`, `warnings`, `error`) und optional
  `batch-report.html` (P2) im Ausgabeordner; Protokoll `batch-log.txt` (englisch, technisch) unter
  `%LOCALAPPDATA%\Firepixel\Logs\`.
- Fenster schließen während des Laufs: Rückfrage „Batch läuft noch – abbrechen und schließen?“.

### 2.9 Vorschau (Karte ③)

- Reiter: **Vorher | Nachher | Vergleich** (Teiler ziehbar, Vorgabe Vergleich), **Plan** (Trockenlauf-
  Tabelle), **Fehler** (nur wenn > 0).
- Zeigt die in der Liste markierte Datei (sonst die erste). ◀ ▶ blättert, Zoom ▾ (Einpassen, 100 %,
  200 %), Mausrad zoomt um die Maus, Ziehen verschiebt; Schachbrett hinter Transparenz; Hintergrund
  wählbar (Schachbrett, Weiß, Schwarz).
- Unter dem Bild: „Vorher 4032×3024 · 3,1 MB“ / „Nachher 1600×1200 · 284 KB (−91 %)“ / Rechenzeit.
  Die Dateigröße „Nachher“ entsteht durch echtes Kodieren der Vorschaudatei im Hintergrund (volle
  Größe); bis dahin „≈“ mit Schätzung.
- **Animationen/Videos**: Abspielen ▶/⏸, Zeitleiste mit Miniaturstreifen, Griffe für Anfang/Ende
  (Schritt `anim.trim`), Bildzähler, Bildrate; Vorschau nachher spielt die Ausgabe (GIF/APNG/WebP)
  ab, inklusive tatsächlicher Palette und Dithering.
- **Schnelle Vorschau**: Rechnung auf einem Proxy (längste Seite ≤ 1024 px, bei Animationen ≤ 480 px
  und höchstens 150 Frames), entprellt 150 ms nach der letzten Änderung, jede Änderung bricht die
  laufende Vorschau ab (Generationszähler). KI-Schritte zwischenspeichern ihre Maske je Datei.
  Schritte mit `preview: false` (§3) werden in der Vorschau übersprungen und als Hinweis genannt.
- Bei schmaler Breite: Vorschau als Reiter in der Rezept-Spalte; einspaltig: Reiter oben.

### 2.10 Anfänger- und Profi-Modus

| | Anfänger (Vorgabe beim ersten Start) | Profi |
|---|---|---|
| Schritt-Galerie | ≈ 20 häufige Schritte, Klartext-Namen | alle Schritte, Suche nach ID, Register-Befehle, JSF |
| Parameter | nur **A**-Parameter, „Mehr Optionen“ | alle, Zahlen mit Einheiten, Ausdrücke in Feldern (`{width}/2`, P3) |
| Umbenennen | Baukasten (§4.1) | Baukasten **und** Regex (§4.2) |
| Filter | Suche, Typ | zusätzlich Glob, Regex, Größe/Pixel |
| Ausgabe | Ziel, Konflikt | Struktur, Zeitstempel, Parallelität, Speichergrenze, Bericht |

Umschalter im Banner (Segment-Knopf). Der Modus wird gemerkt (`BatchMode` in den Einstellungen).
Wechsel verliert nie Werte: im Anfänger-Modus versteckte Parameter behalten ihre Werte und werden in
der Zusammenfassung genannt („+ 2 Profi-Einstellungen“).

### 2.11 Tastatur, Barrierefreiheit, Texte

- Tab-Reihenfolge: Banner → Dateien → Rezept → Vorschau → Ausgabe. Fokusrahmen in `focus` wie §12.1.
  Strg+Eingabe startet, Esc bricht Überlagerungen ab (nicht den Lauf), F5 = Trockenlauf.
- Jedes Bedienelement mit egui-Barrierefreiheitsname (für `ui_find`).
- **Alle Texte** in `crates\fp-i18n\locales\{de,en}\batch*.ftl` (Bereich `batch`, aufgeteilt in
  Dateien je Paket, §6.2), Schlüsselpräfix `batch-`; Schrittnamen `batch-step-<id>`, Parameter
  `batch-param-<id>-<param>`, Tooltips `.tooltip`. Keine festen Texte im Code. Zahlen, Größen und
  Datumsangaben nach Sprache formatiert (Dezimalkomma DE). `qps` darf nirgends überlaufen
  (`clipping_audit` 0 Funde, alle 3 Designs, 1024×768 und 1920×1080, 100/150 %).

---

## 3. Katalog der Schritte

Spalten: **ID** (technisch, englisch, im Rezept), **Parameter** (Bereich, *Vorgabe*; **A** = im
Anfänger-Modus sichtbar), **V** = Live-Vorschau ja/nein, **Verfahren/Bibliothek**, **P** = Priorität
(**P1** erste Nacht, P2 zweite Runde, P3 später). Alle Schritte wirken auf ein `Item`
(§5.1: Einzelbild, Animation, Video-Stream, Dokument oder „nur Datei“). Gruppenfarben: Dateien grün,
Format blau, Geometrie cyan, Farbe violett, Hintergrund orange, Überlagerung gelb, Metadaten grau,
Animation rosa, Optimieren grün-türkis, Vektor/Skript neutral.

### 3.1 Dateien und Umbenennen (Gruppe „Dateien“)

| ID | Name | Parameter | V | Verfahren | P |
|---|---|---|---|---|---|
| `rename` (Endkarte) | Dateiname | siehe §4 | Namensvorschau | eigen (`regex` 1.x) | **P1** |
| `file.copyOnly` | Nur kopieren/verschieben | **A** `action` copy/move *copy* | – | `std::fs`, ohne Dekodieren | **P1** |
| `file.organize` | In Unterordner sortieren | **A** `by`: date (`YYYY/MM`), type, size class, orientation, exif camera; `pattern` frei (`{exif-date:YYYY}/{ext}`) | Plan | eigen | P2 |
| `file.filterSkip` | Nur bestimmte verarbeiten | Bedingung (Breite/Höhe/Größe/Transparenz/Animation/Name-Regex) → Rest überspringen oder nur kopieren | – | eigen | P2 |
| `file.dedupe` | Doppelte Bilder erkennen | `method` exact (SHA-256) / similar (dHash, Abstand 0…16 *4*); Aktion: überspringen/markieren | – | `sha2` (vorhanden), dHash eigen | P3 |
| `file.timestamps` | Datum setzen | Änderungsdatum = Quelle / EXIF-Aufnahme / fest | – | `filetime` 0.2 (MIT/Apache) | P2 |

### 3.2 Format und Konvertieren (Gruppe „Format“; Endkarte „Speichern als“)

| ID | Name | Parameter | V | Verfahren | P |
|---|---|---|---|---|---|
| `output` (Endkarte) | Speichern als | **A** `format`: same, png, jpeg, webp, avif, gif, apng, tiff, bmp, tga, qoi, ico, wbmp, fwpng, psd, jxl (P2), pdf (P2), svg (über `vector.trace`); Formatoptionen siehe unten | ja (Größe) | `fp-export` (vorhanden) | **P1** |
| – PNG | | **A** `bitDepth` 8/24/32/auto *auto*; `palette` 2…256; `interlace` aus; `compression` fast/default/max *default* | ja | `png`, `fp-export::png_out`; max → `optimize.png` | **P1** |
| – JPEG | | **A** `quality` 1…100 *85*; `progressive` an; `subsampling` 4:4:4/4:2:2/4:2:0 *auto (≥90 → 4:4:4)*; `background` für Transparenz *#ffffff* | ja | `fp-export::jpeg` (eigen) | **P1** |
| – WebP | | **A** `quality` 0…100 *82*; **A** `lossless` aus; `alphaQuality` 0…100 *100*; `method` 0…6 *4* | ja | `webp` 0.3.1 (libwebp statisch, BSD-3) | **P1** |
| – AVIF | | **A** `quality` 1…100 *60*; `speed` 1…10 *6*; `alphaQuality` | ja | `ravif` 0.13 (BSD-3) | **P1** |
| – GIF (Standbild) | | **A** `colors` 2…256 *256*; `dither` none/ordered/floydSteinberg/blueNoise *floydSteinberg*; `ditherAmount` 0…100 *80*; `transparency` | ja | `fp-export` Quantisierer + §5.6 | **P1** |
| – TIFF/BMP/TGA/QOI/WBMP | | TIFF `compression` none/lzw/deflate/packbits *lzw* | ja | `fp-export::formats` (vorhanden) | **P1** |
| – ICO | | `sizes` [16,24,32,48,64,128,256] *alle ≤ Bild*; `pngFrom` 256 | ja | `fp-export::write_ico` | P2 |
| – FW-PNG | | – (Dokument bleibt bearbeitbar) | – | `fp-fwpng` | **P1** |
| – PSD | | `layers` an (aus FW-PNG/PSD-Quellen) | – | `fp-export::psd_out` | P2 |
| – JPEG XL | | `lossless` an / `distance` 0…15 *1* | ja | Lizenz/Reife prüfen: `zune-jpegxl` (nur verlustfrei, MIT/Apache/Zlib) bzw. `jxl-encoder` falls permissiv; sonst P3 | P2 |
| – PDF | | `pageSize` Bild/A4/Letter, `fit`, ein PDF je Bild oder **alle in eins** | – | eigener Schreiber (`fp-export` hat PDF-Bausteine für Vektorausgabe) | P2 |
| `format.colorDepth` | Farbtiefe | 8/16 Bit, Graustufen, indiziert | ja | eigen | P3 |
| `format.colorProfile` | Farbprofil | `mode` keep / convertToSrgb / assignSrgb / strip *convertToSrgb* | ja | ICC lesen eigen; Umrechnung `qcms` 0.3 (MPL-2.0, **nur** wenn MPL zugelassen, sonst eigene Matrix-/TRC-Umrechnung für RGB-Matrix-Profile) | P2 |
| `format.multiPage` | Mehrseitig/Ebenen aufteilen | Quelle mit Seiten/Ebenen/Frames (FW-PNG, PSD, TIFF, ICO, GIF) → je Seite/Ebene/Frame eine Datei; `{page}`/`{layer}`/`{frame}` im Namen | – | `fp-io`, `fp-model` | P2 |

**Ziel = „wie Quelle“** behält das Format (bei unlesbarer Ausgabe, z. B. HEIC/PSD → PNG mit Hinweis).
Die Endung folgt immer dem Ausgabeformat (`.jpg`, `.webp`, `.fw.png`); Endungsschreibweise ⋯
(klein/groß, `jpg`/`jpeg`).

### 3.3 Größe, Zuschnitt, Drehen, Leinwand (Gruppe „Geometrie“)

| ID | Name | Parameter | V | Verfahren | P |
|---|---|---|---|---|---|
| `resize` | Größe ändern | **A** `mode`: percent, width, height, longEdge, shortEdge, fit (in Rahmen), fill (füllen + zuschneiden), exact (verzerrt), none; **A** `width`/`height`/`size` 1…32768 px *1600*; `percent` 1…10000 *50*; **A** `onlyShrink` an; `filter` nearest/bilinear/bicubic/lanczos3/auto *auto* (Pixelart → nearest); `dpi` optional | ja | `fp-raster` Skalierer (vorhanden; `modify.canvas.imageSize` für Dokumente), Lanczos eigen, rayon | **P1** |
| `crop` | Zuschneiden | **A** `mode`: aspect (1:1, 4:3, 3:2, 16:9, 9:16, frei), rect (px/%), edges (oben/unten/links/rechts px/%); **A** `anchor` 9 Punkte *Mitte*; `smart` an → Ausschnitt nach Motiv (BiRefNet-Maske bzw. Kantenenergie) | ja | eigen; smart: `fp-ai` (P2) | **P1** (smart P2) |
| `trim` | Ränder abschneiden | **A** `what` transparent / color (Ecke) / both; `tolerance` 0…255 *8*; `padding` 0…500 px *0* | ja | eigen | **P1** |
| `rotate` | Drehen/Spiegeln | **A** `angle` 90/180/270/frei (−360…360); **A** `flip` none/h/v; `autoExif` an (nach EXIF-Ausrichtung drehen, Tag danach 1); `background` bei freiem Winkel *transparent* | ja | eigen (90° verlustfrei per Umordnung; frei: bilinear/bicubic) | **P1** |
| `canvas` | Leinwand/Rand | **A** `mode`: addBorder (px je Seite), toSize (B×H), toAspect, square; **A** `color` *transparent*; `anchor` *Mitte* | ja | eigen; Dokumente: `modify.canvas.canvasSize` | **P1** |
| `canvas.pad` | Auf Vielfaches auffüllen | Vielfaches von 2/4/8/16 (Video/Kacheln) | ja | eigen | P3 |
| `perspective` | Gerade richten | Winkel automatisch (Hough) oder fest | ja | eigen | P3 |
| `tile.split` | In Kacheln teilen | Raster N×M oder Kachelgröße; Name `{row}_{col}` | – | eigen | P2 |

### 3.4 Farbe, Korrektur, Filter, Effekte (Gruppe „Farbe und Effekte“)

| ID | Name | Parameter | V | Verfahren | P |
|---|---|---|---|---|---|
| `adjust.auto` | Automatisch verbessern | **A** `levels` an, `contrast` an, `whiteBalance` aus | ja | `filter.adjustColor.autoLevels` + eigener Weißabgleich (Grauwelt) | **P1** |
| `adjust.basic` | Helligkeit/Kontrast/Sättigung | **A** `brightness` −100…100 *0*; **A** `contrast` −100…100 *0*; **A** `saturation` −100…100 *0*; `hue` −180…180; `gamma` 0,1…5 *1*; `temperature` −100…100; `tint` | ja | Register `filter.adjustColor.brightnessContrast`, `…hueSaturation` + eigen | **P1** |
| `adjust.levels` / `adjust.curves` | Tonwert / Kurven | wie Dialoge (Punkte als JSON); „aus Vorlage“ | ja | Register `filter.adjustColor.levels`/`curves` | P2 |
| `adjust.grayscale` | Schwarzweiß/Sepia/Duoton | **A** `style` gray/sepia/duotone; Farben | ja | eigen | **P1** |
| `adjust.invert` | Umkehren | – | ja | Register `filter.adjustColor.invert` | **P1** |
| `adjust.posterize` / `adjust.threshold` | Tontrennung/Schwellenwert | Stufen 2…64; Schwelle 0…255 | ja | eigen | P2 |
| `filter.sharpen` | Schärfen | **A** `amount` 0…500 % *60*; `radius` 0,1…10 *1*; `threshold` 0…255 *0* | ja | Register `filter.sharpen.unsharpMask` | **P1** |
| `filter.blur` | Weichzeichnen | **A** `kind` gaussian/motion/radial/zoom; `radius` 0…250 *2* | ja | Register `filter.blur.*` | **P1** |
| `filter.noise` | Rauschen hinzufügen/entfernen | add: Menge; remove: Median 1…10 | ja | Register `filter.noise.addNoise`; Median eigen | P2 |
| `filter.pixelate` | Verpixeln (ganz/Bereich) | Blockgröße 2…200 | ja | eigen | P2 |
| `filter.registry` | Beliebiger Filter | Auswahl aus allen `filter.*`-Befehlen des Registers mit ihrem Schema (Formular aus `describe_command`) | ja | Befehlsregister | **P1** |
| `effect.live` | Firepixel-Live-Effekt | **A** `effect` dropShadow/innerShadow/glow/innerGlow/bevel/emboss/… (Liste aus `element.addEffect`); `params` je Effekt; wirkt auf die Bitmap als Element, danach flach rechnen (Leinwand wächst um den Effektrand, Kästchen „Leinwand erweitern“ an) | ja | `element.addEffect` + `fp-effects`/`fp-render` | **P1** |
| `effect.style` | Stil anwenden | Stil aus der Stilbibliothek | ja | Register | P2 |
| `adjust.lut` | Farb-LUT (.cube) | Datei, Stärke 0…100 | ja | eigener `.cube`-Leser, trilinear | P3 |
| `ai.upscale` | KI-Vergrößern ×2/×4 | Modell (Real-ESRGAN-Familie, BSD-3, über `ai.manageModels`) | ja (Ausschnitt) | `fp-ai` (neues Netz, eigene CPU-Inferenz) | P3 |
| `ai.faceBlur` | Gesichter unkenntlich | Stärke | ja | Modell nötig (Lizenz prüfen) | P3 |

### 3.5 Hintergrund und Transparenz (Gruppe „Hintergrund“)

| ID | Name | Parameter | V | Verfahren | P |
|---|---|---|---|---|---|
| `background.remove` | Hintergrund entfernen (KI) | **A** `model` birefnetLite/birefnet *birefnetLite*; **A** `edge` hard/antiAlias/feather *antiAlias*; `tolerance` 0…100 *50*; `expand` −100…100; `smooth` 0…100; `refineEdge` (Haare, `birefnetLiteMatting`) aus; `output` transparent / mask only / keep + mask as alpha | ja (Maske zwischengespeichert) | `fp-ai` BiRefNet (vorhanden, `select.aiSubject`, Feineinstellungen wie `docs\AGENTS.md`); Modell fehlt → Hinweis + „KI-Modelle verwalten…“ | **P1** |
| `background.removeColor` | Farbe entfernen (ohne KI) | **A** `color` *Ecke oben links*/Pipette; **A** `tolerance` 0…255 *24*; `contiguous` an (nur von den Rändern aus); `feather` 0…10 | ja | eigen (Flutfüllung wie `bitmap.floodFill`) | **P1** |
| `background.flatten` | Transparenz → Farbe | **A** `color` *#ffffff*; `kind` color/gradient/image/blur (unscharfe Kopie des Bildes) *color*; `onlyIfTransparent` an | ja | eigen (Alpha-Mischung in linearem Licht optional) | **P1** |
| `background.replace` | Hintergrund ersetzen | wie `remove` + `flatten` in einem (Farbe/Verlauf/Bild/Weichzeichnen) | ja | Kombination | P2 |
| `background.subjectCenter` | Motiv zentrieren | Rand 0…50 % *10*, quadratisch an/aus, Schatten unter dem Motiv aus | ja | BiRefNet-Maske → Box → `canvas` | P2 |
| `background.alphaCleanup` | Alpha aufräumen | Halbtransparenz < x → 0, > y → 255; Farbsaum entfernen (Defringe) 0…5 px | ja | eigen | P2 |
| `background.shadow` | Schatten hinzufügen | wie `effect.live` dropShadow, Vorgaben für Produktbilder | ja | `fp-effects` | P2 |

### 3.6 Wasserzeichen, Text, Rahmen (Gruppe „Überlagerung“)

| ID | Name | Parameter | V | Verfahren | P |
|---|---|---|---|---|---|
| `overlay.text` | Text/Wasserzeichen | **A** `text` mit Platzhaltern (`{name}`, `{date}`, `{exif-date}`, `{n}`, `©`), **A** `position` 9 Punkte / gekachelt / frei; **A** `size` % der Bildbreite 1…50 *4*; `font` (Systemschriften wie Textwerkzeug), `color`, `opacity` 0…100 *60*, `margin` %, `rotation`, `shadow`/`outline` | ja | `fp-text` + `fp-render` (vorhanden) | **P1** |
| `overlay.image` | Logo/Bild einfügen | **A** `image` (Datei, im Rezept einbettbar); **A** `position`; **A** `scale` % der Bildbreite *15*; `opacity`; `margin`; `blend` (Mischmodi des Registers) | ja | `fp-render` | **P1** |
| `overlay.border` | Rahmen | **A** `width` px/% ; **A** `color`; `radius` (abgerundete Ecken, außen transparent) 0…50 %; `inner`/`outer` | ja | eigen | **P1** |
| `overlay.roundCorners` | Ecken abrunden | Radius px/% | ja | eigen (AA) | P2 |
| `overlay.vignette` | Vignette | Stärke, Größe | ja | eigen | P3 |
| `overlay.contactSheet` | Kontaktbogen | Raster, Abstand, Beschriftung; **mehrere Dateien → ein Bild** | ja | eigen (Sammel-Schritt, §5.1) | P2 |

### 3.7 Metadaten (Gruppe „Metadaten“)

| ID | Name | Parameter | V | Verfahren | P |
|---|---|---|---|---|---|
| `metadata.edit` | Metadaten | **A** `keep`: all / none / chosen *all*; **A** `remove`: [gps, camera (Make/Model/Seriennummer), software, thumbnails, xmp, iptc, comments, icc] *[gps]*; Profi: Liste einzelner EXIF-Tags | Info-Tabelle vorher/nachher | Lesen: `kamadak-exif` 0.6 (BSD-2); Schreiben/Entfernen: Container-Ebene mit `img-parts` 0.3/0.4 (MIT/Apache, JPEG/PNG/WebP) und eigenem EXIF-TIFF-Schreiber (Tag-Liste wird gefiltert, Offsets neu); AVIF/JXL/TIFF: eigener Box-/IFD-Code | **P1** (keep/none/gps) · P2 (Einzel-Tags) |
| `metadata.set` | Metadaten setzen | **A** `artist`, **A** `copyright`, `description`, `keywords` (XMP `dc:subject` + IPTC), `title`, `rating` 0…5, `dateTaken` (fest oder verschieben ± Zeit), `gps` setzen/entfernen; Platzhalter erlaubt | Info | EXIF eigen; XMP-Paket mit `xmp-writer` 0.3 (MIT/Apache); IPTC-IIM eigen (einfach) | P2 |
| `metadata.orientation` | Ausrichtung anwenden | EXIF-Ausrichtung in Pixel umsetzen, Tag = 1 | ja | eigen (= `rotate.autoExif`) | **P1** |
| `metadata.export` | Metadaten als Datei | JSON/CSV je Datei oder gesammelt | – | `kamadak-exif` | P3 |

Regel: Beim Konvertieren werden Metadaten **übernommen**, soweit das Zielformat sie tragen kann
(EXIF/XMP/ICC in JPEG, PNG `eXIf`/`iTXt`, WebP, AVIF, TIFF); GIF/BMP/TGA/QOI/ICO verlieren sie mit
Hinweis „Format speichert keine Metadaten“. Ohne Schritt „Metadaten“ gilt `keep: all`, die
Ausrichtung wird angewendet und auf 1 gesetzt.

Alternative zum eigenen EXIF-Schreiber: `little_exif` (MIT/Apache, liest und schreibt EXIF in JPEG, PNG,
WebP, JXL, HEIF, TIFF; noch vor 1.0, bekannte Lücken bei JPEG mit APP12/13). Entscheidung beim Bau:
`little_exif` nur, wenn seine Tests auf unserem Korpus grün sind, sonst eigener Code (≈ 400 Zeilen).

### 3.8 Animation und Video (Gruppe „Animation“)

Animationen (GIF, APNG, animiertes WebP, animiertes AVIF, JPEG-XL-Animation, FW-PNG mit Status) und
Videos werden als **Frame-Folge** behandelt (`Item::Animation`, §5.1). Videos werden **gestreamt**
(nicht ganz in den Speicher).

| ID | Name | Parameter | V | Verfahren | P |
|---|---|---|---|---|---|
| `video.import` (automatisch bei Videos, Karte sichtbar) | Video lesen | **A** `start`/`end` (Zeit, ms-genau) *ganz*; **A** `fps` 1…60 *12*; `maxFrames` 1…3000 *600*; `decoder` auto/builtin/ffmpeg | ja (Zeitleiste) | §5.5 | **P1** (über ffmpeg) · P2 (eingebaut H.264/AV1) |
| `anim.trim` | Kürzen | **A** `start`/`end` (Zeit oder Bildnummer) mit Zeitleiste und Griffen; `removeFrames` Liste/Bereiche (`1-3, 10, 20-`) | ja (Zeitleiste) | eigen | **P1** |
| `anim.speed` | Geschwindigkeit | **A** `factor` 0,1…10 *1*; oder `fps` fest; `minDelay` 20 ms (Browser runden < 20 ms auf 100 ms!) | ja | eigen | **P1** |
| `anim.loop` | Schleife | **A** `count` 0 = endlos…65535; `pingPong` (vor-zurück) aus; `reverse` aus | ja | eigen | **P1** |
| `anim.dropFrames` | Bilder ausdünnen | jedes n-te behalten / Ziel-fps; ähnliche zusammenfassen (Differenz < x → Dauer addieren) *an, x = 0,5 %* | ja | eigen | **P1** |
| `anim.frameDelay` | Bilddauer | alle gleich (ms) / erstes/letztes länger (Pause am Ende 0…10 s) | ja | eigen | P2 |
| `anim.toFrames` | Animation → Bildfolge | Format je Frame, Name `{name}_{frame:4}` | – | eigen | **P1** |
| `anim.fromFrames` | Bildfolge → Animation | **Sammel-Schritt**: alle (gefilterten) Elemente in Listenreihenfolge → eine Animation; `delay` ms *100*; `size` erste/größte/fest | ja | eigen | **P1** |
| `anim.spriteSheet` | Sprite-Sheet | Richtung: Animation/Elemente → Sheet (Spalten, Abstand, Rand, JSON/CSS-Atlas `{frames:[{x,y,w,h}]}`) **und** Sheet → Frames (Raster N×M) | ja | `fp-export::sprites` (vorhanden) erweitern | P2 |
| `anim.crop`, `anim.resize`, Farbe … | – | Alle Geometrie-/Farbschritte wirken auf jedes Frame (gleiche Parameter, Rechnung parallel je Frame) | ja | wie Standbild | **P1** |
| `anim.firstFrame` | Standbild aus Animation | Frame-Nr. / Zeit / „schärfstes“ / Mitte | ja | eigen | P2 |
| `anim.stabilize` | Ruhiger Hintergrund | statische Bereiche über die Zeit einfrieren (Rauschen entfernen, hilft der Kompression) | ja | eigen (§5.6 „zeitliche Glättung“) | P2 |
| `anim.boomerang`, `anim.crossfade` | Spezial | Überblendung am Schleifenende (n Frames) | ja | eigen | P3 |

Ausgabe einer Animation über „Speichern als“: **GIF** (mit Optimierer §3.10), **APNG** (`fp-export::
animation::apng`, Optimierung der Frames: Differenz-Rechteck, Blend-Op), **WebP animiert**
(`animated_webp`, verlustbehaftet/verlustfrei), **AVIF animiert** (P3, braucht AV1-Sequenz-Encoder;
`ravif` kann nur Standbilder), **Bildfolge** (`anim.toFrames`), **MP4** – nicht vorgesehen (Encoder-
Patente; P3 nur über ffmpeg des Nutzers).

### 3.10 GIF-Optimierer „kleine GIFs, sehr gute Qualität“ (Teil von „Speichern als GIF“)

Parameter in der Endkarte bei Format GIF (Anfänger sieht nur **Qualität** und **Zielgröße**):

| Parameter | Bereich, *Vorgabe* | Wirkung |
|---|---|---|
| **A** `preset` | smallest / balanced / best *balanced* | setzt die folgenden Werte |
| **A** `quality` | 1…100 *80* | steuert Farben, Dithering-Stärke, Verlust |
| **A** `maxKB` | 0 (aus) … 100 000 | Zielgröße: Suche über Qualität → fps → Größe (§3.12) |
| `colors` | 2…256 *256* | höchste Palettengröße |
| `palette` | global / perFrame / auto *auto* | auto: global, wenn der Fehler < Schwelle, sonst je Frame (lokale Farbtabelle) |
| `dither` | none / ordered / blueNoise / floydSteinberg / temporal *temporal* | `temporal`: zeitlich stabiles Dithering (§5.6) |
| `ditherAmount` | 0…100 *70* | |
| `lossy` | 0…200 *40* (balanced), 80 (smallest), 0 (best) | verlustbehaftetes LZW (§5.6, eigene Umsetzung) |
| `diffTransparency` | an | unveränderte Pixel → transparent (bessere LZW-Läufe) |
| `diffThreshold` | 0…32 *3* (Abstand in Oklab × 100) | „unverändert“ bei kleinem Unterschied |
| `cropChanged` | an | nur das Rechteck mit Änderungen je Frame speichern |
| `mergeSimilar` | an | fast gleiche Frames zusammenlegen (Dauer addiert) |
| `disposal` | auto | auto wählt je Frame „nicht entsorgen“ oder „Hintergrund“ nach kleinerer Größe |

### 3.11 Dokument-Schritte (FW-PNG/PSD/SVG als Dokument, Gruppe „Dokument“)

| ID | Name | Parameter | V | Verfahren | P |
|---|---|---|---|---|---|
| `document.findReplace` | Suchen und ersetzen | `whatToFind` text/font/color/url/nonWeb216 (wie Original `re\21-batch.md` §4.5/§6), `find`, `replace`, `matchCase`, `wholeWord`, `regex` | ja | vorhandenes Suchen-Bedienfeld/Befehle in fp-engine | P2 |
| `document.pages` | Seiten/Status wählen | welche Seite/welcher Status exportiert wird (alle → je eine Datei) | ja | `fp-engine` | P2 |
| `document.layers` | Ebenen ein-/ausblenden | Ebenen-Name (Regex) sichtbar/unsichtbar vor dem Export | ja | Register `layer.setVisible` | P2 |
| `document.slices` | Slices exportieren | wie Export „HTML und Bilder“ ohne HTML | – | `fp-export::slices` | P3 |

### 3.12 Optimieren und Komprimieren (Gruppe „Optimieren“)

| ID | Name | Parameter | V | Verfahren | P |
|---|---|---|---|---|---|
| `optimize.targetSize` | Zielgröße | **A** `maxKB` 1…100 000; `strategy`: quality only / quality + resize *quality only*; `minQuality` 10…95 *40*; `minScale` 10…100 % *50* | ja (erreichte Größe) | Bisektion über Qualität (JPEG/WebP/AVIF; GIF: quality, lossy, colors, dann fps/Größe; PNG: Farben 256→16, dann Größe); höchstens 8 Kodierungen je Datei; Ergebnis meldet erreichte Größe, ⚠ wenn nicht erreichbar | **P1** |
| `optimize.png` | PNG verlustfrei optimieren | **A** `level` 1…6 *2*; `strip` sicher; `alphaClean` an | ja (Größe) | `oxipng` 9.x (MIT, reines Rust mit `libdeflater`→ Feature aus, nur `zopfli`-frei/`deflate` Rust; Lizenz der Features prüfen) | **P1** |
| `optimize.jpeg` | JPEG verlustfrei optimieren | optimierte Huffman-Tabellen, progressiv, Metadaten nach Schritt „Metadaten“ | ja | eigen (`fp-export::jpeg` hat Huffman-Optimierung) | P2 |
| `optimize.webSafe` | Für das Web | sRGB, 8 Bit, Metadaten ohne GPS, Ausrichtung angewendet | ja | Kombischritt | P2 |
| `optimize.quantize` | Farben reduzieren (PNG8) | 2…256, Dithering wie GIF | ja | `quantette` 0.6 (MIT/Apache, k-means in Oklab, Wu) + eigenes Dithering | **P1** |

### 3.13 Vektor und Automatisierung (Gruppe „Erweitert“)

| ID | Name | Parameter | V | Verfahren | P |
|---|---|---|---|---|---|
| `vector.trace` | Nachzeichnen → SVG/FW-PNG | **A** `preset` logo/blackWhite/illustration/photo; `colors`, `filterSpeckle`, `mode` … wie `modify.traceBitmap`; `refine`; Ausgabe SVG (Pfade) oder FW-PNG (bearbeitbar) | ja (gerendert) | `fp-trace` (vorhanden) | **P1** |
| `script.registry` | Firepixel-Befehl | `command` (Auswahl/Suche im Register), `params` (Formular aus dem Schema); mehrere als ein Schritt | ja | Befehlsregister | **P1** |
| `script.command` | Gespeicherter Befehl | Datei aus dem Befehlsordner (ohne `//--not for batch--` und ohne Batch-Skripte, wie `re\21-batch.md` §4.2) | ja | `fp-script` `commands.run` | **P1** |
| `script.code` | JSF-Code | Code-Feld (Profi); `fw.getDocumentDOM()` ist das aktuelle Element; Variablen `batch.index`, `batch.total`, `batch.source` | ja | `fp-script` | P2 |
| `script.condition` | Wenn … dann | Bedingung (Breite > …, hat Transparenz, ist Animation, Name passt) → nachfolgende Karte(n) nur dann | – | eigen | P3 |
| `script.external` | Externes Programm | Befehlszeile mit `{in}`/`{out}`; nur Profi, ausdrücklich bestätigt, nicht über MCP | – | `std::process` mit `child_env` | P3 |

---

## 4. Umbenennen (Endkarte „Dateiname“)

### 4.1 Anfänger-Modus: Baukasten

Die Karte zeigt oben eine Zeile aus **Bausteinen** (Chips, ziehbar, × zum Entfernen), darunter
„+ Baustein“ und die Optionen. Ganz unten die **Live-Tabelle** (§4.3).

Bausteine (`parts[]`, Reihenfolge = Reihenfolge im Namen):

| Baustein (`kind`) | Chip zeigt | Optionen |
|---|---|---|
| `original` | „Originalname“ | Teil: ganz / erste n Zeichen / letzte n Zeichen / ohne Ziffern am Ende |
| `text` | der Text | frei; Leerzeichen sichtbar als `␣` |
| `counter` | „Nummer 001“ | **Start** 0…999 999 *1*, **Schritt** 1…1000 *1*, **Stellen** 1…9 *3* (mit führenden Nullen), je Ordner neu beginnen aus; Reihenfolge = Sortierung der Liste |
| `date` | „Datum 2026-10-13“ | Quelle: heute / Änderungsdatum / **EXIF-Aufnahmedatum** (Rückfall Änderungsdatum, Hinweis); Format-Vorlagen `YYYY-MM-DD`, `YYYYMMDD`, `YYYY-MM-DD_HH-mm-ss`, `DD.MM.YYYY`, frei (Profi) |
| `size` | „1920x1080“ | Bildgröße nach den Schritten (`{w}x{h}`) oder Quelle; nur Breite/Höhe |
| `folder` | „Ordnername“ | Name des Quellordners |
| `exif` | „Kamera“ | Make, Model, Objektiv, ISO, Blende |
| `format` | „webp“ | Zielformat |

Optionen unter den Bausteinen (in dieser Reihenfolge angewendet, §4.4):
1. **Text ersetzen**: „Ersetze [___] durch [___]“, Kästchen „alle Vorkommen“ (*an*; aus = nur erstes
   wie im Original), „Groß/klein beachten“ (*an*). Mehrere Zeilen möglich (+).
2. **Leerzeichen ersetzen durch** [`_`] (Auswahl `_`, `-`, nichts, frei).
3. **Präfix** [___] / **Suffix** [___] (Suffix vor der Endung).
4. **Schreibweise**: unverändert / klein / GROSS / Erster Buchstabe groß / Titel (Jedes Wort) /
   `kebab-case` / `snake_case` / `camelCase` (Profi).
5. **Umlaute umschreiben** (ä→ae, ß→ss, é→e) aus; **Sonderzeichen entfernen** aus.
6. **Endung**: klein (Vorgabe) / unverändert / groß; `jpeg`→`jpg` an.

Vorgaben der Karte: ein Baustein `original`, sonst alles aus → Name bleibt gleich (nur neue Endung).

### 4.2 Profi-Modus: Regex

- Umschalter in der Karte „Baukasten | Regex“. Felder **Suchen** (Regex) und **Ersetzen** (mit `$1`,
  `${name}`, `$0`, und den Platzhaltern `{n:3}`, `{date:YYYY-MM-DD}`, `{exif-date:…}`, `{w}`, `{h}`,
  `{folder}`, `{ext}`, `{name}`). Kästchen: Groß/klein ignorieren, alle Treffer (*an*), auf ganzen
  Namen inkl. Endung anwenden (*aus*).
- **Klartext-Erklärung** unter dem Feld (aus dem Syntaxbaum von `regex-syntax`, gleiche Crate-Familie
  wie `regex`, MIT/Apache): z. B. `^IMG_(\d+)` → „am Anfang „IMG_“, dann eine oder mehrere Ziffern
  (Gruppe 1)“. Bausteine: Literal, Zeichenklassen (`\d` Ziffer, `\w` Buchstabe/Ziffer/_, `\s`
  Leerraum, `[a-z]`), Wiederholungen (`+`, `*`, `?`, `{2,4}`), Gruppen (benannt), Alternativen,
  Anker. Unbekanntes → „(Ausdruck)“. Texte in `batch_rename.ftl`.
- **Spickzettel** ⓘ mit 8 Beispielen zum Einfügen (Nummer am Ende entfernen, Datum umstellen
  `(\d{4})-(\d\d)-(\d\d)` → `$3.$2.$1`, Leerzeichen zusammenfassen, Präfix tauschen …).
- Ungültiger Ausdruck: Feld rot, Meldung der Bibliothek übersetzt-umrahmt („Fehler an Stelle 7: …“),
  Tabelle zeigt alte Namen gedämpft; „Verarbeiten“ gesperrt.
- Regex-Engine: `regex` 1.x (MIT/Apache; keine Rückverweise/Lookaround, dafür garantiert linear –
  kein Hängen bei 10 000 Dateien). Lookaround nur, falls Bedarf: `fancy-regex` (MIT) P3.

### 4.3 Live-Tabelle alt → neu

- Unter der Karte (aufgeklappt) bzw. im Vorschau-Reiter „Plan“: Spalten **Alt**, **→**, **Neu**,
  **Hinweis**. Zeichenweise Hervorhebung der Änderung (eingefügt grün getönt, entfernt rot
  durchgestrichen, `ui_kit::tint`). Zeigt die ersten 200 Zeilen + „… und 1 834 weitere“ (alle im
  Trockenlauf).
- Hinweise je Zeile: ⚠ **Konflikt** (gleicher Zielname wie Zeile x bzw. existiert im Ziel), ⚠ **ungültige
  Zeichen ersetzt**, ⚠ **gekürzt**, ⚠ **leerer Name** → `_`, ⚠ **führender Punkt** → `_` (wie
  Original), ⓘ **EXIF-Datum fehlt** (Rückfall verwendet).
- Konfliktauflösung nach §2.7 (Vorgabe `name (2)`); im Baukasten zusätzlich Knopf „Nummer
  anhängen“, der einen `counter`-Baustein einfügt.

### 4.4 Reihenfolge der Namensbildung (verbindlich, für Tests)

1. Grundname ohne Endung (`.fw.png` zählt als eine Endung).
2. Bausteine zusammensetzen (Anfänger) bzw. Regex anwenden (Profi; auf Grundname, außer
   „ganzer Name“).
3. Ersetzungen (Anfänger, Zeilen der Reihe nach) → Leerzeichen → Präfix/Suffix → Umlaute/
   Sonderzeichen → Schreibweise.
4. Ungültige Zeichen → `_`, reservierte Namen → `_` anhängen, Länge kürzen.
5. Leerer Name → `_`; führender Punkt → `_`.
6. Endung des **Zielformats** anhängen (bei `file.copyOnly`: Originalendung).
7. Konfliktauflösung (§2.7), deterministisch in Listenreihenfolge.

---

## 5. Technik

### 5.1 Architektur

Neues Crate **`crates\fp-batch`** (Bibliothek, kein UI):

```
fp-batch/src/
  lib.rs          öffentliche API: Recipe, Job, run(), preview(), plan(), StepRegistry
  recipe.rs       serde-Modell, Migration, Validierung
  item.rs         Item = Still(RgbaImage+Meta) | Animation(FrameStream) | Document(Session-Dokument) | FileOnly(PathBuf)
  step.rs         trait Step { id, group, params_schema(), validate(), apply(&mut Item, &Ctx) -> Result<()>, preview_ok(), kind: PerItem|Collect }
  pipeline.rs     Schritte je Element, Proxy-Vorschau, Generationen
  executor.rs     Thread-Pool (rayon), Speicherbudget, Abbrechen, Fortschrittskanal
  naming.rs       §4 (rename), Konflikte, Plan/Trockenlauf
  output.rs       Kodieren, Temporärdatei + atomares Umbenennen, Sicherung
  report.rs       batch-report.json/html, batch-log.txt
  steps/          files.rs, geometry.rs, adjust.rs, background.rs, overlay.rs, metadata.rs,
                  anim.rs, gifopt/ (palette.rs, dither.rs, lzw_lossy.rs, frames.rs, size.rs),
                  optimize.rs, vector.rs, script.rs
  video/          mod.rs (trait VideoSource), ffmpeg.rs, mp4.rs (+ h264.rs, av1.rs), webm.rs
```

Abhängigkeiten: `fp-engine` (Session, Befehlsregister), `fp-script` (Dateien öffnen, JSF), `fp-io`,
`fp-export`, `fp-raster`, `fp-render`, `fp-text`, `fp-effects`, `fp-ai`, `fp-trace`, `fp-i18n`.
`fp-ui` hängt von `fp-batch` ab (neues Modul `crates\fp-ui\src\batch\`), ebenso `firepixel-cli`
und `fp-mcp`.

**Anbindung ans Befehlsregister** (kein Zyklus): `fp-engine` bekommt eine neue Datei
`src\commands\batch.rs` mit einem Haken `trait BatchHost` (Muster wie `script_host`), den
`fp_batch::install_host(&mut Session)` setzt. Befehle:

| Befehl | Parameter | Ergebnis |
|---|---|---|
| `batch.run` | `recipe` (Objekt oder Pfad), `files[]` oder `folder` + `recursive`, `dryRun`, `threads` | Bericht (wie `batch-report.json`) |
| `batch.plan` | wie `run` | Liste Quelle → Ziel mit Konflikten, ohne Schreiben |
| `batch.preview` | `recipe`, `file`, `maxSize` | PNG (Base64) vorher/nachher + Größen |
| `batch.steps` | `language?` | Katalog aller Schritte mit Schema (für Agenten) |
| `batch.validate` | `recipe` | Fehler/Warnungen je Schritt |
| `file.batchProcess` | (bestehend) | bleibt rückwärtskompatibel; neu optional `recipe` → leitet an `batch.run` weiter |

Lange Läufe über MCP: `batch.run` mit `async: true` liefert `{jobId}`; `batch.status {jobId}`,
`batch.cancel {jobId}` (P2). Ohne `async` blockiert der Aufruf bis zum Ende.

### 5.2 Ablauf eines Auftrags

1. **Sammeln**: Pfade → `Item`-Beschreibungen (nur Kopf lesen: Format, Größe, Frames, Dauer).
2. **Planen** (`naming.rs`): Zielnamen für alle Elemente **vor** dem Start, Konflikte auflösen,
   Ausgabeordner anlegen (nicht im Trockenlauf). Ergebnis ist deterministisch, unabhängig von der
   Parallelität.
3. **Ausführen**: rayon-Pool mit `threads` = Kerne − 1 (mindestens 1, einstellbar). Jedes Element:
   dekodieren → Schritte der Reihe nach → kodieren → Temporärdatei `.<name>.fptmp` → `rename`.
   Sammel-Schritte (`anim.fromFrames`, `overlay.contactSheet`, PDF „alle in eins“) laufen nach den
   Einzelschritten in einer zweiten Phase.
4. **Speicherbudget**: geschätzter Bedarf je Element = B × H × 4 × (2 + Schritte mit Kopie) bzw.
   für Animationen Frames im Fenster; ein Byte-Semaphor (Vorgabe min(40 % freier RAM, 6 GB)) lässt nur
   so viele Elemente gleichzeitig zu. Elemente größer als das Budget laufen allein. Videos werden in
   Blöcken zu 32 Frames gestreamt; GIF-Optimierer braucht zwei Durchgänge (Palette, dann Kodieren) →
   Frames werden als komprimierte Zwischenbilder (QOI im Speicher bzw. Temp-Datei) gehalten.
5. **Abbrechen**: `CancelToken` (`Arc<AtomicBool>`, Muster `fp-ai::cancel`), geprüft zwischen Schritten,
   Frames und Kachelzeilen; Kodierer werden nicht hart unterbrochen, ihr Ergebnis wird verworfen.
6. **Fortschritt**: `crossbeam-channel` bzw. `std::sync::mpsc` mit Ereignissen `ItemStarted`,
   `ItemProgress(f32)`, `ItemDone{bytes}`, `ItemFailed{msg}`, `Finished`. Die UI fragt je Frame ab
   (kein Blockieren), `ctx.request_repaint_after(100 ms)`.
7. **Dokumente** (FW-PNG/PSD/SVG, Schritte `document.*`, `script.*`, `effect.live`): eine eigene
   headless `Session` je Arbeits-Thread (wie `firepixel-cli batch`), kein Zugriff auf die Sitzung der
   Oberfläche. Offene Dokumente der Oberfläche werden als Speicherstand kopiert.

### 5.3 Sonderfall „nur Dateischritte“

Enthält das Rezept nur `rename`, `file.*` und „Speichern als: wie Quelle“ ohne weitere Schritte, wird
nichts dekodiert: kopieren (Vorgabe) bzw. verschieben (nur bei gleichem Ort, ausdrücklich gewählt).
Ausnahme `{w}`/`{h}`/EXIF-Bausteine: dann wird nur der Kopf gelesen.

### 5.4 Vorschau-Pipeline

`preview(recipe, item, generation)` im eigenen Thread (ein Platz, neue Anfrage verwirft die alte):
Proxy dekodieren (JPEG mit DCT-Verkleinerung wo möglich), Schritte mit `preview_ok()` anwenden,
Ergebnis als Textur; parallel volle Kodierung für die Größenangabe (abbrechbar). Zwischenspeicher je
(Datei, Änderungszeit, Schritt-Präfix-Hash) → nur ab dem geänderten Schritt neu rechnen.

### 5.5 Video-Dekodierung (Empfehlung)

Anforderungen: einzelne exe, proprietäre Lizenz (kein GPL/LGPL-Linken), kein `unsafe` im eigenen Code,
Patente beachten.

| Weg | Formate | Lizenz | Bewertung |
|---|---|---|---|
| **A. ffmpeg des Nutzers** (Prozess, nicht gelinkt, nicht mitgeliefert) | alles (H.264, HEVC, VP8/9, AV1, ProRes, MOV, MKV …) | ffmpeg selbst LGPL/GPL – unkritisch, weil der Nutzer es installiert und Firepixel es nur als externes Programm aufruft | **P1**: Suche in `PATH`, Einstellung „ffmpeg-Datei…“ (Voreinstellungen ▸ Batch), Umgebungsvariable `FIREPIXEL_FFMPEG`. Aufruf `ffmpeg -ss … -t … -i in -vf fps=…,scale=… -f rawvideo -pix_fmt rgba -` (Pipe, gestreamt), Metadaten über `ffprobe -of json`. Keine Shell, Argumente als Liste, `child_env`. |
| **B. eingebaut: MP4/MOV-Demux + H.264** | H.264 (Baseline, Main inkl. CABAC, große Teile High) in MP4/MOV | `re_mp4` (MIT, Demux, Fork von `mp4`) + `rusty_h264-decoder` (BSD-2, `forbid(unsafe_code)` im Kern, 0.16 vom 07.09.2026; Feature `asm` aus → kein `unsafe`) | **P2**. Deckt Handy-/Bildschirmvideos ab. Reife jung (0.x) → hinter Feature `video-h264`, Fuzz-Korpus, Rückfall auf A. |
| **C. eingebaut: AV1** | AV1 in MP4/WebM | `rav1d` (BSD-2, bereits über `sqzer-rav1d` für AVIF im Baum) + Demux (`re_mp4`; WebM: `matroska-demuxer`, MIT/Apache, Lizenz beim Einbau prüfen) | **P2**, fast kostenlos, weil der Decoder schon da ist. |
| D. Betriebssystem (Media Foundation / AVFoundation) | was installiert ist | – | P3, braucht FFI/`unsafe` in Fremd-Crates und je OS Code; nur falls A/B nicht reichen. |
| E. HEVC, VP9 eingebaut | – | keine reife permissive Rust-Lösung; HEVC mit mehreren Patentpools ohne Freigrenze | **nicht** einbauen → nur über A. |

**Patente**: H.264-Dekodierung ist patentpflichtig; der AVC-Pool hat für Endprodukte nach meinem Stand
eine lizenzgebührenfreie Stückzahl-Grenze (≈ 100 000 Einheiten/Jahr) und viele Patente sind bis
≈ 2027–2030 ausgelaufen – das ist **keine Rechtsberatung**; vor einem Verkauf prüfen lassen. AV1 ist
lizenzgebührenfrei (AOMedia-Patentzusage). HEVC: nicht einbauen. ffmpeg des Nutzers verlagert die
Frage zum Nutzer.

**Empfehlung**: **P1 = A** (ffmpeg optional, erkannt oder wählbar; ohne ffmpeg zeigt die Videozeile
„Video-Decoder fehlt“ mit Erklärung und Knopf „ffmpeg-Datei wählen…“; nichts wird heruntergeladen).
**P2 = C + B** hinter Cargo-Features `video-av1` (Vorgabe an) und `video-h264` (Vorgabe nach
Nutzerentscheidung §7), Reihenfolge `auto`: eingebaut, wenn der Codec unterstützt wird, sonst ffmpeg.
Animierte Eingaben (GIF/APNG/WebP/AVIF/JXL) brauchen keinen Video-Decoder (`fp-io` dekodiert sie
bereits).

### 5.6 Kleine GIFs hoher Qualität (eigene Umsetzung, keine GPL/AGPL-Bausteine)

Nicht benutzen: gifski (AGPL), gifsicle/giflossy (GPL), libimagequant/pngquant (GPL bzw. kommerziell),
zenquant (AGPL). Die Verfahren sind aus Veröffentlichungen bekannt und werden eigenständig umgesetzt
(`fp-batch\src\steps\gifopt\`):

1. **Vorverarbeitung**: Frames auf Zielgröße (Lanczos), Bildrate ausdünnen, **fast gleiche Frames
   zusammenlegen** (Dauer addieren), **zeitliche Glättung**: Pixel, die sich gegenüber dem Vorframe
   nur minimal ändern (Oklab-Abstand < `diffThreshold`), auf den Vorwert setzen → stabiler
   Hintergrund, große transparente Flächen.
2. **Palette**: Farbstatistik gewichtet (Kanten und Gesichtstöne nicht unterrepräsentieren; Pixel, die
   im Vorframe gleich waren, zählen weniger). Quantisierung mit `quantette` (k-means in Oklab, MIT/
   Apache, 0.6 vom 15.05.2026) oder dem vorhandenen `fp-export::quant` (Original-nah). `auto`:
   globale Palette (spart 768 Byte je Frame), lokale Palette nur für Frames, deren mittlerer Fehler
   mit globaler Palette > Schwelle ist (Szenenwechsel). Ein Index bleibt für Transparenz frei.
3. **Dithering**: `temporal` = geordnetes Dithering mit **Blue-Noise-Schwellwertmatrix**, die zwischen
   Frames **fest** bleibt (kein Flimmern, gute LZW-Läufe), Stärke adaptiv (in glatten Flächen mehr,
   an Kanten weniger); Floyd-Steinberg mit Serpentine als Option, aber nur in geänderten Bereichen
   (statische Bereiche übernehmen die Indizes des Vorframes → keine „kriechenden“ Muster).
   Blue-Noise-Matrix 64×64 eigen erzeugt (Void-and-Cluster, Ulichney 1993), als Datei im Crate.
4. **Bilddifferenz + Transparenz**: Pixel mit gleichem Palettenindex wie das sichtbare Vorbild →
   Transparenzindex; **Zuschnitt** auf das Rechteck der geänderten Pixel; je Frame Wahl von
   `disposal` (keep/background) nach kleinerer kodierter Größe.
5. **Verlustbehaftetes LZW** (eigene Umsetzung des bekannten Prinzips): beim Verlängern eines
   Wörterbuch-Eintrags wird ein Pixel akzeptiert, wenn sein Farbabstand zum „erwarteten“ Folgeindex
   unter einer Schwelle liegt (aus `lossy` 0…200 abgeleitet) – längere Codes, weniger Bits; Fehler
   wird per Diffusion auf die Nachbarn verteilt. Das vorhandene „Verlust“ des Original-Exports
   (`fp-export::animation::prepare_frames`, Parameter `lossy`) bleibt für den Export-Dialog
   unverändert; der Optimierer hat seinen eigenen Kodierer.
6. **Zielgröße**: Bisektion über `quality` (steuert colors/lossy/ditherAmount gemeinsam), dann fps,
   dann Größe in 10-%-Schritten; höchstens 8 Durchläufe; Messung auf einem Ausschnitt der Frames
   (jedes 4.) zur Schätzung, letzter Durchlauf voll.
7. **Messung**: Testkorpus `tools\golden\batch\gif\` (5 kurze Clips: Bildschirmaufnahme, Foto-
   Kamerafahrt, Zeichentrick, Verlauf, Pixelart), Kennzahlen Größe und SSIM/Butteraugli-ähnlich
   (eigene SSIM auf Oklab-L). Ziel: „balanced“ ≥ 30 % kleiner als der heutige GIF-Export bei
   gleicher oder besserer SSIM; Werte in `docs\berichte\batch-gifopt.md`.

### 5.7 Oberfläche (Dateien in `crates\fp-ui\src\batch\`)

`mod.rs` (Studio-Zustand, Einhängen in `app_state`/`workspace` über eine Zeile), `banner.rs`,
`files.rs` (Liste, D&D, Filter, Miniaturen über `welcome\thumbs.rs`-Muster), `recipe.rs` (Karten,
Galerie, Ziehen), `params.rs` (Formular aus Schema: Zahl/Schieber/Auswahl/Farbe/Datei/Zeit/Bausteine),
`rename_ui.rs`, `preview.rs` (Vergleich, Zoom, Zeitleiste), `output_bar.rs`, `report_ui.rs`.
Gemeinsame Bausteine nur aus `ui_kit`/`widgets`/`welcome` (`paint_banner`, Karten, `empty_state`,
`progress`, `notice`, `table`). Neue allgemeine Bausteine (Segment-Knopf, Chip, Zeitleiste) in
`batch\kit.rs`, später nach `ui_kit` (DES-3).

### 5.8 CLI und MCP

```
firepixel-cli batch --recipe rezept.fpbatch.json (--in <ordner> [--recursive] | <dateien…>)
                    [--out <ordner>] [--conflict rename|skip|overwrite] [--dry-run] [--threads N]
                    [--report bericht.json] [--json]
firepixel-cli batch --steps [--json]            # Katalog
firepixel-cli batch --actions … (bestehend, bleibt)
```

Exit-Code 0 = alles fertig, 1 = mindestens ein Fehler, 2 = Aufruf falsch, 3 = abgebrochen. Fortschritt
auf stderr (eine Zeile je Datei, `--json` als NDJSON). MCP: Werkzeug `batch` (Kurzform für
`run_command batch.run`) mit `recipe`, `files`, `folder`, `dryRun`; Ressource
`firepixel://docs/batch.md` (Katalog). `docs\AGENTS.md` bekommt einen Abschnitt „Batch“.

### 5.9 Bibliotheken (neu)

| Crate | Version (beim Einbau prüfen, `=` pinnen) | Lizenz | Zweck | P |
|---|---|---|---|---|
| `regex` / `regex-syntax` | 1.11+ / 0.8+ | MIT/Apache-2.0 | Umbenennen, Filter, Klartext-Erklärung | P1 |
| `walkdir` | 2.5 | MIT/Unlicense | Ordner rekursiv | P1 |
| `globset` | 0.4 | MIT/Unlicense | Glob-Filter | P2 |
| `quantette` | 0.6.0 | MIT/Apache-2.0 | Palette (k-means Oklab, Wu) | P1 |
| `oxipng` | 9.x, `default-features = false` (ohne `libdeflater`/C) | MIT | PNG verlustfrei | P1 |
| `kamadak-exif` | 0.6 | BSD-2-Clause | EXIF lesen | P1 |
| `img-parts` | 0.3/0.4 | MIT/Apache-2.0 | Metadaten-Segmente JPEG/PNG/WebP lesen/ersetzen/entfernen | P1 |
| `xmp-writer` | 0.3 | MIT/Apache-2.0 | XMP-Paket schreiben | P2 |
| `little_exif` (optional) | 0.x | MIT/Apache-2.0 | EXIF schreiben (Alternative) | P2 |
| `filetime` | 0.2 | MIT/Apache-2.0 | Zeitstempel | P2 |
| `re_mp4` | 0.3+ | MIT | MP4/MOV-Demux | P2 |
| `rusty_h264-decoder` | 0.16 (ohne `asm`) | BSD-2-Clause | H.264 | P2 (Feature) |
| `matroska-demuxer` | 0.x | MIT/Apache (prüfen) | WebM/MKV-Demux | P2 |
| vorhanden | `rayon`, `serde`, `serde_json`, `sha2`, `png`, `webp`, `ravif`, `sqzer-rav1d`, `zune-jpeg`, `image-webp`, `jxl-oxide`, `heic-decoder`, `visioncortex` | permissiv | | |

Nicht erlaubt: gifski, gifsicle, libimagequant/imagequant, zenquant, ffmpeg-Bindungen (`ffmpeg-next`
linkt LGPL/GPL), `libheif`, `exiv2`/`rexiv2` (GPL), `qcms` nur nach Zustimmung (MPL-2.0). Jede neue
Abhängigkeit kommt in `docs\berichte\<paket>.md` mit Lizenz; Prüfung mit `cargo tree` auf
GPL/AGPL/LGPL-Lizenzfelder (Test in `fp-batch\tests\licenses.rs` liest `cargo metadata`, P2).

### 5.10 Tests

- **Einheitentests** je Schritt (`fp-batch\tests\steps_<gruppe>.rs`): Ergebnis-Pixel/Größe/Metadaten
  geprüft, nicht nur „kein Fehler“.
- **Golden-Dateien** `tools\golden\batch\<fall>\{in,expected}\` + `recipe.json`; Vergleich mit Toleranz
  (max. Kanalabweichung 2, für verlustbehaftete Formate SSIM ≥ 0,98 und Größe ± 5 %); `out\` per
  `.gitignore` ausgeschlossen. Fälle: Web-Verkleinern, JPEG aus transparentem PNG (Weiß), Umbenennen
  nach EXIF-Datum, Video → GIF (über ffmpeg nur wenn vorhanden, sonst `#[ignore]` mit Grund),
  GIF trimmen/beschleunigen, Hintergrund entfernen (`#[ignore]`, braucht Modell), Metadaten ohne GPS,
  Nachzeichnen → SVG, Zielgröße 100 KB.
- **Logikfälle nach §4b**:
  1. *Wiederholung*: Parameter 50× ändern (Schieber, jede Taste) → Rezept enthält den Schritt einmal,
     Vorschau zeigt den Endwert, Rezept-Verlauf hat einen Eintrag je Zug; Schritt 50× hinzufügen und
     entfernen → keine Reste; Lauf zweimal hintereinander → zweiter Lauf erzeugt `name (2)` bzw.
     überspringt nach Regel, nie doppelte Effekte im Bild.
  2. *Vorschau/Abbrechen*: Vorschau ersetzt, statt zu addieren (Effekt-Schritt + Parameteränderung →
     ein Effekt); Abbrechen im Lauf → keine `.fptmp`-Reste, fertige Dateien vollständig, Originale
     unverändert (Prüfsumme); Abbrechen der Galerie lässt das Rezept unverändert.
  3. *Rückgängig/Wiederholen* im Rezept in Folgen (A, B, ↶, C, ↷).
  4. *Auswahlwechsel*: andere Datei markieren, während die Vorschau rechnet → Vorschau zeigt die neue
     Datei, nie das Ergebnis der alten; Karte wechseln während ein Feld fokussiert ist → Wert landet
     im richtigen Schritt.
  5. *Grenzfälle*: leere Liste, 1 Datei, 10 000 Dateien (Plan < 2 s), gleiche Namen aus zwei Ordnern,
     Unicode/Emoji/Umlaute/sehr lange Pfade, schreibgeschützte Quelle, Ziel ohne Schreibrecht,
     kaputte Datei mitten in der Liste (Lauf geht weiter), 0-Byte-Datei, 30 000 × 30 000 px
     (Speicherbudget, läuft allein), Animation mit 1 Frame, Video ohne Decoder, Animation mit
     Frames unterschiedlicher Größe, CMYK-JPEG, 16-Bit-PNG, Graustufen mit Alpha.
- **UI** (egui_kittest, `crates\fp-ui\tests\batch_*.rs`): Aufbau, D&D (`drop_files`), Galerie,
  Karten ziehen, Vorschau-Teiler, Fortschritt; `clipping_audit` 0 Funde (3 Designs × de/en ×
  100/150 % × 1024×768/1920×1080, dazu `qps`); Bilder `docs\screenshots\batch\` je Design/Sprache.
- **CLI/MCP**: `apps\firepixel-cli\tests\batch_recipe.rs`, `fp-mcp` Werkzeug-Test.
- **Neutralität**: `cargo test -p fp-neutral` grün; `cargo test -p fp-i18n` (Vollständigkeit de/en).
- Alles mit `FP_NO_SHELL_OPEN=1`, Testprofil (§4a), nie ins echte Profil.

---

## 6. Bauplan für die Nacht

### 6.1 Pakete

Je Paket ein frischer Agent (Opus), warmer Slot `fp-s<N>` (§7b), ≈ 45–60 min. Dateigrenzen strikt;
Einhängezeilen in fremden Dateien nur wie §1 (eine Zeile + Befund im Bericht).

| Paket | Inhalt | schreibt (exklusiv) | braucht | Dauer |
|---|---|---|---|---|
| **BATCH-1 Kern** | Crate `fp-batch`: `recipe.rs` (Modell, serde, Migration), `item.rs`, `step.rs` (Trait + Registry), `pipeline.rs`, `executor.rs` (rayon, Budget, Abbrechen, Kanal), `naming.rs` (§4.4 vollständig inkl. Baukasten/Regex/Konflikte – Logik, keine UI), `output.rs` (Formate über `fp-export`, Temp + atomar, Sicherung), `report.rs`; Schritte `rename`, `file.copyOnly`, `output`; `fp-engine\src\commands\batch.rs` + Registrierungszeile; CLI `--recipe/--dry-run/--steps` | `crates\fp-batch\**` (ohne `steps\` außer `steps\mod.rs`, `steps\files.rs`), `crates\fp-engine\src\commands\batch.rs`, `apps\firepixel-cli\src\batch.rs` (+ 1 Zeile in `main.rs`) | – | 60 |
| **BATCH-2 UI-Gerüst** | Studio mit 3 Karten, Banner, Modus-Umschalter, Dateiliste (D&D, Ordner rekursiv, Filter, Sortierung, Miniaturen), Rezept-Karten (generisches Formular aus `params_schema`), Galerie, Endkarten, Vorschau (Vorher/Nachher/Vergleich, Zoom), Ausgabe-Leiste, Fortschritt, Ende-Karte; Aufruf über `file.batchProcess` | `crates\fp-ui\src\batch\**` (ohne `rename_ui.rs`, `timeline.rs`), `locales\{de,en}\batch.ftl`, `crates\fp-ui\tests\batch_ui.rs`; Einhängezeilen in `lib.rs`/`menu_dialogs.rs`/`workspace.rs` | BATCH-1 (API-Stand nach 20 min: `recipe.rs`/`step.rs` zuerst committen) | 60 |
| **BATCH-3 Umbenennen-UI** | Baukasten-Chips, Regex-Feld, Klartext-Erklärung (`regex-syntax`), Spickzettel, Live-Tabelle mit Hervorhebung, Konflikthinweise; Vorlagen „nach Datum“ | `crates\fp-ui\src\batch\rename_ui.rs`, `crates\fp-batch\src\explain.rs`, `locales\{de,en}\batch_rename.ftl`, Tests | 1, 2 | 50 |
| **BATCH-4 Geometrie + Farbe + Effekte** | `resize`, `crop` (ohne smart), `trim`, `rotate` (+EXIF), `canvas`, `adjust.*` (P1-Teil), `filter.sharpen/blur/registry`, `effect.live`, `script.registry`, `script.command` | `crates\fp-batch\src\steps\{geometry,adjust,script}.rs`, `locales\…\batch_steps.ftl` (Abschnitte Geometrie/Farbe/Erweitert), Tests | 1 | 60 |
| **BATCH-5 Metadaten + Optimieren** | `metadata.edit` (keep/none/gps/kamera/xmp/icc), `metadata.orientation`, Übernahme beim Konvertieren, `optimize.png` (oxipng), `optimize.quantize` (quantette), `optimize.targetSize` (Standbild) | `steps\{metadata,optimize}.rs`, `crates\fp-batch\src\meta\**`, `locales\…\batch_meta.ftl`, Tests | 1 | 60 |
| **BATCH-6 Hintergrund + Überlagerung** | `background.remove` (BiRefNet über `fp-ai`), `removeColor`, `flatten`; `overlay.text`, `overlay.image`, `overlay.border` | `steps\{background,overlay}.rs`, `locales\…\batch_bg.ftl`, Tests (KI `#[ignore]`) | 1 | 55 |
| **BATCH-7 Animation + Video (ffmpeg)** | `Item::Animation` gestreamt, `anim.trim/speed/loop/dropFrames/toFrames/fromFrames`, Ausgabe APNG/WebP animiert; `video\ffmpeg.rs` (Suche, Einstellung, Pipe); Zeitleiste in der Vorschau | `steps\anim.rs`, `crates\fp-batch\src\video\{mod,ffmpeg}.rs`, `crates\fp-ui\src\batch\timeline.rs`, `locales\…\batch_anim.ftl`, Tests | 1, 2 (Zeitleiste) | 60 |
| **BATCH-8 GIF-Optimierer** | §5.6 vollständig: Palette global/lokal, temporales Blue-Noise-Dithering, Differenz-Transparenz, Zuschnitt, Disposal-Wahl, verlustbehaftetes LZW, Zielgröße; Korpus und Messbericht | `crates\fp-batch\src\steps\gifopt\**`, `tools\golden\batch\gif\**`, `docs\berichte\batch-gifopt.md` | 1 (nur `Item`-Typen) | 60 |
| **BATCH-9 Vektor, Vorlagen, MCP** | `vector.trace`, eingebaute Vorlagen (§2.5) als JSON in `crates\fp-batch\data\recipes\`, Vorlagen-Galerie-Daten, MCP-Werkzeug `batch` + Ressource `batch.md`, `docs\AGENTS.md`-Abschnitt, Bericht HTML | `steps\vector.rs`, `crates\fp-batch\data\**`, `crates\fp-mcp\src\batch.rs` (+1 Zeile), `docs\AGENTS.md`, `docs\AGENTS.en.md` | 1, 4–8 (Schritt-IDs) | 45 |
| **BATCH-10 Video eingebaut** (P2) | `video\mp4.rs` (`re_mp4`), `video\av1.rs` (`rav1d`), `video\h264.rs` (Feature), WebM-Demux, Fuzz-/Korpustests | `crates\fp-batch\src\video\{mp4,av1,h264,webm}.rs` | 7 | 60 |
| **BATCH-11 Feinschliff** | Designs/Sprachen/Größen, `clipping_audit`, Bilder `docs\screenshots\batch\`, schmale Layouts, Tastatur, Barrierefreiheitsnamen, Startbildschirm-Eintrag | `crates\fp-ui\src\batch\**` (nach 2/3/7), `crates\fp-ui\tests\batch_*.rs` | 2, 3, 7 | 50 |

Gemeinsame Datei `crates\fp-batch\src\steps\mod.rs` (Registrierung): jede Gruppe hängt **eine** Zeile
an (`geometry::register(r);`); kleine Edits, vorher neu lesen (§7). `Cargo.toml` von `fp-batch`:
nur BATCH-1 legt an; spätere Pakete fügen ihre Abhängigkeit als einzelne Zeile hinzu (Befund im
Bericht).

### 6.2 Reihenfolge und Dauer

```
Welle A  (0–60 min)   BATCH-1 Kern  ──┬── nach 20 min API-Commit ──► BATCH-2 UI-Gerüst (20–80)
Welle B  (60–120)     BATCH-4 · BATCH-5 · BATCH-6 · BATCH-7 · BATCH-8   (parallel, 5 Agenten)
                      BATCH-3 (80–130, nach 2)
Welle C  (120–180)    BATCH-9 · BATCH-11 · BATCH-10 (P2, optional)
Danach                BATCH-T: Nachprüfung (Brücke, Bilder, Golden-Läufe), Fehler beheben
```

- **Parallel (bis 8 Agenten)**: ≈ **3 h** Wandzeit für P1 (Welle A–C ohne BATCH-10), ≈ 3,5 h mit
  BATCH-10. Höchstens 7 gleichzeitig (Welle B + BATCH-3), innerhalb der Grenze von 8.
- **Nacheinander mit einem Agenten** (wie PLAN §4e „je Paket ein frischer Agent nacheinander“):
  1 → 2 → 4 → 5 → 6 → 7 → 8 → 3 → 9 → 11 ≈ **9,5 h**, mit 10 ≈ 10,5 h. Empfehlung: mindestens
  Welle B parallel (3–5 Agenten), sonst reicht eine Nacht nicht.
- **P1-Umfang der ersten Nacht** = Pakete 1–9 und 11 mit den in §3 als **P1** markierten Schritten.
  P2 (Einzel-EXIF-Tags, `metadata.set`, Sprite-Sheet, Kontaktbogen, PDF, JXL, Video eingebaut,
  smart crop, Motiv zentrieren …) in einer zweiten Runde (BATCH-12 ff., gleiche Dateigrenzen).
- Jedes Paket: eigene Tests, `cargo check --workspace --all-targets`, `cargo test -p fp-i18n`,
  `cargo test -p fp-neutral`, Bericht `docs\berichte\batch-<n>.md`, Zeilen in `NACHPRUEFUNG.md`
  über den Doku-Agenten (§7b.2).

### 6.3 Messungen im Original (M1–M13 aus `re\21-batch.md` §14)

Die moderne Fassung baut den Assistenten **nicht** Steuerelement für Steuerelement nach; Lage, Größen,
Grau-Regeln und Startzustände der Original-Fenster sind nicht mehr Vorgabe. Daher:

| Nr. | Thema | Für das Studio |
|---|---|---|
| M1 | Dateiauswahl-Liste, Doppelte, Aktiv-Regeln | **entfällt** (eigene Regeln §2.3) |
| M2 | Startinhalt, `BatchWhichFiles` | **entfällt** |
| M3 | Mehrfachaufnahme, Doppelklick, ▲/▼ | **entfällt** (Schritte beliebig oft, §2.4) |
| M4 | Welche Befehle in der Liste, `//--not for batch--` | **entfällt**: Markierung ist aus der Datei bekannt; das Studio zeigt alle gespeicherten Befehle ohne Markierung |
| M5 | Export-Liste, „Bearbeiten“-Dialog | **entfällt** (eigene Formatoptionen; „Feinabstimmen…“ öffnet die vorhandene Bildvorschau) |
| M6 | Skalieren-Grenzen, Schieber | **entfällt** (eigene Bereiche §3.3) |
| M7 | Ausgabe/Backups Aktiv-Regeln | **entfällt** |
| M8 | Was gemerkt wird, Skript-speichern-Dialog | **entfällt** (Rezepte als JSON) |
| M9 | Fortschrittsfenster, Protokollort/-format | **entfällt** (eigener Bericht) |
| M10 | Endung bei Formatwechsel, Überschreiben | **entfällt** (eigene Regel: Endung folgt Format, nie stilles Überschreiben) |
| M11 | Serialisierung gespeicherter Batch-Skripte | **nur nötig für P3** „alte Batch-Skripte importieren“ |
| M12 | Skript ausführen → „Zu verarbeitende Dateien“ | **nur nötig für P3** (gleicher Import) |
| M13 | Nur Umbenennen: verschieben vs. kopieren | **entfällt** (Vorgabe kopieren, verschieben nur ausdrücklich) |

Für die erste Nacht ist **keine Messung im Original nötig**.

---

## 7. Offene Entscheidungen für den Nutzer

| # | Frage | Empfehlung |
|---|---|---|
| 1 | Ersetzt das Studio den klassischen Assistenten im Menü ganz? | **Ja.** *Datei ▸ Batch-Verarbeitung…* öffnet das Studio; alle Funktionen des Originals sind enthalten (§1.2). Paritätszeile D12 → „ersetzt durch modernen Batch-Modus“. |
| 2 | H.264 eingebaut (P2, `rusty_h264`, Patentfrage) oder nur über ffmpeg? | **P1 nur ffmpeg (optional, vom Nutzer installiert)**; AV1 eingebaut in P2; H.264 eingebaut als Feature, standardmäßig **an** erst nach eigener Prüfung der Patentlage/Stückzahl (keine Rechtsberatung). |
| 3 | Vorgabe für das Ziel | **Neuer Ordner `batch-output` neben den Quellen**, Konflikt „neuen Namen wählen“; Originale nie ohne ausdrückliche Wahl ersetzen. |
| 4 | Bau parallel oder nacheinander? | **Welle B parallel (5 Agenten)**: P1 in ≈ 3 h statt ≈ 9,5 h; Rest wie PLAN §4e. |
| 5 | Wo erscheint das Studio? | **Als Reiter „Batch“ im Hauptfenster** (wie Startbildschirm), kein eigenes Fenster. |
| 6 | MPL-2.0 (`qcms` für ICC-Umrechnung) zulassen? | **Nein vorerst**; eigene Umrechnung für Matrix-Profile reicht für sRGB, Display P3 und andere RGB-Matrix-Profile, CMYK über die vorhandene Tabelle. |
| 7 | KI-Upscaling/Gesichter (P3) | später, nur mit Modellen permissiver Lizenz, Download nur nach Zustimmung im Modell-Dialog. |
