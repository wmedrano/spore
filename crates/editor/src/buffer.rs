use std::{any::Any, sync::RwLock};

use compact_str::{CompactString, ToCompactString};
use crop::Rope;
use spore_vm::{
    error::{VmError, VmResult},
    val::{CustomType, NativeFunctionContext, UnsafeVal, ValBuilder},
    Vm,
};

impl SporeBuffer {
    pub fn register(vm: Vm) -> Vm {
        vm.with_native_function("new-buffer", new_buffer)
            .with_native_function("buffer-append!", buffer_append)
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct SporeBuffer(pub RwLock<SporeBufferImpl>);

#[derive(Debug, Default)]
pub struct SporeBufferImpl {
    pub name: CompactString,
    pub contents: Rope,
}

fn new_buffer(mut ctx: NativeFunctionContext) -> VmResult<ValBuilder> {
    let args_len = ctx.args_len();
    if args_len > 2 {
        return Err(VmError::ArityError {
            function: "new-buffer".into(),
            expected: 2,
            actual: args_len,
        });
    }
    let mut inner = SporeBufferImpl::default();
    if args_len >= 1 {
        let v = ctx.arg(0);
        match v.as_str() {
            Some(s) => inner.name = s.into(),
            None => {
                return Err(VmError::TypeError {
                    context: "new-buffer",
                    expected: UnsafeVal::STRING_TYPE_NAME,
                    actual: v.type_name(),
                    value: v.to_string(),
                })
            }
        }
    }
    if args_len >= 2 {
        let v = ctx.arg(1);
        match v.as_str() {
            Some(s) => inner.contents.insert(0, s),
            None => {
                return Err(VmError::TypeError {
                    context: "new-buffer",
                    expected: UnsafeVal::STRING_TYPE_NAME,
                    actual: v.type_name(),
                    value: v.to_string(),
                })
            }
        }
    }
    Ok(ctx.new_custom(SporeBuffer(inner.into())))
}

fn buffer_append(mut ctx: NativeFunctionContext) -> VmResult<ValBuilder> {
    if ctx.args_len() != 2 {
        return Err(VmError::ArityError {
            function: "buffer_append".into(),
            expected: 2,
            actual: ctx.args_len(),
        });
    }
    let s = ctx
        .arg(1)
        .as_str()
        .ok_or_else(|| VmError::TypeError {
            context: "buffer-append!",
            expected: UnsafeVal::STRING_TYPE_NAME,
            actual: "something else",
            value: "".to_string(),
        })?
        .to_compact_string();
    {
        let buffer_val = ctx.arg(0);
        let buffer = match buffer_val.as_custom::<SporeBuffer>() {
            Some(b) => b,
            None => {
                return Err(VmError::TypeError {
                    context: "buffer-append!",
                    expected: "custom value of type spore-buffer",
                    actual: "something else",
                    value: buffer_val.to_string(),
                })
            }
        };
        let mut buffer = buffer.0.try_write().unwrap();
        let len = buffer.contents.byte_len();
        buffer.contents.insert(len, &s);
    }
    Ok(ctx.new_void())
}

impl CustomType for SporeBuffer {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl std::fmt::Display for SporeBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.try_read().unwrap().contents.fmt(f)
    }
}
