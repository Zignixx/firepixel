//! Serde-Anbindung der Grundtypen aus `fp-geom` und `fp-color`.
//!
//! Die Grundtypen selbst sind (noch) nicht serde-fähig; die Module hier werden über
//! `#[serde(with = "…")]` an den Feldern benutzt. Formate: Punkt `[x, y]`, Rechteck
//! `[links, oben, rechts, unten]`, Matrix 9 Zahlen zeilenweise, Farbe als ARGB-Zahl, Deckkraft als Byte,
//! Mischmodus als Zahl (`BLDi`).

use fp_color::{Argb, BlendMode, Opacity};
use fp_geom::{Matrix3, Point, Rect};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub mod point {
    use super::*;
    pub fn serialize<S: Serializer>(p: &Point, s: S) -> Result<S::Ok, S::Error> {
        [p.x, p.y].serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Point, D::Error> {
        let [x, y] = <[f32; 2]>::deserialize(d)?;
        Ok(Point::new(x, y))
    }
}

pub mod opt_point {
    use super::*;
    pub fn serialize<S: Serializer>(p: &Option<Point>, s: S) -> Result<S::Ok, S::Error> {
        p.map(|p| [p.x, p.y]).serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Point>, D::Error> {
        Ok(Option::<[f32; 2]>::deserialize(d)?.map(|[x, y]| Point::new(x, y)))
    }
}

pub mod vec_point {
    use super::*;
    pub fn serialize<S: Serializer>(v: &[Point], s: S) -> Result<S::Ok, S::Error> {
        let a: Vec<[f32; 2]> = v.iter().map(|p| [p.x, p.y]).collect();
        a.serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Point>, D::Error> {
        Ok(Vec::<[f32; 2]>::deserialize(d)?.into_iter().map(|[x, y]| Point::new(x, y)).collect())
    }
}

pub mod rect {
    use super::*;
    pub fn serialize<S: Serializer>(r: &Rect, s: S) -> Result<S::Ok, S::Error> {
        [r.left, r.top, r.right, r.bottom].serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Rect, D::Error> {
        let [l, t, r, b] = <[f32; 4]>::deserialize(d)?;
        Ok(Rect::new(l, t, r, b))
    }
}

pub mod opt_rect {
    use super::*;
    pub fn serialize<S: Serializer>(r: &Option<Rect>, s: S) -> Result<S::Ok, S::Error> {
        r.map(|r| [r.left, r.top, r.right, r.bottom]).serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Rect>, D::Error> {
        Ok(Option::<[f32; 4]>::deserialize(d)?.map(|[l, t, r, b]| Rect::new(l, t, r, b)))
    }
}

pub mod matrix {
    use super::*;
    pub fn serialize<S: Serializer>(m: &Matrix3, s: S) -> Result<S::Ok, S::Error> {
        let a = [m.m[0][0], m.m[0][1], m.m[0][2], m.m[1][0], m.m[1][1], m.m[1][2], m.m[2][0], m.m[2][1], m.m[2][2]];
        a.serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Matrix3, D::Error> {
        let a = <[f32; 9]>::deserialize(d)?;
        Ok(Matrix3 { m: [[a[0], a[1], a[2]], [a[3], a[4], a[5]], [a[6], a[7], a[8]]] })
    }
}

pub mod opt_matrix {
    use super::*;
    #[derive(Serialize, Deserialize)]
    struct M(#[serde(with = "super::matrix")] Matrix3);
    pub fn serialize<S: Serializer>(m: &Option<Matrix3>, s: S) -> Result<S::Ok, S::Error> {
        m.map(M).serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Matrix3>, D::Error> {
        Ok(Option::<M>::deserialize(d)?.map(|m| m.0))
    }
}

pub mod argb {
    use super::*;
    pub fn serialize<S: Serializer>(c: &Argb, s: S) -> Result<S::Ok, S::Error> {
        c.0.serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Argb, D::Error> {
        Ok(Argb(u32::deserialize(d)?))
    }
}

pub mod opt_argb {
    use super::*;
    pub fn serialize<S: Serializer>(c: &Option<Argb>, s: S) -> Result<S::Ok, S::Error> {
        c.map(|c| c.0).serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Argb>, D::Error> {
        Ok(Option::<u32>::deserialize(d)?.map(Argb))
    }
}

pub mod vec_argb {
    use super::*;
    pub fn serialize<S: Serializer>(v: &[Argb], s: S) -> Result<S::Ok, S::Error> {
        let a: Vec<u32> = v.iter().map(|c| c.0).collect();
        a.serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Argb>, D::Error> {
        Ok(Vec::<u32>::deserialize(d)?.into_iter().map(Argb).collect())
    }
}

pub mod opacity {
    use super::*;
    pub fn serialize<S: Serializer>(o: &Opacity, s: S) -> Result<S::Ok, S::Error> {
        o.0.serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Opacity, D::Error> {
        Ok(Opacity(u8::deserialize(d)?))
    }
}

pub mod blend {
    use super::*;
    pub fn serialize<S: Serializer>(b: &BlendMode, s: S) -> Result<S::Ok, S::Error> {
        b.0.serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<BlendMode, D::Error> {
        Ok(BlendMode(u8::deserialize(d)?))
    }
}

/// Hilfsfunktionen für `skip_serializing_if`.
pub fn is_false(b: &bool) -> bool {
    !*b
}
pub fn is_true(b: &bool) -> bool {
    *b
}
pub fn default_true() -> bool {
    true
}
