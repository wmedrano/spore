//! The `Inspector` provides functions for pretty printing `Val`s.
//! Returns a Pair.ListIter for the given value, or an error if the value is not a list.
const std = @import("std");
const Pair = @import("Pair.zig");
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

/// Returns a Pair.ListIter for the given value, or an error if the value is not a list.
pub fn listIter(self: Inspector, val: Val) !Pair.ListIter {
    switch (val.repr) {
        .nil => return Pair.iterEmpty(),
        .pair => |handle| {
            const pair = try self.vm.heap.pairs.get(handle);
            return pair.iterList();
        },
        else => return error.WrongType,
    }
}

/// Pretty prints the stack trace.
pub fn stackTrace(self: Inspector) ?PrettyPrinter.StackTrace {
    return PrettyPrinter.StackTrace{ .vm = self.vm };
}


/// A formatter that combines stack trace and error information for comprehensive error reporting.
pub const ErrorReport = struct {
    vm: *const Vm,

    pub fn format(self: ErrorReport, comptime fmt: []const u8, options: std.fmt.FormatOptions, writer: anytype) !void {
        _ = fmt;
        _ = options;
        try writer.print("Error encountered!\n", .{});
        const last_error = if (self.vm.execution_context.last_error) |err|
            PrettyPrinter.Err{ .vm = self.vm, .err = err }
        else
            null;
        try writer.print("{any}\n{any}\n\n\n", .{
            PrettyPrinter.StackTrace{ .vm = self.vm },
            last_error,
        });
    }
};

/// Returns an ErrorReport formatter that combines stack trace and error information.
pub fn errorReport(self: Inspector) ErrorReport {
    return ErrorReport{ .vm = self.vm };
}
