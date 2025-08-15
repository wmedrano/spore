const std = @import("std");
const testing = std.testing;

/// A simple tokenizer for s-expressions.
const Tokenizer = @This();

/// The input text being tokenized.
text: []const u8,
/// The current position in the `text`.
start: usize = 0,

/// The types of tokens that can be produced.
pub const TokenType = enum { open_paren, close_paren, identifier, string, comment };

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
    if (self.isDone()) return null;
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
    const token_type = t: switch (next_ch) {
        '"' => {
            self.eatString();
            break :t TokenType.string;
        },
        ';' => {
            self.eatComment();
            break :t TokenType.comment;
        },
        else => {
            self.eatIdentifier();
            break :t TokenType.identifier;
        },
    };
    return Token{ .start = start, .end = self.start, .token_type = token_type };
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

/// Advances the tokenizer's `start` pointer past a comment.
///
/// A comment starts with a semicolon `;` and continues until a newline
/// character `\n` or the end of the input.
fn eatComment(self: *Tokenizer) void {
    while (!self.isDone()) {
        const next_ch = self.text[self.start];
        if (next_ch ==
            '\n') return;
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

/// Advances the tokenizer's `start` pointer past a quoted string literal.
/// It skips the opening quote, then consumes characters until it finds the closing quote.
/// If no closing quote is found, it consumes until the end of the input.
fn eatString(self: *Tokenizer) void {
    self.start += 1;
    var escaped = false;
    while (!self.isDone()) {
        const next_ch = self.text[self.start];
        self.start += 1;
        switch (next_ch) {
            '\\' => escaped = !escaped,
            '"' => if (escaped) {
                escaped = false;
            } else {
                return;
            },
            else => escaped = false,
        }
    }
}

test "empty string has no tokens" {
    var tokenizer = Tokenizer.init("");
    try testing.expectEqualDeep(null, tokenizer.next());
    try testing.expectEqualDeep(null, tokenizer.next());
}

test "s-expression returns each token" {
    var tokenizer = Tokenizer.init("(+ one 2)");
    try testing.expectEqualDeep(
        Token{ .start = 0, .end = 1, .token_type = .open_paren },
        tokenizer.next(),
    );
    try testing.expectEqualDeep(
        Token{ .start = 1, .end = 2, .token_type = .identifier },
        tokenizer.next(),
    );
    try testing.expectEqualDeep(
        Token{ .start = 3, .end = 6, .token_type = .identifier },
        tokenizer.next(),
    );
    try testing.expectEqualDeep(
        Token{ .start = 7, .end = 8, .token_type = .identifier },
        tokenizer.next(),
    );
    try testing.expectEqualDeep(
        Token{ .start = 8, .end = 9, .token_type = .close_paren },
        tokenizer.next(),
    );
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

test "quoted string is parsed as single string" {
    var tokenizer = Tokenizer.init("\"hello world\n...\"");
    try testing.expectEqualDeep(
        Token{ .start = 0, .end = 17, .token_type = .string },
        tokenizer.next(),
    );
    try testing.expectEqualDeep(null, tokenizer.next());
}

test "comment is parsed" {
    var tokenizer = Tokenizer.init("; this is a comment\n");
    try testing.expectEqualStrings(
        "; this is a comment",
        tokenizer.nextStr().?,
    );
    try testing.expectEqualDeep(null, tokenizer.next());
}

test "multiple lines of comments are parsed as a comment each" {
    var tokenizer = Tokenizer.init("; first comment\n; second comment");
    try testing.expectEqualStrings(
        "; first comment",
        tokenizer.nextStr().?,
    );
    try testing.expectEqualStrings(
        "; second comment",
        tokenizer.nextStr().?,
    );
    try testing.expectEqualDeep(null, tokenizer.next());
}
