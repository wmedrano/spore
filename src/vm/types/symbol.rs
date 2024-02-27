use std::rc::Rc;

/// A symbol.
#[derive(Clone, Debug, PartialEq, Hash, Eq)]
pub struct Symbol(pub Rc<String>);

impl<'a> From<&'a str> for Symbol {
    fn from(s: &'a str) -> Symbol {
        Symbol::from(s.to_string())
    }
}

impl From<String> for Symbol {
    fn from(s: String) -> Symbol {
        Symbol(Rc::new(s))
    }
}

impl std::fmt::Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "'{}", self.0)
    }
}
