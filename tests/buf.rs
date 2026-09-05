//! Tests for the `Buf` implementation.

use cowbytes::{Buf, CowBytes};

#[test]
fn buf_advance_works_on_both_variants() {
    let mut borrowed = CowBytes::Borrowed(&[1u8, 2, 3, 4]);
    borrowed.advance(2);
    assert_eq!(borrowed.remaining(), 2);
    assert_eq!(borrowed.chunk(), [3u8, 4]);

    let mut shared = CowBytes::from(vec![1u8, 2, 3, 4]);
    shared.advance(2);
    assert_eq!(shared.remaining(), 2);
    assert_eq!(shared.chunk(), [3u8, 4]);
}

#[test]
fn buf_copy_to_bytes_does_not_copy_shared_bytes() {
    let bytes = vec![1u8, 2, 3, 4];
    let ptr = bytes.as_ptr() as usize;
    let mut shared = CowBytes::from(bytes);
    let taken = shared.copy_to_bytes(2);
    assert_eq!(&taken[..], &[1u8, 2][..]);
    assert_eq!(taken.as_ptr() as usize, ptr);
    assert_eq!(shared, [3u8, 4]);
}
