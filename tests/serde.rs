//! Tests for `serde` support.
#![cfg(feature = "serde")]

use cowbytes::CowBytes;

#[test]
fn serde_roundtrip() {
    let bytes = CowBytes::from(vec![1u8, 2, 3]);
    let json = serde_json::to_string(&bytes).unwrap();
    // JSON has no byte type, so this arrives as a sequence and cannot be borrowed.
    let back: CowBytes<'_> = serde_json::from_str(&json).unwrap();
    assert_eq!(back, bytes);
}

#[test]
fn deserialize_borrows_when_the_format_lends_its_input() {
    // A self-describing format that lends `&'de str` yields a borrow rather than a copy.
    let json = "\"abc\"";
    let bytes: CowBytes<'_> = serde_json::from_str(json).unwrap();
    assert!(matches!(bytes, CowBytes::Borrowed(_)));
    assert_eq!(bytes, "abc");
    assert_eq!(bytes.as_ptr() as usize, json.as_ptr() as usize + 1);
}
