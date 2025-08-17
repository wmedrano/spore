//! A struct for pretty-printing `Val` instances.
const std = @import("std");
const testing = std.testing;

const errors = @import("errors.zig");
const DetailedError = errors.DetailedError;
const ExecutionContext = @import("ExecutionContext.zig");
const Handle = @import("object_pool.zig").Handle;
const Pair = @import("Pair.zig");
const Symbol = @import("Symbol.zig");
const Val = @import("Val.zig");
const Vm = @import("Vm.zig");

const PrettyPrinter = @This();

/// A reference to the VM, needed for resolving symbols and pairs.
vm: *const Vm,
/// The value to be printed.
val: Val,

/// A struct for pretty-printing multiple `Val`.
pub const Slice = struct {
    vm: *const Vm,
    vals: []const Val,

    pub fn format(self: Slice, comptime fmt: []const u8, options: std.fmt.FormatOptions, writer: anytype) !void {
        _ = fmt;
        _ = options;
        for (self.vals, 0..self.vals.len) |v, idx| {
            if (idx == 0) {
                try writer.print("{}", .{PrettyPrinter{ .vm = self.vm, .val = v }});
            } else {
                try writer.print(" {}", .{PrettyPrinter{ .vm = self.vm, .val = v }});
            }
        }
    }
};

pub const Err = struct {
    vm: *const Vm,
    err: DetailedError,

    pub fn format(self: Err, comptime fmt: []const u8, options: std.fmt.FormatOptions, writer: anytype) !void {
        _ = fmt;
        _ = options;
        switch (self.err) {
            .out_of_memory => try writer.print("Out of memory", .{}),
            .wrong_arity => |e| try writer.print("Wrong arity for function '{s}': want {any} got {any}", .{ e.function, e.want, e.got }),
            .symbol_not_found => |e| {
                const symbol = e.symbol.get(self.vm.heap.string_interner) catch return writer.print("Symbol not found: {any}", .{e.symbol});
                try writer.print("Symbol not found: {}", .{symbol});
            },
            .object_not_found => |e| try writer.print("Object not found: {any}", .{e.object}),
            .io_error => try writer.print("IO Error", .{}),
            .wrong_type => |e| try writer.print("Wrong type: want {s} got {any}", .{ e.want, PrettyPrinter{ .vm = self.vm, .val = e.got } }),
            .divide_by_zero => try writer.print("Division by zero", .{}),
            .stack_overflow => try writer.print("Stack overflow", .{}),
            .stack_underflow => try writer.print("Stack underflow", .{}),
            .internal => try writer.print("Internal", .{}),
        }
    }
};

pub const StackTrace = struct {
    vm: *const Vm,

    pub fn format(self: StackTrace, comptime fmt: []const u8, options: std.fmt.FormatOptions, writer: anytype) !void {
        _ = fmt;
        _ = options;
        try writer.print("Stack Trace:\n", .{});
        for (self.vm.execution_context.previous_call_frames.constSlice()) |call_frame| {
            try formatCallFrame(call_frame, self.vm, writer);
        }
        try formatCallFrame(self.vm.execution_context.call_frame, self.vm, writer);
    }

    pub fn formatCallFrame(frame: ExecutionContext.CallFrame, vm: *const Vm, writer: anytype) !void {
        const function_idx = frame.stack_start - 1;
        const maybe_function = if (function_idx < 0)
            null
        else
            vm.execution_context.stack.get(@intCast(function_idx));
        const function = maybe_function orelse Val.init({});
        try writer.print("  - {any}\n", .{vm.inspector().pretty(function)});
    }
};

/// Formats the `Val` for pretty-printing.
pub fn format(
    self: PrettyPrinter,
    comptime fmt: []const u8,
    options: std.fmt.FormatOptions,
    writer: anytype,
) !void {
    _ = fmt;
    _ = options;
    switch (self.val.repr) {
        .nil => try writer.print("nil", .{}),
        .boolean => |x| try writer.print("{any}", .{x}),
        .int => |x| try writer.print("{}", .{x}),
        .float => |x| try writer.print("{d}", .{x}),
        .symbol => |x| {
            const symbol = x.get(self.vm.heap.string_interner) catch return writer.print("@bad-symbol", .{});
            try writer.print("{}", .{symbol});
        },
        .pair => |handle| {
            const pair = self.vm.heap.pairs.get(handle) catch return writer.print("@bad-pair", .{});
            try formatPair(pair, self.vm, writer);
        },
        .string => |handle| {
            const string = self.vm.heap.strings.get(handle) catch return writer.print("@bad-string", .{});
            try writer.print("\"{s}\"", .{string});
        },
        .native_function => |func| try writer.print("{any}", .{func}),
        .bytecode_function => |handle| {
            const func = self.vm.heap.bytecode_functions.get(handle) catch return writer.print("@bad-function", .{});
            if (func.name) |name| {
                try writer.print("@function-{any}", .{self.vm.inspector().pretty(Val.init(name))});
            } else {
                try writer.print("@function", .{});
            }
        },
        .detailed_error => |handle| {
            const detailed_error = self.vm.heap.detailed_errors.get(handle) catch return writer.print("@bad-error", .{});
            try writer.print("@error-{s}", .{@tagName(detailed_error)});
        },
    }
}

fn formatPair(pair: Pair, vm: *const Vm, writer: anytype) !void {
    try writer.print("({}", .{PrettyPrinter{ .vm = vm, .val = pair.first }});
    try formatRest(pair.second, vm, writer);
}

fn formatRest(rest: Val, vm: *const Vm, writer: anytype) !void {
    switch (rest.repr) {
        .nil => try writer.print(")", .{}),
        .pair => |handle| {
            const next_pair = try vm.heap.pairs.get(handle);
            try writer.print(" {}", .{PrettyPrinter{ .vm = vm, .val = next_pair.first }});
            try formatRest(next_pair.second, vm, writer);
        },
        else => try writer.print(" . {})", .{PrettyPrinter{ .vm = vm, .val = rest }}),
    }
}

test format {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try testing.expectFmt(
        "nil",
        "{}",
        .{PrettyPrinter{ .vm = &vm, .val = Val.init({}) }},
    );
    try testing.expectFmt(
        "45",
        "{}",
        .{PrettyPrinter{ .vm = &vm, .val = Val.init(45) }},
    );
    try testing.expectFmt(
        "45.5",
        "{}",
        .{PrettyPrinter{ .vm = &vm, .val = Val.init(45.5) }},
    );
}

test "pretty print pair" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    const pair = try vm.initVal(Pair.init(Val.init(1), Val.init(2)));
    try testing.expectFmt(
        "(1 . 2)",
        "{}",
        .{PrettyPrinter{ .vm = &vm, .val = pair }},
    );
}

test "pretty print pair list" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    const pair = try vm.initVal(Pair.init(Val.init(1), Val.init({})));
    try testing.expectFmt(
        "(1)",
        "{}",
        .{PrettyPrinter{ .vm = &vm, .val = pair }},
    );
}

test "PrettyPrinter.Err formats wrong_arity" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    const err = errors.DetailedError{ .wrong_arity = .{ .function = "test-func", .want = 2, .got = 1 } };
    try testing.expectFmt(
        "Wrong arity for function 'test-func': want 2 got 1",
        "{}",
        .{PrettyPrinter.Err{ .vm = &vm, .err = err }},
    );
}

test "PrettyPrinter.Err formats symbol_not_found" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    const symbol_val = try vm.initVal(Symbol.init("my-symbol"));
    const err = errors.DetailedError{
        .symbol_not_found = .{ .symbol = symbol_val.repr.symbol },
    };
    try testing.expectFmt(
        "Symbol not found: my-symbol",
        "{}",
        .{PrettyPrinter.Err{ .vm = &vm, .err = err }},
    );
}

test "PrettyPrinter.Err formats object_not_found" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    const err = errors.DetailedError{ .object_not_found = .{
        .object = Val{ .repr = .{ .pair = Handle(Pair){ .id = std.math.maxInt(u32) } } },
    } };
    try testing.expectFmt(
        "Object not found: @pair-4294967295",
        "{}",
        .{PrettyPrinter.Err{ .vm = &vm, .err = err }},
    );
}

test "PrettyPrinter.Err formats io_error" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    const err = errors.DetailedError{ .io_error = {} };
    try testing.expectFmt(
        "IO Error",
        "{}",
        .{PrettyPrinter.Err{ .vm = &vm, .err = err }},
    );
}

test "PrettyPrinter.Err formats wrong_type" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    const val = Val.init(123); // Example value that caused wrong type
    const err = errors.DetailedError{ .wrong_type = .{ .want = "string", .got = val } };
    try testing.expectFmt(
        "Wrong type: want string got 123",
        "{}",
        .{PrettyPrinter.Err{ .vm = &vm, .err = err }},
    );
}

test "PrettyPrinter.Err formats divide_by_zero" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    const err = errors.DetailedError{ .divide_by_zero = {} };
    try testing.expectFmt(
        "Division by zero",
        "{}",
        .{PrettyPrinter.Err{ .vm = &vm, .err = err }},
    );
}

test "PrettyPrinter.Err formats stack_overflow" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    const err = errors.DetailedError{ .stack_overflow = {} };
    try testing.expectFmt(
        "Stack overflow",
        "{}",
        .{PrettyPrinter.Err{ .vm = &vm, .err = err }},
    );
}

test "PrettyPrinter.Err formats stack_underflow" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    const err = errors.DetailedError{ .stack_underflow = {} };
    try testing.expectFmt(
        "Stack underflow",
        "{}",
        .{PrettyPrinter.Err{ .vm = &vm, .err = err }},
    );
}

test "StackTrace format shows all call frames" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    _ = vm.evalStr(
        \\ (defun foo (a b) (number? a b))
        \\ (defun bar (a b) (foo a b))
        \\ (defun baz (a b) (bar a b))
        \\ (baz 1 2)
    ) catch {};
    try testing.expectFmt(
        \\Stack Trace:
        \\  - nil
        \\  - @function-user-source
        \\  - @function-baz
        \\  - @function-bar
        \\  - @function-foo
        \\  - @nativefunction-number?
        \\
    ,
        "{any}",
        .{PrettyPrinter.StackTrace{ .vm = &vm }},
    );
}
