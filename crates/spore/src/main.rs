use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use spore_lib::vm::{module::ModuleSource, Vm};
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
    let vm = Vm::new();
    match args.script {
        Some(filename) => {
            let mut repl = Repl::new(vm, ModuleSource::Virtual("%script%"))?;
            repl.eval_file(&mut std::io::stdout(), &filename)?;
            if repl
                .vm
                .modules()
                .get_value(&ModuleSource::File(filename.clone()), None, "main")
                .is_some()
            {
                repl.eval_input(
                    &mut std::io::stdout(),
                    &format!(
                        "({prefix}/main)",
                        prefix = filename.file_stem().unwrap().to_string_lossy()
                    ),
                )?;
            }
        }
        None => Repl::new(vm, ModuleSource::Virtual("%repl%"))?.run()?,
    }
    Ok(())
}
