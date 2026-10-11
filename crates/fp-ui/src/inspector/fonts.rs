//! Schriftenliste für den Text-Inspektor und den Texteditor: Index der Systemschriften über
//! fp-text (zur Laufzeit, im Hintergrund aufgebaut; bis dahin eine kleine Rückfallliste).

use std::sync::{Arc, OnceLock};

/// Eine Schriftfamilie mit ihren Stilen (Anzeigename, PostScript-Name).
#[derive(Clone, Debug, PartialEq)]
pub struct Family {
    pub name: String,
    pub styles: Vec<(String, String)>,
}

static INDEX: OnceLock<Arc<Vec<Family>>> = OnceLock::new();
static STARTED: OnceLock<()> = OnceLock::new();

fn build() -> Vec<Family> {
    let db = fp_text::font::FontDb::system();
    let mut out: Vec<Family> = Vec::new();
    for f in db.faces() {
        let fam = if f.names.family.is_empty() { f.names.legacy_family.clone() } else { f.names.family.clone() };
        if fam.is_empty() {
            continue;
        }
        let style = if f.names.subfamily.is_empty() { "Regular".to_string() } else { f.names.subfamily.clone() };
        let ps = if f.names.ps_name.is_empty() { fam.replace(' ', "") } else { f.names.ps_name.clone() };
        match out.iter_mut().find(|x| x.name == fam) {
            Some(x) => {
                if !x.styles.iter().any(|(s, _)| *s == style) {
                    x.styles.push((style, ps));
                }
            }
            None => out.push(Family { name: fam, styles: vec![(style, ps)] }),
        }
    }
    out.sort_by_key(|f| f.name.to_lowercase());
    out
}

/// Startet den Aufbau im Hintergrund (einmal).
pub fn warm_up() {
    if STARTED.set(()).is_ok() {
        let _ = std::thread::Builder::new().name("fp-fonts".into()).spawn(|| {
            let _ = INDEX.set(Arc::new(build()));
        });
    }
}

/// Alle Familien (leer, solange der Index noch aufgebaut wird).
pub fn families() -> Arc<Vec<Family>> {
    warm_up();
    INDEX.get().cloned().unwrap_or_else(|| {
        Arc::new(
            ["Arial", "Segoe UI", "Tahoma", "Times New Roman", "Verdana"]
                .iter()
                .map(|n| Family { name: n.to_string(), styles: vec![("Regular".into(), n.replace(' ', ""))] })
                .collect(),
        )
    })
}

/// Familie und Stil zu einem Schriftnamen (PostScript- oder Familienname).
pub fn lookup(name: &str) -> (String, String) {
    let fams = families();
    for f in fams.iter() {
        if f.name.eq_ignore_ascii_case(name) {
            return (f.name.clone(), f.styles.first().map(|s| s.0.clone()).unwrap_or_else(|| "Regular".into()));
        }
        if let Some((s, _)) = f.styles.iter().find(|(_, ps)| ps.eq_ignore_ascii_case(name)) {
            return (f.name.clone(), s.clone());
        }
    }
    let base = name.split('-').next().unwrap_or(name).trim_end_matches("MT").to_string();
    (if base.is_empty() { name.to_string() } else { base }, "Regular".into())
}

/// PostScript-Name zu Familie und Stil.
pub fn ps_name(family: &str, style: &str) -> String {
    let fams = families();
    fams.iter()
        .find(|f| f.name == family)
        .and_then(|f| f.styles.iter().find(|(s, _)| s == style).or(f.styles.first()).map(|(_, ps)| ps.clone()))
        .unwrap_or_else(|| family.to_string())
}
