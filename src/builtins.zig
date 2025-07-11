const std = @import("std");
const testing = std.testing;

const Val = @import("Val.zig");
const Vm = @import("Vm.zig");
const Function = @import("Function.zig");
const Symbol = @import("datastructures/Symbol.zig");

fn addFunctionImpl(vm: *Vm) Function.Error!Val {
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
            else => return Function.Error.TypeError,
        }
    }
    if (has_float) {
        const sum_as_float: f64 = float_sum + @as(f64, @floatFromInt(int_sum));
        return Val.from(sum_as_float);
    }
    return Val.from(int_sum);
}

/// A native function that adds all values on the local stack. It can handle
/// integers, floats, or a mix of both. If any floats are present, the result
/// will be a float.
pub const AddFunction = Function{
    .name = "+",
    .docstring = "Adds all values on the local stack. It can handle integers, " ++
        "floats, or a mix of both. If any floats are present, the result will be a float.",
    .ptr = addFunctionImpl,
};

test "+ sums integers" {
    var vm = Vm.init(testing.allocator);
    defer vm.deinit();

    try vm.execution_context.pushVals(&.{ Val.from(1), Val.from(2), Val.from(3) });
    const result = try AddFunction.call(&vm);
    try testing.expectEqualDeep(Val.from(6), result);
}

test "+ sums floats" {
    var vm = Vm.init(testing.allocator);
    defer vm.deinit();

    try vm.execution_context.pushVals(&.{ Val.from(1.5), Val.from(2.5), Val.from(3.5) });
    const result = try AddFunction.call(&vm);
    try testing.expectEqualDeep(Val.from(7.5), result);
}

test "+ sums mixed integers and floats and returns float" {
    var vm = Vm.init(testing.allocator);
    defer vm.deinit();

    try vm.execution_context.pushVals(&.{ Val.from(1), Val.from(2.5), Val.from(3) });
    const result = try AddFunction.call(&vm);
    try testing.expectEqualDeep(Val.from(6.5), result);
}

test "+ returns TypeError for non-numeric values" {
    var vm = Vm.init(testing.allocator);
    defer vm.deinit();

    const symbol = try Symbol.init("my-var").intern(vm.heap.allocator, &vm.heap.string_interner);
    try vm.execution_context.pushVals(&.{ Val.from(1), Val.from(symbol) });
    try testing.expectError(Function.Error.TypeError, AddFunction.call(&vm));
}
