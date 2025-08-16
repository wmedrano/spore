const std = @import("std");
const testing = @import("std").testing;

const errors = @import("errors.zig");
const DetailedError = errors.DetailedError;
const ExecutionContext = @import("ExecutionContext.zig");
const NativeFunction = @import("NativeFunction.zig");
const String = @import("String.zig");
const Symbol = @import("Symbol.zig");
const Val = @import("Val.zig");
const Vm = @import("Vm.zig");

pub const Error = error{
    StackOverflow,
    StackUnderflow,
    SymbolNotFound,
    WrongType,
    WrongArity,
} || errors.Error || std.mem.Allocator.Error;

pub const Code = enum {
    push,
    pop,
    get,
    set,
    deref,
    iter_next,
    jump,
    jump_if,
    jump_if_not,
    jump_or_else_pop,
    pop_or_else_jump,
    eval,
    squash,
    ret,
};

/// An instruction for a `Vm` to execute.
pub const Instruction = union(Code) {
    /// Push a new value onto the stack.
    push: Val,
    /// Pop the top n elements from the stack.
    pop: i32,
    /// Get the value from the local stack at the given index and push it onto the main stack.
    get: i32,
    /// Pop the top value from the main stack and set it as the value of the local variable at the given index.
    set: i32,
    /// Get the value of the symbol from the global scope and push it onto the stack.
    deref: Symbol.Interned,
    /// Advance an iterator.
    /// The item at `index` is updated with the next value from the iterable at `index + 1`.
    /// The iterable at `index + 1` is updated to be the rest of the iterable.
    /// Pushes `true` if a value was retrieved, `false` otherwise.
    iter_next: struct { index: i32 },
    /// Skip the next n instructions.
    jump: i32,
    /// Pop the top value of the stack and skip the next `n` instructions if the
    /// value is truthy.
    jump_if: i32,
    /// Pop the top value of the stack and skip the next `n` instructions if the
    /// value is falsy.
    jump_if_not: i32,
    /// Jump if the top value of the stack is true or pop it otherwise.
    jump_or_else_pop: i32,
    /// Pop the top value of the stack if it is truthy or jump otherwise.
    pop_or_else_jump: i32,
    ///Evaluate the top n values of the stack as a function call.
    eval: i32,
    /// Remove the top n values and only keep the top. For example:
    ///   - stack: [1 2 3 4 5]
    ///   - squash: 3
    ///   - after: [1 2 5]
    squash: i32,
    /// Return from the current function call.
    ret,

    /// Execute `self` on `vm`.
    pub fn execute(self: Instruction, vm: *Vm) Error!void {
        switch (self) {
            .push => |v| try vm.execution_context.pushVal(v),
            .pop => |n| for (0..@intCast(n)) |_| {
                _ = try vm.execution_context.popVal();
            },
            .get => |idx| {
                const val = vm.execution_context.localStack()[@intCast(idx)];
                try vm.execution_context.pushVal(val);
            },
            .set => |idx| {
                const val = vm.execution_context.stack.pop() orelse return vm.builder().addError(DetailedError{ .stack_underflow = {} });
                vm.execution_context.localStack()[@intCast(idx)] = val;
            },
            .deref => |s| {
                if (vm.execution_context.getGlobal(s)) |val| {
                    return try vm.execution_context.pushVal(val);
                }
                return vm.builder().addError(DetailedError{ .symbol_not_found = .{ .symbol = s } });
            },
            .iter_next => |iter| try executeIterNext(vm, @intCast(iter.index)),
            .jump => |n| vm.execution_context.call_frame.instruction_index += n,
            .jump_if => |n| {
                const val = vm.execution_context.stack.pop() orelse return vm.builder().addError(DetailedError{ .stack_overflow = {} });
                if (val.isTruthy()) vm.execution_context.call_frame.instruction_index += n;
            },
            .jump_if_not => |n| {
                const val = vm.execution_context.stack.pop() orelse return vm.builder().addError(DetailedError{ .stack_overflow = {} });
                if (!val.isTruthy()) vm.execution_context.call_frame.instruction_index += n;
            },
            .jump_or_else_pop => |n| {
                const idx = vm.execution_context.stack.len - 1;
                if (vm.execution_context.stack.buffer[idx].isTruthy())
                    vm.execution_context.call_frame.instruction_index += n
                else
                    _ = vm.execution_context.stack.pop();
            },
            .pop_or_else_jump => |n| {
                const idx = vm.execution_context.stack.len - 1;
                if (vm.execution_context.stack.buffer[idx].isTruthy())
                    _ = vm.execution_context.stack.pop()
                else
                    vm.execution_context.call_frame.instruction_index += n;
            },
            .eval => |n| try executeEval(vm, @intCast(n)),
            .squash => |n| try executeSquash(vm, @intCast(n)),
            .ret => try executeRet(vm),
        }
    }

    /// Evaluate the top `n` items on the stack as a function call.
    ///
    /// The first of the `n` items is the function to be called, and the remaining
    /// `n - 1` items are the arguments. After the call, the function and its
    /// arguments are replaced on the stack with the single return value.
    fn executeEval(vm: *Vm, n: i32) Error!void {
        if (n == 0) return Error.StackUnderflow;
        const function_idx: i32 = @as(i32, @intCast(vm.execution_context.stack.len)) - n;
        const stack_start = function_idx + 1;
        const function_val = vm.execution_context.stack.get(@intCast(function_idx));
        switch (function_val.repr) {
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
                        vm.execution_context.stack.addManyAsSlice(@intCast(extra_slots_size)) catch return vm.builder().addError(DetailedError{ .stack_overflow = {} });
                    for (extra_slots) |*v| v.* = Val.init({});
                }
            },
            .native_function => |function| {
                try vm.execution_context.pushCallFrame(
                    ExecutionContext.CallFrame{ .stack_start = stack_start },
                );
                vm.execution_context.stack.append(try function.call(vm)) catch return vm.builder().addError(DetailedError{ .stack_overflow = {} });
                try (Instruction{ .ret = {} }).execute(vm);
            },
            else => return vm.builder().addError(DetailedError{ .wrong_type = .{ .want = "function", .got = function_val } }),
        }
    }

    /// Advance an iterator.
    /// The item at `next_index` is updated with the next value from the iterable at `next_index + 1`.
    /// The iterable at `next_index + 1` is updated to be the rest of the iterable.
    /// Pushes `true` if a value was retrieved, `false` otherwise.
    fn executeIterNext(vm: *Vm, next_index: usize) Error!void {
        const iterable_index = next_index + 1;
        const local_stack = vm.execution_context.localStack();
        const iterable_val = local_stack[iterable_index];
        const has_value = blk: switch (iterable_val.repr) {
            // Next element in range.
            .int => |end| {
                const previous_val = local_stack[next_index];
                const previous_int = switch (previous_val.repr) {
                    .int => |int_val| int_val,
                    else => return vm.builder().addError(DetailedError{
                        .wrong_type = .{ .want = "int", .got = previous_val },
                    }),
                };
                const next = previous_int + 1;
                if (next >= end) break :blk false;
                local_stack[next_index] = Val.init(next);
                break :blk true;
            },
            // Next item of list.
            .pair => |handle| {
                const pair = try vm.heap.pairs.get(handle);
                local_stack[next_index] = pair.first;
                local_stack[iterable_index] = pair.second;
                break :blk true;
            },
            // End of list.
            .nil => false,
            else => return vm.builder().addError(DetailedError{
                .wrong_type = .{ .want = "iterable", .got = iterable_val },
            }),
        };
        try vm.execution_context.pushVal(Val.init(has_value));
    }

    fn returnVal(vm: *Vm) Val {
        const local_stack = vm.execution_context.localStack();
        if (local_stack.len == 0) return Val.init({});
        return local_stack[local_stack.len - 1];
    }

    fn executeRet(vm: *Vm) !void {
        const return_val = returnVal(vm);
        const previous_stack_len = vm.execution_context.call_frame.stack_start;
        vm.execution_context.call_frame = vm.execution_context.previous_call_frames.pop() orelse return vm.builder().addError(DetailedError{ .stack_underflow = {} });
        vm.execution_context.stack.len = @intCast(previous_stack_len);
        if (vm.execution_context.localStack().len == 0)
            return vm.builder().addError(DetailedError{ .stack_underflow = {} });
        vm.execution_context.stack.set(vm.execution_context.stack.len - 1, return_val);
    }

    fn executeSquash(vm: *Vm, n: usize) Error!void {
        const stack_len = vm.execution_context.stack.len;
        const top_val = vm.execution_context.stack.constSlice()[stack_len - 1];
        vm.execution_context.stack.len = stack_len - n + 1;
        vm.execution_context.stack.slice()[stack_len - n] = top_val;
    }
};

test "instruction is small" {
    try testing.expectEqual(3 * @sizeOf(usize), @sizeOf(Instruction));
}

test "push val pushes to stack" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try (Instruction{ .push = Val.init(42) }).execute(&vm);
    try (Instruction{ .push = Val.init(43) }).execute(&vm);

    try testing.expectFmt(
        "42 43",
        "{}",
        .{vm.inspector().prettySlice(vm.execution_context.stack.constSlice())},
    );
}

test "get symbol pushes value referred to by symbol onto stack" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    const symbol = try vm.builder().internedSymbol(Symbol.init("my-var"));
    try vm.execution_context.setGlobal(vm.heap.allocator, symbol, Val.init(123));

    try (Instruction{ .deref = symbol }).execute(&vm);

    try testing.expectFmt(
        "123",
        "{}",
        .{vm.inspector().prettySlice(vm.execution_context.stack.constSlice())},
    );
}

test "eval calls function" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    const plus = try vm.builder().internedSymbol(Symbol.init("+"));
    try (Instruction{ .deref = plus }).execute(&vm);
    try (Instruction{ .push = Val.init(10) }).execute(&vm);
    try (Instruction{ .push = Val.init(20) }).execute(&vm);
    try (Instruction{ .eval = 3 }).execute(&vm);
    try testing.expectFmt(
        "30",
        "{}",
        .{vm.inspector().prettySlice(vm.execution_context.stack.constSlice())},
    );
}

test "eval on non function produces Wrong Type Error" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try (Instruction{ .push = Val.init(123) }).execute(&vm);
    try testing.expectError(
        Error.WrongType,
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

    try vm.execution_context.pushVals(&.{ Val.init(1), Val.init(2) });
    vm.execution_context.call_frame.instruction_index = 100;

    try (Instruction{ .jump_if = 20 }).execute(&vm);

    try testing.expectEqualDeep(
        &.{Val.init(1)},
        vm.execution_context.stack.constSlice(),
    );
    try testing.expectEqual(
        120,
        vm.execution_context.call_frame.instruction_index,
    );
}

test "jump_if with falsy value pops value and does not increment instruction_index" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    try vm.execution_context.pushVals(&.{ Val.init(1), Val.init(false) });
    vm.execution_context.call_frame.instruction_index = 100;

    try (Instruction{ .jump_if = 20 }).execute(&vm);

    try testing.expectEqualDeep(
        &.{Val.init(1)},
        vm.execution_context.stack.constSlice(),
    );
    try testing.expectEqual(
        100,
        vm.execution_context.call_frame.instruction_index,
    );
}

test "jump_or_else_pop with truthy value keeps value and increments instruction_index" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try vm.execution_context.pushVals(&.{ Val.init(10), Val.init(true) });
    vm.execution_context.call_frame.instruction_index = 50;

    try (Instruction{ .jump_or_else_pop = 5 }).execute(&vm);

    try testing.expectEqualDeep(
        &.{ Val.init(10), Val.init(true) },
        vm.execution_context.stack.constSlice(),
    );
    try testing.expectEqual(
        55,
        vm.execution_context.call_frame.instruction_index,
    );
}

test "jump_or_else_pop with falsy value pops value and does not increment instruction_index" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try vm.execution_context.pushVals(&.{ Val.init(10), Val.init(false) });
    vm.execution_context.call_frame.instruction_index = 50;

    try (Instruction{ .jump_or_else_pop = 5 }).execute(&vm);

    try testing.expectEqualDeep(
        &.{Val.init(10)},
        vm.execution_context.stack.constSlice(),
    );
    try testing.expectEqual(
        50,
        vm.execution_context.call_frame.instruction_index,
    );
}

test "pop_or_else_jump with truthy value pops value and does not increment instruction_index" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try vm.execution_context.pushVals(&.{ Val.init(10), Val.init(true) });
    vm.execution_context.call_frame.instruction_index = 50;

    try (Instruction{ .pop_or_else_jump = 5 }).execute(&vm);

    try testing.expectEqualDeep(
        &.{Val.init(10)},
        vm.execution_context.stack.constSlice(),
    );
    try testing.expectEqual(
        50,
        vm.execution_context.call_frame.instruction_index,
    );
}

test "pop_or_else_jump with falsy value does not pop value and increments instruction_index" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try vm.execution_context.pushVals(&.{ Val.init(10), Val.init(false) });
    vm.execution_context.call_frame.instruction_index = 50;

    try (Instruction{ .pop_or_else_jump = 5 }).execute(&vm);

    try testing.expectEqualDeep(
        &.{ Val.init(10), Val.init(false) },
        vm.execution_context.stack.constSlice(),
    );
    try testing.expectEqual(
        55,
        vm.execution_context.call_frame.instruction_index,
    );
}
