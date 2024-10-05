use crate::{
    error::{VmError, VmResult},
    val::{NativeFunction, NativeFunctionContext, UnsafeVal, Val, ValBuilder},
    Vm,
};

pub mod boxes;
pub mod lists;
pub mod numbers;
pub mod strings;
pub mod structs;
pub mod system;

pub const BUILTINS: &[(&str, NativeFunction)] = &[
    ("global-values", global_values),
    ("not", not),
    ("=", equal),
    ("+", numbers::add),
    ("-", numbers::subtract),
    ("<", numbers::less),
    ("string-length", strings::string_length),
    ("string-join", strings::string_join),
    ("list", lists::list),
    ("list-length", lists::list_length),
    ("struct", structs::strct),
    ("struct-get", structs::struct_get),
    ("struct-set!", structs::struct_set),
    ("new-box", boxes::new_box),
    ("set-box!", boxes::set_box),
    ("unbox", boxes::unbox),
    ("working-directory", system::working_directory),
    ("command", system::command),
];

pub fn global_values<'a>(mut ctx: NativeFunctionContext, args: &[Val]) -> VmResult<ValBuilder<'a>> {
    if !args.is_empty() {
        return Err(VmError::ArityError {
            function: "global-values".into(),
            expected: 0,
            actual: args.len(),
        });
    }
    let values: Vec<UnsafeVal> = ctx
        .vm()
        .values
        .keys()
        .map(|s| UnsafeVal::Symbol(*s))
        .collect();
    let v = unsafe { ctx.vm_mut().objects.insert_list(values) };
    Ok(ValBuilder::new(unsafe {
        Val::from_unsafe_val(UnsafeVal::List(v))
    }))
}

pub fn not<'a>(_: NativeFunctionContext, args: &[Val]) -> VmResult<ValBuilder<'a>> {
    match args {
        [v] => Ok(Val::new_bool(!v.is_truthy()).into()),
        args => Err(VmError::ArityError {
            function: "not".into(),
            expected: 1,
            actual: args.len(),
        }),
    }
}

pub fn equal<'a>(ctx: NativeFunctionContext, args: &[Val<'a>]) -> VmResult<ValBuilder<'a>> {
    match args {
        [a, b] => {
            // Unsafe OK: [equal_imp] holds the a reference to the VM so it can't run garbage
            // collection.
            let (a, b) = unsafe { (a.as_unsafe_val(), b.as_unsafe_val()) };
            Ok(Val::new_bool(equal_impl(ctx.vm(), a, b)).into())
        }
        _ => Err(VmError::ArityError {
            function: "=".into(),
            expected: 2,
            actual: args.len(),
        }),
    }
}

pub fn equal_impl(vm: &Vm, a: UnsafeVal, b: UnsafeVal) -> bool {
    use crate::val::UnsafeVal::*;
    match (a, b) {
        (Void, Void) => true,
        (Bool(a), Bool(b)) => a == b,
        (Int(a), Int(b)) => a == b,
        (Float(a), Float(b)) => a == b,
        (String(a), String(b)) => vm.objects.get_str(a) == vm.objects.get_str(b),
        (List(a), List(b)) => {
            let a = vm.objects.get_list(a);
            let b = vm.objects.get_list(b);
            if a == b {
                return true;
            }
            if a.len() != b.len() {
                return false;
            }
            a.iter().zip(b.iter()).all(|(a, b)| equal_impl(vm, *a, *b))
        }
        (Struct(a), Struct(b)) => {
            if a == b {
                return true;
            }
            let a = vm.objects.get_struct(a);
            let b = vm.objects.get_struct(b);
            if a.len() != b.len() {
                return false;
            }
            for (k, v) in a.iter() {
                let other = match b.get(k) {
                    Some(other) => other,
                    None => return false,
                };
                if !equal_impl(vm, v, other) {
                    return false;
                }
            }
            true
        }
        (ByteCodeFunction(a), ByteCodeFunction(b)) => a == b,
        (NativeFunction(a), NativeFunction(b)) => a == b,
        _ => false,
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn global_values_returns_list() {
        let mut vm = Vm::default();
        assert_ne!(
            vm.eval_str("(list-length (global-values))")
                .unwrap()
                .try_int()
                .unwrap(),
            0
        );
    }

    #[test]
    fn global_values_with_args_returns_error() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("(global-values 0)").unwrap_err(),
            VmError::ArityError {
                function: "global-values".into(),
                expected: 0,
                actual: 1
            }
        );
    }

    #[test]
    fn equal_with_wrong_number_of_args_produces_arity_error() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("(=)").unwrap_err(),
            VmError::ArityError {
                function: "=".into(),
                expected: 2,
                actual: 0,
            },
        );
        assert_eq!(
            vm.eval_str("(= 1)").unwrap_err(),
            VmError::ArityError {
                function: "=".into(),
                expected: 2,
                actual: 1,
            },
        );
        assert_eq!(
            vm.eval_str("(= 1 2 3)").unwrap_err(),
            VmError::ArityError {
                function: "=".into(),
                expected: 2,
                actual: 3,
            },
        );
    }

    #[test]
    fn equal_with_equal_items_returns_true() {
        let mut vm = Vm::default();
        assert!(vm.eval_str("(= false false)").unwrap().try_bool().unwrap());
        assert!(vm.eval_str("(= 1 1)").unwrap().try_bool().unwrap());
        assert!(vm.eval_str("(= 2.0 2.0)").unwrap().try_bool().unwrap());
        assert!(vm
            .eval_str("(= \"string\" \"string\")")
            .unwrap()
            .try_bool()
            .unwrap());
        assert!(vm
            .eval_str("(= (list \"list\") (list \"list\"))")
            .unwrap()
            .try_bool()
            .unwrap());
        assert!(vm
            .eval_str("(= (struct 'field 1) (struct 'field 1))")
            .unwrap()
            .try_bool()
            .unwrap());
        vm.eval_str("(define (foo) 42)").unwrap();
        assert!(vm.eval_str("(= foo foo)").unwrap().try_bool().unwrap());
        assert!(vm.eval_str("(= (foo) (foo))").unwrap().try_bool().unwrap());
        assert!(vm.eval_str("(= + +)").unwrap().try_bool().unwrap());
        assert!(vm.eval_str("(= void void)").unwrap().try_bool().unwrap());
    }

    #[test]
    fn equal_with_same_struct_ref_returns_true() {
        let mut vm = Vm::default();
        vm.eval_str("(define my-struct (struct 'a 1))").unwrap();
        vm.eval_str("(struct-set! my-struct 'b my-struct)").unwrap();
        assert!(vm
            .eval_str("(= my-struct my-struct)")
            .unwrap()
            .try_bool()
            .unwrap());
    }

    #[test]
    fn equal_with_same_list_ref_returns_true() {
        let mut vm = Vm::default();
        vm.eval_str("(define my-list (list 1 2))").unwrap();
        assert!(vm
            .eval_str("(= my-list my-list)")
            .unwrap()
            .try_bool()
            .unwrap());
    }

    #[test]
    fn equal_with_different_items_returns_false() {
        let mut vm = Vm::default();
        assert!(!vm.eval_str("(= 1 1.0)").unwrap().try_bool().unwrap());
        assert!(!vm.eval_str("(= true false)").unwrap().try_bool().unwrap());
        assert!(!vm.eval_str("(= 1 2)").unwrap().try_bool().unwrap());
        assert!(!vm.eval_str("(= 1.0 2.0)").unwrap().try_bool().unwrap());
        assert!(!vm
            .eval_str("(= \"string\" \"other\")")
            .unwrap()
            .try_bool()
            .unwrap());
        assert!(!vm
            .eval_str("(= (list) (list 0))")
            .unwrap()
            .try_bool()
            .unwrap());
        assert!(!vm
            .eval_str("(= (list \"list\" 1) (list \"list\" 2))")
            .unwrap()
            .try_bool()
            .unwrap());
        assert!(!vm
            .eval_str("(= (struct 'field 1) (struct 'field 2))")
            .unwrap()
            .try_bool()
            .unwrap());
        vm.eval_str("(define (foo) 42) (define (bar) 42)").unwrap();
        assert!(!vm.eval_str("(= foo bar)").unwrap().try_bool().unwrap());
        assert!(!vm.eval_str("(= + <)").unwrap().try_bool().unwrap());
    }

    #[test]
    fn not_inverts_bool() {
        let mut vm = Vm::default();
        assert!(!vm.eval_str("(not true)").unwrap().try_bool().unwrap());
        assert!(vm.eval_str("(not false)").unwrap().try_bool().unwrap());
    }

    #[test]
    fn not_with_wrong_not_just_one_arg_produces_arity_error() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("(not)").unwrap_err(),
            VmError::ArityError {
                function: "not".into(),
                expected: 1,
                actual: 0,
            }
        );
        assert_eq!(
            vm.eval_str("(not true false)").unwrap_err(),
            VmError::ArityError {
                function: "not".into(),
                expected: 1,
                actual: 2,
            }
        );
    }

    #[test]
    fn not_with_void_values_returns_true() {
        let mut vm = Vm::default();
        assert!(vm.eval_str("(not void)").unwrap().try_bool().unwrap());
    }

    #[test]
    fn not_with_truthy_values_returns_true() {
        let mut vm = Vm::default();
        assert!(!vm.eval_str("(not true)").unwrap().try_bool().unwrap());
        assert!(!vm.eval_str("(not 1)").unwrap().try_bool().unwrap());
        assert!(!vm.eval_str("(not 1.0)").unwrap().try_bool().unwrap());
        assert!(!vm.eval_str("(not \"\")").unwrap().try_bool().unwrap());
        assert!(!vm.eval_str("(not not)").unwrap().try_bool().unwrap());
        assert!(!vm.eval_str("(not (list))").unwrap().try_bool().unwrap());
    }
}
