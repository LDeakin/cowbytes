//! Contents-based comparison and hashing for [`CowBytes`].

use alloc::borrow::Cow;
use alloc::string::String;
use alloc::vec::Vec;
use core::cmp::Ordering;
use core::hash::{Hash, Hasher};

use bytes::Bytes;

use crate::CowBytes;

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
    &Vec<u8>,
    String,
    &String,
    Bytes,
    &Bytes,
    bytes::BytesMut,
    &bytes::BytesMut,
    Cow<'_, [u8]>,
    &Cow<'_, [u8]>,
    &CowBytes<'_>,
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

impl<const N: usize> PartialEq<CowBytes<'_>> for &[u8; N] {
    #[inline]
    fn eq(&self, other: &CowBytes<'_>) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<const N: usize> PartialOrd<CowBytes<'_>> for &[u8; N] {
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
