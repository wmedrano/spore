use std::{path::PathBuf, rc::Rc};

use anyhow::{anyhow, bail, ensure, Context, Result};

use crate::parser::ast::Ast;

use super::{
    debugger::Debugger,
    ir::{CodeBlock, CodeBlockArgs},
    module::{Module, ModuleManager, ModuleSource},
    types::{
        instruction::Instruction,
        proc::bytecode::{ByteCodeIter, ByteCodeProc},
        symbol::Symbol,
        Val,
    },
    Vm,
};

/// An environment to evaluate bytecode on.
pub struct Environment {
    /// All the modules.
    modules: ModuleManager,
    /// The processing stack.
    stack: Vec<Val>,
    /// Contains the current call frame. This includes the instructions that should be run and the
    /// base of the frame's stack.
    frames: Vec<Frame>,
}

pub struct Frame {
    /// The bytecode that is under execution in the frame.
    bytecode: ByteCodeIter,
    /// The index of the local stack.
    stack_start_idx: usize,
}

struct StackTrace {
    trace: Vec<String>,
}

impl Environment {
    /// Create a new environment.
    pub fn new(vm: &Vm) -> Environment {
        Environment {
            modules: ModuleManager::new(vm.globals.clone()),
            stack: Vec::with_capacity(4096),
            frames: Vec::with_capacity(128),
        }
    }

    /// Evaluate an S-Expression string and return the last value. If there are no expression, than
    /// `Val::Void` is returned.
    pub fn eval_str(&mut self, module: ModuleSource, s: &str) -> Result<Vec<Val>> {
        Ast::from_sexp_str(s)?
            .into_iter()
            .map(|ast| {
                let code_block_args = CodeBlockArgs {
                    name: Some("eval-str".to_string()),
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
        self.eval_bytecode_impl(proc, args, debugger)
            .inspect_err(|_| {
                for frame in self.frames.iter_mut() {
                    let bytecode = frame.bytecode.inner();
                    if bytecode.is_module_definition {
                        self.modules.remove_module(&bytecode.module);
                    }
                }
            })
            .with_context(|| self.stack_trace())
    }

    /// Get all the modules.
    pub fn modules(&mut self) -> &ModuleManager {
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

    /// Evaluate a sequence of bytecode.
    fn eval_bytecode_impl(
        &mut self,
        proc: Rc<ByteCodeProc>,
        args: &[Val],
        debugger: &mut impl Debugger,
    ) -> Result<Val> {
        self.prepare(proc.clone(), args)?;
        debugger.eval_proc(self);
        while let Some(frame) = self.frames.last_mut() {
            let instruction = frame.bytecode.next_instruction();
            match instruction {
                Instruction::PushVal(v) => {
                    let v = v.clone();
                    self.stack.push(v);
                }
                Instruction::Eval(n) => {
                    let n = *n;
                    self.execute_eval_n(n, debugger)?
                }
                Instruction::GetArg(n) => {
                    let n = *n;
                    self.execute_get_arg(n)
                }
                Instruction::GetVal(s) => {
                    let maybe_value = self.modules.get_value(
                        &s.module,
                        s.sub_module.as_ref().map(String::as_str),
                        s.symbol.as_str(),
                    );
                    match maybe_value {
                        Some(v) => self.stack.push(v),
                        None => bail!("value for {s} is not defined"),
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
                    let s = s.clone();
                    let module = frame.bytecode.inner().module.clone();
                    self.execute_set_val(&module, s, debugger)?;
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

    fn prepare(&mut self, proc: Rc<ByteCodeProc>, args: &[Val]) -> Result<()> {
        ensure!(
            proc.arg_count == args.len(),
            "Wrong number of args to {proc}"
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

    #[cold]
    fn stack_trace(&self) -> StackTrace {
        StackTrace {
            trace: self
                .frames
                .iter()
                .map(|f| f.bytecode.inner().name.clone())
                .collect(),
        }
    }

    /// Pop the current frame. This truncates the local stack and replaces the top value of the
    /// stack with the return value. The return value is defined as the value at the top of the
    /// local stack.
    fn pop_frame(&mut self, debugger: &mut impl Debugger) -> Result<()> {
        let frame = self.frames.pop().unwrap();
        let return_val = if self.stack.len() > frame.stack_start_idx {
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
            "interpretter stuck is corrupt, expected stack with minimum stack size {n} but found {stack_len}.",
            stack_len = self.stack.len()
        );
        let proc_idx = self.stack.len() - n;
        let proc_val = std::mem::take(&mut self.stack[proc_idx]);
        match proc_val {
            Val::ByteCodeProc(proc) => {
                let expected_args = proc.arg_count;
                let actual_args = n - 1;
                self.frames.push(Frame {
                    bytecode: ByteCodeIter::from_proc(proc),
                    stack_start_idx: proc_idx + 1,
                });
                debugger.eval_proc(self);
                if expected_args != actual_args {
                    bail!(
                        "{name} expected {expected_args} but found {actual_args}",
                        name = self.current_proc().map(|p| p.name.as_str()).unwrap_or("_")
                    );
                }
            }
            Val::NativeProc(proc) => {
                let stack_base = proc_idx + 1;
                let res = {
                    let args = self.stack.drain(stack_base..);
                    proc.eval(&self.modules, args.as_slice())?
                };
                *self.stack.last_mut().unwrap() = res;
            }
            v => bail!(
                "expected procedure but found {v}\nStack: {stack:?}",
                stack = self.stack
            ),
        };
        Ok(())
    }

    fn execute_set_val(
        &mut self,
        module: &ModuleSource,
        s: Symbol,
        debugger: &mut impl Debugger,
    ) -> Result<()> {
        let v = self.stack.pop().unwrap();
        debugger.define(self, &s, &v);
        self.modules.set_value(module, s, v);
        Ok(())
    }

    fn import_module(&mut self, filepath: PathBuf, debugger: &mut impl Debugger) -> Result<()> {
        let module_source = ModuleSource::File(filepath.clone());
        if let Some(frame) = self.frames.last_mut() {
            if let Some(current_module) = self.modules.get_mut(&frame.bytecode.inner().module) {
                let module_identifier = filepath
                    .file_stem()
                    .ok_or_else(|| anyhow!("Could not parse file stem for filename {filepath:?}"))?
                    .to_string_lossy()
                    .to_string();
                current_module.add_import(module_identifier, module_source.clone());
            }
        }
        if self.modules.has_module(&module_source) {
            return Ok(());
        }
        let contents = std::fs::read_to_string(&filepath)
            .with_context(|| format!("filepath: {filepath:?}"))?;
        let asts = Ast::from_sexp_str(&contents)?;
        let args = CodeBlockArgs {
            name: Some(format!("init-module-{filepath:?}")),
            ..CodeBlockArgs::default()
        };
        let bytecode = CodeBlock::with_ast(args.clone(), asts.iter())?
            .to_module_definition(module_source.clone())?;
        self.modules.add_module(Module::new(module_source.clone()));
        self.stack.push(bytecode.into());
        self.execute_eval_n(1, debugger)
    }
}

impl std::fmt::Display for StackTrace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Stack trace:")?;
        for trace in self.trace.iter() {
            writeln!(f, "  {}", trace)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::vm::Vm;

    use super::*;

    const MODULE: ModuleSource = ModuleSource::Virtual("test");

    fn string_list_to_vec(lst: &Val) -> Vec<String> {
        lst.try_slice()
            .unwrap()
            .into_iter()
            .map(|v| v.try_str())
            .map(Result::unwrap)
            .map(str::to_string)
            .collect()
    }

    fn test_file_path(p: &str) -> String {
        let mut full_path = std::env::current_dir().unwrap();
        full_path.push("test_data");
        full_path.push(p);
        full_path.to_str().unwrap().to_string()
    }

    #[test]
    fn can_execute_ast() {
        assert_eq!(
            Vm::new()
                .build_env()
                .eval_str(MODULE, "(+ 1 2 (- 3 4))")
                .unwrap(),
            vec![2.into()]
        );
    }

    #[test]
    fn if_with_true_returns_first_expr_result() {
        assert_eq!(
            Vm::new()
                .build_env()
                .eval_str(MODULE, "(if true (* 10 2) (+ 10 2))")
                .unwrap(),
            vec![20.into()],
        );
    }

    #[test]
    fn if_with_false_returns_second_expr_result() {
        assert_eq!(
            Vm::new()
                .build_env()
                .eval_str(MODULE, "(if false (* 10 2) (+ 10 2))")
                .unwrap(),
            vec![12.into()],
        )
    }

    #[test]
    fn if_with_true_and_single_arm_returns_true() {
        assert_eq!(
            Vm::new()
                .build_env()
                .eval_str(MODULE, "(if true (* 10 2))")
                .unwrap(),
            vec![20.into()],
        )
    }

    #[test]
    fn if_with_false_and_single_arm_returns_void() {
        assert_eq!(
            Vm::new()
                .build_env()
                .eval_str(MODULE, "(if false (* 10 2))")
                .unwrap(),
            vec![Val::Void],
        )
    }

    #[test]
    fn recursive_function_definition_calls_recursively() {
        let mut env = Vm::new().build_env();
        assert_eq!(
            env.eval_str(
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
        let mut env = Vm::new().build_env();
        let proc_val = env
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
            env.eval_bytecode(proc.clone(), &[Val::Int(1)], &mut ())
                .unwrap(),
            Val::Int(2)
        );
        assert!(env.eval_bytecode(proc.clone(), &[], &mut ()).is_err());
        assert!(env
            .eval_bytecode(proc.clone(), &[Val::Int(1), Val::Int(2)], &mut ())
            .is_err());
    }

    #[test]
    fn import_module_creates_new_module() {
        let mut env = Vm::new().build_env();
        let before_modules = env.eval_str(MODULE, "(modules)").unwrap();
        assert_eq!(
            string_list_to_vec(before_modules.first().unwrap()),
            vec!["%global%".to_string(), "%virtual%/test".to_string()]
        );

        let after_modules = env
            .eval_str(MODULE, "(import \"/dev/null\") (modules)")
            .unwrap();
        assert_eq!(
            string_list_to_vec(after_modules.last().unwrap()),
            vec![
                "%global%".to_string(),
                "%virtual%/test".to_string(),
                "/dev/null".to_string()
            ]
        );
    }

    #[test]
    fn import_nonexistent_module_returns_error() {
        let mut env = Vm::new().build_env();
        let res = env.eval_str(
            MODULE,
            &format!(
                "(import \"{path}\")",
                path = test_file_path("does_not_exist.spore")
            ),
        );
        assert!(res.is_err(), "Expected error but no error encountered");
        assert_eq!(
            env.eval_str(MODULE, "(modules)").unwrap(),
            vec![Val::List(Rc::new(vec![
                "%global%".to_string().into(),
                "%virtual%/test".to_string().into(),
            ]))],
            "Expecting the default set of modules with no other module removal/additions."
        );
    }

    #[test]
    fn import_module_with_runtime_error_returns_error() {
        let mut env = Vm::new().build_env();
        let res = env.eval_str(
            MODULE,
            &format!("(import \"{path}\")", path = test_file_path("bad.spore")),
        );
        assert!(res.is_err());
        assert_eq!(
            env.eval_str(MODULE, "(modules)").unwrap(),
            vec![Val::List(Rc::new(vec![
                "%global%".to_string().into(),
                "%virtual%/test".to_string().into(),
            ]))],
        );
    }

    #[test]
    fn import_module_allows_access_to_module() {
        let mut env = Vm::new().build_env();
        assert_eq!(
            env.eval_str(
                MODULE,
                &format!(
                    "(import \"{path}\") (modules) (list-imports \"{MODULE}\") (circle/circle-area 2)",
                    path = test_file_path("circle.spore")
                ),
            )
            .unwrap(),
            vec![
                // Import statement.
                Val::Void,
                // List of all modules.
                Val::List(Rc::new(vec![
                    "%global%".to_string().into(),
                    "%virtual%/test".to_string().into(),
                    test_file_path("circle.spore").into(),
                ])),
                // List of all modules imported into the default module.
                Val::List(Rc::new(vec!["circle".to_string().into(),])),
                // Result of evaluating circle-area procedure.
                Val::Float(12.56),
            ],
        );
    }
}
