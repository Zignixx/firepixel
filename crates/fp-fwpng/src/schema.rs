//! Hilfen für die Abbildung Tag-Baum ↔ Modell mit verlustfreier Rundreise.
//!
//! [`Rd`] liest die Kinder eines Containers: Jeder Zugriff „verbraucht“ Knoten; am Ende liefert
//! [`Rd::finish`] die Rundreise-Daten ([`Extra`]): nicht verbrauchte Knoten, die Reihenfolge aller
//! Kinder und Schatten für verlustbehaftete Werte.
//!
//! Regeln, die Leser und Schreiber gemeinsam einhalten:
//! * Ein Einzelfeld wird nur verbraucht, wenn sein Schlüssel genau einmal vorkommt. Kommt ein Schlüssel
//!   unerwartet mehrfach vor, bleiben alle Vorkommen roh erhalten.
//! * Der Schreiber ([`Wr`]) überspringt ein modelliertes Feld, wenn derselbe Schlüssel unter den rohen
//!   Knoten steht (dann ist das Rohe maßgeblich).
//! * Reihenfolge: Ist die Ordnung bekannt (aus der Datei gelesen), ordnet [`Wr::finish`] die Ausgabe
//!   danach; neue Felder kommen hinter ihren Vorgänger in der Original-Reihenfolge.
//! * Vorgabewerte: Felder, die das Original nur bei Abweichung schreibt, schreibt der Schreiber, wenn der
//!   Wert abweicht oder der Tag in der gelesenen Ordnung vorkam. Bei neu erzeugten Objekten schreibt er
//!   zusätzlich alle Felder, die das Original immer schreibt.

use crate::tree::format_float;
use fp_color::{BlendMode, Opacity};
use fp_geom::{Matrix3, Point, Rect};
use fp_model::{Extra, RawNode, RawValue, TagKey};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

/// Kurzschreibweise für Schlüssel-Konstanten.
#[macro_export]
macro_rules! k {
    ($s:literal) => {
        fp_model::TagKey::new($s)
    };
}

/// Interniert Reihenfolgen (viele Container haben dieselbe Kinderfolge).
#[derive(Default)]
pub struct Interner {
    map: HashMap<Vec<TagKey>, Arc<[TagKey]>>,
}

impl Interner {
    pub fn get(&mut self, keys: Vec<TagKey>) -> Arc<[TagKey]> {
        if let Some(a) = self.map.get(&keys) {
            return Arc::clone(a);
        }
        let a: Arc<[TagKey]> = Arc::from(keys.clone());
        self.map.insert(keys, Arc::clone(&a));
        a
    }
}

/// Leser über die Kinder eines Containers.
pub struct Rd<'a> {
    nodes: &'a [RawNode],
    used: Vec<bool>,
    shadow: Vec<RawNode>,
}

impl<'a> Rd<'a> {
    pub fn new(nodes: &'a [RawNode]) -> Self {
        Rd { nodes, used: vec![false; nodes.len()], shadow: Vec::new() }
    }

    /// Alle Kinder (auch verbrauchte).
    pub fn all(&self) -> &'a [RawNode] {
        self.nodes
    }

    fn count(&self, key: TagKey) -> usize {
        self.nodes.iter().filter(|n| n.key == key).count()
    }

    /// Kommt der Schlüssel vor?
    pub fn has(&self, key: TagKey) -> bool {
        self.nodes.iter().any(|n| n.key == key)
    }

    /// Verbraucht das einzige Vorkommen des Schlüssels; bei 0 oder mehreren Vorkommen `None`.
    pub fn take(&mut self, key: TagKey) -> Option<&'a RawNode> {
        let mut found = None;
        for (i, n) in self.nodes.iter().enumerate() {
            if n.key == key {
                if found.is_some() {
                    return None;
                }
                found = Some(i);
            }
        }
        let i = found?;
        if self.used[i] {
            return None;
        }
        self.used[i] = true;
        Some(&self.nodes[i])
    }

    /// Verbraucht alle Vorkommen des Schlüssels.
    pub fn take_all(&mut self, key: TagKey) -> Vec<&'a RawNode> {
        let mut v = Vec::new();
        for (i, n) in self.nodes.iter().enumerate() {
            if n.key == key && !self.used[i] {
                self.used[i] = true;
                v.push(n);
            }
        }
        v
    }

    /// Verbraucht die ersten `max` Vorkommen (für bewusst doppelte Tags wie `FRCi`).
    pub fn take_n(&mut self, key: TagKey, max: usize) -> Vec<&'a RawNode> {
        let mut v = Vec::new();
        if self.count(key) > max {
            return v;
        }
        for (i, n) in self.nodes.iter().enumerate() {
            if v.len() >= max {
                break;
            }
            if n.key == key && !self.used[i] {
                self.used[i] = true;
                v.push(n);
            }
        }
        v
    }

    /// Markiert einen Knoten als verbraucht (für Gruppenleser wie `TFSv`).
    pub fn mark(&mut self, index: usize) {
        if let Some(u) = self.used.get_mut(index) {
            *u = true;
        }
    }

    pub fn is_used(&self, index: usize) -> bool {
        self.used.get(index).copied().unwrap_or(true)
    }

    pub fn int(&mut self, key: TagKey) -> Option<u32> {
        let n = self.peek_single(key)?;
        let v = n.as_int()?;
        self.take(key);
        Some(v)
    }

    pub fn i32(&mut self, key: TagKey) -> Option<i32> {
        self.int(key).map(|v| v as i32)
    }

    pub fn f64(&mut self, key: TagKey) -> Option<f64> {
        let n = self.peek_single(key)?;
        let v = n.as_num()?;
        self.take(key);
        Some(v)
    }

    /// Float als `f32`; ändert sich die Formatierung dadurch, wird das Original als Schatten gemerkt.
    pub fn f32(&mut self, key: TagKey) -> Option<f32> {
        let n = self.peek_single(key)?;
        let v = n.as_num()?;
        self.take(key);
        let x = v as f32;
        if format_float(x as f64) != format_float(v) {
            self.shadow.push(n.clone());
        }
        Some(x)
    }

    pub fn bool(&mut self, key: TagKey) -> Option<bool> {
        let n = self.peek_single(key)?;
        let v = n.as_bool()?;
        self.take(key);
        Some(v)
    }

    /// String (ungültiges UTF-16 bleibt roh und liefert `None`).
    pub fn str(&mut self, key: TagKey) -> Option<String> {
        let n = self.peek_single(key)?;
        let v = n.as_str()?.to_string();
        self.take(key);
        Some(v)
    }

    pub fn bytes(&mut self, key: TagKey) -> Option<Vec<u8>> {
        let n = self.peek_single(key)?;
        let RawValue::Bytes(b) = &n.value else { return None };
        self.take(key);
        Some(b.clone())
    }

    /// Kinder einer Liste (verbraucht den Listenknoten).
    pub fn list(&mut self, key: TagKey) -> Option<&'a [RawNode]> {
        let n = self.peek_single(key)?;
        let RawValue::List(v) = &n.value else { return None };
        self.take(key);
        Some(v)
    }

    /// Liste, deren Kinder alle den Schlüssel `item` haben; sonst bleibt sie roh (`None`).
    pub fn strict_list(&mut self, key: TagKey, item: TagKey) -> Option<&'a [RawNode]> {
        let n = self.peek_single(key)?;
        let RawValue::List(v) = &n.value else { return None };
        if v.iter().any(|c| c.key != item) {
            return None;
        }
        self.take(key);
        Some(v)
    }

    fn peek_single(&self, key: TagKey) -> Option<&'a RawNode> {
        let mut found = None;
        for (i, n) in self.nodes.iter().enumerate() {
            if n.key == key {
                if found.is_some() {
                    return None;
                }
                found = Some(i);
            }
        }
        let i = found?;
        if self.used[i] { None } else { Some(&self.nodes[i]) }
    }

    /// Punkt aus zwei Floats; nur wenn beide vorhanden sind.
    pub fn point(&mut self, kx: TagKey, ky: TagKey) -> Option<Point> {
        if self.peek_single(kx)?.as_num().is_none() || self.peek_single(ky)?.as_num().is_none() {
            return None;
        }
        Some(Point::new(self.f32(kx)?, self.f32(ky)?))
    }

    /// Rechteck `LFTf TOPf RITf BOTf` (bzw. andere Schlüssel); nur wenn alle vier vorhanden sind.
    pub fn rect4(&mut self, keys: [TagKey; 4]) -> Option<Rect> {
        for k in keys {
            self.peek_single(k)?.as_num()?;
        }
        Some(Rect::new(self.f32(keys[0])?, self.f32(keys[1])?, self.f32(keys[2])?, self.f32(keys[3])?))
    }

    /// Deckkraft `OPAi` (Promille) als Byte; nicht darstellbare Werte bleiben als Schatten erhalten.
    pub fn opacity(&mut self, key: TagKey) -> Option<Opacity> {
        let n = self.peek_single(key)?;
        let p = n.as_int()?;
        self.take(key);
        let o = Opacity::from_permille(p);
        if o.to_permille() != p {
            self.shadow.push(n.clone());
        }
        Some(o)
    }

    /// Mischmodus `BLDi`; der Wert 0x37 gilt wie beim Original als Normal (Original als Schatten).
    pub fn blend(&mut self, key: TagKey) -> Option<BlendMode> {
        let n = self.peek_single(key)?;
        let v = n.as_int()?;
        self.take(key);
        let b = if v == 0x37 { 0 } else { v as u8 };
        if b as u32 != v {
            self.shadow.push(n.clone());
        }
        Some(BlendMode(b))
    }

    /// Matrix `MTXv`/`ATMv` mit `M00 … M22`; Abweichungen von der kanonischen Form als Schatten.
    pub fn matrix(&mut self, key: TagKey) -> Option<Matrix3> {
        let n = self.peek_single(key)?;
        let RawValue::List(kids) = &n.value else { return None };
        self.take(key);
        let m = decode_matrix(kids);
        if encode_matrix(key, &m) != *n {
            self.shadow.push(n.clone());
        }
        Some(m)
    }

    /// Abschluss: Rundreise-Daten.
    pub fn finish(self, interner: &mut Interner) -> Extra {
        let order = interner.get(self.nodes.iter().map(|n| n.key).collect());
        let nodes: Vec<RawNode> =
            self.nodes.iter().zip(self.used.iter()).filter(|(_, u)| !**u).map(|(n, _)| n.clone()).collect();
        Extra { nodes, order: Some(order), shadow: self.shadow }
    }
}

const MATRIX_KEYS: [[TagKey; 3]; 3] = [
    [TagKey::new(b"M00f"), TagKey::new(b"M01f"), TagKey::new(b"M02f")],
    [TagKey::new(b"M10f"), TagKey::new(b"M11f"), TagKey::new(b"M12f")],
    [TagKey::new(b"M20f"), TagKey::new(b"M21f"), TagKey::new(b"M22f")],
];

/// Matrix aus `M00 … M22` (fehlende Einträge = Einheitsmatrix).
pub fn decode_matrix(kids: &[RawNode]) -> Matrix3 {
    let mut m = Matrix3::IDENTITY;
    for (r, row) in MATRIX_KEYS.iter().enumerate() {
        for (c, k) in row.iter().enumerate() {
            if let Some(v) = kids.iter().find(|n| n.key == *k).and_then(|n| n.as_num()) {
                m.m[r][c] = v as f32;
            }
        }
    }
    m
}

/// Matrix als Knoten (alle neun Einträge in Original-Reihenfolge).
pub fn encode_matrix(key: TagKey, m: &Matrix3) -> RawNode {
    let mut kids = Vec::with_capacity(9);
    for (r, row) in MATRIX_KEYS.iter().enumerate() {
        for (c, k) in row.iter().enumerate() {
            kids.push(RawNode::num(*k, m.m[r][c] as f64));
        }
    }
    RawNode::list(key, kids)
}

/// Schreiber für die Kinder eines Containers.
pub struct Wr<'e> {
    out: Vec<RawNode>,
    extra: &'e Extra,
    shadow_used: Vec<bool>,
}

impl<'e> Wr<'e> {
    pub fn new(extra: &'e Extra) -> Self {
        Wr { out: Vec::with_capacity(16), extra, shadow_used: vec![false; extra.shadow.len()] }
    }

    /// Neu erzeugtes Objekt (keine gelesene Reihenfolge)?
    pub fn is_new(&self) -> bool {
        self.extra.order.is_none()
    }

    /// Kam der Schlüssel in der gelesenen Reihenfolge vor?
    pub fn had(&self, key: TagKey) -> bool {
        self.extra.order.as_ref().is_some_and(|o| o.contains(&key))
    }

    /// Wie oft kam der Schlüssel in der gelesenen Reihenfolge vor?
    pub fn had_count(&self, key: TagKey) -> usize {
        self.extra.order.as_ref().map(|o| o.iter().filter(|k| **k == key).count()).unwrap_or(0)
    }

    /// Soll ein Feld mit Vorgabewert geschrieben werden? (`always` = das Original schreibt es immer)
    pub fn want(&self, key: TagKey, is_default: bool, always: bool) -> bool {
        !is_default || self.had(key) || (always && self.is_new())
    }

    /// Fügt einen Knoten an (übersprungen, wenn der Schlüssel roh vorliegt). Passt ein Schatten zum
    /// gleichen Wert (`same`), wird der Schatten geschrieben.
    pub fn push_checked(&mut self, node: RawNode, same: impl Fn(&RawNode) -> bool) {
        if self.extra.has_raw(node.key) {
            return;
        }
        for (i, s) in self.extra.shadow.iter().enumerate() {
            if !self.shadow_used[i] && s.key == node.key && same(s) {
                self.shadow_used[i] = true;
                self.out.push(s.clone());
                return;
            }
        }
        self.out.push(node);
    }

    /// Fügt einen Knoten an (übersprungen, wenn der Schlüssel roh vorliegt).
    pub fn push(&mut self, node: RawNode) {
        if self.extra.has_raw(node.key) {
            return;
        }
        self.out.push(node);
    }

    pub fn int(&mut self, key: TagKey, v: u32) {
        self.push(RawNode::int(key, v));
    }
    pub fn i32(&mut self, key: TagKey, v: i32) {
        self.push(RawNode::int(key, v as u32));
    }
    pub fn f64(&mut self, key: TagKey, v: f64) {
        self.push(RawNode::num(key, v));
    }
    pub fn f32(&mut self, key: TagKey, v: f32) {
        self.push_checked(RawNode::num(key, v as f64), |s| s.as_num().map(|x| x as f32) == Some(v));
    }
    pub fn bool(&mut self, key: TagKey, v: bool) {
        self.push(RawNode::bool(key, v));
    }
    pub fn str(&mut self, key: TagKey, v: &str) {
        self.push(RawNode::str(key, v));
    }
    pub fn list(&mut self, key: TagKey, kids: Vec<RawNode>) {
        self.push(RawNode::list(key, kids));
    }
    pub fn node(&mut self, n: RawNode) {
        self.push(n);
    }

    pub fn opt_int(&mut self, key: TagKey, v: Option<u32>) {
        if let Some(v) = v {
            self.int(key, v);
        }
    }
    pub fn opt_i32(&mut self, key: TagKey, v: Option<i32>) {
        if let Some(v) = v {
            self.i32(key, v);
        }
    }
    pub fn opt_f32(&mut self, key: TagKey, v: Option<f32>) {
        if let Some(v) = v {
            self.f32(key, v);
        }
    }
    pub fn opt_f64(&mut self, key: TagKey, v: Option<f64>) {
        if let Some(v) = v {
            self.f64(key, v);
        }
    }
    pub fn opt_bool(&mut self, key: TagKey, v: Option<bool>) {
        if let Some(v) = v {
            self.bool(key, v);
        }
    }
    pub fn opt_str(&mut self, key: TagKey, v: Option<&str>) {
        if let Some(v) = v {
            self.str(key, v);
        }
    }

    /// Ganzzahl mit Vorgabe (`always` = das Original schreibt sie immer).
    pub fn int_def(&mut self, key: TagKey, v: u32, default: u32, always: bool) {
        if self.want(key, v == default, always) {
            self.int(key, v);
        }
    }
    pub fn i32_def(&mut self, key: TagKey, v: i32, default: i32, always: bool) {
        if self.want(key, v == default, always) {
            self.i32(key, v);
        }
    }
    pub fn f32_def(&mut self, key: TagKey, v: f32, default: f32, always: bool) {
        if self.want(key, v == default, always) {
            self.f32(key, v);
        }
    }
    pub fn bool_def(&mut self, key: TagKey, v: bool, default: bool, always: bool) {
        if self.want(key, v == default, always) {
            self.bool(key, v);
        }
    }
    pub fn str_def(&mut self, key: TagKey, v: &str, default: &str, always: bool) {
        if self.want(key, v == default, always) {
            self.str(key, v);
        }
    }

    /// Punkt als zwei Floats.
    pub fn point(&mut self, kx: TagKey, ky: TagKey, p: Point) {
        self.f32(kx, p.x);
        self.f32(ky, p.y);
    }

    /// Rechteck als vier Floats.
    pub fn rect4(&mut self, keys: [TagKey; 4], r: Rect) {
        self.f32(keys[0], r.left);
        self.f32(keys[1], r.top);
        self.f32(keys[2], r.right);
        self.f32(keys[3], r.bottom);
    }

    /// Punkt mit Vorgabe je Koordinate (`always` = das Original schreibt die Tags immer).
    pub fn point_def(&mut self, kx: TagKey, ky: TagKey, p: Point, default: Point, always: bool) {
        self.f32_def(kx, p.x, default.x, always);
        self.f32_def(ky, p.y, default.y, always);
    }

    /// Rechteck mit Vorgabe je Kante.
    pub fn rect4_def(&mut self, keys: [TagKey; 4], r: Rect, default: Rect, always: bool) {
        self.f32_def(keys[0], r.left, default.left, always);
        self.f32_def(keys[1], r.top, default.top, always);
        self.f32_def(keys[2], r.right, default.right, always);
        self.f32_def(keys[3], r.bottom, default.bottom, always);
    }

    /// Deckkraft (`OPAi`): Das Original schreibt sie nur ≠ 1000.
    pub fn opacity(&mut self, key: TagKey, o: Opacity) {
        if self.want(key, o.0 == 255, false) {
            self.push_checked(RawNode::int(key, o.to_permille()), |s| {
                s.as_int().map(Opacity::from_permille) == Some(o)
            });
        }
    }

    /// Mischmodus (`BLDi`): nur ≠ 0.
    pub fn blend(&mut self, key: TagKey, b: BlendMode) {
        if self.want(key, b.0 == 0, false) {
            self.push_checked(RawNode::int(key, b.0 as u32), |s| {
                s.as_int().map(|v| if v == 0x37 { 0 } else { v as u8 }) == Some(b.0)
            });
        }
    }

    /// Matrix.
    pub fn matrix(&mut self, key: TagKey, m: &Matrix3) {
        let node = encode_matrix(key, m);
        let mm = *m;
        self.push_checked(node, |s| decode_matrix(s.children()) == mm);
    }

    /// Ergebnis in Dateireihenfolge (siehe Modul-Doku).
    pub fn finish(self) -> Vec<RawNode> {
        arrange(self.out, self.extra)
    }
}

/// Wie [`arrange`], aber für **Listen**, deren Reihenfolge Bedeutung hat (Elemente = Stapelfolge,
/// Ebenen, Status, Konturen, Symbole): Ohne Rohknoten bleibt die Reihenfolge des Modells, denn die
/// gelesene Schlüsselfolge passt nach einer Bearbeitung (Einfügen, Duplizieren, Anordnen) nicht mehr
/// zur Liste (Befund EB: sonst wanderte eine Textkopie unter die Pfade). Beim unveränderten Lesen
/// stimmt die Modellreihenfolge ohnehin mit der Datei überein.
pub fn arrange_list(modeled: Vec<RawNode>, extra: &Extra) -> Vec<RawNode> {
    if extra.nodes.is_empty() {
        return modeled;
    }
    arrange(modeled, extra)
}

/// Ordnet modellierte und rohe Knoten nach der gelesenen Reihenfolge.
pub fn arrange(modeled: Vec<RawNode>, extra: &Extra) -> Vec<RawNode> {
    let Some(order) = &extra.order else {
        let mut v = modeled;
        v.extend(extra.nodes.iter().cloned());
        return v;
    };
    // Schnellweg: Schlüsselfolge identisch und keine Rohknoten.
    if extra.nodes.is_empty() && modeled.len() == order.len() && modeled.iter().zip(order.iter()).all(|(n, k)| n.key == *k) {
        return modeled;
    }
    let mut mq: HashMap<TagKey, VecDeque<usize>> = HashMap::new();
    for (i, n) in modeled.iter().enumerate() {
        mq.entry(n.key).or_default().push_back(i);
    }
    let mut xq: HashMap<TagKey, VecDeque<usize>> = HashMap::new();
    for (i, n) in extra.nodes.iter().enumerate() {
        xq.entry(n.key).or_default().push_back(i);
    }
    enum S {
        M(usize),
        X(usize),
    }
    let mut seq: Vec<S> = Vec::with_capacity(order.len() + 4);
    let mut pos_of_m: Vec<Option<usize>> = vec![None; modeled.len()];
    let mut x_used = vec![false; extra.nodes.len()];
    for k in order.iter() {
        if let Some(i) = mq.get_mut(k).and_then(|q| q.pop_front()) {
            pos_of_m[i] = Some(seq.len());
            seq.push(S::M(i));
        } else if let Some(i) = xq.get_mut(k).and_then(|q| q.pop_front()) {
            x_used[i] = true;
            seq.push(S::X(i));
        }
    }
    // Übrige modellierte Knoten hinter ihren Vorgänger einfügen.
    let mut after: Vec<Vec<usize>> = vec![Vec::new(); seq.len() + 1];
    let mut last_slot = 0usize; // 0 = vorne, p+1 = hinter seq[p]
    for (i, pos) in pos_of_m.iter().enumerate() {
        match pos {
            Some(p) => last_slot = p + 1,
            None => after[last_slot].push(i),
        }
    }
    let mut m_slots: Vec<Option<RawNode>> = modeled.into_iter().map(Some).collect();
    let mut out = Vec::with_capacity(m_slots.len() + extra.nodes.len());
    for i in &after[0] {
        if let Some(n) = m_slots[*i].take() {
            out.push(n);
        }
    }
    for (p, s) in seq.iter().enumerate() {
        match s {
            S::M(i) => {
                if let Some(n) = m_slots[*i].take() {
                    out.push(n);
                }
            }
            S::X(i) => out.push(extra.nodes[*i].clone()),
        }
        for i in &after[p + 1] {
            if let Some(n) = m_slots[*i].take() {
                out.push(n);
            }
        }
    }
    for (i, n) in extra.nodes.iter().enumerate() {
        if !x_used[i] {
            out.push(n.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(k: &[u8; 4], v: u32) -> RawNode {
        RawNode::int(TagKey::new(k), v)
    }

    #[test]
    fn reader_consumes_and_keeps_order() {
        let kids = vec![n(b"AAAi", 1), n(b"UNKi", 9), n(b"BBBi", 2), n(b"DUPi", 3), n(b"DUPi", 4)];
        let mut ii = Interner::default();
        let mut r = Rd::new(&kids);
        assert_eq!(r.int(k!(b"BBBi")), Some(2));
        assert_eq!(r.int(k!(b"AAAi")), Some(1));
        assert_eq!(r.int(k!(b"DUPi")), None, "doppelt → roh");
        let ex = r.finish(&mut ii);
        assert_eq!(ex.nodes.len(), 3);
        // Schreiber in anderer Reihenfolge → Original wird wiederhergestellt.
        let mut w = Wr::new(&ex);
        w.int(k!(b"BBBi"), 2);
        w.int(k!(b"AAAi"), 1);
        w.int(k!(b"DUPi"), 7); // übersprungen, weil roh vorhanden
        assert_eq!(w.finish(), kids);
    }

    #[test]
    fn new_fields_follow_predecessor() {
        let kids = vec![n(b"AAAi", 1), n(b"CCCi", 3)];
        let mut ii = Interner::default();
        let mut r = Rd::new(&kids);
        r.int(k!(b"AAAi"));
        r.int(k!(b"CCCi"));
        let ex = r.finish(&mut ii);
        let mut w = Wr::new(&ex);
        w.int(k!(b"AAAi"), 1);
        w.int(k!(b"BBBi"), 2);
        w.int(k!(b"CCCi"), 3);
        let keys: Vec<String> = w.finish().iter().map(|n| n.key.as_string()).collect();
        assert_eq!(keys, vec!["AAAi", "BBBi", "CCCi"]);
    }

    #[test]
    fn shadows_preserve_lossy_values() {
        let kids = vec![
            RawNode::int(k!(b"OPAi"), 334),
            RawNode::int(k!(b"BLDi"), 0x37),
            RawNode::num(k!(b"XLCf"), 12345678.9),
        ];
        let mut ii = Interner::default();
        let mut r = Rd::new(&kids);
        let o = r.opacity(k!(b"OPAi")).unwrap();
        let b = r.blend(k!(b"BLDi")).unwrap();
        let x = r.f32(k!(b"XLCf")).unwrap();
        assert_eq!(b, BlendMode(0));
        let ex = r.finish(&mut ii);
        let mut w = Wr::new(&ex);
        w.opacity(k!(b"OPAi"), o);
        w.blend(k!(b"BLDi"), b);
        w.f32(k!(b"XLCf"), x);
        assert_eq!(w.finish(), kids);
    }

    #[test]
    fn defaults_and_new_objects() {
        let ex = Extra::default();
        let mut w = Wr::new(&ex);
        w.bool_def(k!(b"VISb"), true, true, false);
        w.bool_def(k!(b"LCKb"), false, false, true);
        let keys: Vec<String> = w.finish().iter().map(|n| n.key.as_string()).collect();
        assert_eq!(keys, vec!["LCKb"]);
    }
}
