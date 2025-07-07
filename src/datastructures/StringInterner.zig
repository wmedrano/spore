const std = @import("std");

const StringInterner = @This();

pub const Interned = struct {
    id: u32,
};

allocator: std.heap.ArenaAllocator,
string_to_interned: std.StringHashMapUnmanaged(Interned) = .{},
interned_to_string: std.ArrayListUnmanaged([]const u8) = .{},

pub fn init() StringInterner {
    return .{ .allocator = std.heap.ArenaAllocator.init(std.heap.page_allocator) };
}

pub fn deinit(self: *StringInterner, allocator: std.mem.Allocator) void {
    self.allocator.deinit();
    self.string_to_interned.deinit(allocator);
    self.interned_to_string.deinit(allocator);
}

pub fn intern(self: *StringInterner, allocator: std.mem.Allocator, string: []const u8) !Interned {
    if (self.string_to_interned.get(string)) |interned| return interned;
    const string_copy = try self.allocator.allocator().dupe(u8, string);
    const interned_string = Interned{ .id = self.string_to_interned.count() };
    try self.interned_to_string.append(allocator, string_copy);
    try self.string_to_interned.put(allocator, string_copy, interned_string);
    return interned_string;
}

pub fn toString(self: StringInterner, interned: Interned) ?[]const u8 {
    const idx = @as(usize, interned.id);
    if (idx >= self.interned_to_string.items.len) return null;
    return self.interned_to_string.items[idx];
}

test "interned strings are equal" {
    var string_interner = StringInterner.init();
    defer string_interner.deinit(std.testing.allocator);
    const interned_a = string_interner.intern(std.testing.allocator, "interned");
    const interned_b = string_interner.intern(std.testing.allocator, "interned");
    try std.testing.expectEqualDeep(interned_a, interned_b);
}

test "different strings are not equal" {
    var string_interner = StringInterner.init();
    defer string_interner.deinit(std.testing.allocator);
    const interned_a = string_interner.intern(std.testing.allocator, "interned_a");
    const interned_b = string_interner.intern(std.testing.allocator, "interned_b");
    try std.testing.expect(!std.meta.eql(interned_a, interned_b));
}

test "interned string can convert back to string" {
    var string_interner = StringInterner.init();
    defer string_interner.deinit(std.testing.allocator);
    const interned_a = try string_interner.intern(std.testing.allocator, "interned_a");
    const interned_b = try string_interner.intern(std.testing.allocator, "interned_b");
    try std.testing.expectEqualStrings(string_interner.toString(interned_a).?, "interned_a");
    try std.testing.expectEqualStrings(string_interner.toString(interned_b).?, "interned_b");
}
