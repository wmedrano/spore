const std = @import("std");
const testing = std.testing;

const BytecodeFunction = @import("BytecodeFunction.zig");
const Compiler = @import("Compiler.zig");
const ObjectPool = @import("datastructures/object_pool.zig").ObjectPool;
const Handle = @import("datastructures/object_pool.zig").Handle;
const StringInterner = @import("datastructures/StringInterner.zig");
const Symbol = @import("datastructures/Symbol.zig");
const ExecutionContext = @import("ExecutionContext.zig");
const GarbageCollector = @import("GarbageCollector.zig");
const Heap = @import("Heap.zig");
const Inspector = @import("Inspector.zig");
const Instruction = @import("instruction.zig").Instruction;
const NativeFunction = @import("NativeFunction.zig");
const Reader = @import("Reader.zig");
const Val = @import("Val.zig");

const Vm = @This();

/// Contains things such as the working call stack and data stack.
execution_context: ExecutionContext,
/// Contains all objects.
heap: Heap,

/// Create a new VM.
pub fn init(allocator: std.mem.Allocator) !Vm {
    var vm = Vm{
        .execution_context = .{},
        .heap = Heap.init(allocator),
    };
    const builtins = @import("builtins.zig");
    try builtins.registerAll(&vm);
    return vm;
}

/// Deinitialize the VM.
pub fn deinit(self: *Vm) void {
    self.execution_context.deinit(self.heap.allocator);
    self.heap.deinit();
}

/// Evaluates a string of source code.
pub fn evalStr(self: *Vm, source: []const u8) !Val {
    var reader = try Reader.init(source);
    var arena = std.heap.ArenaAllocator.init(self.heap.allocator);
    defer arena.deinit();
    var compiler = try Compiler.init(&arena, self);
    while (try reader.next(self.heap.allocator, self)) |expr| {
        try compiler.addExpr(expr);
    }
    const bytecode_handle = try self.heap.bytecode_functions.create(
        self.heap.allocator,
        try compiler.compile(),
        self.heap.dead_color,
    );

    const initial_call_stack_size = self.execution_context.previous_call_frames.len;
    try self.execution_context.pushVal(Val.from(bytecode_handle));
    try (Instruction{ .eval = 1 }).execute(self);
    while (initial_call_stack_size < self.execution_context.previous_call_frames.len) {
        const instruction = self.execution_context.nextInstruction();
        try instruction.execute(self);
    }

    return try self.execution_context.popVal();
}

/// Get a value that can be used to inspect values.
pub fn inspector(self: *const Vm) Inspector {
    return Inspector{ .vm = self };
}

/// Triggers a garbage collection cycle to clean up unused memory.
pub fn garbageCollect(self: *Vm) !void {
    var gc = GarbageCollector.init(self);
    try gc.run();
}

/// Add the function to a `Vm`'s global namespace.
pub fn registerFunction(self: *const NativeFunction, vm: *Vm) !void {
    try vm.execution_context.setGlobal(
        vm.heap.allocator,
        try Symbol.init(self.name).intern(vm.heap.allocator, &vm.heap.string_interner),
        Val.from(self),
    );
}

test evalStr {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    const source =
        \\ ; Define a global variable.
        \\ (def squared-sum 0)
        \\
        \\ ; Define a function.
        \\ (defun square (number) (* number number))
        \\
        \\ ; Iterate over a list
        \\ (for (x (list 1 2 3 4))
        \\   (let* ((squared (square x))
        \\         (new-sum (+ squared squared-sum)))
        \\     (def squared-sum new-sum)))
        \\ squared-sum
    ;
    try testing.expectEqualDeep(
        Val.from(30),
        try vm.evalStr(source),
    );
}

test "evalStr returns last expression value for multiple expressions" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.from(3),
        try vm.evalStr("1 2 3"),
    );
}

test "evalStr returns error for bad syntax" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectError(
        error.ParseError,
        vm.evalStr("))"),
    );
}

test "evalStr with bad syntax doesn't evaluate any code" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectError(
        error.ParseError,
        vm.evalStr("(+ 1 2 3 4) (+ 1 2))"),
    );
    try testing.expectEqualDeep(
        &.{},
        vm.execution_context.stack.constSlice(),
    );
}

test "can eval function" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.from(3),
        vm.evalStr("((function (a b) (+ a b)) 1 2)"),
    );
}

test "or with no arguments returns nil" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.from({}),
        try vm.evalStr("(or)"),
    );
}

test "or with truthy argument returns that value" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.from(42),
        try vm.evalStr("(or 42)"),
    );
}

test "or short-circuits evaluation on true" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    const source = "(or false nil 10 20 (uncalled))";
    try testing.expectEqualDeep(
        Val.from(10),
        try vm.evalStr(source),
    );
}

test "or does not short-circuits evaluation on false" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    const source = "(or false (uncalled))";
    try testing.expectError(
        error.SymbolNotFound,
        vm.evalStr(source),
    );
}

test "or with all falsy arguments returns last value" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.from({}),
        try vm.evalStr("(or false (empty? (list 1)) nil)"),
    );
    try testing.expectEqualDeep(
        Val.from(false),
        try vm.evalStr("(or nil (empty? (list 1)) false)"),
    );
}

test "and with no arguments returns an error" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectError(
        error.InvalidExpression,
        vm.evalStr("(and)"),
    );
}

test "and with truthy argument returns that value" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.from(42),
        try vm.evalStr("(and 42)"),
    );
}

test "and short-circuits evaluation on false" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    const source = "(and 10 20 false (uncalled))";
    try testing.expectEqualDeep(
        Val.from(false),
        try vm.evalStr(source),
    );
}

test "and does not short-circuit evaluation on true" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    const source = "(and true (uncalled))";
    try testing.expectError(
        error.SymbolNotFound,
        vm.evalStr(source),
    );
}

test "and with all truthy arguments returns last value" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.from({}),
        try vm.evalStr("(and 10 20 nil)"),
    );
    try testing.expectEqualDeep(
        Val.from(false),
        try vm.evalStr("(and 10 20 false)"),
    );
    try testing.expectEqualDeep(
        Val.from(30),
        try vm.evalStr("(and 10 20 30)"),
    );
}

test "and with all falsy arguments returns first value" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.from({}),
        try vm.evalStr("(and nil false)"),
    );
    try testing.expectEqualDeep(
        Val.from(false),
        try vm.evalStr("(and false nil)"),
    );
}

test garbageCollect {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    const func = try (try vm.evalStr("(function (a b) (+ a b))")).to(Handle(BytecodeFunction));
    try vm.garbageCollect();
    try testing.expectError(
        error.ObjectNotFound,
        vm.heap.bytecode_functions.get(func),
    );
}
