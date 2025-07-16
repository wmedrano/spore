//! Implements a generic object pool for efficient storage and retrieval of
//! objects.
//!
//! This module provides an `ObjectPool` data structure that allows for storing
//! objects contiguously and accessing them via lightweight `Handle`s. It is
//! designed for scenarios where frequent allocation and deallocation of objects
//! can lead to performance overhead or fragmentation.
const std = @import("std");
const testing = std.testing;

/// A lightweight handle to an object within an `ObjectPool`.
///
/// It primarily stores the index (`id`) of the object in the pool.
pub fn Handle(comptime T: type) type {
    return struct {
        const _ = T;
        id: u32,
    };
}

/// An iterator over the objects in an `ObjectPool`.
pub fn Iterator(comptime T: type) type {
    return struct {
        const Self = @This();
        /// The current index in the `items` array.
        current_index: usize,
        /// A slice of the objects in the pool.
        items: []T,

        /// Returns the next object in the iteration, or `null` if all objects
        /// have been iterated.
        pub fn next(self: *Self) ?*T {
            if (self.current_index >= self.items.len) {
                return null;
            }
            const obj = &self.items[self.current_index];
            self.current_index += 1;
            return obj;
        }
    };
}

/// A generic pool for managing objects, allowing them to be stored and
/// retrieved via lightweight `Handle`s.
///
/// Objects are stored contiguously, and handles are essentially indices into an
/// internal array.
pub fn ObjectPool(comptime T: type) type {
    return struct {
        const Self = @This();
        const Object = struct {
            value: T,
        };
        /// The list of objects stored in the pool.
        objects: std.MultiArrayList(Object) = .{},

        /// Deinitialize `self`, freeing all allocated resources.
        /// This must be called when the pool is no longer needed.
        pub fn deinit(self: *Self, allocator: std.mem.Allocator) void {
            self.objects.deinit(allocator);
        }

        /// Add `obj` to the object pool and return its `Handle`.  The object is
        /// appended to the internal storage.
        ///
        /// Returns an error if memory allocation fails.
        pub fn create(self: *Self, allocator: std.mem.Allocator, value: T) !Handle(T) {
            const id = Handle(T){ .id = @intCast(self.objects.len) };
            try self.objects.append(allocator, .{ .value = value });
            return id;
        }

        /// Get an object by its `Handle`.
        ///
        /// Returns an error `error.ObjectNotFound` if the handle's ID is out of bounds.
        pub fn get(self: Self, handle: Handle(T)) !T {
            const idx = handle.id;
            if (idx >= self.objects.len) return error.ObjectNotFound;
            return self.objects.items(.value)[idx];
        }

        /// Returns an iterator over the objects in the pool.
        pub fn iter(self: Self) Iterator(T) {
            return .{
                .current_index = 0,
                .items = self.objects.items(.value),
            };
        }
    };
}

test "create object can be returned with get" {
    var pool = ObjectPool(usize){};
    defer pool.deinit(testing.allocator);
    _ = try pool.create(testing.allocator, 1);
    const id = try pool.create(testing.allocator, 10);
    _ = try pool.create(testing.allocator, 100);

    try testing.expectEqual(10, try pool.get(id));
}

test "iter iterates over all items in pool" {
    var pool = ObjectPool(usize){};
    defer pool.deinit(testing.allocator);

    _ = try pool.create(testing.allocator, 10);
    _ = try pool.create(testing.allocator, 20);
    _ = try pool.create(testing.allocator, 30);

    var iter = pool.iter();
    try testing.expectEqual(10, iter.next().?.*);
    try testing.expectEqual(20, iter.next().?.*);
    try testing.expectEqual(30, iter.next().?.*);
    try testing.expectEqual(null, iter.next());
}
