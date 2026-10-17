//! Hilfswerkzeug: Tag-Baum (`mkTS`) einer FW-PNG-Datei als Text ausgeben.
//! `FP_DUMP_FILE=x.fw.png FP_DUMP_OUT=x.txt cargo test -p fp-fwpng --test dump -- --ignored`

#[test]
#[ignore]
fn dump_tree() {
    let Ok(p) = std::env::var("FP_DUMP_FILE") else {
        return;
    };
    let bytes = std::fs::read(&p).expect("Datei");
    let raw = fp_fwpng::read_raw(&bytes, false).expect("lesen");
    let text = fp_fwpng::tree::pretty(raw.ts.as_deref().unwrap_or(&[]), 60);
    match std::env::var("FP_DUMP_OUT") {
        Ok(o) => std::fs::write(o, text).expect("schreiben"),
        Err(_) => println!("{text}"),
    }
}
