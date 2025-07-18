const std = @import("std");
const testing = std.testing;

const BytecodeFunction = @import("BytecodeFunction.zig");
const Compiler = @import("Compiler.zig");
const ConsCell = @import("ConsCell.zig");
const ObjectPool = @import("datastructures/object_pool.zig").ObjectPool;
const Handle = @import("datastructures/object_pool.zig").Handle;
const StringInterner = @import("datastructures/StringInterner.zig");
const Symbol = @import("datastructures/Symbol.zig");
const ExecutionContext = @import("ExecutionContext.zig");
const GarbageCollector = @import("GarbageCollector.zig");
const Heap = @import("Heap.zig");
const Tokenizer = @import("parser/Tokenizer.zig");
const PrettyPrinter = @import("PrettyPrinter.zig");
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
    var bytecode = try compiler.compile();
    defer bytecode.deinit(self.heap.allocator);

    const initial_call_stack_size = self.execution_context.previous_call_frames.len;
    try self.execution_context.pushVal(Val.from({}));
    try self.execution_context.pushCallFrame(ExecutionContext.CallFrame{
        .instructions = bytecode.instructions,
        .stack_start = self.execution_context.stack.len,
    });
    while (initial_call_stack_size < self.execution_context.previous_call_frames.len) {
        const instruction = self.execution_context.nextInstruction();
        try instruction.execute(self);
    }

    return try self.execution_context.popVal();
}

/// Return an object that can pretty print `val` when formatted.
pub fn prettyPrint(self: *const Vm, val: Val) PrettyPrinter {
    return PrettyPrinter.init(self, val);
}

/// Return an object that can pretty print `vals` when formatted.
pub fn prettyPrintSlice(self: *const Vm, vals: []const Val) PrettyPrinter.Slice {
    return PrettyPrinter.initSlice(self, vals);
}

/// Triggers a garbage collection cycle to clean up unused memory.
pub fn garbageCollect(self: *Vm) !void {
    var gc = GarbageCollector.init(self);
    try gc.run();
}

test evalStr {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectEqualDeep(
        Val.from(24),
        try vm.evalStr("(def x 12) (+ x x)"),
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
