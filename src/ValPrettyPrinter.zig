//! A struct for pretty-printing `Val` instances. This should be built with
//! `Val.prettyPrinter`.
const std = @import("std");
const testing = std.testing;

const ConsCell = @import("ConsCell.zig");
const Val = @import("Val.zig");
const Vm = @import("Vm.zig");

// New import

const ValPrettyPrinter = @This();

/// A reference to the VM, needed for resolving symbols and cons cells.
vm: *const Vm,
/// The value to be printed.
val: Val,

/// Formats the `Val` for pretty-printing.
pub fn format(
    self: ValPrettyPrinter,
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

test format {
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
