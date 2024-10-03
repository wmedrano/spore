use std::{collections::HashSet, sync::Arc};

use bumpalo::Bump;
use compact_str::{CompactString, ToCompactString};
use ir::{Constant, Ir, IrReturnType};

use crate::{
    error::CompileError,
    parser::{ast::Node, span::Span},
    val::{ByteCode, Instruction, UnsafeVal},
    Settings, Vm,
};

mod ir;

type BumpVec<'a, T> = bumpalo::collections::Vec<'a, T>;
type Result<T> = std::result::Result<T, CompileError>;

pub struct Compiler<'a> {
    vm: &'a mut Vm,
    arena: &'a Bump,
    source: Option<Arc<str>>,
    settings: Settings,
    function_name: Option<CompactString>,
    arguments: BumpVec<'a, CompactString>,
    local_bindings: BumpVec<'a, CompactString>,
    local_space_required: usize,
    instructions: BumpVec<'a, Instruction>,
    instruction_source: BumpVec<'a, Span>,
}

#[derive(Copy, Clone, PartialEq)]
enum CompilerContext {
    Module,
    Subexpression,
}

#[derive(Copy, Clone, PartialEq)]
enum CompileManyBehavior {
    /// Keep all expressions on the stack. Requires one less instruction.
    KeepAll,
    /// Only keep the last expression on the stack. All other expressions are discarded.
    KeepSingleReturn,
}

impl<'a> Compiler<'a> {
    pub fn compile(vm: &'a mut Vm, input_source: &str, arena: &Bump) -> Result<ByteCode> {
        let settings = vm.settings;
        let source = settings.enable_source_maps.then(|| input_source.into());
        let mut compiler = Compiler {
            vm,
            arena,
            source: source.clone(),
            settings,
            function_name: None,
            arguments: BumpVec::new_in(arena),
            local_bindings: BumpVec::new_in(arena),
            local_space_required: 0,
            instructions: BumpVec::new_in(arena),
            instruction_source: BumpVec::new_in(arena),
        };
        compiler.compile_impl(input_source, CompilerContext::Module)?;
        Ok(ByteCode {
            name: "".into(),
            arg_count: 0,
            local_bindings: compiler.local_space_required,
            instructions: compiler.instructions.into_bump_slice().into(),
            source,
            instruction_source: compiler.instruction_source.into_bump_slice().into(),
        })
    }

    fn compile_impl(&mut self, input_source: &str, ctx: CompilerContext) -> Result<()> {
        for node_or_err in Node::parse(input_source) {
            let node = node_or_err.map_err(CompileError::AstError)?;
            let ir = Ir::new(self.arena, input_source, &node)?;
            self.compile_one(&ir, ctx)?;
        }
        Ok(())
    }

    fn arg_idx(&self, symbol: &str) -> Option<usize> {
        for (idx, sym) in self.local_bindings.iter().enumerate().rev() {
            if sym == symbol {
                return Some(idx + self.arguments.len());
            }
        }
        for (idx, sym) in self.arguments.iter().enumerate().rev() {
            if sym == symbol {
                return Some(idx);
            }
        }
        None
    }

    fn compile_one(&mut self, ir: &Ir, ctx: CompilerContext) -> Result<()> {
        match ir {
            Ir::Constant(span, const_val) => self.compile_one_constant(*span, const_val)?,
            Ir::Deref(span, ident) => self.compile_one_deref(*span, ident)?,
            Ir::FunctionCall {
                span,
                function,
                args,
            } => self.compile_one_function_call(*span, function, args)?,
            Ir::Define {
                span,
                identifier,
                expr,
            } => self.compile_one_define(ctx, *span, identifier, expr)?,
            Ir::If {
                span,
                predicate,
                true_expr,
                false_expr,
            } => self.compile_one_if(*span, predicate, true_expr, *false_expr)?,
            Ir::Lambda {
                span,
                name,
                args,
                expressions,
            } => self.compile_one_lambda(*span, *name, args, expressions)?,
            Ir::Let {
                span,
                bindings,
                expressions,
            } => self.compile_one_let(*span, bindings.as_slice(), expressions)?,
            Ir::Return { expr } => self.compile_one_return(expr)?,
        };
        Ok(())
    }

    fn compile_many(&mut self, expressions: &[Ir], behavior: CompileManyBehavior) -> Result<()> {
        match expressions {
            [] => {
                if behavior == CompileManyBehavior::KeepSingleReturn {
                    self.compile_one_constant(Span::new(0, 0), &Constant::Void)?;
                }
            }
            [exprs @ .., last] => {
                for expr in exprs {
                    self.compile_one(expr, CompilerContext::Subexpression)?;
                }
                if !exprs.is_empty() && behavior == CompileManyBehavior::KeepSingleReturn {
                    self.instruction_source.push(Span::new(0, 0));
                    self.instructions.push(Instruction::Pop(exprs.len()));
                }
                self.compile_one(last, CompilerContext::Subexpression)?;
            }
        }
        Ok(())
    }

    fn compile_one_let(
        &mut self,
        span: Span,
        bindings: &[(&str, Ir)],
        expressions: &[Ir],
    ) -> Result<()> {
        for (binding, expr) in bindings {
            self.compile_one(expr, CompilerContext::Subexpression)?;
            self.local_bindings.push(binding.to_compact_string());
            // TODO: Add let binding span to `bindings`.
            self.instruction_source.push(span);
            self.instructions
                .push(Instruction::BindArg(self.arg_idx(binding).unwrap()));
        }
        self.compile_many(expressions, CompileManyBehavior::KeepSingleReturn)?;
        self.local_space_required = self.local_space_required.max(self.local_bindings.len());
        for _ in bindings {
            self.local_bindings.pop().unwrap();
        }
        Ok(())
    }

    fn compile_one_constant(&mut self, span: Span, val: &Constant) -> Result<()> {
        let instruction = match val {
            Constant::Void => Instruction::PushConst(().into()),
            Constant::Bool(x) => Instruction::PushConst((*x).into()),
            Constant::Int(x) => Instruction::PushConst((*x).into()),
            Constant::Float(x) => Instruction::PushConst((*x).into()),
            Constant::String(x) => {
                Instruction::PushConst(UnsafeVal::String(self.vm.objects.insert_string(x.clone())))
            }
        };
        self.instruction_source.push(span);
        self.instructions.push(instruction);
        Ok(())
    }

    fn compile_one_deref(&mut self, span: Span, ident: &str) -> Result<()> {
        match self.arg_idx(ident) {
            Some(idx) => {
                self.instruction_source.push(span);
                self.instructions.push(Instruction::GetArg(idx))
            }
            None if self
                .function_name
                .as_ref()
                .map(|s| s.as_str() == ident)
                .unwrap_or(false) =>
            {
                self.instruction_source.push(span);
                self.instructions.push(Instruction::PushCurrentFunction)
            }
            None => {
                let maybe_inlined_val = self
                    .settings
                    .enable_aggressive_inline
                    .then(|| {
                        let interned_ident = self.vm.get_or_create_symbol(ident);
                        self.vm.values.get(&interned_ident)
                    })
                    .flatten()
                    .map(|c| Instruction::PushConst(*c));
                let interned_ident = self.vm.get_or_create_symbol(ident);
                let instruction = maybe_inlined_val.unwrap_or(Instruction::Deref(interned_ident));
                self.instruction_source.push(span);
                self.instructions.push(instruction)
            }
        };
        Ok(())
    }

    fn compile_one_function_call(&mut self, span: Span, function: &Ir, args: &[Ir]) -> Result<()> {
        let maybe_native_function = self
            .settings
            .enable_aggressive_inline
            .then(|| match function {
                Ir::Deref(_, ident) => {
                    let interned_ident = self.vm.get_or_create_symbol(ident);
                    match self.vm.values.get(&interned_ident) {
                        Some(UnsafeVal::NativeFunction(func)) => Some(*func),
                        _ => None,
                    }
                }
                _ => None,
            })
            .flatten();
        if maybe_native_function.is_none() {
            self.compile_one(function, CompilerContext::Subexpression)?;
        }
        self.compile_many(args, CompileManyBehavior::KeepAll)?;
        match maybe_native_function {
            Some(func) => {
                self.instruction_source.push(span);
                self.instructions.push(Instruction::EvalNative {
                    func,
                    arg_count: args.len(),
                })
            }
            None => {
                self.instruction_source.push(span);
                self.instructions.push(Instruction::Eval(args.len() + 1))
            }
        }
        Ok(())
    }

    fn compile_one_define(
        &mut self,
        ctx: CompilerContext,
        span: Span,
        ident: &str,
        expr: &Ir,
    ) -> Result<()> {
        if ctx != CompilerContext::Module {
            return Err(CompileError::DefineNotAllowed);
        }
        if expr.return_type() != IrReturnType::Value {
            return Err(CompileError::ExpectedExpression { context: "define" });
        }
        self.compile_one(expr, CompilerContext::Subexpression)?;
        self.instruction_source.push(span);
        let interned_ident = self.vm.get_or_create_symbol(ident);
        self.instructions.push(Instruction::Define(interned_ident));
        Ok(())
    }

    fn compile_one_if(
        &mut self,
        span: Span,
        predicate: &Ir,
        true_expr: &Ir,
        false_expr: Option<&Ir>,
    ) -> Result<()> {
        if predicate.return_type() != IrReturnType::Value {
            return Err(CompileError::ExpectedExpression {
                context: "if predicate",
            });
        }
        self.compile_one(predicate, CompilerContext::Subexpression)?;
        // Placeholder for jump instruction.
        let true_jump_idx = self.instructions.len();
        self.instruction_source.push(span);
        self.instructions.push(Instruction::PushConst(().into()));
        match false_expr {
            Some(expr) => self.compile_one(expr, CompilerContext::Subexpression)?,
            None => {
                self.instruction_source.push(Span::new(0, 0));
                self.instructions.push(Instruction::PushConst(().into()))
            }
        }
        let false_jump_idx = self.instructions.len();
        self.instruction_source.push(span);
        self.instructions.push(Instruction::PushConst(().into()));
        self.compile_one(true_expr, CompilerContext::Subexpression)?;
        self.instructions[true_jump_idx] = Instruction::JumpIf(false_jump_idx - true_jump_idx);
        self.instruction_source[false_jump_idx] = span;
        self.instructions[false_jump_idx] =
            Instruction::Jump(self.instructions.len() - false_jump_idx - 1);
        Ok(())
    }

    fn compile_one_lambda(
        &mut self,
        span: Span,
        name: Option<&str>,
        args: &[&str],
        expressions: &[Ir],
    ) -> Result<()> {
        if expressions.is_empty() {
            return Err(CompileError::ExpectedExpression {
                context: "lambda definition expressions",
            });
        }
        let mut arguments_vec = BumpVec::new_in(self.arena);
        for arg in args.iter() {
            arguments_vec.push(CompactString::new(arg));
        }
        let mut lambda_compiler = Compiler {
            vm: self.vm,
            arena: self.arena,
            source: self.source.clone(),
            settings: self.settings,
            function_name: name.map(CompactString::new),
            arguments: arguments_vec,
            local_bindings: BumpVec::new_in(self.arena),
            local_space_required: 0,
            instructions: BumpVec::new_in(self.arena),
            instruction_source: BumpVec::new_in(self.arena),
        };
        if let Some(dupe) = find_duplicate(&lambda_compiler.arguments) {
            return Err(CompileError::ArgumentDefinedMultipleTimes(dupe));
        }
        // We keep all since its faster.
        lambda_compiler.compile_many(expressions, CompileManyBehavior::KeepAll)?;
        let bytecode = ByteCode {
            name: name.unwrap_or("").into(),
            arg_count: args.len(),
            local_bindings: lambda_compiler.local_space_required,
            instructions: lambda_compiler.instructions.into_bump_slice().into(),
            source: lambda_compiler.source,
            instruction_source: lambda_compiler.instruction_source.into_bump_slice().into(),
        };
        let lambda_val =
            UnsafeVal::ByteCodeFunction(lambda_compiler.vm.objects.insert_bytecode(bytecode));
        self.instruction_source.push(span);
        self.instructions.push(Instruction::PushConst(lambda_val));
        Ok(())
    }

    fn compile_one_return(&mut self, expr: &Ir) -> Result<()> {
        self.compile_one(expr, CompilerContext::Subexpression)?;
        self.instruction_source.push(Span::new(0, 0));
        self.instructions.push(Instruction::Return);
        Ok(())
    }
}

fn find_duplicate(vec: &[CompactString]) -> Option<CompactString> {
    // Don't bother with a memory allocation for small slices.
    if vec.len() < 5 {
        for (start, a) in vec.iter().enumerate() {
            for b in vec.iter().skip(start + 1) {
                if a == b {
                    return Some(a.clone());
                }
            }
        }
        return None;
    }
    let mut found = HashSet::with_capacity(vec.len());
    for s in vec.iter() {
        if found.contains(s) {
            return Some(s.clone());
        }
        found.insert(s);
    }
    None
}

#[cfg(test)]
mod tests {
    use crate::parser::ast::AstParseError;

    use super::*;

    #[test]
    fn empty_expression_is_empty() {
        let mut vm = Vm::default();
        let actual = Compiler::compile(&mut vm, "", &Bump::new()).unwrap();
        assert_eq!(
            actual,
            ByteCode {
                name: "".into(),
                arg_count: 0,
                local_bindings: 0,
                instructions: vec![].into(),
                source: Some("".into()),
                instruction_source: vec![].into(),
            }
        );
    }

    #[test]
    fn ast_error_is_returned() {
        let mut vm = Vm::default();
        assert_eq!(
            Compiler::compile(&mut vm, ")", &Bump::new()).unwrap_err(),
            CompileError::AstError(AstParseError::UnexpectedCloseParen)
        );
    }

    ////////////////////////////////////////////////////////////////////////////////
    // BEGIN: Constants
    ////////////////////////////////////////////////////////////////////////////////

    #[test]
    fn literal_value_produces_single_push_const() {
        let mut vm = Vm::default();
        assert_eq!(
            Compiler::compile(&mut vm, "true", &Bump::new()).unwrap(),
            ByteCode {
                name: "".into(),
                arg_count: 0,
                local_bindings: 0,
                instructions: vec![Instruction::PushConst(true.into())].into(),
                source: Some("true".into()),
                instruction_source: vec![Span::new(0, 4)].into(),
            }
        );
        assert_eq!(
            Compiler::compile(&mut vm, "1", &Bump::new()).unwrap(),
            ByteCode {
                name: "".into(),
                arg_count: 0,
                local_bindings: 0,
                instructions: vec![Instruction::PushConst(1.into())].into(),
                source: Some("1".into()),
                instruction_source: vec![Span::new(0, 1)].into(),
            }
        );
        assert_eq!(
            Compiler::compile(&mut vm, "1.0", &Bump::new()).unwrap(),
            ByteCode {
                name: "".into(),
                arg_count: 0,
                local_bindings: 0,
                instructions: vec![Instruction::PushConst(1.0.into())].into(),
                source: Some("1.0".into()),
                instruction_source: vec![Span::new(0, 3)].into(),
            }
        );
        let got = Compiler::compile(&mut vm, "\"string\"", &Bump::new()).unwrap();
        assert_eq!(
            got,
            ByteCode {
                name: "".into(),
                arg_count: 0,
                local_bindings: 0,
                // Warning: Checking for 0 is brittle as it involves knowing the internal details of
                // the id system.
                instructions: vec![got.instructions[0].clone()].into(),
                source: Some("\"string\"".into()),
                instruction_source: vec![Span::new(0, 8)].into(),
            }
        );
        assert_eq!(
            vm.eval_str("\"string\"")
                .unwrap()
                .try_str()
                .unwrap()
                .to_string(),
            "string"
        );
    }

    ////////////////////////////////////////////////////////////////////////////////
    // BEGIN: Deref
    ////////////////////////////////////////////////////////////////////////////////

    #[test]
    fn single_identifier_is_dereffed() {
        let mut vm = Vm::default();
        let actual = Compiler::compile(&mut vm, "my-variable", &Bump::new()).unwrap();
        assert_eq!(
            actual,
            ByteCode {
                name: "".into(),
                arg_count: 0,
                local_bindings: 0,
                instructions: vec![Instruction::Deref(vm.get_symbol("my-variable").unwrap())]
                    .into(),
                source: Some("my-variable".into()),
                instruction_source: vec![Span::new(0, 11)].into(),
            }
        );
    }

    #[test]
    fn single_identifier_with_aggressive_inline_is_push_const() {
        let mut vm = Vm::new(Settings {
            enable_aggressive_inline: true,
            enable_source_maps: false,
        });
        let actual = Compiler::compile(&mut vm, "+", &Bump::new()).unwrap();
        assert_eq!(
            actual,
            ByteCode {
                name: "".into(),
                arg_count: 0,
                local_bindings: 0,
                instructions: vec![Instruction::PushConst(
                    UnsafeVal::NativeFunction(crate::builtins::numbers::add).into()
                )]
                .into(),
                source: None,
                instruction_source: vec![Span::new(0, 1)].into(),
            }
        );
    }

    #[test]
    fn aggressive_inline_with_builtin_function_inlines_function_value() {
        let mut vm = Vm::new(Settings {
            enable_aggressive_inline: true,
            enable_source_maps: false,
        });
        let actual = Compiler::compile(&mut vm, "(+ 1 2)", &Bump::new()).unwrap();
        assert_eq!(
            actual,
            ByteCode {
                name: "".into(),
                arg_count: 0,
                local_bindings: 0,
                instructions: vec![
                    Instruction::PushConst(1.into()),
                    Instruction::PushConst(2.into()),
                    Instruction::EvalNative {
                        func: crate::builtins::numbers::add,
                        arg_count: 2
                    },
                ]
                .into(),
                source: None,
                instruction_source: vec![
                    Span { start: 3, end: 4 },
                    Span { start: 5, end: 6 },
                    Span { start: 0, end: 7 }
                ]
                .into()
            }
        );
    }

    #[test]
    fn aggressive_inline_with_nonexistant_function_falls_back_to_deref() {
        let mut vm = Vm::new(Settings {
            enable_aggressive_inline: true,
            enable_source_maps: false,
        });
        let actual = Compiler::compile(&mut vm, "(does-not-exist 1 2)", &Bump::new()).unwrap();
        assert_eq!(
            actual,
            ByteCode {
                name: "".into(),
                arg_count: 0,
                local_bindings: 0,
                instructions: vec![
                    Instruction::Deref(vm.get_symbol("does-not-exist").unwrap()),
                    Instruction::PushConst(1.into()),
                    Instruction::PushConst(2.into()),
                    Instruction::Eval(3),
                ]
                .into(),
                source: None,
                instruction_source: vec![
                    Span { start: 1, end: 15 },
                    Span { start: 16, end: 17 },
                    Span { start: 18, end: 19 },
                    Span { start: 0, end: 20 }
                ]
                .into()
            }
        );
        let actual = Compiler::compile(&mut vm, "((get-fn) 1 2)", &Bump::new()).unwrap();
        assert_eq!(
            actual,
            ByteCode {
                name: "".into(),
                arg_count: 0,
                local_bindings: 0,
                instructions: vec![
                    Instruction::Deref(vm.get_symbol("get-fn").unwrap()),
                    Instruction::Eval(1),
                    Instruction::PushConst(1.into()),
                    Instruction::PushConst(2.into()),
                    Instruction::Eval(3),
                ]
                .into(),
                source: None,
                instruction_source: vec![
                    Span { start: 2, end: 8 },
                    Span { start: 1, end: 9 },
                    Span { start: 10, end: 11 },
                    Span { start: 12, end: 13 },
                    Span { start: 0, end: 14 },
                ]
                .into()
            }
        );
    }

    ////////////////////////////////////////////////////////////////////////////////
    // BEGIN: Function Call
    ////////////////////////////////////////////////////////////////////////////////

    #[test]
    fn function_call_with_no_args_evalutes_function() {
        let mut vm = Vm::default();
        let actual = Compiler::compile(&mut vm, "(+)", &Bump::new()).unwrap();
        assert_eq!(
            actual,
            ByteCode {
                name: "".into(),
                arg_count: 0,
                local_bindings: 0,
                instructions: vec![
                    Instruction::Deref(vm.get_symbol("+").unwrap()),
                    Instruction::Eval(1)
                ]
                .into(),
                source: Some("(+)".into()),
                instruction_source: vec![Span { start: 1, end: 2 }, Span { start: 0, end: 3 }]
                    .into(),
            }
        );
    }

    #[test]
    fn function_call_args_evalutes_function_on_args() {
        let mut vm = Vm::default();
        let actual = Compiler::compile(&mut vm, "(+ 1 2)", &Bump::new()).unwrap();
        assert_eq!(
            actual,
            ByteCode {
                name: "".into(),
                arg_count: 0,
                local_bindings: 0,
                instructions: vec![
                    Instruction::Deref(vm.get_symbol("+").unwrap()),
                    Instruction::PushConst(1.into()),
                    Instruction::PushConst(2.into()),
                    Instruction::Eval(3),
                ]
                .into(),
                source: Some("(+ 1 2)".into()),
                instruction_source: [
                    Span { start: 1, end: 2 },
                    Span { start: 3, end: 4 },
                    Span { start: 5, end: 6 },
                    Span { start: 0, end: 7 }
                ]
                .into()
            }
        );
    }

    #[test]
    fn multiple_expressions_are_evaluated_in_order() {
        let mut vm = Vm::default();
        let src = "(+ 1 2) (+ 3 4)";
        let actual = Compiler::compile(&mut vm, src, &Bump::new()).unwrap();
        assert_eq!(
            actual,
            ByteCode {
                name: "".into(),
                arg_count: 0,
                local_bindings: 0,
                instructions: vec![
                    Instruction::Deref(vm.get_symbol("+").unwrap()),
                    Instruction::PushConst(1.into()),
                    Instruction::PushConst(2.into()),
                    Instruction::Eval(3),
                    Instruction::Deref(vm.get_symbol("+").unwrap()),
                    Instruction::PushConst(3.into()),
                    Instruction::PushConst(4.into()),
                    Instruction::Eval(3),
                ]
                .into(),
                source: Some(src.into()),
                instruction_source: vec![
                    Span { start: 1, end: 2 },
                    Span { start: 3, end: 4 },
                    Span { start: 5, end: 6 },
                    Span { start: 0, end: 7 },
                    Span { start: 9, end: 10 },
                    Span { start: 11, end: 12 },
                    Span { start: 13, end: 14 },
                    Span { start: 8, end: 15 }
                ]
                .into(),
            }
        );
    }

    #[test]
    fn nested_expressions_are_evaluated() {
        let mut vm = Vm::default();
        let src = "(+ 1 2 (+ 3 4))";
        let actual = Compiler::compile(&mut vm, src, &Bump::new()).unwrap();
        assert_eq!(
            actual,
            ByteCode {
                name: "".into(),
                arg_count: 0,
                local_bindings: 0,
                instructions: vec![
                    Instruction::Deref(vm.get_symbol("+").unwrap()),
                    Instruction::PushConst(1.into()),
                    Instruction::PushConst(2.into()),
                    Instruction::Deref(vm.get_symbol("+").unwrap()),
                    Instruction::PushConst(3.into()),
                    Instruction::PushConst(4.into()),
                    Instruction::Eval(3),
                    Instruction::Eval(4),
                ]
                .into(),
                source: Some(src.into()),
                instruction_source: vec![
                    Span { start: 1, end: 2 },
                    Span { start: 3, end: 4 },
                    Span { start: 5, end: 6 },
                    Span { start: 8, end: 9 },
                    Span { start: 10, end: 11 },
                    Span { start: 12, end: 13 },
                    Span { start: 7, end: 14 },
                    Span { start: 0, end: 15 }
                ]
                .into(),
            }
        );
    }

    #[test]
    fn define_in_function_args_produces_error() {
        let mut vm = Vm::default();
        let actual = Compiler::compile(&mut vm, "(+ 1 (define x 12))", &Bump::new()).unwrap_err();
        assert_eq!(actual, CompileError::DefineNotAllowed);
    }

    #[test]
    fn define_in_function_call_produces_error() {
        let mut vm = Vm::default();
        let actual = Compiler::compile(&mut vm, "((define x 12))", &Bump::new()).unwrap_err();
        assert_eq!(actual, CompileError::DefineNotAllowed);
    }

    ////////////////////////////////////////////////////////////////////////////////
    // BEGIN: Define
    ////////////////////////////////////////////////////////////////////////////////

    #[test]
    fn define_defines_a_new_value() {
        let mut vm = Vm::default();
        let actual = Compiler::compile(&mut vm, "(define x 12)", &Bump::new()).unwrap();
        assert_eq!(
            actual,
            ByteCode {
                name: "".into(),
                arg_count: 0,
                local_bindings: 0,
                instructions: vec![
                    Instruction::PushConst(12.into()),
                    Instruction::Define(vm.get_symbol("x").unwrap()),
                ]
                .into(),
                source: Some("(define x 12)".into()),
                instruction_source: vec![Span { start: 10, end: 12 }, Span { start: 0, end: 13 }]
                    .into(),
            }
        );
    }

    fn instruction_push_const_to_bytecode<'a>(
        instruction: &Instruction,
        vm: &'a Vm,
    ) -> Option<&'a ByteCode> {
        match instruction {
            Instruction::PushConst(UnsafeVal::ByteCodeFunction(id)) => {
                Some(vm.objects.get_bytecode(*id).unwrap())
            }
            _ => None,
        }
    }

    #[test]
    fn define_with_list_identifier_produces_lambda() {
        let mut vm = Vm::default();
        let src = "(define (foo a b) (+ a b))";
        let actual = Compiler::compile(&mut vm, src, &Bump::new()).unwrap();
        assert_eq!(
            actual,
            ByteCode {
                name: "".into(),
                arg_count: 0,
                local_bindings: 0,
                instructions: vec![
                    Instruction::PushConst(
                        UnsafeVal::ByteCodeFunction(
                            vm.objects.get_or_insert_bytecode_slow(ByteCode {
                                name: "foo".into(),
                                arg_count: 2,
                                local_bindings: 0,
                                instructions: vec![
                                    Instruction::Deref(vm.get_symbol("+").unwrap()),
                                    Instruction::GetArg(0),
                                    Instruction::GetArg(1),
                                    Instruction::Eval(3),
                                ]
                                .into(),
                                source: Some(src.into()),
                                instruction_source: vec![
                                    Span { start: 19, end: 20 },
                                    Span { start: 21, end: 22 },
                                    Span { start: 23, end: 24 },
                                    Span { start: 18, end: 25 }
                                ]
                                .into(),
                            })
                        )
                        .into()
                    ),
                    Instruction::Define(vm.get_symbol("foo").unwrap()),
                ]
                .into(),
                source: Some(src.into()),
                instruction_source: vec![Span { start: 8, end: 25 }, Span { start: 9, end: 12 }]
                    .into(),
            },
            "Inner bytecode is: {:?}",
            instruction_push_const_to_bytecode(&actual.instructions[0], &vm)
        );
    }

    #[test]
    fn define_with_subexpression_evaluates_subexpr() {
        let mut vm = Vm::default();
        let actual = Compiler::compile(&mut vm, "(define x (+ 1 2))", &Bump::new()).unwrap();
        assert_eq!(
            actual,
            ByteCode {
                name: "".into(),
                arg_count: 0,
                local_bindings: 0,
                instructions: vec![
                    Instruction::Deref(vm.get_symbol("+").unwrap()),
                    Instruction::PushConst(1.into()),
                    Instruction::PushConst(2.into()),
                    Instruction::Eval(3),
                    Instruction::Define(vm.get_symbol("x").unwrap()),
                ]
                .into(),
                source: Some("(define x (+ 1 2))".into()),
                instruction_source: [
                    Span { start: 11, end: 12 },
                    Span { start: 13, end: 14 },
                    Span { start: 15, end: 16 },
                    Span { start: 10, end: 17 },
                    Span { start: 0, end: 18 }
                ]
                .into()
            }
        );
    }

    #[test]
    fn define_in_define_expr_produces_error() {
        let mut vm = Vm::default();
        let actual =
            Compiler::compile(&mut vm, "(define y (define x 12))", &Bump::new()).unwrap_err();
        assert_eq!(actual, CompileError::DefineNotAllowed);
    }

    ////////////////////////////////////////////////////////////////////////////////
    // BEGIN: If
    ////////////////////////////////////////////////////////////////////////////////

    #[test]
    fn if_expression_produces_branching_instructions() {
        let mut vm = Vm::default();
        let src = "(if (< 1 2) (+ 3 4 5) (+ 6 7 8 9))";
        assert_eq!(
            Compiler::compile(&mut vm, src, &Bump::new()).unwrap(),
            ByteCode {
                name: "".into(),
                arg_count: 0,
                local_bindings: 0,
                instructions: vec![
                    Instruction::Deref(vm.get_symbol("<").unwrap()),
                    Instruction::PushConst(1.into()),
                    Instruction::PushConst(2.into()),
                    Instruction::Eval(3),
                    Instruction::JumpIf(7),
                    Instruction::Deref(vm.get_symbol("+").unwrap()),
                    Instruction::PushConst(6.into()),
                    Instruction::PushConst(7.into()),
                    Instruction::PushConst(8.into()),
                    Instruction::PushConst(9.into()),
                    Instruction::Eval(5),
                    Instruction::Jump(5),
                    Instruction::Deref(vm.get_symbol("+").unwrap()),
                    Instruction::PushConst(3.into()),
                    Instruction::PushConst(4.into()),
                    Instruction::PushConst(5.into()),
                    Instruction::Eval(4),
                ]
                .into(),
                source: Some(src.into()),
                instruction_source: vec![
                    Span { start: 5, end: 6 },
                    Span { start: 7, end: 8 },
                    Span { start: 9, end: 10 },
                    Span { start: 4, end: 11 },
                    Span { start: 0, end: 34 },
                    Span { start: 23, end: 24 },
                    Span { start: 25, end: 26 },
                    Span { start: 27, end: 28 },
                    Span { start: 29, end: 30 },
                    Span { start: 31, end: 32 },
                    Span { start: 22, end: 33 },
                    Span { start: 0, end: 34 },
                    Span { start: 13, end: 14 },
                    Span { start: 15, end: 16 },
                    Span { start: 17, end: 18 },
                    Span { start: 19, end: 20 },
                    Span { start: 12, end: 21 }
                ]
                .into(),
            }
        );
    }

    #[test]
    fn if_expression_with_empty_false_branch_defaults_to_void() {
        let mut vm = Vm::default();
        let src = "(if (< 1 2) (+ 4 5 6))";
        assert_eq!(
            Compiler::compile(&mut vm, src, &Bump::new()).unwrap(),
            ByteCode {
                name: "".into(),
                arg_count: 0,
                local_bindings: 0,
                instructions: vec![
                    Instruction::Deref(vm.get_symbol("<").unwrap()),
                    Instruction::PushConst(1.into()),
                    Instruction::PushConst(2.into()),
                    Instruction::Eval(3),
                    Instruction::JumpIf(2),
                    Instruction::PushConst(().into()),
                    Instruction::Jump(5),
                    Instruction::Deref(vm.get_symbol("+").unwrap()),
                    Instruction::PushConst(4.into()),
                    Instruction::PushConst(5.into()),
                    Instruction::PushConst(6.into()),
                    Instruction::Eval(4)
                ]
                .into(),
                source: Some(src.into()),
                instruction_source: [
                    Span { start: 5, end: 6 },
                    Span { start: 7, end: 8 },
                    Span { start: 9, end: 10 },
                    Span { start: 4, end: 11 },
                    Span { start: 0, end: 22 },
                    Span { start: 0, end: 0 },
                    Span { start: 0, end: 22 },
                    Span { start: 13, end: 14 },
                    Span { start: 15, end: 16 },
                    Span { start: 17, end: 18 },
                    Span { start: 19, end: 20 },
                    Span { start: 12, end: 21 }
                ]
                .into()
            }
        );
    }

    #[test]
    fn if_expression_allows_early_return_on_branches() {
        let mut vm = Vm::default();
        let src = "(if true (return 1) (return 2))";
        assert_eq!(
            Compiler::compile(&mut vm, src, &Bump::new()).unwrap(),
            ByteCode {
                name: "".into(),
                arg_count: 0,
                local_bindings: 0,
                instructions: vec![
                    Instruction::PushConst(true.into()),
                    Instruction::JumpIf(3),
                    Instruction::PushConst(2.into()),
                    Instruction::Return,
                    Instruction::Jump(2),
                    Instruction::PushConst(1.into()),
                    Instruction::Return
                ]
                .into(),
                source: Some(src.into()),
                instruction_source: vec![
                    Span { start: 4, end: 8 },
                    Span { start: 0, end: 31 },
                    Span { start: 28, end: 29 },
                    Span { start: 0, end: 0 },
                    Span { start: 0, end: 31 },
                    Span { start: 17, end: 18 },
                    Span { start: 0, end: 0 }
                ]
                .into(),
            }
        );
    }

    #[test]
    fn early_return_on_predicate_produces_error() {
        let mut vm = Vm::default();
        assert_eq!(
            Compiler::compile(&mut vm, "(if (return 10) 1 2)", &Bump::new()).unwrap_err(),
            CompileError::ExpectedExpression {
                context: "if predicate"
            },
        );
    }

    #[test]
    fn if_expression_with_non_expression_produces_error() {
        let mut vm = Vm::default();
        assert_eq!(
            Compiler::compile(&mut vm, "(if (define x 1) 1 2)", &Bump::new()).unwrap_err(),
            CompileError::DefineNotAllowed
        );
        assert_eq!(
            Compiler::compile(&mut vm, "(if true (define x 1) 2)", &Bump::new()).unwrap_err(),
            CompileError::DefineNotAllowed
        );
        assert_eq!(
            Compiler::compile(&mut vm, "(if true 1 (define x 2))", &Bump::new()).unwrap_err(),
            CompileError::DefineNotAllowed
        );
    }

    #[test]
    fn if_with_wrong_number_of_args_produces_arity_error() {
        let mut vm = Vm::default();
        assert_eq!(
            Compiler::compile(&mut vm, "(if)", &Bump::new()).unwrap_err(),
            CompileError::ExpressionHasWrongArgs {
                expression: "if",
                expected: 2,
                actual: 0
            },
        );
        assert_eq!(
            Compiler::compile(&mut vm, "(if true 1 2 3)", &Bump::new()).unwrap_err(),
            CompileError::ExpressionHasWrongArgs {
                expression: "if",
                expected: 3,
                actual: 4
            },
        );
    }

    ////////////////////////////////////////////////////////////////////////////////
    // BEGIN: Lambda
    ////////////////////////////////////////////////////////////////////////////////

    #[test]
    fn lambda_produces_lambda_expr() {
        let mut vm = Vm::default();
        let src = "(lambda () 1)";
        let actual = Compiler::compile(&mut vm, src, &Bump::new()).unwrap();
        assert_eq!(
            actual,
            ByteCode {
                name: "".into(),
                arg_count: 0,
                local_bindings: 0,
                instructions: vec![Instruction::PushConst(
                    UnsafeVal::ByteCodeFunction(vm.objects.get_or_insert_bytecode_slow(ByteCode {
                        name: "".into(),
                        arg_count: 0,
                        local_bindings: 0,
                        instructions: vec![Instruction::PushConst(1.into())].into(),
                        source: Some(src.into()),
                        instruction_source: [Span { start: 11, end: 12 }].into(),
                    }))
                    .into()
                )]
                .into(),
                source: Some(src.into()),
                instruction_source: vec![Span::new(0, 13)].into(),
            },
            "Inner bytecode is: {:?}",
            instruction_push_const_to_bytecode(&actual.instructions[0], &vm),
        );
    }

    #[test]
    fn lambda_can_reference_args() {
        let mut vm = Vm::default();
        let src = "(lambda (arg0 arg1 arg2) (arg1 arg0 arg2))";
        let actual = Compiler::compile(&mut vm, src, &Bump::new()).unwrap();
        assert_eq!(
            actual,
            ByteCode {
                name: "".into(),
                arg_count: 0,
                local_bindings: 0,
                instructions: vec![Instruction::PushConst(
                    UnsafeVal::ByteCodeFunction(
                        vm.objects.get_or_insert_bytecode_slow(ByteCode {
                            name: "".into(),
                            arg_count: 3,
                            local_bindings: 0,
                            instructions: vec![
                                Instruction::GetArg(1),
                                Instruction::GetArg(0),
                                Instruction::GetArg(2),
                                Instruction::Eval(3)
                            ]
                            .into(),
                            source: Some("(lambda (arg0 arg1 arg2) (arg1 arg0 arg2))".into()),
                            instruction_source: [
                                Span { start: 26, end: 30 },
                                Span { start: 31, end: 35 },
                                Span { start: 36, end: 40 },
                                Span { start: 25, end: 41 }
                            ]
                            .into()
                        })
                    )
                    .into()
                )]
                .into(),
                source: Some(src.into()),
                instruction_source: vec![Span::new(0, 42)].into(),
            },
            "Inner bytecode is: {:?}",
            instruction_push_const_to_bytecode(&actual.instructions[0], &vm),
        );
    }

    #[test]
    fn lambda_that_calls_self_with_push_current_function_instruction() {
        let mut vm = Vm::default();
        let src = "(define (foo n) (foo n))";
        let actual = Compiler::compile(&mut vm, src, &Bump::new()).unwrap();
        assert_eq!(
            actual,
            ByteCode {
                name: "".into(),
                arg_count: 0,
                local_bindings: 0,
                instructions: vec![
                    Instruction::PushConst(
                        UnsafeVal::ByteCodeFunction(
                            vm.objects.get_or_insert_bytecode_slow(ByteCode {
                                name: "foo".into(),
                                arg_count: 1,
                                local_bindings: 0,
                                instructions: vec![
                                    Instruction::PushCurrentFunction,
                                    Instruction::GetArg(0),
                                    Instruction::Eval(2)
                                ]
                                .into(),
                                source: Some(src.into()),
                                instruction_source: vec![
                                    Span { start: 17, end: 20 },
                                    Span { start: 21, end: 22 },
                                    Span { start: 16, end: 23 }
                                ]
                                .into(),
                            })
                        )
                        .into()
                    ),
                    Instruction::Define(vm.get_symbol("foo").unwrap()),
                ]
                .into(),
                source: Some(src.into()),
                instruction_source: vec![Span { start: 8, end: 23 }, Span { start: 9, end: 12 }]
                    .into(),
            },
            "Inner bytecode is {:?}",
            instruction_push_const_to_bytecode(&actual.instructions[0], &vm),
        );
    }

    #[test]
    fn lambda_with_same_arg_defined_multiple_times_produces_error() {
        let mut vm = Vm::default();
        assert_eq!(
            Compiler::compile(&mut vm, "(lambda (arg0 arg0) 0)", &Bump::new()).unwrap_err(),
            CompileError::ArgumentDefinedMultipleTimes("arg0".into())
        );
        // Tests the non-short arg optimized path.
        assert!(Compiler::compile(
            &mut vm,
            "(lambda (arg0 arg1 arg2 arg3 arg4 arg5 arg6) 0)",
            &Bump::new(),
        )
        .is_ok());
        assert_eq!(
            Compiler::compile(
                &mut vm,
                "(lambda (arg0 arg0 arg0 arg0 arg0 arg0 arg0) 0)",
                &Bump::new(),
            )
            .unwrap_err(),
            CompileError::ArgumentDefinedMultipleTimes("arg0".into())
        );
    }

    #[test]
    fn lambda_with_no_expr_produces_error() {
        let mut vm = Vm::default();
        let actual = Compiler::compile(&mut vm, "(lambda ())", &Bump::new()).unwrap_err();
        assert_eq!(
            actual,
            CompileError::ExpectedExpression {
                context: "lambda definition expressions"
            }
        );
    }

    #[test]
    fn lambda_with_define_statement_produces_error() {
        let mut vm = Vm::default();
        let actual =
            Compiler::compile(&mut vm, "(lambda () (define x 12))", &Bump::new()).unwrap_err();
        assert_eq!(actual, CompileError::DefineNotAllowed);
    }

    #[test]
    fn lambda_with_invalid_expression_produces_error() {
        let mut vm = Vm::default();
        let actual = Compiler::compile(&mut vm, "(lambda () (+ ()))", &Bump::new()).unwrap_err();
        assert_eq!(actual, CompileError::EmptyExpression);
    }

    ////////////////////////////////////////////////////////////////////////////////
    // BEGIN: Or
    ////////////////////////////////////////////////////////////////////////////////

    #[test]
    fn or_produces_jumps_to_end_on_first_true() {
        let mut vm = Vm::default();
        let src = "(or false 1 2)";
        assert_eq!(
            Compiler::compile(&mut vm, src, &Bump::new()).unwrap(),
            ByteCode {
                name: "".into(),
                arg_count: 0,
                local_bindings: 2,
                instructions: vec![
                    Instruction::PushConst(UnsafeVal::Bool(false)),
                    Instruction::BindArg(0),
                    Instruction::GetArg(0),
                    Instruction::JumpIf(8),
                    Instruction::PushConst(UnsafeVal::Int(1)),
                    Instruction::BindArg(1),
                    Instruction::GetArg(1),
                    Instruction::JumpIf(2),
                    Instruction::PushConst(UnsafeVal::Int(2)),
                    Instruction::Jump(1),
                    Instruction::GetArg(1),
                    Instruction::Jump(1),
                    Instruction::GetArg(0)
                ]
                .into(),
                source: Some(src.into()),
                instruction_source: vec![
                    Span { start: 4, end: 9 },
                    Span { start: 0, end: 14 },
                    Span { start: 4, end: 9 },
                    Span { start: 0, end: 14 },
                    Span { start: 10, end: 11 },
                    Span { start: 10, end: 14 },
                    Span { start: 10, end: 11 },
                    Span { start: 10, end: 14 },
                    Span { start: 12, end: 13 },
                    Span { start: 10, end: 14 },
                    Span { start: 10, end: 11 },
                    Span { start: 0, end: 14 },
                    Span { start: 4, end: 9 },
                ]
                .into(),
            },
        );
    }

    ////////////////////////////////////////////////////////////////////////////////
    // BEGIN: And
    ////////////////////////////////////////////////////////////////////////////////

    #[test]
    fn and_produces_jumps_to_end_on_first_false() {
        let mut vm = Vm::default();
        let src = "(and false 1 2)";
        assert_eq!(
            Compiler::compile(&mut vm, src, &Bump::new()).unwrap(),
            ByteCode {
                name: "".into(),
                arg_count: 0,
                local_bindings: 2,
                instructions: vec![
                    Instruction::PushConst(UnsafeVal::Bool(false)),
                    Instruction::BindArg(0),
                    Instruction::Deref(vm.get_symbol("not").unwrap()),
                    Instruction::GetArg(0),
                    Instruction::Eval(2),
                    Instruction::JumpIf(10),
                    Instruction::PushConst(UnsafeVal::Int(1)),
                    Instruction::BindArg(1),
                    Instruction::Deref(vm.get_symbol("not").unwrap()),
                    Instruction::GetArg(1),
                    Instruction::Eval(2),
                    Instruction::JumpIf(2),
                    Instruction::PushConst(UnsafeVal::Int(2)),
                    Instruction::Jump(1),
                    Instruction::GetArg(1),
                    Instruction::Jump(1),
                    Instruction::GetArg(0)
                ]
                .into(),
                source: Some(src.into()),
                instruction_source: vec![
                    Span { start: 5, end: 10 },
                    Span { start: 0, end: 15 },
                    Span { start: 5, end: 10 },
                    Span { start: 5, end: 10 },
                    Span { start: 5, end: 10 },
                    Span { start: 0, end: 15 },
                    Span { start: 11, end: 12 },
                    Span { start: 11, end: 15 },
                    Span { start: 11, end: 12 },
                    Span { start: 11, end: 12 },
                    Span { start: 11, end: 12 },
                    Span { start: 11, end: 15 },
                    Span { start: 13, end: 14 },
                    Span { start: 11, end: 15 },
                    Span { start: 11, end: 12 },
                    Span { start: 0, end: 15 },
                    Span { start: 5, end: 10 }
                ]
                .into(),
            },
        );
    }

    ////////////////////////////////////////////////////////////////////////////////
    // BEGIN: Return
    ////////////////////////////////////////////////////////////////////////////////

    #[test]
    fn return_produces_return_instruction() {
        let mut vm = Vm::default();
        let src = "(return (if true 1 2))";
        assert_eq!(
            Compiler::compile(&mut vm, src, &Bump::new()).unwrap(),
            ByteCode {
                name: "".into(),
                arg_count: 0,
                local_bindings: 0,
                instructions: vec![
                    Instruction::PushConst(true.into()),
                    Instruction::JumpIf(2),
                    Instruction::PushConst(2.into()),
                    Instruction::Jump(1),
                    Instruction::PushConst(1.into()),
                    Instruction::Return,
                ]
                .into(),
                source: Some(src.into()),
                instruction_source: vec![
                    Span { start: 12, end: 16 },
                    Span { start: 8, end: 21 },
                    Span { start: 19, end: 20 },
                    Span { start: 8, end: 21 },
                    Span { start: 17, end: 18 },
                    Span { start: 0, end: 0 }
                ]
                .into(),
            }
        );
    }

    #[test]
    fn return_with_non_expression_produces_error() {
        let mut vm = Vm::default();
        assert_eq!(
            Compiler::compile(&mut vm, "(return (define x 0))", &Bump::new()).unwrap_err(),
            CompileError::DefineNotAllowed
        );
    }

    #[test]
    fn return_with_no_expr_produces_error() {
        let mut vm = Vm::default();
        assert_eq!(
            Compiler::compile(&mut vm, "(return)", &Bump::new()).unwrap_err(),
            CompileError::ExpressionHasWrongArgs {
                expression: "return",
                expected: 1,
                actual: 0
            }
        );
    }

    #[test]
    fn return_with_too_many_exprs_produces_error() {
        let mut vm = Vm::default();
        assert_eq!(
            Compiler::compile(&mut vm, "(return 0 1 2)", &Bump::new()).unwrap_err(),
            CompileError::ExpressionHasWrongArgs {
                expression: "return",
                expected: 1,
                actual: 3
            }
        );
    }
}
