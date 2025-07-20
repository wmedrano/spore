const std = @import("std");
const testing = std.testing;

const StringInterner = @import("datastructures/StringInterner.zig");
const Symbol = @import("datastructures/Symbol.zig");
const Instruction = @import("instruction.zig").Instruction;
const Tokenizer = @import("parser/Tokenizer.zig");
const Val = @import("Val.zig");
const Vm = @import("Vm.zig");

/// Manages the execution state of the VM, including the data stack.
const ExecutionContext = @This();

/// Holds information about a single function call.
pub const CallFrame = struct {
    /// The sequence of instructions to be executed for this function call.
    instructions: []const Instruction = &.{},
    /// The index of the next instruction to be executed.
    instruction_index: usize = 0,
    /// The index in the main stack where this function's local stack starts.
    stack_start: usize = 0,
};

/// A map from symbol to its value in the global namespace.
global_values: std.AutoHashMapUnmanaged(Symbol.Interned, Val) = .{},
/// The backing array for the stack.
stack: std.BoundedArray(Val, 1024) = .{},
/// Holds the current call frame.
call_frame: CallFrame = .{},
/// Holds the previous call frames.
previous_call_frames: std.BoundedArray(CallFrame, 64) = .{},

/// Deinitialize self and free all memory.
pub fn deinit(self: *ExecutionContext, allocator: std.mem.Allocator) void {
    self.global_values.deinit(allocator);
}

/// Push `val` onto the stack.
pub fn pushVal(self: *ExecutionContext, val: Val) !void {
    self.stack.append(val) catch return error.StackOverflow;
}

/// Push several `vals` onto the ExecutionContext. The last value of `vals` will be on top.
pub fn pushVals(self: *ExecutionContext, vals: []const Val) !void {
    self.stack.appendSlice(vals) catch return error.StackOverflow;
}

/// Pop a `Val` from the stack.
pub fn popVal(self: *ExecutionContext) !Val {
    const val = self.stack.pop() orelse return error.StackUnderflow;
    return val;
}

/// Set a global value. `symbol` will refer to `val`.
pub fn setGlobal(self: *ExecutionContext, allocator: std.mem.Allocator, symbol: Symbol.Interned, val: Val) std.mem.Allocator.Error!void {
    try self.global_values.put(allocator, symbol, val);
}

/// Get the value of `symbol`.
pub fn getGlobal(self: ExecutionContext, symbol: Symbol.Interned) ?Val {
    return self.global_values.get(symbol);
}

/// Get the portion of the stack belonging to the current call frame. If in the
/// global scope, this returns the entire stack.
pub fn localStack(self: *ExecutionContext) []Val {
    return self.stack.slice()[self.call_frame.stack_start..];
}

/// Retrieves the next instruction from the current call frame and advances the
/// instruction pointer. Returns `null` if there is no current call frame or
/// if all instructions in the current frame have been executed.
pub fn nextInstruction(self: *ExecutionContext) Instruction {
    if (self.call_frame.instruction_index >= self.call_frame.instructions.len) return Instruction{ .ret = {} };
    const instruction = self.call_frame.instructions[self.call_frame.instruction_index];
    self.call_frame.instruction_index += 1;
    return instruction;
}

/// Pushes the current call frame onto the `previous_call_frames` stack and makes `call_frame` the new current call frame.
/// This is used when entering a new function call, saving the context of the caller.
///
/// Parameters:
///   `call_frame`: The new call frame to be made active.
///
/// Returns:
///   `error.StackOverflow` if the `previous_call_frames` stack is full.
pub fn pushCallFrame(self: *ExecutionContext, call_frame: CallFrame) !void {
    self.previous_call_frames.append(self.call_frame) catch return error.StackOverflow;
    self.call_frame = call_frame;
}

test "initial stack is empty" {
    var ctx = ExecutionContext{};
    try testing.expectFmt("{  }", "{any}", .{ctx.stack.constSlice()});
}

test "push val adds to stack" {
    var ctx = ExecutionContext{};
    try ctx.pushVals(&.{ Val.from(1), Val.from(2) });
    try testing.expectFmt("{ 1, 2 }", "{any}", .{ctx.stack.constSlice()});
}

test "push to many vals returns stack overflow" {
    var ctx = ExecutionContext{};
    for (0..ctx.stack.capacity()) |_| {
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
        .{ctx.stack.constSlice()},
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
    var vm = try Vm.init(testing.allocator);
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
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    const symbol = try Symbol.init("my-var").intern(vm.heap.allocator, &vm.heap.string_interner);
    try vm.execution_context.setGlobal(vm.heap.allocator, symbol, Val.from(123));

    try testing.expectEqualDeep(
        Val.from(123),
        vm.execution_context.getGlobal(symbol),
    );
}
