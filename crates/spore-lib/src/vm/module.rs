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

    /// Create a new empty module manager with an empty `global` module.
    pub fn new_empty() -> ModuleManager {
        ModuleManager::new(Module::new(ModuleSource::Global))
    }

    /// Iterate over all the modules.
    pub fn iter(&self) -> impl Iterator<Item = &Module> {
        std::iter::once(&self.global).chain(self.modules.iter())
    }

    /// Adds the given module to the `ModuleManager`. If the module already exists, then it is replaced.
    pub fn add_module(&mut self, module: Module) {
        assert_ne!(module.source, ModuleSource::Global);
        for existing_module in self.modules.iter_mut() {
            if existing_module.source == module.source {
                *existing_module = module;
                return;
            }
        }
        self.modules.push(module);
    }

    /// Remove a module. If the module does not exist or is the global module, then nothing happens.
    pub fn remove_module(&mut self, module: &ModuleSource) {
        assert_ne!(module, &ModuleSource::Global);
        self.modules.retain(|m| &m.source != module);
    }

    /// Get the module with the given source. If the module does not exist, then `None` is returned.
    pub fn get_mut(&mut self, module: &ModuleSource) -> Option<&mut Module> {
        if *module == ModuleSource::Global {
            Some(&mut self.global)
        } else {
            self.modules.iter_mut().find(|m| m.source == *module)
        }
    }

    /// Get the module with the given source. If the module does not exist, then `None` is returned.
    pub fn get(&self, module: &ModuleSource) -> Option<&Module> {
        if *module == ModuleSource::Global {
            Some(&self.global)
        } else {
            self.modules.iter().find(|m| m.source == *module)
        }
    }

    /// Returns `true` if `module` is registered.
    pub fn has_module(&mut self, module: &ModuleSource) -> bool {
        self.get(module).is_some()
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
    pub fn get_value(
        &self,
        module_source: &ModuleSource,
        import_module: Option<impl AsRef<str>>,
        sym: impl Borrow<str>,
    ) -> Option<Val> {
        let sym = sym.borrow();
        if *module_source != ModuleSource::Global {
            if let Some(mut module) = self.modules.iter().find(|m| *module_source == m.source) {
                if let Some(import_module) = import_module {
                    if let Some(ms) = module.imported_modules.get(import_module.as_ref()) {
                        module = self.modules.iter().find(|m| *ms == m.source).unwrap();
                    }
                }
                if let Some(v) = module.get(sym) {
                    return Some(v);
                }
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

/// A module that stores values associated with symbols.
///
/// Modules are used to manage namespaces, allowing for the organization
/// of variables and functions within a program.
#[derive(Clone, Default, Debug)]
pub struct Module {
    /// The source of the module.
    source: ModuleSource,
    /// A map from identifier to the module that has been imported into the current module.
    imported_modules: HashMap<String, ModuleSource>,
    /// A map of symbols to their corresponding values.
    values: HashMap<Symbol, Val>,
}

impl Module {
    /// Create a new empty module.
    pub fn new(source: ModuleSource) -> Module {
        Module {
            source,
            imported_modules: HashMap::new(),
            values: HashMap::new(),
        }
    }

    /// Get the source of the module.
    pub fn source(&self) -> &ModuleSource {
        &self.source
    }

    /// Get a map from an import identifier to the module it points to.
    pub fn imports(&self) -> &HashMap<String, ModuleSource> {
        &self.imported_modules
    }

    /// Add an import
    pub fn add_import(&mut self, module_identifier: String, module: ModuleSource) {
        self.imported_modules.insert(module_identifier, module);
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
            ModuleSource::File(p) => match p.to_str() {
                Some(s) => write!(f, "{s}"),
                None => {
                    write!(f, "{}", p.to_string_lossy())
                }
            },
        }
    }
}
