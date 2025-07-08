const std = @import("std");
const testing = std.testing;

/// A simple tokenizer for S-expressions.
const Tokenizer = @This();

/// The input text being tokenized.
text: []const u8,
/// The current position in the `text`.
start: usize = 0,

/// The types of tokens that can be produced.
pub const TokenType = enum { open_paren, close_paren, identifier };

/// Represents a span of text within the original input.
pub const Token = struct {
    /// The starting index of the span (inclusive).
    start: usize = 0,
    /// The ending index of the span (exclusive).
    end: usize = 0,
    /// The type of token in the span.
    token_type: TokenType,
};

/// Initializes a new `Tokenizer` with the given text.
pub fn init(text: []const u8) Tokenizer {
    return .{
        .text = text,
    };
}

/// Checks if the tokenizer has processed all the input text.
pub fn isDone(self: Tokenizer) bool {
    return self.start >= self.text.len;
}

/// Returns the substring corresponding to this span within the given text.
pub fn substr(self: Tokenizer, token: Token) []const u8 {
    return self.text[token.start..token.end];
}

/// Retrieves the next token as a `Token`.
/// Skips leading whitespace. Returns `null` if no more tokens are available.
/// Tokens can be:
/// - Parentheses `(` or `)`
/// - Identifiers (sequences of non-whitespace, non-parenthesis characters)
pub fn next(self: *Tokenizer) ?Token {
    if (self.isDone()) return null;
    self.eatWhitespace();
    if (self.isDone()) return null; // Check again after eating whitespace
    const next_ch = self.text[self.start];
    if (isParen(next_ch)) {
        const ret = Token{
            .start = self.start,
            .end = self.start + 1,
            .token_type = if (next_ch == '(') .open_paren else .close_paren,
        };
        self.start += 1;
        return ret;
    }
    const start = self.start;
    self.eatIdentifier();
    return .{ .start = start, .end = self.start, .token_type = .identifier };
}

/// Similar to `next`, but returns the substring directly instead of a `Token`.
/// Returns `null` if no more tokens are available.
pub fn nextStr(self: *Tokenizer) ?[]const u8 {
    const token = self.next() orelse return null;
    return self.substr(token);
}

/// Checks if a given character is an opening or closing parenthesis.
fn isParen(ch: u8) bool {
    return ch == '(' or ch == ')';
}

/// Advances the tokenizer's `start` pointer past any leading whitespace characters.
fn eatWhitespace(self: *Tokenizer) void {
    while (!self.isDone()) {
        if (!std.ascii.isWhitespace(self.text[self.start])) return;
        self.start += 1;
    }
}

/// Advances the tokenizer's `start` pointer past an identifier.
/// An identifier is defined as a sequence of non-whitespace, non-parenthesis characters.
fn eatIdentifier(self: *Tokenizer) void {
    while (!self.isDone()) {
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
    try testing.expectEqualDeep(Token{ .start = 0, .end = 1, .token_type = .open_paren }, tokenizer.next());
    try testing.expectEqualDeep(Token{ .start = 1, .end = 2, .token_type = .identifier }, tokenizer.next());
    try testing.expectEqualDeep(Token{ .start = 3, .end = 6, .token_type = .identifier }, tokenizer.next());
    try testing.expectEqualDeep(Token{ .start = 7, .end = 8, .token_type = .identifier }, tokenizer.next());
    try testing.expectEqualDeep(Token{ .start = 8, .end = 9, .token_type = .close_paren }, tokenizer.next());
    try testing.expectEqualDeep(null, tokenizer.next());
}

test "complex s-expression returns each token" {
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
