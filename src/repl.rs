use anyhow::Result;
use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

use crate::parser::ast::Ast;
use crate::vm::expr::Expr;

/// Run the REPL.
pub fn run_repl() -> Result<()> {
    let mut rl = DefaultEditor::new()?;
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

fn eval_str(s: &str) {
    let asts = match Ast::from_sexp_str(s) {
        Ok(ast) => ast,
        Err(err) => {
            println!("{}", format!("{err:?}").red());
            return;
        }
    };
    for ast in asts {
        let expr = Expr::from_ast(&ast);
        match expr.eval() {
            Ok(res) => println!("{}", res.to_string().blue()),
            Err(err) => println!("{}", err.to_string().red()),
        }
    }
}
