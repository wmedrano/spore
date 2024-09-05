use thiserror::Error;

pub type VmResult<T> = Result<T, VmError>;

#[derive(Debug, Error, PartialEq)]
pub enum VmError {
    #[error("type error")]
    TypeError,
    #[error("compile error ocurred: {0}")]
    CompileError(#[from] CompileError),
    #[error("invalid vm state, this is likely a bug: {0}")]
    InvalidVmState(#[from] BacktraceError),
    #[error("{0} is not defined")]
    SymbolNotDefined(String),
}

pub struct BacktraceError(std::backtrace::Backtrace);

impl BacktraceError {
    #[inline(always)]
    pub fn capture() -> BacktraceError {
        BacktraceError(std::backtrace::Backtrace::capture())
    }
}

impl std::error::Error for BacktraceError {}

impl std::fmt::Display for BacktraceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::fmt::Debug for BacktraceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl PartialEq for BacktraceError {
    // We say that all backtrace errors are equivalent to make unit tests accept any backtrace. This
    // is simpler as it is not possible to compare backtraces.
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

/// Describes an AST parsing error.
#[derive(Debug, Error, PartialEq)]
pub enum AstParseError {
    #[error("opening parenthesis was unclosed")]
    UnclosedParen,
    #[error("found unexpected closing parenthesis")]
    UnexpectedCloseParen,
    #[error("string was not properly closed, did you forget \"?")]
    UnclosedString,
}

#[derive(Debug, Error, PartialEq)]
pub enum CompileError {
    #[error("syntax error occurred: {0}")]
    AstError(#[from] AstParseError),
    #[error("found unexpected empty expression")]
    EmptyExpression,
    #[error("expression {expression} expected {expected} arguments but found {actual}")]
    ExpressionHasWrongArgs {
        expression: &'static str,
        expected: usize,
        actual: usize,
    },
    #[error("expected an identifier")]
    ExpectedIdentifier,
    #[error("expected expression but sub-expression did not return a value")]
    ExpectedExpression,
    #[error("define is only allowed at the toplevel and not as a subexpression")]
    DefineNotAllowedInSubexpression,
}
