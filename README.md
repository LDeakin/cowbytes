# cowbytes

[![crates.io](https://img.shields.io/crates/v/cowbytes.svg)](https://crates.io/crates/cowbytes)
[![docs.rs](https://docs.rs/cowbytes/badge.svg)](https://docs.rs/cowbytes)

A clone-on-write bytes type whose non-borrowed variant is [`bytes::Bytes`](https://docs.rs/bytes/latest/bytes/struct.Bytes.html).

```rust,ignore
pub enum CowBytes<'a> {
    Borrowed(&'a [u8]),
    Shared(Bytes),
}
```

Unlike `Cow<'a, [u8]>`, whose owned side is a `Vec<u8>`, **cloning and slicing a shared value are reference count operations rather than copies**.

Unlike `Bytes`, which can only borrow `&'static` data, **an arbitrary slice can be held without copying it**, at the cost of a lifetime.

## Example

```rust
use cowbytes::CowBytes;

// Borrowing does not allocate.
let borrowed: CowBytes<'_> = CowBytes::from(&[1u8, 2, 3][..]);

// A `Vec` is adopted without copying.
let shared: CowBytes<'_> = CowBytes::from(vec![1u8, 2, 3]);

// Equality is by contents, not by variant.
assert_eq!(borrowed, shared);

// Slicing shared bytes keeps the same allocation.
let sliced: CowBytes<'_> = shared.slice(1..3);
assert_eq!(sliced, [2u8, 3]);

// `into_vec` hands the allocation back while it is unshared, rather than copying.
let bytes = vec![1u8, 2, 3];
let ptr = bytes.as_ptr();
let owned: Vec<u8> = CowBytes::from(bytes).into_vec();
assert_eq!(owned.as_ptr(), ptr);
```

## Feature flags

- `std` *(default)* — enables `bytes/std`. Disable for `no_std` (requires `alloc`).
- `serde` — implements `Serialize` and `Deserialize`. Add `#[serde(borrow)]` to struct fields holding `CowBytes<'a>` to allow zero-copy deserialization from input data.

## Licence

Licensed under either of

- Apache Licence, Version 2.0 ([LICENSE-APACHE](./LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT Licence ([LICENSE-MIT](./LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
