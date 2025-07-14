//! Turns an s-expression (stored as a `Val`) into a `[]Instruction`.
const std = @import("std");
const testing = std.testing;

const BytecodeFunction = @import("BytecodeFunction.zig");
const ConsCell = @import("ConsCell.zig");
const Handle = @import("datastructures/object_pool.zig").Handle;
const Symbol = @import("datastructures/Symbol.zig");
const Instruction = @import("Instruction.zig");
const Reader = @import("Reader.zig");
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
    @"if": Symbol.Interned,
    function: Symbol.Interned,
},

/// Initialize a new compiler.
///
/// The allocator to use for temporary items, such as instructions appended to
/// the current compilation context.
pub fn init(arena: *std.heap.ArenaAllocator, vm: *Vm) std.mem.Allocator.Error!Compiler {
    return Compiler{
        .vm = vm,
        .arena = arena,
        .symbols = .{
            .@"if" = try Symbol.init("if").intern(vm.heap.allocator, &vm.heap.string_interner),
            .function = try Symbol.init("function").intern(vm.heap.allocator, &vm.heap.string_interner),
        },
    };
}

/// Finalizes the compilation process, returning the compiled `BytecodeFunction`.
///
/// The instructions accumulated during `addExpr` calls are duplicated using the
/// VM's heap allocator.
pub fn compile(self: *Compiler) !BytecodeFunction {
    const instructions = try self.vm.heap.allocator.dupe(Instruction, self.instructions.items);
    return .{ .instructions = instructions };
}

/// Returns true if the compiler has no instructions compiled yet, false
/// otherwise.
pub fn isEmpty(self: Compiler) bool {
    return self.instructions.items.len == 0;
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

/// Compiles a `ConsCell` expression.
///
/// Handles special forms like `if` and `function` by calling their respective
/// compilation helpers. For regular list expressions, it compiles each element
/// and appends an `eval` instruction.
fn addCons(self: *Compiler, cons_handle: Handle(ConsCell)) Error!void {
    const cons = try self.vm.heap.cons_cells.get(cons_handle);
    var vals = cons.iterList();
    var items: usize = 0;
    while (try vals.next(self.vm)) |val| {
        if (items == 0) {
            if (std.meta.eql(val, Val.from(self.symbols.@"if")))
                return self.addIf(&vals);
            if (std.meta.eql(val, Val.from(self.symbols.function)))
                return self.addFunction(&vals);
        }
        try self.addExpr(val);
        items += 1;
    }

    try self.instructions.append(
        self.arena.allocator(),
        Instruction.init(.{ .eval = items }),
    );
}

/// Calculates the distance between two instruction indices.
///
/// Used for computing offsets for jump instructions.
fn jumpDistance(from: usize, to: usize) usize {
    return to - from;
}

/// Compiles an `if` expression.
///
/// It expects three arguments: a predicate, a true branch, and an optional
/// false branch. It generates `jump_if` and `jump` instructions to implement
/// the conditional logic.
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

/// Compiles a `function` expression.
///
/// It expects the function arguments (currently only `()` is supported) and
/// the function body. It compiles the function body into a new
/// `BytecodeFunction` and pushes a `Val` representing its handle onto the
/// current compilation context's instructions.
fn addFunction(self: *Compiler, exprs: *ConsCell.ListIter) Error!void {
    const args = (try exprs.next(self.vm)) orelse return Error.InvalidExpression;
    if (!std.meta.eql(args, Val.from({}))) return Error.InvalidExpression;

    var function_compiler = try init(self.arena, self.vm);
    while (try exprs.next(self.vm)) |expr| try function_compiler.addExpr(expr);
    if (function_compiler.isEmpty()) try function_compiler.addExpr(Val.from({}));

    const bytecode = try function_compiler.compile();
    const bytecode_handle = try self.vm.heap.bytecode_functions.create(self.vm.heap.allocator, bytecode);
    try self.instructions.append(
        self.arena.allocator(),
        Instruction.init(.{ .push = Val.from(bytecode_handle) }),
    );
}

test compile {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    const plus_sym = try Symbol.init("+").intern(
        testing.allocator,
        &vm.heap.string_interner,
    );
    var arena = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena.deinit();
    var compiler = try init(&arena, &vm);
    var parser = try Reader.init("(+ 1 (+ 2 3))");
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
    var parser = try Reader.init("(plus 1 2)");
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
    var parser = try Reader.init("'sym");
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
    var parser = try Reader.init("(if (+ 1 2) (+ 3 4) (+ 5 6))");
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
    var parser = try Reader.init("(if (+ 1 2) (+ 3 4))");
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
                Instruction.init(.{ .push = Val.from({}) }), // false branch starts here
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

test "compile function makes function" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var arena = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena.deinit();
    var compiler = try init(&arena, &vm);
    var parser = try Reader.init("(function () (+ 1 2))");
    const parsed_val = (try parser.next(testing.allocator, &vm)).?;

    try compiler.addExpr(parsed_val);
    var bytecode = try compiler.compile();
    defer bytecode.deinit(testing.allocator);

    try testing.expectEqual(1, bytecode.instructions.len);
    _ = try bytecode.instructions[0].repr.push.to(Handle(BytecodeFunction));
}

test "compile function without args is compile error" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var arena = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena.deinit();
    var compiler = try init(&arena, &vm);
    var parser = try Reader.init("(function no-args (+ 1 2))");
    const parsed_val = (try parser.next(testing.allocator, &vm)).?;

    try testing.expectError(
        error.InvalidExpression,
        compiler.addExpr(parsed_val),
    );
}

test "compile function body is compiled" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var arena = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena.deinit();
    var compiler = try init(&arena, &vm);
    var parser = try Reader.init("(function () 1 2 3)");
    const parsed_val = (try parser.next(testing.allocator, &vm)).?;

    try compiler.addExpr(parsed_val);
    var bytecode = try compiler.compile();
    defer bytecode.deinit(testing.allocator);

    const function_bytecode = try vm.heap.bytecode_functions.get(
        try bytecode.instructions[0].repr.push.to(Handle(BytecodeFunction)),
    );
    try testing.expectEqualDeep(
        BytecodeFunction{
            .instructions = &[_]Instruction{
                Instruction.init(.{ .push = Val.from(1) }),
                Instruction.init(.{ .push = Val.from(2) }),
                Instruction.init(.{ .push = Val.from(3) }),
            },
        },
        function_bytecode,
    );
}

test "compile function without body has nil body" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var arena = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena.deinit();
    var compiler = try init(&arena, &vm);
    var parser = try Reader.init("(function ())");
    const parsed_val = (try parser.next(testing.allocator, &vm)).?;

    try compiler.addExpr(parsed_val);
    var bytecode = try compiler.compile();
    defer bytecode.deinit(testing.allocator);

    const function_bytecode = try vm.heap.bytecode_functions.get(
        try bytecode.instructions[0].repr.push.to(Handle(BytecodeFunction)),
    );
    try testing.expectEqualDeep(
        BytecodeFunction{
            .instructions = &[_]Instruction{
                Instruction.init(.{ .push = Val.from({}) }),
            },
        },
        function_bytecode,
    );
}
