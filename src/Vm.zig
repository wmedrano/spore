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

const Vm = @This();

/// The allocator used for the Vms objects and metadata.
alloc: std.mem.Allocator,
/// Contains things such as the working call stack and data stack.
execution_context: ExecutionContext,
/// The string interner used by the Vm. This should also be used when creating
/// symbols through `Symbols.intern`.
string_interner: StringInterner,
/// Stores all objects.
cons_cells: ObjectPool(ConsCell),

/// Create a new VM.
pub fn init(allocator: std.mem.Allocator) Vm {
    return .{
        .alloc = allocator,
        .string_interner = StringInterner.init(),
    };
}

/// Deinitialize the VM.
pub fn deinit(self: *Vm) void {
    self.string_interner.deinit(self.alloc);
}

/// Evaluate string `src` as Spore code.
pub fn evalStr(src: []const u8) !void {
    _ = Tokenizer.init(src);
    return error.NotImplemented;
}
