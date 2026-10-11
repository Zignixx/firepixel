# Logik der Setzbefehle (50× ändern, Rückgängig/Wiederholen) – automatischer Gesamttest

Erzeugt von `crates\fp-ui\tests\gesamt_logik.rs` (`cargo test -p fp-ui --test gesamt_logik`). Nicht von Hand ändern.

**32/32 Einträge ✅**

| Eintrag | Befehl | Ergebnis | Hinweis |
|---|---|---|---|
| Modifizieren ▸ Selektives JPEG ▸ Einstellungen... | `modify.selectiveJpeg.settings` | ✅ Doc | ohne Auswahl; 50 Verlaufsschritte für 50 Änderungen |
| Leinwandursprung | `document.setOrigin` | ✅ Doc | ohne Auswahl; 50 Verlaufsschritte für 50 Änderungen |
| Textur | `element.setTexture` | ✅ Doc | Rechteck; 50 Verlaufsschritte für 50 Änderungen |
| Zoom setzen | `view.setZoom` | ✅ Doc | ohne Auswahl; 0 Verlaufsschritte für 50 Änderungen |
| Ansicht verschieben | `view.pan` | ✅ Doc | ohne Auswahl; 0 Verlaufsschritte für 50 Änderungen |
| Sichtbereich melden | `view.setViewport` | ✅ Doc | ohne Auswahl; 0 Verlaufsschritte für 50 Änderungen |
| Ansicht ▸ Raster ▸ Raster bearbeiten... | `view.grid.editGrid` | ✅ Doc | ohne Auswahl; 50 Verlaufsschritte für 50 Änderungen |
| Hilfslinien bearbeiten | `view.guides.editGuides` | ✅ Doc | ohne Auswahl; 50 Verlaufsschritte für 50 Änderungen |
| Modifizieren ▸ Animation ▸ Einstellungen... | `modify.animation.settings` | ✅ Doc | Animationssymbol; 50 Verlaufsschritte für 50 Änderungen |
| Text ▸ Größe ▸ Andere... | `text.size.other` | ✅ Doc | Text; 50 Verlaufsschritte für 50 Änderungen |
| Texteditor | `text.editor` | ✅ Doc | Text; 50 Verlaufsschritte für 50 Änderungen |
| Texteigenschaften | `text.setProperties` | ✅ Doc | Text; 50 Verlaufsschritte für 50 Änderungen |
| Textattribute | `text.setAttributes` | ✅ Doc | Rechteck; 50 Verlaufsschritte für 50 Änderungen |
| Füllung ändern | `element.setFill` | ✅ Doc | Rechteck; 50 Verlaufsschritte für 50 Änderungen |
| Strich ändern | `element.setStroke` | ✅ Doc | Rechteck; 50 Verlaufsschritte für 50 Änderungen |
| Deckkraft | `element.setOpacity` | ✅ Doc | Rechteck; 50 Verlaufsschritte für 50 Änderungen |
| Position und Größe | `element.setBounds` | ✅ Doc | Rechteck; 50 Verlaufsschritte für 50 Änderungen |
| Farben setzen | `color.set` | ✅ Doc | ohne Auswahl; 0 Verlaufsschritte für 50 Änderungen |
| Füllung ändern | `pi.fill` | ✅ Doc | Rechteck; 50 Verlaufsschritte für 50 Änderungen |
| Strich bearbeiten | `pi.brush` | ✅ Doc | Rechteck; 50 Verlaufsschritte für 50 Änderungen |
| Eckrundung | `pi.roundness` | ✅ Doc | Rechteck; 50 Verlaufsschritte für 50 Änderungen |
| Pfadoperation | `pi.compound.setOp` | ✅ Doc | Rechteck + Ellipse; 50 Verlaufsschritte für 50 Änderungen |
| Glättung | `pi.text.antialias` | ✅ Doc | Rechteck; 50 Verlaufsschritte für 50 Änderungen |
| Textversatz | `pi.text.pathOffset` | ✅ Doc | Rechteck; 50 Verlaufsschritte für 50 Änderungen |
| Web-Objekt | `pi.web` | ✅ Doc | Rechteck; 50 Verlaufsschritte für 50 Änderungen |
| Verhalten löschen | `pi.behavior.remove` | ✅ Doc | Rechteck; 50 Verlaufsschritte für 50 Änderungen |
| Ereignis ändern | `pi.behavior.event` | ✅ Doc | Rechteck; 50 Verlaufsschritte für 50 Änderungen |
| Statuseigenschaften | `pi.state.props` | ✅ Doc | ohne Auswahl; 50 Verlaufsschritte für 50 Änderungen |
| GIF-Animationsschleife | `pi.state.loop` | ✅ Doc | ohne Auswahl; 50 Verlaufsschritte für 50 Änderungen |
| Zwiebelschichteneffekt | `pi.state.onionSkin` | ✅ Doc | ohne Auswahl; 50 Verlaufsschritte für 50 Änderungen |
| Hilfslinien | `pi.guides` | ✅ Doc | ohne Auswahl; 50 Verlaufsschritte für 50 Änderungen |
| Farbfeldsatz | `panel.swatches.setChoice` | ✅ Doc | ohne Auswahl; 0 Verlaufsschritte für 50 Änderungen |
