use compact_str::CompactString;
use thiserror::Error;

use super::{
    span::Span,
    tokenizer::{Token, TokenType},
};

type Result<T> = std::result::Result<T, AstParseError>;

/// Describes an AST parsing error.
#[derive(Debug, Error, PartialEq)]
pub enum AstParseError {
    #[error("opening parenthesis was unclosed")]
    UnclosedParen,
    #[error("found unexpected closing parenthesis")]
    UnexpectedCloseParen,
    #[error("string was not properly closed, did you forget \"?")]
    UnclosedString(Span),
}

/// Describes a node in the AST.
#[derive(Debug, PartialEq)]
pub enum Node {
    /// A node containing the start and end points of the identifier.
    Identifier(Span),
    /// A node containing the void literal.
    Void(Span),
    /// A node containing a boolean literal.
    Bool(Span, bool),
    /// A node containing an int literal.
    Int(Span, i64),
    /// A node containing a float literal.
    Float(Span, f64),
    /// A node containing a string literal.
    String(Span),
    /// A node containing a sub-tree.
    Tree(Span, Vec<Node>),
}

impl Node {
    /// Parse the contents of `src` into a stream of `Node`.
    pub fn parse(src: &str) -> impl '_ + Iterator<Item = Result<Self>> {
        let mut tokens = Token::parse_tokens(src);
        std::iter::from_fn(move || Node::parse_next(src, &mut tokens))
    }
}

impl Node {
    /// Parse the contents of `src` into a vector of `Node`.
    ///
    /// For unit testing only, prefer using `Node::parse`.
    #[cfg(test)]
    pub fn parse_to_vec(src: &str) -> Result<Vec<Node>> {
        Node::parse(src).collect()
    }

    /// Parse the next Node from `tokenizer`.
    fn parse_next(src: &str, tokenizer: &mut impl Iterator<Item = Token>) -> Option<Result<Node>> {
        while let Some(next_token) = tokenizer.next() {
            match next_token.token_type {
                TokenType::OpenParen => match Node::parse_until_close(src, tokenizer) {
                    Ok((end, tree)) => {
                        let span = next_token.span.extend_end(end);
                        return Some(Ok(Node::Tree(span, tree)));
                    }
                    Err(err) => return Some(Err(err)),
                },
                TokenType::CloseParen => return Some(Err(AstParseError::UnexpectedCloseParen)),
                TokenType::UnterminatedString => {
                    return Some(Err(AstParseError::UnclosedString(next_token.span)))
                }
                TokenType::String | TokenType::Other => {
                    return Some(Ok(Node::parse_atom(next_token, next_token.as_str(src))))
                }
                TokenType::Comment => continue,
            }
        }
        None
    }

    /// Parse the nodes in `tokenizer` until a closing parenthesis is encountered.
    ///
    /// An error is returned if no closing parenthesis is ever encountered.
    fn parse_until_close(
        src: &str,
        tokenizer: &mut impl Iterator<Item = Token>,
    ) -> Result<(u32, Vec<Node>)> {
        let mut tree = vec![];
        while let Some(next_token) = tokenizer.next() {
            match next_token.token_type {
                TokenType::OpenParen => match Node::parse_until_close(src, tokenizer) {
                    Ok((end, t)) => {
                        let span = next_token.span.extend_end(end);
                        tree.push(Node::Tree(span, t))
                    }
                    err @ Err(_) => return err,
                },
                TokenType::CloseParen => return Ok((next_token.span.end, tree)),
                TokenType::UnterminatedString => {
                    return Err(AstParseError::UnclosedString(next_token.span))
                }
                TokenType::String | TokenType::Other => {
                    tree.push(Node::parse_atom(next_token, next_token.as_str(src)))
                }
                TokenType::Comment => continue,
            }
        }
        Err(AstParseError::UnclosedParen)
    }

    /// Returns the string literal contained in the node or `None` if `self` is not a
    /// [Node::String].
    pub fn to_string_literal(&self, src: &str) -> Option<CompactString> {
        let contents = match self {
            Node::String(span) => span.with_src(src).as_str(),
            _ => return None,
        };
        let mut res = CompactString::with_capacity(contents.len().saturating_sub(2));
        let mut escaped = false;
        for ch in contents[1..contents.len() - 1].chars() {
            if escaped {
                let ch = match ch {
                    'n' => '\n',
                    't' => '\t',
                    ch => ch,
                };
                res.push(ch);
                escaped = false;
            } else {
                match ch {
                    '\\' => escaped = true,
                    // Not a well formed string.
                    '"' => return None,
                    ch => res.push(ch),
                }
            }
        }
        Some(res)
    }

    /// Parse `contents` as if it were an atom. Panics if `token_type` does not correspond to an
    /// atom.
    fn parse_atom(token: Token, contents: &str) -> Node {
        match token.token_type {
            TokenType::OpenParen
            | TokenType::CloseParen
            | TokenType::UnterminatedString
            | TokenType::Comment => {
                // Unreachable OK: The above scenarios are caught by callers of `parse_atom`.
                unreachable!()
            }
            TokenType::String => Node::String(token.span),
            TokenType::Other => {
                let maybe_is_number = contents
                    .chars()
                    .next()
                    .map(|ch| {
                        if ch.is_ascii_digit() {
                            return true;
                        }
                        contents.len() > 1 && matches!(ch, '-' | '+')
                    })
                    .unwrap_or(false);
                if maybe_is_number {
                    if let Ok(int) = contents.parse() {
                        return Node::Int(token.span, int);
                    } else if let Ok(float) = contents.parse() {
                        return Node::Float(token.span, float);
                    }
                }
                match contents {
                    "void" => Node::Void(token.span),
                    "true" => Node::Bool(token.span, true),
                    "false" => Node::Bool(token.span, false),
                    _ => Node::Identifier(token.span),
                }
            }
        }
    }

    pub fn span(&self) -> Span {
        match self {
            Node::Identifier(s) => *s,
            Node::Void(s) => *s,
            Node::Bool(s, _) => *s,
            Node::Int(s, _) => *s,
            Node::Float(s, _) => *s,
            Node::String(s) => *s,
            Node::Tree(s, _) => *s,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whitespace_returns_no_nodes() {
        let src = " \t\n ";
        let actual = Node::parse_to_vec(src).unwrap();
        assert_eq!(actual, vec![]);
    }

    #[test]
    fn atoms_are_parsed() {
        let src = "1 2.0 three \"four\" true false";
        let actual = Node::parse_to_vec(src).unwrap();
        assert_eq!(
            actual,
            vec![
                Node::Int(Span::new(0, 1), 1),
                Node::Float(Span::new(2, 5), 2.0),
                Node::Identifier(Span::new(6, 11)),
                Node::String(Span::new(12, 18)),
                Node::Bool(Span::new(19, 23), true),
                Node::Bool(Span::new(24, 29), false),
            ]
        );
    }

    #[test]
    fn expression_is_parsed_as_tree() {
        let src = "(+ 1 (- a b) \"number\") (str-len \"str\") \"atom\"";
        let actual = Node::parse_to_vec(src).unwrap();
        assert_eq!(
            actual,
            vec![
                Node::Tree(
                    Span::new(0, 22),
                    vec![
                        Node::Identifier(Span::new(1, 2)),
                        Node::Int(Span::new(3, 4), 1),
                        Node::Tree(
                            Span::new(5, 12),
                            vec![
                                Node::Identifier(Span::new(6, 7)),
                                Node::Identifier(Span::new(8, 9)),
                                Node::Identifier(Span::new(10, 11))
                            ]
                        ),
                        Node::String(Span::new(13, 21)),
                    ]
                ),
                Node::Tree(
                    Span::new(23, 38),
                    vec![
                        Node::Identifier(Span::new(24, 31)),
                        Node::String(Span::new(32, 37))
                    ]
                ),
                Node::String(Span::new(39, 45)),
            ]
        );
    }

    #[test]
    fn quoted_strings_within_strings_are_preserved() {
        let src = "\"this \\\"is\\\" a string\"";
        let actual = Node::parse_to_vec(src).unwrap();
        assert_eq!(actual, vec![Node::String(Span::new(0, 22))]);
    }

    #[test]
    fn backslash_with_n_returns_newline() {
        let src = r#""\nn\n""#;
        let actual = Node::parse_to_vec(src).unwrap();
        assert_eq!(actual, vec![Node::String(Span::new(0, 7))]);
        assert_eq!(actual[0].to_string_literal(src).unwrap(), "\nn\n");
    }

    #[test]
    fn backslash_with_t_returns_tab() {
        let src = "\"\\tt\\t\"";
        let actual = Node::parse_to_vec(src).unwrap();
        assert_eq!(actual, vec![Node::String(Span::new(0, 7))]);
        assert_eq!(actual[0].to_string_literal(src).unwrap(), "\tt\t");
    }

    #[test]
    fn backslash_with_backslash_returns_backslash() {
        let src = r#""\\""#;
        let actual = Node::parse_to_vec(src).unwrap();
        assert_eq!(actual, vec![Node::String(Span::new(0, 4))]);
        assert_eq!(actual[0].to_string_literal(src).unwrap(), "\\");
    }

    #[test]
    fn unclosed_paren_returns_error() {
        let src = "(not closed";
        let actual_err = Node::parse_to_vec(src).unwrap_err();
        assert_eq!(actual_err, AstParseError::UnclosedParen);
    }

    #[test]
    fn unexpected_close_paren_returns_error() {
        let src = "not closed)";
        let actual_err = Node::parse_to_vec(src).unwrap_err();
        assert_eq!(actual_err, AstParseError::UnexpectedCloseParen);
    }

    #[test]
    fn unterminated_string_returns_error() {
        let src = "\"start of string but no end";
        let actual_err = Node::parse_to_vec(src).unwrap_err();
        assert_eq!(actual_err, AstParseError::UnclosedString(Span::new(0, 27)));
    }

    #[test]
    fn error_in_subexpression_is_returned() {
        let src = "(((\"unterminated quote)";
        let actual_err = Node::parse_to_vec(src).unwrap_err();
        assert_eq!(actual_err, AstParseError::UnclosedString(Span::new(3, 23)));
    }

    #[test]
    fn hacks_for_code_coverage() {
        // There is not much value in testing this so calling function to appease code coverage
        // tool.
        AstParseError::UnclosedParen.to_string();
    }
}
