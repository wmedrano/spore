const std = @import("std");

const spore = @import("spore_lib");

const Color = @import("terminal/Color.zig");
const Readline = @import("terminal/Readline.zig");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    var vm = try spore.Vm.init(gpa.allocator());
    defer vm.deinit();

    var readline = Readline.init(gpa.allocator());
    defer readline.deinit();

    const stdout = std.io.getStdOut().writer();
    try Readline.printInfo(stdout, "Spore REPL - Enter expressions to evaluate ((help) for commands)\n");
    while (try readline.readLineColored("spore> ", Color.Theme.default.prompt)) |input| {
        defer gpa.allocator().free(input);
        const expr_str = std.mem.trim(u8, input, " \t\r\n");
        if (expr_str.len == 0) continue;
        if (std.mem.eql(u8, expr_str, "exit") or std.mem.eql(u8, expr_str, "quit")) {
            try Readline.printInfo(stdout, "Goodbye!\n");
            break;
        }
        vm.execution_context.resetCalls();
        const result = vm.evalStr(expr_str) catch {
            try Readline.printError(stdout, "Error: ");
            try stdout.print("{}\n", .{vm.inspector().errorReport()});
            continue;
        };
        switch (result.repr) {
            .nil => {},
            else => {
                try Readline.printSuccess(stdout, "=> ");
                try stdout.print("{}\n", .{vm.inspector().pretty(result)});
            },
        }
    }
}
