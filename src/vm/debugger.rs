use std::rc::Rc;

use super::{
    environment::Environment,
    types::{proc::ByteCodeProc, Val},
};

pub trait Debugger {
    #[inline(always)]
    fn start_eval(&mut self, _env: &Environment, _proc: &Rc<ByteCodeProc>, _arg_count: usize) {}

    #[inline(always)]
    fn return_value(&mut self, _val: &Val) {}
}

impl Debugger for () {}

#[derive(Default)]
pub struct TraceDebugger {
    traces: Vec<TraceCall>,
}

struct TraceCall {
    proc: Rc<ByteCodeProc>,
    args: Vec<Val>,
    return_val: Option<Val>,
    depth: usize,
}

impl std::fmt::Display for TraceDebugger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut traces = self.traces.iter();
        if let Some(t) = traces.next() {
            write!(f, "{t}")?;
        }
        for t in traces {
            write!(f, "\n{t}")?;
        }
        Ok(())
    }
}

impl std::fmt::Display for TraceCall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for _ in 0..self.depth {
            write!(f, "  ")?;
        }
        write!(f, "({proc}", proc = self.proc)?;
        for arg in self.args.iter() {
            write!(f, " {arg}")?;
        }
        write!(f, ") => ")?;
        match self.return_val.as_ref() {
            Some(v) => write!(f, "{v}"),
            None => write!(f, "_"),
        }
    }
}

impl Debugger for TraceDebugger {
    fn start_eval(&mut self, env: &Environment, proc: &Rc<ByteCodeProc>, arg_count: usize) {
        let args = env.stack[env.stack.len() - arg_count..].to_vec();
        self.traces.push(TraceCall {
            proc: proc.clone(),
            args,
            return_val: None,
            depth: env.frames.len(),
        })
    }

    fn return_value(&mut self, val: &Val) {
        for t in self.traces.iter_mut().rev() {
            if t.return_val.is_none() {
                t.return_val = Some(val.clone());
                return;
            }
        }
    }
}
