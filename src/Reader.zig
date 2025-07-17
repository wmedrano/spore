//! Parse text into s-expressions formatted as `Val`.
const std = @import("std");
const testing = std.testing;

const ConsCell = @import("ConsCell.zig");
const Symbol = @import("datastructures/Symbol.zig");
const Tokenizer = @import("parser/Tokenizer.zig");
const Val = @import("Val.zig");
const Vm = @import("Vm.zig");

const Reader = @This();

/// The underlying tokenizer. Produces a stream of tokens/substrings that are
/// parsed.
tokenizer: Tokenizer,

/// Initialize a new `Reader`.
pub fn init(source: []const u8) !Reader {
    try validateSource(source);
    return .{ .tokenizer = Tokenizer.init(source) };
}

fn validateSource(source: []const u8) !void {
    var open_parens: i32 = 0;
    var tokenizer = Tokenizer.init(source);
    while (tokenizer.next()) |t| {
        switch (t.token_type) {
            .open_paren => open_parens += 1,
            .close_paren => {
                if (open_parens == 0) return error.ParseError;
                open_parens -= 1;
            },
            else => {},
        }
    }
    if (open_parens != 0) return error.ParseError;
}

/// Parses a single s-expression from the given source string.
///
/// Returns an error if the source contains more than one s-expression,
/// or if it's empty, or contains unbalanced parentheses or invalid syntax.
pub fn readOne(source: []const u8, allocator: std.mem.Allocator, vm: *Vm) !Val {
    var reader = try init(source);
    const val = try reader.next(allocator, vm) orelse return error.ParseError;
    // After parsing one expression, check if there are any remaining tokens.
    // If there are, it means the source contained more than one expression,
    // which is an error for readOne.
    if (try reader.next(allocator, vm)) |_| return error.ParseError;
    return val;
}

/// Get the next s-expression or `null` if there are no more s-expressions.
///
/// `allocator` is used to allocate intermediate data.
pub fn next(self: *Reader, allocator: std.mem.Allocator, vm: *Vm) !?Val {
    const initial_token = self.tokenizer.next() orelse return null;
    switch (initial_token.token_type) {
        .close_paren => return error.ParseError,
        .open_paren => return try self.nextExpr(allocator, vm),
        .identifier => return try identifierToVal(self.tokenizer.substr(initial_token), vm),
    }
}

/// Parses a list expression, consuming tokens until a matching `)` is found.
/// Assumes the initial `(` has already been consumed.
fn nextExpr(self: *Reader, allocator: std.mem.Allocator, vm: *Vm) !Val {
    var vals = std.ArrayList(Val).init(allocator);
    defer vals.deinit();
    while (self.tokenizer.next()) |token| {
        switch (token.token_type) {
            .close_paren => return listToVal(vals.items, vm),
            .open_paren => {
                const sub_expr = try self.nextExpr(allocator, vm);
                try vals.append(sub_expr);
            },
            .identifier => {
                const identifier = try identifierToVal(self.tokenizer.substr(token), vm);
                try vals.append(identifier);
            },
        }
    }
    return error.ParseError;
}

/// Converts a string identifier into a Lisp Object.
fn identifierToVal(identifier: []const u8, vm: *Vm) !Val {
    if (std.mem.eql(u8, identifier, "nil")) return Val.from({});
    if (std.fmt.parseInt(i64, identifier, 10) catch null) |x| return Val.from(x);
    if (std.fmt.parseFloat(f64, identifier) catch null) |x| return Val.from(x);
    const symbol = Symbol.init(identifier);
    const interned_symbol = try symbol.intern(vm.heap.allocator, &vm.heap.string_interner);
    return Val.from(interned_symbol);
}

/// Recursively constructs a Lisp list (a chain of `ConsCell`s) from a slice of
/// `Val`s.
fn listToVal(list: []const Val, vm: *Vm) !Val {
    if (list.len == 0) return Val.from({});
    const head = list[0];
    const tail = try listToVal(list[1..], vm);
    const cons = ConsCell.init(head, tail);
    const cons_handle = try vm.heap.cons_cells.create(
        vm.heap.allocator,
        cons,
        vm.heap.dead_color,
    );
    return Val.from(cons_handle);
}

test Reader {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var reader = try Reader.init("(+ 1 2) (- 1 2)");
    try std.testing.expectFmt(
        "(+ 1 2)",
        "{}",
        .{vm.prettyPrint((try reader.next(testing.allocator, &vm)).?)},
    );
    try std.testing.expectFmt(
        "(- 1 2)",
        "{}",
        .{vm.prettyPrint((try reader.next(testing.allocator, &vm)).?)},
    );
    try std.testing.expectEqualDeep(
        null,
        try reader.next(testing.allocator, &vm),
    );
}

test "unclosed parenthesis produces error" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try std.testing.expectError(
        error.ParseError,
        Reader.init("(+ 1 2 ("),
    );
}

test "unexpected close produces error" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    try std.testing.expectError(
        error.ParseError,
        Reader.init("  ) ()"),
    );
}

test "parse nil" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var reader = try Reader.init("nil");
    try std.testing.expectEqualDeep(
        Val.from({}),
        try reader.next(testing.allocator, &vm),
    );
}

test "parse integer" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var reader = try Reader.init("-1");
    try std.testing.expectEqualDeep(
        Val.from(-1),
        try reader.next(testing.allocator, &vm),
    );
}

test "parse float" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var reader = try Reader.init("3.14");
    try std.testing.expectEqualDeep(
        Val.from(3.14),
        try reader.next(testing.allocator, &vm),
    );
}

test "readOne single expression" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try std.testing.expectEqualDeep(
        Val.from(4),
        try Reader.readOne("4", testing.allocator, &vm),
    );
}

test "readOne empty produces error" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try std.testing.expectError(
        error.ParseError,
        Reader.readOne("", testing.allocator, &vm),
    );
}

test "readOne multiple expressions produces error" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try std.testing.expectError(
        error.ParseError,
        Reader.readOne("1 2", testing.allocator, &vm),
    );
}
