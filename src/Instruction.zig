//! Contains a single instruction for a `Vm` to execute.
const std = @import("std");
const testing = std.testing;

const Val = @import("Val.zig");
const Vm = @import("Vm.zig");
const PrettyPrinter = @import("PrettyPrinter.zig");
const Symbol = @import("datastructures/Symbol.zig");

const Instruction = @This();

/// The internal representation of an instruction.
repr: Repr,

/// An instruction for a `Vm` to execute.
pub const Repr = union(enum) {
    /// Push a new value onto the stack.
    push: Val,
    /// Get the value of the symbol and push it on the stack.
    get: Symbol.Interned,
    /// Evaluate the top n values of the stack as a function call.
    eval: usize,
};

/// Intialize an instruction from its internal representation.
pub fn init(repr: Repr) Instruction {
    return .{ .repr = repr };
}

/// Execute `self` on `vm`.
pub fn execute(self: Instruction, vm: *Vm) !void {
    switch (self.repr) {
        .push => |v| try vm.execution_context.pushVal(v),
        .get => |s| {
            const val = vm.execution_context.getGlobal(s) orelse return error.SymbolNotFound;
            try vm.execution_context.pushVal(val);
        },
        .eval => return error.NotImplemented,
    }
}

test "push val pushes to stack" {
    var vm = Vm.init(testing.allocator);
    defer vm.deinit();
    try init(.{ .push = Val.from(42) }).execute(&vm);
    try init(.{ .push = Val.from(43) }).execute(&vm);

    try testing.expectFmt(
        "42 43",
        "{}",
        .{PrettyPrinter.initSlice(&vm, vm.execution_context.stack.constSlice())},
    );
}

test "get symbol pushes value referred to by symbol onto stack" {
    var vm = Vm.init(testing.allocator);
    defer vm.deinit();
    const symbol = try Symbol.init("my-var").intern(vm.heap.allocator, &vm.heap.string_interner);
    try vm.execution_context.setGlobal(vm.heap.allocator, symbol, Val.from(123));

    try init(.{ .get = symbol }).execute(&vm);

    try testing.expectFmt(
        "123",
        "{}",
        .{PrettyPrinter.initSlice(&vm, vm.execution_context.stack.constSlice())},
    );
}
