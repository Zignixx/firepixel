//! Kennwortschutz für den PDF-Export (re\18 §4.1 „Kennwort zum Öffnen“ / „Kennwort zum
//! Einschränken von Aufgaben“): Standard-Sicherheitsverfahren der PDF-Norm mit RC4, Revision 2
//! (40 Bit, PDF 1.3) bzw. Revision 3 (128 Bit, ab PDF 1.4). MD5 und RC4 sind hier selbst
//! geschrieben, damit keine weitere Abhängigkeit nötig ist.

/// Auffüllfolge der PDF-Norm für Kennwörter.
const PAD: [u8; 32] = [
    0x28, 0xBF, 0x4E, 0x5E, 0x4E, 0x75, 0x8A, 0x41, 0x64, 0x00, 0x4E, 0x56, 0xFF, 0xFA, 0x01, 0x08, 0x2E, 0x2E, 0x00, 0xB6, 0xD0, 0x68, 0x3E, 0x80, 0x2F, 0x0C, 0xA9, 0xFE, 0x64, 0x53, 0x69, 0x7A,
];

/// MD5 (RFC 1321).
pub fn md5(data: &[u8]) -> [u8; 16] {
    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    let k: Vec<u32> = (0..64).map(|i| ((i as f64 + 1.0).sin().abs() * 4294967296.0) as u32).collect();
    let (mut a0, mut b0, mut c0, mut d0) = (0x67452301u32, 0xefcdab89u32, 0x98badcfeu32, 0x10325476u32);
    let mut msg = data.to_vec();
    let bits = (data.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bits.to_le_bytes());
    for chunk in msg.chunks(64) {
        let m: Vec<u32> = chunk.chunks(4).map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]])).collect();
        let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let f = f.wrapping_add(a).wrapping_add(k[i]).wrapping_add(m[g]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(f.rotate_left(S[i]));
        }
        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }
    let mut out = [0u8; 16];
    for (i, v) in [a0, b0, c0, d0].iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
    }
    out
}

/// RC4 (ver- und entschlüsselt gleich).
pub fn rc4(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut s: [u8; 256] = std::array::from_fn(|i| i as u8);
    let mut j = 0u8;
    for i in 0..256 {
        j = j.wrapping_add(s[i]).wrapping_add(key[i % key.len()]);
        s.swap(i, j as usize);
    }
    let (mut i, mut j) = (0u8, 0u8);
    data.iter()
        .map(|&b| {
            i = i.wrapping_add(1);
            j = j.wrapping_add(s[i as usize]);
            s.swap(i as usize, j as usize);
            b ^ s[s[i as usize].wrapping_add(s[j as usize]) as usize]
        })
        .collect()
}

fn padded(pw: &str) -> [u8; 32] {
    let b: Vec<u8> = pw.chars().map(|c| if (c as u32) < 256 { c as u8 } else { b'?' }).take(32).collect();
    let mut out = [0u8; 32];
    out[..b.len()].copy_from_slice(&b);
    out[b.len()..].copy_from_slice(&PAD[..32 - b.len()]);
    out
}

/// Erlaubte Aufgaben bei „Kennwort zum Einschränken von Aufgaben“.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PdfPermissions {
    pub print: bool,
    pub copy: bool,
    pub edit: bool,
    pub comment: bool,
}

/// Ein eingerichteter Kennwortschutz: Schlüssel und die Werte des `/Encrypt`-Eintrags.
#[derive(Clone, Debug)]
pub struct PdfSecurity {
    key: Vec<u8>,
    revision: u8,
    pub o: [u8; 32],
    pub u: [u8; 32],
    pub p: i32,
    pub id: [u8; 16],
}

impl PdfSecurity {
    /// `open`: Kennwort zum Öffnen (leer = ohne), `owner`: Kennwort zum Schutz (`None` = keine
    /// Einschränkung, dann gilt das Öffnen-Kennwort auch als Besitzerkennwort). `rev3` = 128 Bit.
    pub fn new(open: &str, owner: Option<(&str, PdfPermissions)>, rev3: bool, id_seed: &[u8]) -> PdfSecurity {
        let revision = if rev3 { 3 } else { 2 };
        let n = if rev3 { 16 } else { 5 };
        let (owner_pw, perms) = match owner {
            Some((pw, perms)) if !pw.is_empty() => (pw.to_string(), perms),
            _ => (open.to_string(), PdfPermissions { print: true, copy: true, edit: true, comment: true }),
        };
        // Rechte-Bits (1-basiert): 3 Drucken, 4 Ändern, 5 Kopieren, 6 Kommentieren; bei Rev. 3 dazu
        // 9 Formulare, 10 Barrierefreiheit, 11 Zusammenstellen, 12 hochwertiges Drucken
        let mut p: u32 = if rev3 { 0xFFFF_F0C0 } else { 0xFFFF_FFC0 };
        let bit = |n: u32| 1u32 << (n - 1);
        if perms.print {
            p |= bit(3) | if rev3 { bit(12) } else { 0 };
        }
        if perms.edit {
            p |= bit(4) | if rev3 { bit(11) } else { 0 };
        }
        if perms.copy {
            p |= bit(5) | if rev3 { bit(10) } else { 0 };
        }
        if perms.comment {
            p |= bit(6) | if rev3 { bit(9) } else { 0 };
        }
        let p = p as i32;
        let id = md5(id_seed);
        // O (Algorithmus 3.3)
        let mut h = md5(&padded(&owner_pw));
        if rev3 {
            for _ in 0..50 {
                h = md5(&h);
            }
        }
        let okey = &h[..n];
        let mut o = rc4(okey, &padded(open));
        if rev3 {
            for i in 1..=19u8 {
                let k: Vec<u8> = okey.iter().map(|b| b ^ i).collect();
                o = rc4(&k, &o);
            }
        }
        let o: [u8; 32] = o.try_into().unwrap_or([0; 32]);
        // Schlüssel (Algorithmus 3.2)
        let mut buf = padded(open).to_vec();
        buf.extend_from_slice(&o);
        buf.extend_from_slice(&p.to_le_bytes());
        buf.extend_from_slice(&id);
        let mut k = md5(&buf);
        if rev3 {
            for _ in 0..50 {
                k = md5(&k[..n]);
            }
        }
        let key = k[..n].to_vec();
        // U (Algorithmus 3.4 bzw. 3.5)
        let mut u = [0u8; 32];
        if rev3 {
            let mut b = PAD.to_vec();
            b.extend_from_slice(&id);
            let mut x = rc4(&key, &md5(&b));
            for i in 1..=19u8 {
                let k2: Vec<u8> = key.iter().map(|b| b ^ i).collect();
                x = rc4(&k2, &x);
            }
            u[..16].copy_from_slice(&x);
        } else {
            u.copy_from_slice(&rc4(&key, &PAD));
        }
        PdfSecurity { key, revision, o, u, p, id }
    }

    /// Verschlüsselt Daten des Objekts `num` (Generation 0).
    pub fn encrypt(&self, num: usize, data: &[u8]) -> Vec<u8> {
        let mut b = self.key.clone();
        b.extend_from_slice(&(num as u32).to_le_bytes()[..3]);
        b.extend_from_slice(&[0, 0]);
        let h = md5(&b);
        let n = (self.key.len() + 5).min(16);
        rc4(&h[..n], data)
    }

    /// Prüft ein Kennwort zum Öffnen gegen `/U` (für Tests und Gegenprobe).
    pub fn check_user_password(&self, pw: &str) -> bool {
        // gleicher Schlüsselweg mit diesem O, P und ID
        let n = self.key.len();
        let mut buf = padded(pw).to_vec();
        buf.extend_from_slice(&self.o);
        buf.extend_from_slice(&self.p.to_le_bytes());
        buf.extend_from_slice(&self.id);
        let mut k = md5(&buf);
        if self.revision == 3 {
            for _ in 0..50 {
                k = md5(&k[..n]);
            }
        }
        k[..n] == self.key[..]
    }

    /// Der `/Encrypt`-Eintrag.
    pub fn dict(&self) -> String {
        let hex = |b: &[u8]| b.iter().map(|x| format!("{x:02X}")).collect::<String>();
        if self.revision == 3 {
            format!("<< /Filter /Standard /V 2 /R 3 /Length 128 /O <{}> /U <{}> /P {} >>", hex(&self.o), hex(&self.u), self.p)
        } else {
            format!("<< /Filter /Standard /V 1 /R 2 /O <{}> /U <{}> /P {} >>", hex(&self.o), hex(&self.u), self.p)
        }
    }

    /// Dokumentkennung für `/ID`.
    pub fn id_hex(&self) -> String {
        self.id.iter().map(|x| format!("{x:02X}")).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn md5_and_rc4_known_values() {
        let hex = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
        assert_eq!(hex(&md5(b"")), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(hex(&md5(b"The quick brown fox jumps over the lazy dog")), "9e107d9d372bb6826bd81d3542a419d6");
        assert_eq!(hex(&rc4(b"Key", b"Plaintext")), "bbf316e8d940af0ad3");
    }

    #[test]
    fn user_password_checks() {
        for rev3 in [false, true] {
            let s = PdfSecurity::new("geheim", Some(("chef", PdfPermissions { print: true, ..Default::default() })), rev3, b"x");
            assert!(s.check_user_password("geheim"));
            assert!(!s.check_user_password("falsch"));
        }
    }
}
