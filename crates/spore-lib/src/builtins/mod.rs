use std::rc::Rc;

use anyhow::{anyhow, bail, ensure, Result};

use crate::vm::{
    module::{Module, ModuleManager, ModuleSource},
    types::{proc::native::NativeProc, Val},
};

/// Register all builtin functions.
pub fn global_module() -> Module {
    let mut module = Module::new(ModuleSource::Global);
    for proc in [
        NativeProc::new("print", print_proc),
        NativeProc::new("modules", modules_proc),
        NativeProc::new("module-info", module_info_proc),
        NativeProc::new("do", do_proc),
        NativeProc::new("list", list_proc),
        NativeProc::new("list?", listp_proc),
        NativeProc::new("first", first_proc),
        NativeProc::new("rest", rest_proc),
        NativeProc::new("nth", nth_proc),
        NativeProc::new("len", len_proc),
        NativeProc::new("substring", substring_proc),
        NativeProc::new("string-concat", string_concat_proc),
        NativeProc::new("+", add_proc),
        NativeProc::new("-", sub_proc),
        NativeProc::new("*", multiply_proc),
        NativeProc::new("/", divide_proc),
        NativeProc::new("<", less_proc),
        NativeProc::new("<=", less_eq_proc),
        NativeProc::new(">", greater_proc),
        NativeProc::new(">=", greater_eq_proc),
        NativeProc::new("equal?", equalp_proc),
    ]
    .into_iter()
    {
        module.set(proc.name().into(), Val::NativeProc(proc));
    }
    module
}

fn print_proc(_modules: &ModuleManager, args: &[Val]) -> Result<Val> {
    for arg in args.iter() {
        match arg {
            Val::String(s) => print!("{s}"),
            v => print!("{v}"),
        }
    }
    Ok(Val::Void)
}

fn modules_proc(modules: &ModuleManager, args: &[Val]) -> Result<Val> {
    ensure!(
        args.is_empty(),
        "modules expected 0 args but got {n}",
        n = args.len()
    );
    let module_names: Vec<_> = modules
        .iter()
        .map(|module| module.source().to_string())
        .map(Val::from)
        .collect();
    Ok(Val::List(Rc::new(module_names)))
}

fn module_info_proc(modules: &ModuleManager, args: &[Val]) -> Result<Val> {
    match args {
        [module] => {
            let module_str = module.try_str()?;
            for module in modules.iter() {
                if module.source().to_string() == module_str {
                    let info = module.to_string();
                    return Ok(Val::String(info.into()));
                }
            }
            bail!(
                "module {module_str:?} not found, available modules are {mods}",
                mods = modules_proc(modules, &[])?
            );
        }
        _ => bail!("expected (list-imports <module-str>)"),
    }
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

fn list_proc(_modules: &ModuleManager, args: &[Val]) -> Result<Val> {
    let ret = Val::List(Rc::from(args.to_vec()));
    Ok(ret)
}

fn listp_proc(_modules: &ModuleManager, args: &[Val]) -> Result<Val> {
    match args {
        [Val::List(_)] => Ok(Val::Bool(matches!(args[0], Val::List(_)))),
        _ => bail!("listp expected 1 arg but found {}", args.len()),
    }
}

fn first_proc(_modules: &ModuleManager, args: &[Val]) -> Result<Val> {
    match args {
        [arg] => match arg.try_slice()? {
            [] => Ok(Val::Void),
            [arg, ..] => Ok(arg.clone()),
        },
        _ => bail!(
            "<proc first> expected a single argument but found {}",
            args.len()
        ),
    }
}

fn rest_proc(_modules: &ModuleManager, args: &[Val]) -> Result<Val> {
    match args {
        [arg] => match arg.try_slice()? {
            [] => bail!("attempted to call procedure rest on empty list"),
            [_] => Ok(Val::List(Rc::new(Vec::new()))),
            [_, rest @ ..] => Ok(Val::List(Rc::new(Vec::from_iter(rest.iter().cloned())))),
        },
        _ => bail!(
            "<proc rest> expected a single argument but found {}",
            args.len()
        ),
    }
}

fn nth_proc(_modules: &ModuleManager, args: &[Val]) -> Result<Val> {
    match args {
        [lst, nth] => {
            let lst = lst.try_slice()?;
            let nth = nth.try_usize()?;
            lst.get(nth).cloned().ok_or_else(|| {
                anyhow!(
                    "<proc nth> failed to get element {nth} from list of length {len}",
                    len = lst.len()
                )
            })
        }
        _ => bail!(
            "<proc nth> expected an argument for list and nth but found {} arguments",
            args.len()
        ),
    }
}

fn len_proc(_modules: &ModuleManager, args: &[Val]) -> Result<Val> {
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

fn substring_proc(_modules: &ModuleManager, args: &[Val]) -> Result<Val> {
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

fn string_concat_proc(_modules: &ModuleManager, args: &[Val]) -> Result<Val> {
    let strs: Result<Vec<_>> = args.iter().map(|v| v.try_str()).collect();
    let res = strs?.join("");
    Ok(Val::String(Rc::new(res)))
}

fn do_proc(_modules: &ModuleManager, args: &[Val]) -> Result<Val> {
    Ok(args.last().cloned().unwrap_or(Val::Void))
}

/// Add all the values in `args`. If no values are present in `args`, then `0` is returned.
fn add_proc(_modules: &ModuleManager, args: &[Val]) -> Result<Val> {
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
fn sub_proc(_modules: &ModuleManager, args: &[Val]) -> Result<Val> {
    ensure_numbers("-", args)?;
    let res = match args {
        [] => bail!("- requires at least 1 arg"),
        [x] => negate(x),
        [x, ys @ ..] => {
            let sub_part = add_proc(_modules, ys)?;
            add_two(x, &negate(&sub_part))
        }
    };
    Ok(res)
}

/// Divide the first argument by the rest of the arguments. If only a single argument is provided,
/// then the reciprocal of it is returned.
fn divide_proc(_modules: &ModuleManager, args: &[Val]) -> Result<Val> {
    ensure_numbers("/", args)?;
    match args {
        [] => Err(anyhow!("/ requires at least 1 arg")),
        [x] => Ok(reciprocal(x)),
        [x, ys @ ..] => {
            let denom = multiply_proc(_modules, ys)?;
            Ok(multiply_two(x, &reciprocal(&denom)))
        }
    }
}

fn less_proc(_modules: &ModuleManager, args: &[Val]) -> Result<Val> {
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

fn less_eq_proc(_modules: &ModuleManager, args: &[Val]) -> Result<Val> {
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

fn greater_proc(_modules: &ModuleManager, args: &[Val]) -> Result<Val> {
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

fn greater_eq_proc(_modules: &ModuleManager, args: &[Val]) -> Result<Val> {
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
fn multiply_proc(_modules: &ModuleManager, args: &[Val]) -> Result<Val> {
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

fn equalp_proc(_modules: &ModuleManager, args: &[Val]) -> Result<Val> {
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
