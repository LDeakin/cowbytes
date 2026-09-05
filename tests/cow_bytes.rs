//! Tests for the `CowBytes` type and its inherent API.

use cowbytes::CowBytes;

#[test]
fn with_mut_promotes_a_borrow() {
    let mut bytes = CowBytes::Borrowed(&[1u8, 2, 3]);
    bytes.with_mut(|bytes| bytes[0] = 9);
    assert_eq!(bytes, [9u8, 2, 3]);
}

#[test]
fn with_mut_keeps_changes_on_error() {
    let mut bytes = CowBytes::from(vec![1u8, 2, 3]);
    let result: Result<(), &str> = bytes.with_mut(|bytes| {
        bytes[0] = 9;
        Err("failed")
    });
    assert_eq!(result, Err("failed"));
    assert_eq!(bytes, [9u8, 2, 3]);

    // A non-`Result` return value passes straight through.
    assert_eq!(bytes.with_mut(|bytes| bytes.len()), 3);
}

#[test]
fn into_vec_reuses_an_unshared_buffer() {
    // The allocation is handed back rather than copied when it is not shared.
    let bytes = vec![1u8, 2, 3, 4];
    let ptr = bytes.as_ptr() as usize;
    let owned = CowBytes::from(bytes).into_vec();
    assert_eq!(owned.as_ptr() as usize, ptr);
}

#[test]
fn slice_does_not_copy_shared_bytes() {
    // Slicing keeps the same allocation rather than reallocating.
    let bytes = vec![1u8, 2, 3, 4];
    let ptr = bytes.as_ptr() as usize;
    let sliced = CowBytes::from(bytes).slice(1..3);
    assert_eq!(sliced, [2u8, 3]);
    assert_eq!(sliced.as_ptr() as usize, ptr + 1);
}

#[test]
fn slice_accepts_any_range_bounds() {
    let bytes = CowBytes::from(vec![1u8, 2, 3, 4]);
    assert_eq!(bytes.slice(1..), [2u8, 3, 4]);
    assert_eq!(bytes.slice(..2), [1u8, 2]);
    assert_eq!(bytes.slice(..), [1u8, 2, 3, 4]);
    assert_eq!(bytes.slice(1..=2), [2u8, 3]);
    // Slicing borrows rather than consumes, so the original is still usable.
    assert_eq!(bytes.len(), 4);
}

#[test]
fn split_does_not_copy_either_variant() {
    static DATA: &[u8] = &[1u8, 2, 3, 4];

    let bytes = vec![1u8, 2, 3, 4];
    let ptr = bytes.as_ptr() as usize;
    let mut shared = CowBytes::from(bytes);
    let tail = shared.split_off(1);
    assert_eq!(shared, [1u8]);
    assert_eq!(tail, [2u8, 3, 4]);
    assert_eq!(shared.as_ptr() as usize, ptr);
    assert_eq!(tail.as_ptr() as usize, ptr + 1);

    let mut borrowed = CowBytes::Borrowed(DATA);
    let head = borrowed.split_to(1);
    assert_eq!(head, [1u8]);
    assert_eq!(borrowed, [2u8, 3, 4]);
    assert_eq!(head.as_ptr() as usize, DATA.as_ptr() as usize);
    assert_eq!(borrowed.as_ptr() as usize, DATA.as_ptr() as usize + 1);
}

#[test]
fn truncate_and_clear_work_on_both_variants() {
    let mut borrowed = CowBytes::Borrowed(&[1u8, 2, 3]);
    borrowed.truncate(5); // A longer length is a no-op.
    assert_eq!(borrowed, [1u8, 2, 3]);
    borrowed.truncate(2);
    assert_eq!(borrowed, [1u8, 2]);
    borrowed.clear();
    assert!(borrowed.is_empty());

    let mut shared = CowBytes::from(vec![1u8, 2, 3]);
    shared.truncate(2);
    assert_eq!(shared, [1u8, 2]);
    shared.clear();
    assert!(shared.is_empty());
}

#[test]
fn from_static_does_not_copy() {
    static DATA: &[u8] = &[1u8, 2, 3, 4];
    let bytes = CowBytes::from_static(DATA);
    assert_eq!(bytes.as_ptr() as usize, DATA.as_ptr() as usize);
    // Unlike a borrow, this is already 'static and stays put.
    assert_eq!(
        bytes.clone().into_static().as_ptr() as usize,
        DATA.as_ptr() as usize
    );
}

#[test]
fn into_static_copies_only_a_borrow() {
    let owned = vec![1u8, 2, 3];
    let ptr = owned.as_ptr() as usize;
    // Shared bytes are retained as-is.
    assert_eq!(CowBytes::from(owned).into_static().as_ptr() as usize, ptr);
}

#[test]
fn clone_of_shared_bytes_does_not_copy() {
    let bytes = CowBytes::from(vec![1u8, 2, 3]);
    let ptr = bytes.as_ptr() as usize;
    assert_eq!(bytes.clone().as_ptr() as usize, ptr);
}

#[test]
fn len_and_is_empty() {
    assert_eq!(CowBytes::from(&[1u8, 2, 3][..]).len(), 3);
    assert!(CowBytes::default().is_empty());
}
