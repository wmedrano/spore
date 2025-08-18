const std = @import("std");

const spore = @import("spore_lib");
const Readline = @import("Readline.zig");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    var vm = try spore.Vm.init(gpa.allocator());
    defer vm.deinit();

    var readline = Readline.init(gpa.allocator());
    defer readline.deinit();

    const stdout = std.io.getStdOut().writer();
    try stdout.print("Spore REPL - Enter expressions to evaluate ((help) for commands)\n", .{});
    while (try readline.readLine("spore> ")) |input| {
        defer gpa.allocator().free(input);
        const trimmed_input = std.mem.trim(u8, input, " \t\r\n");
        if (trimmed_input.len == 0) continue;
        if (std.mem.eql(u8, trimmed_input, "exit") or std.mem.eql(u8, trimmed_input, "quit")) {
            try stdout.print("Goodbye!\n", .{});
            break;
        }
        const result = vm.evalStr(trimmed_input) catch {
            try stdout.print("Error: {}\n", .{vm.inspector().errorReport()});
            continue;
        };
        switch (result.repr) {
            .nil => {},
            else => try stdout.print("{}\n", .{result}),
        }
    }
}
