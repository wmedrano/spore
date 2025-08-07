const std = @import("std");

const ConsCell = @import("ConsCell.zig");
const Symbol = @import("Symbol.zig");
const errors = @import("errors.zig");
const DetailedError = errors.DetailedError;
const NativeFunction = @import("NativeFunction.zig");
const PrettyPrinter = @import("PrettyPrinter.zig");
const String = @import("String.zig");
const Val = @import("Val.zig");
const Vm = @import("Vm.zig");

const Builder = @This();

vm: *Vm,

/// Converts a slice of Vals to a list.
/// Converts a slice of Vals to a list.
pub fn list(self: Builder, vals: []const Val) errors.Error!Val {
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
/// Creates a new cons cell with the given car and cdr.
///
/// Args:
///     car: The car of the cons cell.
///     cdr: The cdr of the cons cell.
///
/// Returns:
///     A Val representing the new cons cell.
pub fn cons(self: Builder, car: Val, cdr: Val) errors.Error!Val {
    const cons_handle = self.vm.heap.cons_cells.create(
        self.vm.heap.allocator,
        ConsCell.init(car, cdr),
        self.vm.heap.unreachable_color,
    ) catch |err| switch (err) {
        error.OutOfMemory => return self.addError(DetailedError{ .out_of_memory = {} }),
    };
    return Val.init(cons_handle);
}

/// Creates a new string by copying the given value.
///
/// Args:
///     s: The string value. This function copies the string.
///
/// Returns:
///     A Val representing the new string.
/// Creates a new string by copying the given value.
///
/// Args:
///     s: The string value. This function copies the string.
///
/// Returns:
///     A Val representing the new string.
pub fn string(self: Builder, s: []const u8) errors.Error!Val {
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
/// Creates a new owned string with the given value.
///
/// Args:
/// s: The string value. This function takes ownership of the string, and the
///    caller should not use it after this call.
///
/// Returns:
///     A Val representing the new string.
pub fn stringOwned(self: Builder, s: []const u8) errors.Error!Val {
    const handle = try self.vm.heap.strings.create(
        self.vm.heap.allocator,
        String.initOwned(s),
        self.vm.heap.unreachable_color,
    );
    return Val.init(handle);
}

/// Creates a new symbol from the given Symbol object.
///
/// Args:
///     sym: The Symbol object to convert.
///
/// Returns:
///     A Val representing the new symbol.
pub fn symbol(self: Builder, sym: Symbol) errors.Error!Val {
    return Val.init(try self.internedSymbol(sym));
}

/// Interns the given Symbol object.
///
/// This function takes a Symbol object and interns its string representation
/// using the VM's string interner, ensuring that only one copy of each unique
/// symbol string exists in memory.
///
/// Args:
///     sym: The Symbol object to intern.
///
/// Returns:
///     An Interned symbol.
pub fn internedSymbol(self: Builder, sym: Symbol) errors.Error!Symbol.Interned {
    return sym.intern(self.vm.heap.allocator, &self.vm.heap.string_interner);
}

/// Add an error to the virtual machine.
pub fn addError(self: Builder, err: errors.DetailedError) errors.Error {
    self.vm.execution_context.last_error = err;
    return err.err();
}
