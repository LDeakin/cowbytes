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
//! // A `Vec` is adopted without copying, and handed back by `into_vec` when unshared.
//! let owned = CowBytes::from(vec![1u8, 2, 3]);
//! assert_eq!(borrowed, owned);
//!
//! // Slicing shared bytes keeps the same allocation.
//! let sliced = owned.slice(1..3);
//! assert_eq!(sliced, [2u8, 3]);
//! ```
//!
//! # Comparison
//!
//! For each API of `&[u8]` and [`Bytes`]: ✓ means [`CowBytes`] provides it, ✗ means that type
//! has it but [`CowBytes`] does not, and a blank means it does not apply to that type.
//!
//! | Method / trait | `&[u8]` | [`Bytes`] |
//! | --- | :-: | :-: |
//! | [`len`](CowBytes::len) / [`is_empty`](CowBytes::is_empty) | ✓ | ✓ |
//! | range indexing / [`slice`](CowBytes::slice) | ✓ | ✓ |
//! | [`split_off`](CowBytes::split_off) | ✓ | ✓ |
//! | [`Deref`] to `[u8]` | ✓ | ✓ |
//! | [`AsRef`] / [`Borrow`] as `[u8]` | ✓ | ✓ |
//! | [`Clone`] / [`Debug`](core::fmt::Debug) / [`Default`] | ✓ | ✓ |
//! | [`PartialEq`] / [`Eq`] / [`PartialOrd`] / [`Ord`] / [`Hash`] | ✓ | ✓ |
//! | [`From`] / [`Into`] conversions | ✓ | ✓ |
//! | [`IntoIterator`] | ✓ | ✓ |
//! | [`Buf`] | ✓ | ✓ |
//! | `Serialize` / `Deserialize` (`serde` feature) | ✓ | ✓ |
//! | [`new`](CowBytes::new) / [`from_static`](CowBytes::from_static) | | ✓ |
//! | [`FromIterator<u8>`](FromIterator) | | ✓ |
//! | [`split_to`](CowBytes::split_to) / [`truncate`](CowBytes::truncate) / [`clear`](CowBytes::clear) | | ✓ |
//! | `copy_from_slice` / `slice_ref` / `from_owner` | | ✗ |
//! | `is_unique` / `try_into_mut` | | ✗ |
//!
//! Read-only slice methods (`iter`, `get`, `to_vec`, `split_at`, indexing, …) are reachable
//! through [`Deref`]. The two ✗ rows are [`Bytes`] APIs that do not generalise to a borrow:
//! its constructors are covered by [`From`], [`from_static`](CowBytes::from_static) and
//! [`into_static`](CowBytes::into_static), and its reference count introspection is
//! meaningless for the borrowed variant — [`into_vec`](CowBytes::into_vec) already yields a
//! uniquely owned buffer either way.
//!
//! Neither `&[u8]` nor [`Bytes`] offers mutable access to its contents, so [`CowBytes`] exposes
//! [`with_mut`](CowBytes::with_mut) instead, which copies first.
//!
//! # Feature flags
//! - `std` (default): enables `bytes/std`. Disable for `no_std` (requires `alloc`).
// The `serde` bullet is written twice so that it only links to `serde` when that crate is
// actually a dependency, which keeps `cargo doc` free of unresolved links either way.
#![cfg_attr(
    feature = "serde",
    doc = "- `serde`: implements [`Serialize`](serde::Serialize) and [`Deserialize`](serde::Deserialize)."
)]
#![cfg_attr(
    not(feature = "serde"),
    doc = "- `serde`: implements `Serialize` and `Deserialize`."
)]
#![doc = "  Deserializing borrows from the deserializer where the format allows, so a"]
#![doc = "  [`CowBytes`] field of a derived struct needs `#[serde(borrow)]`."]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::borrow::Cow;
use alloc::boxed::Box;
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
    /// [`into_vec`](CowBytes::into_vec) hands the allocation back when it is unshared.
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
            // here are always uniquely owned. This matters for callers that go on to mutate
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
        // `into_vec` yields a uniquely owned buffer, so the bytes are safe to mutate.
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

impl From<Box<[u8]>> for CowBytes<'_> {
    #[inline]
    fn from(bytes: Box<[u8]>) -> Self {
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
            bytes @ CowBytes::Shared(_) => Cow::Owned(bytes.into_vec()),
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
        bytes.into_vec()
    }
}

impl IntoIterator for CowBytes<'_> {
    type Item = u8;
    type IntoIter = bytes::buf::IntoIter<Self>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        bytes::buf::IntoIter::new(self)
    }
}

impl<'b> IntoIterator for &'b CowBytes<'_> {
    type Item = &'b u8;
    type IntoIter = core::slice::Iter<'b, u8>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

impl FromIterator<u8> for CowBytes<'_> {
    #[inline]
    fn from_iter<T: IntoIterator<Item = u8>>(iter: T) -> Self {
        Self::Shared(Bytes::from_iter(iter))
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for CowBytes<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(self.as_slice())
    }
}

/// Borrows from the deserializer when it can, and copies otherwise.
///
/// Because the result may borrow, `CowBytes<'a>` is [`Deserialize<'de>`](serde::Deserialize)
/// only for `'de: 'a`, and so is not [`DeserializeOwned`](serde::de::DeserializeOwned). A
/// `CowBytes` field of a derived struct therefore needs `#[serde(borrow)]`, and formats that do
/// not lend their input (such as `from_reader`) cannot produce one.
#[cfg(feature = "serde")]
impl<'de: 'a, 'a> serde::Deserialize<'de> for CowBytes<'a> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct CowBytesVisitor<'a>(core::marker::PhantomData<&'a [u8]>);

        impl<'de: 'a, 'a> serde::de::Visitor<'de> for CowBytesVisitor<'a> {
            type Value = CowBytes<'a>;

            fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                formatter.write_str("a byte array")
            }

            fn visit_borrowed_bytes<E: serde::de::Error>(
                self,
                v: &'de [u8],
            ) -> Result<Self::Value, E> {
                Ok(CowBytes::Borrowed(v))
            }

            fn visit_borrowed_str<E: serde::de::Error>(
                self,
                v: &'de str,
            ) -> Result<Self::Value, E> {
                Ok(CowBytes::Borrowed(v.as_bytes()))
            }

            fn visit_bytes<E: serde::de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
                Ok(CowBytes::Shared(Bytes::copy_from_slice(v)))
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                self.visit_bytes(v.as_bytes())
            }

            fn visit_byte_buf<E: serde::de::Error>(self, v: Vec<u8>) -> Result<Self::Value, E> {
                Ok(CowBytes::Shared(Bytes::from(v)))
            }

            fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Self::Value, E> {
                self.visit_byte_buf(v.into_bytes())
            }

            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<Self::Value, A::Error> {
                // Formats without a byte type (such as JSON) present bytes as a sequence. The
                // size hint is not trusted, so it is capped rather than used to preallocate.
                let mut bytes = Vec::with_capacity(seq.size_hint().unwrap_or(0).min(4096));
                while let Some(byte) = seq.next_element()? {
                    bytes.push(byte);
                }
                self.visit_byte_buf(bytes)
            }
        }

        deserializer.deserialize_bytes(CowBytesVisitor(core::marker::PhantomData))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec; // `vec!` is not in the prelude without `std`.

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
    fn with_mut_promotes_a_borrow() {
        let mut bytes = CowBytes::Borrowed(&[1u8, 2, 3]);
        bytes.with_mut(|bytes| bytes[0] = 9);
        assert_eq!(bytes, [9u8, 2, 3]);
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
    fn iterates_by_value_and_by_reference() {
        let bytes = CowBytes::from(vec![1u8, 2, 3]);
        assert_eq!(
            (&bytes).into_iter().copied().collect::<Vec<u8>>(),
            [1, 2, 3]
        );
        assert_eq!(bytes.into_iter().collect::<Vec<u8>>(), [1, 2, 3]);
        assert_eq!([1u8, 2, 3].into_iter().collect::<CowBytes>(), [1u8, 2, 3]);
    }

    #[test]
    fn boxed_slice_is_adopted_without_copying() {
        let bytes = vec![1u8, 2, 3].into_boxed_slice();
        let ptr = bytes.as_ptr() as usize;
        assert_eq!(CowBytes::from(bytes).as_ptr() as usize, ptr);
    }

    #[test]
    fn len_and_is_empty() {
        assert_eq!(CowBytes::from(&[1u8, 2, 3][..]).len(), 3);
        assert!(CowBytes::default().is_empty());
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

    #[cfg(feature = "serde")]
    #[test]
    fn serde_roundtrip() {
        let bytes = CowBytes::from(vec![1u8, 2, 3]);
        let json = serde_json::to_string(&bytes).unwrap();
        // JSON has no byte type, so this arrives as a sequence and cannot be borrowed.
        let back: CowBytes<'_> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, bytes);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn deserialize_borrows_when_the_format_lends_its_input() {
        // A self-describing format that lends `&'de str` yields a borrow rather than a copy.
        let json = "\"abc\"";
        let bytes: CowBytes<'_> = serde_json::from_str(json).unwrap();
        assert!(matches!(bytes, CowBytes::Borrowed(_)));
        assert_eq!(bytes, "abc");
        assert_eq!(bytes.as_ptr() as usize, json.as_ptr() as usize + 1);
    }
}

#[cfg(all(doctest, feature = "std"))]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
