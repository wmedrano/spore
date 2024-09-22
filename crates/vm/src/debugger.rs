//! Defines the Spore VM Debugger for introspecting into a live VM.
use std::fmt::Debug;

#[allow(unused_imports)]
use log::*;

use crate::val::{Instruction, ValId};

/// A Spore debugger allows visibility into the Spore virtual machine. Adding logic through the
/// debugger is often costly. For best performance, consider using no debugger with the
/// `DefaultDebugger`.
pub trait Debugger {
    /// Ran at the start of compilation.
    fn start_compile(&mut self) {}

    /// Ran at the start of VM evaluation.
    fn start_eval(&mut self) {}

    /// Runs for each bytecode instruction.
    fn execute_instruction(&mut self, _: &Instruction) {}

    /// Runs at the start of every GC cycle.
    fn start_gc(&mut self) {}

    /// Runs at the end of every GC cycle.
    fn end_gc(&mut self) {}

    /// Runs when a garbage collected value has been created.
    ///
    /// # Arguments
    /// - `&mut self` - A mutable reference to `self`.
    /// - `ValId<T>` - The id of the newly allocated garbage collected value.
    /// - `&T` - The garbage collected value.
    /// - `_slot_is_recycled` - True if the base allocation was reclaimed from a previous
    ///   allocation.
    fn gc_insert<T: Debug>(&mut self, _: ValId<T>, _: &T, _slot_is_recycled: bool) {}

    /// Runs when a garbage collected value is about to be garbage collected.
    fn gc_remove<T: Debug>(&mut self, _: ValId<T>, _: &T) {}
}

/// Implements the [Debugger] trait.
///
/// This doesn't do anything with the debugging information and therefore has no performance
/// penalty.
pub struct DefaultDebugger;

impl Debugger for DefaultDebugger {}
