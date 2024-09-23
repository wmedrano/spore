use std::any::Any;

use compact_str::CompactString;
use crop::Rope;
use spore_vm::{
    error::{VmError, VmResult},
    val::{CustomType, NativeFunctionContext, UnsafeVal, Val, ValBuilder},
    Vm,
};

impl SporeBuffer {
    pub fn register(vm: Vm) -> Vm {
        vm.with_native_function("new-buffer", new_buffer)
            .with_native_function("buffer-insert!", buffer_insert)
            .with_native_function("buffer-delete!", buffer_delete)
            .with_native_function("buffer-cursor-move!", buffer_cursor_move)
    }
}

#[derive(Debug, Default)]
pub struct SporeBuffer {
    pub name: CompactString,
    pub contents: Rope,
    pub cursor: i64,
}

impl SporeBuffer {
    pub fn cursor_offset(&self, offset: i64) -> i64 {
        (self.cursor + offset).clamp(0, self.contents.byte_len() as i64)
    }

    fn insert(&mut self, text: &str) {
        self.contents.insert(self.cursor as usize, text);
        self.cursor = self.cursor_offset(text.len() as i64);
    }

    fn delete(&mut self) {
        if self.cursor == 0 {
            return;
        }
        // TODO: Handle unicode and graphemes better.
        self.contents.replace(
            self.cursor_offset(-1) as usize..self.cursor_offset(0) as usize,
            "",
        );
        self.cursor = self.cursor_offset(-1);
    }
}

fn new_buffer(ctx: NativeFunctionContext) -> VmResult<ValBuilder> {
    let args_len = ctx.args_len();
    if args_len > 2 {
        return Err(VmError::ArityError {
            function: "new-buffer".into(),
            expected: 2,
            actual: args_len,
        });
    }
    let mut buffer = SporeBuffer::default();
    if args_len >= 1 {
        let v = ctx.arg(0);
        match v.unwrap().try_str(ctx.vm()) {
            Ok(s) => buffer.name = s.into(),
            Err(v) => {
                return Err(VmError::TypeError {
                    context: "new-buffer",
                    expected: UnsafeVal::STRING_TYPE_NAME,
                    actual: v.type_name(),
                    value: v.format_quoted(ctx.vm()).to_string(),
                })
            }
        }
    }
    if args_len >= 2 {
        let v = ctx.arg(1);
        match v.unwrap().try_str(ctx.vm()) {
            Ok(s) => {
                buffer.insert(s);
            }
            Err(v) => {
                return Err(VmError::TypeError {
                    context: "new-buffer",
                    expected: UnsafeVal::STRING_TYPE_NAME,
                    actual: v.type_name(),
                    value: v.format_quoted(ctx.vm()).to_string(),
                })
            }
        }
    }
    Ok(ctx.new_custom(buffer))
}

fn buffer_insert(ctx: NativeFunctionContext) -> VmResult<ValBuilder> {
    if ctx.args_len() != 2 {
        return Err(VmError::ArityError {
            function: "buffer-insert!".into(),
            expected: 2,
            actual: ctx.args_len(),
        });
    }
    let insert_string = ctx
        .arg(1)
        .unwrap()
        .try_str(ctx.vm())
        .map_err(|v| VmError::TypeError {
            context: "buffer-insert!",
            expected: UnsafeVal::STRING_TYPE_NAME,
            actual: v.type_name(),
            value: v.format_quoted(ctx.vm()).to_string(),
        })?;
    let buffer_val = ctx.arg(0).unwrap();
    let mut buffer = buffer_val.as_custom_mut::<SporeBuffer>(ctx.vm())?;
    buffer.insert(insert_string);
    Ok(Val::new_void().into())
}

fn buffer_delete(ctx: NativeFunctionContext) -> VmResult<ValBuilder> {
    if ctx.args_len() != 1 {
        return Err(VmError::ArityError {
            function: "buffer-delete!".into(),
            expected: 1,
            actual: ctx.args_len(),
        });
    }
    let buffer_val = ctx.arg(0).unwrap();
    let mut buffer = buffer_val.as_custom_mut::<SporeBuffer>(ctx.vm())?;
    buffer.delete();
    Ok(Val::new_void().into())
}

fn buffer_cursor_move(ctx: NativeFunctionContext) -> VmResult<ValBuilder> {
    let (ctx, args) = ctx.split_args();
    match args {
        [buffer, n] => {
            let mut buffer = buffer.as_custom_mut::<SporeBuffer>(ctx.vm())?;
            let n = n.try_int().map_err(|v| VmError::TypeError {
                context: "buffer-cursor-move!",
                expected: UnsafeVal::INT_TYPE_NAME,
                actual: v.type_name(),
                value: v.format_quoted(ctx.vm()).to_string(),
            })?;
            buffer.cursor = buffer.cursor_offset(n);
            Ok(ValBuilder::new(().into()))
        }
        _ => Err(VmError::ArityError {
            function: "buffer-cursor-move!".into(),
            expected: 2,
            actual: args.len(),
        }),
    }
}

impl CustomType for SporeBuffer {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl std::fmt::Display for SporeBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SporeBuffer")
            .field("name", &self.name)
            .finish()
    }
}
