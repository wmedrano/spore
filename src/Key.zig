//! Defines a key datastructure.
//!
//! Keys are similar to symbols but used for different purposes.
const std = @import("std");
const testing = std.testing;

const StringInterner = @import("StringInterner.zig");

const Key = @This();

/// The string representation of the key.
///
/// Usually a reference and not owned.
key: []const u8,

/// An interned key. Interned keys are faster to compare and take less
/// memory to store.
pub const Interned = struct {
    /// The handle to the interned string for this key.
    key: StringInterner.Interned,

    /// Get the interned value as a `Key`.
    pub fn get(self: Interned, string_interner: StringInterner) !Key {
        const key_str = string_interner.toString(self.key) orelse return error.KeyNotFound;
        return .{ .key = key_str };
    }

    /// Formats self implementing the `std.fmt.Format` interface.
    pub fn format(self: Interned, comptime fmt: []const u8, options: std.fmt.FormatOptions, writer: anytype) !void {
        _ = fmt;
        _ = options;
        try writer.print(":key-{}", .{self.key.id});
    }
};

/// Initialize a new key from `s`.
pub fn init(s: []const u8) Key {
    return .{ .key = s };
}

/// Intern the key using the given `string_interner`.
pub fn intern(self: Key, allocator: std.mem.Allocator, string_interner: *StringInterner) std.mem.Allocator.Error!Interned {
    const interned_string = try string_interner.intern(allocator, self.key);
    return .{ .key = interned_string };
}

/// Formats self implementing the `std.fmt.Format` interface.
pub fn format(self: Key, comptime fmt: []const u8, options: std.fmt.FormatOptions, writer: anytype) !void {
    _ = fmt;
    _ = options;
    try writer.print(":{s}", .{self.key});
}

test "format Key" {
    const s = Key.init("my-key");
    try testing.expectFmt(":my-key", "{any}", .{s});
}

test "format Interned" {
    var string_interner = StringInterner.init(testing.allocator);
    defer string_interner.deinit(testing.allocator);

    const s = try Key.init("my-interned-key").intern(std.testing.allocator, &string_interner);
    try testing.expectFmt(
        "@key-0",
        "{any}",
        .{s},
    );
}
