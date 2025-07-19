//! Defines `BytecodeFunction`, which represents a function implemented in bytecode.
const std = @import("std");
const Vm = @import("Vm.zig");
const Val = @import("Val.zig");
const Symbol = @import("datastructures/Symbol.zig");
const Instruction = @import("Instruction.zig");

/// A function that is implemented in bytecode.
const BytecodeFunction = @This();

/// The instructions to execute.
instructions: []const Instruction,
/// The number of arguments the function takes.
args: usize = 0,
/// The size of the initial local stack. This is the space used up by the `args`
/// and local values.
initial_local_stack_size: usize = 0,

/// Deinitialize the bytecode function, freeing allocated memory.
pub fn deinit(self: *BytecodeFunction, allocator: std.mem.Allocator) void {
    allocator.free(self.instructions);
    self.instructions = &.{};
}

/// Formats the `NativeFunction` for printing, implementing the `std.fmt.Format`
/// interface.
pub fn format(
    _: BytecodeFunction,
    comptime fmt: []const u8,
    options: std.fmt.FormatOptions,
    writer: anytype,
) !void {
    _ = fmt;
    _ = options;
    try writer.print("@function", .{});
}
