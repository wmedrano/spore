use std::{
    collections::HashMap,
    sync::{atomic::AtomicU16, Arc},
};

use compact_str::CompactString;
use gc::{is_garbage_collected, MemoryManager};
use log::*;

use compiler::Compiler;
use error::{BacktraceError, VmError, VmResult};
use val::{
    custom::{CustomType, CustomVal},
    ByteCode, Instruction, NativeFunction, NativeFunctionContext, ProtectedVal, UnsafeVal, ValId,
};

mod ast;
mod builtins;
mod compiler;
pub mod error;
mod gc;
mod tokenizer;
pub mod val;

/// The Spore virtual machine.
#[derive(Debug)]
pub struct Vm {
    /// The data stack. This is used to store temporary values used for computation.
    stack: Vec<UnsafeVal>,
    /// Map from binding name to value. This is used to store global values.
    values: HashMap<CompactString, UnsafeVal>,
    /// The current stack frame. This contains what should be evaluated next and some extra context.
    stack_frame: StackFrame,
    /// The pending stack frames.
    previous_stack_frames: Vec<StackFrame>,
    /// Manages lifetime of all values, aside from simple atoms like bool/int/float.
    pub(crate) objects: MemoryManager,
    /// Contains bytecode compilation settings,
    settings: VmSettings,
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

static VM_ID: AtomicU16 = AtomicU16::new(0);

impl Vm {
    /// Create a new virtual machine.
    pub fn new(settings: VmSettings) -> Vm {
        let start_t = std::time::Instant::now();
        let vm_id = VM_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut vm = Vm {
            // TODO: Determine optimal size for stack. Small values may perform, better, but
            // exceeding the capacity may cause performance degregations.
            stack: Vec::with_capacity(4096),
            values: HashMap::new(),
            // Allocate for a function call depth of 64. This is more than enough for most programs.
            previous_stack_frames: Vec::with_capacity(64),
            stack_frame: StackFrame::default(),
            objects: MemoryManager::new(vm_id),
            settings,
        };
        for (name, func) in builtins::BUILTINS {
            vm = vm.with_native_function(name, *func);
        }
        info!(
            "Initialized Spore VM in {elapsed:?} with {settings:?}",
            elapsed = start_t.elapsed()
        );
        vm
    }

    /// Run the garbage collector.
    pub fn run_gc(&mut self) {
        let is_gc = |v: &UnsafeVal| is_garbage_collected(*v);
        let vals = self
            .stack
            .iter()
            .copied()
            .filter(is_gc)
            .chain(self.values.values().copied().filter(is_gc))
            .chain(
                self.previous_stack_frames
                    .iter()
                    .flat_map(|previous_frame| {
                        previous_frame
                            .bytecode
                            .values()
                            .filter(is_gc)
                            .chain(std::iter::once(previous_frame.bytecode_id.into()))
                    }),
            )
            .chain(self.stack_frame.bytecode.values().filter(is_gc))
            .chain(std::iter::once(self.stack_frame.bytecode_id.into()));
        self.objects.run_gc(vals)
    }

    /// Return the VM with the native function registered.
    pub fn with_native_function(mut self, name: &str, func: NativeFunction) -> Self {
        let func: UnsafeVal = func.into();
        assert!(!is_garbage_collected(func));
        // Unsafe OK: Native functions do not need to register with the vm.
        unsafe { self.register_value(name, func) };
        self
    }

    /// Return the VM with a custom value that is accessible globally.
    pub fn with_custom_value(mut self, name: &str, val: impl CustomType) -> Self {
        let id = self.objects.insert_custom(CustomVal::new(val));
        // Unsafe OK: Custom type is registered in the VM in the line above.
        unsafe { self.register_value(name, id) };
        self
    }

    /// Register a value to the VM.
    ///
    /// # Safety
    /// `val` must already be in the VM if it is a garbage collected type.
    unsafe fn register_value(&mut self, name: &str, val: impl Into<UnsafeVal>) {
        let val = val.into();
        info!(
            "Registering {name:?} to a(n) {tp} value.",
            tp = val.type_name()
        );
        self.values.insert(name.into(), val);
    }

    /// Evaluate a string in the virtual machine.
    pub fn eval_str(&mut self, source: &str) -> VmResult<ProtectedVal> {
        self.run_gc();
        let bytecode = Compiler::compile(self, source)?;
        let bytecode_id = self.objects.insert_bytecode(bytecode);
        let v = self.eval_bytecode(bytecode_id)?;
        // Unsafe OK: `v` was just built so there is no chance for it to garage collect.
        Ok(unsafe { ProtectedVal::new(self, v) })
    }

    /// Evaluate some bytecode in the virtual machine.
    fn eval_bytecode(&mut self, bytecode_id: ValId<Arc<ByteCode>>) -> VmResult<UnsafeVal> {
        self.stack.clear();
        self.previous_stack_frames.clear();
        let bytecode = self.objects.get_bytecode(bytecode_id).clone();
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
    fn run_next(&mut self) -> VmResult<Option<UnsafeVal>> {
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
                let f = UnsafeVal::ByteCodeFunction(self.stack_frame.bytecode_id);
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
            Instruction::JumpIf(n) => {
                if self.stack.pop().unwrap().is_truthy() {
                    self.stack_frame.bytecode_idx += *n;
                }
            }
            Instruction::Jump(n) => self.stack_frame.bytecode_idx += *n,
            Instruction::Return => return Ok(self.execute_return()),
        }
        Ok(None)
    }

    fn execute_eval_native(&mut self, func: NativeFunction, arg_count: usize) -> VmResult<()> {
        let stack_start = self.stack.len() - arg_count;
        let builder = func(NativeFunctionContext::new(self, stack_start))?;
        // Unsafe OK: Value is inserted into VM immediately.
        let v = unsafe { builder.build() };
        match arg_count {
            0 => {
                self.stack.push(v);
            }
            _ => {
                self.stack.truncate(stack_start + 1);
                self.stack[stack_start] = v;
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
            UnsafeVal::NativeFunction(func) => {
                let builder = func(NativeFunctionContext::new(self, stack_start))?;
                // Unsafe OK: Value is inserted into VM immediately.
                let v = unsafe { builder.build() };
                self.stack[function_idx] = v;
                self.stack.truncate(stack_start);
                Ok(())
            }
            UnsafeVal::ByteCodeFunction(bytecode_id) => {
                let bytecode = self.objects.get_bytecode(bytecode_id).clone();
                let arg_count = n - 1;
                if bytecode.arg_count != arg_count {
                    return Err(VmError::ArityError {
                        function: bytecode.name.clone(),
                        expected: bytecode.arg_count,
                        actual: arg_count,
                    });
                }
                if self.previous_stack_frames.capacity() == self.previous_stack_frames.len() {
                    return Err(self.execute_call_stack_limit_reached());
                }
                let new_stack_frame = StackFrame {
                    bytecode_id,
                    bytecode,
                    bytecode_idx: 0,
                    stack_start,
                };
                let previous_stack_frame =
                    std::mem::replace(&mut self.stack_frame, new_stack_frame);
                self.previous_stack_frames.push(previous_stack_frame);
                Ok(())
            }
            _ => Err(VmError::TypeError {
                context: "function invocation",
                expected: UnsafeVal::FUNCTION_TYPE_NAME,
                actual: func_val.type_name(),
                value: func_val.formatted(self).to_string(),
            }),
        }
    }

    fn execute_call_stack_limit_reached(&mut self) -> VmError {
        let mut call_stack = Vec::with_capacity(1 + self.previous_stack_frames.len());
        call_stack.push(self.stack_frame.bytecode.name.clone());
        call_stack.extend(
            self.previous_stack_frames
                .iter()
                .rev()
                .map(|sf| sf.bytecode.name.clone()),
        );
        VmError::MaximumRecursionDepth {
            call_stack,
            max_depth: self.previous_stack_frames.len(),
        }
    }

    /// Execute returning from the current stack frame.
    fn execute_return(&mut self) -> Option<UnsafeVal> {
        // 1. Return the current value to the top of the stack.
        let ret_val: UnsafeVal = if self.stack_frame.stack_start < self.stack.len() {
            // Unwrap OK: The above statement is never true when len == 0.
            self.stack.pop().unwrap()
        } else {
            ().into()
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

impl Drop for Vm {
    fn drop(&mut self) {
        info!(
            "Dropping Spore VM, final GC stats: {gc_stats:#?}",
            gc_stats = self.objects.stats()
        );
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
                expected: UnsafeVal::INT_TYPE_NAME,
                actual: UnsafeVal::BOOL_TYPE_NAME,
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
        let got = vm.eval_str("(if false (+ 1 2))").unwrap();
        assert!(got.is_void(), "{got}");
    }

    #[test]
    fn if_statement_with_truthy_predicate_true_branch() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("(if 1 (+ 1 2) (+ 3 4))").unwrap().as_int(),
            Some(3)
        );
        assert_eq!(vm.eval_str("(if 1 (+ 1 2))").unwrap().as_int(), Some(3));
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
                    .map(Into::into)
                    .collect(),
            }
        );
    }

    #[test]
    fn aggressive_inline_produces_same_results_when_there_are_no_redefinitions() {
        let mut aggressive_inline_vm = Vm::new(VmSettings {
            enable_aggressive_inline: true,
        });
        let mut default_vm = Vm::new(VmSettings {
            enable_aggressive_inline: false,
        });
        let srcs = ["(define x 12)", "x", "(+ x x)"];
        for src in srcs {
            assert_eq!(
                aggressive_inline_vm.eval_str(src).unwrap().to_string(),
                default_vm.eval_str(src).unwrap().to_string(),
            )
        }
    }
}
