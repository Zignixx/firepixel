//! B3 (BA-040): Weichzeichnen/Gauß am Text wie im Original (`re\23-effekte.md` Gruppe 3,
//! Bericht `docs\berichte\fxt.md` Befunde 1 und 2). Ablauf wie im Original: Basisdatei öffnen,
//! `selectAll`, `dom.applyEffects`, `pixelRect` lesen. Ohne Testdateien (`re\_testdateien`) still
//! übersprungen.

use fp_script::{run_source, ScriptOptions};

fn base_text() -> Option<std::path::PathBuf> {
    let mut d = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..6 {
        let p = d.join("re").join("_testdateien").join("par").join("fx").join("base").join("fx_text.fw.png");
        if p.exists() {
            return Some(p);
        }
        if !d.pop() {
            break;
        }
    }
    None
}

fn pixel_rect(effects: &str) -> Option<String> {
    let p = base_text()?;
    let url = format!("file:///{}", p.display().to_string().replace('\\', "/"));
    let src = format!(
        "fw.openDocument({url:?}, false, false); var d = fw.getDocumentDOM(); d.selectAll();
         d.applyEffects({{category:'UNUSED', name:'UNUSED', effects:[{effects}]}});
         var pr = fw.selection[0].pixelRect; var r = pr.left + ',' + pr.top + '-' + pr.right + ',' + pr.bottom; d.close(false); r"
    );
    let mut s = fp_engine::Session::default();
    let o = run_source(&mut s, &src, &ScriptOptions::default());
    assert!(o.ok, "{:?} {:?}", o.error, o.diagnostics);
    o.result
}

const GAUSS: &str = "{d04ef8c0-71b3-11d1-8c8200a024cdc039}";
const BLUR: &str = "{f1cfce41-718e-11d1-8c8200a024cdc039}";
const BLUR_MORE: &str = "{f1cfce42-718e-11d1-8c8200a024cdc039}";

/// Gauß r > 200 („extrem210“): nur das pixelRect des Texts (43,60–148,98) wird deckend gefüllt,
/// der Rand des Objektbilds bleibt transparent.
#[test]
fn gauss_over_200_fills_only_the_text_pixel_rect() {
    let Some(r) = pixel_rect(&format!("{{EffectMoaID:'{GAUSS}', EffectIsVisible:true, gaussian_blur_radius: 210}}")) else { return };
    assert_eq!(r, "43,60-148,98");
}

/// Weichzeichnen und Stärker weichzeichnen 3fach am Text: das Original wächst je Durchgang (pixelRect
/// vorher 43,60–148,98; Messwerte `par\fx\3-weichzeichnen-schaerfen\params.json`).
#[test]
fn blur_three_times_on_text_like_original() {
    for (id, want) in [(BLUR, "41,58-150,100"), (BLUR_MORE, "40,57-151,100")] {
        let one = format!("{{EffectMoaID:'{id}', EffectIsVisible:true}}");
        let three = [one.as_str(); 3].join(",");
        let Some(r3) = pixel_rect(&three) else { return };
        let r1 = pixel_rect(&one).unwrap_or_default();
        assert_eq!(r3, want, "{id}: 1× {r1}, 3× {r3}");
    }
}
