# Hauptmenü – automatischer Gesamttest

Erzeugt von `crates\fp-ui\tests\gesamt_menu.rs` (`cargo test -p fp-ui --test gesamt_menu`). Nicht von Hand ändern.

**345/349 Einträge ✅**

| Eintrag | Befehl | Ergebnis | Hinweis |
|---|---|---|---|
| Datei ▸ Neu | `file.new` | ✅ Dialog | ohne Auswahl |
| Datei ▸ Neu von Vorlage... | `file.newFromTemplate` | ✅ Ui | ohne Auswahl: Dokument geöffnet/gewechselt |
| Datei ▸ Öffnen... | `file.open` | ✅ Dialog | ohne Auswahl: Meldung „„C:\Users\ADMINI~1\AppData\Local\Temp\firepixel-gesamttest\2984-65\browser.exe“ konnte nicht geöffnet werden: FW-PNG: keine PNG-Signatur“ |
| Datei ▸ Im Dateibrowser durchsuchen... | `file.browseInBridge` | ✅ Ui | ohne Auswahl: Befehl liefert {"files":[{"bytes":2132,"modified":1791871328,"name":"vorlage.fw.png","path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttes – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Datei ▸ Zuletzt geöffnete Dateien ▸ 1 C:\Users\ADMINI~1\AppData\Local\Temp\firepixel-gesamttest\2984-31\vorlage.fw.png | `OpenPath("C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-31\\vorlage.fw.png")` | ✅ Ui | ohne Auswahl: Dokument geöffnet/gewechselt |
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
| Datei ▸ Als E-Mail senden ▸ PNG-Datei für Firepixel... | `file.sendToEMail.firepixelPng` | ✅ Ui | ohne Auswahl: Befehl liefert {"attachment":"Unbenannt-2.fw.png","bytes":13887,"eml":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\Firepixel\\Email\\Unbenannt-2.eml"} – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Datei ▸ Als E-Mail senden ▸ Komprimierte JPG-Datei... | `file.sendToEMail.jpegCompressed` | ✅ Ui | ohne Auswahl: Befehl liefert {"attachment":"Unbenannt-2.jpg","bytes":18199,"eml":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\Firepixel\\Email\\Unbenannt-2.eml"} – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Datei ▸ Als E-Mail senden ▸ Exporteinstellungen verwenden... | `file.sendToEMail.useExportSettings` | ✅ Ui | ohne Auswahl: Befehl liefert {"attachment":"Unbenannt-2.gif","bytes":7789,"eml":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\Firepixel\\Email\\Unbenannt-2.eml"} – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Datei ▸ HTML aktualisieren... | `file.updateHtml` | ❌ Error("ungültige Parameter für „file.updateHtml“: C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-3040\\vorlage.fw.png ist keine HTML-Seite (weder </body> noch </html> gefunden). Die Datei bleibt unverändert.") | ohne Auswahl: ungültige Parameter für „file.updateHtml“: C:\Users\ADMINI~1\AppData\Local\Temp\firepixel-gesamttest\2984-3040\vorlage.fw.png ist keine HTML-Seite (weder </body> noch </html> gefunden). Die Datei bleibt unverändert. |
| Datei ▸ Datei-Info... | `file.fileInfo` | ✅ Dialog | ohne Auswahl |
| Datei ▸ Exportassistent... | `file.exportWizard` | ✅ Dialog | ohne Auswahl |
| Datei ▸ Batch-Verarbeitung... | `file.batchProcess` | ❌ Nothing("") | ohne Auswahl: keine erkennbare Wirkung |
| Datei ▸ Vorschau im Browser ▸ Vorschau in Standardbrowser | `file.previewInBrowser.previewInPrimaryBrowser` | ✅ Ui | ohne Auswahl: Befehl liefert {"files":["C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\Firepixel\\Preview\\doc2\\Unbenannt-2_r1_c1.gif","C:\\Users\\ADMINI~1\\AppData\\Local\ – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Datei ▸ Vorschau im Browser ▸ Vorschau in | `file.previewInBrowser.previewInSecondaryBrowser` | ✅ Ui | zweiter Browser: Befehl liefert {"files":["C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\Firepixel\\Preview\\doc2\\Unbenannt-2_r1_c1.gif","C:\\Users\\ADMINI~1\\AppData\\Local\ – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Datei ▸ Vorschau im Browser ▸ Vorschau aller Seiten in Standardbrowser anzeigen | `file.previewInBrowser.previewAllPagesInPrimaryBrowser` | ✅ Ui | ohne Auswahl: Befehl liefert {"files":["C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\Firepixel\\Preview\\doc2\\Unbenannt-2_s1_r1_c1.gif","C:\\Users\\ADMINI~1\\AppData\\Loc – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Datei ▸ Vorschau im Browser ▸ Primärbrowser festlegen... | `file.previewInBrowser.setPrimaryBrowser` | ✅ Ui | ohne Auswahl |
| Datei ▸ Vorschau im Browser ▸ Sekundärbrowser festlegen... | `file.previewInBrowser.setSecondaryBrowser` | ✅ Ui | ohne Auswahl |
| Datei ▸ Seite einrichten... | `file.pageSetup` | ✅ Dialog | ohne Auswahl |
| Datei ▸ Drucken... | `file.print` | ✅ Dialog | ohne Auswahl |
| Datei ▸ HTML-Einrichtung... | `file.htmlSetup` | ✅ Dialog | ohne Auswahl |
| Datei ▸ Beenden | `file.exit` | ✅ Dialog | ohne Auswahl |
| Bearbeiten ▸ Rückgängig: Rechteck | `edit.undo` | ✅ Ui | Pixelauswahl |
| Bearbeiten ▸ Wiederholen: Auswählen | `edit.redo` | ✅ Doc | nach Rückgängig |
| Bearbeiten ▸ Objekt einfügen ▸ Neue Schaltfläche... | `edit.insert.newButton` | ✅ Dialog | ohne Auswahl |
| Bearbeiten ▸ Objekt einfügen ▸ Neues Symbol... | `edit.insert.newSymbol` | ✅ Dialog | ohne Auswahl |
| Bearbeiten ▸ Objekt einfügen ▸ Hotspot | `edit.insert.hotspot` | ✅ Dialog | ohne Auswahl |
| Bearbeiten ▸ Objekt einfügen ▸ Rechteckiges Segment | `edit.insert.slice` | ✅ Dialog | ohne Auswahl |
| Bearbeiten ▸ Objekt einfügen ▸ Polygonsegment | `edit.insert.polygonSlice` | ✅ Dialog | ohne Auswahl |
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
| Bearbeiten ▸ HTML-Code kopieren... | `edit.copyHtmlCode` | ✅ Ui | ohne Auswahl: Datei geschrieben |
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
| Ansicht ▸ Auswahl anpassen | `view.fitSelection` | ✅ Ui | Rechteck |
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
| Ansicht ▸ Hilfslinien ▸ Hilfslinien entfernen | `view.guides.clearGuides` | ✅ Doc | Hilfslinien |
| Ansicht ▸ Intelligente Hilfslinien ▸ Intelligente Hilfslinien anzeigen | `view.smartGuides.showSmartGuides` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Intelligente Hilfslinien ▸ An intelligenten Hilfslinien ausrichten | `view.smartGuides.snapToSmartGuides` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Hilfslinien für 9-Slice-Skalierung ▸ Hilfslinien sperren | `view.nineSliceScalingGuides.lockGuides` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Segment-Hilfslinien | `view.sliceGuides` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Segmentüberlagerung | `view.sliceOverlay` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Ränder | `view.edges` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Design ▸ Normal | `SetTheme(Normal)` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Design ▸ Dunkel | `SetTheme(Dark)` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Design ▸ OLED | `SetTheme(Oled)` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Sprache ▸ Deutsch | `ui.setLanguage {"code":"de"}` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Sprache ▸ English | `ui.setLanguage {"code":"en"}` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Sprache ▸ Testsprache (qps) | `ui.setLanguage {"code":"qps"}` | ✅ Ui | ohne Auswahl |
| Ansicht ▸ Sprache ▸ Sprachordner öffnen... | `ui.openLanguageFolder {}` | ✅ Ui | ohne Auswahl: Meldung „[!! Weitere Sprachen: .ftl-Dateien in C:\Users\ADMINI~1\AppData\Local\Temp\firepixel-gesamttest\2984-profile\Roaming\Firepixel\Languages ····················································!!]“ |
| Auswählen ▸ Alles auswählen | `select.all` | ✅ Doc | ohne Auswahl |
| Auswählen ▸ Auswahl aufheben | `select.none` | ✅ Doc | Rechteck |
| Auswählen ▸ Gesamtauswahl treffen | `select.superselect` | ✅ Doc | Rechteck + Ellipse |
| Auswählen ▸ Teilauswahl treffen | `select.subselect` | ✅ Doc | drei Objekte |
| Auswählen ▸ Ähnliches auswählen | `select.similar` | ✅ Dialog | ohne Auswahl |
| Auswählen ▸ Motiv auswählen | `select.aiSubject` | ✅ Ui | ohne Auswahl |
| Auswählen ▸ Kante verfeinern... | `select.aiRefineEdge` | ✅ Ui | Pixelauswahl: Meldung „[!! [!! Kante verfeinern ······!!]: fertig ··············!!]“ |
| Auswählen ▸ Auswahl umkehren | `select.inverse` | ✅ Doc | ohne Auswahl |
| Auswählen ▸ Weiche Kante… | `select.feather` | ✅ Dialog | ohne Auswahl |
| Auswählen ▸ Auswahlrahmen erweitern... | `select.expandMarquee` | ✅ Dialog | Pixelauswahl |
| Auswählen ▸ Auswahlrahmen verkleinern... | `select.contractMarquee` | ✅ Dialog | Pixelauswahl |
| Auswählen ▸ Auswahlrahmen begrenzen... | `select.borderMarquee` | ✅ Dialog | Pixelauswahl |
| Auswählen ▸ Auswahlrahmen glätten... | `select.smoothMarquee` | ✅ Dialog | Pixelauswahl |
| Auswählen ▸ Auswahlrahmen in Pfad konvertieren | `select.convertMarqueeToPath` | ✅ Doc | Pixelauswahl |
| Auswählen ▸ Bitmap-Auswahl speichern... | `select.saveBitmapSelection` | ✅ Dialog | Pixelauswahl |
| Auswählen ▸ Bitmap-Auswahl wiederherstellen... | `select.restoreBitmapSelection` | ✅ Dialog | gespeicherte Bitmap-Auswahl |
| Auswählen ▸ Bitmap-Auswahl löschen... | `select.deleteBitmapSelection` | ✅ Dialog | gespeicherte Bitmap-Auswahl |
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
| Modifizieren ▸ Symbol ▸ Instanzen tweenen... | `modify.symbol.tweenInstances` | ✅ Dialog | ohne Auswahl |
| Modifizieren ▸ Symbol ▸ Teilen | `modify.symbol.breakApart` | ✅ Dialog | ohne Auswahl |
| Modifizieren ▸ Symbol ▸ Symbol austauschen... | `modify.symbol.swapSymbol` | ✅ Dialog | Grafiksymbol |
| Modifizieren ▸ Popupmenü ▸ Popupmenü hinzufügen... | `modify.popUpMenu.addPopUpMenu` | ✅ Dialog | ohne Auswahl |
| Modifizieren ▸ Popupmenü ▸ Popupmenü bearbeiten... | `modify.popUpMenu.editPopUpMenu` | ✅ Dialog | Segment mit Pop-up-Menü |
| Modifizieren ▸ Popupmenü ▸ Popupmenü löschen | `modify.popUpMenu.deletePopUpMenu` | ✅ Doc | Segment mit Pop-up-Menü |
| Modifizieren ▸ Maske ▸ Alles einblenden | `modify.mask.revealAll` | ✅ Doc | Rechteck |
| Modifizieren ▸ Maske ▸ Alles ausblenden | `modify.mask.hideAll` | ✅ Doc | Rechteck |
| Modifizieren ▸ Maske ▸ Als Maske einfügen | `edit.pasteAsMask` | ✅ Doc | Rechteck |
| Modifizieren ▸ Maske ▸ Als Maske gruppieren | `modify.mask.groupAsMask` | ✅ Doc | Rechteck + Ellipse |
| Modifizieren ▸ Maske ▸ Auswahl einblenden | `modify.mask.revealSelection` | ✅ Doc | Bitmap + Pixelauswahl |
| Modifizieren ▸ Maske ▸ Auswahl ausblenden | `modify.mask.hideSelection` | ✅ Doc | Bitmap + Pixelauswahl |
| Modifizieren ▸ Maske ▸ Maske deaktivieren | `modify.mask.disableMask` | ✅ Doc | Maskengruppe |
| Modifizieren ▸ Maske ▸ Maske löschen | `modify.mask.deleteMask` | ✅ Doc | Maskengruppe |
| Modifizieren ▸ Selektives JPEG ▸ Auswahl als JPEG-Maske speichern | `modify.selectiveJpeg.saveSelectionAsJpegMask` | ✅ Doc | Pixelauswahl |
| Modifizieren ▸ Selektives JPEG ▸ JPEG-Maske als Auswahl wiederherstellen | `modify.selectiveJpeg.restoreJpegMaskAsSelection` | ✅ Doc | ohne Auswahl |
| Modifizieren ▸ Selektives JPEG ▸ Einstellungen... | `modify.selectiveJpeg.settings` | ✅ Dialog | ohne Auswahl |
| Modifizieren ▸ Selektives JPEG ▸ JPEG-Maske entfernen | `modify.selectiveJpeg.removeJpegMask` | ✅ Doc | ohne Auswahl |
| Modifizieren ▸ Auswahl sperren | `modify.lockSelection` | ✅ Doc | Rechteck |
| Modifizieren ▸ Auswahl auf Hintergrund reduzieren | `modify.flattenSelection` | ✅ Doc | Rechteck |
| Modifizieren ▸ Mit darunter liegender auf eine Ebene reduzieren | `modify.mergeDown` | ✅ Doc | Rechteck über Bitmap |
| Modifizieren ▸ Ebenen als Hintergrund | `modify.flattenLayers` | ✅ Doc | zwei Ebenen |
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
| Modifizieren ▸ Ausrichten ▸ Links | `modify.align.left` | ✅ Doc | Rechteck + Ellipse |
| Modifizieren ▸ Ausrichten ▸ Vertikal zentrieren | `modify.align.centerVertical` | ✅ Doc | Rechteck + Ellipse |
| Modifizieren ▸ Ausrichten ▸ Rechts | `modify.align.right` | ✅ Doc | Rechteck + Ellipse |
| Modifizieren ▸ Ausrichten ▸ Oben | `modify.align.top` | ✅ Doc | Rechteck + Ellipse |
| Modifizieren ▸ Ausrichten ▸ Horizontal zentrieren | `modify.align.centerHorizontal` | ✅ Doc | Rechteck + Ellipse |
| Modifizieren ▸ Ausrichten ▸ Unten | `modify.align.bottom` | ✅ Doc | Rechteck + Ellipse |
| Modifizieren ▸ Ausrichten ▸ Breiten verteilen | `modify.align.distributeWidths` | ✅ Doc | drei Objekte |
| Modifizieren ▸ Ausrichten ▸ Höhen verteilen | `modify.align.distributeHeights` | ✅ Doc | drei Objekte |
| Modifizieren ▸ Pfad in Auswahlrahmen konvertieren... | `modify.convertPathToMarquee` | ✅ Dialog | Rechteck |
| Modifizieren ▸ Pfade kombinieren ▸ Verbinden | `modify.combinePaths.join` | ✅ Doc | Rechteck + Ellipse |
| Modifizieren ▸ Pfade kombinieren ▸ Teilen | `modify.combinePaths.split` | ✅ Doc | zusammengesetzter Pfad |
| Modifizieren ▸ Pfade kombinieren ▸ Vereinigen | `modify.combinePaths.union` | ✅ Dialog | ohne Auswahl |
| Modifizieren ▸ Pfade kombinieren ▸ Überschneiden | `modify.combinePaths.intersect` | ✅ Dialog | ohne Auswahl |
| Modifizieren ▸ Pfade kombinieren ▸ Öffnen | `modify.combinePaths.punch` | ✅ Dialog | ohne Auswahl |
| Modifizieren ▸ Pfade kombinieren ▸ Zuschneiden | `modify.combinePaths.crop` | ✅ Dialog | ohne Auswahl |
| Modifizieren ▸ Pfad ändern ▸ Vereinfachen... | `modify.alterPath.simplify` | ✅ Dialog | ohne Auswahl |
| Modifizieren ▸ Pfad ändern ▸ Strich erweitern... | `modify.alterPath.expandStroke` | ✅ Dialog | ohne Auswahl |
| Modifizieren ▸ Pfad ändern ▸ Pfad einlassen... | `modify.alterPath.insetPath` | ✅ Dialog | ohne Auswahl |
| Modifizieren ▸ Pfad ändern ▸ Präzise Füllung | `modify.alterPath.hardFill` | ✅ Doc | Rechteck + Ellipse |
| Modifizieren ▸ Pfad ändern ▸ Anti-Alias-Füllung | `modify.alterPath.antiAliasFill` | ✅ Doc | Rechteck + Ellipse |
| Modifizieren ▸ Pfad ändern ▸ Weiche-Kante-Füllung | `modify.alterPath.featherFill` | ✅ Doc | Rechteck + Ellipse |
| Modifizieren ▸ Bitmap nachzeichnen... | `modify.traceBitmap` | ✅ Dialog | Bitmap |
| Modifizieren ▸ An Pixeln ausrichten | `modify.snapToPixel` | ✅ Doc | Rechteck |
| Modifizieren ▸ Gruppieren | `modify.group` | ✅ Doc | Rechteck |
| Modifizieren ▸ Gruppierung aufheben | `modify.ungroup` | ✅ Doc | Rechteck |
| Text ▸ Schrift ▸ Arial ▸ Regular | `text.font {"font":"ArialMT"}` | ✅ Ui | ohne Auswahl |
| Text ▸ Schrift ▸ Arial ▸ Bold | `text.font {"font":"Arial-BoldMT"}` | ✅ Ui | ohne Auswahl |
| Text ▸ Schrift ▸ Arial ▸ Bold Italic | `text.font {"font":"Arial-BoldItalicMT"}` | ✅ Ui | ohne Auswahl |
| Text ▸ Größe ▸ Andere... | `text.size.other` | ✅ Dialog | ohne Auswahl |
| Text ▸ Größe ▸ Kleiner | `text.size.smaller` | ✅ Doc | Text |
| Text ▸ Größe ▸ Größer | `text.size.larger` | ✅ Doc | Text |
| Text ▸ Größe ▸ 8 | `text.size.8` | ✅ Ui | ohne Auswahl |
| Text ▸ Größe ▸ 9 | `text.size.9` | ✅ Ui | ohne Auswahl |
| Text ▸ Größe ▸ 10 | `text.size.10` | ✅ Ui | ohne Auswahl |
| Text ▸ Größe ▸ 12 | `text.size.12` | ✅ Ui | ohne Auswahl: keine erkennbare Wirkung |
| Text ▸ Größe ▸ 14 | `text.size.14` | ✅ Ui | ohne Auswahl |
| Text ▸ Größe ▸ 18 | `text.size.18` | ✅ Ui | ohne Auswahl |
| Text ▸ Größe ▸ 24 | `text.size.24` | ✅ Ui | ohne Auswahl |
| Text ▸ Größe ▸ 36 | `text.size.36` | ✅ Ui | ohne Auswahl |
| Text ▸ Größe ▸ 48 | `text.size.48` | ✅ Ui | ohne Auswahl |
| Text ▸ Größe ▸ 72 | `text.size.72` | ✅ Ui | ohne Auswahl |
| Text ▸ Größe ▸ 96 | `text.size.96` | ✅ Ui | ohne Auswahl |
| Text ▸ Größe ▸ 120 | `text.size.120` | ✅ Ui | ohne Auswahl |
| Text ▸ Stil ▸ Standard | `text.style.plain` | ✅ Ui | ohne Auswahl: keine erkennbare Wirkung |
| Text ▸ Stil ▸ Faux Fett | `text.style.fauxBold` | ✅ Ui | ohne Auswahl |
| Text ▸ Stil ▸ Faux Kursiv | `text.style.fauxItalic` | ✅ Ui | ohne Auswahl |
| Text ▸ Stil ▸ Unterstrichen | `text.style.underline` | ✅ Ui | ohne Auswahl |
| Text ▸ Ausrichten ▸ Links | `text.align.left` | ✅ Ui | ohne Auswahl: keine erkennbare Wirkung |
| Text ▸ Ausrichten ▸ Horizontal zentrieren | `text.align.centeredHorizontally` | ✅ Ui | ohne Auswahl |
| Text ▸ Ausrichten ▸ Rechts | `text.align.right` | ✅ Ui | ohne Auswahl |
| Text ▸ Ausrichten ▸ Blocksatz | `text.align.justified` | ✅ Ui | ohne Auswahl |
| Text ▸ Ausrichten ▸ Oben | `text.align.top` | ✅ Ui | ohne Auswahl |
| Text ▸ Ausrichten ▸ Vertikal zentrieren | `text.align.centeredVertically` | ✅ Ui | ohne Auswahl |
| Text ▸ Ausrichten ▸ Unten | `text.align.bottom` | ✅ Ui | ohne Auswahl |
| Text ▸ Ausrichten ▸ Vertikaler Blocksatz | `text.align.justifiedVertically` | ✅ Ui | ohne Auswahl |
| Text ▸ In Pfad einfügen | `text.attachInPath` | ✅ Doc | Text + Ellipse |
| Text ▸ An Pfad anfügen | `text.attachToPath` | ✅ Doc | Text + Ellipse |
| Text ▸ Von Pfad lösen | `text.detachFromPath` | ✅ Doc | Text auf Pfad |
| Text ▸ Richtung ▸ Um Pfad drehen | `text.orientation.rotateAroundPath` | ✅ Doc | Text auf Pfad |
| Text ▸ Richtung ▸ Vertikal | `text.orientation.vertical` | ✅ Doc | Text auf Pfad |
| Text ▸ Richtung ▸ Vertikal neigen | `text.orientation.skewVertical` | ✅ Doc | Text auf Pfad |
| Text ▸ Richtung ▸ Horizontal neigen | `text.orientation.skewHorizontal` | ✅ Doc | Text auf Pfad |
| Text ▸ Richtung umkehren | `text.reverseDirection` | ✅ Doc | Text auf Pfad |
| Text ▸ In Pfade konvertieren | `text.convertToPaths` | ✅ Doc | Text |
| Text ▸ Rechtschreibprüfung... | `text.checkSpelling` | ✅ Dialog | ohne Auswahl: Meldung „[!! Keine Fehler gefunden. [!! kein Wörterbuch gefunden: <sprache>.aff/.dic über „Rechtschreibprüfung einrichten“ angeben ··································!!] ··························································!!]“ |
| Text ▸ Rechtschreibprüfung einrichten... | `text.spellingSetup` | ✅ Dialog | ohne Auswahl |
| Befehle ▸ Gespeicherte Befehle verwalten... | `commands.manageSavedCommands` | ✅ Dialog | ohne Auswahl: Meldung „[!! Befehle werden dem Verlauf-Bedienfeld entnommen. Zurzeit sind dort keine gespeicherten Befehle vorhanden. ······································!!]“ |
| Befehle ▸ Erweiterungen verwalten... | `commands.manageExtensions` | ✅ Dialog | ohne Auswahl: Meldung „[!! Es sind keine Erweiterungen installiert. ···············!!]“ |
| Befehle ▸ Skript ausführen... | `commands.runScript` | ✅ Doc | ohne Auswahl |
| Befehle ▸ KI-Modelle verwalten... | `ai.manageModels` | ✅ Dialog | ohne Auswahl |
| Befehle ▸ Auswahl ▸ Früheres Vektorrendern verwenden | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Auswahl\\Früheres Vektorrendern verwenden.jsf"}` | ✅ Doc | Rechteck |
| Befehle ▸ Auswahl ▸ Verbessertes Vektorrendern verwenden | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Auswahl\\Verbessertes Vektorrendern verwenden.jsf"}` | ✅ Doc | Rechteck |
| Befehle ▸ Batch-Befehle ▸ 180°-Drehung | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Batch-Befehle\\180°-Drehung.jsf"}` | ✅ Doc | ohne Auswahl |
| Befehle ▸ Batch-Befehle ▸ 270°-Drehung | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Batch-Befehle\\270°-Drehung.jsf"}` | ✅ Doc | ohne Auswahl |
| Befehle ▸ Batch-Befehle ▸ 90°-Drehung | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Batch-Befehle\\90°-Drehung.jsf"}` | ✅ Doc | ohne Auswahl |
| Befehle ▸ Batch-Befehle ▸ Alles auswählen | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Batch-Befehle\\Alles auswählen.jsf"}` | ✅ Doc | ohne Auswahl |
| Befehle ▸ Batch-Befehle ▸ Auswahl scharfzeichnen | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Batch-Befehle\\Auswahl scharfzeichnen.jsf"}` | ✅ Doc | Rechteck |
| Befehle ▸ Batch-Befehle ▸ Auswahl um 180 Grad drehen | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Batch-Befehle\\Auswahl um 180 Grad drehen.jsf"}` | ✅ Doc | Rechteck |
| Befehle ▸ Batch-Befehle ▸ Auswahl um 270 Grad drehen | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Batch-Befehle\\Auswahl um 270 Grad drehen.jsf"}` | ✅ Doc | Rechteck |
| Befehle ▸ Batch-Befehle ▸ Auswahl um 90 Grad drehen | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Batch-Befehle\\Auswahl um 90 Grad drehen.jsf"}` | ✅ Doc | Rechteck |
| Befehle ▸ Batch-Befehle ▸ Auswahl weichzeichnen | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Batch-Befehle\\Auswahl weichzeichnen.jsf"}` | ✅ Doc | Rechteck |
| Befehle ▸ Batch-Befehle ▸ Auswahlfarbe umkehren | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Batch-Befehle\\Auswahlfarbe umkehren.jsf"}` | ✅ Doc | Rechteck |
| Befehle ▸ Batch-Befehle ▸ Auto-Tonwertauswahl | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Batch-Befehle\\Auto-Tonwertauswahl.jsf"}` | ✅ Doc | Rechteck |
| Befehle ▸ Bildschirmfoto | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Bildschirmfoto.jsf"}` | ✅ Dialog | ohne Auswahl: Meldung „[!! Bildschirmfoto: Firepixel kann den Bildschirm nicht selbst aufnehmen. ·························!!]  [!! Nehmen Sie das Bild mit dem System auf (Windows: Win+Umschalt+S) und fügen Sie es mit Datei ▸ Neu und Bearbeiten ▸ Einfügen (Strg+V) ein. ··············································!!]“ |
| Befehle ▸ Demo - aktuelles Dokument | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Demo - aktuelles Dokument.jsf"}` | ❌ Nothing("") | ohne Auswahl: keine erkennbare Wirkung |
| Befehle ▸ Dokument ▸ Andere Ebenen ausblenden | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Dokument\\Andere Ebenen ausblenden.jsf"}` | ✅ Doc | zwei Ebenen |
| Befehle ▸ Dokument ▸ Andere Ebenen sperren | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Dokument\\Andere Ebenen sperren.jsf"}` | ✅ Doc | zwei Ebenen |
| Befehle ▸ Dokument ▸ Status umkehren | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Dokument\\Status umkehren.jsf"}` | ✅ Doc | ohne Auswahl |
| Befehle ▸ Dokument ▸ Verteilung auf Ebenen | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Dokument\\Verteilung auf Ebenen.jsf"}` | ✅ Doc | Rechteck |
| Befehle ▸ Größe ausgewählter Objekte ändern | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Größe ausgewählter Objekte ändern.jsf"}` | ✅ Doc | Rechteck |
| Befehle ▸ Kreativ ▸ Autom. Vektormaske | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Kreativ\\Autom. Vektormaske.jsf"}` | ✅ Dialog | ohne Auswahl: Meldung „[!! Autom. Vektormaske: Wählen Sie zuerst ein oder mehrere Objekte aus. ························!!]“ |
| Befehle ▸ Kreativ ▸ Bilderrahmen hinzufügen | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Kreativ\\Bilderrahmen hinzufügen.jsf"}` | ✅ Doc | ohne Auswahl |
| Befehle ▸ Kreativ ▸ Drehen und Ausblenden | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Kreativ\\Drehen und Ausblenden.jsf"}` | ✅ Doc | Rechteck |
| Befehle ▸ Kreativ ▸ Graustufenkonvertierung | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Kreativ\\Graustufenkonvertierung.jsf"}` | ✅ Doc | Rechteck |
| Befehle ▸ Kreativ ▸ Pfeilspitzen hinzufügen | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Kreativ\\Pfeilspitzen hinzufügen.jsf"}` | ✅ Doc | Pfad |
| Befehle ▸ Kreativ ▸ Schatten hinzufügen | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Kreativ\\Schatten hinzufügen.jsf"}` | ✅ Doc | Rechteck |
| Befehle ▸ Kreativ ▸ Sepia-Konvertierung | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Kreativ\\Sepia-Konvertierung.jsf"}` | ✅ Doc | Rechteck |
| Befehle ▸ Symbolskript erstellen | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Symbolskript erstellen.jsf"}` | ✅ Dialog | ohne Auswahl: Meldung „[!! Symbolskript erstellen: Wählen Sie zuerst eine Instanz eines Grafiksymbols aus. ····························!!]“ |
| Befehle ▸ Text ▸ Als Großbuchstaben | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Text\\Als Großbuchstaben.jsf"}` | ✅ Doc | Text |
| Befehle ▸ Text ▸ Als Kleinbuchstaben | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Text\\Als Kleinbuchstaben.jsf"}` | ✅ Doc | Text |
| Befehle ▸ Text ▸ Als Pfade wiedergeben | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Text\\Als Pfade wiedergeben.jsf"}` | ✅ Doc | Text |
| Befehle ▸ Text ▸ Als Pixel wiedergeben | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Text\\Als Pixel wiedergeben.jsf"}` | ✅ Doc | Rechteck |
| Befehle ▸ Text ▸ Breite festlegen | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Text\\Breite festlegen.jsf"}` | ✅ Doc | Text |
| Befehle ▸ Text ▸ Erster Buchstabe Großbuchstabe | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Text\\Erster Buchstabe Großbuchstabe.jsf"}` | ✅ Doc | Text |
| Befehle ▸ Text ▸ Feste Breite ein/aus | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Text\\Feste Breite ein∕aus.jsf"}` | ✅ Doc | Text |
| Befehle ▸ Text ▸ Lorem ipsum | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Text\\Lorem ipsum.jsf"}` | ✅ Doc | ohne Auswahl |
| Befehle ▸ Text ▸ Verfestigen | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Text\\Verfestigen.jsf"}` | ✅ Doc | Text |
| Befehle ▸ Warnmeldungen zurücksetzen | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Warnmeldungen zurücksetzen.jsf"}` | ✅ Ui | abgeschalteter Warndialog |
| Befehle ▸ Web ▸ ALT-Text festlegen | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Web\\ALT-Text festlegen.jsf"}` | ✅ Doc | Grafiksymbol |
| Befehle ▸ Web ▸ Gemeinsame Palette | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Web\\Gemeinsame Palette.jsf"}` | ✅ Dialog | ohne Auswahl: Meldung „[!! Gemeinsame Palette: Firepixel legt keine eigene gemeinsame Farbtabelle für mehrere Segmente an. Damit Segmente dieselben Farben verwenden, stellen Sie für jedes Segment im Bedienfeld „Optimieren“ dieselbe Farbtabelle ein. ··············································································!!]“ |
| Befehle ▸ Web ▸ Leeren ALT-Text auswählen | `commands.runScript {"path":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Commands\\Web\\Leeren ALT-Text auswählen.jsf"}` | ✅ Doc | ohne Auswahl |
| Befehle ▸ KI-Steuerkanal (MCP) | `ui.mcpToggle {}` | 🔶 Manual | öffnet externe Programme – GUI-Sitzung |
| Filter ▸ Filter wiederholen | `filter.repeat` | ✅ Doc | nach Filter |
| Filter ▸ Andere ▸ In Alphakanal konv. | `filter.other.convertToAlpha` | ✅ Doc | Rechteck |
| Filter ▸ Andere ▸ Ränder suchen | `filter.other.findEdges` | ✅ Doc | Rechteck |
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
| Filter ▸ Weichzeichnen ▸ Bewegungsunschärfe… | `filter.blur.motion` | ✅ Dialog | Rechteck |
| Filter ▸ Weichzeichnen ▸ Gaußscher Weichz. ... | `filter.blur.gaussian` | ✅ Dialog | Rechteck |
| Filter ▸ Weichzeichnen ▸ Radiales Weichz. ... | `filter.blur.radial` | ✅ Dialog | Rechteck |
| Filter ▸ Weichzeichnen ▸ Stärker weichzeichnen | `filter.blur.blurMore` | ✅ Doc | Rechteck |
| Filter ▸ Weichzeichnen ▸ Weichzeichnen | `filter.blur.blur` | ✅ Doc | Rechteck |
| Filter ▸ Weichzeichnen ▸ Zentriertes Weichz. ... | `filter.blur.zoom` | ✅ Dialog | Rechteck |
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
| Hilfe ▸ Firepixel-Hilfe | `help.firepixelHelp` | ✅ Ui | ohne Auswahl: Befehl liefert {"path":"C:\\Users\\Administrator\\Desktop\\REA\\firepixel\\docs\\MANUAL.en.md","section":null,"sections":["Firepixel – Quick Guide","Gettin – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Hilfe ▸ Firepixel Support Center | `help.supportCenter` | ✅ Ui | ohne Auswahl: Befehl liefert {"path":"C:\\Users\\Administrator\\Desktop\\REA\\firepixel\\docs\\MANUAL.en.md","section":null,"sections":["Firepixel – Quick Guide","Gettin – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Hilfe ▸ Firepixel-Programm zur Produktverbesserung... | `help.productImprovement` | ✅ Ui | ohne Auswahl: Befehl liefert {"dataSent":false,"participate":false,"text":"[!! Firepixel erfasst und sendet keine Nutzungsdaten. ··················!!]"} – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Hilfe ▸ Firepixel Exchange | `help.firepixelExchange` | ✅ Ui | ohne Auswahl: Befehl liefert {"extensions":[],"folder":"C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-gesamttest\\2984-profile\\Extensions"} – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Hilfe ▸ Erweiterungen verwalten... | `help.manageExtensions` | ✅ Dialog | ohne Auswahl: Meldung „[!! Es sind keine Erweiterungen installiert. ···············!!]“ |
| Hilfe ▸ Firepixel-Onlineforen | `help.onlineForums` | ✅ Ui | ohne Auswahl: Befehl liefert {"path":"C:\\Users\\Administrator\\Desktop\\REA\\firepixel\\docs\\MANUAL.en.md","section":null,"sections":["Firepixel – Quick Guide","Gettin – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Hilfe ▸ Über Firepixel... | `help.about` | ✅ Dialog | ohne Auswahl |
