/// Built in functions and values.
pub mod builtins;

/// Parse strings into tokens and ASTs.
pub mod parser;

/// The virtual machine. The heart of the Spore interpreter.
pub mod vm;

/// Run spore as a repl.
pub mod repl;
