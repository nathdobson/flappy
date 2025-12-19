use heapless::{String, Vec};

pub const MAX_GLYPH_BYTES: usize = 12;
pub const MAX_GLYPHS: usize = 16;

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DisplayRequest {
    Run(String<128>),
    Test,
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DisplayResponse {
    Start(Vec<String<MAX_GLYPH_BYTES>, MAX_GLYPHS>),
    Stop(Vec<String<MAX_GLYPH_BYTES>, MAX_GLYPHS>),
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DisplayMessage {
    Request(DisplayRequest),
    Response(DisplayResponse),
}