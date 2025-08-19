const std = @import("std");

const errors = @import("../errors.zig");
const DetailedError = errors.DetailedError;
const NativeFunction = @import("../NativeFunction.zig");
const Val = @import("../Val.zig");
const Vm = @import("../Vm.zig");

/// Registers all conversion functions with the provided Vm.
pub fn registerAll(vm: *Vm) !void {
    try to_string.register(vm);
}

const to_string = NativeFunction{
    .name = "->string",
    .docstring = "Converts the argument to its string representation.",
    .ptr = toStringImpl,
};

fn toStringImpl(vm: *Vm) errors.Error!Val {
    const args = vm.execution_context.localStack();
    if (args.len != 1) return vm.builder().addError(DetailedError{ .wrong_arity = .{
        .function = "->string",
        .want = 1,
        .got = @intCast(args.len),
    } });
    var buffer = std.ArrayList(u8).init(vm.heap.allocator);
    defer buffer.deinit();
    try vm.inspector().pretty(args[0]).format("any", .{}, buffer.writer());
    const return_val = try vm.initVal(buffer.items);
    return return_val;
}