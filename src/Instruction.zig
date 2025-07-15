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
    /// Get the value from the local stack.
    get,
    /// Get the value of the symbol and push it on the stack.
    deref,
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
    get: usize,
    deref: Symbol.Interned,
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
        .get => |idx| {
            const val = vm.execution_context.localStack()[idx];
            try vm.execution_context.pushVal(val);
        },
        .deref => |s| {
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
    const stack_start = function_idx + 1;
    const val = vm.execution_context.stack.get(function_idx);
    switch (val.repr) {
        .bytecode_function => |handle| {
            const function = try vm.heap.bytecode_functions.get(handle);
            try vm.execution_context.pushCallFrame(ExecutionContext.CallFrame{
                .instructions = function.instructions,
                .stack_start = stack_start,
            });
            const got_args = vm.execution_context.localStack().len;
            const want_args = function.args;
            if (got_args != want_args) return Error.WrongArity;
        },
        .native_function => |handle| {
            try vm.execution_context.pushCallFrame(
                ExecutionContext.CallFrame{ .stack_start = stack_start },
            );
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

/// Formats self implementing the `std.fmt.Format` interface.
pub fn format(self: Instruction, comptime fmt: []const u8, options: std.fmt.FormatOptions, writer: anytype) !void {
    _ = fmt;
    _ = options;
    switch (self.repr) {
        .push => |val| try writer.print("(push {})", .{val}),
        .get => |idx| try writer.print("(get {})", .{idx}),
        .deref => |symbol| try writer.print("(deref {})", .{symbol}),
        .jump => |n| try writer.print("(jump {})", .{n}),
        .jump_if => |n| try writer.print("(jump_if {})", .{n}),
        .eval => |n| try writer.print("(eval {})", .{n}),
        .ret => try writer.print("(ret)", .{}),
    }
}

test "format push instruction" {
    const instruction = Instruction.init(.{ .push = Val.from(123) });
    try testing.expectFmt("(push 123)", "{}", .{instruction});
}

test "format get instruction" {
    const instruction = Instruction.init(.{ .get = 5 });
    try testing.expectFmt("(get 5)", "{}", .{instruction});
}

test "format deref instruction" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const symbol = try Symbol.init("my-var").intern(
        std.testing.allocator,
        &vm.heap.string_interner,
    );
    const instruction = Instruction.init(.{ .deref = symbol });
    var buf: [64]u8 = undefined;
    try testing.expectFmt(
        try std.fmt.bufPrint(&buf, "(deref {any})", .{symbol}),
        "{}",
        .{instruction},
    );
}

test "format jump instruction" {
    const instruction = Instruction.init(.{ .jump = 10 });
    try testing.expectFmt("(jump 10)", "{}", .{instruction});
}

test "format jump_if instruction" {
    const instruction = Instruction.init(.{ .jump_if = 20 });
    try testing.expectFmt("(jump_if 20)", "{}", .{instruction});
}

test "format eval instruction" {
    const instruction = Instruction.init(.{ .eval = 3 });
    try testing.expectFmt("(eval 3)", "{}", .{instruction});
}

test "format ret instruction" {
    const instruction = Instruction.init(.{ .ret = {} });
    try testing.expectFmt("(ret)", "{}", .{instruction});
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

    try init(.{ .deref = symbol }).execute(&vm);

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
    try init(.{ .deref = plus }).execute(&vm);
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
