//! `firepixel-cli`: Kommandozeile von Firepixel (ARCHITEKTUR §6).
//!
//! Alle Unterbefehle arbeiten headless über dasselbe Befehlsregister wie Oberfläche, MCP und JSF.
//! Mit `--json` sind die Ausgaben maschinenlesbar (ein JSON-Objekt auf stdout, Fehler als
//! `{"ok": false, "error": …}` mit Exit-Code 1).

use clap::{ArgMatches, CommandFactory, FromArgMatches, Parser, Subcommand};
use fp_engine::Session;
use fp_mcp::docjson::{self, DocJsonOptions};
use fp_mcp::render::{self as fprender, RenderRequest};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod sprache;

const ABOUT: &str = "Firepixel – Kommandozeile (FW-PNG bearbeiten, rendern, Skripte, MCP-Server für KI-Agenten)";

const AFTER_HELP: &str = "Beispiele:
  firepixel-cli commands --filter align
  firepixel-cli info entwurf.fw.png --json
  firepixel-cli run entwurf.fw.png --cmd select.all --cmd modify.align.left --out ausgerichtet.fw.png
  firepixel-cli run --new 400x300 --cmd draw.rectangle --params '{\"rect\":[20,20,200,120],\"fillColor\":\"#3366ff\"}' --out neu.fw.png
  firepixel-cli render entwurf.fw.png --zoom 0.5 --out vorschau.png
  firepixel-cli batch --actions aktionen.json --in eingang --out ausgang
  firepixel-cli script makro.jsf entwurf.fw.png --out ergebnis.fw.png
  firepixel-cli mcp                      (MCP über stdio, z. B. für Claude Code)
  firepixel-cli mcp --http --port 47800  (MCP über HTTP auf 127.0.0.1 mit Token)
  firepixel-cli mcp --bridge             (stdio-MCP zur laufenden Oberfläche, firepixel.exe --mcp)

Befehlsnamen: <menü>.<eintrag> in camelCase (file.save, modify.transform.flipHorizontal, filter.blur.gaussian)
sowie draw.*, element.*, layer.*, page.*, state.*, select.*. Parameter: firepixel-cli commands --filter <name> --json.";

#[derive(Parser, Debug)]
#[command(name = "firepixel-cli", version, about = ABOUT, after_help = AFTER_HELP)]
struct Cli {
    /// Ausgabe als JSON (maschinenlesbar)
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Befehle auf eine Datei anwenden und speichern
    #[command(after_help = "Beispiele:
  firepixel-cli run in.fw.png --cmd select.all --cmd modify.group --out out.fw.png
  firepixel-cli run in.fw.png --cmd filter.blur.gaussian --params '{\"radius\":2}' --out out.fw.png
  firepixel-cli run --new 200x100 --cmd draw.text --params '{\"text\":\"Hallo\",\"anchor\":[10,60],\"size\":32}' --out text.png
  firepixel-cli run in.fw.png --steps schritte.json --out out.fw.png

--params gehört zum vorangehenden --cmd. --out mit Endung .png (ohne .fw) schreibt ein flaches PNG,
sonst eine FW-PNG. --steps nimmt eine JSON-Liste [{\"cmd\": …, \"params\": {…}}, …] (Datei oder Text).")]
    Run {
        /// Eingabedatei (.fw.png oder .png); entfällt mit --new
        file: Option<PathBuf>,
        /// Neues Dokument BREITExHÖHE[:#farbe] statt einer Datei, z. B. 800x600 oder 64x64:#00000000
        #[arg(long, value_name = "WxH")]
        new: Option<String>,
        /// Befehlsname (mehrfach möglich)
        #[arg(long = "cmd", value_name = "NAME")]
        cmds: Vec<String>,
        /// JSON-Parameter für das vorangehende --cmd
        #[arg(long = "params", value_name = "JSON")]
        params: Vec<String>,
        /// Schrittliste als JSON-Datei oder JSON-Text
        #[arg(long, value_name = "JSON|DATEI")]
        steps: Option<String>,
        /// Alle Befehle als ein Rückgängig-Schritt
        #[arg(long)]
        group: bool,
        /// Zieldatei (Vorgabe: Eingabedatei überschreiben nur mit --in-place)
        #[arg(long, value_name = "DATEI")]
        out: Option<PathBuf>,
        /// Eingabedatei überschreiben
        #[arg(long)]
        in_place: bool,
    },
    /// Dieselben Aktionen auf alle Dateien eines Ordners anwenden
    #[command(after_help = "Beispiel aktionen.json:
  [ {\"cmd\": \"select.all\"},
    {\"cmd\": \"modify.canvas.imageSize\", \"params\": {\"percent\": 50}},
    {\"cmd\": \"filter.sharpen.sharpen\"} ]

  firepixel-cli batch --actions aktionen.json --in eingang --out ausgang
  firepixel-cli batch --actions '[{\"cmd\":\"modify.transform.flipHorizontal\"}]' --in a --out b --format png")]
    Batch {
        /// Aktionen als JSON-Datei oder JSON-Text
        #[arg(long, value_name = "JSON|DATEI")]
        actions: String,
        /// Eingangsordner
        #[arg(long = "in", value_name = "ORDNER")]
        input: PathBuf,
        /// Ausgangsordner (wird angelegt)
        #[arg(long = "out", value_name = "ORDNER")]
        output: PathBuf,
        /// Ausgabeformat: fwpng (Vorgabe, editierbar), png, gif, jpg, webp, avif, bmp, tiff, qoi, ico, apng
        #[arg(long, default_value = "fwpng")]
        format: String,
        /// Nur Dateien, deren Name diesen Text enthält
        #[arg(long)]
        filter: Option<String>,
    },
    /// Seite, Ausschnitt oder Element als PNG rendern (offscreen)
    #[command(after_help = "Beispiele:
  firepixel-cli render datei.fw.png --out seite.png
  firepixel-cli render datei.fw.png --page 1 --state 2 --zoom 2 --out s2.png
  firepixel-cli render datei.fw.png --rect 0,0,100,100 --zoom 4 --out ausschnitt.png
  firepixel-cli render datei.fw.png --element 17 --out element.png")]
    Render {
        file: PathBuf,
        #[arg(long)]
        page: Option<usize>,
        #[arg(long)]
        state: Option<usize>,
        #[arg(long, default_value_t = 1.0)]
        zoom: f64,
        /// Ausschnitt l,t,r,b in Dokumentpixeln
        #[arg(long, value_name = "L,T,R,B")]
        rect: Option<String>,
        /// Nur dieses Element (ID aus `info`)
        #[arg(long)]
        element: Option<u64>,
        #[arg(long, value_name = "DATEI")]
        out: PathBuf,
    },
    /// Dokumentbaum anzeigen (Seiten, Status, Ebenen, Elemente mit IDs)
    #[command(after_help = "Beispiele:
  firepixel-cli info datei.fw.png
  firepixel-cli info datei.fw.png --json --depth 1
  firepixel-cli info datei.fw.png --json --all-pages --types text")]
    Info {
        file: PathBuf,
        #[arg(long, default_value_t = 8)]
        depth: usize,
        #[arg(long)]
        all_pages: bool,
        #[arg(long)]
        page: Option<usize>,
        /// Nur diese Elementtypen (kommagetrennt: text,path,rectangle,bitmap,group,instance …)
        #[arg(long, value_delimiter = ',')]
        types: Vec<String>,
        /// customData ausgeben
        #[arg(long)]
        custom_data: bool,
    },
    /// Befehlskatalog anzeigen
    #[command(after_help = "Beispiele:
  firepixel-cli commands
  firepixel-cli commands --category modify.align
  firepixel-cli commands --filter weich --json")]
    Commands {
        /// Suchtext in Name, Beschriftung, Statustext, Menü
        #[arg(long)]
        filter: Option<String>,
        /// Namenspräfix (draw, modify, filter …)
        #[arg(long)]
        category: Option<String>,
        /// Auch Platzhalter ohne Wirkung
        #[arg(long)]
        all: bool,
    },
    /// Datei umwandeln (Format aus der Endung)
    #[command(after_help = "Beispiele:
  firepixel-cli convert entwurf.fw.png entwurf.png     (flaches PNG32)
  firepixel-cli convert entwurf.fw.png entwurf.jpg     (JPEG über die Optimierung wie im Original)
  firepixel-cli convert ebenen.psd ebenen.fw.png       (PSD mit Ebenen als FW-PNG)
  firepixel-cli convert foto.png foto.webp
Lesen: .fw.png .png .psd .gif .jpg .bmp .tif .tga .webp .jxl .qoi .ico .svg .ai .eps
Schreiben: .fw.png .png .gif .jpg .webp .avif .bmp .tif .qoi .ico .apng")]
    Convert { input: PathBuf, output: PathBuf },
    /// JSF-JavaScript (.jsf) ausführen
    #[command(after_help = "Beispiele:
  firepixel-cli script makro.jsf
  firepixel-cli script makro.jsf entwurf.fw.png --out ergebnis.fw.png
  firepixel-cli script --code \"fw.createFirepixelDocument({x:100,y:100},{pixelsPerUnit:72,units:'inch'},'#ffffff').width\"")]
    Script {
        /// Skriptdatei
        script: Option<PathBuf>,
        /// Dokument, das vorher geöffnet wird
        file: Option<PathBuf>,
        /// Skript als Text statt Datei
        #[arg(long)]
        code: Option<String>,
        /// Aktives Dokument danach speichern
        #[arg(long, value_name = "DATEI")]
        out: Option<PathBuf>,
        /// Wert der globalen Variable FP_ARG
        #[arg(long)]
        arg: Option<String>,
    },
    /// MCP-Server für KI-Agenten (stdio oder HTTP auf 127.0.0.1 mit Token)
    #[command(after_help = "Beispiele – Einrichtung in Claude Code (stdio):
  claude mcp add firepixel -- firepixel-cli mcp
HTTP (Token steht danach in %LOCALAPPDATA%\\Firepixel\\mcp-http.json):
  firepixel-cli mcp --http --port 47800
  claude mcp add --transport http firepixel http://127.0.0.1:47800/mcp --header \"Authorization: Bearer <token>\"
Brücke zur laufenden Oberfläche (firepixel.exe --mcp bzw. FIREPIXEL_MCP=1; Token in %LOCALAPPDATA%\\Firepixel\\mcp-bridge.json):
  claude mcp add firepixel-gui -- firepixel-cli mcp --bridge
Doku: docs/AGENTS.md bzw. MCP-Ressource firepixel://docs/agents.md")]
    Mcp {
        /// Streamable HTTP statt stdio
        #[arg(long, conflicts_with = "bridge")]
        http: bool,
        /// stdio-Weiterleitung zur laufenden Firepixel-Oberfläche (Brückenmodus)
        #[arg(long)]
        bridge: bool,
        /// Port auf 127.0.0.1 (0 = frei wählen)
        #[arg(long, default_value_t = fp_mcp::DEFAULT_PORT)]
        port: u16,
        /// Festes Token (mindestens 16 Zeichen; Vorgabe: zufällig)
        #[arg(long)]
        token: Option<String>,
        /// Datei für Adresse und Token (Vorgabe im Benutzerprofil)
        #[arg(long)]
        token_file: Option<PathBuf>,
    },
    /// Sprachdateien prüfen, Vorlage für eine neue Sprache anlegen, Sprachen auflisten
    #[command(subcommand)]
    Sprache(sprache::SpracheCmd),
}

/// Fehler mit Meldung.
struct Fail(String);

impl<E: std::fmt::Display> From<E> for Fail {
    fn from(e: E) -> Self {
        Fail(e.to_string())
    }
}

type Res<T> = Result<T, Fail>;

fn main() -> ExitCode {
    let matches = Cli::command().get_matches();
    let cli = match Cli::from_arg_matches(&matches) {
        Ok(c) => c,
        Err(e) => {
            let _ = e.print();
            return ExitCode::from(2);
        }
    };
    let json_out = cli.json;
    let sub = matches.subcommand().map(|(_, m)| m.clone());
    let r = run(cli, sub.as_ref());
    match r {
        Ok(v) => {
            if json_out {
                println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            } else if let Some(t) = v.get("text").and_then(|t| t.as_str()) {
                println!("{t}");
            } else if !v.is_null() {
                println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            }
            ExitCode::SUCCESS
        }
        Err(Fail(m)) => {
            if json_out {
                println!("{}", json!({"ok": false, "error": m}));
            } else {
                eprintln!("Fehler: {m}");
            }
            ExitCode::from(1)
        }
    }
}

fn run(cli: Cli, sub: Option<&ArgMatches>) -> Res<Value> {
    let json_out = cli.json;
    match cli.cmd {
        Cmd::Run { file, new, cmds, params, steps, group, out, in_place } => cmd_run(sub, file, new, cmds, params, steps, group, out, in_place),
        Cmd::Batch { actions, input, output, format, filter } => cmd_batch(&actions, &input, &output, &format, filter.as_deref()),
        Cmd::Render { file, page, state, zoom, rect, element, out } => cmd_render(&file, page, state, zoom, rect.as_deref(), element, &out),
        Cmd::Info { file, depth, all_pages, page, types, custom_data } => cmd_info(&file, depth, all_pages, page, types, custom_data, json_out),
        Cmd::Commands { filter, category, all } => cmd_commands(filter.as_deref(), category.as_deref(), all, json_out),
        Cmd::Convert { input, output } => cmd_convert(&input, &output),
        Cmd::Script { script, file, code, out, arg } => cmd_script(script, file, code, out, arg),
        Cmd::Mcp { http, bridge, port, token, token_file } => {
            if bridge {
                cmd_mcp_bridge(token_file)
            } else {
                cmd_mcp(http, port, token, token_file)
            }
        }
        Cmd::Sprache(c) => sprache::run(c, json_out).map_err(Fail),
    }
}

// ------------------------------------------------------------------ Hilfen

/// JSON-Text oder Datei mit JSON (`@datei` oder existierender Pfad).
fn json_arg(s: &str) -> Res<Value> {
    let t = s.trim();
    let text = if let Some(p) = t.strip_prefix('@') {
        std::fs::read_to_string(p).map_err(|e| Fail(format!("{p}: {e}")))?
    } else if !(t.starts_with('{') || t.starts_with('[')) && Path::new(t).is_file() {
        std::fs::read_to_string(t).map_err(|e| Fail(format!("{t}: {e}")))?
    } else {
        t.to_string()
    };
    serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| Fail(format!("ungültiges JSON ({e}): {}", text.chars().take(80).collect::<String>())))
}

fn open(s: &mut Session, path: &Path) -> Res<()> {
    fp_script::fileio::open_document(s, path).map_err(|e| Fail(format!("{}: {e}", path.display())))?;
    Ok(())
}

/// `WxH[:#farbe]` → neues Dokument.
fn new_doc(s: &mut Session, spec: &str) -> Res<()> {
    let (size, color) = match spec.split_once(':') {
        Some((a, b)) => (a, Some(b)),
        None => (spec, None),
    };
    let (w, h) = size.split_once(['x', 'X']).ok_or_else(|| Fail(format!("--new erwartet BREITExHÖHE, nicht „{spec}“")))?;
    let w: f64 = w.trim().parse().map_err(|_| Fail(format!("Breite „{w}“")))?;
    let h: f64 = h.trim().parse().map_err(|_| Fail(format!("Höhe „{h}“")))?;
    let mut p = json!({"width": w, "height": h});
    if let Some(c) = color {
        p["background"] = json!(c);
    }
    s.execute("file.new", p)?;
    Ok(())
}

fn is_flat_png(p: &Path) -> bool {
    let l = p.to_string_lossy().to_lowercase();
    l.ends_with(".png") && !l.ends_with(".fw.png")
}

/// Speichert das aktive Dokument: `.fw.png` editierbar, `.png` flach (PNG32), andere Endungen
/// (gif, jpg, webp, avif, bmp, tiff, qoi, ico, apng) über fp-export.
fn write_output(s: &mut Session, out: &Path) -> Res<Value> {
    let l = out.to_string_lossy().to_lowercase();
    if !l.ends_with(".png") {
        let id = s.doc()?.id;
        let n = fp_script::fileio::export_page(s, id, None, None, out, &Value::Null)
            .map_err(|e| Fail(format!("{e} (Formate: .fw.png, {})", fp_script::fileio::export_extensions().iter().map(|x| format!(".{x}")).collect::<Vec<_>>().join(", "))))?;
        return Ok(json!({"path": out.to_string_lossy(), "format": out.extension().map(|e| e.to_string_lossy().to_string()), "bytes": n}));
    }
    if is_flat_png(out) {
        let d = s.doc()?;
        let r = fprender::render(&fp_render::Renderer::default(), d, &RenderRequest { max_pixels: 400_000_000, ..RenderRequest::default() })?;
        if let Some(dir) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(out, &r.png)?;
        return Ok(json!({"path": out.to_string_lossy(), "format": "png", "width": r.width, "height": r.height}));
    }
    s.execute("file.saveAs", json!({"path": out.to_string_lossy()}))?;
    Ok(json!({"path": out.to_string_lossy(), "format": "fwpng"}))
}

fn state_json(s: &Session) -> Value {
    match s.active() {
        Some(d) => json!({"doc": d.id.0, "undo": d.history.undo_count(), "selection": d.selection.elements.iter().map(|e| e.0).collect::<Vec<_>>()}),
        None => Value::Null,
    }
}

// ------------------------------------------------------------------ Unterbefehle

#[allow(clippy::too_many_arguments)]
fn cmd_run(
    sub: Option<&ArgMatches>,
    file: Option<PathBuf>,
    new: Option<String>,
    cmds: Vec<String>,
    params: Vec<String>,
    steps: Option<String>,
    group: bool,
    out: Option<PathBuf>,
    in_place: bool,
) -> Res<Value> {
    let mut s = new_session();
    match (&file, &new) {
        (Some(f), _) => open(&mut s, f)?,
        (None, Some(spec)) => new_doc(&mut s, spec)?,
        (None, None) => return Err(Fail("Eingabedatei oder --new angeben".into())),
    }
    // --params dem vorangehenden --cmd zuordnen (über die Positionen in der Kommandozeile)
    let mut list: Vec<(String, Value)> = cmds.iter().map(|c| (c.clone(), Value::Null)).collect();
    if !params.is_empty() {
        let ci: Vec<usize> = sub.and_then(|m| m.indices_of("cmds")).map(|i| i.collect()).unwrap_or_default();
        let pi: Vec<usize> = sub.and_then(|m| m.indices_of("params")).map(|i| i.collect()).unwrap_or_default();
        for (k, p) in params.iter().enumerate() {
            let pos = pi.get(k).copied().unwrap_or(usize::MAX);
            let owner = ci.iter().rposition(|c| *c < pos).or(if ci.is_empty() { None } else { Some(k.min(ci.len() - 1)) });
            let Some(o) = owner else { return Err(Fail("--params ohne vorangehendes --cmd".into())) };
            list[o].1 = json_arg(p)?;
        }
    }
    if let Some(st) = &steps {
        let v = json_arg(st)?;
        let arr = v.get("steps").cloned().unwrap_or(v);
        for item in arr.as_array().ok_or_else(|| Fail("--steps: JSON-Liste erwartet".into()))? {
            let name = item.get("cmd").or_else(|| item.get("name")).or_else(|| item.get("command")).and_then(|n| n.as_str()).ok_or_else(|| Fail("Schritt ohne \"cmd\"".into()))?;
            list.push((name.to_string(), item.get("params").cloned().unwrap_or(Value::Null)));
        }
    }
    let mut results = Vec::new();
    let mut run_all = |s: &mut Session| -> Result<(), fp_engine::EngineError> {
        for (name, p) in &list {
            let o = s.execute(name, p.clone()).map_err(|e| fp_engine::EngineError::Failed(format!("{name}: {e}")))?;
            results.push(json!({"cmd": name, "result": o.value}));
        }
        Ok(())
    };
    if group { s.run_group("CLI", &mut run_all)? } else { run_all(&mut s)? }
    let saved = match (out, in_place, &file) {
        (Some(o), _, _) => Some(write_output(&mut s, &o)?),
        (None, true, Some(f)) => Some(write_output(&mut s, f)?),
        _ => None,
    };
    let n = results.len();
    Ok(json!({"ok": true, "commands": results, "saved": saved, "state": state_json(&s),
              "text": format!("{n} Befehl(e) ausgeführt{}", saved.as_ref().and_then(|v| v.get("path")).and_then(|p| p.as_str()).map(|p| format!(", gespeichert: {p}")).unwrap_or_default())}))
}

fn cmd_batch(actions: &str, input: &Path, output: &Path, format: &str, filter: Option<&str>) -> Res<Value> {
    let v = json_arg(actions)?;
    let arr = v.get("actions").cloned().unwrap_or(v);
    let list: Vec<(String, Value)> = arr
        .as_array()
        .ok_or_else(|| Fail("--actions: JSON-Liste erwartet".into()))?
        .iter()
        .map(|a| {
            let n = a.get("cmd").or_else(|| a.get("name")).or_else(|| a.get("command")).and_then(|n| n.as_str()).unwrap_or("").to_string();
            (n, a.get("params").cloned().unwrap_or(Value::Null))
        })
        .collect();
    if list.iter().any(|(n, _)| n.is_empty()) {
        return Err(Fail("jede Aktion braucht \"cmd\"".into()));
    }
    let out_ext = match format.trim_start_matches('.').to_lowercase().as_str() {
        "fwpng" | "fw.png" => "fw.png".to_string(),
        e if fp_script::fileio::export_extensions().contains(&e) => e.to_string(),
        other => return Err(Fail(format!("--format: fwpng oder {}, nicht „{other}“", fp_script::fileio::export_extensions().join(", ")))),
    };
    std::fs::create_dir_all(output)?;
    let mut files: Vec<PathBuf> = std::fs::read_dir(input)
        .map_err(|e| Fail(format!("{}: {e}", input.display())))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            let l = p.to_string_lossy().to_lowercase();
            p.is_file() && fp_script::fileio::import_extensions().iter().any(|e| l.ends_with(&format!(".{e}")))
        })
        .filter(|p| filter.is_none_or(|f| p.file_name().is_some_and(|n| n.to_string_lossy().contains(f))))
        .collect();
    files.sort();
    let mut report = Vec::new();
    let mut failed = 0;
    for f in &files {
        let name = f.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let lower = name.to_lowercase();
        let base = if lower.ends_with(".fw.png") { name[..name.len() - 7].to_string() } else { f.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default() };
        let target = output.join(format!("{base}.{out_ext}"));
        let mut s = new_session();
        let r: Res<()> = (|| {
            open(&mut s, f)?;
            for (n, p) in &list {
                s.execute(n, p.clone()).map_err(|e| Fail(format!("{n}: {e}")))?;
            }
            write_output(&mut s, &target)?;
            Ok(())
        })();
        match r {
            Ok(()) => report.push(json!({"file": name, "ok": true, "out": target.to_string_lossy()})),
            Err(Fail(m)) => {
                failed += 1;
                report.push(json!({"file": name, "ok": false, "error": m}));
            }
        }
    }
    let ok = files.len() - failed;
    Ok(json!({"ok": failed == 0, "processed": files.len(), "failed": failed, "files": report, "text": format!("{ok} von {} Dateien verarbeitet → {}", files.len(), output.display())}))
}

fn parse_rect(s: &str) -> Res<fp_geom::Rect> {
    let v: Vec<f32> = s.split(',').map(|x| x.trim().parse::<f32>()).collect::<Result<_, _>>().map_err(|_| Fail(format!("--rect l,t,r,b erwartet, nicht „{s}“")))?;
    if v.len() != 4 {
        return Err(Fail("--rect braucht vier Zahlen".into()));
    }
    Ok(fp_geom::Rect::new(v[0], v[1], v[2], v[3]))
}

fn cmd_render(file: &Path, page: Option<usize>, state: Option<usize>, zoom: f64, rect: Option<&str>, element: Option<u64>, out: &Path) -> Res<Value> {
    let mut s = new_session();
    open(&mut s, file)?;
    let req = RenderRequest { page, state, zoom, rect: rect.map(parse_rect).transpose()?, element, max_pixels: 400_000_000 };
    let d = s.doc()?;
    let r = fprender::render(&fp_render::Renderer::default(), d, &req)?;
    if let Some(dir) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(out, &r.png)?;
    let mut m = r.meta();
    m["ok"] = json!(true);
    m["path"] = json!(out.to_string_lossy());
    m["text"] = json!(format!("{}×{} px → {}", r.width, r.height, out.display()));
    Ok(m)
}

fn tree_text(v: &Value) -> String {
    let mut out = String::new();
    out += &format!("{} – {} Seite(n)\n", v["title"].as_str().unwrap_or(""), v["pages"].as_array().map_or(0, |a| a.len()));
    fn el(out: &mut String, e: &Value, ind: usize) {
        let b = &e["bounds"];
        out.push_str(&format!(
            "{:ind$}#{} {} {}{} [{}, {}, {}, {}]\n",
            "",
            e["id"],
            e["type"].as_str().unwrap_or(""),
            e["name"].as_str().map(|n| format!("„{n}“ ")).unwrap_or_default(),
            e["text"].as_str().map(|t| format!("\"{}\" ", t.chars().take(30).collect::<String>())).unwrap_or_default(),
            b["left"],
            b["top"],
            b["right"],
            b["bottom"],
            ind = ind
        ));
        for c in e["children"].as_array().into_iter().flatten() {
            el(out, c, ind + 2);
        }
    }
    for p in v["pages"].as_array().into_iter().flatten() {
        out += &format!("Seite {} „{}“ {}×{} px, {} Status, Leinwand {}\n", p["index"], p["name"].as_str().unwrap_or(""), p["width"], p["height"], p["states"], p["background"].as_str().unwrap_or(""));
        for l in p["layers"].as_array().into_iter().flatten().rev() {
            out += &format!("  Ebene #{} „{}“ ({}{})\n", l["id"], l["name"].as_str().unwrap_or(""), l["kind"].as_str().unwrap_or(""), if l["visible"] == json!(false) { ", ausgeblendet" } else { "" });
            for e in l["elements"].as_array().into_iter().flatten() {
                el(&mut out, e, 4);
            }
        }
    }
    out
}

fn cmd_info(file: &Path, depth: usize, all_pages: bool, page: Option<usize>, types: Vec<String>, custom_data: bool, json_out: bool) -> Res<Value> {
    let mut s = new_session();
    open(&mut s, file)?;
    let d = s.doc()?;
    let opts = DocJsonOptions { page, all_pages, depth, types, custom_data, max_elements: 100_000, ..DocJsonOptions::default() };
    let mut v = docjson::document_json(d, &opts);
    if !json_out {
        let t = tree_text(&v);
        v = json!({"text": t.trim_end()});
    }
    Ok(v)
}

fn cmd_commands(filter: Option<&str>, category: Option<&str>, all: bool, json_out: bool) -> Res<Value> {
    let s = new_session();
    let f = filter.map(|x| x.to_lowercase());
    let c = category.map(|x| x.to_lowercase());
    let mut list = Vec::new();
    let mut lines = Vec::new();
    for cmd in s.registry().iter() {
        let i = cmd.info();
        if !all && !i.implemented {
            continue;
        }
        if let Some(c) = &c {
            let n = i.name.to_lowercase();
            if !(n == *c || n.starts_with(&format!("{c}."))) {
                continue;
            }
        }
        if let Some(f) = &f {
            let hay = format!("{} {} {} {}", i.name, i.label, i.description.clone().unwrap_or_default(), i.menu_path.join(" ")).to_lowercase();
            if !hay.contains(f.as_str()) {
                continue;
            }
        }
        let ps: Vec<String> = i.params.iter().map(|p| if p.required { format!("{}*", p.name) } else { p.name.clone() }).collect();
        lines.push(format!("{:<44} {}{}", i.name, i.label, if ps.is_empty() { String::new() } else { format!("  ({})", ps.join(", ")) }));
        list.push(json!({"name": i.name, "label": i.label, "menu": i.menu_path, "implemented": i.implemented, "params": i.params_schema(), "requires": i.requires, "description": i.description}));
    }
    if json_out {
        Ok(json!({"count": list.len(), "commands": list}))
    } else {
        Ok(json!({"text": format!("{}\n\n{} Befehle (* = Pflichtparameter)", lines.join("\n"), lines.len())}))
    }
}

fn cmd_convert(input: &Path, output: &Path) -> Res<Value> {
    let mut s = new_session();
    open(&mut s, input)?;
    let mut v = write_output(&mut s, output)?;
    v["ok"] = json!(true);
    v["text"] = json!(format!("{} → {}", input.display(), output.display()));
    Ok(v)
}

fn cmd_script(script: Option<PathBuf>, file: Option<PathBuf>, code: Option<String>, out: Option<PathBuf>, arg: Option<String>) -> Res<Value> {
    let mut s = new_session();
    if let Some(f) = &file {
        open(&mut s, f)?;
    }
    let mut opts = fp_script::ScriptOptions::default();
    if let Some(a) = arg {
        opts.globals.push(("FP_ARG".into(), json!(a)));
    }
    let outcome = match (&code, &script) {
        (Some(c), _) => fp_script::run_source(&mut s, c, &opts),
        (None, Some(p)) => fp_script::run_file(&mut s, p, &opts),
        (None, None) => return Err(Fail("Skriptdatei oder --code angeben".into())),
    };
    let saved = match &out {
        Some(o) if s.active().is_some() => Some(write_output(&mut s, o)?),
        Some(_) => return Err(Fail("kein Dokument zum Speichern offen".into())),
        None => None,
    };
    let mut v = outcome.to_json();
    v["saved"] = json!(saved);
    let mut text = String::new();
    for a in &outcome.alerts {
        text += &format!("alert: {a}\n");
    }
    for l in &outcome.log {
        text += &format!("{l}\n");
    }
    if let Some(r) = &outcome.result {
        text += &format!("Ergebnis: {r}\n");
    }
    if !outcome.not_implemented.is_empty() {
        text += &format!("nicht implementiert: {}\n", outcome.not_implemented.join(", "));
    }
    if let Some(e) = &outcome.error {
        return Err(Fail(format!("{}Skriptfehler: {e}", text)));
    }
    v["text"] = json!(text.trim_end());
    Ok(v)
}

fn cmd_mcp(http: bool, port: u16, token: Option<String>, token_file: Option<PathBuf>) -> Res<Value> {
    let host: std::sync::Arc<dyn fp_mcp::SessionHost> = std::sync::Arc::new(fp_mcp::LocalHost::with_default_session());
    if !http {
        fp_mcp::run_stdio_blocking(host)?;
        return Ok(Value::Null);
    }
    let rt = tokio_runtime()?;
    rt.block_on(async move {
        let opts = fp_mcp::HttpOptions { port, token, token_file, write_token_file: true };
        let srv = fp_mcp::start_http(host, opts).await.map_err(|e| Fail(e.to_string()))?;
        eprintln!("Firepixel-MCP (HTTP) läuft auf {}", srv.url());
        eprintln!("Token: {}", srv.token());
        eprintln!("Header: Authorization: Bearer {}", srv.token());
        eprintln!("Token-Datei: {}", srv.token_file().display());
        eprintln!("Beenden mit Strg+C.");
        srv.wait().await;
        Ok::<(), Fail>(())
    })?;
    Ok(Value::Null)
}

/// stdio-MCP, weitergeleitet an die laufende Oberfläche (Adresse/Token aus der Token-Datei).
fn cmd_mcp_bridge(token_file: Option<PathBuf>) -> Res<Value> {
    let file = token_file.unwrap_or_else(fp_mcp::bridge_token_file);
    let target = fp_mcp::proxy::BridgeTarget::from_token_file(&file)?;
    eprintln!("Firepixel-MCP: Weiterleitung an die laufende Oberfläche (127.0.0.1:{}{})", target.port, target.path);
    fp_mcp::proxy::run_stdio_proxy(target)?;
    Ok(Value::Null)
}

/// Sitzung mit eingesteckter JSF-Laufzeit (commands.runScript/commands.run).
fn new_session() -> Session {
    let mut s = Session::default();
    fp_script::install_host(&mut s);
    s
}

fn tokio_runtime() -> Res<tokio::runtime::Runtime> {
    Ok(tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build()?)
}
