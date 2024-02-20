use anyhow::Result;
use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

use crate::parser::ast::{Ast, ParseAstError};
use crate::vm::compiler::Compiler;
use crate::vm::types::proc::ByteCodeIter;
use crate::vm::types::symbol::Symbol;
use crate::vm::types::Val;
use crate::vm::Vm;

/// Run the REPL.
pub fn run_repl() -> Result<()> {
    let mut rl = DefaultEditor::new()?;
    println!("{}", "Welcome to Spore!".cyan());
    println!("{}", "  https://github.com/wmedrano/spore".cyan());
    println!();
    let mut expr_count = 0;
    let mut sexpr = String::new();
    loop {
        let readline = rl.readline(if sexpr.is_empty() { ">> " } else { ".. " });
        match readline {
            Ok(line) => {
                sexpr += line.as_str();
                if line_is_complete(&sexpr) {
                    rl.add_history_entry(sexpr.as_str())?;
                    eval_str(sexpr.as_str(), &mut expr_count);
                    sexpr.clear();
                }
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

fn line_is_complete(s: &str) -> bool {
    let s = s.strip_prefix(",\\w+").unwrap_or(s);
    !matches!(
        Ast::from_sexp_str(s),
        Err(ParseAstError::MissingClosingParen { .. })
    )
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
    let bytecode = Compiler::new().compile_and_finalize(ast)?;
    Vm::singleton().env().eval_bytecode(bytecode.into())
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
        let bytecode = match Compiler::new().compile_and_finalize(&ast) {
            Ok(b) => b,
            Err(err) => {
                println!("{}", err.to_string().red());
                continue;
            }
        };
        for (idx, bc) in ByteCodeIter::from_proc(bytecode.into()).enumerate() {
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
