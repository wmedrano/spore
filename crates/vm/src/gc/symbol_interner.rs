use std::collections::HashMap;

use compact_str::{CompactString, ToCompactString};

use crate::val::Symbol;

#[derive(Clone, Debug, Default)]
pub struct SymbolInterner {
    vm_id: u16,
    strings: Vec<CompactString>,
    string_to_index: HashMap<CompactString, u32>,
}

impl SymbolInterner {
    pub fn new(vm_id: u16) -> SymbolInterner {
        SymbolInterner {
            vm_id,
            strings: Vec::new(),
            string_to_index: HashMap::new(),
        }
    }

    pub fn symbol_to_str(&self, id: Symbol) -> Option<&str> {
        if id.vm_id != self.vm_id {
            return None;
        }
        self.strings.get(id.idx as usize).map(CompactString::as_str)
    }

    pub fn get_symbol(&self, s: &str) -> Option<Symbol> {
        self.string_to_index.get(s).copied().map(|idx| Symbol {
            vm_id: self.vm_id,
            idx,
        })
    }

    pub fn get_or_create_symbol(&mut self, vm_id: u16, s: &str) -> Symbol {
        assert_eq!(vm_id, self.vm_id);
        let idx = match self.string_to_index.get(s) {
            Some(idx) => *idx,
            None => {
                let idx = self.strings.len() as u32;
                self.strings.push(s.to_compact_string());
                self.string_to_index.insert(s.to_compact_string(), idx);
                idx
            }
        };
        Symbol { vm_id, idx }
    }
}
