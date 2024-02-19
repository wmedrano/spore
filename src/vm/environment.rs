use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use anyhow::{bail, Result};

use super::types::{instruction::Instruction, proc::Procedure, symbol::Symbol, Val};

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
    /// Evaluate a sequence of bytecode.
    pub fn eval_bytecode(&mut self, bc: &[Instruction]) -> Result<()> {
        let mut iter = bc.iter();
        while let Some(bc) = iter.next() {
            match bc {
                Instruction::PushVal(v) => self.execute_push_val(v),
                Instruction::Eval(n) => self.execute_eval_n(*n)?,
                Instruction::JumpIf(n) => self.execute_jump_if(*n, &mut iter)?,
                Instruction::Jump(n) => self.execute_jump(*n, &mut iter),
                Instruction::GetSym(s) => self.execute_get_sym(s.as_str())?,
                Instruction::GetArg(n) => self.execute_get_arg(n),
            }
        }
        Ok(())
    }

    /// Resets the environment. This clears the stack but leaves global definitions in tact.
    pub fn reset_locals(&mut self) {
        self.stack.clear();
        self.local.stack_base = 0;
    }

    /// Set the local environment and return the old local environment.
    pub fn set_local_environment(&mut self, e: LocalEnvironment) -> LocalEnvironment {
        let mut e = e;
        std::mem::swap(&mut e, &mut self.local);
        e
    }

    /// Get the local stack. The local stack contains the arguments and values for the current
    /// procedure call.
    pub fn local_stack(&self) -> &[Val] {
        &self.stack[self.local.stack_base..]
    }

    /// Pop the top value of the local stack. If there are no values on the local stack, then `None`
    /// is returned.
    pub fn pop_local(&mut self) -> Option<Val> {
        if self.local.stack_base < self.stack.len() {
            self.stack.pop()
        } else {
            None
        }
    }

    /// Get a value from the current environment.
    pub fn get_value(&self, sym: impl AsRef<str>) -> Option<Val> {
        let registry = self.globals.lock().unwrap();
        registry.get(sym.as_ref()).cloned()
    }

    fn execute_get_arg(&mut self, n: &usize) {
        let v = self.stack.get(self.local.stack_base + n).unwrap().clone();
        self.stack.push(v);
    }

    fn execute_jump_if<'a>(
        &mut self,
        n: usize,
        bc_iter: &mut impl Iterator<Item = &'a Instruction>,
    ) -> Result<()> {
        match self.stack.pop() {
            Some(v) => {
                if v.is_truthy()? {
                    self.execute_jump(n, bc_iter);
                }
            }
            None => bail!("bytecode if found no value to evaluate if statement"),
        }
        Ok(())
    }

    fn execute_jump<'b>(&self, n: usize, bc_iter: &mut impl Iterator<Item = &'b Instruction>) {
        bc_iter.nth(n - 1);
    }

    fn execute_push_val(&mut self, v: &Val) {
        self.stack.push(v.clone());
    }

    fn execute_eval_n(&mut self, n: usize) -> Result<()> {
        let proc_idx = self.stack.len() - n;
        let proc = match self.stack.get(proc_idx) {
            None => bail!("eval_top must have at least one value on the stack"),
            Some(Val::Proc(p)) => p.clone(),
            Some(v) => bail!("value {v} is not a valid procedure."),
        };
        let current_locals = self.set_local_environment(LocalEnvironment {
            stack_base: proc_idx + 1,
        });
        let result = match proc.as_ref() {
            Procedure::Native(_, proc) => proc(self.local_stack())?,
            Procedure::ByteCode(proc) => {
                self.eval_bytecode(proc.instructions())?;
                self.pop_local().unwrap_or(Val::Void)
            }
        };
        self.stack[proc_idx] = result;
        self.stack.truncate(self.local.stack_base);
        self.set_local_environment(current_locals);
        Ok(())
    }

    fn execute_get_sym(&mut self, s: &str) -> Result<()> {
        match self.get_value(s) {
            Some(v) => self.stack.push(v.clone()),
            None => bail!("{s} is not defined"),
        }
        Ok(())
    }
}
