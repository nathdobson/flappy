#![feature(
    assert_matches,
    allocator_api,
    never_type,
    type_alias_impl_trait,
    vec_push_within_capacity,
    ptr_metadata,
    unsafe_pinned,
    async_fn_traits,
    core_intrinsics,
    unboxed_closures,
    deref_patterns,
    trait_alias
)]
#![feature(unsize)]
#![feature(coerce_unsized)]
#![cfg_attr(not(test), no_std)]
#![deny(unused_must_use, unsafe_op_in_unsafe_fn)]
#![allow(
    dead_code,
    unused_parens,
    unused_imports,
    internal_features,
    unused_mut,
    unused_variables,
    incomplete_features
)]
extern crate alloc;
extern crate core;

pub mod ast;
pub mod compiler;
#[cfg(test)]
mod compiler_test;
pub mod interp;
pub mod lexer;
#[cfg(test)]
mod lexer_test;
mod lookahead;
pub mod parser;
#[cfg(test)]
mod parser_test;
pub mod stack;
#[cfg(test)]
mod stack_test;
#[cfg(test)]
mod testutils;
pub mod token;
mod vec_ext;
pub mod vm;
pub mod native;
