//! A struct for pretty-printing `Val` instances.
const std = @import("std");
const testing = std.testing;

const ConsCell = @import("ConsCell.zig");
const Val = @import("Val.zig");
const Vm = @import("Vm.zig");

const PrettyPrinter = @This();

/// A reference to the VM, needed for resolving symbols and cons cells.
vm: *const Vm,
/// The value to be printed.
val: Val,

/// Create a new pretty printer for `val`.
pub fn init(vm: *const Vm, val: Val) PrettyPrinter {
    return .{ .vm = vm, .val = val };
}

/// Create a new pretty printer for `vals`.
pub fn initSlice(vm: *const Vm, vals: []const Val) SlicePrettyPrinter {
    return .{ .vm = vm, .vals = vals };
}

/// A struct for pretty-printing multiple `Val`.
pub const SlicePrettyPrinter = struct {
    vm: *const Vm,
    vals: []const Val,

    pub fn format(self: SlicePrettyPrinter, comptime fmt: []const u8, options: std.fmt.FormatOptions, writer: anytype) !void {
        _ = fmt;
        _ = options;
        for (self.vals, 0..self.vals.len) |v, idx| {
            if (idx == 0) {
                try writer.print("{}", .{init(self.vm, v)});
            } else {
                try writer.print(" {}", .{init(self.vm, v)});
            }
        }
    }
};

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
            const symbol = try x.get(self.vm.heap.string_interner);
            try writer.print("{}", .{symbol});
        },
        .cons => |handle| {
            const cons = try self.vm.heap.cons_cells.get(handle);
            try formatCons(cons, self.vm, writer);
        },
        .function => |handle| {
            const func = try self.vm.heap.functions.get(handle);
            try writer.print("{any}", .{func});
        },
    }
}

fn formatCons(cons: ConsCell, vm: *const Vm, writer: anytype) !void {
    try writer.print("({}", .{init(vm, cons.car)});
    try formatCdr(cons.cdr, vm, writer);
}

fn formatCdr(cdr: Val, vm: *const Vm, writer: anytype) !void {
    switch (cdr.repr) {
        .nil => try writer.print(")", .{}),
        .cons => |handle| {
            const next = try vm.heap.cons_cells.get(handle);
            try writer.print(" {}", .{init(vm, next.car)});
            try formatCdr(next.cdr, vm, writer);
        },
        else => try writer.print(" . {})", .{init(vm, cdr)}),
    }
}

test format {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectFmt("nil", "{}", .{PrettyPrinter.init(&vm, Val.from({}))});
    try testing.expectFmt("45", "{}", .{PrettyPrinter.init(&vm, Val.from(45))});
    try testing.expectFmt("45.5", "{}", .{PrettyPrinter.init(&vm, Val.from(45.5))});
}

test "pretty print cons pair" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    const cons = Val.from(
        try vm.heap.cons_cells.create(
            vm.heap.allocator,
            ConsCell.init(Val.from(1), Val.from(2)),
        ),
    );
    try testing.expectFmt("(1 . 2)", "{}", .{PrettyPrinter.init(&vm, cons)});
}

test "pretty print cons list" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    const cons = Val.from(
        try vm.heap.cons_cells.create(
            vm.heap.allocator,
            ConsCell.init(Val.from(1), Val.from({})),
        ),
    );
    try testing.expectFmt("(1)", "{}", .{PrettyPrinter.init(&vm, cons)});
}
