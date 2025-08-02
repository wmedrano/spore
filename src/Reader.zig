//! Parse text into s-expressions formatted as `Val`.
const std = @import("std");
const testing = std.testing;

const ConsCell = @import("ConsCell.zig");
const Symbol = @import("datastructures/Symbol.zig");
const Tokenizer = @import("parser/Tokenizer.zig");
const String = @import("String.zig");
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
    while (self.tokenizer.next()) |token| {
        switch (token.token_type) {
            .close_paren => return error.ParseError,
            .open_paren => return try self.nextExpr(allocator, vm),
            .identifier => return try identifierToVal(self.tokenizer.substr(token), vm),
            .string => return try stringToVal(self.tokenizer.substr(token), vm),
            .comment => {},
        }
    }
    return null;
}

/// Parses a list expression, consuming tokens until a matching `)` is found.
/// Assumes the initial `(` has already been consumed.
fn nextExpr(self: *Reader, allocator: std.mem.Allocator, vm: *Vm) !Val {
    var vals = std.ArrayList(Val).init(allocator);
    defer vals.deinit();
    while (self.tokenizer.next()) |token| {
        switch (token.token_type) {
            .close_paren => return vm.builder().list(vals.items),
            .open_paren => {
                const sub_expr = try self.nextExpr(allocator, vm);
                try vals.append(sub_expr);
            },
            .identifier => {
                const identifier = try identifierToVal(self.tokenizer.substr(token), vm);
                try vals.append(identifier);
            },
            .string => {
                const string = try stringToVal(self.tokenizer.substr(token), vm);
                try vals.append(string);
            },
            .comment => {},
        }
    }
    return error.ParseError;
}

/// Converts a string identifier into a Lisp Object.
fn identifierToVal(identifier: []const u8, vm: *Vm) !Val {
    if (std.mem.eql(u8, identifier, "nil")) return Val.init({});
    if (std.mem.eql(u8, identifier, "true")) return Val.init(true);
    if (std.mem.eql(u8, identifier, "false")) return Val.init(false);
    if (std.fmt.parseInt(i64, identifier, 10) catch null) |x| return Val.init(x);
    if (std.fmt.parseFloat(f64, identifier) catch null) |x| return Val.init(x);
    return vm.builder().symbol(Symbol.init(identifier));
}

/// Converts a string representation into a Lisp string.
fn stringToVal(identifier: []const u8, vm: *Vm) !Val {
    if (identifier.len < 2) return error.ParseError;
    if (identifier[0] != '"') return error.ParseError;
    if (identifier[identifier.len - 1] != '"') return error.ParseError;

    var string = std.ArrayList(u8).init(vm.heap.allocator);
    defer string.deinit();
    var escaped = false;
    for (identifier[1 .. identifier.len - 1]) |ch| {
        if (escaped) {
            escaped = false;
            try string.append(ch);
        } else {
            switch (ch) {
                '\\' => escaped = true,
                '"' => return error.ParseError,
                else => try string.append(ch),
            }
        }
    }
    if (escaped) return error.ParseError;
    const val = try vm.builder().string(string.items);
    return val;
}

test Reader {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var reader = try Reader.init("(+ 1 2) (- 1 2)");
    try std.testing.expectFmt(
        "(+ 1 2)",
        "{}",
        .{vm.inspector().pretty((try reader.next(testing.allocator, &vm)).?)},
    );
    try std.testing.expectFmt(
        "(- 1 2)",
        "{}",
        .{vm.inspector().pretty((try reader.next(testing.allocator, &vm)).?)},
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
        Val.init({}),
        try reader.next(testing.allocator, &vm),
    );
}

test "parse integer" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var reader = try Reader.init("-1");
    try std.testing.expectEqualDeep(
        Val.init(-1),
        try reader.next(testing.allocator, &vm),
    );
}

test "parse float" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var reader = try Reader.init("3.14");
    try std.testing.expectEqualDeep(
        Val.init(3.14),
        try reader.next(testing.allocator, &vm),
    );
}

test "comment is ignored" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var reader = try Reader.init(
        \\; this is a comment
        \\42
    );
    const val = try reader.next(testing.allocator, &vm);
    try testing.expectEqualDeep(Val.init(42), val);
    try testing.expectEqualDeep(null, try reader.next(testing.allocator, &vm));
}

test "comment inside an expression is ignored" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var reader = try Reader.init(
        \\(1 ; comment
        \\ 2)
    );
    const val = try reader.next(testing.allocator, &vm);
    try testing.expectFmt(
        "(1 2)",
        "{}",
        .{vm.inspector().pretty(val.?)},
    );
}

test "multiple comments are ignored" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var reader = try Reader.init(
        \\; line 1
        \\   ; line 2
        \\()
    );
    try testing.expectEqual(
        Val.init({}),
        try reader.next(testing.allocator, &vm),
    );
}

test "readOne single expression" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    try std.testing.expectEqualDeep(
        Val.init(4),
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

test "parse true" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var reader = try Reader.init("true");
    try std.testing.expectEqualDeep(
        Val.init(true),
        try reader.next(testing.allocator, &vm),
    );
}

test "parse false" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();
    var reader = try Reader.init("false");
    try std.testing.expectEqualDeep(
        Val.init(false),
        try reader.next(testing.allocator, &vm),
    );
}
