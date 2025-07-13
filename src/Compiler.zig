//! Turns an s-expression (stored as a `Val`) into a `[]Instruction`.
const std = @import("std");
const testing = std.testing;

const BytecodeFunction = @import("BytecodeFunction.zig");
const ConsCell = @import("ConsCell.zig");
const Handle = @import("datastructures/object_pool.zig").Handle;
const Symbol = @import("datastructures/Symbol.zig");
const Instruction = @import("Instruction.zig");
const SexpParser = @import("SexpParser.zig");
const Val = @import("Val.zig");
const Vm = @import("Vm.zig");

const Compiler = @This();

pub const Error = error{
    /// A wrong type was supplied.
    TypeError,
    /// An error occurred while resolving a ConsCell handle.
    ObjectNotFound,
    /// An invalid expression. For example, an if statement without any
    /// arguments.
    InvalidExpression,
} || std.mem.Allocator.Error;

/// The `Vm` for the compiler.
vm: *Vm,
/// The allocator to use for temporary items.
arena: *std.heap.ArenaAllocator,
/// The compiled expression.
instructions: std.ArrayListUnmanaged(Instruction) = .{},
/// Symbols.
symbols: struct {
    if_sym: Symbol.Interned,
},

/// Initialize a new compiler.
///
/// The allocator to use for temporary items, such as instructions appended to
/// the current compilation context.
pub fn init(arena: *std.heap.ArenaAllocator, vm: *Vm) std.mem.Allocator.Error!Compiler {
    const if_sym = try Symbol.init("if").intern(vm.heap.allocator, &vm.heap.string_interner);
    return Compiler{
        .vm = vm,
        .arena = arena,
        .symbols = .{
            .if_sym = if_sym,
        },
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

fn addCons(self: *Compiler, cons_handle: Handle(ConsCell)) Error!void {
    const cons = try self.vm.heap.cons_cells.get(cons_handle);
    var vals = cons.iterList();
    var items: usize = 0;
    while (try vals.next(self.vm)) |val| {
        if (items == 0) {
            if (std.meta.eql(val, Val.from(self.symbols.if_sym)))
                return self.addIf(&vals);
        }
        try self.addExpr(val);
        items += 1;
    }

    try self.instructions.append(
        self.arena.allocator(),
        Instruction.init(.{ .eval = items }),
    );
}

fn jumpDistance(from: usize, to: usize) usize {
    return to - from;
}

fn addIf(self: *Compiler, exprs: *ConsCell.ListIter) Error!void {
    const pred = (try exprs.next(self.vm)) orelse return Error.InvalidExpression;
    const true_branch = (try exprs.next(self.vm)) orelse return Error.InvalidExpression;
    const false_branch = (try exprs.next(self.vm)) orelse Val.from({});
    if ((try exprs.next(self.vm)) != null) return Error.InvalidExpression;

    try self.addExpr(pred);
    const jump_if_idx = self.instructions.items.len;
    try self.instructions.append(self.arena.allocator(), Instruction.init(.{ .jump_if = 0 }));

    const false_branch_idx = self.instructions.items.len;
    try self.addExpr(false_branch);
    const false_jump_idx = self.instructions.items.len;
    try self.instructions.append(self.arena.allocator(), Instruction.init(.{ .jump = 0 }));

    const true_branch_idx = self.instructions.items.len;
    try self.addExpr(true_branch);
    const final_idx = self.instructions.items.len;

    self.instructions.items[jump_if_idx] = Instruction.init(.{
        .jump_if = jumpDistance(false_branch_idx, true_branch_idx),
    });
    self.instructions.items[false_jump_idx] = Instruction.init(
        .{ .jump = jumpDistance(true_branch_idx, final_idx) },
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
    var compiler = try init(&arena, &vm);
    var parser = try SexpParser.init("(plus 1 (plus 2 3))");
    const parsed_val = (try parser.next(testing.allocator, &vm)).?;

    try compiler.addExpr(parsed_val);
    var bytecode = try compiler.compile();
    defer bytecode.deinit(testing.allocator);

    try testing.expectEqualDeep(
        BytecodeFunction{ .instructions = &[_]Instruction{
            Instruction.init(.{ .get = plus_sym }),
            Instruction.init(.{ .push = Val.from(1) }),
            Instruction.init(.{ .get = plus_sym }),
            Instruction.init(.{ .push = Val.from(2) }),
            Instruction.init(.{ .push = Val.from(3) }),
            Instruction.init(.{ .eval = 3 }),
            Instruction.init(.{ .eval = 3 }),
        } },
        bytecode,
    );
}

test "compile improper list" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var arena = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena.deinit();
    var compiler = try init(&arena, &vm);
    const cons = ConsCell{
        .car = Val.from(try Symbol.init("a").intern(testing.allocator, &vm.heap.string_interner)),
        .cdr = Val.from(42),
    };
    const cons_handle = try vm.heap.cons_cells.create(vm.heap.allocator, cons);

    try testing.expectError(
        Compiler.Error.TypeError,
        compiler.addExpr(Val.from(cons_handle)),
    );
}

test "compile atom" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var arena = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena.deinit();
    var compiler = try init(&arena, &vm);

    try compiler.addExpr(Val.from(42));
    var bytecode = try compiler.compile();
    defer bytecode.deinit(testing.allocator);

    try testing.expectEqualDeep(
        BytecodeFunction{ .instructions = &[_]Instruction{Instruction.init(.{ .push = Val.from(42) })} },
        bytecode,
    );
}

test "compile simple list" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var arena = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena.deinit();
    var compiler = try init(&arena, &vm);
    var parser = try SexpParser.init("(plus 1 2)");
    const parsed_val = (try parser.next(testing.allocator, &vm)).?;
    const plus_sym = try Symbol.init("plus").intern(
        testing.allocator,
        &vm.heap.string_interner,
    );

    try compiler.addExpr(parsed_val);
    var bytecode = try compiler.compile();
    defer bytecode.deinit(testing.allocator);

    try testing.expectEqualDeep(
        BytecodeFunction{ .instructions = &[_]Instruction{
            Instruction.init(.{ .get = plus_sym }),
            Instruction.init(.{ .push = Val.from(1) }),
            Instruction.init(.{ .push = Val.from(2) }),
            Instruction.init(.{ .eval = 3 }),
        } },
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
    var compiler = try init(&arena, &vm);
    var parser = try SexpParser.init("'sym");
    const parsed_val = (try parser.next(testing.allocator, &vm)).?;

    try compiler.addExpr(parsed_val);
    var bytecode = try compiler.compile();
    defer bytecode.deinit(testing.allocator);

    try testing.expectEqualDeep(
        BytecodeFunction{ .instructions = &[_]Instruction{
            Instruction.init(.{ .push = Val.from(sym) }),
        } },
        bytecode,
    );
}

test "compile if statement" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    const plus_sym = try Symbol.init("+").intern(
        testing.allocator,
        &vm.heap.string_interner,
    );
    var arena = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena.deinit();
    var compiler = try init(&arena, &vm);
    var parser = try SexpParser.init("(if (+ 1 2) (+ 3 4) (+ 5 6))");
    const parsed_val = (try parser.next(testing.allocator, &vm)).?;

    try compiler.addExpr(parsed_val);
    var bytecode = try compiler.compile();
    defer bytecode.deinit(testing.allocator);

    try testing.expectEqualDeep(
        BytecodeFunction{
            .instructions = &[_]Instruction{
                Instruction.init(.{ .get = plus_sym }),
                Instruction.init(.{ .push = Val.from(1) }),
                Instruction.init(.{ .push = Val.from(2) }),
                Instruction.init(.{ .eval = 3 }),
                Instruction.init(.{ .jump_if = 5 }),
                Instruction.init(.{ .get = plus_sym }), // false branch starts here
                Instruction.init(.{ .push = Val.from(5) }),
                Instruction.init(.{ .push = Val.from(6) }),
                Instruction.init(.{ .eval = 3 }),
                Instruction.init(.{ .jump = 4 }),
                Instruction.init(.{ .get = plus_sym }), // true branch starts here
                Instruction.init(.{ .push = Val.from(3) }),
                Instruction.init(.{ .push = Val.from(4) }),
                Instruction.init(.{ .eval = 3 }),
            },
        },
        bytecode,
    );
}

test "compile if statement without false branch uses nil false branch" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    const plus_sym = try Symbol.init("+").intern(
        testing.allocator,
        &vm.heap.string_interner,
    );
    var arena = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena.deinit();
    var compiler = try init(&arena, &vm);
    var parser = try SexpParser.init("(if (+ 1 2) (+ 3 4))");
    const parsed_val = (try parser.next(testing.allocator, &vm)).?;

    try compiler.addExpr(parsed_val);
    var bytecode = try compiler.compile();
    defer bytecode.deinit(testing.allocator);

    try testing.expectEqualDeep(
        BytecodeFunction{
            .instructions = &[_]Instruction{
                Instruction.init(.{ .get = plus_sym }),
                Instruction.init(.{ .push = Val.from(1) }),
                Instruction.init(.{ .push = Val.from(2) }),
                Instruction.init(.{ .eval = 3 }),
                Instruction.init(.{ .jump_if = 2 }),
                Instruction.init(.{ .push = Val.from({}) }),
                Instruction.init(.{ .jump = 4 }),
                Instruction.init(.{ .get = plus_sym }), // true branch starts here
                Instruction.init(.{ .push = Val.from(3) }),
                Instruction.init(.{ .push = Val.from(4) }),
                Instruction.init(.{ .eval = 3 }),
            },
        },
        bytecode,
    );
}
