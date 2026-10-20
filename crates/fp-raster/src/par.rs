//! Parallelisierung über Zeilen (rayon), deterministisch.
//!
//! Alle Rechnungen in diesem Crate sind je Ausgabepixel unabhängig oder je Zeile in fester
//! Reihenfolge; eine Aufteilung auf Threads ändert daher kein Bit am Ergebnis (wie der
//! Mehrkern-Gauß des Originals, 06 §4.7). Kleine Flächen laufen ohne Thread-Overhead.

use rayon::prelude::*;

/// Ab dieser Pixelzahl wird parallel gerechnet.
pub(crate) const PAR_MIN_PIXELS: usize = 1 << 14;

/// Ruft `f(zeilenindex, zeile)` für jede Zeile von `data` (Zeilenlänge `width`) auf, bei großen
/// Flächen parallel.
pub(crate) fn for_each_row<T, F>(data: &mut [T], width: usize, f: F)
where
    T: Send,
    F: Fn(usize, &mut [T]) + Sync + Send,
{
    if width == 0 || data.is_empty() {
        return;
    }
    if data.len() >= PAR_MIN_PIXELS && rayon::current_num_threads() > 1 {
        data.par_chunks_mut(width)
            .enumerate()
            .for_each(|(i, row)| f(i, row));
    } else {
        data.chunks_mut(width)
            .enumerate()
            .for_each(|(i, row)| f(i, row));
    }
}

/// Like [`for_each_row`], when `f` only touches `span` pixels of each row: the decision for threads
/// counts the touched pixels, not the whole rows (a small image on a wide target, e.g. thousands
/// of small objects mixed onto one large image, otherwise paid the thread hand-off each time).
pub(crate) fn for_each_row_span<T, F>(data: &mut [T], width: usize, span: usize, f: F)
where
    T: Send,
    F: Fn(usize, &mut [T]) + Sync + Send,
{
    if width == 0 || data.is_empty() {
        return;
    }
    if (data.len() / width) * span >= PAR_MIN_PIXELS && rayon::current_num_threads() > 1 {
        data.par_chunks_mut(width)
            .enumerate()
            .for_each(|(i, row)| f(i, row));
    } else {
        data.chunks_mut(width)
            .enumerate()
            .for_each(|(i, row)| f(i, row));
    }
}
