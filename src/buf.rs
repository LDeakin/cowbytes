//! [`Buf`] for [`CowBytes`].

use bytes::{Buf, Bytes};

use crate::CowBytes;

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
