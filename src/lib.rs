use std::collections::HashMap;

use compiler::Compiler;
use error::{BacktraceError, VmError, VmResult};
use val::{ByteCode, Instruction, Val};

mod ast;
mod builtins;
mod compiler;
mod error;
mod tokenizer;
mod val;

/// The spore virtual machine.
pub struct Vm {
    /// The data stack. This is used to store temporary values used for compuation.
    stack: Vec<Val>,
    /// Map from binding name to value. This is used to store global values.
    values: HashMap<String, Val>,
    /// The current continuation. This contains what should be evaluated next and some extra
    /// context.
    continuation: Continuation,
    /// The pending continuations.
    continuations: Vec<Continuation>,
}

/// Used to decide the next instruction to take.
#[derive(Default, Debug)]
pub struct Continuation {
    /// The instructions that will be taken.
    bytecode: ByteCode,
    /// The index of the next instruction within bytecode.
    bytecode_idx: usize,
    /// The index of the stack for the first value of this continuation's local stack.
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
            continuations: Vec::with_capacity(64),
            continuation: Continuation::default(),
        };
        vm.register_native_function("+", builtins::add);
        vm.register_native_function("<", builtins::less);
        vm
    }

    /// Register a native function that can be called within the virtual machine.
    pub fn register_native_function(
        &mut self,
        name: impl Into<String>,
        func: fn(&[Val]) -> VmResult<Val>,
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
        self.continuations.clear();
        self.continuation = Continuation {
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

    /// Run the next instruction in the virtual machine.
    ///
    /// If there are no more instructions to run, then `Some(return_value)` will be
    /// returned. Otherwise, `None` will be returned.
    fn run_next(&mut self) -> VmResult<Option<Val>> {
        let maybe_instruction = self
            .continuation
            .bytecode
            .instructions
            .get(self.continuation.bytecode_idx);
        self.continuation.bytecode_idx += 1;
        let instruction = match maybe_instruction {
            Some(instruction) => instruction,
            None => return Ok(self.execute_return()),
        };
        match instruction {
            Instruction::PushConst(c) => self.stack.push(c.clone()),
            Instruction::GetArg(n) => {
                let val = self.stack[self.continuation.stack_start + *n].clone();
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
                Some(Val::Bool(true)) => self.continuation.bytecode_idx += *n,
                Some(Val::Bool(false)) => {}
                v => {
                    return Err(VmError::TypeError {
                        expected: Val::BOOL_TYPE_NAME,
                        actual: v.unwrap_or(Val::Void).type_name(),
                    })
                }
            },
            Instruction::Jump(n) => self.continuation.bytecode_idx += *n,
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
                let v = func(args)?;
                self.stack[function_idx] = v;
                self.stack.truncate(stack_start);
                Ok(())
            }
            Val::ByteCodeFunction(bytecode) => {
                let arg_count = n - 1;
                if bytecode.arg_count != arg_count {
                    return Err(VmError::ArityError {
                        expected: bytecode.arg_count,
                        actual: arg_count,
                    });
                }
                if self.continuations.capacity() == self.continuations.len() {
                    return Err(VmError::MaximumRecursionDepth(self.continuations.len()));
                }
                self.continuations
                    .push(std::mem::take(&mut self.continuation));
                self.continuation = Continuation {
                    bytecode: bytecode.clone(),
                    bytecode_idx: 0,
                    stack_start,
                };
                Ok(())
            }
            v => Err(VmError::TypeError {
                expected: Val::FUNCTION_TYPE_NAME,
                actual: v.type_name(),
            }),
        }
    }

    /// Execute returning from the current continuation.
    fn execute_return(&mut self) -> Option<Val> {
        // 1. Return the current value to the top of the stack.
        let ret_val = if self.continuation.stack_start < self.stack.len() {
            self.stack.pop().unwrap_or(Val::Void)
        } else {
            Val::Void
        };
        self.stack.truncate(self.continuation.stack_start);
        match self.stack.last_mut() {
            Some(v) => *v = ret_val,
            None => self.stack.push(ret_val),
        }
        // 2. Set up the new continuation or return the new value if there are no more
        // continuations.
        match self.continuations.pop() {
            Some(c) => {
                self.continuation = c;
                None
            }
            None => {
                std::mem::take(&mut self.continuation);
                Some(self.stack.pop().unwrap_or(Val::Void))
            }
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
            }
        );
        assert_eq!(
            vm.eval_str("(if 1 (+ 1 2))").unwrap_err(),
            VmError::TypeError {
                expected: Val::BOOL_TYPE_NAME,
                actual: Val::INT_TYPE_NAME,
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
    fn lambda_called_with_wrong_number_of_args_produces_error() {
        let mut vm = Vm::new();
        assert_eq!(
            vm.eval_str("((lambda () 10) 1)").unwrap_err(),
            VmError::ArityError {
                expected: 0,
                actual: 1
            },
        );
        assert_eq!(
            vm.eval_str("((lambda (a) a))").unwrap_err(),
            VmError::ArityError {
                expected: 1,
                actual: 0
            },
        );
    }

    #[test]
    fn can_call_function_recursively() {
        let mut vm = Vm::new();
        assert_eq!(
            vm.eval_str(
                "(define fib (lambda (n) (if (< n 2) n (+ (fib (+ n -1)) (fib (+ n -2))))))"
            )
            .unwrap(),
            Val::Void,
        );
        assert_eq!(vm.eval_str("(fib 10)").unwrap(), Val::Int(55));
    }

    #[test]
    fn infinite_recursion_halts() {
        let mut vm = Vm::new();
        assert_eq!(
            vm.eval_str("(define recurse (lambda () (recurse)))")
                .unwrap(),
            Val::Void,
        );
        assert_eq!(
            vm.eval_str("(recurse)").unwrap_err(),
            VmError::MaximumRecursionDepth(64)
        );
    }
}
