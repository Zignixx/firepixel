//! HTML-Bericht `batch-report.html` (SPEC-BATCH §2.8, P2; BATCH-11b).
//!
//! Eine einzelne, eigenständige HTML-Datei neben `batch-report.json` im Ausgabeordner: Kopf mit
//! Rezept und Zeit, Kacheln (fertig, übersprungen, Fehler, Größe vorher → nachher, Dauer), eine
//! Fehlerliste (Datei, Schritt, Meldung) und je Datei eine Karte mit **Vorschaubildern vorher und
//! nachher** (als kleine PNG eingebettet, die Datei bleibt also auch nach dem Verschieben vollständig),
//! Maßen, Größen und Hinweisen. Helles und dunkles Erscheinungsbild folgen dem System
//! (`prefers-color-scheme`); keine Skripte, keine Netzzugriffe. Texte in der Sprache des Laufs
//! (`batch-html-*` in `locales\<code>\batch_finish.ftl`).

use crate::error::{BatchError, Result};
use crate::item::{self, ItemKind};
use crate::report::{ItemReport, ItemStatus, Report};
use rayon::prelude::*;
use std::path::{Path, PathBuf};

/// Kantenlänge der eingebetteten Vorschaubilder (Pixel).
pub const THUMB_SIZE: u32 = 112;
/// Höchstens so viele Dateien bekommen Vorschaubilder (große Läufe bleiben schnell und klein).
pub const MAX_THUMBS: usize = 400;
/// Größere Quellen werden nicht für ein Vorschaubild dekodiert.
const MAX_THUMB_SOURCE_BYTES: u64 = 64 * 1024 * 1024;

/// Pfad des HTML-Berichts zu einem JSON-Bericht: `batch-report.json` → `batch-report.html`,
/// `x.json` → `x.html`.
pub fn html_path_for(json_path: &Path) -> PathBuf {
    if json_path
        .file_name()
        .is_some_and(|n| n == crate::report::REPORT_FILE)
    {
        json_path.with_file_name(crate::report::REPORT_HTML_FILE)
    } else {
        json_path.with_extension("html")
    }
}

/// Text sicher für HTML.
pub fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\'' => o.push_str("&#39;"),
            c => o.push(c),
        }
    }
    o
}

/// Base64 (Standardalphabet mit `=`), für `data:`-Adressen.
pub fn base64(data: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut o = String::with_capacity(data.len().div_ceil(3) * 4);
    for ch in data.chunks(3) {
        let b = [ch[0], *ch.get(1).unwrap_or(&0), *ch.get(2).unwrap_or(&0)];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        o.push(A[(n >> 18) as usize & 63] as char);
        o.push(A[(n >> 12) as usize & 63] as char);
        o.push(if ch.len() > 1 {
            A[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        o.push(if ch.len() > 2 {
            A[n as usize & 63] as char
        } else {
            '='
        });
    }
    o
}

/// Vorschaubild einer Datei: (PNG als `data:`-Adresse, Breite, Höhe der Datei). `None`, wenn die
/// Datei fehlt, zu groß ist oder nicht gelesen werden kann (SVG, Video ohne ffmpeg …).
fn thumb(path: &Path, decode: bool) -> (Option<String>, Option<(u32, u32)>) {
    let Ok(info) = item::probe(path) else {
        return (None, None);
    };
    let dims = (info.width > 0).then_some((info.width, info.height));
    if !decode
        || info.bytes > MAX_THUMB_SOURCE_BYTES
        || matches!(info.kind, ItemKind::Video | ItemKind::Other)
    {
        return (None, dims);
    }
    let img = match item::load(&info, false).and_then(|it| it.preview_image()) {
        Ok(i) => i,
        Err(_) => return (None, dims),
    };
    let dims = dims.or(Some((img.width() as u32, img.height() as u32)));
    let (w, h) = (img.width() as u32, img.height() as u32);
    if w == 0 || h == 0 {
        return (None, dims);
    }
    let s = (THUMB_SIZE as f64 / w.max(h) as f64).min(1.0);
    let (tw, th) = (
        ((w as f64 * s).round() as u32).max(1),
        ((h as f64 * s).round() as u32).max(1),
    );
    let small = if s < 1.0 {
        fp_raster::resample::resize(&img, tw, th, fp_raster::Sampler::Bilinear)
    } else {
        img
    };
    let raw: Vec<u8> = small.data().iter().flat_map(|p| *p).collect();
    let mut buf = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut buf, small.width() as u32, small.height() as u32);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_compression(png::Compression::Fast);
        let ok = enc
            .write_header()
            .and_then(|mut w| w.write_image_data(&raw));
        if ok.is_err() {
            return (None, dims);
        }
    }
    (
        Some(format!("data:image/png;base64,{}", base64(&buf))),
        dims,
    )
}

/// Änderung in Prozent als Text („−91 %“, „+4 %“).
fn percent(before: u64, after: u64) -> String {
    if before == 0 {
        return String::new();
    }
    let p = (after as f64 / before as f64 - 1.0) * 100.0;
    let sign = if p < -0.05 {
        "−"
    } else if p > 0.05 {
        "+"
    } else {
        "±"
    };
    format!(
        "{sign}{}\u{a0}%",
        fp_i18n::fmt_number(p.abs().round(), 0, 0)
    )
}

fn file_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| p.display().to_string())
}

/// Relativer Verweis vom Berichtsordner auf eine Ausgabedatei (nur innerhalb des Ordners).
fn rel_link(dir: &Path, target: &Path) -> Option<String> {
    let rel = target.strip_prefix(dir).ok()?;
    let parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();
    Some(
        parts
            .iter()
            .map(|p| url_part(p))
            .collect::<Vec<_>>()
            .join("/"),
    )
}

fn url_part(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                o.push(b as char)
            }
            _ => o.push_str(&format!("%{b:02X}")),
        }
    }
    o
}

/// Zeit aus dem ISO-Text („2026-10-13T14:03:11+02:00“ → „2026-10-13 14:03“).
fn short_time(iso: &str) -> String {
    let s = iso.replace('T', " ");
    s.chars().take(16).collect()
}

/// Dauer des Laufs in Sekunden (aus `started`/`finished`).

fn status_key(s: ItemStatus) -> (&'static str, &'static str) {
    match s {
        ItemStatus::Done => ("batch-html-status-done", "ok"),
        ItemStatus::Planned => ("batch-html-status-planned", "info"),
        ItemStatus::Skipped => ("batch-html-status-skipped", "warn"),
        ItemStatus::Failed => ("batch-html-status-failed", "err"),
        ItemStatus::Cancelled | ItemStatus::Waiting => ("batch-html-status-cancelled", "muted"),
    }
}

/// Vorschaubilder und Maße je Eintrag (parallel).
struct Thumbs {
    before: Option<String>,
    after: Option<String>,
    dims_before: Option<(u32, u32)>,
    dims_after: Option<(u32, u32)>,
}

fn collect_thumbs(items: &[ItemReport], with_images: bool) -> Vec<Thumbs> {
    let lang = fp_i18n::current_language();
    items
        .par_iter()
        .enumerate()
        .map(|(k, it)| {
            fp_i18n::with_language(&lang, || {
                let img = with_images && k < MAX_THUMBS;
                let (before, dims_before) = thumb(&it.source, img);
                let (after, dims_after) = match (&it.target, it.status) {
                    (Some(t), ItemStatus::Done) => thumb(t, img),
                    _ => (None, None),
                };
                Thumbs {
                    before,
                    after,
                    dims_before,
                    dims_after,
                }
            })
        })
        .collect()
}

const CSS: &str = r#"
:root{--bg:#eef0f3;--card:#ffffff;--text:#1d2330;--weak:#667085;--line:#dfe3ea;--accent:#e8590c;--accent2:#f59f00;
--ok:#2f9e44;--warn:#e67700;--err:#e03131;--info:#1c7ed6;--chip:#f1f3f5;--check:#e9ecef;--shadow:0 2px 10px rgba(16,24,40,.08)}
@media (prefers-color-scheme:dark){:root{--bg:#1b1e24;--card:#262a32;--text:#e8eaef;--weak:#9aa3b2;--line:#363b45;
--chip:#30353f;--check:#3a3f49;--shadow:0 2px 12px rgba(0,0,0,.35)}}
*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--text);font:14px/1.45 "Segoe UI",system-ui,-apple-system,sans-serif}
.wrap{max-width:1180px;margin:0 auto;padding:16px}
.banner{position:relative;overflow:hidden;border-radius:8px;padding:18px 20px;color:#fff;
background:linear-gradient(115deg,#7a2a05 0%,#c2410c 45%,#f08c00 100%);box-shadow:var(--shadow)}
.banner:after{content:"";position:absolute;inset:0;opacity:.18;
background-image:linear-gradient(90deg,rgba(255,255,255,.9) 1px,transparent 1px),linear-gradient(rgba(255,255,255,.9) 1px,transparent 1px);
background-size:10px 10px;-webkit-mask-image:linear-gradient(90deg,transparent 35%,#000 100%);mask-image:linear-gradient(90deg,transparent 35%,#000 100%)}
.banner h1{margin:0;font-size:22px;font-weight:700;letter-spacing:.2px}.banner p{margin:4px 0 0;opacity:.92}
.tiles{display:grid;grid-template-columns:repeat(auto-fit,minmax(150px,1fr));gap:16px;margin:16px 0}
.tile{background:var(--card);border-radius:8px;padding:12px 14px;box-shadow:var(--shadow);border-left:4px solid var(--line)}
.tile b{display:block;font-size:22px;line-height:1.2}.tile span{color:var(--weak);font-size:12px}
.tile.ok{border-left-color:var(--ok)}.tile.warn{border-left-color:var(--warn)}.tile.err{border-left-color:var(--err)}.tile.size{border-left-color:var(--accent)}.tile.time{border-left-color:var(--info)}
h2{font-size:15px;margin:22px 0 10px;font-weight:700}
.card{background:var(--card);border-radius:8px;box-shadow:var(--shadow)}
table.errors{width:100%;border-collapse:collapse}.errors th,.errors td{text-align:left;padding:8px 12px;border-bottom:1px solid var(--line);vertical-align:top}
.errors th{color:var(--weak);font-weight:600;font-size:12px}.errors tr:last-child td{border-bottom:0}.errors td.msg{color:var(--err)}
.grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(340px,1fr));gap:16px}
.item{padding:12px;display:flex;flex-direction:column;gap:8px;border-top:3px solid var(--line)}
.item.ok{border-top-color:var(--ok)}.item.warn{border-top-color:var(--warn)}.item.err{border-top-color:var(--err)}.item.info{border-top-color:var(--info)}
.pics{display:flex;align-items:center;gap:10px}.pic{flex:1;text-align:center;min-width:0}
.box{height:120px;border-radius:6px;display:flex;align-items:center;justify-content:center;overflow:hidden;
background-color:var(--chip);background-image:linear-gradient(45deg,var(--check) 25%,transparent 25%),linear-gradient(-45deg,var(--check) 25%,transparent 25%),linear-gradient(45deg,transparent 75%,var(--check) 75%),linear-gradient(-45deg,transparent 75%,var(--check) 75%);
background-size:12px 12px;background-position:0 0,0 6px,6px -6px,-6px 0}
.box img{max-width:112px;max-height:112px;image-rendering:auto}.box em{color:var(--weak);font-size:12px;font-style:normal;background:var(--card);padding:2px 6px;border-radius:4px}
.cap{color:var(--weak);font-size:12px;margin-top:4px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.arrow{color:var(--weak);font-size:18px}
.name{font-weight:600;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.name a{color:inherit}
.meta{display:flex;flex-wrap:wrap;gap:6px;align-items:center;color:var(--weak);font-size:12px}
.chip{display:inline-block;padding:1px 8px;border-radius:10px;font-size:12px;font-weight:600;background:var(--chip)}
.chip.ok{color:var(--ok)}.chip.warn{color:var(--warn)}.chip.err{color:var(--err)}.chip.info{color:var(--info)}.chip.muted{color:var(--weak)}
.delta.down{color:var(--ok);font-weight:600}.delta.up{color:var(--warn);font-weight:600}
.note{font-size:12px;color:var(--warn)}.note.err{color:var(--err)}
footer{color:var(--weak);font-size:12px;text-align:center;margin:24px 0 8px}
"#;

/// Baut den HTML-Bericht. `with_images` = Vorschaubilder einbetten (sonst nur Zahlen).
pub fn render(report: &Report, out_dir: Option<&Path>, with_images: bool) -> String {
    let t = fp_i18n::t;
    let thumbs = collect_thumbs(&report.items, with_images);
    let tt = &report.totals;
    let mut h = String::with_capacity(16 * 1024 + report.items.len() * 2048);
    let lang = fp_i18n::current_language();
    h.push_str(&format!("<!doctype html>\n<html lang=\"{}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">", esc(&lang)));
    h.push_str(&format!("<meta name=\"generator\" content=\"Firepixel\"><title>{} – {}</title><style>{CSS}</style></head><body><div class=\"wrap\">", esc(&t("batch-html-title")), esc(&report.recipe)));
    // Kopf
    let when = fp_i18n::tr!(
        "batch-html-when",
        start = short_time(&report.started),
        end = short_time(&report.finished)
    );
    let mut sub = vec![
        fp_i18n::tr!("batch-html-recipe", name = report.recipe.clone()),
        when,
    ];
    if report.dry_run {
        sub.push(t("batch-html-dry-run"));
    }
    if report.cancelled {
        sub.push(t("batch-html-cancelled-run"));
    }
    h.push_str(&format!(
        "<header class=\"banner\"><h1>{}</h1><p>{}</p></header>",
        esc(&t("batch-html-title")),
        esc(&sub.join(" · "))
    ));
    // Kacheln
    h.push_str("<section class=\"tiles\">");
    let tile = |h: &mut String, cls: &str, big: String, label: String| {
        h.push_str(&format!(
            "<div class=\"tile {cls}\"><b>{}</b><span>{}</span></div>",
            esc(&big),
            esc(&label)
        ))
    };
    if report.dry_run {
        tile(
            &mut h,
            "ok",
            tt.planned.to_string(),
            t("batch-html-status-planned"),
        );
    } else {
        tile(&mut h, "ok", tt.done.to_string(), t("batch-html-tile-done"));
    }
    tile(
        &mut h,
        "warn",
        tt.skipped.to_string(),
        t("batch-html-tile-skipped"),
    );
    tile(
        &mut h,
        "err",
        tt.failed.to_string(),
        t("batch-html-tile-failed"),
    );
    if tt.cancelled > 0 {
        tile(
            &mut h,
            "",
            tt.cancelled.to_string(),
            t("batch-html-tile-cancelled"),
        );
    }
    if tt.bytes_out > 0 {
        tile(
            &mut h,
            "size",
            format!(
                "{} → {}",
                fp_i18n::fmt_size(tt.bytes_in),
                fp_i18n::fmt_size(tt.bytes_out)
            ),
            fp_i18n::tr!(
                "batch-html-tile-size",
                change = percent(tt.bytes_in, tt.bytes_out)
            ),
        );
    } else {
        tile(
            &mut h,
            "size",
            fp_i18n::fmt_size(tt.bytes_in),
            t("batch-html-tile-size-in"),
        );
    }
    if let Some(s) = report.wall_seconds() {
        tile(
            &mut h,
            "time",
            fp_i18n::tr!("batch-html-seconds", s = fp_i18n::fmt_number(s, 1, 1)),
            t("batch-html-tile-duration"),
        );
    }
    h.push_str("</section>");
    // Hinweise zum ganzen Lauf
    for w in &report.warnings {
        h.push_str(&format!("<p class=\"note\">⚠ {}</p>", esc(w)));
    }
    // Fehlerliste
    let failed: Vec<&ItemReport> = report
        .items
        .iter()
        .filter(|i| i.status == ItemStatus::Failed)
        .collect();
    if !failed.is_empty() {
        h.push_str(&format!("<h2>{}</h2><div class=\"card\"><table class=\"errors\"><tr><th>{}</th><th>{}</th><th>{}</th></tr>", esc(&fp_i18n::tr!("batch-html-errors", n = failed.len())), esc(&t("batch-html-col-file")), esc(&t("batch-html-col-step")), esc(&t("batch-html-col-message"))));
        for i in failed {
            let step = i
                .step
                .as_deref()
                .map(crate::step::step_label)
                .unwrap_or_else(|| "—".into());
            h.push_str(&format!(
                "<tr><td title=\"{}\">{}</td><td>{}</td><td class=\"msg\">{}</td></tr>",
                esc(&i.source.display().to_string()),
                esc(&file_name(&i.source)),
                esc(&step),
                esc(i.error.as_deref().unwrap_or(""))
            ));
        }
        h.push_str("</table></div>");
    }
    // Dateien
    h.push_str(&format!(
        "<h2>{}</h2><section class=\"grid\">",
        esc(&fp_i18n::tr!("batch-html-files", n = report.items.len()))
    ));
    let none = esc(&t("batch-html-no-preview"));
    for (it, th) in report.items.iter().zip(&thumbs) {
        let (sk, cls) = status_key(it.status);
        let card_cls = if it.status == ItemStatus::Done && !it.warnings.is_empty() {
            "warn"
        } else {
            cls
        };
        h.push_str(&format!("<article class=\"card item {card_cls}\">"));
        let name = esc(&file_name(&it.source));
        let target_name = it.target.as_deref().map(file_name).unwrap_or_default();
        let link = match (&it.target, out_dir, it.status) {
            (Some(tg), Some(d), ItemStatus::Done) => rel_link(d, tg),
            _ => None,
        };
        let title = match (&link, it.status) {
            (Some(l), _) => format!("{name} → <a href=\"{}\">{}</a>", esc(l), esc(&target_name)),
            (None, ItemStatus::Done | ItemStatus::Planned) if !target_name.is_empty() => {
                format!("{name} → {}", esc(&target_name))
            }
            _ => name.clone(),
        };
        h.push_str(&format!(
            "<div class=\"name\" title=\"{}\">{title}</div>",
            esc(&it.source.display().to_string())
        ));
        // Bilder
        let pic = |img: &Option<String>, dims: Option<(u32, u32)>, cap: &str| -> String {
            let inner = match img {
                Some(d) => format!("<img alt=\"\" src=\"{d}\">"),
                None => format!("<em>{none}</em>"),
            };
            let dim = dims.map(|(w, h)| format!(" · {w}×{h}")).unwrap_or_default();
            format!(
                "<div class=\"pic\"><div class=\"box\">{inner}</div><div class=\"cap\">{}{}</div></div>",
                esc(cap),
                esc(&dim)
            )
        };
        h.push_str("<div class=\"pics\">");
        h.push_str(&pic(&th.before, th.dims_before, &t("batch-html-before")));
        if it.status == ItemStatus::Done {
            h.push_str("<span class=\"arrow\">→</span>");
            h.push_str(&pic(&th.after, th.dims_after, &t("batch-html-after")));
        }
        h.push_str("</div>");
        // Zahlen
        let mut meta = format!("<span class=\"chip {cls}\">{}</span>", esc(&t(sk)));
        if it.status == ItemStatus::Done && it.bytes_out > 0 {
            let p = percent(it.bytes_in, it.bytes_out);
            let dir = if it.bytes_out < it.bytes_in {
                "down"
            } else {
                "up"
            };
            meta.push_str(&format!(
                "<span>{} → {}</span><span class=\"delta {dir}\">{}</span>",
                esc(&fp_i18n::fmt_size(it.bytes_in)),
                esc(&fp_i18n::fmt_size(it.bytes_out)),
                esc(&p)
            ));
        } else {
            meta.push_str(&format!(
                "<span>{}</span>",
                esc(&fp_i18n::fmt_size(it.bytes_in))
            ));
        }
        if it.ms > 0 {
            meta.push_str(&format!(
                "<span>{}</span>",
                esc(&fp_i18n::tr!("batch-html-ms", ms = it.ms))
            ));
        }
        h.push_str(&format!("<div class=\"meta\">{meta}</div>"));
        for x in &it.extra_targets {
            h.push_str(&format!(
                "<div class=\"meta\">{}</div>",
                esc(&fp_i18n::tr!("batch-html-also", name = file_name(x)))
            ));
        }
        if let Some(e) = &it.error {
            h.push_str(&format!("<div class=\"note err\">✕ {}</div>", esc(e)));
        }
        for w in &it.warnings {
            h.push_str(&format!("<div class=\"note\">⚠ {}</div>", esc(w)));
        }
        h.push_str("</article>");
    }
    h.push_str("</section>");
    h.push_str(&format!(
        "<footer>{}</footer></div></body></html>\n",
        esc(&fp_i18n::tr!(
            "batch-html-footer",
            time = short_time(&report.finished)
        ))
    ));
    h
}

/// Schreibt den HTML-Bericht (atomar) nach `path`; Verweise relativ zum Ordner von `path`.
pub fn write(report: &Report, path: &Path) -> Result<()> {
    let html = render(report, path.parent(), true);
    crate::output::write_bytes_atomic(path, html.as_bytes())
        .map_err(|e| BatchError::Io(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_and_escape() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(
            esc("<a href=\"x\">&'</a>"),
            "&lt;a href=&quot;x&quot;&gt;&amp;&#39;&lt;/a&gt;"
        );
        assert_eq!(url_part("a b#ä.png"), "a%20b%23%C3%A4.png");
        assert_eq!(
            html_path_for(Path::new("C:\\o\\batch-report.json")),
            PathBuf::from("C:\\o\\batch-report.html")
        );
        assert_eq!(html_path_for(Path::new("r.json")), PathBuf::from("r.html"));
        assert_eq!(percent(1000, 90), "−91\u{a0}%");
        assert_eq!(percent(100, 104), "+4\u{a0}%");
    }
}
