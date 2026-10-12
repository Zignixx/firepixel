# Kontextmenüs der Leinwand – automatischer Gesamttest

Erzeugt von `crates\fp-ui\tests\gesamt_menu.rs` (`cargo test -p fp-ui --test gesamt_menu`). Nicht von Hand ändern.

**283/283 Einträge ✅**

| Eintrag | Befehl | Ergebnis | Hinweis |
|---|---|---|---|
| Kontextmenü Nichts ausgewählt ▸ Einfügen | `edit.paste` | ✅ Doc | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Alles einblenden | `view.showAll` | ✅ Doc | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Vergrößerung ▸ Auswahl anpassen | `view.fitSelection` | ✅ Ui | Rechteck |
| Kontextmenü Nichts ausgewählt ▸ Vergrößerung ▸ Alles anpassen | `view.fitAll` | ✅ Ui | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Vergrößerung ▸ 6% | `view.magnification.6` | ✅ Ui | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Vergrößerung ▸ 12% | `view.magnification.12` | ✅ Ui | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Vergrößerung ▸ 25% | `view.magnification.25` | ✅ Ui | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Vergrößerung ▸ 50% | `view.magnification.50` | ✅ Ui | ohne Auswahl: keine erkennbare Wirkung |
| Kontextmenü Nichts ausgewählt ▸ Vergrößerung ▸ 66% | `view.magnification.66` | ✅ Ui | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Vergrößerung ▸ 100% | `view.magnification.100` | ✅ Ui | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Vergrößerung ▸ 150% | `view.magnification.150` | ✅ Ui | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Vergrößerung ▸ 200% | `view.magnification.200` | ✅ Ui | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Vergrößerung ▸ 300% | `view.magnification.300` | ✅ Ui | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Vergrößerung ▸ 400% | `view.magnification.400` | ✅ Ui | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Vergrößerung ▸ 800% | `view.magnification.800` | ✅ Ui | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Vergrößerung ▸ 1600% | `view.magnification.1600` | ✅ Ui | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Vergrößerung ▸ 3200% | `view.magnification.3200` | ✅ Ui | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Vergrößerung ▸ 6400% | `view.magnification.6400` | ✅ Ui | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Alles auswählen | `select.all` | ✅ Doc | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Ebenen als Hintergrund | `modify.flattenLayers` | ✅ Doc | zwei Ebenen |
| Kontextmenü Nichts ausgewählt ▸ Leinwand ändern ▸ Bildgröße... | `modify.canvas.imageSize` | ✅ Dialog | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Leinwand ändern ▸ Leinwandgröße... | `modify.canvas.canvasSize` | ✅ Dialog | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Leinwand ändern ▸ Leinwandfarbe... | `modify.canvas.canvasColor` | ✅ Dialog | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Leinwand ändern ▸ Leinwand zuschneiden | `modify.canvas.trimCanvas` | ✅ Doc | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Leinwand ändern ▸ Leinwand anpassen | `modify.canvas.fitCanvas` | ✅ Doc | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Leinwand ändern ▸ 180° drehen | `modify.canvas.rotate180Degree` | ✅ Doc | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Leinwand ändern ▸ 90° im UZS drehen | `modify.canvas.rotate90DegreeCw` | ✅ Doc | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Leinwand ändern ▸ 90° gegen UZS drehen | `modify.canvas.rotate90DegreeCcw` | ✅ Doc | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Neue Schaltfläche einfügen... | `edit.insert.newButton` | ✅ Dialog | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Neues Symbol einfügen... | `edit.insert.newSymbol` | ✅ Dialog | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Leere Bitmap einfügen | `edit.insert.emptyBitmap` | ✅ Doc | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Importieren... | `file.import` | ✅ Doc | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Exportieren... | `file.export` | ✅ Dialog | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Bildvorschau... | `file.imagePreview` | ✅ Dialog | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Symbolleisten ▸ Standard | `window.toolbars.main` | ✅ Ui | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Bedienfelder ▸ Werkzeuge | `window.tools` | ✅ Ui | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Bedienfelder ▸ Eigenschaften | `window.properties` | ✅ Ui | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Bedienfelder ▸ Optimieren | `window.optimize` | ✅ Ui | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Bedienfelder ▸ Ebenen | `window.layers` | ✅ Ui | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Bedienfelder ▸ Status | `window.states` | ✅ Ui | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Bedienfelder ▸ Verlauf | `window.history` | ✅ Ui | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Bedienfelder ▸ Automatische Formen | `window.autoShapes` | ✅ Ui | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Bedienfelder ▸ Stile | `window.styles` | ✅ Ui | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Bedienfelder ▸ Bibliothek | `window.documentLibrary` | ✅ Ui | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Bedienfelder ▸ URL | `window.url` | ✅ Ui | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Bedienfelder ▸ Farbmischer | `window.colorMixer` | ✅ Ui | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Bedienfelder ▸ Farbfelder | `window.swatches` | ✅ Ui | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Bedienfelder ▸ Info | `window.info` | ✅ Ui | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Bedienfelder ▸ Verhalten | `window.behaviors` | ✅ Ui | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Bedienfelder ▸ Suchen | `edit.findAndReplace` | ✅ Ui | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Voreinstellungen... | `edit.preferences` | ✅ Dialog | ohne Auswahl |
| Kontextmenü Nichts ausgewählt ▸ Vollbildmodus beenden | `view.exitFullScreen` | ✅ Ui | ohne Auswahl |
| Kontextmenü Objekt ausgewählt ▸ Bearbeiten ▸ Ausschneiden | `edit.cut` | ✅ Doc | Rechteck |
| Kontextmenü Objekt ausgewählt ▸ Bearbeiten ▸ Kopieren | `edit.copy` | ✅ Ui | Rechteck |
| Kontextmenü Objekt ausgewählt ▸ Bearbeiten ▸ Pfadumriss kopieren | `edit.copyAsVectors` | ✅ Ui | Rechteck |
| Kontextmenü Objekt ausgewählt ▸ Bearbeiten ▸ HTML-Code kopieren... | `edit.copyHtmlCode` | ✅ Ui | ohne Auswahl: Meldung „HTML-Code in die Zwischenablage kopiert“ |
| Kontextmenü Objekt ausgewählt ▸ Bearbeiten ▸ Einfügen | `edit.paste` | ✅ Doc | ohne Auswahl |
| Kontextmenü Objekt ausgewählt ▸ Bearbeiten ▸ Als Maske einfügen | `edit.pasteAsMask` | ✅ Doc | Rechteck |
| Kontextmenü Objekt ausgewählt ▸ Bearbeiten ▸ Innen einfügen | `edit.pasteInside` | ✅ Doc | Rechteck |
| Kontextmenü Objekt ausgewählt ▸ Bearbeiten ▸ Attribute einfügen | `edit.pasteAttributes` | ✅ Doc | Rechteck |
| Kontextmenü Objekt ausgewählt ▸ Bearbeiten ▸ Duplizieren | `edit.duplicate` | ✅ Doc | Rechteck |
| Kontextmenü Objekt ausgewählt ▸ Bearbeiten ▸ Klonen | `edit.clone` | ✅ Doc | Rechteck |
| Kontextmenü Objekt ausgewählt ▸ Bearbeiten ▸ Gewählte Bitmap zuschneiden | `edit.cropSelectedBitmap` | ✅ Doc | Bitmap |
| Kontextmenü Objekt ausgewählt ▸ Bearbeiten ▸ Dokument zuschneiden | `edit.cropDocument` | ✅ Doc | Rechteck |
| Kontextmenü Objekt ausgewählt ▸ Vergrößerung ▸ Auswahl anpassen | `view.fitSelection` | ✅ Ui | Rechteck |
| Kontextmenü Objekt ausgewählt ▸ Vergrößerung ▸ Alles anpassen | `view.fitAll` | ✅ Ui | ohne Auswahl |
| Kontextmenü Objekt ausgewählt ▸ Vergrößerung ▸ 6% | `view.magnification.6` | ✅ Ui | ohne Auswahl |
| Kontextmenü Objekt ausgewählt ▸ Vergrößerung ▸ 12% | `view.magnification.12` | ✅ Ui | ohne Auswahl |
| Kontextmenü Objekt ausgewählt ▸ Vergrößerung ▸ 25% | `view.magnification.25` | ✅ Ui | ohne Auswahl |
| Kontextmenü Objekt ausgewählt ▸ Vergrößerung ▸ 50% | `view.magnification.50` | ✅ Ui | ohne Auswahl: keine erkennbare Wirkung |
| Kontextmenü Objekt ausgewählt ▸ Vergrößerung ▸ 66% | `view.magnification.66` | ✅ Ui | ohne Auswahl |
| Kontextmenü Objekt ausgewählt ▸ Vergrößerung ▸ 100% | `view.magnification.100` | ✅ Ui | ohne Auswahl |
| Kontextmenü Objekt ausgewählt ▸ Vergrößerung ▸ 150% | `view.magnification.150` | ✅ Ui | ohne Auswahl |
| Kontextmenü Objekt ausgewählt ▸ Vergrößerung ▸ 200% | `view.magnification.200` | ✅ Ui | ohne Auswahl |
| Kontextmenü Objekt ausgewählt ▸ Vergrößerung ▸ 300% | `view.magnification.300` | ✅ Ui | ohne Auswahl |
| Kontextmenü Objekt ausgewählt ▸ Vergrößerung ▸ 400% | `view.magnification.400` | ✅ Ui | ohne Auswahl |
| Kontextmenü Objekt ausgewählt ▸ Vergrößerung ▸ 800% | `view.magnification.800` | ✅ Ui | ohne Auswahl |
| Kontextmenü Objekt ausgewählt ▸ Vergrößerung ▸ 1600% | `view.magnification.1600` | ✅ Ui | ohne Auswahl |
| Kontextmenü Objekt ausgewählt ▸ Vergrößerung ▸ 3200% | `view.magnification.3200` | ✅ Ui | ohne Auswahl |
| Kontextmenü Objekt ausgewählt ▸ Vergrößerung ▸ 6400% | `view.magnification.6400` | ✅ Ui | ohne Auswahl |
| Kontextmenü Objekt ausgewählt ▸ Auswählen ▸ Alle | `select.all` | ✅ Doc | ohne Auswahl |
| Kontextmenü Objekt ausgewählt ▸ Auswählen ▸ Auswahl aufheben | `select.none` | ✅ Doc | Rechteck |
| Kontextmenü Objekt ausgewählt ▸ Auswählen ▸ Gesamtauswahl treffen | `select.superselect` | ✅ Doc | Rechteck + Ellipse |
| Kontextmenü Objekt ausgewählt ▸ Auswählen ▸ Teilauswahl treffen | `select.subselect` | ✅ Doc | drei Objekte |
| Kontextmenü Objekt ausgewählt ▸ Auswahl ausblenden | `view.hideSelection` | ✅ Doc | Rechteck |
| Kontextmenü Objekt ausgewählt ▸ Kanten | `view.edges` | ✅ Ui | ohne Auswahl |
| Kontextmenü Objekt ausgewählt ▸ Pfad in Auswahlrahmen konvertieren... | `modify.convertPathToMarquee` | ✅ Doc | Rechteck |
| Kontextmenü Objekt ausgewählt ▸ Bitmap nachzeichnen... | `modify.traceBitmap` | ✅ Dialog | Bitmap |
| Kontextmenü Objekt ausgewählt ▸ Anordnen ▸ In den Vordergrund | `modify.arrange.bringToFront` | ✅ Doc | Rechteck |
| Kontextmenü Objekt ausgewählt ▸ Anordnen ▸ Schrittweise nach vorne | `modify.arrange.bringForward` | ✅ Doc | Rechteck |
| Kontextmenü Objekt ausgewählt ▸ Anordnen ▸ Schrittweise nach hinten | `modify.arrange.sendBackward` | ✅ Doc | Rechteck |
| Kontextmenü Objekt ausgewählt ▸ Anordnen ▸ In den Hintergrund | `modify.arrange.sendToBack` | ✅ Doc | Rechteck |
| Kontextmenü Objekt ausgewählt ▸ Transformieren ▸ Skalieren | `modify.transform.scale` | ✅ Ui | Rechteck |
| Kontextmenü Objekt ausgewählt ▸ Transformieren ▸ Neigen | `modify.transform.skew` | ✅ Ui | Rechteck |
| Kontextmenü Objekt ausgewählt ▸ Transformieren ▸ Verzerren | `modify.transform.distort` | ✅ Ui | Rechteck |
| Kontextmenü Objekt ausgewählt ▸ Transformieren ▸ Numerisch transformieren... | `modify.transform.numericTransform` | ✅ Dialog | Rechteck |
| Kontextmenü Objekt ausgewählt ▸ Transformieren ▸ 180° drehen | `modify.transform.rotate180Degree` | ✅ Doc | Rechteck |
| Kontextmenü Objekt ausgewählt ▸ Transformieren ▸ 90° im UZS drehen | `modify.transform.rotate90DegreeCw` | ✅ Doc | Rechteck |
| Kontextmenü Objekt ausgewählt ▸ Transformieren ▸ 90° gegen UZS drehen | `modify.transform.rotate90DegreeCcw` | ✅ Doc | Rechteck |
| Kontextmenü Objekt ausgewählt ▸ Transformieren ▸ Horizontal spiegeln | `modify.transform.flipHorizontal` | ✅ Doc | Rechteck |
| Kontextmenü Objekt ausgewählt ▸ Transformieren ▸ Vertikal spiegeln | `modify.transform.flipVertical` | ✅ Doc | Rechteck |
| Kontextmenü Objekt ausgewählt ▸ Transformieren ▸ Transformationen entfernen | `modify.transform.removeTransformations` | ✅ Doc | Rechteck |
| Kontextmenü Objekt ausgewählt ▸ An Pixeln ausrichten | `modify.snapToPixel` | ✅ Doc | Rechteck |
| Kontextmenü Objekt ausgewählt ▸ Gruppieren | `modify.group` | ✅ Doc | Rechteck |
| Kontextmenü Objekt ausgewählt ▸ Gruppierung aufheben | `modify.ungroup` | ✅ Doc | Rechteck |
| Kontextmenü Objekt ausgewählt ▸ Als Maske gruppieren | `modify.mask.groupAsMask` | ✅ Doc | Rechteck + Ellipse |
| Kontextmenü Objekt ausgewählt ▸ Pfad kombinieren ▸ Vereinigung | `path.combine.union` | ✅ Doc | Rechteck + Ellipse |
| Kontextmenü Objekt ausgewählt ▸ Pfad kombinieren ▸ Ausstanzen | `path.combine.punch` | ✅ Doc | Rechteck + Ellipse |
| Kontextmenü Objekt ausgewählt ▸ Pfad kombinieren ▸ Überschneiden | `path.combine.intersect` | ✅ Doc | Rechteck + Ellipse |
| Kontextmenü Objekt ausgewählt ▸ Pfad kombinieren ▸ Zuschneiden | `path.combine.crop` | ✅ Doc | Rechteck + Ellipse |
| Kontextmenü Objekt ausgewählt ▸ Pfad kombinieren ▸ Normal | `path.combine.normal` | ✅ Doc | Rechteck + Ellipse |
| Kontextmenü Objekt ausgewählt ▸ Symbol austauschen... | `modify.symbol.swapSymbol` | ✅ Dialog | Grafiksymbol |
| Kontextmenü Objekt ausgewählt ▸ In Symbol konvertieren | `modify.symbol.convertToSymbol` | ✅ Dialog | Rechteck |
| Kontextmenü Objekt ausgewählt ▸ Animation ▸ Auswahl animieren... | `modify.animation.animateSelection` | ✅ Dialog | Rechteck |
| Kontextmenü Objekt ausgewählt ▸ Animation ▸ Einstellungen... | `modify.animation.settings` | ✅ Dialog | Animationssymbol |
| Kontextmenü Objekt ausgewählt ▸ Animation ▸ Animation entfernen | `modify.animation.removeAnimation` | ✅ Doc | Animationssymbol |
| Kontextmenü Objekt ausgewählt ▸ Symbol ▸ Symbol bearbeiten | `modify.symbol.editSymbol` | ✅ Ui | Grafiksymbol: Dokument geöffnet/gewechselt |
| Kontextmenü Objekt ausgewählt ▸ Symbol ▸ An Position bearbeiten | `modify.symbol.editInPlace` | ✅ Ui | Grafiksymbol: Dokument geöffnet/gewechselt |
| Kontextmenü Objekt ausgewählt ▸ Symbol ▸ Instanzen tweenen... | `modify.symbol.tweenInstances` | ✅ Dialog | ohne Auswahl |
| Kontextmenü Objekt ausgewählt ▸ Symbol ▸ Teilen | `modify.symbol.breakApart` | ✅ Dialog | ohne Auswahl |
| Kontextmenü Objekt ausgewählt ▸ Hotspot einfügen | `edit.insert.hotspot` | ✅ Dialog | ohne Auswahl |
| Kontextmenü Objekt ausgewählt ▸ Rechteckiges Segment einfügen | `edit.insert.slice` | ✅ Dialog | ohne Auswahl |
| Kontextmenü Objekt ausgewählt ▸ Polygonsegment einfügen | `edit.insert.polygonSlice` | ✅ Dialog | ohne Auswahl |
| Kontextmenü Objekt ausgewählt ▸ Ausgewähltes Segment exportieren... | `file.export {}` | ✅ Dialog | ohne Auswahl |
| Kontextmenü Objekt ausgewählt ▸ CSS-Sprites exportieren... | `file.export {}` | ✅ Dialog | ohne Auswahl |
| Kontextmenü Objekt ausgewählt ▸ Auswahl sperren | `modify.lockSelection` | ✅ Doc | Rechteck |
| Kontextmenü Objekt ausgewählt ▸ Auswahl auf Hintergrund reduzieren | `modify.flattenSelection` | ✅ Doc | Rechteck |
| Kontextmenü Objekt ausgewählt ▸ Mit darunter liegender auf eine Ebene reduzieren | `modify.mergeDown` | ✅ Doc | Rechteck über Bitmap |
| Kontextmenü Objekt ausgewählt ▸ Ebenen als Hintergrund | `modify.flattenLayers` | ✅ Doc | zwei Ebenen |
| Kontextmenü Objekt ausgewählt ▸ Vollbildmodus beenden | `view.exitFullScreen` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Bearbeiten ▸ Ausschneiden | `edit.cut` | ✅ Doc | Rechteck |
| Kontextmenü Text ▸ Bearbeiten ▸ Kopieren | `edit.copy` | ✅ Ui | Rechteck |
| Kontextmenü Text ▸ Bearbeiten ▸ Pfadumriss kopieren | `edit.copyAsVectors` | ✅ Ui | Rechteck |
| Kontextmenü Text ▸ Bearbeiten ▸ Einfügen | `edit.paste` | ✅ Doc | ohne Auswahl |
| Kontextmenü Text ▸ Bearbeiten ▸ Als Maske einfügen | `edit.pasteAsMask` | ✅ Doc | Rechteck |
| Kontextmenü Text ▸ Bearbeiten ▸ Innen einfügen | `edit.pasteInside` | ✅ Doc | Rechteck |
| Kontextmenü Text ▸ Bearbeiten ▸ Attribute einfügen | `edit.pasteAttributes` | ✅ Doc | Rechteck |
| Kontextmenü Text ▸ Bearbeiten ▸ Duplizieren | `edit.duplicate` | ✅ Doc | Rechteck |
| Kontextmenü Text ▸ Bearbeiten ▸ Klonen | `edit.clone` | ✅ Doc | Rechteck |
| Kontextmenü Text ▸ Anordnen ▸ In den Vordergrund | `modify.arrange.bringToFront` | ✅ Doc | Rechteck |
| Kontextmenü Text ▸ Anordnen ▸ Schrittweise nach vorne | `modify.arrange.bringForward` | ✅ Doc | Rechteck |
| Kontextmenü Text ▸ Anordnen ▸ Schrittweise nach hinten | `modify.arrange.sendBackward` | ✅ Doc | Rechteck |
| Kontextmenü Text ▸ Anordnen ▸ In den Hintergrund | `modify.arrange.sendToBack` | ✅ Doc | Rechteck |
| Kontextmenü Text ▸ Vergrößerung ▸ Auswahl anpassen | `view.fitSelection` | ✅ Ui | Rechteck |
| Kontextmenü Text ▸ Vergrößerung ▸ Alles anpassen | `view.fitAll` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Vergrößerung ▸ 6% | `view.magnification.6` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Vergrößerung ▸ 12% | `view.magnification.12` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Vergrößerung ▸ 25% | `view.magnification.25` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Vergrößerung ▸ 50% | `view.magnification.50` | ✅ Ui | ohne Auswahl: keine erkennbare Wirkung |
| Kontextmenü Text ▸ Vergrößerung ▸ 66% | `view.magnification.66` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Vergrößerung ▸ 100% | `view.magnification.100` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Vergrößerung ▸ 150% | `view.magnification.150` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Vergrößerung ▸ 200% | `view.magnification.200` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Vergrößerung ▸ 300% | `view.magnification.300` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Vergrößerung ▸ 400% | `view.magnification.400` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Vergrößerung ▸ 800% | `view.magnification.800` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Vergrößerung ▸ 1600% | `view.magnification.1600` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Vergrößerung ▸ 3200% | `view.magnification.3200` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Vergrößerung ▸ 6400% | `view.magnification.6400` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Auswählen ▸ Alle | `select.all` | ✅ Doc | ohne Auswahl |
| Kontextmenü Text ▸ Auswählen ▸ Auswahl aufheben | `select.none` | ✅ Doc | Rechteck |
| Kontextmenü Text ▸ Auswählen ▸ Gesamtauswahl treffen | `select.superselect` | ✅ Doc | Rechteck + Ellipse |
| Kontextmenü Text ▸ Auswählen ▸ Teilauswahl treffen | `select.subselect` | ✅ Doc | drei Objekte |
| Kontextmenü Text ▸ Größe ▸ Andere... | `text.size.other` | ✅ Dialog | ohne Auswahl |
| Kontextmenü Text ▸ Größe ▸ 8 | `text.size.8` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Größe ▸ 9 | `text.size.9` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Größe ▸ 10 | `text.size.10` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Größe ▸ 12 | `text.size.12` | ✅ Ui | ohne Auswahl: keine erkennbare Wirkung |
| Kontextmenü Text ▸ Größe ▸ 14 | `text.size.14` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Größe ▸ 18 | `text.size.18` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Größe ▸ 24 | `text.size.24` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Größe ▸ 36 | `text.size.36` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Größe ▸ 48 | `text.size.48` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Größe ▸ 72 | `text.size.72` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Größe ▸ 96 | `text.size.96` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Größe ▸ 120 | `text.size.120` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Stil ▸ Standard | `text.style.plain` | ✅ Ui | ohne Auswahl: keine erkennbare Wirkung |
| Kontextmenü Text ▸ Stil ▸ Fett | `text.style.fauxBold` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Stil ▸ Kursiv | `text.style.fauxItalic` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Stil ▸ Unterstrichen | `text.style.underline` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Ausrichten ▸ Links | `text.align.left` | ✅ Ui | ohne Auswahl: keine erkennbare Wirkung |
| Kontextmenü Text ▸ Ausrichten ▸ Zentrieren | `text.align.centeredHorizontally` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Ausrichten ▸ Rechts | `text.align.right` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Ausrichten ▸ Blocksatz | `text.align.justified` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Ausrichten ▸ Oben | `text.align.top` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Ausrichten ▸ Zentrieren | `text.align.centeredVertically` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Ausrichten ▸ Unten | `text.align.bottom` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Ausrichten ▸ Blocksatz | `text.align.justifiedVertically` | ✅ Ui | ohne Auswahl |
| Kontextmenü Text ▸ Von Pfad lösen | `text.detachFromPath` | ✅ Doc | Text auf Pfad |
| Kontextmenü Text ▸ Richtung ▸ Um Pfad drehen | `text.orientation.rotateAroundPath` | ✅ Doc | Text auf Pfad |
| Kontextmenü Text ▸ Richtung ▸ Vertikal | `text.orientation.vertical` | ✅ Doc | Text auf Pfad |
| Kontextmenü Text ▸ Richtung ▸ Vertikal neigen | `text.orientation.skewVertical` | ✅ Doc | Text auf Pfad |
| Kontextmenü Text ▸ Richtung ▸ Horizontal neigen | `text.orientation.skewHorizontal` | ✅ Doc | Text auf Pfad |
| Kontextmenü Text ▸ Richtung umkehren | `text.reverseDirection` | ✅ Doc | Text auf Pfad |
| Kontextmenü Text ▸ In Pfade konvertieren | `text.convertToPaths` | ✅ Doc | Text |
| Kontextmenü Text ▸ Numerisch transformieren... | `modify.transform.numericTransform` | ✅ Dialog | Rechteck |
| Kontextmenü Text ▸ Transformationen entfernen | `modify.transform.removeTransformations` | ✅ Doc | Rechteck |
| Kontextmenü Text ▸ In Symbol konvertieren | `modify.symbol.convertToSymbol` | ✅ Dialog | Rechteck |
| Kontextmenü Text ▸ Hotspot einfügen | `edit.insert.hotspot` | ✅ Dialog | ohne Auswahl |
| Kontextmenü Text ▸ Segment einfügen | `edit.insert.slice` | ✅ Dialog | ohne Auswahl |
| Kontextmenü Text ▸ Vollbildmodus beenden | `view.exitFullScreen` | ✅ Ui | ohne Auswahl |
| Kontextmenü Pixel ▸ Bearbeiten ▸ Ausschneiden | `edit.cut` | ✅ Doc | Rechteck |
| Kontextmenü Pixel ▸ Bearbeiten ▸ Kopieren | `edit.copy` | ✅ Ui | Rechteck |
| Kontextmenü Pixel ▸ Bearbeiten ▸ Einfügen | `edit.paste` | ✅ Doc | ohne Auswahl |
| Kontextmenü Pixel ▸ Bearbeiten ▸ Als Maske einfügen | `edit.pasteAsMask` | ✅ Doc | Rechteck |
| Kontextmenü Pixel ▸ Bearbeiten ▸ Duplizieren | `edit.duplicate` | ✅ Doc | Rechteck |
| Kontextmenü Pixel ▸ Bearbeiten ▸ Klonen | `edit.clone` | ✅ Doc | Rechteck |
| Kontextmenü Pixel ▸ Vergrößerung ▸ Auswahl anpassen | `view.fitSelection` | ✅ Ui | Rechteck |
| Kontextmenü Pixel ▸ Vergrößerung ▸ Alles anpassen | `view.fitAll` | ✅ Ui | ohne Auswahl |
| Kontextmenü Pixel ▸ Vergrößerung ▸ 6% | `view.magnification.6` | ✅ Ui | ohne Auswahl |
| Kontextmenü Pixel ▸ Vergrößerung ▸ 12% | `view.magnification.12` | ✅ Ui | ohne Auswahl |
| Kontextmenü Pixel ▸ Vergrößerung ▸ 25% | `view.magnification.25` | ✅ Ui | ohne Auswahl |
| Kontextmenü Pixel ▸ Vergrößerung ▸ 50% | `view.magnification.50` | ✅ Ui | ohne Auswahl: keine erkennbare Wirkung |
| Kontextmenü Pixel ▸ Vergrößerung ▸ 66% | `view.magnification.66` | ✅ Ui | ohne Auswahl |
| Kontextmenü Pixel ▸ Vergrößerung ▸ 100% | `view.magnification.100` | ✅ Ui | ohne Auswahl |
| Kontextmenü Pixel ▸ Vergrößerung ▸ 150% | `view.magnification.150` | ✅ Ui | ohne Auswahl |
| Kontextmenü Pixel ▸ Vergrößerung ▸ 200% | `view.magnification.200` | ✅ Ui | ohne Auswahl |
| Kontextmenü Pixel ▸ Vergrößerung ▸ 300% | `view.magnification.300` | ✅ Ui | ohne Auswahl |
| Kontextmenü Pixel ▸ Vergrößerung ▸ 400% | `view.magnification.400` | ✅ Ui | ohne Auswahl |
| Kontextmenü Pixel ▸ Vergrößerung ▸ 800% | `view.magnification.800` | ✅ Ui | ohne Auswahl |
| Kontextmenü Pixel ▸ Vergrößerung ▸ 1600% | `view.magnification.1600` | ✅ Ui | ohne Auswahl |
| Kontextmenü Pixel ▸ Vergrößerung ▸ 3200% | `view.magnification.3200` | ✅ Ui | ohne Auswahl |
| Kontextmenü Pixel ▸ Vergrößerung ▸ 6400% | `view.magnification.6400` | ✅ Ui | ohne Auswahl |
| Kontextmenü Pixel ▸ Gewählte Bitmap zuschneiden | `edit.cropSelectedBitmap` | ✅ Doc | Bitmap |
| Kontextmenü Pixel ▸ Dokument zuschneiden | `edit.cropDocument` | ✅ Doc | Rechteck |
| Kontextmenü Pixel ▸ Neue Bitmap durch Kopieren | `edit.insert.bitmapViaCopy` | ✅ Doc | Bitmap + Pixelauswahl |
| Kontextmenü Pixel ▸ Neue Bitmap durch Ausschneiden | `edit.insert.bitmapViaCut` | ✅ Doc | Bitmap + Pixelauswahl |
| Kontextmenü Pixel ▸ Mit darunter liegender auf eine Ebene reduzieren | `modify.mergeDown` | ✅ Doc | Rechteck über Bitmap |
| Kontextmenü Pixel ▸ Alles auswählen | `select.all` | ✅ Doc | ohne Auswahl |
| Kontextmenü Pixel ▸ Auswahl aufheben | `select.none` | ✅ Doc | Rechteck |
| Kontextmenü Pixel ▸ Ähnliches auswählen | `select.similar` | ✅ Dialog | ohne Auswahl |
| Kontextmenü Pixel ▸ Auswahlrahmen modifizieren ▸ Auswahl umkehren | `select.inverse` | ✅ Doc | ohne Auswahl |
| Kontextmenü Pixel ▸ Auswahlrahmen modifizieren ▸ Weiche Kante... | `select.feather` | ✅ Dialog | ohne Auswahl |
| Kontextmenü Pixel ▸ Auswahlrahmen modifizieren ▸ Erweitern... | `select.expandMarquee` | ✅ Dialog | Pixelauswahl |
| Kontextmenü Pixel ▸ Auswahlrahmen modifizieren ▸ Verkleinern... | `select.contractMarquee` | ✅ Dialog | Pixelauswahl |
| Kontextmenü Pixel ▸ Auswahlrahmen modifizieren ▸ Rahmen... | `select.borderMarquee` | ✅ Dialog | Pixelauswahl |
| Kontextmenü Pixel ▸ Auswahlrahmen modifizieren ▸ Glätten... | `select.smoothMarquee` | ✅ Dialog | Pixelauswahl |
| Kontextmenü Pixel ▸ Auswahlrahmen modifizieren ▸ In Pfad konvertieren | `select.convertMarqueeToPath` | ✅ Doc | Pixelauswahl |
| Kontextmenü Pixel ▸ Ränder | `view.edges` | ✅ Ui | ohne Auswahl |
| Kontextmenü Pixel ▸ Auswahl speichern | `select.saveBitmapSelection` | ✅ Dialog | Pixelauswahl |
| Kontextmenü Pixel ▸ Auswahl wiederherstellen | `select.restoreBitmapSelection` | ✅ Dialog | gespeicherte Bitmap-Auswahl |
| Kontextmenü Pixel ▸ Selektives JPEG ▸ Auswahl als JPEG-Maske speichern | `modify.selectiveJpeg.saveSelectionAsJpegMask` | ✅ Doc | Pixelauswahl |
| Kontextmenü Pixel ▸ Selektives JPEG ▸ JPEG-Maske als Auswahl wiederherstellen | `modify.selectiveJpeg.restoreJpegMaskAsSelection` | ✅ Doc | ohne Auswahl |
| Kontextmenü Pixel ▸ Selektives JPEG ▸ Einstellungen... | `modify.selectiveJpeg.settings` | ✅ Dialog | ohne Auswahl |
| Kontextmenü Pixel ▸ Selektives JPEG ▸ JPEG-Maske entfernen | `modify.selectiveJpeg.removeJpegMask` | ✅ Doc | ohne Auswahl |
| Kontextmenü Pixel ▸ Frei transformieren | `modify.transform.freeTransform` | ✅ Ui | Rechteck |
| Kontextmenü Pixel ▸ Numerisch transformieren... | `modify.transform.numericTransform` | ✅ Dialog | Rechteck |
| Kontextmenü Pixel ▸ Hotspot einfügen | `edit.insert.hotspot` | ✅ Dialog | ohne Auswahl |
| Kontextmenü Pixel ▸ Segment einfügen | `edit.insert.slice` | ✅ Dialog | ohne Auswahl |
| Kontextmenü Pixel ▸ Vollbildmodus beenden | `view.exitFullScreen` | ✅ Ui | ohne Auswahl |
| Kontextmenü Transformieren ▸ Skalieren | `modify.transform.scale` | ✅ Ui | Rechteck |
| Kontextmenü Transformieren ▸ Neigen | `modify.transform.skew` | ✅ Ui | Rechteck |
| Kontextmenü Transformieren ▸ Verzerren | `modify.transform.distort` | ✅ Ui | Rechteck |
| Kontextmenü Transformieren ▸ Numerisch transformieren... | `modify.transform.numericTransform` | ✅ Dialog | Rechteck |
| Kontextmenü Transformieren ▸ 180°-Drehung | `modify.transform.rotate180Degree` | ✅ Doc | Rechteck |
| Kontextmenü Transformieren ▸ 90° im UZS drehen | `modify.transform.rotate90DegreeCw` | ✅ Doc | Rechteck |
| Kontextmenü Transformieren ▸ 90° gegen UZS drehen | `modify.transform.rotate90DegreeCcw` | ✅ Doc | Rechteck |
| Kontextmenü Transformieren ▸ Horizontal spiegeln | `modify.transform.flipHorizontal` | ✅ Doc | Rechteck |
| Kontextmenü Transformieren ▸ Vertikal spiegeln | `modify.transform.flipVertical` | ✅ Doc | Rechteck |
| Kontextmenü Transformieren ▸ Transformationen entfernen | `modify.transform.removeTransformations` | ✅ Doc | Rechteck |
| Kontextmenü Zuschneiden ▸ Dokument zuschneiden | `edit.cropSelectedBitmap` | ✅ Doc | Bitmap |
| Kontextmenü Zuschneiden ▸ Zuschneiden abbrechen | `ui.toolKey {"key":"escape"}` | ✅ Ui | ohne Auswahl: Meldung „unbekannter Befehl „ui.toolKey““ |
| Kontextmenü Zoom ▸ Größer zoomen | `view.zoomIn` | ✅ Ui | ohne Auswahl |
| Kontextmenü Zoom ▸ Kleiner zoomen | `view.zoomOut` | ✅ Ui | ohne Auswahl |
| Kontextmenü Zoom ▸ Vergrößerung ▸ 6% | `view.magnification.6` | ✅ Ui | ohne Auswahl |
| Kontextmenü Zoom ▸ Vergrößerung ▸ 12% | `view.magnification.12` | ✅ Ui | ohne Auswahl |
| Kontextmenü Zoom ▸ Vergrößerung ▸ 25% | `view.magnification.25` | ✅ Ui | ohne Auswahl |
| Kontextmenü Zoom ▸ Vergrößerung ▸ 50% | `view.magnification.50` | ✅ Ui | ohne Auswahl: keine erkennbare Wirkung |
| Kontextmenü Zoom ▸ Vergrößerung ▸ 66% | `view.magnification.66` | ✅ Ui | ohne Auswahl |
| Kontextmenü Zoom ▸ Vergrößerung ▸ 100% | `view.magnification.100` | ✅ Ui | ohne Auswahl |
| Kontextmenü Zoom ▸ Vergrößerung ▸ 150% | `view.magnification.150` | ✅ Ui | ohne Auswahl |
| Kontextmenü Zoom ▸ Vergrößerung ▸ 200% | `view.magnification.200` | ✅ Ui | ohne Auswahl |
| Kontextmenü Zoom ▸ Vergrößerung ▸ 300% | `view.magnification.300` | ✅ Ui | ohne Auswahl |
| Kontextmenü Zoom ▸ Vergrößerung ▸ 400% | `view.magnification.400` | ✅ Ui | ohne Auswahl |
| Kontextmenü Zoom ▸ Vergrößerung ▸ 800% | `view.magnification.800` | ✅ Ui | ohne Auswahl |
| Kontextmenü Zoom ▸ Vergrößerung ▸ 1600% | `view.magnification.1600` | ✅ Ui | ohne Auswahl |
| Kontextmenü Zoom ▸ Vergrößerung ▸ 3200% | `view.magnification.3200` | ✅ Ui | ohne Auswahl |
| Kontextmenü Zoom ▸ Vergrößerung ▸ 6400% | `view.magnification.6400` | ✅ Ui | ohne Auswahl |
| Kontextmenü Zoom ▸ Auswahl anpassen | `view.fitSelection` | ✅ Ui | Rechteck |
| Kontextmenü Zoom ▸ Alles anpassen | `view.fitAll` | ✅ Ui | ohne Auswahl |
| Kontextmenü Pipette ▸ Farbe kopieren | `ui.copyColor {"which":"sample"}` | ✅ Ui | ohne Auswahl: in die Zwischenablage |
| Kontextmenü Pipette ▸ Füllfarbe kopieren | `ui.copyColor {"which":"fill"}` | ✅ Ui | ohne Auswahl: Meldung „[!! Farbe #999999 kopiert ········!!]“ |
| Kontextmenü Pipette ▸ Strichfarbe kopieren | `ui.copyColor {"which":"stroke"}` | ✅ Ui | ohne Auswahl: Meldung „[!! Farbe #000000 kopiert ········!!]“ |
