const std = @import("std");
const testing = std.testing;

const Val = @This();

repr: Repr,

/// Create a new `Val` from a given value, deducing its type.
/// Supports i64 and f64.
pub fn init(val: anytype) Val {
    const T = @TypeOf(val);
    switch (T) {
        i64, comptime_int => return initRepr(Repr.newInt(val)),
        f64, comptime_float => return initRepr(Repr.newFloat(val)),
        else => @compileError("Unsupported type for Val.new: " ++ @typeName(T)),
    }
}

/// Format a `Val` for printing.
pub fn format(self: Val, comptime fmt: []const u8, options: std.fmt.FormatOptions, writer: anytype) !void {
    try self.repr.format(fmt, options, writer);
}

/// Create a new `Val` from its internal representation. For internal use only.
fn initRepr(repr: Repr) Val {
    return .{ .repr = repr };
}

/// The internal representation of a value.
const Repr = union(enum) {
    int: i64,
    float: f64,

    /// Create a new `ValRepr` that holds an integer.
    pub fn newInt(int: i64) Repr {
        return .{ .int = int };
    }

    /// Create a new `ValRepr` that holds a float.
    pub fn newFloat(float: f64) Repr {
        return .{ .float = float };
    }

    /// Formats the `ValRepr` for printing, implementing the `std.fmt.Format`
    /// interface.
    pub fn format(
        self: Repr,
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
    try testing.expectFmt("45", "{}", .{Val.init(45)});
    try testing.expectFmt("45.5", "{}", .{Val.init(45.5)});
}
