//! A Cons cell, fundamental to Lisp-like data structures.
const Val = @import("Val.zig");

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
