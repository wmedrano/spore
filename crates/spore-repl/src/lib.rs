use std::io::Write;
use std::path::Path;
use std::rc::Rc;

use anyhow::{anyhow, bail, Context, Result};
use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use spore_lib::parser::ast::{Ast, ParseAstError};
use spore_lib::vm::debugger::TraceDebugger;
use spore_lib::vm::ir::{CodeBlock, CodeBlockArgs};
use spore_lib::vm::module::{Module, ModuleSource};
use spore_lib::vm::types::instruction::Instruction;
use spore_lib::vm::types::{
    proc::bytecode::{ByteCodeIter, ByteCodeProc},
    symbol::Symbol,
    Val,
};
use spore_lib::vm::Vm;

/// Contains functionality for pasing commands.
pub mod command;

/// Represents the Read-Eval-Print Loop (REPL) for the Spore language.
pub struct Repl {
    pub vm: Vm,
    editor: DefaultEditor,
    module: ModuleSource,
    expression_count: usize,
}

impl Repl {
    /// Creates a new Repl instance.
    pub fn new(vm: Vm, module: ModuleSource) -> Result<Repl> {
        let mut vm = vm;
        if !vm.modules_mut().has_module(&module) {
            vm.modules_mut().add_module(Module::new(module.clone()));
        }
        let editor = DefaultEditor::new()?;
        Ok(Repl {
            vm,
            editor,
            module,
            expression_count: 0,
        })
    }

    /// Runs teh REPL, accepting and evaluating user input.
    pub fn run(&mut self) -> Result<()> {
        let mut out = std::io::stdout();
        let mut repl_input = String::new();
        writeln!(
            out,
            "{welcome}\n  {repo_link}\n",
            welcome = "Welcome to Spore!".cyan(),
            repo_link = "https://github.com/wmedrano/spore".cyan()
        )?;
        print_help(&mut out)?;
        loop {
            let readline = self
                .editor
                .readline(if repl_input.is_empty() { ">> " } else { ".. " });
            match readline {
                Ok(line) => {
                    repl_input += line.as_str();
                    if line_is_complete(&repl_input) {
                        if let Err(err) = self.eval_input(&mut out, repl_input.as_str()) {
                            writeln!(out, "{error}\n{err}", error = "Error:".to_string().red())
                                .unwrap();
                        }
                        self.editor
                            .add_history_entry(std::mem::take(&mut repl_input))?;
                    }
                }
                Err(ReadlineError::Eof | ReadlineError::Interrupted) => {
                    writeln!(out).unwrap();
                    break;
                }
                Err(err) => {
                    writeln!(out, "{}\n{:?}", "Error".red(), err).unwrap();
                    break;
                }
            }
        }
        Ok(())
    }

    /// Run the given file in the REPL.
    pub fn eval_file(&mut self, out: &mut impl Write, filename: impl AsRef<Path>) -> Result<()> {
        let filename = filename.as_ref();
        self.eval_input(out, &format!("(import {filename:?})"))
            .with_context(|| anyhow!("Failed to run script from file {filename:?}."))
    }

    /// Evaluate the input.
    pub fn eval_input(&mut self, out: &mut impl Write, input: &str) -> Result<()> {
        let (cmd, expr) = command::parse_command(input);
        let asts = || match Ast::from_sexp_str(expr) {
            Ok(ast) => Ok(ast),
            Err(err) => {
                bail!("{}", err.display_with_context(expr));
            }
        };
        match cmd {
            "" => eval_asts(
                out,
                &self.module,
                asts()?,
                &mut self.vm,
                &mut self.expression_count,
                false,
            ),
            ",tokens" => {
                for token in spore_lib::parser::lexer::tokenize(expr) {
                    writeln!(out, "{token:?}")?;
                }
            }
            ",ast" => {
                for ast in asts()? {
                    writeln!(out, "{}", format!("{ast}").blue())?;
                }
            }
            ",ir" => {
                for (idx, ast) in asts()?.iter().enumerate() {
                    let codeblock = CodeBlock::with_ast(
                        CodeBlockArgs {
                            name: Some(format!("{idx}").into()),
                            ..CodeBlockArgs::default()
                        },
                        std::iter::once(ast),
                    )?;
                    writeln!(out, "{}", format!("{codeblock:#?}").blue())?;
                }
            }
            ",bytecode" => analyze_bytecode(out, &self.module, &mut self.vm, asts()?),
            ",trace" => eval_asts(
                out,
                &self.module,
                asts()?,
                &mut self.vm,
                &mut self.expression_count,
                true,
            ),
            ",help" => print_help(out)?,
            unknown => bail!(
                "Unknown command \"{unknown}\", expected one if {:?}.",
                [",tokens", ",ast", ",ir", ",bytecode", ",trace", ",help"]
            ),
        }
        Ok(())
    }
}

fn print_help(out: &mut impl Write) -> Result<()> {
    writeln!(out, "{}", "Commands".blue())?;
    let mut print_cmd = |cmd: &str, doc| writeln!(out, "  {cmd} - {doc}", cmd = cmd.blue());
    print_cmd(",tokens", "Parsed tokens for the expression(s).")?;
    print_cmd(",ast", "Ast for the expression(s).")?;
    print_cmd(",ir", "Intermediate representation for the expression(s).")?;
    print_cmd(",bytecode", "Bytecode for the expression(s)")?;
    print_cmd(
        ",trace",
        "Trace the input and output of all function calls.",
    )?;
    print_cmd(",help", "Show the help documentation.")?;
    writeln!(out)?;
    Ok(())
}

/// Returns `true` if the given input string is a complete expressions.
fn line_is_complete(s: &str) -> bool {
    !matches!(
        Ast::from_sexp_str(s),
        Err(ParseAstError::MissingClosingParen { .. })
    )
}

/// Evaluate `asts` under the given `vm`.
///
/// # Params
/// asts - The asts to run.
/// vm - The vm to evaluate under.
/// expr_count - The number of expressions that have been evaluated in the REPL. This is incremented for each
///   expression. This is also used to store variables (with names like $0, $1, $2, ...) under vm.
/// trace - If the trace output should be printed.
fn eval_asts(
    out: &mut impl Write,
    module: &ModuleSource,
    asts: Vec<Ast>,
    vm: &mut Vm,
    expr_count: &mut usize,
    trace: bool,
) {
    for ast in asts {
        let mut maybe_trace = if trace {
            Some(TraceDebugger::default())
        } else {
            None
        };
        let res = {
            let code_block_args = CodeBlockArgs {
                name: Some(format!("repl-proc-{n}", n = *expr_count + 1).into()),
                ..CodeBlockArgs::default()
            };
            let ast = &ast;
            match CodeBlock::with_ast(code_block_args, std::iter::once(ast)) {
                Ok(ir) => ir.to_proc(module.clone()),
                Err(err) => {
                    writeln!(out, "{}", err.to_string().red()).unwrap();
                    return;
                }
            }
        }
        .and_then(|bc| match maybe_trace.as_mut() {
            Some(t) => vm.eval_bytecode(bc.into(), &[], t),
            None => vm.eval_bytecode(bc.into(), &[], &mut ()),
        });
        if let Some(trace) = maybe_trace {
            writeln!(out, "{trace}").unwrap();
        }
        match res {
            Ok(Val::Void) => (),
            Ok(v) => {
                *expr_count += 1;
                let sym = Symbol::from(format!("${expr_count}").as_str());
                vm.modules_mut()
                    .set_value(module.clone(), sym.clone(), v.clone());
                writeln!(out, "{} = {}", sym.as_str().to_string().cyan(), v).unwrap();
            }
            Err(errs) => {
                for err in errs.chain() {
                    writeln!(out, "{}", err.to_string().red()).unwrap();
                }
            }
        }
    }
}

/// Analyze the bytecode for `asts`.
fn analyze_bytecode(out: &mut impl Write, module: &ModuleSource, vm: &mut Vm, asts: Vec<Ast>) {
    for ast in asts {
        let code_block_args = CodeBlockArgs {
            name: Some("repl-analyze-bytecode".into()),
            ..CodeBlockArgs::default()
        };
        let ast = &ast;
        let block_or_err = match CodeBlock::with_ast(code_block_args, std::iter::once(ast)) {
            Ok(ir) => ir.to_proc(module.clone()),
            Err(err) => {
                writeln!(out, "{}", err.to_string().red()).unwrap();
                return;
            }
        };
        let proc = match block_or_err {
            Ok(b) => b,
            Err(err) => {
                writeln!(out, "{}", err.to_string().red()).unwrap();
                continue;
            }
        };
        let bytecode = analyze_bytecode_iter(vm, proc);
        for (idx, bc) in bytecode.enumerate() {
            writeln!(out, "  {:02} - {bc}", format!("{:02}", idx + 1).blue()).unwrap();
        }
        writeln!(out).unwrap();
    }
}

/// Get the `ByteCodeIter` for `proc`. If `proc` is a simple expression that returns a procedure or a symbol that refers
/// to a procedure, then an iterator for that procedure is returned.
fn analyze_bytecode_iter(vm: &mut Vm, proc: ByteCodeProc) -> ByteCodeIter {
    let proc = Rc::new(proc);
    let mut iter = ByteCodeIter::from_proc(proc.clone());
    if iter.clone().count() == 1 {
        let instruction = iter.next().unwrap();
        match instruction {
            Instruction::GetVal(sym) => {
                let maybe_val = vm.modules().get_value(
                    &sym.module,
                    sym.sub_module.as_ref().map(|s| s.as_str()),
                    sym.symbol.as_str(),
                );
                if let Some(Val::ByteCodeProc(bc)) = maybe_val {
                    return ByteCodeIter::from_proc(bc.clone());
                }
            }
            Instruction::PushVal(Val::ByteCodeProc(bc)) => {
                return ByteCodeIter::from_proc(bc.clone());
            }
            _ => (),
        }
    }
    ByteCodeIter::from_proc(proc)
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, path::PathBuf};

    use spore_lib::vm::module::Module;

    use super::*;

    const REPL_MODULE: ModuleSource = ModuleSource::Virtual("%test-repl%");

    #[test]
    fn creating_repl_initializes_module() {
        let vm = Vm::new();
        assert_eq!(
            vm.modules()
                .iter()
                .map(|m| m.source().clone())
                .collect::<HashSet<_>>(),
            HashSet::from_iter([ModuleSource::Global]),
        );

        let repl = Repl::new(vm, REPL_MODULE).unwrap();
        assert_eq!(
            repl.vm
                .modules()
                .iter()
                .map(|m| m.source().clone())
                .collect::<HashSet<_>>(),
            HashSet::from_iter([ModuleSource::Global, REPL_MODULE]),
        );
    }

    #[test]
    fn eval_empty_string_produces_nothing() {
        let mut repl = Repl::new(Vm::new(), REPL_MODULE).unwrap();
        let mut out = Vec::new();
        repl.eval_input(&mut out, "").unwrap();
        assert_eq!(String::from_utf8(out).unwrap().as_str(), "");
    }

    #[test]
    fn eval_test_file_loads_as_module() {
        let mut repl = Repl::new(Vm::new(), REPL_MODULE).unwrap();
        let mut out = Vec::new();
        repl.eval_file(&mut out, "test_data/main.spore").unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "");

        let mut expected_module = Module::new(ModuleSource::File(PathBuf::from_iter([
            repl.vm.working_directory(),
            Path::new("test_data/main.spore"),
        ])));
        expected_module.set(Symbol::from("x"), Val::Int(10));
        expected_module.set(Symbol::from("y"), Val::Int(20));
        assert_eq!(
            repl.vm.modules().get(expected_module.source()),
            Some(&expected_module),
        )
    }

    #[test]
    fn eval_string_produces_numbered_outputs() {
        let mut repl = Repl::new(Vm::new(), REPL_MODULE).unwrap();
        let mut out = Vec::new();
        repl.eval_input(&mut out, "(+ 1 2) (* 3 4)").unwrap();
        assert_eq!(
            String::from_utf8(out).unwrap(),
            format!("{} = 3\n{} = 12\n", "$1".cyan(), "$2".cyan())
        );
    }

    #[test]
    fn eval_string_increases_counts() {
        let mut repl = Repl::new(Vm::new(), REPL_MODULE).unwrap();
        repl.eval_input(&mut Vec::new(), "1 2 3").unwrap();
        let mut out = Vec::new();
        repl.eval_input(&mut out, "(+ 1 2) (* 3 4)").unwrap();
        assert_eq!(
            String::from_utf8(out).unwrap(),
            format!("{} = 3\n{} = 12\n", "$4".cyan(), "$5".cyan())
        );
    }

    #[test]
    fn help_command_prints_help() {
        let mut help = Vec::new();
        print_help(&mut help).unwrap();
        assert!(!help.is_empty());

        let mut repl = Repl::new(Vm::new(), REPL_MODULE).unwrap();
        let mut out = Vec::new();
        repl.eval_input(&mut out, ",help").unwrap();
        assert_eq!(out, help);
    }
}
