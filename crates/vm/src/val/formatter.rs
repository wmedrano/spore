use crate::Vm;

use super::{internal::InternalValImpl, InternalVal};

pub struct ValFormatter<'a> {
    vm: &'a Vm,
    v: InternalVal,
    quote_strings: bool,
}

impl<'a> ValFormatter<'a> {
    /// Create a new value formatter that implements display.
    pub fn new(vm: &'a Vm, v: InternalVal) -> ValFormatter {
        ValFormatter {
            vm,
            v,
            quote_strings: false,
        }
    }

    /// Create a new value formatter that implements display. Strings are printed in quotes. For
    /// example, a string containing the string test-string will print to "test-string".
    pub fn new_quoted(vm: &'a Vm, v: InternalVal) -> ValFormatter {
        ValFormatter {
            vm,
            v,
            quote_strings: true,
        }
    }
}

impl<'a> std::fmt::Display for ValFormatter<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.v.0 {
            InternalValImpl::Void => Ok(()),
            InternalValImpl::Bool(x) => write!(f, "{x}"),
            InternalValImpl::Int(x) => write!(f, "{x}"),
            InternalValImpl::Float(x) => write!(f, "{x}"),
            // TODO: Allow printing with quotes.
            InternalValImpl::String(x) => {
                if self.quote_strings {
                    write!(f, "{:?}", self.vm.val_store.get_str(*x))
                } else {
                    write!(f, "{}", self.vm.val_store.get_str(*x))
                }
            }
            InternalValImpl::List(x) => {
                write!(f, "(")?;
                for (idx, val) in self.vm.val_store.get_list(*x).iter().enumerate() {
                    if idx == 0 {
                        write!(f, "{}", val.format_quoted(self.vm))?;
                    } else {
                        write!(f, " {}", val.format_quoted(self.vm))?;
                    }
                }
                write!(f, ")")
            }
            InternalValImpl::ByteCodeFunction(bc) => {
                let bc = self.vm.val_store.get_bytecode(*bc);
                write!(
                    f,
                    "<function {name}>",
                    name = if bc.name.is_empty() {
                        "_"
                    } else {
                        bc.name.as_str()
                    }
                )
            }
            InternalValImpl::NativeFunction(_) => write!(f, "<native-function>"),
            InternalValImpl::Custom(c) => {
                let c = self.vm.val_store.get_custom(*c);
                write!(f, "{c}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_void_is_empty() {
        assert_eq!(
            InternalVal::from(()).formatted(&Vm::default()).to_string(),
            ""
        );
    }

    #[test]
    fn format_bool_is_true_or_false() {
        assert_eq!(
            InternalVal::from(true)
                .formatted(&Vm::default())
                .to_string(),
            "true"
        );
        assert_eq!(
            InternalVal::from(false)
                .formatted(&Vm::default())
                .to_string(),
            "false"
        );
    }

    #[test]
    fn format_int_prints_number() {
        assert_eq!(
            InternalVal::from(0).formatted(&Vm::default()).to_string(),
            "0"
        );
        assert_eq!(
            InternalVal::from(-1).formatted(&Vm::default()).to_string(),
            "-1"
        );
    }

    #[test]
    fn format_float_prints_number() {
        assert_eq!(
            InternalVal::from(0.0).formatted(&Vm::default()).to_string(),
            "0"
        );
        assert_eq!(
            InternalVal::from(-1.5)
                .formatted(&Vm::default())
                .to_string(),
            "-1.5"
        );
    }

    #[test]
    fn format_string_produces_string() {
        let mut vm = Vm::default();
        let string_id = vm.val_store.insert_string("my string".into());
        assert_eq!(
            InternalVal::from(string_id).formatted(&vm).to_string(),
            "my string"
        );
    }

    #[test]
    fn format_function_prints_name() {
        let mut vm = Vm::default();
        let v = vm.eval_str("(define (foo) 10) foo").unwrap();
        assert_eq!(v.to_string(), "<function foo>");
    }

    #[test]
    fn format_native_function_prints_native_function() {
        let mut vm = Vm::default();
        let v = vm.eval_str("+").unwrap();
        assert_eq!(v.to_string(), "<native-function>");
    }
}
