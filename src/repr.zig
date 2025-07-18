//! A dynamically-typed value capable of holding many different types.
const std = @import("std");

const BytecodeFunction = @import("BytecodeFunction.zig");
const ConsCell = @import("ConsCell.zig");
const Handle = @import("datastructures/object_pool.zig").Handle;
const Symbol = @import("datastructures/Symbol.zig");
const NativeFunction = @import("NativeFunction.zig");
const String = @import("String.zig");

pub const Tag = enum {
    nil,
    true_bool,
    int,
    float,
    symbol,
    cons,
    string,
    native_function,
    bytecode_function,
};

/// The internal representation of a value.
pub const ValRepr = union(Tag) {
    /// The `nil` value. This is equivalent to an empty list.
    nil,
    /// The true value. There is only one.
    true_bool,
    /// An integer.
    int: i64,
    /// A floating point number.
    float: f64,
    /// A symbol. Interned to keep the size of `ValRepr` small.
    symbol: Symbol.Interned,
    /// A cons cell pair. Stored as a handle to keep the size of `ValRepr` small.
    cons: Handle(ConsCell),
    /// A string. Stored as a handle to keep the size of `ValRepr` small.
    string: Handle(String),
    /// A native_function. Stored as a handle to keep the size of `ValRepr` small.
    native_function: Handle(NativeFunction),
    /// A bytecode function. Stored as a handle to keep the size of `ValRepr`
    /// small.
    bytecode_function: Handle(BytecodeFunction),

    /// Create a new `ValRepr` that holds a nil value.
    pub fn newNil() ValRepr {
        return .{ .nil = {} };
    }

    /// Create a new `ValRepr` that holds a bool value.
    pub fn newBool(b: bool) ValRepr {
        return if (b) .{ .true_bool = {} } else newNil();
    }

    /// Create a new `ValRepr` that holds an integer.
    pub fn newInt(int: i64) ValRepr {
        return .{ .int = int };
    }

    /// Create a new `ValRepr` that holds a float.
    pub fn newFloat(float: f64) ValRepr {
        return .{ .float = float };
    }

    /// Create a new `ValRepr` that holds a symbol.
    pub fn newSymbol(val: Symbol.Interned) ValRepr {
        return .{ .symbol = val };
    }

    /// Create a new `ValRepr` that holds a ConsCell handle.
    pub fn newCons(handle: Handle(ConsCell)) ValRepr {
        return .{ .cons = handle };
    }

    /// Create a new `ValRepr` that holds a string handle.
    pub fn newString(handle: Handle(String)) ValRepr {
        return .{ .string = handle };
    }

    /// Create a new `ValRepr` that holds a NativeFunction handle.
    pub fn newNativeFunction(handle: Handle(NativeFunction)) ValRepr {
        return .{ .native_function = handle };
    }

    /// Create a new `ValRepr` that holds a BytecodeFunction handle.
    pub fn newBytecodeFunction(handle: Handle(BytecodeFunction)) ValRepr {
        return .{ .bytecode_function = handle };
    }

    /// Formats the `ValRepr` for printing, implementing the `std.fmt.Format`
    /// interface.
    pub fn format(
        self: ValRepr,
        comptime fmt: []const u8,
        options: std.fmt.FormatOptions,
        writer: anytype,
    ) !void {
        _ = fmt;
        _ = options;
        switch (self) {
            .nil => try writer.print("nil", .{}),
            .true_bool => try writer.print("true", .{}),
            .int => |x| try writer.print("{}", .{x}),
            .float => |x| try writer.print("{d}", .{x}),
            .symbol => |x| try writer.print("@symbol-{}", .{x}),
            .cons => |handle| try writer.print("@cons-{}", .{handle.id}),
            .string => |handle| try writer.print("@string-{}", .{handle.id}),
            .native_function => |handle| try writer.print("@function-{}", .{handle.id}),
            .bytecode_function => |handle| try writer.print("@bytecode-function-{}", .{handle.id}),
        }
    }
};
