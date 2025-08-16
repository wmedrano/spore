//! A memory heap for managing the lifetime of various VM objects like pairs,
//! strings, and functions.
const std = @import("std");

const BytecodeFunction = @import("BytecodeFunction.zig");
const Color = @import("object_pool.zig").Color;
const NativeFunction = @import("NativeFunction.zig");
const ObjectPool = @import("object_pool.zig").ObjectPool;
const Pair = @import("Pair.zig");
const String = @import("String.zig");
const StringInterner = @import("StringInterner.zig");

const Heap = @This();

/// The allocator used for the Vms objects and metadata.
allocator: std.mem.Allocator,
/// The color of objects that aren't reachable.
///
/// Objects that are newly made should be marked as unreachable. If not, the
/// garbage collector will not scan child values which could lead to premature
/// garbage collection of those values.
unreachable_color: Color = .red,
/// The string interner used by the Vm. This should also be used when creating
/// symbols through `Symbols.intern`.
string_interner: StringInterner,
/// Stores all pair objects.
pairs: ObjectPool(Pair) = .{},
/// Stores all string objects.
strings: ObjectPool(String) = .{},
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
    self.pairs.deinit(self.allocator);

    var string_iter = self.strings.iter();
    while (string_iter.next()) |s| s.deinit(self.allocator);
    self.strings.deinit(self.allocator);

    var bytecode_iter = self.bytecode_functions.iter();
    while (bytecode_iter.next()) |bytecode| bytecode.deinit(self.allocator);
    self.bytecode_functions.deinit(self.allocator);
}
