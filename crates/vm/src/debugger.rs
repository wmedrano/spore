//! Defines the Spore VM Debugger for introspecting into a live VM.
#[allow(unused_imports)]
use log::*;

use crate::val::Instruction;

/// A Spore debugger allows visibility into the Spore virtual machine. Adding logic through the
/// debugger is often costly. For best performance, consider using no debugger with the
/// `DefaultDebugger`.
pub trait Debugger {
    /// Runs for each bytecode instruction.
    fn execute_instruction(&mut self, _: &Instruction) {}
}

/// Implements the [Debugger] trait.
///
/// This doesn't do anything with the debugging information and therefore has no performance
/// penalty.
#[derive(Copy, Clone, Debug, Default)]
pub struct DefaultDebugger;

impl Debugger for DefaultDebugger {}

/// Logs all instructions.
#[derive(Copy, Clone, Debug, Default)]
pub enum LogDebugger {
    #[default]
    Stderr,
    Log,
}

impl Debugger for LogDebugger {
    fn execute_instruction(&mut self, i: &Instruction) {
        match self {
            LogDebugger::Stderr => eprintln!("Instruction: {i:?}"),
            LogDebugger::Log => info!("Instruction: {i:?}"),
        }
    }
}
