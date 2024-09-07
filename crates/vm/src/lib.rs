use std::collections::HashMap;

use compiler::Compiler;
use error::{BacktraceError, VmError, VmResult};
use val::{ByteCode, Instruction};

mod ast;
mod builtins;
mod compiler;
mod tokenizer;
mod val;

pub mod error;
pub use val::Val;

/// The spore virtual machine.
pub struct Vm {
    /// The data stack. This is used to store temporary values used for compuation.
    stack: Vec<Val>,
    /// Map from binding name to value. This is used to store global values.
    values: HashMap<String, Val>,
    /// The current stack frame. This contains what should be evaluated next and some extra context.
    stack_frame: StackFrame,
    /// The pending stack frames.
    previous_stack_frames: Vec<StackFrame>,
}

/// Used to decide the next instruction to take.
#[derive(Default, Debug)]
pub struct StackFrame {
    /// The instructions that will be taken.
    bytecode: ByteCode,
    /// The index of the next instruction within bytecode.
    bytecode_idx: usize,
    /// The index of the stack for the first value of this stack frame's local stack.
    stack_start: usize,
}

impl Default for Vm {
    /// Create a new virtual machine.
    fn default() -> Vm {
        Vm::new()
    }
}

impl Vm {
    /// Create a new virtual machine.
    pub fn new() -> Vm {
        let mut vm = Vm {
            // TODO: Determine optimal size for stack. Small values may perform, better, but
            // exceeding the capacity may cause performance degregations.
            stack: Vec::with_capacity(4096),
            values: HashMap::new(),
            // Allocate for a function call depth of 64. This is more than enough for most programs.
            previous_stack_frames: Vec::with_capacity(64),
            stack_frame: StackFrame::default(),
        };
        vm.register_native_function("+", builtins::add);
        vm.register_native_function("<", builtins::less);
        vm
    }

    /// Register a native function that can be called within the virtual machine.
    pub fn register_native_function(
        &mut self,
        name: impl Into<String>,
        func: fn(&Vm, &[Val]) -> VmResult<Val>,
    ) {
        self.values.insert(name.into(), Val::NativeFunction(func));
    }

    /// Evaluate a string in the virtual machine.
    pub fn eval_str(&mut self, source: &str) -> VmResult<Val> {
        let bytecode = Compiler::compile(source)?;
        self.eval_bytecode(bytecode)
    }

    /// Evaluate some bytecode in the virtual machine.
    pub fn eval_bytecode(&mut self, bytecode: ByteCode) -> VmResult<Val> {
        self.stack.clear();
        self.previous_stack_frames.clear();
        self.stack_frame = StackFrame {
            bytecode,
            bytecode_idx: 0,
            stack_start: 0,
        };
        loop {
            if let Some(v) = self.run_next()? {
                return Ok(v);
            }
        }
    }

    /// Create an object that can pretty print `v`.
    pub fn formatted_val<'a>(&self, v: &'a Val) -> impl 'a + std::fmt::Display {
        FormattedVal { v }
    }

    /// Run the next instruction in the virtual machine.
    ///
    /// If there are no more instructions to run, then `Some(return_value)` will be
    /// returned. Otherwise, `None` will be returned.
    fn run_next(&mut self) -> VmResult<Option<Val>> {
        let maybe_instruction = self
            .stack_frame
            .bytecode
            .instructions
            .get(self.stack_frame.bytecode_idx);
        self.stack_frame.bytecode_idx += 1;
        let instruction = match maybe_instruction {
            Some(instruction) => instruction,
            None => return Ok(self.execute_return()),
        };
        match instruction {
            Instruction::PushConst(c) => self.stack.push(c.clone()),
            Instruction::GetArg(n) => {
                let val = self.stack[self.stack_frame.stack_start + *n].clone();
                self.stack.push(val);
            }
            Instruction::Deref(symbol) => {
                let v = match self.values.get(symbol) {
                    Some(v) => v.clone(),
                    None => return Err(VmError::SymbolNotDefined(symbol.clone())),
                };
                self.stack.push(v);
            }
            Instruction::Define(symbol) => {
                let v = self.stack.pop().ok_or_else(BacktraceError::capture)?;
                self.values.insert(symbol.clone(), v);
            }
            Instruction::Eval(n) => self.execute_eval(*n)?,
            Instruction::JumpIf(n) => match self.stack.pop() {
                Some(Val::Bool(true)) => self.stack_frame.bytecode_idx += *n,
                Some(Val::Bool(false)) => {}
                v => {
                    let v = v.unwrap_or(Val::Void);
                    return Err(VmError::TypeError {
                        expected: Val::BOOL_TYPE_NAME,
                        actual: v.type_name(),
                        value: self.formatted_val(&v).to_string(),
                    });
                }
            },
            Instruction::Jump(n) => self.stack_frame.bytecode_idx += *n,
        }
        Ok(None)
    }

    /// Execute the evaluation of the top n values in the stack.
    ///
    /// The deepest value should be a function with the rest of the values being the arguments.
    fn execute_eval(&mut self, n: usize) -> VmResult<()> {
        if n == 0 {
            Err(BacktraceError::capture())?;
        }
        let function_idx = self
            .stack
            .len()
            .checked_sub(n)
            .ok_or_else(BacktraceError::capture)?;
        let stack_start = function_idx + 1;
        match &self.stack[function_idx] {
            Val::NativeFunction(func) => {
                let args = &self.stack[stack_start..];
                let v = func(self, args)?;
                self.stack[function_idx] = v;
                self.stack.truncate(stack_start);
                Ok(())
            }
            Val::ByteCodeFunction(bytecode) => {
                let arg_count = n - 1;
                if bytecode.arg_count != arg_count {
                    return Err(VmError::ArityError {
                        function: bytecode.name.clone().into(),
                        expected: bytecode.arg_count,
                        actual: arg_count,
                    });
                }
                if self.previous_stack_frames.capacity() == self.previous_stack_frames.len() {
                    let mut call_stack = Vec::with_capacity(1 + self.previous_stack_frames.len());
                    call_stack.push(self.stack_frame.bytecode.name.clone());
                    call_stack.extend(
                        self.previous_stack_frames
                            .iter()
                            .rev()
                            .map(|sf| sf.bytecode.name.clone()),
                    );
                    return Err(VmError::MaximumRecursionDepth {
                        call_stack,
                        max_depth: self.previous_stack_frames.len(),
                    });
                }
                self.previous_stack_frames
                    .push(std::mem::take(&mut self.stack_frame));
                self.stack_frame = StackFrame {
                    bytecode: bytecode.clone(),
                    bytecode_idx: 0,
                    stack_start,
                };
                Ok(())
            }
            v => Err(VmError::TypeError {
                expected: Val::FUNCTION_TYPE_NAME,
                actual: v.type_name(),
                value: self.formatted_val(&v).to_string(),
            }),
        }
    }

    /// Execute returning from the current stack frame.
    fn execute_return(&mut self) -> Option<Val> {
        // 1. Return the current value to the top of the stack.
        let ret_val = if self.stack_frame.stack_start < self.stack.len() {
            self.stack.pop().unwrap_or(Val::Void)
        } else {
            Val::Void
        };
        self.stack.truncate(self.stack_frame.stack_start);
        match self.stack.last_mut() {
            Some(v) => *v = ret_val,
            None => self.stack.push(ret_val),
        }
        // 2. Set up the new stack frame or return the new value if there are no more stack frame.
        match self.previous_stack_frames.pop() {
            Some(c) => {
                self.stack_frame = c;
                None
            }
            None => {
                std::mem::take(&mut self.stack_frame);
                Some(self.stack.pop().unwrap_or(Val::Void))
            }
        }
    }
}

pub struct FormattedVal<'a> {
    v: &'a Val,
}

impl<'a> std::fmt::Display for FormattedVal<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.v {
            Val::Void => Ok(()),
            Val::Bool(x) => write!(f, "{x}"),
            Val::Int(x) => write!(f, "{x}"),
            Val::Float(x) => write!(f, "{x}"),
            Val::String(x) => write!(f, "{x}"),
            Val::ByteCodeFunction(bc) => write!(
                f,
                "<function {name}>",
                name = if bc.name.is_empty() {
                    "_"
                } else {
                    bc.name.as_str()
                }
            ),
            Val::NativeFunction(_) => write!(f, "<function native-function>"),
        }
    }
}

#[cfg(test)]
mod tests {
    use error::CompileError;

    use super::*;

    #[test]
    fn constant_expression_evaluates_to_constant() {
        let mut vm = Vm::new();
        let actual = vm.eval_str("42").unwrap();
        assert_eq!(actual, Val::Int(42));
    }

    #[test]
    fn expression_can_evaluate() {
        let mut vm = Vm::new();
        let actual = vm.eval_str("(+ 1 2 3 4.0)").unwrap();
        assert_eq!(actual, Val::Float(10.0));
    }

    #[test]
    fn vm_error_is_reported() {
        let mut vm = Vm::new();
        let actual = vm.eval_str("(+ true false)").unwrap_err();
        assert_eq!(
            actual,
            VmError::TypeError {
                expected: Val::INT_TYPE_NAME,
                actual: Val::BOOL_TYPE_NAME,
                value: "true".to_string(),
            }
        );
    }

    #[test]
    fn compile_error_is_reported() {
        let mut vm = Vm::new();
        let actual = vm.eval_str("((define x 12))").unwrap_err();
        assert_eq!(
            actual,
            VmError::CompileError(CompileError::ExpectedExpression {
                context: "function call"
            })
        );
    }

    #[test]
    fn defined_variable_can_be_referenced() {
        let mut vm = Vm::new();
        assert_eq!(vm.eval_str("(define x 12) (+ x x)").unwrap(), Val::Int(24));
        assert_eq!(vm.eval_str("(+ x 10)").unwrap(), Val::Int(22));
    }

    #[test]
    fn if_statement_can_return_any_of() {
        let mut vm = Vm::new();
        assert_eq!(vm.eval_str("(if true (+ 1 2))").unwrap(), Val::Int(3));
        assert_eq!(
            vm.eval_str("(if true (+ 1 2) (+ 3 4))").unwrap(),
            Val::Int(3)
        );
        assert_eq!(
            vm.eval_str("(if false (+ 1 2) (+ 3 4))").unwrap(),
            Val::Int(7)
        );
        assert_eq!(vm.eval_str("(if false (+ 1 2))").unwrap(), Val::Void);
    }

    #[test]
    fn if_statement_with_non_bool_predicate_produces_error() {
        let mut vm = Vm::new();
        assert_eq!(
            vm.eval_str("(if 1 (+ 1 2) (+ 3 4))").unwrap_err(),
            VmError::TypeError {
                expected: Val::BOOL_TYPE_NAME,
                actual: Val::INT_TYPE_NAME,
                value: "1".to_string(),
            }
        );
        assert_eq!(
            vm.eval_str("(if 1 (+ 1 2))").unwrap_err(),
            VmError::TypeError {
                expected: Val::BOOL_TYPE_NAME,
                actual: Val::INT_TYPE_NAME,
                value: "1".to_string(),
            }
        );
    }

    #[test]
    fn lambda_can_be_evaluated() {
        let mut vm = Vm::new();
        assert_eq!(vm.eval_str("((lambda () 7))").unwrap(), Val::Int(7));
        assert_eq!(vm.eval_str("((lambda () (+ 1 2 3)))").unwrap(), Val::Int(6));
    }

    #[test]
    fn lambda_with_args_can_be_evaluated() {
        let mut vm = Vm::new();
        assert_eq!(vm.eval_str("((lambda (a b) 4) 1 2)").unwrap(), Val::Int(4));
        assert_eq!(
            vm.eval_str("((lambda (a b) (+ a b)) 1 2)").unwrap(),
            Val::Int(3)
        );
    }

    #[test]
    fn function_called_with_wrong_number_of_args_produces_error() {
        let mut vm = Vm::new();
        assert_eq!(
            vm.eval_str("((lambda () 10) 1)").unwrap_err(),
            VmError::ArityError {
                function: "".into(),
                expected: 0,
                actual: 1
            },
        );
        assert_eq!(
            vm.eval_str("((lambda (a) a))").unwrap_err(),
            VmError::ArityError {
                function: "".into(),
                expected: 1,
                actual: 0
            },
        );
        assert_eq!(
            vm.eval_str("(define (takes-two-args arg1 arg2) (+ arg1 arg2))")
                .unwrap(),
            Val::Void
        );
        assert_eq!(
            vm.eval_str("(takes-two-args 1)").unwrap_err(),
            VmError::ArityError {
                function: "takes-two-args".into(),
                expected: 2,
                actual: 1,
            },
        );
    }

    #[test]
    fn can_call_function_recursively() {
        let mut vm = Vm::new();
        assert_eq!(
            vm.eval_str("(define (fib n) (if (< n 2) n (+ (fib (+ n -1)) (fib (+ n -2)))))")
                .unwrap(),
            Val::Void
        );
        assert_eq!(vm.eval_str("(fib 10)").unwrap(), Val::Int(55));
    }

    #[test]
    fn infinite_recursion_halts() {
        let mut vm = Vm::new();
        assert_eq!(
            vm.eval_str("(define (recurse) (recurse))").unwrap(),
            Val::Void,
        );
        assert_eq!(
            vm.eval_str("(recurse)").unwrap_err(),
            VmError::MaximumRecursionDepth {
                max_depth: 64,
                call_stack: std::iter::repeat("recurse")
                    .take(64)
                    .chain(std::iter::once(""))
                    .map(String::from)
                    .collect(),
            }
        );
    }
}
