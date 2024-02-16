use anyhow::{anyhow, Result};
use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

use crate::parser::ast::Ast;
use crate::vm::bytecode::ByteCode;
use crate::vm::types::{Symbol, Val};
use crate::vm::Vm;

/// Run the REPL.
pub fn run_repl() -> Result<()> {
    let mut rl = DefaultEditor::new()?;
    println!("{}", "Welcome to Spore!".cyan());
    println!("{}", "  https://github.com/wmedrano/spore".cyan());
    println!();
    let mut expr_count = 0;
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str())?;
                eval_str(line.as_str(), &mut expr_count);
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

fn eval_str(s: &str, expr_count: &mut usize) {
    if let Some(s) = s.strip_prefix(",ast") {
        analyze_ast(s);
    } else if let Some(s) = s.strip_prefix(",bytecode") {
        analyze_bytecode(s);
    } else {
        eval_sexpr(s, expr_count);
    }
}

fn eval_sexpr(s: &str, expr_count: &mut usize) {
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
            Ok(v) => {
                *expr_count += 1;
                let sym = Symbol::from(format!("${expr_count}"));
                let _ = Vm::singleton().register_value(sym.clone(), v.clone());
                println!("{} = {}", sym.as_str().to_string().cyan(), v);
            }
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
