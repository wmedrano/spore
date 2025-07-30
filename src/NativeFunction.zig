const std = @import("std");
const Vm = @import("Vm.zig");
const Val = @import("Val.zig");
const Symbol = @import("datastructures/Symbol.zig");

const NativeFunction = @This();

/// An error that can occur when calling a function.
pub const Error = error{
    /// The function was called with the wrong number of arguments.
    WrongArity,
    /// An argument to the function had an incorrect type.
    TypeError,
    /// An object was not found, likely due to being garbage collected.
    ObjectNotFound,
    /// A division by zero was attempted.
    DivisionByZero,
} || std.mem.Allocator.Error;

/// The name of the function.
name: []const u8,
/// The docstring for the function.
docstring: []const u8,
/// The pointer to the native function implementation.
ptr: *const fn (*Vm) Error!Val,

/// Call the function. The arguments for the function are assumed to be on the
/// local stack of the current call frame.
pub fn call(self: NativeFunction, vm: *Vm) Error!Val {
    return self.ptr(vm);
}

/// Formats the `NativeFunction` for printing, implementing the `std.fmt.Format`
/// interface.
pub fn format(
    self: NativeFunction,
    comptime fmt: []const u8,
    options: std.fmt.FormatOptions,
    writer: anytype,
) !void {
    _ = fmt;
    _ = options;
    try writer.print("@nativefunction-{s}", .{self.name});
}

/// Add the function to a `Vm`'s global namespace.
pub fn register(self: *const NativeFunction, vm: *Vm) !void {
    try vm.execution_context.setGlobal(
        vm.heap.allocator,
        try Symbol.init(self.name).intern(vm.heap.allocator, &vm.heap.string_interner),
        Val.init(self),
    );
}
