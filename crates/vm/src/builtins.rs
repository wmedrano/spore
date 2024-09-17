use crate::{
    error::{VmError, VmResult},
    val::{
        native_function::{NativeFunction, NativeFunctionContext},
        InternalVal,
    },
    Vm,
};

pub const BUILTINS: &[(&str, NativeFunction)] = &[
    ("+", add),
    ("<", less),
    ("string-join", string_join),
    ("list", list),
    ("working-directory", working_directory),
];

pub fn add(ctx: NativeFunctionContext) -> VmResult<InternalVal> {
    let mut int_sum: i64 = 0;
    let mut float_sum: f64 = 0.0;
    let mut has_float = false;
    for arg in ctx.args() {
        match arg {
            InternalVal::Int(x) => int_sum += *x,
            InternalVal::Float(x) => {
                float_sum += *x;
                has_float = true;
            }
            v => {
                return Err(VmError::TypeError {
                    context: "+",
                    expected: InternalVal::INT_TYPE_NAME,
                    actual: v.type_name(),
                    value: v.formatted(ctx.vm()).to_string(),
                })
            }
        }
    }
    if has_float {
        Ok(InternalVal::Float(float_sum + int_sum as f64))
    } else {
        Ok(InternalVal::Int(int_sum))
    }
}

fn less_two_impl(vm: &Vm, a: &InternalVal, b: &InternalVal) -> VmResult<bool> {
    match (a, b) {
        (InternalVal::Int(a), InternalVal::Int(b)) => Ok(*a < *b),
        (InternalVal::Float(a), InternalVal::Float(b)) => Ok(*a < *b),
        (InternalVal::Float(a), InternalVal::Int(b)) => Ok(*a < (*b as f64)),
        (InternalVal::Int(a), InternalVal::Float(b)) => Ok((*a as f64) < *b),
        (a, InternalVal::Int(_)) | (a, InternalVal::Float(_)) => Err(VmError::TypeError {
            context: "<",
            expected: "int or float",
            actual: a.type_name(),
            value: a.formatted(vm).to_string(),
        }),
        (_, b) => Err(VmError::TypeError {
            context: "<",
            expected: "int or float",
            actual: b.type_name(),
            value: b.formatted(vm).to_string(),
        }),
    }
}

pub fn less_impl(vm: &Vm, args: &[InternalVal]) -> VmResult<InternalVal> {
    match args {
        [] | [_] => Ok(InternalVal::Bool(true)),
        [a, b] => Ok(InternalVal::Bool(less_two_impl(vm, a, b)?)),
        [a, b, ..] => match less_two_impl(vm, a, b)? {
            true => less_impl(vm, &args[1..]),
            false => Ok(InternalVal::Bool(false)),
        },
    }
}

pub fn less(ctx: NativeFunctionContext) -> VmResult<InternalVal> {
    less_impl(ctx.vm(), ctx.args())
}

pub fn string_join(mut ctx: NativeFunctionContext) -> VmResult<InternalVal> {
    let args = ctx.args();
    let (strings, separator) = match args {
        [] => {
            return Err(VmError::ArityError {
                function: "string-join".to_string(),
                expected: 1,
                actual: 0,
            })
        }
        [InternalVal::List(list)] => (*list, ""),
        [v] => {
            return Err(VmError::TypeError {
                context: "string-join",
                expected: InternalVal::LIST_TYPE_NAME,
                actual: v.type_name(),
                value: v.format_quoted(ctx.vm()).to_string(),
            });
        }
        [InternalVal::List(list), InternalVal::String(string)] => {
            (*list, ctx.vm().val_store.get_str(*string))
        }
        [_, v] => {
            return Err(VmError::TypeError {
                context: "string-join",
                expected: InternalVal::STRING_TYPE_NAME,
                actual: v.type_name(),
                value: v.format_quoted(ctx.vm()).to_string(),
            });
        }
        _ => {
            return Err(VmError::ArityError {
                function: "string-join".to_string(),
                expected: 2,
                actual: args.len(),
            })
        }
    };
    let mut result = String::new();
    for (idx, string_id) in ctx.vm().val_store.get_list(strings).iter().enumerate() {
        if idx > 0 {
            result.push_str(separator);
        }
        match string_id {
            InternalVal::String(string_id) => {
                result.push_str(ctx.vm().val_store.get_str(*string_id));
            }
            v => {
                return Err(VmError::TypeError {
                    context: "string-join",
                    expected: InternalVal::STRING_TYPE_NAME,
                    actual: v.type_name(),
                    value: v.format_quoted(ctx.vm()).to_string(),
                })
            }
        };
    }
    // Unsafe OK: Value is returned immediately so vm does not have chance to run garbage
    // collection.
    Ok(unsafe { ctx.new_string(result) })
}

pub fn list(mut ctx: NativeFunctionContext) -> VmResult<InternalVal> {
    let list = ctx.args().to_vec();
    // Unsafe OK: Value is returned immediately so vm does not have chance to run garbage
    // collection.
    Ok(unsafe { ctx.new_list(list) })
}

pub fn working_directory(mut ctx: NativeFunctionContext) -> VmResult<InternalVal> {
    let arg_len = ctx.arg_len();
    if arg_len != 0 {
        return Err(VmError::ArityError {
            function: "working-directory".to_string(),
            expected: 0,
            actual: arg_len,
        });
    }
    let working_directory = match std::env::current_dir() {
        Ok(path) => path.to_string_lossy().to_string(),
        Err(err) => return Err(VmError::CustomError(err.to_string())),
    };
    // Unsafe OK: Value is returned immediately so vm does not have chance to run garbage
    // collection.
    Ok(unsafe { ctx.new_string(working_directory) })
}
