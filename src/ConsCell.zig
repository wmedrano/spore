//! A Cons cell, fundamental to Lisp-like data structures.
const std = @import("std");
const testing = std.testing;

const Val = @import("Val.zig");
const Vm = @import("Vm.zig");

/// A Cons cell, fundamental to Lisp-like data structures.
const ConsCell = @This();

/// The first element of the pair.
car: Val,
/// The second element of the pair.
cdr: Val,

/// Initializes a new `ConsCell` with the given car and cdr values.
pub fn init(car: Val, cdr: Val) ConsCell {
    return .{
        .car = car,
        .cdr = cdr,
    };
}

/// An iterator for traversing a `ConsCell` as a list.
pub const ListIter = struct {
    /// The current `ConsCell` being iterated, or null if the end of the list is
    /// reached.
    cons: ?ConsCell,

    /// Errors that can occur during list iteration.
    pub const Error = error{
        TypeError,
        ObjectNotFound,
    };

    /// Advances the iterator and returns the next element in the list or `null`
    /// if the end of the list is reached.
    ///
    /// Returns `ListIter.Error.TypeError` if the `cdr` is not a `cons` or `nil`.
    /// This is a property of valid lists.
    pub fn next(self: *ListIter, vm: *const Vm) Error!?Val {
        const cons = self.cons orelse return null;
        const ret = cons.car;
        switch (cons.cdr.repr) {
            .cons => |handle| self.cons = try vm.heap.cons_cells.get(handle),
            .nil => self.cons = null,
            else => return Error.TypeError,
        }
        return ret;
    }
};

/// Creates and returns a new `ListIter` for iterating over the `ConsCell` as a
/// list.
pub fn iterList(self: ConsCell) ListIter {
    return ListIter{
        .cons = self,
    };
}

/// Creates and returns a new `ListIter` for an empty list.
pub fn iterEmpty() ListIter {
    return ListIter{
        .cons = null,
    };
}
