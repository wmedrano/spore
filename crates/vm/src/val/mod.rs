//! Contains representation of values within the Spore machine. Each type has differing lifetime and
//! safety guarantees.
mod bytecode;
pub(crate) mod custom;
mod formatter;
mod id;
mod native_function;
mod protected_val;
mod unsafe_val;

pub use bytecode::{ByteCode, Instruction};
pub use custom::CustomType;
pub use formatter::ValFormatter;
pub use id::ValId;
pub use native_function::{NativeFunction, NativeFunctionContext, ValBuilder};
pub use protected_val::ProtectedVal;
pub use unsafe_val::UnsafeVal;

/// A container for a list.
pub type ListVal = Vec<UnsafeVal>;
