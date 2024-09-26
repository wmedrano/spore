use crate::Vm;

use super::UnsafeVal;

/// Formats a value for display.
pub struct ValFormatter<'a> {
    vm: &'a Vm,
    val: UnsafeVal,
    quote_strings: bool,
}

impl<'a> ValFormatter<'a> {
    /// Create a new value formatter that implements display.
    pub fn new(vm: &'a Vm, v: UnsafeVal) -> ValFormatter {
        ValFormatter {
            vm,
            val: v,
            quote_strings: false,
        }
    }

    /// Create a new value formatter that implements display. Strings are printed in quotes. For
    /// example, a string containing the string test-string will print to "test-string".
    pub fn new_quoted(vm: &'a Vm, v: UnsafeVal) -> ValFormatter {
        ValFormatter {
            vm,
            val: v,
            quote_strings: true,
        }
    }
}

impl<'a> std::fmt::Display for ValFormatter<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.val {
            UnsafeVal::Void => write!(f, "<void>"),
            UnsafeVal::Bool(x) => write!(f, "{x}"),
            UnsafeVal::Int(x) => write!(f, "{x}"),
            UnsafeVal::Float(x) => write!(f, "{x}"),
            // TODO: Allow printing with quotes.
            UnsafeVal::String(x) => {
                if self.quote_strings {
                    write!(f, "{:?}", self.vm.objects.get_str(*x))
                } else {
                    write!(f, "{}", self.vm.objects.get_str(*x))
                }
            }
            UnsafeVal::MutableBox(x) => {
                let inner = ValFormatter {
                    vm: self.vm,
                    val: *self.vm.objects.get_mutable_box(*x),
                    quote_strings: true,
                };
                write!(f, "box<{}>", inner)
            }
            UnsafeVal::List(x) => {
                write!(f, "(")?;
                for (idx, val) in self.vm.objects.get_list(*x).iter().enumerate() {
                    if idx == 0 {
                        write!(f, "{}", val.format_quoted(self.vm))?;
                    } else {
                        write!(f, " {}", val.format_quoted(self.vm))?;
                    }
                }
                write!(f, ")")
            }
            UnsafeVal::Struct(x) => {
                write!(f, "(struct")?;
                for (name, val) in self.vm.objects.get_struct(*x).iter() {
                    let val = val.format_quoted(self.vm);
                    write!(f, " '{name} {val}")?;
                }
                write!(f, ")")
            }
            UnsafeVal::ByteCodeFunction(bc) => {
                let bc = self.vm.objects.get_bytecode(*bc);
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
            UnsafeVal::NativeFunction(_) => write!(f, "<native-function>"),
            UnsafeVal::Custom(c) => {
                let c = self.vm.objects.get_custom(*c);
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
            UnsafeVal::from(()).formatted(&Vm::default()).to_string(),
            "<void>"
        );
    }

    #[test]
    fn format_bool_is_true_or_false() {
        assert_eq!(
            UnsafeVal::from(true).formatted(&Vm::default()).to_string(),
            "true"
        );
        assert_eq!(
            UnsafeVal::from(false).formatted(&Vm::default()).to_string(),
            "false"
        );
    }

    #[test]
    fn format_int_prints_number() {
        assert_eq!(
            UnsafeVal::from(0).formatted(&Vm::default()).to_string(),
            "0"
        );
        assert_eq!(
            UnsafeVal::from(-1).formatted(&Vm::default()).to_string(),
            "-1"
        );
    }

    #[test]
    fn format_float_prints_number() {
        assert_eq!(
            UnsafeVal::from(0.0).formatted(&Vm::default()).to_string(),
            "0"
        );
        assert_eq!(
            UnsafeVal::from(-1.5).formatted(&Vm::default()).to_string(),
            "-1.5"
        );
    }

    #[test]
    fn format_string_produces_string() {
        let mut vm = Vm::default();
        let string_id = vm.objects.insert_string("my string".into());
        assert_eq!(
            UnsafeVal::from(string_id).formatted(&vm).to_string(),
            "my string"
        );
    }

    #[test]
    fn format_struct_produces_all_key_values() {
        let mut vm = Vm::default();
        vm.eval_str("(define x (struct \"field-1\" 1 \"field-2\" \"2\"))")
            .unwrap();
        let res = vm.eval_str("x").unwrap();
        assert!(
            res.formatted(res.vm()).to_string() == "(struct 'field-1 1 'field-2 \"2\")"
                || res.formatted(res.vm()).to_string() == "(struct 'field-2 \"2\" 'field-1 1)",
            "{}",
            res.formatted(res.vm()).to_string()
        );
    }

    #[test]
    fn format_mutable_box_produces_underlying_value() {
        let mut vm = Vm::default();
        vm.eval_str("(define x (new-box \"string\"))").unwrap();
        let res = vm.eval_str("x").unwrap();
        assert!(res.get_mutable_box_ref().is_ok());
        assert_eq!(res.formatted(res.vm()).to_string(), "box<\"string\">");
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
