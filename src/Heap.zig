const std = @import("std");

const ConsCell = @import("ConsCell.zig");
const ObjectPool = @import("datastructures/object_pool.zig").ObjectPool;
const StringInterner = @import("datastructures/StringInterner.zig");
const Function = @import("Function.zig");

const Heap = @This();

/// The allocator used for the Vms objects and metadata.
allocator: std.mem.Allocator,
/// The string interner used by the Vm. This should also be used when creating
/// symbols through `Symbols.intern`.
string_interner: StringInterner,
/// Stores all cons cell objects.
cons_cells: ObjectPool(ConsCell) = .{},
/// Stores all function objects.
functions: ObjectPool(Function) = .{},

/// Initializes the heap, preparing it for allocations.
pub fn init(allocator: std.mem.Allocator) Heap {
    return .{
        .allocator = allocator,
        .string_interner = StringInterner.init(allocator),
    };
}

/// Deinitializes the heap, freeing all memory used by the objects it contains.
pub fn deinit(self: *Heap) void {
    self.string_interner.deinit(self.allocator);
    self.cons_cells.deinit(self.allocator);
}
