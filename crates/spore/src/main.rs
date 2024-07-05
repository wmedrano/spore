use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use spore_lib::{
    parser::ast::Ast,
    vm::{
        ir::{CodeBlock, CodeBlockArgs},
        module::ModuleSource,
        Vm,
    },
};
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
        Some(filename) => run_script(vm, filename)?,
        None => run_interactive(vm)?,
    }
    Ok(())
}

fn run_script(vm: Vm, filename: PathBuf) -> Result<()> {
    let mut repl = Repl::new(vm, ModuleSource::Virtual("%script%"))?;
    repl.eval_file(&mut std::io::stdout(), &filename)?;
    let has_main = repl
        .vm
        .modules()
        .get_value(&ModuleSource::File(filename.clone()), None, "main")
        .is_some();
    if has_main {
        repl.eval_input(
            &mut std::io::stdout(),
            &format!(
                "({prefix}/main)",
                prefix = filename.file_stem().unwrap().to_string_lossy()
            ),
        )?;
    }
    Ok(())
}

fn run_interactive(vm: Vm) -> Result<()> {
    let module = ModuleSource::Virtual("%repl%");
    let mut repl = Repl::new(vm, module.clone())?;
    run_init(&mut repl.vm, module)?;
    repl.run()?;
    Ok(())
}

fn run_init(vm: &mut Vm, module: ModuleSource) -> Result<()> {
    let path = expanduser::expanduser("~/.spore/init.spore")?;
    std::fs::read_to_string(&path).unwrap();
    if let Ok(init_src) = std::fs::read_to_string(path) {
        let asts = Ast::from_sexp_str(&init_src)?;
        let code_block_args = CodeBlockArgs {
            name: Some(format!("user-init-proc").into()),
            ..CodeBlockArgs::default()
        };
        let proc = CodeBlock::with_ast(code_block_args, asts.iter())?.to_proc(module)?;
        vm.eval_bytecode(proc.into(), &[], &mut ())?;
    }
    Ok(())
}
