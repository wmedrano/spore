const std = @import("std");

const ConsCell = @import("ConsCell.zig");
const Symbol = @import("Symbol.zig");
const Val = @import("Val.zig");
const Vm = @import("Vm.zig");

const LexicalScope = @This();

/// Represents a variable defined within the current compilation scope.
///
/// It tracks the variable's symbol and its corresponding index on the local
/// stack where its value will be stored or retrieved.
pub const Binding = struct {
    /// The symbol of the scoped variable.
    symbol: ?Symbol.Interned,
    /// The location of the symbol on the local stack.
    local_index: i32,
};

/// List of all bindings (i.e. variables) in this scope.
bindings: std.ArrayListUnmanaged(Binding) = .{},

/// Initializes a new `LexicalScope` with the provided arguments.
///
/// Each argument is added as a binding within the scope, associating its
/// symbol with an index on the local stack.
pub fn initWithArgs(allocator: std.mem.Allocator, vm: *const Vm, args: *ConsCell.ListIter) !LexicalScope {
    var bindings = std.ArrayListUnmanaged(Binding){};
    var arg_count: i32 = 0;
    while (try args.next(vm)) |arg| {
        const symbol = try Val.to(arg, Symbol.Interned);
        try bindings.append(allocator, Binding{
            .symbol = symbol,
            .local_index = arg_count,
        });
        arg_count += 1;
    }
    return LexicalScope{
        .bindings = bindings,
    };
}

/// Returns the minimum size required for the local stack to accommodate all
/// variables defined within this lexical scope.
pub fn minimumLocalStackSize(self: LexicalScope) i32 {
    var max_idx: i32 = -1;
    for (self.bindings.items) |binding| max_idx = @max(max_idx, binding.local_index);
    return max_idx + 1;
}

/// Retrieves a `Binding` for the given `symbol` within this lexical scope.
///
/// It searches from the most recently added bindings backwards to find the
/// correct binding.
///
/// Returns `null` if the symbol is not found.
pub fn get(self: LexicalScope, symbol: Symbol.Interned) ?Binding {
    const target: ?Symbol.Interned = symbol;
    for (0..self.bindings.items.len) |idx| {
        const variable = self.bindings.items[self.bindings.items.len - 1 - idx];
        if (std.meta.eql(variable.symbol, target)) return variable;
    }
    return null;
}

/// Adds a new named binding to the lexical scope.
///
/// The binding is associated with the given `symbol` and assigned the next
/// available local stack index.
pub fn add(self: *LexicalScope, allocator: std.mem.Allocator, symbol: Symbol.Interned) !Binding {
    return try self.addInternal(allocator, symbol);
}

/// Adds a new anonymous binding to the lexical scope.
///
/// An anonymous binding does not have an associated symbol but takes up a
/// slot on the local stack. This is useful for temporary values that need
/// to be stored but aren't directly referenced by name.
pub fn addAnonymous(self: *LexicalScope, allocator: std.mem.Allocator) !Binding {
    return try self.addInternal(allocator, null);
}

/// Removes a specific `Binding` from the lexical scope by setting its
/// symbol to `null`.
///
/// This effectively deactivates the binding, though its stack slot remains
/// allocated.
pub fn remove(self: *LexicalScope, target: Binding) void {
    for (self.bindings.items) |*binding| {
        if (std.meta.eql(binding.*, target)) {
            binding.symbol = null;
        }
    }
}

/// Internal helper function to add a new binding (named or anonymous) to the scope.
///
/// It assigns the next available local stack index to the new binding.
fn addInternal(self: *LexicalScope, allocator: std.mem.Allocator, symbol: ?Symbol.Interned) !Binding {
    const binding = Binding{
        .symbol = symbol,
        .local_index = @intCast(self.bindings.items.len),
    };
    try self.bindings.append(allocator, binding);
    return binding;
}
