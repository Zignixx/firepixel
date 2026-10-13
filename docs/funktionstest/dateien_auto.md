# Dateien (Öffnen, Speichern, Export, Import, Drag & Drop) – automatischer Gesamttest

Erzeugt von `crates\fp-ui\tests\gesamt_dateien.rs` (`cargo test -p fp-ui --test gesamt_dateien`). Nicht von Hand ändern.

**24/24 Einträge ✅**

| Eintrag | Befehl | Ergebnis | Hinweis |
|---|---|---|---|
| Datei ▸ Speichern unter / Öffnen (FW-PNG-Rundreise) | `file.saveAs + OpenPath` | ✅ Doc | geöffnet, 800×600 |
| Datei ▸ Zuletzt geöffnete Dateien | `recent_files` | ✅ Ui |  |
| Datei ▸ Exportieren ▸ gif | `file.export` | ✅ Doc | bild.gif (7789 Byte) exportiert und geöffnet, 800×600 |
| Datei ▸ Exportieren ▸ animatedgif | `file.export` | ✅ Doc | bild.gif (9171 Byte) exportiert und geöffnet, 800×600 |
| Datei ▸ Exportieren ▸ jpeg | `file.export` | ✅ Doc | bild.jpg (18199 Byte) exportiert und geöffnet, 800×600 |
| Datei ▸ Exportieren ▸ png8 | `file.export` | ✅ Doc | bild.png (5850 Byte) exportiert und geöffnet, 800×600 |
| Datei ▸ Exportieren ▸ png24 | `file.export` | ✅ Doc | bild.png (8973 Byte) exportiert und geöffnet, 800×600 |
| Datei ▸ Exportieren ▸ png32 | `file.export` | ✅ Doc | bild.png (9279 Byte) exportiert und geöffnet, 800×600 |
| Datei ▸ Exportieren ▸ tiff | `file.export` | ✅ Doc | bild.tif (1920228 Byte) exportiert und geöffnet, 800×600 |
| Datei ▸ Exportieren ▸ bmp | `file.export` | ✅ Doc | bild.bmp (1440054 Byte) exportiert und geöffnet, 800×600 |
| Datei ▸ Exportieren ▸ wbmp | `file.export` | ✅ Doc | bild.wbmp (60006 Byte) exportiert und geöffnet, 800×600 |
| Datei ▸ Exportieren ▸ webp | `file.export` | ✅ Doc | bild.webp (6390 Byte) exportiert und geöffnet, 800×600 |
| Datei ▸ Exportieren ▸ webplossless | `file.export` | ✅ Doc | bild.webp (3128 Byte) exportiert und geöffnet, 800×600 |
| Datei ▸ Exportieren ▸ animatedwebp | `file.export` | ✅ Doc | bild.webp (7340 Byte) exportiert und geöffnet, 800×600 |
| Datei ▸ Exportieren ▸ avif | `file.export` | ✅ Doc | bild.avif (5931 Byte) exportiert und geöffnet, 800×600 |
| Datei ▸ Exportieren ▸ apng | `file.export` | ✅ Doc | bild.png (12427 Byte) exportiert und geöffnet, 800×600 |
| Datei ▸ Exportieren ▸ qoi | `file.export` | ✅ Doc | bild.qoi (15125 Byte) exportiert und geöffnet, 800×600 |
| Datei ▸ Exportieren ▸ ico | `file.export` | ✅ Doc | bild.ico (105955 Byte) exportiert und geöffnet, 256×256 |
| Datei ▸ Importieren (PNG) | `file.import` | ✅ Doc | Zwischendatei der Systemzwischenablage (PNG) als Bitmap importiert |
| Datei ▸ Öffnen ▸ d_dateimenue.png | `OpenPath` | ✅ Doc | geöffnet, 900×565 |
| Datei ▸ Öffnen ▸ a_bevel_a128_d0.gif | `OpenPath` | ✅ Doc | geöffnet, 87×36 |
| Datei ▸ Öffnen ▸ jpegsel_ohnetext.jpg | `OpenPath` | ✅ Doc | geöffnet, 100×100 |
| Datei ▸ Öffnen ▸ photo_420_10bit.heic | `OpenPath` | ✅ Doc | geöffnet, 640×768 |
| Hauptfenster ▸ Drag & Drop einer Datei | `dropped_files` | ✅ Ui |  |
