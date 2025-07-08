//! A dynamically-typed value capable of holding many different types.
const std = @import("std");
const testing = std.testing;

const StringInterner = @import("datastructures/StringInterner.zig");
const Symbol = @import("datastructures/Symbol.zig");
const ConsCell = @import("ConsCell.zig"); // New import
const Handle = @import("datastructures/object_pool.zig").Handle;

const Val = @This();

repr: Repr,

/// Create a new `Val` from a given value, deducing its type.
/// Supports `void`, `i64`, `f64`, `Symbol.Interned`, and `Handle(ConsCell)`.
pub fn init(val: anytype) Val {
    const T = @TypeOf(val);
    switch (T) {
        void => return initRepr(Repr.newNil()),
        i64, comptime_int => return initRepr(Repr.newInt(val)),
        f64, comptime_float => return initRepr(Repr.newFloat(val)),
        Symbol.Interned => return initRepr(Repr.newSymbol(val)),
        Handle(ConsCell) => return initRepr(Repr.newCons(val)),
        else => @compileError("Unsupported type for Val.new: " ++ @typeName(T)),
    }
}

/// Formats self implementing the `std.fmt.Format` interface.
pub fn format(self: Val, comptime fmt: []const u8, options: std.fmt.FormatOptions, writer: anytype) !void {
    try self.repr.format(fmt, options, writer);
}

/// Create a new `Val` from its internal representation. For internal use only.
fn initRepr(repr: Repr) Val {
    return .{ .repr = repr };
}

/// The internal representation of a value.
const Repr = union(enum) {
    nil,
    int: i64,
    float: f64,
    symbol: Symbol.Interned,
    cons: Handle(ConsCell),

    pub fn newNil() Repr {
        return .{ .nil = {} };
    }

    /// Create a new `Repr` that holds an integer.
    pub fn newInt(int: i64) Repr {
        return .{ .int = int };
    }

    /// Create a new `Repr` that holds a float.
    pub fn newFloat(float: f64) Repr {
        return .{ .float = float };
    }

    /// Create a new `Repr` that holds a symbol.
    pub fn newSymbol(val: Symbol.Interned) Repr {
        return .{ .symbol = val };
    }

    /// Create a new `Repr` that holds a ConsCell handle.
    pub fn newCons(handle: Handle(ConsCell)) Repr {
        return .{ .cons = handle };
    }

    /// Formats the `Repr` for printing, implementing the `std.fmt.Format`
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
            .nil => try writer.print("nil", .{}),
            .int => |x| try writer.print("{}", .{x}),
            .float => |x| try writer.print("{d}", .{x}),
            .symbol => |x| try writer.print("{}", .{x}),
            .cons => |handle| try writer.print("(cons @{})", .{handle.id}),
        }
    }
};

test "print val" {
    try testing.expectFmt("nil", "{}", .{Val.init({})});
    try testing.expectFmt("45", "{}", .{Val.init(45)});
    try testing.expectFmt("45.5", "{}", .{Val.init(45.5)});
}
