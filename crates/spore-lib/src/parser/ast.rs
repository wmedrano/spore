use super::token::{Token, TokenType};

/// The abstract syntax tree.
#[derive(Clone, Debug, PartialEq)]
pub enum Ast {
    /// Signifies this is a root node.
    Leaf(Token<AstLeaf>),
    /// Signifies that this is a tree with the given children.
    Tree(AstTree),
}

#[derive(Clone, Debug, PartialEq)]
pub enum AstLeaf {
    If,
    Lambda,
    Define,
    Identifier(String),
    Symbol(String),
    String(String),
    Float(f64),
    Int(isize),
    Bool(bool),
    Comment(String),
    CommentDatum,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AstTree(pub Vec<Ast>);

impl From<Vec<Ast>> for AstTree {
    fn from(v: Vec<Ast>) -> Self {
        AstTree(v)
    }
}

impl AstTree {
    /// Iterate over the AST children. This includes comment nodes, unlike `iter`.
    pub fn iter_with_comments(&self) -> impl Iterator<Item = &'_ Ast> {
        self.0.iter()
    }

    /// Iterate over the AST children. This skips any comments.
    pub fn iter(&self) -> impl Iterator<Item = &'_ Ast> {
        let mut skip_next = false;
        self.0.iter().filter(move |ast| {
            if skip_next {
                skip_next = false;
                return false;
            }
            match ast {
                Ast::Leaf(Token {
                    item: AstLeaf::Comment(_),
                    ..
                }) => false,
                Ast::Leaf(Token {
                    item: AstLeaf::CommentDatum,
                    ..
                }) => {
                    skip_next = true;
                    false
                }
                _ => true,
            }
        })
    }
}

impl Default for Ast {
    fn default() -> Ast {
        Ast::Leaf(Token {
            item: AstLeaf::Comment("".to_string()),
            range: 0..0,
        })
    }
}

impl Ast {
    /// Convert a string into an AST.
    pub fn from_sexp_str(s: &str) -> Result<Vec<Ast>, ParseAstError> {
        let tokens = crate::parser::lexer::tokenize(s);
        Ast::from_tokens(tokens)
    }

    /// Convert an iterator of tokens into an AST.
    pub fn from_tokens(
        tokens: impl Iterator<Item = Token<TokenType>>,
    ) -> Result<Vec<Ast>, ParseAstError> {
        let mut tokens = tokens;
        Ast::from_tokens_impl(&mut tokens, None, 0)
    }

    /// Returns the identifier if the AST node is a leaf node with an identifier, otherwise returns
    /// `None`.
    pub fn as_identifier(&self) -> Option<&str> {
        match self {
            Ast::Leaf(Token {
                item: AstLeaf::Identifier(ident),
                ..
            }) => Some(ident),
            _ => None,
        }
    }

    /// Returns a list of identifiers if the AST node is a tree containing only identifiers,
    /// otherwise `None`.
    pub fn as_identifier_list(&self) -> Option<Vec<&str>> {
        match self {
            Ast::Tree(tree) => {
                let mut identifiers = Vec::new();
                for child in tree.iter() {
                    if let Some(ident) = child.as_identifier() {
                        identifiers.push(ident);
                    } else {
                        return None;
                    }
                }
                Some(identifiers)
            }
            _ => None,
        }
    }

    /// Convert an iterator over `Token`s into an `Ast`.
    ///
    ///   - tokens - The iterator of tokens to take as input.
    ///   - opening_paren - The index of the current opening paren. Used to determine if parsing
    ///                     should end at the closing paren. If an open paren is provided but no
    ///                     closing paren is encountered, then an error is returned.
    fn from_tokens_impl(
        tokens: &mut impl Iterator<Item = Token<TokenType>>,
        opening_paren: Option<usize>,
        last_idx: usize,
    ) -> Result<Vec<Ast>, ParseAstError> {
        let mut exps = Vec::new();
        let mut end_idx = last_idx;
        while let Some(token) = tokens.next() {
            end_idx = token.range.end;
            // LeftParen  - Start parsing a sub expression and add it as a subtree.
            // RightParen - End parsing of a sub expression and returns the results.
            // Literals   - Add the atom as a leaf node.
            match &token.item {
                TokenType::LeftParen => {
                    let sub_ast =
                        Self::from_tokens_impl(tokens, Some(token.range.start), token.range.start)?;
                    exps.push(Ast::Tree(AstTree(sub_ast)));
                }
                TokenType::RightParen => match opening_paren {
                    Some(_) => return Ok(exps),
                    None => {
                        return Err(ParseAstError::UnexpectedClosingParen {
                            idx: token.range.start,
                        })
                    }
                },
                TokenType::Identifier(s) => {
                    // Starting with a double quote indicates that the lexer was unable to parse
                    // this as a valid `TokenType::String`. We report the error here as the lexer
                    // does not have error reporting capabilities.
                    if s.starts_with('"') {
                        return Err(ParseAstError::InvalidIdentifier(Token {
                            item: s.clone(),
                            range: token.range,
                        }));
                    }
                    if let Some(sym) = s.strip_prefix('\'') {
                        exps.push(Ast::Leaf(
                            token.with_item(AstLeaf::Symbol(sym.trim().to_string())),
                        ));
                    } else {
                        match s.as_str() {
                            "if" => exps.push(Ast::Leaf(token.with_item(AstLeaf::If))),
                            "lambda" => exps.push(Ast::Leaf(token.with_item(AstLeaf::Lambda))),
                            "define" => exps.push(Ast::Leaf(token.with_item(AstLeaf::Define))),
                            _ => exps
                                .push(Ast::Leaf(token.with_item(AstLeaf::Identifier(s.clone())))),
                        }
                    }
                }
                TokenType::String(s) => {
                    exps.push(Ast::Leaf(token.with_item(AstLeaf::String(s.clone()))));
                }
                TokenType::Int(v) => exps.push(Ast::Leaf(token.with_item(AstLeaf::Int(*v)))),
                TokenType::Float(v) => exps.push(Ast::Leaf(token.with_item(AstLeaf::Float(*v)))),
                TokenType::Bool(v) => exps.push(Ast::Leaf(token.with_item(AstLeaf::Bool(*v)))),
                TokenType::Comment(c) => {
                    exps.push(Ast::Leaf(token.with_item(AstLeaf::Comment(c.clone()))))
                }
                TokenType::CommentDatum => {
                    exps.push(Ast::Leaf(token.with_item(AstLeaf::CommentDatum)));
                }
            };
        }
        match opening_paren {
            Some(open_idx) => Err(ParseAstError::MissingClosingParen { open_idx, end_idx }),
            None => Ok(exps),
        }
    }

    fn display_with_depth(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        depth: usize,
    ) -> std::fmt::Result {
        match self {
            Ast::Leaf(l) => {
                for _ in 0..depth {
                    write!(f, "  ")?;
                }
                writeln!(f, "{}", l.item)
            }
            Ast::Tree(children) => {
                let depths = std::iter::once(depth).chain(std::iter::repeat(depth + 1));
                for (child, depth) in children.iter_with_comments().zip(depths) {
                    child.display_with_depth(f, depth)?;
                }
                Ok(())
            }
        }
    }
}

impl std::fmt::Display for Ast {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.display_with_depth(f, 0)
    }
}

impl std::fmt::Display for AstLeaf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AstLeaf::If => write!(f, "<if>"),
            AstLeaf::Lambda => write!(f, "<lambda>"),
            AstLeaf::Define => write!(f, "<define>"),
            AstLeaf::Identifier(ident) => write!(f, "<identifier {ident}>"),
            AstLeaf::Symbol(sym) => write!(f, "<symbol {sym}"),
            AstLeaf::String(s) => write!(f, "<string {s}>"),
            AstLeaf::Float(x) => write!(f, "<float {x}>"),
            AstLeaf::Int(x) => write!(f, "<int {x}>"),
            AstLeaf::Bool(x) => write!(f, "<bool {x}>"),
            AstLeaf::Comment(_) => write!(f, "<comment ...>"),
            AstLeaf::CommentDatum => write!(f, "<comment-datum>"),
        }
    }
}

/// Holds a parse error.
#[derive(Debug, PartialEq)]
pub enum ParseAstError {
    /// There was a missing closing paren.
    MissingClosingParen { open_idx: usize, end_idx: usize },
    /// No closing paren was expected.
    UnexpectedClosingParen { idx: usize },
    /// The identifier is not valid.
    InvalidIdentifier(Token<String>),
}

impl std::fmt::Display for ParseAstError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ParseAstError {}

impl ParseAstError {
    pub fn display_with_context(&self, src: &str) -> String {
        let make_spacing = |n| String::from_iter(std::iter::repeat(' ').take(n));
        match self {
            ParseAstError::MissingClosingParen { open_idx, end_idx } => {
                let context = &src[*open_idx..*end_idx];
                format!("{context}\n^\nMissing closing paren.")
            }
            ParseAstError::UnexpectedClosingParen { idx } => {
                let start = (*idx).saturating_sub(5);
                let context = &src[start..(*idx + 5).clamp(0, src.len())];
                let space = make_spacing(src[start..*idx].chars().count());
                format!("{context}\n{space}^\nUnexpected closing paren.")
            }
            ParseAstError::InvalidIdentifier(ident) => {
                let context = ident.item.as_str();
                format!("{context}\n^ Invalid identifier.")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn parse_multiple() {
        use Ast::*;
        use AstLeaf::*;
        assert_eq!(
            Ast::from_sexp_str("1 1.0 \"hello\"").unwrap(),
            vec![
                Leaf(Token {
                    item: Int(1,),
                    range: 0..1,
                }),
                Leaf(Token {
                    item: Float(1.0),
                    range: 2..5,
                }),
                Leaf(Token {
                    item: String("hello".to_string()),
                    range: 6..13,
                }),
            ]
        )
    }

    #[test]
    fn parse_list() {
        use Ast::*;
        use AstLeaf::*;
        assert_eq!(
            Ast::from_sexp_str("(1 2 3)").unwrap(),
            vec![Tree(
                vec![
                    Leaf(Token {
                        item: Int(1,),
                        range: 1..2,
                    }),
                    Leaf(Token {
                        item: Int(2,),
                        range: 3..4,
                    }),
                    Leaf(Token {
                        item: Int(3,),
                        range: 5..6,
                    }),
                ]
                .into()
            )]
        )
    }

    #[test]
    fn empty_list() {
        assert_eq!(
            Ast::from_sexp_str("()").unwrap(),
            vec![Ast::Tree(vec![].into())]
        )
    }

    #[test]
    fn nested_lists() {
        use Ast::*;
        use AstLeaf::*;
        assert_eq!(
            Ast::from_sexp_str("(1 2 (3 4) (5))").unwrap(),
            vec![Tree(
                vec![
                    Leaf(Token {
                        item: Int(1),
                        range: 1..2
                    }),
                    Leaf(Token {
                        item: Int(2),
                        range: 3..4,
                    }),
                    Tree(
                        vec![
                            Leaf(Token {
                                item: Int(3),
                                range: 6..7,
                            }),
                            Leaf(Token {
                                item: Int(4),
                                range: 8..9,
                            }),
                        ]
                        .into()
                    ),
                    Tree(
                        vec![Leaf(Token {
                            item: Int(5),
                            range: 12..13,
                        })]
                        .into()
                    ),
                ]
                .into()
            )]
        )
    }

    #[test]
    fn no_matching_paren_is_err() {
        assert_eq!(
            Ast::from_sexp_str(" (1 2").unwrap_err(),
            ParseAstError::MissingClosingParen {
                open_idx: 1,
                end_idx: 5
            },
        )
    }

    #[test]
    fn unexpected_closing_paren_is_err() {
        assert_eq!(
            Ast::from_sexp_str("(1 2)) (2").unwrap_err(),
            ParseAstError::UnexpectedClosingParen { idx: 5 },
        );
    }

    #[test]
    fn no_closing_quote_is_err() {
        assert_eq!(
            Ast::from_sexp_str("\"hello").unwrap_err(),
            ParseAstError::InvalidIdentifier(Token {
                item: "\"hello".to_string(),
                range: 0..6,
            }),
        );
    }

    #[test]
    fn can_parse_string() {
        use Ast::*;
        use AstLeaf::*;
        assert_eq!(
            Ast::from_sexp_str("'hello \"to\" 'the 'world").unwrap(),
            vec![
                Leaf(Token {
                    item: Symbol("hello".to_string()),
                    range: 0..6,
                },),
                Leaf(Token {
                    item: String("to".to_string()),
                    range: 7..11,
                },),
                Leaf(Token {
                    item: Symbol("the".to_string()),
                    range: 12..16,
                },),
                Leaf(Token {
                    item: Symbol("world".to_string()),
                    range: 17..23,
                },),
            ],
        )
    }

    #[test]
    fn can_parse_symbol() {
        use Ast::*;
        use AstLeaf::*;
        assert_eq!(
            Ast::from_sexp_str("'hello 'world").unwrap(),
            vec![
                Leaf(Token {
                    item: Symbol("hello".to_string()),
                    range: 0..6,
                }),
                Leaf(Token {
                    item: Symbol("world".to_string()),
                    range: 7..13,
                })
            ]
        );
    }

    #[test]
    fn comments_are_left_in_ast() {
        use Ast::*;
        use AstLeaf::*;
        assert_eq!(
            Ast::from_sexp_str("symbol 1 2 3 ; comment").unwrap(),
            vec![
                Leaf(Token {
                    item: Identifier("symbol".to_string()),
                    range: 0..6,
                }),
                Leaf(Token {
                    item: Int(1),
                    range: 7..8,
                }),
                Leaf(Token {
                    item: Int(2),
                    range: 9..10,
                }),
                Leaf(Token {
                    item: Int(3),
                    range: 11..12,
                }),
                Leaf(Token {
                    item: Comment("; comment".to_string()),
                    range: 13..22,
                }),
            ]
        );
    }

    #[test]
    fn ast_display() {
        let ast = Ast::from_sexp_str("(* (+ 1 2) (- 3 4))").unwrap();
        assert_eq!(
            ast.first().unwrap().to_string(),
            r#"<identifier *>
  <identifier +>
    <int 1>
    <int 2>
  <identifier ->
    <int 3>
    <int 4>
"#
        );
    }
}
