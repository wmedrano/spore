use std::{borrow::Borrow, sync::Arc};

/// A symbol.
#[derive(Clone, Debug, PartialEq, Hash, Eq)]
pub struct Symbol(Arc<String>);

impl Symbol {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl<'a> From<&'a str> for Symbol {
    fn from(s: &'a str) -> Symbol {
        Symbol(Arc::new(s.to_string()))
    }
}

impl From<String> for Symbol {
    fn from(s: String) -> Symbol {
        Symbol(Arc::new(s))
    }
}

impl AsRef<str> for Symbol {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for Symbol {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "'{}", self.as_str())
    }
}
