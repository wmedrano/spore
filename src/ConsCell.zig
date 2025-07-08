//! A Cons cell, fundamental to Lisp-like data structures.
const Val = @import("Val.zig");

const ConsCell = @This();

car: Val,
cdr: Val,

/// Initializes a new `ConsCell` with the given car and cdr values.
pub fn init(car: Val, cdr: Val) ConsCell {
    return .{
        .car = car,
        .cdr = cdr,
    };
}
