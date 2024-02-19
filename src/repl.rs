use anyhow::Result;
use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

use crate::parser::ast::Ast;
use crate::vm::compiler::Compiler;
use crate::vm::types::{GenericProcedure, Symbol, Val};
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
    if s.starts_with(',') {
        if let Some(s) = s.strip_prefix(",ast ") {
            analyze_ast(s);
        } else if let Some(s) = s.strip_prefix(",bytecode ") {
            analyze_bytecode(s);
        } else {
            println!(
                "{}: Command {} not recognized, valid commands are {}",
                "Error".to_string().red(),
                s.split_whitespace().next().unwrap().to_string().yellow(),
                format!("{:?}", [",ast ", ",bytecode ", ",sexp "]).cyan(),
            )
        }
    } else {
        eval_sexpr(s, expr_count);
    }
}

fn eval_sexpr(s: &str, expr_count: &mut usize) {
    let asts = match Ast::from_sexp_str(s) {
        Ok(ast) => ast,
        Err(err) => {
            println!("{}", err.display_with_context(s).to_string().red());
            return;
        }
    };
    for ast in asts {
        match eval_ast(&ast) {
            Ok(Val::Void) => (),
            Ok(v) => {
                *expr_count += 1;
                let sym = Symbol::from(format!("${expr_count}"));
                let _ = Vm::singleton().register_global_value(sym.clone(), v.clone());
                println!("{} = {}", sym.as_str().to_string().cyan(), v);
            }
            Err(err) => println!("{}", err.to_string().red()),
        }
    }
}

fn eval_ast(ast: &Ast) -> Result<Val> {
    let bytecode = Compiler::new().compile_and_finalize("".to_string(), ast)?;
    let mut env = Vm::singleton().env();
    bytecode.eval(&mut env)
}

fn analyze_bytecode(s: &str) {
    let asts = match Ast::from_sexp_str(s) {
        Ok(ast) => ast,
        Err(err) => {
            println!("{}", err.display_with_context(s).to_string().red());
            return;
        }
    };
    for ast in asts {
        let bytecode = match Compiler::new().compile_and_finalize("".to_string(), &ast) {
            Ok(b) => b,
            Err(err) => {
                println!("{}", err.to_string().red());
                continue;
            }
        };
        for (idx, bc) in bytecode.instructions().iter().enumerate() {
            println!("  {:02} - {bc}", format!("{:02}", idx + 1).blue(),);
        }
        println!();
    }
}

fn analyze_ast(s: &str) {
    let asts = match Ast::from_sexp_str(s) {
        Ok(ast) => ast,
        Err(err) => {
            println!("{}", err.display_with_context(s).to_string().red());
            return;
        }
    };
    for ast in asts {
        println!("{}", format!("{ast:#?}").blue());
    }
}
