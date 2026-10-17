//! RT-1 (re 27 P1, P2): „Pinselstrich bearbeiten“ schaltet die Pinselart nie um, auch nicht durch
//! Werte, die nur die natürliche Engine kennt; „Tintenmenge“ ist `concentration` (Promille).

use fp_engine::Session;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::brush_edit::BrushEditDialog;
use serde_json::json;

fn state() -> AppState {
    fp_engine::shell::disable();
    fp_i18n::profile::use_test_profile();
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 200, "height": 100, "background": "#ffffff"})).expect("Dokument");
    s.execute("select.none", json!({})).expect("nichts");
    AppState::new(s, UiSettings::default(), None)
}

#[test]
fn p1_simple_brush_stays_simple_p2_ink_is_concentration() {
    let mut st = state();
    let mut d = BrushEditDialog::for_tool(&st);
    assert_eq!(d.brush.brush_type, 1, "Werkzeugvorgabe: einfacher Pinsel");
    // Werte der natürlichen Engine
    d.brush.tip_count = 3;
    d.brush.angle = 45;
    d.brush.aspect = 500;
    d.set_sensitivity(b'P', b'Z', 50.0);
    // Tintenmenge 45 %
    d.brush.concentration = 450;
    assert_eq!(d.effective().brush_type, 1, "kein Umschalten im Dialog");
    let (cmd, p) = d.invocation();
    assert_eq!(cmd, "pi.tool.brush");
    let sent: fp_model::Brush = serde_json::from_value(p["brush"].clone()).expect("Pinsel");
    assert_eq!(sent.brush_type, 1);
    st.session.execute(&cmd, p).expect("OK");
    let tb = fp_engine::commands::panel::tool_brush(&st.session.tools.options);
    assert_eq!(tb.brush_type, 1, "gespeichert, Pinselart bleibt");
    assert_eq!((tb.tip_count, tb.angle, tb.aspect), (3, 45, 500), "Werte werden trotzdem gespeichert");
    assert_eq!(tb.sensitivity(b'P', b'Z'), 500);
    assert_eq!(tb.concentration, 450, "Tintenmenge = concentration");
    // Mehrfach öffnen und OK: bleibt einfach (keine schleichende Umschaltung)
    for _ in 0..20 {
        let mut d = BrushEditDialog::for_tool(&st);
        d.brush.tip_count = d.brush.tip_count % 32 + 1;
        let (cmd, p) = d.invocation();
        st.session.execute(&cmd, p).expect("OK");
    }
    assert_eq!(fp_engine::commands::panel::tool_brush(&st.session.tools.options).brush_type, 1);
}
