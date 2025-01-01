use bumpalo::Bump;

use crate::{
    instruction::Instruction,
    module::Module,
    object_store::TypedObjectStore,
    val::{
        functions::{ByteCodeFunction, NativeFunction},
        symbol::SymbolTable,
        Val,
    },
};

pub struct Vm {
    globals: Module,
    stack: Vec<Val>,
    stack_frame: StackFrame,
    previous_stack_frames: Vec<StackFrame>,
    compile_arena: Bump,
    objects: Objects,
}

#[derive(Default)]
struct Objects {
    native_functions: TypedObjectStore<NativeFunction>,
    bytecode_functions: TypedObjectStore<ByteCodeFunction>,
    symbols: SymbolTable,
    null_bytecode: ByteCodeFunction,
}

struct StackFrame {
    stack_start: usize,
    bytecode_idx: usize,
    function: ByteCodeFunction,
}

impl Default for StackFrame {
    fn default() -> StackFrame {
        StackFrame {
            stack_start: 0,
            bytecode_idx: 0,
            function: ByteCodeFunction::default(),
        }
    }
}

impl Default for Vm {
    fn default() -> Self {
        let mut vm = Vm {
            globals: Module::new(),
            stack: Vec::with_capacity(4096),
            stack_frame: StackFrame {
                stack_start: 0,
                bytecode_idx: 0,
                function: ByteCodeFunction::default(),
            },
            previous_stack_frames: Vec::with_capacity(128),
            compile_arena: Bump::new(),
            objects: Objects::default(),
        };
        vm.register_function("+", plus_fn)
            .register_function_raw("define", define_fn);
        vm
    }
}

impl Vm {
    pub fn register_function(
        &mut self,
        name: &str,
        f: impl 'static + Fn(&[Val]) -> Val,
    ) -> &mut Self {
        self.register_native_function(name, NativeFunction::new(f))
    }

    pub fn register_function_raw(
        &mut self,
        name: &str,
        f: impl 'static + Fn(&mut Vm) -> Val,
    ) -> &mut Self {
        self.register_native_function(name, NativeFunction::from(f))
    }

    pub fn register_native_function(&mut self, name: &str, f: NativeFunction) -> &mut Self {
        let symbol = self.objects.symbols.symbol_id(name);
        assert!(
            !self.globals.values.contains_key(&symbol),
            "register_function called with existing function named {name}."
        );
        let id = self.objects.native_functions.register(f);
        self.globals.values.insert(symbol, Val::NativeFunction(id));
        self
    }

    pub fn args(&self) -> &[Val] {
        let start = self.stack_frame.stack_start;
        &self.stack[start..]
    }
}

impl Vm {
    pub fn eval_str(&mut self, s: &str) -> Val {
        let bytecode =
            ByteCodeFunction::with_str(&mut self.objects.symbols, &self.compile_arena, s);
        self.compile_arena.reset();
        self.eval(bytecode)
    }

    pub fn eval(&mut self, bytecode: ByteCodeFunction) -> Val {
        assert_eq!(bytecode.args, 0);
        let initial_stack_frames = self.previous_stack_frames.len();
        let previous_stack_frame = std::mem::replace(
            &mut self.stack_frame,
            StackFrame {
                stack_start: self.stack.len(),
                bytecode_idx: 0,
                function: bytecode,
            },
        );
        self.previous_stack_frames.push(previous_stack_frame);
        while self.previous_stack_frames.len() != initial_stack_frames {
            self.run_next();
        }
        self.stack.last().cloned().unwrap_or(Val::Void)
    }

    fn run_next(&mut self) {
        let bytecode_idx = self.stack_frame.bytecode_idx;
        self.stack_frame.bytecode_idx = bytecode_idx + 1;
        let instruction = self
            .stack_frame
            .function
            .instructions
            .get(bytecode_idx)
            .unwrap_or(&Instruction::Return);
        match instruction {
            Instruction::Push(v) => self.stack.push(v.clone()),
            Instruction::Eval(n) => self.execute_eval(*n),
            Instruction::Deref(symbol) => {
                let v = match self.globals.values.get(symbol) {
                    Some(v) => v.clone(),
                    None => {
                        todo!(
                            "symbol {symbol:?} not found",
                            symbol = self.objects.symbols.symbol_name(*symbol)
                        )
                    }
                };
                self.stack.push(v);
            }
            Instruction::Return => self.execute_return(),
        }
    }

    fn execute_return(&mut self) {
        let stack_start = self.stack_frame.stack_start;
        match self.previous_stack_frames.pop() {
            Some(sf) => self.stack_frame = sf,
            None => self.stack_frame.stack_start = 0,
        }
        self.stack_frame = self.previous_stack_frames.pop().unwrap_or_default();
        let return_value = if self.stack.len() >= stack_start {
            self.stack.last().unwrap().clone()
        } else {
            todo!()
        };
        self.stack.truncate(stack_start);
        match self.stack.last_mut() {
            Some(v) => *v = return_value,
            None => self.stack.push(return_value),
        }
    }

    fn execute_eval(&mut self, n: usize) {
        let function_idx = self.stack.len() - n;
        let stack_start = function_idx + 1;
        let function = self.stack[function_idx].clone();
        match function {
            Val::NativeFunction(native_function) => {
                let previous_stack_frame = std::mem::replace(
                    &mut self.stack_frame,
                    StackFrame {
                        stack_start,
                        bytecode_idx: 0,
                        function: self.objects.null_bytecode.clone(),
                    },
                );
                self.previous_stack_frames.push(previous_stack_frame);
                let function = self
                    .objects
                    .native_functions
                    .get(native_function)
                    .unwrap()
                    .clone();
                let ret = function.call(self);
                self.stack.truncate(stack_start);
                *self.stack.last_mut().unwrap() = ret;
                self.stack_frame = self.previous_stack_frames.pop().unwrap();
            }
            Val::BytecodeFunction(bytecode_function) => {
                let previous_stack_frame = std::mem::replace(
                    &mut self.stack_frame,
                    StackFrame {
                        stack_start,
                        bytecode_idx: 0,
                        function: self
                            .objects
                            .bytecode_functions
                            .get(bytecode_function)
                            .unwrap()
                            .clone(),
                    },
                );
                self.previous_stack_frames.push(previous_stack_frame);
            }
            v => todo!("{v:?} is not callable"),
        }
    }
}

fn plus_fn(args: &[Val]) -> Val {
    let mut int_sum = 0;
    let mut float_sum = 0.0;
    for arg in args {
        match arg {
            Val::Int(x) => int_sum += *x,
            Val::Float(x) => float_sum += *x,
            v => todo!("{v:?} not handled in + operator"),
        }
    }
    if float_sum == 0.0 {
        Val::Int(int_sum)
    } else {
        Val::Float(float_sum + int_sum as f64)
    }
}

fn define_fn(vm: &mut Vm) -> Val {
    let (sym, val) = match vm.args() {
        [Val::Symbol(sym), val] => (sym.clone(), val.clone()),
        _ => todo!(),
    };
    vm.globals.values.insert(sym, val);
    Val::Void
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn function_call() {
        assert_eq!(Vm::default().eval_str("(+ 1 2 3 4)"), Val::Int(10));
    }

    #[test]
    fn define() {
        let mut vm = Vm::default();
        assert_eq!(vm.eval_str("(define 'x 12)"), Val::Void);
        assert_eq!(vm.eval_str("x"), Val::Int(12));
    }
}
