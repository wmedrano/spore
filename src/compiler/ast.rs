use super::{
    span::Span,
    tokenizer::{tokenize, Token, TokenType},
};

#[derive(Clone, Debug, PartialEq)]
/// Represents an Abstract Syntax Tree.
pub enum Ast {
    /// Represents a tree node with a span and children.
    Tree { span: Span, children: Vec<Ast> },
    /// Represents a leaf node with a span.
    Leaf { span: Span },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
/// Represents an error that can occur during AST building.
pub enum AstError {
    /// Represents an unclosed parenthesis.
    UnclosedParen(Span),
    /// Represents an unexpected close parenthesis.
    UnexpectedCloseParen(Span),
}

impl Ast {
    /// Creates a vector of ASTs from a source string.
    pub fn with_source(source: &str) -> Vec<Self> {
        let mut asts = Vec::new();
        let mut tokens = tokenize(source);
        while let Some(ast) = Ast::next_ast(&mut tokens) {
            asts.push(ast.unwrap());
        }
        asts
    }

    /// Parses the next AST from the token stream.
    fn next_ast(tokens: &mut impl Iterator<Item = Token>) -> Option<Result<Self, AstError>> {
        let token = tokens.next()?;
        match token.token_type {
            TokenType::Identifier => Some(Ok(Ast::Leaf { span: token.span })),
            TokenType::OpenParen => {
                let (sub_span, sub_ast) = match Ast::parse_until_close(token.span.start, tokens) {
                    Ok(x) => x,
                    Err(err) => return Some(Err(err)),
                };
                Some(Ok(Ast::Tree {
                    span: sub_span,
                    children: sub_ast,
                }))
            }
            TokenType::CloseParen => Some(Err(AstError::UnexpectedCloseParen(token.span))),
        }
    }

    /// Parses ASTs until a closing parenthesis is encountered.
    fn parse_until_close(
        span_start: u32,
        tokens: &mut impl Iterator<Item = Token>,
    ) -> Result<(Span, Vec<Self>), AstError> {
        let mut asts = Vec::new();
        while let Some(token) = tokens.next() {
            match token.token_type {
                TokenType::Identifier => asts.push(Ast::Leaf { span: token.span }),
                TokenType::OpenParen => {
                    let (sub_span, sub_ast) = Ast::parse_until_close(token.span.start, tokens)?;
                    asts.push(Ast::Tree {
                        span: sub_span,
                        children: sub_ast,
                    });
                }
                TokenType::CloseParen => {
                    let span = Span {
                        start: span_start,
                        end: token.span.end,
                    };
                    return Ok((span, asts));
                }
            }
        }
        Err(AstError::UnclosedParen(Span {
            start: span_start,
            end: span_start + 1,
        }))
    }
}

impl Ast {
    /// Returns the span of the AST.
    pub fn span(&self) -> Span {
        match self {
            Ast::Leaf { span } => *span,
            Ast::Tree { span, .. } => *span,
        }
    }

    /// Returns the text of the leaf node.
    pub fn leaf_text<'a>(&self, source: &'a str) -> Option<&'a str> {
        match self {
            Ast::Leaf { span } => Some(span.text(source)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_source_has_empty_ast() {
        assert_eq!(Ast::with_source(""), Vec::<Ast>::new(),);
    }
}
