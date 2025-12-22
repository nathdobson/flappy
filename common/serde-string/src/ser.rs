use serde::ser::{
    Error, Impossible, SerializeSeq, SerializeStruct, SerializeTuple, SerializeTupleStruct,
};
use serde::{Serialize, Serializer};
use std::fmt::{Display, Formatter};

pub struct StringSerializer;

#[derive(Default)]
pub struct StringSeqSerializer {
    output: Option<String>,
}

#[derive(Debug)]
pub enum StringSerializerError {
    Custom(String),
    UnsupportedType,
    MissingEntry,
}
impl Display for StringSerializerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            StringSerializerError::Custom(custom) => write!(f, "{}", custom),
            StringSerializerError::UnsupportedType => write!(f, "unsupported type"),
            StringSerializerError::MissingEntry => write!(f, "missing entry"),
        }
    }
}
impl std::error::Error for StringSerializerError {}
impl Error for StringSerializerError {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        StringSerializerError::Custom(msg.to_string())
    }
}

impl Serializer for StringSerializer {
    type Ok = String;
    type Error = StringSerializerError;
    type SerializeSeq = StringSeqSerializer;
    type SerializeTuple = StringSeqSerializer;
    type SerializeTupleStruct = StringSeqSerializer;
    type SerializeTupleVariant = Impossible<Self::Ok, Self::Error>;
    type SerializeMap = Impossible<Self::Ok, Self::Error>;
    type SerializeStruct = StringSeqSerializer;
    type SerializeStructVariant = Impossible<Self::Ok, Self::Error>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Err(StringSerializerError::UnsupportedType)
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Err(StringSerializerError::UnsupportedType)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Err(StringSerializerError::UnsupportedType)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Err(StringSerializerError::UnsupportedType)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Err(StringSerializerError::UnsupportedType)
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Err(StringSerializerError::UnsupportedType)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Err(StringSerializerError::UnsupportedType)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Err(StringSerializerError::UnsupportedType)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Err(StringSerializerError::UnsupportedType)
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        Err(StringSerializerError::UnsupportedType)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Err(StringSerializerError::UnsupportedType)
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        Err(StringSerializerError::UnsupportedType)
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(v.to_owned())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Err(StringSerializerError::UnsupportedType)
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(StringSerializerError::UnsupportedType)
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(StringSerializerError::UnsupportedType)
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        Err(StringSerializerError::UnsupportedType)
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Err(StringSerializerError::UnsupportedType)
    }

    fn serialize_newtype_struct<T>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        Err(StringSerializerError::UnsupportedType)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        if len == Some(1) {
            Ok(StringSeqSerializer::default())
        } else {
            Err(StringSerializerError::UnsupportedType)
        }
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        if len == 1 {
            Ok(StringSeqSerializer::default())
        } else {
            Err(StringSerializerError::UnsupportedType)
        }
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        if len == 1 {
            Ok(StringSeqSerializer::default())
        } else {
            Err(StringSerializerError::UnsupportedType)
        }
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(StringSerializerError::UnsupportedType)
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(StringSerializerError::UnsupportedType)
    }

    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        if len == 1 {
            Ok(StringSeqSerializer::default())
        } else {
            Err(StringSerializerError::UnsupportedType)
        }
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(StringSerializerError::UnsupportedType)
    }
}

impl SerializeTupleStruct for StringSeqSerializer {
    type Ok = String;
    type Error = StringSerializerError;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.output = Some(value.serialize(StringSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.output.ok_or(StringSerializerError::MissingEntry)?)
    }
}

impl SerializeTuple for StringSeqSerializer {
    type Ok = String;
    type Error = StringSerializerError;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.output = Some(value.serialize(StringSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.output.ok_or(StringSerializerError::MissingEntry)?)
    }
}

impl SerializeSeq for StringSeqSerializer {
    type Ok = String;
    type Error = StringSerializerError;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.output = Some(value.serialize(StringSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.output.ok_or(StringSerializerError::MissingEntry)?)
    }
}

impl SerializeStruct for StringSeqSerializer {
    type Ok = String;
    type Error = StringSerializerError;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.output = Some(value.serialize(StringSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.output.ok_or(StringSerializerError::MissingEntry)?)
    }
}
