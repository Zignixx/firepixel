//! TW-133 (re 50 section 1.2): `JSTi 4` ("stretched") loads as left; the other codes are kept.

use fp_text::Alignment;

#[test]
fn from_jsti_maps_stretched_to_left() {
    assert_eq!(Alignment::from_jsti(4), Alignment::Left);
    assert_eq!(Alignment::from_jsti(0x14), Alignment::Left);
    assert_eq!(Alignment::from_jsti(0), Alignment::Left);
    assert_eq!(Alignment::from_jsti(1), Alignment::Center);
    assert_eq!(Alignment::from_jsti(2), Alignment::Right);
    assert_eq!(Alignment::from_jsti(3), Alignment::Justify);
    assert_eq!(Alignment::from_jsti(0x13), Alignment::Justify);
}
