use std::collections::HashMap;

use super::types::{symbol::Symbol, Val};

/// Manages multiple modules and provides access to global and local variables.
///
/// This struct is responsible for organizing and managing different modules,
/// including a global module and local modules. It allows for setting and
/// retrieving values associated with symbols across different namespaces.
#[derive(Clone)]
pub struct ModuleManager {
    /// The global module, accessible from all other modules.
    global: Module,
    /// The name of the currently active module.
    current_module: String,
    /// A map of module names to their corresponding Module instances.
    modules: HashMap<String, Module>,
}

impl ModuleManager {
    /// Creates a new ModuleManager with a given global module.
    ///
    /// Initializes the ModuleManager with the provided global module and
    /// creates a default local module named "%local%".
    ///
    /// # Arguments
    ///
    /// * `global` - The Module to be used as the global module.
    ///
    /// # Returns
    ///
    /// A new ModuleManager instance.
    pub fn new(global: Module) -> ModuleManager {
        ModuleManager {
            global,
            current_module: "".to_string(),
            modules: std::iter::once(("".to_string(), Module::new())).collect(),
        }
    }

    /// Retrieves a value associated with a symbol from the current module or global module.
    ///
    /// First checks the current module for the symbol. If not found, falls back to the global module.
    ///
    /// # Arguments
    ///
    /// * `sym` - A reference to the Symbol to look up.
    ///
    /// # Returns
    ///
    /// An Option<Val> containing the value if found, or None if the symbol is not present in either module.
    pub fn get(&self, sym: &Symbol) -> Option<Val> {
        match self.modules.get(&self.current_module).unwrap().get(sym) {
            Some(v) => Some(v),
            None => self.global.get(sym),
        }
    }

    /// Sets a value for a symbol in the current local module.
    ///
    /// # Arguments
    ///
    /// * `sym` - The Symbol to associate with the value.
    /// * `val` - The Val to be associated with the symbol.
    ///
    /// # Panics
    ///
    /// Panics if the current module does not exist in the modules HashMap.
    pub fn set_local(&mut self, sym: Symbol, val: Val) {
        self.modules
            .get_mut(&self.current_module)
            .unwrap()
            .set(sym, val)
    }
}

/// A module that stores values associated with symbols.
///
/// Modules are used to manage namespaces, allowing for the organization
/// of variables and functions within a program.
#[derive(Clone, Default)]
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
    pub fn new() -> Module {
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
