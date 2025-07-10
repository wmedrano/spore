const std = @import("std");
const testing = std.testing;

const StringInterner = @import("datastructures/StringInterner.zig");
const Symbol = @import("datastructures/Symbol.zig");
const Tokenizer = @import("parser/Tokenizer.zig");
const Val = @import("Val.zig");
const Vm = @import("Vm.zig");

/// Manages the execution state of the VM, including the data stack.
const ExecutionContext = @This();

/// The maximum number of values that can be on the stack.
const MAX_STACK_SIZE = 1024;
/// The current number of values on the stack.
stack_len: usize = 0,
/// The backing array for the stack.
stack_data: [MAX_STACK_SIZE]Val = undefined,
/// A map from symbol to its value in the global namespace.
global_values: std.AutoHashMapUnmanaged(Symbol.Interned, Val) = .{},

/// Deinitialize self and free all memory.
pub fn deinit(self: *ExecutionContext, allocator: std.mem.Allocator) void {
    self.global_values.deinit(allocator);
}

/// Get the complete stack for the `ExecutionContext`. The last value in the slice corresponds
/// to the top of the stack.
pub fn stack(self: ExecutionContext) []const Val {
    return self.stack_data[0..self.stack_len];
}

/// Push `val` onto the stack.
pub fn pushVal(self: *ExecutionContext, val: Val) !void {
    if (self.stack_len == MAX_STACK_SIZE) {
        return error.StackOverflow;
    }
    self.stack_data[self.stack_len] = val;
    self.stack_len += 1;
}

/// Push several `vals` onto the ExecutionContext. The last value of `vals` will be on top.
pub fn pushVals(self: *ExecutionContext, vals: []const Val) !void {
    for (vals) |v| {
        try self.pushVal(v);
    }
}

/// Pop a `Val` from the stack.
pub fn popVal(self: *ExecutionContext) !Val {
    if (self.stack_len == 0) {
        return error.StackUnderflow;
    }
    self.stack_len -= 1;
    return self.stack_data[self.stack_len];
}

/// Set a global value. `symbol` will refer to `val`.
pub fn setGlobal(self: *ExecutionContext, allocator: std.mem.Allocator, symbol: Symbol.Interned, val: Val) !void {
    try self.global_values.put(allocator, symbol, val);
}

/// Get the value of `symbol`.
pub fn getGlobal(self: ExecutionContext, symbol: Symbol.Interned) ?Val {
    return self.global_values.get(symbol);
}

test "initial stack is empty" {
    var ctx = ExecutionContext{};
    try testing.expectFmt("{  }", "{any}", .{ctx.stack()});
}

test "push val adds to stack" {
    var ctx = ExecutionContext{};
    try ctx.pushVals(&.{ Val.from(1), Val.from(2) });
    try testing.expectFmt("{ 1, 2 }", "{any}", .{ctx.stack()});
}

test "push to many vals returns stack overflow" {
    var ctx = ExecutionContext{};
    for (0..MAX_STACK_SIZE) |_| {
        try ctx.pushVal(Val.from(1));
    }

    try testing.expectError(
        error.StackOverflow,
        ctx.pushVal(Val.from(1)),
    );
}

test "pop val removes from stack" {
    var ctx = ExecutionContext{};
    try ctx.pushVals(&.{ Val.from(1), Val.from(2), Val.from(3) });

    try testing.expectEqualDeep(
        Val.from(3),
        ctx.popVal(),
    );
    try testing.expectFmt(
        "{ 1, 2 }",
        "{any}",
        .{ctx.stack()},
    );
}

test "pop empty stack returns stack underflow" {
    var ctx = ExecutionContext{};
    try testing.expectError(
        error.StackUnderflow,
        ctx.popVal(),
    );
}

test "getGlobal on non-existant symbol returns null" {
    var vm = Vm.init(testing.allocator);
    defer vm.deinit();
    const symbol = try Symbol.init("my-var").intern(
        vm.heap.allocator,
        &vm.heap.string_interner,
    );

    try testing.expectEqualDeep(
        null,
        vm.execution_context.getGlobal(symbol),
    );
}

test "getGlobal on symbol registered with setGlobal returns that symbol" {
    var vm = Vm.init(testing.allocator);
    defer vm.deinit();
    const symbol = try Symbol.init("my-var").intern(vm.heap.allocator, &vm.heap.string_interner);
    try vm.execution_context.setGlobal(vm.heap.allocator, symbol, Val.from(123));

    try testing.expectEqualDeep(
        Val.from(123),
        vm.execution_context.getGlobal(symbol),
    );
}
