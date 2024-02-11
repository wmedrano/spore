use crate::token::{is_special_char, Token};

/// Scan text and return a stream of tokens.
pub fn scan(text: &str) -> impl Iterator<Item = Token<&'_ str>> {
    let mut chars = text.char_indices().peekable();
    std::iter::from_fn(move || {
        // 1. Skip all the whitespace.
        while let Some(_) = chars.next_if(|(_, ch)| ch.is_whitespace()) {}
        // 2. Get the start position or return immediately if it is a special token.
        let start = match chars.next()? {
            (idx, ch) if is_special_char(ch) => {
                return Some(Token::new_from_source(text, idx..idx + 1))
            }
            (idx, _) => idx,
        };
        // 3. Eat until a whitespace or special character.
        while let Some((_, ch)) = chars.peek() {
            match *ch {
                ch if ch.is_whitespace() || is_special_char(ch) => break,
                _ => {
                    chars.next().unwrap();
                }
            }
        }
        // 4. Get the end and format the return value.
        let end = chars.peek().map(|(idx, _)| *idx).unwrap_or(text.len());
        Some(Token::new_from_source(text, start..end))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn scan_to_vec(text: &str) -> Vec<Token<&'_ str>> {
        scan(text).collect()
    }

    #[test]
    fn word_is_parsed() {
        assert_eq!(
            scan_to_vec("word"),
            vec![Token {
                item: "word",
                range: 0..4
            }]
        );
    }

    #[test]
    fn multiple_words_are_parsed_as_separate_tokens() {
        assert_eq!(
            scan_to_vec("first second third"),
            vec![
                Token {
                    item: "first",
                    range: 0..5
                },
                Token {
                    item: "second",
                    range: 6..12
                },
                Token {
                    item: "third",
                    range: 13..18
                },
            ]
        );
    }

    #[test]
    fn whitespace_is_ignored() {
        assert_eq!(
            scan_to_vec("\t second "),
            vec![Token {
                item: "second",
                range: 2..8
            },]
        );
    }

    #[test]
    fn special_characters_are_their_own_token() {
        assert_eq!(
            scan_to_vec("(hello )\"world'here  )"),
            vec![
                Token {
                    item: "(",
                    range: 0..1,
                },
                Token {
                    item: "hello",
                    range: 1..6,
                },
                Token {
                    item: ")",
                    range: 7..8
                },
                Token {
                    item: "\"",
                    range: 8..9
                },
                Token {
                    item: "world",
                    range: 9..14,
                },
                Token {
                    item: "'",
                    range: 14..15,
                },
                Token {
                    item: "here",
                    range: 15..19,
                },
                Token {
                    item: ")",
                    range: 21..22,
                },
            ]
        );
    }
}
