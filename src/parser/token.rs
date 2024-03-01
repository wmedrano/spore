use std::ops::Range;

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
    Comment(String),
    CommentDatum,
    String(String),
    Int(isize),
    Float(f64),
    Bool(bool),
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
        self.with_item(item)
    }

    pub fn with_item<V>(&self, v: V) -> Token<V> {
        Token {
            item: v,
            range: self.range.clone(),
        }
    }
}
