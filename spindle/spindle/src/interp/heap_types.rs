use crate::interp::inline_slice::{InlineSlice, InlineSliceBuilder};
use heapless::String;
use heapless::string::StringView;
use unsized_builder::StringBuilder;

pub type HeapString = InlineSlice<String<0>, StringView>;
pub type HeapStringBuilder = InlineSliceBuilder<StringBuilder, String<0>>;
