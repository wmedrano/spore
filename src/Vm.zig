const std = @import("std");
const testing = std.testing;

const ConsCell = @import("ConsCell.zig");
const ObjectPool = @import("datastructures/object_pool.zig").ObjectPool;
const Handle = @import("datastructures/object_pool.zig").Handle;
const StringInterner = @import("datastructures/StringInterner.zig");
const Symbol = @import("datastructures/Symbol.zig");
const ExecutionContext = @import("ExecutionContext.zig");
const Tokenizer = @import("parser/Tokenizer.zig");
const Val = @import("Val.zig");
const Heap = @import("Heap.zig");

const Vm = @This();

/// Contains things such as the working call stack and data stack.
execution_context: ExecutionContext,
/// Contains all objects.
heap: Heap,

/// Create a new VM.
pub fn init(allocator: std.mem.Allocator) Vm {
    return .{
        .execution_context = .{},
        .heap = Heap.init(allocator),
    };
}

/// Deinitialize the VM.
pub fn deinit(self: *Vm) void {
    self.execution_context.deinit(self.heap.allocator);
    self.heap.deinit();
}
