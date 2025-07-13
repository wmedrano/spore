const std = @import("std");
const testing = std.testing;

const Compiler = @import("Compiler.zig");
const ConsCell = @import("ConsCell.zig");
const ObjectPool = @import("datastructures/object_pool.zig").ObjectPool;
const Handle = @import("datastructures/object_pool.zig").Handle;
const StringInterner = @import("datastructures/StringInterner.zig");
const Symbol = @import("datastructures/Symbol.zig");
const ExecutionContext = @import("ExecutionContext.zig");
const Heap = @import("Heap.zig");
const Tokenizer = @import("parser/Tokenizer.zig");
const SexpParser = @import("SexpParser.zig");
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
    try builtins.AddFunction.register(&vm);
    try builtins.DefineFunction.register(&vm);
    return vm;
}

/// Deinitialize the VM.
pub fn deinit(self: *Vm) void {
    self.execution_context.deinit(self.heap.allocator);
    self.heap.deinit();
}

/// Evaluates a string of source code.
pub fn evalStr(self: *Vm, source: []const u8) !Val {
    var sexp_parser = try SexpParser.init(source);
    var arena = std.heap.ArenaAllocator.init(self.heap.allocator);
    defer arena.deinit();
    var compiler = Compiler.init(&arena, self);
    while (try sexp_parser.next(self.heap.allocator, self)) |expr| {
        try compiler.addExpr(expr);
    }
    var bytecode = try compiler.compile();
    defer bytecode.deinit(self.heap.allocator);
    for (bytecode.instructions) |instruction| {
        try instruction.execute(self);
    }
    return self.execution_context.stackTopVal() orelse Val.from({});
}

test evalStr {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    const return_val = try vm.evalStr("(+ 1 2 3 4)");
    try testing.expectEqualDeep(
        Val.from(10),
        return_val,
    );
}

test "evalStr returns last expression value for multiple expressions" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    const return_val = try vm.evalStr("1 2 3");
    try testing.expectEqualDeep(
        Val.from(3),
        return_val,
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
