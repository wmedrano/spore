//! Implements a mark-and-sweep garbage collector for the virtual machine.
const std = @import("std");
const testing = std.testing;

const BytecodeFunction = @import("BytecodeFunction.zig");
const ConsCell = @import("ConsCell.zig");
const Handle = @import("datastructures/object_pool.zig").Handle;
const ExecutionContext = @import("ExecutionContext.zig");
const Heap = @import("Heap.zig");
const Instruction = @import("instruction.zig").Instruction;
const NativeFunction = @import("NativeFunction.zig");
const Val = @import("Val.zig");
const Vm = @import("Vm.zig");

const GarbageCollector = @This();

vm: *Vm,

const Error = error{ObjectNotFound} || std.mem.Allocator.Error;

/// Initializes a new `GarbageCollector` instance.
///
/// Args:
///     vm: A pointer to the `Vm` instance to be garbage collected.
pub fn init(vm: *Vm) GarbageCollector {
    return .{ .vm = vm };
}

/// Executes a complete garbage collection cycle.
///
/// This involves marking all reachable objects from the VM's roots and then
/// sweeping (freeing) all unmarked (unreachable) objects.
pub fn run(self: *GarbageCollector) !void {
    try self.mark();
    try self.sweep();
}

/// Marks all reachable objects from the VM's roots.
///
/// This function traverses the VM's stack and global values, marking all
/// `Val` instances and their reachable sub-objects as alive.
fn mark(self: *GarbageCollector) !void {
    for (self.vm.execution_context.stack.constSlice()) |val| try self.markOne(val);
    var globals_iter = self.vm.execution_context.global_values.valueIterator();
    while (globals_iter.next()) |val| try self.markOne(val.*);
}

/// Marks a single `Val` and its reachable sub-objects as alive.
///
/// Args:
///     val: The `Val` to mark.
fn markOne(self: *GarbageCollector, val: Val) Error!void {
    const alive_color = self.vm.heap.dead_color.swap();
    switch (val.repr) {
        .boolean, .nil, .int, .float, .symbol, .native_function => {},
        .cons => |handle| {
            if (self.vm.heap.cons_cells.setColor(handle, alive_color) != alive_color) {
                const cons = try self.vm.heap.cons_cells.get(handle);
                try self.markOne(cons.car);
                try self.markOne(cons.cdr);
            }
        },
        .string => |handle| {
            _ = self.vm.heap.strings.setColor(handle, alive_color);
        },
        .bytecode_function => |handle| {
            if (self.vm.heap.bytecode_functions.setColor(handle, alive_color) != alive_color) {
                const function = try self.vm.heap.bytecode_functions.get(handle);
                try self.markInstructions(function.instructions);
            }
        },
    }
}

/// Marks all `Val` instances within a slice of `Instruction`s as alive.
///
/// Args:
///     instructions: A slice of `Instruction`s to traverse.
fn markInstructions(self: *GarbageCollector, instructions: []const Instruction) !void {
    for (instructions) |instruction| switch (instruction) {
        .push => |v| try self.markOne(v),
        .pop,
        .get,
        .set,
        .deref,
        .iter_next,
        .jump,
        .jump_if,
        .jump_if_not,
        .eval,
        .squash,
        .ret,
        => {},
    };
}

/// Sweeps (frees) all unmarked (unreachable) objects in the heap.
///
/// This function iterates through the heap's object pools, freeing any objects
/// that were not marked as alive during the marking phase.
fn sweep(self: *GarbageCollector) !void {
    _ = try self.vm.heap.cons_cells.sweep(self.vm.heap.allocator, self.vm.heap.dead_color);

    var bytecode_iter = try self.vm.heap.bytecode_functions.sweep(self.vm.heap.allocator, self.vm.heap.dead_color);
    while (bytecode_iter.next()) |bytecode| bytecode.deinit(self.vm.heap.allocator);

    var string_iter = try self.vm.heap.strings.sweep(self.vm.heap.allocator, self.vm.heap.dead_color);
    while (string_iter.next()) |string| string.deinit(self.vm.heap.allocator);

    self.vm.heap.dead_color = self.vm.heap.dead_color.swap();
}

test "run GC reuseses function slot" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    // Allocates 2 functions.
    // 1. Function that stores the entire evaluation of evalStr.
    // 2. Inner function.
    _ = try vm.evalStr("((function (a b) (+ a b)) 1 2)");
    const free_bytecode_functions = vm.heap.bytecode_functions.free_list.items.len;
    try vm.garbageCollect();
    try testing.expectEqual(
        free_bytecode_functions + 2,
        vm.heap.bytecode_functions.free_list.items.len,
    );
}
