const std = @import("std");
const testing = std.testing;

const StringInterner = @import("datastructures/StringInterner.zig");
const Symbol = @import("datastructures/Symbol.zig");
const Tokenizer = @import("parser/Tokenizer.zig");
const Val = @import("Val.zig");

/// Manages the execution state of the VM, including the data stack.
const ExecutionContext = @This();

/// The maximum number of values that can be on the stack.
const MAX_STACK_SIZE = 1024;
/// The current number of values on the stack.
stack_len: usize = 0,
/// The backing array for the stack.
stack_data: [MAX_STACK_SIZE]Val = undefined,

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

test "initial stack is empty" {
    var ctx = ExecutionContext{};
    try testing.expectFmt("{  }", "{any}", .{ctx.stack()});
}

test "push val adds to stack" {
    var ctx = ExecutionContext{};
    try ctx.pushVals(&.{ Val.init(1), Val.init(2) });
    try testing.expectFmt("{ 1, 2 }", "{any}", .{ctx.stack()});
}

test "push to many vals returns stack overflow" {
    var ctx = ExecutionContext{};
    for (0..MAX_STACK_SIZE) |_| {
        try ctx.pushVal(Val.init(1));
    }
    try testing.expectError(error.StackOverflow, ctx.pushVal(Val.init(1)));
}

test "pop val removes from stack" {
    var ctx = ExecutionContext{};
    try ctx.pushVals(&.{ Val.init(1), Val.init(2), Val.init(3) });
    try testing.expectEqualDeep(Val.init(3), ctx.popVal());
    try testing.expectFmt("{ 1, 2 }", "{any}", .{ctx.stack()});
}

test "pop empty stack returns stack underflow" {
    var ctx = ExecutionContext{};
    try testing.expectError(error.StackUnderflow, ctx.popVal());
}
