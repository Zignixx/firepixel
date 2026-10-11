# Logik der Dialoge (50× ändern, Vorschau, Abbrechen, OK, Rückgängig/Wiederholen, Auswahlwechsel) – automatischer Gesamttest

Erzeugt von `crates\fp-ui\tests\gesamt_logik.rs` (`cargo test -p fp-ui --test gesamt_logik`). Nicht von Hand ändern.

**72/72 Einträge ✅**

| Eintrag | Befehl | Ergebnis | Hinweis |
|---|---|---|---|
| Datei ▸ Exportassistent... | `file.exportWizard` | ✅ Doc | ohne Auswahl |
| Datei ▸ Batch-Verarbeitung... | `file.batchProcess` | ✅ Doc | ohne Auswahl |
| Datei ▸ Seite einrichten... | `file.pageSetup` | ✅ Doc | ohne Auswahl |
| Datei ▸ Drucken... | `file.print` | ✅ Doc | ohne Auswahl |
| Datei ▸ HTML-Einrichtung... | `file.htmlSetup` | ✅ Doc | ohne Auswahl |
| Bearbeiten ▸ Objekt einfügen ▸ Neue Schaltfläche... | `edit.insert.newButton` | ✅ Doc | ohne Auswahl |
| Bearbeiten ▸ Objekt einfügen ▸ Neues Symbol... | `edit.insert.newSymbol` | ✅ Doc | ohne Auswahl |
| Ansicht ▸ Raster ▸ Raster bearbeiten... | `view.grid.editGrid` | ✅ Doc | ohne Auswahl |
| Auswählen ▸ Weiche Kante… | `select.feather` | ✅ Doc | Pixelauswahl |
| Auswählen ▸ Auswahlrahmen erweitern... | `select.expandMarquee` | ✅ Doc | Pixelauswahl |
| Auswählen ▸ Auswahlrahmen verkleinern... | `select.contractMarquee` | ✅ Doc | Pixelauswahl |
| Auswählen ▸ Auswahlrahmen begrenzen... | `select.borderMarquee` | ✅ Doc | Pixelauswahl |
| Auswählen ▸ Auswahlrahmen glätten... | `select.smoothMarquee` | ✅ Doc | Pixelauswahl |
| Auswählen ▸ Bitmap-Auswahl speichern... | `select.saveBitmapSelection` | ✅ Doc | Pixelauswahl |
| Auswählen ▸ Bitmap-Auswahl wiederherstellen... | `select.restoreBitmapSelection` | ✅ Ui | Hinweisdialog ohne Befehl |
| Auswählen ▸ Bitmap-Auswahl löschen... | `select.deleteBitmapSelection` | ✅ Ui | Hinweisdialog ohne Befehl |
| Modifizieren ▸ Leinwand ▸ Leinwandfarbe... | `modify.canvas.canvasColor` | ✅ Doc | ohne Auswahl |
| Modifizieren ▸ Animation ▸ Auswahl animieren... | `modify.animation.animateSelection` | ✅ Doc | Rechteck |
| Modifizieren ▸ Animation ▸ Einstellungen... | `modify.animation.settings` | ✅ Doc | Animationssymbol |
| Modifizieren ▸ Symbol ▸ In Symbol konvertieren... | `modify.symbol.convertToSymbol` | ✅ Doc | Rechteck |
| Modifizieren ▸ Symbol ▸ Instanzen tweenen... | `modify.symbol.tweenInstances` | ✅ Doc | zwei Instanzen |
| Modifizieren ▸ Symbol ▸ Symbol austauschen... | `modify.symbol.swapSymbol` | ✅ Doc | Grafiksymbol |
| Modifizieren ▸ Selektives JPEG ▸ Einstellungen... | `modify.selectiveJpeg.settings` | ✅ Doc | ohne Auswahl |
| Modifizieren ▸ Pfad ändern ▸ Vereinfachen... | `modify.alterPath.simplify` | ✅ Doc | Rechteck |
| Modifizieren ▸ Pfad ändern ▸ Strich erweitern... | `modify.alterPath.expandStroke` | ✅ Doc | Rechteck |
| Modifizieren ▸ Pfad ändern ▸ Pfad einlassen... | `modify.alterPath.insetPath` | ✅ Doc | Rechteck |
| Text ▸ Größe ▸ Andere... | `text.size.other` | ✅ Doc | Text |
| Text ▸ Rechtschreibprüfung einrichten... | `text.spellingSetup` | ✅ Doc | ohne Auswahl |
| Filter ▸ Farbe anpassen ▸ Farbton/Sättigung... | `filter.adjustColor.hueSaturation` | ✅ Doc | Rechteck, mit Vorschau |
| Filter ▸ Farbe anpassen ▸ Helligkeit/Kontrast... | `filter.adjustColor.brightnessContrast` | ✅ Doc | Rechteck, mit Vorschau |
| Filter ▸ Rauschen ▸ Rauschen hinzufügen... | `filter.noise.addNoise` | ✅ Doc | Rechteck, mit Vorschau |
| Filter ▸ Scharfzeichnen ▸ Unscharf maskieren... | `filter.sharpen.unsharpMask` | ✅ Doc | Rechteck, mit Vorschau |
| Filter ▸ Weichzeichnen ▸ Bewegungsunschärfe... | `filter.blur.motion` | ✅ Doc | Rechteck, mit Vorschau |
| Filter ▸ Weichzeichnen ▸ Gaußscher Weichzeichner... | `filter.blur.gaussian` | ✅ Doc | Rechteck, mit Vorschau |
| Filter ▸ Weichzeichnen ▸ Radiales Weichzeichnen... | `filter.blur.radial` | ✅ Doc | Rechteck, mit Vorschau |
| Filter ▸ Weichzeichnen ▸ Zentriertes Weichzeichnen... | `filter.blur.zoom` | ✅ Doc | Rechteck, mit Vorschau |
| Filter (+) ▸ Abschrägung und Prägung ▸ Abschrägung innen – hinzufügen | `Effect { name: "bevel", bevel: Some(0) }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ Abschrägung und Prägung ▸ Abschrägung innen – bearbeiten | `Effect { name: "bevel", bevel: Some(0) }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ Abschrägung und Prägung ▸ Abschrägung außen – hinzufügen | `Effect { name: "bevel", bevel: Some(1) }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ Abschrägung und Prägung ▸ Abschrägung außen – bearbeiten | `Effect { name: "bevel", bevel: Some(1) }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ Abschrägung und Prägung ▸ Prägung eingelassen – hinzufügen | `Effect { name: "bevel", bevel: Some(3) }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ Abschrägung und Prägung ▸ Prägung eingelassen – bearbeiten | `Effect { name: "bevel", bevel: Some(3) }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ Abschrägung und Prägung ▸ Prägung erhaben – hinzufügen | `Effect { name: "bevel", bevel: Some(2) }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ Abschrägung und Prägung ▸ Prägung erhaben – bearbeiten | `Effect { name: "bevel", bevel: Some(2) }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ Anpassen der Farbe ▸ Farbfüllung – hinzufügen | `Effect { name: "colorFill", bevel: None }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ Anpassen der Farbe ▸ Farbfüllung – bearbeiten | `Effect { name: "colorFill", bevel: None }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ Anpassen der Farbe ▸ Farbton/Sättigung... – hinzufügen | `Filter("filter.adjustColor.hueSaturation")` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ Anpassen der Farbe ▸ Farbton/Sättigung... – bearbeiten | `Filter("filter.adjustColor.hueSaturation")` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ Anpassen der Farbe ▸ Helligkeit/Kontrast... – hinzufügen | `Filter("filter.adjustColor.brightnessContrast")` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ Anpassen der Farbe ▸ Helligkeit/Kontrast... – bearbeiten | `Filter("filter.adjustColor.brightnessContrast")` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ Rauschen ▸ Rauschen hinzufügen... – hinzufügen | `Filter("filter.noise.addNoise")` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ Rauschen ▸ Rauschen hinzufügen... – bearbeiten | `Filter("filter.noise.addNoise")` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ Schärfen ▸ Unscharf maskieren... – hinzufügen | `Filter("filter.sharpen.unsharpMask")` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ Schärfen ▸ Unscharf maskieren... – bearbeiten | `Filter("filter.sharpen.unsharpMask")` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ Schatten und Glühen ▸ Einfarbiger Schatten – hinzufügen | `Effect { name: "solidShadow", bevel: None }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ Schatten und Glühen ▸ Einfarbiger Schatten – bearbeiten | `Effect { name: "solidShadow", bevel: None }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ Schatten und Glühen ▸ Glühen – hinzufügen | `Effect { name: "glow", bevel: Some(4) }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ Schatten und Glühen ▸ Glühen – bearbeiten | `Effect { name: "glow", bevel: Some(4) }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ Schatten und Glühen ▸ Innerer Schatten – hinzufügen | `Effect { name: "innerShadow", bevel: None }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ Schatten und Glühen ▸ Innerer Schatten – bearbeiten | `Effect { name: "innerShadow", bevel: None }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ Schatten und Glühen ▸ Inneres Glühen – hinzufügen | `Effect { name: "innerGlow", bevel: None }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ Schatten und Glühen ▸ Inneres Glühen – bearbeiten | `Effect { name: "innerGlow", bevel: None }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ Schatten und Glühen ▸ Schlagschatten – hinzufügen | `Effect { name: "dropShadow", bevel: None }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ Schatten und Glühen ▸ Schlagschatten – bearbeiten | `Effect { name: "dropShadow", bevel: None }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ Weichzeichnen ▸ Bewegungsunschärfe... – hinzufügen | `Filter("filter.blur.motion")` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ Weichzeichnen ▸ Bewegungsunschärfe... – bearbeiten | `Filter("filter.blur.motion")` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ Weichzeichnen ▸ Gaußscher Weichzeichner... – hinzufügen | `Filter("filter.blur.gaussian")` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ Weichzeichnen ▸ Gaußscher Weichzeichner... – bearbeiten | `Filter("filter.blur.gaussian")` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ Weichzeichnen ▸ Radialer Weichzeichner... – hinzufügen | `Filter("filter.blur.radial")` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ Weichzeichnen ▸ Radialer Weichzeichner... – bearbeiten | `Filter("filter.blur.radial")` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ Weichzeichnen ▸ Zoom-Weichzeichner... – hinzufügen | `Filter("filter.blur.zoom")` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ Weichzeichnen ▸ Zoom-Weichzeichner... – bearbeiten | `Filter("filter.blur.zoom")` | ✅ Doc | ohne Auswahl, mit Vorschau |
