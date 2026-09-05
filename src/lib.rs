//! A clone-on-write bytes type whose non-borrowed variant is [`Bytes`].
//!
//! ```rust,ignore
//! pub enum CowBytes<'a> {
//!     Borrowed(&'a [u8]),
//!     Shared(Bytes),
//! }
//! ```
//!
//! Unlike [`Cow<[u8]>`](alloc::borrow::Cow), whose owned side is a [`Vec<u8>`], cloning and slicing a shared value are reference count operations rather than copies.
//!
//! Unlike [`Bytes`], which can only borrow `&'static` data, an arbitrary slice can be held without copying it, at the cost of a lifetime.
//!
//! ```rust
//! # use cowbytes::CowBytes;
//! // Borrowing does not allocate.
//! let borrowed: CowBytes<'_> = CowBytes::from(&[1u8, 2, 3][..]);
//! 
//! // A `Vec` is adopted without copying.
//! let shared: CowBytes<'_> = CowBytes::from(vec![1u8, 2, 3]);
//! 
//! // Equality is by contents, not by variant.
//! assert_eq!(borrowed, shared);
//! 
//! // Slicing shared bytes keeps the same allocation.
//! let sliced: CowBytes<'_> = shared.slice(1..3);
//! assert_eq!(sliced, [2u8, 3]);
//! 
//! // `into_vec` hands the allocation back while it is unshared, rather than copying.
//! let bytes = vec![1u8, 2, 3];
//! let ptr = bytes.as_ptr();
//! let owned: Vec<u8> = CowBytes::from(bytes).into_vec();
//! assert_eq!(owned.as_ptr(), ptr);
//! ```
//!
//! # Feature flags
//! - `std` (default): enables `bytes/std`. Disable for `no_std` (requires `alloc`).
#![cfg_attr(
    feature = "serde",
    doc = "- `serde`: implements [`Serialize`](::serde::Serialize) and [`Deserialize`](::serde::Deserialize)."
)]
#![cfg_attr(
    not(feature = "serde"),
    doc = "- `serde`: implements `Serialize` and `Deserialize`."
)]
#![doc = "  Deserializing borrows from the deserializer where the format allows, so a"]
#![doc = "  [`CowBytes`] field of a derived struct needs `#[serde(borrow)]`."]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod buf;
mod cmp;
mod convert;
mod iter;
#[cfg(feature = "serde")]
mod serde;

use alloc::vec::Vec;
use core::borrow::Borrow;
use core::ops::{Deref, RangeBounds};

pub use bytes::{Buf, Bytes};

/// A [`Cow`](alloc::borrow::Cow) whose non-borrowed variant is a reference counted [`Bytes`]
/// rather than an owned [`Vec<u8>`].
///
/// See the [crate documentation](crate) for an overview.
#[derive(Clone, Debug)]
pub enum CowBytes<'a> {
    /// Bytes borrowed from an existing buffer.
    Borrowed(&'a [u8]),
    /// Bytes shared with a reference counted buffer.
    ///
    /// Cloning and slicing these bytes does not copy the underlying buffer. A [`Vec`] is
    /// represented here too: [`Bytes::from`] adopts one without copying, and
    /// [`into_vec`](CowBytes::into_vec) hands the allocation back while it is unshared.
    Shared(Bytes),
}

impl<'a> CowBytes<'a> {
    /// Creates an empty [`CowBytes`] without allocating.
    #[must_use]
    #[inline]
    pub const fn new() -> Self {
        Self::Borrowed(&[])
    }

    /// Creates a [`CowBytes`] from a static slice without copying or allocating.
    ///
    /// Unlike [`into_static`](CowBytes::into_static) on a borrowed value, this never copies, since
    /// [`Bytes`] can reference `'static` data directly.
    #[must_use]
    #[inline]
    pub const fn from_static(bytes: &'static [u8]) -> Self {
        Self::Shared(Bytes::from_static(bytes))
    }

    /// Returns the bytes as a slice.
    #[must_use]
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        match self {
            Self::Borrowed(bytes) => bytes,
            Self::Shared(bytes) => bytes,
        }
    }

    /// Returns the number of bytes.
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.as_slice().len()
    }

    /// Returns true if there are no bytes.
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.as_slice().is_empty()
    }

    /// Returns the bytes as a [`Vec`], copying only if they are borrowed.
    #[must_use]
    #[inline]
    pub fn into_vec(self) -> Vec<u8> {
        match self {
            Self::Borrowed(bytes) => bytes.to_vec(),
            // `try_into_mut` only succeeds if the buffer is not shared, so the bytes returned
            // here are always unshared. This matters for callers that go on to mutate
            // them through an `UnsafeCellSlice`.
            Self::Shared(bytes) => bytes
                .try_into_mut()
                .map_or_else(|bytes| bytes.to_vec(), Into::into),
        }
    }

    /// Returns a subslice of the bytes without copying.
    ///
    /// A subsequent [`into_vec`](CowBytes::into_vec) may have to shift the bytes to the
    /// front of the buffer, which is still cheaper than copying them here.
    ///
    /// # Panics
    /// Panics if `range` is out of bounds.
    ///
    /// # Examples
    /// ```
    /// # use cowbytes::CowBytes;
    /// let bytes = CowBytes::from(&[1u8, 2, 3][..]);
    /// assert_eq!(bytes.slice(1..), [2u8, 3]);
    /// assert_eq!(bytes.slice(..2), [1u8, 2]);
    /// ```
    #[must_use]
    #[inline]
    pub fn slice(&self, range: impl RangeBounds<usize>) -> CowBytes<'a> {
        // `(Bound, Bound)` implements both `RangeBounds` and `SliceIndex`, so the same
        // resolved range serves the borrowed and the shared arm.
        let range = (range.start_bound().cloned(), range.end_bound().cloned());
        match self {
            Self::Borrowed(bytes) => CowBytes::Borrowed(&bytes[range]),
            Self::Shared(bytes) => CowBytes::Shared(bytes.slice(range)),
        }
    }

    /// Splits the bytes in two at `at` without copying, returning the bytes at and beyond it.
    ///
    /// Afterwards `self` contains `[0, at)`. Follows the signature of [`Bytes::split_off`]
    /// rather than the range-taking `split_off` on slices.
    ///
    /// # Panics
    /// Panics if `at > len`.
    ///
    /// # Examples
    /// ```
    /// # use cowbytes::CowBytes;
    /// let mut bytes = CowBytes::from(vec![1u8, 2, 3]);
    /// assert_eq!(bytes.split_off(1), [2u8, 3]);
    /// assert_eq!(bytes, [1u8]);
    /// ```
    #[must_use = "use `truncate` to discard the bytes beyond `at`"]
    #[inline]
    pub fn split_off(&mut self, at: usize) -> CowBytes<'a> {
        match self {
            Self::Borrowed(bytes) => {
                let (head, tail) = bytes.split_at(at);
                *bytes = head;
                CowBytes::Borrowed(tail)
            }
            Self::Shared(bytes) => CowBytes::Shared(bytes.split_off(at)),
        }
    }

    /// Splits the bytes in two at `at` without copying, returning the bytes before it.
    ///
    /// Afterwards `self` contains `[at, len)`.
    ///
    /// # Panics
    /// Panics if `at > len`.
    #[must_use = "use `advance` from `Buf` to discard the bytes before `at`"]
    #[inline]
    pub fn split_to(&mut self, at: usize) -> CowBytes<'a> {
        match self {
            Self::Borrowed(bytes) => {
                let (head, tail) = bytes.split_at(at);
                *bytes = tail;
                CowBytes::Borrowed(head)
            }
            Self::Shared(bytes) => CowBytes::Shared(bytes.split_to(at)),
        }
    }

    /// Shortens the bytes to `len`, without copying and keeping the first `len` of them.
    ///
    /// Does nothing if `len` is greater than the current length.
    #[inline]
    pub fn truncate(&mut self, len: usize) {
        match self {
            Self::Borrowed(bytes) => {
                if len < bytes.len() {
                    *bytes = &bytes[..len];
                }
            }
            Self::Shared(bytes) => bytes.truncate(len),
        }
    }

    /// Discards all of the bytes.
    #[inline]
    pub fn clear(&mut self) {
        self.truncate(0);
    }

    /// Converts into [`Bytes`], copying only if the bytes are borrowed.
    ///
    /// # Examples
    /// ```
    /// # use cowbytes::{Bytes, CowBytes};
    /// let bytes = CowBytes::from(&[1u8, 2, 3][..]);
    /// assert_eq!(bytes.into_bytes(), Bytes::from_static(&[1, 2, 3]));
    /// ```
    #[must_use]
    #[inline]
    pub fn into_bytes(self) -> Bytes {
        Bytes::from(self)
    }

    /// Convert into a [`CowBytes<'static>`], copying only if the bytes are borrowed.
    ///
    /// Unlike [`into_vec`](CowBytes::into_vec), shared bytes are retained as-is rather
    /// than copied into a [`Vec`].
    #[must_use]
    #[inline]
    pub fn into_static(self) -> CowBytes<'static> {
        match self {
            Self::Borrowed(bytes) => CowBytes::Shared(Bytes::copy_from_slice(bytes)),
            Self::Shared(bytes) => CowBytes::Shared(bytes),
        }
    }

    /// Applies `f` to the bytes, copying them first if they are borrowed or shared, and returns
    /// whatever `f` returns.
    ///
    /// [`Bytes`] offers no in-place mutable access, so this takes a closure rather than returning
    /// a mutable reference. The bytes are left as `f` mutated them whatever it returns, so a
    /// fallible `f` may return a [`Result`] without losing its changes.
    ///
    /// # Examples
    /// ```
    /// # use cowbytes::CowBytes;
    /// let mut bytes = CowBytes::from(&[1u8, 2, 3][..]);
    /// bytes.with_mut(|bytes| bytes[0] = 9);
    /// assert_eq!(bytes, [9u8, 2, 3]);
    ///
    /// // A fallible closure keeps its changes even on the error path.
    /// let result = bytes.with_mut(|bytes| {
    ///     bytes[1] = 8;
    ///     Err::<(), _>("failed")
    /// });
    /// assert_eq!(result, Err("failed"));
    /// assert_eq!(bytes, [9u8, 8, 3]);
    /// ```
    #[inline]
    pub fn with_mut<R>(&mut self, f: impl FnOnce(&mut [u8]) -> R) -> R {
        // `into_vec` yields an unshared buffer, so the bytes are safe to mutate.
        let mut bytes = core::mem::replace(self, Self::Borrowed(&[])).into_vec();
        let result = f(&mut bytes);
        *self = Self::Shared(Bytes::from(bytes));
        result
    }
}

impl Default for CowBytes<'_> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for CowBytes<'_> {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl AsRef<[u8]> for CowBytes<'_> {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl Borrow<[u8]> for CowBytes<'_> {
    #[inline]
    fn borrow(&self) -> &[u8] {
        self.as_slice()
    }
}

#[cfg(all(doctest, feature = "std"))]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
