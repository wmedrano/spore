const std = @import("std");

const spore = @import("spore_lib");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};

    const program_str = try std.io.getStdIn().reader().readAllAlloc(
        gpa.allocator(),
        std.math.maxInt(usize),
    );
    defer gpa.allocator().free(program_str);

    var vm = try spore.Vm.init(gpa.allocator());
    defer vm.deinit();
    _ = vm.evalStr(program_str) catch |err| {
        std.debug.print("Error encountered!\n", .{});
        std.debug.print("{any}\n{any}\n\n\n", .{
            vm.inspector().stackTrace(),
            vm.inspector().lastError(),
        });
        return err;
    };
}
