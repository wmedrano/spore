const std = @import("std");
const testing = std.testing;

const ConsCell = @import("ConsCell.zig");
const Handle = @import("datastructures/object_pool.zig").Handle;
const Symbol = @import("datastructures/Symbol.zig");
const NativeFunction = @import("NativeFunction.zig");
const String = @import("String.zig");
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
    try cons.register(vm);
    try car.register(vm);
    try cons_q.register(vm);
    try cdr.register(vm);
    try list.register(vm);
    try empty_q.register(vm);
    try equal_q.register(vm);
}

const number_q = NativeFunction{
    .name = "number?",
    .docstring = "Returns true if the argument is a number (integer or float), false otherwise.",
    .ptr = numberQImpl,
};

fn numberQImpl(vm: *Vm) NativeFunction.Error!Val {
    const args = vm.execution_context.localStack();
    if (args.len != 1) return NativeFunction.Error.WrongArity;
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

fn symbolQImpl(vm: *Vm) NativeFunction.Error!Val {
    const args = vm.execution_context.localStack();
    if (args.len != 1) return NativeFunction.Error.WrongArity;
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

fn nullQImpl(vm: *Vm) NativeFunction.Error!Val {
    const args = vm.execution_context.localStack();
    if (args.len != 1) return NativeFunction.Error.WrongArity;
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

fn stringQImpl(vm: *Vm) NativeFunction.Error!Val {
    const args = vm.execution_context.localStack();
    if (args.len != 1) return NativeFunction.Error.WrongArity;
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

fn toStringImpl(vm: *Vm) NativeFunction.Error!Val {
    const args = vm.execution_context.localStack();
    if (args.len != 1) return NativeFunction.Error.WrongArity;
    var buffer = std.ArrayList(u8).init(vm.heap.allocator);
    defer buffer.deinit();
    const val = vm.inspector().pretty(args[0]);
    try val.format("any", .{}, buffer.writer());
    const return_val = try vm.builder().string(buffer.items);
    return return_val;
}

const print = NativeFunction{
    .name = "print",
    .docstring = "Prints all arguments.",
    .ptr = printImpl,
};

fn printImpl(vm: *Vm) NativeFunction.Error!Val {
    const args = vm.execution_context.localStack();
    var buffer = std.ArrayList(u8).init(vm.heap.allocator);
    defer buffer.deinit();
    const vals = vm.inspector().prettySlice(args);
    std.debug.print("{any}", .{vals});
    return Val.init({});
}

const println = NativeFunction{
    .name = "println",
    .docstring = "Prints all arguments followed by a newline.",
    .ptr = printlnImpl,
};

fn printlnImpl(vm: *Vm) NativeFunction.Error!Val {
    const args = vm.execution_context.localStack();
    var buffer = std.ArrayList(u8).init(vm.heap.allocator);
    defer buffer.deinit();
    const vals = vm.inspector().prettySlice(args);
    std.debug.print("{any}\n", .{vals});
    return Val.init({});
}

const add = NativeFunction{
    .name = "+",
    .docstring = "Adds all values on the local stack. It can handle integers, " ++
        "floats, or a mix of both. If any floats are present, the result will be a float.",
    .ptr = addImpl,
};

fn addSlice(vals: []const Val) NativeFunction.Error!Val {
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
            else => return NativeFunction.Error.TypeError,
        }
    }
    if (has_float) {
        const sum_as_float: f64 = float_sum + @as(f64, @floatFromInt(int_sum));
        return Val.init(sum_as_float);
    }
    return Val.init(int_sum);
}

fn addImpl(vm: *Vm) NativeFunction.Error!Val {
    return try addSlice(vm.execution_context.localStack());
}

const subtract = NativeFunction{
    .name = "-",
    .docstring = "Subtracts values. With one argument, negates it. With two, subtracts the second from the first. With more, subtracts the sum of the rest from the first.",
    .ptr = subtractImpl,
};

fn negate(val: Val) !Val {
    switch (val.repr) {
        .int => |x| return Val.init(-x),
        .float => |x| return Val.init(-x),
        else => return NativeFunction.Error.TypeError,
    }
}

fn subtractTwo(val1: Val, val2: Val) NativeFunction.Error!Val {
    const x1 = val1.repr;
    const x2 = val2.repr;

    switch (x1) {
        .int => |int1| {
            switch (x2) {
                .int => |int2| return Val.init(int1 - int2),
                .float => |float2| return Val.init(@as(f64, @floatFromInt(int1)) - float2),
                else => return NativeFunction.Error.TypeError,
            }
        },
        .float => |float1| {
            switch (x2) {
                .int => |int2| return Val.init(float1 - @as(f64, @floatFromInt(int2))),
                .float => |float2| return Val.init(float1 - float2),
                else => return NativeFunction.Error.TypeError,
            }
        },
        else => return NativeFunction.Error.TypeError,
    }
}

fn subtractImpl(vm: *Vm) NativeFunction.Error!Val {
    const args = vm.execution_context.localStack();
    switch (args.len) {
        0 => return NativeFunction.Error.WrongArity,
        1 => return try negate(args[0]),
        2 => return try subtractTwo(args[0], args[1]),
        else => return try subtractTwo(
            args[0],
            try addSlice(args[1..]),
        ),
    }
}

const multiply = NativeFunction{
    .name = "*",
    .docstring = "Multiplies all values on the local stack. It can handle integers, " ++
        "floats, or a mix of both. If any floats are present, the result will be a float.",
    .ptr = multiplyImpl,
};

fn multiplySlice(vals: []const Val) NativeFunction.Error!Val {
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
            else => return NativeFunction.Error.TypeError,
        }
    }
    if (has_float) {
        const product_as_float: f64 = float_product * @as(f64, @floatFromInt(int_product));
        return Val.init(product_as_float);
    }
    return Val.init(int_product);
}

fn multiplyImpl(vm: *Vm) NativeFunction.Error!Val {
    return try multiplySlice(vm.execution_context.localStack());
}

const mod = NativeFunction{
    .name = "mod",
    .docstring = "Returns the modulus of two integers. Example: `(mod 10 3)` returns `1`.",
    .ptr = modImpl,
};

fn toFloat(val: Val) NativeFunction.Error!f64 {
    switch (val.repr) {
        .int => |x| return @floatFromInt(x),
        .float => |x| return x,
        else => return NativeFunction.Error.TypeError,
    }
}

const divide = NativeFunction{
    .name = "/",
    .docstring = "Divides numbers. With one argument, returns 1.0 divided by the argument. With two, divides the first by the second. Returns a float.",
    .ptr = divideImpl,
};

fn divideImpl(vm: *Vm) NativeFunction.Error!Val {
    const args = vm.execution_context.localStack();
    switch (args.len) {
        1 => {
            const denominator = try toFloat(args[0]);
            if (denominator == 0.0) return NativeFunction.Error.DivisionByZero;
            return Val.init(1.0 / denominator);
        },
        2 => return {
            const numerator = try toFloat(args[0]);
            const denominator = try toFloat(args[1]);
            if (denominator == 0.0) return NativeFunction.Error.DivisionByZero;
            return Val.init(numerator / denominator);
        },
        else => return NativeFunction.Error.WrongArity,
    }
}

fn modImpl(vm: *Vm) NativeFunction.Error!Val {
    const args = vm.execution_context.localStack();
    if (args.len != 2) return NativeFunction.Error.WrongArity;
    const a = try args[0].to(i64);
    const b = try args[1].to(i64);
    if (b == 0) return NativeFunction.Error.DivisionByZero;
    return Val.init(@mod(a, b));
}

const equal_q = NativeFunction{
    .name = "=",
    .docstring = "Returns true if two numbers are equal, nil otherwise. Returns a TypeError for non-numeric arguments.",
    .ptr = equalQImpl,
};

fn equalQImpl(vm: *Vm) NativeFunction.Error!Val {
    const args = vm.execution_context.localStack();
    if (args.len != 2) return NativeFunction.Error.WrongArity;

    switch (args[0].repr) {
        .int => |int1| {
            switch (args[1].repr) {
                .int => |int2| return Val.init(int1 == int2),
                .float => |float2| return Val.init(@as(f64, @floatFromInt(int1)) == float2),
                else => return NativeFunction.Error.TypeError,
            }
        },
        .float => |float1| {
            switch (args[1].repr) {
                .int => |int2| return Val.init(float1 == @as(f64, @floatFromInt(int2))),
                .float => |float2| return Val.init(float1 == float2),
                else => return NativeFunction.Error.TypeError,
            }
        },
        else => return NativeFunction.Error.TypeError,
    }
}

const internal_define = NativeFunction{
    .name = "internal-define",
    .docstring = "Defines a global variable. It expects two arguments: a symbol " ++
        "(the name of the variable) and a value.",
    .ptr = internalDefineImpl,
};

fn internalDefineImpl(vm: *Vm) NativeFunction.Error!Val {
    const args = vm.execution_context.localStack();
    if (args.len != 2) return NativeFunction.Error.WrongArity;
    const symbol = try args[0].to(Symbol.Interned);
    const value = args[1];
    try vm.execution_context.setGlobal(vm.heap.allocator, symbol, value);
    return Val.init({});
}

const cons = NativeFunction{
    .name = "cons",
    .docstring = "Returns a cons cell (pair) with 2 values.",
    .ptr = consImpl,
};

fn consImpl(vm: *Vm) NativeFunction.Error!Val {
    const args = vm.execution_context.localStack();
    if (args.len != 2) return NativeFunction.Error.WrongArity;
    return vm.builder().cons(args[0], args[1]);
}

const car = NativeFunction{
    .name = "car",
    .docstring = "Returns the car (first element) of a Cons cell.",
    .ptr = carImpl,
};

fn carImpl(vm: *Vm) NativeFunction.Error!Val {
    const args = vm.execution_context.localStack();
    if (args.len != 1) return NativeFunction.Error.WrongArity;
    const cons_handle = try args[0].to(Handle(ConsCell));
    const cons_cell = try vm.heap.cons_cells.get(cons_handle);
    return cons_cell.car;
}

const cdr = NativeFunction{
    .name = "cdr",
    .docstring = "Returns the cdr (second element) of a Cons cell.",
    .ptr = cdrImpl,
};

fn cdrImpl(vm: *Vm) NativeFunction.Error!Val {
    const args = vm.execution_context.localStack();
    if (args.len != 1) return NativeFunction.Error.WrongArity;
    const cons_handle = try args[0].to(Handle(ConsCell));
    const cons_cell = try vm.heap.cons_cells.get(cons_handle);
    return cons_cell.cdr;
}

const cons_q = NativeFunction{
    .name = "cons?",
    .docstring = "Returns true if the argument is a cons cell, false otherwise.",
    .ptr = consQImpl,
};

fn consQImpl(vm: *Vm) NativeFunction.Error!Val {
    const args = vm.execution_context.localStack();
    if (args.len != 1) return NativeFunction.Error.WrongArity;
    const val = args[0];
    switch (val.repr) {
        .cons => return Val.init(true),
        else => return Val.init(false),
    }
}

const list = NativeFunction{
    .name = "list",
    .docstring = "Returns a new list containing all provided arguments.",
    .ptr = listImpl,
};

fn listImpl(vm: *Vm) NativeFunction.Error!Val {
    const args = vm.execution_context.localStack();
    return vm.builder().list(args);
}

const empty_q = NativeFunction{
    .name = "empty?",
    .docstring = "Returns true if the argument is nil (representing an empty list), false if it's a cons cell, and throws an error for othre types.",
    .ptr = emptyQImpl,
};

fn emptyQImpl(vm: *Vm) NativeFunction.Error!Val {
    const args = vm.execution_context.localStack();
    if (args.len != 1) return NativeFunction.Error.WrongArity;
    switch (args[0].repr) {
        .nil => return Val.init(true),
        .cons => return Val.init(false),
        else => return NativeFunction.Error.TypeError,
    }
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

    _ = try vm.evalStr("(internal-define 'x 42)");
    try testing.expectEqualDeep(
        Val.init(42),
        vm.evalStr("x"),
    );
}

test "car returns first element of a cons cell" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const result = try vm.evalStr("(car (cons 1 2))");
    try testing.expectEqualDeep(
        Val.init(1),
        result,
    );
}

test "car returns TypeError for non-cons cell value" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    try testing.expectError(
        NativeFunction.Error.TypeError,
        vm.evalStr("(car 1)"),
    );
}

test "cdr returns second element of a cons cell" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const result = try vm.evalStr("(cdr (cons 1 2))");
    try testing.expectEqualDeep(
        Val.init(2),
        result,
    );
}

test "cdr returns TypeError for non-cons cell value" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    try testing.expectError(
        NativeFunction.Error.TypeError,
        vm.evalStr("(cdr 1)"),
    );
}

test "cons? returns true for cons cell" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.init(true),
        try vm.evalStr("(cons? (cons 1 2))"),
    );
}

test "cons? returns false for non-cons cell" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.init(false),
        try vm.evalStr("(cons? 1)"),
    );
    try testing.expectEqualDeep(
        Val.init(false),
        try vm.evalStr("(cons? 'a)"),
    );
    try testing.expectEqualDeep(
        Val.init(false),
        try vm.evalStr("(cons? nil)"),
    );
    try testing.expectEqualDeep(
        Val.init(false),
        try vm.evalStr("(cons? \"hello\")"),
    );
}

test "cons? returns WrongArity error for wrong number of arguments" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectError(
        NativeFunction.Error.WrongArity,
        vm.evalStr("(cons?)"),
    );
    try testing.expectError(
        NativeFunction.Error.WrongArity,
        vm.evalStr("(cons? (cons 1 2) 3)"),
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
        try vm.evalStr("(number? 'hello)"),
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
        try vm.evalStr("(symbol? 'my-symbol)"),
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
        try vm.evalStr("(null? 'a)"),
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
        try vm.evalStr("(string? 'hello)"),
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
        NativeFunction.Error.WrongArity,
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
        NativeFunction.Error.TypeError,
        vm.evalStr("(- 'a)"),
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
        NativeFunction.Error.TypeError,
        vm.evalStr("(- 5 'a)"),
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
        NativeFunction.Error.TypeError,
        vm.evalStr("(- 10 1 'a)"),
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
        NativeFunction.Error.TypeError,
        vm.evalStr("(mod 10.0 3)"),
    );
    try testing.expectError(
        NativeFunction.Error.TypeError,
        vm.evalStr("(mod 10 'a)"),
    );
}

test "mod with division by zero returns division by zero error" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectError(
        NativeFunction.Error.DivisionByZero,
        vm.evalStr("(mod 10 0)"),
    );
}

test "mod with wrong arity returns wrong arity error" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectError(
        NativeFunction.Error.WrongArity,
        vm.evalStr("(mod 10)"),
    );
    try testing.expectError(
        NativeFunction.Error.WrongArity,
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

test "= returns TypeError for non-numeric arguments" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectError(
        NativeFunction.Error.TypeError,
        vm.evalStr("(= 5 'a)"),
    );
    try testing.expectError(
        NativeFunction.Error.TypeError,
        vm.evalStr("(= 'a 5)"),
    );
    try testing.expectError(
        NativeFunction.Error.TypeError,
        vm.evalStr("(= 'a 'b)"),
    );
    try testing.expectError(
        NativeFunction.Error.TypeError,
        vm.evalStr("(= nil 5)"),
    );
    try testing.expectError(
        NativeFunction.Error.TypeError,
        vm.evalStr("(= 5 nil)"),
    );
}

test "= returns WrongArity error for wrong number of arguments" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectError(
        NativeFunction.Error.WrongArity,
        vm.evalStr("(= 5)"),
    );
    try testing.expectError(
        NativeFunction.Error.WrongArity,
        vm.evalStr("(= 5 6 7)"),
    );
    try testing.expectError(
        NativeFunction.Error.WrongArity,
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
        NativeFunction.Error.DivisionByZero,
        vm.evalStr("(/ 10 0)"),
    );
    try testing.expectError(
        NativeFunction.Error.DivisionByZero,
        vm.evalStr("(/ 0)"),
    );
}

test "/ returns TypeError for non-numeric arguments" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectError(
        NativeFunction.Error.TypeError,
        vm.evalStr("(/ 10 'a)"),
    );
    try testing.expectError(
        NativeFunction.Error.TypeError,
        vm.evalStr("(/ 'a 10)"),
    );
    try testing.expectError(
        NativeFunction.Error.TypeError,
        vm.evalStr("(/ 'a)"),
    );
}

test "/ returns WrongArity error for wrong number of arguments" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectError(
        NativeFunction.Error.WrongArity,
        vm.evalStr("(/)"),
    );
    try testing.expectError(
        NativeFunction.Error.WrongArity,
        vm.evalStr("(/ 10 2 3)"),
    );
}
