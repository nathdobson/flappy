#![no_std]

#[cfg(test)]
mod test;

use heapless::String;
use heapless::Vec;

type Content = String<128>;
#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FlappyRequest {
    Run(Content),
    Test,
}

pub const MAX_GLYPH_BYTES: usize = 12;
pub const MAX_GLYPHS: usize = 16;

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FlappyResponse {
    Start(Vec<String<MAX_GLYPH_BYTES>, MAX_GLYPHS>),
    Stop(Vec<String<MAX_GLYPH_BYTES>, MAX_GLYPHS>),
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FlappyMessage {
    Request(FlappyRequest),
    Response(FlappyResponse),
}
