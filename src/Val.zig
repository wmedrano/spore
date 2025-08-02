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
    boolean,
    nil,
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
    /// A boolean value.
    boolean: bool,
    /// The `nil` value. This is equivalent to an empty list.
    nil,
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
            .boolean => |x| try writer.print("{any}", .{x}),
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

/// Create a new `Val` from a given value, deducing its type.
/// Supports `void`, `i64`, `f64`, `Symbol.Interned`, `Handle(ConsCell)`, and `Handle(String)`.
pub fn init(val: anytype) Val {
    const T = @TypeOf(val);
    switch (T) {
        void => return Val{ .repr = .{ .nil = {} } },
        bool => return Val{ .repr = .{ .boolean = val } },
        i64, comptime_int => return Val{ .repr = .{ .int = val } },
        f64, comptime_float => return Val{ .repr = .{ .float = val } },
        Symbol.Interned => return Val{ .repr = .{ .symbol = val } },
        Handle(ConsCell) => return Val{ .repr = .{ .cons = val } },
        ConsCell => @compileError("Unsupported type for Val.new: " ++ @typeName(T) ++
            ", did you mean " ++ @typeName(Handle(ConsCell)) ++ "?"),
        Handle(String) => return Val{ .repr = .{ .string = val } },
        String => @compileError("Unsupported type for Val.new: " ++ @typeName(T) ++
            ", did you mean " ++ @typeName(Handle(String)) ++ "?"),
        *const NativeFunction => return Val{ .repr = .{ .native_function = val } },
        NativeFunction => @compileError("Unsupported type for Val.new: " ++ @typeName(T) ++
            ", did you mean " ++ @typeName(*const NativeFunction) ++ "?"),
        Handle(BytecodeFunction) => return Val{ .repr = .{ .bytecode_function = val } },
        BytecodeFunction => @compileError("Unsupported type for Val.new: " ++ @typeName(T) ++
            ", did you mean " ++ @typeName(Handle(BytecodeFunction)) ++ "?"),
        else => @compileError("Unsupported type for Val.new: " ++ @typeName(T)),
    }
}

/// An error that occurs when converting a Zig object into a Spore object.
pub const ToValError = error{WrongType};

/// Convert `Val` into a value of type `T`.
/// Supported types for `T` are: `void`, `i64`, `f64`, `Symbol.Interned`, `Handle(ConsCell)`, and `Handle(String)`.
pub fn to(self: Val, T: type) ToValError!T {
    switch (T) {
        void => switch (self.repr) {
            .nil => return {},
            else => return ToValError.WrongType,
        },
        bool => switch (self.repr) {
            .boolean => |x| return x,
            else => return ToValError.WrongType,
        },
        i64 => switch (self.repr) {
            .int => |x| return x,
            else => return ToValError.WrongType,
        },
        f64 => switch (self.repr) {
            .float => |x| return x,
            else => return ToValError.WrongType,
        },
        Symbol.Interned => switch (self.repr) {
            .symbol => |x| return x,
            else => return ToValError.WrongType,
        },
        Symbol => @compileError("Unsupported type for Val.to: " ++ @typeName(T) ++
            ", did you mean " ++ @typeName(Symbol.Interned)),
        Handle(ConsCell) => switch (self.repr) {
            .cons => |x| return x,
            else => return ToValError.WrongType,
        },
        ConsCell => @compileError("Unsupported type for Val.to: " ++ @typeName(T) ++
            ", did you mean " ++ @typeName(Handle(ConsCell))),
        Handle(String) => switch (self.repr) {
            .string => |x| return x,
            else => return ToValError.WrongType,
        },
        String => @compileError("Unsupported type for Val.to: " ++ @typeName(T) ++
            ", did you mean " ++ @typeName(Handle(String))),
        Handle(NativeFunction) => switch (self.repr) {
            .native_function => |x| return x,
            else => return ToValError.WrongType,
        },
        Handle(BytecodeFunction) => switch (self.repr) {
            .bytecode_function => |x| return x,
            else => return ToValError.WrongType,
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
        .boolean => |b| return b,
        else => return true,
    }
}

test "Val is small" {
    try testing.expectEqual(2 * @sizeOf(usize), @sizeOf(Val));
}

test "Val.to nil/void" {
    const nil_val = Val.init({});
    _ = try nil_val.to(void);
    try testing.expectError(ToValError.WrongType, nil_val.to(i64));
}

test "Val.to i64" {
    const int_val = Val.init(42);
    try testing.expectEqual(42, try int_val.to(i64));
    try testing.expectError(ToValError.WrongType, int_val.to(f64));
}

test "Val.to f64" {
    const float_val = Val.init(3.14);
    try testing.expectEqual(3.14, try float_val.to(f64));
    try testing.expectError(ToValError.WrongType, float_val.to(i64));
}

test "Val.to Symbol.Interned" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const symbol = try vm.builder().symbol(Symbol.init("hello"));
    try testing.expectEqual(
        try vm.builder().internedSymbol(Symbol.init("hello")),
        symbol.to(Symbol.Interned),
    );
    try testing.expectError(
        ToValError.WrongType,
        symbol.to(i64),
    );
}

test "Val.to Handle(ConsCell)" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const handle = try vm.heap.cons_cells.create(
        vm.heap.allocator,
        ConsCell.init(Val.init(1), Val.init(2)),
        vm.heap.unreachable_color,
    );
    const cons_val = Val.init(handle);
    try testing.expectEqual(handle, try cons_val.to(Handle(ConsCell)));
    try testing.expectError(ToValError.WrongType, cons_val.to(i64));
}

test "bool true is truthy" {
    const bool_val = Val.init(true);
    try testing.expectEqual(true, bool_val.isTruthy());
}

test "bool false is falsey" {
    const bool_val = Val.init(false);
    try testing.expectEqual(false, bool_val.isTruthy());
}

test "nil is falsey" {
    const nil_val = Val.init({});
    try testing.expectEqual(false, nil_val.isTruthy());
}

test "int is truthy" {
    const int_val = Val.init(42);
    try testing.expectEqual(true, int_val.isTruthy());
}

test "float is truthy" {
    const float_val = Val.init(3.14);
    try testing.expect(float_val.isTruthy());
}

test "symbol is truthy" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const symbol = try vm.builder().symbol(Symbol.init("hello"));
    try testing.expect(symbol.isTruthy());
}

test "cons is truthy" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const handle = try vm.heap.cons_cells.create(
        vm.heap.allocator,
        ConsCell.init(Val.init(1), Val.init(2)),
        vm.heap.unreachable_color,
    );
    const cons_val = Val.init(handle);
    try testing.expect(cons_val.isTruthy());
}
