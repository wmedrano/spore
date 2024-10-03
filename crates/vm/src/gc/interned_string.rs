use std::collections::HashMap;

use compact_str::{CompactString, ToCompactString};

use crate::val::Symbol;

#[derive(Clone, Debug, Default)]
pub struct StringInterner {
    vm_id: u16,
    strings: Vec<CompactString>,
    string_to_index: HashMap<CompactString, u32>,
}

impl StringInterner {
    pub fn new(vm_id: u16) -> StringInterner {
        StringInterner {
            vm_id,
            strings: Vec::new(),
            string_to_index: HashMap::new(),
        }
    }

    pub fn get_str(&self, id: Symbol) -> Option<&str> {
        if id.vm_id != self.vm_id {
            return None;
        }
        self.strings.get(id.idx as usize).map(CompactString::as_str)
    }

    pub fn get(&self, s: &str) -> Option<Symbol> {
        self.string_to_index.get(s).copied().map(|idx| Symbol {
            vm_id: self.vm_id,
            idx,
        })
    }

    pub fn get_or_insert(&mut self, vm_id: u16, s: &str) -> Symbol {
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
