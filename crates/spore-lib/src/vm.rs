use std::{
    path::{Path, PathBuf},
    rc::Rc,
};

use anyhow::{anyhow, bail, ensure, Context, Result};
use smol_str::SmolStr;

use crate::parser::ast::Ast;

use debugger::Debugger;
use ir::{CodeBlock, CodeBlockArgs};
use module::{Module, ModuleManager, ModuleSource};
use types::{
    instruction::Instruction,
    proc::bytecode::{ByteCodeIter, ByteCodeProc},
    symbol::Symbol,
    Val,
};

pub mod debugger;
pub mod ir;
pub mod module;
pub mod types;

/// An environment to evaluate bytecode on.
pub struct Vm {
    /// All the modules.
    modules: ModuleManager,
    /// The processing stack.
    stack: Vec<Val>,
    /// Contains the current call frame. This includes the instructions that should be run and the
    /// base of the frame's stack.
    frames: Vec<Frame>,
    /// The directory to pull libraries from.
    root: PathBuf,
}

pub struct Frame {
    /// The bytecode that is under execution in the frame.
    bytecode: ByteCodeIter,
    /// The index of the local stack.
    stack_start_idx: usize,
}

/// Contains information about a stack frame.
struct StackFrameInfo {
    /// The name of the procedure.
    proc_name: SmolStr,
    /// The module for the procedure.
    module: ModuleSource,
}

struct StackTrace {
    trace: Vec<StackFrameInfo>,
}

impl Default for Vm {
    fn default() -> Vm {
        Vm::new()
    }
}

impl Vm {
    /// Create a new vm.
    pub fn new() -> Vm {
        Vm {
            modules: ModuleManager::new(crate::builtins::global_module()),
            stack: Vec::with_capacity(4096),
            frames: Vec::with_capacity(128),
            root: std::env::current_dir().unwrap(),
        }
    }

    /// Get the working directory for the VM.
    pub fn working_directory(&self) -> &Path {
        &self.root
    }

    /// Evaluate an S-Expression string and return the last value. If there are no expression, than
    /// `Val::Void` is returned.
    pub fn eval_str(&mut self, module: ModuleSource, s: &str) -> Result<Vec<Val>> {
        Ast::from_sexp_str(s)?
            .into_iter()
            .map(|ast| {
                let code_block_args = CodeBlockArgs {
                    name: Some(SmolStr::new_static("eval-str")),
                    ..CodeBlockArgs::default()
                };
                let ir = CodeBlock::with_ast(code_block_args, std::iter::once(&ast))?;
                let proc = ir.to_proc(module.clone())?;
                self.eval_bytecode(proc.into(), &[], &mut ())
            })
            .collect()
    }

    /// Evaluate a bytecode procedure with the given arguments and the given debugger.
    pub fn eval_bytecode(
        &mut self,
        proc: Rc<ByteCodeProc>,
        args: &[Val],
        debugger: &mut impl Debugger,
    ) -> Result<Val> {
        self.prepare(proc.clone(), args)?;
        debugger.eval_proc(self);
        self.eval_bytecode_impl(debugger)
            .inspect_err(|_| {
                self.clean_failed_modules();
            })
            .with_context(|| self.stack_trace())
    }

    /// Get all the modules.
    pub fn modules(&self) -> &ModuleManager {
        &self.modules
    }

    /// Get all the modules.
    pub fn modules_mut(&mut self) -> &mut ModuleManager {
        &mut self.modules
    }

    /// The values on the current stack frame.
    pub fn frame_stack(&self) -> &[Val] {
        match self.frames.last() {
            None => &[],
            Some(f) => &self.stack[f.stack_start_idx..],
        }
    }

    /// The current depth of the call stack.
    pub fn frame_depth(&self) -> usize {
        self.frames.len()
    }

    /// Get the current procedure.
    pub fn current_proc(&self) -> Option<&Rc<ByteCodeProc>> {
        self.frames.last().map(|f| f.bytecode.inner())
    }

    fn prepare(&mut self, proc: Rc<ByteCodeProc>, args: &[Val]) -> Result<()> {
        ensure!(
            proc.arg_count == args.len(),
            "Wrong number of args to {proc}. Expected {expected} but got {got}.",
            expected = proc.arg_count,
            got = args.len(),
        );
        if !self.modules.has_module(&proc.module) {
            self.modules.add_module(Module::new(proc.module.clone()));
        }
        self.frames.clear();
        self.stack.clear();
        self.stack.extend_from_slice(args);
        self.frames.push(Frame {
            bytecode: ByteCodeIter::from_proc(proc),
            stack_start_idx: 0,
        });
        Ok(())
    }

    fn clean_failed_modules(&mut self) {
        for frame in self.frames.iter_mut() {
            let bytecode = frame.bytecode.inner();
            if bytecode.is_module_definition {
                self.modules.remove_module(&bytecode.module);
            }
        }
    }

    /// Evaluate a sequence of bytecode.
    fn eval_bytecode_impl(&mut self, debugger: &mut impl Debugger) -> Result<Val> {
        while let Some(frame) = self.frames.last_mut() {
            let instruction = frame.bytecode.next_instruction();
            match instruction {
                Instruction::PushVal(v) => {
                    let v = v.clone();
                    self.stack.push(v);
                }
                Instruction::Eval(n) => {
                    let n = *n;
                    self.execute_eval_n(n, debugger)?;
                }
                Instruction::GetArg(n) => {
                    let n = *n;
                    self.execute_get_arg(n)
                }
                Instruction::GetVal(s) => {
                    let maybe_value = self.modules.get_value(
                        &s.module,
                        s.sub_module.as_deref(),
                        s.symbol.as_str(),
                    );
                    match maybe_value {
                        Some(v) => self.stack.push(v),
                        None => bail!("Value for {s} is not defined."),
                    }
                }
                Instruction::JumpIf(n) => {
                    let n = *n;
                    self.execute_jump_if(n)?
                }
                Instruction::Jump(n) => {
                    let n = *n;
                    frame.bytecode.jump(n);
                }
                Instruction::SetVal(s) => {
                    let s: Symbol = s.as_ref().clone();
                    let module = frame.bytecode.inner().module.clone();
                    self.execute_set_val(module, s, debugger)?;
                }
                Instruction::ImportModule(filepath) => {
                    let filepath = filepath.as_ref().clone();
                    self.import_module(filepath, debugger)?;
                }
                Instruction::Return => {
                    self.pop_frame(debugger)?;
                }
            }
        }
        let ret = self.stack.pop().unwrap_or_default();
        debugger.return_value(&ret);
        Ok(ret)
    }

    #[cold]
    fn stack_trace(&self) -> StackTrace {
        StackTrace {
            trace: self
                .frames
                .iter()
                .map(|f| StackFrameInfo {
                    proc_name: f.bytecode.inner().name.clone(),
                    module: f.bytecode.inner().module.clone(),
                })
                .collect(),
        }
    }

    /// Pop the current frame. This truncates the local stack and replaces the top value of the
    /// stack with the return value. The return value is defined as the value at the top of the
    /// local stack.
    fn pop_frame(&mut self, debugger: &mut impl Debugger) -> Result<()> {
        let frame = self.frames.pop().unwrap();
        let has_return_value = self.stack.len() > frame.stack_start_idx
            && !frame.bytecode.inner().is_module_definition;
        let return_val = if has_return_value {
            self.stack.pop().unwrap_or_default()
        } else {
            Val::Void
        };
        debugger.return_value(&return_val);
        self.stack.truncate(frame.stack_start_idx);
        self.stack.pop();
        self.stack.push(return_val);
        Ok(())
    }

    fn execute_get_arg(&mut self, n: usize) {
        let start_idx = self.frames.last().unwrap().stack_start_idx;
        let idx = start_idx + n;
        let v = self.stack.get(idx).unwrap().clone();
        self.stack.push(v);
    }

    fn execute_jump_if(&mut self, n: usize) -> Result<()> {
        let v = self.stack.pop().unwrap_or_default();
        if v.is_truthy()? {
            self.frames.last_mut().unwrap().bytecode.jump(n);
        }
        Ok(())
    }

    fn execute_eval_n(&mut self, n: usize, debugger: &mut impl Debugger) -> Result<()> {
        ensure!(
            n <= self.stack.len(),
            "Interpretter stuck is corrupt, expected stack with minimum stack size {n} but found {stack_len}.",
            stack_len = self.stack.len()
        );
        let proc_idx = self.stack.len() - n;
        let stack_start_idx = proc_idx + 1;
        let proc_val = std::mem::take(&mut self.stack[proc_idx]);
        match proc_val {
            Val::ByteCodeProc(proc) => {
                let expected_args = proc.arg_count;
                let actual_args = n - 1;
                self.frames.push(Frame {
                    bytecode: ByteCodeIter::from_proc(proc),
                    stack_start_idx,
                });
                debugger.eval_proc(self);
                if expected_args != actual_args {
                    bail!(
                        "{name} expected {expected_args} but found {actual_args}.",
                        name = self.current_proc().map(|p| p.name.as_str()).unwrap_or("_")
                    );
                }
            }
            Val::NativeProc(proc) => {
                self.frames.push(Frame {
                    bytecode: proc.placeholder_bytecode_iter(),
                    stack_start_idx,
                });
                debugger.eval_proc(self);
                let res = proc.eval(&self.modules, &self.stack[stack_start_idx..])?;
                self.stack.push(res);
                self.pop_frame(debugger)?;
            }
            v => bail!(
                "Expected procedure but found {v}.\nStack: {stack:?}",
                stack = self.stack
            ),
        };
        Ok(())
    }

    fn execute_set_val(
        &mut self,
        module: ModuleSource,
        s: Symbol,
        debugger: &mut impl Debugger,
    ) -> Result<()> {
        let v = self.stack.pop().unwrap();
        debugger.define(self, &s, &v);
        self.modules.set_value(module, s, v);
        Ok(())
    }

    fn import_module(&mut self, filepath: PathBuf, debugger: &mut impl Debugger) -> Result<()> {
        let filepath = self.resolve_path(&filepath)?;
        let module_source = ModuleSource::File(filepath.clone());
        if let Some(frame) = self.frames.last_mut() {
            if let Some(current_module) = self.modules.get_mut(&frame.bytecode.inner().module) {
                let module_identifier = filepath
                    .file_stem()
                    .ok_or_else(|| anyhow!("Could not parse file stem for filename {filepath:?}."))?
                    .to_string_lossy()
                    .to_string();
                current_module.add_import(module_identifier, module_source.clone());
            }
        }
        if self.modules.has_module(&module_source) {
            return Ok(());
        }
        let contents = std::fs::read_to_string(&filepath)
            .with_context(|| format!("Failed to read file {filepath:?} to string."))?;
        let asts = Ast::from_sexp_str(&contents)
            .with_context(|| anyhow!("Failed to make AST for file {filepath:?}."))?;
        let args = CodeBlockArgs {
            name: Some(SmolStr::new(format!("init-module-{filepath:?}"))),
            ..CodeBlockArgs::default()
        };
        let bytecode = CodeBlock::with_ast(args.clone(), asts.iter())
            .with_context(|| anyhow!("Failed to analyze AST for {filepath:?}."))?
            .to_module_definition(module_source.clone())
            .with_context(|| anyhow!("Failed to create bytecode for {filepath:?}."))?;
        self.modules.add_module(Module::new(module_source.clone()));
        self.stack.push(bytecode.into());
        self.execute_eval_n(1, debugger)
    }

    fn resolve_path(&self, path: &Path) -> Result<PathBuf> {
        let mut resolved_path = self.root.clone();
        resolved_path.extend(path);
        std::fs::canonicalize(&resolved_path).with_context(|| {
            anyhow!(
                "Failed to resolve {path:?} with working directory {root:?}.",
                root = self.root
            )
        })
    }
}

impl std::fmt::Display for StackTrace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Stack trace:")?;
        for trace in self.trace.iter() {
            writeln!(
                f,
                "  - {module}/{proc}",
                module = trace.module,
                proc = trace.proc_name,
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    const MODULE: ModuleSource = ModuleSource::Virtual("test");

    #[test]
    fn can_execute_ast() {
        assert_eq!(
            Vm::new().eval_str(MODULE, "(+ 1 2 (- 3 4))").unwrap(),
            vec![2.into()]
        );
    }

    #[test]
    fn if_with_true_returns_first_expr_result() {
        assert_eq!(
            Vm::new()
                .eval_str(MODULE, "(if true (* 10 2) (+ 10 2))")
                .unwrap(),
            vec![20.into()],
        );
    }

    #[test]
    fn if_with_false_returns_second_expr_result() {
        assert_eq!(
            Vm::new()
                .eval_str(MODULE, "(if false (* 10 2) (+ 10 2))")
                .unwrap(),
            vec![12.into()],
        )
    }

    #[test]
    fn if_with_true_and_single_arm_returns_true() {
        assert_eq!(
            Vm::new().eval_str(MODULE, "(if true (* 10 2))").unwrap(),
            vec![20.into()],
        )
    }

    #[test]
    fn if_with_false_and_single_arm_returns_void() {
        assert_eq!(
            Vm::new().eval_str(MODULE, "(if false (* 10 2))").unwrap(),
            vec![Val::Void],
        )
    }

    #[test]
    fn recursive_function_definition_calls_recursively() {
        assert_eq!(
            Vm::new()
                .eval_str(
                    MODULE,
                    r#"
(define (fib n) (if (<= n 2) 1 (+ (fib (- n 1)) (fib (- n 2)))))
(fib 10)
"#
                )
                .unwrap(),
            vec![Val::Void, 55.into()],
        );
    }

    #[test]
    fn eval_with_wrong_number_of_args_returns_error() {
        let mut vm = Vm::new();
        let proc_val = vm
            .eval_str(MODULE, "(lambda (x) (+ 1 x))")
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        let proc = match proc_val {
            Val::ByteCodeProc(bc) => bc,
            _ => unreachable!(),
        };
        assert_eq!(
            vm.eval_bytecode(proc.clone(), &[Val::Int(1)], &mut ())
                .unwrap(),
            Val::Int(2)
        );
        assert!(vm.eval_bytecode(proc.clone(), &[], &mut ()).is_err());
        assert!(vm
            .eval_bytecode(proc.clone(), &[Val::Int(1), Val::Int(2)], &mut ())
            .is_err());
    }

    #[test]
    fn import_module_creates_new_module() {
        let mut vm = Vm::new();
        assert_eq!(
            vm.modules()
                .iter()
                .map(|m| m.source().clone())
                .collect::<Vec<_>>(),
            vec![ModuleSource::Global]
        );

        vm.eval_str(MODULE, "(import \"/dev/null\")").unwrap();
        assert_eq!(
            vm.modules()
                .iter()
                .map(|m| m.source().clone())
                .collect::<Vec<_>>(),
            vec![
                ModuleSource::Global,
                MODULE,
                ModuleSource::File(PathBuf::from("/dev/null"))
            ]
        );
    }

    #[test]
    fn import_nonexistent_module_returns_error() {
        let mut vm = Vm::new();
        let res = vm.eval_str(MODULE, "(import \"test_data/does_not_exist.spore\")");
        assert!(res.is_err(), "Expected error but no error encountered");
        assert_eq!(
            vm.modules()
                .iter()
                .map(|m| m.source().clone())
                .collect::<Vec<_>>(),
            vec![ModuleSource::Global, MODULE]
        );
    }

    #[test]
    fn import_module_with_runtime_error_returns_error() {
        let mut vm = Vm::new();
        let res = vm.eval_str(MODULE, &format!("(import \"test_data/bad.spore\")"));
        assert!(res.is_err());
        assert_eq!(
            vm.modules()
                .iter()
                .map(|m| m.source().clone())
                .collect::<Vec<_>>(),
            vec![ModuleSource::Global, MODULE]
        );
    }

    #[test]
    fn import_module_return_void() {
        let mut vm = Vm::new();
        assert_eq!(
            vm.eval_str(MODULE, &"(import \"test_data/circle.spore\")")
                .unwrap(),
            vec![Val::Void,],
        );
    }

    #[test]
    fn import_module_allows_access_to_module() {
        let mut vm = Vm::new();
        vm.eval_str(MODULE, &format!("(import \"test_data/circle.spore\")"))
            .unwrap();
        assert_eq!(
            vm.modules()
                .iter()
                .map(|m| m.source().clone())
                .collect::<Vec<_>>(),
            vec![
                ModuleSource::Global,
                MODULE,
                ModuleSource::File(PathBuf::from_iter([
                    vm.working_directory(),
                    Path::new("test_data/circle.spore")
                ]))
            ]
        );
        assert_eq!(
            vm.eval_str(MODULE, &format!("(circle/circle-area 2)",),)
                .unwrap(),
            vec![Val::Float(12.56),],
        );
    }
}
