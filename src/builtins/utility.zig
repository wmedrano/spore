const std = @import("std");

const errors = @import("../errors.zig");
const DetailedError = errors.DetailedError;
const NativeFunction = @import("../NativeFunction.zig");
const Pair = @import("../Pair.zig");
const Symbol = @import("../Symbol.zig");
const Val = @import("../Val.zig");
const Vm = @import("../Vm.zig");

/// Registers all utility functions with the provided Vm.
pub fn registerAll(vm: *Vm) !void {
    try internal_define.register(vm);
    try range.register(vm);
    try help.register(vm);
}

const internal_define = NativeFunction{
    .name = "internal-define",
    .docstring = "Defines a global variable. It expects two arguments: a symbol " ++
        "(the name of the variable) and a value.",
    .ptr = internalDefineImpl,
};

fn internalDefineImpl(vm: *Vm) errors.Error!Val {
    const args = vm.execution_context.localStack();
    if (args.len != 2) return vm.builder().addError(DetailedError{ .wrong_arity = .{
        .function = "internal-define",
        .want = 2,
        .got = @intCast(args.len),
    } });
    const symbol = args[0].to(Symbol.Interned) catch return vm.builder().addError(
        DetailedError{ .wrong_type = .{ .want = "symbol", .got = args[0] } },
    );
    const value = args[1];
    try vm.execution_context.setGlobal(vm.heap.allocator, symbol, value);
    return Val.init({});
}

const range = NativeFunction{
    .name = "range",
    .docstring = "Creates a pair representing a half-open integer range [start, end). Both arguments must be integers.",
    .ptr = rangeImpl,
};

fn rangeImpl(vm: *Vm) errors.Error!Val {
    const args = vm.execution_context.localStack();
    if (args.len != 2) return vm.builder().addError(DetailedError{ .wrong_arity = .{
        .function = "range",
        .want = 2,
        .got = @intCast(args.len),
    } });

    const start = args[0].to(i64) catch return vm.builder().addError(
        DetailedError{ .wrong_type = .{ .want = "int", .got = args[0] } },
    );
    const end = args[1].to(i64) catch return vm.builder().addError(
        DetailedError{ .wrong_type = .{ .want = "int", .got = args[1] } },
    );

    return vm.initVal(Pair.init(Val.init(start), Val.init(end)));
}

const help = NativeFunction{
    .name = "help",
    .docstring = "Shows help information about the Spore REPL and basic syntax.",
    .ptr = helpImpl,
};

fn helpImpl(vm: *Vm) errors.Error!Val {
    const args = vm.execution_context.localStack();
    if (args.len != 0) return vm.builder().addError(DetailedError{ .wrong_arity = .{
        .function = "help",
        .want = 0,
        .got = @intCast(args.len),
    } });

    const help_text =
        \\Spore REPL Commands:
        \\  (help)       - Show this help screen
        \\  exit, quit   - Exit the REPL
        \\  Ctrl+D       - Exit the REPL (EOF)
        \\
        \\Basic Spore syntax:
        \\  (+ 1 2)      - Addition
        \\  (print "hi") - Print a string
        \\  (def x 42)   - Define a variable
        \\
        \\Line editing:
        \\  Left/Right arrows - Move cursor
        \\  Ctrl+A - Beginning of line
        \\  Ctrl+E - End of line
        \\
        \\Enter any Spore expression to evaluate it.
        \\
    ;
    
    std.fmt.format(std.io.getStdOut().writer(), "{s}", .{help_text}) catch |err| {
        std.log.warn("Failed to write help text: {}", .{err});
        return vm.builder().addError(DetailedError{ .io_error = {} });
    };
    return Val.init({});
}