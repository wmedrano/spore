use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use anyhow::{bail, Result};

use super::{
    bytecode::OpCode,
    types::{Symbol, Val},
};

/// An environment to evaluate bytecode on.
pub struct Environment {
    /// The registry of global values.
    pub globals: Arc<Mutex<HashMap<Symbol, Val>>>,
    /// The processing stack.
    pub stack: Vec<Val>,
    /// An environment for the current stack frame. This usually corresponds to the current
    /// procedure call.
    pub local: LocalEnvironment,
}

pub struct LocalEnvironment {
    /// The base index for the current call. Anything at this index or above is either an argument
    /// or a local variable to the current call.
    pub stack_base: usize,
}

impl Environment {
    pub fn reset_locals(&mut self) {
        self.stack.clear();
        self.local.stack_base = 0;
    }

    pub fn set_local_environment(&mut self, e: LocalEnvironment) -> LocalEnvironment {
        let mut e = e;
        std::mem::swap(&mut e, &mut self.local);
        e
    }

    pub fn pop(&mut self) -> Option<Val> {
        self.stack.pop()
    }

    /// Get a value from the current environment.
    pub fn get_value(&self, sym: impl AsRef<str>) -> Option<Val> {
        let registry = self.globals.lock().unwrap();
        registry.get(sym.as_ref()).cloned()
    }

    /// Evaluate a sequence of bytecode.
    pub fn eval_bytecode(&mut self, bc: &[OpCode]) -> Result<()> {
        let mut iter = bc.iter();
        while let Some(bc) = iter.next() {
            match bc {
                OpCode::PushVal(v) => self.push_value(v),
                OpCode::Eval(n) => self.eval_n(*n)?,
                OpCode::JumpIf(n) => self.jump_if(*n, &mut iter)?,
                OpCode::Jump(n) => self.jump(*n, &mut iter),
                OpCode::GetSym(s) => self.get_sym(s.as_str())?,
                OpCode::GetArg(n) => self.get_arg(n),
            }
        }
        Ok(())
    }

    fn get_arg(&mut self, n: &usize) {
        let v = self.stack.get(self.local.stack_base + n).unwrap().clone();
        self.stack.push(v);
    }

    fn jump_if<'a>(
        &mut self,
        n: usize,
        bc_iter: &mut impl Iterator<Item = &'a OpCode>,
    ) -> Result<()> {
        match self.stack.pop() {
            Some(v) => {
                if v.is_truthy()? {
                    self.jump(n, bc_iter);
                }
            }
            None => bail!("bytecode if found no value to evaluate if statement"),
        }
        Ok(())
    }

    fn jump<'b>(&self, n: usize, bc_iter: &mut impl Iterator<Item = &'b OpCode>) {
        bc_iter.nth(n - 1);
    }

    fn push_value(&mut self, v: &Val) {
        self.stack.push(v.clone());
    }

    fn eval_n(&mut self, n: usize) -> Result<()> {
        let proc_idx = self.stack.len() - n;
        let proc = match self.stack.get(proc_idx) {
            None => bail!("eval_top must have at least one value on the stack"),
            Some(Val::Proc(p)) => p.clone(),
            Some(v) => bail!("value {v} is not a valid procedure."),
        };
        let current_locals = self.set_local_environment(LocalEnvironment {
            stack_base: proc_idx + 1,
        });
        let result = proc.eval(self)?;
        self.stack[proc_idx] = result;
        self.stack.truncate(self.local.stack_base);
        self.set_local_environment(current_locals);
        Ok(())
    }

    fn get_sym(&mut self, s: &str) -> Result<()> {
        match self.get_value(s) {
            Some(v) => self.stack.push(v.clone()),
            None => bail!("{s} is not defined"),
        }
        Ok(())
    }
}
