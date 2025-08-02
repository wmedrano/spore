//! The `Inspector` provides functions for pretty printing `Val`s.
//! Returns a ConsCell.ListIter for the given value, or an error if the value is not a list.
const ConsCell = @import("ConsCell.zig");
const PrettyPrinter = @import("PrettyPrinter.zig");
const Val = @import("Val.zig");
const Vm = @import("Vm.zig");

const Inspector = @This();

vm: *const Vm,

/// Pretty prints a single `Val`.
pub fn pretty(self: Inspector, val: Val) PrettyPrinter {
    return PrettyPrinter{
        .vm = self.vm,
        .val = val,
    };
}

/// Pretty prints a slice of `Val`s.
pub fn prettySlice(self: Inspector, vals: []const Val) PrettyPrinter.Slice {
    return PrettyPrinter.Slice{
        .vm = self.vm,
        .vals = vals,
    };
}

/// Returns a ConsCell.ListIter for the given value, or an error if the value is not a list.
pub fn listIter(self: Inspector, val: Val) !ConsCell.ListIter {
    switch (val.repr) {
        .nil => return ConsCell.iterEmpty(),
        .cons => |handle| {
            const cons = try self.vm.heap.cons_cells.get(handle);
            return cons.iterList();
        },
        else => return error.WrongType,
    }
}

/// Pretty prints the stack trace.
pub fn stackTrace(self: Inspector) ?PrettyPrinter.StackTrace {
    return PrettyPrinter.StackTrace{ .vm = self.vm };
}

/// Pretty prints the last error.
pub fn lastError(self: Inspector) ?PrettyPrinter.Err {
    const err = self.vm.execution_context.last_error orelse return null;
    return PrettyPrinter.Err{ .vm = self.vm, .err = err };
}
