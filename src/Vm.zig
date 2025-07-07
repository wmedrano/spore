const std = @import("std");
const testing = std.testing;
const Val = @import("Val.zig");

const Vm = @This();
const MAX_STACK_SIZE = 1024;
const Tokenizer = @import("parser/Tokenizer.zig");

stack_len: usize = 0,
stack_data: [MAX_STACK_SIZE]Val = undefined,

// Create a new VM.
pub fn init() Vm {
    return .{};
}

pub fn eval_str(src: []const u8) !void {
    _ = Tokenizer.init(src);
    return error.NotImplemented;
}

// Get the complete stack for the `Vm`. The last value in the slice corresponds
// to the top of the stack.
pub fn stack(self: Vm) []const Val {
    return self.stack_data[0..self.stack_len];
}

// Push `val` onto the stack.
pub fn pushVal(self: *Vm, val: Val) !void {
    if (self.stack_len == MAX_STACK_SIZE) {
        return error.StackOverflow;
    }
    self.stack_data[self.stack_len] = val;
    self.stack_len += 1;
}

// Push several `vals` onto the Vm. The last value of `vals` will be on top.
pub fn pushVals(self: *Vm, vals: []const Val) !void {
    for (vals) |v| {
        try self.pushVal(v);
    }
}

// Pop a `Val` from thes stack.
pub fn popVal(self: *Vm) !Val {
    if (self.stack_len == 0) {
        return error.StackUnderflow;
    }
    self.stack_len -= 1;
    return self.stack_data[self.stack_len];
}

test "initial stack is empty" {
    const vm = Vm.init();
    try testing.expectFmt("{  }", "{any}", .{vm.stack()});
}

test "push val adds to stack" {
    var vm = Vm.init();
    try vm.pushVals(&.{ Val.new_int(1), Val.new_int(2) });
    try testing.expectFmt("{ 1, 2 }", "{any}", .{vm.stack()});
}

test "push to many vals returns stack overflow" {
    var vm = Vm.init();
    for (0..MAX_STACK_SIZE) |_| {
        try vm.pushVal(Val.new_int(1));
    }
    try testing.expectError(error.StackOverflow, vm.pushVal(Val.new_int(1)));
}

test "pop val removes from stack" {
    var vm = Vm.init();
    try vm.pushVals(&.{ Val.new_int(1), Val.new_int(2), Val.new_int(3) });
    try testing.expectEqualDeep(Val.new_int(3), vm.popVal());
    try testing.expectFmt("{ 1, 2 }", "{any}", .{vm.stack()});
}

test "pop empty stack returns stack underflow" {
    var vm = Vm.init();
    try testing.expectError(error.StackUnderflow, vm.popVal());
}
