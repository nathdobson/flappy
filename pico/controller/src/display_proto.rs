use heapless::Vec;

pub const MAX_CHARS: usize = 12;

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DisplaySettings {
    pub calibration: Vec<usize, MAX_CHARS>,
}
