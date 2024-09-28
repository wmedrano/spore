use std::{collections::HashSet, sync::Arc};

use bumpalo::Bump;
use compact_str::CompactString;

type BumpVec<'a, T> = bumpalo::collections::Vec<'a, T>;

use crate::{
    error::CompileError,
    parser::{ast::Node, tokenizer::Span},
    val::{ByteCode, Instruction, UnsafeVal},
    Settings, Vm,
};

type Result<T> = std::result::Result<T, CompileError>;

pub struct Compiler<'a> {
    vm: &'a mut Vm,
    arena: &'a Bump,
    source: Option<Arc<str>>,
    settings: Settings,
    function_name: Option<CompactString>,
    arguments: BumpVec<'a, CompactString>,
    instructions: BumpVec<'a, Instruction>,
    instruction_source: BumpVec<'a, Span>,
}

#[derive(Copy, Clone, PartialEq)]
enum CompilerContext {
    Module,
    Subexpression,
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
            instructions: BumpVec::new_in(arena),
            instruction_source: BumpVec::new_in(arena),
        };
        compiler.compile_impl(input_source, CompilerContext::Module)?;
        Ok(ByteCode {
            name: "".into(),
            arg_count: 0,
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
        for (idx, sym) in self.arguments.iter().enumerate() {
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
                name,
                args,
                expressions,
            } => self.compile_one_lambda(*name, args, expressions)?,
            Ir::Return { expr } => self.compile_one_return(expr)?,
        };
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
                    .then(|| self.vm.values.get(ident))
                    .flatten()
                    .map(|c| Instruction::PushConst(*c));
                let instruction = maybe_inlined_val.unwrap_or(Instruction::Deref((*ident).into()));
                self.instruction_source.push(span);
                self.instructions.push(instruction)
            }
        };
        Ok(())
    }

    fn compile_one_function_call(&mut self, span: Span, function: &Ir, args: &[Ir]) -> Result<()> {
        if function.return_type() == IrReturnType::None {
            return Err(CompileError::ExpectedExpression {
                context: "function call",
            });
        }
        let maybe_native_function = self
            .settings
            .enable_aggressive_inline
            .then(|| match function {
                Ir::Deref(_, ident) => match self.vm.values.get(*ident) {
                    Some(UnsafeVal::NativeFunction(func)) => Some(*func),
                    _ => None,
                },
                _ => None,
            })
            .flatten();
        if maybe_native_function.is_none() {
            self.compile_one(function, CompilerContext::Subexpression)?;
        }
        for arg in args {
            if arg.return_type() != IrReturnType::Value {
                return Err(CompileError::ExpectedExpression {
                    context: "function call argument",
                });
            }
            self.compile_one(arg, CompilerContext::Subexpression)?;
        }
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
        identifier: &str,
        expr: &Ir,
    ) -> Result<()> {
        if ctx != CompilerContext::Module {
            return Err(CompileError::DefineNotAllowedInSubexpression);
        }
        if expr.return_type() != IrReturnType::Value {
            return Err(CompileError::ExpectedExpression { context: "define" });
        }
        self.compile_one(expr, CompilerContext::Subexpression)?;
        self.instruction_source.push(span);
        self.instructions
            .push(Instruction::Define((*identifier).into()));
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
            Some(expr) => {
                if expr.return_type() == IrReturnType::None {
                    return Err(CompileError::ExpectedExpression {
                        context: "if expression, false branch",
                    });
                }
                self.compile_one(expr, CompilerContext::Subexpression)?
            }
            None => {
                self.instruction_source.push(Span::new(0, 0));
                self.instructions.push(Instruction::PushConst(().into()))
            }
        }
        let false_jump_idx = self.instructions.len();
        self.instruction_source.push(span);
        self.instructions.push(Instruction::PushConst(().into()));
        if true_expr.return_type() == IrReturnType::None {
            return Err(CompileError::ExpectedExpression {
                context: "if expression, true branch",
            });
        }
        self.compile_one(true_expr, CompilerContext::Subexpression)?;
        self.instructions[true_jump_idx] = Instruction::JumpIf(false_jump_idx - true_jump_idx);
        self.instruction_source[false_jump_idx] = span;
        self.instructions[false_jump_idx] =
            Instruction::Jump(self.instructions.len() - false_jump_idx - 1);
        Ok(())
    }

    fn compile_one_lambda(
        &mut self,
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
            instructions: BumpVec::new_in(self.arena),
            instruction_source: BumpVec::new_in(self.arena),
        };
        if let Some(dupe) = find_duplicate(&lambda_compiler.arguments) {
            return Err(CompileError::ArgumentDefinedMultipleTimes(dupe));
        }
        for expr in expressions.iter() {
            lambda_compiler.compile_one(expr, CompilerContext::Subexpression)?;
        }
        let lambda_val =
            UnsafeVal::ByteCodeFunction(lambda_compiler.vm.objects.insert_bytecode(ByteCode {
                name: name.unwrap_or("").into(),
                arg_count: args.len(),
                instructions: lambda_compiler.instructions.into_bump_slice().into(),
                source: lambda_compiler.source,
                instruction_source: lambda_compiler.instruction_source.into_bump_slice().into(),
            }));
        self.instruction_source.push(Span::new(0, 0));
        self.instructions.push(Instruction::PushConst(lambda_val));
        Ok(())
    }

    fn compile_one_return(&mut self, expr: &Ir) -> Result<()> {
        if expr.return_type() == IrReturnType::None {
            return Err(CompileError::ExpectedExpression {
                context: "return statement expected expression",
            });
        }
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

/// Contains a constant.
///
/// All values should require no additional cleanup for best use with [Ir]'s arena.
#[derive(Clone, Debug)]
enum Constant {
    /// Just void, nothing.
    Void,
    /// A boolean consant.
    Bool(bool),
    /// An integer constant.
    Int(i64),
    /// A float constant.
    Float(f64),
    /// A string constant. The string is escaped.
    /// For example, "what \"is\" a string" is parsed as:
    ///     what "is" a string
    String(CompactString),
}

/// Contains the intermediate representation. This is a slightly more processed AST that is usefull
/// for compiling.
///
/// # Memory Leak
/// Value's are usually arena allocated so `drop` will not be called for a conventional
/// cleanup. This means all values must either be normal values, references, or arena allocated
/// references. To detect possible memory leaks, try running valgrind on the test suite.
#[derive(Debug)]
enum Ir<'a> {
    /// A constant literal.
    Constant(Span, Constant),
    /// Dereference a symbol.
    Deref(Span, &'a str),
    /// A function call expression of the form: (<function> <args>...)
    FunctionCall {
        /// The source code where this function call is defined.
        span: Span,
        /// The function to call.
        function: &'a Self,
        /// The arguments to the function.
        args: BumpVec<'a, Self>,
    },
    /// A define expression of the form: (define <name> <expr>)
    Define {
        /// The source code where this function call is defined.
        span: Span,
        /// The identifier to define.
        identifier: &'a str,
        /// The value of the definition.
        expr: &'a Self,
    },
    /// A if expression.
    If {
        span: Span,
        predicate: &'a Self,
        true_expr: &'a Self,
        false_expr: Option<&'a Self>,
    },
    /// A lambda.
    Lambda {
        name: Option<&'a str>,
        args: BumpVec<'a, &'a str>,
        expressions: BumpVec<'a, Self>,
    },
    /// Return the result of the given expression.
    Return { expr: &'a Self },
}

impl<'a> Ir<'a> {
    fn new(arena: &'a Bump, src: &'a str, node: &'a Node) -> Result<Ir<'a>> {
        let ir = match node {
            Node::Void(span) => Ir::Constant(*span, Constant::Void),
            Node::Bool(span, b) => Ir::Constant(*span, Constant::Bool(*b)),
            Node::Int(span, int) => Ir::Constant(*span, Constant::Int(*int)),
            Node::Float(span, float) => Ir::Constant(*span, Constant::Float(*float)),
            Node::String(span) => Ir::Constant(
                *span,
                Constant::String(node.to_string_literal(src).unwrap()),
            ),
            Node::Identifier(ident_span) => Ir::Deref(*ident_span, ident_span.as_str(src)),
            Node::Tree(span, tree) => Self::new_tree(arena, src, *span, tree)?,
        };
        Ok(ir)
    }

    fn new_define(
        arena: &'a Bump,
        src: &'a str,
        span: Span,
        define_args: &'a [Node],
    ) -> Result<Ir<'a>> {
        let ir = match define_args {
            [Node::Identifier(identifier_span), expr] => Ir::Define {
                span,
                identifier: identifier_span.as_str(src),
                expr: arena.alloc(Ir::new(arena, src, expr)?),
            },
            [Node::Tree(_, lambda_signature), exprs @ ..] => match lambda_signature.as_slice() {
                [Node::Identifier(identifier_span), args @ ..] => {
                    let mut args_vec = BumpVec::new_in(arena);
                    for arg in args.iter() {
                        let ident = node_to_ident(src, arg)?;
                        args_vec.push(ident);
                    }
                    let mut exprs_vec = BumpVec::new_in(arena);
                    for expr in exprs.iter() {
                        let expr_ir = Ir::new(arena, src, expr)?;
                        exprs_vec.push(expr_ir);
                    }
                    Ir::Define {
                        span: *identifier_span,
                        identifier: identifier_span.as_str(src),
                        expr: arena.alloc(Ir::Lambda {
                            name: Some(identifier_span.as_str(src)),
                            args: args_vec,
                            expressions: exprs_vec,
                        }),
                    }
                }
                _ => {
                    return Err(CompileError::ExpectedIdentifierList {
                        context: "function definition",
                    })
                }
            },
            [_, _] => return Err(CompileError::ExpectedIdentifier),
            _ => {
                return Err(CompileError::ExpressionHasWrongArgs {
                    expression: "define",
                    expected: 2,
                    actual: define_args.len(),
                })
            }
        };
        Ok(ir)
    }
    fn new_tree(arena: &'a Bump, src: &'a str, span: Span, tree: &'a [Node]) -> Result<Ir<'a>> {
        let ir = match tree {
            [Node::Identifier(maybe_define), define_args @ ..]
                if maybe_define.as_str(src) == "define" =>
            {
                Self::new_define(arena, src, span, define_args)?
            }
            [Node::Identifier(maybe_if), args @ ..] if maybe_if.as_str(src) == "if" => match args {
                [predicate, true_expr] => Ir::If {
                    span,
                    predicate: arena.alloc(Ir::new(arena, src, predicate)?),
                    true_expr: arena.alloc(Ir::new(arena, src, true_expr)?),
                    false_expr: None,
                },
                [predicate, true_expr, false_expr] => Ir::If {
                    span,
                    predicate: arena.alloc(Ir::new(arena, src, predicate)?),
                    true_expr: arena.alloc(Ir::new(arena, src, true_expr)?),
                    false_expr: Some(arena.alloc(Ir::new(arena, src, false_expr)?)),
                },
                _ => {
                    return Err(CompileError::ExpressionHasWrongArgs {
                        expression: "if",
                        expected: if args.len() > 3 { 3 } else { 2 },
                        actual: args.len(),
                    })
                }
            },
            [Node::Identifier(maybe_lambda), lambda_args, exprs @ ..]
                if maybe_lambda.as_str(src) == "lambda" =>
            {
                let lambda_args = match lambda_args {
                    Node::Tree(_, t) => t,
                    _ => {
                        return Err(CompileError::ExpectedIdentifierList {
                            context: "lambda/function definition",
                        })
                    }
                };
                let mut args_vec = BumpVec::new_in(arena);
                for arg in lambda_args.iter() {
                    let ident = node_to_ident(src, arg)?;
                    args_vec.push(ident);
                }
                let mut exprs_vec = BumpVec::new_in(arena);
                for expr in exprs.iter() {
                    let expr_ir = Ir::new(arena, src, expr)?;
                    exprs_vec.push(expr_ir);
                }
                Ir::Lambda {
                    name: None,
                    args: args_vec,
                    expressions: exprs_vec,
                }
            }
            [Node::Identifier(maybe_return), return_args @ ..]
                if maybe_return.as_str(src) == "return" =>
            {
                match return_args {
                    [expr] => Ir::Return {
                        expr: arena.alloc(Ir::new(arena, src, expr)?),
                    },
                    _ => {
                        return Err(CompileError::ExpressionHasWrongArgs {
                            expression: "return",
                            expected: 1,
                            actual: return_args.len(),
                        })
                    }
                }
            }
            [f, args @ ..] => match f {
                Node::Identifier(maybe_if) if maybe_if.as_str(src) == "if" => match args {
                    [predicate, true_expr] => Ir::If {
                        span: *maybe_if,
                        predicate: arena.alloc(Ir::new(arena, src, predicate)?),
                        true_expr: arena.alloc(Ir::new(arena, src, true_expr)?),
                        false_expr: None,
                    },
                    [predicate, true_expr, false_expr] => Ir::If {
                        span: *maybe_if,
                        predicate: arena.alloc(Ir::new(arena, src, predicate)?),
                        true_expr: arena.alloc(Ir::new(arena, src, true_expr)?),
                        false_expr: Some(arena.alloc(Ir::new(arena, src, false_expr)?)),
                    },
                    _ => {
                        return Err(CompileError::ExpressionHasWrongArgs {
                            expression: "if",
                            expected: if args.len() > 3 { 3 } else { 2 },
                            actual: args.len(),
                        })
                    }
                },
                ident => {
                    let mut args_vec = BumpVec::new_in(arena);
                    for arg in args.iter() {
                        let arg_ir = Ir::new(arena, src, arg)?;
                        args_vec.push(arg_ir);
                    }
                    Ir::FunctionCall {
                        span,
                        function: arena.alloc(Ir::new(arena, src, ident)?),
                        args: args_vec,
                    }
                }
            },
            [] => return Err(CompileError::EmptyExpression),
        };
        Ok(ir)
    }

    /// Returns `true` if the IR contains an expression. Expressions return values while statements
    /// do not.
    fn return_type(&self) -> IrReturnType {
        match self {
            Ir::Constant(_, _) => IrReturnType::Value,
            Ir::Deref(_, _) => IrReturnType::Value,
            Ir::FunctionCall { .. } => IrReturnType::Value,
            Ir::Define { .. } => IrReturnType::None,
            Ir::If { .. } => IrReturnType::Value,
            Ir::Lambda { .. } => IrReturnType::Value,
            Ir::Return { .. } => IrReturnType::EarlyReturn,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum IrReturnType {
    /// The IR does not return anything. This means nothing is pushed to the stack.
    None,
    /// A value is pushed to the top of the stack.
    Value,
    /// The current function is returned, exiting the current function call frame.
    EarlyReturn,
}

fn node_to_ident<'a>(src: &'a str, node: &Node) -> Result<&'a str> {
    match node {
        Node::Identifier(ident) => Ok(ident.as_str(src)),
        _ => Err(CompileError::ExpectedIdentifierList {
            context: "lambda/function definition",
        }),
    }
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
                instructions: vec![Instruction::Deref("my-variable".into())].into(),
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
                instructions: vec![
                    Instruction::Deref("does-not-exist".into()),
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
                instructions: vec![
                    Instruction::Deref("get-fn".into()),
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
                instructions: vec![Instruction::Deref("+".into()), Instruction::Eval(1)].into(),
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
                instructions: vec![
                    Instruction::Deref("+".into()),
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
                instructions: vec![
                    Instruction::Deref("+".into()),
                    Instruction::PushConst(1.into()),
                    Instruction::PushConst(2.into()),
                    Instruction::Eval(3),
                    Instruction::Deref("+".into()),
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
                instructions: vec![
                    Instruction::Deref("+".into()),
                    Instruction::PushConst(1.into()),
                    Instruction::PushConst(2.into()),
                    Instruction::Deref("+".into()),
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
        assert_eq!(
            actual,
            CompileError::ExpectedExpression {
                context: "function call argument"
            }
        );
    }

    #[test]
    fn define_in_function_call_produces_error() {
        let mut vm = Vm::default();
        let actual = Compiler::compile(&mut vm, "((define x 12))", &Bump::new()).unwrap_err();
        assert_eq!(
            actual,
            CompileError::ExpectedExpression {
                context: "function call"
            }
        );
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
                instructions: vec![
                    Instruction::PushConst(12.into()),
                    Instruction::Define("x".into()),
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
                instructions: vec![
                    Instruction::PushConst(
                        UnsafeVal::ByteCodeFunction(
                            vm.objects.get_or_insert_bytecode_slow(ByteCode {
                                name: "foo".into(),
                                arg_count: 2,
                                instructions: vec![
                                    Instruction::Deref("+".into()),
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
                    Instruction::Define("foo".into()),
                ]
                .into(),
                source: Some(src.into()),
                instruction_source: vec![Span { start: 0, end: 0 }, Span { start: 9, end: 12 }]
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
                instructions: vec![
                    Instruction::Deref("+".into()),
                    Instruction::PushConst(1.into()),
                    Instruction::PushConst(2.into()),
                    Instruction::Eval(3),
                    Instruction::Define("x".into()),
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
        assert_eq!(
            actual,
            CompileError::ExpectedExpression { context: "define" }
        );
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
                instructions: vec![
                    Instruction::Deref("<".into()),
                    Instruction::PushConst(1.into()),
                    Instruction::PushConst(2.into()),
                    Instruction::Eval(3),
                    Instruction::JumpIf(7),
                    Instruction::Deref("+".into()),
                    Instruction::PushConst(6.into()),
                    Instruction::PushConst(7.into()),
                    Instruction::PushConst(8.into()),
                    Instruction::PushConst(9.into()),
                    Instruction::Eval(5),
                    Instruction::Jump(5),
                    Instruction::Deref("+".into()),
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
                instructions: vec![
                    Instruction::Deref("<".into()),
                    Instruction::PushConst(1.into()),
                    Instruction::PushConst(2.into()),
                    Instruction::Eval(3),
                    Instruction::JumpIf(2),
                    Instruction::PushConst(().into()),
                    Instruction::Jump(5),
                    Instruction::Deref("+".into()),
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
            CompileError::ExpectedExpression {
                context: "if predicate"
            }
        );
        assert_eq!(
            Compiler::compile(&mut vm, "(if true (define x 1) 2)", &Bump::new()).unwrap_err(),
            CompileError::ExpectedExpression {
                context: "if expression, true branch"
            }
        );
        assert_eq!(
            Compiler::compile(&mut vm, "(if true 1 (define x 2))", &Bump::new()).unwrap_err(),
            CompileError::ExpectedExpression {
                context: "if expression, false branch"
            }
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
                instructions: vec![Instruction::PushConst(
                    UnsafeVal::ByteCodeFunction(vm.objects.get_or_insert_bytecode_slow(ByteCode {
                        name: "".into(),
                        arg_count: 0,
                        instructions: vec![Instruction::PushConst(1.into())].into(),
                        source: Some(src.into()),
                        instruction_source: [Span { start: 11, end: 12 }].into(),
                    }))
                    .into()
                )]
                .into(),
                source: Some(src.into()),
                instruction_source: vec![Span::new(0, 0)].into(),
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
                instructions: vec![Instruction::PushConst(
                    UnsafeVal::ByteCodeFunction(
                        vm.objects.get_or_insert_bytecode_slow(ByteCode {
                            name: "".into(),
                            arg_count: 3,
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
                instruction_source: vec![Span::new(0, 0)].into(),
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
                instructions: vec![
                    Instruction::PushConst(
                        UnsafeVal::ByteCodeFunction(
                            vm.objects.get_or_insert_bytecode_slow(ByteCode {
                                name: "foo".into(),
                                arg_count: 1,
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
                    Instruction::Define("foo".into()),
                ]
                .into(),
                source: Some(src.into()),
                instruction_source: vec![Span { start: 0, end: 0 }, Span { start: 9, end: 12 }]
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
        assert_eq!(actual, CompileError::DefineNotAllowedInSubexpression);
    }

    #[test]
    fn lambda_with_invalid_expression_produces_error() {
        let mut vm = Vm::default();
        let actual = Compiler::compile(&mut vm, "(lambda () (+ ()))", &Bump::new()).unwrap_err();
        assert_eq!(actual, CompileError::EmptyExpression);
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
            CompileError::ExpectedExpression {
                context: "return statement expected expression",
            },
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
