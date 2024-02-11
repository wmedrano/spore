use std::ops::Range;

mod lexer;
mod scanner;

/// Converts a string into an iterator of tokens.
pub fn tokenize(s: &str) -> impl '_ + Iterator<Item = Token<TokenType>> {
    lexer::tokenize(s)
}

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
    pub fn map<V>(&self, f: impl Fn(&T) -> V) -> Token<V> {
        let item = f(&self.item);
        Token {
            item,
            range: self.range.clone(),
        }
    }
}
