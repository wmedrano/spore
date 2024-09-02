use tokenizer::Token;

mod tokenizer;

#[derive(Debug, PartialEq)]
pub enum Node<'a> {
    Identifier(&'a str),
    Int(i64),
    Float(f64),
    String(String),
    Tree(Vec<Node<'a>>),
}

#[derive(Debug, PartialEq)]
pub enum AstParseError {
    UnclosedParen,
    UnexpectedCloseParen,
    UnclosedString,
}

type Result<T> = std::result::Result<T, AstParseError>;

impl<'a> Node<'a> {
    pub fn parse(src: &'a str) -> impl Iterator<Item = Result<Self>> {
        let mut tokens = Token::parse_tokens(src);
        std::iter::from_fn(move || Node::parse_next(&mut tokens))
    }
}

impl<'a> Node<'a> {
    #[cfg(test)]
    pub fn parse_to_vec(src: &'a str) -> Result<Vec<Node<'a>>> {
        Node::parse(src).collect()
    }

    fn parse_next(tokenizer: &mut impl Iterator<Item = Token<'a>>) -> Option<Result<Node<'a>>> {
        let next_token = match tokenizer.next() {
            Some(t) => t,
            None => return None,
        };
        match next_token.0 {
            tokenizer::TokenType::OpenParen => match Node::parse_until_close(tokenizer) {
                Ok(tree) => Some(Ok(Node::Tree(tree))),
                Err(err) => return Some(Err(err)),
            },
            tokenizer::TokenType::CloseParen => {
                return Some(Err(AstParseError::UnexpectedCloseParen))
            }
            tokenizer::TokenType::UnterminatedString => {
                return Some(Err(AstParseError::UnclosedString))
            }
            tokenizer::TokenType::String | tokenizer::TokenType::Other => {
                return Some(Ok(Node::parse_atom(next_token.0, next_token.1)))
            }
        }
    }

    fn parse_until_close(tokenizer: &mut impl Iterator<Item = Token<'a>>) -> Result<Vec<Node<'a>>> {
        let mut tree = vec![];
        loop {
            let next_token = match tokenizer.next() {
                Some(t) => t,
                None => {
                    return Err(AstParseError::UnclosedParen);
                }
            };
            match next_token.0 {
                tokenizer::TokenType::OpenParen => match Node::parse_until_close(tokenizer) {
                    Ok(t) => tree.push(Node::Tree(t)),
                    err @ Err(_) => return err,
                },
                tokenizer::TokenType::CloseParen => return Ok(tree),
                tokenizer::TokenType::UnterminatedString => {
                    return Err(AstParseError::UnclosedString)
                }
                tokenizer::TokenType::String | tokenizer::TokenType::Other => {
                    tree.push(Node::parse_atom(next_token.0, next_token.1))
                }
            }
        }
    }

    fn parse_atom(token_type: tokenizer::TokenType, contents: &'a str) -> Node<'a> {
        match token_type {
            tokenizer::TokenType::OpenParen
            | tokenizer::TokenType::CloseParen
            | tokenizer::TokenType::UnterminatedString => unreachable!(),
            tokenizer::TokenType::String => {
                let mut res = String::with_capacity(contents.len().saturating_sub(2));
                let mut escaped = false;
                for ch in contents[1..contents.len() - 1].chars() {
                    if escaped {
                        res.push(ch);
                        escaped = false;
                    } else {
                        match ch {
                            '\\' => escaped = true,
                            '"' => unreachable!(),
                            ch => res.push(ch),
                        }
                    }
                }
                Node::String(res)
            }
            tokenizer::TokenType::Other => {
                if let Ok(int) = contents.parse() {
                    return Node::Int(int);
                } else if let Ok(float) = contents.parse() {
                    return Node::Float(float);
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
        let src = "1 2.0 three \"four\"";
        let actual = Node::parse_to_vec(src).unwrap();
        assert_eq!(
            actual,
            vec![
                Node::Int(1),
                Node::Float(2.0),
                Node::Identifier("three"),
                Node::String("four".to_string()),
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
        assert_eq!(actual_err, AstParseError::UnclosedString);
    }
}
