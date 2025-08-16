//! A pair, fundamental to Lisp-like data structures.
const std = @import("std");
const testing = std.testing;

const Val = @import("Val.zig");
const Vm = @import("Vm.zig");

/// A pair, fundamental to Lisp-like data structures.
const Pair = @This();

/// The first element of the pair.
first: Val,
/// The second element of the pair.
second: Val,

/// Initializes a new `Pair` with the given first and second values.
pub fn init(first: Val, second: Val) Pair {
    return .{
        .first = first,
        .second = second,
    };
}

/// An iterator for traversing a `Pair` as a list.
pub const ListIter = struct {
    /// The current `Pair` being iterated, or null if the end of the list is
    /// reached.
    pair: ?Pair,

    /// Errors that can occur during list iteration.
    pub const Error = error{
        WrongType,
        ObjectNotFound,
    };

    /// Returns `true` if the list iterator is empty (i.e., at the end of the
    /// list), `false` otherwise.
    pub fn empty(self: ListIter) bool {
        if (self.pair) |_| return false;
        return true;
    }

    /// Advances the iterator and returns the next element in the list or `null`
    /// if the end of the list is reached.
    ///
    /// Returns `ListIter.Error.WrongType` if the `second` is not a `pair` or `nil`.
    /// This is a property of valid lists.
    pub fn next(self: *ListIter, vm: *const Vm) Error!?Val {
        const pair = self.pair orelse return null;
        const ret = pair.first;
        switch (pair.second.repr) {
            .pair => |handle| self.pair = try vm.heap.pairs.get(handle),
            .nil => self.pair = null,
            else => return Error.WrongType,
        }
        return ret;
    }
};

/// Creates and returns a new `ListIter` for iterating over the `Pair` as a
/// list.
pub fn iterList(self: Pair) ListIter {
    return ListIter{
        .pair = self,
    };
}

/// Creates and returns a new `ListIter` for an empty list.
pub fn iterEmpty() ListIter {
    return ListIter{
        .pair = null,
    };
}
