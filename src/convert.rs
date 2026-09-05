//! Conversions to and from [`CowBytes`].

use alloc::borrow::Cow;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use bytes::Bytes;

use crate::CowBytes;

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

impl<'a> From<&'a String> for CowBytes<'a> {
    #[inline]
    fn from(bytes: &'a String) -> Self {
        Self::Borrowed(bytes.as_bytes())
    }
}

impl From<String> for CowBytes<'_> {
    #[inline]
    fn from(bytes: String) -> Self {
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

impl<'a> From<Cow<'a, str>> for CowBytes<'a> {
    #[inline]
    fn from(bytes: Cow<'a, str>) -> Self {
        match bytes {
            Cow::Borrowed(bytes) => Self::Borrowed(bytes.as_bytes()),
            Cow::Owned(bytes) => Self::Shared(Bytes::from(bytes.into_bytes())),
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
            CowBytes::Borrowed(bytes) => Self::copy_from_slice(bytes),
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

impl From<CowBytes<'_>> for Box<[u8]> {
    #[inline]
    fn from(bytes: CowBytes<'_>) -> Self {
        bytes.into_vec().into_boxed_slice()
    }
}
