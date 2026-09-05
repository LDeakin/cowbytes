//! Iteration over [`CowBytes`].

use bytes::Bytes;

use crate::CowBytes;

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
