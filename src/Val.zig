//! A dynamically-typed value capable of holding many different types.
const std = @import("std");
const testing = std.testing;

const StringInterner = @import("datastructures/StringInterner.zig");
const Symbol = @import("datastructures/Symbol.zig");
const ConsCell = @import("ConsCell.zig"); // New import
const Handle = @import("datastructures/object_pool.zig").Handle;
const Vm = @import("Vm.zig");

const Val = @This();

repr: Repr,

pub const ToValError = error{WrongType};

/// Create a new `Val` from a given value, deducing its type.
/// Supports `void`, `i64`, `f64`, `Symbol.Interned`, and `Handle(ConsCell)`.
pub fn from(val: anytype) Val {
    const T = @TypeOf(val);
    switch (T) {
        void => return initRepr(Repr.newNil()),
        i64, comptime_int => return initRepr(Repr.newInt(val)),
        f64, comptime_float => return initRepr(Repr.newFloat(val)),
        Symbol.Interned => return initRepr(Repr.newSymbol(val)),
        Handle(ConsCell) => return initRepr(Repr.newCons(val)),
        ConsCell => @compileError("Unsupported type for Val.new: " ++ @typeName(T) ++
            ", did you mean " ++ @typeName(Handle(ConsCell)) ++ ")"),
        else => @compileError("Unsupported type for Val.new: " ++ @typeName(T)),
    }
}

/// Convert `Val` into a value of type `T`.
/// Supported types for `T` are: `void`, `i64`, `f64`, `Symbol.Interned`, and `Handle(ConsCell)`.
pub fn to(self: Val, T: type) ToValError!T {
    switch (T) {
        void => switch (self.repr) {
            .nil => return {},
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
        else => @compileError("Unsupported type for Val.to: " ++ @typeName(T)),
    }
}

/// Formats self implementing the `std.fmt.Format` interface. Prefer using the
/// object returned by `prettyPrinter`.
pub fn format(self: Val, comptime fmt: []const u8, options: std.fmt.FormatOptions, writer: anytype) !void {
    try self.repr.format(fmt, options, writer);
}

/// Return an object that pretty prints when formatted.
pub fn prettyPrinter(self: Val, vm: *const Vm) PrettyPrinter {
    return .{ .vm = vm, .val = self };
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

    /// Create a new `Repr` that holds a nil value.
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
            .symbol => |x| try writer.print("(symbol @{})", .{x}),
            .cons => |handle| try writer.print("(cons @{})", .{handle.id}),
        }
    }
};

/// A struct for pretty-printing `Val` instances. This should be built with
/// `Val.prettyPrinter`.
pub const PrettyPrinter = struct {
    /// A reference to the VM, needed for resolving symbols and cons cells.
    vm: *const Vm,
    /// The value to be printed.
    val: Val,

    /// Formats the `Val` for pretty-printing.
    pub fn format(
        self: PrettyPrinter,
        comptime fmt: []const u8,
        options: std.fmt.FormatOptions,
        writer: anytype,
    ) !void {
        _ = fmt;
        _ = options;
        switch (self.val.repr) {
            .nil => try writer.print("nil", .{}),
            .int => |x| try writer.print("{}", .{x}),
            .float => |x| try writer.print("{d}", .{x}),
            .symbol => |x| {
                const symbol = try x.get(self.vm.string_interner);
                try writer.print("{}", .{symbol});
            },
            .cons => |handle| {
                const cons = try self.vm.cons_cells.get(handle);
                try formatCons(cons, self.vm, writer);
            },
        }
    }

    fn formatCons(cons: ConsCell, vm: *const Vm, writer: anytype) !void {
        const car = cons.car.prettyPrinter(vm);
        try writer.print("({}", .{car});
        try formatCdr(cons.cdr, vm, writer);
    }

    fn formatCdr(cdr: Val, vm: *const Vm, writer: anytype) !void {
        switch (cdr.repr) {
            .nil => try writer.print(")", .{}),
            .cons => |handle| {
                const next = try vm.cons_cells.get(handle);
                try writer.print(" {}", .{next.car.prettyPrinter(vm)});
                try formatCdr(next.cdr, vm, writer);
            },
            else => try writer.print(" . {})", .{cdr.prettyPrinter(vm)}),
        }
    }
};

test prettyPrinter {
    var vm = Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectFmt("nil", "{}", .{Val.from({}).prettyPrinter(&vm)});
    try testing.expectFmt("45", "{}", .{Val.from(45).prettyPrinter(&vm)});
    try testing.expectFmt("45.5", "{}", .{Val.from(45.5).prettyPrinter(&vm)});
}

test "pretty print cons pair" {
    var vm = Vm.init(testing.allocator);
    defer vm.deinit();
    const cons = Val.from(
        try vm.cons_cells.create(
            vm.allocator,
            ConsCell.init(Val.from(1), Val.from(2)),
        ),
    );
    try testing.expectFmt("(1 . 2)", "{}", .{cons.prettyPrinter(&vm)});
}

test "pretty print cons list" {
    var vm = Vm.init(testing.allocator);
    defer vm.deinit();
    const cons = Val.from(
        try vm.cons_cells.create(
            vm.allocator,
            ConsCell.init(Val.from(1), Val.from({})),
        ),
    );
    try testing.expectFmt("(1)", "{}", .{cons.prettyPrinter(&vm)});
}

test "Val.to nil/void" {
    const nil_val = Val.from({});
    _ = try nil_val.to(void);
    try testing.expectError(ToValError.WrongType, nil_val.to(i64));
}

test "Val.to i64" {
    const int_val = Val.from(42);
    try testing.expectEqual(42, try int_val.to(i64));
    try testing.expectError(ToValError.WrongType, int_val.to(f64));
}

test "Val.to f64" {
    const float_val = Val.from(3.14);
    try testing.expectEqual(3.14, try float_val.to(f64));
    try testing.expectError(ToValError.WrongType, float_val.to(i64));
}

test "Val.to Symbol.Interned" {
    var vm = Vm.init(testing.allocator);
    defer vm.deinit();

    const symbol = try Symbol.init("hello").intern(testing.allocator, &vm.string_interner);
    const symbol_val = Val.from(symbol);
    try testing.expectEqual(symbol, try symbol_val.to(Symbol.Interned));
    try testing.expectError(ToValError.WrongType, symbol_val.to(i64));
}

test "Val.to Handle(ConsCell)" {
    var vm = Vm.init(testing.allocator);
    defer vm.deinit();

    const handle = try vm.cons_cells.create(
        vm.allocator,
        ConsCell.init(Val.from(1), Val.from(2)),
    );
    const cons_val = Val.from(handle);
    try testing.expectEqual(handle, try cons_val.to(Handle(ConsCell)));
    try testing.expectError(ToValError.WrongType, cons_val.to(i64));
}
