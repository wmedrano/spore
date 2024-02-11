use std::ops::Range;

mod lexer;
mod scanner;

/// A token that holds the type and the place in the text where it occurred.
#[derive(Debug, PartialEq)]
pub struct Token<T> {
    /// The item.
    pub item: T,
    /// The location for where the token occurred.
    pub range: Range<usize>,
}

#[derive(Debug, PartialEq)]
pub enum TokenType {
    LeftParen,
    RightParen,
    Identifier(String),
    String(String),
    Int(isize),
    Float(f64),
}

impl<'a> Token<&'a str> {
    /// Create a new `&'a str` token from source text for the given range.
    pub fn new_from_source(source: &'a str, range: Range<usize>) -> Token<&'a str> {
        Token {
            item: &source[range.clone()],
            range,
        }
    }
}

impl<T> Token<T> {
    /// Transform the item using function `f`.
    pub fn map<V>(&self, f: impl Fn(&T) -> V) -> Token<V> {
        let item = f(&self.item);
        Token {
            item,
            range: self.range.clone(),
        }
    }
}

/// The abstract syntax tree.
#[derive(Debug, PartialEq)]
pub enum Ast {
    /// Signifies this is a root node.
    Root(Token<TokenType>),
    /// Signifies that this is a tree with the given children.
    Tree(Vec<Ast>),
}

impl Ast {
    pub fn from_str(s: &str) -> Vec<Ast> {
        let tokens = lexer::tokenize(s);
        Ast::from_tokens(tokens)
    }

    pub fn from_tokens(tokens: impl Iterator<Item = Token<TokenType>>) -> Vec<Ast> {
        let mut tokens = tokens;
        match Ast::from_tokens_impl(&mut tokens, false) {
            Ast::Root(_) => unreachable!(),
            Ast::Tree(exprs) => exprs,
        }
    }

    fn from_tokens_impl(
        tokens: &mut impl Iterator<Item = Token<TokenType>>,
        find_matching_paren: bool,
    ) -> Ast {
        let mut children = Vec::new();
        while let Some(token) = tokens.next() {
            match token.item {
                TokenType::LeftParen => children.push(Self::from_tokens_impl(tokens, true)),
                TokenType::RightParen => {
                    if find_matching_paren {
                        return Ast::Tree(children);
                    } else {
                        todo!("Parse error, found unexpected closing ) paren.")
                    }
                }
                TokenType::Identifier(_)
                | TokenType::String(_)
                | TokenType::Int(_)
                | TokenType::Float(_) => children.push(Ast::Root(token)),
            };
        }
        if find_matching_paren {
            todo!("Parse error, found no closing ) paren.")
        }
        Ast::Tree(children)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn parse_multiple() {
        use Ast::*;
        use TokenType::*;
        assert_eq!(
            Ast::from_str("1 1.0 \"hello\""),
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
        use TokenType::*;
        assert_eq!(
            Ast::from_str("(1 2 3)"),
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
        assert_eq!(Ast::from_str("()"), vec![Ast::Tree(vec![])])
    }

    #[test]
    fn nested_lists() {
        use Ast::*;
        use TokenType::*;
        assert_eq!(
            Ast::from_str("(1 2 (3 4) (5))"),
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
}
