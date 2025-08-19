const std = @import("std");

const errors = @import("../errors.zig");
const DetailedError = errors.DetailedError;
const NativeFunction = @import("../NativeFunction.zig");
const Val = @import("../Val.zig");
const Vm = @import("../Vm.zig");

/// Registers all I/O functions with the provided Vm.
pub fn registerAll(vm: *Vm) !void {
    try print.register(vm);
    try println.register(vm);
}

const print = NativeFunction{
    .name = "print",
    .docstring = "Prints all arguments.",
    .ptr = printImpl,
};

fn printImpl(vm: *Vm) errors.Error!Val {
    const args = vm.execution_context.localStack();
    var buffer = std.ArrayList(u8).init(vm.heap.allocator);
    defer buffer.deinit();
    const vals = vm.inspector().prettySlice(args);
    std.fmt.format(std.io.getStdOut().writer(), "{any}", .{vals}) catch return vm.builder().addError(DetailedError{ .io_error = {} });
    return Val.init({});
}

const println = NativeFunction{
    .name = "println",
    .docstring = "Prints all arguments followed by a newline.",
    .ptr = printlnImpl,
};

fn printlnImpl(vm: *Vm) errors.Error!Val {
    const args = vm.execution_context.localStack();
    var buffer = std.ArrayList(u8).init(vm.heap.allocator);
    defer buffer.deinit();
    const vals = vm.inspector().prettySlice(args);
    std.fmt.format(std.io.getStdOut().writer(), "{any}\n", .{vals}) catch return errors.Error.IoError;
    return Val.init({});
}
