use std::collections::HashSet;

use crate::{
    ast::Node,
    error::CompileError,
    val::{ByteCode, Instruction},
    Val,
};

type Result<T> = std::result::Result<T, CompileError>;

pub struct Compiler {
    arguments: Vec<String>,
    instructions: Vec<Instruction>,
}

#[derive(Copy, Clone, PartialEq)]
enum CompilerContext {
    Module,
    Subexpression,
}

impl Compiler {
    pub fn compile(input_source: &str) -> Result<ByteCode> {
        let mut compiler = Compiler {
            arguments: Vec::new(),
            instructions: Vec::new(),
        };
        compiler.compile_impl(input_source, CompilerContext::Module)?;
        Ok(ByteCode {
            arg_count: 0,
            instructions: std::mem::take(&mut compiler.instructions),
        })
    }

    fn compile_impl(&mut self, input_source: &str, ctx: CompilerContext) -> Result<()> {
        for node_or_err in Node::parse(input_source) {
            let node = node_or_err.map_err(CompileError::AstError)?;
            let ir = Ir::new(&node)?;
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
            Ir::Define { identifier, expr } => {
                if ctx != CompilerContext::Module {
                    return Err(CompileError::DefineNotAllowedInSubexpression);
                }
                if !expr.is_expression() {
                    return Err(CompileError::ExpectedExpression { context: "define" });
                }
                self.compile_one(expr, CompilerContext::Subexpression)?;
                self.instructions
                    .push(Instruction::Define(identifier.to_string()));
            }
            Ir::FunctionCall { function, args } => {
                if !function.is_expression() {
                    return Err(CompileError::ExpectedExpression {
                        context: "function call",
                    });
                }
                self.compile_one(function, CompilerContext::Subexpression)?;
                for arg in args {
                    if !arg.is_expression() {
                        return Err(CompileError::ExpectedExpression {
                            context: "function call argument",
                        });
                    }
                    self.compile_one(arg, CompilerContext::Subexpression)?;
                }
                self.instructions.push(Instruction::Eval(args.len() + 1));
            }
            Ir::If {
                predicate,
                true_expr,
                false_expr,
            } => {
                if !predicate.is_expression() {
                    return Err(CompileError::ExpectedExpression {
                        context: "if predicate",
                    });
                }
                self.compile_one(predicate, CompilerContext::Subexpression)?;
                let true_jump_idx = self.instructions.len();
                self.instructions.push(Instruction::PushConst(Val::Void));
                match false_expr {
                    Some(expr) => {
                        if !expr.is_expression() {
                            return Err(CompileError::ExpectedExpression {
                                context: "if expression, false branch",
                            });
                        }
                        self.compile_one(expr, CompilerContext::Subexpression)?
                    }
                    None => self.instructions.push(Instruction::PushConst(Val::Void)),
                }
                let false_jump_idx = self.instructions.len();
                self.instructions.push(Instruction::PushConst(Val::Void));
                if !true_expr.is_expression() {
                    return Err(CompileError::ExpectedExpression {
                        context: "if expression, true branch",
                    });
                }
                self.compile_one(true_expr, CompilerContext::Subexpression)?;
                self.instructions[true_jump_idx] =
                    Instruction::JumpIf(false_jump_idx - true_jump_idx);
                self.instructions[false_jump_idx] =
                    Instruction::Jump(self.instructions.len() - false_jump_idx);
            }
            Ir::Deref(ident) => match self.arg_idx(ident) {
                Some(idx) => self.instructions.push(Instruction::GetArg(idx)),
                None => self
                    .instructions
                    .push(Instruction::Deref(ident.to_string())),
            },
            Ir::Constant(const_val) => {
                let val = match const_val {
                    Constant::Bool(x) => Val::Bool(*x),
                    Constant::Int(x) => Val::Int(*x),
                    Constant::Float(x) => Val::Float(*x),
                    Constant::String(x) => Val::String(x.to_string()),
                };
                self.instructions.push(Instruction::PushConst(val));
            }
            Ir::Lambda { args, expressions } => {
                if expressions.is_empty() {
                    return Err(CompileError::ExpectedExpression {
                        context: "lambda definition expressions",
                    });
                }
                let mut lambda_compiler = Compiler {
                    arguments: args.clone(),
                    instructions: Vec::new(),
                };
                if let Some(dupe) = find_duplicate(&lambda_compiler.arguments) {
                    return Err(CompileError::ArgumentDefinedMultipleTimes(dupe));
                }
                for expr in expressions.iter() {
                    lambda_compiler.compile_one(expr, CompilerContext::Subexpression)?;
                }
                let lambda_val = Val::ByteCodeFunction(ByteCode {
                    arg_count: args.len(),
                    instructions: lambda_compiler.instructions,
                });
                self.instructions.push(Instruction::PushConst(lambda_val));
            }
        };
        Ok(())
    }
}

fn find_duplicate(vec: &[String]) -> Option<String> {
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
enum Constant<'a> {
    /// A boolean consant.
    Bool(bool),
    /// An integer constant.
    Int(i64),
    /// A float constant.
    Float(f64),
    /// A string constant. The string is escaped.
    /// For example, "what \"is\" a string" is parsed as:
    ///     what "is" a string
    String(&'a str),
}

/// Contains the intermediate representation. This is a slightly more processed AST that is usefull
/// for compiling.
enum Ir<'a> {
    /// A define expression of the form: (define <name> <expr>)
    Define {
        /// The identifier to define.
        identifier: &'a str,
        /// The value of the definition.
        expr: Box<Self>,
    },
    /// A function call expression of the form: (<function> <args>...)
    FunctionCall {
        /// The function to call.
        function: Box<Self>,
        /// The arguments to the function.
        args: Vec<Self>,
    },
    /// A if expression.
    If {
        predicate: Box<Self>,
        true_expr: Box<Self>,
        false_expr: Option<Box<Self>>,
    },
    /// Dereference a symbol.
    Deref(&'a str),
    /// A constant literal.
    Constant(Constant<'a>),
    /// A lambda.
    Lambda {
        args: Vec<String>,
        expressions: Vec<Self>,
    },
}

impl<'a> Ir<'a> {
    fn new(node: &'a Node<'a>) -> Result<Ir> {
        let ir = match node {
            Node::Identifier(ident) => Ir::Deref(ident),
            Node::Bool(b) => Ir::Constant(Constant::Bool(*b)),
            Node::Int(int) => Ir::Constant(Constant::Int(*int)),
            Node::Float(float) => Ir::Constant(Constant::Float(*float)),
            Node::String(string) => Ir::Constant(Constant::String(string.as_str())),
            Node::Tree(tree) => match tree.as_slice() {
                [Node::Identifier("define"), define_args @ ..] => match define_args {
                    [Node::Identifier(identifier), expr] => Ir::Define {
                        identifier,
                        expr: Box::new(Ir::new(expr)?),
                    },
                    [Node::Tree(lambda_signature), exprs @ ..] => {
                        match lambda_signature.as_slice() {
                            [Node::Identifier(identifier), args @ ..] => Ir::Define {
                                identifier,
                                expr: Box::new(Ir::Lambda {
                                    args: args
                                        .iter()
                                        .map(node_to_ident)
                                        .collect::<Result<Vec<String>>>()?,
                                    expressions: exprs
                                        .iter()
                                        .map(Ir::new)
                                        .collect::<Result<Vec<_>>>()?,
                                }),
                            },
                            _ => {
                                return Err(CompileError::ExpectedIdentifierList {
                                    context: "function definition",
                                })
                            }
                        }
                    }
                    [_, _] => return Err(CompileError::ExpectedIdentifier),
                    _ => {
                        return Err(CompileError::ExpressionHasWrongArgs {
                            expression: "define",
                            expected: 2,
                            actual: define_args.len(),
                        })
                    }
                },
                [Node::Identifier("if"), args @ ..] => match args {
                    [predicate, true_expr] => Ir::If {
                        predicate: Box::new(Ir::new(predicate)?),
                        true_expr: Box::new(Ir::new(true_expr)?),
                        false_expr: None,
                    },
                    [predicate, true_expr, false_expr] => Ir::If {
                        predicate: Box::new(Ir::new(predicate)?),
                        true_expr: Box::new(Ir::new(true_expr)?),
                        false_expr: Some(Box::new(Ir::new(false_expr)?)),
                    },
                    _ => {
                        return Err(CompileError::ExpressionHasWrongArgs {
                            expression: "if",
                            expected: if args.len() > 3 { 3 } else { 2 },
                            actual: args.len(),
                        })
                    }
                },
                [Node::Identifier("lambda"), lambda_args, exprs @ ..] => {
                    let lambda_args = match lambda_args {
                        Node::Tree(t) => t,
                        _ => {
                            return Err(CompileError::ExpectedIdentifierList {
                                context: "lambda/function definition",
                            })
                        }
                    };
                    Ir::Lambda {
                        args: lambda_args
                            .iter()
                            .map(node_to_ident)
                            .collect::<Result<Vec<String>>>()?,
                        expressions: exprs.iter().map(Ir::new).collect::<Result<Vec<_>>>()?,
                    }
                }
                [f, args @ ..] => match f {
                    Node::Identifier("if") => match args {
                        [predicate, true_expr] => Ir::If {
                            predicate: Box::new(Ir::new(predicate)?),
                            true_expr: Box::new(Ir::new(true_expr)?),
                            false_expr: None,
                        },
                        [predicate, true_expr, false_expr] => Ir::If {
                            predicate: Box::new(Ir::new(predicate)?),
                            true_expr: Box::new(Ir::new(true_expr)?),
                            false_expr: Some(Box::new(Ir::new(false_expr)?)),
                        },
                        _ => {
                            return Err(CompileError::ExpressionHasWrongArgs {
                                expression: "if",
                                expected: if args.len() > 3 { 3 } else { 2 },
                                actual: args.len(),
                            })
                        }
                    },
                    ident => Ir::FunctionCall {
                        function: Box::new(Ir::new(ident)?),
                        args: args.iter().map(Ir::new).collect::<Result<Vec<_>>>()?,
                    },
                },
                [] => return Err(CompileError::EmptyExpression),
            },
        };
        Ok(ir)
    }

    /// Returns `true` if the IR contains an expression. Expressions return values while statements
    /// do not.
    fn is_expression(&self) -> bool {
        match self {
            Ir::Define { .. } => false,
            Ir::FunctionCall { .. } => true,
            Ir::If { .. } => true,
            Ir::Deref(_) => true,
            Ir::Constant(_) => true,
            Ir::Lambda { .. } => true,
        }
    }
}

fn node_to_ident(node: &Node) -> Result<String> {
    match node {
        Node::Identifier(ident) => Ok(ident.to_string()),
        _ => Err(CompileError::ExpectedIdentifierList {
            context: "lambda/function definition",
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_expression_is_empty() {
        let actual = Compiler::compile("").unwrap();
        assert_eq!(
            actual,
            ByteCode {
                arg_count: 0,
                instructions: vec![]
            }
        );
    }

    #[test]
    fn simple_expression_is_evaluated() {
        let actual = Compiler::compile("(+ 1 2)").unwrap();
        assert_eq!(
            actual,
            ByteCode {
                arg_count: 0,
                instructions: vec![
                    Instruction::Deref("+".to_string()),
                    Instruction::PushConst(Val::Int(1)),
                    Instruction::PushConst(Val::Int(2)),
                    Instruction::Eval(3),
                ]
            }
        );
    }

    #[test]
    fn multiple_expressions_are_evaluated_in_order() {
        let actual = Compiler::compile("(+ 1 2) (+ 3 4)").unwrap();
        assert_eq!(
            actual,
            ByteCode {
                arg_count: 0,
                instructions: vec![
                    Instruction::Deref("+".to_string()),
                    Instruction::PushConst(Val::Int(1)),
                    Instruction::PushConst(Val::Int(2)),
                    Instruction::Eval(3),
                    Instruction::Deref("+".to_string()),
                    Instruction::PushConst(Val::Int(3)),
                    Instruction::PushConst(Val::Int(4)),
                    Instruction::Eval(3),
                ]
            }
        );
    }

    #[test]
    fn nested_expressions_are_evaluated() {
        let actual = Compiler::compile("(+ 1 2 (+ 3 4))").unwrap();
        assert_eq!(
            actual,
            ByteCode {
                arg_count: 0,
                instructions: vec![
                    Instruction::Deref("+".to_string()),
                    Instruction::PushConst(Val::Int(1)),
                    Instruction::PushConst(Val::Int(2)),
                    Instruction::Deref("+".to_string()),
                    Instruction::PushConst(Val::Int(3)),
                    Instruction::PushConst(Val::Int(4)),
                    Instruction::Eval(3),
                    Instruction::Eval(4),
                ]
            }
        );
    }

    #[test]
    fn define_defines_a_new_value() {
        let actual = Compiler::compile("(define x 12)").unwrap();
        assert_eq!(
            actual,
            ByteCode {
                arg_count: 0,
                instructions: vec![
                    Instruction::PushConst(Val::Int(12)),
                    Instruction::Define("x".to_string()),
                ]
            }
        );
    }

    #[test]
    fn define_with_list_identifier_produces_lambda() {
        let actual = Compiler::compile("(define (foo a b) (+ a b))").unwrap();
        assert_eq!(
            actual,
            ByteCode {
                arg_count: 0,
                instructions: vec![
                    Instruction::PushConst(Val::ByteCodeFunction(ByteCode {
                        arg_count: 2,
                        instructions: vec![
                            Instruction::Deref("+".to_string()),
                            Instruction::GetArg(0),
                            Instruction::GetArg(1),
                            Instruction::Eval(3),
                        ],
                    })),
                    Instruction::Define("foo".to_string()),
                ]
            }
        );
    }

    #[test]
    fn define_with_subexpression_evaluates_subexpr() {
        let actual = Compiler::compile("(define x (+ 1 2))").unwrap();
        assert_eq!(
            actual,
            ByteCode {
                arg_count: 0,
                instructions: vec![
                    Instruction::Deref("+".to_string()),
                    Instruction::PushConst(Val::Int(1)),
                    Instruction::PushConst(Val::Int(2)),
                    Instruction::Eval(3),
                    Instruction::Define("x".to_string()),
                ]
            }
        );
    }

    #[test]
    fn define_in_define_expr_produces_error() {
        let actual = Compiler::compile("(define y (define x 12))").unwrap_err();
        assert_eq!(
            actual,
            CompileError::ExpectedExpression { context: "define" }
        );
    }

    #[test]
    fn define_in_function_args_produces_error() {
        let actual = Compiler::compile("(+ 1 (define x 12))").unwrap_err();
        assert_eq!(
            actual,
            CompileError::ExpectedExpression {
                context: "function call argument"
            }
        );
    }

    #[test]
    fn define_in_function_call_produces_error() {
        let actual = Compiler::compile("((define x 12))").unwrap_err();
        assert_eq!(
            actual,
            CompileError::ExpectedExpression {
                context: "function call"
            }
        );
    }

    #[test]
    fn lambda_produces_lambda_expr() {
        let actual = Compiler::compile("(lambda () 1)").unwrap();
        assert_eq!(
            actual,
            ByteCode {
                arg_count: 0,
                instructions: vec![Instruction::PushConst(Val::ByteCodeFunction(ByteCode {
                    arg_count: 0,
                    instructions: vec![Instruction::PushConst(Val::Int(1))],
                })),]
            }
        );
    }

    #[test]
    fn lambda_can_reference_args() {
        let actual = Compiler::compile("(lambda (arg0 arg1 arg2) (arg1 arg0 arg2))").unwrap();
        assert_eq!(
            actual,
            ByteCode {
                arg_count: 0,
                instructions: vec![Instruction::PushConst(Val::ByteCodeFunction(ByteCode {
                    arg_count: 3,
                    instructions: vec![
                        Instruction::GetArg(1),
                        Instruction::GetArg(0),
                        Instruction::GetArg(2),
                        Instruction::Eval(3)
                    ],
                })),]
            }
        );
    }

    #[test]
    fn lambda_with_same_arg_defined_twice_produces_error() {
        let actual = Compiler::compile("(lambda (arg0 arg0) (arg0 arg0))").unwrap_err();
        assert_eq!(
            actual,
            CompileError::ArgumentDefinedMultipleTimes("arg0".to_string())
        );
    }

    #[test]
    fn lambda_with_no_expr_produces_error() {
        let actual = Compiler::compile("(lambda ())").unwrap_err();
        assert_eq!(
            actual,
            CompileError::ExpectedExpression {
                context: "lambda definition expressions"
            }
        );
    }
}
