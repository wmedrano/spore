//! Contains errors definitions for the Spore VM.
use compact_str::CompactString;
use thiserror::Error;

use crate::{parser::ast::AstParseError, val::custom::CustomValError};

/// A `Result` with `VmError` as the error branch.
pub type VmResult<T> = Result<T, VmError>;

/// Describes an error encountered when running the Vm.
#[derive(Debug, Error, PartialEq)]
pub enum VmError {
    #[error("{context} expected type {expected} but found type {actual}: {value}")]
    TypeError {
        context: &'static str,
        expected: &'static str,
        actual: &'static str,
        value: String,
    },
    #[error("wrong arity, function {function} expected {expected} args but found {actual} args")]
    ArityError {
        function: CompactString,
        expected: usize,
        actual: usize,
    },
    #[error("compile error ocurred: {0}")]
    CompileError(#[from] CompileError),
    #[error("invalid vm state, this is likely a bug: {0}")]
    InvalidVmState(#[from] BacktraceError),
    #[error("{0} is not defined")]
    SymbolNotDefined(String),
    #[error(
        "maximum function recursion depth of {max_depth} reached, call stack is {call_stack:?}"
    )]
    MaximumRecursionDepth {
        max_depth: usize,
        call_stack: Vec<CompactString>,
    },
    #[error("{0}")]
    CustomValError(#[from] CustomValError),
    #[error("{0}")]
    CustomError(String),
}

/// Describes a generic error along with its stacktrace.
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
        false
    }
}

/// Describes an error during compilation to Spore bytecode.
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
    #[error("{context} expected expression but sub-expression did not return a value")]
    ExpectedExpression { context: &'static str },
    #[error("{context} expected identifier list")]
    ExpectedIdentifierList { context: &'static str },
    #[error("define is only allowed at the toplevel and not as a subexpression")]
    DefineNotAllowedInSubexpression,
    #[error("argument {0} was defined multiple times")]
    ArgumentDefinedMultipleTimes(CompactString),
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;

    #[test]
    fn backtraces_are_all_eq() {
        // Backtraces are abstract so we assume (mostly for tests sake) that they are not
        // equivalent.
        assert_ne!(BacktraceError::capture(), BacktraceError::capture());
    }

    #[test]
    fn hacks_for_code_coverage() {
        // A collection of functions that are not worth testing.
        VmError::CustomError("".to_string()).source();
        assert_ne!(CompileError::EmptyExpression.to_string(), "");
        assert_ne!(format!("{:?}", CompileError::EmptyExpression), "");
        assert_ne!(BacktraceError::capture().to_string(), "");
        assert_ne!(format!("{:?}", BacktraceError::capture()), "");
    }
}
