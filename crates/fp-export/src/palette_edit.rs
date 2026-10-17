//! Palettenbearbeitung wie im Optimieren-Bedienfeld (08 §2.5, `PALv` mit `CLLb`/`CLTb`/`CLMb`+`BCLi`):
//! Farbe sperren/entsperren, bearbeiten, auf Web-Farbe einrasten, transparent schalten, hinzufügen,
//! löschen, alle entsperren, Bearbeitungen verwerfen, Palette laden/speichern (ACT) und die
//! Palettenansicht des Bedienfelds (Farbe, Zähler, Flags, websicher).
//!
//! Alle Änderungen schreiben nur [`OptimizeSettings::palette`] bzw. die Palettenart; der Export
//! (`pipeline::quantize_indexed`) wendet sie wie das Original an: gesperrte Farben bleiben bzw. werden
//! angehängt, transparent markierte bekommen den Transparenzindex (nur mit Indextransparenz),
//! bearbeitete ersetzen ihren Eintrag, wenn die Originalfarbe exakt in der Palette steht.

use fp_raster::RgbaImage;

use crate::options::{OptimizeSettings, PaletteColor, PaletteKind, Transparency};
use crate::pipeline::{ExportContext, image_pixels, quantize_indexed};

/// Ein Eintrag der Palettenansicht.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaletteEntry {
    /// Farbe im Bild (vor einer Bearbeitung), `0xAARRGGBB`.
    pub color: u32,
    /// Angezeigte bzw. exportierte Farbe (nach Bearbeitung).
    pub shown: u32,
    /// Pixelzahl (Popularität).
    pub count: u64,
    pub locked: bool,
    pub transparent: bool,
    pub edited: bool,
    /// Alle Kanäle Vielfache von 51 (websicher, Raute im Bedienfeld).
    pub web_safe: bool,
}

fn rgb(c: u32) -> u32 {
    c & 0x00FF_FFFF
}

/// Websichere Nachbarfarbe (jeder Kanal auf das nächste Vielfache von 51, Grenzen 25/26 …).
pub fn web_snap(c: u32) -> u32 {
    let s = |v: u32| {
        if v % 51 <= 25 {
            v / 51 * 51
        } else {
            (v / 51 + 1) * 51
        }
    };
    (c & 0xFF00_0000) | s((c >> 16) & 255) << 16 | s((c >> 8) & 255) << 8 | s(c & 255)
}

/// Alle Kanäle Vielfache von 51?
pub fn is_web_safe(c: u32) -> bool {
    [(c >> 16) & 255, (c >> 8) & 255, c & 255]
        .iter()
        .all(|v| v % 51 == 0)
}

fn entry_mut(s: &mut OptimizeSettings, color: u32) -> &mut PaletteColor {
    let pos = s.palette.iter().position(|p| rgb(p.color) == rgb(color));
    let i = match pos {
        Some(i) => i,
        None => {
            s.palette.push(PaletteColor::new(color | 0xFF00_0000));
            s.palette.len() - 1
        }
    };
    &mut s.palette[i]
}

/// Entfernt Einträge ohne Flag (außer bei benutzerdefinierter Palette, dort sind sie die Palette).
fn tidy(s: &mut OptimizeSettings) {
    if s.palette_kind != PaletteKind::Custom {
        s.palette
            .retain(|p| p.locked || p.transparent || p.edited.is_some());
    }
}

/// Farbe sperren bzw. entsperren.
pub fn set_locked(s: &mut OptimizeSettings, color: u32, locked: bool) {
    entry_mut(s, color).locked = locked;
    tidy(s);
}

/// Farbe bearbeiten (neue Farbe, deckend). `None` verwirft die Bearbeitung.
pub fn edit_color(s: &mut OptimizeSettings, color: u32, new: Option<u32>) {
    let e = entry_mut(s, color);
    e.edited = new
        .map(|c| c | 0xFF00_0000)
        .filter(|&c| rgb(c) != rgb(color));
    tidy(s);
}

/// Farbe auf die nächste websichere Farbe einrasten (als Bearbeitung).
pub fn snap_to_web(s: &mut OptimizeSettings, color: u32) {
    edit_color(s, color, Some(web_snap(color)));
}

/// Farbe transparent schalten; schaltet die Transparenz auf „Index“, falls keine gewählt ist
/// (das Original löscht das Flag ohne Indextransparenz).
pub fn set_transparent(s: &mut OptimizeSettings, color: u32, transparent: bool) {
    if transparent && s.transparency == Transparency::None {
        s.transparency = Transparency::Index;
    }
    entry_mut(s, color).transparent = transparent;
    tidy(s);
}

/// Farbe hinzufügen (Pipette im Bedienfeld): als gesperrter Eintrag, damit sie in der Palette bleibt.
pub fn add_color(s: &mut OptimizeSettings, color: u32) {
    entry_mut(s, color).locked = true;
}

/// Farbe löschen: Die Palette wird mit den übrigen Farben benutzerdefiniert (Flags bleiben).
pub fn delete_color(s: &mut OptimizeSettings, current: &[PaletteEntry], color: u32) {
    let mut out: Vec<PaletteColor> = Vec::with_capacity(current.len());
    for e in current.iter().filter(|e| rgb(e.color) != rgb(color)) {
        let flags = s
            .palette
            .iter()
            .find(|p| rgb(p.color) == rgb(e.color))
            .copied();
        out.push(flags.unwrap_or(PaletteColor::new(e.color | 0xFF00_0000)));
    }
    s.palette = out;
    s.palette_kind = PaletteKind::Custom;
    s.max_colors = (s.palette.len() as i32).clamp(2, 256);
}

/// Alle Farben entsperren.
pub fn unlock_all(s: &mut OptimizeSettings) {
    for p in &mut s.palette {
        p.locked = false;
    }
    tidy(s);
}

/// Alle Bearbeitungen, Sperren und Transparenzmarken verwerfen („Palette neu erstellen“).
pub fn rebuild(s: &mut OptimizeSettings) {
    if s.palette_kind == PaletteKind::Custom {
        for p in &mut s.palette {
            *p = PaletteColor::new(p.color);
        }
    } else {
        s.palette.clear();
    }
}

/// Palette laden (z. B. ACT/ASE über `fp_color::swatches`): benutzerdefinierte Palette.
pub fn load_palette(s: &mut OptimizeSettings, colors: &[[u8; 3]]) {
    s.palette = colors
        .iter()
        .map(|c| {
            PaletteColor::new(0xFF00_0000 | (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32)
        })
        .collect();
    s.palette_kind = PaletteKind::Custom;
    s.max_colors = (s.palette.len() as i32).clamp(2, 256);
}

/// Palette zum Speichern (angezeigte Farben in Palettenreihenfolge, RGB).
pub fn palette_rgb(entries: &[PaletteEntry]) -> Vec<[u8; 3]> {
    entries
        .iter()
        .map(|e| [(e.shown >> 16) as u8, (e.shown >> 8) as u8, e.shown as u8])
        .collect()
}

/// Palettenansicht des Bedienfelds für ein Bild mit den aktuellen Einstellungen.
pub fn current_palette(
    img: &RgbaImage,
    s: &OptimizeSettings,
    ctx: &ExportContext,
) -> Vec<PaletteEntry> {
    let px = image_pixels(img);
    let ix = quantize_indexed(&px, img.width(), img.height(), s, ctx);
    entries_from_result(&ix.palette, &ix.counts, ix.transparent_index, s)
}

/// Palettenansicht aus einem schon berechneten Ergebnis (Palette, Zähler, Transparenzindex des
/// Indexbilds) und den Flags der Einstellungen – ohne die Pipeline ein zweites Mal zu rechnen
/// (Farbtabelle der Bildvorschau, re\17-bildvorschau.md §13). Ein Eintrag mit Alpha 0 ist
/// transparent (Indextransparenz), ebenso ein markierter Eintrag bei eingeschalteter Transparenz.
pub fn entries_from_result(
    palette: &[u32],
    counts: &[u64],
    transparent_index: Option<usize>,
    s: &OptimizeSettings,
) -> Vec<PaletteEntry> {
    let mut out = Vec::with_capacity(palette.len());
    for (i, &c) in palette.iter().enumerate() {
        let flags = s
            .palette
            .iter()
            .find(|p| rgb(p.color) == rgb(c) || p.edited.is_some_and(|n| rgb(n) == rgb(c)));
        let original = flags.map(|f| f.color).unwrap_or(c);
        let edited = flags.is_some_and(|f| f.edited.is_some());
        out.push(PaletteEntry {
            color: original,
            shown: c,
            count: counts.get(i).copied().unwrap_or(0),
            locked: flags.is_some_and(|f| f.locked),
            transparent: transparent_index == Some(i)
                || flags.is_some_and(|f| f.transparent && s.transparency != Transparency::None),
            edited,
            web_safe: is_web_safe(c),
        });
    }
    out
}

/// Pipette „Farbe für Transparenz wählen“: genau diese Farbe wird transparent, alle anderen nicht;
/// die Transparenz steht danach auf „Indextransparenz“ (re\17-bildvorschau.md §2.6).
pub fn set_only_transparent(s: &mut OptimizeSettings, color: u32) {
    for p in &mut s.palette {
        p.transparent = false;
    }
    s.transparency = Transparency::Index;
    entry_mut(s, color).transparent = true;
    tidy(s);
}

/// Bearbeitung einer Farbe entfernen („Bearbeitung entfernen“).
pub fn remove_edit(s: &mut OptimizeSettings, color: u32) {
    edit_color(s, color, None);
}

/// Alle Bearbeitungen entfernen (Sperren und Transparenz bleiben).
pub fn remove_all_edits(s: &mut OptimizeSettings) {
    for p in &mut s.palette {
        p.edited = None;
    }
    tidy(s);
}

/// Mehrere Farben löschen: die übrigen Farben werden die benutzerdefinierte Palette (Flags bleiben).
pub fn delete_colors(s: &mut OptimizeSettings, current: &[PaletteEntry], colors: &[u32]) {
    let rest: Vec<PaletteEntry> = current
        .iter()
        .filter(|e| !colors.iter().any(|&c| rgb(c) == rgb(e.color)))
        .copied()
        .collect();
    let mut out: Vec<PaletteColor> = Vec::with_capacity(rest.len());
    for e in &rest {
        let flags = s
            .palette
            .iter()
            .find(|p| rgb(p.color) == rgb(e.color))
            .copied();
        out.push(flags.unwrap_or(PaletteColor::new(e.color | 0xFF00_0000)));
    }
    s.palette = out;
    s.palette_kind = PaletteKind::Custom;
    s.max_colors = (s.palette.len() as i32).clamp(2, 256);
}

/// Ist irgendeine Farbe gesperrt bzw. bearbeitet (Menü „Alle Farben freigeben“, „Alle Bearbeitungen
/// entfernen“ grau, wenn nicht)?
pub fn any_locked(s: &OptimizeSettings) -> bool {
    s.palette.iter().any(|p| p.locked)
}

pub fn any_edited(s: &OptimizeSettings) -> bool {
    s.palette.iter().any(|p| p.edited.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use fp_geom::IRect;

    fn img() -> RgbaImage {
        let mut px = Vec::new();
        for y in 0..20 {
            for x in 0..20 {
                px.push(if x < 10 {
                    [200, 10, 10, 255]
                } else if y < 10 {
                    [10, 200, 10, 255]
                } else {
                    [12, 12, 190, 255]
                });
            }
        }
        RgbaImage::from_raw(IRect::from_xywh(0, 0, 20, 20), px).unwrap()
    }

    #[test]
    fn web_snap_rounds_like_websnap_table() {
        assert_eq!(web_snap(0xFF19_1A33), 0xFF00_3333);
        assert_eq!(web_snap(0xFFFF_FFFF), 0xFFFF_FFFF);
        assert_eq!(web_snap(0xFF4C_4D00), 0xFF33_6600);
    }

    #[test]
    fn edit_lock_transparent_delete() {
        let ctx = ExportContext::default();
        let mut s = OptimizeSettings::default();
        let pal = current_palette(&img(), &s, &ctx);
        assert!(
            pal.iter().any(|e| e.color & 0xFFFFFF == 0xC80A0A),
            "{pal:?}"
        );
        edit_color(&mut s, 0xFFC8_0A0A, Some(0xFF00_0000));
        let pal = current_palette(&img(), &s, &ctx);
        let red = pal
            .iter()
            .find(|e| e.color & 0xFFFFFF == 0xC80A0A)
            .expect("bearbeitete Farbe");
        assert!(red.edited && red.shown & 0xFFFFFF == 0);
        set_locked(&mut s, 0xFF0A_C80A, true);
        set_transparent(&mut s, 0xFF0C_0CBE, true);
        assert_eq!(s.transparency, Transparency::Index);
        let pal = current_palette(&img(), &s, &ctx);
        assert!(pal.iter().any(|e| e.locked));
        assert!(pal.iter().any(|e| e.transparent), "{pal:?}");
        delete_color(&mut s, &pal, 0xFF0A_C80A);
        assert_eq!(s.palette_kind, PaletteKind::Custom);
        assert!(s.palette.iter().all(|p| p.color & 0xFFFFFF != 0x0AC80A));
        rebuild(&mut s);
        assert!(
            s.palette
                .iter()
                .all(|p| !p.locked && !p.transparent && p.edited.is_none())
        );
    }

    /// Farbtabelle der Bildvorschau (re\17 §2.5/§2.6): Ansicht aus einem fertigen Ergebnis ohne
    /// zweiten Lauf, Pipette „Farbe für Transparenz wählen“, Bearbeitungen entfernen, mehrere löschen.
    #[test]
    fn entries_from_result_and_dialog_edits() {
        let ctx = ExportContext::default();
        let mut s = OptimizeSettings::default();
        let px = image_pixels(&img());
        let ix = quantize_indexed(&px, 20, 20, &s, &ctx);
        assert_eq!(
            entries_from_result(&ix.palette, &ix.counts, ix.transparent_index, &s),
            current_palette(&img(), &s, &ctx),
            "gleiche Ansicht ohne zweiten Lauf"
        );
        // nur diese Farbe transparent, Liste auf Indextransparenz
        set_transparent(&mut s, 0xFF0A_C80A, true);
        set_only_transparent(&mut s, 0xFFC8_0A0A);
        assert_eq!(s.transparency, Transparency::Index);
        assert_eq!(
            s.palette
                .iter()
                .filter(|p| p.transparent)
                .map(|p| p.color & 0xFF_FFFF)
                .collect::<Vec<_>>(),
            vec![0xC80A0A]
        );
        let pal = current_palette(&img(), &s, &ctx);
        assert_eq!(pal.iter().filter(|e| e.transparent).count(), 1, "{pal:?}");
        assert!(
            pal.iter()
                .find(|e| e.color & 0xFF_FFFF == 0xC80A0A)
                .is_some_and(|e| e.transparent && e.shown >> 24 == 0)
        );
        // Bearbeitungen
        snap_to_web(&mut s, 0xFF0C_0CBE);
        edit_color(&mut s, 0xFF0A_C80A, Some(0xFF11_2233));
        assert!(any_edited(&s) && !any_locked(&s));
        let pal = current_palette(&img(), &s, &ctx);
        let blue = pal
            .iter()
            .find(|e| e.color & 0xFF_FFFF == 0x0C0CBE)
            .expect("blau");
        assert!(
            blue.edited && blue.web_safe && blue.shown & 0xFF_FFFF == 0x0000CC,
            "{blue:?}"
        );
        remove_edit(&mut s, 0xFF0C_0CBE);
        assert_eq!(s.palette.iter().filter(|p| p.edited.is_some()).count(), 1);
        remove_all_edits(&mut s);
        assert!(!any_edited(&s));
        assert!(
            s.palette.iter().any(|p| p.transparent),
            "Transparenz bleibt"
        );
        // mehrere Farben löschen → benutzerdefinierte Palette mit dem Rest
        let pal = current_palette(&img(), &s, &ctx);
        let n = pal.len();
        delete_colors(&mut s, &pal, &[0xFF0A_C80A, 0xFF0C_0CBE]);
        assert_eq!(
            (s.palette_kind, s.palette.len()),
            (PaletteKind::Custom, n - 2)
        );
        assert!(
            s.palette
                .iter()
                .any(|p| p.color & 0xFF_FFFF == 0xC80A0A && p.transparent),
            "Flags bleiben"
        );
    }
}
