use heapless::Vec;

use proto::MAX_GLYPHS;

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DisplaySettings {
    pub calibration: Vec<usize, MAX_GLYPHS>,
}
