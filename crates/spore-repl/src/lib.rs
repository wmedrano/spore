use std::rc::Rc;

use anyhow::{bail, Result};
use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use spore_lib::parser::ast::{Ast, ParseAstError};
use spore_lib::vm::debugger::TraceDebugger;
use spore_lib::vm::environment::Environment;
use spore_lib::vm::ir::{CodeBlock, CodeBlockArgs};
use spore_lib::vm::types::instruction::Instruction;
use spore_lib::vm::types::{
    proc::bytecode::{ByteCodeIter, ByteCodeProc},
    symbol::Symbol,
    Val,
};
use spore_lib::vm::Vm;

pub mod command;

pub struct Repl {
    env: Environment,
    editor: DefaultEditor,
    repl_input: String,
    expression_count: usize,
}

impl Repl {
    /// Create a new repl.
    pub fn new() -> Result<Repl> {
        let editor = DefaultEditor::new()?;
        Ok(Repl {
            env: Vm::new().build_env(),
            editor,
            repl_input: String::new(),
            expression_count: 0,
        })
    }

    /// Run the REPL.
    pub fn run(&mut self) -> Result<()> {
        println!(
            "{welcome}\n  {repo_link}\n",
            welcome = "Welcome to Spore!".cyan(),
            repo_link = "https://github.com/wmedrano/spore".cyan()
        );
        loop {
            let readline = self.editor.readline(if self.repl_input.is_empty() {
                ">> "
            } else {
                ".. "
            });
            match readline {
                Ok(line) => {
                    self.repl_input += line.as_str();
                    if line_is_complete(&self.repl_input) {
                        self.editor.add_history_entry(self.repl_input.as_str())?;
                        if let Err(err) = self.eval_input() {
                            println!("{error}\n{err}", error = "Error:".to_string().red());
                        }
                    }
                }
                Err(ReadlineError::Eof | ReadlineError::Interrupted) => {
                    println!();
                    break;
                }
                Err(err) => {
                    println!("{}\n{:?}", "Error".red(), err);
                    break;
                }
            }
        }
        Ok(())
    }

    /// Evaluate the current input.
    fn eval_input(&mut self) -> Result<()> {
        let input = std::mem::take(&mut self.repl_input);
        let (cmd, expr) = command::parse_command(input.as_str());
        let asts = || match Ast::from_sexp_str(expr) {
            Ok(ast) => Ok(ast),
            Err(err) => {
                bail!("{}", err.display_with_context(expr));
            }
        };
        match cmd {
            "" => eval_asts(asts()?, &mut self.env, &mut self.expression_count, false),
            ",tokens" => {
                for token in spore_lib::parser::lexer::tokenize(expr) {
                    println!("{token:?}");
                }
            }
            ",ast" => {
                for ast in asts()? {
                    println!("{}", format!("{ast}").blue());
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
                    println!("{}", format!("{codeblock:#?}").blue());
                }
            }
            ",bytecode" => analyze_bytecode(&mut self.env, asts()?),
            ",trace" => eval_asts(asts()?, &mut self.env, &mut self.expression_count, true),
            ",help" => {
                let print_cmd = |cmd: &str, doc| {
                    println!("{cmd} - {doc}", cmd = cmd.blue());
                };
                print_cmd(",tokens", "Print the parsed tokens for the expression(s).");
                print_cmd(",ast", "Print the ast for the expression(s).");
                print_cmd(
                    ",ir",
                    "Print the intermediate representation for the expression(s).",
                );
                print_cmd(",bytecode", "Print the bytecode for the expression(s)");
                print_cmd(
                    ",trace",
                    "Print the input and output of all function calls.",
                );
                print_cmd(",help", "Print the help documentation.");
            }
            unknown => bail!(
                "unknown command \"{unknown}\", expected one if {:?}",
                [",tokens", ",ast", ",ir", ",bytecode", ",trace", ",help"]
            ),
        }
        Ok(())
    }
}

fn line_is_complete(s: &str) -> bool {
    !matches!(
        Ast::from_sexp_str(s),
        Err(ParseAstError::MissingClosingParen { .. })
    )
}

fn eval_asts(asts: Vec<Ast>, env: &mut Environment, expr_count: &mut usize, trace: bool) {
    for ast in asts {
        let mut maybe_trace = if trace {
            Some(TraceDebugger::default())
        } else {
            None
        };
        let res = {
            let code_block_args = CodeBlockArgs {
                name: Some("repl-eval".to_string()),
                ..CodeBlockArgs::default()
            };
            let ast = &ast;
            match CodeBlock::with_ast(code_block_args, std::iter::once(ast)) {
                Ok(ir) => ir.to_bytecode(),
                Err(err) => {
                    println!("{}", err.to_string().red());
                    return;
                }
            }
        }
        .and_then(|bc| match maybe_trace.as_mut() {
            Some(t) => env.eval_bytecode(bc.into(), &[], t),
            None => env.eval_bytecode(bc.into(), &[], &mut ()),
        });
        if let Some(trace) = maybe_trace {
            println!("{trace}");
        }
        match res {
            Ok(Val::Void) => (),
            Ok(v) => {
                *expr_count += 1;
                let sym = Symbol::from(format!("${expr_count}"));
                env.modules_mut().set_local(sym.clone(), v.clone());
                println!("{} = {}", sym.as_str().to_string().cyan(), v);
            }
            Err(errs) => {
                for err in errs.chain() {
                    println!("{}", err.to_string().red());
                }
            }
        }
    }
}

fn analyze_bytecode(env: &mut Environment, asts: Vec<Ast>) {
    for ast in asts {
        let proc = match {
            let code_block_args = CodeBlockArgs {
                name: Some("repl-analyze-bytecode".to_string()),
                ..CodeBlockArgs::default()
            };
            let ast = &ast;
            match CodeBlock::with_ast(code_block_args, std::iter::once(ast)) {
                Ok(ir) => ir.to_bytecode(),
                Err(err) => {
                    println!("{}", err.to_string().red());
                    return;
                }
            }
        } {
            Ok(b) => b,
            Err(err) => {
                println!("{}", err.to_string().red());
                continue;
            }
        };
        let bytecode = maybe_expand_bytecode(env, proc);
        for (idx, bc) in bytecode.enumerate() {
            println!("  {:02} - {bc}", format!("{:02}", idx + 1).blue(),);
        }
        println!();
    }
}

fn maybe_expand_bytecode(env: &mut Environment, proc: ByteCodeProc) -> ByteCodeIter {
    let proc = Rc::new(proc);
    let mut iter = ByteCodeIter::from_proc(proc.clone());
    if iter.clone().count() == 1 {
        let instruction = iter.next().unwrap();
        match instruction {
            Instruction::GetVal(sym) => {
                if let Some(Val::ByteCodeProc(bc)) = env.modules().get(&sym) {
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
