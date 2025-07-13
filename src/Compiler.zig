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

pub const Error = error{
    /// A list form was not a proper list (did not end in nil).
    ImproperList,
    /// An error occurred while resolving a ConsCell handle.
    ObjectNotFound,
} || std.mem.Allocator.Error;

/// The `Vm` for the compiler.
vm: *Vm,
/// The allocator to use for temporary items.
arena: *std.heap.ArenaAllocator,
/// The compiled expression.
instructions: std.ArrayListUnmanaged(Instruction) = .{},

/// Initialize a new compiler.
///
/// The allocator to use for temporary items, such as instructions appended to
/// the current compilation context.
pub fn init(arena: *std.heap.ArenaAllocator, vm: *Vm) Compiler {
    return .{
        .vm = vm,
        .arena = arena,
    };
}

/// Finalizes the compilation process, returning the compiled `BytecodeFunction`.
/// The instructions accumulated during `addExpr` calls are duplicated using the VM's heap allocator.
pub fn compile(self: *Compiler) !BytecodeFunction {
    const instructions = try self.vm.heap.allocator.dupe(Instruction, self.instructions.items);
    return .{ .instructions = instructions };
}

/// Compiles a `Val` and adds it to the current compilation context.
pub fn addExpr(self: *Compiler, expr: Val) !void {
    switch (expr.repr) {
        .nil, .int, .float, .native_function, .bytecode_function => {
            const instruction = Instruction.init(.{ .push = expr });
            try self.instructions.append(self.arena.allocator(), instruction);
        },
        .symbol => |s| {
            const instruction = if (s.unquote()) |interned_symbol|
                Instruction.init(.{ .push = Val.from(interned_symbol) })
            else
                Instruction.init(.{ .get = s });
            try self.instructions.append(self.arena.allocator(), instruction);
        },
        .cons => |cons_handle| try self.addCons(cons_handle),
    }
}

fn addCons(self: *Compiler, cons: Handle(ConsCell)) Error!void {
    var next_val: ?Val = Val.from(cons);
    var items: usize = 0;
    while (next_val) |val| {
        switch (val.repr) {
            .cons => |h| {
                const cell = try self.vm.heap.cons_cells.get(h);
                try self.addExpr(cell.car);
                next_val = cell.cdr;
                items += 1;
            },
            .nil => next_val = null,

            else => return error.ImproperList,
        }
    }

    try self.instructions.append(
        self.arena.allocator(),
        Instruction.init(.{ .eval = items }),
    );
}

test compile {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    const plus_sym = try Symbol.init("plus").intern(
        testing.allocator,
        &vm.heap.string_interner,
    );
    var arena = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena.deinit();
    var compiler = init(&arena, &vm);
    var parser = try SexpParser.init("(plus 1 (plus 2 3))");
    const parsed_val = (try parser.next(testing.allocator, &vm)).?;

    try compiler.addExpr(parsed_val);
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

test "compile improper list" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var arena = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena.deinit();
    var compiler = init(&arena, &vm);
    const cons = ConsCell{
        .car = Val.from(try Symbol.init("a").intern(testing.allocator, &vm.heap.string_interner)),
        .cdr = Val.from(42),
    };
    const cons_handle = try vm.heap.cons_cells.create(vm.heap.allocator, cons);

    try testing.expectError(
        Compiler.Error.ImproperList,
        compiler.addExpr(Val.from(cons_handle)),
    );
}

test "compile atom" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var arena = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena.deinit();
    var compiler = init(&arena, &vm);

    try compiler.addExpr(Val.from(42));
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
    var arena = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena.deinit();
    var compiler = init(&arena, &vm);
    var parser = try SexpParser.init("(plus 1 2)");
    const parsed_val = (try parser.next(testing.allocator, &vm)).?;
    const plus_sym = try Symbol.init("plus").intern(
        testing.allocator,
        &vm.heap.string_interner,
    );

    try compiler.addExpr(parsed_val);
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

test "quoted symbol is unquoted" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    const sym = try Symbol.init("sym").intern(
        testing.allocator,
        &vm.heap.string_interner,
    );
    var arena = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena.deinit();
    var compiler = init(&arena, &vm);
    var parser = try SexpParser.init("'sym");
    const parsed_val = (try parser.next(testing.allocator, &vm)).?;

    try compiler.addExpr(parsed_val);
    var bytecode = try compiler.compile();
    defer bytecode.deinit(testing.allocator);

    const expected = [_]Instruction{
        Instruction.init(.{ .push = Val.from(sym) }),
    };
    try testing.expectEqualDeep(
        BytecodeFunction{ .instructions = &expected },
        bytecode,
    );
}
