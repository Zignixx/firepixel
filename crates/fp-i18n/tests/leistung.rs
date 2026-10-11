//! Leistungsmessung von `t()` (je Frame tausendfach aufgerufen). Läuft nur auf Wunsch:
//! `cargo test -p fp-i18n --release --test leistung -- --ignored --nocapture`.
//!
//! Zum Vergleich im selben Lauf: das frühere Verfahren (globaler Cache hinter einer `RwLock`,
//! Lesesperre bei jedem Aufruf), nachgebaut als `alt_t`. Jede Messung 5×, gezeigt wird das
//! Minimum (die Maschine ist mit parallelen Builds belastet).

use std::collections::HashMap;
use std::hint::black_box;
use std::sync::{Arc, OnceLock, RwLock};
use std::time::Instant;

fn alt_cache() -> &'static RwLock<HashMap<String, Arc<str>>> {
    static C: OnceLock<RwLock<HashMap<String, Arc<str>>>> = OnceLock::new();
    C.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Früheres Verfahren: Lesesperre + HashMap (SipHash) + neue Zeichenkette.
fn alt_t(key: &str) -> String {
    if let Ok(c) = alt_cache().read()
        && let Some(s) = c.get(key)
    {
        return s.to_string();
    }
    let s = fp_i18n::t(key);
    if let Ok(mut c) = alt_cache().write() {
        c.insert(key.to_string(), Arc::from(s.as_str()));
    }
    s
}

fn messen(name: &str, n: u32, mut f: impl FnMut(u32)) -> f64 {
    for i in 0..1000 {
        f(i);
    }
    let mut best = f64::MAX;
    for _ in 0..5 {
        let start = Instant::now();
        for i in 0..n {
            f(i);
        }
        best = best.min(start.elapsed().as_nanos() as f64 / n as f64);
    }
    println!("{name:<46} {best:>9.1} ns");
    best
}

fn parallel(name: &str, n: u32, f: fn(&str) -> usize) {
    const KEYS: [&str; 8] = ["btn-ok", "btn-cancel", "btn-yes", "btn-no", "btn-close", "btn-apply", "btn-browse", "language-name"];
    let mut best = f64::MAX;
    for _ in 0..3 {
        let start = Instant::now();
        let threads: Vec<_> = (0..8)
            .map(|_| {
                std::thread::spawn(move || {
                    let mut s = 0usize;
                    for i in 0..n {
                        s += f(KEYS[(i % 8) as usize]);
                    }
                    s
                })
            })
            .collect();
        let mut sink = 0;
        for th in threads {
            sink += th.join().unwrap_or(0);
        }
        black_box(sink);
        best = best.min(start.elapsed().as_nanos() as f64 / n as f64);
    }
    println!("{name:<46} {best:>9.1} ns je Aufruf und Thread");
}

#[test]
#[ignore]
fn t_aufrufe_messen() {
    fp_i18n::set_language("de");
    let keys = ["btn-ok", "btn-cancel", "btn-yes", "btn-no", "btn-close", "btn-apply", "btn-browse", "language-name"];
    let n = 1_000_000;
    println!("— 1 Thread, 8 Schlüssel abwechselnd —");
    let alt = messen("früher: RwLock + HashMap + String", n, |i| {
        black_box(alt_t(black_box(keys[(i % 8) as usize])));
    });
    let neu = messen("t(): Thread-Cache + String", n, |i| {
        black_box(fp_i18n::t(black_box(keys[(i % 8) as usize])));
    });
    let arc = messen("t_arc(): Thread-Cache, ohne neue Zeichenkette", n, |i| {
        black_box(fp_i18n::t_arc(black_box(keys[(i % 8) as usize])));
    });
    messen("t() unbekannter Schlüssel", n, |_| {
        black_box(fp_i18n::t(black_box("gibt-es-nicht")));
    });
    messen("has()", n, |i| {
        black_box(fp_i18n::has(black_box(keys[(i % 8) as usize])));
    });
    messen("tr!() Zahl, 100 wiederkehrende Werte", n / 10, |i| {
        black_box(fp_i18n::tr!("test-layers", n = i % 100));
    });
    messen("tr!() Zahl, jedes Mal neu (schlechtester Fall)", n / 20, |i| {
        black_box(fp_i18n::tr!("test-layers", n = i));
    });
    messen("tr!() Text-Platzhalter, gleicher Wert", n / 10, |_| {
        black_box(fp_i18n::tr!("test-unsaved", name = "a.fw.png"));
    });
    messen("shortcut_label(\"Strg+Umschalt+S\")", n / 10, |_| {
        black_box(fp_i18n::shortcut_label(black_box("Strg+Umschalt+S")));
    });
    println!("— 8 Threads gleichzeitig —");
    parallel("früher", n / 4, |k| alt_t(k).len());
    parallel("t()", n / 4, |k| fp_i18n::t(k).len());
    parallel("t_arc()", n / 4, |k| fp_i18n::t_arc(k).len());
    println!("— Frame mit 5000 Texten —");
    let frame = messen("t() × 5000 (Wert = ns je Frame)", 50, |_| {
        for i in 0..5000u32 {
            black_box(fp_i18n::t(keys[(i % 8) as usize]));
        }
    });
    println!("→ {:.0} µs je Frame mit 5000 Texten", frame / 1000.0);
    let start = Instant::now();
    fp_i18n::set_language("en");
    for i in 0..5000u32 {
        black_box(fp_i18n::t(keys[(i % 8) as usize]));
    }
    println!("→ Sprachwechsel + erster Frame (Caches neu füllen): {:.0} µs", start.elapsed().as_nanos() as f64 / 1000.0);
    fp_i18n::set_language("de");
    println!("Faktor früher/t(): {:.2}, früher/t_arc(): {:.2}", alt / neu, alt / arc);
}
