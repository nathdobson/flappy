use crate::compiler::codegen::Codegen;
use crate::compiler::lexer::Lexer;
use crate::compiler::parser::Parser;
use crate::compiler::stack::{StackStorage, new_stack};
use crate::compiler::stack_executor::stack_executor;
use crate::native::{NativeFn, PrintFn};
use crate::vm::VmProgram;
use arena::ArenaStorage;

pub async fn with_test_compile<T>(
    code: &str,
    callback: impl for<'vm> AsyncFnOnce(&'vm VmProgram<'vm>) -> T,
) -> T {
    let capacity: usize = 100000;
    let mut arena_par_slice = vec![0u8; capacity];
    let mut arena_par_storage = ArenaStorage::new(&mut arena_par_slice);
    let arena_par = arena_par_storage.start();
    let mut stack = new_stack::<65536>();
    let stack: &mut StackStorage = &mut stack;
    let mut stack = stack.start();
    let program = stack_executor(stack.reborrow(), async |spawn| {
        Parser::new(Lexer::new(code, arena_par), arena_par)
            .parse_program(spawn)
            .await
            .unwrap()
    })
    .await
    .unwrap();

    let mut arena_vm_slice = vec![0u8; capacity];
    let mut arena_vm_storage = ArenaStorage::new(&mut arena_vm_slice);
    let arena_vm = arena_vm_storage.start();
    let program = stack_executor(stack.reborrow(), async |spawn| {
        Codegen::new(arena_vm, &[&PrintFn], &program)
            .compile(spawn)
            .await
            .unwrap()
    })
    .await
    .unwrap();

    callback(&program).await
}
