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
const BytecodeFunction = @import("BytecodeFunction.zig");

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
/// The compiled expression.
instructions: std.ArrayListUnmanaged(Instruction) = .{},

/// Initialize a new compiler.
pub fn init(allocator: std.mem.Allocator, vm: *Vm) Compiler {
    return .{
        .vm = vm,
        .allocator = allocator,
    };
}

/// Compiles a `Val` s-expression into a slice of `Instruction`s owned by the
/// provided allocator.
pub fn add(self: *Compiler, val: Val) !void {
    try self.compileImpl(val);
}

// TODO: Document compile.
pub fn compile(self: *Compiler) !BytecodeFunction {
    return .{
        .instructions = try self.instructions.toOwnedSlice(self.vm.heap.allocator),
    };
}

// TODO: Document deinit.
pub fn deinit(self: *Compiler) void {
    self.instructions.deinit(self.allocator);
}

fn compileImpl(self: *Compiler, val: Val) CompileError!void {
    switch (val.repr) {
        .nil, .int, .float, .native_function, .bytecode_function => {
            try self.instructions.append(
                self.allocator,
                Instruction.init(.{ .push = val }),
            );
        },
        .symbol => |s| {
            try self.instructions.append(
                self.allocator,
                Instruction.init(.{ .get = s }),
            );
        },
        .cons => |cons_handle| try self.compileCons(cons_handle),
    }
}

fn compileCons(self: *Compiler, cons: Handle(ConsCell)) CompileError!void {
    var next_val: ?Val = Val.from(cons);
    var items: usize = 0;
    while (next_val) |val| {
        switch (val.repr) {
            .cons => |h| {
                const cell = try self.vm.heap.cons_cells.get(h);
                try self.compileImpl(cell.car);
                next_val = cell.cdr;
                items += 1;
            },
            .nil => next_val = null,
            // TODO: Add a unit test once this is possible. At the moment, there
            // is no way to create a cons cell that is not a list.
            else => return error.ImproperList,
        }
    }

    try self.instructions.append(
        self.allocator,
        Instruction.init(.{ .eval = items }),
    );
}

test "compile atom" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var compiler = init(testing.allocator, &vm);

    try compiler.add(Val.from(42));
    var bytecode = try compiler.compile();
    defer bytecode.deinit(testing.allocator);

    const expected = [_]Instruction{Instruction.init(.{ .push = Val.from(42) })};
    try testing.expectEqualDeep(
        BytecodeFunction{ .instructions = &expected },
        bytecode,
    );
}

test "compile simple list" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var compiler = init(testing.allocator, &vm);
    var parser = try SexpParser.init("(plus 1 2)");
    const parsed_val = (try parser.next(testing.allocator, &vm)).?;
    const plus_sym = try Symbol.init("plus").intern(
        testing.allocator,
        &vm.heap.string_interner,
    );

    try compiler.add(parsed_val);
    var bytecode = try compiler.compile();
    defer bytecode.deinit(testing.allocator);

    const expected = [_]Instruction{
        Instruction.init(.{ .get = plus_sym }),
        Instruction.init(.{ .push = Val.from(1) }),
        Instruction.init(.{ .push = Val.from(2) }),
        Instruction.init(.{ .eval = 3 }),
    };
    try testing.expectEqualDeep(
        BytecodeFunction{ .instructions = &expected },
        bytecode,
    );
}

test add {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    const plus_sym = try Symbol.init("plus").intern(
        testing.allocator,
        &vm.heap.string_interner,
    );
    var compiler = init(testing.allocator, &vm);
    var parser = try SexpParser.init("(plus 1 (plus 2 3))");
    const parsed_val = (try parser.next(testing.allocator, &vm)).?;

    try compiler.add(parsed_val);
    var bytecode = try compiler.compile();
    defer bytecode.deinit(testing.allocator);

    const expected = [_]Instruction{
        Instruction.init(.{ .get = plus_sym }),
        Instruction.init(.{ .push = Val.from(1) }),
        Instruction.init(.{ .get = plus_sym }),
        Instruction.init(.{ .push = Val.from(2) }),
        Instruction.init(.{ .push = Val.from(3) }),
        Instruction.init(.{ .eval = 3 }),
        Instruction.init(.{ .eval = 3 }),
    };
    try testing.expectEqualDeep(
        BytecodeFunction{ .instructions = &expected },
        bytecode,
    );
}
