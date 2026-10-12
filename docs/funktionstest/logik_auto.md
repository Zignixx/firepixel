# Logik der Dialoge (50× ändern, Vorschau, Abbrechen, OK, Rückgängig/Wiederholen, Auswahlwechsel) – automatischer Gesamttest

Erzeugt von `crates\fp-ui\tests\gesamt_logik.rs` (`cargo test -p fp-ui --test gesamt_logik`). Nicht von Hand ändern.

**70/70 Einträge ✅**

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
| Text ▸ Größe ▸ Andere... | `text.size.other` | ✅ Doc | ohne Auswahl |
| Text ▸ Rechtschreibprüfung einrichten... | `text.spellingSetup` | ✅ Doc | ohne Auswahl |
| Filter ▸ Farbe anpassen ▸ Farbton/Sättigung... | `filter.adjustColor.hueSaturation` | ✅ Doc | Rechteck, mit Vorschau |
| Filter ▸ Farbe anpassen ▸ Helligkeit/Kontrast... | `filter.adjustColor.brightnessContrast` | ✅ Doc | Rechteck, mit Vorschau |
| Filter ▸ Rauschen ▸ Rauschen hinzufügen... | `filter.noise.addNoise` | ✅ Doc | Rechteck, mit Vorschau |
| Filter ▸ Scharfzeichnen ▸ Unscharf maskieren... | `filter.sharpen.unsharpMask` | ✅ Doc | Rechteck, mit Vorschau |
| Filter ▸ Weichzeichnen ▸ Bewegungsunschärfe… | `filter.blur.motion` | ✅ Doc | Rechteck, mit Vorschau |
| Filter ▸ Weichzeichnen ▸ Gaußscher Weichz. ... | `filter.blur.gaussian` | ✅ Doc | Rechteck, mit Vorschau |
| Filter ▸ Weichzeichnen ▸ Radiales Weichz. ... | `filter.blur.radial` | ✅ Doc | Rechteck, mit Vorschau |
| Filter ▸ Weichzeichnen ▸ Zentriertes Weichz. ... | `filter.blur.zoom` | ✅ Doc | Rechteck, mit Vorschau |
| Filter (+) ▸ effect-group-bevel ▸ effect-bevel-inner – hinzufügen | `Effect { name: "bevel", bevel: Some(0) }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ effect-group-bevel ▸ effect-bevel-inner – bearbeiten | `Effect { name: "bevel", bevel: Some(0) }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ effect-group-bevel ▸ effect-bevel-outer – hinzufügen | `Effect { name: "bevel", bevel: Some(1) }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ effect-group-bevel ▸ effect-bevel-outer – bearbeiten | `Effect { name: "bevel", bevel: Some(1) }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ effect-group-bevel ▸ effect-emboss-inset – hinzufügen | `Effect { name: "bevel", bevel: Some(3) }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ effect-group-bevel ▸ effect-emboss-inset – bearbeiten | `Effect { name: "bevel", bevel: Some(3) }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ effect-group-bevel ▸ effect-emboss-raised – hinzufügen | `Effect { name: "bevel", bevel: Some(2) }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ effect-group-bevel ▸ effect-emboss-raised – bearbeiten | `Effect { name: "bevel", bevel: Some(2) }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ effect-group-adjust-color ▸ effect-color-fill – hinzufügen | `Effect { name: "colorFill", bevel: None }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ effect-group-adjust-color ▸ effect-color-fill – bearbeiten | `Effect { name: "colorFill", bevel: None }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ effect-group-adjust-color ▸ filter-adjustColor-hueSaturation – hinzufügen | `Filter("filter.adjustColor.hueSaturation")` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ effect-group-adjust-color ▸ filter-adjustColor-hueSaturation – bearbeiten | `Filter("filter.adjustColor.hueSaturation")` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ effect-group-adjust-color ▸ filter-adjustColor-brightnessContrast – hinzufügen | `Filter("filter.adjustColor.brightnessContrast")` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ effect-group-adjust-color ▸ filter-adjustColor-brightnessContrast – bearbeiten | `Filter("filter.adjustColor.brightnessContrast")` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ effect-group-noise ▸ filter-noise-addNoise – hinzufügen | `Filter("filter.noise.addNoise")` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ effect-group-noise ▸ filter-noise-addNoise – bearbeiten | `Filter("filter.noise.addNoise")` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ effect-group-sharpen ▸ filter-sharpen-unsharpMask – hinzufügen | `Filter("filter.sharpen.unsharpMask")` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ effect-group-sharpen ▸ filter-sharpen-unsharpMask – bearbeiten | `Filter("filter.sharpen.unsharpMask")` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ effect-group-shadow-glow ▸ effect-solid-shadow – hinzufügen | `Effect { name: "solidShadow", bevel: None }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ effect-group-shadow-glow ▸ effect-solid-shadow – bearbeiten | `Effect { name: "solidShadow", bevel: None }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ effect-group-shadow-glow ▸ effect-glow – hinzufügen | `Effect { name: "glow", bevel: Some(4) }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ effect-group-shadow-glow ▸ effect-glow – bearbeiten | `Effect { name: "glow", bevel: Some(4) }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ effect-group-shadow-glow ▸ effect-inner-shadow – hinzufügen | `Effect { name: "innerShadow", bevel: None }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ effect-group-shadow-glow ▸ effect-inner-shadow – bearbeiten | `Effect { name: "innerShadow", bevel: None }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ effect-group-shadow-glow ▸ effect-inner-glow – hinzufügen | `Effect { name: "innerGlow", bevel: None }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ effect-group-shadow-glow ▸ effect-inner-glow – bearbeiten | `Effect { name: "innerGlow", bevel: None }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ effect-group-shadow-glow ▸ effect-drop-shadow – hinzufügen | `Effect { name: "dropShadow", bevel: None }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ effect-group-shadow-glow ▸ effect-drop-shadow – bearbeiten | `Effect { name: "dropShadow", bevel: None }` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ effect-group-blur ▸ filter-blur-motion – hinzufügen | `Filter("filter.blur.motion")` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ effect-group-blur ▸ filter-blur-motion – bearbeiten | `Filter("filter.blur.motion")` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ effect-group-blur ▸ filter-blur-gaussian – hinzufügen | `Filter("filter.blur.gaussian")` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ effect-group-blur ▸ filter-blur-gaussian – bearbeiten | `Filter("filter.blur.gaussian")` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ effect-group-blur ▸ filter-blur-radial – hinzufügen | `Filter("filter.blur.radial")` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ effect-group-blur ▸ filter-blur-radial – bearbeiten | `Filter("filter.blur.radial")` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ effect-group-blur ▸ filter-blur-zoom – hinzufügen | `Filter("filter.blur.zoom")` | ✅ Doc | ohne Auswahl, mit Vorschau |
| Filter (+) ▸ effect-group-blur ▸ filter-blur-zoom – bearbeiten | `Filter("filter.blur.zoom")` | ✅ Doc | ohne Auswahl, mit Vorschau |
