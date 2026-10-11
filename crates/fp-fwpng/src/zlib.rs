//! zlib-Hilfen (RFC 1950) über `flate2`.

use crate::error::{FwError, Result};
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use std::io::{Read, Write};

/// Entpackt einen zlib-Strom.
pub fn inflate(data: &[u8]) -> Result<Vec<u8>> {
    let mut d = ZlibDecoder::new(data);
    let mut out = Vec::with_capacity(data.len() * 4);
    d.read_to_end(&mut out).map_err(|e| FwError::Zlib(e.to_string()))?;
    Ok(out)
}

/// Entpackt höchstens `limit` Byte (Schutz vor übergroßen Strömen in beschädigten Dateien).
pub fn inflate_limited(data: &[u8], limit: usize) -> Result<Vec<u8>> {
    let d = ZlibDecoder::new(data);
    let mut out = Vec::with_capacity(limit.min(1 << 20));
    d.take(limit as u64 + 1).read_to_end(&mut out).map_err(|e| FwError::Zlib(e.to_string()))?;
    if out.len() > limit {
        return Err(FwError::Zlib(format!("Strom größer als {limit} Byte")));
    }
    Ok(out)
}

/// Packt Daten (Stufe 6 wie zlib-Standard).
pub fn deflate(data: &[u8]) -> Vec<u8> {
    let mut e = ZlibEncoder::new(Vec::with_capacity(data.len() / 2 + 64), Compression::new(6));
    // Schreiben in einen Vec kann nicht fehlschlagen.
    let _ = e.write_all(data);
    e.finish().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let d = b"MKBv{FRCi{1}}".repeat(100);
        let z = deflate(&d);
        assert_eq!(inflate(&z).unwrap(), d);
        assert!(inflate_limited(&z, 10).is_err());
        assert!(inflate(b"kaputt").is_err());
    }
}
