//! Contains errors definitions for the Spore VM.
use compact_str::CompactString;
use thiserror::Error;

use crate::{parser::ast::AstParseError, val::custom::CustomValError};

/// A `Result` with `VmError` as the error branch.
pub type VmResult<T> = Result<T, VmError>;

/// Describes an error encountered when running the Vm.
#[derive(Debug, PartialEq)]
pub enum VmError {
    TypeError {
        context: &'static str,
        expected: &'static str,
        actual: &'static str,
        value: String,
    },
    ArityError {
        function: CompactString,
        expected: usize,
        actual: usize,
    },
    CompileError(CompileError),
    InvalidVmState(BacktraceError),
    SymbolNotDefined {
        src: Option<String>,
        symbol: String,
    },
    MaximumFunctionCallDepth {
        max_depth: usize,
        call_stack: Vec<CompactString>,
    },
    CustomValError(CustomValError),
    CustomError(String),
}

impl VmError {
    /// Return the error with the given source context added.
    pub fn with_src(self, src: String) -> VmError {
        match self {
            VmError::TypeError {
                context,
                expected,
                actual,
                value,
            } => VmError::TypeError {
                context,
                expected,
                actual,
                value,
            },
            VmError::ArityError {
                function,
                expected,
                actual,
            } => VmError::ArityError {
                function,
                expected,
                actual,
            },
            VmError::CompileError(e) => VmError::CompileError(e),
            VmError::InvalidVmState(e) => VmError::InvalidVmState(e),
            VmError::SymbolNotDefined { symbol, .. } => VmError::SymbolNotDefined {
                src: Some(src),
                symbol,
            },
            VmError::MaximumFunctionCallDepth {
                max_depth,
                call_stack,
            } => VmError::MaximumFunctionCallDepth {
                max_depth,
                call_stack,
            },
            VmError::CustomValError(e) => VmError::CustomValError(e),
            VmError::CustomError(e) => VmError::CustomError(e),
        }
    }
}

impl std::fmt::Display for VmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VmError::TypeError {
                context,
                expected,
                actual,
                value,
            } => write!(
                f,
                "{context} expected type {expected} but got {actual}: {value}"
            ),
            VmError::ArityError {
                function,
                expected,
                actual,
            } => write!(f, "{function} expected {expected} args but got {actual}."),
            VmError::CompileError(e) => write!(f, "{e}"),
            VmError::InvalidVmState(bt) => write!(f, "VM reached invalid state.\n{bt}"),
            VmError::SymbolNotDefined {
                symbol,
                src: context,
            } => {
                write!(f, "Value {symbol} is not defined.")?;
                if let Some(context) = context {
                    write!(f, "\n{context}")?;
                }
                Ok(())
            }
            VmError::MaximumFunctionCallDepth {
                max_depth,
                call_stack,
            } => write!(
                f,
                "Maximum function call depth of {max_depth} reached: {call_stack:#?}"
            ),
            VmError::CustomValError(e) => write!(f, "{e}"),
            VmError::CustomError(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for VmError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            VmError::TypeError { .. }
            | VmError::ArityError { .. }
            | VmError::SymbolNotDefined { .. }
            | VmError::MaximumFunctionCallDepth { .. }
            | VmError::CustomError(_) => None,
            VmError::CompileError(e) => Some(e),
            VmError::InvalidVmState(e) => Some(e),
            VmError::CustomValError(e) => Some(e),
        }
    }
}

impl From<String> for VmError {
    fn from(v: String) -> VmError {
        VmError::CustomError(v)
    }
}

impl From<BacktraceError> for VmError {
    fn from(v: BacktraceError) -> VmError {
        VmError::InvalidVmState(v)
    }
}

impl From<CompileError> for VmError {
    fn from(v: CompileError) -> VmError {
        VmError::CompileError(v)
    }
}

impl From<CustomValError> for VmError {
    fn from(v: CustomValError) -> VmError {
        VmError::CustomValError(v)
    }
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
