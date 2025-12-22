#![allow(unused_variables)]
#![deny(unused_must_use)]
#![feature(never_type)]
#![allow(dead_code)]

use crate::de::StringDeserializer;
use crate::ser::StringSerializer;
use serde::{Deserialize, Serialize};

pub use de::StringDeserializerError;
pub use ser::StringSerializerError;
mod de;
mod ser;
#[cfg(test)]
mod test;

pub fn to_string<T: Serialize>(value: &T) -> Result<String, StringSerializerError> {
    value.serialize(StringSerializer)
}
pub fn from_str<'de, T: Deserialize<'de>>(value: &'de str) -> Result<T, StringDeserializerError> {
    T::deserialize(StringDeserializer(value))
}
