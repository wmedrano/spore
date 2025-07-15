const std = @import("std");
const testing = std.testing;

const Val = @import("Val.zig");
const Vm = @import("Vm.zig");
const NativeFunction = @import("NativeFunction.zig");
const Symbol = @import("datastructures/Symbol.zig");

/// A native function that adds all values on the local stack. It can handle
/// integers, floats, or a mix of both. If any floats are present, the result
/// will be a float.
pub const add = NativeFunction{
    .name = "+",
    .docstring = "Adds all values on the local stack. It can handle integers, " ++
        "floats, or a mix of both. If any floats are present, the result will be a float.",
    .ptr = addImpl,
};

fn addImpl(vm: *Vm) NativeFunction.Error!Val {
    var int_sum: i64 = 0;
    var float_sum: f64 = 0.0;
    var has_float = false;
    for (vm.execution_context.localStack()) |val| {
        switch (val.repr) {
            .int => |x| int_sum += x,
            .float => |x| {
                has_float = true;
                float_sum += x;
            },
            else => return NativeFunction.Error.TypeError,
        }
    }
    if (has_float) {
        const sum_as_float: f64 = float_sum + @as(f64, @floatFromInt(int_sum));
        return Val.from(sum_as_float);
    }
    return Val.from(int_sum);
}

/// A native function that defines a global variable. It expects two arguments: a
/// symbol (the name of the variable) and a value.
pub const define = NativeFunction{
    .name = "define",
    .docstring = "Defines a global variable. It expects two arguments: a symbol " ++
        "(the name of the variable) and a value.",
    .ptr = defineImpl,
};

fn defineImpl(vm: *Vm) NativeFunction.Error!Val {
    const args = vm.execution_context.localStack();
    if (args.len != 2) return NativeFunction.Error.WrongArity;
    const symbol = try args[0].to(Symbol.Interned);
    const value = args[1];
    try vm.execution_context.setGlobal(vm.heap.allocator, symbol, value);
    return Val.from({});
}

/// Registers all built-in native functions with the provided Vm.
pub fn registerAll(vm: *Vm) !void {
    try add.register(vm);
    try define.register(vm);
}

test "+ sums integers" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    try vm.execution_context.pushVals(&.{ Val.from(1), Val.from(2), Val.from(3) });
    const result = try add.call(&vm);
    try testing.expectEqualDeep(Val.from(6), result);
}

test "+ sums floats" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.from(7.5),
        try vm.evalStr("(+ 3.0 4.5)"),
    );
}

test "+ sums mixed integers and floats and returns float" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    try testing.expectEqualDeep(
        Val.from(6.5),
        try vm.evalStr("(+ 1 2.5 3)"),
    );
}

test "+ returns TypeError for non-numeric values" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    try testing.expectError(
        NativeFunction.Error.TypeError,
        vm.evalStr("(+ 1 'my-var)"),
    );
}

test "define sets global variable" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    _ = try vm.evalStr("(define 'x 42)");
    try testing.expectEqualDeep(
        Val.from(42),
        vm.evalStr("x"),
    );
}
