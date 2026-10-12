//! CLI-Tests je Unterbefehl-Gruppe (assert_cmd).

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::Stdio;

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fp_cli_test_{}_{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let _ = std::fs::create_dir_all(&d);
    d
}

fn cli() -> Command {
    let mut c = Command::cargo_bin("firepixel-cli").unwrap_or_else(|e| panic!("{e}"));
    // Ausgaben auf Deutsch prüfen (sonst Systemsprache)
    c.env("FIREPIXEL_LANG", "de");
    c.env(fp_i18n::profile::ENV_TEST_PROFILE, fp_i18n::profile::use_test_profile()).env("FP_NO_SHELL_OPEN", "1"); // nie ins echte Nutzerprofil (EB-3)
    c
}

fn json_of(out: &[u8]) -> Value {
    serde_json::from_slice(out).unwrap_or_else(|e| panic!("kein JSON ({e}): {}", String::from_utf8_lossy(out)))
}

/// Testdatei: 100×80, rotes Rechteck, Text.
fn fixture(dir: &std::path::Path) -> PathBuf {
    let f = dir.join("probe.fw.png");
    cli()
        .args(["run", "--new", "100x80", "--cmd", "draw.rectangle", "--params", r##"{"rect":[10,10,60,50],"fillColor":"#ff0000","strokeWidth":0,"name":"Kasten"}"##])
        .arg("--out")
        .arg(&f)
        .assert()
        .success();
    assert!(f.is_file());
    f
}

#[test]
fn help_has_examples() {
    cli().arg("--help").assert().success().stdout(predicate::str::contains("Beispiele"));
    for sub in ["run", "batch", "render", "info", "commands", "convert", "script", "mcp"] {
        cli().args([sub, "--help"]).assert().success().stdout(predicate::str::contains("Beispiel"));
    }
}

#[test]
fn commands_lists_catalog() {
    let out = cli().args(["commands", "--category", "modify.align", "--json"]).assert().success().get_output().stdout.clone();
    let v = json_of(&out);
    assert!(v["count"].as_u64().unwrap_or(0) >= 6);
    assert!(v["commands"].as_array().is_some_and(|a| a.iter().any(|c| c["name"] == "modify.align.left")));
    cli().args(["commands", "--filter", "gaussian"]).assert().success().stdout(predicate::str::contains("filter.blur.gaussian"));
}

#[test]
fn run_info_render_convert() {
    let d = tmp("run");
    let f = fixture(&d);
    // info als JSON
    let out = cli().arg("info").arg(&f).arg("--json").assert().success().get_output().stdout.clone();
    let v = json_of(&out);
    let els = v["pages"][0]["layers"].as_array().map(|ls| ls.iter().flat_map(|l| l["elements"].as_array().cloned().unwrap_or_default()).collect::<Vec<_>>()).unwrap_or_default();
    assert!(els.iter().any(|e| e["type"] == "rectangle" && e["name"] == "Kasten"), "{v}");
    // info als Text
    cli().arg("info").arg(&f).assert().success().stdout(predicate::str::contains("Kasten"));
    // run: mehrere Befehle mit Parametern
    let g = d.join("gruppe.fw.png");
    let out = cli()
        .arg("run")
        .arg(&f)
        .args(["--cmd", "select.all", "--cmd", "element.setOpacity", "--params", r#"{"opacity":50}"#, "--cmd", "modify.transform.flipHorizontal", "--json", "--out"])
        .arg(&g)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v = json_of(&out);
    assert_eq!(v["ok"], true);
    assert_eq!(v["commands"].as_array().map(|a| a.len()), Some(3));
    assert!(g.is_file());
    // render mit Zoom
    let png = d.join("r.png");
    let out = cli().arg("render").arg(&g).args(["--zoom", "2", "--json", "--out"]).arg(&png).assert().success().get_output().stdout.clone();
    let v = json_of(&out);
    assert_eq!(v["width"], 200);
    assert_eq!(v["height"], 160);
    let bytes = std::fs::read(&png).unwrap_or_default();
    assert_eq!(&bytes[..4], &[137, 80, 78, 71]);
    // Ausschnitt
    cli().arg("render").arg(&g).args(["--rect", "0,0,20,20", "--out"]).arg(d.join("a.png")).assert().success();
    // convert fw.png → png
    let flat = d.join("flach.png");
    cli().arg("convert").arg(&f).arg(&flat).assert().success();
    assert!(std::fs::read(&flat).is_ok_and(|b| b.len() > 8));
    // Fehler: unbekannter Befehl → Exit 1 und JSON-Fehler
    let out = cli().arg("run").arg(&f).args(["--cmd", "gibt.esNicht", "--json"]).assert().code(1).get_output().stdout.clone();
    assert_eq!(json_of(&out)["ok"], false);
}

#[test]
fn batch_processes_folder() {
    let d = tmp("batch");
    let inp = d.join("in");
    let outp = d.join("out");
    let _ = std::fs::create_dir_all(&inp);
    for n in ["a", "b"] {
        cli().args(["run", "--new", "40x30", "--cmd", "draw.ellipse", "--params", r#"{"rect":[5,5,35,25]}"#, "--out"]).arg(inp.join(format!("{n}.fw.png"))).assert().success();
    }
    let out = cli()
        .args(["batch", "--json", "--actions", r#"[{"cmd":"select.all"},{"cmd":"modify.transform.rotate90DegreeCw"}]"#, "--format", "png", "--in"])
        .arg(&inp)
        .arg("--out")
        .arg(&outp)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v = json_of(&out);
    assert_eq!(v["processed"], 2);
    assert_eq!(v["failed"], 0);
    assert!(outp.join("a.png").is_file() && outp.join("b.png").is_file());
}

#[test]
fn script_runs_jsf() {
    let d = tmp("script");
    let js = d.join("t.jsf");
    let _ = std::fs::write(
        &js,
        "var d = fw.createFirepixelDocument({x:50, y:40}, {pixelsPerUnit:72, units:\"inch\"}, \"#ffffff\");\n\
         d = fw.getDocumentDOM(); d.addNewOval({left:5, top:5, right:45, bottom:35}); alert(fw.selection[0]); d.width;",
    );
    let out_f = d.join("s.fw.png");
    let out = cli().arg("script").arg(&js).arg("--json").arg("--out").arg(&out_f).assert().success().get_output().stdout.clone();
    let v = json_of(&out);
    assert_eq!(v["ok"], true, "{v}");
    assert_eq!(v["result"], "50");
    assert_eq!(v["alerts"][0], "[object Path]");
    assert!(out_f.is_file());
    cli().args(["script", "--code", "throw -2147216337;"]).assert().code(1).stderr(predicate::str::contains("-2147216337"));
}

#[test]
fn mcp_http_and_bridge_proxy() {
    let d = tmp("bridge");
    let tok = d.join("token.json");
    // fehlende Token-Datei: verständlicher Fehler, Exit 1
    cli().args(["mcp", "--bridge", "--token-file"]).arg(d.join("fehlt.json")).assert().code(1).stderr(predicate::str::contains("--mcp"));
    let exe = assert_cmd::cargo::cargo_bin("firepixel-cli");
    let mut server = fp_i18n::profile::child_env(&mut std::process::Command::new(&exe))
        .args(["mcp", "--http", "--port", "0", "--token-file"])
        .arg(&tok)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|e| panic!("{e}"));
    for _ in 0..100 {
        if tok.is_file() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    assert!(tok.is_file(), "Token-Datei fehlt");
    let input = [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"1"}}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"new_document","arguments":{"width":30,"height":20}}}"#,
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"run_command","arguments":{"name":"draw.rectangle","params":{"rect":[2,2,20,15]}}}}"#,
        r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"render","arguments":{}}}"#,
        r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"screenshot","arguments":{}}}"#,
    ]
    .join("\n");
    let out = cli().args(["mcp", "--bridge", "--token-file"]).arg(&tok).write_stdin(input).assert().success().get_output().stdout.clone();
    let _ = server.kill();
    let _ = server.wait();
    let text = String::from_utf8_lossy(&out).to_string();
    let msgs: Vec<Value> = text.lines().filter_map(|l| serde_json::from_str(l).ok()).collect();
    let by_id = |id: u64| msgs.iter().find(|m| m["id"] == id).cloned().unwrap_or(Value::Null);
    assert!(by_id(1)["result"]["serverInfo"]["name"] == "firepixel", "{text}");
    assert!(by_id(3)["result"]["structuredContent"]["result"]["id"].is_u64(), "{text}");
    assert_eq!(by_id(4)["result"]["content"][0]["type"], "image", "{text}");
    // headless-Server: screenshot meldet den Brückenmodus als Voraussetzung
    assert_eq!(by_id(5)["result"]["isError"], true, "{text}");
}

#[test]
fn run_steps_in_place_and_info_filters() {
    let d = tmp("steps");
    let f = fixture(&d);
    // --steps als ein Rückgängig-Schritt, Ergebnis in die Eingabedatei
    cli()
        .arg("run")
        .arg(&f)
        .args(["--steps", r#"[{"name":"select.all"},{"name":"element.setName","params":{"name":"Umbenannt"}}]"#, "--in-place"])
        .assert()
        .success();
    let out = cli().arg("info").arg(&f).args(["--json", "--types", "rectangle"]).assert().success().get_output().stdout.clone();
    let v = json_of(&out);
    let s = v.to_string();
    assert!(s.contains("Umbenannt") && !s.contains("Kasten"), "{s}");
    // Element einzeln rendern (ID aus info)
    let id = v["pages"][0]["layers"].as_array().and_then(|ls| ls.iter().find_map(|l| l["elements"].as_array().and_then(|e| e.first()).and_then(|e| e["id"].as_u64())));
    let id = id.unwrap_or_else(|| panic!("keine Element-ID: {s}"));
    let png = d.join("el.png");
    let out = cli().arg("render").arg(&f).args(["--element", &id.to_string(), "--json", "--out"]).arg(&png).assert().success().get_output().stdout.clone();
    let r = json_of(&out);
    assert_eq!(r["width"], 50, "{r}");
    assert_eq!(r["height"], 40, "{r}");
    // convert in ein Fremdformat
    cli().arg("convert").arg(&f).arg(d.join("x.gif")).assert().success();
    assert!(std::fs::read(d.join("x.gif")).is_ok_and(|b| b.starts_with(b"GIF8")));
}

#[test]
fn mcp_stdio_server_answers() {
    let exe = assert_cmd::cargo::cargo_bin("firepixel-cli");
    let mut child = fp_i18n::profile::child_env(&mut std::process::Command::new(exe)).arg("mcp").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().unwrap_or_else(|e| panic!("{e}"));
    let mut stdin = child.stdin.take().unwrap_or_else(|| panic!("stdin"));
    let stdout = child.stdout.take().unwrap_or_else(|| panic!("stdout"));
    let mut lines = BufReader::new(stdout).lines();
    let mut send = |s: &str| {
        let _ = stdin.write_all(s.as_bytes());
        let _ = stdin.write_all(b"\n");
        let _ = stdin.flush();
    };
    send(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"1"}}}"#);
    let first = lines.next().and_then(|l| l.ok()).unwrap_or_default();
    assert!(first.contains("firepixel"), "{first}");
    send(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
    send(r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"new_document","arguments":{"width":30,"height":20}}}"#);
    // Anfragen laufen nebenläufig: erst auf die Antwort warten, dann die abhängige Anfrage schicken
    for _ in 0..10 {
        let Some(Ok(l)) = lines.next() else { break };
        if l.contains("\"id\":2") {
            break;
        }
    }
    send(r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"render","arguments":{}}}"#);
    let mut got_image = false;
    for _ in 0..10 {
        let Some(Ok(l)) = lines.next() else { break };
        if l.contains("\"id\":3") {
            got_image = l.contains("\"type\":\"image\"") && l.contains("image/png");
            break;
        }
    }
    drop(send);
    let _ = child.kill();
    let _ = child.wait();
    assert!(got_image);
}

/// UE-3: Ausgaben, Fehler und Hilfe der Kommandozeile in Englisch (`--lang en` bzw. `FIREPIXEL_LANG`).
#[test]
fn ausgaben_englisch() {
    let out = cli().args(["--lang", "en", "commands", "--filter", "gaussian"]).assert().success().get_output().stdout.clone();
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("Gaussian Blur") || s.contains("filter.blur.gaussian"), "{s}");
    assert!(s.contains("(* = required parameter)"), "{s}");
    cli().env("FIREPIXEL_LANG", "en").args(["run"]).assert().code(1).stderr(predicate::str::contains("Error: give an input file or --new"));
    cli().args(["--lang", "en", "--gibtsnicht"]).assert().code(2).stderr(predicate::str::contains("unknown option").and(predicate::str::contains("Help:")));
    cli().args(["--lang", "de", "--gibtsnicht"]).assert().code(2).stderr(predicate::str::contains("unbekannte Option"));
    cli().args(["--lang", "en", "render", "--help"]).assert().success().stdout(predicate::str::contains("Usage:").and(predicate::str::contains("Examples:")));
    cli().args(["--lang", "en", "sprache", "liste"]).assert().success().stdout(predicate::str::contains("built-in").and(predicate::str::contains("Language folders:")));
}
