//! Defines a symbol datastructure.
//!
//! Symbols are similar to strings, but generally immutable and used to hold
//! information like function and variable names.
const std = @import("std");
const testing = std.testing;

const StringInterner = @import("StringInterner.zig");

const Symbol = @This();

/// The string representation of the symbol.
///
/// Usually a reference and not owned.
symbol: []const u8,

/// An interned symbol. Interned symbols are faster to compare and take less
/// memory to store.
pub const Interned = struct {
    /// The handle to the interned string for this symbol.
    symbol: StringInterner.Interned,

    /// Get the interned value as a `Symbol`.
    pub fn get(self: Interned, string_interner: StringInterner) !Symbol {
        const symbol_str = string_interner.toString(self.symbol) orelse return error.SymbolNotFound;
        return .{ .symbol = symbol_str };
    }

    /// Formats self implementing the `std.fmt.Format` interface.
    pub fn format(self: Interned, comptime fmt: []const u8, options: std.fmt.FormatOptions, writer: anytype) !void {
        _ = fmt;
        _ = options;
        try writer.print("@symbol-{}", .{self.symbol.id});
    }
};

/// Initialize a new symbol from `s`.
pub fn init(s: []const u8) Symbol {
    return .{ .symbol = s };
}

/// Intern the symbol using the given `string_interner`.
pub fn intern(self: Symbol, allocator: std.mem.Allocator, string_interner: *StringInterner) !Interned {
    const interned_string = try string_interner.intern(allocator, self.symbol);
    return .{ .symbol = interned_string };
}

/// Formats self implementing the `std.fmt.Format` interface.
pub fn format(self: Symbol, comptime fmt: []const u8, options: std.fmt.FormatOptions, writer: anytype) !void {
    _ = fmt;
    _ = options;
    try writer.print("{s}", .{self.symbol});
}

test "format Symbol" {
    const s = Symbol.init("my-symbol");
    try testing.expectFmt("my-symbol", "{any}", .{s});

    const s_quoted = Symbol.init("'my-symbol");
    try testing.expectFmt("'my-symbol", "{any}", .{s_quoted});
}
