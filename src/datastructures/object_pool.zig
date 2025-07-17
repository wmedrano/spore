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
        /// A slice to determine if the object is alive.
        alive: []bool,

        const IterT = struct { handle: Handle(T), value: *T };

        /// Returns the next object in the iteration, or `null` if all objects
        /// have been iterated.
        pub fn next(self: *Self) ?*T {
            const next_with_handle = self.nextWithHandle() orelse return null;
            return next_with_handle.value;
        }

        /// Returns the next value and handle in the iteration, or `null` if all
        /// objects have been iterated.
        pub fn nextWithHandle(self: *Self) ?IterT {
            while (self.current_index < self.items.len and !self.alive[self.current_index])
                self.current_index += 1;
            if (self.current_index >= self.items.len) {
                return null;
            }
            const ret = IterT{
                .handle = Handle(T){ .id = @intCast(self.current_index) },
                .value = &self.items[self.current_index],
            };
            self.current_index += 1;
            return ret;
        }
    };
}

pub fn SweepedIter(comptime T: type) type {
    return struct {
        const Self = @This();
        /// The handles that were sweeped.
        handles: []const Handle(T),
        /// All the objects.
        values: []T,
        /// The next handle.
        index: usize,

        pub fn next(self: *Self) ?*T {
            if (self.index >= self.handles.len) return null;
            const handle = self.handles[self.index];
            self.index += 1;
            return &self.values[handle.id];
        }
    };
}

pub const Color = enum {
    red,
    blue,
    pub fn swap(self: Color) Color {
        return switch (self) {
            .red => .blue,
            .blue => .red,
        };
    }
};

/// A generic pool for managing objects, allowing them to be stored and
/// retrieved via lightweight `Handle`s.
///
/// Objects are stored contiguously, and handles are essentially indices into an
/// internal array.
pub fn ObjectPool(comptime T: type) type {
    return struct {
        const Self = @This();
        const Slot = struct {
            value: T,
            color: Color,
            alive: bool = true,
        };
        /// The list of objects stored in the pool.
        objects: std.MultiArrayList(Slot) = .{},
        /// A list of indices that are free for use.
        free_list: std.ArrayListUnmanaged(Handle(T)) = .{},

        /// Deinitialize `self`, freeing all allocated resources.
        /// This must be called when the pool is no longer needed.
        pub fn deinit(self: *Self, allocator: std.mem.Allocator) void {
            self.objects.deinit(allocator);
            self.free_list.deinit(allocator);
        }

        /// Add `obj` to the object pool and return its `Handle`.  The object is
        /// appended to the internal storage.
        ///
        /// Returns an error if memory allocation fails.
        pub fn create(self: *Self, allocator: std.mem.Allocator, value: T, color: Color) !Handle(T) {
            if (self.free_list.pop()) |recycled_id| {
                self.objects.set(recycled_id.id, Slot{ .value = value, .color = color });
                return recycled_id;
            }
            const id = Handle(T){ .id = @intCast(self.objects.len) };
            try self.objects.append(allocator, Slot{
                .value = value,
                .color = color,
            });
            return id;
        }

        /// Get an object by its `Handle`.
        ///
        /// Returns an error `error.ObjectNotFound` if the handle's ID is out of bounds.
        pub fn get(self: Self, handle: Handle(T)) !T {
            const idx = handle.id;
            if (idx >= self.objects.len) return error.ObjectNotFound;
            const slice = self.objects.slice();
            if (!slice.items(.alive)[idx]) return error.ObjectNotFound;
            return slice.items(.value)[idx];
        }

        /// Sets the color of the object identified by `handle` to `color`.
        ///
        /// Returns the old color of the object.
        pub fn setColor(self: *Self, handle: Handle(T), color: Color) Color {
            var colors = self.objects.items(.color);
            const idx: usize = @intCast(handle.id);
            const old_color = colors[idx];
            colors[idx] = color;
            return old_color;
        }

        /// Returns an iterator over the objects in the pool.
        pub fn iter(self: Self) Iterator(T) {
            return .{
                .current_index = 0,
                .items = self.objects.items(.value),
                .alive = self.objects.items(.alive),
            };
        }

        /// Sweep all objects that are of `target_color`.
        pub fn sweep(self: *Self, allocator: std.mem.Allocator, target_color: Color) !SweepedIter(T) {
            const slice = self.objects.slice();
            const free_list_start = self.free_list.items.len;
            for (0..slice.len, slice.items(.alive), slice.items(.color)) |idx, *alive, color| {
                if (!alive.*) continue;
                if (color == target_color) {
                    const handle = Handle(T){ .id = @intCast(idx) };
                    try self.free_list.append(allocator, handle);
                    alive.* = false;
                }
            }
            return SweepedIter(T){
                .handles = self.free_list.items[free_list_start..],
                .values = slice.items(.value),
                .index = 0,
            };
        }
    };
}

test "create object can be returned with get" {
    var pool = ObjectPool(usize){};
    defer pool.deinit(testing.allocator);
    _ = try pool.create(testing.allocator, 1, .red);
    const id = try pool.create(testing.allocator, 10, .red);
    _ = try pool.create(testing.allocator, 100, .red);

    try testing.expectEqual(10, try pool.get(id));
}

test "iter iterates over all items in pool" {
    var pool = ObjectPool(usize){};
    defer pool.deinit(testing.allocator);

    _ = try pool.create(testing.allocator, 10, .red);
    _ = try pool.create(testing.allocator, 20, .red);
    _ = try pool.create(testing.allocator, 30, .red);

    var iter = pool.iter();
    try testing.expectEqual(10, iter.next().?.*);
    try testing.expectEqual(20, iter.next().?.*);
    try testing.expectEqual(30, iter.next().?.*);
    try testing.expectEqual(null, iter.next());
}
