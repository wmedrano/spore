/// Describes the type of token.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TokenType {
    /// An open parenthesis.
    OpenParen,
    /// A close parenthesis.
    CloseParen,
    /// A string literal.
    String,
    /// A string literal, but missing the closing quote.
    UnterminatedString,
    /// Something else. Usually an atom(int, float literal) or an identifier.
    Other,
}

/// Contains a token type and the portion of the text defining the token.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Token {
    pub token_type: TokenType,
    pub start: usize,
    pub end: usize,
}

impl Token {
    pub fn as_str<'a>(&self, src: &'a str) -> &'a str {
        &src[self.start..self.end]
    }

    /// Parse an input source into a stream of tokens.
    pub fn parse_tokens(input_source: &str) -> impl '_ + Iterator<Item = Token> {
        let mut start = 0;
        std::iter::from_fn(move || {
            Token::parse_next(input_source, start).inspect(|t| start = t.end)
        })
    }

    /// Parse an input source into a vector of tokens. Used for convenience in unit tests, prefer
    /// using `Self::parse_tokens`.
    #[cfg(test)]
    pub fn parse_tokens_to_vec<'a>(input_src: &'a str) -> Vec<(TokenType, &'a str)> {
        let tokens = Token::parse_tokens(input_src)
            .map(|token| (token.token_type, &input_src[token.start..token.end]));
        tokens.collect()
    }

    /// Parse the next token within an input source string or return `None`. Returns a tuple
    /// containing the parsed token and the rest of the string.
    fn parse_next(src: &str, start: usize) -> Option<Token> {
        let input_src = &src[start..].trim_start();
        let start = src.len() - input_src.len();
        match input_src.chars().next() {
            None => return None,
            Some('"') => return Some(Token::parse_next_string(src, start)),
            Some('(') => {
                return Some(Token {
                    token_type: TokenType::OpenParen,
                    start,
                    end: start + 1,
                })
            }
            Some(')') => {
                return Some(Token {
                    token_type: TokenType::CloseParen,
                    start,
                    end: start + 1,
                })
            }
            _ => {}
        }
        for (idx, ch) in input_src.char_indices() {
            let is_end = match ch {
                '(' | ')' => true,
                _ => ch.is_whitespace(),
            };
            if is_end {
                return Some(Token {
                    token_type: TokenType::Other,
                    start,
                    end: start + idx,
                });
            }
        }
        Some(Token {
            token_type: TokenType::Other,
            start,
            end: src.len(),
        })
    }

    /// Parse the next string in input source. `input_source` must start with a '"'
    /// character. Returns a tuple of the parsed token and the rest of the string.
    fn parse_next_string(src: &str, start: usize) -> Token {
        let input_src = &src[start..];
        let mut is_escaped = false;
        for (idx, ch) in input_src.char_indices() {
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
                        return Token {
                            token_type: TokenType::String,
                            start,
                            end: start + idx + 1,
                        };
                    }
                    is_escaped = false;
                }
                _ => {
                    is_escaped = false;
                }
            };
        }
        Token {
            token_type: TokenType::UnterminatedString,
            start,
            end: src.len(),
        }
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
                (TokenType::Other, "1"),
                (TokenType::Other, "two"),
                (TokenType::Other, "3.0"),
            ]
        );
    }

    #[test]
    fn phrase_in_quotes_is_string() {
        let actual = Token::parse_tokens_to_vec("\"hello world!\"not-text");
        assert_eq!(
            actual,
            vec![
                (TokenType::String, "\"hello world!\""),
                (TokenType::Other, "not-text")
            ]
        );
    }

    #[test]
    fn backslash_quote_in_quote_escapes_quote_as_part_of_string() {
        let actual = Token::parse_tokens_to_vec(r#" \" "\"quotes\""   "#);
        assert_eq!(
            actual,
            vec![
                (TokenType::Other, "\\\""),
                (TokenType::String, "\"\\\"quotes\\\"\"")
            ]
        );
    }

    #[test]
    fn unclosed_string_is_unterminated_string() {
        let actual = Token::parse_tokens_to_vec("\"I am not closed");
        assert_eq!(
            actual,
            vec![(TokenType::UnterminatedString, "\"I am not closed")]
        );
    }

    #[test]
    fn parenthesis_are_parsed_into_own_tokens() {
        let actual = Token::parse_tokens_to_vec("(left right)");
        assert_eq!(
            actual,
            vec![
                (TokenType::OpenParen, "("),
                (TokenType::Other, "left"),
                (TokenType::Other, "right"),
                (TokenType::CloseParen, ")")
            ]
        );
    }
}
