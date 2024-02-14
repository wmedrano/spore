use anyhow::{bail, Result};

use crate::vm::{
    types::{Number, Procedure, Val},
    Vm,
};

/// Register all builtin functions.
pub fn register_all(vm: &Vm) {
    vm.register_fns(
        [
            Procedure::new(Some("+"), add_fn),
            Procedure::new(Some("-"), sub_fn),
            Procedure::new(Some("*"), multiply_fn),
            Procedure::new(Some("/"), divide_fn),
            Procedure::new(Some("list"), list_fn),
        ]
        .into_iter(),
    )
}

fn ensure_numbers(op: &str, args: &[Val]) -> Result<()> {
    for arg in args {
        match arg {
            Val::Number(_) => (),
            _ => bail!("{op} expected number but got {arg}",),
        }
    }
    Ok(())
}

/// Add all the values in `args`. If no values are present in `args`, then `0` is returned.
fn add_fn(args: &[Val]) -> Result<Val> {
    ensure_numbers("+", args)?;
    let res = match args {
        [] => Number::Int(0).into(),
        [x] => x.clone(),
        [x, y] => add_two(x, y),
        [x, y, zs @ ..] => {
            let mut res = add_two(x, y);
            for z in zs {
                res = add_two(&res, z);
            }
            res
        }
    };
    Ok(res)
}

/// Subtract from the first argument all the rest of the arguments. If there is only a single
/// argument, then it is negated.
fn sub_fn(args: &[Val]) -> Result<Val> {
    ensure_numbers("-", args)?;
    let res = match args {
        [] => bail!("- requires at least 1 arg"),
        [x] => negate(x),
        [x, ys @ ..] => {
            let sub_part = add_fn(ys)?;
            add_two(x, &negate(&sub_part))
        }
    };
    Ok(res)
}

/// Divide the first argument by the rest of the arguments. If only a single argument is provided,
/// then the reciprocal of it is returned.
fn divide_fn(args: &[Val]) -> Result<Val> {
    ensure_numbers("/", args)?;
    match args {
        [] => bail!("/ requires at least 1 arg"),
        [x] => Ok(reciprocal(x)),
        [x, ys @ ..] => {
            let denom = multiply_fn(ys)?;
            Ok(multiply_two(x, &reciprocal(&denom)))
        }
    }
}

/// Multiply all arguments in `args`. If there are no values, then `1` is returned.
fn multiply_fn(args: &[Val]) -> Result<Val> {
    ensure_numbers("*", args)?;
    let res = match args {
        [] => Number::Int(1).into(),
        [x] => x.clone(),
        [x, y] => multiply_two(x, y),
        [x, y, zs @ ..] => {
            let mut res = multiply_two(x, y);
            for z in zs {
                res = multiply_two(&res, z);
            }
            res
        }
    };
    Ok(res)
}

/// Return `args` as a list.
fn list_fn(args: &[Val]) -> Result<Val> {
    let items = Vec::from_iter(args.iter().cloned());
    Ok(items.into())
}

fn negate(x: &Val) -> Val {
    match x {
        Val::Number(x) => match x {
            Number::Int(x) => Number::Int(-x).into(),
            Number::Float(x) => Number::Float(-x).into(),
        },
        _ => unreachable!(),
    }
}

fn reciprocal(x: &Val) -> Val {
    match x {
        Val::Number(Number::Float(x)) => Number::Float(x.recip()).into(),
        _ => unreachable!(),
    }
}

fn add_two(x: &Val, y: &Val) -> Val {
    match (x, y) {
        (Val::Number(x), Val::Number(y)) => match (x, y) {
            (Number::Int(x), Number::Int(y)) => Number::Int(x + y).into(),
            (Number::Float(x), Number::Float(y)) => Number::Float(x + y).into(),
            (Number::Int(x), Number::Float(y)) | (Number::Float(y), Number::Int(x)) => {
                Number::Float(*x as f64 + y).into()
            }
        },
        _ => unreachable!(),
    }
}

fn multiply_two(x: &Val, y: &Val) -> Val {
    match (x, y) {
        (Val::Number(x), Val::Number(y)) => match (x, y) {
            (Number::Int(x), Number::Int(y)) => Number::Int(x * y).into(),
            (Number::Float(x), Number::Float(y)) => Number::Float(x * y).into(),
            (Number::Int(x), Number::Float(y)) | (Number::Float(y), Number::Int(x)) => {
                Number::Float(*x as f64 * y).into()
            }
        },
        _ => unreachable!(),
    }
}
