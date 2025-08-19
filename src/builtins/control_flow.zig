const std = @import("std");

const errors = @import("../errors.zig");
const DetailedError = errors.DetailedError;
const NativeFunction = @import("../NativeFunction.zig");
const Pair = @import("../Pair.zig");
const Val = @import("../Val.zig");
const Vm = @import("../Vm.zig");

/// Registers all control flow functions with the provided Vm.
pub fn registerAll(vm: *Vm) !void {
    try apply.register(vm);
    try not.register(vm);
}

const apply = NativeFunction{
    .name = "apply",
    .docstring = "Applies a function to a list of arguments. Example: `(apply + (list 1 2 3))` returns `6`.",
    .ptr = applyImpl,
};

fn applyImpl(vm: *Vm) errors.Error!Val {
    const args = vm.execution_context.localStack();
    if (args.len != 2) return vm.builder().addError(DetailedError{ .wrong_arity = .{
        .function = "apply",
        .want = 2,
        .got = @intCast(args.len),
    } });

    const function_val = args[0];
    const args_list = args[1];

    switch (function_val.repr) {
        .native_function, .bytecode_function => {},
        else => return vm.builder().addError(
            DetailedError{
                .wrong_type = .{ .want = "function", .got = function_val },
            },
        ),
    }

    try vm.execution_context.pushVal(function_val);
    var list_iter = vm.inspector().to(Pair.ListIter, args_list) catch |err| switch (err) {
        error.WrongType => return vm.builder().addError(DetailedError{ .wrong_type = .{ .want = "list", .got = args_list } }),
        error.ObjectNotFound => return vm.builder().addError(DetailedError{ .object_not_found = .{ .object = args_list } }),
    };

    var arg_count: u32 = 0;
    while (list_iter.next(vm) catch |err| switch (err) {
        error.WrongType => return vm.builder().addError(DetailedError{ .wrong_type = .{ .want = "list", .got = args_list } }),
        error.ObjectNotFound => return vm.builder().addError(DetailedError{ .object_not_found = .{ .object = args_list } }),
    }) |arg_val| {
        try vm.execution_context.pushVal(arg_val);
        arg_count += 1;
    }

    return try vm.executeCall(@intCast(arg_count + 1));
}

const not = NativeFunction{
    .name = "not",
    .docstring = "Returns true if the argument is falsy (false or nil), false otherwise.",
    .ptr = notImpl,
};

fn notImpl(vm: *Vm) errors.Error!Val {
    const args = vm.execution_context.localStack();
    if (args.len != 1) return vm.builder().addError(DetailedError{ .wrong_arity = .{
        .function = "not",
        .want = 1,
        .got = @intCast(args.len),
    } });
    const val = args[0];
    return Val.init(!val.isTruthy());
}
