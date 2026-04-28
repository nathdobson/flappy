#![no_std]
#![feature(allocator_api)]
#![deny(unused_must_use)]
#![allow(unused_imports)]
#![allow(unused_mut)]
#![allow(unused_variables)]
#![allow(dead_code)]
#![feature(never_type)]
#![allow(unreachable_code)]

mod error;
pub use error::Error;
mod reader;
mod varint;
mod writer;
pub mod client;
