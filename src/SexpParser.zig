//! Parse text into s-expressions formatted as `Val`.
const std = @import("std");
const testing = std.testing;

const ConsCell = @import("ConsCell.zig");
const Symbol = @import("datastructures/Symbol.zig");
const Tokenizer = @import("parser/Tokenizer.zig");
const Val = @import("Val.zig");
const Vm = @import("Vm.zig");
const PrettyPrinter = @import("PrettyPrinter.zig");

const SexpParser = @This();

/// The underlying tokenizer. Produces a stream of tokens/substrings that are
/// parsed.
tokenizer: Tokenizer,

/// Initialize a new `SexpParser`.
pub fn init(source: []const u8) SexpParser {
    return .{ .tokenizer = Tokenizer.init(source) };
}

/// Get the next s-expression or `null` if there are no more s-expressions.
pub fn next(self: *SexpParser, allocator: std.mem.Allocator, vm: *Vm) !?Val {
    const initial_token = self.tokenizer.next() orelse return null;
    switch (initial_token.token_type) {
        .close_paren => return error.ParseError,
        .open_paren => return try self.nextExpr(allocator, vm),
        .identifier => return try identifierToVal(self.tokenizer.substr(initial_token), vm),
    }
}

/// Parses a list expression, consuming tokens until a matching `)` is found.
/// Assumes the initial `(` has already been consumed.
fn nextExpr(self: *SexpParser, allocator: std.mem.Allocator, vm: *Vm) !Val {
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
    const interned_symbol = try symbol.intern(vm.allocator, &vm.string_interner);
    return Val.from(interned_symbol);
}

/// Recursively constructs a Lisp list (a chain of `ConsCell`s) from a slice of
/// `Val`s.
fn listToVal(list: []const Val, vm: *Vm) !Val {
    if (list.len == 0) return Val.from({});
    const head = list[0];
    const tail = try listToVal(list[1..], vm);
    const cons = ConsCell.init(head, tail);
    const cons_handle = try vm.cons_cells.create(vm.allocator, cons);
    return Val.from(cons_handle);
}

test SexpParser {
    var vm = Vm.init(testing.allocator);
    defer vm.deinit();
    var sexp_parser = SexpParser.init("(+ 1 2) (- 1 2)");
    try std.testing.expectFmt(
        "(+ 1 2)",
        "{}",
        .{PrettyPrinter.init(&vm, (try sexp_parser.next(testing.allocator, &vm)).?)},
    );
    try std.testing.expectFmt(
        "(- 1 2)",
        "{}",
        .{PrettyPrinter.init(&vm, (try sexp_parser.next(testing.allocator, &vm)).?)},
    );
    try std.testing.expectEqualDeep(
        null,
        try sexp_parser.next(testing.allocator, &vm),
    );
}

test "unclosed parenthesis produces error" {
    var vm = Vm.init(testing.allocator);
    defer vm.deinit();
    var sexp_parser = SexpParser.init("(+ 1 2 (");
    try std.testing.expectError(error.ParseError, sexp_parser.next(testing.allocator, &vm));
}

test "unexpected close produces error" {
    var vm = Vm.init(testing.allocator);
    defer vm.deinit();
    var sexp_parser = SexpParser.init("  ) ()");
    try std.testing.expectError(error.ParseError, sexp_parser.next(testing.allocator, &vm));
}

test "parse nil" {
    var vm = Vm.init(testing.allocator);
    defer vm.deinit();
    var sexp_parser = SexpParser.init("nil");
    try std.testing.expectEqualDeep(
        Val.from({}),
        try sexp_parser.next(testing.allocator, &vm),
    );
}

test "parse integer" {
    var vm = Vm.init(testing.allocator);
    defer vm.deinit();
    var sexp_parser = SexpParser.init("-1");
    try std.testing.expectEqualDeep(
        Val.from(-1),
        try sexp_parser.next(testing.allocator, &vm),
    );
}

test "parse float" {
    var vm = Vm.init(testing.allocator);
    defer vm.deinit();
    var sexp_parser = SexpParser.init("3.14");
    try std.testing.expectEqualDeep(
        Val.from(3.14),
        try sexp_parser.next(testing.allocator, &vm),
    );
}
