#[derive(Debug, PartialEq)]
pub enum TokenType {
    OpenParen,
    CloseParen,
    String,
    UnterminatedString,
    Other,
}

#[derive(Debug, PartialEq)]
pub struct Token<'a>(pub TokenType, pub &'a str);

impl<'a> Token<'a> {
    pub fn parse_tokens(input_source: &'a str) -> impl Iterator<Item = Token<'a>> {
        let mut input_source = input_source;
        std::iter::from_fn(move || match Token::parse_next(input_source) {
            None => None,
            Some((next_str, token)) => {
                input_source = next_str;
                Some(token)
            }
        })
    }

    #[cfg(test)]
    pub fn parse_tokens_to_vec(input_source: &'a str) -> Vec<Token<'a>> {
        let tokens = Token::parse_tokens(input_source).inspect(|item| {
            debug_assert!(
                input_source.as_ptr() <= item.1.as_ptr(),
                "Token is not a substring of the input source."
            );
            debug_assert!(
                item.1.as_ptr().wrapping_offset(item.1.len() as isize)
                    <= input_source
                        .as_ptr()
                        .wrapping_offset(input_source.len() as isize),
                "Token is not a substring of the input source",
            );
        });
        tokens.collect()
    }

    fn parse_next(input_source: &'a str) -> Option<(&'a str, Token<'a>)> {
        let input_source = input_source.trim_start();
        if input_source.is_empty() {
            return None;
        }
        match input_source.chars().next() {
            None => return None,
            Some('"') => return Some(Token::parse_next_string(input_source)),
            Some('(') => {
                return Some((
                    &input_source[1..],
                    Token(TokenType::OpenParen, &input_source[0..1]),
                ))
            }
            Some(')') => {
                return Some((
                    &input_source[1..],
                    Token(TokenType::CloseParen, &input_source[0..1]),
                ))
            }
            _ => {}
        }
        let mut end = input_source.len();
        for (idx, ch) in input_source.char_indices() {
            let is_end = match ch {
                '(' | ')' => true,
                _ => ch.is_whitespace(),
            };
            if is_end {
                end = idx;
                break;
            }
        }
        let rest_source = &input_source[end..];
        let token = Token(TokenType::Other, &input_source[0..end]);
        Some((rest_source, token))
    }

    fn parse_next_string(input_source: &'a str) -> (&'a str, Token<'a>) {
        let mut is_escaped = false;
        for (idx, ch) in input_source.char_indices() {
            if idx == 0 {
                debug_assert_eq!(ch, '"');
                continue;
            };
            match ch {
                '\\' => {
                    is_escaped = !is_escaped;
                }
                '"' => {
                    if !is_escaped {
                        let end = idx + 1;
                        let token = Token(TokenType::String, &input_source[0..end]);
                        return (&input_source[end..], token);
                    }
                    is_escaped = false;
                }
                _ => {
                    is_escaped = false;
                }
            };
        }
        (
            &input_source[input_source.len()..input_source.len()],
            Token(TokenType::UnterminatedString, input_source),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_str_produces_empty_stream() {
        let actual = Token::parse_tokens_to_vec("");
        assert_eq!(actual, Vec::new());
    }

    #[test]
    fn whitespace_only_produces_empty_stream() {
        let actual = Token::parse_tokens_to_vec(" \n\t");
        assert_eq!(actual, Vec::new());
    }

    #[test]
    fn whitespace_separated_values_produce_item_for_each() {
        let src = "\t1  two\n3.0\n";
        let actual = Token::parse_tokens_to_vec(src);
        assert_eq!(
            actual,
            vec![
                Token(TokenType::Other, "1"),
                Token(TokenType::Other, "two"),
                Token(TokenType::Other, "3.0"),
            ]
        );
    }

    #[test]
    fn phrase_in_quotes_is_string() {
        let actual = Token::parse_tokens_to_vec("\"hello world!\"not-text");
        assert_eq!(
            actual,
            vec![
                Token(TokenType::String, "\"hello world!\""),
                Token(TokenType::Other, "not-text")
            ]
        );
    }

    #[test]
    fn backslash_quote_in_quote_escapes_quote_as_part_of_string() {
        let actual = Token::parse_tokens_to_vec(r#" \" "\"quotes\""   "#);
        assert_eq!(
            actual,
            vec![
                Token(TokenType::Other, "\\\""),
                Token(TokenType::String, "\"\\\"quotes\\\"\"")
            ]
        );
    }

    #[test]
    fn unclosed_string_is_unterminated_string() {
        let actual = Token::parse_tokens_to_vec("\"I am not closed");
        assert_eq!(
            actual,
            vec![Token(TokenType::UnterminatedString, "\"I am not closed")]
        );
    }

    #[test]
    fn parenthesis_are_parsed_into_own_tokens() {
        let actual = Token::parse_tokens_to_vec("(left right)");
        assert_eq!(
            actual,
            vec![
                Token(TokenType::OpenParen, "("),
                Token(TokenType::Other, "left"),
                Token(TokenType::Other, "right"),
                Token(TokenType::CloseParen, ")")
            ]
        );
    }
}
