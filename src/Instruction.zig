//! Contains a single instruction for a `Vm` to execute.
const std = @import("std");
const testing = @import("std").testing;

const Val = @import("Val.zig");
const Vm = @import("Vm.zig");
const PrettyPrinter = @import("PrettyPrinter.zig");
const Symbol = @import("datastructures/Symbol.zig");
const ExecutionContext = @import("ExecutionContext.zig");

const Instruction = @This();

/// The internal representation of an instruction.
repr: Repr,

pub const Error = error{
    ObjectNotFound,
    StackOverflow,
    StackUnderflow,
    SymbolNotFound,
    TypeError,
    WrongArity,
} || std.mem.Allocator.Error;

pub const Code = enum {
    /// Push a new value onto the stack.
    push,
    /// Get the value of the symbol and push it on the stack.
    get,
    /// Skip the next n instructions.
    jump,
    /// Pop the top value of the stack and skip the next `n` instructions if the
    /// value is truthy.
    jump_if,
    /// Evaluate the top n values of the stack as a function call.
    eval,
    /// Return from the current function call.
    ret,
};

/// An instruction for a `Vm` to execute.
pub const Repr = union(Code) {
    push: Val,
    get: Symbol.Interned,
    jump: usize,
    jump_if: usize,
    eval: usize,
    ret,
};

/// Intialize an instruction from its internal representation.
pub fn init(repr: Repr) Instruction {
    return .{ .repr = repr };
}

/// Execute `self` on `vm`.
pub fn execute(self: Instruction, vm: *Vm) Error!void {
    switch (self.repr) {
        .push => |v| try vm.execution_context.pushVal(v),
        .get => |s| {
            const val = vm.execution_context.getGlobal(s) orelse return Error.SymbolNotFound;
            try vm.execution_context.pushVal(val);
        },
        .jump => |n| vm.execution_context.call_frame.instruction_index += n,
        .jump_if => |n| {
            const val = vm.execution_context.stack.pop() orelse return Error.StackUnderflow;
            if (val.isTruthy()) vm.execution_context.call_frame.instruction_index += n;
        },
        .eval => |n| try executeEval(vm, n),
        .ret => try executeRet(vm),
    }
}

/// Evaluate the top `n` items on the stack as a function call.
///
/// The first of the `n` items is the function to be called, and the remaining
/// `n - 1` items are the arguments. After the call, the function and its
/// arguments are replaced on the stack with the single return value.
fn executeEval(vm: *Vm, n: usize) Error!void {
    if (n == 0) return Error.StackUnderflow;
    const function_idx = vm.execution_context.stack.len - n;
    const val = vm.execution_context.stack.get(function_idx);
    try vm.execution_context.pushCallFrame(ExecutionContext.CallFrame{
        .instructions = &.{},
        .instruction_index = 0,
        .stack_start = function_idx + 1,
    });
    switch (val.repr) {
        .bytecode_function => |handle| {
            const function = try vm.heap.bytecode_functions.get(handle);
            const got_args = vm.execution_context.localStack().len;
            const want_args = function.args;
            if (got_args != want_args) return Error.WrongArity;
        },
        .native_function => |handle| {
            const function = try vm.heap.native_functions.get(handle);
            vm.execution_context.stack.append(try function.call(vm)) catch return Error.StackOverflow;
            try Instruction.init(.{ .ret = {} }).execute(vm);
        },
        else => return Error.TypeError,
    }
}

fn returnVal(vm: *Vm) Val {
    const local_stack = vm.execution_context.localStack();
    if (local_stack.len == 0) return Val.from({});
    return local_stack[local_stack.len - 1];
}

fn executeRet(vm: *Vm) !void {
    const return_val = returnVal(vm);
    const previous_stack_len = vm.execution_context.call_frame.stack_start;
    vm.execution_context.call_frame = vm.execution_context.previous_call_frames.pop() orelse return Error.StackUnderflow;
    vm.execution_context.stack.len = previous_stack_len;
    if (vm.execution_context.localStack().len > 0) {
        vm.execution_context.stack.set(vm.execution_context.stack.len - 1, return_val);
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
        .{vm.prettyPrintSlice(vm.execution_context.stack.constSlice())},
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
        .{vm.prettyPrintSlice(vm.execution_context.stack.constSlice())},
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
        .{vm.prettyPrintSlice(vm.execution_context.stack.constSlice())},
    );
}

test "eval on non function produces TypeErrorError" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try init(.{ .push = Val.from(123) }).execute(&vm);
    try testing.expectError(
        Error.TypeError,
        init(.{ .eval = 1 }).execute(&vm),
    );
}

test "jump instruction increments instruction_index in call frame" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    vm.execution_context.call_frame.instruction_index = 200;

    try init(.{ .jump = 27 }).execute(&vm);
    try testing.expectEqual(
        227,
        vm.execution_context.call_frame.instruction_index,
    );
}

test "jump_if with truthy value pops value and increments instruction_index" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    try vm.execution_context.pushVals(&.{ Val.from(1), Val.from(2) });
    vm.execution_context.call_frame.instruction_index = 100;

    try init(.{ .jump_if = 20 }).execute(&vm);

    try testing.expectEqualDeep(
        &.{Val.from(1)},
        vm.execution_context.stack.constSlice(),
    );
    try testing.expectEqual(
        120,
        vm.execution_context.call_frame.instruction_index,
    );
}

test "jump_if with falsey value pops value and does not increment instruction_index" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    try vm.execution_context.pushVals(&.{ Val.from(1), Val.from({}) });
    vm.execution_context.call_frame.instruction_index = 100;

    try init(.{ .jump_if = 20 }).execute(&vm);

    try testing.expectEqualDeep(
        &.{Val.from(1)},
        vm.execution_context.stack.constSlice(),
    );
    try testing.expectEqual(
        100,
        vm.execution_context.call_frame.instruction_index,
    );
}
