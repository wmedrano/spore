const ConsCell = @import("ConsCell.zig");
const NativeFunction = @import("NativeFunction.zig");
const PrettyPrinter = @import("PrettyPrinter.zig");
const String = @import("String.zig");
const Val = @import("Val.zig");
const Vm = @import("Vm.zig");

const Builder = @This();

vm: *Vm,

/// Converts a slice of Vals to a list.
pub fn list(self: Builder, vals: []const Val) NativeFunction.Error!Val {
    if (vals.len == 0) return Val.init({});
    const head = vals[0];
    const tail = try self.list(vals[1..]);
    return self.cons(head, tail);
}

/// Creates a new cons cell with the given car and cdr.
///
/// Args:
///     car: The car of the cons cell.
///     cdr: The cdr of the cons cell.
///
/// Returns:
///     A Val representing the new cons cell.
pub fn cons(self: Builder, car: Val, cdr: Val) NativeFunction.Error!Val {
    const cons_handle = try self.vm.heap.cons_cells.create(
        self.vm.heap.allocator,
        ConsCell.init(car, cdr),
        self.vm.heap.unreachable_color,
    );
    return Val.init(cons_handle);
}

/// Creates a new string by copying the given value.
///
/// Args:
///     s: The string value. This function copies the string.
///
/// Returns:
///     A Val representing the new string.
pub fn string(self: Builder, s: []const u8) NativeFunction.Error!Val {
    const handle = try self.vm.heap.strings.create(
        self.vm.heap.allocator,
        try String.initCopy(self.vm.heap.allocator, s),
        self.vm.heap.unreachable_color,
    );
    return Val.init(handle);
}

/// Creates a new owned string with the given value.
///
/// Args:
/// s: The string value. This function takes ownership of the string, and the
///    caller should not use it after this call.
///
/// Returns:
///     A Val representing the new string.
pub fn stringOwned(self: Builder, s: []const u8) NativeFunction.Error!Val {
    const handle = try self.vm.heap.strings.create(
        self.vm.heap.allocator,
        String.initOwned(s),
        self.vm.heap.unreachable_color,
    );
    return Val.init(handle);
}
