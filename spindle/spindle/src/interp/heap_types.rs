use crate::interp::inline_slice::{InlineSlice, InlineSliceInPlace};
use heapless::String;
use heapless::string::{StringInPlace, StringView};

pub type HeapString = InlineSlice<String<0>, StringView>;
pub type HeapStringInPlace = InlineSliceInPlace<StringInPlace, String<0>>;

