use crate::compiler::Compiler;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::vm::{
    VmCallExpr, VmExpr, VmExprStmt, VmFunction, VmFunctionName, VmLetStmt, VmOperator,
    VmOperatorExpr, VmProgram, VmStmt,
};
use arena::ArenaStorage;

#[test]
fn test_parser() {
    use itertools::Itertools;

    let code = r#"
        let foo = 2 + 2;
        print(foo);
    "#;
    const CAP: usize = 100000;
    let mut arena_par_storage = Box::new(ArenaStorage::<CAP>::new());
    let arena_par = arena_par_storage.start();
    let program = Parser::new(Lexer::new(code), arena_par)
        .parse_program()
        .unwrap();
    let mut arena_vm_storage = Box::new(ArenaStorage::<CAP>::new());
    let arena_vm = arena_vm_storage.start();
    let program = Compiler::new(arena_vm, &program).compile().unwrap();
    assert_eq!(
        program,
        VmProgram {
            functions: arena_vm
                .alloc_vec([VmFunction {
                    stmt: arena_vm
                        .alloc_box(VmStmt::LetStmt(VmLetStmt {
                            expr: arena_vm
                                .alloc_box(VmExpr::Operator(VmOperatorExpr {
                                    operator: VmOperator::Plus,
                                    left: arena_vm.alloc_box(VmExpr::Number(2)).unwrap(),
                                    right: arena_vm.alloc_box(VmExpr::Number(2)).unwrap(),
                                }))
                                .unwrap(),
                            next: arena_vm
                                .alloc_box(VmStmt::ExprStmt(VmExprStmt {
                                    expr: arena_vm
                                        .alloc_box(VmExpr::Call(VmCallExpr {
                                            function: VmFunctionName::Print,
                                            args: arena_vm
                                                .alloc_vec([arena_vm
                                                    .alloc_box(VmExpr::Var(0))
                                                    .unwrap()])
                                                .unwrap(),
                                        }))
                                        .unwrap(),
                                    next: arena_vm.alloc_box(VmStmt::Noop).unwrap()
                                }))
                                .unwrap()
                        }))
                        .unwrap()
                }])
                .unwrap()
        }
    );
}
