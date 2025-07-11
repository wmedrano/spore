const std = @import("std");
const Vm = @import("Vm.zig");
const Val = @import("Val.zig");

const Function = @This();

pub const FunctionError = error{
    WrongArity,
    TypeError,
};

/// The name of the function.
name: []const u8,
/// The number of args the function expects.
arity: u8,
/// The pointer to the native function implementation.
ptr: *fn (*Vm) FunctionError!Val,

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
