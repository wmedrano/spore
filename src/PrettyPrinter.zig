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

/// A struct for pretty-printing multiple `Val`.
pub const Slice = struct {
    vm: *const Vm,
    vals: []const Val,

    pub fn format(self: Slice, comptime fmt: []const u8, options: std.fmt.FormatOptions, writer: anytype) !void {
        _ = fmt;
        _ = options;
        for (self.vals, 0..self.vals.len) |v, idx| {
            if (idx == 0) {
                try writer.print("{}", .{PrettyPrinter{ .vm = self.vm, .val = v }});
            } else {
                try writer.print(" {}", .{PrettyPrinter{ .vm = self.vm, .val = v }});
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
        .true_bool => try writer.print("true", .{}),
        .int => |x| try writer.print("{}", .{x}),
        .float => |x| try writer.print("{d}", .{x}),
        .symbol => |x| {
            const symbol = x.get(self.vm.heap.string_interner) catch return writer.print("@bad-symbol", .{});
            try writer.print("{}", .{symbol});
        },
        .cons => |handle| {
            const cons = self.vm.heap.cons_cells.get(handle) catch return writer.print("@bad-cons", .{});
            try formatCons(cons, self.vm, writer);
        },
        .string => |handle| {
            const string = self.vm.heap.strings.get(handle) catch return writer.print("@bad-string", .{});
            try writer.print("{s}", .{string});
        },
        .native_function => |func| try writer.print("{any}", .{func}),
        .bytecode_function => |handle| {
            const func = self.vm.heap.bytecode_functions.get(handle) catch return writer.print("@bad-function", .{});
            try writer.print("{any}", .{func});
        },
    }
}

fn formatCons(cons: ConsCell, vm: *const Vm, writer: anytype) !void {
    try writer.print("({}", .{PrettyPrinter{ .vm = vm, .val = cons.car }});
    try formatCdr(cons.cdr, vm, writer);
}

fn formatCdr(cdr: Val, vm: *const Vm, writer: anytype) !void {
    switch (cdr.repr) {
        .nil => try writer.print(")", .{}),
        .cons => |handle| {
            const next = try vm.heap.cons_cells.get(handle);
            try writer.print(" {}", .{PrettyPrinter{ .vm = vm, .val = next.car }});
            try formatCdr(next.cdr, vm, writer);
        },
        else => try writer.print(" . {})", .{PrettyPrinter{ .vm = vm, .val = cdr }}),
    }
}

test format {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectFmt("nil", "{}", .{PrettyPrinter{ .vm = &vm, .val = Val.from({}) }});
    try testing.expectFmt("45", "{}", .{PrettyPrinter{ .vm = &vm, .val = Val.from(45) }});
    try testing.expectFmt("45.5", "{}", .{PrettyPrinter{ .vm = &vm, .val = Val.from(45.5) }});
}

test "pretty print cons pair" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    const cons = Val.from(
        try vm.heap.cons_cells.create(
            vm.heap.allocator,
            ConsCell.init(Val.from(1), Val.from(2)),
            vm.heap.dead_color,
        ),
    );
    try testing.expectFmt("(1 . 2)", "{}", .{PrettyPrinter{ .vm = &vm, .val = cons }});
}

test "pretty print cons list" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    const cons = Val.from(
        try vm.heap.cons_cells.create(
            vm.heap.allocator,
            ConsCell.init(Val.from(1), Val.from({})),
            vm.heap.dead_color,
        ),
    );
    try testing.expectFmt("(1)", "{}", .{PrettyPrinter{ .vm = &vm, .val = cons }});
}
