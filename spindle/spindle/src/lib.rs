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
    super_let,
    try_blocks
)]
#![feature(unsize)]
#![feature(coerce_unsized)]
#![feature(pin_coerce_unsized_trait)]
#![feature(debug_closure_helpers)]
#![feature(ptr_alignment_type)]
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
use crate::compiler::stack::{Stack, StackStorage, new_stack};
use crate::compiler::stack_executor::stack_executor;
use crate::interp::Interp;
use crate::interp::error::InterpError;
use crate::interp::heap::{Heap, HeapStorage};
use crate::interp::value::Value;
use crate::native::{NativeFn, PrintFn};
use crate::vm::VmProgram;
use arena::Arena;
use core::alloc::AllocError;
use heapless::{Vec, VecView};

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
    const STACK: usize,
    const STACK_VALUE: usize,
    const HEAP_COUNT: usize,
    const HEAP_BYTES: usize,
> {
    arena: [u8; ARENA],
    stack: StackStorage<[u8; STACK]>,
    value_stack: Vec<Value, STACK_VALUE>,
    heap: HeapStorage<HEAP_COUNT, HEAP_BYTES>,
}

#[derive(Clone)]
pub struct SpindleOptions {
    pub compaction_ratio: f32,
}

impl Default for SpindleOptions {
    fn default() -> Self {
        SpindleOptions {
            compaction_ratio: 1.0,
        }
    }
}
pub struct SpindleMut<'vm> {
    arena: &'vm Arena,
    stack: Stack<'vm>,
    value_stack: &'vm mut VecView<Value>,
    heap: Heap<'vm>,
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
    pub fn start(&'_ mut self, options: SpindleOptions) -> Result<SpindleMut<'_>, AllocError> {
        Ok(SpindleMut {
            arena: Arena::new(&mut self.arena)?,
            stack: (&mut self.stack as &mut StackStorage).start(),
            value_stack: &mut self.value_stack,
            heap: self.heap.start(options.compaction_ratio),
        })
    }
    pub async fn run<'src>(
        &mut self,
        options: SpindleOptions,
        code: &'src str,
        natives: &[&dyn NativeFn],
    ) -> Result<(), SpindleError<'src>> {
        self.start(options)?.run(code, natives).await
    }
    pub async fn parse<'vm, 'src: 'vm>(
        &'vm mut self,
        options: SpindleOptions,
        code: &'src str,
    ) -> Result<&'vm Program<'src, 'vm>, SpindleError<'src>> {
        self.start(options)?.parse(code).await
    }
}

impl<'vm> SpindleMut<'vm> {
    async fn run<'src>(
        mut self,
        code: &'src str,
        natives: &[&dyn NativeFn],
    ) -> Result<(), SpindleError<'src>> {
        let program = self.compile(code, natives).await?;
        Ok(Interp::new(&program, self.value_stack, self.heap, natives)
            .interp(self.stack.reborrow())
            .await?)
    }
    async fn compile<'src: 'vm>(
        &mut self,
        code: &'src str,
        natives: &'vm [&'vm dyn NativeFn],
    ) -> Result<VmProgram<'vm>, SpindleError<'src>> {
        let program = self.parse(code).await?;
        Ok(stack_executor(self.stack.reborrow(), async |mut spawn| {
            let program: VmProgram<'_> = Codegen::new(self.arena, natives, &program)
                .compile(spawn.reborrow())
                .await?;
            Ok::<_, SpindleError>(program)
        })
        .await??)
    }
    async fn parse<'src: 'vm>(
        &mut self,
        code: &'src str,
    ) -> Result<&'vm Program<'src, 'vm>, SpindleError<'src>> {
        Ok(stack_executor(self.stack.reborrow(), async |mut spawn| {
            let lexer: Lexer<'src, '_> = Lexer::new(code, self.arena);
            let program: &'_ Program<'src, '_> = Parser::new(lexer, self.arena)
                .parse_program(spawn.reborrow())
                .await?;
            Ok::<_, SpindleError>(program)
        })
        .await??)
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
