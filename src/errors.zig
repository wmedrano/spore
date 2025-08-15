const std = @import("std");

const Symbol = @import("Symbol.zig");
const Val = @import("Val.zig");

pub const Error = error{
    /// An attempt was made to resolve a symbol that does not exist in the
    /// current scope.
    SymbolNotFound,
    /// A function was called with an incorrect number of arguments.
    WrongArity,
    /// An allocation failed due to insufficient memory.
    OutOfMemory,
    /// An attempt was made to access an object that does not exist in the
    /// object pool. This can happen if the object was garbage collected.
    ObjectNotFound,
    /// An error occurred during an I/O operation.
    IoError,
    /// A value has an unexpected type.
    WrongType,
    /// An attempt was made to divide by zero.
    DivisionByZero,
    /// The call stack has exceeded its maximum allowed depth.
    StackOverflow,
    /// An attempt was made to pop a value from an empty stack.
    StackUnderflow,
    /// An unexpected internal error occurred within the VM. This indicates a
    /// bug in the interpreter's implementation and should not be experienced by
    /// users.
    Internal,
};

pub const DetailedError = union(enum) {
    out_of_memory: void,
    wrong_arity: struct { function: []const u8, want: i32, got: i32 },
    symbol_not_found: struct { symbol: Symbol.Interned },
    object_not_found: struct { object: Val },
    io_error: void,
    wrong_type: struct { want: []const u8, got: Val },
    divide_by_zero: void,
    stack_overflow: void,
    stack_underflow: void,
    internal: void,

    /// Returns the `Val` that is associated with the error, if any.
    ///
    /// Returns:
    ///     An optional `Val`, which contains the referenced value if applicable,
    ///     otherwise `null`.
    pub fn referencedVal(self: DetailedError) ?Val {
        switch (self) {
            .out_of_memory => return null,
            .wrong_arity => return null,
            .symbol_not_found => return null,
            .object_not_found => |e| return e.object,
            .io_error => return null,
            .wrong_type => |e| return e.got,
            .divide_by_zero => return null,
            .stack_overflow => return null,
            .stack_underflow => return null,
            .internal => return null,
        }
    }

    /// Get the `DetailedError` as a normal Zig error.
    pub fn err(self: DetailedError) Error {
        switch (self) {
            .out_of_memory => return Error.OutOfMemory,
            .wrong_arity => return Error.WrongArity,
            .symbol_not_found => return Error.SymbolNotFound,
            .object_not_found => return Error.ObjectNotFound,
            .io_error => return Error.IoError,
            .wrong_type => return Error.WrongType,
            .divide_by_zero => return Error.DivisionByZero,
            .stack_overflow => return Error.StackOverflow,
            .stack_underflow => return Error.StackUnderflow,
            .internal => return Error.Internal,
        }
    }
};
