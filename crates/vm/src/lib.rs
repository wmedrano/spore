use std::{collections::HashMap, sync::Arc};

use smol_str::SmolStr;
use val::Val;

use compiler::Compiler;
use error::BacktraceError;
use val::{
    bytecode::{ByteCode, Instruction},
    native_function::{NativeFunction, NativeFunctionContext},
    InternalVal,
};
use val_store::{ValId, ValStore};

pub use ast::AstParseError;
pub use error::{CompileError, VmError, VmResult};

mod ast;
mod builtins;
mod compiler;
mod error;
mod tokenizer;
mod val;
mod val_store;

/// The Spore virtual machine.
#[derive(Debug)]
pub struct Vm {
    /// The data stack. This is used to store temporary values used for computation.
    stack: Vec<InternalVal>,
    /// Map from binding name to value. This is used to store global values.
    values: HashMap<SmolStr, InternalVal>,
    /// The current stack frame. This contains what should be evaluated next and some extra context.
    stack_frame: StackFrame,
    /// The pending stack frames.
    previous_stack_frames: Vec<StackFrame>,
    /// Manages lifetime of all values, aside from simple atoms like bool/int/float.
    pub(crate) val_store: ValStore,
    /// Contains bytecode compilation settings,
    pub(crate) settings: VmSettings,
}

/// Settings for the Spore virtual machine.
#[derive(Copy, Clone, Debug, Default)]
pub struct VmSettings {
    /// If aggressive inlining should be used. This should be disabled for any interactive
    /// development where values may be redefined.
    pub enable_aggressive_inline: bool,
}

/// Used to decide the next instruction to take.
#[derive(Default, Debug)]
struct StackFrame {
    bytecode_id: ValId<Arc<ByteCode>>,
    /// The instructions that will be taken.
    bytecode: Arc<ByteCode>,
    /// The index of the next instruction within bytecode.
    bytecode_idx: usize,
    /// The index of the stack for the first value of this stack frame's local stack.
    stack_start: usize,
}

impl Default for Vm {
    /// Create a new virtual machine.
    fn default() -> Vm {
        Vm::new(VmSettings::default())
    }
}

impl Vm {
    /// Create a new virtual machine.
    pub fn new(settings: VmSettings) -> Vm {
        let mut vm = Vm {
            // TODO: Determine optimal size for stack. Small values may perform, better, but
            // exceeding the capacity may cause performance degregations.
            stack: Vec::with_capacity(4096),
            values: HashMap::new(),
            // Allocate for a function call depth of 64. This is more than enough for most programs.
            previous_stack_frames: Vec::with_capacity(64),
            stack_frame: StackFrame::default(),
            val_store: ValStore::default(),
            settings,
        };
        vm.register_native_function("+", builtins::add);
        vm.register_native_function("<", builtins::less);
        vm.register_native_function("list", builtins::list);
        vm.register_native_function("string-join", builtins::string_join);
        vm.register_native_function("working-directory", builtins::working_directory);
        vm
    }

    /// Run the garbage collector.
    pub fn run_gc(&mut self) {
        let vals = self
            .stack
            .iter()
            .copied()
            .chain(self.values.values().copied())
            .chain(std::iter::once(InternalVal::ByteCodeFunction(
                self.stack_frame.bytecode_id,
            )))
            .chain(self.stack_frame.bytecode.values())
            .chain(self.previous_stack_frames.iter().flat_map(|sf| {
                std::iter::once(InternalVal::ByteCodeFunction(sf.bytecode_id))
                    .chain(sf.bytecode.values())
            }));
        self.val_store.run_gc(vals)
    }

    /// Register a native function that can be called within the virtual machine.
    fn register_native_function(&mut self, name: impl Into<SmolStr>, func: NativeFunction) {
        self.values
            .insert(name.into(), InternalVal::NativeFunction(func));
    }

    /// Evaluate a string in the virtual machine.
    pub fn eval_str(&mut self, source: &str) -> VmResult<Val> {
        self.run_gc();
        let bytecode = Compiler::compile(self, source)?;
        let bytecode_id = self.val_store.insert_bytecode(bytecode);
        let v = self.eval_bytecode(bytecode_id)?;
        Ok(Val::new(self, v))
    }

    /// Evaluate some bytecode in the virtual machine.
    fn eval_bytecode(&mut self, bytecode_id: ValId<Arc<ByteCode>>) -> VmResult<InternalVal> {
        self.stack.clear();
        self.previous_stack_frames.clear();
        let bytecode = self.val_store.get_bytecode(bytecode_id).clone();
        self.stack_frame = StackFrame {
            bytecode_id,
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
    fn run_next(&mut self) -> VmResult<Option<InternalVal>> {
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
            Instruction::PushConst(c) => self.stack.push(*c),
            Instruction::PushCurrentFunction => {
                let f = InternalVal::ByteCodeFunction(self.stack_frame.bytecode_id);
                self.stack.push(f);
            }
            Instruction::GetArg(n) => {
                let val = self.stack[self.stack_frame.stack_start + *n];
                self.stack.push(val);
            }
            Instruction::Deref(symbol) => {
                let v = match self.values.get(symbol) {
                    Some(v) => *v,
                    None => return Err(VmError::SymbolNotDefined(symbol.to_string())),
                };
                self.stack.push(v);
            }
            Instruction::Define(symbol) => {
                let v = self.stack.pop().ok_or_else(BacktraceError::capture)?;
                self.values.insert(symbol.clone(), v);
            }
            Instruction::Eval(n) => self.execute_eval(*n)?,
            Instruction::EvalNative { func, arg_count } => {
                self.execute_eval_native(*func, *arg_count)?
            }
            Instruction::JumpIf(n) => match self.stack.pop() {
                Some(InternalVal::Bool(true)) => self.stack_frame.bytecode_idx += *n,
                Some(InternalVal::Bool(false)) => {}
                v => {
                    let v = v.unwrap_or(InternalVal::Void);
                    return Err(VmError::TypeError {
                        context: "if",
                        expected: InternalVal::BOOL_TYPE_NAME,
                        actual: v.type_name(),
                        value: v.formatted(self).to_string(),
                    });
                }
            },
            Instruction::Jump(n) => self.stack_frame.bytecode_idx += *n,
        }
        Ok(None)
    }

    fn execute_eval_native(&mut self, func: NativeFunction, arg_count: usize) -> VmResult<()> {
        match arg_count {
            0 => {
                // Unsafe OK: stack_start is stack length.
                let v = func(unsafe { NativeFunctionContext::new(self, self.stack.len()) })?;
                self.stack.push(v);
            }
            _ => {
                let stack_start = self.stack.len() - arg_count;
                // Unsafe OK: stack_start is less than stack length.
                let res = func(unsafe { NativeFunctionContext::new(self, stack_start) })?;
                self.stack.truncate(stack_start + 1);
                self.stack[stack_start] = res;
            }
        };
        Ok(())
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
        let func_val = self.stack[function_idx];
        match func_val {
            InternalVal::NativeFunction(func) => {
                // Unsafe OK: stack_start is less than stack length.
                self.stack[function_idx] =
                    func(unsafe { NativeFunctionContext::new(self, stack_start) })?;
                self.stack.truncate(stack_start);
                Ok(())
            }
            InternalVal::ByteCodeFunction(bytecode_id) => {
                let bytecode = self.val_store.get_bytecode(bytecode_id).clone();
                let arg_count = n - 1;
                if bytecode.arg_count != arg_count {
                    return Err(VmError::ArityError {
                        function: bytecode.name.clone(),
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
                let previous_stack_frame = std::mem::replace(
                    &mut self.stack_frame,
                    StackFrame {
                        bytecode_id,
                        bytecode,
                        bytecode_idx: 0,
                        stack_start,
                    },
                );
                self.previous_stack_frames.push(previous_stack_frame);
                Ok(())
            }
            v => Err(VmError::TypeError {
                context: "function call",
                expected: InternalVal::FUNCTION_TYPE_NAME,
                actual: v.type_name(),
                value: v.formatted(self).to_string(),
            }),
        }
    }

    /// Execute returning from the current stack frame.
    fn execute_return(&mut self) -> Option<InternalVal> {
        // 1. Return the current value to the top of the stack.
        let ret_val = if self.stack_frame.stack_start < self.stack.len() {
            // Unwrap OK: The above statement is never true when len == 0.
            self.stack.pop().unwrap()
        } else {
            InternalVal::Void
        };
        // 2. Set up the next continuation.
        match self.previous_stack_frames.pop() {
            // 2a. Pop the stack frame and replace the top value in the stack with the return value.
            Some(c) => {
                self.stack.truncate(self.stack_frame.stack_start);
                match self.stack.last_mut() {
                    Some(v) => *v = ret_val,
                    None => unreachable!(),
                }
                self.stack_frame = c;
                None
            }
            // 2b. There is nothing to continue to so return the value.
            None => {
                std::mem::take(&mut self.stack_frame);
                self.stack.clear();
                Some(ret_val)
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
        let mut vm = Vm::default();
        let actual = vm.eval_str("42").unwrap();
        assert_eq!(actual.as_int(), Some(42));
    }

    #[test]
    fn expression_can_evaluate() {
        let mut vm = Vm::default();
        let actual = vm.eval_str("(+ 1 2 3 4.0)").unwrap();
        assert_eq!(actual.as_float(), Some(10.0));
    }

    #[test]
    fn list_function_returns_list() {
        let mut vm = Vm::default();
        let actual = vm.eval_str("(list 1 2.3 \"three\")").unwrap();
        assert_eq!(actual.to_string(), "(1 2.3 \"three\")");
    }

    #[test]
    fn vm_error_is_reported() {
        let mut vm = Vm::default();
        let actual = vm.eval_str("(+ true false)").unwrap_err();
        assert_eq!(
            actual,
            VmError::TypeError {
                context: "+",
                expected: InternalVal::INT_TYPE_NAME,
                actual: InternalVal::BOOL_TYPE_NAME,
                value: "true".to_string(),
            }
        );
    }

    #[test]
    fn compile_error_is_reported() {
        let mut vm = Vm::default();
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
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("(define x 12) (+ x x)").unwrap().as_int(),
            Some(24)
        );
        assert_eq!(vm.eval_str("(+ x 10)").unwrap().as_int(), Some(22));
    }

    #[test]
    fn if_statement_can_return_any_of() {
        let mut vm = Vm::default();
        assert_eq!(vm.eval_str("(if true (+ 1 2))").unwrap().as_int(), Some(3));
        assert_eq!(
            vm.eval_str("(if true (+ 1 2) (+ 3 4))").unwrap().as_int(),
            Some(3)
        );
        assert_eq!(
            vm.eval_str("(if false (+ 1 2) (+ 3 4))").unwrap().as_int(),
            Some(7)
        );
        let got = vm.eval_str("(if false (+ 1 2))").unwrap().is_void();
        assert!(got, "{got}");
    }

    #[test]
    fn if_statement_with_non_bool_predicate_produces_error() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("(if 1 (+ 1 2) (+ 3 4))").unwrap_err(),
            VmError::TypeError {
                context: "if",
                expected: InternalVal::BOOL_TYPE_NAME,
                actual: InternalVal::INT_TYPE_NAME,
                value: "1".to_string(),
            }
        );
        assert_eq!(
            vm.eval_str("(if 1 (+ 1 2))").unwrap_err(),
            VmError::TypeError {
                context: "if",
                expected: InternalVal::BOOL_TYPE_NAME,
                actual: InternalVal::INT_TYPE_NAME,
                value: "1".to_string(),
            }
        );
    }

    #[test]
    fn lambda_can_be_evaluated() {
        let mut vm = Vm::default();
        assert_eq!(vm.eval_str("((lambda () 7))").unwrap().as_int(), Some(7));
        assert_eq!(
            vm.eval_str("((lambda () (+ 1 2 3)))").unwrap().as_int(),
            Some(6)
        );
    }

    #[test]
    fn lambda_with_args_can_be_evaluated() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("((lambda (a b) 4) 1 2)").unwrap().as_int(),
            Some(4)
        );
        assert_eq!(
            vm.eval_str("((lambda (a b) (+ a b)) 1 2)")
                .unwrap()
                .as_int(),
            Some(3)
        );
    }

    #[test]
    fn function_called_with_wrong_number_of_args_produces_error() {
        let mut vm = Vm::default();
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
        let got = vm
            .eval_str("(define (takes-two-args arg1 arg2) (+ arg1 arg2))")
            .unwrap();
        assert!(got.is_void(), "{got}");
        drop(got);
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
        let mut vm = Vm::default();
        let got = vm
            .eval_str("(define (fib n) (if (< n 2) n (+ (fib (+ n -1)) (fib (+ n -2)))))")
            .unwrap();
        assert!(got.is_void(), "{got}");
        drop(got);
        assert_eq!(vm.eval_str("(fib 10)").unwrap().as_int(), Some(55));
    }

    #[test]
    fn infinite_recursion_halts() {
        let mut vm = Vm::default();
        let got = vm.eval_str("(define (recurse) (recurse))").unwrap();
        assert!(got.is_void(), "{got}");
        drop(got);
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
