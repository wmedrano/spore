use crate::{
    ast::Node,
    error::CompileError,
    val::{ByteCode, Instruction},
    Val,
};

type Result<T> = std::result::Result<T, CompileError>;

pub struct Compiler {
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
            instructions: Vec::new(),
        };
        compiler.compile_impl(input_source, CompilerContext::Module)?;
        Ok(ByteCode {
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

    fn compile_one(&mut self, ir: &Ir, ctx: CompilerContext) -> Result<()> {
        match ir {
            Ir::Define { identifier, expr } => {
                if ctx != CompilerContext::Module {
                    return Err(CompileError::DefineNotAllowedInSubexpression);
                }
                if !expr.is_expression() {
                    return Err(CompileError::ExpectedExpression);
                }
                self.compile_one(expr, CompilerContext::Subexpression)?;
                self.instructions
                    .push(Instruction::Define(identifier.to_string()));
            }
            Ir::FunctionCall { function, args } => {
                if !function.is_expression() {
                    return Err(CompileError::ExpectedExpression);
                }
                self.compile_one(function, CompilerContext::Subexpression)?;
                for arg in args {
                    if !arg.is_expression() {
                        return Err(CompileError::ExpectedExpression);
                    }
                    self.compile_one(arg, CompilerContext::Subexpression)?;
                }
                self.instructions.push(Instruction::Eval(args.len() + 1));
            }
            Ir::Deref(ident) => self
                .instructions
                .push(Instruction::Deref(ident.to_string())),
            Ir::Constant(const_val) => {
                let val = match const_val {
                    Constant::Bool(x) => Val::Bool(*x),
                    Constant::Int(x) => Val::Int(*x),
                    Constant::Float(x) => Val::Float(*x),
                    Constant::String(x) => Val::String(x.to_string()),
                };
                self.instructions.push(Instruction::PushConst(val));
            }
        };
        Ok(())
    }
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
    /// Dereference a symbol.
    Deref(&'a str),
    /// A constant literal.
    Constant(Constant<'a>),
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
                [] => return Err(CompileError::EmptyExpression),
                [f, args @ ..] => match f {
                    Node::Identifier("define") => match args {
                        [Node::Identifier(identifier), expr] => Ir::Define {
                            identifier,
                            expr: Box::new(Ir::new(expr)?),
                        },
                        [_, _] => return Err(CompileError::ExpectedIdentifier),
                        _ => {
                            return Err(CompileError::ExpressionHasWrongArgs {
                                expression: "define",
                                expected: 2,
                                actual: args.len(),
                            })
                        }
                    },
                    ident => Ir::FunctionCall {
                        function: Box::new(Ir::new(ident)?),
                        args: args.iter().map(Ir::new).collect::<Result<Vec<_>>>()?,
                    },
                },
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
            Ir::Deref(_) => true,
            Ir::Constant(_) => true,
        }
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
                instructions: vec![
                    Instruction::PushConst(Val::Int(12)),
                    Instruction::Define("x".to_string()),
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
        assert_eq!(actual, CompileError::ExpectedExpression);
    }

    #[test]
    fn define_in_function_args_produces_error() {
        let actual = Compiler::compile("(+ 1 (define x 12))").unwrap_err();
        assert_eq!(actual, CompileError::ExpectedExpression);
    }

    #[test]
    fn define_in_function_call_produces_error() {
        let actual = Compiler::compile("((define x 12))").unwrap_err();
        assert_eq!(actual, CompileError::ExpectedExpression);
    }
}
