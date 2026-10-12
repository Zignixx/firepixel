# Gestaltung von Fenstern, Dialogen und Modalen (Agent DESIGN, ab 12.10.2026)

Nutzerwunsch: Fenster und Modale, die funktionieren, aber „ohne Liebe“ aussehen, werden schöner.
Dieses Dokument enthält die **Gestaltungsregeln** (verbindlich für neue und überarbeitete Fenster),
die **gemeinsamen Bausteine** (`crates\fp-ui\src\ui_kit.rs`) und die **Bestandsaufnahme mit
Prioritätenliste**.

Bilder: `docs\screenshots\design_vorher_*.png` (Stand vor der Überarbeitung) und
`design_nachher_*.png`; je Bild die drei Designs nebeneinander (**Normal | Dunkel | OLED**).
Erzeugen: `cargo test -j 4 -p fp-ui --test design_bilder -- --ignored --nocapture` mit
`FP_DESIGN_BILD=vorher|nachher` (Startbildschirm: `--test design_willkommen -- --ignored`).

---

## 1. Grundsätze

1. **Nähe zum Original bleibt Pflicht.** Dialoge, die es im Original gibt (Leinwandgröße,
   Bildgröße, Neues Dokument, Voreinstellungen, Export, Tonwertkorrektur, Kurven, Filter- und
   Effektdialoge, Tastenkürzel, Rechtschreibung, Pop-up-Menü-Editor …), behalten **Aufbau,
   Reihenfolge der Felder, Beschriftungen und Bedienung** (Tab-Reihenfolge, Standardknopf,
   Zugriffstasten, Eingabe/Esc). Überarbeitet wird nur die Ausführung: Abstände, Ausrichtung,
   Gruppenrahmen, Kopf, Knopfleiste, Farben, Zustände. Wortlaut und Bedienung bleiben gleich; ihre
   Tests bleiben grün.
2. **Eigene Firepixel-Fenster** (KI-Modelle, KI-Modell fehlt, Sprache, MCP-Steuerkanal,
   Startbildschirm, Meldungen) folgen diesen Regeln frei.
3. **Keine festen Texte.** Alle Bausteine nehmen Texte als Parameter; die Aufrufer holen sie über
   `fp_i18n::t`. Neue Texte in `de` und `en` gleichzeitig (Regeln §1b).
4. **Farben ausschließlich aus dem Theme** (`theme.rs`, `Theme::current(ctx)`), keine Farbwerte im
   Dialogcode. Abgeleitete Töne nur über `theme::mix`/`with_alpha` bzw. `ui_kit::tint`/`card_bg`.
5. **Clean Room**: eigene Symbole (`icons.rs`, `ui_kit::KitIcon`), eigenes Logo
   (`assets\icon`), keine Grafiken oder Markenzeichen des Originals; Neutralitätsregel §1a.

## 2. Raster, Abstände, Ränder

Alles liegt auf einem **8er-Raster** (`ui_kit::sp`):

| Konstante | Wert | Verwendung |
|---|---|---|
| `sp::XS` | 4 | Symbol ↔ Text in dichten Zeilen, Titel ↔ Erklärsatz |
| `sp::S` | 8 | Abstand innerhalb einer Gruppe, zwischen Knöpfen, unter Abschnittsüberschriften |
| `sp::M` | 12 | Beschriftung ↔ Feld, Innenabstand von Hinweisblöcken und Kopfbereichen |
| `sp::L` | 16 | **Dialogrand**, Abstand zwischen Abschnitten |
| `sp::XL` | 24 | großzügige Flächen (Startbildschirm, Spaltenabstand) |
| `sp::ROW` | 24 | Zeilenhöhe Beschriftung-Wert und Listen |
| `sp::BUTTON_H` | 23 | Knopfhöhe (wie bisher) |

- Dialogrand ringsum 16 px; nichts klebt am Rand oder an der Titelleiste.
- Elemente einer Zeile sind **senkrecht mittig** ausgerichtet, Beschriftungen einer Spalte
  **linksbündig auf einer Kante**.
- Gruppenrahmen (`dialogs::group_box`) nur, wo das Original sie hat; sonst Abschnitte (§4).
- Dialoge, deren Inhalt wachsen kann (Listen, Tabellen, lange Texte), sind
  **größenveränderlich** (`dialog_frame_resizable`) oder scrollen; feste Dialoge berechnen ihre
  Größe aus den Textbreiten (`dialogs::extra_w`, `buttons_w`), nie umgekehrt.

## 3. Kopfbereich je Dialogart

| Art | Kopf | Beispiele |
|---|---|---|
| **Einstellungsdialog des Originals** | kein zusätzlicher Kopf (wie im Original); nur Titelleiste | Leinwandgröße, Bildgröße, Filter, Effekte, Tonwert, Kurven |
| **Eigenes Verwaltungsfenster** | `ui_kit::dialog_header`: Symbol 32 px, Titel 15 px fett, Erklärsatz (umbrochen, gedämpft), Kartenfläche, geätzte Linie | KI-Modelle verwalten, Sprache, MCP-Steuerkanal, Gespeicherte Befehle |
| **Rückfrage / Meldung** | Symbol 32 px links (Info/Warnung/Fehler, `KitIcon`), Text rechts daneben, kein Titel im Inhalt (steht in der Titelleiste) | Ungespeichert, Bestätigen, Meldung |
| **Info-Fenster** | App-Logo (`HeaderIcon::App`), Name, Version | Über Firepixel |

## 4. Abschnitte und Trennlinien

- `ui_kit::section(ui, titel)`: Überschrift **12 px fett in `section_label`**, danach eine geätzte
  Linie bis zum rechten Rand; darunter 4 px, dann der Inhalt. Abstand zum nächsten Abschnitt 16 px.
- `ui_kit::separator` / `hline` / `vline`: geätzte Linie (dunkel + Lichtkante) im Normal- und
  Dunkel-Design, eine feine Linie im OLED-Design. Keine dreifachen Splitter-Linien in Dialogen
  (die gehören zu Docks).

## 5. Beschriftung-Wert-Raster

- Beschriftungsspalte = breiteste Beschriftung + 12 px (`ui_kit::label_width`), für **alle Zeilen
  eines Abschnitts gleich**; Felder beginnen auf derselben Kante.
- Zeilenhöhe 24 px, Feldhöhe 20–22 px, senkrecht mittig (`ui_kit::kv_row`).
- Einheiten (px, %, °) direkt rechts vom Feld mit 4 px Abstand, gedämpft.
- Zu lange Beschriftungen (andere Sprachen) werden mit „…“ gekürzt **und** als Überlauf gemeldet
  (`dialogs::report_overflow`) – das Layout soll sie aber durch `label_width` gar nicht erst kürzen.

## 6. Knopfleiste

- `ui_kit::button_bar`: am unteren Rand, geätzte Linie darüber, Innenabstand 12 px.
- **Rechts** die Hauptknöpfe in Windows-Reihenfolge: **OK (Standardknopf, Rahmen in `focus`)**,
  Abbrechen, ggf. Übernehmen. **Links** Nebenknöpfe (Hilfe, Zurücksetzen, Vorschau-Schalter).
- Alle Knöpfe einer Leiste **gleich breit** (längste Beschriftung + 12 px, mindestens 76 px),
  Höhe 23 px, Abstand 8 px.
- Eingabetaste löst den Standardknopf aus, Esc bricht ab (über `dialogs::push_button`).
- Dialoge des Originals mit Knöpfen rechts oben (z. B. Tonwert, Kurven: OK/Abbrechen/Auto rechts
  neben dem Inhalt) **behalten** diese Lage; vereinheitlicht werden dort nur Breite, Höhe, Abstand.

## 7. Hinweis- und Warnblöcke

`ui_kit::notice(ui, Tone, titel, text)`: getönte Fläche (`ui_kit::tint`), 3 px Leitstreifen links
in der Tonfarbe, Symbol 16 px, optional fetter Titel, umbrochener Text. Töne: `Info` (`focus`),
`Warning` (`icon_yellow`), `Error` (`icon_red`), `Success` (`icon_green`). Fehler und Warnungen in
Dialogen nie nur als roter Text ohne Fläche.

## 8. Tabellen und Listen

`ui_kit::table`: Kopf im Bedienfeld-Kopfstil (`widgets::header_bar`, 22 px, fett 11 px), Spalten
mit feinen Trennern, Zeilen 22–24 px, **Zebrastreifen** (`ui_kit::zebra`), **Hover**
(`list_hover_bg`), **Auswahl** (`list_selected_bg`/`list_selected_text`, OLED mit Akzentrand),
senkrechter Bildlauf, Zellentext mit „…“ und Tooltip (`ui_kit::cell_text`). Zahlen rechtsbündig
(`Column::right`). Rahmen in `list_border`.

## 9. Leere Zustände und Fortschritt

- **Leer**: `ui_kit::empty_state` – gedämpftes Symbol 32 px, Titel (fett, gedämpft), Hinweis was
  zu tun ist (klein). Nie eine leere weiße Fläche ohne Erklärung.
- **Fortschritt**: `ui_kit::progress` – Balken 8 px, abgerundet, Füllung `focus`, Text darüber;
  unbestimmt = wandernder Abschnitt. Abbrechen-Knopf rechts in der Knopfleiste.

## 10. Farben und Typografie

| Rolle | Farbe (Palette) |
|---|---|
| Dialogfläche | `frame_bg` |
| Karte, Kopfbereich, Startbildschirm | `ui_kit::card_bg` (= `menu_bg`) |
| Text / gedämpft / gesperrt | `text` / `text_weak` / `text_disabled` |
| Abschnittsüberschrift | `section_label` |
| Link | `focus` (Normal leicht abgedunkelt) |
| Linien | `line_dark`, `line_mid`, `line_light` (über `ui_kit::hline`) |
| Fokus, Standardknopf, Fortschritt | `focus` |

| Stufe (`ui_kit::type_scale`) | Größe | Gewicht | Verwendung |
|---|---|---|---|
| `CAPTION` | 11 | normal / fett | Tabellenköpfe, Hinweise unter Feldern, Fußzeilen |
| `BODY` | 11,5 | normal | Beschriftungen, Felder, Fließtext (= `fonts.ui`) |
| `SECTION` | 12 | fett | Abschnittsüberschriften |
| `TITLE` | 15 | fett | Titel im Kopfbereich |
| `DISPLAY` | 30 | fett | Programmname im Startbildschirm |

Nur diese Stufen; keine kursiven oder farbigen Fließtexte (außer Links und Hinweisblöcken).

## 11. Verhalten in den drei Designs

- **Normal**: hellgrau wie das Original; Karten fast weiß, geätzte Linien, Verläufe in Köpfen.
- **Dunkel**: gleiche Gliederung in dunklen Grautönen; Karten etwas dunkler als die Fläche.
- **OLED**: reines Schwarz; Linien einfach und fein; Fokus, Auswahl, Hover und Kopfbereiche mit
  Akzentschein (`theme.glow`); Symbole farbig. Kein Grau-auf-Schwarz unter 4,5:1 Kontrast für Text.
- Jedes überarbeitete Fenster wird in allen drei Designs, in Deutsch, Englisch und `qps`, bei
  1024×768 und 1920×1080 sowie 100 % und 150 % geprüft (keine Überläufe, nichts abgeschnitten).

## 12. Startbildschirm (neu, `src\welcome.rs`)

Aufbau wie im Original, eigenes Branding:
- **Kopfbanner**: ruhige, dunkle Fläche aus dem Theme (Normal: dunkles Schieferblau aus `text`,
  `line_dark` und `focus`; Dunkel: aus `app_bar_bg`/`canvas_bg`; OLED: Schwarz), Struktur aus
  einem nach rechts dichter werdenden **Pixelfeld** (OLED in den Gruppenfarben), das eigene
  **Flammen-Pixel-Logo** (`assets\icon`), „Firepixel“ und ein Leitsatz.
- **Links** „Zuletzt geöffnete Dateien“ (bis 8, Symbol je Dateiart: FW-PNG, Bild, Skript;
  Tooltip mit vollem Pfad; lange Namen mit „…“; fehlende Dateien gedämpft; **leerer Zustand**),
  darunter „Öffnen...“.
- **Rechts** „Neu erstellen“ (Firepixel-Dokument, Aus Vorlage...) und „Erweitern“ (Gespeicherte
  Befehle, Erweiterungen verwalten).
- **Unten** »-Links zu eigenen Zielen: Kurzanleitung (`docs\ANLEITUNG.md`), Tastenkürzel,
  KI-Modelle verwalten, Automatisierung und KI-Steuerung; rechts das Logo mit Version.
- **Fußzeile** „Nicht mehr anzeigen“ = Voreinstellung `ShowStartPage` (wieder einschalten:
  Voreinstellungen ▸ Allgemein). In der laufenden Sitzung bleibt der Bildschirm sichtbar.
- Zwei Spalten ab 540 px Kartenbreite, darunter einspaltig; Bildlauf, wenn die Höhe nicht reicht.

---

## 13. Bestandsaufnahme (vorher) und Prioritäten

Siehe Abschnitt unten; wird nach dem Bildlauf ergänzt.
