//! Leser für `.safetensors`-Dateien (8 Byte Kopflänge, JSON-Kopf, Rohdaten). Gewichte werden beim
//! Laden nach `f32` gewandelt (F32, F16, BF16; I64 nur für Indextabellen).

use crate::error::{AiError, Result};
use crate::nn::Conv2d;
use std::collections::HashMap;
use std::path::Path;

/// Ein geladener Tensor.
#[derive(Clone, Debug)]
pub struct Weight {
    pub shape: Vec<usize>,
    pub data: Vec<f32>,
}

/// Alle Tensoren einer Datei.
#[derive(Debug, Default)]
pub struct Weights {
    map: HashMap<String, Weight>,
}

fn f16_to_f32(h: u16) -> f32 {
    let sign = ((h >> 15) & 1) as u32;
    let exp = ((h >> 10) & 0x1f) as u32;
    let frac = (h & 0x3ff) as u32;
    let bits = if exp == 0 {
        if frac == 0 {
            sign << 31
        } else {
            // subnormal
            let mut e = 127 - 15 + 1;
            let mut f = frac;
            while f & 0x400 == 0 {
                f <<= 1;
                e -= 1;
            }
            (sign << 31) | ((e as u32) << 23) | ((f & 0x3ff) << 13)
        }
    } else if exp == 0x1f {
        (sign << 31) | (0xff << 23) | (frac << 13)
    } else {
        (sign << 31) | ((exp + 127 - 15) << 23) | (frac << 13)
    };
    f32::from_bits(bits)
}

impl Weights {
    /// Datei laden.
    pub fn load(path: &Path) -> Result<Weights> {
        let bytes = std::fs::read(path)?;
        Weights::parse(&bytes)
    }

    /// Aus Bytes.
    pub fn parse(bytes: &[u8]) -> Result<Weights> {
        let bad = |m: &str| AiError::Weights(m.to_string());
        let n = u64::from_le_bytes(bytes.get(..8).ok_or_else(|| bad("Datei zu kurz"))?.try_into().map_err(|_| bad("Kopf"))?) as usize;
        let head = bytes.get(8..8 + n).ok_or_else(|| bad("Kopf abgeschnitten"))?;
        let data = &bytes[8 + n..];
        let json: serde_json::Value = serde_json::from_slice(head).map_err(|e| AiError::Weights(format!("Kopf: {e}")))?;
        let obj = json.as_object().ok_or_else(|| bad("Kopf ist kein Objekt"))?;
        let mut map = HashMap::with_capacity(obj.len());
        for (name, v) in obj {
            if name == "__metadata__" {
                continue;
            }
            let dtype = v["dtype"].as_str().ok_or_else(|| bad("dtype"))?;
            let shape: Vec<usize> = v["shape"].as_array().ok_or_else(|| bad("shape"))?.iter().map(|x| x.as_u64().unwrap_or(0) as usize).collect();
            let off = v["data_offsets"].as_array().ok_or_else(|| bad("data_offsets"))?;
            let (a, b) = (off.first().and_then(|x| x.as_u64()).unwrap_or(0) as usize, off.get(1).and_then(|x| x.as_u64()).unwrap_or(0) as usize);
            let raw = data.get(a..b).ok_or_else(|| AiError::Weights(format!("Daten von {name} außerhalb der Datei")))?;
            let count: usize = shape.iter().product();
            let vals: Vec<f32> = match dtype {
                "F32" => raw.as_chunks::<4>().0.iter().map(|c| f32::from_le_bytes(*c)).collect(),
                "F16" => raw.as_chunks::<2>().0.iter().map(|c| f16_to_f32(u16::from_le_bytes(*c))).collect(),
                "BF16" => raw.as_chunks::<2>().0.iter().map(|c| f32::from_bits((u16::from_le_bytes(*c) as u32) << 16)).collect(),
                "I64" => raw.as_chunks::<8>().0.iter().map(|c| i64::from_le_bytes(*c) as f32).collect(),
                other => return Err(AiError::Weights(format!("Datentyp {other} von {name} nicht unterstützt"))),
            };
            if vals.len() != count {
                return Err(AiError::Weights(format!("Größe von {name} passt nicht")));
            }
            map.insert(name.clone(), Weight { shape, data: vals });
        }
        Ok(Weights { map })
    }

    /// Anzahl aller Werte (Speicherbedarf / 4).
    pub fn float_count(&self) -> usize {
        self.map.values().map(|w| w.data.len()).sum()
    }

    /// Gibt es den Tensor?
    pub fn has(&self, name: &str) -> bool {
        self.map.contains_key(name)
    }

    /// Tensor holen.
    pub fn get(&self, name: &str) -> Result<&Weight> {
        self.map.get(name).ok_or_else(|| AiError::Weights(format!("Gewicht {name} fehlt")))
    }

    /// Tensor herausnehmen (jedes Gewicht wird beim Aufbau genau einmal gebraucht).
    pub fn take(&mut self, name: &str) -> Result<Weight> {
        self.map.remove(name).ok_or_else(|| AiError::Weights(format!("Gewicht {name} fehlt")))
    }

    /// Tensor-Daten herausnehmen.
    pub fn vec(&mut self, name: &str) -> Result<Vec<f32>> {
        Ok(self.take(name)?.data)
    }

    /// Optionalen Tensor herausnehmen.
    pub fn opt(&mut self, name: &str) -> Option<Vec<f32>> {
        self.map.remove(name).map(|w| w.data)
    }

    /// Lineare Schicht `prefix.weight` `[out, in]`, `prefix.bias`.
    pub fn linear(&mut self, prefix: &str) -> Result<Linear> {
        let w = self.take(&format!("{prefix}.weight"))?;
        if w.shape.len() != 2 {
            return Err(AiError::Weights(format!("{prefix}.weight ist keine Matrix")));
        }
        Ok(Linear::new(w.data, self.opt(&format!("{prefix}.bias")), w.shape[0], w.shape[1]))
    }

    /// LayerNorm `prefix.weight`, `prefix.bias`.
    pub fn norm(&mut self, prefix: &str, eps: f32) -> Result<Norm> {
        Ok(Norm { w: self.vec(&format!("{prefix}.weight"))?, b: self.vec(&format!("{prefix}.bias"))?, eps })
    }

    /// Faltung `prefix.weight` `[O, C, k, k]`.
    pub fn conv(&mut self, prefix: &str, stride: usize, pad: usize, dil: usize) -> Result<Conv2d> {
        let w = self.take(&format!("{prefix}.weight"))?;
        if w.shape.len() != 4 || w.shape[2] != w.shape[3] {
            return Err(AiError::Weights(format!("{prefix}.weight ist keine quadratische Faltung")));
        }
        Ok(Conv2d { cout: w.shape[0], cin: w.shape[1], k: w.shape[2], w: w.data, b: self.opt(&format!("{prefix}.bias")), stride, pad, dil })
    }

    /// Faltung mit anschließender BatchNorm (eval), zusammengefaltet.
    pub fn conv_bn(&mut self, conv: &str, bn: &str, stride: usize, pad: usize, dil: usize) -> Result<Conv2d> {
        let mut c = self.conv(conv, stride, pad, dil)?;
        if self.has(&format!("{bn}.running_mean")) {
            c.fold_bn(
                &self.vec(&format!("{bn}.weight"))?,
                &self.vec(&format!("{bn}.bias"))?,
                &self.vec(&format!("{bn}.running_mean"))?,
                &self.vec(&format!("{bn}.running_var"))?,
                1e-5,
            );
        }
        Ok(c)
    }
}

/// Lineare Schicht (Gewichte vorab transponiert, `[cin, cout]`).
#[derive(Clone, Debug, Default)]
pub struct Linear {
    pub wt: Vec<f32>,
    pub b: Option<Vec<f32>>,
    pub cout: usize,
    pub cin: usize,
}

impl Linear {
    /// Aus PyTorch-Gewichten `w[cout, cin]`.
    pub fn new(w: Vec<f32>, b: Option<Vec<f32>>, cout: usize, cin: usize) -> Linear {
        Linear { wt: crate::nn::transpose(&w, cout, cin), b, cout, cin }
    }
    /// `x[n, cin]` → `[n, cout]`.
    pub fn forward(&self, x: &[f32], n: usize) -> Vec<f32> {
        crate::nn::linear_t(x, n, self.cin, &self.wt, self.b.as_deref(), self.cout)
    }
}

/// LayerNorm-Gewichte.
#[derive(Clone, Debug, Default)]
pub struct Norm {
    pub w: Vec<f32>,
    pub b: Vec<f32>,
    pub eps: f32,
}

impl Norm {
    /// Auf Zeilen (Token-Layout).
    pub fn rows(&self, x: &mut [f32]) {
        crate::nn::layer_norm_rows(x, self.w.len(), &self.w, &self.b, self.eps);
    }
    /// Auf Kanäle (`[C, HW]`).
    pub fn channels(&self, x: &mut [f32]) {
        crate::nn::layer_norm_channels(x, self.w.len(), &self.w, &self.b, self.eps);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_floats() {
        assert_eq!(f16_to_f32(0x3c00), 1.0);
        assert_eq!(f16_to_f32(0xc000), -2.0);
        assert_eq!(f16_to_f32(0x0000), 0.0);
        assert!((f16_to_f32(0x3555) - 0.333_251_95).abs() < 1e-7);
        assert!((f16_to_f32(0x0001) - 5.960_464_5e-8).abs() < 1e-12);
    }

    #[test]
    fn parse_minimal() {
        let head = br#"{"a":{"dtype":"F32","shape":[2],"data_offsets":[0,8]},"__metadata__":{"format":"pt"}}"#;
        let mut b = (head.len() as u64).to_le_bytes().to_vec();
        b.extend_from_slice(head);
        b.extend_from_slice(&1.5f32.to_le_bytes());
        b.extend_from_slice(&(-2.0f32).to_le_bytes());
        let mut w = Weights::parse(&b).unwrap_or_default();
        assert_eq!(w.vec("a").unwrap_or_default(), vec![1.5, -2.0]);
        assert!(Weights::parse(&b[..10]).is_err());
    }
}
