use std::io::Write;
use std::rc::Rc;

use anyhow::{bail, Result};
use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use spore_lib::parser::ast::{Ast, ParseAstError};
use spore_lib::vm::debugger::TraceDebugger;
use spore_lib::vm::environment::Environment;
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
    env: Environment,
    editor: DefaultEditor,
    module: ModuleSource,
    expression_count: usize,
}

impl Repl {
    /// Creates a new Repl instance.
    pub fn new() -> Result<Repl> {
        let editor = DefaultEditor::new()?;
        let module = ModuleSource::Virtual("repl");
        let mut env = Vm::new().build_env();
        env.modules_mut().add_module(Module::new(module.clone()));
        Ok(Repl {
            env,
            editor,
            module,
            expression_count: 0,
        })
    }

    /// Runs teh REPL, accepting and evaluating user input.
    pub fn run(&mut self, out: &mut impl Write) -> Result<()> {
        let mut repl_input = String::new();
        writeln!(
            out,
            "{welcome}\n  {repo_link}\n",
            welcome = "Welcome to Spore!".cyan(),
            repo_link = "https://github.com/wmedrano/spore".cyan()
        )?;
        loop {
            let readline = self
                .editor
                .readline(if repl_input.is_empty() { ">> " } else { ".. " });
            match readline {
                Ok(line) => {
                    repl_input += line.as_str();
                    if line_is_complete(&repl_input) {
                        if let Err(err) = self.eval_input(out, std::mem::take(&mut repl_input)) {
                            writeln!(out, "{error}\n{err}", error = "Error:".to_string().red())
                                .unwrap();
                        }
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

    /// Evaluate the input.
    pub fn eval_input(&mut self, out: &mut impl Write, input: String) -> Result<()> {
        self.editor.add_history_entry(input.as_str())?;
        let (cmd, expr) = command::parse_command(&input);
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
                &mut self.env,
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
                            name: Some(format!("{idx}")),
                            ..CodeBlockArgs::default()
                        },
                        std::iter::once(ast),
                    )?;
                    writeln!(out, "{}", format!("{codeblock:#?}").blue())?;
                }
            }
            ",bytecode" => analyze_bytecode(out, &self.module, &mut self.env, asts()?),
            ",trace" => eval_asts(
                out,
                &self.module,
                asts()?,
                &mut self.env,
                &mut self.expression_count,
                true,
            ),
            ",help" => {
                let mut print_cmd =
                    |cmd: &str, doc| writeln!(out, "{cmd} - {doc}", cmd = cmd.blue());
                print_cmd(",tokens", "Print the parsed tokens for the expression(s).")?;
                print_cmd(",ast", "Print the ast for the expression(s).")?;
                print_cmd(
                    ",ir",
                    "Print the intermediate representation for the expression(s).",
                )?;
                print_cmd(",bytecode", "Print the bytecode for the expression(s)")?;
                print_cmd(
                    ",trace",
                    "Print the input and output of all function calls.",
                )?;
                print_cmd(",help", "Print the help documentation.")?;
            }
            unknown => bail!(
                "unknown command \"{unknown}\", expected one if {:?}",
                [",tokens", ",ast", ",ir", ",bytecode", ",trace", ",help"]
            ),
        }
        Ok(())
    }
}

/// Returns `true` if the given input string is a complete expressions.
fn line_is_complete(s: &str) -> bool {
    !matches!(
        Ast::from_sexp_str(s),
        Err(ParseAstError::MissingClosingParen { .. })
    )
}

/// Evaluate `asts` under the given `env`.
///
/// # Params
/// asts - The asts to run.
/// env - The environment to evaluate under.
/// expr_count - The number of expressions that have been evaluated in the REPL. This is incremented for each
///   expression. This is also used to store variables (with names like $0, $1, $2, ...) under env.
/// trace - If the trace output should be printed.
fn eval_asts(
    out: &mut impl Write,
    module: &ModuleSource,
    asts: Vec<Ast>,
    env: &mut Environment,
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
                name: Some(format!("repl-proc-{n}", n = *expr_count + 1)),
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
            Some(t) => env.eval_bytecode(bc.into(), &[], t),
            None => env.eval_bytecode(bc.into(), &[], &mut ()),
        });
        if let Some(trace) = maybe_trace {
            writeln!(out, "{trace}").unwrap();
        }
        match res {
            Ok(Val::Void) => (),
            Ok(v) => {
                *expr_count += 1;
                let sym = Symbol::from(format!("${expr_count}"));
                env.modules_mut().set_value(module, sym.clone(), v.clone());
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
fn analyze_bytecode(
    out: &mut impl Write,
    module: &ModuleSource,
    env: &mut Environment,
    asts: Vec<Ast>,
) {
    for ast in asts {
        let code_block_args = CodeBlockArgs {
            name: Some("repl-analyze-bytecode".to_string()),
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
        let bytecode = analyze_bytecode_iter(env, proc);
        for (idx, bc) in bytecode.enumerate() {
            writeln!(out, "  {:02} - {bc}", format!("{:02}", idx + 1).blue()).unwrap();
        }
        writeln!(out).unwrap();
    }
}

/// Get the `ByteCodeIter` for `proc`. If `proc` is a simple expression that returns a procedure or a symbol that refers
/// to a procedure, then an iterator for that procedure is returned.
fn analyze_bytecode_iter(env: &mut Environment, proc: ByteCodeProc) -> ByteCodeIter {
    let proc = Rc::new(proc);
    let mut iter = ByteCodeIter::from_proc(proc.clone());
    if iter.clone().count() == 1 {
        let instruction = iter.next().unwrap();
        match instruction {
            Instruction::GetVal(sym) => {
                let maybe_val =
                    env.modules()
                        .get_value(&sym.module, sym.sub_module, sym.symbol.as_str());
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
