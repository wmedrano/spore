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
            CharType::Hashtag => match chars.peek().cloned() {
                Some((_, '|')) => {
                    scan_until_end_block_comment(&mut chars);
                    let end = chars.peek().map(|(idx, _)| *idx).unwrap_or(text.len());
                    return Some(Token::new_from_source(text, start..end));
                }
                Some((_, ';')) => {
                    chars.next();
                    let end = chars.peek().map(|(idx, _)| *idx).unwrap_or(text.len());
                    return Some(Token::new_from_source(text, start..end));
                }
                _ => MatchType::None,
            },
            CharType::Comment => {
                let mut end = start + 1;
                while let Some((_, ch)) = chars.next() {
                    end += 1;
                    if ch == '\n' {
                        break;
                    }
                }
                return Some(Token::new_from_source(text, start..end));
            }
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
                CharType::Comment => {
                    if matches!(
                        initial_char_type,
                        MatchType::Quote | MatchType::IdentifierQuote
                    ) {
                        chars.next().unwrap();
                    } else {
                        break;
                    }
                }
                CharType::Hashtag => {
                    chars.next().unwrap();
                    if text.get(idx - 1..idx) == Some("|") {
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
    Comment,
    Hashtag,
    Other(char),
}

/// Returns the type of character ch is or `None` if it is a normal character.
fn classify_char(ch: char) -> CharType {
    match ch {
        '(' => CharType::LeftParen,
        ')' => CharType::RightParen,
        '"' => CharType::Quote,
        '|' => CharType::IdentifierQuote,
        ';' => CharType::Comment,
        '#' => CharType::Hashtag,
        ch if ch.is_whitespace() => CharType::Whitespace,
        ch => CharType::Other(ch),
    }
}

fn scan_until_end_block_comment(chs: &mut impl Iterator<Item = (usize, char)>) {
    let mut previous = ' ';
    while let Some((_, ch)) = chs.next() {
        if previous == '|' && ch == '#' {
            return;
        }
        previous = ch;
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

    #[test]
    fn comments() {
        assert_eq!(
            scan_to_vec("(define x 12) ; Everything \"here\" is a comment.\n ; And here \n (not-here) ; not before here"),
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
                    item: "x",
                    range: 8..9,
                },
                Token {
                    item: "12",
                    range: 10..12,
                },
                Token {
                    item: ")",
                    range: 12..13,
                },
                Token {
                    item: "; Everything \"here\" is a comment.\n",
                    range: 14..48,
                },
                Token {
                    item: "; And here \n",
                    range: 49..61,
                },
                Token {
                    item: "(",
                    range: 62..63,
                },
                Token {
                    item: "not-here",
                    range: 63..71,
                },
                Token {
                    item: ")",
                    range: 71..72,
                },
                Token {
                    item: "; not before here",
                    range: 73..90,
                }]);
    }

    #[test]
    fn block_comment_is_parsed() {
        assert_eq!(
            scan_to_vec("(def hello #|world\ndoes\nnot\nyet\nexist|#42)"),
            vec![
                Token {
                    item: "(",
                    range: 0..1,
                },
                Token {
                    item: "def",
                    range: 1..4,
                },
                Token {
                    item: "hello",
                    range: 5..10,
                },
                Token {
                    item: "#|world\ndoes\nnot\nyet\nexist|#",
                    range: 11..39,
                },
                Token {
                    item: "42",
                    range: 39..41,
                },
                Token {
                    item: ")",
                    range: 41..42,
                },
            ]
        );
    }

    #[test]
    fn datum_comment() {
        assert_eq!(
            scan_to_vec("1 #;2 #; 3 4"),
            vec![
                Token {
                    item: "1",
                    range: 0..1,
                },
                Token {
                    item: "#;",
                    range: 2..4,
                },
                Token {
                    item: "2",
                    range: 4..5,
                },
                Token {
                    item: "#;",
                    range: 6..8,
                },
                Token {
                    item: "3",
                    range: 9..10,
                },
                Token {
                    item: "4",
                    range: 11..12,
                },
            ]
        );
    }
}
