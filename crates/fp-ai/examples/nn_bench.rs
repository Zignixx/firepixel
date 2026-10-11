//! Entwicklungswerkzeug: misst die Grundoperationen der KI-Inferenz für typische Formen.
//! `cargo run --release -p fp-ai --example nn_bench`

use fp_ai::nn::{self, Conv2d};
use rayon::prelude::*;
use std::time::Instant;

fn time(name: &str, n: usize, mut f: impl FnMut()) {
    f();
    let t = Instant::now();
    for _ in 0..n {
        f();
    }
    println!("{name}: {:.1} ms", t.elapsed().as_secs_f64() * 1000.0 / n as f64);
}

fn rnd(n: usize) -> Vec<f32> {
    (0..n).map(|i| ((i * 2_654_435_761usize) % 1000) as f32 / 1000.0 - 0.5).collect()
}

fn main() {
    nn::pool().install(|| {
        {
            let (n, cin, cout) = (17689usize, 192usize, 576usize);
            let x = rnd(n * cin);
            let wt = rnd(cin * cout);
            let mut reuse = vec![0.0f32; n * cout];
            time("linear 17689x192->576 Ausgabe wiederverwendet", 5, || nn::gemm(&x, n, cin, &wt, cout, false, &mut reuse, false, true));
            time("linear 17689x192->576 Ausgabe neu (vec!)", 5, || {
                let mut o = vec![0.0f32; n * cout];
                nn::gemm(&x, n, cin, &wt, cout, false, &mut o, false, true);
                std::hint::black_box(&o);
            });
            time("linear 17689x192->576 Ausgabe neu, vorher seriell genullt", 5, || {
                let mut o: Vec<f32> = Vec::with_capacity(n * cout);
                o.resize(n * cout, 0.0);
                nn::gemm(&x, n, cin, &wt, cout, false, &mut o, false, true);
                std::hint::black_box(&o);
            });
            time("vec![0; 10M] alle Seiten parallel beschreiben", 5, || {
                let mut o = vec![0.0f32; n * cout];
                o.par_chunks_mut(1024).for_each(|c| c[0] = 1.0);
                std::hint::black_box(&o);
            });
        }
        for (n, cin, cout) in [(65536, 96, 288), (65536, 96, 384), (65536, 384, 96), (16384, 192, 768), (4096, 384, 1536), (4096, 1536, 384), (1024, 768, 3072)] {
            let x = rnd(n * cin);
            let w = rnd(cout * cin);
            let gmac = (n * cin * cout) as f64 / 1e9;
            let mut out = vec![0.0f32; n * cout];
            let t = Instant::now();
            let reps = 5;
            for _ in 0..reps {
                nn::gemm(&x, n, cin, &w, cout, true, &mut out, false, true);
            }
            let a = t.elapsed().as_secs_f64() / reps as f64;
            let t = Instant::now();
            for _ in 0..reps {
                let rows = 1024;
                out.par_chunks_mut(rows * cout).enumerate().for_each(|(i, oc)| {
                    let nr = oc.len() / cout;
                    nn::gemm(&x[i * rows * cin..(i * rows + nr) * cin], nr, cin, &w, cout, true, oc, false, false);
                });
            }
            let b = t.elapsed().as_secs_f64() / reps as f64;
            // Gewichte vorab transponiert ([cin, cout]): b ohne Transponierung
            let wt = nn::transpose(&w, cout, cin);
            let t = Instant::now();
            for _ in 0..reps {
                nn::gemm(&x, n, cin, &wt, cout, false, &mut out, false, true);
            }
            let c = t.elapsed().as_secs_f64() / reps as f64;
            println!(
                "linear {n}x{cin}->{cout}: faer-par {:.1} ms ({:.0} GMAC/s), 1024er-Blöcke {:.1} ms ({:.0}), vortransponiert {:.1} ms ({:.0})",
                a * 1e3,
                gmac / a,
                b * 1e3,
                gmac / b,
                c * 1e3,
                gmac / c
            );
        }
        for (cin, cout, k, hw) in [(64usize, 256usize, 7usize, 256usize), (64, 147, 7, 256), (240, 64, 3, 256), (64, 96, 3, 256), (3, 64, 3, 1024), (64, 1, 3, 1024), (2880, 64, 3, 32), (3072, 64, 3, 32)] {
            let c = Conv2d { w: rnd(cout * cin * k * k), b: None, cout, cin, k, stride: 1, pad: k / 2, dil: 1 };
            let x = rnd(cin * hw * hw);
            let gmac = (cout * cin * k * k * hw * hw) as f64 / 1e9;
            let t = Instant::now();
            let _ = c.forward(&x, hw, hw);
            let _ = c.forward(&x, hw, hw);
            let a = t.elapsed().as_secs_f64() / 2.0;
            println!("conv {cin}->{cout} k{k} @{hw}²: {:.1} ms ({:.0} GMAC/s)", a * 1e3, gmac / a);
        }
        let (h, w, cin, k) = (256usize, 256usize, 64usize, 7usize);
        let c = Conv2d { w: rnd(256 * cin * k * k), b: None, cout: 256, cin, k, stride: 1, pad: 3, dil: 1 };
        let x = rnd(cin * h * w);
        let off = rnd(2 * k * k * h * w);
        let m = vec![1.0f32; k * k * h * w];
        time("deform 64->256 k7 @256²", 2, || {
            let _ = nn::deform_conv2d(&x, h, w, &c, &off, &m);
        });
        time("vec![0; 25M] + parallel schreiben", 3, || {
            let mut v = vec![0.0f32; 25 << 20];
            v.par_chunks_mut(16384).for_each(|c| c[0] = 1.0);
            std::hint::black_box(&v);
        });
        let mut reuse = vec![0.0f32; 25 << 20];
        time("vorhandene 25M parallel schreiben", 3, || {
            reuse.par_chunks_mut(16384).for_each(|c| c.fill(1.0));
        });
        let ex = rnd(1 << 24);
        time("exp 16M (parallel)", 3, || {
            let s: f32 = ex.par_chunks(16384).map(|c| c.iter().map(|v| (v * 4.0).exp()).sum::<f32>()).sum();
            std::hint::black_box(s);
        });
        let mut big = rnd(65536 * 384);
        time("gelu 25M", 3, || nn::gelu_inplace(&mut big));
        let (nb, n, heads, d) = (1369usize, 49usize, 3usize, 32usize);
        let qkv = rnd(nb * n * 3 * heads * d);
        let pk = |off| nn::Packed { buf: &qkv, stride: 3 * heads * d, off };
        time("fensteraufmerksamkeit 1369x49 (3 Köpfe)", 3, || {
            let _ = nn::attention_packed(pk(0), pk(heads * d), pk(2 * heads * d), nb, n, n, heads, d, 0.2, None);
        });
        {
            let (m, kk, n) = (228usize, 96usize, 4096usize);
            let a = rnd(m * kk);
            let b = rnd(kk * n);
            let mut c = vec![0.0f32; m * n];
            time("gemm seq 228x96x4096", 5, || nn::gemm(&a, m, kk, &b, n, false, &mut c, false, false));
            let v = rnd(n * kk);
            let mut o = vec![0.0f32; m * kk];
            time("gemm seq 228x4096x96", 5, || nn::gemm(&c, m, n, &v, kk, false, &mut o, false, false));
            let mut s2 = c.clone();
            time("softmax 228x4096", 5, || nn::softmax_rows(&mut s2, n));
            time("72× gemm seq 228x96x4096 parallel", 3, || {
                (0..72).into_par_iter().for_each(|_| {
                    let mut c = nn::scratch_take(m * n);
                    nn::gemm(&a, m, kk, &b, n, false, &mut c, false, false);
                    nn::scratch_give(c);
                })
            });
            time("72× softmax 228x4096 parallel", 3, || {
                (0..72).into_par_iter().for_each(|_| {
                    let mut c = nn::scratch_take(m * n);
                    nn::softmax_rows(&mut c, n);
                    nn::scratch_give(c);
                })
            });
            time("72× exp(0) 933K parallel", 3, || {
                (0..72).into_par_iter().for_each(|i| {
                    let mut s = 0.0f32;
                    for j in 0..m * n {
                        s += ((j % 3) as f32 * 0.0 - (i as f32) * 0.0).exp();
                    }
                    std::hint::black_box(s);
                })
            });
            time("72× exp(x) 933K parallel", 3, || {
                (0..72).into_par_iter().for_each(|i| {
                    let mut s = 0.0f32;
                    for j in 0..m * n {
                        s += (((j % 1000) as f32) * -0.001 - (i as f32) * 0.001).exp();
                    }
                    std::hint::black_box(s);
                })
            });
            time("72× softmax auf Zufallswerten parallel", 3, || {
                (0..72).into_par_iter().for_each(|_| {
                    let mut x = c.clone();
                    nn::softmax_rows(&mut x, n);
                    std::hint::black_box(&x);
                })
            });
            time("72× gemm seq 228x4096x96 parallel", 3, || {
                (0..72).into_par_iter().for_each(|_| {
                    let mut o = vec![0.0f32; m * kk];
                    nn::gemm(&c, m, n, &v, kk, false, &mut o, false, false);
                })
            });
        }
        let q = rnd(4096 * 384);
        time("globale Aufmerksamkeit 4096 (4 Köpfe, 96)", 3, || {
            let _ = nn::attention(&q, &q, &q, 1, 4096, 4096, 4, 96, 0.1, None);
        });
    });
}
