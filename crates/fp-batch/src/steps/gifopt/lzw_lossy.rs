//! GIF-LZW-Kodierer mit optionalem Verlust (SPEC-BATCH §5.6 Punkt 5), eigene Umsetzung des bekannten
//! Prinzips: Beim Verlängern eines Wörterbuch-Eintrags wird für das nächste Pixel nicht nur der exakte
//! Index akzeptiert, sondern jeder Folge-Index im Wörterbuch, dessen Farbe nah genug an der
//! gewünschten Farbe liegt (Schwelle aus `lossy`, Abstand mit stärker gewichteter Helligkeit, siehe
//! [`LOSSY_L_WEIGHT`]). Das ergibt längere Codes, also weniger Bits. Der Fehler einer Ersetzung wird an
//! das rechte und das untere Nachbarpixel weitergegeben.
//!
//! Auch ohne Verlust nutzt der Kodierer gleichwertige Möglichkeiten: Ein unverändertes Pixel darf
//! transparent (Vorbild bleibt sichtbar) **oder** in genau der sichtbaren Farbe kodiert werden – je
//! nachdem, was den laufenden Code verlängert.

use super::color::{Lab, dist2};

/// Plan eines Pixels für den Kodierer.
#[derive(Clone, Copy, Debug)]
pub struct PlanPx {
    /// Vorgabe-Index (Ergebnis von Palette + Dithering bzw. Transparenz).
    pub default: u8,
    /// Darf durch einen anderen Index ersetzt werden.
    pub flex: bool,
    /// Transparenz ist gleichwertig (unverändertes Pixel).
    pub can_transp: bool,
    /// Gewünschte Farbe (Oklab) für die Abstandsprüfung.
    pub reference: Lab,
    /// Index, der durch Verlust nicht gewählt werden darf (die bisher sichtbare Farbe eines geänderten
    /// Pixels – sonst würde eine kleine Änderung, z. B. eine Blende, ganz verschluckt); 256 = keiner.
    pub forbid: u16,
}

impl PlanPx {
    /// Fester Index.
    pub fn fixed(i: u8) -> PlanPx {
        PlanPx {
            default: i,
            flex: false,
            can_transp: false,
            reference: [0.0; 3],
            forbid: 256,
        }
    }
}

/// Einstellungen eines Kodiervorgangs.
pub struct LzwParams<'a> {
    pub min_code_size: u8,
    pub palette: &'a [Lab],
    pub transparent: Option<u8>,
    /// Höchster Oklab-Abstand einer Ersetzung (0 = nur gleiche Farbe).
    pub threshold: f32,
    /// Breite des Rechtecks (für die Fehlerweitergabe nach unten).
    pub width: usize,
}

/// Gewicht der Helligkeit in der Verlust-Toleranz (BATCH-8b): Ersetzungen dürfen den Farbton deutlich
/// stärker verschieben als die Helligkeit (Helligkeitsfehler sieht man als Struktur, kleine
/// Farbtonfehler in feinem Korn kaum). Gemessen: bei gleicher Größe deutlich bessere SSIM.
pub const LOSSY_L_WEIGHT: f32 = 3.0;

const NONE: u16 = u16::MAX;
const MAX_CODES: usize = 4096;

struct BitWriter {
    out: Vec<u8>,
    acc: u32,
    n: u32,
}

impl BitWriter {
    fn put(&mut self, code: u16, width: u8) {
        self.acc |= (code as u32) << self.n;
        self.n += width as u32;
        while self.n >= 8 {
            self.out.push(self.acc as u8);
            self.acc >>= 8;
            self.n -= 8;
        }
    }
    fn finish(mut self) -> Vec<u8> {
        if self.n > 0 {
            self.out.push(self.acc as u8);
        }
        self.out
    }
}

/// Wörterbuch als Trie (erstes Kind / nächster Geschwister-Eintrag).
struct Trie {
    first: Vec<u16>,
    sibling: Vec<u16>,
    index: Vec<u8>,
}

impl Trie {
    fn new() -> Trie {
        Trie {
            first: vec![NONE; MAX_CODES],
            sibling: vec![NONE; MAX_CODES],
            index: vec![0; MAX_CODES],
        }
    }
    fn reset(&mut self) {
        self.first.iter_mut().for_each(|v| *v = NONE);
    }
    fn add(&mut self, parent: u16, idx: u8, code: u16) {
        self.index[code as usize] = idx;
        self.first[code as usize] = NONE;
        self.sibling[code as usize] = self.first[parent as usize];
        self.first[parent as usize] = code;
    }
}

/// Kodiert die Pixel eines Rechtecks. Ergebnis: gepackte LZW-Codes (ohne Mindest-Codelänge und ohne
/// Unterblöcke) und die tatsächlich gewählten Indizes je Pixel.
pub fn encode(plan: &[PlanPx], p: &LzwParams) -> (Vec<u8>, Vec<u8>) {
    let mcs = p.min_code_size.clamp(2, 8);
    let clear = 1u16 << mcs;
    let eoi = clear + 1;
    let mut w = BitWriter {
        out: Vec::with_capacity(plan.len() / 2 + 16),
        acc: 0,
        n: 0,
    };
    let mut chosen = vec![0u8; plan.len()];
    let mut trie = Trie::new();
    let mut width = mcs + 1;
    let mut next = eoi + 1;
    w.put(clear, width);
    if plan.is_empty() {
        w.put(eoi, width);
        return (w.finish(), chosen);
    }
    let lossy = p.threshold > 0.0;
    let thr2 = if lossy {
        p.threshold * p.threshold
    } else {
        1e-10
    };
    let row = p.width.max(1);
    // Fehlerweitergabe (nur mit Verlust)
    let mut carry_right = [0f32; 3];
    let mut carry_down = if lossy {
        vec![[0f32; 3]; row]
    } else {
        Vec::new()
    };

    let accept = |px: &PlanPx, cand: u8, carry: &Lab| -> Option<f32> {
        if cand == px.default {
            return Some(0.0);
        }
        if !px.flex || cand as u16 == px.forbid {
            return None;
        }
        if Some(cand) == p.transparent {
            return if px.can_transp { Some(0.0) } else { None };
        }
        let c = p.palette.get(cand as usize)?;
        if px.can_transp {
            // unverändertes Pixel: nur exakt die sichtbare Farbe (sonst wandert es von Frame zu Frame)
            return if dist2(c, &px.reference) <= 1e-10 {
                Some(0.0)
            } else {
                None
            };
        }
        let r = [
            px.reference[0] + carry[0],
            px.reference[1] + carry[1],
            px.reference[2] + carry[2],
        ];
        let d = super::color::wdist2(c, &r, LOSSY_L_WEIGHT);
        if d <= thr2 { Some(d) } else { None }
    };

    let mut cur = plan[0].default as u16;
    chosen[0] = plan[0].default;
    let mut prev_pick = plan[0].default;
    for i in 1..plan.len() {
        let px = &plan[i];
        let x = i % row;
        if x == 0 {
            carry_right = [0.0; 3];
        }
        let carry = if lossy {
            let d = carry_down[x];
            [
                carry_right[0] + d[0],
                carry_right[1] + d[1],
                carry_right[2] + d[2],
            ]
        } else {
            [0.0; 3]
        };
        // passenden Folge-Eintrag suchen; mehrere gleichwertige → der, mit dem es am weitesten geht
        // (Vorausschau), dann Vorgabe-Index, dann kleinster Abstand
        let mut best: Option<(u16, u8, f32, usize)> = None;
        let mut cands = 0usize;
        let mut c = trie.first[cur as usize];
        while c != NONE {
            let idx = trie.index[c as usize];
            if let Some(cost) = accept(px, idx, &carry) {
                cands += 1;
                let score = if cands > 1 || best.is_some() {
                    lookahead(&trie, plan, i, c, &accept)
                } else {
                    0
                };
                if cands == 2 {
                    // erster Kandidat bekommt nachträglich seine Vorausschau
                    if let Some(b) = best.as_mut() {
                        b.3 = lookahead(&trie, plan, i, b.0, &accept);
                    }
                }
                let better = match best {
                    None => true,
                    Some(b) => {
                        score > b.3
                            || (score == b.3
                                && (idx == px.default || (b.1 != px.default && cost < b.2)))
                    }
                };
                if better {
                    best = Some((c, idx, cost, score));
                }
            }
            c = trie.sibling[c as usize];
        }
        let pick = match best {
            Some((code, idx, _, _)) => {
                cur = code;
                idx
            }
            None => {
                // neuer Code: unter den gleichwertigen Anfängen den mit der längsten Vorausschau
                let mut start = px.default;
                if px.flex {
                    let mut best_len = lookahead(&trie, plan, i, px.default as u16, &accept);
                    for alt in [
                        Some(prev_pick),
                        if px.can_transp { p.transparent } else { None },
                    ]
                    .into_iter()
                    .flatten()
                    {
                        if alt != start && accept(px, alt, &carry).is_some() {
                            let l = lookahead(&trie, plan, i, alt as u16, &accept);
                            if l > best_len {
                                best_len = l;
                                start = alt;
                            }
                        }
                    }
                }
                w.put(cur, width);
                if (next as usize) < MAX_CODES {
                    trie.add(cur, start, next);
                    next += 1;
                    if next > (1u16 << width) && width < 12 {
                        width += 1;
                    }
                } else {
                    w.put(clear, width);
                    trie.reset();
                    width = mcs + 1;
                    next = eoi + 1;
                }
                cur = start as u16;
                start
            }
        };
        prev_pick = pick;
        chosen[i] = pick;
        if lossy {
            let mut err = [0f32; 3];
            if pick != px.default && Some(pick) != p.transparent {
                let q = p.palette[pick as usize];
                err = [
                    (px.reference[0] + carry[0] - q[0]) * 0.5,
                    (px.reference[1] + carry[1] - q[1]) * 0.5,
                    (px.reference[2] + carry[2] - q[2]) * 0.5,
                ];
            }
            carry_right = err;
            carry_down[x] = [err[0] * 0.5, err[1] * 0.5, err[2] * 0.5];
        }
    }
    w.put(cur, width);
    w.put(eoi, width);
    (w.finish(), chosen)
}

/// Wie weit lässt sich ab Pixel `i` (schon mit `code` abgedeckt) im Wörterbuch weitergehen?
/// Höchstens 48 Schritte, ohne Fehlerweitergabe, Vorgabe-Index bevorzugt.
fn lookahead(
    trie: &Trie,
    plan: &[PlanPx],
    i: usize,
    code: u16,
    accept: &impl Fn(&PlanPx, u8, &Lab) -> Option<f32>,
) -> usize {
    let mut cur = code;
    let mut n = 0;
    for px in plan.iter().skip(i + 1).take(48) {
        let mut next = NONE;
        let mut c = trie.first[cur as usize];
        while c != NONE {
            let idx = trie.index[c as usize];
            if idx == px.default {
                next = c;
                break;
            }
            if next == NONE && accept(px, idx, &[0.0; 3]).is_some() {
                next = c;
            }
            c = trie.sibling[c as usize];
        }
        if next == NONE {
            break;
        }
        cur = next;
        n += 1;
    }
    n
}

/// Mindest-Codelänge für eine Farbtabelle mit `entries` Einträgen.
pub fn min_code_size(entries: usize) -> u8 {
    let mut b = 2u8;
    while (1usize << b) < entries {
        b += 1;
    }
    b
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Einfacher GIF-LZW-Dekodierer (nur für Tests).
    pub fn decode(data: &[u8], mcs: u8) -> Vec<u8> {
        let clear = 1usize << mcs;
        let eoi = clear + 1;
        let mut dict: Vec<Vec<u8>> = Vec::new();
        let reset = |d: &mut Vec<Vec<u8>>| {
            d.clear();
            for i in 0..clear {
                d.push(vec![i as u8]);
            }
            d.push(vec![]);
            d.push(vec![]);
        };
        reset(&mut dict);
        let mut width = mcs as u32 + 1;
        let (mut acc, mut n, mut pos) = (0u32, 0u32, 0usize);
        let mut out = Vec::new();
        let mut prev: Option<usize> = None;
        loop {
            while n < width {
                if pos >= data.len() {
                    return out;
                }
                acc |= (data[pos] as u32) << n;
                pos += 1;
                n += 8;
            }
            let code = (acc & ((1 << width) - 1)) as usize;
            acc >>= width;
            n -= width;
            if code == clear {
                reset(&mut dict);
                width = mcs as u32 + 1;
                prev = None;
                continue;
            }
            if code == eoi {
                return out;
            }
            let entry = if code < dict.len() {
                dict[code].clone()
            } else {
                let p = &dict[prev.expect("code")];
                let mut e = p.clone();
                e.push(p[0]);
                e
            };
            out.extend_from_slice(&entry);
            if let Some(pv) = prev
                && dict.len() < 4096
            {
                let mut e = dict[pv].clone();
                e.push(entry[0]);
                dict.push(e);
                if dict.len() == (1 << width) && width < 12 {
                    width += 1;
                }
            }
            prev = Some(code);
        }
    }

    fn params(mcs: u8) -> LzwParams<'static> {
        LzwParams {
            min_code_size: mcs,
            palette: &[],
            transparent: None,
            threshold: 0.0,
            width: 64,
        }
    }

    #[test]
    fn lossless_round_trip_long_and_random() {
        let mut seed = 12345u64;
        for (len, colors) in [
            (1usize, 4usize),
            (2, 2),
            (70_000, 256),
            (50_000, 3),
            (9_000, 16),
        ] {
            let data: Vec<u8> = (0..len)
                .map(|i| {
                    seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
                    // Mischung aus Läufen und Rauschen
                    if (i / 37) % 3 == 0 {
                        ((i / 500) % colors) as u8
                    } else {
                        ((seed >> 40) as usize % colors) as u8
                    }
                })
                .collect();
            let mcs = min_code_size(colors);
            let plan: Vec<PlanPx> = data.iter().map(|d| PlanPx::fixed(*d)).collect();
            let (bytes, chosen) = encode(&plan, &params(mcs));
            assert_eq!(chosen, data);
            assert_eq!(decode(&bytes, mcs), data, "len {len}");
        }
    }

    #[test]
    fn transparency_equivalence_lengthens_runs() {
        // Muster: abwechselnd „unverändert“ (transparent oder Farbe 1 gleichwertig) und Farbe 1
        let pal = [[0.0, 0.0, 0.0], [0.5, 0.0, 0.0]];
        let plan: Vec<PlanPx> = (0..4000)
            .map(|i| {
                if i >= 1500 && i % 2 == 0 {
                    PlanPx {
                        default: 2,
                        flex: true,
                        can_transp: true,
                        reference: [0.5, 0.0, 0.0],
                        forbid: 256,
                    }
                } else {
                    PlanPx::fixed(1)
                }
            })
            .collect();
        let p = LzwParams {
            min_code_size: 2,
            palette: &pal,
            transparent: Some(2),
            threshold: 0.0,
            width: 4000,
        };
        let (bytes, chosen) = encode(&plan, &p);
        let strict: Vec<PlanPx> = plan.iter().map(|q| PlanPx::fixed(q.default)).collect();
        let (b2, _) = encode(&strict, &p);
        assert!(bytes.len() < b2.len(), "{} vs {}", bytes.len(), b2.len());
        assert_eq!(decode(&bytes, 2), chosen);
        assert!(chosen.iter().all(|c| *c == 1 || *c == 2));
    }

    #[test]
    fn lossy_respects_threshold() {
        // Palette entlang des Farbtons (a), Abstand 1/15 je Stufe
        let pal: Vec<Lab> = (0..16).map(|i| [0.6, i as f32 / 15.0, 0.0]).collect();
        let mut seed = 7u64;
        let plan: Vec<PlanPx> = (0..20_000)
            .map(|_| {
                seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
                let v = 7 + ((seed >> 60) as u8 % 3);
                PlanPx {
                    default: v,
                    flex: true,
                    can_transp: false,
                    reference: pal[v as usize],
                    forbid: 256,
                }
            })
            .collect();
        let mk = |t: f32| LzwParams {
            min_code_size: 4,
            palette: &pal,
            transparent: None,
            threshold: t,
            width: 200,
        };
        let (lossless, c0) = encode(&plan, &mk(0.0));
        let (lossy, c1) = encode(&plan, &mk(0.08));
        assert!(
            lossy.len() * 10 < lossless.len() * 8,
            "{} vs {}",
            lossy.len(),
            lossless.len()
        );
        assert_eq!(decode(&lossy, 4), c1);
        assert_eq!(
            c0.iter()
                .zip(&plan)
                .filter(|(c, p)| **c != p.default)
                .count(),
            0
        );
        // jede Ersetzung höchstens einen Schritt (1/15 ≈ 0,067) plus weitergegebener Fehler entfernt
        assert!(
            c1.iter()
                .zip(&plan)
                .all(|(c, p)| (*c as i32 - p.default as i32).abs() <= 2)
        );
    }

    /// Helligkeit wird strenger behandelt als Farbton (BATCH-8b): dieselben Abstände entlang L werden
    /// mit derselben Schwelle kaum ersetzt.
    #[test]
    fn lossy_is_stricter_on_lightness() {
        let mut seed = 11u64;
        let picks: Vec<u8> = (0..20_000)
            .map(|_| {
                seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
                7 + ((seed >> 60) as u8 % 3)
            })
            .collect();
        let run = |pal: &[Lab]| -> (usize, usize) {
            let plan: Vec<PlanPx> = picks
                .iter()
                .map(|v| PlanPx {
                    default: *v,
                    flex: true,
                    can_transp: false,
                    reference: pal[*v as usize],
                    forbid: 256,
                })
                .collect();
            let p = LzwParams {
                min_code_size: 4,
                palette: pal,
                transparent: None,
                threshold: 0.08,
                width: 200,
            };
            let (bytes, chosen) = encode(&plan, &p);
            (
                bytes.len(),
                chosen.iter().zip(&picks).filter(|(c, d)| c != d).count(),
            )
        };
        let chroma: Vec<Lab> = (0..16).map(|i| [0.6, i as f32 / 15.0 * 0.5, 0.0]).collect();
        let light: Vec<Lab> = (0..16)
            .map(|i| [0.2 + i as f32 / 15.0 * 0.5, 0.0, 0.0])
            .collect();
        let (bc, nc) = run(&chroma);
        let (bl, nl) = run(&light);
        assert!(
            nc > 4 * nl.max(1) && bc < bl,
            "Farbton {nc} Ersetzungen/{bc} B, Helligkeit {nl}/{bl} B"
        );
    }
}
