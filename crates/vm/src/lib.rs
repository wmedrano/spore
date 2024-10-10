use std::{collections::HashMap, sync::atomic::AtomicU16};

use bumpalo::Bump;
use compact_str::CompactString;
use gc::{is_garbage_collected, MemoryManager};
use log::*;

use compiler::Compiler;
use error::{BacktraceError, VmError, VmResult};
pub use settings::Settings;
use stack_frame::{StackFrame, StackFrameManager};
use val::{
    custom::CustomVal, ByteCode, CustomType, Instruction, NativeFunction, NativeFunctionContext,
    ProtectedVal, Symbol, UnsafeVal, Val, ValId,
};

mod builtins;
mod compiler;
pub mod error;
mod gc;
pub mod parser;
pub mod repl;
mod settings;
mod stack_frame;
pub mod val;

type BumpVec<'a, T> = bumpalo::collections::Vec<'a, T>;

/// The GitHub issues page to file issues to.
pub const ISSUE_LINK: &str = "https://github.com/wmedrano/spore/issues";

/// The Spore virtual machine.
///
/// # Example
/// ```rust
/// let mut vm = spore_vm::Vm::default();
/// vm.eval_str("(define foo 42)").unwrap();
/// let foo = vm.val_by_name("foo").unwrap().try_int().unwrap(); // 42
/// vm.eval_str("(define (bar x) (+ x foo))").unwrap();
/// let bar_10 = vm
///     .eval_function_by_name("bar", std::iter::once(10.into()))
///     .unwrap()
///     .try_int()
///     .unwrap(); // 52
/// ```
#[derive(Debug)]
pub struct Vm {
    /// The data stack. This is used to store temporary values used for computation.
    stack: Vec<UnsafeVal>,
    /// Map from binding name to value. This is used to store global values.
    values: HashMap<Symbol, UnsafeVal>,
    /// The current stack frame. This contains what should be evaluated next and some extra context.
    stack_frames: StackFrameManager,
    /// Manages lifetime of all values, aside from simple atoms like bool/int/float.
    pub(crate) objects: MemoryManager,
    /// Contains bytecode compilation settings,
    settings: Settings,
    /// An arena for temporary computations for things like compilation and garbage collection.
    tmp_arena: Option<Bump>,
}

impl Default for Vm {
    /// Create a new virtual machine.
    fn default() -> Vm {
        Vm::new(Settings::default())
    }
}

// A unique (enough) identifier for a VM. Used to identify if a value was generated from the VM or
// not. The values start at 1 to ensure that the default (0) is not from a valid VM.
static VM_ID: AtomicU16 = AtomicU16::new(1);

impl Vm {
    /// Create a new virtual machine.
    pub fn new(settings: Settings) -> Vm {
        let start_t = std::time::Instant::now();
        let mut vm_id = 0;
        while vm_id == 0 {
            vm_id = VM_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        let mut vm = Vm {
            // TODO: Determine optimal size for stack. Small values may perform, better, but
            // exceeding the capacity may cause performance degregations.
            stack: Vec::with_capacity(4096),
            values: HashMap::new(),
            stack_frames: StackFrameManager::default(),
            objects: MemoryManager::new(vm_id),
            settings,
            tmp_arena: Some(Bump::new()),
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

    /// Return the VM with the native function registered.
    pub fn with_native_function(mut self, name: &str, func: NativeFunction) -> Self {
        let func: UnsafeVal = func.into();
        assert!(!is_garbage_collected(func));
        // Unsafe OK: Native functions do not need to register with the vm.
        unsafe { self.register_value(name, func) };
        self
    }

    /// Return the VM with a custom value that is accessible globally.
    ///
    /// # Example
    ///
    /// ```rust
    /// #[derive(Debug, Default)]
    /// pub struct MyType(i64);
    /// impl spore_vm::val::CustomType for MyType {}
    /// impl std::fmt::Display for MyType {
    ///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    ///         write!(f, "my number is {}", self.0)
    ///     }
    /// }
    ///
    /// let mut vm = spore_vm::Vm::default()
    ///     .with_custom_value("my_value", MyType(10));
    /// let val = vm.val_by_name("my_value").unwrap();
    /// let mut custom_val = val.try_custom_mut::<MyType>(&vm).unwrap();
    /// custom_val.0 = 100;
    /// ```
    pub fn with_custom_value(mut self, name: &str, val: impl CustomType) -> Self {
        let id = self.objects.insert_custom(CustomVal::new(val));
        // Unsafe OK: Custom type is registered in the VM in the line above.
        unsafe { self.register_value(name, id) };
        self
    }

    /// Return the VM returned by calling `fn`.
    pub fn with(self, f: impl Fn(Vm) -> Vm) -> Self {
        f(self)
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
        let interned_sym = self.get_or_create_symbol(name);
        self.values.insert(interned_sym, val);
    }
}

impl Vm {
    /// Get the value with the given name or [None] if it does not exist.
    pub fn val_by_name(&self, name: &str) -> Option<Val> {
        let interned_name = self.get_symbol(name)?;
        self.values
            .get(&interned_name)
            .copied()
            // Unsafe OK: The value has not been garbage collected as its part of the values map.
            .map(|v| unsafe { Val::from_unsafe_val(v) })
    }

    /// Evaluate a string in the virtual machine.
    ///
    /// ```rust
    /// let mut vm = spore_vm::Vm::default();
    /// let x = vm.eval_str("(+ 20 22)").unwrap().try_int().unwrap();
    /// ```
    pub fn eval_str(&mut self, source: &str) -> VmResult<ProtectedVal> {
        let bytecode = {
            let mut arena = self.tmp_arena.take().unwrap_or_else(|| {
                warn!("Arena was unexpectedly unavailable. Please file an issue at {ISSUE_LINK} with proper context.");
                Bump::new()
            });
            arena.reset();
            let bytecode = Compiler::compile(self, source, &arena)?;
            self.tmp_arena = Some(arena);
            bytecode
        };

        let bytecode_id = self.objects.insert_bytecode(bytecode);
        let bytecode = self.objects.get_bytecode(bytecode_id).unwrap();

        self.stack.clear();
        self.stack
            .extend(std::iter::repeat(UnsafeVal::Void).take(bytecode.local_bindings));
        self.stack_frames
            .reset_with_stack_frame(StackFrame::new(bytecode_id, bytecode, 0));
        unsafe { self.run_gc() };
        self.run_all_protected()
    }

    /// Call a function with the given name.
    ///
    /// ```rust
    /// let mut vm = spore_vm::Vm::default();
    /// vm.eval_str("(define (fib n) (if (< n 2) n (+ (fib (+ n -1)) (fib (+ n -2)))))")
    ///     .unwrap();
    /// let ans = vm
    ///     .eval_function_by_name("fib", std::iter::once(10.into()))
    ///     .unwrap()
    ///     .try_int()
    ///     .unwrap();
    /// ```
    pub fn eval_function_by_name(
        &mut self,
        name: &str,
        args: impl ExactSizeIterator<Item = Val<'static>>,
    ) -> VmResult<ProtectedVal> {
        let symbol_not_defined_err = || VmError::SymbolNotDefined {
            src: None,
            symbol: name.to_string(),
        };
        let interned_name = self.get_symbol(name).ok_or_else(symbol_not_defined_err)?;
        let function_val = self
            .values
            .get(&interned_name)
            .copied()
            .ok_or_else(symbol_not_defined_err)?;
        self.stack_frames.reset();
        self.stack.clear();
        self.stack.push(function_val);
        self.stack.extend(args.map(|arg| arg.as_unsafe_val()));
        unsafe { self.run_gc() };
        let stack_len = self.stack.len();
        self.execute_eval(stack_len)?;
        self.run_all_protected()
    }

    fn annotate_src(&self, error: VmError) -> VmError {
        for stack_frame in self.stack_frames.iter() {
            if let Some(src) = stack_frame.previous_instruction_source(self) {
                return error.with_src(src);
            }
        }
        error
    }

    /// Runs the virtual machine until either:
    ///   1. Completion, there is nothing left to run.
    ///   2. An error has occurred.
    ///   3. An FFI stack frame has been reached.
    pub(crate) fn run_all(&mut self) -> VmResult<UnsafeVal> {
        loop {
            if let Some(v) = self.run_next().map_err(|err| self.annotate_src(err))? {
                return Ok(v);
            }
        }
    }

    /// Similar to [Self::run_all] but the returned value is protected from garbage collection.
    pub(crate) fn run_all_protected(&mut self) -> VmResult<ProtectedVal> {
        let v = self.run_all()?;
        Ok(ProtectedVal::new(self, unsafe { Val::from_unsafe_val(v) }))
    }

    /// Run the next instruction in the virtual machine.
    ///
    /// If there are no more instructions to run, then `Some(return_value)` will be
    /// returned. Otherwise, `None` will be returned.
    fn run_next(&mut self) -> VmResult<Option<UnsafeVal>> {
        let instruction = self
            .stack_frames
            .current
            .instructions
            .get(self.stack_frames.current.instruction_idx)
            .unwrap_or(&Instruction::Return);
        self.stack_frames.current.instruction_idx += 1;
        match instruction {
            Instruction::PushConst(c) => self.stack.push(*c),
            Instruction::PushCurrentFunction => {
                let f = UnsafeVal::ByteCodeFunction(self.stack_frames.current.bytecode_id);
                self.stack.push(f);
            }
            Instruction::Pop(n) => {
                let start = self.stack.len() - n;
                self.stack.drain(start..);
            }
            Instruction::GetArg(n) => {
                let val = self.stack[self.stack_frames.current.stack_start + *n];
                self.stack.push(val);
            }
            Instruction::BindArg(n) => {
                let val = self.stack.pop().unwrap();
                self.stack[self.stack_frames.current.stack_start + *n] = val;
            }
            Instruction::Deref(symbol) => {
                let v = match self.values.get(symbol) {
                    Some(v) => *v,
                    None => {
                        return Err(VmError::SymbolNotDefined {
                            src: None,
                            symbol: self
                                .symbol_to_str(*symbol)
                                .unwrap_or("*symbol-not-registered*")
                                .to_string(),
                        });
                    }
                };
                self.stack.push(v);
            }
            Instruction::Define(symbol) => {
                let v = self.stack.pop().ok_or_else(BacktraceError::capture)?;
                self.values.insert(*symbol, v);
            }
            Instruction::Eval(n) => {
                self.execute_eval(*n)?;
            }
            Instruction::EvalNative { func, arg_count } => {
                self.execute_eval_native(*func, *arg_count)?;
            }
            Instruction::JumpIf(n) => {
                if self.stack.pop().unwrap().is_truthy() {
                    self.stack_frames.current.instruction_idx += *n;
                }
            }
            Instruction::Jump(n) => {
                self.stack_frames.current.instruction_idx += *n;
            }
            Instruction::Return => return Ok(self.execute_return()),
        }
        Ok(None)
    }

    fn execute_eval_native(&mut self, func: NativeFunction, arg_count: usize) -> VmResult<()> {
        let stack_start = self.stack.len() - arg_count;
        self.stack_frames.push(StackFrame::new(
            Default::default(),
            &Default::default(),
            stack_start,
        ));
        let args = unsafe {
            let slice = std::slice::from_raw_parts(self.stack.as_ptr().add(stack_start), arg_count);
            Val::from_unsafe_val_slice(slice)
        };
        let builder = func(NativeFunctionContext::new(self), args)?;
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
        self.stack_frames.pop();
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
                let args = unsafe {
                    let slice =
                        std::slice::from_raw_parts(self.stack.as_ptr().add(stack_start), n - 1);
                    Val::from_unsafe_val_slice(slice)
                };
                self.stack_frames.push(StackFrame::new(
                    Default::default(),
                    &Default::default(),
                    stack_start,
                ));
                let builder = func(NativeFunctionContext::new(self), args)?;
                // Unsafe OK: Value is inserted into VM immediately.
                let v = unsafe { builder.build() };
                self.stack[function_idx] = v;
                self.stack.truncate(stack_start);
                self.stack_frames.pop();
                Ok(())
            }
            UnsafeVal::ByteCodeFunction(bytecode_id) => {
                let bytecode = {
                    let bytecode = self.objects.get_bytecode(bytecode_id).unwrap();
                    let arg_count = n - 1;
                    if bytecode.arg_count != arg_count {
                        return Err(VmError::ArityError {
                            function: bytecode.name.clone(),
                            expected: bytecode.arg_count,
                            actual: arg_count,
                        });
                    }
                    if self.stack_frames.at_capacity() {
                        return Err(self.execute_call_stack_limit_reached());
                    }
                    bytecode
                };
                self.stack
                    .extend(std::iter::repeat(UnsafeVal::Void).take(bytecode.local_bindings));
                self.stack_frames
                    .push(StackFrame::new(bytecode_id, bytecode, stack_start));
                Ok(())
            }
            _ => Err(VmError::TypeError {
                src: None,
                context: "function invocation",
                expected: UnsafeVal::FUNCTION_TYPE_NAME,
                actual: func_val.type_name(),
                value: func_val.formatted(self).to_string(),
            }),
        }
    }

    pub fn stack_trace(&self) -> Vec<CompactString> {
        let depth = self.stack_frames.stack_trace_depth();
        let mut call_stack = Vec::with_capacity(depth);
        for stack_frame in self.stack_frames.iter() {
            if stack_frame.has_valid_function_call() {
                call_stack.push(stack_frame.bytecode(self).name.clone());
            } else {
                call_stack.push("native-call".into());
            }
        }
        call_stack
    }

    fn execute_call_stack_limit_reached(&mut self) -> VmError {
        let call_stack = self.stack_trace();
        let max_depth = call_stack.len();
        VmError::MaximumFunctionCallDepth {
            call_stack,
            max_depth,
        }
    }

    /// Execute returning from the current stack frame.
    fn execute_return(&mut self) -> Option<UnsafeVal> {
        // 1. Return the current value to the top of the stack.
        let ret_val: UnsafeVal = if self.stack_frames.current.stack_start < self.stack.len() {
            // Unwrap OK: The above statement is never true when len == 0.
            self.stack.pop().unwrap()
        } else {
            ().into()
        };
        // 2. Set up the next continuation.
        match self.stack_frames.previous.pop() {
            // 2a. Pop the stack frame and replace the top value in the stack with the return value.
            Some(c) if c.has_valid_function_call() => {
                self.stack.truncate(self.stack_frames.current.stack_start);
                match self.stack.last_mut() {
                    Some(v) => *v = ret_val,
                    None => unreachable!(),
                }
                self.stack_frames.current = c;
                None
            }
            // 2b. The previous frame is an ffi boundary. Return the value back to the ffi layer.
            Some(c) => {
                self.stack.truncate(self.stack_frames.current.stack_start);
                self.stack_frames.current = c;
                Some(ret_val)
            }
            // 2c. There is nothing to continue to so return the value.
            None => {
                self.stack.truncate(0);
                std::mem::take(&mut self.stack_frames.current);
                Some(ret_val)
            }
        }
    }
}

impl Vm {
    /// Run the garbage collector.
    ///
    /// This does not need to be manually invoked as it is called automatically at the start of
    /// evaluation through functions like [Self::eval_str] and [Self::eval_function_by_name].
    ///
    /// # Safety
    ///
    pub unsafe fn run_gc(&mut self) {
        let is_gc = |v: &UnsafeVal| is_garbage_collected(*v);
        let mut arena = self.tmp_arena.take().unwrap_or_else(|| {
            warn!("Arena was unexpectedly unavailable. Please file an issue at {ISSUE_LINK} with proper context.");
            Bump::new()
        });
        {
            let mut bytecodes: BumpVec<(ValId<_>, ByteCode)> = BumpVec::new_in(&arena);
            for stack_frame in self.stack_frames.iter() {
                if stack_frame.has_valid_function_call() {
                    bytecodes.push((stack_frame.bytecode_id, stack_frame.bytecode(self).clone()));
                }
            }
            let vals = self
                .stack
                .iter()
                .copied()
                .filter(is_gc)
                .chain(self.values.values().copied().filter(is_gc))
                .chain(bytecodes.iter().flat_map(|(id, bytecode)| {
                    bytecode
                        .values()
                        .filter(is_gc)
                        .chain(std::iter::once((*id).into()))
                }));
            self.objects.run_gc(&arena, vals);
        }
        arena.reset();
        self.tmp_arena = Some(arena);
    }
}

impl Vm {
    /// Get the symbol for the given `s`, or `None` if it does not exist within the VM.
    pub fn get_symbol(&self, s: &str) -> Option<Symbol> {
        self.objects.get_symbol(s)
    }

    /// Get the given symbol within the VM or create it if it does not exist.
    pub fn get_or_create_symbol(&mut self, s: &str) -> Symbol {
        self.objects.get_or_create_symbol(s)
    }

    /// Get the `str` representation for a symbol.
    pub fn symbol_to_str(&self, s: Symbol) -> Option<&str> {
        self.objects.symbol_to_str(s)
    }
}

impl Drop for Vm {
    fn drop(&mut self) {
        info!("Dropping Spore VM.");
    }
}

#[cfg(test)]
mod tests {
    use error::CompileError;
    use parser::span::Span;

    use super::*;

    #[test]
    fn constant_expression_evaluates_to_constant() {
        let mut vm = Vm::default();
        let actual = vm.eval_str("42").unwrap();
        assert_eq!(actual.try_int().unwrap(), 42);
    }

    #[test]
    fn expression_can_evaluate() {
        let mut vm = Vm::default();
        let actual = vm.eval_str("(+ 1 2 3 4.0)").unwrap();
        assert_eq!(actual.try_float().unwrap(), 10.0);
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
        let src = "(+ true false)";
        let actual = vm.eval_str(src).unwrap_err();
        assert_eq!(
            actual,
            VmError::TypeError {
                src: Some(Span::new(0, 14).with_src(src.into())),
                context: "+",
                expected: "int or float",
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
            VmError::CompileError(CompileError::DefineNotAllowed)
        );
    }

    #[test]
    fn defined_variable_can_be_referenced() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("(define x 12) (+ x x)")
                .unwrap()
                .try_int()
                .unwrap(),
            24
        );
        assert_eq!(vm.eval_str("(+ x 10)").unwrap().try_int().unwrap(), 22);
    }

    #[test]
    fn if_statement_can_return_any_of() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("(if true (+ 1 2))").unwrap().try_int().unwrap(),
            3
        );
        assert_eq!(
            vm.eval_str("(if true (+ 1 2) (+ 3 4))")
                .unwrap()
                .try_int()
                .unwrap(),
            3
        );
        assert_eq!(
            vm.eval_str("(if false (+ 1 2) (+ 3 4))")
                .unwrap()
                .try_int()
                .unwrap(),
            7
        );
        let got = vm.eval_str("(if false (+ 1 2))").unwrap();
        assert!(got.is_void(), "{got}");
    }

    #[test]
    fn if_statement_with_truthy_predicate_true_branch() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("(if 1 (+ 1 2) (+ 3 4))")
                .unwrap()
                .try_int()
                .unwrap(),
            3
        );
        assert_eq!(vm.eval_str("(if 1 (+ 1 2))").unwrap().try_int().unwrap(), 3);
    }

    #[test]
    fn lambda_can_be_evaluated() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("((lambda () 7))").unwrap().try_int().unwrap(),
            7
        );
        assert_eq!(
            vm.eval_str("((lambda () (+ 1 2 3)))")
                .unwrap()
                .try_int()
                .unwrap(),
            6
        );
    }

    #[test]
    fn lambda_with_args_can_be_evaluated() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("((lambda (a b) 4) 1 2)")
                .unwrap()
                .try_int()
                .unwrap(),
            4,
        );
        assert_eq!(
            vm.eval_str("((lambda (a b) (+ a b)) 1 2)")
                .unwrap()
                .try_int()
                .unwrap(),
            3
        );
    }

    #[test]
    fn function_called_with_wrong_number_of_args_returns_error() {
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
        let mut got = vm
            .eval_str("(define (takes-two-args arg1 arg2) (+ arg1 arg2))")
            .unwrap();
        assert!(got.is_void(), "{got}");
        let (vm, _) = got.split();
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
    fn can_get_val_by_name() {
        let mut vm = Vm::default();
        vm.eval_str("(define one 1) (define two 2)").unwrap();
        assert_eq!(vm.val_by_name("one").unwrap().try_int().unwrap(), 1);
        assert_eq!(vm.val_by_name("two").unwrap().try_int().unwrap(), 2);
    }

    #[test]
    fn getting_val_that_does_not_exist_returns_err() {
        let mut vm = Vm::default();
        vm.eval_str("(define one 1) (define two 2)").unwrap();
        assert!(vm.val_by_name("three").is_none());
    }

    #[test]
    fn can_eval_by_function_with_native_function() {
        let mut vm = Vm::default();
        let ans = vm
            .eval_function_by_name("+", [10.into(), 5.into()].into_iter())
            .unwrap()
            .try_int()
            .unwrap();
        assert_eq!(ans, 15);
    }

    #[test]
    fn eval_function_that_does_not_exist_returns_error() {
        let mut vm = Vm::default();
        vm.eval_str("(define (foo) 1)").unwrap();
        assert_eq!(
            vm.eval_function_by_name("bar", std::iter::empty())
                .unwrap_err(),
            VmError::SymbolNotDefined {
                src: None,
                symbol: "bar".into()
            },
        );
    }

    #[test]
    fn eval_function_that_is_not_function_returns_error() {
        let mut vm = Vm::default();
        vm.eval_str("(define foo 100)").unwrap();
        assert_eq!(
            vm.eval_function_by_name("foo", std::iter::empty())
                .unwrap_err(),
            VmError::TypeError {
                src: None,
                context: "function invocation",
                expected: UnsafeVal::FUNCTION_TYPE_NAME,
                actual: UnsafeVal::INT_TYPE_NAME,
                value: "100".into(),
            }
        );
    }

    #[test]
    fn can_call_function_recursively() {
        let mut vm = Vm::default();
        vm.eval_str("(define (fib n) (if (< n 2) n (+ (fib (+ n -1)) (fib (+ n -2)))))")
            .unwrap();
        let ans = vm
            .eval_function_by_name("fib", std::iter::once(10.into()))
            .unwrap()
            .try_int()
            .unwrap();
        assert_eq!(ans, 55);
    }

    #[test]
    fn infinite_recursion_halts() {
        let mut vm = Vm::default();
        assert!(vm
            .eval_str("(define (recurse) (recurse))")
            .unwrap()
            .is_void());
        assert_eq!(
            vm.eval_str("(recurse)").unwrap_err(),
            VmError::MaximumFunctionCallDepth {
                max_depth: 65,
                call_stack: std::iter::once("")
                    .chain(std::iter::repeat("recurse").take(64))
                    .map(Into::into)
                    .collect(),
            }
        );
    }

    #[test]
    fn aggressive_inline_returns_same_results_when_there_are_no_redefinitions() {
        let mut aggressive_inline_vm = Vm::new(Settings {
            enable_aggressive_inline: true,
            enable_source_maps: false,
        });
        let mut default_vm = Vm::new(Settings {
            enable_aggressive_inline: false,
            enable_source_maps: true,
        });
        let srcs = ["(define x 12)", "x", "(+ x x)"];
        for src in srcs {
            assert_eq!(
                aggressive_inline_vm.eval_str(src).unwrap().to_string(),
                default_vm.eval_str(src).unwrap().to_string(),
            )
        }
    }

    #[test]
    fn let_statement() {
        let mut vm = Vm::default();
        assert_eq!(
            vm.eval_str("(let ([x 10] [y 20] [z (+ x y)]) (+ x y z))")
                .unwrap()
                .try_int()
                .unwrap(),
            60
        );
    }

    #[test]
    fn when_multiple_bindings_exist_last_one_is_used() {
        let mut vm = Vm::default();
        let src = r#"
(let ([x 1])
  (let ([x 2]
        [x (+ x x)])
    x))
"#;
        assert_eq!(vm.eval_str(src).unwrap().try_int().unwrap(), 4);
    }

    #[test]
    fn multiple_bindings_dont_affect_previous_binding_when_out_of_scope() {
        let mut vm = Vm::default();
        let src = r#"
(let ([x 1])
  (let ([x 2]
        [x (+ x x)])
    x)
x)
"#;
        assert_eq!(vm.eval_str(src).unwrap().try_int().unwrap(), 1);
    }

    #[test]
    fn local_bindings_take_precedence_over_arguments() {
        let mut vm = Vm::default();
        let src = r#"
(define (foo x)
  (let ([old-x x]
        [x     10])
    (+ old-x x)))

(foo 100)
"#;
        assert_eq!(vm.eval_str(src).unwrap().try_int().unwrap(), 110);
    }

    #[test]
    fn empty_or_returns_false() {
        let mut vm = Vm::default();
        let src = "(or)";
        assert!(!vm.eval_str(src).unwrap().try_bool().unwrap());
    }

    #[test]
    fn or_with_true_returns_true() {
        let mut vm = Vm::default();
        let src = "(or false false true false)";
        assert!(vm.eval_str(src).unwrap().try_bool().unwrap());
    }

    #[test]
    fn or_with_truthy_values_returns_first_truthy_value() {
        let mut vm = Vm::default();
        let src = "(or false false 5 4 3 2)";
        assert_eq!(vm.eval_str(src).unwrap().try_int().unwrap(), 5);
    }

    #[test]
    fn or_with_all_false_or_void_returns_last_arg() {
        let mut vm = Vm::default();
        assert!(vm
            .eval_str("(or void false void false void)")
            .unwrap()
            .is_void());
        assert!(!vm
            .eval_str("(or void false void false void false)")
            .unwrap()
            .try_bool()
            .unwrap());
    }

    #[test]
    fn and_with_no_args_returns_true() {
        let mut vm = Vm::default();
        let src = "(and)";
        assert!(vm.eval_str(src).unwrap().try_bool().unwrap());
    }

    #[test]
    fn and_with_all_truthy_args_returns_last_arg() {
        let mut vm = Vm::default();
        let src = "(and 1 2 3 4)";
        assert_eq!(vm.eval_str(src).unwrap().try_int().unwrap(), 4);
    }

    #[test]
    fn and_with_false_arg_returns_first_false_arg() {
        let mut vm = Vm::default();
        assert!(!vm
            .eval_str("(and 1 2 false 3 4)")
            .unwrap()
            .try_bool()
            .unwrap());
        assert!(vm.eval_str("(and 1 2 void 3 4)").unwrap().is_void());
    }
}
