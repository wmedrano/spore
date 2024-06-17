use std::collections::HashMap;

use anyhow::Result;

use crate::parser::ast::Ast;

use super::{
    environment::Environment,
    ir::{CodeBlock, IrInstruction},
    types::{proc::bytecode::ByteCodeProc, symbol::Symbol},
};

/// Compiles Asts into `ByteCodeProc` objects.
pub struct Compiler<'a> {
    env: &'a mut Environment,
}

impl<'a> Compiler<'a> {
    /// Create a new compiler.
    pub fn new(env: &'a mut Environment) -> Compiler<'a> {
        Compiler { env }
    }

    /// Compile `ast` onto the current context and return the callable `ByteCodeProc`.
    pub fn compile(self, name: impl Into<Option<String>>, ast: &Ast) -> Result<ByteCodeProc> {
        let mut ir = CodeBlock::with_ast(name.into(), HashMap::new(), std::iter::once(ast))?;
	self.optimize(&mut ir);
        ir.to_bytecode()
    }

    fn optimize(&self, code: &mut CodeBlock) {
	for instruction in code.instructions.iter_mut() {
	    self.inline_deref(instruction);
	}
    }

    /// Mutates an `(deref identifier)` instruction into just its `value`. This affects `IrInstruction::DerefIdentifier`
    /// as well as any possible subexpressions under `instruction`.
    fn inline_deref(&self, instruction: &mut IrInstruction) {
        match instruction {
            IrInstruction::PushProc(proc) => {
                for instruction in proc.instructions.iter_mut() {
                    self.inline_deref(instruction);
                }
            }
            IrInstruction::DerefIdentifier(ident) => {
                if let Some(v) = self.env.get_global(&Symbol::from(ident.as_str())) {
                    *instruction = IrInstruction::PushConst(v);
                }
            }
            IrInstruction::PushConst(_) => (),
            IrInstruction::CallProc { proc, args } => {
                self.inline_deref(proc);
                args.iter_mut().for_each(|arg| self.inline_deref(arg));
            }
            IrInstruction::If {
                pred,
                true_expr,
                false_expr,
            } => {
                self.inline_deref(pred);
                self.inline_deref(true_expr);
                if let Some(expr) = false_expr.as_mut() {
                    self.inline_deref(expr);
                }
            }
            IrInstruction::Define { value, .. } => {
                self.inline_deref(value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::vm::{
        types::{instruction::Instruction, Val},
        Vm,
    };

    use super::*;

    #[test]
    fn lambda_compiles_to_bytecode() {
        let mut env = Vm::new().build_env();
        let ast = Ast::from_sexp_str("(lambda (n) (+ n 1))").unwrap();
        let instructions = Compiler::new(&mut env)
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
                    Instruction::PushVal(Val::NativeProc(_)),
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
        let mut env = Vm::new().build_env();
        let ast = Ast::from_sexp_str("(+ 1 #; \"this is skipped\" #;2 3)").unwrap();
        let bytecode = Compiler::new(&mut env).compile(None, &ast[0]).unwrap();
        assert!(
            matches!(
                bytecode.bytecode.as_slice(),
                [
                    Instruction::PushVal(Val::NativeProc(_)),
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
