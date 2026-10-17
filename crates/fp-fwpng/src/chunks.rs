//! Private Chunks: `mkBF`, `mkBT`, `prVW`/`pRVW`, `mkPG` (Byte-Layouts 02 §2.2).

use crate::error::{FwError, Result};
use crate::zlib;
use fp_model::{Tile, TileKind};
use std::sync::Arc;

/// Magic von `mkBF`.
pub const MAGIC_BF: u32 = 0xFADE_CAFE;
/// Magic von `mkBT`.
pub const MAGIC_BT: u32 = 0xFACE_CAFE;
/// Magic von `prVW`.
pub const MAGIC_PV: u32 = 0xCAFE_BEEF;
/// Kopf eines `mkBT`-Chunks.
pub const BT_HEADER: usize = 76;

fn be32(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_be_bytes([
        *d.get(o)?,
        *d.get(o + 1)?,
        *d.get(o + 2)?,
        *d.get(o + 3)?,
    ]))
}

fn bad(kind: &str, msg: impl Into<String>) -> FwError {
    FwError::BadChunk {
        kind: kind.into(),
        msg: msg.into(),
    }
}

/// `mkBF`: 72 Byte, Magic `FADECAFE`, u32 = 4, 64 Nullbytes.
pub fn check_format_block(data: &[u8]) -> Result<()> {
    if be32(data, 0) != Some(MAGIC_BF) {
        return Err(bad("mkBF", "Magic FADECAFE fehlt"));
    }
    Ok(())
}

/// Erzeugt den `mkBF`-Inhalt wie CS6.
pub fn make_format_block() -> Vec<u8> {
    let mut d = vec![0u8; 72];
    d[..4].copy_from_slice(&MAGIC_BF.to_be_bytes());
    d[4..8].copy_from_slice(&4u32.to_be_bytes());
    d
}

/// Liest eine Kachel (`mkBT`). `inflate` = Pixel sofort entpacken (sonst leer, `compressed` gesetzt).
pub fn read_tile(data: &[u8], inflate: bool) -> Result<Tile> {
    if data.len() < BT_HEADER || be32(data, 0) != Some(MAGIC_BT) {
        return Err(bad("mkBT", "Kopf ungültig (FACECAFE)"));
    }
    let id = be32(data, 4).unwrap_or(0);
    let kind = TileKind::from_code(be32(data, 8).unwrap_or(0));
    let reserved = &data[12..BT_HEADER];
    let reserved = if reserved.iter().any(|&b| b != 0) {
        Some(Arc::<[u8]>::from(reserved))
    } else {
        None
    };
    let z: Arc<[u8]> = Arc::from(&data[BT_HEADER..]);
    let pixels: Arc<[u8]> = if inflate {
        Arc::from(zlib::inflate_limited(&z, fp_model::image::TILE_BYTES * 4)?)
    } else {
        Arc::from(Vec::new())
    };
    Ok(Tile {
        id,
        kind,
        pixels,
        compressed: Some(z),
        reserved,
    })
}

/// Erzeugt einen `mkBT`-Inhalt; übernimmt den Original-zlib-Strom, falls vorhanden.
pub fn make_tile(t: &Tile, id: u32) -> Vec<u8> {
    let z: Vec<u8> = match &t.compressed {
        Some(z) => z.to_vec(),
        None => zlib::deflate(&t.pixels),
    };
    let mut d = Vec::with_capacity(BT_HEADER + z.len());
    d.extend_from_slice(&MAGIC_BT.to_be_bytes());
    d.extend_from_slice(&id.to_be_bytes());
    d.extend_from_slice(&t.kind.code().to_be_bytes());
    match &t.reserved {
        Some(r) if r.len() == 64 => d.extend_from_slice(r),
        _ => d.extend_from_slice(&[0u8; 64]),
    }
    d.extend_from_slice(&z);
    d
}

/// Entpackte Vorschau (`prVW`/`pRVW`).
#[derive(Clone, Debug, PartialEq)]
pub struct Preview {
    pub width: u16,
    pub height: u16,
    /// 256 × RGB (unbenutzte Einträge beliebig).
    pub palette: Vec<u8>,
    /// Indexbytes im festen 128er-Raster (128 × 128).
    pub indices: Vec<u8>,
    /// Seitenname (CS5/CS6: UTF-16LE ab `0x4308`).
    pub page_name: Option<String>,
    /// Länge der entpackten Daten (17.160 oder 17.672 Byte).
    pub raw_len: usize,
}

/// Offsets im entpackten `prVW`.
pub const PV_PAL: usize = 8;
pub const PV_IDX: usize = 0x308;
pub const PV_STRIDE: usize = 128;
pub const PV_SIZE: usize = 0x4308;
pub const PV_SIZE_NAMED: usize = 0x4508;

impl Preview {
    /// Dekodiert einen `prVW`-Chunk (zlib).
    pub fn decode(chunk: &[u8]) -> Result<Preview> {
        let z = zlib::inflate_limited(chunk, 1 << 20)?;
        if be32(&z, 0) != Some(MAGIC_PV) {
            return Err(bad("prVW", "Magic CAFEBEEF fehlt"));
        }
        if z.len() < PV_SIZE {
            return Err(bad("prVW", format!("zu kurz ({} Byte)", z.len())));
        }
        let w = u16::from_be_bytes([z[4], z[5]]);
        let h = u16::from_be_bytes([z[6], z[7]]);
        if w as usize > PV_STRIDE || h as usize > PV_STRIDE {
            return Err(bad("prVW", format!("Größe {w}×{h} über 128")));
        }
        let page_name = if z.len() >= PV_SIZE + 2 {
            let units: Vec<u16> = z[PV_SIZE..]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            let end = units.iter().position(|&u| u == 0).unwrap_or(units.len());
            Some(String::from_utf16_lossy(&units[..end]))
        } else {
            None
        };
        Ok(Preview {
            width: w,
            height: h,
            palette: z[PV_PAL..PV_IDX].to_vec(),
            indices: z[PV_IDX..PV_SIZE].to_vec(),
            page_name,
            raw_len: z.len(),
        })
    }

    /// Index des Pixels (x, y).
    pub fn index(&self, x: usize, y: usize) -> u8 {
        self.indices.get(y * PV_STRIDE + x).copied().unwrap_or(0)
    }

    /// Farbe des Pixels als RGB.
    pub fn rgb(&self, x: usize, y: usize) -> [u8; 3] {
        let i = self.index(x, y) as usize * 3;
        match self.palette.get(i..i + 3) {
            Some(c) => [c[0], c[1], c[2]],
            None => [0, 0, 0],
        }
    }

    /// Bild als RGBA8 (immer deckend, wie der Original-Leser).
    pub fn to_rgba(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.width as usize * self.height as usize * 4);
        for y in 0..self.height as usize {
            for x in 0..self.width as usize {
                let c = self.rgb(x, y);
                out.extend_from_slice(&[c[0], c[1], c[2], 255]);
            }
        }
        out
    }

    /// Kodiert die Vorschau (zlib). Mit Seitenname 17.672 Byte, sonst 17.160 Byte.
    pub fn encode(&self) -> Vec<u8> {
        let size = if self.page_name.is_some() {
            PV_SIZE_NAMED
        } else {
            PV_SIZE
        };
        let mut z = vec![0u8; size];
        z[..4].copy_from_slice(&MAGIC_PV.to_be_bytes());
        z[4..6].copy_from_slice(&self.width.to_be_bytes());
        z[6..8].copy_from_slice(&self.height.to_be_bytes());
        let pl = self.palette.len().min(768);
        z[PV_PAL..PV_PAL + pl].copy_from_slice(&self.palette[..pl]);
        let il = self.indices.len().min(PV_SIZE - PV_IDX);
        z[PV_IDX..PV_IDX + il].copy_from_slice(&self.indices[..il]);
        if let Some(name) = &self.page_name {
            let units: Vec<u16> = name.encode_utf16().take(255).collect();
            for (k, u) in units.iter().enumerate() {
                let o = PV_SIZE + 2 * k;
                z[o..o + 2].copy_from_slice(&u.to_le_bytes());
            }
        }
        zlib::deflate(&z)
    }
}

/// `mkPG`: Seitenname (UTF-16LE mit Null), zlib.
pub fn decode_page_name(chunk: &[u8]) -> Result<String> {
    let z = zlib::inflate_limited(chunk, 1 << 16)?;
    let units: Vec<u16> = z
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    let end = units.iter().position(|&u| u == 0).unwrap_or(units.len());
    Ok(String::from_utf16_lossy(&units[..end]))
}

/// Erzeugt einen `mkPG`-Inhalt.
pub fn encode_page_name(name: &str) -> Vec<u8> {
    let mut d = Vec::new();
    for u in name.encode_utf16() {
        d.extend_from_slice(&u.to_le_bytes());
    }
    d.extend_from_slice(&[0, 0]);
    zlib::deflate(&d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_roundtrip() {
        let px: Vec<u8> = (0..65536u32).map(|i| (i % 7) as u8).collect();
        let t = Tile::new_image(5, px.clone());
        let d = make_tile(&t, 77);
        let back = read_tile(&d, true).unwrap();
        assert_eq!(back.id, 77);
        assert_eq!(&back.pixels[..], &px[..]);
        assert!(read_tile(&d[..10], true).is_err());
    }

    #[test]
    fn preview_roundtrip() {
        let mut pal = vec![0u8; 768];
        pal[..3].copy_from_slice(&[255, 255, 255]);
        let mut idx = vec![0u8; 128 * 128];
        idx[5] = 1;
        let p = Preview {
            width: 112,
            height: 112,
            palette: pal,
            indices: idx,
            page_name: Some("Seite 1".into()),
            raw_len: 0,
        };
        let back = Preview::decode(&p.encode()).unwrap();
        assert_eq!(back.page_name.as_deref(), Some("Seite 1"));
        assert_eq!(back.index(5, 0), 1);
        assert_eq!(back.raw_len, PV_SIZE_NAMED);
        assert_eq!(
            decode_page_name(&encode_page_name("Page 1")).unwrap(),
            "Page 1"
        );
        check_format_block(&make_format_block()).unwrap();
    }
}
