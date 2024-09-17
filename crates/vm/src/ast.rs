use thiserror::Error;

use crate::tokenizer::{Token, TokenType};

type Result<T> = std::result::Result<T, AstParseError>;

/// Describes an AST parsing error.
#[derive(Debug, Error, PartialEq)]
pub enum AstParseError {
    #[error("opening parenthesis was unclosed")]
    UnclosedParen,
    #[error("found unexpected closing parenthesis")]
    UnexpectedCloseParen,
    #[error("string was not properly closed, did you forget \"?")]
    UnclosedString(usize),
}

/// Describes a node in the AST.
#[derive(Debug, PartialEq)]
pub enum Node<'a> {
    /// A node containing an identifier.
    Identifier(&'a str),
    /// A node containing a boolean literal.
    Bool(bool),
    /// A node containing an int literal.
    Int(i64),
    /// A node containing a float literal.
    Float(f64),
    /// A node containing a string literal.
    String(String),
    /// A node containing a sub-tree.
    Tree(Vec<Node<'a>>),
}

impl<'a> Node<'a> {
    /// Parse the contents of `src` into a stream of `Node`.
    pub fn parse(src: &'a str) -> impl Iterator<Item = Result<Self>> {
        let mut tokens = Token::parse_tokens(src);
        std::iter::from_fn(move || Node::parse_next(src, &mut tokens))
    }
}

impl<'a> Node<'a> {
    /// Parse the contents of `src` into a vector of `Node`.
    ///
    /// For unit testing only, prefer using `Node::parse`.
    #[cfg(test)]
    pub fn parse_to_vec(src: &'a str) -> Result<Vec<Node<'a>>> {
        Node::parse(src).collect()
    }

    /// Parse the next Node from `tokenizer`.
    fn parse_next(
        src: &'a str,
        tokenizer: &mut impl Iterator<Item = Token>,
    ) -> Option<Result<Node<'a>>> {
        let next_token = match tokenizer.next() {
            Some(t) => t,
            None => return None,
        };
        match next_token.token_type {
            TokenType::OpenParen => match Node::parse_until_close(src, tokenizer) {
                Ok(tree) => Some(Ok(Node::Tree(tree))),
                Err(err) => Some(Err(err)),
            },
            TokenType::CloseParen => Some(Err(AstParseError::UnexpectedCloseParen)),
            TokenType::UnterminatedString => {
                Some(Err(AstParseError::UnclosedString(next_token.start)))
            }
            TokenType::String | TokenType::Other => {
                return Some(Ok(Node::parse_atom(
                    next_token.token_type,
                    next_token.as_str(src),
                )))
            }
        }
    }

    /// Parse the nodes in `tokenizer` until a closing parenthesis is encountered.
    ///
    /// An error is returned if no closing parenthesis is ever encountered.
    fn parse_until_close(
        src: &'a str,
        tokenizer: &mut impl Iterator<Item = Token>,
    ) -> Result<Vec<Node<'a>>> {
        let mut tree = vec![];
        loop {
            let next_token = match tokenizer.next() {
                Some(t) => t,
                None => {
                    return Err(AstParseError::UnclosedParen);
                }
            };
            match next_token.token_type {
                TokenType::OpenParen => match Node::parse_until_close(src, tokenizer) {
                    Ok(t) => tree.push(Node::Tree(t)),
                    err @ Err(_) => return err,
                },
                TokenType::CloseParen => return Ok(tree),
                TokenType::UnterminatedString => {
                    return Err(AstParseError::UnclosedString(next_token.start))
                }
                TokenType::String | TokenType::Other => tree.push(Node::parse_atom(
                    next_token.token_type,
                    next_token.as_str(src),
                )),
            }
        }
    }

    /// Parse `contents` as if it were an atom. Panics if `token_type` does not correspond to an
    /// atom.
    fn parse_atom(token_type: TokenType, contents: &'a str) -> Node<'a> {
        match token_type {
            TokenType::OpenParen | TokenType::CloseParen | TokenType::UnterminatedString => {
                // Unreachable OK: The above scenarios are caught by callers of `parse_atom`.
                unreachable!()
            }
            TokenType::String => {
                let mut res = String::with_capacity(contents.len().saturating_sub(2));
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
                            // Unreachable OK: An unescaped quote signals the end of the
                            // string. This token is guaranteed to be a well formed string so a
                            // naked quote won't be encountered in the middle of the string.
                            '"' => unreachable!(),
                            ch => res.push(ch),
                        }
                    }
                }
                Node::String(res)
            }
            TokenType::Other => {
                if let Ok(int) = contents.parse() {
                    return Node::Int(int);
                } else if let Ok(float) = contents.parse() {
                    return Node::Float(float);
                } else if contents == "true" {
                    return Node::Bool(true);
                } else if contents == "false" {
                    return Node::Bool(false);
                } else {
                    return Node::Identifier(contents);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whitespace_produces_no_nodes() {
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
                Node::Int(1),
                Node::Float(2.0),
                Node::Identifier("three"),
                Node::String("four".to_string()),
                Node::Bool(true),
                Node::Bool(false),
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
                Node::Tree(vec![
                    Node::Identifier("+"),
                    Node::Int(1),
                    Node::Tree(vec![
                        Node::Identifier("-"),
                        Node::Identifier("a"),
                        Node::Identifier("b")
                    ]),
                    Node::String("number".to_string()),
                ]),
                Node::Tree(vec![
                    Node::Identifier("str-len"),
                    Node::String("str".to_string())
                ]),
                Node::String("atom".to_string()),
            ]
        );
    }

    #[test]
    fn quoted_strings_within_strings_are_preserved() {
        let src = "\"this \\\"is\\\" a string\"";
        let actual = Node::parse_to_vec(src).unwrap();
        assert_eq!(
            actual,
            vec![Node::String("this \"is\" a string".to_string())]
        );
    }

    #[test]
    fn backslash_with_n_produces_newline() {
        let src = "\"\\nn\\n\"";
        let actual = Node::parse_to_vec(src).unwrap();
        assert_eq!(actual, vec![Node::String("\nn\n".to_string())]);
    }

    #[test]
    fn backslash_with_t_produces_tab() {
        let src = "\"\\tt\\t\"";
        let actual = Node::parse_to_vec(src).unwrap();
        assert_eq!(actual, vec![Node::String("\tt\t".to_string())]);
    }

    #[test]
    fn unclosed_paren_produces_error() {
        let src = "(not closed";
        let actual_err = Node::parse_to_vec(src).unwrap_err();
        assert_eq!(actual_err, AstParseError::UnclosedParen);
    }

    #[test]
    fn unexpected_close_paren_produces_error() {
        let src = "not closed)";
        let actual_err = Node::parse_to_vec(src).unwrap_err();
        assert_eq!(actual_err, AstParseError::UnexpectedCloseParen);
    }

    #[test]
    fn unterminated_string_produces_error() {
        let src = "\"start of string but no end";
        let actual_err = Node::parse_to_vec(src).unwrap_err();
        assert_eq!(actual_err, AstParseError::UnclosedString(0));
    }

    #[test]
    fn error_in_subexpression_is_returned() {
        let src = "(((\"unterminated quote)";
        let actual_err = Node::parse_to_vec(src).unwrap_err();
        assert_eq!(actual_err, AstParseError::UnclosedString(3));
    }
}
