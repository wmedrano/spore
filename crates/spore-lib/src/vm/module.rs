use std::{borrow::Borrow, collections::HashMap, path::PathBuf};

use super::types::{symbol::Symbol, Val};

/// Manages multiple modules and provides access to global and local variables.
///
/// This struct is responsible for organizing and managing different modules,
/// including a global module and local modules. It allows for setting and
/// retrieving values associated with symbols across different namespaces.
#[derive(Clone)]
pub struct ModuleManager {
    global: Module,
    modules: Vec<Module>,
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
        let mut global = global;
        global.source = ModuleSource::Global;
        ModuleManager {
            global,
            modules: Vec::new(),
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
    pub fn get(&self, module: &ModuleSource, sym: impl Borrow<str>) -> Option<Val> {
        let sym = sym.borrow();
        if *module != ModuleSource::Global {
            let maybe_val = self
                .modules
                .iter()
                .find(|m| module == &m.source)
                .and_then(|m| m.get(sym));
            if let Some(v) = maybe_val {
                return Some(v);
            }
        }
        self.global.get(sym)
    }

    /// Sets a value for a symbol in the given module. If the module does not exist, then it is created.
    ///
    /// # Arguments
    ///
    /// * `sym` - The Symbol to associate with the value.
    /// * `val` - The Val to be associated with the symbol.
    ///
    /// # Panics
    ///
    /// Panics if the current module does not exist in the modules HashMap.
    pub fn set_value(&mut self, module: &ModuleSource, sym: Symbol, val: Val) {
        let module = match self.modules.iter_mut().find(|m| m.source == *module) {
            Some(m) => m,
            None => {
                self.modules.push(Module::new(module.clone()));
                self.modules.last_mut().unwrap()
            }
        };
        module.set(sym, val);
    }

    /// Returns `true` if `module` is registered.
    pub fn has_module(&mut self, module: &ModuleSource) -> bool {
        self.modules.iter().find(|m| m.source == *module).is_some()
    }
}

/// A module that stores values associated with symbols.
///
/// Modules are used to manage namespaces, allowing for the organization
/// of variables and functions within a program.
#[derive(Clone, Default)]
pub struct Module {
    /// The source of the module.
    source: ModuleSource,
    /// A map of symbols to their corresponding values.
    values: HashMap<Symbol, Val>,
}

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
pub enum ModuleSource {
    /// The global module containing all the builtins.
    #[default]
    Global,
    /// A module that is not backed by any file.
    Virtual(&'static str),
    /// A module that is backed by a file.
    File(PathBuf),
}

impl Module {
    /// Create a new empty module.
    pub fn new(source: ModuleSource) -> Module {
        Module {
            source,
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
    pub fn get(&self, sym: impl Borrow<str>) -> Option<Val> {
        self.values.get(sym.borrow()).cloned()
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

impl std::fmt::Display for ModuleSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModuleSource::Global => write!(f, "%global%"),
            ModuleSource::Virtual(v) => write!(f, "%virtual%/{v}"),
            ModuleSource::File(p) => write!(f, "{p:?}"),
        }
    }
}
