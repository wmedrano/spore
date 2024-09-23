use std::any::Any;

use compact_str::CompactString;
use crop::Rope;
#[allow(unused_imports)]
use log::*;
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
    pub cursor: Cursor,
}

#[derive(Copy, Clone, Debug)]
pub enum Cursor {
    Absolute(usize),
    ColRow(usize, usize),
}

impl Default for Cursor {
    fn default() -> Cursor {
        Cursor::Absolute(0)
    }
}

impl SporeBuffer {
    pub fn cursor_offset(&self, offset: i64) -> usize {
        let absolute = match self.cursor {
            Cursor::Absolute(x) => x,
            Cursor::ColRow(x, y) => {
                let y = y.clamp(0, self.contents.line_len().saturating_sub(1));
                let line_start = self.contents.byte_of_line(y);
                let line = self.contents.line(y);
                let x = x.clamp(0, line.byte_len());
                line_start + x
            }
        };
        let cursor = (absolute as i64 + offset).clamp(0, self.contents.byte_len() as i64);
        cursor as usize
    }

    fn insert(&mut self, text: &str) {
        self.contents.insert(self.cursor_offset(0), text);
        self.cursor = Cursor::Absolute(self.cursor_offset(text.len() as i64));
    }

    fn delete(&mut self) {
        if matches!(self.cursor, Cursor::Absolute(0) | Cursor::ColRow(0, 0)) {
            return;
        }
        // TODO: Handle unicode and graphemes better.
        self.contents
            .replace(self.cursor_offset(-1)..self.cursor_offset(0), "");
        self.cursor = Cursor::Absolute(self.cursor_offset(-1));
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
        [buffer, xs, ys] => {
            let mut buffer = buffer.as_custom_mut::<SporeBuffer>(ctx.vm())?;
            let xs = xs.try_int().map_err(|v| VmError::TypeError {
                context: "buffer-cursor-move! (arg-idx=1)",
                expected: UnsafeVal::INT_TYPE_NAME,
                actual: v.type_name(),
                value: v.format_quoted(ctx.vm()).to_string(),
            })?;
            if xs != 0 {
                buffer.cursor = Cursor::Absolute(buffer.cursor_offset(xs));
            }
            let ys = ys.try_int().map_err(|v| VmError::TypeError {
                context: "buffer-cursor-move! (arg-idx=2)",
                expected: UnsafeVal::INT_TYPE_NAME,
                actual: v.type_name(),
                value: v.format_quoted(ctx.vm()).to_string(),
            })?;
            if ys != 0 {
                let (x, y) = match buffer.cursor {
                    Cursor::Absolute(pos) => {
                        let line = buffer.contents.line_of_byte(pos);
                        let line_start = buffer.contents.byte_of_line(line);
                        let x = pos - line_start;
                        let y = (line as i64 + ys)
                            .clamp(0, buffer.contents.line_len().saturating_sub(1) as i64);
                        (x, y)
                    }
                    Cursor::ColRow(x, y) => (x, y as i64 + ys),
                };
                buffer.cursor = Cursor::ColRow(
                    x,
                    y.clamp(0, buffer.contents.line_len().saturating_sub(1) as i64) as usize,
                )
            }
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
