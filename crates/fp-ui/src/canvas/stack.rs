//! Stapelfolge der Vorschauen auf der Leinwand (N34, BA-039): Beim Ziehen, Transformieren und
//! Malen liegt die Vorschau (gerenderte Textur bzw. Malkacheln) nicht mehr über allen Objekten,
//! sondern an der Stelle des Objekts in der Stapelfolge. Dafür wird die Seite in Schichten zerlegt:
//!
//! - das Hintergrundbild mit allem, was unter dem (untersten) gezogenen Objekt liegt,
//! - je Lauf aufeinanderfolgender gezogener Objekte deren Vorschau,
//! - darüber jeweils eine durchsichtige Schicht mit den Objekten bis zum nächsten Lauf.
//!
//! Die Schichten rendert der Hintergrund-Thread einmal zu Beginn (und bei neuem Ausschnitt/Zoom);
//! während des Ziehens werden nur Texturen übereinandergelegt – so schnell wie bisher. Liegt das
//! gezogene Objekt oben (häufigster Fall), gibt es keine zusätzliche Schicht.
//!
//! Teile („Atome“) der Seite sind ganze Ebenen ohne gezogenes Objekt oder Elemente; Gruppen und
//! Unterebenen werden nur dann zerlegt, wenn sie ein gezogenes Objekt enthalten.

use fp_model::{Document, Element, ElementKind, LayerKind, LayerSlot};
use std::collections::HashSet;
use std::sync::Arc;

/// Teil einer Seite, der in einer Schicht sichtbar oder ausgeblendet ist.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Atom {
    /// Element (mit seinem ganzen Unterbaum).
    Elem(u64),
    /// Ganzer Eintrag `page.root.layers[i]` (Ebene, geteilte Ebene, Masterseiten-Ebenen).
    Slot(usize),
}

/// Lauf aufeinanderfolgender gezogener Objekte und die Schicht darüber.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Run {
    /// Gezogene Elemente des Laufs (Vorschau-Textur).
    pub ids: Vec<u64>,
    /// In der Schicht über dem Lauf auszublendende Teile; `None` = Schicht ist leer.
    pub above_hidden: Option<Vec<Atom>>,
}

/// Schichten einer Vorschau.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StackPlan {
    /// Im Hintergrundbild ausgeblendete Teile.
    pub hidden: Vec<Atom>,
    /// Läufe von unten nach oben.
    pub runs: Vec<Run>,
}

impl StackPlan {
    /// Herkömmliche Vorschau ohne Stapelfolge: `ids` ausblenden (außer bei `copy`), Vorschau oben.
    pub fn flat(ids: &[u64], copy: bool) -> StackPlan {
        StackPlan {
            hidden: if copy {
                Vec::new()
            } else {
                ids.iter().map(|i| Atom::Elem(*i)).collect()
            },
            runs: vec![Run {
                ids: ids.to_vec(),
                above_hidden: None,
            }],
        }
    }

    /// Hidden-Sets der Schichten über den Läufen (leere Schichten als `None`).
    pub fn layers(&self) -> Vec<Option<Vec<Atom>>> {
        self.runs.iter().map(|r| r.above_hidden.clone()).collect()
    }

    /// Gibt es mindestens eine Schicht über einer Vorschau?
    pub fn has_layers(&self) -> bool {
        self.runs.iter().any(|r| r.above_hidden.is_some())
    }
}

/// Enthält der Unterbaum von `e` (ohne `e` selbst) eines der Elemente?
fn contains_any(e: &Element, set: &HashSet<u64>) -> bool {
    match &e.kind {
        ElementKind::Group(g) => g
            .children
            .iter()
            .chain(g.mask.iter())
            .any(|c| set.contains(&c.id.0) || contains_any(c, set)),
        ElementKind::SubLayer(l) => l.cels.iter().any(|c| {
            c.elements
                .iter()
                .any(|x| set.contains(&x.id.0) || contains_any(x, set))
        }),
        _ => false,
    }
}

fn list_contains(list: &[Arc<Element>], set: &HashSet<u64>) -> bool {
    list.iter()
        .any(|x| set.contains(&x.id.0) || contains_any(x, set))
}

/// Teile einer Elementliste (oben → unten gespeichert) von unten nach oben.
fn atoms_of_list(list: &[Arc<Element>], set: &HashSet<u64>, out: &mut Vec<(Atom, bool)>) {
    for e in list.iter().rev() {
        if set.contains(&e.id.0) {
            out.push((Atom::Elem(e.id.0), true));
        } else if !e.common.visible || matches!(e.kind, ElementKind::WebObject(_)) {
            // zeichnet nichts: braucht keine Schicht
        } else if contains_any(e, set) {
            match &e.kind {
                // Gruppe ohne Maske: Kinder einzeln (die Maske lässt sich nicht aufteilen)
                ElementKind::Group(g)
                    if !g
                        .mask
                        .as_ref()
                        .is_some_and(|m| set.contains(&m.id.0) || contains_any(m, set)) =>
                {
                    atoms_of_list(&g.children, set, out)
                }
                ElementKind::SubLayer(l) => {
                    if let Some(c) = l.cels.first() {
                        atoms_of_list(&c.elements, set, out);
                    }
                }
                _ => out.push((Atom::Elem(e.id.0), false)),
            }
        } else {
            out.push((Atom::Elem(e.id.0), false));
        }
    }
}

/// Teile der Seite von unten nach oben, je mit „gezogen“; `None`, wenn nicht alle gezogenen
/// Elemente als eigene Teile vorkommen (z. B. Maske einer Maskengruppe, Teil eines Compound Shapes).
pub fn atoms(
    doc: &Document,
    page: usize,
    frame: usize,
    dragged: &[u64],
) -> Option<Vec<(Atom, bool)>> {
    let p = doc.pages.get(page)?;
    let set: HashSet<u64> = dragged.iter().copied().collect();
    let mut out = Vec::new();
    for (i, slot) in p.root.layers.iter().enumerate() {
        let layer = match slot {
            LayerSlot::Own(l) => Some(Arc::clone(l)),
            LayerSlot::Shared(r) => doc.shared_layer(&r.id).and_then(|d| d.layer.clone()),
            LayerSlot::MasterPage(m) if m.is_visible() => None,
            // unsichtbare Masterseiten-Ebenen und Unbekanntes zeichnen nichts
            _ => continue,
        };
        // Web-Ebene und ausgeblendete Ebenen zeichnen nichts (wie fp-render): keine eigene Schicht
        if let Some(l) = &layer
            && (l.kind == LayerKind::Web
                || !l
                    .visibility
                    .get(frame)
                    .or(l.visibility.first())
                    .copied()
                    .unwrap_or(true))
            && !l.cels.iter().any(|c| list_contains(&c.elements, &set))
        {
            continue;
        }
        match layer.as_ref().and_then(|l| l.cel(frame)) {
            Some(c) if list_contains(&c.elements, &set) => {
                atoms_of_list(&c.elements, &set, &mut out)
            }
            _ => out.push((Atom::Slot(i), false)),
        }
    }
    let found: HashSet<u64> = out
        .iter()
        .filter_map(|(a, d)| match (a, d) {
            (Atom::Elem(id), true) => Some(*id),
            _ => None,
        })
        .collect();
    (found == set).then_some(out)
}

/// Schichten für die gezogenen Elemente `dragged`. `copy` (Alt-Ziehen, Malen): das Original bleibt
/// stehen, die Vorschau liegt direkt darüber (wie das Duplikat nach dem Loslassen).
pub fn plan(
    doc: &Document,
    page: usize,
    frame: usize,
    dragged: &[u64],
    copy: bool,
) -> Option<StackPlan> {
    let seq = atoms(doc, page, frame, dragged)?;
    // Gruppen: N0, (D1, N1), (D2, N2), …
    let mut below: Vec<Atom> = Vec::new();
    let mut runs: Vec<(Vec<Atom>, Vec<Atom>)> = Vec::new();
    for (a, d) in seq.iter().copied() {
        match (d, runs.last_mut()) {
            (false, None) => below.push(a),
            (false, Some((_, n))) => n.push(a),
            (true, Some((ds, n))) if n.is_empty() => ds.push(a),
            (true, _) => runs.push((vec![a], Vec::new())),
        }
    }
    let all: Vec<Atom> = seq.iter().map(|(a, _)| *a).collect();
    // sichtbare Teile je Schicht: L0 = N0 (+ D1 bei Kopie), Li = Ni (+ D(i+1) bei Kopie)
    let visible = |i: usize| -> HashSet<Atom> {
        let mut v: HashSet<Atom> = if i == 0 {
            below.iter().copied().collect()
        } else {
            runs[i - 1].1.iter().copied().collect()
        };
        if copy && let Some((ds, _)) = runs.get(i) {
            v.extend(ds.iter().copied());
        }
        v
    };
    let hidden_except = |v: &HashSet<Atom>| -> Vec<Atom> {
        all.iter().filter(|a| !v.contains(a)).copied().collect()
    };
    let hidden = hidden_except(&visible(0));
    let runs = runs
        .iter()
        .enumerate()
        .map(|(i, (ds, _))| {
            let v = visible(i + 1);
            Run {
                ids: ds
                    .iter()
                    .filter_map(|a| {
                        if let Atom::Elem(id) = a {
                            Some(*id)
                        } else {
                            None
                        }
                    })
                    .collect(),
                above_hidden: (!v.is_empty()).then(|| hidden_except(&v)),
            }
        })
        .collect();
    Some(StackPlan { hidden, runs })
}

/// Schlüssel der Eingaben von [`plan`] (Dokumentstand, Seite, Status, Elemente, Kopie).
pub fn source_key(generation: u64, page: usize, frame: usize, ids: &[u64], copy: bool) -> u64 {
    let mut h: u64 = 0x84222325cbf29ce4;
    for v in [generation, page as u64, frame as u64, copy as u64]
        .into_iter()
        .chain(ids.iter().copied())
    {
        h ^= v;
        h = h.wrapping_mul(0x0100_0000_01b3).rotate_left(9);
    }
    h
}

/// Kennung einer Schichtung (0 = nichts ausgeblendet, keine Schichten).
pub fn variant(p: &StackPlan, extra_hidden: &[u64]) -> u64 {
    if p.hidden.is_empty() && !p.has_layers() && extra_hidden.is_empty() {
        return 0;
    }
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut put = |v: u64| {
        for b in v.to_le_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    };
    let atoms = |list: &[Atom], put: &mut dyn FnMut(u64)| {
        put(list.len() as u64);
        for a in list {
            match a {
                Atom::Elem(id) => put(*id << 1),
                Atom::Slot(i) => put(((*i as u64) << 1) | 1),
            }
        }
    };
    atoms(&p.hidden, &mut put);
    for r in &p.runs {
        put(0xfeed);
        match &r.above_hidden {
            Some(l) => atoms(l, &mut put),
            None => put(u64::MAX),
        }
    }
    put(0xbeef);
    for id in extra_hidden {
        put(*id);
    }
    h | 1
}

fn hide_in_list(list: &mut [Arc<Element>], set: &HashSet<u64>, found: &mut HashSet<u64>) {
    for e in list.iter_mut() {
        if set.contains(&e.id.0) {
            Arc::make_mut(e).common.visible = false;
            found.insert(e.id.0);
        } else if contains_any(e, set) {
            // Render-Cache des Containers verwerfen (er enthält die ausgeblendeten Kinder)
            let em = Element::edit(e);
            match &mut em.kind {
                ElementKind::Group(g) => {
                    hide_in_list(&mut g.children, set, found);
                    if let Some(m) = g.mask.as_mut() {
                        hide_in_list(std::slice::from_mut(m), set, found);
                    }
                }
                ElementKind::SubLayer(l) => {
                    for c in &mut Arc::make_mut(l).cels {
                        hide_in_list(&mut Arc::make_mut(c).elements, set, found);
                    }
                }
                _ => {}
            }
        }
    }
}

/// Blendet die Teile `atoms` in `doc` (einer Kopie) aus: Elemente unsichtbar, ganze Ebenen-Einträge
/// entfernt (Copy-on-Write: nur betroffene Zweige werden kopiert).
pub fn hide(doc: &mut Document, page: usize, atoms: &[Atom]) {
    if atoms.is_empty() || page >= doc.pages.len() {
        return;
    }
    let elems: HashSet<u64> = atoms
        .iter()
        .filter_map(|a| {
            if let Atom::Elem(id) = a {
                Some(*id)
            } else {
                None
            }
        })
        .collect();
    let mut slots: Vec<usize> = atoms
        .iter()
        .filter_map(|a| {
            if let Atom::Slot(i) = a {
                Some(*i)
            } else {
                None
            }
        })
        .collect();
    let mut found = HashSet::new();
    if !elems.is_empty() {
        let p = Arc::make_mut(&mut doc.pages[page]);
        for (i, slot) in p.root.layers.iter_mut().enumerate() {
            if slots.contains(&i) {
                continue;
            }
            if let LayerSlot::Own(l) = slot
                && l.cels.iter().any(|c| list_contains(&c.elements, &elems))
            {
                for c in &mut Arc::make_mut(l).cels {
                    if list_contains(&c.elements, &elems) {
                        hide_in_list(&mut Arc::make_mut(c).elements, &elems, &mut found);
                    }
                }
            }
        }
        // übrige (geteilte Ebenen) über die Adresse
        for id in elems.iter().filter(|id| !found.contains(id)) {
            fp_engine::docops::edit_by_id(doc, page, fp_model::ElementId(*id), |e| {
                e.common.visible = false
            });
        }
    }
    if !slots.is_empty() {
        slots.sort_unstable();
        slots.dedup();
        let p = Arc::make_mut(&mut doc.pages[page]);
        for i in slots.into_iter().rev() {
            if i < p.root.layers.len() {
                p.root.layers.remove(i);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Drei Rechtecke a (unten), b, c (oben) auf einer Ebene.
    fn doc3() -> (fp_engine::Session, [u64; 3]) {
        let mut s = fp_engine::Session::default();
        let _ = s.execute(
            "file.new",
            json!({"width": 100, "height": 100, "background": "#ffffff"}),
        );
        let mut ids = [0u64; 3];
        for (i, r) in [[0, 0, 40, 40], [20, 20, 60, 60], [40, 40, 80, 80]]
            .iter()
            .enumerate()
        {
            let _ = s.execute("draw.rectangle", json!({"rect": r}));
            ids[i] = s.active().expect("d").selection.elements[0].0;
        }
        (s, ids)
    }

    #[test]
    fn middle_object_gets_layer_above_n34() {
        let (s, [a, b, c]) = doc3();
        let d = s.active().expect("d");
        let p = plan(&d.doc, d.page, d.frame, &[b], false).expect("Plan");
        // Hintergrund: b und c aus; Schicht über b: nur c sichtbar
        assert!(
            p.hidden.contains(&Atom::Elem(b))
                && p.hidden.contains(&Atom::Elem(c))
                && !p.hidden.contains(&Atom::Elem(a)),
            "{p:?}"
        );
        assert_eq!(p.runs.len(), 1);
        assert_eq!(p.runs[0].ids, vec![b]);
        let above = p.runs[0].above_hidden.as_ref().expect("Schicht über b");
        assert!(
            above.contains(&Atom::Elem(a))
                && above.contains(&Atom::Elem(b))
                && !above.contains(&Atom::Elem(c))
        );
    }

    #[test]
    fn top_object_needs_no_layer() {
        let (s, [_, _, c]) = doc3();
        let d = s.active().expect("d");
        let p = plan(&d.doc, d.page, d.frame, &[c], false).expect("Plan");
        assert_eq!(p.hidden, vec![Atom::Elem(c)]);
        assert!(!p.has_layers());
        // Kopie oben: nichts ausgeblendet, kein Neurendern
        let p = plan(&d.doc, d.page, d.frame, &[c], true).expect("Plan");
        assert_eq!(variant(&p, &[]), 0);
    }

    #[test]
    fn interleaved_selection_makes_two_runs() {
        let (s, [a, b, c]) = doc3();
        let d = s.active().expect("d");
        let p = plan(&d.doc, d.page, d.frame, &[a, c], false).expect("Plan");
        assert_eq!(
            p.runs.iter().map(|r| r.ids.clone()).collect::<Vec<_>>(),
            vec![vec![a], vec![c]]
        );
        assert!(
            p.runs[0]
                .above_hidden
                .as_ref()
                .is_some_and(|h| !h.contains(&Atom::Elem(b)))
        );
        assert!(p.runs[1].above_hidden.is_none());
        // Kopie: Original a bleibt im Hintergrund, c-Original in der Schicht über a
        let p = plan(&d.doc, d.page, d.frame, &[a, c], true).expect("Plan");
        assert!(!p.hidden.contains(&Atom::Elem(a)));
        assert!(
            p.runs[0]
                .above_hidden
                .as_ref()
                .is_some_and(|h| !h.contains(&Atom::Elem(c)) && !h.contains(&Atom::Elem(b)))
        );
    }

    #[test]
    fn hide_keeps_only_wanted_parts() {
        let (s, [a, b, c]) = doc3();
        let d = s.active().expect("d");
        let mut doc = (*d.doc).clone();
        hide(&mut doc, d.page, &[Atom::Elem(a), Atom::Elem(c)]);
        let vis = |id: u64| {
            fp_engine::docops::element_on_page(&doc, d.page, fp_model::ElementId(id))
                .expect("e")
                .common
                .visible
        };
        assert!(!vis(a) && vis(b) && !vis(c));
    }
}
