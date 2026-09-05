//! A clone-on-write bytes type whose owned variant is [`Bytes`].
//!
//! [`CowBytes`] is a [`Cow`]-like enum that is either a borrowed `&[u8]` or a reference counted
//! [`Bytes`]. It sits between the two types in the standard toolbox:
//!
//! - Unlike [`Cow<[u8]>`](Cow), whose owned side is a [`Vec<u8>`], cloning and slicing an owned
//!   value are reference count operations rather than copies.
//! - Unlike [`Bytes`], which can only borrow `&'static` data, an arbitrary slice can be held
//!   without copying it, at the cost of a lifetime.
//!
//! ```
//! # use cowbytes::CowBytes;
//! // Borrowed: no allocation.
//! let borrowed = CowBytes::from(&[1u8, 2, 3][..]);
//!
//! // A `Vec` is adopted without copying, and handed back by `into_owned` when unshared.
//! let owned = CowBytes::from(vec![1u8, 2, 3]);
//! assert_eq!(borrowed, owned);
//!
//! // Slicing shared bytes keeps the same allocation.
//! let sliced = owned.slice(1..3);
//! assert_eq!(sliced, [2u8, 3]);
//! ```
//!
//! # Feature flags
//! - `std` (default): enables `bytes/std`. Disable for `no_std` (requires `alloc`).
//! - `serde`: implements [`Serialize`](serde::Serialize) and [`Deserialize`](serde::Deserialize).

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::borrow::Cow;
use alloc::string::String;
use alloc::vec::Vec;
use core::borrow::Borrow;
use core::cmp::Ordering;
use core::hash::{Hash, Hasher};
use core::ops::{Deref, RangeBounds};

pub use bytes::{Buf, Bytes};

/// A [`Cow`] whose owned variant is [`Bytes`] rather than [`Vec<u8>`].
///
/// See the [crate documentation](crate) for an overview.
#[derive(Clone, Debug)]
pub enum CowBytes<'a> {
    /// Bytes borrowed from an existing buffer.
    Borrowed(&'a [u8]),
    /// Bytes shared with a reference counted buffer.
    ///
    /// Cloning and slicing these bytes does not copy the underlying buffer. Owned bytes are
    /// represented here too: [`Bytes::from`] takes a [`Vec`] without copying, and
    /// [`into_owned`](CowBytes::into_owned) hands the allocation back when it is unshared.
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

    /// Returns the bytes as an owned [`Vec`], copying only if they are borrowed.
    #[must_use]
    #[inline]
    pub fn into_owned(self) -> Vec<u8> {
        match self {
            Self::Borrowed(bytes) => bytes.to_vec(),
            // `try_into_mut` only succeeds if the buffer is not shared, so the bytes returned
            // here are always uniquely owned. This matters for callers that go on to mutate
            // them through an `UnsafeCellSlice`.
            Self::Shared(bytes) => bytes
                .try_into_mut()
                .map_or_else(|bytes| bytes.to_vec(), Into::into),
        }
    }

    /// Returns a subslice of the bytes without copying.
    ///
    /// A subsequent [`into_owned`](CowBytes::into_owned) may have to shift the bytes to the
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

    /// Convert into a [`CowBytes<'static>`], copying only if the bytes are borrowed.
    ///
    /// Unlike [`into_owned`](CowBytes::into_owned), shared bytes are retained as-is rather
    /// than copied into a [`Vec`].
    #[must_use]
    #[inline]
    pub fn into_static(self) -> CowBytes<'static> {
        match self {
            Self::Borrowed(bytes) => CowBytes::Shared(Bytes::copy_from_slice(bytes)),
            Self::Shared(bytes) => CowBytes::Shared(bytes),
        }
    }

    /// Applies `f` to the bytes, copying them first if they are borrowed or shared.
    ///
    /// [`Bytes`] offers no in-place mutable access, so this takes a closure rather than returning
    /// a mutable reference.
    #[inline]
    pub fn mutate(&mut self, f: impl FnOnce(&mut [u8])) {
        self.try_mutate::<core::convert::Infallible>(|bytes| {
            f(bytes);
            Ok(())
        })
        .unwrap_or_else(|err| match err {});
    }

    /// Applies a fallible `f` to the bytes, copying them first if they are borrowed or shared.
    ///
    /// The bytes are left as `f` mutated them even if it returns an error.
    ///
    /// # Errors
    /// Returns the error from `f`.
    #[inline]
    pub fn try_mutate<E>(&mut self, f: impl FnOnce(&mut [u8]) -> Result<(), E>) -> Result<(), E> {
        // `into_owned` yields a uniquely owned buffer, so the bytes are safe to mutate.
        let mut bytes = core::mem::replace(self, Self::Borrowed(&[])).into_owned();
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

/// Reads the bytes as a [`Buf`], so a [`CowBytes`] can be handed to buffer-generic code without
/// first copying a borrow into a [`Bytes`].
impl Buf for CowBytes<'_> {
    #[inline]
    fn remaining(&self) -> usize {
        self.len()
    }

    #[inline]
    fn chunk(&self) -> &[u8] {
        self.as_slice()
    }

    #[inline]
    fn advance(&mut self, cnt: usize) {
        match self {
            Self::Borrowed(bytes) => *bytes = &bytes[cnt..],
            Self::Shared(bytes) => bytes.advance(cnt),
        }
    }

    #[inline]
    fn copy_to_bytes(&mut self, len: usize) -> Bytes {
        match self {
            // Only a borrow has to be copied; the default implementation would copy either.
            Self::Borrowed(bytes) => {
                let (head, tail) = bytes.split_at(len);
                *bytes = tail;
                Bytes::copy_from_slice(head)
            }
            Self::Shared(bytes) => bytes.copy_to_bytes(len),
        }
    }
}

impl<T: AsRef<[u8]> + ?Sized> PartialEq<T> for CowBytes<'_> {
    #[inline]
    fn eq(&self, other: &T) -> bool {
        self.as_slice() == other.as_ref()
    }
}

impl Eq for CowBytes<'_> {}

/// Orders by contents, matching the contents-based [`PartialEq`]. Covers `PartialOrd<Self>`,
/// since [`CowBytes`] is itself `AsRef<[u8]>`.
impl<T: AsRef<[u8]> + ?Sized> PartialOrd<T> for CowBytes<'_> {
    #[inline]
    fn partial_cmp(&self, other: &T) -> Option<Ordering> {
        Some(self.as_slice().cmp(other.as_ref()))
    }
}

impl Ord for CowBytes<'_> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_slice().cmp(other.as_slice())
    }
}

/// Implements the mirror of the blanket [`PartialEq`]/[`PartialOrd`] above, so that a
/// [`CowBytes`] may appear on either side of a comparison.
macro_rules! impl_reverse_cmp {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl PartialEq<CowBytes<'_>> for $ty {
                #[inline]
                fn eq(&self, other: &CowBytes<'_>) -> bool {
                    <$ty as AsRef<[u8]>>::as_ref(self) == other.as_slice()
                }
            }

            impl PartialOrd<CowBytes<'_>> for $ty {
                #[inline]
                fn partial_cmp(&self, other: &CowBytes<'_>) -> Option<Ordering> {
                    Some(<$ty as AsRef<[u8]>>::as_ref(self).cmp(other.as_slice()))
                }
            }
        )+
    };
}

impl_reverse_cmp!(
    [u8],
    &[u8],
    str,
    &str,
    Vec<u8>,
    String,
    Bytes,
    bytes::BytesMut,
    Cow<'_, [u8]>,
);

impl<const N: usize> PartialEq<CowBytes<'_>> for [u8; N] {
    #[inline]
    fn eq(&self, other: &CowBytes<'_>) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<const N: usize> PartialOrd<CowBytes<'_>> for [u8; N] {
    #[inline]
    fn partial_cmp(&self, other: &CowBytes<'_>) -> Option<Ordering> {
        Some(self.as_slice().cmp(other.as_slice()))
    }
}

/// Hashes the bytes, so that it is consistent with the contents-based [`PartialEq`].
impl Hash for CowBytes<'_> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state);
    }
}

impl<'a> From<&'a [u8]> for CowBytes<'a> {
    #[inline]
    fn from(bytes: &'a [u8]) -> Self {
        Self::Borrowed(bytes)
    }
}

impl<'a> From<&'a Vec<u8>> for CowBytes<'a> {
    #[inline]
    fn from(bytes: &'a Vec<u8>) -> Self {
        Self::Borrowed(bytes)
    }
}

impl<'a, const N: usize> From<&'a [u8; N]> for CowBytes<'a> {
    #[inline]
    fn from(bytes: &'a [u8; N]) -> Self {
        Self::Borrowed(bytes)
    }
}

impl<'a> From<&'a str> for CowBytes<'a> {
    #[inline]
    fn from(bytes: &'a str) -> Self {
        Self::Borrowed(bytes.as_bytes())
    }
}

impl From<alloc::string::String> for CowBytes<'_> {
    #[inline]
    fn from(bytes: alloc::string::String) -> Self {
        Self::Shared(Bytes::from(bytes.into_bytes()))
    }
}

impl From<Vec<u8>> for CowBytes<'_> {
    #[inline]
    fn from(bytes: Vec<u8>) -> Self {
        Self::Shared(Bytes::from(bytes))
    }
}

impl<'a> From<Cow<'a, [u8]>> for CowBytes<'a> {
    #[inline]
    fn from(bytes: Cow<'a, [u8]>) -> Self {
        match bytes {
            Cow::Borrowed(bytes) => Self::Borrowed(bytes),
            Cow::Owned(bytes) => Self::Shared(Bytes::from(bytes)),
        }
    }
}

impl<'a> From<CowBytes<'a>> for Cow<'a, [u8]> {
    #[inline]
    fn from(bytes: CowBytes<'a>) -> Self {
        match bytes {
            CowBytes::Borrowed(bytes) => Cow::Borrowed(bytes),
            bytes @ CowBytes::Shared(_) => Cow::Owned(bytes.into_owned()),
        }
    }
}

impl From<bytes::BytesMut> for CowBytes<'_> {
    #[inline]
    fn from(bytes: bytes::BytesMut) -> Self {
        Self::Shared(bytes.freeze())
    }
}

impl From<Bytes> for CowBytes<'_> {
    #[inline]
    fn from(bytes: Bytes) -> Self {
        Self::Shared(bytes)
    }
}

impl From<CowBytes<'_>> for Bytes {
    #[inline]
    fn from(bytes: CowBytes<'_>) -> Self {
        match bytes {
            CowBytes::Borrowed(bytes) => Bytes::copy_from_slice(bytes),
            CowBytes::Shared(bytes) => bytes,
        }
    }
}

impl From<CowBytes<'_>> for Vec<u8> {
    #[inline]
    fn from(bytes: CowBytes<'_>) -> Self {
        bytes.into_owned()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for CowBytes<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(self.as_slice())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for CowBytes<'_> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Deserialize to an owned `Vec`, since the borrowed data cannot outlive the deserializer.
        let bytes = <Vec<u8> as serde::Deserialize>::deserialize(deserializer)?;
        Ok(Self::from(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equality_ignores_the_variant() {
        // A derived `PartialEq` would compare variants, not contents.
        let borrowed = CowBytes::Borrowed(&[1u8, 2, 3]);
        let owned = CowBytes::from(vec![1u8, 2, 3]);
        assert_eq!(borrowed, owned);
        assert_eq!(owned, borrowed);
        assert_ne!(borrowed, CowBytes::from(vec![1u8, 2, 4]));
    }

    #[test]
    fn mutate_promotes_a_borrow() {
        let mut bytes = CowBytes::Borrowed(&[1u8, 2, 3]);
        bytes.mutate(|bytes| bytes[0] = 9);
        assert_eq!(bytes, [9u8, 2, 3]);
    }

    #[test]
    fn into_owned_reuses_an_unshared_buffer() {
        // The allocation is handed back rather than copied when it is not shared.
        let bytes = vec![1u8, 2, 3, 4];
        let ptr = bytes.as_ptr() as usize;
        let owned = CowBytes::from(bytes).into_owned();
        assert_eq!(owned.as_ptr() as usize, ptr);
    }

    #[test]
    fn slice_does_not_copy_owned_bytes() {
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

    #[test]
    fn cow_roundtrip_preserves_the_borrow() {
        let cow = Cow::Borrowed(&[1u8, 2, 3][..]);
        let raw = CowBytes::from(cow);
        assert!(matches!(raw, CowBytes::Borrowed(_)));
        assert!(matches!(Cow::from(raw), Cow::Borrowed(_)));
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

    #[cfg(feature = "std")]
    #[test]
    fn hash_matches_equality() {
        use core::hash::BuildHasher;
        let state = std::collections::hash_map::RandomState::new();
        let borrowed = CowBytes::Borrowed(&[1u8, 2, 3]);
        let owned = CowBytes::from(vec![1u8, 2, 3]);
        assert_eq!(borrowed, owned);
        assert_eq!(state.hash_one(&borrowed), state.hash_one(&owned));
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
    fn len_and_is_empty() {
        assert_eq!(CowBytes::from(&[1u8, 2, 3][..]).len(), 3);
        assert!(CowBytes::default().is_empty());
    }

    #[test]
    fn try_mutate_keeps_changes_on_error() {
        let mut bytes = CowBytes::from(vec![1u8, 2, 3]);
        let result: Result<(), &str> = bytes.try_mutate(|bytes| {
            bytes[0] = 9;
            Err("failed")
        });
        assert_eq!(result, Err("failed"));
        assert_eq!(bytes, [9u8, 2, 3]);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_roundtrip() {
        let bytes = CowBytes::from(vec![1u8, 2, 3]);
        let json = serde_json::to_string(&bytes).unwrap();
        let back: CowBytes<'static> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, bytes);
    }
}

#[cfg(all(doctest, feature = "std"))]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
