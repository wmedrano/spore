//! Turns an s-expression (stored as a `Val`) into a `[]Instruction`.
const std = @import("std");
const testing = std.testing;

const BytecodeFunction = @import("BytecodeFunction.zig");
const ConsCell = @import("ConsCell.zig");
const Handle = @import("datastructures/object_pool.zig").Handle;
const Symbol = @import("datastructures/Symbol.zig");
const Instruction = @import("instruction.zig").Instruction;
const LexicalScope = @import("LexicalScope.zig");
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
    /// An error internal to the interpreter. These should be reported.
    Internal,
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
    let: Symbol.Interned,
    @"for": Symbol.Interned,
    car: Symbol.Interned,
    cdr: Symbol.Interned,
},
/// The compiled expression.
instructions: std.ArrayListUnmanaged(Instruction) = .{},
/// The variables that are in scope.
lexical_scope: LexicalScope = .{},
/// The number of arguments.
arg_count: i32,

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
    var args_iter = switch (args.repr) {
        .nil => ConsCell.iterEmpty(),
        .cons => |handle| (try vm.heap.cons_cells.get(handle)).iterList(),
        else => return Error.InvalidExpression,
    };
    const lexical_scope = try LexicalScope.initWithArgs(arena.allocator(), vm, &args_iter);
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
            .let = try Symbol.init("let").intern(vm.heap.allocator, &vm.heap.string_interner),
            .@"for" = try Symbol.init("for").intern(vm.heap.allocator, &vm.heap.string_interner),
            .car = try Symbol.init("car").intern(vm.heap.allocator, &vm.heap.string_interner),
            .cdr = try Symbol.init("cdr").intern(vm.heap.allocator, &vm.heap.string_interner),
        },
        .instructions = .{},
        .lexical_scope = lexical_scope,
        .arg_count = lexical_scope.minimumLocalStackSize(),
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
        .args = self.arg_count,
        .initial_local_stack_size = self.lexical_scope.minimumLocalStackSize(),
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
        .boolean, .nil, .int, .float, .string, .native_function, .bytecode_function => {
            const instruction = Instruction{ .push = expr };
            try self.addInstruction(instruction);
        },
        .symbol => |s| {
            if (s.unquote()) |interned_symbol|
                try self.addInstruction(Instruction{ .push = Val.from(interned_symbol) })
            else
                try self.deref(s);
        },
        .cons => |cons_handle| try self.addCons(cons_handle),
    }
}

/// Compiles a series of expressions and adds them to the current compilation context.
///
/// This function iterates through a list of expressions, compiling each one.
/// The `top_val` parameter controls how the results of these expressions are
/// handled on the stack. If `top_val` is `.last`, only the result of the
/// last expression remains on the stack. If `top_val` is `.none`, all
/// results are popped.
///
/// Args:
///     exprs: An iterator over the expressions to compile.
///     top_val: An enum indicating whether to keep the last expression's value
///              on the stack (`.last`) or pop all values (`.none`).
fn addExprs(self: *Compiler, exprs: *ConsCell.ListIter, comptime top_val: enum { none, last }) !void {
    var exprs_count: i32 = 0;
    while (try exprs.next(self.vm)) |expr| {
        exprs_count += 1;
        try self.addExpr(expr);
    }
    if (top_val == .last) {
        switch (exprs_count) {
            0 => try self.addInstruction(Instruction{ .push = Val.from({}) }),
            1 => {},
            else => try self.addInstruction(Instruction{ .squash = exprs_count }),
        }
    }
    if (top_val == .none) {
        if (exprs_count > 0)
            try self.addInstruction(Instruction{ .pop = exprs_count });
    }
}

/// Appends a single instruction to the compiler's instruction list.
///
/// Args:
///     instruction: The instruction to add.
fn addInstruction(self: *Compiler, instruction: Instruction) !void {
    try self.instructions.append(self.arena.allocator(), instruction);
}

/// Appends a slice of instructions to the compiler's instruction list.
///
/// Args:
///     instructions: The slice of instructions to add.
fn addInstructions(self: *Compiler, instructions: []const Instruction) !void {
    try self.instructions.appendSlice(self.arena.allocator(), instructions);
}

/// Dereferences a symbol, generating the appropriate instruction.
///
/// If the symbol refers to a variable in the current compilation scope (a local
/// variable), a `.get` instruction is emitted to retrieve its value from the
/// local stack. Otherwise, a `.deref` instruction is emitted, indicating that
/// the symbol should be looked up in the global environment during runtime.
fn deref(self: *Compiler, symbol: Symbol.Interned) !void {
    if (self.lexical_scope.get(symbol)) |binding| {
        try self.addInstruction(Instruction{ .get = binding.local_index });
    } else {
        try self.addInstruction(Instruction{ .deref = symbol });
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
    var items: i32 = 0;
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
            if (std.meta.eql(val, Val.from(self.symbols.let)))
                return self.addLet(&vals);
            if (std.meta.eql(val, Val.from(self.symbols.@"for")))
                return self.addFor(&vals);
        }
        try self.addExpr(val);
        items += 1;
    }

    try self.addInstruction(Instruction{ .eval = items });
}

/// Calculates the distance between two instruction indices.
///
/// Used for computing offsets for jump instructions.
fn jumpDistance(from: usize, to: usize) i32 {
    const from_int: i32 = @intCast(from);
    const to_int: i32 = @intCast(to);
    return to_int - from_int;
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
    try self.addInstruction(Instruction{ .jump_if = 0 });

    const false_branch_idx = self.instructions.items.len;
    try self.addExpr(false_branch);
    const false_jump_idx = self.instructions.items.len;
    try self.addInstruction(Instruction{ .jump = 0 });

    const true_branch_idx = self.instructions.items.len;
    try self.addExpr(true_branch);
    const final_idx = self.instructions.items.len;

    self.instructions.items[jump_if_idx] = Instruction{
        .jump_if = jumpDistance(false_branch_idx, true_branch_idx),
    };
    self.instructions.items[false_jump_idx] =
        Instruction{ .jump = jumpDistance(true_branch_idx, final_idx) };
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
        try self.addInstruction(Instruction{ .push = Val.from({}) });
    try self.addInstruction(Instruction{ .ret = {} });
}

/// Compiles a `def` expression.
///
/// This special form defines a global variable. It expects two arguments:
/// a symbol representing the variable's name, and an expression whose
/// evaluated result will be the variable's value. This function emits
/// instructions to call the `internal-define` function to set the global variable.
///
/// Args:
///     exprs: An iterator over the expressions in the `def` expression.
///
/// Returns:
///     An error if the `def` expression is malformed (e.g., incorrect number
///     of arguments, non-symbol name, or a quoted symbol).
fn addDef(self: *Compiler, exprs: *ConsCell.ListIter) Error!void {
    const symbol = (try exprs.next(self.vm)) orelse return Error.InvalidExpression;
    const expr = (try exprs.next(self.vm)) orelse return Error.InvalidExpression;
    const interned_symbol = symbol.to(Symbol.Interned) catch return Error.InvalidExpression;
    if (interned_symbol.quoted) return Error.InvalidExpression;
    if (try exprs.next(self.vm)) |_| return Error.InvalidExpression;

    try self.addInstructions(&.{
        Instruction{ .deref = self.symbols.internal_define },
        Instruction{ .push = symbol },
    });
    try self.addExpr(expr);
    try self.addInstruction(Instruction{ .eval = 3 });
}

/// Compiles a `defun` expression.
///
/// This special form defines a global function. It expects a symbol for the
/// function's name, a list of arguments, and the function body. It leverages
/// `addFunction` to compile the function's bytecode and then uses the
/// `internal-define` mechanism to register the compiled function globally
/// under the given symbol.
///
/// Args:
///     exprs: An iterator over the expressions in the `defun` expression,
///            starting with the function name.
///
/// Returns:
///     An error if the `defun` expression is malformed.
fn addDefun(self: *Compiler, exprs: *ConsCell.ListIter) Error!void {
    const symbol = (try exprs.next(self.vm)) orelse return Error.InvalidExpression;
    const interned_symbol = symbol.to(Symbol.Interned) catch return Error.InvalidExpression;
    if (interned_symbol.quoted) return Error.InvalidExpression;

    try self.addInstructions(&.{
        Instruction{ .deref = self.symbols.internal_define },
        Instruction{ .push = symbol },
    });
    try self.addFunction(exprs);
    try self.addInstruction(Instruction{ .eval = 3 });
}

/// Compiles a `function` expression.
///
/// It expects the function arguments and
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
    try self.addInstruction(Instruction{ .push = Val.from(bytecode_handle) });
}

// Compiles a `let` expression.
// A `let` expression introduces new local variables within its scope.
// It takes a list of bindings, where each binding is a pair of a variable name
// (a symbol) and an expression to evaluate for that variable's initial value.
// After evaluating and setting the bindings, it evaluates the body of the `let`
// expression. If the `let` body has multiple expressions, they are compiled
// with a `squash` instruction to return the value of the last expression.
// If there are no expressions in the body, `nil` is returned.
//  * Args:
//     exprs: An iterator over the expressions in the `let` expression.
//  * Returns:
//     An error if the `let` expression is malformed or if there is an issue
//     during compilation.
fn addLet(self: *Compiler, exprs: *ConsCell.ListIter) Error!void {
    const bindings = (try exprs.next(self.vm)) orelse return Error.InvalidExpression;
    var bindings_iter = try self.vm.inspector().listIter(bindings);
    var lexical_binds = std.ArrayList(LexicalScope.Binding).init(self.arena.allocator());
    defer {
        for (lexical_binds.items) |b| self.lexical_scope.remove(b);
        lexical_binds.deinit();
    }
    while (try bindings_iter.next(self.vm)) |binding| {
        var binding_parts = try self.vm.inspector().listIter(binding);
        const binding_name = try binding_parts.next(self.vm) orelse return Error.InvalidExpression;
        const binding_expr = try binding_parts.next(self.vm) orelse return Error.InvalidExpression;
        if (try binding_parts.next(self.vm)) |_| return Error.InvalidExpression;
        try self.addExpr(binding_expr);
        const name = binding_name.to(Symbol.Interned) catch return Error.InvalidExpression;
        const lexical_bind = try self.lexical_scope.add(self.arena.allocator(), name);
        try lexical_binds.append(lexical_bind);
        try self.addInstruction(Instruction{ .set = lexical_bind.local_index });
    }
    try self.addExprs(exprs, .last);
}

/// Compiles a `for` expression.
///
/// This special form implements a loop that iterates over a list. It expects
/// two main parts: a binding (a symbol for the iteration variable and an
/// expression that evaluates to a list), and a body of expressions to be
/// executed for each element in the list. The function generates bytecode
/// to manage the iteration, including checking for loop termination,
/// binding the current list element, and advancing to the next.
///
/// Args:
///     exprs: An iterator over the expressions in the `for` expression.
///
/// Returns:
///     An error if the `for` expression is malformed or if there is an issue
///     during compilation.
fn addFor(self: *Compiler, exprs: *ConsCell.ListIter) Error!void {
    // Get bindings
    const bindings = (try exprs.next(self.vm)) orelse return Error.InvalidExpression;
    var bindings_iter = try self.vm.inspector().listIter(bindings);
    const binding_name_val = (try bindings_iter.next(self.vm)) orelse return Error.InvalidExpression;
    const binding_name = binding_name_val.to(Symbol.Interned) catch return Error.InvalidExpression;
    const iterable_expr = (try bindings_iter.next(self.vm)) orelse return Error.InvalidExpression;
    if (try bindings_iter.next(self.vm)) |_| return Error.InvalidExpression;

    // Setup: Evaluate and store cons.
    const next_bind = try self.lexical_scope.add(self.arena.allocator(), binding_name);
    const iterable_bind = try self.lexical_scope.addAnonymous(self.arena.allocator());
    if (next_bind.local_index + 1 != iterable_bind.local_index) return Error.Internal;
    try self.addExpr(iterable_expr);

    defer self.lexical_scope.remove(iterable_bind);
    try self.addInstruction(Instruction{ .set = iterable_bind.local_index });
    // Loop: Set next value and advance iterator.
    defer self.lexical_scope.remove(next_bind);
    const loop_start_idx = self.instructions.items.len;
    try self.addInstruction(
        Instruction{ .iter_next = .{ .index = next_bind.local_index } },
    );
    // Loop: Check exit criteria.
    const loop_exit_idx = self.instructions.items.len;
    try self.addInstruction(Instruction{ .jump_if_not = 0 }); // Fixed at end, see FIX.
    // Loop: Eval expressions
    try self.addExprs(exprs, .none);
    // Loop: Jump to start of loop.
    const loop_jump_to_start_idx = self.instructions.items.len;
    try self.addInstruction(Instruction{ .jump = jumpDistance(loop_jump_to_start_idx + 1, loop_start_idx) });
    const loop_end_idx = self.instructions.items.len;

    // FIX, see prior FIX.
    self.instructions.items[loop_exit_idx].jump_if_not = jumpDistance(loop_exit_idx + 1, loop_end_idx);
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
                Instruction{ .deref = plus_sym },
                Instruction{ .push = Val.from(1) },
                Instruction{ .deref = plus_sym },
                Instruction{ .push = Val.from(2) },
                Instruction{ .push = Val.from(3) },
                Instruction{ .eval = 3 },
                Instruction{ .eval = 3 },
            },
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
        BytecodeFunction{ .instructions = &[_]Instruction{Instruction{ .push = Val.from(42) }} },
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
            Instruction{ .deref = plus_sym },
            Instruction{ .push = Val.from(1) },
            Instruction{ .push = Val.from(2) },
            Instruction{ .eval = 3 },
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
            Instruction{ .push = Val.from(sym) },
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
                Instruction{ .deref = plus_sym },
                Instruction{ .push = Val.from(1) },
                Instruction{ .push = Val.from(2) },
                Instruction{ .eval = 3 },
                Instruction{ .jump_if = 5 },
                Instruction{ .deref = plus_sym }, // false branch starts here
                Instruction{ .push = Val.from(5) },
                Instruction{ .push = Val.from(6) },
                Instruction{ .eval = 3 },
                Instruction{ .jump = 4 },
                Instruction{ .deref = plus_sym }, // true branch starts here
                Instruction{ .push = Val.from(3) },
                Instruction{ .push = Val.from(4) },
                Instruction{ .eval = 3 },
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
                Instruction{ .deref = plus_sym },
                Instruction{ .push = Val.from(1) },
                Instruction{ .push = Val.from(2) },
                Instruction{ .eval = 3 },
                Instruction{ .jump_if = 2 },
                Instruction{ .push = Val.from({}) }, // false branch starts here
                Instruction{ .jump = 4 },
                Instruction{ .deref = plus_sym }, // true branch starts here
                Instruction{ .push = Val.from(3) },
                Instruction{ .push = Val.from(4) },
                Instruction{ .eval = 3 },
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
    _ = try bytecode.instructions[0].push.to(Handle(BytecodeFunction));
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
        try bytecode.instructions[0].push.to(Handle(BytecodeFunction)),
    );
    try testing.expectEqualDeep(
        BytecodeFunction{
            .instructions = &[_]Instruction{
                Instruction{ .push = Val.from(1) },
                Instruction{ .push = Val.from(2) },
                Instruction{ .push = Val.from(3) },
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
        try bytecode.instructions[0].push.to(Handle(BytecodeFunction)),
    );
    try testing.expectEqualDeep(
        BytecodeFunction{
            .instructions = &[_]Instruction{
                Instruction{ .push = Val.from({}) },
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
        try bytecode.instructions[0].push.to(Handle(BytecodeFunction)),
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
        try bytecode.instructions[0].push.to(Handle(BytecodeFunction)),
    );
    try testing.expectEqualDeep(
        BytecodeFunction{
            .instructions = &[_]Instruction{
                Instruction{ .get = 0 },
                Instruction{ .get = 1 },
                Instruction{ .get = 2 },
                Instruction{ .eval = 3 },
            },
            .args = 3,
            .initial_local_stack_size = 3,
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
                Instruction{ .push = Val.from(10) },
                Instruction{ .ret = {} },
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
                Instruction{ .push = Val.from({}) },
                Instruction{ .ret = {} },
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
                Instruction{ .deref = define_sym },
                Instruction{ .push = Val.from(my_var_sym) },
                Instruction{ .push = Val.from(123) },
                Instruction{ .eval = 3 },
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
                Instruction{ .deref = define_sym },
                Instruction{ .push = Val.from(my_func_sym) },
                bytecode.instructions[2],
                Instruction{ .eval = 3 },
            },
            .args = 0,
        },
        bytecode,
    );
    const function_bytecode = try vm.heap.bytecode_functions.get(
        try bytecode.instructions[2].push.to(Handle(BytecodeFunction)),
    );
    try testing.expectEqualDeep(
        BytecodeFunction{
            .instructions = &[_]Instruction{
                Instruction{ .get = 0 },
            },
            .args = 1,
            .initial_local_stack_size = 1,
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

test "let evaluates bindings" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var arena = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena.deinit();
    var compiler = try init(&arena, &vm);

    try compiler.addExpr(try Reader.readOne("(let ((x (+ 1 2)) (y (+ x 3))) (+ x y))", testing.allocator, &vm));
    var bytecode = try compiler.compile();
    defer bytecode.deinit(testing.allocator);
    const plus_sym = try Symbol.init("+").intern(testing.allocator, &vm.heap.string_interner);
    try testing.expectEqualDeep(
        BytecodeFunction{
            .instructions = &[_]Instruction{
                Instruction{ .deref = plus_sym },
                Instruction{ .push = Val.from(1) },
                Instruction{ .push = Val.from(2) },
                Instruction{ .eval = 3 },
                Instruction{ .set = 0 },
                Instruction{ .deref = plus_sym },
                Instruction{ .get = 0 },
                Instruction{ .push = Val.from(3) },
                Instruction{ .eval = 3 },
                Instruction{ .set = 1 },
                Instruction{ .deref = plus_sym },
                Instruction{ .get = 0 },
                Instruction{ .get = 1 },
                Instruction{ .eval = 3 },
            },
            .args = 0,
            .initial_local_stack_size = 2,
        },
        bytecode,
    );
}

test "multiple expressions in let squashed to single" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var arena = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena.deinit();
    var compiler = try init(&arena, &vm);

    try compiler.addExpr(try Reader.readOne("(let () 1 2 3 4)", testing.allocator, &vm));
    var bytecode = try compiler.compile();
    defer bytecode.deinit(testing.allocator);
    try testing.expectEqualDeep(
        BytecodeFunction{
            .instructions = &[_]Instruction{
                Instruction{ .push = Val.from(1) },
                Instruction{ .push = Val.from(2) },
                Instruction{ .push = Val.from(3) },
                Instruction{ .push = Val.from(4) },
                Instruction{ .squash = 4 },
            },
            .args = 0,
            .initial_local_stack_size = 0,
        },
        bytecode,
    );
}
