"""Generates Rust tables for fp-fwpng from re/format/ate_keys.json and re/format/vocabulary.json."""
import json, os, sys

HERE = os.path.dirname(os.path.abspath(__file__))
FP = os.path.normpath(os.path.join(HERE, '..', '..', '..'))
RE = os.path.normpath(os.path.join(FP, '..', 're'))
ATE = os.path.join(RE, 'format', 'ate_keys.json')
VOC = os.path.join(RE, 'format', 'vocabulary.json')
OUT_ATE = os.path.join(FP, 'crates', 'fp-fwpng', 'src', 'ate_keys.rs')
OUT_VOC = os.path.join(FP, 'crates', 'fp-fwpng', 'src', 'vocab.rs')

ate = json.load(open(ATE, encoding='utf-8'))
lines = [
    '//! ATE key tables (specification 02 §12.7, source `re/format/ate_keys.json`).',
    '//!',
    '//! Per ATE class: numeric key → (name, version flag). The version flag marks from which ATE',
    '//! version on a key exists. Older files (stream version 4) write the keys with flag 5 as plain',
    '//! text; a reader maps them to the number via [`key_by_name`].',
    '//! Generated automatically; do not edit by hand.',
    '',
    '/// An ATE class with path patterns (regular expressions from the analysis, documentation only).',
    'pub struct AteClass {',
    '    pub name: &\'static str,',
    '    pub paths: &\'static [&\'static str],',
    '    pub keys: &\'static [(u16, &\'static str, u8)],',
    '}',
    '',
    '/// All classes.',
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
    '/// Class for a name (start of the class name, e.g. `"StyleSheetData"`).',
    'pub fn class(prefix: &str) -> Option<&\'static AteClass> {',
    '    ATE_CLASSES.iter().find(|c| c.name.starts_with(prefix))',
    '}',
    '',
    '/// Name of a numeric key in a class.',
    'pub fn key_name(class: &AteClass, key: u16) -> Option<&\'static str> {',
    '    class.keys.iter().find(|k| k.0 == key).map(|k| k.1)',
    '}',
    '',
    '/// Numeric key for a plain-text name (older files).',
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
    '//! Tag vocabulary from 710 FW-PNGs (source `re/format/vocabulary.json`, specification 02 §9).',
    '//!',
    '//! Per entry: chunk (`mkBS`/`mkTS`), key, occurrences, parent containers. Used for checks and',
    '//! documentation; the reader also handles unknown tags. Generated automatically.',
    '',
    '/// A vocabulary entry.',
    'pub struct VocabEntry {',
    '    pub chunk: &\'static str,',
    '    pub key: &\'static str,',
    '    pub count: u32,',
    '    pub parents: &\'static [&\'static str],',
    '}',
    '',
    '/// All 422 tag/type combinations observed in the corpus.',
    'pub const VOCABULARY: &[VocabEntry] = &[',
]
for chunk, key, count, parents in rows:
    L.append('    VocabEntry { chunk: %s, key: %s, count: %d, parents: &[%s] },' % (
        json.dumps(chunk), json.dumps(key), count, ', '.join(json.dumps(p) for p, _ in parents)))
L.append('];')
L.append('')
L += [
    '/// Tags known to the code of the original EXE but missing from the corpus (02 §12.5).',
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
    '/// Vocabulary entry for a key in the document tree (`mkTS`).',
    'pub fn lookup(key: &str) -> Option<&\'static VocabEntry> {',
    '    VOCABULARY.iter().find(|e| e.chunk == "mkTS" && e.key == key)',
    '}',
    '',
    '/// Is the key known (corpus or EXE)?',
    'pub fn is_known(key: &str) -> bool {',
    '    VOCABULARY.iter().any(|e| e.key == key) || EXE_ONLY_TAGS.contains(&key)',
    '}',
    '',
]
open(OUT_VOC, 'w', encoding='utf-8', newline='\n').write('\n'.join(L))
print('ok', len(rows))
