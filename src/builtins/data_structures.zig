const std = @import("std");

const errors = @import("../errors.zig");
const DetailedError = errors.DetailedError;
const Handle = @import("../object_pool.zig").Handle;
const NativeFunction = @import("../NativeFunction.zig");
const Pair = @import("../Pair.zig");
const Val = @import("../Val.zig");
const Vm = @import("../Vm.zig");

/// Registers all data structure functions with the provided Vm.
pub fn registerAll(vm: *Vm) !void {
    try pair.register(vm);
    try first.register(vm);
    try second.register(vm);
    try list.register(vm);
    try empty_q.register(vm);
}

const pair = NativeFunction{
    .name = "pair",
    .docstring = "Returns a pair with 2 values.",
    .ptr = pairImpl,
};

fn pairImpl(vm: *Vm) errors.Error!Val {
    const args = vm.execution_context.localStack();
    if (args.len != 2) return vm.builder().addError(DetailedError{ .wrong_arity = .{
        .function = "pair",
        .want = 2,
        .got = @intCast(args.len),
    } });
    return vm.initVal(Pair.init(args[0], args[1]));
}

const first = NativeFunction{
    .name = "first",
    .docstring = "Returns the first element of a pair.",
    .ptr = firstImpl,
};

fn firstImpl(vm: *Vm) errors.Error!Val {
    const args = vm.execution_context.localStack();
    if (args.len != 1) return vm.builder().addError(DetailedError{ .wrong_arity = .{
        .function = "first",
        .want = 1,
        .got = @intCast(args.len),
    } });
    const pair_handle = args[0].to(Handle(Pair)) catch return vm.builder().addError(
        DetailedError{ .wrong_type = .{ .want = "pair", .got = args[0] } },
    );
    const p = try vm.heap.pairs.get(pair_handle);
    return p.first;
}

const second = NativeFunction{
    .name = "second",
    .docstring = "Returns the second element of a pair.",
    .ptr = secondImpl,
};

fn secondImpl(vm: *Vm) errors.Error!Val {
    const args = vm.execution_context.localStack();
    if (args.len != 1) return vm.builder().addError(DetailedError{ .wrong_arity = .{
        .function = "second",
        .want = 1,
        .got = @intCast(args.len),
    } });
    const pair_handle = args[0].to(Handle(Pair)) catch return vm.builder().addError(
        DetailedError{ .wrong_type = .{ .want = "pair", .got = args[0] } },
    );
    const p = try vm.heap.pairs.get(pair_handle);
    return p.second;
}

const list = NativeFunction{
    .name = "list",
    .docstring = "Returns a new list containing all provided arguments.",
    .ptr = listImpl,
};

fn listImpl(vm: *Vm) errors.Error!Val {
    const args = vm.execution_context.localStack();
    return vm.initVal(args);
}

const empty_q = NativeFunction{
    .name = "empty?",
    .docstring = "Returns true if the argument is nil (representing an empty list), false if it's a pair, and throws an error for othre types.",
    .ptr = emptyQImpl,
};

fn emptyQImpl(vm: *Vm) errors.Error!Val {
    const args = vm.execution_context.localStack();
    if (args.len != 1) return vm.builder().addError(DetailedError{ .wrong_arity = .{
        .function = "empty?",
        .want = 1,
        .got = @intCast(args.len),
    } });
    switch (args[0].repr) {
        .nil => return Val.init(true),
        .pair => return Val.init(false),
        else => return vm.builder().addError(DetailedError{ .wrong_type = .{ .want = "list", .got = args[0] } }),
    }
}