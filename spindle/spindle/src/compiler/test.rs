use crate::compiler::ast::CallExpr;
use crate::compiler::codegen::Codegen;
use crate::compiler::lexer::Lexer;
use crate::compiler::parser::Parser;
use crate::testutils::with_test_compile;
use crate::vm::VmBlock;
use crate::vm::VmFunction;
use crate::vm::VmInstr;
use crate::vm::VmOperator;
use crate::vm::VmProgram;
use crate::vm::VmFunctionName;
use arena::ArenaStorage;
use std::assert_matches;
#[tokio::test]
async fn test_parser() {
    use itertools::Itertools;

    let code = r#"
        let foo = 2 + 2;
        print(foo);
        print(foo);
    "#;
    with_test_compile(code, async |program| {
        assert_matches!(
            program,
            VmProgram {
                functions: [VmFunction {
                    blocks: [VmBlock {
                        instrs: [
                            VmInstr::Integer(2),
                            VmInstr::Integer(2),
                            VmInstr::Binop(VmOperator::Plus),
                            VmInstr::Load(0),
                            VmInstr::Call(VmFunctionName::Native(0), 1),
                            VmInstr::Pop,
                            VmInstr::Load(0),
                            VmInstr::Call(VmFunctionName::Native(0), 1),
                            VmInstr::Pop,
                            VmInstr::Pop,
                        ],
                        term: _
                    }]
                }]
            }
        );
    })
    .await;
}
