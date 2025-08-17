//! Helper for constructing Spore values from native Zig types.
//!
//! The Builder provides a convenient interface for converting Zig types into
//! Spore's Val representation, handling memory allocation and type conversion
//! automatically. It supports various native types including primitives,
//! strings, arrays, and custom Spore types.

const std = @import("std");
const testing = std.testing;

const BytecodeFunction = @import("BytecodeFunction.zig");
const errors = @import("errors.zig");
const DetailedError = errors.DetailedError;
const Handle = @import("object_pool.zig").Handle;
const NativeFunction = @import("NativeFunction.zig");
const Pair = @import("Pair.zig");
const PrettyPrinter = @import("PrettyPrinter.zig");
const String = @import("String.zig");
const Symbol = @import("Symbol.zig");
const Val = @import("Val.zig");
const Vm = @import("Vm.zig");

const Builder = @This();

/// Reference to the virtual machine that owns the heap and execution context.
/// Used for memory allocation and error handling during value construction.
vm: *Vm,

/// Converts a native Zig value into a Spore Val.
///
/// This function provides polymorphic conversion from various Zig types to
/// Spore's unified value representation. It handles memory allocation for
/// heap-allocated types and performs necessary type conversions.
///
/// Supported types:
/// - Primitives: void, bool, i64, f64, comptime_int, comptime_float
/// - Spore types: Symbol.Interned, Handle(Pair), Handle(String), etc.
/// - Strings: []u8, []const u8 (allocated and copied)
/// - Arrays: []Val, []const Val (converted to linked lists)
/// - Custom types: Symbol, Pair (converted to heap-allocated versions)
///
/// Args:
///     val: The value to convert (type determined at compile time)
///
/// Returns:
///     A Spore Val containing the converted value
pub fn init(self: Builder, val: anytype) errors.Error!Val {
    const T = @TypeOf(val);
    switch (T) {
        // Types supported by Val.init that don't need Builder processing
        void => return Val.init(val),
        bool => return Val.init(val),
        i64, comptime_int => return Val.init(val),
        f64, comptime_float => return Val.init(val),
        Symbol.Interned => return Val.init(val),
        Handle(Pair) => return Val.init(val),
        Handle(String) => return Val.init(val),
        *const NativeFunction => return Val.init(val),
        Handle(BytecodeFunction) => return Val.init(val),
        Handle(DetailedError) => return Val.init(val),
        // Additional types that need Builder processing
        []u8, []const u8 => {
            const handle = try self.vm.heap.strings.create(
                self.vm.heap.allocator,
                try String.initCopy(self.vm.heap.allocator, val),
                self.vm.heap.unreachable_color,
            );
            return Val.init(handle);
        },
        []Val, []const Val => {
            var result = Val.init({});
            var i: usize = val.len;
            while (i > 0) {
                i -= 1;
                const head = val[i];
                const pair_handle = self.vm.heap.pairs.create(
                    self.vm.heap.allocator,
                    Pair.init(head, result),
                    self.vm.heap.unreachable_color,
                ) catch |err| switch (err) {
                    error.OutOfMemory => return self.addError(DetailedError{ .out_of_memory = {} }),
                };
                result = try self.init(pair_handle);
            }
            return result;
        },
        Symbol => {
            const interned = try self.internSymbol(val);
            return Val.init(interned);
        },
        Pair => {
            const handle = self.vm.heap.pairs.create(
                self.vm.heap.allocator,
                Pair.init(val.first, val.second),
                self.vm.heap.unreachable_color,
            ) catch |err| switch (err) {
                error.OutOfMemory => return self.addError(DetailedError{ .out_of_memory = {} }),
            };
            return self.init(handle);
        },
        DetailedError => {
            const handle = self.vm.heap.detailed_errors.create(
                self.vm.heap.allocator,
                val,
                self.vm.heap.unreachable_color,
            ) catch |err| switch (err) {
                error.OutOfMemory => return self.addError(DetailedError{ .out_of_memory = {} }),
            };
            return self.init(handle);
        },
        else => @compileError("Unsupported type for Builder.init: " ++ @typeName(T)),
    }
}

/// Creates a Spore string value from an owned byte slice.
///
/// This function creates a String object that takes ownership of the provided
/// byte slice without copying it. The slice must remain valid for the lifetime
/// of the resulting string value.
///
/// Args:
///     s: The byte slice to take ownership of
///
/// Returns:
///     A Spore Val containing the string
pub fn stringOwned(self: Builder, s: []const u8) errors.Error!Val {
    const handle = try self.vm.heap.strings.create(
        self.vm.heap.allocator,
        String.initOwned(s),
        self.vm.heap.unreachable_color,
    );
    return Val.init(handle);
}

/// Interns the given Symbol object.
///
/// This function takes a Symbol object and interns its string representation
/// using the VM's string interner, ensuring that only one copy of each unique
/// symbol string exists in memory.
///
/// Args:
///     sym: The Symbol object to intern.
///
/// Returns:
///     An Interned symbol.
pub fn internSymbol(self: Builder, sym: Symbol) errors.Error!Symbol.Interned {
    return sym.intern(self.vm.heap.allocator, &self.vm.heap.string_interner);
}

/// Records an error in the virtual machine's execution context.
///
/// This function stores the provided detailed error in the VM's execution
/// context and returns the corresponding Error enum value for propagation.
///
/// Args:
///     err: The detailed error information to record
///
/// Returns:
///     The Error enum value corresponding to the detailed error
pub fn addError(self: Builder, err: errors.DetailedError) errors.Error {
    self.vm.execution_context.last_error = err;
    return err.err();
}

test "Builder.init converts nil" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const result = try vm.builder().init({});
    try testing.expectFmt(
        "nil",
        "{}",
        .{vm.inspector().pretty(result)},
    );
}

test "Builder.init converts boolean" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    try testing.expectFmt(
        "true",
        "{}",
        .{vm.inspector().pretty(try vm.builder().init(true))},
    );
    try testing.expectFmt(
        "false",
        "{}",
        .{vm.inspector().pretty(try vm.builder().init(false))},
    );
}

test "Builder.init converts integer" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    try testing.expectFmt(
        "42",
        "{}",
        .{vm.inspector().pretty(try vm.builder().init(42))},
    );
}

test "Builder.init converts float" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    try testing.expectFmt(
        "3.14",
        "{}",
        .{vm.inspector().pretty(try vm.builder().init(@as(f64, 3.14)))},
    );
}

test "Builder.init converts interned symbol handle" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    try testing.expectFmt(
        "test-symbol",
        "{}",
        .{vm.inspector().pretty(try vm.builder().init(try vm.builder().internSymbol(Symbol.init("test-symbol"))))},
    );
}

test "Builder.init converts pair handle" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const pair = Pair.init(Val.init(1), Val.init(2));
    const pair_handle = try vm.heap.pairs.create(
        vm.heap.allocator,
        pair,
        vm.heap.unreachable_color,
    );
    const result = try vm.builder().init(pair_handle);

    try testing.expectFmt("(1 . 2)", "{}", .{vm.inspector().pretty(result)});
}

test "Builder.init converts string handle" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const string = try String.initCopy(vm.heap.allocator, "hello");
    const string_handle = try vm.heap.strings.create(
        vm.heap.allocator,
        string,
        vm.heap.unreachable_color,
    );
    const result = try vm.builder().init(string_handle);

    try testing.expectFmt(
        "\"hello\"",
        "{}",
        .{vm.inspector().pretty(result)},
    );
}

test "Builder.init converts native function pointer" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    // Helper function for testing
    const dummyFunction = struct {
        fn call(vm_ptr: *Vm) errors.Error!Val {
            _ = vm_ptr;
            return Val.init(42);
        }
    }.call;

    const native_func = &NativeFunction{
        .name = "test-func",
        .docstring = "Test function",
        .ptr = dummyFunction,
    };
    const result = try vm.builder().init(native_func);

    try testing.expect(result.repr == .native_function);
    try testing.expect(result.repr.native_function == native_func);
}

test "Builder.init converts bytecode function handle" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const func_handle = try vm.heap.bytecode_functions.create(
        vm.heap.allocator,
        BytecodeFunction{
            .instructions = &.{},
        },
        vm.heap.unreachable_color,
    );
    const result = try vm.builder().init(func_handle);

    try testing.expect(result.repr == .bytecode_function);
    try testing.expect(result.repr.bytecode_function.id == func_handle.id);
}

test "Builder.init converts byte slice to heap string" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    try testing.expectFmt(
        "\"hello world\"",
        "{}",
        .{vm.inspector().pretty(try vm.builder().init(@as([]const u8, "hello world")))},
    );
}

test "Builder.init converts const byte slice to heap string" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const buffer = "hello";
    try testing.expectFmt(
        "\"hello\"",
        "{}",
        .{vm.inspector().pretty(try vm.builder().init(@as([]const u8, buffer)))},
    );
}

test "Builder.init converts Val slice to linked list" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const vals = [_]Val{ Val.init(1), Val.init(2), Val.init(3) };
    try testing.expectFmt(
        "(1 2 3)",
        "{}",
        .{vm.inspector().pretty(try vm.builder().init(@as([]const Val, &vals)))},
    );
}

test "Builder.init converts const Val slice to linked list" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const vals = [_]Val{ Val.init(1), Val.init(2), Val.init(3) };
    try testing.expectFmt(
        "(1 2 3)",
        "{}",
        .{vm.inspector().pretty(try vm.builder().init(@as([]const Val, &vals)))},
    );
}

test "Builder.init converts Symbol to interned symbol" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    try testing.expectFmt(
        "my-symbol",
        "{}",
        .{vm.inspector().pretty(try vm.builder().init(Symbol.init("my-symbol")))},
    );
}

test "Builder.init converts Pair to heap-allocated pair" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    try testing.expectFmt(
        "(10 . 20)",
        "{}",
        .{vm.inspector().pretty(try vm.builder().init(Pair.init(Val.init(10), Val.init(20))))},
    );
}

test "Builder.init handles out of memory when converting Val slice" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    // Set up a failing allocator to trigger OOM
    var failing_allocator = testing.FailingAllocator.init(testing.allocator, .{ .fail_index = 0 });
    vm.heap.allocator = failing_allocator.allocator();

    const vals = [_]Val{ Val.init(1), Val.init(2) };
    try testing.expectError(
        errors.Error.OutOfMemory,
        vm.builder().init(@as([]const Val, &vals)),
    );
}

test "Builder.init handles out of memory when converting Pair" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    // Set up a failing allocator to trigger OOM
    var failing_allocator = testing.FailingAllocator.init(testing.allocator, .{ .fail_index = 0 });
    vm.heap.allocator = failing_allocator.allocator();

    try testing.expectError(
        errors.Error.OutOfMemory,
        vm.builder().init(Pair.init(Val.init(1), Val.init(2))),
    );
}

test "Builder.stringOwned creates string taking ownership of slice" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const owned_slice = try vm.heap.allocator.dupe(u8, "owned string");

    try testing.expectFmt("\"owned string\"", "{}", .{vm.inspector().pretty(try vm.builder().stringOwned(owned_slice))});
}

test "Builder.stringOwned handles out of memory" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    var failing_allocator = testing.FailingAllocator.init(testing.allocator, .{ .fail_index = 0 });
    vm.heap.allocator = failing_allocator.allocator();
    try testing.expectError(
        errors.Error.OutOfMemory,
        vm.builder().stringOwned("test"),
    );
}

test "Builder.internSymbol interns a new symbol" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const retrieved = try (try vm.builder().internSymbol(Symbol.init("new-symbol"))).get(vm.heap.string_interner);
    try testing.expectEqualStrings(
        "new-symbol",
        retrieved.symbol,
    );
}

test "Builder.internSymbol returns existing interned symbol" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const symbol = Symbol.init("existing-symbol");

    const first_result = try vm.builder().internSymbol(symbol);
    const second_result = try vm.builder().internSymbol(symbol);

    try testing.expect(
        first_result.symbol.id == second_result.symbol.id,
    );
}

test "Builder.addError records detailed error in VM context" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const detailed_error = DetailedError{ .divide_by_zero = {} };
    try testing.expectEqual(
        errors.Error.DivisionByZero,
        vm.builder().addError(detailed_error),
    );
    try testing.expect(
        std.meta.eql(vm.execution_context.last_error, detailed_error),
    );
}

test "Builder.addError returns corresponding error enum" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    try testing.expectEqual(
        errors.Error.OutOfMemory,
        vm.builder().addError(DetailedError{ .out_of_memory = {} }),
    );
    try testing.expectEqual(
        errors.Error.StackOverflow,
        vm.builder().addError(DetailedError{ .stack_overflow = {} }),
    );
    try testing.expectEqual(
        errors.Error.StackUnderflow,
        vm.builder().addError(DetailedError{ .stack_underflow = {} }),
    );
    try testing.expectEqual(
        errors.Error.Internal,
        vm.builder().addError(DetailedError{ .internal = {} }),
    );
}

test "Builder.init converts DetailedError handle" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const detailed_error = DetailedError{ .wrong_type = .{ .want = "number", .got = Val.init(42) } };
    const handle = try vm.heap.detailed_errors.create(
        vm.heap.allocator,
        detailed_error,
        vm.heap.unreachable_color,
    );
    const result = try vm.builder().init(handle);

    try testing.expect(result.repr == .detailed_error);
    try testing.expectEqual(handle, result.repr.detailed_error);
}

test "Builder.init converts DetailedError to heap-allocated error" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const detailed_error = DetailedError{ .symbol_not_found = .{ .symbol = try vm.builder().internSymbol(Symbol.init("undefined")) } };
    const result = try vm.builder().init(detailed_error);

    try testing.expect(result.repr == .detailed_error);
    const retrieved_error = try vm.heap.detailed_errors.get(result.repr.detailed_error);
    try testing.expect(std.meta.eql(retrieved_error, detailed_error));
}

test "Builder.init handles out of memory when converting DetailedError" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    var failing_allocator = testing.FailingAllocator.init(testing.allocator, .{ .fail_index = 0 });
    vm.heap.allocator = failing_allocator.allocator();

    try testing.expectError(
        errors.Error.OutOfMemory,
        vm.builder().init(DetailedError{ .internal = {} }),
    );
}
