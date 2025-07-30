//! Represents a dynamically allocated, immutable UTF-8 string.
const std = @import("std");
const testing = std.testing;

const String = @This();

/// The underlying byte slice storing the string data.
data: []const u8,

/// Initializes a new String by allocating and copying the provided byte slice.
/// The returned String owns this memory.
pub fn initCopy(allocator: std.mem.Allocator, s: []const u8) !String {
    return String{ .data = try allocator.dupe(u8, s) };
}

/// Initializes a new String from an existing slice.
/// This does not allocate memory.
pub fn initOwned(s: []const u8) String {
    return String{ .data = s };
}

/// Deinitializes the String, freeing its allocated memory.
/// This should be called when the String is no longer needed.
pub fn deinit(self: *String, allocator: std.mem.Allocator) void {
    allocator.free(self.data);
    self.data = &.{};
}

/// Returns a const slice of the underlying string data.
/// This does not transfer ownership.
pub fn slice(self: String) []const u8 {
    return self.data;
}

/// Formats self implementing the `std.fmt.Format` interface.
pub fn format(self: String, comptime fmt: []const u8, _: std.fmt.FormatOptions, writer: anytype) !void {
    try writer.print("{" ++ fmt ++ "}", .{self.slice()});
}
