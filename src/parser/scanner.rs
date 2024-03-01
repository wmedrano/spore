use super::token::Token;

/// Scan text and return a stream of tokens.
pub fn scan(text: &str) -> impl Iterator<Item = Token<&'_ str>> {
    let mut chars = text.char_indices().peekable();
    std::iter::from_fn(move || {
        // 1. Skip all the whitespace.
        while chars.next_if(|(_, ch)| ch.is_whitespace()).is_some() {}
        // 2. Get the start position or return immediately if it is paren.
        let (start, initial_char) = chars.next()?;
        #[derive(Copy, Clone, PartialEq)]
        enum MatchType {
            None,
            Quote,
            IdentifierQuote,
        }
        let initial_char_type = match classify_char(initial_char) {
            CharType::LeftParen | CharType::RightParen => {
                return Some(Token::new_from_source(text, start..start + 1))
            }
            CharType::Quote => MatchType::Quote,
            CharType::IdentifierQuote => MatchType::IdentifierQuote,
            CharType::Other(_) => MatchType::None,
            CharType::Whitespace => unreachable!(),
        };
        // 3. Eat until a whitespace or special character.
        while let Some((idx, ch)) = chars.peek().copied() {
            match classify_char(ch) {
                CharType::Whitespace | CharType::LeftParen | CharType::RightParen => {
                    if matches!(
                        initial_char_type,
                        MatchType::Quote | MatchType::IdentifierQuote
                    ) {
                        chars.next().unwrap();
                    } else {
                        break;
                    }
                }
                CharType::Quote => {
                    if &text[idx - 1..idx] == "\\"
                        || initial_char_type == MatchType::IdentifierQuote
                    {
                        chars.next().unwrap();
                    } else {
                        if matches!(initial_char_type, MatchType::Quote) {
                            chars.next().unwrap();
                        }
                        break;
                    }
                }
                CharType::IdentifierQuote => {
                    chars.next().unwrap();
                    if initial_char_type == MatchType::IdentifierQuote {
                        break;
                    }
                }
                CharType::Other(_) => {
                    chars.next().unwrap();
                }
            }
        }
        // 4. Get the end and format the return value.
        let end = chars.peek().map(|(idx, _)| *idx).unwrap_or(text.len());
        Some(Token::new_from_source(text, start..end))
    })
}

#[derive(PartialEq, Debug)]
enum CharType {
    Whitespace,
    LeftParen,
    RightParen,
    Quote,
    IdentifierQuote,
    Other(char),
}

/// Returns the type of character ch is or `None` if it is a normal character.
fn classify_char(ch: char) -> CharType {
    match ch {
        '(' => CharType::LeftParen,
        ')' => CharType::RightParen,
        '"' => CharType::Quote,
        '|' => CharType::IdentifierQuote,
        ch if ch.is_whitespace() => CharType::Whitespace,
        ch => CharType::Other(ch),
    }
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
    fn parens_are_their_own_token() {
        assert_eq!(
            scan_to_vec(")this-is-an-exp()"),
            vec![
                Token {
                    item: ")",
                    range: 0..1,
                },
                Token {
                    item: "this-is-an-exp",
                    range: 1..15,
                },
                Token {
                    item: "(",
                    range: 15..16,
                },
                Token {
                    item: ")",
                    range: 16..17,
                },
            ]
        );
    }

    #[test]
    fn string_is_parsed_as_single_token() {
        assert_eq!(
            scan_to_vec("\"this is a string\""),
            vec![Token {
                item: "\"this is a string\"",
                range: 0..18
            },]
        );
    }

    #[test]
    fn vertical_bars_contain_identifiers() {
        assert_eq!(
            scan_to_vec("(define |identifier with \"spaces\"| 4)"),
            vec![
                Token {
                    item: "(",
                    range: 0..1,
                },
                Token {
                    item: "define",
                    range: 1..7,
                },
                Token {
                    item: "|identifier with \"spaces\"|",
                    range: 8..34,
                },
                Token {
                    item: "4",
                    range: 35..36,
                },
                Token {
                    item: ")",
                    range: 36..37,
                },
            ]
        );
    }
}
