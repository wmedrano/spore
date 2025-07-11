//! Turns an s-expression (stored as a `Val`) into a `[]Instruction`.
const std = @import("std");
const testing = std.testing;

const ConsCell = @import("ConsCell.zig");
const Handle = @import("datastructures/object_pool.zig").Handle;
const Instruction = @import("Instruction.zig");
const SexpParser = @import("SexpParser.zig");
const Val = @import("Val.zig");
const Vm = @import("Vm.zig");
const Symbol = @import("datastructures/Symbol.zig");

const Compiler = @This();

pub const CompileError = error{
    /// A list form was not a proper list (did not end in nil).
    ImproperList,
    /// An error occurred while resolving a ConsCell handle.
    ObjectNotFound,
} || std.mem.Allocator.Error;

/// The `Vm` for the compiler.
vm: *Vm,
/// The allocator to use for temporary items.
allocator: std.mem.Allocator,

/// Initialize a new compiler.
pub fn init(allocator: std.mem.Allocator, vm: *Vm) Compiler {
    return .{
        .vm = vm,
        .allocator = allocator,
    };
}

/// Compiles a `Val` s-expression into a slice of `Instruction`s owned by the
/// provided allocator.
pub fn compile(self: *Compiler, val: Val) ![]Instruction {
    var instructions = std.ArrayList(Instruction).init(self.allocator);
    errdefer instructions.deinit();

    try self.compileImpl(val, &instructions);
    return instructions.toOwnedSlice();
}

fn compileImpl(self: *Compiler, val: Val, instructions: *std.ArrayList(Instruction)) CompileError!void {
    switch (val.repr) {
        .nil, .int, .float, .function => {
            try instructions.append(Instruction.init(.{ .push = val }));
        },
        .symbol => |s| {
            try instructions.append(Instruction.init(.{ .get = s }));
        },
        .cons => |cons_handle| {
            try self.compileCons(cons_handle, instructions);
        },
    }
}

fn compileCons(self: *Compiler, cons: Handle(ConsCell), instructions: *std.ArrayList(Instruction)) CompileError!void {
    var next_val: ?Val = Val.from(cons);
    var items: usize = 0;
    while (next_val) |val| {
        switch (val.repr) {
            .cons => |h| {
                const cell = try self.vm.heap.cons_cells.get(h);
                try self.compileImpl(cell.car, instructions);
                next_val = cell.cdr;
                items += 1;
            },
            .nil => next_val = null,
            // TODO: Add a unit test once this is possible. At the moment, there
            // is no way to create a cons cell that is not a list.
            else => return error.ImproperList,
        }
    }

    try instructions.append(Instruction.init(.{ .eval = items }));
}

test "compile atom" {
    var vm = Vm.init(testing.allocator);
    defer vm.deinit();
    var compiler = init(testing.allocator, &vm);

    const instructions = try compiler.compile(Val.from(42));
    defer testing.allocator.free(instructions);

    const expected = [_]Instruction{
        Instruction.init(.{ .push = Val.from(42) }),
    };
    try testing.expectEqualDeep(&expected, instructions);
}

test "compile simple list" {
    var vm = Vm.init(testing.allocator);
    defer vm.deinit();
    var compiler = init(testing.allocator, &vm);
    var parser = SexpParser.init("(plus 1 2)");
    const parsed_val = (try parser.next(testing.allocator, &vm)).?;
    const plus_sym = try Symbol.init("plus").intern(
        testing.allocator,
        &vm.heap.string_interner,
    );

    const instructions = try compiler.compile(parsed_val);
    defer testing.allocator.free(instructions);

    const expected = [_]Instruction{
        Instruction.init(.{ .get = plus_sym }),
        Instruction.init(.{ .push = Val.from(1) }),
        Instruction.init(.{ .push = Val.from(2) }),
        Instruction.init(.{ .eval = 3 }),
    };
    try testing.expectEqualDeep(&expected, instructions);
}

test compile {
    var vm = Vm.init(testing.allocator);
    defer vm.deinit();
    const plus_sym = try Symbol.init("plus").intern(
        testing.allocator,
        &vm.heap.string_interner,
    );
    var compiler = init(testing.allocator, &vm);
    var parser = SexpParser.init("(plus 1 (plus 2 3))");
    const parsed_val = (try parser.next(testing.allocator, &vm)).?;

    const instructions = try compiler.compile(parsed_val);
    defer testing.allocator.free(instructions);

    const expected = [_]Instruction{
        Instruction.init(.{ .get = plus_sym }),
        Instruction.init(.{ .push = Val.from(1) }),
        Instruction.init(.{ .get = plus_sym }),
        Instruction.init(.{ .push = Val.from(2) }),
        Instruction.init(.{ .push = Val.from(3) }),
        Instruction.init(.{ .eval = 3 }),
        Instruction.init(.{ .eval = 3 }),
    };
    try testing.expectEqualDeep(&expected, instructions);
}
