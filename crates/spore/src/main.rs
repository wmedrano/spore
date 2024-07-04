use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use spore_lib::vm::Vm;
use spore_repl::Repl;

/// The Spore programming language.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Run a file as a script. This essentially runs the file as a Spore program in the REPL and
    /// exits on completion.
    #[arg(long)]
    script: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut repl = Repl::new(Vm::new()).unwrap();
    match args.script {
        Some(filename) => repl.eval_file(&mut std::io::stdout(), &filename),
        None => repl.run(),
    }
}
