//! A memory heap for managing the lifetime of various VM objects like cons
//cells, strings, and functions.
const std = @import("std");

const ConsCell = @import("ConsCell.zig");
const ObjectPool = @import("datastructures/object_pool.zig").ObjectPool;
const StringInterner = @import("datastructures/StringInterner.zig");
const NativeFunction = @import("NativeFunction.zig");
const BytecodeFunction = @import("BytecodeFunction.zig");

const Heap = @This();

/// The allocator used for the Vms objects and metadata.
allocator: std.mem.Allocator,
/// The string interner used by the Vm. This should also be used when creating
/// symbols through `Symbols.intern`.
string_interner: StringInterner,
/// Stores all cons cell objects.
cons_cells: ObjectPool(ConsCell) = .{},
/// Stores all native function objects.
native_functions: ObjectPool(NativeFunction) = .{},
/// Stores all bytecode function objects.
bytecode_functions: ObjectPool(BytecodeFunction) = .{},

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
    self.native_functions.deinit(self.allocator);
    var bytecode_iter = self.bytecode_functions.iter();
    while (bytecode_iter.next()) |bytecode| {
        bytecode.deinit(self.allocator);
    }
    self.bytecode_functions.deinit(self.allocator);
}
