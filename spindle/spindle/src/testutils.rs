use crate::compiler::Compiler;
use crate::lexer::Lexer;
use crate::parser::Parser;
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
    let program = Parser::new(Lexer::new(code), arena_par)
        .parse_program()
        .unwrap();
    let mut arena_vm_slice = vec![0u8; capacity];
    let mut arena_vm_storage = ArenaStorage::new(&mut arena_vm_slice);
    let arena_vm = arena_vm_storage.start();
    let program = Compiler::new(arena_vm, &program).compile().unwrap();
    callback(&program).await
}
