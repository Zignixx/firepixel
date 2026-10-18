//! IPTC-IIM (B-4): datasets inside the image-resource block list that `Meta::extra`
//! ([`super::EXTRA_IPTC`]) stores (resource `0x0404`), reading and replacing datasets.
//!
//! Resource block: `8BIM`, id u16, Pascal name padded to an even length, size u32, data padded
//! to an even length (all big-endian). IIM dataset: `0x1C`, record, dataset, length u16, data.

/// Resource id of the IPTC-IIM block.
pub const IPTC_RESOURCE: u16 = 0x0404;
/// Record 2 datasets used by `metadata.set`.
pub const DS_OBJECT_NAME: u8 = 5;
pub const DS_KEYWORDS: u8 = 25;
pub const DS_BYLINE: u8 = 80;
pub const DS_COPYRIGHT: u8 = 116;
pub const DS_CAPTION: u8 = 120;

/// One resource block (id, name bytes, data).
type Resource = (u16, Vec<u8>, Vec<u8>);

fn be16(d: &[u8], i: usize) -> Option<u16> {
    Some(u16::from_be_bytes(d.get(i..i + 2)?.try_into().ok()?))
}
fn be32(d: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_be_bytes(d.get(i..i + 4)?.try_into().ok()?))
}

/// Resource blocks of an image-resource list (stops at the first damaged block).
pub fn resources(d: &[u8]) -> Vec<Resource> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 12 <= d.len() && &d[i..i + 4] == b"8BIM" {
        let Some(id) = be16(d, i + 4) else { break };
        let nl = d[i + 6] as usize;
        let name_len = (1 + nl + 1) & !1;
        let Some(name) = d.get(i + 7..i + 7 + nl) else {
            break;
        };
        let p = i + 6 + name_len;
        let Some(size) = be32(d, p) else { break };
        let Some(data) = d.get(p + 4..p + 4 + size as usize) else {
            break;
        };
        out.push((id, name.to_vec(), data.to_vec()));
        i = p + 4 + ((size as usize + 1) & !1);
    }
    out
}

/// Serialises resource blocks.
pub fn write_resources(list: &[Resource]) -> Vec<u8> {
    let mut o = Vec::new();
    for (id, name, data) in list {
        o.extend_from_slice(b"8BIM");
        o.extend_from_slice(&id.to_be_bytes());
        let nl = name.len().min(255);
        o.push(nl as u8);
        o.extend_from_slice(&name[..nl]);
        if (1 + nl) % 2 == 1 {
            o.push(0);
        }
        o.extend_from_slice(&(data.len() as u32).to_be_bytes());
        o.extend_from_slice(data);
        if data.len() % 2 == 1 {
            o.push(0);
        }
    }
    o
}

/// IIM datasets `(record, dataset, data)`.
pub fn datasets(iim: &[u8]) -> Vec<(u8, u8, Vec<u8>)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 5 <= iim.len() && iim[i] == 0x1C {
        let (rec, ds) = (iim[i + 1], iim[i + 2]);
        let Some(len) = be16(iim, i + 3) else { break };
        if len & 0x8000 != 0 {
            break; // extended length: not used here
        }
        let Some(data) = iim.get(i + 5..i + 5 + len as usize) else {
            break;
        };
        out.push((rec, ds, data.to_vec()));
        i += 5 + len as usize;
    }
    out
}

/// Serialises IIM datasets (data longer than 32767 bytes is cut).
pub fn write_datasets(list: &[(u8, u8, Vec<u8>)]) -> Vec<u8> {
    let mut o = Vec::new();
    for (rec, ds, data) in list {
        let n = data.len().min(0x7FFF);
        o.extend_from_slice(&[0x1C, *rec, *ds]);
        o.extend_from_slice(&(n as u16).to_be_bytes());
        o.extend_from_slice(&data[..n]);
    }
    o
}

/// IIM of a resource list (`None` without IPTC block).
pub fn iim_of(irb: &[u8]) -> Option<Vec<u8>> {
    resources(irb)
        .into_iter()
        .find(|r| r.0 == IPTC_RESOURCE)
        .map(|r| r.2)
}

/// Resource list holding only `iim` (for containers that store raw IIM, e.g. TIFF tag 33723).
pub fn irb_from_iim(iim: &[u8]) -> Vec<u8> {
    write_resources(&[(IPTC_RESOURCE, Vec::new(), iim.to_vec())])
}

/// Replaces record-2 datasets in `irb` (or a new list): every `(dataset, values)` drops the old
/// values of that dataset and adds the new ones (UTF-8, coded character set 1:90 = `ESC % G`).
/// Other resources and datasets stay.
pub fn set_record2(irb: Option<&[u8]>, sets: &[(u8, Vec<String>)]) -> Vec<u8> {
    let mut res = irb.map(resources).unwrap_or_default();
    let mut ds = res
        .iter()
        .find(|r| r.0 == IPTC_RESOURCE)
        .map(|r| datasets(&r.2))
        .unwrap_or_default();
    ds.retain(|(rec, d, _)| !(*rec == 1 && *d == 90) && !(*rec == 2 && *d == 0));
    ds.retain(|(rec, d, _)| !(*rec == 2 && sets.iter().any(|s| s.0 == *d)));
    let mut out: Vec<(u8, u8, Vec<u8>)> = vec![(1, 90, b"\x1B%G".to_vec()), (2, 0, vec![0, 4])];
    out.extend(ds);
    for (d, values) in sets {
        for v in values {
            out.push((2, *d, v.as_bytes().to_vec()));
        }
    }
    // record order: 1 before 2, dataset order within a record
    out.sort_by_key(|x| (x.0, x.1));
    let iim = write_datasets(&out);
    match res.iter_mut().find(|r| r.0 == IPTC_RESOURCE) {
        Some(r) => r.2 = iim,
        None => res.push((IPTC_RESOURCE, Vec::new(), iim)),
    }
    write_resources(&res)
}

/// Text values of a record-2 dataset.
pub fn record2(irb: &[u8], dataset: u8) -> Vec<String> {
    iim_of(irb)
        .map(|i| {
            datasets(&i)
                .into_iter()
                .filter(|(r, d, _)| *r == 2 && *d == dataset)
                .map(|(_, _, v)| String::from_utf8_lossy(&v).into_owned())
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replace_keeps_other_blocks() {
        let other = write_resources(&[(0x0425, b"x".to_vec(), vec![1, 2, 3])]);
        let a = set_record2(
            Some(&other),
            &[(DS_KEYWORDS, vec!["sea".into(), "sky".into()])],
        );
        assert_eq!(resources(&a).len(), 2);
        assert_eq!(record2(&a, DS_KEYWORDS), vec!["sea", "sky"]);
        let b = set_record2(Some(&a), &[(DS_KEYWORDS, vec!["Grün".into()])]);
        assert_eq!(record2(&b, DS_KEYWORDS), vec!["Grün"]);
        assert_eq!(resources(&b)[0], (0x0425, b"x".to_vec(), vec![1, 2, 3]));
    }
}
