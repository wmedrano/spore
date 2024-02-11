use std::ops::Range;

#[derive(Debug, PartialEq)]
pub struct Token<T> {
    pub item: T,
    pub range: Range<usize>,
}

/// Returns true if the character is a special token.
pub fn is_special_char(ch: char) -> bool {
    matches!(ch, '(' | ')' | '\'' | '"')
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
