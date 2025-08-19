const std = @import("std");

const errors = @import("../errors.zig");
const DetailedError = errors.DetailedError;
const NativeFunction = @import("../NativeFunction.zig");
const Val = @import("../Val.zig");
const Vm = @import("../Vm.zig");

/// Registers all arithmetic functions with the provided Vm.
pub fn registerAll(vm: *Vm) !void {
    try add.register(vm);
    try multiply.register(vm);
    try subtract.register(vm);
    try mod.register(vm);
    try divide.register(vm);
    try equal_q.register(vm);
}

const add = NativeFunction{
    .name = "+",
    .docstring = "Adds all values on the local stack. It can handle integers, " ++
        "floats, or a mix of both. If any floats are present, the result will be a float.",
    .ptr = addImpl,
};

fn addSlice(vm: *Vm, vals: []const Val) errors.Error!Val {
    var int_sum: i64 = 0;
    var float_sum: f64 = 0.0;
    var has_float = false;
    for (vals) |val| {
        switch (val.repr) {
            .int => |x| int_sum += x,
            .float => |x| {
                has_float = true;
                float_sum += x;
            },
            else => return vm.builder().addError(DetailedError{ .wrong_type = .{ .want = "int or float", .got = val } }),
        }
    }
    if (has_float) {
        const sum_as_float: f64 = float_sum + @as(f64, @floatFromInt(int_sum));
        return Val.init(sum_as_float);
    }
    return Val.init(int_sum);
}

fn addImpl(vm: *Vm) errors.Error!Val {
    return try addSlice(vm, vm.execution_context.localStack());
}

const subtract = NativeFunction{
    .name = "-",
    .docstring = "Subtracts values. With one argument, negates it. With two, subtracts the second from the first. With more, subtracts the sum of the rest from the first.",
    .ptr = subtractImpl,
};

fn negate(vm: *Vm, val: Val) !Val {
    switch (val.repr) {
        .int => |x| return Val.init(-x),
        .float => |x| return Val.init(-x),
        else => return vm.builder().addError(DetailedError{ .wrong_type = .{ .got = val, .want = "int or float" } }),
    }
}

fn subtractTwo(vm: *Vm, val1: Val, val2: Val) errors.Error!Val {
    switch (val1.repr) {
        .int => |int1| {
            switch (val2.repr) {
                .int => |int2| return Val.init(int1 - int2),
                .float => |float2| return Val.init(@as(f64, @floatFromInt(int1)) - float2),
                else => return vm.builder().addError(DetailedError{ .wrong_type = .{ .got = val2, .want = "int or float" } }),
            }
        },
        .float => |float1| {
            switch (val2.repr) {
                .int => |int2| return Val.init(float1 - @as(f64, @floatFromInt(int2))),
                .float => |float2| return Val.init(float1 - float2),
                else => return vm.builder().addError(DetailedError{ .wrong_type = .{ .got = val2, .want = "int or float" } }),
            }
        },
        else => return vm.builder().addError(DetailedError{ .wrong_type = .{ .want = "int or float", .got = val1 } }),
    }
}

fn subtractImpl(vm: *Vm) errors.Error!Val {
    const args = vm.execution_context.localStack();
    switch (args.len) {
        0 => return vm.builder().addError(DetailedError{ .wrong_arity = .{
            .function = "-",
            .want = 1,
            .got = @intCast(args.len),
        } }),
        1 => return try negate(vm, args[0]),
        2 => return try subtractTwo(vm, args[0], args[1]),
        else => return try subtractTwo(
            vm,
            args[0],
            try addSlice(vm, args[1..]),
        ),
    }
}

const multiply = NativeFunction{
    .name = "*",
    .docstring = "Multiplies all values on the local stack. It can handle integers, " ++
        "floats, or a mix of both. If any floats are present, the result will be a float.",
    .ptr = multiplyImpl,
};

fn multiplySlice(vm: *Vm, vals: []const Val) errors.Error!Val {
    var int_product: i64 = 1;
    var float_product: f64 = 1.0;
    var has_float = false;
    for (vals) |val| {
        switch (val.repr) {
            .int => |x| int_product *= x,
            .float => |x| {
                has_float = true;
                float_product *= x;
            },
            else => return vm.builder().addError(DetailedError{ .wrong_type = .{ .want = "int or float", .got = val } }),
        }
    }
    if (has_float) {
        const product_as_float: f64 = float_product * @as(f64, @floatFromInt(int_product));
        return Val.init(product_as_float);
    }
    return Val.init(int_product);
}

fn multiplyImpl(vm: *Vm) errors.Error!Val {
    return try multiplySlice(vm, vm.execution_context.localStack());
}

const mod = NativeFunction{
    .name = "mod",
    .docstring = "Returns the modulus of two integers. Example: `(mod 10 3)` returns `1`.",
    .ptr = modImpl,
};

fn modImpl(vm: *Vm) errors.Error!Val {
    const args = vm.execution_context.localStack();
    if (args.len != 2) return vm.builder().addError(DetailedError{ .wrong_arity = .{
        .function = "mod",
        .want = 2,
        .got = @intCast(args.len),
    } });
    const a = args[0].to(i64) catch return vm.builder().addError(
        DetailedError{ .wrong_type = .{ .want = "int", .got = args[0] } },
    );
    const b = args[1].to(i64) catch return vm.builder().addError(
        DetailedError{ .wrong_type = .{ .want = "int", .got = args[1] } },
    );
    if (b == 0) return errors.Error.DivisionByZero;
    return Val.init(@mod(a, b));
}

fn toFloat(vm: *Vm, val: Val) errors.Error!f64 {
    switch (val.repr) {
        .int => |x| return @floatFromInt(x),
        .float => |x| return x,
        else => return vm.builder().addError(DetailedError{ .wrong_type = .{ .got = val, .want = "int or float" } }),
    }
}

const divide = NativeFunction{
    .name = "/",
    .docstring = "Divides numbers. With one argument, returns 1.0 divided by the argument. With two, divides the first by the second. Returns a float.",
    .ptr = divideImpl,
};

fn divideImpl(vm: *Vm) errors.Error!Val {
    const args = vm.execution_context.localStack();
    switch (args.len) {
        1 => {
            const denominator = try toFloat(vm, args[0]);
            if (denominator == 0.0) return vm.builder().addError(DetailedError{ .divide_by_zero = {} });
            return Val.init(1.0 / denominator);
        },
        2 => return {
            const numerator = try toFloat(vm, args[0]);
            const denominator = try toFloat(vm, args[1]);
            if (denominator == 0.0) return vm.builder().addError(DetailedError{ .divide_by_zero = {} });
            return Val.init(numerator / denominator);
        },
        else => return vm.builder().addError(DetailedError{ .wrong_arity = .{
            .function = "/",
            .want = 1,
            .got = @intCast(args.len),
        } }),
    }
}

const equal_q = NativeFunction{
    .name = "=",
    .docstring = "Returns true if two numbers are equal, nil otherwise. Returns a WrongType for non-numeric arguments.",
    .ptr = equalQImpl,
};

fn equalQImpl(vm: *Vm) errors.Error!Val {
    const args = vm.execution_context.localStack();
    if (args.len != 2) return vm.builder().addError(DetailedError{ .wrong_arity = .{
        .function = "=",
        .want = 2,
        .got = @intCast(args.len),
    } });

    switch (args[0].repr) {
        .int => |int1| {
            switch (args[1].repr) {
                .int => |int2| return Val.init(int1 == int2),
                .float => |float2| return Val.init(@as(f64, @floatFromInt(int1)) == float2),
                else => return vm.builder().addError(DetailedError{ .wrong_type = .{ .want = "int or float", .got = args[1] } }),
            }
        },
        .float => |float1| {
            switch (args[1].repr) {
                .int => |int2| return Val.init(float1 == @as(f64, @floatFromInt(int2))),
                .float => |float2| return Val.init(float1 == float2),
                else => return vm.builder().addError(DetailedError{ .wrong_type = .{ .want = "int or float", .got = args[1] } }),
            }
        },
        else => return vm.builder().addError(DetailedError{ .wrong_type = .{ .want = "int or float", .got = args[0] } }),
    }
}
