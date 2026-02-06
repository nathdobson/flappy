#![feature(
    assert_matches,
    allocator_api,
    never_type,
    type_alias_impl_trait,
    vec_push_within_capacity,
    ptr_metadata,
    unsafe_pinned,
    core_intrinsics
)]
#![cfg_attr(not(test), no_std)]
#![deny(unused_must_use)]
#![allow(
    dead_code,
    unused_parens,
    unused_imports,
    internal_features,
    unused_mut,
    unused_variables
)]
extern crate alloc;
extern crate core;

mod ast;
mod compiler;
#[cfg(test)]
mod compiler_test;
// mod freelist;
mod interp;
mod lexer;
#[cfg(test)]
mod lexer_test;
mod parser;
#[cfg(test)]
mod parser_test;
mod pin_stack;
mod pool;
// mod stack;
mod token;
mod vec_ext;
mod vm;
