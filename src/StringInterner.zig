//! Produce interned strings. Using interned strings may improve performance by
//! saving memory and making string comparisons cheaper.
const std = @import("std");
const testing = std.testing;

const StringInterner = @This();

/// An interned string. Interned strings are small and may be compared bitwise
/// for equality if they are constructed from the same `StringInterner`.
pub const Interned = struct {
    /// The unique identifier for the interned string.
    id: u32,
};

/// The arena in which all strings are allocated.
allocator: std.heap.ArenaAllocator,
/// A map from string to the interned string that uniquely identifiers it.
string_to_interned: std.StringHashMapUnmanaged(Interned) = .{},
/// An array where the index corresponds to the id and the entry to the string.
interned_to_string: std.ArrayListUnmanaged([]const u8) = .{},

/// Initializes a new `StringInterner`.
///
/// `allocator` will be used to initialize the arena for interned strings.
pub fn init(allocator: std.mem.Allocator) StringInterner {
    return .{ .allocator = std.heap.ArenaAllocator.init(allocator) };
}

/// Deinitializes the `StringInterner`, freeing all allocated resources.
///
/// This includes the internal `ArenaAllocator` and the backing memory for
/// the hash map and array.
pub fn deinit(self: *StringInterner, allocator: std.mem.Allocator) void {
    self.allocator.deinit();
    self.string_to_interned.deinit(allocator);
    self.interned_to_string.deinit(allocator);
}

/// Interns a given string, returning an `Interned` identifier for it.
///
/// If the string has already been interned, its existing `Interned` identifier
/// is returned. Otherwise, a copy of the string is made using the internal
/// `ArenaAllocator`, and a new `Interned` identifier is assigned and returned.
pub fn intern(self: *StringInterner, allocator: std.mem.Allocator, string: []const u8) !Interned {
    if (self.string_to_interned.get(string)) |interned| return interned;
    const string_copy = try self.allocator.allocator().dupe(u8, string);
    const interned_string = Interned{ .id = self.string_to_interned.count() };
    try self.interned_to_string.append(allocator, string_copy);
    try self.string_to_interned.put(allocator, string_copy, interned_string);
    return interned_string;
}

/// Retrieves the original string corresponding to an `Interned` identifier.
/// Returns `null` if the `Interned` identifier is invalid.
pub fn toString(self: StringInterner, interned: Interned) ?[]const u8 {
    const idx = @as(usize, interned.id);
    if (idx >= self.interned_to_string.items.len) return null;
    return self.interned_to_string.items[idx];
}

test "interned strings are equal" {
    var string_interner = StringInterner.init(testing.allocator);
    defer string_interner.deinit(testing.allocator);
    const interned_a = try string_interner.intern(testing.allocator, "interned");
    const interned_b = try string_interner.intern(testing.allocator, "interned");
    try testing.expectEqualDeep(interned_a, interned_b);
}

test "different strings are not equal" {
    var string_interner = StringInterner.init(testing.allocator);
    defer string_interner.deinit(testing.allocator);
    const interned_a = try string_interner.intern(testing.allocator, "interned_a");
    const interned_b = try string_interner.intern(testing.allocator, "interned_b");
    try testing.expect(!std.meta.eql(interned_a, interned_b));
}

test "interned string can convert back to string" {
    var string_interner = StringInterner.init(testing.allocator);
    defer string_interner.deinit(testing.allocator);
    const interned_a = try string_interner.intern(testing.allocator, "interned_a");
    const interned_b = try string_interner.intern(testing.allocator, "interned_b");
    try testing.expectEqualStrings(string_interner.toString(interned_a).?, "interned_a");
    try testing.expectEqualStrings(string_interner.toString(interned_b).?, "interned_b");
}
