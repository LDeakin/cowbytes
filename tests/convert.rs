//! Tests for conversions to and from `CowBytes`.

use std::borrow::Cow;

use cowbytes::CowBytes;

#[test]
fn cow_roundtrip_preserves_the_borrow() {
    let cow = Cow::Borrowed(&[1u8, 2, 3][..]);
    let raw = CowBytes::from(cow);
    assert!(matches!(raw, CowBytes::Borrowed(_)));
    assert!(matches!(Cow::from(raw), Cow::Borrowed(_)));
}

#[test]
fn boxed_slice_is_adopted_without_copying() {
    let bytes = vec![1u8, 2, 3].into_boxed_slice();
    let ptr = bytes.as_ptr() as usize;
    assert_eq!(CowBytes::from(bytes).as_ptr() as usize, ptr);
}
