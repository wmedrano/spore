use crate::{
    error::{VmError, VmResult},
    val::{InternalVal, Val},
    Vm,
};

pub fn add(_: &Vm, args: &[InternalVal]) -> VmResult<InternalVal> {
    let mut int_sum: i64 = 0;
    let mut float_sum: f64 = 0.0;
    let mut has_float = false;
    for arg in args {
        match arg {
            InternalVal::Int(x) => int_sum += *x,
            InternalVal::Float(x) => {
                float_sum += *x;
                has_float = true;
            }
            v => {
                return Err(VmError::TypeError {
                    expected: InternalVal::INT_TYPE_NAME,
                    actual: v.type_name(),
                    value: Val { v: v.clone() }.to_string(),
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

fn less_impl(_: &Vm, a: &InternalVal, b: &InternalVal) -> VmResult<bool> {
    match (a, b) {
        (InternalVal::Int(a), InternalVal::Int(b)) => Ok(*a < *b),
        (InternalVal::Float(a), InternalVal::Float(b)) => Ok(*a < *b),
        (InternalVal::Float(a), InternalVal::Int(b)) => Ok(*a < (*b as f64)),
        (InternalVal::Int(a), InternalVal::Float(b)) => Ok((*a as f64) < *b),
        (a, InternalVal::Int(_)) | (a, InternalVal::Float(_)) => Err(VmError::TypeError {
            expected: "int or float",
            actual: a.type_name(),
            value: Val { v: a.clone() }.to_string(),
        }),
        (_, b) => Err(VmError::TypeError {
            expected: "int or float",
            actual: b.type_name(),
            value: Val { v: b.clone() }.to_string(),
        }),
    }
}

pub fn less(vm: &Vm, args: &[InternalVal]) -> VmResult<InternalVal> {
    match args {
        [] | [_] => Ok(InternalVal::Bool(true)),
        [a, b] => Ok(InternalVal::Bool(less_impl(vm, a, b)?)),
        [a, b, ..] => match less_impl(vm, a, b)? {
            true => less(vm, &args[1..]),
            false => Ok(InternalVal::Bool(false)),
        },
    }
}
