//! The `Inspector` provides functions for pretty printing `Val`s.
const std = @import("std");
const testing = std.testing;

const BytecodeFunction = @import("BytecodeFunction.zig");
const DetailedError = @import("errors.zig").DetailedError;
const Handle = @import("object_pool.zig").Handle;
const NativeFunction = @import("NativeFunction.zig");
const Pair = @import("Pair.zig");
const PrettyPrinter = @import("PrettyPrinter.zig");
const String = @import("String.zig");
const Symbol = @import("Symbol.zig");
const Val = @import("Val.zig");
const Vm = @import("Vm.zig");

const Inspector = @This();

vm: *const Vm,

/// Pretty prints a single `Val`.
pub fn pretty(self: Inspector, val: Val) PrettyPrinter {
    return PrettyPrinter{
        .vm = self.vm,
        .val = val,
    };
}

/// Pretty prints a slice of `Val`s.
pub fn prettySlice(self: Inspector, vals: []const Val) PrettyPrinter.Slice {
    return PrettyPrinter.Slice{
        .vm = self.vm,
        .vals = vals,
    };
}

/// Pretty prints the stack trace.
pub fn stackTrace(self: Inspector) ?PrettyPrinter.StackTrace {
    return PrettyPrinter.StackTrace{ .vm = self.vm };
}

/// A formatter that combines stack trace and error information for comprehensive error reporting.
pub const ErrorReport = struct {
    vm: *const Vm,

    pub fn format(self: ErrorReport, comptime fmt: []const u8, options: std.fmt.FormatOptions, writer: anytype) !void {
        _ = fmt;
        _ = options;
        try writer.print("Error encountered!\n", .{});
        const last_error = if (self.vm.execution_context.last_error) |err|
            PrettyPrinter.Err{ .vm = self.vm, .err = err }
        else
            null;
        try writer.print("{any}\nError:\n  {any}\n", .{
            PrettyPrinter.StackTrace{ .vm = self.vm },
            last_error,
        });
    }
};

/// Returns an ErrorReport formatter that combines stack trace and error information.
pub fn errorReport(self: Inspector) ErrorReport {
    return ErrorReport{ .vm = self.vm };
}

/// An error that occurs when converting a Val to a Zig type through Inspector.
pub const ToError = error{ WrongType, ObjectNotFound };

/// Convert `Val` into a value of type `T`.
///
/// Similar to `Val.to` but supports additional types that require VM access for resolution.
/// Supported types include all `Val.to` types plus: `[]const u8`, `Symbol`, `Pair`, `DetailedError`, `Pair.ListIter`.
/// Does not support `[]Val` or `[]const Val`.
pub fn to(self: Inspector, T: type, val: Val) ToError!T {
    switch (T) {
        // Types supported by Val.to - delegate directly
        void, bool, i64, f64 => return val.to(T) catch ToError.WrongType,
        Symbol.Interned => return val.to(T) catch ToError.WrongType,
        Handle(Pair) => return val.to(T) catch ToError.WrongType,
        Handle(String) => return val.to(T) catch ToError.WrongType,
        // Note: *const NativeFunction is not supported by Val.to, handle directly
        *const NativeFunction => switch (val.repr) {
            .native_function => |x| return x,
            else => return ToError.WrongType,
        },
        NativeFunction => switch (val.repr) {
            .native_function => |x| return x.*,
            else => return ToError.WrongType,
        },
        Handle(BytecodeFunction) => return val.to(T) catch ToError.WrongType,
        Handle(DetailedError) => return val.to(T) catch ToError.WrongType,
        // Additional types requiring VM access
        []const u8 => switch (val.repr) {
            .string => |handle| {
                const string = try self.vm.heap.strings.get(handle);
                return string.data;
            },
            else => return ToError.WrongType,
        },
        Symbol => switch (val.repr) {
            .symbol => |interned| return try interned.get(self.vm.heap.string_interner),
            else => return ToError.WrongType,
        },
        Pair => switch (val.repr) {
            .pair => |handle| return try self.vm.heap.pairs.get(handle),
            else => return ToError.WrongType,
        },
        DetailedError => switch (val.repr) {
            .detailed_error => |handle| return try self.vm.heap.detailed_errors.get(handle),
            else => return ToError.WrongType,
        },
        Pair.ListIter => switch (val.repr) {
            .nil => return Pair.iterEmpty(),
            .pair => |handle| {
                const pair = try self.vm.heap.pairs.get(handle);
                return pair.iterList();
            },
            else => return ToError.WrongType,
        },
        else => @compileError("Unsupported type for Inspector.to: " ++ @typeName(T)),
    }
}

test "Inspector.to void" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const nil_val = Val.init({});
    _ = try vm.inspector().to(void, nil_val);
    try testing.expectError(
        ToError.WrongType,
        vm.inspector().to(void, Val.init(42)),
    );
}

test "Inspector.to bool" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const bool_val = Val.init(true);
    try testing.expectEqual(true, try vm.inspector().to(bool, bool_val));
    try testing.expectError(
        ToError.WrongType,
        vm.inspector().to(bool, Val.init(42)),
    );
}

test "Inspector.to i64" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const int_val = Val.init(42);
    try testing.expectEqual(42, try vm.inspector().to(i64, int_val));
    try testing.expectError(
        ToError.WrongType,
        vm.inspector().to(i64, Val.init(true)),
    );
}

test "Inspector.to f64" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const float_val = Val.init(3.14);
    try testing.expectEqual(3.14, try vm.inspector().to(f64, float_val));
    try testing.expectError(
        ToError.WrongType,
        vm.inspector().to(f64, Val.init(42)),
    );
}

test "Inspector.to Symbol.Interned" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const symbol = try vm.initVal(Symbol.init("hello"));
    const interned = try vm.inspector().to(Symbol.Interned, symbol);
    try testing.expectEqual(symbol.repr.symbol, interned);
    try testing.expectError(
        ToError.WrongType,
        vm.inspector().to(Symbol.Interned, Val.init(42)),
    );
}

test "Inspector.to Handle(Pair)" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const pair_val = try vm.initVal(Pair.init(Val.init(1), Val.init(2)));
    const handle = try vm.inspector().to(Handle(Pair), pair_val);
    try testing.expectEqual(pair_val.repr.pair, handle);
    try testing.expectError(
        ToError.WrongType,
        vm.inspector().to(Handle(Pair), Val.init(42)),
    );
}

test "Inspector.to Handle(String)" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const string_val = try vm.builder().init(@as([]const u8, "test"));
    const handle = try vm.inspector().to(Handle(String), string_val);
    try testing.expectEqual(string_val.repr.string, handle);
    try testing.expectError(
        ToError.WrongType,
        vm.inspector().to(Handle(String), Val.init(42)),
    );
}

test "Inspector.to *const NativeFunction" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const errors = @import("errors.zig");
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
    const func_val = Val.init(native_func);
    const result = try vm.inspector().to(*const NativeFunction, func_val);
    try testing.expectEqual(native_func, result);
    try testing.expectError(
        ToError.WrongType,
        vm.inspector().to(*const NativeFunction, Val.init(42)),
    );
}

test "Inspector.to Handle(BytecodeFunction)" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const func_handle = try vm.heap.bytecode_functions.create(
        vm.heap.allocator,
        BytecodeFunction{
            .instructions = &.{},
        },
        vm.heap.unreachable_color,
    );
    const func_val = Val.init(func_handle);
    const result = try vm.inspector().to(Handle(BytecodeFunction), func_val);
    try testing.expectEqual(func_handle, result);
    try testing.expectError(
        ToError.WrongType,
        vm.inspector().to(Handle(BytecodeFunction), Val.init(42)),
    );
}

test "Inspector.to Handle(DetailedError)" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const detailed_error = DetailedError{ .divide_by_zero = {} };
    const handle = try vm.heap.detailed_errors.create(
        vm.heap.allocator,
        detailed_error,
        vm.heap.unreachable_color,
    );
    const error_val = Val.init(handle);
    const result = try vm.inspector().to(Handle(DetailedError), error_val);
    try testing.expectEqual(handle, result);
    try testing.expectError(
        ToError.WrongType,
        vm.inspector().to(Handle(DetailedError), Val.init(42)),
    );
}

test "Inspector.to []const u8 from string" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const string_val = try vm.builder().init(@as([]const u8, "hello world"));
    const bytes = try vm.inspector().to([]const u8, string_val);
    try testing.expectEqualStrings("hello world", bytes);
}

test "Inspector.to []const u8 wrong type error" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    try testing.expectError(
        ToError.WrongType,
        vm.inspector().to([]const u8, Val.init(42)),
    );
    try testing.expectError(
        ToError.WrongType,
        vm.inspector().to([]const u8, Val.init({})),
    );
}

test "Inspector.to Symbol from interned symbol" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const symbol_val = try vm.initVal(Symbol.init("test-symbol"));
    const symbol = try vm.inspector().to(Symbol, symbol_val);
    try testing.expectEqualStrings("test-symbol", symbol.symbol);
}

test "Inspector.to Symbol wrong type error" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    try testing.expectError(
        ToError.WrongType,
        vm.inspector().to(Symbol, Val.init(42)),
    );
    try testing.expectError(
        ToError.WrongType,
        vm.inspector().to(Symbol, Val.init({})),
    );
}

test "Inspector.to Pair from pair handle" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const pair_val = try vm.initVal(
        Pair.init(Val.init(10), Val.init(20)),
    );
    const pair = try vm.inspector().to(Pair, pair_val);
    try testing.expectEqual(10, try pair.first.to(i64));
    try testing.expectEqual(20, try pair.second.to(i64));
}

test "Inspector.to Pair wrong type error" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    try testing.expectError(
        ToError.WrongType,
        vm.inspector().to(Pair, Val.init(42)),
    );
    try testing.expectError(
        ToError.WrongType,
        vm.inspector().to(Pair, Val.init({})),
    );
}

test "Inspector.to DetailedError from error handle" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const original_error = DetailedError{ .stack_overflow = {} };
    const error_val = try vm.builder().init(original_error);
    const detailed_error = try vm.inspector().to(DetailedError, error_val);
    try testing.expect(std.meta.eql(original_error, detailed_error));
}

test "Inspector.to DetailedError wrong type error" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    try testing.expectError(
        ToError.WrongType,
        vm.inspector().to(DetailedError, Val.init(42)),
    );
    try testing.expectError(
        ToError.WrongType,
        vm.inspector().to(DetailedError, Val.init({})),
    );
}

test "Inspector.to Pair.ListIter from nil creates empty iterator" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const nil_val = Val.init({});
    const list_iter = try vm.inspector().to(Pair.ListIter, nil_val);
    try testing.expect(list_iter.empty());
}

test "Inspector.to Pair.ListIter from pair creates list iterator" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const pair_val = try vm.initVal(Pair.init(Val.init(1), Val.init({})));
    const list_iter = try vm.inspector().to(Pair.ListIter, pair_val);
    try testing.expect(!list_iter.empty());
}

test "Inspector.to Pair.ListIter wrong type returns error" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    try testing.expectError(
        ToError.WrongType,
        vm.inspector().to(Pair.ListIter, Val.init(42)),
    );
    try testing.expectError(
        ToError.WrongType,
        vm.inspector().to(Pair.ListIter, Val.init(true)),
    );
}
