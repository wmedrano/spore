use crate::{
    error::{VmError, VmResult},
    val::Val,
};

pub fn add(args: &[Val]) -> VmResult<Val> {
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
            _ => return Err(VmError::TypeError),
        }
    }
    if has_float {
        Ok(Val::Float(float_sum + int_sum as f64))
    } else {
        Ok(Val::Int(int_sum))
    }
}
