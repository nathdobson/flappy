#![no_std]

#[cfg(test)]
mod test;

use heapless::String;

type Content = String<128>;
#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FlappyRequest {
    Run(Content),
    Test,
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FlappyResponse {
    Start(Content),
    Stop(Content),
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FlappyMessage {
    Request(FlappyRequest),
    Response(FlappyResponse),
}
