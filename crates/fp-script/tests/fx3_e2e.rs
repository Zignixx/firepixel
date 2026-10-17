//! FX-3 (Analyse-Werkzeug): Messreihe `re\23-effekte.md` Gruppe 3 mit demselben Ablauf wie im
//! Original (Datei öffnen → `selectAll` → `dom.applyEffects` → `dom.exportTo` PNG 32 ganze
//! Leinwand) in Firepixel wiederholen. Nur auf Anforderung:
//! `FP_FX3_REF=<Ordner der Gruppe> FP_FX3_OUT=<Ziel> cargo test -p fp-script --test fx3_e2e -- --ignored`

use fp_script::{ScriptOptions, run_source};

#[test]
#[ignore]
fn fx3_end_to_end_export() {
    let (Ok(refdir), Ok(out)) = (std::env::var("FP_FX3_REF"), std::env::var("FP_FX3_OUT")) else {
        return;
    };
    let refdir = std::path::PathBuf::from(refdir);
    let params: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(refdir.join("params.json")).expect("params.json"),
    )
    .expect("json");
    let base = refdir.join("..").join("base");
    let url =
        |p: &std::path::Path| format!("file:///{}", p.display().to_string().replace('\\', "/"));
    let only = std::env::var("FP_FX3_ONLY").unwrap_or_default();
    let mut cases = Vec::new();
    for c in params["cases"].as_array().expect("cases") {
        let name = format!(
            "{}_{}",
            c["effect"].as_str().unwrap_or(""),
            c["setting"].as_str().unwrap_or("")
        );
        if only.is_empty() || name.contains(&only) {
            cases.push(format!("[{:?}, {}]", name, c["effects"]));
        }
    }
    for (obj, file) in [
        ("vektor", "fx_vector.fw.png"),
        ("text", "fx_text.fw.png"),
        ("bitmap", "fx_bitmap.fw.png"),
    ] {
        let src = format!(
            "var CASES = [{}];
             fw.openDocument({:?}, false, false);
             var d = fw.getDocumentDOM(); d.selectAll();
             var log = [];
             for (var c = 0; c < CASES.length; c++) {{
               d.applyEffects({{category:'UNUSED', name:'UNUSED', effects:CASES[c][1]}});
               var eo = d.exportOptions; eo.exportFormat = 'PNG'; eo.colorMode = '32 bit'; eo.crop = false;
               var u = {:?} + CASES[c][0] + '_{obj}.png';
               d.exportTo(u, eo);
               var pr = fw.selection[0].pixelRect;
               log.push(CASES[c][0] + ' ' + pr.left + ',' + pr.top + '-' + pr.right + ',' + pr.bottom);
             }}
             d.close(false);
             log.join('\\n')",
            cases.join(",\n"),
            url(&base.join(file)),
            format!("{}/", url(std::path::Path::new(&out)))
        );
        let mut s = fp_engine::Session::default();
        let o = run_source(&mut s, &src, &ScriptOptions::default());
        assert!(o.ok, "{obj}: {:?} {:?}", o.error, o.diagnostics);
        println!("{}", o.result.unwrap_or_default());
    }
}

fn result(src: &str) -> String {
    let mut s = fp_engine::Session::default();
    let o = run_source(&mut s, src, &ScriptOptions::default());
    assert!(o.ok, "Fehler: {:?}\nDiagnose: {:?}", o.error, o.diagnostics);
    o.result.unwrap_or_default()
}

const NEW: &str = "var dom = fw.createFirepixelDocument({x: 200, y: 200}, {pixelsPerUnit: 72, units: 'inch'}, '#ffffff00'); dom = fw.getDocumentDOM();
    dom.addNewRectangle({left:50, top:50, right:150, bottom:120}, 0);
    dom.setFillNColor({category:'fc_Solid', name:'fn_Normal', shape:'solid'}, '#3a7bd5');\n";

/// §4b: Filter-Live-Effekt 50× mit neuem Wert setzen → genau ein Eintrag mit dem Endwert, gleiche
/// Grenzen wie einmal gesetzt; Rückgängig stellt den Ausgangszustand her; Auswahlwechsel wirkt nur
/// auf das neu gewählte Objekt.
#[test]
fn filter_effect_repeat_undo_and_selection() {
    let gauss = "{d04ef8c0-71b3-11d1-8c8200a024cdc039}";
    let r = result(&format!(
        "{NEW} var a = []; try {{
         for (var i = 0; i < 50; i++) dom.applyEffects({{category:'UNUSED', name:'UNUSED', effects:[{{EffectMoaID:'{gauss}', EffectIsVisible:true, gaussian_blur_radius: 0.5 + i}}]}});
         dom.applyEffects({{category:'UNUSED', name:'UNUSED', effects:[{{EffectMoaID:'{gauss}', EffectIsVisible:true, gaussian_blur_radius: 4.1}}]}});
         var l = fw.selection[0].effectList.effects;
         a.push(l.length, l[0].gaussian_blur_radius);
         var u = dom.layers[0].frames[0].elements[0]; var cnt = function () {{ return u.effectList ? u.effectList.effects.length : 0; }};
         fw.undo(); a.push(cnt()); fw.redo(); a.push(cnt(), u.effectList.effects[0].gaussian_blur_radius);
         dom.addNewRectangle({{left:10, top:10, right:30, bottom:30}}, 0);
         dom.applyEffects({{category:'UNUSED', name:'UNUSED', effects:[{{EffectMoaID:'{{538016b6-ffd6-418f-a10c590e6ee841ae}}', EffectIsVisible:true, motion_blur_angle:0, motion_blur_distance:1}}]}});
         var els = dom.layers[0].frames[0].elements;
         var kinds = [];
         for (var k = 0; k < els.length; k++) {{ var f = els[k].effectList ? els[k].effectList.effects : []; kinds.push(f.length + ':' + (f.length ? (f[0].gaussian_blur_radius || ('m' + f[0].motion_blur_distance)) : '-')); }}
         kinds.sort(); a.push(els.length, kinds.join(','));
         }} catch (x) {{ a.push('ERR ' + x); }} a.join('|')"
    ));
    // Rückgängig nimmt die zusammengefassten Änderungen zurück (kein Effekt mehr, nicht 50 Schritte),
    // Wiederholen stellt den Endwert her; der zweite Effekt landet nur am neu gewählten Objekt.
    assert_eq!(r, "1|4.1|0|1|4.1|2|1:4.1,1:m1", "{r}");
}
