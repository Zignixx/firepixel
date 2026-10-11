//! Entwicklungswerkzeug: rechnet ein Modell auf einem Bild und schreibt Eingabe und Ausgaben als
//! rohe `f32`-Dateien (Vergleich mit der Python-Referenz) sowie Zeiten.
//!
//! ```text
//! cargo run --release -p fp-ai --example ai_probe -- biref <modell.safetensors> <bild> <ausgabeordner> [größe]
//! cargo run --release -p fp-ai --example ai_probe -- sam <modell.safetensors> <bild> <ausgabeordner> x y [x y …]
//! ```

use fp_ai::cancel::Progress;
use fp_ai::{birefnet::BiRefNet, nn, pipeline, sam2, weights::Weights};
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

fn write_f32(p: &PathBuf, v: &[f32]) {
    let mut f = std::fs::File::create(p).expect("Datei");
    let mut b = Vec::with_capacity(v.len() * 4);
    for x in v {
        b.extend_from_slice(&x.to_le_bytes());
    }
    f.write_all(&b).expect("schreiben");
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let mode = a[1].as_str();
    let model = PathBuf::from(&a[2]);
    let img_path = PathBuf::from(&a[3]);
    let out = PathBuf::from(&a[4]);
    std::fs::create_dir_all(&out).expect("Ordner");
    let bytes = std::fs::read(&img_path).expect("Bild");
    let img = fp_io::jpeg_in::decode(&bytes).expect("JPEG").image;
    let prog = Progress::new();
    let t = Instant::now();
    let wt = Weights::load(&model).expect("Gewichte");
    println!("Gewichte gelesen: {:.2} s", t.elapsed().as_secs_f32());
    match mode {
        "biref" => {
            let s: usize = a.get(5).and_then(|x| x.parse().ok()).unwrap_or(1024);
            let t = Instant::now();
            let net = nn::pool().install(|| BiRefNet::load(wt)).expect("Modell");
            println!("Modell aufgebaut: {:.2} s ({:?})", t.elapsed().as_secs_f32(), net.swin_config());
            let x = pipeline::model_input(&img, img.bounds(), s);
            write_f32(&out.join("input.bin"), &x);
            for run in 0..2 {
                let t = Instant::now();
                let y = nn::pool().install(|| net.forward(&x, s, &prog)).expect("Vorwärts");
                println!("Lauf {run}: {:.2} s", t.elapsed().as_secs_f32());
                if run == 0 {
                    write_f32(&out.join("out.bin"), &y);
                }
            }
        }
        "sam" => {
            let t = Instant::now();
            let sam = nn::pool().install(|| sam2::Sam2::load(wt)).expect("Modell");
            println!("Modell aufgebaut: {:.2} s", t.elapsed().as_secs_f32());
            let x = pipeline::model_input(&img, img.bounds(), sam2::SIZE);
            write_f32(&out.join("input.bin"), &x);
            let mut emb = None;
            for run in 0..2 {
                let t = Instant::now();
                let e = nn::pool().install(|| sam.encode(&x, &prog)).expect("Encoder");
                println!("Encoder Lauf {run}: {:.2} s", t.elapsed().as_secs_f32());
                emb = Some(e);
            }
            let e = emb.expect("Einbettung");
            write_f32(&out.join("emb.bin"), &e.emb);
            write_f32(&out.join("s0.bin"), &e.s0);
            write_f32(&out.join("s1.bin"), &e.s1);
            let (w, h) = (img.width() as f32, img.height() as f32);
            let mut pts = Vec::new();
            let mut i = 5;
            while i + 1 < a.len() {
                let x: f32 = a[i].parse().unwrap_or(0.0);
                let y: f32 = a[i + 1].parse().unwrap_or(0.0);
                pts.push((x * 1024.0 / w, y * 1024.0 / h, 1));
                i += 2;
            }
            let p = sam2::Prompt { points: pts.clone(), multimask: pts.len() == 1, ..Default::default() };
            let mut res = None;
            for run in 0..3 {
                let t = Instant::now();
                let d = nn::pool().install(|| sam.decode(&e, &p)).expect("Decoder");
                println!("Decoder Lauf {run}: {:.1} ms (iou {:.3}, obj {:.2})", t.elapsed().as_secs_f64() * 1000.0, d.iou, d.object_score);
                res = Some(d);
            }
            let d = res.expect("Ergebnis");
            write_f32(&out.join("logits.bin"), &d.logits);
            let pj: Vec<String> = pts.iter().map(|(x, y, l)| format!("[{x}, {y}, {l}]")).collect();
            std::fs::write(out.join("sam.json"), format!("{{\"iou\": {}, \"obj\": {}, \"points\": [{}]}}", d.iou, d.object_score, pj.join(", "))).expect("json");
        }
        _ => eprintln!("Modus biref|sam"),
    }
}
