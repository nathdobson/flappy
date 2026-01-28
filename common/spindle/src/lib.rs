#![feature(assert_matches)]
#![feature(allocator_api)]
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unused_must_use)]
#![allow(dead_code, unused_parens, unused_imports)]
extern crate alloc;
extern crate core;

mod lexer;
#[cfg(test)]
mod lexer_test;
mod parser;
#[cfg(test)]
mod parser_test;
mod ast;
mod token;
