//! Commands menu extras (BA-104 / CMD-1): screen capture into a new document, a JSF script that
//! recreates a graphic symbol, and a shared adaptive palette over all open documents.

use super::DOC;
use crate::command::{CommandOutput, CommandRegistry, ParamKind, cmd};
use crate::error::{EngineError, Result};
use crate::export_backend::ExportRenderer;
use crate::session::Session;
use fp_color::Argb;
use fp_export::render::{Background, LayerFilter, PageRenderer as _, RenderRequest};
use fp_model::{Element, ElementKind, LayerKind, LayerSlot, Symbol, SymbolKind};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub fn register(r: &mut CommandRegistry) {
    r.add(
        cmd("commands.screenCapture")
            .label("Bildschirmfoto")
            .opt_default(
                "delay",
                ParamKind::Integer {
                    min: Some(0),
                    max: Some(10),
                },
                json!(0),
                "Countdown in Sekunden vor der Aufnahme",
            )
            .opt(
                "rect",
                ParamKind::Rect,
                "Bildschirmausschnitt in Bildschirmpixeln (ohne: ganzer Hauptbildschirm)",
            )
            .run(|s, p| {
                let delay = p.i64_or("delay", 0)?.clamp(0, 10) as u64;
                let rect = p
                    .opt_rect("rect")?
                    .map(|r| [r.left as i64, r.top as i64, r.right as i64, r.bottom as i64])
                    .filter(|r| r[2] > r[0] && r[3] > r[1]);
                let tmp = capture_temp_path();
                capture_screen(&tmp, delay, rect)?;
                let out = s.execute(
                    "file.open",
                    json!({"path": tmp.display().to_string(), "untitled": true}),
                );
                let _ = std::fs::remove_file(&tmp);
                let out = out?;
                let (w, h) = s
                    .doc()
                    .ok()
                    .and_then(|d| {
                        d.doc
                            .pages
                            .first()
                            .map(|p| (p.canvas.width, p.canvas.height))
                    })
                    .unwrap_or((0.0, 0.0));
                let mut v = out.value;
                if let Value::Object(m) = &mut v {
                    m.insert("width".into(), json!(w));
                    m.insert("height".into(), json!(h));
                }
                Ok(CommandOutput::session_with(v))
            }),
    );
    r.add(
        cmd("commands.symbolScript")
            .label("Symbolskript erstellen")
            .requires(DOC)
            .opt(
                "symbol",
                ParamKind::String,
                "Name des Grafiksymbols (ohne: Symbol der ausgewählten Instanz)",
            )
            .run(|s, p| {
                let d = s.doc()?;
                let sym = match p.opt_str("symbol")? {
                    Some(n) => d.doc.symbol_by_name(&n).map(|(_, x)| Arc::clone(x)),
                    None => d.selection.elements.iter().find_map(|id| {
                        let path = d.doc.find_element(*id)?;
                        match &d.doc.element_at(&path)?.kind {
                            ElementKind::Instance(i) => {
                                d.doc.symbol_by_id(i.symbol_id).map(|(_, x)| Arc::clone(x))
                            }
                            _ => None,
                        }
                    }),
                };
                let unavailable = |key: &str| EngineError::Unavailable {
                    name: "commands.symbolScript".into(),
                    reason: fp_i18n::t(key),
                };
                let sym = sym.ok_or_else(|| unavailable("err-symbol-script-no-instance"))?;
                if sym.kind() != SymbolKind::Graphic {
                    return Err(unavailable("err-symbol-script-not-graphic"));
                }
                Ok(CommandOutput::value(
                    json!({"name": sym.name, "script": symbol_script(&sym)}),
                ))
            }),
    );
    r.add(
        cmd("commands.sharedPalette")
            .label("Gemeinsame Palette")
            .opt_default(
                "colors",
                ParamKind::Integer {
                    min: Some(2),
                    max: Some(256),
                },
                json!(256),
                "Anzahl der Farben",
            )
            .opt_default(
                "swatches",
                ParamKind::Bool,
                json!(true),
                "Palette im Bedienfeld „Farbfelder“ zeigen",
            )
            .opt(
                "path",
                ParamKind::Path,
                "Palette zusätzlich in diese ACT-Datei schreiben",
            )
            .run(|s, p| {
                let n = p.i64_or("colors", 256)?.clamp(2, 256) as i32;
                let pal = shared_palette(s, n)?;
                let colors: Vec<Argb> = pal.iter().map(|c| Argb(c | 0xff00_0000)).collect();
                if let Some(path) = p.opt_str("path")? {
                    let bytes = super::panel::swatch_file_bytes("act", &colors);
                    fp_i18n::profile::guard_write(Path::new(&path));
                    std::fs::write(&path, bytes).map_err(|e| EngineError::Io(e.to_string()))?;
                }
                let hex: Vec<String> = colors.iter().map(|c| super::argb_hex(*c)).collect();
                if p.bool_or("swatches", true)? {
                    super::panel::set_swatches(s, &colors);
                    return Ok(CommandOutput::session_with(json!({"colors": hex})));
                }
                Ok(CommandOutput::value(json!({"colors": hex})))
            }),
    );
}

fn capture_counter() -> u64 {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// Shared adaptive palette (`n` colors) over every page of every open document (first state),
/// built with the export quantiser's global adaptive palette (one histogram over all images).
pub fn shared_palette(s: &Session, n: i32) -> Result<Vec<u32>> {
    let r = ExportRenderer::new(s.renderer());
    let mut pixels: Vec<u32> = Vec::new();
    for d in s.documents() {
        for page in 0..d.doc.pages.len() {
            let img = r
                .render(
                    &d.doc,
                    &RenderRequest {
                        page,
                        frame: 0,
                        layers: LayerFilter::All,
                        background: Background::Canvas,
                        area: None,
                    },
                )
                .map_err(EngineError::Failed)?;
            pixels.extend(img.data().iter().filter(|p| p[3] > 16).map(|p| {
                0xff00_0000 | (u32::from(p[0]) << 16) | (u32::from(p[1]) << 8) | u32::from(p[2])
            }));
        }
    }
    if pixels.is_empty() {
        return Err(EngineError::NoDocument);
    }
    let mut q = fp_export::quant::Quantizer::new(fp_export::quant::TRANSPARENCY_NONE, 0);
    let len = pixels.len();
    Ok(q.build_adaptive_global(&pixels, len, n, false, 0, &[])
        .to_vec())
}

/// JSF source that recreates the graphic symbol `sym` (paths with fill and stroke, texts) in the
/// current document and converts the new objects into a graphic symbol of the same name.
pub fn symbol_script(sym: &Symbol) -> String {
    let mut calls = Vec::new();
    let mut skipped = Vec::new();
    let elements: Vec<&Arc<Element>> = sym
        .root
        .layers
        .iter()
        .filter_map(|slot| match slot {
            LayerSlot::Own(l) if l.kind == LayerKind::Normal => l.cels.first(),
            _ => None,
        })
        .flat_map(|cel| cel.elements.iter())
        .collect();
    // symbol coordinates are centred on the origin: move the objects onto the canvas (20, 20)
    let mut ext = [f32::MAX, f32::MAX];
    for e in &elements {
        extent(e, &mut ext);
    }
    let off = if ext[0] == f32::MAX {
        (0.0, 0.0)
    } else {
        (20.0 - ext[0], 20.0 - ext[1])
    };
    // elements are stored top first; recreate bottom first so the stacking order stays
    for e in elements.iter().rev() {
        element_calls(e, off, &mut calls, &mut skipped);
    }
    let name = serde_json::to_string(&sym.name).unwrap_or_else(|_| "\"\"".into());
    let mut out = String::new();
    out.push_str(&format!(
        "// Symbol script for {name}, created by Firepixel (Commands > Create Symbol Script).\n\
         // Running it recreates the graphic symbol from its shapes, fills, strokes and texts.\n"
    ));
    for k in &skipped {
        out.push_str(&format!("// not recreated: {k}\n"));
    }
    out.push_str(
        "if (!fw.getDocumentDOM()) fp.run(\"file.new\", { width: 500, height: 500 });\n\
         var made = [];\n\
         function add(cmd, p) { var r = fp.run(cmd, p); if (r && r.id !== undefined) made.push(r.id); }\n",
    );
    for (c, p) in &calls {
        out.push_str(&format!("add(\"{c}\", {p});\n"));
    }
    out.push_str(&format!(
        "if (made.length) {{\n  fp.run(\"select.set\", {{ ids: made }});\n  fp.run(\"modify.symbol.convertToSymbol\", {{ kind: \"graphic\", name: {name} }});\n}}\n"
    ));
    out
}

fn hex(c: Argb) -> String {
    super::argb_hex(c)
}

/// Smallest x and y of the objects a symbol script recreates.
fn extent(e: &Element, ext: &mut [f32; 2]) {
    let mut put = |x: f32, y: f32| {
        ext[0] = ext[0].min(x);
        ext[1] = ext[1].min(y);
    };
    match &e.kind {
        ElementKind::Path(pe) => pe
            .contours
            .iter()
            .flat_map(|c| c.nodes.iter())
            .for_each(|n| put(n.anchor.x, n.anchor.y)),
        ElementKind::Text(t) => put(t.bounds.left, t.bounds.top),
        ElementKind::Group(g) => g.children.iter().for_each(|c| extent(c, ext)),
        _ => {}
    }
}

fn element_calls(
    e: &Element,
    (dx, dy): (f32, f32),
    calls: &mut Vec<(String, String)>,
    skipped: &mut Vec<String>,
) {
    match &e.kind {
        ElementKind::Path(pe) => {
            let contours: Vec<Value> = pe
                .contours
                .iter()
                .map(|c| {
                    let nodes: Vec<Value> = c
                        .nodes
                        .iter()
                        .map(|n| {
                            json!({
                                "anchor": [n.anchor.x + dx, n.anchor.y + dy],
                                "in": [n.pred.x + dx, n.pred.y + dy],
                                "out": [n.succ.x + dx, n.succ.y + dy],
                            })
                        })
                        .collect();
                    json!({"closed": c.closed, "nodes": nodes})
                })
                .collect();
            let a = &pe.attrs;
            let mut p = json!({"contours": contours, "fill": a.fill.is_some()});
            if a.fill.is_some() {
                p["fillColor"] = json!(hex(a.fill_color));
            }
            match &a.brush {
                Some(b) => {
                    p["strokeWidth"] = json!(b.diameter.max(0));
                    p["strokeColor"] = json!(hex(a.stroke_color));
                }
                None => p["strokeWidth"] = json!(0),
            }
            if let Some(n) = &e.common.name {
                p["name"] = json!(n);
            }
            calls.push(("draw.path".into(), p.to_string()));
        }
        ElementKind::Text(t) => {
            let text: String = t.runs.iter().map(|r| r.text.as_str()).collect();
            let first = t
                .resolved_runs()
                .first()
                .map(|(a, _)| a.clone())
                .unwrap_or_else(|| t.initial.clone());
            let size = first.size.unwrap_or(12.0);
            let mut p = json!({"text": text, "runs": t.runs, "size": size});
            if let Some(f) = &first.font {
                p["font"] = json!(f);
            }
            if let Some(c) = first.color {
                p["color"] = json!(hex(Argb(c)));
            }
            let b = t.bounds;
            if t.frame_type == Some(1) {
                p["rect"] = json!([b.left + dx, b.top + dy, b.right + dx, b.bottom + dy]);
            } else {
                p["anchor"] = json!([b.left + dx, b.top + dy + size]);
            }
            calls.push(("draw.text".into(), p.to_string()));
        }
        ElementKind::Group(g) => {
            for c in g.children.iter().rev() {
                element_calls(c, (dx, dy), calls, skipped);
            }
        }
        ElementKind::Bitmap(_) => skipped.push("bitmap".into()),
        ElementKind::Instance(_) => skipped.push("symbol instance".into()),
        ElementKind::CompoundShape(_) => skipped.push("compound shape".into()),
        _ => skipped.push("other object".into()),
    }
}

/// Captures the primary screen (or `rect` = [left, top, right, bottom] in screen pixels) after
/// `delay` seconds into the PNG file `out` with the system's own capture tools (no native code
/// in Firepixel).
pub fn capture_screen(out: &Path, delay: u64, rect: Option<[i64; 4]>) -> Result<()> {
    let fail = |m: String| EngineError::Failed(fp_i18n::tr!("err-screen-capture", detail = m));
    let _ = std::fs::remove_file(out);
    let status = capture_command(out, delay, rect)
        .and_then(|mut c| c.status().map_err(|e| e.to_string()))
        .map_err(fail)?;
    if !status.success() || !out.is_file() {
        return Err(fail(fp_i18n::tr!(
            "err-screen-capture-status",
            status = status.to_string()
        )));
    }
    Ok(())
}

#[cfg(windows)]
fn capture_command(
    out: &Path,
    delay: u64,
    rect: Option<[i64; 4]>,
) -> std::result::Result<std::process::Command, String> {
    use std::os::windows::process::CommandExt;
    let bounds = match rect {
        Some([l, t, r, b]) => format!(
            "$b = New-Object System.Drawing.Rectangle {l},{t},{},{};",
            r - l,
            b - t
        ),
        None => "$b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds;".into(),
    };
    let path = out.display().to_string().replace('\'', "''");
    let script = format!(
        "$ErrorActionPreference = 'Stop';\
         try {{ Add-Type -MemberDefinition '[DllImport(\"user32.dll\")] public static extern bool SetProcessDPIAware();' -Name Dpi -Namespace FpCapture; [void][FpCapture.Dpi]::SetProcessDPIAware() }} catch {{}};\
         Add-Type -AssemblyName System.Windows.Forms, System.Drawing;\
         Start-Sleep -Milliseconds {ms};\
         {bounds}\
         $bmp = New-Object System.Drawing.Bitmap $b.Width, $b.Height;\
         $g = [System.Drawing.Graphics]::FromImage($bmp);\
         $g.CopyFromScreen($b.Location, [System.Drawing.Point]::Empty, $b.Size);\
         $bmp.Save('{path}', [System.Drawing.Imaging.ImageFormat]::Png);\
         $g.Dispose(); $bmp.Dispose()",
        ms = delay * 1000
    );
    let mut c = std::process::Command::new("powershell.exe");
    c.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
    // CREATE_NO_WINDOW: no console window flashes up in the capture
    c.creation_flags(0x0800_0000);
    Ok(c)
}

#[cfg(target_os = "macos")]
fn capture_command(
    out: &Path,
    delay: u64,
    rect: Option<[i64; 4]>,
) -> std::result::Result<std::process::Command, String> {
    let mut c = std::process::Command::new("screencapture");
    c.arg("-x");
    if delay > 0 {
        c.arg(format!("-T{delay}"));
    }
    if let Some([l, t, r, b]) = rect {
        c.arg(format!("-R{l},{t},{},{}", r - l, b - t));
    }
    c.arg(out);
    Ok(c)
}

#[cfg(all(not(windows), not(target_os = "macos")))]
fn capture_command(
    out: &Path,
    delay: u64,
    rect: Option<[i64; 4]>,
) -> std::result::Result<std::process::Command, String> {
    std::thread::sleep(std::time::Duration::from_secs(delay));
    let has = |tool: &str| {
        std::env::var_os("PATH")
            .map(|p| std::env::split_paths(&p).any(|d| d.join(tool).is_file()))
            .unwrap_or(false)
    };
    if has("grim") {
        let mut c = std::process::Command::new("grim");
        if let Some([l, t, r, b]) = rect {
            c.args(["-g", &format!("{l},{t} {}x{}", r - l, b - t)]);
        }
        c.arg(out);
        return Ok(c);
    }
    if has("import") {
        let mut c = std::process::Command::new("import");
        c.args(["-window", "root"]);
        if let Some([l, t, r, b]) = rect {
            c.args(["-crop", &format!("{}x{}+{l}+{t}", r - l, b - t)]);
        }
        c.arg(out);
        return Ok(c);
    }
    if has("gnome-screenshot") {
        let mut c = std::process::Command::new("gnome-screenshot");
        c.arg("-f").arg(out);
        return Ok(c);
    }
    Err(fp_i18n::t("err-screen-capture-no-tool"))
}

/// Temporary file for one capture.
fn capture_temp_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "firepixel-capture-{}-{}.png",
        std::process::id(),
        capture_counter()
    ))
}
