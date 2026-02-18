#![feature(
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
    trait_alias,
    super_let
)]
#![feature(unsize)]
#![feature(coerce_unsized)]
#![feature(pin_coerce_unsized_trait)]
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

use crate::compiler::ast::Program;
use crate::compiler::codegen::{Codegen, CompileError};
use crate::compiler::lexer::Lexer;
use crate::compiler::parser::{AnnotatedParserError, Parser};
use crate::compiler::stack::{StackStorage, new_stack};
use crate::compiler::stack_executor::stack_executor;
use crate::interp::Interp;
use crate::interp::heap::HeapStorage;
use crate::interp::value::Value;
use crate::native::{NativeFn, PrintFn};
use crate::vm::VmProgram;
use arena::ArenaStorage;
use core::alloc::AllocError;
use heapless::Vec;
use crate::interp::error::InterpError;

pub mod compiler;
pub mod interp;
pub mod native;
#[cfg(test)]
mod testutils;
mod vec_ext;
pub mod vm;

#[derive(Debug)]
pub enum SpindleError<'src> {
    AllocError,
    ParserError(AnnotatedParserError<'src>),
    CompileError(CompileError<'src>),
    InterpError(InterpError),
}

pub struct Spindle<
    const ARENA: usize,
    const STACK_PAR: usize,
    const STACK_VALUE: usize,
    const HEAP_COUNT: usize,
    const HEAP_BYTES: usize,
> {
    arena: [u8; ARENA],
    stack: StackStorage<[u8; STACK_PAR]>,
    value_stack: Vec<Value, STACK_VALUE>,
    heap: HeapStorage<HEAP_COUNT, HEAP_BYTES>,
}

impl<
    const ARENA: usize,
    const STACK_PAR: usize,
    const STACK_VALUE: usize,
    const HEAP_COUNT: usize,
    const HEAP_BYTES: usize,
> Spindle<ARENA, STACK_PAR, STACK_VALUE, HEAP_COUNT, HEAP_BYTES>
{
    pub fn new() -> Self {
        Spindle {
            arena: [0u8; ARENA],
            stack: new_stack(),
            value_stack: Vec::new(),
            heap: HeapStorage::new(),
        }
    }
    async fn run<'src>(
        &mut self,
        code: &'src str,
        natives: &[&dyn NativeFn],
    ) -> Result<(), SpindleError<'src>> {
        super let arena = ArenaStorage::new(&mut self.arena).start();
        super let mut stack = (&mut self.stack as &mut StackStorage).start();
        let program: VmProgram = stack_executor(stack.reborrow(), async |mut spawn| {
            let lexer: Lexer<'src, '_> = Lexer::new(code, arena);
            let program: &'_ Program<'src, '_> = Parser::new(lexer, arena)
                .parse_program(spawn.reborrow())
                .await?;
            let program: VmProgram<'_> = Codegen::new(arena, natives, &program)
                .compile(spawn.reborrow())
                .await?;
            Ok::<VmProgram<'_>, SpindleError<'src>>(program)
        })
        .await??;
        Interp::new(&program, &mut self.value_stack, self.heap.start(), natives).interp(stack.reborrow()).await?;
        Ok(())
    }
}

impl<'src> From<AllocError> for SpindleError<'src> {
    fn from(_: AllocError) -> Self {
        SpindleError::AllocError
    }
}

impl<'src> From<AnnotatedParserError<'src>> for SpindleError<'src> {
    fn from(error: AnnotatedParserError<'src>) -> Self {
        SpindleError::ParserError(error)
    }
}

impl<'src> From<CompileError<'src>> for SpindleError<'src> {
    fn from(error: CompileError<'src>) -> Self {
        SpindleError::CompileError(error)
    }
}

impl<'src> From<InterpError> for SpindleError<'src> {
    fn from(error: InterpError) -> Self {
        SpindleError::InterpError(error)
    }
}