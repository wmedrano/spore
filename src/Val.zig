//! A dynamically-typed value capable of holding many different types.
const std = @import("std");
const testing = std.testing;

const BytecodeFunction = @import("BytecodeFunction.zig");
const ConsCell = @import("ConsCell.zig");
const Handle = @import("datastructures/object_pool.zig").Handle;
const Symbol = @import("datastructures/Symbol.zig");
const NativeFunction = @import("NativeFunction.zig");
const PrettyPrinter = @import("PrettyPrinter.zig");
const String = @import("String.zig");
const Vm = @import("Vm.zig");

const Val = @This();

/// The internal representation of the `Val` object. This is optimized to be
/// small.
repr: Repr,

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
pub const Repr = union(Tag) {
    /// The `nil` value. This is equivalent to an empty list.
    nil,
    /// The true value. There is only one.
    true_bool,
    /// An integer.
    int: i64,
    /// A floating point number.
    float: f64,
    /// A symbol. Interned to keep the size of `Repr` small.
    symbol: Symbol.Interned,
    /// A cons cell pair. Stored as a handle to keep the size of `Repr` small.
    cons: Handle(ConsCell),
    /// A string. Stored as a handle to keep the size of `Repr` small.
    string: Handle(String),
    /// A native_function. Stored as a handle to keep the size of `Repr` small.
    native_function: *const NativeFunction,
    /// A bytecode function. Stored as a handle to keep the size of `Repr`
    /// small.
    bytecode_function: Handle(BytecodeFunction),

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
            .true_bool => try writer.print("true", .{}),
            .int => |x| try writer.print("{}", .{x}),
            .float => |x| try writer.print("{d}", .{x}),
            .symbol => |x| try writer.print("@symbol-{}", .{x}),
            .cons => |handle| try writer.print("@cons-{}", .{handle.id}),
            .string => |handle| try writer.print("@string-{}", .{handle.id}),
            .native_function => |func| try writer.print("@function-{s}", .{func.name}),
            .bytecode_function => |handle| try writer.print("@bytecode-function-{}", .{handle.id}),
        }
    }
};

/// Create a new `Val` from its internal representation. For internal use only.
fn init(repr: Repr) Val {
    return .{ .repr = repr };
}

/// Create a new `Val` from a given value, deducing its type.
/// Supports `void`, `i64`, `f64`, `Symbol.Interned`, `Handle(ConsCell)`, and `Handle(String)`.
pub fn from(val: anytype) Val {
    const T = @TypeOf(val);
    switch (T) {
        void => return init(.{ .nil = {} }),
        bool => return init(if (val) .{ .true_bool = {} } else .{ .nil = {} }),
        i64, comptime_int => return init(.{ .int = val }),
        f64, comptime_float => return init(.{ .float = val }),
        Symbol.Interned => return init(.{ .symbol = val }),
        Handle(ConsCell) => return init(.{ .cons = val }),
        ConsCell => @compileError("Unsupported type for Val.new: " ++ @typeName(T) ++
            ", did you mean " ++ @typeName(Handle(ConsCell)) ++ "?"),
        Handle(String) => return init(.{ .string = val }),
        String => @compileError("Unsupported type for Val.new: " ++ @typeName(T) ++
            ", did you mean " ++ @typeName(Handle(String)) ++ "?"),
        *const NativeFunction => return init(.{ .native_function = val }),
        NativeFunction => @compileError("Unsupported type for Val.new: " ++ @typeName(T) ++
            ", did you mean " ++ @typeName(*const NativeFunction) ++ "?"),
        Handle(BytecodeFunction) => return init(.{ .bytecode_function = val }),
        BytecodeFunction => @compileError("Unsupported type for Val.new: " ++ @typeName(T) ++
            ", did you mean " ++ @typeName(Handle(BytecodeFunction)) ++ "?"),
        else => @compileError("Unsupported type for Val.new: " ++ @typeName(T)),
    }
}

/// An error that occurs when converting a Zig object into a Spore object.
pub const ToValError = error{TypeError};

/// Convert `Val` into a value of type `T`.
/// Supported types for `T` are: `void`, `i64`, `f64`, `Symbol.Interned`, `Handle(ConsCell)`, and `Handle(String)`.
pub fn to(self: Val, T: type) ToValError!T {
    switch (T) {
        void => switch (self.repr) {
            .nil => return {},
            else => return ToValError.TypeError,
        },
        bool => switch (self.repr) {
            .nil => return false,
            .true_bool => return true,
            else => return ToValError.TypeError,
        },
        i64 => switch (self.repr) {
            .int => |x| return x,
            else => return ToValError.TypeError,
        },
        f64 => switch (self.repr) {
            .float => |x| return x,
            else => return ToValError.TypeError,
        },
        Symbol.Interned => switch (self.repr) {
            .symbol => |x| return x,
            else => return ToValError.TypeError,
        },
        Symbol => @compileError("Unsupported type for Val.to: " ++ @typeName(T) ++
            ", did you mean " ++ @typeName(Symbol.Interned)),
        Handle(ConsCell) => switch (self.repr) {
            .cons => |x| return x,
            else => return ToValError.TypeError,
        },
        ConsCell => @compileError("Unsupported type for Val.to: " ++ @typeName(T) ++
            ", did you mean " ++ @typeName(Handle(ConsCell))),
        Handle(String) => switch (self.repr) {
            .string => |x| return x,
            else => return ToValError.TypeError,
        },
        String => @compileError("Unsupported type for Val.to: " ++ @typeName(T) ++
            ", did you mean " ++ @typeName(Handle(String))),
        Handle(NativeFunction) => switch (self.repr) {
            .native_function => |x| return x,
            else => return ToValError.TypeError,
        },
        Handle(BytecodeFunction) => switch (self.repr) {
            .bytecode_function => |x| return x,
            else => return ToValError.TypeError,
        },
        else => @compileError("Unsupported type for Val.to: " ++ @typeName(T)),
    }
}

/// Formats self implementing the `std.fmt.Format` interface. Prefer using
/// `PrettyPrinter` for more readable formatting.
pub fn format(self: Val, comptime fmt: []const u8, options: std.fmt.FormatOptions, writer: anytype) !void {
    try self.repr.format(fmt, options, writer);
}

/// Returns `true` if `Val` is considered truthy.
///
/// Truthiness is used to determine branching in if statements.
pub fn isTruthy(self: Val) bool {
    switch (self.repr) {
        .nil => return false,
        else => return true,
    }
}

test "Val is small" {
    try testing.expectEqual(2 * @sizeOf(usize), @sizeOf(Val));
}

test "Val.to nil/void" {
    const nil_val = Val.from({});
    _ = try nil_val.to(void);
    try testing.expectError(ToValError.TypeError, nil_val.to(i64));
}

test "Val.to i64" {
    const int_val = Val.from(42);
    try testing.expectEqual(42, try int_val.to(i64));
    try testing.expectError(ToValError.TypeError, int_val.to(f64));
}

test "Val.to f64" {
    const float_val = Val.from(3.14);
    try testing.expectEqual(3.14, try float_val.to(f64));
    try testing.expectError(ToValError.TypeError, float_val.to(i64));
}

test "Val.to Symbol.Interned" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const symbol = try Symbol.init("hello").intern(testing.allocator, &vm.heap.string_interner);
    const symbol_val = Val.from(symbol);
    try testing.expectEqual(symbol, try symbol_val.to(Symbol.Interned));
    try testing.expectError(ToValError.TypeError, symbol_val.to(i64));
}

test "Val.to Handle(ConsCell)" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const handle = try vm.heap.cons_cells.create(
        vm.heap.allocator,
        ConsCell.init(Val.from(1), Val.from(2)),
        vm.heap.dead_color,
    );
    const cons_val = Val.from(handle);
    try testing.expectEqual(handle, try cons_val.to(Handle(ConsCell)));
    try testing.expectError(ToValError.TypeError, cons_val.to(i64));
}

test "nil is falsey" {
    const nil_val = Val.from({});
    try testing.expectEqual(false, nil_val.isTruthy());
}

test "int is truthy" {
    const int_val = Val.from(42);
    try testing.expectEqual(true, int_val.isTruthy());
}

test "float is truthy" {
    const float_val = Val.from(3.14);
    try testing.expect(float_val.isTruthy());
}

test "symbol is truthy" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const symbol = try Symbol.init("hello").intern(testing.allocator, &vm.heap.string_interner);
    const symbol_val = Val.from(symbol);
    try testing.expect(symbol_val.isTruthy());
}

test "cons is truthy" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const handle = try vm.heap.cons_cells.create(
        vm.heap.allocator,
        ConsCell.init(Val.from(1), Val.from(2)),
        vm.heap.dead_color,
    );
    const cons_val = Val.from(handle);
    try testing.expect(cons_val.isTruthy());
}
