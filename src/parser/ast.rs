use super::token::{Token, TokenType};

/// The abstract syntax tree.
#[derive(Debug, PartialEq)]
pub enum Ast {
    /// Signifies this is a root node.
    Root(Token<AstRoot>),
    /// Signifies that this is a tree with the given children.
    Tree(Vec<Ast>),
}

#[derive(Debug, PartialEq)]
pub enum AstRoot {
    Identifier(String),
    String(String),
    Float(f64),
    Int(isize),
}

/// Holds a parse error.
#[derive(Debug, PartialEq)]
pub enum ParseAstError {
    /// There was a missing closing paren.
    MissingClosingParen { open_idx: usize, end_idx: usize },
    /// No closing paren was expected.
    UnexpectedClosingParen { idx: usize },
    /// The identifier is not valid.
    InvalidIdentifier(Token<String>),
}

impl Ast {
    /// Convert a string into an AST.
    pub fn from_str(s: &str) -> Result<Vec<Ast>, ParseAstError> {
        let tokens = crate::parser::lexer::tokenize(s);
        Ast::from_tokens(tokens)
    }

    /// Convert an iterator of tokens into an AST.
    pub fn from_tokens(
        tokens: impl Iterator<Item = Token<TokenType>>,
    ) -> Result<Vec<Ast>, ParseAstError> {
        let mut tokens = tokens;
        match Ast::from_tokens_impl(&mut tokens, None)? {
            Ast::Root(_) => unreachable!(),
            Ast::Tree(exprs) => Ok(exprs),
        }
    }

    /// Convert an iterator over `Token`s into an `Ast`.
    ///
    ///   - tokens - The iterator of tokens to take as input.
    ///   - opening_paren - The index of the current opening paren. Used to determine if parsing
    ///                     should end at the closing paren. If an open paren is provided but no
    ///                     closing paren is encountered, then an error is returned.
    fn from_tokens_impl(
        tokens: &mut impl Iterator<Item = Token<TokenType>>,
        opening_paren: Option<usize>,
    ) -> Result<Ast, ParseAstError> {
        let mut children = Vec::new();
        let mut end_idx = 0;
        while let Some(token) = tokens.next() {
            end_idx = token.range.end;
            match &token.item {
                TokenType::LeftParen => {
                    let sub_ast = Self::from_tokens_impl(tokens, Some(token.range.start))?;
                    children.push(sub_ast);
                }
                TokenType::RightParen => match opening_paren {
                    Some(_) => return Ok(Ast::Tree(children)),
                    None => {
                        return Err(ParseAstError::UnexpectedClosingParen {
                            idx: token.range.start,
                        })
                    }
                },
                TokenType::Identifier(s) => {
                    // Starting with a double quote indicates that the lexer was unable to parse
                    // this as a valid `TokenType::String`. We report the error here as the lexer
                    // does not have error reporting capabilities.
                    if s.starts_with('"') {
                        return Err(ParseAstError::InvalidIdentifier(Token {
                            item: s.clone(),
                            range: token.range,
                        }));
                    }
                    children.push(Ast::Root(token.with_item(AstRoot::Identifier(s.clone()))));
                }
                TokenType::String(s) => {
                    children.push(Ast::Root(token.with_item(AstRoot::String(s.clone()))));
                }
                TokenType::Int(v) => children.push(Ast::Root(token.with_item(AstRoot::Int(*v)))),
                TokenType::Float(v) => {
                    children.push(Ast::Root(token.with_item(AstRoot::Float(*v))))
                }
            };
        }
        match opening_paren {
            Some(open_idx) => Err(ParseAstError::MissingClosingParen { open_idx, end_idx }),
            None => Ok(Ast::Tree(children)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn parse_multiple() {
        use Ast::*;
        use AstRoot::*;
        assert_eq!(
            Ast::from_str("1 1.0 \"hello\"").unwrap(),
            vec![
                Root(Token {
                    item: Int(1,),
                    range: 0..1,
                }),
                Root(Token {
                    item: Float(1.0),
                    range: 2..5,
                }),
                Root(Token {
                    item: String("hello".to_string()),
                    range: 6..13,
                }),
            ]
        )
    }

    #[test]
    fn parse_list() {
        use Ast::*;
        use AstRoot::*;
        assert_eq!(
            Ast::from_str("(1 2 3)").unwrap(),
            vec![Tree(vec![
                Root(Token {
                    item: Int(1,),
                    range: 1..2,
                }),
                Root(Token {
                    item: Int(2,),
                    range: 3..4,
                }),
                Root(Token {
                    item: Int(3,),
                    range: 5..6,
                }),
            ])]
        )
    }

    #[test]
    fn empty_list() {
        assert_eq!(Ast::from_str("()").unwrap(), vec![Ast::Tree(vec![])])
    }

    #[test]
    fn nested_lists() {
        use Ast::*;
        use AstRoot::*;
        assert_eq!(
            Ast::from_str("(1 2 (3 4) (5))").unwrap(),
            vec![Tree(vec![
                Root(Token {
                    item: Int(1),
                    range: 1..2
                }),
                Root(Token {
                    item: Int(2),
                    range: 3..4,
                }),
                Tree(vec![
                    Root(Token {
                        item: Int(3),
                        range: 6..7,
                    }),
                    Root(Token {
                        item: Int(4),
                        range: 8..9,
                    }),
                ]),
                Tree(vec![Root(Token {
                    item: Int(5),
                    range: 12..13,
                }),]),
            ])]
        )
    }

    #[test]
    fn no_matching_paren_is_err() {
        assert_eq!(
            Ast::from_str(" (1 2").unwrap_err(),
            ParseAstError::MissingClosingParen {
                open_idx: 1,
                end_idx: 5
            },
        )
    }

    #[test]
    fn unexpected_closing_paren_is_err() {
        assert_eq!(
            Ast::from_str("(1 2)) (2").unwrap_err(),
            ParseAstError::UnexpectedClosingParen { idx: 5 },
        );
    }

    #[test]
    fn no_closing_quote_is_err() {
        assert_eq!(
            Ast::from_str("\"hello").unwrap_err(),
            ParseAstError::InvalidIdentifier(Token {
                item: "\"hello".to_string(),
                range: 0..6,
            }),
        );
    }
}
