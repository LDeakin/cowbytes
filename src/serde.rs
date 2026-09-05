//! `serde` support for [`CowBytes`].

use alloc::string::String;
use alloc::vec::Vec;

use bytes::Bytes;

use crate::CowBytes;

impl ::serde::Serialize for CowBytes<'_> {
    fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(self.as_slice())
    }
}

/// Borrows from the deserializer when it can, and copies otherwise.
///
/// Because the result may borrow, `CowBytes<'a>` is [`Deserialize<'de>`](::serde::Deserialize)
/// only for `'de: 'a`, and so is not [`DeserializeOwned`](::serde::de::DeserializeOwned). A
/// `CowBytes` field of a derived struct therefore needs `#[serde(borrow)]`, and formats that do
/// not lend their input (such as `from_reader`) cannot produce one.
impl<'de: 'a, 'a> ::serde::Deserialize<'de> for CowBytes<'a> {
    fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct CowBytesVisitor<'a>(core::marker::PhantomData<&'a [u8]>);

        impl<'de: 'a, 'a> ::serde::de::Visitor<'de> for CowBytesVisitor<'a> {
            type Value = CowBytes<'a>;

            fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                formatter.write_str("a byte array")
            }

            fn visit_borrowed_bytes<E: ::serde::de::Error>(
                self,
                v: &'de [u8],
            ) -> Result<Self::Value, E> {
                Ok(CowBytes::Borrowed(v))
            }

            fn visit_borrowed_str<E: ::serde::de::Error>(
                self,
                v: &'de str,
            ) -> Result<Self::Value, E> {
                Ok(CowBytes::Borrowed(v.as_bytes()))
            }

            fn visit_bytes<E: ::serde::de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
                Ok(CowBytes::Shared(Bytes::copy_from_slice(v)))
            }

            fn visit_str<E: ::serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                self.visit_bytes(v.as_bytes())
            }

            fn visit_byte_buf<E: ::serde::de::Error>(self, v: Vec<u8>) -> Result<Self::Value, E> {
                Ok(CowBytes::Shared(Bytes::from(v)))
            }

            fn visit_string<E: ::serde::de::Error>(self, v: String) -> Result<Self::Value, E> {
                self.visit_byte_buf(v.into_bytes())
            }

            fn visit_seq<A: ::serde::de::SeqAccess<'de>>(
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
