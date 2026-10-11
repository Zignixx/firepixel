# Hauptmenü – automatischer Gesamttest

Erzeugt von `crates\fp-ui\tests\gesamt_menu.rs` (`cargo test -p fp-ui --test gesamt_menu`). Nicht von Hand ändern.

**306/306 Einträge ✅**

| Eintrag | Befehl | Ergebnis | Hinweis |
|---|---|---|---|
| Datei ▸ Neu | `file.new` | ✅ Dialog | ohne Auswahl |
| Datei ▸ Neu von Vorlage... | `file.newFromTemplate` | ✅ Ui | ohne Auswahl: Dokument geöffnet/gewechselt |
| Datei ▸ Öffnen... | `file.open` | ✅ Dialog | ohne Auswahl: Meldung „„C:\Users\ADMINI~1\AppData\Local\Temp\firepixel-gesamttest\14740-25\browser.exe“ konnte nicht geöffnet werden: FW-PNG: keine PNG-Signatur“ |
| Datei ▸ Bridge durchsuchen... | `file.browseInBridge` | ✅ Ui | ohne Auswahl: Befehl liefert {"files":[{"bytes":2121,"modified":1791748533,"name":"vorlage.fw.png","path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttes – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Datei ▸ Zuletzt geöffnete Dateien ▸ 1 C:\Users\ADMINI~1\AppData\Local\Temp\firepixel-gesamttest\14740-2\vorlage.fw.png | `OpenPath("C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\14740-2\\vorlage.fw.png")` | ✅ Ui | ohne Auswahl: Dokument geöffnet/gewechselt |
| Datei ▸ Scannen ▸ Twain importieren... | `file.twainAcquire` | ✅ Ui | ohne Auswahl: Meldung „Bilderfassung: Starten von Fremdprogrammen ist abgeschaltet (FP_NO_SHELL_OPEN)“ |
| Datei ▸ Scannen ▸ Twain auswählen... | `file.twainSelect` | ✅ Ui | ohne Auswahl: Meldung „Bilderfassung: Starten von Fremdprogrammen ist abgeschaltet (FP_NO_SHELL_OPEN)“ |
| Datei ▸ Schließen | `file.close` | ✅ Dialog | ohne Auswahl |
| Datei ▸ Alle schließen | `file.closeAll` | ✅ Dialog | ohne Auswahl |
| Datei ▸ Speichern | `file.save` | ✅ Ui | ohne Auswahl |
| Datei ▸ Speichern unter... | `file.saveAs` | ✅ Ui | ohne Auswahl |
| Datei ▸ Alles speichern | `file.saveAll` | ✅ Ui | ohne Auswahl |
| Datei ▸ Als Vorlage speichern... | `file.saveAsTemplate` | ✅ Ui | ohne Auswahl: Datei geschrieben |
| Datei ▸ Wiederherstellen | `file.revert` | ✅ Dialog | gespeichert + geändert |
| Datei ▸ Importieren... | `file.import` | ✅ Doc | ohne Auswahl |
| Datei ▸ Exportieren... | `file.export` | ✅ Dialog | ohne Auswahl |
| Datei ▸ Bildvorschau... | `file.imagePreview` | ✅ Dialog | ohne Auswahl |
| Datei ▸ Als E-Mail senden ▸ PNG-Datei für Firepixel... | `file.sendToEMail.firepixelPng` | ✅ Ui | ohne Auswahl: Befehl liefert {"attachment":"Unbenannt-2.fw.png","bytes":14085,"eml":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\Firepixel\\EMail\\Unbenannt-2.eml"} – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Datei ▸ Als E-Mail senden ▸ Komprimierte JPG-Datei... | `file.sendToEMail.jpegCompressed` | ✅ Ui | ohne Auswahl: Befehl liefert {"attachment":"Unbenannt-2.jpg","bytes":17655,"eml":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\Firepixel\\EMail\\Unbenannt-2.eml"} – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Datei ▸ Als E-Mail senden ▸ Exporteinstellungen verwenden... | `file.sendToEMail.useExportSettings` | ✅ Ui | ohne Auswahl: Befehl liefert {"attachment":"Unbenannt-2.gif","bytes":7828,"eml":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\Firepixel\\EMail\\Unbenannt-2.eml"} – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Datei ▸ HTML aktualisieren... | `file.updateHtml` | ✅ Ui | ohne Auswahl: Datei geschrieben |
| Datei ▸ Datei-Info... | `file.fileInfo` | ✅ Dialog | ohne Auswahl: Meldung „Titel: Unbenannt-2.png Pfad: (nicht gespeichert) Seiten: 2 Symbole: 3 Formatversion: 10“ |
| Datei ▸ Exportassistent... | `file.exportWizard` | ✅ Dialog | ohne Auswahl |
| Datei ▸ Batch-Verarbeitung... | `file.batchProcess` | ✅ Dialog | ohne Auswahl |
| Datei ▸ Vorschau im Browser ▸ Vorschau im Primärbrowser | `file.previewInBrowser.previewInPrimaryBrowser` | ✅ Ui | ohne Auswahl: Befehl liefert {"files":["C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\Firepixel\\Vorschau\\dok2\\Unbenannt-2_r1_c1.gif","C:\\Users\\ADMINI~1\\AppData\\Local – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Datei ▸ Vorschau im Browser ▸ Vorschau im Sekundärbrowser | `file.previewInBrowser.previewInSecondaryBrowser` | ✅ Ui | ohne Auswahl: Befehl liefert {"files":["C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\Firepixel\\Vorschau\\dok2\\Unbenannt-2_r1_c1.gif","C:\\Users\\ADMINI~1\\AppData\\Local – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Datei ▸ Vorschau im Browser ▸ Vorschau aller Seiten im Primärbrowser | `file.previewInBrowser.previewAllPagesInPrimaryBrowser` | ✅ Ui | ohne Auswahl: Befehl liefert {"files":["C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\Firepixel\\Vorschau\\dok2\\Unbenannt-2_s1_r1_c1.gif","C:\\Users\\ADMINI~1\\AppData\\Lo – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Datei ▸ Vorschau im Browser ▸ Primärbrowser festlegen... | `file.previewInBrowser.setPrimaryBrowser` | ✅ Ui | ohne Auswahl |
| Datei ▸ Vorschau im Browser ▸ Sekundärbrowser festlegen... | `file.previewInBrowser.setSecondaryBrowser` | ✅ Ui | ohne Auswahl |
| Datei ▸ Seite einrichten... | `file.pageSetup` | ✅ Dialog | ohne Auswahl |
| Datei ▸ Drucken... | `file.print` | ✅ Dialog | ohne Auswahl |
| Datei ▸ HTML-Einrichtung... | `file.htmlSetup` | ✅ Dialog | ohne Auswahl |
| Datei ▸ Beenden | `file.exit` | ✅ Dialog | ohne Auswahl |
| Bearbeiten ▸ Rückgängig | `edit.undo` | ✅ Ui | Pixelauswahl |
| Bearbeiten ▸ Wiederholen | `edit.redo` | ✅ Doc | nach Rückgängig |
| Bearbeiten ▸ Objekt einfügen ▸ Neue Schaltfläche... | `edit.insert.newButton` | ✅ Dialog | ohne Auswahl |
| Bearbeiten ▸ Objekt einfügen ▸ Neues Symbol... | `edit.insert.newSymbol` | ✅ Dialog | ohne Auswahl |
| Bearbeiten ▸ Objekt einfügen ▸ Hotspot | `edit.insert.hotspot` | ✅ Doc | Rechteck |
| Bearbeiten ▸ Objekt einfügen ▸ Rechteckiges Segment | `edit.insert.slice` | ✅ Doc | Rechteck |
| Bearbeiten ▸ Objekt einfügen ▸ Polygonsegment | `edit.insert.polygonSlice` | ✅ Doc | Rechteck |
| Bearbeiten ▸ Objekt einfügen ▸ Leere Bitmap | `edit.insert.emptyBitmap` | ✅ Doc | ohne Auswahl |
| Bearbeiten ▸ Objekt einfügen ▸ Bitmap durch Kopieren | `edit.insert.bitmapViaCopy` | ✅ Doc | Bitmap + Pixelauswahl |
| Bearbeiten ▸ Objekt einfügen ▸ Bitmap durch Ausschneiden | `edit.insert.bitmapViaCut` | ✅ Doc | Bitmap + Pixelauswahl |
| Bearbeiten ▸ Objekt einfügen ▸ Ebene | `edit.insert.layer` | ✅ Dialog | ohne Auswahl |
| Bearbeiten ▸ Objekt einfügen ▸ Status | `edit.insert.state` | ✅ Dialog | ohne Auswahl |
| Bearbeiten ▸ Objekt einfügen ▸ Seite | `edit.insert.page` | ✅ Doc | ohne Auswahl |
| Bearbeiten ▸ Suchen und ersetzen... | `edit.findAndReplace` | ✅ Ui | ohne Auswahl |
| Bearbeiten ▸ Ausschneiden | `edit.cut` | ✅ Doc | Rechteck |
| Bearbeiten ▸ Kopieren | `edit.copy` | ✅ Ui | Rechteck |
| Bearbeiten ▸ Als Vektoren kopieren | `edit.copyAsVectors` | ✅ Ui | Rechteck |
| Bearbeiten ▸ HTML-Code kopieren... | `edit.copyHtmlCode` | ✅ Ui | ohne Auswahl: Meldung „HTML-Code in die Zwischenablage kopiert“ |
| Bearbeiten ▸ Einfügen | `edit.paste` | ✅ Doc | ohne Auswahl |
| Bearbeiten ▸ Löschen | `edit.clear` | ✅ Doc | Rechteck |
| Bearbeiten ▸ Als Maske einfügen | `edit.pasteAsMask` | ✅ Doc | Rechteck |
| Bearbeiten ▸ Innen einfügen | `edit.pasteInside` | ✅ Doc | Rechteck |
| Bearbeiten ▸ Attribute einfügen | `edit.pasteAttributes` | ✅ Doc | Rechteck |
| Bearbeiten ▸ Duplizieren | `edit.duplicate` | ✅ Doc | Rechteck |
| Bearbeiten ▸ Klonen | `edit.clone` | ✅ Doc | Rechteck |
| Bearbeiten ▸ Ausgewählte Bitmap zuschneiden | `edit.cropSelectedBitmap` | ✅ Doc | Bitmap |
| Bearbeiten ▸ Dokument zuschneiden | `edit.cropDocument` | ✅ Doc | Rechteck |
| Bearbeiten ▸ Voreinstellungen... | `edit.preferences` | ✅ Dialog | ohne Auswahl |
| Bearbeiten ▸ Tastaturkurzbefehle... | `edit.keyboardShortcuts` | ✅ Dialog | ohne Auswahl |
| Ansicht ▸ Größer zoomen | `view.zoomIn` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Kleiner zoomen | `view.zoomOut` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Vergrößerung ▸ 6% | `view.magnification.6` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Vergrößerung ▸ 12% | `view.magnification.12` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Vergrößerung ▸ 25% | `view.magnification.25` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Vergrößerung ▸ 50% | `view.magnification.50` | ✅ Ui | ohne Auswahl: keine erkennbare Wirkung |
| Ansicht ▸ Vergrößerung ▸ 66% | `view.magnification.66` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Vergrößerung ▸ 100% | `view.magnification.100` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Vergrößerung ▸ 150% | `view.magnification.150` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Vergrößerung ▸ 200% | `view.magnification.200` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Vergrößerung ▸ 300% | `view.magnification.300` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Vergrößerung ▸ 400% | `view.magnification.400` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Vergrößerung ▸ 800% | `view.magnification.800` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Vergrößerung ▸ 1600% | `view.magnification.1600` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Vergrößerung ▸ 3200% | `view.magnification.3200` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Vergrößerung ▸ 6400% | `view.magnification.6400` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Auswahl anpassen | `view.fitSelection` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Alles anpassen | `view.fitAll` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Gamma 1.8 | `view.macintoshGamma` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Auswahl ausblenden | `view.hideSelection` | ✅ Doc | Rechteck |
| Ansicht ▸ QuickInfos | `view.tooltips` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Alles einblenden | `view.showAll` | ✅ Doc | ohne Auswahl |
| Ansicht ▸ Lineale | `view.rulers` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Raster ▸ Raster anzeigen | `view.grid.showGrid` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Raster ▸ Am Raster ausrichten | `view.grid.snapToGrid` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Raster ▸ Raster bearbeiten... | `view.grid.editGrid` | ✅ Dialog | ohne Auswahl |
| Ansicht ▸ Hilfslinien ▸ Hilfslinien anzeigen | `view.guides.showGuides` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Hilfslinien ▸ Hilfslinien sperren | `view.guides.lockGuides` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Hilfslinien ▸ An Hilfslinien ausrichten | `view.guides.snapToGuides` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Hilfslinien ▸ Hilfslinien entfernen | `view.guides.clearGuides` | ✅ Doc | ohne Auswahl |
| Ansicht ▸ Intelligente Hilfslinien ▸ Intelligente Hilfslinien anzeigen | `view.smartGuides.showSmartGuides` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Intelligente Hilfslinien ▸ An intelligenten Hilfslinien ausrichten | `view.smartGuides.snapToSmartGuides` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Hilfslinien für 9-Slice-Skalierung ▸ Hilfslinien sperren | `view.nineSliceScalingGuides.lockGuides` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Segment-Hilfslinien | `view.sliceGuides` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Segmentüberlagerung | `view.sliceOverlay` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Ränder | `view.edges` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Design ▸ Normal | `SetTheme(Normal)` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Design ▸ Dunkel | `SetTheme(Dark)` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Design ▸ OLED | `SetTheme(Oled)` | ✅ Ui | ohne Auswahl |
| Auswählen ▸ Alles auswählen | `select.all` | ✅ Doc | ohne Auswahl |
| Auswählen ▸ Auswahl aufheben | `select.none` | ✅ Doc | Rechteck |
| Auswählen ▸ Gesamtauswahl treffen | `select.superselect` | ✅ Doc | Rechteck + Ellipse |
| Auswählen ▸ Teilauswahl treffen | `select.subselect` | ✅ Doc | Rechteck + Ellipse |
| Auswählen ▸ Ähnliches auswählen | `select.similar` | ✅ Doc | Rechteck + Ellipse |
| Auswählen ▸ Motiv auswählen | `select.aiSubject` | ✅ Ui | ohne Auswahl |
| Auswählen ▸ Kante verfeinern... | `select.aiRefineEdge` | ✅ Ui | Pixelauswahl: Meldung „Kante verfeinern: fertig“ |
| Auswählen ▸ Auswahl umkehren | `select.inverse` | ✅ Doc | ohne Auswahl |
| Auswählen ▸ Weiche Kante… | `select.feather` | ✅ Dialog | Pixelauswahl |
| Auswählen ▸ Auswahlrahmen erweitern... | `select.expandMarquee` | ✅ Dialog | Pixelauswahl |
| Auswählen ▸ Auswahlrahmen verkleinern... | `select.contractMarquee` | ✅ Dialog | Pixelauswahl |
| Auswählen ▸ Auswahlrahmen begrenzen... | `select.borderMarquee` | ✅ Dialog | Pixelauswahl |
| Auswählen ▸ Auswahlrahmen glätten... | `select.smoothMarquee` | ✅ Dialog | Pixelauswahl |
| Auswählen ▸ Auswahlrahmen in Pfad konvertieren | `select.convertMarqueeToPath` | ✅ Doc | Pixelauswahl |
| Auswählen ▸ Bitmap-Auswahl speichern... | `select.saveBitmapSelection` | ✅ Dialog | Pixelauswahl |
| Auswählen ▸ Bitmap-Auswahl wiederherstellen... | `select.restoreBitmapSelection` | ✅ Dialog | ohne Auswahl |
| Auswählen ▸ Bitmap-Auswahl löschen... | `select.deleteBitmapSelection` | ✅ Dialog | ohne Auswahl |
| Modifizieren ▸ Leinwand ▸ Bildgröße... | `modify.canvas.imageSize` | ✅ Dialog | ohne Auswahl |
| Modifizieren ▸ Leinwand ▸ Leinwandgröße... | `modify.canvas.canvasSize` | ✅ Dialog | ohne Auswahl |
| Modifizieren ▸ Leinwand ▸ Leinwandfarbe... | `modify.canvas.canvasColor` | ✅ Dialog | ohne Auswahl |
| Modifizieren ▸ Leinwand ▸ Leinwand zuschneiden | `modify.canvas.trimCanvas` | ✅ Doc | ohne Auswahl |
| Modifizieren ▸ Leinwand ▸ Leinwand anpassen | `modify.canvas.fitCanvas` | ✅ Doc | ohne Auswahl |
| Modifizieren ▸ Leinwand ▸ 180° drehen | `modify.canvas.rotate180Degree` | ✅ Doc | ohne Auswahl |
| Modifizieren ▸ Leinwand ▸ 90° im UZS drehen | `modify.canvas.rotate90DegreeCw` | ✅ Doc | ohne Auswahl |
| Modifizieren ▸ Leinwand ▸ 90° gegen UZS drehen | `modify.canvas.rotate90DegreeCcw` | ✅ Doc | ohne Auswahl |
| Modifizieren ▸ Animation ▸ Auswahl animieren... | `modify.animation.animateSelection` | ✅ Dialog | Rechteck |
| Modifizieren ▸ Animation ▸ Einstellungen... | `modify.animation.settings` | ✅ Dialog | Animationssymbol |
| Modifizieren ▸ Animation ▸ Animation entfernen | `modify.animation.removeAnimation` | ✅ Doc | Animationssymbol |
| Modifizieren ▸ Symbol ▸ In Symbol konvertieren... | `modify.symbol.convertToSymbol` | ✅ Dialog | Rechteck |
| Modifizieren ▸ Symbol ▸ Symbol bearbeiten | `modify.symbol.editSymbol` | ✅ Ui | Grafiksymbol: Dokument geöffnet/gewechselt |
| Modifizieren ▸ Symbol ▸ An Position bearbeiten | `modify.symbol.editInPlace` | ✅ Ui | Grafiksymbol: Dokument geöffnet/gewechselt |
| Modifizieren ▸ Symbol ▸ Instanzen tweenen... | `modify.symbol.tweenInstances` | ✅ Dialog | zwei Instanzen |
| Modifizieren ▸ Symbol ▸ Teilen | `modify.symbol.breakApart` | ✅ Doc | Grafiksymbol |
| Modifizieren ▸ Symbol ▸ Symbol austauschen... | `modify.symbol.swapSymbol` | ✅ Dialog | Grafiksymbol |
| Modifizieren ▸ Popupmenü ▸ Popupmenü hinzufügen... | `modify.popUpMenu.addPopUpMenu` | ✅ Dialog | Segment |
| Modifizieren ▸ Popupmenü ▸ Popupmenü bearbeiten... | `modify.popUpMenu.editPopUpMenu` | ✅ Dialog | Segment: Meldung „Bitte zuerst ein Segment oder einen Hotspot mit Pop-up-Menü auswählen.“ |
| Modifizieren ▸ Popupmenü ▸ Popupmenü löschen | `modify.popUpMenu.deletePopUpMenu` | ✅ Doc | Segment |
| Modifizieren ▸ Maske ▸ Alles einblenden | `modify.mask.revealAll` | ✅ Doc | Rechteck |
| Modifizieren ▸ Maske ▸ Alles ausblenden | `modify.mask.hideAll` | ✅ Doc | Rechteck |
| Modifizieren ▸ Maske ▸ Als Maske einfügen | `edit.pasteAsMask` | ✅ Doc | Rechteck |
| Modifizieren ▸ Maske ▸ Als Maske gruppieren | `modify.mask.groupAsMask` | ✅ Doc | Rechteck + Ellipse |
| Modifizieren ▸ Maske ▸ Auswahl einblenden | `modify.mask.revealSelection` | ✅ Doc | Bitmap + Pixelauswahl |
| Modifizieren ▸ Maske ▸ Auswahl ausblenden | `modify.mask.hideSelection` | ✅ Doc | Bitmap + Pixelauswahl |
| Modifizieren ▸ Maske ▸ Maske deaktivieren | `modify.mask.disableMask` | ✅ Doc | Maskengruppe |
| Modifizieren ▸ Maske ▸ Maske löschen | `modify.mask.deleteMask` | ✅ Doc | Maskengruppe |
| Modifizieren ▸ Selektives JPEG ▸ Auswahl als JPEG-Maske speichern | `modify.selectiveJpeg.saveSelectionAsJpegMask` | ✅ Doc | Pixelauswahl |
| Modifizieren ▸ Selektives JPEG ▸ JPEG-Maske als Auswahl wiederherstellen | `modify.selectiveJpeg.restoreJpegMaskAsSelection` | ✅ Doc | mit JPEG-Maske |
| Modifizieren ▸ Selektives JPEG ▸ Einstellungen... | `modify.selectiveJpeg.settings` | ✅ Dialog | ohne Auswahl |
| Modifizieren ▸ Selektives JPEG ▸ JPEG-Maske entfernen | `modify.selectiveJpeg.removeJpegMask` | ✅ Doc | mit JPEG-Maske |
| Modifizieren ▸ Auswahl sperren | `modify.lockSelection` | ✅ Doc | Rechteck |
| Modifizieren ▸ Auswahl auf Hintergrund reduzieren | `modify.flattenSelection` | ✅ Doc | Rechteck |
| Modifizieren ▸ Mit darunter liegender auf eine Ebene reduzieren | `modify.mergeDown` | ✅ Doc | Text |
| Modifizieren ▸ Ebenen als Hintergrund | `modify.flattenLayers` | ✅ Doc | ohne Auswahl |
| Modifizieren ▸ Transformieren ▸ Frei transformieren | `modify.transform.freeTransform` | ✅ Ui | Rechteck |
| Modifizieren ▸ Transformieren ▸ Skalieren | `modify.transform.scale` | ✅ Ui | Rechteck |
| Modifizieren ▸ Transformieren ▸ Neigen | `modify.transform.skew` | ✅ Ui | Rechteck |
| Modifizieren ▸ Transformieren ▸ Verzerren | `modify.transform.distort` | ✅ Ui | Rechteck |
| Modifizieren ▸ Transformieren ▸ Numerisch transformieren... | `modify.transform.numericTransform` | ✅ Dialog | Rechteck |
| Modifizieren ▸ Transformieren ▸ 180° drehen | `modify.transform.rotate180Degree` | ✅ Doc | Rechteck |
| Modifizieren ▸ Transformieren ▸ 90° im UZS drehen | `modify.transform.rotate90DegreeCw` | ✅ Doc | Rechteck |
| Modifizieren ▸ Transformieren ▸ 90° gegen UZS drehen | `modify.transform.rotate90DegreeCcw` | ✅ Doc | Rechteck |
| Modifizieren ▸ Transformieren ▸ Horizontal spiegeln | `modify.transform.flipHorizontal` | ✅ Doc | Rechteck |
| Modifizieren ▸ Transformieren ▸ Vertikal spiegeln | `modify.transform.flipVertical` | ✅ Doc | Rechteck |
| Modifizieren ▸ Transformieren ▸ Transformationen entfernen | `modify.transform.removeTransformations` | ✅ Doc | Rechteck |
| Modifizieren ▸ Anordnen ▸ In den Vordergrund | `modify.arrange.bringToFront` | ✅ Doc | Rechteck |
| Modifizieren ▸ Anordnen ▸ Schrittweise nach vorne | `modify.arrange.bringForward` | ✅ Doc | Rechteck |
| Modifizieren ▸ Anordnen ▸ Schrittweise nach hinten | `modify.arrange.sendBackward` | ✅ Doc | Rechteck |
| Modifizieren ▸ Anordnen ▸ In den Hintergrund | `modify.arrange.sendToBack` | ✅ Doc | Rechteck |
| Modifizieren ▸ Ausrichten ▸ Links | `modify.align.left` | ✅ Doc | Rechteck |
| Modifizieren ▸ Ausrichten ▸ Vertikal zentrieren | `modify.align.centerVertical` | ✅ Doc | Rechteck |
| Modifizieren ▸ Ausrichten ▸ Rechts | `modify.align.right` | ✅ Doc | Rechteck |
| Modifizieren ▸ Ausrichten ▸ Oben | `modify.align.top` | ✅ Doc | Rechteck |
| Modifizieren ▸ Ausrichten ▸ Horizontal zentrieren | `modify.align.centerHorizontal` | ✅ Doc | Rechteck |
| Modifizieren ▸ Ausrichten ▸ Unten | `modify.align.bottom` | ✅ Doc | Rechteck |
| Modifizieren ▸ Ausrichten ▸ Breiten verteilen | `modify.align.distributeWidths` | ✅ Doc | drei Objekte |
| Modifizieren ▸ Ausrichten ▸ Höhen verteilen | `modify.align.distributeHeights` | ✅ Doc | drei Objekte |
| Modifizieren ▸ Pfad in Auswahlrahmen konvertieren... | `modify.convertPathToMarquee` | ✅ Doc | Rechteck |
| Modifizieren ▸ Pfade kombinieren ▸ Verbinden | `modify.combinePaths.join` | ✅ Doc | Rechteck + Ellipse |
| Modifizieren ▸ Pfade kombinieren ▸ Teilen | `modify.combinePaths.split` | ✅ Doc | Rechteck |
| Modifizieren ▸ Pfade kombinieren ▸ Vereinigen | `modify.combinePaths.union` | ✅ Doc | Rechteck + Ellipse |
| Modifizieren ▸ Pfade kombinieren ▸ Überschneiden | `modify.combinePaths.intersect` | ✅ Doc | Rechteck + Ellipse |
| Modifizieren ▸ Pfade kombinieren ▸ Öffnen | `modify.combinePaths.punch` | ✅ Doc | Rechteck + Ellipse |
| Modifizieren ▸ Pfade kombinieren ▸ Zuschneiden | `modify.combinePaths.crop` | ✅ Doc | Rechteck + Ellipse |
| Modifizieren ▸ Pfad ändern ▸ Vereinfachen... | `modify.alterPath.simplify` | ✅ Dialog | Rechteck |
| Modifizieren ▸ Pfad ändern ▸ Strich erweitern... | `modify.alterPath.expandStroke` | ✅ Dialog | Rechteck |
| Modifizieren ▸ Pfad ändern ▸ Pfad einlassen... | `modify.alterPath.insetPath` | ✅ Dialog | Rechteck |
| Modifizieren ▸ Pfad ändern ▸ Präzise Füllung | `modify.alterPath.hardFill` | ✅ Doc | Rechteck |
| Modifizieren ▸ Pfad ändern ▸ Anti-Alias-Füllung | `modify.alterPath.antiAliasFill` | ✅ Doc | Rechteck |
| Modifizieren ▸ Pfad ändern ▸ Weiche-Kante-Füllung | `modify.alterPath.featherFill` | ✅ Doc | Rechteck |
| Modifizieren ▸ An Pixeln ausrichten | `modify.snapToPixel` | ✅ Doc | Rechteck |
| Modifizieren ▸ Gruppieren | `modify.group` | ✅ Doc | Rechteck |
| Modifizieren ▸ Gruppierung aufheben | `modify.ungroup` | ✅ Doc | drei Objekte |
| Text ▸ Schrift ▸ Arial | `text.font {"font":"Arial"}` | ✅ Doc | Text |
| Text ▸ Schrift ▸ Segoe UI | `text.font {"font":"SegoeUI"}` | ✅ Doc | Text |
| Text ▸ Schrift ▸ Tahoma | `text.font {"font":"Tahoma"}` | ✅ Doc | Text |
| Text ▸ Größe ▸ Andere... | `text.size.other` | ✅ Dialog | Text |
| Text ▸ Größe ▸ Kleiner | `text.size.smaller` | ✅ Doc | Text |
| Text ▸ Größe ▸ Größer | `text.size.larger` | ✅ Doc | Text |
| Text ▸ Größe ▸ 8 | `text.size.8` | ✅ Doc | Text |
| Text ▸ Größe ▸ 9 | `text.size.9` | ✅ Doc | Text |
| Text ▸ Größe ▸ 10 | `text.size.10` | ✅ Doc | Text |
| Text ▸ Größe ▸ 12 | `text.size.12` | ✅ Doc | Text |
| Text ▸ Größe ▸ 14 | `text.size.14` | ✅ Doc | Text |
| Text ▸ Größe ▸ 18 | `text.size.18` | ✅ Doc | Text |
| Text ▸ Größe ▸ 24 | `text.size.24` | ✅ Doc | Text |
| Text ▸ Größe ▸ 36 | `text.size.36` | ✅ Doc | Text |
| Text ▸ Größe ▸ 48 | `text.size.48` | ✅ Doc | Text |
| Text ▸ Größe ▸ 72 | `text.size.72` | ✅ Doc | Text |
| Text ▸ Größe ▸ 96 | `text.size.96` | ✅ Doc | Text |
| Text ▸ Größe ▸ 120 | `text.size.120` | ✅ Doc | Text |
| Text ▸ Stil ▸ Standard | `text.style.plain` | ✅ Doc | Text |
| Text ▸ Stil ▸ Faux Fett | `text.style.fauxBold` | ✅ Doc | Text |
| Text ▸ Stil ▸ Faux Kursiv | `text.style.fauxItalic` | ✅ Doc | Text |
| Text ▸ Stil ▸ Unterstrichen | `text.style.underline` | ✅ Doc | Text |
| Text ▸ Ausrichten ▸ Links | `text.align.left` | ✅ Doc | Text |
| Text ▸ Ausrichten ▸ Horizontal zentrieren | `text.align.centeredHorizontally` | ✅ Doc | Text |
| Text ▸ Ausrichten ▸ Rechts | `text.align.right` | ✅ Doc | Text |
| Text ▸ Ausrichten ▸ Blocksatz | `text.align.justified` | ✅ Doc | Text |
| Text ▸ Ausrichten ▸ Oben | `text.align.top` | ✅ Doc | Text |
| Text ▸ Ausrichten ▸ Vertikal zentrieren | `text.align.centeredVertically` | ✅ Doc | Text |
| Text ▸ Ausrichten ▸ Unten | `text.align.bottom` | ✅ Doc | Text |
| Text ▸ Ausrichten ▸ Vertikaler Blocksatz | `text.align.justifiedVertically` | ✅ Doc | Text |
| Text ▸ In Pfad einfügen | `text.attachInPath` | ✅ Doc | Text + Ellipse |
| Text ▸ An Pfad anfügen | `text.attachToPath` | ✅ Doc | Text + Ellipse |
| Text ▸ Von Pfad lösen | `text.detachFromPath` | ✅ Doc | Text auf Pfad |
| Text ▸ Richtung ▸ Um Pfad drehen | `text.orientation.rotateAroundPath` | ✅ Doc | Text |
| Text ▸ Richtung ▸ Vertikal | `text.orientation.vertical` | ✅ Doc | Text |
| Text ▸ Richtung ▸ Vertikal neigen | `text.orientation.skewVertical` | ✅ Doc | Text |
| Text ▸ Richtung ▸ Horizontal neigen | `text.orientation.skewHorizontal` | ✅ Doc | Text |
| Text ▸ Richtung umkehren | `text.reverseDirection` | ✅ Doc | Text |
| Text ▸ In Pfade konvertieren | `text.convertToPaths` | ✅ Doc | Text |
| Text ▸ Rechtschreibprüfung... | `text.checkSpelling` | ✅ Dialog | ohne Auswahl: Meldung „Keine Fehler gefunden. kein Wörterbuch gefunden: <sprache>.aff/.dic über „Rechtschreibprüfung einrichten“ angeben“ |
| Text ▸ Rechtschreibprüfung einrichten... | `text.spellingSetup` | ✅ Dialog | ohne Auswahl |
| Befehle ▸ Gespeicherte Befehle verwalten... | `commands.manageSavedCommands` | ✅ Dialog | ohne Auswahl |
| Befehle ▸ Erweiterungen verwalten... | `commands.manageExtensions` | ✅ Dialog | ohne Auswahl: Meldung „Es sind keine Erweiterungen installiert.“ |
| Befehle ▸ Skript ausführen... | `commands.runScript` | ✅ Doc | ohne Auswahl |
| Befehle ▸ KI-Modelle verwalten... | `ai.manageModels` | ✅ Dialog | ohne Auswahl |
| Befehle ▸ Testbefehl | `commands.runScript {"path":"C:\\Users\\Administrator\\AppData\\Roaming\\Firepixel\\Commands\\Testbefehl.jsf"}` | ✅ Doc | ohne Auswahl |
| Befehle ▸ Testbefehl2 | `commands.runScript {"path":"C:\\Users\\Administrator\\AppData\\Roaming\\Firepixel\\Commands\\Testbefehl2.jsf"}` | ✅ Ui | ohne Auswahl: Meldung „Skript: ReferenceError: select is not defined“ |
| Befehle ▸ Warnmeldungen zurücksetzen | `ui.resetWarnings {}` | ✅ Ui | ohne Auswahl: Meldung „Warnmeldungen zurückgesetzt“ |
| Befehle ▸ KI-Steuerkanal (MCP) | `ui.mcpToggle {}` | ✅ Ui | ohne Auswahl: Meldung „KI-Steuerkanal wird gestartet (Adresse und Token in %LOCALAPPDATA%\Firepixel\mcp-bridge.json)“ |
| Filter ▸ Filter wiederholen | `filter.repeat` | ✅ Doc | nach Filter |
| Filter ▸ Andere ▸ Kanten finden | `filter.other.findEdges` | ✅ Doc | Rechteck |
| Filter ▸ Andere ▸ In Alpha konvertieren | `filter.other.convertToAlpha` | ✅ Doc | Rechteck |
| Filter ▸ Farbe anpassen ▸ Auto-Tonwertkorrektur | `filter.adjustColor.autoLevels` | ✅ Doc | Rechteck |
| Filter ▸ Farbe anpassen ▸ Farbfüllung... | `filter.adjustColor.colorFill` | ✅ Doc | Rechteck |
| Filter ▸ Farbe anpassen ▸ Farbton/Sättigung... | `filter.adjustColor.hueSaturation` | ✅ Dialog | Rechteck |
| Filter ▸ Farbe anpassen ▸ Helligkeit/Kontrast... | `filter.adjustColor.brightnessContrast` | ✅ Dialog | Rechteck |
| Filter ▸ Farbe anpassen ▸ Kurven... | `filter.adjustColor.curves` | ✅ Dialog | Rechteck |
| Filter ▸ Farbe anpassen ▸ Tonwertkorrektur... | `filter.adjustColor.levels` | ✅ Dialog | Rechteck |
| Filter ▸ Farbe anpassen ▸ Umkehren | `filter.adjustColor.invert` | ✅ Doc | Rechteck |
| Filter ▸ Rauschen ▸ Rauschen hinzufügen... | `filter.noise.addNoise` | ✅ Dialog | Rechteck |
| Filter ▸ Scharfzeichnen ▸ Scharfzeichnen | `filter.sharpen.sharpen` | ✅ Doc | Rechteck |
| Filter ▸ Scharfzeichnen ▸ Stark scharfzeichnen | `filter.sharpen.sharpenMore` | ✅ Doc | Rechteck |
| Filter ▸ Scharfzeichnen ▸ Unscharf maskieren... | `filter.sharpen.unsharpMask` | ✅ Dialog | Rechteck |
| Filter ▸ Weichzeichnen ▸ Bewegungsunschärfe... | `filter.blur.motion` | ✅ Dialog | Rechteck |
| Filter ▸ Weichzeichnen ▸ Gaußscher Weichzeichner... | `filter.blur.gaussian` | ✅ Dialog | Rechteck |
| Filter ▸ Weichzeichnen ▸ Radiales Weichzeichnen... | `filter.blur.radial` | ✅ Dialog | Rechteck |
| Filter ▸ Weichzeichnen ▸ Stark weichzeichnen | `filter.blur.blurMore` | ✅ Doc | Rechteck |
| Filter ▸ Weichzeichnen ▸ Weichzeichnen | `filter.blur.blur` | ✅ Doc | Rechteck |
| Filter ▸ Weichzeichnen ▸ Zentriertes Weichzeichnen... | `filter.blur.zoom` | ✅ Dialog | Rechteck |
| Fenster ▸ Fenster duplizieren | `window.duplicateWindow` | ✅ Ui | ohne Auswahl |
| Fenster ▸ Bedienfelder ausblenden | `window.hidePanels` | ✅ Ui | ohne Auswahl |
| Fenster ▸ Arbeitsbereichlayouts ▸ Erweiterter Modus | `LoadWorkspace("Erweiterter Modus")` | ✅ Ui | ohne Auswahl |
| Fenster ▸ Arbeitsbereichlayouts ▸ Symbolmodus | `LoadWorkspace("Symbolmodus")` | ✅ Ui | ohne Auswahl |
| Fenster ▸ Arbeitsbereichlayouts ▸ Symbolmodus mit Bedienfeldnamen | `LoadWorkspace("Symbolmodus mit Bedienfeldnamen")` | ✅ Ui | ohne Auswahl |
| Fenster ▸ Arbeitsbereichlayouts ▸ Aktuelles speichern... | `ui.saveWorkspace {}` | ✅ Dialog | ohne Auswahl |
| Fenster ▸ Arbeitsbereichlayouts ▸ Standardarbeitsbereich | `ResetWorkspace` | ✅ Ui | ohne Auswahl |
| Fenster ▸ Erweiterungen ▸ Keine Erweiterungen | `` | ✅ Ui | Hinweiseintrag (leere Laufzeitliste) |
| Fenster ▸ Werkzeugleisten ▸ Standard | `window.toolbars.main` | ✅ Ui | ohne Auswahl |
| Fenster ▸ Werkzeuge | `window.tools` | ✅ Ui | ohne Auswahl |
| Fenster ▸ Eigenschaften | `window.properties` | ✅ Ui | ohne Auswahl |
| Fenster ▸ Optimieren | `window.optimize` | ✅ Ui | ohne Auswahl |
| Fenster ▸ Ebenen | `window.layers` | ✅ Ui | ohne Auswahl |
| Fenster ▸ Allgemeine Bibliothek | `window.commonLibrary` | ✅ Ui | ohne Auswahl |
| Fenster ▸ CSS-Eigenschaften | `window.cssProperties` | ✅ Ui | ohne Auswahl |
| Fenster ▸ Seiten | `window.pages` | ✅ Ui | ohne Auswahl |
| Fenster ▸ Status | `window.states` | ✅ Ui | ohne Auswahl |
| Fenster ▸ Verlauf | `window.history` | ✅ Ui | ohne Auswahl |
| Fenster ▸ Automatische Formen | `window.autoShapes` | ✅ Ui | ohne Auswahl |
| Fenster ▸ Stile | `window.styles` | ✅ Ui | ohne Auswahl |
| Fenster ▸ Dokumentbibliothek | `window.documentLibrary` | ✅ Ui | ohne Auswahl |
| Fenster ▸ URL | `window.url` | ✅ Ui | ohne Auswahl |
| Fenster ▸ Farbmischer | `window.colorMixer` | ✅ Ui | ohne Auswahl |
| Fenster ▸ Farbfelder | `window.swatches` | ✅ Ui | ohne Auswahl |
| Fenster ▸ Info | `window.info` | ✅ Ui | ohne Auswahl |
| Fenster ▸ Verhalten | `window.behaviors` | ✅ Ui | ohne Auswahl |
| Fenster ▸ Suchen und ersetzen | `edit.findAndReplace` | ✅ Ui | ohne Auswahl |
| Fenster ▸ Ausrichten | `TogglePanel(Align)` | ✅ Ui | ohne Auswahl |
| Fenster ▸ Eigenschaften für automatische Form | `TogglePanel(AutoShapeProperties)` | ✅ Ui | ohne Auswahl |
| Fenster ▸ Sonstige ▸ Bildbearbeitung | `TogglePanel(ImageEditing)` | ✅ Ui | ohne Auswahl |
| Fenster ▸ Sonstige ▸ Farbpalette | `TogglePanel(ColorPalette)` | ✅ Ui | ohne Auswahl |
| Fenster ▸ Sonstige ▸ Sonderzeichen | `TogglePanel(SpecialCharacters)` | ✅ Ui | ohne Auswahl |
| Fenster ▸ Pfad | `TogglePanel(Path)` | ✅ Ui | ohne Auswahl |
| Fenster ▸ Symboleigenschaften | `TogglePanel(SymbolProperties)` | ✅ Ui | ohne Auswahl |
| Fenster ▸ Überlappend | `window.cascade` | ✅ Ui | ohne Auswahl |
| Fenster ▸ Anordnen | `window.tile` | ✅ Ui | ohne Auswahl |
| Fenster ▸ 1 *Unbenannt-2.png | `Activate(DocId(2))` | ✅ Ui | zweites Dokument: Dokument geöffnet/gewechselt |
| Hilfe ▸ Firepixel-Hilfe | `help.firepixelHelp` | ✅ Ui | ohne Auswahl: Befehl liefert {"path":"C:\\Users\\Administrator\\Desktop\\REA\\firepixel\\docs\\ANLEITUNG.md","section":"Erste Schritte","sections":["Firepixel – Kurzanle – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Hilfe ▸ Firepixel Support Center | `help.supportCenter` | ✅ Ui | ohne Auswahl: Befehl liefert {"path":"C:\\Users\\Administrator\\Desktop\\REA\\firepixel\\docs\\ANLEITUNG.md","section":"Fehlerbehebung","sections":["Firepixel – Kurzanle – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Hilfe ▸ Programm zur Produktverbesserung... | `help.productImprovement` | ✅ Ui | ohne Auswahl: Befehl liefert {"dataSent":false,"participate":false,"text":"Firepixel erfasst und sendet keine Nutzungsdaten."} – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Hilfe ▸ Firepixel Exchange | `help.firepixelExchange` | ✅ Ui | ohne Auswahl: Befehl liefert {"extensions":[],"folder":"C:\\Users\\Administrator\\AppData\\Roaming\\Firepixel\\Extensions"} – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Hilfe ▸ Erweiterungen verwalten... | `help.manageExtensions` | ✅ Dialog | ohne Auswahl: Meldung „Es sind keine Erweiterungen installiert.“ |
| Hilfe ▸ Onlineforen | `help.onlineForums` | ✅ Ui | ohne Auswahl: Befehl liefert {"path":"C:\\Users\\Administrator\\Desktop\\REA\\firepixel\\docs\\ANLEITUNG.md","section":"Fehlerbehebung","sections":["Firepixel – Kurzanle – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Hilfe ▸ Über Firepixel... | `help.about` | ✅ Dialog | ohne Auswahl |
