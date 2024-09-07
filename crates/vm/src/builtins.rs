use crate::{
    error::{VmError, VmResult},
    val::Val,
    Vm,
};

pub fn add(vm: &Vm, args: &[Val]) -> VmResult<Val> {
    let mut int_sum: i64 = 0;
    let mut float_sum: f64 = 0.0;
    let mut has_float = false;
    for arg in args {
        match arg {
            Val::Int(x) => int_sum += *x,
            Val::Float(x) => {
                float_sum += *x;
                has_float = true;
            }
            v => {
                return Err(VmError::TypeError {
                    expected: Val::INT_TYPE_NAME,
                    actual: v.type_name(),
                    value: vm.formatted_val(v).to_string(),
                })
            }
        }
    }
    if has_float {
        Ok(Val::Float(float_sum + int_sum as f64))
    } else {
        Ok(Val::Int(int_sum))
    }
}

fn less_impl(vm: &Vm, a: &Val, b: &Val) -> VmResult<bool> {
    match (a, b) {
        (Val::Int(a), Val::Int(b)) => Ok(*a < *b),
        (Val::Float(a), Val::Float(b)) => Ok(*a < *b),
        (Val::Float(a), Val::Int(b)) => Ok(*a < (*b as f64)),
        (Val::Int(a), Val::Float(b)) => Ok((*a as f64) < *b),
        (a, Val::Int(_)) | (a, Val::Float(_)) => Err(VmError::TypeError {
            expected: "int or float",
            actual: a.type_name(),
            value: vm.formatted_val(a).to_string(),
        }),
        (_, b) => Err(VmError::TypeError {
            expected: "int or float",
            actual: b.type_name(),
            value: vm.formatted_val(b).to_string(),
        }),
    }
}

pub fn less(vm: &Vm, args: &[Val]) -> VmResult<Val> {
    match args {
        [] | [_] => Ok(Val::Bool(true)),
        [a, b] => Ok(Val::Bool(less_impl(vm, a, b)?)),
        [a, b, ..] => match less_impl(vm, a, b)? {
            true => less(vm, &args[1..]),
            false => Ok(Val::Bool(false)),
        },
    }
}
