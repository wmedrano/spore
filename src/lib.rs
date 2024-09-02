#[derive(Debug, PartialEq)]
pub struct Token<'a>(&'a str);

#[derive(Debug)]
pub enum TokenParseError {
    UnclosedStringLiteral,
}

impl<'a> Token<'a> {
    pub fn parse_tokens(text: &'a str) -> impl Iterator<Item = Token<'a>> {
        let mut text = text;
        std::iter::from_fn(move || match Token::parse_next(text) {
            None => None,
            Some((next_text, token)) => {
                text = next_text;
                Some(token)
            }
        })
    }

    pub fn parse_tokens_to_vec(text: &'a str) -> Vec<Token<'a>> {
        let tokens = Token::parse_tokens(text);
        tokens.collect()
    }

    fn parse_next(text: &'a str) -> Option<(&'a str, Token<'a>)> {
        let text = text.trim_start();
        if text.is_empty() {
            return None;
        }
        match text.chars().next() {
            None => return None,
            Some('"') => return Some(Token::parse_next_string(text)),
            Some('(') | Some(')') => return Some((&text[1..], Token(&text[0..1]))),
            _ => {}
        }
        let mut end = text.len();
        for (idx, ch) in text.char_indices() {
            let is_end = match ch {
                '(' | ')' => true,
                _ => ch.is_whitespace(),
            };
            if is_end {
                end = idx;
                break;
            }
        }
        let rest_text = &text[end..];
        let token = Token(&text[0..end]);
        Some((rest_text, token))
    }

    fn parse_next_string(text: &'a str) -> (&'a str, Token<'a>) {
        let mut is_escaped = false;
        for (idx, ch) in text.char_indices() {
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
                        let token = Token(&text[0..end]);
                        return (&text[end..], token);
                    }
                    is_escaped = false;
                }
                _ => {
                    is_escaped = false;
                }
            };
        }
        (&text[text.len()..text.len()], Token(text))
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
        assert_eq!(actual, vec![Token("1"), Token("two"), Token("3.0"),]);
    }

    #[test]
    fn phrase_in_quotes_is_string() {
        let actual = Token::parse_tokens_to_vec("\"hello world!\"not-text");
        assert_eq!(actual, vec![Token("\"hello world!\""), Token("not-text")]);
    }

    #[test]
    fn backslash_quote_in_quote_escapes_quote_as_part_of_string() {
        let actual = Token::parse_tokens_to_vec(r#" \" "\"quotes\""   "#);
        assert_eq!(actual, vec![Token("\\\""), Token("\"\\\"quotes\\\"\"")]);
    }

    #[test]
    fn unclosed_string_produces_what_is_has_been_built_up_to() {
        let actual = Token::parse_tokens_to_vec("\"I am not closed");
        assert_eq!(actual, vec![Token("\"I am not closed")]);
    }

    #[test]
    fn parenthesis_are_parsed_into_own_tokens() {
        let actual = Token::parse_tokens_to_vec("(left right)");
        assert_eq!(
            actual,
            vec![Token("("), Token("left"), Token("right"), Token(")")]
        );
    }
}
