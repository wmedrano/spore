use std::error::Error;

use spore_vm::{repl::Repl, Vm};

fn main() -> Result<(), Box<dyn Error>> {
    let mut repl = Repl::new(Vm::default())?;
    loop {
        if let Err(err) = repl.eval_next_input() {
            match err {
                rustyline::error::ReadlineError::Eof
                | rustyline::error::ReadlineError::Interrupted => return Ok(()),
                err => println!("{err}"),
            }
        }
    }
}
