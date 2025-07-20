const std = @import("std");
const testing = @import("std").testing;

const Val = @import("Val.zig");
const Vm = @import("Vm.zig");
const PrettyPrinter = @import("PrettyPrinter.zig");
const Symbol = @import("datastructures/Symbol.zig");
const ExecutionContext = @import("ExecutionContext.zig");

pub const Error = error{
    ObjectNotFound,
    StackOverflow,
    StackUnderflow,
    SymbolNotFound,
    TypeError,
    WrongArity,
} || std.mem.Allocator.Error;

pub const Code = enum {
    push,
    get,
    set,
    deref,
    jump,
    jump_if,
    jump_if_not,
    eval,
    squash,
    ret,
};

/// An instruction for a `Vm` to execute.
pub const Instruction = union(Code) {
    /// Push a new value onto the stack.
    push: Val,
    /// Get the value from the local stack at the given index and push it onto the main stack.
    get: usize,
    /// Pop the top value from the main stack and set it as the value of the local variable at the given index.
    set: usize,
    /// Get the value of the symbol from the global scope and push it onto the stack.
    deref: Symbol.Interned,
    /// Skip the next n instructions.
    jump: usize,
    /// Pop the top value of the stack and skip the next `n` instructions if the
    /// value is truthy.
    jump_if: usize,
    /// Pop the top value of the stack and skip the next `n` instructions if the
    /// value is falsey.
    jump_if_not: usize,
    ///Evaluate the top n values of the stack as a function call.
    eval: usize,
    /// Remove the top n values and only keep the top. For example:
    ///   - stack: [1 2 3 4 5]
    ///   - squash: 3
    ///   - after: [1 2 5]
    squash: usize,
    /// Return from the current function call.
    ret,

    /// Execute `self` on `vm`.
    pub fn execute(self: Instruction, vm: *Vm) Error!void {
        switch (self) {
            .push => |v| try vm.execution_context.pushVal(v),
            .get => |idx| {
                const val = vm.execution_context.localStack()[idx];
                try vm.execution_context.pushVal(val);
            },
            .set => |idx| {
                const val = vm.execution_context.stack.pop() orelse return Error.StackUnderflow;
                vm.execution_context.localStack()[idx] = val;
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
            .jump_if_not => |n| {
                const val = vm.execution_context.stack.pop() orelse return Error.StackUnderflow;
                if (!val.isTruthy()) vm.execution_context.call_frame.instruction_index += n;
            },
            .eval => |n| try executeEval(vm, n),
            .squash => |n| try executeSquash(vm, n),
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
                const extra_slots_size = function.initial_local_stack_size - function.args;
                if (extra_slots_size > 0) {
                    const extra_slots =
                        vm.execution_context.stack.addManyAsSlice(extra_slots_size) catch
                            return Error.StackOverflow;
                    for (extra_slots) |*v| v.* = Val.from({});
                }
            },
            .native_function => |handle| {
                try vm.execution_context.pushCallFrame(
                    ExecutionContext.CallFrame{ .stack_start = stack_start },
                );
                const function = try vm.heap.native_functions.get(handle);
                vm.execution_context.stack.append(try function.call(vm)) catch return Error.StackOverflow;
                try (Instruction{ .ret = {} }).execute(vm);
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

    fn executeSquash(vm: *Vm, n: usize) Error!void {
        const stack_len = vm.execution_context.stack.len;
        const top_val = vm.execution_context.stack.constSlice()[stack_len - 1];
        vm.execution_context.stack.len = stack_len - n + 1;
        vm.execution_context.stack.slice()[stack_len - n] = top_val;
    }
};

test "push val pushes to stack" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try (Instruction{ .push = Val.from(42) }).execute(&vm);
    try (Instruction{ .push = Val.from(43) }).execute(&vm);

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

    try (Instruction{ .deref = symbol }).execute(&vm);

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
    try (Instruction{ .deref = plus }).execute(&vm);
    try (Instruction{ .push = Val.from(10) }).execute(&vm);
    try (Instruction{ .push = Val.from(20) }).execute(&vm);
    try (Instruction{ .eval = 3 }).execute(&vm);
    try testing.expectFmt(
        "30",
        "{}",
        .{vm.prettyPrintSlice(vm.execution_context.stack.constSlice())},
    );
}

test "eval on non function produces TypeErrorError" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try (Instruction{ .push = Val.from(123) }).execute(&vm);
    try testing.expectError(
        Error.TypeError,
        (Instruction{ .eval = 1 }).execute(&vm),
    );
}

test "jump instruction increments instruction_index in call frame" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    vm.execution_context.call_frame.instruction_index = 200;

    try (Instruction{ .jump = 27 }).execute(&vm);
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

    try (Instruction{ .jump_if = 20 }).execute(&vm);

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

    try (Instruction{ .jump_if = 20 }).execute(&vm);

    try testing.expectEqualDeep(
        &.{Val.from(1)},
        vm.execution_context.stack.constSlice(),
    );
    try testing.expectEqual(
        100,
        vm.execution_context.call_frame.instruction_index,
    );
}
