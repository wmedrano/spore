const std = @import("std");
const testing = std.testing;

/// A lightweight handle to an object within an `ObjectPool`.
///
/// It primarily stores the index (`id`) of the object in the pool.
pub fn Handle(comptime T: type) type {
    _ = T;
    return struct {
        id: u32,
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
        /// The list of objects stored in the pool.
        objects: std.ArrayListUnmanaged(T) = .{},

        /// Deinitialize `self`, freeing all allocated resources.
        /// This must be called when the pool is no longer needed.
        pub fn deinit(self: *Self, allocator: std.mem.Allocator) void {
            self.objects.deinit(allocator);
        }

        /// Add `obj` to the object pool and return its `Handle`.  The object is
        /// appended to the internal storage.
        ///
        /// Returns an error if memory allocation fails.
        pub fn create(self: *Self, allocator: std.mem.Allocator, obj: T) !Handle(T) {
            const id = Handle(T){ .id = @intCast(self.objects.items.len) };
            try self.objects.append(allocator, obj);
            return id;
        }

        /// Get an object by its `Handle`.
        ///
        /// Returns an error `error.ObjectNotFound` if the handle's ID is out of bounds.
        pub fn get(self: Self, handle: Handle(T)) !T {
            const idx = handle.id;
            if (idx >= self.objects.items.len) return error.ObjectNotFound;
            return self.objects.items[idx];
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
