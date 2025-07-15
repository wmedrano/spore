const std = @import("std");
const testing = std.testing;

const ConsCell = @import("ConsCell.zig");
const Handle = @import("datastructures/object_pool.zig").Handle;
const Symbol = @import("datastructures/Symbol.zig");
const NativeFunction = @import("NativeFunction.zig");
const Val = @import("Val.zig");
const Vm = @import("Vm.zig");

/// Registers all built-in native functions with the provided Vm.
pub fn registerAll(vm: *Vm) !void {
    try number_q.register(vm);
    try symbol_q.register(vm);
    try null_q.register(vm);
    try add.register(vm);
    try define.register(vm);
    try cons.register(vm);
    try car.register(vm);
    try cdr.register(vm);
    try list.register(vm);
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
        .int, .float => return Val.from(true),
        else => return Val.from(false),
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
        .symbol => return Val.from(true),
        else => return Val.from(false),
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
        .nil => return Val.from(true),
        else => return Val.from(false),
    }
}

const add = NativeFunction{
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

const define = NativeFunction{
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

const cons = NativeFunction{
    .name = "cons",
    .docstring = "Returns a cons cell (pair) with 2 values.",
    .ptr = consImpl,
};

fn consImpl(vm: *Vm) NativeFunction.Error!Val {
    const args = vm.execution_context.localStack();
    if (args.len != 2) return NativeFunction.Error.WrongArity;
    const cons_cell = ConsCell.init(args[0], args[1]);
    const cons_handle = try vm.heap.cons_cells.create(vm.heap.allocator, cons_cell);
    return Val.from(cons_handle);
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

const list = NativeFunction{
    .name = "list",
    .docstring = "Returns a new list containing all provided arguments.",
    .ptr = listImpl,
};

fn listImpl(vm: *Vm) NativeFunction.Error!Val {
    const args = vm.execution_context.localStack();
    return buildListFromVals(args, vm);
}

// Helper function, adapted from Reader.zig's listToVal
fn buildListFromVals(vals: []const Val, vm: *Vm) NativeFunction.Error!Val {
    if (vals.len == 0) return Val.from({}); // Base case: empty list is nil
    const head = vals[0];
    const tail = try buildListFromVals(vals[1..], vm); // Recursive call for the rest of the list
    const cons_cell = ConsCell.init(head, tail);
    const cons_handle = try vm.heap.cons_cells.create(vm.heap.allocator, cons_cell);
    return Val.from(cons_handle);
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

test "car returns first element of a cons cell" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const result = try vm.evalStr("(car (cons 1 2))");
    try testing.expectEqualDeep(
        Val.from(1),
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
        Val.from(2),
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

test "number? returns true for integer" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.from(true),
        try vm.evalStr("(number? 123)"),
    );
}

test "number? returns true for float" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.from(true),
        try vm.evalStr("(number? 3.14)"),
    );
}

test "number? returns false for symbol" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.from(false),
        try vm.evalStr("(number? 'hello)"),
    );
}

test "number? returns false for nil" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.from(false),
        try vm.evalStr("(number? nil)"),
    );
}

test "symbol? returns true for symbol" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.from(true),
        try vm.evalStr("(symbol? 'my-symbol)"),
    );
}

test "symbol? returns false for integer" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.from(false),
        try vm.evalStr("(symbol? 123)"),
    );
}

test "symbol? returns false for nil" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.from(false),
        try vm.evalStr("(symbol? nil)"),
    );
}

test "null? returns true for nil" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.from(true),
        try vm.evalStr("(null? nil)"),
    );
}

test "null? returns false for integer" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.from(false),
        try vm.evalStr("(null? 1)"),
    );
}

test "null? returns false for symbol" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.from(false),
        try vm.evalStr("(null? 'a)"),
    );
}
