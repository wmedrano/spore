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

#[derive(Copy, Clone, Debug, Default)]
pub struct Cursor {
    /// The byte index of the cursor.
    byte_idx: usize,
    /// The x and y position. This is only populated when scrolling vertically to keep the desired
    /// column position for further vertical scrolls.
    x_y: Option<(usize, usize)>,
}

impl Cursor {
    /// The byte index of the cursor.
    pub fn byte_idx(self) -> usize {
        self.byte_idx
    }

    /// Move the cursor horizontally by `delta_x`. `buffer` is used to check maximum bounds.
    ///
    /// If the cursor reaches the end of the line, it will wrap to the next line.
    pub fn move_horizontal(&mut self, buffer: &Rope, delta_x: i64) {
        let pos = self.byte_idx as i64 + delta_x;
        self.byte_idx = pos.clamp(0, buffer.byte_len() as i64) as usize;
        self.x_y.take();
    }

    /// Move the cursor vertically by `delta_y`. `buffer` is used to check maximum bounds.
    ///
    /// If the cursor is on the first or last line, then this is a no op.
    pub fn move_vertical(&mut self, buffer: &Rope, delta_y: i64) {
        let (x, y) = self.x_y.unwrap_or_else(|| {
            let y = buffer.line_of_byte(self.byte_idx);
            let line_start = buffer.byte_of_line(y);
            let x = self.byte_idx - line_start;
            (x, y)
        });
        let new_y =
            (y as i64 + delta_y).clamp(0, buffer.line_len().saturating_sub(1) as i64) as usize;
        self.x_y = Some((x, new_y));
        let line = buffer.line(new_y);
        self.byte_idx = buffer.byte_of_line(new_y) + x.clamp(0, line.byte_len());
    }
}

impl SporeBuffer {
    fn insert(&mut self, text: &str) {
        self.contents.insert(self.cursor.byte_idx(), text);
        self.cursor
            .move_horizontal(&self.contents, text.len() as i64);
    }

    fn delete(&mut self) {
        // TODO: Handle unicode and graphemes better.
        self.contents.replace(
            self.cursor.byte_idx().saturating_sub(1)..self.cursor.byte_idx(),
            "",
        );
        self.cursor.move_horizontal(&self.contents, -1);
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
    if !insert_string.is_empty() {
        let buffer_val = ctx.arg(0).unwrap();
        let mut buffer = buffer_val.as_custom_mut::<SporeBuffer>(ctx.vm())?;
        buffer.insert(insert_string);
    }
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
            let ys = ys.try_int().map_err(|v| VmError::TypeError {
                context: "buffer-cursor-move! (arg-idx=2)",
                expected: UnsafeVal::INT_TYPE_NAME,
                actual: v.type_name(),
                value: v.format_quoted(ctx.vm()).to_string(),
            })?;
            if xs != 0 {
                let mut cursor = std::mem::take(&mut buffer.cursor);
                cursor.move_horizontal(&buffer.contents, xs);
                buffer.cursor = cursor;
            }
            if ys != 0 {
                let mut cursor = std::mem::take(&mut buffer.cursor);
                cursor.move_vertical(&buffer.contents, ys);
                buffer.cursor = cursor;
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

impl CustomType for SporeBuffer {}

impl std::fmt::Display for SporeBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SporeBuffer")
            .field("name", &self.name)
            .finish()
    }
}
