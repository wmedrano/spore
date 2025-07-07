const std = @import("std");
const testing = std.testing;

/// A simple tokenizer for S-expressions.
const Tokenizer = @This();

/// The input text being tokenized.
text: []const u8,
/// The current position in the `text`.
start: usize = 0,

/// Represents a span of text within the original input.
pub const Span = struct {
    /// The starting index of the span (inclusive).
    start: usize = 0,
    /// The ending index of the span (exclusive).
    end: usize = 0,

    /// Returns the substring corresponding to this span within the given text.
    pub fn substr(self: Span, text: []const u8) []const u8 {
        return text[self.start..self.end];
    }
};

/// Initializes a new `Tokenizer` with the given text.
pub fn init(text: []const u8) Tokenizer {
    return .{
        .text = text,
    };
}

/// Checks if the tokenizer has processed all the input text.
pub fn is_done(self: Tokenizer) bool {
    return self.start >= self.text.len;
}

/// Retrieves the next token as a `Span`.
/// Skips leading whitespace. Returns `null` if no more tokens are available.
/// Tokens can be:
/// - Parentheses `(` or `)`
/// - Identifiers (sequences of non-whitespace, non-parenthesis characters)
pub fn next(self: *Tokenizer) ?Span {
    if (self.is_done()) return null;
    self.eatWhitespace();
    if (self.is_done()) return null; // Check again after eating whitespace
    const next_ch = self.text[self.start];
    if (isParen(next_ch)) {
        const ret = Span{ .start = self.start, .end = self.start + 1 };
        self.start += 1;
        return ret;
    }
    const start = self.start;
    self.eatIdentifier();
    return .{ .start = start, .end = self.start };
}

/// Similar to `next`, but returns the substring directly instead of a `Span`.
/// Returns `null` if no more tokens are available.
pub fn nextStr(self: *Tokenizer) ?[]const u8 {
    const span = self.next() orelse return null;
    return span.substr(self.text);
}

/// Checks if a given character is an opening or closing parenthesis.
fn isParen(ch: u8) bool {
    return ch == '(' or ch == ')';
}

/// Advances the tokenizer's `start` pointer past any leading whitespace characters.
fn eatWhitespace(self: *Tokenizer) void {
    while (!self.is_done()) {
        if (!std.ascii.isWhitespace(self.text[self.start])) return;
        self.start += 1;
    }
}

/// Advances the tokenizer's `start` pointer past an identifier.
/// An identifier is defined as a sequence of non-whitespace, non-parenthesis characters.
fn eatIdentifier(self: *Tokenizer) void {
    while (!self.is_done()) {
        const next_ch = self.text[self.start];
        if (std.ascii.isWhitespace(next_ch)) return;
        if (isParen(next_ch)) return;
        self.start += 1;
    }
}

test "empty string has no tokens" {
    var tokenizer = Tokenizer.init("");
    try testing.expectEqualDeep(null, tokenizer.next());
    try testing.expectEqualDeep(null, tokenizer.next());
}

test "s-expression returns each token" {
    var tokenizer = Tokenizer.init("(+ one 2)");
    try testing.expectEqualDeep(Span{ .start = 0, .end = 1 }, tokenizer.next());
    try testing.expectEqualDeep(Span{ .start = 1, .end = 2 }, tokenizer.next());
    try testing.expectEqualDeep(Span{ .start = 3, .end = 6 }, tokenizer.next());
    try testing.expectEqualDeep(Span{ .start = 7, .end = 8 }, tokenizer.next());
    try testing.expectEqualDeep(Span{ .start = 8, .end = 9 }, tokenizer.next());
    try testing.expectEqualDeep(null, tokenizer.next());
}

test "complex s-expression returns each token via nextStr" {
    var tokenizer = Tokenizer.init("(plus (divide one two) tree)");
    try testing.expectEqualStrings("(", tokenizer.nextStr().?);
    try testing.expectEqualStrings("plus", tokenizer.nextStr().?);
    try testing.expectEqualStrings("(", tokenizer.nextStr().?);
    try testing.expectEqualStrings("divide", tokenizer.nextStr().?);
    try testing.expectEqualStrings("one", tokenizer.nextStr().?);
    try testing.expectEqualStrings("two", tokenizer.nextStr().?);
    try testing.expectEqualStrings(")", tokenizer.nextStr().?);
    try testing.expectEqualStrings("tree", tokenizer.nextStr().?);
    try testing.expectEqualStrings(")", tokenizer.nextStr().?);
    try testing.expectEqualDeep(null, tokenizer.nextStr());
}
