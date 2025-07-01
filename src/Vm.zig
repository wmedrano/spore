const std = @import("std");
const testing = std.testing;
const Val = @import("Val.zig");

const Vm = @This();
const MAX_STACK_SIZE = 1024;

stack_len: usize = 0,
stack_data: [MAX_STACK_SIZE]Val = undefined,

pub fn init() Vm {
    return .{};
}

pub fn stack(self: Vm) []const Val {
    return self.stack_data[0..self.stack_len];
}

pub fn pushVal(self: *Vm, val: Val) void {
    self.stack_data[self.stack_len] = val;
    self.stack_len += 1;
}

pub fn pushVals(self: *Vm, vals: []const Val) void {
    for (vals) |v| {
        self.pushVal(v);
    }
}

pub fn popVal(self: *Vm) Val {
    self.stack_len -= 1;
    return self.stack_data[self.stack_len];
}

test "initial stack is empty" {
    const vm = Vm.init();
    try testing.expectFmt("{  }", "{any}", .{vm.stack()});
}

test "push val adds to stack" {
    var vm = Vm.init();
    vm.pushVals(&.{ Val.new_int(1), Val.new_int(2) });
    try testing.expectFmt("{ 1, 2 }", "{any}", .{vm.stack()});
}

test "pop val removes from stack" {
    var vm = Vm.init();
    vm.pushVals(&.{ Val.new_int(1), Val.new_int(2), Val.new_int(3) });
    try testing.expectEqualDeep(Val.new_int(3), vm.popVal());
    try testing.expectFmt("{ 1, 2 }", "{any}", .{vm.stack()});
}
