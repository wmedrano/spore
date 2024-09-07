use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

mod repl;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut rl = DefaultEditor::new()?;
    let mut repl = repl::Repl::default();
    loop {
        let prompt = if repl.has_partial_input() {
            ".. "
        } else {
            ">> "
        };
        let readline = rl.readline(prompt);
        match readline {
            Ok(line) => repl.eval_expr(&line, &mut std::io::stdout())?,
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
    Ok(())
}
