use heapless::{String, Vec};

pub const MAX_GLYPH_BYTES: usize = 12;
pub const MAX_GLYPHS: usize = 16;
pub const DISPLAY_REQUEST_CAPACITY: usize = 1024;

pub const STEPS_PER_REVOLUTION: usize = 2048;

#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DisplayRequest {
    Run(String<DISPLAY_REQUEST_CAPACITY>),
    RunSpindle(String<DISPLAY_REQUEST_CAPACITY>),
    Test,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DisplayResponse {
    Start(Vec<String<MAX_GLYPH_BYTES>, MAX_GLYPHS>),
    Stop(Vec<String<MAX_GLYPH_BYTES>, MAX_GLYPHS>),
}
