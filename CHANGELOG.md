# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased](https://github.com/LDeakin/cowbytes/compare/v0.1.1...HEAD)

## [0.1.1](https://github.com/LDeakin/cowbytes/compare/v0.1.1) - 2026-09-05

### Added
- Add trusted publishing

### Changed
- Minor documentation improvements

## [0.1.0](https://github.com/LDeakin/cowbytes/compare/v0.1.0) - 2026-09-05

### Added
- `CowBytes`, like `Cow<[u8]>`, but with reference-counted `bytes::Bytes` instead of an owned `Vec<u8>`
  - `no_std` support via the default-on `std` feature
  - `serde` support behind the `serde` feature
