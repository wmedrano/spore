//! Contains a single instruction for a `Vm` to execute.
const std = @import("std");
const testing = std.testing;

const Val = @import("Val.zig");
const Vm = @import("Vm.zig");
const PrettyPrinter = @import("PrettyPrinter.zig");
const Symbol = @import("datastructures/Symbol.zig");
const ExecutionContext = @import("ExecutionContext.zig");

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
        .eval => |n| try executeEval(vm, n),
    }
}

/// Evaluate the top `n` items on the stack as a function call.
///
/// The first of the `n` items is the function to be called, and the remaining
/// `n - 1` items are the arguments. After the call, the function and its
/// arguments are replaced on the stack with the single return value.
fn executeEval(vm: *Vm, n: usize) !void {
    const function_idx = vm.execution_context.stack.len - n;
    const val = vm.execution_context.stack.get(function_idx);
    vm.execution_context.call_frames.append(ExecutionContext.CallFrame{
        .instructions = &.{},
        .instruction_index = 0,
        .stack_start = function_idx + 1,
    }) catch return error.StackOverflow;
    switch (val.repr) {
        .function => |handle| {
            const function = try vm.heap.functions.get(handle);
            const result = try function.call(vm);
            try vm.execution_context.stack.resize(function_idx + 1);
            vm.execution_context.stack.set(function_idx, result);
            vm.execution_context.call_frames.pop() orelse return error.StackUnderflow;
        },
        else => return error.WrongType,
    }
}

test "push val pushes to stack" {
    var vm = try Vm.init(testing.allocator);
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
    var vm = try Vm.init(testing.allocator);
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

test "eval calls function" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    const plus = try Symbol.init("+").intern(vm.heap.allocator, &vm.heap.string_interner);
    try init(.{ .get = plus }).execute(&vm);
    try init(.{ .push = Val.from(10) }).execute(&vm);
    try init(.{ .push = Val.from(20) }).execute(&vm);
    try init(.{ .eval = 3 }).execute(&vm);
    try testing.expectFmt(
        "30",
        "{}",
        .{PrettyPrinter.initSlice(&vm, vm.execution_context.stack.constSlice())},
    );
}

test "eval on non function produces WrongTypeError" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try init(.{ .push = Val.from(123) }).execute(&vm);
    try testing.expectError(
        error.WrongType,
        init(.{ .eval = 1 }).execute(&vm),
    );
}
