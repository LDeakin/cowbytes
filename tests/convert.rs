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

#[test]
fn cow_str_conversions() {
    let borrowed_cow: Cow<str> = Cow::Borrowed("hello");
    let bytes = CowBytes::from(borrowed_cow);
    assert!(matches!(bytes, CowBytes::Borrowed(_)));
    assert_eq!(bytes, "hello");

    let owned_cow: Cow<str> = Cow::Owned(String::from("world"));
    let bytes = CowBytes::from(owned_cow);
    assert!(matches!(bytes, CowBytes::Shared(_)));
    assert_eq!(bytes, "world");
}

#[test]
fn string_reference_conversion() {
    let s = String::from("hello");
    let bytes = CowBytes::from(&s);
    assert!(matches!(bytes, CowBytes::Borrowed(_)));
    assert_eq!(bytes, "hello");
}

#[test]
fn into_boxed_slice() {
    let bytes = CowBytes::from(vec![1u8, 2, 3]);
    let boxed: Box<[u8]> = Box::from(bytes);
    assert_eq!(&*boxed, &[1u8, 2, 3]);
}
