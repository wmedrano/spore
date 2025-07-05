const std = @import("std");
const testing = std.testing;

const Val = @This();

repr: ValRepr,

// Create a new `Val` from its internal representation. For internal use only.
fn init(repr: ValRepr) Val {
    return .{ .repr = repr };
}

// Create a new `Val` that holds an integer.
pub fn new_int(int: i64) Val {
    return init(.{ .int = int });
}

// Create a new `Val` that holds a float.
pub fn new_float(float: f64) Val {
    return init(.{ .float = float });
}

// Format a `Val` for printing.
pub fn format(self: Val, comptime fmt: []const u8, options: std.fmt.FormatOptions, writer: anytype) !void {
    try self.repr.format(fmt, options, writer);
}

// The internal representation of a value.
const ValRepr = union(enum) {
    int: i64,
    float: f64,

    pub fn format(
        self: ValRepr,
        comptime fmt: []const u8,
        options: std.fmt.FormatOptions,
        writer: anytype,
    ) !void {
        _ = fmt;
        _ = options;
        switch (self) {
            .int => |x| try writer.print("{}", .{x}),
            .float => |x| try writer.print("{d}", .{x}),
        }
    }
};

test "print val" {
    try testing.expectFmt("45", "{}", .{Val.new_int(45)});
    try testing.expectFmt("45.5", "{}", .{Val.new_float(45.5)});
}
