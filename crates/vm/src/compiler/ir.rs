use bumpalo::Bump;
use compact_str::CompactString;

use crate::{
    error::CompileError,
    parser::{ast::Node, span::Span},
};

type BumpVec<'a, T> = bumpalo::collections::Vec<'a, T>;
type Result<T> = std::result::Result<T, CompileError>;

/// Contains a constant.
///
/// All values should require no additional cleanup for best use with [Ir]'s arena.
#[derive(Clone, Debug)]
pub enum Constant {
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
#[derive(Clone, Debug)]
pub enum Ir<'a> {
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
        span: Span,
        name: Option<&'a str>,
        args: BumpVec<'a, &'a str>,
        expressions: BumpVec<'a, Self>,
    },
    /// A let expression.
    Let {
        span: Span,
        bindings: BumpVec<'a, (&'a str, Self)>,
        expressions: BumpVec<'a, Self>,
    },
    /// Return the result of the given expression.
    Return { expr: &'a Self },
}

impl<'a> Ir<'a> {
    pub fn new(arena: &'a Bump, src: &'a str, node: &Node) -> Result<Ir<'a>> {
        let ir = match node {
            Node::Void(span) => Ir::Constant(*span, Constant::Void),
            Node::Bool(span, b) => Ir::Constant(*span, Constant::Bool(*b)),
            Node::Int(span, int) => Ir::Constant(*span, Constant::Int(*int)),
            Node::Float(span, float) => Ir::Constant(*span, Constant::Float(*float)),
            Node::String(span) => Ir::Constant(
                *span,
                Constant::String(node.to_string_literal(src).unwrap()),
            ),
            Node::Identifier(ident_span) => {
                Ir::Deref(*ident_span, ident_span.with_src(src).as_str())
            }
            Node::Tree(span, tree) => Self::new_tree(arena, src, *span, tree)?,
        };
        Ok(ir)
    }

    pub fn new_many(arena: &'a Bump, src: &'a str, nodes: &[Node]) -> Result<BumpVec<'a, Ir<'a>>> {
        let mut res = BumpVec::with_capacity_in(nodes.len(), arena);
        for node in nodes {
            res.push(Self::new(arena, src, node)?);
        }
        Ok(res)
    }

    /// Returns `true` if the IR contains an expression. Expressions return values while statements
    /// do not.
    pub fn return_type(&self) -> IrReturnType {
        match self {
            Ir::Constant(_, _) => IrReturnType::Value,
            Ir::Deref(_, _) => IrReturnType::Value,
            Ir::FunctionCall { .. } => IrReturnType::Value,
            Ir::Define { .. } => IrReturnType::Value,
            Ir::If { .. } => IrReturnType::Value,
            Ir::Lambda { .. } => IrReturnType::Value,
            Ir::Let { .. } => IrReturnType::Value,
            Ir::Return { .. } => IrReturnType::EarlyReturn,
        }
    }

    fn new_tree(arena: &'a Bump, src: &'a str, span: Span, tree: &[Node]) -> Result<Ir<'a>> {
        let ir = match tree {
            [leading_node @ Node::Identifier(leading_ident), rest @ ..] => {
                match leading_ident.with_src(src).as_str() {
                    "define" => Self::new_define(arena, src, span, rest)?,
                    "if" => match rest {
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
                                expected: if rest.len() > 3 { 3 } else { 2 },
                                actual: rest.len(),
                            })
                        }
                    },
                    "lambda" => match rest {
                        [lambda_args, exprs @ ..] => {
                            let lambda_args = match lambda_args {
                                Node::Tree(_, t) => t,
                                _ => {
                                    return Err(CompileError::ExpectedIdentifierList {
                                        context: "lambda/function definition",
                                    })
                                }
                            };
                            Self::new_lambda(arena, src, span, None, lambda_args, exprs)?
                        }
                        _ => {
                            return Err(CompileError::ExpressionHasWrongArgs {
                                expression: "lambda",
                                expected: 2,
                                actual: rest.len(),
                            })
                        }
                    },
                    "let" => match rest {
                        [bindings, exprs @ ..] => Self::new_let(arena, src, span, bindings, exprs)?,
                        [] => {
                            return Err(CompileError::ExpressionHasWrongArgs {
                                expression: "let",
                                expected: 1,
                                actual: rest.len(),
                            })
                        }
                    },
                    "or" => Self::new_or_expression(arena, src, span, rest)?,
                    "and" => Self::new_and_expression(arena, src, span, rest)?,
                    "return" => match rest {
                        [expr] => Ir::Return {
                            expr: arena.alloc(Ir::new(arena, src, expr)?),
                        },
                        _ => {
                            return Err(CompileError::ExpressionHasWrongArgs {
                                expression: "return",
                                expected: 1,
                                actual: rest.len(),
                            })
                        }
                    },
                    _function => Self::new_function_call(arena, src, span, leading_node, rest)?,
                }
            }
            [function @ Node::Tree(_, _), rest @ ..] => {
                Self::new_function_call(arena, src, span, function, rest)?
            }
            [Node::Void(_)
            | Node::Bool(_, _)
            | Node::Int(_, _)
            | Node::Float(_, _)
            | Node::String(_), ..] => {
                return Err(CompileError::ConstantNotCallable(
                    span.with_src(src).to_string(),
                ))
            }
            [] => return Err(CompileError::EmptyExpression),
        };
        Ok(ir)
    }

    fn new_function_call(
        arena: &'a Bump,
        src: &'a str,
        span: Span,
        function: &Node,
        args: &[Node],
    ) -> Result<Ir<'a>> {
        let args_vec = Self::new_many(arena, src, args)?;
        Ok(Ir::FunctionCall {
            span,
            function: arena.alloc(Ir::new(arena, src, function)?),
            args: args_vec,
        })
    }

    fn new_define(
        arena: &'a Bump,
        src: &'a str,
        span: Span,
        define_args: &[Node],
    ) -> Result<Ir<'a>> {
        let ir = match define_args {
            [Node::Identifier(ident_span), expr] => Ir::Define {
                span,
                identifier: ident_span.with_src(src).as_str(),
                expr: arena.alloc(Ir::new(arena, src, expr)?),
            },
            [Node::Tree(lambda_signature_span, lambda_signature), exprs @ ..] => {
                let lambda_span = if let Some(expr) = exprs.last() {
                    lambda_signature_span.extend_end(expr.span().end)
                } else {
                    *lambda_signature_span
                };
                match lambda_signature.as_slice() {
                    [Node::Identifier(ident_span), lambda_args @ ..] => {
                        let name = ident_span.with_src(src).as_str();
                        let lambda_ir = Ir::new_lambda(
                            arena,
                            src,
                            lambda_span,
                            Some(name),
                            lambda_args,
                            exprs,
                        )?;
                        Ir::Define {
                            span: *ident_span,
                            identifier: ident_span.with_src(src).as_str(),
                            expr: arena.alloc(lambda_ir),
                        }
                    }
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
        };
        Ok(ir)
    }

    fn new_lambda(
        arena: &'a Bump,
        src: &'a str,
        span: Span,
        name: Option<&'a str>,
        lambda_args: &[Node],
        exprs: &[Node],
    ) -> Result<Ir<'a>> {
        let mut args_vec = BumpVec::with_capacity_in(lambda_args.len(), arena);
        for arg in lambda_args.iter() {
            let ident = node_to_ident(src, arg)?;
            args_vec.push(ident);
        }
        let exprs_vec = Self::new_many(arena, src, exprs)?;
        Ok(Ir::Lambda {
            span,
            name,
            args: args_vec,
            expressions: exprs_vec,
        })
    }

    fn new_let(
        arena: &'a Bump,
        src: &'a str,
        span: Span,
        bindings: &Node,
        exprs: &[Node],
    ) -> Result<Ir<'a>> {
        let bindings_ast = match bindings {
            Node::Tree(_, tree) => tree.as_slice(),
            _ => return Err(CompileError::BadLetBindings),
        };
        let bindings = Self::parse_let_bindings(arena, src, bindings_ast)?;
        let expressions = Self::new_many(arena, src, exprs)?;
        Ok(Ir::Let {
            span,
            bindings,
            expressions,
        })
    }

    fn parse_let_bindings(
        arena: &'a Bump,
        src: &'a str,
        bindings: &[Node],
    ) -> Result<BumpVec<'a, (&'a str, Ir<'a>)>> {
        let mut ret = BumpVec::with_capacity_in(bindings.len(), arena);
        for node in bindings {
            match node {
                Node::Tree(_, tree) => match tree.as_slice() {
                    [Node::Identifier(ident), expr] => {
                        ret.push((ident.with_src(src).as_str(), Self::new(arena, src, expr)?));
                    }
                    _ => return Err(CompileError::BadLetBindings),
                },
                _ => return Err(CompileError::BadLetBindings),
            }
        }
        Ok(ret)
    }

    fn new_or_expression(
        arena: &'a Bump,
        src: &'a str,
        span: Span,
        exprs: &[Node],
    ) -> Result<Ir<'a>> {
        let ir = match exprs {
            [] => Ir::Constant(span, Constant::Bool(false)),
            [expr_node] => Ir::new(arena, src, expr_node)?,
            [expr_node, rest_nodes @ ..] => {
                let expr = Self::new(arena, src, expr_node)?;
                let deref_expr = arena.alloc(Ir::Deref(expr_node.span(), "__or_internal"));
                let rest_span = rest_nodes.first().unwrap().span().extend_end(span.end);
                let rest_expr =
                    arena.alloc(Self::new_or_expression(arena, src, rest_span, rest_nodes)?);
                let or_branch_expr = Ir::If {
                    span,
                    predicate: deref_expr,
                    true_expr: deref_expr,
                    false_expr: Some(rest_expr),
                };
                Ir::Let {
                    span,
                    bindings: BumpVec::from_iter_in(
                        std::iter::once(("__or_internal", expr)),
                        arena,
                    ),
                    expressions: BumpVec::from_iter_in(std::iter::once(or_branch_expr), arena),
                }
            }
        };
        Ok(ir)
    }

    fn new_and_expression(
        arena: &'a Bump,
        src: &'a str,
        span: Span,
        exprs: &[Node],
    ) -> Result<Ir<'a>> {
        let ir = match exprs {
            [] => Ir::Constant(span, Constant::Bool(true)),
            [expr_node] => Ir::new(arena, src, expr_node)?,
            [expr_node, rest_nodes @ ..] => {
                let expr = Self::new(arena, src, expr_node)?;
                let deref_expr = arena.alloc(Ir::Deref(expr_node.span(), "__and_internal"));
                let rest_span = rest_nodes.first().unwrap().span().extend_end(span.end);
                let rest_expr =
                    arena.alloc(Self::new_and_expression(arena, src, rest_span, rest_nodes)?);
                let and_branch_expr = Ir::If {
                    span,
                    predicate: arena.alloc(Ir::FunctionCall {
                        span: expr_node.span(),
                        function: arena.alloc(Ir::Deref(expr_node.span(), "not")),
                        args: BumpVec::from_iter_in(std::iter::once(deref_expr.clone()), arena),
                    }),
                    true_expr: deref_expr,
                    false_expr: Some(rest_expr),
                };
                Ir::Let {
                    span,
                    bindings: BumpVec::from_iter_in(
                        std::iter::once(("__and_internal", expr)),
                        arena,
                    ),
                    expressions: BumpVec::from_iter_in(std::iter::once(and_branch_expr), arena),
                }
            }
        };
        Ok(ir)
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum IrReturnType {
    /// A value is pushed to the top of the stack.
    Value,
    /// The current function is returned, exiting the current function call frame.
    EarlyReturn,
}

fn node_to_ident<'a>(src: &'a str, node: &Node) -> Result<&'a str> {
    match node {
        Node::Identifier(ident) => Ok(ident.with_src(src).as_str()),
        _ => Err(CompileError::ExpectedIdentifierList {
            context: "lambda/function definition",
        }),
    }
}
