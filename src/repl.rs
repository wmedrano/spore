use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::{DefaultEditor, Result};

use crate::vm::Vm;

/// Run the REPL.
pub fn run_repl() -> Result<()> {
    let mut rl = DefaultEditor::new()?;
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str()).unwrap();
                for result in Vm::eval_str(line.as_str()) {
                    println!("{}", result.display_string().bright_green());
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("{}", "Ctrl-C".green());
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("{}", "Ctrl-D".green());
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
