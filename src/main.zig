const std = @import("std");

const spore = @import("spore_lib");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    const args = try std.process.argsAlloc(gpa.allocator());
    defer std.process.argsFree(gpa.allocator(), args);

    const filename = switch (args.len) {
        0 => unreachable(),
        2 => args[1],
        else => {
            std.debug.print("Bad arguments.\nUsage: {s} <filename>\n", .{args[0]});
            return;
        },
    };

    const program_str = try loadProgram(gpa.allocator(), filename);
    defer gpa.allocator().free(program_str);

    var vm = try spore.Vm.init(gpa.allocator());
    defer vm.deinit();
    _ = vm.evalStr(program_str) catch |err| {
        std.debug.print("Error encountered!\n", .{});
        std.debug.print("  {any}\n", .{vm.inspector().lastError()});
        return err;
    };
}

fn loadProgram(allocator: std.mem.Allocator, filename: []const u8) ![]u8 {
    const file = try std.fs.cwd().openFile(filename, .{});
    defer file.close();
    return try file.readToEndAlloc(allocator, std.math.maxInt(usize));
}
