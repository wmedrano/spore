const std = @import("std");
const Vm = @import("Vm.zig");
const Val = @import("Val.zig");

const Function = @This();

/// An error that can occur when calling a function.
pub const Error = error{
    /// The function was called with the wrong number of arguments.
    WrongArity,
    /// An argument to the function had an incorrect type.
    TypeError,
};

/// The name of the function.
name: []const u8,
/// The docstring for the function.
docstring: []const u8,
/// The pointer to the native function implementation.
ptr: *const fn (*Vm) Error!Val,

/// Call the function. The arguments for the function are assumed to be on the
/// local stack of the current call frame.
pub fn call(self: Function, vm: *Vm) Error!Val {
    return self.ptr(vm);
}

pub fn format(
    self: Function,
    comptime fmt: []const u8,
    options: std.fmt.FormatOptions,
    writer: anytype,
) !void {
    _ = fmt;
    _ = options;
    try writer.print("@nativefunction-{s}", .{self.name});
}
