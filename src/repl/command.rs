use anyhow::{anyhow, Result};
use std::str::FromStr;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MetaCommand {
    None,
    Ast,
    ByteCode,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Command<'a> {
    pub command: MetaCommand,
    pub expression: &'a str,
}

impl<'a> TryFrom<&'a str> for Command<'a> {
    type Error = anyhow::Error;

    fn try_from(input: &'a str) -> Result<Self> {
        let input = input.trim();
        let mut iter_chars = input.char_indices().peekable();
        if iter_chars.next() != Some((0, ',')) {
            return Ok(Command {
                command: MetaCommand::None,
                expression: input,
            });
        }
        while let Some((_, ch)) = iter_chars.next() {
            if ch.is_whitespace() {
                break;
            }
        }
        let split_idx = iter_chars
            .next()
            .map(|(idx, _)| idx)
            .unwrap_or(input.len() - 1);
        let command = MetaCommand::from_str(&input[0..split_idx])?;
        let expression = &input[split_idx..];
        Ok(Command {
            command,
            expression,
        })
    }
}

impl std::str::FromStr for MetaCommand {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim() {
            "" => Ok(MetaCommand::None),
            ",ast" => Ok(MetaCommand::Ast),
            ",bytecode" => Ok(MetaCommand::ByteCode),
            s => Err(anyhow!("{s:?} is not a valid meta command")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_on_sexp_produces_sexp() {
        assert_eq!(
            Command::try_from("(+ 1 2)").unwrap(),
            Command {
                command: MetaCommand::None,
                expression: "(+ 1 2)",
            }
        );
    }

    #[test]
    fn trailing_whitespace_is_trimmed() {
        assert_eq!(
            Command::try_from(" (+ 1 2) ").unwrap(),
            Command {
                command: MetaCommand::None,
                expression: "(+ 1 2)",
            }
        );
    }

    #[test]
    fn ast_metacommand_is_parsed() {
        assert_eq!(
            Command::try_from(" ,ast (+ 1 2)").unwrap(),
            Command {
                command: MetaCommand::Ast,
                expression: "(+ 1 2)",
            }
        );
    }

    #[test]
    fn bytecode_metacommand_is_parsed() {
        assert_eq!(
            Command::try_from(",bytecode (+ 1 2)").unwrap(),
            Command {
                command: MetaCommand::ByteCode,
                expression: "(+ 1 2)",
            }
        );
    }

    #[test]
    fn bad_metacommand_returns_error() {
        assert!(Command::try_from(", (+ 1 2)").is_err(),);
        assert!(Command::try_from(",bad (+ 1 2)").is_err(),);
        assert!(Command::try_from(",+ (+ 1 2)").is_err(),);
        assert!(Command::try_from(",_ (+ 1 2)").is_err(),);
    }
}
