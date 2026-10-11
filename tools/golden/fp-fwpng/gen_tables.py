"""Erzeugt Rust-Tabellen fuer fp-fwpng aus re/format/ate_keys.json und re/format/vocabulary.json."""
import json, sys

ATE = r'C:\Users\Administrator\Desktop\REA\re\format\ate_keys.json'
VOC = r'C:\Users\Administrator\Desktop\REA\re\format\vocabulary.json'
OUT_ATE = r'C:\Users\Administrator\Desktop\REA\firepixel\crates\fp-fwpng\src\ate_keys.rs'
OUT_VOC = r'C:\Users\Administrator\Desktop\REA\firepixel\crates\fp-fwpng\src\vocab.rs'

ate = json.load(open(ATE, encoding='utf-8'))
lines = [
    '//! ATE-Schlüsseltabellen (Spezifikation 02 §12.7, Quelle `re/format/ate_keys.json`).',
    '//!',
    '//! Je ATE-Klasse: Zahlenschlüssel → (Name, Versionsflag). Das Versionsflag kennzeichnet, ab welcher',
    '//! ATE-Fassung ein Schlüssel existiert. Ältere Dateien (Stream-Version 4) schreiben die Schlüssel mit',
    '//! Flag 5 im Klartext; ein Leser bildet sie über [`key_by_name`] auf die Nummer ab.',
    '//! Automatisch erzeugt; nicht von Hand ändern.',
    '',
    '/// Eine ATE-Klasse mit Pfadmustern (reguläre Ausdrücke aus der Analyse, nur zur Dokumentation).',
    'pub struct AteClass {',
    '    pub name: &\'static str,',
    '    pub paths: &\'static [&\'static str],',
    '    pub keys: &\'static [(u16, &\'static str, u8)],',
    '}',
    '',
    '/// Alle Klassen.',
    'pub const ATE_CLASSES: &[AteClass] = &[',
]
consts = {}
for cname, c in ate.items():
    keys = sorted(((int(k), v[0], v[1]) for k, v in c['schluessel'].items()), key=lambda x: x[0])
    paths = c.get('pfade', [])
    lines.append('    AteClass {')
    lines.append('        name: %s,' % json.dumps(cname, ensure_ascii=False))
    lines.append('        paths: &[%s],' % ', '.join(json.dumps(p, ensure_ascii=False) for p in paths))
    lines.append('        keys: &[%s],' % ', '.join('(%d, %s, %d)' % (k, json.dumps(n), f) for k, n, f in keys))
    lines.append('    },')
lines.append('];')
lines.append('')
lines += [
    '/// Klasse zu einem Namen (Anfang des Klassennamens, z. B. `"StyleSheetData"`).',
    'pub fn class(prefix: &str) -> Option<&\'static AteClass> {',
    '    ATE_CLASSES.iter().find(|c| c.name.starts_with(prefix))',
    '}',
    '',
    '/// Name eines Zahlenschlüssels in einer Klasse.',
    'pub fn key_name(class: &AteClass, key: u16) -> Option<&\'static str> {',
    '    class.keys.iter().find(|k| k.0 == key).map(|k| k.1)',
    '}',
    '',
    '/// Zahlenschlüssel zu einem Klartextnamen (ältere Dateien).',
    'pub fn key_by_name(class: &AteClass, name: &str) -> Option<u16> {',
    '    class.keys.iter().find(|k| k.1 == name).map(|k| k.0)',
    '}',
    '',
]
open(OUT_ATE, 'w', encoding='utf-8', newline='\n').write('\n'.join(lines))

voc = json.load(open(VOC, encoding='utf-8'))
rows = []
for k, d in voc['tags'].items():
    key = d['tag'] + d['type']
    rows.append((d['chunk'], key, d['count'], sorted(d['parents'].items(), key=lambda x: -x[1])))
rows.sort(key=lambda r: (r[0], r[1]))
L = [
    '//! Tag-Vokabular aus 710 FW-PNGs (Quelle `re/format/vocabulary.json`, Spezifikation 02 §9).',
    '//!',
    '//! Je Eintrag: Chunk (`mkBS`/`mkTS`), Schlüssel, Vorkommen, Elterncontainer. Dient zur Prüfung und',
    '//! Dokumentation; der Leser verarbeitet auch unbekannte Tags. Automatisch erzeugt.',
    '',
    '/// Ein Vokabular-Eintrag.',
    'pub struct VocabEntry {',
    '    pub chunk: &\'static str,',
    '    pub key: &\'static str,',
    '    pub count: u32,',
    '    pub parents: &\'static [&\'static str],',
    '}',
    '',
    '/// Alle 422 im Korpus beobachteten Tag/Typ-Kombinationen.',
    'pub const VOCABULARY: &[VocabEntry] = &[',
]
for chunk, key, count, parents in rows:
    L.append('    VocabEntry { chunk: %s, key: %s, count: %d, parents: &[%s] },' % (
        json.dumps(chunk), json.dumps(key), count, ', '.join(json.dumps(p) for p, _ in parents)))
L.append('];')
L.append('')
L += [
    '/// Tags, die der Code der Original-EXE kennt, die aber im Korpus fehlen (02 §12.5).',
    'pub const EXE_ONLY_TAGS: &[&str] = &[',
]
exe_only = ['PTRy', 'EXSv', 'MSVv', 'MSSv', 'OSSv', 'NMSs', 'MJPv', 'MNTi', 'TMMi', 'PCWf', 'FNTs', 'LNKb', 'MENb',
            'AEOi', 'ASOi', 'ANRi', 'BUTs', 'ALTs', 'MSGs', 'HTMs', 'ANEi', 'ANSi', 'ROLi', 'BEHv', 'BEVv', 'BETi',
            'BEEi', 'CDTs', 'CHKb', 'RMOb', 'PREb', 'RELb', 'ENDb', 'FRTi', 'MITi', 'MIDi', 'MIHi', 'MIWi', 'MIVb',
            'CSPi', 'CPDi', 'CSHi', 'HICi', 'HIMb', 'BOCi', 'BOSi', 'OPQb', 'OVWb', 'OVHb', 'TMOi', 'VERi', 'XSLf',
            'YSLf', 'EVVv', 'EVTs', 'EPDv', 'DCFv', 'VVLs', 'ZEPv', 'ULLv', 'CLLb', 'CLMb', 'CLTb', 'CLPi', 'CLSb',
            'CNSb', 'XSZi', 'YSZi', 'LRLi', 'LRTi', 'LRRi', 'LRBi', 'CBDs', 'CXFs', 'NMMi']
L.append('    ' + ', '.join(json.dumps(t) for t in exe_only) + ',')
L.append('];')
L += [
    '',
    '/// Vokabular-Eintrag zu einem Schlüssel im Dokumentbaum (`mkTS`).',
    'pub fn lookup(key: &str) -> Option<&\'static VocabEntry> {',
    '    VOCABULARY.iter().find(|e| e.chunk == "mkTS" && e.key == key)',
    '}',
    '',
    '/// Ist der Schlüssel bekannt (Korpus oder EXE)?',
    'pub fn is_known(key: &str) -> bool {',
    '    VOCABULARY.iter().any(|e| e.key == key) || EXE_ONLY_TAGS.contains(&key)',
    '}',
    '',
]
open(OUT_VOC, 'w', encoding='utf-8', newline='\n').write('\n'.join(L))
print('ok', len(rows))
