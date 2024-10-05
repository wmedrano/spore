use rustyline::DefaultEditor;

use crate::{parser::ast::Node, val::ProtectedVal, Vm};

/// Run an read/evaluate/print/loop.
///
/// ```rust
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let mut repl = spore_vm::repl::Repl::new(spore_vm::Vm::default())?;
///     loop {
///         if let Err(err) = repl.eval_next_input() {
///             match err {
///                 rustyline::error::ReadlineError::Eof
///                 | rustyline::error::ReadlineError::Interrupted => return Ok(()),
///                 err => println!("{err}"),
///             }
///         }
///     }
/// }
/// ```
pub struct Repl {
    vm: Vm,
    editor: DefaultEditor,
}

impl Repl {
    /// Create a new REPL.
    pub fn new(vm: Vm) -> rustyline::Result<Repl> {
        let editor = DefaultEditor::new()?;
        Ok(Repl { vm, editor })
    }

    /// Convert `self` into a [Vm].
    pub fn into_vm(self) -> Vm {
        self.vm
    }

    /// Get the underlying [Vm].
    pub fn as_vm_mut(&mut self) -> &mut Vm {
        &mut self.vm
    }

    /// Get the underlying [Vm].
    pub fn as_vm(&self) -> &Vm {
        &self.vm
    }

    /// Evaluate the next user input. The input is read through `stdin` and the result is written
    /// through `stdout`.
    pub fn eval_next_input(&mut self) -> rustyline::Result<ProtectedVal<'_>> {
        let mut input = String::new();
        let mut is_ready = false;
        while !is_ready {
            let prompt = ">> ";
            match self.editor.readline(prompt) {
                Ok(line) => {
                    input.push_str(line.as_str());
                    for node_or_err in Node::parse(&input) {
                        match node_or_err {
                            Ok(_) => is_ready = true,
                            Err(crate::parser::ast::AstParseError::UnclosedParen) => {
                                is_ready = false;
                                break;
                            }
                            Err(err) => {
                                return rustyline::Result::Err(rustyline::error::ReadlineError::Io(
                                    std::io::Error::new(std::io::ErrorKind::InvalidInput, err),
                                ))
                            }
                        }
                    }
                }
                Err(err) => return Err(err),
            };
        }
        let res = self
            .vm
            .eval_str(&input)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
        println!("{res}");
        Ok(res)
    }
}
