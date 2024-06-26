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
            "#;" => TokenType::CommentDatum,
            s if is_string_literal(s) => {
                let s = s[1..s.len() - 1].replace("\\\"", "\"");
                TokenType::String(s)
            }
            s if s.starts_with(';') => TokenType::Comment(s.to_string()),
            s if s.starts_with("#|") && s.ends_with("|#") => TokenType::Comment(s.to_string()),
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
            tokenize(r#"(concatenate "hello" "world" "quotes \"are\" ok")"#),
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
                    item: String("quotes \"are\" ok".to_string()),
                    range: 29..48,
                },
                Token {
                    item: RightParen,
                    range: 48..49,
                },
            ]
        );
    }

    #[test]
    fn tokenize_block_comment() {
        use TokenType::*;
        assert_eq!(
            tokenize("(def var #|comment here between #| and .. | and #|# 42"),
            vec![
                Token {
                    item: LeftParen,
                    range: 0..1,
                },
                Token {
                    item: Identifier("def".to_string()),
                    range: 1..4,
                },
                Token {
                    item: Identifier("var".to_string()),
                    range: 5..8,
                },
                Token {
                    item: Comment("#|comment here between #| and .. | and #|#".to_string()),
                    range: 9..51,
                },
                Token {
                    item: Int(42),
                    range: 52..54,
                },
            ]
        );
    }
}
