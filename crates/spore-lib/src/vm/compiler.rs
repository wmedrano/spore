use std::collections::HashMap;

use anyhow::Result;

use crate::parser::ast::Ast;

use super::{ir::CodeBlock, types::proc::bytecode::ByteCodeProc};

/// Compiles Asts into `ByteCodeProc` objects.
pub struct Compiler;

impl Compiler {
    /// Create a new compiler.
    pub fn new() -> Compiler {
        Compiler
    }

    /// Compile `ast` onto the current context and return the callable `ByteCodeProc`.
    pub fn compile(self, name: impl Into<Option<String>>, ast: &Ast) -> Result<ByteCodeProc> {
        let ir = CodeBlock::with_ast(name.into(), HashMap::new(), std::iter::once(ast))?;
        ir.to_bytecode()
    }
}

#[cfg(test)]
mod tests {
    use crate::vm::types::{instruction::Instruction, Val};

    use super::*;

    #[test]
    fn lambda_compiles_to_bytecode() {
        let ast = Ast::from_sexp_str("(lambda (n) (+ n 1))").unwrap();
        let instructions = Compiler::new()
            .compile(None, &ast[0])
            .unwrap()
            .bytecode
            .into_iter()
            .next()
            .unwrap();
        let bytecode = match instructions {
            Instruction::PushVal(Val::ByteCodeProc(proc)) => proc,
            v => panic!("Expected PushVal(ByteCodeProc) but found {v:?}"),
        };
        assert!(
            matches!(
                bytecode.bytecode.as_slice(),
                [
                    Instruction::GetVal(_),
                    Instruction::GetArg(0),
                    Instruction::PushVal(Val::Int(1)),
                    Instruction::Eval(3),
                ]
            ),
            "Found {:?}",
            bytecode.bytecode
        );
    }

    #[test]
    fn comment_next_datum_skips_datum() {
        let ast = Ast::from_sexp_str("(+ 1 #; \"this is skipped\" #;2 3)").unwrap();
        let bytecode = Compiler::new().compile(None, &ast[0]).unwrap();
        assert!(
            matches!(
                bytecode.bytecode.as_slice(),
                [
                    Instruction::GetVal(_),
                    Instruction::PushVal(Val::Int(1)),
                    Instruction::PushVal(Val::Int(3)),
                    Instruction::Eval(3),
                ]
            ),
            "Found {:?}",
            bytecode.bytecode
        );
    }
}
