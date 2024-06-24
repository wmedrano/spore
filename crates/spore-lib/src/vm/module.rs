use std::collections::HashMap;

use super::types::{symbol::Symbol, Val};

/// A module that stores values associated with symbols.
///
/// Modules are used to manage namespaces, allowing for the organization
/// of variables and functions within a program.
#[derive(Clone)]
pub struct Module {
    /// A map of symbols to their corresponding values.
    values: HashMap<Symbol, Val>,
}

impl Module {
    /// Creates a new module with an optional name.
    ///
    /// The name is not used within this implementation but can be used
    /// for debugging or logging purposes.
    ///
    /// # Arguments
    ///
    /// * `_name` - A value that can be converted into a `String`, representing
    ///             the name of the module.
    ///
    /// # Returns
    ///
    /// A new `Module` instance.
    pub fn new(_name: impl Into<String>) -> Module {
        Module {
            values: HashMap::new(),
        }
    }

    /// Retrieves the value associated with a given symbol.
    ///
    /// # Arguments
    ///
    /// * `sym` - A reference to the `Symbol` for which to retrieve the value.
    ///
    /// # Returns
    ///
    /// An `Option<Val>` that is `Some(Val)` if the symbol exists in the module,
    /// and `None` if the symbol does not exist.
    pub fn get(&self, sym: &Symbol) -> Option<Val> {
        self.values.get(sym).cloned()
    }

    /// Sets the value associated with a given symbol.
    ///
    /// If the symbol already exists in the module, its value is overwritten.
    ///
    /// # Arguments
    ///
    /// * `sym` - The `Symbol` with which to associate the value.
    /// * `val` - The `Val` to associate with the symbol.
    pub fn set(&mut self, sym: Symbol, val: Val) {
        self.values.insert(sym, val);
    }
}
