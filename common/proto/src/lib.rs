#![no_std]

#[cfg(test)]
mod test;

use heapless::String;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum FlappyRequest {
    Run(String<128>),
}

#[derive(Serialize, Deserialize, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum FlappyResponse {
    Start,
    Stop,
}

#[derive(Serialize, Deserialize, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum FlappyMessage {
    Request(FlappyRequest),
    Response(FlappyResponse),
}
