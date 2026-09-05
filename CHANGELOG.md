# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `CowBytes`, a clone-on-write bytes type whose owned variant is `bytes::Bytes`
- `no_std` support via the default-on `std` feature
- Optional `serde` support behind the `serde` feature
  - Deserialization borrows from the deserializer where the format lends its input, so
    `CowBytes<'a>` is `Deserialize<'de>` for `'de: 'a` and is not `DeserializeOwned`
- `CowBytes::new`, matching `Bytes::new`
- `Buf`, `IntoIterator` (owned and by reference), `FromIterator<u8>` and `From<Box<[u8]>>` impls
- `PartialOrd<T>` for any `T: AsRef<[u8]>`, and reverse `PartialEq`/`PartialOrd` impls so that a
  `CowBytes` may appear on either side of a comparison
- A `&[u8]` / `Bytes` API comparison table in the crate documentation

### Changed
- `CowBytes::slice` takes `&self` and `impl RangeBounds<usize>` rather than `self` and
  `Range<usize>`, matching `Bytes::slice`
