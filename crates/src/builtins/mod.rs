use std::rc::Rc;

use anyhow::{anyhow, bail, Result};

use crate::vm::{
    types::{proc::native::NativeProc, Val},
    Vm,
};

/// Register all builtin functions.
pub fn register_all(vm: &mut Vm) {
    vm.register_global_fn([
        NativeProc::new("%no-op", no_op_fn),
        NativeProc::new("list", list_fn),
        NativeProc::new("len", len_fn),
        NativeProc::new("substring", substring_fn),
        NativeProc::new("string-concat", string_concat_fn),
        NativeProc::new("+", add_fn),
        NativeProc::new("-", sub_fn),
        NativeProc::new("*", multiply_fn),
        NativeProc::new("/", divide_fn),
        NativeProc::new("<", less_fn),
        NativeProc::new("<=", less_eq_fn),
        NativeProc::new(">", greater_fn),
        NativeProc::new(">=", greater_eq_fn),
        NativeProc::new("equal?", equalp_fn),
    ])
    .unwrap()
}

fn ensure_numbers(op: &str, args: &[Val]) -> Result<()> {
    for arg in args {
        match arg {
            Val::Int(_) | Val::Float(_) => (),
            _ => bail!("{op} expected number but got {arg}",),
        }
    }
    Ok(())
}

fn list_fn(args: &[Val]) -> Result<Val> {
    let ret = Val::List(Rc::from(args.to_vec()));
    Ok(ret)
}

fn len_fn(args: &[Val]) -> Result<Val> {
    match args {
        [] => bail!("len expected at least 1 argument."),
        [arg] => match arg {
            Val::List(lst) => Ok(Val::Int(lst.len() as isize)),
            Val::String(s) => Ok(Val::Int(s.len() as isize)),
            v => bail!("expected <list> or <string> but found {}", v.type_name()),
        },
        _ => bail!("len expected only 1 argument."),
    }
}

fn substring_fn(args: &[Val]) -> Result<Val> {
    match args {
        [s, start, end] => {
            let s = s.try_str()?;
            let start = start.try_usize()?;
            let end = end.try_usize()?;
            if start > end {
                bail!("assertion start <= end ({start} <= {end}) failed");
            }
            if end > s.len() {
                bail!(
                    "assertion end < len(str) ({end} < {len}) failed",
                    len = s.len()
                );
            }
            Ok(Val::String(Rc::new(String::from(&s[start..end]))))
        }
        _ => bail!(
            "substring expected 3 arguments but found {n}.",
            n = args.len()
        ),
    }
}

fn string_concat_fn(args: &[Val]) -> Result<Val> {
    let strs: Result<Vec<_>> = args.iter().map(|v| v.try_str()).collect();
    let res = strs?.join("");
    Ok(Val::String(Rc::new(res)))
}

fn no_op_fn(args: &[Val]) -> Result<Val> {
    let res = args.last().cloned().unwrap_or(Val::Void);
    Ok(res)
}

/// Add all the values in `args`. If no values are present in `args`, then `0` is returned.
fn add_fn(args: &[Val]) -> Result<Val> {
    ensure_numbers("+", args)?;
    let res = match args {
        [] => 0.into(),
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
        [] => Err(anyhow!("/ requires at least 1 arg")),
        [x] => Ok(reciprocal(x)),
        [x, ys @ ..] => {
            let denom = multiply_fn(ys)?;
            Ok(multiply_two(x, &reciprocal(&denom)))
        }
    }
}

fn less_fn(args: &[Val]) -> Result<Val> {
    ensure_numbers("<", args)?;
    let res = match args {
        [Val::Int(x), Val::Int(y)] => x < y,
        [Val::Float(x), Val::Float(y)] => x < y,
        [Val::Int(x), Val::Float(y)] => (*x as f64) < *y,
        [Val::Float(x), Val::Int(y)] => *x < (*y as f64),
        _ => bail!("< requires 2 numbers but found {:?}", args),
    };
    Ok(res.into())
}

fn less_eq_fn(args: &[Val]) -> Result<Val> {
    ensure_numbers("<=", args)?;
    let res = match args {
        [Val::Int(x), Val::Int(y)] => x <= y,
        [Val::Float(x), Val::Float(y)] => x <= y,
        [Val::Int(x), Val::Float(y)] => (*x as f64) <= *y,
        [Val::Float(x), Val::Int(y)] => *x <= (*y as f64),
        _ => bail!("< requires 2 numbers but found {:?}", args),
    };
    Ok(res.into())
}

fn greater_fn(args: &[Val]) -> Result<Val> {
    ensure_numbers(">", args)?;
    let res = match args {
        [Val::Int(x), Val::Int(y)] => x > y,
        [Val::Float(x), Val::Float(y)] => x > y,
        [Val::Int(x), Val::Float(y)] => (*x as f64) > *y,
        [Val::Float(x), Val::Int(y)] => *x > (*y as f64),
        _ => bail!("> requires 2 args but found {}", args.len()),
    };
    Ok(res.into())
}

fn greater_eq_fn(args: &[Val]) -> Result<Val> {
    ensure_numbers(">=", args)?;
    let res = match args {
        [Val::Int(x), Val::Int(y)] => x >= y,
        [Val::Float(x), Val::Float(y)] => x >= y,
        [Val::Int(x), Val::Float(y)] => (*x as f64) >= *y,
        [Val::Float(x), Val::Int(y)] => *x >= (*y as f64),
        _ => bail!("> requires 2 args but found {}", args.len()),
    };
    Ok(res.into())
}

/// Multiply all arguments in `args`. If there are no values, then `1` is returned.
fn multiply_fn(args: &[Val]) -> Result<Val> {
    ensure_numbers("*", args)?;
    let res = match args {
        [] => 1.into(),
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

fn equalp_fn(args: &[Val]) -> Result<Val> {
    match args {
        [a, b] => Ok(Val::Bool(a == b)),
        _ => Err(anyhow!(
            "equal? expects 2 arguments but found {}",
            args.len()
        )),
    }
}

fn negate(x: &Val) -> Val {
    match x {
        Val::Int(x) => (-*x).into(),
        Val::Float(x) => (-*x).into(),
        _ => unreachable!(),
    }
}

fn reciprocal(x: &Val) -> Val {
    match x {
        Val::Int(x) => ((*x as f64).recip()).into(),
        Val::Float(x) => x.recip().into(),
        _ => unreachable!(),
    }
}

fn add_two(x: &Val, y: &Val) -> Val {
    match (x, y) {
        (Val::Int(x), Val::Int(y)) => (x + y).into(),
        (Val::Float(x), Val::Float(y)) => (x + y).into(),
        (Val::Int(x), Val::Float(y)) => ((*x as f64) + *y).into(),
        (Val::Float(x), Val::Int(y)) => (*x + (*y as f64)).into(),
        (a, b) => unreachable!("tried to add {a} and {b}"),
    }
}

fn multiply_two(x: &Val, y: &Val) -> Val {
    match (x, y) {
        (Val::Int(x), Val::Int(y)) => (x * y).into(),
        (Val::Float(x), Val::Float(y)) => (x * y).into(),
        (Val::Int(x), Val::Float(y)) => ((*x as f64) * *y).into(),
        (Val::Float(x), Val::Int(y)) => (*x * (*y as f64)).into(),
        _ => unreachable!(),
    }
}
