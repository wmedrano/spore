const std = @import("std");

const errors = @import("../errors.zig");
const DetailedError = errors.DetailedError;
const NativeFunction = @import("../NativeFunction.zig");
const Val = @import("../Val.zig");
const Vm = @import("../Vm.zig");

/// Registers all type predicate functions with the provided Vm.
pub fn registerAll(vm: *Vm) !void {
    try number_q.register(vm);
    try symbol_q.register(vm);
    try null_q.register(vm);
    try string_q.register(vm);
    try pair_q.register(vm);
}

const number_q = NativeFunction{
    .name = "number?",
    .docstring = "Returns true if the argument is a number (integer or float), false otherwise.",
    .ptr = numberQImpl,
};

fn numberQImpl(vm: *Vm) errors.Error!Val {
    const args = vm.execution_context.localStack();
    if (args.len != 1) return vm.builder().addError(DetailedError{ .wrong_arity = .{
        .function = "number?",
        .want = 1,
        .got = @intCast(args.len),
    } });
    const val = args[0];
    switch (val.repr) {
        .int, .float => return Val.init(true),
        else => return Val.init(false),
    }
}

const symbol_q = NativeFunction{
    .name = "symbol?",
    .docstring = "Returns true if the argument is a symbol, false otherwise.",
    .ptr = symbolQImpl,
};

fn symbolQImpl(vm: *Vm) errors.Error!Val {
    const args = vm.execution_context.localStack();
    if (args.len != 1) return vm.builder().addError(DetailedError{ .wrong_arity = .{
        .function = "symbol?",
        .want = 1,
        .got = @intCast(args.len),
    } });
    const val = args[0];
    switch (val.repr) {
        .symbol => return Val.init(true),
        else => return Val.init(false),
    }
}

const null_q = NativeFunction{
    .name = "null?",
    .docstring = "Returns true if the argument is nil, false otherwise.",
    .ptr = nullQImpl,
};

fn nullQImpl(vm: *Vm) errors.Error!Val {
    const args = vm.execution_context.localStack();
    if (args.len != 1) return vm.builder().addError(DetailedError{ .wrong_arity = .{
        .function = "null?",
        .want = 1,
        .got = @intCast(args.len),
    } });
    const val = args[0];
    switch (val.repr) {
        .nil => return Val.init(true),
        else => return Val.init(false),
    }
}

const string_q = NativeFunction{
    .name = "string?",
    .docstring = "Returns true if the argument is a string, false otherwise.",
    .ptr = stringQImpl,
};

fn stringQImpl(vm: *Vm) errors.Error!Val {
    const args = vm.execution_context.localStack();
    if (args.len != 1) return vm.builder().addError(DetailedError{ .wrong_arity = .{
        .function = "string?",
        .want = 1,
        .got = @intCast(args.len),
    } });
    switch (args[0].repr) {
        .string => return Val.init(true),
        else => return Val.init(false),
    }
}

const pair_q = NativeFunction{
    .name = "pair?",
    .docstring = "Returns true if the argument is a pair, false otherwise.",
    .ptr = pairQImpl,
};

fn pairQImpl(vm: *Vm) errors.Error!Val {
    const args = vm.execution_context.localStack();
    if (args.len != 1) return vm.builder().addError(DetailedError{ .wrong_arity = .{
        .function = "pair?",
        .want = 1,
        .got = @intCast(args.len),
    } });
    const val = args[0];
    switch (val.repr) {
        .pair => return Val.init(true),
        else => return Val.init(false),
    }
}