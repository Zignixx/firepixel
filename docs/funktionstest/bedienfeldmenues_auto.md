# Bedienfeldmenüs – automatischer Gesamttest

Erzeugt von `crates\fp-ui\tests\gesamt_menu.rs` (`cargo test -p fp-ui --test gesamt_menu`). Nicht von Hand ändern.

**181/181 Einträge ✅**

| Eintrag | Befehl | Ergebnis | Hinweis |
|---|---|---|---|
| Optimieren ▸ Einstellungen speichern... | `panel.optimize.saveSettings` | ✅ Dialog | ohne Auswahl |
| Optimieren ▸ Einstellungen löschen | `panel.optimize.deleteSettings` | ✅ Dialog | ohne Auswahl: Meldung „Es gibt keine gespeicherten Einstellungen.“ |
| Optimieren ▸ Optimieren für Dateigröße... | `panel.optimize.optimizeToSize` | ✅ Dialog | ohne Auswahl |
| Optimieren ▸ Exportassistent... | `panel.optimize.exportWizard` | ✅ Dialog | ohne Auswahl |
| Optimieren ▸ Mit Zeilensprung | `panel.optimize.interlaced` | ✅ Doc | ohne Auswahl |
| Optimieren ▸ Progressives JPEG | `panel.optimize.progressive` | ✅ Doc | ohne Auswahl |
| Optimieren ▸ JPEG-Ränder scharfzeichnen | `panel.optimize.sharpenJpegEdges` | ✅ Doc | ohne Auswahl |
| Optimieren ▸ Sperrung aller Farben aufheben | `panel.optimize.unlockAll` | ✅ Doc | ohne Auswahl |
| Optimieren ▸ Nicht verwendete Farben entfernen | `panel.optimize.removeUnusedColors` | ✅ Doc | ohne Auswahl |
| Optimieren ▸ Paletteneintrag ersetzen... | `panel.optimize.editColor` | ✅ Dialog | ohne Auswahl |
| Optimieren ▸ Transparent machen | `panel.optimize.makeTransparent` | ✅ Doc | ohne Auswahl |
| Optimieren ▸ Farbfeld-Rückmeldung anzeigen | `panel.optimize.swatchFeedback` | ✅ Ui | ohne Auswahl |
| Optimieren ▸ Bearbeitung entfernen | `panel.optimize.removeEdit` | ✅ Doc | ohne Auswahl |
| Optimieren ▸ Alle Bearbeitungen entfernen | `panel.optimize.removeAllEdits` | ✅ Doc | ohne Auswahl |
| Optimieren ▸ Palette laden... | `panel.optimize.loadPalette` | ✅ Doc | ohne Auswahl |
| Optimieren ▸ Palette speichern... | `panel.optimize.savePalette` | ✅ Ui | ohne Auswahl: Datei geschrieben |
| Optimieren ▸ Hilfe | `help.firepixelHelp {"topic":"panels"}` | ✅ Ui | ohne Auswahl: Befehl liefert {"path":"C:\\Users\\Administrator\\Desktop\\REA\\firepixel\\docs\\ANLEITUNG.md","section":"Bedienfelder","sections":["Firepixel – Kurzanleit – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Optimieren ▸ Schließen | `TogglePanel(Optimize)` | ✅ Ui | ohne Auswahl |
| Optimieren ▸ Registerkartengruppe schließen | `ui.closeGroup {"panel":"Optimize"}` | ✅ Ui | ohne Auswahl |
| Verlauf ▸ Ausgewählte Schritte wiederholen | `panel.history.replaySelected` | ✅ Doc | ohne Auswahl |
| Verlauf ▸ Schritte kopieren | `panel.history.copySteps` | ✅ Ui | ohne Auswahl: Meldung „Schritte in die Zwischenablage kopiert“ |
| Verlauf ▸ Als Befehl speichern... | `panel.history.saveAsCommand` | ✅ Dialog | ohne Auswahl |
| Verlauf ▸ Verlauf löschen | `panel.history.clear` | ✅ Dialog | ohne Auswahl |
| Verlauf ▸ Hilfe | `help.firepixelHelp {"topic":"panels"}` | ✅ Ui | ohne Auswahl: Befehl liefert {"path":"C:\\Users\\Administrator\\Desktop\\REA\\firepixel\\docs\\ANLEITUNG.md","section":"Bedienfelder","sections":["Firepixel – Kurzanleit – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Verlauf ▸ Schließen | `TogglePanel(History)` | ✅ Ui | ohne Auswahl |
| Verlauf ▸ Registerkartengruppe schließen | `ui.closeGroup {"panel":"History"}` | ✅ Ui | ohne Auswahl |
| Ausrichten ▸ Hilfe | `help.firepixelHelp {"topic":"panels"}` | ✅ Ui | ohne Auswahl: Befehl liefert {"path":"C:\\Users\\Administrator\\Desktop\\REA\\firepixel\\docs\\ANLEITUNG.md","section":"Bedienfelder","sections":["Firepixel – Kurzanleit – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Ausrichten ▸ Schließen | `TogglePanel(Align)` | ✅ Ui | ohne Auswahl |
| Ausrichten ▸ Registerkartengruppe schließen | `ui.closeGroup {"panel":"Align"}` | ✅ Ui | ohne Auswahl |
| Seiten ▸ Neue Seite | `panel.pages.new` | ✅ Doc | ohne Auswahl |
| Seiten ▸ Seite duplizieren | `panel.pages.duplicate` | ✅ Doc | ohne Auswahl |
| Seiten ▸ Seite löschen | `panel.pages.delete` | ✅ Doc | ohne Auswahl |
| Seiten ▸ Seite umbenennen | `panel.pages.rename` | ✅ Dialog | ohne Auswahl |
| Seiten ▸ Mit Masterseite verknüpfen | `panel.pages.linkToMaster` | ✅ Doc | Masterseite |
| Seiten ▸ Als Masterseite festlegen | `panel.pages.setMaster` | ✅ Doc | ohne Auswahl |
| Seiten ▸ Masterseite zurücksetzen | `panel.pages.resetMaster` | ✅ Doc | Masterseite |
| Seiten ▸ Ausgewählte Seite(n) exportieren... | `panel.pages.export` | ✅ Ui | ohne Auswahl: Datei geschrieben |
| Seiten ▸ Nummerierung | `panel.pages.numbering` | ✅ Ui | ohne Auswahl |
| Seiten ▸ Optionen für Miniaturansicht... | `panel.pages.thumbnailOptions` | ✅ Dialog | ohne Auswahl |
| Seiten ▸ Hilfe | `help.firepixelHelp {"topic":"panels"}` | ✅ Ui | ohne Auswahl: Befehl liefert {"path":"C:\\Users\\Administrator\\Desktop\\REA\\firepixel\\docs\\ANLEITUNG.md","section":"Bedienfelder","sections":["Firepixel – Kurzanleit – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Seiten ▸ Schließen | `TogglePanel(Pages)` | ✅ Ui | ohne Auswahl |
| Seiten ▸ Registerkartengruppe schließen | `ui.closeGroup {"panel":"Pages"}` | ✅ Ui | ohne Auswahl |
| Status ▸ Status hinzufügen... | `panel.states.add` | ✅ Dialog | ohne Auswahl |
| Status ▸ Status duplizieren... | `panel.states.duplicate` | ✅ Dialog | ohne Auswahl |
| Status ▸ Status löschen | `panel.states.delete` | ✅ Doc | ohne Auswahl |
| Status ▸ In Status kopieren... | `panel.states.copyTo` | ✅ Dialog | Rechteck |
| Status ▸ Auf Status verteilen | `panel.states.distribute` | ✅ Doc | Rechteck + Ellipse |
| Status ▸ Autom. zuschneiden | `panel.states.autoCrop` | ✅ Doc | ohne Auswahl |
| Status ▸ Nummerierung | `panel.states.numbering` | ✅ Ui | ohne Auswahl |
| Status ▸ Auto-Differenz | `panel.states.autoDifference` | ✅ Doc | ohne Auswahl |
| Status ▸ Eigenschaften... | `panel.states.properties` | ✅ Dialog | ohne Auswahl |
| Status ▸ Hilfe | `help.firepixelHelp {"topic":"panels"}` | ✅ Ui | ohne Auswahl: Befehl liefert {"path":"C:\\Users\\Administrator\\Desktop\\REA\\firepixel\\docs\\ANLEITUNG.md","section":"Bedienfelder","sections":["Firepixel – Kurzanleit – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Status ▸ Schließen | `TogglePanel(States)` | ✅ Ui | ohne Auswahl |
| Status ▸ Registerkartengruppe schließen | `ui.closeGroup {"panel":"States"}` | ✅ Ui | ohne Auswahl |
| Ebenen ▸ Neues Bild | `panel.layers.newBitmap` | ✅ Doc | ohne Auswahl |
| Ebenen ▸ Neue Ebene... | `panel.layers.newLayer` | ✅ Dialog | ohne Auswahl |
| Ebenen ▸ Neue Unterebene... | `panel.layers.newSubLayer` | ✅ Dialog | ohne Auswahl |
| Ebenen ▸ Ebene duplizieren... | `panel.layers.duplicate` | ✅ Dialog | ohne Auswahl |
| Ebenen ▸ Ebene statusübergreifend freigeben | `panel.layers.shareAcrossStates` | ✅ Doc | ohne Auswahl |
| Ebenen ▸ Einzelebene bearbeiten | `panel.layers.singleLayerEditing` | ✅ Ui | ohne Auswahl |
| Ebenen ▸ Ebene löschen | `panel.layers.delete` | ✅ Doc | zwei Ebenen |
| Ebenen ▸ Masterseitenebene hinzufügen | `panel.layers.addMasterPageLayer` | ✅ Doc | Masterseite |
| Ebenen ▸ Ebene für Seiten freigeben... | `panel.layers.shareToPages` | ✅ Dialog | ohne Auswahl |
| Ebenen ▸ Ebene für alle Seiten freigeben | `panel.layers.shareToAllPages` | ✅ Doc | ohne Auswahl |
| Ebenen ▸ Freigegebene Ebene ausschließen | `panel.layers.excludeShared` | ✅ Doc | freigegebene Ebene |
| Ebenen ▸ Freigegebene Ebene lösen | `panel.layers.detachShared` | ✅ Doc | freigegebene Ebene |
| Ebenen ▸ Auswahl auf Hintergrund reduzieren | `panel.layers.flattenSelection` | ✅ Doc | Rechteck |
| Ebenen ▸ Mit darunter liegender auf eine Ebene reduzieren | `panel.layers.mergeDown` | ✅ Doc | Rechteck über Bitmap |
| Ebenen ▸ Alles ausblenden | `panel.layers.hideAll` | ✅ Doc | ohne Auswahl |
| Ebenen ▸ Alles anzeigen | `panel.layers.showAll` | ✅ Doc | ohne Auswahl |
| Ebenen ▸ Alles sperren | `panel.layers.lockAll` | ✅ Doc | ohne Auswahl |
| Ebenen ▸ Alle Sperrungen aufheben | `panel.layers.unlockAll` | ✅ Doc | ohne Auswahl |
| Ebenen ▸ Maske hinzufügen | `panel.layers.addMask` | ✅ Doc | Rechteck |
| Ebenen ▸ Maske bearbeiten | `panel.layers.editMask` | ✅ Doc | Maskengruppe |
| Ebenen ▸ Maske deaktivieren | `panel.layers.disableMask` | ✅ Doc | Maskengruppe |
| Ebenen ▸ Maske löschen | `panel.layers.deleteMask` | ✅ Doc | Maskengruppe |
| Ebenen ▸ Miniaturoptionen... | `panel.layers.thumbnailOptions` | ✅ Dialog | ohne Auswahl |
| Ebenen ▸ Hilfe | `help.firepixelHelp {"topic":"panels"}` | ✅ Ui | ohne Auswahl: Befehl liefert {"path":"C:\\Users\\Administrator\\Desktop\\REA\\firepixel\\docs\\ANLEITUNG.md","section":"Bedienfelder","sections":["Firepixel – Kurzanleit – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Ebenen ▸ Schließen | `TogglePanel(Layers)` | ✅ Ui | ohne Auswahl |
| Ebenen ▸ Registerkartengruppe schließen | `ui.closeGroup {"panel":"Layers"}` | ✅ Ui | ohne Auswahl |
| Stile ▸ Neuer Stil... | `panel.styles.new` | ✅ Dialog | Rechteck |
| Stile ▸ Stil duplizieren... | `panel.styles.duplicate` | ✅ Doc | ohne Auswahl |
| Stile ▸ Stile löschen | `panel.styles.delete` | ✅ Doc | ohne Auswahl |
| Stile ▸ Stil umbenennen... | `panel.styles.rename` | ✅ Dialog | ohne Auswahl |
| Stile ▸ Stil neu definieren | `panel.styles.redefine` | ✅ Doc | Rechteck |
| Stile ▸ Abweichungen löschen | `panel.styles.clearOverrides` | ✅ Doc | Rechteck |
| Stile ▸ Verknüpfung mit Stil aufheben | `panel.styles.breakLink` | ✅ Doc | Rechteck |
| Stile ▸ Nicht verwendete Stile auswählen | `panel.styles.selectUnused` | ✅ Ui | ohne Auswahl |
| Stile ▸ Stiloptionen... | `panel.styles.options` | ✅ Dialog | ohne Auswahl |
| Stile ▸ Stil laden... | `panel.styles.load` | ✅ Doc | ohne Auswahl |
| Stile ▸ Stilbibliothek importieren... | `panel.styles.importLibrary` | ✅ Doc | ohne Auswahl |
| Stile ▸ Stilbibliothek speichern | `panel.styles.saveLibrary` | ✅ Ui | ohne Auswahl: Datei geschrieben |
| Stile ▸ Große Symbole | `panel.styles.largeIcons` | ✅ Ui | ohne Auswahl |
| Stile ▸ Hilfe | `help.firepixelHelp {"topic":"panels"}` | ✅ Ui | ohne Auswahl: Befehl liefert {"path":"C:\\Users\\Administrator\\Desktop\\REA\\firepixel\\docs\\ANLEITUNG.md","section":"Bedienfelder","sections":["Firepixel – Kurzanleit – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Stile ▸ Schließen | `TogglePanel(Styles)` | ✅ Ui | ohne Auswahl |
| Stile ▸ Registerkartengruppe schließen | `ui.closeGroup {"panel":"Styles"}` | ✅ Ui | ohne Auswahl |
| Farbpalette ▸ Hilfe | `help.firepixelHelp {"topic":"panels"}` | ✅ Ui | ohne Auswahl: Befehl liefert {"path":"C:\\Users\\Administrator\\Desktop\\REA\\firepixel\\docs\\ANLEITUNG.md","section":"Bedienfelder","sections":["Firepixel – Kurzanleit – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Farbpalette ▸ Schließen | `TogglePanel(ColorPalette)` | ✅ Ui | ohne Auswahl |
| Farbpalette ▸ Registerkartengruppe schließen | `ui.closeGroup {"panel":"ColorPalette"}` | ✅ Ui | ohne Auswahl |
| Farbfelder ▸ Farbfelder hinzufügen... | `panel.swatches.add` | ✅ Ui | ohne Auswahl |
| Farbfelder ▸ Farbfelder ersetzen... | `panel.swatches.replace` | ✅ Ui | ohne Auswahl |
| Farbfelder ▸ Farbfelder speichern... | `panel.swatches.save` | ✅ Ui | ohne Auswahl: Datei geschrieben |
| Farbfelder ▸ Farbfelder löschen | `panel.swatches.clear` | ✅ Ui | ohne Auswahl |
| Farbfelder ▸ Farbwürfel | `panel.swatches.colorCubes` | ✅ Ui | ohne Auswahl: keine erkennbare Wirkung |
| Farbfelder ▸ Kontinuierlicher Farbton | `panel.swatches.continuousTone` | ✅ Ui | ohne Auswahl |
| Farbfelder ▸ Windows-System | `panel.swatches.windows` | ✅ Ui | ohne Auswahl |
| Farbfelder ▸ Macintosh-System | `panel.swatches.macintosh` | ✅ Ui | ohne Auswahl |
| Farbfelder ▸ Graustufen | `panel.swatches.grayscale` | ✅ Ui | ohne Auswahl |
| Farbfelder ▸ Aktuelle Exportpalette | `panel.swatches.exportPalette` | ✅ Ui | ohne Auswahl |
| Farbfelder ▸ Nach Farbe sortieren | `panel.swatches.sortByColor` | ✅ Ui | ohne Auswahl |
| Farbfelder ▸ Hilfe | `help.firepixelHelp {"topic":"panels"}` | ✅ Ui | ohne Auswahl: Befehl liefert {"path":"C:\\Users\\Administrator\\Desktop\\REA\\firepixel\\docs\\ANLEITUNG.md","section":"Bedienfelder","sections":["Firepixel – Kurzanleit – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Farbfelder ▸ Schließen | `TogglePanel(Swatches)` | ✅ Ui | ohne Auswahl |
| Farbfelder ▸ Registerkartengruppe schließen | `ui.closeGroup {"panel":"Swatches"}` | ✅ Ui | ohne Auswahl |
| Sonderzeichen ▸ Hilfe | `help.firepixelHelp {"topic":"panels"}` | ✅ Ui | ohne Auswahl: Befehl liefert {"path":"C:\\Users\\Administrator\\Desktop\\REA\\firepixel\\docs\\ANLEITUNG.md","section":"Bedienfelder","sections":["Firepixel – Kurzanleit – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Sonderzeichen ▸ Schließen | `TogglePanel(SpecialCharacters)` | ✅ Ui | ohne Auswahl |
| Sonderzeichen ▸ Registerkartengruppe schließen | `ui.closeGroup {"panel":"SpecialCharacters"}` | ✅ Ui | ohne Auswahl |
| Bildbearbeitung ▸ Hilfe | `help.firepixelHelp {"topic":"panels"}` | ✅ Ui | ohne Auswahl: Befehl liefert {"path":"C:\\Users\\Administrator\\Desktop\\REA\\firepixel\\docs\\ANLEITUNG.md","section":"Bedienfelder","sections":["Firepixel – Kurzanleit – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Bildbearbeitung ▸ Schließen | `TogglePanel(ImageEditing)` | ✅ Ui | ohne Auswahl |
| Bildbearbeitung ▸ Registerkartengruppe schließen | `ui.closeGroup {"panel":"ImageEditing"}` | ✅ Ui | ohne Auswahl |
| Pfad ▸ Hilfe | `help.firepixelHelp {"topic":"panels"}` | ✅ Ui | ohne Auswahl: Befehl liefert {"path":"C:\\Users\\Administrator\\Desktop\\REA\\firepixel\\docs\\ANLEITUNG.md","section":"Bedienfelder","sections":["Firepixel – Kurzanleit – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Pfad ▸ Schließen | `TogglePanel(Path)` | ✅ Ui | ohne Auswahl |
| Pfad ▸ Registerkartengruppe schließen | `ui.closeGroup {"panel":"Path"}` | ✅ Ui | ohne Auswahl |
| Automatische Formen ▸ Hilfe | `help.firepixelHelp {"topic":"panels"}` | ✅ Ui | ohne Auswahl: Befehl liefert {"path":"C:\\Users\\Administrator\\Desktop\\REA\\firepixel\\docs\\ANLEITUNG.md","section":"Bedienfelder","sections":["Firepixel – Kurzanleit – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Automatische Formen ▸ Schließen | `TogglePanel(AutoShapes)` | ✅ Ui | ohne Auswahl |
| Automatische Formen ▸ Registerkartengruppe schließen | `ui.closeGroup {"panel":"AutoShapes"}` | ✅ Ui | ohne Auswahl |
| Dokumentbibliothek ▸ Neues Symbol... | `panel.library.newSymbol` | ✅ Dialog | ohne Auswahl |
| Dokumentbibliothek ▸ Duplizieren | `panel.library.duplicate` | ✅ Doc | ohne Auswahl |
| Dokumentbibliothek ▸ Löschen | `panel.library.delete` | ✅ Dialog | ohne Auswahl |
| Dokumentbibliothek ▸ Symbol bearbeiten | `panel.library.edit` | ✅ Ui | ohne Auswahl: Dokument geöffnet/gewechselt |
| Dokumentbibliothek ▸ Eigenschaften... | `panel.library.properties` | ✅ Dialog | ohne Auswahl |
| Dokumentbibliothek ▸ Hilfslinien für 9-Slice-Skalierung | `panel.library.nineSlice` | ✅ Doc | ohne Auswahl |
| Dokumentbibliothek ▸ Hilfslinien für 9-Slice-Skalierung sperren | `panel.library.lockNineSlice` | ✅ Doc | ohne Auswahl |
| Dokumentbibliothek ▸ In allgemeiner Bibliothek speichern... | `panel.library.saveToCommon` | ✅ Dialog | ohne Auswahl |
| Dokumentbibliothek ▸ Nicht verwendete Elemente auswählen | `panel.library.selectUnused` | ✅ Ui | ohne Auswahl |
| Dokumentbibliothek ▸ Aktualisieren... | `panel.library.update` | ✅ Doc | importiertes Symbol |
| Dokumentbibliothek ▸ Abspielen | `panel.library.play` | ✅ Ui | ohne Auswahl |
| Dokumentbibliothek ▸ Symbole importieren... | `panel.library.import` | ✅ Doc | ohne Auswahl |
| Dokumentbibliothek ▸ Symbole exportieren... | `panel.library.export` | ✅ Ui | ohne Auswahl: Datei geschrieben |
| Dokumentbibliothek ▸ Hilfe | `help.firepixelHelp {"topic":"panels"}` | ✅ Ui | ohne Auswahl: Befehl liefert {"path":"C:\\Users\\Administrator\\Desktop\\REA\\firepixel\\docs\\ANLEITUNG.md","section":"Bedienfelder","sections":["Firepixel – Kurzanleit – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Dokumentbibliothek ▸ Schließen | `TogglePanel(DocumentLibrary)` | ✅ Ui | ohne Auswahl |
| Dokumentbibliothek ▸ Registerkartengruppe schließen | `ui.closeGroup {"panel":"DocumentLibrary"}` | ✅ Ui | ohne Auswahl |
| Allgemeine Bibliothek ▸ Hilfe | `help.firepixelHelp {"topic":"panels"}` | ✅ Ui | ohne Auswahl: Befehl liefert {"path":"C:\\Users\\Administrator\\Desktop\\REA\\firepixel\\docs\\ANLEITUNG.md","section":"Bedienfelder","sections":["Firepixel – Kurzanleit – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Allgemeine Bibliothek ▸ Schließen | `TogglePanel(CommonLibrary)` | ✅ Ui | ohne Auswahl |
| Allgemeine Bibliothek ▸ Registerkartengruppe schließen | `ui.closeGroup {"panel":"CommonLibrary"}` | ✅ Ui | ohne Auswahl |
| Info ▸ Hexadezimal | `panel.info.hex` | ✅ Ui | ohne Auswahl: keine erkennbare Wirkung |
| Info ▸ RGB | `panel.info.rgb` | ✅ Ui | ohne Auswahl |
| Info ▸ CMY | `panel.info.cmy` | ✅ Ui | ohne Auswahl |
| Info ▸ HSB | `panel.info.hsb` | ✅ Ui | ohne Auswahl |
| Info ▸ Pixel | `panel.info.pixels` | ✅ Ui | ohne Auswahl: keine erkennbare Wirkung |
| Info ▸ Zoll | `panel.info.inches` | ✅ Ui | ohne Auswahl |
| Info ▸ Zentimeter | `panel.info.centimeters` | ✅ Ui | ohne Auswahl |
| Info ▸ Attribute skalieren | `panel.info.scaleAttributes` | ✅ Ui | ohne Auswahl |
| Info ▸ Hilfe | `help.firepixelHelp {"topic":"panels"}` | ✅ Ui | ohne Auswahl: Befehl liefert {"path":"C:\\Users\\Administrator\\Desktop\\REA\\firepixel\\docs\\ANLEITUNG.md","section":"Bedienfelder","sections":["Firepixel – Kurzanleit – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Info ▸ Schließen | `TogglePanel(Info)` | ✅ Ui | ohne Auswahl |
| Info ▸ Registerkartengruppe schließen | `ui.closeGroup {"panel":"Info"}` | ✅ Ui | ohne Auswahl |
| Farbmischer ▸ RGB | `panel.mixer.rgb` | ✅ Ui | ohne Auswahl |
| Farbmischer ▸ Hexadezimal | `panel.mixer.hex` | ✅ Ui | ohne Auswahl: keine erkennbare Wirkung |
| Farbmischer ▸ CMY | `panel.mixer.cmy` | ✅ Ui | ohne Auswahl |
| Farbmischer ▸ HSB | `panel.mixer.hsb` | ✅ Ui | ohne Auswahl |
| Farbmischer ▸ Graustufen | `panel.mixer.grayscale` | ✅ Ui | ohne Auswahl |
| Farbmischer ▸ Hilfe | `help.firepixelHelp {"topic":"panels"}` | ✅ Ui | ohne Auswahl: Befehl liefert {"path":"C:\\Users\\Administrator\\Desktop\\REA\\firepixel\\docs\\ANLEITUNG.md","section":"Bedienfelder","sections":["Firepixel – Kurzanleit – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Farbmischer ▸ Schließen | `TogglePanel(ColorMixer)` | ✅ Ui | ohne Auswahl |
| Farbmischer ▸ Registerkartengruppe schließen | `ui.closeGroup {"panel":"ColorMixer"}` | ✅ Ui | ohne Auswahl |
| URL ▸ Hilfe | `help.firepixelHelp {"topic":"panels"}` | ✅ Ui | ohne Auswahl: Befehl liefert {"path":"C:\\Users\\Administrator\\Desktop\\REA\\firepixel\\docs\\ANLEITUNG.md","section":"Bedienfelder","sections":["Firepixel – Kurzanleit – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| URL ▸ Schließen | `TogglePanel(Url)` | ✅ Ui | ohne Auswahl |
| URL ▸ Registerkartengruppe schließen | `ui.closeGroup {"panel":"Url"}` | ✅ Ui | ohne Auswahl |
| Verhalten ▸ Hilfe | `help.firepixelHelp {"topic":"panels"}` | ✅ Ui | ohne Auswahl: Befehl liefert {"path":"C:\\Users\\Administrator\\Desktop\\REA\\firepixel\\docs\\ANLEITUNG.md","section":"Bedienfelder","sections":["Firepixel – Kurzanleit – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Verhalten ▸ Schließen | `TogglePanel(Behaviors)` | ✅ Ui | ohne Auswahl |
| Verhalten ▸ Registerkartengruppe schließen | `ui.closeGroup {"panel":"Behaviors"}` | ✅ Ui | ohne Auswahl |
| Suchen und ersetzen ▸ Hilfe | `help.firepixelHelp {"topic":"panels"}` | ✅ Ui | ohne Auswahl: Befehl liefert {"path":"C:\\Users\\Administrator\\Desktop\\REA\\firepixel\\docs\\ANLEITUNG.md","section":"Bedienfelder","sections":["Firepixel – Kurzanleit – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Suchen und ersetzen ▸ Schließen | `TogglePanel(FindReplace)` | ✅ Ui | ohne Auswahl |
| Suchen und ersetzen ▸ Registerkartengruppe schließen | `ui.closeGroup {"panel":"FindReplace"}` | ✅ Ui | ohne Auswahl |
| Symboleigenschaften ▸ Hilfe | `help.firepixelHelp {"topic":"panels"}` | ✅ Ui | ohne Auswahl: Befehl liefert {"path":"C:\\Users\\Administrator\\Desktop\\REA\\firepixel\\docs\\ANLEITUNG.md","section":"Bedienfelder","sections":["Firepixel – Kurzanleit – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Symboleigenschaften ▸ Schließen | `TogglePanel(SymbolProperties)` | ✅ Ui | ohne Auswahl |
| Symboleigenschaften ▸ Registerkartengruppe schließen | `ui.closeGroup {"panel":"SymbolProperties"}` | ✅ Ui | ohne Auswahl |
| Eigenschaften für automatische Form ▸ Hilfe | `help.firepixelHelp {"topic":"panels"}` | ✅ Ui | ohne Auswahl: Befehl liefert {"path":"C:\\Users\\Administrator\\Desktop\\REA\\firepixel\\docs\\ANLEITUNG.md","section":"Bedienfelder","sections":["Firepixel – Kurzanleit – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| Eigenschaften für automatische Form ▸ Schließen | `TogglePanel(AutoShapeProperties)` | ✅ Ui | ohne Auswahl |
| Eigenschaften für automatische Form ▸ Registerkartengruppe schließen | `ui.closeGroup {"panel":"AutoShapeProperties"}` | ✅ Ui | ohne Auswahl |
| CSS-Eigenschaften ▸ Hilfe | `help.firepixelHelp {"topic":"panels"}` | ✅ Ui | ohne Auswahl: Befehl liefert {"path":"C:\\Users\\Administrator\\Desktop\\REA\\firepixel\\docs\\ANLEITUNG.md","section":"Bedienfelder","sections":["Firepixel – Kurzanleit – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) |
| CSS-Eigenschaften ▸ Schließen | `TogglePanel(CssProperties)` | ✅ Ui | ohne Auswahl |
| CSS-Eigenschaften ▸ Registerkartengruppe schließen | `ui.closeGroup {"panel":"CssProperties"}` | ✅ Ui | ohne Auswahl |
