const std = @import("std");
const testing = std.testing;

const errors = @import("errors.zig");
const DetailedError = errors.DetailedError;
const Handle = @import("object_pool.zig").Handle;
const NativeFunction = @import("NativeFunction.zig");
const Pair = @import("Pair.zig");
const String = @import("String.zig");
const Symbol = @import("Symbol.zig");
const Val = @import("Val.zig");
const Vm = @import("Vm.zig");

/// Registers all built-in native functions with the provided Vm.
pub fn registerAll(vm: *Vm) !void {
    try number_q.register(vm);
    try symbol_q.register(vm);
    try null_q.register(vm);
    try string_q.register(vm);
    try to_string.register(vm);
    try print.register(vm);
    try println.register(vm);
    try add.register(vm);
    try multiply.register(vm);
    try subtract.register(vm);
    try mod.register(vm);
    try divide.register(vm);
    try internal_define.register(vm);
    try pair.register(vm);
    try first.register(vm);
    try pair_q.register(vm);
    try second.register(vm);
    try list.register(vm);
    try empty_q.register(vm);
    try equal_q.register(vm);
    try range.register(vm);
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
    const return_val = try vm.builder().string(buffer.items);
    return return_val;
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

const internal_define = NativeFunction{
    .name = "internal-define",
    .docstring = "Defines a global variable. It expects two arguments: a symbol " ++
        "(the name of the variable) and a value.",
    .ptr = internalDefineImpl,
};

fn internalDefineImpl(vm: *Vm) errors.Error!Val {
    const args = vm.execution_context.localStack();
    if (args.len != 2) return vm.builder().addError(DetailedError{ .wrong_arity = .{
        .function = "internal-define",
        .want = 2,
        .got = @intCast(args.len),
    } });
    const symbol = args[0].to(Symbol.Interned) catch return vm.builder().addError(
        DetailedError{ .wrong_type = .{ .want = "symbol", .got = args[0] } },
    );
    const value = args[1];
    try vm.execution_context.setGlobal(vm.heap.allocator, symbol, value);
    return Val.init({});
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
    return vm.builder().pair(args[0], args[1]);
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

const list = NativeFunction{
    .name = "list",
    .docstring = "Returns a new list containing all provided arguments.",
    .ptr = listImpl,
};

fn listImpl(vm: *Vm) errors.Error!Val {
    const args = vm.execution_context.localStack();
    return vm.builder().list(args);
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

const range = NativeFunction{
    .name = "range",
    .docstring = "Creates a pair representing a half-open integer range [start, end). Both arguments must be integers.",
    .ptr = rangeImpl,
};

fn rangeImpl(vm: *Vm) errors.Error!Val {
    const args = vm.execution_context.localStack();
    if (args.len != 2) return vm.builder().addError(DetailedError{ .wrong_arity = .{
        .function = "range",
        .want = 2,
        .got = @intCast(args.len),
    } });
    
    const start = args[0].to(i64) catch return vm.builder().addError(
        DetailedError{ .wrong_type = .{ .want = "int", .got = args[0] } },
    );
    const end = args[1].to(i64) catch return vm.builder().addError(
        DetailedError{ .wrong_type = .{ .want = "int", .got = args[1] } },
    );
    
    return vm.builder().pair(Val.init(start), Val.init(end));
}

test "+ sums integers" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    try vm.execution_context.pushVals(&.{ Val.init(1), Val.init(2), Val.init(3) });
    const result = try add.call(&vm);
    try testing.expectEqualDeep(Val.init(6), result);
}

test "+ sums floats" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.init(7.5),
        try vm.evalStr("(+ 3.0 4.5)"),
    );
}

test "+ sums mixed integers and floats and returns float" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    try testing.expectEqualDeep(
        Val.init(6.5),
        try vm.evalStr("(+ 1 2.5 3)"),
    );
}

test "+ returns WrongType for non-numeric values" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    try testing.expectError(
        errors.Error.WrongType,
        vm.evalStr("(+ 1 \"my-var\")"),
    );
}

test "define sets global variable" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    _ = try vm.evalStr("(internal-define (quote x) 42)");
    try testing.expectEqualDeep(
        Val.init(42),
        vm.evalStr("x"),
    );
}

test "first returns first element of a pair" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const result = try vm.evalStr("(first (pair 1 2))");
    try testing.expectEqualDeep(
        Val.init(1),
        result,
    );
}

test "first returns WrongType for non-pair value" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    try testing.expectError(
        errors.Error.WrongType,
        vm.evalStr("(first 1)"),
    );
}

test "second returns second element of a pair" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const result = try vm.evalStr("(second (pair 1 2))");
    try testing.expectEqualDeep(
        Val.init(2),
        result,
    );
}

test "second returns WrongType for non-pair value" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    try testing.expectError(
        errors.Error.WrongType,
        vm.evalStr("(second 1)"),
    );
}

test "pair? returns true for pair" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.init(true),
        try vm.evalStr("(pair? (pair 1 2))"),
    );
}

test "pair? returns false for non-pair" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.init(false),
        try vm.evalStr("(pair? 1)"),
    );
    try testing.expectEqualDeep(
        Val.init(false),
        try vm.evalStr("(pair? (quote a))"),
    );
    try testing.expectEqualDeep(
        Val.init(false),
        try vm.evalStr("(pair? nil)"),
    );
    try testing.expectEqualDeep(
        Val.init(false),
        try vm.evalStr("(pair? \"hello\")"),
    );
}

test "pair? returns WrongArity error for wrong number of arguments" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectError(
        errors.Error.WrongArity,
        vm.evalStr("(pair?)"),
    );
    try testing.expectError(
        errors.Error.WrongArity,
        vm.evalStr("(pair? (pair 1 2) 3)"),
    );
}

test "number? returns true for integer" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.init(true),
        try vm.evalStr("(number? 123)"),
    );
}

test "number? returns true for float" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.init(true),
        try vm.evalStr("(number? 3.14)"),
    );
}

test "number? returns false for symbol" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.init(false),
        try vm.evalStr("(number? (quote hello))"),
    );
}

test "number? returns false for nil" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.init(false),
        try vm.evalStr("(number? nil)"),
    );
}

test "symbol? returns true for symbol" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.init(true),
        try vm.evalStr("(symbol? (quote my-symbol))"),
    );
}

test "symbol? returns false for integer" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.init(false),
        try vm.evalStr("(symbol? 123)"),
    );
}

test "symbol? returns false for nil" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.init(false),
        try vm.evalStr("(symbol? nil)"),
    );
}

test "null? returns true for nil" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.init(true),
        try vm.evalStr("(null? nil)"),
    );
}

test "null? returns false for integer" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.init(false),
        try vm.evalStr("(null? 1)"),
    );
}

test "null? returns false for symbol" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.init(false),
        try vm.evalStr("(null? (quote a))"),
    );
}

test "string? returns true for string" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.init(true),
        try vm.evalStr("(string? \"hello\")"),
    );
}

test "string? returns false for symbol" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.init(false),
        try vm.evalStr("(string? (quote hello))"),
    );
}

test "string? returns false for number" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.init(false),
        try vm.evalStr("(string? 123)"),
    );
}

test "- with no arguments is wrong arity" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectError(
        errors.Error.WrongArity,
        vm.evalStr("(-)"),
    );
}

test "- with one argument negates number" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.init(-5),
        try vm.evalStr("(- 5)"),
    );
    try testing.expectEqualDeep(
        Val.init(5),
        try vm.evalStr("(- -5)"),
    );
    try testing.expectEqualDeep(
        Val.init(-3.5),
        try vm.evalStr("(- 3.5)"),
    );
    try testing.expectError(
        errors.Error.WrongType,
        vm.evalStr("(- (quote a))"),
    );
}

test "- with two arguments subtracts args[1] from args[0]" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.init(3),
        try vm.evalStr("(- 5 2)"),
    );
    try testing.expectEqualDeep(
        Val.init(3.25),
        try vm.evalStr("(- 5.5 2.25)"),
    );
    try testing.expectEqualDeep(
        Val.init(2.5),
        try vm.evalStr("(- 5 2.5)"),
    );
    try testing.expectError(
        errors.Error.WrongType,
        vm.evalStr("(- 5 (quote a))"),
    );
}

test "- with multiple arguments subtracts args[1..] from args[0]." {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.init(4),
        try vm.evalStr("(- 10 1 2 3)"),
    );
    try testing.expectEqualDeep(
        Val.init(13.0),
        try vm.evalStr("(- 20.5 5 2.5)"),
    );
    try testing.expectError(
        errors.Error.WrongType,
        vm.evalStr("(- 10 1 (quote a))"),
    );
}

test "mod returns the modulus of two integers" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.init(1),
        try vm.evalStr("(mod 10 3)"),
    );
    try testing.expectEqualDeep(
        Val.init(2),
        try vm.evalStr("(mod -10 3)"),
    );
}

test "mod with non-integer returns type error" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectError(
        errors.Error.WrongType,
        vm.evalStr("(mod 10.0 3)"),
    );
    try testing.expectError(
        errors.Error.WrongType,
        vm.evalStr("(mod 10 (quote a))"),
    );
}

test "mod with division by zero returns division by zero error" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectError(
        errors.Error.DivisionByZero,
        vm.evalStr("(mod 10 0)"),
    );
}

test "mod with wrong arity returns wrong arity error" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectError(
        errors.Error.WrongArity,
        vm.evalStr("(mod 10)"),
    );
    try testing.expectError(
        errors.Error.WrongArity,
        vm.evalStr("(mod 10 3 1)"),
    );
}

test "= returns true for equal integers" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.init(true),
        try vm.evalStr("(= 5 5)"),
    );
}

test "= returns false for unequal integers" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.init(false),
        try vm.evalStr("(= 5 6)"),
    );
}

test "= returns true for equal floats" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.init(true),
        try vm.evalStr("(= 5.0 5.0)"),
    );
}

test "= returns false for unequal floats" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.init(false),
        try vm.evalStr("(= 5.0 5.1)"),
    );
}

test "= returns true for equal mixed int and float" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.init(true),
        try vm.evalStr("(= 5 5.0)"),
    );
    try testing.expectEqualDeep(
        Val.init(true),
        try vm.evalStr("(= 5.0 5)"),
    );
}

test "= returns false for unequal mixed int and float" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.init(false),
        try vm.evalStr("(= 5 5.1)"),
    );
    try testing.expectEqualDeep(
        Val.init(false),
        try vm.evalStr("(= 5.1 5)"),
    );
}

test "= returns WrongType for non-numeric arguments" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectError(
        errors.Error.WrongType,
        vm.evalStr("(= 5 (quote a))"),
    );
    try testing.expectError(
        errors.Error.WrongType,
        vm.evalStr("(= (quote a) 5)"),
    );
    try testing.expectError(
        errors.Error.WrongType,
        vm.evalStr("(= (quote a) (quote b))"),
    );
    try testing.expectError(
        errors.Error.WrongType,
        vm.evalStr("(= nil 5)"),
    );
    try testing.expectError(
        errors.Error.WrongType,
        vm.evalStr("(= 5 nil)"),
    );
}

test "= returns WrongArity error for wrong number of arguments" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectError(
        errors.Error.WrongArity,
        vm.evalStr("(= 5)"),
    );
    try testing.expectError(
        errors.Error.WrongArity,
        vm.evalStr("(= 5 6 7)"),
    );
    try testing.expectError(
        errors.Error.WrongArity,
        vm.evalStr("(=)"),
    );
}

test "/ divides two integers" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.init(2.0),
        try vm.evalStr("(/ 4 2)"),
    );
}

test "/ divides two floats" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.init(2.5),
        try vm.evalStr("(/ 5.0 2.0)"),
    );
}

test "/ divides mixed integer and float" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.init(2.5),
        try vm.evalStr("(/ 5 2.0)"),
    );
    try testing.expectEqualDeep(
        Val.init(2.5),
        try vm.evalStr("(/ 5.0 2)"),
    );
}

test "/ with one argument returns 1.0 divided by argument" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.init(0.5),
        try vm.evalStr("(/ 2)"),
    );
    try testing.expectEqualDeep(
        Val.init(1.0),
        try vm.evalStr("(/ 1.0)"),
    );
}

test "/ returns DivisionByZero for denominator 0" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectError(
        errors.Error.DivisionByZero,
        vm.evalStr("(/ 10 0)"),
    );
    try testing.expectError(
        errors.Error.DivisionByZero,
        vm.evalStr("(/ 0)"),
    );
}

test "/ returns WrongType for non-numeric arguments" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectError(
        errors.Error.WrongType,
        vm.evalStr("(/ 10 (quote a))"),
    );
    try testing.expectError(
        errors.Error.WrongType,
        vm.evalStr("(/ (quote a) 10)"),
    );
    try testing.expectError(
        errors.Error.WrongType,
        vm.evalStr("(/ (quote a))"),
    );
}

test "/ returns WrongArity error for wrong number of arguments" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectError(
        errors.Error.WrongArity,
        vm.evalStr("(/)"),
    );
    try testing.expectError(
        errors.Error.WrongArity,
        vm.evalStr("(/ 10 2 3)"),
    );
}

test "range creates a pair with two integers" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    
    try testing.expectEqualDeep(
        Val.init(0),
        try vm.evalStr("(first (range 0 5))"),
    );
    try testing.expectEqualDeep(
        Val.init(5),
        try vm.evalStr("(second (range 0 5))"),
    );
}

test "range works with negative integers" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    
    try testing.expectEqualDeep(
        Val.init(-3),
        try vm.evalStr("(first (range -3 2))"),
    );
    try testing.expectEqualDeep(
        Val.init(2),
        try vm.evalStr("(second (range -3 2))"),
    );
}

test "range returns WrongType for float arguments" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    
    try testing.expectError(
        errors.Error.WrongType,
        vm.evalStr("(range 0.0 5)"),
    );
    try testing.expectError(
        errors.Error.WrongType,
        vm.evalStr("(range 0 5.0)"),
    );
    try testing.expectError(
        errors.Error.WrongType,
        vm.evalStr("(range 0.5 5.5)"),
    );
}

test "range returns WrongType for non-numeric arguments" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    
    try testing.expectError(
        errors.Error.WrongType,
        vm.evalStr("(range (quote a) 5)"),
    );
    try testing.expectError(
        errors.Error.WrongType,
        vm.evalStr("(range 0 (quote b))"),
    );
    try testing.expectError(
        errors.Error.WrongType,
        vm.evalStr("(range \"hello\" 5)"),
    );
}

test "range returns WrongArity error for wrong number of arguments" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    
    try testing.expectError(
        errors.Error.WrongArity,
        vm.evalStr("(range)"),
    );
    try testing.expectError(
        errors.Error.WrongArity,
        vm.evalStr("(range 0)"),
    );
    try testing.expectError(
        errors.Error.WrongArity,
        vm.evalStr("(range 0 5 10)"),
    );
}

test "range can be used in for loops like pair" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    
    // Test that range works in a for loop by accumulating values
    const result = try vm.evalStr(
        \\(def sum 0)
        \\(for (x (range 1 4))
        \\  (def sum (+ sum x)))
        \\sum
    );
    try testing.expectEqualDeep(
        Val.init(6), // 1 + 2 + 3 = 6
        result,
    );
}
