use anyhow::Result;
use spore_vm::{
    error::{AstParseError, CompileError, VmError},
    Vm,
};

pub struct Repl {
    input: String,
    vm: Vm,
}

impl Default for Repl {
    fn default() -> Repl {
        Repl::new(Vm::default())
    }
}

impl Repl {
    /// Create a new Repl.
    pub fn new(vm: Vm) -> Repl {
        Repl {
            input: String::new(),
            vm,
        }
    }

    /// Returns true if the Repl has partial input waiting to be completed.
    pub fn has_partial_input(&self) -> bool {
        !self.input.is_empty()
    }

    /// Attempts to evaluate `expr`.
    pub fn eval_expr(&mut self, expr: &str, out: &mut impl std::io::Write) -> Result<()> {
        self.input.push_str(expr);
        match self.vm.eval_str(&self.input) {
            Ok(v) if v.is_void() => {}
            Ok(v) => writeln!(out, "{}", v)?,
            Err(VmError::CompileError(CompileError::AstError(AstParseError::UnclosedParen))) => {
                self.input.push('\n');
                return Ok(());
            }
            Err(err) => Err(err)?,
        }
        self.input.clear();
        Ok(())
    }

    #[cfg(test)]
    fn eval_expr_to_string(&mut self, expr: &str) -> Result<String> {
        let mut out = Vec::new();
        self.eval_expr(expr, &mut out)?;
        Ok(String::from_utf8(out)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_expression_produces_nothing() {
        let mut repl = Repl::default();
        assert_eq!(repl.eval_expr_to_string("").unwrap(), "");
    }

    #[test]
    fn simple_expression_prints_result() {
        let mut repl = Repl::default();
        assert_eq!(repl.eval_expr_to_string("(+ 2 2)").unwrap(), "4\n");
    }

    #[test]
    fn unclosed_expression_can_be_completed_with_multiple_eval_calls() {
        let mut repl = Repl::default();
        assert_eq!(repl.eval_expr_to_string("(+ 2 2").unwrap(), "");
        assert_eq!(repl.eval_expr_to_string("2").unwrap(), "");
        assert_eq!(repl.eval_expr_to_string(")").unwrap(), "6\n");
    }

    #[test]
    fn compile_error_returns_error() {
        let mut repl = Repl::default();
        assert!(repl.eval_expr_to_string("(+ 2 2))").is_err());
    }

    #[test]
    fn runtime_error_returns_error() {
        let mut repl = Repl::default();
        assert!(repl.eval_expr_to_string("(+ \"string\")").is_err());
    }
}
