//! Umfang der API-Umsetzung (Zahlen für STATUS.md): `cargo test -p fp-script --test coverage -- --nocapture`.

#[test]
fn api_coverage_numbers() {
    let (m, mt, p, pt) = fp_script::coverage();
    let (em, ep) = fp_script::coverage::exceptions();
    println!(
        "Methoden {m}/{mt} (+{em} begründet ausgelassen), Properties {p}/{pt} (+{ep} begründet ausgelassen)"
    );
    assert_eq!(
        m + em,
        mt,
        "jede Methode ist umgesetzt oder begründet ausgelassen"
    );
    assert_eq!(
        p + ep,
        pt,
        "jede Property ist umgesetzt oder begründet ausgelassen"
    );
}
