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
    /// An attempt to define a variable with a non-symbol name.
    InvalidDefinitionName,
} || std.mem.Allocator.Error;

/// The `Vm` for the compiler.
vm: *Vm,
/// The allocator to use for temporary items.
arena: *std.heap.ArenaAllocator,
/// Symbols.
symbols: struct {
    @"if": Symbol.Interned,
    function: Symbol.Interned,
    @"return": Symbol.Interned,
    internal_define: Symbol.Interned,
    def: Symbol.Interned,
    defun: Symbol.Interned,
},
/// The compiled expression.
instructions: std.ArrayListUnmanaged(Instruction) = .{},
/// The variables that are in scope.
scoped_variables: std.ArrayListUnmanaged(ScopedVariable) = .{},

/// Represents a variable defined within the current compilation scope.
///
/// It tracks the variable's symbol and its corresponding index on the local
/// stack where its value will be stored or retrieved.
const ScopedVariable = struct {
    /// The symbol of the scoped variable.
    symbol: Symbol.Interned,
    /// The location of the symbol on the local stack.
    index: usize,
};

/// Initialize a new compiler.
///
/// The allocator to use for temporary items, such as instructions appended to
/// the current compilation context.
pub fn init(arena: *std.heap.ArenaAllocator, vm: *Vm) Error!Compiler {
    return try initFunction(arena, vm, Val.from({}));
}

/// Initializes a new compiler for compiling a function.
///
/// This function sets up a `Compiler` instance specifically for generating bytecode for a function.
/// It takes the allocator, the VM, and a `Val` representing the function's arguments.
/// The `args` `Val` is used to determine the number of arguments the function expects.
///
/// Args:
///     arena: The allocator to use for temporary items during compilation.
///     vm: The virtual machine instance.
///     args: A `Val` representing the function's arguments (e.g., `(a b)`).
///
/// Returns:
///     A new `Compiler` instance configured for function compilation.
fn initFunction(arena: *std.heap.ArenaAllocator, vm: *Vm, args: Val) Error!Compiler {
    const scoped_variables = blk: {
        if (std.meta.eql(args, Val.from({}))) break :blk std.ArrayListUnmanaged(ScopedVariable){};
        const args_list = try vm.heap.cons_cells.get(args.to(Handle(ConsCell)) catch return Error.InvalidExpression);
        var args_iter = args_list.iterList();

        var scoped_variables = std.ArrayListUnmanaged(ScopedVariable){};
        while (try args_iter.next(vm)) |val| {
            const scoped_variable = ScopedVariable{
                .symbol = val.to(Symbol.Interned) catch return Error.InvalidExpression,
                .index = scoped_variables.items.len,
            };
            try scoped_variables.append(arena.allocator(), scoped_variable);
        }
        break :blk scoped_variables;
    };
    return Compiler{
        .vm = vm,
        .arena = arena,
        .symbols = .{
            .@"if" = try Symbol.init("if").intern(vm.heap.allocator, &vm.heap.string_interner),
            .function = try Symbol.init("function").intern(vm.heap.allocator, &vm.heap.string_interner),
            .@"return" = try Symbol.init("return").intern(vm.heap.allocator, &vm.heap.string_interner),
            .internal_define = try Symbol.init("internal-define").intern(vm.heap.allocator, &vm.heap.string_interner),
            .def = try Symbol.init("def").intern(vm.heap.allocator, &vm.heap.string_interner),
            .defun = try Symbol.init("defun").intern(vm.heap.allocator, &vm.heap.string_interner),
        },
        .instructions = .{},
        .scoped_variables = scoped_variables,
    };
}

/// Finalizes the compilation process, returning the compiled `BytecodeFunction`.
///
/// The instructions accumulated during `addExpr` calls are duplicated using the
/// VM's heap allocator.
pub fn compile(self: *Compiler) !BytecodeFunction {
    const instructions = try self.vm.heap.allocator.dupe(Instruction, self.instructions.items);
    return .{
        .instructions = instructions,
        .args = self.scoped_variables.items.len,
    };
}

/// Returns true if the compiler has no instructions compiled yet, false
/// otherwise.
pub fn isEmpty(self: Compiler) bool {
    return self.instructions.items.len == 0;
}

/// Compiles a `Val` and adds it to the current compilation context.
pub fn addExpr(self: *Compiler, expr: Val) !void {
    switch (expr.repr) {
        .nil, .true_bool, .int, .float, .string, .native_function, .bytecode_function => {
            const instruction = Instruction.init(.{ .push = expr });
            try self.instructions.append(self.arena.allocator(), instruction);
        },
        .symbol => |s| {
            if (s.unquote()) |interned_symbol|
                try self.instructions.append(self.arena.allocator(), Instruction.init(.{ .push = Val.from(interned_symbol) }))
            else
                try self.deref(s);
        },
        .cons => |cons_handle| try self.addCons(cons_handle),
    }
}

/// Dereferences a symbol, generating the appropriate instruction.
///
/// If the symbol refers to a variable in the current compilation scope (a local
/// variable), a `.get` instruction is emitted to retrieve its value from the
/// local stack. Otherwise, a `.deref` instruction is emitted, indicating that
/// the symbol should be looked up in the global environment during runtime.
fn deref(self: *Compiler, symbol: Symbol.Interned) !void {
    if (self.getVariable(symbol)) |scoped_variable| {
        try self.instructions.append(
            self.arena.allocator(),
            Instruction.init(.{ .get = scoped_variable.index }),
        );
    } else {
        try self.instructions.append(
            self.arena.allocator(),
            Instruction.init(.{ .deref = symbol }),
        );
    }
}

/// Searches for a symbol within the currently defined scoped variables.
///
/// It iterates through the `scoped_variables` in reverse order (most recently
/// defined first) to find a `ScopedVariable` whose symbol matches the
/// provided `symbol`. This ensures that inner scopes shadow outer scopes.
///
/// Args:
///     symbol: The interned symbol to search for.
///
/// Returns:
///     The `ScopedVariable` if found, otherwise `null`.
fn getVariable(self: Compiler, symbol: Symbol.Interned) ?ScopedVariable {
    for (0..self.scoped_variables.items.len) |idx| {
        const variable = self.scoped_variables.items[self.scoped_variables.items.len - 1 - idx];
        if (std.meta.eql(variable.symbol, symbol)) return variable;
    }
    return null;
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
            if (std.meta.eql(val, Val.from(self.symbols.@"return")))
                return self.addReturn(&vals);
            if (std.meta.eql(val, Val.from(self.symbols.def)))
                return self.addDef(&vals);
            if (std.meta.eql(val, Val.from(self.symbols.defun)))
                return self.addDefun(&vals);
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

/// Compiles a `return` expression.
///
/// It expects at most one argument. If no argument is given, `nil` is used as
/// the return value. It generates code to evaluate the argument (if any) and
/// then a `ret` instruction. An error is returned if more than one argument is
/// provided.
fn addReturn(self: *Compiler, exprs: *ConsCell.ListIter) Error!void {
    const return_val = try exprs.next(self.vm);
    if ((try exprs.next(self.vm)) != null) return Error.InvalidExpression;

    if (return_val) |val|
        try self.addExpr(val)
    else
        try self.instructions.append(
            self.arena.allocator(),
            Instruction.init(.{ .push = Val.from({}) }),
        );
    try self.instructions.append(
        self.arena.allocator(),
        Instruction.init(.{ .ret = {} }),
    );
}

fn addDef(self: *Compiler, exprs: *ConsCell.ListIter) Error!void {
    const symbol = (try exprs.next(self.vm)) orelse return Error.InvalidExpression;
    const expr = (try exprs.next(self.vm)) orelse return Error.InvalidExpression;
    const interned_symbol = symbol.to(Symbol.Interned) catch return Error.InvalidExpression;
    if (interned_symbol.quoted) return Error.InvalidExpression;
    if (try exprs.next(self.vm)) |_| return Error.InvalidExpression;

    try self.instructions.append(self.arena.allocator(), Instruction.init(.{ .deref = self.symbols.internal_define }));
    try self.instructions.append(self.arena.allocator(), Instruction.init(.{ .push = symbol }));
    try self.addExpr(expr);
    try self.instructions.append(self.arena.allocator(), Instruction.init(.{ .eval = 3 }));
}

fn addDefun(self: *Compiler, exprs: *ConsCell.ListIter) Error!void {
    const symbol = (try exprs.next(self.vm)) orelse return Error.InvalidExpression;
    const interned_symbol = symbol.to(Symbol.Interned) catch return Error.InvalidExpression;
    if (interned_symbol.quoted) return Error.InvalidExpression;

    try self.instructions.append(self.arena.allocator(), Instruction.init(.{ .deref = self.symbols.internal_define }));
    try self.instructions.append(self.arena.allocator(), Instruction.init(.{ .push = symbol }));
    try self.addFunction(exprs);
    try self.instructions.append(self.arena.allocator(), Instruction.init(.{ .eval = 3 }));
}

/// Compiles a `function` expression.
///
/// It expects the function arguments (currently only `()` is supported) and
/// the function body. It compiles the function body into a new
/// `BytecodeFunction` and pushes a `Val` representing its handle onto the
/// current compilation context's instructions.
fn addFunction(self: *Compiler, exprs: *ConsCell.ListIter) Error!void {
    const args = (try exprs.next(self.vm)) orelse return Error.InvalidExpression;

    var function_compiler = try initFunction(self.arena, self.vm, args);
    while (try exprs.next(self.vm)) |expr| try function_compiler.addExpr(expr);
    if (function_compiler.isEmpty()) try function_compiler.addExpr(Val.from({}));

    const bytecode = try function_compiler.compile();
    const bytecode_handle = try self.vm.heap.bytecode_functions.create(
        self.vm.heap.allocator,
        bytecode,
        self.vm.heap.dead_color,
    );
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
    try compiler.addExpr(try Reader.readOne("(+ 1 (+ 2 3))", testing.allocator, &vm));
    var bytecode = try compiler.compile();
    defer bytecode.deinit(testing.allocator);

    try testing.expectEqualDeep(
        BytecodeFunction{
            .instructions = &[_]Instruction{
                Instruction.init(.{ .deref = plus_sym }),
                Instruction.init(.{ .push = Val.from(1) }),
                Instruction.init(.{ .deref = plus_sym }),
                Instruction.init(.{ .push = Val.from(2) }),
                Instruction.init(.{ .push = Val.from(3) }),
                Instruction.init(.{ .eval = 3 }),
                Instruction.init(.{ .eval = 3 }),
            },
            .args = 0,
        },
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
    const cons_handle = try vm.heap.cons_cells.create(
        vm.heap.allocator,
        cons,
        vm.heap.dead_color,
    );

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
    const plus_sym = try Symbol.init("plus").intern(
        testing.allocator,
        &vm.heap.string_interner,
    );

    try compiler.addExpr(try Reader.readOne("(plus 1 2)", testing.allocator, &vm));
    var bytecode = try compiler.compile();
    defer bytecode.deinit(testing.allocator);

    try testing.expectEqualDeep(
        BytecodeFunction{ .instructions = &[_]Instruction{
            Instruction.init(.{ .deref = plus_sym }),
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
    try compiler.addExpr(try Reader.readOne("'sym", testing.allocator, &vm));
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
    try compiler.addExpr(try Reader.readOne("(if (+ 1 2) (+ 3 4) (+ 5 6))", testing.allocator, &vm));
    var bytecode = try compiler.compile();
    defer bytecode.deinit(testing.allocator);

    try testing.expectEqualDeep(
        BytecodeFunction{
            .instructions = &[_]Instruction{
                Instruction.init(.{ .deref = plus_sym }),
                Instruction.init(.{ .push = Val.from(1) }),
                Instruction.init(.{ .push = Val.from(2) }),
                Instruction.init(.{ .eval = 3 }),
                Instruction.init(.{ .jump_if = 5 }),
                Instruction.init(.{ .deref = plus_sym }), // false branch starts here
                Instruction.init(.{ .push = Val.from(5) }),
                Instruction.init(.{ .push = Val.from(6) }),
                Instruction.init(.{ .eval = 3 }),
                Instruction.init(.{ .jump = 4 }),
                Instruction.init(.{ .deref = plus_sym }), // true branch starts here
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
    try compiler.addExpr(try Reader.readOne("(if (+ 1 2) (+ 3 4))", testing.allocator, &vm));
    var bytecode = try compiler.compile();
    defer bytecode.deinit(testing.allocator);

    try testing.expectEqualDeep(
        BytecodeFunction{
            .instructions = &[_]Instruction{
                Instruction.init(.{ .deref = plus_sym }),
                Instruction.init(.{ .push = Val.from(1) }),
                Instruction.init(.{ .push = Val.from(2) }),
                Instruction.init(.{ .eval = 3 }),
                Instruction.init(.{ .jump_if = 2 }),
                Instruction.init(.{ .push = Val.from({}) }), // false branch starts here
                Instruction.init(.{ .jump = 4 }),
                Instruction.init(.{ .deref = plus_sym }), // true branch starts here
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
    try compiler.addExpr(try Reader.readOne("(function () (+ 1 2))", testing.allocator, &vm));
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
    try testing.expectError(
        error.InvalidExpression,
        compiler.addExpr(try Reader.readOne("(function no-args (+ 1 2))", testing.allocator, &vm)),
    );
}

test "compile function body is compiled" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var arena = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena.deinit();
    var compiler = try init(&arena, &vm);
    try compiler.addExpr(try Reader.readOne("(function () 1 2 3)", testing.allocator, &vm));
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
    try compiler.addExpr(try Reader.readOne("(function ())", testing.allocator, &vm));
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

test "compile function with args has correct number of args" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var arena = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena.deinit();
    var compiler = try init(&arena, &vm);
    try compiler.addExpr(try Reader.readOne("(function (a b c))", testing.allocator, &vm));
    var bytecode = try compiler.compile();
    defer bytecode.deinit(testing.allocator);

    const function_bytecode = try vm.heap.bytecode_functions.get(
        try bytecode.instructions[0].repr.push.to(Handle(BytecodeFunction)),
    );
    try testing.expectEqualDeep(
        3,
        function_bytecode.args,
    );
}

test "compile function with reference to arg resolves to correct reference" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var arena = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena.deinit();
    var compiler = try init(&arena, &vm);
    try compiler.addExpr(try Reader.readOne("(function (func a b) (func a b))", testing.allocator, &vm));
    var bytecode = try compiler.compile();
    defer bytecode.deinit(testing.allocator);

    const function_bytecode = try vm.heap.bytecode_functions.get(
        try bytecode.instructions[0].repr.push.to(Handle(BytecodeFunction)),
    );
    try testing.expectEqualDeep(
        BytecodeFunction{
            .instructions = &[_]Instruction{
                Instruction.init(.{ .get = 0 }),
                Instruction.init(.{ .get = 1 }),
                Instruction.init(.{ .get = 2 }),
                Instruction.init(.{ .eval = 3 }),
            },
            .args = 3,
        },
        function_bytecode,
    );
}

test "compile return with value" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var arena = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena.deinit();
    var compiler = try init(&arena, &vm);
    try compiler.addExpr(try Reader.readOne("(return 10)", testing.allocator, &vm));
    var bytecode = try compiler.compile();
    defer bytecode.deinit(testing.allocator);

    try testing.expectEqualDeep(
        BytecodeFunction{
            .instructions = &[_]Instruction{
                Instruction.init(.{ .push = Val.from(10) }),
                Instruction.init(.{ .ret = {} }),
            },
            .args = 0,
        },
        bytecode,
    );
}

test "compile return without value" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var arena = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena.deinit();
    var compiler = try init(&arena, &vm);
    try compiler.addExpr(try Reader.readOne("(return)", testing.allocator, &vm));
    var bytecode = try compiler.compile();
    defer bytecode.deinit(testing.allocator);

    try testing.expectEqualDeep(
        BytecodeFunction{
            .instructions = &[_]Instruction{
                Instruction.init(.{ .push = Val.from({}) }),
                Instruction.init(.{ .ret = {} }),
            },
            .args = 0,
        },
        bytecode,
    );
}

test "compile return with too many args" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var arena = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena.deinit();
    var compiler = try init(&arena, &vm);
    try testing.expectError(
        error.InvalidExpression,
        compiler.addExpr(try Reader.readOne("(return 10 20)", testing.allocator, &vm)),
    );
}

test "compile def turns to define" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var arena = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena.deinit();
    var compiler = try init(&arena, &vm);

    try compiler.addExpr(try Reader.readOne("(def my-var 123)", testing.allocator, &vm));
    var bytecode = try compiler.compile();
    defer bytecode.deinit(testing.allocator);

    const define_sym = try Symbol.init("internal-define").intern(testing.allocator, &vm.heap.string_interner);
    const my_var_sym = try Symbol.init("my-var").intern(testing.allocator, &vm.heap.string_interner);
    try testing.expectEqualDeep(
        BytecodeFunction{
            .instructions = &[_]Instruction{
                Instruction.init(.{ .deref = define_sym }),
                Instruction.init(.{ .push = Val.from(my_var_sym) }),
                Instruction.init(.{ .push = Val.from(123) }),
                Instruction.init(.{ .eval = 3 }),
            },
            .args = 0,
        },
        bytecode,
    );
}

test "compile def with quoted symbol fails" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var arena = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena.deinit();
    var compiler = try init(&arena, &vm);

    try testing.expectError(
        Error.InvalidExpression,
        compiler.addExpr(try Reader.readOne("(def 'my-var 123)", testing.allocator, &vm)),
    );
}

test "compile def with multiple exprs fails" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var arena = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena.deinit();
    var compiler = try init(&arena, &vm);

    try testing.expectError(
        Error.InvalidExpression,
        compiler.addExpr(try Reader.readOne("(def my-var 10 20)", testing.allocator, &vm)),
    );
}

test "compile defun turns to define with correct function bytecode" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var arena = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena.deinit();
    var compiler = try init(&arena, &vm);

    try compiler.addExpr(try Reader.readOne("(defun my-func (a) a)", testing.allocator, &vm));
    var bytecode = try compiler.compile();
    defer bytecode.deinit(testing.allocator);

    const define_sym = try Symbol.init("internal-define").intern(testing.allocator, &vm.heap.string_interner);
    const my_func_sym = try Symbol.init("my-func").intern(testing.allocator, &vm.heap.string_interner);
    try testing.expectEqualDeep(
        BytecodeFunction{
            .instructions = &[_]Instruction{
                Instruction.init(.{ .deref = define_sym }),
                Instruction.init(.{ .push = Val.from(my_func_sym) }),
                bytecode.instructions[2],
                Instruction.init(.{ .eval = 3 }),
            },
            .args = 0,
        },
        bytecode,
    );
    const function_bytecode = try vm.heap.bytecode_functions.get(
        try bytecode.instructions[2].repr.push.to(Handle(BytecodeFunction)),
    );
    try testing.expectEqualDeep(
        BytecodeFunction{
            .instructions = &[_]Instruction{
                Instruction.init(.{ .get = 0 }),
            },
            .args = 1,
        },
        function_bytecode,
    );
}

test "compile defun with non-symbol name fails" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var arena = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena.deinit();
    var compiler = try init(&arena, &vm);

    try testing.expectError(
        Error.InvalidExpression,
        compiler.addExpr(try Reader.readOne("(defun 123 () 1)", testing.allocator, &vm)),
    );
}

test "compile defun with quoted symbol fails" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var arena = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena.deinit();
    var compiler = try init(&arena, &vm);

    try testing.expectError(
        Error.InvalidExpression,
        compiler.addExpr(try Reader.readOne("(defun 'my-func () 1)", testing.allocator, &vm)),
    );
}

test "compile defun with atom as args fails" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var arena = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena.deinit();
    var compiler = try init(&arena, &vm);

    try testing.expectError(
        Error.InvalidExpression,
        compiler.addExpr(try Reader.readOne("(defun my-func 1 2 3)", testing.allocator, &vm)),
    );
}

test "compile defun with missing args fails" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var arena = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena.deinit();
    var compiler = try init(&arena, &vm);

    try testing.expectError(
        Error.InvalidExpression,
        compiler.addExpr(try Reader.readOne("(defun my-func)", testing.allocator, &vm)),
    );
}
