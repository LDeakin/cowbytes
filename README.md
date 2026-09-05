# cowbytes

[![crates.io](https://img.shields.io/crates/v/cowbytes.svg)](https://crates.io/crates/cowbytes)
[![docs.rs](https://docs.rs/cowbytes/badge.svg)](https://docs.rs/cowbytes)

A clone-on-write bytes type whose non-borrowed variant is [`bytes::Bytes`].

```text
pub enum CowBytes<'a> {
    Borrowed(&'a [u8]),
    Shared(Bytes),
}
```

`CowBytes` sits between the two types in the standard toolbox:

- Unlike `Cow<'a, [u8]>`, whose owned side is a `Vec<u8>`, **cloning and slicing a shared value are reference count operations rather than copies**.
- Unlike `Bytes`, which can only borrow `&'static` data, **an arbitrary slice can be held without copying it**, at the cost of a lifetime.

This is useful for zero-copy I/O pipelines, where a buffer may be borrowed from a caller, shared with a cache, or handed to a store, and you would rather not copy it to move between those states.

## Example

```rust
use cowbytes::CowBytes;

// Borrowing does not allocate.
let borrowed = CowBytes::from(&[1u8, 2, 3][..]);

// A `Vec` is adopted without copying.
let shared = CowBytes::from(vec![1u8, 2, 3]);

// Equality is by contents, not by variant.
assert_eq!(borrowed, shared);

// Slicing shared bytes keeps the same allocation.
let sliced = shared.slice(1..3);
assert_eq!(sliced, [2u8, 3]);

// `into_vec` hands the allocation back while it is unshared, rather than copying.
let bytes = vec![1u8, 2, 3];
let ptr = bytes.as_ptr();
let owned = CowBytes::from(bytes).into_vec();
assert_eq!(owned.as_ptr(), ptr);
```

## Feature flags

- `std` *(default)* — enables `bytes/std`. Disable for `no_std` (requires `alloc`).
- `serde` — implements `Serialize` and `Deserialize`.

## Licence

Licensed under either of

- Apache Licence, Version 2.0 ([LICENSE-APACHE](./LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT Licence ([LICENSE-MIT](./LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
