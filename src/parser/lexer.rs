use super::{
    scanner::scan,
    token::{Token, TokenType},
};

/// Converts a string into an iterator of tokens.
pub fn tokenize(s: &str) -> impl '_ + Iterator<Item = Token<TokenType>> {
    classify_tokens(scan(s))
}

fn classify_tokens<'a, T: 'a + AsRef<str>>(
    tokens: impl Iterator<Item = Token<T>>,
) -> impl Iterator<Item = Token<TokenType>> {
    tokens.map(|t| {
        t.map(|raw_t| match raw_t.as_ref() {
            "(" => TokenType::LeftParen,
            ")" => TokenType::RightParen,
            s if is_string_literal(s) => TokenType::String(s[1..s.len() - 1].to_string()),
            s => {
                if let Ok(i) = s.parse() {
                    return TokenType::Int(i);
                }
                if let Ok(f) = s.parse() {
                    return TokenType::Float(f);
                }
                if let Ok(b) = s.parse() {
                    return TokenType::Bool(b);
                }
                TokenType::Identifier(s.to_string())
            }
        })
    })
}

fn is_string_literal(s: &str) -> bool {
    s.starts_with('"') && s.ends_with('"') && !s.ends_with("\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn tokenize(s: &str) -> Vec<Token<TokenType>> {
        super::tokenize(s).collect()
    }

    #[test]
    fn tokenize_with_strings() {
        use TokenType::*;
        assert_eq!(
            tokenize("(concatenate \"hello\" \"world\")"),
            vec![
                Token {
                    item: LeftParen,
                    range: 0..1,
                },
                Token {
                    item: Identifier("concatenate".to_string()),
                    range: 1..12,
                },
                Token {
                    item: String("hello".to_string()),
                    range: 13..20,
                },
                Token {
                    item: String("world".to_string()),
                    range: 21..28,
                },
                Token {
                    item: RightParen,
                    range: 28..29,
                },
            ]
        );
    }
}
