const std = @import("std");

const ConsCell = @import("ConsCell.zig");
const Symbol = @import("datastructures/Symbol.zig");
const NativeFunction = @import("NativeFunction.zig");
const PrettyPrinter = @import("PrettyPrinter.zig");
const String = @import("String.zig");
const Val = @import("Val.zig");
const Vm = @import("Vm.zig");

const Builder = @This();

vm: *Vm,

/// Converts a slice of Vals to a list.
/// Converts a slice of Vals to a list.
pub fn list(self: Builder, vals: []const Val) NativeFunction.Error!Val {
    if (vals.len == 0) return Val.init({});
    const head = vals[0];
    const tail = try self.list(vals[1..]);
    return self.cons(head, tail);
}

/// Creates a new cons cell with the given car and cdr.
///
/// Args:
///     car: The car of the cons cell.
///     cdr: The cdr of the cons cell.
///
/// Returns:
///     A Val representing the new cons cell.
/// Creates a new cons cell with the given car and cdr.
///
/// Args:
///     car: The car of the cons cell.
///     cdr: The cdr of the cons cell.
///
/// Returns:
///     A Val representing the new cons cell.
pub fn cons(self: Builder, car: Val, cdr: Val) NativeFunction.Error!Val {
    const cons_handle = try self.vm.heap.cons_cells.create(
        self.vm.heap.allocator,
        ConsCell.init(car, cdr),
        self.vm.heap.unreachable_color,
    );
    return Val.init(cons_handle);
}

/// Creates a new string by copying the given value.
///
/// Args:
///     s: The string value. This function copies the string.
///
/// Returns:
///     A Val representing the new string.
/// Creates a new string by copying the given value.
///
/// Args:
///     s: The string value. This function copies the string.
///
/// Returns:
///     A Val representing the new string.
pub fn string(self: Builder, s: []const u8) NativeFunction.Error!Val {
    const handle = try self.vm.heap.strings.create(
        self.vm.heap.allocator,
        try String.initCopy(self.vm.heap.allocator, s),
        self.vm.heap.unreachable_color,
    );
    return Val.init(handle);
}

/// Creates a new owned string with the given value.
///
/// Args:
/// s: The string value. This function takes ownership of the string, and the
///    caller should not use it after this call.
///
/// Returns:
///     A Val representing the new string.
/// Creates a new owned string with the given value.
///
/// Args:
/// s: The string value. This function takes ownership of the string, and the
///    caller should not use it after this call.
///
/// Returns:
///     A Val representing the new string.
pub fn stringOwned(self: Builder, s: []const u8) NativeFunction.Error!Val {
    const handle = try self.vm.heap.strings.create(
        self.vm.heap.allocator,
        String.initOwned(s),
        self.vm.heap.unreachable_color,
    );
    return Val.init(handle);
}

/// Creates and adds an error to the VM's error list when a symbol is not found.
///
/// Args:
///     sym: The interned symbol that was not found.
///
/// Returns:
///     A NativeFunction.Error.SymbolNotFound error.
pub fn symbolNotFound(self: Builder, sym: Symbol.Interned) NativeFunction.Error {
    const err_str = try std.fmt.allocPrint(
        self.vm.heap.allocator,
        "`{any}` not defined.",
        .{self.vm.inspector().pretty(Val.init(sym))},
    );
    self.vm.execution_context.last_error = try self.cons(
        try self.vm.builder().stringOwned(err_str),
        self.vm.execution_context.last_error,
    );
    return NativeFunction.Error.SymbolNotFound;
}

/// Creates and adds an error to the VM's error list when a stack underflow occurs.
///
/// Returns:
///     A NativeFunction.Error.StackUnderflow error.
pub fn stackUnderflow(self: Builder) NativeFunction.Error {
    const err_str = try self.string("Stack underflow! Likely a language bug.");
    try self.addError(err_str);
    return NativeFunction.Error.StackUnderflow;
}

/// Creates and adds an error to the VM's error list when a stack overflow occurs.
///
/// Returns:
///     A NativeFunction.Error.StackUnderflow error.
pub fn stackOverflow(self: Builder) NativeFunction.Error {
    const err_str = try self.string("Stack overflow!\nFunction call stack is probably too deep, but file a bug if this is a perfectly valid program.");
    try self.addError(err_str);
    return NativeFunction.Error.StackUnderflow;
}

/// Creates and adds a type error to the VM's error list.
///
/// Args:
///     expected: The expected type as a string.
///     got: The actual value that caused the type error.
///
/// Returns:
///     A NativeFunction.Error.TypeError error.
pub fn typeError(self: Builder, expected: []const u8, got: Val) NativeFunction.Error {
    const err_str = try std.fmt.allocPrint(
        self.vm.heap.allocator,
        "expected type {s} but got {any}",
        .{ expected, self.vm.inspector().pretty(got) },
    );
    const err = try self.stringOwned(err_str);
    try self.addError(err);
    return NativeFunction.Error.TypeError;
}

/// Creates and adds an arity error to the VM's error list.
///
/// Args:
///     expected_arity: A string describing the expected arity (e.g., "1", "2 or more", "at least 1").
///     actual: The actual number of arguments received.
///
/// Returns:
///     A NativeFunction.Error.WrongArity error.
pub fn arityError(self: Builder, expected_arity: []const u8, actual: usize) NativeFunction.Error {
    const err_str = try std.fmt.allocPrint(
        self.vm.heap.allocator,
        "Wrong number of arguments: expected {s}, got {d}",
        .{ expected_arity, actual },
    );
    const err = try self.stringOwned(err_str);
    try self.addError(err);
    return NativeFunction.Error.WrongArity;
}

fn addError(self: Builder, err: Val) !void {
    self.vm.execution_context.last_error = try self.cons(
        err,
        self.vm.execution_context.last_error,
    );
}
