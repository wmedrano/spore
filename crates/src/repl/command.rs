/// Parse a command from input. This returns a tuple of `(command, expression)`.
///
/// Examples:
///   `parse_command("(+ 4 5)") => ("", "(+ 4 5)")`
///   `parse_command(",bytecode (+ 4 5)") => (",bytecode", "(+ 4 5)")`
pub fn parse_command(input: &str) -> (&str, &str) {
    let input = input.trim();
    let mut iter_chars = input.char_indices().peekable();
    if iter_chars.next() != Some((0, ',')) {
        return ("", input);
    }
    while iter_chars.next_if(|(_, ch)| !ch.is_whitespace()).is_some() {}
    while iter_chars.next_if(|(_, ch)| ch.is_whitespace()).is_some() {}
    let split_idx = iter_chars
        .next()
        .map(|(idx, _)| idx)
        .unwrap_or(input.len() - 1);
    let command = &input[..split_idx].trim();
    let expression = &input[split_idx..];
    (command, expression)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_on_sexp_produces_sexp() {
        assert_eq!(parse_command("(+ 1 2)"), ("", "(+ 1 2)",));
    }

    #[test]
    fn trailing_whitespace_is_trimmed() {
        assert_eq!(parse_command(" (+ 1 2) "), ("", "(+ 1 2)",));
        assert_eq!(parse_command("  ,ast  (+ 1 2)  "), (",ast", "(+ 1 2)",));
    }

    #[test]
    fn metacommands_are_parsed() {
        assert_eq!(parse_command(",ast (+ 1 2)"), (",ast", "(+ 1 2)",));
        assert_eq!(
            parse_command(",bytecode (+ 1 2)"),
            (",bytecode", "(+ 1 2)",)
        );
        assert_eq!(parse_command(",custom (+ 1 2)"), (",custom", "(+ 1 2)",));
        assert_eq!(parse_command(", (+ 1 2)"), (",", "(+ 1 2)",));
    }
}
