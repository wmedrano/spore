const std = @import("std");

const spore = @import("spore_lib");
const Readline = @import("terminal/Readline.zig");
const Color = @import("terminal/Color.zig");

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
        const trimmed_input = std.mem.trim(u8, input, " \t\r\n");
        if (trimmed_input.len == 0) continue;
        if (std.mem.eql(u8, trimmed_input, "exit") or std.mem.eql(u8, trimmed_input, "quit")) {
            try Readline.printInfo(stdout, "Goodbye!\n");
            break;
        }
        const result = vm.evalStr(trimmed_input) catch {
            try Readline.printError(stdout, "Error: ");
            try stdout.print("{}\n", .{vm.inspector().errorReport()});
            continue;
        };
        switch (result.repr) {
            .nil => try Readline.printSpecial(stdout, "nil\n"),
            else => {
                try Readline.printSuccess(stdout, "=> ");
                try stdout.print("{}\n", .{result});
            },
        }
    }
}
