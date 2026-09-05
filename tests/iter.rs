//! Tests for iteration.

use cowbytes::CowBytes;

#[test]
fn iterates_by_value_and_by_reference() {
    let bytes = CowBytes::from(vec![1u8, 2, 3]);
    assert_eq!(
        (&bytes).into_iter().copied().collect::<Vec<u8>>(),
        [1, 2, 3]
    );
    assert_eq!(bytes.into_iter().collect::<Vec<u8>>(), [1, 2, 3]);
    assert_eq!([1u8, 2, 3].into_iter().collect::<CowBytes>(), [1u8, 2, 3]);
}
