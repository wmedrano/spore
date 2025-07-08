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
        const cdr = cons.cdr.prettyPrinter(vm);
        try writer.print("({} . {})", .{ car, cdr });
    }
};

test prettyPrinter {
    var vm = Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectFmt("nil", "{}", .{Val.init({}).prettyPrinter(&vm)});
    try testing.expectFmt("45", "{}", .{Val.init(45).prettyPrinter(&vm)});
    try testing.expectFmt("45.5", "{}", .{Val.init(45.5).prettyPrinter(&vm)});
}

test "pretty print cons" {
    var vm = Vm.init(testing.allocator);
    defer vm.deinit();
    const cons = Val.init(
        try vm.cons_cells.create(
            vm.allocator,
            ConsCell.init(Val.init(1), Val.init(2)),
        ),
    );
    try testing.expectFmt("(1 . 2)", "{}", .{cons.prettyPrinter(&vm)});
}
