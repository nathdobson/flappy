#![no_std]

#[cfg(test)]
mod test;

use heapless::String;
use serde::{Deserialize, Serialize};

type Content = String<128>;
#[derive(Serialize, Deserialize, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum FlappyRequest {
    Run(Content),
}

#[derive(Serialize, Deserialize, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum FlappyResponse {
    Start(Content),
    Stop(Content),
}

#[derive(Serialize, Deserialize, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum FlappyMessage {
    Request(FlappyRequest),
    Response(FlappyResponse),
}
