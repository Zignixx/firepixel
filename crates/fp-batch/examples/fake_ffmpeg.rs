//! Test-Attrappe für ffmpeg (nur für `tests\anim_video.rs`; kein Teil des Programms).
//!
//! Versteht genau die Aufrufe, die `fp_batch::video::ffmpeg` macht:
//! - `-version` → Versionszeile
//! - `-i <datei>` ohne Ausgabe → Kopfdaten auf stderr wie ffmpeg, Exit 1
//! - `-ss S -i <datei> -t T -vf fps=F,scale=W:H… -frames:v N -f rawvideo -pix_fmt rgba pipe:1` → Frames
//!
//! Eingabedatei (Text): `FAKEVIDEO <breite> <höhe> <dauer_ms> <fps> [rotate90] [slow] [stall]`
//! oder `FAKEFAIL`. Frame i hat die Farbe R = i, G = (Zeit in 10 ms) mod 256, B = 128.
//! `slow`: 20 ms je Frame und Fortschritt in `<datei>.progress`; `stall`: nach dem Start 30 s nichts.

use std::io::Write;

fn arg_after<'a>(args: &'a [String], key: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == key)
        .and_then(|i| args.get(i + 1))
        .map(String::as_str)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "-version") {
        println!("ffmpeg version 9.9-fake Copyright (c) test");
        println!("configuration: --fake");
        return;
    }
    let Some(input) = arg_after(&args, "-i") else {
        eprintln!("missing input");
        std::process::exit(1);
    };
    let text = match std::fs::read_to_string(input) {
        Ok(t) => t,
        Err(_) => {
            eprintln!("{input}: No such file or directory");
            std::process::exit(1);
        }
    };
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.first() != Some(&"FAKEVIDEO") {
        eprintln!("{input}: Invalid data found when processing input");
        std::process::exit(1);
    }
    let num = |i: usize| {
        words
            .get(i)
            .and_then(|w| w.parse::<f64>().ok())
            .unwrap_or(0.0)
    };
    let (w, h, dur, fps) = (num(1) as u32, num(2) as u32, num(3), num(4));
    let flag = |f: &str| words.contains(&f);
    let raw = arg_after(&args, "-f") == Some("rawvideo");
    if !raw {
        let s = dur / 1000.0;
        eprintln!("Input #0, mov,mp4,m4a,3gp,3g2,mj2, from '{input}':");
        eprintln!(
            "  Duration: {:02}:{:02}:{:05.2}, start: 0.000000, bitrate: 1 kb/s",
            (s / 3600.0) as u32,
            ((s / 60.0) as u32) % 60,
            s % 60.0
        );
        eprintln!(
            "  Stream #0:0[0x1](und): Video: h264 (High) (avc1 / 0x31637661), yuv420p(progressive), {w}x{h} [SAR 1:1 DAR 4:3], 100 kb/s, {fps} fps, {fps} tbr, 12800 tbn (default)"
        );
        if flag("rotate90") {
            eprintln!("    Side data:");
            eprintln!("      displaymatrix: rotation of -90.00 degrees");
        }
        eprintln!("At least one output file must be specified");
        std::process::exit(1);
    }
    let ss = arg_after(&args, "-ss")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.0)
        * 1000.0;
    let t = arg_after(&args, "-t")
        .and_then(|v| v.parse::<f64>().ok())
        .map(|v| v * 1000.0);
    let vf = arg_after(&args, "-vf").unwrap_or("");
    let mut out_fps = fps;
    let (mut ow, mut oh) = (w, h);
    for part in vf.split(',') {
        if let Some(f) = part.strip_prefix("fps=") {
            out_fps = f.parse().unwrap_or(fps);
        } else if let Some(s) = part.strip_prefix("scale=") {
            let dims: Vec<&str> = s.split(':').collect();
            ow = dims.first().and_then(|v| v.parse().ok()).unwrap_or(w);
            oh = dims.get(1).and_then(|v| v.parse().ok()).unwrap_or(h);
        }
    }
    let max = arg_after(&args, "-frames:v")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(u64::MAX);
    let end = t.map(|t| (ss + t).min(dur)).unwrap_or(dur);
    let n = if end > ss {
        (((end - ss) * out_fps / 1000.0) - 1e-6).ceil() as u64
    } else {
        0
    };
    let n = n.min(max);
    if flag("stall") {
        std::thread::sleep(std::time::Duration::from_secs(30));
    }
    let progress = format!("{input}.progress");
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let mut buf = vec![0u8; ow as usize * oh as usize * 4];
    for i in 0..n {
        let time_ms = ss + i as f64 * 1000.0 / out_fps;
        let px = [
            (i % 256) as u8,
            ((time_ms / 10.0).round() as u64 % 256) as u8,
            128,
            255,
        ];
        for c in buf.as_chunks_mut::<4>().0 {
            c.copy_from_slice(&px);
        }
        if out.write_all(&buf).is_err() {
            return;
        }
        if flag("slow") {
            let _ = out.flush();
            let _ = std::fs::write(&progress, (i + 1).to_string());
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }
}
