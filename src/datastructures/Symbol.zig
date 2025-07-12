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
/// True if the symbol is quoted.
quoted: bool,

/// An interned symbol. Interned symbols are faster to compare and take less
/// memory to store.
pub const Interned = struct {
    /// The handle to the interned string for this symbol.
    symbol: StringInterner.Interned,
    /// True if the symbol is quoted.
    quoted: bool,

    /// Get the interned value as a `Symbol`.
    pub fn get(self: Interned, string_interner: StringInterner) !Symbol {
        const symbol_str = string_interner.toString(self.symbol) orelse return error.SymbolNotFound;
        return .{ .symbol = symbol_str, .quoted = self.quoted };
    }

    /// Returns an unquoted version of the symbol if it was quoted, otherwise
    /// returns null.
    pub fn unquote(self: Interned) ?Interned {
        if (!self.quoted) return null;
        return .{ .symbol = self.symbol, .quoted = false };
    }

    /// Formats self implementing the `std.fmt.Format` interface.
    pub fn format(self: Interned, comptime fmt: []const u8, options: std.fmt.FormatOptions, writer: anytype) !void {
        _ = fmt;
        _ = options;
        if (self.quoted)
            try writer.print("@quotesymbol-{}", .{self.symbol.id})
        else
            try writer.print("@symbol-{}", .{self.symbol.id});
    }
};

/// Initialize a new symbol from `s`.
pub fn init(s: []const u8) Symbol {
    if (s.len > 0 and s[0] == '\'') {
        return .{
            .symbol = s[1..],
            .quoted = true,
        };
    }
    return .{ .symbol = s, .quoted = false };
}

/// Intern the symbol using the given `string_interner`.
pub fn intern(self: Symbol, allocator: std.mem.Allocator, string_interner: *StringInterner) !Interned {
    const interned_string = try string_interner.intern(allocator, self.symbol);
    return .{ .symbol = interned_string, .quoted = self.quoted };
}

/// Formats self implementing the `std.fmt.Format` interface.
pub fn format(self: Symbol, comptime fmt: []const u8, options: std.fmt.FormatOptions, writer: anytype) !void {
    _ = fmt;
    _ = options;
    if (self.quoted)
        try writer.print("'{s}", .{self.symbol})
    else
        try writer.print("{s}", .{self.symbol});
}

test "format Symbol unquoted" {
    const s = Symbol.init("my-symbol");
    try testing.expectFmt("my-symbol", "{any}", .{s});
}

test "format Symbol quoted" {
    const s_quoted = Symbol.init("'my-symbol");
    try testing.expectFmt("'my-symbol", "{any}", .{s_quoted});
}

test "format Interned unquoted" {
    var string_interner = StringInterner.init(testing.allocator);
    defer string_interner.deinit(testing.allocator);

    const s = try Symbol.init("my-interned-symbol").intern(std.testing.allocator, &string_interner);
    try testing.expectFmt(
        "@symbol-0",
        "{any}",
        .{s},
    );
}

test "format Interned quoted" {
    var string_interner = StringInterner.init(testing.allocator);
    defer string_interner.deinit(testing.allocator);

    const s_quoted = try Symbol.init("'my-interned-symbol").intern(std.testing.allocator, &string_interner);
    try testing.expectFmt(
        "@quotesymbol-0",
        "{any}",
        .{s_quoted},
    );
}
