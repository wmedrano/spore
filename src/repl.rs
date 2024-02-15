use anyhow::{anyhow, Result};
use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

use crate::parser::ast::Ast;
use crate::vm::bytecode::ByteCode;
use crate::vm::types::Val;
use crate::vm::vm::Vm;

/// Run the REPL.
pub fn run_repl() -> Result<()> {
    let mut rl = DefaultEditor::new()?;
    println!("{}", "Welcome to Spore!".cyan());
    println!("{}", "  https://github.com/wmedrano/spore".cyan());
    println!();
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str())?;
                eval_str(line.as_str());
            }
            Err(ReadlineError::Eof | ReadlineError::Interrupted) => {
                println!();
                break;
            }
            Err(err) => {
                println!("{}: {:?}", "Error".red(), err);
                break;
            }
        }
    }
    Ok(())
}

macro_rules! prefix_to_command {
    ( $s:expr, $($prefix:expr => $fn:expr),*) => {
        if false {}
        $(
            else if $s.starts_with($prefix) {
                $fn(&$s[$prefix.len()..]);
            }
        )*
    };
}

fn eval_str(s: &str) {
    prefix_to_command!(s,
                       ",ast" => analyze_ast,
                       ",bytecode " => analyze_bytecode,
                       "" => eval_sexpr);
}

fn eval_sexpr(s: &str) {
    let asts = match Ast::from_sexp_str(s) {
        Ok(ast) => ast,
        Err(err) => {
            println!("{}", err.to_string().red());
            return;
        }
    };
    let vm = Vm::singleton();
    for ast in asts {
        match eval_ast(vm, &ast) {
            Ok(v) => println!("{}", v.to_string().cyan()),
            Err(err) => println!("{}", err.to_string().red()),
        }
    }
}

fn eval_ast(vm: &Vm, ast: &Ast) -> Result<Val> {
    let bytecode = ByteCode::with_ast(ast)?;
    let res = vm.eval_bytecode(&bytecode, Vec::with_capacity(4096))?;
    res.into_iter()
        .next()
        .ok_or_else(|| anyhow!("bytecode interpreter failed to return any values"))
}

fn analyze_bytecode(s: &str) {
    let asts = match Ast::from_sexp_str(s) {
        Ok(ast) => ast,
        Err(err) => {
            println!("{}", err.to_string().red());
            return;
        }
    };
    for ast in asts {
        let bytecode = match ByteCode::with_ast(&ast) {
            Ok(b) => b,
            Err(err) => {
                println!("{}", err.to_string().red());
                continue;
            }
        };
        for (idx, bc) in bytecode.into_iter().enumerate() {
            println!("  {:02} - {bc}", format!("{:02}", idx + 1).blue(),);
        }
        println!();
    }
}

fn analyze_ast(s: &str) {
    let asts = match Ast::from_sexp_str(s) {
        Ok(ast) => ast,
        Err(err) => {
            println!("{}", err.to_string().red());
            return;
        }
    };
    for ast in asts {
        println!("{}", format!("{ast:#?}").blue());
    }
}
