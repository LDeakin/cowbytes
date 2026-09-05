//! Tests for comparison and hashing.

use cowbytes::{Bytes, CowBytes};

#[test]
fn equality_ignores_the_variant() {
    // A derived `PartialEq` would compare variants, not contents.
    let borrowed = CowBytes::Borrowed(&[1u8, 2, 3]);
    let shared = CowBytes::from(vec![1u8, 2, 3]);
    assert_eq!(borrowed, shared);
    assert_eq!(shared, borrowed);
    assert_ne!(borrowed, CowBytes::from(vec![1u8, 2, 4]));
}

#[test]
fn ordering_compares_contents() {
    // Ordering is by contents, so it holds across variants.
    let borrowed = CowBytes::Borrowed(&[1u8, 2]);
    let shared = CowBytes::from(vec![1u8, 3]);
    assert!(borrowed < shared);

    let shorter = CowBytes::Borrowed(&[1u8]);
    let longer = CowBytes::from(vec![1u8, 0]);
    assert!(shorter < longer);
}

#[test]
fn comparisons_work_from_either_side() {
    let bytes = CowBytes::from(vec![1u8, 2, 3]);
    assert_eq!(vec![1u8, 2, 3], bytes);
    assert_eq!(bytes, vec![1u8, 2, 3]);
    assert_eq!([1u8, 2, 3], bytes);
    assert_eq!(&[1u8, 2, 3][..], bytes);
    assert_eq!(Bytes::from_static(&[1u8, 2, 3]), bytes);
    assert!(vec![1u8, 2] < bytes);
    assert!(bytes > Bytes::from_static(&[1u8, 2]));
}

#[test]
fn hash_matches_equality() {
    use std::hash::BuildHasher;
    let state = std::collections::hash_map::RandomState::new();
    let borrowed = CowBytes::Borrowed(&[1u8, 2, 3]);
    let shared = CowBytes::from(vec![1u8, 2, 3]);
    assert_eq!(borrowed, shared);
    assert_eq!(state.hash_one(&borrowed), state.hash_one(&shared));
}
